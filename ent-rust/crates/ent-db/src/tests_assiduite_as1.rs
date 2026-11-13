//! Socle 12, lot AS1 (référentiels et droits). Réf. : `docs/socles/12_ASSIDUITE.md` §15.1, §22.1, §24.
//! Critères de fin : un rôle libre « AED » fait l'appel d'une permanence et propose une qualification
//! sans qualifier ; une règle utilisée par un mois arrêté est immuable. Sans `ENT_DATABASE_URL` :
//! ignorés par absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use ent_auth::{Action, Portee, Role};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { e.install_brain_p0(); }).await;
    Some((ent, cast))
}

fn regle(valide_du: chrono::NaiveDate) -> SaisieRegle {
    SaisieRegle {
        valide_du,
        definition_demi_journee: "minutes".into(),
        seuil_minutes: 45,
        heure_coupure: chrono::NaiveTime::from_hms_opt(12, 30, 0).unwrap(),
        type_mois: "civil".into(),
        profil_id: None,
        seuil_signalement_demi_journees: 4,
        delai_consolidation_min: 10,
        delai_verification_min: 15,
        rythme_relance: "standard".into(),
        seuil_retard_absence_pct: 50,
        seuil_absences_repetees: 3,
        seuil_retards: 5,
    }
}

fn motif(code: &str, nature: &str) -> SaisieMotif {
    SaisieMotif {
        code: code.into(),
        nature_code: nature.into(),
        libelle: "Rendez-vous chez l'orthodontiste".into(),
        libelle_famille: Some("Rendez-vous médical".into()),
        visible_famille: true,
        actif: true,
    }
}

fn interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

// ─── Référentiels globaux ───────────────────────────────────────────────────

#[tokio::test]
async fn as1_cinq_natures_legitimes_de_l131_8() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let natures = ent.natures_motif_for_actor(etab, dir, "direction").await.unwrap();
    let legit: Vec<_> = natures.iter().filter(|n| n.legitime_l131_8).map(|n| n.code.as_str()).collect();
    assert_eq!(legit.len(), 5, "exactement les cinq motifs de la loi : {legit:?}");
    assert!(legit.contains(&"maladie_enfant") && legit.contains(&"reunion_solennelle_famille"));
    assert!(natures.iter().find(|n| n.code == "maladie_enfant").unwrap().medical);
    assert!(!natures.iter().find(|n| n.code == "transport").unwrap().legitime_l131_8, "un motif de classement n'est pas « réputé légitime »");
    // Un professeur n'a aucun droit sur ce paramétrage.
    assert!(interdit(ent.natures_motif_for_actor(etab, cast.prof.user_id, "prof").await));
    let profils = ent.profils_academiques_for_actor(etab, dir, "direction").await.unwrap();
    let legal = profils.iter().find(|p| p.code_academie == "LEGAL").expect("profil légal");
    assert_eq!(legal.niveaux[0].seuil_demi_journees, 4, "R131-7 : quatre demi-journées");
    assert_eq!(legal.age_limite_penal, 16);
}

// ─── Règle d'assiduité datée (R-AS15, DA-11) ────────────────────────────────

