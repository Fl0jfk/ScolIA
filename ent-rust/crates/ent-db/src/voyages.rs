//! Socle 42, lot V0a : sorties et voyages scolaires collectifs — **logique pure**
//! (`docs/socles/42_SORTIES_VOYAGES.md` §8, §9, §10, §20).
//!
//! Rien ici ne touche la base : l'automate des états (R-SV1, R-SV4, R-SV5), les contrôles
//! d'audit (NX-15), la « prochaine étape » (NX-14) et les règles d'effectif (NX-27, NX-28).
//! Les commandes qui écrivent (droit, état, version, journal, outbox dans **une** transaction)
//! sont dans `voyages_repo.rs` et s'appuient sur ces fonctions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// États d'une sortie (§8.1). L'ordre n'a pas de sens métier.
pub const ETATS: [&str; 15] = [
    "brouillon",
    "soumise",
    "a_completer",
    "preparation",
    "a_signer",
    "budget_a_valider",
    "vote",
    "a_autoriser",
    "ouverte",
    "prete",
    "en_cours",
    "rentree",
    "close",
    "refusee",
    "annulee",
];

/// Étapes amont vers lesquelles une sortie autorisée peut être rouverte (NX-09, NX-83) et où
/// peut revenir une demande de modifications (NX-05).
pub const ETAPES_AMONT: [&str; 5] = [
    "soumise",
    "preparation",
    "a_signer",
    "budget_a_valider",
    "a_autoriser",
];

pub const NATURES: [&str; 2] = ["obligatoire", "facultative"];
pub const FORMES: [&str; 2] = ["sans_nuitee", "voyage"];
pub const DESTINATIONS: [&str; 4] = ["proximite", "france", "frontalier", "etranger"];
pub const ROLES_ENCADRANT: [&str; 4] = ["responsable", "encadrant", "non_compte", "intervenant"];
pub const TYPES_ETAPE: [&str; 8] = [
    "depart",
    "depot",
    "reprise",
    "visite",
    "hebergement",
    "retour",
    "reunion",
    "autre",
];
pub const MOTIFS_PARTICIPANT: [&str; 7] = [
    "sante",
    "famille",
    "decision_personnelle",
    "financier",
    "exigence_manquante",
    "decision_etablissement",
    "autre",
];

/// États terminaux : plus aucune écriture sur le projet.
pub fn est_terminal(etat: &str) -> bool {
    matches!(etat, "close" | "refusee" | "annulee")
}

/// États où l'on peut encore modifier le projet (le départ constaté ferme les modifications ;
/// l'appel de départ est le lot V3).
pub fn est_modifiable(etat: &str) -> bool {
    !est_terminal(etat) && !matches!(etat, "en_cours" | "rentree")
}

/// Ce qui détermine le circuit (R-SV4, version « code » de V0a : les règles en tables
/// versionnées et le contexte public/privé arrivent avec les lots V1 et V8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contexte<'a> {
    pub nature: &'a str,
    pub forme: &'a str,
    pub besoin_car: bool,
}

impl Contexte<'_> {
    /// Une sortie obligatoire de proximité sans nuitée n'a ni budget ni devis à valider
    /// (§8.2, R-SV8) : `soumise` → `a_autoriser` → `ouverte`/`prete`.
    pub fn circuit_court(&self) -> bool {
        self.nature == "obligatoire" && self.forme == "sans_nuitee" && !self.besoin_car
    }

    /// Un coût est à valider par la comptabilité : voyage, ou car nécessaire.
    pub fn a_budget(&self) -> bool {
        self.forme == "voyage" || self.besoin_car
    }
}

/// Étape suivante de la validation pédagogique (`soumise` → …), selon le contexte.
pub fn apres_validation_pedagogique(c: &Contexte<'_>) -> &'static str {
    if c.circuit_court() {
        "a_autoriser"
    } else {
        "preparation"
    }
}

/// Après la préparation : devis à signer (car), sinon budget, sinon validation finale.
pub fn apres_preparation(c: &Contexte<'_>) -> &'static str {
    if c.besoin_car {
        "a_signer"
    } else if c.a_budget() {
        "budget_a_valider"
    } else {
        "a_autoriser"
    }
}

/// Résultat de la validation finale (NX-08) : « validée » (`prete`) si la liste est déjà
/// confirmée, sinon « ouverte » (en attente de la liste).
pub fn apres_autorisation(liste_confirmee: bool) -> &'static str {
    if liste_confirmee {
        "prete"
    } else {
        "ouverte"
    }
}

/// Commandes de l'automate (R-SV1 : tout passage d'état est une commande typée).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commande {
    Soumettre,
    ValiderPedagogie,
    DemanderModifications,
    AnnulerDemande,
    Renvoyer,
    Refuser,
    DemanderSignature,
    PasserAuxFinances,
    ValiderBudget,
    Autoriser,
    ConfirmerListe,
    Rouvrir,
    Annuler,
    Requalifier,
}

