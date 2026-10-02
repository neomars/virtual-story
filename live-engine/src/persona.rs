//! Personas : fiches de personnalité éditables, stockées en JSON dans `data/personas/`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Persona {
    pub id: String,
    pub name: String,
    /// Doit être ≥ 18 : toute fiche en dessous est refusée à l'enregistrement.
    pub age: u32,
    pub summary: String,
    pub personality: String,
    pub speaking_style: String,
    pub scenario: String,
    /// Limites que le personnage ne franchit pas (en plus des règles du moteur).
    pub boundaries: String,
    pub first_message: String,
    pub language: String,
    /// Nom d'une voix de référence (fichier dans le dossier des voix) ou voix du serveur TTS.
    pub voice: Option<String>,
    /// Expressivité de la voix (Chatterbox, 0.25-2.0 ; ~0.7 = intime et expressif).
    pub tts_exaggeration: Option<f32>,
    /// Rythme/guidage (Chatterbox, 0-1 ; plus bas = débit plus lent et posé).
    pub tts_cfg_weight: Option<f32>,
    pub temperature: Option<f32>,
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            age: 18,
            summary: String::new(),
            personality: String::new(),
            speaking_style: String::new(),
            scenario: String::new(),
            boundaries: String::new(),
            first_message: String::new(),
            language: "fr".into(),
            voice: None,
            tts_exaggeration: None,
            tts_cfg_weight: None,
            temperature: None,
        }
    }
}

impl Persona {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || !self.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("id invalide (lettres, chiffres, - et _ uniquement)".into());
        }
        if self.name.trim().is_empty() {
            return Err("le nom est obligatoire".into());
        }
        if self.age < 18 {
            return Err("le personnage doit être un adulte (âge ≥ 18)".into());
        }
        Ok(())
    }
}

/// Personnages fournis avec l'app, copiés au premier lancement si le dossier est vide.
const DEFAULT_PERSONAS: [(&str, &str); 2] = [
    ("camille", include_str!("../data/personas/camille.json")),
    ("lea", include_str!("../data/personas/lea.json")),
];

pub struct PersonaStore {
    dir: PathBuf,
}

impl PersonaStore {
    pub fn new(dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self { dir: dir.to_path_buf() })
    }

    /// Écrit les personnages par défaut si aucun fichier .json n'existe encore.
    pub fn seed_defaults(&self) {
        let empty = std::fs::read_dir(&self.dir)
            .map(|d| !d.flatten().any(|e| e.path().extension().is_some_and(|x| x == "json")))
            .unwrap_or(true);
        if empty {
            for (id, json) in DEFAULT_PERSONAS {
                let _ = std::fs::write(self.dir.join(format!("{id}.json")), json);
            }
        }
    }

    pub fn list(&self) -> Vec<Persona> {
        let mut v: Vec<Persona> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|s| serde_json::from_str::<Persona>(&s).ok())
            .filter(|p| p.validate().is_ok())
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn get(&self, id: &str) -> Option<Persona> {
        // L'id est validé à l'écriture ; on le revalide ici pour éviter tout chemin forgé.
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return None;
        }
        let s = std::fs::read_to_string(self.dir.join(format!("{id}.json"))).ok()?;
        serde_json::from_str::<Persona>(&s).ok().filter(|p| p.validate().is_ok())
    }

    pub fn save(&self, p: &Persona) -> Result<(), String> {
        p.validate()?;
        let s = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
        std::fs::write(self.dir.join(format!("{}.json", p.id)), s).map_err(|e| e.to_string())
    }

    pub fn delete(&self, id: &str) -> bool {
        id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && std::fs::remove_file(self.dir.join(format!("{id}.json"))).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personnages_par_defaut_valides_et_semes_une_seule_fois() {
        for (id, json) in DEFAULT_PERSONAS {
            let p: Persona = serde_json::from_str(json).expect("JSON valide");
            assert_eq!(p.id, id);
            assert!(p.validate().is_ok());
            assert!(p.age >= 18);
        }
        let dir = std::env::temp_dir().join(format!("vs-personas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = PersonaStore::new(&dir).unwrap();
        store.seed_defaults();
        assert_eq!(store.list().len(), 2);
        store.delete("lea");
        store.seed_defaults(); // le dossier n'est plus vide : on ne ressème pas
        assert_eq!(store.list().len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn refuse_un_age_mineur() {
        let p = Persona { id: "x".into(), name: "X".into(), age: 17, ..Default::default() };
        assert!(p.validate().is_err());
    }

    #[test]
    fn refuse_un_id_avec_chemin() {
        let p = Persona { id: "../etc".into(), name: "X".into(), age: 30, ..Default::default() };
        assert!(p.validate().is_err());
    }
}
