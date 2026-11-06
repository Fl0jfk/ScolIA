//! Choix de l'enfant d'une famille (partagé entre modules) : logique pure testée et sélecteur.
//! Un seul enfant : rien à choisir ; jusqu'à trois : contrôle segmenté ; au-delà : liste déroulante.

use ent_web_api::Enfant;
use ent_web_ui::{OptionSegment, Segmente, Selection};
use leptos::prelude::*;
use uuid::Uuid;

/// « Harry Potter » (prénom puis nom).
pub fn nom_enfant(e: &Enfant) -> String {
    format!("{} {}", e.prenom.trim(), e.nom.trim())
        .trim()
        .to_string()
}

/// Enfant à afficher : celui demandé s'il existe, sinon le premier.
pub fn enfant_choisi(enfants: &[Enfant], demande: Option<Uuid>) -> Option<Uuid> {
    demande
        .filter(|id| enfants.iter().any(|e| e.id == *id))
        .or_else(|| enfants.first().map(|e| e.id))
}

/// Sélecteur d'enfant. `choisi` contient l'identifiant (texte) de l'enfant retenu.
#[component]
pub fn ChoixEnfant(enfants: Vec<Enfant>, choisi: RwSignal<String>) -> impl IntoView {
    (enfants.len() > 1).then(|| {
        let options: Vec<(String, String)> = enfants
            .iter()
            .map(|e| (e.id.to_string(), nom_enfant(e)))
            .collect();
        let contenu = if enfants.len() <= 3 {
            let options = options
                .into_iter()
                .map(|(v, l)| OptionSegment::new(v, l))
                .collect();
            view! { <Segmente nom="enfant" legende="Enfant" options=options valeur=choisi/> }
                .into_any()
        } else {
            view! { <Selection id="enfant" libelle="Enfant" valeur=choisi options=options/> }
                .into_any()
        };
        view! { <div class="abs-enfants">{contenu}</div> }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enfant(id: u128, prenom: &str, nom: &str) -> Enfant {
        Enfant {
            id: Uuid::from_u128(id),
            nom: nom.into(),
            prenom: prenom.into(),
            classe: None,
            status: "actif".into(),
            lien: "parent".into(),
            peut_voir_notes: true,
            peut_voir_vs: true,
            peut_justifier_absence: true,
            peut_autoriser_sorties: false,
        }
    }

    #[test]
    fn choix_de_l_enfant() {
        let l = vec![
            enfant(1, "Harry", "Potter"),
            enfant(2, " Ginny ", "Weasley"),
        ];
        assert_eq!(nom_enfant(&l[1]), "Ginny Weasley");
        assert_eq!(
            enfant_choisi(&l, Some(Uuid::from_u128(2))),
            Some(Uuid::from_u128(2))
        );
        // Demande inconnue ou absente : le premier.
        assert_eq!(
            enfant_choisi(&l, Some(Uuid::from_u128(9))),
            Some(Uuid::from_u128(1))
        );
        assert_eq!(enfant_choisi(&l, None), Some(Uuid::from_u128(1)));
        assert_eq!(enfant_choisi(&[], None), None);
    }
}
