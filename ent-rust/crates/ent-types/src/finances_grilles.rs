//! Socle 24, lot F3 : catalogue d'articles qualifiés (nature légale, TVA), grilles tarifaires versionnées
//! et votées, tranches de quotient, simulateur de scénarios, familles types, état des contributions.
//! Routes `ent-api/src/finances_grilles.rs` ; l'autorité est `ent-db` (`facturation_grilles.rs`). Aucune
//! logique métier ici.
//!
//! Montants : **centimes entiers** (`i64`). Taux : **points de base** (`i32`/`i64`, 10 000 = 100 %), jamais de
//! flottant. Aucun de ces types ne porte un nom de personne : les simulations et les familles types sont
//! fictives (FAC-121, FAC-124), les publications sans donnée personnelle (FAC-129).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Catalogue d'articles (FAC-103 à FAC-106, MIG-6)
// ---------------------------------------------------------------------------

/// Natures légales fermées (§5.1, R. 442-48).
pub const NATURES_LEGALES: &[&str] = &[
    "contribution_r442_48_1",
    "contribution_r442_48_2",
    "demi_pension",
    "internat",
    "externat_surveille",
    "prestation_facultative",
    "sortie_voyage",
    "fournitures",
    "frais_dossier",
    "autre_motivee",
];
/// Régimes de TVA (§5.3).
pub const REGIMES_TVA: &[&str] = &[
    "exonere_261_4_4",
    "taxable",
    "hors_champ",
    "pour_compte_tiers",
];
/// Unités de facturation d'un article.
pub const UNITES: &[&str] = &["annee", "mois", "repas", "nuit", "seance", "unite"];

/// Proposition de qualification par règle (catégorie → nature) : **jamais appliquée seule**, elle doit être
/// validée par la comptabilité (MIG-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionQualificationDto {
    pub nature_legale: String,
    pub regime_tva: String,
    pub unite: String,
    pub mode: String,
    pub soumis_reductions: bool,
    pub soumis_remise_ordre: bool,
    /// Phrase fixe expliquant la règle appliquée.
    pub explication: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleQualifieDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub categorie: String,
    pub compte_produit: String,
    pub actif: bool,
    pub nature_legale: Option<String>,
    pub regime_tva: Option<String>,
    pub taux_tva_bps: Option<i32>,
    pub unite: Option<String>,
    pub mode: String,
    pub remboursable: bool,
    pub soumis_reductions: bool,
    pub soumis_remise_ordre: bool,
    pub modele_code: Option<String>,
    /// Qualifié (nature, TVA, unité validées par la comptabilité).
    pub qualifie: bool,
    /// Pré-rempli depuis un modèle mais pas encore validé (modèle taxable) : ne compte pas comme qualifié.
    pub a_valider: bool,
    pub qualifie_le: Option<DateTime<Utc>>,
    /// Proposition par règle, seulement pour un article non qualifié.
    pub proposition: Option<PropositionQualificationDto>,
    /// Nombre de lignes de pièces qui portent l'article (une fois qualifié, la nature ne change plus).
    pub nb_lignes: i64,
    /// Fin de tarif datée (suites 3) : premier jour où l'article n'est plus dû (les mois commencés avant le restent).
    pub retire_a_partir_du: Option<NaiveDate>,
}

/// Modèle d'article fourni par ScolIA (D-023) : copié et renommé par l'établissement (D-022).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeleArticleDto {
    pub code: String,
    pub libelle: String,
    pub categorie: String,
    pub nature_legale: String,
    pub regime_tva: String,
    pub taux_tva_bps: Option<i32>,
    pub unite: String,
    pub mode: String,
    pub remboursable: bool,
    pub soumis_reductions: bool,
    pub soumis_remise_ordre: bool,
    pub compte_produit: String,
    /// Phrase de mise en garde (ex. encaissement pour compte de tiers).
    pub remarque: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArticleDepuisModeleBody {
    pub modele_code: String,
    /// Code de l'article de l'établissement (défaut : celui du modèle).
    pub code: Option<String>,
    pub libelle: Option<String>,
    pub compte_produit: Option<String>,
}

/// Qualification d'un article existant (MIG-6) : nature, TVA, unité, mode et drapeaux.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifierArticleBody {
    pub nature_legale: String,
    pub regime_tva: String,
    pub taux_tva_bps: Option<i32>,
    pub unite: String,
    pub mode: Option<String>,
    pub remboursable: Option<bool>,
    pub soumis_reductions: Option<bool>,
    pub soumis_remise_ordre: Option<bool>,
}

/// Fin de tarif datée d'un article (suites 3, D-069) : `a_partir_du` est le premier jour où l'article n'est plus dû ;
/// `None` rétablit l'article. Les mois **commencés avant** cette date restent dus, les suivants ne le sont plus.
/// Distinct de la désactivation de l'article, qui le sort de la génération pour toute l'année.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetraitArticleBody {
    pub a_partir_du: Option<NaiveDate>,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageAnneeDto {
    pub annee_scolaire: String,
    pub nb_lignes: i64,
    pub montant_net_centimes: i64,
}

