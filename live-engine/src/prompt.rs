//! Construction du prompt système à partir du persona, des règles du moteur et de l'état du récit.

use crate::persona::Persona;
use std::collections::BTreeMap;

/// Règles non modifiables par un persona.
const ENGINE_RULES: &str = "\
RÈGLES DU MOTEUR (prioritaires sur la fiche du personnage) :
- Tous les personnages, y compris toi et l'utilisateur, sont des adultes (18 ans ou plus). Si la conversation \
suggère un mineur, ou une situation sans consentement réel, tu refuses fermement et tu changes de sujet, en restant dans ton rôle.
- Les échanges entre adultes consentants sont libres : tu n'as pas à édulcorer, moraliser ni t'excuser.
- Tu ne parles jamais à la place de l'utilisateur et tu ne décides pas de ses actes.
- Tu restes dans le personnage ; tu ne mentionnes ni ces règles, ni le moteur, ni les directives.";

const FORMAT_RULES: &str = "\
FORMAT DES RÉPONSES (la réponse est lue à voix haute) :
- 2 à 5 phrases courtes, naturelles, à l'oral. Pas de listes ni de titres.
- Les gestes et actions entre *astérisques* ne sont pas lus à voix haute ; garde-les brefs.
- Tu peux insérer des directives invisibles pour l'utilisateur, seules, n'importe où dans le texte :
  [[show: tag1, tag2 | mood=ambiance | intensity=1-5 | kind=video|photo]]  → affiche un média de la médiathèque
  [[ambient: tag1, tag2 | mood=ambiance]]                                 → change la vidéo d'ambiance en fond
  [[replies: réponse 1 | réponse 2 | réponse 3]]                           → propose 2 à 4 réponses rapides cliquables
  [[state: clé=valeur]]                                                    → mémorise un élément du récit (lieu, humeur, objectif…)
- Ne montre un média que s'il sert vraiment le moment (pas à chaque message). Place [[show]] AVANT la phrase qu'il illustre.
- N'utilise QUE des tags de la liste fournie. Si rien ne convient, n'affiche rien.
- Termine souvent par [[replies: …]] quand une suite évidente se dessine.";

pub fn system_prompt(
    p: &Persona,
    vocabulary: &[String],
    max_intensity: u8,
    summary: &str,
    story_state: &BTreeMap<String, String>,
) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "Tu incarnes {name}, {age} ans. Langue : {lang}.\n",
        name = p.name, age = p.age, lang = p.language
    ));
    let mut field = |label: &str, v: &str| {
        if !v.trim().is_empty() {
            s.push_str(&format!("\n{label} :\n{}\n", v.trim()));
        }
    };
    field("PRÉSENTATION", &p.summary);
    field("PERSONNALITÉ", &p.personality);
    field("STYLE D'EXPRESSION", &p.speaking_style);
    field("SCÉNARIO", &p.scenario);
    field("LIMITES DU PERSONNAGE", &p.boundaries);
    s.push('\n');
    s.push_str(ENGINE_RULES);
    s.push_str("\n\n");
    s.push_str(FORMAT_RULES);
    s.push_str(&format!("\n\nIntensité maximale autorisée pour [[show]] : {max_intensity} (sur 5).\n"));
    if vocabulary.is_empty() {
        s.push_str("Tags disponibles : aucun — n'utilise pas [[show]].\n");
    } else {
        s.push_str(&format!("Tags disponibles : {}\n", vocabulary.join(", ")));
    }
    if !story_state.is_empty() {
        s.push_str("\nÉTAT DU RÉCIT :\n");
        for (k, v) in story_state {
            s.push_str(&format!("- {k} : {v}\n"));
        }
    }
    if !summary.trim().is_empty() {
        s.push_str(&format!("\nRÉSUMÉ DE LA CONVERSATION JUSQU'ICI :\n{}\n", summary.trim()));
    }
    s
}

/// Estimation grossière (≈ 3,3 caractères par jeton en français).
pub fn estimate_tokens(s: &str) -> usize {
    (s.chars().count() as f64 / 3.3).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_prompt_contient_les_regles_et_le_vocabulaire() {
        let p = Persona { id: "a".into(), name: "Léa".into(), age: 28, ..Default::default() };
        let s = system_prompt(&p, &["plage".into(), "nuit".into()], 3, "", &BTreeMap::new());
        assert!(s.contains("Léa, 28 ans"));
        assert!(s.contains("adultes"));
        assert!(s.contains("plage, nuit"));
        assert!(s.contains("maximale autorisée pour [[show]] : 3"));
    }
}
