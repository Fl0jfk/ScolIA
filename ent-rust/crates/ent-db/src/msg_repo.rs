use chrono::{DateTime, Utc};
use domain_eleve::{
    DomainError, Message, MsgAccuse, MsgFil, MsgModele, MsgPiece, MsgPieceMeta, MsgPriorite,
};
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

/// Prédicat de lisibilité d'un fil pour un personnel (alias `f`) ; paramètres `$3` = compte,
/// `$4` = `reserve_ok`, `$5` = `foyer_ok` (voir [`PgMsgRepo::fils_lisibles_personnel`]).
const PREDICAT_LISIBLE_PERSONNEL: &str = "(
       (f.foyer_id IS NULL AND f.vie_scolaire)
    OR (f.foyer_id IS NOT NULL AND $5)
    OR (f.foyer_id IS NULL AND NOT f.vie_scolaire AND $4)
    OR f.created_by_user_id = $3
    OR EXISTS (SELECT 1 FROM ent_msg m
                WHERE m.etablissement_id = f.etablissement_id
                  AND m.fil_id = f.id AND m.auteur_user_id = $3))";

/// Échappe `\`, `%` et `_` d'une recherche libre pour `ILIKE ... ESCAPE '\'`.
fn echapper_like(r: &str) -> String {
    r.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Curseur de pagination de la boîte : position du dernier fil rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurseurFils {
    /// 0 = priorité haute, 1 = normale.
    pub rang: i16,
    pub dernier_message: DateTime<Utc>,
    pub fil_id: Uuid,
}

impl CurseurFils {
    pub fn de_fil(f: &MsgFil) -> Self {
        Self {
            rang: f.priorite.rang(),
            dernier_message: f.last_message_at,
            fil_id: f.id,
        }
    }

    /// Forme texte opaque pour l'API : `rang|horodatage|id`.
    pub fn encoder(&self) -> String {
        format!(
            "{}|{}|{}",
            self.rang,
            self.dernier_message.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            self.fil_id
        )
    }

    pub fn decoder(s: &str) -> Option<Self> {
        let mut p = s.split('|');
        let rang: i16 = p.next()?.parse().ok()?;
        let dernier_message = DateTime::parse_from_rfc3339(p.next()?).ok()?.with_timezone(&Utc);
        let fil_id = Uuid::parse_str(p.next()?).ok()?;
        if p.next().is_some() || !(0..=1).contains(&rang) {
            return None;
        }
        Some(Self { rang, dernier_message, fil_id })
    }
}

/// Filtres d'une page de la boîte d'un personnel.
#[derive(Debug, Clone, Default)]
pub struct FiltreFils<'a> {
    pub eleve_id: Option<Uuid>,
    /// Périmètre d'un professeur (`None` = tout l'établissement).
    pub eleves: Option<&'a [Uuid]>,
    pub avec_archives: bool,
    pub dossier: Option<&'a str>,
    pub recherche: Option<&'a str>,
    pub apres: Option<CurseurFils>,
    pub limite: i64,
}

/// Fil en attente de réponse du personnel (MSG-329).
#[derive(Debug, Clone)]
pub struct FilNonTraite {
    pub fil: MsgFil,
    pub en_attente_depuis: DateTime<Utc>,
}

pub struct PgMsgRepo {
    pool: PgPool,
}

/// Portée de lecture d'un fil à sa création (M0a, D3 et D5).
#[derive(Debug, Clone, Copy)]
pub struct PorteeFil {
    pub prive_user_id: Option<Uuid>,
    pub foyer_id: Option<Uuid>,
    /// `responsables` (défaut), `famille` ou `eleve`.
    pub audience: &'static str,
    pub vie_scolaire: bool,
    /// M0b (D22) : foyer lecteur d'un fil du personnel « aux responsables » (un fil par foyer).
    /// Distinct de `foyer_id` (payeurs d'une relance).
    pub foyer_lecteur_id: Option<Uuid>,
}

impl PorteeFil {
    /// Fil de l'établissement aux responsables, invisible de l'élève.
    pub const fn responsables() -> Self {
        Self {
            prive_user_id: None,
            foyer_id: None,
            audience: "responsables",
            vie_scolaire: false,
            foyer_lecteur_id: None,
        }
    }