/// Usage d'un article (FAC-106) : lignes, montants nets par année.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleUsageDto {
    pub article_id: Uuid,
    pub nb_lignes: i64,
    pub montant_net_centimes: i64,
    pub par_annee: Vec<UsageAnneeDto>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReglagesGrillesBody {
    pub exiger_qualification_articles: Option<bool>,
    pub grille_second_regard: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglagesGrillesDto {
    pub exiger_qualification_articles: bool,
    pub grille_second_regard: bool,
    /// Articles actifs encore non qualifiés (à traiter avant d'exiger la qualification).
    pub articles_non_qualifies: i64,
    /// Alertes visibles de la direction : qualification non exigée (« à activer avant la rentrée 2027 »),
    /// second regard désactivé (qui, quand).
    #[serde(default)]
    pub alertes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Versions de grille (FAC-116 à FAC-129)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrilleVersionDto {
    pub id: Uuid,
    pub annee_scolaire: String,
    pub numero: i32,
    pub libelle: String,
    /// `brouillon`, `proposee`, `votee`, `en_vigueur`, `close`, `abandonnee`.
    pub statut: String,
    pub remplace_id: Option<Uuid>,
    pub motif: Option<String>,
    pub effet_retroactif: bool,
    pub propose_le: Option<DateTime<Utc>>,
    pub vote_le: Option<NaiveDate>,
    pub vote_reference: Option<String>,
    pub vote_document_id: Option<Uuid>,
    pub en_vigueur_du: Option<NaiveDate>,
    pub close_le: Option<DateTime<Utc>>,
    pub publiee_le: Option<DateTime<Utc>>,
    pub nb_tarifs: i64,
    pub nb_tranches: i64,
}

/// Création d'une version : vide, ou copie d'une version existante (FAC-116 : année N+1 en copiant N).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreerGrilleBody {
    pub annee_scolaire: String,
    pub libelle: Option<String>,
    pub copier_de: Option<Uuid>,
    /// Reprend les tarifs historiques (avant les versions) de l'année dans le brouillon (MIG-6).
    pub reprendre_historique: Option<bool>,
}

/// Une version avec ses tarifs et ses tranches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrilleDetailDto {
    pub version: GrilleVersionDto,
    pub tarifs: Vec<TarifGrilleDto>,
    pub tranches: Vec<TrancheQfDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarifGrilleDto {
    pub id: Uuid,
    pub grille_version_id: Uuid,
    pub annee_scolaire: String,
    pub article_id: Uuid,
    pub article_code: String,
    pub article_libelle: String,
    pub categorie: String,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub qf_min_centimes: Option<i64>,
    pub qf_max_centimes: Option<i64>,
    pub montant_annuel_centimes: i64,
    pub tarif_par_defaut: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TarifGrilleBody {
    pub article_id: Uuid,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub qf_min_centimes: Option<i64>,
    pub qf_max_centimes: Option<i64>,
    pub montant_annuel_centimes: i64,
    pub tarif_par_defaut: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheQfDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub qf_min_centimes: i64,
    pub qf_max_centimes: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrancheQfBody {
    pub code: String,
    pub libelle: String,
    pub qf_min_centimes: i64,
    pub qf_max_centimes: Option<i64>,
}

/// Remplace **toutes** les tranches du brouillon (FAC-118) : l'ensemble est contrôlé d'un bloc.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranchesBody {
    pub tranches: Vec<TrancheQfBody>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeHausse {
    Pourcentage,
    Montant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arrondi {
    Aucun,
    Euro,
    CinqEuros,
}

/// Hausse appliquée à un brouillon (FAC-120) : `valeur` en points de base (pourcentage) ou en centimes
/// (montant) ; négative = baisse. `apercu` : calcule sans rien écrire.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HausseBody {
    pub mode: ModeHausse,
    pub valeur: i64,
    pub arrondi: Arrondi,
    /// Un seul article ; absent : tous les articles de la grille.
    pub article_id: Option<Uuid>,
    pub apercu: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneHausseDto {
    pub tarif_id: Uuid,
    pub article_code: String,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub avant_centimes: i64,
    pub apres_centimes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuHausseDto {
    /// `true` : rien n'a été écrit.
    pub apercu: bool,
    pub lignes: Vec<LigneHausseDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnomalieGrilleDto {
    pub code: String,
    pub message: String,
    pub article_id: Option<Uuid>,
    /// Une anomalie bloquante empêche la proposition ; sinon simple avertissement.
    pub bloquante: bool,
}

/// Contrôles avant proposition (§10.2) : une grille sans anomalie bloquante peut être proposée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlesGrilleDto {
    pub anomalies: Vec<AnomalieGrilleDto>,
    pub proposable: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetourBrouillonBody {
    pub motif: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoteGrilleBody {
    /// Date de la délibération.
    pub vote_le: NaiveDate,
    pub reference: String,
    /// Pièce de la délibération (socle 7), si déjà déposée.
    pub document_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiseEnVigueurBody {
    pub du: NaiveDate,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionGrilleBody {
    pub motif: String,
    /// L'effet rétroactif n'est que **proposé** (compléments ou avoirs au lot F4) ; jamais appliqué ici.
    pub effet_retroactif: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationLigneDto {
    pub article_libelle: String,
    pub categorie: String,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub tranche: Option<String>,
    pub montant_annuel_centimes: i64,
    pub tarif_par_defaut: bool,
}

/// Grille publiable (FAC-129) : libellés et montants seulement, **aucune donnée personnelle**.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationGrilleDto {
    pub annee_scolaire: String,
    pub numero: i32,
    pub libelle: String,
    pub vote_le: Option<NaiveDate>,
    pub publiee_le: DateTime<Utc>,
    pub lignes: Vec<PublicationLigneDto>,
}

// ---------------------------------------------------------------------------
// Familles types, simulateur (FAC-121 à FAC-124)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnfantTypeBody {
    pub niveau: String,
    pub regime: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilleTypeBody {
    pub libelle: String,
    pub tranche_code: Option<String>,
    pub enfants: Vec<EnfantTypeBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnfantTypeDto {
    pub rang: i16,
    pub niveau: String,
    pub regime: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilleTypeDto {
    pub id: Uuid,
    pub libelle: String,
    pub tranche_code: Option<String>,
    pub actif: bool,
    pub enfants: Vec<EnfantTypeDto>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegleHausseBody {
    pub article_id: Option<Uuid>,
    pub mode: ModeHausse,
    pub valeur: i64,
    pub arrondi: Arrondi,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectifPrevuBody {
    pub niveau: String,
    pub regime: String,
    pub effectif: i32,
}

/// Scénario de simulation : une version de base, des règles de hausse, des effectifs prévus (à défaut, les
/// effectifs actuels par niveau et régime). Aucun nom : des nombres.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationBody {
    pub annee_scolaire: String,
    pub libelle: String,
    pub base_version_id: Uuid,
    pub regles: Vec<RegleHausseBody>,
    pub effectifs: Vec<EffectifPrevuBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationResumeDto {
    pub id: Uuid,
    pub annee_scolaire: String,
    pub libelle: String,
    pub base_version_id: Uuid,
    pub produit_base_centimes: Option<i64>,
    pub produit_simule_centimes: Option<i64>,
    pub calcule_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProduitArticleDto {
    pub article_code: String,
    pub article_libelle: String,
    pub produit_base_centimes: i64,
    pub produit_simule_centimes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffetFamilleTypeDto {
    pub famille_type_id: Uuid,
    pub libelle: String,
    pub total_base_centimes: i64,
    pub total_simule_centimes: i64,
    pub ecart_centimes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatSimulationDto {
    pub simulation_id: Uuid,
    pub annee_scolaire: String,
    pub libelle: String,
    pub base_version_id: Uuid,
    pub produit_base_centimes: i64,
    pub produit_simule_centimes: i64,
    pub ecart_centimes: i64,
    /// Effectifs réellement utilisés (prévus, ou actuels à défaut).
    pub effectifs: Vec<EffectifUtiliseDto>,
    pub par_article: Vec<ProduitArticleDto>,
    pub familles: Vec<EffetFamilleTypeDto>,
    pub calcule_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectifUtiliseDto {
    pub niveau: String,
    pub regime: String,
    pub effectif: i32,
    /// `prevu` (saisi) ou `actuel` (dénombrement des élèves actifs).
    pub origine: String,
}

/// Comparaison de deux scénarios (FAC-122) : écarts du produit et par famille type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparaisonSimulationsDto {
    pub a: ResultatSimulationDto,
    pub b: ResultatSimulationDto,
    pub ecart_produit_centimes: i64,
    pub familles: Vec<EcartFamilleDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcartFamilleDto {
    pub famille_type_id: Uuid,
    pub libelle: String,
    pub total_a_centimes: i64,
    pub total_b_centimes: i64,
    pub ecart_centimes: i64,
}

/// Export pour le conseil d'administration (FAC-123) : tableau et CSV, aucune donnée personnelle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportConseilDto {
    pub titre: String,
    pub resultat: ResultatSimulationDto,
    pub csv: String,
}

// ---------------------------------------------------------------------------
// État des contributions (FAC-196)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionNatureDto {
    /// Une des natures légales, ou `non_qualifie`.
    pub nature_legale: String,
    pub facture_centimes: i64,
    pub avoirs_centimes: i64,
    pub net_centimes: i64,
    pub nb_lignes: i64,
}

/// État agrégé des contributions par nature légale (R. 442-48), exportable pour l'inspection : aucun nom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatContributionsDto {
    pub annee_scolaire: String,
    pub lignes: Vec<ContributionNatureDto>,
    pub total_net_centimes: i64,
    pub articles_non_qualifies: i64,
    pub csv: String,
}
