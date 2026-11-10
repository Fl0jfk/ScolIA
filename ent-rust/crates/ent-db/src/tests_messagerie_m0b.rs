//! Messagerie, lot M0b (`docs/socles/16_MESSAGERIE.md` §27.1 et §33.2, `PERSONAS_16.md`) : tests
//! d'attaque contre la base locale de test (feature `labo`). Chaque scénario sème son propre
//! établissement de labo (jamais de donnée réelle) ; sans `ENT_DATABASE_URL`, les scénarios sont
//! ignorés (en CI, leur absence fait échouer le test).
//!
//! Comme en M0a, un test d'attaque commence par un **témoin** : la même action avant la mesure de
//! protection (ou par un acteur légitime) réussit, sinon le test ne prouverait rien. Chaque
//! scénario échouait avant le correctif correspondant.

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use crate::msg_repo::CurseurFils;
use chrono::{Duration, TimeZone, Utc};
use domain_eleve::{
    CreerAnnonce, DomainError, MessageLue, MsgFil, MsgPiece, OuvrirFil, RepondreFil,
};
use ent_auth::{Action, Portee, Role};
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

fn role(r: Role) -> &'static str {
    r.as_str()
}

// ─── Outils de mise en scène ────────────────────────────────────────────────────────────────

async fn nouvel_utilisateur(ent: &PgEnt, etab: Uuid, base: &str, nom: &str) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1,$2,'x',$3, now(), now())",
    )
    .bind(user)
    .bind(format!("{}-{}@ent.lab", nom.to_lowercase(), user.simple()))
    .bind(nom)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
         VALUES ($1,$2,$3, now())",
    )
    .bind(user)
    .bind(etab)
    .bind(base)
    .execute(ent.pool())
    .await
    .unwrap();
    user
}

async fn nouveau_foyer(ent: &PgEnt, etab: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(etab)
        .bind("Foyer de test")
        .execute(ent.pool())
        .await
        .unwrap();
    id
}

/// Lie un compte responsable à l'élève dans ce foyer (la personne est créée par le déclencheur).
async fn lier(ent: &PgEnt, etab: Uuid, eleve: Uuid, user: Uuid, foyer: Option<Uuid>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien)
         VALUES ($1,$2,$3,$4,$5,'pere')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

/// Appareil push dont le jeton est vide après nettoyage : le fournisseur simulé refuse l'envoi.
async fn appareil(ent: &PgEnt, etab: Uuid, user: Uuid, token: &str) {
    sqlx::query(
        "INSERT INTO ent_push_device (id, etablissement_id, user_id, platform, token)
         VALUES ($1,$2,$3,'android',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user)
    .bind(token)
    .execute(ent.pool())
    .await
    .unwrap();
}

/// Compte d'élève du labo (créé s'il n'existe pas).
async fn compte_eleve(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let existant: Option<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(ent.pool())
    .await
    .unwrap();
    if let Some((u,)) = existant {
        return u;
    }
    let user = nouvel_utilisateur(ent, etab, "eleve", "Eleve").await;
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3)")
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .execute(ent.pool())
        .await
        .unwrap();
    user
}

async fn ouvrir_fils(
    ent: &PgEnt,
    etab: Uuid,
    r: Role,
    user: Uuid,
    eleve: Uuid,
    sujet: &str,
) -> Vec<MsgFil> {
    ent.ouvrir_fils_for_actor(
        role(r),
        OuvrirFil {
            etablissement_id: etab,
            eleve_id: eleve,
            sujet: sujet.into(),
            premier_message: format!("Contenu confidentiel — {sujet}"),
            actor_user_id: user,
        },
        OptionsEnvoiMessage::default(),
    )
    .await
    .expect("ouverture")
    .into_iter()
    .map(|o| o.fil)
    .collect()
}

async fn ouvrir_un(
    ent: &PgEnt,
    etab: Uuid,
    r: Role,
    user: Uuid,
    eleve: Uuid,
    sujet: &str,
) -> MsgFil {
    let mut fils = ouvrir_fils(ent, etab, r, user, eleve, sujet).await;
    assert_eq!(fils.len(), 1, "un seul fil attendu pour « {sujet} »");
    fils.remove(0)
}

async fn lit(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, fil: Uuid) -> bool {
    match ent.list_messages_for_actor(etab, user, role(r), fil).await {
        Ok(_) => true,
        Err(EntDbError::Forbidden) => false,
        Err(e) => panic!("lecture du fil {fil} : {e:?}"),
    }
}

