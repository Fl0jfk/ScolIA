use chrono::NaiveDate;
use domain_eleve::{DomainError, Eleve, EleveStatus};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

/// Paramètres de [`PgEleveRepo::rechercher`] : tout ce que l'appelant a le droit de voir.
pub(crate) struct ParamsRecherche<'a> {
    pub etablissement_id: Uuid,
    pub q: &'a str,
    pub limit: i64,
    /// Portée de l'annuaire (G0) : `None` = tout l'établissement.
    pub designation: Option<&'a [Uuid]>,
    /// État civil (G2) ouvert partout : le nom de naissance et les prénoms sont interrogeables.
    pub civil: bool,
    /// Scolarité (G1) ouverte partout : le début du numéro d'élève est interrogeable.
    pub numero: bool,
    /// Identifiants (G3) ouverts partout : INE et badge (valeur complète) interrogeables.
    pub identifiants: bool,
    /// G6 ouvert partout : les élèves en confidentialité renforcée sont visibles.
    pub proteges_tous: bool,
    /// Élèves protégés visibles par lien direct (portée restreinte) ou par G6 restreint.
    pub proteges_ids: &'a [Uuid],
}

pub struct PgEleveRepo {
    pool: PgPool,
}

impl PgEleveRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<Eleve>, DomainError> {
        let row = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;

