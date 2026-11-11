//! Socle 19, lot AG1 : noyau de l'agenda (événements, séries, audience sous les droits de l'auteur,
//! réponses, coupon, journal, vue calculée, iCal), contre la base locale de labo (feature `labo`).
//! Chaque test porte la règle ou le piège de `19_AGENDA_RDV.md` qu'il vérifie. Sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés. Selon `ENT_RLS_MODE`, le code testé tourne sur le
//! rôle applicatif strict (sans BYPASSRLS).

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use crate::tests_agenda_ag0::{
    classe_4b, corps_notifications, dir, labo as labo_ag0, reset as reset_ag0,
};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Portee, Role};
use ent_types::agenda::*;
use uuid::Uuid;

/// Vide l'agenda d'un établissement (tests de labo seulement : le journal est en ajout seul).
async fn vider_agenda_pour_tests(pool: &sqlx::PgPool, etab: Uuid) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_agenda_journal DISABLE TRIGGER ent_agenda_journal_immuable")
        .execute(&mut *tx)
        .await
        .unwrap();
    for t in [
        "ent_agenda_journal",
        "ent_agenda_coupon_reponse",
        "ent_agenda_coupon",
        "ent_agenda_participant",
        "ent_agenda_audience",
        "ent_agenda_evenement",
        "ent_agenda_serie_jour",
        "ent_agenda_serie",
        "ent_agenda_type_local",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("ALTER TABLE ent_agenda_journal ENABLE TRIGGER ent_agenda_journal_immuable")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let (ent, cast) = labo_ag0().await?;
    reset(&ent, cast.etablissement_id).await;
    Some((ent, cast))
}

async fn reset(ent: &PgEnt, etab: Uuid) {
    vider_agenda_pour_tests(ent.pool_verif(), etab).await;
    reset_ag0(ent, etab).await;
}

fn jour(dans: i64) -> NaiveDate {
    (Utc::now() + Duration::days(dans)).date_naive()
}

fn h(h: u32, m: u32) -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(h, m, 0)
}

fn evt(titre: &str, dans_jours: i64) -> NouvelEvenement {
    NouvelEvenement {
        type_code: "reunion".into(),
        titre: titre.into(),
        description: None,
        jour: jour(dans_jours),
        jour_fin: None,
        heure_debut: h(17, 0),
        heure_fin: h(18, 0),
        salle_id: None,
        lieu_texte: Some("Salle des professeurs".into()),
        confidentialite: None,
        invites: vec![],
        coupon: None,
    }
}

const PROF: &str = "prof";
const CPE: &str = "cpe";

