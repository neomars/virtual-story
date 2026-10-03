//! Catalogue de modèles, résolution des fichiers sur Hugging Face, téléchargement reprenable avec
//! vérification SHA-256, et détection des modèles installés.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Llm,
    Stt,
    Tts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Dépôt Hugging Face « auteur/nom ».
    pub repo: String,
    /// Sous-dossier du dépôt (vide = racine).
    #[serde(default)]
    pub subdir: String,
    /// Noms de fichiers exacts à télécharger (prioritaire sur `pattern`).
    #[serde(default)]
    pub files: Vec<String>,
    /// Sous-chaîne (insensible à la casse) pour retrouver le .gguf voulu (ex. « Q4_K_M »).
    #[serde(default)]
    pub pattern: Option<String>,
    /// Dépôts de secours essayés dans l'ordre si `repo` est introuvable ou ne contient pas le fichier voulu.
    #[serde(default)]
    pub alt_repos: Vec<String>,
    /// Extensions à télécharger (sans point) quand on veut tous les fichiers d'un dépôt de ce type
    /// (ex. `["safetensors", "pt", "json"]` pour un modèle PyTorch). Utilisé si ni `files` ni `pattern`.
    #[serde(default)]
    pub include_ext: Vec<String>,
    /// Taille indicative en Go (avant résolution).
    #[serde(default)]
    pub size_gb: f64,
    /// Arguments supplémentaires pour le serveur de ce modèle (ex. options du chat template).
    #[serde(default)]
    pub args: Vec<String>,
    /// Exécutable à lancer pour ce modèle (TTS) ; arguments dans `args`, avec les jokers
    /// `{model}` (premier fichier), `{dir}`, `{host}` et `{port}`.
    #[serde(default)]
    pub exec: Option<String>,
    /// Remarque d'usage.
    #[serde(default)]
    pub note: String,
}

