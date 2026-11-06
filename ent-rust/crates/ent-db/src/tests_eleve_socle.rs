//! Socle 2 (l'élève), lot E0 : correctifs. Scénarios contre la base locale de test
//! (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use domain_eleve::{CreateEleve, RechercherHomonymes, UpdateIdentite};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn nouvel_eleve(etab: Uuid, nom: &str, acteur: Uuid) -> CreateEleve {
    CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "Zélie".into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: chrono::NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: Some("6A".into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: Some(acteur.to_string()),
    }
}

/// Nom sans chiffre (l'identité versionnée refuse les chiffres dans un nom).
fn alpha(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..8].replace(char::is_numeric, "x"))
}

fn unique(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..8])
}

async fn un_i64(ent: &PgEnt, sql: &str, a: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).bind(a).fetch_one(ent.pool()).await.unwrap();
    n
}

/// E0 : la recherche d'homonymes ne donne la date de naissance qu'aux profils qui
/// y ont droit, et la date fournie par un profil réduit n'entre pas dans le score.
#[tokio::test]
async fn homonymes_minimises_selon_le_profil() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let nom = unique("Homon");
    let e = ent
        .create_eleve(nouvel_eleve(etab, &nom, cast.administratif.user_id))
        .await
        .unwrap();
    let cherche = |date| RechercherHomonymes {
        etablissement_id: etab,
        nom: nom.clone(),
        prenom: "Zélie".into(),
        date_naissance: date,
    };
    let date = e.date_naissance;
    // Gestionnaire : date visible.
    let c = ent
        .rechercher_homonymes_for_actor(etab, cast.administratif.user_id, Role::Administratif.as_str(), cherche(date))
        .await
        .unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].date_naissance, date);
    let score_complet = c[0].score;
    // CPE (niveau réduit) : aucune date en sortie, et la date fournie est ignorée
    // (même score avec ou sans date : pas d'oracle de date de naissance).
    for fournie in [date, chrono::NaiveDate::from_ymd_opt(1999, 1, 1), None] {
        let c = ent
            .rechercher_homonymes_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), cherche(fournie))
            .await
            .unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].date_naissance, None, "date de naissance exposée au CPE");
        assert!(c[0].score <= score_complet);
    }
    let sans = ent
        .rechercher_homonymes_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), cherche(None))
        .await
        .unwrap()[0]
        .score;
    let avec = ent
        .rechercher_homonymes_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), cherche(date))
        .await
        .unwrap()[0]
        .score;
    assert_eq!(sans, avec);
    // Infirmerie (niveau santé) : voit la date.
    let c = ent
        .rechercher_homonymes_for_actor(etab, cast.infirmieres[0].user_id, Role::Infirmerie.as_str(), cherche(date))
        .await
        .unwrap();
    assert_eq!(c[0].date_naissance, date);
}

/// E0 : création = élève + période + journal + événement, sans état civil dans
/// l'événement ni le journal.
#[tokio::test]
async fn creation_ecrit_journal_et_evenement_sans_etat_civil() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let nom = unique("Creat");
    let e = ent
        .create_eleve(nouvel_eleve(etab, &nom, cast.administratif.user_id))
        .await
        .unwrap();
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.created'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let texte = payload.to_string();
    assert!(!texte.contains(&nom) && !texte.contains("Zélie"), "état civil dans l'événement : {texte}");
    assert_eq!(
        un_i64(
            &ent,
            "SELECT count(*) FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'eleve_cree'",
            e.id
        )
        .await,
        1
    );
    assert_eq!(
        un_i64(&ent, "SELECT count(*) FROM ent_eleve_periode WHERE eleve_id = $1", e.id).await,
        1
    );
}