async fn publier_a(
    ent: &PgEnt,
    cast: &LaboCast,
    par: (Uuid, &str),
    id: Uuid,
    audiences: Vec<AudienceDto>,
) -> EvenementDetail {
    let etab = cast.etablissement_id;
    let ap = ent
        .agenda_publier_evenement(
            etab,
            par.0,
            par.1,
            id,
            DemandePublication {
                audiences: audiences.clone(),
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    ent.agenda_publier_evenement(
        etab,
        par.0,
        par.1,
        id,
        DemandePublication {
            audiences,
            empreinte: Some(ap.empreinte),
            apercu_seulement: false,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap_err()
}

fn classe(id: Uuid) -> AudienceDto {
    AudienceDto {
        kind: "classe".into(),
        cible_id: Some(id),
    }
}

// ─── R-AG5 : brouillon, place du lecteur ─────────────────────────────────────────────────────

#[tokio::test]
async fn ag1_brouillon_invisible_et_evenement_participants_hors_partie_publique_introuvable() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    let c = ent
        .agenda_creer_evenement(etab, prof.0, prof.1, evt("Réunion d'équipe", 10))
        .await
        .unwrap();
    // L'organisateur le voit ; ni un collègue, ni une famille, ni un autre établissement.
    let d = ent
        .agenda_evenement(etab, prof.0, prof.1, c.id)
        .await
        .unwrap();
    assert_eq!(
        (d.statut.as_str(), d.je_suis_organisateur),
        ("brouillon", true)
    );
    for (u, r) in [(cast.cpe.user_id, CPE), (cast.parent_lea.user_id, "parent")] {
        let e = ent.agenda_evenement(etab, u, r, c.id).await;
        assert!(
            matches!(
                e,
                Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
            ),
            "brouillon : {e:?}"
        );
    }
    assert!(
        ent.agenda_evenement(Uuid::new_v4(), prof.0, prof.1, c.id)
            .await
            .is_err(),
        "autre établissement"
    );
    // Une famille ne crée pas d'événement ; un titre vide, un type inconnu ou une plage inversée sont refusés.
    assert!(matches!(
        ent.agenda_creer_evenement(etab, cast.parent_lea.user_id, "parent", evt("x", 5))
            .await,
        Err(EntDbError::Forbidden)
    ));
    for mauvais in [
        NouvelEvenement {
            titre: "  ".into(),
            ..evt("x", 5)
        },
        NouvelEvenement {
            type_code: "inconnu".into(),
            ..evt("x", 5)
        },
        NouvelEvenement {
            heure_fin: h(16, 0),
            ..evt("x", 5)
        },
        NouvelEvenement {
            heure_fin: None,
            ..evt("x", 5)
        },
        NouvelEvenement {
            salle_id: Some(Uuid::new_v4()),
            lieu_texte: None,
            ..evt("x", 5)
        },
        NouvelEvenement {
            confidentialite: Some("public".into()),
            ..evt("x", 5)
        },
    ] {
        let r = ent
            .agenda_creer_evenement(etab, prof.0, prof.1, mauvais)
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    }
    reset(&ent, etab).await;
}

// ─── R-AG8 : audience résolue sous les droits de l'auteur, aperçu et confirmation ────────────

#[tokio::test]
async fn ag1_publication_a_une_classe_avec_apercu_et_confirmation_par_empreinte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_4b(&ent, etab).await;
    let c = ent
        .agenda_creer_evenement(etab, adm, role, evt("Réunion de rentrée 4B", 12))
        .await
        .unwrap();
    let aud = vec![classe(c4b)];
    let ap = ent
        .agenda_publier_evenement(
            etab,
            adm,
            role,
            c.id,
            DemandePublication {
                audiences: aud.clone(),
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert!(
        ap.nb_responsables >= 1 && ap.exclus_hors_droits == 0,
        "{ap:?}"
    );
    // Rien n'a été écrit par l'aperçu.
    assert_eq!(
        ent.agenda_evenement(etab, adm, role, c.id)
            .await
            .unwrap()
            .statut,
        "brouillon"
    );
    // Sans confirmation ou avec une empreinte périmée : refus.
    let r = ent
        .agenda_publier_evenement(
            etab,
            adm,
            role,
            c.id,
            DemandePublication {
                audiences: aud.clone(),
                empreinte: None,
                apercu_seulement: false,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = ent
        .agenda_publier_evenement(
            etab,
            adm,
            role,
            c.id,
            DemandePublication {
                audiences: aud.clone(),
                empreinte: Some("0".repeat(64)),
                apercu_seulement: false,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let d = publier_a(&ent, &cast, (adm, role), c.id, aud).await;
    assert_eq!(d.statut, "publie");
    assert!(d
        .participants
        .as_ref()
        .unwrap()
        .iter()
        .any(|p| p.user_id == cast.parent_lea.user_id
            && p.role == "famille"
            && p.reponse == "en_attente"));
    // La famille de 4B voit l'événement et est prévenue ; la famille d'une autre classe n'en sait rien.
    let vu = ent
        .agenda_evenement(etab, cast.parent_lea.user_id, "parent", c.id)
        .await
        .unwrap();
    assert_eq!(vu.ma_reponse.as_deref(), Some("en_attente"));
    assert!(
        vu.participants.is_none(),
        "un invité ne reçoit que les décomptes"
    );
    assert!(
        !corps_notifications(&ent, etab, cast.parent_lea.user_id, "agenda_invitation")
            .await
            .is_empty()
    );
    let autre = ent
        .agenda_evenement(etab, cast.parent_tom.user_id, "parent", c.id)
        .await;
    assert!(
        matches!(
            autre,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{autre:?}"
    );
    assert!(
        corps_notifications(&ent, etab, cast.parent_tom.user_id, "agenda_invitation")
            .await
            .is_empty()
    );
    // Déjà publié : on ne publie pas deux fois.
    let r = ent
        .agenda_publier_evenement(
            etab,
            adm,
            role,
            c.id,
            DemandePublication {
                audiences: vec![],
                empreinte: None,
                apercu_seulement: false,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    reset(&ent, etab).await;
}

#[tokio::test]
async fn ag1_audience_limitee_aux_droits_de_l_auteur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let c4b = classe_4b(&ent, etab).await;
    let c5a: Uuid = sqlx::query_scalar(
        "SELECT c.id FROM ent_classe c
           JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.code = '5A' AND c.archivee_le IS NULL
          ORDER BY y.courante DESC, y.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let prof = (cast.prof.user_id, PROF);
    let c = ent
        .agenda_creer_evenement(etab, prof.0, prof.1, evt("Information 4B", 8))
        .await
        .unwrap();
    // Un professeur n'a pas le droit de publier à une audience par défaut.
    let r = ent
        .agenda_publier_evenement(
            etab,
            prof.0,
            prof.1,
            c.id,
            DemandePublication {
                audiences: vec![classe(c4b)],
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Un rôle limité à 4B (professeur principal) lui est attribué.
    let r = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("Agenda 4B {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::AgendaEvenementsPublier,
                    portee: Portee::Designes,
                    cibles: vec![c4b],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, cast.admin.user_id, "admin", r.id, prof.0, None)
        .await
        .unwrap();
    // Toute l'établissement : interdit à un rôle limité ; une autre classe : élèves exclus, aucun destinataire.
    let tout = ent
        .agenda_publier_evenement(
            etab,
            prof.0,
            prof.1,
            c.id,
            DemandePublication {
                audiences: vec![AudienceDto {
                    kind: "etablissement".into(),
                    cible_id: None,
                }],
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(tout, Err(EntDbError::Forbidden)), "{tout:?}");
    let pers = ent
        .agenda_publier_evenement(
            etab,
            prof.0,
            prof.1,
            c.id,
            DemandePublication {
                audiences: vec![AudienceDto {
                    kind: "tous_personnels".into(),
                    cible_id: None,
                }],
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(pers, Err(EntDbError::Forbidden)), "{pers:?}");
    let hors = ent
        .agenda_publier_evenement(
            etab,
            prof.0,
            prof.1,
            c.id,
            DemandePublication {
                audiences: vec![classe(c5a)],
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hors.nb_responsables, 0);
    assert!(hors.exclus_hors_droits >= 1, "{hors:?}");
    // Sa classe : accepté.
    let d = publier_a(&ent, &cast, prof, c.id, vec![classe(c4b)]).await;
    assert_eq!(d.statut, "publie");
    // Une cible d'un autre établissement est refusée.
    let c2 = ent
        .agenda_creer_evenement(etab, prof.0, prof.1, evt("Autre", 9))
        .await
        .unwrap();
    let r = ent
        .agenda_publier_evenement(
            etab,
            prof.0,
            prof.1,
            c2.id,
            DemandePublication {
                audiences: vec![classe(Uuid::new_v4())],
                empreinte: None,
                apercu_seulement: true,
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    ent.supprimer_role_for_actor(
        etab,
        cast.admin.user_id,
        "admin",
        r_id(&ent, etab, prof.0).await,
    )
    .await
    .ok();
    reset(&ent, etab).await;
}

async fn r_id(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT role_id FROM ent_role_attribution WHERE etablissement_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT 1")
        .bind(etab)
        .bind(user)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

// ─── Piège 7 : un élève (ou un parent) n'est pas invitable nominativement ────────────────────

#[tokio::test]
async fn ag1_invites_nominatifs_personnel_seulement_et_publication_sans_audience() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    for non_invitable in [cast.parent_lea.user_id, Uuid::new_v4()] {
        let r = ent
            .agenda_creer_evenement(
                etab,
                prof.0,
                prof.1,
                NouvelEvenement {
                    invites: vec![non_invitable],
                    ..evt("x", 6)
                },
            )
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    }
    let c = ent
        .agenda_creer_evenement(
            etab,
            prof.0,
            prof.1,
            NouvelEvenement {
                invites: vec![cast.cpe.user_id],
                ..evt("Point avec la vie scolaire", 6)
            },
        )
        .await
        .unwrap();
    // Sans audience, la publication n'exige que le droit de créer (invités nominatifs).
    let d = ent
        .agenda_publier_evenement(
            etab,
            prof.0,
            prof.1,
            c.id,
            DemandePublication {
                audiences: vec![],
                empreinte: None,
                apercu_seulement: false,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(d.statut, "publie");
    assert_eq!(
        ent.agenda_evenement(etab, cast.cpe.user_id, CPE, c.id)
            .await
            .unwrap()
            .ma_reponse
            .as_deref(),
        Some("en_attente")
    );
    assert!(
        !corps_notifications(&ent, etab, cast.cpe.user_id, "agenda_invitation")
            .await
            .is_empty()
    );
    // Un autre collègue, non invité, n'y a pas accès.
    assert!(ent
        .agenda_evenement(
            etab,
            cast.profs
                .get(1)
                .map(|p| p.user_id)
                .unwrap_or_else(Uuid::new_v4),
            PROF,
            c.id
        )
        .await
        .is_err());
    reset(&ent, etab).await;
}

// ─── Réponses, coupon, empêchement (AGD-128, AGD-129, AGD-144) ────────────────────────────────

#[tokio::test]
async fn ag1_reponses_coupon_et_empechement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    let cpe = cast.cpe.user_id;
    let n = NouvelEvenement {
        invites: vec![cpe],
        coupon: Some(NouveauCoupon {
            question: "Venez-vous au pot ?".into(),
            choix: vec!["Oui".into(), "Non".into(), "Peut-être".into()],
        }),
        ..evt("Fin de trimestre", 7)
    };
    let c = ent
        .agenda_creer_evenement(etab, prof.0, prof.1, n)
        .await
        .unwrap();
    // Avant publication : l'invité ne répond pas (404).
    let r = ent
        .agenda_repondre(
            etab,
            cpe,
            c.id,
            ReponseInvitation {
                reponse: "oui".into(),
                motif_excuse: None,
            },
        )
        .await;
    assert!(r.is_err(), "brouillon");
    ent.agenda_publier_evenement(
        etab,
        prof.0,
        prof.1,
        c.id,
        DemandePublication {
            audiences: vec![],
            empreinte: None,
            apercu_seulement: false,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap_err();
    let d = ent
        .agenda_repondre(
            etab,
            cpe,
            c.id,
            ReponseInvitation {
                reponse: "oui".into(),
                motif_excuse: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(d.ma_reponse.as_deref(), Some("oui"));
    assert!(ent
        .agenda_repondre(
            etab,
            cpe,
            c.id,
            ReponseInvitation {
                reponse: "bof".into(),
                motif_excuse: None
            }
        )
        .await
        .is_err());
    // Un non-invité ne répond pas.
    assert!(ent
        .agenda_repondre(
            etab,
            cast.parent_lea.user_id,
            c.id,
            ReponseInvitation {
                reponse: "oui".into(),
                motif_excuse: None
            }
        )
        .await
        .is_err());
    // Coupon : choix valide, modifiable, hors bornes refusé ; décompte pour l'organisateur seulement.
    ent.agenda_repondre_coupon(etab, cpe, c.id, ReponseCoupon { choix: 3 })
        .await
        .unwrap();
    let d = ent
        .agenda_repondre_coupon(etab, cpe, c.id, ReponseCoupon { choix: 1 })
        .await
        .unwrap();
    assert_eq!(d.coupon.as_ref().unwrap().mon_choix, Some(1));
    assert!(d.coupon.as_ref().unwrap().decompte.is_none());
    assert!(ent
        .agenda_repondre_coupon(etab, cpe, c.id, ReponseCoupon { choix: 4 })
        .await
        .is_err());
    let orga = ent
        .agenda_evenement(etab, prof.0, prof.1, c.id)
        .await
        .unwrap();
    assert_eq!(orga.coupon.unwrap().decompte, Some(vec![1, 0, 0]));
    assert_eq!(
        (orga.nb_oui, orga.nb_en_attente),
        (2, 0),
        "l'organisateur compte comme présent"
    );
    // Empêchement : « oui » puis « non » prévient l'organisateur, avec le motif.
    ent.agenda_repondre(
        etab,
        cpe,
        c.id,
        ReponseInvitation {
            reponse: "non".into(),
            motif_excuse: Some("Conseil de discipline".into()),
        },
    )
    .await
    .unwrap();
    let avis = corps_notifications(&ent, etab, prof.0, "agenda_empechement").await;
    assert!(
        avis.iter().any(|a| a.contains("Conseil de discipline")),
        "{avis:?}"
    );
    let orga = ent
        .agenda_evenement(etab, prof.0, prof.1, c.id)
        .await
        .unwrap();
    assert!(orga
        .participants
        .unwrap()
        .iter()
        .any(|p| p.user_id == cpe && p.reponse == "non"));
    reset(&ent, etab).await;
}

// ─── Séries matérialisées, modification, annulation (AGD-115 à AGD-120) ───────────────────────

#[tokio::test]
async fn ag1_serie_modification_par_portee_et_annulation_avec_motif() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    let cpe = cast.cpe.user_id;
    let lundi = {
        use chrono::Datelike;
        let j = jour(14);
        j + Duration::days(7 - j.weekday().num_days_from_monday() as i64)
    };
    let modele = NouvelEvenement {
        jour: lundi,
        invites: vec![cpe],
        ..evt("Point hebdomadaire", 0)
    };
    let s = ent
        .agenda_creer_serie(
            etab,
            prof.0,
            prof.1,
            NouvelleSerie {
                modele: modele.clone(),
                jours_semaine: vec![1, 3],
                intervalle_semaines: None,
                au: lundi + Duration::days(20),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        s.nb_occurrences, 6,
        "lundis et mercredis sur trois semaines"
    );
    let serie = s.serie_id.unwrap();
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM ent_agenda_evenement WHERE etablissement_id = $1 AND serie_id = $2 ORDER BY debut")
        .bind(etab)
        .bind(serie)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(ids.len(), 6);
    // Bornes : trop d'occurrences, jour invalide, coupon sur une série.
    for mauvaise in [
        NouvelleSerie {
            modele: modele.clone(),
            jours_semaine: vec![1, 2, 3, 4, 5, 6, 7],
            intervalle_semaines: None,
            au: lundi + Duration::days(300),
        },
        NouvelleSerie {
            modele: modele.clone(),
            jours_semaine: vec![8],
            intervalle_semaines: None,
            au: lundi + Duration::days(7),
        },
        NouvelleSerie {
            modele: NouvelEvenement {
                coupon: Some(NouveauCoupon {
                    question: "q".into(),
                    choix: vec!["a".into(), "b".into()],
                }),
                ..modele.clone()
            },
            jours_semaine: vec![1],
            intervalle_semaines: None,
            au: lundi + Duration::days(7),
        },
    ] {
        let r = ent.agenda_creer_serie(etab, prof.0, prof.1, mauvaise).await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    }
    // Publication de toute la série en une fois (occurrences en brouillon) : une seule invitation.
    let sans = DemandePublication {
        audiences: vec![],
        empreinte: None,
        apercu_seulement: false,
        portee: Some("toutes".into()),
    };
    ent.agenda_publier_evenement(etab, prof.0, prof.1, ids[0], sans.clone())
        .await
        .unwrap()
        .unwrap_err();
    let statuts: Vec<String> = sqlx::query_scalar(
        "SELECT statut FROM ent_agenda_evenement WHERE etablissement_id = $1 AND serie_id = $2",
    )
    .bind(etab)
    .bind(serie)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(statuts.iter().all(|s| s == "publie"), "{statuts:?}");
    assert_eq!(
        corps_notifications(&ent, etab, cpe, "agenda_invitation")
            .await
            .len(),
        1,
        "une seule invitation pour toute la série"
    );
    // Republier : plus rien à publier, mais pas d'erreur sur les occurrences déjà publiées ; on
    // décale la 2ᵉ (détachement + avis).
    let d2 = ent
        .agenda_evenement(etab, prof.0, prof.1, ids[1])
        .await
        .unwrap();
    let m = ModifEvenement {
        version: d2.version,
        heure_debut: h(18, 0),
        heure_fin: h(19, 0),
        ..Default::default()
    };
    // Version périmée : conflit.
    let r = ent
        .agenda_modifier_evenement(
            etab,
            prof.0,
            prof.1,
            ids[1],
            ModifEvenement {
                version: d2.version - 1,
                ..m.clone()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let d2b = ent
        .agenda_modifier_evenement(etab, prof.0, prof.1, ids[1], m)
        .await
        .unwrap();
    assert!(d2b.occurrence_detachee && d2b.version > d2.version);
    assert!(
        corps_notifications(&ent, etab, cpe, "agenda_modifie")
            .await
            .len()
            == 1,
        "le changement d'horaire prévient l'invité"
    );
    // Modification d'un titre pour les suivantes : toutes celles à partir de la 3ᵉ (la 2ᵉ est avant).
    let d3 = ent
        .agenda_evenement(etab, prof.0, prof.1, ids[2])
        .await
        .unwrap();
    ent.agenda_modifier_evenement(
        etab,
        prof.0,
        prof.1,
        ids[2],
        ModifEvenement {
            version: d3.version,
            portee: Some("suivantes".into()),
            titre: Some("Point hebdo".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let titres: Vec<String> = sqlx::query_scalar("SELECT titre FROM ent_agenda_evenement WHERE etablissement_id = $1 AND serie_id = $2 ORDER BY debut")
        .bind(etab)
        .bind(serie)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(&titres[..2], ["Point hebdomadaire", "Point hebdomadaire"]);
    assert!(titres[2..].iter().all(|t| t == "Point hebdo"), "{titres:?}");
    // Le jour ne se change que sur une occurrence ; un autre que l'organisateur n'y touche pas (404).
    let r = ent
        .agenda_modifier_evenement(
            etab,
            prof.0,
            prof.1,
            ids[2],
            ModifEvenement {
                version: 99,
                portee: Some("toutes".into()),
                jour: Some(lundi),
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = ent
        .agenda_modifier_evenement(
            etab,
            cpe,
            CPE,
            ids[2],
            ModifEvenement {
                version: 1,
                titre: Some("Piraté".into()),
                ..Default::default()
            },
        )
        .await;
    assert!(r.is_err());
    // Annulation : motif obligatoire ; « suivantes » annule les occurrences à partir de celle-ci.
    let r = ent
        .agenda_annuler_evenement(
            etab,
            prof.0,
            prof.1,
            ids[3],
            AnnulationEvenement {
                motif: "  ".into(),
                portee: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    ent.agenda_annuler_evenement(
        etab,
        prof.0,
        prof.1,
        ids[3],
        AnnulationEvenement {
            motif: "Vacances scolaires".into(),
            portee: Some("suivantes".into()),
        },
    )
    .await
    .unwrap();
    let statuts: Vec<String> = sqlx::query_scalar("SELECT statut FROM ent_agenda_evenement WHERE etablissement_id = $1 AND serie_id = $2 ORDER BY debut")
        .bind(etab)
        .bind(serie)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert!(statuts[3..].iter().all(|s| s == "annule"), "{statuts:?}");
    assert!(statuts[..3].iter().all(|s| s != "annule"), "{statuts:?}");
    assert!(corps_notifications(&ent, etab, cpe, "agenda_annule")
        .await
        .iter()
        .any(|a| a.contains("Vacances scolaires")));
    // Un événement annulé ne se modifie plus et n'accepte plus de réponse.
    let r = ent
        .agenda_repondre(
            etab,
            cpe,
            ids[3],
            ReponseInvitation {
                reponse: "oui".into(),
                motif_excuse: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    reset(&ent, etab).await;
}

// ─── Piège 5 / R-AG3 : le journal garde tout, en ajout seul ───────────────────────────────────

#[tokio::test]
async fn ag1_journal_en_ajout_seul_et_historique_reserve() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let prof = (cast.prof.user_id, PROF);
    let c = ent
        .agenda_creer_evenement(etab, prof.0, prof.1, evt("Conseil pédagogique", 9))
        .await
        .unwrap();
    ent.agenda_publier_evenement(
        etab,
        prof.0,
        prof.1,
        c.id,
        DemandePublication {
            audiences: vec![],
            empreinte: None,
            apercu_seulement: false,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap_err();
    let d = ent
        .agenda_evenement(etab, prof.0, prof.1, c.id)
        .await
        .unwrap();
    ent.agenda_annuler_evenement(
        etab,
        prof.0,
        prof.1,
        c.id,
        AnnulationEvenement {
            motif: "Salle indisponible".into(),
            portee: None,
        },
    )
    .await
    .unwrap();
    let h = ent
        .agenda_historique(etab, prof.0, prof.1, c.id)
        .await
        .unwrap();
    let actions: Vec<&str> = h.iter().map(|f| f.action.as_str()).collect();
    assert_eq!(actions, ["creer", "publier", "annuler"], "{d:?}");
    assert_eq!(h[2].apres.as_deref(), Some("Salle indisponible"));
    // La direction (agenda.journal.lire) lit aussi ; un collègue non : 404.
    assert_eq!(
        ent.agenda_historique(etab, adm, role, c.id)
            .await
            .unwrap()
            .len(),
        3
    );
    assert!(ent
        .agenda_historique(etab, cast.cpe.user_id, CPE, c.id)
        .await
        .is_err());
    // Ajout seul : ni modification ni suppression.
    assert!(
        sqlx::query("UPDATE ent_agenda_journal SET action = 'x' WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM ent_agenda_journal WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    reset(&ent, etab).await;
}

// ─── R-AG1 : vue calculée, bornes, sources absentes, minimisation ────────────────────────────

#[tokio::test]
async fn ag1_vue_agenda_bornee_sources_dites_et_partie_publique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    let (adm, role) = dir(&cast);
    let c4b = classe_4b(&ent, etab).await;
    let du = jour(0);
    let au = jour(30);
    // Plage bornée à 42 jours (ND-07 : jamais « tout »).
    let r = ent
        .agenda_vue(etab, prof.0, prof.1, du, du + Duration::days(60), &[], None)
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    assert!(ent
        .agenda_vue(etab, prof.0, prof.1, au, du, &[], None)
        .await
        .is_err());
    assert!(ent
        .agenda_vue(etab, prof.0, prof.1, du, au, &["inconnue".into()], None)
        .await
        .is_err());
    // Un événement public (partie publique), un réservé aux participants de 4B.
    let publique = ent
        .agenda_creer_evenement(
            etab,
            adm,
            role,
            NouvelEvenement {
                confidentialite: Some("etablissement".into()),
                ..evt("Portes ouvertes", 5)
            },
        )
        .await
        .unwrap();
    publier_a(&ent, &cast, (adm, role), publique.id, vec![]).await;
    let reserve = ent
        .agenda_creer_evenement(etab, adm, role, evt("Réunion parents 4B", 6))
        .await
        .unwrap();
    publier_a(&ent, &cast, (adm, role), reserve.id, vec![classe(c4b)]).await;
    // Un brouillon d'un collègue n'apparaît jamais.
    ent.agenda_creer_evenement(etab, adm, role, evt("Brouillon secret", 7))
        .await
        .unwrap();
    let titres = |v: &VueAgenda| {
        v.elements
            .iter()
            .filter(|e| e.source == "evenements")
            .map(|e| e.titre.clone())
            .collect::<Vec<_>>()
    };
    // Le professeur : la partie publique seulement (il n'est pas invité à la réunion de 4B).
    let v = ent
        .agenda_vue(etab, prof.0, prof.1, du, au, &[], None)
        .await
        .unwrap();
    assert_eq!(titres(&v), ["Portes ouvertes"]);
    assert!(
        v.elements
            .iter()
            .find(|e| e.titre == "Portes ouvertes")
            .unwrap()
            .role
            .as_deref()
            == Some("lecteur")
    );
    // La source « sorties » n'est pas branchée : elle est dite, pas tue (R-AG1).
    assert!(
        v.sources_indisponibles
            .iter()
            .any(|s| s.source == "sorties"),
        "{:?}",
        v.sources_indisponibles
    );
    // Le filtre par source.
    let seul = ent
        .agenda_vue(etab, prof.0, prof.1, du, au, &["evenements".into()], None)
        .await
        .unwrap();
    assert!(
        seul.elements.iter().all(|e| e.source == "evenements")
            && seul.sources_indisponibles.is_empty()
    );
    // Une famille de 4B : sa réunion avec l'enfant concerné ; l'agenda d'un enfant filtre.
    let lea = cast.parent_lea.user_id;
    let v = ent
        .agenda_vue(etab, lea, "parent", du, au, &["evenements".into()], None)
        .await
        .unwrap();
    let mien = v
        .elements
        .iter()
        .find(|e| e.titre == "Réunion parents 4B")
        .expect("réunion visible");
    assert_eq!(
        (mien.role.as_deref(), mien.reponse.as_deref(), mien.eleve_id),
        (Some("famille"), Some("en_attente"), Some(cast.eleve_lea_id))
    );
    let v = ent
        .agenda_vue(
            etab,
            lea,
            "parent",
            du,
            au,
            &["evenements".into()],
            Some(Uuid::new_v4()),
        )
        .await
        .unwrap();
    assert!(
        titres(&v).iter().all(|t| t != "Réunion parents 4B"),
        "enfant inconnu : rien"
    );
    // L'autre famille (5A) ne voit pas la réunion de 4B.
    let v = ent
        .agenda_vue(
            etab,
            cast.parent_tom.user_id,
            "parent",
            du,
            au,
            &["evenements".into()],
            None,
        )
        .await
        .unwrap();
    assert!(
        titres(&v).iter().all(|t| t != "Réunion parents 4B")
            && titres(&v).contains(&"Portes ouvertes".to_string())
    );
    // Agenda d'une classe (non nominatif) : réservé à `agenda.lire_structure` ; pas de brouillon.
    let vc = ent
        .agenda_classe(etab, prof.0, prof.1, c4b, du, au)
        .await
        .unwrap();
    assert!(
        vc.elements.iter().any(|e| e.titre == "Réunion parents 4B")
            && vc.elements.iter().all(|e| e.titre != "Brouillon secret")
    );
    assert!(vc.elements.iter().all(|e| e.eleve_id.is_none()));
    assert!(ent
        .agenda_classe(etab, lea, "parent", c4b, du, au)
        .await
        .is_err());
    reset(&ent, etab).await;
}

#[tokio::test]
async fn ag1_vue_agenda_rendez_vous_visibles_de_leurs_seules_parties() {
    use crate::rencontres::{NouveauxCreneaux, NouvelleRencontre};
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_4b(&ent, etab).await;
    crate::tests_agenda_ag0::donner_service(&ent, etab, cast.prof.user_id, c4b).await;
    let debut = (Utc::now() + Duration::days(4))
        .date_naive()
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc();
    let r = ent
        .creer_rencontre(
            etab,
            adm,
            role,
            NouvelleRencontre {
                titre: "AG1".into(),
                classe_id: c4b,
            },
        )
        .await
        .unwrap();
    ent.ajouter_creneaux(
        etab,
        adm,
        role,
        r.id,
        NouveauxCreneaux {
            enseignant_user_id: cast.prof.user_id,
            debut,
            duree_minutes: 10,
            nombre: 1,
        },
    )
    .await
    .unwrap();
    ent.ouvrir_rencontre(etab, adm, role, r.id).await.unwrap();
    let k: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_creneau_rdv WHERE etablissement_id = $1 AND rencontre_id = $2",
    )
    .bind(etab)
    .bind(r.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    ent.reserver_creneau(etab, cast.parent_lea.user_id, k, cast.eleve_lea_id)
        .await
        .unwrap();
    let (du, au) = (jour(0), jour(10));
    let src = ["rendez_vous".to_string()];
    // Le professeur voit le rendez-vous (avec le prénom de l'enfant, sans identifiant d'enfant).
    let v = ent
        .agenda_vue(etab, cast.prof.user_id, PROF, du, au, &src, None)
        .await
        .unwrap();
    assert_eq!(v.elements.len(), 1);
    assert!(
        v.elements[0].titre.starts_with("Rendez-vous parent") && v.elements[0].eleve_id.is_none()
    );
    // La famille voit son rendez-vous ; l'autre famille et un autre enseignant : rien.
    let v = ent
        .agenda_vue(etab, cast.parent_lea.user_id, "parent", du, au, &src, None)
        .await
        .unwrap();
    assert_eq!(
        (v.elements.len(), v.elements[0].eleve_id),
        (1, Some(cast.eleve_lea_id))
    );
    for (u, ro) in [(cast.parent_tom.user_id, "parent"), (cast.cpe.user_id, CPE)] {
        assert!(
            ent.agenda_vue(etab, u, ro, du, au, &src, None)
                .await
                .unwrap()
                .elements
                .is_empty(),
            "{ro}"
        );
    }
    reset(&ent, etab).await;
}

// ─── Fuseau de l'établissement (R-AG11) et iCal minimisé (R-AG25, piège 3) ─────────────────────

#[tokio::test]
async fn ag1_heures_dans_le_fuseau_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    sqlx::query("INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, 'Indian/Reunion') ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = 'Indian/Reunion'")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let c = ent
        .agenda_creer_evenement(
            etab,
            prof.0,
            prof.1,
            NouvelEvenement {
                heure_debut: h(9, 0),
                heure_fin: h(10, 30),
                ..evt("Réunion à La Réunion", 10)
            },
        )
        .await
        .unwrap();
    let d = ent
        .agenda_evenement(etab, prof.0, prof.1, c.id)
        .await
        .unwrap();
    // 09:00 à La Réunion (UTC+4) = 05:00 UTC.
    assert_eq!(d.debut.format("%H:%M").to_string(), "05:00");
    assert_eq!(d.fin - d.debut, Duration::minutes(90));
    // Journée entière : minuit local → minuit local du lendemain.
    let j = ent
        .agenda_creer_evenement(
            etab,
            prof.0,
            prof.1,
            NouvelEvenement {
                heure_debut: None,
                heure_fin: None,
                jour_fin: Some(jour(11)),
                ..evt("Voyage", 10)
            },
        )
        .await
        .unwrap();
    let dj = ent
        .agenda_evenement(etab, prof.0, prof.1, j.id)
        .await
        .unwrap();
    assert!(dj.journee_entiere);
    assert_eq!(dj.debut.format("%H:%M").to_string(), "20:00");
    assert_eq!(dj.fin - dj.debut, Duration::days(2));
    reset(&ent, etab).await;
}

#[tokio::test]
async fn ag1_ics_minimise_et_avis_sans_titre_pour_un_evenement_restreint() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    let cpe = cast.cpe.user_id;
    let c = ent
        .agenda_creer_evenement(
            etab,
            prof.0,
            prof.1,
            NouvelEvenement {
                confidentialite: Some("restreint".into()),
                invites: vec![cpe],
                description: Some("Situation de l'élève X".into()),
                ..evt("Équipe éducative — élève X", 6)
            },
        )
        .await
        .unwrap();
    ent.agenda_publier_evenement(
        etab,
        prof.0,
        prof.1,
        c.id,
        DemandePublication {
            audiences: vec![],
            empreinte: None,
            apercu_seulement: false,
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap_err();
    // R-AG7 : l'avis ne porte pas le titre.
    let avis = corps_notifications(&ent, etab, cpe, "agenda_invitation").await;
    assert!(avis.iter().all(|a| !a.contains("élève X")), "{avis:?}");
    // L'iCal de l'invité ne montre ni titre, ni description, ni participants.
    let ics = ent.agenda_ics(etab, cpe, CPE, c.id).await.unwrap();
    assert!(
        ics.contains("SUMMARY:Événement de l'établissement"),
        "{ics}"
    );
    for interdit in ["élève X", "Situation", "ATTENDEE", "ORGANIZER"] {
        assert!(!ics.contains(interdit), "{interdit}");
    }
    // Un non-invité n'obtient pas d'iCal (404) ; l'organisateur voit son titre.
    assert!(ent
        .agenda_ics(etab, cast.parent_lea.user_id, "parent", c.id)
        .await
        .is_err());
    assert!(ent
        .agenda_ics(etab, prof.0, prof.1, c.id)
        .await
        .unwrap()
        .contains("Équipe éducative"));
    reset(&ent, etab).await;
}

// ─── Types d'événements de l'établissement ───────────────────────────────────────────────────

#[tokio::test]
async fn ag1_types_locaux_reserves_a_l_administrateur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    assert!(matches!(
        ent.agenda_creer_type_local(etab, prof.0, prof.1, "atelier", "Atelier", "evenement")
            .await,
        Err(EntDbError::Forbidden)
    ));
    for (code, fam) in [
        ("reunion", "reunion"),
        ("Atelier", "evenement"),
        ("atelier", "inconnue"),
    ] {
        assert!(
            ent.agenda_creer_type_local(etab, cast.admin.user_id, "admin", code, "Atelier", fam)
                .await
                .is_err(),
            "{code}/{fam}"
        );
    }
    ent.agenda_creer_type_local(
        etab,
        cast.admin.user_id,
        "admin",
        "atelier",
        "Atelier",
        "evenement",
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.agenda_creer_type_local(
            etab,
            cast.admin.user_id,
            "admin",
            "atelier",
            "Autre",
            "evenement"
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent
        .agenda_types(etab)
        .await
        .unwrap()
        .iter()
        .any(|t| t.0 == "atelier"));
    ent.agenda_creer_evenement(
        etab,
        prof.0,
        prof.1,
        NouvelEvenement {
            type_code: "atelier".into(),
            ..evt("Atelier théâtre", 5)
        },
    )
    .await
    .unwrap();
    reset(&ent, etab).await;
}

// ─── Isolation par établissement (RLS, piège 6) ───────────────────────────────────────────────

#[tokio::test]
async fn ag1_aucune_lecture_d_un_autre_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let prof = (cast.prof.user_id, PROF);
    let c = ent
        .agenda_creer_evenement(etab, prof.0, prof.1, evt("Réunion", 5))
        .await
        .unwrap();
    let autre = Uuid::new_v4();
    assert!(ent
        .agenda_evenement(autre, prof.0, prof.1, c.id)
        .await
        .is_err());
    assert!(ent
        .agenda_historique(autre, prof.0, prof.1, c.id)
        .await
        .is_err());
    assert!(ent.agenda_ics(autre, prof.0, prof.1, c.id).await.is_err());
    let (du, au) = (jour(0), jour(10));
    assert!(ent
        .agenda_vue(autre, prof.0, prof.1, du, au, &["evenements".into()], None)
        .await
        .map(|v| v.elements.is_empty())
        .unwrap_or(true));
    // Le journal et les participants portent bien l'établissement.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_agenda_participant WHERE etablissement_id = $1 AND evenement_id = $2")
        .bind(etab)
        .bind(c.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
    // Aucune colonne JSON dans les tables de l'agenda (D-036).
    let json: Vec<String> = sqlx::query_scalar(
        "SELECT table_name || '.' || column_name FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name LIKE 'ent_agenda_%' AND data_type IN ('json', 'jsonb')",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(json.is_empty(), "{json:?}");
    let _ = (NaiveDate::MIN, DateTime::<Utc>::MIN_UTC, Role::Prof);
    reset(&ent, etab).await;
}
