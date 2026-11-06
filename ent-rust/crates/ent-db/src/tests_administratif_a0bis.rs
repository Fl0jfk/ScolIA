//! Socle 4, lot A0 bis : fuseau de l'établissement (A0-a), contrat `DeclarationResponsables` v1
//! et adresse à protéger (A0-b), « ne pas solliciter » (A0-c), confirmation du lien au guichet ou
//! en un geste (A0-d). Contre la base locale de test (feature `labo`).

use super::tests_administratif_a0::{
    compter, declaration, dir, eleve_neuf, labo, lien_saisie, personne_avec_compte, saisie,
};
use super::*;
use crate::dossier_personnes::{
    integrer_declaration_tx, PieceIdentite, ReserveDeclaree, SourceLien,
};
use ent_auth::Role;
use uuid::Uuid;

const ADRESSE: &str = "12 rue Secrète du Refuge, 75011 Paris";

async fn jour(ent: &PgEnt, etab: Uuid, instant: &str) -> String {
    let (j,): (chrono::NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2::timestamptz)")
        .bind(etab)
        .bind(instant)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    j.to_string()
}

async fn reinitialiser_fuseau(ent: &PgEnt, etab: Uuid) {
    sqlx::query("DELETE FROM ent_etablissement_parametre WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
}

async fn lien_actif(ent: &PgEnt, lien: Uuid) -> bool {
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_lien_actif WHERE id = $1")
        .bind(lien)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    n == 1
}

/// A0-a — bord de journée avec deux fuseaux : le même instant tombe sur deux jours différents
/// selon l'établissement ; la vue des liens actifs suit le fuseau de l'établissement.
#[tokio::test]
async fn fuseau_etablissement_bord_de_journee_deux_fuseaux() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    reinitialiser_fuseau(&ent, etab).await;
    // Établissement sans paramètre = Europe/Paris (comportement du lot A0 inchangé).
    assert_eq!(ent.fuseau_etablissement(etab).await.unwrap(), "Europe/Paris");
    let inconnu = Uuid::new_v4();
    assert_eq!(jour(&ent, inconnu, "2026-08-31 21:59:59+00").await, "2026-08-31");
    assert_eq!(jour(&ent, inconnu, "2026-08-31 22:00:00+00").await, "2026-09-01");

    // Fuseau invalide ou droit insuffisant : refusé, rien n'est écrit.
    assert!(ent
        .definir_fuseau_etablissement_for_actor(etab, d, r, "Mars/Olympus")
        .await
        .is_err());
    assert!(matches!(
        ent.definir_fuseau_etablissement_for_actor(
            etab,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            "Indian/Reunion"
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert_eq!(ent.fuseau_etablissement(etab).await.unwrap(), "Europe/Paris");

    // La Réunion (UTC+4) : 31 août 20:00 UTC = 1er septembre 00:00 local.
    ent.definir_fuseau_etablissement_for_actor(etab, d, r, "Indian/Reunion")
        .await
        .unwrap();
    assert_eq!(ent.fuseau_etablissement(etab).await.unwrap(), "Indian/Reunion");
    assert_eq!(jour(&ent, etab, "2026-08-31 19:59:59+00").await, "2026-08-31");
    assert_eq!(jour(&ent, etab, "2026-08-31 20:00:00+00").await, "2026-09-01");
    // Le même instant à Paris reste le 31 août : deux fuseaux, deux jours.
    assert_eq!(jour(&ent, inconnu, "2026-08-31 20:00:00+00").await, "2026-08-31");

    // La vue suit le fuseau : Kiritimati (UTC+14) et Pago Pago (UTC-11) diffèrent d'au moins
    // un jour civil. Un lien qui finit « aujourd'hui à Pago Pago » est déjà fini à Kiritimati.
    let (pago,): (chrono::NaiveDate,) =
        sqlx::query_as("SELECT (now() AT TIME ZONE 'Pacific/Pago_Pago')::date")
            .fetch_one(ent.pool())
            .await
            .unwrap();
    let eleve = eleve_neuf(&ent, etab, "Fuseau").await;
    let l = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Fuseau", None, None)),
                lien: LienSaisie {
                    date_fin: Some(pago),
                    ..lien_saisie(None, None)
                },
            },
        )
        .await
        .unwrap()
        .actif();
    ent.definir_fuseau_etablissement_for_actor(etab, d, r, "Pacific/Kiritimati")
        .await
        .unwrap();
    assert!(!lien_actif(&ent, l.lien_id).await, "fini à Kiritimati (UTC+14)");
    ent.definir_fuseau_etablissement_for_actor(etab, d, r, "Pacific/Pago_Pago")
        .await
        .unwrap();
    assert!(lien_actif(&ent, l.lien_id).await, "encore actif à Pago Pago (UTC-11)");
    reinitialiser_fuseau(&ent, etab).await;
}

/// A0-b — une déclaration avec `adresse_a_proteger` n'écrit AUCUNE adresse en clair ; les
/// champs réservés sont conservés ; la direction est alertée et seule elle ouvre l'adresse.
#[tokio::test]
async fn declaration_adresse_a_proteger_jamais_en_clair() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let eleve = eleve_neuf(&ent, etab, "Protegee").await;
    let u = Uuid::new_v4().simple().to_string();
    let mut decl = declaration("Refuge", "Ines", Some(&format!("ines-{u}@exemple.fr")), 1);
    decl.adresse = Some(ADRESSE.into());
    decl.reserve = ReserveDeclaree {
        adresse_a_proteger: true,
        ne_pas_solliciter: false,
        ne_pas_solliciter_ref: None,
        langue: Some("PRS".into()),
        besoin_interprete: true,
        organisme: Some("Aide sociale à l'enfance du département".into()),
        nature_code: Some("service_gardien".into()),
    };
    let mut tx = ent.pool().begin().await.unwrap();
    let res = integrer_declaration_tx(
        &mut tx,
        etab,
        eleve,
        &[decl.clone()],
        &[],
        SourceLien::Inscription,
        d,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let integre = &res.responsables[0];
    assert!(integre.adresse_parquee.is_some());
    let pid = integre.personne_id;

    // 1. Jamais dans la fiche.
    let (adresse,): (Option<String>,) =
        sqlx::query_as("SELECT adresse FROM ent_personne WHERE id = $1")
            .bind(pid)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(adresse, None, "aucune adresse dans ent_personne");
    // 2. Nulle part en clair : ni fiche, ni parking, ni journal, ni événements.
    let motif = "%Secrète du Refuge%";
    for sql in [
        "SELECT count(*) FROM ent_personne WHERE adresse LIKE $1 OR nom LIKE $1",
        "SELECT count(*) FROM ent_adresse_protegee WHERE adresse LIKE $1",
        "SELECT count(*) FROM ent_journal_modif WHERE payload::text LIKE $1 OR libelle LIKE $1",
        "SELECT count(*) FROM ent_metier_event WHERE payload::text LIKE $1",
    ] {
        let (n,): (i64,) = sqlx::query_as(sql)
            .bind(motif)
            .fetch_one(ent.pool())
            .await
            .unwrap();
        assert_eq!(n, 0, "adresse en clair trouvée : {sql}");
    }
    let (scellee,): (String,) =
        sqlx::query_as("SELECT adresse FROM ent_adresse_protegee WHERE personne_id = $1")
            .bind(pid)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert!(scellee.starts_with("enc:v1:"), "adresse scellée");

    // 3. Champs réservés conservés (langue normalisée en minuscules).
    let (langue, interprete, organisme, nature, version): (
        Option<String>,
        bool,
        Option<String>,
        Option<String>,
        i16,
    ) = sqlx::query_as(
        "SELECT langue, besoin_interprete, organisme, nature_code, contrat_version
           FROM ent_personne_reserve WHERE personne_id = $1",
    )
    .bind(pid)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(langue.as_deref(), Some("prs"));
    assert!(interprete);
    assert!(organisme.unwrap().starts_with("Aide sociale"));
    assert_eq!(nature.as_deref(), Some("service_gardien"));
    assert_eq!(version, 1);

    // 4. Alerte pour la direction, sans adresse ; refusée aux autres rôles.
    let alertes = ent.alertes_protection_for_actor(etab, d, r).await.unwrap();
    let alerte = alertes
        .adresses_a_traiter
        .iter()
        .find(|a| a.personne_id == pid)
        .expect("alerte d'adresse parquée");
    assert!(!format!("{alertes:?}").contains("Secrète"));
    for (u, role) in [
        (cast.administratif.user_id, Role::Administratif.as_str()),
        (cast.prof.user_id, Role::Prof.as_str()),
        (cast.parent_lea.user_id, Role::Parent.as_str()),
    ] {
        assert!(matches!(
            ent.alertes_protection_for_actor(etab, u, role).await,
            Err(EntDbError::Forbidden)
        ));
        assert!(matches!(
            ent.ouvrir_adresse_protegee_for_actor(etab, u, role, alerte.id).await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Un autre établissement ne voit rien et n'ouvre rien.
    assert!(ent
        .ouvrir_adresse_protegee_for_actor(Uuid::new_v4(), d, r, alerte.id)
        .await
        .is_err());
    // La direction ouvre (journalisé) et l'alerte est traitée.
    let claire = ent
        .ouvrir_adresse_protegee_for_actor(etab, d, r, alerte.id)
        .await
        .unwrap();
    assert_eq!(claire, ADRESSE);
    assert!(
        ent.alertes_protection_for_actor(etab, d, r)
            .await
            .unwrap()
            .adresses_a_traiter
            .iter()
            .all(|a| a.id != alerte.id)
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_journal_modif WHERE action = 'adresse_protegee_consultee' AND payload->>'alerte_id' = $1::text",
            alerte.id
        )
        .await,
        1
    );

    // 5. Contrat validé AVANT toute écriture : une langue invalide ne crée rien.
    let avant = compter(
        &ent,
        "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1",
        etab,
    )
    .await;
    let mut mauvaise = declaration("Invalide", "Langue", None, 1);
    mauvaise.reserve.langue = Some("pas une langue !".into());
    let mut tx = ent.pool().begin().await.unwrap();
    let e = integrer_declaration_tx(
        &mut tx,
        etab,
        eleve,
        &[mauvaise],
        &[],
        SourceLien::Inscription,
        d,
    )
    .await;
    assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    tx.rollback().await.unwrap();
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1",
            etab
        )
        .await,
        avant
    );
}

/// A0-b — personne déjà connue (rapprochement choisi par un humain) : l'adresse à protéger est
/// parquée aussi et la fiche existante n'est pas modifiée ; une déclaration ne lève jamais un
/// « ne pas solliciter » déjà posé.
#[tokio::test]
async fn adresse_protegee_d_une_personne_existante_et_reserve_fusionnee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let u = Uuid::new_v4().simple().to_string();
    let existante = ent
        .creer_personne_for_actor(etab, d, r, &saisie("Existante", Some(&format!("ex-{u}@exemple.fr")), None))
        .await
        .unwrap()
        .id;
    ent.definir_ne_pas_solliciter_for_actor(etab, d, r, existante, true, Some("JAF-2026-0042".into()))
        .await
        .unwrap();
    let eleve = eleve_neuf(&ent, etab, "Existante").await;
    let mut decl = declaration("Existante", "Ada", Some(&format!("ex-{u}@exemple.fr")), 1);
    decl.adresse = Some(ADRESSE.into());
    decl.reserve.adresse_a_proteger = true;
    let mut tx = ent.pool().begin().await.unwrap();
    let res = integrer_declaration_tx(
        &mut tx,
        etab,
        eleve,
        &[decl.clone()],
        &[crate::dossier_personnes::Resolution {
            cle: decl.cle,
            personne: Some(crate::dossier_personnes::ResolutionPersonne::MemePersonne(existante)),
            foyer: None,
        }],
        SourceLien::Saisie,
        d,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(res.responsables[0].adresse_parquee.is_some());
    assert!(res.responsables[0].ne_pas_solliciter, "protection conservée");
    let (adresse,): (Option<String>,) =
        sqlx::query_as("SELECT adresse FROM ent_personne WHERE id = $1")
            .bind(existante)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(adresse, None);
    // La référence de la pièce est scellée et ne se lit jamais en clair en base.
    let (reference,): (String,) = sqlx::query_as(
        "SELECT ne_pas_solliciter_ref FROM ent_personne_reserve WHERE personne_id = $1",
    )
    .bind(existante)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(reference.starts_with("enc:v1:") && !reference.contains("JAF"));
    // Confirmée par la direction : n'est plus « à confirmer ».
    assert!(ent
        .alertes_protection_for_actor(etab, d, r)
        .await
        .unwrap()
        .ne_pas_solliciter_a_confirmer
        .iter()
        .all(|p| p.personne_id != existante));
}

/// A0-c — une personne « ne pas solliciter » : jamais de liste d'invitations à son nom (elle
/// révélerait l'élève), aucune réponse en ligne, aucun envoi automatique de l'établissement ;
/// seule la direction pose ou lève la protection.
#[tokio::test]
async fn ne_pas_solliciter_aucune_sollicitation_ni_envoi() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let foyers = crate::foyer_repo::PgFoyerRepo::new(ent.pool().clone());
    let u = Uuid::new_v4().simple().to_string();
    let email_c = format!("compte-{u}@exemple.fr");
    let email_s = format!("sans-{u}@exemple.fr");
    let (p_compte, user) = personne_avec_compte(&ent, etab, "Compte", &email_c).await;
    let eleve = eleve_neuf(&ent, etab, "Sollicitation").await;

    // Parent sans compte, lien réel : destinataire tant qu'il n'est pas protégé.
    let sans = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Sans", Some(&email_s), None)),
                lien: lien_saisie(None, None),
            },
        )
        .await
        .unwrap()
        .actif();
    assert_eq!(foyers.emails_parents_eleve(etab, eleve).await.unwrap(), vec![email_s.clone()]);

    // Parent avec compte : invitation (lien en attente), visible de lui.
    let LienResultat::EnAttente { attente_id, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve, &lien_saisie(Some(p_compte), None))
        .await
        .unwrap()
    else {
        panic!("attendu : lien en attente");
    };
    assert_eq!(ent.mes_liens_en_attente(etab, user).await.unwrap().len(), 1);

    // Seule la direction pose la protection.
    assert!(matches!(
        ent.definir_ne_pas_solliciter_for_actor(
            etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            sans.personne.id,
            true,
            None
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    for p in [sans.personne.id, p_compte] {
        ent.definir_ne_pas_solliciter_for_actor(etab, d, r, p, true, None)
            .await
            .unwrap();
    }
    // Plus aucun envoi automatique vers la personne protégée.
    assert!(foyers.emails_parents_eleve(etab, eleve).await.unwrap().is_empty());
    // La personne avec compte ne voit plus la demande et ne peut plus y répondre en ligne.
    assert!(ent.mes_liens_en_attente(etab, user).await.unwrap().is_empty());
    assert!(ent
        .repondre_lien_en_attente(etab, user, attente_id, true)
        .await
        .is_err());
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_eleve_lien WHERE eleve_id = $1 AND personne_id IS NOT NULL", eleve).await,
        1,
        "le lien du parent protégé n'a pas été ouvert en ligne"
    );

    // La direction lève la protection : tout redevient possible.
    for p in [sans.personne.id, p_compte] {
        ent.definir_ne_pas_solliciter_for_actor(etab, d, r, p, false, None)
            .await
            .unwrap();
    }
    assert_eq!(foyers.emails_parents_eleve(etab, eleve).await.unwrap(), vec![email_s]);
    assert_eq!(ent.mes_liens_en_attente(etab, user).await.unwrap().len(), 1);
    // Une référence sans protection est refusée.
    assert!(ent
        .definir_ne_pas_solliciter_for_actor(etab, d, r, p_compte, false, Some("x".into()))
        .await
        .is_ok());
}

/// A0-d — « Vous êtes déclaré responsable de X : confirmer » : le professeur-parent connecté
/// confirme en un geste, sans e-mail d'invitation ; la voie est tracée.
#[tokio::test]
async fn professeur_parent_confirme_son_lien_en_un_geste() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let u = Uuid::new_v4().simple().to_string();
    let (pers, user) = personne_avec_compte(&ent, etab, "ProfParent", &format!("pp-{u}@exemple.fr")).await;
    sqlx::query("UPDATE ent_membership SET role = 'prof' WHERE user_id = $1")
        .bind(user)
        .execute(ent.pool())
        .await
        .unwrap();
    let eleve = eleve_neuf(&ent, etab, "ProfParent").await;
    let envois_avant = compter(
        &ent,
        "SELECT count(*) FROM ent_inscription_envoi WHERE etablissement_id = $1",
        etab,
    )
    .await;
    let LienResultat::EnAttente { attente_id, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve, &lien_saisie(Some(pers), None))
        .await
        .unwrap()
    else {
        panic!("attendu : lien en attente");
    };
    let mes = ent.mes_liens_en_attente(etab, user).await.unwrap();
    assert_eq!(mes.len(), 1);
    assert_eq!(mes[0].eleve_initiale, "P");
    ent.repondre_lien_en_attente(etab, user, attente_id, true)
        .await
        .unwrap();
    assert!(ent.lien_parent_eleve(etab, user, eleve).await.unwrap().is_some());
    let (voie, piece): (String, Option<String>) = sqlx::query_as(
        "SELECT confirmation_voie, piece_identite_type FROM ent_lien_attente WHERE id = $1",
    )
    .bind(attente_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((voie.as_str(), piece), ("personne_connectee", None));
    // Aucune invitation par e-mail n'a été nécessaire ni émise.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_inscription_envoi WHERE etablissement_id = $1",
            etab
        )
        .await,
        envois_avant
    );
}

