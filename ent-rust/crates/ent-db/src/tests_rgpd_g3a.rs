//! Socle 27, **lot G3a** : guichet des demandes de droits (dépôt, qualification, identité et qualité,
//! horloge légale, décisions, clôture). Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).
//!
//! Chaque test fixe lui-même les dates qu'il compare (la date de réception d'une demande, la date de
//! référence d'un lien) : **aucun test ne dépend de l'heure à laquelle il tourne** (règle 33). Le seul jour
//! lu de l'établissement est `aujourdhui_etablissement`, qui est précisément l'horloge que le guichet
//! utilise ; les assertions portent sur des écarts, jamais sur une date absolue.

use super::*;
use crate::droits_demandes::{
    echeance_legale, prolongation_max, DepotConnecte, DepotSansCompte, SaisieDemande,
    ECART_SAISIE_ALERTE_JOURS, MESSAGE_SANS_COMPTE,
};
use crate::droits_repo::{Droit, RoleSaisie};
use chrono::Duration;
use ent_auth::{Action, Portee};
use uuid::Uuid;

const DIR: &str = "direction";
const ADF: &str = "administratif";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

/// Une demande d'accès déposée au guichet pour l'élève Léa, reçue le jour de l'établissement.
async fn demande_guichet(ent: &PgEnt, cast: &LaboCast) -> (Uuid, chrono::NaiveDate) {
    let etab = cast.etablissement_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            cast.direction.user_id,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: Some("courrier postal"),
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: Some("tout le dossier scolaire"),
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("saisie au guichet");
    (v.id, jour)
}

// ─────────────────────────────── Numéro et horloge ───────────────────────────────

/// Le numéro est propre à l'établissement, suit l'année de **réception** et n'a pas de trou visible : une
/// transaction annulée ne consomme pas de numéro (le compteur est incrémenté dans la transaction du dépôt).
#[tokio::test]
async fn g3a_numero_par_etablissement_sans_trou_visible() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let (id1, _) = demande_guichet(&ent, &cast).await;
    let v1 = ent.droits_demande(etab, dir, DIR, id1).await.unwrap();
    // Une saisie refusée (date future) n'avance pas le compteur.
    let refus = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour + Duration::days(1),
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Demain"),
                demandeur_contact: None,
                qualite: "soi_meme",
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some("Inconnu"),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await;
    assert!(
        matches!(refus, Err(EntDbError::SaisieInvalide(_))),
        "une date de réception future est refusée"
    );
    let (id2, _) = demande_guichet(&ent, &cast).await;
    let v2 = ent.droits_demande(etab, dir, DIR, id2).await.unwrap();
    let n = |r: &str| r.rsplit_once('-').unwrap().1.parse::<i32>().unwrap();
    assert_eq!(
        n(&v2.reference),
        n(&v1.reference) + 1,
        "numéros consécutifs : {} puis {}",
        v1.reference,
        v2.reference
    );
    assert!(
        v1.reference.starts_with(&jour.format("%Y").to_string()),
        "la référence porte l'année de réception : {}",
        v1.reference
    );
}

