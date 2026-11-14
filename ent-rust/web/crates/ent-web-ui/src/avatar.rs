//! Avatar à initiales, robuste aux noms difficiles (noms composés, particules,
//! accents, emoji, espaces multiples, nom absent) — voir `break-ui`.

use leptos::prelude::*;

/// Particules ignorées pour les initiales (« Maria de la Cruz » → « MC »).
const PARTICULES: &[&str] = &[
    "de", "du", "des", "la", "le", "les", "d'", "l'", "van", "von", "der", "y", "e", "da", "di",
];

/// Première lettre d'un mot (grappe de caractères Unicode simple : on ignore les
/// caractères qui ne sont pas des lettres, comme les emoji ou la ponctuation).
fn premiere_lettre(mot: &str) -> Option<String> {
    mot.chars()
        .find(|c| c.is_alphabetic())
        .map(|c| c.to_uppercase().collect())
}

/// Deux initiales au plus, en majuscules. `?` si aucune lettre exploitable.
/// Les mots en capitales (NOM de famille) et en minuscules sont traités pareil.
pub fn initiales(nom: &str) -> String {
    let mots: Vec<&str> = nom
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|m| !m.is_empty())
        .filter(|m| !PARTICULES.contains(&m.to_lowercase().as_str()))
        .filter(|m| m.chars().any(char::is_alphabetic))
        .collect();
    let lettres: Vec<String> = match mots.as_slice() {
        [] => vec![],
        [seul] => premiere_lettre(seul).into_iter().collect(),
        [premier, .., dernier] => [premiere_lettre(premier), premiere_lettre(dernier)]
            .into_iter()
            .flatten()
            .collect(),
    };
    if lettres.is_empty() {
        "?".into()
    } else {
        lettres.concat()
    }
}

/// Pastille ronde avec les initiales (décorative : le nom est toujours écrit à côté).
#[component]
pub fn Avatar(#[prop(into)] nom: String, #[prop(optional)] accent: bool) -> impl IntoView {
    view! {
        <span class=if accent { "ui-avatar ui-avatar--accent" } else { "ui-avatar" } aria-hidden="true">
            {initiales(&nom)}
        </span>
    }
}