async fn ecrit(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, fil: Uuid, corps: &str) -> bool {
    match ent
        .repondre_fil_for_actor(
            role(r),
            RepondreFil {
                etablissement_id: etab,
                fil_id: fil,
                corps: corps.into(),
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

async fn messages(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, fil: Uuid) -> Vec<MessageLue> {
    ent.list_messages_for_actor(etab, user, role(r), fil)
        .await
        .expect("lecture")
        .1
}

async fn dans_la_liste(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, fil: Uuid) -> bool {
    ent.list_fils_for_actor(etab, user, role(r), None, true, None)
        .await
        .map(|l| l.iter().any(|f| f.id == fil))
        .unwrap_or(false)
}

async fn compter(ent: &PgEnt, sql: &str, etab: Uuid, a: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql)
        .bind(etab)
        .bind(a)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    n
}

/// Prochain dimanche 21 h (heure de Paris) strictement dans le futur : en plage calme, et la
/// remise (lundi 7 h) est toujours à venir.
fn dimanche_21h() -> chrono::DateTime<Utc> {
    let mut jour = Utc::now().date_naive() + Duration::days(1);
    while chrono::Datelike::weekday(&jour) != chrono::Weekday::Sun {
        jour += Duration::days(1);
    }
    // 21 h à Paris = 19 h ou 20 h UTC selon la saison : 20 h UTC est toujours entre 21 h et 22 h
    // à Paris (donc en plage), et reste le même dimanche.
    Utc.from_utc_datetime(&jour.and_hms_opt(20, 0, 0).unwrap())
}

async fn activer_plage_calme(ent: &PgEnt, etab: Uuid) {
    sqlx::query("UPDATE ent_msg_reglage SET plage_calme = true, motif = NULL WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
}

// ─── D21 : le fil d'un élève n'est pas lu de ses professeurs ────────────────────────────────

#[tokio::test]
async fn m0b_d21_fil_d_eleve_illisible_des_professeurs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let (prof, cpe, dir) = (cast.prof.user_id, cast.cpe.user_id, cast.direction.user_id);
    let chloe = compte_eleve(&ent, etab, harry).await;

    // Témoin : le professeur est bien dans le périmètre de Harry (il peut ouvrir un fil).
    let _ = ouvrir_un(&ent, etab, Role::Prof, prof, harry, "Témoin périmètre").await;

    // L'élève écrit à la CPE (signalement).
    let fil = ouvrir_un(&ent, etab, Role::Eleve, chloe, harry, "Je ne me sens pas en sécurité").await;
    // Témoins : son auteur, la CPE et la direction (rôles réservés) le lisent.
    assert!(lit(&ent, etab, chloe, Role::Eleve, fil.id).await, "témoin : l'élève");
    assert!(lit(&ent, etab, cpe, Role::Cpe, fil.id).await, "témoin : la CPE");
    assert!(lit(&ent, etab, dir, Role::Direction, fil.id).await, "témoin : la direction");
    // Attaque : le professeur ne le lit ni ne l'écrit, ne le voit pas dans sa boîte.
    assert!(!lit(&ent, etab, prof, Role::Prof, fil.id).await, "D21 : lecture par un professeur");
    assert!(!ecrit(&ent, etab, prof, Role::Prof, fil.id, "Je peux aider").await);
    assert!(!dans_la_liste(&ent, etab, prof, Role::Prof, fil.id).await);
    // Ajouté au fil (il y a écrit), le professeur le lit : « qui y a écrit ».
    assert!(ecrit(&ent, etab, cpe, Role::Cpe, fil.id, "Je vous écoute").await);
}

// ─── D22 : un fil du personnel par foyer ────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d22_fil_du_personnel_cree_par_foyer_et_non_lu_des_professeurs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let (prof, cpe, dir) = (cast.prof.user_id, cast.cpe.user_id, cast.direction.user_id);
    let lily = cast.parent_lea.user_id;
    // James : second responsable, dans un autre foyer (parents séparés).
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    let foyer_james = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, harry, james, Some(foyer_james)).await;
    appareil(&ent, etab, lily, "tok-lily").await;
    appareil(&ent, etab, james, "tok-james").await;

    let t0 = Utc::now();
    let fils = ouvrir_fils(&ent, etab, Role::Direction, dir, harry, "Absences de Harry").await;
    // Un fil par foyer.
    assert_eq!(fils.len(), 2, "D22 : un fil par foyer");
    let lecteurs: std::collections::BTreeSet<Option<Uuid>> = {
        let mut set = std::collections::BTreeSet::new();
        for f in &fils {
            let (l,): (Option<Uuid>,) =
                sqlx::query_as("SELECT foyer_lecteur_id FROM ent_msg_fil WHERE id = $1")
                    .bind(f.id)
                    .fetch_one(ent.pool())
                    .await
                    .unwrap();
            set.insert(l);
        }
        set
    };
    assert_eq!(lecteurs.len(), 2);
    assert!(lecteurs.contains(&Some(cast.foyer_lea_id)) && lecteurs.contains(&Some(foyer_james)));
    let ids: Vec<Uuid> = fils.iter().map(|f| f.id).collect();
    let pour = |foyer: Uuid| {
        let ids = ids.clone();
        let pool = ent.pool().clone();
        async move {
            let (id,): (Uuid,) = sqlx::query_as(
                "SELECT id FROM ent_msg_fil WHERE id = ANY($1) AND foyer_lecteur_id = $2",
            )
            .bind(ids)
            .bind(foyer)
            .fetch_one(&pool)
            .await
            .unwrap();
            id
        }
    };
    let fil_lily = pour(cast.foyer_lea_id).await;
    let fil_james = pour(foyer_james).await;

    // Témoins : chacun lit le fil de son foyer, l'auteur lit les deux.
    assert!(lit(&ent, etab, lily, Role::Parent, fil_lily).await, "témoin : Lily");
    assert!(lit(&ent, etab, james, Role::Parent, fil_james).await, "témoin : James");
    assert!(lit(&ent, etab, dir, Role::Direction, fil_lily).await && lit(&ent, etab, dir, Role::Direction, fil_james).await);
    // Attaque : pas de lecture de l'autre foyer, ni en lecture ni en réponse.
    assert!(!lit(&ent, etab, lily, Role::Parent, fil_james).await, "D22 : Lily lit le fil de James");
    assert!(!lit(&ent, etab, james, Role::Parent, fil_lily).await, "D22 : James lit le fil de Lily");
    assert!(!ecrit(&ent, etab, james, Role::Parent, fil_lily, "Où habitez-vous ?").await);
    assert!(!dans_la_liste(&ent, etab, james, Role::Parent, fil_lily).await);
    // Le professeur de Harry ne lit aucun des deux ; la CPE (rôle réservé) les lit.
    assert!(!lit(&ent, etab, prof, Role::Prof, fil_lily).await, "D22 : un professeur lit le fil aux responsables");
    assert!(!dans_la_liste(&ent, etab, prof, Role::Prof, fil_james).await);
    assert!(lit(&ent, etab, cpe, Role::Cpe, fil_lily).await);

    // Avis : la réponse de Lily ne sonne que chez le personnel, jamais chez James.
    assert!(ecrit(&ent, etab, lily, Role::Parent, fil_lily, "Nous avons déménagé").await);
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil_lily, "Merci, noté").await);
    let pushes_james = compter(
        &ent,
        "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2",
        etab,
        james,
    )
    .await;
    let pushes_lily = compter(
        &ent,
        "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2",
        etab,
        lily,
    )
    .await;
    assert!(pushes_lily > 0, "témoin : Lily est avisée de sa réponse");
    // James a reçu l'avis de l'ouverture de son propre fil, rien de celui de Lily.
    let (messages_james,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_push_outbox
          WHERE etablissement_id = $1 AND user_id = $2 AND created_at >= $3
            AND source_id IN (SELECT id FROM ent_msg WHERE fil_id = $4)",
    )
    .bind(etab)
    .bind(james)
    .bind(t0)
    .bind(fil_lily)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(messages_james, 0, "D22 : James reçoit des avis du fil de Lily ({pushes_james} au total)");
}

// ─── D23 : l'élève majeur ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d23_fil_du_personnel_a_l_eleve_majeur_ne_va_pas_aux_parents() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let eleve = Uuid::new_v4();
    let naissance = Utc::now().date_naive() - Duration::days(365 * 19 + 10);
    for (id, nom, ne) in [(eleve, "Yasmine", naissance), (Uuid::new_v4(), "Mineure", Utc::now().date_naive() - Duration::days(365 * 15))] {
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, date_naissance, created_at, updated_at)
             VALUES ($1,$2,$3,'Test','inscrit',$4, now(), now())",
        )
        .bind(id)
        .bind(etab)
        .bind(nom)
        .bind(ne)
        .execute(ent.pool())
        .await
        .unwrap();
    }
    let mere = nouvel_utilisateur(&ent, etab, "parent", "Mere").await;
    lier(&ent, etab, eleve, mere, Some(nouveau_foyer(&ent, etab).await)).await;
    let yasmine = compte_eleve(&ent, etab, eleve).await;

    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, eleve, "Vos absences").await;
    // Témoin : l'élève lit ce qui la concerne.
    assert!(lit(&ent, etab, yasmine, Role::Eleve, fil.id).await, "D23 : l'élève majeure ne lit pas son fil");
    assert!(dans_la_liste(&ent, etab, yasmine, Role::Eleve, fil.id).await);
    // Attaque : sa mère ne le lit pas, ne le voit pas, n'est pas avisée.
    assert!(!lit(&ent, etab, mere, Role::Parent, fil.id).await, "D23 : un parent lit le fil d'un majeur");
    assert!(!dans_la_liste(&ent, etab, mere, Role::Parent, fil.id).await);
    // Témoin mineur : pour une élève mineure, le parent lit.
    let mineure: Uuid = sqlx::query_scalar("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND nom = 'Mineure'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let pere = nouvel_utilisateur(&ent, etab, "parent", "Pere").await;
    lier(&ent, etab, mineure, pere, Some(nouveau_foyer(&ent, etab).await)).await;
    let fil_mineure = ouvrir_un(&ent, etab, Role::Direction, dir, mineure, "Vos absences (mineure)").await;
    assert!(lit(&ent, etab, pere, Role::Parent, fil_mineure.id).await, "témoin : parent d'une mineure");
}