/// A0-d — confirmation au guichet : droit `personnes.gerer`, type de pièce et auteur tracés,
/// jamais de copie ni de numéro ; une demande ne se confirme qu'une fois.
#[tokio::test]
async fn confirmation_au_guichet_piece_et_auteur_traces() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let (adm, adm_role) = (cast.administratif.user_id, Role::Administratif.as_str());
    let u = Uuid::new_v4().simple().to_string();
    let (pers, user) = personne_avec_compte(&ent, etab, "Guichet", &format!("gu-{u}@exemple.fr")).await;
    let eleve = eleve_neuf(&ent, etab, "Guichet").await;
    let LienResultat::EnAttente { attente_id, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve, &lien_saisie(Some(pers), None))
        .await
        .unwrap()
    else {
        panic!("attendu : lien en attente");
    };
    // Droit insuffisant ou autre établissement : rien ne se passe.
    for (acteur, role) in [
        (cast.prof.user_id, Role::Prof.as_str()),
        (cast.parent_lea.user_id, Role::Parent.as_str()),
    ] {
        assert!(matches!(
            ent.confirmer_lien_guichet_for_actor(etab, acteur, role, attente_id, PieceIdentite::Passeport)
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    assert!(ent
        .confirmer_lien_guichet_for_actor(Uuid::new_v4(), adm, adm_role, attente_id, PieceIdentite::Passeport)
        .await
        .is_err());
    assert!(ent.lien_parent_eleve(etab, user, eleve).await.unwrap().is_none());

    // Le secrétariat confirme après avoir vu une carte d'identité.
    let lien = ent
        .confirmer_lien_guichet_for_actor(etab, adm, adm_role, attente_id, PieceIdentite::CarteIdentite)
        .await
        .unwrap();
    assert!(lien.is_some());
    assert!(ent.lien_parent_eleve(etab, user, eleve).await.unwrap().is_some());
    let (statut, voie, piece, par): (String, String, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT statut, confirmation_voie, piece_identite_type, decide_par
           FROM ent_lien_attente WHERE id = $1",
    )
    .bind(attente_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(statut, "confirme");
    assert_eq!(voie, "guichet");
    assert_eq!(piece.as_deref(), Some("carte_identite"));
    assert_eq!(par, Some(adm), "l'auteur est tracé");
    // Le journal porte le TYPE de pièce et l'auteur, rien d'autre (ni numéro, ni copie).
    let (payload, auteur): (serde_json::Value, Uuid) = sqlx::query_as(
        "SELECT payload, actor_user_id FROM ent_journal_modif
          WHERE action = 'lien_confirme_guichet' AND eleve_id = $1",
    )
    .bind(eleve)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(auteur, adm);
    assert_eq!(payload["piece_identite_type"], "carte_identite");
    let cles: Vec<&String> = payload.as_object().unwrap().keys().collect();
    assert_eq!(cles.len(), 3, "attente_id, personne_id, piece_identite_type : {cles:?}");
    // Une demande déjà décidée ne se rejoue pas.
    assert!(ent
        .confirmer_lien_guichet_for_actor(etab, adm, adm_role, attente_id, PieceIdentite::Passeport)
        .await
        .is_err());
    // La base refuse une confirmation « guichet » sans type de pièce, et inversement.
    let e = sqlx::query(
        "UPDATE ent_lien_attente SET confirmation_voie = 'guichet', piece_identite_type = NULL WHERE id = $1",
    )
    .bind(attente_id)
    .execute(ent.pool())
    .await;
    assert!(e.is_err());
    let e = sqlx::query(
        "UPDATE ent_lien_attente SET confirmation_voie = 'invitation', piece_identite_type = 'passeport' WHERE id = $1",
    )
    .bind(attente_id)
    .execute(ent.pool())
    .await;
    assert!(e.is_err());
}
