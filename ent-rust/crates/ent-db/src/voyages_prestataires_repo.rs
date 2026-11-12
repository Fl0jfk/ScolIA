//! Socle 42, lot V0c : prestataires, demandes de devis, devis, signature, bon de commande, avenants et
//! envois sortants (`docs/socles/42_SORTIES_VOYAGES.md` §13 ; VOY-155 à VOY-168 ; NX-41 à NX-50).
//!
//! * **L'annuaire est la seule source de destinataires** : un envoi vise une adresse **vérifiée par un
//!   humain** du prestataire (clé étrangère composite en base) ; aucune adresse ne vient du navigateur
//!   ni d'un PDF (corrige NJ-11, VD-06, VD-17) ;
//! * **envoi aux prestataires avec droits** : `voyages.prestataires` d'établissement ou porteur de la
//!   sortie, jamais un autre profil (corrige NJ-07, VD-14) ; confirmation explicite après aperçu ;
//! * **signature** : seule la direction (`voyages.autoriser`) ; elle porte sur l'**empreinte lue** de
//!   la pièce, est en ajout seul et ne se remplace pas (corrige NJ-13) ; **commande atomique**
//!   (signature, commande, bon de commande PDF, envoi en file, étape du dossier : VD-28) ;
//! * **copie comptabilité** : boîte de l'**établissement** (réglage), jamais une valeur codée en dur
//!   (corrige NJ-12) ;
//! * **avenant** numéroté « annule et remplace » : la signature précédente reste au dossier, la
//!   commande repasse « à confirmer » (VD-07, VD-08) ;
//! * chauffeur et téléphone : champs dédiés, **visibles des seuls encadrants** (et du porteur), effacés
//!   à la clôture (corrige NJ-28) ; jamais recopiés dans un PDF adressé à un autre prestataire ;
//! * événements `voyages.*` **sans identité d'élève** ni coordonnée de famille (NJ-55).

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::MetierEvent;
use ent_mail::{MailAttachment, OutgoingMail};
use ent_object::upload::{TypeFichier, UsageFichier};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages::est_terminal;
use crate::voyages_budget::montant_en_centimes;
use crate::voyages_budget_repo::{emettre_document, DocumentVue};
use crate::voyages_pdf::rendre;
use crate::voyages_prestataires::{
    adresse_valide, blocs_commande, blocs_demande_devis, contient_reference, ligne_propre,
    message_envoi, reference_courrier, siren_valide, FaitsCommande, FaitsDemande, CATEGORIES,
};
use crate::voyages_repo::{
    charger_avec_acces, interdit, journaliser, personne_de, publier, refus, texte, texte_opt, tx, Acces,
    Fait, ParamsCommande, SortieLigne,
};

