//! Formulaires, autorisations et consentements (socle 2, lot S2-8, D24 :
//! FRM-01→05).
//!
//! **Modèle.** Un formulaire est un `consentement`, une `autorisation` (objets
//! stables, ex. `droit_image`, versionnés) ou un `sondage`. Il est ciblé
//! (établissement, niveaux, classes, groupes). Chaque réponse est une
//! **décision datée, immuable et signée** (`accepte`, `refuse`, `retire`) avec
//! la qualité du signataire et l'empreinte du texte signé ; l'historique n'est
//! jamais écrasé.
//!
//! **Qui signe.** Pour un élève mineur : un responsable avec lien actif et
//! autorité parentale. Pour un élève majeur : l'élève lui-même (son compte), plus
//! les parents. Un signataire dont le lien est suspendu ou terminé n'est plus pris
//! en compte.
//!
//! **Décision effective** (règle de prudence, [`etat_effectif`]) : pour un
//! élève, parmi les signataires qualifiés, la **dernière** décision de chacun ;
//! s'il y en a un seul qui refuse ou retire, l'état est `refuse` (un seul
//! responsable suffit pour retirer un droit à l'image) ; sinon `accepte` s'il y a
//! au moins une acceptation ; sinon `aucune`. Un refus d'une ancienne version
//! d'un objet reste valable tant qu'il n'est pas remplacé.
//!
//! **Application.** Le refus du `droit_image` masque la photo de l'élève
//! dans le trombinoscope et à la lecture de la photo ([`OBJET_DROIT_IMAGE`]).
//! L'absence de décision ne masque rien : la photo sert aux usages
//! administratifs (DOS-11) ; seul un refus explicite l'empêche.

use crate::campagnes::{
    eleves_de_audience, responsables_de, sha256_hex, types_com, valider_audience,
};
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;

/// Objet du consentement dont dépend l'affichage des photos.
pub const OBJET_DROIT_IMAGE: &str = "droit_image";

fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

// ---------------------------------------------------------------------------
// Règle de décision effective (pure)
// ---------------------------------------------------------------------------