// ─── D24 : plage calme ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d24_remise_de_la_plage_calme_dans_le_fuseau_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    activer_plage_calme(&ent, etab).await;
    let remise = |y: i32, m: u32, d: u32, h: u32, mi: u32, fuseau: &'static str| {
        let ent = &ent;
        async move {
            sqlx::query("DELETE FROM ent_etablissement_parametre WHERE etablissement_id = $1")
                .bind(etab)
                .execute(ent.pool())
                .await
                .ok();
            // Heure locale `h:mi` du fuseau demandé → instant UTC.
            let (instant,): (chrono::DateTime<Utc>,) = sqlx::query_as(
                "SELECT make_timestamp($1,$2,$3,$4,$5,0) AT TIME ZONE $6",
            )
            .bind(y)
            .bind(m as i32)
            .bind(d as i32)
            .bind(h as i32)
            .bind(mi as i32)
            .bind(fuseau)
            .fetch_one(ent.pool())
            .await
            .unwrap();
            let (r,): (Option<chrono::DateTime<Utc>>,) =
                sqlx::query_as("SELECT ent_msg_remise_calme($1, $2)")
                    .bind(etab)
                    .bind(instant)
                    .fetch_one(ent.pool())
                    .await
                    .unwrap();
            // Rendu en heure locale pour comparer sans ambiguïté.
            match r {
                None => None,
                Some(r) => {
                    let (loc,): (String,) = sqlx::query_as(
                        "SELECT to_char($1 AT TIME ZONE 'Europe/Paris', 'YYYY-MM-DD HH24:MI')",
                    )
                    .bind(r)
                    .fetch_one(ent.pool())
                    .await
                    .unwrap();
                    Some(loc)
                }
            }
        }
    };
    let paris = "Europe/Paris";
    // 2026-11-02 est un lundi.
    assert_eq!(remise(2026, 11, 2, 10, 0, paris).await, None, "lundi 10 h : hors plage");
    assert_eq!(remise(2026, 11, 2, 6, 59, paris).await.as_deref(), Some("2026-11-02 07:00"));
    assert_eq!(remise(2026, 11, 3, 20, 0, paris).await.as_deref(), Some("2026-11-04 07:00"), "mardi 20 h");
    assert_eq!(remise(2026, 11, 3, 23, 30, paris).await.as_deref(), Some("2026-11-04 07:00"));
    assert_eq!(remise(2026, 11, 6, 19, 59, paris).await, None, "vendredi 19 h 59 : hors plage");
    assert_eq!(remise(2026, 11, 6, 20, 0, paris).await.as_deref(), Some("2026-11-09 07:00"), "vendredi 20 h → lundi 7 h");
    assert_eq!(remise(2026, 11, 7, 12, 0, paris).await.as_deref(), Some("2026-11-09 07:00"), "samedi midi");
    assert_eq!(remise(2026, 11, 8, 21, 0, paris).await.as_deref(), Some("2026-11-09 07:00"), "dimanche 21 h");
    // Passage à l'heure d'hiver (25 octobre 2026) : samedi 24 → lundi 26 7 h, heure locale.
    assert_eq!(remise(2026, 10, 24, 15, 0, paris).await.as_deref(), Some("2026-10-26 07:00"));
    // Fuseau de l'établissement (La Réunion, UTC+4) : 21 h locale mardi → mercredi 7 h locale.
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1,'Indian/Reunion')
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = 'Indian/Reunion'",
    )
    .bind(etab)
    .execute(ent.pool())
    .await
    .unwrap();
    let (instant,): (chrono::DateTime<Utc>,) =
        sqlx::query_as("SELECT make_timestamp(2026,11,3,21,0,0) AT TIME ZONE 'Indian/Reunion'")
            .fetch_one(ent.pool())
            .await
            .unwrap();
    let (r,): (Option<chrono::DateTime<Utc>>,) = sqlx::query_as("SELECT ent_msg_remise_calme($1,$2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let (loc,): (String,) = sqlx::query_as(
        "SELECT to_char($1 AT TIME ZONE 'Indian/Reunion', 'YYYY-MM-DD HH24:MI')",
    )
    .bind(r.unwrap())
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(loc, "2026-11-04 07:00");
    // Plage désactivée : jamais de remise différée.
    sqlx::query("UPDATE ent_msg_reglage SET plage_calme = false, motif = 'test' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    let (r,): (Option<chrono::DateTime<Utc>>,) = sqlx::query_as("SELECT ent_msg_remise_calme($1,$2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(r.is_none());
}

#[tokio::test]
async fn m0b_d24_message_du_personnel_differe_invisible_jusqu_a_la_remise_et_avis_planifie() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe) = (cast.direction.user_id, cast.cpe.user_id);
    let lily = cast.parent_lea.user_id;
    let harry = cast.eleve_lea_id;
    appareil(&ent, etab, lily, "tok-lily-plage").await;

    // Témoin : plage désactivée (labo), le message est remis tout de suite, avec son avis.
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Hors plage").await;
    assert!(lit(&ent, etab, lily, Role::Parent, fil.id).await, "témoin : remis immédiatement");
    assert!(
        compter(&ent, "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2", etab, lily).await > 0,
        "témoin : avis immédiat"
    );

    // Dimanche 21 h : plage calme active.
    activer_plage_calme(&ent, etab).await;
    let maintenant = dimanche_21h();
    let avant_push = compter(&ent, "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2", etab, lily).await;
    let envoye = ent
        .repondre_fil_a(
            role(Role::Direction),
            RepondreFil {
                etablissement_id: etab,
                fil_id: fil.id,
                corps: "Message du dimanche soir".into(),
                actor_user_id: dir,
            },
            OptionsEnvoiMessage::default(),
            maintenant,
        )
        .await
        .unwrap();
    let remise = envoye.remise_prevue_le.expect("D24 : message différé");
    assert!(remise > Utc::now(), "remise à venir");
    // L'auteur le voit ; la famille et les collègues, non.
    let vus_auteur = messages(&ent, etab, dir, Role::Direction, fil.id).await;
    assert!(vus_auteur.iter().any(|m| m.message.id == envoye.message.id), "l'auteur voit son message");
    let vus_lily = messages(&ent, etab, lily, Role::Parent, fil.id).await;
    assert!(!vus_lily.iter().any(|m| m.message.id == envoye.message.id), "D24 : la famille lit un message différé");
    let vus_cpe = messages(&ent, etab, cpe, Role::Cpe, fil.id).await;
    assert!(!vus_cpe.iter().any(|m| m.message.id == envoye.message.id), "D24 : un collègue lit un message différé");
    // Aucun avis maintenant ; un avis planifié à la remise par la chaîne S2-8.
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2", etab, lily).await,
        avant_push,
        "D24 : push pendant la plage calme"
    );
    let planifie: Vec<(chrono::DateTime<Utc>, String)> = sqlx::query_as(
        "SELECT planifie_pour, statut FROM ent_notification_envoi
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND source_id = $3",
    )
    .bind(etab)
    .bind(lily)
    .bind(envoye.message.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(planifie.len(), 1);
    assert_eq!(planifie[0].1, "en_attente");
    assert_eq!(planifie[0].0.timestamp(), remise.timestamp());
    // Annulable par son auteur seul, avant la remise.
    assert!(
        ent.annuler_message_differe_for_actor(etab, cpe, role(Role::Cpe), envoye.message.id).await.is_err(),
        "un collègue annule un message différé"
    );
    // La remise venue, la famille le lit.
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() - interval '1 minute' WHERE id = $1")
        .bind(envoye.message.id)
        .execute(ent.pool())
        .await
        .unwrap();
    let vus_lily = messages(&ent, etab, lily, Role::Parent, fil.id).await;
    assert!(vus_lily.iter().any(|m| m.message.id == envoye.message.id), "remis après la fin de la plage");
}

#[tokio::test]
async fn m0b_d24_fil_ouvert_en_plage_calme_absent_de_la_liste_famille_puis_annulation() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    activer_plage_calme(&ent, etab).await;
    let ouverts = ent
        .ouvrir_fils_a(
            role(Role::Direction),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "Dimanche soir".into(),
                premier_message: "Bonsoir".into(),
                actor_user_id: dir,
            },
            OptionsEnvoiMessage::default(),
            None,
            dimanche_21h(),
        )
        .await
        .unwrap();
    let o = &ouverts[0];
    assert!(o.remise_prevue_le.is_some());
    assert!(!dans_la_liste(&ent, etab, lily, Role::Parent, o.fil.id).await, "D24 : fil différé listé chez la famille");
    assert!(!lit(&ent, etab, lily, Role::Parent, o.fil.id).await);
    // Annulé par son auteur, le message disparaît.
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), o.message.id)
        .await
        .unwrap();
    assert!(
        ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), o.message.id).await.is_err(),
        "annulation répétée"
    );
}

