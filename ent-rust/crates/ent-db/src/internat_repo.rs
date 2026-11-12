//! Dépôt SQL de l'internat (socle 41, lots I0 et I1).
//!
//! * Fonctions `*_tx` : travaillent sur une connexion de transaction déjà « locataire »
//!   (`TenantTx`, RLS) pour que le service garde **une seule transaction** par commande (verrou de la
//!   chambre, contrôles, insertion : R-IT10).
//! * [`PgInternatRepo`] : enveloppes d'une transaction courte pour les lectures et les écritures
//!   simples (compatibilité avec les appelants historiques).
//!
//! Toute requête filtre sur `etablissement_id` (garde statique d'isolation).

use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    AffectationInternat, ChambreGenre, ChambreInternat, DomainError, EtatChambre,
    PresenceInternat, PresenceInternatStatut, PublicChambre, SuiviInternat,
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub struct PgInternatRepo {
    pool: PgPool,
}

const COLONNES_CHAMBRE: &str = "id, etablissement_id, batiment, etage, code, capacite, genre, public, \
     zone_id, accessible_pmr, douche, lavabo, bureau, etat, ferme_du, ferme_au, notes_entretien, \
     actif, created_by_user_id, created_at";

const COLONNES_AFFECTATION: &str = "id, etablissement_id, eleve_id, chambre_id, lit, lit_id, sejour_id, \
     motif_code, debut, fin, created_by_user_id, created_at";

