//! Socle 24, lot F1 — **le service unique de facturation** (R-F1, R-F25, R-F27, §19.1).
//!
//! Il n'y a qu'une logique financière, deux entrées :
//!
//! * le chemin `/finances/*` (`finances.rs`, `finances_sepa.rs`, `finances_reactions.rs`) ;
//! * le chemin historique `/facturation/*` (`service.rs`, `fac_repo.rs`), devenu **adaptateur** : ses
//!   routes gardent leur contrat JSON (test d'or `or_facturation_historique`), mais chaque geste passe
//!   par les mêmes briques que le chemin neuf : droits fins (`ent_auth::Action::Facturation*`),
//!   journal financier, événements `facturation.*` sans montant, écritures confinées à ce service.
//!
//! **Périmètre du service (R-F1)** : les seuls fichiers qui écrivent dans `ent_fac_*`, `ent_fin_*`
//! et `ent_foyer_iban_histo` sont ceux de [`FICHIERS_DU_SERVICE`]. Le test
//! `ent-db/tests/garde_finances_ecritures.rs` lit les sources et échoue si un autre module écrit
//! dans ces tables : les autres domaines **demandent** (commandes de ce service), ils n'écrivent
//! pas. Aucune exception : le mandat SEPA signé à l'inscription passe par
//! [`crate::facturation_mandats`].
//!
//! **Journal (R-F27)** : chaque geste écrit une ligne d'`ent_fin_journal` (ajout seul, migration
//! 521) dans la transaction du geste : qui, quelle casquette, par quel chemin, quoi, sur quel
//! objet, pour quel motif. Le montant n'est écrit qu'ici (droit `facturation.journal`).
//!
//! **Événements (R-F25)** : publiés sous le nom canonique `facturation.*` ; l'ancien nom `fac.*`
//! reste lisible par alias (`ent_events::equivalents_facturation`). La charge d'un événement
//! financier ne contient **aucun montant, aucun nom, aucun IBAN** : [`payload_neutre`] retire ces
//! clés quel que soit l'appelant, c'est donc une propriété du service et non une habitude.

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::finances::db;
use crate::service::EntDbError;

/// Fichiers de `ent-db/src` autorisés à écrire dans les tables financières (garde R-F1). Toute
/// nouvelle écriture financière s'ajoute **dans un de ces fichiers** (ou ici, avec une ligne de
/// justification) : le test de garde échoue sinon.
pub const FICHIERS_DU_SERVICE: &[&str] = &[
    "fac_repo.rs",
    "facturation_comptes.rs",
    "facturation_consequences.rs",
    "facturation_mandats.rs",
    "facturation_payeurs.rs",
    "facturation_service.rs",
    "finances.rs",
    "finances_reactions.rs",
    "finances_sepa.rs",
    "iban_histo_repo.rs",
];

/// Chemin par lequel un geste est arrivé (colonne `ent_fin_journal.chemin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chemin {
    /// Routes `/finances/*`.
    Finances,
    /// Routes `/facturation/*` (adaptateurs du service).
    Historique,
    /// Automatisme : consommateur d'événement, rattrapage.
    Systeme,
    /// Vue guichet du secrétariat et de l'accueil.
    Guichet,
    /// Comptes débiteurs, payeurs, prises en charge (lot F2).
    Comptes,
    /// Espace famille.
    Famille,
}

impl Chemin {
    pub fn code(self) -> &'static str {
        match self {
            Self::Finances => "finances",
            Self::Historique => "historique",
            Self::Systeme => "systeme",
            Self::Guichet => "guichet",
            Self::Comptes => "comptes",
            Self::Famille => "famille",
        }
    }
}

