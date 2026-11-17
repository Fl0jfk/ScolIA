//! Socle 18, lot FO1 — accords, autorisations et consentements : types, calcul du verdict (pur) et port de signature
//! (`docs/socles/18_FORMULAIRES.md` §6–§10, §27).
//!
//! **Un accord est un fait daté**, donné par une personne habilitée à la date, sur une **version** de texte précise, pour des
//! **usages** précis, révocable selon sa nature. Le port [`PgEnt::accords_verifier`] répond à « a-t-on le droit, ici,
//! maintenant, pour cette personne ? » ; **aucun verdict `autorise` par défaut** : une erreur ou un cas inconnu donne
//! `inconnu`, et l'appelant refuse l'usage (R-FC3).
//!
//! Ce module est **pur** (aucun accès à la base) : le calcul du verdict est testé sans base. Les requêtes sont dans
//! `accords_service.rs` (gestion, décisions, « mes accords », reprise) et `accords_verifier.rs` (port).
//!
//! **Signature** : les autorisations et consentements passeront par la brique de signature du socle 7 (lot D6, à venir ;
//! idées I-013 et I-014). Le socle 18 n'écrit **aucune signature** : il appelle le port [`PortSignature`] ; tant qu'il est
//! absent ([`SignatureAbsente`]), la décision porte le canal `en_ligne` sans identifiant de preuve, et une preuve **S2**
//! (acte non usuel) est refusée plutôt que simulée.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::service::EntDbError;

// ---------------------------------------------------------------------------
// Vocabulaire fermé
// ---------------------------------------------------------------------------

pub const NATURES: [&str; 4] = ["information", "collecte", "autorisation", "consentement"];
pub const REGLES: [&str; 5] = ["un", "tous", "eleve_seul", "eleve_et_titulaires", "soi"];
pub const PREUVES: [&str; 3] = ["S0", "S1", "S2"];
pub const VALIDITES: [&str; 5] = ["annee_scolaire", "jusqu_a", "evenement", "scolarite", "sans_terme"];
pub const DROITS_LIEN: [&str; 4] = ["signer_quotidien", "signer_documents", "repondre_formulaires", "autoriser_sorties"];
/// Usage technique des décisions reprises de l'ancien module (`ent_formulaire`, 193) : l'ancien texte n'avait pas d'usages.
pub const USAGE_REPRISE: &str = "reprise_ancien_module";

/// Événements métier (identifiants seulement, jamais un texte libre).
pub mod evenements {
    pub const OBJET_ACTIVE: &str = "autorisations.objet_active";
    pub const OBJET_DESACTIVE: &str = "autorisations.objet_desactive";
    pub const VERSION_PUBLIEE: &str = "autorisations.version_publiee";
    pub const DECISION: &str = "autorisations.decision";
    pub const RETIRE: &str = "autorisations.retire";
    pub const ETAT_MODIFIE: &str = "autorisations.etat_modifie";
    pub const DESACCORD: &str = "autorisations.desaccord";
    /// Désaccord déclaré, publié pour le socle 4 (`eleve.desaccord_declare`, `04_ADMINISTRATIF.md` §983).
    pub const DESACCORD_DECLARE: &str = "eleve.desaccord_declare";
    pub const DESACCORD_LEVE: &str = "eleve.desaccord_leve";
}

/// Verdict du port `verifier` (§5.2). Codes stables ; jamais de booléen par défaut vrai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Autorise,
    Refuse,
    EnAttente,
    Desaccord,
    Expire,
    ARenouveler,
    Protege,
    Inconnu,
    NonRequis,
}

impl Verdict {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Autorise => "autorise",
            Self::Refuse => "refuse",
            Self::EnAttente => "en_attente",
            Self::Desaccord => "desaccord",
            Self::Expire => "expire",
            Self::ARenouveler => "a_renouveler",
            Self::Protege => "protege",
            Self::Inconnu => "inconnu",
            Self::NonRequis => "non_requis",
        }
    }

    /// L'usage est-il permis ? Seuls `autorise` et `non_requis` (usage de mission) le permettent : tout le reste
    /// — attente, désaccord, expiration, renouvellement, protection, inconnu — est un refus (R-FC3, R-FC18).
    pub const fn permet(self) -> bool {
        matches!(self, Self::Autorise | Self::NonRequis)
    }
}