/// E0 : si le journal échoue, rien n'est créé (plus de `let _ =` sur le journal) —
/// ni élève, ni période, ni événement.
#[tokio::test]
async fn creation_defaite_si_le_journal_echoue() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let nom = unique("Rollb");
    sqlx::query(
        "CREATE OR REPLACE FUNCTION ent_test_refuse_journal() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.action IN ('eleve_cree','identite_modifiee') THEN
           RAISE EXCEPTION 'journal refusé (test)'; END IF; RETURN NEW; END $$",
    )
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_refuse_journal ON ent_journal_modif")
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER ent_test_refuse_journal BEFORE INSERT ON ent_journal_modif
         FOR EACH ROW EXECUTE FUNCTION ent_test_refuse_journal()",
    )
    .execute(ent.pool())
    .await
    .unwrap();
    let resultat = ent
        .create_eleve(nouvel_eleve(etab, &nom, cast.administratif.user_id))
        .await;
    let (restant,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve WHERE nom = $1")
        .bind(&nom)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    // La mise à jour d'identité doit, elle aussi, échouer sans effet.
    let existant = cast.eleve_lea_id;
    let nom_de = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            let (n,): (String,) = sqlx::query_as("SELECT nom FROM ent_eleve WHERE id = $1")
                .bind(existant)
                .fetch_one(&pool)
                .await
                .unwrap();
            n
        }
    };
    let avant = nom_de(&ent).await;
    let modif = ent
        .update_identite(UpdateIdentite {
            etablissement_id: etab,
            eleve_id: existant,
            nom: Some(alpha("Chg")),
            prenom: None,
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: None,
            photo_key: None,
            date_entree: None,
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await;
    let apres = nom_de(&ent).await;
    // Nettoyage avant toute assertion (le trigger est global).
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_refuse_journal ON ent_journal_modif")
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(resultat.is_err(), "la création doit échouer si le journal échoue");
    assert_eq!(restant, 0, "élève créé malgré l'échec du journal");
    assert!(modif.is_err(), "la modification doit échouer si le journal échoue");
    assert_eq!(avant, apres, "identité modifiée malgré l'échec du journal");
}

/// E0 : la modification d'identité ne publie et ne journalise que des NOMS de champs.
#[tokio::test]
async fn identite_evenement_et_journal_sans_valeurs() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let e = ent
        .create_eleve(nouvel_eleve(etab, &unique("Ident"), cast.administratif.user_id))
        .await
        .unwrap();
    let nouveau = alpha("Nouveau");
    ent.update_identite(UpdateIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        nom: Some(nouveau.clone()),
        prenom: None,
        ine: None,
        sexe: None,
        date_naissance: Some(chrono::NaiveDate::from_ymd_opt(2009, 12, 1)),
        numero_badge: None,
        redoublant: None,
        photo_key: None,
        date_entree: None,
        actor_user_id: Some(cast.administratif.user_id.to_string()),
    })
    .await
    .unwrap();
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.identity_changed'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    // E2 : nom et date de naissance passent par une version (correction) ; l'événement
    // porte la version, la nature et les NOMS de champs.
    assert_eq!(payload["nature"], "correction");
    assert_eq!(payload["champs"], serde_json::json!(["nom_naissance", "date_naissance"]));
    let (journal, libelle): (serde_json::Value, String) = sqlx::query_as(
        "SELECT payload, libelle FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'identite_correction'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    for t in [payload.to_string(), journal.to_string(), libelle] {
        assert!(!t.contains(&nouveau) && !t.contains("2009"), "valeur d'identité en clair : {t}");
    }
}

/// E0/M1 : les données ne peuvent être rattachées qu'à un élève du MÊME établissement.
#[tokio::test]
async fn cles_composees_refusent_l_autre_etablissement_et_l_inconnu() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let sql = "INSERT INTO ent_eleve_periode (id, etablissement_id, eleve_id, kind, valeur, date_debut)
               VALUES (gen_random_uuid(), $1, $2, 'classe', '6A', now()::date)";
    // Élève de l'établissement, mais avec l'identifiant d'un autre établissement.
    let croise = sqlx::query(sql)
        .bind(Uuid::new_v4())
        .bind(cast.eleve_lea_id)
        .execute(ent.pool())
        .await;
    assert!(croise.is_err(), "rattachement inter-établissements accepté");
    // Élève inexistant.
    let inconnu = sqlx::query(sql)
        .bind(cast.etablissement_id)
        .bind(Uuid::new_v4())
        .execute(ent.pool())
        .await;
    assert!(inconnu.is_err(), "rattachement à un élève inexistant accepté");
    // Aucun orphelin non traité n'est signalé sur la base de test.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_orphelin WHERE traite_le IS NULL")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

// ───────────────────────────── Lot E1 : droits et minimisation ─────────────────────────────

use crate::droits_repo::{Droit, RoleSaisie};
use domain_eleve::{EtatGroupe, GroupeChamps, NiveauIdentite, RaisonMasquage};
use ent_auth::{Action, Portee};

/// Ancienne règle figée par profil (conservée ici comme ORACLE de non-régression).
fn niveau_historique(role: Role) -> NiveauIdentite {
    match role {
        Role::Admin | Role::Direction | Role::Administratif | Role::Parent | Role::Eleve => {
            NiveauIdentite::Complet
        }
        Role::Infirmerie => NiveauIdentite::Sante,
        _ => NiveauIdentite::Reduit,
    }
}

