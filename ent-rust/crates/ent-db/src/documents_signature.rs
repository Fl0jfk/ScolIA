//! Socle 7, lot D6 : **la brique de signature**, partie pure (sans base) — modes, niveaux, natures déclarées, menu par
//! nature, chaîne de preuve, défis, codes à usage unique, ports d'horodatage RFC 3161 (`07_DOCUMENTS.md` §10 ;
//! `CARNET_IDEES.md` I-013, I-014, décisions de Florian du 16-11).
//!
//! * **Quatre modes, tout en maison et gratuit** : passkey (le plus solide), code SMS à usage unique (repli), tablette au
//!   guichet (agent identifié + confirmation), papier (impression, signature, dépôt du scan, contrôle humain).
//! * **Menu par nature** : 1 à 4 modes, un seul par défaut ; un mode hors de ce que la nature admet est **bloqué**, un
//!   mode admis mais faible est **signalé** (l'avertissement est écrit dans la preuve).
//! * **Preuve** : empreinte SHA-256 complète (64 caractères), journal des signatures **chaîné** (chaque entrée scelle la
//!   précédente), vérifiable hors ligne par [`verifier_chaine`].
//! * **Horodatage qualifié** : un port ([`ServiceHorodatage`]) ; rien n'est acheté ; désactivé par défaut.