/// Qualités qui décident **pour** l'élève, à la place ou avec les titulaires de l'autorité parentale.
fn est_qualite_titulaire(q: &str) -> bool {
    matches!(q, "titulaire" | "tuteur" | "service_gardien" | "delegataire_image" | "curateur_assiste")
}

/// Une décision retenue pour un usage : la **dernière** de ce décideur, encore qualifiée à la date.
#[derive(Debug, Clone, Copy)]
pub struct DecisionLue<'a> {
    pub qualite: &'a str,
    pub accorde: bool,
    /// Accord hors validité (année scolaire passée, événement terminé, date dépassée).
    pub expiree: bool,
    /// Reprise sans preuve : un accord de ce type ne vaut jamais autorisation (R-FC43).
    pub sans_preuve: bool,
}

/// Entrée du calcul pour un sujet, un objet et un usage, à une date.
#[derive(Debug, Clone)]
pub struct EntreeVerdict<'a> {
    pub nature: &'a str,
    pub regle: &'a str,
    /// Âge à partir duquel l'élève donne son avis (consentement) ; `None` : pas d'avis.
    pub avis_des_ans: Option<i16>,
    pub age_eleve: Option<i32>,
    pub usage_mission: bool,
    pub usage_image: bool,
    pub protege: bool,
    pub decide_seul: bool,
    /// Titulaires qui doivent accorder (règle `tous`), avec ou sans compte.
    pub titulaires_attendus: usize,
    /// Le décideur a décidé sur d'autres usages de l'objet mais pas sur celui-ci (usage ajouté depuis).
    pub a_renouveler: bool,
    pub decisions: Vec<DecisionLue<'a>>,
}

