//! Client LLM : API OpenAI-compatible (llama.cpp `llama-server`, Ollama, vLLM…), en streaming.

use futures_util::{stream, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::VecDeque;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

impl ChatMsg {
    pub fn new(role: &str, content: impl Into<String>) -> Self {
        Self { role: role.into(), content: content.into() }
    }
}

#[derive(Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    base: String,
    model: String,
}

type ByteStream = Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>;

/// Extrait le texte d'une ligne SSE `data: {...}`. `None` = rien à émettre ; `Some(None)` = fin.
fn parse_sse_line(line: &str) -> Option<Option<String>> {
    let line = line.trim();
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(None);
    }
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    let tok = v["choices"][0]["delta"]["content"].as_str()?;
    if tok.is_empty() { None } else { Some(Some(tok.to_string())) }
}

struct SseState {
    bytes: ByteStream,
    buf: Vec<u8>,
    pending: VecDeque<String>,
    done: bool,
}

impl LlmClient {
    pub fn new(base: &str, model: &str) -> Self {
        Self { http: reqwest::Client::new(), base: base.trim_end_matches('/').into(), model: model.into() }
    }

    pub async fn healthy(&self) -> bool {
        for path in ["/health", "/v1/models"] {
            if let Ok(r) = self.http.get(format!("{}{}", self.base, path)).send().await {
                if r.status().is_success() { return true; }
            }
        }
        false
    }

    fn body(&self, messages: &[ChatMsg], temperature: f32, stream: bool, max_tokens: Option<u32>) -> serde_json::Value {
        let mut b = json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "stream": stream,
            "top_p": 0.95,
            "min_p": 0.05,
            "repeat_penalty": 1.08,
        });
        if let Some(m) = max_tokens { b["max_tokens"] = json!(m); }
        b
    }

    /// Flux de jetons de texte.
    pub async fn stream(
        &self,
        messages: &[ChatMsg],
        temperature: f32,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<String>>> {
        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base))
            .json(&self.body(messages, temperature, true, None))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("LLM : HTTP {} {}", resp.status(), resp.text().await.unwrap_or_default());
        }
        let st = SseState { bytes: Box::pin(resp.bytes_stream()), buf: vec![], pending: VecDeque::new(), done: false };
        Ok(stream::unfold(st, |mut st| async move {
            loop {
                if let Some(t) = st.pending.pop_front() {
                    return Some((Ok(t), st));
                }
                if st.done {
                    return None;
                }
                if let Some(pos) = st.buf.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = st.buf.drain(..=pos).collect();
                    match parse_sse_line(&String::from_utf8_lossy(&line)) {
                        Some(Some(t)) => st.pending.push_back(t),
                        Some(None) => st.done = true,
                        None => {}
                    }
                    continue;
                }
                match st.bytes.next().await {
                    Some(Ok(b)) => st.buf.extend_from_slice(&b),
                    Some(Err(e)) => {
                        st.done = true;
                        return Some((Err(e.into()), st));
                    }
                    None => st.done = true,
                }
            }
        }))
    }

    /// Réponse complète (utilisé pour résumer l'historique).
    pub async fn complete(&self, messages: &[ChatMsg], temperature: f32, max_tokens: u32) -> anyhow::Result<String> {
        let v: serde_json::Value = self
            .http
            .post(format!("{}/v1/chat/completions", self.base))
            .json(&self.body(messages, temperature, false, Some(max_tokens)))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(v["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lignes_sse() {
        assert_eq!(parse_sse_line(r#"data: {"choices":[{"delta":{"content":"Salut"}}]}"#), Some(Some("Salut".into())));
        assert_eq!(parse_sse_line("data: [DONE]"), Some(None));
        assert_eq!(parse_sse_line(r#"data: {"choices":[{"delta":{}}]}"#), None);
        assert_eq!(parse_sse_line(": keep-alive"), None);
    }
}
