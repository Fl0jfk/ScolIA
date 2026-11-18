//! Tests natifs du modèle d'écran des élèves (E9a et E9b). Un test par règle, dont chaque anomalie
//! corrigée en cours de route.

use super::*;
use chrono::{DateTime, NaiveDate, Utc};
use ent_types::eleve_synthese::{PastilleDto, TuileDto};
use ent_types::identite::IdentiteVersionDto;
use uuid::Uuid;

/// Identifiants distincts (`Uuid::new_v4` n'existe pas dans le navigateur : FRONTEND_LEPTOS §17).
fn identifiant() -> Uuid {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    Uuid::from_u128(u128::from(N.fetch_add(1, Ordering::Relaxed)))
}

fn jour(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

#[test]
fn designation_met_le_nom_en_capitales_et_ignore_les_espaces() {
    assert_eq!(designation(" Léa ", " Martin-Dupont "), "Léa MARTIN-DUPONT");
    assert_eq!(designation("", "Martin"), "MARTIN");
}

#[test]
fn statuts_lisibles_et_inconnu_sans_erreur() {
    assert_eq!(libelle_statut("actif"), "Scolarisé");
    assert_eq!(libelle_statut("preinscrit"), "Préinscrit");
    assert_eq!(libelle_statut("sorti"), "Sorti de l'établissement");
    assert_eq!(libelle_statut("autre"), "Statut inconnu");
    // Les valeurs du filtre sont celles que le serveur accepte, plus « tous ».
    let envoyees: Vec<&str> = STATUTS.iter().map(|(v, _)| *v).collect();
    assert_eq!(envoyees, ["", "actif", "preinscrit", "sorti"]);
}

#[test]
fn le_resume_de_page_dit_ou_l_on_en_est() {
    assert_eq!(resume_page(0, 0, 0), "Aucun élève");
    assert_eq!(resume_page(1, 0, 1), "1 élève");
    assert_eq!(resume_page(30, 0, 30), "30 élèves");
    assert_eq!(resume_page(312, 0, 50), "1 à 50 sur 312 élèves");
    assert_eq!(resume_page(312, 300, 12), "301 à 312 sur 312 élèves");
    // Page au-delà de la fin (filtres changés pendant la lecture) : rien d'absurde.
    assert_eq!(resume_page(40, 100, 0), "Aucun élève");
}

#[test]
fn la_pagination_ne_sort_jamais_des_bornes() {
    assert_eq!(page_precedente(0, 50), None);
    assert_eq!(page_precedente(50, 50), Some(0));
    assert_eq!(page_precedente(20, 50), Some(0));
    assert_eq!(page_precedente(100, 50), Some(50));
    assert_eq!(
        page_suivante(0, 50, 50),
        None,
        "exactement une page : pas de suivante"
    );
    assert_eq!(page_suivante(0, 50, 51), Some(50));
    assert_eq!(page_suivante(50, 50, 100), None);
    assert_eq!(page_suivante(50, 50, 101), Some(100));
    assert_eq!(
        page_suivante(0, 0, 5),
        Some(1),
        "taille nulle : au moins un pas"
    );
}

#[test]
fn une_seule_lettre_ne_cherche_rien_et_la_saisie_est_bornee() {
    assert_eq!(recherche_a_envoyer(""), "");
    assert_eq!(
        recherche_a_envoyer(" a "),
        "",
        "une lettre : liste complète"
    );
    assert_eq!(recherche_a_envoyer("  Léa  "), "Léa");
    assert_eq!(recherche_a_envoyer(&"x".repeat(500)).chars().count(), 100);
}

#[test]
fn le_raccourci_slash_ne_vole_pas_la_saisie() {
    assert!(ouvre_la_recherche("/", false, false, false, false));
    assert!(
        !ouvre_la_recherche("/", false, false, false, true),
        "pendant une saisie"
    );
    assert!(
        !ouvre_la_recherche("/", true, false, false, false),
        "Ctrl+/"
    );
    assert!(!ouvre_la_recherche("/", false, true, false, false), "⌘+/");
    assert!(!ouvre_la_recherche("a", false, false, false, false));
    assert!(
        est_zone_de_saisie("input", false)
            && est_zone_de_saisie("TEXTAREA", false)
            && est_zone_de_saisie("Select", false)
    );
    assert!(est_zone_de_saisie("DIV", true), "contenu éditable");
    assert!(!est_zone_de_saisie("BUTTON", false) && !est_zone_de_saisie("A", false));
}

#[test]
fn le_message_de_portee_n_apparait_que_si_la_liste_est_restreinte() {
    assert!(message_portee("etablissement").is_none());
    assert!(message_portee("perimetre").is_some());
    assert!(
        message_portee("quelconque").is_some(),
        "dans le doute, on explique"
    );
}

#[test]
fn la_liste_vide_explique_selon_les_filtres() {
    let (t1, m1) = message_liste_vide(true);
    let (t2, _) = message_liste_vide(false);
    assert_ne!(t1, t2);
    assert!(m1.contains("accent") && m1.contains("filtre"));
}

fn pastille(
    code: &str,
    gravite: &str,
    valeur: Option<&str>,
    date: Option<NaiveDate>,
) -> PastilleDto {
    PastilleDto {
        code: code.into(),
        domaine: "scolarite".into(),
        libelle: "Interne".into(),
        icone: "lit".into(),
        gravite: gravite.into(),
        valeur_code: valeur.map(str::to_string),
        valeur_date: date,
        du: Some(jour(2026, 9, 1)),
    }
}

#[test]
fn une_pastille_se_lit_en_toutes_lettres_valeur_comprise() {
    assert_eq!(
        phrase_pastille(&pastille("interne", "information", None, None)),
        "Interne"
    );
    assert_eq!(
        phrase_pastille(&pastille("interne", "information", Some("interne"), None)),
        "Interne (interne)"
    );
    assert_eq!(
        phrase_pastille(&pastille("pai", "alerte", None, Some(jour(2027, 6, 30)))),
        format!("Interne, jusqu'au {}", date_fr(jour(2027, 6, 30)))
    );
    assert_eq!(teinte_gravite("alerte"), "mal");
    assert_eq!(teinte_gravite("attention"), "avert");
    assert_eq!(teinte_gravite("information"), "info");
    assert_eq!(teinte_gravite("?"), "info");
}

#[test]
fn chaque_raison_de_masquage_est_expliquee_sans_rien_reveler() {
    for r in [
        Some("droit_manquant"),
        Some("hors_portee"),
        Some("reserve_au_personnel"),
        None,
        Some("inconnue"),
    ] {
        let t = raison_masquage(r);
        assert!(!t.is_empty());
        // Le texte dit pourquoi, jamais ce que la tuile contiendrait.
        for mot in ["PAI", "INE", "allerg", "motif"] {
            assert!(
                !t.contains(mot),
                "{mot} ne doit pas figurer dans l'explication d'un masquage"
            );
        }
    }
    assert_ne!(
        raison_masquage(Some("hors_portee")),
        raison_masquage(Some("reserve_au_personnel"))
    );
}

#[test]
fn les_onglets_se_retrouvent_par_leur_code_et_un_code_inconnu_retombe_sur_la_synthese() {
    for o in Onglet::TOUS {
        assert_eq!(Onglet::depuis_code(o.code()), o);
    }
    assert_eq!(Onglet::depuis_code(""), Onglet::Synthese);
    assert_eq!(
        Onglet::depuis_code("sante"),
        Onglet::Synthese,
        "un onglet qui n'existe pas n'ouvre rien d'autre"
    );
    assert_eq!(Onglet::depuis_code("<script>"), Onglet::Synthese);
}

/// Les liens du serveur pointent des écrans à venir : aucune tuile ne mène à une page introuvable.
#[test]
fn chaque_tuile_du_contrat_a_une_destination_connue_et_jamais_un_lien_mort() {
    let eleve = Uuid::nil();
    for domaine in [
        "identite",
        "identifiants",
        "scolarite",
        "classe",
        "sante",
        "protection",
    ] {
        match destination_tuile(domaine, eleve) {
            Destination::Onglet(_) | Destination::Aucune => {}
            Destination::Page(p) => {
                assert!(p.starts_with("/web/infirmerie/eleves/"), "{domaine} → {p}")
            }
        }
    }
    assert_eq!(
        destination_tuile("identite", eleve),
        Destination::Onglet(Onglet::Identite)
    );
    assert_eq!(
        destination_tuile("identifiants", eleve),
        Destination::Onglet(Onglet::Identite)
    );
    assert_eq!(destination_tuile("scolarite", eleve), Destination::Aucune);
    assert_eq!(
        destination_tuile("domaine_futur", eleve),
        Destination::Aucune
    );
    assert!(!corps_tuile_sans_ecran("scolarite").is_empty());
}

#[test]
fn une_tuile_visible_sans_pastille_ni_ecran_est_reperee() {
    let t = |domaine: &str, n: usize| TuileDto {
        domaine: domaine.into(),
        libelle: "X".into(),
        rang: 1,
        lien: Some("/x".into()),
        etat: "visible".into(),
        raison: None,
        pastilles: (0..n)
            .map(|_| pastille("a", "information", None, None))
            .collect(),
    };
    assert!(tuile_vide(&t("scolarite", 0)));
    assert!(!tuile_vide(&t("scolarite", 1)));
    assert!(
        !tuile_vide(&t("identite", 0)),
        "elle mène à l'onglet identité"
    );
}

fn version(effet: NaiveDate, nature: &str, nom: &str, prenoms: &[&str]) -> IdentiteVersionDto {
    IdentiteVersionDto {
        id: identifiant(),
        nom_naissance: nom.into(),
        nom_usage: None,
        prenoms: prenoms.iter().map(|p| (*p).to_string()).collect(),
        rang_prenom_usuel: 1,
        prenom_usage: None,
        sexe: Some("F".into()),
        date_naissance: Some(jour(2012, 3, 14)),
        naissance_commune: Some("Paris".into()),
        naissance_commune_cog: None,
        naissance_pays_cog: None,
        naissance_lieu_etranger: None,
        date_naissance_precision: "jour".into(),
        date_naissance_statut: "declaree".into(),
        nom_ecriture_origine: None,
        prenoms_ecriture_origine: None,
        nature: nature.into(),
        effet_du: effet,
        remplace_version_id: None,
        remplacee_par: None,
        motif_code: None,
        motif: None,
        piece_document_id: None,
        source: "saisie".into(),
        sensible: false,
        enregistre_le: DateTime::<Utc>::from_timestamp(
            effet.and_hms_opt(8, 0, 0).unwrap().and_utc().timestamp(),
            0,
        )
        .unwrap(),
        enregistre_par: None,
    }
}

#[test]
fn la_date_de_naissance_ne_s_invente_jamais() {
    let d = Some(jour(2012, 3, 14));
    assert_eq!(
        date_naissance_affichee(d, "jour", "declaree"),
        "mercredi 14 mars 2012"
    );
    assert_eq!(date_naissance_affichee(d, "mois", "declaree"), "mars 2012");
    assert_eq!(
        date_naissance_affichee(d, "annee", "presumee"),
        "2012 (date présumée)"
    );
    assert_eq!(
        date_naissance_affichee(d, "jour", "contestee"),
        "mercredi 14 mars 2012 (date contestée)"
    );
    assert_eq!(
        date_naissance_affichee(None, "inconnue", "declaree"),
        "Date de naissance inconnue"
    );
    assert_eq!(date_naissance_affichee(None, "jour", "declaree"), "—");
}

#[test]
fn les_lignes_d_identite_n_affichent_que_ce_que_la_version_contient() {
    let mut v = version(jour(2020, 9, 1), "creation", "Martin", &["Léa", "Rose"]);
    let l = lignes_identite(&v);
    let valeur = |nom: &str| {
        l.iter()
            .find(|x| x.libelle == nom)
            .map(|x| x.valeur.clone())
    };
    assert_eq!(valeur("Nom de naissance").as_deref(), Some("MARTIN"));
    assert_eq!(valeur("Prénoms").as_deref(), Some("Léa, Rose"));
    assert_eq!(valeur("Sexe à l'état civil").as_deref(), Some("Féminin"));
    assert_eq!(valeur("Lieu de naissance").as_deref(), Some("Paris"));
    assert!(
        valeur("Nom d'usage").is_none() && valeur("Prénom d'usage").is_none(),
        "pas de ligne vide d'usage"
    );
    v.nom_usage = Some("Dupont".into());
    v.prenom_usage = Some("Rose".into());
    let l = lignes_identite(&v);
    assert!(l
        .iter()
        .any(|x| x.libelle == "Nom d'usage" && x.valeur == "DUPONT"));
    assert!(l
        .iter()
        .any(|x| x.libelle == "Prénom d'usage" && x.valeur == "Rose"));
}

#[test]
fn la_version_courante_ignore_l_avenir_et_ce_qui_a_ete_corrige() {
    let ancienne = version(jour(2020, 9, 1), "creation", "Martin", &["Léa"]);
    let mut corrigee = version(jour(2021, 9, 1), "correction", "Marten", &["Léa"]);
    corrigee.remplacee_par = Some(identifiant());
    let bonne = version(jour(2021, 9, 2), "correction", "Martin", &["Léa"]);
    let future = version(
        jour(2030, 1, 1),
        "changement_etat_civil",
        "Durand",
        &["Léa"],
    );
    let toutes = [ancienne.clone(), corrigee, bonne.clone(), future];
    let c = version_courante(&toutes, jour(2026, 11, 17)).unwrap();
    assert_eq!(
        c.id, bonne.id,
        "ni la version corrigée, ni celle qui n'a pas encore d'effet"
    );
    assert_eq!(
        version_courante(&toutes, jour(2020, 10, 1)).unwrap().id,
        ancienne.id
    );
    assert!(version_courante(&toutes, jour(2019, 1, 1)).is_none());
    assert!(version_courante(&[], jour(2026, 1, 1)).is_none());
}

#[test]
fn la_frise_compare_chaque_version_a_la_precedente_dans_le_temps() {
    let a = version(jour(2020, 9, 1), "creation", "Martin", &["Léa"]);
    let mut b = version(jour(2022, 9, 1), "correction", "Martin", &["Léa", "Rose"]);
    b.motif = Some("d'après l'acte de naissance".into());
    b.piece_document_id = Some(identifiant());
    let mut c = version(
        jour(2024, 9, 1),
        "changement_etat_civil",
        "Durand",
        &["Léa", "Rose"],
    );
    c.date_naissance_statut = "presumee".into();
    // Désordre volontaire en entrée : la frise range elle-même, la plus récente d'abord.
    let etapes = etapes_frise(&[b.clone(), c.clone(), a.clone()]);
    assert_eq!(
        etapes.iter().map(|e| e.id).collect::<Vec<_>>(),
        [c.id, b.id, a.id]
    );
    assert!(
        etapes[2].champs.is_empty(),
        "la création n'« a rien changé »"
    );
    assert_eq!(etapes[1].champs, ["Prénoms"]);
    assert_eq!(etapes[0].champs, ["Nom de naissance", "Date de naissance"]);
    assert_eq!(etapes[1].nature, "Correction d'une erreur");
    assert_eq!(etapes[0].nature, "Changement d'état civil");
    assert!(
        etapes[1].avec_piece && etapes[1].motif.as_deref() == Some("d'après l'acte de naissance")
    );
    assert!(etapes_frise(&[]).is_empty());
}

#[test]
fn une_correction_se_distingue_d_un_changement_d_etat_civil() {
    assert_ne!(
        libelle_nature("correction"),
        libelle_nature("changement_etat_civil")
    );
    assert_ne!(
        ton_nature("correction"),
        ton_nature("changement_etat_civil")
    );
    assert_eq!(libelle_nature("n'importe quoi"), "Modification");
    assert_eq!(libelle_source("siecle"), "SIECLE");
    assert_eq!(libelle_source("zzz"), "Autre source");
}

#[test]
fn identifiants_et_consultations_ont_des_libelles_neutres() {
    assert_eq!(libelle_statut_identifiant("conteste"), "Contesté");
    assert_eq!(ton_statut_identifiant("conteste"), TonEtat::Mal);
    assert_eq!(ton_statut_identifiant("confirme"), TonEtat::Ok);
    assert_eq!(libelle_systeme("INE"), "INE");
    assert_eq!(libelle_systeme("BADGE"), "Badge");
    for a in [
        "fiche_protegee",
        "etat_civil",
        "identifiants",
        "coordonnees",
        "sensible",
        "pastilles_sensibles",
        "autre",
    ] {
        // Le libellé dit CE QUI a été lu, jamais la valeur.
        assert!(!libelle_consultation(a).is_empty());
    }
}