/// Calcul du verdict (§9.1). Pur et déterministe.
pub fn calculer_verdict(e: &EntreeVerdict<'_>) -> Verdict {
    // 1-2. Mission : pas d'accord requis ; un élève protégé n'apparaît jamais dans une image publiée.
    if e.protege && e.usage_image && !e.usage_mission {
        return Verdict::Protege;
    }
    if e.usage_mission {
        return if e.protege && e.usage_image { Verdict::Protege } else { Verdict::NonRequis };
    }
    let d = &e.decisions;
    let refus = |f: &dyn Fn(&str) -> bool| d.iter().any(|x| f(x.qualite) && !x.accorde);
    let accords_valides = |f: &dyn Fn(&str) -> bool| {
        d.iter().filter(|x| f(x.qualite) && x.accorde && !x.expiree && !x.sans_preuve).count()
    };
    let perimes = |f: &dyn Fn(&str) -> bool| d.iter().filter(|x| f(x.qualite) && x.accorde && x.expiree).count();
    let sans_preuve = |f: &dyn Fn(&str) -> bool| d.iter().any(|x| f(x.qualite) && x.accorde && !x.expiree && x.sans_preuve);

    // Sujet adulte (personnel, parent, élève majeur) : la personne décide pour elle-même.
    let seul = |q: &str| matches!(q, "personne_soi" | "eleve_majeur");
    if e.regle == "soi" || e.regle == "eleve_seul" || e.decide_seul {
        // Un élève qui décide seul : ses parents ne comptent plus (R-FC12) ; une règle `eleve_seul` ne regarde que lui.
        let propre = |q: &str| seul(q) || (e.regle == "eleve_seul" && matches!(q, "eleve_cosignataire" | "eleve_avis"));
        if refus(&propre) {
            return Verdict::Refuse;
        }
        if accords_valides(&propre) > 0 {
            return Verdict::Autorise;
        }
        if sans_preuve(&propre) {
            return Verdict::Inconnu;
        }
        if perimes(&propre) > 0 {
            return Verdict::Expire;
        }
        return if e.a_renouveler { Verdict::ARenouveler } else { Verdict::EnAttente };
    }

    // Mineur (ou majeur qui ne décide pas seul) : titulaires + avis de l'enfant + co-signature.
    // Avis de l'enfant (R-FC8, CK-04) : à partir de l'âge fixé, son refus l'emporte ; son accord ne suffit pas.
    let avis_actif = e.nature == "consentement"
        && e.avis_des_ans.is_some_and(|a| e.age_eleve.is_some_and(|age| age >= i32::from(a)));
    if avis_actif && refus(&|q| q == "eleve_avis") {
        return Verdict::Refuse;
    }
    let cosign = e.regle == "eleve_et_titulaires";
    if cosign && refus(&|q| q == "eleve_cosignataire") {
        return Verdict::Refuse;
    }
    let tit = |q: &str| est_qualite_titulaire(q);
    let (nb_ok, nb_refus, nb_perimes) = (accords_valides(&tit), d.iter().filter(|x| tit(x.qualite) && !x.accorde).count(), perimes(&tit));
    let attente = if e.a_renouveler { Verdict::ARenouveler } else { Verdict::EnAttente };
    match e.regle {
        "tous" => {
            if nb_refus > 0 {
                Verdict::Refuse
            } else if nb_ok >= e.titulaires_attendus.max(1) {
                Verdict::Autorise
            } else if nb_ok > 0 {
                Verdict::EnAttente
            } else if sans_preuve(&tit) {
                Verdict::Inconnu
            } else if nb_perimes > 0 {
                Verdict::Expire
            } else {
                attente
            }
        }
        // `un` et `eleve_et_titulaires` : un accord suffit ; un refus connu sans accord refuse ; refus et accord se
        // contredisent : désaccord, la présomption tombe (R-FC7).
        _ => {
            if nb_refus > 0 {
                return if nb_ok > 0 { Verdict::Desaccord } else { Verdict::Refuse };
            }
            if nb_ok > 0 {
                return if cosign && accords_valides(&|q| q == "eleve_cosignataire") == 0 {
                    Verdict::EnAttente
                } else {
                    Verdict::Autorise
                };
            }
            if sans_preuve(&tit) {
                Verdict::Inconnu
            } else if nb_perimes > 0 {
                Verdict::Expire
            } else {
                attente
            }
        }
    }
}

/// Règle d'accord : on peut monter, jamais baisser sous celle du catalogue pour la même famille d'objet (R-FC6).
pub fn regle_ne_baisse_pas(catalogue: &str, choisie: &str) -> bool {
    match catalogue {
        "un" => matches!(choisie, "un" | "tous" | "eleve_et_titulaires"),
        "tous" => matches!(choisie, "tous" | "eleve_et_titulaires"),
        autre => choisie == autre,
    }
}

/// Une nature « consentement » ne porte jamais sur un usage nécessaire à la mission (R-FC1), et une « collecte » ne
/// demande jamais une donnée facultative : ici, le contrôle porte sur la déclaration des usages.
pub fn controle_nature_usages(nature: &str, usages_mission: usize, usages_total: usize) -> Result<(), &'static str> {
    if nature == "consentement" && usages_total > 0 && usages_mission == usages_total {
        return Err(
            "ceci relève de la mission de l'établissement : utilisez une collecte ou une information, pas un consentement",
        );
    }
    if nature == "collecte" && usages_mission < usages_total {
        return Err("une collecte ne porte que sur des usages de mission : un usage facultatif relève du consentement");
    }
    Ok(())
}

/// Un usage d'**image** (la protection d'un élève s'applique à tout usage d'image hors personnels, R-FC11).
pub fn usage_d_image(objet_code: &str, usage_code: &str) -> bool {
    matches!(objet_code, "droit_image" | "publication_productions" | "mention_nominative" | "voix")
        && usage_code != "interne_personnels"
}

