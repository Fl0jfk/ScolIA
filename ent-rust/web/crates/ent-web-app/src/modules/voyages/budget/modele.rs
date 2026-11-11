//! Logique d'écran du budget d'une sortie (socle 42, lot V0b), sans Leptos ni réseau : montants
//! affichés depuis des centimes, libellés des postes, regroupement des lignes, contrôle de saisie
//! **non métier** (champs vides) et conversion vers le DTO envoyé.
//!
//! Aucune règle métier ici : le format des montants (« 1 350,50 », « 1.350 » refusé), le calcul de la
//! part des familles, l'équilibre et les droits sont décidés par le serveur (R-SV1, R-SV16) ; l'écran
//! affiche ce que le serveur renvoie, y compris ses refus.

use ent_types::voyages::{
    BudgetLigneDto, BudgetVersionDto, CalculBudgetDto, LigneBudgetSaisieDto, PrixFigeDto,
};
use ent_web_ui::TonEtat;

/// Postes de dépense (code serveur, libellé) dans l'ordre d'affichage.
pub const POSTES_DEPENSE: [(&str, &str); 8] = [
    ("transport", "Transport"),
    ("hebergement", "Hébergement"),
    ("restauration", "Restauration"),
    ("activites", "Activités et entrées"),
    ("assurance", "Assurance"),
    ("divers", "Frais divers"),
    (
        "encadrement",
        "Encadrement (transport, hébergement, repas des accompagnateurs)",
    ),
    (
        "surcout_handicap",
        "Surcoût lié au handicap ou à un besoin médical",
    ),
];

pub const POSTES_RECETTE: [(&str, &str); 7] = [
    ("etablissement", "Établissement"),
    ("apel", "APEL (collectif)"),
    ("collectivite", "Collectivité"),
    ("subvention_europeenne", "Subvention européenne (Erasmus+)"),
    ("don_affecte", "Don affecté"),
    ("ventes_eleves", "Ventes et actions des élèves"),
    (
        "aides_individuelles",
        "Aides individuelles (total, sans nom)",
    ),
];

pub const AFFECTATIONS: [(&str, &str); 2] = [
    ("general", "Réduit la part des familles"),
    ("encadrement", "Couvre l'encadrement et le surcoût"),
];

/// `135050` → `1 350,50 €` (espaces insécables).
pub fn euros(centimes: i64) -> String {
    let signe = if centimes < 0 { "-" } else { "" };
    let abs = centimes.unsigned_abs();
    let entiers = (abs / 100).to_string();
    let mut groupes = String::new();
    for (i, c) in entiers.chars().enumerate() {
        if i > 0 && (entiers.len() - i).is_multiple_of(3) {
            groupes.push('\u{a0}');
        }
        groupes.push(c);
    }
    format!("{signe}{groupes},{:02}\u{a0}€", abs % 100)
}

/// Montant à préremplir dans un champ de saisie : `135050` → `1 350,50` (format attendu par le serveur).
pub fn saisie_depuis_centimes(centimes: i64) -> String {
    euros(centimes).trim_end_matches("\u{a0}€").to_string()
}

pub fn libelle_poste(poste: &str) -> &'static str {
    POSTES_DEPENSE
        .iter()
        .chain(POSTES_RECETTE.iter())
        .find(|(c, _)| *c == poste)
        .map_or("Autre", |(_, l)| l)
}

pub fn libelle_source(source: &str) -> &'static str {
    match source {
        "devis_signe" => "Devis signé",
        "devis_recu" => "Devis reçu",
        "commande" => "Commande",
        "saisie_motivee" => "Saisie motivée",
        _ => "Source inconnue",
    }
}

pub fn libelle_etat_version(etat: &str) -> &'static str {
    match etat {
        "brouillon" => "En cours d'édition",
        "validee" => "Validée",
        "remplacee" => "Remplacée",
        _ => "État inconnu",
    }
}

pub fn libelle_cause_prix(cause: &str) -> &'static str {
    match cause {
        "ouverture" => "Figé à l'autorisation",
        "baisse" => "Baisse",
        "hausse_decidee" => "Hausse décidée par la direction",
        _ => "Changement",
    }
}

pub fn libelle_equilibre(c: &CalculBudgetDto) -> &'static str {
    if c.equilibre {
        "Budget équilibré"
    } else {
        "Budget déséquilibré : validation impossible"
    }
}

pub fn ton_equilibre(c: &CalculBudgetDto) -> TonEtat {
    if c.equilibre {
        TonEtat::Ok
    } else {
        TonEtat::Mal
    }
}

