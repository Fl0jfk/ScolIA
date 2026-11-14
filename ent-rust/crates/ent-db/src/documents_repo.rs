//! Socle 7, lot D2 : **service de la brique `documents`** — commandes et lectures
//! (`docs/socles/07_DOCUMENTS.md` §4.2 D7–D10, §7.1, §8, §9.5, §17, §18, §19).
//!
//! Règles du lot, toutes appliquées ici (jamais dans le navigateur) :
//! * **une seule voie vers un document émis** : [`PgEnt::emettre_document_for_actor`] (§5.2 règle 1) ; fait, journal et
//!   événement s'écrivent dans **une** transaction (`TenantTx`, RLS) ;
//! * **numéro sans trou** : l'incrément du compteur a lieu dans la transaction d'émission, un échec (rendu impossible,
//!   cachet indisponible, stockage en panne) annule tout, numéro compris (§9.5) ;
//! * **versions immuables** (trigger 3701) : une correction est une **nouvelle version** avec motif (D9) ; l'ancienne
//!   est « remplacée » par simple avance de `version_courante` ;
//! * **annulation** (D10) : motif, journal, retrait des mises à disposition, **aucun message automatique** ;
//! * **parents séparés** (D8) : un exemplaire nominatif par destinataire, même numéro, rang distinct ; chacun ne lit
//!   que le sien ; **adresse** : un exemplaire ne porte jamais que l'adresse de son propre destinataire (D7, CHECK 3701) ;
//! * **droits** : actions `documents.*` du catalogue (D-017) ; le **domaine** du document choisit l'action de lecture
//!   (R-D6) ; hors droit, le registre ne sert que le **titre neutre** (RP1-05) ; C4 n'est servi à personne ;
//! * événements `documents.*` **sans titre ni contenu** (identifiants, nature, sensibilité).

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEvent, MetierEventDraft};
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::documents::*;
use crate::outbox::Publier;
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

fn interdit() -> EntDbError {
    EntDbError::Forbidden
}

/// Ce que le moteur de l'émission emprunte à d'autres briques : rendu, cachet et adresse publique de vérification.
#[derive(Clone, Copy)]
pub struct Moteur<'a> {
    pub rendu: &'a dyn RenduDocument,
    pub cachet: &'a dyn CachetEtablissement,
    /// Adresse de base de la page publique de vérification (`https://{etab}.scolia.fr/verifier`), sans barre finale.
    pub url_verification: &'a str,
}

/// Le sujet d'un document : ce dont il parle (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SujetDoc {
    Eleve(Uuid),
    Personne(Uuid),
    Agent(Uuid),
    Etablissement,
}

impl SujetDoc {
    fn type_code(self) -> &'static str {
        match self {
            SujetDoc::Eleve(_) => "eleve",
            SujetDoc::Personne(_) => "personne",
            SujetDoc::Agent(_) => "agent",
            SujetDoc::Etablissement => "etablissement",
        }
    }
}

/// Un destinataire d'exemplaire nominatif (D8). `adresse` est **la sienne** (D7) ; le service ne la vérifie pas contre
/// la protection d'un tiers : il garantit seulement qu'elle n'est jamais imprimée sur l'exemplaire d'un autre.
#[derive(Debug, Clone, Default)]
pub struct DestinataireDoc {
    pub personne_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub nom: Option<String>,
    pub adresse: Option<String>,
    /// Mettre l'exemplaire dans le coffre de cette personne.
    pub au_coffre: bool,
}

#[derive(Debug, Clone)]
pub struct DemandeEmission {
    pub nature_code: String,
    pub sujet: SujetDoc,
    pub titre: String,
    pub annee_scolaire_id: Option<Uuid>,
    pub donnees: Vec<DonneeRendu>,
    pub destinataires: Vec<DestinataireDoc>,
    /// Fait d'origine (`brique`, `type`, `id`) : lu, jamais relié par clé étrangère (§5.3).
    pub source: Option<(String, String, Uuid)>,
    /// Agent habilité au nom de qui l'établissement émet (mention « par délégation »).
    pub au_nom_de_agent_id: Option<Uuid>,
    pub langue: String,
}