impl PgInternatRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn tx(&self, etab: Uuid) -> Result<TenantTx<'static>, DomainError> {
        TenantTx::begin(&self.pool, etab).await.map_err(map_sqlx)
    }

    pub async fn insert_chambre(&self, c: &ChambreInternat) -> Result<(), DomainError> {
        let mut tx = self.tx(c.etablissement_id).await?;
        insert_chambre_tx(&mut tx, c).await?;
        tx.commit().await.map_err(map_sqlx)
    }

    pub async fn get_chambre(
        &self,
        etablissement_id: Uuid,
        chambre_id: Uuid,
    ) -> Result<Option<ChambreInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        get_chambre_tx(&mut tx, etablissement_id, chambre_id, false).await
    }

    pub async fn list_chambres(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<ChambreInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let sql = format!(
            "SELECT {COLONNES_CHAMBRE} FROM ent_internat_chambre \
             WHERE etablissement_id = $1 AND ($2 = false OR actif = true) \
             ORDER BY batiment, etage, code"
        );
        let rows = sqlx::query_as::<_, ChambreRow>(&sql)
            .bind(etablissement_id)
            .bind(actifs_seulement)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(ChambreRow::into_chambre).collect())
    }

    pub async fn count_occupants_actifs(
        &self,
        etablissement_id: Uuid,
        chambre_id: Uuid,
        jour: NaiveDate,
    ) -> Result<i16, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM ent_internat_affectation \
             WHERE etablissement_id = $1 AND chambre_id = $2 AND debut <= $3 \
               AND (fin IS NULL OR fin >= $3)",
        )
        .bind(etablissement_id)
        .bind(chambre_id)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n as i16)
    }

    pub async fn insert_affectation(&self, a: &AffectationInternat) -> Result<(), DomainError> {
        let mut tx = self.tx(a.etablissement_id).await?;
        insert_affectation_tx(&mut tx, a).await?;
        tx.commit().await.map_err(map_sqlx)
    }

    pub async fn update_affectation_fin(&self, a: &AffectationInternat) -> Result<(), DomainError> {
        let mut tx = self.tx(a.etablissement_id).await?;
        update_affectation_fin_tx(&mut tx, a).await?;
        tx.commit().await.map_err(map_sqlx)
    }

    pub async fn get_affectation(
        &self,
        etablissement_id: Uuid,
        affectation_id: Uuid,
    ) -> Result<Option<AffectationInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let sql = format!(
            "SELECT {COLONNES_AFFECTATION} FROM ent_internat_affectation \
             WHERE etablissement_id = $1 AND id = $2"
        );
        let row = sqlx::query_as::<_, AffRow>(&sql)
            .bind(etablissement_id)
            .bind(affectation_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        Ok(row.map(AffRow::into_aff))
    }

    pub async fn get_affectation_active(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<AffectationInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        get_affectation_active_tx(&mut tx, etablissement_id, eleve_id, jour).await
    }

    /// Affectations actives pour un jour (appel du soir / grille internat).
    pub async fn list_affectations_actives(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<AffectationInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let sql = format!(
            "SELECT {COLONNES_AFFECTATION} FROM ent_internat_affectation \
             WHERE etablissement_id = $1 AND debut <= $2 AND (fin IS NULL OR fin >= $2) \
             ORDER BY chambre_id, lit NULLS LAST, debut"
        );
        let rows = sqlx::query_as::<_, AffRow>(&sql)
            .bind(etablissement_id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AffRow::into_aff).collect())
    }

    pub async fn has_affectation_ouverte(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM ent_internat_affectation \
             WHERE etablissement_id = $1 AND eleve_id = $2 AND fin IS NULL",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n > 0)
    }

    /// Ajoute un FAIT de présence (jamais d'écrasement : R-IT13). La base refuse une nuit future et
    /// une nuit passée sans motif (trigger de la migration 2820).
    pub async fn insert_presence_fait(
        &self,
        p: &PresenceInternat,
        motif_texte: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut tx = self.tx(p.etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_internat_presence_fait \
             (id, etablissement_id, eleve_id, affectation_id, nuit, marque, note, a, recu_le, \
              par_user_id, motif_correction_code, motif_correction_texte) \
             VALUES ($1,$2,$3,$4,$5,$6,$7, now(), now(), $8,$9,$10)",
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(p.eleve_id)
        .bind(p.affectation_id)
        .bind(p.jour)
        .bind(p.statut.as_str())
        .bind(&p.note)
        .bind(p.actor_user_id)
        .bind(&p.motif_correction_code)
        .bind(motif_texte)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)
    }

    /// Nuit courante de l'établissement (R-IT5), calculée par la base dans son fuseau.
    pub async fn nuit_courante(&self, etablissement_id: Uuid) -> Result<NaiveDate, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        sqlx::query_scalar("SELECT ent_nuit_etab($1, now())")
            .bind(etablissement_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx)
    }

    /// Valeur courante = fait le plus récent de l'élève pour la nuit.
    pub async fn get_presence(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<PresenceInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let row = sqlx::query_as::<_, PresenceRow>(
            "SELECT id, etablissement_id, eleve_id, affectation_id, nuit, marque, note, \
                    motif_correction_code, par_user_id, recu_le \
               FROM ent_internat_presence_fait \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND nuit = $3 \
              ORDER BY recu_le DESC, id DESC LIMIT 1",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(PresenceRow::into_presence))
    }

    /// Valeurs courantes d'une nuit (un fait par élève : le plus récent).
    pub async fn list_presences_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<PresenceInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let rows = sqlx::query_as::<_, PresenceRow>(
            "SELECT DISTINCT ON (eleve_id) id, etablissement_id, eleve_id, affectation_id, nuit, \
                    marque, note, motif_correction_code, par_user_id, recu_le \
               FROM ent_internat_presence_fait \
              WHERE etablissement_id = $1 AND nuit = $2 \
              ORDER BY eleve_id, recu_le DESC, id DESC",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(PresenceRow::into_presence).collect())
    }

    /// Historique complet des faits d'un élève pour une nuit (le plus ancien d'abord).
    pub async fn list_presence_faits(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<PresenceInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let rows = sqlx::query_as::<_, PresenceRow>(
            "SELECT id, etablissement_id, eleve_id, affectation_id, nuit, marque, note, \
                    motif_correction_code, par_user_id, recu_le \
               FROM ent_internat_presence_fait \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND nuit = $3 \
              ORDER BY recu_le, id",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(PresenceRow::into_presence).collect())
    }

    pub async fn insert_suivi(&self, s: &SuiviInternat) -> Result<(), DomainError> {
        let mut tx = self.tx(s.etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_internat_suivi \
             (id, etablissement_id, eleve_id, jour, texte, visible_famille, created_by_user_id, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(s.id)
        .bind(s.etablissement_id)
        .bind(s.eleve_id)
        .bind(s.jour)
        .bind(&s.texte)
        .bind(s.visible_famille)
        .bind(s.created_by_user_id)
        .bind(s.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)
    }

    /// Suivis d'un élève. `famille_seulement = true` : seulement ceux publiés à la famille ;
    /// `false` : TOUS, y compris les suivis privés — réservé aux rôles `internat.gerer` (RI-03).
    pub async fn list_suivis_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        famille_seulement: bool,
    ) -> Result<Vec<SuiviInternat>, DomainError> {
        let mut tx = self.tx(etablissement_id).await?;
        let rows = sqlx::query_as::<_, SuiviRow>(
            "SELECT id, etablissement_id, eleve_id, jour, texte, visible_famille, \
                    created_by_user_id, created_at \
               FROM ent_internat_suivi \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3 = false OR visible_famille = true) \
              ORDER BY jour DESC, created_at DESC",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(famille_seulement)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(SuiviRow::into_suivi).collect())
    }
}

