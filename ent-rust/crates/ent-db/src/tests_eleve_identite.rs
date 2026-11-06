//! Socle 2 (l'élève), lot E2 : identité versionnée. Scénarios contre la base locale de
//! test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use crate::identite::{ChangementEtatCivil, CorrectionIdentite, UsageIdentite};
use chrono::{Duration, NaiveDate};
use domain_eleve::{CreateEleve, ModifIdentite, UpdateIdentite};
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

fn unique(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..8].replace(char::is_numeric, "x"))
}

fn aujourdhui() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

async fn nouvel_eleve(ent: &PgEnt, etab: Uuid, nom: &str, acteur: Uuid) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "Zélie".into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
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
    })
    .await
    .unwrap()
}

/// Dépose une « pièce » (ligne de document privé de l'élève ; le contenu et le lien
/// pré-signé relèvent du mécanisme de fichiers existant).
async fn piece(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, mime_type, source)
         VALUES ($1,$2,$3,'administratif','Acte de naissance','application/pdf','upload')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

fn champs_nom(n: &str) -> ModifIdentite {
    ModifIdentite {
        nom_naissance: Some(n.into()),
        ..Default::default()
    }
}

fn correction(etab: Uuid, eleve: Uuid, acteur: Uuid, modif: ModifIdentite) -> CorrectionIdentite {
    CorrectionIdentite {
        etablissement_id: etab,
        eleve_id: eleve,
        version_id: None,
        modif,
        motif: "faute de frappe à la saisie".into(),
        piece_document_id: None,
        source: "saisie".into(),
        acteur,
    }
}

#[derive(sqlx::FromRow, Debug)]
struct Projection {
    nom: String,
    prenom: String,
    nom_naissance: Option<String>,
    sexe: Option<String>,
    date_naissance: Option<NaiveDate>,
    identite_version_id: Option<Uuid>,
}

