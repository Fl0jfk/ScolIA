//! Socle 27, lot G1 : référentiel de conservation, DPO facultatif, garde de la purge, tableau des
//! sanctions à effacer. Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).

use super::*;
use crate::conservation::CATALOGUE;
use crate::conservation_regles::EtatAffichage;
use crate::droits_repo::{Droit, RoleSaisie};
use chrono::NaiveDate;
use ent_auth::{Action, Portee};
use sqlx::Row;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

const DIR: &str = "direction";

/// Un rôle paramétré « DPO » (droit de valider) attribué à un compte du personnel.
async fn donner_role_dpo(ent: &PgEnt, cast: &LaboCast) -> (Uuid, &'static str) {
    let etab = cast.etablissement_id;
    // Seul l'administrateur technique (qui couvre tous les droits) crée un rôle porteur de la validation :
    // la direction ne se l'attribue pas (`exiger_couverture`).
    let admin = cast.admin.user_id;
    assert!(
        ent.creer_role_for_actor(
            etab,
            cast.direction.user_id,
            DIR,
            RoleSaisie {
                nom: "Tentative".into(),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::ConservationReglesValider,
                    portee: Portee::Etablissement,
                    cibles: vec![]
                }],
            },
        )
        .await
        .is_err(),
        "la direction ne peut pas se donner le droit de valider"
    );
    let compte = cast.comptas[0].user_id;
    let droit = |action| Droit {
        action,
        portee: Portee::Etablissement,
        cibles: vec![],
    };
    let role = ent
        .creer_role_for_actor(
            etab,
            admin,
            "admin",
            RoleSaisie {
                nom: format!("DPO {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![
                    droit(Action::ConformiteLire),
                    droit(Action::ConservationReglesValider),
                ],
            },
        )
        .await
        .expect("rôle DPO");
    ent.attribuer_role_for_actor(etab, admin, "admin", role.id, compte, None)
        .await
        .expect("attribution");
    (compte, "personnel")
}