#[tokio::test]
async fn m0b_d24_derogation_de_la_direction_tracee_et_reglage_avec_motif() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, lily, harry) = (cast.direction.user_id, cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    activer_plage_calme(&ent, etab).await;
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Urgence").await;
    let cmd = |user: Uuid| RepondreFil {
        etablissement_id: etab,
        fil_id: fil.id,
        corps: "Sortie de l'école annulée demain".into(),
        actor_user_id: user,
    };
    // Sans droit, la dérogation est refusée (la CPE n'a pas `messagerie.urgence_hors_plage`).
    let r = ent
        .repondre_fil_a(
            role(Role::Cpe),
            cmd(cpe),
            OptionsEnvoiMessage { urgence_motif: Some("urgent".into()) },
            dimanche_21h(),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // La direction remet immédiatement, avec motif tracé dans la ligne du message.
    let m = ent
        .repondre_fil_a(
            role(Role::Direction),
            cmd(dir),
            OptionsEnvoiMessage { urgence_motif: Some("Alerte météo".into()) },
            dimanche_21h(),
        )
        .await
        .unwrap();
    assert!(m.remise_prevue_le.is_none());
    let (motif, par): (Option<String>, Option<Uuid>) =
        sqlx::query_as("SELECT derogation_motif, derogation_par FROM ent_msg WHERE id = $1")
            .bind(m.message.id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!((motif.as_deref(), par), (Some("Alerte météo"), Some(dir)));
    assert!(messages(&ent, etab, lily, Role::Parent, fil.id).await.iter().any(|x| x.message.id == m.message.id));

    // Réglage : réduire ou désactiver exige un motif ; le défaut national s'enregistre sans motif.
    let saisie = |plage: bool, debut: i16, motif: Option<&str>| ReglagePlageCalmeSaisie {
        plage_calme: plage,
        debut_heure: debut,
        fin_heure: 7,
        weekend: true,
        motif: motif.map(Into::into),
    };
    assert!(matches!(
        ent.definir_reglage_plage_calme_for_actor(etab, dir, role(Role::Direction), saisie(false, 20, None)).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.definir_reglage_plage_calme_for_actor(etab, cpe, role(Role::Cpe), saisie(true, 20, None)).await,
        Err(EntDbError::Forbidden)
    ));
    let r = ent
        .definir_reglage_plage_calme_for_actor(etab, dir, role(Role::Direction), saisie(true, 20, None))
        .await
        .unwrap();
    assert!(r.plage_calme && r.motif.is_none());
    let r = ent
        .definir_reglage_plage_calme_for_actor(etab, dir, role(Role::Direction), saisie(true, 21, Some("Internat")))
        .await
        .unwrap();
    assert_eq!((r.debut_heure, r.motif.as_deref()), (21, Some("Internat")));
}

// ─── D25 : lien créé après une interdiction par libellé ─────────────────────────────────────

#[tokio::test]
async fn m0b_d25_lien_cree_apres_une_interdiction_par_libelle_est_a_verifier() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let harry = cast.eleve_lea_id;
    // Interdiction de communication visant « M. Interdit » (libellé, personne absente de la fiche).
    let interdiction = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction
            (id, etablissement_id, eleve_id, personne_libelle, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,'M. Interdit', ARRAY['communication'], 'decision_justice', 'ne pas communiquer',
                 ent_aujourdhui_etab($2) - 1, $4)",
    )
    .bind(interdiction)
    .bind(etab)
    .bind(harry)
    .bind(dir)
    .execute(ent.pool())
    .await
    .unwrap();
    // Témoin : le responsable déjà lié avant l'interdiction reste communicant.
    let lily = cast.parent_lea.user_id;
    let communicant = |user: Uuid| {
        let ent = &ent;
        async move {
            compter(
                ent,
                "SELECT count(*) FROM ent_eleve_lien_communicant WHERE etablissement_id = $1 AND user_id = $2",
                etab,
                user,
            )
            .await
                > 0
        }
    };
    // Le lien de Lily précède l'interdiction : on rajeunit l'interdiction après lui.
    assert!(communicant(lily).await, "témoin : lien antérieur");
    // L'homme interdit se fait lier au guichet sous une autre identité.
    let intrus = nouvel_utilisateur(&ent, etab, "parent", "Intrus").await;
    let lien = lier(&ent, etab, harry, intrus, Some(nouveau_foyer(&ent, etab).await)).await;
    assert!(!communicant(intrus).await, "D25 : lien postérieur à l'interdiction communicant");
    // Il n'est donc ni lecteur ni destinataire dans la messagerie.
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Après l'interdiction").await;
    assert!(!lit(&ent, etab, intrus, Role::Parent, fil.id).await);
    assert!(communicant(lily).await, "la mère déjà liée n'est pas touchée");
    // À vérifier : la direction le voit, décide ; un professeur ne le peut pas.
    let a_verifier = ent
        .liens_a_verifier_for_actor(etab, dir, role(Role::Direction))
        .await
        .unwrap();
    assert!(a_verifier.iter().any(|l| l.lien_id == lien && l.interdiction_id == interdiction));
    assert!(matches!(
        ent.liens_a_verifier_for_actor(etab, cast.prof.user_id, role(Role::Prof)).await,
        Err(EntDbError::Forbidden)
    ));
    // Maintenu inactif : toujours exclu ; confirmé : le lien redevient communicant.
    ent.verifier_lien_for_actor(etab, dir, role(Role::Direction), lien, interdiction, false, Some("Même personne"))
        .await
        .unwrap();
    assert!(!communicant(intrus).await);
    ent.verifier_lien_for_actor(etab, dir, role(Role::Direction), lien, interdiction, true, Some("Autre personne, vérifié"))
        .await
        .unwrap();
    assert!(communicant(intrus).await, "lien confirmé");
    // Un fil ouvert après la confirmation est créé aussi pour son foyer.
    let apres = ouvrir_fils(&ent, etab, Role::Direction, dir, harry, "Après la confirmation").await;
    let mut lu = false;
    for f in &apres {
        lu |= lit(&ent, etab, intrus, Role::Parent, f.id).await;
    }
    assert!(lu, "lien confirmé : aucun fil à son foyer");
    // Sans motif, pas de décision.
    assert!(matches!(
        ent.verifier_lien_for_actor(etab, dir, role(Role::Direction), lien, interdiction, true, None).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ─── D26 : l'action `messagerie.fils_reserves.lire` ─────────────────────────────────────────

#[tokio::test]
async fn m0b_d26_un_aed_avec_le_droit_traite_les_messages_des_familles() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let yanis = nouvel_utilisateur(&ent, etab, "personnel", "Yanis").await;
    // Lily écrit à l'établissement (fil privé).
    let fil = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Mon fils sera absent lundi").await;
    // Témoin : la CPE (rôle de base réservé) le lit.
    assert!(lit(&ent, etab, cast.cpe.user_id, Role::Cpe, fil.id).await);
    // L'AED, sans le droit : ni lecture ni liste (il n'a même pas `messagerie.eleve`).
    let r = ent.list_messages_for_actor(etab, yanis, role(Role::Personnel), fil.id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Rôle D-017 « AED vie scolaire » : messagerie sur tout l'établissement + fils réservés.
    let r = ent
        .creer_role_for_actor(
            etab,
            dir,
            role(Role::Direction),
            RoleSaisie {
                nom: format!("AED vie scolaire {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![
                    Droit { action: Action::MessagerieEleve, portee: Portee::Etablissement, cibles: vec![] },
                    Droit { action: Action::MessagerieFilsReserves, portee: Portee::Etablissement, cibles: vec![] },
                ],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir, role(Role::Direction), r.id, yanis, None)
        .await
        .unwrap();
    assert!(lit(&ent, etab, yanis, Role::Personnel, fil.id).await, "D26 : l'AED avec le droit ne lit pas");
    assert!(dans_la_liste(&ent, etab, yanis, Role::Personnel, fil.id).await);
    // La file « non traités » l'affiche ; sans le droit, elle est refusée.
    let file = ent
        .list_fils_non_traites_for_actor(etab, yanis, role(Role::Personnel), 50)
        .await
        .unwrap();
    assert!(file.iter().any(|n| n.fil.id == fil.id));
    assert!(matches!(
        ent.list_fils_non_traites_for_actor(etab, cast.prof.user_id, role(Role::Prof), 50).await,
        Err(EntDbError::Forbidden)
    ));
}

// ─── D28 : garde sur les profils santé et psychologue ───────────────────────────────────────

#[tokio::test]
async fn m0b_d28_messagerie_non_attribuable_aux_profils_sante_et_psychologue() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let infirmiere = nouvel_utilisateur(&ent, etab, "infirmerie", "Infirmiere").await;
    let droits = || {
        vec![Droit { action: Action::MessagerieEleve, portee: Portee::Etablissement, cibles: vec![] }]
    };
    let saisie = |modele: Option<&str>| RoleSaisie {
        nom: format!("Messagerie {}", &Uuid::new_v4().simple().to_string()[..6]),
        description: None,
        modele: modele.map(Into::into),
        droits: droits(),
    };
    // Témoin : le même rôle, modelé sur un profil d'enseignement, est créé et attribuable à un AED.
    let role_ok = ent.creer_role_for_actor(etab, dir, role(Role::Direction), saisie(Some("prof"))).await.unwrap();
    // Un rôle modelé sur le profil infirmerie ne peut pas recevoir ces droits.
    assert!(ent.creer_role_for_actor(etab, dir, role(Role::Direction), saisie(Some("infirmerie"))).await.is_err());
    assert!(ent.creer_role_for_actor(etab, dir, role(Role::Direction), saisie(Some("psy"))).await.is_err());
    // L'attribution à un profil santé est refusée.
    let r = ent.attribuer_role_for_actor(etab, dir, role(Role::Direction), role_ok.id, infirmiere, None).await;
    assert!(r.is_err(), "D28 : attribution à l'infirmerie");
    // Même posé en base (rôle attribué avant la garde, délégation), le droit est retiré à la lecture.
    sqlx::query(
        "INSERT INTO ent_role_attribution (id, etablissement_id, role_id, user_id, debut, source)
         VALUES ($1,$2,$3,$4, ent_aujourdhui_etab($2), 'manuelle')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(role_ok.id)
    .bind(infirmiere)
    .execute(ent.pool())
    .await
    .unwrap();
    let droits = ent.droits_effectifs(etab, infirmiere, "infirmerie").await.unwrap();
    assert!(
        !droits.iter().any(|d| d.action == Action::MessagerieEleve || d.action == Action::MessagerieFilsReserves),
        "D28 : droit de messagerie effectif pour l'infirmerie"
    );
    // Témoin : un AED (profil personnel) garde le droit.
    let aed = nouvel_utilisateur(&ent, etab, "personnel", "Aed").await;
    ent.attribuer_role_for_actor(etab, dir, role(Role::Direction), role_ok.id, aed, None).await.unwrap();
    assert!(ent.droits_effectifs(etab, aed, "personnel").await.unwrap().iter().any(|d| d.action == Action::MessagerieEleve));
}

// ─── MSG-329 et D13 : file des messages non traités, personnel prévenu ─────────────────────

#[tokio::test]
async fn m0b_msg329_d13_file_non_traites_et_personnel_prevenu() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, dir, prof, lily, harry) = (
        cast.cpe.user_id,
        cast.direction.user_id,
        cast.prof.user_id,
        cast.parent_lea.user_id,
        cast.eleve_lea_id,
    );
    let notifs = |user: Uuid| {
        let ent = &ent;
        async move {
            compter(
                ent,
                "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'messagerie_famille'",
                etab,
                user,
            )
            .await
        }
    };
    let avant = (notifs(cpe).await, notifs(dir).await);
    let fil = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Question sur la cantine").await;
    // D13 : sans personnel déjà intervenu, ceux qui traitent les messages des familles sont prévenus.
    assert!(notifs(cpe).await > avant.0 && notifs(dir).await > avant.1, "D13 : personnel non prévenu");
    assert_eq!(notifs(prof).await, 0, "le professeur n'est pas prévenu d'un fil qu'il ne lit pas");
    // Le texte de l'avis est neutre : jamais le sujet ni le contenu.
    let (titre, corps): (String, String) = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'messagerie_famille' LIMIT 1",
    )
    .bind(etab)
    .bind(cpe)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(!titre.contains("cantine") && !corps.contains("cantine"));
    // File « non traités » : le fil y est ; la CPE répond, il en sort ; la famille relance, il revient.
    let file = ent.list_fils_non_traites_for_actor(etab, cpe, role(Role::Cpe), 100).await.unwrap();
    assert!(file.iter().any(|n| n.fil.id == fil.id), "MSG-329 : fil absent de la file");
    assert!(ecrit(&ent, etab, cpe, Role::Cpe, fil.id, "Bonjour, je regarde").await);
    let file = ent.list_fils_non_traites_for_actor(etab, cpe, role(Role::Cpe), 100).await.unwrap();
    assert!(!file.iter().any(|n| n.fil.id == fil.id), "fil traité encore dans la file");
    // La CPE (qui a écrit) est prévenue de la relance, la direction ne l'est plus.
    let avant_cpe = notifs(cpe).await;
    let avant_dir = notifs(dir).await;
    assert!(ecrit(&ent, etab, lily, Role::Parent, fil.id, "Merci, et pour le mercredi ?").await);
    assert!(notifs(cpe).await > avant_cpe, "D13 : la CPE n'est pas prévenue de la relance");
    assert_eq!(notifs(dir).await, avant_dir, "la direction n'est prévenue qu'au premier message");
    let file = ent.list_fils_non_traites_for_actor(etab, cpe, role(Role::Cpe), 100).await.unwrap();
    assert!(file.iter().any(|n| n.fil.id == fil.id));
    // Les plus anciens d'abord.
    let dates: Vec<_> = file.iter().map(|n| n.en_attente_depuis).collect();
    assert!(dates.windows(2).all(|w| w[0] <= w[1]));
}

// ─── D11 : une réponse désarchive ───────────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d11_la_reponse_d_un_parent_desarchive_le_fil() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, lily, harry) = (cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let fil = ouvrir_un(&ent, etab, Role::Cpe, cpe, harry, "À archiver").await;
    ent.archiver_fil_for_actor(
        role(Role::Cpe),
        domain_eleve::ArchiverFil { etablissement_id: etab, fil_id: fil.id, actor_user_id: cpe },
    )
    .await
    .unwrap();
    let archive = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        let id = fil.id;
        async move {
            let (a,): (bool,) = sqlx::query_as("SELECT archive FROM ent_msg_fil WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
            a
        }
    };
    assert!(archive(&ent).await, "témoin : archivé");
    // Désarchivage explicite par le personnel.
    ent.desarchiver_fil_for_actor(
        role(Role::Cpe),
        domain_eleve::ArchiverFil { etablissement_id: etab, fil_id: fil.id, actor_user_id: cpe },
    )
    .await
    .unwrap();
    assert!(!archive(&ent).await, "D11 : pas de désarchivage");
    // Réponse d'un parent sur un fil archivé : il revient dans la boîte.
    ent.archiver_fil_for_actor(
        role(Role::Cpe),
        domain_eleve::ArchiverFil { etablissement_id: etab, fil_id: fil.id, actor_user_id: cpe },
    )
    .await
    .unwrap();
    assert!(archive(&ent).await);
    assert!(ecrit(&ent, etab, lily, Role::Parent, fil.id, "Une dernière question").await);
    assert!(!archive(&ent).await, "D11 : la réponse reste dans l'archive");
    let boite = ent
        .list_fils_for_actor(etab, cpe, role(Role::Cpe), None, false, None)
        .await
        .unwrap();
    assert!(boite.iter().any(|f| f.id == fil.id), "D11 : absent de la boîte (hors archives)");
}