/// Objets dont le catalogue exige l'accord de **tous** les titulaires (FD-17 : « un seul accord suffit toujours » est faux
/// pour l'image, la voix, la mention nominative, l'enregistrement d'un cours et le voyage à l'étranger).
pub const OBJETS_EXIGEANT_TOUS: [&str; 5] =
    ["droit_image", "voix", "mention_nominative", "enregistrement_visio", "voyage_etranger"];

/// Un objet exige la règle `tous` s'il est du catalogue ci-dessus **ou** s'il porte sur l'image d'un élève (famille de
/// désaccord `image`, y compris un objet propre à l'établissement).
pub fn objet_exige_tous(code: &str, desaccord_objet: Option<&str>) -> bool {
    OBJETS_EXIGEANT_TOUS.contains(&code) || desaccord_objet == Some("image")
}

/// Règle **effective** du calcul : un objet qui exige `tous` ne se calcule jamais avec `un`, même si une ligne ancienne (reprise,
/// objet créé avant la correction) porte encore `un` en base. Défense en profondeur : la création et le relèvement de règle
/// refusent déjà cet état.
pub fn regle_effective<'a>(regle: &'a str, code: &str, desaccord_objet: Option<&str>, sujet_type: &str) -> &'a str {
    if sujet_type == "eleve" && regle == "un" && objet_exige_tous(code, desaccord_objet) {
        "tous"
    } else {
        regle
    }
}

/// Validité à une date : `true` si la décision prise à `decide_le` n'est plus valable le `jour`.
pub fn decision_expiree(
    validite: &str,
    decide_le: NaiveDate,
    jour: NaiveDate,
    valide_jusqu_au: Option<NaiveDate>,
    evenement_fin: Option<DateTime<Utc>>,
    maintenant: DateTime<Utc>,
) -> bool {
    use chrono::Datelike;
    // L'année scolaire commence le 1er septembre (CK-36 : expiré au 31 août).
    let annee = |d: NaiveDate| if d.month() >= 9 { d.year() } else { d.year() - 1 };
    match validite {
        "annee_scolaire" => annee(decide_le) != annee(jour),
        "jusqu_a" => valide_jusqu_au.is_some_and(|fin| jour > fin),
        "evenement" => evenement_fin.is_some_and(|fin| maintenant > fin),
        _ => false,
    }
}

/// Encode des éléments avec leur longueur (`<n>:<texte>`) avant de hacher : aucune collision par séparateur ambigu
/// (FD-18, R-FC45).
pub fn empreinte_encodee(elements: &[&str]) -> String {
    let mut s = String::new();
    for e in elements {
        s.push_str(&e.chars().count().to_string());
        s.push(':');
        s.push_str(e);
        s.push(';');
    }
    crate::campagnes::sha256_hex(&s)
}

// ---------------------------------------------------------------------------
// Port de signature (brique du socle 7, lot D6)
// ---------------------------------------------------------------------------

/// Ce qu'on demande à la brique de signature du socle 7.
#[derive(Debug, Clone)]
pub struct DemandeSignature<'a> {
    pub etablissement_id: Uuid,
    pub objet_code: &'a str,
    pub version_numero: i32,
    /// Empreinte de la version signée (texte et usages couverts).
    pub empreinte_version: &'a str,
    pub decideur_user_id: Uuid,
    pub qualite: &'a str,
    /// Niveau exigé par l'objet : `S0`, `S1` ou `S2`.
    pub niveau: &'a str,
}

/// Port de signature : renvoie l'identifiant de la preuve (socle 7) ou `None` si aucune brique n'est branchée.
pub trait PortSignature: Send + Sync {
    fn signer<'a>(
        &'a self,
        demande: DemandeSignature<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Uuid>, EntDbError>> + Send + 'a>>;
}

/// Adapteur par défaut tant que le lot D6 du socle 7 n'est pas livré : aucune preuve, jamais de preuve simulée.
pub struct SignatureAbsente;

