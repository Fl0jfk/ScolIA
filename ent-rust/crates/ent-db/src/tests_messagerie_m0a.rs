//! Messagerie, lot M0a (`docs/socles/16_MESSAGERIE.md` §2.2, défauts D1 à D10) : tests d'attaque
//! contre la base locale de test (feature `labo`). Chaque scénario sème son propre établissement
//! de labo (jamais de donnée réelle) ; sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Un test d'attaque commence toujours par un **témoin** : la même action avant la mesure de
//! protection (ou par un acteur légitime) réussit, sinon le test ne prouverait rien.

use super::*;
use crate::msg_repo::{PgMsgRepo, PorteeFil};
use chrono::Utc;
use domain_eleve::{
    CarnetCategorie, CreerAnnonce, CreerCarnetEntree, Message, MsgFil, OuvrirFil, RepondreFil,
};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    // Mail, SMS et push simulés : jamais de réseau.
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn repo(ent: &PgEnt) -> PgMsgRepo {
    PgMsgRepo::new(ent.pool().clone())
}

fn staff(ent_role: Role) -> &'static str {
    ent_role.as_str()
}

async fn ouvrir(
    ent: &PgEnt,
    etab: Uuid,
    role: Role,
    user: Uuid,
    eleve: Uuid,
    sujet: &str,
) -> MsgFil {
    ent.ouvrir_fil_for_actor(
        staff(role),
        OuvrirFil {
            etablissement_id: etab,
            eleve_id: eleve,
            sujet: sujet.into(),
            premier_message: format!("Contenu confidentiel — {sujet}"),
            actor_user_id: user,
        },
    )
    .await
    .expect("ouverture du fil")
    .0
}

async fn lit(ent: &PgEnt, etab: Uuid, user: Uuid, role: Role, fil: Uuid) -> bool {
    match ent
        .list_messages_for_actor(etab, user, staff(role), fil)
        .await
    {
        Ok(_) => true,
        Err(EntDbError::Forbidden) => false,
        Err(e) => panic!("lecture du fil {fil} : {e:?}"),
    }
}

async fn liste_contient(ent: &PgEnt, etab: Uuid, user: Uuid, role: Role, fil: Uuid) -> bool {
    match ent
        .list_fils_for_actor(etab, user, staff(role), None, true, None)
        .await
    {
        Ok(l) => l.iter().any(|f| f.id == fil),
        Err(EntDbError::Forbidden) => false,
        Err(e) => panic!("liste des fils : {e:?}"),
    }
}

async fn ecrit(ent: &PgEnt, etab: Uuid, user: Uuid, role: Role, fil: Uuid) -> bool {
    match ent
        .repondre_fil_for_actor(
            staff(role),
            RepondreFil {
                etablissement_id: etab,
                fil_id: fil,
                corps: "réponse".into(),
                actor_user_id: user,
            },
        )
        .await
    {
        Ok(_) => true,
        Err(EntDbError::Forbidden) => false,
        Err(e) => panic!("réponse au fil {fil} : {e:?}"),
    }
}