async fn nb_journal(ent: &PgEnt, etab: Uuid) -> i64 {
    sqlx::query("SELECT count(*) AS n FROM cons_regle_journal WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .expect("compte")
        .get("n")
}

#[tokio::test]
async fn g1_catalogue_charge_et_en_attente_de_dpo() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    // Avant tout chargement : tout est « non chargé », rien n'est autorisé.
    let p = ent
        .politique_conservation(etab, dir, DIR)
        .await
        .expect("politique");
    assert_eq!(p.non_chargees.len(), CATALOGUE.len());
    assert!(!p.dpo_nomme);
    // Chargement : une règle proposée par catégorie, idempotent.
    let n = ent
        .charger_catalogue_conservation(etab, dir, DIR)
        .await
        .expect("charge");
    assert_eq!(n, CATALOGUE.len());
    assert_eq!(
        ent.charger_catalogue_conservation(etab, dir, DIR)
            .await
            .expect("rejeu"),
        0
    );
    let p = ent
        .politique_conservation(etab, dir, DIR)
        .await
        .expect("politique");
    assert!(p.non_chargees.is_empty());
    assert_eq!(p.nb_validees, 0);
    assert!(
        p.regles
            .iter()
            .all(|r| r.etat == EtatAffichage::EnAttenteDpo),
        "sans DPO : en attente de DPO"
    );
    assert!(p.synthese.contains("en attente de DPO"));
    assert!(
        !p.synthese.to_lowercase().contains("conforme"),
        "jamais « conforme » (R-RG9)"
    );
    // Rien n'autorise une purge réelle.
    for c in CATALOGUE {
        assert!(
            !ent.purge_autorisee(etab, c.code).await.unwrap(),
            "{} purgeable sans validation",
            c.code
        );
    }
    assert!(!ent.purge_autorisee(etab, "inconnue.x").await.unwrap());
    // Droits : l'administrateur technique, un professeur, une famille ne lisent rien.
    for (u, r) in [
        (cast.admin.user_id, "admin"),
        (cast.prof.user_id, "prof"),
        (cast.parent_lea.user_id, "parent"),
    ] {
        assert!(
            matches!(
                ent.politique_conservation(etab, u, r).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
        assert!(
            matches!(
                ent.charger_catalogue_conservation(etab, u, r).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
    }
    // Chaque règle chargée est journalisée.
    assert!(nb_journal(&ent, etab).await >= CATALOGUE.len() as i64);
}

#[tokio::test]
async fn g1_la_validation_est_celle_du_role_designe_et_seule_elle_autorise_la_purge() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.charger_catalogue_conservation(etab, dir, DIR)
        .await
        .unwrap();
    // La direction n'a pas le droit de valider par défaut.
    assert!(matches!(
        ent.valider_regle_conservation(etab, dir, DIR, "vs.appel")
            .await,
        Err(EntDbError::Forbidden)
    ));
    let (dpo, role) = donner_role_dpo(&ent, &cast).await;
    // Le DPO valide une catégorie : elle seule devient purgeable.
    let v = ent
        .valider_regle_conservation(etab, dpo, role, "vs.appel")
        .await
        .expect("valide");
    assert_eq!(v.etat, EtatAffichage::Validee);
    assert!(v.valide_le.is_some());
    assert!(ent.purge_autorisee(etab, "vs.appel").await.unwrap());
    assert!(!ent.purge_autorisee(etab, "vs.justificatif").await.unwrap());
    // Valider deux fois : plus de proposition ouverte.
    assert!(matches!(
        ent.valider_regle_conservation(etab, dpo, role, "vs.appel")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Validation en bloc : tout ou rien (une catégorie inconnue annule le bloc).
    assert!(ent
        .valider_regles_conservation(etab, dpo, role, &["vs.justificatif", "inconnue.x"])
        .await
        .is_err());
    assert!(
        !ent.purge_autorisee(etab, "vs.justificatif").await.unwrap(),
        "le bloc refusé n'a rien validé"
    );
    let n = ent
        .valider_regles_conservation(etab, dpo, role, &["vs.justificatif", "cdt.contenu"])
        .await
        .unwrap()
        .len();
    assert_eq!(n, 2);
    assert!(ent.purge_autorisee(etab, "cdt.contenu").await.unwrap());
    // Le DPO n'a pas le droit de proposer/charger (autre droit).
    assert!(matches!(
        ent.charger_catalogue_conservation(etab, dpo, role).await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn g1_dpo_facultatif_designation_et_retrait() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.charger_catalogue_conservation(etab, dir, DIR)
        .await
        .unwrap();
    let g = ent.gouvernance(etab, dir, DIR).await.unwrap();
    assert!(!g.dpo_nomme);
    // Sans DPO : en attente de DPO ; avec DPO nommé : à valider ; retrait : de nouveau en attente.
    let p = ent.politique_conservation(etab, dir, DIR).await.unwrap();
    assert!(p
        .regles
        .iter()
        .all(|r| r.etat == EtatAffichage::EnAttenteDpo));
    let g = ent
        .designer_dpo(etab, dir, DIR, "Maître Exemple", "dpo@exemple.test", true)
        .await
        .unwrap();
    assert!(g.dpo_nomme && g.dpo_externe == Some(true));
    let p = ent.politique_conservation(etab, dir, DIR).await.unwrap();
    assert!(p.dpo_nomme && p.regles.iter().all(|r| r.etat == EtatAffichage::AValider));
    let g = ent.retirer_dpo(etab, dir, DIR).await.unwrap();
    assert!(!g.dpo_nomme);
    assert!(ent
        .politique_conservation(etab, dir, DIR)
        .await
        .unwrap()
        .regles
        .iter()
        .all(|r| r.etat == EtatAffichage::EnAttenteDpo));
    // Saisies invalides et droits.
    assert!(ent
        .designer_dpo(etab, dir, DIR, "  ", "x@y.test", false)
        .await
        .is_err());
    assert!(ent
        .designer_dpo(etab, dir, DIR, "Nom", "ab", false)
        .await
        .is_err());
    assert!(matches!(
        ent.designer_dpo(
            etab,
            cast.prof.user_id,
            "prof",
            "Nom",
            "dpo@exemple.test",
            false
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn g1_ajustement_dans_les_bornes_avec_motif() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.charger_catalogue_conservation(etab, dir, DIR)
        .await
        .unwrap();
    let ajuster = |code: &'static str, u: Option<u16>, i: Option<u16>, motif: &'static str| {
        ent.ajuster_regle_conservation(etab, dir, DIR, code, u, i, None, motif)
    };
    // Bornes légales connues : traces d'accès entre 6 et 36 mois.
    assert!(ajuster("acc.trace", Some(48), None, "trop long")
        .await
        .is_err());
    assert!(ajuster("acc.trace", Some(5), None, "trop court")
        .await
        .is_err());
    // Motif obligatoire, catégorie connue, durée absurde refusée.
    assert!(ajuster("acc.trace", Some(24), None, "  ").await.is_err());
    assert!(ajuster("inconnue.x", Some(1), None, "motif").await.is_err());
    assert!(ajuster("vs.appel", Some(5000), None, "absurde")
        .await
        .is_err());
    // Critère de tri refusé hors catégorie triée.
    assert!(ent
        .ajuster_regle_conservation(
            etab,
            dir,
            DIR,
            "vs.appel",
            Some(6),
            None,
            Some("critère"),
            "motif"
        )
        .await
        .is_err());
    // Ajustement valide : nouvelle version proposée, l'ancienne proposition est remplacée.
    let v = ajuster("acc.trace", Some(24), None, "politique de l'établissement")
        .await
        .unwrap();
    assert_eq!(v.version, 2);
    assert_eq!(v.utilite_mois, Some(24));
    assert_eq!(
        v.motif_ajustement.as_deref(),
        Some("politique de l'établissement")
    );
    let v = ajuster("acc.trace", Some(18), None, "encore plus court")
        .await
        .unwrap();
    assert_eq!(v.version, 3);
    let ouvertes: i64 = sqlx::query("SELECT count(*) AS n FROM cons_regle WHERE etablissement_id = $1 AND categorie_code = 'acc.trace' AND etat = 'proposee'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap()
        .get("n");
    assert_eq!(ouvertes, 1, "une seule proposition ouverte par catégorie");
    // La règle validée en vigueur reste appliquée tant que l'ajustement n'est pas validé.
    let (dpo, role) = donner_role_dpo(&ent, &cast).await;
    ent.valider_regle_conservation(etab, dpo, role, "vs.appel")
        .await
        .unwrap();
    let v = ajuster("vs.appel", Some(6), None, "durée plus courte")
        .await
        .unwrap();
    assert_eq!(v.etat, EtatAffichage::Validee);
    assert_eq!(v.version, 1, "la règle en vigueur est la validée");
    assert!(v.proposition_en_cours);
    assert!(ent.purge_autorisee(etab, "vs.appel").await.unwrap());
    let v = ent
        .valider_regle_conservation(etab, dpo, role, "vs.appel")
        .await
        .unwrap();
    assert_eq!(v.version, 2);
    assert_eq!(v.utilite_mois, Some(6));
    let validees: i64 = sqlx::query("SELECT count(*) AS n FROM cons_regle WHERE etablissement_id = $1 AND categorie_code = 'vs.appel' AND etat = 'validee'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap()
        .get("n");
    assert_eq!(validees, 1, "une seule règle validée en vigueur");
}

#[tokio::test]
async fn g1_journal_en_ajout_seul_et_cloisonnement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.charger_catalogue_conservation(etab, dir, DIR)
        .await
        .unwrap();
    assert!(nb_journal(&ent, etab).await > 0);
    for sql in [
        "UPDATE cons_regle_journal SET par_role = 'x'",
        "DELETE FROM cons_regle_journal",
        "TRUNCATE cons_regle_journal",
    ] {
        let e = sqlx::query(sql).execute(ent.pool()).await.expect_err(sql);
        assert!(e.to_string().contains("ajout seul"), "{sql} : {e}");
    }
    // Un autre établissement ne voit rien et un acteur de A n'agit pas sur B.
    let autre = ent.seed_labo_cast().await.expect("second établissement");
    assert_ne!(autre.etablissement_id, etab);
    let p = ent
        .politique_conservation(autre.etablissement_id, autre.direction.user_id, DIR)
        .await
        .unwrap();
    assert_eq!(
        p.non_chargees.len(),
        CATALOGUE.len(),
        "les règles de A sont invisibles de B"
    );
    // Même avec l'identifiant d'un acteur de A, la lecture de B ne montre aucune règle de A.
    let p = ent
        .politique_conservation(autre.etablissement_id, dir, DIR)
        .await
        .unwrap();
    assert_eq!(p.non_chargees.len(), CATALOGUE.len());
    assert!(!ent
        .purge_autorisee(autre.etablissement_id, "vs.appel")
        .await
        .unwrap());
}

#[tokio::test]
async fn g1_pourquoi_garde_t_on() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.charger_catalogue_conservation(etab, dir, DIR)
        .await
        .unwrap();
    let e = ent
        .pourquoi_garde_t_on(etab, dir, DIR, "ent_absence")
        .await
        .unwrap();
    assert_eq!(e.categorie, "vs.appel");
    assert_eq!(e.etat, EtatAffichage::EnAttenteDpo);
    assert!(e.texte.contains("en attente de DPO") && e.texte.contains("rien n'est détruit"));
    let e = ent
        .pourquoi_garde_t_on(etab, dir, DIR, "disc.sanction")
        .await
        .unwrap();
    assert_eq!(e.categorie, "disc.sanction");
    assert!(ent
        .pourquoi_garde_t_on(etab, dir, DIR, "table_inconnue")
        .await
        .is_err());
    assert!(matches!(
        ent.pourquoi_garde_t_on(etab, cast.prof.user_id, "prof", "ent_absence")
            .await,
        Err(EntDbError::Forbidden)
    ));
}

async fn sanction(
    ent: &PgEnt,
    cast: &LaboCast,
    code: &str,
    categorie: &str,
    jour: NaiveDate,
) -> Uuid {
    let etab = cast.etablissement_id;
    let typ = ent
        .creer_sanction_type(domain_eleve::CreerReferentielVs {
            etablissement_id: etab,
            code: code.into(),
            libelle: code.into(),
            actor_user_id: None,
        })
        .await
        .unwrap();
    ent.categoriser_type_sanction(etab, cast.direction.user_id, DIR, typ.id, categorie)
        .await
        .unwrap();
    ent.prononcer_sanction(domain_eleve::PrononcerSanction {
        etablissement_id: etab,
        eleve_id: cast.eleve_lea_id,
        type_id: typ.id,
        motif_id: None,
        incident_id: None,
        jour,
        duree_minutes: None,
        note: Some("détail confidentiel".into()),
        actor_user_id: cast.direction.user_id,
    })
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn g1_tableau_des_sanctions_a_effacer() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let d = |y, m, j| NaiveDate::from_ymd_opt(y, m, j).unwrap();
    let avert = sanction(&ent, &cast, "g1_avert", "avertissement", d(2026, 10, 12)).await;
    let definitive = sanction(
        &ent,
        &cast,
        "g1_def",
        "exclusion_definitive",
        d(2026, 10, 12),
    )
    .await;
    // Veille de l'échéance (1er septembre 2027) : l'avertissement est « à venir ».
    let t = ent
        .sanctions_a_effacer(etab, dir, DIR, d(2027, 8, 31))
        .await
        .unwrap();
    let ligne = t
        .iter()
        .find(|s| s.sanction_id == avert)
        .expect("avertissement listé");
    assert_eq!(ligne.etat, "a_venir");
    assert_eq!(ligne.echeance, d(2027, 9, 1));
    assert!(
        t.iter().all(|s| s.sanction_id != definitive),
        "l'exclusion définitive n'est jamais effacée automatiquement"
    );
    // Le jour de l'échéance : « échue ». Lecture seule : la sanction n'est pas effacée par le tableau.
    let t = ent
        .sanctions_a_effacer(etab, dir, DIR, d(2027, 9, 1))
        .await
        .unwrap();
    assert_eq!(
        t.iter().find(|s| s.sanction_id == avert).unwrap().etat,
        "echue"
    );
    let (effacee,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT effacee_at FROM ent_sanction WHERE id = $1")
            .bind(avert)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(effacee.is_none());
    // Droit et trace d'accès : un professeur est refusé ; la consultation est tracée.
    assert!(matches!(
        ent.sanctions_a_effacer(etab, cast.prof.user_id, "prof", d(2027, 9, 1))
            .await,
        Err(EntDbError::Forbidden)
    ));
    let n: i64 = sqlx::query("SELECT count(*) AS n FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'sanctions_a_effacer'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap()
        .get("n");
    assert!(n >= 2, "la consultation du tableau est tracée");
    // Trace en échec : le tableau n'est pas servi (R-RG8).
    let nom = format!("g1_refus_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE FUNCTION {nom}_fn() RETURNS trigger AS $$ BEGIN RAISE EXCEPTION 'refus (test G1)'; END; $$ LANGUAGE plpgsql"))
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(&format!(
        "CREATE TRIGGER {nom} BEFORE INSERT ON ent_journal_modif FOR EACH ROW \
         WHEN (NEW.etablissement_id = '{etab}'::uuid AND NEW.action = 'sanctions_a_effacer') EXECUTE FUNCTION {nom}_fn()"
    ))
    .execute(ent.pool())
    .await
    .unwrap();
    let refus = ent.sanctions_a_effacer(etab, dir, DIR, d(2027, 9, 1)).await;
    sqlx::query(&format!("DROP TRIGGER {nom} ON ent_journal_modif"))
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(&format!("DROP FUNCTION {nom}_fn()"))
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(refus.is_err(), "tableau servi malgré l'échec de la trace");
}
