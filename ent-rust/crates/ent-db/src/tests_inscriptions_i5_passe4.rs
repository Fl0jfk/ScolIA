//! Socle 3, lot I5 — **passe 4** (nouveau modèle des verrous), contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Aucun test ne dépend de l'heure réelle ni du jour UTC : les instants passés au service sont posés (2027).
//!
//! **Méthode des tests de concurrence (déterministe, aucune course)** :
//! 1. une connexion **externe** prend le verrou consultatif de l'offre (`pg_advisory_lock`, même clé que le code) ;
//! 2. l'opération testée est lancée dans une tâche ; on vérifie qu'elle **attend** (`pg_locks` : verrou consultatif non
//!    accordé, `pg_stat_activity.wait_event_type = 'Lock'`) ;
//! 3. on vérifie qu'elle ne tient **aucun verrou de dossier** (un `FOR UPDATE NOWAIT` externe sur chaque dossier réussit) :
//!    l'offre se prend en premier, les dossiers ensuite ;
//! 4. on relâche le verrou : l'opération **aboutit**, sans interblocage (`40P01`) ni erreur technique.
//!
//! | Sujet | Test(s) |
//! |---|---|
//! | un seul point de verrou (garde statique) | `p4_garde_statique_un_seul_point_de_verrou` |
//! | clé du verrou, rang calculé en lecture | `p4_cle_de_verrou_et_rang_calcule` |
//! | désistement du rang 3 pendant une décision sur le rang 20 (25 dossiers) | `p4_desistement_rang_3_pendant_decision_rang_20` |
//! | déplacements croisés A→B / B→A, campagnes indépendantes | `p4_deplacements_croises_et_campagnes_independantes` |
//! | publication de séance concurrente d'une décision | `p4_publication_de_seance_concurrente_d_une_decision` |
//! | modification de capacité concurrente d'une décision | `p4_capacite_concurrente_d_une_decision` |
//! | deux réutilisations croisées | `p4_reutilisations_croisees` |
//! | F3 contrôle d'écart avant les verrous | `p4_controle_d_ecart_avant_les_verrous` |
//! | F5 interblocage ≠ conflit métier | `p4_interblocage_n_est_pas_un_conflit` |
//! | M3, F6 export, chaque table, condition exécutée | `p4_export_chaque_table_condition_executee` |
//! | M4 export des conditions : bout de chaîne | `p4_export_des_conditions_bout_de_chaine` |
//! | M5 garde à `search_path` fixé, table temporaire remplie | `p4_garde_search_path_table_temporaire_remplie` |
//! | M6 caducité : famille d'états compatibles, alerte direction | `p4_caducite_famille_d_etats_et_alerte_direction` |
//! | F1 `dossier_dto` sur une seule connexion | `p4_dossier_dto_une_seule_connexion` |

use std::sync::Arc;
use std::time::Duration;

use ent_types::inscriptions::*;
use ent_types::inscriptions_decisions::*;
use uuid::Uuid;

use super::tests_inscriptions::{dir, labo};
use super::tests_inscriptions_i3_i4::{demarche_deux_enfants, regle, PiecesTest};
use super::tests_inscriptions_i5::{campagne_cap, corps, decider, dossier, email, maintenant, Dos};
use super::tests_inscriptions_i5_revue::{nombre, sqlstate};
use super::*;

fn jours(n: i64) -> chrono::Duration {
    chrono::Duration::days(n)
}

// ───────────────────────────── Infrastructure : verrou externe, attente observée ─────────────────────────────

/// Clé du verrou consultatif d'une offre, **recalculée ici** de façon indépendante (même formule que la fonction SQL
/// `ent_inscription_offre_cle` ; `p4_cle_de_verrou_et_rang_calcule` vérifie qu'elles coïncident). Ne dépend d'aucune
/// fonction de la migration 315 : le même test se rejoue sur le code d'avant.
async fn cle_offre(ent: &PgEnt, offre: Uuid) -> i64 {
    sqlx::query_scalar("SELECT hashtextextended('ent_inscription_offre:' || $1::text, 0)")
        .bind(offre)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// Verrous consultatifs d'offres tenus par une connexion **externe** (détachée du pool : elle se ferme à la chute).
struct VerrouExterne {
    conn: sqlx::PgConnection,
    cles: Vec<i64>,
}

async fn tenir(ent: &PgEnt, offres: &[Uuid]) -> VerrouExterne {
    let mut conn = ent.pool_verif().acquire().await.unwrap().detach();
    let mut cles = Vec::new();
    for o in offres {
        let cle = cle_offre(ent, *o).await;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(cle)
            .execute(&mut conn)
            .await
            .unwrap();
        cles.push(cle);
    }
    VerrouExterne { conn, cles }
}

impl VerrouExterne {
    async fn relacher(mut self) {
        for cle in self.cles.clone() {
            sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(cle)
                .execute(&mut self.conn)
                .await
                .unwrap();
        }
    }
}

/// Nombre de connexions **en attente** (`wait_event_type = 'Lock'`) du verrou consultatif de clé `cle`.
async fn en_attente(ent: &PgEnt, cle: i64) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_locks l JOIN pg_stat_activity a ON a.pid = l.pid
          WHERE l.locktype = 'advisory' AND NOT l.granted AND l.objsubid = 1
            AND a.wait_event_type = 'Lock'
            AND ((l.classid::bigint << 32) | l.objid::bigint) = $1",
    )
    .bind(cle)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

