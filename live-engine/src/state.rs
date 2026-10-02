use crate::config::Config;
use crate::hardware::LlmPlan;
use crate::llm::LlmClient;
use crate::media::MediaLibrary;
use crate::persona::PersonaStore;
use crate::voice::{Stt, Tts};

pub struct AppState {
    pub cfg: Config,
    pub plan: LlmPlan,
    pub media: MediaLibrary,
    pub personas: PersonaStore,
    pub llm: LlmClient,
    pub stt: Option<Stt>,
    pub tts: Option<Tts>,
}
