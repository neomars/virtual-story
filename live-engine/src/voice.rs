//! Voix : reconnaissance (whisper.cpp `whisper-server`) et synthèse (API OpenAI `/v1/audio/speech`).

use crate::config::{SttConfig, TtsConfig};
use serde_json::json;

#[derive(Clone)]
pub struct Stt {
    http: reqwest::Client,
    cfg: SttConfig,
}

impl Stt {
    pub fn new(cfg: SttConfig) -> Self {
        Self { http: reqwest::Client::new(), cfg }
    }

    /// `wav` : PCM 16 kHz mono (le navigateur l'encode avant l'envoi).
    pub async fn transcribe(&self, wav: Vec<u8>) -> anyhow::Result<String> {
        let part = reqwest::multipart::Part::bytes(wav).file_name("audio.wav").mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("response_format", "json")
            .text("temperature", "0.0")
            .text("language", self.cfg.language.clone());
        let v: serde_json::Value = self
            .http
            .post(format!("{}/inference", self.cfg.url.trim_end_matches('/')))
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(v["text"].as_str().unwrap_or("").trim().to_string())
    }

    pub async fn healthy(&self) -> bool {
        self.http.get(&self.cfg.url).send().await.is_ok()
    }
}

#[derive(Clone)]
pub struct Tts {
    http: reqwest::Client,
    cfg: TtsConfig,
}

impl Tts {
    pub fn new(cfg: TtsConfig) -> Self {
        Self { http: reqwest::Client::new(), cfg }
    }

    pub fn mime(&self) -> &'static str {
        match self.cfg.format.as_str() {
            "wav" => "audio/wav",
            "opus" => "audio/ogg",
            "aac" => "audio/aac",
            "flac" => "audio/flac",
            _ => "audio/mpeg",
        }
    }

    pub async fn speak(&self, text: &str, voice: Option<&str>) -> anyhow::Result<Vec<u8>> {
        let bytes = self
            .http
            .post(format!("{}/v1/audio/speech", self.cfg.url.trim_end_matches('/')))
            .json(&json!({
                "model": self.cfg.model,
                "input": text,
                "voice": voice.unwrap_or(&self.cfg.default_voice),
                "response_format": self.cfg.format,
            }))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }

    pub async fn healthy(&self) -> bool {
        self.http.get(&self.cfg.url).send().await.is_ok()
    }
}