pub fn builtin_catalog() -> Vec<CatalogEntry> {
    let e = |id: &str, kind, name: &str, desc: &str, repo: &str, files: &[&str], pattern: Option<&str>, size: f64, args: &[&str]| {
        CatalogEntry {
            id: id.into(), kind, name: name.into(), description: desc.into(), repo: repo.into(),
            subdir: String::new(), files: files.iter().map(|s| s.to_string()).collect(),
            pattern: pattern.map(String::from), alt_repos: vec![], include_ext: vec![], size_gb: size,
            args: args.iter().map(|s| s.to_string()).collect(), exec: None, note: String::new(),
        }
    };
    let mut tts = e("chatterbox-multilingual", Kind::Tts, "Chatterbox multilingue — voix clonée, expressive (recommandé)",
        "Synthèse vocale en français à partir d'un court échantillon de voix (≈ 10-20 s) et d'un réglage d'expressivité. \
         Licence MIT. ≈ 3,5 Go de téléchargement, ≈ 3,5 à 5 Go de VRAM. Nécessite d'installer le moteur de voix (Python/PyTorch). \
         Le premier démarrage a besoin d'internet (tokenizer chinois, téléchargé une fois).",
        "ResembleAI/chatterbox",
        &["ve.pt", "t3_mtl23ls_v2.safetensors", "s3gen.pt", "grapheme_mtl_merged_expanded_v1.json", "conds.pt", "Cangjie5_TC.json"],
        None, 3.5,
        &["--host", "{host}", "--port", "{port}", "--model-dir", "{dir}", "--voices-dir", "{voices}"]);
    tts.exec = Some("tts-server".into());
    let mut v = vec![
        e("gemma4-12b-heretic", Kind::Llm, "Gemma 4 12B Heretic (Q4_K_M)",
          "Gemma 4 12B décensuré par abliteration. ≈ 8,6 Go de VRAM : pour une carte de 16 Go ou plus, ou si la voix ne tourne pas sur le GPU. \
           Sur 12 Go avec la voix, préfère un modèle « Léger ».",
          "igorls/gemma-4-12B-it-heretic-GGUF", &[], Some("Q4_K_M"), 7.4,
          &["--reasoning", "off"]),
        // ---- Petits modèles non censurés (≈ 2 à 5,5 Go) : ils laissent de la VRAM à la voix et au micro.
        // Les chiffres de refus viennent des auteurs (extraits de leurs fiches) ; aucun n'est testé ici sur le français.
        e("gemma4-e4b-heretic", Kind::Llm, "Gemma 4 E4B Heretic (Q4_K_M) — recommandé avec la voix sur 12 Go",
          "≈ 4 Md de paramètres effectifs. Non censuré (3 refus sur 100 annoncés par l'auteur), dérive minime annoncée par rapport à Gemma 4 : \
           bon français attendu mais non mesuré. Tient avec la voix et le micro sur une carte de 12 Go.",
          "llmfan46/gemma-4-E4B-it-ultra-uncensored-heretic-GGUF", &[], Some("Q4_K_M"), 5.0, &["--reasoning", "off"]),
        e("gemma4-e4b-abliterated", Kind::Llm, "Gemma 4 E4B abliteré (Q4_K_M)",
          "Variante « norm-preserving abliteration » : 0,7 % de refus annoncés par l'auteur (non vérifié ici). Q4_K_M retenu.",
          "TrevorJS/gemma-4-E4B-it-uncensored-GGUF", &[], Some("Q4_K_M"), 5.3, &["--reasoning", "off"]),
        e("gemma4-e4b-rp", Kind::Llm, "Gemma 4 E4B Heretic RP (Q4_K_M)",
          "Seul fine-tune jeu de rôle sur Gemma 4 E4B trouvé (continuité de scène et de personnage). Réglage surtout anglophone : \
           à tester pour le français. Taille estimée.",
          "Ilya626/gemma-4-E4B-it-SDFT_Heretic_RP-GGUF", &[], Some("Q4_K_M"), 5.0, &["--reasoning", "off"]),
        e("gemma4-e2b-uncensored", Kind::Llm, "Gemma 4 E2B abliteré (Q4_K_M) — mini",
          "≈ 2 Md de paramètres effectifs, 0,4 % de refus annoncés par l'auteur (non vérifié ici). Plus faible en français et en cohérence que le E4B : \
           pour les petites configurations.",
          "TrevorJS/gemma-4-E2B-it-uncensored-GGUF", &[], Some("Q4_K_M"), 3.4, &["--reasoning", "off"]),
        e("ministral3-3b-heresy", Kind::Llm, "Ministral 3 3B Heresy (Q4_K_M) — mini",
          "3 Md de paramètres, orienté jeu de rôle non filtré, français pris en charge par la base Mistral. Auteur peu connu, \
           aucun chiffre de refus : à tester.",
          "Abiray/Ministral-3-3B-Instruct-2512-Heresy-Unfiltered-GGUF", &[], Some("Q4_K_M"), 2.1, &[]),
        e("qwen35-4b-hauhau", Kind::Llm, "Qwen 3.5 4B non censuré (Q4_K_M) — mini",
          "0 refus sur 465 annoncés par l'auteur (peut ajouter de courts avertissements). Qwen annonce 201 langues ; \
           le mode réflexion est coupé au chargement.",
          "HauhauCS/Qwen3.5-4B-Uncensored-HauhauCS-Aggressive", &[], Some("Q4_K_M"), 2.6, &["--reasoning", "off"]),
        e("qwen35-9b-hauhau", Kind::Llm, "Qwen 3.5 9B non censuré (Q4_K_M)",
          "Version 9B du précédent : plus cohérent, à la limite de ce qui reste confortable avec la voix sur 12 Go.",
          "HauhauCS/Qwen3.5-9B-Uncensored-HauhauCS-Aggressive", &[], Some("Q4_K_M"), 5.3, &["--reasoning", "off"]),
        e("gemma4-12b-iq3", Kind::Llm, "Gemma 4 12B Heretic compressé (IQ3_XS)",
          "Un Gemma 4 12B abliteré (autre auteur que le « Heretic » standard), compressé en IQ3 : ≈ 5,4 Go au lieu de 7,4 Go. \
           Perte de qualité visible d'après les comparatifs de quantification, mais plus capable qu'un 4B. Refus non mesurés ici.",
          "mradermacher/gemma-4-12b-heretic-abliterated-i1-GGUF", &[], Some("i1-IQ3_XS"), 5.4, &["--reasoning", "off"]),
        e("ministral3-14b-nymphaea-rp", Kind::Llm, "Ministral 3 14B Nymphaea-RP (i1-Q5_K_M)",
          "Fine-tune jeu de rôle non censuré (base Mistral). Non vérifié sur le format des balises.",
          "mradermacher/Ministral-3-14B-Nymphaea-RP-i1-GGUF", &[], Some("i1-Q5_K_M"), 9.7, &[]),
        e("rocinante-x-12b", Kind::Llm, "Rocinante-X 12B (Q5_K_M)",
          "Fine-tune jeu de rôle sur Mistral Nemo : rapide, valeur sûre de la communauté.",
          "bartowski/TheDrummer_Rocinante-X-12B-v1-GGUF", &[], Some("Q5_K_M"), 8.7, &[]),
        e("cydonia-24b", Kind::Llm, "Cydonia 24B v4.2.0 (IQ4_XS)",
          "Plus intelligent (24B), un peu de poids en RAM ; environ 15-25 tok/s estimés.",
          "bartowski/TheDrummer_Cydonia-24B-v4.2.0-GGUF", &[], Some("IQ4_XS"), 12.8, &[]),
        e("whisper-large-v3-turbo-q5", Kind::Stt, "Whisper large-v3-turbo (Q5_0) — recommandé",
          "Reconnaissance vocale multilingue, très bonne en français (≈ 547 Mo, ≈ 1 Go de VRAM).",
          "ggerganov/whisper.cpp", &["ggml-large-v3-turbo-q5_0.bin"], None, 0.57, &[]),
        e("whisper-small-q5", Kind::Stt, "Whisper small (Q5_1) — léger",
          "Plus petit et plus rapide, un peu moins précis (≈ 181 Mo).",
          "ggerganov/whisper.cpp", &["ggml-small-q5_1.bin"], None, 0.19, &[]),
        e("whisper-medium-q5", Kind::Stt, "Whisper medium (Q5_0)",
          "Compromis précision/vitesse (≈ 0,5 Go).",
          "ggerganov/whisper.cpp", &["ggml-medium-q5_0.bin"], None, 0.54, &[]),
    ];
    v.push(tts);
    // Le dépôt du modèle recommandé n'a été vu que dans des résultats de recherche : on prévoit des dépôts de secours
    // (variante non « ultra » du même auteur, puis quantifications imatrix de mradermacher).
    for c in &mut v {
        if c.id == "gemma4-e4b-heretic" {
            c.alt_repos = vec![
                "llmfan46/gemma-4-E4B-it-uncensored-heretic-GGUF".into(),
                "mradermacher/gemma-4-E4B-it-ultra-uncensored-heretic-GGUF".into(),
            ];
        }
    }
    v
}