/// Interdiction de communication en cours visant le responsable `parent` pour `eleve`.
async fn poser_interdiction(ent: &PgEnt, etab: Uuid, eleve: Uuid, parent: Uuid, decide_par: Uuid) -> Uuid {
    let (personne,): (Uuid,) = sqlx::query_as(
        "SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(eleve)
    .bind(parent)
    .fetch_one(ent.pool())
    .await
    .expect("personne du lien");
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction
            (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4, ARRAY['communication'], 'decision_justice', 'ne pas communiquer',
                 ent_aujourdhui_etab($2) - 1, $5)",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .bind(personne)
    .bind(decide_par)
    .execute(ent.pool())
    .await
    .expect("interdiction");
    id
}

/// Rend le responsable joignable par SMS et par push (appareil simulé).
async fn rendre_joignable(ent: &PgEnt, etab: Uuid, user: Uuid) {
    sqlx::query("UPDATE ent_user SET telephone = '0612345678' WHERE id = $1")
        .bind(user)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_personne SET telephone = '0612345678'
          WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_push_device (id, etablissement_id, user_id, platform, token)
         VALUES ($1,$2,$3,'android',$4) ON CONFLICT (etablissement_id, token) DO UPDATE SET revoked_at = NULL",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user)
    .bind(format!("tok-m0a-{user}"))
    .execute(ent.pool())
    .await
    .unwrap();
}

/// (courriels, SMS, push) émis à propos de l'élève depuis `depuis`.
async fn envois(ent: &PgEnt, etab: Uuid, eleve: Uuid, depuis: chrono::DateTime<Utc>) -> (i64, i64, i64) {
    let mut n = [0i64; 3];
    for (i, t) in ["ent_mail_outbox", "ent_sms_outbox", "ent_push_outbox"].iter().enumerate() {
        let (c,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {t} WHERE etablissement_id = $1 AND eleve_id = $2 AND created_at >= $3"
        ))
        .bind(etab)
        .bind(eleve)
        .bind(depuis)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        n[i] = c;
    }
    (n[0], n[1], n[2])
}

fn carnet(etab: Uuid, eleve: Uuid, auteur: Uuid, titre: &str) -> CreerCarnetEntree {
    CreerCarnetEntree {
        etablissement_id: etab,
        eleve_id: eleve,
        date_entree: Utc::now().date_naive(),
        categorie: CarnetCategorie::Information,
        titre: titre.into(),
        corps: format!("Détail confidentiel — {titre}"),
        visible_famille: true,
        actor_user_id: auteur,
    }
}

// ─── D1 : l'interdiction de communication est relue partout ──────────────────────────────

#[tokio::test]
async fn m0a_d1_parent_interdit_ne_lit_ni_n_ecrit_ni_ne_signe_ni_ne_recoit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    rendre_joignable(&ent, etab, lily).await;
    let parent = Role::Parent;

    // Témoin : avant l'interdiction, Lily lit, écrit, signe et reçoit.
    let t0 = Utc::now();
    let avant = ouvrir(&ent, etab, Role::Direction, dir, harry, "Avant interdiction").await;
    assert!(liste_contient(&ent, etab, lily, parent, avant.id).await, "témoin : liste");
    assert!(lit(&ent, etab, lily, parent, avant.id).await, "témoin : lecture");
    assert!(ecrit(&ent, etab, lily, parent, avant.id).await, "témoin : écriture");
    let e1 = ent
        .creer_carnet_entree_for_actor(staff(Role::Direction), carnet(etab, harry, dir, "Entrée témoin"))
        .await
        .unwrap();
    assert!(!ent
        .list_carnet_for_actor(etab, lily, staff(parent), harry)
        .await
        .unwrap()
        .is_empty());
    let (mails, sms, push) = envois(&ent, etab, harry, t0).await;
    assert!(mails > 0 && sms > 0 && push > 0, "témoin : envois {mails}/{sms}/{push}");

    // Interdiction de communication posée sur Lily.
    let interdiction = poser_interdiction(&ent, etab, harry, lily, dir).await;
    let t1 = Utc::now();

    // Lecture : ni liste, ni fil existant, ni carnet.
    assert!(!liste_contient(&ent, etab, lily, parent, avant.id).await, "fil existant listé");
    assert!(!lit(&ent, etab, lily, parent, avant.id).await, "fil existant lu");
    assert!(matches!(
        ent.list_carnet_for_actor(etab, lily, staff(parent), harry).await,
        Err(EntDbError::Forbidden)
    ));
    // Écriture : ni réponse, ni ouverture de fil, ni signature.
    assert!(!ecrit(&ent, etab, lily, parent, avant.id).await, "réponse permise");
    assert!(matches!(
        ent.ouvrir_fil_for_actor(
            staff(parent),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "x".into(),
                premier_message: "x".into(),
                actor_user_id: lily,
            },
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.signer_carnet_for_actor(etab, lily, staff(parent), e1.id).await,
        Err(EntDbError::Forbidden)
    ));
    // Envois : nouveau fil, nouvelle entrée de carnet, annonce — rien pour la famille interdite.
    let apres = ouvrir(&ent, etab, Role::Direction, dir, harry, "Après interdiction").await;
    ent.creer_carnet_entree_for_actor(staff(Role::Direction), carnet(etab, harry, dir, "Entrée interdite"))
        .await
        .unwrap();
    ent.publier_annonce_for_actor(
        staff(Role::Direction),
        CreerAnnonce {
            etablissement_id: etab,
            titre: "Annonce générale".into(),
            corps: "Réunion".into(),
            audience: "tous".into(),
            cible_classe: None,
            actor_user_id: dir,
        },
    )
    .await
    .unwrap();
    assert!(!lit(&ent, etab, lily, parent, apres.id).await);
    let (mails, sms, push) = envois(&ent, etab, harry, t1).await;
    assert_eq!((mails, sms, push), (0, 0, 0), "envois vers un parent interdit");
    // La personne n'apparaît plus dans les vues d'envoi.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve_lien_communicant WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(lily)
    .bind(harry)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 0);

    // Le personnel, lui, continue de lire (l'interdiction vise la personne, pas le dossier).
    assert!(lit(&ent, etab, dir, Role::Direction, avant.id).await);

    // Levée : l'accès revient (non-régression).
    sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $2 WHERE id = $1")
        .bind(interdiction)
        .bind(dir)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(lit(&ent, etab, lily, parent, avant.id).await, "accès non rétabli après la levée");
}