// ─── D12 : pagination et recherche ──────────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d12_la_boite_du_personnel_se_pagine_et_se_recherche_sur_le_serveur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, harry) = (cast.cpe.user_id, cast.eleve_lea_id);
    // 55 fils : la boîte était tronquée à 50, sans suite.
    let mut ids = Vec::new();
    for i in 0..55 {
        let sujet = if i == 3 { "Voyage à Rome".to_string() } else { format!("Fil numéro {i}") };
        ids.push(ouvrir_un(&ent, etab, Role::Cpe, cpe, harry, &sujet).await.id);
    }
    let page = |apres: Option<String>, limite: i64, q: Option<&str>| {
        let ent = &ent;
        let q = q.map(str::to_string);
        async move {
            ent.list_fils_page_for_actor(
                etab,
                cpe,
                role(Role::Cpe),
                ParamsListeFils { include_archives: true, limite: Some(limite), apres, recherche: q, ..Default::default() },
            )
            .await
            .unwrap()
        }
    };
    let mut vus: Vec<Uuid> = Vec::new();
    let mut curseur = None;
    let mut pages = 0;
    loop {
        let p = page(curseur.clone(), 20, None).await;
        assert!(p.fils.len() <= 20);
        vus.extend(p.fils.iter().map(|f| f.id));
        pages += 1;
        match p.suivant {
            Some(c) => curseur = Some(c),
            None => break,
        }
        assert!(pages < 10, "pagination sans fin");
    }
    let uniques: std::collections::BTreeSet<_> = vus.iter().collect();
    assert_eq!(vus.len(), uniques.len(), "doublon entre pages");
    for id in &ids {
        assert!(vus.contains(id), "D12 : fil {id} jamais atteignable");
    }
    assert!(pages >= 3);
    // Ordre : du plus récent au plus ancien.
    let (premiers, _) = (page(None, 5, None).await, ());
    assert_eq!(premiers.fils[0].id, *ids.last().unwrap());
    // Recherche serveur sur le sujet (un fil lointain, absent de la première page).
    let r = page(None, 50, Some("rome")).await;
    assert_eq!(r.fils.len(), 1);
    assert_eq!(r.fils[0].id, ids[3]);
    // Le joker n'est pas interprété.
    assert!(page(None, 50, Some("%")).await.fils.is_empty());
    // Curseur mal formé : refusé.
    assert!(ent
        .list_fils_page_for_actor(
            etab,
            cpe,
            role(Role::Cpe),
            ParamsListeFils { apres: Some("n'importe quoi".into()), ..Default::default() }
        )
        .await
        .is_err());
}

