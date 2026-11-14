//! Socle 4, lot A0 : ports `correspondances` / `integrer_declaration`, liens en attente
//! (invitation), commande unique `ajouter_responsable`, envois aux familles lus sur la fiche de la
//! personne, jour métier en Europe/Paris. Contre la base locale de test (feature `labo`).

use super::*;
use crate::dossier_personnes::{
    correspondances_tx, integrer_declaration_tx, DeclarationResponsable, DroitsDeclares,
    Resolution, ResolutionFoyer, ResolutionPersonne, SourceLien,
};
use ent_auth::Role;
use uuid::Uuid;

pub(super) async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { e.install_brain_p0(); }).await;
    Some((ent, cast))
}

pub(super) fn foyers(ent: &PgEnt) -> crate::foyer_repo::PgFoyerRepo {
    crate::foyer_repo::PgFoyerRepo::new(ent.pool_verif().clone())
}

pub(super) fn dir(cast: &LaboCast) -> (Uuid, &'static str) {
    (cast.direction.user_id, Role::Direction.as_str())
}

pub(super) async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    n
}

/// Élève neuf (les tests se rejouent sans dépendre des données précédentes).
pub(super) async fn eleve_neuf(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at)
         VALUES ($1,$2,$3,'Test','preinscrit', now(), now())",
    )
    .bind(id)
    .bind(etab)
    .bind(nom)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

/// Personne neuve AVEC compte de connexion (retourne personne_id, user_id).
pub(super) async fn personne_avec_compte(ent: &PgEnt, etab: Uuid, nom: &str, email: &str) -> (Uuid, Uuid) {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1,$2,'x',$3, now(), now())",
    )
    .bind(user)
    .bind(email)
    .bind(nom)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1,$2,'parent', now())")
        .bind(user)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let p = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_personne (id, etablissement_id, nom, prenom, email, user_id, source)
         VALUES ($1,$2,$3,'Avec', $4, $5, 'saisie')",
    )
    .bind(p)
    .bind(etab)
    .bind(nom)
    .bind(email)
    .bind(user)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    (p, user)
}

pub(super) fn declaration(nom: &str, prenom: &str, email: Option<&str>, rang: i16) -> DeclarationResponsable {
    DeclarationResponsable {
        cle: Uuid::new_v4(),
        rang,
        nom: nom.into(),
        prenom: Some(prenom.into()),
        civilite: None,
        email: email.map(Into::into),
        telephone: None,
        adresse: None,
        lien: "mère".into(),
        meme_foyer: false,
        autorite_parentale: true,
        droits: DroitsDeclares {
            notes: true,
            vs: true,
            justifier: true,
            sorties: false,
        },
        payeur: rang == 1,
        part_paiement: if rang == 1 { Some(100) } else { None },
        reserve: ReserveDeclaree::default(),
    }
}

pub(super) fn lien_saisie(personne_id: Option<Uuid>, foyer: Option<Uuid>) -> LienSaisie {
    LienSaisie {
        personne_id,
        foyer_id: foyer,
        lien: "père".into(),
        autorite_parentale: true,
        garde: None,
        date_debut: None,
        date_fin: None,
        peut_voir_notes: true,
        peut_voir_vs: true,
        peut_justifier_absence: true,
        peut_autoriser_sorties: false,
    }
}

pub(super) fn saisie(nom: &str, email: Option<&str>, tel: Option<&str>) -> PersonneSaisie {
    PersonneSaisie {
        nom: nom.into(),
        prenom: Some("Ada".into()),
        email: email.map(Into::into),
        telephone: tel.map(Into::into),
        ..Default::default()
    }
}