// ───────────────────────── Fonctions sur transaction (service : une commande = une transaction) ─────────────────────────

/// Insère une chambre et ses lits (un lit par place, E3 : UUID applicatifs).
pub(crate) async fn insert_chambre_tx(
    conn: &mut PgConnection,
    c: &ChambreInternat,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO ent_internat_chambre \
         (id, etablissement_id, batiment, etage, code, capacite, genre, public, zone_id, \
          accessible_pmr, douche, lavabo, bureau, etat, ferme_du, ferme_au, notes_entretien, actif, \
          created_by_user_id, created_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
    )
    .bind(c.id)
    .bind(c.etablissement_id)
    .bind(&c.batiment)
    .bind(c.etage)
    .bind(&c.code)
    .bind(c.capacite)
    .bind(c.genre.as_str())
    .bind(c.public.map(PublicChambre::as_str))
    .bind(c.zone_id)
    .bind(c.accessible_pmr)
    .bind(c.douche)
    .bind(c.lavabo)
    .bind(c.bureau)
    .bind(c.etat.as_str())
    .bind(c.ferme_du)
    .bind(c.ferme_au)
    .bind(&c.notes_entretien)
    .bind(c.actif)
    .bind(c.created_by_user_id)
    .bind(c.created_at)
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    for numero in 1..=c.capacite {
        sqlx::query(
            "INSERT INTO ent_internat_lit (id, etablissement_id, chambre_id, numero) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(c.etablissement_id)
        .bind(c.id)
        .bind(numero)
        .execute(&mut *conn)
        .await
        .map_err(map_sqlx)?;
    }
    Ok(())
}

/// Lit une chambre ; `verrou = true` pose un verrou de ligne (`FOR UPDATE`) qui sérialise les
/// affectations et modifications concurrentes de la même chambre (R-IT10).
pub(crate) async fn get_chambre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    chambre_id: Uuid,
    verrou: bool,
) -> Result<Option<ChambreInternat>, DomainError> {
    let sql = format!(
        "SELECT {COLONNES_CHAMBRE} FROM ent_internat_chambre \
         WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    let row = sqlx::query_as::<_, ChambreRow>(&sql)
        .bind(etab)
        .bind(chambre_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx)?;
    Ok(row.and_then(ChambreRow::into_chambre))
}

/// Écrit une chambre modifiée et ajuste ses lits : ajout des numéros manquants ; retrait des
/// numéros en trop **seulement s'ils n'ont jamais été affectés** (sinon refus : neutraliser le lit).
pub(crate) async fn update_chambre_tx(
    conn: &mut PgConnection,
    c: &ChambreInternat,
) -> Result<(), DomainError> {
    sqlx::query(
        "UPDATE ent_internat_chambre SET code = $3, capacite = $4, genre = $5, public = $6, zone_id = $7, \
                accessible_pmr = $8, douche = $9, lavabo = $10, bureau = $11, etat = $12, ferme_du = $13, \
                ferme_au = $14, notes_entretien = $15, actif = $16, modifie_le = now() \
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(c.etablissement_id)
    .bind(c.id)
    .bind(&c.code)
    .bind(c.capacite)
    .bind(c.genre.as_str())
    .bind(c.public.map(PublicChambre::as_str))
    .bind(c.zone_id)
    .bind(c.accessible_pmr)
    .bind(c.douche)
    .bind(c.lavabo)
    .bind(c.bureau)
    .bind(c.etat.as_str())
    .bind(c.ferme_du)
    .bind(c.ferme_au)
    .bind(&c.notes_entretien)
    .bind(c.actif)
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    // Lits manquants.
    sqlx::query(
        "INSERT INTO ent_internat_lit (id, etablissement_id, chambre_id, numero) \
         SELECT gen_random_uuid(), $1, $2, n FROM generate_series(1, $3::int) AS n \
         ON CONFLICT (etablissement_id, chambre_id, numero) DO NOTHING",
    )
    .bind(c.etablissement_id)
    .bind(c.id)
    .bind(c.capacite)
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    // Lits en trop : jamais affectés → retirés ; déjà utilisés → refus.
    let utilises: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM ent_internat_lit l \
          WHERE l.etablissement_id = $1 AND l.chambre_id = $2 AND l.numero > $3 \
            AND EXISTS (SELECT 1 FROM ent_internat_affectation a \
                         WHERE a.etablissement_id = l.etablissement_id AND a.lit_id = l.id)",
    )
    .bind(c.etablissement_id)
    .bind(c.id)
    .bind(c.capacite)
    .fetch_one(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    if utilises > 0 {
        return Err(DomainError::InternatCapaciteSousOccupation);
    }
    sqlx::query(
        "DELETE FROM ent_internat_lit WHERE etablissement_id = $1 AND chambre_id = $2 AND numero > $3",
    )
    .bind(c.etablissement_id)
    .bind(c.id)
    .bind(c.capacite)
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    Ok(())
}

pub(crate) async fn insert_affectation_tx(
    conn: &mut PgConnection,
    a: &AffectationInternat,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO ent_internat_affectation \
         (id, etablissement_id, eleve_id, chambre_id, lit, lit_id, sejour_id, motif_code, debut, fin, \
          created_by_user_id, created_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
    )
    .bind(a.id)
    .bind(a.etablissement_id)
    .bind(a.eleve_id)
    .bind(a.chambre_id)
    .bind(&a.lit)
    .bind(a.lit_id)
    .bind(a.sejour_id)
    .bind(&a.motif_code)
    .bind(a.debut)
    .bind(a.fin)
    .bind(a.created_by_user_id)
    .bind(a.created_at)
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    Ok(())
}

pub(crate) async fn update_affectation_fin_tx(
    conn: &mut PgConnection,
    a: &AffectationInternat,
) -> Result<(), DomainError> {
    sqlx::query(
        "UPDATE ent_internat_affectation SET fin = $3, modifie_le = now() \
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(a.id)
    .bind(a.fin)
    .execute(&mut *conn)
    .await
    .map_err(map_sqlx)?;
    Ok(())
}

pub(crate) async fn get_affectation_active_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
) -> Result<Option<AffectationInternat>, DomainError> {
    let sql = format!(
        "SELECT {COLONNES_AFFECTATION} FROM ent_internat_affectation \
         WHERE etablissement_id = $1 AND eleve_id = $2 AND debut <= $3 AND (fin IS NULL OR fin >= $3) \
         ORDER BY debut DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, AffRow>(&sql)
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx)?;
    Ok(row.map(AffRow::into_aff))
}

/// Affectation ouverte (sans fin) de l'élève, quelle que soit sa date de début.
pub(crate) async fn get_affectation_ouverte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
) -> Result<Option<AffectationInternat>, DomainError> {
    let sql = format!(
        "SELECT {COLONNES_AFFECTATION} FROM ent_internat_affectation \
         WHERE etablissement_id = $1 AND eleve_id = $2 AND fin IS NULL LIMIT 1"
    );
    let row = sqlx::query_as::<_, AffRow>(&sql)
        .bind(etab)
        .bind(eleve_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx)?;
    Ok(row.map(AffRow::into_aff))
}

/// `(debut, fin)` des affectations d'une chambre qui couvrent un jour ≥ `depuis`
/// (ouvertes, ou terminées après `depuis`) : entrée du calcul du pic d'occupation.
pub(crate) async fn periodes_chambre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    chambre_id: Uuid,
    depuis: NaiveDate,
) -> Result<Vec<(NaiveDate, Option<NaiveDate>)>, DomainError> {
    sqlx::query_as::<_, (NaiveDate, Option<NaiveDate>)>(
        "SELECT debut, fin FROM ent_internat_affectation \
          WHERE etablissement_id = $1 AND chambre_id = $2 AND (fin IS NULL OR fin >= $3)",
    )
    .bind(etab)
    .bind(chambre_id)
    .bind(depuis)
    .fetch_all(&mut *conn)
    .await
    .map_err(map_sqlx)
}