    /// Fil du personnel aux responsables d'**un** foyer (D22) : lu des seuls liens communicants
    /// de ce foyer, jamais de l'autre foyer.
    pub const fn responsables_du_foyer(foyer_lecteur_id: Uuid) -> Self {
        Self { foyer_lecteur_id: Some(foyer_lecteur_id), ..Self::responsables() }
    }

    /// Fil du personnel à l'élève majeur (D23) : l'élève le lit, jamais un parent.
    pub const fn eleve_majeur() -> Self {
        Self { audience: "eleve", ..Self::responsables() }
    }

    /// Fil de vie scolaire (carnet, alerte d'absences) aux responsables ayant le droit.
    pub const fn vie_scolaire() -> Self {
        Self { vie_scolaire: true, ..Self::responsables() }
    }

    /// Fil de relance de facture, réservé aux payeurs du foyer.
    pub const fn foyer(foyer_id: Uuid) -> Self {
        Self { foyer_id: Some(foyer_id), ..Self::responsables() }
    }

    /// Fil ouvert par un responsable : privé entre lui et l'établissement.
    pub const fn prive_parent(user_id: Uuid) -> Self {
        Self { prive_user_id: Some(user_id), ..Self::responsables() }
    }

    /// Fil ouvert par un élève : privé à l'élève et au personnel, jamais un parent.
    pub const fn eleve(user_id: Uuid) -> Self {
        Self { prive_user_id: Some(user_id), audience: "eleve", ..Self::responsables() }
    }
}

