//! **Guichet des demandes de droits** (socle 27, lot G3a) : dépôt, qualification, identité et qualité,
//! horloge légale, décisions par droit, clôture. Réf. : `27_RGPD_ARCHIVES.md` §9.1 à §9.4, §12.1, §13.1,
//! §14.3 (RGP-140 à RGP-169) et ajout personas §26 (RGP-302 à RGP-307, RGP-316).
//!
//! Ce que ce lot fait, et ce qu'il ne fait pas :
//!
//! * **fait** : les quatre entrées (dépôt connecté, dépôt sans compte, saisie d'un courrier ou d'un
//!   passage au guichet, transmission par le DPO ou la CNIL), le numéro par établissement **sans trou
//!   visible**, l'accusé de réception automatique, la vérification d'identité et de qualité **à la date de
//!   réception**, l'horloge légale d'un mois et sa prolongation motivée, les alertes avant échéance, les
//!   décisions droit par droit, la clôture avec destruction de la pièce d'identité (R-RG16), la
//!   réouverture, le tableau des demandes ;
//! * **pas fait ici** (lot G3b, migration 2545) : l'extrait assemblé, les masquages de tiers et la remise
//!   chiffrée. L'action `droits.remettre` existe déjà au catalogue, aucune fonction de ce module ne la
//!   consomme. Les états `a_relire`, `prete` et `remise` de `rgp_demande` sont réservés à ce lot.
//!
//! Règles appliquées : R-RG15 (rien ne part sans relecture : aucune remise ici), **R-RG16** (pièce détruite
//! à la clôture), **R-RG17** (refus pour demande excessive = décision motivée de la direction, jamais
//! automatique), **R-RG19** (une demande ouverte gèle les sorts finaux : le gel réel est posé par le moteur
//! du lot G6, qui lit [`PgEnt::demandes_ouvertes_pour_sujet`]), **R-RG32** (les écrans ne montrent que des
//! compteurs et la demande qui justifie l'accès), **R-RG37** (l'établissement qui détient répond),
//! **R-RG40** (toute transition écrit le journal dans la même transaction), **R-RG42** (une demande d'un
//! inconnu ne confirme jamais la scolarisation), **E14** (aucune donnée métier dans un journal ou un avis).
//!
//! Cloisonnement : tout passe par [`TenantTx`] (RLS posée) et par `exiger_action_etab` ; aucune fonction ne
//! reçoit d'établissement venant de la requête HTTP (il vient de la session).

use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use serde::Serialize;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// Délai légal de réponse, en mois (RGPD art. 12.3).
pub const DELAI_REPONSE_MOIS: u32 = 1;
/// Prolongation maximale, en mois (RGPD art. 12.3 : « deux mois supplémentaires »).
pub const PROLONGATION_MAX_MOIS: u32 = 2;
/// Jours avant l'échéance auxquels les personnes habilitées sont alertées (S-544).
pub const ALERTE_ECHEANCE_JOURS: [i64; 2] = [10, 3];
/// Écart toléré entre la réception et la saisie d'une demande, en jours (RGP-303) : au-delà, le guichet
/// l'affiche comme un retard de relève du courrier (CL27-64).
pub const ECART_SAISIE_ALERTE_JOURS: i64 = 5;
/// La vérification d'identité **ne suspend pas** le délai légal (§9.2 : lecture retenue, la plus protectrice
/// pour la personne ; marquée [H, à confirmer DPO] dans la conception). L'attente est comptée et affichée
/// ([`DemandeVue::identite_attente_jours`]) pour qu'un DPO qui tranche l'inverse dispose du fait ; aucune
/// échéance n'est repoussée tant que ce choix n'est pas validé (valeur par défaut prudente, D-069).
pub const IDENTITE_SUSPEND_DELAI: bool = false;

/// Natures de notification déclarées au socle 8 pour ce lot (§18.1).
pub const KIND_ACCUSE: &str = "rgpd.demande_accusee";
pub const KIND_PROLONGEE: &str = "rgpd.demande_prolongee";
pub const KIND_ECHEANCE_PROCHE: &str = "rgpd.demande_echeance_proche";

/// Droits qu'une demande peut porter (§17.4).
pub const DROITS: [&str; 7] = [
    "acces",
    "rectification",
    "effacement",
    "limitation",
    "opposition",
    "portabilite",
    "consultations",
];

/// Qualités invoquables par un demandeur (§9.3).
pub const QUALITES: [&str; 9] = [
    "soi_meme",
    "eleve_mineur",
    "titulaire_autorite",
    "parent_sans_autorite",
    "representant_majeur_protege",
    "mandataire",
    "ancien_eleve",
    "personnel",
    "tiers",
];

const CANAUX_GUICHET: [&str; 5] = ["courrier", "courriel", "guichet", "dpo", "cnil"];

/// Message **invariable** rendu à un dépôt sans compte : il ne dit jamais si la personne est connue de
/// l'établissement (R-RG42, CL27-66). Aucun accusé n'est envoyé à l'adresse fournie par la demande ; le
/// guichet reprend contact par un moyen déjà au dossier, ou en main propre.
pub const MESSAGE_SANS_COMPTE: &str = "Votre demande est enregistrée. Si l'établissement détient des \
données vous concernant, il vous répondra par un moyen de contact déjà connu de son dossier, ou vous \
recevrez une réponse au guichet. Le délai légal d'un mois court à compter de la réception.";

fn texte(v: &str, min: usize, max: usize) -> Option<String> {
    let v = v.trim();
    let n = v.chars().count();
    (n >= min && n <= max && !v.chars().any(|c| c.is_control() && c != '\n')).then(|| v.to_string())
}

/// Échéance légale : un mois après la réception, au jour de l'établissement (art. 12.3). Un 31 janvier
/// donne le 28 ou 29 février (`Months` sature sur le dernier jour du mois).
pub fn echeance_legale(recue_le: NaiveDate) -> NaiveDate {
    recue_le
        .checked_add_months(Months::new(DELAI_REPONSE_MOIS))
        .unwrap_or(recue_le)
}

/// Borne haute d'une prolongation : deux mois après l'échéance légale (art. 12.3).
pub fn prolongation_max(echeance: NaiveDate) -> NaiveDate {
    echeance
        .checked_add_months(Months::new(PROLONGATION_MAX_MOIS))
        .unwrap_or(echeance)
}

// ---------------------------------------------------------------------------------------------
// Vues
// ---------------------------------------------------------------------------------------------

/// Un droit demandé et sa décision.
#[derive(Clone, Debug, Serialize)]
pub struct DroitVue {
    pub droit: String,
    pub decision: String,
    pub motif_code: Option<String>,
    pub motif_texte: Option<String>,
    pub decide_le: Option<DateTime<Utc>>,
}

/// Une vérification d'identité (sans la pièce : seule la mention reste après la clôture, R-RG16).
#[derive(Clone, Debug, Serialize)]
pub struct IdentiteVue {
    pub mode: String,
    pub resultat: String,
    pub qualite_constatee: Option<String>,
    pub qualite_note: Option<String>,
    pub verifie_le: Option<DateTime<Utc>>,
    /// Une pièce est encore détenue (elle doit disparaître à la clôture).
    pub piece_detenue: bool,
    pub piece_detruite_le: Option<DateTime<Utc>>,
}

/// Une ligne du journal des transitions (jamais de donnée métier, E14).
#[derive(Clone, Debug, Serialize)]
pub struct JournalVue {
    pub evenement: String,
    pub detail: Option<String>,
    pub par_role: Option<String>,
    pub le: DateTime<Utc>,
}

/// La demande, vue du guichet.
#[derive(Clone, Debug, Serialize)]
pub struct DemandeVue {
    pub id: Uuid,
    /// Numéro lisible par l'usager : « 2027-014 ».
    pub reference: String,
    pub canal: String,
    pub recue_le: NaiveDate,
    pub saisie_le: DateTime<Utc>,
    /// RGP-303 : l'écart entre réception et saisie dépasse [`ECART_SAISIE_ALERTE_JOURS`].
    pub saisie_tardive: bool,
    pub echeance_le: NaiveDate,
    pub prolongee_jusqu_au: Option<NaiveDate>,
    pub prolongation_motif: Option<String>,
    /// Échéance qui s'applique réellement (prolongation comprise).
    pub echeance_effective: NaiveDate,
    /// Jours restants au jour de l'établissement (négatif : en retard).
    pub jours_restants: i64,
    pub en_retard: bool,
    pub qualite: String,
    pub demandeur_nom: Option<String>,
    pub demandeur_a_un_compte: bool,
    pub sujet_type: String,
    pub sujet_id: Option<Uuid>,
    pub sujet_nom_saisi: Option<String>,
    pub qualification: String,
    pub qualification_motif: Option<String>,
    pub perimetre_texte: Option<String>,
    pub perimetre_du: Option<NaiveDate>,
    pub perimetre_au: Option<NaiveDate>,
    pub etat: String,
    pub gestionnaire_personne_id: Option<Uuid>,
    pub identite_demandee_le: Option<NaiveDate>,
    pub identite_fournie_le: Option<NaiveDate>,
    /// Jours d'attente d'une précision d'identité (comptés, non déduits de l'échéance : voir
    /// [`IDENTITE_SUSPEND_DELAI`]).
    pub identite_attente_jours: i64,
    pub reouvertures: i32,
    pub close_le: Option<DateTime<Utc>>,
    pub reponse_resume: Option<String>,
    pub refus_motif: Option<String>,
    pub titulaires_informes_le: Option<DateTime<Utc>>,
    pub titulaires_non_informes_motif: Option<String>,
    pub droits: Vec<DroitVue>,
    pub identites: Vec<IdentiteVue>,
    pub journal: Vec<JournalVue>,
    /// Une pièce d'identité est encore détenue : la clôture la détruira (R-RG16).
    pub piece_a_detruire: bool,
}

/// Ligne du tableau des demandes (**métadonnées seulement**, R-RG32 : aucun nom d'élève, aucun contenu).
#[derive(Clone, Debug, Serialize)]
pub struct DemandeLigne {
    pub id: Uuid,
    pub reference: String,
    pub canal: String,
    pub recue_le: NaiveDate,
    pub echeance_effective: NaiveDate,
    pub jours_restants: i64,
    pub en_retard: bool,
    pub etat: String,
    pub qualification: String,
    pub qualite: String,
    pub sujet_type: String,
    pub gestionnaire_personne_id: Option<Uuid>,
    pub droits: Vec<String>,
    pub saisie_tardive: bool,
}

/// Le tableau des demandes et ses compteurs (RGP-169).
#[derive(Clone, Debug, Serialize)]
pub struct TableauDemandes {
    pub jour: NaiveDate,
    pub lignes: Vec<DemandeLigne>,
    pub ouvertes: usize,
    pub en_retard: usize,
    pub echeance_proche: usize,
    pub identite_a_verifier: usize,
    pub a_qualifier: usize,
    pub closes: usize,
}

/// Suivi d'une demande, côté demandeur (RGP-155) : états lisibles, aucune information interne.
#[derive(Clone, Debug, Serialize)]
pub struct SuiviDemande {
    pub id: Uuid,
    pub reference: String,
    pub recue_le: NaiveDate,
    pub echeance_effective: NaiveDate,
    pub etat: String,
    /// Phrase prête à afficher (« reçue le 3, échéance le 3 du mois suivant, en cours d'instruction »).
    pub avancement: String,
    pub droits: Vec<String>,
    pub prolongee_jusqu_au: Option<NaiveDate>,
    pub prolongation_motif: Option<String>,
    pub close_le: Option<DateTime<Utc>>,
    pub reponse_resume: Option<String>,
}

