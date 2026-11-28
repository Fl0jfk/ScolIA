//! Socle 21, lot V14 « cohabitation de l'an 1 » (`docs/socles/21_EVALUATIONS.md` §16, §17.15 ; EVA-233 à EVA-237, CE-89 ;
//! livraison : `docs/socles/21_EVALUATIONS_V14_LIVRAISON.md`).
//!
//! L'an 1 du lycée pilote (D-032), notes et bulletins restent dans le logiciel tiers (Charlemagne). ScolIA :
//! * **reçoit** les bulletins PDF de ce logiciel (un dépôt par période, un fichier par élève, EVA-234) ;
//! * les **rapproche** des élèves : INE d'abord, puis nom, prénom et date de naissance, tous concordants et uniques ;
//!   sinon le fichier reste dans la file « à rapprocher » et **n'est jamais distribué** (CE-89) ;
//! * les **distribue seul** (EVA-235) : un document au coffre de l'élève (tiroir famille, lu sous le droit « voir les
//!   notes », comme un bulletin produit par ScolIA) ; destinataires relus **au moment de la distribution** et figés :
//!   responsables au lien actif avec `peut_voir_notes`, hors interdiction de communication, et le compte de l'élève ;
//! * **exporte l'assiduité** de la période par élève (EVA-233), bornée à la période et à aujourd'hui, chaque export
//!   journalisé ;
//! * règle le **bandeau « où sont les notes »** (EVA-237).
//!
//! Droits (valeurs par défaut prudentes, D-069, sans nouvelle action) :
//! * déposer, rapprocher, écarter, voir la file et l'aperçu : `imports.appliquer` (secrétariat, direction) ou
//!   `bulletins.publier` ;
//! * contrôler, distribuer, retirer, régler le bandeau : `bulletins.publier` (direction par défaut) ;
//! * exporter l'assiduité : `exports.officiels` (secrétariat, direction).
//!
//! Ordre des contrôles (règle 5) : personnel, droit, puis existence ; une famille qui demande un bulletin qu'elle ne
//! peut pas lire reçoit « introuvable », jamais un indice d'existence.

use chrono::NaiveDate;
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use ent_object::upload::{TypeFichier, UsageFichier};
use ent_object::ObjectStore;
use ent_types::evaluations_v14::{
    BulletinTiersDto, BulletinTiersFamilleDto, CohabitationDto, DeposerBulletinTiersBody,
    EleveSansDestinataireDto, ExportAssiduiteTiersDto, LigneAssiduiteTiersDto, LotTiersDto,
    MotifBulletinTiersBody, OuvrirLotBody, RapportDistributionDto, RapprocherBulletinBody,
    ReglerCohabitationBody, LOGICIELS_TIERS,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus, texte_borne};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Événements du lot (charges sans note ni appréciation, R-M14).
pub mod evenements {
    /// Un bulletin du logiciel tiers vient d'être distribué (agrégat : l'élève) : avis aux destinataires figés.
    pub const BULLETIN_TIERS_DISTRIBUE: &str = "evaluations.bulletin_tiers_distribue";
    /// Un bulletin distribué est retiré (erreur de rapprochement, version fautive).
    pub const BULLETIN_TIERS_RETIRE: &str = "evaluations.bulletin_tiers_retire";
    /// Un export d'assiduité vers le logiciel tiers a été produit (même type que les exports de livrets).
    pub const EXPORT_ASSIDUITE_TIERS: &str = crate::evaluations::types_eva::EXPORT_REALISE;
}

/// Fichiers d'un dépôt au plus (une classe entière par fichier n'est pas découpée ici).
pub const MAX_FICHIERS_LOT: i64 = 2000;

const SANS_ACCES: &str = "aucun droit sur les bulletins du logiciel tiers";

fn empreinte(octets: &[u8]) -> String {
    Sha256::digest(octets).iter().map(|b| format!("{b:02x}")).collect()
}

fn motif_valide(m: &str) -> Result<String, EntDbError> {
    let m = m.trim();
    if !(3..=500).contains(&m.chars().count()) {
        return Err(refus("motif de 3 à 500 caractères requis"));
    }
    texte_borne(m, 500)
}

fn indice(v: &Option<String>, max: usize) -> Result<Option<String>, EntDbError> {
    match v.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) if s.chars().count() > max => Err(refus("indice de rapprochement trop long")),
        Some(s) => Ok(Some(texte_borne(s, max)?)),
    }
}

/// INE au format national : 9 chiffres et 2 lettres, ou 10 chiffres et 1 lettre (chiffres et lettres majuscules).
pub fn ine_plausible(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 11
        && (b[..9].iter().all(u8::is_ascii_digit)
            && b[9].is_ascii_alphanumeric()
            && b[10].is_ascii_uppercase()
            && (b[9].is_ascii_digit() || b[9].is_ascii_uppercase()))
}

/// INE lu dans le nom du fichier (`2NDE_A_123456789AB_DUPONT.pdf`) : un seul segment plausible, sinon rien.
pub fn ine_du_nom_fichier(nom: &str) -> Option<String> {
    let base = nom.rsplit(['/', '\\']).next().unwrap_or(nom);
    let tige = base.rsplit_once('.').map(|(t, _)| t).unwrap_or(base);
    let trouves: Vec<String> = tige
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_uppercase)
        .filter(|s| ine_plausible(s))
        .collect();
    match trouves.as_slice() {
        [un] => Some(un.clone()),
        _ => None,
    }
}

/// Champ de tableur (échappé contre l'injection de formule, séparateur `;`).
fn cellule(s: &str) -> String {
    crate::voyages::champ_csv(s)
}

