//! Socle 2 (l'élève), lot E5 : propositions et paramétrage des champs. Scénarios contre la
//! base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.
//!
//! Chaque test remet à zéro le paramétrage de l'établissement de labo avant et après, pour ne
//! pas laisser un profil (ex. « public », qui interdit la modification locale de l'état civil)
//! aux autres suites.

use super::*;
use crate::identite::{CorrectionIdentite, UsageIdentite};
use crate::param_eleve::ModifChamp;
use crate::propositions::{DecisionProposition, DepotProposition, FiltrePropositions};
use chrono::{Duration, NaiveDate, Utc};
use domain_eleve::{
    CibleProposition, CreateEleve, ModifIdentite, ModificationLocale, NatureVersion,
    OrigineProposition, ProfilEtablissement, SaisieFamille, SourceFoi, SourceProposition,
};
use ent_auth::Role;
use serde_json::{json, Value};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { }).await;
    reset(&ent, cast.etablissement_id).await;
    Some((ent, cast))
}

async fn reset(ent: &PgEnt, etab: Uuid) {
    for t in [
        "ent_param_eleve_champ",
        "ent_param_eleve_regle_auto",
        "ent_param_eleve_profil",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
}

fn alea(n: usize) -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .map(|c| if c.is_ascii_digit() { (b'a' + (c as u8 - b'0')) as char } else { c })
        .take(n)
        .collect()
}

fn nom_unique(base: &str) -> String {
    format!("{base}{}", alea(5))
}