/// Qui peut tenter la commande, avant le contrôle des droits fins (`voyages_repo`).
pub fn etats_de_depart(cmd: Commande) -> &'static [&'static str] {
    match cmd {
        Commande::Soumettre => &["brouillon"],
        Commande::ValiderPedagogie => &["soumise"],
        Commande::DemanderModifications => &[
            "soumise",
            "preparation",
            "a_signer",
            "budget_a_valider",
            "a_autoriser",
        ],
        Commande::AnnulerDemande | Commande::Renvoyer => &["a_completer"],
        Commande::Refuser => &[
            "soumise",
            "a_completer",
            "preparation",
            "a_signer",
            "budget_a_valider",
            "a_autoriser",
        ],
        Commande::DemanderSignature => &["preparation"],
        Commande::PasserAuxFinances => &["preparation", "a_signer"],
        Commande::ValiderBudget => &["budget_a_valider"],
        Commande::Autoriser => &["a_autoriser"],
        Commande::ConfirmerListe => &["a_autoriser", "ouverte"],
        Commande::Rouvrir => &["ouverte", "prete"],
        Commande::Annuler => &[
            "brouillon",
            "soumise",
            "a_completer",
            "preparation",
            "a_signer",
            "budget_a_valider",
            "a_autoriser",
            "ouverte",
            "prete",
            "en_cours",
        ],
        Commande::Requalifier => &[
            "brouillon",
            "soumise",
            "a_completer",
            "preparation",
            "a_signer",
            "budget_a_valider",
            "a_autoriser",
            "ouverte",
            "prete",
        ],
    }
}

// ───────────────────────── Contrôles et audit ─────────────────────────

/// Gravité d'un point d'audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gravite {
    /// Empêche l'étape (soumission, liste…).
    Bloquant,
    /// À traiter mais n'empêche pas (NX-79 : « à préciser »).
    Avertissement,
    /// Conseil.
    Conseil,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PointAudit {
    pub code: &'static str,
    pub gravite: Gravite,
    pub libelle: String,
}

/// Données lues pour l'audit (déjà minimisées : aucun nom).
#[derive(Debug, Clone)]
pub struct EtatDossier<'a> {
    pub etat: &'a str,
    pub nature: &'a str,
    pub forme: &'a str,
    pub destination: &'a str,
    pub pays: Option<&'a str>,
    pub besoin_car: bool,
    pub effectif_mode: &'a str,
    pub effectif_eleves_declare: Option<i32>,
    pub effectif_adultes_declare: Option<i32>,
    pub places_encadrant_a_pourvoir: i32,
    pub nb_publics: i64,
    /// Participants comptés dans la liste (inscrit, prêt, bloqué, en attente exclus).
    pub nb_inscrits: i64,
    pub nb_encadrants_comptes: i64,
    pub nb_responsables: i64,
    pub nb_etapes_depot: i64,
    pub nb_etapes_reprise: i64,
    pub liste_confirmee: bool,
    pub contournement_devis: bool,
}