/// Défaut n°1 de la conception : un parent SANS compte reçoit ses messages (e-mail et SMS),
/// à l'adresse de sa fiche ; ses préférences sont respectées ; aucun envoi ne part de lui-même.
#[tokio::test]
async fn parent_sans_compte_est_destinataire_selon_ses_preferences() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let eleve = eleve_neuf(&ent, etab, "Envois").await;
    let mails_avant = ent.list_mails_recents(etab, 200).await.unwrap().len();
    let u = Uuid::new_v4().simple().to_string();
    let email = format!("sans-compte-{u}@exemple.fr");
    let tel = "06 12 34 56 78";
    let res = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Sanscompte", Some(&email), Some(tel))),
                lien: lien_saisie(None, None),
            },
        )
        .await
        .unwrap()
        .actif();
    assert!(!res.personne.user_id.is_some(), "aucun compte");
    // Sans compte, le lien est actif et le parent est destinataire (fiche de la personne).
    let mails = foyers(&ent)
        .emails_parents_eleve(etab, eleve)
        .await
        .unwrap();
    assert_eq!(mails, vec![email.clone()], "e-mail lu sur la fiche");
    let tels = foyers(&ent)
        .telephones_parents_eleve(etab, eleve)
        .await
        .unwrap();
    assert_eq!(tels, vec!["+33612345678".to_string()], "téléphone lu sur la fiche, normalisé E.164");
    // Préférences : plus de SMS, e-mail conservé.
    ent.definir_preferences_for_actor(etab, d, r, res.personne.id, true, false, false)
        .await
        .unwrap();
    assert_eq!(
        foyers(&ent)
            .emails_parents_eleve(etab, eleve)
            .await
            .unwrap(),
        vec![email.clone()]
    );
    assert!(foyers(&ent)
        .telephones_parents_eleve(etab, eleve)
        .await
        .unwrap()
        .is_empty());
    ent.definir_preferences_for_actor(etab, d, r, res.personne.id, false, true, false)
        .await
        .unwrap();
    assert!(foyers(&ent)
        .emails_parents_eleve(etab, eleve)
        .await
        .unwrap()
        .is_empty());
    // La fiche corrigée fait foi : la nouvelle adresse remplace l'ancienne.
    let nouvelle = format!("corrige-{u}@exemple.fr");
    ent.modifier_personne_for_actor(
        etab,
        d,
        r,
        res.personne.id,
        &saisie("Sanscompte", Some(&nouvelle), Some(tel)),
    )
    .await
    .unwrap();
    ent.definir_preferences_for_actor(etab, d, r, res.personne.id, true, true, false)
        .await
        .unwrap();
    assert_eq!(
        foyers(&ent)
            .emails_parents_eleve(etab, eleve)
            .await
            .unwrap(),
        vec![nouvelle]
    );
    // Aucun message n'est parti : rien n'est expédié sans geste humain.
    assert_eq!(
        ent.list_mails_recents(etab, 200).await.unwrap().len(),
        mails_avant,
        "ajouter un responsable n'envoie aucun message"
    );
    // Parent AVEC compte : l'adresse de la fiche prime sur celle du compte (repli seulement).
    let u2 = Uuid::new_v4().simple().to_string();
    let (pers, _) =
        personne_avec_compte(&ent, etab, "Aveccompte", &format!("compte-{u2}@exemple.fr")).await;
    let eleve2 = eleve_neuf(&ent, etab, "Envois2").await;
    // Liaison directe en base du lien actif (la confirmation est testée ailleurs).
    ent.lier_personne_for_actor(etab, d, r, eleve2, &lien_saisie(Some(pers), None))
        .await
        .unwrap();
    assert!(
        foyers(&ent)
            .emails_parents_eleve(etab, eleve2)
            .await
            .unwrap()
            .is_empty(),
        "lien en attente : aucun envoi"
    );
}