/// L'échéance est **un mois** après la réception, au jour de l'établissement ; la prolongation ajoute deux
/// mois au plus, une seule fois, et seulement avant l'échéance du premier mois (art. 12.3).
#[tokio::test]
async fn g3a_horloge_un_mois_puis_deux_au_plus() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (id, jour) = demande_guichet(&ent, &cast).await;
    let v = ent.droits_demande(etab, dir, DIR, id).await.unwrap();
    assert_eq!(v.recue_le, jour);
    assert_eq!(v.echeance_le, echeance_legale(jour));
    assert_eq!(v.echeance_effective, v.echeance_le);
    assert!(!v.en_retard);
    assert!(!v.saisie_tardive);
    let max = prolongation_max(v.echeance_le);
    // Au-delà de deux mois : refusé.
    assert!(
        matches!(
            ent.droits_prolonger(
                etab,
                dir,
                DIR,
                id,
                max + Duration::days(1),
                "dossier volumineux"
            )
            .await,
            Err(EntDbError::SaisieInvalide(_))
        ),
        "deux mois au plus"
    );
    // Sans motif : refusé.
    assert!(matches!(
        ent.droits_prolonger(etab, dir, DIR, id, max, "  ").await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // En deçà de l'échéance : refusé (une « prolongation » qui raccourcit n'existe pas).
    assert!(matches!(
        ent.droits_prolonger(etab, dir, DIR, id, v.echeance_le, "x pour test")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v = ent
        .droits_prolonger(
            etab,
            dir,
            DIR,
            id,
            max,
            "dossier volumineux, plusieurs services",
        )
        .await
        .expect("prolongation");
    assert_eq!(v.prolongee_jusqu_au, Some(max));
    assert_eq!(v.echeance_effective, max);
    // Une seconde prolongation est refusée.
    assert!(matches!(
        ent.droits_prolonger(etab, dir, DIR, id, max, "encore un délai")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Le demandeur n'a pas de compte ici : rien n'a été notifié dans l'ENT (R-RG42).
    let avis: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification
          WHERE source_event_id = $1 AND kind = 'rgpd.demande_prolongee'",
    )
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(avis, 0, "aucun avis à une adresse fournie par la demande");
    let j: Vec<String> = v.journal.iter().map(|l| l.evenement.clone()).collect();
    assert!(j.contains(&"prolongee".to_string()), "{j:?}");
}

/// RGP-303 : une demande reçue bien avant sa saisie est signalée (relève du courrier, CL27-64). La date de
/// réception, l'année, le canal et la date de saisie sont **figés** par la base.
#[tokio::test]
async fn g3a_saisie_tardive_signalee_et_dates_figees() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let recue = jour - Duration::days(ECART_SAISIE_ALERTE_JOURS + 9);
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: recue,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Courrier de l'été"),
                demandeur_contact: None,
                qualite: "ancien_eleve",
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some("Ancien élève"),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("saisie");
    assert!(v.saisie_tardive, "écart de saisie signalé");
    assert_eq!(v.recue_le, recue);
    assert_eq!(
        v.echeance_le,
        echeance_legale(recue),
        "l'horloge part de la réception, pas de la saisie"
    );
    assert!(
        v.jours_restants < 0 || v.jours_restants <= 30 - ECART_SAISIE_ALERTE_JOURS,
        "l'échéance a déjà consommé le délai de relève ({} jours restants)",
        v.jours_restants
    );
    let detail = v
        .journal
        .iter()
        .find(|l| l.evenement == "recue")
        .and_then(|l| l.detail.clone())
        .unwrap_or_default();
    assert!(detail.contains("jours après la réception"), "{detail}");
    // Les colonnes figées refusent toute réécriture (garde de 2544).
    for (col, val) in [("recue_le", "'2000-01-01'::date")] {
        let sql = format!("UPDATE rgp_demande SET {col} = {val} WHERE id = $1");
        let e = sqlx::query(&sql).bind(v.id).execute(ent.pool()).await;
        assert!(e.is_err(), "{col} doit être figé");
    }
    let e = sqlx::query("UPDATE rgp_demande SET numero = numero + 1000 WHERE id = $1")
        .bind(v.id)
        .execute(ent.pool())
        .await;
    assert!(e.is_err(), "le numéro est figé");
    let e = sqlx::query("DELETE FROM rgp_demande WHERE id = $1")
        .bind(v.id)
        .execute(ent.pool())
        .await;
    assert!(e.is_err(), "une demande ne se supprime pas");
}

/// Une demande reçue il y a plus de deux ans est une erreur de saisie, pas un courrier retrouvé.
#[tokio::test]
async fn g3a_reception_trop_ancienne_refusee() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let r = ent
        .droits_saisir_au_guichet(
            etab,
            cast.direction.user_id,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour - Duration::days(1000),
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Très vieux courrier"),
                demandeur_contact: None,
                qualite: "ancien_eleve",
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some("Ancien élève"),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
}

// ─────────────────────────── Dépôt connecté, qualité à la date ───────────────────────────

/// RGP-140 : un parent dépose pour son enfant ; sa qualité est constatée par le socle 4 **à la date**, un
/// accusé de réception part (sans nom d'élève, E14), et il suit sa demande sans voir le détail interne.
#[tokio::test]
async fn g3a_depot_connecte_accuse_sans_nom_d_eleve() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let parent = cast.parent_lea.user_id;
    let v = ent
        .droits_deposer_connecte(
            etab,
            parent,
            "parent",
            DepotConnecte {
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                droits: &["acces", "consultations"],
                perimetre_texte: Some("l'année en cours"),
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("dépôt connecté");
    assert_eq!(v.etat, "recue");
    assert!(v.demandeur_a_un_compte);
    assert_eq!(v.droits.len(), 2);
    assert!(v.droits.iter().all(|d| d.decision == "en_examen"));
    // L'identité est acquise par la session ; la qualité est constatée.
    assert_eq!(v.identites.len(), 1);
    assert_eq!(v.identites[0].mode, "session_forte");
    assert_eq!(v.identites[0].resultat, "verifiee");
    assert!(matches!(
        v.identites[0].qualite_constatee.as_deref(),
        Some("confirmee" | "confirmee_sans_autorite" | "restreinte")
    ));
    // Accusé de réception : date, échéance, et aucun nom d'élève.
    let avis: Vec<(String, String)> = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification
          WHERE source_event_id = $1 AND kind = 'rgpd.demande_accusee' AND destinataire_user_id = $2",
    )
    .bind(v.id)
    .bind(parent)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(avis.len(), 1, "un accusé de réception");
    let corps = &avis[0].1;
    assert!(corps.contains(&v.reference), "{corps}");
    assert!(
        corps.contains(&v.echeance_le.format("%d/%m/%Y").to_string()),
        "{corps}"
    );
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(
        !corps.contains(&nom_eleve),
        "aucun nom d'élève dans la notification : {corps}"
    );
    // Suivi côté demandeur : phrase lisible, aucune donnée interne.
    let suivi = ent.droits_mes_demandes(etab, parent).await.unwrap();
    let s = suivi.iter().find(|s| s.id == v.id).expect("ma demande");
    assert!(s.avancement.contains("Reçue le"), "{}", s.avancement);
    assert_eq!(s.droits.len(), 2);
    // Un autre parent ne voit pas cette demande dans son suivi.
    let autre = ent
        .droits_mes_demandes(etab, cast.parent_tom.user_id)
        .await
        .unwrap();
    assert!(
        !autre.iter().any(|s| s.id == v.id),
        "le suivi est filtré sur le compte"
    );
}

/// §9.3 et R-RG38 : un dépôt en ligne ne sert pas à réclamer les données d'un tiers ni celles de l'autre
/// parent ; un parent sans lien à la date est refusé ; un élève sujet d'un autre établissement est
/// introuvable (404 neutre, jamais « refusé »).
#[tokio::test]
async fn g3a_depot_connecte_qualites_refusees() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let depot = |q: &'static str, sujet: Option<Uuid>, t: &'static str| DepotConnecte {
        qualite: q,
        sujet_type: t,
        sujet_id: sujet,
        droits: &["acces"],
        perimetre_texte: None,
        perimetre_du: None,
        perimetre_au: None,
    };
    // Qualité « tiers » et « parent sans autorité » : au guichet, avec les pièces.
    for q in ["tiers", "parent_sans_autorite"] {
        let r = ent
            .droits_deposer_connecte(
                etab,
                cast.parent_lea.user_id,
                "parent",
                depot(q, Some(cast.eleve_lea_id), "eleve"),
            )
            .await;
        assert!(
            matches!(r, Err(EntDbError::SaisieInvalide(_))),
            "{q} doit passer par le guichet"
        );
    }
    // Un parent qui invoque l'autorité sur l'élève d'une autre famille : aucun lien à la date → refus.
    let r = ent
        .droits_deposer_connecte(
            etab,
            cast.parent_lea.user_id,
            "parent",
            depot("titulaire_autorite", Some(cast.eleve_tom_id), "eleve"),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "aucun lien à la date : refus"
    );
    // Sujet inexistant dans cet établissement : introuvable (rien ne dit qu'il existe ailleurs).
    let r = ent
        .droits_deposer_connecte(
            etab,
            cast.parent_lea.user_id,
            "parent",
            depot("titulaire_autorite", Some(Uuid::new_v4()), "eleve"),
        )
        .await;
    assert!(matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Droit inconnu, et liste vide.
    let r = ent
        .droits_deposer_connecte(
            etab,
            cast.parent_lea.user_id,
            "parent",
            DepotConnecte {
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                droits: &[],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
}

/// §9.3 — une qualité « pour moi-même » ne vaut que **sur soi** : le sujet est résolu depuis le compte
/// quand il n'est pas donné, et refusé quand il désigne quelqu'un d'autre. Sans ce contrôle, un compte
/// ouvrirait un dossier sur la personne de son choix et recevrait son extrait au lot G3b.
#[tokio::test]
async fn g3a_qualite_pour_soi_ne_vaut_que_sur_soi() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let parent = cast.parent_lea.user_id;
    let depot = |sujet: Option<Uuid>, t: &'static str| DepotConnecte {
        qualite: "soi_meme",
        sujet_type: t,
        sujet_id: sujet,
        droits: &["acces"],
        perimetre_texte: None,
        perimetre_du: None,
        perimetre_au: None,
    };
    // Sans sujet : le serveur résout la fiche du compte (l'usager ne connaît pas son identifiant interne).
    let v = ent
        .droits_deposer_connecte(etab, parent, "parent", depot(None, "personne"))
        .await
        .expect("dépôt pour soi, sujet résolu");
    assert_eq!(v.sujet_type, "personne");
    let ma_personne: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(parent)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(v.sujet_id, Some(ma_personne), "le sujet est bien le compte");
    // Avec le sujet d'une autre personne : refusé.
    let autre: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_personne
          WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(cast.parent_tom.user_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(
        matches!(
            ent.droits_deposer_connecte(etab, parent, "parent", depot(Some(autre), "personne"))
                .await,
            Err(EntDbError::Forbidden)
        ),
        "« pour moi-même » sur la fiche d'un autre : refusé"
    );
    // Et sur un élève qui n'est pas le compte : refusé aussi.
    assert!(matches!(
        ent.droits_deposer_connecte(
            etab,
            parent,
            "parent",
            depot(Some(cast.eleve_lea_id), "eleve")
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Un élève qui invoque « élève mineur » sur un autre élève : refusé.
    let eleve = cast
        .eleves
        .iter()
        .find(|e| e.compte.is_some())
        .expect("un élève avec un compte");
    let compte = eleve.compte.as_ref().unwrap().user_id;
    let autre_eleve = cast
        .eleves
        .iter()
        .find(|e| e.id != eleve.id)
        .expect("un second élève");
    assert!(
        matches!(
            ent.droits_deposer_connecte(
                etab,
                compte,
                "eleve",
                DepotConnecte {
                    qualite: "eleve_mineur",
                    sujet_type: "eleve",
                    sujet_id: Some(autre_eleve.id),
                    droits: &["acces"],
                    perimetre_texte: None,
                    perimetre_du: None,
                    perimetre_au: None,
                }
            )
            .await,
            Err(EntDbError::Forbidden)
        ),
        "un élève ne demande pas les données d'un autre élève"
    );
    // Un élève qui ne donne pas de sujet obtient son propre dossier.
    let v = ent
        .droits_deposer_connecte(
            etab,
            compte,
            "eleve",
            DepotConnecte {
                qualite: "eleve_mineur",
                sujet_type: "personne",
                sujet_id: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("dépôt de l'élève pour lui-même");
    assert_eq!(v.sujet_type, "eleve");
    assert_eq!(v.sujet_id, Some(eleve.id));
    // Les qualités qui exigent une pièce restent au guichet (CL27-07, CL27-15).
    for q in ["mandataire", "representant_majeur_protege"] {
        let r = ent
            .droits_deposer_connecte(
                etab,
                parent,
                "parent",
                DepotConnecte {
                    qualite: q,
                    sujet_type: "eleve",
                    sujet_id: Some(cast.eleve_lea_id),
                    droits: &["acces"],
                    perimetre_texte: None,
                    perimetre_du: None,
                    perimetre_au: None,
                },
            )
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{q}");
    }
}

/// CL27-04 : l'élève mineur dépose lui-même. L'information de ses titulaires est la règle (S-550) ; ne pas
/// les informer exige un **motif sérieux écrit** et relève de la direction (`droits.arbitrer`).
#[tokio::test]
async fn g3a_eleve_mineur_depose_et_information_des_titulaires() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    // Un élève du labo avec son propre compte.
    let eleve = cast
        .eleves
        .iter()
        .find(|e| e.compte.is_some())
        .expect("un élève avec un compte");
    let compte = eleve.compte.as_ref().unwrap().user_id;
    let v = ent
        .droits_deposer_connecte(
            etab,
            compte,
            "eleve",
            DepotConnecte {
                qualite: "eleve_mineur",
                sujet_type: "eleve",
                sujet_id: Some(eleve.id),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("dépôt de l'élève pour lui-même");
    assert_eq!(v.qualite, "eleve_mineur");
    assert!(v.titulaires_informes_le.is_none());
    assert!(v.titulaires_non_informes_motif.is_none());
    // Ne pas informer sans motif : refusé.
    assert!(matches!(
        ent.droits_titulaires_information(etab, dir, DIR, v.id, false, None)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Le secrétariat peut informer, mais pas décider de ne pas informer.
    let adf = cast.administratif.user_id;
    assert!(matches!(
        ent.droits_titulaires_information(etab, adf, ADF, v.id, false, Some("sécurité de l'élève"))
            .await,
        Err(EntDbError::Forbidden)
    ));
    let w = ent
        .droits_titulaires_information(etab, adf, ADF, v.id, true, None)
        .await
        .expect("information des titulaires");
    assert!(w.titulaires_informes_le.is_some());
    // La direction peut revenir sur la décision, avec motif.
    let w = ent
        .droits_titulaires_information(
            etab,
            dir,
            DIR,
            v.id,
            false,
            Some("motif sérieux : sécurité de l'élève, décision de la direction"),
        )
        .await
        .expect("non-information motivée");
    assert!(w.titulaires_informes_le.is_none());
    assert!(w.titulaires_non_informes_motif.is_some());
    // La même décision sur une demande qui n'est pas celle d'un élève mineur n'a pas de sens.
    let (autre, _) = demande_guichet(&ent, &cast).await;
    assert!(matches!(
        ent.droits_titulaires_information(etab, dir, DIR, autre, true, None)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

// ─────────────────────────────── Dépôt sans compte ───────────────────────────────

/// RGP-142 et R-RG42 : le dépôt sans compte rend **toujours** le même message, ne confirme aucune
/// scolarisation, ne rapproche aucun dossier et n'envoie aucun avis à l'adresse fournie. La demande naît en
/// `identite_a_verifier` et son horloge court dès la réception.
#[tokio::test]
async fn g3a_depot_sans_compte_ne_confirme_rien() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    // Une personne connue et une personne inconnue reçoivent le MÊME message.
    let connu = ent
        .droits_deposer_sans_compte(
            etab,
            DepotSansCompte {
                nom_saisi: "Parent sans compte",
                contact: "parent.sans.compte@exemple.test",
                qualite: "titulaire_autorite",
                sujet_nom_saisi: &nom_eleve,
                droits: &["acces"],
                perimetre_texte: Some("le dossier de mon enfant"),
            },
        )
        .await
        .expect("dépôt sans compte (personne connue)");
    let inconnu = ent
        .droits_deposer_sans_compte(
            etab,
            DepotSansCompte {
                nom_saisi: "Quelqu'un d'autre",
                contact: "quelquun@exemple.test",
                qualite: "ancien_eleve",
                sujet_nom_saisi: "Personne Introuvable",
                droits: &["acces"],
                perimetre_texte: None,
            },
        )
        .await
        .expect("dépôt sans compte (personne inconnue)");
    assert_eq!(connu, MESSAGE_SANS_COMPTE);
    assert_eq!(inconnu, MESSAGE_SANS_COMPTE, "même réponse : aucune fuite");
    // Côté guichet : les demandes existent, en `identite_a_verifier`, sans sujet rapproché.
    let t = ent.droits_tableau(etab, dir, DIR, false).await.unwrap();
    let sans_compte: Vec<_> = t
        .lignes
        .iter()
        .filter(|l| l.etat == "identite_a_verifier" && l.sujet_type == "inconnu")
        .collect();
    assert!(
        sans_compte.len() >= 2,
        "les deux dépôts sont dans la file : {}",
        sans_compte.len()
    );
    let une = sans_compte[0];
    assert_eq!(une.recue_le, jour);
    assert_eq!(une.echeance_effective, echeance_legale(jour));
    // Aucun avis, aucun envoi : rien ne part vers une adresse fournie par la demande (R-RG42).
    for l in &sans_compte {
        let avis: i64 =
            sqlx::query_scalar("SELECT count(*) FROM ent_notification WHERE source_event_id = $1")
                .bind(l.id)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        assert_eq!(avis, 0, "aucun avis sur un dépôt sans compte");
        let envois: i64 =
            sqlx::query_scalar("SELECT count(*) FROM ent_notification_envoi WHERE source_id = $1")
                .bind(l.id)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        assert_eq!(envois, 0, "aucun envoi sur un dépôt sans compte");
    }
    // Une qualité que le formulaire public n'accepte pas est refusée.
    assert!(matches!(
        ent.droits_deposer_sans_compte(
            etab,
            DepotSansCompte {
                nom_saisi: "Un avocat",
                contact: "cabinet@exemple.test",
                qualite: "mandataire",
                sujet_nom_saisi: "Un client",
                droits: &["acces"],
                perimetre_texte: None,
            },
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ───────────────────── Identité, qualité à la date, pièce détruite ─────────────────────

/// RGP-144 et R-RG16 : la vérification est tracée avec la qualité constatée **à la date de réception** ; la
/// pièce consultée est détruite à la clôture, et seule la mention « vérifiée le, par » subsiste.
#[tokio::test]
async fn g3a_identite_tracee_et_piece_detruite_a_la_cloture() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    // Mode « pièce » sans référence de pièce : refusé.
    assert!(matches!(
        ent.droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, None)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let piece = Uuid::new_v4();
    let v = ent
        .droits_verifier_identite(
            etab,
            dir,
            DIR,
            id,
            "piece",
            true,
            Some("pièce consultée au guichet"),
            Some(piece),
        )
        .await
        .expect("vérification");
    assert_eq!(v.etat, "en_cours", "l'identité établie ouvre l'instruction");
    assert!(v.piece_a_detruire);
    assert!(v.identites.iter().any(|i| i.piece_detenue));
    // La pièce est bien référencée en base avant la clôture.
    let avant: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande_identite WHERE demande_id = $1 AND piece_document_id IS NOT NULL",
    )
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(avant, 1);
    // Clôture impossible tant qu'un droit reste en examen.
    assert!(matches!(
        ent.droits_clore(etab, dir, DIR, id, "réponse remise").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.droits_decider(etab, dir, DIR, id, "acces", "accorde", Some("art15"), None)
        .await
        .expect("décision");
    let v = ent
        .droits_clore(
            etab,
            dir,
            DIR,
            id,
            "extrait remis en main propre contre émargement",
        )
        .await
        .expect("clôture");
    assert_eq!(v.etat, "close");
    assert!(!v.piece_a_detruire, "la pièce a disparu (R-RG16)");
    let apres: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande_identite WHERE demande_id = $1 AND piece_document_id IS NOT NULL",
    )
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(apres, 0, "plus aucune référence de pièce");
    let mention = v
        .identites
        .iter()
        .find(|i| i.mode == "piece")
        .expect("mention conservée");
    assert!(mention.verifie_le.is_some(), "« vérifiée le » subsiste");
    assert!(mention.piece_detruite_le.is_some());
    let j: Vec<String> = v.journal.iter().map(|l| l.evenement.clone()).collect();
    assert!(j.contains(&"piece_detruite".to_string()), "{j:?}");
    assert!(j.contains(&"close".to_string()), "{j:?}");
    // Une demande close ne se modifie plus : il faut la rouvrir.
    assert!(matches!(
        ent.droits_qualifier(etab, dir, DIR, id, "rgpd", "requalification")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

/// §9.2 : la précision d'identité demandée est **comptée** et n'avance pas l'échéance (choix prudent).
#[tokio::test]
async fn g3a_precision_identite_comptee_sans_repousser_lecheance() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    let avant = ent.droits_demande(etab, dir, DIR, id).await.unwrap();
    let v = ent
        .droits_demander_identite(etab, dir, DIR, id, "une pièce d'identité du demandeur")
        .await
        .expect("précision demandée");
    assert_eq!(v.etat, "identite_a_verifier");
    assert!(v.identite_demandee_le.is_some());
    assert_eq!(
        v.echeance_effective, avant.echeance_effective,
        "l'échéance ne bouge pas : le délai court (§9.2)"
    );
    let v = ent
        .droits_verifier_identite(etab, dir, DIR, id, "code_courriel", true, None, None)
        .await
        .unwrap();
    assert!(v.identite_fournie_le.is_some());
    assert_eq!(v.identite_attente_jours, 0, "répondu le même jour");
    assert_eq!(v.echeance_effective, avant.echeance_effective);
}

/// Un échec de vérification ne clôt pas la demande : la direction décide du refus motivé (R-RG17).
#[tokio::test]
async fn g3a_echec_identite_puis_refus_motive_de_la_direction() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    let v = ent
        .droits_verifier_identite(
            etab,
            adf,
            ADF,
            id,
            "code_courriel",
            false,
            Some("le courriel du dossier ne répond pas"),
            None,
        )
        .await
        .expect("échec tracé");
    assert_eq!(v.etat, "identite_a_verifier", "l'échec ne clôt rien");
    assert_eq!(v.identites[0].resultat, "echouee");
    // Le secrétariat ne refuse pas : c'est un arbitrage de la direction.
    assert!(matches!(
        ent.droits_refuser(etab, adf, ADF, id, "identité non établie")
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.droits_refuser(etab, dir, DIR, id, " ").await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v = ent
        .droits_refuser(
            etab,
            dir,
            DIR,
            id,
            "identité du demandeur non établie malgré deux relances (art. 12.6)",
        )
        .await
        .expect("refus motivé");
    assert_eq!(v.etat, "refusee");
    assert!(v.refus_motif.is_some());
    // Aucun droit ne reste muet : le refus les motive tous.
    assert!(v
        .droits
        .iter()
        .all(|d| d.decision == "refuse" && d.motif_texte.is_some()));
}

// ─────────────────────────── Qualification et décisions ───────────────────────────

/// RGP-141 : la qualification est motivée, modifiable et tracée. RGP-157/162 : une décision de refus ou de
/// partiel exige un motif par article et relève de l'arbitrage.
#[tokio::test]
async fn g3a_qualification_motivee_et_decisions_par_droit() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    assert!(
        matches!(
            ent.droits_qualifier(etab, adf, ADF, id, "rgpd", "x").await,
            Err(EntDbError::SaisieInvalide(_))
        ),
        "motif trop court"
    );
    assert!(matches!(
        ent.droits_qualifier(etab, adf, ADF, id, "inventé", "un motif correct")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v = ent
        .droits_qualifier(
            etab,
            adf,
            ADF,
            id,
            "rgpd",
            "demande d'accès aux données de l'élève par un titulaire de l'autorité",
        )
        .await
        .expect("qualification");
    assert_eq!(v.qualification, "rgpd");
    assert!(v.qualification_motif.is_some());
    // L'identité n'était pas établie : la qualification ne saute pas cette étape.
    assert_eq!(v.etat, "identite_a_verifier");
    // Requalification en CADA (§4.4, RGP-316) : tracée, l'historique garde les deux.
    let v = ent
        .droits_qualifier(
            etab,
            dir,
            DIR,
            id,
            "cada",
            "la demande porte en réalité sur un document administratif (CRPA L311-6)",
        )
        .await
        .expect("requalification");
    assert_eq!(v.qualification, "cada");
    assert_eq!(
        v.journal
            .iter()
            .filter(|l| l.evenement == "qualifiee")
            .count(),
        2,
        "les deux qualifications sont au journal"
    );
    // Ajout d'un droit, puis décisions.
    let v = ent
        .droits_ajouter_droit(etab, adf, ADF, id, "effacement")
        .await
        .expect("droit ajouté");
    assert_eq!(v.droits.len(), 2);
    assert!(matches!(
        ent.droits_ajouter_droit(etab, adf, ADF, id, "effacement")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Accorder : `droits.traiter` suffit.
    let v = ent
        .droits_decider(etab, adf, ADF, id, "acces", "accorde", Some("art15"), None)
        .await
        .expect("accès accordé");
    assert_eq!(
        v.droits
            .iter()
            .find(|d| d.droit == "acces")
            .unwrap()
            .decision,
        "accorde"
    );
    // Refuser : réservé à l'arbitrage, et motivé par article ET texte.
    assert!(matches!(
        ent.droits_decider(
            etab,
            adf,
            ADF,
            id,
            "effacement",
            "refuse",
            Some("art17_3_b"),
            Some("obligation légale")
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.droits_decider(
            etab,
            dir,
            DIR,
            id,
            "effacement",
            "refuse",
            Some("art17_3_b"),
            None
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v = ent
        .droits_decider(
            etab,
            dir,
            DIR,
            id,
            "effacement",
            "refuse",
            Some("art17_3_b"),
            Some("conservation imposée par le Code de l'éducation et versement aux archives"),
        )
        .await
        .expect("refus motivé");
    let e = v.droits.iter().find(|d| d.droit == "effacement").unwrap();
    assert_eq!(e.decision, "refuse");
    assert_eq!(e.motif_code.as_deref(), Some("art17_3_b"));
    assert!(e.motif_texte.is_some());
    // Un droit qui n'a pas été demandé ne se décide pas.
    assert!(matches!(
        ent.droits_decider(etab, dir, DIR, id, "portabilite", "sans_objet", None, None)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

/// RGP-166 et RGP-167 : la clôture lève le gel (R-RG19), la réouverture ne remet pas l'horloge à zéro.
#[tokio::test]
async fn g3a_cloture_leve_le_gel_et_reouverture_garde_l_horloge() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    // Une demande ouverte gèle les sorts finaux de la personne (lue par le moteur du lot G6).
    let gelees = ent
        .demandes_ouvertes_pour_sujet(etab, "eleve", cast.eleve_lea_id)
        .await
        .unwrap();
    assert!(gelees.contains(&id), "la demande ouverte gèle le sujet");
    ent.droits_decider(etab, dir, DIR, id, "acces", "accorde", None, None)
        .await
        .unwrap();
    let v = ent
        .droits_clore(etab, dir, DIR, id, "extrait remis, masquages relus")
        .await
        .unwrap();
    let echeance = v.echeance_effective;
    let gelees = ent
        .demandes_ouvertes_pour_sujet(etab, "eleve", cast.eleve_lea_id)
        .await
        .unwrap();
    assert!(!gelees.contains(&id), "la clôture lève le gel");
    // Réouverture : arbitrage, motif, horloge conservée.
    assert!(matches!(
        ent.droits_rouvrir(etab, cast.administratif.user_id, ADF, id, "réclamation")
            .await,
        Err(EntDbError::Forbidden)
    ));
    let v = ent
        .droits_rouvrir(
            etab,
            dir,
            DIR,
            id,
            "réclamation du demandeur : extrait incomplet",
        )
        .await
        .expect("réouverture");
    assert_eq!(v.etat, "en_cours");
    assert_eq!(v.reouvertures, 1);
    assert_eq!(
        v.echeance_effective, echeance,
        "la réouverture ne réécrit pas le délai"
    );
    assert!(v.close_le.is_none());
    // Une demande ouverte ne se rouvre pas.
    assert!(matches!(
        ent.droits_rouvrir(etab, dir, DIR, id, "encore une réclamation")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Le gel est de nouveau posé.
    let gelees = ent
        .demandes_ouvertes_pour_sujet(etab, "eleve", cast.eleve_lea_id)
        .await
        .unwrap();
    assert!(gelees.contains(&id));
}

// ─────────────────────────── Droits, cloisonnement, minimisation ───────────────────────────

/// §12.1 : aucun profil hors direction et secrétariat n'approche le guichet ; la remise et les arbitrages
/// restent à la direction. R-RG32 : le tableau ne porte que des métadonnées.
#[tokio::test]
async fn g3a_droits_et_minimisation_du_tableau() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    for (u, role) in [
        (cast.admin.user_id, "admin"),
        (cast.prof.user_id, "prof"),
        (cast.cpe.user_id, "cpe"),
        (cast.compta.user_id, "compta"),
        (cast.parent_lea.user_id, "parent"),
    ] {
        assert!(
            matches!(
                ent.droits_tableau(etab, u, role, false).await,
                Err(EntDbError::Forbidden)
            ),
            "{role} ne voit pas le tableau"
        );
        assert!(
            matches!(
                ent.droits_demande(etab, u, role, id).await,
                Err(EntDbError::Forbidden)
            ),
            "{role} ne voit pas la fiche"
        );
        assert!(
            matches!(
                ent.droits_referentiel(etab, u, role).await,
                Err(EntDbError::Forbidden)
            ),
            "{role} ne lit pas la grille d'identité"
        );
    }
    // Le secrétariat voit et instruit, mais n'arbitre pas.
    let adf = cast.administratif.user_id;
    let t = ent.droits_tableau(etab, adf, ADF, false).await.unwrap();
    assert!(t.lignes.iter().any(|l| l.id == id));
    assert!(t.ouvertes >= 1);
    let (grille, modeles) = ent.droits_referentiel(etab, adf, ADF).await.unwrap();
    assert!(!grille.is_empty() && !modeles.is_empty());
    // Le tableau est fait de métadonnées : aucun nom de personne concernée, aucun contenu de réponse.
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let json = serde_json::to_string(&t).unwrap();
    assert!(
        !json.contains(&nom_eleve),
        "aucun nom d'élève dans le tableau (R-RG32)"
    );
    assert!(
        !json.contains("dossier scolaire"),
        "aucun texte de périmètre"
    );
    // Les compteurs sont cohérents.
    assert_eq!(
        t.ouvertes,
        t.lignes
            .iter()
            .filter(|l| l.etat != "close" && l.etat != "refusee")
            .count()
    );
    // La direction voit tout.
    assert!(ent.droits_demande(etab, dir, DIR, id).await.is_ok());
}

/// Un rôle maison porteur de `droits.traiter` ouvre le guichet : le contrôle passe par le moteur de droits,
/// pas par une liste de rôles figée (D-017).
#[tokio::test]
async fn g3a_role_maison_porteur_du_droit() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let accueil = cast
        .administratifs
        .iter()
        .find(|p| p.user_id != cast.administratif.user_id)
        .unwrap_or(&cast.administratif);
    let role_id = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("Guichet RGPD {}", Uuid::new_v4()),
                description: Some("instruit les demandes de droits".into()),
                modele: None,
                droits: vec![Droit {
                    action: Action::DroitsTraiter,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .expect("rôle maison")
        .id;
    ent.attribuer_role_for_actor(
        etab,
        cast.admin.user_id,
        "admin",
        role_id,
        accueil.user_id,
        None,
    )
    .await
    .expect("attribution");
    let t = ent
        .droits_tableau(etab, accueil.user_id, accueil.role.as_str(), false)
        .await;
    assert!(
        t.is_ok(),
        "un rôle maison porteur du droit ouvre le guichet"
    );
    // Il n'arbitre pas pour autant.
    let (id, _) = demande_guichet(&ent, &cast).await;
    assert!(matches!(
        ent.droits_refuser(
            etab,
            accueil.user_id,
            accueil.role.as_str(),
            id,
            "refus pour test"
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
}

/// Cloisonnement entre établissements : une demande d'un autre établissement est **introuvable**, jamais
/// « refusée » (404 neutre) ; le tableau ne la montre pas.
#[tokio::test]
async fn g3a_cloisonnement_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let dir = cast.direction.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    // Un établissement voisin, avec sa propre direction (compte créé pour lui seul).
    let autre = Uuid::new_v4();
    let email = format!("dir-voisin-{}@ent.local", Uuid::new_v4().simple());
    let dir2 = ent
        .register_user(
            &email,
            "MotDePasseLabo!2027",
            Some("Direction voisine"),
            autre,
            "direction",
        )
        .await
        .expect("compte de direction voisine")
        .id;
    // L'établissement vient de la session, jamais de la requête : une direction placée sur un autre
    // établissement n'y voit **aucune** demande (la portée des droits est l'établissement de la session,
    // et la RLS filtre les lignes). On vérifie donc l'invisibilité, pas un refus de droit.
    let vide = ent.droits_tableau(autre, dir, DIR, true).await.unwrap();
    assert!(
        vide.lignes.is_empty(),
        "aucune demande du premier établissement n'apparaît dans le second"
    );
    // Depuis le second établissement, la demande du premier est introuvable, même pour un compte
    // habilité là-bas (la clé composite et la RLS la rendent invisible) : 404 neutre, pas 403.
    let r = ent.droits_demande(autre, dir2, DIR, id).await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "introuvable, pas « refusée » : {r:?}"
    );
    let t = ent.droits_tableau(autre, dir2, DIR, false).await.unwrap();
    assert!(t.lignes.is_empty(), "aucune demande du voisin");
}

// ─────────────────────────────── Alertes d'échéance ───────────────────────────────

/// S-544 : alerte J-10 et J-3 aux personnes habilitées, une seule fois par palier, sans nom d'élève.
#[tokio::test]
async fn g3a_alerte_avant_echeance_idempotente() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    // Une demande dont l'échéance tombe dans exactement 10 jours : la date de réception est choisie pour
    // cela (le test ne dépend pas du jour où il tourne, règle 33).
    let mut recue = jour - Duration::days(20);
    while (echeance_legale(recue) - jour).num_days() != 10 {
        recue -= Duration::days(1);
        assert!(
            (jour - recue).num_days() < 70,
            "date de réception introuvable pour J-10"
        );
    }
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courriel",
                recue_le: recue,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Demandeur à échéance"),
                demandeur_contact: None,
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("saisie");
    assert_eq!((v.echeance_effective - jour).num_days(), 10);
    let n1 = ent.droits_alerter_echeances(etab).await.expect("alertes");
    assert!(n1 >= 1, "au moins une personne habilitée est alertée");
    let n2 = ent.droits_alerter_echeances(etab).await.expect("rejeu");
    assert_eq!(n2, 0, "une seule alerte par palier (idempotent)");
    let avis: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT destinataire_user_id, corps FROM ent_notification
          WHERE source_event_id = $1 AND kind = 'rgpd.demande_echeance_proche'",
    )
    .bind(v.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(!avis.is_empty());
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    for (u, corps) in &avis {
        assert!(corps.contains(&v.reference), "{corps}");
        assert!(!corps.contains(&nom_eleve), "aucun nom d'élève : {corps}");
        // Les destinataires portent bien le droit.
        assert!(
            ent.a_action_etablissement(etab, *u, "direction", Action::DroitsTraiter)
                .await
                .unwrap()
                || ent
                    .a_action_etablissement(etab, *u, "administratif", Action::DroitsTraiter)
                    .await
                    .unwrap(),
            "destinataire habilité"
        );
    }
    // Une famille n'est jamais destinataire d'une alerte interne.
    assert!(
        !avis.iter().any(|(u, _)| *u == cast.parent_lea.user_id),
        "aucune famille alertée"
    );
    let j: Vec<String> = ent
        .droits_demande(etab, dir, DIR, v.id)
        .await
        .unwrap()
        .journal
        .iter()
        .map(|l| l.evenement.clone())
        .collect();
    assert!(j.contains(&"echeance_alertee".to_string()), "{j:?}");
}

// ─────────────────────────────── Journal en ajout seul ───────────────────────────────

/// R-RG40 : chaque transition écrit le journal dans la même transaction, et ce journal est en **ajout
/// seul** (ni `UPDATE`, ni `DELETE`, ni `TRUNCATE`).
#[tokio::test]
async fn g3a_journal_en_ajout_seul() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (id, _) = demande_guichet(&ent, &cast).await;
    ent.droits_qualifier(etab, dir, DIR, id, "rgpd", "demande d'accès ordinaire")
        .await
        .unwrap();
    let lignes: i64 =
        sqlx::query_scalar("SELECT count(*) FROM rgp_demande_journal WHERE demande_id = $1")
            .bind(id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert!(lignes >= 2, "réception et qualification au journal");
    for sql in [
        "UPDATE rgp_demande_journal SET detail = 'réécrit' WHERE demande_id = $1",
        "DELETE FROM rgp_demande_journal WHERE demande_id = $1",
    ] {
        let e = sqlx::query(sql).bind(id).execute(ent.pool()).await;
        assert!(e.is_err(), "ajout seul : {sql}");
    }
    let e = sqlx::query("TRUNCATE rgp_demande_journal")
        .execute(ent.pool())
        .await;
    assert!(e.is_err(), "TRUNCATE interdit");
    // Le compteur de numéros ne recule pas et ne se supprime pas.
    let e = sqlx::query("UPDATE rgp_demande_compteur SET dernier = 0 WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await;
    assert!(e.is_err(), "le compteur ne recule pas");
    let e = sqlx::query("DELETE FROM rgp_demande_compteur WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await;
    assert!(e.is_err(), "le compteur ne se supprime pas");
    // Le détail du journal ne porte aucune donnée d'élève (E14).
    let details: Vec<Option<String>> =
        sqlx::query_scalar("SELECT detail FROM rgp_demande_journal WHERE demande_id = $1")
            .bind(id)
            .fetch_all(ent.pool())
            .await
            .unwrap();
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    for d in details.iter().flatten() {
        assert!(!d.contains(&nom_eleve), "donnée d'élève au journal : {d}");
    }
}

/// Le classement des nouvelles tables est bien celui attendu (registre E12 et catégorie de conservation).
#[tokio::test]
async fn g3a_registre_des_tables_et_categorie() {
    use crate::conservation::categorie_de_nom;
    use crate::registre_donnees::classer;
    for t in [
        "rgp_demande",
        "rgp_demande_droit",
        "rgp_demande_identite",
        "rgp_demande_journal",
        "rgp_demande_compteur",
    ] {
        let c = classer(t).unwrap_or_else(|| panic!("{t} non classée"));
        assert_eq!(c.domaine, "conformite", "{t}");
        assert!(c.colonne_etablissement, "{t} : par établissement");
        assert_eq!(categorie_de_nom(t), Some("rgpd.demande"), "{t}");
    }
    assert!(classer("rgp_demande").unwrap().donnees_personnelles);
    assert!(
        !classer("rgp_demande_compteur")
            .unwrap()
            .donnees_personnelles,
        "un compteur d'entiers n'est pas une donnée personnelle"
    );
    // La gouvernance garde sa catégorie : la nouvelle règle ne l'a pas emportée.
    assert_eq!(
        categorie_de_nom("rgp_gouvernance"),
        Some("rgpd.gouvernance")
    );
}
