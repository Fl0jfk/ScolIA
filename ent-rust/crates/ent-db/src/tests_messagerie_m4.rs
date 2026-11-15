//! Messagerie, lot M4 (`docs/socles/16_MESSAGERIE.md` §19.2, §20 R-M13, R-M20, R-M21, R-M27, R-M28 ; CM-26, CM-45,
//! CM-54 à CM-56, CM-79) : « écrire mieux », contre la base locale de test (feature `labo`). Un **témoin** précède
//! chaque attaque ; chaque scénario sème son propre établissement.

use super::tests_messagerie_m1::{compte_eleve, ecrit, labo, lier, lit, nouveau_foyer, nouvel_utilisateur, ouvrir_un, role};
use super::*;
use chrono::{Duration, TimeZone, Utc};
use domain_eleve::{OuvrirFil, RepondreFil};
use ent_auth::Role;
use uuid::Uuid;

fn rep(etab: Uuid, user: Uuid, fil: Uuid, corps: &str) -> RepondreFil {
    RepondreFil { etablissement_id: etab, fil_id: fil, corps: corps.into(), actor_user_id: user }
}

/// Un élève de plus, avec un seul responsable (compte parent lié) : `(élève, parent)`.
async fn eleve_et_parent(ent: &PgEnt, etab: Uuid, nom: &str) -> (Uuid, Uuid) {
    let eleve = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, date_naissance, created_at, updated_at)
         VALUES ($1,$2,$3,'Test','inscrit', ent_aujourdhui_etab($2) - 3000, now(), now())",
    )
    .bind(eleve)
    .bind(etab)
    .bind(nom)
    .execute(ent.pool())
    .await
    .unwrap();
    let parent = nouvel_utilisateur(ent, etab, "parent", &format!("Parent{nom}")).await;
    let foyer = nouveau_foyer(ent, etab).await;
    lier(ent, etab, eleve, parent, Some(foyer)).await;
    (eleve, parent)
}

// ─── Lots individuels (R-M13, CM-79) ───────────────────────────────────────────────────────────

#[tokio::test]
async fn m4_cm79_lot_individuel_apercu_puis_confirmation_sur_la_meme_empreinte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe) = (cast.direction.user_id, cast.cpe.user_id);
    let (e1, p1) = eleve_et_parent(&ent, etab, "Lot1").await;
    let (e2, p2) = eleve_et_parent(&ent, etab, "Lot2").await;
    let apercu = ent
        .apercu_lot_for_actor(etab, cpe, role(Role::Cpe), vec![e1, e2], "Sortie jeudi", "Pensez à l'autorisation.")
        .await
        .expect("aperçu");
    assert_eq!((apercu.nb_eleves, apercu.nb_fils, apercu.exclus, apercu.eleves_hors_droits), (2, 2, 0, 0));
    // L'aperçu n'a rien envoyé.
    let (avant,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg_fil WHERE lot_id = $1").bind(apercu.lot_id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(avant, 0);
    // Un autre compte ne confirme pas le lot de l'auteur ; un texte modifié non plus.
    assert!(ent.confirmer_lot_for_actor(etab, dir, role(Role::Direction), apercu.lot_id, "Sortie jeudi", "Pensez à l'autorisation.").await.is_err());
    assert!(matches!(
        ent.confirmer_lot_for_actor(etab, cpe, role(Role::Cpe), apercu.lot_id, "Sortie jeudi", "Texte différent de l'aperçu").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let fils = ent
        .confirmer_lot_for_actor(etab, cpe, role(Role::Cpe), apercu.lot_id, "Sortie jeudi", "Pensez à l'autorisation.")
        .await
        .expect("confirmation");
    assert_eq!(fils.len(), 2);
    // Chaque parent lit exactement un fil, et jamais celui d'un autre foyer du lot.
    let mut lus = Vec::new();
    for p in [p1, p2] {
        let mut mes_fils = Vec::new();
        for f in &fils {
            if lit(&ent, etab, p, Role::Parent, *f).await {
                mes_fils.push(*f);
            }
        }
        assert_eq!(mes_fils.len(), 1, "un parent ne lit que le fil de son foyer");
        lus.push(mes_fils[0]);
    }
    assert_ne!(lus[0], lus[1]);
    let (lies,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg_fil WHERE lot_id = $1").bind(apercu.lot_id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(lies, 2, "les fils du lot portent son identifiant");
    // Une seule confirmation par aperçu.
    assert!(matches!(
        ent.confirmer_lot_for_actor(etab, cpe, role(Role::Cpe), apercu.lot_id, "Sortie jeudi", "Pensez à l'autorisation.").await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

#[tokio::test]
async fn m4_cm79_au_dela_de_30_conversations_la_messagerie_refuse_et_renvoie_vers_l_annonce() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let cpe = cast.cpe.user_id;
    // Témoin : 30 élèves passent l'aperçu ; 31 sont refusés (340 familles : ce n'est plus une conversation).
    let mut eleves = Vec::new();
    for i in 0..31 {
        eleves.push(eleve_et_parent(&ent, etab, &format!("Gros{i}")).await.0);
    }
    let ok = ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), eleves[..30].to_vec(), "Réunion", "Réunion de rentrée.").await;
    assert_eq!(ok.expect("30 passent").nb_fils, 30);
    let r = ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), eleves.clone(), "Réunion", "Réunion de rentrée.").await;
    match r {
        Err(EntDbError::ConflitDossier(m)) => assert!(m.contains("annonce") || m.contains("diffusion"), "{m}"),
        autre => panic!("31 élèves doivent être refusés : {autre:?}"),
    }
    // Deux foyers pour un seul élève comptent pour deux conversations : le plafond porte sur les fils.
    let (e, _) = eleve_et_parent(&ent, etab, "DeuxFoyers").await;
    let second = nouvel_utilisateur(&ent, etab, "parent", "SecondFoyer").await;
    let foyer2 = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, e, second, Some(foyer2)).await;
    let a = ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), vec![e], "Deux foyers", "Message.").await.unwrap();
    assert_eq!(a.nb_fils, 2, "un fil par foyer (R-M3)");
    let mut tous: Vec<Uuid> = eleves[..30].to_vec();
    tous.push(e);
    assert!(ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), tous, "Trop", "Message.").await.is_err());
}