/// Points d'audit d'un dossier (NX-15, VOY-131), sans accès aux données personnelles.
pub fn audit(d: &EtatDossier<'_>) -> Vec<PointAudit> {
    let mut v = Vec::new();
    let mut p = |code, gravite, libelle: &str| {
        v.push(PointAudit {
            code,
            gravite,
            libelle: libelle.to_string(),
        })
    };
    if d.nb_publics == 0 && d.nb_inscrits == 0 {
        p(
            "public_absent",
            Gravite::Bloquant,
            "Aucune classe, aucun groupe et aucun élève : qui part ?",
        );
    }
    if d.nb_encadrants_comptes == 0 {
        p(
            "encadrant_absent",
            Gravite::Bloquant,
            "Aucun encadrant choisi : désignez au moins un adulte du personnel.",
        );
    } else if d.nb_responsables == 0 {
        p(
            "responsable_absent",
            Gravite::Avertissement,
            "Aucun responsable parmi les encadrants (R-SV7).",
        );
    }
    if d.places_encadrant_a_pourvoir > 0 {
        p(
            "encadrant_a_preciser",
            Gravite::Avertissement,
            "Des places d'encadrant restent à pourvoir (adultes à préciser).",
        );
    }
    if d.destination == "etranger" && d.pays.is_none() {
        p("pays_absent", Gravite::Bloquant, "Le pays de destination est obligatoire à l'étranger.");
    }
    if d.forme == "voyage" && d.effectif_eleves_declare.is_none() {
        p(
            "effectif_absent",
            Gravite::Bloquant,
            "L'effectif d'élèves est nécessaire pour un voyage (devis, budget).",
        );
    }
    if let (Some(declare), true) = (d.effectif_eleves_declare, d.nb_inscrits > 0) {
        let i = d.nb_inscrits;
        let d64 = i64::from(declare);
        if d.effectif_mode == "maximum" && i > d64 {
            p(
                "effectif_depasse",
                Gravite::Bloquant,
                "Plus d'élèves inscrits que l'effectif maximum déclaré.",
            );
        }
        if d.effectif_mode == "exact" && i != d64 {
            p(
                "effectif_inexact",
                Gravite::Avertissement,
                "Le nombre d'élèves inscrits diffère de l'effectif exact déclaré.",
            );
        }
    }
    if d.forme == "voyage" && matches!(d.etat, "ouverte" | "a_autoriser" | "prete") {
        if d.nb_etapes_depot == 0 {
            p(
                "depot_absent",
                Gravite::Bloquant,
                "Horaire et lieu de dépôt des élèves non renseignés.",
            );
        }
        if d.nb_etapes_reprise == 0 {
            p(
                "reprise_absente",
                Gravite::Bloquant,
                "Horaire et lieu de reprise des élèves non renseignés.",
            );
        }
    }
    if d.besoin_car && d.contournement_devis {
        p(
            "devis_contourne",
            Gravite::Avertissement,
            "Passage aux finances sans devis signé (motif au journal).",
        );
    }
    if d.besoin_car && !d.contournement_devis && matches!(d.etat, "preparation") {
        p(
            "devis_conseil",
            Gravite::Conseil,
            "Deux devis de transporteurs suffisent généralement pour comparer.",
        );
    }
    if !d.liste_confirmee && matches!(d.etat, "ouverte") {
        p(
            "liste_a_confirmer",
            Gravite::Bloquant,
            "La liste des élèves n'est pas encore confirmée.",
        );
    }
    v
}

/// Points bloquants pour confirmer la liste (NX-28).
pub fn blocages_liste(d: &EtatDossier<'_>) -> Vec<PointAudit> {
    let mut b: Vec<PointAudit> = audit(d)
        .into_iter()
        .filter(|p| {
            p.gravite == Gravite::Bloquant
                && matches!(
                    p.code,
                    "effectif_depasse" | "depot_absent" | "reprise_absente"
                )
        })
        .collect();
    if d.nb_inscrits == 0 {
        b.push(PointAudit {
            code: "liste_vide",
            gravite: Gravite::Bloquant,
            libelle: "La liste doit contenir au moins un élève.".into(),
        });
    }
    if d.effectif_mode == "exact" {
        if let Some(declare) = d.effectif_eleves_declare {
            if d.nb_inscrits != i64::from(declare) {
                b.push(PointAudit {
                    code: "effectif_inexact",
                    gravite: Gravite::Bloquant,
                    libelle: "L'effectif est « exact » : le nombre d'inscrits doit lui être égal.".into(),
                });
            }
        }
    }
    b
}

/// Points bloquants pour soumettre le projet (NX-01 : classes et au moins un accompagnateur).
pub fn blocages_soumission(d: &EtatDossier<'_>) -> Vec<PointAudit> {
    audit(d)
        .into_iter()
        .filter(|p| {
            p.gravite == Gravite::Bloquant
                && matches!(
                    p.code,
                    "public_absent" | "encadrant_absent" | "pays_absent" | "effectif_absent"
                )
        })
        .collect()
}

// ───────────────────────── Prochaine étape (NX-14) ─────────────────────────

/// Qui doit agir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Acteur {
    Porteur,
    Direction,
    Comptabilite,
    Secretariat,
    Personne,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProchaineEtape {
    pub qui: Acteur,
    pub libelle: &'static str,
    /// Commande proposée (code de route), absente quand rien n'est à faire.
    pub commande: Option<&'static str>,
}

