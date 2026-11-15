//! Socle 2 (l'élève), lot E6 : détection des doublons (ELV-22) et paires déclarées distinctes
//! (ELV-33). Scénarios contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`,
//! ils sont ignorés. Chaque scénario travaille dans son propre établissement (jetable).

use super::*;
use crate::identite::CorrectionIdentite;
use chrono::NaiveDate;
use domain_eleve::{CreateEleve, ModifIdentite};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { }).await;
    Some((ent, cast))
}

fn jour(y: i32, m: u32, d: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(y, m, d)
}

async fn eleve(
    ent: &PgEnt,
    etab: Uuid,
    nom: &str,
    prenom: &str,
    naissance: Option<NaiveDate>,
    ine: Option<&str>,
) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: prenom.into(),
        ine: ine.map(String::from),
        sexe: Some("F".into()),
        date_naissance: naissance,
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

const ADM: &str = "administratif";

async fn liste(ent: &PgEnt, cast: &LaboCast, role: Role, etab: Uuid) -> Result<Vec<crate::eleve_doublons::DoublonVue>, EntDbError> {
    let user = match role {
        Role::Administratif => cast.administratif.user_id,
        Role::Direction => cast.direction.user_id,
        Role::Cpe => cast.cpe.user_id,
        _ => unreachable!(),
    };
    ent.doublons_pour_acteur(role.as_str(), etab, user, 200).await
}

fn paire(l: &[crate::eleve_doublons::DoublonVue], x: Uuid, y: Uuid) -> Option<&crate::eleve_doublons::DoublonVue> {
    l.iter().find(|d| {
        (d.a.eleve.id == x && d.b.eleve.id == y) || (d.a.eleve.id == y && d.b.eleve.id == x)
    })
}

/// Les vraies paires sont proposées avec leur score, les jumeaux et les homonymes nés un autre
/// jour ne le sont pas, un second passage ne crée rien de nouveau.
#[tokio::test]
async fn detection_propose_les_doublons_et_epargne_jumeaux_et_homonymes() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = Uuid::new_v4();
    let a = eleve(&ent, etab, "Dupont", "Jean", jour(2012, 3, 4), None).await;
    let b = eleve(&ent, etab, "DUPONT", "jean", jour(2012, 3, 4), None).await;
    let c = eleve(&ent, etab, "Benali", "Yanis", jour(2011, 5, 6), None).await;
    let d = eleve(&ent, etab, "Benali", "Yannis", jour(2011, 5, 6), None).await;
    let lea = eleve(&ent, etab, "Martin", "Léa", jour(2012, 5, 6), None).await;
    let leo = eleve(&ent, etab, "Martin", "Léo", jour(2012, 5, 6), None).await;
    let e = eleve(&ent, etab, "Dupont", "Jean", jour(2013, 9, 10), None).await; // homonyme d'un autre jour
    let f = eleve(&ent, etab, "Durand", "Paul", jour(2010, 1, 1), Some("IN123456789")).await;
    let g = eleve(&ent, etab, "Moreau", "Luc", jour(2009, 2, 2), Some("IN123456789")).await;

    // Rien n'a été détecté : la liste est vide, le droit est contrôlé.
    assert!(liste(&ent, &cast, Role::Administratif, etab).await.unwrap().is_empty());
    assert!(matches!(liste(&ent, &cast, Role::Cpe, etab).await, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.detecter_doublons_pour_acteur(Role::Cpe.as_str(), etab, cast.cpe.user_id).await,
        Err(EntDbError::Forbidden)
    ));

    let bilan = ent.detecter_doublons_pour_acteur(ADM, etab, cast.administratif.user_id).await.unwrap();
    assert_eq!(bilan.paires, 3, "{bilan:?}");
    assert_eq!(bilan.nouvelles, 3);
    let l = liste(&ent, &cast, Role::Administratif, etab).await.unwrap();
    assert_eq!(l.len(), 3);
    assert_eq!(paire(&l, a.id, b.id).unwrap().score, 100);
    assert_eq!(paire(&l, c.id, d.id).unwrap().score, 85);
    let ine = paire(&l, f.id, g.id).unwrap();
    assert_eq!(ine.score, 92);
    assert!(ine.raisons.iter().any(|r| r == "ine_identique"));
    // Du plus sûr au moins sûr.
    assert!(l.windows(2).all(|w| w[0].score >= w[1].score));
    // Ni jumeaux ni homonymes.
    assert!(paire(&l, lea.id, leo.id).is_none(), "jumeaux");
    assert!(paire(&l, a.id, e.id).is_none() && paire(&l, b.id, e.id).is_none(), "homonyme né un autre jour");
    // Les élèves de la liste sont minimisés selon l'acteur (le secrétariat voit l'état civil).
    assert!(l[0].a.eleve.date_naissance.is_some());

    // Idempotent : un second passage ne crée rien de nouveau.
    let bilan2 = ent.detecter_doublons(etab).await.unwrap();
    assert_eq!((bilan2.paires, bilan2.nouvelles, bilan2.devenues_obsoletes), (3, 0, 0));
    assert_eq!(ent.nombre_doublons_a_examiner(etab).await.unwrap(), 3);
}

