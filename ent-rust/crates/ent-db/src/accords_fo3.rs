//! Socle 18, lot FO3 — accords sur objets : règle d'accord relevée, désaccord remonté au dossier de l'élève, objet créé par un
//! autre socle (sortie scolaire), dossier de preuve d'une décision par la brique de signature du socle 7 (lot D6)
//! (`docs/socles/18_FORMULAIRES.md` §7.1, §10, §21.1 : FRM-111, FRM-112, FRM-118, FRM-119, FRM-124 ; règles R-FC6, R-FC7, R-FC14,
//! R-FC44 ; défaut FD-17).
//!
//! Limites de D6 qu'on **n'a pas contournées** (`docs/socles/DOCUMENTS_D6.md`) :
//! * l'horodatage d'un circuit n'est que « reçu, non vérifié » : le dossier de preuve le dit tel quel, il n'est jamais présenté
//!   comme un horodatage qualifié ;
//! * la tablette sans confirmation (G0) n'est jamais une preuve ici, et l'agent qui constate n'est pas le décideur : seules les
//!   signatures **personnelles** (passkey, code SMS) du décideur sont retenues. Guichet et papier restent à brancher (FO3 suite).

use crate::accords::*;
use crate::accords_service::{
    empreinte_version, introuvable, journaliser, publier_evt, PreuveRetenue,
};
use crate::accords_verifier::{objet_par_code, version_en_cours};
use crate::notifications::db;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::BTreeSet;
use uuid::Uuid;

fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

/// Catalogue des objets qu'un autre socle peut faire créer pour un événement (sortie, voyage).
const CATALOGUES_POUR_SOURCE: [(&str, &str); 3] = [
    ("sortie_facultative", "sortie"),
    ("voyage_france", "voyage_fr"),
    ("voyage_etranger", "voyage_etr"),
];