async fn eleve_avec(ent: &PgEnt, etab: Uuid, nom: &str, naissance: (i32, u32, u32)) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom_unique(nom),
        prenom: "Zélie".into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: NaiveDate::from_ymd_opt(naissance.0, naissance.1, naissance.2),
        numero_badge: None,
        redoublant: false,
        classe: Some("6A".into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

async fn piece(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, mime_type, source)
         VALUES ($1,$2,$3,'administratif','Acte de naissance','application/pdf','upload')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

fn depot(
    etab: Uuid,
    eleve: Uuid,
    cible: CibleProposition,
    source: SourceProposition,
    origine: Option<OrigineProposition>,
    champs: Vec<(&str, Value)>,
    piece: Option<Uuid>,
) -> DepotProposition {
    DepotProposition {
        etablissement_id: etab,
        eleve_id: eleve,
        cible,
        source,
        origine,
        source_ref: Some(Uuid::new_v4()),
        champs: champs.into_iter().map(|(c, v)| (c.to_string(), v)).collect(),
        piece_document_id: piece,
        commentaire: None,
        expire_le: None,
        depose_par: None,
    }
}

fn import_siecle(etab: Uuid, eleve: Uuid, champs: Vec<(&str, Value)>) -> DepotProposition {
    depot(
        etab,
        eleve,
        CibleProposition::Identite,
        SourceProposition::Import,
        Some(OrigineProposition::Siecle),
        champs,
        None,
    )
}

async fn nom_courant(ent: &PgEnt, eleve: Uuid) -> String {
    let (n,): (String,) = sqlx::query_as("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    n
}

async fn ouvrir_famille(ent: &PgEnt, etab: Uuid, admin: Uuid, champ: &str) {
    ent.parametrer_champ(
        Role::Admin.as_str(),
        etab,
        admin,
        ModifChamp {
            champ: champ.into(),
            saisie_famille: Some(SaisieFamille::Proposer),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

fn decision(etab: Uuid, id: Uuid, acteur: Uuid) -> DecisionProposition {
    DecisionProposition {
        proposition_id: id,
        etablissement_id: etab,
        acteur,
        ..Default::default()
    }
}

// ───────────────────────── Paramétrage ─────────────────────────

/// Sans réglage, rien ne change (modification libre) ; un profil s'applique en bloc, reste
/// ajustable champ par champ et devient « personnalisé » dès qu'on ajuste.
#[tokio::test]
async fn profils_d_etablissement_et_reglage_par_champ() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    let role_admin = Role::Admin.as_str();
    // Par défaut : aucun profil, tout libre, rien d'ouvert à la famille.
    let p0 = ent.parametrage_eleve(role_admin, etab, admin).await.unwrap();
    assert!(p0.profil.is_none());
    assert!(p0.champs.iter().all(|c| !c.regle
        && c.reglage.modification_locale == ModificationLocale::Libre
        && c.reglage.saisie_famille == SaisieFamille::Non));
    // Seul l'administrateur règle (eleves.parametrer).
    for role in [Role::Direction, Role::Administratif, Role::Cpe, Role::Prof] {
        let r = ent.parametrage_eleve(role.as_str(), etab, cast.direction.user_id).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role:?}");
    }
    // Profil public : SIECLE fait foi, état civil interdit localement, règles automatiques.
    let p = ent
        .appliquer_profil(role_admin, etab, admin, ProfilEtablissement::Public)
        .await
        .unwrap();
    assert_eq!(p.profil.as_deref(), Some("public"));
    let nom = p.champs.iter().find(|c| c.code == "nom_naissance").unwrap();
    assert_eq!(nom.reglage.source_qui_fait_foi, SourceFoi::Siecle);
    assert_eq!(nom.reglage.modification_locale, ModificationLocale::Interdite);
    assert_eq!(nom.regles_auto, vec!["import".to_string()]);
    let ine = p.champs.iter().find(|c| c.code == "ine").unwrap();
    assert_eq!(ine.reglage.saisie_famille, SaisieFamille::Non, "l'INE n'est jamais ouvert à la famille");
    let prenoms = p.champs.iter().find(|c| c.code == "prenoms").unwrap();
    assert!(prenoms.regles_auto.is_empty(), "un champ sensible n'a pas de règle automatique");
    // Ajustement d'un champ (réglable par champ) : le profil devient « personnalisé ».
    let c = ent
        .parametrer_champ(
            role_admin,
            etab,
            admin,
            ModifChamp {
                champ: "nom_naissance".into(),
                modification_locale: Some(ModificationLocale::ParProposition),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(c.reglage.modification_locale, ModificationLocale::ParProposition);
    let p = ent.parametrage_eleve(role_admin, etab, admin).await.unwrap();
    assert_eq!(p.profil.as_deref(), Some("personnalise"));
    // Bornes du logiciel : l'INE ne s'ouvre jamais à la famille ; « interdite » exige une
    // source externe ; champ et moment inconnus refusés.
    let bornes = [
        ModifChamp { champ: "ine".into(), saisie_famille: Some(SaisieFamille::Proposer), ..Default::default() },
        ModifChamp { champ: "nom_usage".into(), modification_locale: Some(ModificationLocale::Interdite), ..Default::default() },
        ModifChamp { champ: "n_importe_quoi".into(), visible: Some(false), ..Default::default() },
        ModifChamp { champ: "nom_naissance".into(), obligatoire_a: Some(vec!["demain".into()]), ..Default::default() },
    ];
    for m in bornes {
        let r = ent.parametrer_champ(role_admin, etab, admin, m).await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    }
    // Privé sous contrat : l'établissement fait foi, tout reste libre, aucune règle automatique.
    let p = ent
        .appliquer_profil(role_admin, etab, admin, ProfilEtablissement::PriveSousContrat)
        .await
        .unwrap();
    assert!(p.champs.iter().all(|c| c.regles_auto.is_empty()
        && c.reglage.modification_locale == ModificationLocale::Libre));
    assert_eq!(p.profil.as_deref(), Some("prive_sous_contrat"));
    let r = ent
        .appliquer_profil(role_admin, etab, admin, ProfilEtablissement::Personnalise)
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
    // Journal et événement : noms de champs, jamais de valeur.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'parametrage.eleve_modifie'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(n >= 3);
    reset(&ent, etab).await;
}

/// Une règle automatique n'existe que si elle peut s'appliquer ; elle disparaît si la source
/// qui fait foi n'est plus externe.
#[tokio::test]
async fn regle_automatique_explicite_et_coherente() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    let role = Role::Admin.as_str();
    // Sans source externe qui fait foi : refusée.
    let r = ent
        .definir_regle_auto(role, etab, admin, "nom_naissance", SourceProposition::Import, true)
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    ent.parametrer_champ(
        role,
        etab,
        admin,
        ModifChamp {
            champ: "nom_naissance".into(),
            source_qui_fait_foi: Some(SourceFoi::Siecle),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    ent.definir_regle_auto(role, etab, admin, "nom_naissance", SourceProposition::Import, true)
        .await
        .unwrap();
    // Un champ sensible n'a jamais de règle ; la famille n'est pas un flux machine.
    for (champ, source) in [("prenom_usage", SourceProposition::Import), ("nom_naissance", SourceProposition::Famille)] {
        let r = ent.definir_regle_auto(role, etab, admin, champ, source, true).await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{champ} {r:?}");
    }
    // L'établissement reprend la main : la règle devenue trompeuse est retirée.
    let c = ent
        .parametrer_champ(
            role,
            etab,
            admin,
            ModifChamp {
                champ: "nom_naissance".into(),
                source_qui_fait_foi: Some(SourceFoi::Scolia),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(c.regles_auto.is_empty());
    reset(&ent, etab).await;
}

/// Champ verrouillé ou source externe qui fait foi : les commandes directes d'identité et
/// d'identifiant sont refusées ; sans réglage, tout reste libre (aucun changement de
/// comportement pour un établissement existant).
#[tokio::test]
async fn champ_verrouille_ou_source_externe_refuse_la_modification_directe() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = eleve_avec(&ent, etab, "Direct", (2011, 4, 2)).await;
    let correction = |nom: &str| CorrectionIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        version_id: None,
        modif: ModifIdentite { nom_naissance: Some(nom.into()), ..Default::default() },
        motif_code: "faute_de_frappe".into(),
        motif: None,
        piece_document_id: None,
        source: "saisie".into(),
        acteur: secr,
    };
    // Sans réglage : libre.
    ent.corriger_identite(role, correction(&nom_unique("Libre"))).await.unwrap();
    // Profil public : SIECLE fait foi → interdit.
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public)
        .await
        .unwrap();
    let r = ent.corriger_identite(role, correction(&nom_unique("Interdit"))).await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    // L'INE saisi à la main : refusé aussi.
    let r = ent
        .ajouter_identifiant(
            role,
            crate::identifiants::AjoutIdentifiant {
                etablissement_id: etab,
                eleve_id: e.id,
                systeme: domain_eleve::SystemeIdentifiant::Ine,
                valeur: format!("1234{}", alea(7).to_uppercase()),
                provenance: domain_eleve::ProvenanceIdentifiant::Saisie,
                valide_du: None,
                preuve_document_id: None,
                acteur: Some(secr),
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    // Le prénom d'usage reste local et libre ; le badge n'a pas de verrou.
    ent.definir_usage(
        role,
        UsageIdentite {
            etablissement_id: etab,
            eleve_id: e.id,
            nom_usage: Some(Some(nom_unique("Usage"))),
            prenom_usage: None,
            effet_du: None,
            motif_code: None,
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: secr,
        },
    )
    .await
    .unwrap();
    // Verrou total : même un champ libre devient intouchable.
    ent.parametrer_champ(
        Role::Admin.as_str(),
        etab,
        admin,
        ModifChamp { champ: "nom_usage".into(), verrouille: Some(true), ..Default::default() },
    )
    .await
    .unwrap();
    let r = ent
        .definir_usage(
            role,
            UsageIdentite {
                etablissement_id: etab,
                eleve_id: e.id,
                nom_usage: Some(Some(nom_unique("Gele"))),
                prenom_usage: None,
                effet_du: Some(domain_eleve::horloge::jour_paris() + Duration::days(1)),
                motif_code: None,
                motif: None,
                piece_document_id: None,
                source: "saisie".into(),
                acteur: secr,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    reset(&ent, etab).await;
}

// ───────────────────────── Aucune écriture externe sans proposition ─────────────────────────

/// Critère de fin : un import SIECLE n'écrit jamais directement. Avec une règle explicite, la
/// proposition est appliquée automatiquement (tracée) ; un import rejoué ne crée rien ; et la
/// base elle-même refuse une version de source externe qui ne référence pas une proposition
/// décidée.
#[tokio::test]
async fn import_passe_par_une_proposition_et_la_base_refuse_le_contournement() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public)
        .await
        .unwrap();
    let e = eleve_avec(&ent, etab, "Import", (2011, 4, 2)).await;
    let nouveau = nom_unique("Siecle");
    let r = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nouveau))]))
        .await
        .unwrap();
    let auto = r.appliquee_auto.expect("règle explicite du profil public");
    assert!(r.en_attente.is_none());
    assert_eq!(auto.entete.statut, "appliquee_auto");
    assert_eq!(auto.entete.regle_auto.as_deref(), Some("regle_auto:import"));
    assert_eq!(auto.champs[0].decision, "acceptee");
    assert_eq!(nom_courant(&ent, e.id).await, nouveau);
    // La version porte la proposition en `source_ref`, nature import, source import.
    let (nature, source, sref): (String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT nature, source, source_ref FROM ent_eleve_identite
         WHERE eleve_id = $1 AND remplacee_par IS NULL ORDER BY effet_du DESC, enregistre_le DESC LIMIT 1",
    )
    .bind(e.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((nature.as_str(), source.as_str(), sref), ("import", "import", Some(auto.entete.id)));
    // Import rejoué : aucune différence, rien n'est créé.
    let r2 = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nouveau))]))
        .await
        .unwrap();
    assert!(r2.aucune_difference());
    assert_eq!(r2.champs_identiques, vec!["nom_naissance".to_string()]);
    // Le contournement échoue en base : version de source externe sans proposition décidée.
    let base = {
        let mut conn = ent.pool_verif().acquire().await.unwrap();
        crate::identite::PgIdentiteRepo::en_vigueur_a(&mut conn, etab, e.id, domain_eleve::horloge::jour_paris())
            .await
            .unwrap()
            .unwrap()
    };
    for (source_ref, source) in [(None, "import"), (Some(Uuid::new_v4()), "famille"), (None, "eleve")] {
        let mut v = base.clone();
        v.id = Uuid::new_v4();
        v.nature = NatureVersion::Correction;
        v.nom_naissance = nom_unique("Direct");
        v.remplace_version_id = Some(base.id);
        v.remplacee_par = None;
        v.source = source.into();
        v.source_ref = source_ref;
        v.effet_du = base.effet_du;
        let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool_verif(), etab).await.unwrap();
        let err = sqlx::query(
            "INSERT INTO ent_eleve_identite (id, etablissement_id, eleve_id, nom_naissance, prenoms, rang_prenom_usuel,
                date_naissance_precision, date_naissance_statut, nature, effet_du, source, source_ref, enregistre_par)
             VALUES ($1,$2,$3,$4,$5,1,'jour','declaree','correction',$6,$7,$8,NULL)",
        )
        .bind(v.id)
        .bind(etab)
        .bind(e.id)
        .bind(&v.nom_naissance)
        .bind(&v.prenoms)
        .bind(base.effet_du + Duration::days(7))
        .bind(source)
        .bind(source_ref)
        .execute(&mut *tx)
        .await
        .unwrap_err();
        let code = match &err {
            sqlx::Error::Database(d) => d.code().map(|c| c.to_string()),
            _ => None,
        };
        assert_eq!(code.as_deref(), Some("42501"), "{source} : {err}");
    }
    // Un identifiant de flux officiel n'entre pas par le chemin système direct.
    let r = ent
        .ajouter_identifiant_systeme(crate::identifiants::AjoutIdentifiant {
            etablissement_id: etab,
            eleve_id: e.id,
            systeme: domain_eleve::SystemeIdentifiant::Ine,
            valeur: format!("9876{}", alea(7).to_uppercase()),
            provenance: domain_eleve::ProvenanceIdentifiant::ImportAaf,
            valide_du: None,
            preuve_document_id: None,
            acteur: None,
        })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    reset(&ent, etab).await;
}