/// Lignes d'un sens, regroupées par poste dans l'ordre du catalogue (puis par rang).
pub fn lignes_du_sens<'a>(v: &'a BudgetVersionDto, sens: &str) -> Vec<&'a BudgetLigneDto> {
    let postes: &[(&str, &str)] = if sens == "depense" {
        &POSTES_DEPENSE
    } else {
        &POSTES_RECETTE
    };
    let mut out: Vec<&BudgetLigneDto> = Vec::new();
    for (code, _) in postes {
        let mut du_poste: Vec<&BudgetLigneDto> = v
            .lignes
            .iter()
            .filter(|l| l.sens == sens && l.poste == *code)
            .collect();
        du_poste.sort_by_key(|l| l.rang);
        out.extend(du_poste);
    }
    out
}

/// Phrase décrivant un prix figé dans l'historique.
pub fn resume_prix(p: &PrixFigeDto, jour: &str) -> String {
    format!(
        "{jour} — {} : {}",
        libelle_cause_prix(&p.cause),
        euros(p.montant_centimes)
    )
}

/// Saisie d'une ligne telle que tapée (aucun nombre : le montant reste un texte).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieLigne {
    pub sens: String,
    pub poste: String,
    pub libelle: String,
    pub montant: String,
    pub motif: String,
    pub affectation: String,
}

impl SaisieLigne {
    /// Contrôle **d'ergonomie** seulement (champs vides) : les vraies règles sont celles du serveur.
    pub fn controle(&self) -> Result<(), String> {
        if self.poste.trim().is_empty() {
            return Err("Choisissez un poste.".into());
        }
        if self.libelle.trim().is_empty() {
            return Err("Le libellé est obligatoire.".into());
        }
        if self.montant.trim().is_empty() {
            return Err("Le montant est obligatoire (par exemple 1 350,50).".into());
        }
        if self.motif.trim().is_empty() {
            return Err(
                "Un motif est obligatoire : d'où vient ce montant (devis, estimation, décision) ?"
                    .into(),
            );
        }
        Ok(())
    }

    pub fn vers_dto(&self, rev: i32) -> LigneBudgetSaisieDto {
        let non_vide = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        LigneBudgetSaisieDto {
            rev,
            sens: self.sens.clone(),
            poste: self.poste.clone(),
            libelle: self.libelle.trim().to_string(),
            montant: self.montant.trim().to_string(),
            motif: non_vide(&self.motif),
            // L'affectation ne concerne que les recettes ; le serveur la valide.
            affectation: if self.sens == "recette" {
                non_vide(&self.affectation)
            } else {
                None
            },
        }
    }
}

/// La version sur laquelle porte une validation : le brouillon en cours, sinon la version validée.
pub fn version_a_valider(
    brouillon: Option<&BudgetVersionDto>,
    validee: Option<&BudgetVersionDto>,
) -> Option<(i32, i32)> {
    brouillon.or(validee).map(|v| (v.numero, v.rev))
}