#[derive(Debug, Clone)]
pub struct DocumentEmis {
    pub document_id: Uuid,
    pub version_id: Uuid,
    pub numero_version: i32,
    pub numero_affiche: String,
    pub code_verification: Option<String>,
    pub exemplaires: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DemandeReemission {
    pub document_id: Uuid,
    pub motif: String,
    pub donnees: Vec<DonneeRendu>,
    pub destinataires: Vec<DestinataireDoc>,
    pub au_nom_de_agent_id: Option<Uuid>,
}

/// Un document tel que le sert la lecture.
#[derive(Debug, Clone)]
pub struct DocumentLu {
    pub document_id: Uuid,
    pub nature_code: String,
    pub etat: String,
    pub numero_version: i32,
    pub version_courante: i32,
    pub empreinte_sha256: String,
    pub exemplaire_rang: Option<i32>,
    pub octets: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct FiltreRegistre {
    pub nature_code: Option<String>,
    pub eleve_id: Option<Uuid>,
    pub etat: Option<String>,
    pub limite: Option<i64>,
}

/// Une ligne du registre : le titre véritable ou, hors droit, le titre neutre (RP1-05).
#[derive(Debug, Clone)]
pub struct LigneRegistre {
    pub document_id: Uuid,
    pub nature_code: String,
    pub numero_affiche: Option<String>,
    pub etat: String,
    pub titre: String,
    pub titre_neutre_servi: bool,
    pub sensibilite: String,
    pub eleve_id: Option<Uuid>,
    pub version_courante: i32,
    pub cree_le: DateTime<Utc>,
}

/// Identité des documents d'un établissement (§8.2) : ce que l'établissement ajuste, et rien d'autre.
#[derive(Debug, Clone)]
pub struct IdentiteDocuments {
    pub style_code: String,
    pub couleur_principale: Option<String>,
    pub mentions_pied: Option<String>,
    pub fenetre_enveloppe: String,
}

pub const STYLES: &[&str] = &["classique", "contemporaine", "epuree", "institutionnelle"];

#[derive(Debug, Clone)]
pub struct ParamNature {
    pub active: bool,
    pub style_code: Option<String>,
    pub textes_encadres: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct NouvelleEntite {
    pub type_code: String,
    pub raison_sociale: String,
    pub adresse: Option<String>,
    pub mentions_pied: Option<String>,
    /// (`uai` | `siret` | `nda`, valeur)
    pub identifiants: Vec<(String, String)>,
}

// ───────────────────────── Accès internes ─────────────────────────

#[allow(clippy::too_many_arguments)]
async fn journaliser(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    document: Option<Uuid>,
    version: Option<i32>,
    action: &str,
    etats: (Option<&str>, Option<&str>),
    cible: Option<&str>,
    motif: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_doc_journal
             (id, etablissement_id, document_id, numero_version, action, acteur_user_id, etat_avant, etat_apres, cible, motif)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(document)
    .bind(version)
    .bind(action)
    .bind(a.user_id)
    .bind(etats.0)
    .bind(etats.1)
    .bind(cible)
    .bind(motif)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn publier(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    type_name: &str,
    document: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
) -> Result<MetierEvent, EntDbError> {
    Ok(conn
        .publier(MetierEventDraft {
            etablissement_id: a.etablissement_id,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Documents,
            aggregate_id: document,
            eleve_id,
            payload,
            actor_user_id: Some(a.user_id.to_string()),
        })
        .await?)
}

/// Normalise un texte libre : rogné, non vide, borné.
fn texte(v: &str, min: usize, max: usize) -> Result<String, EntDbError> {
    let t = v.trim();
    let n = t.chars().count();
    if n < min || n > max || t.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide());
    }
    Ok(t.to_string())
}

struct LigneDocument {
    id: Uuid,
    nature_code: String,
    entite_id: Uuid,
    eleve_id: Option<Uuid>,
    sensibilite: String,
    domaine: String,
    titre: String,
    titre_neutre: String,
    etat: String,
    serie_code: Option<String>,
    numero_annee: Option<i32>,
    numero: Option<i32>,
    version_courante: i32,
    cree_le: DateTime<Utc>,
}

const COLONNES_DOCUMENT: &str = "id, nature_code, entite_id, eleve_id, sensibilite, domaine,
     titre, titre_neutre, etat, serie_code, numero_annee, numero, version_courante,
     cree_le";

fn ligne_document(r: &sqlx::postgres::PgRow) -> LigneDocument {
    LigneDocument {
        id: r.get("id"),
        nature_code: r.get("nature_code"),
        entite_id: r.get("entite_id"),
        eleve_id: r.get("eleve_id"),
        sensibilite: r.get("sensibilite"),
        domaine: r.get("domaine"),
        titre: r.get("titre"),
        titre_neutre: r.get("titre_neutre"),
        etat: r.get("etat"),
        serie_code: r.get("serie_code"),
        numero_annee: r.get("numero_annee"),
        numero: r.get("numero"),
        version_courante: r.get("version_courante"),
        cree_le: r.get("cree_le"),
    }
}

impl LigneDocument {
    fn numero_affiche(&self) -> Option<String> {
        Some(numero_affiche(
            self.numero_annee?,
            self.serie_code.as_deref()?,
            self.numero?,
        ))
    }
}

/// L'entité « établissement » : créée au premier besoin (une seule par établissement, index partiel 3700), au nom de
/// l'établissement lu au registre racine. Idempotent et sans verrou durable.
pub(crate) async fn assurer_entite_etablissement(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<Uuid, EntDbError> {
    sqlx::query(
        "INSERT INTO ent_doc_entite (id, etablissement_id, type, raison_sociale)
         SELECT $1, $2, 'etablissement', COALESCE(NULLIF(btrim(e.nom), ''), 'Établissement')
           FROM (SELECT 1) AS une LEFT JOIN ent_etablissement e ON e.id = $2
         ON CONFLICT (etablissement_id) WHERE type = 'etablissement' DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_doc_entite WHERE etablissement_id = $1 AND type = 'etablissement'",
    )
    .bind(etab)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// Entité émettrice d'une série : celle de la famille si l'établissement en a désigné une, sinon l'établissement.
async fn entite_de_serie(
    conn: &mut PgConnection,
    etab: Uuid,
    serie: Serie,
) -> Result<Uuid, EntDbError> {
    let defaut = assurer_entite_etablissement(&mut *conn, etab).await?;
    let choisie: Option<(Uuid,)> = sqlx::query_as(
        "SELECT f.entite_id FROM ent_doc_entite_famille f
           JOIN ent_doc_entite e ON e.etablissement_id = f.etablissement_id AND e.id = f.entite_id AND e.actif
          WHERE f.etablissement_id = $1 AND f.famille_nature = $2",
    )
    .bind(etab)
    .bind(serie.famille())
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(choisie.map(|c| c.0).unwrap_or(defaut))
}

impl PgEnt {
    /// L'acteur est du personnel et possède l'action sur l'établissement entier.
    pub(crate) async fn exiger_action_documents(
        &self,
        a: &Acteur<'_>,
        action: Action,
    ) -> Result<Role, EntDbError> {
        let r = Role::parse(a.role).ok_or_else(interdit)?;
        if !r.is_staff()
            || !self
                .a_action_etablissement(a.etablissement_id, a.user_id, a.role, action)
                .await?
        {
            return Err(interdit());
        }
        Ok(r)
    }

    /// Droit sur le sujet d'un document : l'action est exercée sur l'élève (portée) ou sur l'établissement.
    pub(crate) async fn doc_peut_sur_sujet(
        &self,
        a: &Acteur<'_>,
        action: Action,
        eleve_id: Option<Uuid>,
    ) -> Result<bool, EntDbError> {
        let r = match Role::parse(a.role) {
            Some(r) if r.is_staff() => r,
            _ => return Ok(false),
        };
        let _ = r;
        match eleve_id {
            Some(e) => {
                self.autoriser_eleve(a.etablissement_id, a.user_id, a.role, action, e)
                    .await
            }
            None => {
                self.a_action_etablissement(a.etablissement_id, a.user_id, a.role, action)
                    .await
            }
        }
    }

    // ───────────────────────── Paramétrage (§8.2, §9.3) ─────────────────────────

    /// Identité des documents de l'établissement (valeurs par défaut si jamais renseignée).
    pub async fn identite_documents(&self, etab: Uuid) -> Result<IdentiteDocuments, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = lire_identite(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        Ok(r.0)
    }

    /// Modifie l'identité (`documents.modeles.parametrer`) : style parmi les quatre styles maison, couleur refusée si
    /// le contraste sur blanc n'atteint pas AA, mentions bornées. Journalisé ; chaque modification se voit partout.
    pub async fn parametrer_identite_for_actor(
        &self,
        a: &Acteur<'_>,
        style_code: &str,
        couleur_principale: Option<&str>,
        mentions_pied: Option<&str>,
        fenetre_enveloppe: &str,
    ) -> Result<IdentiteDocuments, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer)
            .await?;
        if !STYLES.contains(&style_code) || !matches!(fenetre_enveloppe, "gauche" | "droite") {
            return Err(refus(
                RefusMetier::Invalide,
                "style ou position de fenêtre inconnu",
            ));
        }
        let couleur =
            match couleur_principale.map(str::trim).filter(|c| !c.is_empty()) {
                Some(c) if couleur_acceptable(c) => Some(c.to_ascii_uppercase()),
                Some(_) => return Err(refus(
                    RefusMetier::Invalide,
                    "couleur refusée : le contraste sur blanc n'atteint pas le niveau AA (4,5:1)",
                )),
                None => None,
            };
        let mentions = match mentions_pied.map(str::trim).filter(|m| !m.is_empty()) {
            Some(m) => Some(texte(m, 1, 500)?),
            None => None,
        };
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let (avant, _) = lire_identite(&mut tx, a.etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_doc_identite (etablissement_id, style_code, couleur_principale, mentions_pied, fenetre_enveloppe, modifie_par)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT (etablissement_id) DO UPDATE SET
                 style_code = EXCLUDED.style_code, couleur_principale = EXCLUDED.couleur_principale,
                 mentions_pied = EXCLUDED.mentions_pied, fenetre_enveloppe = EXCLUDED.fenetre_enveloppe,
                 version = ent_doc_identite.version + 1, modifie_le = now(), modifie_par = EXCLUDED.modifie_par",
        )
        .bind(a.etablissement_id)
        .bind(style_code)
        .bind(&couleur)
        .bind(&mentions)
        .bind(fenetre_enveloppe)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_doc_journal (id, etablissement_id, action, acteur_user_id, cible, avant, apres)
             VALUES ($1,$2,'parametrage_modifie',$3,'identite',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(format!("style={} couleur={}", avant.style_code, avant.couleur_principale.as_deref().unwrap_or("-")))
        .bind(format!("style={style_code} couleur={}", couleur.as_deref().unwrap_or("-")))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(IdentiteDocuments {
            style_code: style_code.to_string(),
            couleur_principale: couleur,
            mentions_pied: mentions,
            fenetre_enveloppe: fenetre_enveloppe.to_string(),
        })
    }

    /// Active ou désactive une nature, déroge au style (signalé) et fixe ses textes encadrés (0 à 2 emplacements, 400
    /// caractères au plus, sans mise en forme).
    pub async fn parametrer_nature_for_actor(
        &self,
        a: &Acteur<'_>,
        nature_code: &str,
        active: bool,
        style_code: Option<&str>,
        textes_encadres: &[(&str, &str)],
    ) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer)
            .await?;
        if nature(nature_code).is_none() {
            return Err(introuvable());
        }
        if style_code.is_some_and(|s| !STYLES.contains(&s)) || textes_encadres.len() > 2 {
            return Err(refus(
                RefusMetier::Invalide,
                "style inconnu ou plus de deux textes encadrés",
            ));
        }
        let mut textes = Vec::new();
        for (emplacement, t) in textes_encadres {
            if !crate::demandes_files::code_ok(emplacement, 40) {
                return Err(invalide());
            }
            textes.push((emplacement.to_string(), texte(t, 1, 400)?));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_doc_nature_param (etablissement_id, nature_code, active, style_code, maj_par)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (etablissement_id, nature_code) DO UPDATE SET
                 active = EXCLUDED.active, style_code = EXCLUDED.style_code, maj_le = now(), maj_par = EXCLUDED.maj_par",
        )
        .bind(a.etablissement_id)
        .bind(nature_code)
        .bind(active)
        .bind(style_code)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "DELETE FROM ent_doc_nature_texte WHERE etablissement_id = $1 AND nature_code = $2",
        )
        .bind(a.etablissement_id)
        .bind(nature_code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (emplacement, t) in &textes {
            sqlx::query(
                "INSERT INTO ent_doc_nature_texte (etablissement_id, nature_code, emplacement, texte, maj_par)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(a.etablissement_id)
            .bind(nature_code)
            .bind(emplacement)
            .bind(t)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journaliser(
            &mut tx,
            a,
            None,
            None,
            "parametrage_modifie",
            (None, None),
            Some(nature_code),
            Some(if active {
                "nature activée"
            } else {
                "nature désactivée"
            }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn param_nature(
        &self,
        etab: Uuid,
        nature_code: &str,
    ) -> Result<ParamNature, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = lire_param_nature(&mut tx, etab, nature_code).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Crée une entité émettrice (organisme de gestion, de formation) et ses identifiants (R-D13).
    pub async fn creer_entite_for_actor(
        &self,
        a: &Acteur<'_>,
        e: NouvelleEntite,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer)
            .await?;
        if !matches!(
            e.type_code.as_str(),
            "organisme_gestion" | "organisme_formation" | "ensemble"
        ) {
            // L'entité « établissement » est unique et créée par le service lui-même.
            return Err(refus(RefusMetier::Invalide, "type d'entité inconnu"));
        }
        let raison = texte(&e.raison_sociale, 1, 200)?;
        for (t, v) in &e.identifiants {
            if !matches!(t.as_str(), "uai" | "siret" | "nda") {
                return Err(invalide());
            }
            texte(v, 1, 40)?;
        }
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_doc_entite (id, etablissement_id, type, raison_sociale, adresse, mentions_pied)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(&e.type_code)
        .bind(&raison)
        .bind(e.adresse.as_deref().map(|x| texte(x, 1, 500)).transpose()?)
        .bind(e.mentions_pied.as_deref().map(|x| texte(x, 1, 500)).transpose()?)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (t, v) in &e.identifiants {
            sqlx::query("INSERT INTO ent_doc_entite_identifiant (etablissement_id, entite_id, type, valeur) VALUES ($1,$2,$3,$4)")
                .bind(a.etablissement_id)
                .bind(id)
                .bind(t)
                .bind(v.trim())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        journaliser(
            &mut tx,
            a,
            None,
            None,
            "entite_creee",
            (None, None),
            Some(&e.type_code),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(id)
    }

    /// Désigne l'entité émettrice d'une famille de natures (R-D13) : ses numéros ont leur propre série.
    pub async fn affecter_famille_entite_for_actor(
        &self,
        a: &Acteur<'_>,
        famille: &str,
        entite_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let existe: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_doc_entite WHERE etablissement_id = $1 AND id = $2 AND actif",
        )
        .bind(a.etablissement_id)
        .bind(entite_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if existe.is_none() {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_doc_entite_famille (etablissement_id, famille_nature, entite_id) VALUES ($1,$2,$3)
             ON CONFLICT (etablissement_id, famille_nature) DO UPDATE SET entite_id = EXCLUDED.entite_id, maj_le = now()",
        )
        .bind(a.etablissement_id)
        .bind(famille)
        .bind(entite_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            // Une famille hors liste fermée est une saisie invalide, pas une panne.
            if matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23514")) {
                invalide()
            } else {
                db(e)
            }
        })?;
        journaliser(
            &mut tx,
            a,
            None,
            None,
            "parametrage_modifie",
            (None, None),
            Some(famille),
            Some("entité de la famille"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Émission (§9.5) ─────────────────────────

    /// **Émet** un document : une seule transaction pour le compteur, le rendu, le scellement, l'écriture de la
    /// version et des exemplaires, les mises à disposition, le journal et `documents.emis`. Un échec annule tout
    /// (aucun numéro perdu, aucun document sans cachet). Droits : `documents.emettre` + les actions de la nature.
    pub async fn emettre_document_for_actor(
        &self,
        a: &Acteur<'_>,
        d: DemandeEmission,
        moteur: &Moteur<'_>,
    ) -> Result<DocumentEmis, EntDbError> {
        let n = nature(&d.nature_code)
            .ok_or_else(|| refus(RefusMetier::Invalide, "nature de document inconnue"))?;
        let titre = texte(&d.titre, 1, 200)?;
        let eleve_id = match d.sujet {
            SujetDoc::Eleve(e) => Some(e),
            _ => None,
        };
        // 1. Droits : l'action générale ET celles de la nature, sur le sujet (portée).
        for action in std::iter::once(&Action::DocumentsEmettre).chain(n.actions_emission.iter()) {
            if !self.doc_peut_sur_sujet(a, *action, eleve_id).await? {
                return Err(interdit());
            }
        }
        if n.destinataires == Destinataires::TitulairesEtEleve && d.destinataires.is_empty() {
            return Err(refus(
                RefusMetier::Conflit,
                "aucun destinataire : le document n'a personne à qui parvenir",
            ));
        }
        if d.destinataires.len() > 8 {
            return Err(invalide());
        }

        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        if !lire_param_nature(&mut tx, a.etablissement_id, n.code)
            .await?
            .active
        {
            return Err(refus(
                RefusMetier::Conflit,
                "cette nature de document n'est pas activée dans l'établissement",
            ));
        }
        self.doc_verifier_sujet(&mut tx, a.etablissement_id, d.sujet)
            .await?;
        let (jour, annee): (NaiveDate, i32) = sqlx::query_as(
            "SELECT ent_aujourdhui_etab($1), EXTRACT(YEAR FROM ent_aujourdhui_etab($1))::int",
        )
        .bind(a.etablissement_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let entite = entite_de_serie(&mut tx, a.etablissement_id, n.serie).await?;
        let numero: i32 = sqlx::query_scalar("SELECT ent_doc_numero_suivant($1,$2,$3,$4)")
            .bind(a.etablissement_id)
            .bind(entite)
            .bind(n.serie.code())
            .bind(annee)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let code = (n.verification != Verification::Aucune).then(generer_code_verification);
        let document_id = Uuid::new_v4();
        let version_id = Uuid::new_v4();
        let affiche = numero_affiche(annee, n.serie.code(), numero);

        let rendus = self
            .doc_rendre_exemplaires(
                &mut tx,
                a,
                n,
                &d.donnees,
                &d.destinataires,
                d.au_nom_de_agent_id,
                entite,
                &affiche,
                code.as_deref(),
                1,
                None,
                jour,
                &d.langue,
                moteur,
            )
            .await?;

        // Écriture : document → version → données → exemplaires → mises à disposition.
        sqlx::query(
            "INSERT INTO ent_doc_document
                 (id, etablissement_id, nature_code, entite_id, sujet_type, sujet_id, eleve_id, personne_id, agent_id,
                  annee_scolaire_id, origine, sensibilite, domaine, titre, titre_neutre, etat, serie_code, numero_annee, numero,
                  verification_publique, version_courante, source_brique, source_type, source_id,
                  categorie_conservation, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'emis',$11,$12,$13,$14,'emis',$15,$16,$17,$18,1,$19,$20,$21,$22,$23)",
        )
        .bind(document_id)
        .bind(a.etablissement_id)
        .bind(n.code)
        .bind(entite)
        .bind(d.sujet.type_code())
        .bind(match d.sujet {
            SujetDoc::Eleve(x) | SujetDoc::Personne(x) | SujetDoc::Agent(x) => x,
            SujetDoc::Etablissement => a.etablissement_id,
        })
        .bind(eleve_id)
        .bind(match d.sujet { SujetDoc::Personne(x) => Some(x), _ => None })
        .bind(match d.sujet { SujetDoc::Agent(x) => Some(x), _ => None })
        .bind(d.annee_scolaire_id)
        .bind(n.sensibilite.code())
        .bind(n.domaine.code())
        .bind(&titre)
        .bind(n.titre_neutre)
        .bind(n.serie.code())
        .bind(annee)
        .bind(numero)
        .bind(n.verification.code())
        .bind(d.source.as_ref().map(|s| s.0.clone()))
        .bind(d.source.as_ref().map(|s| s.1.clone()))
        .bind(d.source.as_ref().map(|s| s.2))
        .bind(n.conservation)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let exemplaires = self
            .doc_ecrire_version(
                &mut tx,
                a,
                n,
                document_id,
                version_id,
                1,
                None,
                code.as_deref(),
                &rendus,
                &d.donnees,
                &d.destinataires,
                d.au_nom_de_agent_id,
                &d.langue,
                eleve_id,
            )
            .await?;
        journaliser(
            &mut tx,
            a,
            Some(document_id),
            Some(1),
            "emis",
            (None, Some("emis")),
            Some(n.code),
            None,
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            "documents.emis",
            document_id,
            eleve_id,
            json!({ "nature": n.code, "sensibilite": n.sensibilite.code(), "version": 1 }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(DocumentEmis {
            document_id,
            version_id,
            numero_version: 1,
            numero_affiche: affiche,
            code_verification: code,
            exemplaires,
        })
    }

    /// Le sujet existe dans l'établissement (clé composite) ; sinon refus propre plutôt qu'une violation de clé.
    async fn doc_verifier_sujet(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        s: SujetDoc,
    ) -> Result<(), EntDbError> {
        let sql = match s {
            SujetDoc::Eleve(_) => "SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
            SujetDoc::Personne(_) => {
                "SELECT 1 FROM ent_personne WHERE etablissement_id = $1 AND id = $2"
            }
            SujetDoc::Agent(_) => "SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2",
            SujetDoc::Etablissement => return Ok(()),
        };
        let id = match s {
            SujetDoc::Eleve(x) | SujetDoc::Personne(x) | SujetDoc::Agent(x) => x,
            SujetDoc::Etablissement => return Ok(()),
        };
        let r: Option<(i32,)> = sqlx::query_as(sql)
            .bind(etab)
            .bind(id)
            .fetch_optional(conn)
            .await
            .map_err(db)?;
        r.map(|_| ()).ok_or_else(introuvable)
    }

    /// Rend et scelle la version de base et un exemplaire par destinataire ; **stocke** les objets (clés opaques) et
    /// les inscrit au registre dans la transaction. Un destinataire ne reçoit jamais que sa propre adresse (D7).
    #[allow(clippy::too_many_arguments)]
    async fn doc_rendre_exemplaires(
        &self,
        tx: &mut PgConnection,
        a: &Acteur<'_>,
        n: &'static NatureDecl,
        donnees: &[DonneeRendu],
        destinataires: &[DestinataireDoc],
        au_nom_de: Option<Uuid>,
        entite: Uuid,
        affiche: &str,
        code: Option<&str>,
        numero_version: i32,
        motif: Option<&str>,
        jour: NaiveDate,
        langue: &str,
        moteur: &Moteur<'_>,
    ) -> Result<Vec<ObjetRendu>, EntDbError> {
        let identite = lire_identite(tx, a.etablissement_id).await?;
        let param = lire_param_nature(tx, a.etablissement_id, n.code).await?;
        let (raison, identifiants) = lire_entite(tx, a.etablissement_id, entite).await?;
        let style = param
            .style_code
            .clone()
            .unwrap_or_else(|| identite.0.style_code.clone());
        let lisible = code.map(code_lisible);
        let url = code.map(|c| format!("{}/{}", moteur.url_verification.trim_end_matches('/'), c));
        let date = jour.format("%Y-%m-%d").to_string();
        let par_delegation_de = match au_nom_de {
            Some(agent) => Some(nom_agent(tx, a.etablissement_id, agent).await?),
            None => None,
        };
        let mut sorties = Vec::new();
        // rang 0 = version de base (sans destinataire) ; rangs 1.. = exemplaires nominatifs.
        let cibles: Vec<Option<&DestinataireDoc>> = std::iter::once(None)
            .chain(destinataires.iter().map(Some))
            .collect();
        for (i, dest) in cibles.iter().enumerate() {
            let demande = DemandeRendu {
                nature: n,
                style: &style,
                couleur_principale: identite.0.couleur_principale.as_deref(),
                mentions_pied: identite.0.mentions_pied.as_deref(),
                entite_raison_sociale: &raison,
                entite_identifiants: &identifiants,
                donnees,
                textes_encadres: &param.textes_encadres,
                numero_affiche: affiche,
                code_verification: lisible.as_deref(),
                url_verification: url.as_deref(),
                numero_version,
                motif,
                date_emission: &date,
                par_delegation_de: par_delegation_de.as_deref(),
                destinataire_nom: dest.and_then(|x| x.nom.as_deref()),
                // D7 : SON adresse seulement ; la version de base n'en porte aucune.
                destinataire_adresse: dest.and_then(|x| x.adresse.as_deref()),
                langue,
            };
            let rendu = moteur.rendu.rendre(&demande).map_err(|e| match e {
                ErreurRendu::Indisponible => refus(
                    RefusMetier::Conflit,
                    "moteur de rendu indisponible : émission impossible",
                ),
                ErreurRendu::Impossible(_) => {
                    refus(RefusMetier::Conflit, "rendu du document impossible")
                }
            })?;
            if rendu.octets.is_empty() || rendu.nb_pages == 0 {
                return Err(refus(RefusMetier::Conflit, "rendu du document vide"));
            }
            let scelle = moteur
                .cachet
                .sceller(a.etablissement_id, &rendu.octets)
                .map_err(|e| match e {
                    ErreurCachet::Indisponible => refus(
                        RefusMetier::Conflit,
                        "cachet de l'établissement indisponible : émission impossible",
                    ),
                    ErreurCachet::Impossible(_) => {
                        refus(RefusMetier::Conflit, "scellement du document impossible")
                    }
                })?;
            let cle = ent_object::ObjectStore::key_for(
                a.etablissement_id,
                "documents",
                Uuid::new_v4(),
                "pdf",
            );
            let info = self.objet_put(&cle, &scelle.octets).await?;
            crate::objets::enregistrer_objet(
                &mut *tx,
                a.etablissement_id,
                &info,
                Some("application/pdf"),
            )
            .await?;
            sorties.push(ObjetRendu {
                rang: i as i32,
                cle_objet: cle,
                sha256: info.sha256.clone(),
                nb_pages: rendu.nb_pages as i32,
                cachet_cle_ref: scelle.cle_ref,
                style: style.clone(),
                identite_version: identite.1,
            });
        }
        Ok(sorties)
    }

    /// Écrit la version, ses données figées (scellées), ses exemplaires et leurs mises à disposition.
    #[allow(clippy::too_many_arguments)]
    async fn doc_ecrire_version(
        &self,
        tx: &mut PgConnection,
        a: &Acteur<'_>,
        n: &'static NatureDecl,
        document_id: Uuid,
        version_id: Uuid,
        numero_version: i32,
        motif: Option<&str>,
        code: Option<&str>,
        rendus: &[ObjetRendu],
        donnees: &[DonneeRendu],
        destinataires: &[DestinataireDoc],
        au_nom_de: Option<Uuid>,
        langue: &str,
        eleve_id: Option<Uuid>,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let base = &rendus[0];
        sqlx::query(
            "INSERT INTO ent_doc_version
                 (id, etablissement_id, document_id, numero_version, cle_objet, empreinte_sha256, nb_pages, langue,
                  trame_code, trame_version, style_code, identite_version, motif, cachet_cle_ref, emise_par,
                  au_nom_de_agent_id, par_delegation, code_verification)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,1,$10,$11,$12,$13,$14,$15,$16,$17)",
        )
        .bind(version_id)
        .bind(a.etablissement_id)
        .bind(document_id)
        .bind(numero_version)
        .bind(&base.cle_objet)
        .bind(&base.sha256)
        .bind(base.nb_pages)
        .bind(langue)
        .bind(n.code)
        .bind(&base.style)
        .bind(base.identite_version)
        .bind(motif)
        .bind(&base.cachet_cle_ref)
        .bind(a.user_id)
        .bind(au_nom_de)
        .bind(au_nom_de.is_some())
        .bind(code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let coffre = ChampsSensibles::depuis_env();
        for dn in donnees {
            let valeur = coffre
                .sceller("ent_doc_version_donnee", "valeur", version_id, &dn.valeur)
                .map_err(EntDbError::Domain)?;
            sqlx::query("INSERT INTO ent_doc_version_donnee (etablissement_id, version_id, cle, valeur) VALUES ($1,$2,$3,$4)")
                .bind(a.etablissement_id)
                .bind(version_id)
                .bind(&dn.cle)
                .bind(valeur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        // Exemplaires : un par destinataire ; à défaut (nature d'établissement), un seul exemplaire interne.
        let mut exemplaires = Vec::new();
        let nominatifs: Vec<Option<&DestinataireDoc>> = if destinataires.is_empty() {
            vec![None]
        } else {
            destinataires.iter().map(Some).collect()
        };
        for (i, dest) in nominatifs.iter().enumerate() {
            let rang = i as i32 + 1;
            // Rendu de rang `rang` si des destinataires existent ; sinon on réutilise la version de base.
            let r = if destinataires.is_empty() {
                base
            } else {
                &rendus[rang as usize]
            };
            let id = Uuid::new_v4();
            let personne = dest.and_then(|x| x.personne_id);
            sqlx::query(
                "INSERT INTO ent_doc_exemplaire
                     (id, etablissement_id, version_id, rang, destinataire_personne_id, destinataire_agent_id,
                      adresse_de_personne_id, cle_objet, empreinte_sha256)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(id)
            .bind(a.etablissement_id)
            .bind(version_id)
            .bind(rang)
            .bind(personne)
            .bind(dest.and_then(|x| x.agent_id))
            .bind(dest.and_then(|x| x.adresse.as_ref().and(personne)))
            .bind(&r.cle_objet)
            .bind(&r.sha256)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            exemplaires.push(id);
            if let Some(dst) = dest {
                if dst.au_coffre && (dst.personne_id.is_some() || dst.agent_id.is_some()) {
                    sqlx::query(
                        "INSERT INTO ent_doc_mise_a_disposition
                             (id, etablissement_id, document_id, exemplaire_id, personne_id, agent_id, canal, cree_par)
                         VALUES ($1,$2,$3,$4,$5,$6,'coffre',$7)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(a.etablissement_id)
                    .bind(document_id)
                    .bind(id)
                    .bind(dst.personne_id)
                    .bind(dst.agent_id)
                    .bind(a.user_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    journaliser(
                        tx,
                        a,
                        Some(document_id),
                        Some(numero_version),
                        "mis_a_disposition",
                        (None, None),
                        Some("coffre"),
                        None,
                    )
                    .await?;
                    publier(
                        tx,
                        a,
                        "documents.mis_a_disposition",
                        document_id,
                        eleve_id,
                        json!({ "nature": n.code, "canal": "coffre", "rang": rang }),
                    )
                    .await?;
                }
            }
        }
        Ok(exemplaires)
    }

    // ───────────────────────── Réémission (D9) ─────────────────────────

    /// **Réémet** un document émis avec une erreur (classe fausse) : nouvelle version, motif obligatoire, même
    /// numéro et même code ; l'ancienne version passe « remplacée » (le code répond « remplacé le … ») ; le coffre
    /// montre la nouvelle (anciennes mises à disposition retirées, nouvelles posées). Refusée pour une nature dont la
    /// correction est un document compensatoire (facture : avoir).
    pub async fn reemettre_document_for_actor(
        &self,
        a: &Acteur<'_>,
        r: DemandeReemission,
        moteur: &Moteur<'_>,
    ) -> Result<DocumentEmis, EntDbError> {
        let motif = texte(&r.motif, 3, 500)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let ligne = sqlx::query(&format!(
            "SELECT {COLONNES_DOCUMENT} FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(r.document_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let doc = ligne_document(&ligne);
        let n = nature(&doc.nature_code).ok_or_else(introuvable)?;
        for action in std::iter::once(&Action::DocumentsEmettre).chain(n.actions_emission.iter()) {
            if !self.doc_peut_sur_sujet(a, *action, doc.eleve_id).await? {
                return Err(interdit());
            }
        }
        if doc.etat != "emis" {
            return Err(refus(
                RefusMetier::Conflit,
                "seul un document émis se réémet",
            ));
        }
        if n.correction != Correction::NouvelleVersion {
            return Err(refus(RefusMetier::Conflit, "cette nature se corrige par un document compensatoire, pas par une nouvelle version"));
        }
        if n.destinataires == Destinataires::TitulairesEtEleve && r.destinataires.is_empty() {
            return Err(refus(RefusMetier::Conflit, "aucun destinataire"));
        }
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
            .bind(a.etablissement_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let (langue,): (String,) = sqlx::query_as(
            "SELECT langue FROM ent_doc_version WHERE etablissement_id = $1 AND document_id = $2 AND numero_version = $3",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .bind(doc.version_courante)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let nouvelle = doc.version_courante + 1;
        let version_id = Uuid::new_v4();
        let affiche = doc.numero_affiche().ok_or_else(conflit)?;
        // Une version, un code : le code de l'ancienne version répondra « remplacé » (D9).
        let code = (n.verification != Verification::Aucune).then(generer_code_verification);
        let rendus = self
            .doc_rendre_exemplaires(
                &mut tx,
                a,
                n,
                &r.donnees,
                &r.destinataires,
                r.au_nom_de_agent_id,
                doc.entite_id,
                &affiche,
                code.as_deref(),
                nouvelle,
                Some(&motif),
                jour,
                &langue,
                moteur,
            )
            .await?;
        // Avance d'abord la version courante (le trigger refuse tout saut) puis écrit la nouvelle version.
        sqlx::query(
            "UPDATE ent_doc_document SET version_courante = version_courante + 1, maj_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Les anciennes mises à disposition sont retirées (motif), les nouvelles posées par l'écriture de la version.
        sqlx::query(
            "UPDATE ent_doc_mise_a_disposition SET retiree_le = now(), retrait_motif = $3
              WHERE etablissement_id = $1 AND document_id = $2 AND retiree_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .bind(format!("remplacé par la version {nouvelle}"))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let exemplaires = self
            .doc_ecrire_version(
                &mut tx,
                a,
                n,
                doc.id,
                version_id,
                nouvelle,
                Some(&motif),
                code.as_deref(),
                &rendus,
                &r.donnees,
                &r.destinataires,
                r.au_nom_de_agent_id,
                &langue,
                doc.eleve_id,
            )
            .await?;
        journaliser(
            &mut tx,
            a,
            Some(doc.id),
            Some(nouvelle),
            "reemis",
            (Some("emis"), Some("emis")),
            Some(n.code),
            Some(&motif),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            "documents.remplace",
            doc.id,
            doc.eleve_id,
            json!({ "nature": n.code, "sensibilite": n.sensibilite.code(), "version": nouvelle }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(DocumentEmis {
            document_id: doc.id,
            version_id,
            numero_version: nouvelle,
            numero_affiche: affiche,
            code_verification: code,
            exemplaires,
        })
    }

    // ───────────────────────── Annulation (D10) ─────────────────────────

    /// **Annule** un document émis par erreur (★) : motif obligatoire, journal, retrait des mises à disposition (le
    /// destinataire ne le voit plus dans son coffre), vérification publique « annulé ». Aucun message n'est envoyé par
    /// le socle 7 : il publie `documents.annule`, le socle 8 propose un message humain.
    pub async fn annuler_document_for_actor(
        &self,
        a: &Acteur<'_>,
        document_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        let motif = texte(motif, 3, 500)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let ligne = sqlx::query(&format!(
            "SELECT {COLONNES_DOCUMENT} FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let doc = ligne_document(&ligne);
        if !self
            .doc_peut_sur_sujet(a, Action::DocumentsAnnuler, doc.eleve_id)
            .await?
        {
            return Err(interdit());
        }
        if doc.etat != "emis" {
            return Err(refus(
                RefusMetier::Conflit,
                "seul un document émis peut être annulé",
            ));
        }
        sqlx::query(
            "UPDATE ent_doc_document SET etat = 'annule', annule_le = now(), annule_par = $3, annule_motif = $4, maj_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .bind(a.user_id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_doc_mise_a_disposition SET retiree_le = now(), retrait_motif = 'document annulé'
              WHERE etablissement_id = $1 AND document_id = $2 AND retiree_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            Some(doc.id),
            Some(doc.version_courante),
            "annule",
            (Some("emis"), Some("annule")),
            Some(&doc.nature_code),
            Some(&motif),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            "documents.annule",
            doc.id,
            doc.eleve_id,
            json!({ "nature": doc.nature_code, "sensibilite": doc.sensibilite }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    // ───────────────────────── Lecture (§17.2) ─────────────────────────

    /// Lit un document : son **titulaire** (mise à disposition en cours) ne lit que **son** exemplaire ; le personnel
    /// lit la version courante (ou une version antérieure) si le droit de son **domaine** (R-D6) couvre le sujet ;
    /// C4 n'est servi à personne. Les lectures de C2 et plus sont tracées (`ent_doc_consultation`) ; la première
    /// ouverture par un destinataire pose l'accusé et publie `documents.ouvert`.
    pub async fn lire_document_for_actor(
        &self,
        a: &Acteur<'_>,
        document_id: Uuid,
        version: Option<i32>,
    ) -> Result<DocumentLu, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let ligne = sqlx::query(&format!(
            "SELECT {COLONNES_DOCUMENT} FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(a.etablissement_id)
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let doc = ligne_document(&ligne);
        if doc.sensibilite == "C4" {
            return Err(interdit());
        }
        // 1. Titulaire d'une mise à disposition en cours : SON exemplaire de la version courante.
        let mad: Option<(Uuid, Uuid, i32, String, String, bool)> = sqlx::query_as(
            "SELECT m.id, ex.id, ex.rang, ex.cle_objet, ex.empreinte_sha256, m.premiere_ouverture_le IS NULL
               FROM ent_doc_mise_a_disposition m
               JOIN ent_doc_exemplaire ex ON ex.etablissement_id = m.etablissement_id AND ex.id = m.exemplaire_id
               JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
               JOIN ent_personne p ON p.etablissement_id = m.etablissement_id AND p.id = m.personne_id
              WHERE m.etablissement_id = $1 AND m.document_id = $2 AND p.user_id = $3
                AND m.retiree_le IS NULL AND (m.jusqu_au IS NULL OR m.jusqu_au > now())
                AND v.numero_version = $4
              ORDER BY ex.rang LIMIT 1",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .bind(a.user_id)
        .bind(doc.version_courante)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((mad_id, _ex, rang, cle, sha, premiere)) = mad {
            if doc.etat != "emis" {
                return Err(introuvable());
            }
            let ev = if premiere {
                sqlx::query("UPDATE ent_doc_mise_a_disposition SET premiere_ouverture_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(a.etablissement_id)
                    .bind(mad_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                Some(
                    publier(
                        &mut tx,
                        a,
                        "documents.ouvert",
                        doc.id,
                        doc.eleve_id,
                        json!({ "nature": doc.nature_code, "sensibilite": doc.sensibilite, "rang": rang }),
                    )
                    .await?,
                )
            } else {
                None
            };
            tx.commit().await.map_err(db)?;
            if let Some(ev) = ev {
                self.diffuser_apres_commit(&[ev]);
            }
            let octets = self
                .lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha))
                .await?;
            return Ok(DocumentLu {
                document_id: doc.id,
                nature_code: doc.nature_code,
                etat: doc.etat,
                numero_version: doc.version_courante,
                version_courante: doc.version_courante,
                empreinte_sha256: sha,
                exemplaire_rang: Some(rang),
                octets,
            });
        }
        // 2. Personnel : le droit du domaine, sur le sujet.
        let domaine = Domaine::parse(&doc.domaine).ok_or_else(interdit)?;
        if !self
            .doc_peut_sur_sujet(a, domaine.action_lecture(), doc.eleve_id)
            .await?
        {
            return Err(interdit());
        }
        if doc.etat == "brouillon" {
            return Err(introuvable());
        }
        let numero = version.unwrap_or(doc.version_courante);
        let v: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, cle_objet, empreinte_sha256 FROM ent_doc_version
              WHERE etablissement_id = $1 AND document_id = $2 AND numero_version = $3",
        )
        .bind(a.etablissement_id)
        .bind(doc.id)
        .bind(numero)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (version_id, cle, sha) = v.ok_or_else(introuvable)?;
        if Sensibilite::parse(&doc.sensibilite).is_some_and(Sensibilite::trace_la_lecture) {
            sqlx::query(
                "INSERT INTO ent_doc_consultation (id, etablissement_id, document_id, version_id, acteur_user_id, profil, canal, motif)
                 VALUES ($1,$2,$3,$4,$5,$6,'web','lecture')",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(doc.id)
            .bind(version_id)
            .bind(a.user_id)
            .bind(a.role)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        let octets = self
            .lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha))
            .await?;
        Ok(DocumentLu {
            document_id: doc.id,
            nature_code: doc.nature_code,
            etat: doc.etat,
            numero_version: numero,
            version_courante: doc.version_courante,
            empreinte_sha256: sha,
            exemplaire_rang: None,
            octets,
        })
    }

    /// Registre des documents émis (`documents.registre.lire`) : le titre véritable si le droit du domaine couvre le
    /// sujet, le **titre neutre** sinon (RP1-05). Les documents C4 n'y paraissent qu'en titre neutre.
    pub async fn registre_for_actor(
        &self,
        a: &Acteur<'_>,
        f: FiltreRegistre,
    ) -> Result<Vec<LigneRegistre>, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsRegistreLire)
            .await?;
        let limite = f.limite.unwrap_or(100).clamp(1, 500);
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let lignes = sqlx::query(&format!(
            "SELECT {COLONNES_DOCUMENT} FROM ent_doc_document
              WHERE etablissement_id = $1 AND etat <> 'brouillon'
                AND ($2::text IS NULL OR nature_code = $2)
                AND ($3::uuid IS NULL OR eleve_id = $3)
                AND ($4::text IS NULL OR etat = $4)
              ORDER BY cree_le DESC, id LIMIT $5"
        ))
        .bind(a.etablissement_id)
        .bind(f.nature_code.as_deref())
        .bind(f.eleve_id)
        .bind(f.etat.as_deref())
        .bind(limite)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut portees: std::collections::HashMap<
            &'static str,
            Option<std::collections::HashSet<Uuid>>,
        > = Default::default();
        let mut sortie = Vec::new();
        for r in &lignes {
            let doc = ligne_document(r);
            let domaine = Domaine::parse(&doc.domaine).ok_or_else(interdit)?;
            let action = domaine.action_lecture();
            if !portees.contains_key(action.code()) {
                let p = self
                    .eleves_autorises(a.etablissement_id, a.user_id, a.role, action)
                    .await?;
                // Sans portée sur élèves, l'action d'établissement seule vaut « tout » (None) ; sinon l'ensemble.
                let tout = self
                    .a_action_etablissement(a.etablissement_id, a.user_id, a.role, action)
                    .await?;
                portees.insert(
                    action.code(),
                    if tout {
                        None
                    } else {
                        Some(p.unwrap_or_default())
                    },
                );
            }
            let autorise = match (&portees[action.code()], doc.eleve_id) {
                (None, _) => true,
                (Some(ens), Some(e)) => ens.contains(&e),
                (Some(_), None) => false,
            } && doc.sensibilite != "C4";
            sortie.push(LigneRegistre {
                document_id: doc.id,
                numero_affiche: doc.numero_affiche(),
                nature_code: doc.nature_code,
                etat: doc.etat,
                titre: if autorise {
                    doc.titre
                } else {
                    doc.titre_neutre
                },
                titre_neutre_servi: !autorise,
                sensibilite: doc.sensibilite,
                eleve_id: doc.eleve_id,
                version_courante: doc.version_courante,
                cree_le: doc.cree_le,
            });
        }
        Ok(sortie)
    }
}

/// Un objet rendu et stocké, prêt à être inscrit en version ou en exemplaire.
struct ObjetRendu {
    #[allow(dead_code)]
    rang: i32,
    cle_objet: String,
    sha256: String,
    nb_pages: i32,
    cachet_cle_ref: String,
    style: String,
    identite_version: i32,
}

/// (style, couleur principale, mentions de pied, fenêtre d'enveloppe, version).
type LigneIdentiteDocs = (String, Option<String>, Option<String>, String, i32);

async fn lire_identite(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<(IdentiteDocuments, i32), EntDbError> {
    let r: Option<LigneIdentiteDocs> = sqlx::query_as(
        "SELECT style_code, couleur_principale, mentions_pied, fenetre_enveloppe, version
           FROM ent_doc_identite WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(match r {
        Some((style_code, couleur_principale, mentions_pied, fenetre_enveloppe, version)) => (
            IdentiteDocuments {
                style_code,
                couleur_principale,
                mentions_pied,
                fenetre_enveloppe,
            },
            version,
        ),
        None => (
            IdentiteDocuments {
                style_code: "classique".into(),
                couleur_principale: None,
                mentions_pied: None,
                fenetre_enveloppe: "gauche".into(),
            },
            1,
        ),
    })
}

async fn lire_param_nature(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &str,
) -> Result<ParamNature, EntDbError> {
    let p: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT active, style_code FROM ent_doc_nature_param WHERE etablissement_id = $1 AND nature_code = $2",
    )
    .bind(etab)
    .bind(code)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let textes: Vec<(String, String)> = sqlx::query_as(
        "SELECT emplacement, texte FROM ent_doc_nature_texte WHERE etablissement_id = $1 AND nature_code = $2 ORDER BY emplacement",
    )
    .bind(etab)
    .bind(code)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    // Une nature jamais paramétrée est active (les natures intégrées sont livrées activées).
    let (active, style_code) = p.unwrap_or((true, None));
    Ok(ParamNature {
        active,
        style_code,
        textes_encadres: textes,
    })
}

async fn lire_entite(
    conn: &mut PgConnection,
    etab: Uuid,
    entite: Uuid,
) -> Result<(String, Vec<(String, String)>), EntDbError> {
    let (raison,): (String,) = sqlx::query_as(
        "SELECT raison_sociale FROM ent_doc_entite WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(entite)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let ids: Vec<(String, String)> = sqlx::query_as(
        "SELECT type, valeur FROM ent_doc_entite_identifiant WHERE etablissement_id = $1 AND entite_id = $2 ORDER BY type",
    )
    .bind(etab)
    .bind(entite)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok((raison, ids))
}

/// Nom de la personne habilitée (fiche de l'agent) pour la mention « par délégation ».
async fn nom_agent(conn: &mut PgConnection, etab: Uuid, agent: Uuid) -> Result<String, EntDbError> {
    let r: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT p.nom, p.prenom FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2",
    )
    .bind(etab)
    .bind(agent)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    let (nom, prenom) = r.ok_or_else(introuvable)?;
    Ok(match prenom {
        Some(p) if !p.trim().is_empty() => format!("{} {}", p.trim(), nom.trim()),
        _ => nom.trim().to_string(),
    })
}