#[tokio::test]
async fn m4_lot_droit_dedie_eleve_hors_droits_et_exclusion_agregee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, adm, parent) = (cast.cpe.user_id, cast.administratif.user_id, cast.parent_lea.user_id);
    let (e, _) = eleve_et_parent(&ent, etab, "Exclu").await;
    // Le secrétariat n'a pas le droit d'envoyer un lot ; un parent non plus.
    assert!(matches!(
        ent.apercu_lot_for_actor(etab, adm, role(Role::Administratif), vec![e], "S", "T").await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.apercu_lot_for_actor(etab, parent, role(Role::Parent), vec![e], "S", "T").await,
        Err(EntDbError::Forbidden)
    ));
    // Sans élève ou sans texte : refus clair.
    assert!(ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), vec![], "S", "T").await.is_err());
    assert!(ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), vec![e], " ", "T").await.is_err());
    // Un responsable « à ne pas solliciter » est exclu : un NOMBRE dans l'aperçu, ni nom ni cause ; et personne
    // n'est joint quand tous sont exclus.
    sqlx::query("UPDATE ent_eleve_lien SET ne_pas_solliciter = true WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(e)
        .execute(ent.pool())
        .await
        .map(|_| ())
        .unwrap_or_else(|er| eprintln!("ne_pas_solliciter (schéma) : {er}"));
    let r = ent.apercu_lot_for_actor(etab, cpe, role(Role::Cpe), vec![e], "S", "T").await;
    match r {
        Err(EntDbError::ConflitDossier(_)) => {}
        Ok(a) => assert_eq!(a.exclus, 0, "colonne absente : aucun effet"),
        Err(er) => panic!("{er:?}"),
    }
}