#[tokio::test]
async fn m0a_d1_interdiction_echue_ou_autre_objet_sans_effet() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    let fil = ouvrir(&ent, etab, Role::Direction, dir, harry, "Fil ordinaire").await;
    let id = poser_interdiction(&ent, etab, harry, lily, dir).await;
    assert!(!lit(&ent, etab, lily, Role::Parent, fil.id).await);
    // Échue hier : sans effet.
    sqlx::query("UPDATE ent_eleve_interdiction SET au = ent_aujourdhui_etab(etablissement_id) - 1 WHERE id = $1")
        .bind(id)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(lit(&ent, etab, lily, Role::Parent, fil.id).await, "interdiction échue encore appliquée");
    // Un autre objet (remise) ne ferme pas la messagerie.
    sqlx::query("UPDATE ent_eleve_interdiction SET au = NULL, objet = ARRAY['remise'] WHERE id = $1")
        .bind(id)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(lit(&ent, etab, lily, Role::Parent, fil.id).await, "interdiction de remise appliquée à la messagerie");
}

// ─── D2 : fils privés et relances ───────────────────────────────────────────────────────

#[tokio::test]
async fn m0a_d2_le_personnel_qui_suit_l_eleve_ne_lit_pas_les_fils_prives_ni_les_relances() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    let rogue = cast.profs[0].user_id; // enseigne en 4B : suit Harry
    let (cpe, adm) = (cast.cpe.user_id, cast.administratif.user_id);
    let prof = Role::Prof;
    assert!(ent.prof_suit_eleve(etab, rogue, harry).await.unwrap(), "prérequis : Rogue suit Harry");

    // Témoin : un fil ordinaire de l'établissement reste lisible par Rogue.
    let ordinaire = ouvrir(&ent, etab, Role::Direction, dir, harry, "Fil ordinaire").await;
    assert!(lit(&ent, etab, rogue, prof, ordinaire.id).await, "témoin : fil ordinaire");

    // Fil privé ouvert par Lily : direction, CPE et secrétariat le lisent, pas Rogue.
    let (prive, _) = ent
        .ouvrir_fil_for_actor(
            staff(Role::Parent),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "Situation familiale".into(),
                premier_message: "Nouvelle adresse : 4 rue des Lilas".into(),
                actor_user_id: lily,
            },
        )
        .await
        .unwrap();
    for (u, r) in [(dir, Role::Direction), (cpe, Role::Cpe), (adm, Role::Administratif)] {
        assert!(lit(&ent, etab, u, r, prive.id).await, "{r:?} doit lire le fil privé");
    }
    assert!(!liste_contient(&ent, etab, rogue, prof, prive.id).await, "fil privé listé pour Rogue");
    assert!(!lit(&ent, etab, rogue, prof, prive.id).await, "fil privé lu par Rogue");
    assert!(!ecrit(&ent, etab, rogue, prof, prive.id).await, "Rogue écrit dans le fil privé");

    // Relance de facture (fil de foyer) : lisible du secrétariat, jamais de Rogue.
    let relance_fil = {
        let (fil, msg) = domain_eleve::plan_ouvrir_fil(OuvrirFil {
            etablissement_id: etab,
            eleve_id: harry,
            sujet: "Relance de facture".into(),
            premier_message: "Montant dû : 1 240,00 €".into(),
            actor_user_id: adm,
        })
        .unwrap();
        repo(&ent)
            .insert_fil_portee(&fil, PorteeFil::foyer(cast.foyer_lea_id))
            .await
            .unwrap();
        repo(&ent).insert_message(&msg).await.unwrap();
        fil
    };
    assert!(lit(&ent, etab, adm, Role::Administratif, relance_fil.id).await, "témoin : secrétariat");
    assert!(!liste_contient(&ent, etab, rogue, prof, relance_fil.id).await, "relance listée pour Rogue");
    assert!(!lit(&ent, etab, rogue, prof, relance_fil.id).await, "relance lue par Rogue");
    // La CPE, non nommée pour les relances, ne les lit pas non plus.
    assert!(!lit(&ent, etab, cpe, Role::Cpe, relance_fil.id).await, "relance lue par la CPE");

    // Un personnel qui a écrit dans un fil privé continue de le lire.
    let (autre, _) = ent
        .ouvrir_fil_for_actor(
            staff(Role::Parent),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "Autre sujet".into(),
                premier_message: "Bonjour".into(),
                actor_user_id: lily,
            },
        )
        .await
        .unwrap();
    assert!(!lit(&ent, etab, rogue, prof, autre.id).await);
    repo(&ent)
        .insert_message(&Message {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            fil_id: autre.id,
            auteur_user_id: rogue,
            corps: "Réponse antérieure".into(),
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    assert!(lit(&ent, etab, rogue, prof, autre.id).await, "personnel ayant écrit");
}

// ─── D3 : l'élève ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m0a_d3_l_eleve_ne_lit_ni_ecrit_dans_les_fils_aux_responsables_et_ses_parents_ne_lisent_pas_les_siens() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    let compte = cast.eleves[0].compte.clone().expect("compte élève de Harry");
    let eleve = Role::Eleve;

    // Fil de l'établissement aux parents + alerte d'absences (vie scolaire) + carnet.
    let aux_parents = ouvrir(&ent, etab, Role::Direction, dir, harry, "Entretien avec la famille").await;
    let alerte = {
        let (fil, msg) = domain_eleve::plan_ouvrir_fil(OuvrirFil {
            etablissement_id: etab,
            eleve_id: harry,
            sujet: "Seuil d'absences atteint".into(),
            premier_message: "Harry a 12 demi-journées d'absence".into(),
            actor_user_id: cast.cpe.user_id,
        })
        .unwrap();
        repo(&ent).insert_fil_portee(&fil, PorteeFil::vie_scolaire()).await.unwrap();
        repo(&ent).insert_message(&msg).await.unwrap();
        fil
    };
    let entree = ent
        .creer_carnet_entree_for_actor(staff(Role::Direction), carnet(etab, harry, dir, "Observation"))
        .await
        .unwrap();
    let fil_carnet = entree.fil_notif_id.expect("fil du carnet");

    // Témoin : les parents les lisent.
    for f in [aux_parents.id, alerte.id, fil_carnet] {
        assert!(lit(&ent, etab, lily, Role::Parent, f).await, "témoin : Lily lit {f}");
    }
    // L'élève ne les liste pas, ne les lit pas, n'y écrit pas.
    for f in [aux_parents.id, alerte.id, fil_carnet] {
        assert!(!liste_contient(&ent, etab, compte.user_id, eleve, f).await, "{f} listé pour l'élève");
        assert!(!lit(&ent, etab, compte.user_id, eleve, f).await, "{f} lu par l'élève");
        assert!(!ecrit(&ent, etab, compte.user_id, eleve, f).await, "élève écrit dans {f}");
    }

    // Un fil ouvert par l'élève : lui le lit ; ses parents non ; la direction oui.
    let (fil_eleve, _) = ent
        .ouvrir_fil_for_actor(
            staff(eleve),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "J'ai un problème à la maison".into(),
                premier_message: "Je voudrais en parler à la CPE.".into(),
                actor_user_id: compte.user_id,
            },
        )
        .await
        .unwrap();
    assert!(lit(&ent, etab, compte.user_id, eleve, fil_eleve.id).await, "témoin : l'élève lit son fil");
    assert!(lit(&ent, etab, dir, Role::Direction, fil_eleve.id).await, "témoin : la direction lit");
    assert!(!liste_contient(&ent, etab, lily, Role::Parent, fil_eleve.id).await, "fil de l'élève listé pour son parent");
    assert!(!lit(&ent, etab, lily, Role::Parent, fil_eleve.id).await, "fil de l'élève lu par son parent");
    assert!(!ecrit(&ent, etab, lily, Role::Parent, fil_eleve.id).await, "parent écrit dans le fil de l'élève");

    // Aucun parent n'est notifié d'un fil d'élève (destinataires vides).
    assert_eq!(
        repo(&ent)
            .destinataires_famille_fil(etab, fil_eleve.id)
            .await
            .unwrap(),
        Some(Vec::new())
    );
}

