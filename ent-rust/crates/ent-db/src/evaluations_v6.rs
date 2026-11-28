//! Socle 21, lot V6 « appréciations » (`docs/socles/21_EVALUATIONS.md` §12, §17.10 ; EVA-194 à EVA-206 hors IA).
//!
//! * **Une appréciation vivante par clé** (période, élève, type, cours) ; chaque enregistrement est une **version**
//!   (`eva_appreciation_historique`, déclencheurs de 465) dont l'éditeur est gardé (`modifie_par`).
//! * **Un brouillon n'est montré qu'à son auteur** : ni la série, ni la relecture, ni l'historique ne lisent le
//!   brouillon d'un autre (on dit seulement qu'il existe, sur la clé de l'acteur, pour expliquer un refus).
//! * **Fenêtre** : l'état de la période de la **classe de l'élève** (`eva_periode_eleve_jour`, R-V29) décide. Cours :
//!   jusqu'à la clôture des appréciations (rang < 2) ; intermédiaire : période ouverte (rang 0) ; générale, vie
//!   scolaire, dispositif : jusqu'au conseil (rang < 3). Après, la direction (`notes.corriger`) corrige seule, avec un
//!   motif journalisé (`eva_appreciation_correction`), jusqu'à la clôture de la période (rang < 4).
//! * **Droits** (règle 3, D-069 faute d'actions dédiées au catalogue) : appréciation de cours ou intermédiaire →
//!   professeur en saisie sur le cours (titulaire, co-enseignant, suppléant) pour un élève du cours dans la période ;
//!   générale → professeur principal de la classe de l'élève ou `bulletins.publier` ; vie scolaire → CPE ou
//!   `bulletins.publier` ; dispositif → `bulletins.publier` ; partout, `notes.corriger`. Personnel seulement.
//! * **Ordre des contrôles** (règle 5) : personnel, droit, visibilité de l'élève (socle 2), puis existence et état ;
//!   une ligne inconnue répond comme un droit refusé à qui ne porte pas `notes.corriger` (aucun oracle).
//! * **Contrôles de cohérence** non bloquants (`ent_types::evaluations_v6::controler`).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::{Action, Role};
use ent_types::evaluations_v6::{
    cle_identique, controler, limite_caracteres, type_avec_cours, AjouterMotInterditBody,
    AjouterPhraseBody, AppreciationV6Dto, AppreciationsEleveDto, ContexteControle, ControleDto,
    JeterBrouillonBody, LigneSerieDto, MotInterditDto, PhraseBanqueDto, RemarqueBody, RemarqueDto,
    SaisieAppreciationDto, SaisirAppreciationBody, SerieCoursDto, VersionAppreciationDto,
    MAX_MOTS_INTERDITS, MAX_PHRASES_BANQUE, TYPES_APPRECIATION,
};
use sqlx::postgres::PgRow;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus, texte_borne};
use crate::evaluations_v1::moyennes_tx;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

const COLONNES: &str =
    "a.id, a.periode_id, a.eleve_id, a.type, a.cours_id, a.texte, a.etat, a.version, a.aide_ia,
    COALESCE(a.modifie_par, a.auteur) AS editeur, a.maj_le";

/// Rang d'un état de période (même échelle que `eva_etat_rang`, 463).
fn rang(etat: &str) -> i32 {
    match etat {
        "ouverte" => 0,
        "notes_arretees" => 1,
        "appreciations_closes" => 2,
        "conseil_tenu" => 3,
        _ => 4,
    }
}

/// Rang à partir duquel la fenêtre d'un type est fermée à son auteur.
fn fin_fenetre(type_appreciation: &str) -> i32 {
    match type_appreciation {
        "intermediaire" => 1,
        "cours" => 2,
        _ => 3,
    }
}

/// Ligne lue (l'éditeur reste interne).
struct Lue {
    dto: AppreciationV6Dto,
    editeur: Uuid,
}

fn lue(r: &PgRow, acteur: Uuid) -> Result<Lue, EntDbError> {
    let texte: String = r.try_get("texte").map_err(db)?;
    let type_appreciation: String = r.try_get("type").map_err(db)?;
    let editeur: Uuid = r.try_get("editeur").map_err(db)?;
    Ok(Lue {
        dto: AppreciationV6Dto {
            id: r.try_get("id").map_err(db)?,
            periode_id: r.try_get("periode_id").map_err(db)?,
            eleve_id: r.try_get("eleve_id").map_err(db)?,
            cours_id: r.try_get("cours_id").map_err(db)?,
            longueur: texte.chars().count(),
            limite: limite_caracteres(&type_appreciation),
            texte,
            type_appreciation,
            etat: r.try_get("etat").map_err(db)?,
            version: r.try_get("version").map_err(db)?,
            aide_ia: r.try_get("aide_ia").map_err(db)?,
            de_moi: editeur == acteur,
            maj_le: r.try_get("maj_le").map_err(db)?,
        },
        editeur,
    })
}

/// Un brouillon ne se montre qu'à son éditeur ; un texte jeté à personne.
fn visible(l: &Lue, acteur: Uuid) -> bool {
    match l.dto.etat.as_str() {
        "validee" => true,
        "brouillon" => l.editeur == acteur,
        _ => false,
    }
}