async fn projection(ent: &PgEnt, eleve: Uuid) -> Projection {
    sqlx::query_as(
        "SELECT nom, prenom, nom_naissance, sexe, date_naissance, identite_version_id
         FROM ent_eleve WHERE id = $1",
    )
    .bind(eleve)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

/// Création : une version `creation` (auteur tracé) et la projection égale à la version.
#[tokio::test]
async fn creation_ecrit_la_version_initiale() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let admin = cast.administratif.user_id;
    let e = nouvel_eleve(&ent, cast.etablissement_id, &unique("Init"), admin).await;
    let (nature, source, par, nom_n, prenoms): (String, String, Option<Uuid>, String, Vec<String>) =
        sqlx::query_as(
            "SELECT nature, source, enregistre_par, nom_naissance, prenoms
             FROM ent_eleve_identite WHERE eleve_id = $1",
        )
        .bind(e.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!((nature.as_str(), source.as_str()), ("creation", "saisie"));
    assert_eq!(par, Some(admin));
    assert_eq!(nom_n, e.nom);
    assert_eq!(prenoms, vec!["Zélie".to_string()]);
    let p = projection(&ent, e.id).await;
    assert_eq!((p.nom, p.prenom), (e.nom.clone(), "Zélie".to_string()));
    assert_eq!(p.nom_naissance.as_deref(), Some(e.nom.as_str()));
    assert_eq!(p.sexe.as_deref(), Some("F"));
    assert!(p.identite_version_id.is_some());
}

/// M2 : la reprise de l'existant est idempotente ; le nom repris est présumé nom de
/// naissance ; un sexe inconnu est listé sans être perdu.
#[tokio::test]
async fn reprise_de_l_existant_idempotente() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let id = Uuid::new_v4();
    let nom = unique("Ancien");
    // Élève « ancien » : inséré sans que le trigger de création écrive de version.
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_eleve DISABLE TRIGGER ent_eleve_identite_initiale")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, sexe, date_naissance)
         VALUES ($1,$2,$3,'Hugo','actif','X','2011-05-02')",
    )
    .bind(id)
    .bind(etab)
    .bind(&nom)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE ent_eleve ENABLE TRIGGER ent_eleve_identite_initiale")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let sans: (Option<Uuid>,) = sqlx::query_as("SELECT identite_version_id FROM ent_eleve WHERE id = $1")
        .bind(id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(sans.0.is_none());

    let (n1,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(n1 >= 1);
    let (n2,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n2, 0, "la reprise doit être idempotente");
    // Rejouer les migrations ne crée rien non plus.
    ent.migrate().await.unwrap();
    let (versions,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_identite WHERE eleve_id = $1")
        .bind(id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(versions, 1);

    let (nature, source, nom_n, sexe): (String, String, String, Option<String>) = sqlx::query_as(
        "SELECT nature, source, nom_naissance, sexe FROM ent_eleve_identite WHERE eleve_id = $1",
    )
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((nature.as_str(), source.as_str()), ("reprise", "reprise"));
    assert_eq!(nom_n, nom);
    assert_eq!(sexe, None, "sexe inconnu : NULL dans la version");
    let (a_traiter, origine): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*), max(valeur_origine) FROM ent_eleve_identite_a_traiter
         WHERE eleve_id = $1 AND champ = 'sexe'",
    )
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((a_traiter, origine.as_deref()), (1, Some("X")), "valeur d'origine conservée dans le rapport");
    // Égalité projection ↔ version pour tous les élèves.
    let (ecarts,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve e
         LEFT JOIN ent_eleve_identite v ON v.id = e.identite_version_id
         WHERE v.id IS NULL
            OR e.nom IS DISTINCT FROM coalesce(v.nom_usage, v.nom_naissance)
            OR e.nom_naissance IS DISTINCT FROM v.nom_naissance
            OR e.date_naissance IS DISTINCT FROM v.date_naissance",
    )
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(ecarts, 0, "projection ≠ version");
}

/// Correction ≠ changement : la correction REMPLACE (même date d'effet, l'ancienne reste
/// lisible) ; le changement AJOUTE une version datée (la fiche ne change qu'à la date).
#[tokio::test]
async fn correction_remplace_changement_ajoute() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = nouvel_eleve(&ent, etab, &unique("Dupond"), admin).await;
    let v0 = projection(&ent, e.id).await.identite_version_id.unwrap();

    // — Correction —
    let corrige = unique("Dupont");
    let r = ent
        .corriger_identite(role, correction(etab, e.id, admin, champs_nom(&corrige)))
        .await
        .unwrap();
    assert_eq!(r.champs, vec!["nom_naissance"]);
    assert_eq!(r.version.remplace_version_id, Some(v0));
    let (remplacee_par, effet0): (Option<Uuid>, NaiveDate) =
        sqlx::query_as("SELECT remplacee_par, effet_du FROM ent_eleve_identite WHERE id = $1")
            .bind(v0)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(remplacee_par, Some(r.version.id));
    assert_eq!(r.version.effet_du, effet0, "une correction garde la date d'effet de la version corrigée");
    let p = projection(&ent, e.id).await;
    assert_eq!(p.nom, corrige);
    assert_eq!(p.identite_version_id, Some(r.version.id));
    // Corriger une version déjà remplacée : refusé.
    let deja = ent
        .corriger_identite(
            role,
            CorrectionIdentite {
                version_id: Some(v0),
                ..correction(etab, e.id, admin, champs_nom(&unique("Autre")))
            },
        )
        .await;
    assert!(matches!(deja, Err(EntDbError::ConflitDossier(_))), "{deja:?}");

    // — Changement réel : pièce et motif obligatoires, effet daté —
    let futur = aujourdhui() + Duration::days(30);
    let nouveau_nom = unique("Martin");
    let changement = |piece_id: Uuid, nom: &str, jour: NaiveDate| ChangementEtatCivil {
        etablissement_id: etab,
        eleve_id: e.id,
        modif: champs_nom(nom),
        effet_du: jour,
        motif: "jugement de changement de nom".into(),
        piece_document_id: piece_id,
        source: "saisie".into(),
        acteur: admin,
    };
    // Pièce d'un AUTRE élève : refusée.
    let autre_piece = piece(&ent, etab, cast.eleve_lea_id).await;
    let refus = ent
        .enregistrer_changement_etat_civil(role, changement(autre_piece, &nouveau_nom, futur))
        .await;
    assert!(matches!(refus, Err(EntDbError::SaisieInvalide(_))), "{refus:?}");
    // Pièce inconnue : refusée.
    let inconnue = ent
        .enregistrer_changement_etat_civil(role, changement(Uuid::new_v4(), &nouveau_nom, futur))
        .await;
    assert!(inconnue.is_err());
    let ma_piece = piece(&ent, etab, e.id).await;
    let c = ent
        .enregistrer_changement_etat_civil(role, changement(ma_piece, &nouveau_nom, futur))
        .await
        .unwrap();
    assert_eq!(c.version.remplace_version_id, None, "un changement ne remplace rien");
    assert_eq!(c.version.piece_document_id, Some(ma_piece));
    assert_eq!(c.version.effet_du, futur);
    // La fiche garde la valeur actuelle jusqu'à la date d'effet…
    assert_eq!(projection(&ent, e.id).await.nom, corrige);
    // … puis la tâche quotidienne la projette (idempotente).
    sqlx::query("SELECT ent_eleve_identite_resynchroniser($1)")
        .bind(futur)
        .execute(ent.pool())
        .await
        .unwrap();
    assert_eq!(projection(&ent, e.id).await.nom, nouveau_nom);
    let (n,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_resynchroniser($1)")
        .bind(futur)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Deux versions sur la même date d'effet : conflit (c'est une correction).
    let doublon = ent
        .enregistrer_changement_etat_civil(role, changement(ma_piece, &unique("Autre"), futur))
        .await;
    assert!(matches!(doublon, Err(EntDbError::ConflitDossier(_))), "{doublon:?}");
    // Retour à aujourd'hui : on resynchronise (la base de test est partagée).
    sqlx::query("SELECT ent_eleve_identite_resynchroniser(current_date)")
        .execute(ent.pool())
        .await
        .unwrap();
    assert_eq!(projection(&ent, e.id).await.nom, nouveau_nom, "la projection ne recule pas sans nouvelle version");

    // Historique complet pour la direction : 3 versions (création remplacée, correction, changement).
    let h = ent
        .historique_identite_pour_acteur(etab, cast.direction.user_id, Role::Direction.as_str(), e.id)
        .await
        .unwrap();
    assert_eq!(h.len(), 3);
    assert!(h.iter().any(|v| v.nature.as_str() == "creation" && v.remplacee_par.is_some()));
    assert!(h.iter().any(|v| v.nature.as_str() == "correction" && v.motif.is_some()));
    assert!(h.iter().any(|v| v.nature.as_str() == "changement_etat_civil"));
}

