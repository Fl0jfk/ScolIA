//! Socle 27, **suites du palier G3b + G4** (contre-revue des lots 1 et 2 : M2, M4, M5, F1, F2, F3, F5, F6 ; faible du
//! lot 5 : reprise du balayage des alertes). Jeu fictif « parents séparés » du lot G3b-2. Aucune heure réelle dans
//! les décisions testées (règle 33) ; base exigée sous `CI` ou `ENT_EXIGER_BASE`.

use super::tests_rgpd_g3b::donner_role_dpo;
use super::tests_rgpd_g3b2::{demande, labo, parents_separes, ADF, ADRESSE_AUTRE, DIR, TEL_AUTRE};
use super::*;
use crate::droits_extrait::ContenuExtrait;
use crate::droits_extrait_masquage::appliquer_masques;
use crate::service::RefusMetier;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

fn texte(c: &ContenuExtrait) -> String {
    format!(
        "{:?}",
        c.sections
            .iter()
            .flat_map(|s| s.tables.iter())
            .collect::<Vec<_>>()
    )
}

fn est_conflit(e: &EntDbError) -> bool {
    matches!(
        e,
        EntDbError::Refus(RefusMetier::Conflit, _) | EntDbError::ConflitDossier(_)
    )
}

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

fn est_invalide(e: &EntDbError) -> bool {
    matches!(e, EntDbError::SaisieInvalide(_))
}

/// Section qui porte le dossier d'inscription (et ses responsables) dans un extrait.
fn section_responsables(c: &ContenuExtrait) -> String {
    c.sections
        .iter()
        .find(|s| {
            s.tables
                .iter()
                .any(|t| t.nom == "ent_inscription_responsable")
        })
        .map(|s| s.fournisseur.clone())
        .expect("le dossier d'inscription est dans l'extrait")
}

/// F1, F2 : un marquage ne se supprime pas, l'élément est figé, une décision ne revient pas « à décider » ; la
/// période et l'auteur d'un extrait sont figés ; un extrait périmé le reste.
#[tokio::test]
async fn g3b4_gardes_des_marquages_et_des_extraits() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, true).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let id = e[0].id;
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, id)
        .await
        .unwrap();
    let m = c.marquages[0].id;
    let p = ent.pool_verif();
    assert!(
        sqlx::query("DELETE FROM rgp_extrait_masquage WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(m)
            .execute(p)
            .await
            .is_err(),
        "F1 : un marquage ne se supprime pas"
    );
    assert!(
        sqlx::query("UPDATE rgp_extrait_masquage SET table_nom = 'ent_autre' WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(m)
            .execute(p)
            .await
            .is_err(),
        "F1 : l'élément marqué est figé"
    );
    ent.droits_masquage_decider(etab, dir, DIR, d.id, m, "masque", None)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "UPDATE rgp_extrait_masquage SET decision = 'a_decider', decide_le = NULL
              WHERE etablissement_id = $1 AND id = $2"
        )
        .bind(etab)
        .bind(m)
        .execute(p)
        .await
        .is_err(),
        "F1 : une décision ne revient pas à « à décider »"
    );
    assert!(
        sqlx::query("UPDATE rgp_extrait SET periode_du = DATE '2020-01-01' WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(p)
            .await
            .is_err(),
        "F2 : la période assemblée est figée"
    );
    assert!(
        sqlx::query("UPDATE rgp_extrait SET assemble_par_compte_id = NULL WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(p)
            .await
            .is_err(),
        "F2 : l'auteur de l'assemblage est figé"
    );
    // Une nouvelle version périme la première ; on ne la « dépérime » pas.
    ent.droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "UPDATE rgp_extrait SET perime_le = NULL, perime_motif = NULL WHERE etablissement_id = $1 AND id = $2"
        )
        .bind(etab)
        .bind(id)
        .execute(p)
        .await
        .is_err(),
        "F2 : un extrait périmé le reste"
    );
}

