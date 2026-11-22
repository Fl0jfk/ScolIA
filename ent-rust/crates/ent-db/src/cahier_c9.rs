//! Socle 20, lot C9 : **progressions et reconduction** (`20_CAHIER_DE_TEXTES.md` §16).
//!
//! * Une progression est le plan **privé** de son auteur (étapes : titre, notions, durée en séances). Elle ne s'ouvre qu'à un
//!   cours (ses intervenants à la date, suppléant compris) ou à une discipline par un partage explicite de son auteur ; jamais
//!   à la direction, aux élèves ni aux familles. Partager une progression n'ouvre **jamais** le cahier d'un autre cours : une
//!   progression ne contient que des étapes.
//! * **Planifier** crée des entrées en **brouillon** (jamais publiées) sur les séances libres du cours, à partir d'aujourd'hui ;
//!   les séances qui tombent ensuite passent par les propositions de report du lot C6. Idempotent : une étape déjà planifiée
//!   n'est pas replanifiée.
//! * **Reconduire** copie une progression (sans dates, sans partage) ou les entrées **de l'appelant** d'un ancien cours vers un
//!   cours de même niveau, en brouillon. Jamais de donnée d'élève : ni remise, ni coche, ni contribution, ni travail, ni nom.
//!   Les pièces jointes ne sont pas copiées dans ce lot.
//! * Ordre des contrôles : droit d'écriture du cahier (professeur), puis droit sur l'objet, puis existence.

use chrono::{Duration, NaiveDate};
use ent_auth::Action;
use ent_events::types;
use ent_types::cahier_c9::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::cahier_c2::{conflit, exiger_annee_ouverte_tx, invalide, texte_borne};
use crate::cahier_c8::jour_tx;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

const MAX_ETAPES: usize = 60;
const MAX_PROGRESSIONS: i64 = 300;
const FENETRE_PLANIFICATION_JOURS: i64 = 400;

/// Ce qui est lisible d'une progression pour l'appelant : l'auteur, ou un partage en vigueur avec un de ses cours / sa discipline.
const LISIBLE: &str = "p.auteur_user_id = $3
    OR EXISTS (SELECT 1 FROM ent_cdt_progression_partage s
                WHERE s.etablissement_id = p.etablissement_id AND s.progression_id = p.id AND s.jusqu_au IS NULL
                  AND ((s.cours_id IS NOT NULL AND ent_prof_sur_cours(p.etablissement_id, s.cours_id, $3, $4, false))
                    OR (s.matiere_id IS NOT NULL AND EXISTS (
                          SELECT 1 FROM ent_cours c WHERE c.etablissement_id = p.etablissement_id AND c.matiere_id = s.matiere_id
                             AND ent_prof_sur_cours(p.etablissement_id, c.id, $3, $4, false)))))";