/// Refus de toute écriture directe de l'identité et immutabilité des versions.
#[tokio::test]
async fn ecriture_directe_refusee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = nouvel_eleve(&ent, etab, &unique("Garde"), admin).await;
    for sql in [
        "UPDATE ent_eleve SET nom = 'Pirate' WHERE id = $1",
        "UPDATE ent_eleve SET prenom = 'Pirate' WHERE id = $1",
        "UPDATE ent_eleve SET sexe = 'M' WHERE id = $1",
        "UPDATE ent_eleve SET date_naissance = '2000-01-01' WHERE id = $1",
        "UPDATE ent_eleve SET nom_naissance = 'Pirate' WHERE id = $1",
        "UPDATE ent_eleve SET prenoms = ARRAY['Pirate'] WHERE id = $1",
        "UPDATE ent_eleve SET identite_version_id = gen_random_uuid() WHERE id = $1",
        "UPDATE ent_eleve_identite SET nom_naissance = 'Pirate' WHERE eleve_id = $1",
        "UPDATE ent_eleve_identite SET sensible = true WHERE eleve_id = $1",
        "DELETE FROM ent_eleve_identite WHERE eleve_id = $1",
    ] {
        let r = sqlx::query(sql).bind(e.id).execute(ent.pool()).await;
        let refus = r.expect_err(sql);
        let code = refus.as_database_error().and_then(|d| d.code().map(|c| c.to_string()));
        assert_eq!(code.as_deref(), Some("42501"), "{sql} → {refus}");
    }
    // Les autres colonnes restent modifiables, et l'identité n'a pas bougé.
    sqlx::query("UPDATE ent_eleve SET regime = 'DP' WHERE id = $1")
        .bind(e.id)
        .execute(ent.pool())
        .await
        .unwrap();
    let p = projection(&ent, e.id).await;
    assert_eq!(p.nom, e.nom);
    // Une écriture « sans effet » sur l'identité (mêmes valeurs) est tolérée : les chemins
    // d'upsert historiques réécrivent la ligne entière sans la changer.
    sqlx::query("UPDATE ent_eleve SET nom = nom WHERE id = $1")
        .bind(e.id)
        .execute(ent.pool())
        .await
        .unwrap();
}