/// CSV de l'export d'assiduité (UTF-8 avec BOM, séparateur `;`, lisible d'un tableur français).
pub fn csv_assiduite_tiers(e: &ExportAssiduiteTiersDto, classe: &str) -> String {
    let mut s = String::from("\u{feff}");
    s.push_str(
        "INE;Nom;Prénom;Date de naissance;Classe;Du;Au;Demi-journées d'absence;Justifiées;Non justifiées;Retards\r\n",
    );
    for l in &e.lignes {
        let champs = [
            cellule(l.ine.as_deref().unwrap_or("")),
            cellule(&l.nom),
            cellule(&l.prenom),
            l.date_naissance.map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_default(),
            cellule(classe),
            e.du.format("%d/%m/%Y").to_string(),
            e.au.format("%d/%m/%Y").to_string(),
            l.demi_journees.to_string(),
            l.justifiees.to_string(),
            l.non_justifiees.to_string(),
            l.retards.to_string(),
        ];
        s.push_str(&champs.join(";"));
        s.push_str("\r\n");
    }
    s
}

struct PeriodeV14 {
    code: String,
    libelle: String,
    debut: NaiveDate,
    fin: NaiveDate,
}

const COLONNES_BULLETIN: &str = "b.id, b.lot_id, b.periode_id, b.nom_fichier, b.taille, b.statut, b.indice_ine, b.indice_nom,
    b.indice_prenom, b.indice_naissance, b.eleve_id, e.nom AS eleve_nom, e.prenom AS eleve_prenom, b.rapprochement, b.motif,
    b.distribue_le,
    (SELECT count(*) FROM eva_tiers_destinataire d WHERE d.bulletin_id = b.id)::int AS destinataires";

const FROM_BULLETIN: &str = "FROM eva_tiers_bulletin b
    LEFT JOIN ent_eleve e ON e.id = b.eleve_id AND e.etablissement_id = b.etablissement_id";

fn dto_bulletin(r: &PgRow) -> Result<BulletinTiersDto, EntDbError> {
    Ok(BulletinTiersDto {
        id: r.try_get("id").map_err(db)?,
        lot_id: r.try_get("lot_id").map_err(db)?,
        periode_id: r.try_get("periode_id").map_err(db)?,
        nom_fichier: r.try_get("nom_fichier").map_err(db)?,
        taille: r.try_get("taille").map_err(db)?,
        statut: r.try_get("statut").map_err(db)?,
        indice_ine: r.try_get("indice_ine").map_err(db)?,
        indice_nom: r.try_get("indice_nom").map_err(db)?,
        indice_prenom: r.try_get("indice_prenom").map_err(db)?,
        indice_naissance: r.try_get("indice_naissance").map_err(db)?,
        eleve_id: r.try_get("eleve_id").map_err(db)?,
        eleve_nom: r.try_get("eleve_nom").map_err(db)?,
        eleve_prenom: r.try_get("eleve_prenom").map_err(db)?,
        rapprochement: r.try_get("rapprochement").map_err(db)?,
        motif: r.try_get("motif").map_err(db)?,
        distribue_le: r.try_get("distribue_le").map_err(db)?,
        destinataires: r.try_get::<i32, _>("destinataires").map_err(db)?.max(0) as u32,
    })
}

/// Élèves que l'on peut rapprocher d'un bulletin de la période : de l'établissement, non absorbés par une fusion,
/// scolarisés (une affectation) pendant la période.
const CANDIDATS: &str = "SELECT e.id, e.ine FROM ent_eleve e
     WHERE e.etablissement_id = $1 AND e.fusionne_dans IS NULL
       AND EXISTS (SELECT 1 FROM ent_affectation a
                    WHERE a.etablissement_id = e.etablissement_id AND a.eleve_id = e.id
                      AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $2))";

/// Journal (ajout seul).
#[allow(clippy::too_many_arguments)]
async fn journaliser(
    c: &mut PgConnection,
    etab: Uuid,
    bulletin: Uuid,
    action: &str,
    eleve: Option<Uuid>,
    voie: Option<&str>,
    acteur: Uuid,
    motif: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO eva_tiers_journal (id, etablissement_id, bulletin_id, action, eleve_id, voie, acteur, motif)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(bulletin)
    .bind(action)
    .bind(eleve)
    .bind(voie)
    .bind(acteur)
    .bind(motif)
    .execute(&mut *c)
    .await
    .map_err(db)?;
    Ok(())
}

