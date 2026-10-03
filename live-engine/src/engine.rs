//! Superviseur : charge / décharge les modèles en lançant llama-server, whisper-server et le serveur
//! TTS comme processus enfants, avec le plan mémoire calculé depuis l'architecture RÉELLE du GGUF.

use crate::config::{Config, LlmConfig};
use crate::gguf;
use crate::hardware::{self, GpuInfo, LlmPlan};
use crate::models::{CatalogEntry, Installed, Kind, Models};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const LOG_LINES: usize = 400;
const READY_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize)]
pub struct ComponentStatus {
    /// stopped | loading | ready | error
    pub state: String,
    pub model: Option<String>,
    pub error: Option<String>,
    pub plan: Option<LlmPlan>,
    /// Alerte non bloquante (ex. le serveur semble tourner sur le CPU).
    pub warning: Option<String>,
    pub pid: Option<u32>,
    #[serde(skip)]
    gen: u64,
}

impl Default for ComponentStatus {
    fn default() -> Self {
        Self { state: "stopped".into(), model: None, error: None, plan: None, warning: None, pid: None, gen: 0 }
    }
}

struct Running {
    kill: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub llm: Option<String>,
    pub stt: Option<String>,
    pub tts: Option<String>,
}

pub struct Engine {
    cfg: Arc<Config>,
    models: Arc<Models>,
    status: Arc<Mutex<HashMap<Kind, ComponentStatus>>>,
    logs: Arc<Mutex<HashMap<Kind, VecDeque<String>>>>,
    running: tokio::sync::Mutex<HashMap<Kind, Running>>,
    counter: AtomicU64,
    tts_runtime: Mutex<RuntimeStatus>,
}

fn port_of(url: &str, default: u16) -> u16 {
    url.trim_end_matches('/').rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(default)
}

fn fill(arg: &str, model: &str, dir: &str, voices: &str, host: &str, port: u16) -> String {
    arg.replace("{model}", model)
        .replace("{dir}", dir)
        .replace("{voices}", voices)
        .replace("{host}", host)
        .replace("{port}", &port.to_string())
}

/// Environnement Python isolé du moteur de voix (même convention que scripts/tts/tts-server).
pub fn tts_venv() -> PathBuf {
    if let Ok(v) = std::env::var("VS_TTS_VENV") {
        if !v.is_empty() { return v.into(); }
    }
    let base = std::env::var("XDG_DATA_HOME").ok().filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".local/share")
    });
    base.join("virtual-story").join("tts-venv")
}

/// Version de l'installation du moteur de voix (écrite dans `.ready` par scripts/tts/setup-tts.sh).
/// 2 = setuptools<81 (pkg_resources pour le filigrane « perth » de Chatterbox). Une installation plus ancienne
/// est proposée à la mise à jour au lieu d'échouer au chargement.
pub const TTS_RUNTIME_VERSION: &str = "2";

/// Le moteur de voix est prêt si l'environnement installé est de la bonne version (ou en mode test TTS_FAKE=1).
pub fn tts_runtime_ready() -> bool {
    std::env::var("TTS_FAKE").ok().as_deref() == Some("1")
        || std::fs::read_to_string(tts_venv().join(".ready")).map(|v| v.trim() == TTS_RUNTIME_VERSION).unwrap_or(false)
}