/// Attend (au plus 10 s) que `n` connexions au moins attendent le verrou de cette offre ; sinon le test échoue : l'opération
/// n'a **pas** attendu le verrou de l'offre.
#[track_caller]
fn attendre_attentes<'a>(
    ent: &'a PgEnt,
    offre: Uuid,
    n: i64,
    quoi: &'static str,
) -> impl std::future::Future<Output = ()> + 'a {
    let appelant = std::panic::Location::caller();
    async move {
        let cle = cle_offre(ent, offre).await;
        for _ in 0..200 {
            if en_attente(ent, cle).await >= n {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!(
            "{appelant} : {quoi} : {n} opération(s) auraient dû attendre le verrou de l'offre (pg_locks, wait_event_type = Lock), \
             {} seulement l'ont fait",
            en_attente(ent, cle).await
        );
    }
}

/// Le dossier n'est verrouillé par personne (un `FOR UPDATE NOWAIT` externe réussit) : l'opération qui attend l'offre n'a
/// pas encore pris de dossier.
async fn dossier_libre(ent: &PgEnt, dossier: Uuid) -> bool {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let r = sqlx::query("SELECT id FROM ent_inscription_dossier WHERE id = $1 FOR UPDATE NOWAIT")
        .bind(dossier)
        .execute(&mut *tx)
        .await;
    tx.rollback().await.unwrap();
    r.is_ok()
}

async fn tous_libres(ent: &PgEnt, ds: &[&Dos], quoi: &str) {
    for d in ds {
        assert!(
            dossier_libre(ent, d.id).await,
            "{quoi} : une opération qui attend l'offre tient déjà un verrou de dossier (l'offre doit se prendre en premier)"
        );
    }
}

/// Une erreur **technique** (interblocage, panne, verrou refusé) : jamais acceptée. Seul un conflit métier l'est.
fn est_technique(e: &EntDbError) -> bool {
    !matches!(
        e,
        EntDbError::ConflitDossier(_) | EntDbError::SaisieInvalide(_) | EntDbError::Domain(DomainError::NotFound)
    )
}

/// Attend l'issue d'une tâche (au plus 30 s).
async fn issue<T>(h: tokio::task::JoinHandle<T>, quoi: &str) -> T {
    tokio::time::timeout(Duration::from_secs(30), h)
        .await
        .unwrap_or_else(|_| panic!("{quoi} : l'opération ne s'est pas terminée après le relâchement du verrou"))
        .unwrap()
}

async fn positions(ent: &PgEnt, c: &LaboCast, offre: Uuid) -> Vec<(Uuid, i64)> {
    let (u, r) = dir(c);
    ent.liste_attente_for_actor(c.etablissement_id, u, r, offre)
        .await
        .unwrap()
        .lignes
        .into_iter()
        .map(|l| (l.dossier_id, l.position))
        .collect()
}

// ───────────────────────────── Garde statique : un seul point de verrou ─────────────────────────────

const SOURCES_INSCRIPTION: [(&str, &str); 8] = [
    ("inscriptions.rs", include_str!("inscriptions.rs")),
    ("inscriptions_accords.rs", include_str!("inscriptions_accords.rs")),
    ("inscriptions_attente.rs", include_str!("inscriptions_attente.rs")),
    ("inscriptions_decisions.rs", include_str!("inscriptions_decisions.rs")),
    ("inscriptions_etats.rs", include_str!("inscriptions_etats.rs")),
    ("inscriptions_modele.rs", include_str!("inscriptions_modele.rs")),
    ("inscriptions_pieces.rs", include_str!("inscriptions_pieces.rs")),
    ("inscriptions_reactions.rs", include_str!("inscriptions_reactions.rs")),
];

/// Le mot `table` (borné : `ent_inscription_offre` n'est pas `ent_inscription_offre_historique`) figure dans `texte`.
fn contient_table(texte: &str, table: &str) -> bool {
    let mut debut = 0;
    while let Some(i) = texte[debut..].find(table) {
        let a = debut + i;
        let b = a + table.len();
        let avant = texte[..a].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let apres = texte[b..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if avant && apres {
            return true;
        }
        debut = b;
    }
    false
}

/// Texte du littéral de chaîne qui contient l'octet `pos` (du `"` précédent au `"` suivant, guillemets échappés ignorés).
fn litteral_autour(src: &str, pos: usize) -> &str {
    let oct = src.as_bytes();
    let mut a = pos;
    while a > 0 && !(oct[a - 1] == b'"' && (a < 2 || oct[a - 2] != b'\\')) {
        a -= 1;
    }
    let mut b = pos;
    while b < oct.len() && !(oct[b] == b'"' && oct[b - 1] != b'\\') {
        b += 1;
    }
    &src[a..b]
}

fn est_commentaire(src: &str, pos: usize) -> bool {
    let debut = src[..pos].rfind('\n').map_or(0, |i| i + 1);
    src[debut..].trim_start().starts_with("//")
}

/// **Garde statique** (passe 4) : dans les fichiers d'inscription, aucun `FOR UPDATE` / `FOR NO KEY UPDATE` sur une offre ou un
/// dossier en dehors de `verrouiller_dans_l_ordre_tx` ; aucun verrou consultatif ailleurs ; aucun `NOWAIT` ; plus de
/// compactage ; aucune écriture de `rang_attente`.
#[test]
fn p4_garde_statique_un_seul_point_de_verrou() {
    let mut fautes: Vec<String> = Vec::new();
    for (nom, src) in SOURCES_INSCRIPTION {
        // Étendue de la fonction unique (dans `inscriptions_attente.rs` seulement).
        let fonction = (nom == "inscriptions_attente.rs").then(|| {
            let a = src
                .find("pub(crate) async fn verrouiller_dans_l_ordre_tx")
                .expect("fonction unique présente");
            let b = a + src[a..].find("\n}\n").expect("fin de la fonction unique");
            a..b
        });
        let dans_fonction = |pos: usize| fonction.as_ref().is_some_and(|f| f.contains(&pos));
        for motif in ["FOR UPDATE", "FOR NO KEY UPDATE", "FOR SHARE", "FOR KEY SHARE"] {
            let mut debut = 0;
            while let Some(i) = src[debut..].find(motif) {
                let pos = debut + i;
                debut = pos + motif.len();
                // `FOR UPDATE` est un préfixe de rien d'autre ici ; `FOR NO KEY UPDATE` ne contient pas `FOR UPDATE`.
                if est_commentaire(src, pos) || dans_fonction(pos) {
                    continue;
                }
                let lit = litteral_autour(src, pos);
                for table in ["ent_inscription_offre", "ent_inscription_dossier"] {
                    if contient_table(lit, table) && !lit.contains(" OF ") {
                        fautes.push(format!("{nom} : `{motif}` sur {table} hors de verrouiller_dans_l_ordre_tx"));
                    }
                }
            }
        }
        for (interdit, pourquoi) in [
            ("pg_advisory", "verrou consultatif hors de verrouiller_dans_l_ordre_tx"),
            ("NOWAIT", "NOWAIT sans réessai (l'ordre étant unique, on attend)"),
            ("SKIP LOCKED", "SKIP LOCKED sur la liste (un autre dossier désigné à la place)"),
            ("compacter_rangs", "compactage physique du rang (le rang se calcule en lecture)"),
        ] {
            let mut debut = 0;
            while let Some(i) = src[debut..].find(interdit) {
                let pos = debut + i;
                debut = pos + interdit.len();
                if est_commentaire(src, pos) || dans_fonction(pos) {
                    continue;
                }
                // `ent_inscription_reactions` : file d'envoi transverse, `SKIP LOCKED` sur ses propres lignes seulement.
                if interdit == "SKIP LOCKED" && litteral_autour(src, pos).contains("ent_inscription_envoi") {
                    continue;
                }
                fautes.push(format!("{nom} : {pourquoi}"));
            }
        }
        let mut debut = 0;
        while let Some(i) = src[debut..].find("rang_attente") {
            let pos = debut + i;
            debut = pos + 12;
            if est_commentaire(src, pos) {
                continue;
            }
            let lit = litteral_autour(src, pos);
            let ecrit = lit.contains("SET rang_attente") || lit.contains("rang_attente = $") || lit.contains("rang_attente=");
            if ecrit {
                fautes.push(format!("{nom} : écriture de rang_attente (colonne en lecture seule depuis 315)"));
            }
        }
    }
    assert!(fautes.is_empty(), "garde statique des verrous :\n{}", fautes.join("\n"));
}

// ───────────────────────────── Clé du verrou, rang calculé ─────────────────────────────

/// La fonction SQL `ent_inscription_offre_cle` (source unique du code) coïncide avec la formule des tests ; le rang est
/// **calculé** : un départ ne décale rien dans la table, la colonne `rang_attente` est en lecture seule.
#[tokio::test]
async fn p4_cle_de_verrou_et_rang_calcule() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let attendu = cle_offre(&ent, offre).await;
    let fonction: i64 = sqlx::query_scalar("SELECT ent_inscription_offre_cle($1)")
        .bind(offre)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(fonction, attendu, "une seule clé, pour le code comme pour les tests");

    let x = dossier(&ent, &c, camp, "OccupantX", None).await;
    decider(&ent, &c, &x, &corps("admis", Some(offre), "Admis")).await.unwrap();
    let mut file = Vec::new();
    for i in 0..5 {
        let d = dossier(&ent, &c, camp, &format!("File{i}"), None).await;
        decider(&ent, &c, &d, &corps("liste_attente", Some(offre), "Complet"))
            .await
            .unwrap();
        file.push(d);
    }
    let avant = positions(&ent, &c, offre).await;
    assert_eq!(
        avant.iter().map(|p| p.1).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5],
        "rangs calculés par la fonction unique"
    );
    // La colonne stockée n'est jamais écrite : aucun rang en base pour ces dossiers.
    for d in &file {
        let stocke: Option<i32> = sqlx::query_scalar("SELECT rang_attente FROM ent_inscription_dossier WHERE id = $1")
            .bind(d.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        assert_eq!(stocke, None, "rang_attente n'est plus écrit");
    }
    // Un départ au milieu : les suivants avancent **en lecture**, sans qu'aucune ligne ne soit réécrite.
    let versions_avant: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT id, updated_at FROM ent_inscription_dossier WHERE id = ANY($1) ORDER BY id",
    )
    .bind(file.iter().map(|d| d.id).collect::<Vec<_>>())
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    ent.retirer_candidature_famille(etab, &file[1].jeton, file[1].id).await.unwrap();
    let apres = positions(&ent, &c, offre).await;
    assert_eq!(
        apres,
        vec![(file[0].id, 1), (file[2].id, 2), (file[3].id, 3), (file[4].id, 4)],
        "le départ du rang 2 ne laisse pas de trou et ne change pas l'ordre"
    );
    let versions_apres: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT id, updated_at FROM ent_inscription_dossier WHERE id = ANY($1) ORDER BY id",
    )
    .bind(file.iter().map(|d| d.id).collect::<Vec<_>>())
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    for (id, avant_maj) in versions_avant {
        if id == file[1].id {
            continue;
        }
        let apres_maj = versions_apres.iter().find(|v| v.0 == id).unwrap().1;
        assert_eq!(apres_maj, avant_maj, "aucun dossier derrière le partant n'est réécrit (plus de compactage)");
    }
    // Le rang de chaque dossier passe par la même fonction : personnel et famille.
    let (d3, d2) = (&file[3], &file[2]);
    let rang_perso: Option<i64> = sqlx::query_scalar("SELECT ent_inscription_rang_attente($1, $2, false)")
        .bind(etab)
        .bind(d3.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(rang_perso, Some(3));
    let dto = ent.dossier_for_actor(etab, u, r, d2.id).await.unwrap();
    assert_eq!(dto.rang_attente, Some(2), "le dossier du personnel porte le rang calculé");
    // La colonne reste en lecture seule (reprise) : toute modification est refusée.
    let ecriture = sqlx::query("UPDATE ent_inscription_dossier SET rang_attente = 99 WHERE id = $1")
        .bind(d3.id)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(sqlstate(&ecriture), "23514", "rang_attente : lecture seule : {ecriture:?}");
}

