//! Socle 19, lots AG2/AG3 : réservation de lieux et disponibilités, contre la base locale de labo.
//! Chaque test porte le défaut NJ-xx du Next.js qu'il ferme (règle 14) ou la règle D-058 qu'il vérifie.

use super::*;
use crate::tests_agenda_ag0::corps_notifications;
use crate::tests_edt_t1_t2::{adm, creer_salle, creneau, decor, dir, h, labo as labo_edt, Decor};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use ent_auth::Role;
use ent_types::agenda::*;
use uuid::Uuid;

/// Un lundi futur (au moins 9 jours), dans l'année courante du décor.
fn lundi_futur() -> NaiveDate {
    let mut j = Utc::now().date_naive() + Duration::days(9);
    j += Duration::days((8 - i64::from(j.weekday().number_from_monday())) % 7);
    j
}

async fn nettoyer_reservations(ent: &PgEnt, etab: Uuid) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_agenda_reservation_fait DISABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    for t in [
        "ent_agenda_reservation_fait",
        "ent_agenda_reservation",
        "ent_agenda_lieu_regle",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("ALTER TABLE ent_agenda_reservation_fait ENABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn labo() -> Option<(PgEnt, LaboCast, Decor)> {
    let (ent, cast) = labo_edt().await?;
    nettoyer_reservations(&ent, cast.etablissement_id).await;
    let d = decor(&ent, &cast).await;
    Some((ent, cast, d))
}

fn prof(cast: &LaboCast, i: usize) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.profs[i].user_id,
        role: Role::Prof.as_str(),
    }
}

fn demande(salle: Uuid, jour: NaiveDate, a: u32, b: u32, intitule: &str) -> NouvelleReservation {
    NouvelleReservation {
        salle_id: salle,
        jour,
        heure_debut: h(a, 0),
        heure_fin: h(b, 0),
        intitule: intitule.into(),
        motif_categorie: None,
        effectif: None,
        mise_en_place: None,
        commentaire: None,
        pour_user_id: None,
        serie: None,
        confirmer_partiel: false,
    }
}

/// NJ-62 : le Next.js renvoyait toutes les réservations à n'importe quel compte, sans borne de période.
#[tokio::test]
async fn nj62_planning_borne_et_commentaire_reserve_aux_parties() {
    let Some((ent, cast, d)) = labo().await else {
        return;
    };
    let j = lundi_futur();
    let p0 = prof(&cast, 0);
    let p1 = prof(&cast, 1);
    let mut n = demande(d.salle, j, 14, 15, "Réunion pédagogique");
    n.commentaire = Some("Code porte 4521".into());
    let r = ent.agenda_reserver_lieu(&p0, n).await.unwrap();
    assert!(
        r.ecrit && r.occurrences[0].statut.as_deref() == Some("confirmee"),
        "{r:?}"
    );

    // Plage non bornée ou de plus de 42 jours : refusée.
    assert!(ent
        .agenda_planning_salle(&p1, d.salle, j, j + Duration::days(PLANNING_MAX_JOURS_TEST))
        .await
        .is_err());
    assert!(ent
        .agenda_planning_salle(&p1, d.salle, j + Duration::days(1), j)
        .await
        .is_err());
    // Un collègue voit qui et quoi, pas le commentaire.
    let vue = ent
        .agenda_planning_salle(&p1, d.salle, j, j + Duration::days(1))
        .await
        .unwrap();
    assert_eq!(vue.reservations.len(), 1);
    assert_eq!(vue.reservations[0].intitule, "Réunion pédagogique");
    assert!(vue.reservations[0].commentaire.is_none());
    assert!(!vue.reservations[0].je_suis_demandeur);
    // Le demandeur et le gestionnaire le voient.
    let mien = ent
        .agenda_planning_salle(&p0, d.salle, j, j + Duration::days(1))
        .await
        .unwrap();
    assert_eq!(
        mien.reservations[0].commentaire.as_deref(),
        Some("Code porte 4521")
    );
    let gest = ent
        .agenda_planning_salle(&adm(&cast), d.salle, j, j + Duration::days(1))
        .await
        .unwrap();
    assert!(gest.reservations[0].commentaire.is_some());
    // Un parent n'a aucun accès au planning des salles.
    let parent = Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.parent_lea.user_id,
        role: Role::Parent.as_str(),
    };
    assert!(matches!(
        ent.agenda_planning_salle(&parent, d.salle, j, j + Duration::days(1))
            .await,
        Err(EntDbError::Forbidden)
    ));
}