/// Environnement présent mais d'une ancienne version (ou installation interrompue).
fn tts_runtime_outdated() -> bool {
    tts_venv().join("bin").join("python").exists()
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RuntimeStatus {
    /// absent | outdated | installing | ready | error
    pub state: String,
    pub error: Option<String>,
}

/// Paramètres du modèle d'après son GGUF (repli : valeurs de la configuration).
pub fn llm_params(info: Option<&gguf::GgufInfo>, base: &LlmConfig) -> LlmConfig {
    let mut p = base.clone();
    if let Some(i) = info {
        p.n_layers = i.block_count;
        p.n_kv_heads = i.head_count_kv.max(1);
        p.head_dim = i.head_dim();
        if i.context_length > 0 {
            p.ctx_tokens = p.ctx_tokens.min(i.context_length);
        }
    }
    p
}

impl Engine {
    pub fn new(cfg: Arc<Config>, models: Arc<Models>) -> Arc<Self> {
        Arc::new(Self {
            cfg,
            models,
            status: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(HashMap::new())),
            running: tokio::sync::Mutex::new(HashMap::new()),
            counter: AtomicU64::new(1),
            tts_runtime: Mutex::new(RuntimeStatus::default()),
        })
    }

    /// VRAM à garder pour le micro et la voix qui doivent encore se charger (répartie selon leur poids habituel :
    /// Whisper ≈ 1,2 Go, voix ≈ 4,5 Go, total = `hardware.vram_other_models_gb`). Seuls les composants choisis comptent.
    fn pending_other_gb(&self) -> f64 {
        let sel = self.selection();
        let pending = |k: Kind, chosen: bool, enabled: bool| enabled && chosen && self.status_of(k).state != "ready";
        let w = if pending(Kind::Stt, sel.stt.is_some(), self.cfg.stt.enabled && self.cfg.stt.use_gpu) { 1.2 } else { 0.0 }
            + if pending(Kind::Tts, sel.tts.is_some(), self.cfg.tts.enabled) { 4.5 } else { 0.0 };
        self.cfg.hardware.vram_other_models_gb * w / 5.7
    }

    pub fn status_of(&self, k: Kind) -> ComponentStatus {
        self.status.lock().unwrap().get(&k).cloned().unwrap_or_default()
    }

    fn set_status(&self, k: Kind, gen: u64, f: impl FnOnce(&mut ComponentStatus)) {
        let mut g = self.status.lock().unwrap();
        let st = g.entry(k).or_default();
        if gen == 0 || st.gen == gen {
            f(st);
        }
    }

    /// Plan du LLM chargé, ou plan de repli calculé depuis la configuration.
    pub fn llm_plan(&self) -> LlmPlan {
        self.status_of(Kind::Llm)
            .plan
            .unwrap_or_else(|| hardware::plan(&self.cfg.hardware, &self.cfg.llm, self.cfg.llm.model_size_gb))
    }

    pub fn logs(&self, k: Kind, n: usize) -> Vec<String> {
        let g = self.logs.lock().unwrap();
        g.get(&k).map(|d| d.iter().rev().take(n).rev().cloned().collect()).unwrap_or_default()
    }

    fn push_log(logs: &Mutex<HashMap<Kind, VecDeque<String>>>, k: Kind, line: String) {
        let mut g = logs.lock().unwrap();
        let d = g.entry(k).or_default();
        if d.len() >= LOG_LINES { d.pop_front(); }
        d.push_back(line);
    }

    fn resolve_bin(&self, name: &str) -> PathBuf {
        let p = self.cfg.server.bin_dir.join(name);
        if p.exists() { p } else { PathBuf::from(name) }
    }

    fn selection_file(&self) -> PathBuf {
        self.cfg.server.data_dir.join("live-state.json")
    }

    pub fn selection(&self) -> Selection {
        std::fs::read_to_string(self.selection_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    fn remember(&self, k: Kind, id: Option<&str>) {
        let mut s = self.selection();
        let v = id.map(String::from);
        match k { Kind::Llm => s.llm = v, Kind::Stt => s.stt = v, Kind::Tts => s.tts = v }
        let _ = std::fs::create_dir_all(&self.cfg.server.data_dir);
        let _ = std::fs::write(self.selection_file(), serde_json::to_vec_pretty(&s).unwrap_or_default());
    }

    pub async fn gpu(&self) -> Option<GpuInfo> {
        hardware::detect_gpu().await
    }

    /// Charge un modèle installé : lance le serveur correspondant. Retourne dès que le processus est lancé ;
    /// l'état passe à `ready` quand le serveur répond (voir `status`).
    pub async fn load(self: &Arc<Self>, id: &str) -> Result<(), String> {
        if !self.cfg.engine.managed {
            return Err("moteur non géré (engine.managed = false) : lance les serveurs toi-même".into());
        }
        let entry = self.models.entry(id).ok_or("modèle inconnu")?.clone();
        if entry.kind == Kind::Tts && !tts_runtime_ready() {
            return Err("moteur de voix non installé : clique sur « Installer le moteur de voix »".into());
        }
        let installed = self.models.installed(id).ok_or("modèle non installé : télécharge-le d'abord")?;
        let kind = entry.kind;
        self.unload(kind).await;

        let (bin, args, url, any_status, plan) = self.build_spawn(&entry, &installed).await?;
        let gen = self.counter.fetch_add(1, Ordering::SeqCst);
        self.set_status(kind, 0, |s| {
            *s = ComponentStatus { state: "loading".into(), model: Some(id.into()), error: None, plan: plan.clone(), warning: None, pid: None, gen };
        });
        self.remember(kind, Some(id));

        let mut cmd = Command::new(self.resolve_bin(&bin));
        cmd.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        // llama-server ne surveille pas son parent : si le moteur est tué brutalement, le noyau lui envoie
        // SIGTERM (sinon ~11 Go de VRAM resteraient occupés par un orphelin).
        #[cfg(target_os = "linux")]
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong);
                Ok(())
            });
        }
        if let Ok(abs) = std::fs::canonicalize(&self.cfg.server.bin_dir) {
            let cur = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
            cmd.env("LD_LIBRARY_PATH", format!("{}:{cur}", abs.display()));
        }
        Self::push_log(&self.logs, kind, format!("$ {bin} {}", args.join(" ")));
        let mut child = cmd.spawn().map_err(|e| {
            let msg = format!("impossible de lancer « {bin} » ({e}) — est-il dans {} ou dans le PATH ?", self.cfg.server.bin_dir.display());
            self.set_status(kind, gen, |s| { s.state = "error".into(); s.error = Some(msg.clone()) });
            msg
        })?;
        let child_pid = child.id();
        self.set_status(kind, gen, |s| s.pid = child_pid);
        // llama-server / whisper-server démarrent sans erreur sur le CPU si les bibliothèques CUDA manquent :
        // on repère dans leurs journaux qu'un périphérique CUDA a bien été initialisé.
        let expect_cuda = self.gpu().await.is_some()
            && (kind == Kind::Llm || (kind == Kind::Stt && self.cfg.stt.use_gpu));
        let cuda_seen = Arc::new(std::sync::atomic::AtomicBool::new(false));
        for (stream, is_err) in [(child.stdout.take().map(|s| Box::pin(s) as std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>), false),
                                 (child.stderr.take().map(|s| Box::pin(s) as std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>), true)] {
            if let Some(s) = stream {
                let logs = self.logs.clone();
                let seen = cuda_seen.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(s).lines();
                    while let Ok(Some(l)) = lines.next_line().await {
                        if mentions_cuda_device(&l) { seen.store(true, Ordering::Relaxed); }
                        Self::push_log(&logs, kind, if is_err { format!("[err] {l}") } else { l });
                    }
                });
            }
        }

        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let me = self.clone();
        let task = tokio::spawn(async move {
            let ready = wait_ready(url, any_status);
            tokio::pin!(ready);
            let mut is_ready = false;
            loop {
                tokio::select! {
                    exit = child.wait() => {
                        let msg = match exit {
                            Ok(st) => format!("le processus s'est arrêté ({st}) — voir les journaux"),
                            Err(e) => format!("erreur d'attente du processus : {e}"),
                        };
                        me.set_status(kind, gen, |s| { s.state = "error".into(); s.error = Some(msg) });
                        break;
                    }
                    _ = &mut kill_rx => {
                        terminate(&mut child).await;
                        me.set_status(kind, gen, |s| *s = ComponentStatus { gen, ..Default::default() });
                        break;
                    }
                    ok = &mut ready, if !is_ready => {
                        is_ready = true;
                        if ok {
                            me.set_status(kind, gen, |s| s.state = "ready".into());
                            if expect_cuda {
                                tokio::time::sleep(Duration::from_millis(800)).await; // laisser les journaux arriver
                                if !gpu_in_use(child_pid, &cuda_seen).await {
                                    let driver = hardware::detect_gpu().await.map(|g| g.driver).unwrap_or_default();
                                    let hint = if !driver.is_empty() && !hardware::driver_at_least(&driver, 570, 26) {
                                        format!("Votre pilote NVIDIA ({driver}) est trop ancien pour CUDA 12.8 (≥ 570.26 requis) : mettez-le à jour.")
                                    } else {
                                        "Vérifiez que libggml-cuda.so, libcudart et libcublas sont dans bin/ (voir les journaux).".to_string()
                                    };
                                    me.set_status(kind, gen, |s| s.warning = Some(format!(
                                        "Ce serveur n'utilise pas le GPU (aucune mémoire GPU pour son processus) : il tourne sur le CPU, très lent. {hint}")));
                                }
                            }
                        } else {
                            me.set_status(kind, gen, |s| { s.state = "error".into(); s.error = Some("délai de démarrage dépassé (5 min)".into()) });
                        }
                    }
                }
            }
        });
        self.running.lock().await.insert(kind, Running { kill: kill_tx, task });
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    async fn build_spawn(&self, e: &CatalogEntry, inst: &Installed) -> Result<(String, Vec<String>, String, bool, Option<LlmPlan>), String> {
        let model = inst.files.first().ok_or("modèle sans fichier")?.to_string_lossy().to_string();
        let dir = inst.dir.to_string_lossy().to_string();
        match e.kind {
            Kind::Llm => {
                let info = gguf::read_info(inst.files.first().unwrap()).map_err(|er| format!("GGUF illisible : {er}"))?;
                let params = llm_params(Some(&info), &self.cfg.llm);
                let mut hw = self.cfg.hardware.clone();
                let gpu = self.gpu().await;
                if hw.vram_gb <= 0.0 {
                    hw.vram_gb = gpu.as_ref().map(|g| g.total_mb as f64 / 1024.0).unwrap_or(15.0);
                }
                // On ne réserve de la VRAM que pour la voix et le micro choisis et pas encore chargés.
                hw.vram_other_models_gb = self.pending_other_gb();
                // Garde-fou : jamais plus que la VRAM réellement libre (navigateur, autres applications…).
                if let Some(g) = &gpu {
                    let free_budget = g.total_mb.saturating_sub(g.used_mb) as f64 / 1024.0 - 0.3 - hw.vram_other_models_gb;
                    if free_budget < hw.vram_gb - hw.vram_reserve_gb - hw.vram_other_models_gb {
                        hw.vram_reserve_gb = (hw.vram_gb - hw.vram_other_models_gb - free_budget).max(hw.vram_reserve_gb);
                    }
                }
                let weights_gb = inst.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let plan = hardware::plan_for_gguf(&hw, &params, weights_gb, &info);
                let port = port_of(&self.cfg.llm.url, 8080);
                let mut extra = self.cfg.llm.extra_args.clone();
                extra.extend(e.args.iter().cloned());
                let args = hardware::llama_server_args(&plan, &model, port, &extra);
                Ok(("llama-server".into(), args, format!("{}/health", self.cfg.llm.url.trim_end_matches('/')), false, Some(plan)))
            }
            Kind::Stt => {
                let port = port_of(&self.cfg.stt.url, 8081);
                let mut args: Vec<String> = vec!["-m".into(), model, "--host".into(), "127.0.0.1".into(), "--port".into(), port.to_string(),
                                                 "-l".into(), self.cfg.stt.language.clone()];
                if !self.cfg.stt.use_gpu { args.push("-ng".into()); }
                args.extend(e.args.iter().cloned());
                Ok(("whisper-server".into(), args, self.cfg.stt.url.clone(), true, None))
            }
            Kind::Tts => {
                let exec = e.exec.clone().ok_or("entrée TTS sans `exec` dans le catalogue")?;
                let port = port_of(&self.cfg.tts.url, 8880);
                let voices = self.cfg.voices_dir();
                let _ = std::fs::create_dir_all(&voices);
                let args = e.args.iter().map(|a| fill(a, &model, &dir, &voices.to_string_lossy(), "127.0.0.1", port)).collect();
                Ok((exec, args, self.cfg.tts.url.clone(), true, None))
            }
        }
    }

    pub async fn unload(&self, kind: Kind) {
        let r = self.running.lock().await.remove(&kind);
        if let Some(r) = r {
            let _ = r.kill.send(());
            let _ = tokio::time::timeout(Duration::from_secs(15), r.task).await;
        }
        let gen = self.status_of(kind).gen;
        self.set_status(kind, gen, |s| *s = ComponentStatus { gen, ..Default::default() });
        self.remember(kind, None);
    }

    /// Arrêt de l'app : on libère le GPU sans oublier la sélection (rechargée au prochain démarrage).
    pub async fn shutdown(&self) {
        let sel = self.selection();
        for k in [Kind::Tts, Kind::Stt, Kind::Llm] {
            if let Some(r) = self.running.lock().await.remove(&k) {
                let _ = r.kill.send(());
                let _ = tokio::time::timeout(Duration::from_secs(10), r.task).await;
            }
        }
        let _ = std::fs::write(self.selection_file(), serde_json::to_vec_pretty(&sel).unwrap_or_default());
    }

    /// Recharge les derniers modèles choisis (LLM d'abord, puis STT et TTS, pour éviter une course à la VRAM).
    pub async fn autoload(self: &Arc<Self>) {
        let sel = self.selection();
        for (kind, id) in [(Kind::Llm, sel.llm), (Kind::Stt, sel.stt), (Kind::Tts, sel.tts)] {
            let Some(id) = id else { continue };
            if self.models.installed(&id).is_none() { continue }
            match self.load(&id).await {
                Ok(()) => {
                    for _ in 0..(READY_TIMEOUT.as_secs() * 2) {
                        if self.status_of(kind).state != "loading" { break }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
                Err(e) => tracing::warn!("rechargement de {id} impossible : {e}"),
            }
        }
    }

    pub fn tts_runtime_status(&self) -> RuntimeStatus {
        let mut st = self.tts_runtime.lock().unwrap().clone();
        if st.state != "installing" && st.state != "error" {
            st.state = if tts_runtime_ready() { "ready" } else if tts_runtime_outdated() { "outdated" } else { "absent" }.into();
        }
        st
    }

    /// Installe le moteur de voix en lançant `setup-tts.sh` ; la sortie va dans les journaux du TTS.
    pub async fn install_tts_runtime(self: &Arc<Self>) -> Result<(), String> {
        {
            let mut g = self.tts_runtime.lock().unwrap();
            if g.state == "installing" { return Ok(()) }
            *g = RuntimeStatus { state: "installing".into(), error: None };
        }
        let script = self.cfg.server.bin_dir.join("setup-tts.sh");
        if !script.exists() {
            let msg = format!("{} introuvable : lance scripts/build-sidecars.sh", script.display());
            *self.tts_runtime.lock().unwrap() = RuntimeStatus { state: "error".into(), error: Some(msg.clone()) };
            return Err(msg);
        }
        let mut cmd = Command::new("bash");
        cmd.arg(&script).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| format!("lancement impossible : {e}"))?;
        Self::push_log(&self.logs, Kind::Tts, format!("$ bash {}", script.display()));
        for (s, err) in [(child.stdout.take().map(|s| Box::pin(s) as std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>), false),
                         (child.stderr.take().map(|s| Box::pin(s) as std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>), true)] {
            if let Some(s) = s {
                let logs = self.logs.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(s).lines();
                    while let Ok(Some(l)) = lines.next_line().await {
                        Self::push_log(&logs, Kind::Tts, if err { format!("[err] {l}") } else { l });
                    }
                });
            }
        }
        let me = self.clone();
        tokio::spawn(async move {
            let st = match child.wait().await {
                Ok(s) if s.success() && tts_runtime_ready() => RuntimeStatus { state: "ready".into(), error: None },
                Ok(s) => RuntimeStatus { state: "error".into(), error: Some(format!("l'installation a échoué ({s}) — voir les journaux")) },
                Err(e) => RuntimeStatus { state: "error".into(), error: Some(e.to_string()) },
            };
            *me.tts_runtime.lock().unwrap() = st;
        });
        Ok(())
    }

    pub async fn status_json(&self) -> Value {
        let gpu = self.gpu().await;
        let apps = hardware::gpu_memory_by_pid().await;
        let comp = |k: Kind| {
            let st = self.status_of(k);
            let mut v = serde_json::to_value(&st).unwrap_or(Value::Null);
            if let (Some(pid), Some(map)) = (st.pid, &apps) {
                if let Some(mb) = map.get(&pid) { v["gpu_mb"] = json!(mb); }
            }
            v
        };
        json!({
            "managed": self.cfg.engine.managed,
            "gpu": gpu,
            "configured_vram_gb": self.cfg.hardware.vram_gb,
            "llm": comp(Kind::Llm), "stt": comp(Kind::Stt), "tts": comp(Kind::Tts),
            "selection": self.selection(),
            "tts_runtime": self.tts_runtime_status(),
        })
    }
}

