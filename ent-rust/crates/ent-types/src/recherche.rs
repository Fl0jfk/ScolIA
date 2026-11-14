//! Socle 29, lot P5 : recherche globale (`docs/socles/29_PORTAILS.md` §12.1, §22.4).
//!
//! Types d'**échange**. Le résultat est un **libellé minimal** et une action ; jamais une coordonnée,
//! un identifiant national, un motif ni une donnée de santé. Le texte d'une recherche n'est jamais
//! conservé : seul un compteur journalier existe (R-PO19).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Source fédérée d'un résultat (chacune applique ses propres droits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRecherche {
    Eleves,
    Personnes,
    Personnels,
    Classes,
    Groupes,
    Salles,
}

/// Un résultat : libellé minimal (« DUPONT Léa — 4B ») et, si l'écran existe, où l'ouvrir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatRecherche {
    pub source: SourceRecherche,
    pub id: Uuid,
    pub libelle: String,
    pub lien: Option<String>,
}

/// `GET /recherche?q=…` : au plus 10 résultats par source, 40 au total, sans pagination.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponseRecherche {
    pub resultats: Vec<ResultatRecherche>,
    /// Le plafond du jour approche : la recherche est ralentie (sans aucun détail sur le plafond).
    pub ralenti: bool,
}

/// Une recherche bloquée (administrateur) : « prénom + initiale », des nombres, jamais un texte de requête.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocageRecherche {
    pub compte_id: Uuid,
    pub nom: String,
    pub bloque_jusqu_a: DateTime<Utc>,
    pub recherches_du_jour: i32,
}

/// Corps de `POST /recherche/debloquer-moi` (POR-259) : motif de la liste fermée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpsDeblocageSoi {
    /// `rentree`, `inscriptions` ou `autre`.
    pub motif: String,
}

/// Réponse unique à une recherche bloquée (429) : aucun chiffre, aucun seuil.
pub const MESSAGE_RECHERCHE_BLOQUEE: &str =
    "La recherche est suspendue temporairement. Réessayez plus tard ou contactez l'administrateur.";
pub const MESSAGE_RECHERCHE_TROP_COURTE: &str = "Saisissez au moins deux caractères.";