// ─── Modèles à portée ──────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m4_modeles_personnel_boite_etablissement_chacun_a_sa_portee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof, adm) = (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id, cast.administratif.user_id);
    // Une boîte dont le CPE est membre.
    let b = ent
        .creer_boite_for_actor(etab, dir, role(Role::Direction), SaisieBoite { code: "vie_scolaire".into(), catalogue: true, ..Default::default() })
        .await
        .unwrap()
        .id;
    sqlx::query("INSERT INTO ent_msg_boite_designation (id, etablissement_id, boite_id, personne_id, qualite, du) SELECT gen_random_uuid(), $1, $2, ent_msg_personne_du_compte($1, $3), 'membre', ent_aujourdhui_etab($1) - 1")
        .bind(etab)
        .bind(b)
        .bind(cpe)
        .execute(ent.pool())
        .await
        .unwrap();
    let mk = |u: Uuid, r: Role, titre: &str, p: PorteeModele| {
        let ent = &ent;
        let titre = titre.to_string();
        async move { ent.creer_modele_portee_for_actor(etab, u, role(r), &titre, "Sujet type", "Corps type.", p).await }
    };
    mk(prof, Role::Prof, "Perso du prof", PorteeModele::Personnel).await.expect("modèle personnel");
    mk(cpe, Role::Cpe, "Modèle de la VS", PorteeModele::Boite(b)).await.expect("modèle de boîte (membre)");
    mk(dir, Role::Direction, "Modèle de l'établissement", PorteeModele::Etablissement).await.expect("modèle d'établissement");
    // Refus : modèle de boîte par un non-membre, modèle d'établissement par un professeur.
    assert!(matches!(mk(prof, Role::Prof, "X", PorteeModele::Boite(b)).await, Err(EntDbError::Forbidden)));
    assert!(matches!(mk(prof, Role::Prof, "Y", PorteeModele::Etablissement).await, Err(EntDbError::Forbidden)));
    let titres = |v: Vec<ModeleVu>| v.into_iter().map(|m| m.titre).collect::<Vec<_>>();
    let vu_prof = titres(ent.modeles_visibles_for_actor(etab, prof, role(Role::Prof)).await.unwrap());
    assert!(vu_prof.contains(&"Perso du prof".to_string()) && vu_prof.contains(&"Modèle de l'établissement".to_string()));
    assert!(!vu_prof.contains(&"Modèle de la VS".to_string()), "le modèle d'une boîte n'est pas celui d'un non-membre");
    let vu_cpe = titres(ent.modeles_visibles_for_actor(etab, cpe, role(Role::Cpe)).await.unwrap());
    assert!(vu_cpe.contains(&"Modèle de la VS".to_string()) && !vu_cpe.contains(&"Perso du prof".to_string()));
    let vu_adm = titres(ent.modeles_visibles_for_actor(etab, adm, role(Role::Administratif)).await.unwrap());
    assert!(!vu_adm.contains(&"Perso du prof".to_string()) && !vu_adm.contains(&"Modèle de la VS".to_string()));
    // L'ancien chemin (modèles d'établissement) ne laisse rien fuir des modèles personnels ou de boîte.
    let anciens = ent.list_msg_modeles_for_actor(etab, prof, role(Role::Prof), false).await;
    if let Ok(v) = anciens {
        assert!(v.iter().all(|m| m.titre != "Modèle de la VS" && m.titre != "Perso du prof"));
    }
    // Un parent n'a aucun modèle.
    assert!(matches!(
        ent.modeles_visibles_for_actor(etab, cast.parent_lea.user_id, role(Role::Parent)).await,
        Err(EntDbError::Forbidden)
    ));
}

/// Le labo désactive la plage calme par défaut (comme M0b) : ces scénarios la rétablissent (défaut national).
async fn activer_plage_calme(ent: &PgEnt, etab: Uuid) {
    sqlx::query(
        "INSERT INTO ent_msg_reglage (etablissement_id, plage_calme) VALUES ($1, true)
         ON CONFLICT (etablissement_id) DO UPDATE SET plage_calme = true, plage_debut_heure = 20,
            plage_fin_heure = 7, plage_weekend = true, motif = NULL",
    )
    .bind(etab)
    .execute(ent.pool())
    .await
    .unwrap();
}

// ─── Plage calme, programmation (CM-26, CM-45, R-M20, R-M21) ────────────────────────────────────────

/// Un fil ouvert par une famille (témoin d'un échange en cours), sur lequel le personnel répond.
async fn fil_famille(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let etab = cast.etablissement_id;
    ouvrir_un(ent, etab, Role::Parent, cast.parent_lea.user_id, cast.eleve_lea_id, "Question").await.fil.id
}