// ─── D5 : droits du lien (vie scolaire, autorité parentale) ─────────────────────────────

#[tokio::test]
async fn m0a_d5_parent_sans_droit_vie_scolaire_ne_lit_pas_les_fils_de_vie_scolaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (draco, cpe) = (cast.eleve_tom_id, cast.cpe.user_id);
    let narcissa = cast.parent_tom.user_id; // peut_voir_vs = false sur Draco
    assert!(!cast.eleves[1].peut_voir_vs, "prérequis : Narcissa sans vie scolaire");

    let ordinaire = ouvrir(&ent, etab, Role::Cpe, cpe, draco, "Rendez-vous").await;
    let (vs, msg) = domain_eleve::plan_ouvrir_fil(OuvrirFil {
        etablissement_id: etab,
        eleve_id: draco,
        sujet: "Alerte absences".into(),
        premier_message: "Seuil atteint".into(),
        actor_user_id: cpe,
    })
    .unwrap();
    repo(&ent).insert_fil_portee(&vs, PorteeFil::vie_scolaire()).await.unwrap();
    repo(&ent).insert_message(&msg).await.unwrap();

    assert!(lit(&ent, etab, narcissa, Role::Parent, ordinaire.id).await, "témoin : fil ordinaire");
    assert!(!liste_contient(&ent, etab, narcissa, Role::Parent, vs.id).await, "fil de VS listé");
    assert!(!lit(&ent, etab, narcissa, Role::Parent, vs.id).await, "fil de VS lu");
    assert!(!ecrit(&ent, etab, narcissa, Role::Parent, vs.id).await, "écriture dans un fil de VS");
}