const PLANNING_MAX_JOURS_TEST: i64 = crate::agenda_lieux::PLANNING_MAX_JOURS;

/// NJ-76 : n'importe quel compte pouvait annuler ou modifier la réservation d'un autre.
#[tokio::test]
async fn nj76_annuler_reservation_d_autrui_est_404_et_gestionnaire_motive() {
    let Some((ent, cast, d)) = labo().await else {
        return;
    };
    let j = lundi_futur();
    let (p0, p1) = (prof(&cast, 0), prof(&cast, 1));
    let r = ent
        .agenda_reserver_lieu(&p0, demande(d.salle, j, 10, 11, "Soutien"))
        .await
        .unwrap();
    let id = r.occurrences[0].reservation_id.unwrap();
    let vide = AnnulationReservation {
        motif: None,
        portee: None,
    };
    // Un autre professeur : 404 comme pour une réservation inexistante, et rien n'est changé.
    assert!(matches!(
        ent.agenda_annuler_reservation(&p1, id, vide.clone()).await,
        Err(EntDbError::Domain(_))
    ));
    assert!(matches!(
        ent.agenda_annuler_reservation(&p1, Uuid::new_v4(), vide.clone())
            .await,
        Err(EntDbError::Domain(_))
    ));
    assert!(matches!(
        ent.agenda_decider_reservation(
            &p1,
            id,
            DecisionReservation {
                accepter: true,
                motif: None
            }
        )
        .await,
        Err(EntDbError::Domain(_))
    ));
    let (statut,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_agenda_reservation WHERE id = $1")
            .bind(id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut, "confirmee");
    // Le gestionnaire doit motiver ; le demandeur est prévenu avec le motif.
    assert!(matches!(
        ent.agenda_annuler_reservation(&adm(&cast), id, vide.clone())
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.agenda_annuler_reservation(
        &adm(&cast),
        id,
        AnnulationReservation {
            motif: Some("Examen blanc".into()),
            portee: None,
        },
    )
    .await
    .unwrap();
    let corps = corps_notifications(
        &ent,
        cast.etablissement_id,
        p0.user_id,
        "agenda_lieu_annule",
    )
    .await;
    assert!(
        corps.iter().any(|c| c.contains("Examen blanc")),
        "{corps:?}"
    );
    // La salle est de nouveau libre : un autre peut la réserver, puis son auteur l'annule seul.
    let r2 = ent
        .agenda_reserver_lieu(&p1, demande(d.salle, j, 10, 11, "Autre"))
        .await
        .unwrap();
    let id2 = r2.occurrences[0].reservation_id.unwrap();
    assert_eq!(
        ent.agenda_annuler_reservation(&p1, id2, vide.clone())
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        ent.agenda_annuler_reservation(&p1, id2, vide).await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

/// ND-01 / ND-02 / ND-03 : conflit dit par nom, série à rapport par occurrence, créneau passé refusé.
#[tokio::test]
async fn conflits_serie_a_rapport_et_creneau_passe() {
    let Some((ent, cast, d)) = labo().await else {
        return;
    };
    let j = lundi_futur();
    let (p0, p1) = (prof(&cast, 0), prof(&cast, 1));
    ent.agenda_reserver_lieu(
        &p0,
        demande(d.salle, j + Duration::days(7), 16, 17, "Atelier"),
    )
    .await
    .unwrap();
    // Conflit unitaire : le message nomme l'intitulé du collègue.
    let e = ent
        .agenda_reserver_lieu(
            &p1,
            demande(d.salle, j + Duration::days(7), 16, 17, "Autre"),
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::ConflitDossierDetail(ref m)) if m.contains("Atelier")),
        "{e:?}"
    );
    // Série de 3 lundis dont le deuxième est pris : rien n'est écrit sans confirmation, rapport par date.
    let mut s = demande(d.salle, j, 16, 17, "Club");
    s.serie = Some(SerieReservation {
        jours_semaine: vec![1],
        intervalle_semaines: Some(1),
        au: j + Duration::days(14),
    });
    let rap = ent.agenda_reserver_lieu(&p1, s.clone()).await.unwrap();
    assert!(!rap.ecrit);
    assert_eq!(rap.occurrences.len(), 3);
    assert_eq!(
        rap.occurrences
            .iter()
            .filter(|o| o.conflit.is_some())
            .count(),
        1
    );
    s.confirmer_partiel = true;
    let rap = ent.agenda_reserver_lieu(&p1, s).await.unwrap();
    assert!(rap.ecrit);
    assert_eq!(
        rap.occurrences
            .iter()
            .filter(|o| o.reservation_id.is_some())
            .count(),
        2
    );
    // Passé, fin avant début : refusés.
    let hier = Utc::now().date_naive() - Duration::days(1);
    assert!(ent
        .agenda_reserver_lieu(&p0, demande(d.salle, hier, 9, 10, "Hier"))
        .await
        .is_err());
    assert!(ent
        .agenda_reserver_lieu(&p0, demande(d.salle, j, 10, 9, "Inversé"))
        .await
        .is_err());
}

/// D-058 : toutes les salles (classes comprises), réglage par salle ; un cours prime sur une réservation.
#[tokio::test]
async fn regles_par_salle_et_cours_prioritaire() {
    let Some((ent, cast, d)) = labo().await else {
        return;
    };
    let (p0, g) = (prof(&cast, 0), dir(&cast));
    let j = lundi_futur();
    // Un cours occupe B12 le lundi de 8 h à 9 h : la réservation est refusée, la recherche l'écarte.
    let _ = creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    ent.ref_generer_seances(
        &adm(&cast),
        d.annee_id,
        Some(j),
        Some(j + Duration::days(6)),
    )
    .await
    .unwrap();
    let e = ent
        .agenda_reserver_lieu(&p0, demande(d.salle, j, 8, 9, "Rattrapage"))
        .await;
    assert!(
        matches!(e, Err(EntDbError::ConflitDossierDetail(ref m)) if m.contains("4B")),
        "{e:?}"
    );
    let libres = ent
        .agenda_rechercher_salles(
            &p0,
            RechercheSalle {
                jour: j,
                heure_debut: h(8, 0),
                heure_fin: h(9, 0),
                capacite_min: None,
                type_salle: None,
            },
        )
        .await
        .unwrap();
    assert!(!libres.iter().any(|s| s.salle_id == d.salle));
    // Mode « jamais » : refusée même libre ; « sur validation » : demande, puis décision du valideur.
    let autre = creer_salle(
        &ent,
        cast.etablissement_id,
        cast.administratif.user_id,
        "AG3-V",
    )
    .await;
    let regle = |mode: &str, valideur| RegleLieu {
        mode: mode.into(),
        valideur_user_id: valideur,
        ..RegleLieu::default()
    };
    assert!(matches!(
        ent.agenda_parametrer_lieu(&p0, autre, regle("jamais", None))
            .await,
        Err(EntDbError::Forbidden)
    ));
    ent.agenda_parametrer_lieu(&g, autre, regle("jamais", None))
        .await
        .unwrap();
    assert!(matches!(
        ent.agenda_reserver_lieu(&p0, demande(autre, j, 10, 11, "X"))
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent
        .agenda_parametrer_lieu(&g, autre, regle("sur_validation", None))
        .await
        .is_err());
    ent.agenda_parametrer_lieu(
        &g,
        autre,
        regle("sur_validation", Some(cast.administratif.user_id)),
    )
    .await
    .unwrap();
    let r = ent
        .agenda_reserver_lieu(&p0, demande(autre, j, 10, 11, "Projet"))
        .await
        .unwrap();
    assert_eq!(r.occurrences[0].statut.as_deref(), Some("demandee"));
    let id = r.occurrences[0].reservation_id.unwrap();
    assert!(!corps_notifications(
        &ent,
        cast.etablissement_id,
        cast.administratif.user_id,
        "agenda_lieu_demande"
    )
    .await
    .is_empty());
    // Refus sans motif impossible ; un tiers n'a pas à connaître la demande.
    let vide = DecisionReservation {
        accepter: false,
        motif: None,
    };
    assert!(matches!(
        ent.agenda_decider_reservation(&adm(&cast), id, vide).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.agenda_decider_reservation(
        &adm(&cast),
        id,
        DecisionReservation {
            accepter: true,
            motif: None,
        },
    )
    .await
    .unwrap();
    let mes = ent
        .agenda_mes_reservations(&p0, j, j + Duration::days(1))
        .await
        .unwrap();
    assert_eq!(mes[0].statut, "confirmee");
}

/// D-058 de bout en bout : cours supprimé → salle libre → réservation → remplaçant nommé → réservation
/// tombée, demandeur prévenu avec le motif public (jamais le motif d'absence) et des salles de rechange.
#[tokio::test]
async fn cours_supprime_libere_puis_remplacant_fait_tomber_la_reservation() {
    let Some((ent, cast, d)) = labo().await else {
        return;
    };
    let (p0, a) = (prof(&cast, 0), adm(&cast));
    let j = lundi_futur();
    let _ = creneau(
        &ent,
        &cast,
        Some(d.cours),
        1,
        h(11, 0),
        h(12, 0),
        Some("B12"),
    )
    .await;
    ent.ref_generer_seances(&a, d.annee_id, Some(j), Some(j + Duration::days(6)))
        .await
        .unwrap();
    let (seance,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_edt_seance WHERE etablissement_id = $1 AND jour = $2 AND salle = 'B12' ORDER BY debut_utc LIMIT 1",
    )
    .bind(cast.etablissement_id)
    .bind(j)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1")
        .bind(seance)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // La salle de classe est libérée : proposée avec sa raison, et réservable.
    let libres = ent
        .agenda_rechercher_salles(
            &p0,
            RechercheSalle {
                jour: j,
                heure_debut: h(11, 0),
                heure_fin: h(12, 0),
                capacite_min: None,
                type_salle: None,
            },
        )
        .await
        .unwrap();
    let b12 = libres
        .iter()
        .find(|s| s.salle_id == d.salle)
        .expect("B12 libérée");
    assert_eq!(b12.raison_libre.as_deref(), Some("cours de 4B supprimé"));
    let r = ent
        .agenda_reserver_lieu(&p0, demande(d.salle, j, 11, 12, "Soutien"))
        .await
        .unwrap();
    let id = r.occurrences[0].reservation_id.unwrap();
    // Le cours est rétabli par un remplaçant : le cours prime, la réservation tombe.
    let remp = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap()
        .user_id;
    sqlx::query(
        "UPDATE ent_edt_seance SET statut = 'remplacee', remplacant_user_id = $2 WHERE id = $1",
    )
    .bind(seance)
    .bind(remp)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let n = ent
        .agenda_reviser_reservations_edt(Utc::now(), Some(cast.etablissement_id))
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(
        ent.agenda_reviser_reservations_edt(Utc::now(), Some(cast.etablissement_id))
            .await
            .unwrap(),
        0,
        "idempotent"
    );
    let (statut, motif): (String, Option<String>) =
        sqlx::query_as("SELECT statut, motif_decision FROM ent_agenda_reservation WHERE id = $1")
            .bind(id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut, "tombee");
    assert!(motif.unwrap().contains("4B"));
    let corps = corps_notifications(
        &ent,
        cast.etablissement_id,
        p0.user_id,
        "agenda_lieu_tombee",
    )
    .await;
    assert_eq!(corps.len(), 1, "{corps:?}");
    assert!(
        corps[0].contains("4B") && !corps[0].to_lowercase().contains("maladie"),
        "{}",
        corps[0]
    );
}

/// AG2 : le créneau commun vient de l'EDT réel ; santé et social ne sont jamais détaillés (R-AG37).
#[tokio::test]
async fn disponibilites_calculees_sans_motif_et_sans_sante() {
    let Some((ent, cast, d)) = labo().await else {
        return;
    };
    let (g, j) = (dir(&cast), lundi_futur());
    let _ = d;
    let (p0, p1) = (cast.profs[0].user_id, cast.profs[1].user_id);
    let v = ent
        .agenda_libre_occupe(&g, &[p0, p1], j, h(8, 0), h(9, 0))
        .await
        .unwrap();
    assert_eq!(v.len(), 2);
    assert!(v.iter().all(|x| x.etat == "libre" || x.etat == "occupe"));
    let inf = cast.infirmieres[0].user_id;
    let v = ent
        .agenda_libre_occupe(&g, &[inf], j, h(8, 0), h(9, 0))
        .await
        .unwrap();
    assert_eq!(v[0].etat, "non_communique");
    assert!(ent
        .agenda_creneau_commun(&g, &[p0, inf], j, j, 30)
        .await
        .is_err());
    let c = ent
        .agenda_creneau_commun(&g, &[p0, p1], j, j + Duration::days(4), 30)
        .await
        .unwrap();
    assert!(c.iter().all(|p| (p.fin - p.debut).num_minutes() >= 30));
    // Un parent ne consulte pas les disponibilités du personnel.
    let parent = Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.parent_lea.user_id,
        role: Role::Parent.as_str(),
    };
    assert!(matches!(
        ent.agenda_libre_occupe(&parent, &[p0], j, h(8, 0), h(9, 0))
            .await,
        Err(EntDbError::Forbidden)
    ));
}