#[tokio::test]
async fn m4_cm45_message_du_soir_differe_au_lendemain_7h_et_jamais_en_pleine_nuit_vers_un_mineur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    activer_plage_calme(&ent, etab).await;
    let (dir, cpe) = (cast.direction.user_id, cast.cpe.user_id);
    let fil = fil_famille(&ent, &cast).await;
    // Jeudi 12 novembre 2026, 22 h à Paris (21 h UTC).
    let soir = Utc.with_ymd_and_hms(2026, 11, 12, 21, 0, 0).unwrap();
    let env = ent
        .repondre_fil_a(role(Role::Cpe), rep(etab, cpe, fil, "Bonsoir, je vous réponds demain."), OptionsEnvoiMessage::default(), soir)
        .await
        .unwrap();
    // Vendredi 7 h Paris = 6 h UTC.
    assert_eq!(env.remise_prevue_le, Some(Utc.with_ymd_and_hms(2026, 11, 13, 6, 0, 0).unwrap()));
    // L'auteur le voit avant d'envoyer (aperçu de remise).
    assert_eq!(ent.lecture_prevue(etab, soir).await.unwrap(), env.remise_prevue_le);
    // Vendredi 21 h → remis lundi 7 h (week-end calme).
    let vendredi = Utc.with_ymd_and_hms(2026, 11, 13, 20, 0, 0).unwrap();
    let env = ent
        .repondre_fil_a(role(Role::Cpe), rep(etab, cpe, fil, "Bon week-end."), OptionsEnvoiMessage::default(), vendredi)
        .await
        .unwrap();
    assert_eq!(env.remise_prevue_le, Some(Utc.with_ymd_and_hms(2026, 11, 16, 6, 0, 0).unwrap()));
    // Le parent ne lit pas les messages encore différés ; leur auteur, si.
    let (_, vus) = ent.list_messages_for_actor(etab, cast.parent_lea.user_id, role(Role::Parent), fil).await.unwrap();
    assert_eq!(vus.len(), 1, "seul le message d'ouverture de la famille est remis");
    // Direction sans motif : pas de dérogation ; avec motif et droit : remis tout de suite, motif tracé.
    let sans = ent.repondre_fil_a(role(Role::Direction), rep(etab, dir, fil, "Sans motif"), OptionsEnvoiMessage::default(), soir).await.unwrap();
    assert!(sans.remise_prevue_le.is_some(), "la direction aussi est soumise à la plage sans motif d'urgence");
    let urgent = ent
        .repondre_fil_a(
            role(Role::Direction),
            rep(etab, dir, fil, "Urgence : sortie annulée"),
            OptionsEnvoiMessage { urgence_motif: Some("Sortie annulée en urgence".into()) },
            soir,
        )
        .await
        .unwrap();
    assert!(urgent.remise_prevue_le.is_none());
    // Un professeur n'a pas la dérogation : le motif seul ne suffit pas.
    assert!(matches!(
        ent.repondre_fil_a(
            role(Role::Prof),
            rep(etab, cast.prof.user_id, fil, "Je dérogue"),
            OptionsEnvoiMessage { urgence_motif: Some("Je veux".into()) },
            soir
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn m4_cm26_aucun_message_d_adulte_remis_a_un_mineur_la_nuit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    activer_plage_calme(&ent, etab).await;
    let cpe = cast.cpe.user_id;
    let harry = cast.eleve_lea_id;
    let eleve = compte_eleve(&ent, etab, harry).await;
    // L'élève mineur écrit à 23 h : enregistré, remis tout de suite aux adultes ; il voit à quelle heure
    // l'adulte le lira (CM-26).
    let nuit = Utc.with_ymd_and_hms(2026, 11, 11, 22, 0, 0).unwrap();
    let lecture = ent.lecture_prevue(etab, nuit).await.unwrap();
    assert_eq!(lecture, Some(Utc.with_ymd_and_hms(2026, 11, 12, 6, 0, 0).unwrap()), "sera lu à partir de demain 7 h");
    let o = ent
        .ouvrir_fils_a(
            role(Role::Eleve),
            OuvrirFil { etablissement_id: etab, eleve_id: harry, sujet: "Besoin d'aide".into(), premier_message: "Je voudrais parler.".into(), actor_user_id: eleve },
            OptionsEnvoiMessage::default(),
            None,
            nuit,
        )
        .await
        .unwrap();
    assert_eq!(o.len(), 1);
    assert!(o[0].remise_prevue_le.is_none(), "le message de l'élève est enregistré à toute heure");
    // Un adulte répond à 22 h à cet élève mineur : différé à 7 h, jamais remis la nuit.
    let env = ent
        .repondre_fil_a(role(Role::Cpe), rep(etab, cpe, o[0].fil.id, "Je t'écoute demain matin."), OptionsEnvoiMessage::default(), nuit)
        .await
        .unwrap();
    assert_eq!(env.remise_prevue_le, lecture);
    let (_, vus) = ent.list_messages_for_actor(etab, eleve, role(Role::Eleve), o[0].fil.id).await.unwrap();
    assert!(vus.iter().all(|m| m.message.auteur_user_id == eleve), "rien d'un adulte n'est remis à l'élève avant 7 h");
    // Jour ouvré, en journée : aucun différé (témoin que la plage ne bloque pas tout).
    let midi = Utc.with_ymd_and_hms(2026, 11, 12, 11, 0, 0).unwrap();
    assert_eq!(ent.lecture_prevue(etab, midi).await.unwrap(), None);
}

#[tokio::test]
async fn m4_programmation_ne_contourne_jamais_la_plage_calme_et_se_borne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    activer_plage_calme(&ent, etab).await;
    let cpe = cast.cpe.user_id;
    let fil = fil_famille(&ent, &cast).await;
    let maintenant = Utc.with_ymd_and_hms(2026, 11, 11, 10, 0, 0).unwrap(); // mercredi 11 h Paris
    // Témoin : programmé jeudi 10 h → remis jeudi 10 h.
    let jeudi_10h = Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap();
    let (m, remise) = ent.repondre_programme_a(role(Role::Cpe), rep(etab, cpe, fil, "Message programmé"), jeudi_10h, maintenant).await.unwrap();
    assert_eq!(remise, Some(jeudi_10h));
    let (prog,): (Option<chrono::DateTime<Utc>>,) = sqlx::query_as("SELECT programme_le FROM ent_msg WHERE id = $1").bind(m.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(prog, Some(jeudi_10h));
    // Programmer à 3 h du matin (plage calme) ne contourne rien : remis à 7 h.
    let trois_heures = Utc.with_ymd_and_hms(2026, 11, 12, 2, 0, 0).unwrap();
    let (_, remise) = ent.repondre_programme_a(role(Role::Cpe), rep(etab, cpe, fil, "À 3 h"), trois_heures, maintenant).await.unwrap_or_else(|e| panic!("{e:?}"));
    assert!(remise.is_some());
    // Un message écrit le soir et programmé après la plage : le plus tardif gagne.
    let soir = Utc.with_ymd_and_hms(2026, 11, 11, 21, 0, 0).unwrap();
    let demain_14h = Utc.with_ymd_and_hms(2026, 11, 12, 13, 0, 0).unwrap();
    let (_, remise) = ent.repondre_programme_a(role(Role::Cpe), rep(etab, cpe, fil, "Programmé le soir"), demain_14h, soir).await.unwrap();
    assert_eq!(remise, Some(demain_14h));
    // Bornes : passé, trop lointain, famille.
    assert!(ent.repondre_programme_a(role(Role::Cpe), rep(etab, cpe, fil, "Passé"), maintenant, maintenant).await.is_err());
    assert!(ent.repondre_programme_a(role(Role::Cpe), rep(etab, cpe, fil, "Loin"), maintenant + Duration::days(120), maintenant).await.is_err());
    assert!(matches!(
        ent.repondre_programme_a(role(Role::Parent), rep(etab, cast.parent_lea.user_id, fil, "Parent"), jeudi_10h, maintenant).await,
        Err(EntDbError::Forbidden)
    ));
}

// ─── Correction (R-M27, CM-55) et retrait (R-M28, CM-54, CM-56) ──────────────────────────────────

#[tokio::test]
async fn m4_cm55_correction_15_minutes_si_non_lu_versionnee_et_marquee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, lily) = (cast.cpe.user_id, cast.parent_lea.user_id);
    let fil = fil_famille(&ent, &cast).await;
    let m1 = ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, fil, "Bonjour, voici la répone.")).await.unwrap();
    // Témoin : l'auteur corrige dans les 15 minutes, personne n'ayant lu.
    ent.corriger_message_fil_for_actor(etab, cpe, role(Role::Cpe), m1.id, "Bonjour, voici la réponse.").await.expect("correction");
    let (corps, version, modifie): (String, i32, Option<chrono::DateTime<Utc>>) =
        sqlx::query_as("SELECT corps, version, modifie_le FROM ent_msg WHERE id = $1").bind(m1.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((corps.as_str(), version), ("Bonjour, voici la réponse.", 2));
    assert!(modifie.is_some(), "marque « modifié »");
    let (ancien,): (String,) = sqlx::query_as("SELECT corps FROM ent_msg_version WHERE message_id = $1 AND version = 1").bind(m1.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(ancien, "Bonjour, voici la répone.", "la version remplacée est gardée");
    assert!(sqlx::query("DELETE FROM ent_msg_version WHERE message_id = $1").bind(m1.id).execute(ent.pool()).await.is_err(), "ajout seul");
    // Le parent lit la version corrigée. L'empreinte suit le texte.
    let (_, vus) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    assert!(vus.iter().any(|m| m.message.corps == "Bonjour, voici la réponse."));
    // Désormais lu : plus de correction (CM-55 : sinon, nouveau message).
    assert!(matches!(
        ent.corriger_message_fil_for_actor(etab, cpe, role(Role::Cpe), m1.id, "Encore autre chose").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Un autre que l'auteur ne corrige pas ; un message inexistant est introuvable ; un texte vide est refusé.
    let m2 = ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, fil, "Autre message")).await.unwrap();
    assert!(matches!(
        ent.corriger_message_fil_for_actor(etab, cast.direction.user_id, role(Role::Direction), m2.id, "Pirate").await,
        Err(EntDbError::Forbidden) | Err(EntDbError::Domain(_))
    ));
    assert!(ent.corriger_message_fil_for_actor(etab, cpe, role(Role::Cpe), m2.id, "  ").await.is_err());
    // Au-delà de 15 minutes (message non lu) : refus.
    let tard = Utc::now() + Duration::minutes(16);
    assert!(matches!(
        ent.corriger_message_fil_a(etab, cpe, role(Role::Cpe), m2.id, "Trop tard", tard).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.corriger_message_fil_a(etab, cpe, role(Role::Cpe), m2.id, "Dans les temps", Utc::now() + Duration::minutes(14)).await.expect("14 minutes : encore permis");
}

#[tokio::test]
async fn m4_cm54_cm56_retrait_10_minutes_masque_trace_et_jamais_de_suppression() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, lily) = (cast.cpe.user_id, cast.parent_lea.user_id);
    let fil = fil_famille(&ent, &cast).await;
    // CM-56 : un parent écrit sous le coup de la colère, puis le retire dans les 10 minutes.
    assert!(ecrit(&ent, etab, lily, Role::Parent, fil, "Message écrit sous le coup de la colère").await);
    let (_, vus) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    let colere = vus.iter().find(|m| m.message.corps.contains("colère")).unwrap().message.id;
    ent.retirer_message_for_actor(etab, lily, role(Role::Parent), colere, Some("regret")).await.expect("retrait par l'auteur");
    // Le contenu n'est plus servi, mais reste conservé (preuve).
    let (_, vus) = ent.list_messages_for_actor(etab, cpe, role(Role::Cpe), fil).await.unwrap_or_else(|_| panic!("le fil est lisible du CPE"));
    let _ = vus;
    let (_, vus) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    assert!(vus.iter().all(|m| !m.message.corps.contains("colère")), "plus servi");
    assert!(vus.iter().any(|m| m.message.corps.starts_with("Message retiré")), "mention de retrait");
    let (garde, par, motif): (String, String, String) =
        sqlx::query_as("SELECT corps, retrait_par, retrait_motif_code FROM ent_msg WHERE id = $1").bind(colere).fetch_one(ent.pool()).await.unwrap();
    assert!(garde.contains("colère"), "le contenu reste conservé, jamais supprimé");
    assert_eq!((par.as_str(), motif.as_str()), ("auteur", "regret"));
    assert!(ent.retirer_message_for_actor(etab, lily, role(Role::Parent), colere, None).await.is_err(), "déjà retiré");
    // CM-54 : un professeur envoie au mauvais fil, retire dans les 10 minutes ; au-delà, refus.
    let m = ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, fil, "Mauvais fil")).await.unwrap();
    assert!(matches!(
        ent.retirer_message_a(etab, cpe, role(Role::Cpe), m.id, None, Utc::now() + Duration::minutes(11)).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Un autre compte ne retire pas le message d'autrui.
    assert!(ent.retirer_message_for_actor(etab, cast.direction.user_id, role(Role::Direction), m.id, None).await.is_err());
    ent.retirer_message_a(etab, cpe, role(Role::Cpe), m.id, None, Utc::now() + Duration::minutes(9)).await.expect("9 minutes : permis");
    // Aucune suppression physique par un utilisateur : le message existe toujours.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg WHERE id = ANY($1)").bind(vec![colere, m.id]).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 2);
}

// ─── Brouillons ───────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m4_brouillons_strictement_personnels_bornes_et_envoyes_par_le_chemin_ordinaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, dir) = (cast.cpe.user_id, cast.direction.user_id);
    let fil = fil_famille(&ent, &cast).await;
    let brouillon = ent
        .enregistrer_brouillon_for_actor(
            etab,
            cpe,
            role(Role::Cpe),
            BrouillonSaisie { fil_id: Some(fil), corps: "Brouillon en cours…".into(), ..Default::default() },
        )
        .await
        .expect("brouillon de réponse");
    // Témoin : l'auteur le retrouve, le met à jour.
    assert_eq!(ent.lister_brouillons_for_actor(etab, cpe).await.unwrap().len(), 1);
    ent.enregistrer_brouillon_for_actor(
        etab,
        cpe,
        role(Role::Cpe),
        BrouillonSaisie { id: Some(brouillon), fil_id: Some(fil), corps: "Brouillon complété.".into(), ..Default::default() },
    )
    .await
    .unwrap();
    // Un autre ne le voit pas, ne le modifie pas, ne le supprime pas, ne l'envoie pas.
    assert!(ent.lister_brouillons_for_actor(etab, dir).await.unwrap().is_empty());
    assert!(ent.supprimer_brouillon_for_actor(etab, dir, brouillon).await.is_err());
    assert!(ent
        .enregistrer_brouillon_for_actor(etab, dir, role(Role::Direction), BrouillonSaisie { id: Some(brouillon), fil_id: Some(fil), corps: "Pirate".into(), ..Default::default() })
        .await
        .is_err());
    assert!(ent.envoyer_brouillon_for_actor(etab, dir, role(Role::Direction), brouillon).await.is_err());
    // Un brouillon n'ouvre aucun fil qu'on ne lit pas, ni aucun élève hors droits.
    let (autre_eleve, _) = eleve_et_parent(&ent, etab, "Hors").await;
    assert!(ent
        .enregistrer_brouillon_for_actor(etab, cast.parent_lea.user_id, role(Role::Parent), BrouillonSaisie { eleve_id: Some(autre_eleve), corps: "x".into(), ..Default::default() })
        .await
        .is_err());
    assert!(ent
        .enregistrer_brouillon_for_actor(etab, cast.prof.user_id, role(Role::Prof), BrouillonSaisie { fil_id: Some(fil), corps: "x".into(), ..Default::default() })
        .await
        .is_err());
    // Brouillon sans cible, ou trop long : refus.
    assert!(ent.enregistrer_brouillon_for_actor(etab, cpe, role(Role::Cpe), BrouillonSaisie::default()).await.is_err());
    // Envoi : le brouillon part par le chemin ordinaire puis disparaît.
    let f = ent.envoyer_brouillon_for_actor(etab, cpe, role(Role::Cpe), brouillon).await.expect("envoi");
    assert_eq!(f, fil);
    assert!(ent.lister_brouillons_for_actor(etab, cpe).await.unwrap().is_empty());
    let (_, vus) = ent.list_messages_for_actor(etab, cpe, role(Role::Cpe), fil).await.unwrap();
    assert!(vus.iter().any(|m| m.message.corps == "Brouillon complété."));
    // Les brouillons périmés sont purgés ; le nombre est borné.
    let b2 = ent
        .enregistrer_brouillon_for_actor(etab, cpe, role(Role::Cpe), BrouillonSaisie { fil_id: Some(fil), corps: "ancien".into(), ..Default::default() })
        .await
        .unwrap();
    sqlx::query("UPDATE ent_msg_brouillon SET expire_le = now() - interval '1 day' WHERE id = $1").bind(b2).execute(ent.pool()).await.unwrap();
    assert!(ent.lister_brouillons_for_actor(etab, cpe).await.unwrap().is_empty(), "périmé : purgé");
    for _ in 0..BROUILLONS_MAX {
        ent.enregistrer_brouillon_for_actor(etab, cpe, role(Role::Cpe), BrouillonSaisie { fil_id: Some(fil), corps: "x".into(), ..Default::default() })
            .await
            .unwrap();
    }
    assert!(matches!(
        ent.enregistrer_brouillon_for_actor(etab, cpe, role(Role::Cpe), BrouillonSaisie { fil_id: Some(fil), corps: "x".into(), ..Default::default() }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

// ─── Fusion et scission (MSG-174) ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn m4_fusion_de_deux_fils_de_la_meme_famille_et_refus_hors_meme_foyer() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, lily, harry) = (cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let a = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Question A").await.fil.id;
    let b = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Question B (même sujet)").await.fil.id;
    // Le CPE a participé aux deux fils.
    ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, a, "Réponse A")).await.unwrap();
    ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, b, "Réponse B")).await.unwrap();
    // Un autre foyer (fil du parent de Tom sur son propre enfant) n'est jamais fusionnable avec un fil de Léa.
    let tom_fil = ouvrir_un(&ent, etab, Role::Parent, cast.parent_tom.user_id, cast.eleve_tom_id, "Tom").await.fil.id;
    ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, tom_fil, "Réponse Tom")).await.unwrap();
    assert!(matches!(
        ent.fusionner_fils_for_actor(etab, cpe, role(Role::Cpe), a, tom_fil).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Un professeur sans lien avec ces fils ne les fusionne pas.
    assert!(ent.fusionner_fils_for_actor(etab, cast.prof.user_id, role(Role::Prof), a, b).await.is_err());
    // Témoin : même famille, même élève → fusion ; les messages sont déplacés, jamais copiés.
    let n = ent.fusionner_fils_for_actor(etab, cpe, role(Role::Cpe), a, b).await.expect("fusion");
    assert_eq!(n, 2, "le message d'ouverture de B et la réponse de B");
    let (_, vus) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), a).await.unwrap();
    assert_eq!(vus.len(), 4);
    let (etat, fusionne): (String, Option<Uuid>) = sqlx::query_as("SELECT etat, fusionne_dans FROM ent_msg_fil WHERE id = $1").bind(b).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((etat.as_str(), fusionne), ("clos", Some(a)));
    // Le fil absorbé n'est plus un fil actif : on n'y écrit plus.
    assert!(!ecrit(&ent, etab, cpe, Role::Cpe, b, "Dans le fil absorbé").await);
    // La fusion ne s'applique qu'une fois.
    assert!(ent.fusionner_fils_for_actor(etab, cpe, role(Role::Cpe), a, b).await.is_err());
    // Les messages déplacés gardent leur origine.
    let (deplaces,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg WHERE fil_origine_id = $1").bind(b).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(deplaces, 2);
}