#[tokio::test]
async fn m0a_d5_d7_signature_du_carnet_reservee_au_titulaire_de_l_autorite() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    let e1 = ent
        .creer_carnet_entree_for_actor(staff(Role::Direction), carnet(etab, harry, dir, "À signer"))
        .await
        .unwrap();
    // Lien sans autorité parentale (tiers autorisé) : ne signe pas.
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(harry)
        .bind(lily)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(matches!(
        ent.signer_carnet_for_actor(etab, lily, staff(Role::Parent), e1.id).await,
        Err(EntDbError::Forbidden)
    ));
    // Témoin : avec l'autorité, la signature passe.
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(harry)
        .bind(lily)
        .execute(ent.pool())
        .await
        .unwrap();
    ent.signer_carnet_for_actor(etab, lily, staff(Role::Parent), e1.id)
        .await
        .expect("signature par le titulaire");
}

// ─── D6 : « ne pas solliciter » ─────────────────────────────────────────────────────────

#[tokio::test]
async fn m0a_d6_ne_pas_solliciter_exclu_des_envois_mais_pas_de_ses_propres_fils() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    rendre_joignable(&ent, etab, lily).await;

    // Témoin : avant la décision, le push d'un fil de l'établissement arrive.
    let t0 = Utc::now();
    ouvrir(&ent, etab, Role::Direction, dir, harry, "Avant la décision").await;
    let (_, _, push) = envois(&ent, etab, harry, t0).await;
    assert!(push > 0, "témoin : push");

    // Décision « ne pas solliciter » sur Lily.
    let (personne,): (Uuid,) = sqlx::query_as(
        "SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(harry)
    .bind(lily)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    ent.definir_ne_pas_solliciter_for_actor(etab, dir, Role::Direction.as_str(), personne, true, None)
        .await
        .expect("ne pas solliciter");
    let t1 = Utc::now();

    // Plus aucun envoi ouvert par l'établissement : push de fil, annonce, carnet.
    ouvrir(&ent, etab, Role::Direction, dir, harry, "Après la décision").await;
    ent.creer_carnet_entree_for_actor(staff(Role::Direction), carnet(etab, harry, dir, "Observation"))
        .await
        .unwrap();
    ent.publier_annonce_for_actor(
        staff(Role::Direction),
        CreerAnnonce {
            etablissement_id: etab,
            titre: "Annonce".into(),
            corps: "Texte".into(),
            audience: "tous".into(),
            cible_classe: None,
            actor_user_id: dir,
        },
    )
    .await
    .unwrap();
    assert_eq!(envois(&ent, etab, harry, t1).await, (0, 0, 0), "envois vers « ne pas solliciter »");
    // Les vues : communicante (lecture, réponse) oui ; sollicitable (envoi) non.
    for (vue, attendu) in [("ent_eleve_lien_communicant", 1i64), ("ent_eleve_lien_sollicitable", 0)] {
        let (n,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {vue} WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3"
        ))
        .bind(etab)
        .bind(lily)
        .bind(harry)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert_eq!(n, attendu, "{vue}");
    }

    // Elle répond encore dans un fil qu'elle a ouvert (réponse, pas sollicitation).
    let (ouvert_par_elle, _) = ent
        .ouvrir_fil_for_actor(
            staff(Role::Parent),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "Ma question".into(),
                premier_message: "Bonjour".into(),
                actor_user_id: lily,
            },
        )
        .await
        .expect("la personne peut écrire à l'établissement");
    let t2 = Utc::now();
    assert!(ecrit(&ent, etab, dir, Role::Direction, ouvert_par_elle.id).await);
    let (_, _, push) = envois(&ent, etab, harry, t2).await;
    assert!(push > 0, "la réponse dans son propre fil lui est notifiée");
}