/// Texte du bouton de validation selon l'étape de la sortie (le serveur décide s'il est permis).
pub fn libelle_validation(etat_sortie: &str, a_brouillon: bool) -> Option<&'static str> {
    match (etat_sortie, a_brouillon) {
        ("budget_a_valider", _) => Some("Valider le budget"),
        ("a_autoriser" | "ouverte" | "prete", true) => Some("Valider cette révision"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ligne(sens: &str, poste: &str, rang: i32, c: i64) -> BudgetLigneDto {
        BudgetLigneDto {
            id: uuid::Uuid::nil(),
            sens: sens.into(),
            poste: poste.into(),
            libelle: format!("{poste}{rang}"),
            montant_centimes: c,
            source: "saisie_motivee".into(),
            motif: Some("m".into()),
            affectation: None,
            rang,
        }
    }

    fn calcul(equilibre: bool) -> CalculBudgetDto {
        CalculBudgetDto {
            nb_payants: 3,
            depenses_total: 0,
            depenses_familles: 0,
            encadrement: 0,
            surcout_handicap: 0,
            recettes_total: 0,
            recettes_encadrement: 0,
            recettes_reduisant_la_part: 0,
            marge_pct: 7,
            marge: 0,
            cout_reel_par_eleve: None,
            part_familles: None,
            total_familles: 0,
            deficit: 0,
            arrondi_pris_en_charge: 0,
            excedent: 0,
            equilibre,
            causes: vec![],
        }
    }

    fn version(lignes: Vec<BudgetLigneDto>) -> BudgetVersionDto {
        BudgetVersionDto {
            numero: 1,
            etat: "brouillon".into(),
            rev: 4,
            nb_payants: 3,
            marge_pct: 7,
            marge_motif: None,
            motif_revision: None,
            part_familles_centimes: None,
            cree_le: Utc::now(),
            valide_le: None,
            lignes,
            calcul: calcul(true),
        }
    }

    #[test]
    fn montants_affiches_depuis_des_centimes() {
        assert_eq!(euros(135_050), "1\u{a0}350,50\u{a0}€");
        assert_eq!(euros(5), "0,05\u{a0}€");
        assert_eq!(euros(123_456_789), "1\u{a0}234\u{a0}567,89\u{a0}€");
        assert_eq!(euros(-100), "-1,00\u{a0}€");
        // Le champ de saisie est prérempli au format attendu par le serveur (virgule, sans « € »).
        assert_eq!(saisie_depuis_centimes(135_050), "1\u{a0}350,50");
    }

    #[test]
    fn chaque_poste_a_un_libelle_et_aucun_n_est_en_double() {
        let mut codes: Vec<&str> = POSTES_DEPENSE
            .iter()
            .chain(POSTES_RECETTE.iter())
            .map(|(c, _)| *c)
            .collect();
        assert_eq!(codes.len(), 15);
        for c in &codes {
            assert_ne!(libelle_poste(c), "Autre", "{c}");
        }
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), 15, "aucun code en double");
        assert_eq!(libelle_poste("inconnu"), "Autre");
        assert_eq!(libelle_source("saisie_motivee"), "Saisie motivée");
        assert_eq!(
            libelle_cause_prix("hausse_decidee"),
            "Hausse décidée par la direction"
        );
        assert_eq!(libelle_etat_version("remplacee"), "Remplacée");
    }

    #[test]
    fn lignes_regroupees_par_poste_dans_l_ordre_du_catalogue() {
        let v = version(vec![
            ligne("depense", "divers", 1, 1),
            ligne("depense", "transport", 2, 2),
            ligne("depense", "transport", 1, 3),
            ligne("recette", "apel", 1, 4),
        ]);
        let d: Vec<(&str, i32)> = lignes_du_sens(&v, "depense")
            .iter()
            .map(|l| (l.poste.as_str(), l.rang))
            .collect();
        assert_eq!(d, [("transport", 1), ("transport", 2), ("divers", 1)]);
        assert_eq!(lignes_du_sens(&v, "recette").len(), 1);
    }

    #[test]
    fn equilibre_libelle_et_ton() {
        assert_eq!(libelle_equilibre(&calcul(true)), "Budget équilibré");
        assert_eq!(ton_equilibre(&calcul(true)), TonEtat::Ok);
        assert!(libelle_equilibre(&calcul(false)).contains("validation impossible"));
        assert_eq!(ton_equilibre(&calcul(false)), TonEtat::Mal);
    }

    #[test]
    fn saisie_de_ligne_controle_d_ergonomie_et_conversion() {
        let mut s = SaisieLigne {
            sens: "recette".into(),
            poste: "apel".into(),
            libelle: "  APEL  ".into(),
            montant: " 1.350 ".into(),
            motif: "Décision".into(),
            affectation: "general".into(),
        };
        // Le front ne devine pas le format : « 1.350 » part tel quel, le serveur le refusera.
        assert!(s.controle().is_ok());
        let d = s.vers_dto(7);
        assert_eq!(
            (d.rev, d.libelle.as_str(), d.montant.as_str()),
            (7, "APEL", "1.350")
        );
        assert_eq!(d.affectation.as_deref(), Some("general"));
        // Une dépense n'envoie jamais d'affectation.
        s.sens = "depense".into();
        assert_eq!(s.vers_dto(1).affectation, None);
        for (champ, message) in [
            ("poste", "poste"),
            ("libelle", "libellé"),
            ("montant", "montant"),
            ("motif", "motif"),
        ] {
            let mut x = s.clone();
            match champ {
                "poste" => x.poste.clear(),
                "libelle" => x.libelle = "  ".into(),
                "montant" => x.montant.clear(),
                _ => x.motif.clear(),
            }
            let e = x.controle().unwrap_err();
            assert!(e.to_lowercase().contains(message), "{champ} : {e}");
        }
    }

    #[test]
    fn version_validee_ou_brouillon_et_bouton_selon_l_etape() {
        let b = version(vec![]);
        let mut v = version(vec![]);
        v.numero = 2;
        v.rev = 9;
        assert_eq!(version_a_valider(Some(&b), Some(&v)), Some((1, 4)));
        assert_eq!(version_a_valider(None, Some(&v)), Some((2, 9)));
        assert_eq!(version_a_valider(None, None), None);
        assert_eq!(
            libelle_validation("budget_a_valider", false),
            Some("Valider le budget")
        );
        assert_eq!(
            libelle_validation("a_autoriser", true),
            Some("Valider cette révision")
        );
        assert_eq!(libelle_validation("ouverte", false), None);
        assert_eq!(libelle_validation("soumise", true), None);
    }

    #[test]
    fn resume_du_prix_fige() {
        let p = PrixFigeDto {
            rang: 2,
            montant_centimes: 40_000,
            cause: "baisse".into(),
            motif: None,
            fige_le: Utc::now(),
        };
        assert_eq!(
            resume_prix(&p, "12/03/2027"),
            "12/03/2027 — Baisse : 400,00\u{a0}€"
        );
    }
}