// ───────────────────────────── Concurrence déterministe ─────────────────────────────

/// Désistement du rang 3 pendant une décision sur le rang 20, sur une liste de 25 dossiers. Avec l'ancien modèle, le départ
/// compactait **tous** les dossiers derrière lui (dont le rang 20, tenu par la décision qui attendait l'offre) : interblocage.
/// Ici : les deux opérations attendent l'offre **sans tenir aucun dossier**, puis aboutissent.
#[tokio::test]
async fn p4_desistement_rang_3_pendant_decision_rang_20() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let x = dossier(&ent, &c, camp, "Occupant", None).await;
    decider(&ent, &c, &x, &corps("admis", Some(offre), "Admis")).await.unwrap();
    let mut file: Vec<Dos> = Vec::new();
    for i in 0..25 {
        let d = dossier(&ent, &c, camp, &format!("Rang{}", i + 1), None).await;
        decider(&ent, &c, &d, &corps("liste_attente", Some(offre), "Complet"))
            .await
            .unwrap();
        file.push(d);
    }
    assert_eq!(positions(&ent, &c, offre).await.len(), 25, "liste de 25 dossiers");

    let verrou = tenir(&ent, &[offre]).await;
    let (rang3, rang20) = (&file[2], &file[19]);
    let desistement = {
        let (e, jeton, id) = (ent.clone(), rang3.jeton.clone(), rang3.id);
        tokio::spawn(async move { e.retirer_candidature_famille(etab, &jeton, id).await.map(|_| ()) })
    };
    let decision = {
        let (e, id) = (ent.clone(), rang20.id);
        let b = corps("refuse", Some(offre), "Refus du rang 20");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &b, false, maintenant()).await.map(|_| ())
        })
    };
    attendre_attentes(&ent, offre, 2, "désistement + décision").await;
    // Ni l'une ni l'autre ne tient un dossier pendant qu'elle attend l'offre : le rang 3, le rang 20 et le premier.
    tous_libres(&ent, &[rang3, rang20, &file[0], &file[24]], "désistement + décision").await;
    assert!(!desistement.is_finished() && !decision.is_finished(), "elles attendent");
    verrou.relacher().await;
    let (a, b) = (issue(desistement, "désistement").await, issue(decision, "décision").await);
    assert!(a.is_ok(), "désistement du rang 3 : {a:?}");
    assert!(b.is_ok(), "décision sur le rang 20 : {b:?}");

    // 25 − 2 = 23 dossiers, rangs 1..=23 sans trou, ordre d'entrée conservé.
    let apres = positions(&ent, &c, offre).await;
    assert_eq!(apres.len(), 23);
    assert_eq!(apres.iter().map(|p| p.1).collect::<Vec<_>>(), (1..=23).collect::<Vec<i64>>());
    let attendu: Vec<Uuid> = file
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 2 && *i != 19)
        .map(|(_, d)| d.id)
        .collect();
    assert_eq!(apres.iter().map(|p| p.0).collect::<Vec<_>>(), attendu);
}