#[test]
fn m0b_d12_curseur_aller_retour() {
    let c = CurseurFils { rang: 1, dernier_message: Utc::now(), fil_id: Uuid::new_v4() };
    let relu = CurseurFils::decoder(&c.encoder()).expect("décodage");
    assert_eq!(relu.rang, c.rang);
    assert_eq!(relu.fil_id, c.fil_id);
    assert_eq!(relu.dernier_message.timestamp_micros(), c.dernier_message.timestamp_micros());
    assert!(CurseurFils::decoder("2|2026-01-01T00:00:00Z|x").is_none());
    assert!(CurseurFils::decoder("0|2026-01-01T00:00:00Z|00000000-0000-0000-0000-000000000000|x").is_none());
}

// ─── D14 : une panne du fournisseur push n'est plus invisible ───────────────────────────────

#[tokio::test]
async fn m0b_d14_echec_d_envoi_push_journalise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    // Jeton vide après nettoyage : le fournisseur simulé refuse l'envoi.
    appareil(&ent, etab, lily, "   ").await;
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Panne du fournisseur").await;
    let (n_echec,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_push_outbox
          WHERE etablissement_id = $1 AND user_id = $2 AND status = 'failed'",
    )
    .bind(etab)
    .bind(lily)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n_echec, 1, "D14 : l'échec d'envoi n'est pas journalisé");
    // Le fil, lui, est bien ouvert : la panne n'a pas annulé le geste.
    assert!(lit(&ent, etab, lily, Role::Parent, fil.id).await);
}

// ─── D15 : l'accusé est posé par le côté destinataire ───────────────────────────────────────

#[tokio::test]
async fn m0b_d15_accuse_pose_par_le_destinataire_seulement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, dir, lily, harry) = (cast.cpe.user_id, cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    // Lily écrit ; la CPE répond (elle participe) ; la direction (supervision) ouvre le fil.
    let fil = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Question").await;
    assert!(ecrit(&ent, etab, cpe, Role::Cpe, fil.id, "Je regarde").await);
    let msg_lily = {
        let (id,): (Uuid,) = sqlx::query_as(
            "SELECT id FROM ent_msg WHERE fil_id = $1 AND auteur_user_id = $2",
        )
        .bind(fil.id)
        .bind(lily)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        id
    };
    let accuses = |message: Uuid| {
        let ent = &ent;
        async move {
            compter(ent, "SELECT count(*) FROM ent_msg_accuse WHERE etablissement_id = $1 AND message_id = $2", etab, message).await
        }
    };
    // Attaque : la direction ouvre le fil sans y avoir écrit : aucun accusé sur le message de Lily.
    assert!(lit(&ent, etab, dir, Role::Direction, fil.id).await);
    assert_eq!(accuses(msg_lily).await, 0, "D15 : un lecteur de supervision pose un accusé");
    // Témoin : la CPE, qui participe, lit le message de la famille : accusé posé.
    assert!(lit(&ent, etab, cpe, Role::Cpe, fil.id).await);
    assert_eq!(accuses(msg_lily).await, 1, "témoin : accusé du destinataire");
    // Le message du personnel n'est pas accusé par un autre membre du personnel.
    let msg_cpe: Uuid = sqlx::query_scalar("SELECT id FROM ent_msg WHERE fil_id = $1 AND auteur_user_id = $2")
        .bind(fil.id)
        .bind(cpe)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(accuses(msg_cpe).await, 0, "D15 : la direction accuse un message de la CPE");
    // La famille, destinataire, accuse le message du personnel.
    assert!(lit(&ent, etab, lily, Role::Parent, fil.id).await);
    assert_eq!(accuses(msg_cpe).await, 1, "témoin : accusé de la famille");
}

// ─── D16 : taille de pièce bornée au CHECK, 413 clair ───────────────────────────────────────

#[tokio::test]
async fn m0b_d16_piece_au_dela_de_256_kio_refusee_en_413_meme_si_le_reglage_est_plus_haut() {
    let Some((mut ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, harry) = (cast.direction.user_id, cast.eleve_lea_id);
    // Réglage d'exploitation relevé à 1 Mio (ENT_UPLOAD_MAX_MESSAGE) : la colonne reste à 256 Kio.
    ent.installer_politique_fichiers(ent_object::upload::PolitiqueFichiers::nouvelle(
        ent_object::upload::TaillesMax {
            piece_message: 1024 * 1024,
            ..ent_object::upload::TaillesMax::default()
        },
    ));
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Certificat").await;
    let message = messages(&ent, etab, dir, Role::Direction, fil.id).await[0].message.id;
    let pdf = |octets: usize| {
        let mut v = b"%PDF-1.4\n".to_vec();
        v.resize(octets, b' ');
        v.extend_from_slice(b"\n%%EOF\n");
        v
    };
    // Témoin : une pièce de 100 Kio passe.
    attacher(&ent, etab, message, dir, pdf(100 * 1024)).await.expect("témoin : pièce de 100 Kio");
    // 300 Kio : refus de taille (HTTP 413), jamais une erreur de base (HTTP 500).
    match attacher(&ent, etab, message, dir, pdf(300 * 1024)).await {
        Err(EntDbError::Fichier(e)) => assert!(e.est_taille(), "{e:?}"),
        autre => panic!("D16 : attendu un refus de taille, obtenu {autre:?}"),
    }
    // Au plus 5 pièces par message (R-M44).
    for _ in 0..4 {
        attacher(&ent, etab, message, dir, pdf(10 * 1024)).await.expect("pièces 2 à 5");
    }
    assert!(matches!(
        attacher(&ent, etab, message, dir, pdf(10 * 1024)).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ─── D17 : longueurs et fil de modèle ───────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d17_longueurs_maximales_et_fil_de_modele_prevenant_la_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let ouvrir = |sujet: String, corps: String| {
        ent.ouvrir_fil_for_actor(
            role(Role::Direction),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet,
                premier_message: corps,
                actor_user_id: dir,
            },
        )
    };
    // Témoins : exactement 200 et 20 000 caractères passent.
    ouvrir("s".repeat(200), "m".repeat(20_000)).await.expect("témoin : bornes incluses");
    // Au-delà : refus.
    assert!(matches!(
        ouvrir("s".repeat(201), "ok".into()).await,
        Err(EntDbError::Domain(DomainError::MessageTropLong))
    ));
    assert!(matches!(
        ouvrir("ok".into(), "m".repeat(20_001)).await,
        Err(EntDbError::Domain(DomainError::MessageTropLong))
    ));
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Réponses").await;
    let r = ent
        .repondre_fil_for_actor(
            role(Role::Direction),
            RepondreFil { etablissement_id: etab, fil_id: fil.id, corps: "m".repeat(20_001), actor_user_id: dir },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::MessageTropLong))));
    // La base elle-même refuse (garde-fou si une écriture contourne le service).
    let direct = sqlx::query("UPDATE ent_msg_fil SET sujet = $2 WHERE id = $1")
        .bind(fil.id)
        .bind("s".repeat(201))
        .execute(ent.pool())
        .await;
    assert!(direct.is_err(), "CHECK de longueur du sujet");
    // Fil ouvert depuis un modèle : la famille est prévenue (pas de push avant M0b).
    appareil(&ent, etab, lily, "tok-lily-modele").await;
    let modele = ent
        .creer_msg_modele_for_actor(
            role(Role::Direction),
            domain_eleve::CreerMsgModele {
                etablissement_id: etab,
                titre: "Absence".into(),
                sujet: "Absence de {prenom}".into(),
                corps: "Bonjour, {prenom} est absent.".into(),
                actor_user_id: dir,
            },
        )
        .await
        .unwrap();
    let avant = compter(&ent, "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2", etab, lily).await;
    let (fil_modele, _) = ent
        .ouvrir_fil_depuis_modele_for_actor(role(Role::Direction), etab, dir, modele.id, harry)
        .await
        .unwrap();
    assert!(
        compter(&ent, "SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2", etab, lily).await > avant,
        "D17 : aucun push pour un fil de modèle"
    );
    assert!(lit(&ent, etab, lily, Role::Parent, fil_modele.id).await);
}

