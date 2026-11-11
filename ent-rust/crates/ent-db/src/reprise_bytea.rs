//! Reprise des pièces historiques stockées en `BYTEA` vers le magasin d'objets
//! (PRA_STOCKAGE_OBJETS §5, étape 2 ; exigence E5).
//!
//! Tables concernées : `ent_msg_piece` (messagerie) et `ent_pai_piece` (PAI : **données de
//! santé**). Depuis la migration 601, plus aucun chemin applicatif n'écrit `contenu` ; il ne
//! reste d'octets en base que pour l'historique, que cet outil fait sortir.
//!
//! Principes :
//! * **Idempotente et reprenable.** La sélection est « `contenu` encore rempli » : une pièce
//!   déjà reprise n'est plus candidate ; une exécution interrompue (panne, arrêt volontaire,
//!   `limite_pieces`) reprend exactement où elle en était, sans doublon ni perte.
//! * **Par établissement**, lots par curseur d'identifiant, une transaction **par pièce**
//!   (la ligne est verrouillée `FOR UPDATE` pendant sa copie : aucune écriture concurrente).
//! * **`contenu` n'est jamais vidé avant vérification** : écriture immuable de l'objet,
//!   **relecture depuis le stockage** et comparaison du SHA-256 à celui des octets de la base,
//!   enregistrement dans `ent_objet`, puis seulement vidage — le tout dans la même transaction.
//!   Au moindre échec la ligne est laissée intacte (la transaction n'est pas validée).
//! * **Vidage conditionné** (`vider`) : la copie vérifiée et le vidage sont deux décisions ;
//!   le plan (§5) exige un point de sauvegarde vérifié avant de vider. Sans `vider`, la reprise
//!   copie et référence seulement ; une seconde exécution (avec `vider`) revérifie puis vide.
//! * **Simulation** : lit et contrôle tout, n'écrit rien (ni objet, ni registre, ni ligne).
//! * **E14** : clé d'objet opaque `{etab}/{messages|pai}/{uuid}.{ext}` (jamais le nom du
//!   fichier) ; ni nom, ni clé, ni identifiant dans les journaux ni dans les causes d'échec.

use ent_object::{sha256_hex, ObjectStoreError};
use sqlx::Row;
use uuid::Uuid;