/// Une ligne du journal financier, construite avec des méthodes chaînées.
#[derive(Clone, Debug)]
pub struct Acte {
    pub etab: Uuid,
    pub acteur: Option<Uuid>,
    pub casquette: String,
    pub chemin: Chemin,
    pub action: String,
    pub objet_type: String,
    pub objet_id: Option<Uuid>,
    pub foyer_id: Option<Uuid>,
    pub compte_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub montant_centimes: Option<i64>,
    pub motif: Option<String>,
    pub detail: Option<String>,
}

impl Acte {
    /// `action` : verbe pointé (`facture.emettre`) ; `objet_type` : `facture`, `mandat`, `compte`…
    pub fn new(
        etab: Uuid,
        acteur: Option<Uuid>,
        casquette: &str,
        chemin: Chemin,
        action: &str,
        objet_type: &str,
    ) -> Self {
        Self {
            etab,
            acteur,
            casquette: casquette.to_string(),
            chemin,
            action: action.to_string(),
            objet_type: objet_type.to_string(),
            objet_id: None,
            foyer_id: None,
            compte_id: None,
            eleve_id: None,
            montant_centimes: None,
            motif: None,
            detail: None,
        }
    }

    /// Geste d'un automatisme (aucun acteur humain).
    pub fn systeme(etab: Uuid, chemin: Chemin, action: &str, objet_type: &str) -> Self {
        Self::new(etab, None, "systeme", chemin, action, objet_type)
    }

    pub fn objet(mut self, id: Uuid) -> Self {
        self.objet_id = Some(id);
        self
    }
    pub fn foyer(mut self, id: Uuid) -> Self {
        self.foyer_id = Some(id);
        self
    }
    pub fn compte(mut self, id: Uuid) -> Self {
        self.compte_id = Some(id);
        self
    }
    pub fn eleve(mut self, id: Uuid) -> Self {
        self.eleve_id = Some(id);
        self
    }
    pub fn montant(mut self, centimes: i64) -> Self {
        self.montant_centimes = Some(centimes);
        self
    }
    pub fn motif(mut self, motif: &str) -> Self {
        self.motif = coupe(motif, 500);
        self
    }
    pub fn detail(mut self, detail: &str) -> Self {
        self.detail = coupe(detail, 500);
        self
    }
}

fn coupe(s: &str, max: usize) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.chars().take(max).collect())
}

/// Écrit la ligne de journal **dans la transaction du geste** (ajout seul, migration 521).
pub async fn journaliser_tx(conn: &mut PgConnection, a: &Acte) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_fin_journal (id, etablissement_id, acteur_user_id, casquette, chemin, action,
                                      objet_type, objet_id, foyer_id, compte_id, eleve_id,
                                      montant_centimes, motif, detail)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(id)
    .bind(a.etab)
    .bind(a.acteur)
    .bind(&a.casquette)
    .bind(a.chemin.code())
    .bind(&a.action)
    .bind(&a.objet_type)
    .bind(a.objet_id)
    .bind(a.foyer_id)
    .bind(a.compte_id)
    .bind(a.eleve_id)
    .bind(a.montant_centimes)
    .bind(&a.motif)
    .bind(&a.detail)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// Variante pour les dépôts qui rendent `DomainError` (`fac_repo`) : même ligne, même transaction.
pub(crate) async fn journaliser_domain_tx(
    conn: &mut PgConnection,
    a: &Acte,
) -> Result<(), domain_eleve::DomainError> {
    journaliser_tx(conn, a).await.map(|_| ()).map_err(|e| match e {
        EntDbError::Domain(d) => d,
        _ => domain_eleve::DomainError::Infrastructure,
    })
}

/// Une clé de charge d'événement qui ne doit jamais voyager (R-F25) : montants, noms, coordonnées
/// bancaires, libellés saisis, références de paiement.
pub fn cle_interdite(cle: &str) -> bool {
    let c = cle.to_ascii_lowercase();
    c.contains("centimes")
        || c.starts_with("montant")
        || c.contains("iban")
        || c == "bic"
        || c.starts_with("bic_")
        || c.ends_with("_bic")
        || c == "rum"
        || c == "nom"
        || c == "prenom"
        || c.ends_with("_nom")
        || c == "libelle"
        || c == "titulaire"
        || c == "reference"
        || c == "email"
        || c == "telephone"
        || c == "adresse"
        || c == "empreinte"
}