impl PgMsgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insère un fil avec sa portée de lecture (D-014, RP1-11 ; M0a D3 et D5) :
    /// `prive_user_id` = fil privé entre ce compte et l'établissement ; `foyer_id` = fil réservé
    /// aux payeurs de ce foyer (relance de facture) ; `audience` et `vie_scolaire` : voir
    /// [`PorteeFil`].
    pub async fn insert_fil_portee(
        &self,
        fil: &MsgFil,
        portee: PorteeFil,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, fil.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg_fil (
                id, etablissement_id, eleve_id, sujet, created_by_user_id,
                created_at, last_message_at, archive, modele_id, priorite, dossier,
                prive_user_id, foyer_id, audience, vie_scolaire, foyer_lecteur_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
            "#,
        )
        .bind(fil.id)
        .bind(fil.etablissement_id)
        .bind(fil.eleve_id)
        .bind(&fil.sujet)
        .bind(fil.created_by_user_id)
        .bind(fil.created_at)
        .bind(fil.last_message_at)
        .bind(fil.archive)
        .bind(fil.modele_id)
        .bind(fil.priorite.as_str())
        .bind(fil.dossier.as_deref())
        .bind(portee.prive_user_id)
        .bind(portee.foyer_id)
        .bind(portee.audience)
        .bind(portee.vie_scolaire)
        .bind(portee.foyer_lecteur_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    /// Parmi `fil_ids`, ceux qu'un compte **famille** peut lire (D-014, RP1-11 ; M0a D1, D3, D5) :
    /// - jamais le fil privé d'un autre compte ;
    /// - un fil de relance (`foyer_id`) seulement pour les payeurs actifs de ce foyer, jamais
    ///   pour un élève ;
    /// - un parent : lien actif **non visé par une interdiction de communication** ; un fil de
    ///   vie scolaire exige `peut_voir_vs` et l'autorité parentale ; jamais un fil `eleve` ;
    /// - un élève : seulement les fils d'audience `eleve` ou `famille`.
    pub async fn fils_lisibles_famille(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        est_parent: bool,
        fil_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT f.id
              FROM ent_msg_fil f
             WHERE f.etablissement_id = $1 AND f.id = ANY($2)
               AND (f.prive_user_id IS NULL OR f.prive_user_id = $3)
               -- M0b (D22) : un fil du personnel à un foyer n'est lu que de ce foyer.
               AND (f.foyer_lecteur_id IS NULL OR NOT $4
                    OR EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                                WHERE l.etablissement_id = f.etablissement_id
                                  AND l.eleve_id = f.eleve_id AND l.user_id = $3
                                  AND l.foyer_id = f.foyer_lecteur_id))
               -- M0b (D24) : un fil dont aucun message n'est encore remis n'existe pas pour la
               -- famille (sauf pour son propre auteur).
               AND (NOT EXISTS (SELECT 1 FROM ent_msg m
                                 WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id)
                    OR EXISTS (SELECT 1 FROM ent_msg m
                                WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id
                                  AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now()
                                       OR m.auteur_user_id = $3)))
               AND (f.foyer_id IS NULL
                    OR EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                                WHERE l.etablissement_id = f.etablissement_id
                                  AND l.eleve_id = f.eleve_id AND l.user_id = $3
                                  AND l.foyer_id = f.foyer_id AND l.est_payeur))
               AND CASE WHEN $4 THEN
                        f.audience IN ('famille', 'responsables')
                        AND EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                                     WHERE l.etablissement_id = f.etablissement_id
                                       AND l.eleve_id = f.eleve_id AND l.user_id = $3
                                       AND (NOT f.vie_scolaire
                                            OR (l.peut_voir_vs AND l.autorite_parentale)))
                    ELSE
                        f.audience IN ('famille', 'eleve') AND f.foyer_id IS NULL
                    END
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_ids)
        .bind(user_id)
        .bind(est_parent)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Parmi `fil_ids`, ceux qu'un **personnel** peut lire (M0a D2, M0b D21 et D22).
    ///
    /// Règle commune ([`PREDICAT_LISIBLE_PERSONNEL`]) :
    /// - un fil de vie scolaire (carnet, alerte d'absences) reste lisible du personnel suivant
    ///   l'élève ;
    /// - un fil de relance (`foyer_id`) : `foyer_ok` (direction, secrétariat, comptabilité) ;
    /// - tout autre fil (privé d'un parent, d'un élève, du personnel aux responsables d'un foyer
    ///   ou à un élève majeur) : `reserve_ok` (droit `messagerie.fils_reserves.lire`) ;
    /// - dans tous les cas, son auteur et tout personnel qui y a écrit.
    ///
    /// Le lien d'enseignement seul (`messagerie.eleve` d'un professeur) ne suffit plus : il ouvre
    /// l'écriture d'un nouveau fil, pas la lecture de ceux des autres.
    pub async fn fils_lisibles_personnel(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        reserve_ok: bool,
        foyer_ok: bool,
        fil_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT f.id FROM ent_msg_fil f
              WHERE f.etablissement_id = $1 AND f.id = ANY($2) AND {PREDICAT_LISIBLE_PERSONNEL}"
        );
        let rows: Vec<(Uuid,)> = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(fil_ids)
            .bind(user_id)
            .bind(reserve_ok)
            .bind(foyer_ok)
            .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Une page de la boîte d'un personnel (M0b, D12) : filtrage de lisibilité **dans la
    /// requête** (une page ne se vide pas après coup), recherche sur le sujet, curseur stable
    /// `(priorité, dernier message, id)`. Retourne au plus `filtre.limite + 1` fils : le
    /// dernier, s'il est présent, signale une page suivante.
    pub async fn page_fils_personnel(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        reserve_ok: bool,
        foyer_ok: bool,
        filtre: &FiltreFils<'_>,
    ) -> Result<Vec<MsgFil>, DomainError> {
        let sql = format!(
            "SELECT f.id, f.etablissement_id, f.eleve_id, f.sujet, f.created_by_user_id,
                    f.created_at, f.last_message_at, f.archive, f.modele_id, f.priorite, f.dossier
               FROM ent_msg_fil f
              WHERE f.etablissement_id = $1 AND $2::uuid[] IS NOT NULL AND {PREDICAT_LISIBLE_PERSONNEL}
                AND ($6::uuid IS NULL OR f.eleve_id = $6)
                AND ($7::uuid[] IS NULL OR f.eleve_id = ANY($7))
                AND ($8 OR NOT f.archive)
                AND ($9::text IS NULL OR f.dossier = $9)
                AND ($10::text IS NULL OR f.sujet ILIKE '%' || $10 || '%' ESCAPE '\\')
                AND ($11::smallint IS NULL
                     OR (CASE f.priorite WHEN 'haute' THEN 0 ELSE 1 END) > $11
                     OR ((CASE f.priorite WHEN 'haute' THEN 0 ELSE 1 END) = $11
                         AND (f.last_message_at < $12
                              OR (f.last_message_at = $12 AND f.id < $13))))
              ORDER BY CASE f.priorite WHEN 'haute' THEN 0 ELSE 1 END, f.last_message_at DESC, f.id DESC
              LIMIT $14"
        );
        let dossier = filtre.dossier.map(str::trim).filter(|d| !d.is_empty());
        let recherche = filtre
            .recherche
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(echapper_like);
        let rows = sqlx::query_as::<_, FilRow>(&sql)
            .bind(etablissement_id)
            .bind(Vec::<Uuid>::new()) // $2 (réservé au prédicat commun)
            .bind(user_id)
            .bind(reserve_ok)
            .bind(foyer_ok)
            .bind(filtre.eleve_id)
            .bind(filtre.eleves)
            .bind(filtre.avec_archives || filtre.eleve_id.is_some())
            .bind(dossier)
            .bind(recherche)
            .bind(filtre.apres.map(|c| c.rang))
            .bind(filtre.apres.map(|c| c.dernier_message))
            .bind(filtre.apres.map(|c| c.fil_id))
            .bind(filtre.limite.clamp(1, 200) + 1)
            .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    /// File « messages des familles non traités » (MSG-329, M0b-1) : fils privés d'un parent,
    /// d'un élève ou du personnel à un foyer dont le **dernier message remis** est de la famille
    /// (aucune réponse du personnel depuis), les plus anciens d'abord. `eleves` borne le
    /// périmètre (`None` = tout l'établissement). Ne retourne que des fils lisibles avec
    /// `reserve_ok` : l'appelant exige `messagerie.fils_reserves.lire`.
    pub async fn fils_non_traites(
        &self,
        etablissement_id: Uuid,
        eleves: Option<&[Uuid]>,
        limite: i64,
    ) -> Result<Vec<FilNonTraite>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct Ligne {
            #[sqlx(flatten)]
            fil: FilRow,
            en_attente_depuis: DateTime<Utc>,
        }
        let rows = sqlx::query_as::<_, Ligne>(
            r#"
            SELECT f.id, f.etablissement_id, f.eleve_id, f.sujet, f.created_by_user_id,
                   f.created_at, f.last_message_at, f.archive, f.modele_id, f.priorite, f.dossier,
                   d.created_at AS en_attente_depuis
              FROM ent_msg_fil f
              JOIN LATERAL (
                    SELECT m.auteur_user_id, m.created_at
                      FROM ent_msg m
                     WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id
                       AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now())
                     ORDER BY m.created_at DESC, m.id DESC LIMIT 1) d ON true
             WHERE f.etablissement_id = $1
               AND NOT f.archive AND f.foyer_id IS NULL AND NOT f.vie_scolaire
               AND (f.prive_user_id IS NOT NULL OR f.foyer_lecteur_id IS NOT NULL
                    OR f.audience = 'eleve')
               AND EXISTS (SELECT 1 FROM ent_membership mb
                            WHERE mb.etablissement_id = f.etablissement_id
                              AND mb.user_id = d.auteur_user_id
                              AND mb.role IN ('parent', 'eleve'))
               AND ($2::uuid[] IS NULL OR f.eleve_id = ANY($2))
             ORDER BY d.created_at ASC, f.id ASC
             LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleves)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(|l| {
                Some(FilNonTraite { fil: l.fil.into_fil()?, en_attente_depuis: l.en_attente_depuis })
            })
            .collect())
    }

    /// Rôle de base de chaque compte (membres de l'établissement).
    pub async fn roles_de(
        &self,
        etablissement_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if user_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT user_id, role FROM ent_membership
              WHERE etablissement_id = $1 AND user_id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(user_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    /// Personnels à prévenir quand une famille écrit (M0b, D13) : ceux qui ont déjà écrit dans le
    /// fil (hors `sauf`) ; à défaut, les titulaires de `messagerie.fils_reserves.lire` à
    /// l'échelle de l'établissement (rôle de base `bases` ou rôle paramétré actif).
    pub async fn personnels_a_prevenir(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        sauf: Uuid,
        bases: &[&str],
    ) -> Result<Vec<Uuid>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let ont_ecrit: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT m.auteur_user_id
               FROM ent_msg m
               JOIN ent_membership mb ON mb.etablissement_id = m.etablissement_id
                                     AND mb.user_id = m.auteur_user_id
              WHERE m.etablissement_id = $1 AND m.fil_id = $2 AND m.auteur_user_id <> $3
                AND mb.role NOT IN ('parent', 'eleve')",
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(sauf)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if !ont_ecrit.is_empty() {
            return Ok(ont_ecrit.into_iter().map(|(u,)| u).collect());
        }
        let bases: Vec<String> = bases.iter().map(|b| (*b).to_string()).collect();
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT m.user_id FROM ent_membership m
              WHERE m.etablissement_id = $1 AND m.role = ANY($2) AND m.user_id <> $3
             UNION
             SELECT a.user_id
               FROM ent_role_attribution a
               JOIN ent_role_droit d ON d.role_id = a.role_id
                                    AND d.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.clos_le IS NULL AND a.user_id <> $3
                AND a.debut <= ent_aujourdhui_etab($1) AND (a.fin IS NULL OR a.fin >= ent_aujourdhui_etab($1))
                AND d.action = 'messagerie.fils_reserves.lire' AND d.portee = 'etablissement'",
        )
        .bind(etablissement_id)
        .bind(&bases)
        .bind(sauf)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(u,)| u).collect())
    }

    /// Comptes responsables à notifier pour ce fil : `None` = tous les liens notifiables de
    /// l'élève ; `Some(ids)` = seulement ceux-là (fil privé, de foyer, d'élève ou de vie
    /// scolaire). Le filtre des liens interdits ou « ne pas solliciter » est appliqué ensuite.
    pub async fn destinataires_famille_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Option<Vec<Uuid>>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        #[allow(clippy::type_complexity)]
        let row: Option<(Option<Uuid>, Option<Uuid>, Uuid, String, bool, Option<Uuid>)> =
            sqlx::query_as(
                "SELECT prive_user_id, foyer_id, eleve_id, audience, vie_scolaire, foyer_lecteur_id
               FROM ent_msg_fil WHERE etablissement_id = $1 AND id = $2",
            )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let Some((prive, foyer, eleve, audience, vie_scolaire, foyer_lecteur)) = row else {
            return Ok(Some(Vec::new()));
        };
        if audience == "eleve" {
            // Le fil d'un élève n'est jamais annoncé à un parent.
            return Ok(Some(Vec::new()));
        }
        if let Some(u) = prive {
            return Ok(Some(vec![u]));
        }
        if let Some(fid) = foyer_lecteur {
            // M0b (D22) : le fil d'un foyer n'est annoncé qu'aux responsables de ce foyer.
            let ids: Vec<(Uuid,)> = sqlx::query_as(
                r#"SELECT DISTINCT user_id FROM ent_eleve_lien_communicant
                    WHERE etablissement_id = $1 AND eleve_id = $2 AND foyer_id = $3
                      AND user_id IS NOT NULL"#,
            )
            .bind(etablissement_id)
            .bind(eleve)
            .bind(fid)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            return Ok(Some(ids.into_iter().map(|(u,)| u).collect()));
        }
        if let Some(fid) = foyer {
            let ids: Vec<(Uuid,)> = sqlx::query_as(
                r#"SELECT DISTINCT user_id FROM ent_eleve_lien_communicant
                    WHERE etablissement_id = $1 AND eleve_id = $2 AND foyer_id = $3
                      AND est_payeur AND user_id IS NOT NULL"#,
            )
            .bind(etablissement_id)
            .bind(eleve)
            .bind(fid)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            return Ok(Some(ids.into_iter().map(|(u,)| u).collect()));
        }
        if vie_scolaire {
            let ids: Vec<(Uuid,)> = sqlx::query_as(
                r#"SELECT DISTINCT user_id FROM ent_eleve_lien_communicant
                    WHERE etablissement_id = $1 AND eleve_id = $2
                      AND peut_voir_vs AND autorite_parentale AND user_id IS NOT NULL"#,
            )
            .bind(etablissement_id)
            .bind(eleve)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            return Ok(Some(ids.into_iter().map(|(u,)| u).collect()));
        }
        Ok(None)
    }

    pub async fn set_fil_archive(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        archive: bool,
        archived_at: Option<DateTime<Utc>>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            UPDATE ent_msg_fil
            SET archive = $3, archived_at = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(archive)
        .bind(archived_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_fil_boite(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        priorite: MsgPriorite,
        dossier: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            UPDATE ent_msg_fil
            SET priorite = $3, dossier = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(priorite.as_str())
        .bind(dossier)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn insert_modele(&self, m: &MsgModele) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, m.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg_modele (
                id, etablissement_id, titre, sujet, corps, actif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(&m.titre)
        .bind(&m.sujet)
        .bind(&m.corps)
        .bind(m.actif)
        .bind(m.created_by_user_id)
        .bind(m.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_modele(
        &self,
        etablissement_id: Uuid,
        modele_id: Uuid,
    ) -> Result<Option<MsgModele>, DomainError> {
        let row = sqlx::query_as::<_, ModeleRow>(
            r#"
            SELECT id, etablissement_id, titre, sujet, corps, actif,
                   created_by_user_id, created_at
            FROM ent_msg_modele
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(modele_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(ModeleRow::into_modele))
    }

    pub async fn list_modeles(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<MsgModele>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = if actifs_seulement {
            sqlx::query_as::<_, ModeleRow>(
                r#"
                SELECT id, etablissement_id, titre, sujet, corps, actif,
                       created_by_user_id, created_at
                FROM ent_msg_modele
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY titre ASC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, ModeleRow>(
                r#"
                SELECT id, etablissement_id, titre, sujet, corps, actif,
                       created_by_user_id, created_at
                FROM ent_msg_modele
                WHERE etablissement_id = $1
                ORDER BY titre ASC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(ModeleRow::into_modele).collect())
    }

    pub async fn insert_message(&self, msg: &Message) -> Result<(), DomainError> {
        self.insert_message_remise(msg, None, None).await
    }

    /// Insère un message ; `remise` = instant de remise différée par la plage calme (M0b, D24),
    /// `derogation` = (motif, auteur) d'une remise immédiate demandée par la direction.
    pub async fn insert_message_remise(
        &self,
        msg: &Message,
        remise: Option<DateTime<Utc>>,
        derogation: Option<(&str, Uuid)>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, msg.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg (
                id, etablissement_id, fil_id, auteur_user_id, corps, created_at,
                remise_prevue_le, derogation_motif, derogation_par
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(msg.id)
        .bind(msg.etablissement_id)
        .bind(msg.fil_id)
        .bind(msg.auteur_user_id)
        .bind(&msg.corps)
        .bind(msg.created_at)
        .bind(remise)
        .bind(derogation.map(|d| d.0))
        .bind(derogation.map(|d| d.1))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn touch_fil_last(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            UPDATE ent_msg_fil SET last_message_at = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Option<MsgFil>, DomainError> {
        let row = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(FilRow::into_fil))
    }

    pub async fn list_fils_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<MsgFil>, DomainError> {
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    pub async fn list_fils_eleves(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
    ) -> Result<Vec<MsgFil>, DomainError> {
        if eleve_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND eleve_id = ANY($2)
            ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    /// Messages remis du fil, plus ceux que `lecteur` a lui-même écrits et dont la remise est
    /// différée par la plage calme (M0b, D24 : invisibles de tout autre jusqu'à la remise).
    pub async fn list_messages(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        lecteur: Uuid,
    ) -> Result<Vec<Message>, DomainError> {
        let rows = sqlx::query_as::<_, MsgRow>(
            r#"
            SELECT id, etablissement_id, fil_id, auteur_user_id, corps, created_at
            FROM ent_msg
            WHERE etablissement_id = $1 AND fil_id = $2
              AND (remise_prevue_le IS NULL OR remise_prevue_le <= now() OR auteur_user_id = $3)
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(lecteur)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(MsgRow::into_msg).collect())
    }

    /// Insert accusés ; ignore les doublons (message_id, lecteur_user_id).
    pub async fn upsert_accuses(&self, accuses: &[MsgAccuse]) -> Result<usize, DomainError> {
        let mut inserted = 0usize;
        for a in accuses {
            let mut tx = TenantTx::begin(&self.pool, a.etablissement_id).await.map_err(map_sqlx)?;
            let res = sqlx::query(
                r#"
                INSERT INTO ent_msg_accuse (
                    id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at
                ) VALUES ($1,$2,$3,$4,$5,$6)
                ON CONFLICT (message_id, lecteur_user_id) DO NOTHING
                "#,
            )
            .bind(a.id)
            .bind(a.etablissement_id)
            .bind(a.fil_id)
            .bind(a.message_id)
            .bind(a.lecteur_user_id)
            .bind(a.lu_at)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            tx.commit().await.map_err(map_sqlx)?;
            inserted += res.rows_affected() as usize;
        }
        Ok(inserted)
    }

    pub async fn list_accuses_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<MsgAccuse>, DomainError> {
        let rows = sqlx::query_as::<_, AccuseRow>(
            r#"
            SELECT id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at
            FROM ent_msg_accuse
            WHERE etablissement_id = $1 AND fil_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AccuseRow::into_accuse).collect())
    }

    /// Messages non accusés par le lecteur (hors ses propres messages), par fil.
    pub async fn count_non_lus_par_fils(
        &self,
        etablissement_id: Uuid,
        lecteur_user_id: Uuid,
        fil_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, i64>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT m.fil_id, COUNT(*)::bigint
            FROM ent_msg m
            WHERE m.etablissement_id = $1
              AND m.fil_id = ANY($2)
              AND m.auteur_user_id <> $3
              AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now())
              AND NOT EXISTS (
                SELECT 1 FROM ent_msg_accuse a
                WHERE a.message_id = m.id AND a.lecteur_user_id = $3
              )
            GROUP BY m.fil_id
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_ids)
        .bind(lecteur_user_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    pub async fn insert_piece(&self, piece: &MsgPiece) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, piece.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg_piece (
                id, etablissement_id, fil_id, message_id, nom, mime_type,
                taille_octets, contenu, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(piece.id)
        .bind(piece.etablissement_id)
        .bind(piece.fil_id)
        .bind(piece.message_id)
        .bind(&piece.nom)
        .bind(&piece.mime_type)
        .bind(piece.taille_octets)
        .bind(&piece.contenu)
        .bind(piece.created_by_user_id)
        .bind(piece.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_pieces_meta_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<MsgPieceMeta>, DomainError> {
        let rows = sqlx::query_as::<_, PieceMetaRow>(
            r#"
            SELECT id, message_id, nom, mime_type, taille_octets,
                   created_by_user_id, created_at
            FROM ent_msg_piece
            WHERE etablissement_id = $1 AND fil_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(PieceMetaRow::into_meta).collect())
    }

    pub async fn get_piece(
        &self,
        etablissement_id: Uuid,
        piece_id: Uuid,
    ) -> Result<Option<MsgPiece>, DomainError> {
        let row = sqlx::query_as::<_, PieceRow>(
            r#"
            SELECT id, etablissement_id, fil_id, message_id, nom, mime_type,
                   taille_octets, contenu, created_by_user_id, created_at
            FROM ent_msg_piece
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(piece_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(PieceRow::into_piece))
    }

    /// Liens communicants de l'élève groupés par foyer (`None` = lien sans foyer) : un fil du
    /// personnel est créé par groupe (M0b, D22). Ordre stable (foyers par identifiant).
    pub async fn groupes_communicants(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<(Option<Uuid>, Vec<Uuid>)>, DomainError> {
        let rows: Vec<(Option<Uuid>, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT foyer_id, user_id FROM ent_eleve_lien_communicant
              WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id IS NOT NULL
              ORDER BY foyer_id NULLS LAST, user_id",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        let mut groupes: Vec<(Option<Uuid>, Vec<Uuid>)> = Vec::new();
        for (foyer, user) in rows {
            match groupes.last_mut() {
                Some((f, users)) if *f == foyer => users.push(user),
                _ => groupes.push((foyer, vec![user])),
            }
        }
        Ok(groupes)
    }

    /// Supprime un message dont la remise est encore à venir (R-M48), s'il est de `auteur`.
    /// `false` si le message est déjà remis, d'un autre auteur ou absent.
    pub async fn annuler_message_differe(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        auteur: Uuid,
    ) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let ok: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_msg
              WHERE etablissement_id = $1 AND id = $2 AND auteur_user_id = $3
                AND remise_prevue_le > now() FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .bind(auteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if ok.is_none() {
            return Ok(false);
        }
        for table in ["ent_msg_piece", "ent_msg_accuse"] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE etablissement_id = $1 AND message_id = $2"
            ))
            .bind(etablissement_id)
            .bind(message_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        sqlx::query("DELETE FROM ent_msg WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(message_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(true)
    }

    pub async fn count_pieces_message(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
    ) -> Result<i64, DomainError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1 AND message_id = $2",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    /// Le message est-il remis, ou écrit par `lecteur` lui-même (M0b, D24) ?
    pub async fn message_remis_pour(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        lecteur: Uuid,
    ) -> Result<bool, DomainError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT coalesce((SELECT remise_prevue_le IS NULL OR remise_prevue_le <= now()
                                     OR auteur_user_id = $3
                                FROM ent_msg WHERE etablissement_id = $1 AND id = $2), false)",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .bind(lecteur)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(ok)
    }

    pub async fn get_message(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Message>, DomainError> {
        let row = sqlx::query_as::<_, MsgRow>(
            r#"
            SELECT id, etablissement_id, fil_id, auteur_user_id, corps, created_at
            FROM ent_msg
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(message_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(MsgRow::into_msg))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FilRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    sujet: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    last_message_at: DateTime<Utc>,
    archive: bool,
    modele_id: Option<Uuid>,
    priorite: String,
    dossier: Option<String>,
}

impl FilRow {
    fn into_fil(self) -> Option<MsgFil> {
        let priorite = MsgPriorite::parse(&self.priorite)?;
        Some(MsgFil {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            sujet: self.sujet,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            last_message_at: self.last_message_at,
            archive: self.archive,
            modele_id: self.modele_id,
            priorite,
            dossier: self.dossier,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ModeleRow {
    id: Uuid,
    etablissement_id: Uuid,
    titre: String,
    sujet: String,
    corps: String,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ModeleRow {
    fn into_modele(self) -> MsgModele {
        MsgModele {
            id: self.id,
            etablissement_id: self.etablissement_id,
            titre: self.titre,
            sujet: self.sujet,
            corps: self.corps,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MsgRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    auteur_user_id: Uuid,
    corps: String,
    created_at: DateTime<Utc>,
}

impl MsgRow {
    fn into_msg(self) -> Message {
        Message {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            auteur_user_id: self.auteur_user_id,
            corps: self.corps,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AccuseRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    lecteur_user_id: Uuid,
    lu_at: DateTime<Utc>,
}

impl AccuseRow {
    fn into_accuse(self) -> MsgAccuse {
        MsgAccuse {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            message_id: self.message_id,
            lecteur_user_id: self.lecteur_user_id,
            lu_at: self.lu_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PieceMetaRow {
    id: Uuid,
    message_id: Uuid,
    nom: String,
    mime_type: String,
    taille_octets: i32,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PieceMetaRow {
    fn into_meta(self) -> MsgPieceMeta {
        MsgPieceMeta {
            id: self.id,
            message_id: self.message_id,
            nom: self.nom,
            mime_type: self.mime_type,
            taille_octets: self.taille_octets,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PieceRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    nom: String,
    mime_type: String,
    taille_octets: i32,
    contenu: Vec<u8>,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PieceRow {
    fn into_piece(self) -> MsgPiece {
        MsgPiece {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            message_id: self.message_id,
            nom: self.nom,
            mime_type: self.mime_type,
            taille_octets: self.taille_octets,
            contenu: self.contenu,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("msg_repo", err)
}