/// Calcul **serveur** (VOY-130) : qui doit agir et quoi, selon l'état et le contexte.
pub fn prochaine_etape(etat: &str, c: &Contexte<'_>, liste_confirmee: bool) -> ProchaineEtape {
    let e = |qui, libelle, commande| ProchaineEtape {
        qui,
        libelle,
        commande,
    };
    match etat {
        "brouillon" => e(
            Acteur::Porteur,
            "Compléter le projet puis le soumettre à la direction.",
            Some("soumettre"),
        ),
        "soumise" => e(
            Acteur::Direction,
            "Valider la pédagogie du projet, demander des modifications ou refuser.",
            Some("valider-pedagogie"),
        ),
        "a_completer" => e(
            Acteur::Porteur,
            "Modifier le projet selon la demande de la direction, puis le renvoyer.",
            Some("renvoyer"),
        ),
        "preparation" => match apres_preparation(c) {
            "a_signer" => e(
                Acteur::Porteur,
                "Demander les devis du car, choisir un devis puis le présenter à la signature.",
                Some("demander-signature"),
            ),
            "budget_a_valider" => e(
                Acteur::Porteur,
                "Préparer l'encadrement et le programme, puis passer aux finances.",
                Some("passer-finances"),
            ),
            _ => e(
                Acteur::Porteur,
                "Préparer l'encadrement et le programme, puis demander la validation finale.",
                Some("passer-finances"),
            ),
        },
        "a_signer" => e(
            Acteur::Direction,
            "Signer le devis du transporteur choisi, ou passer aux finances sans devis signé (motif).",
            Some("passer-finances"),
        ),
        "budget_a_valider" => e(
            Acteur::Comptabilite,
            "Valider le budget, ou demander des modifications.",
            Some("valider-budget"),
        ),
        "vote" => e(
            Acteur::Direction,
            "Enregistrer la délibération du conseil d'administration.",
            None,
        ),
        "a_autoriser" => e(
            Acteur::Direction,
            "Autoriser la sortie (validation finale).",
            Some("autoriser"),
        ),
        "ouverte" if !liste_confirmee => e(
            Acteur::Porteur,
            "Choisir les élèves puis confirmer la liste.",
            Some("confirmer-liste"),
        ),
        "ouverte" => e(
            Acteur::Porteur,
            "La liste est confirmée : la sortie va être prête.",
            Some("confirmer-liste"),
        ),
        "prete" => e(Acteur::Personne, "La sortie est prête : rien à faire avant le départ.", None),
        "en_cours" => e(Acteur::Personne, "La sortie est en cours.", None),
        "rentree" => e(Acteur::Secretariat, "Retour effectué : clore le dossier.", None),
        _ => e(Acteur::Personne, "Dossier terminé.", None),
    }
}

// ───────────────────────── Séries ─────────────────────────

/// Dates d'une série hebdomadaire : toutes les occurrences de `jour_semaine` (1 = lundi … 7 =
/// dimanche) entre `du` et `au` inclus, hors `exclus` (jours fériés, vacances, fermetures).
/// Retourne `(retenues, écartées)`. Borné à 120 occurrences pour éviter un abus.
pub fn dates_de_serie(
    jour_semaine: u32,
    du: chrono::NaiveDate,
    au: chrono::NaiveDate,
    exclus: &std::collections::HashSet<chrono::NaiveDate>,
) -> (Vec<chrono::NaiveDate>, Vec<chrono::NaiveDate>) {
    use chrono::{Datelike, Duration};
    let mut retenues = Vec::new();
    let mut ecartees = Vec::new();
    let mut j = du;
    // Premier jour correspondant.
    while j.weekday().number_from_monday() != jour_semaine && j <= au {
        j += Duration::days(1);
    }
    while j <= au && retenues.len() + ecartees.len() < 120 {
        if exclus.contains(&j) {
            ecartees.push(j);
        } else {
            retenues.push(j);
        }
        j += Duration::days(7);
    }
    (retenues, ecartees)
}

/// Référence lisible d'une sortie : `VOY-AAAA-XXXXXXXX` (année de début, 8 hexadécimaux du
/// numéro interne). Unique par établissement (contrainte en base).
pub fn reference(debut: DateTime<Utc>, id: Uuid) -> String {
    use chrono::Datelike;
    format!(
        "VOY-{}-{}",
        debut.year(),
        id.simple().to_string()[..8].to_uppercase()
    )
}

