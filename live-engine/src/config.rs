//! Configuration (live.toml). Toutes les valeurs ont un défaut adapté à
//! 15 Go de VRAM / 64 Go de RAM.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub engine: EngineConfig,
    pub hardware: HardwareConfig,
    pub llm: LlmConfig,
    pub stt: SttConfig,
    pub tts: TtsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind: String,
    pub data_dir: PathBuf,
    pub uploads_dir: PathBuf,
    pub frontend_dir: PathBuf,
    /// Dossier des modèles téléchargés (défaut : `<data_dir>/models`).
    pub models_dir: Option<PathBuf>,
    /// Dossier des exécutables (llama-server, whisper-server…). Sinon, recherche dans le PATH.
    pub bin_dir: PathBuf,
    /// Si défini, les routes d'écriture exigent `Authorization: Bearer <token>`.
    pub admin_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    /// L'app lance et arrête elle-même llama-server / whisper-server / TTS.
    /// Mets `false` si tu les gères à la main (voir `[llm].url`, `[stt].url`, `[tts].url`).
    pub managed: bool,
    /// Recharge au démarrage les derniers modèles choisis.
    pub autoload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HardwareConfig {
    pub vram_gb: f64,
    pub ram_gb: f64,
    /// VRAM laissée à l'OS / l'affichage / le navigateur.
    pub vram_reserve_gb: f64,
    /// VRAM réservée à d'autres modèles résidents (Whisper GPU, génération d'images…).
    pub vram_other_models_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// URL d'un serveur compatible OpenAI (llama.cpp `llama-server`, Ollama, vLLM…).
    pub url: String,
    pub model: String,
    /// Valeurs de repli quand aucun modèle géré n'est chargé (l'architecture réelle est lue dans le GGUF).
    pub model_size_gb: f64,
    pub n_layers: u32,
    pub n_kv_heads: u32,
    pub head_dim: u32,
    pub ctx_tokens: u32,
    pub temperature: f32,
    /// Arguments supplémentaires ajoutés à `llama-server` pour tous les modèles.
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SttConfig {
    pub enabled: bool,
    /// Serveur whisper.cpp (`whisper-server`), route `/inference`.
    pub url: String,
    pub language: String,
    /// `false` : whisper-server tourne sur CPU (libère ~1 Go de VRAM).
    pub use_gpu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TtsConfig {
    pub enabled: bool,
    /// Serveur compatible OpenAI `/v1/audio/speech` (Kokoro-FastAPI, Piper-HTTP, etc.).
    pub url: String,
    pub model: String,
    pub default_voice: String,
    pub format: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3001".into(),
            data_dir: "data".into(),
            uploads_dir: "../backend/uploads".into(),
            frontend_dir: "../frontend/dist".into(),
            models_dir: None,
            bin_dir: "bin".into(),
            admin_token: None,
        }
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self { managed: true, autoload: true }
    }
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self { vram_gb: 15.0, ram_gb: 64.0, vram_reserve_gb: 1.0, vram_other_models_gb: 1.0 }
    }
}

// Défaut : un 12B (Mistral-Nemo et dérivés) en Q5_K_M ≈ 8,7 Go, 40 couches, GQA 8 têtes KV.
impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:8080".into(),
            model: "local".into(),
            model_size_gb: 8.7,
            n_layers: 40,
            n_kv_heads: 8,
            head_dim: 128,
            ctx_tokens: 16384,
            temperature: 0.85,
            extra_args: vec![],
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self { enabled: true, url: "http://127.0.0.1:8081".into(), language: "fr".into(), use_gpu: true }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: "http://127.0.0.1:8880".into(),
            model: "kokoro".into(),
            default_voice: "ff_siwis".into(),
            format: "mp3".into(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let mut cfg: Self = if path.exists() {
            toml::from_str(&std::fs::read_to_string(path)?)?
        } else {
            Self::default()
        };
        cfg.apply_env();
        Ok(cfg)
    }

    /// Les variables d'environnement l'emportent (utilisées par l'app Electron empaquetée).
    fn apply_env(&mut self) {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        if let Some(v) = var("LIVE_BIND") { self.server.bind = v }
        if let Some(v) = var("LIVE_DATA_DIR") { self.server.data_dir = v.into() }
        if let Some(v) = var("LIVE_MODELS_DIR") { self.server.models_dir = Some(v.into()) }
        if let Some(v) = var("LIVE_BIN_DIR") { self.server.bin_dir = v.into() }
        if let Some(v) = var("LIVE_UPLOADS_DIR") { self.server.uploads_dir = v.into() }
        if let Some(v) = var("LIVE_FRONTEND_DIR") { self.server.frontend_dir = v.into() }
    }

    pub fn models_dir(&self) -> PathBuf {
        self.server.models_dir.clone().unwrap_or_else(|| self.server.data_dir.join("models"))
    }
}
