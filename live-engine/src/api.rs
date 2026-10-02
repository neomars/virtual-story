//! Routes REST d'administration + point d'entrée WebSocket.

use crate::media::MediaPatch;
use crate::models::Kind;
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
        .route("/plan", get(|State(a): State<App>| async move { Json(json!(a.plan())) }))
        .route("/models", get(models_list))
        .route("/models/{id}/download", post(model_download))
        .route("/models/{id}/cancel", post(model_cancel))
        .route("/models/{id}", axum::routing::delete(model_delete))
        .route("/engine/status", get(engine_status))
        .route("/engine/load", post(engine_load))
        .route("/engine/unload", post(engine_unload))
        .route("/engine/logs/{kind}", get(engine_logs))
        .route("/ws", get(ws_upgrade))
        .route("/media", get(media_list))
        .route("/media/scan", post(media_scan))
        .route("/media/import-legacy", post(media_import_legacy))
        .route("/media/bulk", post(media_bulk))
        .route("/media/{id}", axum::routing::patch(media_patch))
        .route("/personas", get(personas_list))
        .route("/personas/{id}", get(persona_get).put(persona_put).delete(persona_delete))
}

/// Un site web quelconque ne doit pas pouvoir ouvrir une session sur ton moteur local (et ton GPU) :
/// on n'accepte que les origines locales ou identiques à l'hôte demandé. Sans en-tête Origin
/// (client non-navigateur), on accepte.
pub fn origin_allowed(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    let after = origin.split_once("://").map(|(_, r)| r).unwrap_or(origin);
    let origin_host = after.split('/').next().unwrap_or("");
    let hostname = |h: &str| -> String {
        if let Some(rest) = h.strip_prefix('[') { format!("[{}", rest.split(']').next().unwrap_or("") ) + "]" }
        else { h.split(':').next().unwrap_or("").to_lowercase() }
    };
    let oh = hostname(origin_host);
    matches!(oh.as_str(), "localhost" | "127.0.0.1" | "[::1]") || host.is_some_and(|h| hostname(h) == oh)
}

async fn ws_upgrade(ws: WebSocketUpgrade, headers: HeaderMap, State(app): State<App>) -> axum::response::Response {
    let get = |k: &str| headers.get(k).and_then(|v| v.to_str().ok());
    if !origin_allowed(get("origin"), get("host")) {
        return (StatusCode::FORBIDDEN, "origine non autorisée").into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, app)).into_response()
}

async fn health(State(a): State<App>) -> Json<Value> {
    let stt = match &a.stt { Some(s) => s.healthy().await, None => false };
    let tts = match &a.tts { Some(t) => t.healthy().await, None => false };
    Json(json!({
        "llm": a.llm.healthy().await, "stt": stt, "tts": tts,
        "plan": a.plan(),
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

// ---------- Modèles et moteur ----------

async fn models_list(State(a): State<App>) -> Json<Value> {
    let sel = a.engine.selection();
    let items: Vec<Value> = a
        .models
        .catalog
        .iter()
        .map(|e| {
            let inst = a.models.installed(&e.id);
            let active = [&sel.llm, &sel.stt, &sel.tts].iter().any(|s| s.as_deref() == Some(e.id.as_str()));
            json!({
                "entry": e,
                "installed": inst.is_some(),
                "size_bytes": inst.as_ref().map(|i| i.size_bytes),
                "download": a.models.state(&e.id),
                "active": active,
            })
        })
        .collect();
    Json(json!(items))
}

async fn model_download(State(a): State<App>, h: HeaderMap, Path(id): Path<String>) -> ApiResult {
    authorize(&a, &h)?;
    let started = a.models.start_download(&id).map_err(|e| err(StatusCode::NOT_FOUND, e))?;
    Ok(Json(json!({ "started": started })))
}

async fn model_cancel(State(a): State<App>, h: HeaderMap, Path(id): Path<String>) -> ApiResult {
    authorize(&a, &h)?;
    a.models.cancel(&id);
    Ok(Json(json!({ "cancelled": id })))
}

async fn model_delete(State(a): State<App>, h: HeaderMap, Path(id): Path<String>) -> ApiResult {
    authorize(&a, &h)?;
    // On décharge d'abord le modèle s'il est en cours d'utilisation (fichier ouvert par le serveur).
    if let Some(e) = a.models.entry(&id) {
        let kind = e.kind;
        if a.engine.status_of(kind).model.as_deref() == Some(id.as_str()) {
            a.engine.unload(kind).await;
        }
    }
    if a.models.delete(&id) { Ok(Json(json!({ "deleted": id }))) } else { Err(err(StatusCode::NOT_FOUND, "modèle introuvable")) }
}

async fn engine_status(State(a): State<App>) -> Json<Value> {
    Json(a.engine.status_json().await)
}

#[derive(Deserialize)]
struct LoadReq {
    id: String,
}

async fn engine_load(State(a): State<App>, h: HeaderMap, Json(r): Json<LoadReq>) -> ApiResult {
    authorize(&a, &h)?;
    a.engine.load(&r.id).await.map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({ "loading": r.id })))
}

#[derive(Deserialize)]
struct UnloadReq {
    kind: Kind,
}

async fn engine_unload(State(a): State<App>, h: HeaderMap, Json(r): Json<UnloadReq>) -> ApiResult {
    authorize(&a, &h)?;
    a.engine.unload(r.kind).await;
    Ok(Json(json!({ "unloaded": r.kind })))
}

async fn engine_logs(State(a): State<App>, Path(kind): Path<String>) -> ApiResult {
    let k = match kind.as_str() {
        "llm" => Kind::Llm,
        "stt" => Kind::Stt,
        "tts" => Kind::Tts,
        _ => return Err(err(StatusCode::NOT_FOUND, "composant inconnu")),
    };
    Ok(Json(json!({ "lines": a.engine.logs(k, 200) })))
}

#[cfg(test)]
mod tests {
    use super::origin_allowed;

    #[test]
    fn origines_locales_acceptees() {
        assert!(origin_allowed(None, Some("127.0.0.1:3001")));
        assert!(origin_allowed(Some("http://localhost:5173"), Some("127.0.0.1:3001")));
        assert!(origin_allowed(Some("http://127.0.0.1:3000"), Some("127.0.0.1:3001")));
        assert!(origin_allowed(Some("http://[::1]:3000"), Some("[::1]:3001")));
        assert!(origin_allowed(Some("http://mon-pc.local:3000"), Some("mon-pc.local:3001")));
    }

    #[test]
    fn sites_externes_refuses() {
        assert!(!origin_allowed(Some("https://evil.example"), Some("127.0.0.1:3001")));
        assert!(!origin_allowed(Some("http://localhost.evil.example"), Some("127.0.0.1:3001")));
    }
}
