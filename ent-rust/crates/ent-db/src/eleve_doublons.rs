//! Socle 2 (l'élève), lot E6 — détection des doublons d'élèves (ELV-22) et paires déclarées
//! distinctes (ELV-33, R-E20).
//!
//! * **Détection** : la base propose des paires candidates (INE identique, même désignation,
//!   même date de naissance et noms proches, noms très proches par l'index trigramme) ; la
//!   logique pure `domain_eleve::evaluer_doublon` décide (jumeaux jamais proposés, INE différents
//!   qui baissent le score…). Le résultat est un **cache** (`ent_eleve_doublon`) reconstruit
//!   chaque jour et à la demande ; une paire qui ne se détecte plus devient `obsolete`.
//! * **Lecture** : droit `eleves.identite.modifier` (ou `eleves.fusionner`) à la portée
//!   établissement ; une paire dont un élève est en **confidentialité renforcée** n'existe pas
//!   pour qui n'a pas G6 (on ne révèle ni l'élève ni le fait qu'il a un homonyme).
//! * **ELV-33** : « ce sont bien deux élèves » ; mémorisé avec les versions d'identité du moment,
//!   la paire ne revient que si l'identité de l'un des deux change.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    evaluer_doublon, paire_ordonnee, DomainError, GroupeChamps, IdentiteDoublon, RaisonDoublon,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use uuid::Uuid;

use crate::eleve_champs::EleveVue;
use crate::eleve_repo::PgEleveRepo;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Au plus ce nombre de paires candidates par passage (borne mémoire ; une base saine en a
/// quelques dizaines).
const MAX_CANDIDATS: i64 = 50_000;

/// Une paire de doublons probables, telle que la direction la voit.
#[derive(Debug, Clone)]
pub struct DoublonVue {
    pub id: Uuid,
    pub score: u8,
    pub raisons: Vec<String>,
    pub detecte_le: DateTime<Utc>,
    pub a: EleveVue,
    pub b: EleveVue,
}

/// Bilan d'un passage de détection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanDetection {
    pub paires: usize,
    pub nouvelles: usize,
    pub devenues_obsoletes: usize,
}

#[derive(Debug, sqlx::FromRow)]
struct LigneIdentite {
    id: Uuid,
    nom: String,
    prenom: String,
    nom_naissance: Option<String>,
    prenoms: Option<Vec<String>>,
    date_naissance: Option<NaiveDate>,
    date_naissance_precision: String,
    date_naissance_statut: String,
    ine: Option<String>,
    identite_version_id: Option<Uuid>,
}

impl LigneIdentite {
    fn en_identite(&self) -> IdentiteDoublon {
        IdentiteDoublon {
            id: self.id,
            nom: self.nom.clone(),
            prenom: self.prenom.clone(),
            nom_naissance: self.nom_naissance.clone().unwrap_or_default(),
            prenoms: self.prenoms.clone().unwrap_or_default(),
            date_naissance: self.date_naissance,
            precision: self.date_naissance_precision.clone(),
            statut_date: self.date_naissance_statut.clone(),
            ine: self.ine.clone(),
        }
    }
}