#[derive(Debug, Clone, Deserialize)]
pub struct ObjetPourSource {
    /// `sortie_facultative`, `voyage_france` ou `voyage_etranger`.
    pub catalogue_code: String,
    /// Socle appelant (42 pour les sorties et voyages).
    pub source_socle: i16,
    /// Nature de l'objet source (`sortie`, `voyage`…) et son identifiant : une source n'a qu'un objet (idempotent).
    pub source_type: String,
    pub source_id: Uuid,
    pub libelle: String,
    /// Fin de l'événement : au-delà, les accords expirent et un retrait devient une demande à l'établissement (R-FC14).
    pub evenement_fin: Option<DateTime<Utc>>,
    /// Texte soumis aux familles ; absent : le texte de départ du catalogue (provisoire, à relire).
    pub texte: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesaccordVue {
    pub id: Uuid,
    pub sujet_eleve_id: Uuid,
    pub objet: String,
    pub objet_code: Option<String>,
    pub source: String,
    pub declare_le: DateTime<Utc>,
    pub leve_le: Option<DateTime<Utc>>,
    pub leve_motif: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreuveVue {
    pub signature_id: Uuid,
    pub circuit_id: Uuid,
    pub circuit_etat: String,
    pub niveau: String,
    pub methode: String,
    pub signee_le: DateTime<Utc>,
    pub empreinte_signee: String,
    pub tete_chaine_sha256: Option<String>,
    /// État de l'horodatage tel que D6 le tient : jamais « vérifié » (`recu_non_verifie` au mieux).
    pub horodatage: Option<String>,
    pub preuve_pdf_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DossierPreuve {
    pub decision_id: Uuid,
    pub objet_code: String,
    pub version_numero: i32,
    pub sujet_eleve_id: Option<Uuid>,
    pub decideur_user_id: Uuid,
    pub qualite: String,
    pub decision: String,
    pub canal: String,
    pub decide_le: DateTime<Utc>,
    pub empreinte_decision: String,
    pub preuve: Option<PreuveVue>,
    /// Ce que la preuve ne garantit pas, en clair (honnêteté sur les limites de D6).
    pub mentions: Vec<String>,
}

impl PgEnt {
    // -----------------------------------------------------------------------
    // Règle d'accord (R-FC6, FD-17)
    // -----------------------------------------------------------------------

    /// Relever la règle d'un objet (`un` → `tous`, etc.) : on monte, on ne baisse jamais (R-FC6) ; un objet d'image reste à
    /// `tous` (FD-17). Les décisions déjà prises restent : le verdict est recalculé à chaque appel.
    pub async fn accords_relever_regle(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        nouvelle_regle: &str,
    ) -> Result<ObjetAccord, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AutorisationsParametrer)
            .await?;
        if !REGLES.contains(&nouvelle_regle) {
            return Err(EntDbError::SaisieInvalide("règle d'accord inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code)
            .await?
            .ok_or_else(introuvable)?;
        if o.regle == nouvelle_regle {
            return Err(EntDbError::ConflitDossier(
                "cet objet suit déjà cette règle",
            ));
        }
        if o.sujet_type == "personne" || nouvelle_regle == "soi" || nouvelle_regle == "eleve_seul" {
            return Err(EntDbError::ConflitDossier(
                "cette règle ne se change pas : elle tient au sujet de l'accord",
            ));
        }
        if o.nature == "information" && nouvelle_regle == "tous" {
            return Err(EntDbError::SaisieInvalide(
                "une information ne se refuse pas : règle `tous` interdite",
            ));
        }
        if !regle_ne_baisse_pas(&o.regle, nouvelle_regle) {
            return Err(EntDbError::ConflitDossier(
                "la règle d'accord ne peut que monter, jamais baisser (R-FC6)",
            ));
        }
        if objet_exige_tous(&o.code, o.desaccord_objet.as_deref()) && nouvelle_regle != "tous" {
            return Err(EntDbError::ConflitDossier(
                "le droit à l'image exige la règle `tous` (FD-17)",
            ));
        }
        sqlx::query("UPDATE ent_acc_objet SET regle = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(o.id)
            .bind(nouvelle_regle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            "regle_relevee",
            Some(o.id),
            None,
            None,
            None,
            acteur,
            None,
            Some(nouvelle_regle),
        )
        .await?;
        publier_evt(
            &mut tx,
            etab,
            evenements::ETAT_MODIFIE,
            o.id,
            None,
            acteur,
            json!({ "objet_id": o.id, "regle": nouvelle_regle }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.accords_objet(etab, objet_code).await
    }

    // -----------------------------------------------------------------------
    // Désaccord déclaré, remonté au socle 4 (FRM-111, FRM-112)
    // -----------------------------------------------------------------------

    /// Ouvre un désaccord pour un élève (sans doublon) et le **publie** pour le socle 4. Rend `true` s'il vient d'être ouvert.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn ouvrir_desaccord(
        &self,
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        famille: &str,
        objet_acc_id: Option<Uuid>,
        source: &str,
        declarant: Uuid,
    ) -> Result<bool, EntDbError> {
        let id = Uuid::new_v4();
        let ouvert = sqlx::query(
            "INSERT INTO ent_acc_desaccord (id, etablissement_id, sujet_eleve_id, objet, objet_acc_id, source, declarant_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(famille)
        .bind(objet_acc_id)
        .bind(source)
        .bind(declarant)
        .execute(&mut **tx)
        .await
        .map_err(db)?
        .rows_affected()
            == 1;
        if ouvert {
            journaliser(
                tx,
                etab,
                "desaccord_ouvert",
                objet_acc_id,
                None,
                Some(eleve),
                None,
                declarant,
                None,
                Some(famille),
            )
            .await?;
            publier_evt(
                tx,
                etab,
                evenements::DESACCORD_DECLARE,
                id,
                Some(eleve),
                declarant,
                json!({ "eleve_id": eleve, "objet": famille, "objet_ref": objet_acc_id, "source": source }),
            )
            .await?;
        }
        Ok(ouvert)
    }

    /// Déclarer un désaccord (FRM-111). Un titulaire communicant le déclare pour son enfant ; la direction ou le secrétariat
    /// (`autorisations.gerer`) l'enregistre pour l'établissement (décision de justice, courrier). Dès lors, tous les objets de la
    /// famille sont « désaccord » pour cet élève, donc **non autorisés**, jusqu'à la levée par pièce.
    pub async fn accords_declarer_desaccord(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleve_id: Uuid,
        famille: &str,
    ) -> Result<DesaccordVue, EntDbError> {
        if !["image", "sortie", "general"].contains(&famille) {
            return Err(EntDbError::SaisieInvalide(
                "famille de désaccord inconnue (image, sortie ou general)",
            ));
        }
        let gerer = self
            .a_action_etablissement(etab, user, role, Action::AutorisationsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let source = if gerer {
            let connu: Option<(i32,)> =
                sqlx::query_as("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(eleve_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
            connu.ok_or_else(introuvable)?;
            "etablissement"
        } else {
            let titulaire: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_eleve_lien_communicant
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3 AND autorite_parentale LIMIT 1",
            )
            .bind(etab)
            .bind(eleve_id)
            .bind(user)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            titulaire.ok_or_else(introuvable)?;
            "declaration"
        };
        if !self
            .ouvrir_desaccord(&mut tx, etab, eleve_id, famille, None, source, user)
            .await?
        {
            return Err(EntDbError::ConflitDossier(
                "un désaccord est déjà ouvert pour cette famille",
            ));
        }
        let vue = lire_desaccord_ouvert(&mut tx, etab, eleve_id, famille).await?;
        tx.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Lever un désaccord (FRM-112) : seulement avec une **pièce** (accord écrit, décision de justice) déjà versée au dossier de
    /// l'élève, et un motif. Droit `autorisations.gerer`.
    pub async fn accords_lever_desaccord(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        desaccord_id: Uuid,
        piece_document_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AutorisationsGerer)
            .await?;
        let motif = motif.trim();
        if motif.is_empty() || motif.chars().count() > 300 {
            return Err(EntDbError::SaisieInvalide(
                "motif requis (300 caractères au plus)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d: Option<(Uuid, String, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
            "SELECT sujet_eleve_id, objet, leve_le FROM ent_acc_desaccord WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(desaccord_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve, famille, leve) = d.ok_or_else(introuvable)?;
        if leve.is_some() {
            return Err(EntDbError::ConflitDossier("ce désaccord est déjà levé"));
        }
        let piece: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3")
                .bind(etab)
                .bind(piece_document_id)
                .bind(eleve)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        if piece.is_none() {
            return Err(refus(
                RefusMetier::Invalide,
                "la pièce doit être un document du dossier de cet élève",
            ));
        }
        sqlx::query(
            "UPDATE ent_acc_desaccord SET leve_le = now(), leve_par = $3, leve_motif = $4, piece_document_id = $5
              WHERE etablissement_id = $1 AND id = $2 AND leve_le IS NULL",
        )
        .bind(etab)
        .bind(desaccord_id)
        .bind(user)
        .bind(motif)
        .bind(piece_document_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            "desaccord_leve",
            None,
            None,
            Some(eleve),
            None,
            user,
            None,
            Some(&famille),
        )
        .await?;
        publier_evt(
            &mut tx,
            etab,
            evenements::DESACCORD_LEVE,
            desaccord_id,
            Some(eleve),
            user,
            json!({ "eleve_id": eleve, "objet": famille }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Désaccords d'un élève (ouverts et levés), pour la direction et le secrétariat. Lecture journalisée (FD-09).
    pub async fn accords_desaccords(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<DesaccordVue>, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AutorisationsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, String, Option<String>, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<String>)> =
            sqlx::query_as(
                "SELECT d.id, d.sujet_eleve_id, d.objet, o.code, d.source, d.declare_le, d.leve_le, d.leve_motif
                   FROM ent_acc_desaccord d LEFT JOIN ent_acc_objet o ON o.etablissement_id = d.etablissement_id AND o.id = d.objet_acc_id
                  WHERE d.etablissement_id = $1 AND d.sujet_eleve_id = $2 ORDER BY d.declare_le DESC, d.id",
            )
            .bind(etab)
            .bind(eleve_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            "lecture_detaillee",
            None,
            None,
            Some(eleve_id),
            None,
            user,
            Some(lignes.len() as i32),
            Some("desaccords"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(
                |(
                    id,
                    sujet_eleve_id,
                    objet,
                    objet_code,
                    source,
                    declare_le,
                    leve_le,
                    leve_motif,
                )| DesaccordVue {
                    id,
                    sujet_eleve_id,
                    objet,
                    objet_code,
                    source,
                    declare_le,
                    leve_le,
                    leve_motif,
                },
            )
            .collect())
    }

    // -----------------------------------------------------------------------
    // Objet créé par un autre socle (sortie scolaire) — `creer_pour`
    // -----------------------------------------------------------------------

    /// Crée (ou retrouve) l'objet d'accord d'un événement d'un autre socle : sortie ou voyage du socle 42. **Idempotent** : une
    /// source n'a qu'un objet. L'objet suit le catalogue (règle, droit du lien, preuve, validité « événement ») et sa version 1 est
    /// **publiée** aussitôt avec le texte de départ, **provisoire** tant qu'elle n'est pas relue (R-FC72).
    ///
    /// Aucune route ne l'expose : le socle appelant a déjà contrôlé son propre droit (organiser la sortie) ; la brique vérifie que
    /// l'acteur est un membre du personnel de l'établissement et que la source est valide. Ne s'appelle jamais depuis une route
    /// sans contrôle de droit propre du socle appelant.
    pub async fn accords_creer_pour(
        &self,
        etab: Uuid,
        acteur: Uuid,
        p: ObjetPourSource,
    ) -> Result<ObjetAccord, EntDbError> {
        let Some((_, court)) = CATALOGUES_POUR_SOURCE
            .iter()
            .find(|(c, _)| *c == p.catalogue_code)
        else {
            return Err(EntDbError::SaisieInvalide(
                "catalogue non permis pour un événement (sortie ou voyage seulement)",
            ));
        };
        let libelle = p.libelle.trim().to_string();
        if libelle.is_empty() || libelle.chars().count() > 120 {
            return Err(EntDbError::SaisieInvalide(
                "libellé invalide (1 à 120 caractères)",
            ));
        }
        if !crate::formulaires::objet_valide(&p.source_type) || !(1..=60).contains(&p.source_socle)
        {
            return Err(EntDbError::SaisieInvalide("source invalide"));
        }
        let texte_perso = p
            .texte
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from);
        if texte_perso
            .as_ref()
            .is_some_and(|t| t.chars().count() > 10_000)
        {
            return Err(EntDbError::SaisieInvalide(
                "texte trop long (10 000 caractères au plus)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let membre: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(acteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if membre.is_none_or(|(r,)| matches!(r.as_str(), "parent" | "eleve")) {
            return Err(EntDbError::Forbidden);
        }
        // Une seule création à la fois par source.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 18))")
            .bind(format!("{etab}:{}:{}", p.source_type, p.source_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existant: Option<(Uuid, String, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT o.id, o.code, o.evenement_fin FROM ent_acc_objet_source s
               JOIN ent_acc_objet o ON o.etablissement_id = s.etablissement_id AND o.id = s.objet_id
              WHERE s.etablissement_id = $1 AND s.source_type = $2 AND s.source_id = $3",
        )
        .bind(etab)
        .bind(&p.source_type)
        .bind(p.source_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((id, code, fin)) = existant {
            // La date de fin suit l'événement (report, raccourcissement) : seule modification permise sur un objet de source.
            if p.evenement_fin.is_some() && p.evenement_fin != fin {
                sqlx::query("UPDATE ent_acc_objet SET evenement_fin = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .bind(p.evenement_fin)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                journaliser(
                    &mut tx,
                    etab,
                    "objet_pour_source",
                    Some(id),
                    None,
                    None,
                    None,
                    acteur,
                    None,
                    Some("fin_modifiee"),
                )
                .await?;
                publier_evt(
                    &mut tx,
                    etab,
                    evenements::ETAT_MODIFIE,
                    id,
                    None,
                    acteur,
                    json!({ "objet_id": id, "fin_modifiee": true }),
                )
                .await?;
            }
            tx.commit().await.map_err(db)?;
            return self.accords_objet(etab, &code).await;
        }
        let cat: Option<(String, String, Option<String>, String, String, Option<String>, String)> = sqlx::query_as(
            "SELECT nature, regle_min, droit_lien_code, preuve_min, validite, desaccord_objet, texte_depart
               FROM ent_acc_catalogue WHERE code = $1",
        )
        .bind(&p.catalogue_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (nature, regle, droit, preuve, validite, desaccord, texte_depart) =
            cat.ok_or_else(introuvable)?;
        let id = Uuid::new_v4();
        let code = format!("{court}_{}", &id.simple().to_string()[..12]);
        sqlx::query(
            "INSERT INTO ent_acc_objet
                (id, etablissement_id, code, libelle, nature, regle, droit_lien_code, preuve_min, validite, evenement_fin,
                 sujet_type, catalogue_code, desaccord_objet, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'eleve',$11,$12,$13)",
        )
        .bind(id)
        .bind(etab)
        .bind(&code)
        .bind(&libelle)
        .bind(&nature)
        .bind(&regle)
        .bind(&droit)
        .bind(&preuve)
        .bind(&validite)
        .bind(p.evenement_fin)
        .bind(&p.catalogue_code)
        .bind(&desaccord)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_usage (id, etablissement_id, objet_id, code, libelle, mission, ordre)
             SELECT gen_random_uuid(), $1, $2, u.code, u.libelle, u.mission, u.ordre
               FROM ent_acc_catalogue_usage u WHERE u.catalogue_code = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(&p.catalogue_code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let usages: BTreeSet<String> = sqlx::query_as::<_, (String,)>(
            "SELECT code FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(c,)| c)
        .collect();
        let texte = texte_perso.unwrap_or(texte_depart);
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_acc_version (id, etablissement_id, objet_id, numero, texte, empreinte, cree_par)
             VALUES ($1,$2,$3,1,$4,$5,$6)",
        )
        .bind(version_id)
        .bind(etab)
        .bind(id)
        .bind(&texte)
        .bind(empreinte_version(&code, 1, &texte, &usages))
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_version_usage (etablissement_id, version_id, usage_id)
             SELECT $1, $2, id FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $3",
        )
        .bind(etab)
        .bind(version_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Publiée aussitôt (les usages couverts sont figés dès la publication : on les écrit d'abord).
        sqlx::query("UPDATE ent_acc_version SET publiee_le = now(), publiee_par = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(version_id)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_objet_source (etablissement_id, objet_id, source_socle, source_type, source_id, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(etab)
        .bind(id)
        .bind(p.source_socle)
        .bind(&p.source_type)
        .bind(p.source_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            "objet_pour_source",
            Some(id),
            Some(version_id),
            None,
            None,
            acteur,
            None,
            Some(&p.catalogue_code),
        )
        .await?;
        publier_evt(
            &mut tx,
            etab,
            evenements::OBJET_ACTIVE,
            id,
            None,
            acteur,
            json!({ "objet_id": id, "source": p.source_type }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.accords_objet(etab, &code).await
    }

    // -----------------------------------------------------------------------
    // Dossier de preuve par la brique de signature (socle 7, lot D6) — FRM-119
    // -----------------------------------------------------------------------

    /// Rattache un document émis du socle 7 (le texte de l'accord, à signer) à la **version en cours** d'un objet et à un élève.
    /// Une signature ne prouve une décision que si elle porte sur un document ainsi rattaché. Droit `autorisations.gerer`.
    pub async fn accords_lier_document(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        objet_code: &str,
        eleve_id: Uuid,
        document_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AutorisationsGerer)
            .await?;
        self.lier_document_interne(etab, user, objet_code, eleve_id, document_id)
            .await
    }

    /// Entrée **des socles** (sorties, voyages) : ils ont déjà contrôlé leur propre droit. Ne s'appelle jamais depuis une route
    /// sans contrôle de droit propre.
    pub async fn accords_lier_document_service(
        &self,
        etab: Uuid,
        user: Uuid,
        objet_code: &str,
        eleve_id: Uuid,
        document_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.lier_document_interne(etab, user, objet_code, eleve_id, document_id)
            .await
    }

    async fn lier_document_interne(
        &self,
        etab: Uuid,
        user: Uuid,
        objet_code: &str,
        eleve_id: Uuid,
        document_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code)
            .await?
            .ok_or_else(introuvable)?;
        if o.sujet_type != "eleve" {
            return Err(EntDbError::SaisieInvalide(
                "cet accord n'a pas d'élève pour sujet",
            ));
        }
        let (version_id, ..) = version_en_cours(&mut tx, etab, o.id)
            .await?
            .ok_or(EntDbError::ConflitDossier("aucune version publiée"))?;
        let doc: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3")
                .bind(etab)
                .bind(document_id)
                .bind(eleve_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        if doc.is_none() {
            return Err(refus(
                RefusMetier::Invalide,
                "le document doit être un document émis pour cet élève",
            ));
        }
        sqlx::query(
            "INSERT INTO ent_acc_document_lien (id, etablissement_id, document_id, objet_id, version_id, sujet_eleve_id, lie_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(document_id)
        .bind(o.id)
        .bind(version_id)
        .bind(eleve_id)
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            "document_lie",
            Some(o.id),
            Some(version_id),
            Some(eleve_id),
            None,
            user,
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Décider **avec une signature D6** déjà faite par le décideur (passkey ou code à usage unique) sur le document rattaché à
    /// l'accord. C'est la voie qui satisfait un objet de preuve `S2` : sans brique ou sans signature valable, la décision est
    /// **refusée**, jamais simulée. Le décideur et sa qualité sont calculés comme pour toute décision (R-FC2).
    pub async fn accords_decider_signe(
        &self,
        etab: Uuid,
        user: Uuid,
        d: DecisionSaisie,
        signature_id: Uuid,
    ) -> Result<MaDecision, EntDbError> {
        let tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (tx, m) = self
            .decider_dans_tx(
                tx,
                etab,
                user,
                d,
                crate::accords_service::Preuve::D6(signature_id),
                false,
                None,
            )
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(m)
    }

    /// Vérifie qu'une signature D6 prouve bien la décision en cours. Tout doute est un refus.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn verifier_signature_d6(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        sujet: Uuid,
        objet_code: &str,
        objet_id: Uuid,
        version_id: Uuid,
        preuve_min: &str,
        signature_id: Uuid,
    ) -> Result<PreuveRetenue, EntDbError> {
        let ligne: Option<(Uuid, String, String, Option<Uuid>, Option<Uuid>, String, String, Uuid)> = sqlx::query_as(
            "SELECT s.circuit_id, s.niveau, s.methode, g.personne_id, g.eleve_id, g.etat, c.etat, c.document_id
               FROM ent_doc_signature s
               JOIN ent_doc_signataire g ON g.etablissement_id = s.etablissement_id AND g.id = s.signataire_id
               JOIN ent_doc_circuit c ON c.etablissement_id = s.etablissement_id AND c.id = s.circuit_id
              WHERE s.etablissement_id = $1 AND s.id = $2",
        )
        .bind(etab)
        .bind(signature_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some((
            circuit_id,
            niveau,
            methode,
            personne,
            eleve_signataire,
            etat_signataire,
            etat_circuit,
            document_id,
        )) = ligne
        else {
            return Err(refus(RefusMetier::Invalide, "signature introuvable"));
        };
        if etat_signataire != "signe" || !matches!(etat_circuit.as_str(), "ouvert" | "complet") {
            return Err(refus(
                RefusMetier::Conflit,
                "cette signature n'est plus valable (circuit clos, annulé ou remplacé)",
            ));
        }
        // G0 (tablette sans confirmation) et les constats d'un agent ne sont jamais la décision d'un titulaire (limite de D6).
        let niveau_ok = match preuve_min {
            "S2" => niveau == "S2",
            _ => matches!(niveau.as_str(), "S1" | "S2"),
        };
        if !niveau_ok || !matches!(methode.as_str(), "passkey" | "code_sms") {
            return Err(refus(
                RefusMetier::Invalide,
                "la signature n'atteint pas le niveau de preuve exigé (passkey ou code personnel du décideur)",
            ));
        }
        // Le signataire est le décideur lui-même.
        let sa_personne: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL")
                .bind(etab)
                .bind(user)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
        let son_compte_eleve: Option<(i32,)> = match eleve_signataire {
            Some(e) => sqlx::query_as("SELECT 1 FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
                .bind(etab)
                .bind(e)
                .bind(user)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?,
            None => None,
        };
        let est_lui = matches!((personne, sa_personne), (Some(p), Some((q,))) if p == q)
            || (eleve_signataire == Some(sujet) && son_compte_eleve.is_some());
        if !est_lui {
            return Err(refus(
                RefusMetier::Interdit,
                "cette signature n'est pas celle du décideur",
            ));
        }
        // Le document signé est bien celui de cet accord, de cette version, pour cet élève.
        let lie: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_acc_document_lien
              WHERE etablissement_id = $1 AND document_id = $2 AND objet_id = $3 AND version_id = $4 AND sujet_eleve_id = $5",
        )
        .bind(etab)
        .bind(document_id)
        .bind(objet_id)
        .bind(version_id)
        .bind(sujet)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if lie.is_none() {
            return Err(refus(
                RefusMetier::Invalide,
                "la signature ne porte pas sur le document de cet accord",
            ));
        }
        let deja: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_acc_preuve WHERE etablissement_id = $1 AND signature_id = $2")
                .bind(etab)
                .bind(signature_id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
        if deja.is_some() {
            return Err(refus(
                RefusMetier::Conflit,
                "cette signature prouve déjà une décision",
            ));
        }
        let canal = if methode == "passkey" {
            "passkey"
        } else {
            "code"
        };
        let _ = objet_code;
        Ok(PreuveRetenue {
            signature_id,
            circuit_id,
            niveau,
            methode,
            canal,
        })
    }

    /// Dossier de preuve d'une décision (FRM-119) : la décision, la signature D6 qui la prouve, l'état du circuit et de
    /// l'horodatage **tels que D6 les tient**, et ce que cela ne garantit pas. Droit `autorisations.gerer` ; lecture journalisée.
    pub async fn accords_dossier_preuve(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        decision_id: Uuid,
    ) -> Result<DossierPreuve, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AutorisationsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d: Option<(String, i32, Option<Uuid>, Uuid, String, String, String, DateTime<Utc>, String, Uuid)> = sqlx::query_as(
            "SELECT o.code, v.numero, x.sujet_eleve_id, x.decideur_user_id, x.qualite, x.decision, x.canal, x.decide_le, x.empreinte, o.id
               FROM ent_acc_decision x
               JOIN ent_acc_objet o ON o.etablissement_id = x.etablissement_id AND o.id = x.objet_id
               JOIN ent_acc_version v ON v.etablissement_id = x.etablissement_id AND v.id = x.version_id
              WHERE x.etablissement_id = $1 AND x.id = $2",
        )
        .bind(etab)
        .bind(decision_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (
            objet_code,
            version_numero,
            sujet,
            decideur,
            qualite,
            decision,
            canal,
            decide_le,
            empreinte,
            objet_id,
        ) = d.ok_or_else(introuvable)?;
        let p: Option<(Uuid, Uuid, String, String, String, DateTime<Utc>, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT p.signature_id, p.circuit_id, c.etat, s.niveau, s.methode, s.horodatage_serveur, s.empreinte_signee,
                    c.tete_chaine_sha256, c.preuve_sha256
               FROM ent_acc_preuve p
               JOIN ent_doc_signature s ON s.etablissement_id = p.etablissement_id AND s.id = p.signature_id
               JOIN ent_doc_circuit c ON c.etablissement_id = p.etablissement_id AND c.id = p.circuit_id
              WHERE p.etablissement_id = $1 AND p.decision_id = $2",
        )
        .bind(etab)
        .bind(decision_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let mut mentions = Vec::new();
        let preuve = match p {
            None => {
                mentions.push("Aucune signature de la brique du socle 7 : la décision est enregistrée en ligne, sans preuve renforcée.".into());
                None
            }
            Some((
                signature_id,
                circuit_id,
                circuit_etat,
                niveau,
                methode,
                signee_le,
                empreinte_signee,
                tete,
                preuve_sha,
            )) => {
                let horodatage: Option<(String,)> = sqlx::query_as(
                    "SELECT etat FROM ent_doc_horodatage WHERE etablissement_id = $1 AND circuit_id = $2",
                )
                .bind(etab)
                .bind(circuit_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let horodatage = horodatage.map(|(e,)| e);
                mentions.push(match horodatage.as_deref() {
                    Some("recu_non_verifie") => {
                        "Horodatage reçu d'un service tiers mais non vérifié : il ne vaut pas horodatage qualifié.".to_string()
                    }
                    Some(autre) => format!("Horodatage non obtenu (état : {autre})."),
                    None => "Aucun horodatage n'a été demandé : la date est celle du serveur ScolIA.".to_string(),
                });
                if niveau == "S1" {
                    mentions.push(
                        "Preuve simple : code à usage unique envoyé au téléphone du dossier."
                            .into(),
                    );
                }
                if matches!(circuit_etat.as_str(), "annule" | "obsolete") {
                    mentions.push("Le circuit de signature a été annulé ou remplacé : cette signature ne compte plus comme preuve.".into());
                }
                Some(PreuveVue {
                    signature_id,
                    circuit_id,
                    circuit_etat,
                    niveau,
                    methode,
                    signee_le,
                    empreinte_signee,
                    tete_chaine_sha256: tete,
                    horodatage,
                    preuve_pdf_sha256: preuve_sha,
                })
            }
        };
        journaliser(
            &mut tx,
            etab,
            "dossier_preuve_lu",
            Some(objet_id),
            None,
            sujet,
            None,
            user,
            Some(1),
            Some("dossier_preuve"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(DossierPreuve {
            decision_id,
            objet_code,
            version_numero,
            sujet_eleve_id: sujet,
            decideur_user_id: decideur,
            qualite,
            decision,
            canal,
            decide_le,
            empreinte_decision: empreinte,
            preuve,
            mentions,
        })
    }
}

async fn lire_desaccord_ouvert(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    eleve: Uuid,
    famille: &str,
) -> Result<DesaccordVue, EntDbError> {
    let l: (Uuid, String, DateTime<Utc>, String) = sqlx::query_as(
        "SELECT id, source, declare_le, objet FROM ent_acc_desaccord
          WHERE etablissement_id = $1 AND sujet_eleve_id = $2 AND objet = $3 AND objet_acc_id IS NULL AND leve_le IS NULL",
    )
    .bind(etab)
    .bind(eleve)
    .bind(famille)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    Ok(DesaccordVue {
        id: l.0,
        sujet_eleve_id: eleve,
        objet: l.3,
        objet_code: None,
        source: l.1,
        declare_le: l.2,
        leve_le: None,
        leve_motif: None,
    })
}