/// Deux déplacements croisés (A passe de l'offre 1 à l'offre 2 pendant que B passe de 2 à 1) : même ordre des offres. Plus
/// aucun verrou de campagne ne les sérialise (une offre voulue appartient à la campagne du dossier : le croisement a lieu
/// dans une campagne) ; deux **campagnes** indépendantes ne s'attendent pas.
#[tokio::test]
async fn p4_deplacements_croises_et_campagnes_independantes() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre1) = campagne_cap(&ent, &c, Some(2), "2027-2028").await;
    let offre2 = ent
        .creer_offre_for_actor(
            etab,
            u,
            r,
            camp,
            &OffreBody {
                niveau_code: "5EME".into(),
                parcours_code: Some("latin".into()),
                libelle_public: "5ᵉ latin".into(),
                capacite: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id;
    let a = dossier(&ent, &c, camp, "DeplaceA", None).await;
    let b = dossier(&ent, &c, camp, "DeplaceB", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre1), "Admis")).await.unwrap();
    decider(&ent, &c, &b, &corps("admis", Some(offre2), "Admis")).await.unwrap();
    // Une autre campagne, indépendante, avec son offre.
    let (camp_bis, offre_bis) = campagne_cap(&ent, &c, Some(2), "2028-2029").await;
    let autre = dossier(&ent, &c, camp_bis, "AutreCampagne", None).await;

    let (petite, grande) = if offre1 < offre2 { (offre1, offre2) } else { (offre2, offre1) };
    // L'extérieur tient les **deux** offres : la campagne bis, elle, n'attend rien.
    let verrou = tenir(&ent, &[petite, grande]).await;
    let autre_decision = {
        let (e, id) = (ent.clone(), autre.id);
        let b = corps("admis", Some(offre_bis), "Admis ailleurs");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &b, false, maintenant()).await.map(|_| ())
        })
    };
    let ailleurs = tokio::time::timeout(Duration::from_secs(20), autre_decision)
        .await
        .expect("une autre campagne n'attend pas les offres tenues");
    assert!(ailleurs.unwrap().is_ok(), "campagne indépendante");

    let (vers_a, vers_b) = (offre2, offre1);
    let op_a = {
        let (e, id) = (ent.clone(), a.id);
        let bd = corps("admis", Some(vers_a), "A change d'offre");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &bd, true, maintenant()).await.map(|_| ())
        })
    };
    let op_b = {
        let (e, id) = (ent.clone(), b.id);
        let bd = corps("admis", Some(vers_b), "B change d'offre");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &bd, true, maintenant()).await.map(|_| ())
        })
    };
    // Même ordre : les deux demandent d'abord l'offre de plus petit UUID (une seule l'obtient quand l'extérieur la lâche, mais
    // ici l'extérieur tient les deux : les deux attendent la plus petite, aucune ne passe à la grande).
    attendre_attentes(&ent, petite, 2, "A→B et B→A attendent d'abord la plus petite offre").await;
    assert_eq!(
        en_attente(&ent, cle_offre(&ent, grande).await).await,
        0,
        "aucune ne tient la plus petite offre en attendant la grande : même ordre"
    );
    tous_libres(&ent, &[&a, &b], "déplacements croisés").await;
    verrou.relacher().await;
    let (ra, rb) = (issue(op_a, "A→B").await, issue(op_b, "B→A").await);
    assert!(ra.is_ok(), "A : {ra:?}");
    assert!(rb.is_ok(), "B : {rb:?}");
    let offre_de = |d: Uuid| {
        let ent = ent.clone();
        async move {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT v.offre_id FROM ent_inscription_dossier d JOIN ent_inscription_voeu v ON v.id = d.voeu_retenu_id WHERE d.id = $1",
            )
            .bind(d)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap()
        }
    };
    assert_eq!(offre_de(a.id).await, offre2);
    assert_eq!(offre_de(b.id).await, offre1);
}

