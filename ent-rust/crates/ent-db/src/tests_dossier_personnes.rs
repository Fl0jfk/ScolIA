//! Socle 2, lot S2-1 : dossier élève et personnes contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Distribution du labo : `parent_lea` est responsable de Harry (`eleve_lea_id`,
//! tous droits), `parent_tom` de Draco (`eleve_tom_id`).

use super::*;
use crate::outbox::ConfigDistribution;
use domain_eleve::{today_utc, DomainError};
use ent_auth::Role;
use ent_events::types;
use std::time::Duration;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn personne(nom: &str, prenom: &str, email: Option<&str>, tel: Option<&str>) -> PersonneSaisie {
    PersonneSaisie {
        nom: nom.into(),
        prenom: Some(prenom.into()),
        email: email.map(Into::into),
        telephone: tel.map(Into::into),
        ..Default::default()
    }
}

fn lien(personne_id: Uuid, nature: &str, foyer: Option<Uuid>) -> LienSaisie {
    LienSaisie {
        personne_id: Some(personne_id),
        foyer_id: foyer,
        lien: nature.into(),
        autorite_parentale: true,
        garde: None,
        date_debut: None,
        date_fin: None,
        peut_voir_notes: true,
        peut_voir_vs: true,
        peut_justifier_absence: true,
        peut_autoriser_sorties: false,
    }
}

async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).bind(id).fetch_one(ent.pool()).await.unwrap();
    n
}

fn dir(cast: &LaboCast) -> (Uuid, &'static str) {
    (cast.direction.user_id, Role::Direction.as_str())
}

