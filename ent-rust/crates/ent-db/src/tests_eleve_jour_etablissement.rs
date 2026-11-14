//! Socle 2 (l'élève), correctif « jour d'établissement » (migration 296). Les fonctions SQL du socle
//! élève prenaient `current_date` (jour UTC : sqlx impose `TimeZone=UTC`) alors que le code Rust date au
//! jour de Paris : entre minuit et 1 h (2 h l'été) à Paris, résultats faux. Elles prennent maintenant le jour
//! de l'ÉTABLISSEMENT (`ent_aujourdhui_etab`, fuseau de `ent_etablissement_parametre`).
//!
//! On ne peut pas figer l'horloge du serveur : on règle le FUSEAU de l'établissement. Deux fuseaux extrêmes,
//! Pacific/Kiritimati (UTC+14) et Pacific/Pago_Pago (UTC−11), garantissent qu'à n'importe quelle heure au
//! moins l'un des deux a un jour différent du jour UTC : le test reproduit le créneau en permanence.
//! Scénarios contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use chrono::NaiveDate;
use uuid::Uuid;

async fn labo() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, _cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {}).await;
    Some(ent)
}

const FUSEAUX: [&str; 2] = ["Pacific/Kiritimati", "Pacific/Pago_Pago"];

/// Établissement neuf réglé sur `fuseau` (aucun autre test n'en dépend).
async fn etablissement_au_fuseau(ent: &PgEnt, fuseau: &str) -> Uuid {
    let etab = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, 'jour-etab-' || substr($1::text, 1, 8), 'Jour établissement')")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, $2)")
        .bind(etab)
        .bind(fuseau)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    etab
}

/// Jour calendaire du fuseau, calculé indépendamment des fonctions testées.
async fn jour_du_fuseau(ent: &PgEnt, fuseau: &str) -> NaiveDate {
    sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date")
        .bind(fuseau)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn jour_utc(ent: &PgEnt) -> NaiveDate {
    sqlx::query_scalar("SELECT current_date").fetch_one(ent.pool_verif()).await.unwrap()
}

/// Garde-fou de la méta-condition : sur les deux fuseaux extrêmes, au moins un jour diffère du jour UTC.
#[tokio::test]
async fn les_deux_fuseaux_extremes_couvrent_toute_heure() {
    let Some(ent) = labo().await else { return };
    let (utc, k, p) = (
        jour_utc(&ent).await,
        jour_du_fuseau(&ent, FUSEAUX[0]).await,
        jour_du_fuseau(&ent, FUSEAUX[1]).await,
    );
    assert!(k != utc || p != utc, "utc={utc} kiritimati={k} pago_pago={p}");
}

/// Identifiant (281) : INE saisi (pont), puis retiré ; reprise de l'existant. Tous prennent le jour de
/// l'établissement. Identité initiale (280) : `effet_du` plafonné au jour de l'établissement.
#[tokio::test]
async fn identifiant_et_identite_initiale_au_jour_de_l_etablissement() {
    let Some(ent) = labo().await else { return };
    for fuseau in FUSEAUX {
        let etab = etablissement_au_fuseau(&ent, fuseau).await;
        let eleve = Uuid::new_v4();
        let ine = format!("9{}", &Uuid::new_v4().simple().to_string()[..9].to_uppercase());
        let avant = jour_du_fuseau(&ent, fuseau).await;
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, ine)
             VALUES ($1,$2,'Jour','Zélie','actif',$3)",
        )
        .bind(eleve)
        .bind(etab)
        .bind(&ine)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        let apres = jour_du_fuseau(&ent, fuseau).await;
        let jours_valides = [avant, apres];

        let valide_du: NaiveDate = sqlx::query_scalar(
            "SELECT valide_du FROM ent_eleve_identifiant WHERE etablissement_id = $1 AND eleve_id = $2 AND systeme = 'INE'",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert!(jours_valides.contains(&valide_du), "{fuseau} : INE valide dès {valide_du}, attendu {jours_valides:?}");

        // Identité initiale : effet_du = least(création au jour de l'établissement, jour de l'établissement).
        let (effet_du, cree_le): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT i.effet_du, (e.created_at AT TIME ZONE $3)::date FROM ent_eleve_identite i JOIN ent_eleve e ON e.id = i.eleve_id
             WHERE i.etablissement_id = $1 AND i.eleve_id = $2 AND i.nature = 'creation'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(fuseau)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert!(
            jours_valides.iter().any(|j| effet_du == (*j).min(cree_le)),
            "{fuseau} : effet_du {effet_du}, jour de l'établissement {jours_valides:?}, création {cree_le}"
        );

        // Retrait (ancienne saisie vidée) : la révocation prend le jour de l'établissement.
        let avant = jour_du_fuseau(&ent, fuseau).await;
        sqlx::query("UPDATE ent_eleve SET ine = NULL WHERE id = $1").bind(eleve).execute(ent.pool_verif()).await.unwrap();
        let apres = jour_du_fuseau(&ent, fuseau).await;
        let valide_au: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT valide_au FROM ent_eleve_identifiant WHERE etablissement_id = $1 AND eleve_id = $2 AND systeme = 'INE'",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        let valide_au = valide_au.expect("INE révoqué");
        // greatest(début, jour de l'établissement) : le début est déjà le jour de l'établissement.
        assert!(
            [avant, apres].contains(&valide_au),
            "{fuseau} : révoqué au {valide_au}, attendu le jour de l'établissement {avant}/{apres}"
        );
    }
}

