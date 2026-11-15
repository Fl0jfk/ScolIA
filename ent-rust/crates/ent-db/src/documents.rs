//! Socle 7 (« Documents, pièces et signatures »), lots D2 et D3 : **types, déclarations de natures, règles pures et
//! ports** de la brique `documents` (`docs/socles/07_DOCUMENTS.md` §5.3, §6, §8, §9, §14, §17 à §19).
//!
//! Ce module ne touche pas la base : il fixe le vocabulaire (sensibilité, domaine, série, états), les natures
//! déclarées par le code (le catalogue ne vit pas en base, §5.3), la fabrication des codes de vérification et des
//! numéros affichés, les contrôles de contraste de l'identité, et les **deux ports** par lesquels le moteur
//! s'appuie sur ce que d'autres briques fournissent :
//!
//! * [`RenduDocument`] — le moteur de rendu PDF (D-062 : moteur maison, écrit par le lot D1) ;
//! * [`CachetEtablissement`] — le cachet électronique de l'établissement (PAdES, §10.7).
//!
//! Aucun PDF n'est écrit ici : tant que D1 n'est pas sur `ENT`, l'émission réelle s'arrête sur
//! [`ErreurRendu::Indisponible`] (« le cachet indisponible bloque l'émission », §9.5, Q-D7) — jamais de document sans
//! moteur ni sans cachet. Les tests branchent des doublures.

use ent_auth::Action;

// ───────────────────────── Vocabulaire ─────────────────────────

/// Niveau de sensibilité d'un document (§17.2) : C0 public, C1 interne courant, C2 sensible, C3 très sensible
/// (santé, justice), C4 jamais servi par les écrans ordinaires (pièce d'un changement de prénom, motif scellé).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sensibilite {
    C0,
    C1,
    C2,
    C3,
    C4,
}

impl Sensibilite {
    pub fn code(self) -> &'static str {
        match self {
            Sensibilite::C0 => "C0",
            Sensibilite::C1 => "C1",
            Sensibilite::C2 => "C2",
            Sensibilite::C3 => "C3",
            Sensibilite::C4 => "C4",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "C0" => Sensibilite::C0,
            "C1" => Sensibilite::C1,
            "C2" => Sensibilite::C2,
            "C3" => Sensibilite::C3,
            "C4" => Sensibilite::C4,
            _ => return None,
        })
    }

    /// Les lectures de C2 et plus sont tracées (`ent_doc_consultation`, §17.2).
    pub fn trace_la_lecture(self) -> bool {
        self >= Sensibilite::C2
    }
}

/// Domaine d'un document (R-D6) : il décide QUEL droit ouvre la lecture (§18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domaine {
    Scolarite,
    Sante,
    Justice,
    Financier,
    Rh,
    Social,
    Disciplinaire,
}

impl Domaine {
    pub fn code(self) -> &'static str {
        match self {
            Domaine::Scolarite => "scolarite",
            Domaine::Sante => "sante",
            Domaine::Justice => "justice",
            Domaine::Financier => "financier",
            Domaine::Rh => "rh",
            Domaine::Social => "social",
            Domaine::Disciplinaire => "disciplinaire",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "scolarite" => Domaine::Scolarite,
            "sante" => Domaine::Sante,
            "justice" => Domaine::Justice,
            "financier" => Domaine::Financier,
            "rh" => Domaine::Rh,
            "social" => Domaine::Social,
            "disciplinaire" => Domaine::Disciplinaire,
            _ => return None,
        })
    }

    /// L'action du catalogue qui ouvre la lecture de ce domaine (§18.2, R-D6).
    pub fn action_lecture(self) -> Action {
        match self {
            Domaine::Scolarite | Domaine::Social | Domaine::Disciplinaire => Action::DocumentsLire,
            Domaine::Sante => Action::DocumentsLireSante,
            Domaine::Justice => Action::DocumentsRestreintsLire,
            Domaine::Financier => Action::DocumentsFinancierLire,
            Domaine::Rh => Action::DocumentsPersonnelsLire,
        }
    }
}

/// Série de numérotation, une par famille (§8.4). Format affiché `2026-SCO-000123`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Serie {
    Sco,
    Adm,
    Vs,
    Dis,
    Ped,
    Stg,
    Fin,
    Per,
    Etb,
    Cra,
    Crd,
}