impl PortSignature for SignatureAbsente {
    fn signer<'a>(
        &'a self,
        demande: DemandeSignature<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Uuid>, EntDbError>> + Send + 'a>> {
        Box::pin(async move {
            if demande.niveau == "S2" {
                return Err(EntDbError::ConflitDossier(
                    "preuve S2 requise : la brique de signature (socle 7, lot D6) n'est pas encore en service",
                ));
            }
            Ok(None)
        })
    }
}

// ---------------------------------------------------------------------------
// Vues
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ObjetAccord {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub nature: String,
    pub regle: String,
    pub avis_eleve_des_ans: Option<i16>,
    pub droit_lien_code: Option<String>,
    pub preuve_min: String,
    pub validite: String,
    pub sujet_type: String,
    pub catalogue_code: Option<String>,
    pub actif: bool,
    /// Numéro de la version publiée en cours.
    pub version_publiee: Option<i32>,
    /// Le texte en cours n'a pas été relu : « texte provisoire », usage réel déconseillé (R-FC72).
    pub texte_provisoire: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UsageAccord {
    pub code: String,
    pub libelle: String,
    pub mission: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionAccord {
    pub id: Uuid,
    pub numero: i32,
    pub texte: String,
    pub type_changement: Option<String>,
    pub publiee_le: Option<DateTime<Utc>>,
    pub publiee_par: Option<Uuid>,
    pub texte_relu_le: Option<DateTime<Utc>>,
    /// Codes des usages couverts.
    pub usages: Vec<String>,
}

/// Entrée du catalogue de référence, avec son état d'activation dans l'établissement.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EntreeCatalogue {
    pub code: String,
    pub libelle: String,
    pub nature: String,
    pub regle_min: String,
    pub sujet_type: String,
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelUsage {
    pub code: String,
    pub libelle: String,
    pub mission: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelObjet {
    pub code: String,
    pub libelle: String,
    pub nature: String,
    pub regle: String,
    pub avis_eleve_des_ans: Option<i16>,
    pub droit_lien_code: Option<String>,
    pub preuve_min: String,
    pub validite: String,
    pub valide_jusqu_au: Option<NaiveDate>,
    /// Fin de l'événement (validité `evenement`) : au-delà, les accords expirent et un retrait devient une demande.
    pub evenement_fin: Option<DateTime<Utc>>,
    pub sujet_type: String,
    /// Famille de désaccord du socle 4 (`image`, `sortie`, `general`) : un objet d'**image** exige la règle `tous` (FD-17).
    #[serde(default)]
    pub desaccord_objet: Option<String>,
    pub usages: Vec<NouvelUsage>,
    pub texte: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedactionVersion {
    pub texte: String,
    /// Codes des usages couverts par la version (parmi ceux de l'objet, ou ajoutés ci-dessous).
    pub usages: Vec<String>,
    pub nouveaux_usages: Vec<NouvelUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChoixUsage {
    pub code: String,
    pub accorde: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecisionSaisie {
    pub objet_code: String,
    /// Sujet élève ; `None` pour un objet dont le sujet est la personne elle-même.
    pub eleve_id: Option<Uuid>,
    pub usages: Vec<ChoixUsage>,
}

/// Ce que le décideur reçoit : **sa** décision seulement, jamais celle d'un autre (R-FC16).
#[derive(Debug, Clone, Serialize)]
pub struct MaDecision {
    pub decision_id: Uuid,
    pub decision: String,
    pub qualite: String,
    pub version: i32,
    pub usages: Vec<(String, bool)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonUsage {
    pub code: String,
    pub libelle: String,
    /// Ma dernière décision sur cet usage (`Some(true)` accordé, `Some(false)` refusé ou retiré).
    pub ma_decision: Option<bool>,
    pub decide_le: Option<DateTime<Utc>>,
    /// J'ai décidé sur cet objet, mais pas sur cet usage (usage ajouté depuis) : à renouveler.
    pub a_renouveler: bool,
    pub preuve: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonAccord {
    pub objet_code: String,
    pub libelle: String,
    pub nature: String,
    pub version_courante: i32,
    pub texte: String,
    pub texte_provisoire: bool,
    pub usages: Vec<MonUsage>,
    /// Qualité sous laquelle je décide pour ce sujet.
    pub qualite: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MesAccordsSujet {
    pub eleve_id: Option<Uuid>,
    pub prenom: String,
    pub accords: Vec<MonAccord>,
}

// -- Port `verifier` ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sujet {
    Eleve(Uuid),
    Personne(Uuid),
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemandeVerification {
    pub objet_code: String,
    pub usage_code: String,
    pub sujets: Vec<Sujet>,
    /// Jour de l'établissement ; défaut : aujourd'hui.
    pub jour: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerdictSujet {
    pub sujet: Sujet,
    pub verdict: Verdict,
    /// Raison codée, rendue aux seuls habilités à gérer (`autorisations.gerer`).
    pub raison: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerdictEleve {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DecisionDetail {
    pub decision_id: Uuid,
    pub decideur_user_id: Uuid,
    pub qualite: String,
    pub decision: String,
    pub canal: String,
    pub version: i32,
    pub decide_le: DateTime<Utc>,
    pub preuve_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BilanReprise {
    pub objets: usize,
    pub versions: usize,
    pub decisions: usize,
    pub deja_reprises: usize,
    pub ignorees: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(qualite: &'static str, accorde: bool) -> DecisionLue<'static> {
        DecisionLue { qualite, accorde, expiree: false, sans_preuve: false }
    }

    fn base(regle: &'static str, decisions: Vec<DecisionLue<'static>>) -> EntreeVerdict<'static> {
        EntreeVerdict {
            nature: "consentement",
            regle,
            avis_des_ans: Some(11),
            age_eleve: Some(13),
            usage_mission: false,
            usage_image: true,
            protege: false,
            decide_seul: false,
            titulaires_attendus: 2,
            a_renouveler: false,
            decisions,
        }
    }

    #[test]
    fn aucun_verdict_autorise_par_defaut() {
        // R-FC3 : sans aucune décision, quelle que soit la règle, rien n'est autorisé.
        for regle in REGLES {
            let v = calculer_verdict(&base(regle, vec![]));
            assert!(!v.permet(), "{regle} sans décision ne doit jamais permettre : {v:?}");
        }
        assert!(Verdict::Autorise.permet() && Verdict::NonRequis.permet());
        for v in [Verdict::Refuse, Verdict::EnAttente, Verdict::Desaccord, Verdict::Expire, Verdict::ARenouveler, Verdict::Protege, Verdict::Inconnu] {
            assert!(!v.permet(), "{v:?}");
        }
    }

    #[test]
    fn ck03_image_regle_tous_un_seul_accord_en_attente() {
        let v = calculer_verdict(&base("tous", vec![d("titulaire", true)]));
        assert_eq!(v, Verdict::EnAttente);
        let v = calculer_verdict(&base("tous", vec![d("titulaire", true), d("titulaire", true)]));
        assert_eq!(v, Verdict::Autorise);
    }

    #[test]
    fn ck04_les_deux_parents_acceptent_l_enfant_de_13_ans_refuse() {
        let v = calculer_verdict(&base("tous", vec![d("titulaire", true), d("titulaire", true), d("eleve_avis", false)]));
        assert_eq!(v, Verdict::Refuse, "l'avis de l'enfant prioritaire");
        // CK-05 : en dessous de l'âge, l'avis n'est pas demandé et ne compte pas.
        let mut e = base("tous", vec![d("titulaire", true), d("titulaire", true), d("eleve_avis", false)]);
        e.age_eleve = Some(9);
        assert_eq!(calculer_verdict(&e), Verdict::Autorise);
        // L'accord de l'enfant ne suffit pas sans les titulaires.
        assert_eq!(calculer_verdict(&base("tous", vec![d("eleve_avis", true)])), Verdict::EnAttente);
    }

    #[test]
    fn ck01_ck02_regle_un_accord_puis_refus_du_second_est_un_desaccord() {
        let mut e = base("un", vec![d("titulaire", true)]);
        e.nature = "autorisation";
        e.usage_image = false;
        assert_eq!(calculer_verdict(&e), Verdict::Autorise);
        e.decisions.push(d("titulaire", false));
        assert_eq!(calculer_verdict(&e), Verdict::Desaccord);
        e.decisions = vec![d("titulaire", false)];
        assert_eq!(calculer_verdict(&e), Verdict::Refuse);
    }

    #[test]
    fn mission_non_requis_mais_protege_prime_pour_une_image() {
        let mut e = base("tous", vec![]);
        e.usage_mission = true;
        assert_eq!(calculer_verdict(&e), Verdict::NonRequis);
        e.protege = true;
        assert_eq!(calculer_verdict(&e), Verdict::Protege, "R-FC11 : même un usage de mission d'image reste protégé");
        // Hors image, un élève protégé reste non concerné.
        e.usage_image = false;
        assert_eq!(calculer_verdict(&e), Verdict::NonRequis);
        // Protégé : quels que soient les accords.
        let mut p = base("tous", vec![d("titulaire", true), d("titulaire", true)]);
        p.protege = true;
        assert_eq!(calculer_verdict(&p), Verdict::Protege);
    }

    #[test]
    fn ck19_majeur_decide_seul_les_parents_cessent() {
        let mut e = base("tous", vec![d("titulaire", true), d("titulaire", true)]);
        e.decide_seul = true;
        assert_eq!(calculer_verdict(&e), Verdict::EnAttente, "les accords des parents ne comptent plus");
        e.decisions.push(d("eleve_majeur", true));
        assert_eq!(calculer_verdict(&e), Verdict::Autorise);
        e.decisions = vec![d("titulaire", false), d("eleve_majeur", true)];
        assert_eq!(calculer_verdict(&e), Verdict::Autorise, "le refus d'un parent ne pèse plus sur un majeur");
    }

    #[test]
    fn soi_et_eleve_seul() {
        let mut e = base("soi", vec![d("personne_soi", true)]);
        e.usage_image = false;
        assert_eq!(calculer_verdict(&e), Verdict::Autorise);
        e.decisions = vec![d("personne_soi", true), d("personne_soi", false)];
        assert_eq!(calculer_verdict(&e), Verdict::Refuse);
        let mut s = base("eleve_seul", vec![d("eleve_cosignataire", true)]);
        s.nature = "information";
        assert_eq!(calculer_verdict(&s), Verdict::Autorise);
        s.decisions = vec![d("titulaire", true)];
        assert_eq!(calculer_verdict(&s), Verdict::EnAttente, "un titulaire ne décide pas à la place de l'élève");
    }

    #[test]
    fn eleve_et_titulaires_exige_la_cosignature() {
        let mut e = base("eleve_et_titulaires", vec![d("titulaire", true)]);
        e.nature = "information";
        assert_eq!(calculer_verdict(&e), Verdict::EnAttente);
        e.decisions.push(d("eleve_cosignataire", true));
        assert_eq!(calculer_verdict(&e), Verdict::Autorise);
    }

    #[test]
    fn expire_a_renouveler_et_reprise_sans_preuve() {
        let mut e = base("tous", vec![DecisionLue { qualite: "titulaire", accorde: true, expiree: true, sans_preuve: false }]);
        e.titulaires_attendus = 1;
        assert_eq!(calculer_verdict(&e), Verdict::Expire);
        let mut r = base("un", vec![]);
        r.a_renouveler = true;
        assert_eq!(calculer_verdict(&r), Verdict::ARenouveler, "CK-30 : usage ajouté, accord antérieur sur les autres usages");
        let i = base("un", vec![DecisionLue { qualite: "titulaire", accorde: true, expiree: false, sans_preuve: true }]);
        assert_eq!(calculer_verdict(&i), Verdict::Inconnu, "R-FC43 : jamais accordé sans preuve");
    }

    #[test]
    fn un_refus_expire_ou_non_reste_un_refus() {
        let e = base("un", vec![DecisionLue { qualite: "titulaire", accorde: false, expiree: true, sans_preuve: false }]);
        assert_eq!(calculer_verdict(&e), Verdict::Refuse);
    }

    #[test]
    fn regles_ne_baissent_pas() {
        assert!(regle_ne_baisse_pas("tous", "tous") && regle_ne_baisse_pas("tous", "eleve_et_titulaires"));
        assert!(!regle_ne_baisse_pas("tous", "un"));
        assert!(regle_ne_baisse_pas("un", "tous") && !regle_ne_baisse_pas("eleve_et_titulaires", "un"));
        assert!(regle_ne_baisse_pas("soi", "soi") && !regle_ne_baisse_pas("soi", "un"));
    }

    #[test]
    fn nature_et_usages_de_mission() {
        assert!(controle_nature_usages("consentement", 1, 1).is_err(), "R-FC1 : un consentement ne porte pas sur la mission seule");
        assert!(controle_nature_usages("consentement", 1, 3).is_ok());
        assert!(controle_nature_usages("collecte", 1, 2).is_err());
        assert!(controle_nature_usages("collecte", 2, 2).is_ok());
        assert!(controle_nature_usages("autorisation", 0, 1).is_ok());
    }

    #[test]
    fn validite_annee_scolaire_au_31_aout() {
        let j = |a, m, d| NaiveDate::from_ymd_opt(a, m, d).unwrap();
        let maintenant = Utc::now();
        // Accord de juin 2027 : valable jusqu'au 31 août 2027 (CK-36), expiré au 1er septembre.
        assert!(!decision_expiree("annee_scolaire", j(2027, 6, 10), j(2027, 8, 31), None, None, maintenant));
        assert!(decision_expiree("annee_scolaire", j(2027, 6, 10), j(2027, 9, 1), None, None, maintenant));
        // Accord d'octobre : valable jusqu'à fin août suivant.
        assert!(!decision_expiree("annee_scolaire", j(2026, 10, 1), j(2027, 8, 31), None, None, maintenant));
        assert!(decision_expiree("jusqu_a", j(2026, 10, 1), j(2027, 1, 2), Some(j(2027, 1, 1)), None, maintenant));
        assert!(!decision_expiree("sans_terme", j(2020, 1, 1), j(2030, 1, 1), None, None, maintenant));
        let fin = maintenant - chrono::Duration::hours(1);
        assert!(decision_expiree("evenement", j(2026, 10, 1), j(2026, 10, 2), None, Some(fin), maintenant));
    }

    #[test]
    fn empreinte_sans_separateur_ambigu() {
        // FD-18 : ("a|b", "c") et ("a", "b|c") ne collisionnent plus.
        assert_ne!(empreinte_encodee(&["a|b", "c"]), empreinte_encodee(&["a", "b|c"]));
        assert_ne!(empreinte_encodee(&["ab", "c"]), empreinte_encodee(&["a", "bc"]));
        assert_eq!(empreinte_encodee(&["x", "y"]), empreinte_encodee(&["x", "y"]));
        assert_eq!(empreinte_encodee(&["x"]).len(), 64);
    }

    #[test]
    fn usages_d_image() {
        assert!(usage_d_image("droit_image", "site_public"));
        assert!(!usage_d_image("droit_image", "interne_personnels"));
        assert!(!usage_d_image("sortie_facultative", "participation"));
    }

    #[tokio::test]
    async fn la_signature_absente_ne_simule_jamais_une_preuve_s2() {
        let port = SignatureAbsente;
        let mut dem = DemandeSignature {
            etablissement_id: Uuid::nil(),
            objet_code: "voyage_etranger",
            version_numero: 1,
            empreinte_version: "x",
            decideur_user_id: Uuid::nil(),
            qualite: "titulaire",
            niveau: "S1",
        };
        assert_eq!(port.signer(dem.clone()).await.unwrap(), None);
        dem.niveau = "S2";
        assert!(port.signer(dem).await.is_err());
    }
}