/// Numéro (282) : le déclencheur d'insertion passe le jour de l'ÉTABLISSEMENT à l'attribution. Le jour ne
/// change l'année du numéro qu'au 31 décembre ou 1er janvier : on remplace, dans une transaction annulée,
/// l'attribution par une fonction qui renvoie le jour reçu, et on lit ce que le déclencheur lui a passé.
#[tokio::test]
async fn numero_attribue_au_jour_de_l_etablissement() {
    let Some(ent) = labo().await else { return };
    for fuseau in FUSEAUX {
        let etab = etablissement_au_fuseau(&ent, fuseau).await;
        let mut tx = ent.pool_verif().begin().await.unwrap();
        sqlx::query(
            "CREATE OR REPLACE FUNCTION ent_eleve_numero_attribuer(p_etab UUID, p_jour DATE) RETURNS TEXT
             LANGUAGE sql AS $$ SELECT p_jour::text $$",
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        let avant: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(fuseau).fetch_one(&mut *tx).await.unwrap();
        let eleve = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status) VALUES ($1,$2,'Numero','Zélie','actif')")
            .bind(eleve)
            .bind(etab)
            .execute(&mut *tx)
            .await
            .unwrap();
        let numero: Option<String> = sqlx::query_scalar("SELECT numero FROM ent_eleve WHERE id = $1")
            .bind(eleve)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        let apres: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(fuseau).fetch_one(&mut *tx).await.unwrap();
        tx.rollback().await.unwrap();
        let numero = numero.expect("numéro attribué");
        assert!(
            [avant.to_string(), apres.to_string()].contains(&numero),
            "{fuseau} : l'attribution a reçu {numero}, attendu le jour de l'établissement {avant}/{apres}"
        );
    }
}