fn droit(action: Action, portee: Portee, cibles: Vec<Uuid>) -> Droit {
    Droit { action, portee, cibles }
}

fn role_saisie(nom: &str, droits: Vec<Droit>) -> RoleSaisie {
    RoleSaisie {
        nom: format!("{nom} {}", &Uuid::new_v4().simple().to_string()[..6]),
        description: None,
        modele: None,
        droits,
    }
}

/// E1 : le niveau d'identité, désormais DÉRIVÉ des actions, est identique à l'ancienne
/// règle par profil pour chaque profil historique — par le modèle par défaut comme par
/// les droits effectifs lus en base.
#[tokio::test]
async fn e1_niveau_derive_identique_a_l_historique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let acteurs = [
        (Role::Admin, cast.admin.user_id),
        (Role::Direction, cast.direction.user_id),
        (Role::Administratif, cast.administratif.user_id),
        (Role::Cpe, cast.cpe.user_id),
        (Role::Compta, cast.compta.user_id),
        (Role::Prof, cast.prof.user_id),
        (Role::Infirmerie, cast.infirmieres[0].user_id),
        (Role::Psy, cast.psys[0].user_id),
        (Role::Parent, cast.parent_lea.user_id),
    ];
    for (role, user) in acteurs {
        assert_eq!(
            PgEnt::niveau_identite(role.as_str()),
            niveau_historique(role),
            "modèle par défaut : {role:?}"
        );
        let groupes = ent
            .profil_champs(etab, user, role.as_str())
            .await
            .unwrap()
            .pour(harry);
        assert_eq!(groupes.niveau(), niveau_historique(role), "droits effectifs : {role:?}");
        // La désignation est ouverte à tout le personnel, comme l'ancien annuaire.
        assert!(groupes.ouvert(GroupeChamps::Designation), "{role:?}");
        // Jamais d'historique ni de sensible sans le droit dédié (aucune fuite de G5/G6).
        let direction = matches!(role, Role::Admin | Role::Direction | Role::Administratif);
        assert_eq!(groupes.ouvert(GroupeChamps::Historique), direction, "{role:?} G5");
        assert_eq!(
            groupes.ouvert(GroupeChamps::Sensible),
            matches!(role, Role::Admin | Role::Direction),
            "{role:?} G6"
        );
    }
}