impl PgEnt {
    /// Droit de gérer les doublons : correction d'identité ou fusion, portée établissement.
    async fn exiger_gestion_doublons(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, Action::ElevesIdentiteModifier).await?
            || self.a_action_etablissement(etab, acteur, role, Action::ElevesFusionner).await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// ELV-22 (traitement) : recalcule les paires de doublons probables d'un établissement.
    /// Sans acteur : lancé par la tâche quotidienne ou par une route qui a déjà contrôlé le droit.
    pub async fn detecter_doublons(&self, etab: Uuid) -> Result<BilanDetection, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("SELECT set_config('pg_trgm.similarity_threshold', '0.55', true)")
            .execute(&mut *tx)
            .await?;
        // Paires candidates (a < b), quatre voies indépendantes.
        let candidats: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "WITH base AS (
                SELECT id, ine, date_naissance, date_naissance_precision, cle_recherche, cle_recherche_civil
                  FROM ent_eleve WHERE etablissement_id = $1 AND fusionne_dans IS NULL)
             SELECT DISTINCT a, b FROM (
                -- 1. INE identique
                SELECT x.id AS a, y.id AS b FROM base x JOIN base y
                    ON x.id < y.id AND x.ine IS NOT NULL AND x.ine <> '' AND upper(x.ine) = upper(y.ine)
                UNION ALL
                -- 2. même désignation, ou même état civil
                SELECT x.id, y.id FROM base x JOIN base y
                    ON x.id < y.id AND x.cle_recherche <> '' AND x.cle_recherche = y.cle_recherche
                UNION ALL
                SELECT x.id, y.id FROM base x JOIN base y
                    ON x.id < y.id AND x.cle_recherche_civil <> '' AND x.cle_recherche_civil = y.cle_recherche_civil
                UNION ALL
                -- 3. même date de naissance et noms proches (une faute de saisie, une inversion)
                SELECT x.id, y.id FROM base x JOIN base y
                    ON x.id < y.id AND x.date_naissance = y.date_naissance
                   AND x.date_naissance_precision = 'jour' AND y.date_naissance_precision = 'jour'
                   AND similarity(x.cle_recherche, y.cle_recherche) >= 0.3
                UNION ALL
                -- 4. noms très proches (index trigramme), date de naissance quelconque
                SELECT x.id, y.id FROM base x
                  JOIN LATERAL (SELECT b.id FROM ent_eleve b
                                 WHERE b.etablissement_id = $1 AND b.fusionne_dans IS NULL
                                   AND b.id > x.id AND b.cle_recherche % x.cle_recherche) y ON TRUE
             ) c LIMIT $2",
        )
        .bind(etab)
        .bind(MAX_CANDIDATS)
        .fetch_all(&mut *tx)
        .await?;
        let ids: Vec<Uuid> = {
            let mut s: HashSet<Uuid> = HashSet::new();
            for (a, b) in &candidats {
                s.insert(*a);
                s.insert(*b);
            }
            s.into_iter().collect()
        };
        let lignes: Vec<LigneIdentite> = sqlx::query_as(
            "SELECT id, nom, prenom, nom_naissance, prenoms, date_naissance, date_naissance_precision,
                    date_naissance_statut, ine, identite_version_id
               FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await?;
        let par_id: HashMap<Uuid, &LigneIdentite> = lignes.iter().map(|l| (l.id, l)).collect();
        // Paires déjà déclarées distinctes, avec les versions d'identité du moment.
        let non: Vec<(Uuid, Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT eleve_a, eleve_b, identite_a, identite_b FROM ent_eleve_non_doublon WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await?;
        let non: HashMap<(Uuid, Uuid), (Option<Uuid>, Option<Uuid>)> =
            non.into_iter().map(|(a, b, va, vb)| ((a, b), (va, vb))).collect();

        let mut trouvees: Vec<(Uuid, Uuid, i16, String)> = Vec::new();
        for (a, b) in candidats {
            let (a, b) = paire_ordonnee(a, b);
            let (Some(la), Some(lb)) = (par_id.get(&a), par_id.get(&b)) else { continue };
            // Déclarée distincte et aucune des deux identités n'a changé depuis : on n'en parle plus.
            if let Some((va, vb)) = non.get(&(a, b)) {
                if *va == la.identite_version_id && *vb == lb.identite_version_id {
                    continue;
                }
            }
            if let Some(s) = evaluer_doublon(&la.en_identite(), &lb.en_identite()) {
                let raisons: Vec<&str> = s.raisons.iter().map(|r: &RaisonDoublon| r.code()).collect();
                trouvees.push((a, b, i16::from(s.score), raisons.join(",")));
            }
        }
        trouvees.sort();
        trouvees.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);

        let (va, vb, vs, vr): (Vec<Uuid>, Vec<Uuid>, Vec<i16>, Vec<String>) = trouvees.iter().cloned().fold(
            (vec![], vec![], vec![], vec![]),
            |mut acc, (a, b, s, r)| {
                acc.0.push(a);
                acc.1.push(b);
                acc.2.push(s);
                acc.3.push(r);
                acc
            },
        );
        // Nouvelles paires (jamais vues, ou revenues après avoir été obsolètes).
        let (nouvelles,): (i64,) = sqlx::query_as(
            "WITH t AS (SELECT * FROM unnest($2::uuid[], $3::uuid[], $4::int2[], $5::text[]) AS t(a, b, s, r)),
                  maj AS (
                    INSERT INTO ent_eleve_doublon (id, etablissement_id, eleve_a, eleve_b, score, raisons, statut)
                    SELECT gen_random_uuid(), $1, t.a, t.b, t.s, string_to_array(t.r, ','), 'a_examiner' FROM t
                    ON CONFLICT (etablissement_id, eleve_a, eleve_b) DO UPDATE
                       SET score = EXCLUDED.score, raisons = EXCLUDED.raisons, maj_le = now(),
                           statut = CASE WHEN ent_eleve_doublon.statut = 'fusionne'
                                         THEN 'fusionne' ELSE 'a_examiner' END,
                           detecte_le = CASE WHEN ent_eleve_doublon.statut = 'obsolete'
                                             THEN now() ELSE ent_eleve_doublon.detecte_le END
                    RETURNING (xmax = 0) AS insere)
             SELECT count(*) FILTER (WHERE insere) FROM maj",
        )
        .bind(etab)
        .bind(&va)
        .bind(&vb)
        .bind(&vs)
        .bind(&vr)
        .fetch_one(&mut *tx)
        .await?;
        // Ce qui ne se détecte plus devient obsolète (rien n'est supprimé).
        let obsoletes = sqlx::query(
            "UPDATE ent_eleve_doublon d SET statut = 'obsolete', maj_le = now()
              WHERE d.etablissement_id = $1 AND d.statut = 'a_examiner'
                AND NOT EXISTS (SELECT 1 FROM unnest($2::uuid[], $3::uuid[]) AS t(a, b)
                                 WHERE t.a = d.eleve_a AND t.b = d.eleve_b)",
        )
        .bind(etab)
        .bind(&va)
        .bind(&vb)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        sqlx::query(
            "INSERT INTO ent_eleve_doublon_passage (etablissement_id, dernier_jour, nb_paires, passe_le)
             VALUES ($1, ent_jour_etablissement($1), $2, now())
             ON CONFLICT (etablissement_id) DO UPDATE
                SET dernier_jour = EXCLUDED.dernier_jour, nb_paires = EXCLUDED.nb_paires, passe_le = now()",
        )
        .bind(etab)
        .bind(trouvees.len() as i32)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(BilanDetection {
            paires: trouvees.len(),
            nouvelles: usize::try_from(nouvelles).unwrap_or(0),
            devenues_obsoletes: usize::try_from(obsoletes).unwrap_or(0),
        })
    }

    /// ELV-22 à la demande de la secrétaire ou de la direction.
    pub async fn detecter_doublons_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
    ) -> Result<BilanDetection, EntDbError> {
        self.exiger_gestion_doublons(etab, acteur, role).await?;
        self.detecter_doublons(etab).await
    }

    /// Tâche quotidienne : un passage par établissement et par jour (jour de l'établissement).
    /// Un échec sur un établissement ne bloque pas les autres. Retourne le nombre
    /// d'établissements traités.
    pub async fn tache_doublons_eleves(&self) -> Result<usize, EntDbError> {
        let etabs: Vec<(Uuid,)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::DoublonsEleves,
            "SELECT DISTINCT e.etablissement_id FROM ent_eleve e
              WHERE e.fusionne_dans IS NULL
                AND NOT EXISTS (SELECT 1 FROM ent_eleve_doublon_passage p
                                 WHERE p.etablissement_id = e.etablissement_id
                                   AND p.dernier_jour >= ent_jour_etablissement(e.etablissement_id))",
        )
        .await?;
        let mut n = 0;
        for (etab,) in etabs {
            match self.detecter_doublons(etab).await {
                Ok(_) => n += 1,
                Err(e) => tracing::warn!(error = %e, "détection de doublons d'élèves non faite"),
            }
        }
        Ok(n)
    }

    /// Doublons probables à examiner, du plus sûr au moins sûr. Les paires qui touchent un élève
    /// en confidentialité renforcée n'existent que pour qui détient G6.
    pub async fn doublons_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        limite: i64,
    ) -> Result<Vec<DoublonVue>, EntDbError> {
        self.exiger_gestion_doublons(etab, acteur, role).await?;
        let profil = self.profil_champs(etab, acteur, role).await?;
        let voit_proteges = profil.ouvert_partout(GroupeChamps::Sensible);
        type LigneDoublon = (Uuid, Uuid, Uuid, i16, Vec<String>, DateTime<Utc>);
        let lignes: Vec<LigneDoublon> = sqlx::query_as(
            "WITH j AS (SELECT ent_jour_etablissement($1) AS jour)
             SELECT d.id, d.eleve_a, d.eleve_b, d.score, d.raisons, d.detecte_le
               FROM ent_eleve_doublon d CROSS JOIN j
              WHERE d.etablissement_id = $1 AND d.statut = 'a_examiner'
                AND ($2 OR NOT EXISTS (
                      SELECT 1 FROM ent_eleve_confidentialite c
                       WHERE c.etablissement_id = d.etablissement_id AND c.eleve_id IN (d.eleve_a, d.eleve_b)
                         AND c.leve_le IS NULL AND c.du <= j.jour AND (c.au IS NULL OR c.au > j.jour)))
              ORDER BY d.score DESC, d.detecte_le, d.id
              LIMIT $3",
        )
        .bind(etab)
        .bind(voit_proteges)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let ids: Vec<Uuid> = lignes.iter().flat_map(|l| [l.1, l.2]).collect();
        let eleves: HashMap<Uuid, domain_eleve::Eleve> = PgEleveRepo::new(self.pool().clone())
            .list_by_ids(etab, &ids)
            .await?
            .into_iter()
            .map(|e| (e.id, e))
            .collect();
        let mut out = Vec::new();
        for (id, a, b, score, raisons, detecte_le) in lignes {
            let (Some(ea), Some(eb)) = (eleves.get(&a), eleves.get(&b)) else { continue };
            out.push(DoublonVue {
                id,
                score: u8::try_from(score).unwrap_or(0),
                raisons,
                detecte_le,
                a: EleveVue::de(ea.clone(), profil.pour(a)),
                b: EleveVue::de(eb.clone(), profil.pour(b)),
            });
        }
        Ok(out)
    }

    /// Nombre de doublons probables à examiner (tuile « qualité des données », ELV-27).
    pub async fn nombre_doublons_a_examiner(&self, etab: Uuid) -> Result<i64, EntDbError> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM ent_eleve_doublon WHERE etablissement_id = $1 AND statut = 'a_examiner'",
        )
        .bind(etab)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    /// ELV-33 : déclarer deux fiches **distinctes** (jumeaux, homonymes, fratrie). La paire n'est
    /// plus proposée tant qu'aucune des deux identités ne change.
    pub async fn declarer_non_doublon(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve_x: Uuid,
        eleve_y: Uuid,
        motif_code: &str,
    ) -> Result<(), EntDbError> {
        if !self.a_action_etablissement(etab, acteur, role, Action::ElevesIdentiteModifier).await? {
            return Err(EntDbError::Forbidden);
        }
        if !["jumeaux", "homonymes", "fratrie", "autre"].contains(&motif_code) {
            return Err(EntDbError::SaisieInvalide("motif de non-doublon inconnu"));
        }
        if eleve_x == eleve_y {
            return Err(EntDbError::SaisieInvalide("deux élèves distincts sont attendus"));
        }
        let (a, b) = paire_ordonnee(eleve_x, eleve_y);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Verrou dans l'ordre des identifiants (pas d'interblocage entre deux déclarations croisées).
        let versions: Vec<(Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, identite_version_id, fusionne_dans FROM ent_eleve
              WHERE etablissement_id = $1 AND id IN ($2, $3) ORDER BY id FOR UPDATE",
        )
        .bind(etab)
        .bind(a)
        .bind(b)
        .fetch_all(&mut *tx)
        .await?;
        if versions.len() != 2 {
            return Err(DomainError::NotFound.into());
        }
        if versions.iter().any(|v| v.2.is_some()) {
            return Err(EntDbError::ConflitDossier("une des fiches est déjà fusionnée"));
        }
        // Une fiche en confidentialité renforcée ne se manipule pas sans G6.
        let profil = self.profil_champs(etab, acteur, role).await?;
        if !profil.ouvert_partout(GroupeChamps::Sensible) {
            let protege: bool = sqlx::query_scalar(
                "SELECT ent_eleve_confidentiel($1, $2) OR ent_eleve_confidentiel($1, $3)",
            )
            .bind(etab)
            .bind(a)
            .bind(b)
            .fetch_one(&mut *tx)
            .await?;
            if protege {
                return Err(DomainError::NotFound.into());
            }
        }
        sqlx::query(
            "INSERT INTO ent_eleve_non_doublon (id, etablissement_id, eleve_a, eleve_b, motif_code,
                                                identite_a, identite_b, decide_par)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (etablissement_id, eleve_a, eleve_b) DO UPDATE
                SET motif_code = EXCLUDED.motif_code, identite_a = EXCLUDED.identite_a,
                    identite_b = EXCLUDED.identite_b, decide_par = EXCLUDED.decide_par, decide_le = now()",
        )
        .bind(etab)
        .bind(a)
        .bind(b)
        .bind(motif_code)
        .bind(versions[0].1)
        .bind(versions[1].1)
        .bind(acteur)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_eleve_doublon SET statut = 'obsolete', maj_le = now()
              WHERE etablissement_id = $1 AND eleve_a = $2 AND eleve_b = $3 AND statut = 'a_examiner'",
        )
        .bind(etab)
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;
        // Événement : identifiants des deux fiches, motif codé — aucune valeur d'identité.
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::ELEVE_NON_DOUBLON_DECLARE.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: a,
            eleve_id: Some(a),
            payload: json!({ "eleves": [a, b], "motif_code": motif_code }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        for e in [a, b] {
            sqlx::query(
                "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                                 payload, visible_famille, actor_user_id, created_at)
                 VALUES ($1,$2,$3,'eleve_sensible','non_doublon_declare',$4,$5,false,$6, now())",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(e)
            .bind("Fiche déclarée distincte d'une autre fiche")
            .bind(json!({ "motif_code": motif_code }))
            .bind(acteur)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
