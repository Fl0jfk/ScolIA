//! Socle 21, lot V5 « parcours » : résultats externes (CE-02, CE-06, CE-07), enseignements suivis hors de l'établissement
//! (R-V38) et parcours d'un élève dans une période (CE-01 à CE-24).
//!
//! * **Une note appartient à l'élève et à la date** (règle 6) : le moteur range une note dans la période du **bulletin**
//!   (`eva_periode_dans`, SQL de 477), jamais dans celle de la classe qu'avait l'élève ce jour-là ; la classe de
//!   rattachement (`eva_classe_rattachement`) est la seule qui produit le bulletin et l'effectif (D15).
//! * **Un résultat externe ne compte pas par défaut** : le compter dans UN cours est une décision tracée (auteur, date,
//!   motif) du professeur du cours (titulaire ou co-enseignant, pas un suppléant) ou de la direction ; la publier à la
//!   famille est un second geste ; après l'arrêt de la période de l'élève, seule la direction (`notes.corriger`).
//! * **Ordre des contrôles** (règle 5) : personnel, puis droit, puis visibilité de l'élève, puis existence et état ; une
//!   ligne inconnue répond comme un droit refusé à qui ne porte pas `notes.corriger` (aucun oracle d'existence).
//! * **Rien d'écrit sans attribution** : chaque ligne porte son auteur ; une moyenne externe corrigée est journalisée
//!   (ajout seul) ; rien ne se supprime (retrait motivé).
//! * **Jamais de motif d'absence ni de donnée de santé** dans le parcours.

use chrono::NaiveDate;
use ent_auth::Action;
use ent_types::evaluations_v5::{
    CompterExterneBody, DeclarerEnseignementExterneBody, DeclarerResultatExterneBody,
    EnseignementExterneDto, EnseignementSuiviDto, EvaluationParcoursDto, MoyenneExterneDto,
    ParcoursPeriodeDto, PublierMoyenneExterneBody, ResultatExterneDto, ResultatExterneFamilleDto,
    RetirerExterneBody, RetirerMoyenneExterneBody, SaisirMoyenneExterneBody, SegmentClasseDto,
    MENTION_AUTRE_ORGANISME, ORGANISMES_EXTERNES, ORIGINES_EXTERNES,
};
use sqlx::postgres::PgRow;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus, texte_borne};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Jour sans objet : les actions réservées à `notes.corriger` ne lisent aucun public.
const SANS_JOUR: NaiveDate = NaiveDate::MIN;

const COLONNES_EXTERNE: &str = "x.id, x.eleve_id, x.matiere_id, m.libelle AS matiere_libelle, x.origine, x.libelle,
    x.date_resultat, x.periode_id, (x.valeur * 1000)::bigint AS valeur_m, (x.bareme * 1000)::bigint AS bareme_m,
    (x.coef * 1000)::bigint AS coef_m, x.compte_dans_cours_id, x.compte_motif, (x.publiee_le IS NOT NULL) AS publiee,
    (x.retiree_le IS NOT NULL) AS retiree, x.motif_retrait";

fn dto_externe(r: &PgRow) -> Result<ResultatExterneDto, EntDbError> {
    Ok(ResultatExterneDto {
        id: r.try_get("id").map_err(db)?,
        eleve_id: r.try_get("eleve_id").map_err(db)?,
        matiere_id: r.try_get("matiere_id").map_err(db)?,
        matiere_libelle: r.try_get("matiere_libelle").map_err(db)?,
        origine: r.try_get("origine").map_err(db)?,
        libelle: r.try_get("libelle").map_err(db)?,
        date_resultat: r.try_get("date_resultat").map_err(db)?,
        periode_id: r.try_get("periode_id").map_err(db)?,
        valeur_m: r.try_get("valeur_m").map_err(db)?,
        bareme_m: r.try_get("bareme_m").map_err(db)?,
        coef_m: r.try_get("coef_m").map_err(db)?,
        compte_dans_cours_id: r.try_get("compte_dans_cours_id").map_err(db)?,
        compte_motif: r.try_get("compte_motif").map_err(db)?,
        publiee: r.try_get("publiee").map_err(db)?,
        retiree: r.try_get("retiree").map_err(db)?,
        motif_retrait: r.try_get("motif_retrait").map_err(db)?,
    })
}

fn motif_valide(m: &str) -> Result<String, EntDbError> {
    let m = m.trim();
    let n = m.chars().count();
    if !(3..=500).contains(&n) {
        return Err(refus("motif de 3 à 500 caractères requis"));
    }
    texte_borne(m, 500)
}

/// Ce que le droit personnel sur un élève ouvre (règle 3 : moindre privilège).
struct Droit {
    /// `notes.corriger` : la direction.
    corriger: bool,
    /// Portée établissement sur l'une des actions demandées (ou direction) : voit et agit sur toutes les matières.
    /// Sinon (un professeur), le droit se limite aux cours qu'il enseigne et à leurs matières.
    large: bool,
}

/// Cours et matières qu'un professeur enseigne au jour de référence.
struct Enseigne {
    matieres: Vec<Uuid>,
    cours: Vec<Uuid>,
}

impl Droit {
    fn matiere_ok(&self, e: &Enseigne, m: Uuid) -> bool {
        self.large || e.matieres.contains(&m)
    }
    fn cours_ok(&self, e: &Enseigne, c: Uuid) -> bool {
        self.large || e.cours.contains(&c)
    }
}

/// Un motif ou un statut `dispense` ne se servent qu'à la direction (jamais d'indice sur la santé ni sur un autre cours).
fn masquer_externe(mut x: ResultatExterneDto, corriger: bool) -> ResultatExterneDto {
    if !corriger {
        x.compte_motif = None;
        x.motif_retrait = None;
    }
    x
}