/// E1 : un rôle libre ouvre des groupes (Accueil), une portée « désignés » peut cibler un
/// site ou un niveau (Q-E9), et chaque masquage est expliqué (droit manquant / hors portée).
#[tokio::test]
async fn e1_roles_libres_groupes_portees_site_niveau() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let (user, base) = (cast.profs[0].user_id, Role::Prof.as_str());
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);

    // Un site et un niveau, rattachés à la classe de Harry seulement.
    let (classe_harry,): (Uuid,) = sqlx::query_as(
        "SELECT classe_id FROM ent_affectation WHERE eleve_id = $1 AND classe_id IS NOT NULL
         ORDER BY date_debut DESC LIMIT 1",
    )
    .bind(harry)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let code = unique("S");
    let (site,): (Uuid,) = sqlx::query_as(
        "INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES (gen_random_uuid(), $1, $2, 'Site test') RETURNING id",
    )
    .bind(etab)
    .bind(&code)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let (niveau,): (Uuid,) = sqlx::query_as(
        "INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES (gen_random_uuid(), $1, $2, 'Niveau test') RETURNING id",
    )
    .bind(etab)
    .bind(&code)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let ancien: (Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT site_id, niveau_id FROM ent_classe WHERE id = $1")
            .bind(classe_harry)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    sqlx::query("UPDATE ent_classe SET site_id = $2, niveau_id = $3 WHERE id = $1")
        .bind(classe_harry)
        .bind(site)
        .bind(niveau)
        .execute(ent.pool())
        .await
        .unwrap();

    // Cibles étrangères à l'établissement : refusées (isolation).
    let etranger = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            role_saisie(
                "Cible étrangère",
                vec![droit(Action::ElevesEtatCivilLire, Portee::Designes, vec![Uuid::new_v4()])],
            ),
        )
        .await;
    assert!(etranger.is_err(), "cible d'un autre établissement acceptée");

    // Avant : un professeur ne voit ni la scolarité ni l'état civil, et on lui dit pourquoi.
    let avant = ent.profil_champs(etab, user, base).await.unwrap().pour(harry);
    assert_eq!(
        avant.etat(GroupeChamps::EtatCivil),
        EtatGroupe::Masque(RaisonMasquage::DroitManquant)
    );
    assert!(avant.masques().iter().any(|m| m.groupe == "etat_civil"
        && m.raison == "droit_manquant"
        && m.action == Some("eleves.etat_civil.lire")));

    // Rôle « Accueil » : annuaire + scolarité sur tout l'établissement, rien d'autre.
    let accueil = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            role_saisie(
                "Accueil",
                vec![
                    droit(Action::ElevesAnnuaire, Portee::Etablissement, vec![]),
                    droit(Action::ElevesLire, Portee::Etablissement, vec![]),
                    droit(Action::AbsencesLire, Portee::Etablissement, vec![]),
                ],
            ),
        )
        .await
        .unwrap();
    // Secrétariat de site / référent de niveau : état civil sur ce site OU ce niveau.
    let site_role = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            role_saisie(
                "Secrétariat de site",
                vec![droit(Action::ElevesEtatCivilLire, Portee::Designes, vec![site])],
            ),
        )
        .await
        .unwrap();
    let niveau_role = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            role_saisie(
                "Référent de niveau",
                vec![droit(Action::ElevesIdentifiantsLire, Portee::Designes, vec![niveau])],
            ),
        )
        .await
        .unwrap();
    for r in [&accueil, &site_role, &niveau_role] {
        ent.attribuer_role_for_actor(etab, dir.0, dir.1, r.id, user, None)
            .await
            .unwrap();
    }

    let profil = ent.profil_champs(etab, user, base).await.unwrap();
    let (gh, gd) = (profil.pour(harry), profil.pour(draco));
    // Accueil : scolarité partout.
    assert!(gh.ouvert(GroupeChamps::Scolarite) && gd.ouvert(GroupeChamps::Scolarite));
    // Site désigné : état civil pour Harry, hors portée pour Draco (autre site).
    assert!(gh.ouvert(GroupeChamps::EtatCivil));
    assert_eq!(
        gd.etat(GroupeChamps::EtatCivil),
        EtatGroupe::Masque(RaisonMasquage::HorsPortee)
    );
    // Niveau désigné : identifiants pour Harry seulement.
    assert!(gh.ouvert(GroupeChamps::Identifiants));
    assert_eq!(
        gd.etat(GroupeChamps::Identifiants),
        EtatGroupe::Masque(RaisonMasquage::HorsPortee)
    );
    // Le badge n'est interrogeable que si les identifiants sont visibles partout.
    assert!(!profil.badge_interrogeable());
    // Minimisation effective par le point unique, et sans effet sur G5/G6.
    let vue = ent
        .get_eleve_vue_for_actor(etab, user, base, draco)
        .await
        .unwrap()
        .unwrap();
    assert!(vue.eleve.date_naissance.is_none() && vue.eleve.ine.is_none());
    assert!(!vue.groupes.ouvert(GroupeChamps::Historique));
    assert!(vue.groupes.masques().iter().any(|m| m.groupe == "etat_civil" && m.raison == "hors_portee"));

    // Retrait des rôles : retour immédiat au modèle du profil.
    for r in [&accueil, &site_role, &niveau_role] {
        ent.retirer_role_for_actor(etab, dir.0, dir.1, r.id, user).await.unwrap();
    }
    let apres = ent.profil_champs(etab, user, base).await.unwrap().pour(harry);
    assert!(!apres.ouvert(GroupeChamps::Scolarite) && !apres.ouvert(GroupeChamps::EtatCivil));

    sqlx::query("UPDATE ent_classe SET site_id = $2, niveau_id = $3 WHERE id = $1")
        .bind(classe_harry)
        .bind(ancien.0)
        .bind(ancien.1)
        .execute(ent.pool())
        .await
        .unwrap();
}

/// E1 : la famille voit G0 à G3 de son enfant, jamais G5 ni G6 ; l'élève idem de lui-même.
#[tokio::test]
async fn e1_famille_g0_a_g3_jamais_g5_g6() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let g = ent
        .profil_champs(cast.etablissement_id, cast.parent_lea.user_id, Role::Parent.as_str())
        .await
        .unwrap()
        .pour(cast.eleve_lea_id);
    for ouvert in [
        GroupeChamps::Designation,
        GroupeChamps::Scolarite,
        GroupeChamps::EtatCivil,
        GroupeChamps::Identifiants,
    ] {
        assert!(g.ouvert(ouvert), "{ouvert:?}");
    }
    for ferme in [GroupeChamps::Historique, GroupeChamps::Sensible] {
        assert_eq!(g.etat(ferme), EtatGroupe::Masque(RaisonMasquage::ReserveAuPersonnel));
    }
}