/// Saisie d'une demande au guichet (courrier, courriel, passage, DPO, CNIL).
#[derive(Clone, Debug)]
pub struct SaisieDemande<'a> {
    pub canal: &'a str,
    /// Date de réception = date du courrier reçu (RGP-143) ; jamais dans le futur.
    pub recue_le: NaiveDate,
    pub demandeur_compte_id: Option<Uuid>,
    pub demandeur_personne_id: Option<Uuid>,
    pub demandeur_nom_saisi: Option<&'a str>,
    pub demandeur_contact: Option<&'a str>,
    pub qualite: &'a str,
    pub sujet_type: &'a str,
    pub sujet_id: Option<Uuid>,
    pub sujet_nom_saisi: Option<&'a str>,
    pub droits: &'a [&'a str],
    pub perimetre_texte: Option<&'a str>,
    pub perimetre_du: Option<NaiveDate>,
    pub perimetre_au: Option<NaiveDate>,
}

/// Dépôt par un demandeur **connecté** (RGP-140).
#[derive(Clone, Debug)]
pub struct DepotConnecte<'a> {
    pub qualite: &'a str,
    pub sujet_type: &'a str,
    pub sujet_id: Option<Uuid>,
    pub droits: &'a [&'a str],
    pub perimetre_texte: Option<&'a str>,
    pub perimetre_du: Option<NaiveDate>,
    pub perimetre_au: Option<NaiveDate>,
}

/// Dépôt **sans compte** (RGP-142) : ancien élève, parent sans compte. Aucune donnée n'est confirmée au
/// déposant (R-RG42) ; la vérification d'identité est faite ensuite par le guichet (§9.2, RGP-307).
#[derive(Clone, Debug)]
pub struct DepotSansCompte<'a> {
    pub nom_saisi: &'a str,
    pub contact: &'a str,
    pub qualite: &'a str,
    pub sujet_nom_saisi: &'a str,
    pub droits: &'a [&'a str],
    pub perimetre_texte: Option<&'a str>,
}

/// Une étape de la grille de vérification d'identité d'un demandeur sans compte (RGP-307).
#[derive(Clone, Copy, Debug, Serialize)]
pub struct EtapeVerification {
    pub code: &'static str,
    pub libelle: &'static str,
    /// Ce que le guichet fait, en une phrase.
    pub geste: &'static str,
    /// L'étape est obligatoire avant de répondre.
    pub obligatoire: bool,
}

/// Grille de vérification proportionnée (§9.2, RGP-307). Elle ne fait aucune promesse d'exhaustivité :
/// c'est l'ordre des gestes, pas une pièce à constituer. Rien n'y est conservé (R-RG16).
pub const GRILLE_VERIFICATION: [EtapeVerification; 6] = [
    EtapeVerification {
        code: "rapprochement",
        libelle: "Rapprocher la demande d'un dossier",
        geste: "Chercher la personne sur son nom et sa date de naissance ; en cas de doute, ne rien confirmer au demandeur.",
        obligatoire: true,
    },
    EtapeVerification {
        code: "canal_connu",
        libelle: "Reprendre contact par un moyen déjà au dossier",
        geste: "Écrire ou téléphoner à l'adresse ou au numéro du dossier, jamais à celui fourni par la demande (R-RG42).",
        obligatoire: true,
    },
    EtapeVerification {
        code: "code_courriel",
        libelle: "Code envoyé au courriel du dossier",
        geste: "Suffit lorsque le courriel figure au dossier et que la demande ne porte que sur les données du demandeur.",
        obligatoire: false,
    },
    EtapeVerification {
        code: "piece",
        libelle: "Pièce d'identité consultée",
        geste: "Consultée au guichet ou reçue, puis détruite à la clôture ; jamais conservée (R-RG16, CL27-70).",
        obligatoire: false,
    },
    EtapeVerification {
        code: "mandat",
        libelle: "Mandat pour un tiers",
        geste: "Avocat, association : mandat écrit exigé, sinon la réponse part au mandant (CL27-07).",
        obligatoire: false,
    },
    EtapeVerification {
        code: "echec",
        libelle: "Échec de la vérification",
        geste: "Réponse motivée « nous ne pouvons pas vous identifier », sans dire si la personne est connue (§9.2).",
        obligatoire: false,
    },
];

/// Un modèle de réponse par cas fréquent (RGP-304). Ce sont des **points de départ éditables** : la réponse
/// remise est toujours relue par un humain.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ModeleReponse {
    pub code: &'static str,
    pub libelle: &'static str,
    /// Droit auquel le modèle s'applique (`None` : tous).
    pub droit: Option<&'static str>,
    /// Code de motif proposé pour la décision.
    pub motif_code: &'static str,
    pub texte: &'static str,
}

/// Modèles de réponse par cas fréquent (RGP-304, §9.5). Les refus citent l'article et la raison, jamais un
/// motif générique.
pub const MODELES_REPONSE: [ModeleReponse; 8] = [
    ModeleReponse {
        code: "acces_accorde",
        libelle: "Accès accordé",
        droit: Some("acces"),
        motif_code: "art15",
        texte: "Vous trouverez ci-joint les données que l'établissement détient sur la période demandée, \
leurs finalités, leurs destinataires, leur durée de conservation et leur source. Les éléments qui \
concernent une autre personne ont été retirés : la copie ne peut pas porter atteinte aux droits d'autrui \
(article 15.4 du RGPD).",
    },
    ModeleReponse {
        code: "effacement_refuse_obligation",
        libelle: "Effacement refusé — obligation légale",
        droit: Some("effacement"),
        motif_code: "art17_3_b",
        texte: "L'effacement ne peut pas être accordé pour ces éléments : leur conservation est nécessaire \
au respect d'une obligation légale et à l'exécution d'une mission d'intérêt public (article 17.3 b du \
RGPD). Leur durée de conservation vous est indiquée ci-dessous ; ils seront éliminés à son terme.",
    },
    ModeleReponse {
        code: "effacement_refuse_archives",
        libelle: "Effacement refusé — archives",
        droit: Some("effacement"),
        motif_code: "art17_3_d",
        texte: "Ces documents relèvent de l'archivage dans l'intérêt public (article 17.3 d du RGPD et Code \
du patrimoine) : leur élimination est soumise au visa de l'administration des archives et ne peut pas être \
décidée par l'établissement.",
    },
    ModeleReponse {
        code: "effacement_partiel",
        libelle: "Effacement partiel accordé",
        droit: Some("effacement"),
        motif_code: "partiel",
        texte: "Les éléments facultatifs que vous visez ont été effacés. Les éléments dont la conservation \
est imposée par un texte sont maintenus ; vous en trouverez la liste et l'échéance ci-dessous.",
    },
    ModeleReponse {
        code: "portabilite_sans_objet",
        libelle: "Portabilité sans objet, extrait remis quand même",
        droit: Some("portabilite"),
        motif_code: "art20_3",
        texte: "Le droit à la portabilité ne s'applique pas à un traitement nécessaire à l'exécution d'une \
mission d'intérêt public (article 20.3 du RGPD). L'établissement vous remet néanmoins un extrait \
réutilisable de vos données, par souci de transparence.",
    },
    ModeleReponse {
        code: "opposition_examinee",
        libelle: "Opposition examinée",
        droit: Some("opposition"),
        motif_code: "art21",
        texte: "Votre opposition a été examinée au regard de votre situation particulière. Elle est \
accueillie pour les usages facultatifs que vous visez. Pour les traitements nécessaires à la mission de \
l'établissement, des motifs impérieux s'y opposent ; ils vous sont exposés ci-dessous.",
    },
    ModeleReponse {
        code: "identite_insuffisante",
        libelle: "Identité non établie",
        droit: None,
        motif_code: "art12_6",
        texte: "L'établissement n'a pas pu s'assurer de votre identité à partir des éléments reçus et ne \
peut donc pas donner suite à votre demande (article 12.6 du RGPD). Vous pouvez vous présenter au secrétariat \
avec une pièce d'identité, qui sera consultée puis détruite.",
    },
    ModeleReponse {
        code: "cada_document",
        libelle: "Demande d'accès à un document administratif (CADA)",
        droit: None,
        motif_code: "crpa_l311_6",
        texte: "Votre demande porte sur un document administratif. Les mentions qui révèlent la vie privée, \
une appréciation ou un jugement de valeur sur une personne nommément désignée ont été occultées avant \
communication (article L311-6 du code des relations entre le public et l'administration).",
    },
];

// ---------------------------------------------------------------------------------------------
// Lecture d'une ligne
// ---------------------------------------------------------------------------------------------

const COLONNES: &str = "id, annee, numero, canal, recue_le, echeance_le, prolongee_jusqu_au, \
prolongation_motif, demandeur_compte_id, demandeur_nom_saisi, qualite, sujet_type, sujet_id, \
sujet_nom_saisi, qualification, qualification_motif, perimetre_texte, perimetre_du, perimetre_au, etat, \
gestionnaire_personne_id, identite_demandee_le, identite_fournie_le, reouvertures, close_le, \
reponse_resume, refus_motif, titulaires_informes_le, titulaires_non_informes_motif, cree_le";

struct Brut {
    id: Uuid,
    annee: i32,
    numero: i32,
    canal: String,
    recue_le: NaiveDate,
    echeance_le: NaiveDate,
    prolongee_jusqu_au: Option<NaiveDate>,
    prolongation_motif: Option<String>,
    demandeur_compte_id: Option<Uuid>,
    demandeur_nom_saisi: Option<String>,
    qualite: String,
    sujet_type: String,
    sujet_id: Option<Uuid>,
    sujet_nom_saisi: Option<String>,
    qualification: String,
    qualification_motif: Option<String>,
    perimetre_texte: Option<String>,
    perimetre_du: Option<NaiveDate>,
    perimetre_au: Option<NaiveDate>,
    etat: String,
    gestionnaire_personne_id: Option<Uuid>,
    identite_demandee_le: Option<NaiveDate>,
    identite_fournie_le: Option<NaiveDate>,
    reouvertures: i32,
    close_le: Option<DateTime<Utc>>,
    reponse_resume: Option<String>,
    refus_motif: Option<String>,
    titulaires_informes_le: Option<DateTime<Utc>>,
    titulaires_non_informes_motif: Option<String>,
    cree_le: DateTime<Utc>,
}

fn brut(r: &sqlx::postgres::PgRow) -> Brut {
    Brut {
        id: r.get("id"),
        annee: r.get("annee"),
        numero: r.get("numero"),
        canal: r.get("canal"),
        recue_le: r.get("recue_le"),
        echeance_le: r.get("echeance_le"),
        prolongee_jusqu_au: r.get("prolongee_jusqu_au"),
        prolongation_motif: r.get("prolongation_motif"),
        demandeur_compte_id: r.get("demandeur_compte_id"),
        demandeur_nom_saisi: r.get("demandeur_nom_saisi"),
        qualite: r.get("qualite"),
        sujet_type: r.get("sujet_type"),
        sujet_id: r.get("sujet_id"),
        sujet_nom_saisi: r.get("sujet_nom_saisi"),
        qualification: r.get("qualification"),
        qualification_motif: r.get("qualification_motif"),
        perimetre_texte: r.get("perimetre_texte"),
        perimetre_du: r.get("perimetre_du"),
        perimetre_au: r.get("perimetre_au"),
        etat: r.get("etat"),
        gestionnaire_personne_id: r.get("gestionnaire_personne_id"),
        identite_demandee_le: r.get("identite_demandee_le"),
        identite_fournie_le: r.get("identite_fournie_le"),
        reouvertures: r.get("reouvertures"),
        close_le: r.get("close_le"),
        reponse_resume: r.get("reponse_resume"),
        refus_motif: r.get("refus_motif"),
        titulaires_informes_le: r.get("titulaires_informes_le"),
        titulaires_non_informes_motif: r.get("titulaires_non_informes_motif"),
        cree_le: r.get("cree_le"),
    }
}