impl PgEnt {
    /// Cours (et matières) que l'acteur enseigne au jour `jour` : lecture ou saisie sur le cours.
    async fn eva_v5_enseigne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        jour: NaiveDate,
        saisie: bool,
    ) -> Result<Enseigne, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT c.id, c.matiere_id FROM ent_cours c
               JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND y.date_debut <= $3 AND y.date_fin >= $3
                AND ent_prof_sur_cours($1, c.id, $2, $3, $4)",
        )
        .bind(etab)
        .bind(acteur)
        .bind(jour)
        .bind(saisie)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut e = Enseigne {
            matieres: Vec::new(),
            cours: Vec::new(),
        };
        for r in &rows {
            e.cours.push(r.try_get("id").map_err(db)?);
            if let Some(m) = r.try_get::<Option<Uuid>, _>("matiere_id").map_err(db)? {
                e.matieres.push(m);
            }
        }
        Ok(e)
    }

    /// Droit personnel sur un élève, puis garde de visibilité (règle 5). Rend vrai si l'acteur porte `notes.corriger`.
    ///
    /// Le droit vient, dans l'ordre : de la portée établissement d'une des actions demandées ; d'un cours **dont le public
    /// du jour** `jour` compte l'élève et que l'acteur enseigne (professeur ou co-enseignant : pour `notes.saisir` et
    /// `notes.lire` seulement) ; enfin des portées désignées du socle 3 (`autoriser_eleve_le`). Le jour est explicite : aucune
    /// horloge n'intervient dans le droit du professeur.
    async fn eva_v5_droit_eleve(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
        actions: &[Action],
    ) -> Result<Droit, EntDbError> {
        Self::exiger_staff(role)?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        let mut autorise = corriger && actions.contains(&Action::NotesCorriger);
        for a in actions {
            if !autorise && self.a_action_etablissement(etab, acteur, role, *a).await? {
                autorise = true;
            }
        }
        // Portée établissement (ou direction) : pas de limite de matière. Sinon : les cours du professeur seulement.
        let large = autorise || corriger;
        // F-2 : un geste d'ÉCRITURE (aucune action de lecture demandée) exige la saisie EFFECTIVE sur le cours ; un
        // assistant ou un intervenant (lecture seule) ne déclare rien.
        let saisie = !actions.contains(&Action::NotesLire);
        if !autorise && actions.iter().any(|a| *a != Action::NotesCorriger) {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            autorise = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $2, NULL, $3, $4, $5))",
            )
            .bind(etab)
            .bind(jour)
            .bind(acteur)
            .bind(eleve)
            .bind(saisie)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
        }
        if !autorise {
            // Gestes de la direction (`SANS_JOUR`, aucune date métier) : jour de l'établissement, comme avant.
            let jour_droit = if jour == SANS_JOUR {
                self.aujourdhui_etablissement(etab).await?
            } else {
                jour
            };
            for a in actions {
                if self
                    .autoriser_eleve_le(etab, acteur, role, *a, eleve, jour_droit)
                    .await?
                {
                    autorise = true;
                    break;
                }
            }
        }
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        Ok(Droit { corriger, large })
    }

    /// Une ligne externe : introuvable pour qui porte `notes.corriger`, droit refusé pour tout autre (aucun oracle).
    async fn eva_v5_charger_externe(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<PgRow, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_EXTERNE}, x.saisi_par, x.retiree_le FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        match r {
            Some(r) => Ok(r),
            None => Err(
                if self
                    .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
                    .await?
                {
                    introuvable()
                } else {
                    EntDbError::Forbidden
                },
            ),
        }
    }

    /// Droit de décider sur un cours : professeur du cours (pas un suppléant) ou `notes.corriger`.
    async fn eva_v5_droit_decision_cours(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let a = match self.eva_v3_acces(etab, acteur, role, cours, jour).await {
            Ok(a) => a,
            // F1 : un cours inconnu ne se distingue pas d'un cours qu'on n'a pas le droit de décider (existence en dernier).
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)) => {
                return Err(
                    if self
                        .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
                        .await?
                    {
                        introuvable()
                    } else {
                        EntDbError::Forbidden
                    },
                )
            }
            Err(e) => return Err(e),
        };
        if !(a.corriger || (a.saisie && !a.suppleant)) {
            return Err(EntDbError::Forbidden);
        }
        Ok(a.corriger)
    }

    /// Garde de période (M1), la MÊME pour compter, publier et retirer un résultat compté : tant que la période de
    /// l'élève est ouverte, le professeur du cours décide ; une fois arrêtée, la direction seule (motif exigé par
    /// l'appelant). La date de référence est celle du résultat, à défaut le début de sa période.
    async fn eva_v5_garde_periode(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        eleve: Uuid,
        date_resultat: Option<NaiveDate>,
        periode_id: Option<Uuid>,
        a_la_direction: bool,
    ) -> Result<(), EntDbError> {
        let date_ref = match (date_resultat, periode_id) {
            (Some(d), _) => Some(d),
            (None, Some(p)) => sqlx::query_scalar(
                "SELECT date_debut FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(p)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?,
            (None, None) => None,
        };
        if let Some(d) = date_ref {
            let etat: Option<String> =
                sqlx::query_scalar("SELECT etat FROM eva_periode_eleve_jour($1, $2, $3)")
                    .bind(etab)
                    .bind(eleve)
                    .bind(d)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(db)?;
            if etat.as_deref() != Some("ouverte") && !a_la_direction {
                return Err(refus(
                    "période de l'élève arrêtée pour ce résultat : la direction décide, avec un motif",
                ));
            }
        }
        Ok(())
    }

    /// Déclare un résultat externe. Naît non compté, non publié. Secrétariat, professeur ou direction (`notes.saisir`
    /// ou `notes.corriger` sur l'élève) ; élève protégé : introuvable.
    pub async fn eva_v5_declarer_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &DeclarerResultatExterneBody,
    ) -> Result<ResultatExterneDto, EntDbError> {
        let droit = self
            .eva_v5_droit_eleve(
                etab,
                acteur,
                role,
                body.eleve_id,
                jour,
                &[Action::NotesSaisir, Action::NotesCorriger],
            )
            .await?;
        // Moindre privilège : un professeur ne déclare que dans une matière qu'il enseigne.
        if !droit.large
            && !self
                .eva_v5_enseigne(etab, acteur, jour, true)
                .await?
                .matieres
                .contains(&body.matiere_id)
        {
            return Err(EntDbError::Forbidden);
        }
        // F-2 : un professeur ne déclare que pour l'année scolaire en cours (celle de `jour`) ; un résultat daté d'une
        // année précédente, ou rangé dans une de ses périodes, revient à la direction (la période y est arrêtée).
        if !droit.large {
            let reference: Option<NaiveDate> = match (body.date_resultat, body.periode_id) {
                (Some(d), _) => Some(d),
                (None, Some(p)) => {
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    let d = sqlx::query_scalar(
                        "SELECT date_debut FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(p)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
                    tx.commit().await.map_err(db)?;
                    d
                }
                (None, None) => None,
            };
            if let Some(d) = reference {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let annee_courante: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire
                                     WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
                                       AND date_debut <= $3 AND date_fin >= $3)",
                )
                .bind(etab)
                .bind(jour)
                .bind(d)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                tx.commit().await.map_err(db)?;
                if !annee_courante {
                    return Err(refus(
                        "résultat d'une année précédente : seule la direction le déclare",
                    ));
                }
            }
        }
        if !ORIGINES_EXTERNES.contains(&body.origine.as_str()) {
            return Err(refus(
                "origine inconnue : etablissement, cned, etranger, reprise ou autre",
            ));
        }
        if body.bareme_m <= 0 || body.bareme_m > 1_000_000 {
            return Err(refus("barème hors limites"));
        }
        if body.valeur_m < 0 || body.valeur_m > body.bareme_m {
            return Err(refus("valeur hors du barème"));
        }
        let coef_m = body.coef_m.unwrap_or(1000);
        if !(0..=100_000).contains(&coef_m) {
            return Err(refus("coefficient hors limites"));
        }
        if body.date_resultat.is_none() && body.periode_id.is_none() {
            return Err(refus("la date du résultat (ou sa période) est requise"));
        }
        if body.date_resultat.is_some_and(|d| d > jour) {
            return Err(refus("la date du résultat est dans le futur"));
        }
        let libelle = match body.libelle.as_deref() {
            Some(l) if !l.trim().is_empty() => Some(texte_borne(l, 160)?),
            _ => None,
        };
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let matiere_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_matiere WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(body.matiere_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !matiere_ok {
            return Err(refus("matière inconnue"));
        }
        // M5 : la pièce appartient à l'établissement ET à l'élève (jamais l'identifiant d'un autre dossier).
        let piece_ok: bool = sqlx::query_scalar("SELECT eva_document_de_l_eleve($1, $2, $3)")
            .bind(etab)
            .bind(body.eleve_id)
            .bind(body.document_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !piece_ok {
            return Err(refus("pièce inconnue pour cet élève"));
        }
        if let Some(p) = body.periode_id {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_periode WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(p)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(refus("période inconnue"));
            }
        }
        sqlx::query(
            "INSERT INTO eva_resultat_externe (id, etablissement_id, eleve_id, matiere_id, periode_id, origine, valeur, bareme,
                                               coef, document_id, saisi_par, date_resultat, libelle)
             VALUES ($1, $2, $3, $4, $5, $6, ($7::bigint)::numeric / 1000, ($8::bigint)::numeric / 1000,
                     ($9::bigint)::numeric / 1000, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(etab)
        .bind(body.eleve_id)
        .bind(body.matiere_id)
        .bind(body.periode_id)
        .bind(&body.origine)
        .bind(body.valeur_m)
        .bind(body.bareme_m)
        .bind(coef_m)
        .bind(body.document_id)
        .bind(acteur)
        .bind(body.date_resultat)
        .bind(&libelle)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_EXTERNE} FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_externe(&r)
    }

    /// Décide de **compter** un résultat externe dans la moyenne d'UN cours (CE-02) : tracé, motivé, une fois.
    pub async fn eva_v5_compter_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        id: Uuid,
        body: &CompterExterneBody,
    ) -> Result<ResultatExterneDto, EntDbError> {
        let l = self.eva_v5_charger_externe(etab, acteur, role, id).await?;
        let eleve: Uuid = l.try_get("eleve_id").map_err(db)?;
        let matiere: Uuid = l.try_get("matiere_id").map_err(db)?;
        let a_la_direction = self
            .eva_v5_droit_decision_cours(etab, acteur, role, body.cours_id, jour)
            .await?;
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        let motif = motif_valide(&body.motif)?;
        if l.try_get::<bool, _>("retiree").map_err(db)? {
            return Err(conflit("ce résultat a été retiré"));
        }
        if l.try_get::<Option<Uuid>, _>("compte_dans_cours_id")
            .map_err(db)?
            .is_some()
        {
            return Err(conflit(
                "la décision est déjà prise : retirer le résultat puis le redéclarer pour la changer",
            ));
        }
        let date_resultat: Option<NaiveDate> = l.try_get("date_resultat").map_err(db)?;
        let periode_id: Option<Uuid> = l.try_get("periode_id").map_err(db)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cours_matiere: Option<Uuid> = sqlx::query_scalar(
            "SELECT matiere_id FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(body.cours_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if cours_matiere != Some(matiere) {
            return Err(refus("ce cours n'enseigne pas la matière du résultat"));
        }
        let suit: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $2, $3, NULL, $4))",
        )
        .bind(etab)
        .bind(jour)
        .bind(body.cours_id)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !suit {
            return Err(refus("l'élève ne suit pas ce cours aujourd'hui"));
        }
        // Après l'arrêt de la période de l'élève, compter un résultat change une moyenne arrêtée : direction seulement.
        Self::eva_v5_garde_periode(
            &mut tx,
            etab,
            eleve,
            date_resultat,
            periode_id,
            a_la_direction,
        )
        .await?;
        let n = sqlx::query(
            "UPDATE eva_resultat_externe
                SET compte_dans_cours_id = $3, compte_decide_par = $4, compte_decide_le = now(), compte_motif = $5
              WHERE etablissement_id = $1 AND id = $2 AND compte_dans_cours_id IS NULL AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(body.cours_id)
        .bind(acteur)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("le résultat a changé depuis votre lecture"));
        }
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_EXTERNE} FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_externe(&r)
    }

    /// Publie à la famille et à l'élève un résultat externe **déjà compté** (second geste, distinct de la décision).
    pub async fn eva_v5_publier_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        id: Uuid,
    ) -> Result<ResultatExterneDto, EntDbError> {
        let l = self.eva_v5_charger_externe(etab, acteur, role, id).await?;
        let eleve: Uuid = l.try_get("eleve_id").map_err(db)?;
        let Some(cours) = l
            .try_get::<Option<Uuid>, _>("compte_dans_cours_id")
            .map_err(db)?
        else {
            // Droit sur l'élève d'abord : un non-compté ne se devine pas par un autre message.
            self.eva_v5_droit_eleve(
                etab,
                acteur,
                role,
                eleve,
                jour,
                &[Action::NotesSaisir, Action::NotesCorriger],
            )
            .await?;
            return Err(conflit(
                "seul un résultat compté se publie : décider d'abord de le compter",
            ));
        };
        let a_la_direction = self
            .eva_v5_droit_decision_cours(etab, acteur, role, cours, jour)
            .await?;
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        if l.try_get::<bool, _>("retiree").map_err(db)? {
            return Err(conflit("ce résultat a été retiré"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // M1 : publier un résultat compté change ce que la famille lit d'une période ; même garde que « compter ».
        Self::eva_v5_garde_periode(
            &mut tx,
            etab,
            eleve,
            l.try_get("date_resultat").map_err(db)?,
            l.try_get("periode_id").map_err(db)?,
            a_la_direction,
        )
        .await?;
        let n = sqlx::query(
            "UPDATE eva_resultat_externe SET publiee_le = now(), publiee_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND publiee_le IS NULL AND retiree_le IS NULL
                AND compte_dans_cours_id IS NOT NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit(
                "le résultat est déjà publié ou a changé depuis votre lecture",
            ));
        }
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_EXTERNE} FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_externe(&r)
    }

    /// Retire un résultat externe (motivé, une fois). L'auteur ou la direction ; un résultat compté se retire par le
    /// professeur du cours ou la direction.
    pub async fn eva_v5_retirer_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        id: Uuid,
        body: &RetirerExterneBody,
    ) -> Result<ResultatExterneDto, EntDbError> {
        let l = self.eva_v5_charger_externe(etab, acteur, role, id).await?;
        let eleve: Uuid = l.try_get("eleve_id").map_err(db)?;
        let auteur: Uuid = l.try_get("saisi_par").map_err(db)?;
        let corriger = self
            .eva_v5_droit_eleve(
                etab,
                acteur,
                role,
                eleve,
                jour,
                &[Action::NotesSaisir, Action::NotesCorriger],
            )
            .await?
            .corriger;
        let compte = l
            .try_get::<Option<Uuid>, _>("compte_dans_cours_id")
            .map_err(db)?;
        let a_la_direction = if let Some(cours) = compte {
            self.eva_v5_droit_decision_cours(etab, acteur, role, cours, jour)
                .await?
        } else if !(corriger || auteur == acteur) {
            return Err(EntDbError::Forbidden);
        } else {
            corriger
        };
        let motif = motif_valide(&body.motif)?;
        if l.try_get::<bool, _>("retiree").map_err(db)? {
            return Err(conflit("ce résultat est déjà retiré"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // M1 : retirer un résultat COMPTÉ change une moyenne ; même garde de période que « compter ».
        if compte.is_some() {
            Self::eva_v5_garde_periode(
                &mut tx,
                etab,
                eleve,
                l.try_get("date_resultat").map_err(db)?,
                l.try_get("periode_id").map_err(db)?,
                a_la_direction,
            )
            .await?;
        }
        let n = sqlx::query(
            "UPDATE eva_resultat_externe SET retiree_le = now(), retiree_par = $3, motif_retrait = $4
              WHERE etablissement_id = $1 AND id = $2 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("le résultat a changé depuis votre lecture"));
        }
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_EXTERNE} FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_externe(&r)
    }

    async fn eva_v5_externes_eleve_tx(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<ResultatExterneDto>, EntDbError> {
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES_EXTERNE} FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
              ORDER BY x.date_resultat NULLS LAST, x.cree_le, x.id"
        ))
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        rows.iter().map(dto_externe).collect()
    }

    /// Résultats externes d'un élève, vue du **personnel** (`notes.lire` sur l'élève, élève visible).
    pub async fn eva_v5_externes_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
    ) -> Result<Vec<ResultatExterneDto>, EntDbError> {
        let droit = self
            .eva_v5_droit_eleve(etab, acteur, role, eleve, jour, &[Action::NotesLire])
            .await?;
        let enseigne = if droit.large {
            None
        } else {
            Some(self.eva_v5_enseigne(etab, acteur, jour, false).await?)
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let v = Self::eva_v5_externes_eleve_tx(&mut tx, etab, eleve).await?;
        tx.commit().await.map_err(db)?;
        // Moindre privilège : un professeur ne lit que les matières qu'il enseigne ; les motifs, à la direction seule.
        Ok(v.into_iter()
            .filter(|x| {
                enseigne
                    .as_ref()
                    .is_none_or(|e| droit.matiere_ok(e, x.matiere_id))
            })
            .map(|x| masquer_externe(x, droit.corriger))
            .collect())
    }

    /// Résultats externes vus par la **famille** ou l'élève : publiés, comptés, non retirés ; ni auteur ni motif.
    pub async fn eva_v5_externes_famille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<ResultatExterneFamilleDto>, EntDbError> {
        if !self.actor_can_read_notes(etab, acteur, role, eleve).await? {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT x.id, m.libelle AS matiere_libelle, x.libelle, x.date_resultat,
                    (x.valeur * 1000)::bigint AS valeur_m, (x.bareme * 1000)::bigint AS bareme_m
               FROM eva_resultat_externe x
               JOIN ent_matiere m ON m.id = x.matiere_id AND m.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                AND x.publiee_le IS NOT NULL
                AND x.compte_dans_cours_id IS NOT NULL AND x.retiree_le IS NULL
              ORDER BY x.date_resultat NULLS LAST, x.id",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        rows.iter()
            .map(|r| {
                Ok(ResultatExterneFamilleDto {
                    id: r.try_get("id").map_err(db)?,
                    matiere_libelle: r.try_get("matiere_libelle").map_err(db)?,
                    libelle: r.try_get("libelle").map_err(db)?,
                    date_resultat: r.try_get("date_resultat").map_err(db)?,
                    valeur_m: r.try_get("valeur_m").map_err(db)?,
                    bareme_m: r.try_get("bareme_m").map_err(db)?,
                    mention: MENTION_AUTRE_ORGANISME.to_string(),
                })
            })
            .collect()
    }

    // ───────────────────────── Enseignements suivis hors de l'établissement ─────────────────────────

    /// Déclare un enseignement suivi ailleurs (CNED, conservatoire, étranger…). Direction (`notes.corriger`).
    pub async fn eva_v5_declarer_enseignement_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &DeclarerEnseignementExterneBody,
    ) -> Result<EnseignementExterneDto, EntDbError> {
        self.eva_v5_droit_eleve(
            etab,
            acteur,
            role,
            body.eleve_id,
            SANS_JOUR,
            &[Action::NotesCorriger],
        )
        .await?;
        if !ORGANISMES_EXTERNES.contains(&body.organisme.as_str()) {
            return Err(refus(
                "organisme inconnu : cned, etablissement, conservatoire, etranger ou autre",
            ));
        }
        if body.au.is_some_and(|a| a < body.du) {
            return Err(refus("la fin précède le début"));
        }
        let libelle = match body.organisme_libelle.as_deref() {
            Some(l) if !l.trim().is_empty() => Some(texte_borne(l, 160)?),
            _ => None,
        };
        let referent = match body.referent.as_deref() {
            Some(l) if !l.trim().is_empty() => Some(texte_borne(l, 160)?),
            _ => None,
        };
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let matiere_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_matiere WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(body.matiere_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !matiere_ok {
            return Err(refus("matière inconnue"));
        }
        sqlx::query(
            "INSERT INTO eva_enseignement_externe (id, etablissement_id, eleve_id, matiere_id, organisme, organisme_libelle,
                                                   du, au, referent, compte_moyenne_generale, declare_par)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(etab)
        .bind(body.eleve_id)
        .bind(body.matiere_id)
        .bind(&body.organisme)
        .bind(&libelle)
        .bind(body.du)
        .bind(body.au)
        .bind(&referent)
        .bind(body.compte_moyenne_generale)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let v = Self::eva_v5_enseignement_tx(&mut tx, etab, id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// `voit_dispense` : seule la direction lit un statut `dispense` (jamais d'indice de santé hors direction).
    async fn eva_v5_enseignement_tx(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        id: Uuid,
        voit_dispense: bool,
    ) -> Result<EnseignementExterneDto, EntDbError> {
        let e = sqlx::query(
            "SELECT id, eleve_id, matiere_id, organisme, organisme_libelle, du, au, compte_moyenne_generale
               FROM eva_enseignement_externe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        let ms = sqlx::query(
            "SELECT id, periode_id, statut, (valeur * 1000)::bigint AS valeur_m, (bareme * 1000)::bigint AS bareme_m,
                    appreciation_transcrite, (publiee_le IS NOT NULL) AS publiee
               FROM eva_moyenne_externe WHERE etablissement_id = $1 AND enseignement_externe_id = $2
                AND ($3 OR statut <> 'dispense')
              ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(id)
        .bind(voit_dispense)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        Ok(EnseignementExterneDto {
            id: e.try_get("id").map_err(db)?,
            eleve_id: e.try_get("eleve_id").map_err(db)?,
            matiere_id: e.try_get("matiere_id").map_err(db)?,
            organisme: e.try_get("organisme").map_err(db)?,
            organisme_libelle: e.try_get("organisme_libelle").map_err(db)?,
            du: e.try_get("du").map_err(db)?,
            au: e.try_get("au").map_err(db)?,
            compte_moyenne_generale: e.try_get("compte_moyenne_generale").map_err(db)?,
            moyennes: ms
                .iter()
                .map(|m| {
                    Ok(MoyenneExterneDto {
                        id: m.try_get("id").map_err(db)?,
                        periode_id: m.try_get("periode_id").map_err(db)?,
                        statut: m.try_get("statut").map_err(db)?,
                        valeur_m: m.try_get("valeur_m").map_err(db)?,
                        bareme_m: m.try_get("bareme_m").map_err(db)?,
                        appreciation_transcrite: m
                            .try_get("appreciation_transcrite")
                            .map_err(db)?,
                        publiee: m.try_get("publiee").map_err(db)?,
                    })
                })
                .collect::<Result<_, EntDbError>>()?,
        })
    }

    /// Saisit, ou corrige (journal en ajout seul), la moyenne d'un enseignement externe pour une période. Direction.
    pub async fn eva_v5_saisir_moyenne_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        enseignement_id: Uuid,
        body: &SaisirMoyenneExterneBody,
    ) -> Result<EnseignementExterneDto, EntDbError> {
        Self::exiger_staff(role)?;
        // Existence : inconnue = droit refusé pour qui ne porte pas `notes.corriger` (aucun oracle).
        let eleve: Option<Uuid> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let e = sqlx::query_scalar(
                "SELECT eleve_id FROM eva_enseignement_externe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(enseignement_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            e
        };
        let Some(eleve) = eleve else {
            return Err(
                if self
                    .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
                    .await?
                {
                    introuvable()
                } else {
                    EntDbError::Forbidden
                },
            );
        };
        self.eva_v5_droit_eleve(
            etab,
            acteur,
            role,
            eleve,
            SANS_JOUR,
            &[Action::NotesCorriger],
        )
        .await?;
        let bareme_m = body.bareme_m.unwrap_or(20_000);
        if bareme_m <= 0 || bareme_m > 1_000_000 {
            return Err(refus("barème hors limites"));
        }
        match (body.statut.as_str(), body.valeur_m) {
            ("moyenne", Some(v)) if (0..=bareme_m).contains(&v) => {}
            ("moyenne", _) => {
                return Err(refus(
                    "une moyenne porte une valeur comprise dans le barème",
                ))
            }
            ("en_attente" | "dispense", None) => {}
            ("en_attente" | "dispense", Some(_)) => {
                return Err(refus("ce statut ne porte aucune valeur"))
            }
            _ => return Err(refus("statut inconnu : moyenne, en_attente ou dispense")),
        }
        let appreciation = match body.appreciation_transcrite.as_deref() {
            Some(a) if !a.trim().is_empty() => Some(texte_borne(a, 600)?),
            _ => None,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // F3 : la période visée est une période de l'année et de la périodisation de l'élève (celle d'une de ses classes),
        // et touche les dates de l'enseignement suivi ailleurs.
        let periode_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM ent_periode p
                   JOIN eva_enseignement_externe ens ON ens.etablissement_id = p.etablissement_id AND ens.id = $3
                  WHERE p.etablissement_id = $1 AND p.id = $2
                    AND p.date_debut <= COALESCE(ens.au, 'infinity'::date) AND p.date_fin >= ens.du
                    AND EXISTS (SELECT 1 FROM ent_affectation a
                                 WHERE a.etablissement_id = p.etablissement_id AND a.eleve_id = ens.eleve_id
                                   AND a.annee_id = p.annee_id AND a.classe_id IS NOT NULL
                                   AND ent_periodisation_pour_classe(a.etablissement_id, a.classe_id) = p.periodisation_id))",
        )
        .bind(etab)
        .bind(body.periode_id)
        .bind(enseignement_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !periode_ok {
            return Err(refus(
                "période inconnue, ou hors de l'année et de la périodisation de l'élève, ou hors des dates de l'enseignement",
            ));
        }
        // M5 : la pièce appartient à l'établissement ET à l'élève.
        let piece_ok: bool = sqlx::query_scalar("SELECT eva_document_de_l_eleve($1, $2, $3)")
            .bind(etab)
            .bind(eleve)
            .bind(body.document_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !piece_ok {
            return Err(refus("pièce inconnue pour cet élève"));
        }
        type Existante = (Uuid, i64, String, Option<i64>, Option<String>, Option<Uuid>);
        let existante: Option<Existante> = sqlx::query_as(
            "SELECT id, (bareme * 1000)::bigint, statut, (valeur * 1000)::bigint, appreciation_transcrite, document_id
               FROM eva_moyenne_externe
              WHERE etablissement_id = $1 AND enseignement_externe_id = $2 AND periode_id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(enseignement_id)
        .bind(body.periode_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match existante {
            Some((m, bareme_actuel, statut, valeur, apprec, document)) => {
                if bareme_actuel != bareme_m {
                    return Err(refus("le barème d'une moyenne externe ne change pas"));
                }
                // Rien ne change : aucune écriture, aucun journal, aucun motif exigé.
                let change = statut != body.statut
                    || valeur != body.valeur_m
                    || apprec != appreciation
                    || document != body.document_id;
                if change {
                    // M3 : une correction porte un motif de 3 à 500 caractères, journalisé avec l'auteur.
                    let motif = match body.motif.as_deref() {
                        Some(m) => motif_valide(m)?,
                        None => return Err(refus("motif de 3 à 500 caractères requis")),
                    };
                    sqlx::query(
                        "UPDATE eva_moyenne_externe
                            SET statut = $3, valeur = ($4::bigint)::numeric / 1000, appreciation_transcrite = $5,
                                document_id = $6, saisi_par = $7, motif_correction = $8
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(m)
                    .bind(&body.statut)
                    .bind(body.valeur_m)
                    .bind(&appreciation)
                    .bind(body.document_id)
                    .bind(acteur)
                    .bind(&motif)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
            }
            None => {
                // F4 : deux saisies concurrentes de la même période : la seconde reçoit un conflit, jamais une erreur 500.
                let n = sqlx::query(
                    "INSERT INTO eva_moyenne_externe (id, etablissement_id, enseignement_externe_id, periode_id, valeur, bareme,
                                                      appreciation_transcrite, statut, document_id, saisi_par)
                     VALUES ($1, $2, $3, $4, ($5::bigint)::numeric / 1000, ($6::bigint)::numeric / 1000, $7, $8, $9, $10)
                     ON CONFLICT (enseignement_externe_id, periode_id) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(enseignement_id)
                .bind(body.periode_id)
                .bind(body.valeur_m)
                .bind(bareme_m)
                .bind(&appreciation)
                .bind(&body.statut)
                .bind(body.document_id)
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?
                .rows_affected();
                if n == 0 {
                    return Err(conflit(
                        "une moyenne vient d'être saisie pour cette période : relire, puis corriger avec un motif",
                    ));
                }
            }
        }
        let v = Self::eva_v5_enseignement_tx(&mut tx, etab, enseignement_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Charge une moyenne externe pour un geste de la direction (publier, retirer la publication) : ligne inconnue =
    /// droit refusé pour qui ne porte pas `notes.corriger` (aucun oracle d'existence), puis droit sur l'élève.
    async fn eva_v5_charger_moyenne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        moyenne_id: Uuid,
    ) -> Result<(Uuid, Uuid), EntDbError> {
        Self::exiger_staff(role)?;
        let ligne: Option<(Uuid, Uuid)> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let e = sqlx::query_as(
                "SELECT m.enseignement_externe_id, ens.eleve_id FROM eva_moyenne_externe m
                   JOIN eva_enseignement_externe ens ON ens.etablissement_id = m.etablissement_id
                    AND ens.id = m.enseignement_externe_id
                  WHERE m.etablissement_id = $1 AND m.id = $2",
            )
            .bind(etab)
            .bind(moyenne_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            e
        };
        let Some((enseignement, eleve)) = ligne else {
            return Err(
                if self
                    .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
                    .await?
                {
                    introuvable()
                } else {
                    EntDbError::Forbidden
                },
            );
        };
        self.eva_v5_droit_eleve(
            etab,
            acteur,
            role,
            eleve,
            SANS_JOUR,
            &[Action::NotesCorriger],
        )
        .await?;
        Ok((enseignement, eleve))
    }

    /// Publie à la famille et à l'élève une moyenne externe saisie (second geste, distinct de la saisie ; une fois).
    /// Direction (`notes.corriger`) : c'est elle qui a déclaré que la moyenne externe vaut moyenne de matière.
    pub async fn eva_v5_publier_moyenne_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        moyenne_id: Uuid,
        body: &PublierMoyenneExterneBody,
    ) -> Result<EnseignementExterneDto, EntDbError> {
        let vue = &body.vue;
        let (enseignement, _eleve) = self
            .eva_v5_charger_moyenne(etab, acteur, role, moyenne_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE eva_moyenne_externe SET publiee_le = now(), publiee_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND publiee_le IS NULL AND statut = 'moyenne'
                -- Compare-et-écrit : on ne publie que la version que la direction a vue.
                AND statut = $4 AND (valeur * 1000)::bigint IS NOT DISTINCT FROM $5 AND (bareme * 1000)::bigint = $6",
        )
        .bind(etab)
        .bind(moyenne_id)
        .bind(acteur)
        .bind(&vue.statut)
        .bind(vue.valeur_m)
        .bind(vue.bareme_m)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit(
                "la moyenne est déjà publiée, n'est pas une moyenne, ou a été corrigée depuis votre lecture : relisez-la",
            ));
        }
        let v = Self::eva_v5_enseignement_tx(&mut tx, etab, enseignement, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Retire la publication d'une moyenne externe (M-A, D-069) : la famille et l'élève ne la voient plus. Direction
    /// (`notes.corriger`), motif de 3 à 500 caractères obligatoire, tracé au journal (événement « retrait »). Une
    /// correction de la valeur d'une moyenne publiée fait le même effet sans geste distinct (voir 478).
    pub async fn eva_v5_retirer_publication_moyenne_externe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        moyenne_id: Uuid,
        body: &RetirerMoyenneExterneBody,
    ) -> Result<EnseignementExterneDto, EntDbError> {
        let vue = &body.vue;
        let (enseignement, _eleve) = self
            .eva_v5_charger_moyenne(etab, acteur, role, moyenne_id)
            .await?;
        let motif = motif_valide(&body.motif)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE eva_moyenne_externe
                SET publiee_le = NULL, publiee_par = NULL, motif_correction = $3, retrait_par = $7
              WHERE etablissement_id = $1 AND id = $2 AND publiee_le IS NOT NULL
                -- Compare-et-écrit : on ne retire que la version que la direction a vue.
                AND statut = $4 AND (valeur * 1000)::bigint IS NOT DISTINCT FROM $5 AND (bareme * 1000)::bigint = $6",
        )
        .bind(etab)
        .bind(moyenne_id)
        .bind(&motif)
        .bind(&vue.statut)
        .bind(vue.valeur_m)
        .bind(vue.bareme_m)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit(
                "la moyenne n'est pas publiée, ou a été corrigée depuis votre lecture : relisez-la",
            ));
        }
        let v = Self::eva_v5_enseignement_tx(&mut tx, etab, enseignement, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Enseignements externes d'un élève (personnel : `notes.lire` sur l'élève).
    pub async fn eva_v5_enseignements_externes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
    ) -> Result<Vec<EnseignementExterneDto>, EntDbError> {
        let droit = self
            .eva_v5_droit_eleve(etab, acteur, role, eleve, jour, &[Action::NotesLire])
            .await?;
        let enseigne = self.eva_v5_enseigne(etab, acteur, jour, false).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, matiere_id FROM eva_enseignement_externe
              WHERE etablissement_id = $1 AND eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2)) ORDER BY du, id",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut v = Vec::new();
        for (id, matiere) in ids {
            // Moindre privilège : un professeur ne lit que les matières qu'il enseigne.
            if droit.matiere_ok(&enseigne, matiere) {
                v.push(Self::eva_v5_enseignement_tx(&mut tx, etab, id, droit.corriger).await?);
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Parcours d'un élève dans une période ─────────────────────────

    /// Le parcours d'un élève dans une période (`ent_periode`) : classes traversées et classe de rattachement, arrivée,
    /// départ, enseignements suivis (option abandonnée), évaluations de ses cours et la place qu'elles y tiennent
    /// (concerné ou non à leur date), résultats externes. Personnel seulement (`notes.lire` sur l'élève, élève visible).
    pub async fn eva_v5_parcours_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
        periode_id: Uuid,
    ) -> Result<ParcoursPeriodeDto, EntDbError> {
        let droit = self
            .eva_v5_droit_eleve(etab, acteur, role, eleve, jour, &[Action::NotesLire])
            .await?;
        let enseigne = self.eva_v5_enseigne(etab, acteur, jour, false).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (debut, fin, annee): (NaiveDate, NaiveDate, Uuid) = sqlx::query_as(
            "SELECT date_debut, date_fin, annee_id FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let rattachement: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT c.id, c.code FROM ent_classe c
              WHERE c.etablissement_id = $1 AND c.id = eva_classe_rattachement($1, $2, $3, $4)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(debut)
        .bind(fin)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let segments: Vec<SegmentClasseDto> = sqlx::query(
            "SELECT a.classe_id, c.code, a.date_debut, a.date_fin
               FROM ent_affectation a JOIN ent_classe c ON c.id = a.classe_id AND c.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.classe_id IS NOT NULL
                AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $3)
              ORDER BY a.date_debut, a.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            Ok(SegmentClasseDto {
                classe_id: r.try_get("classe_id").map_err(db)?,
                classe_code: r.try_get("code").map_err(db)?,
                du: r.try_get("date_debut").map_err(db)?,
                au: r.try_get("date_fin").map_err(db)?,
            })
        })
        .collect::<Result<_, EntDbError>>()?;
        let premiere: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT min(date_debut) FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3",
        )
        .bind(etab)
        .bind(eleve)
        .bind(annee)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let arrive_le = premiere.filter(|d| *d > debut && *d <= fin);
        let sortie: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT date_sortie FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let parti_le = sortie.filter(|d| *d >= debut && *d <= fin);
        // Enseignements : cours de la classe ou du groupe de l'élève (affectations) ou d'une inclusion, sur la période.
        let enseignements: Vec<(EnseignementSuiviDto, Option<Uuid>)> = sqlx::query(
            "WITH suivis AS (
                 SELECT c.id AS cours_id, c.matiere_id, c.classe_id AS classe_cours, c.groupe_id,
                        greatest(a.date_debut, c.ouvert_le, $3) AS du,
                        least(a.date_fin, c.ferme_le) AS au,
                        a.classe_id AS classe_aff
                   FROM ent_cours c
                   JOIN ent_affectation a ON a.etablissement_id = c.etablissement_id AND a.annee_id = c.annee_id
                    AND ((c.classe_id IS NOT NULL AND a.classe_id = c.classe_id)
                      OR (c.groupe_id IS NOT NULL AND a.groupe_id = c.groupe_id))
                  WHERE c.etablissement_id = $1 AND a.eleve_id = $2 AND c.annee_id = $5
                    AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                    AND c.ouvert_le <= $4 AND (c.ferme_le IS NULL OR c.ferme_le >= $3)
                 UNION ALL
                 SELECT c.id, c.matiere_id, c.classe_id, c.groupe_id, greatest(m.du, c.ouvert_le, $3),
                        least(m.au, c.ferme_le),
                        NULL::uuid
                   FROM ent_cours_membre m JOIN ent_cours c ON c.id = m.cours_id AND c.etablissement_id = m.etablissement_id
                  WHERE m.etablissement_id = $1 AND m.eleve_id = $2 AND m.nature = 'inclusion' AND m.annulee_le IS NULL
                    AND c.annee_id = $5 AND m.du <= $4 AND (m.au IS NULL OR m.au >= $3)
                    AND c.ouvert_le <= $4 AND (c.ferme_le IS NULL OR c.ferme_le >= $3))
             SELECT s.cours_id, mt.libelle AS matiere_libelle, cl.code AS classe_code, s.classe_cours AS classe_cours_id, s.du, s.au,
                    -- Option abandonnée : le suivi s'arrête dans la période alors que l'élève reste dans la MÊME classe.
                    CASE WHEN s.au IS NOT NULL AND s.au < $4 AND (s.groupe_id IS NOT NULL OR s.classe_aff IS NULL)
                              AND EXISTS (SELECT 1 FROM ent_affectation z
                                           WHERE z.etablissement_id = $1 AND z.eleve_id = $2 AND z.classe_id IS NOT NULL
                                             AND z.date_debut <= s.au + 1 AND (z.date_fin IS NULL OR z.date_fin > s.au)
                                             AND z.classe_id = eva_classe_rattachement($1, $2, $3, $4))
                              AND NOT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.etablissement_id = $1 AND e.id = $2
                                                 AND e.date_sortie IS NOT NULL AND e.date_sortie <= s.au)
                         THEN s.au END AS abandonne_le
               FROM suivis s
               JOIN ent_matiere mt ON mt.id = s.matiere_id AND mt.etablissement_id = $1
               LEFT JOIN ent_classe cl ON cl.id = s.classe_cours AND cl.etablissement_id = $1
              ORDER BY mt.libelle, s.du, s.cours_id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(debut)
        .bind(fin)
        .bind(annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            Ok((
                EnseignementSuiviDto {
                    cours_id: r.try_get("cours_id").map_err(db)?,
                    matiere_libelle: r.try_get("matiere_libelle").map_err(db)?,
                    classe_code: r.try_get("classe_code").map_err(db)?,
                    du: r.try_get("du").map_err(db)?,
                    au: r.try_get("au").map_err(db)?,
                    abandonne_le: r.try_get("abandonne_le").map_err(db)?,
                },
                r.try_get::<Option<Uuid>, _>("classe_cours_id").map_err(db)?,
            ))
        })
        .collect::<Result<Vec<(EnseignementSuiviDto, Option<Uuid>)>, EntDbError>>()?
        .into_iter()
        // Moindre privilège (M2) : un professeur ne voit que ses propres cours (et leurs matières), jamais ceux des autres.
        .filter(|(e, _)| droit.cours_ok(&enseigne, e.cours_id))
        .collect();
        let classe_du_cours: Vec<Option<Uuid>> = enseignements.iter().map(|(_, c)| *c).collect();
        let enseignements: Vec<EnseignementSuiviDto> =
            enseignements.into_iter().map(|(e, _)| e).collect();
        // F-1 (moindre privilège) : un professeur ne voit du parcours que ce qui concerne SES cours. Fenêtre = période de
        // ses cours ; segments de classe, rattachement, arrivée et départ ne sont servis que pour les classes où il
        // enseigne et dans cette fenêtre (jamais l'historique de classes d'un élève dans d'autres classes). Un cours de
        // groupe sans classe ne justifie aucun segment. Sans cours, rien : la réponse reste vide (200), jamais un indice.
        let (segments, rattachement, arrive_le, parti_le) = if droit.large {
            (segments, rattachement, arrive_le, parti_le)
        } else {
            // Période d'enseignement RÉELLE de l'acteur par cours (intervenance : du/au) ; un cours sans ligne
            // d'intervenance (professeur du service seul) n'est pas borné.
            let cours_pf: Vec<Uuid> = enseignements.iter().map(|e| e.cours_id).collect();
            let bornes: std::collections::HashMap<Uuid, (NaiveDate, Option<NaiveDate>)> =
                sqlx::query(
                    "SELECT s.cours_id, min(i.du) AS du,
                            CASE WHEN bool_or(i.au IS NULL) THEN NULL ELSE max(i.au) END AS au
                       FROM ent_service_enseignement s
                       JOIN ent_service_intervenant i ON i.etablissement_id = s.etablissement_id AND i.service_id = s.id
                       JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
                       JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                      WHERE s.etablissement_id = $1 AND s.cours_id = ANY($2) AND pe.user_id = $3
                        AND i.annule_le IS NULL
                        AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant', 'intervenant', 'assistant')
                      GROUP BY s.cours_id",
                )
                .bind(etab)
                .bind(&cours_pf)
                .bind(acteur)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?
                .iter()
                .map(|r| {
                    Ok((
                        r.try_get::<Uuid, _>("cours_id").map_err(db)?,
                        (
                            r.try_get::<NaiveDate, _>("du").map_err(db)?,
                            r.try_get::<Option<NaiveDate>, _>("au").map_err(db)?,
                        ),
                    ))
                })
                .collect::<Result<_, EntDbError>>()?;
            // F-1 : fenêtre PAR CLASSE (identifiant, jamais le code), bornée par la période où l'acteur enseigne
            // réellement dans cette classe (présence de l'élève ∩ cours ∩ intervenance), pas par l'enveloppe de ses cours.
            let mut fenetres: Vec<(Uuid, NaiveDate, NaiveDate)> = Vec::new();
            for (e, classe) in enseignements.iter().zip(&classe_du_cours) {
                let Some(classe) = classe else { continue };
                let (mut a, mut b) = (e.du, e.au.unwrap_or(fin));
                if let Some((pdu, pau)) = bornes.get(&e.cours_id) {
                    a = a.max(*pdu);
                    if let Some(pau) = pau {
                        b = b.min(*pau);
                    }
                }
                if a > b {
                    continue;
                }
                match fenetres.iter_mut().find(|f| f.0 == *classe) {
                    Some(f) => {
                        f.1 = f.1.min(a);
                        f.2 = f.2.max(b);
                    }
                    None => fenetres.push((*classe, a, b)),
                }
            }
            let dedans = |d: &NaiveDate| fenetres.iter().any(|(_, a, b)| *d >= *a && *d <= *b);
            let segments: Vec<SegmentClasseDto> = segments
                .into_iter()
                .filter_map(|mut sg| {
                    let (_, a, b) = *fenetres.iter().find(|f| f.0 == sg.classe_id)?;
                    if sg.du > b || sg.au.is_some_and(|x| x < a) {
                        return None;
                    }
                    sg.du = sg.du.max(a);
                    sg.au = Some(sg.au.map_or(b, |x| x.min(b)));
                    Some(sg)
                })
                .collect();
            (
                segments,
                rattachement.filter(|r| fenetres.iter().any(|f| f.0 == r.0)),
                arrive_le.filter(dedans),
                parti_le.filter(dedans),
            )
        };
        // Évaluations des cours de l'élève dans la période, et sa place à leur date.
        let cours_ids: Vec<Uuid> = enseignements.iter().map(|e| e.cours_id).collect();
        let evaluations: Vec<EvaluationParcoursDto> = sqlx::query(
            "SELECT ev.id, ev.titre, ev.date_eval, mt.libelle AS matiere_libelle, cl.code AS classe_code,
                    EXISTS (SELECT 1 FROM ent_public_cours_jour($1, ev.date_eval, ev.cours_id, NULL, $2)) AS concerne,
                    r.statut
               FROM eva_evaluation ev
               JOIN ent_cours c ON c.id = ev.cours_id AND c.etablissement_id = ev.etablissement_id
               JOIN ent_matiere mt ON mt.id = c.matiere_id AND mt.etablissement_id = c.etablissement_id
               LEFT JOIN ent_classe cl ON cl.id = c.classe_id AND cl.etablissement_id = c.etablissement_id
               LEFT JOIN eva_resultat r ON r.etablissement_id = ev.etablissement_id AND r.evaluation_id = ev.id AND r.eleve_id = $2
              WHERE ev.etablissement_id = $1 AND ev.cours_id = ANY($5) AND ev.mode = 'note' AND ev.etat = 'ouverte'
                AND ev.date_eval BETWEEN $3 AND $4
              ORDER BY ev.date_eval, ev.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(debut)
        .bind(fin)
        .bind(&cours_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            Ok(EvaluationParcoursDto {
                evaluation_id: r.try_get("id").map_err(db)?,
                titre: r.try_get("titre").map_err(db)?,
                date_eval: r.try_get("date_eval").map_err(db)?,
                matiere_libelle: r.try_get("matiere_libelle").map_err(db)?,
                classe_code: r.try_get("classe_code").map_err(db)?,
                concerne: r.try_get("concerne").map_err(db)?,
                // Un statut `dispense` ne se sert qu'à la direction.
                statut: r
                    .try_get::<Option<String>, _>("statut")
                    .map_err(db)?
                    .filter(|st| droit.corriger || st != "dispense"),
            })
        })
        .collect::<Result<_, EntDbError>>()?;
        let externes: Vec<ResultatExterneDto> =
            Self::eva_v5_externes_eleve_tx(&mut tx, etab, eleve)
                .await?
                .into_iter()
                .filter(|x| {
                    x.date_resultat
                        .map(|d| d >= debut && d <= fin)
                        .unwrap_or(x.periode_id == Some(periode_id))
                })
                .filter(|x| droit.matiere_ok(&enseigne, x.matiere_id))
                .map(|x| masquer_externe(x, droit.corriger))
                .collect();
        tx.commit().await.map_err(db)?;
        Ok(ParcoursPeriodeDto {
            eleve_id: eleve,
            periode_id,
            classe_rattachement_id: rattachement.as_ref().map(|r| r.0),
            classe_rattachement_code: rattachement.map(|r| r.1),
            segments,
            arrive_le,
            parti_le,
            enseignements,
            evaluations,
            resultats_externes: externes,
        })
    }
}