/// Aucun état civil dans les événements ni dans le journal ; une version sensible n'est
/// pas montrée à la famille.
#[tokio::test]
async fn aucun_etat_civil_dans_les_evenements() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = nouvel_eleve(&ent, etab, &unique("Evt"), admin).await;
    let nom = unique("Valeursecrete");
    let prenom_usage = unique("Alias");
    ent.corriger_identite(
        role,
        correction(
            etab,
            e.id,
            admin,
            ModifIdentite {
                nom_naissance: Some(nom.clone()),
                date_naissance: Some(NaiveDate::from_ymd_opt(2008, 7, 9)),
                naissance_commune: Some(Some("Quimperlé".into())),
                ..Default::default()
            },
        ),
    )
    .await
    .unwrap();
    let u = ent
        .definir_usage(
            role,
            UsageIdentite {
                etablissement_id: etab,
                eleve_id: e.id,
                nom_usage: None,
                prenom_usage: Some(Some(prenom_usage.clone())),
                effet_du: None,
                motif: Some("demande de l'élève".into()),
                piece_document_id: None,
                source: "saisie".into(),
                acteur: admin,
            },
        )
        .await
        .unwrap();
    assert!(u.version.sensible);
    let evenements: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.identity_changed'",
    )
    .bind(e.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(evenements.len(), 2);
    let journal: Vec<(serde_json::Value, String, bool)> = sqlx::query_as(
        "SELECT payload, libelle, visible_famille FROM ent_journal_modif
         WHERE eleve_id = $1 AND action LIKE 'identite_%'",
    )
    .bind(e.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(journal.len(), 2);
    let tout = format!("{evenements:?}{journal:?}");
    for interdit in [nom.as_str(), prenom_usage.as_str(), "2008", "Quimperl"] {
        assert!(!tout.contains(interdit), "état civil en clair ({interdit}) : {tout}");
    }
    assert!(tout.contains("prenom_usage") && tout.contains("nom_naissance"), "noms de champs attendus");
    // Version sensible : jamais visible de la famille.
    let sensible_visible = journal.iter().filter(|(p, _, vis)| *vis && p["nature"] == "usage").count();
    assert_eq!(sensible_visible, 0);
    let correction_visible = journal.iter().filter(|(p, _, vis)| *vis && p["nature"] == "correction").count();
    assert_eq!(correction_visible, 1);
}

/// Droits : écriture réservée à `eleves.identite.modifier`, historique à
/// `eleves.historique.lire`, versions sensibles à `eleves.confidentiel.gerer`.
#[tokio::test]
async fn droits_ecriture_et_historique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = nouvel_eleve(&ent, etab, &unique("Droit"), admin).await;
    ent.definir_usage(
        role,
        UsageIdentite {
            etablissement_id: etab,
            eleve_id: e.id,
            nom_usage: None,
            prenom_usage: Some(Some("Zel".into())),
            effet_du: None,
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: admin,
        },
    )
    .await
    .unwrap();
    ent.corriger_identite(role, correction(etab, e.id, admin, champs_nom(&unique("Visible"))))
        .await
        .unwrap();

    // Écriture refusée au CPE, au professeur, à la famille.
    for (u, r) in [
        (cast.cpe.user_id, Role::Cpe.as_str()),
        (cast.prof.user_id, Role::Prof.as_str()),
        (cast.parent_lea.user_id, Role::Parent.as_str()),
    ] {
        let x = ent
            .corriger_identite(r, correction(etab, e.id, u, champs_nom(&unique("Refus"))))
            .await;
        assert!(matches!(x, Err(EntDbError::Forbidden)), "{r}: {x:?}");
        let h = ent.historique_identite_pour_acteur(etab, u, r, e.id).await;
        assert!(matches!(h, Err(EntDbError::Forbidden)), "{r}: {h:?}");
    }
    // Administratif : historique sans la version sensible ; direction : tout.
    let h = ent
        .historique_identite_pour_acteur(etab, admin, role, e.id)
        .await
        .unwrap();
    assert!(h.iter().all(|v| !v.sensible), "version sensible visible sans G6");
    assert!(h.iter().all(|v| v.nature.as_str() != "usage"));
    let h2 = ent
        .historique_identite_pour_acteur(etab, cast.direction.user_id, Role::Direction.as_str(), e.id)
        .await
        .unwrap();
    assert!(h2.iter().any(|v| v.sensible && v.nature.as_str() == "usage"));
    assert!(h2.len() > h.len());
    // La lecture est journalisée (acteur, objet), jamais le contenu.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif WHERE eleve_id = $1 AND domaine = 'identite'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(n >= 2);

    // Cloisonnement : établissement inconnu ou version d'un autre élève.
    let etranger = ent
        .corriger_identite(role, correction(Uuid::new_v4(), e.id, admin, champs_nom(&unique("X"))))
        .await;
    assert!(etranger.is_err());
    let autre = ent
        .corriger_identite(
            role,
            CorrectionIdentite {
                version_id: Some(h[0].id),
                ..correction(etab, cast.eleve_lea_id, admin, champs_nom(&unique("Y")))
            },
        )
        .await;
    assert!(autre.is_err(), "version d'un autre élève acceptée");
}

