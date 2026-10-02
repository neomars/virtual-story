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
    #[serde(skip)]
    gen: u64,
}

impl Default for ComponentStatus {
    fn default() -> Self {
        Self { state: "stopped".into(), model: None, error: None, plan: None, gen: 0 }
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
}

fn port_of(url: &str, default: u16) -> u16 {
    url.trim_end_matches('/').rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(default)
}

fn fill(arg: &str, model: &str, dir: &str, host: &str, port: u16) -> String {
    arg.replace("{model}", model).replace("{dir}", dir).replace("{host}", host).replace("{port}", &port.to_string())
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
        })
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
        let installed = self.models.installed(id).ok_or("modèle non installé : télécharge-le d'abord")?;
        let kind = entry.kind;
        self.unload(kind).await;

        let (bin, args, url, any_status, plan) = self.build_spawn(&entry, &installed).await?;
        let gen = self.counter.fetch_add(1, Ordering::SeqCst);
        self.set_status(kind, 0, |s| {
            *s = ComponentStatus { state: "loading".into(), model: Some(id.into()), error: None, plan: plan.clone(), gen };
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
        for (stream, is_err) in [(child.stdout.take().map(|s| Box::pin(s) as std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>), false),
                                 (child.stderr.take().map(|s| Box::pin(s) as std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>), true)] {
            if let Some(s) = stream {
                let logs = self.logs.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(s).lines();
                    while let Ok(Some(l)) = lines.next_line().await {
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
                        let _ = child.kill().await;
                        me.set_status(kind, gen, |s| *s = ComponentStatus { gen, ..Default::default() });
                        break;
                    }
                    ok = &mut ready, if !is_ready => {
                        is_ready = true;
                        if ok {
                            me.set_status(kind, gen, |s| s.state = "ready".into());
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
                if hw.vram_gb <= 0.0 {
                    hw.vram_gb = self.gpu().await.map(|g| g.total_mb as f64 / 1024.0).unwrap_or(15.0);
                }
                let weights_gb = inst.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let plan = hardware::plan(&hw, &params, weights_gb);
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
                let args = e.args.iter().map(|a| fill(a, &model, &dir, "127.0.0.1", port)).collect();
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

    pub async fn status_json(&self) -> Value {
        let comp = |k: Kind| serde_json::to_value(self.status_of(k)).unwrap_or(Value::Null);
        let gpu = self.gpu().await;
        json!({
            "managed": self.cfg.engine.managed,
            "gpu": gpu,
            "configured_vram_gb": self.cfg.hardware.vram_gb,
            "llm": comp(Kind::Llm), "stt": comp(Kind::Stt), "tts": comp(Kind::Tts),
            "selection": self.selection(),
        })
    }
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
    fn port_depuis_url() {
        assert_eq!(port_of("http://127.0.0.1:8081/", 1), 8081);
        assert_eq!(port_of("http://localhost", 9), 9);
    }

    #[test]
    fn jokers_des_arguments() {
        assert_eq!(fill("--m={model}:{port}@{host} {dir}", "/m.onnx", "/d", "h", 7), "--m=/m.onnx:7@h /d");
    }

    #[test]
    fn parametres_lus_du_gguf() {
        let info = gguf::GgufInfo {
            architecture: "x".into(), block_count: 48, context_length: 8192, head_count: 16, head_count_kv: 8,
            key_length: Some(256), embedding_length: None, sliding_window: None,
        };
        let p = llm_params(Some(&info), &LlmConfig::default());
        assert_eq!((p.n_layers, p.n_kv_heads, p.head_dim, p.ctx_tokens), (48, 8, 256, 8192));
    }
}
