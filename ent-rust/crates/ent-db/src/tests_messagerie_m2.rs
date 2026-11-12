//! Messagerie, lot M2 (`docs/socles/16_MESSAGERIE.md` §8, §9, R-M3, R-M5 à R-M8, R-M12) : matrice
//! et destinataires, contre la base locale de test (feature `labo`). Même discipline que M0a, M0b
//! et M1 : un **témoin** précède chaque attaque, chaque scénario sème son propre établissement.
//! La décision pure de la matrice est testée sans base dans `msg_matrice::tests`.

use super::tests_messagerie_m1::{
    compte_eleve as _compte_eleve, compter, corps_du_fil, ecrit, labo, lier, lit, nouveau_foyer,
    nouvel_utilisateur, ouvrir, ouvrir_un, participants, role,
};
use super::*;
use crate::msg_matrice::{
    AeshVersFamille, Cible, Decision, EleveVersEleve, Raison, Refus, Reglage, ReglagesMatrice,
};
use chrono::Duration;
use domain_eleve::AttacherPiece;
use ent_auth::Role;
use uuid::Uuid;

async fn eleve_majeur(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, date_naissance, created_at, updated_at)
         VALUES ($1,$2,$3,'Test','inscrit',$4, now(), now())",
    )
    .bind(id)
    .bind(etab)
    .bind(nom)
    .bind(domain_eleve::horloge::jour_paris() - Duration::days(365 * 19 + 10))
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn personne_du_lien(ent: &PgEnt, etab: Uuid, eleve: Uuid, user: Uuid) -> Uuid {
    let (p,): (Uuid,) = sqlx::query_as(
        "SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("personne du lien");
    p
}

async fn permis(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    r: Role,
    cible: Cible,
    eleve: Uuid,
    lien: Option<Uuid>,
    ouverture: bool,
) -> Decision {
    ent.peut_ecrire_for_actor(etab, user, role(r), cible, eleve, lien, ouverture)
        .await
        .expect("peut_ecrire")
}

// ─── Les sept réglages bornés ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn m2_reglages_bornes_reserves_a_la_direction_et_historises() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, prof) = (cast.direction.user_id, cast.prof.user_id);
    // Défaut §8.2 tant que rien n'est enregistré.
    assert_eq!(
        ent.reglages_matrice(etab).await.unwrap(),
        ReglagesMatrice::default()
    );
    // Un professeur ne règle rien ; la direction, si.
    let ferme = ReglagesMatrice {
        familles_vers_professeurs: false,
        eleves_vers_eleves: EleveVersEleve::Delegues,
        aesh_vers_familles: AeshVersFamille::Ouvrir,
        ..ReglagesMatrice::default()
    };
    assert!(matches!(
        ent.definir_reglages_matrice_for_actor(etab, prof, role(Role::Prof), ferme)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert_eq!(
        ent.reglages_matrice(etab).await.unwrap(),
        ReglagesMatrice::default(),
        "rien n'a changé"
    );
    let lu = ent
        .definir_reglages_matrice_for_actor(etab, dir, role(Role::Direction), ferme)
        .await
        .expect("direction");
    assert_eq!(lu, ferme);
    // Deuxième modification : l'ancienne valeur est conservée (historique en ajout seul).
    ent.definir_reglages_matrice_for_actor(
        etab,
        dir,
        role(Role::Direction),
        ReglagesMatrice::default(),
    )
    .await
    .unwrap();
    let (n, vers_profs): (i64, bool) = sqlx::query_as(
        "SELECT count(*), bool_and(familles_vers_professeurs) FROM ent_msg_reglage_version WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 2, "deux valeurs remplacées : la ligne de départ, puis la valeur fermée");
    assert!(!vers_profs, "la valeur fermée remplacée est bien conservée");
    let r = sqlx::query("DELETE FROM ent_msg_reglage_version WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await;
    assert!(r.is_err(), "l'historique est en ajout seul");
    // La base refuse toute valeur hors des bornes (jamais une liste libre).
    let r = sqlx::query(
        "UPDATE ent_msg_reglage SET eleves_vers_eleves = 'tous' WHERE etablissement_id = $1",
    )
    .bind(etab)
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "valeur hors bornes");
}

#[tokio::test]
async fn m2_pieces_des_familles_fermees_par_reglage() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (
        cast.direction.user_id,
        cast.parent_lea.user_id,
        cast.eleve_lea_id,
    );
    let ouvert = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Certificat").await;
    let piece = |user: Uuid| AttacherPiece {
        etablissement_id: etab,
        message_id: ouvert.message.id,
        nom: "certificat.pdf".into(),
        mime_type: "application/pdf".into(),
        contenu: b"%PDF-1.4".to_vec(),
        actor_user_id: user,
    };
    assert!(
        ent.attacher_piece_for_actor(role(Role::Parent), piece(lily))
            .await
            .is_ok(),
        "témoin : réglage par défaut"
    );
    ent.definir_reglages_matrice_for_actor(
        etab,
        dir,
        role(Role::Direction),
        ReglagesMatrice {
            pieces_familles: false,
            ..ReglagesMatrice::default()
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.attacher_piece_for_actor(role(Role::Parent), piece(lily))
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Le personnel garde les pièces (le réglage ne vise que les familles et les élèves).
    let reponse = ent
        .repondre_fil_for_actor(
            role(Role::Direction),
            domain_eleve::RepondreFil {
                etablissement_id: etab,
                fil_id: ouvert.fil.id,
                corps: "Bien reçu".into(),
                actor_user_id: dir,
            },
        )
        .await
        .unwrap();
    let mut p = piece(dir);
    p.message_id = reponse.id;
    assert!(ent.attacher_piece_for_actor(role(Role::Direction), p).await.is_ok());
}

#[tokio::test]
async fn m2_secretariat_vers_eleve_majeur_selon_le_reglage() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, adm) = (cast.direction.user_id, cast.administratif.user_id);
    let yasmine = eleve_majeur(&ent, etab, "Yasmine").await;
    assert_eq!(
        permis(
            &ent,
            etab,
            adm,
            Role::Administratif,
            Cible::Eleve,
            yasmine,
            None,
            true
        )
        .await,
        Decision::Permis(Raison::ServiceDeLEtablissement),
        "témoin : permis par défaut"
    );
    assert_eq!(
        ouvrir(
            &ent,
            etab,
            Role::Administratif,
            adm,
            yasmine,
            "Votre dossier"
        )
        .await
        .len(),
        1,
        "témoin"
    );
    ent.definir_reglages_matrice_for_actor(
        etab,
        dir,
        role(Role::Direction),
        ReglagesMatrice {
            secretariat_vers_majeurs: false,
            ..ReglagesMatrice::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        permis(
            &ent,
            etab,
            adm,
            Role::Administratif,
            Cible::Eleve,
            yasmine,
            None,
            true
        )
        .await,
        Decision::Refuse(Refus::Reglage(Reglage::SecretariatVersMajeurs))
    );
    let r = ent
        .ouvrir_fils_for_actor(
            role(Role::Administratif),
            domain_eleve::OuvrirFil {
                etablissement_id: etab,
                eleve_id: yasmine,
                sujet: "Encore".into(),
                premier_message: "Bonjour".into(),
                actor_user_id: adm,
            },
            OptionsEnvoiMessage::default(),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "le serveur recalcule la matrice à l'ouverture"
    );
    // La direction n'est pas concernée par ce réglage.
    assert_eq!(
        ouvrir(
            &ent,
            etab,
            Role::Direction,
            dir,
            yasmine,
            "Votre orientation"
        )
        .await
        .len(),
        1
    );
}

// ─── La matrice branchée sur les faits ──────────────────────────────────────────────────────

#[tokio::test]
async fn m2_matrice_sur_les_faits_perimetre_reglage_interdiction_ne_pas_solliciter() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let (dir, prof) = (cast.direction.user_id, cast.prof.user_id);
    let lily = cast.parent_lea.user_id;

    // Périmètre (D-022 à la date, CM-38) : un professeur n'écrit qu'aux familles de ses élèves.
    assert_eq!(
        permis(
            &ent,
            etab,
            prof,
            Role::Prof,
            Cible::Famille,
            harry,
            None,
            true
        )
        .await,
        Decision::Permis(Raison::ProfesseurDuCours),
        "témoin : Harry est dans sa classe"
    );
    assert_eq!(
        permis(
            &ent,
            etab,
            prof,
            Role::Prof,
            Cible::Famille,
            draco,
            None,
            true
        )
        .await,
        Decision::Refuse(Refus::HorsPerimetre),
        "Draco n'est pas dans ses classes"
    );
    // Famille → professeur de son enfant : réglage de l'établissement ; jamais vers une autre famille.
    assert_eq!(
        permis(
            &ent,
            etab,
            lily,
            Role::Parent,
            Cible::Professeur,
            harry,
            None,
            true
        )
        .await,
        Decision::Permis(Raison::LienFamilial)
    );
    assert_eq!(
        permis(
            &ent,
            etab,
            lily,
            Role::Parent,
            Cible::AutreFamille,
            harry,
            None,
            true
        )
        .await,
        Decision::Refuse(Refus::FamilleVersFamille)
    );
    assert_eq!(
        permis(
            &ent,
            etab,
            lily,
            Role::Parent,
            Cible::Professeur,
            draco,
            None,
            true
        )
        .await,
        Decision::Refuse(Refus::HorsPerimetre),
        "pas de lien avec Draco : pas de professeur de Draco"
    );
    ent.definir_reglages_matrice_for_actor(
        etab,
        dir,
        role(Role::Direction),
        ReglagesMatrice {
            familles_vers_professeurs: false,
            ..ReglagesMatrice::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        permis(
            &ent,
            etab,
            lily,
            Role::Parent,
            Cible::Professeur,
            harry,
            None,
            true
        )
        .await,
        Decision::Refuse(Refus::Reglage(Reglage::FamillesVersProfesseurs))
    );

    // « Ne pas solliciter » (CM-11) : on n'ouvre plus, on répond.
    let personne = personne_du_lien(&ent, etab, harry, lily).await;
    assert!(
        permis(
            &ent,
            etab,
            dir,
            Role::Direction,
            Cible::Famille,
            harry,
            Some(lily),
            true
        )
        .await
        .est_permis(),
        "témoin"
    );
    ent.definir_ne_pas_solliciter_for_actor(etab, dir, role(Role::Direction), personne, true, None)
        .await
        .expect("ne pas solliciter");
    assert_eq!(
        permis(
            &ent,
            etab,
            dir,
            Role::Direction,
            Cible::Famille,
            harry,
            Some(lily),
            true
        )
        .await,
        Decision::Refuse(Refus::DestinataireExclu)
    );
    assert!(
        permis(
            &ent,
            etab,
            dir,
            Role::Direction,
            Cible::Famille,
            harry,
            Some(lily),
            false
        )
        .await
        .est_permis(),
        "la réponse reste permise"
    );
    ent.definir_ne_pas_solliciter_for_actor(
        etab,
        dir,
        role(Role::Direction),
        personne,
        false,
        None,
    )
    .await
    .unwrap();

    // Interdiction de communication (CM-05) : refus neutre, même en réponse, et pour la personne visée.
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction
            (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4, ARRAY['communication'], 'decision_justice', 'ne pas communiquer',
                 ent_aujourdhui_etab($2) - 1, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(harry)
    .bind(personne)
    .bind(dir)
    .execute(ent.pool())
    .await
    .unwrap();
    for ouverture in [true, false] {
        assert_eq!(
            permis(
                &ent,
                etab,
                dir,
                Role::Direction,
                Cible::Famille,
                harry,
                Some(lily),
                ouverture
            )
            .await,
            Decision::Refuse(Refus::DestinataireExclu),
            "ouverture = {ouverture}"
        );
    }
    assert_eq!(
        permis(
            &ent,
            etab,
            lily,
            Role::Parent,
            Cible::BoiteVieScolaire,
            harry,
            None,
            true
        )
        .await,
        Decision::Refuse(Refus::DestinataireExclu),
        "la personne visée n'écrit plus non plus"
    );
    // Les fils d'un autre enfant ne sont pas visés (CM-05).
    assert!(permis(
        &ent,
        etab,
        cast.parent_tom.user_id,
        Role::Parent,
        Cible::BoiteVieScolaire,
        draco,
        None,
        true
    )
    .await
    .est_permis());
}

// ─── Aperçu des destinataires (R-M5) et ne pas solliciter (R-M7) ────────────────────────────

#[tokio::test]
async fn m2_apercu_sans_coordonnees_un_fil_par_foyer_exclusions_agregees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, prof) = (cast.eleve_lea_id, cast.direction.user_id, cast.prof.user_id);
    let lily = cast.parent_lea.user_id;
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    let foyer_james = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, harry, james, Some(foyer_james)).await;
    // Des coordonnées existent bien en base (sinon l'absence de fuite ne prouverait rien).
    sqlx::query("UPDATE ent_personne SET email = 'secret@exemple.fr', telephone = '0612345678', adresse = '12 rue du Secret' WHERE etablissement_id = $1 AND user_id = ANY($2)")
        .bind(etab)
        .bind(vec![lily, james])
        .execute(ent.pool())
        .await
        .unwrap();

    let apercu = ent
        .apercu_destinataires_for_actor(etab, dir, role(Role::Direction), harry)
        .await
        .expect("aperçu");
    assert_eq!(
        apercu.conversations.len(),
        2,
        "deux foyers : deux conversations (R-M3)"
    );
    assert_eq!(apercu.exclus, 0);
    assert!(apercu.decision.est_permis());
    let rendu = serde_json::to_string(&apercu).unwrap();
    for interdit in ["secret@", "0612345678", "rue du Secret", "@"] {
        assert!(
            !rendu.contains(interdit),
            "coordonnée dans l'aperçu : {interdit} — {rendu}"
        );
    }
    // Un professeur hors périmètre n'obtient pas l'aperçu d'un élève qui n'est pas le sien.
    assert!(matches!(
        ent.apercu_destinataires_for_actor(etab, prof, role(Role::Prof), cast.eleve_tom_id)
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Un parent n'a aucun aperçu (il n'a pas d'annuaire des familles).
    assert!(matches!(
        ent.apercu_destinataires_for_actor(etab, lily, role(Role::Parent), harry)
            .await,
        Err(EntDbError::Forbidden)
    ));

    // « Ne pas solliciter » sur James : une seule conversation, une exclusion **agrégée** (ni nom ni cause).
    let personne_james = personne_du_lien(&ent, etab, harry, james).await;
    ent.definir_ne_pas_solliciter_for_actor(
        etab,
        dir,
        role(Role::Direction),
        personne_james,
        true,
        None,
    )
    .await
    .unwrap();
    let apercu = ent
        .apercu_destinataires_for_actor(etab, dir, role(Role::Direction), harry)
        .await
        .unwrap();
    assert_eq!(apercu.conversations.len(), 1);
    assert_eq!(apercu.exclus, 1);
    let rendu = serde_json::to_string(&apercu).unwrap();
    assert!(
        !rendu.contains("James"),
        "le responsable exclu n'est pas nommé : {rendu}"
    );

    // R-M7 : aucun fil ouvert par l'établissement vers lui ; il n'a donc aucun participant.
    let fils = ouvrir(&ent, etab, Role::Direction, dir, harry, "Sortie de classe").await;
    assert_eq!(fils.len(), 1);
    let (n_james,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_msg_participant p JOIN ent_personne pe ON pe.id = p.personne_id
          WHERE p.fil_id = $1 AND pe.user_id = $2",
    )
    .bind(fils[0].fil.id)
    .bind(james)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        n_james, 0,
        "R-M7 : « ne pas solliciter » n'entre pas dans un fil ouvert par l'établissement"
    );
    assert!(
        lit(&ent, etab, lily, Role::Parent, fils[0].fil.id).await,
        "témoin : le foyer sollicitable le lit"
    );
    // Il répond encore dans le fil qu'il ouvre lui-même.
    let sien = ouvrir_un(&ent, etab, Role::Parent, james, harry, "Question de James").await;
    assert!(
        ecrit(
            &ent,
            etab,
            dir,
            Role::Direction,
            sien.fil.id,
            "Nous vous répondons."
        )
        .await
    );
    assert!(
        lit(&ent, etab, james, Role::Parent, sien.fil.id).await,
        "il lit la réponse dans son propre fil"
    );
}

#[tokio::test]
async fn m2_apercu_d_un_eleve_majeur_s_adresse_a_l_eleve() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let yasmine = eleve_majeur(&ent, etab, "Yasmine").await;
    let mere = nouvel_utilisateur(&ent, etab, "parent", "Mere").await;
    lier(
        &ent,
        etab,
        yasmine,
        mere,
        Some(nouveau_foyer(&ent, etab).await),
    )
    .await;
    let apercu = ent
        .apercu_destinataires_for_actor(etab, dir, role(Role::Direction), yasmine)
        .await
        .unwrap();
    assert!(apercu.destinataire_est_l_eleve);
    assert!(
        apercu.conversations.is_empty(),
        "R-M12 : pas de parent dans l'aperçu d'un élève majeur"
    );
}

// ─── Séparation d'un fil commun (MSG-118, CM-03, CM-04) ────────────────────────────────────

#[tokio::test]
async fn m2_separation_d_un_fil_commun_garde_le_passe_et_separe_l_avenir() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir) = (cast.eleve_lea_id, cast.direction.user_id);
    let lily = cast.parent_lea.user_id;
    // Arthur : second responsable, **même foyer** que Lily (parents ensemble) : fil commun (CM-03).
    let arthur = nouvel_utilisateur(&ent, etab, "parent", "Arthur").await;
    lier(&ent, etab, harry, arthur, Some(cast.foyer_lea_id)).await;
    let fils = ouvrir(&ent, etab, Role::Direction, dir, harry, "Voyage scolaire").await;
    assert_eq!(fils.len(), 1, "un seul foyer : un seul fil commun");
    let commun = fils[0].fil.id;
    assert!(
        lit(&ent, etab, lily, Role::Parent, commun).await
            && lit(&ent, etab, arthur, Role::Parent, commun).await,
        "témoin"
    );
    assert!(
        ecrit(
            &ent,
            etab,
            lily,
            Role::Parent,
            commun,
            "Lily : nous sommes d'accord"
        )
        .await
    );
    assert!(
        ecrit(
            &ent,
            etab,
            arthur,
            Role::Parent,
            commun,
            "Arthur : moi aussi"
        )
        .await
    );

    // Un parent d'un autre foyer ne peut séparer le fil de personne.
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    lier(
        &ent,
        etab,
        harry,
        james,
        Some(nouveau_foyer(&ent, etab).await),
    )
    .await;
    assert!(matches!(
        ent.separer_fil_for_actor(etab, james, role(Role::Parent), commun, vec![])
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Arthur demande son propre fil (MSG-118).
    let jumeau = ent
        .separer_fil_for_actor(etab, arthur, role(Role::Parent), commun, vec![])
        .await
        .expect("séparation");
    // Il lit le jumeau, plus le fil commun ; Lily garde le commun et ne lit pas le jumeau.
    assert!(lit(&ent, etab, arthur, Role::Parent, jumeau).await);
    assert!(
        !lit(&ent, etab, arthur, Role::Parent, commun).await,
        "sorti du fil commun"
    );
    assert!(lit(&ent, etab, lily, Role::Parent, commun).await);
    assert!(
        !lit(&ent, etab, lily, Role::Parent, jumeau).await,
        "Lily ne lit pas le fil d'Arthur"
    );
    // Le passé reste lisible des deux (CM-04) : trois messages copiés dans le jumeau.
    assert_eq!(
        corps_du_fil(&ent, jumeau).await,
        corps_du_fil(&ent, commun).await,
        "le passé est copié"
    );
    assert_eq!(corps_du_fil(&ent, jumeau).await.len(), 3);
    // L'avenir est séparé : un message de la direction dans un fil n'est pas dans l'autre.
    assert!(
        ecrit(
            &ent,
            etab,
            dir,
            Role::Direction,
            jumeau,
            "Pour Arthur seulement"
        )
        .await
    );
    assert!(
        ecrit(
            &ent,
            etab,
            dir,
            Role::Direction,
            commun,
            "Pour Lily seulement"
        )
        .await
    );
    assert!(!corps_du_fil(&ent, commun)
        .await
        .contains(&"Pour Arthur seulement".to_string()));
    assert!(!corps_du_fil(&ent, jumeau)
        .await
        .contains(&"Pour Lily seulement".to_string()));
    // Le jumeau pointe l'original ; le rapport et les événements le tracent, sans contenu.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_fil WHERE id = $1 AND jumeau_de = $2",
            &[jumeau, commun]
        )
        .await,
        1
    );
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_msg_reprise_rapport WHERE fil_id = $1 AND regle = 'SEPARATION'", &[commun]).await,
        1
    );
    let p = participants(&ent, commun).await;
    assert_eq!(
        p.iter().filter(|(c, _)| c == "famille").count(),
        1,
        "il reste un seul responsable dans le fil commun : {p:?}"
    );
    let noms: Vec<(String,)> = sqlx::query_as(
        "SELECT type FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type LIKE 'messagerie.%'",
    )
    .bind(etab)
    .bind(commun)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(noms.iter().any(|n| n.0 == "messagerie.fil_dedouble"));
    assert!(noms.iter().any(|n| n.0 == "messagerie.participant_retire"));
    // Plus rien à séparer : Lily est seule dans le fil commun.
    assert!(matches!(
        ent.separer_fil_for_actor(etab, lily, role(Role::Parent), commun, vec![])
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ─── Majorité (R-M12) ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m2_majorite_les_responsables_sortent_des_fils_ouverts_a_propos_de_l_eleve() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let eleve = Uuid::new_v4();
    // Mineur au départ (17 ans), il aura 18 ans en changeant la date de naissance.
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, date_naissance, created_at, updated_at)
         VALUES ($1,$2,'Lucas','Test','inscrit',$3, now(), now())",
    )
    .bind(eleve)
    .bind(etab)
    .bind(domain_eleve::horloge::jour_paris() - Duration::days(365 * 17))
    .execute(ent.pool())
    .await
    .unwrap();
    let mere = nouvel_utilisateur(&ent, etab, "parent", "Mere").await;
    lier(
        &ent,
        etab,
        eleve,
        mere,
        Some(nouveau_foyer(&ent, etab).await),
    )
    .await;
    let a_propos = ouvrir_un(&ent, etab, Role::Direction, dir, eleve, "Vos absences")
        .await
        .fil
        .id;
    let sien = ouvrir_un(&ent, etab, Role::Parent, mere, eleve, "Ma question")
        .await
        .fil
        .id;
    assert!(
        lit(&ent, etab, mere, Role::Parent, a_propos).await
            && lit(&ent, etab, mere, Role::Parent, sien).await,
        "témoin"
    );
    // Mineur : rien n'est retiré.
    assert_eq!(ent.appliquer_majorite_eleve(etab, eleve).await.unwrap(), 0);
    assert!(lit(&ent, etab, mere, Role::Parent, a_propos).await);
    // Majorité : la mère sort du fil ouvert par l'établissement à propos de lui, garde le sien.
    // L'identité n'est modifiable que par les commandes du socle 2 : le test passe par la
    // projection d'identité (seule écriture directe admise), le temps d'une transaction.
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve SET date_naissance = $2 WHERE id = $1")
        .bind(eleve)
        .bind(domain_eleve::horloge::jour_paris() - Duration::days(365 * 19))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(ent.appliquer_majorite_eleve(etab, eleve).await.unwrap(), 1);
    assert!(
        !lit(&ent, etab, mere, Role::Parent, a_propos).await,
        "R-M12 : plus de lecture à la majorité"
    );
    assert!(
        lit(&ent, etab, mere, Role::Parent, sien).await,
        "son propre échange reste"
    );
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_msg_participant WHERE fil_id = $1 AND motif_sortie = 'majorite'", &[a_propos]).await,
        1,
        "motif codé, jamais un texte"
    );
    // Idempotent.
    assert_eq!(ent.appliquer_majorite_eleve(etab, eleve).await.unwrap(), 0);
}