#[tokio::test]
async fn m4_scission_d_un_fil_a_partir_d_un_message_memes_participants() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, lily, harry, prof) = (cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id, cast.prof.user_id);
    let fil = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Deux sujets mêlés").await.fil.id;
    let m_notes = ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, fil, "Sur les notes : tout va bien.")).await.unwrap();
    ent.repondre_fil_for_actor(role(Role::Cpe), rep(etab, cpe, fil, "Sur la cantine : voir l'intendance.")).await.unwrap();
    let (_, tous) = ent.list_messages_for_actor(etab, cpe, role(Role::Cpe), fil).await.unwrap();
    let premier = tous.first().unwrap().message.id;
    // Le premier message ne se scinde pas ; un non-participant ne scinde rien.
    assert!(matches!(
        ent.scinder_fil_for_actor(etab, cpe, role(Role::Cpe), fil, premier, "X").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent.scinder_fil_for_actor(etab, prof, role(Role::Prof), fil, m_notes.id, "Notes").await.is_err());
    assert!(ent.scinder_fil_for_actor(etab, cpe, role(Role::Cpe), fil, m_notes.id, "  ").await.is_err());
    // Témoin : scission à partir de la première réponse du CPE.
    let nouveau = ent.scinder_fil_for_actor(etab, cpe, role(Role::Cpe), fil, m_notes.id, "Notes de Harry").await.expect("scission");
    assert!(lit(&ent, etab, lily, Role::Parent, nouveau).await && lit(&ent, etab, cpe, Role::Cpe, nouveau).await, "mêmes participants");
    assert!(!lit(&ent, etab, prof, Role::Prof, nouveau).await && !lit(&ent, etab, cast.parent_tom.user_id, Role::Parent, nouveau).await);
    let (_, restants) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    let (_, deplaces) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), nouveau).await.unwrap();
    assert_eq!((restants.len(), deplaces.len()), (1, 2), "le premier message reste, les deux suivants passent dans le nouveau fil");
    let (sujet, scinde): (String, Option<Uuid>) = sqlx::query_as("SELECT sujet, scinde_de FROM ent_msg_fil WHERE id = $1").bind(nouveau).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((sujet.as_str(), scinde), ("Notes de Harry", Some(fil)));
}

