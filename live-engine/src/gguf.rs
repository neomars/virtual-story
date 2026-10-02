//! Lecture de l'en-tête GGUF (métadonnées uniquement) : on obtient ainsi l'architecture RÉELLE du
//! modèle (couches, têtes KV, dimension de tête, contexte) au lieu de la deviner.
//! Format : https://github.com/ggml-org/ggml/blob/master/docs/gguf.md (v2/v3, petit-boutiste).

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GgufInfo {
    pub architecture: String,
    pub block_count: u32,
    pub context_length: u32,
    pub head_count: u32,
    /// Maximum sur les couches si le GGUF fournit un tableau par couche.
    pub head_count_kv: u32,
    pub key_length: Option<u32>,
    pub embedding_length: Option<u32>,
    pub sliding_window: Option<u32>,
    /// Têtes KV par couche, si le GGUF fournit un tableau (Gemma 4 : 8 en fenêtre glissante, 1 en global).
    pub head_count_kv_per_layer: Vec<u32>,
    /// Couches à fenêtre glissante : tableau de booléens (Gemma 4)…
    pub sliding_window_pattern: Vec<bool>,
    /// …ou motif scalaire n (Gemma 3 : n=6 → 5 couches locales puis 1 globale).
    pub sliding_window_pattern_n: Option<u32>,
    pub key_length_swa: Option<u32>,
    pub value_length: Option<u32>,
    pub value_length_swa: Option<u32>,
    /// Dernières couches qui réutilisent le KV des précédentes (aucun cache propre).
    pub shared_kv_layers: u32,
}

impl GgufInfo {
    /// Dimension d'une tête d'attention.
    pub fn head_dim(&self) -> u32 {
        self.key_length
            .filter(|k| *k > 0)
            .or_else(|| self.embedding_length?.checked_div(self.head_count))
            .unwrap_or(128)
    }
}

enum V {
    Num(i64),
    Str(String),
    /// Tableau numérique (ex. têtes KV par couche) ; les autres tableaux sont sautés.
    NumArr(Vec<i64>),
    Other,
}

fn rd<const N: usize, R: Read>(r: &mut R) -> std::io::Result<[u8; N]> {
    let mut b = [0u8; N];
    r.read_exact(&mut b)?;
    Ok(b)
}
fn u32_<R: Read>(r: &mut R) -> std::io::Result<u32> { Ok(u32::from_le_bytes(rd(r)?)) }
fn u64_<R: Read>(r: &mut R) -> std::io::Result<u64> { Ok(u64::from_le_bytes(rd(r)?)) }

fn skip<R: Read + Seek>(r: &mut BufReader<R>, n: u64) -> std::io::Result<()> {
    r.seek_relative(i64::try_from(n).map_err(|_| std::io::Error::other("saut trop grand"))?)
}

fn read_string<R: Read>(r: &mut R) -> std::io::Result<String> {
    let len = u64_(r)?;
    if len > 1 << 20 {
        return Err(std::io::Error::other("chaîne GGUF trop longue"));
    }
    let mut b = vec![0u8; len as usize];
    r.read_exact(&mut b)?;
    Ok(String::from_utf8_lossy(&b).into_owned())
}

/// Taille en octets d'un type scalaire, ou None pour string/array.
fn scalar_size(t: u32) -> Option<u64> {
    Some(match t {
        0 | 1 | 7 => 1,
        2 | 3 => 2,
        4..=6 => 4,
        10..=12 => 8,
        _ => return None,
    })
}