/// ELV-33 : « ce sont bien deux élèves » ; la paire ne revient que si une identité change.
#[tokio::test]
async fn non_doublon_memorise_jusqu_a_un_changement_d_identite() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, secr) = (Uuid::new_v4(), cast.administratif.user_id);
    let a = eleve(&ent, etab, "Lambert", "Zoé", jour(2012, 3, 4), None).await;
    let b = eleve(&ent, etab, "LAMBERT", "zoe", jour(2012, 3, 4), None).await;
    ent.detecter_doublons(etab).await.unwrap();
    assert!(paire(&liste(&ent, &cast, Role::Administratif, etab).await.unwrap(), a.id, b.id).is_some());

    // Motif inconnu, même fiche deux fois, droit manquant : refusés.
    assert!(ent.declarer_non_doublon(ADM, etab, secr, a.id, b.id, "parce que").await.is_err());
    assert!(ent.declarer_non_doublon(ADM, etab, secr, a.id, a.id, "jumeaux").await.is_err());
    assert!(matches!(
        ent.declarer_non_doublon(Role::Cpe.as_str(), etab, cast.cpe.user_id, a.id, b.id, "jumeaux").await,
        Err(EntDbError::Forbidden)
    ));
    ent.declarer_non_doublon(ADM, etab, secr, b.id, a.id, "jumeaux").await.unwrap();
    // Déclaré : la paire disparaît de la liste et ne revient pas au passage suivant.
    assert!(liste(&ent, &cast, Role::Administratif, etab).await.unwrap().is_empty());
    ent.detecter_doublons(etab).await.unwrap();
    assert!(liste(&ent, &cast, Role::Administratif, etab).await.unwrap().is_empty());
    // L'événement ne porte aucune valeur d'identité.
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'eleve.non_doublon_declare'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!payload.to_string().to_lowercase().contains("lambert"), "{payload}");
    // L'identité de l'une des deux change (correction du nom) : la paire revient.
    ent.corriger_identite(
        ADM,
        CorrectionIdentite {
            etablissement_id: etab,
            eleve_id: b.id,
            version_id: None,
            modif: ModifIdentite { nom_naissance: Some("Lambeert".into()), ..Default::default() },
            motif_code: "faute_de_frappe".into(),
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: secr,
        },
    )
    .await
    .unwrap();
    ent.detecter_doublons(etab).await.unwrap();
    let l = liste(&ent, &cast, Role::Administratif, etab).await.unwrap();
    assert!(paire(&l, a.id, b.id).is_some(), "l'identité a changé : la paire revient");
}

/// Une paire qui touche un élève en confidentialité renforcée n'existe que pour qui détient G6.
#[tokio::test]
async fn paire_protegee_invisible_sans_g6() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = Uuid::new_v4();
    let a = eleve(&ent, etab, "Garnier", "Inès", jour(2012, 3, 4), None).await;
    let b = eleve(&ent, etab, "Garnier", "Ines", jour(2012, 3, 4), None).await;
    ent.detecter_doublons(etab).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du)
         VALUES (gen_random_uuid(), $1, $2, 'enc:v1:test', ARRAY['hors_annuaire'], ent_aujourdhui() - 1)",
    )
    .bind(etab)
    .bind(b.id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    assert!(liste(&ent, &cast, Role::Administratif, etab).await.unwrap().is_empty(), "invisible pour le secrétariat");
    let l = liste(&ent, &cast, Role::Direction, etab).await.unwrap();
    assert!(paire(&l, a.id, b.id).is_some(), "la direction (G6) la voit");
    // Déclarer la paire distincte exige aussi de voir l'élève protégé.
    assert!(matches!(
        ent.declarer_non_doublon(ADM, etab, cast.administratif.user_id, a.id, b.id, "jumeaux").await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

/// Cloisonnement : les doublons d'un établissement n'apparaissent pas dans un autre, et la
/// tâche quotidienne ne passe qu'une fois par jour et par établissement.
#[tokio::test]
async fn cloisonnement_et_tache_quotidienne() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, autre) = (Uuid::new_v4(), Uuid::new_v4());
    // La tâche parcourt les établissements connus de la plateforme (racine `ent_etablissement`).
    for (id, slug) in [(etab, "doublons-a"), (autre, "doublons-b")] {
        sqlx::query("INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, $2 || '-' || substr($1::text, 1, 8), 'Test doublons')")
            .bind(id)
            .bind(slug)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    let a = eleve(&ent, etab, "Perrin", "Mila", jour(2012, 3, 4), None).await;
    let b = eleve(&ent, etab, "Perrin", "Mila", jour(2012, 3, 4), None).await;
    // Même identité dans un autre établissement : jamais rapprochée (D-013).
    let c = eleve(&ent, autre, "Perrin", "Mila", jour(2012, 3, 4), Some("XX11223344")).await;
    let n = ent.tache_doublons_eleves().await.unwrap();
    assert!(n >= 2, "les deux établissements ont été traités ({n})");
    let l = liste(&ent, &cast, Role::Administratif, etab).await.unwrap();
    assert!(paire(&l, a.id, b.id).is_some());
    assert!(l.iter().all(|d| d.a.eleve.id != c.id && d.b.eleve.id != c.id));
    assert!(liste(&ent, &cast, Role::Administratif, autre).await.unwrap().is_empty());
    // Un second appel le même jour ne repasse pas sur ces établissements.
    let (avant,): (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT passe_le FROM ent_eleve_doublon_passage WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    ent.tache_doublons_eleves().await.unwrap();
    let (apres,): (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT passe_le FROM ent_eleve_doublon_passage WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(avant, apres, "un seul passage par jour");
}
