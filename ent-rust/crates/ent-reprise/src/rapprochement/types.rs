//! Types du rapprochement : raisons, confiance, candidats, verdicts (lot R3).
//!
//! Chaque proposition porte ses **raisons** (codes stables, textes sans aucune valeur de donnée) et un
//! **niveau de confiance**. Seul un verdict `Automatique` n'attend pas de décision, et seule une identité
//! forte (INE unique des deux côtés, sans contradiction) ou une décision humaine rejouée peut le produire.

use serde::Serialize;

/// Niveau de confiance d'un candidat. L'ordre compte (`Faible < Moyenne < Forte < Certaine`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confiance {
    Faible,
    Moyenne,
    Forte,
    /// Réservée aux verdicts automatiques (identité forte, décision humaine rejouée).
    Certaine,
}

impl Confiance {
    pub fn code(self) -> &'static str {
        match self {
            Confiance::Faible => "faible",
            Confiance::Moyenne => "moyenne",
            Confiance::Forte => "forte",
            Confiance::Certaine => "certaine",
        }
    }

    pub fn depuis(s: &str) -> Option<Confiance> {
        Some(match s {
            "faible" => Confiance::Faible,
            "moyenne" => Confiance::Moyenne,
            "forte" => Confiance::Forte,
            "certaine" => Confiance::Certaine,
            _ => return None,
        })
    }
}

/// Rôle d'une raison dans la proposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarite {
    /// Appuie le rapprochement.
    Pour,
    /// Le contredit ou doit alerter : jamais de décision en lot, jamais d'automatisme.
    Contre,
    /// Information utile à la décision, sans poids.
    Info,
}

impl Polarite {
    pub fn code(self) -> &'static str {
        match self {
            Polarite::Pour => "pour",
            Polarite::Contre => "contre",
            Polarite::Info => "info",
        }
    }
}

macro_rules! raisons {
    ($( $v:ident, $code:literal, $pol:ident, $txt:literal; )+) => {
        /// Pourquoi un candidat est proposé (ou mis en garde). Codes stables : affichés par l'écran,
        /// conservés dans la trace des décisions.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum Raison { $( $v, )+ }

        impl Raison {
            pub const TOUTES: &'static [Raison] = &[ $( Raison::$v, )+ ];
            pub fn code(self) -> &'static str { match self { $( Raison::$v => $code, )+ } }
            pub fn polarite(self) -> Polarite { match self { $( Raison::$v => Polarite::$pol, )+ } }
            /// Phrase affichable, **sans aucune valeur de donnée**.
            pub fn explication(self) -> &'static str { match self { $( Raison::$v => $txt, )+ } }
            pub fn depuis(s: &str) -> Option<Raison> { Self::TOUTES.iter().copied().find(|r| r.code() == s) }
        }
    };
}