impl Serie {
    pub fn code(self) -> &'static str {
        match self {
            Serie::Sco => "SCO",
            Serie::Adm => "ADM",
            Serie::Vs => "VS",
            Serie::Dis => "DIS",
            Serie::Ped => "PED",
            Serie::Stg => "STG",
            Serie::Fin => "FIN",
            Serie::Per => "PER",
            Serie::Etb => "ETB",
            Serie::Cra => "CRA",
            Serie::Crd => "CRD",
        }
    }

    /// Famille de nature qui choisit l'entité émettrice (`ent_doc_entite_famille`).
    pub fn famille(self) -> &'static str {
        match self {
            Serie::Sco => "scolarite",
            Serie::Adm => "admission",
            Serie::Vs => "vie_scolaire",
            Serie::Dis => "discipline",
            Serie::Ped => "pedagogie",
            Serie::Stg => "stages",
            Serie::Fin => "finances",
            Serie::Per => "personnels",
            Serie::Etb => "etablissement",
            Serie::Cra | Serie::Crd => "courrier",
        }
    }
}

/// Numéro affiché : `2026-SCO-000123`. Jamais accepté comme code de vérification (§9.7).
pub fn numero_affiche(annee: i32, serie: &str, numero: i32) -> String {
    format!("{annee}-{serie}-{numero:06}")
}

/// Mode de vérification publique déclaré par la nature (§9.7, Q-D15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verification {
    /// Établissement, nature, date, état, empreinte, désignation minimale du titulaire.
    Complete,
    /// « Document authentique émis par … le … », sans nature ni identité.
    AuthenticiteSeule,
    /// Pas de QR, pas de réponse (natures C3).
    Aucune,
}

impl Verification {
    pub fn code(self) -> &'static str {
        match self {
            Verification::Complete => "complete",
            Verification::AuthenticiteSeule => "authenticite_seule",
            Verification::Aucune => "aucune",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "complete" => Verification::Complete,
            "authenticite_seule" => Verification::AuthenticiteSeule,
            "aucune" => Verification::Aucune,
            _ => return None,
        })
    }
}

/// Famille de mise en page d'une nature (§9.3) : le gabarit de structure que chaque style décline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilleTrame {
    Attestation,
    Lettre,
    Convocation,
    Releve,
    Contrat,
    Carte,
    Liste,
    Bulletin,
}

/// Qui reçoit les exemplaires (§5.3). D3 : les titulaires, jamais l'élève seul avant sa majorité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destinataires {
    /// Un exemplaire par titulaire autorisé à recevoir des documents ; l'élève majeur reçoit le sien.
    TitulairesEtEleve,
    /// Aucun exemplaire nominatif : document de l'établissement (registre), un seul exemplaire interne.
    Etablissement,
}

/// Correction d'un document émis (R-D2) : nouvelle version, ou document compensatoire (facture : avoir, D-027 E).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correction {
    NouvelleVersion,
    DocumentCompensatoire(&'static str),
}

/// Ce que l'établissement signe : D3 n'émet que des natures **sans signature personnelle** (cachet de l'établissement
/// et mention « par délégation » quand l'émetteur est habilité) ; le parapheur vient avec D6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureEtab {
    CachetSeul,
}

/// Déclaration d'une nature de document (§5.3, §8.14). Les natures sont des déclarations de code : la base ne garde
/// que le paramétrage de l'établissement.
#[derive(Debug, Clone, Copy)]
pub struct NatureDecl {
    /// Code stable, publié, préfixé par le domaine de la brique qui possède la nature.
    pub code: &'static str,
    pub libelle: &'static str,
    /// Titre servi à qui n'a pas le droit de lire le titre véritable (RP1-05).
    pub titre_neutre: &'static str,
    pub famille: FamilleTrame,
    pub serie: Serie,
    pub sensibilite: Sensibilite,
    pub domaine: Domaine,
    pub destinataires: Destinataires,
    pub signature: SignatureEtab,
    pub verification: Verification,
    pub correction: Correction,
    /// Les natures dont un titre véritable révèle un fait sensible (sanction, santé…) ne le servent jamais hors droit.
    pub mentions_neutres: bool,
    /// Catégorie de conservation (§13) — la règle est validée ailleurs ; la nature ne déclare que sa catégorie.
    pub conservation: &'static str,
    /// Action d'émission propre de la nature, exigée **en plus** de `documents.emettre` (§18.2).
    pub actions_emission: &'static [Action],
    /// Langues prévues (la première est celle qui fait foi).
    pub langues: &'static [&'static str],
}