/// Reprise : chaque lien existant porte une personne ; une personne par compte et
/// par établissement ; rejouer les migrations ne crée rien ; le code existant
/// (`link_parent`) crée la personne au besoin.
#[tokio::test]
async fn reprise_des_liens_idempotente() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let sans_personne = "SELECT count(*) FROM ent_eleve_lien WHERE etablissement_id = $1 AND personne_id IS NULL";
    assert_eq!(compter(&ent, sans_personne, etab).await, 0);
    let personnes = "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1";
    let avant = compter(&ent, personnes, etab).await;
    assert!(avant > 0);
    let doublons = "SELECT count(*) FROM (SELECT user_id FROM ent_personne WHERE etablissement_id = $1
                     AND user_id IS NOT NULL GROUP BY user_id HAVING count(*) > 1) d";
    assert_eq!(compter(&ent, doublons, etab).await, 0);
    // Cohérence lien.user_id = personne.user_id partout.
    let incoherents = "SELECT count(*) FROM ent_eleve_lien l JOIN ent_personne p ON p.id = l.personne_id
                        WHERE l.etablissement_id = $1 AND l.user_id IS DISTINCT FROM p.user_id";
    assert_eq!(compter(&ent, incoherents, etab).await, 0);

    // Simule des liens « d'avant » (sans personne) : la reprise les rattache.
    sqlx::query("ALTER TABLE ent_eleve_lien DISABLE TRIGGER ent_eleve_lien_personne_sync_trg")
        .execute(ent.pool())
        .await
        .unwrap();
    // NOT NULL est réappliqué par la migration rejouée.
    sqlx::query("ALTER TABLE ent_eleve_lien ALTER COLUMN personne_id DROP NOT NULL")
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve_lien SET personne_id = NULL WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM ent_personne WHERE etablissement_id = $1 AND source = 'reprise'
                  AND NOT EXISTS (SELECT 1 FROM ent_eleve_lien l WHERE l.personne_id = ent_personne.id)")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_eleve_lien ENABLE TRIGGER ent_eleve_lien_personne_sync_trg")
        .execute(ent.pool())
        .await
        .unwrap();
    ent.migrate().await.unwrap();
    ent.migrate().await.unwrap();
    assert_eq!(compter(&ent, sans_personne, etab).await, 0);
    assert_eq!(compter(&ent, doublons, etab).await, 0);
    assert_eq!(compter(&ent, incoherents, etab).await, 0);
    let apres = compter(&ent, personnes, etab).await;
    ent.migrate().await.unwrap();
    assert_eq!(compter(&ent, personnes, etab).await, apres, "rejouer ne crée rien");

    // Code existant : un lien créé par compte reçoit sa personne automatiquement.
    let nouveau = ent
        .register_user(
            &format!("nouveau-{}@ent.lab", Uuid::new_v4().simple()),
            "DevLocalPass1!",
            Some("Nouveau Parent"),
            etab,
            Role::Parent.as_str(),
        )
        .await
        .unwrap();
    ent.link_parent(CreateLien {
        etablissement_id: etab,
        eleve_id: cast.eleve_tom_id,
        user_id: nouveau.id,
        foyer_id: Some(cast.foyer_tom_id),
        lien: "tuteur".into(),
        peut_voir_notes: true,
        peut_voir_vs: false,
        peut_justifier_absence: false,
        peut_autoriser_sorties: false,
        est_payeur: false,
        part_paiement: None,
    })
    .await
    .unwrap();
    let (src,): (String,) = sqlx::query_as(
        "SELECT p.source FROM ent_personne p WHERE p.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(etab)
    .bind(nouveau.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(src, "compte");
    let resp = ent
        .responsables_eleve_for_actor(etab, cast.direction.user_id, Role::Direction.as_str(), cast.eleve_tom_id)
        .await
        .unwrap();
    assert!(resp.iter().any(|r| r.personne.user_id == Some(nouveau.id) && r.personne.nom == "Nouveau Parent"));
}

/// Famille avec un parent sans compte ni e-mail ; parents séparés (une fiche et
/// un foyer par parent) ; minimisation entre responsables.
#[tokio::test]
async fn parent_sans_compte_et_parents_separes() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let harry = cast.eleve_lea_id;

    let pere = ent
        .creer_personne_for_actor(etab, d, r, &personne("Potter", "James", None, Some("06 11 22 33 44")))
        .await
        .unwrap();
    assert_eq!(pere.user_id, None);
    assert_eq!(pere.email, None);
    // Foyer distinct : parents séparés.
    let foyer_pere = ent.create_foyer(etab, Some("Foyer du père")).await.unwrap();
    let l = ent
        .lier_personne_for_actor(etab, d, r, harry, &LienSaisie {
            garde: Some("alternee".into()),
            ..lien(pere.id, "père", Some(foyer_pere))
        })
        .await
        .unwrap()
        .actif();
    assert!(l.actif && l.autorite_parentale);
    assert_eq!(l.garde.as_deref(), Some("alternee"));
    // Doublon de lien refusé.
    let e = ent.lier_personne_for_actor(etab, d, r, harry, &lien(pere.id, "père", None)).await;
    assert!(matches!(e, Err(EntDbError::ConflitDossier(_))), "{e:?}");

    // Fiche (direction) : les deux responsables, alerte « un e-mail par parent ».
    let fiche = ent.fiche_eleve_for_actor(etab, d, r, harry).await.unwrap().unwrap();
    assert!(fiche.responsables.len() >= 2);
    assert!(fiche.responsables.iter().any(|x| x.personne.id == pere.id && x.personne.telephone.is_some()));
    assert!(fiche.alertes.iter().any(|a| a.contains("Parents séparés")), "{:?}", fiche.alertes);
    assert!(fiche.actions.iter().any(|a| a.code == "liens.suspendre"));

    // L'e-mail renseigné, l'alerte disparaît.
    ent.modifier_personne_for_actor(etab, d, r, pere.id, &personne("Potter", "James", Some("james@exemple.fr"), Some("06 11 22 33 44")))
        .await
        .unwrap();
    let fiche = ent.fiche_eleve_for_actor(etab, d, r, harry).await.unwrap().unwrap();
    assert!(!fiche.alertes.iter().any(|a| a.contains("Parents séparés")), "{:?}", fiche.alertes);

    // Vu par l'autre parent (compte) : le père sans ses coordonnées, soi avec.
    let vu = ent
        .responsables_eleve_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), harry)
        .await
        .unwrap();
    let p = vu.iter().find(|x| x.personne.id == pere.id).unwrap();
    assert_eq!(p.personne.email, None);
    assert_eq!(p.personne.telephone, None);
    let moi = vu.iter().find(|x| x.personne.user_id == Some(cast.parent_lea.user_id)).unwrap();
    assert!(moi.personne.email.is_some());
    // Un parent sans compte n'ouvre aucun accès : aucun compte n'est rattaché.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_lien_actif WHERE personne_id = $1 AND user_id IS NOT NULL")
        .bind(pere.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);

    // Contact d'urgence : sans téléphone refusé ; proposé par un parent → à valider.
    let sans_tel = ContactSaisie {
        personne_id: None,
        nouvelle_personne: Some(personne("Dursley", "Pétunia", None, None)),
        relation: "tante".into(),
        ordre: 1,
        peut_recuperer: true,
        commentaire: None,
    };
    assert!(matches!(
        ent.ajouter_contact_for_actor(etab, d, r, harry, &sans_tel).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let propose = ent
        .ajouter_contact_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), harry, &ContactSaisie {
            nouvelle_personne: Some(personne("Dursley", "Pétunia", None, Some("01 02 03 04 05"))),
            ..sans_tel.clone()
        })
        .await
        .unwrap();
    assert_eq!(propose.statut, "propose");
    // Un parent ne choisit pas une personne existante de l'établissement.
    assert!(matches!(
        ent.ajouter_contact_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), harry, &ContactSaisie {
            personne_id: Some(pere.id),
            nouvelle_personne: None,
            ..sans_tel.clone()
        })
        .await,
        Err(EntDbError::Forbidden)
    ));
    ent.valider_contact_for_actor(etab, d, r, propose.id).await.unwrap();
    let contacts = ent.contacts_eleve_for_actor(etab, d, r, harry).await.unwrap();
    assert!(contacts.iter().any(|c| c.id == propose.id && c.statut == "valide"));
    // Le parent de Draco ne voit pas les contacts de Harry.
    assert!(matches!(
        ent.contacts_eleve_for_actor(etab, cast.parent_tom.user_id, Role::Parent.as_str(), harry).await,
        Err(EntDbError::Forbidden)
    ));

    // Payeurs : un payeur sans foyer refusé ; parts ≠ 100 refusées ; 60/40 accepté.
    let resp = ent.responsables_eleve_for_actor(etab, d, r, harry).await.unwrap();
    let mere = resp.iter().find(|x| x.personne.user_id == Some(cast.parent_lea.user_id)).unwrap();
    let e = ent.definir_payeurs_for_actor(etab, d, r, harry, &[(mere.lien_id, 60), (l.lien_id, 30)]).await;
    assert!(matches!(e, Err(EntDbError::Domain(DomainError::PartsPaiementInvalides))), "{e:?}");
    let ok = ent
        .definir_payeurs_for_actor(etab, cast.compta.user_id, Role::Compta.as_str(), harry, &[(mere.lien_id, 60), (l.lien_id, 40)])
        .await
        .unwrap();
    assert_eq!(ok.iter().filter(|x| x.est_payeur).count(), 2);
    assert!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE event_id IN (SELECT id FROM ent_metier_event WHERE type = 'foyer.payeur_defini') AND etablissement_id = $1", etab).await >= 1);
}