        Ok(row.map(EleveRow::into_eleve))
    }

    pub async fn get_by_badge(
        &self,
        etablissement_id: Uuid,
        numero_badge: &str,
    ) -> Result<Option<Eleve>, DomainError> {
        let badge = numero_badge.trim();
        if badge.is_empty() {
            return Ok(None);
        }
        let row = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND lower(numero_badge) = lower($2)
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(badge)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(EleveRow::into_eleve))
    }

    pub async fn labels(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if eleve_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
            r#"
            SELECT id, prenom, nom
            FROM ent_eleve
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|(id, prenom, nom)| {
                let label = format!("{} {}", prenom.trim(), nom.trim())
                    .trim()
                    .to_string();
                (id, if label.is_empty() { nom } else { label })
            })
            .collect())
    }

    pub async fn search(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
        par_badge: bool,
    ) -> Result<Vec<Eleve>, DomainError> {
        // Un octet NUL est refusé par PostgreSQL : rejet explicite (400), pas une panne.
        if q.contains('\0') {
            return Err(DomainError::ValeurInvalide);
        }
        // `%`, `_` et `\` saisis par l'utilisateur sont littéraux (échappés, clause ESCAPE).
        let echappe = q
            .trim()
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like = format!("%{echappe}%");
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND fusionne_dans IS NULL
              AND (
                lower(nom) LIKE $2 ESCAPE '\'
                OR lower(prenom) LIKE $2 ESCAPE '\'
                OR lower(coalesce(classe, '')) LIKE $2 ESCAPE '\'
                OR ($4 AND lower(coalesce(numero_badge, '')) LIKE $2 ESCAPE '\')
              )
            ORDER BY nom, prenom
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(like)
        .bind(limit)
        .bind(par_badge)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    /// Recherche instantanée (ELV-20, lot E6) : tolérante aux accents, à la casse, aux traits
    /// d'union et aux fautes (index trigramme sur `cle_recherche`, `unaccent`).
    ///
    /// **Aucune donnée n'est lue hors des droits de l'appelant** : la portée de l'annuaire, les
    /// groupes interrogeables (état civil G2, numéro G1, identifiants G3) et la visibilité des
    /// élèves protégés sont tous des conditions SQL — le tri et la limite s'appliquent donc
    /// APRÈS le filtrage (un professeur qui a 30 élèves n'en voit pas 20 de moins parce que 50
    /// autres élèves de l'établissement portent le même prénom).
    ///
    /// Chaque mot de la requête doit retrouver l'élève (nom, prénom, classe ; nom de naissance
    /// et prénoms d'état civil seulement avec G2 partout). Un identifiant complet (INE, badge)
    /// ou un début de numéro d'élève retrouve l'élève seulement avec les droits correspondants :
    /// jamais de recherche par préfixe d'INE (pas d'énumération).
    pub(crate) async fn rechercher(
        conn: &mut PgConnection,
        p: &ParamsRecherche<'_>,
    ) -> Result<Vec<Eleve>, sqlx::Error> {
        if p.q.contains('\0') {
            return Err(sqlx::Error::Protocol("octet NUL dans la requête".into()));
        }
        // Normalisation par la MÊME fonction SQL que la clé stockée (aucune dérive possible).
        let (cle,): (String,) = sqlx::query_as("SELECT ent_cle_recherche($1)")
            .bind(p.q.chars().take(200).collect::<String>())
            .fetch_one(&mut *conn)
            .await?;
        let tokens: Vec<String> = cle
            .split_whitespace()
            .take(5)
            .map(|t| t.chars().take(40).collect())
            .collect();
        // Une saisie faite seulement de ponctuation (« % », « _ », « \\ ») ne cherche rien : elle ne
        // doit pas dégénérer en annuaire complet (seule la requête vide liste).
        if tokens.is_empty() && !p.q.trim().is_empty() {
            return Ok(Vec::new());
        }
        // Seuil de similarité des mots (fautes de frappe), local à la transaction.
        sqlx::query("SELECT set_config('pg_trgm.word_similarity_threshold', '0.5', true)")
            .execute(&mut *conn)
            .await?;

        let brut: String = p.q.trim().chars().take(40).collect();
        let ident: String = brut
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        let ident_ok = p.identifiants && ident.chars().count() >= 4;
        let numero: String = brut.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        let numero_ok = p.numero && numero.chars().count() >= 4 && numero.chars().any(|c| c.is_ascii_digit());

        let mut conds: Vec<String> = Vec::new();
        let mut scores: Vec<String> = Vec::new();
        for (i, t) in tokens.iter().enumerate() {
            let n = 10 + i; // $10, $11… = jetons
            let long = t.chars().count() >= 3;
            let sim = t.chars().count() >= 4;
            let mut c = String::from("(");
            if long {
                c += &format!("e.cle_recherche LIKE '%' || ${n} || '%'");
            } else {
                c += &format!("(' ' || e.cle_recherche) LIKE '% ' || ${n} || '%'");
            }
            if sim {
                c += &format!(" OR ${n} <% e.cle_recherche");
            }
            c += &format!(" OR lower(coalesce(e.classe, '')) LIKE ${n} || '%'");
            if p.civil {
                if long {
                    c += &format!(" OR e.cle_recherche_civil LIKE '%' || ${n} || '%'");
                } else {
                    c += &format!(" OR (' ' || e.cle_recherche_civil) LIKE '% ' || ${n} || '%'");
                }
                if sim {
                    c += &format!(" OR ${n} <% e.cle_recherche_civil");
                }
            }
            c += ")";
            conds.push(c);
            scores.push(format!(
                "GREATEST(CASE WHEN (' ' || e.cle_recherche) LIKE '% ' || ${n} || '%' THEN 1.0 ELSE 0.0 END, \
                          word_similarity(${n}, e.cle_recherche))"
            ));
        }
        let noms = if conds.is_empty() { "TRUE".to_string() } else { conds.join(" AND ") };
        // Requête vide : pas de classement par pertinence (un ORDER BY constant est refusé).
        let ordre = if scores.is_empty() {
            "e.nom, e.prenom, e.id".to_string()
        } else {
            format!("({}) DESC, e.nom, e.prenom, e.id", scores.join(" + "))
        };
        // Requête vide : annuaire alphabétique (comportement historique). Requête non vide :
        // les mots, OU un identifiant complet / un début de numéro (si les droits l'ouvrent).
        let critere = if tokens.is_empty() {
            "TRUE".to_string()
        } else {
            format!(
                "(({noms}) OR ($5 AND (upper(e.ine) = $6 OR upper(e.numero_badge) = $6)) \
                   OR ($7 AND e.numero ILIKE $8 || '%'))"
            )
        };
        let sql = format!(
            "WITH j AS (SELECT ent_jour_etablissement($1) AS jour)
             SELECT e.id, e.etablissement_id, e.nom, e.prenom, e.ine, e.sexe, e.date_naissance,
                    e.numero_badge, e.redoublant, e.photo_key, e.status, e.classe, e.regime, e.mef,
                    e.site, e.date_entree, e.date_sortie, e.motif_sortie
               FROM ent_eleve e CROSS JOIN j
              WHERE e.etablissement_id = $1
                AND e.fusionne_dans IS NULL
                AND ($2::uuid[] IS NULL OR e.id = ANY($2))
                AND ($3 OR e.id = ANY($4) OR NOT EXISTS (
                      SELECT 1 FROM ent_eleve_confidentialite c
                       WHERE c.etablissement_id = e.etablissement_id AND c.eleve_id = e.id
                         AND c.leve_le IS NULL AND c.du <= j.jour AND (c.au IS NULL OR c.au > j.jour)))
                AND {critere}
              ORDER BY {ordre}
              LIMIT $9"
        );
        let mut q = sqlx::query_as::<_, EleveRow>(&sql)
            .bind(p.etablissement_id)
            .bind(p.designation.map(<[Uuid]>::to_vec))
            .bind(p.proteges_tous)
            .bind(p.proteges_ids.to_vec())
            .bind(ident_ok)
            .bind(ident)
            .bind(numero_ok)
            .bind(numero)
            .bind(p.limit.clamp(1, 2000));
        for t in &tokens {
            q = q.bind(t.clone());
        }
        Ok(q.fetch_all(&mut *conn).await?.into_iter().map(EleveRow::into_eleve).collect())
    }

    /// Élèves d'un établissement par identifiants (ordre alphabétique).
    pub async fn list_by_ids(
        &self,
        etablissement_id: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<Eleve>, DomainError> {
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1 AND id = ANY($2)
            ORDER BY nom, prenom
            "#,
        )
        .bind(etablissement_id)
        .bind(ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    /// Effectif d'une classe **au jour J** = périodes + dates d'entrée/sortie.
    pub async fn list_en_classe_au(
        &self,
        etablissement_id: Uuid,
        classe: &str,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, DomainError> {
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT e.id, e.etablissement_id, e.nom, e.prenom, e.ine, e.sexe, e.date_naissance,
                   e.numero_badge, e.redoublant, e.photo_key, e.status, e.classe, e.regime,
                   e.mef, e.site, e.date_entree, e.date_sortie, e.motif_sortie
            FROM ent_eleve e
            INNER JOIN ent_eleve_periode p
                ON p.etablissement_id = e.etablissement_id
               AND p.eleve_id = e.id
               AND p.kind = 'classe'
               AND p.valeur = $2
               AND p.date_debut <= $3
               AND (p.date_fin IS NULL OR p.date_fin >= $3)
            WHERE e.etablissement_id = $1
              AND (e.date_entree IS NULL OR e.date_entree <= $3)
              AND (e.date_sortie IS NULL OR e.date_sortie > $3)
            ORDER BY e.nom, e.prenom
            "#,
        )
        .bind(etablissement_id)
        .bind(classe)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;

        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    /// Effectif actif à une date donnée (entrées/sorties datées), sans jointure
    /// de période : réponse au « combien d'élèves ? » du copilote.
    pub async fn count_actifs_au(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<i64, DomainError> {
        let (n,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND (date_entree IS NULL OR date_entree <= $2)
              AND (date_sortie IS NULL OR date_sortie > $2)
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    /// Liste des élèves actifs à une date (entrées/sorties datées) — base des
    /// diffusions établissement → familles.
    pub async fn list_actifs_au(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, DomainError> {
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND (date_entree IS NULL OR date_entree <= $2)
              AND (date_sortie IS NULL OR date_sortie > $2)
            ORDER BY nom, prenom
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    pub async fn upsert(&self, eleve: &Eleve) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, eleve.etablissement_id).await.map_err(map_sqlx)?;
        Self::upsert_sur(&mut tx, eleve).await?;
        tx.commit().await.map_err(map_sqlx)
    }

    /// Lecture avec verrou de ligne dans la transaction du fait métier (E0) :
    /// évite qu'une modification concurrente soit écrasée en silence.
    pub async fn get_pour_maj(
        conn: &mut PgConnection,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<Eleve>, DomainError> {
        let row = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(EleveRow::into_eleve))
    }

    /// Écriture sur une connexion fournie (transaction de l'appelant).
    pub async fn upsert_sur(conn: &mut PgConnection, eleve: &Eleve) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            INSERT INTO ent_eleve (
                id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                redoublant, photo_key, status, classe, regime, mef, site,
                date_entree, date_sortie, motif_sortie, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18, now()
            )
            ON CONFLICT (id) DO UPDATE SET
                -- nom, prénom, sexe, date de naissance : identité versionnée (E2), écrite
                -- par la base depuis une version ; jamais ici.
                ine = EXCLUDED.ine,
                numero_badge = EXCLUDED.numero_badge,
                redoublant = EXCLUDED.redoublant,
                photo_key = EXCLUDED.photo_key,
                status = EXCLUDED.status,
                classe = EXCLUDED.classe,
                regime = EXCLUDED.regime,
                mef = EXCLUDED.mef,
                site = EXCLUDED.site,
                date_entree = EXCLUDED.date_entree,
                date_sortie = EXCLUDED.date_sortie,
                motif_sortie = EXCLUDED.motif_sortie,
                updated_at = now()
            WHERE ent_eleve.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(eleve.id)
        .bind(eleve.etablissement_id)
        .bind(&eleve.nom)
        .bind(&eleve.prenom)
        .bind(&eleve.ine)
        .bind(&eleve.sexe)
        .bind(eleve.date_naissance)
        .bind(&eleve.numero_badge)
        .bind(eleve.redoublant)
        .bind(&eleve.photo_key)
        .bind(eleve.status.as_str())
        .bind(&eleve.classe)
        .bind(&eleve.regime)
        .bind(&eleve.mef)
        .bind(&eleve.site)
        .bind(eleve.date_entree)
        .bind(eleve.date_sortie)
        .bind(&eleve.motif_sortie)
        .execute(&mut *conn)
        .await
        .map_err(map_sqlx)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::TenantMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EleveRow {
    id: Uuid,
    etablissement_id: Uuid,
    nom: String,
    prenom: String,
    ine: Option<String>,
    sexe: Option<String>,
    date_naissance: Option<NaiveDate>,
    numero_badge: Option<String>,
    redoublant: bool,
    photo_key: Option<String>,
    status: String,
    classe: Option<String>,
    regime: Option<String>,
    mef: Option<String>,
    site: Option<String>,
    date_entree: Option<NaiveDate>,
    date_sortie: Option<NaiveDate>,
    motif_sortie: Option<String>,
}

impl EleveRow {
    fn into_eleve(self) -> Eleve {
        Eleve {
            id: self.id,
            etablissement_id: self.etablissement_id,
            nom: self.nom,
            prenom: self.prenom,
            ine: self.ine,
            sexe: self.sexe,
            date_naissance: self.date_naissance,
            numero_badge: self.numero_badge,
            redoublant: self.redoublant,
            photo_key: self.photo_key,
            status: parse_status(&self.status),
            classe: self.classe,
            regime: self.regime,
            mef: self.mef,
            site: self.site,
            date_entree: self.date_entree,
            date_sortie: self.date_sortie,
            motif_sortie: self.motif_sortie,
        }
    }
}

fn parse_status(s: &str) -> EleveStatus {
    match s {
        "sorti" => EleveStatus::Sorti,
        "preinscrit" => EleveStatus::Preinscrit,
        _ => EleveStatus::Actif,
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("eleve_repo", err)
}