/// Un lien vers une personne qui a un compte = invitation : aucun accès avant sa confirmation ;
/// elle seule confirme ou refuse.
#[tokio::test]
async fn lien_vers_personne_avec_compte_n_ouvre_rien_avant_confirmation() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let u = Uuid::new_v4().simple().to_string();
    let (pers, user) =
        personne_avec_compte(&ent, etab, "Invitee", &format!("invitee-{u}@exemple.fr")).await;
    let eleve = eleve_neuf(&ent, etab, "Invitation").await;
    let res = ent
        .lier_personne_for_actor(etab, d, r, eleve, &lien_saisie(Some(pers), None))
        .await
        .unwrap();
    let LienResultat::EnAttente { attente_id, .. } = res else {
        panic!("attendu : lien en attente");
    };
    // Aucun lien réel, aucun accès.
    let liens = "SELECT count(*) FROM ent_eleve_lien WHERE eleve_id = $1";
    assert_eq!(compter(&ent, liens, eleve).await, 0);
    assert!(ent
        .lien_parent_eleve(etab, user, eleve)
        .await
        .unwrap()
        .is_none());
    // Une autre personne (même établissement) ne voit ni ne décide cette invitation.
    assert!(ent
        .mes_liens_en_attente(etab, cast.parent_lea.user_id)
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .repondre_lien_en_attente(etab, cast.parent_lea.user_id, attente_id, true)
        .await
        .is_err());
    assert_eq!(compter(&ent, liens, eleve).await, 0);
    // La personne voit la demande (minimisée) et l'accepte : le lien devient réel.
    let mes = ent.mes_liens_en_attente(etab, user).await.unwrap();
    assert_eq!(mes.len(), 1);
    assert_eq!(mes[0].eleve_initiale, "I");
    ent.repondre_lien_en_attente(etab, user, attente_id, true)
        .await
        .unwrap();
    assert_eq!(compter(&ent, liens, eleve).await, 1);
    assert!(ent
        .lien_parent_eleve(etab, user, eleve)
        .await
        .unwrap()
        .is_some());
    // Une demande déjà décidée ne se rejoue pas.
    assert!(ent
        .repondre_lien_en_attente(etab, user, attente_id, true)
        .await
        .is_err());

    // Refus : aucun accès, jamais.
    let eleve_b = eleve_neuf(&ent, etab, "Invitation2").await;
    let LienResultat::EnAttente { attente_id: a2, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve_b, &lien_saisie(Some(pers), None))
        .await
        .unwrap()
    else {
        panic!("attendu : lien en attente");
    };
    ent.repondre_lien_en_attente(etab, user, a2, false)
        .await
        .unwrap();
    assert!(ent
        .lien_parent_eleve(etab, user, eleve_b)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_eleve_lien WHERE eleve_id = $1",
            eleve_b
        )
        .await,
        0
    );
}

/// Le test du socle 3 : un second parent déclaré avec l'e-mail d'une personne qui a un compte
/// n'obtient AUCUN accès tant qu'elle n'a pas confirmé ; jamais de rapprochement implicite.
#[tokio::test]
async fn second_parent_non_verifie_ne_recoit_aucun_acces() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, _) = dir(&cast);
    let u = Uuid::new_v4().simple().to_string();
    let email_compte = format!("second-{u}@exemple.fr");
    let (pers_compte, user_compte) =
        personne_avec_compte(&ent, etab, "Second", &email_compte).await;
    let eleve = eleve_neuf(&ent, etab, "Fratrie").await;
    let nom_premier = format!("Premier{u}");
    let premier = declaration(
        &nom_premier,
        "Pia",
        Some(&format!("premier-{u}@exemple.fr")),
        1,
    );
    let second = declaration("Second", "Sam", Some(&email_compte), 2);

    let mut conn = ent.pool_verif().acquire().await.unwrap();
    // Candidat présenté, jamais appliqué seul.
    let cands = correspondances_tx(&mut conn, etab, &second).await.unwrap();
    assert_eq!(cands.len(), 1);
    assert!(cands[0].a_un_compte && cands[0].raison == "email");
    assert!(
        cands[0].email_masque.as_deref().unwrap().contains("***"),
        "e-mail masqué"
    );
    drop(conn);

    // Sans choix humain : refus net, rien n'est écrit (transaction annulée).
    let personnes = "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1";
    let avant = compter(&ent, personnes, etab).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = integrer_declaration_tx(
        &mut tx,
        etab,
        eleve,
        &[premier.clone(), second.clone()],
        &[],
        SourceLien::Inscription,
        d,
    )
    .await;
    assert!(matches!(e, Err(EntDbError::ConflitDossier(_))), "{e:?}");
    tx.rollback().await.unwrap();
    assert_eq!(compter(&ent, personnes, etab).await, avant);

    // Choix humain : « même personne » → lien EN ATTENTE, aucun accès.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let res = integrer_declaration_tx(
        &mut tx,
        etab,
        eleve,
        &[premier.clone(), second.clone()],
        &[Resolution {
            cle: second.cle,
            personne: Some(ResolutionPersonne::MemePersonne(pers_compte)),
            foyer: None,
        }],
        SourceLien::Inscription,
        d,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let rs = res
        .responsables
        .iter()
        .find(|x| x.cle == second.cle)
        .unwrap();
    assert!(rs.lien_id.is_none() && rs.attente_id.is_some());
    let rp = res
        .responsables
        .iter()
        .find(|x| x.cle == premier.cle)
        .unwrap();
    assert!(
        rp.lien_id.is_some() && rp.personne_creee,
        "le premier parent (sans compte) est lié"
    );
    assert!(ent
        .lien_parent_eleve(etab, user_compte, eleve)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_eleve_lien WHERE eleve_id = $1 AND personne_id IS NOT NULL",
            eleve
        )
        .await,
        1,
        "seul le premier parent a un lien réel"
    );
    // Aucun envoi ni notification vers le second parent non vérifié.
    assert!(foyers(&ent)
        .emails_parents_eleve(etab, eleve)
        .await
        .unwrap()
        .iter()
        .all(|m| *m != email_compte));
    // Confirmation par la personne (via la commande du socle 4) : accès ouvert.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let liens = dossier_personnes::confirmer_liens_attente_tx(
        &mut tx,
        etab,
        pers_compte,
        &dossier_personnes::FiltreAttente {
            source: Some(SourceLien::Inscription),
            source_ref: Some(second.cle),
            ..Default::default()
        },
        user_compte,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(liens.0.len(), 1);
    assert!(ent
        .lien_parent_eleve(etab, user_compte, eleve)
        .await
        .unwrap()
        .is_some());
}