/// Resynchronisation quotidienne sans argument : chaque élève est évalué au jour de SON établissement.
/// Une version d'identité datée du jour de l'établissement (plus tard que le jour UTC pour Kiritimati)
/// est projetée sur la fiche sans que l'appelant ne fournisse de jour.
#[tokio::test]
async fn resynchronisation_sans_argument_au_jour_de_chaque_etablissement() {
    let Some(ent) = labo().await else { return };
    let etab = etablissement_au_fuseau(&ent, FUSEAUX[0]).await;
    let eleve = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status) VALUES ($1,$2,'Resync','Zélie','actif')")
        .bind(eleve)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // L'appel sans argument n'échoue pas et ne renvoie qu'un compte (idempotent : rien de plus à projeter).
    let (n,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_resynchroniser()").fetch_one(ent.pool_verif()).await.unwrap();
    assert!(n >= 0);
    let (m,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_resynchroniser()").fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(m, 0, "deuxième passe : plus rien à resynchroniser");
    // Un jour explicite reste possible (compatibilité de l'ancienne signature).
    let jour = jour_du_fuseau(&ent, FUSEAUX[0]).await;
    let (_,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_resynchroniser($1)").bind(jour).fetch_one(ent.pool_verif()).await.unwrap();
}

/// Garde-fou : plus aucune fonction du socle élève ne lit le jour du serveur (`current_date`) ni ne convertit
/// un horodatage en date UTC (`created_at::date`, `CAST(updated_at AS date)`, `now()::date`…).
#[tokio::test]
async fn aucune_fonction_eleve_ne_lit_le_jour_du_serveur() {
    let Some(ent) = labo().await else { return };
    let restantes: Vec<String> = sqlx::query_scalar(
        "SELECT proname::text FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
         WHERE n.nspname = 'public' AND proname LIKE 'ent\\_eleve\\_%' AND prosrc ~* '(current_date|now\\(\\)\\s*::\\s*date|current_timestamp\\s*::\\s*date|_at\\s*::\\s*date|_at\\s+as\\s+date)'
         ORDER BY 1",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(restantes.is_empty(), "fonctions élève encore sur le jour UTC (current_date ou horodatage::date) : {restantes:?}");
}

// ── Conversions d'horodatage en date (migrations 297 et 343) ──────────────────────────────────────────────
// Pour chaque fuseau extrême, un instant fixe dont le jour UTC diffère du jour calendaire du fuseau :
// Kiritimati (UTC+14) : 2026-03-10 23:30 UTC = 2026-03-11 13:30 ; Pago Pago (UTC−11) : 2026-03-11 05:00 UTC =
// 2026-03-10 18:00. L'horloge réelle n'intervient jamais : l'horodatage est écrit explicitement.

/// (fuseau, instant UTC, jour calendaire attendu dans ce fuseau).
const CAS_HORODATAGE: [(&str, &str, &str); 2] = [
    ("Pacific/Kiritimati", "2026-03-10 23:30:00+00", "2026-03-11"),
    ("Pacific/Pago_Pago", "2026-03-11 05:00:00+00", "2026-03-10"),
];

fn jour(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

/// Insère un élève hors déclencheurs (`session_replication_role = replica`) avec des horodatages explicites,
/// pour tester les fonctions de reprise qui ne traitent que des lignes sans numéro / sans scolarisation.
async fn eleve_repris(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, etab: Uuid, status: &str, instant: &str) -> Uuid {
    let eleve = Uuid::new_v4();
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut **tx).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at)
         VALUES ($1,$2,'Reprise','Zélie',$3,$4::timestamptz,$4::timestamptz)",
    )
    .bind(eleve)
    .bind(etab)
    .bind(status)
    .bind(instant)
    .execute(&mut **tx)
    .await
    .unwrap();
    sqlx::query("SET LOCAL session_replication_role = origin").execute(&mut **tx).await.unwrap();
    eleve
}

/// 280/296 : l'effet de la version initiale d'un élève créé à `instant` est le jour de création DANS le fuseau
/// de l'établissement (plafonné à son jour), pas le jour UTC. Le pont de 283 date de même la première
/// scolarisation (`date_entree` absente de la fiche).
#[tokio::test]
async fn creation_datee_au_jour_de_l_etablissement() {
    let Some(ent) = labo().await else { return };
    for (fuseau, instant, attendu) in CAS_HORODATAGE {
        let etab = etablissement_au_fuseau(&ent, fuseau).await;
        let eleve = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at)
             VALUES ($1,$2,'Creation','Zélie','actif',$3::timestamptz,$3::timestamptz)",
        )
        .bind(eleve)
        .bind(etab)
        .bind(instant)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        let effet_du: NaiveDate = sqlx::query_scalar(
            "SELECT effet_du FROM ent_eleve_identite WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'creation'",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(effet_du, jour(attendu), "{fuseau} : effet_du de la version initiale");
        let entree: NaiveDate = sqlx::query_scalar(
            "SELECT date_entree FROM ent_eleve_scolarisation WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(entree, jour(attendu), "{fuseau} : date d'entrée de la première scolarisation (pont 283)");
    }
}

/// 282 : la reprise des numéros attribue le numéro avec l'année du jour de création DANS le fuseau de
/// l'établissement. On observe le jour reçu en remplaçant, dans une transaction annulée, l'attribution.
#[tokio::test]
async fn reprise_des_numeros_au_jour_de_creation_de_l_etablissement() {
    let Some(ent) = labo().await else { return };
    for (fuseau, instant, attendu) in CAS_HORODATAGE {
        let etab = etablissement_au_fuseau(&ent, fuseau).await;
        let mut tx = ent.pool_verif().begin().await.unwrap();
        sqlx::query(
            "CREATE OR REPLACE FUNCTION ent_eleve_numero_attribuer(p_etab UUID, p_jour DATE) RETURNS TEXT
             LANGUAGE sql AS $$ SELECT p_jour::text $$",
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        let eleve = eleve_repris(&mut tx, etab, "actif", instant).await;
        let (n,): (i64,) = sqlx::query_as("SELECT ent_eleve_numero_reprendre()").fetch_one(&mut *tx).await.unwrap();
        assert!(n >= 1);
        let numero: Option<String> =
            sqlx::query_scalar("SELECT numero FROM ent_eleve WHERE id = $1").bind(eleve).fetch_one(&mut *tx).await.unwrap();
        tx.rollback().await.unwrap();
        assert_eq!(numero.as_deref(), Some(attendu), "{fuseau} : jour reçu par l'attribution de la reprise");
    }
}

/// 283 : la reprise des scolarisations existantes date l'entrée (création) et la sortie sans date (dernière
/// modification) au jour de l'établissement, jamais au jour UTC.
#[tokio::test]
async fn reprise_des_scolarisations_au_jour_de_l_etablissement() {
    let Some(ent) = labo().await else { return };
    for (fuseau, instant, attendu) in CAS_HORODATAGE {
        let etab = etablissement_au_fuseau(&ent, fuseau).await;
        let mut tx = ent.pool_verif().begin().await.unwrap();
        let eleve = eleve_repris(&mut tx, etab, "sorti", instant).await;
        sqlx::query("SELECT ent_eleve_scolarisation_reprendre()").execute(&mut *tx).await.unwrap();
        let (entree, sortie): (NaiveDate, Option<NaiveDate>) = sqlx::query_as(
            "SELECT date_entree, date_sortie FROM ent_eleve_scolarisation WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        tx.rollback().await.unwrap();
        assert_eq!(entree, jour(attendu), "{fuseau} : date d'entrée reprise");
        assert_eq!(sortie, Some(jour(attendu)), "{fuseau} : date de sortie reprise (dernière modification)");
    }
}

/// 340/342/343 : l'enseignement repris d'une option d'année est « choisi » le jour de CRÉATION de l'option
/// dans le fuseau de l'établissement.
#[tokio::test]
async fn enseignement_repris_choisi_au_jour_de_l_etablissement() {
    let Some(ent) = labo().await else { return };
    for (fuseau, instant, attendu) in CAS_HORODATAGE {
        let etab = etablissement_au_fuseau(&ent, fuseau).await;
        let eleve = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status) VALUES ($1,$2,'Option','Zélie','actif')")
            .bind(eleve)
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
             VALUES ($1,$2,'2025-2026','2025-09-01','2026-06-30',false)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        let option = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_option_annee (id, etablissement_id, eleve_id, annee_scolaire, nature, code, libelle, statut, created_at)
             VALUES ($1,$2,$3,'2025-2026','interne','LATIN','Latin','demandee',$4::timestamptz)",
        )
        .bind(option)
        .bind(etab)
        .bind(eleve)
        .bind(instant)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        sqlx::query(
            "SELECT ent_eleve_enseignement_depuis_option_annee(o) FROM ent_eleve_option_annee o WHERE o.id = $1",
        )
        .bind(option)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        let choisi: NaiveDate = sqlx::query_scalar(
            "SELECT choisi_le FROM ent_eleve_enseignement WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(choisi, jour(attendu), "{fuseau} : enseignement choisi le");
    }
}