fn read_scalar<R: Read>(r: &mut R, t: u32) -> std::io::Result<i64> {
    Ok(match t {
        0 => u8::from_le_bytes(rd(r)?) as i64,
        1 => i8::from_le_bytes(rd(r)?) as i64,
        2 => u16::from_le_bytes(rd(r)?) as i64,
        3 => i16::from_le_bytes(rd(r)?) as i64,
        4 => u32::from_le_bytes(rd(r)?) as i64,
        5 => i32::from_le_bytes(rd(r)?) as i64,
        6 => { let _ = rd::<4, _>(r)?; 0 } // f32 : sans intérêt ici
        7 => u8::from_le_bytes(rd(r)?) as i64,
        10 => u64::from_le_bytes(rd(r)?) as i64,
        11 => i64::from_le_bytes(rd(r)?),
        12 => { let _ = rd::<8, _>(r)?; 0 }
        _ => return Err(std::io::Error::other(format!("type GGUF inconnu : {t}"))),
    })
}

fn skip_value<R: Read + Seek>(r: &mut BufReader<R>, t: u32) -> std::io::Result<()> {
    if let Some(n) = scalar_size(t) {
        return skip(r, n);
    }
    match t {
        8 => { let len = u64_(r)?; skip(r, len) }
        9 => {
            let et = u32_(r)?;
            let len = u64_(r)?;
            if let Some(n) = scalar_size(et) {
                skip(r, n.checked_mul(len).ok_or_else(|| std::io::Error::other("tableau trop grand"))?)
            } else {
                for _ in 0..len { skip_value(r, et)?; }
                Ok(())
            }
        }
        _ => Err(std::io::Error::other(format!("type GGUF inconnu : {t}"))),
    }
}

fn read_value<R: Read + Seek>(r: &mut BufReader<R>, t: u32, keep: bool) -> std::io::Result<V> {
    if !keep {
        skip_value(r, t)?;
        return Ok(V::Other);
    }
    if scalar_size(t).is_some() {
        return Ok(V::Num(read_scalar(r, t)?));
    }
    match t {
        8 => Ok(V::Str(read_string(r)?)),
        9 => {
            let et = u32_(r)?;
            let len = u64_(r)?;
            if scalar_size(et).is_some() && len <= 4096 {
                let mut v = Vec::with_capacity(len as usize);
                for _ in 0..len { v.push(read_scalar(r, et)?); }
                Ok(V::NumArr(v))
            } else {
                for _ in 0..len { skip_value(r, et)?; }
                Ok(V::Other)
            }
        }
        _ => Err(std::io::Error::other(format!("type GGUF inconnu : {t}"))),
    }
}

fn as_u32(v: &V) -> Option<u32> {
    match v {
        V::Num(n) if *n >= 0 => Some(*n as u32),
        V::NumArr(a) => a.iter().copied().max().filter(|m| *m >= 0).map(|m| m as u32),
        _ => None,
    }
}