// ─── D8, D18, D27 : annonces ────────────────────────────────────────────────────────────────

fn annonce(etab: Uuid, auteur: Uuid, audience: &str, classe: Option<&str>) -> CreerAnnonce {
    CreerAnnonce {
        etablissement_id: etab,
        titre: "Décès d'un camarade".into(),
        corps: "Texte très sensible de l'annonce".into(),
        audience: audience.into(),
        cible_classe: classe.map(Into::into),
        actor_user_id: auteur,
    }
}

async fn expedier(ent: &PgEnt) {
    let x = ent.expediteur(crate::notifications::ConfigExpedition::default());
    x.expedier(Utc::now()).await.unwrap();
}

#[tokio::test]
async fn m0b_d8_d27_annonce_apercu_confirmation_retrait_et_enveloppe_neutre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    appareil(&ent, etab, lily, "tok-lily-annonce").await;
    sqlx::query("UPDATE ent_user SET telephone = '0612345678' WHERE id = $1").bind(lily).execute(ent.pool()).await.unwrap();
    let t0 = Utc::now();
    // Aperçu : rien ne part, un chiffre est annoncé.
    let ap = ent
        .apercu_annonce_for_actor(role(Role::Direction), annonce(etab, dir, "parents", None))
        .await
        .unwrap();
    assert!(ap.nb_eleves > 0 && ap.nb_responsables > 0, "aperçu chiffré");
    assert_eq!(ap.annonce.statut, StatutAnnonce::Apercu);
    let envois_annonce = |id: Uuid| {
        let ent = &ent;
        async move {
            compter(
                ent,
                "SELECT count(*) FROM ent_notification_envoi WHERE etablissement_id = $1 AND source_id = $2",
                etab,
                id,
            )
            .await
        }
    };
    assert_eq!(envois_annonce(ap.annonce.id).await, 0, "D8 : envois partis avant confirmation");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND source_event_id = $2", etab, ap.annonce.id).await,
        0
    );
    // Invisible de la famille tant qu'elle n'est pas confirmée ; visible du personnel.
    let visibles_famille = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_annonce WHERE etablissement_id = $1 AND statut = 'publiee'",
            )
            .bind(etab)
            .fetch_one(&pool)
            .await
            .unwrap();
            n
        }
    };
    assert_eq!(visibles_famille(&ent).await, 0);
    let vue_famille = ent
        .list_annonces_famille_for_actor(etab, lily, role(Role::Parent), harry)
        .await
        .unwrap();
    assert!(vue_famille.iter().all(|a| a.id != ap.annonce.id), "aperçu visible de la famille");
    assert!(ent.list_annonces_for_actor(etab, role(Role::Direction), 100).await.unwrap().iter().any(|a| a.id == ap.annonce.id));
    // Un professeur ne confirme ni ne retire.
    assert!(matches!(
        ent.confirmer_annonce_for_actor(role(Role::Prof), etab, cast.prof.user_id, ap.annonce.id).await,
        Err(EntDbError::Forbidden)
    ));
    // Confirmation : publiée, mise en file (aucun envoi synchrone dans la requête).
    let a = ent.confirmer_annonce_for_actor(role(Role::Direction), etab, dir, ap.annonce.id).await.unwrap();
    assert_eq!(a.statut, StatutAnnonce::Publiee);
    assert!(envois_annonce(a.id).await > 0, "mise en file S2-8");
    let (sync,): (i64,) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_mail_outbox WHERE etablissement_id = $1 AND created_at >= $2)
              + (SELECT count(*) FROM ent_sms_outbox WHERE etablissement_id = $1 AND created_at >= $2)
              + (SELECT count(*) FROM ent_push_outbox WHERE etablissement_id = $1 AND created_at >= $2)",
    )
    .bind(etab)
    .bind(t0)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(sync, 0, "D8 : le fournisseur est appelé dans la requête");
    // Elle est maintenant visible de la famille, avec une notification dans l'ENT.
    assert!(ent
        .list_annonces_famille_for_actor(etab, lily, role(Role::Parent), harry)
        .await
        .unwrap()
        .iter()
        .any(|x| x.id == a.id));
    assert!(compter(&ent, "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND source_event_id = $2", etab, a.id).await > 0);
    // Réconfirmer est refusé (idempotence).
    assert!(ent.confirmer_annonce_for_actor(role(Role::Direction), etab, dir, a.id).await.is_err());
    // D27 : l'expédition envoie un e-mail NEUTRE — ni titre ni corps de l'annonce.
    expedier(&ent).await;
    let mails: Vec<(String, String)> = sqlx::query_as(
        "SELECT subject, body_text FROM ent_mail_outbox WHERE etablissement_id = $1 AND created_at >= $2",
    )
    .bind(etab)
    .bind(t0)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(!mails.is_empty(), "témoin : un e-mail est parti à l'expédition");
    for (sujet, corps) in &mails {
        assert!(
            !sujet.contains("Décès") && !corps.contains("sensible") && !corps.contains("Décès"),
            "D27 : le contenu de l'annonce est dans l'e-mail : {sujet} / {corps}"
        );
    }
    // Retrait : motif obligatoire ; envois en attente annulés ; plus visible de la famille.
    let b = ent
        .confirmer_annonce_for_actor(
            role(Role::Direction),
            etab,
            dir,
            ent.apercu_annonce_for_actor(role(Role::Direction), annonce(etab, dir, "parents", None)).await.unwrap().annonce.id,
        )
        .await
        .unwrap();
    assert!(envois_annonce(b.id).await > 0);
    assert!(matches!(
        ent.retirer_annonce_for_actor(role(Role::Direction), etab, dir, b.id, "  ").await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.retirer_annonce_for_actor(role(Role::Direction), etab, dir, b.id, "Envoyée par erreur").await.unwrap();
    let (en_attente,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification_envoi WHERE etablissement_id = $1 AND source_id = $2 AND statut = 'en_attente'",
    )
    .bind(etab)
    .bind(b.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(en_attente, 0, "D8 : envois non annulés après retrait");
    expedier(&ent).await;
    let (partis,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification_envoi WHERE etablissement_id = $1 AND source_id = $2 AND statut IN ('envoye', 'simule')",
    )
    .bind(etab)
    .bind(b.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(partis, 0, "D8 : un envoi retiré est parti");
    assert!(ent
        .list_annonces_famille_for_actor(etab, lily, role(Role::Parent), harry)
        .await
        .unwrap()
        .iter()
        .all(|x| x.id != b.id), "annonce retirée encore visible");
    assert!(ent.retirer_annonce_for_actor(role(Role::Direction), etab, dir, b.id, "Encore").await.is_err());
}

#[tokio::test]
async fn m0b_d18_annonce_ciblee_par_identifiant_de_classe_survit_au_renommage() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let annee = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin)
         VALUES ($1,$2,'M0b-d18', date '2026-01-01', date '2030-12-31')",
    )
    .bind(annee)
    .bind(etab)
    .execute(ent.pool())
    .await
    .unwrap();
    let classe = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,'6Z','Sixième Z')")
        .bind(classe)
        .bind(etab)
        .bind(annee)
        .execute(ent.pool())
        .await
        .unwrap();
    // Un élève dans la classe (affectation du jour) avec son responsable.
    let eleve = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at)
         VALUES ($1,$2,'Zed','Zoe','inscrit', now(), now())",
    )
    .bind(eleve)
    .bind(etab)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut)
         VALUES ($1,$2,$3,$4,$5, ent_aujourdhui_etab($2) - 1)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee)
    .bind(eleve)
    .bind(classe)
    .execute(ent.pool())
    .await
    .unwrap();
    let parent = nouvel_utilisateur(&ent, etab, "parent", "ParentZ").await;
    lier(&ent, etab, eleve, parent, Some(nouveau_foyer(&ent, etab).await)).await;
    // L'annonce vise « Sixième Z » ; la classe est ensuite renommée.
    let a = ent
        .publier_annonce_for_actor(role(Role::Direction), annonce(etab, dir, "classe", Some("Sixième Z")))
        .await
        .unwrap();
    assert_eq!(a.classe_id, Some(classe), "D18 : classe non rattachée par identifiant");
    sqlx::query("UPDATE ent_classe SET code = '6Z-bis', libelle = 'Sixième Zèbre' WHERE id = $1")
        .bind(classe)
        .execute(ent.pool())
        .await
        .unwrap();
    // Témoin : la famille de la classe la voit avant et après le renommage ; une autre famille, non.
    let vue = ent.list_annonces_famille_for_actor(etab, parent, role(Role::Parent), eleve).await.unwrap();
    assert!(vue.iter().any(|x| x.id == a.id), "D18 : classe renommée, l'annonce n'atteint plus personne");
    let (envois,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND source_event_id = $2 AND destinataire_user_id = $3",
    )
    .bind(etab)
    .bind(a.id)
    .bind(parent)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(envois, 1);
    let autre = ent
        .list_annonces_famille_for_actor(etab, cast.parent_lea.user_id, role(Role::Parent), cast.eleve_lea_id)
        .await
        .unwrap();
    assert!(autre.iter().all(|x| x.id != a.id), "l'annonce d'une classe atteint une autre famille");
    // Un libellé inconnu ne fait pas échouer la publication (l'ancien ciblage par libellé
    // reste possible quand aucune classe du référentiel ne porte ce nom).
    ent.publier_annonce_for_actor(role(Role::Direction), annonce(etab, dir, "classe", Some("Classe inconnue")))
        .await
        .unwrap();
}