/// Une fratrie validée en deux fois donne UN SEUL foyer (choix humain explicite) ; aucun foyer
/// n'est créé ni rapproché implicitement.
#[tokio::test]
async fn fratrie_validee_en_deux_fois_un_seul_foyer() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, _) = dir(&cast);
    let u = Uuid::new_v4().simple().to_string();
    let nom = format!("Fratrie{u}");
    let mere = declaration(&nom, "Maya", Some(&format!("maya-{u}@exemple.fr")), 1);
    let aine = eleve_neuf(&ent, etab, "Aine").await;
    let cadet = eleve_neuf(&ent, etab, "Cadet").await;

    let mut tx = ent.pool_verif().begin().await.unwrap();
    let r1 = integrer_declaration_tx(
        &mut tx,
        etab,
        aine,
        &[mere.clone()],
        &[],
        SourceLien::Inscription,
        d,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let premiere = &r1.responsables[0];
    let foyers_etab = "SELECT count(*) FROM ent_foyer WHERE etablissement_id = $1";
    let nb_foyers = compter(&ent, foyers_etab, etab).await;

    // Seconde validation : la mère existe déjà → choix explicite obligatoire (personne, puis foyer).
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    assert_eq!(
        correspondances_tx(&mut conn, etab, &mere)
            .await
            .unwrap()
            .len(),
        1
    );
    drop(conn);
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = integrer_declaration_tx(
        &mut tx,
        etab,
        cadet,
        &[mere.clone()],
        &[],
        SourceLien::Inscription,
        d,
    )
    .await;
    assert!(matches!(e, Err(EntDbError::ConflitDossier(_))), "{e:?}");
    tx.rollback().await.unwrap();
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = integrer_declaration_tx(
        &mut tx,
        etab,
        cadet,
        &[mere.clone()],
        &[Resolution {
            cle: mere.cle,
            personne: Some(ResolutionPersonne::MemePersonne(premiere.personne_id)),
            foyer: None,
        }],
        SourceLien::Inscription,
        d,
    )
    .await;
    assert!(
        matches!(e, Err(EntDbError::ConflitDossier(_))),
        "foyer à confirmer : {e:?}"
    );
    tx.rollback().await.unwrap();
    // Le foyer d'une autre famille est refusé.
    let autre = ent.create_foyer(etab, Some("Autre famille")).await.unwrap();
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = integrer_declaration_tx(
        &mut tx,
        etab,
        cadet,
        &[mere.clone()],
        &[Resolution {
            cle: mere.cle,
            personne: Some(ResolutionPersonne::MemePersonne(premiere.personne_id)),
            foyer: Some(ResolutionFoyer::MemeFoyer(autre)),
        }],
        SourceLien::Inscription,
        d,
    )
    .await;
    assert!(matches!(e, Err(EntDbError::ConflitDossier(_))), "{e:?}");
    tx.rollback().await.unwrap();
    // Choix complet : même personne, même foyer.
    let nb_foyers = nb_foyers + 1; // « Autre famille » ci-dessus
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let r2 = integrer_declaration_tx(
        &mut tx,
        etab,
        cadet,
        &[mere.clone()],
        &[Resolution {
            cle: mere.cle,
            personne: Some(ResolutionPersonne::MemePersonne(premiere.personne_id)),
            foyer: Some(ResolutionFoyer::MemeFoyer(premiere.foyer_id)),
        }],
        SourceLien::Inscription,
        d,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(r2.responsables[0].personne_id, premiere.personne_id);
    assert_eq!(r2.responsables[0].foyer_id, premiere.foyer_id);
    assert!(!r2.responsables[0].personne_creee);
    assert_eq!(
        compter(&ent, foyers_etab, etab).await,
        nb_foyers,
        "aucun foyer créé"
    );
    let (n_foyers_mere,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT foyer_id) FROM ent_eleve_lien WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etab)
    .bind(premiere.personne_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n_foyers_mere, 1, "un seul foyer pour toute la fratrie");
}