pub fn parse<R: Read + Seek>(r: &mut BufReader<R>) -> std::io::Result<GgufInfo> {
    if &rd::<4, _>(r)? != b"GGUF" {
        return Err(std::io::Error::other("ce fichier n'est pas un GGUF"));
    }
    let version = u32_(r)?;
    if !(2..=3).contains(&version) {
        return Err(std::io::Error::other(format!("version GGUF non gérée : {version}")));
    }
    let _tensors = u64_(r)?;
    let kv_count = u64_(r)?;

    let mut info = GgufInfo::default();
    for _ in 0..kv_count {
        let key = read_string(r)?;
        let t = u32_(r)?;
        let wanted = key == "general.architecture"
            || [".block_count", ".context_length", ".attention.head_count", ".attention.head_count_kv",
                ".attention.key_length", ".embedding_length", ".attention.sliding_window",
                ".attention.sliding_window_pattern", ".attention.key_length_swa", ".attention.value_length",
                ".attention.value_length_swa", ".attention.shared_kv_layers"]
                .iter()
                .any(|suf| key.ends_with(suf));
        let v = read_value(r, t, wanted)?;
        if !wanted { continue; }
        if key == "general.architecture" {
            if let V::Str(s) = &v { info.architecture = s.clone(); }
            continue;
        }
        // Tableaux par couche (têtes KV, motif de fenêtre glissante) : on garde le détail.
        if key.ends_with(".attention.head_count_kv") {
            if let V::NumArr(a) = &v { info.head_count_kv_per_layer = a.iter().map(|x| (*x).max(0) as u32).collect() }
        }
        if key.ends_with(".attention.sliding_window_pattern") {
            match &v {
                V::NumArr(a) => info.sliding_window_pattern = a.iter().map(|x| *x != 0).collect(),
                V::Num(n) if *n > 0 => info.sliding_window_pattern_n = Some(*n as u32),
                _ => {}
            }
            continue;
        }
        let Some(n) = as_u32(&v) else { continue };
        // Le premier gagne (un GGUF d'un seul modèle n'a qu'une architecture).
        if key.ends_with(".block_count") && info.block_count == 0 { info.block_count = n }
        else if key.ends_with(".context_length") && info.context_length == 0 { info.context_length = n }
        else if key.ends_with(".attention.head_count_kv") && info.head_count_kv == 0 { info.head_count_kv = n }
        else if key.ends_with(".attention.head_count") && info.head_count == 0 { info.head_count = n }
        else if key.ends_with(".attention.key_length") && info.key_length.is_none() { info.key_length = Some(n) }
        else if key.ends_with(".embedding_length") && info.embedding_length.is_none() { info.embedding_length = Some(n) }
        else if key.ends_with(".attention.sliding_window") && info.sliding_window.is_none() { info.sliding_window = Some(n) }
        else if key.ends_with(".attention.key_length_swa") && info.key_length_swa.is_none() { info.key_length_swa = Some(n) }
        else if key.ends_with(".attention.value_length_swa") && info.value_length_swa.is_none() { info.value_length_swa = Some(n) }
        else if key.ends_with(".attention.value_length") && info.value_length.is_none() { info.value_length = Some(n) }
        else if key.ends_with(".attention.shared_kv_layers") { info.shared_kv_layers = n }
    }
    if info.block_count == 0 {
        return Err(std::io::Error::other("GGUF sans block_count : architecture illisible"));
    }
    if info.head_count_kv == 0 { info.head_count_kv = info.head_count.max(1); }
    Ok(info)
}

