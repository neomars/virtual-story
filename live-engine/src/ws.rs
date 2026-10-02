//! Session live (WebSocket) : texte ou voix en entrée ; texte, médias, réponses rapides
//! et audio synthétisé en sortie, le tout en streaming.
//!
//! Protocole client → serveur (JSON texte) :
//!   {"type":"start","persona":"id","max_intensity":3,"tts":true}
//!   {"type":"user","text":"..."}
//!   {"type":"interrupt"}
//!   {"type":"settings","max_intensity":3,"tts":false}
//! et en binaire : un WAV 16 kHz mono (parole de l'utilisateur).
//!
//! Serveur → client (JSON) : hello, transcript, token, media, replies, state,
//! audio (suivi d'une trame binaire), done, interrupted, error.

use crate::director::{speakable, Directive, Piece, SentenceSplitter, StreamParser};
use crate::llm::ChatMsg;
use crate::media::SearchQuery;
use crate::persona::Persona;
use crate::prompt::{estimate_tokens, system_prompt};
use crate::state::AppState;
use crate::voice::TtsStyle;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{stream, Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::{AbortHandle, JoinHandle};

type Out = mpsc::UnboundedSender<Message>;
type TokenStream = Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>;

struct Session {
    persona: Option<Persona>,
    history: Vec<ChatMsg>,
    summary: String,
    shown: HashSet<i64>,
    story: BTreeMap<String, String>,
    max_intensity: u8,
    tts_on: bool,
    gen_id: u64,
    handles: Vec<AbortHandle>,
}

fn send(out: &Out, v: Value) {
    let _ = out.send(Message::Text(v.to_string().into()));
}

pub async fn handle_socket(socket: WebSocket, app: Arc<AppState>) {
    let (mut sink, mut source) = socket.split();
    let (out, mut rx) = mpsc::unbounded_channel::<Message>();

    let writer = tokio::spawn(async move {
        use futures_util::SinkExt;
        while let Some(m) = rx.recv().await {
            if sink.send(m).await.is_err() {
                break;
            }
        }
    });

    let session = Arc::new(Mutex::new(Session {
        persona: None,
        history: vec![],
        summary: String::new(),
        shown: HashSet::new(),
        story: BTreeMap::new(),
        max_intensity: 5,
        tts_on: app.tts.is_some(),
        gen_id: 0,
        handles: vec![],
    }));

    let (llm_ok, stt_ok, tts_ok) = (
        app.llm.healthy().await,
        match &app.stt { Some(s) => s.healthy().await, None => false },
        match &app.tts { Some(t) => t.healthy().await, None => false },
    );
    send(&out, json!({"type":"hello","llm":llm_ok,"stt":stt_ok,"tts":tts_ok,
                      "plan": app.plan()}));

    while let Some(Ok(msg)) = source.next().await {
        match msg {
            Message::Text(t) => {
                let Ok(v) = serde_json::from_str::<Value>(t.as_str()) else { continue };
                match v["type"].as_str().unwrap_or("") {
                    "start" => on_start(&app, &session, &out, &v).await,
                    "user" => {
                        let text = v["text"].as_str().unwrap_or("").trim().to_string();
                        if !text.is_empty() {
                            start_turn(&app, &session, &out, Some(text)).await;
                        }
                    }
                    "interrupt" => {
                        interrupt(&session).await;
                        send(&out, json!({"type":"interrupted"}));
                    }
                    "settings" => {
                        let mut s = session.lock().await;
                        if let Some(i) = v["max_intensity"].as_u64() {
                            s.max_intensity = (i as u8).clamp(1, 5);
                        }
                        if let Some(b) = v["tts"].as_bool() {
                            s.tts_on = b && app.tts.is_some();
                        }
                    }
                    _ => {}
                }
            }
            Message::Binary(wav) => {
                let Some(stt) = app.stt.clone() else {
                    send(&out, json!({"type":"error","message":"reconnaissance vocale désactivée"}));
                    continue;
                };
                // Barge-in : l'utilisateur parle, on coupe l'IA.
                interrupt(&session).await;
                match stt.transcribe(wav.to_vec()).await {
                    Ok(text) if !text.trim().is_empty() => {
                        send(&out, json!({"type":"transcript","text":text}));
                        start_turn(&app, &session, &out, Some(text)).await;
                    }
                    Ok(_) => send(&out, json!({"type":"transcript","text":""})),
                    Err(e) => send(&out, json!({"type":"error","message":format!("STT : {e}")})),
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    interrupt(&session).await;
    writer.abort();
}

async fn interrupt(session: &Arc<Mutex<Session>>) {
    let mut s = session.lock().await;
    s.gen_id += 1;
    for h in s.handles.drain(..) {
        h.abort();
    }
}

async fn on_start(app: &Arc<AppState>, session: &Arc<Mutex<Session>>, out: &Out, v: &Value) {
    let id = v["persona"].as_str().unwrap_or("");
    let Some(persona) = app.personas.get(id) else {
        send(out, json!({"type":"error","message":"persona introuvable ou invalide"}));
        return;
    };
    interrupt(session).await;
    {
        let mut s = session.lock().await;
        s.history.clear();
        s.summary.clear();
        s.shown.clear();
        s.story.clear();
        if let Some(i) = v["max_intensity"].as_u64() {
            s.max_intensity = (i as u8).clamp(1, 5);
        }
        if let Some(b) = v["tts"].as_bool() {
            s.tts_on = b && app.tts.is_some();
        }
        s.persona = Some(persona.clone());
    }
    send(out, json!({"type":"persona","persona":persona}));
    if persona.first_message.trim().is_empty() {
        // Le personnage ouvre la conversation lui-même.
        start_turn(app, session, out, None).await;
    } else {
        let text = persona.first_message.clone();
        launch(app, session, out, Box::pin(stream::iter(vec![Ok(text)]))).await;
    }
}

/// Ajoute le message utilisateur (s'il y en a un), compacte l'historique, puis lance la génération.
async fn start_turn(app: &Arc<AppState>, session: &Arc<Mutex<Session>>, out: &Out, user: Option<String>) {
    interrupt(session).await;
    {
        let mut s = session.lock().await;
        if s.persona.is_none() {
            send(out, json!({"type":"error","message":"aucun persona sélectionné"}));
            return;
        }
        match user {
            Some(u) => s.history.push(ChatMsg::new("user", u)),
            None => s.history.push(ChatMsg::new(
                "user",
                "(L'utilisateur vient d'arriver. Accueille-le dans ton style, en une ou deux phrases.)",
            )),
        }
    }
    compact(app, session).await;

    let (messages, temp) = {
        let s = session.lock().await;
        let p = s.persona.clone().unwrap();
        let vocab = app.media.vocabulary(s.max_intensity, 60);
        let sys = system_prompt(&p, &vocab, s.max_intensity, &s.summary, &s.story);
        let mut m = vec![ChatMsg::new("system", sys)];
        m.extend(s.history.iter().cloned());
        (m, p.temperature.unwrap_or(app.cfg.llm.temperature))
    };
    match app.llm.stream(&messages, temp).await {
        Ok(st) => launch(app, session, out, Box::pin(st)).await,
        Err(e) => send(out, json!({"type":"error","message":format!("LLM : {e}")})),
    }
}

/// Résume les plus anciens tours quand le contexte approche de sa limite.
async fn compact(app: &Arc<AppState>, session: &Arc<Mutex<Session>>) {
    let to_summarize = {
        let s = session.lock().await;
        let used: usize = s.history.iter().map(|m| estimate_tokens(&m.content)).sum::<usize>()
            + estimate_tokens(&s.summary) + 1500; // marge : prompt système
        if used as f64 > app.plan().ctx_tokens as f64 * 0.75 && s.history.len() > 6 {
            let n = s.history.len() / 2;
            Some((s.history[..n].to_vec(), s.summary.clone(), n))
        } else {
            None
        }
    };
    let Some((old, prev, n)) = to_summarize else { return };
    let transcript = old.iter().map(|m| format!("{}: {}", m.role, m.content)).collect::<Vec<_>>().join("\n");
    let ask = vec![
        ChatMsg::new("system", "Tu résumes des conversations de façon factuelle et compacte (≤ 200 mots), en gardant : \
            faits établis, relation entre les personnages, lieux, objectifs en cours, promesses. Aucune moralisation."),
        ChatMsg::new("user", format!("Résumé précédent :\n{prev}\n\nNouveaux échanges :\n{transcript}\n\nNouveau résumé :")),
    ];
    let summary = app.llm.complete(&ask, 0.3, 400).await.unwrap_or(prev);
    let mut s = session.lock().await;
    let n = n.min(s.history.len());
    s.history.drain(..n);
    s.summary = summary;
}

async fn launch(app: &Arc<AppState>, session: &Arc<Mutex<Session>>, out: &Out, tokens: TokenStream) {
    let gen_id = {
        let mut s = session.lock().await;
        s.gen_id += 1;
        s.gen_id
    };
    let (app2, session2, out2) = (app.clone(), session.clone(), out.clone());
    let (tts_tx, mut tts_rx) = mpsc::unbounded_channel::<(u32, JoinHandle<Option<Vec<u8>>>)>();

    // Expéditeur audio : envoie les phrases synthétisées dans l'ordre, même si elles se terminent dans le désordre.
    let out_audio = out.clone();
    let mime = app.tts.as_ref().map(|t| t.mime()).unwrap_or("audio/mpeg");
    let audio_task = tokio::spawn(async move {
        while let Some((seq, h)) = tts_rx.recv().await {
            if let Ok(Some(bytes)) = h.await {
                send(&out_audio, json!({"type":"audio","gen":gen_id,"seq":seq,"mime":mime}));
                let _ = out_audio.send(Message::Binary(bytes.into()));
            }
        }
    });

    let gen_task = tokio::spawn(async move {
        run_generation(app2, session2, out2, tokens, gen_id, tts_tx).await;
    });
    let mut s = session.lock().await;
    s.handles.push(gen_task.abort_handle());
    s.handles.push(audio_task.abort_handle());
}

async fn run_generation(
    app: Arc<AppState>,
    session: Arc<Mutex<Session>>,
    out: Out,
    mut tokens: TokenStream,
    gen_id: u64,
    tts_tx: mpsc::UnboundedSender<(u32, JoinHandle<Option<Vec<u8>>>)>,
) {
    let mut parser = StreamParser::default();
    let mut splitter = SentenceSplitter::default();
    let mut raw = String::new();
    let mut seq: u32 = 0;
    let (tts_on, voice, style) = {
        let s = session.lock().await;
        let p = s.persona.as_ref();
        (
            s.tts_on,
            p.and_then(|p| p.voice.clone()),
            TtsStyle { exaggeration: p.and_then(|p| p.tts_exaggeration), cfg_weight: p.and_then(|p| p.tts_cfg_weight) },
        )
    };

    let speak = |sentence: String, seq: &mut u32| {
        if !tts_on { return; }
        let Some(tts) = app.tts.clone() else { return };
        let text = speakable(&sentence);
        if text.is_empty() { return; }
        let voice = voice.clone();
        let h = tokio::spawn(async move {
            match tts.speak(&text, voice.as_deref(), &style).await {
                Ok(b) => Some(b),
                Err(e) => { tracing::warn!("TTS : {e}"); None }
            }
        });
        let _ = tts_tx.send((*seq, h));
        *seq += 1;
    };

    let mut handle_pieces = |pieces: Vec<Piece>, seq: &mut u32, raw: &mut String| {
        let mut directives = vec![];
        for p in pieces {
            match p {
                Piece::Text(t) => {
                    raw.push_str(&t);
                    send(&out, json!({"type":"token","gen":gen_id,"text":t}));
                    for sentence in splitter.feed(&t) {
                        speak(sentence, seq);
                    }
                }
                Piece::Directive(d) => directives.push(d),
            }
        }
        directives
    };

    let mut stream_error: Option<String> = None;
    while let Some(tok) = tokens.next().await {
        match tok {
            Ok(t) => {
                let pieces = parser.feed(&t);
                let ds = handle_pieces(pieces, &mut seq, &mut raw);
                apply_directives(&app, &session, &out, ds).await;
            }
            Err(e) => { stream_error = Some(e.to_string()); break; }
        }
    }
    let pieces = parser.finish();
    let ds = handle_pieces(pieces, &mut seq, &mut raw);
    apply_directives(&app, &session, &out, ds).await;
    if let Some(rest) = splitter.finish() {
        speak(rest, &mut seq);
    }

    if let Some(e) = stream_error {
        send(&out, json!({"type":"error","message":format!("LLM : {e}")}));
    }
    {
        let mut s = session.lock().await;
        if s.gen_id == gen_id && !raw.trim().is_empty() {
            s.history.push(ChatMsg::new("assistant", raw.trim().to_string()));
        }
    }
    send(&out, json!({"type":"done","gen":gen_id}));
}

async fn apply_directives(app: &Arc<AppState>, session: &Arc<Mutex<Session>>, out: &Out, ds: Vec<Directive>) {
    for d in ds {
        match d {
            Directive::Show { tags, mood, intensity, media_kind } => {
                let mut s = session.lock().await;
                let cap = s.max_intensity;
                let q = SearchQuery {
                    tags, mood, kind: media_kind,
                    intensity: intensity.map(|i| i.min(cap)),
                    max_intensity: cap,
                    exclude: s.shown.clone(),
                    ..Default::default()
                };
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as u64).unwrap_or(0);
                match app.media.pick(&q, seed) {
                    Some(m) => {
                        s.shown.insert(m.id);
                        send(out, json!({"type":"media","mode":"main","item":m}));
                    }
                    None => send(out, json!({"type":"media_miss"})),
                }
            }
            Directive::Ambient { tags, mood } => {
                let s = session.lock().await;
                let q = SearchQuery { tags, mood, ambient_only: true, max_intensity: s.max_intensity, ..Default::default() };
                if let Some(m) = app.media.pick(&q, 0) {
                    send(out, json!({"type":"media","mode":"ambient","item":m}));
                }
            }
            Directive::Replies { items } => send(out, json!({"type":"replies","items":items})),
            Directive::State { key, value } => {
                let mut s = session.lock().await;
                s.story.insert(key.clone(), value.clone());
                send(out, json!({"type":"state","key":key,"value":value}));
            }
        }
    }
}