/// Période (bornes) et situation de l'élève dans cette période : classe et état, au jour de référence (le jour de
/// l'établissement ramené dans la période), sinon à la fin de la période.
struct Situation {
    debut: NaiveDate,
    fin: NaiveDate,
    jour_ref: NaiveDate,
    classe_id: Option<Uuid>,
    etat: Option<String>,
}

async fn situation_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    periode: Uuid,
    jour: NaiveDate,
) -> Result<Situation, EntDbError> {
    let p: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(periode)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let Some((debut, fin)) = p else {
        return Err(introuvable());
    };
    let jour_ref = jour.clamp(debut, fin);
    let mut s = Situation {
        debut,
        fin,
        jour_ref,
        classe_id: None,
        etat: None,
    };
    for j in [jour_ref, fin] {
        let r: Option<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT classe_id, periode_id, etat FROM eva_periode_eleve_jour($1, $2, $3)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(j)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        if let Some((c, p, e)) = r {
            if p == periode {
                s.classe_id = Some(c);
                s.etat = Some(e);
                break;
            }
        }
    }
    Ok(s)
}

/// L'acteur est-il professeur principal de la classe au jour donné ?
async fn est_pp_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    classe: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s
                          JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                         WHERE s.etablissement_id = $1 AND s.prof_user_id = $2 AND s.classe_id = $3
                           AND s.professeur_principal AND y.date_debut <= $4 AND y.date_fin >= $4)",
    )
    .bind(etab)
    .bind(acteur)
    .bind(classe)
    .bind(jour)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)
}

/// L'élève relève-t-il du cours pendant la période : public au jour de référence, ou public d'une évaluation du cours
/// datée dans la période (même définition que la grille V3) ?
async fn eleve_du_cours_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cours: Uuid,
    eleve: Uuid,
    s: &Situation,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $2, $3, NULL, $4))
             OR EXISTS (SELECT 1 FROM eva_evaluation e
                          CROSS JOIN LATERAL eva_grille_eleves(e.etablissement_id, e.id) g
                         WHERE e.etablissement_id = $1 AND e.cours_id = $3 AND e.mode = 'note'
                           AND e.etat IN ('ouverte', 'annulee') AND e.date_eval BETWEEN $5 AND $6
                           AND g.eleve_id = $4)",
    )
    .bind(etab)
    .bind(s.jour_ref)
    .bind(cours)
    .bind(eleve)
    .bind(s.debut)
    .bind(s.fin)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)
}

fn motif_correction(m: Option<&str>) -> Result<String, EntDbError> {
    let m = m.map(str::trim).unwrap_or("");
    if !(10..=500).contains(&m.chars().count()) {
        return Err(refus(
            "fenêtre fermée : une correction de la direction exige un motif de 10 à 500 caractères",
        ));
    }
    texte_borne(m, 500)
}

fn texte_appreciation(texte: &str, type_appreciation: &str) -> Result<String, EntDbError> {
    let t = texte.trim();
    if t.is_empty() {
        return Err(refus("appréciation vide"));
    }
    if t.chars().count() > limite_caracteres(type_appreciation) {
        return Err(refus(if type_appreciation == "generale" {
            "appréciation trop longue : 600 caractères au plus (jamais tronquée)"
        } else {
            "appréciation trop longue : 400 caractères au plus (jamais tronquée)"
        }));
    }
    texte_borne(t, limite_caracteres(type_appreciation))
}

impl PgEnt {
    async fn eva_v6_inconnu(&self, etab: Uuid, acteur: Uuid, role: &str) -> EntDbError {
        match self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await
        {
            Ok(true) => introuvable(),
            Ok(false) => EntDbError::Forbidden,
            Err(e) => e,
        }
    }

