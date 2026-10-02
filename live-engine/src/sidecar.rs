//! Démarre `llama-server` avec les paramètres issus du planificateur matériel.

use crate::config::Config;
use crate::hardware::{llama_server_args, LlmPlan};
use tokio::process::{Child, Command};

pub fn spawn_llama(cfg: &Config, plan: &LlmPlan) -> anyhow::Result<Option<Child>> {
    if !cfg.llm.autostart {
        return Ok(None);
    }
    let model = cfg
        .llm
        .model_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("llm.autostart exige llm.model_path"))?;
    let port: u16 = cfg
        .llm
        .url
        .rsplit(':')
        .next()
        .and_then(|p| p.trim_end_matches('/').parse().ok())
        .unwrap_or(8080);
    let args = llama_server_args(plan, &model.to_string_lossy(), port, &cfg.llm.extra_args);
    tracing::info!("lancement : {} {}", cfg.llm.binary, args.join(" "));
    let child = Command::new(&cfg.llm.binary).args(&args).kill_on_drop(true).spawn()?;
    Ok(Some(child))
}