use chrono::{DateTime, SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ───────────────────────── Modes et niveaux ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeSignature {
    Passkey,
    CodeSms,
    Guichet,
    Papier,
}

impl ModeSignature {
    pub const TOUS: [ModeSignature; 4] = [Self::Passkey, Self::CodeSms, Self::Guichet, Self::Papier];

    pub fn code(self) -> &'static str {
        match self {
            Self::Passkey => "passkey",
            Self::CodeSms => "code_sms",
            Self::Guichet => "guichet",
            Self::Papier => "papier",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|m| m.code() == s)
    }

    /// Libellé du choix présenté à l'établissement (une seule question : « Comment la personne doit-elle signer ? »).
    pub fn libelle(self) -> &'static str {
        match self {
            Self::Passkey => "En ligne avec la clé d'accès (passkey), gratuit",
            Self::CodeSms => "En ligne avec un code par SMS, gratuit",
            Self::Guichet => "Au guichet sur tablette, avec un agent, gratuit",
            Self::Papier => "Sur papier : impression, signature, scan déposé, gratuit",
        }
    }

    /// Valeur de la preuve, en une phrase (« en savoir plus »).
    pub fn valeur(self) -> &'static str {
        match self {
            Self::Passkey => {
                "Signature renforcée : la clé d'accès de la personne signe l'empreinte du document ; la plus solide des modes ScolIA."
            }
            Self::CodeSms => {
                "Accord simple : un code à usage unique reçu sur le téléphone enregistré ; tracé, mais exposé au détournement de carte SIM."
            }
            Self::Guichet => {
                "Présence constatée : un agent identifié vérifie l'identité (sans la copier) et la personne confirme par un code ; sans confirmation, la signature reste simple."
            }
            Self::Papier => "Signature manuscrite : le scan est contrôlé par un humain (signature présente, pages complètes).",
        }
    }

    /// Niveau de preuve obtenu par ce mode, hors cas du guichet sans confirmation.
    pub fn niveau(self) -> Niveau {
        match self {
            Self::Passkey => Niveau::S2,
            Self::CodeSms => Niveau::S1,
            Self::Guichet => Niveau::G,
            Self::Papier => Niveau::P,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Niveau {
    S1,
    S2,
    G,
    /// Tablette sans confirmation : signature simple, affichée comme telle.
    G0,
    P,
}

impl Niveau {
    pub fn code(self) -> &'static str {
        match self {
            Self::S1 => "S1",
            Self::S2 => "S2",
            Self::G => "G",
            Self::G0 => "G0",
            Self::P => "P",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Self::S1, Self::S2, Self::G, Self::G0, Self::P].into_iter().find(|n| n.code() == s)
    }
    pub fn libelle(self) -> &'static str {
        match self {
            Self::S1 => "accord simple (code à usage unique)",
            Self::S2 => "signature renforcée (clé d'accès)",
            Self::G => "constatée au guichet (agent, identité vue, confirmation)",
            Self::G0 => "tablette au guichet sans confirmation (signature simple)",
            Self::P => "papier manuscrit, contrôlé",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegleAccord {
    Aucun,
    Un,
    Tous,
}

impl RegleAccord {
    pub fn code(self) -> &'static str {
        match self {
            Self::Aucun => "aucun",
            Self::Un => "un",
            Self::Tous => "tous",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Self::Aucun, Self::Un, Self::Tous].into_iter().find(|r| r.code() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ordre {
    Parallele,
    Sequentiel,
}

impl Ordre {
    pub fn code(self) -> &'static str {
        match self {
            Self::Parallele => "parallele",
            Self::Sequentiel => "sequentiel",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Self::Parallele, Self::Sequentiel].into_iter().find(|o| o.code() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleSignataire {
    Titulaire,
    Eleve,
    Agent,
    Etablissement,
    Externe,
}

impl RoleSignataire {
    pub fn code(self) -> &'static str {
        match self {
            Self::Titulaire => "titulaire",
            Self::Eleve => "eleve",
            Self::Agent => "agent",
            Self::Etablissement => "etablissement",
            Self::Externe => "externe",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Self::Titulaire, Self::Eleve, Self::Agent, Self::Etablissement, Self::Externe]
            .into_iter()
            .find(|r| r.code() == s)
    }
    pub fn libelle(self) -> &'static str {
        match self {
            Self::Titulaire => "responsable légal",
            Self::Eleve => "élève",
            Self::Agent => "agent",
            Self::Etablissement => "établissement",
            Self::Externe => "signataire externe",
        }
    }
}

// ───────────────────────── Natures qui se signent ─────────────────────────

/// Déclaration de signature d'une nature de document : ce que **le module appelant** déclare (règle d'accord, ordre,
/// modes que la nature admet, modes faibles, horodatage par défaut). Les établissements règlent ensuite le menu et
/// l'horodatage **à l'intérieur** de ce que la nature admet.
#[derive(Debug, Clone, Copy)]
pub struct NatureSignature {
    pub code: &'static str,
    pub libelle: &'static str,
    pub regle: RegleAccord,
    pub ordre: Ordre,
    pub eleve_cosigne: bool,
    /// Modes que la nature admet (tout autre mode est **bloqué**).
    pub modes_possibles: &'static [ModeSignature],
    /// Menu par défaut, dans l'ordre d'affichage ; le premier est le mode présélectionné.
    pub modes_defaut: &'static [ModeSignature],
    /// Parmi les possibles, ceux qu'on **signale** comme faibles pour cette nature.
    pub modes_faibles: &'static [ModeSignature],
    /// Horodatage qualifié par défaut (désactivé tant qu'aucun service n'est branché).
    pub horodatage_defaut: bool,
    /// Échéance par défaut, en jours à partir du lancement.
    pub echeance_jours: i64,
}

use ModeSignature::{CodeSms, Guichet, Papier, Passkey};

/// Natures déclarées par les modules (D8 ajoutera les autres au fil des socles). Fermé par défaut : une nature absente
/// de cette liste ne peut pas être signée.
pub const NATURES_SIGNATURE: &[NatureSignature] = &[
    // Famille qui accuse réception d'une attestation : accord simple suffisant.
    NatureSignature {
        code: "inscriptions.attestation_inscription",
        libelle: "Attestation d'inscription (accusé de la famille)",
        regle: RegleAccord::Un,
        ordre: Ordre::Parallele,
        eleve_cosigne: false,
        modes_possibles: &[Passkey, CodeSms, Guichet, Papier],
        modes_defaut: &[Passkey, CodeSms, Guichet, Papier],
        modes_faibles: &[],
        horodatage_defaut: false,
        echeance_jours: 30,
    },
    // Finalisation du dossier d'inscription : horodatée par défaut (décision de Florian, 16-11).
    NatureSignature {
        code: "inscriptions.finalisation_dossier",
        libelle: "Finalisation du dossier d'inscription",
        regle: RegleAccord::Tous,
        ordre: Ordre::Parallele,
        eleve_cosigne: false,
        modes_possibles: &[Passkey, CodeSms, Guichet, Papier],
        modes_defaut: &[Passkey, CodeSms, Guichet, Papier],
        modes_faibles: &[CodeSms],
        horodatage_defaut: true,
        echeance_jours: 30,
    },
    // Convocation disciplinaire : remise en main propre contre signature ; le simple code SMS n'y suffit jamais.
    NatureSignature {
        code: "discipline.convocation",
        libelle: "Convocation disciplinaire",
        regle: RegleAccord::Tous,
        ordre: Ordre::Parallele,
        eleve_cosigne: false,
        modes_possibles: &[Guichet, Papier, Passkey],
        modes_defaut: &[Guichet, Papier, Passkey],
        modes_faibles: &[Passkey],
        horodatage_defaut: true,
        echeance_jours: 10,
    },
    // Convention de stage : l'élève cosigne ; le tuteur choisit lui-même son mode.
    NatureSignature {
        code: "stages.convention_stage",
        libelle: "Convention de stage",
        regle: RegleAccord::Un,
        ordre: Ordre::Sequentiel,
        eleve_cosigne: true,
        modes_possibles: &[Passkey, CodeSms, Papier],
        modes_defaut: &[Passkey, CodeSms, Papier],
        modes_faibles: &[CodeSms],
        horodatage_defaut: false,
        echeance_jours: 21,
    },
    // Autorisation courante (sortie, voyage) : un responsable suffit.
    NatureSignature {
        code: "voyages.autorisation_parentale",
        libelle: "Autorisation parentale de sortie ou de voyage",
        regle: RegleAccord::Un,
        ordre: Ordre::Parallele,
        eleve_cosigne: false,
        modes_possibles: &[Passkey, CodeSms, Papier],
        modes_defaut: &[CodeSms, Passkey, Papier],
        modes_faibles: &[],
        horodatage_defaut: false,
        echeance_jours: 14,
    },
    // Contrat de travail : signature avancée recommandée (CDD) ; le code SMS est bloqué.
    NatureSignature {
        code: "rh.contrat_travail",
        libelle: "Contrat de travail",
        regle: RegleAccord::Aucun,
        ordre: Ordre::Sequentiel,
        eleve_cosigne: false,
        modes_possibles: &[Passkey, Guichet, Papier],
        modes_defaut: &[Passkey, Guichet, Papier],
        modes_faibles: &[],
        horodatage_defaut: false,
        echeance_jours: 15,
    },
];

pub fn nature_signature(code: &str) -> Option<&'static NatureSignature> {
    NATURES_SIGNATURE.iter().find(|n| n.code == code)
}

/// Cohérence d'une déclaration (testée sur toutes les natures livrées).
pub fn verifier_nature_signature(n: &NatureSignature) -> Result<(), String> {
    if n.modes_possibles.is_empty() || n.modes_possibles.len() > 4 {
        return Err(format!("{} : 1 à 4 modes possibles", n.code));
    }
    if n.modes_defaut.is_empty() || n.modes_defaut.len() > 4 {
        return Err(format!("{} : menu par défaut de 1 à 4 modes", n.code));
    }
    if n.modes_defaut.iter().any(|m| !n.modes_possibles.contains(m)) {
        return Err(format!("{} : menu par défaut hors des modes possibles", n.code));
    }
    if n.modes_faibles.iter().any(|m| !n.modes_possibles.contains(m)) {
        return Err(format!("{} : mode faible hors des modes possibles", n.code));
    }
    for (i, m) in n.modes_defaut.iter().enumerate() {
        if n.modes_defaut[..i].contains(m) {
            return Err(format!("{} : mode en double", n.code));
        }
    }
    if n.echeance_jours < 1 || n.echeance_jours > 365 {
        return Err(format!("{} : échéance par défaut hors bornes", n.code));
    }
    Ok(())
}

// ───────────────────────── Menu des modes ─────────────────────────

/// Réglage d'un établissement : menu ordonné, le mode par défaut est l'un des modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReglageMenu {
    pub modes: Vec<ModeSignature>,
    pub defaut: ModeSignature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurMode {
    /// Aucun mode, ou plus de quatre.
    NombreInvalide,
    Doublon,
    /// Le mode par défaut n'est pas dans le menu.
    DefautAbsent,
    /// Le mode est hors de ce que la nature admet : bloqué.
    Bloque(&'static str),
    /// Le mode est admis par la nature mais absent du menu de ce circuit.
    HorsMenu,
}

pub fn valider_reglage(n: &NatureSignature, r: &ReglageMenu) -> Result<(), ErreurMode> {
    if r.modes.is_empty() || r.modes.len() > 4 {
        return Err(ErreurMode::NombreInvalide);
    }
    for (i, m) in r.modes.iter().enumerate() {
        if r.modes[..i].contains(m) {
            return Err(ErreurMode::Doublon);
        }
        if !n.modes_possibles.contains(m) {
            return Err(ErreurMode::Bloque(MOTIF_BLOQUE));
        }
    }
    if !r.modes.contains(&r.defaut) {
        return Err(ErreurMode::DefautAbsent);
    }
    Ok(())
}

pub const MOTIF_BLOQUE: &str = "ce mode est trop faible pour cette nature de document : il est bloqué";
pub const MOTIF_FAIBLE: &str = "ce mode est faible pour cette nature de document : la preuve le mentionnera";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoixMenu {
    pub mode: ModeSignature,
    pub defaut: bool,
    /// Avertissement affiché au choix (mode faible), sinon `None`.
    pub signale: Option<&'static str>,
}

/// Menu effectif : le réglage de l'établissement, sinon celui de la nature.
pub fn menu(n: &NatureSignature, reglage: Option<&ReglageMenu>) -> Vec<ChoixMenu> {
    let (modes, defaut): (Vec<ModeSignature>, ModeSignature) = match reglage {
        Some(r) => (r.modes.clone(), r.defaut),
        None => (n.modes_defaut.to_vec(), n.modes_defaut[0]),
    };
    modes
        .into_iter()
        .map(|m| ChoixMenu {
            mode: m,
            defaut: m == defaut,
            signale: n.modes_faibles.contains(&m).then_some(MOTIF_FAIBLE),
        })
        .collect()
}

/// Un mode est-il admissible pour cette nature, dans ce menu ? `Ok(Some(..))` = admis avec avertissement.
pub fn evaluer_mode(
    n: &NatureSignature,
    menu_effectif: &[ChoixMenu],
    mode: ModeSignature,
) -> Result<Option<&'static str>, ErreurMode> {
    if !n.modes_possibles.contains(&mode) {
        return Err(ErreurMode::Bloque(MOTIF_BLOQUE));
    }
    match menu_effectif.iter().find(|c| c.mode == mode) {
        None => Err(ErreurMode::HorsMenu),
        Some(c) => Ok(c.signale),
    }
}

// ───────────────────────── Intention, empreintes, chaîne ─────────────────────────

pub const INTENTION_VERSION: i16 = 1;

/// Phrase affichée au signataire et acceptée par lui ; son empreinte est scellée dans la chaîne.
pub fn intention_texte(libelle_nature: &str, role: &str) -> String {
    format!(
        "En qualité de {role}, je certifie avoir lu le document « {libelle_nature} » dans la version affichée et je le signe."
    )
}

pub fn sha256_hex(octets: &[u8]) -> String {
    hex(&Sha256::digest(octets))
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Empreinte précédente de la première entrée d'un circuit : liée à ce circuit.
pub fn tete_initiale(circuit_id: Uuid) -> String {
    sha256_hex(format!("scolia.circuit:{circuit_id}").as_bytes())
}

/// Horodatage canonique (microsecondes, UTC) : celui que stocke la base.
pub fn horodatage_canonique(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Micros, true)
}

/// Tronque à la microseconde (précision de `timestamptz`) : ce qui est haché est ce qui est relu.
pub fn tronquer_micros(t: DateTime<Utc>) -> DateTime<Utc> {
    use chrono::Timelike;
    let ns = t.timestamp_subsec_nanos();
    t.with_nanosecond(ns / 1000 * 1000).unwrap_or(t)
}

/// Lien entre le défi d'authentification et ce qui est signé : SHA-256(défi ‖ empreinte ‖ signataire ‖ intention ‖ nonce).
pub fn lien_defi(
    defi: &str,
    empreinte: &str,
    signataire_id: Uuid,
    intention_sha256: &str,
    nonce: &str,
) -> String {
    sha256_hex(
        format!("scolia.defi.v1\n{defi}\n{empreinte}\n{signataire_id}\n{intention_sha256}\n{nonce}").as_bytes(),
    )
}

/// Les champs qu'une entrée de la chaîne scelle (tout ce qui compte juridiquement, rien de plus).
#[derive(Debug, Clone)]
pub struct EntreeChaine {
    pub circuit_id: Uuid,
    pub signataire_id: Uuid,
    pub version_id: Uuid,
    pub rang: i32,
    pub empreinte_signee: String,
    pub methode: String,
    pub niveau: String,
    pub intention_sha256: String,
    pub horodatage: DateTime<Utc>,
    pub lien_sha256: Option<String>,
    pub precedent: String,
}

pub fn sceller_entree(e: &EntreeChaine) -> String {
    sha256_hex(
        format!(
            "scolia.signature.v1\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            e.precedent,
            e.circuit_id,
            e.signataire_id,
            e.version_id,
            e.rang,
            e.empreinte_signee,
            e.methode,
            e.niveau,
            e.intention_sha256,
            horodatage_canonique(e.horodatage),
            e.lien_sha256.as_deref().unwrap_or("-"),
        )
        .as_bytes(),
    )
}

/// Une entrée relue de la base, avec son sceau.
#[derive(Debug, Clone)]
pub struct EntreeLue {
    pub entree: EntreeChaine,
    pub entree_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuptureChaine {
    pub rang: i32,
    pub raison: &'static str,
}

/// **Vérification hors ligne** de la chaîne d'un circuit : rangs consécutifs, chaque entrée porte le sceau de la
/// précédente, chaque sceau est recalculable. Fonctionne à partir des seules données du dossier de preuve.
pub fn verifier_chaine(circuit_id: Uuid, entrees: &[EntreeLue]) -> Result<String, RuptureChaine> {
    let mut precedent = tete_initiale(circuit_id);
    for (i, l) in entrees.iter().enumerate() {
        let rang = i as i32 + 1;
        if l.entree.circuit_id != circuit_id {
            return Err(RuptureChaine { rang, raison: "entrée d'un autre circuit" });
        }
        if l.entree.rang != rang {
            return Err(RuptureChaine { rang, raison: "rang non consécutif" });
        }
        if l.entree.precedent != precedent {
            return Err(RuptureChaine { rang, raison: "l'entrée ne scelle pas la précédente" });
        }
        if sceller_entree(&l.entree) != l.entree_sha256 {
            return Err(RuptureChaine { rang, raison: "sceau de l'entrée altéré" });
        }
        precedent = l.entree_sha256.clone();
    }
    Ok(precedent)
}

// ───────────────────────── Codes à usage unique ─────────────────────────

/// Code à six chiffres tiré du générateur du système (UUID v4), sans biais de modulo.
pub fn generer_code_numerique() -> String {
    loop {
        let o = Uuid::new_v4().into_bytes();
        let n = u32::from_be_bytes([o[0], o[1], o[2], o[3]]);
        if n < 4_294_000_000 {
            return format!("{:06}", n % 1_000_000);
        }
    }
}

/// Empreinte d'un code : jamais le code en clair en base.
pub fn empreinte_code(defi_id: Uuid, code: &str) -> String {
    sha256_hex(format!("scolia.code.v1:{defi_id}:{code}").as_bytes())
}

pub fn nonce_aleatoire() -> String {
    hex(&Uuid::new_v4().into_bytes())
}

/// Masque un numéro de mobile : « 06 ** ** ** 12 » pour un numéro français, sinon seuls les deux derniers chiffres.
pub fn masquer_telephone(brut: &str) -> String {
    let mut chiffres: String = brut.chars().filter(char::is_ascii_digit).collect();
    if chiffres.starts_with("33") && chiffres.len() == 11 {
        chiffres = format!("0{}", &chiffres[2..]);
    }
    let n = chiffres.len();
    if n < 4 {
        return "** ** ** **".to_string();
    }
    format!("{} ** ** ** {}", &chiffres[..2], &chiffres[n - 2..])
}

// ───────────────────────── Horodatage qualifié (RFC 3161) : le port ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeHorodatage {
    /// Le compte de ScolIA, un seul pour tous les établissements.
    Mutualise,
    /// Le prestataire de l'établissement (adresse et identifiants saisis par lui).
    Propre,
}

impl ModeHorodatage {
    pub fn code(self) -> &'static str {
        match self {
            Self::Mutualise => "mutualise",
            Self::Propre => "propre",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Self::Mutualise, Self::Propre].into_iter().find(|m| m.code() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurHorodatage {
    #[error("service d'horodatage indisponible : {0}")]
    Indisponible(String),
    #[error("horodatage refusé : {0}")]
    Refuse(String),
}

/// Un service d'horodatage RFC 3161 : rend le jeton (DER) pour une empreinte SHA-256.
pub trait ServiceHorodatage: Send + Sync {
    fn horodater(&self, empreinte: &[u8; 32]) -> Result<Vec<u8>, ErreurHorodatage>;
}

/// Réglage d'un service « propre » (adresse, identifiants).
#[derive(Debug, Clone)]
pub struct ConfigServiceHorodatage {
    pub url: String,
    pub identifiant: Option<String>,
    pub secret: Option<String>,
}

/// Rend le service à utiliser pour un établissement : `None` = aucun branché (horodatage désactivé de fait).
pub trait FournisseurHorodatage: Send + Sync {
    fn service(
        &self,
        etablissement_id: Uuid,
        mode: ModeHorodatage,
        config: Option<&ConfigServiceHorodatage>,
    ) -> Option<Box<dyn ServiceHorodatage>>;
}

/// Aucun service branché (défaut : rien n'est acheté).
#[derive(Debug, Default, Clone, Copy)]
pub struct HorodatageAucun;

impl FournisseurHorodatage for HorodatageAucun {
    fn service(&self, _: Uuid, _: ModeHorodatage, _: Option<&ConfigServiceHorodatage>) -> Option<Box<dyn ServiceHorodatage>> {
        None
    }
}

/// Client réseau RFC 3161 (adaptateur de `ent_object::rfc3161`).
pub struct ServiceRfc3161 {
    client: ent_object::rfc3161::ClientRfc3161,
}

impl ServiceRfc3161 {
    pub fn new(c: &ConfigServiceHorodatage) -> Self {
        Self {
            client: ent_object::rfc3161::ClientRfc3161::new(&c.url, c.identifiant.as_deref(), c.secret.as_deref()),
        }
    }
}

impl ServiceHorodatage for ServiceRfc3161 {
    fn horodater(&self, empreinte: &[u8; 32]) -> Result<Vec<u8>, ErreurHorodatage> {
        let o = Uuid::new_v4().into_bytes();
        let mut nonce = [0u8; 8];
        nonce.copy_from_slice(&o[..8]);
        self.client.horodater(empreinte, nonce).map_err(|e| match e {
            ent_object::rfc3161::ErreurRfc3161::Refuse(_) => ErreurHorodatage::Refuse(e.to_string()),
            _ => ErreurHorodatage::Indisponible(e.to_string()),
        })
    }
}

/// Fournisseur de production : le compte mutualisé vient de la configuration de ScolIA (adresse, identifiants), le
/// compte propre de l'établissement de ses réglages. Sans configuration : aucun service.
#[derive(Debug, Default, Clone)]
pub struct FournisseurRfc3161 {
    pub mutualise: Option<ConfigServiceHorodatage>,
}

impl FournisseurHorodatage for FournisseurRfc3161 {
    fn service(
        &self,
        _: Uuid,
        mode: ModeHorodatage,
        config: Option<&ConfigServiceHorodatage>,
    ) -> Option<Box<dyn ServiceHorodatage>> {
        let c = match mode {
            ModeHorodatage::Mutualise => self.mutualise.as_ref()?,
            ModeHorodatage::Propre => config?,
        };
        Some(Box::new(ServiceRfc3161::new(c)))
    }
}

// ───────────────────────── Tests unitaires (sans base) ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natures_coherentes() {
        for n in NATURES_SIGNATURE {
            verifier_nature_signature(n).unwrap();
        }
        let mut codes: Vec<_> = NATURES_SIGNATURE.iter().map(|n| n.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), NATURES_SIGNATURE.len());
    }

    #[test]
    fn horodatage_par_defaut_convocations_et_inscriptions_seulement() {
        let oui: Vec<_> = NATURES_SIGNATURE.iter().filter(|n| n.horodatage_defaut).map(|n| n.code).collect();
        assert_eq!(oui, vec!["inscriptions.finalisation_dossier", "discipline.convocation"]);
    }

    #[test]
    fn menu_bloque_signale_et_hors_menu() {
        let conv = nature_signature("discipline.convocation").unwrap();
        // Code SMS : bloqué (trop faible pour une convocation).
        let m = menu(conv, None);
        assert_eq!(evaluer_mode(conv, &m, CodeSms), Err(ErreurMode::Bloque(MOTIF_BLOQUE)));
        assert_eq!(m[0].mode, Guichet);
        assert!(m[0].defaut);
        // Passkey : admis mais signalé.
        assert_eq!(evaluer_mode(conv, &m, Passkey), Ok(Some(MOTIF_FAIBLE)));
        // Réglage de l'établissement : un seul choix ; le reste est « hors menu ».
        let r = ReglageMenu { modes: vec![Papier], defaut: Papier };
        valider_reglage(conv, &r).unwrap();
        let m2 = menu(conv, Some(&r));
        assert_eq!(evaluer_mode(conv, &m2, Guichet), Err(ErreurMode::HorsMenu));
        assert_eq!(evaluer_mode(conv, &m2, Papier), Ok(None));
    }

    #[test]
    fn reglage_refuse_les_menus_invalides() {
        let n = nature_signature("inscriptions.finalisation_dossier").unwrap();
        let ok = ReglageMenu { modes: vec![Passkey, CodeSms, Guichet, Papier], defaut: Passkey };
        assert_eq!(valider_reglage(n, &ok), Ok(()));
        assert_eq!(valider_reglage(n, &ReglageMenu { modes: vec![], defaut: Passkey }), Err(ErreurMode::NombreInvalide));
        assert_eq!(
            valider_reglage(n, &ReglageMenu { modes: vec![Passkey, Passkey], defaut: Passkey }),
            Err(ErreurMode::Doublon)
        );
        assert_eq!(
            valider_reglage(n, &ReglageMenu { modes: vec![Passkey], defaut: Papier }),
            Err(ErreurMode::DefautAbsent)
        );
        let rh = nature_signature("rh.contrat_travail").unwrap();
        assert!(matches!(
            valider_reglage(rh, &ReglageMenu { modes: vec![CodeSms], defaut: CodeSms }),
            Err(ErreurMode::Bloque(_))
        ));
    }

    fn entree(c: Uuid, rang: i32, precedent: &str) -> EntreeLue {
        let e = EntreeChaine {
            circuit_id: c,
            signataire_id: Uuid::from_u128(rang as u128),
            version_id: Uuid::from_u128(99),
            rang,
            empreinte_signee: sha256_hex(b"doc"),
            methode: "passkey".into(),
            niveau: "S2".into(),
            intention_sha256: sha256_hex(b"intention"),
            horodatage: DateTime::from_timestamp(1_700_000_000 + i64::from(rang), 123_000).unwrap(),
            lien_sha256: Some(sha256_hex(b"lien")),
            precedent: precedent.to_string(),
        };
        let s = sceller_entree(&e);
        EntreeLue { entree: e, entree_sha256: s }
    }

    #[test]
    fn chaine_valide_et_ruptures() {
        let c = Uuid::from_u128(7);
        let e1 = entree(c, 1, &tete_initiale(c));
        let e2 = entree(c, 2, &e1.entree_sha256);
        let e3 = entree(c, 3, &e2.entree_sha256);
        let tete = verifier_chaine(c, &[e1.clone(), e2.clone(), e3.clone()]).unwrap();
        assert_eq!(tete, e3.entree_sha256);
        assert_eq!(tete.len(), 64, "empreinte SHA-256 complète");
        // Altération d'un champ : sceau invalide.
        let mut f = e2.clone();
        f.entree.niveau = "S1".into();
        assert_eq!(verifier_chaine(c, &[e1.clone(), f, e3.clone()]).unwrap_err().rang, 2);
        // Entrée supprimée : rang non consécutif.
        assert_eq!(verifier_chaine(c, &[e1.clone(), e3.clone()]).unwrap_err().rang, 2);
        // Entrée réordonnée.
        assert!(verifier_chaine(c, &[e2, e1.clone()]).is_err());
        // Autre circuit.
        assert!(verifier_chaine(Uuid::from_u128(8), &[e1]).is_err());
    }

    #[test]
    fn codes_et_masques() {
        let c = generer_code_numerique();
        assert_eq!(c.len(), 6);
        assert!(c.bytes().all(|b| b.is_ascii_digit()));
        let id = Uuid::from_u128(1);
        assert_ne!(empreinte_code(id, "123456"), empreinte_code(id, "123457"));
        assert_eq!(masquer_telephone("+33 6 12 34 56 12"), "06 ** ** ** 12");
        assert_eq!(masquer_telephone("0612345678"), "06 ** ** ** 78");
        assert_eq!(masquer_telephone("12"), "** ** ** **");
        assert_eq!(nonce_aleatoire().len(), 32);
    }

    #[test]
    fn troncature_microsecondes_stable() {
        let t = DateTime::from_timestamp(1_700_000_000, 123_456_789).unwrap();
        let u = tronquer_micros(t);
        assert_eq!(u.timestamp_subsec_nanos(), 123_456_000);
        assert_eq!(horodatage_canonique(u), "2023-11-14T22:13:20.123456Z");
    }

    #[test]
    fn fournisseur_sans_config_ne_rend_rien() {
        let f = FournisseurRfc3161::default();
        assert!(f.service(Uuid::nil(), ModeHorodatage::Mutualise, None).is_none());
        assert!(f.service(Uuid::nil(), ModeHorodatage::Propre, None).is_none());
        assert!(HorodatageAucun.service(Uuid::nil(), ModeHorodatage::Propre, None).is_none());
    }
}
