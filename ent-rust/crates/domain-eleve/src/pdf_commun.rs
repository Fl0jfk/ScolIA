//! Habillage commun des PDF de `domain-eleve` (D-062) : métadonnées et rendu « une ligne =
//! une information » via la brique `ent-pdf`. Aucune écriture PDF à la main ici.

use chrono::{Datelike, NaiveDate};
use ent_pdf::{DateHeure, Meta};

const AUTEUR: &str = "ScolIA";
const PIED: &str = "ScolIA — document généré par l'ENT";

/// Métadonnées : la date du document (à minuit UTC) sert de date de création, ce qui garde
/// le rendu déterministe.
pub(crate) fn meta(titre: &str, date: Option<NaiveDate>) -> Meta {
    Meta {
        titre: titre.to_string(),
        auteur: AUTEUR.to_string(),
        creation: date.map(|d| DateHeure {
            an: d.year().clamp(1, 9999) as u16,
            mois: d.month() as u8,
            jour: d.day() as u8,
            heure: 0,
            minute: 0,
            seconde: 0,
        }),
    }
}

pub(crate) fn pied() -> &'static str {
    PIED
}

pub(crate) fn rendre_lignes(titre: &str, date: Option<NaiveDate>, lignes: &[String]) -> Vec<u8> {
    ent_pdf::rendre_lignes(meta(titre, date), PIED, lignes)
}