/// Référence lisible par l'usager : « 2027-014 ».
pub fn reference(annee: i32, numero: i32) -> String {
    format!("{annee}-{numero:03}")
}

/// Échéance qui s'applique : la prolongation si elle existe, sinon l'échéance légale. La suspension pour
/// précision d'identité n'y est ajoutée que si [`IDENTITE_SUSPEND_DELAI`] est vrai (§9.2).
fn echeance_effective(b: &Brut) -> NaiveDate {
    let base = b.prolongee_jusqu_au.unwrap_or(b.echeance_le);
    if !IDENTITE_SUSPEND_DELAI {
        return base;
    }
    match attente_jours(b) {
        j if j > 0 => base + chrono::Duration::days(j),
        _ => base,
    }
}

/// Jours d'attente d'une précision d'identité, bornés à la date de fourniture quand elle existe.
fn attente_jours(b: &Brut) -> i64 {
    let Some(demandee) = b.identite_demandee_le else {
        return 0;
    };
    let fin = b.identite_fournie_le.unwrap_or(demandee);
    (fin - demandee).num_days().max(0)
}

fn ouverte(etat: &str) -> bool {
    !matches!(etat, "close" | "refusee")
}

fn avancement(b: &Brut, echeance: NaiveDate) -> String {
    let d = b.recue_le.format("%d/%m/%Y");
    let e = echeance.format("%d/%m/%Y");
    let etat = match b.etat.as_str() {
        "recue" => "reçue, en attente de qualification",
        "identite_a_verifier" => "en attente de vérification de votre identité",
        "en_cours" => "en cours d'instruction",
        "a_relire" => "en cours de relecture",
        "prete" => "prête, en attente de remise",
        "remise" => "remise",
        "close" => "close",
        "refusee" => "refusée",
        _ => "en cours",
    };
    format!("Reçue le {d}, échéance le {e} — {etat}.")
}

// ---------------------------------------------------------------------------------------------
// Journal (R-RG40) et accusé de réception (RGP-145)
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn journal(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    evenement: &str,
    detail: Option<&str>,
    par_compte: Option<Uuid>,
    par_role: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO rgp_demande_journal (id, etablissement_id, demande_id, evenement, detail, par_compte_id, par_role)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(demande)
    .bind(evenement)
    .bind(detail)
    .bind(par_compte)
    .bind(par_role)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Accusé de réception (RGP-145) : date, échéance, contact du DPO quand il est nommé. **Aucun nom d'élève**
/// dans la notification (consigne du lot, E14, R-RG32) : la demande est nommée par sa référence.
async fn accuser_reception(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    reference: &str,
    recue_le: NaiveDate,
    echeance: NaiveDate,
    destinataire: Uuid,
) -> Result<bool, EntDbError> {
    let contact: Option<String> =
        sqlx::query_scalar("SELECT dpo_contact FROM rgp_gouvernance WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
            .flatten();
    let corps = format!(
        "Votre demande {reference} a été reçue le {}. L'établissement y répondra au plus tard le {}.{}",
        recue_le.format("%d/%m/%Y"),
        echeance.format("%d/%m/%Y"),
        contact
            .map(|c| format!(" Contact du délégué à la protection des données : {c}."))
            .unwrap_or_default()
    );
    let n = notifier_personne(
        &mut *conn,
        EnvoiAPlanifier {
            etablissement_id: etab,
            user_id: destinataire,
            kind: KIND_ACCUSE,
            titre: "Demande enregistrée",
            corps: &corps,
            eleve_id: None,
            notification_id: None,
            // Le socle 8 borne `source_type` : un avis émis par le guichet est un geste du système.
            source_type: "systeme",
            source_id: demande,
            cle_dedup: format!("rgpd:demande:{demande}:accuse"),
            droit_requis: None,
            regroupable: false,
            canaux_voulus: &["email", "push"],
        },
        demande,
        Utc::now(),
    )
    .await?;
    Ok(n > 0)
}

// ---------------------------------------------------------------------------------------------
// Qualité vérifiée par le socle 4, à la DATE de réception (§9.2, §9.3)
// ---------------------------------------------------------------------------------------------

/// Ce que le socle 4 constate du demandeur, **au jour de réception** de la demande.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum QualiteConstatee {
    /// Lien actif à la date, avec autorité parentale.
    Confirmee,
    /// Lien actif à la date, **sans** exercice de l'autorité parentale (C. civ. 373-2-1, CL27-03).
    ConfirmeeSansAutorite,
    /// Lien actif mais restreint (suspension, adresse protégée) : arbitrage de la direction (CL27-02).
    Restreinte,
    /// Aucun lien actif à cette date.
    Absente,
    /// La qualité ne se vérifie pas par un lien familial (élève majeur, personnel, ancien élève, tiers).
    SansObjet,
}

impl QualiteConstatee {
    pub fn code(self) -> &'static str {
        match self {
            Self::Confirmee => "confirmee",
            Self::ConfirmeeSansAutorite => "confirmee_sans_autorite",
            Self::Restreinte => "restreinte",
            Self::Absente => "absente",
            Self::SansObjet => "sans_objet",
        }
    }
}

/// Lien du demandeur sur l'élève sujet, **à la date** `jour` : un lien hors de ses dates ou suspendu ne
/// compte pas (socle 4, migration 121). La requête n'utilise jamais `now()` : l'horloge de la demande est
/// sa date de réception, pas le jour où le guichet regarde (règle 33 des consignes de pilotage).
async fn qualite_a_la_date(
    conn: &mut PgConnection,
    etab: Uuid,
    demandeur_compte: Option<Uuid>,
    sujet_type: &str,
    sujet_id: Option<Uuid>,
    jour: NaiveDate,
) -> Result<QualiteConstatee, EntDbError> {
    let (Some(compte), Some(eleve)) = (demandeur_compte, sujet_id) else {
        return Ok(QualiteConstatee::SansObjet);
    };
    if sujet_type != "eleve" {
        return Ok(QualiteConstatee::SansObjet);
    }
    let l: Option<(bool, bool)> = sqlx::query_as(
        "SELECT l.autorite_parentale, (l.suspendu_le IS NOT NULL) AS suspendu
           FROM ent_eleve_lien l
           JOIN ent_personne p ON p.user_id = l.user_id AND p.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3
            AND (l.date_debut IS NULL OR l.date_debut <= $4)
            AND (l.date_fin IS NULL OR l.date_fin >= $4)
            AND p.fusionnee_dans IS NULL
          ORDER BY l.autorite_parentale DESC
          LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .bind(compte)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((autorite, suspendu)) = l else {
        return Ok(QualiteConstatee::Absente);
    };
    if suspendu {
        return Ok(QualiteConstatee::Restreinte);
    }
    // Protection au dossier (adresse mise de côté, « ne pas solliciter ») : la qualité devient **restreinte**
    // et la suite passe par un arbitrage de la direction (CL27-02, R-RG38, R-RG46). Le guichet n'apprend ici
    // que l'existence d'une protection, jamais son contenu — et le journal visible ne l'expose jamais au
    // parent qu'elle vise (R-RG46, lot G4).
    let protegee: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM ent_adresse_protegee a
              JOIN ent_personne p ON p.id = a.personne_id AND p.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1
                AND (p.eleve_id = $2
                     OR EXISTS (SELECT 1 FROM ent_eleve_lien l
                                 WHERE l.etablissement_id = a.etablissement_id
                                   AND l.eleve_id = $2 AND l.user_id = p.user_id)))
            OR EXISTS (
             SELECT 1 FROM ent_personne_reserve r
              JOIN ent_personne p ON p.id = r.personne_id AND p.etablissement_id = r.etablissement_id
              WHERE r.etablissement_id = $1 AND r.ne_pas_solliciter
                AND (p.eleve_id = $2
                     OR EXISTS (SELECT 1 FROM ent_eleve_lien l
                                 WHERE l.etablissement_id = r.etablissement_id
                                   AND l.eleve_id = $2 AND l.user_id = p.user_id)))",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(match (autorite, protegee) {
        (_, true) => QualiteConstatee::Restreinte,
        (true, false) => QualiteConstatee::Confirmee,
        (false, false) => QualiteConstatee::ConfirmeeSansAutorite,
    })
}