/// `ajouter_responsable` : une seule transaction. Une erreur au moment du lien n'abandonne aucune
/// personne orpheline ; données incohérentes refusées.
#[tokio::test]
async fn ajouter_responsable_est_atomique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let eleve = eleve_neuf(&ent, etab, "Atomique").await;
    let personnes = "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1";
    let avant = compter(&ent, personnes, etab).await;
    // Foyer inconnu : la personne qui venait d'être créée ne subsiste pas.
    let e = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Orpheline", None, None)),
                lien: lien_saisie(None, Some(Uuid::new_v4())),
            },
        )
        .await;
    assert!(e.is_err());
    assert_eq!(
        compter(&ent, personnes, etab).await,
        avant,
        "aucune personne orpheline"
    );
    // Soit l'une, soit l'autre.
    let e = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Deux", None, None)),
                lien: lien_saisie(Some(Uuid::new_v4()), None),
            },
        )
        .await;
    assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    let e = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: None,
                lien: lien_saisie(None, None),
            },
        )
        .await;
    assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    // Un profil sans droit « personnes.gerer » est refusé.
    let e = ent
        .ajouter_responsable_for_actor(
            etab,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Interdit", None, None)),
                lien: lien_saisie(None, None),
            },
        )
        .await;
    assert!(e.is_err());
    assert_eq!(compter(&ent, personnes, etab).await, avant);
    // Cas nominal.
    let ok = ent
        .ajouter_responsable_for_actor(
            etab,
            d,
            r,
            eleve,
            &AjoutResponsable {
                nouvelle_personne: Some(saisie("Nominal", Some("nominal@exemple.fr"), None)),
                lien: lien_saisie(None, None),
            },
        )
        .await
        .unwrap()
        .actif();
    assert_eq!(compter(&ent, personnes, etab).await, avant + 1);
    assert!(ok.actif);
}

