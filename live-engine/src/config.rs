//! Configuration (live.toml). Toutes les valeurs ont un défaut adapté à
//! 15 Go de VRAM / 64 Go de RAM.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
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
    /// Si défini, les routes d'écriture exigent `Authorization: Bearer <token>`.
    pub admin_token: Option<String>,
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
    /// Fichier GGUF (utilisé pour estimer la taille et pour le démarrage auto).
    pub model_path: Option<PathBuf>,
    /// Taille du modèle en Go si le fichier n'est pas encore présent.
    pub model_size_gb: f64,
    pub n_layers: u32,
    pub n_kv_heads: u32,
    pub head_dim: u32,
    pub ctx_tokens: u32,
    pub temperature: f32,
    /// Lance `llama-server` automatiquement avec les paramètres calculés.
    pub autostart: bool,
    pub binary: String,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SttConfig {
    pub enabled: bool,
    /// Serveur whisper.cpp (`whisper-server`), route `/inference`.
    pub url: String,
    pub language: String,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            hardware: HardwareConfig::default(),
            llm: LlmConfig::default(),
            stt: SttConfig::default(),
            tts: TtsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3001".into(),
            data_dir: "data".into(),
            uploads_dir: "../backend/uploads".into(),
            frontend_dir: "../frontend/dist".into(),
            admin_token: None,
        }
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
            model_path: None,
            model_size_gb: 8.7,
            n_layers: 40,
            n_kv_heads: 8,
            head_dim: 128,
            ctx_tokens: 16384,
            temperature: 0.85,
            autostart: false,
            binary: "llama-server".into(),
            extra_args: vec![],
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self { enabled: true, url: "http://127.0.0.1:8081".into(), language: "fr".into() }
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
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }
}