/// Règles automatiques : champ sensible, origine qui ne fait pas foi, champ verrouillé et
/// absence de règle laissent la proposition en attente, avec la raison ; un lot mixte est
/// partagé (l'automatique s'applique, le reste attend une personne).
#[tokio::test]
async fn regles_automatiques_ne_s_appliquent_pas_aux_champs_sensibles_ni_hors_source() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    let role = Role::Admin.as_str();
    ent.appliquer_profil(role, etab, admin, ProfilEtablissement::Public).await.unwrap();
    let e = eleve_avec(&ent, etab, "Mixte", (2011, 4, 2)).await;
    let nouveau = nom_unique("Mixte");
    // Lot SIECLE mixte : nom (automatique) + prénoms (sensible → décision humaine).
    let r = ent
        .deposer_proposition_systeme(import_siecle(
            etab,
            e.id,
            vec![("nom_naissance", json!(nouveau)), ("prenoms", json!(["Aurélie", "Zoé"]))],
        ))
        .await
        .unwrap();
    assert_eq!(r.appliquee_auto.as_ref().unwrap().champs.len(), 1);
    let attente = r.en_attente.as_ref().expect("les prénoms attendent une personne");
    assert_eq!(attente.champs[0].champ, "prenoms");
    assert_eq!(r.raisons_non_auto, vec![("prenoms".to_string(), "champ_sensible")]);
    assert_eq!(nom_courant(&ent, e.id).await, nouveau);
    // Origine qui ne fait pas foi (reprise alors que SIECLE fait foi) : en attente.
    let autre = nom_unique("Reprise");
    let r = ent
        .deposer_proposition_systeme(depot(
            etab,
            e.id,
            CibleProposition::Identite,
            SourceProposition::Import,
            Some(OrigineProposition::Reprise),
            vec![("nom_naissance", json!(autre))],
            None,
        ))
        .await
        .unwrap();
    assert!(r.appliquee_auto.is_none());
    assert_eq!(r.raisons_non_auto, vec![("nom_naissance".to_string(), "source_ne_fait_pas_foi")]);
    assert_eq!(nom_courant(&ent, e.id).await, nouveau, "la fiche n'a pas bougé");
    // Champ verrouillé : même la source qui fait foi attend.
    ent.parametrer_champ(role, etab, admin, ModifChamp { champ: "nom_naissance".into(), verrouille: Some(true), ..Default::default() })
        .await
        .unwrap();
    let encore = nom_unique("Gele");
    let r = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(encore))]))
        .await
        .unwrap();
    assert!(r.appliquee_auto.is_none());
    assert_eq!(r.raisons_non_auto[0].1, "champ_verrouille");
    let id = r.en_attente.unwrap().entete.id;
    let refuse = ent
        .decider_proposition(
            Role::Administratif.as_str(),
            DecisionProposition { champs_acceptes: vec!["nom_naissance".into()], ..decision(etab, id, cast.administratif.user_id) },
        )
        .await;
    assert!(matches!(refuse, Err(EntDbError::Refus(_, _))), "{refuse:?}");
    // Sans règle active : rien ne s'applique seul.
    ent.parametrer_champ(role, etab, admin, ModifChamp { champ: "nom_naissance".into(), verrouille: Some(false), ..Default::default() })
        .await
        .unwrap();
    ent.definir_regle_auto(role, etab, admin, "nom_naissance", SourceProposition::Import, false)
        .await
        .unwrap();
    let r = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nom_unique("SansRegle")))]))
        .await
        .unwrap();
    assert!(r.appliquee_auto.is_none());
    assert_eq!(r.raisons_non_auto[0].1, "aucune_regle");
    reset(&ent, etab).await;
}

/// L'INE proposé par un import officiel : règle explicite → confirmé, provenance AAF ;
/// valeur invalide refusée au dépôt ; valeur déjà portée par l'élève : rien.
#[tokio::test]
async fn ine_par_proposition_import_officiel() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public)
        .await
        .unwrap();
    let e = eleve_avec(&ent, etab, "Ine", (2011, 4, 2)).await;
    let valeur = format!("5555{}", alea(7).to_uppercase());
    let d = |v: &str, origine| {
        depot(
            etab,
            e.id,
            CibleProposition::Identifiant,
            SourceProposition::Import,
            Some(origine),
            vec![("ine", json!(v))],
            None,
        )
    };
    let r = ent.deposer_proposition_systeme(d("court", OrigineProposition::Aaf)).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = ent.deposer_proposition_systeme(d(&valeur.to_lowercase(), OrigineProposition::Aaf)).await.unwrap();
    assert!(r.appliquee_auto.is_some(), "{r:?}");
    let (statut, prov, v): (String, String, String) = sqlx::query_as(
        "SELECT statut, provenance, valeur FROM ent_eleve_identifiant WHERE eleve_id = $1 AND systeme = 'INE' AND valide_au IS NULL",
    )
    .bind(e.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((statut.as_str(), prov.as_str(), v.as_str()), ("confirme", "import_aaf", valeur.as_str()));
    let r = ent.deposer_proposition_systeme(d(&valeur, OrigineProposition::Aaf)).await.unwrap();
    assert!(r.aucune_difference());
    // Un INE confirmé n'est jamais écrasé en silence par une valeur moins sûre (reprise) :
    // la proposition n'a aucune règle (la reprise ne fait pas foi), une personne l'accepte,
    // et l'application est refusée tant que l'INE confirmé n'est pas révoqué.
    let moins_sur = format!("7777{}", alea(7).to_uppercase());
    let r = ent.deposer_proposition_systeme(d(&moins_sur, OrigineProposition::Reprise)).await.unwrap();
    assert!(r.appliquee_auto.is_none());
    let id = r.en_attente.unwrap().entete.id;
    // Sous SIECLE, un INE de la reprise ne s'accepte même pas à la main (champ interdit
    // localement) ; l'établissement qui reprend la main (privé) peut l'accepter… et butte
    // alors sur l'INE confirmé.
    let r_sous_siecle = ent
        .decider_proposition(
            Role::Administratif.as_str(),
            DecisionProposition { champs_acceptes: vec!["ine".into()], ..decision(etab, id, cast.administratif.user_id) },
        )
        .await;
    assert!(matches!(r_sous_siecle, Err(EntDbError::Refus(_, _))), "{r_sous_siecle:?}");
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::PriveSousContrat)
        .await
        .unwrap();
    let refus = ent
        .decider_proposition(
            Role::Administratif.as_str(),
            DecisionProposition { champs_acceptes: vec!["ine".into()], ..decision(etab, id, cast.administratif.user_id) },
        )
        .await;
    assert!(matches!(refus, Err(EntDbError::ConflitDossier(_))), "{refus:?}");
    let (actif,): (String,) = sqlx::query_as(
        "SELECT valeur FROM ent_eleve_identifiant WHERE eleve_id = $1 AND systeme = 'INE' AND valide_au IS NULL",
    )
    .bind(e.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(actif, valeur, "l'INE confirmé est intact");
    // Retour au profil public (règles automatiques actives) pour la panne ci-dessous.
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public)
        .await
        .unwrap();
    // Panne d'application automatique (déclencheur de panne, comme au lot E0) : la
    // proposition reste EN ATTENTE pour une décision humaine, rien n'est à moitié écrit.
    sqlx::query(
        "CREATE OR REPLACE FUNCTION ent_test_panne_ine() RETURNS TRIGGER LANGUAGE plpgsql AS $$
         BEGIN IF NEW.valeur LIKE 'PANNE%' THEN RAISE EXCEPTION 'panne simulée' USING ERRCODE = '23514'; END IF; RETURN NEW; END $$",
    )
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_panne_ine ON ent_eleve_identifiant").execute(ent.pool_verif()).await.unwrap();
    sqlx::query("CREATE TRIGGER ent_test_panne_ine BEFORE INSERT ON ent_eleve_identifiant FOR EACH ROW EXECUTE FUNCTION ent_test_panne_ine()")
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let panne = format!("PANNE{}", alea(7).to_uppercase());
    let r = ent.deposer_proposition_systeme(d(&panne, OrigineProposition::Aaf)).await;
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_panne_ine ON ent_eleve_identifiant").execute(ent.pool_verif()).await.unwrap();
    let r = r.unwrap();
    assert!(r.appliquee_auto.is_none());
    assert!(r.raisons_non_auto.iter().any(|(_, m)| *m == "application_impossible"), "{r:?}");
    let attente = r.en_attente.expect("la proposition attend une personne");
    assert_eq!(attente.entete.statut, "en_attente");
    assert_eq!(attente.champs[0].decision, "en_attente");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_identifiant WHERE eleve_id = $1 AND valeur LIKE 'PANNE%'")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
    reset(&ent, etab).await;
}

