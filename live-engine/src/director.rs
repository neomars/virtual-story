//! « Réalisateur » : le LLM écrit du texte normal avec des directives inline,
//! par exemple `[[show: plage, soleil | mood=joyeux | intensity=2]]`.
//! Ce format est bien plus fiable que le function-calling sur des modèles locaux,
//! et permet de précharger le média dès que la directive apparaît dans le flux.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Directive {
    Show { tags: Vec<String>, mood: Option<String>, intensity: Option<u8>, media_kind: Option<String> },
    Ambient { tags: Vec<String>, mood: Option<String> },
    Replies { items: Vec<String> },
    State { key: String, value: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    Text(String),
    Directive(Directive),
}

/// Analyseur incrémental : on lui donne les jetons au fil de l'eau.
#[derive(Default)]
pub struct StreamParser {
    buf: String,
}

const OPEN: &str = "[[";
const CLOSE: &str = "]]";
/// Une directive plus longue que ça est du texte ordinaire (sécurité contre un `[[` isolé).
const MAX_DIRECTIVE: usize = 300;

impl StreamParser {
    pub fn feed(&mut self, chunk: &str) -> Vec<Piece> {
        self.buf.push_str(chunk);
        let mut out = Vec::new();
        loop {
            if let Some(start) = self.buf.find(OPEN) {
                if start > 0 {
                    out.push(Piece::Text(self.buf[..start].to_string()));
                    self.buf.drain(..start);
                }
                if let Some(end) = self.buf.find(CLOSE) {
                    let body = self.buf[OPEN.len()..end].to_string();
                    self.buf.drain(..end + CLOSE.len());
                    if let Some(d) = parse_directive(&body) {
                        out.push(Piece::Directive(d));
                    }
                    continue;
                }
                if self.buf.len() > MAX_DIRECTIVE {
                    // Fausse alerte : on relâche le texte.
                    let t = std::mem::take(&mut self.buf);
                    out.push(Piece::Text(t));
                }
                return out;
            }
            // Pas de `[[` complet. On retient un éventuel `[` final (début de `[[`).
            let keep = if self.buf.ends_with('[') { 1 } else { 0 };
            let emit_len = self.buf.len() - keep;
            if emit_len > 0 {
                out.push(Piece::Text(self.buf[..emit_len].to_string()));
                self.buf.drain(..emit_len);
            }
            return out;
        }
    }

    pub fn finish(&mut self) -> Vec<Piece> {
        let rest = std::mem::take(&mut self.buf);
        if rest.is_empty() { vec![] } else { vec![Piece::Text(rest)] }
    }
}

fn csv(s: &str) -> Vec<String> {
    s.split(',').map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect()
}

pub fn parse_directive(body: &str) -> Option<Directive> {
    let (name, rest) = body.split_once(':')?;
    let name = name.trim().to_lowercase();
    match name.as_str() {
        "show" | "ambient" => {
            let mut parts = rest.split('|');
            let tags = csv(parts.next().unwrap_or(""));
            let (mut mood, mut intensity, mut kind) = (None, None, None);
            for p in parts {
                if let Some((k, v)) = p.split_once('=') {
                    let v = v.trim().to_lowercase();
                    match k.trim().to_lowercase().as_str() {
                        "mood" => mood = Some(v),
                        "intensity" => intensity = v.parse::<u8>().ok().map(|i| i.clamp(1, 5)),
                        "kind" | "type" => kind = Some(match v.as_str() {
                            "photo" | "image" => "photo".to_string(),
                            _ => "video".to_string(),
                        }),
                        _ => {}
                    }
                }
            }
            Some(if name == "show" {
                Directive::Show { tags, mood, intensity, media_kind: kind }
            } else {
                Directive::Ambient { tags, mood }
            })
        }
        "replies" => {
            let items: Vec<String> = rest.split('|').map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()).take(4).collect();
            if items.is_empty() { None } else { Some(Directive::Replies { items }) }
        }
        "state" => {
            let (k, v) = rest.split_once('=')?;
            Some(Directive::State { key: k.trim().to_lowercase(), value: v.trim().to_string() })
        }
        _ => None,
    }
}