/// Certificat de scolarité (§4.1) : « l'élève X est inscrit dans l'établissement à la date du jour ».
pub const CERTIFICAT_SCOLARITE: NatureDecl = NatureDecl {
    code: "eleve.certificat_scolarite",
    libelle: "Certificat de scolarité",
    titre_neutre: "Document de l'établissement",
    famille: FamilleTrame::Attestation,
    serie: Serie::Sco,
    sensibilite: Sensibilite::C1,
    domaine: Domaine::Scolarite,
    destinataires: Destinataires::TitulairesEtEleve,
    signature: SignatureEtab::CachetSeul,
    // Fait pour être montré à un tiers (banque, caisse, transporteur) : vérification complète, désignation minimale.
    verification: Verification::Complete,
    correction: Correction::NouvelleVersion,
    mentions_neutres: false,
    conservation: "scolarite_attestation",
    actions_emission: &[],
    langues: &["fr"],
};

/// Attestation d'inscription (socle 3) : « l'élève X est inscrit pour l'année Y, en niveau Z ».
pub const ATTESTATION_INSCRIPTION: NatureDecl = NatureDecl {
    code: "inscriptions.attestation_inscription",
    libelle: "Attestation d'inscription",
    titre_neutre: "Document de l'établissement",
    famille: FamilleTrame::Attestation,
    serie: Serie::Adm,
    sensibilite: Sensibilite::C1,
    domaine: Domaine::Scolarite,
    destinataires: Destinataires::TitulairesEtEleve,
    signature: SignatureEtab::CachetSeul,
    verification: Verification::Complete,
    correction: Correction::NouvelleVersion,
    mentions_neutres: false,
    conservation: "scolarite_attestation",
    actions_emission: &[],
    langues: &["fr"],
};

/// Natures intégrées (livrées par le code). L'assemblage vérifie chacune ([`verifier_nature`]).
pub const NATURES: &[NatureDecl] = &[CERTIFICAT_SCOLARITE, ATTESTATION_INSCRIPTION];

pub fn nature(code: &str) -> Option<&'static NatureDecl> {
    NATURES.iter().find(|n| n.code == code)
}

