use crate::config::Config;
use crate::engine::Engine;
use crate::hardware::LlmPlan;
use crate::llm::LlmClient;
use crate::media::MediaLibrary;
use crate::models::Models;
use crate::persona::PersonaStore;
use crate::voice::{Stt, Tts};
use std::sync::Arc;

pub struct AppState {
    pub cfg: Arc<Config>,
    pub media: MediaLibrary,
    pub personas: PersonaStore,
    pub llm: LlmClient,
    pub stt: Option<Stt>,
    pub tts: Option<Tts>,
    pub models: Arc<Models>,
    pub engine: Arc<Engine>,
}

impl AppState {
    /// Plan mémoire du LLM actuellement chargé (ou plan de repli de la configuration).
    pub fn plan(&self) -> LlmPlan {
        self.engine.llm_plan()
    }
}