use crate::objets::{cle_reprise, enregistrer_objet};
use crate::service::{object_key_is_owned, EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Table à reprendre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TablePiece {
    /// `ent_msg_piece` (pièces jointes de messagerie).
    Messagerie,
    /// `ent_pai_piece` (ordonnances et pièces de PAI — santé).
    Pai,
}

impl TablePiece {
    pub const TOUTES: [TablePiece; 2] = [TablePiece::Messagerie, TablePiece::Pai];

    pub fn table(self) -> &'static str {
        match self {
            Self::Messagerie => "ent_msg_piece",
            Self::Pai => "ent_pai_piece",
        }
    }

    /// Espace de la clé d'objet (identique à celui des écritures nouvelles).
    pub fn espace(self) -> &'static str {
        match self {
            Self::Messagerie => "messages",
            Self::Pai => "pai",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Messagerie => "messagerie",
            Self::Pai => "pai",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "messagerie" | "messages" => Some(Self::Messagerie),
            "pai" => Some(Self::Pai),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionsReprise {
    /// Ne rien écrire (ni objet, ni registre, ni ligne) ; contrôle tout le reste.
    pub simulation: bool,
    /// Vider `contenu` après vérification. À n'activer qu'après un point de sauvegarde vérifié.
    pub vider: bool,
    /// Taille d'un lot de sélection (curseur d'identifiant). Minimum 1.
    pub taille_lot: i64,
    /// Arrêt propre après ce nombre de pièces examinées (budget de maintenance, tests
    /// d'interruption). `None` : tout.
    pub limite_pieces: Option<u64>,
}

impl Default for OptionsReprise {
    fn default() -> Self {
        Self {
            simulation: false,
            vider: false,
            taille_lot: 50,
            limite_pieces: None,
        }
    }
}

/// Cause d'échec : code stable, **sans** donnée de la pièce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CauseEchec {
    /// Le magasin d'objets a refusé ou n'a pas répondu (écriture ou relecture).
    StockageIndisponible,
    /// Un objet existe déjà sous cette clé avec un contenu différent (immuable : refusé).
    ObjetDejaPresentDifferent,
    /// L'objet relu depuis le stockage n'a pas l'empreinte des octets de la base.
    RelectureDifferente,
    /// `taille_octets` ne correspond pas aux octets de la base : ligne laissée intacte.
    TailleIncoherente,
    /// La ligne ou le registre `ent_objet` annonce une autre empreinte que les octets de la base.
    RegistreIncoherent,
    /// Erreur de base de données.
    Base,
}

impl CauseEchec {
    pub fn code(self) -> &'static str {
        match self {
            Self::StockageIndisponible => "stockage_indisponible",
            Self::ObjetDejaPresentDifferent => "objet_deja_present_different",
            Self::RelectureDifferente => "relecture_differente",
            Self::TailleIncoherente => "taille_incoherente",
            Self::RegistreIncoherent => "registre_incoherent",
            Self::Base => "base",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchecReprise {
    pub piece_id: Uuid,
    pub cause: CauseEchec,
}

/// Rapport d'une reprise : une table, un établissement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RapportReprise {
    pub table: TablePiece,
    pub etablissement_id: Uuid,
    pub simulation: bool,
    pub vider: bool,
    /// Pièces déjà sorties de la base avant cette exécution (objet référencé, `contenu` vide).
    pub deja_reprises: u64,
    /// Pièces examinées par cette exécution (octets encore en base au moment du passage).
    pub examinees: u64,
    /// Copiées et vérifiées (en simulation : qui le seraient).
    pub copiees: u64,
    /// Déjà copiées par une exécution précédente sans vidage, revérifiées.
    pub reverifiees: u64,
    /// `contenu` vidé après vérification (jamais en simulation).
    pub videes: u64,
    pub octets_copies: u64,
    pub en_echec: Vec<EchecReprise>,
    /// Pièces dont les octets sont encore en base à la fin.
    pub restantes: u64,
    /// Arrêt volontaire (`limite_pieces`) avant la fin de l'établissement.
    pub interrompue: bool,
}

impl RapportReprise {
    fn nouveau(table: TablePiece, etablissement_id: Uuid, o: &OptionsReprise) -> Self {
        Self {
            table,
            etablissement_id,
            simulation: o.simulation,
            vider: o.vider,
            deja_reprises: 0,
            examinees: 0,
            copiees: 0,
            reverifiees: 0,
            videes: 0,
            octets_copies: 0,
            en_echec: Vec::new(),
            restantes: 0,
            interrompue: false,
        }
    }

    /// Rapport lisible, sans nom de fichier ni clé ; identifiants de pièces en échec seulement.
    pub fn resume(&self) -> String {
        let mut s = format!(
            "{} · établissement {} · {} : déjà repris {}, examinés {}, copiés {}, revérifiés {}, vidés {}, \
             en échec {}, restants {}{}",
            self.table.code(),
            self.etablissement_id,
            if self.simulation { "SIMULATION" } else { "réel" },
            self.deja_reprises,
            self.examinees,
            self.copiees,
            self.reverifiees,
            self.videes,
            self.en_echec.len(),
            self.restantes,
            if self.interrompue { " (interrompu)" } else { "" },
        );
        for e in &self.en_echec {
            s.push_str(&format!("\n  - pièce {} : {}", e.piece_id, e.cause.code()));
        }
        s
    }
}

enum Issue {
    /// Plus d'octets en base au moment du passage (reprise concurrente) : rien à faire.
    DejaRepris,
    Copiee { octets: u64, videe: bool },
    Reverifiee { octets: u64, videe: bool },
}

impl PgEnt {
    /// Établissements à parcourir : ceux du registre `ent_etablissement` **et** ceux qui ont
    /// encore des octets en base (un établissement de labo ou historique peut ne pas être au
    /// registre). La reprise est menée un établissement après l'autre. Sous RLS stricte sans
    /// établissement posé, seule la première source est visible : passer alors
    /// `--etablissement`.
    pub async fn etablissements_pour_reprise(&self) -> Result<Vec<Uuid>, EntDbError> {
        // Contexte système nommé (docs/securite/RLS_STRICTE.md §4) : racine des établissements et
        // établissements portant des adhésions (tables sans RLS) ; la reprise ouvre ensuite un
        // `TenantTx` par établissement. Les pièces sous RLS ne sont plus lues hors contexte.
        Ok(crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("reprise_bytea"),
        )
        .await?)
    }

    /// Reprise d'une table pour **un** établissement (lots, transaction par pièce).
    /// Idempotente : relancer après interruption ou échec ne refait que le reste.
    pub async fn reprendre_pieces_etablissement(
        &self,
        table: TablePiece,
        etablissement_id: Uuid,
        options: &OptionsReprise,
    ) -> Result<RapportReprise, EntDbError> {
        let mut rapport = RapportReprise::nouveau(table, etablissement_id, options);
        let lot = options.taille_lot.max(1);
        let t = table.table();

        rapport.deja_reprises = self
            .compter_pieces(
                table,
                etablissement_id,
                "storage_key IS NOT NULL AND (contenu IS NULL OR octet_length(contenu) = 0)",
            )
            .await?;

        let mut curseur = Uuid::nil();
        'lots: loop {
            let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
            let ids: Vec<Uuid> = sqlx::query_scalar(&format!(
                "SELECT id FROM {t}
                 WHERE etablissement_id = $1 AND id > $2
                   AND contenu IS NOT NULL AND octet_length(contenu) > 0
                 ORDER BY id LIMIT $3"
            ))
            .bind(etablissement_id)
            .bind(curseur)
            .bind(lot)
            .fetch_all(&mut *tx)
            .await?;
            tx.commit().await?;
            if ids.is_empty() {
                break;
            }
            for id in ids {
                if options
                    .limite_pieces
                    .is_some_and(|max| rapport.examinees >= max)
                {
                    rapport.interrompue = true;
                    break 'lots;
                }
                curseur = id;
                rapport.examinees += 1;
                match self.reprendre_une_piece(table, etablissement_id, id, options).await {
                    Ok(Issue::DejaRepris) => rapport.examinees -= 1,
                    Ok(Issue::Copiee { octets, videe }) => {
                        rapport.copiees += 1;
                        rapport.octets_copies += octets;
                        rapport.videes += u64::from(videe);
                    }
                    Ok(Issue::Reverifiee { octets, videe }) => {
                        rapport.reverifiees += 1;
                        rapport.octets_copies += octets;
                        rapport.videes += u64::from(videe);
                    }
                    Err(cause) => rapport.en_echec.push(EchecReprise {
                        piece_id: id,
                        cause,
                    }),
                }
            }
        }

        rapport.restantes = self
            .compter_pieces(
                table,
                etablissement_id,
                "contenu IS NOT NULL AND octet_length(contenu) > 0",
            )
            .await?;
        // Journal : décomptes seulement (E14, santé) — ni nom, ni clé, ni identifiant.
        tracing::info!(
            table = table.code(),
            simulation = rapport.simulation,
            examinees = rapport.examinees,
            copiees = rapport.copiees,
            reverifiees = rapport.reverifiees,
            videes = rapport.videes,
            en_echec = rapport.en_echec.len(),
            restantes = rapport.restantes,
            "reprise des pièces BYTEA : lot terminé"
        );
        Ok(rapport)
    }

    /// Reprise de toutes les tables demandées, établissement par établissement. `limite_pieces`
    /// est un budget **global**.
    pub async fn reprendre_pieces(
        &self,
        tables: &[TablePiece],
        etablissement: Option<Uuid>,
        options: &OptionsReprise,
    ) -> Result<Vec<RapportReprise>, EntDbError> {
        let etabs = match etablissement {
            Some(e) => vec![e],
            None => self.etablissements_pour_reprise().await?,
        };
        let mut options = options.clone();
        let mut rapports = Vec::new();
        for etab in etabs {
            for &table in tables {
                let r = self
                    .reprendre_pieces_etablissement(table, etab, &options)
                    .await?;
                if let Some(max) = options.limite_pieces.as_mut() {
                    *max = max.saturating_sub(r.examinees);
                }
                let fin = r.interrompue;
                rapports.push(r);
                if fin {
                    return Ok(rapports);
                }
            }
        }
        Ok(rapports)
    }

    async fn compter_pieces(
        &self,
        table: TablePiece,
        etablissement_id: Uuid,
        condition: &str,
    ) -> Result<u64, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let n: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {} WHERE etablissement_id = $1 AND {condition}",
            table.table()
        ))
        .bind(etablissement_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(n.max(0) as u64)
    }

    /// Une pièce, une transaction. Tout échec → la transaction est abandonnée : la ligne n'a
    /// pas bougé (`contenu` intact).
    async fn reprendre_une_piece(
        &self,
        table: TablePiece,
        etab: Uuid,
        id: Uuid,
        o: &OptionsReprise,
    ) -> Result<Issue, CauseEchec> {
        let t = table.table();
        let base = |_: sqlx::Error| CauseEchec::Base;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(base)?;
        let row = sqlx::query(&format!(
            "SELECT nom, taille_octets, contenu, storage_key, sha256
             FROM {t} WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(base)?;
        let Some(row) = row else {
            return Ok(Issue::DejaRepris);
        };
        let contenu: Option<Vec<u8>> = row.get("contenu");
        let contenu = contenu.unwrap_or_default();
        if contenu.is_empty() {
            return Ok(Issue::DejaRepris);
        }
        let nom: String = row.get("nom");
        let taille: i32 = row.get("taille_octets");
        let cle_ligne: Option<String> = row.get("storage_key");
        let sha_ligne: Option<String> = row.get("sha256");

        if i64::from(taille) != contenu.len() as i64 {
            return Err(CauseEchec::TailleIncoherente);
        }
        let sha = sha256_hex(&contenu);
        if sha_ligne.as_deref().is_some_and(|s| s != sha) {
            return Err(CauseEchec::RegistreIncoherent);
        }
        let deja_reference = cle_ligne.is_some() && sha_ligne.is_some();
        let cle = match cle_ligne {
            Some(k) if object_key_is_owned(etab, table.espace(), id, &k) => k,
            _ => cle_reprise(etab, table.espace(), id, &nom),
        };
        let octets = contenu.len() as u64;
        let issue = |videe: bool| {
            if deja_reference {
                Issue::Reverifiee { octets, videe }
            } else {
                Issue::Copiee { octets, videe }
            }
        };

        let store = self.objects();
        if o.simulation {
            // Aucune écriture : on contrôle seulement qu'un objet déjà présent serait compatible.
            match store.existe(&cle) {
                Ok(true) => {
                    let relu = store.get(&cle).map_err(cause_stockage)?;
                    if sha256_hex(&relu) != sha {
                        return Err(CauseEchec::ObjetDejaPresentDifferent);
                    }
                }
                Ok(false) => {}
                Err(e) => return Err(cause_stockage(e)),
            }
            return Ok(issue(false));
        }

        // 1. Écriture immuable (idempotente à contenu identique) puis 2. relecture depuis le
        //    stockage, comparée à l'empreinte des octets de la base.
        let info = store.put(&cle, &contenu).map_err(cause_stockage)?;
        let relu = store.get(&cle).map_err(cause_stockage)?;
        if info.sha256 != sha || sha256_hex(&relu) != sha || relu.len() as u64 != octets {
            return Err(CauseEchec::RelectureDifferente);
        }

        // 3. Registre : une ligne existante avec une autre empreinte est une incohérence.
        let existant: Option<String> = sqlx::query_scalar(
            "SELECT sha256 FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2",
        )
        .bind(etab)
        .bind(&cle)
        .fetch_optional(&mut *tx)
        .await
        .map_err(base)?;
        if existant.as_deref().is_some_and(|s| s != sha) {
            return Err(CauseEchec::RegistreIncoherent);
        }
        let mime: String = sqlx::query_scalar(&format!(
            "SELECT mime_type FROM {t} WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(base)?;
        enregistrer_objet(&mut *tx, etab, &info, Some(&mime))
            .await
            .map_err(|_| CauseEchec::Base)?;

        // 4. Référence posée ; `contenu` vidé seulement si demandé, après tout ce qui précède.
        sqlx::query(&format!(
            "UPDATE {t}
             SET storage_key = $3, sha256 = $4, repris_le = COALESCE(repris_le, now()),
                 contenu = CASE WHEN $5 THEN '\\x'::bytea ELSE contenu END
             WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .bind(&cle)
        .bind(&sha)
        .bind(o.vider)
        .execute(&mut *tx)
        .await
        .map_err(base)?;
        tx.commit().await.map_err(base)?;
        Ok(issue(o.vider))
    }
}

fn cause_stockage(e: ObjectStoreError) -> CauseEchec {
    match e {
        ObjectStoreError::Immutable => CauseEchec::ObjetDejaPresentDifferent,
        ObjectStoreError::Integrity => CauseEchec::RelectureDifferente,
        _ => CauseEchec::StockageIndisponible,
    }
}