// ───────────────────────── Famille ─────────────────────────

/// ELV-18 : la famille ne propose que ce que l'établissement ouvre (jamais l'INE), avec une
/// pièce pour l'état civil ; seul un responsable avec lien actif dépose ; la famille ne voit
/// que ses propres propositions, jamais la valeur actuelle de la fiche.
#[tokio::test]
async fn famille_ne_propose_que_ce_qui_est_autorise() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    let nouveau = nom_unique("Famille");
    let cmd = |champs: Vec<(&str, Value)>, piece| {
        depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None, champs, piece)
    };
    // Rien n'est ouvert par défaut.
    let r = ent.deposer_proposition_famille("parent", cmd(vec![("nom_naissance", json!(nouveau))], None), parent).await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    assert!(ent.champs_signalables("parent", etab, parent, lea).await.unwrap().is_empty());
    ouvrir_famille(&ent, etab, admin, "nom_naissance").await;
    let s = ent.champs_signalables("parent", etab, parent, lea).await.unwrap();
    assert_eq!(s.iter().map(|c| c.code).collect::<Vec<_>>(), vec!["nom_naissance"]);
    assert!(s[0].piece_requise);
    // Pièce obligatoire pour l'état civil ; pièce d'un autre élève refusée.
    let r = ent.deposer_proposition_famille("parent", cmd(vec![("nom_naissance", json!(nouveau))], None), parent).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let autre_eleve = eleve_avec(&ent, etab, "Autre", (2011, 4, 2)).await;
    let pj_autre = piece(&ent, etab, autre_eleve.id).await;
    let r = ent.deposer_proposition_famille("parent", cmd(vec![("nom_naissance", json!(nouveau))], Some(pj_autre)), parent).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let pj = piece(&ent, etab, lea).await;
    // Un champ non ouvert dans le même lot fait échouer tout le dépôt.
    let r = ent
        .deposer_proposition_famille(
            "parent",
            cmd(vec![("nom_naissance", json!(nouveau)), ("date_naissance", json!("2010-01-01"))], Some(pj)),
            parent,
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    // L'INE n'est jamais proposable par la famille, même si on tente de l'ouvrir.
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(etab, lea, CibleProposition::Identifiant, SourceProposition::Famille, None, vec![("ine", json!("ABCDEFGH123"))], Some(pj)),
            parent,
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    // Dépôt valide : en attente, rien n'a changé dans la fiche.
    let avant = nom_courant(&ent, lea).await;
    let r = ent
        .deposer_proposition_famille("parent", cmd(vec![("nom_naissance", json!(nouveau))], Some(pj)), parent)
        .await
        .unwrap();
    let p = r.en_attente.expect("jamais appliquée seule");
    assert!(r.appliquee_auto.is_none());
    assert_eq!(p.entete.source, "famille");
    assert_eq!(p.entete.depose_par, Some(parent));
    assert_eq!(nom_courant(&ent, lea).await, avant);
    // Ni un autre parent, ni un compte non responsable, ni le personnel ne déposent « en famille ».
    for (role, user) in [("parent", cast.parent_tom.user_id), ("prof", cast.prof.user_id), ("administratif", cast.administratif.user_id)] {
        let r = ent
            .deposer_proposition_famille(role, cmd(vec![("nom_naissance", json!(nom_unique("Intrus")))], Some(pj)), user)
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} {r:?}");
        let r = ent.mes_propositions(role, etab, user, lea).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} {r:?}");
    }
    // La famille voit sa proposition, sans la valeur actuelle de la fiche.
    let mes = ent.mes_propositions("parent", etab, parent, lea).await.unwrap();
    assert!(mes.iter().any(|m| m.entete.id == p.entete.id && m.champs[0].actuel.is_none() && m.champs[0].propose == json!(nouveau)));
    // Une proposition plus récente remplace l'ancienne (obsolète).
    let n2 = nom_unique("FamilleBis");
    ent.deposer_proposition_famille("parent", cmd(vec![("nom_naissance", json!(n2))], Some(pj)), parent)
        .await
        .unwrap();
    let mes = ent.mes_propositions("parent", etab, parent, lea).await.unwrap();
    assert_eq!(mes.iter().find(|m| m.entete.id == p.entete.id).unwrap().entete.statut, "obsolete");
    // Un tiers ne retire pas la demande d'un autre ; le déposant le peut.
    let en_cours = mes.iter().find(|m| m.entete.statut == "en_attente").unwrap().entete.id;
    let r = ent.retirer_proposition(etab, cast.parent_tom.user_id, en_cours).await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");
    ent.retirer_proposition(etab, parent, en_cours).await.unwrap();
    let r = ent.retirer_proposition(etab, parent, en_cours).await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    reset(&ent, etab).await;
}