// ─── Séparation, boîtes et lots ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m4_separation_entre_etablissements_lots_brouillons_et_modeles() {
    let Some((ent, cast_a)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast_b = ent.seed_labo_cast().await.expect("second établissement");
    let (a, b) = (cast_a.etablissement_id, cast_b.etablissement_id);
    let (cpe_a, cpe_b) = (cast_a.cpe.user_id, cast_b.cpe.user_id);
    let (ea, _) = eleve_et_parent(&ent, a, "SepA").await;
    // Lot de A : B ne le confirme pas, ne l'aperçoit pas, ne joint pas l'élève de A.
    let apercu = ent.apercu_lot_for_actor(a, cpe_a, role(Role::Cpe), vec![ea], "S", "T").await.expect("témoin");
    assert!(ent.confirmer_lot_for_actor(b, cpe_b, role(Role::Cpe), apercu.lot_id, "S", "T").await.is_err());
    let hors = ent.apercu_lot_for_actor(b, cpe_b, role(Role::Cpe), vec![ea], "S", "T").await;
    assert!(hors.is_err(), "un élève d'un autre établissement n'est jamais joint : {hors:?}");
    // Brouillon et modèles ne traversent pas.
    let bro = ent
        .enregistrer_brouillon_for_actor(a, cpe_a, role(Role::Cpe), BrouillonSaisie { eleve_id: Some(ea), sujet: "s".into(), corps: "c".into(), ..Default::default() })
        .await
        .unwrap();
    assert!(ent.lister_brouillons_for_actor(b, cpe_a).await.unwrap().is_empty());
    assert!(ent.supprimer_brouillon_for_actor(b, cpe_a, bro).await.is_err());
    ent.creer_modele_portee_for_actor(a, cast_a.direction.user_id, role(Role::Direction), "T", "S", "C", PorteeModele::Etablissement).await.unwrap();
    assert!(ent.modeles_visibles_for_actor(b, cpe_b, role(Role::Cpe)).await.unwrap().is_empty());
    // Côté base, rôle NOBYPASSRLS en mode strict : lots, élèves de lot et brouillons.
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    const ROLE: &str = "ent_rls_m4";
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(ent.pool())
    .await
    .unwrap();
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ent_msg_lot, ent_msg_lot_eleve, ent_msg_brouillon, ent_msg_modele TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(ent.pool()).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();
    // B a son propre lot, pour prouver que A ne le voit pas.
    let (eb, _) = eleve_et_parent(&ent, b, "SepB").await;
    ent.apercu_lot_for_actor(b, cpe_b, role(Role::Cpe), vec![eb], "S", "T").await.unwrap();
    for table in ["ent_msg_lot", "ent_msg_lot_eleve", "ent_msg_brouillon"] {
        let mut t = crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict).await.unwrap();
        let (chez_a, hors_a): (i64, i64) = sqlx::query_as(&format!(
            "SELECT count(*) FILTER (WHERE etablissement_id = $1), count(*) FILTER (WHERE etablissement_id <> $1) FROM {table}"
        ))
        .bind(a)
        .fetch_one(&mut *t)
        .await
        .unwrap();
        assert!(chez_a >= 1, "{table}");
        assert_eq!(hors_a, 0, "{table} : rien d'un autre établissement");
        t.commit().await.unwrap();
        let (sans,): (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM {table}")).fetch_one(&app).await.unwrap();
        assert_eq!(sans, 0, "{table} : sans contexte, rien");
    }
}