    /// Droit d'ÉCRIRE la clé donnée (sans la fenêtre). Rend vrai si l'acteur porte `notes.corriger`.
    #[allow(clippy::too_many_arguments)]
    async fn eva_v6_droit_ecrire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        type_appreciation: &str,
        cours: Option<Uuid>,
        eleve: Uuid,
        s: &Situation,
    ) -> Result<bool, EntDbError> {
        Self::exiger_staff(role)?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        let mut ok = corriger;
        if !ok {
            ok = match type_appreciation {
                "cours" | "intermediaire" => {
                    let cours = cours.ok_or_else(|| refus("cours requis"))?;
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    let a = match Self::eva_v3_acces_tx(
                        &mut tx, etab, acteur, cours, s.jour_ref, false, false,
                    )
                    .await
                    {
                        Ok(a) => a,
                        // Un cours inconnu ne se distingue pas d'un cours sans droit (aucun oracle).
                        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)) => {
                            return Err(EntDbError::Forbidden)
                        }
                        Err(e) => return Err(e),
                    };
                    let ok = a.saisie && eleve_du_cours_tx(&mut tx, etab, cours, eleve, s).await?;
                    tx.commit().await.map_err(db)?;
                    ok
                }
                "generale" => {
                    let pp = match s.classe_id {
                        Some(c) => {
                            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                            let pp = est_pp_tx(&mut tx, etab, acteur, c, s.jour_ref).await?;
                            tx.commit().await.map_err(db)?;
                            pp
                        }
                        None => false,
                    };
                    pp || self
                        .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
                        .await?
                }
                "vie_scolaire" => {
                    Role::parse(role) == Some(Role::Cpe)
                        || self
                            .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
                            .await?
                }
                _ => {
                    self.a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
                        .await?
                }
            };
        }
        if !ok {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        Ok(corriger)
    }

    /// Droit de RELIRE les appréciations d'un élève (EVA-204) : professeur principal de sa classe, `bulletins.publier`
    /// ou `notes.corriger`.
    async fn eva_v6_droit_relire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        s: &Situation,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        let mut ok = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?
            || self
                .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
                .await?;
        if !ok {
            if let Some(c) = s.classe_id {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                ok = est_pp_tx(&mut tx, etab, acteur, c, s.jour_ref).await?;
                tx.commit().await.map_err(db)?;
            }
        }
        if !ok {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve).await
    }

    async fn eva_v6_situation(
        &self,
        etab: Uuid,
        eleve: Uuid,
        periode: Uuid,
        jour: NaiveDate,
    ) -> Result<Situation, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = situation_tx(&mut tx, etab, eleve, periode, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(s)
    }

    async fn eva_v6_mots_interdits(&self, etab: Uuid) -> Result<Vec<String>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let v: Vec<String> = sqlx::query_scalar(
            "SELECT mot FROM eva_appreciation_mot_interdit WHERE etablissement_id = $1 ORDER BY lower(mot)",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Contexte des contrôles d'une appréciation saisie : prénoms voisins (public du cours, sinon classe), moyenne du
    /// cours, textes identiques sur la même clé de groupe (validés, ou brouillons de l'acteur).
    #[allow(clippy::too_many_arguments)]
    async fn eva_v6_contexte(
        &self,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        periode: Uuid,
        type_appreciation: &str,
        cours: Option<Uuid>,
        texte: &str,
        s: &Situation,
    ) -> Result<ContexteControle, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let prenom_eleve: String = sqlx::query_scalar(
            "SELECT prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .unwrap_or_default();
        let autres_prenoms: Vec<String> = match (cours, s.classe_id) {
            (Some(c), _) => sqlx::query_scalar(
                "SELECT e.prenom FROM ent_public_cours_jour($1, $2, $3) p
                   JOIN ent_eleve e ON e.id = p.eleve_id AND e.etablissement_id = $1
                  WHERE p.eleve_id <> $4",
            )
            .bind(etab)
            .bind(s.jour_ref)
            .bind(c)
            .bind(eleve)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?,
            (None, Some(cl)) => sqlx::query_scalar(
                "SELECT DISTINCT e.prenom FROM ent_affectation a
                   JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
                  WHERE a.etablissement_id = $1 AND a.classe_id = $2 AND a.eleve_id <> $3
                    AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $4)",
            )
            .bind(etab)
            .bind(cl)
            .bind(eleve)
            .bind(s.debut)
            .bind(s.fin)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?,
            (None, None) => Vec::new(),
        };
        let voisins = sqlx::query(
            "SELECT a.texte, a.etat, COALESCE(a.modifie_par, a.auteur) AS editeur FROM eva_appreciation a
              WHERE a.etablissement_id = $1 AND a.periode_id = $2 AND a.type = $3
                AND a.cours_id IS NOT DISTINCT FROM $4 AND a.eleve_id <> $5 AND a.etat <> 'jete'",
        )
        .bind(etab)
        .bind(periode)
        .bind(type_appreciation)
        .bind(cours)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let cle = cle_identique(texte);
        let mut identiques_ailleurs = 0;
        for v in &voisins {
            let etat: String = v.try_get("etat").map_err(db)?;
            let editeur: Uuid = v.try_get("editeur").map_err(db)?;
            if etat == "brouillon" && editeur != acteur {
                continue;
            }
            let t: String = v.try_get("texte").map_err(db)?;
            if cle_identique(&t) == cle {
                identiques_ailleurs += 1;
            }
        }
        let matiere: Option<Uuid> = match cours {
            Some(c) => sqlx::query_scalar(
                "SELECT matiere_id FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(c)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .flatten(),
            None => None,
        };
        // Droit déjà vérifié par l'appelant (écriture sur cette clé) : la moyenne est celle que l'auteur voit à la grille.
        let moyenne_m = match matiere {
            Some(m) => moyennes_tx(&mut tx, etab, eleve, periode, false)
                .await?
                .matieres
                .iter()
                .find(|x| x.matiere_id == m)
                .and_then(|x| x.moyenne.affichee_m),
            None => None,
        };
        tx.commit().await.map_err(db)?;
        Ok(ContexteControle {
            prenom_eleve,
            autres_prenoms,
            moyenne_m,
            identiques_ailleurs,
            mots_interdits: self.eva_v6_mots_interdits(etab).await?,
        })
    }

    // ───────────────────────── Saisie (EVA-194, EVA-201, EVA-203, EVA-205) ─────────────────────────

    /// Écrit une appréciation (première version, nouvelle version, validation d'un brouillon, correction de la direction
    /// après la fenêtre). Rend l'appréciation et les avertissements de cohérence sur son texte.
    pub async fn eva_v6_saisir_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &SaisirAppreciationBody,
    ) -> Result<SaisieAppreciationDto, EntDbError> {
        Self::exiger_staff(role)?;
        let t = body.type_appreciation.as_str();
        if !TYPES_APPRECIATION.contains(&t) {
            return Err(refus("type d'appréciation inconnu"));
        }
        if type_avec_cours(t) != body.cours_id.is_some() {
            return Err(refus(
                "cours obligatoire pour une appréciation de cours ou intermédiaire, interdit sinon",
            ));
        }
        let texte = texte_appreciation(&body.texte, t)?;
        let s = self
            .eva_v6_situation(etab, body.eleve_id, body.periode_id, jour)
            .await
            .map_err(|e| match e {
                EntDbError::Domain(domain_eleve::DomainError::NotFound) => EntDbError::Forbidden,
                e => e,
            });
        let s = match s {
            Ok(s) => s,
            Err(EntDbError::Forbidden) => {
                return Err(self.eva_v6_inconnu(etab, acteur, role).await)
            }
            Err(e) => return Err(e),
        };
        let corriger = self
            .eva_v6_droit_ecrire(etab, acteur, role, t, body.cours_id, body.eleve_id, &s)
            .await?;
        let Some(etat_periode) = s.etat.clone() else {
            return Err(refus(
                "élève sans classe pour cette période : aucune appréciation possible",
            ));
        };
        let r = rang(&etat_periode);
        let correction = if r >= fin_fenetre(t) {
            if !corriger {
                return Err(conflit(
                    "fenêtre des appréciations fermée pour la classe de l'élève : la direction corrige, avec un motif",
                ));
            }
            if r >= 4 {
                return Err(conflit(
                    "période close : la correction relève des corrections après publication",
                ));
            }
            if !body.valider {
                return Err(refus(
                    "une correction après la fenêtre est validée, jamais un brouillon",
                ));
            }
            Some(motif_correction(body.motif_correction.as_deref())?)
        } else {
            None
        };

        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existante = sqlx::query(&format!(
            "SELECT {COLONNES} FROM eva_appreciation a
              WHERE a.etablissement_id = $1 AND a.periode_id = $2 AND a.eleve_id = $3 AND a.type = $4
                AND a.cours_id IS NOT DISTINCT FROM $5 AND a.etat <> 'jete'
              FOR UPDATE"
        ))
        .bind(etab)
        .bind(body.periode_id)
        .bind(body.eleve_id)
        .bind(t)
        .bind(body.cours_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let etat = if body.valider { "validee" } else { "brouillon" };
        let (id, ecrite) = match existante {
            None => {
                if body.version_attendue.is_some() {
                    return Err(conflit(
                        "appréciation modifiée ou jetée entre-temps : relisez-la",
                    ));
                }
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO eva_appreciation (id, etablissement_id, periode_id, eleve_id, type, cours_id, texte,
                                                   etat, auteur, modifie_par)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
                )
                .bind(id)
                .bind(etab)
                .bind(body.periode_id)
                .bind(body.eleve_id)
                .bind(t)
                .bind(body.cours_id)
                .bind(&texte)
                .bind(etat)
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(|e| match &e {
                    sqlx::Error::Database(d) if d.code().as_deref() == Some("23505") => {
                        conflit("appréciation écrite entre-temps par un autre : relisez-la")
                    }
                    _ => db(e),
                })?;
                (id, true)
            }
            Some(row) => {
                let l = lue(&row, acteur)?;
                if l.dto.etat == "brouillon" && l.editeur != acteur {
                    return Err(conflit(
                        "un autre auteur tient un brouillon sur cette appréciation : il doit le valider ou le jeter",
                    ));
                }
                if body.version_attendue != Some(l.dto.version) {
                    return Err(conflit("appréciation modifiée entre-temps : relisez-la"));
                }
                if l.dto.etat == "validee" && !body.valider {
                    return Err(refus(
                        "une appréciation validée ne repasse pas en brouillon",
                    ));
                }
                let avant = l.dto.version;
                let apres: i32 = sqlx::query_scalar(
                    "UPDATE eva_appreciation SET texte = $3, etat = $4, modifie_par = $5
                      WHERE etablissement_id = $1 AND id = $2 RETURNING version",
                )
                .bind(etab)
                .bind(l.dto.id)
                .bind(&texte)
                .bind(etat)
                .bind(acteur)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                (l.dto.id, apres != avant)
            }
        };
        if let (Some(motif), true) = (&correction, ecrite) {
            sqlx::query(
                "INSERT INTO eva_appreciation_correction (id, etablissement_id, appreciation_id, version, etat_periode,
                                                          motif, par)
                 SELECT $1, $2, a.id, a.version, $4, $5, $6 FROM eva_appreciation a
                  WHERE a.etablissement_id = $2 AND a.id = $3",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(&etat_periode)
            .bind(motif)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let row = sqlx::query(&format!(
            "SELECT {COLONNES} FROM eva_appreciation a WHERE a.etablissement_id = $1 AND a.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let appreciation = lue(&row, acteur)?.dto;
        let ctx = self
            .eva_v6_contexte(
                etab,
                acteur,
                body.eleve_id,
                body.periode_id,
                t,
                body.cours_id,
                &texte,
                &s,
            )
            .await?;
        Ok(SaisieAppreciationDto {
            controles: controler(&texte, &ctx),
            appreciation,
        })
    }

    /// Jette un brouillon (son auteur seulement). Rien ne se supprime : l'état devient `jete`.
    pub async fn eva_v6_jeter_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        body: &JeterBrouillonBody,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let row = sqlx::query(&format!(
            "SELECT {COLONNES} FROM eva_appreciation a WHERE a.etablissement_id = $1 AND a.id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(row) = row else {
            return Err(EntDbError::Forbidden);
        };
        let l = lue(&row, acteur)?;
        // Le brouillon d'un autre, une validée ou un texte jeté : rien à faire ici, et rien à apprendre.
        if l.dto.etat != "brouillon" || l.editeur != acteur {
            return Err(EntDbError::Forbidden);
        }
        if l.dto.version != body.version_attendue {
            return Err(conflit("brouillon modifié entre-temps : relisez-le"));
        }
        sqlx::query(
            "UPDATE eva_appreciation SET etat = 'jete', modifie_par = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Saisie en série et contrôles (EVA-195, EVA-200) ─────────────────────────

    /// Les élèves d'un cours pour une période, leur moyenne du cours à côté, leur appréciation de cours (validée, ou
    /// brouillon de l'acteur) et ses avertissements. Mêmes élèves et mêmes droits de lecture que la grille V3 (élèves
    /// protégés retirés).
    pub async fn eva_v6_serie_cours_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours: Uuid,
        periode: Uuid,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<SerieCoursDto, EntDbError> {
        let grille = self
            .eva_v3_grille_for_actor(etab, acteur, role, cours, periode, jour, maintenant)
            .await?;
        let corriger = grille.peut_corriger;
        let ids: Vec<Uuid> = grille.eleves.iter().map(|e| e.eleve_id).collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (debut, fin): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let jour_ref = jour.clamp(debut, fin);
        let etats = sqlx::query(
            "SELECT x.id AS eleve_id, pe.periode_id, pe.etat
               FROM unnest($2::uuid[]) AS x(id)
               LEFT JOIN LATERAL eva_periode_eleve_jour($1, x.id, $3) pe ON true",
        )
        .bind(etab)
        .bind(&ids)
        .bind(jour_ref)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut etat_de: HashMap<Uuid, String> = HashMap::new();
        for r in &etats {
            let p: Option<Uuid> = r.try_get("periode_id").map_err(db)?;
            if p == Some(periode) {
                etat_de.insert(
                    r.try_get("eleve_id").map_err(db)?,
                    r.try_get("etat").map_err(db)?,
                );
            }
        }
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES} FROM eva_appreciation a
              WHERE a.etablissement_id = $1 AND a.periode_id = $2 AND a.type = 'cours' AND a.cours_id = $3
                AND a.etat <> 'jete'"
        ))
        .bind(etab)
        .bind(periode)
        .bind(cours)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut app: HashMap<Uuid, Lue> = HashMap::new();
        for r in &rows {
            let l = lue(r, acteur)?;
            app.insert(l.dto.eleve_id, l);
        }
        // Textes visibles (pour le contrôle « identique ») et prénoms du cours.
        let mut compte: HashMap<String, usize> = HashMap::new();
        for l in app.values().filter(|l| visible(l, acteur)) {
            *compte.entry(cle_identique(&l.dto.texte)).or_default() += 1;
        }
        let prenoms: Vec<String> = grille.eleves.iter().map(|e| e.prenom.clone()).collect();
        let mots = self.eva_v6_mots_interdits(etab).await?;
        let mut eleves = Vec::with_capacity(grille.eleves.len());
        for e in &grille.eleves {
            let etat = etat_de.get(&e.eleve_id).cloned();
            let ouverte = etat
                .as_deref()
                .is_some_and(|x| rang(x) < fin_fenetre("cours"));
            let a = app.get(&e.eleve_id);
            let brouillon_d_un_autre =
                a.is_some_and(|l| l.dto.etat == "brouillon" && l.editeur != acteur);
            let appreciation = a.filter(|l| visible(l, acteur)).map(|l| l.dto.clone());
            let controles: Vec<ControleDto> = match &appreciation {
                Some(x) => {
                    let n = compte.get(&cle_identique(&x.texte)).copied().unwrap_or(1);
                    controler(
                        &x.texte,
                        &ContexteControle {
                            prenom_eleve: e.prenom.clone(),
                            autres_prenoms: prenoms.clone(),
                            moyenne_m: e.moyenne_m,
                            identiques_ailleurs: n.saturating_sub(1),
                            mots_interdits: mots.clone(),
                        },
                    )
                }
                None => Vec::new(),
            };
            eleves.push(LigneSerieDto {
                eleve_id: e.eleve_id,
                nom: e.nom.clone(),
                prenom: e.prenom.clone(),
                moyenne_m: e.moyenne_m,
                nb_comptees: e.nb_comptees,
                etat_periode: etat,
                saisie_ouverte: grille.peut_saisir && ouverte && !brouillon_d_un_autre,
                appreciation,
                brouillon_d_un_autre,
                controles,
            });
        }
        Ok(SerieCoursDto {
            cours_id: cours,
            periode_id: periode,
            libelle: grille.enseignement.libelle.clone(),
            peut_saisir: grille.peut_saisir || corriger,
            limite: limite_caracteres("cours"),
            eleves,
        })
    }

    // ───────────────────────── Relecture (EVA-204) et historique ─────────────────────────

    /// Les appréciations validées d'un élève pour une période (tous types), plus les brouillons de l'acteur.
    pub async fn eva_v6_appreciations_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        periode: Uuid,
        jour: NaiveDate,
    ) -> Result<AppreciationsEleveDto, EntDbError> {
        Self::exiger_staff(role)?;
        let s = match self.eva_v6_situation(etab, eleve, periode, jour).await {
            Ok(s) => s,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)) => {
                return Err(self.eva_v6_inconnu(etab, acteur, role).await)
            }
            Err(e) => return Err(e),
        };
        self.eva_v6_droit_relire(etab, acteur, role, eleve, &s)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES} FROM eva_appreciation a
               LEFT JOIN ent_cours c ON c.id = a.cours_id AND c.etablissement_id = a.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = c.matiere_id AND m.etablissement_id = c.etablissement_id
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.periode_id = $3 AND a.etat <> 'jete'
              ORDER BY CASE a.type WHEN 'cours' THEN 0 WHEN 'dispositif' THEN 1 WHEN 'vie_scolaire' THEN 2
                                   WHEN 'generale' THEN 3 ELSE 4 END,
                       m.libelle NULLS LAST, a.cree_le, a.id"
        ))
        .bind(etab)
        .bind(eleve)
        .bind(periode)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut appreciations = Vec::new();
        for r in &rows {
            let l = lue(r, acteur)?;
            if visible(&l, acteur) {
                appreciations.push(l.dto);
            }
        }
        Ok(AppreciationsEleveDto {
            eleve_id: eleve,
            periode_id: periode,
            etat_periode: s.etat,
            appreciations,
        })
    }

    /// Charge une appréciation visible de l'acteur et vérifie qu'il peut la relire ou l'écrire. Une ligne inconnue, un
    /// brouillon d'un autre ou un texte jeté : introuvable pour `notes.corriger`, droit refusé sinon.
    async fn eva_v6_charger_visible(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        jour: NaiveDate,
    ) -> Result<(Lue, Situation), EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let row = sqlx::query(&format!(
            "SELECT {COLONNES} FROM eva_appreciation a WHERE a.etablissement_id = $1 AND a.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let l = match row {
            Some(r) => lue(&r, acteur)?,
            None => return Err(self.eva_v6_inconnu(etab, acteur, role).await),
        };
        if !visible(&l, acteur) {
            return Err(self.eva_v6_inconnu(etab, acteur, role).await);
        }
        let s = self
            .eva_v6_situation(etab, l.dto.eleve_id, l.dto.periode_id, jour)
            .await?;
        if l.editeur != acteur
            && self
                .eva_v6_droit_relire(etab, acteur, role, l.dto.eleve_id, &s)
                .await
                .is_err()
        {
            self.eva_v6_droit_ecrire(
                etab,
                acteur,
                role,
                &l.dto.type_appreciation,
                l.dto.cours_id,
                l.dto.eleve_id,
                &s,
            )
            .await?;
        }
        Ok((l, s))
    }

    /// Versions d'une appréciation. Les versions « brouillon » d'un autre éditeur n'apparaissent jamais.
    pub async fn eva_v6_historique_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<VersionAppreciationDto>, EntDbError> {
        self.eva_v6_charger_visible(etab, acteur, role, id, jour)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT h.version, h.texte, h.etat, h.aide_ia, h.auteur, h.le, c.motif
               FROM eva_appreciation_historique h
               LEFT JOIN eva_appreciation_correction c
                 ON c.etablissement_id = h.etablissement_id AND c.appreciation_id = h.appreciation_id
                AND c.version = h.version
              WHERE h.etablissement_id = $1 AND h.appreciation_id = $2
              ORDER BY h.version",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut out = Vec::new();
        for r in &rows {
            let etat: String = r.try_get("etat").map_err(db)?;
            let auteur: Uuid = r.try_get("auteur").map_err(db)?;
            if etat != "validee" && auteur != acteur {
                continue;
            }
            out.push(VersionAppreciationDto {
                version: r.try_get("version").map_err(db)?,
                texte: r.try_get("texte").map_err(db)?,
                etat,
                aide_ia: r.try_get("aide_ia").map_err(db)?,
                de_moi: auteur == acteur,
                le: r.try_get("le").map_err(db)?,
                motif_correction: r.try_get("motif").map_err(db)?,
            });
        }
        Ok(out)
    }

    // ───────────────────────── Remarques de relecture (EVA-204) ─────────────────────────

    /// Adresse une remarque à l'éditeur de la version relue (professeur principal, `bulletins.publier`,
    /// `notes.corriger`). La version doit être la version en vigueur, validée.
    pub async fn eva_v6_remarquer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        jour: NaiveDate,
        body: &RemarqueBody,
    ) -> Result<RemarqueDto, EntDbError> {
        let (l, s) = self
            .eva_v6_charger_visible(etab, acteur, role, id, jour)
            .await?;
        self.eva_v6_droit_relire(etab, acteur, role, l.dto.eleve_id, &s)
            .await?;
        let texte = body.texte.trim();
        if !(3..=500).contains(&texte.chars().count()) {
            return Err(refus("remarque de 3 à 500 caractères"));
        }
        let texte = texte_borne(texte, 500)?;
        if l.dto.etat != "validee" {
            return Err(refus("seule une appréciation validée se relit"));
        }
        if l.dto.version != body.version {
            return Err(conflit(
                "appréciation modifiée depuis la relecture : relisez-la",
            ));
        }
        if l.editeur == acteur {
            return Err(refus("vous êtes l'auteur de cette version"));
        }
        let rid = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cree_le: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO eva_appreciation_remarque (id, etablissement_id, appreciation_id, version, destinataire, texte, auteur)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING cree_le",
        )
        .bind(rid)
        .bind(etab)
        .bind(id)
        .bind(l.dto.version)
        .bind(l.editeur)
        .bind(&texte)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(RemarqueDto {
            id: rid,
            appreciation_id: id,
            eleve_id: l.dto.eleve_id,
            periode_id: l.dto.periode_id,
            type_appreciation: l.dto.type_appreciation,
            cours_id: l.dto.cours_id,
            version: l.dto.version,
            texte,
            cree_le,
            traitee: false,
            de_moi: true,
        })
    }

    /// Les remarques reçues (non traitées d'abord) et envoyées par l'acteur, 200 au plus. Les élèves qu'il ne doit plus
    /// voir (fiche protégée) en sont retirés.
    pub async fn eva_v6_mes_remarques_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<RemarqueDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT r.id, r.appreciation_id, a.eleve_id, a.periode_id, a.type, a.cours_id, r.version, r.texte,
                    r.cree_le, (r.traitee_le IS NOT NULL) AS traitee, (r.auteur = $2) AS de_moi
               FROM eva_appreciation_remarque r
               JOIN eva_appreciation a ON a.id = r.appreciation_id AND a.etablissement_id = r.etablissement_id
              WHERE r.etablissement_id = $1 AND (r.destinataire = $2 OR r.auteur = $2)
              ORDER BY (r.traitee_le IS NOT NULL), r.cree_le DESC, r.id
              LIMIT 200",
        )
        .bind(etab)
        .bind(acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in &rows {
            out.push(RemarqueDto {
                id: r.try_get("id").map_err(db)?,
                appreciation_id: r.try_get("appreciation_id").map_err(db)?,
                eleve_id: r.try_get("eleve_id").map_err(db)?,
                periode_id: r.try_get("periode_id").map_err(db)?,
                type_appreciation: r.try_get("type").map_err(db)?,
                cours_id: r.try_get("cours_id").map_err(db)?,
                version: r.try_get("version").map_err(db)?,
                texte: r.try_get("texte").map_err(db)?,
                cree_le: r.try_get("cree_le").map_err(db)?,
                traitee: r.try_get("traitee").map_err(db)?,
                de_moi: r.try_get("de_moi").map_err(db)?,
            });
        }
        let ids: Vec<Uuid> = out
            .iter()
            .map(|r| r.eleve_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let profil = self.profil_champs(etab, acteur, role).await?;
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;
        out.retain(|r| !caches.contains(&r.eleve_id));
        Ok(out)
    }

    /// Marque « traitée » une remarque reçue (son destinataire seulement, une fois).
    pub async fn eva_v6_traiter_remarque_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(Uuid, bool)> = sqlx::query_as(
            "SELECT destinataire, traitee_le IS NOT NULL FROM eva_appreciation_remarque
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match r {
            Some((dest, _)) if dest != acteur => return Err(EntDbError::Forbidden),
            None => return Err(EntDbError::Forbidden),
            Some((_, true)) => return Err(conflit("remarque déjà traitée")),
            Some((_, false)) => {}
        }
        sqlx::query(
            "UPDATE eva_appreciation_remarque SET traitee_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Banque de phrases (EVA-196) ─────────────────────────

    /// La banque personnelle de l'acteur dans cet établissement (R-V35 : jamais celle d'un autre établissement).
    pub async fn eva_v6_banque_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<PhraseBanqueDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT id, texte, etiquettes FROM eva_banque_phrase
              WHERE etablissement_id = $1 AND user_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, texte, etiquettes)| PhraseBanqueDto {
                id,
                texte,
                etiquettes,
            })
            .collect())
    }

    /// Ajoute une phrase à la banque de l'acteur (la même phrase deux fois : la première est rendue).
    pub async fn eva_v6_ajouter_phrase_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &AjouterPhraseBody,
    ) -> Result<PhraseBanqueDto, EntDbError> {
        Self::exiger_staff(role)?;
        let texte = body.texte.trim();
        if texte.is_empty() {
            return Err(refus("phrase vide"));
        }
        let texte = texte_borne(texte, 500)?;
        let etiquettes = match body.etiquettes.as_deref().map(str::trim) {
            Some("") | None => None,
            Some(e) => Some(texte_borne(e, 200)?),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Verrou de la banque de la personne : le plafond tient sous la concurrence.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('eva_banque:' || $1::text || ':' || $2::text, 0))")
            .bind(etab)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existante: Option<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT id, etiquettes FROM eva_banque_phrase WHERE etablissement_id = $1 AND user_id = $2 AND texte = $3",
        )
        .bind(etab)
        .bind(acteur)
        .bind(&texte)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((id, etiquettes)) = existante {
            tx.commit().await.map_err(db)?;
            return Ok(PhraseBanqueDto {
                id,
                texte,
                etiquettes,
            });
        }
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM eva_banque_phrase WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n >= MAX_PHRASES_BANQUE {
            return Err(refus("banque pleine : 500 phrases au plus, retirez-en une"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO eva_banque_phrase (id, etablissement_id, user_id, texte, etiquettes) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(etab)
        .bind(acteur)
        .bind(&texte)
        .bind(&etiquettes)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(PhraseBanqueDto {
            id,
            texte,
            etiquettes,
        })
    }

    /// Retire une phrase de SA banque (une phrase d'un autre : droit refusé, sans oracle).
    pub async fn eva_v6_retirer_phrase_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "DELETE FROM eva_banque_phrase WHERE etablissement_id = $1 AND id = $2 AND user_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        tx.commit().await.map_err(db)?;
        if n == 0 {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    // ───────────────────────── Mots interdits (S-117) ─────────────────────────

    /// La liste courte des mots à éviter : lisible par tout le personnel (les contrôles la citent).
    pub async fn eva_v6_mots_interdits_for_actor(
        &self,
        etab: Uuid,
        role: &str,
    ) -> Result<Vec<MotInterditDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, mot FROM eva_appreciation_mot_interdit WHERE etablissement_id = $1 ORDER BY lower(mot), id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, mot)| MotInterditDto { id, mot })
            .collect())
    }

    async fn eva_v6_exiger_parametrage(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if self
            .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
            .await?
            || self
                .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
                .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Ajoute un mot à la liste (direction : `bulletins.publier` ou `notes.corriger`). Un doublon (casse ignorée) rend
    /// le mot existant.
    pub async fn eva_v6_ajouter_mot_interdit_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &AjouterMotInterditBody,
    ) -> Result<MotInterditDto, EntDbError> {
        self.eva_v6_exiger_parametrage(etab, acteur, role).await?;
        let mot = body.mot.trim();
        if !(2..=60).contains(&mot.chars().count()) {
            return Err(refus("mot de 2 à 60 caractères"));
        }
        let mot = texte_borne(mot, 60)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended('eva_mots_interdits:' || $1::text, 0))",
        )
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let existant: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, mot FROM eva_appreciation_mot_interdit WHERE etablissement_id = $1 AND lower(mot) = lower($2)",
        )
        .bind(etab)
        .bind(&mot)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((id, mot)) = existant {
            tx.commit().await.map_err(db)?;
            return Ok(MotInterditDto { id, mot });
        }
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM eva_appreciation_mot_interdit WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n >= MAX_MOTS_INTERDITS {
            return Err(refus("liste courte : 100 mots au plus"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO eva_appreciation_mot_interdit (id, etablissement_id, mot, cree_par) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(etab)
        .bind(&mot)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(MotInterditDto { id, mot })
    }

    pub async fn eva_v6_retirer_mot_interdit_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.eva_v6_exiger_parametrage(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "DELETE FROM eva_appreciation_mot_interdit WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        tx.commit().await.map_err(db)?;
        if n == 0 {
            return Err(introuvable());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenetres_par_type() {
        assert_eq!(fin_fenetre("intermediaire"), 1);
        assert_eq!(fin_fenetre("cours"), 2);
        for t in ["generale", "vie_scolaire", "dispositif"] {
            assert_eq!(fin_fenetre(t), 3, "{t}");
        }
        assert_eq!(rang("ouverte"), 0);
        assert_eq!(rang("conseil_tenu"), 3);
        assert_eq!(rang("close"), 4);
    }

    #[test]
    fn texte_refuse_trop_long_sans_troncature() {
        let t = "a".repeat(401);
        assert!(texte_appreciation(&t, "cours").is_err());
        assert!(texte_appreciation(&t, "generale").is_ok());
        assert!(texte_appreciation("   ", "cours").is_err());
        assert_eq!(texte_appreciation("  Bien.  ", "cours").unwrap(), "Bien.");
    }
}