/// Compatibilité : l'ancienne saisie d'identité devient une correction versionnée et
/// refuse de masquer un nom d'usage.
#[tokio::test]
async fn ancienne_saisie_d_identite_passe_par_une_version() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = nouvel_eleve(&ent, etab, &unique("Legacy"), admin).await;
    let cmd = |nom: &str| UpdateIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        nom: Some(nom.into()),
        prenom: Some("Zélie-Rose".into()),
        ine: None,
        sexe: None,
        date_naissance: None,
        numero_badge: None,
        redoublant: None,
        photo_key: None,
        date_entree: None,
        actor_user_id: Some(admin.to_string()),
    };
    let nouveau = unique("Renomme");
    ent.update_identite(cmd(&nouveau)).await.unwrap();
    let p = projection(&ent, e.id).await;
    assert_eq!((p.nom.as_str(), p.prenom.as_str()), (nouveau.as_str(), "Zélie-Rose"));
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve_identite WHERE eleve_id = $1 AND nature = 'correction'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Avec un prénom d'usage défini, l'ancienne saisie ne peut plus changer le prénom affiché.
    ent.definir_usage(
        role,
        UsageIdentite {
            etablissement_id: etab,
            eleve_id: e.id,
            nom_usage: None,
            prenom_usage: Some(Some("Zel".into())),
            effet_du: None,
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: admin,
        },
    )
    .await
    .unwrap();
    let refus = ent
        .update_identite(UpdateIdentite {
            nom: None,
            prenom: Some("Autre".into()),
            ..cmd("")
        })
        .await;
    assert!(matches!(refus, Err(EntDbError::ConflitDossier(_))), "{refus:?}");
    assert_eq!(projection(&ent, e.id).await.prenom, "Zel");
}