// ─── D20 : fusion de fiches ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m0b_d20_fusion_de_fiches_les_fils_suivent_l_eleve_conserve_sans_changer_de_lecteurs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    use crate::eleve_fusion::{DemandeFusion, ResultatDefusion, ResultatFusion};
    let etab = cast.etablissement_id;
    let (dir, cpe) = (cast.direction.user_id, cast.cpe.user_id);
    let mk = |nom: &str| {
        let ent = &ent;
        let nom = nom.to_string();
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at)
                 VALUES ($1,$2,$3,'Doublon','inscrit', now(), now())",
            )
            .bind(id)
            .bind(etab)
            .bind(nom)
            .execute(ent.pool())
            .await
            .unwrap();
            id
        }
    };
    let (a, b) = (mk("Alpha").await, mk("Alpha").await);
    let (pa, pb) = (
        nouvel_utilisateur(&ent, etab, "parent", "ParentA").await,
        nouvel_utilisateur(&ent, etab, "parent", "ParentB").await,
    );
    lier(&ent, etab, a, pa, Some(nouveau_foyer(&ent, etab).await)).await;
    lier(&ent, etab, b, pb, Some(nouveau_foyer(&ent, etab).await)).await;
    // Un fil privé de chaque parent, un fil du personnel à chaque foyer.
    let fil_pa = ouvrir_un(&ent, etab, Role::Parent, pa, a, "Question A").await;
    let fil_pb = ouvrir_un(&ent, etab, Role::Parent, pb, b, "Question B").await;
    let fil_dir_b = ouvrir_un(&ent, etab, Role::Direction, dir, b, "Annonce à B").await;
    let _ = fil_dir_b;
    // Témoins avant la fusion.
    assert!(lit(&ent, etab, pa, Role::Parent, fil_pa.id).await && lit(&ent, etab, pb, Role::Parent, fil_pb.id).await);
    assert!(!lit(&ent, etab, pa, Role::Parent, fil_pb.id).await);
    // Fusion : B est absorbé dans A.
    let demande = |conflits| DemandeFusion {
        etablissement_id: etab,
        absorbe_id: b,
        conserve_id: a,
        motif_code: "doublon_saisie".into(),
        motif: None,
        conflits_acquittes: conflits,
        acteur: dir,
    };
    let mut res = ent
        .fusionner_eleves_pour_acteur(role(Role::Direction), demande(Vec::new()))
        .await
        .unwrap();
    if let ResultatFusion::ConflitsAResoudre(conflits) = res {
        res = ent
            .fusionner_eleves_pour_acteur(role(Role::Direction), demande(conflits))
            .await
            .unwrap();
    }
    let ResultatFusion::Faite(fusion) = res else { panic!("fusion non faite : {res:?}") };
    // Les fils sont tous portés par l'élève conservé.
    let sur_a: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_msg_fil WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(a)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(sur_a, 3, "D20 : fils non rattachés à l'élève conservé");
    // Chaque parent lit toujours son fil, et seulement le sien : la fusion n'ouvre rien.
    assert!(lit(&ent, etab, pb, Role::Parent, fil_pb.id).await, "D20 : le parent absorbé perd son fil");
    assert!(lit(&ent, etab, pa, Role::Parent, fil_pa.id).await);
    assert!(!lit(&ent, etab, pa, Role::Parent, fil_pb.id).await, "D20 : un parent lit le fil privé de l'autre après fusion");
    assert!(!lit(&ent, etab, pb, Role::Parent, fil_pa.id).await, "D20 : fil privé de A lu par B après fusion");
    // La CPE (rôle réservé) lit les trois fils sous l'élève conservé.
    assert!(lit(&ent, etab, cpe, Role::Cpe, fil_pb.id).await);
    // Défusion : tout revient à l'identique.
    match ent
        .defusionner_eleves_pour_acteur(role(Role::Direction), etab, dir, fusion.id)
        .await
        .unwrap()
    {
        ResultatDefusion::Faite(_) => {}
        autre => panic!("défusion refusée : {autre:?}"),
    }
    let sur_b: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_msg_fil WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(b)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(sur_b, 2, "défusion : fils non rendus");
}

// ─── Pièces : un message différé ne dévoile pas ses pièces ──────────────────────────────────

#[tokio::test]
async fn m0b_d24_piece_d_un_message_differe_introuvable_pour_la_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily, harry) = (cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Pièce").await;
    let msg = messages(&ent, etab, dir, Role::Direction, fil.id).await[0].message.id;
    let piece = ent
        .attacher_piece_for_actor(
            role(Role::Direction),
            domain_eleve::AttacherPiece {
                etablissement_id: etab,
                message_id: msg,
                nom: "note.txt".into(),
                mime_type: "text/plain".into(),
                contenu: b"Bonjour".to_vec(),
                actor_user_id: dir,
            },
        )
        .await
        .unwrap();
    // Témoin : remis, la famille télécharge.
    let p: MsgPiece = ent
        .telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id)
        .await
        .expect("témoin : pièce d'un message remis");
    assert_eq!(p.contenu, b"Bonjour");
    // Le message devient différé (écrit en plage calme) : la pièce disparaît pour la famille.
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '5 hours' WHERE id = $1")
        .bind(msg)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(
        ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.is_err(),
        "D24 : pièce d'un message différé lisible de la famille"
    );
    assert!(ent.telecharger_piece_for_actor(etab, dir, role(Role::Direction), piece.id).await.is_ok(), "l'auteur la garde");
}

async fn attacher(
    ent: &PgEnt,
    etab: Uuid,
    message: Uuid,
    auteur: Uuid,
    contenu: Vec<u8>,
) -> Result<domain_eleve::MsgPieceMeta, EntDbError> {
    ent.attacher_piece_for_actor(
        role(Role::Direction),
        domain_eleve::AttacherPiece {
            etablissement_id: etab,
            message_id: message,
            nom: "certificat.pdf".to_string(),
            mime_type: "application/pdf".into(),
            contenu,
            actor_user_id: auteur,
        },
    )
    .await
}