/// Garde-fou d'erreur : traduit les erreurs levées par les gardes de la base (migrations 2820/2823)
/// en erreurs de domaine précises ; le reste passe par la classification commune.
pub(crate) fn map_sqlx(e: sqlx::Error) -> DomainError {
    if let sqlx::Error::Database(db) = &e {
        let code = db.code().map(|c| c.to_string()).unwrap_or_default();
        let msg = db.message();
        if msg.contains("chambre pleine") {
            return DomainError::InternatChambrePleine;
        }
        if msg.contains("nuit future") {
            return DomainError::InternatNuitFuture;
        }
        if msg.contains("motif de correction obligatoire") {
            return DomainError::InternatMotifRequis;
        }
        if code == "23P01" {
            if msg.contains("lit_sans_chevauchement") {
                return DomainError::InternatLitIndisponible;
            }
            if msg.contains("sejour_sans_chevauchement") {
                return DomainError::InternatSejourInvalide;
            }
        }
        if code == "23505" && msg.contains("affectation_active_uq") {
            return DomainError::InternatDejaAffecte;
        }
        if code == "23505" && msg.contains("ent_internat_chambre_code_uq") {
            return DomainError::Conflit;
        }
    }
    crate::pg_err::map_sqlx("internat", e)
}

#[derive(sqlx::FromRow)]
struct ChambreRow {
    id: Uuid,
    etablissement_id: Uuid,
    batiment: String,
    etage: i16,
    code: String,
    capacite: i16,
    genre: String,
    public: Option<String>,
    zone_id: Option<Uuid>,
    accessible_pmr: bool,
    douche: bool,
    lavabo: bool,
    bureau: bool,
    etat: String,
    ferme_du: Option<NaiveDate>,
    ferme_au: Option<NaiveDate>,
    notes_entretien: Option<String>,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ChambreRow {
    fn into_chambre(self) -> Option<ChambreInternat> {
        Some(ChambreInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            batiment: self.batiment,
            etage: self.etage,
            code: self.code,
            capacite: self.capacite,
            genre: ChambreGenre::parse(&self.genre)?,
            public: self.public.as_deref().and_then(PublicChambre::parse),
            zone_id: self.zone_id,
            accessible_pmr: self.accessible_pmr,
            douche: self.douche,
            lavabo: self.lavabo,
            bureau: self.bureau,
            etat: EtatChambre::parse(&self.etat)?,
            ferme_du: self.ferme_du,
            ferme_au: self.ferme_au,
            notes_entretien: self.notes_entretien,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AffRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    chambre_id: Uuid,
    lit: Option<String>,
    lit_id: Option<Uuid>,
    sejour_id: Option<Uuid>,
    motif_code: Option<String>,
    debut: NaiveDate,
    fin: Option<NaiveDate>,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl AffRow {
    fn into_aff(self) -> AffectationInternat {
        AffectationInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            chambre_id: self.chambre_id,
            lit: self.lit,
            lit_id: self.lit_id,
            sejour_id: self.sejour_id,
            motif_code: self.motif_code,
            debut: self.debut,
            fin: self.fin,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PresenceRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    affectation_id: Option<Uuid>,
    nuit: NaiveDate,
    marque: String,
    note: String,
    motif_correction_code: Option<String>,
    par_user_id: Uuid,
    recu_le: DateTime<Utc>,
}

impl PresenceRow {
    fn into_presence(self) -> Option<PresenceInternat> {
        Some(PresenceInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            // Un fait sans affectation (attendu hors hébergement, lot I2) n'existe pas encore en I0.
            affectation_id: self.affectation_id.unwrap_or_default(),
            jour: self.nuit,
            statut: PresenceInternatStatut::parse(&self.marque)?,
            note: self.note,
            motif_correction_code: self.motif_correction_code,
            actor_user_id: self.par_user_id,
            created_at: self.recu_le,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SuiviRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    texte: String,
    visible_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl SuiviRow {
    fn into_suivi(self) -> SuiviInternat {
        SuiviInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            texte: self.texte,
            visible_famille: self.visible_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