// ─── D7 : le carnet ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m0a_d7_carnet_professeur_hors_perimetre_et_roles_sans_droit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let rogue = cast.profs[0].user_id; // 4B : Harry oui, Draco (5A) non
    let t0 = Utc::now();

    // Témoin : dans son périmètre, il écrit.
    ent.creer_carnet_entree_for_actor(staff(Role::Prof), carnet(etab, harry, rogue, "Retard répété"))
        .await
        .expect("professeur de la classe");
    // Hors périmètre : refusé, et aucun envoi à la famille de Draco.
    rendre_joignable(&ent, etab, cast.parent_tom.user_id).await;
    let r = ent
        .creer_carnet_entree_for_actor(staff(Role::Prof), carnet(etab, draco, rogue, "Observation"))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert_eq!(envois(&ent, etab, draco, t0).await, (0, 0, 0));
    // Sans droit « suivi.eleve » : la comptabilité n'écrit pas au carnet.
    let r = ent
        .creer_carnet_entree_for_actor(staff(Role::Compta), carnet(etab, harry, cast.compta.user_id, "x"))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Un parent n'écrit pas au carnet.
    let r = ent
        .creer_carnet_entree_for_actor(staff(Role::Parent), carnet(etab, harry, cast.parent_lea.user_id, "x"))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