/// Élève : seul l'élève majeur (ou émancipé) dépose pour lui-même ; un mineur passe par ses
/// responsables.
#[tokio::test]
async fn eleve_majeur_ou_emancipe_depose_pour_lui_meme() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    // Un compte ne porte qu'un élève par établissement (unicité) : un compte par cas, nettoyé
    // avant et après pour ne rien laisser aux autres suites.
    let (compte, compte_mineur) = (cast.parent_tom.user_id, cast.cpe.user_id);
    let nettoyer = || async {
        sqlx::query("DELETE FROM ent_eleve_compte WHERE etablissement_id = $1 AND user_id = ANY($2)")
            .bind(etab)
            .bind(vec![compte, compte_mineur])
            .execute(ent.pool_verif())
            .await
            .unwrap();
    };
    nettoyer().await;
    let majeur = eleve_avec(&ent, etab, "Majeur", (2000, 1, 15)).await;
    let mineur = eleve_avec(&ent, etab, "Mineur", (2012, 1, 15)).await;
    for (e, u) in [(&majeur, compte), (&mineur, compte_mineur)] {
        sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id, created_at) VALUES ($1,$2,$3, now())")
            .bind(etab)
            .bind(e.id)
            .bind(u)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    ouvrir_famille(&ent, etab, admin, "nom_usage").await;
    let cmd = |eleve| depot(etab, eleve, CibleProposition::Identite, SourceProposition::Eleve, None, vec![("nom_usage", json!(nom_unique("Usage")))], None);
    let r = ent.deposer_proposition_famille("eleve", cmd(majeur.id), compte).await.unwrap();
    assert_eq!(r.en_attente.unwrap().entete.source, "eleve");
    let r = ent.deposer_proposition_famille("eleve", cmd(mineur.id), compte_mineur).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "mineur : {r:?}");
    // Un compte « élève » qui n'est pas celui de cet élève : refusé.
    let r = ent.deposer_proposition_famille("eleve", cmd(majeur.id), cast.parent_lea.user_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Émancipé : capable comme un majeur.
    let pj = piece(&ent, etab, mineur.id).await;
    sqlx::query(
        "INSERT INTO ent_eleve_capacite (id, etablissement_id, eleve_id, nature, du, piece_document_id)
         VALUES ($1,$2,$3,'emancipation', current_date - 1, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(mineur.id)
    .bind(pj)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = ent.deposer_proposition_famille("eleve", cmd(mineur.id), compte_mineur).await.unwrap();
    assert!(r.en_attente.is_some());
    nettoyer().await;
    reset(&ent, etab).await;
}

// ───────────────────────── Décision ─────────────────────────

/// ELV-19 : acceptation partielle, nature choisie, refus motivé et visible du déposant ;
/// une proposition décidée ne se décide pas deux fois ; seul l'habilité décide.
#[tokio::test]
async fn decision_partielle_nature_choisie_et_refus_motive() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    let role = Role::Administratif.as_str();
    ouvrir_famille(&ent, etab, admin, "nom_naissance").await;
    ouvrir_famille(&ent, etab, admin, "date_naissance").await;
    let pj = piece(&ent, etab, lea).await;
    let nouveau = nom_unique("Decide");
    let (actuelle,): (Option<NaiveDate>,) = sqlx::query_as("SELECT date_naissance FROM ent_eleve WHERE id = $1")
        .bind(lea)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let autre_date = actuelle.unwrap() + Duration::days(40);
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(
                etab,
                lea,
                CibleProposition::Identite,
                SourceProposition::Famille,
                None,
                vec![("nom_naissance", json!(nouveau)), ("date_naissance", json!(autre_date.to_string()))],
                Some(pj),
            ),
            parent,
        )
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    // Habilitation : le professeur, le parent et la direction sans l'action ne décident pas.
    for (role, user) in [("prof", cast.prof.user_id), ("parent", parent), ("cpe", cast.cpe.user_id)] {
        let r = ent
            .decider_proposition(role, DecisionProposition { champs_acceptes: vec!["nom_naissance".into()], ..decision(etab, id, user) })
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} {r:?}");
        let l = ent.propositions_pour_acteur(role, etab, user, FiltrePropositions::default()).await;
        assert!(matches!(l, Err(EntDbError::Forbidden)), "{role}");
    }
    // Nature à choisir pour une proposition de la famille.
    let r = ent
        .decider_proposition(role, DecisionProposition { champs_acceptes: vec!["nom_naissance".into()], refus_code: Some("piece_insuffisante".into()), ..decision(etab, id, secr) })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Un refus exige un motif codé.
    let r = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into()],
            nature: Some(NatureVersion::Correction),
            motif_code: Some("d_apres_piece".into()),
            ..decision(etab, id, secr)
        })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Champ absent de la proposition.
    let r = ent
        .decider_proposition(role, DecisionProposition { champs_acceptes: vec!["sexe".into()], ..decision(etab, id, secr) })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Acceptation partielle : le nom comme correction (d'après la pièce), la date refusée.
    let p = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into()],
            nature: Some(NatureVersion::Correction),
            motif_code: Some("d_apres_piece".into()),
            refus_code: Some("valeur_incorrecte".into()),
            refus_motif: Some("La date du livret de famille est celle de la fiche.".into()),
            ..decision(etab, id, secr)
        })
        .await
        .unwrap();
    assert_eq!(p.entete.statut, "acceptee_partiellement");
    assert_eq!(p.entete.decide_par, Some(secr));
    assert_eq!(p.champs.iter().find(|c| c.champ == "nom_naissance").unwrap().decision, "acceptee");
    assert_eq!(p.champs.iter().find(|c| c.champ == "date_naissance").unwrap().decision, "refusee");
    assert_eq!(nom_courant(&ent, lea).await, nouveau);
    let (nature, source, motif, sref, piece_v): (String, String, Option<String>, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT nature, source, motif_code, source_ref, piece_document_id FROM ent_eleve_identite
         WHERE eleve_id = $1 AND remplacee_par IS NULL ORDER BY effet_du DESC, enregistre_le DESC LIMIT 1",
    )
    .bind(lea)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((nature.as_str(), source.as_str(), motif.as_deref(), sref), ("correction", "famille", Some("d_apres_piece"), Some(id)));
    assert!(piece_v.is_none() || piece_v == Some(pj));
    let (d,): (Option<NaiveDate>,) = sqlx::query_as("SELECT date_naissance FROM ent_eleve WHERE id = $1").bind(lea).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(d, actuelle, "la date refusée n'a pas bougé");
    // Le déposant voit la décision et le motif (pas de valeur actuelle).
    let mes = ent.mes_propositions("parent", etab, parent, lea).await.unwrap();
    let m = mes.iter().find(|m| m.entete.id == id).unwrap();
    assert_eq!(m.entete.statut, "acceptee_partiellement");
    assert_eq!(m.entete.decision_code.as_deref(), Some("valeur_incorrecte"));
    assert!(m.entete.decision_motif.as_deref().unwrap().contains("livret"));
    // Déjà décidée : une seconde décision échoue, et la proposition est immuable en base.
    let r = ent
        .decider_proposition(role, DecisionProposition { champs_acceptes: vec![], refus_code: Some("autre".into()), refus_motif: Some("x".into()), ..decision(etab, id, secr) })
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let r = sqlx::query("UPDATE ent_eleve_proposition SET statut = 'acceptee' WHERE id = $1").bind(id).execute(ent.pool_verif()).await;
    assert!(r.is_err());
    let r = sqlx::query("DELETE FROM ent_eleve_proposition WHERE id = $1").bind(id).execute(ent.pool_verif()).await;
    assert!(r.is_err());
    reset(&ent, etab).await;
}

/// Un changement d'état civil exige une pièce, un motif codé et se date ; le nom d'usage
/// demandé par la famille devient une version `usage` (jamais une correction).
#[tokio::test]
async fn nature_changement_etat_civil_et_usage() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    let role = Role::Administratif.as_str();
    ouvrir_famille(&ent, etab, admin, "nom_naissance").await;
    ouvrir_famille(&ent, etab, admin, "nom_usage").await;
    let pj = piece(&ent, etab, lea).await;
    let (nom, usage) = (nom_unique("Jugement"), nom_unique("Usuel"));
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None,
                vec![("nom_naissance", json!(nom)), ("nom_usage", json!(usage))], Some(pj)),
            parent,
        )
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    // Changement d'état civil sans motif codé : refusé.
    let r = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into(), "nom_usage".into()],
            nature: Some(NatureVersion::ChangementEtatCivil),
            ..decision(etab, id, secr)
        })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Rien n'a été écrit par la tentative ratée (transaction défaite).
    assert_ne!(nom_courant(&ent, lea).await, nom);
    let p = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into(), "nom_usage".into()],
            nature: Some(NatureVersion::ChangementEtatCivil),
            motif_code: Some("decision_etat_civil".into()),
            effet_du: Some(domain_eleve::horloge::jour_paris() + Duration::days(1)),
            ..decision(etab, id, secr)
        })
        .await
        .unwrap();
    assert_eq!(p.entete.statut, "acceptee");
    // Une seule version porte l'état civil ET le nom d'usage (un instantané ne se coupe pas) ;
    // datée de demain, elle n'est pas encore la version courante.
    let lignes: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT nature, nom_usage, nom_naissance FROM ent_eleve_identite WHERE eleve_id = $1 AND source_ref = $2",
    )
    .bind(lea)
    .bind(id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes, vec![("changement_etat_civil".to_string(), Some(usage.clone()), nom.clone())]);
    assert_ne!(nom_courant(&ent, lea).await, nom, "datée de demain");
    // Un nom d'usage seul est une version de nature `usage` (motif : demande de la famille).
    let usage2 = nom_unique("Usuel");
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None,
                vec![("nom_usage", json!(usage2))], None),
            parent,
        )
        .await
        .unwrap();
    let id2 = r.en_attente.unwrap().entete.id;
    ent.decider_proposition(role, DecisionProposition { champs_acceptes: vec!["nom_usage".into()], ..decision(etab, id2, secr) })
        .await
        .unwrap();
    let (nature, motif): (String, Option<String>) = sqlx::query_as(
        "SELECT nature, motif_code FROM ent_eleve_identite WHERE eleve_id = $1 AND source_ref = $2",
    )
    .bind(lea)
    .bind(id2)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((nature.as_str(), motif.as_deref()), ("usage", Some("demande_famille")));
    assert_eq!(nom_courant(&ent, lea).await, usage2);
    reset(&ent, etab).await;
}