raisons! {
    IneIdentique, "ine_identique", Pour, "Même INE des deux côtés.";
    IneUniqueDeuxCotes, "ine_unique_deux_cotes", Pour, "Cet INE n'existe qu'une fois dans l'export et qu'une fois dans ScolIA.";
    NomIdentique, "nom_identique", Pour, "Même nom (accents, tirets et casse ignorés).";
    NomNaissanceIdentique, "nom_naissance_identique", Pour, "Même nom de naissance (le nom d'usage diffère).";
    NomPartiel, "nom_partiel", Pour, "Un nom contient l'autre (nom composé).";
    PrenomIdentique, "prenom_identique", Pour, "Même prénom.";
    PrenomSecondaire, "prenom_secondaire", Pour, "Prénom secondaire ou ordre des prénoms différent.";
    NomPrenomProches, "nom_prenom_proches", Pour, "Nom et prénom très proches (une lettre d'écart) : faute de frappe possible.";
    NaissanceIdentique, "naissance_identique", Pour, "Même date de naissance.";
    MemeClasse, "meme_classe", Pour, "Même classe (libellé reconnu comme équivalent).";
    MemeEleve, "meme_eleve", Pour, "Responsable déjà lié, dans ScolIA, à un élève rapproché de ce foyer.";
    EmailIdentique, "email_identique", Pour, "Même adresse e-mail.";
    EmailProIdentique, "email_pro_identique", Pour, "Même adresse e-mail professionnelle.";
    EmailPersonnelIdentique, "email_personnel_identique", Info, "Même adresse personnelle : simple indice, jamais une clé de rapprochement.";
    TelephoneIdentique, "telephone_identique", Pour, "Même numéro de téléphone.";
    CodeSiecleIdentique, "code_siecle_identique", Pour, "Même code de division SIECLE.";
    CodeIdentique, "code_identique", Pour, "Même code.";
    LibelleIdentique, "libelle_identique", Pour, "Même libellé (casse, accents et synonymes de niveau ignorés).";
    DatesIdentiques, "dates_identiques", Pour, "Mêmes dates de début et de fin.";
    DatesProches, "dates_proches", Pour, "Dates de début et de fin à quelques jours près.";
    SynonymeValide, "synonyme_valide", Pour, "Équivalence déjà validée par l'établissement.";
    NaissanceInconnue, "naissance_inconnue", Info, "Date de naissance absente d'un côté : elle ne confirme ni n'infirme.";
    NaissanceDifferente, "naissance_differente", Contre, "Dates de naissance différentes.";
    IdentiteDiscordante, "identite_discordante", Contre, "Même identifiant mais ni le nom ni le prénom ne correspondent.";
    IneEnDoubleNext, "ine_en_double_next", Contre, "Cet INE figure sur plusieurs fiches de l'export : doublon probable dans le Next.js.";
    IneEnDoubleCoeur, "ine_en_double_coeur", Contre, "Cet INE figure sur plusieurs fiches de ScolIA.";
    IneDifferent, "ine_different", Contre, "INE différents : un des deux est peut-être mal saisi.";
    EmailPartage, "email_partage", Contre, "Adresse partagée par plusieurs personnes : elle ne désigne pas une seule personne.";
    NomDifferent, "nom_different", Contre, "Même identifiant de contact mais nom différent.";
    CibleDejaReliee, "cible_deja_reliee", Contre, "Cette fiche ScolIA est déjà reliée à une autre fiche de l'export.";
    CiblePlusieursSources, "cible_plusieurs_sources", Contre, "Plusieurs fiches de l'export proposent cette même fiche ScolIA.";
    JumeauxPossibles, "jumeaux_possibles", Info, "Même nom et même naissance qu'une autre fiche de l'export, prénom différent : jumeaux possibles, à vérifier.";
    AmbiguiteDansLeCoeur, "ambiguite_dans_le_coeur", Contre, "Plusieurs fiches de ScolIA correspondent aussi bien.";
    AdministrateurPlateforme, "administrateur_plateforme", Contre, "Compte d'administration de la plateforme du Next.js : n'a pas d'équivalent dans l'établissement.";
}

/// Un candidat ScolIA proposé pour une fiche de l'export, avec ce qui le désigne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidat {
    pub cible_id: String,
    pub confiance: Confiance,
    /// Raisons classées (pour, contre, info), sans doublon.
    pub raisons: Vec<Raison>,
    /// Ce qui sert à décider (minimisé) ; jamais l'INE entier ni l'adresse entière.
    pub resume: Resume,
}

impl Candidat {
    pub fn a_contre(&self) -> bool {
        self.raisons
            .iter()
            .any(|r| r.polarite() == Polarite::Contre)
    }
}

/// Résumé **minimisé** d'une fiche, pour la file « à décider » : uniquement ce qui permet de distinguer
/// deux personnes. Jamais d'adresse postale, de contact complet, de santé, d'identifiant complet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Resume {
    pub nom: Option<String>,
    pub prenom: Option<String>,
    /// Date au format `AAAA-MM-JJ`.
    pub naissance: Option<String>,
    pub classe: Option<String>,
    /// Fin de l'INE (« …890A »).
    pub ine_fin: Option<String>,
    /// Adresse masquée (« j***@domaine.fr »).
    pub contact_masque: Option<String>,
    /// Précision courte : catégorie, libellé de site, dates d'une année…
    pub detail: Option<String>,
}

/// Motif pour lequel une ligne reste « à décider ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Motif {
    /// Aucun candidat : seule une décision peut créer la fiche (le cœur fait foi) ou l'ignorer.
    AucunCandidat,
    /// Un candidat, sans contradiction, mais sans identité forte.
    Incertain,
    /// Plusieurs candidats également plausibles.
    Ambigu,
    /// Un candidat, mais une contradiction (naissance, INE en double…) : à regarder de près.
    Contradiction,
}

impl Motif {
    pub fn code(self) -> &'static str {
        match self {
            Motif::AucunCandidat => "aucun_candidat",
            Motif::Incertain => "rapprochement_incertain",
            Motif::Ambigu => "rapprochement_ambigu",
            Motif::Contradiction => "rapprochement_contradictoire",
        }
    }
}

