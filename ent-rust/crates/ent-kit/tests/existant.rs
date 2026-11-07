//! Le prototype face au code existant (lecture seule des sources) :
//! - la liste découverte des migrations est **identique** (noms, ordre) à la
//!   liste écrite à la main dans `ent-db/src/migrate.rs`, et le tag calculé
//!   égale `MIGRATION_TAG` ;
//! - les contraintes redéfinies dans plusieurs migrations (risque F-009) sont
//!   gelées : toute nouvelle redéfinition fait échouer ce test ;
//! - les 68 codes du catalogue `ent_auth::Action` ont la forme exigée par le kit.

use ent_kit::actions::code_valide;
use ent_kit::migrations::{
    assembler, contraintes_redefinies, tag, DeclMigrations, Plage, MIGRATIONS_NOYAU,
};

const MIGRATE_RS: &str = include_str!("../../ent-db/src/migrate.rs");
const ACTIONS_RS: &str = include_str!("../../ent-auth/src/actions.rs");

/// Noms des `include_str!("../../../migrations/NNN_x.sql")` dans l'ordre du fichier.
fn liste_manuelle() -> Vec<String> {
    MIGRATE_RS
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let debut = l.find("migrations/")? + "migrations/".len();
            let fin = l[debut..].find(".sql")? + debut;
            l.starts_with("include_str!")
                .then(|| l[debut..fin].to_string())
        })
        .collect()
}

fn tag_manuel() -> String {
    let l = MIGRATE_RS
        .lines()
        .find(|l| l.starts_with("pub const MIGRATION_TAG"))
        .expect("MIGRATION_TAG présent");
    l.split('"')
        .nth(1)
        .expect("valeur entre guillemets")
        .to_string()
}

#[test]
fn la_liste_decouverte_egale_la_liste_manuelle() {
    let decouverte: Vec<&str> = MIGRATIONS_NOYAU.iter().map(|m| m.nom).collect();
    let manuelle = liste_manuelle();
    assert!(
        manuelle.len() > 100,
        "parse de migrate.rs : {} entrées",
        manuelle.len()
    );
    assert_eq!(decouverte, manuelle, "même ensemble, même ordre");
    assert_eq!(tag(MIGRATIONS_NOYAU), Some(tag_manuel().as_str()));
}

#[test]
fn le_noyau_historique_s_assemble_dans_sa_plage() {
    let decl = DeclMigrations {
        brique: "noyau",
        plages: const { &[Plage::new(1, 999)] },
        migrations: MIGRATIONS_NOYAU,
    };
    let liste = assembler(&[decl]).expect("historique valide");
    assert_eq!(liste.len(), MIGRATIONS_NOYAU.len());
    assert!(liste.windows(2).all(|p| p[0].numero < p[1].numero));
}

/// Redéfinitions existantes (2026-11-07). Chacune est un piège de rejeu
/// (F-009) : on ne les corrige pas ici (ce serait changer le schéma), on
/// interdit d'en ajouter. `ent_eleve_dispositif_kind_chk` est en plus une
/// table du noyau redéfinie par une migration de la santé (212).
const REDEFINITIONS_CONNUES: &[(&str, &[&str])] = &[
    (
        "ent_absence_signal_statut_chk",
        &["012_absence_signal", "035_absence_signal_workflow"],
    ),
    (
        "ent_absence_statut_chk",
        &["010_absence_justif", "011_absence_decisions"],
    ),
    (
        "ent_affectation_source_chk",
        &["092_referentiel_scolaire", "110_referentiel_structure"],
    ),
    (
        "ent_eleve_dispositif_kind_chk",
        &["016_eleve_documents", "212_accompagnement"],
    ),
    (
        "ent_fac_encaissement_moyen_chk",
        &["025_encaissements", "030_sepa"],
    ),
    (
        "ent_sortie_controle_source_chk",
        &["062_sortie_autorisation", "065_sortie_site_edt"],
    ),
];

#[test]
fn aucune_nouvelle_contrainte_redefinie() {
    let trouvees = contraintes_redefinies(MIGRATIONS_NOYAU);
    let attendues: std::collections::BTreeMap<String, Vec<&str>> = REDEFINITIONS_CONNUES
        .iter()
        .map(|(n, l)| (n.to_string(), l.to_vec()))
        .collect();
    assert_eq!(
        trouvees, attendues,
        "contrainte redéfinie dans une nouvelle migration : élargir la migration d'origine (F-009)"
    );
}

#[test]
fn les_codes_du_catalogue_existant_ont_la_forme_du_kit() {
    // Lignes `Self::X => "domaine.verbe",` de `Action::code()`.
    let codes: Vec<&str> = ACTIONS_RS
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let reste = l.strip_prefix("Self::")?;
            let code = reste.split("=> \"").nth(1)?.strip_suffix("\",")?;
            code.contains('.').then_some(code)
        })
        .filter(|c| !c.contains(' '))
        .collect();
    let mut uniques = codes.clone();
    uniques.sort_unstable();
    uniques.dedup();
    assert_eq!(codes.len(), uniques.len(), "codes uniques");
    assert!(codes.len() >= 68, "{} codes relus", codes.len());
    let invalides: Vec<_> = codes.iter().filter(|c| !code_valide(c)).collect();
    assert!(invalides.is_empty(), "codes hors forme : {invalides:?}");
}