/// Public : une demande locale sur un champ géré par SIECLE ne peut qu'être refusée
/// (« à corriger dans la source ») ; la proposition SIECLE, elle, s'accepte.
#[tokio::test]
async fn champ_gere_par_siecle_refuse_la_demande_locale() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    let role = Role::Administratif.as_str();
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public).await.unwrap();
    // La famille peut signaler (profil : proposer) …
    let pj = piece(&ent, etab, lea).await;
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None,
                vec![("nom_naissance", json!(nom_unique("Signale")))], Some(pj)),
            parent,
        )
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    // … mais le secrétariat ne peut pas l'accepter localement.
    let r = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into()],
            nature: Some(NatureVersion::Correction),
            motif_code: Some("d_apres_piece".into()),
            ..decision(etab, id, secr)
        })
        .await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    let p = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec![],
            refus_code: Some("a_corriger_dans_la_source".into()),
            ..decision(etab, id, secr)
        })
        .await
        .unwrap();
    assert_eq!(p.entete.statut, "refusee");
    let mes = ent.mes_propositions("parent", etab, parent, lea).await.unwrap();
    assert_eq!(mes.iter().find(|m| m.entete.id == id).unwrap().entete.decision_code.as_deref(), Some("a_corriger_dans_la_source"));
    // Une proposition d'import d'une origine qui ne fait pas foi non plus ; celle de SIECLE oui.
    let e = eleve_avec(&ent, etab, "Source", (2011, 4, 2)).await;
    ent.definir_regle_auto(Role::Admin.as_str(), etab, admin, "nom_naissance", SourceProposition::Import, false).await.unwrap();
    let nouveau = nom_unique("Siecle");
    let r = ent.deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nouveau))])).await.unwrap();
    let id = r.en_attente.unwrap().entete.id;
    ent.decider_proposition(role, DecisionProposition { champs_acceptes: vec!["nom_naissance".into()], ..decision(etab, id, secr) })
        .await
        .unwrap();
    assert_eq!(nom_courant(&ent, e.id).await, nouveau);
    reset(&ent, etab).await;
}

/// Conflit : la fiche a changé (modification locale plus récente) depuis le dépôt →
/// l'acceptation est refusée, le refus reste possible.
#[tokio::test]
async fn modification_locale_plus_recente_est_un_conflit() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    let role = Role::Administratif.as_str();
    ouvrir_famille(&ent, etab, admin, "nom_naissance").await;
    let pj = piece(&ent, etab, lea).await;
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None,
                vec![("nom_naissance", json!(nom_unique("Propose")))], Some(pj)),
            parent,
        )
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    ent.corriger_identite(role, CorrectionIdentite {
        etablissement_id: etab,
        eleve_id: lea,
        version_id: None,
        modif: ModifIdentite { nom_naissance: Some(nom_unique("Local")), ..Default::default() },
        motif_code: "faute_de_frappe".into(),
        motif: None,
        piece_document_id: None,
        source: "saisie".into(),
        acteur: secr,
    })
    .await
    .unwrap();
    let r = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into()],
            nature: Some(NatureVersion::Correction),
            motif_code: Some("d_apres_piece".into()),
            ..decision(etab, id, secr)
        })
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let p = ent
        .decider_proposition(role, DecisionProposition {
            champs_acceptes: vec![],
            refus_code: Some("deja_traite".into()),
            ..decision(etab, id, secr)
        })
        .await
        .unwrap();
    assert_eq!(p.entete.statut, "refusee");
    reset(&ent, etab).await;
}

/// ELV-40 : le personnel signale une erreur vue sur le terrain — une proposition, jamais une
/// modification ; la famille et les profils sans accès à l'élève ne le peuvent pas.
#[tokio::test]
async fn signalement_du_personnel_est_une_proposition() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve_avec(&ent, etab, "Terrain", (2011, 4, 2)).await;
    let avant = nom_courant(&ent, e.id).await;
    let cmd = |v: &str| depot(etab, e.id, CibleProposition::Identite, SourceProposition::Famille, None, vec![("nom_naissance", json!(v))], None);
    let r = ent
        .deposer_signalement_personnel(Role::Cpe.as_str(), cmd(&nom_unique("Yanis")), cast.cpe.user_id)
        .await
        .unwrap();
    let p = r.en_attente.unwrap();
    assert_eq!(p.entete.source, "personnel");
    assert_eq!(nom_courant(&ent, e.id).await, avant);
    for (role, user) in [("parent", cast.parent_lea.user_id), ("eleve", cast.parent_tom.user_id)] {
        let r = ent.deposer_signalement_personnel(role, cmd("Interdit"), user).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} {r:?}");
    }
    reset(&ent, etab).await;
}

// ───────────────────────── Confidentialité et isolation ─────────────────────────

/// Les valeurs sont scellées en base, absentes des événements et du journal ; la lecture de
/// la file est journalisée.
#[tokio::test]
async fn valeurs_scellees_et_absentes_des_traces() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    ouvrir_famille(&ent, etab, admin, "nom_naissance").await;
    let pj = piece(&ent, etab, lea).await;
    let secret = nom_unique("Confidentiel");
    let r = ent
        .deposer_proposition_famille(
            "parent",
            depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None,
                vec![("nom_naissance", json!(secret))], Some(pj)),
            parent,
        )
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    let (actuel, propose): (String, String) = sqlx::query_as(
        "SELECT actuel, propose FROM ent_eleve_proposition_champ WHERE proposition_id = $1",
    )
    .bind(id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    for v in [&actuel, &propose] {
        assert!(v.starts_with("enc:v1:"), "valeur non scellée");
        assert!(!v.contains(&secret));
    }
    let (n_evt,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE payload::text ILIKE '%' || $1 || '%'",
    )
    .bind(&secret)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let (n_jrn,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif WHERE payload::text ILIKE '%' || $1 || '%'",
    )
    .bind(&secret)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n_evt, n_jrn), (0, 0), "aucune valeur dans les événements ni le journal");
    let (n_dep,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE type = 'eleve.proposition_deposee' AND payload->>'proposition_id' = $1",
    )
    .bind(id.to_string())
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n_dep, 1);
    // Lecture par le secrétariat : valeurs ouvertes, accès journalisé.
    let l = ent
        .propositions_pour_acteur(Role::Administratif.as_str(), etab, cast.administratif.user_id,
            FiltrePropositions { eleve_id: Some(lea), statut: Some("en_attente".into()), ..Default::default() })
        .await
        .unwrap();
    let p = l.iter().find(|p| p.entete.id == id).unwrap();
    assert_eq!(p.champs[0].propose, json!(secret));
    assert!(p.champs[0].actuel.is_some());
    reset(&ent, etab).await;
}