/// **Liste blanche** (revue M7) : seules les clés suivantes voyagent dans la charge d'un événement financier —
/// des identifiants (`id`, `*_id`, `*_ids`), des codes, des dates et des compteurs. Tout le reste est retiré
/// (restes à payer, cumuls, valeurs avant/après, empreintes…), même sous un nom qu'aucune règle ne prévoyait.
const CLES_AUTORISEES: &[&str] = &[
    "numero", "quittance_numero", "statut", "moyen", "categorie", "nature", "nb", "code", "palier", "rang",
    "source", "sens", "type", "type_piece", "format", "kind", "mode", "canal", "origine", "annee_scolaire",
    "date_effet", "date_execution", "date", "jour", "debut", "fin", "ics_change", "nom_change", "a_verifier",
    "eleve_majeur", "crediteur_modifie", "motif_rejet", "suite", "avis_demi_journee", "prevenue",
    // Conteneurs : leur contenu est filtré par la même liste.
    "lignes", "echeances", "factures", "pieces", "avoirs", "comptes", "designations", "lettrage", "ecritures",
    "propositions", "prelevements",
];

pub fn cle_autorisee(cle: &str) -> bool {
    cle == "id" || cle.ends_with("_id") || cle.ends_with("_ids") || CLES_AUTORISEES.contains(&cle)
}

/// Retire de la charge d'un événement financier tout ce que R-F25 interdit : récursif. Deux filtres :
/// la liste noire [`cle_interdite`] (montants, noms, coordonnées) **et** la liste blanche
/// [`cle_autorisee`] (identifiants, codes, dates, compteurs).
pub fn payload_neutre(v: Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.into_iter()
                .filter(|(k, _)| !cle_interdite(k) && cle_autorisee(k))
                .map(|(k, x)| (k, payload_neutre(x)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.into_iter().map(payload_neutre).collect()),
        autre => autre,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn payload_neutre_retire_montants_noms_et_coordonnees() {
        let v = payload_neutre(json!({
            "facture_id": "a", "foyer_id": "b", "numero": "F2030-000001",
            "total_centimes": 5500, "montant_du_centimes": 1, "iban_avant": "FR76…",
            "rum": "SCOLIA1", "libelle": "Cantine", "titulaire": "M. X", "nb": 3,
            "date_execution": "2030-01-01", "reference": "CHQ-1",
            "lignes": [{ "montant_centimes": 1, "ligne_id": "c", "nom": "Y", "ok": true }],
            "deja_plus_ce": 4850, "reste": 12, "iban_masque": "FR76 ••• 1234", "ancien": "x", "nouveau": "y",
        }));
        assert_eq!(
            v,
            json!({
                "facture_id": "a", "foyer_id": "b", "numero": "F2030-000001", "nb": 3,
                "date_execution": "2030-01-01",
                "lignes": [{ "ligne_id": "c" }],
            })
        );
    }

    #[test]
    fn le_service_ne_liste_que_des_fichiers_connus() {
        let mut v: Vec<_> = FICHIERS_DU_SERVICE.to_vec();
        let n = v.len();
        v.sort();
        v.dedup();
        assert_eq!(n, v.len(), "doublon dans FICHIERS_DU_SERVICE");
    }

    #[test]
    fn acte_tronque_motif_et_detail() {
        let a = Acte::systeme(Uuid::new_v4(), Chemin::Systeme, "x.y", "facture")
            .motif(&"m".repeat(900))
            .detail("   ");
        assert_eq!(a.motif.as_deref().map(str::len), Some(500));
        assert_eq!(a.detail, None);
    }
}