/// État effectif d'un élève pour un objet, à partir de la **dernière** décision
/// de chaque signataire qualifié `(qualite, decision)`.
pub fn etat_effectif(dernieres: &[(&str, &str)], majeur: bool) -> &'static str {
    let attendue = if majeur { "eleve_majeur" } else { "responsable" };
    let pertinentes: Vec<&str> = dernieres
        .iter()
        .filter(|(q, _)| *q == attendue)
        .map(|(_, d)| *d)
        .collect();
    if pertinentes.iter().any(|d| matches!(*d, "refuse" | "retire")) {
        "refuse"
    } else if pertinentes.contains(&"accepte") {
        "accepte"
    } else {
        "aucune"
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Formulaire {
    pub id: Uuid,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub type_formulaire: String,
    pub objet: Option<String>,
    pub version: i32,
    pub titre: String,
    pub texte: String,
    pub statut: String,
    pub audience: String,
    pub cible_ids: Vec<Uuid>,
    pub cree_par: Uuid,
    pub publie_le: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

const COLONNES_FORMULAIRE: &str = "id, type, objet, version, titre, texte, statut, audience, \
    cible_ids, cree_par, publie_le, created_at";

#[derive(Debug, Clone, Deserialize)]
pub struct NouveauFormulaire {
    #[serde(rename = "type")]
    pub type_formulaire: String,
    pub objet: Option<String>,
    pub titre: String,
    pub texte: String,
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
}

/// Ce qu'une famille voit : un formulaire à traiter pour un de ses enfants.
#[derive(Debug, Clone, Serialize)]
pub struct FormulaireFamille {
    pub formulaire_id: Uuid,
    #[serde(rename = "type")]
    pub type_formulaire: String,
    pub objet: Option<String>,
    pub version: i32,
    pub titre: String,
    pub texte: String,
    pub eleve_id: Uuid,
    pub eleve_prenom: String,
    /// Ma dernière décision (`accepte`, `refuse`, `retire`), si j'en ai donné une.
    pub ma_decision: Option<String>,
    /// État effectif pour l'élève (tous signataires), pour un consentement.
    pub etat: Option<String>,
    pub a_repondre: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EtatConsentement {
    pub eleve_id: Uuid,
    pub objet: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsentementEleve {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatsFormulaire {
    pub cibles: usize,
    pub accepte: usize,
    pub refuse: usize,
    pub sans_reponse: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReponseFormulaire {
    pub eleve_id: Uuid,
    /// `accepte`, `refuse` ou `retire`.
    pub decision: String,
    pub commentaire: Option<String>,
}

// ---------------------------------------------------------------------------
// Lecture des décisions effectives
// ---------------------------------------------------------------------------

/// État effectif de chaque élève demandé (clé absente = `aucune`).
async fn etats_effectifs(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    objet: Option<&str>,
    formulaire_id: Option<Uuid>,
) -> Result<HashMap<Uuid, &'static str>, EntDbError> {
    if eleves.is_empty() {
        return Ok(HashMap::new());
    }
    // Dernière décision de chaque signataire, encore qualifié au moment de la lecture.
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "WITH derniere AS (
            SELECT DISTINCT ON (r.eleve_id, r.signataire_user_id)
                   r.eleve_id, r.signataire_user_id, r.qualite, r.decision
              FROM ent_formulaire_reponse r
             WHERE r.etablissement_id = $1 AND r.eleve_id = ANY($2)
               AND ($3::text IS NULL OR r.objet = $3)
               AND ($4::uuid IS NULL OR r.formulaire_id = $4)
             ORDER BY r.eleve_id, r.signataire_user_id, r.decide_le DESC, r.id DESC)
         SELECT d.eleve_id, d.qualite, d.decision
           FROM derniere d
          WHERE (d.qualite = 'responsable' AND EXISTS (
                    SELECT 1 FROM ent_eleve_lien_actif k
                     WHERE k.etablissement_id = $1 AND k.eleve_id = d.eleve_id
                       AND k.user_id = d.signataire_user_id AND k.autorite_parentale))
             OR (d.qualite = 'eleve_majeur' AND EXISTS (
                    SELECT 1 FROM ent_eleve_compte c
                     WHERE c.etablissement_id = $1 AND c.eleve_id = d.eleve_id
                       AND c.user_id = d.signataire_user_id))",
    )
    .bind(etab)
    .bind(eleves)
    .bind(objet)
    .bind(formulaire_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let majeurs: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_eleve
          WHERE etablissement_id = $1 AND id = ANY($2)
            AND date_naissance IS NOT NULL
            AND date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab($1)",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let majeurs: HashSet<Uuid> = majeurs.into_iter().map(|(id,)| id).collect();
    let mut par_eleve: HashMap<Uuid, Vec<(String, String)>> = HashMap::new();
    for (e, q, d) in rows {
        par_eleve.entry(e).or_default().push((q, d));
    }
    Ok(eleves
        .iter()
        .map(|e| {
            let d: Vec<(&str, &str)> = par_eleve
                .get(e)
                .map(|v| v.iter().map(|(q, d)| (q.as_str(), d.as_str())).collect())
                .unwrap_or_default();
            (*e, etat_effectif(&d, majeurs.contains(e)))
        })
        .collect())
}

impl PgEnt {
    /// Élèves, parmi `eleves`, qui ont **refusé** l'objet (application des
    /// consentements : photo masquée, etc.).
    pub(crate) async fn eleves_ayant_refuse(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
        objet: &str,
    ) -> Result<HashSet<Uuid>, EntDbError> {
        let mut conn = self.pool().acquire().await.map_err(db)?;
        Ok(etats_effectifs(&mut conn, etab, eleves, Some(objet), None)
            .await?
            .into_iter()
            .filter(|(_, e)| *e == "refuse")
            .map(|(id, _)| id)
            .collect())
    }

    async fn exiger_formulaires(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::FormulairesGerer).await
    }

    // ----- Gestion (direction, CPE, administratif) ---------------------------

    pub async fn creer_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        n: NouveauFormulaire,
    ) -> Result<Formulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        if !["consentement", "autorisation", "sondage"].contains(&n.type_formulaire.as_str()) {
            return Err(EntDbError::SaisieInvalide("type de formulaire inconnu"));
        }
        let titre = n.titre.trim().to_string();
        let texte = n.texte.trim().to_string();
        if titre.is_empty() || titre.chars().count() > 200 {
            return Err(EntDbError::SaisieInvalide("titre invalide"));
        }
        if texte.is_empty() || texte.chars().count() > 10_000 {
            return Err(EntDbError::SaisieInvalide("texte invalide"));
        }
        let objet = match (n.type_formulaire.as_str(), n.objet.as_deref().map(str::trim)) {
            ("sondage", None) | ("sondage", Some("")) => None,
            ("sondage", Some(_)) => {
                return Err(EntDbError::SaisieInvalide("un sondage n'a pas d'objet"))
            }
            (_, Some(o))
                if o.len() >= 2
                    && o.len() <= 40
                    && o.starts_with(|c: char| c.is_ascii_lowercase())
                    && o.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') =>
            {
                Some(o.to_string())
            }
            _ => {
                return Err(EntDbError::SaisieInvalide(
                    "objet requis (minuscules, chiffres et _ ; ex. droit_image)",
                ))
            }
        };
        let mut cibles = n.cible_ids.clone();
        cibles.sort();
        cibles.dedup();
        let mut tx = self.pool().begin().await.map_err(db)?;
        valider_audience(&mut tx, etab, &n.audience, &cibles).await?;
        if let Some(o) = &objet {
            // Les versions d'un même objet se créent en série.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 11))")
                .bind(format!("{etab}:{o}"))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let (version,): (i32,) = sqlx::query_as(
            "SELECT COALESCE(max(version), 0) + 1 FROM ent_formulaire
              WHERE etablissement_id = $1 AND objet IS NOT DISTINCT FROM $2 AND $2 IS NOT NULL",
        )
        .bind(etab)
        .bind(&objet)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_formulaire
                (id, etablissement_id, type, objet, version, titre, texte, audience, cible_ids, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(&n.type_formulaire)
        .bind(&objet)
        .bind(version)
        .bind(&titre)
        .bind(&texte)
        .bind(&n.audience)
        .bind(&cibles)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.formulaire(etab, id).await
    }

    async fn formulaire(&self, etab: Uuid, id: Uuid) -> Result<Formulaire, EntDbError> {
        sqlx::query_as(&format!(
            "SELECT {COLONNES_FORMULAIRE} FROM ent_formulaire WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    pub async fn formulaires_gestion(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<Formulaire>, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        sqlx::query_as(&format!(
            "SELECT {COLONNES_FORMULAIRE} FROM ent_formulaire
              WHERE etablissement_id = $1 ORDER BY created_at DESC LIMIT 200"
        ))
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Publie (une version précédente publiée du même objet est archivée) et
    /// prévient les familles concernées.
    pub async fn publier_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Formulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        let f = self.formulaire(etab, id).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let publie = sqlx::query(
            "UPDATE ent_formulaire SET statut = 'publie', publie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'brouillon'",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if publie.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("le formulaire n'est pas un brouillon"));
        }
        if let Some(o) = &f.objet {
            sqlx::query(
                "UPDATE ent_formulaire SET statut = 'archive', archive_le = now()
                  WHERE etablissement_id = $1 AND objet = $2 AND id <> $3 AND statut = 'publie'",
            )
            .bind(etab)
            .bind(o)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_com::FORMULAIRE_PUBLIE.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({
                    "formulaire_id": id, "type": f.type_formulaire, "objet": f.objet,
                    "version": f.version, "audience": f.audience,
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        let eleves: Vec<Uuid> = eleves_de_audience(
            &mut tx,
            etab,
            &f.audience,
            &f.cible_ids,
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        let responsables = responsables_de(&mut tx, etab, &eleves).await?;
        let titre = format!("À compléter : {}", f.titre);
        let corps = "Un formulaire vous attend dans l'ENT (rubrique Formulaires et consentements).";
        let maintenant = Utc::now();
        for r in responsables {
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: r.user_id,
                    kind: "formulaire",
                    titre: &titre,
                    corps,
                    eleve_id: Some(r.eleve_id),
                    notification_id: None,
                    source_type: "systeme",
                    source_id: id,
                    cle_dedup: format!("form:{id}"),
                    droit_requis: Some("lien"),
                    regroupable: true,
                    canaux_voulus: &["email", "push"],
                },
                evt.id,
                maintenant,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.formulaire(etab, id).await
    }

    pub async fn archiver_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Formulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        sqlx::query(
            "UPDATE ent_formulaire SET statut = 'archive', archive_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'archive'",
        )
        .bind(etab)
        .bind(id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        self.formulaire(etab, id).await
    }

    /// Résultats d'un formulaire (FRM-05) : comptages, jamais d'identité.
    pub async fn resultats_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ResultatsFormulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        let f = self.formulaire(etab, id).await?;
        let mut conn = self.pool().acquire().await.map_err(db)?;
        let cibles: Vec<Uuid> = eleves_de_audience(
            &mut conn,
            etab,
            &f.audience,
            &f.cible_ids,
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        let (objet, form) = match &f.objet {
            Some(o) => (Some(o.as_str()), None),
            None => (None, Some(id)),
        };
        let etats = etats_effectifs(&mut conn, etab, &cibles, objet, form).await?;
        let compte = |v: &str| etats.values().filter(|e| **e == v).count();
        Ok(ResultatsFormulaire {
            cibles: cibles.len(),
            accepte: compte("accepte"),
            refuse: compte("refuse"),
            sans_reponse: compte("aucune"),
        })
    }

    // ----- Lecture du personnel (FRM-04) -------------------------------------

    /// Qui consent pour un objet donné, dans une classe : lecture du seul objet
    /// demandé, pour les élèves de la classe.
    pub async fn consentements_classe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet: &str,
        classe_id: Uuid,
    ) -> Result<Vec<ConsentementEleve>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConsentementsLire)
            .await?;
        let mut conn = self.pool().acquire().await.map_err(db)?;
        let eleves: Vec<Uuid> = eleves_de_audience(
            &mut conn,
            etab,
            "classe",
            &[classe_id],
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        let etats = etats_effectifs(&mut conn, etab, &eleves, Some(objet), None).await?;
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, nom, prenom FROM ent_eleve
              WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY nom, prenom",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(noms
            .into_iter()
            .map(|(id, nom, prenom)| ConsentementEleve {
                eleve_id: id,
                nom,
                prenom,
                decision: etats.get(&id).copied().unwrap_or("aucune").to_string(),
            })
            .collect())
    }

    // ----- Famille / élève majeur ---------------------------------------------

    /// Enfants pour lesquels `user_id` peut signer : (eleve_id, prénom, majeur).
    async fn signables(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, String, bool, &'static str)>, EntDbError> {
        let rows: Vec<(Uuid, String, bool, String)> = sqlx::query_as(
            "SELECT e.id, e.prenom,
                    (e.date_naissance IS NOT NULL
                     AND e.date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab($1)) AS majeur,
                    'responsable' AS qualite
               FROM ent_eleve_lien_actif l
               JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.autorite_parentale
             UNION
             SELECT e.id, e.prenom,
                    (e.date_naissance IS NOT NULL
                     AND e.date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab($1)),
                    'eleve_majeur'
               FROM ent_eleve_compte c
               JOIN ent_eleve e ON e.id = c.eleve_id AND e.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.user_id = $2",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        // Mineur : le responsable signe ; majeur : l'élève signe (et lui seul compte).
        Ok(rows
            .into_iter()
            .filter(|(_, _, majeur, q)| (q == "eleve_majeur") == *majeur)
            .map(|(id, prenom, majeur, q)| {
                (id, prenom, majeur, if q == "eleve_majeur" { "eleve_majeur" } else { "responsable" })
            })
            .collect())
    }

    /// Formulaires publiés à traiter (ou déjà traités) pour mes enfants.
    pub async fn formulaires_famille(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<FormulaireFamille>, EntDbError> {
        let enfants = self.signables(etab, user_id).await?;
        if enfants.is_empty() {
            return Ok(Vec::new());
        }
        let formulaires: Vec<Formulaire> = sqlx::query_as(&format!(
            "SELECT {COLONNES_FORMULAIRE} FROM ent_formulaire
              WHERE etablissement_id = $1 AND statut = 'publie'
              ORDER BY publie_le DESC LIMIT 100"
        ))
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = enfants.iter().map(|e| e.0).collect();
        let mut conn = self.pool().acquire().await.map_err(db)?;
        // Mes dernières décisions par (élève, objet ou formulaire).
        let miennes: Vec<(Uuid, Option<String>, Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT ON (eleve_id, COALESCE(objet, formulaire_id::text))
                    eleve_id, objet, formulaire_id, decision
               FROM ent_formulaire_reponse
              WHERE etablissement_id = $1 AND signataire_user_id = $2 AND eleve_id = ANY($3)
              ORDER BY eleve_id, COALESCE(objet, formulaire_id::text), decide_le DESC, id DESC",
        )
        .bind(etab)
        .bind(user_id)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for f in formulaires {
            let cibles = eleves_de_audience(
                &mut conn,
                etab,
                &f.audience,
                &f.cible_ids,
                domain_eleve::horloge::jour_paris(),
            )
            .await?;
            let concernes: Vec<&(Uuid, String, bool, &str)> =
                enfants.iter().filter(|e| cibles.contains(&e.0)).collect();
            if concernes.is_empty() {
                continue;
            }
            let ids_c: Vec<Uuid> = concernes.iter().map(|e| e.0).collect();
            let etats = if f.objet.is_some() {
                etats_effectifs(&mut conn, etab, &ids_c, f.objet.as_deref(), None).await?
            } else {
                HashMap::new()
            };
            for (eleve_id, prenom, _, _) in concernes {
                let ma = miennes
                    .iter()
                    .find(|m| {
                        m.0 == *eleve_id
                            && match &f.objet {
                                Some(o) => m.1.as_deref() == Some(o.as_str()),
                                None => m.2 == f.id,
                            }
                    })
                    .map(|m| m.3.clone());
                sortie.push(FormulaireFamille {
                    formulaire_id: f.id,
                    type_formulaire: f.type_formulaire.clone(),
                    objet: f.objet.clone(),
                    version: f.version,
                    titre: f.titre.clone(),
                    texte: f.texte.clone(),
                    eleve_id: *eleve_id,
                    eleve_prenom: prenom.clone(),
                    a_repondre: ma.is_none(),
                    ma_decision: ma,
                    etat: etats.get(eleve_id).map(|e| (*e).to_string()),
                });
            }
        }
        Ok(sortie)
    }

    /// Enregistre ma décision (signature) pour un enfant. Idempotent : répéter la
    /// même décision n'écrit rien. `retire` suppose une acceptation antérieure.
    pub async fn repondre_formulaire(
        &self,
        etab: Uuid,
        user_id: Uuid,
        formulaire_id: Uuid,
        r: ReponseFormulaire,
    ) -> Result<EtatConsentement, EntDbError> {
        if !["accepte", "refuse", "retire"].contains(&r.decision.as_str()) {
            return Err(EntDbError::SaisieInvalide("décision inconnue"));
        }
        let commentaire = r
            .commentaire
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string);
        if commentaire.as_deref().is_some_and(|c| c.chars().count() > 1000) {
            return Err(EntDbError::SaisieInvalide("commentaire trop long"));
        }
        let f = self.formulaire(etab, formulaire_id).await?;
        if f.statut != "publie" {
            return Err(EntDbError::ConflitDossier("ce formulaire n'est plus ouvert aux réponses"));
        }
        // Qualité du signataire (indiscernable d'un inconnu s'il n'a pas le droit).
        let enfants = self.signables(etab, user_id).await?;
        let Some((_, _, _, qualite)) = enfants.iter().find(|e| e.0 == r.eleve_id) else {
            return Err(introuvable());
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let cibles = eleves_de_audience(
            &mut tx,
            etab,
            &f.audience,
            &f.cible_ids,
            domain_eleve::horloge::jour_paris(),
        )
        .await?;
        if !cibles.contains(&r.eleve_id) {
            return Err(introuvable());
        }
        // Les réponses d'un même signataire pour un même sujet se sérialisent.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 12))")
            .bind(format!("{etab}:{user_id}:{}", r.eleve_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let derniere: Option<(String,)> = sqlx::query_as(
            "SELECT decision FROM ent_formulaire_reponse
              WHERE etablissement_id = $1 AND signataire_user_id = $2 AND eleve_id = $3
                AND ($4::text IS NULL OR objet = $4)
                AND ($5::uuid IS NULL OR formulaire_id = $5)
              ORDER BY decide_le DESC, id DESC LIMIT 1",
        )
        .bind(etab)
        .bind(user_id)
        .bind(r.eleve_id)
        .bind(&f.objet)
        .bind(if f.objet.is_none() { Some(formulaire_id) } else { None })
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let derniere = derniere.map(|d| d.0);
        if r.decision == "retire" {
            if f.type_formulaire == "sondage" {
                return Err(EntDbError::SaisieInvalide("un sondage ne se retire pas"));
            }
            if derniere.as_deref() != Some("accepte") {
                return Err(EntDbError::ConflitDossier("aucun consentement à retirer"));
            }
        }
        if derniere.as_deref() != Some(r.decision.as_str()) {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_formulaire_reponse
                    (id, etablissement_id, formulaire_id, objet, version, eleve_id,
                     signataire_user_id, qualite, decision, commentaire, empreinte)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            )
            .bind(id)
            .bind(etab)
            .bind(formulaire_id)
            .bind(&f.objet)
            .bind(f.version)
            .bind(r.eleve_id)
            .bind(user_id)
            .bind(*qualite)
            .bind(&r.decision)
            .bind(&commentaire)
            .bind(sha256_hex(&format!(
                "{}|{}|{}|{}",
                f.objet.as_deref().unwrap_or(""),
                f.version,
                f.titre,
                f.texte
            )))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let type_evt = match (f.type_formulaire.as_str(), r.decision.as_str()) {
                ("sondage", _) => types_com::FORMULAIRE_REPONDU,
                (_, "accepte") => types_com::CONSENTEMENT_DONNE,
                (_, "refuse") => types_com::CONSENTEMENT_REFUSE,
                _ => types_com::CONSENTEMENT_RETIRE,
            };
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: type_evt.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: formulaire_id,
                eleve_id: Some(r.eleve_id),
                // Identifiants seulement : ni commentaire ni texte libre.
                payload: json!({
                    "formulaire_id": formulaire_id, "objet": f.objet, "version": f.version,
                    "decision_id": id,
                }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await?;
        }
        let etat = etats_effectifs(
            &mut tx,
            etab,
            &[r.eleve_id],
            f.objet.as_deref(),
            if f.objet.is_none() { Some(formulaire_id) } else { None },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EtatConsentement {
            eleve_id: r.eleve_id,
            objet: f.objet.clone().unwrap_or_default(),
            decision: etat.get(&r.eleve_id).copied().unwrap_or("aucune").to_string(),
        })
    }

    /// Photos masquées par un refus du droit à l'image, parmi ces élèves.
    pub async fn eleves_photo_masquee(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
    ) -> Result<BTreeSet<Uuid>, EntDbError> {
        Ok(self
            .eleves_ayant_refuse(etab, eleves, OBJET_DROIT_IMAGE)
            .await?
            .into_iter()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_seul_refus_suffit() {
        let d = [("responsable", "accepte"), ("responsable", "refuse")];
        assert_eq!(etat_effectif(&d, false), "refuse");
        let d = [("responsable", "accepte"), ("responsable", "retire")];
        assert_eq!(etat_effectif(&d, false), "refuse");
    }

    #[test]
    fn acceptation_et_absence() {
        assert_eq!(etat_effectif(&[("responsable", "accepte")], false), "accepte");
        assert_eq!(etat_effectif(&[], false), "aucune");
    }

    #[test]
    fn majeur_seul_son_avis_compte() {
        // Un parent refuse mais l'élève est majeur : seule la décision de l'élève compte.
        let d = [("responsable", "refuse"), ("eleve_majeur", "accepte")];
        assert_eq!(etat_effectif(&d, true), "accepte");
        // Mineur : la décision de l'élève (ancienne qualité) ne compte pas.
        let d = [("eleve_majeur", "accepte")];
        assert_eq!(etat_effectif(&d, false), "aucune");
        let d = [("responsable", "accepte")];
        assert_eq!(etat_effectif(&d, true), "aucune", "parents sans effet sur un majeur");
    }
}