/// Isolation entre établissements : un élève d'un autre établissement n'existe pas, une
/// proposition ne se décide pas ni ne se lit depuis un autre établissement.
#[tokio::test]
async fn isolation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab_a, admin) = (cast.etablissement_id, cast.admin.user_id);
    let etab_b = Uuid::new_v4();
    let secr = cast.administratif.user_id;
    let eb = eleve_avec(&ent, etab_b, "AutreEtab", (2011, 4, 2)).await;
    // Dépôt d'un import sur un élève de B depuis le contexte A : l'élève n'existe pas.
    let r = ent
        .deposer_proposition_systeme(import_siecle(etab_a, eb.id, vec![("nom_naissance", json!(nom_unique("X")))]))
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");
    // Proposition de A : invisible et indécidable depuis B.
    ent.appliquer_profil(Role::Admin.as_str(), etab_a, admin, ProfilEtablissement::PriveSousContrat).await.unwrap();
    let ea = eleve_avec(&ent, etab_a, "ChezA", (2011, 4, 2)).await;
    let r = ent
        .deposer_proposition_systeme(depot(etab_a, ea.id, CibleProposition::Identite, SourceProposition::Import,
            Some(OrigineProposition::Reprise), vec![("nom_naissance", json!(nom_unique("Reprise")))], None))
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    let r = ent
        .decider_proposition(Role::Administratif.as_str(), DecisionProposition {
            champs_acceptes: vec!["nom_naissance".into()],
            ..decision(etab_b, id, secr)
        })
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");
    let r = ent.proposition_pour_acteur(Role::Administratif.as_str(), etab_b, secr, id).await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");
    let l = ent
        .propositions_pour_acteur(Role::Administratif.as_str(), etab_b, secr, FiltrePropositions::default())
        .await
        .unwrap();
    assert!(l.iter().all(|p| p.entete.etablissement_id == etab_b));
    assert!(l.iter().all(|p| p.entete.id != id));
    // Le paramétrage est cloisonné : B ne voit pas le profil de A.
    let pb = ent.parametrage_eleve(Role::Admin.as_str(), etab_b, admin).await.unwrap();
    assert!(pb.profil.is_none());
    reset(&ent, etab_a).await;
}

/// Une cible non prise en charge (nouvel élève, scolarisation…) est refusée explicitement,
/// et une échéance dépassée expire la proposition (jamais de durée par défaut).
#[tokio::test]
async fn cibles_non_prises_en_charge_et_expiration() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve_avec(&ent, etab, "Expire", (2011, 4, 2)).await;
    for cible in [CibleProposition::NouvelEleve, CibleProposition::Scolarisation, CibleProposition::Photo] {
        let r = ent
            .deposer_proposition_systeme(depot(etab, e.id, cible, SourceProposition::Import, Some(OrigineProposition::Siecle),
                vec![("nom_naissance", json!("X"))], None))
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{cible:?} {r:?}");
    }
    // Champ d'une autre cible, champ inconnu, doublon, lot vide.
    for champs in [
        vec![("ine", json!("ABCDEFGH123"))],
        vec![("inconnu", json!("x"))],
        vec![("nom_naissance", json!("A")), ("nom_naissance", json!("B"))],
        vec![],
    ] {
        let r = ent.deposer_proposition_systeme(import_siecle(etab, e.id, champs)).await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    }
    // Une personne connectée ne se fait pas passer pour un flux machine, et inversement.
    let r = ent
        .deposer_proposition_systeme(depot(etab, e.id, CibleProposition::Identite, SourceProposition::Famille, None,
            vec![("nom_naissance", json!(nom_unique("Fam")))], None))
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Échéance dépassée → expirée.
    let mut d = import_siecle(etab, e.id, vec![("nom_naissance", json!(nom_unique("Echu")))]);
    d.expire_le = Some(Utc::now() - Duration::hours(1));
    let r = ent.deposer_proposition_systeme(d).await.unwrap();
    let id = r.en_attente.unwrap().entete.id;
    assert!(ent.expirer_propositions_echues().await.unwrap() >= 1);
    let p = ent.proposition_pour_acteur(Role::Administratif.as_str(), etab, cast.administratif.user_id, id).await.unwrap();
    assert_eq!(p.entete.statut, "expiree");
    // Pas d'échéance : jamais expirée par défaut.
    let r = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nom_unique("Sans")))]))
        .await
        .unwrap();
    let id = r.en_attente.unwrap().entete.id;
    ent.expirer_propositions_echues().await.unwrap();
    let p = ent.proposition_pour_acteur(Role::Administratif.as_str(), etab, cast.administratif.user_id, id).await.unwrap();
    assert_eq!(p.entete.statut, "en_attente");
    reset(&ent, etab).await;
}

/// Sans règle et sans réglage (établissement qui n'a rien paramétré), les imports restent en
/// attente et la file se pagine ; un ancien lot remplacé devient obsolète.
#[tokio::test]
async fn file_des_propositions_filtres_et_remplacement_des_imports() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, secr) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve_avec(&ent, etab, "File", (2011, 4, 2)).await;
    let role = Role::Administratif.as_str();
    let r1 = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nom_unique("Un")))]))
        .await
        .unwrap();
    let id1 = r1.en_attente.unwrap().entete.id;
    let r2 = ent
        .deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(nom_unique("Deux")))]))
        .await
        .unwrap();
    let id2 = r2.en_attente.unwrap().entete.id;
    let p1 = ent.proposition_pour_acteur(role, etab, secr, id1).await.unwrap();
    assert_eq!(p1.entete.statut, "obsolete");
    assert_eq!(p1.entete.decision_code.as_deref(), Some("remplacee"));
    let f = FiltrePropositions { eleve_id: Some(e.id), statut: Some("en_attente".into()), champ: Some("nom_naissance".into()), source: Some("import".into()), ..Default::default() };
    let l = ent.propositions_pour_acteur(role, etab, secr, f).await.unwrap();
    assert_eq!(l.iter().map(|p| p.entete.id).collect::<Vec<_>>(), vec![id2]);
    let l = ent
        .propositions_pour_acteur(role, etab, secr, FiltrePropositions { limite: Some(1), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(l.len(), 1);
    // Une proposition obsolète n'est plus décidable.
    let r = ent
        .decider_proposition(role, DecisionProposition { champs_acceptes: vec!["nom_naissance".into()], ..decision(etab, id1, secr) })
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    reset(&ent, etab).await;
}

/// Une modification LOCALE plus récente (correction du secrétariat) n'est jamais écrasée seule
/// par un import : conflit visible, la proposition attend une personne.
#[tokio::test]
async fn import_ne_ecrase_pas_une_modification_locale_plus_recente() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let role = Role::Admin.as_str();
    // SIECLE fait foi mais la modification locale reste libre ; règle automatique explicite.
    ent.parametrer_champ(role, etab, admin, ModifChamp {
        champ: "nom_naissance".into(),
        source_qui_fait_foi: Some(SourceFoi::Siecle),
        ..Default::default()
    })
    .await
    .unwrap();
    ent.definir_regle_auto(role, etab, admin, "nom_naissance", SourceProposition::Import, true).await.unwrap();
    let e = eleve_avec(&ent, etab, "Conflit", (2011, 4, 2)).await;
    // Import ordinaire : appliqué.
    let n1 = nom_unique("Siecle");
    let r = ent.deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(n1))])).await.unwrap();
    assert!(r.appliquee_auto.is_some());
    // Le secrétariat corrige localement.
    let local = nom_unique("Local");
    ent.corriger_identite(Role::Administratif.as_str(), CorrectionIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        version_id: None,
        modif: ModifIdentite { nom_naissance: Some(local.clone()), ..Default::default() },
        motif_code: "faute_de_frappe".into(),
        motif: None,
        piece_document_id: None,
        source: "saisie".into(),
        acteur: secr,
    })
    .await
    .unwrap();
    // L'import suivant (valeur différente) n'écrase pas la correction : en attente, conflit.
    let n2 = nom_unique("Siecle");
    let r = ent.deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(n2))])).await.unwrap();
    assert!(r.appliquee_auto.is_none());
    assert_eq!(r.raisons_non_auto, vec![("nom_naissance".to_string(), "conflit_avec_la_fiche")]);
    assert_eq!(nom_courant(&ent, e.id).await, local);
    // Une personne tranche : acceptée, l'import redevient la dernière version (plus de conflit).
    let id = r.en_attente.unwrap().entete.id;
    ent.decider_proposition(Role::Administratif.as_str(), DecisionProposition {
        champs_acceptes: vec!["nom_naissance".into()],
        ..decision(etab, id, secr)
    })
    .await
    .unwrap();
    assert_eq!(nom_courant(&ent, e.id).await, n2);
    let n3 = nom_unique("Siecle");
    let r = ent.deposer_proposition_systeme(import_siecle(etab, e.id, vec![("nom_naissance", json!(n3))])).await.unwrap();
    assert!(r.appliquee_auto.is_some());
    reset(&ent, etab).await;
}