/// Échappe un champ de tableur : neutralise l'injection de formule (`= + - @` en tête) et
/// protège séparateurs, guillemets et retours à la ligne.
pub fn champ_csv(s: &str) -> String {
    let mut t = s.replace(['\r', '\n'], " ");
    if t.starts_with(['=', '+', '-', '@', '\t']) {
        t.insert(0, '\'');
    }
    if t.contains([';', '"']) {
        t = format!("\"{}\"", t.replace('"', "\"\""));
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn c(nature: &'static str, forme: &'static str, car: bool) -> Contexte<'static> {
        Contexte {
            nature,
            forme,
            besoin_car: car,
        }
    }

    fn dossier() -> EtatDossier<'static> {
        EtatDossier {
            etat: "brouillon",
            nature: "facultative",
            forme: "sans_nuitee",
            destination: "proximite",
            pays: None,
            besoin_car: false,
            effectif_mode: "maximum",
            effectif_eleves_declare: Some(20),
            effectif_adultes_declare: Some(2),
            places_encadrant_a_pourvoir: 0,
            nb_publics: 1,
            nb_inscrits: 18,
            nb_encadrants_comptes: 2,
            nb_responsables: 1,
            nb_etapes_depot: 0,
            nb_etapes_reprise: 0,
            liste_confirmee: false,
            contournement_devis: false,
        }
    }

    #[test]
    fn circuit_selon_le_contexte() {
        // Sortie obligatoire de proximité : une seule validation, ni budget ni devis.
        let court = c("obligatoire", "sans_nuitee", false);
        assert!(court.circuit_court() && !court.a_budget());
        assert_eq!(apres_validation_pedagogique(&court), "a_autoriser");
        // Facultative sans car : préparation puis validation finale (pas de budget).
        let fac = c("facultative", "sans_nuitee", false);
        assert_eq!(apres_validation_pedagogique(&fac), "preparation");
        assert_eq!(apres_preparation(&fac), "a_autoriser");
        // Avec car : devis à signer ; voyage sans car : budget.
        assert_eq!(apres_preparation(&c("facultative", "sans_nuitee", true)), "a_signer");
        assert_eq!(apres_preparation(&c("facultative", "voyage", false)), "budget_a_valider");
        // Une sortie obligatoire avec car n'est plus « courte » (il y a un coût).
        assert!(!c("obligatoire", "sans_nuitee", true).circuit_court());
    }

    #[test]
    fn autorisation_selon_la_liste() {
        assert_eq!(apres_autorisation(true), "prete");
        assert_eq!(apres_autorisation(false), "ouverte");
    }

    #[test]
    fn etats_de_depart_coherents() {
        for cmd in [
            Commande::Soumettre,
            Commande::ValiderPedagogie,
            Commande::DemanderModifications,
            Commande::AnnulerDemande,
            Commande::Renvoyer,
            Commande::Refuser,
            Commande::DemanderSignature,
            Commande::PasserAuxFinances,
            Commande::ValiderBudget,
            Commande::Autoriser,
            Commande::ConfirmerListe,
            Commande::Rouvrir,
            Commande::Annuler,
            Commande::Requalifier,
        ] {
            for e in etats_de_depart(cmd) {
                assert!(ETATS.contains(e), "{cmd:?} : état inconnu {e}");
                assert!(!est_terminal(e), "{cmd:?} part d'un état terminal {e}");
            }
        }
        // Les étapes de retour d'une demande de modifications sont des étapes amont.
        for e in etats_de_depart(Commande::DemanderModifications) {
            assert!(ETAPES_AMONT.contains(e));
        }
        // Un dossier refusé ou annulé ne s'annule, ni ne se rouvre.
        assert!(!etats_de_depart(Commande::Annuler).contains(&"annulee"));
        assert!(!etats_de_depart(Commande::Rouvrir).contains(&"refusee"));
    }

    #[test]
    fn audit_signale_les_manques_sans_nom() {
        let mut d = dossier();
        d.nb_publics = 0;
        d.nb_inscrits = 0;
        d.nb_encadrants_comptes = 0;
        let codes: Vec<_> = audit(&d).iter().map(|p| p.code).collect();
        assert!(codes.contains(&"public_absent") && codes.contains(&"encadrant_absent"));
        assert_eq!(blocages_soumission(&d).len(), 2);
        // Un voyage à l'étranger exige pays et effectif.
        let mut v = dossier();
        v.forme = "voyage";
        v.destination = "etranger";
        v.effectif_eleves_declare = None;
        let codes: Vec<_> = blocages_soumission(&v).iter().map(|p| p.code).collect();
        assert!(codes.contains(&"pays_absent") && codes.contains(&"effectif_absent"));
    }

    #[test]
    fn places_encadrant_a_pourvoir_n_est_pas_bloquant() {
        // NX-79 : accompagnateurs déclarés plus nombreux que les noms = « à préciser ».
        let mut d = dossier();
        d.places_encadrant_a_pourvoir = 2;
        let a = audit(&d);
        let p = a.iter().find(|p| p.code == "encadrant_a_preciser").unwrap();
        assert_eq!(p.gravite, Gravite::Avertissement);
        assert!(blocages_soumission(&d).is_empty());
    }

    #[test]
    fn liste_effectif_maximum_et_exact() {
        let mut d = dossier();
        d.effectif_eleves_declare = Some(10);
        d.nb_inscrits = 11;
        assert!(blocages_liste(&d).iter().any(|p| p.code == "effectif_depasse"));
        d.nb_inscrits = 10;
        assert!(blocages_liste(&d).is_empty());
        d.effectif_mode = "exact";
        d.nb_inscrits = 9;
        assert!(blocages_liste(&d).iter().any(|p| p.code == "effectif_inexact"));
        d.nb_inscrits = 0;
        assert!(blocages_liste(&d).iter().any(|p| p.code == "liste_vide"));
    }

    #[test]
    fn voyage_exige_depot_et_reprise_a_la_liste() {
        let mut d = dossier();
        d.forme = "voyage";
        d.etat = "ouverte";
        let codes: Vec<_> = blocages_liste(&d).iter().map(|p| p.code).collect();
        assert!(codes.contains(&"depot_absent") && codes.contains(&"reprise_absente"));
        d.nb_etapes_depot = 1;
        d.nb_etapes_reprise = 1;
        assert!(blocages_liste(&d).is_empty());
    }

    #[test]
    fn prochaine_etape_nomme_qui_agit() {
        let fac = c("facultative", "voyage", true);
        assert_eq!(prochaine_etape("brouillon", &fac, false).qui, Acteur::Porteur);
        assert_eq!(prochaine_etape("soumise", &fac, false).qui, Acteur::Direction);
        assert_eq!(prochaine_etape("budget_a_valider", &fac, false).qui, Acteur::Comptabilite);
        assert_eq!(prochaine_etape("a_autoriser", &fac, false).commande, Some("autoriser"));
        assert_eq!(
            prochaine_etape("ouverte", &fac, false).commande,
            Some("confirmer-liste")
        );
        assert_eq!(prochaine_etape("prete", &fac, true).commande, None);
        // Tous les états ont une réponse.
        for e in ETATS {
            assert!(!prochaine_etape(e, &fac, false).libelle.is_empty());
        }
    }

    #[test]
    fn serie_exclut_feries_et_vacances() {
        // Les mardis de septembre 2026 : 1, 8, 15, 22, 29.
        let du = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let au = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let exclus: std::collections::HashSet<_> =
            [NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()].into_iter().collect();
        let (ok, ko) = dates_de_serie(2, du, au, &exclus);
        assert_eq!(ok.len(), 4);
        assert_eq!(ko, vec![NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()]);
        assert!(ok.iter().all(|d| chrono::Datelike::weekday(d).number_from_monday() == 2));
        // Aucune occurrence si la plage est trop courte.
        let (ok, _) = dates_de_serie(2, du, du, &Default::default());
        assert_eq!(ok, vec![du]);
        let (ok, _) = dates_de_serie(3, du, du, &Default::default());
        assert!(ok.is_empty());
    }

    #[test]
    fn reference_lisible_et_stable() {
        let id = Uuid::parse_str("0123abcd-0000-0000-0000-000000000000").unwrap();
        let d = DateTime::parse_from_rfc3339("2027-03-04T08:00:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(reference(d, id), "VOY-2027-0123ABCD");
    }

    #[test]
    fn csv_neutralise_les_formules() {
        assert_eq!(champ_csv("Dupont"), "Dupont");
        assert_eq!(champ_csv("=CMD()"), "'=CMD()");
        assert_eq!(champ_csv("+33"), "'+33");
        assert_eq!(champ_csv("a;b"), "\"a;b\"");
        assert_eq!(champ_csv("a\nb"), "a b");
        assert_eq!(champ_csv("dit \"oui\";"), "\"dit \"\"oui\"\";\"");
    }
}