/// Résultat du rapprochement d'une fiche de l'export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Verdict {
    /// Rapprochée sans attendre : identité forte, jamais une ressemblance.
    Automatique {
        cible_id: String,
        regle: &'static str,
        raisons: Vec<Raison>,
    },
    /// Rien à reprendre dans le cœur (élève sorti, année passée absente) : conservée en archive.
    Archive { raison: &'static str },
    /// Attend la décision d'une personne habilitée.
    ADecider {
        motif: Motif,
        candidats: Vec<Candidat>,
        resume_source: Resume,
    },
}

/// Classe un ensemble de candidats : motif de la ligne « à décider ».
pub fn motif_de(candidats: &[Candidat]) -> Motif {
    match candidats {
        [] => Motif::AucunCandidat,
        [c] if c.a_contre() => Motif::Contradiction,
        [_] => Motif::Incertain,
        [a, b, ..] if a.confiance == b.confiance => Motif::Ambigu,
        [a, ..] if a.a_contre() => Motif::Contradiction,
        _ => Motif::Incertain,
    }
}

/// Trie : confiance décroissante, puis identifiant (déterministe) ; limite à `max`.
pub fn classer(mut candidats: Vec<Candidat>, max: usize) -> Vec<Candidat> {
    for c in &mut candidats {
        c.raisons.sort();
        c.raisons.dedup();
    }
    candidats.sort_by(|a, b| {
        b.confiance
            .cmp(&a.confiance)
            .then_with(|| a.a_contre().cmp(&b.a_contre()))
            .then_with(|| a.cible_id.cmp(&b.cible_id))
    });
    candidats.truncate(max);
    candidats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str, conf: Confiance, raisons: &[Raison]) -> Candidat {
        Candidat {
            cible_id: id.into(),
            confiance: conf,
            raisons: raisons.to_vec(),
            resume: Resume::default(),
        }
    }

    #[test]
    fn codes_de_raisons_uniques_et_relisibles() {
        let mut vus = std::collections::BTreeSet::new();
        for r in Raison::TOUTES {
            assert!(vus.insert(r.code()), "code en double : {}", r.code());
            assert_eq!(Raison::depuis(r.code()), Some(*r));
            assert!(!r.explication().is_empty());
            assert!(r.code().chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
        assert_eq!(Raison::depuis("inconnue"), None);
    }

    #[test]
    fn les_explications_ne_portent_aucune_valeur() {
        for r in Raison::TOUTES {
            assert!(
                !r.explication().chars().any(|c| c.is_ascii_digit()),
                "{r:?}"
            );
            assert!(!r.explication().contains('@'));
        }
    }

    #[test]
    fn confiance_ordre_et_aller_retour() {
        assert!(Confiance::Faible < Confiance::Moyenne && Confiance::Forte < Confiance::Certaine);
        for x in [
            Confiance::Faible,
            Confiance::Moyenne,
            Confiance::Forte,
            Confiance::Certaine,
        ] {
            assert_eq!(Confiance::depuis(x.code()), Some(x));
        }
    }

    #[test]
    fn motifs() {
        assert_eq!(motif_de(&[]), Motif::AucunCandidat);
        assert_eq!(
            motif_de(&[c("a", Confiance::Forte, &[Raison::NomIdentique])]),
            Motif::Incertain
        );
        assert_eq!(
            motif_de(&[c("a", Confiance::Forte, &[Raison::NaissanceDifferente])]),
            Motif::Contradiction
        );
        assert_eq!(
            motif_de(&[
                c("a", Confiance::Forte, &[Raison::NomIdentique]),
                c("b", Confiance::Forte, &[Raison::NomIdentique])
            ]),
            Motif::Ambigu
        );
        assert_eq!(
            motif_de(&[
                c("a", Confiance::Forte, &[Raison::NomIdentique]),
                c("b", Confiance::Faible, &[Raison::NomIdentique])
            ]),
            Motif::Incertain
        );
    }

    #[test]
    fn classement_deterministe_et_borne() {
        let v = classer(
            vec![
                c(
                    "z",
                    Confiance::Moyenne,
                    &[Raison::NomIdentique, Raison::NomIdentique],
                ),
                c("b", Confiance::Forte, &[]),
                c("a", Confiance::Forte, &[]),
            ],
            2,
        );
        assert_eq!(
            v.iter().map(|x| x.cible_id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        let v = classer(
            vec![c(
                "z",
                Confiance::Moyenne,
                &[Raison::NomIdentique, Raison::NomIdentique],
            )],
            5,
        );
        assert_eq!(v[0].raisons, vec![Raison::NomIdentique]);
    }
}