/// Arrêt en douceur : SIGTERM (les serveurs libèrent proprement le GPU), puis SIGKILL après 4 s.
async fn terminate(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // SAFETY : envoi d'un signal à un processus qu'on a lancé nous-mêmes.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
        if tokio::time::timeout(Duration::from_secs(4), child.wait()).await.is_ok() {
            return;
        }
    }
    let _ = child.kill().await;
}

/// Le processus utilise-t-il vraiment le GPU ? Réponse sûre : `true` dès qu'un indice positif existe (mot-clé CUDA
/// dans les journaux, ou mémoire GPU attribuée au PID par nvidia-smi) ou quand on ne peut pas savoir ; `false`
/// seulement si nvidia-smi liste les processus GPU et que le nôtre n'y figure pas (ou avec moins de 200 Mo).
async fn gpu_in_use(pid: Option<u32>, cuda_seen: &std::sync::atomic::AtomicBool) -> bool {
    if cuda_seen.load(Ordering::Relaxed) {
        return true;
    }
    match (pid, hardware::gpu_memory_by_pid().await) {
        (Some(pid), Some(apps)) => apps.get(&pid).copied().unwrap_or(0) >= 200,
        _ => true,
    }
}

/// Ligne de journal prouvant qu'un backend CUDA est actif (llama.cpp / whisper.cpp).
pub fn mentions_cuda_device(line: &str) -> bool {
    line.contains("ggml_cuda_init: found") || line.contains("loaded CUDA backend") || line.contains("CUDA0")
        || line.contains("Device 0: NVIDIA")
}