impl PgEnt {
    async fn c9_exiger_prof(&self, a: &Acteur<'_>) -> Result<(), EntDbError> {
        if self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Progression lisible de l'appelant : `Some(mienne)` ; `None` si elle n'existe pas ou ne lui est pas ouverte (même réponse :
    /// on ne révèle pas l'existence d'une progression privée).
    async fn c9_lisible(
        &self,
        a: &Acteur<'_>,
        tx: &mut PgConnection,
        id: Uuid,
    ) -> Result<Option<bool>, EntDbError> {
        let jour = jour_tx(tx, a.etablissement_id).await?;
        let r: Option<(bool,)> = sqlx::query_as(&format!(
            "SELECT p.auteur_user_id = $3 FROM ent_cdt_progression p WHERE p.etablissement_id = $1 AND p.id = $2 AND ({LISIBLE})"
        ))
        .bind(a.etablissement_id)
        .bind(id)
        .bind(a.user_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(r.map(|(m,)| m))
    }

    /// Le professeur intervient-il (aujourd'hui) dans ce cours ?
    async fn c9_intervient(
        &self,
        a: &Acteur<'_>,
        tx: &mut PgConnection,
        cours: Uuid,
    ) -> Result<bool, EntDbError> {
        self.c8_intervenant(a, tx, cours).await
    }

    // ───────────────────────────── lecture et gestion ─────────────────────────────

    /// Mes progressions et celles de collègues partagées avec un de mes cours ou ma discipline.
    pub async fn cdt_c9_progressions(&self, a: &Acteur<'_>) -> Result<ProgressionsDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(&format!(
            "SELECT p.id FROM ent_cdt_progression p
              WHERE p.etablissement_id = $1 AND p.archivee_le IS NULL AND ({LISIBLE}) AND $2::bool
              ORDER BY p.modifie_le DESC, p.id LIMIT $5"
        ))
        .bind(etab)
        .bind(true)
        .bind(a.user_id)
        .bind(jour)
        .bind(MAX_PROGRESSIONS)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = ProgressionsDto {
            mes: Vec::new(),
            partagees: Vec::new(),
        };
        for (id,) in ids {
            let r = resume(&mut tx, etab, id, a.user_id).await?;
            if r.mienne {
                out.mes.push(r);
            } else {
                out.partagees.push(r);
            }
        }
        Ok(out)
    }

    pub async fn cdt_c9_progression_lire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<ProgressionDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let Some(mienne) = self.c9_lisible(a, &mut tx, id).await? else {
            return Err(introuvable());
        };
        detail(&mut tx, a.etablissement_id, id, a.user_id, mienne).await
    }

    async fn c9_valider(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        b: &ProgressionBody,
    ) -> Result<Vec<(Option<Uuid>, String, String, i16)>, EntDbError> {
        let titre = texte_borne(
            &b.titre,
            200,
            true,
            "titre vide ou trop long (200 caractères au plus)",
        )?;
        let _ = titre;
        if b.etapes.is_empty() || b.etapes.len() > MAX_ETAPES {
            return Err(invalide("une progression compte de 1 à 60 étapes"));
        }
        let mut etapes = Vec::new();
        for e in &b.etapes {
            let t = texte_borne(
                &e.titre,
                180,
                true,
                "titre d'étape vide ou trop long (180 caractères au plus)",
            )?;
            let n = texte_borne(
                &e.notions,
                2000,
                false,
                "notions trop longues (2 000 caractères au plus)",
            )?;
            if !(1..=30).contains(&e.duree_seances) {
                return Err(invalide("durée d'une étape : 1 à 30 séances"));
            }
            etapes.push((e.id, t, n, e.duree_seances));
        }
        for (col, table, id) in [
            ("matiere", "ent_matiere", b.matiere_id),
            ("niveau", "ent_niveau", b.niveau_id),
            ("annee", "ent_annee_scolaire", b.annee_id),
        ] {
            if let Some(id) = id {
                let _ = col;
                let (ok,): (bool,) = sqlx::query_as(&format!(
                    "SELECT EXISTS (SELECT 1 FROM {table} WHERE etablissement_id = $1 AND id = $2)"
                ))
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !ok {
                    return Err(introuvable());
                }
            }
        }
        Ok(etapes)
    }

