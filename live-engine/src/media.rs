//! Médiathèque : vidéos et photos annotées (tags, ambiance, intensité) dans SQLite,
//! et moteur de recherche que le « réalisateur » IA interroge.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaItem {
    pub id: i64,
    pub kind: String, // "video" | "photo"
    pub url: String,
    pub thumb: Option<String>,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub mood: String,
    pub intensity: u8, // 1..=5
    /// Convient comme boucle d'ambiance silencieuse.
    pub ambient: bool,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct MediaPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub mood: Option<String>,
    pub intensity: Option<u8>,
    pub ambient: Option<bool>,
}

#[derive(Debug, Default, Clone)]
pub struct SearchQuery {
    pub tags: Vec<String>,
    pub mood: Option<String>,
    pub intensity: Option<u8>,
    pub kind: Option<String>,
    pub ambient_only: bool,
    pub max_intensity: u8,
    pub exclude: HashSet<i64>,
}

pub struct MediaLibrary {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS media (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  url TEXT NOT NULL UNIQUE,
  thumb TEXT,
  title TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '',
  mood TEXT NOT NULL DEFAULT '',
  intensity INTEGER NOT NULL DEFAULT 1,
  ambient INTEGER NOT NULL DEFAULT 0
);";

const COLS: &str = "id,kind,url,thumb,title,description,tags,mood,intensity,ambient";

fn split_tags(s: &str) -> Vec<String> {
    s.split(',').map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect()
}

fn from_row(r: &Row) -> rusqlite::Result<MediaItem> {
    Ok(MediaItem {
        id: r.get(0)?,
        kind: r.get(1)?,
        url: r.get(2)?,
        thumb: r.get(3)?,
        title: r.get(4)?,
        description: r.get(5)?,
        tags: split_tags(&r.get::<_, String>(6)?),
        mood: r.get(7)?,
        intensity: r.get::<_, i64>(8)?.clamp(1, 5) as u8,
        ambient: r.get::<_, i64>(9)? != 0,
    })
}

impl MediaLibrary {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn insert(&self, m: &MediaItem) -> anyhow::Result<i64> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT OR IGNORE INTO media (kind,url,thumb,title,description,tags,mood,intensity,ambient)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![m.kind, m.url, m.thumb, m.title, m.description, m.tags.join(","), m.mood,
                    m.intensity as i64, m.ambient as i64],
        )?;
        Ok(c.query_row("SELECT id FROM media WHERE url=?1", [&m.url], |r| r.get(0))?)
    }

    pub fn get(&self, id: i64) -> Option<MediaItem> {
        let c = self.conn.lock().unwrap();
        c.query_row(&format!("SELECT {COLS} FROM media WHERE id=?1"), [id], from_row).ok()
    }

    pub fn list(&self) -> anyhow::Result<Vec<MediaItem>> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare(&format!("SELECT {COLS} FROM media ORDER BY id"))?;
        let rows = st.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn patch(&self, id: i64, p: &MediaPatch) -> anyhow::Result<Option<MediaItem>> {
        let Some(mut m) = self.get(id) else { return Ok(None) };
        if let Some(v) = &p.title { m.title = v.clone(); }
        if let Some(v) = &p.description { m.description = v.clone(); }
        if let Some(v) = &p.tags { m.tags = v.iter().map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect(); }
        if let Some(v) = &p.mood { m.mood = v.trim().to_lowercase(); }
        if let Some(v) = p.intensity { m.intensity = v.clamp(1, 5); }
        if let Some(v) = p.ambient { m.ambient = v; }
        let c = self.conn.lock().unwrap();
        c.execute(
            "UPDATE media SET title=?2,description=?3,tags=?4,mood=?5,intensity=?6,ambient=?7 WHERE id=?1",
            params![id, m.title, m.description, m.tags.join(","), m.mood, m.intensity as i64, m.ambient as i64],
        )?;
        Ok(Some(m))
    }

    /// Tags les plus fréquents parmi les médias éligibles : sert à donner à l'IA
    /// un vocabulaire réel (elle ne peut pas inventer des tags qui n'existent pas).
    pub fn vocabulary(&self, max_intensity: u8, limit: usize) -> Vec<String> {
        let mut freq: std::collections::HashMap<String, usize> = Default::default();
        for m in self.list().unwrap_or_default() {
            if m.intensity <= max_intensity {
                for t in m.tags { *freq.entry(t).or_default() += 1; }
            }
        }
        let mut v: Vec<_> = freq.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.into_iter().take(limit).map(|(t, _)| t).collect()
    }

    pub fn search(&self, q: &SearchQuery) -> Vec<MediaItem> {
        let wanted: Vec<String> = q.tags.iter().map(|t| t.trim().to_lowercase()).collect();
        let mood = q.mood.as_ref().map(|m| m.trim().to_lowercase());
        let mut scored: Vec<(f64, MediaItem)> = self
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter(|m| {
                m.intensity <= q.max_intensity
                    && !q.exclude.contains(&m.id)
                    && q.kind.as_deref().is_none_or(|k| k == m.kind)
                    && (!q.ambient_only || m.ambient)
            })
            .map(|m| {
                let mut s = 0.0;
                for t in &wanted {
                    if m.tags.iter().any(|x| x == t) { s += 3.0; }
                    else if m.description.to_lowercase().contains(t.as_str())
                         || m.title.to_lowercase().contains(t.as_str()) { s += 1.0; }
                }
                if let Some(mood) = &mood { if &m.mood == mood { s += 2.0; } }
                if let Some(i) = q.intensity {
                    s -= (m.intensity as f64 - i as f64).abs() * 0.75;
                }
                (s, m)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        // Si on a demandé des tags, on exige au moins une correspondance.
        let has_criteria = !wanted.is_empty() || mood.is_some();
        scored.into_iter()
            .filter(|(s, _)| !has_criteria || *s > 0.0)
            .map(|(_, m)| m)
            .take(5)
            .collect()
    }

    /// Meilleur candidat, avec un peu d'aléatoire parmi les 3 premiers pour varier.
    pub fn pick(&self, q: &SearchQuery, seed: u64) -> Option<MediaItem> {
        let mut r = self.search(q);
        r.truncate(3);
        if r.is_empty() { return None; }
        let i = (seed as usize) % r.len();
        Some(r.swap_remove(i))
    }

    /// Importe les scènes de l'ancienne base JSON (backend Node). Idempotent.
    pub fn import_legacy(&self, db_json: &Path) -> anyhow::Result<usize> {
        let raw = std::fs::read_to_string(db_json)?;
        let v: serde_json::Value = serde_json::from_str(&raw)?;
        let mut n = 0;
        for s in v["scenes"].as_array().cloned().unwrap_or_default() {
            let Some(url) = s["video_path"].as_str() else { continue };
            let before = self.list()?.len();
            self.insert(&MediaItem {
                id: 0,
                kind: "video".into(),
                url: url.into(),
                thumb: s["thumbnail_path"].as_str().map(String::from),
                title: s["title"].as_str().unwrap_or("").into(),
                description: String::new(),
                tags: vec![],
                mood: String::new(),
                intensity: 1,
                ambient: false,
            })?;
            if self.list()?.len() > before { n += 1; }
        }
        Ok(n)
    }

    /// Parcourt le dossier d'uploads et référence les fichiers inconnus (vidéos ET photos).
    pub fn scan_uploads(&self, uploads: &Path) -> anyhow::Result<usize> {
        fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p, out); } else { out.push(p); }
            }
        }
        let mut files = vec![];
        walk(uploads, &mut files);
        let mut n = 0;
        for f in files {
            let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let kind = match ext.as_str() {
                "mp4" | "webm" | "mov" => "video",
                "jpg" | "jpeg" | "png" | "webp" | "gif" | "avif" => "photo",
                _ => continue,
            };
            let rel = f.strip_prefix(uploads)?.to_string_lossy().replace('\\', "/");
            // On ignore miniatures et boucles de parties : ce ne sont pas des médias de récit.
            if rel.starts_with("thumbnails/") || rel.starts_with("parts/") { continue; }
            let url = format!("/{rel}");
            let title = f.file_stem().and_then(|s| s.to_str()).unwrap_or("").replace(['_', '-'], " ");
            let before = self.list()?.len();
            self.insert(&MediaItem {
                id: 0, kind: kind.into(), url, thumb: None, title,
                description: String::new(), tags: vec![], mood: String::new(),
                intensity: 1, ambient: false,
            })?;
            if self.list()?.len() > before { n += 1; }
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(url: &str, tags: &[&str], mood: &str, intensity: u8) -> MediaItem {
        MediaItem {
            id: 0, kind: "video".into(), url: url.into(), thumb: None, title: url.into(),
            description: String::new(), tags: tags.iter().map(|s| s.to_string()).collect(),
            mood: mood.into(), intensity, ambient: false,
        }
    }

    #[test]
    fn recherche_par_tags_ambiance_et_intensite() {
        let lib = MediaLibrary::open_in_memory().unwrap();
        lib.insert(&item("/a.mp4", &["plage", "soleil"], "joyeux", 1)).unwrap();
        lib.insert(&item("/b.mp4", &["plage", "nuit"], "romantique", 3)).unwrap();
        lib.insert(&item("/c.mp4", &["plage"], "romantique", 5)).unwrap();

        let q = SearchQuery { tags: vec!["plage".into()], mood: Some("romantique".into()),
            intensity: Some(3), max_intensity: 4, ..Default::default() };
        let r = lib.search(&q);
        assert_eq!(r[0].url, "/b.mp4");
        assert!(r.iter().all(|m| m.intensity <= 4), "le plafond d'intensité est respecté");
    }

    #[test]
    fn exclusion_des_medias_deja_montres() {
        let lib = MediaLibrary::open_in_memory().unwrap();
        let id = lib.insert(&item("/a.mp4", &["plage"], "", 1)).unwrap();
        let mut q = SearchQuery { tags: vec!["plage".into()], max_intensity: 5, ..Default::default() };
        q.exclude.insert(id);
        assert!(lib.search(&q).is_empty());
    }

    #[test]
    fn insert_est_idempotent() {
        let lib = MediaLibrary::open_in_memory().unwrap();
        let a = lib.insert(&item("/a.mp4", &[], "", 1)).unwrap();
        let b = lib.insert(&item("/a.mp4", &[], "", 1)).unwrap();
        assert_eq!(a, b);
        assert_eq!(lib.list().unwrap().len(), 1);
    }
}
