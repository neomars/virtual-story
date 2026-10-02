mod api;
mod config;
mod director;
mod hardware;
mod llm;
mod media;
mod persona;
mod prompt;
mod sidecar;
mod state;
mod voice;
mod ws;

use crate::config::Config;
use crate::state::AppState;
use axum::Router;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg_path = PathBuf::from(std::env::var("LIVE_CONFIG").unwrap_or_else(|_| "live.toml".into()));
    let cfg = Config::load(&cfg_path)?;

    // Taille réelle du modèle si le fichier existe, sinon la valeur configurée.
    let weights_gb = cfg
        .llm
        .model_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len() as f64 / (1024.0 * 1024.0 * 1024.0))
        .unwrap_or(cfg.llm.model_size_gb);
    let plan = hardware::plan(&cfg.hardware, &cfg.llm, weights_gb);

    if std::env::args().nth(1).as_deref() == Some("plan") {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        println!("\nlignes de commande llama-server :");
        let model = cfg.llm.model_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or("model.gguf".into());
        let quoted: Vec<String> = hardware::llama_server_args(&plan, &model, 8080, &cfg.llm.extra_args)
            .into_iter()
            .map(|a| if a.contains(|c: char| c.is_whitespace() || "{}\"'$".contains(c)) { format!("'{a}'") } else { a })
            .collect();
        println!("llama-server {}", quoted.join(" "));
        return Ok(());
    }

    tracing::info!(
        "plan GPU : {}/{} couches, ctx {}, VRAM ≈ {:.1}/{:.1} Go, RAM ≈ {:.1} Go {}",
        plan.n_gpu_layers.min(cfg.llm.n_layers), cfg.llm.n_layers, plan.ctx_tokens, plan.vram_used_gb, plan.vram_budget_gb,
        plan.ram_used_gb, plan.notes.join(" ; ")
    );
    let _llama = sidecar::spawn_llama(&cfg, &plan)?; // gardé en vie jusqu'à la fin du processus

    let media = media::MediaLibrary::open(&cfg.server.data_dir.join("live.db"))?;
    let personas = persona::PersonaStore::new(&cfg.server.data_dir.join("personas"))?;
    let bind = cfg.server.bind.clone();
    let uploads = cfg.server.uploads_dir.clone();
    let frontend = cfg.server.frontend_dir.clone();

    let state = Arc::new(AppState {
        llm: llm::LlmClient::new(&cfg.llm.url, &cfg.llm.model),
        stt: cfg.stt.enabled.then(|| voice::Stt::new(cfg.stt.clone())),
        tts: cfg.tts.enabled.then(|| voice::Tts::new(cfg.tts.clone())),
        plan,
        media,
        personas,
        cfg,
    });

    // /api/live/*  → moteur ; le reste → médias uploadés, puis frontend compilé (SPA).
    let spa = ServeDir::new(&frontend).fallback(ServeFile::new(frontend.join("index.html")));
    let app = Router::new()
        .nest("/api/live", api::router())
        .fallback_service(ServeDir::new(&uploads).fallback(spa))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("Virtual Story Live — http://{bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