/// Publication d'une séance de commission concurrente d'une décision sur un de ses dossiers : toutes deux attendent l'offre
/// sans tenir de dossier, puis aboutissent (la publication résout d'abord offres et dossiers, verrouille une seule fois).
#[tokio::test]
async fn p4_publication_de_seance_concurrente_d_une_decision() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(5), "2027-2028").await;
    let ds = [
        dossier(&ent, &c, camp, "SeanceUn", None).await,
        dossier(&ent, &c, camp, "SeanceDeux", None).await,
        dossier(&ent, &c, camp, "SeanceTrois", None).await,
    ];
    let seance = ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                campagne_id: camp,
                intitule: "Séance concurrente".into(),
                seance_le: super::tests_inscriptions_i5::t("2027-03-20T09:00:00Z"),
                membres: vec![u],
                dossier_ids: ds.iter().map(|d| d.id).collect(),
            },
        )
        .await
        .unwrap();
    let recap = ent
        .decider_en_lot_for_actor(
            etab,
            u,
            r,
            seance.id,
            &DecisionsEnLotBody {
                decisions: ds
                    .iter()
                    .map(|d| DecisionEnLotItem {
                        dossier_id: d.id,
                        decision: corps("admis", Some(offre), "Admis en commission"),
                    })
                    .collect(),
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(recap.reussies, 3, "{:?}", recap.resultats);

    let verrou = tenir(&ent, &[offre]).await;
    let publication = {
        let e = ent.clone();
        let id = seance.id;
        tokio::spawn(async move {
            e.publier_commission_for_actor(etab, u, r, id, &PublierBody::default(), maintenant()).await
        })
    };
    let revue = {
        let (e, id) = (ent.clone(), ds[1].id);
        let b = corps("refuse", Some(offre), "Revue du deuxième dossier");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &b, true, maintenant()).await.map(|_| ())
        })
    };
    attendre_attentes(&ent, offre, 2, "publication de séance + revue").await;
    tous_libres(&ent, &[&ds[0], &ds[1], &ds[2]], "publication de séance + revue").await;
    verrou.relacher().await;
    let (p, rv) = (issue(publication, "publication").await, issue(revue, "revue").await);
    // Selon l'ordre d'obtention de l'offre, la publication voit la décision remplacée (refus métier) ou publie avant la revue :
    // les deux issues sont **métier** ; jamais un interblocage ni une erreur technique.
    if let Err(e) = &p {
        assert!(!est_technique(e), "publication : erreur technique {e:?}");
    }
    if let Err(e) = &rv {
        assert!(!est_technique(e), "revue : erreur technique {e:?}");
    }
    assert!(p.is_ok() || rv.is_ok(), "l'une des deux aboutit : {:?} / {:?}", p.as_ref().err(), rv.as_ref().err());
    // Cohérence : le dossier revu est refusé si la revue a abouti.
    if rv.is_ok() {
        assert_eq!(super::tests_inscriptions_i5::etat(&ent, etab, &ds[1]).await, "refuse");
    }
}

/// Modification de capacité concurrente d'une décision sur la même offre.
#[tokio::test]
async fn p4_capacite_concurrente_d_une_decision() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let x = dossier(&ent, &c, camp, "Occupant", None).await;
    decider(&ent, &c, &x, &corps("admis", Some(offre), "Admis")).await.unwrap();
    let y = dossier(&ent, &c, camp, "Candidat", None).await;
    let verrou = tenir(&ent, &[offre]).await;
    let capacite = {
        let e = ent.clone();
        tokio::spawn(async move {
            e.modifier_offre_for_actor(
                etab,
                u,
                r,
                offre,
                &OffreBody {
                    niveau_code: "6EME".into(),
                    libelle_public: "6ᵉ".into(),
                    capacite: Some(2),
                    motif: Some("Ouverture d'une place".into()),
                    ..Default::default()
                },
            )
            .await
            .map(|_| ())
        })
    };
    let decision = {
        let (e, id) = (ent.clone(), y.id);
        let b = corps("admis", Some(offre), "Admission sur la place ouverte");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &b, false, maintenant()).await.map(|_| ())
        })
    };
    attendre_attentes(&ent, offre, 2, "capacité + décision").await;
    tous_libres(&ent, &[&x, &y], "capacité + décision").await;
    verrou.relacher().await;
    let (cap, dec) = (issue(capacite, "capacité").await, issue(decision, "décision").await);
    assert!(cap.is_ok(), "capacité : {cap:?}");
    // Capacité d'abord : l'admission passe ; décision d'abord : l'offre est pleine, refus métier (dépassement non confirmé).
    match &dec {
        Ok(()) => assert_eq!(super::tests_inscriptions_i5::etat(&ent, etab, &y).await, "admis"),
        Err(e) => assert!(!est_technique(e), "décision : erreur technique {e:?}"),
    }
    let admis: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_inscription_dossier d JOIN ent_inscription_voeu v ON v.id = d.voeu_retenu_id
          WHERE v.offre_id = $1 AND d.etat IN ('admis', 'confirme')",
    )
    .bind(offre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(admis <= 2, "jamais plus de places prises que la capacité");
}

/// Deux réutilisations croisées de pièces de foyer : chacune attend l'offre sans tenir ses dossiers, puis aboutit.
#[tokio::test]
async fn p4_reutilisations_croisees() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (a, ar) = super::tests_inscriptions::adm(&c);
    let camp = super::tests_inscriptions_i3_i4::campagne_avec(
        &ent,
        &c,
        "un_seul",
        &[
            regle("justificatif_domicile", "Justificatif de domicile", true, "dossier", vec![]),
            regle("livret_famille", "Livret de famille", true, "dossier", vec![]),
        ],
    )
    .await;
    let offre: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_inscription_offre WHERE etablissement_id = $1 AND campagne_id = $2 AND archivee_le IS NULL",
    )
    .bind(etab)
    .bind(camp)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let jeton = super::tests_inscriptions::verifier(&ent, etab, &email("croise")).await;
    let dem = ent
        .deposer_demarche(etab, &jeton, &demarche_deux_enfants(camp, None))
        .await
        .unwrap();
    let (d1, d2) = (dem.dossiers[0].id, dem.dossiers[1].id);
    ent.deposer_reel(etab, &jeton, d1, "livret_famille").await;
    ent.deposer_reel(etab, &jeton, d2, "justificatif_domicile").await;
    // Les deux dossiers retiennent l'offre (liste d'attente : aucune place à compter, aucun accord à réunir).
    for d in [d1, d2] {
        ent.changer_statut_for_actor(
            etab,
            a,
            ar,
            d,
            &StatutBody {
                statut: "en_instruction".into(),
                motif: None,
            },
        )
        .await
        .unwrap();
        let (u, r) = dir(&c);
        ent.decider_complete_for_actor(
            etab,
            u,
            r,
            d,
            &corps("liste_attente", Some(offre), "Offre retenue"),
            false,
            maintenant(),
        )
        .await
        .unwrap();
    }
    let verrou = tenir(&ent, &[offre]).await;
    let op1 = {
        let (e, j) = (ent.clone(), jeton.clone());
        tokio::spawn(async move { e.reutiliser_piece_famille(etab, &j, d2, "livret_famille", d1).await.map(|_| ()) })
    };
    let op2 = {
        let (e, j) = (ent.clone(), jeton.clone());
        tokio::spawn(async move {
            e.reutiliser_piece_famille(etab, &j, d1, "justificatif_domicile", d2).await.map(|_| ())
        })
    };
    attendre_attentes(&ent, offre, 2, "deux réutilisations croisées").await;
    for d in [d1, d2] {
        assert!(dossier_libre(&ent, d).await, "une réutilisation qui attend l'offre ne tient pas encore ses dossiers");
    }
    verrou.relacher().await;
    let (r1, r2) = (issue(op1, "d2 reprend d1").await, issue(op2, "d1 reprend d2").await);
    assert!(r1.is_ok(), "d2 reprend le livret de d1 : {r1:?}");
    assert!(r2.is_ok(), "d1 reprend le justificatif de d2 : {r2:?}");
}