#[tokio::test]
async fn as1_regle_versionnee_jamais_dans_le_passe_et_figee_par_un_mois_arrete() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let aujourdhui = ent.aujourdhui_etablissement(etab).await.unwrap();
    // Base de labo persistante : des dates aléatoires évitent toute collision avec un passage précédent
    // (une règle figée ne se rejoue pas).
    let decalage = 100 + i64::from(Uuid::new_v4().as_u128() as u32 % 20_000);
    let janvier = aujourdhui + chrono::Duration::days(decalage + 60);
    let septembre = aujourdhui + chrono::Duration::days(decalage + 20);

    // Jamais rétroactive.
    let hier = aujourdhui - chrono::Duration::days(1);
    assert!(matches!(
        ent.definir_regle_asd_for_actor(etab, dir, "direction", regle(hier)).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Hors bornes refusé.
    let mut mauvaise = regle(janvier);
    mauvaise.seuil_minutes = 0;
    assert!(matches!(
        ent.definir_regle_asd_for_actor(etab, dir, "direction", mauvaise).await,
        Err(EntDbError::SaisieInvalide(_))
    ));

    let avant = ent.regle_asd_en_vigueur(etab, septembre).await.unwrap();
    ent.definir_regle_asd_for_actor(etab, dir, "direction", regle(janvier)).await.unwrap();
    // « Changement de règle en janvier → septembre inchangé » : la règle d'avant reste celle d'avant.
    assert_eq!(ent.regle_asd_en_vigueur(etab, septembre).await.unwrap(), avant);
    let apres = ent.regle_asd_en_vigueur(etab, janvier).await.unwrap();
    assert_eq!((apres.seuil_minutes, apres.valide_du), (45, janvier));
    assert_eq!(ent.regle_asd_en_vigueur(etab, janvier + chrono::Duration::days(200)).await.unwrap().seuil_minutes, 45);

    // La version du jour peut être remplacée tant qu'aucun mois arrêté ne l'a utilisée.
    let mut v2 = regle(janvier);
    v2.seuil_minutes = 50;
    ent.definir_regle_asd_for_actor(etab, dir, "direction", v2).await.unwrap();
    assert_eq!(ent.regle_asd_en_vigueur(etab, janvier).await.unwrap().seuil_minutes, 50);

    // Un mois arrêté fige la règle utilisée (posé par AS4) : plus aucune modification possible,
    // ni par le service, ni directement en base (déclencheur).
    ent.figer_regle_en_vigueur(etab, janvier).await.unwrap();
    assert!(ent.regle_asd_en_vigueur(etab, janvier).await.unwrap().figee);
    let mut v3 = regle(janvier);
    v3.seuil_minutes = 99;
    assert!(matches!(
        ent.definir_regle_asd_for_actor(etab, dir, "direction", v3).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    for sql in [
        "UPDATE ent_asd_regle SET seuil_minutes = 1 WHERE etablissement_id = $1 AND valide_du = $2",
        "UPDATE ent_asd_regle SET figee_le = NULL WHERE etablissement_id = $1 AND valide_du = $2",
        "DELETE FROM ent_asd_regle WHERE etablissement_id = $1 AND valide_du = $2",
    ] {
        let r = sqlx::query(sql).bind(etab).bind(janvier).execute(ent.pool_verif()).await;
        assert!(r.is_err(), "règle figée : {sql}");
    }
    assert_eq!(ent.regle_asd_en_vigueur(etab, janvier).await.unwrap().seuil_minutes, 50);
    // Une version plus tardive reste possible : le passé figé n'est pas touché.
    ent.definir_regle_asd_for_actor(etab, dir, "direction", regle(janvier + chrono::Duration::days(30))).await.unwrap();
    assert_eq!(ent.regles_asd_for_actor(etab, dir, "direction").await.unwrap().len() >= 2, true);
}

#[tokio::test]
async fn as1_la_regle_est_une_action_de_direction_et_l_ancienne_route_la_suit() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    // DA-11 : le CPE propose des signalements, il ne paramètre pas la règle.
    for (u, r) in [(cast.cpe.user_id, "cpe"), (cast.prof.user_id, "prof"), (cast.administratif.user_id, "administratif")] {
        assert!(interdit(ent.definir_regle_asd_for_actor(etab, u, r, regle(ent.aujourdhui_etablissement(etab).await.unwrap())).await), "{r}");
        assert!(interdit(ent.definir_regle_assiduite(etab, u, r, RegleAssiduite::default()).await), "{r} (ancienne route)");
    }
    // La direction le peut ; l'ancien point d'entrée pose une version datée d'AUJOURD'HUI et le
    // décompte la lit (aucune rupture pour l'existant).
    let mut r = RegleAssiduite::default();
    r.seuil_minutes = 41;
    r.seuil_signalement = 6;
    ent.definir_regle_assiduite(etab, cast.direction.user_id, "direction", r).await.unwrap();
    let lue = ent.regle_assiduite(etab).await.unwrap();
    assert_eq!((lue.seuil_minutes, lue.seuil_signalement), (41, 6));
    let v = ent.regle_asd_en_vigueur(etab, ent.aujourdhui_etablissement(etab).await.unwrap()).await.unwrap();
    assert_eq!(v.seuil_minutes, 41, "même version, lue par les deux chemins");
}

// ─── Motifs d'établissement ─────────────────────────────────────────────────

#[tokio::test]
async fn as1_motifs_versionnes_sans_motif_technique_pour_les_familles() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    // DA-20 : le motif technique « appel » n'est jamais proposé aux familles.
    let famille = ent.motifs_famille(etab).await.unwrap();
    assert!(!famille.is_empty(), "les motifs courants sont amorcés");
    assert!(famille.iter().all(|m| m.code != MOTIF_TECHNIQUE_APPEL));
    assert!(ent.motifs_for_actor(etab, cast.prof.user_id, "prof").await.is_err(), "le professeur ne lit pas les motifs");
    // Création rattachée à une nature légale ou de classement.
    let m = ent.creer_motif_for_actor(etab, dir, "direction", motif("orthodontie", "rendez_vous_medical")).await.unwrap();
    assert_eq!(m.nature_code, "rendez_vous_medical");
    assert!(matches!(
        ent.creer_motif_for_actor(etab, dir, "direction", motif("orthodontie", "autre")).await,
        Err(EntDbError::ConflitDossier(_))
    ), "code déjà pris");
    assert!(matches!(
        ent.creer_motif_for_actor(etab, dir, "direction", motif("appel", "autre")).await,
        Err(EntDbError::SaisieInvalide(_))
    ), "le code technique est réservé");
    assert!(matches!(
        ent.creer_motif_for_actor(etab, dir, "direction", motif("x_inconnu", "nature_inconnue")).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(interdit(ent.creer_motif_for_actor(etab, cast.cpe.user_id, "cpe", motif("autre_cpe", "autre")).await));
    // Modification historisée (DA-10, DA-20) ; la nature ne change pas.
    let mut s = motif("orthodontie", "autre");
    s.libelle = "Rendez-vous orthodontie".into();
    s.actif = false;
    let m2 = ent.modifier_motif_for_actor(etab, dir, "direction", m.id, s).await.unwrap();
    assert_eq!((m2.nature_code.as_str(), m2.actif), ("rendez_vous_medical", false));
    let versions = ent.versions_motif_for_actor(etab, dir, "direction", m.id).await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].libelle, "Rendez-vous chez l'orthodontiste");
    assert!(!ent.motifs_famille(etab).await.unwrap().iter().any(|x| x.id == m.id), "motif inactif : plus proposé");
    // Historique en ajout seul.
    assert!(sqlx::query("UPDATE ent_asd_motif_version SET libelle = 'x' WHERE etablissement_id = $1")
        .bind(etab).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_asd_motif_version WHERE etablissement_id = $1")
        .bind(etab).execute(ent.pool_verif()).await.is_err());
}

// ─── Activités appelées ─────────────────────────────────────────────────────

#[tokio::test]
async fn as1_activites_appelees_se_parametrent_par_la_direction() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let v = ent.activites_appelees_for_actor(etab, dir, "direction").await.unwrap();
    assert_eq!(v.len(), 5);
    assert!(v.iter().any(|a| a.type_activite == "cdi" && !a.active), "le CDI n'est pas appelé tant qu'on ne l'active pas");
    ent.definir_activite_appelee_for_actor(etab, dir, "direction", "cdi", true).await.unwrap();
    assert!(ent.activites_appelees_for_actor(etab, dir, "direction").await.unwrap().iter().any(|a| a.type_activite == "cdi" && a.active));
    ent.definir_activite_appelee_for_actor(etab, dir, "direction", "cdi", false).await.unwrap();
    assert!(!ent.activite_est_appelee(etab, "cdi").await.unwrap());
    assert!(matches!(
        ent.definir_activite_appelee_for_actor(etab, dir, "direction", "cantine", true).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(interdit(ent.definir_activite_appelee_for_actor(etab, cast.cpe.user_id, "cpe", "cdi", true).await));
}

// ─── Droits : fin des rôles codés en dur (DA-18) ────────────────────────────

#[tokio::test]
async fn as1_un_role_libre_aed_fait_l_appel_et_propose_sans_qualifier() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    // Un profil de base « personnel » : aucun droit propre (RP1-06).
    let aed = cast.comptas[0].user_id;
    let base = Role::Personnel.as_str();
    for a in [Action::AppelFaire, Action::AbsencesProposerQualification, Action::AbsencesQualifier, Action::AppelCorriger] {
        assert!(!ent.a_action_etablissement(etab, aed, base, a).await.unwrap(), "{a:?} sans rôle");
    }
    let droit = |action| Droit { action, portee: Portee::Etablissement, cibles: vec![] };
    let role = ent
        .creer_role_for_actor(etab, dir, "direction", RoleSaisie {
            nom: format!("AED {}", &Uuid::new_v4().simple().to_string()[..6]),
            description: None,
            modele: None,
            droits: vec![droit(Action::AppelFaire), droit(Action::AbsencesProposerQualification), droit(Action::LocalisationPoser)],
        })
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir, "direction", role.id, aed, None).await.unwrap();
    assert!(ent.a_action_etablissement(etab, aed, base, Action::AppelFaire).await.unwrap());
    assert!(ent.a_action_etablissement(etab, aed, base, Action::AbsencesProposerQualification).await.unwrap());
    assert!(ent.a_action_etablissement(etab, aed, base, Action::LocalisationPoser).await.unwrap());
    // … mais il ne qualifie pas, ne corrige pas un appel validé, ne fait pas l'appel des autres.
    for a in [Action::AbsencesQualifier, Action::AppelCorriger, Action::AppelFaireTout, Action::AppelDeleguer,
              Action::AssiduiteRegleGerer, Action::AbsencesLireMotif] {
        assert!(!ent.a_action_etablissement(etab, aed, base, a).await.unwrap(), "{a:?} ne doit pas être accordée");
    }
    let d = ent.droits_appel(etab, aed, base).await.unwrap();
    assert!(d.faire && !d.tout);
    // `appel.faire_tout` inclut `appel.faire` (supervision).
    let d = ent.droits_appel(etab, cast.cpe.user_id, "cpe").await.unwrap();
    assert!(d.faire && d.tout);
    // Famille et élèves : aucun droit paramétrable (garde-fou 1).
    for r in ["parent", "eleve"] {
        let d = ent.droits_appel(etab, cast.parent_lea.user_id, r).await.unwrap();
        assert!(!d.faire && !d.tout, "{r}");
    }
}