async fn wait_ready(url: String, any_status: bool) -> bool {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().expect("client");
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        if let Ok(r) = http.get(&url).send().await {
            if any_status || r.status().is_success() { return true }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_du_backend_cuda_dans_les_journaux() {
        assert!(mentions_cuda_device("ggml_cuda_init: found 1 CUDA devices:"));
        assert!(mentions_cuda_device("load_backend: loaded CUDA backend from /app/libggml-cuda.so"));
        assert!(mentions_cuda_device("llama_kv_cache: CUDA0 KV buffer size = 391.00 MiB"));
        assert!(!mentions_cuda_device("load_tensors: CPU_Mapped model buffer size = 7400.00 MiB"));
        assert!(!mentions_cuda_device("srv  load_model: loading model"));
    }

    #[test]
    fn port_depuis_url() {
        assert_eq!(port_of("http://127.0.0.1:8081/", 1), 8081);
        assert_eq!(port_of("http://localhost", 9), 9);
    }

    #[test]
    fn jokers_des_arguments() {
        assert_eq!(fill("--m={model}:{port}@{host} {dir} {voices}", "/m.onnx", "/d", "/v", "h", 7), "--m=/m.onnx:7@h /d /v");
    }

    #[test]
    fn parametres_lus_du_gguf() {
        let info = gguf::GgufInfo {
            architecture: "x".into(), block_count: 48, context_length: 8192, head_count: 16, head_count_kv: 8,
            key_length: Some(256), ..Default::default()
        };
        let p = llm_params(Some(&info), &LlmConfig::default());
        assert_eq!((p.n_layers, p.n_kv_heads, p.head_dim, p.ctx_tokens), (48, 8, 256, 8192));
    }
}