pub fn read_info(path: &Path) -> std::io::Result<GgufInfo> {
    parse(&mut BufReader::with_capacity(1 << 16, File::open(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn s(buf: &mut Vec<u8>, v: &str) {
        buf.extend((v.len() as u64).to_le_bytes());
        buf.extend(v.as_bytes());
    }
    fn kv_u32(buf: &mut Vec<u8>, k: &str, v: u32) {
        s(buf, k);
        buf.extend(4u32.to_le_bytes());
        buf.extend(v.to_le_bytes());
    }

    fn sample(kv_arr: bool) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(b"GGUF");
        b.extend(3u32.to_le_bytes());
        b.extend(0u64.to_le_bytes()); // tenseurs
        b.extend(10u64.to_le_bytes()); // clés
        s(&mut b, "general.architecture"); b.extend(8u32.to_le_bytes()); s(&mut b, "gemma9");
        // un gros tableau de chaînes à sauter (tokenizer)
        s(&mut b, "tokenizer.ggml.tokens"); b.extend(9u32.to_le_bytes()); b.extend(8u32.to_le_bytes());
        b.extend(3u64.to_le_bytes()); s(&mut b, "a"); s(&mut b, "bb"); s(&mut b, "ccc");
        kv_u32(&mut b, "gemma9.block_count", 48);
        kv_u32(&mut b, "gemma9.context_length", 131072);
        kv_u32(&mut b, "gemma9.embedding_length", 3840);
        kv_u32(&mut b, "gemma9.attention.head_count", 16);
        if kv_arr {
            // têtes KV par couche : [8, 8, 4]
            s(&mut b, "gemma9.attention.head_count_kv"); b.extend(9u32.to_le_bytes()); b.extend(4u32.to_le_bytes());
            b.extend(3u64.to_le_bytes());
            for v in [8u32, 8, 4] { b.extend(v.to_le_bytes()); }
        } else {
            kv_u32(&mut b, "gemma9.attention.head_count_kv", 8);
        }
        kv_u32(&mut b, "gemma9.attention.key_length", 256);
        kv_u32(&mut b, "gemma9.attention.sliding_window", 1024);
        s(&mut b, "general.name"); b.extend(8u32.to_le_bytes()); s(&mut b, "x");
        b
    }

    #[test]
    fn lit_les_metadonnees_utiles() {
        let i = parse(&mut BufReader::new(Cursor::new(sample(false)))).unwrap();
        assert_eq!(i.architecture, "gemma9");
        assert_eq!((i.block_count, i.head_count, i.head_count_kv), (48, 16, 8));
        assert_eq!(i.context_length, 131072);
        assert_eq!(i.head_dim(), 256);
        assert_eq!(i.sliding_window, Some(1024));
    }

    /// GGUF au gabarit Gemma 4 12B : 48 couches, tableaux de têtes KV et de motif de fenêtre.
    fn gemma4_like() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(b"GGUF");
        b.extend(3u32.to_le_bytes());
        b.extend(0u64.to_le_bytes());
        b.extend(9u64.to_le_bytes());
        s(&mut b, "general.architecture"); b.extend(8u32.to_le_bytes()); s(&mut b, "gemma4");
        kv_u32(&mut b, "gemma4.block_count", 48);
        kv_u32(&mut b, "gemma4.context_length", 262144);
        kv_u32(&mut b, "gemma4.attention.head_count", 16);
        kv_u32(&mut b, "gemma4.attention.key_length", 512);
        kv_u32(&mut b, "gemma4.attention.key_length_swa", 256);
        kv_u32(&mut b, "gemma4.attention.sliding_window", 1024);
        s(&mut b, "gemma4.attention.head_count_kv"); b.extend(9u32.to_le_bytes()); b.extend(4u32.to_le_bytes());
        b.extend(48u64.to_le_bytes());
        for i in 0..48 { b.extend((if i % 6 == 5 { 1u32 } else { 8u32 }).to_le_bytes()); }
        s(&mut b, "gemma4.attention.sliding_window_pattern"); b.extend(9u32.to_le_bytes()); b.extend(7u32.to_le_bytes());
        b.extend(48u64.to_le_bytes());
        for i in 0..48 { b.push(if i % 6 == 5 { 0 } else { 1 }); }
        b
    }

    #[test]
    fn lit_les_tableaux_par_couche_de_gemma4() {
        let i = parse(&mut BufReader::new(Cursor::new(gemma4_like()))).unwrap();
        assert_eq!(i.block_count, 48);
        assert_eq!(i.head_count_kv_per_layer.len(), 48);
        assert_eq!((i.head_count_kv_per_layer[0], i.head_count_kv_per_layer[5]), (8, 1));
        assert_eq!(i.sliding_window_pattern.iter().filter(|x| **x).count(), 40);
        assert_eq!((i.key_length, i.key_length_swa, i.sliding_window), (Some(512), Some(256), Some(1024)));
    }

    #[test]
    fn tableau_de_tetes_kv_prend_le_maximum() {
        let i = parse(&mut BufReader::new(Cursor::new(sample(true)))).unwrap();
        assert_eq!(i.head_count_kv, 8);
    }

    #[test]
    fn head_dim_deduit_de_embedding_si_key_length_absent() {
        let i = GgufInfo { embedding_length: Some(5120), head_count: 32, ..Default::default() };
        assert_eq!(i.head_dim(), 160);
    }

    #[test]
    fn refuse_un_fichier_non_gguf() {
        assert!(parse(&mut BufReader::new(Cursor::new(b"nope-not-gguf-at-all".to_vec()))).is_err());
    }
}