// ─── D9 : droits effectifs, pas rôles codés en dur ──────────────────────────────────────

#[tokio::test]
async fn m0a_d9_messagerie_du_personnel_selon_les_droits_effectifs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    // Témoin : la direction a « messagerie.eleve ».
    assert!(ent
        .list_fils_for_actor(etab, cast.direction.user_id, staff(Role::Direction), None, true, None)
        .await
        .is_ok());
    // Sans le droit (comptabilité, infirmerie) : ni boîte, ni modèles, ni archivage.
    for (u, r) in [
        (cast.compta.user_id, Role::Compta),
        (cast.infirmieres[0].user_id, Role::Infirmerie),
    ] {
        assert!(matches!(
            ent.list_fils_for_actor(etab, u, staff(r), None, true, None).await,
            Err(EntDbError::Forbidden)
        ), "{r:?} liste la boîte");
        assert!(matches!(
            ent.list_msg_modeles_for_actor(etab, u, staff(r), false).await,
            Err(EntDbError::Forbidden)
        ), "{r:?} liste les modèles");
        let r2 = ent
            .ouvrir_fil_for_actor(
                staff(r),
                OuvrirFil {
                    etablissement_id: etab,
                    eleve_id: harry,
                    sujet: "x".into(),
                    premier_message: "x".into(),
                    actor_user_id: u,
                },
            )
            .await;
        assert!(matches!(r2, Err(EntDbError::Forbidden)), "{r:?} : {r2:?}");
    }
    // Retrait du droit : un rôle de base qui perd « messagerie.eleve » perd la boîte (le test
    // d'un rôle paramétré relève de la matrice D-017 ; ici le contrôle suit `droits_effectifs`).
    assert!(ent
        .a_action_quelconque(etab, cast.direction.user_id, staff(Role::Direction), ent_auth::Action::MessagerieEleve)
        .await
        .unwrap());
    assert!(!ent
        .a_action_quelconque(etab, cast.compta.user_id, staff(Role::Compta), ent_auth::Action::MessagerieEleve)
        .await
        .unwrap());
}

// ─── D4 : aucun contenu hors de l'ENT ───────────────────────────────────────────────────