/// Élève majeur : fiche personne de l'élève lui-même, payeur ; refus si mineur.
#[tokio::test]
async fn eleve_majeur_payeur_de_sa_scolarite() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let draco = cast.eleve_tom_id;
    sqlx::query("UPDATE ent_eleve SET date_naissance = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(draco)
        .bind(today_utc() - chrono::Duration::days(365 * 12))
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(matches!(
        ent.personne_eleve_majeur_for_actor(etab, d, r, draco).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    sqlx::query("UPDATE ent_eleve SET date_naissance = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(draco)
        .bind(today_utc() - chrono::Duration::days(365 * 19))
        .execute(ent.pool())
        .await
        .unwrap();
    let soi = ent.personne_eleve_majeur_for_actor(etab, d, r, draco).await.unwrap();
    let encore = ent.personne_eleve_majeur_for_actor(etab, d, r, draco).await.unwrap();
    assert_eq!(soi.id, encore.id, "une seule fiche par élève");
    assert_eq!(soi.user_id, None);
    // Lien « parent » refusé pour la fiche d'un élève ; lien élève majeur sans droits famille.
    assert!(ent.lier_personne_for_actor(etab, d, r, draco, &lien(soi.id, "père", None)).await.is_err());
    let l = ent
        .lier_personne_for_actor(etab, d, r, draco, &lien(soi.id, "eleve_majeur", Some(cast.foyer_tom_id)))
        .await
        .unwrap()
        .actif();
    assert!(!l.autorite_parentale && !l.peut_voir_notes && !l.peut_voir_vs);
    let ok = ent.definir_payeurs_for_actor(etab, d, r, draco, &[(l.lien_id, 100)]).await.unwrap();
    assert!(ok.iter().any(|x| x.lien_id == l.lien_id && x.est_payeur && x.part_paiement == Some(100)));
    let fiche = ent.fiche_eleve_for_actor(etab, d, r, draco).await.unwrap().unwrap();
    assert!(fiche.majeur);
    // Les parents gardent leur accès par défaut (Q10 non tranchée).
    assert!(ent
        .get_eleve_for_actor(etab, cast.parent_tom.user_id, Role::Parent.as_str(), draco)
        .await
        .unwrap()
        .is_some());
}

/// Suspension d'un lien (décision de justice) → accès coupé PARTOUT, puis rétabli.
/// Même effet pour un lien hors de ses dates.
#[tokio::test]
async fn suspension_coupe_l_acces_partout() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let harry = cast.eleve_lea_id;
    let parent = cast.parent_lea.user_id;
    let pr = Role::Parent.as_str();

    async fn acces(ent: &PgEnt, etab: Uuid, parent: Uuid, eleve: Uuid) -> Vec<bool> {
        let pr = Role::Parent.as_str();
        vec![
            ent.get_eleve_for_actor(etab, parent, pr, eleve).await.unwrap().is_some(),
            ent.lien_parent_eleve(etab, parent, eleve).await.unwrap().is_some(),
            ent.eleves_lies(etab, parent).await.unwrap().contains(&eleve),
            ent.mes_enfants_for_parent(etab, parent, pr)
                .await
                .unwrap()
                .iter()
                .any(|e| e.eleve.id == eleve),
            ent.actor_can_read_notes(etab, parent, pr, eleve).await.unwrap_or(false),
            ent.actor_can_read_absences(etab, parent, pr, eleve).await.unwrap_or(false),
            ent.actor_can_access_msg_fil(etab, parent, pr, eleve).await.unwrap_or(false),
            ent.list_journal_eleve_for_actor(etab, parent, pr, eleve, 10).await.is_ok(),
            ent.fiche_eleve_for_actor(etab, parent, pr, eleve).await.unwrap().is_some(),
            ent.responsables_eleve_for_actor(etab, parent, pr, eleve).await.is_ok(),
            ent.chronologie_eleve_for_actor(etab, parent, pr, eleve, 10).await.is_ok(),
            {
                let (n,): (i64,) = sqlx::query_as(
                    "SELECT count(*) FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
                )
                .bind(etab)
                .bind(eleve)
                .bind(parent)
                .fetch_one(ent.pool())
                .await
                .unwrap();
                n == 1
            },
        ]
    }

    assert!(acces(&ent, etab, parent, harry).await.iter().all(|x| *x), "accès initial complet");
    let resp = ent.responsables_eleve_for_actor(etab, d, r, harry).await.unwrap();
    let lien_mere = resp.iter().find(|x| x.personne.user_id == Some(parent)).unwrap().lien_id;

    // Sans le droit : administratif refusé (direction seulement), motif obligatoire.
    assert!(matches!(
        ent.suspendre_lien_for_actor(etab, cast.administratif.user_id, Role::Administratif.as_str(), lien_mere, "Jugement", None).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.suspendre_lien_for_actor(etab, d, r, lien_mere, " ", None).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let s = ent
        .suspendre_lien_for_actor(etab, d, r, lien_mere, "Ordonnance du juge aux affaires familiales", Some("JAF 2026/123"))
        .await
        .unwrap();
    assert!(!s.actif && s.suspendu_le.is_some());
    let apres = acces(&ent, etab, parent, harry).await;
    assert!(apres.iter().all(|x| !*x), "aucun accès après suspension : {apres:?}");
    // Le motif n'est ni dans l'événement ni dans une entrée visible famille.
    let fuite = compter(&ent, "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND payload::text ILIKE '%juge%'", etab).await
        + compter(&ent, "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND payload::text ILIKE '%juge%'", etab).await;
    assert_eq!(fuite, 0);
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event e JOIN ent_outbox o ON o.event_id = e.id WHERE e.etablissement_id = $1 AND e.type = $2")
        .bind(etab)
        .bind(types::LIEN_SUSPENDU)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 1, "fait publié dans l'outbox, en transaction");

    // Notifications : le consommateur relit les liens actifs → rien pour la mère.
    let d0 = ent.distributeur(ConfigDistribution {
        taille_lot: 50,
        max_tentatives: 3,
        delai_base: Duration::ZERO,
        delai_max: Duration::ZERO,
        intervalle: Duration::from_millis(20),
        etablissement: Some(etab),
    });
    {
        use crate::outbox::Publier;
        let mut tx = ent.pool().begin().await.unwrap();
        tx.publier(ent_events::MetierEventDraft {
            etablissement_id: etab,
            type_name: types::RETARD_DECLARE.into(),
            aggregate: ent_events::AggregateKind::Retard,
            aggregate_id: Uuid::new_v4(),
            eleve_id: Some(harry),
            payload: serde_json::json!({ "jour": today_utc().to_string() }),
            actor_user_id: None,
        })
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    d0.distribuer_tout().await.unwrap();
    assert!(ent
        .mes_notifications(etab, parent, 50)
        .await
        .unwrap()
        .iter()
        .all(|n| n.kind != "retard"));

    // Suspension en double refusée ; rétablissement → accès rendu.
    assert!(matches!(
        ent.suspendre_lien_for_actor(etab, d, r, lien_mere, "encore", None).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.retablir_lien_for_actor(etab, d, r, lien_mere).await.unwrap();
    assert!(acces(&ent, etab, parent, harry).await.iter().all(|x| *x));

    // Lien terminé (date de fin passée) : même coupure ; futur : pas encore actif.
    let mut saisie = lien(Uuid::nil(), "mère", Some(cast.foyer_lea_id));
    saisie.personne_id = None;
    saisie.date_fin = Some(today_utc() - chrono::Duration::days(1));
    saisie.date_debut = Some(today_utc() - chrono::Duration::days(30));
    ent.modifier_lien_for_actor(etab, d, r, lien_mere, &saisie).await.unwrap();
    assert!(acces(&ent, etab, parent, harry).await.iter().all(|x| !*x));
    saisie.date_debut = Some(today_utc() + chrono::Duration::days(1));
    saisie.date_fin = None;
    ent.modifier_lien_for_actor(etab, d, r, lien_mere, &saisie).await.unwrap();
    assert!(!ent.lien_parent_eleve(etab, parent, harry).await.unwrap().is_some());
    saisie.date_debut = None;
    ent.modifier_lien_for_actor(etab, d, r, lien_mere, &saisie).await.unwrap();
    assert!(acces(&ent, etab, parent, harry).await.iter().all(|x| *x));
    let _ = pr;
}

/// Matrice droits × périmètres des actions du dossier (modèles par défaut).
#[tokio::test]
async fn matrice_droits_des_actions_du_dossier() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let acteurs: Vec<(&str, Uuid, Role)> = vec![
        ("admin", cast.admin.user_id, Role::Admin),
        ("direction", cast.direction.user_id, Role::Direction),
        ("administratif", cast.administratif.user_id, Role::Administratif),
        ("cpe", cast.cpe.user_id, Role::Cpe),
        ("compta", cast.compta.user_id, Role::Compta),
        ("prof", cast.prof.user_id, Role::Prof),
        ("parent", cast.parent_lea.user_id, Role::Parent),
        ("parent_autre", cast.parent_tom.user_id, Role::Parent),
    ];
    // Un professeur lit les responsables des élèves qu'il suit (portée « mes classes »).
    let prof_suit = ent.prof_suit_eleve(etab, cast.prof.user_id, harry).await.unwrap();
    // (créer personne, payeurs, suspendre, fusions, lire responsables de Harry)
    let attendu = |nom: &str| -> [bool; 5] {
        match nom {
            "admin" | "direction" => [true, true, true, true, true],
            "administratif" => [true, true, false, false, true],
            "cpe" => [false, false, false, false, true],
            "compta" => [false, true, false, false, true],
            "prof" => [false, false, false, false, prof_suit],
            "parent" => [false, false, false, false, true],
            "parent_autre" => [false, false, false, false, false],
            _ => unreachable!(),
        }
    };
    let resp = ent
        .responsables_eleve_for_actor(etab, cast.direction.user_id, Role::Direction.as_str(), harry)
        .await
        .unwrap();
    let lien_mere = resp.iter().find(|x| x.personne.user_id == Some(cast.parent_lea.user_id)).unwrap();
    for (nom, uid, role) in acteurs {
        let r = role.as_str();
        let creer = ent
            .creer_personne_for_actor(etab, uid, r, &personne("Matrice", nom, None, None))
            .await;
        let payeurs = ent.definir_payeurs_for_actor(etab, uid, r, harry, &[(lien_mere.lien_id, 100)]).await;
        // Motif invalide : passe le contrôle de droit puis échoue sans rien suspendre.
        let suspendre = ent.suspendre_lien_for_actor(etab, uid, r, lien_mere.lien_id, "x", None).await;
        let fusions = ent.list_fusions_for_actor(etab, uid, r).await;
        let lire = ent.responsables_eleve_for_actor(etab, uid, r, harry).await;
        let obtenu = [
            creer.is_ok(),
            payeurs.is_ok(),
            matches!(suspendre, Err(EntDbError::SaisieInvalide(_))),
            fusions.is_ok(),
            lire.is_ok(),
        ];
        assert_eq!(obtenu, attendu(nom), "{nom}");
        for (ok, e) in [
            (obtenu[0], creer.err()),
            (obtenu[1], payeurs.err()),
            (obtenu[3], fusions.err()),
            (obtenu[4], lire.err()),
        ] {
            if !ok {
                assert!(matches!(e, Some(EntDbError::Forbidden)), "{nom}: {e:?}");
            }
        }
    }
    // Un rôle paramétré peut ouvrir l'action (D-017) : CPE + « personnes.gerer ».
    let d = (cast.direction.user_id, Role::Direction.as_str());
    let role = ent
        .creer_role_for_actor(etab, d.0, d.1, RoleSaisie {
            nom: format!("Accueil {}", &Uuid::new_v4().simple().to_string()[..6]),
            description: None,
            modele: None,
            droits: vec![Droit { action: ent_auth::Action::PersonnesGerer, portee: ent_auth::Portee::Etablissement, cibles: vec![] }],
        })
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, d.0, d.1, role.id, cast.cpe.user_id, None).await.unwrap();
    assert!(ent
        .creer_personne_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), &personne("Accueil", "Ok", None, None))
        .await
        .is_ok());
}

/// Isolation : rien d'un établissement n'est lisible ni modifiable depuis un autre.
#[tokio::test]
async fn isolation_entre_etablissements() {
    let Some((ent, a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = ent.seed_labo_cast().await.unwrap();
    let (da, ra) = dir(&a);
    let (db_, rb) = dir(&b);
    let p = ent
        .creer_personne_for_actor(a.etablissement_id, da, ra, &personne("Secret", "A", Some("a@exemple.fr"), None))
        .await
        .unwrap();
    let la = ent
        .lier_personne_for_actor(a.etablissement_id, da, ra, a.eleve_lea_id, &lien(p.id, "tuteur", None))
        .await
        .unwrap()
        .actif();
    let nf = |e: &Result<_, EntDbError>| matches!(e, Err(EntDbError::Domain(DomainError::NotFound)));
    assert!(nf(&ent.get_personne_for_actor(b.etablissement_id, db_, rb, p.id).await.map(|_| ())));
    assert!(nf(&ent
        .lier_personne_for_actor(b.etablissement_id, db_, rb, b.eleve_lea_id, &lien(p.id, "tuteur", None))
        .await
        .map(|_| ())));
    assert!(nf(&ent
        .lier_personne_for_actor(b.etablissement_id, db_, rb, a.eleve_lea_id, &lien(p.id, "tuteur", None))
        .await
        .map(|_| ())));
    assert!(nf(&ent.suspendre_lien_for_actor(b.etablissement_id, db_, rb, la.lien_id, "Jugement", None).await.map(|_| ())));
    assert!(nf(&ent.modifier_personne_for_actor(b.etablissement_id, db_, rb, p.id, &personne("X", "Y", None, None)).await.map(|_| ())));
    assert!(ent.fiche_eleve_for_actor(b.etablissement_id, db_, rb, a.eleve_lea_id).await.unwrap().is_none());
    let trouve = ent.rechercher_personnes_for_actor(b.etablissement_id, db_, rb, "Secret").await.unwrap();
    assert!(trouve.is_empty());
    // Compte d'un autre établissement : non rattachable (indiscernable d'inconnu).
    assert!(nf(&ent
        .rattacher_compte_for_actor(a.etablissement_id, da, ra, p.id, b.parent_lea.user_id)
        .await
        .map(|_| ())));
    // Le déclencheur refuse une personne d'un autre établissement même en SQL direct.
    let e = sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien) VALUES ($1,$2,$3,$4,'x')")
        .bind(Uuid::new_v4())
        .bind(b.etablissement_id)
        .bind(b.eleve_lea_id)
        .bind(p.id)
        .execute(ent.pool())
        .await;
    assert!(e.is_err());
}

type Etat = (
    Vec<(Uuid, Uuid, Uuid, Option<Uuid>, String)>,
    Vec<(Uuid, Uuid)>,
    Vec<(Uuid, Option<Uuid>, Option<Uuid>)>,
);

async fn etat(ent: &PgEnt, etab: Uuid) -> Etat {
    let liens = sqlx::query_as(
        "SELECT id, eleve_id, personne_id, user_id, lien FROM ent_eleve_lien WHERE etablissement_id = $1 ORDER BY id",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    let contacts = sqlx::query_as(
        "SELECT id, personne_id FROM ent_eleve_contact_urgence WHERE etablissement_id = $1 ORDER BY id",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    let personnes = sqlx::query_as(
        "SELECT id, user_id, fusionnee_dans FROM ent_personne WHERE etablissement_id = $1 ORDER BY id",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    (liens, contacts, personnes)
}

/// Fusion de doublons réversible : la défusion restaure exactement l'état d'avant.
#[tokio::test]
async fn fusion_et_defusion_de_personnes() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let resp = ent.responsables_eleve_for_actor(etab, d, r, harry).await.unwrap();
    let mere = resp.iter().find(|x| x.personne.user_id == Some(cast.parent_lea.user_id)).unwrap().personne.clone();

    // Doublon saisi à la main : lié à Harry (doublon du lien existant) et à Draco,
    // contact d'urgence de Draco.
    let dup = ent
        .creer_personne_for_actor(etab, d, r, &personne(&mere.nom, "bis", None, Some("06 00 00 00 01")))
        .await
        .unwrap();
    let l_harry = ent.lier_personne_for_actor(etab, d, r, harry, &lien(dup.id, "mère", None)).await.unwrap().actif();
    let l_draco = ent.lier_personne_for_actor(etab, d, r, draco, &lien(dup.id, "tante", None)).await.unwrap().actif();
    let c = ent
        .ajouter_contact_for_actor(etab, d, r, draco, &ContactSaisie {
            personne_id: Some(dup.id),
            nouvelle_personne: None,
            relation: "tante".into(),
            ordre: 2,
            peut_recuperer: true,
            commentaire: None,
        })
        .await
        .unwrap();

    let avant = etat(&ent, etab).await;

    // Refus : sans le droit ; deux comptes différents ; motif vide.
    assert!(matches!(
        ent.fusionner_personnes_for_actor(etab, cast.administratif.user_id, Role::Administratif.as_str(), dup.id, mere.id, "doublon").await,
        Err(EntDbError::Forbidden)
    ));
    let autre_compte = ent
        .responsables_eleve_for_actor(etab, d, r, draco)
        .await
        .unwrap()
        .into_iter()
        .find(|x| x.personne.user_id == Some(cast.parent_tom.user_id))
        .unwrap()
        .personne;
    assert!(matches!(
        ent.fusionner_personnes_for_actor(etab, d, r, autre_compte.id, mere.id, "doublon").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent.fusionner_personnes_for_actor(etab, d, r, dup.id, mere.id, "  ").await.is_err());

    // Fusion du doublon (sans compte) dans la mère (avec compte).
    let f = ent
        .fusionner_personnes_for_actor(etab, d, r, dup.id, mere.id, "Doublon saisi à l'inscription")
        .await
        .unwrap();
    let liens_dup = compter(&ent, "SELECT count(*) FROM ent_eleve_lien WHERE personne_id = $1", dup.id).await;
    assert_eq!(liens_dup, 0);
    let resp_draco = ent.responsables_eleve_for_actor(etab, d, r, draco).await.unwrap();
    let deplace = resp_draco.iter().find(|x| x.lien_id == l_draco.lien_id).unwrap();
    assert_eq!(deplace.personne.id, mere.id);
    assert_eq!(deplace.personne.user_id, Some(cast.parent_lea.user_id), "le lien déplacé suit le compte de la cible");
    assert!(ent.responsables_eleve_for_actor(etab, d, r, harry).await.unwrap().iter().all(|x| x.lien_id != l_harry.lien_id));
    let contacts = ent.contacts_eleve_for_actor(etab, d, r, draco).await.unwrap();
    assert_eq!(contacts.iter().find(|x| x.id == c.id).unwrap().personne.id, mere.id);
    // La personne absorbée n'est plus proposée et ne peut plus être liée.
    assert!(ent.rechercher_personnes_for_actor(etab, d, r, "bis").await.unwrap().is_empty());
    assert!(matches!(
        ent.lier_personne_for_actor(etab, d, r, harry, &lien(dup.id, "mère", None)).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Fusion en double refusée.
    assert!(ent.fusionner_personnes_for_actor(etab, d, r, dup.id, mere.id, "encore").await.is_err());

    // Défusion : état exactement restauré ; une seconde annulation est refusée.
    let g = ent.defusionner_personnes_for_actor(etab, d, r, f.id).await.unwrap();
    assert!(g.annule_le.is_some());
    assert_eq!(etat(&ent, etab).await, avant, "état restauré à l'identique");
    assert!(matches!(
        ent.defusionner_personnes_for_actor(etab, d, r, f.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Au-delà de 30 jours : défusion refusée.
    let f2 = ent.fusionner_personnes_for_actor(etab, d, r, dup.id, mere.id, "Doublon").await.unwrap();
    sqlx::query("UPDATE ent_personne_fusion SET fusionne_le = now() - interval '31 days' WHERE id = $1")
        .bind(f2.id)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(matches!(
        ent.defusionner_personnes_for_actor(etab, d, r, f2.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Compte transféré : doublon AVEC compte absorbé par une fiche sans compte.
    let sans = ent.creer_personne_for_actor(etab, d, r, &personne("Weasley", "Arthur", None, None)).await.unwrap();
    let f3 = ent
        .fusionner_personnes_for_actor(etab, d, r, autre_compte.id, sans.id, "Fiche saisie en double")
        .await
        .unwrap();
    assert!(ent.lien_parent_eleve(etab, cast.parent_tom.user_id, draco).await.unwrap().is_some(), "accès conservé");
    let cible = ent.get_personne_for_actor(etab, d, r, sans.id).await.unwrap();
    assert_eq!(cible.user_id, Some(cast.parent_tom.user_id));
    ent.defusionner_personnes_for_actor(etab, d, r, f3.id).await.unwrap();
    let source = ent.get_personne_for_actor(etab, d, r, autre_compte.id).await.unwrap();
    assert_eq!(source.user_id, Some(cast.parent_tom.user_id));
    assert!(ent.lien_parent_eleve(etab, cast.parent_tom.user_id, draco).await.unwrap().is_some());
}

/// Chronologie : journal + événements, filtrée (pas de santé pour la direction,
/// pas d'entrée « dossier » pour la famille).
#[tokio::test]
async fn chronologie_filtree_par_les_droits() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let harry = cast.eleve_lea_id;
    let p = ent.creer_personne_for_actor(etab, d, r, &personne("Black", "Sirius", None, None)).await.unwrap();
    ent.lier_personne_for_actor(etab, d, r, harry, &lien(p.id, "parrain", None)).await.unwrap().actif();
    // Un passage à l'infirmerie (événement forgé : seule la classification compte).
    sqlx::query("INSERT INTO ent_metier_event (id, etablissement_id, type, aggregate, aggregate_id, eleve_id, payload, created_at)
                 VALUES ($1,$2,'infirmerie.passage','infirmerie',$1,$3,'{\"motif\":\"secret\"}', now())")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(harry)
        .execute(ent.pool())
        .await
        .unwrap();
    let chrono_dir = ent.chronologie_eleve_for_actor(etab, d, r, harry, 200).await.unwrap();
    assert!(chrono_dir.iter().any(|e| e.titre == "Responsable lié ou modifié"));
    assert!(chrono_dir.iter().all(|e| e.domaine != "sante"));
    assert!(chrono_dir.windows(2).all(|w| w[0].quand >= w[1].quand), "du plus récent au plus ancien");
    let chrono_inf = ent
        .chronologie_eleve_for_actor(etab, cast.infirmieres[0].user_id, Role::Infirmerie.as_str(), harry, 200)
        .await
        .unwrap();
    assert!(chrono_inf.iter().any(|e| e.domaine == "sante" && e.titre == "Passage à l'infirmerie"));
    assert!(chrono_inf.iter().all(|e| !e.titre.contains("secret")));
    let chrono_parent = ent
        .chronologie_eleve_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), harry, 200)
        .await
        .unwrap();
    assert!(chrono_parent.iter().all(|e| ["scolarite", "vie_scolaire", "notes"].contains(&e.domaine)));
    assert!(ent
        .chronologie_eleve_for_actor(etab, cast.parent_tom.user_id, Role::Parent.as_str(), harry, 200)
        .await
        .is_err());
}