/// Un lot mixte dont la part automatique échoue ne perd RIEN : la part automatique rejoint les
/// champs à décider dans une seule proposition en attente, et la fiche n'a pas bougé.
#[tokio::test]
async fn echec_de_la_part_automatique_ne_perd_aucune_modification() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public).await.unwrap();
    let e = eleve_avec(&ent, etab, "Panne", (2011, 4, 2)).await;
    let avant = nom_courant(&ent, e.id).await;
    sqlx::query(
        "CREATE OR REPLACE FUNCTION ent_test_panne_identite() RETURNS TRIGGER LANGUAGE plpgsql AS $$
         BEGIN IF NEW.nom_naissance LIKE 'Panne%' AND NEW.source = 'import' THEN
             RAISE EXCEPTION 'panne simulée' USING ERRCODE = '23514'; END IF; RETURN NEW; END $$",
    )
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_panne_identite ON ent_eleve_identite").execute(ent.pool_verif()).await.unwrap();
    sqlx::query("CREATE TRIGGER ent_test_panne_identite BEFORE INSERT ON ent_eleve_identite FOR EACH ROW EXECUTE FUNCTION ent_test_panne_identite()")
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let r = ent
        .deposer_proposition_systeme(import_siecle(
            etab,
            e.id,
            vec![("nom_naissance", json!(nom_unique("Panne"))), ("prenoms", json!(["Aurélie"]))],
        ))
        .await;
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_panne_identite ON ent_eleve_identite").execute(ent.pool_verif()).await.unwrap();
    let r = r.unwrap();
    assert!(r.appliquee_auto.is_none());
    let p = r.en_attente.clone().expect("tout attend une personne");
    let mut champs: Vec<&str> = p.champs.iter().map(|c| c.champ.as_str()).collect();
    champs.sort();
    assert_eq!(champs, vec!["nom_naissance", "prenoms"]);
    assert!(r.raisons_non_auto.iter().any(|(c, m)| c == "nom_naissance" && *m == "application_impossible"), "{r:?}");
    assert_eq!(nom_courant(&ent, e.id).await, avant);
    // Aucune proposition intermédiaire laissée en base pour cet élève.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_proposition WHERE eleve_id = $1")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
    reset(&ent, etab).await;
}

/// Responsable légal = lien actif ET autorité parentale ; sans autorité, ni dépôt ni lecture.
#[tokio::test]
async fn sans_autorite_parentale_pas_de_depot() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.admin.user_id);
    let (lea, parent) = (cast.eleve_lea_id, cast.parent_lea.user_id);
    ouvrir_famille(&ent, etab, admin, "nom_usage").await;
    let cmd = || depot(etab, lea, CibleProposition::Identite, SourceProposition::Famille, None,
        vec![("nom_usage", json!(nom_unique("Usuel")))], None);
    let (initial,): (bool,) = sqlx::query_as(
        "SELECT autorite_parentale FROM ent_eleve_lien l JOIN ent_personne p ON p.id = l.personne_id
         WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND p.user_id = $3 LIMIT 1",
    )
    .bind(etab)
    .bind(lea)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(initial);
    async fn majl(ent: &PgEnt, (etab, lea, parent): (Uuid, Uuid, Uuid), v: bool) {
        sqlx::query(
            "UPDATE ent_eleve_lien SET autorite_parentale = $4 WHERE etablissement_id = $1 AND eleve_id = $2
             AND personne_id IN (SELECT id FROM ent_personne WHERE user_id = $3)",
        )
        .bind(etab)
        .bind(lea)
        .bind(parent)
        .bind(v)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    let ids = (etab, lea, parent);
    majl(&ent, ids, false).await;
    let r = ent.deposer_proposition_famille("parent", cmd(), parent).await;
    let l = ent.mes_propositions("parent", etab, parent, lea).await;
    let c = ent.champs_signalables("parent", etab, parent, lea).await;
    majl(&ent, ids, true).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert!(matches!(l, Err(EntDbError::Forbidden)));
    assert!(matches!(c, Err(EntDbError::Forbidden)));
    assert!(ent.deposer_proposition_famille("parent", cmd(), parent).await.unwrap().en_attente.is_some());
    reset(&ent, etab).await;
}

/// Verrous : confirmer ou révoquer un INE géré par SIECLE est une écriture locale refusée ;
/// contester (un doute) reste possible ; le chemin système respecte aussi le verrou.
#[tokio::test]
async fn transitions_d_identifiant_respectent_le_verrou() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin, secr) = (cast.etablissement_id, cast.admin.user_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = eleve_avec(&ent, etab, "Transition", (2011, 4, 2)).await;
    let valeur = format!("4321{}", alea(7).to_uppercase());
    let r = ent
        .ajouter_identifiant(role, crate::identifiants::AjoutIdentifiant {
            etablissement_id: etab,
            eleve_id: e.id,
            systeme: domain_eleve::SystemeIdentifiant::Ine,
            valeur,
            provenance: domain_eleve::ProvenanceIdentifiant::Saisie,
            valide_du: None,
            preuve_document_id: None,
            acteur: Some(secr),
        })
        .await
        .unwrap();
    ent.appliquer_profil(Role::Admin.as_str(), etab, admin, ProfilEtablissement::Public).await.unwrap();
    let id = r.identifiant.id;
    let r = ent.confirmer_identifiant(role, etab, e.id, id, secr, "verification").await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    let r = ent.revoquer_identifiant(role, etab, e.id, id, secr, Some("erreur")).await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    ent.contester_identifiant(role, etab, e.id, id, secr, Some("doute sur l'INE")).await.unwrap();
    let r = ent
        .ajouter_identifiant_systeme(crate::identifiants::AjoutIdentifiant {
            etablissement_id: etab,
            eleve_id: e.id,
            systeme: domain_eleve::SystemeIdentifiant::Ine,
            valeur: format!("8765{}", alea(7).to_uppercase()),
            provenance: domain_eleve::ProvenanceIdentifiant::DocumentFamille,
            valide_du: None,
            preuve_document_id: None,
            acteur: None,
        })
        .await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    reset(&ent, etab).await;
}