/// F3 : un acteur écarté (déporté) reçoit « introuvable » **sans avoir tenu ni attendu aucun verrou de lot** : l'offre est
/// tenue par l'extérieur, l'appel ne bloque pas.
#[tokio::test]
async fn p4_controle_d_ecart_avant_les_verrous() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Ecarte", None).await;
    decider(&ent, &c, &d, &corps("liste_attente", Some(offre), "Complet")).await.unwrap();
    ent.se_deporter_for_actor(etab, u, r, d.id, &DeportBody { motif: "Parent du candidat".into() })
        .await
        .unwrap();
    let verrou = tenir(&ent, &[offre]).await;
    let (e, id) = (ent.clone(), d.id);
    let b = corps("refuse", Some(offre), "Refus d'un écarté");
    let appel = tokio::spawn(async move {
        e.decider_complete_for_actor(etab, u, r, id, &b, true, maintenant()).await.map(|_| ())
    });
    let issue_rapide = tokio::time::timeout(Duration::from_secs(10), appel)
        .await
        .expect("un acteur écarté n'attend aucun verrou de lot");
    let resultat = issue_rapide.unwrap();
    assert!(
        matches!(&resultat, Err(EntDbError::Domain(DomainError::NotFound))),
        "refus exact : introuvable, obtenu {resultat:?}"
    );
    assert_eq!(en_attente(&ent, cle_offre(&ent, offre).await).await, 0);
    verrou.relacher().await;
}

/// F5 : un interblocage (`40P01`) n'est **jamais** confondu avec un conflit métier. On en provoque un vrai (deux connexions,
/// verrous consultatifs croisés) et on passe son erreur à la classification des inscriptions.
#[tokio::test]
async fn p4_interblocage_n_est_pas_un_conflit() {
    let Some((ent, _c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (k1, k2) = (Uuid::new_v4().as_u128() as i64, Uuid::new_v4().as_u128() as i64);
    let mut c1 = ent.pool_verif().acquire().await.unwrap().detach();
    let mut c2 = ent.pool_verif().acquire().await.unwrap().detach();
    sqlx::query("BEGIN").execute(&mut c1).await.unwrap();
    sqlx::query("BEGIN").execute(&mut c2).await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(k1).execute(&mut c1).await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(k2).execute(&mut c2).await.unwrap();
    let h1 = tokio::spawn(async move {
        let r = sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(k2).execute(&mut c1).await;
        (r, c1)
    });
    // Laisser la première attendre avant de fermer le cercle.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let r2 = sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(k1).execute(&mut c2).await;
    let (r1, _c1) = tokio::time::timeout(Duration::from_secs(20), h1).await.unwrap().unwrap();
    let erreurs: Vec<sqlx::Error> = [r1, r2].into_iter().filter_map(Result::err).collect();
    assert_eq!(erreurs.len(), 1, "un interblocage : exactement une victime");
    let victime = erreurs.into_iter().next().unwrap();
    assert!(
        matches!(&victime, sqlx::Error::Database(d) if d.code().as_deref() == Some("40P01")),
        "interblocage détecté par la base : {victime:?}"
    );
    let classe = crate::inscriptions::db(victime);
    assert!(
        !matches!(classe, EntDbError::ConflitDossier(_)),
        "un interblocage n'est pas un conflit métier : {classe:?}"
    );
    assert!(
        matches!(classe, EntDbError::Domain(DomainError::Reessayer)),
        "erreur technique distincte (réessayable) : {classe:?}"
    );
}

// ───────────────────────────── Export : chaque table, condition exécutée ─────────────────────────────

fn classe(nom: &str) -> crate::registre_donnees::ClasseTable {
    *crate::registre_donnees::REGISTRE.iter().find(|t| t.nom == nom).unwrap()
}

async fn exportables(ent: &PgEnt, nom: &str, dossier: Uuid) -> i64 {
    let cond = classe(nom).export_si.unwrap_or("true");
    nombre(
        ent,
        &format!("SELECT count(*) FROM {nom} x WHERE x.dossier_id = $1 AND ({cond})"),
        dossier,
    )
    .await
}

async fn existantes(ent: &PgEnt, nom: &str, dossier: Uuid) -> i64 {
    nombre(ent, &format!("SELECT count(*) FROM {nom} WHERE dossier_id = $1"), dossier).await
}

/// M3, F6 : **chaque** table d'inscription qui porte une règle de ligne voit sa vraie condition exécutée, avant puis après
/// publication ; le vœu créé par la décision ne sort pas avant ; `place_no`, `origine` et `designee_le` sont hors export.
#[tokio::test]
async fn p4_export_chaque_table_condition_executee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    ent.definir_conditions_campagne_for_actor(
        etab,
        u,
        r,
        camp,
        &ConditionsCampagneBody {
            codes: vec!["reponse_famille".into(), "contrat_signe".into()],
        },
    )
    .await
    .unwrap();
    let d = dossier(&ent, &c, camp, "ExportM3", None).await;
    let mut b = corps("admis_sous_reserve", Some(offre), "Admission avec réserve interne");
    b.reserve = Some("Réserve confidentielle".into());
    b.publier_le = Some(maintenant() + jours(7));
    ent.decider_complete_for_actor(etab, u, r, d.id, &b, false, maintenant()).await.unwrap();

    // Le vœu créé par la décision porte son origine.
    let origine: String = sqlx::query_scalar("SELECT origine FROM ent_inscription_voeu WHERE dossier_id = $1")
        .bind(d.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(origine, "decision");

    // **Toutes** les tables d'inscription qui déclarent une règle de ligne et portent `dossier_id` sont exécutées : une table
    // nouvelle sans cas ici fait échouer le test.
    let avec_regle: Vec<&str> = crate::registre_donnees::REGISTRE
        .iter()
        .filter(|t| t.domaine == "inscriptions" && t.export_portabilite && t.export_si.is_some())
        .map(|t| t.nom)
        .collect();
    let couvertes = [
        "ent_inscription_condition",
        "ent_inscription_decision",
        "ent_inscription_proposition_place",
        "ent_inscription_suivi_dossier",
        "ent_inscription_voeu",
    ];
    for nom in &avec_regle {
        assert!(couvertes.contains(nom), "{nom} : table à règle de ligne sans cas de test d'export");
    }
    for nom in couvertes {
        assert!(classe(nom).export_si.is_some(), "{nom} : règle de ligne déclarée");
    }
    // Avant publication : des lignes existent, aucune n'est exportable (suivi : aucune ligne rattachée à la décision).
    for nom in ["ent_inscription_condition", "ent_inscription_decision", "ent_inscription_proposition_place", "ent_inscription_voeu"] {
        assert!(existantes(&ent, nom, d.id).await > 0, "{nom} : des lignes existent");
        assert_eq!(exportables(&ent, nom, d.id).await, 0, "{nom} : rien d'exportable avant la publication");
    }
    let suivi_rattache: i64 = nombre(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_inscription_suivi_dossier x WHERE x.dossier_id = $1
                AND ({}) AND x.decision_id IS NOT NULL",
            classe("ent_inscription_suivi_dossier").export_si.unwrap()
        ),
        d.id,
    )
    .await;
    assert_eq!(suivi_rattache, 0, "aucune ligne de suivi rattachée à la décision non publiée n'est exportable");
    // Publication à l'échéance : les lignes deviennent exportables.
    assert!(ent.publier_decisions_echues_tache(maintenant() + jours(8)).await.unwrap() >= 1);
    for nom in ["ent_inscription_condition", "ent_inscription_decision", "ent_inscription_proposition_place", "ent_inscription_voeu"] {
        assert!(exportables(&ent, nom, d.id).await > 0, "{nom} : exportable une fois la décision publiée");
    }
    // F6 : colonnes hors export de la proposition.
    for col in ["place_no", "origine", "designee_le", "motif", "envoyee_par", "rang_au_moment"] {
        assert!(classe("ent_inscription_proposition_place").colonnes_hors_export.contains(&col), "proposition.{col}");
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns
                             WHERE table_schema = 'public' AND table_name = 'ent_inscription_proposition_place' AND column_name = $1)",
        )
        .bind(col)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert!(existe, "proposition.{col} : colonne réelle");
    }
}