/// M4 (R-RG46) : le secrétariat ne voit ni le marquage d'une protection, ni son emplacement, ni la valeur ; seul un
/// compteur dit qu'il reste des éléments réservés à la direction. Les autres raisons réservées portent le même alias.
#[tokio::test]
async fn g3b4_le_secretariat_ne_situe_pas_une_protection() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, adm) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.administratif.user_id,
    );
    parents_separes(&ent, &cast, true).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let brut = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let proteges = brut
        .marquages
        .iter()
        .filter(|m| m.raison == "adresse_protegee")
        .count();
    assert!(proteges > 0, "le jeu porte bien une protection");
    assert_eq!(
        brut.elements_reserves_direction, 0,
        "la direction voit tout"
    );
    let vu = ent
        .droits_extrait_lire(etab, adm, ADF, d.id, e[0].id)
        .await
        .unwrap();
    // E1 : la protection et les marquages qui la doubleraient sur la même cible sont tous cachés, et comptés.
    assert!(vu.elements_reserves_direction >= proteges);
    assert_eq!(
        vu.elements_reserves_direction,
        brut.marquages.len() - vu.marquages.len()
    );
    assert!(vu
        .marquages
        .iter()
        .all(|m| m.raison_interne != "adresse_protegee"));
    assert!(
        vu.marquages
            .iter()
            .filter(|m| m.levee_reservee_direction)
            .all(|m| m.raison == "a_arbitrer"),
        "toutes les raisons réservées portent le même alias"
    );
    let t = texte(&vu);
    assert!(!t.contains(ADRESSE_AUTRE) && !t.contains("réservé à la direction"));
    let liste = ent
        .droits_masquages(etab, adm, ADF, d.id, e[0].id)
        .await
        .unwrap();
    assert!(liste.iter().all(|m| m.raison_interne != "adresse_protegee"));
    assert_eq!(
        ent.droits_masquages(etab, dir, DIR, d.id, e[0].id)
            .await
            .unwrap()
            .iter()
            .filter(|m| m.raison_interne == "adresse_protegee")
            .count(),
        proteges
    );
}

/// M5 : l'ajout manuel désigne un élément qui existe ; une colonne sans ligne vise toutes les lignes ; le même
/// élément ne se masque pas deux fois.
#[tokio::test]
async fn g3b4_ajout_manuel_valide_et_masque_toutes_les_lignes() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, false).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let x = e[0].id;
    let brut = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, x)
        .await
        .unwrap();
    let f = section_responsables(&brut);
    let t = "ent_inscription_responsable";
    let ajouter = |table: &'static str,
                   ligne: Option<Uuid>,
                   rang: Option<i32>,
                   colonne: Option<&'static str>| {
        let (ent, f) = (&ent, f.clone());
        async move {
            ent.droits_masquage_ajouter(
                etab,
                dir,
                DIR,
                d.id,
                x,
                &f,
                table,
                ligne,
                rang,
                colonne,
                "précision de la famille",
            )
            .await
        }
    };
    let err = ajouter("ent_table_inconnue", None, None, None)
        .await
        .unwrap_err();
    assert!(est_invalide(&err), "table absente : {err:?}");
    let err = ajouter(t, Some(Uuid::new_v4()), None, None)
        .await
        .unwrap_err();
    assert!(est_invalide(&err), "ligne absente : {err:?}");
    let err = ajouter(t, None, Some(99), None).await.unwrap_err();
    assert!(est_invalide(&err), "rang absent : {err:?}");
    let err = ajouter(t, None, None, Some("colonne_inconnue"))
        .await
        .unwrap_err();
    assert!(est_invalide(&err), "colonne absente : {err:?}");
    let err = ajouter(t, Some(Uuid::new_v4()), Some(0), None)
        .await
        .unwrap_err();
    assert!(est_invalide(&err), "ligne et rang ensemble : {err:?}");
    let v = ajouter(t, None, None, Some("telephone")).await.unwrap();
    let m = v
        .iter()
        .find(|m| m.raison == "manuel")
        .expect("masquage manuel");
    assert_eq!((m.rang, m.ligne_id), (None, None), "toute la colonne");
    let err = ajouter(t, None, None, Some("telephone")).await.unwrap_err();
    assert!(est_conflit(&err), "doublon : {err:?}");
    // Le téléphone du demandeur lui-même est masqué : la colonne entière l'a été.
    let marques = ent.droits_masquages(etab, dir, DIR, d.id, x).await.unwrap();
    let remis = texte(&appliquer_masques(&brut, &marques));
    assert!(!remis.contains("0600000001") && !remis.contains(TEL_AUTRE));
}