/// L'élève est-il majeur **à la date** donnée (CL27-04, élève majeur de §9.3) ?
async fn eleve_majeur_le(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<Option<bool>, EntDbError> {
    let m: Option<Option<NaiveDate>> = sqlx::query_scalar(
        "SELECT ent_eleve_majorite_le(date_naissance, date_naissance_precision, date_naissance_statut)
           FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(m.map(|d| d.is_some_and(|j| j <= jour)))
}

// ---------------------------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------------------------

impl PgEnt {
    /// Grille de vérification d'identité (RGP-307) et modèles de réponse (RGP-304) : référentiel figé dans
    /// le code, sans accès à la base. Lecture réservée au guichet.
    pub async fn droits_referentiel(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(&'static [EtapeVerification], &'static [ModeleReponse]), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsRecevoir)
            .await?;
        Ok((&GRILLE_VERIFICATION, &MODELES_REPONSE))
    }

    // ── Dépôt ───────────────────────────────────────────────────────────────────────────────

    /// **RGP-140** — dépôt par un demandeur connecté (parent, élève, élève majeur, personnel). La qualité
    /// invoquée est vérifiée par le socle 4 **à la date de la demande** ; une qualité contredite par les
    /// liens est refusée tout de suite (jamais un dossier ouvert sur une qualité fausse).
    ///
    /// Ce qui est déduit (exigence « masterclass », cas de §9.3 et §13.1) :
    /// * un parent ne demande jamais les données propres de l'autre parent : `qualite = 'tiers'` sur une
    ///   personne est refusée (R-RG38) ;
    /// * un élève mineur dépose pour lui-même (CL27-04) ; ses titulaires sont informés à la qualification,
    ///   sauf motif sérieux (S-550) ;
    /// * un élève majeur exerce seul : un parent qui invoque `titulaire_autorite` sur un élève majeur est
    ///   refusé ;
    /// * une demande « au nom de l'enfant » depuis le compte d'un parent est la demande **du parent**
    ///   (CL27-65) : la qualité le dit, et le sujet reste l'enfant ;
    /// * une qualité « pour moi-même » (`soi_meme`, `eleve_mineur`, `personnel`, `ancien_eleve`) ne vaut que
    ///   **sur soi** : le sujet est résolu depuis le compte quand il n'est pas donné, et refusé quand il
    ///   désigne quelqu'un d'autre. Sans ce contrôle, un compte pourrait ouvrir un dossier sur la personne
    ///   de son choix et recevoir son extrait au lot G3b.
    pub async fn droits_deposer_connecte(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: DepotConnecte<'_>,
    ) -> Result<DemandeVue, EntDbError> {
        // Aucun droit d'établissement n'est exigé : toute personne rattachée peut déposer pour elle-même ou
        // pour l'enfant dont elle est titulaire. L'appartenance et la qualité font le contrôle.
        let membre = self.acces_etat(etab, acteur).await?;
        if membre.is_none() {
            return Err(EntDbError::Forbidden);
        }
        valider_droits(d.droits)?;
        if !QUALITES.contains(&d.qualite) {
            return Err(invalide("qualité inconnue"));
        }
        if matches!(
            d.qualite,
            "tiers" | "parent_sans_autorite" | "mandataire" | "representant_majeur_protege"
        ) {
            // Un dépôt en ligne ne sert pas à réclamer les données d'un tiers : ces qualités exigent une
            // pièce (mandat, jugement de tutelle) et passent par le guichet (CL27-07, CL27-15, R-RG38).
            return Err(invalide(
                "cette qualité se dépose au guichet, avec les pièces",
            ));
        }
        let perimetre = d.perimetre_texte.and_then(|t| texte(t, 3, 2000));
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Qualité « pour moi-même » : le sujet est le compte lui-même. Résolu ici (l'usager ne connaît pas
        // son identifiant interne) et vérifié quand il est fourni.
        let (sujet_type, sujet_id) =
            resoudre_sujet_de_soi(&mut tx, etab, acteur, d.qualite, d.sujet_type, d.sujet_id)
                .await?;
        verifier_sujet(&mut tx, etab, sujet_type, sujet_id, None).await?;
        let constatee =
            qualite_a_la_date(&mut tx, etab, Some(acteur), sujet_type, sujet_id, jour).await?;
        exiger_coherence_qualite(
            &mut tx, etab, d.qualite, sujet_type, sujet_id, jour, constatee,
        )
        .await?;
        let personne: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
        )
        .bind(etab)
        .bind(acteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = inserer(
            &mut tx,
            etab,
            Insertion {
                canal: "en_ligne",
                recue_le: jour,
                jour,
                demandeur_compte_id: Some(acteur),
                demandeur_personne_id: personne,
                demandeur_nom_saisi: None,
                demandeur_contact: None,
                qualite: d.qualite,
                sujet_type,
                sujet_id,
                sujet_nom_saisi: None,
                perimetre_texte: perimetre.as_deref(),
                perimetre_du: d.perimetre_du,
                perimetre_au: d.perimetre_au,
                droits: d.droits,
                // L'identité est acquise (session) ; la qualité vient d'être constatée.
                etat: "recue",
                identite: Some(("session_forte", constatee)),
                cree_par: Some(acteur),
            },
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            "recue",
            Some("dépôt en ligne (demandeur connecté)"),
            Some(acteur),
            Some(role),
        )
        .await?;
        let (annee, numero): (i32, i32) = sqlx::query_as(
            "SELECT annee, numero FROM rgp_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let reference = reference(annee, numero);
        let envoye = accuser_reception(
            &mut tx,
            etab,
            id,
            &reference,
            jour,
            echeance_legale(jour),
            acteur,
        )
        .await?;
        if envoye {
            journal(&mut tx, etab, id, "accuse_envoye", None, None, None).await?;
        }
        let vue = charger(&mut tx, etab, id, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-142** — dépôt sans compte (ancien élève, parent sans compte). Public : aucune session.
    ///
    /// Rien n'est confirmé au déposant (R-RG42, CL27-66) : la fonction rend toujours
    /// [`MESSAGE_SANS_COMPTE`] et l'identifiant interne de la demande n'est **pas** rendu (un inconnu ne
    /// doit pas pouvoir suivre, ni deviner, un dossier). La demande naît en `identite_a_verifier` ; le
    /// guichet reprend contact par un canal déjà au dossier. Le délai court dès la réception (§9.2).
    pub async fn droits_deposer_sans_compte(
        &self,
        etab: Uuid,
        d: DepotSansCompte<'_>,
    ) -> Result<&'static str, EntDbError> {
        valider_droits(d.droits)?;
        if !matches!(
            d.qualite,
            "soi_meme" | "ancien_eleve" | "titulaire_autorite"
        ) {
            return Err(invalide("qualité non acceptée sans compte"));
        }
        let nom = texte(d.nom_saisi, 2, 200).ok_or(invalide("nom invalide"))?;
        let contact = texte(d.contact, 3, 254).ok_or(invalide("contact invalide"))?;
        let sujet_nom = texte(d.sujet_nom_saisi, 2, 200).ok_or(invalide("nom invalide"))?;
        let perimetre = d.perimetre_texte.and_then(|t| texte(t, 3, 2000));
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let id = inserer(
            &mut tx,
            etab,
            Insertion {
                canal: "en_ligne",
                recue_le: jour,
                jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some(&nom),
                demandeur_contact: Some(&contact),
                qualite: d.qualite,
                // Le sujet n'est pas rapproché d'un dossier par le dépôt lui-même : le guichet le fait, après
                // vérification. Un rapprochement automatique dirait au déposant que la personne existe.
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some(&sujet_nom),
                perimetre_texte: perimetre.as_deref(),
                perimetre_du: None,
                perimetre_au: None,
                droits: d.droits,
                etat: "identite_a_verifier",
                identite: None,
                cree_par: None,
            },
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            "recue",
            Some("dépôt en ligne sans compte : identité à vérifier"),
            None,
            None,
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            "identite_demandee",
            Some("reprise de contact par un canal du dossier (R-RG42)"),
            None,
            None,
        )
        .await?;
        sqlx::query("UPDATE rgp_demande SET identite_demandee_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(jour)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await?;
        Ok(MESSAGE_SANS_COMPTE)
    }

    /// **RGP-143** — saisie par le guichet d'une demande reçue par courrier, courriel ou en personne, ou
    /// transmise par le DPO ou la CNIL. La **date de réception est celle du courrier** : elle peut précéder
    /// la saisie, jamais la suivre. RGP-303 : l'écart est conservé (`cree_le` figé par la base) et le
    /// tableau le signale au-delà de [`ECART_SAISIE_ALERTE_JOURS`] jours.
    pub async fn droits_saisir_au_guichet(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        s: SaisieDemande<'_>,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsRecevoir)
            .await?;
        if !CANAUX_GUICHET.contains(&s.canal) {
            return Err(invalide("canal inconnu pour une saisie au guichet"));
        }
        valider_droits(s.droits)?;
        if !QUALITES.contains(&s.qualite) {
            return Err(invalide("qualité inconnue"));
        }
        let jour = self.aujourdhui_etablissement(etab).await?;
        if s.recue_le > jour {
            return Err(invalide("la date de réception ne peut pas être future"));
        }
        // Une réception antérieure de plus de deux ans est une erreur de saisie, pas un courrier retrouvé.
        if (jour - s.recue_le).num_days() > 730 {
            return Err(invalide(
                "date de réception trop ancienne : vérifier la saisie",
            ));
        }
        let nom = s.demandeur_nom_saisi.and_then(|n| texte(n, 2, 200));
        if s.demandeur_compte_id.is_none() && nom.is_none() {
            return Err(invalide("nom du demandeur obligatoire sans compte"));
        }
        let contact = s.demandeur_contact.and_then(|c| texte(c, 3, 254));
        let sujet_nom = s.sujet_nom_saisi.and_then(|n| texte(n, 2, 200));
        let perimetre = s.perimetre_texte.and_then(|t| texte(t, 3, 2000));
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        verifier_sujet(
            &mut tx,
            etab,
            s.sujet_type,
            s.sujet_id,
            sujet_nom.as_deref(),
        )
        .await?;
        if let Some(p) = s.demandeur_personne_id {
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_personne WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(p)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(EntDbError::Domain(DomainError::NotFound));
            }
        }
        let id = inserer(
            &mut tx,
            etab,
            Insertion {
                canal: s.canal,
                recue_le: s.recue_le,
                jour,
                demandeur_compte_id: s.demandeur_compte_id,
                demandeur_personne_id: s.demandeur_personne_id,
                demandeur_nom_saisi: nom.as_deref(),
                demandeur_contact: contact.as_deref(),
                qualite: s.qualite,
                sujet_type: s.sujet_type,
                sujet_id: s.sujet_id,
                sujet_nom_saisi: sujet_nom.as_deref(),
                perimetre_texte: perimetre.as_deref(),
                perimetre_du: s.perimetre_du,
                perimetre_au: s.perimetre_au,
                droits: s.droits,
                // Une demande arrivée par courrier n'est pas authentifiée : l'identité reste à vérifier,
                // même si le guichet a vu la personne (il l'enregistre alors par `droits_verifier_identite`).
                etat: "identite_a_verifier",
                identite: None,
                cree_par: Some(acteur),
            },
        )
        .await?;
        let ecart = (jour - s.recue_le).num_days();
        let detail = if ecart > ECART_SAISIE_ALERTE_JOURS {
            format!(
                "saisie {ecart} jours après la réception (canal {})",
                s.canal
            )
        } else {
            format!("saisie au guichet (canal {})", s.canal)
        };
        journal(
            &mut tx,
            etab,
            id,
            "recue",
            Some(&detail),
            Some(acteur),
            Some(role),
        )
        .await?;
        // Accusé de réception : seulement si le demandeur a un compte dans cet établissement. Sinon, il part
        // par le canal du dossier, hors ENT (R-RG42) : le guichet le fait et le journal l'enregistre.
        if let Some(compte) = s.demandeur_compte_id {
            let (annee, numero): (i32, i32) = sqlx::query_as(
                "SELECT annee, numero FROM rgp_demande WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let envoye = accuser_reception(
                &mut tx,
                etab,
                id,
                &reference(annee, numero),
                s.recue_le,
                echeance_legale(s.recue_le),
                compte,
            )
            .await?;
            if envoye {
                journal(&mut tx, etab, id, "accuse_envoye", None, None, None).await?;
            }
        }
        let vue = charger(&mut tx, etab, id, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    // ── Instruction ─────────────────────────────────────────────────────────────────────────

    /// **RGP-141** — qualifie la demande (RGPD, CADA, simple demande de document). Motivée et modifiable.
    /// La requalification est tracée : le journal garde l'enchaînement des qualifications.
    ///
    /// Déduction (§9.4, CL27-04) : quand un **élève mineur** a déposé pour lui-même, la qualification est le
    /// moment où le guichet décide d'informer ou non ses titulaires (S-550) ; la fonction
    /// [`PgEnt::droits_titulaires_information`] l'enregistre et la demande reste signalée tant que rien n'est
    /// décidé.
    pub async fn droits_qualifier(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        qualification: &str,
        motif: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        if !matches!(qualification, "rgpd" | "cada" | "document") {
            return Err(invalide("qualification inconnue"));
        }
        let motif = texte(motif, 3, 1000).ok_or(invalide("motif de qualification obligatoire"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        // Qualifier fait passer la demande à l'instruction, sauf si l'identité reste à établir.
        let suite = if etat == "identite_a_verifier" {
            "identite_a_verifier"
        } else {
            "en_cours"
        };
        sqlx::query(
            "UPDATE rgp_demande
                SET qualification = $3, qualification_motif = $4, qualifiee_le = now(),
                    qualifiee_par_compte_id = $5, etat = $6, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(qualification)
        .bind(&motif)
        .bind(acteur)
        .bind(suite)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "qualifiee",
            Some(qualification),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-144** — enregistre une vérification d'identité et de qualité. La qualité est **recalculée à la
    /// date de réception** par le socle 4 (jamais au jour du contrôle) : un lien ouvert ou fermé depuis la
    /// demande ne change pas ce qui était vrai à la réception.
    ///
    /// `piece_document_id` : document du coffre (socle 7) consulté. Il est **détruit à la clôture**
    /// (R-RG16, CL27-70) ; aucune autre fonction ne le rend.
    #[allow(clippy::too_many_arguments)]
    pub async fn droits_verifier_identite(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        mode: &str,
        reussie: bool,
        note: Option<&str>,
        piece_document_id: Option<Uuid>,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsRecevoir)
            .await?;
        if !matches!(mode, "session_forte" | "code_courriel" | "piece" | "mandat") {
            return Err(invalide("mode de vérification inconnu"));
        }
        if mode == "piece" && piece_document_id.is_none() && reussie {
            return Err(invalide("référence de la pièce consultée obligatoire"));
        }
        let note = note.and_then(|n| texte(n, 3, 1000));
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "SELECT etat, recue_le, demandeur_compte_id, sujet_type, sujet_id, qualite
               FROM rgp_demande WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let etat: String = r.get("etat");
        exiger_ouverte(&etat)?;
        let recue_le: NaiveDate = r.get("recue_le");
        let compte: Option<Uuid> = r.get("demandeur_compte_id");
        let sujet_type: String = r.get("sujet_type");
        let sujet_id: Option<Uuid> = r.get("sujet_id");
        let constatee =
            qualite_a_la_date(&mut tx, etab, compte, &sujet_type, sujet_id, recue_le).await?;
        let resultat = if reussie { "verifiee" } else { "echouee" };
        sqlx::query(
            "INSERT INTO rgp_demande_identite
                 (id, etablissement_id, demande_id, mode, resultat, qualite_constatee, qualite_note,
                  piece_document_id, verifie_par_compte_id, verifie_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(demande)
        .bind(mode)
        .bind(resultat)
        .bind(constatee.code())
        .bind(&note)
        .bind(if reussie { piece_document_id } else { None })
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if reussie {
            sqlx::query(
                "UPDATE rgp_demande
                    SET identite_fournie_le = COALESCE(identite_fournie_le, $3),
                        etat = CASE WHEN etat = 'identite_a_verifier' THEN 'en_cours' ELSE etat END,
                        modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(demande)
            .bind(jour)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journal(
            &mut tx,
            etab,
            demande,
            if reussie {
                "identite_verifiee"
            } else {
                "identite_echouee"
            },
            Some(&format!("{mode} — qualité {}", constatee.code())),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// Demande une précision d'identité (§9.2) : la date d'ouverture de l'attente est posée et l'état passe à
    /// `identite_a_verifier`. L'attente est **comptée**, elle ne repousse pas l'échéance tant que
    /// [`IDENTITE_SUSPEND_DELAI`] est faux.
    pub async fn droits_demander_identite(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        quoi: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsRecevoir)
            .await?;
        let quoi = texte(quoi, 3, 1000).ok_or(invalide("préciser ce qui est demandé"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        sqlx::query(
            "UPDATE rgp_demande
                SET identite_demandee_le = COALESCE(identite_demandee_le, $3),
                    identite_fournie_le = NULL, etat = 'identite_a_verifier', modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(jour)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "identite_demandee",
            Some(&quoi),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-147** — affecte la demande à un gestionnaire (une personne de l'établissement). Direction et
    /// DPO ; le droit `droits.traiter` suffit pour se l'attribuer, l'arbitrage n'est pas requis.
    pub async fn droits_affecter(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        gestionnaire_personne_id: Option<Uuid>,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        if let Some(p) = gestionnaire_personne_id {
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_personne
                                 WHERE etablissement_id = $1 AND id = $2 AND fusionnee_dans IS NULL)",
            )
            .bind(etab)
            .bind(p)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(EntDbError::Domain(DomainError::NotFound));
            }
        }
        sqlx::query(
            "UPDATE rgp_demande
                SET gestionnaire_personne_id = $3,
                    affectee_le = CASE WHEN $3::uuid IS NULL THEN NULL ELSE now() END,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(gestionnaire_personne_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "affectee",
            gestionnaire_personne_id
                .map(|_| "gestionnaire désigné")
                .or(Some("affectation retirée")),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-148** — prolonge de deux mois au plus, avec motif, et informe le demandeur **avant** l'échéance
    /// du premier mois (art. 12.3). Réservé à `droits.arbitrer`.
    ///
    /// Cas limites traités : une prolongation demandée après l'échéance du premier mois est refusée
    /// (l'information due dans le premier mois n'a plus d'objet, le retard est constaté tel quel) ; une
    /// seconde prolongation est refusée (le texte n'en prévoit qu'une).
    pub async fn droits_prolonger(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        jusqu_au: NaiveDate,
        motif: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        let motif = texte(motif, 3, 1000).ok_or(invalide("motif de prolongation obligatoire"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "SELECT etat, echeance_le, prolongee_jusqu_au, demandeur_compte_id, annee, numero
               FROM rgp_demande WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let etat: String = r.get("etat");
        exiger_ouverte(&etat)?;
        let echeance: NaiveDate = r.get("echeance_le");
        let deja: Option<NaiveDate> = r.get("prolongee_jusqu_au");
        if deja.is_some() {
            return Err(EntDbError::ConflitDossier(
                "une demande ne se prolonge qu'une fois (deux mois au plus)",
            ));
        }
        if jour > echeance {
            return Err(EntDbError::ConflitDossier(
                "l'échéance du premier mois est passée : la prolongation devait être notifiée avant",
            ));
        }
        if jusqu_au <= echeance {
            return Err(invalide("la prolongation doit dépasser l'échéance légale"));
        }
        if jusqu_au > prolongation_max(echeance) {
            return Err(invalide("deux mois de prolongation au plus (art. 12.3)"));
        }
        sqlx::query(
            "UPDATE rgp_demande
                SET prolongee_jusqu_au = $3, prolongation_motif = $4, prolongee_le = now(),
                    prolongee_par_compte_id = $5, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(jusqu_au)
        .bind(&motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "prolongee",
            Some(&motif),
            Some(acteur),
            Some(role),
        )
        .await?;
        // Information du demandeur : seulement s'il a un compte ici (sinon, canal du dossier, R-RG42).
        if let Some(compte) = r.get::<Option<Uuid>, _>("demandeur_compte_id") {
            let reference = reference(r.get("annee"), r.get("numero"));
            let corps = format!(
                "Le traitement de votre demande {reference} demande un délai supplémentaire. La réponse vous \
parviendra au plus tard le {}. Motif : {motif}",
                jusqu_au.format("%d/%m/%Y")
            );
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: compte,
                    kind: KIND_PROLONGEE,
                    titre: "Demande prolongée",
                    corps: &corps,
                    eleve_id: None,
                    notification_id: None,
                    // Le socle 8 borne `source_type` : un avis émis par le guichet est un geste du système.
                    source_type: "systeme",
                    source_id: demande,
                    cle_dedup: format!("rgpd:demande:{demande}:prolongee"),
                    droit_requis: None,
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                demande,
                Utc::now(),
            )
            .await?;
        }
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// Ajoute un droit à une demande ouverte (un parent qui complète sa demande ne rouvre pas un dossier).
    pub async fn droits_ajouter_droit(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        droit: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        if !DROITS.contains(&droit) {
            return Err(invalide("droit inconnu"));
        }
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        let r = sqlx::query(
            "INSERT INTO rgp_demande_droit (id, etablissement_id, demande_id, droit)
             VALUES ($1, $2, $3, $4) ON CONFLICT (etablissement_id, demande_id, droit) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(demande)
        .bind(droit)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("ce droit est déjà demandé"));
        }
        journal(
            &mut tx,
            etab,
            demande,
            "droit_ajoute",
            Some(droit),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-157, RGP-160 à RGP-162** — décide d'un droit : accordé, partiel, refusé ou sans objet, avec un
    /// motif par article. Un **refus ou un partiel exige toujours un motif écrit** (contrainte de base) ;
    /// jamais de refus par défaut (§9.5).
    ///
    /// Partage des droits : accorder relève de `droits.traiter` ; **refuser** (comme la limitation d'une
    /// opposition) relève de `droits.arbitrer` — la direction (R-RG17).
    #[allow(clippy::too_many_arguments)]
    pub async fn droits_decider(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        droit: &str,
        decision: &str,
        motif_code: Option<&str>,
        motif_texte: Option<&str>,
    ) -> Result<DemandeVue, EntDbError> {
        let arbitrage = matches!(decision, "refuse" | "partiel");
        self.exiger_action_etab(
            etab,
            acteur,
            role,
            if arbitrage {
                Action::DroitsArbitrer
            } else {
                Action::DroitsTraiter
            },
        )
        .await?;
        if !DROITS.contains(&droit) {
            return Err(invalide("droit inconnu"));
        }
        if !matches!(decision, "accorde" | "partiel" | "refuse" | "sans_objet") {
            return Err(invalide("décision inconnue"));
        }
        let code = motif_code.and_then(|c| texte(c, 2, 60)).filter(|c| {
            c.chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '.')
        });
        let txt = motif_texte.and_then(|t| texte(t, 3, 4000));
        if arbitrage && (code.is_none() || txt.is_none()) {
            return Err(invalide(
                "un refus ou un partiel est motivé (article et texte)",
            ));
        }
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        let r = sqlx::query(
            "UPDATE rgp_demande_droit
                SET decision = $4, motif_code = $5, motif_texte = $6, decide_le = now(),
                    decide_par_compte_id = $7
              WHERE etablissement_id = $1 AND demande_id = $2 AND droit = $3",
        )
        .bind(etab)
        .bind(demande)
        .bind(droit)
        .bind(decision)
        .bind(&code)
        .bind(&txt)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        journal(
            &mut tx,
            etab,
            demande,
            "droit_decide",
            Some(&format!("{droit} : {decision}")),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **CL27-04, S-550** — enregistre la décision d'informer (ou non) les titulaires de l'autorité
    /// parentale quand l'élève mineur a demandé lui-même. Informer est la règle ; ne pas informer exige un
    /// **motif sérieux écrit**, et la décision est tracée. Réservé à `droits.arbitrer` lorsqu'on n'informe
    /// pas (la direction prend la responsabilité).
    pub async fn droits_titulaires_information(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        informer: bool,
        motif: Option<&str>,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(
            etab,
            acteur,
            role,
            if informer {
                Action::DroitsTraiter
            } else {
                Action::DroitsArbitrer
            },
        )
        .await?;
        let motif = motif.and_then(|m| texte(m, 3, 1000));
        if !informer && motif.is_none() {
            return Err(invalide(
                "ne pas informer les titulaires exige un motif sérieux écrit",
            ));
        }
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "SELECT etat, qualite FROM rgp_demande WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        exiger_ouverte(&r.get::<String, _>("etat"))?;
        if r.get::<String, _>("qualite") != "eleve_mineur" {
            return Err(EntDbError::ConflitDossier(
                "cette décision ne concerne que la demande d'un élève mineur",
            ));
        }
        sqlx::query(
            "UPDATE rgp_demande
                SET titulaires_informes_le = CASE WHEN $3 THEN now() ELSE NULL END,
                    titulaires_non_informes_motif = CASE WHEN $3 THEN NULL ELSE $4 END,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(informer)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "titulaires_informes",
            Some(if informer {
                "titulaires informés"
            } else {
                "titulaires non informés : motif tracé"
            }),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    // ── Clôture et réouverture ──────────────────────────────────────────────────────────────

    /// **RGP-166** — clôt la demande : réponse résumée, pièce d'identité **détruite** (R-RG16), gel levé
    /// (le moteur du lot G6 ne voit plus de demande ouverte sur la personne, R-RG19).
    ///
    /// Un droit resté `en_examen` empêche la clôture : on ne clôt pas une demande sans avoir répondu à
    /// chacun des droits exercés (cas limite déduit de §9.4 point 6).
    pub async fn droits_clore(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        reponse_resume: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let resume =
            texte(reponse_resume, 3, 4000).ok_or(invalide("résumé de réponse obligatoire"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        let en_examen: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM rgp_demande_droit
              WHERE etablissement_id = $1 AND demande_id = $2 AND decision = 'en_examen'",
        )
        .bind(etab)
        .bind(demande)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if en_examen > 0 {
            return Err(EntDbError::ConflitDossier(
                "chaque droit exercé reçoit une décision avant la clôture",
            ));
        }
        sqlx::query(
            "UPDATE rgp_demande
                SET etat = 'close', close_le = now(), close_par_compte_id = $3, reponse_resume = $4,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(acteur)
        .bind(&resume)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let pieces = detruire_pieces(&mut tx, etab, demande, acteur, role).await?;
        journal(
            &mut tx,
            etab,
            demande,
            "close",
            Some(&format!("{pieces} pièce(s) d'identité détruite(s)")),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-164** — refus d'une demande manifestement excessive, ou impossibilité d'identifier le
    /// demandeur : **décision motivée de la direction**, jamais automatique (R-RG17, art. 12.5). La pièce
    /// d'identité est détruite comme à toute clôture (R-RG16).
    pub async fn droits_refuser(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        motif: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        let motif = texte(motif, 3, 2000).ok_or(invalide("motif de refus obligatoire"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        sqlx::query(
            "UPDATE rgp_demande
                SET etat = 'refusee', close_le = now(), close_par_compte_id = $3, refus_motif = $4,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .bind(acteur)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Les droits restés en examen deviennent « refusés », avec le même motif : aucune décision muette.
        sqlx::query(
            "UPDATE rgp_demande_droit
                SET decision = 'refuse', motif_code = 'art12_5', motif_texte = $4, decide_le = now(),
                    decide_par_compte_id = $3
              WHERE etablissement_id = $1 AND demande_id = $2 AND decision = 'en_examen'",
        )
        .bind(etab)
        .bind(demande)
        .bind(acteur)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let pieces = detruire_pieces(&mut tx, etab, demande, acteur, role).await?;
        journal(
            &mut tx,
            etab,
            demande,
            "refusee",
            Some(&format!("{pieces} pièce(s) d'identité détruite(s)")),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-167** — rouvre une demande close sur réclamation du demandeur. L'horloge **ne repart pas à
    /// zéro** : la réouverture est tracée, l'échéance d'origine reste la référence et le retard éventuel
    /// reste visible (on ne réécrit pas le passé). Direction et DPO (`droits.arbitrer`).
    pub async fn droits_rouvrir(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        motif: &str,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        let motif = texte(motif, 3, 1000).ok_or(invalide("motif de réouverture obligatoire"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        if ouverte(&etat) {
            return Err(EntDbError::ConflitDossier("cette demande n'est pas close"));
        }
        sqlx::query(
            "UPDATE rgp_demande
                SET etat = 'en_cours', close_le = NULL, close_par_compte_id = NULL,
                    reouvertures = reouvertures + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "rouverte",
            Some(&motif),
            Some(acteur),
            Some(role),
        )
        .await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    // ── Lectures ────────────────────────────────────────────────────────────────────────────

    /// Fiche d'une demande, pour le guichet (`droits.traiter`). Journal compris.
    pub async fn droits_demande(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
    ) -> Result<DemandeVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let vue = charger(&mut tx, etab, demande, jour).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// **RGP-169** — tableau des demandes : échéances, retards, par gestionnaire. **Métadonnées seulement**
    /// (R-RG32) : aucun nom de personne concernée, aucun contenu. `closes_aussi` ajoute les demandes closes
    /// des douze derniers mois.
    pub async fn droits_tableau(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        closes_aussi: bool,
    ) -> Result<TableauDemandes, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COLONNES} FROM rgp_demande
              WHERE etablissement_id = $1
                AND (etat NOT IN ('close', 'refusee')
                     OR ($2 AND close_le >= now() - interval '12 months'))
              ORDER BY (etat IN ('close', 'refusee')), echeance_le, numero"
        );
        let rows = sqlx::query(&sql)
            .bind(etab)
            .bind(closes_aussi)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut lignes = Vec::with_capacity(rows.len());
        let mut t = TableauDemandes {
            jour,
            lignes: Vec::new(),
            ouvertes: 0,
            en_retard: 0,
            echeance_proche: 0,
            identite_a_verifier: 0,
            a_qualifier: 0,
            closes: 0,
        };
        for r in &rows {
            let b = brut(r);
            let droits: Vec<String> = sqlx::query_scalar(
                "SELECT droit FROM rgp_demande_droit
                  WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY droit",
            )
            .bind(etab)
            .bind(b.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let eff = echeance_effective(&b);
            let restants = (eff - jour).num_days();
            let est_ouverte = ouverte(&b.etat);
            let retard = est_ouverte && restants < 0;
            if est_ouverte {
                t.ouvertes += 1;
                if retard {
                    t.en_retard += 1;
                } else if restants <= ALERTE_ECHEANCE_JOURS[0] {
                    t.echeance_proche += 1;
                }
                if b.etat == "identite_a_verifier" {
                    t.identite_a_verifier += 1;
                }
                if b.qualification == "a_qualifier" {
                    t.a_qualifier += 1;
                }
            } else {
                t.closes += 1;
            }
            lignes.push(DemandeLigne {
                id: b.id,
                reference: reference(b.annee, b.numero),
                canal: b.canal.clone(),
                recue_le: b.recue_le,
                echeance_effective: eff,
                jours_restants: restants,
                en_retard: retard,
                etat: b.etat.clone(),
                qualification: b.qualification.clone(),
                qualite: b.qualite.clone(),
                sujet_type: b.sujet_type.clone(),
                gestionnaire_personne_id: b.gestionnaire_personne_id,
                droits,
                saisie_tardive: (b.cree_le.date_naive() - b.recue_le).num_days()
                    > ECART_SAISIE_ALERTE_JOURS,
            });
        }
        t.lignes = lignes;
        tx.commit().await?;
        Ok(t)
    }

    /// **RGP-155** — suivi par le demandeur de **ses** demandes : états lisibles, jamais le détail interne
    /// (gestionnaire, notes, journal). Aucun droit d'établissement requis : le filtre est le compte.
    pub async fn droits_mes_demandes(
        &self,
        etab: Uuid,
        acteur: Uuid,
    ) -> Result<Vec<SuiviDemande>, EntDbError> {
        if self.acces_etat(etab, acteur).await?.is_none() {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COLONNES} FROM rgp_demande
              WHERE etablissement_id = $1 AND demandeur_compte_id = $2
              ORDER BY recue_le DESC, numero DESC"
        );
        let rows = sqlx::query(&sql)
            .bind(etab)
            .bind(acteur)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in &rows {
            let b = brut(r);
            let droits: Vec<String> = sqlx::query_scalar(
                "SELECT droit FROM rgp_demande_droit
                  WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY droit",
            )
            .bind(etab)
            .bind(b.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let eff = echeance_effective(&b);
            out.push(SuiviDemande {
                id: b.id,
                reference: reference(b.annee, b.numero),
                recue_le: b.recue_le,
                echeance_effective: eff,
                etat: b.etat.clone(),
                avancement: avancement(&b, eff),
                droits,
                prolongee_jusqu_au: b.prolongee_jusqu_au,
                prolongation_motif: b.prolongation_motif.clone(),
                close_le: b.close_le,
                reponse_resume: b.reponse_resume.clone(),
            });
        }
        tx.commit().await?;
        Ok(out)
    }

    /// **R-RG19** — demandes ouvertes qui portent sur ce sujet : le moteur de conservation (lot G6) les lit
    /// pour geler les sorts finaux de la personne. Appel **système**, sans acteur : aucune route ne
    /// l'expose.
    pub async fn demandes_ouvertes_pour_sujet(
        &self,
        etab: Uuid,
        sujet_type: &str,
        sujet_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM rgp_demande
              WHERE etablissement_id = $1 AND sujet_type = $2 AND sujet_id = $3
                AND etat NOT IN ('close', 'refusee')",
        )
        .bind(etab)
        .bind(sujet_type)
        .bind(sujet_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(ids)
    }

    /// Alertes avant échéance (RGP-169, S-544 : J-10 et J-3) : un avis par demande et par palier, aux
    /// personnes habilitées de l'établissement (`droits.traiter`) et au DPO quand il a un compte.
    /// Appel **système** (tâche de fond) : aucun acteur, aucune route. Idempotent — la clé de déduplication
    /// porte le palier, et le journal garde la trace de l'envoi.
    pub async fn droits_alerter_echeances(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COLONNES} FROM rgp_demande
              WHERE etablissement_id = $1 AND etat NOT IN ('close', 'refusee')"
        );
        let rows = sqlx::query(&sql)
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        // Destinataires : les comptes habilités de l'établissement. La liste vient du moteur de droits, pas
        // d'une liste figée de rôles ; elle est calculée **une fois**, pas par demande.
        let destinataires = comptes_habilites(self, &mut tx, etab, jour).await?;
        let mut envoyes = 0usize;
        for r in &rows {
            let b = brut(r);
            let eff = echeance_effective(&b);
            let restants = (eff - jour).num_days();
            let Some(palier) = ALERTE_ECHEANCE_JOURS
                .iter()
                .copied()
                .find(|p| *p == restants)
            else {
                continue;
            };
            if destinataires.is_empty() {
                break;
            }
            let reference = reference(b.annee, b.numero);
            let corps = format!(
                "La demande {reference} arrive à échéance le {} ({palier} jours). Qualification : {}.",
                eff.format("%d/%m/%Y"),
                b.qualification
            );
            for u in destinataires.iter().copied() {
                envoyes += notifier_personne(
                    &mut tx,
                    EnvoiAPlanifier {
                        etablissement_id: etab,
                        user_id: u,
                        kind: KIND_ECHEANCE_PROCHE,
                        titre: "Demande de droit à échéance",
                        corps: &corps,
                        eleve_id: None,
                        notification_id: None,
                        // Le socle 8 borne `source_type` : un avis émis par le guichet est un geste du système.
                        source_type: "systeme",
                        source_id: b.id,
                        cle_dedup: format!("rgpd:demande:{}:echeance:{palier}", b.id),
                        droit_requis: None,
                        regroupable: true,
                        canaux_voulus: &["email", "push"],
                    },
                    b.id,
                    Utc::now(),
                )
                .await?;
            }
            journal(
                &mut tx,
                etab,
                b.id,
                "echeance_alertee",
                Some(&format!("J-{palier}")),
                None,
                None,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(envoyes)
    }
}

// ---------------------------------------------------------------------------------------------
// Fonctions internes
// ---------------------------------------------------------------------------------------------

fn valider_droits(droits: &[&str]) -> Result<(), EntDbError> {
    if droits.is_empty() {
        return Err(invalide("au moins un droit exercé"));
    }
    if droits.len() > DROITS.len() {
        return Err(invalide("droit en double"));
    }
    for d in droits {
        if !DROITS.contains(d) {
            return Err(invalide("droit inconnu"));
        }
        if droits.iter().filter(|x| *x == d).count() > 1 {
            return Err(invalide("droit en double"));
        }
    }
    Ok(())
}

fn exiger_ouverte(etat: &str) -> Result<(), EntDbError> {
    if ouverte(etat) {
        Ok(())
    } else {
        Err(EntDbError::ConflitDossier(
            "cette demande est close : la rouvrir d'abord",
        ))
    }
}

async fn etat_courant(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
) -> Result<String, EntDbError> {
    sqlx::query_scalar(
        "SELECT etat FROM rgp_demande WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(demande)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or(EntDbError::Domain(DomainError::NotFound))
}

/// Le sujet existe-t-il dans **cet** établissement ? Un identifiant d'un autre établissement est
/// introuvable (404 neutre), jamais « refusé » : rien ne dit qu'il existe ailleurs.
async fn verifier_sujet(
    conn: &mut PgConnection,
    etab: Uuid,
    sujet_type: &str,
    sujet_id: Option<Uuid>,
    sujet_nom_saisi: Option<&str>,
) -> Result<(), EntDbError> {
    match (sujet_type, sujet_id) {
        ("inconnu", None) => {
            if sujet_nom_saisi.is_none() {
                return Err(invalide("nom de la personne concernée obligatoire"));
            }
            Ok(())
        }
        ("eleve", Some(id)) => exister(conn, etab, "ent_eleve", id).await,
        ("personne", Some(id)) | ("agent", Some(id)) => {
            exister(conn, etab, "ent_personne", id).await
        }
        _ => Err(invalide("personne concernée incohérente")),
    }
}

async fn exister(
    conn: &mut PgConnection,
    etab: Uuid,
    table: &'static str,
    id: Uuid,
) -> Result<(), EntDbError> {
    // `table` est un littéral de ce fichier, jamais une entrée : aucune interpolation de donnée.
    let sql =
        format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE etablissement_id = $1 AND id = $2)");
    let existe: bool = sqlx::query_scalar(&sql)
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    if existe {
        Ok(())
    } else {
        Err(EntDbError::Domain(DomainError::NotFound))
    }
}

/// Les qualités qui ne valent que **sur soi** (§9.3 : « pour elle-même »).
fn qualite_de_soi(q: &str) -> bool {
    matches!(
        q,
        "soi_meme" | "eleve_mineur" | "personnel" | "ancien_eleve"
    )
}

/// Pour une qualité « pour moi-même », rend le sujet **du compte** : son dossier d'élève s'il en a un,
/// sinon sa fiche personne. Quand l'appelant a fourni un sujet, il doit être **le sien** — sinon
/// [`EntDbError::Forbidden`]. Sans ce contrôle, un compte ouvrirait un dossier sur la personne de son choix.
///
/// Pour les autres qualités (`titulaire_autorite`), le sujet fourni est rendu tel quel : c'est le lien du
/// socle 4, vérifié par [`qualite_a_la_date`], qui décide.
async fn resoudre_sujet_de_soi<'a>(
    conn: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    qualite: &str,
    sujet_type: &'a str,
    sujet_id: Option<Uuid>,
) -> Result<(&'a str, Option<Uuid>), EntDbError> {
    if !qualite_de_soi(qualite) {
        return Ok((sujet_type, sujet_id));
    }
    // Le dossier d'élève rattaché au compte, s'il existe (élève, ancien élève devenu majeur).
    let mon_eleve: Option<Uuid> = sqlx::query_scalar(
        "SELECT eleve_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(acteur)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let ma_personne: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_personne
          WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(acteur)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    match sujet_id {
        // Rien de fourni : on résout. L'élève d'abord (son dossier scolaire porte le plus de données).
        None => match (mon_eleve, ma_personne) {
            (Some(e), _) => Ok(("eleve", Some(e))),
            (None, Some(p)) => Ok(("personne", Some(p))),
            // Un compte sans fiche dans cet établissement ne peut pas demander « ses » données ici : il
            // passe par le guichet, qui le rapprochera d'un dossier (ou constatera qu'il n'y en a pas).
            (None, None) => Err(EntDbError::Forbidden),
        },
        // Fourni : ce doit être soi, et rien d'autre.
        Some(s) if Some(s) == mon_eleve => Ok(("eleve", Some(s))),
        Some(s) if Some(s) == ma_personne => Ok(("personne", Some(s))),
        Some(_) => Err(EntDbError::Forbidden),
    }
}

/// Cohérence entre la qualité **invoquée** et ce que constate le socle 4 à la date (§9.3, CL27-03, CL27-04).
async fn exiger_coherence_qualite(
    conn: &mut PgConnection,
    etab: Uuid,
    qualite: &str,
    sujet_type: &str,
    sujet_id: Option<Uuid>,
    jour: NaiveDate,
    constatee: QualiteConstatee,
) -> Result<(), EntDbError> {
    match qualite {
        "titulaire_autorite" => {
            if sujet_type != "eleve" {
                return Err(invalide("un titulaire de l'autorité demande pour un élève"));
            }
            if matches!(constatee, QualiteConstatee::Absente) {
                return Err(EntDbError::Forbidden);
            }
            // Élève majeur : il exerce seul ses droits (§4.2, §9.3).
            if let Some(eleve) = sujet_id {
                if eleve_majeur_le(conn, etab, eleve, jour).await? == Some(true) {
                    return Err(EntDbError::ConflitDossier(
                        "l'élève est majeur : il exerce seul ses droits",
                    ));
                }
            }
            Ok(())
        }
        "eleve_mineur" => {
            let Some(eleve) = sujet_id else {
                return Err(invalide("élève concerné obligatoire"));
            };
            if sujet_type != "eleve" {
                return Err(invalide("personne concernée incohérente"));
            }
            match eleve_majeur_le(conn, etab, eleve, jour).await? {
                Some(true) => Err(invalide("cet élève est majeur : qualité « soi-même »")),
                Some(false) => Ok(()),
                // Date de naissance inconnue ou contestée : on reçoit la demande (la minorité est le cas
                // prudent), la qualité sera tranchée à la vérification.
                None => Ok(()),
            }
        }
        // Un élève majeur, un personnel ou un ancien élève demande pour lui-même : le sujet a déjà été
        // ramené à son propre dossier par `resoudre_sujet_de_soi` (aucun lien familial à constater). La
        // portée réelle des données remises est vérifiée à l'assemblage de l'extrait (lot G3b).
        "soi_meme" | "personnel" | "ancien_eleve" => Ok(()),
        _ => Err(invalide("qualité non acceptée pour un dépôt en ligne")),
    }
}

/// Comptes de l'établissement qui portent `droits.traiter` (destinataires des alertes d'échéance). Les
/// droits sont calculés par le **moteur de droits** (modèle du profil + rôles attribués + délégations), pas
/// par une liste de rôles figée : un établissement qui confie le guichet à un rôle maison est servi aussi.
/// Seuls les personnels aux accès ouverts sont interrogés (un accès suspendu ou fermé n'est pas alerté).
async fn comptes_habilites(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let membres: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT m.user_id, m.role FROM ent_membership m
          WHERE m.etablissement_id = $1
            AND m.role NOT IN ('parent', 'eleve')
            AND COALESCE(m.etat, 'ouvert') = 'ouvert'
            AND (m.date_fin IS NULL OR m.date_fin >= $2)",
    )
    .bind(etab)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut out = Vec::new();
    for (u, role) in membres {
        let droits = ent
            .droits_effectifs_conn(&mut *conn, etab, u, &role)
            .await?;
        if droits.iter().any(|d| d.action == Action::DroitsTraiter) {
            out.push(u);
        }
    }
    Ok(out)
}

/// **R-RG16** — détruit les pièces d'identité encore détenues par la demande. Idempotent : une pièce déjà
/// détruite n'est pas retouchée. Rend le nombre de pièces détruites.
async fn detruire_pieces(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    acteur: Uuid,
    role: &str,
) -> Result<usize, EntDbError> {
    let pieces: Vec<Uuid> = sqlx::query_scalar(
        "SELECT piece_document_id FROM rgp_demande_identite
          WHERE etablissement_id = $1 AND demande_id = $2 AND piece_document_id IS NOT NULL",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if pieces.is_empty() {
        return Ok(0);
    }
    // La référence disparaît ici ; la destruction du document lui-même appartient au socle 7 (coffre), qui
    // est le propriétaire (R-RG3 : le 27 ne supprime pas la ligne d'un autre socle). L'ordre est émis par
    // la même transaction, sous la forme d'une marque de suppression demandée.
    sqlx::query(
        "UPDATE rgp_demande_identite
            SET piece_document_id = NULL, piece_detruite_le = now()
          WHERE etablissement_id = $1 AND demande_id = $2 AND piece_document_id IS NOT NULL",
    )
    .bind(etab)
    .bind(demande)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journal(
        conn,
        etab,
        demande,
        "piece_detruite",
        Some(&format!("{} pièce(s)", pieces.len())),
        Some(acteur),
        Some(role),
    )
    .await?;
    Ok(pieces.len())
}

struct Insertion<'a> {
    canal: &'a str,
    recue_le: NaiveDate,
    jour: NaiveDate,
    demandeur_compte_id: Option<Uuid>,
    demandeur_personne_id: Option<Uuid>,
    demandeur_nom_saisi: Option<&'a str>,
    demandeur_contact: Option<&'a str>,
    qualite: &'a str,
    sujet_type: &'a str,
    sujet_id: Option<Uuid>,
    sujet_nom_saisi: Option<&'a str>,
    perimetre_texte: Option<&'a str>,
    perimetre_du: Option<NaiveDate>,
    perimetre_au: Option<NaiveDate>,
    droits: &'a [&'a str],
    etat: &'a str,
    /// Vérification posée dès le dépôt (session forte) : mode et qualité constatée.
    identite: Option<(&'a str, QualiteConstatee)>,
    cree_par: Option<Uuid>,
}

/// Insère la demande, son numéro (compteur de l'année de **réception**, pour que la référence suive l'année
/// du courrier) et ses droits. Tout vit dans la transaction de l'appelant : une annulation ne consomme pas
/// de numéro (« sans trou visible »).
async fn inserer(
    conn: &mut PgConnection,
    etab: Uuid,
    i: Insertion<'_>,
) -> Result<Uuid, EntDbError> {
    let annee = i.recue_le.year();
    let numero: i32 = sqlx::query_scalar("SELECT rgp_demande_numero_suivant($1, $2)")
        .bind(etab)
        .bind(annee)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let id = Uuid::new_v4();
    let echeance = echeance_legale(i.recue_le);
    sqlx::query(
        "INSERT INTO rgp_demande
             (id, etablissement_id, annee, numero, canal, recue_le, echeance_le, demandeur_compte_id,
              demandeur_personne_id, demandeur_nom_saisi, demandeur_contact, qualite, sujet_type, sujet_id,
              sujet_nom_saisi, perimetre_texte, perimetre_du, perimetre_au, etat, cree_par_compte_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
    )
    .bind(id)
    .bind(etab)
    .bind(annee)
    .bind(numero)
    .bind(i.canal)
    .bind(i.recue_le)
    .bind(echeance)
    .bind(i.demandeur_compte_id)
    .bind(i.demandeur_personne_id)
    .bind(i.demandeur_nom_saisi)
    .bind(i.demandeur_contact)
    .bind(i.qualite)
    .bind(i.sujet_type)
    .bind(i.sujet_id)
    .bind(i.sujet_nom_saisi)
    .bind(i.perimetre_texte)
    .bind(i.perimetre_du)
    .bind(i.perimetre_au)
    .bind(i.etat)
    .bind(i.cree_par)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    for d in i.droits {
        sqlx::query(
            "INSERT INTO rgp_demande_droit (id, etablissement_id, demande_id, droit)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(d)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    if let Some((mode, constatee)) = i.identite {
        sqlx::query(
            "INSERT INTO rgp_demande_identite
                 (id, etablissement_id, demande_id, mode, resultat, qualite_constatee,
                  verifie_par_compte_id, verifie_le)
             VALUES ($1, $2, $3, $4, 'verifiee', $5, $6, now())",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(mode)
        .bind(constatee.code())
        .bind(i.demandeur_compte_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        journal(
            conn,
            etab,
            id,
            "identite_verifiee",
            Some(&format!("{mode} — qualité {}", constatee.code())),
            i.demandeur_compte_id,
            None,
        )
        .await?;
        sqlx::query("UPDATE rgp_demande SET identite_fournie_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(i.jour)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    Ok(id)
}

async fn charger(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    jour: NaiveDate,
) -> Result<DemandeVue, EntDbError> {
    let sql = format!("SELECT {COLONNES} FROM rgp_demande WHERE etablissement_id = $1 AND id = $2");
    let r = sqlx::query(&sql)
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    let b = brut(&r);
    let droits = sqlx::query(
        "SELECT droit, decision, motif_code, motif_texte, decide_le FROM rgp_demande_droit
          WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY droit",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .iter()
    .map(|r| DroitVue {
        droit: r.get("droit"),
        decision: r.get("decision"),
        motif_code: r.get("motif_code"),
        motif_texte: r.get("motif_texte"),
        decide_le: r.get("decide_le"),
    })
    .collect();
    let identites: Vec<IdentiteVue> = sqlx::query(
        "SELECT mode, resultat, qualite_constatee, qualite_note, verifie_le,
                (piece_document_id IS NOT NULL) AS piece_detenue, piece_detruite_le
           FROM rgp_demande_identite
          WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY cree_le",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .iter()
    .map(|r| IdentiteVue {
        mode: r.get("mode"),
        resultat: r.get("resultat"),
        qualite_constatee: r.get("qualite_constatee"),
        qualite_note: r.get("qualite_note"),
        verifie_le: r.get("verifie_le"),
        piece_detenue: r.get("piece_detenue"),
        piece_detruite_le: r.get("piece_detruite_le"),
    })
    .collect();
    let journal: Vec<JournalVue> = sqlx::query(
        "SELECT evenement, detail, par_role, le FROM rgp_demande_journal
          WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY le, id",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .iter()
    .map(|r| JournalVue {
        evenement: r.get("evenement"),
        detail: r.get("detail"),
        par_role: r.get("par_role"),
        le: r.get("le"),
    })
    .collect();
    let eff = echeance_effective(&b);
    let restants = (eff - jour).num_days();
    Ok(DemandeVue {
        id: b.id,
        reference: reference(b.annee, b.numero),
        canal: b.canal.clone(),
        recue_le: b.recue_le,
        saisie_le: b.cree_le,
        saisie_tardive: (b.cree_le.date_naive() - b.recue_le).num_days()
            > ECART_SAISIE_ALERTE_JOURS,
        echeance_le: b.echeance_le,
        prolongee_jusqu_au: b.prolongee_jusqu_au,
        prolongation_motif: b.prolongation_motif.clone(),
        echeance_effective: eff,
        jours_restants: restants,
        en_retard: ouverte(&b.etat) && restants < 0,
        qualite: b.qualite.clone(),
        demandeur_nom: b.demandeur_nom_saisi.clone(),
        demandeur_a_un_compte: b.demandeur_compte_id.is_some(),
        sujet_type: b.sujet_type.clone(),
        sujet_id: b.sujet_id,
        sujet_nom_saisi: b.sujet_nom_saisi.clone(),
        qualification: b.qualification.clone(),
        qualification_motif: b.qualification_motif.clone(),
        perimetre_texte: b.perimetre_texte.clone(),
        perimetre_du: b.perimetre_du,
        perimetre_au: b.perimetre_au,
        etat: b.etat.clone(),
        gestionnaire_personne_id: b.gestionnaire_personne_id,
        identite_demandee_le: b.identite_demandee_le,
        identite_fournie_le: b.identite_fournie_le,
        identite_attente_jours: attente_jours(&b),
        reouvertures: b.reouvertures,
        close_le: b.close_le,
        reponse_resume: b.reponse_resume.clone(),
        refus_motif: b.refus_motif.clone(),
        titulaires_informes_le: b.titulaires_informes_le,
        titulaires_non_informes_motif: b.titulaires_non_informes_motif.clone(),
        piece_a_detruire: identites.iter().any(|i| i.piece_detenue),
        droits,
        identites,
        journal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echeance_un_mois_et_fin_de_mois() {
        // Cas simple.
        assert_eq!(
            echeance_legale(NaiveDate::from_ymd_opt(2027, 3, 3).unwrap()),
            NaiveDate::from_ymd_opt(2027, 4, 3).unwrap()
        );
        // 31 janvier : pas de 31 février, l'échéance sature au dernier jour (2028 est bissextile).
        assert_eq!(
            echeance_legale(NaiveDate::from_ymd_opt(2027, 1, 31).unwrap()),
            NaiveDate::from_ymd_opt(2027, 2, 28).unwrap()
        );
        assert_eq!(
            echeance_legale(NaiveDate::from_ymd_opt(2028, 1, 31).unwrap()),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
        );
        // Franchissement d'année.
        assert_eq!(
            echeance_legale(NaiveDate::from_ymd_opt(2027, 12, 15).unwrap()),
            NaiveDate::from_ymd_opt(2028, 1, 15).unwrap()
        );
    }

    #[test]
    fn prolongation_bornee_a_deux_mois() {
        let e = NaiveDate::from_ymd_opt(2027, 4, 3).unwrap();
        assert_eq!(
            prolongation_max(e),
            NaiveDate::from_ymd_opt(2027, 6, 3).unwrap()
        );
        // 31 décembre + 2 mois : saturation au 28/29 février.
        assert_eq!(
            prolongation_max(NaiveDate::from_ymd_opt(2027, 12, 31).unwrap()),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
        );
    }

    #[test]
    fn reference_lisible() {
        assert_eq!(reference(2027, 14), "2027-014");
        assert_eq!(reference(2027, 1), "2027-001");
        // Au-delà de 999, le numéro n'est pas tronqué.
        assert_eq!(reference(2027, 1234), "2027-1234");
    }

    #[test]
    fn droits_valides_et_doublons_refuses() {
        assert!(valider_droits(&["acces"]).is_ok());
        assert!(valider_droits(&["acces", "effacement"]).is_ok());
        assert!(valider_droits(&[]).is_err(), "au moins un droit");
        assert!(valider_droits(&["acces", "acces"]).is_err(), "doublon");
        assert!(valider_droits(&["inventé"]).is_err());
    }

    #[test]
    fn texte_borne_et_sans_caractere_de_controle() {
        assert_eq!(texte("  bonjour  ", 3, 20).as_deref(), Some("bonjour"));
        assert!(texte("ab", 3, 20).is_none(), "trop court");
        assert!(texte(&"a".repeat(21), 3, 20).is_none(), "trop long");
        assert!(texte("a\u{7}bc", 3, 20).is_none(), "caractère de contrôle");
        // Un retour à la ligne est accepté dans un texte libre (périmètre décrit par l'usager).
        assert_eq!(texte("abc\ndef", 3, 20).as_deref(), Some("abc\ndef"));
    }

    #[test]
    fn un_etat_clos_nest_plus_ouvert() {
        for e in [
            "recue",
            "identite_a_verifier",
            "en_cours",
            "a_relire",
            "prete",
            "remise",
        ] {
            assert!(ouverte(e), "{e}");
        }
        for e in ["close", "refusee"] {
            assert!(!ouverte(e), "{e}");
            assert!(exiger_ouverte(e).is_err());
        }
    }

    /// La suspension est **comptée** mais ne repousse pas l'échéance (choix prudent, §9.2).
    #[test]
    fn attente_identite_comptee_sans_repousser_lecheance() {
        let mut b = brut_test();
        b.identite_demandee_le = Some(NaiveDate::from_ymd_opt(2027, 3, 5).unwrap());
        b.identite_fournie_le = Some(NaiveDate::from_ymd_opt(2027, 3, 12).unwrap());
        assert_eq!(attente_jours(&b), 7);
        const { assert!(!IDENTITE_SUSPEND_DELAI) }; // défaut prudent : le délai court
        assert_eq!(echeance_effective(&b), b.echeance_le);
        // Une prolongation, elle, déplace bien l'échéance.
        b.prolongee_jusqu_au = Some(NaiveDate::from_ymd_opt(2027, 6, 3).unwrap());
        assert_eq!(
            echeance_effective(&b),
            NaiveDate::from_ymd_opt(2027, 6, 3).unwrap()
        );
    }

    #[test]
    fn attente_sans_demande_est_nulle() {
        let b = brut_test();
        assert_eq!(attente_jours(&b), 0);
    }

    #[test]
    fn avancement_lisible_par_le_demandeur() {
        let mut b = brut_test();
        b.etat = "en_cours".into();
        let a = avancement(&b, b.echeance_le);
        assert!(a.contains("03/03/2027"), "{a}");
        assert!(a.contains("03/04/2027"), "{a}");
        assert!(a.contains("instruction"), "{a}");
    }

    #[test]
    fn grille_et_modeles_relus() {
        assert!(GRILLE_VERIFICATION.iter().any(|e| e.obligatoire));
        for e in GRILLE_VERIFICATION {
            assert!(!e.libelle.is_empty() && !e.geste.is_empty(), "{}", e.code);
        }
        let mut codes: Vec<&str> = MODELES_REPONSE.iter().map(|m| m.code).collect();
        codes.sort_unstable();
        let avant = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), avant, "code de modèle en double");
        for m in MODELES_REPONSE {
            assert!(m.texte.len() > 40, "{} : texte trop court", m.code);
            if let Some(d) = m.droit {
                assert!(DROITS.contains(&d), "{} : droit inconnu", m.code);
            }
        }
    }

    fn brut_test() -> Brut {
        Brut {
            id: Uuid::nil(),
            annee: 2027,
            numero: 1,
            canal: "en_ligne".into(),
            recue_le: NaiveDate::from_ymd_opt(2027, 3, 3).unwrap(),
            echeance_le: NaiveDate::from_ymd_opt(2027, 4, 3).unwrap(),
            prolongee_jusqu_au: None,
            prolongation_motif: None,
            demandeur_compte_id: None,
            demandeur_nom_saisi: None,
            qualite: "soi_meme".into(),
            sujet_type: "eleve".into(),
            sujet_id: None,
            sujet_nom_saisi: None,
            qualification: "a_qualifier".into(),
            qualification_motif: None,
            perimetre_texte: None,
            perimetre_du: None,
            perimetre_au: None,
            etat: "recue".into(),
            gestionnaire_personne_id: None,
            identite_demandee_le: None,
            identite_fournie_le: None,
            reouvertures: 0,
            close_le: None,
            reponse_resume: None,
            refus_motif: None,
            titulaires_informes_le: None,
            titulaires_non_informes_motif: None,
            cree_le: DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
        }
    }
}