/// M4 : seul le **bout de chaîne** compte pour les conditions : une décision programmée puis **remplacée** (ni publiée ni
/// caduque) ne bloque plus l'export des conditions du dossier.
#[tokio::test]
async fn p4_export_des_conditions_bout_de_chaine() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    ent.definir_conditions_campagne_for_actor(
        etab,
        u,
        r,
        camp,
        &ConditionsCampagneBody { codes: vec!["contrat_signe".into()] },
    )
    .await
    .unwrap();
    let d = dossier(&ent, &c, camp, "BoutDeChaine", None).await;
    // 1. Admission publiée tout de suite.
    ent.decider_complete_for_actor(etab, u, r, d.id, &corps("admis", Some(offre), "Admis"), false, maintenant())
        .await
        .unwrap();
    assert!(exportables(&ent, "ent_inscription_condition", d.id).await > 0, "admission publiée : conditions exportables");
    // 2. Revue programmée (non publiée) : elle bloque l'export tant qu'elle est le bout de la chaîne.
    let mut b2 = corps("admis", Some(offre), "Revue programmée");
    b2.publier_le = Some(maintenant() + jours(7));
    b2.reserve = None;
    ent.decider_complete_for_actor(etab, u, r, d.id, &b2, true, maintenant()).await.unwrap();
    assert_eq!(
        exportables(&ent, "ent_inscription_condition", d.id).await,
        0,
        "une décision en attente au bout de la chaîne bloque l'export"
    );
    // 3. Elle est remplacée avant publication par une autre, publiée tout de suite : la remplacée (ni publiée ni caduque, mais
    //    **avec successeur**) ne bloque plus rien.
    ent.decider_complete_for_actor(etab, u, r, d.id, &corps("admis", Some(offre), "Revue publiée"), true, maintenant())
        .await
        .unwrap();
    let remplacee_non_publiee: i64 = nombre(
        &ent,
        "SELECT count(*) FROM ent_inscription_decision x WHERE x.dossier_id = $1 AND x.publiee_le IS NULL AND x.caduque_le IS NULL
            AND EXISTS (SELECT 1 FROM ent_inscription_decision y WHERE y.remplace_decision_id = x.id)",
        d.id,
    )
    .await;
    assert_eq!(remplacee_non_publiee, 1, "une décision remplacée avant publication existe");
    assert!(
        exportables(&ent, "ent_inscription_condition", d.id).await > 0,
        "une décision remplacée ne bloque plus l'export des conditions"
    );
}

// ───────────────────────────── M5 : garde à search_path fixé ─────────────────────────────