/// M2 : une décision n'est reportée sur la version suivante que si l'élément n'a pas changé.
#[tokio::test]
async fn g3b4_decision_non_reportee_quand_l_element_change() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, false).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e1 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e1[0].id)
        .await
        .unwrap();
    for m in &c.marquages {
        ent.droits_masquage_decider(
            etab,
            dir,
            DIR,
            d.id,
            m.id,
            "leve",
            Some("pièce vue par la direction"),
        )
        .await
        .unwrap();
    }
    // Rien n'a changé : tout est reporté.
    let e2 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    assert_eq!(e2[0].masquages_a_decider, 0);
    // Le numéro de l'autre parent change : la levée motivée sur l'ancien numéro ne vaut pas pour le nouveau.
    sqlx::query(
        "UPDATE ent_inscription_responsable SET telephone = '0699999999' WHERE etablissement_id = $1 AND telephone = $2",
    )
    .bind(etab)
    .bind(TEL_AUTRE)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let e3 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    assert!(
        e3[0].masquages_a_decider > 0,
        "l'élément a changé : la levée est à redécider"
    );
}

/// F5 : une décision de marquage après l'avis du DPO efface cet avis. F6 : le DPO reçoit 404, pas 403, pour un
/// extrait qui ne lui est pas soumis.
#[tokio::test]
async fn g3b4_avis_dpo_efface_et_refus_neutre() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, false).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let dpo = donner_role_dpo(&ent, &cast).await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let x = e[0].id;
    let err = ent
        .droits_extrait_lire(etab, dpo, "personnel", d.id, x)
        .await
        .unwrap_err();
    assert!(est_introuvable(&err), "F6 : {err:?}");
    let err = ent
        .droits_masquages(etab, dpo, "personnel", d.id, x)
        .await
        .unwrap_err();
    assert!(est_introuvable(&err), "F6 : {err:?}");
    ent.droits_extrait_soumettre_dpo(etab, dir, DIR, d.id, x)
        .await
        .unwrap();
    ent.droits_extrait_avis_dpo(
        etab,
        dpo,
        "personnel",
        d.id,
        x,
        "favorable",
        "rien à signaler",
    )
    .await
    .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, x)
        .await
        .unwrap();
    ent.droits_masquage_decider(etab, dir, DIR, d.id, c.marquages[0].id, "masque", None)
        .await
        .unwrap();
    let v = ent.droits_extraits(etab, dir, DIR, d.id).await.unwrap();
    let v = v.iter().find(|v| v.id == x).unwrap();
    assert_eq!(
        v.avis_dpo_code, None,
        "l'avis portait sur une autre relecture"
    );
    assert!(v.soumis_dpo, "la soumission reste");
}

/// F3 : les tables de messagerie écartées sont déclarées dans la section, avec leur motif.
#[tokio::test]
async fn g3b4_tables_de_messagerie_ecartees_declarees() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let s = c
        .sections
        .iter()
        .find(|s| s.fournisseur == "messagerie")
        .expect("section messagerie");
    for t in [
        "ent_msg_telephone",
        "ent_msg_lien_decision",
        "ent_msg_brouillon",
    ] {
        assert!(
            s.exclusions
                .iter()
                .any(|x| x.table == t && !x.motif.is_empty()),
            "{t} déclarée"
        );
    }
    assert!(
        !s.exclusions.iter().any(|x| x.table == "ent_msg"),
        "une table rendue n'est pas déclarée écartée"
    );
}

/// Faible du lot 5 : le balayage d'alertes note l'instant atteint, et repart de lui après un arrêt long.
#[tokio::test]
async fn g4b_le_balayage_des_alertes_note_sa_reprise() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let t0: DateTime<Utc> = DateTime::parse_from_rfc3339("2027-02-01T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let jusqu = |ent: &PgEnt| {
        let p = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, DateTime<Utc>>(
                "SELECT jusqu_au FROM acc_balayage WHERE etablissement_id = $1",
            )
            .bind(etab)
            .fetch_one(&p)
            .await
            .unwrap()
        }
    };
    ent.acces_alertes_evaluer_a(etab, t0).await.unwrap();
    assert_eq!(jusqu(&ent).await, t0);
    let t1 = t0 + Duration::days(3);
    ent.acces_alertes_evaluer_a(etab, t1).await.unwrap();
    assert_eq!(jusqu(&ent).await, t1);
    // Une évaluation rejouée plus tôt (horloge injectée en arrière) ne fait pas reculer la reprise.
    ent.acces_alertes_evaluer_a(etab, t0).await.unwrap();
    assert_eq!(jusqu(&ent).await, t1);
}