#[tokio::test]
async fn m0a_d4_ni_sujet_ni_contenu_dans_les_push_sms_courriels_du_carnet_ni_les_evenements() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, dir, lily) = (cast.eleve_lea_id, cast.direction.user_id, cast.parent_lea.user_id);
    rendre_joignable(&ent, etab, lily).await;
    let t0 = Utc::now();
    let sujet = "Signalement de harcèlement concernant Léa";
    ouvrir(&ent, etab, Role::Direction, dir, harry, sujet).await;
    ent.creer_carnet_entree_for_actor(
        staff(Role::Direction),
        carnet(etab, harry, dir, "Comportement agressif envers Théo"),
    )
    .await
    .unwrap();

    // Témoin : des envois ont bien eu lieu.
    let (mails, sms, push) = envois(&ent, etab, harry, t0).await;
    assert!(mails > 0 && sms > 0 && push > 0, "témoin : {mails}/{sms}/{push}");
    // Aucun motif sensible dans les textes sortants.
    for (table, colonnes) in [
        ("ent_mail_outbox", "subject || ' ' || body_text"),
        ("ent_sms_outbox", "body"),
        ("ent_push_outbox", "title || ' ' || body"),
    ] {
        let (n,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table}
              WHERE etablissement_id = $1 AND eleve_id = $2 AND created_at >= $3
                AND ({colonnes} ILIKE '%harcèlement%' OR {colonnes} ILIKE '%agressif%'
                     OR {colonnes} ILIKE '%Théo%' OR {colonnes} ILIKE '%confidentiel%')"
        ))
        .bind(etab)
        .bind(harry)
        .bind(t0)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert_eq!(n, 0, "contenu sensible dans {table}");
    }
    // Aucun sujet ni titre dans les événements immuables du fil et du carnet.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event
          WHERE etablissement_id = $1 AND created_at >= $2
            AND (payload::text ILIKE '%harcèlement%' OR payload::text ILIKE '%agressif%')",
    )
    .bind(etab)
    .bind(t0)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 0, "sujet ou titre dans un événement");
    // Le contenu reste lisible dans l'ENT, après authentification.
    let fils = ent
        .list_fils_for_actor(etab, lily, staff(Role::Parent), None, true, None)
        .await
        .unwrap();
    assert!(fils.iter().any(|f| f.sujet == sujet), "témoin : le sujet reste dans l'ENT");
}

// ─── D10 : RLS permissive sur les tables de communication ───────────────────────────────

pub(crate) const TABLES_COMMUNICATION_RLS: &[&str] = &[
    "ent_msg_fil", "ent_msg", "ent_msg_accuse", "ent_msg_piece", "ent_msg_modele",
    "ent_carnet_entree", "ent_annonce",
    "ent_preference_notification", "ent_notification_plafond", "ent_notification_envoi",
    "ent_campagne", "ent_campagne_destinataire",
    "ent_rencontre", "ent_creneau_rdv",
    "ent_formulaire", "ent_formulaire_reponse",
    "ent_notification",
];

#[tokio::test]
async fn m0a_d10_rls_posee_sur_toutes_les_tables_de_communication() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    for t in TABLES_COMMUNICATION_RLS {
        let (actif, force, politiques): (bool, bool, i64) = sqlx::query_as(
            "SELECT c.relrowsecurity, c.relforcerowsecurity,
                    (SELECT count(*) FROM pg_policies p
                      WHERE p.tablename = c.relname AND p.policyname = 'ent_rls_etablissement')
               FROM pg_class c WHERE c.relname = $1 AND c.relkind = 'r'",
        )
        .bind(t)
        .fetch_one(ent.pool())
        .await
        .unwrap_or_else(|e| panic!("table {t} introuvable : {e}"));
        assert!(actif && force && politiques == 1, "RLS absente ou incomplète sur {t}");
    }
    // Isolation : un fil ne se lit pas avec l'identifiant d'un autre établissement.
    let etab = cast.etablissement_id;
    let fil = ouvrir(&ent, etab, Role::Direction, cast.direction.user_id, cast.eleve_lea_id, "Fil").await;
    let autre = Uuid::new_v4();
    assert!(ent
        .list_messages_for_actor(autre, cast.direction.user_id, "direction", fil.id)
        .await
        .is_err());
    assert!(repo(&ent)
        .get_fil(autre, fil.id)
        .await
        .unwrap()
        .is_none());
}