/// M5 : une table temporaire **remplie** homonyme ne trompe pas la garde de chaîne. Deux sens, qui distinguent le
/// comportement d'une fonction sans `search_path` fixé de celui d'une fonction qui lit la vraie table :
/// * un prédécesseur **absent de la vraie table** mais présent (même dossier) dans la table temporaire est refusé (`23514`) ;
/// * un prédécesseur **réel** du même dossier est accepté alors que la table temporaire dit qu'il appartient à un autre.
#[tokio::test]
async fn p4_garde_search_path_table_temporaire_remplie() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, _offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "GardeTemp", None).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let reel = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO public.ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne)
         VALUES ($1,$2,$3,'ajourne','Prédécesseur réel')",
    )
    .bind(reel)
    .bind(etab)
    .bind(d.id)
    .execute(&mut *tx)
    .await
    .unwrap();
    let faux = Uuid::new_v4();
    sqlx::query("CREATE TEMP TABLE ent_inscription_decision (id uuid, dossier_id uuid, etablissement_id uuid, remplace_decision_id uuid)")
        .execute(&mut *tx)
        .await
        .unwrap();
    // La table temporaire est REMPLIE : un « prédécesseur » valide du même dossier (absent de la vraie table), et le
    // prédécesseur réel présenté comme appartenant à un autre dossier.
    for (id, dossier_id) in [(faux, d.id), (reel, Uuid::new_v4())] {
        sqlx::query("INSERT INTO pg_temp.ent_inscription_decision (id, dossier_id, etablissement_id) VALUES ($1,$2,$3)")
            .bind(id)
            .bind(dossier_id)
            .bind(etab)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("SAVEPOINT s1").execute(&mut *tx).await.unwrap();
    let a_tort = sqlx::query(
        "INSERT INTO public.ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne, remplace_decision_id)
         VALUES ($1,$2,$3,'ajourne','Prédécesseur de la table temporaire',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(faux)
    .execute(&mut *tx)
    .await;
    assert_eq!(sqlstate(&a_tort), "23514", "prédécesseur seulement dans la table temporaire : refusé ({a_tort:?})");
    sqlx::query("ROLLBACK TO SAVEPOINT s1").execute(&mut *tx).await.unwrap();
    let a_raison = sqlx::query(
        "INSERT INTO public.ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne, remplace_decision_id)
         VALUES ($1,$2,$3,'ajourne','Prédécesseur réel',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(reel)
    .execute(&mut *tx)
    .await;
    assert!(a_raison.is_ok(), "prédécesseur réel du même dossier : accepté malgré la table temporaire ({a_raison:?})");
    tx.rollback().await.unwrap();
}

// ───────────────────────────── M6 : caducité et alerte ─────────────────────────────

async fn notifications_caduque(ent: &PgEnt, decision: Uuid) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE source_event_id = $1 AND kind = 'inscription_decision_caduque'",
    )
    .bind(decision)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

/// M6 : la caducité compare à une **famille d'états compatibles** par sens ; une décision programmée devenue caduque alerte
/// la direction (avis interne, aucun nom d'élève dans le titre).
#[tokio::test]
async fn p4_caducite_famille_d_etats_et_alerte_direction() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = super::tests_inscriptions::adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;

    // 1. `admis` programmée ; le dossier devient `confirme` avant l'échéance : décision toujours publiable.
    let admis = dossier(&ent, &c, camp, "AdmisConfirme", None).await;
    let mut b = corps("admis", Some(offre), "Admission programmée");
    b.publier_le = Some(maintenant() + jours(7));
    let d1 = ent.decider_complete_for_actor(etab, u, r, admis.id, &b, false, maintenant()).await.unwrap();
    sqlx::query("UPDATE ent_inscription_dossier SET etat = 'confirme' WHERE id = $1")
        .bind(admis.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();

    // 2. `ajourne` programmée ; le dossier passe en `attente_famille` : toujours publiable.
    let ajourne = dossier(&ent, &c, camp, "AjourneAttente", None).await;
    let mut b = corps("ajourne", None, "Ajournement programmé");
    b.publier_le = Some(maintenant() + jours(7));
    let d2 = ent.decider_complete_for_actor(etab, u, r, ajourne.id, &b, false, maintenant()).await.unwrap();
    ent.changer_statut_for_actor(
        etab,
        a,
        ar,
        ajourne.id,
        &StatutBody { statut: "attente_famille".into(), motif: None },
    )
    .await
    .unwrap();

    // 3. `ajourne` programmée ; le dossier est annulé : caduque, direction alertée.
    let annule = dossier(&ent, &c, camp, "AjourneAnnule", None).await;
    let mut b = corps("ajourne", None, "Ajournement programmé");
    b.publier_le = Some(maintenant() + jours(7));
    let d3 = ent.decider_complete_for_actor(etab, u, r, annule.id, &b, false, maintenant()).await.unwrap();
    ent.transition_for_actor(
        etab,
        u,
        r,
        annule.id,
        &TransitionBody { action: "annuler".into(), motif: "Annulation demandée".into() },
    )
    .await
    .unwrap();

    // 4. `liste_attente` programmée ; le dossier est refusé (autre famille d'états) : caduque.
    let attente = dossier(&ent, &c, camp, "AttenteRefusee", None).await;
    let mut b = corps("liste_attente", Some(offre), "Liste programmée");
    b.publier_le = Some(maintenant() + jours(7));
    let d4 = ent.decider_complete_for_actor(etab, u, r, attente.id, &b, false, maintenant()).await.unwrap();
    sqlx::query("UPDATE ent_inscription_dossier SET etat = 'refuse' WHERE id = $1")
        .bind(attente.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();

    let publiees = ent.publier_decisions_echues_tache(maintenant() + jours(7)).await.unwrap();
    assert_eq!(publiees, 2, "admis→confirme et ajourne→attente_famille sont publiées");
    let statut = |id: Uuid| async move {
        sqlx::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT publiee_le, caduque_le FROM ent_inscription_decision WHERE id = $1",
        )
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
    };
    for (id, nom) in [(d1.id, "admis→confirme"), (d2.id, "ajourne→attente_famille")] {
        let (publiee, caduque) = statut(id).await;
        assert!(publiee.is_some() && caduque.is_none(), "{nom} : publiée, pas caduque");
        assert!(notifications_caduque(&ent, id).await.is_empty(), "{nom} : aucune alerte");
    }
    for (id, nom, prenom) in [(d3.id, "ajourne→annule", "AjourneAnnule"), (d4.id, "liste_attente→refuse", "AttenteRefusee")] {
        let (publiee, caduque) = statut(id).await;
        assert!(publiee.is_none() && caduque.is_some(), "{nom} : caduque, jamais publiée");
        let alertes = notifications_caduque(&ent, id).await;
        assert!(!alertes.is_empty(), "{nom} : la direction est alertée");
        for (titre, corps_alerte) in alertes {
            assert!(!titre.contains(prenom) && !corps_alerte.contains(prenom), "{nom} : aucun nom d'élève dans l'avis");
        }
    }
    // La direction (le compte de test) reçoit bien l'avis interne.
    let destinataires: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE source_event_id = $1 AND destinataire_user_id = $2",
    )
    .bind(d3.id)
    .bind(u)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(destinataires, 1, "l'avis interne atteint la direction, une seule fois");
}

// ───────────────────────────── F1 : une seule connexion ─────────────────────────────

/// F1 : `dossier_dto` lit tout dans **une seule** transaction, sur **une seule** connexion, et relit l'état du dossier dans le
/// snapshot : avec un pool d'**une** connexion, il ne se bloque pas (il ne demande pas une seconde connexion).
#[tokio::test]
async fn p4_dossier_dto_une_seule_connexion() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "UneConnexion", None).await;
    decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis")).await.unwrap();
    let url = crate::url_base_de_test().unwrap();
    // Pool d'**une** connexion : toute lecture qui en demanderait une seconde pendant qu'elle tient la première échoue.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .unwrap();
    let petit = PgEnt::from_pool(pool);
    let (u, r) = dir(&c);
    let lecture = tokio::time::timeout(Duration::from_secs(20), petit.dossier_for_actor(etab, u, r, d.id))
        .await
        .expect("dossier_dto ne demande pas une seconde connexion pendant qu'il en tient une");
    let dto = lecture.unwrap();
    assert_eq!(dto.etat, "admis", "état lu dans le snapshot");
}