// ─── Reprise idempotente (motifs et règle) ──────────────────────────────────

#[tokio::test]
async fn as1_reprise_des_motifs_et_de_la_regle_est_idempotente() {
    let Some((ent, _cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    // Établissement neuf : une règle figée par un autre test ne se supprime pas.
    let etab = Uuid::new_v4();
    let pool = ent.pool_verif();
    // Un établissement « ancien » : règle et motif propres dans les anciennes tables.
    sqlx::query("DELETE FROM ent_asd_regle WHERE etablissement_id = $1").bind(etab).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_vs_regle_assiduite (etablissement_id, heure_coupure, seuil_minutes, seuil_signalement, delai_verification_minutes)
         VALUES ($1, '12:30', 45, 5, 20)
         ON CONFLICT (etablissement_id) DO UPDATE SET heure_coupure = '12:30', seuil_minutes = 45, seuil_signalement = 5, delai_verification_minutes = 20",
    )
    .bind(etab).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO ent_absence_motif_etab (etablissement_id, code, libelle) VALUES ($1, 'permis_conduire', 'Examen du code') ON CONFLICT DO NOTHING")
        .bind(etab).execute(pool).await.unwrap();
    let script = include_str!("../../../migrations/2241_assiduite_as1_referentiels.sql");
    for _ in 0..2 {
        sqlx::raw_sql(script).execute(pool).await.expect("migration rejouable (E8)");
    }
    let (n, s, c, sig, d): (i64, i32, chrono::NaiveTime, i32, i32) = sqlx::query_as(
        "SELECT count(*) OVER (), seuil_minutes, heure_coupure, seuil_signalement_demi_journees, delai_verification_min
           FROM ent_asd_regle WHERE etablissement_id = $1",
    )
    .bind(etab).fetch_one(pool).await.unwrap();
    assert_eq!((n, s, sig, d), (1, 45, 5, 20), "une seule version reprise, valeurs du passé conservées");
    assert_eq!(c, chrono::NaiveTime::from_hms_opt(12, 30, 0).unwrap());
    let (jour,): (chrono::NaiveDate,) = sqlx::query_as("SELECT valide_du FROM ent_asd_regle WHERE etablissement_id = $1")
        .bind(etab).fetch_one(pool).await.unwrap();
    assert_eq!(jour, chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(), "le passé reste calculé avec la règle d'alors");
    let (nm,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_asd_motif WHERE etablissement_id = $1 AND code = 'permis_conduire'")
        .bind(etab).fetch_one(pool).await.unwrap();
    assert_eq!(nm, 1);
    let (nt,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_asd_motif WHERE etablissement_id = $1 AND code = 'appel'")
        .bind(etab).fetch_one(pool).await.unwrap();
    assert_eq!(nt, 0, "le motif technique n'est pas repris (DA-20)");
}