/// Catalogue intégré + entrées personnelles de `catalog.json` (même id = remplacement).
pub fn load_catalog(data_dir: &Path) -> Vec<CatalogEntry> {
    let mut cat = builtin_catalog();
    if let Ok(raw) = std::fs::read_to_string(data_dir.join("catalog.json")) {
        match serde_json::from_str::<Vec<CatalogEntry>>(&raw) {
            Ok(extra) => {
                for x in extra {
                    if let Some(slot) = cat.iter_mut().find(|c| c.id == x.id) { *slot = x } else { cat.push(x) }
                }
            }
            Err(e) => tracing::warn!("catalog.json ignoré : {e}"),
        }
    }
    cat
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteFile {
    pub path: String,
    pub size: u64,
    pub sha256: Option<String>,
}

impl RemoteFile {
    fn basename(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

/// Analyse la réponse de `GET /api/models/{repo}/tree/main/{subdir}`.
pub fn parse_tree(json: &serde_json::Value) -> Vec<RemoteFile> {
    json.as_array()
        .map(|a| {
            a.iter()
                .filter(|f| f["type"] == "file")
                .filter_map(|f| {
                    Some(RemoteFile {
                        path: f["path"].as_str()?.to_string(),
                        size: f["lfs"]["size"].as_u64().or_else(|| f["size"].as_u64())?,
                        sha256: f["lfs"]["oid"].as_str().map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn is_shard(name: &str) -> bool {
    // ...-00001-of-00003.gguf
    let n = name.to_lowercase();
    n.ends_with(".gguf") && n.contains("-of-")
}

/// Choisit les fichiers à télécharger. Pure, donc testable sans réseau.
pub fn select_files(entry: &CatalogEntry, listing: &[RemoteFile]) -> Result<Vec<RemoteFile>, String> {
    if !entry.files.is_empty() {
        return entry
            .files
            .iter()
            .map(|wanted| {
                listing
                    .iter()
                    .find(|f| f.basename() == wanted)
                    .cloned()
                    .ok_or_else(|| format!("fichier « {wanted} » introuvable dans {}", entry.repo))
            })
            .collect();
    }
    if entry.pattern.is_none() && !entry.include_ext.is_empty() {
        let exts: Vec<String> = entry.include_ext.iter().map(|e| e.to_lowercase()).collect();
        let mut all: Vec<RemoteFile> = listing
            .iter()
            .filter(|f| f.basename().rsplit_once('.').is_some_and(|(_, e)| exts.contains(&e.to_lowercase())))
            .cloned()
            .collect();
        all.sort_by(|a, b| a.path.cmp(&b.path));
        return if all.is_empty() {
            Err(format!("aucun fichier {} dans {}", exts.join("/"), entry.repo))
        } else {
            Ok(all)
        };
    }
    let pat = entry
        .pattern
        .as_ref()
        .ok_or("entrée de catalogue sans `files`, `pattern` ni `include_ext`")?
        .to_lowercase();
    let mut hits: Vec<RemoteFile> = listing
        .iter()
        .filter(|f| {
            let n = f.basename().to_lowercase();
            n.ends_with(".gguf") && n.contains(&pat) && !n.contains("mmproj")
        })
        .cloned()
        .collect();
    hits.sort_by(|a, b| a.path.cmp(&b.path));
    match hits.len() {
        0 => Err(format!("aucun .gguf contenant « {pat} » dans {}", entry.repo)),
        1 => Ok(hits),
        _ if hits.iter().all(|h| is_shard(h.basename())) => Ok(hits), // modèle en plusieurs morceaux
        _ => Err(format!(
            "plusieurs fichiers correspondent à « {pat} » dans {} : {} — précise `files` dans catalog.json",
            entry.repo,
            hits.iter().map(|h| h.basename().to_string()).collect::<Vec<_>>().join(", ")
        )),
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DownloadState {
    /// listing | downloading | verifying | done | error | cancelled
    pub status: String,
    pub file: String,
    pub downloaded: u64,
    pub total: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    id: String,
    repo: String,
    files: Vec<ManifestFile>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestFile {
    name: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Installed {
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
    pub size_bytes: u64,
}

pub struct Models {
    pub catalog: Vec<CatalogEntry>,
    models_dir: PathBuf,
    http: reqwest::Client,
    hf_base: String,
    token: Option<String>,
    states: Mutex<HashMap<String, DownloadState>>,
    handles: Mutex<HashMap<String, tokio::task::AbortHandle>>,
}

impl Models {
    pub fn new(catalog: Vec<CatalogEntry>, models_dir: PathBuf) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&models_dir);
        Arc::new(Self {
            catalog,
            models_dir,
            http: reqwest::Client::builder()
                .user_agent("virtual-story-live/0.1")
                .connect_timeout(std::time::Duration::from_secs(20))
                .build()
                .expect("client HTTP"),
            hf_base: std::env::var("HF_ENDPOINT").unwrap_or_else(|_| "https://huggingface.co".into()),
            token: std::env::var("HF_TOKEN").ok().filter(|t| !t.is_empty()),
            states: Mutex::new(HashMap::new()),
            handles: Mutex::new(HashMap::new()),
        })
    }

    pub fn entry(&self, id: &str) -> Option<&CatalogEntry> {
        self.catalog.iter().find(|c| c.id == id)
    }

    pub fn dir_of(&self, id: &str) -> PathBuf {
        self.models_dir.join(id)
    }

    pub fn state(&self, id: &str) -> Option<DownloadState> {
        self.states.lock().unwrap().get(id).cloned()
    }

    fn set_state(&self, id: &str, f: impl FnOnce(&mut DownloadState)) {
        let mut g = self.states.lock().unwrap();
        f(g.entry(id.to_string()).or_default());
    }

    /// Un modèle est installé si son manifeste existe et que tous ses fichiers ont la bonne taille.
    pub fn installed(&self, id: &str) -> Option<Installed> {
        let dir = self.dir_of(id);
        let m: Manifest = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).ok()?).ok()?;
        let mut files = vec![];
        let mut total = 0;
        for f in &m.files {
            let p = dir.join(&f.name);
            if std::fs::metadata(&p).ok()?.len() != f.size {
                return None;
            }
            total += f.size;
            files.push(p);
        }
        files.sort();
        Some(Installed { dir, files, size_bytes: total })
    }

    pub fn delete(&self, id: &str) -> bool {
        self.entry(id).is_some() && std::fs::remove_dir_all(self.dir_of(id)).is_ok()
    }

    async fn list_remote(&self, entry: &CatalogEntry) -> anyhow::Result<Vec<RemoteFile>> {
        let sub = entry.subdir.trim_matches('/');
        let url = if sub.is_empty() {
            format!("{}/api/models/{}/tree/main", self.hf_base, entry.repo)
        } else {
            format!("{}/api/models/{}/tree/main/{}", self.hf_base, entry.repo, sub)
        };
        let mut req = self.http.get(&url);
        if let Some(t) = &self.token { req = req.bearer_auth(t); }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Hugging Face : HTTP {} pour {}", resp.status(), url);
        }
        Ok(parse_tree(&resp.json().await?))
    }

    /// Lance le téléchargement en tâche de fond. `false` si déjà en cours.
    pub fn start_download(self: &Arc<Self>, id: &str) -> Result<bool, String> {
        let entry = self.entry(id).ok_or("modèle inconnu")?.clone();
        if matches!(self.state(id).as_ref().map(|s| s.status.as_str()), Some("listing" | "downloading" | "verifying")) {
            return Ok(false);
        }
        self.set_state(id, |s| *s = DownloadState { status: "listing".into(), ..Default::default() });
        let me = self.clone();
        let id2 = id.to_string();
        let h = tokio::spawn(async move {
            let res = me.run_download(&entry).await;
            me.handles.lock().unwrap().remove(&id2);
            match res {
                Ok(()) => me.set_state(&id2, |s| { s.status = "done".into(); s.error = None }),
                Err(e) => {
                    tracing::warn!("téléchargement {id2} : {e:#}");
                    me.set_state(&id2, |s| { s.status = "error".into(); s.error = Some(format!("{e:#}")) })
                }
            }
        });
        self.handles.lock().unwrap().insert(id.to_string(), h.abort_handle());
        Ok(true)
    }

    pub fn cancel(&self, id: &str) {
        if let Some(h) = self.handles.lock().unwrap().remove(id) {
            h.abort();
            self.set_state(id, |s| s.status = "cancelled".into());
        }
    }

    async fn run_download(&self, entry: &CatalogEntry) -> anyhow::Result<()> {
        // Premier dépôt (principal puis secours) qui existe ET contient le fichier voulu.
        let mut errors = Vec::new();
        let mut chosen = None;
        for repo in std::iter::once(&entry.repo).chain(entry.alt_repos.iter()) {
            let mut e = entry.clone();
            e.repo = repo.clone();
            match self.list_remote(&e).await.map_err(|x| x.to_string()).and_then(|l| select_files(&e, &l)) {
                Ok(files) => { chosen = Some((e, files)); break }
                Err(er) => errors.push(format!("{repo} : {er}")),
            }
        }
        let (entry, files) = chosen.ok_or_else(|| anyhow::anyhow!("aucun dépôt utilisable — {}", errors.join(" ; ")))?;
        let entry = &entry;
        let dir = self.dir_of(&entry.id);
        tokio::fs::create_dir_all(&dir).await?;
        let total: u64 = files.iter().map(|f| f.size).sum();
        self.set_state(&entry.id, |s| { s.status = "downloading".into(); s.total = total; s.downloaded = 0 });

        let mut done_before = 0u64;
        for f in &files {
            let name = f.basename().to_string();
            if name.contains("..") || name.is_empty() {
                anyhow::bail!("nom de fichier suspect : {}", f.path);
            }
            self.set_state(&entry.id, |s| s.file = name.clone());
            self.fetch_file(entry, f, &dir.join(&name), done_before).await?;
            done_before += f.size;
        }
        let manifest = Manifest {
            id: entry.id.clone(),
            repo: entry.repo.clone(),
            files: files.iter().map(|f| ManifestFile { name: f.basename().to_string(), size: f.size }).collect(),
        };
        tokio::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest)?).await?;
        Ok(())
    }

    async fn fetch_file(&self, entry: &CatalogEntry, f: &RemoteFile, dest: &Path, done_before: u64) -> anyhow::Result<()> {
        if tokio::fs::metadata(dest).await.map(|m| m.len() == f.size).unwrap_or(false) {
            self.set_state(&entry.id, |s| s.downloaded = done_before + f.size);
            return Ok(());
        }
        let part = PathBuf::from(format!("{}.part", dest.display()));
        let mut have = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
        if have > f.size {
            tokio::fs::remove_file(&part).await.ok();
            have = 0;
        }
        let url = format!("{}/{}/resolve/main/{}", self.hf_base, entry.repo, f.path);
        loop {
            let mut req = self.http.get(&url);
            if let Some(t) = &self.token { req = req.bearer_auth(t); }
            if have > 0 { req = req.header("Range", format!("bytes={have}-")); }
            let resp = req.send().await?;
            let status = resp.status().as_u16();
            let mut out = match status {
                206 => tokio::fs::OpenOptions::new().append(true).open(&part).await?,
                200 => { have = 0; tokio::fs::File::create(&part).await? } // le serveur ignore Range : on repart de zéro
                416 => { tokio::fs::remove_file(&part).await.ok(); have = 0; continue } // .part incohérent
                s => anyhow::bail!("téléchargement de {} : HTTP {s}", f.path),
            };
            let mut got = have;
            let mut stream = resp.bytes_stream();
            use futures_util::StreamExt;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                out.write_all(&chunk).await?;
                got += chunk.len() as u64;
                self.set_state(&entry.id, |s| s.downloaded = done_before + got);
            }
            out.flush().await?;
            drop(out);
            if got != f.size {
                anyhow::bail!("téléchargement incomplet de {} ({got}/{} octets) — relance pour reprendre", f.path, f.size);
            }
            break;
        }
        if let Some(expected) = &f.sha256 {
            self.set_state(&entry.id, |s| s.status = "verifying".into());
            let p = part.clone();
            let actual = tokio::task::spawn_blocking(move || sha256_file(&p)).await??;
            if !actual.eq_ignore_ascii_case(expected) {
                tokio::fs::remove_file(&part).await.ok();
                anyhow::bail!("somme SHA-256 incorrecte pour {} : fichier supprimé, relance le téléchargement", f.path);
            }
            self.set_state(&entry.id, |s| s.status = "downloading".into());
        }
        tokio::fs::rename(&part, dest).await?;
        Ok(())
    }
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rf(path: &str) -> RemoteFile {
        RemoteFile { path: path.into(), size: 10, sha256: None }
    }
    fn entry(files: &[&str], pattern: Option<&str>) -> CatalogEntry {
        CatalogEntry {
            id: "x".into(), kind: Kind::Llm, name: "x".into(), description: String::new(), repo: "a/b".into(),
            subdir: String::new(), files: files.iter().map(|s| s.to_string()).collect(),
            pattern: pattern.map(String::from), alt_repos: vec![], include_ext: vec![], size_gb: 0.0, args: vec![], exec: None, note: String::new(),
        }
    }

    #[test]
    fn analyse_la_reponse_de_l_api_hf() {
        let j = json!([
            {"type":"directory","path":"sub","oid":"1"},
            {"type":"file","path":"m-Q4_K_M.gguf","size":134,"oid":"git","lfs":{"oid":"abcd","size":7400000000u64,"pointerSize":134}},
            {"type":"file","path":"README.md","size":900,"oid":"g2"}
        ]);
        let t = parse_tree(&j);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].size, 7_400_000_000); // la taille LFS, pas celle du pointeur
        assert_eq!(t[0].sha256.as_deref(), Some("abcd"));
        assert_eq!(t[1].sha256, None);
    }

    #[test]
    fn selection_par_motif() {
        let l = vec![rf("m-Q4_K_M.gguf"), rf("m-Q5_K_M.gguf"), rf("mmproj-BF16.gguf"), rf("README.md")];
        let r = select_files(&entry(&[], Some("q4_k_m")), &l).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].path, "m-Q4_K_M.gguf");
    }

    #[test]
    fn selection_ambigue_est_refusee() {
        let l = vec![rf("a-Q4_K_M.gguf"), rf("b-Q4_K_M.gguf")];
        assert!(select_files(&entry(&[], Some("Q4_K_M")), &l).unwrap_err().contains("plusieurs fichiers"));
    }

    #[test]
    fn modele_en_plusieurs_morceaux() {
        let l = vec![rf("m-Q4_K_M-00001-of-00002.gguf"), rf("m-Q4_K_M-00002-of-00002.gguf")];
        assert_eq!(select_files(&entry(&[], Some("Q4_K_M")), &l).unwrap().len(), 2);
    }

    #[test]
    fn fichiers_exacts_et_absents() {
        let l = vec![rf("ggml-small.bin"), rf("ggml-small.en.bin")];
        assert_eq!(select_files(&entry(&["ggml-small.bin"], None), &l).unwrap()[0].path, "ggml-small.bin");
        assert!(select_files(&entry(&["ggml-tiny.bin"], None), &l).is_err());
    }

    #[test]
    fn selection_par_extensions() {
        let mut e = entry(&[], None);
        e.include_ext = vec!["safetensors".into(), "PT".into(), "json".into()];
        let l = vec![rf("ve.pt"), rf("t3.safetensors"), rf("tokenizer.json"), rf("README.md"), rf("sample.wav")];
        let r = select_files(&e, &l).unwrap();
        assert_eq!(r.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), vec!["t3.safetensors", "tokenizer.json", "ve.pt"]);
        e.include_ext = vec!["gguf".into()];
        assert!(select_files(&e, &l).is_err());
    }

    #[test]
    fn catalogue_ids_uniques() {
        let c = builtin_catalog();
        let mut ids: Vec<_> = c.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len());
        assert!(c.iter().all(|e| !e.files.is_empty() || e.pattern.is_some() || !e.include_ext.is_empty()));
        assert!(c.iter().any(|e| e.kind == Kind::Tts && e.exec.is_some()), "un TTS installable est proposé");
        // Tout modèle de texte annonce une taille (sinon l'interface le classerait comme « mini » à tort).
        assert!(c.iter().filter(|e| e.kind == Kind::Llm).all(|e| e.size_gb > 0.0));
        assert!(c.iter().all(|e| !e.name.is_empty() && !e.repo.is_empty() && e.repo.contains('/')));
        // Le modèle recommandé avec la voix a des dépôts de secours.
        assert!(!c.iter().find(|e| e.id == "gemma4-e4b-heretic").unwrap().alt_repos.is_empty());
        // Chatterbox : liste exacte des fichiers de from_local (pas le modèle anglais).
        let t = c.iter().find(|e| e.id == "chatterbox-multilingual").unwrap();
        assert!(t.files.iter().any(|f| f == "t3_mtl23ls_v2.safetensors") && !t.files.iter().any(|f| f.starts_with("t3_cfg")));
    }

    #[test]
    fn sha256_connu() {
        let dir = std::env::temp_dir().join(format!("vs-sha-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("f");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(sha256_file(&p).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn telechargement_reprise_et_verification_sur_serveur_local() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        // Faux « Hugging Face » : API de liste + fichier avec support de Range.
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let sha = {
            let mut h = Sha256::new();
            h.update(&body);
            h.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (b2, sha2) = (body.clone(), sha.clone());
        tokio::spawn(async move {
            loop {
                let (mut s, _) = listener.accept().await.unwrap();
                let (b, sha) = (b2.clone(), sha2.clone());
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = s.read(&mut buf).await.unwrap();
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    if req.starts_with("GET /api/models/a/b/tree/main") {
                        let j = format!(r#"[{{"type":"file","path":"m-Q4_K_M.gguf","size":134,"lfs":{{"oid":"{sha}","size":{}}}}}]"#, b.len());
                        let r = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{j}", j.len());
                        s.write_all(r.as_bytes()).await.unwrap();
                    } else {
                        let start = req.lines().find_map(|l| l.to_lowercase().strip_prefix("range: bytes=").map(|v| v.trim_end_matches('-').to_string()))
                            .and_then(|v| v.parse::<usize>().ok());
                        let (code, data) = match start { Some(st) => ("206 Partial Content", &b[st..]), None => ("200 OK", &b[..]) };
                        let h = format!("HTTP/1.1 {code}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len());
                        s.write_all(h.as_bytes()).await.unwrap();
                        s.write_all(data).await.unwrap();
                    }
                    let _ = s.shutdown().await;
                });
            }
        });

        let dir = std::env::temp_dir().join(format!("vs-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("HF_ENDPOINT", format!("http://{addr}"));
        let m = Models::new(vec![entry(&[], Some("Q4_K_M"))], dir.clone());
        // Un .part partiel existe déjà : le téléchargement doit reprendre à la suite.
        std::fs::create_dir_all(m.dir_of("x")).unwrap();
        std::fs::write(m.dir_of("x").join("m-Q4_K_M.gguf.part"), &body[..50_000]).unwrap();

        assert!(m.start_download("x").unwrap());
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if matches!(m.state("x").map(|s| s.status).as_deref(), Some("done" | "error")) { break; }
        }
        let st = m.state("x").unwrap();
        assert_eq!(st.status, "done", "{st:?}");
        let inst = m.installed("x").expect("installé");
        assert_eq!(std::fs::read(&inst.files[0]).unwrap(), body);
        std::fs::remove_dir_all(dir).ok();
    }
}