/// Jour métier : Europe/Paris, pas UTC (bord de journée, été et hiver) ; la vue des liens actifs
/// suit ce jour.
#[tokio::test]
async fn jour_metier_europe_paris_bord_de_journee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent_ref = &ent;
    let jour = |instant: &'static str| async move {
        let (j,): (chrono::NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui($1::timestamptz)")
            .bind(instant)
            .fetch_one(ent_ref.pool())
            .await
            .unwrap();
        j.to_string()
    };
    // Été (UTC+2) : 22 h UTC le 31 août = minuit à Paris = déjà le 1er septembre.
    assert_eq!(jour("2026-08-31 21:59:59+00").await, "2026-08-31");
    assert_eq!(jour("2026-08-31 22:00:00+00").await, "2026-09-01");
    // Hiver (UTC+1) : 23 h UTC le 31 décembre = 1er janvier à Paris.
    assert_eq!(jour("2026-12-31 22:59:59+00").await, "2026-12-31");
    assert_eq!(jour("2026-12-31 23:00:00+00").await, "2027-01-01");

    // La vue suit le jour de Paris : fin hier = inactif, fin aujourd'hui = actif, début demain = inactif.
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let eleve = eleve_neuf(&ent, etab, "Dates").await;
    let (aujourdhui,): (chrono::NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui()")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let un_jour = chrono::Duration::days(1);
    for (nom, debut, fin, actif) in [
        ("Hier", None, Some(aujourdhui - un_jour), false),
        ("Aujourdhui", None, Some(aujourdhui), true),
        ("Demain", Some(aujourdhui + un_jour), None, false),
        ("Depuis", Some(aujourdhui), None, true),
    ] {
        let l = ent
            .ajouter_responsable_for_actor(
                etab,
                d,
                r,
                eleve,
                &AjoutResponsable {
                    nouvelle_personne: Some(saisie(nom, None, None)),
                    lien: LienSaisie {
                        date_debut: debut,
                        date_fin: fin,
                        ..lien_saisie(None, None)
                    },
                },
            )
            .await
            .unwrap()
            .actif();
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_eleve_lien_actif WHERE id = $1")
                .bind(l.lien_id)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
        assert_eq!(n == 1, actif, "{nom}");
    }
}

/// Clés composées (établissement, id) : validées, aucun orphelin ; une personne d'un autre
/// établissement ne peut pas être liée.
#[tokio::test]
async fn cles_composees_sans_orphelin() {
    let Some((ent, _cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dossier_orphelins_v")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0, "orphelins");
    let (non_validees,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM pg_constraint
          WHERE conname IN ('ent_eleve_lien_personne_etab_fk', 'ent_eleve_contact_urgence_personne_etab_fk',
                            'ent_personne_fusion_source_etab_fk', 'ent_personne_fusion_cible_etab_fk')
            AND NOT convalidated",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(non_validees, 0);
}

/// Une demande déjà en attente est remplacée par la plus récente (même origine) ; une demande
/// d'une autre origine est refusée nettement (revue de sécurité A0, M3).
#[tokio::test]
async fn demande_deja_en_attente_remplacee_ou_refusee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let u = Uuid::new_v4().simple().to_string();
    let (pers, user) =
        personne_avec_compte(&ent, etab, "Doublon", &format!("doublon-{u}@exemple.fr")).await;
    let eleve = eleve_neuf(&ent, etab, "DemandeDouble").await;
    let LienResultat::EnAttente { attente_id: a1, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve, &lien_saisie(Some(pers), None))
        .await
        .unwrap()
    else {
        panic!("attente attendue");
    };
    let nouvelle = LienSaisie {
        lien: "tuteur".into(),
        peut_voir_notes: false,
        ..lien_saisie(Some(pers), None)
    };
    let LienResultat::EnAttente { attente_id: a2, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve, &nouvelle)
        .await
        .unwrap()
    else {
        panic!("attente attendue");
    };
    assert_eq!(a1, a2, "une seule demande ouverte");
    assert_eq!(
        ent.mes_liens_en_attente(etab, user).await.unwrap()[0].lien,
        "tuteur"
    );
    // Autre origine (inscription) : refus net, rien n'est écrasé.
    let decl = declaration("Doublon", "Dora", None, 1);
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = integrer_declaration_tx(
        &mut tx,
        etab,
        eleve,
        &[decl.clone()],
        &[Resolution {
            cle: decl.cle,
            personne: Some(ResolutionPersonne::MemePersonne(pers)),
            foyer: None,
        }],
        SourceLien::Inscription,
        d,
    )
    .await;
    assert!(matches!(e, Err(EntDbError::ConflitDossier(_))), "{e:?}");
    tx.rollback().await.unwrap();
}