    pub async fn cdt_c9_progression_creer(
        &self,
        a: &Acteur<'_>,
        b: ProgressionBody,
    ) -> Result<ProgressionDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let agent = self.c8_agent_du_compte(a, &mut tx).await?;
        let etapes = self.c9_valider(&mut tx, etab, &b).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_cdt_progression (id, etablissement_id, auteur_user_id, auteur_agent_id, annee_id, titre, matiere_id, niveau_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(a.user_id)
        .bind(agent)
        .bind(b.annee_id)
        .bind(b.titre.trim())
        .bind(b.matiere_id)
        .bind(b.niveau_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (i, (_, t, n, d)) in etapes.iter().enumerate() {
            inserer_etape(&mut tx, etab, id, (i + 1) as i16, t, n, *d).await?;
        }
        let dto = detail(&mut tx, etab, id, a.user_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Modifie une progression (auteur seulement). Les étapes déjà planifiées se conservent par leur identifiant ; une étape
    /// qui porte des entrées ne se supprime pas.
    pub async fn cdt_c9_progression_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        b: ProgressionBody,
    ) -> Result<ProgressionDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some(mienne) = self.c9_lisible(a, &mut tx, id).await? else {
            return Err(introuvable());
        };
        if !mienne {
            return Err(EntDbError::Forbidden);
        }
        let etapes = self.c9_valider(&mut tx, etab, &b).await?;
        let (archivee,): (bool,) = sqlx::query_as("SELECT archivee_le IS NOT NULL FROM ent_cdt_progression WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if archivee {
            return Err(conflit("progression archivée"));
        }
        let existantes: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_cdt_progression_etape WHERE etablissement_id = $1 AND progression_id = $2")
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let existantes: Vec<Uuid> = existantes.into_iter().map(|(i,)| i).collect();
        for (eid, _, _, _) in &etapes {
            if let Some(e) = eid {
                if !existantes.contains(e) {
                    return Err(invalide("étape inconnue dans cette progression"));
                }
            }
        }
        for e in &existantes {
            if !etapes.iter().any(|(i, ..)| *i == Some(*e)) {
                let (utilisee,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_cdt_entree WHERE etablissement_id = $1 AND progression_etape_id = $2)")
                    .bind(etab)
                    .bind(e)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if utilisee {
                    return Err(conflit(
                        "une étape déjà planifiée dans un cahier ne se supprime pas : conservez-la",
                    ));
                }
                sqlx::query(
                    "DELETE FROM ent_cdt_progression_etape WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(e)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        for (i, (eid, t, n, d)) in etapes.iter().enumerate() {
            match eid {
                Some(e) => {
                    sqlx::query("UPDATE ent_cdt_progression_etape SET ordre = $3, titre = $4, notions = $5, duree_seances = $6 WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab)
                        .bind(e)
                        .bind((i + 1) as i16)
                        .bind(t)
                        .bind(n)
                        .bind(d)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                }
                None => inserer_etape(&mut tx, etab, id, (i + 1) as i16, t, n, *d).await?,
            }
        }
        sqlx::query("UPDATE ent_cdt_progression SET titre = $3, matiere_id = $4, niveau_id = $5, annee_id = $6, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(b.titre.trim())
            .bind(b.matiere_id)
            .bind(b.niveau_id)
            .bind(b.annee_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let dto = detail(&mut tx, etab, id, a.user_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Archive une progression (disparaît des listes ; rien n'est supprimé). Auteur seulement.
    pub async fn cdt_c9_progression_archiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some(mienne) = self.c9_lisible(a, &mut tx, id).await? else {
            return Err(introuvable());
        };
        if !mienne {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query("UPDATE ent_cdt_progression SET archivee_le = COALESCE(archivee_le, now()) WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE ent_cdt_progression_partage SET jusqu_au = now() WHERE etablissement_id = $1 AND progression_id = $2 AND jusqu_au IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────────── partage ─────────────────────────────

    /// Partage une progression avec un cours où l'auteur intervient, ou avec une discipline qu'il enseigne.
    pub async fn cdt_c9_partager(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        b: PartageBody,
    ) -> Result<ProgressionDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some(mienne) = self.c9_lisible(a, &mut tx, id).await? else {
            return Err(introuvable());
        };
        if !mienne {
            return Err(EntDbError::Forbidden);
        }
        match (b.cours_id, b.matiere_id) {
            (Some(c), None) => {
                let (existe,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND id = $2)")
                    .bind(etab)
                    .bind(c)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if !existe {
                    return Err(introuvable());
                }
                if !self.c9_intervient(a, &mut tx, c).await? {
                    return Err(EntDbError::Forbidden);
                }
            }
            (None, Some(m)) => {
                let (existe,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_matiere WHERE etablissement_id = $1 AND id = $2)")
                    .bind(etab)
                    .bind(m)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if !existe {
                    return Err(introuvable());
                }
                let (enseigne,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_cours c WHERE c.etablissement_id = $1 AND c.matiere_id = $2 AND ent_prof_sur_cours($1, c.id, $3, $4, false))",
                )
                .bind(etab)
                .bind(m)
                .bind(a.user_id)
                .bind(jour)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !enseigne {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ => {
                return Err(invalide(
                    "indiquez un cours ou une discipline, pas les deux",
                ))
            }
        }
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_progression_partage WHERE etablissement_id = $1 AND progression_id = $2 AND jusqu_au IS NULL
                              AND cours_id IS NOT DISTINCT FROM $3 AND matiere_id IS NOT DISTINCT FROM $4)",
        )
        .bind(etab)
        .bind(id)
        .bind(b.cours_id)
        .bind(b.matiere_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit("déjà partagée avec cette cible"));
        }
        sqlx::query("INSERT INTO ent_cdt_progression_partage (id, etablissement_id, progression_id, cours_id, matiere_id, par_user_id) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(b.cours_id)
            .bind(b.matiere_id)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let dto = detail(&mut tx, etab, id, a.user_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Retire un partage (auteur) : la progression redevient privée pour cette cible, les copies déjà faites restent.
    pub async fn cdt_c9_partage_retirer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        partage_id: Uuid,
    ) -> Result<ProgressionDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some(mienne) = self.c9_lisible(a, &mut tx, id).await? else {
            return Err(introuvable());
        };
        if !mienne {
            return Err(EntDbError::Forbidden);
        }
        let n = sqlx::query("UPDATE ent_cdt_progression_partage SET jusqu_au = now() WHERE etablissement_id = $1 AND progression_id = $2 AND id = $3 AND jusqu_au IS NULL")
            .bind(etab)
            .bind(id)
            .bind(partage_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        let dto = detail(&mut tx, etab, id, a.user_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // ───────────────────────────── planifier ─────────────────────────────

    /// Planifie les étapes d'une progression sur les séances libres d'un cours (entrées en brouillon). Aperçu possible.
    pub async fn cdt_c9_planifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        b: PlanifierBody,
    ) -> Result<PlanDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if self.c9_lisible(a, &mut tx, id).await?.is_none() {
            return Err(introuvable());
        }
        // F3 : droit de saisie sur le cours AVANT son existence (un cours inconnu et un cours sans droit : même refus).
        self.cdt_exiger_saisie(a, b.cours_id, None).await?;
        // F8 : une progression archivée ne se planifie plus (elle se reconduit).
        let (archivee,): (bool,) = sqlx::query_as(
            "SELECT archivee_le IS NOT NULL FROM ent_cdt_progression WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if archivee {
            return Err(conflit(
                "progression archivée : reconduisez-la avant de la planifier",
            ));
        }
        let annee: Option<(Uuid,)> = sqlx::query_as(
            "SELECT annee_id FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(b.cours_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((annee_id,)) = annee else {
            return Err(introuvable());
        };
        exiger_annee_ouverte_tx(&mut tx, etab, annee_id).await?;
        let aujourdhui = jour_tx(&mut tx, etab).await?;
        let debut = b.a_partir_du.unwrap_or(aujourdhui).max(aujourdhui);
        let fin = b
            .jusqu_au
            .unwrap_or(debut + Duration::days(FENETRE_PLANIFICATION_JOURS));
        if fin < debut {
            return Err(invalide("la fin de la fenêtre précède son début"));
        }
        if (fin - debut).num_days() > FENETRE_PLANIFICATION_JOURS {
            return Err(invalide(
                "fenêtre de planification trop longue (400 jours au plus)",
            ));
        }
        let etapes: Vec<(Uuid, String, String, i16)> =
            sqlx::query_as("SELECT id, titre, notions, duree_seances FROM ent_cdt_progression_etape WHERE etablissement_id = $1 AND progression_id = $2 ORDER BY ordre")
                .bind(etab)
                .bind(id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
        let libres = seances_libres(&mut tx, etab, b.cours_id, debut, fin).await?;
        let mut libres = libres.into_iter();
        let mut affectations = Vec::new();
        let mut sans_seance = Vec::new();
        let mut deja = 0i64;
        let mut notions_par_etape = std::collections::BTreeMap::new();
        for (eid, titre, notions, duree) in &etapes {
            let (planifiees,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_cdt_entree WHERE etablissement_id = $1 AND cours_id = $2 AND progression_etape_id = $3 AND etat <> 'retiree'",
            )
            .bind(etab)
            .bind(b.cours_id)
            .bind(eid)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let reste = (*duree as i64 - planifiees).max(0);
            if reste == 0 {
                deja += 1;
                continue;
            }
            notions_par_etape.insert(*eid, notions.clone());
            let mut manque = 0;
            for rang in (planifiees + 1)..=(*duree as i64) {
                match libres.next() {
                    Some((seance_id, jour)) => affectations.push(AffectationDto {
                        seance_id,
                        jour,
                        etape_id: *eid,
                        etape_titre: titre.clone(),
                        rang: rang as i16,
                    }),
                    None => manque += 1,
                }
            }
            if manque > 0 {
                sans_seance.push(format!("{titre} ({manque} séance(s) manquante(s))"));
            }
        }
        let mut creees = 0i64;
        if !b.apercu && !affectations.is_empty() {
            let agent = self.c8_agent_du_compte(a, &mut tx).await?;
            for aff in &affectations {
                let duree = etapes
                    .iter()
                    .find(|e| e.0 == aff.etape_id)
                    .map_or(1, |e| e.3);
                let titre = if duree > 1 {
                    format!("{} ({}/{})", aff.etape_titre, aff.rang, duree)
                } else {
                    aff.etape_titre.clone()
                };
                let notions = notions_par_etape
                    .get(&aff.etape_id)
                    .cloned()
                    .unwrap_or_default();
                let contenu = if notions.trim().is_empty() {
                    format!("À traiter : {}", aff.etape_titre)
                } else {
                    format!("À traiter : {}\nNotions : {}", aff.etape_titre, notions)
                };
                let eid = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, titre, contenu, etat, auteur_user_id, auteur_agent_id, progression_etape_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, 'brouillon', $8, $9, $10)",
                )
                .bind(eid)
                .bind(etab)
                .bind(b.cours_id)
                .bind(aff.seance_id)
                .bind(annee_id)
                .bind(&titre)
                .bind(&contenu)
                .bind(a.user_id)
                .bind(agent)
                .bind(aff.etape_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                let liste = notions
                    .split([';', '\n'])
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .take(20);
                for (k, n) in liste.enumerate() {
                    sqlx::query("INSERT INTO ent_cdt_entree_notion (etablissement_id, entree_id, ordre, libelle) VALUES ($1, $2, $3, $4)")
                        .bind(etab)
                        .bind(eid)
                        .bind((k + 1) as i16)
                        .bind(n.chars().take(200).collect::<String>())
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                }
                creees += 1;
            }
            publier_cdt(
                &mut tx,
                a,
                types::CAHIER_PROGRESSION_PLANIFIEE,
                id,
                None,
                json!({ "cours_id": b.cours_id, "entrees": creees }),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(PlanDto {
            cours_id: b.cours_id,
            progression_id: id,
            apercu: b.apercu,
            affectations,
            creees,
            deja_planifiees: deja,
            sans_seance,
        })
    }

    // ───────────────────────────── reconduire ─────────────────────────────

    /// Copie une progression lisible vers une progression **privée** neuve (sans dates, sans partage, sans lien avec les
    /// entrées de l'ancien cahier). `annee_id` : l'année visée (facultative).
    pub async fn cdt_c9_reconduire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        b: ReconduireBody,
    ) -> Result<ProgressionDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let agent = self.c8_agent_du_compte(a, &mut tx).await?;
        if self.c9_lisible(a, &mut tx, id).await?.is_none() {
            return Err(introuvable());
        }
        if let Some(an) = b.annee_id {
            let (ok,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2)")
                .bind(etab)
                .bind(an)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        let nouvelle = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_cdt_progression (id, etablissement_id, auteur_user_id, auteur_agent_id, annee_id, titre, matiere_id, niveau_id, copie_de_id)
             SELECT $3, etablissement_id, $4, $5, $6, titre, matiere_id, niveau_id, id FROM ent_cdt_progression WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouvelle)
        .bind(a.user_id)
        .bind(agent)
        .bind(b.annee_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_progression_etape (id, etablissement_id, progression_id, ordre, titre, notions, duree_seances)
             SELECT gen_random_uuid(), etablissement_id, $3, ordre, titre, notions, duree_seances
               FROM ent_cdt_progression_etape WHERE etablissement_id = $1 AND progression_id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouvelle)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let dto = detail(&mut tx, etab, nouvelle, a.user_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Copie les entrées **de l'appelant** d'un cours (souvent l'ancien) vers un cours où il écrit, en brouillon, sur les
    /// séances libres à partir d'aujourd'hui. Aucune autre donnée n'est copiée. Aperçu possible.
    pub async fn cdt_c9_copier_cours(
        &self,
        a: &Acteur<'_>,
        b: CopierCoursBody,
    ) -> Result<CopieCoursDto, EntDbError> {
        self.c9_exiger_prof(a).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let cible: Option<(Uuid,)> = sqlx::query_as(
            "SELECT annee_id FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(b.cible_cours_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (source_ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(b.source_cours_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // F3 : droit de saisie sur le cours cible AVANT toute existence (cible ou source inconnues : même refus).
        self.cdt_exiger_saisie(a, b.cible_cours_id, None).await?;
        // Ordre des contrôles : personnel, droit, puis la forme de la demande (jamais avant le droit).
        if b.source_cours_id == b.cible_cours_id {
            return Err(invalide(
                "le cours source et le cours de destination sont identiques",
            ));
        }
        let Some((annee_id,)) = cible else {
            return Err(introuvable());
        };
        if !source_ok {
            return Err(EntDbError::Forbidden);
        }
        exiger_annee_ouverte_tx(&mut tx, etab, annee_id).await?;
        // R4 : n'a le droit de recopier que celui qui a enseigné ce cours (à n'importe quelle date), et seulement ses entrées.
        let (a_enseigne,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM ent_service_enseignement s
                 WHERE s.etablissement_id = $1 AND s.cours_id = $2
                   AND (EXISTS (SELECT 1 FROM ent_service_intervenant i
                                  JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
                                  JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                                 WHERE i.etablissement_id = s.etablissement_id AND i.service_id = s.id AND i.annule_le IS NULL
                                   AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant') AND pe.user_id = $3)
                        OR s.prof_user_id = $3))",
        )
        .bind(etab)
        .bind(b.source_cours_id)
        .bind(a.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !a_enseigne {
            return Err(EntDbError::Forbidden);
        }
        // F7 : la copie ne se fait qu'entre cours du même niveau (le niveau de la classe du cours ; deux niveaux inconnus, ou
        // deux cours de groupes sans niveau, restent comparables).
        let (meme_niveau,): (bool,) = sqlx::query_as(
            "SELECT (SELECT cl.niveau_id FROM ent_cours c LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
                      WHERE c.etablissement_id = $1 AND c.id = $2)
                    IS NOT DISTINCT FROM
                    (SELECT cl.niveau_id FROM ent_cours c LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
                      WHERE c.etablissement_id = $1 AND c.id = $3)",
        )
        .bind(etab)
        .bind(b.source_cours_id)
        .bind(b.cible_cours_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !meme_niveau {
            return Err(conflit("la copie ne se fait qu'entre cours du même niveau"));
        }
        let aujourdhui = jour_tx(&mut tx, etab).await?;
        let debut = b.a_partir_du.unwrap_or(aujourdhui).max(aujourdhui);
        let fin = debut + Duration::days(FENETRE_PLANIFICATION_JOURS);
        #[allow(clippy::type_complexity)]
        let sources: Vec<(Uuid, Option<String>, String)> = sqlx::query_as(
            "SELECT e.id, e.titre, e.contenu FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.auteur_user_id = $3 AND e.etat IN ('brouillon', 'programmee', 'publiee')
              ORDER BY s.jour, s.heure_debut, e.id LIMIT 400",
        )
        .bind(etab)
        .bind(b.source_cours_id)
        .bind(a.user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (autrui,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_cdt_entree WHERE etablissement_id = $1 AND cours_id = $2 AND etat <> 'retiree' AND auteur_user_id IS DISTINCT FROM $3",
        )
        .bind(etab)
        .bind(b.source_cours_id)
        .bind(a.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let libres = seances_libres(&mut tx, etab, b.cible_cours_id, debut, fin).await?;
        let mut copiees = Vec::new();
        let agent = if b.apercu {
            None
        } else {
            self.c8_agent_du_compte(a, &mut tx).await?
        };
        let n_sources = sources.len();
        for ((src, titre, contenu), (seance, jour)) in sources.into_iter().zip(libres) {
            if !b.apercu {
                let nouvelle = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, titre, contenu, etat, auteur_user_id, auteur_agent_id, copie_de_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, 'brouillon', $8, $9, $10)",
                )
                .bind(nouvelle)
                .bind(etab)
                .bind(b.cible_cours_id)
                .bind(seance)
                .bind(annee_id)
                .bind(&titre)
                .bind(&contenu)
                .bind(a.user_id)
                .bind(agent)
                .bind(src)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                sqlx::query(
                    "INSERT INTO ent_cdt_entree_notion (etablissement_id, entree_id, ordre, libelle)
                     SELECT etablissement_id, $3, ordre, libelle FROM ent_cdt_entree_notion WHERE etablissement_id = $1 AND entree_id = $2",
                )
                .bind(etab)
                .bind(src)
                .bind(nouvelle)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            copiees.push(CopieEntreeDto {
                entree_source_id: src,
                titre,
                seance_cible_id: seance,
                jour_cible: jour,
            });
        }
        if !b.apercu && !copiees.is_empty() {
            publier_cdt(
                &mut tx,
                a,
                types::CAHIER_COURS_COPIE,
                b.cible_cours_id,
                None,
                json!({ "source": b.source_cours_id, "entrees": copiees.len() }),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(CopieCoursDto {
            apercu: b.apercu,
            sans_seance: (n_sources - copiees.len()) as i64,
            copiees,
            ignorees_autrui: autrui,
        })
    }
}

/// Séances non annulées d'un cours, sans entrée, hors jours fermés, sur la fenêtre, dans l'ordre chronologique.
async fn seances_libres(
    tx: &mut PgConnection,
    etab: Uuid,
    cours: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<Vec<(Uuid, NaiveDate)>, EntDbError> {
    sqlx::query_as(
        "SELECT s.id, s.jour FROM ent_edt_seance s
          WHERE s.etablissement_id = $1 AND s.jour BETWEEN $3 AND $4 AND s.statut <> 'annulee'
            AND ent_cdt_cours_de_seance($1, s.id) = $2
            AND NOT ent_cdt_jour_ferme($1, s.jour)
            AND NOT EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.seance_id = s.id)
          ORDER BY s.jour, s.heure_debut, s.id LIMIT 500",
    )
    .bind(etab)
    .bind(cours)
    .bind(du)
    .bind(au)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)
}

async fn inserer_etape(
    tx: &mut PgConnection,
    etab: Uuid,
    progression: Uuid,
    ordre: i16,
    titre: &str,
    notions: &str,
    duree: i16,
) -> Result<(), EntDbError> {
    sqlx::query("INSERT INTO ent_cdt_progression_etape (id, etablissement_id, progression_id, ordre, titre, notions, duree_seances) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(progression)
        .bind(ordre)
        .bind(titre)
        .bind(notions)
        .bind(duree)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    Ok(())
}

async fn resume(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    user: Uuid,
) -> Result<ProgressionResumeDto, EntDbError> {
    #[allow(clippy::type_complexity)]
    let r: (String, Uuid, Option<Uuid>, Option<Uuid>, chrono::DateTime<chrono::Utc>, i64, i64, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT p.titre, p.auteur_user_id, p.annee_id, p.copie_de_id, p.modifie_le,
                (SELECT count(*) FROM ent_cdt_progression_etape e WHERE e.etablissement_id = p.etablissement_id AND e.progression_id = p.id),
                (SELECT COALESCE(sum(duree_seances), 0) FROM ent_cdt_progression_etape e WHERE e.etablissement_id = p.etablissement_id AND e.progression_id = p.id),
                (SELECT libelle FROM ent_matiere m WHERE m.etablissement_id = p.etablissement_id AND m.id = p.matiere_id),
                (SELECT libelle FROM ent_niveau n WHERE n.etablissement_id = p.etablissement_id AND n.id = p.niveau_id)
           FROM ent_cdt_progression p WHERE p.etablissement_id = $1 AND p.id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let auteur = crate::cahier_c8::nom(tx, etab, Some(r.1)).await?;
    Ok(ProgressionResumeDto {
        id,
        titre: r.0,
        auteur,
        mienne: r.1 == user,
        nb_etapes: r.5 as i32,
        nb_seances: r.6 as i32,
        matiere_libelle: r.7,
        niveau_libelle: r.8,
        annee_id: r.2,
        copie_de_id: r.3,
        modifie_le: r.4,
    })
}

async fn detail(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    user: Uuid,
    mienne: bool,
) -> Result<ProgressionDto, EntDbError> {
    let resume = resume(tx, etab, id, user).await?;
    let (matiere_id, niveau_id): (Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT matiere_id, niveau_id FROM ent_cdt_progression WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
    let etapes: Vec<(Uuid, i16, String, String, i16)> =
        sqlx::query_as("SELECT id, ordre, titre, notions, duree_seances FROM ent_cdt_progression_etape WHERE etablissement_id = $1 AND progression_id = $2 ORDER BY ordre")
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
    let partages = if mienne {
        let l: Vec<(Uuid, Option<Uuid>, Option<String>, Option<Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            "SELECT s.id, s.cours_id, c.libelle, s.matiere_id, m.libelle, s.depuis
               FROM ent_cdt_progression_partage s
               LEFT JOIN ent_cours c ON c.etablissement_id = s.etablissement_id AND c.id = s.cours_id
               LEFT JOIN ent_matiere m ON m.etablissement_id = s.etablissement_id AND m.id = s.matiere_id
              WHERE s.etablissement_id = $1 AND s.progression_id = $2 AND s.jusqu_au IS NULL ORDER BY s.depuis, s.id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        l.into_iter()
            .map(
                |(id, cours_id, cours_libelle, matiere_id, matiere_libelle, depuis)| PartageDto {
                    id,
                    cours_id,
                    cours_libelle,
                    matiere_id,
                    matiere_libelle,
                    depuis,
                },
            )
            .collect()
    } else {
        Vec::new()
    };
    Ok(ProgressionDto {
        resume,
        matiere_id,
        niveau_id,
        etapes: etapes
            .into_iter()
            .map(|(id, ordre, titre, notions, duree_seances)| EtapeDto {
                id,
                ordre,
                titre,
                notions,
                duree_seances,
            })
            .collect(),
        partages,
    })
}