const PRESTATAIRES_MAX_PAR_ENVOI: usize = 20;
const TENTATIVES_MAX: i32 = 3;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AdresseVue {
    pub id: Uuid,
    pub email: String,
    pub domaine: String,
    pub verifiee_le: DateTime<Utc>,
    pub desactivee_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrestataireVue {
    pub id: Uuid,
    pub raison_sociale: String,
    pub siren: Option<String>,
    pub telephone: Option<String>,
    pub actif: bool,
    pub version: i32,
    pub categories: Vec<String>,
    pub adresses: Vec<AdresseVue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrestataireSaisie {
    pub raison_sociale: String,
    #[serde(default)]
    pub siren: Option<String>,
    #[serde(default)]
    pub telephone: Option<String>,
    pub categories: Vec<String>,
    /// Adresses saisies **par la personne connectée** : elles sont vérifiées par ce geste.
    #[serde(default)]
    pub adresses: Vec<String>,
    /// Version lue (modification) : refusée si l'annuaire a changé depuis.
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub actif: Option<bool>,
}

/// Ce que voit un porteur pour choisir à qui demander un devis : noms et catégories, **pas** les
/// adresses (réservées à ceux qui tiennent l'annuaire).
#[derive(Debug, Clone, Serialize)]
pub struct PrestataireChoix {
    pub id: Uuid,
    pub raison_sociale: String,
    pub categories: Vec<String>,
    pub joignable: bool,
    pub demande_envoyee: bool,
    pub devis_recu: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct EnvoiVue {
    pub id: Uuid,
    pub prestataire_id: Uuid,
    pub raison_sociale: String,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_: String,
    pub statut: String,
    pub objet: String,
    pub tentatives: i32,
    pub envoye_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DevisVue {
    pub id: Uuid,
    pub prestataire_id: Uuid,
    pub raison_sociale: String,
    pub origine: String,
    pub reference: Option<String>,
    pub montant_centimes: Option<i64>,
    /// Lecture automatique du texte : **proposée**, jamais retenue sans confirmation humaine.
    pub montant_propose_centimes: Option<i64>,
    pub valide_jusqu_au: Option<NaiveDate>,
    pub inclus: Option<String>,
    pub conditions_annulation: Option<String>,
    pub statut: String,
    pub rev: i32,
    pub document_id: Option<Uuid>,
    pub cree_le: DateTime<Utc>,
    /// Empreinte que la direction signe (calculée à la lecture).
    #[sqlx(skip)]
    pub empreinte: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CommandeVue {
    pub id: Uuid,
    pub numero: i32,
    pub devis_id: Uuid,
    pub prestataire_id: Uuid,
    pub raison_sociale: String,
    pub statut: String,
    pub montant_centimes: i64,
    pub effectif_eleves_commande: i32,
    pub effectif_adultes_commande: i32,
    pub debut_commande: DateTime<Utc>,
    pub fin_commande: DateTime<Utc>,
    pub confirmee_le: Option<DateTime<Utc>>,
    pub signe_le: DateTime<Utc>,
    pub niveau_signature: String,
    pub nb_avenants: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AchatsVue {
    pub sortie_id: Uuid,
    pub peut_gerer: bool,
    pub peut_signer: bool,
    pub devis: Vec<DevisVue>,
    pub envois: Vec<EnvoiVue>,
    pub commandes: Vec<CommandeVue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContactsTransport {
    pub commande_id: Uuid,
    pub numero: i32,
    pub raison_sociale: String,
    pub chauffeur_nom: Option<String>,
    pub chauffeur_tel: Option<String>,
    pub confirmee_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemandeSaisie {
    pub prestataires: Vec<Uuid>,
    /// Le geste est confirmé après l'aperçu de la liste des destinataires (N3).
    #[serde(default)]
    pub confirme: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PieceSaisie {
    pub nom: String,
    #[serde(skip)]
    pub contenu: Vec<u8>,
    #[serde(default)]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DevisSaisie {
    pub prestataire_id: Uuid,
    #[serde(default)]
    pub reference: Option<String>,
    /// Montant TTC au format français ; absent : le devis reste « à confirmer ».
    #[serde(default)]
    pub montant: Option<String>,
    #[serde(default)]
    pub valide_jusqu_au: Option<NaiveDate>,
    #[serde(default)]
    pub inclus: Option<String>,
    #[serde(default)]
    pub conditions_annulation: Option<String>,
    #[serde(skip)]
    pub piece: Option<PieceSaisie>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmationCommande {
    #[serde(default)]
    pub chauffeur_nom: Option<String>,
    #[serde(default)]
    pub chauffeur_tel: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AvenantSaisie {
    pub motif: String,
    pub effectif_eleves: i32,
    pub effectif_adultes: i32,
    #[serde(default)]
    pub debut: Option<DateTime<Utc>>,
    #[serde(default)]
    pub fin: Option<DateTime<Utc>>,
    #[serde(default)]
    pub confirme: bool,
}

// ───────────────────────── Droits et gardes ─────────────────────────

/// Gère les achats de la sortie : le porteur ou le droit d'établissement `voyages.prestataires`.
fn gere_achats(acces: &Acces) -> bool {
    acces.gere(Action::VoyagesPrestataires)
}

/// Lit devis, commandes et envois : ceux qui les gèrent, plus la direction et la comptabilité.
fn lit_achats(acces: &Acces) -> bool {
    gere_achats(acces)
        || acces.a(Action::VoyagesAutoriser)
        || acces.a(Action::VoyagesValider)
        || acces.a(Action::VoyagesBudgetValider)
}

fn etape_achats(etat: &str) -> bool {
    matches!(etat, "preparation" | "a_signer")
}

fn garde_gerer(acces: &Acces) -> Result<(), EntDbError> {
    if gere_achats(acces) {
        Ok(())
    } else {
        Err(interdit())
    }
}

fn exiger_confirme(confirme: bool) -> Result<(), EntDbError> {
    if confirme {
        Ok(())
    } else {
        Err(refus(
            RefusMetier::Invalide,
            "Confirmez l'envoi après avoir relu la liste des destinataires.",
        ))
    }
}

// ───────────────────────── Lectures internes ─────────────────────────

pub(crate) async fn nom_etablissement(conn: &mut PgConnection, etab: Uuid) -> Result<String, EntDbError> {
    let n: Option<String> = sqlx::query_scalar("SELECT nom FROM ent_etablissement WHERE id = $1")
        .bind(etab)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
    Ok(ligne_propre(&n.unwrap_or_else(|| "Établissement".to_string()), 120))
}

async fn reglage(conn: &mut PgConnection, etab: Uuid) -> Result<(Option<String>, Option<String>), EntDbError> {
    let r: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT boite_comptabilite_email, pied_de_page FROM ent_sortie_reglage WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.unwrap_or((None, None)))
}

async fn dates_paris(conn: &mut PgConnection, d: DateTime<Utc>) -> Result<String, EntDbError> {
    sqlx::query_scalar("SELECT to_char($1::timestamptz AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY à HH24:MI')")
        .bind(d)
        .fetch_one(conn)
        .await
        .map_err(db)
}

async fn adresse_active(conn: &mut PgConnection, etab: Uuid, prestataire: Uuid) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT id FROM ent_prestataire_adresse
          WHERE etablissement_id = $1 AND prestataire_id = $2 AND desactivee_le IS NULL
          ORDER BY verifiee_le, id LIMIT 1",
    )
    .bind(etab)
    .bind(prestataire)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

async fn journal_annuaire(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    action: &str,
    libelle: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle, actor_user_id)
         VALUES ($1,$2,NULL,'voyages_annuaire',$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(action)
    .bind(libelle)
    .bind(a.user_id)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn charger_prestataire(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<PrestataireVue, EntDbError> {
    let r: Option<(Uuid, String, Option<String>, Option<String>, bool, i32)> = sqlx::query_as(
        "SELECT id, raison_sociale, siren, telephone, actif, version FROM ent_prestataire
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (id, raison_sociale, siren, telephone, actif, version) = r.ok_or_else(introuvable)?;
    let categories: Vec<String> = sqlx::query_scalar(
        "SELECT categorie FROM ent_prestataire_categorie WHERE etablissement_id = $1 AND prestataire_id = $2 ORDER BY categorie",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let adresses = sqlx::query_as::<_, AdresseVue>(
        "SELECT id, email, domaine, verifiee_le, desactivee_le FROM ent_prestataire_adresse
          WHERE etablissement_id = $1 AND prestataire_id = $2 ORDER BY verifiee_le, id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(PrestataireVue { id, raison_sociale, siren, telephone, actif, version, categories, adresses })
}

fn valider_prestataire(s: &PrestataireSaisie) -> Result<(String, Option<String>, Option<String>, Vec<String>), EntDbError> {
    let nom = texte(&s.raison_sociale, 200)?;
    let siren = match s.siren.as_deref().map(str::trim).filter(|x| !x.is_empty()) {
        None => None,
        Some(x) if siren_valide(x) => Some(x.to_string()),
        Some(_) => {
            return Err(refus(RefusMetier::Invalide, "Le SIREN est invalide (9 chiffres, clé de contrôle)."))
        }
    };
    let tel = texte_opt(s.telephone.as_deref(), 30)?;
    if let Some(t) = &tel {
        if t.len() < 3 || !t.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '.' | '-' | '(' | ')')) {
            return Err(refus(RefusMetier::Invalide, "Le téléphone est invalide."));
        }
    }
    let mut cats: Vec<String> = Vec::new();
    for c in &s.categories {
        if !CATEGORIES.contains(&c.as_str()) {
            return Err(refus(RefusMetier::Invalide, "Catégorie de prestataire inconnue."));
        }
        if !cats.contains(c) {
            cats.push(c.clone());
        }
    }
    if cats.is_empty() {
        return Err(refus(RefusMetier::Invalide, "Indiquez au moins une catégorie (transporteur, hébergeur…)."));
    }
    Ok((nom, siren, tel, cats))
}

async fn inserer_adresse(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    prestataire: Uuid,
    brut: &str,
) -> Result<Uuid, EntDbError> {
    let (email, domaine) = adresse_valide(brut)
        .ok_or_else(|| refus(RefusMetier::Invalide, "Adresse de courrier invalide (une seule adresse, sans nom ni séparateur)."))?;
    let id = Uuid::new_v4();
    let r = sqlx::query(
        "INSERT INTO ent_prestataire_adresse (id, etablissement_id, prestataire_id, email, domaine, verifiee_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(a.etablissement_id)
    .bind(prestataire)
    .bind(&email)
    .bind(&domaine)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await;
    if let Err(sqlx::Error::Database(e)) = &r {
        if e.is_unique_violation() {
            return Err(refus(RefusMetier::Conflit, "Cette adresse appartient déjà à un prestataire de l'annuaire."));
        }
    }
    r.map_err(db)?;
    Ok(id)
}

// ───────────────────────── Annuaire (VOY-155, NX-41) ─────────────────────────

impl PgEnt {
    /// L'annuaire complet, adresses comprises : réservé au droit d'établissement `voyages.prestataires`
    /// (secrétariat, comptabilité, direction). Le porteur passe par [`Self::voyage_prestataires_choix`].
    pub async fn prestataires_liste(&self, a: &Acteur<'_>) -> Result<Vec<PrestataireVue>, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_prestataire WHERE etablissement_id = $1 ORDER BY lower(raison_sociale), id LIMIT 500",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut v = Vec::new();
        for id in ids {
            v.push(charger_prestataire(&mut t, a.etablissement_id, id).await?);
        }
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Crée un prestataire et ses adresses : l'adresse n'entre que par un **humain** habilité.
    pub async fn prestataire_creer(&self, a: &Acteur<'_>, s: PrestataireSaisie) -> Result<PrestataireVue, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let (nom, siren, tel, cats) = valider_prestataire(&s)?;
        let id = Uuid::new_v4();
        let mut t = tx(self, a).await?;
        let r = sqlx::query(
            "INSERT INTO ent_prestataire (id, etablissement_id, raison_sociale, siren, telephone, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(&nom)
        .bind(&siren)
        .bind(&tel)
        .bind(a.user_id)
        .execute(&mut *t)
        .await;
        if let Err(sqlx::Error::Database(e)) = &r {
            if e.is_unique_violation() {
                return Err(refus(RefusMetier::Conflit, "Un prestataire de ce nom existe déjà."));
            }
        }
        r.map_err(db)?;
        for c in &cats {
            sqlx::query(
                "INSERT INTO ent_prestataire_categorie (id, etablissement_id, prestataire_id, categorie) VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(id)
            .bind(c)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        for adr in &s.adresses {
            inserer_adresse(&mut t, a, id, adr).await?;
        }
        journal_annuaire(&mut t, a, "prestataire_cree", &format!("Prestataire créé : {nom}")).await?;
        let vue = charger_prestataire(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Modifie un prestataire (nom, SIREN, téléphone, catégories, activité) sur la version lue.
    pub async fn prestataire_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        s: PrestataireSaisie,
    ) -> Result<PrestataireVue, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let (nom, siren, tel, cats) = valider_prestataire(&s)?;
        let version = s.version.ok_or_else(invalide)?;
        let mut t = tx(self, a).await?;
        let r = sqlx::query(
            "UPDATE ent_prestataire SET raison_sociale = $4, siren = $5, telephone = $6,
                    actif = coalesce($7, actif), version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(version)
        .bind(&nom)
        .bind(&siren)
        .bind(&tel)
        .bind(s.actif)
        .execute(&mut *t)
        .await;
        if let Err(sqlx::Error::Database(e)) = &r {
            if e.is_unique_violation() {
                return Err(refus(RefusMetier::Conflit, "Un prestataire de ce nom existe déjà."));
            }
        }
        if r.map_err(db)?.rows_affected() == 0 {
            // Introuvable ou modifié entre-temps.
            charger_prestataire(&mut t, a.etablissement_id, id).await?;
            return Err(refus(RefusMetier::Conflit, "L'annuaire a changé entre-temps : rechargez la fiche."));
        }
        sqlx::query("DELETE FROM ent_prestataire_categorie WHERE etablissement_id = $1 AND prestataire_id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        for c in &cats {
            sqlx::query(
                "INSERT INTO ent_prestataire_categorie (id, etablissement_id, prestataire_id, categorie) VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(id)
            .bind(c)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        journal_annuaire(&mut t, a, "prestataire_modifie", &format!("Prestataire modifié : {nom}")).await?;
        let vue = charger_prestataire(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Ajoute une adresse vérifiée par la personne connectée.
    pub async fn prestataire_adresse_ajouter(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        email: &str,
    ) -> Result<PrestataireVue, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        charger_prestataire(&mut t, a.etablissement_id, id).await?;
        inserer_adresse(&mut t, a, id, email).await?;
        journal_annuaire(&mut t, a, "prestataire_adresse_ajoutee", "Adresse de prestataire ajoutée").await?;
        let vue = charger_prestataire(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Désactive une adresse (jamais supprimée : les envois passés y renvoient).
    pub async fn prestataire_adresse_desactiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        adresse: Uuid,
    ) -> Result<PrestataireVue, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let n = sqlx::query(
            "UPDATE ent_prestataire_adresse SET desactivee_le = now()
              WHERE etablissement_id = $1 AND prestataire_id = $2 AND id = $3 AND desactivee_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(adresse)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journal_annuaire(&mut t, a, "prestataire_adresse_desactivee", "Adresse de prestataire désactivée").await?;
        let vue = charger_prestataire(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Réglage de l'établissement pour les sorties : boîte comptabilité (copie des commandes). Direction
    /// et comptabilité ; l'adresse est saisie par la personne connectée, jamais fournie par une commande.
    pub async fn sorties_reglage_ecrire(
        &self,
        a: &Acteur<'_>,
        boite_comptabilite: Option<&str>,
        pied_de_page: Option<&str>,
    ) -> Result<(), EntDbError> {
        let d = self.voyages_droits(a).await?;
        if !(d.contains(&Action::VoyagesAutoriser) || d.contains(&Action::VoyagesBudgetValider)) {
            return Err(interdit());
        }
        let boite = match boite_comptabilite.map(str::trim).filter(|x| !x.is_empty()) {
            None => None,
            Some(b) => Some(
                adresse_valide(b)
                    .ok_or_else(|| refus(RefusMetier::Invalide, "Adresse de la boîte comptabilité invalide."))?
                    .0,
            ),
        };
        let pied = texte_opt(pied_de_page, 500)?;
        let mut t = tx(self, a).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_reglage (id, etablissement_id, boite_comptabilite_email, pied_de_page, modifie_par_user_id)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET boite_comptabilite_email = EXCLUDED.boite_comptabilite_email,
                    pied_de_page = EXCLUDED.pied_de_page, modifie_le = now(),
                    modifie_par_user_id = EXCLUDED.modifie_par_user_id",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(&boite)
        .bind(&pied)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journal_annuaire(&mut t, a, "reglage_sorties_modifie", "Réglage des sorties modifié (boîte comptabilité, pied de page)")
            .await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Prestataires proposés au porteur : sans adresses.
    pub async fn voyage_prestataires_choix(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<PrestataireChoix>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        garde_gerer(&acces)?;
        let lignes: Vec<(Uuid, String, bool, bool, bool)> = sqlx::query_as(
            "SELECT p.id, p.raison_sociale,
                    EXISTS (SELECT 1 FROM ent_prestataire_adresse ad WHERE ad.etablissement_id = p.etablissement_id
                             AND ad.prestataire_id = p.id AND ad.desactivee_le IS NULL),
                    EXISTS (SELECT 1 FROM ent_sortie_envoi e WHERE e.etablissement_id = p.etablissement_id
                             AND e.sortie_id = $2 AND e.prestataire_id = p.id AND e.type = 'demande_devis'
                             AND e.statut <> 'annule'),
                    EXISTS (SELECT 1 FROM ent_sortie_devis d WHERE d.etablissement_id = p.etablissement_id
                             AND d.sortie_id = $2 AND d.prestataire_id = p.id)
               FROM ent_prestataire p WHERE p.etablissement_id = $1 AND p.actif
              ORDER BY lower(p.raison_sociale), p.id LIMIT 500",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (pid, nom, joignable, demande, recu) in lignes {
            let categories: Vec<String> = sqlx::query_scalar(
                "SELECT categorie FROM ent_prestataire_categorie WHERE etablissement_id = $1 AND prestataire_id = $2 ORDER BY categorie",
            )
            .bind(a.etablissement_id)
            .bind(pid)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            out.push(PrestataireChoix {
                id: pid,
                raison_sociale: nom,
                categories,
                joignable,
                demande_envoyee: demande,
                devis_recu: recu,
            });
        }
        t.commit().await.map_err(db)?;
        Ok(out)
    }
}

// ───────────────────────── Envois : mise en file et traitement ─────────────────────────

struct NouvelEnvoi<'a> {
    prestataire: Uuid,
    adresse: Uuid,
    type_: &'a str,
    reference: &'a str,
    objet: String,
    corps: String,
    document: Option<Uuid>,
    commande: Option<Uuid>,
    copie: Option<&'a str>,
}

async fn mettre_en_file(conn: &mut PgConnection, a: &Acteur<'_>, sortie: Uuid, e: NouvelEnvoi<'_>) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_envoi
             (id, etablissement_id, sortie_id, prestataire_id, adresse_id, type, reference_courrier, objet, corps,
              document_id, commande_id, copie_comptabilite_email, cree_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(id)
    .bind(a.etablissement_id)
    .bind(sortie)
    .bind(e.prestataire)
    .bind(e.adresse)
    .bind(e.type_)
    .bind(e.reference)
    .bind(&e.objet)
    .bind(&e.corps)
    .bind(e.document)
    .bind(e.commande)
    .bind(e.copie)
    .bind(a.user_id)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(id)
}

struct EnvoiAPartir {
    id: Uuid,
    adresse: Option<String>,
    objet: String,
    corps: String,
    commande_id: Option<Uuid>,
    type_: String,
    copie: Option<String>,
    document: Option<(String, Vec<u8>)>,
    tentatives: i32,
}

impl PgEnt {
    /// Envoie les messages en file de l'établissement (demandes, relances, bons de commande,
    /// avenants). Verrou de ligne (`SKIP LOCKED`) : deux traitements simultanés ne partent jamais deux
    /// fois. Un échec n'est jamais fatal pour le geste métier : le message reste en file jusqu'à
    /// [`TENTATIVES_MAX`] essais, puis passe en `echec`. Retourne le nombre de messages partis.
    pub async fn voyages_envois_traiter(&self, etab: Uuid, limite: i64) -> Result<usize, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, Option<Uuid>, String, String, String, Option<Uuid>, Option<String>, i32)> =
            sqlx::query_as(
                "SELECT e.id, e.adresse_id, e.commande_id, e.type, e.objet, e.corps, e.document_id,
                        e.copie_comptabilite_email, e.tentatives
                   FROM ent_sortie_envoi e
                  WHERE e.etablissement_id = $1 AND e.statut = 'a_envoyer'
                  ORDER BY e.cree_le, e.id LIMIT $2 FOR UPDATE SKIP LOCKED",
            )
            .bind(etab)
            .bind(limite.clamp(1, 100))
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
        let mut a_partir = Vec::new();
        for (id, adresse_id, commande_id, type_, objet, corps, document_id, copie, tentatives) in lignes {
            let adresse: Option<String> = sqlx::query_scalar(
                "SELECT email FROM ent_prestataire_adresse WHERE etablissement_id = $1 AND id = $2 AND desactivee_le IS NULL",
            )
            .bind(etab)
            .bind(adresse_id)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            let document = match document_id {
                None => None,
                Some(d) => {
                    let r: Option<(String, String, String)> = sqlx::query_as(
                        "SELECT cle_objet, nom, sha256 FROM ent_sortie_document WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(d)
                    .fetch_optional(&mut *t)
                    .await
                    .map_err(db)?;
                    match r {
                        Some((cle, nom, sha)) => {
                            let o = self.lire_objet_empreinte(etab, &cle, Some(&sha)).await.ok();
                            o.map(|b| (nom, b))
                        }
                        None => None,
                    }
                }
            };
            a_partir.push(EnvoiAPartir { id, adresse, objet, corps, commande_id, type_, copie, document, tentatives });
        }
        let mut partis = 0;
        for e in a_partir {
            let (statut, erreur): (&str, Option<&str>) = match (&e.adresse, &e.document) {
                (None, _) => ("echec", Some("adresse_desactivee")),
                (Some(_), None) => ("echec", Some("piece_illisible")),
                (Some(adr), Some((nom, octets))) => {
                    let mut ok = false;
                    if self.mailer().is_ready() {
                        let piece = MailAttachment {
                            filename: nom.clone(),
                            mime: "application/pdf".into(),
                            bytes: octets.clone(),
                        };
                        let r = self
                            .mailer()
                            .send(OutgoingMail {
                                to: adr.clone(),
                                subject: e.objet.clone(),
                                body_text: e.corps.clone(),
                                attachments: vec![piece.clone()],
                            })
                            .await;
                        ok = matches!(&r, Ok(rc) if self.envoi_reussi(rc));
                        if ok {
                            if let Some(copie) = &e.copie {
                                // Copie à la boîte comptabilité de l'établissement : sans effet sur le statut.
                                let _ = self
                                    .mailer()
                                    .send(OutgoingMail {
                                        to: copie.clone(),
                                        subject: e.objet.clone(),
                                        body_text: e.corps.clone(),
                                        attachments: vec![piece],
                                    })
                                    .await;
                            }
                        }
                    }
                    if ok {
                        ("envoye", None)
                    } else if e.tentatives + 1 >= TENTATIVES_MAX {
                        ("echec", Some("envoi_non_abouti"))
                    } else {
                        ("a_envoyer", Some("envoi_non_abouti"))
                    }
                }
            };
            sqlx::query(
                "UPDATE ent_sortie_envoi SET statut = $3, tentatives = tentatives + 1, derniere_erreur = $4,
                        envoye_le = CASE WHEN $3 = 'envoye' THEN now() ELSE NULL END, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(e.id)
            .bind(statut)
            .bind(erreur)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            if statut == "envoye" {
                partis += 1;
                if let (Some(c), "bon_commande") = (e.commande_id, e.type_.as_str()) {
                    sqlx::query(
                        "UPDATE ent_sortie_commande SET statut = 'envoyee', modifie_le = now()
                          WHERE etablissement_id = $1 AND id = $2 AND statut = 'a_envoyer'",
                    )
                    .bind(etab)
                    .bind(c)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                }
            }
        }
        t.commit().await.map_err(db)?;
        Ok(partis)
    }

    /// Tente l'envoi sans jamais faire échouer le geste qui vient d'être validé.
    async fn envois_apres_commit(&self, etab: Uuid) {
        if let Err(e) = self.voyages_envois_traiter(etab, 20).await {
            tracing::warn!(erreur = %e, "voyages : traitement des envois reporté");
        }
    }
}

// ───────────────────────── Demandes de devis (VOY-156, VOY-158) ─────────────────────────

impl PgEnt {
    /// NX-42 : demande de devis aux prestataires **choisis**, avec un PDF rendu côté serveur à partir
    /// des faits du dossier (effectifs, dates, point de rendez-vous, consignes de l'organisateur ; ni
    /// nom d'élève ni coordonnées de familles). N3 : confirmation explicite après aperçu.
    pub async fn voyage_devis_demander(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        s: DemandeSaisie,
    ) -> Result<Vec<EnvoiVue>, EntDbError> {
        exiger_confirme(s.confirme)?;
        let mut choix = s.prestataires.clone();
        choix.sort();
        choix.dedup();
        if choix.is_empty() || choix.len() > PRESTATAIRES_MAX_PAR_ENVOI {
            return Err(refus(RefusMetier::Invalide, "Choisissez de un à vingt prestataires."));
        }
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if !etape_achats(&so.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Les demandes de devis se font pendant la préparation du dossier.",
            ));
        }
        let etab_nom = nom_etablissement(&mut t, a.etablissement_id).await?;
        let (_, pied) = reglage(&mut t, a.etablissement_id).await?;
        let reference = reference_courrier(&so.reference, a.etablissement_id, so.id);
        let debut = dates_paris(&mut t, so.debut).await?;
        let fin = dates_paris(&mut t, so.fin).await?;
        let destination = so.destination_libelle.clone().unwrap_or_else(|| so.destination.clone());
        let blocs = blocs_demande_devis(&FaitsDemande {
            etablissement: &etab_nom,
            reference_courrier: &reference,
            titre: &ligne_propre(&so.titre, 200),
            destination: &ligne_propre(&destination, 300),
            debut,
            fin,
            point_rendez_vous: so.point_rendez_vous.as_deref(),
            effectif_eleves: so.effectif_eleves_declare,
            effectif_adultes: so.effectif_adultes_declare,
            car_sur_place: so.car_sur_place,
            consignes: so.consignes_transport.as_deref(),
            pied_de_page: pied.as_deref(),
        });
        // Chaque destinataire : prestataire actif de l'établissement, avec une adresse vérifiée, qui
        // n'a pas déjà reçu la demande (pour relancer : `voyage_devis_relancer`).
        let mut cibles: Vec<(Uuid, Uuid)> = Vec::new();
        for p in &choix {
            let actif: Option<bool> = sqlx::query_scalar(
                "SELECT actif FROM ent_prestataire WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(p)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            if actif != Some(true) {
                return Err(refus(RefusMetier::Invalide, "Prestataire inconnu ou inactif."));
            }
            let adr = adresse_active(&mut t, a.etablissement_id, *p).await?.ok_or_else(|| {
                refus(RefusMetier::Invalide, "Un prestataire choisi n'a aucune adresse vérifiée dans l'annuaire.")
            })?;
            let deja: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_envoi WHERE etablissement_id = $1 AND sortie_id = $2
                                 AND prestataire_id = $3 AND type = 'demande_devis' AND statut <> 'annule')",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(p)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if deja {
                return Err(refus(
                    RefusMetier::Conflit,
                    "La demande a déjà été envoyée à l'un de ces prestataires : utilisez la relance.",
                ));
            }
            cibles.push((*p, adr));
        }
        let titre_pdf = format!("ScolIA — demande de devis — {}", so.reference);
        let pdf = rendre(&titre_pdf, &blocs);
        let (doc, ev) =
            emettre_document(self, &mut t, a, &so, "demande_devis", None, &format!("demande-devis-{}.pdf", so.reference), &pdf)
                .await?;
        let (objet, corps) = message_envoi("demande_devis", &etab_nom, &reference, None);
        let mut ids = Vec::new();
        for (p, adr) in &cibles {
            ids.push(
                mettre_en_file(
                    &mut t,
                    a,
                    id,
                    NouvelEnvoi {
                        prestataire: *p,
                        adresse: *adr,
                        type_: "demande_devis",
                        reference: &reference,
                        objet: objet.clone(),
                        corps: corps.clone(),
                        document: Some(doc.id),
                        commande: None,
                        copie: None,
                    },
                )
                .await?,
            );
        }
        journaliser(
            &mut t,
            a,
            id,
            Fait { apres: Some(format!("{} prestataire(s)", cibles.len())), ..Fait::simple("devis_demandes") },
        )
        .await?;
        let ev2 = publier(&mut t, a, "voyages.devis_demandes", id, json!({ "nb": cibles.len() })).await?;
        let vues = envois_de(&mut t, a.etablissement_id, id, Some(&ids)).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev, ev2]);
        self.envois_apres_commit(a.etablissement_id).await;
        // Relire après l'envoi : le statut a pu passer à « envoye ».
        let mut t = tx(self, a).await?;
        let v = envois_de(&mut t, a.etablissement_id, id, Some(&ids)).await.unwrap_or(vues);
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// NX-43 : relance les prestataires qui ont reçu la demande et n'ont **pas** répondu (aucun devis).
    pub async fn voyage_devis_relancer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        confirme: bool,
    ) -> Result<Vec<EnvoiVue>, EntDbError> {
        exiger_confirme(confirme)?;
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if !etape_achats(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Les relances se font pendant la préparation du dossier."));
        }
        let etab_nom = nom_etablissement(&mut t, a.etablissement_id).await?;
        let reference = reference_courrier(&so.reference, a.etablissement_id, so.id);
        let a_relancer: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT e.prestataire_id, e.document_id FROM ent_sortie_envoi e
              WHERE e.etablissement_id = $1 AND e.sortie_id = $2 AND e.type = 'demande_devis' AND e.statut = 'envoye'
                AND NOT EXISTS (SELECT 1 FROM ent_sortie_devis d WHERE d.etablissement_id = e.etablissement_id
                                 AND d.sortie_id = e.sortie_id AND d.prestataire_id = e.prestataire_id)
                AND NOT EXISTS (SELECT 1 FROM ent_sortie_envoi r WHERE r.etablissement_id = e.etablissement_id
                                 AND r.sortie_id = e.sortie_id AND r.prestataire_id = e.prestataire_id
                                 AND r.type = 'relance' AND r.statut = 'a_envoyer')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        if a_relancer.is_empty() {
            return Err(refus(RefusMetier::Conflit, "Aucun prestataire à relancer : tous ont répondu ou la demande n'est pas partie."));
        }
        let (objet, corps) = message_envoi("relance", &etab_nom, &reference, None);
        let mut ids = Vec::new();
        for (p, doc) in &a_relancer {
            let Some(adr) = adresse_active(&mut t, a.etablissement_id, *p).await? else { continue };
            ids.push(
                mettre_en_file(
                    &mut t,
                    a,
                    id,
                    NouvelEnvoi {
                        prestataire: *p,
                        adresse: adr,
                        type_: "relance",
                        reference: &reference,
                        objet: objet.clone(),
                        corps: corps.clone(),
                        document: *doc,
                        commande: None,
                        copie: None,
                    },
                )
                .await?,
            );
        }
        if ids.is_empty() {
            return Err(refus(RefusMetier::Conflit, "Aucun prestataire à relancer n'a d'adresse active."));
        }
        journaliser(&mut t, a, id, Fait { apres: Some(format!("{} relance(s)", ids.len())), ..Fait::simple("devis_relances") })
            .await?;
        let ev = publier(&mut t, a, "voyages.devis_relances", id, json!({ "nb": ids.len() })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        self.envois_apres_commit(a.etablissement_id).await;
        let mut t = tx(self, a).await?;
        let v = envois_de(&mut t, a.etablissement_id, id, Some(&ids)).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }
}

async fn envois_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    seuls: Option<&[Uuid]>,
) -> Result<Vec<EnvoiVue>, EntDbError> {
    sqlx::query_as::<_, EnvoiVue>(
        "SELECT e.id, e.prestataire_id, p.raison_sociale, e.type, e.statut, e.objet, e.tentatives, e.envoye_le, e.cree_le
           FROM ent_sortie_envoi e
           JOIN ent_prestataire p ON p.etablissement_id = e.etablissement_id AND p.id = e.prestataire_id
          WHERE e.etablissement_id = $1 AND e.sortie_id = $2 AND ($3::uuid[] IS NULL OR e.id = ANY($3))
          ORDER BY e.cree_le DESC, e.id",
    )
    .bind(etab)
    .bind(sortie)
    .bind(seuls)
    .fetch_all(conn)
    .await
    .map_err(db)
}

// ───────────────────────── Devis (VOY-161, VOY-165) ─────────────────────────

const COLS_DEVIS: &str = "d.id, d.prestataire_id, p.raison_sociale, d.origine, d.reference, d.montant_centimes,
     d.montant_propose_centimes, d.valide_jusqu_au, d.inclus, d.conditions_annulation, d.statut, d.rev,
     d.document_id, d.cree_le";

async fn devis_de(
    e: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    seul: Option<Uuid>,
) -> Result<Vec<DevisVue>, EntDbError> {
    let mut v = sqlx::query_as::<_, DevisVue>(&format!(
        "SELECT {COLS_DEVIS} FROM ent_sortie_devis d
           JOIN ent_prestataire p ON p.etablissement_id = d.etablissement_id AND p.id = d.prestataire_id
          WHERE d.etablissement_id = $1 AND d.sortie_id = $2 AND ($3::uuid IS NULL OR d.id = $3)
          ORDER BY d.cree_le, d.id"
    ))
    .bind(etab)
    .bind(sortie)
    .bind(seul)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for d in &mut v {
        d.empreinte = Some(empreinte_devis(e, conn, etab, d).await?);
    }
    Ok(v)
}

/// Empreinte de ce que la direction signe : celle de la **pièce** du devis (relue et vérifiée dans le
/// magasin) ; sans pièce, celle des faits saisis (prestataire, référence, montant confirmé, validité).
async fn empreinte_devis(e: &PgEnt, conn: &mut PgConnection, etab: Uuid, d: &DevisVue) -> Result<String, EntDbError> {
    if let Some(doc) = d.document_id {
        let r: Option<(String, String)> = sqlx::query_as(
            "SELECT cle_objet, sha256 FROM ent_sortie_document WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(doc)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let (cle, sha) = r.ok_or_else(introuvable)?;
        e.lire_objet_empreinte(etab, &cle, Some(&sha)).await?;
        return Ok(sha);
    }
    let canon = format!(
        "devis|{}|{}|{}|{}|{}",
        d.id,
        d.prestataire_id,
        d.reference.as_deref().unwrap_or(""),
        d.montant_centimes.map_or(String::new(), |m| m.to_string()),
        d.valide_jusqu_au.map_or(String::new(), |x| x.to_string()),
    );
    Ok(ent_object::sha256_hex(canon.as_bytes()))
}

/// Vérifie une pièce de devis (PDF uniquement, même politique que les autres dépôts : plafond, type
/// détecté sur les octets, analyse antivirus) et la dépose comme document de la sortie.
async fn deposer_piece_devis(
    e: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    so: &SortieLigne,
    p: &PieceSaisie,
) -> Result<(DocumentVue, MetierEvent), EntDbError> {
    let v = e
        .politique_fichiers()
        .valider(UsageFichier::Document, &p.nom, p.mime.as_deref(), &p.contenu)
        .map_err(EntDbError::Fichier)?;
    if v.type_fichier != TypeFichier::Pdf {
        return Err(refus(RefusMetier::Invalide, "Le devis se dépose en PDF."));
    }
    emettre_document(e, conn, a, so, "devis", None, &ligne_propre(&v.nom, 120), &p.contenu).await
}

impl PgEnt {
    /// NX-72 : saisit un devis reçu hors courrier (téléphone, papier numérisé). Avec un montant saisi,
    /// c'est la personne connectée qui le confirme ; sans montant, le devis reste « à confirmer ».
    pub async fn voyage_devis_ajouter(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        s: DevisSaisie,
    ) -> Result<DevisVue, EntDbError> {
        let reference = texte_opt(s.reference.as_deref(), 80)?;
        let inclus = texte_opt(s.inclus.as_deref(), 2000)?;
        let conditions = texte_opt(s.conditions_annulation.as_deref(), 2000)?;
        let montant = match s.montant.as_deref().map(str::trim).filter(|x| !x.is_empty()) {
            None => None,
            Some(m) => Some(montant_en_centimes(m).map_err(|e| refus(RefusMetier::Invalide, e))?),
        };
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if est_terminal(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Le dossier est clos."));
        }
        let commande_existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_commande WHERE etablissement_id = $1 AND sortie_id = $2 AND statut <> 'annulee')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !etape_achats(&so.etat) && !commande_existe {
            return Err(refus(RefusMetier::Conflit, "Les devis se saisissent pendant la préparation du dossier."));
        }
        let ok_presta: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_prestataire WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(s.prestataire_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !ok_presta {
            return Err(refus(RefusMetier::Invalide, "Prestataire inconnu."));
        }
        let mut events = Vec::new();
        let doc = match &s.piece {
            Some(p) => {
                let (d, ev) = deposer_piece_devis(self, &mut t, a, &so, p).await?;
                events.push(ev);
                Some(d.id)
            }
            None => None,
        };
        let statut = if commande_existe {
            "apres_commande"
        } else if montant.is_some() {
            "recu"
        } else {
            "a_confirmer"
        };
        let devis = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_devis
                 (id, etablissement_id, sortie_id, prestataire_id, origine, document_id, reference, montant_centimes,
                  montant_confirme_le, montant_confirme_par_user_id, valide_jusqu_au, inclus, conditions_annulation,
                  statut, cree_par_user_id)
             VALUES ($1,$2,$3,$4,'manuel',$5,$6,$7, CASE WHEN $7::bigint IS NULL THEN NULL ELSE now() END,
                     CASE WHEN $7::bigint IS NULL THEN NULL ELSE $8::uuid END, $9,$10,$11,$12,$8)",
        )
        .bind(devis)
        .bind(a.etablissement_id)
        .bind(id)
        .bind(s.prestataire_id)
        .bind(doc)
        .bind(&reference)
        .bind(montant)
        .bind(a.user_id)
        .bind(s.valide_jusqu_au)
        .bind(&inclus)
        .bind(&conditions)
        .bind(statut)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, id, Fait { apres: Some(statut.to_string()), ..Fait::simple("devis_recu") }).await?;
        events.push(publier(&mut t, a, "voyages.devis_recu", id, json!({ "origine": "manuel" })).await?);
        let v = devis_de(self, &mut t, a.etablissement_id, id, Some(devis)).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        v.into_iter().next().ok_or_else(introuvable)
    }

    /// Confirme le montant d'un devis (VOY-162) : un humain saisit le montant TTC, qu'il ait été
    /// proposé par la lecture du texte ou non. Le montant retenu n'est jamais celui de la lecture.
    pub async fn voyage_devis_confirmer_montant(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        devis: Uuid,
        rev: i32,
        montant: &str,
    ) -> Result<DevisVue, EntDbError> {
        let centimes = montant_en_centimes(montant).map_err(|e| refus(RefusMetier::Invalide, e))?;
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        let r: Option<(String, i32)> = sqlx::query_as(
            "SELECT statut, rev FROM ent_sortie_devis WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(devis)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (statut, courant) = r.ok_or_else(introuvable)?;
        if courant != rev {
            return Err(refus(RefusMetier::Conflit, "Le devis a changé entre-temps : rechargez-le."));
        }
        if !matches!(statut.as_str(), "a_confirmer" | "recu") || (!etape_achats(&so.etat)) {
            return Err(refus(RefusMetier::Conflit, "Le montant de ce devis ne peut plus être modifié."));
        }
        sqlx::query(
            "UPDATE ent_sortie_devis SET montant_centimes = $3, montant_confirme_le = now(),
                    montant_confirme_par_user_id = $4, statut = 'recu', rev = rev + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(devis)
        .bind(centimes)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, id, Fait { apres: Some(format!("{centimes} centimes")), ..Fait::simple("devis_montant_confirme") })
            .await?;
        let ev = publier(&mut t, a, "voyages.devis_montant_confirme", id, json!({})).await?;
        let v = devis_de(self, &mut t, a.etablissement_id, id, Some(devis)).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        v.into_iter().next().ok_or_else(introuvable)
    }

    /// NX-48 (VOY-165) : choisit le devis retenu (organisateur, secrétariat, comptabilité). Un seul
    /// devis est retenu à la fois ; le changement de choix est permis tant que rien n'est signé.
    pub async fn voyage_devis_choisir(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        devis: Uuid,
        rev: i32,
    ) -> Result<Vec<DevisVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if !etape_achats(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Le choix d'un devis se fait pendant la préparation du dossier."));
        }
        let r: Option<(String, i32, Option<i64>)> = sqlx::query_as(
            "SELECT statut, rev, montant_centimes FROM ent_sortie_devis
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(devis)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (statut, courant, montant) = r.ok_or_else(introuvable)?;
        if courant != rev {
            return Err(refus(RefusMetier::Conflit, "Le devis a changé entre-temps : rechargez-le."));
        }
        if montant.is_none() || !matches!(statut.as_str(), "recu" | "choisi") {
            return Err(refus(
                RefusMetier::Conflit,
                "Seul un devis dont le montant est confirmé peut être choisi.",
            ));
        }
        let signe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_devis WHERE etablissement_id = $1 AND sortie_id = $2 AND statut = 'signe')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if signe {
            return Err(refus(RefusMetier::Conflit, "Un devis est déjà signé : passez par un avenant."));
        }
        sqlx::query(
            "UPDATE ent_sortie_devis SET statut = 'recu', rev = rev + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND statut = 'choisi' AND id <> $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(devis)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_sortie_devis SET statut = 'choisi', rev = rev + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'choisi'",
        )
        .bind(a.etablissement_id)
        .bind(devis)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, id, Fait::simple("devis_choisi")).await?;
        let ev = publier(&mut t, a, "voyages.devis_choisi", id, json!({})).await?;
        let v = devis_de(self, &mut t, a.etablissement_id, id, None).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// Écarte un devis (jamais supprimé : le dossier garde ce qui a été reçu).
    pub async fn voyage_devis_ecarter(&self, a: &Acteur<'_>, id: Uuid, devis: Uuid) -> Result<Vec<DevisVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if !etape_achats(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Les devis ne se trient que pendant la préparation du dossier."));
        }
        let n = sqlx::query(
            "UPDATE ent_sortie_devis SET statut = 'ecarte', rev = rev + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 AND statut IN ('a_confirmer','recu','choisi')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(devis)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit());
        }
        journaliser(&mut t, a, id, Fait::simple("devis_ecarte")).await?;
        let ev = publier(&mut t, a, "voyages.devis_ecarte", id, json!({})).await?;
        let v = devis_de(self, &mut t, a.etablissement_id, id, None).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// Lit une pièce de devis (PDF) : droit contrôlé, empreinte vérifiée, jamais d'URL publique.
    pub async fn voyage_devis_piece_lire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        devis: Uuid,
    ) -> Result<(String, Vec<u8>), EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !lit_achats(&acces) {
            return Err(introuvable());
        }
        let r: Option<(String, String, String)> = sqlx::query_as(
            "SELECT doc.cle_objet, doc.nom, doc.sha256 FROM ent_sortie_devis d
               JOIN ent_sortie_document doc ON doc.etablissement_id = d.etablissement_id AND doc.id = d.document_id
              WHERE d.etablissement_id = $1 AND d.sortie_id = $2 AND d.id = $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(devis)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        let (cle, nom, sha) = r.ok_or_else(introuvable)?;
        let o = self.lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha)).await?;
        Ok((nom, o))
    }
}

// ───────────────────────── Signature et commande atomique (VOY-166, VOY-167) ─────────────────────────

/// Appelée par la commande `signer-devis` de l'automate, dans **sa** transaction : le droit
/// `voyages.autoriser`, l'étape `a_signer` et la version du dossier sont déjà contrôlés. Retourne les
/// événements à diffuser après validation.
pub(crate) async fn signer_devis(
    e: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    so: &SortieLigne,
    p: &ParamsCommande,
) -> Result<Vec<MetierEvent>, EntDbError> {
    let devis = p.devis_id.ok_or_else(|| refus(RefusMetier::Invalide, "Indiquez le devis à signer."))?;
    let rev = p.devis_rev.ok_or_else(|| refus(RefusMetier::Invalide, "Une signature porte sur le devis que vous avez lu : rechargez-le."))?;
    let lue = p
        .empreinte
        .as_deref()
        .map(|x| x.trim().to_lowercase())
        .filter(|x| x.len() == 64)
        .ok_or_else(|| refus(RefusMetier::Invalide, "Une signature porte sur l'empreinte du devis que vous avez lue."))?;
    if p.confirme != Some(true) {
        return Err(refus(RefusMetier::Invalide, "Confirmez que vous acceptez ce devis."));
    }
    let courant = sqlx::query_as::<_, DevisVue>(&format!(
        "SELECT {COLS_DEVIS} FROM ent_sortie_devis d
           JOIN ent_prestataire p ON p.etablissement_id = d.etablissement_id AND p.id = d.prestataire_id
          WHERE d.etablissement_id = $1 AND d.sortie_id = $2 AND d.id = $3 FOR UPDATE OF d"
    ))
    .bind(a.etablissement_id)
    .bind(so.id)
    .bind(devis)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    if courant.rev != rev || courant.statut != "choisi" {
        return Err(refus(
            RefusMetier::Conflit,
            "Le devis a changé ou n'est plus le devis choisi : rechargez-le avant de signer.",
        ));
    }
    let montant = courant
        .montant_centimes
        .ok_or_else(|| refus(RefusMetier::Conflit, "Le montant du devis n'est pas confirmé."))?;
    // La pièce signée est celle qui est au magasin à cet instant : l'empreinte lue doit être identique.
    let reelle = empreinte_devis(e, conn, a.etablissement_id, &courant).await?;
    if reelle != lue {
        return Err(refus(
            RefusMetier::Conflit,
            "La pièce du devis ne correspond plus à ce que vous avez lu : rechargez-la avant de signer.",
        ));
    }
    let adresse = adresse_active(conn, a.etablissement_id, courant.prestataire_id).await?.ok_or_else(|| {
        refus(RefusMetier::Conflit, "Ce prestataire n'a plus d'adresse vérifiée : ajoutez-en une dans l'annuaire.")
    })?;
    let signataire = personne_de(conn, a).await?;
    let intention = format!(
        "J'accepte le devis {} de {} d'un montant de {},{:02} € TTC pour la sortie {}.",
        courant.reference.as_deref().unwrap_or("sans référence"),
        ligne_propre(&courant.raison_sociale, 120),
        montant / 100,
        montant % 100,
        so.reference
    );
    let signature = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_signature
             (id, etablissement_id, sortie_id, devis_id, empreinte, niveau, signataire_user_id, signataire_personne_id, intention)
         VALUES ($1,$2,$3,$4,$5,'S1',$6,$7,$8)",
    )
    .bind(signature)
    .bind(a.etablissement_id)
    .bind(so.id)
    .bind(devis)
    .bind(&reelle)
    .bind(a.user_id)
    .bind(signataire)
    .bind(&intention)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_sortie_devis SET statut = 'signe', rev = rev + 1, modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(devis)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Effectifs commandés : ceux déclarés au dossier, à défaut les participants et encadrants inscrits.
    let (nb_eleves, nb_adultes): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2
                  AND etat IN ('inscrit','pret','bloque')),
                (SELECT count(*) FROM ent_sortie_encadrant WHERE etablissement_id = $1 AND sortie_id = $2
                  AND role IN ('responsable','encadrant'))",
    )
    .bind(a.etablissement_id)
    .bind(so.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let eleves = so.effectif_eleves_declare.unwrap_or(nb_eleves as i32);
    let adultes = so.effectif_adultes_declare.unwrap_or(nb_adultes as i32);
    let numero: i32 = sqlx::query_scalar(
        "SELECT coalesce(max(numero), 0)::int + 1 FROM ent_sortie_commande WHERE etablissement_id = $1 AND sortie_id = $2",
    )
    .bind(a.etablissement_id)
    .bind(so.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let commande = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_commande
             (id, etablissement_id, sortie_id, devis_id, signature_id, prestataire_id, adresse_id, numero, montant_centimes,
              effectif_eleves_commande, effectif_adultes_commande, debut_commande, fin_commande, cree_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(commande)
    .bind(a.etablissement_id)
    .bind(so.id)
    .bind(devis)
    .bind(signature)
    .bind(courant.prestataire_id)
    .bind(adresse)
    .bind(numero)
    .bind(montant)
    .bind(eleves)
    .bind(adultes)
    .bind(so.debut)
    .bind(so.fin)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Bon de commande : PDF rendu côté serveur, puis mis en file vers l'adresse de l'annuaire.
    let etab_nom = nom_etablissement(conn, a.etablissement_id).await?;
    let (copie, pied) = reglage(conn, a.etablissement_id).await?;
    let reference = reference_courrier(&so.reference, a.etablissement_id, so.id);
    let debut = dates_paris(conn, so.debut).await?;
    let fin = dates_paris(conn, so.fin).await?;
    let signe_le = dates_paris(conn, Utc::now()).await?;
    let blocs = blocs_commande(&FaitsCommande {
        etablissement: &etab_nom,
        reference_courrier: &reference,
        numero,
        avenant: None,
        motif_avenant: None,
        prestataire: &ligne_propre(&courant.raison_sociale, 120),
        reference_devis: courant.reference.as_deref(),
        montant_centimes: montant,
        effectif_eleves: eleves,
        effectif_adultes: adultes,
        debut,
        fin,
        empreinte_devis: &reelle,
        signe_le,
        pied_de_page: pied.as_deref(),
    });
    let pdf = rendre(&format!("ScolIA — bon de commande {} — {}", numero, so.reference), &blocs);
    let (doc, ev_doc) = emettre_document(
        e,
        conn,
        a,
        so,
        "bon_commande",
        None,
        &format!("bon-de-commande-{}-{}.pdf", so.reference, numero),
        &pdf,
    )
    .await?;
    let (objet, corps) = message_envoi("bon_commande", &etab_nom, &reference, Some(numero));
    mettre_en_file(
        conn,
        a,
        so.id,
        NouvelEnvoi {
            prestataire: courant.prestataire_id,
            adresse,
            type_: "bon_commande",
            reference: &reference,
            objet,
            corps,
            document: Some(doc.id),
            commande: Some(commande),
            copie: copie.as_deref(),
        },
    )
    .await?;
    journaliser(
        conn,
        a,
        so.id,
        Fait {
            champ: Some("devis"),
            apres: Some(format!("signé (S1), empreinte {}", &reelle[..12])),
            ..Fait::simple("devis_signe")
        },
    )
    .await?;
    let ev = publier(conn, a, "voyages.devis_signe", so.id, json!({ "niveau": "S1" })).await?;
    let ev2 = publier(conn, a, "voyages.commande_creee", so.id, json!({ "numero": numero })).await?;
    Ok(vec![ev_doc, ev, ev2])
}

impl PgEnt {
    /// Envoi du bon de commande (et des messages en file) après la validation d'une signature.
    pub(crate) async fn voyages_envois_apres_signature(&self, etab: Uuid) {
        self.envois_apres_commit(etab).await;
    }
}

// ───────────────────────── Commande : confirmation, chauffeur, avenant ─────────────────────────

fn tel_valide(t: &str) -> bool {
    t.len() >= 3
        && t.len() <= 30
        && t.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '.' | '-' | '(' | ')'))
}

impl PgEnt {
    /// NX-45, §13.3 : enregistre la confirmation du prestataire et, dans des **champs dédiés**, le
    /// nom et le téléphone du conducteur. Visibles des seuls encadrants (et du porteur), effacés à la
    /// clôture ; jamais écrits dans un PDF.
    pub async fn voyage_commande_confirmer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        commande: Uuid,
        s: ConfirmationCommande,
    ) -> Result<CommandeVue, EntDbError> {
        let nom = texte_opt(s.chauffeur_nom.as_deref(), 120)?;
        let tel = texte_opt(s.chauffeur_tel.as_deref(), 30)?;
        if tel.as_deref().is_some_and(|t| !tel_valide(t)) {
            return Err(refus(RefusMetier::Invalide, "Numéro de téléphone invalide."));
        }
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if est_terminal(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Le dossier est clos."));
        }
        let n = sqlx::query(
            "UPDATE ent_sortie_commande SET statut = 'confirmee', confirmee_le = now(),
                    chauffeur_nom = coalesce($4, chauffeur_nom), chauffeur_tel = coalesce($5, chauffeur_tel),
                    modifie_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3
                AND statut IN ('envoyee','a_confirmer','confirmee')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(commande)
        .bind(&nom)
        .bind(&tel)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(refus(RefusMetier::Conflit, "Cette commande n'est pas encore partie chez le prestataire, ou elle est annulée."));
        }
        journaliser(&mut t, a, id, Fait::simple("commande_confirmee")).await?;
        let ev = publier(&mut t, a, "voyages.commande_confirmee", id, json!({})).await?;
        let v = commandes_de(&mut t, a.etablissement_id, id, Some(commande)).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        v.into_iter().next().ok_or_else(introuvable)
    }

    /// Contacts du conducteur : encadrants de la sortie et porteur seulement.
    pub async fn voyage_contacts_transport(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<ContactsTransport>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.porteur || acces.encadrant) {
            return Err(introuvable());
        }
        let v = sqlx::query_as::<_, (Uuid, i32, String, Option<String>, Option<String>, Option<DateTime<Utc>>)>(
            "SELECT c.id, c.numero, p.raison_sociale, c.chauffeur_nom, c.chauffeur_tel, c.confirmee_le
               FROM ent_sortie_commande c
               JOIN ent_prestataire p ON p.etablissement_id = c.etablissement_id AND p.id = c.prestataire_id
              WHERE c.etablissement_id = $1 AND c.sortie_id = $2 AND c.statut <> 'annulee' ORDER BY c.numero",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v
            .into_iter()
            .map(|(commande_id, numero, raison_sociale, chauffeur_nom, chauffeur_tel, confirmee_le)| ContactsTransport {
                commande_id,
                numero,
                raison_sociale,
                chauffeur_nom,
                chauffeur_tel,
                confirmee_le,
            })
            .collect())
    }

    /// NX-44 (VOY-157) : avenant numéroté « annule et remplace » au **prestataire de la commande**
    /// (adresse de l'annuaire). La signature précédente reste au dossier ; la commande repasse à
    /// « à confirmer » (corrige VD-07 et VD-08).
    pub async fn voyage_avenant_emettre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        commande: Uuid,
        s: AvenantSaisie,
    ) -> Result<CommandeVue, EntDbError> {
        exiger_confirme(s.confirme)?;
        let motif = texte(&s.motif, 1000)?;
        if s.effectif_eleves < 0 || s.effectif_adultes < 0 || s.effectif_eleves > 2000 || s.effectif_adultes > 500 {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gerer(&acces)?;
        if est_terminal(&so.etat) || matches!(so.etat.as_str(), "en_cours" | "rentree") {
            return Err(refus(RefusMetier::Conflit, "Le dossier n'est plus modifiable : pas d'avenant."));
        }
        let c: Option<(Uuid, Uuid, String, i64, DateTime<Utc>, DateTime<Utc>, Uuid, Option<String>)> = sqlx::query_as(
            "SELECT c.prestataire_id, c.signature_id, c.statut, c.montant_centimes, c.debut_commande, c.fin_commande,
                    c.devis_id, d.reference
               FROM ent_sortie_commande c
               JOIN ent_sortie_devis d ON d.etablissement_id = c.etablissement_id AND d.id = c.devis_id
              WHERE c.etablissement_id = $1 AND c.sortie_id = $2 AND c.id = $3 FOR UPDATE OF c",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(commande)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (prestataire, signature, statut, montant, d0, f0, _devis, ref_devis) = c.ok_or_else(introuvable)?;
        if statut == "annulee" {
            return Err(refus(RefusMetier::Conflit, "Cette commande est annulée."));
        }
        let debut = s.debut.unwrap_or(d0);
        let fin = s.fin.unwrap_or(f0);
        if fin <= debut {
            return Err(refus(RefusMetier::Invalide, "La fin doit suivre le début."));
        }
        let adresse = adresse_active(&mut t, a.etablissement_id, prestataire).await?.ok_or_else(|| {
            refus(RefusMetier::Conflit, "Le prestataire n'a plus d'adresse vérifiée : ajoutez-en une dans l'annuaire.")
        })?;
        let numero_cmd: i32 = sqlx::query_scalar("SELECT numero FROM ent_sortie_commande WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(commande)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        let n: i32 = sqlx::query_scalar(
            "SELECT coalesce(max(numero), 0)::int + 1 FROM ent_sortie_avenant WHERE etablissement_id = $1 AND commande_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(commande)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let etab_nom = nom_etablissement(&mut t, a.etablissement_id).await?;
        let (copie, pied) = reglage(&mut t, a.etablissement_id).await?;
        let reference = reference_courrier(&so.reference, a.etablissement_id, so.id);
        let presta_nom: String =
            sqlx::query_scalar("SELECT raison_sociale FROM ent_prestataire WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(prestataire)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let emp: String = sqlx::query_scalar("SELECT empreinte FROM ent_sortie_signature WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(signature)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        let signe: DateTime<Utc> =
            sqlx::query_scalar("SELECT signe_le FROM ent_sortie_signature WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(signature)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let (dtxt, ftxt, stxt) = (
            dates_paris(&mut t, debut).await?,
            dates_paris(&mut t, fin).await?,
            dates_paris(&mut t, signe).await?,
        );
        let blocs = blocs_commande(&FaitsCommande {
            etablissement: &etab_nom,
            reference_courrier: &reference,
            numero: numero_cmd,
            avenant: Some(n),
            motif_avenant: Some(&motif),
            prestataire: &ligne_propre(&presta_nom, 120),
            reference_devis: ref_devis.as_deref(),
            montant_centimes: montant,
            effectif_eleves: s.effectif_eleves,
            effectif_adultes: s.effectif_adultes,
            debut: dtxt,
            fin: ftxt,
            empreinte_devis: &emp,
            signe_le: stxt,
            pied_de_page: pied.as_deref(),
        });
        let pdf = rendre(&format!("ScolIA — avenant {n} — {}", so.reference), &blocs);
        let (doc, ev_doc) = emettre_document(
            self,
            &mut t,
            a,
            &so,
            "avenant",
            None,
            &format!("avenant-{}-{}-{}.pdf", so.reference, numero_cmd, n),
            &pdf,
        )
        .await?;
        let (objet, corps) = message_envoi("avenant", &etab_nom, &reference, Some(n));
        let envoi = mettre_en_file(
            &mut t,
            a,
            id,
            NouvelEnvoi {
                prestataire,
                adresse,
                type_: "avenant",
                reference: &reference,
                objet,
                corps,
                document: Some(doc.id),
                commande: Some(commande),
                copie: copie.as_deref(),
            },
        )
        .await?;
        sqlx::query(
            "INSERT INTO ent_sortie_avenant
                 (id, etablissement_id, sortie_id, commande_id, numero, motif, effectif_eleves_commande,
                  effectif_adultes_commande, debut_commande, fin_commande, document_id, envoi_id, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(id)
        .bind(commande)
        .bind(n)
        .bind(&motif)
        .bind(s.effectif_eleves)
        .bind(s.effectif_adultes)
        .bind(debut)
        .bind(fin)
        .bind(doc.id)
        .bind(envoi)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_sortie_commande SET statut = 'a_confirmer', effectif_eleves_commande = $3,
                    effectif_adultes_commande = $4, debut_commande = $5, fin_commande = $6, adresse_id = $7,
                    confirmee_le = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(commande)
        .bind(s.effectif_eleves)
        .bind(s.effectif_adultes)
        .bind(debut)
        .bind(fin)
        .bind(adresse)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            id,
            Fait { apres: Some(format!("avenant n° {n}")), motif: Some(&motif), ..Fait::simple("avenant_emis") },
        )
        .await?;
        let ev = publier(&mut t, a, "voyages.avenant", id, json!({ "numero": n })).await?;
        let v = commandes_de(&mut t, a.etablissement_id, id, Some(commande)).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev_doc, ev]);
        self.envois_apres_commit(a.etablissement_id).await;
        v.into_iter().next().ok_or_else(introuvable)
    }
}

async fn commandes_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    seule: Option<Uuid>,
) -> Result<Vec<CommandeVue>, EntDbError> {
    sqlx::query_as::<_, CommandeVue>(
        "SELECT c.id, c.numero, c.devis_id, c.prestataire_id, p.raison_sociale, c.statut, c.montant_centimes,
                c.effectif_eleves_commande, c.effectif_adultes_commande, c.debut_commande, c.fin_commande,
                c.confirmee_le, s.signe_le, s.niveau AS niveau_signature,
                (SELECT count(*) FROM ent_sortie_avenant av WHERE av.etablissement_id = c.etablissement_id
                  AND av.commande_id = c.id) AS nb_avenants
           FROM ent_sortie_commande c
           JOIN ent_prestataire p ON p.etablissement_id = c.etablissement_id AND p.id = c.prestataire_id
           JOIN ent_sortie_signature s ON s.etablissement_id = c.etablissement_id AND s.id = c.signature_id
          WHERE c.etablissement_id = $1 AND c.sortie_id = $2 AND ($3::uuid IS NULL OR c.id = $3)
          ORDER BY c.numero",
    )
    .bind(etab)
    .bind(sortie)
    .bind(seule)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// À la clôture, au refus ou à l'annulation du dossier : le conducteur et son téléphone sont effacés.
pub(crate) async fn effacer_contacts_chauffeur(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_sortie_commande SET chauffeur_nom = NULL, chauffeur_tel = NULL, contacts_effaces_le = now()
          WHERE etablissement_id = $1 AND sortie_id = $2 AND (chauffeur_nom IS NOT NULL OR chauffeur_tel IS NOT NULL)",
    )
    .bind(etab)
    .bind(sortie)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

// ───────────────────────── Lecture d'ensemble ─────────────────────────

impl PgEnt {
    /// Devis, envois et commandes d'une sortie, pour ceux qui les gèrent, la direction et la
    /// comptabilité. Tout autre profil : « introuvable ».
    pub async fn voyage_achats(&self, a: &Acteur<'_>, id: Uuid) -> Result<AchatsVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !lit_achats(&acces) {
            return Err(introuvable());
        }
        let devis = devis_de(self, &mut t, a.etablissement_id, id, None).await?;
        let envois = envois_de(&mut t, a.etablissement_id, id, None).await?;
        let commandes = commandes_de(&mut t, a.etablissement_id, id, None).await?;
        t.commit().await.map_err(db)?;
        Ok(AchatsVue {
            sortie_id: id,
            peut_gerer: gere_achats(&acces),
            peut_signer: acces.a(Action::VoyagesAutoriser),
            devis,
            envois,
            commandes,
        })
    }

    /// Relit le bon de commande ou l'avenant le plus récent d'une commande (PDF), droit contrôlé.
    pub async fn voyage_envois_file(&self, a: &Acteur<'_>) -> Result<usize, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        self.voyages_envois_traiter(a.etablissement_id, 50).await
    }
}

/// La référence de correspondance de la sortie (pour l'écran et le courrier entrant).
pub(crate) fn reference_pour(so: &SortieLigne, etab: Uuid) -> String {
    reference_courrier(&so.reference, etab, so.id)
}

pub(crate) fn texte_contient_reference(texte: &str, so: &SortieLigne, etab: Uuid) -> bool {
    contient_reference(texte, &reference_pour(so, etab))
}