/// Destinataires d'un bulletin, relus à l'instant (EVA-235) : responsables au lien **communicant** (actif, personne non
/// absorbée, hors interdiction de communication) avec `peut_voir_notes`, compte parent membre ; et le compte de l'élève.
async fn destinataires(c: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<Vec<(Uuid, &'static str)>, EntDbError> {
    let rows: Vec<(Uuid, bool)> = sqlx::query_as(
        "SELECT DISTINCT l.user_id, false
           FROM ent_eleve_lien_communicant l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.peut_voir_notes AND l.user_id IS NOT NULL
            AND m.role = 'parent'
            -- Le même compte privé du droit sur un autre de ses liens vers cet élève n'est pas destinataire.
            AND NOT EXISTS (SELECT 1 FROM ent_eleve_lien x
                             WHERE x.etablissement_id = l.etablissement_id AND x.eleve_id = l.eleve_id
                               AND x.user_id = l.user_id AND NOT x.peut_voir_notes)
         UNION
         SELECT c.user_id, true
           FROM ent_eleve_compte c
           JOIN ent_membership m ON m.user_id = c.user_id AND m.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND m.role = 'eleve'",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *c)
    .await
    .map_err(db)?;
    let mut v: Vec<(Uuid, &'static str)> = rows
        .into_iter()
        .map(|(u, e)| (u, if e { "eleve" } else { "responsable" }))
        .collect();
    v.sort();
    v.dedup_by_key(|(u, _)| *u);
    Ok(v)
}

impl PgEnt {
    fn v14_exiger_staff(role: &str) -> Result<(), EntDbError> {
        if Role::parse(role).is_some_and(|r| r.is_staff()) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Déposer, rapprocher, écarter, consulter la file : `imports.appliquer` ou `bulletins.publier`.
    async fn v14_exiger_depot(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        Self::v14_exiger_staff(role)?;
        if self.a_action_etablissement(etab, acteur, role, Action::ImportsAppliquer).await?
            || self.a_action_etablissement(etab, acteur, role, Action::BulletinsPublier).await?
        {
            Ok(())
        } else {
            tracing::debug!("{SANS_ACCES}");
            Err(EntDbError::Forbidden)
        }
    }

    /// Distribuer, retirer, contrôler, régler le bandeau : `bulletins.publier`.
    async fn v14_exiger_publier(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        Self::v14_exiger_staff(role)?;
        if self.a_action_etablissement(etab, acteur, role, Action::BulletinsPublier).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    async fn v14_periode(&self, c: &mut PgConnection, etab: Uuid, periode: Uuid) -> Result<PeriodeV14, EntDbError> {
        let r: Option<(String, String, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT code, libelle, date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode)
        .fetch_optional(&mut *c)
        .await
        .map_err(db)?;
        let (code, libelle, debut, fin) = r.ok_or_else(introuvable)?;
        Ok(PeriodeV14 { code, libelle, debut, fin })
    }

    // ───────────────────────────── EVA-234 : dépôt et rapprochement ─────────────────────────────

    /// Ouvre un dépôt pour une période déjà commencée.
    pub async fn eva_v14_ouvrir_lot_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        b: &OuvrirLotBody,
    ) -> Result<LotTiersDto, EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let logiciel = b.logiciel.as_deref().unwrap_or("charlemagne");
        if !LOGICIELS_TIERS.contains(&logiciel) {
            return Err(refus("logiciel inconnu"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let p = self.v14_periode(&mut tx, etab, b.periode_id).await?;
        if jour < p.debut {
            return Err(conflit("la période n'a pas commencé : aucun bulletin ne peut encore être déposé"));
        }
        let libelle = match b.libelle.as_deref().map(str::trim) {
            None | Some("") => format!("Bulletins {}", p.libelle),
            Some(l) if l.chars().count() > 120 => return Err(refus("intitulé de 120 caractères au plus")),
            Some(l) => texte_borne(l, 120)?,
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO eva_tiers_lot (id, etablissement_id, periode_id, logiciel, libelle, depose_par)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.periode_id)
        .bind(logiciel)
        .bind(&libelle)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.v14_lot(etab, id).await
    }

    async fn v14_lot(&self, etab: Uuid, lot: Uuid) -> Result<LotTiersDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "SELECT l.id, l.periode_id, p.libelle AS periode_libelle, l.logiciel, l.libelle, l.depose_le,
                    count(b.id)::int AS fichiers,
                    count(b.id) FILTER (WHERE b.statut = 'a_rapprocher')::int AS a_rapprocher,
                    count(b.id) FILTER (WHERE b.statut = 'rapproche')::int AS rapproches,
                    count(b.id) FILTER (WHERE b.statut = 'distribue')::int AS distribues,
                    count(b.id) FILTER (WHERE b.statut = 'ecarte')::int AS ecartes
               FROM eva_tiers_lot l
               JOIN ent_periode p ON p.id = l.periode_id AND p.etablissement_id = l.etablissement_id
               LEFT JOIN eva_tiers_bulletin b ON b.lot_id = l.id AND b.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.id = $2
              GROUP BY l.id, p.libelle",
        )
        .bind(etab)
        .bind(lot)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        let n = |c: &str| -> Result<u32, EntDbError> { Ok(r.try_get::<i32, _>(c).map_err(db)?.max(0) as u32) };
        Ok(LotTiersDto {
            id: r.try_get("id").map_err(db)?,
            periode_id: r.try_get("periode_id").map_err(db)?,
            periode_libelle: r.try_get("periode_libelle").map_err(db)?,
            logiciel: r.try_get("logiciel").map_err(db)?,
            libelle: r.try_get("libelle").map_err(db)?,
            depose_le: r.try_get("depose_le").map_err(db)?,
            fichiers: n("fichiers")?,
            a_rapprocher: n("a_rapprocher")?,
            rapproches: n("rapproches")?,
            distribues: n("distribues")?,
            ecartes: n("ecartes")?,
        })
    }

    /// Dépôts d'une période (les plus récents d'abord).
    pub async fn eva_v14_lots_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
    ) -> Result<Vec<LotTiersDto>, EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM eva_tiers_lot WHERE etablissement_id = $1 AND periode_id = $2 ORDER BY depose_le DESC, id",
        )
        .bind(etab)
        .bind(periode)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut v = Vec::with_capacity(ids.len());
        for (id,) in ids {
            v.push(self.v14_lot(etab, id).await?);
        }
        Ok(v)
    }

    /// Un dépôt et ses fichiers.
    pub async fn eva_v14_lot_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        lot: Uuid,
    ) -> Result<(LotTiersDto, Vec<BulletinTiersDto>), EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let l = self.v14_lot(etab, lot).await?;
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES_BULLETIN} {FROM_BULLETIN}
              WHERE b.etablissement_id = $1 AND b.lot_id = $2
              ORDER BY (b.statut = 'a_rapprocher') DESC, e.nom NULLS FIRST, e.prenom, b.nom_fichier, b.id"
        ))
        .bind(etab)
        .bind(lot)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let b = rows.iter().map(dto_bulletin).collect::<Result<Vec<_>, _>>()?;
        Ok((l, b))
    }

    async fn v14_bulletin(&self, c: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<BulletinTiersDto, EntDbError> {
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_BULLETIN} {FROM_BULLETIN} WHERE b.etablissement_id = $1 AND b.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *c)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        dto_bulletin(&r)
    }

    /// Rapprochement automatique (EVA-234) : INE, puis nom + prénom + date de naissance. Un seul candidat, sinon rien.
    /// Un élève qui a déjà un bulletin « rapproché » en attente pour la période n'est pas repris (le doublon reste dans
    /// la file, à trancher par la secrétaire).
    #[allow(clippy::too_many_arguments)]
    async fn v14_rapprocher_auto(
        c: &mut PgConnection,
        etab: Uuid,
        periode: Uuid,
        p: &PeriodeV14,
        ine: Option<&str>,
        nom: Option<&str>,
        prenom: Option<&str>,
        naissance: Option<NaiveDate>,
    ) -> Result<Option<(Uuid, &'static str)>, EntDbError> {
        let mut trouve: Option<(Uuid, &'static str)> = None;
        if let Some(ine) = ine {
            let v: Vec<(Uuid, Option<String>)> = sqlx::query_as(&format!(
                "{CANDIDATS} AND upper(btrim(e.ine)) = upper(btrim($4))"
            ))
            .bind(etab)
            .bind(p.debut)
            .bind(p.fin)
            .bind(ine)
            .fetch_all(&mut *c)
            .await
            .map_err(db)?;
            if let [(id, _)] = v.as_slice() {
                trouve = Some((*id, "ine"));
            }
        }
        if trouve.is_none() {
            if let (Some(nom), Some(prenom), Some(naissance)) = (nom, prenom, naissance) {
                let v: Vec<(Uuid, Option<String>)> = sqlx::query_as(&format!(
                    "{CANDIDATS} AND e.date_naissance = $4
                       AND e.cle_recherche = ent_cle_recherche($5 || ' ' || $6)"
                ))
                .bind(etab)
                .bind(p.debut)
                .bind(p.fin)
                .bind(naissance)
                .bind(nom)
                .bind(prenom)
                .fetch_all(&mut *c)
                .await
                .map_err(db)?;
                if let [(id, ine_eleve)] = v.as_slice() {
                    // Un INE annoncé qui contredit celui de l'élève : jamais d'appariement par l'identité.
                    let contredit = matches!((ine, ine_eleve.as_deref().map(str::trim)),
                        (Some(a), Some(b)) if !b.is_empty() && !a.eq_ignore_ascii_case(b));
                    if !contredit {
                        trouve = Some((*id, "identite"));
                    }
                }
            }
        }
        if let Some((eleve, _)) = trouve {
            if Self::v14_attente_pour(c, etab, periode, eleve, None).await? {
                return Ok(None);
            }
        }
        Ok(trouve)
    }

    /// Un autre bulletin de l'élève attend déjà la distribution pour cette période ?
    async fn v14_attente_pour(
        c: &mut PgConnection,
        etab: Uuid,
        periode: Uuid,
        eleve: Uuid,
        sauf: Option<Uuid>,
    ) -> Result<bool, EntDbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM eva_tiers_bulletin
              WHERE etablissement_id = $1 AND periode_id = $2 AND eleve_id = $3 AND statut = 'rapproche'
                AND ($4::uuid IS NULL OR id <> $4)",
        )
        .bind(etab)
        .bind(periode)
        .bind(eleve)
        .bind(sauf)
        .fetch_one(&mut *c)
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    /// Dépose un bulletin PDF dans un lot et tente le rapprochement automatique.
    pub async fn eva_v14_deposer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        lot: Uuid,
        b: &DeposerBulletinTiersBody,
        contenu: &[u8],
    ) -> Result<BulletinTiersDto, EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let valide = self
            .politique_fichiers()
            .valider(UsageFichier::Document, &b.nom_fichier, Some("application/pdf"), contenu)
            .map_err(EntDbError::Fichier)?;
        if valide.type_fichier != TypeFichier::Pdf {
            return Err(refus("seul un bulletin PDF peut être déposé"));
        }
        let ine = match indice(&b.ine, 20)? {
            Some(i) => Some(i.to_ascii_uppercase()),
            None => ine_du_nom_fichier(&b.nom_fichier),
        };
        let nom = indice(&b.nom, 120)?;
        let prenom = indice(&b.prenom, 120)?;
        let nom_fichier: String = valide.nom.chars().take(255).collect();
        let sha = empreinte(contenu);
        let taille = i32::try_from(contenu.len()).map_err(|_| refus("fichier trop gros"))?;

        // Contrôles avant d'écrire le moindre objet.
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let lot_row: Option<(Uuid,)> =
            sqlx::query_as("SELECT periode_id FROM eva_tiers_lot WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(lot)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let (periode,) = lot_row.ok_or_else(introuvable)?;
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM eva_tiers_bulletin WHERE etablissement_id = $1 AND lot_id = $2")
                .bind(etab)
                .bind(lot)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if n >= MAX_FICHIERS_LOT {
            return Err(conflit("dépôt complet : ouvrez un autre dépôt pour la suite"));
        }
        let (doublon,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM eva_tiers_bulletin
                             WHERE etablissement_id = $1 AND periode_id = $2 AND empreinte = $3 AND statut <> 'ecarte')",
        )
        .bind(etab)
        .bind(periode)
        .bind(&sha)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if doublon {
            return Err(conflit("ce fichier a déjà été déposé pour cette période"));
        }
        tx.commit().await.map_err(db)?;

        let id = Uuid::new_v4();
        // La clé est produite ici, jamais fournie par le client ; elle suit le schéma des bulletins (`{etab}/bulletins/{id}.pdf`),
        // seul admis pour un document « bulletin » du coffre.
        let cle = ObjectStore::key_for(etab, "bulletins", id, "pdf");
        self.stocker_objet(etab, &cle, Some("application/pdf"), contenu).await?;

        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Verrou de la période du dépôt : deux dépôts simultanés ne rapprochent pas deux fichiers vers le même élève.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('eva_tiers:' || $1::text, 0))")
            .bind(periode)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let p = self.v14_periode(&mut tx, etab, periode).await?;
        let auto = Self::v14_rapprocher_auto(
            &mut tx,
            etab,
            periode,
            &p,
            ine.as_deref(),
            nom.as_deref(),
            prenom.as_deref(),
            b.date_naissance,
        )
        .await?;
        let (eleve, voie, statut) = match auto {
            Some((e, v)) => (Some(e), Some(v), "rapproche"),
            None => (None, None, "a_rapprocher"),
        };
        sqlx::query(
            "INSERT INTO eva_tiers_bulletin (id, etablissement_id, lot_id, periode_id, nom_fichier, storage_key, empreinte,
                                             taille, indice_ine, indice_nom, indice_prenom, indice_naissance, eleve_id,
                                             rapprochement, rapproche_par, rapproche_le, statut)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                     CASE WHEN $13::uuid IS NULL THEN NULL ELSE now() END, $16)",
        )
        .bind(id)
        .bind(etab)
        .bind(lot)
        .bind(periode)
        .bind(&nom_fichier)
        .bind(&cle)
        .bind(&sha)
        .bind(taille)
        .bind(&ine)
        .bind(&nom)
        .bind(&prenom)
        .bind(b.date_naissance)
        .bind(eleve)
        .bind(voie)
        .bind(eleve.map(|_| acteur))
        .bind(statut)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e.as_database_error().and_then(|d| d.constraint()) {
            Some("eva_tiers_bulletin_empreinte_uq") => conflit("ce fichier a déjà été déposé pour cette période"),
            _ => db(e),
        })?;
        journaliser(&mut tx, etab, id, "depose", None, None, acteur, None).await?;
        if let Some(e) = eleve {
            journaliser(&mut tx, etab, id, "rapproche", Some(e), voie, acteur, None).await?;
        }
        let dto = self.v14_bulletin(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Rapprochement manuel, ou correction d'un rapprochement automatique tant que le bulletin n'est pas distribué.
    pub async fn eva_v14_rapprocher_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        b: &RapprocherBulletinBody,
    ) -> Result<BulletinTiersDto, EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT periode_id, statut FROM eva_tiers_bulletin WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (periode, statut) = r.ok_or_else(introuvable)?;
        if !matches!(statut.as_str(), "a_rapprocher" | "rapproche") {
            return Err(conflit("ce bulletin n'est plus à rapprocher (distribué, remplacé, retiré ou écarté)"));
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('eva_tiers:' || $1::text, 0))")
            .bind(periode)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let p = self.v14_periode(&mut tx, etab, periode).await?;
        let (ok,): (bool,) = sqlx::query_as(&format!("SELECT EXISTS ({CANDIDATS} AND e.id = $4)"))
            .bind(etab)
            .bind(p.debut)
            .bind(p.fin)
            .bind(b.eleve_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !ok {
            return Err(refus("élève inconnu ou non scolarisé pendant la période"));
        }
        if Self::v14_attente_pour(&mut tx, etab, periode, b.eleve_id, Some(id)).await? {
            return Err(conflit(
                "un autre bulletin de cet élève attend déjà la distribution pour cette période : écartez l'un des deux",
            ));
        }
        sqlx::query(
            "UPDATE eva_tiers_bulletin
                SET eleve_id = $3, rapprochement = 'manuel', rapproche_par = $4, rapproche_le = now(), statut = 'rapproche'
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(b.eleve_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, id, "rapproche", Some(b.eleve_id), Some("manuel"), acteur, None).await?;
        let dto = self.v14_bulletin(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Écarte un fichier non distribué (mauvais fichier, doublon, élève parti) : il ne sera jamais distribué.
    pub async fn eva_v14_ecarter_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        b: &MotifBulletinTiersBody,
    ) -> Result<BulletinTiersDto, EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let motif = motif_valide(&b.motif)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let n = sqlx::query(
            "UPDATE eva_tiers_bulletin SET statut = 'ecarte', motif = $3
              WHERE etablissement_id = $1 AND id = $2 AND statut IN ('a_rapprocher', 'rapproche')",
        )
        .bind(etab)
        .bind(id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            // Existe-t-il ? (personnel habilité : pas d'oracle à protéger, le message aide.)
            self.v14_bulletin(&mut tx, etab, id).await?;
            return Err(conflit("un bulletin distribué se retire, il ne s'écarte pas"));
        }
        journaliser(&mut tx, etab, id, "ecarte", None, None, acteur, Some(&motif)).await?;
        let dto = self.v14_bulletin(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Aperçu du PDF pour le personnel qui rapproche ou distribue (tous statuts).
    pub async fn eva_v14_apercu_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(String, Vec<u8>), EntDbError> {
        self.v14_exiger_depot(etab, acteur, role).await?;
        let r: Option<(String, String, String)> = sqlx::query_as(
            "SELECT nom_fichier, storage_key, empreinte FROM eva_tiers_bulletin WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let (nom, cle, sha) = r.ok_or_else(introuvable)?;
        let octets = self.lire_objet_empreinte(etab, &cle, Some(&sha)).await?;
        Ok((nom, octets))
    }

    // ───────────────────────────── EVA-235 : contrôle et distribution ─────────────────────────────

    /// Contrôle avant distribution (`ecrire = false`, rien n'est écrit) ou distribution d'un dépôt (`ecrire = true`).
    /// Seuls les bulletins « rapprochés » partent ; ceux « à rapprocher » restent (CE-89).
    async fn v14_distribuer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        lot: Uuid,
        ecrire: bool,
    ) -> Result<RapportDistributionDto, EntDbError> {
        self.v14_exiger_publier(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let lot_row: Option<(Uuid,)> =
            sqlx::query_as("SELECT periode_id FROM eva_tiers_lot WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(lot)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let (periode,) = lot_row.ok_or_else(introuvable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('eva_tiers:' || $1::text, 0))")
            .bind(periode)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let p = self.v14_periode(&mut tx, etab, periode).await?;
        if jour < p.debut {
            return Err(conflit("la période n'a pas commencé : rien à distribuer"));
        }
        let a_partir: Vec<(Uuid, Uuid, String, String, String)> = sqlx::query_as(
            "SELECT b.id, b.eleve_id, b.storage_key, e.nom, e.prenom
               FROM eva_tiers_bulletin b
               JOIN ent_eleve e ON e.id = b.eleve_id AND e.etablissement_id = b.etablissement_id
              WHERE b.etablissement_id = $1 AND b.lot_id = $2 AND b.statut = 'rapproche'
              ORDER BY e.nom, e.prenom, b.id
              FOR UPDATE OF b",
        )
        .bind(etab)
        .bind(lot)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (a_rapprocher,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM eva_tiers_bulletin WHERE etablissement_id = $1 AND lot_id = $2 AND statut = 'a_rapprocher'",
        )
        .bind(etab)
        .bind(lot)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut rapport = RapportDistributionDto {
            controle: !ecrire,
            lot_id: lot,
            distribues: 0,
            remplacements: 0,
            a_rapprocher: a_rapprocher.max(0) as u32,
            destinataires: 0,
            sans_destinataire: Vec::new(),
        };
        for (id, eleve, cle, nom, prenom) in a_partir {
            let dest = destinataires(&mut tx, etab, eleve).await?;
            let ancien: Option<(Uuid, Uuid)> = sqlx::query_as(
                "SELECT id, document_id FROM eva_tiers_bulletin
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND periode_id = $3 AND statut = 'distribue'",
            )
            .bind(etab)
            .bind(eleve)
            .bind(periode)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            rapport.distribues += 1;
            rapport.destinataires += dest.len() as u32;
            if ancien.is_some() {
                rapport.remplacements += 1;
            }
            if dest.is_empty() {
                rapport.sans_destinataire.push(EleveSansDestinataireDto {
                    bulletin_id: id,
                    eleve_id: eleve,
                    nom: nom.clone(),
                    prenom: prenom.clone(),
                });
            }
            if !ecrire {
                continue;
            }
            if let Some((ancien_id, ancien_doc)) = ancien {
                // L'ancienne version reste au dossier, mais n'est plus visible des familles (comme un bulletin ScolIA).
                sqlx::query(
                    "UPDATE ent_eleve_document SET confidentialite = 'restreint' WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(ancien_doc)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                sqlx::query(
                    "UPDATE eva_tiers_bulletin SET statut = 'remplace' WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(ancien_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                journaliser(&mut tx, etab, ancien_id, "remplace", Some(eleve), None, acteur, None).await?;
            }
            let doc = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, code, mime_type,
                                                 storage_key, confidentialite, source, created_by_user_id)
                 VALUES ($1, $2, $3, 'famille', $4, $5, 'application/pdf', $6, 'standard', 'bulletin', $7)",
            )
            .bind(doc)
            .bind(etab)
            .bind(eleve)
            .bind(format!("Bulletin {}", p.libelle))
            .bind(format!("BULLETIN_TIERS_{}", p.code))
            .bind(&cle)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE eva_tiers_bulletin SET statut = 'distribue', distribue_le = now(), distribue_par = $3, document_id = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(acteur)
            .bind(doc)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            for (user, qualite) in &dest {
                sqlx::query(
                    "INSERT INTO eva_tiers_destinataire (bulletin_id, etablissement_id, user_id, qualite)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(id)
                .bind(etab)
                .bind(user)
                .bind(qualite)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            journaliser(&mut tx, etab, id, "distribue", Some(eleve), None, acteur, None).await?;
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: evenements::BULLETIN_TIERS_DISTRIBUE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve,
                eleve_id: Some(eleve),
                payload: json!({ "bulletin_id": id, "periode_id": periode, "document_id": doc }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        }
        if ecrire {
            tx.commit().await.map_err(db)?;
        }
        Ok(rapport)
    }

    pub async fn eva_v14_controler_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        lot: Uuid,
    ) -> Result<RapportDistributionDto, EntDbError> {
        self.v14_distribuer(etab, acteur, role, jour, lot, false).await
    }

    pub async fn eva_v14_distribuer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        lot: Uuid,
    ) -> Result<RapportDistributionDto, EntDbError> {
        self.v14_distribuer(etab, acteur, role, jour, lot, true).await
    }

    /// Retire un bulletin distribué (mauvais élève, version fautive) : il disparaît du coffre des familles, motif tracé.
    pub async fn eva_v14_retirer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        b: &MotifBulletinTiersBody,
    ) -> Result<BulletinTiersDto, EntDbError> {
        self.v14_exiger_publier(etab, acteur, role).await?;
        let motif = motif_valide(&b.motif)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(Uuid, Uuid)> = sqlx::query_as(
            "UPDATE eva_tiers_bulletin SET statut = 'retire', motif = $3
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'distribue'
              RETURNING eleve_id, document_id",
        )
        .bind(etab)
        .bind(id)
        .bind(&motif)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((eleve, doc)) = r else {
            self.v14_bulletin(&mut tx, etab, id).await?;
            return Err(conflit("seul un bulletin distribué se retire"));
        };
        sqlx::query("UPDATE ent_eleve_document SET confidentialite = 'restreint' WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(doc)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journaliser(&mut tx, etab, id, "retire", Some(eleve), None, acteur, Some(&motif)).await?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: evenements::BULLETIN_TIERS_RETIRE.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({ "bulletin_id": id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let dto = self.v14_bulletin(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // ───────────────────────────── Lecture famille et élève ─────────────────────────────

    /// Bulletins du logiciel tiers distribués pour un élève : famille (lien « voir les notes »), élève lui-même,
    /// ou personnel qui lit les notes. Une famille sans droit reçoit une liste vide, jamais une erreur qui dirait
    /// qu'un bulletin existe.
    pub async fn eva_v14_bulletins_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<BulletinTiersFamilleDto>, EntDbError> {
        if !self.actor_can_read_notes(etab, acteur, role, eleve).await? {
            if Role::parse(role).is_some_and(|r| r.is_staff()) {
                return Err(EntDbError::Forbidden);
            }
            return Ok(Vec::new());
        }
        let rows: Vec<(Uuid, Uuid, String, chrono::DateTime<chrono::Utc>, Uuid)> = sqlx::query_as(
            "SELECT b.id, b.periode_id, p.libelle, b.distribue_le, b.document_id
               FROM eva_tiers_bulletin b
               JOIN ent_periode p ON p.id = b.periode_id AND p.etablissement_id = b.etablissement_id
              WHERE b.etablissement_id = $1 AND b.eleve_id = $2 AND b.statut = 'distribue'
              ORDER BY p.date_debut DESC, b.distribue_le DESC",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, periode_id, periode_libelle, distribue_le, document_id)| BulletinTiersFamilleDto {
                id,
                periode_id,
                periode_libelle,
                distribue_le,
                document_id,
            })
            .collect())
    }

    /// PDF d'un bulletin distribué, pour la famille, l'élève ou le personnel qui lit les notes. Tout le reste (non
    /// distribué, remplacé, retiré, autre élève, droit absent) répond « introuvable » à une famille.
    pub async fn eva_v14_pdf_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(String, Vec<u8>), EntDbError> {
        let r: Option<(Option<Uuid>, String, String, String, String)> = sqlx::query_as(
            "SELECT b.eleve_id, b.statut, b.storage_key, b.empreinte, p.code
               FROM eva_tiers_bulletin b
               JOIN ent_periode p ON p.id = b.periode_id AND p.etablissement_id = b.etablissement_id
              WHERE b.etablissement_id = $1 AND b.id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let staff = Role::parse(role).is_some_and(|r| r.is_staff());
        let Some((Some(eleve), statut, cle, sha, code)) = r else {
            return Err(introuvable());
        };
        if statut != "distribue" || !self.actor_can_read_notes(etab, acteur, role, eleve).await? {
            return Err(if staff && statut == "distribue" { EntDbError::Forbidden } else { introuvable() });
        }
        let octets = self.lire_objet_empreinte(etab, &cle, Some(&sha)).await?;
        let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        Ok((format!("bulletin-{code}.pdf"), octets))
    }

    // ───────────────────────────── EVA-237 : bandeau « où sont les notes » ─────────────────────────────

    /// Bandeau de l'établissement (tout membre ; `None` : aucun réglage).
    pub async fn eva_v14_cohabitation(&self, etab: Uuid) -> Result<Option<CohabitationDto>, EntDbError> {
        let r: Option<(bool, String, Option<String>, String)> = sqlx::query_as(
            "SELECT actif, logiciel_libelle, url, message FROM eva_cohabitation WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(r.map(|(actif, logiciel_libelle, url, message)| CohabitationDto { actif, logiciel_libelle, url, message }))
    }

    pub async fn eva_v14_regler_cohabitation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        b: &ReglerCohabitationBody,
    ) -> Result<CohabitationDto, EntDbError> {
        self.v14_exiger_publier(etab, acteur, role).await?;
        let logiciel = b.logiciel_libelle.trim();
        if !(1..=80).contains(&logiciel.chars().count()) {
            return Err(refus("nom du logiciel de 1 à 80 caractères"));
        }
        let logiciel = texte_borne(logiciel, 80)?;
        let message = b.message.trim();
        if !(1..=600).contains(&message.chars().count()) {
            return Err(refus("message de 1 à 600 caractères"));
        }
        let message = texte_borne(message, 600)?;
        let url = match b.url.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(u) => {
                let ok = u.len() <= 500
                    && u.starts_with("https://")
                    && u.len() > "https://".len()
                    && !u.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\''));
                if !ok {
                    return Err(refus("lien en https:// seulement, sans espace ni guillemet"));
                }
                Some(u.to_string())
            }
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO eva_cohabitation (etablissement_id, actif, logiciel_libelle, url, message, modifie_par)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET actif = EXCLUDED.actif, logiciel_libelle = EXCLUDED.logiciel_libelle, url = EXCLUDED.url,
                    message = EXCLUDED.message, modifie_par = EXCLUDED.modifie_par, modifie_le = now()",
        )
        .bind(etab)
        .bind(b.actif)
        .bind(&logiciel)
        .bind(&url)
        .bind(&message)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(CohabitationDto { actif: b.actif, logiciel_libelle: logiciel, url, message })
    }

    // ───────────────────────────── EVA-233 : export de l'assiduité ─────────────────────────────

    /// Assiduité d'une classe sur une période, par élève, pour le bulletin du logiciel tiers. Bornée à la période et à
    /// `jour` (rien de futur), effectif de la classe de rattachement (V5, D15) ; chaque export est journalisé.
    /// Rend aussi le libellé de la classe (pour le CSV).
    pub async fn eva_v14_exporter_assiduite_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<(ExportAssiduiteTiersDto, String), EntDbError> {
        Self::v14_exiger_staff(role)?;
        if !self.a_action_etablissement(etab, acteur, role, Action::ExportsOfficiels).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let p = self.v14_periode(&mut tx, etab, periode).await?;
        let cl: Option<(String,)> = sqlx::query_as(
            "SELECT COALESCE(NULLIF(btrim(libelle), ''), code) FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(classe)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (classe_libelle,) = cl.ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        if jour < p.debut {
            return Err(conflit("la période n'a pas commencé : aucune assiduité à exporter"));
        }
        let au = p.fin.min(jour);
        let du = p.debut;
        let effectif = self.eva_effectif(etab, Some(classe), None, du, au).await?;
        let mut lignes = Vec::with_capacity(effectif.len());
        for (eleve, nom, prenom) in effectif {
            let dj = self.calculer_demi_journees(etab, Some(eleve), du, au).await?;
            let comptees = dj.iter().filter(|d| d.compte).count() as u32;
            let nj = dj.iter().filter(|d| d.compte && d.non_justifiee).count() as u32;
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let (ine, naissance): (Option<String>, Option<NaiveDate>) =
                sqlx::query_as("SELECT ine, date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(eleve)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
            let (retards,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_appel_ligne l
                   JOIN ent_appel a ON a.id = l.appel_id AND a.etablissement_id = l.etablissement_id
                  WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.etat = 'retard'
                    AND a.jour BETWEEN $3 AND $4",
            )
            .bind(etab)
            .bind(eleve)
            .bind(du)
            .bind(au)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            lignes.push(LigneAssiduiteTiersDto {
                eleve_id: eleve,
                ine: ine.filter(|s| !s.trim().is_empty()),
                nom,
                prenom,
                date_naissance: naissance,
                demi_journees: comptees,
                justifiees: comptees - nj,
                non_justifiees: nj,
                retards: retards.max(0) as u32,
            });
        }
        let e = ExportAssiduiteTiersDto {
            export_id: Uuid::new_v4(),
            periode_id: periode,
            classe_id: classe,
            du,
            au,
            partiel: au < p.fin,
            lignes,
        };
        let csv = csv_assiduite_tiers(&e, &classe_libelle);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO eva_tiers_export (id, etablissement_id, periode_id, classe_id, du, au, eleves, empreinte, exporte_par)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(e.export_id)
        .bind(etab)
        .bind(periode)
        .bind(classe)
        .bind(du)
        .bind(au)
        .bind(e.lignes.len() as i32)
        .bind(empreinte(csv.as_bytes()))
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: evenements::EXPORT_ASSIDUITE_TIERS.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: periode,
            eleve_id: None,
            payload: json!({ "export_id": e.export_id, "format": "assiduite_tiers", "classe_id": classe,
                             "nb_eleves": e.lignes.len() }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok((e, classe_libelle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ine_plausibles_et_lus_dans_le_nom() {
        assert!(ine_plausible("123456789AB"));
        assert!(ine_plausible("1234567890A"));
        assert!(!ine_plausible("12345678AB"));
        assert!(!ine_plausible("123456789ab"));
        assert!(!ine_plausible("ABCDEFGHIJK"));
        assert_eq!(ine_du_nom_fichier("2NDE_A_123456789ab_DUPONT.pdf").as_deref(), Some("123456789AB"));
        assert_eq!(ine_du_nom_fichier("C:\\depot\\1234567890A.pdf").as_deref(), Some("1234567890A"));
        // Deux INE ou aucun : rien n'est deviné.
        assert_eq!(ine_du_nom_fichier("123456789AB_123456789CD.pdf"), None);
        assert_eq!(ine_du_nom_fichier("DUPONT Jean.pdf"), None);
    }

    #[test]
    fn csv_echappe_les_formules_et_porte_les_bornes() {
        let e = ExportAssiduiteTiersDto {
            export_id: Uuid::nil(),
            periode_id: Uuid::nil(),
            classe_id: Uuid::nil(),
            du: NaiveDate::from_ymd_opt(2031, 9, 1).unwrap(),
            au: NaiveDate::from_ymd_opt(2031, 11, 30).unwrap(),
            partiel: true,
            lignes: vec![LigneAssiduiteTiersDto {
                eleve_id: Uuid::nil(),
                ine: None,
                nom: "=HYPERLINK(\"x\")".into(),
                prenom: "Léa; Marie".into(),
                date_naissance: NaiveDate::from_ymd_opt(2015, 2, 3),
                demi_journees: 4,
                justifiees: 3,
                non_justifiees: 1,
                retards: 2,
            }],
        };
        let csv = csv_assiduite_tiers(&e, "2de A");
        assert!(csv.starts_with('\u{feff}'));
        let ligne = csv.lines().nth(1).unwrap();
        assert!(ligne.contains("\"'=HYPERLINK(\"\"x\"\")\""), "{ligne}");
        assert!(ligne.contains("\"Léa; Marie\""), "{ligne}");
        assert!(ligne.ends_with("03/02/2015;2de A;01/09/2031;30/11/2031;4;3;1;2"), "{ligne}");
    }
}