/// Règles de l'assemblage sur une déclaration de nature (§5.3) : code unique et préfixé, titre neutre non vide,
/// aucune nature C3/C4 avec vérification publique, aucune nature de plus de C1 sans titre neutre, première langue
/// française.
pub fn verifier_nature(n: &NatureDecl) -> Result<(), String> {
    let (prefixe, reste) = n
        .code
        .split_once('.')
        .ok_or("code sans préfixe de domaine")?;
    let code_ok = |s: &str| {
        !s.is_empty()
            && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    if !code_ok(prefixe) || !code_ok(reste) || n.code.len() > 80 {
        return Err(format!("code de nature invalide : {}", n.code));
    }
    if n.titre_neutre.trim().is_empty() || n.libelle.trim().is_empty() {
        return Err("libellé ou titre neutre vide".into());
    }
    if n.sensibilite >= Sensibilite::C3 && n.verification != Verification::Aucune {
        return Err("une nature C3 ou plus n'a pas de vérification publique".into());
    }
    if n.verification == Verification::Complete && n.sensibilite >= Sensibilite::C2 {
        return Err(
            "la vérification complète est réservée aux documents faits pour être montrés (C0, C1)"
                .into(),
        );
    }
    if n.sensibilite >= Sensibilite::C2 && !n.mentions_neutres {
        return Err("une nature C2 ou plus déclare un titre neutre (mentions_neutres)".into());
    }
    if n.langues.first() != Some(&"fr") {
        return Err("la première langue d'une nature est le français (il fait foi)".into());
    }
    if n.conservation.is_empty() {
        return Err("catégorie de conservation absente".into());
    }
    Ok(())
}

// ───────────────────────── Code de vérification ─────────────────────────

/// Alphabet du code de vérification : 32 symboles sans ambiguïté visuelle (ni `I`, `O`, `0`, `1`) — 5 bits chacun ;
/// 12 caractères = **60 bits** (§6). Doit rester identique à la contrainte SQL `^[A-HJ-NP-Z2-9]{12}$`.
pub const ALPHABET_CODE: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub const LONGUEUR_CODE: usize = 12;

/// Code de vérification tiré du générateur aléatoire du système (UUID v4, `getrandom`). Les octets 6 et 8 portent les
/// marqueurs de version et de variante : on les écarte et on garde 8 octets entièrement aléatoires (64 bits), dont 60
/// servent aux 12 symboles.
pub fn generer_code_verification() -> String {
    let o = uuid::Uuid::new_v4().into_bytes();
    let mut bits: u64 = 0;
    for i in [0usize, 1, 2, 3, 4, 5, 9, 10] {
        bits = (bits << 8) | u64::from(o[i]);
    }
    (0..LONGUEUR_CODE)
        .map(|i| ALPHABET_CODE[((bits >> (5 * i)) & 31) as usize] as char)
        .collect()
}

/// Normalise une saisie humaine (`abcd-efgh jklm`, minuscules, séparateurs) en code canonique, ou `None` si elle ne
/// peut pas être un code : mauvaise longueur ou caractère hors alphabet (jamais de correction « intelligente » de `0`
/// en `O` : un code lu de travers doit échouer comme un code inexistant).
pub fn normaliser_code(saisie: &str) -> Option<String> {
    let propre: String = saisie
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if propre.len() != LONGUEUR_CODE || !propre.bytes().all(|b| ALPHABET_CODE.contains(&b)) {
        return None;
    }
    Some(propre)
}

/// Présentation lisible d'un code : `ABCD-EFGH-JKLM`.
pub fn code_lisible(code: &str) -> String {
    code.as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

// ───────────────────────── Identité : contraste AA ─────────────────────────

fn canal_lineaire(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.03928 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Rapport de contraste WCAG d'une couleur `#RRGGBB` sur blanc, ou `None` si la forme est invalide.
pub fn contraste_sur_blanc(couleur: &str) -> Option<f64> {
    let hex = couleur.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(hex, 16).ok()?;
    let (r, g, b) = (
        ((n >> 16) & 255) as u8,
        ((n >> 8) & 255) as u8,
        (n & 255) as u8,
    );
    let l = 0.2126 * canal_lineaire(r) + 0.7152 * canal_lineaire(g) + 0.0722 * canal_lineaire(b);
    Some(1.05 / (l + 0.05))
}

/// La couleur principale est refusée si son contraste sur blanc n'atteint pas AA (4,5:1, §9.3).
pub fn couleur_acceptable(couleur: &str) -> bool {
    contraste_sur_blanc(couleur).is_some_and(|c| c >= 4.5)
}

// ───────────────────────── Ports du moteur ─────────────────────────

/// Une donnée figée utilisée pour le rendu (`ent_doc_version_donnee`) : clé de gabarit et valeur. Le moteur ne reçoit
/// que des **valeurs**, jamais du texte à interpréter : l'injection est impossible par construction (§9.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonneeRendu {
    pub cle: String,
    pub valeur: String,
}

/// Ce que le moteur reçoit pour produire un exemplaire.
#[derive(Debug, Clone)]
pub struct DemandeRendu<'a> {
    pub nature: &'a NatureDecl,
    /// Style choisi (identité de l'établissement ou dérogation de la nature).
    pub style: &'a str,
    pub couleur_principale: Option<&'a str>,
    pub mentions_pied: Option<&'a str>,
    pub entite_raison_sociale: &'a str,
    pub entite_identifiants: &'a [(String, String)],
    pub donnees: &'a [DonneeRendu],
    /// Textes encadrés de la nature (emplacement, texte).
    pub textes_encadres: &'a [(String, String)],
    /// `2026-SCO-000123`.
    pub numero_affiche: &'a str,
    /// Code de vérification lisible (`ABCD-EFGH-JKLM`) et adresse à encoder dans le QR ; `None` si la nature
    /// n'imprime pas de QR (vérification `aucune`).
    pub code_verification: Option<&'a str>,
    pub url_verification: Option<&'a str>,
    pub numero_version: i32,
    /// Motif de la version (réémission) : imprimé en note « remplace la version … ».
    pub motif: Option<&'a str>,
    /// Date d'émission au jour de l'établissement, `AAAA-MM-JJ`.
    pub date_emission: &'a str,
    /// Nom de la personne habilitée au nom de qui l'établissement émet (« par délégation »), si l'émetteur n'est pas
    /// le signataire naturel.
    pub par_delegation_de: Option<&'a str>,
    /// Exemplaire nominatif : nom du destinataire et, au plus, SA propre adresse (D7). Jamais celle d'un autre.
    pub destinataire_nom: Option<&'a str>,
    pub destinataire_adresse: Option<&'a str>,
    pub langue: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRendu {
    pub octets: Vec<u8>,
    pub nb_pages: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurRendu {
    /// Aucun moteur n'est branché (lot D1 non intégré) : l'émission s'arrête, rien n'est écrit.
    #[error("moteur de rendu indisponible")]
    Indisponible,
    #[error("rendu impossible : {0}")]
    Impossible(String),
}

/// Le moteur de rendu PDF (D-062 : maison, accents WinAnsi, tableaux, QR). Synchrone : un certificat se rend en moins
/// de 200 ms (§9.2), sans réseau ni exécution de code venant des données.
pub trait RenduDocument: Send + Sync {
    fn rendre(&self, demande: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu>;
}

/// Moteur par défaut tant que D1 n'est pas branché : refuse toujours (aucun faux PDF).
#[derive(Debug, Default, Clone, Copy)]
pub struct RenduIndisponible;

impl RenduDocument for RenduIndisponible {
    fn rendre(&self, _: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
        Err(ErreurRendu::Indisponible)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scelle {
    pub octets: Vec<u8>,
    /// Identifiant de la clé du cachet (rotation, révocation) : consigné sur la version.
    pub cle_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurCachet {
    #[error("cachet de l'établissement indisponible")]
    Indisponible,
    #[error("scellement impossible : {0}")]
    Impossible(String),
}

/// Le cachet électronique de l'établissement (personne morale, non qualifié, §10.7). La clé vit hors base.
pub trait CachetEtablissement: Send + Sync {
    fn sceller(&self, etablissement_id: uuid::Uuid, pdf: &[u8]) -> Result<Scelle, ErreurCachet>;
}

/// Cachet par défaut tant que la clé n'est pas configurée : refuse toujours (§9.5 : jamais de document non scellé).
#[derive(Debug, Default, Clone, Copy)]
pub struct CachetIndisponible;

impl CachetEtablissement for CachetIndisponible {
    fn sceller(&self, _: uuid::Uuid, _: &[u8]) -> Result<Scelle, ErreurCachet> {
        Err(ErreurCachet::Indisponible)
    }
}

// ───────────────────────── Tests unitaires (sans base) ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natures_integrees_valides_et_uniques() {
        for n in NATURES {
            verifier_nature(n).unwrap_or_else(|e| panic!("{} : {e}", n.code));
        }
        let mut codes: Vec<_> = NATURES.iter().map(|n| n.code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), NATURES.len());
        assert_eq!(
            nature("eleve.certificat_scolarite").map(|n| n.serie),
            Some(Serie::Sco)
        );
        assert!(nature("inconnue.nature").is_none());
    }

    #[test]
    fn regles_de_l_assemblage() {
        // C2 : titre neutre obligatoire ; vérification complète interdite.
        let c2 = NatureDecl {
            sensibilite: Sensibilite::C2,
            ..CERTIFICAT_SCOLARITE
        };
        assert!(verifier_nature(&c2).is_err());
        let c2_authentique = NatureDecl {
            sensibilite: Sensibilite::C2,
            verification: Verification::AuthenticiteSeule,
            mentions_neutres: true,
            ..CERTIFICAT_SCOLARITE
        };
        assert!(verifier_nature(&c2_authentique).is_ok());
        // C3 : aucune vérification publique.
        let c3 = NatureDecl {
            sensibilite: Sensibilite::C3,
            mentions_neutres: true,
            ..c2_authentique
        };
        assert!(verifier_nature(&c3).is_err());
        assert!(verifier_nature(&NatureDecl {
            verification: Verification::Aucune,
            ..c3
        })
        .is_ok());
        // Code sans préfixe de domaine, langue de référence autre que le français.
        assert!(verifier_nature(&NatureDecl {
            code: "sansdomaine",
            ..CERTIFICAT_SCOLARITE
        })
        .is_err());
        assert!(verifier_nature(&NatureDecl {
            langues: &["en", "fr"],
            ..CERTIFICAT_SCOLARITE
        })
        .is_err());
    }

    #[test]
    fn code_de_verification_60_bits_sans_ambiguite() {
        let mut vus = std::collections::HashSet::new();
        for _ in 0..2000 {
            let c = generer_code_verification();
            assert_eq!(c.len(), LONGUEUR_CODE);
            assert!(c.bytes().all(|b| ALPHABET_CODE.contains(&b)), "{c}");
            assert!(!c.contains(['I', 'O', '0', '1']), "{c}");
            assert_eq!(normaliser_code(&c).as_deref(), Some(c.as_str()));
            assert!(vus.insert(c), "code en double sur 2000 tirages");
        }
        // L'alphabet a 32 symboles distincts (5 bits chacun, 12 × 5 = 60 bits).
        let mut a = ALPHABET_CODE.to_vec();
        a.sort_unstable();
        a.dedup();
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn normalisation_stricte_du_code_saisi() {
        assert_eq!(
            normaliser_code("abcd-efgh jklm").as_deref(),
            Some("ABCDEFGHJKLM")
        );
        assert_eq!(code_lisible("ABCDEFGHJKLM"), "ABCD-EFGH-JKLM");
        // Pas de correction de 0 en O, de 1 en I : un code lu de travers échoue.
        assert!(normaliser_code("ABCD-EFGH-JKL0").is_none());
        assert!(normaliser_code("ABCD-EFGH-JKLI").is_none());
        assert!(normaliser_code("ABCD").is_none());
        assert!(normaliser_code("").is_none());
        // Le numéro séquentiel n'est jamais un code.
        assert!(normaliser_code("2026-SCO-000123").is_none());
        assert_eq!(numero_affiche(2026, "SCO", 123), "2026-SCO-000123");
    }

    #[test]
    fn contraste_aa_de_la_couleur_principale() {
        assert!(couleur_acceptable("#000000"));
        assert!(couleur_acceptable("#1F3A93"));
        assert!(!couleur_acceptable("#FFFF00"), "jaune sur blanc");
        assert!(!couleur_acceptable("#AAAAAA"));
        assert!(!couleur_acceptable("rouge"));
        assert!(!couleur_acceptable("#12345"));
    }

    #[test]
    fn domaine_choisit_le_droit_de_lecture() {
        assert_eq!(Domaine::Scolarite.action_lecture(), Action::DocumentsLire);
        assert_eq!(Domaine::Sante.action_lecture(), Action::DocumentsLireSante);
        assert_eq!(
            Domaine::Justice.action_lecture(),
            Action::DocumentsRestreintsLire
        );
        assert_eq!(
            Domaine::Financier.action_lecture(),
            Action::DocumentsFinancierLire
        );
        assert_eq!(
            Domaine::Rh.action_lecture(),
            Action::DocumentsPersonnelsLire
        );
        for d in [
            Domaine::Scolarite,
            Domaine::Sante,
            Domaine::Justice,
            Domaine::Financier,
            Domaine::Rh,
            Domaine::Social,
            Domaine::Disciplinaire,
        ] {
            assert_eq!(Domaine::parse(d.code()), Some(d));
        }
        assert!(Sensibilite::C2.trace_la_lecture() && !Sensibilite::C1.trace_la_lecture());
    }

    #[test]
    fn ports_par_defaut_refusent() {
        assert_eq!(
            RenduIndisponible.rendre(&demande_test()),
            Err(ErreurRendu::Indisponible)
        );
        assert_eq!(
            CachetIndisponible.sceller(uuid::Uuid::nil(), b"%PDF"),
            Err(ErreurCachet::Indisponible)
        );
    }

    fn demande_test() -> DemandeRendu<'static> {
        DemandeRendu {
            nature: &CERTIFICAT_SCOLARITE,
            style: "classique",
            couleur_principale: None,
            mentions_pied: None,
            entite_raison_sociale: "Lycée Test",
            entite_identifiants: &[],
            donnees: &[],
            textes_encadres: &[],
            numero_affiche: "2026-SCO-000001",
            code_verification: None,
            url_verification: None,
            numero_version: 1,
            motif: None,
            date_emission: "2026-11-14",
            par_delegation_de: None,
            destinataire_nom: None,
            destinataire_adresse: None,
            langue: "fr",
        }
    }
}