/// Découpe le texte en phrases pour alimenter la synthèse vocale au fil de l'eau.
#[derive(Default)]
pub struct SentenceSplitter {
    buf: String,
}

impl SentenceSplitter {
    pub fn feed(&mut self, text: &str) -> Vec<String> {
        self.buf.push_str(text);
        let mut out = Vec::new();
        loop {
            let idx = self.buf.char_indices().find(|(i, c)| {
                matches!(c, '.' | '!' | '?' | '…' | '\n')
                    // pas de coupure sur "3.5" ou "etc.x" : il faut un espace/fin après
                    && self.buf[i + c.len_utf8()..].chars().next().is_some_and(|n| n.is_whitespace())
            });
            match idx {
                Some((i, c)) => {
                    let end = i + c.len_utf8();
                    let s = self.buf[..end].trim().to_string();
                    self.buf.drain(..end);
                    if s.chars().any(|c| c.is_alphanumeric()) { out.push(s); }
                }
                None => break,
            }
        }
        out
    }

    pub fn finish(&mut self) -> Option<String> {
        let s = std::mem::take(&mut self.buf).trim().to_string();
        if s.chars().any(|c| c.is_alphanumeric()) { Some(s) } else { None }
    }
}

/// Retire le balisage de mise en forme (*gestes*, emojis…) qui n'a pas à être lu à voix haute.
pub fn speakable(s: &str) -> String {
    let mut out = String::new();
    let mut in_action = false;
    for c in s.chars() {
        match c {
            '*' => in_action = !in_action,
            _ if in_action => {}
            '_' | '#' | '`' => {}
            c if (c as u32) >= 0x1F000 => {}
            c => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(chunks: &[&str]) -> Vec<Piece> {
        let mut p = StreamParser::default();
        let mut out = vec![];
        for c in chunks { out.extend(p.feed(c)); }
        out.extend(p.finish());
        out
    }

    fn text(pieces: &[Piece]) -> String {
        pieces.iter().filter_map(|p| if let Piece::Text(t) = p { Some(t.as_str()) } else { None }).collect()
    }

    #[test]
    fn directive_decoupee_en_plusieurs_jetons() {
        let r = all(&["Regarde ça. [", "[show: plage, soleil | mo", "od=joyeux | intensity=2]", "] Joli, non ?"]);
        assert_eq!(text(&r), "Regarde ça.  Joli, non ?");
        assert!(r.contains(&Piece::Directive(Directive::Show {
            tags: vec!["plage".into(), "soleil".into()],
            mood: Some("joyeux".into()), intensity: Some(2), media_kind: None,
        })));
    }

    #[test]
    fn crochet_isole_reste_du_texte() {
        let r = all(&["a [b] c"]);
        assert_eq!(text(&r), "a [b] c");
    }

    #[test]
    fn replies_et_state() {
        let r = all(&["[[replies: Oui | Non | Peut-être]][[state: confiance=3]]"]);
        assert_eq!(r, vec![
            Piece::Directive(Directive::Replies { items: vec!["Oui".into(), "Non".into(), "Peut-être".into()] }),
            Piece::Directive(Directive::State { key: "confiance".into(), value: "3".into() }),
        ]);
    }

    #[test]
    fn intensite_bornee() {
        match parse_directive("show: x | intensity=99") {
            Some(Directive::Show { intensity, .. }) => assert_eq!(intensity, Some(5)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn phrases_pour_le_tts() {
        let mut s = SentenceSplitter::default();
        let mut out = s.feed("Bonjour. Il fait 3.5 degrés ! Et ");
        out.extend(s.feed("toi ? Moi ça va"));
        out.extend(s.finish());
        assert_eq!(out, vec!["Bonjour.", "Il fait 3.5 degrés !", "Et toi ?", "Moi ça va"]);
    }

    #[test]
    fn speakable_retire_les_gestes() {
        assert_eq!(speakable("*sourit* Salut toi 😀 !"), "Salut toi !");
    }
}
