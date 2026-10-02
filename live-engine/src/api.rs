//! Routes REST d'administration + point d'entrée WebSocket.

use crate::media::MediaPatch;
use crate::persona::Persona;
use crate::state::AppState;
use crate::ws::handle_socket;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

type App = Arc<AppState>;
type ApiResult = Result<Json<Value>, (StatusCode, Json<Value>)>;

fn err(code: StatusCode, msg: impl ToString) -> (StatusCode, Json<Value>) {
    (code, Json(json!({ "message": msg.to_string() })))
}

fn authorize(app: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
    let Some(token) = &app.cfg.server.admin_token else { return Ok(()) };
    let ok = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|t| t == token);
    if ok { Ok(()) } else { Err(err(StatusCode::UNAUTHORIZED, "jeton admin requis")) }
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/health", get(health))
        .route("/plan", get(|State(a): State<App>| async move { Json(json!(a.plan)) }))
        .route("/ws", get(ws_upgrade))
        .route("/media", get(media_list))
        .route("/media/scan", post(media_scan))
        .route("/media/import-legacy", post(media_import_legacy))
        .route("/media/bulk", post(media_bulk))
        .route("/media/{id}", axum::routing::patch(media_patch))
        .route("/personas", get(personas_list))
        .route("/personas/{id}", get(persona_get).put(persona_put).delete(persona_delete))
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(app): State<App>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn health(State(a): State<App>) -> Json<Value> {
    let stt = match &a.stt { Some(s) => s.healthy().await, None => false };
    let tts = match &a.tts { Some(t) => t.healthy().await, None => false };
    Json(json!({
        "llm": a.llm.healthy().await, "stt": stt, "tts": tts,
        "plan": a.plan,
        "media_count": a.media.list().map(|l| l.len()).unwrap_or(0),
    }))
}

async fn media_list(State(a): State<App>) -> ApiResult {
    let items = a.media.list().map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!(items)))
}

async fn media_patch(State(a): State<App>, h: HeaderMap, Path(id): Path<i64>, Json(p): Json<MediaPatch>) -> ApiResult {
    authorize(&a, &h)?;
    match a.media.patch(id, &p).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))? {
        Some(m) => Ok(Json(json!(m))),
        None => Err(err(StatusCode::NOT_FOUND, "média introuvable")),
    }
}

#[derive(Deserialize)]
struct Bulk {
    ids: Vec<i64>,
    #[serde(flatten)]
    patch: MediaPatch,
}

async fn media_bulk(State(a): State<App>, h: HeaderMap, Json(b): Json<Bulk>) -> ApiResult {
    authorize(&a, &h)?;
    let mut n = 0;
    for id in b.ids {
        if a.media.patch(id, &b.patch).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?.is_some() {
            n += 1;
        }
    }
    Ok(Json(json!({ "updated": n })))
}

async fn media_scan(State(a): State<App>, h: HeaderMap) -> ApiResult {
    authorize(&a, &h)?;
    let n = a.media.scan_uploads(&a.cfg.server.uploads_dir).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "added": n })))
}

async fn media_import_legacy(State(a): State<App>, h: HeaderMap) -> ApiResult {
    authorize(&a, &h)?;
    let path = a.cfg.server.uploads_dir.join("../db.json");
    let n = a.media.import_legacy(&path).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({ "added": n })))
}

async fn personas_list(State(a): State<App>) -> Json<Value> {
    Json(json!(a.personas.list()))
}

async fn persona_get(State(a): State<App>, Path(id): Path<String>) -> ApiResult {
    a.personas.get(&id).map(|p| Json(json!(p))).ok_or_else(|| err(StatusCode::NOT_FOUND, "persona introuvable"))
}

async fn persona_put(State(a): State<App>, h: HeaderMap, Path(id): Path<String>, Json(mut p): Json<Persona>) -> ApiResult {
    authorize(&a, &h)?;
    p.id = id;
    a.personas.save(&p).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!(p)))
}

async fn persona_delete(State(a): State<App>, h: HeaderMap, Path(id): Path<String>) -> ApiResult {
    authorize(&a, &h)?;
    if a.personas.delete(&id) { Ok(Json(json!({"deleted": id}))) } else { Err(err(StatusCode::NOT_FOUND, "persona introuvable")) }
}
