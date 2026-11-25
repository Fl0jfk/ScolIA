//! Catalogue des codes d'erreur de l'API (socle 11, lot MB1, `docs/socles/11_APPLICATIONS_MOBILES.md` §5.3).
//!
//! Une erreur servie à une appli mobile (en-tête `X-ScolIA-Appli`, routes `/mobile/*`) a toujours la forme
//! `{ "erreur": { code, message, champs, reessayer_apres_s, trace } }`. Le `code` (`domaine.cause`) est la seule chose
//! que l'appli interprète ; le message est un texte d'affichage. Un code ne change jamais de sens ; un code retiré est
//! remplacé, jamais réutilisé. Aucune dépendance serveur (compile pour WebAssembly).

use serde::{Deserialize, Serialize};

macro_rules! codes {
    ($( $(#[$m:meta])* $variante:ident => $texte:literal, $statut:literal, $neutre:literal; )+) => {
        /// Code d'erreur stable, sérialisé en texte `domaine.cause`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum CodeErreur {
            $( $(#[$m])* #[serde(rename = $texte)] $variante, )+
        }

        impl CodeErreur {
            /// Tous les codes du catalogue (pour les tests de non-régression et l'affichage des fixtures).
            pub const TOUS: &'static [CodeErreur] = &[ $( CodeErreur::$variante, )+ ];

            /// Identifiant texte `domaine.cause`.
            pub const fn as_str(self) -> &'static str {
                match self { $( CodeErreur::$variante => $texte, )+ }
            }

            /// Statut HTTP habituel du code.
            pub const fn statut_http(self) -> u16 {
                match self { $( CodeErreur::$variante => $statut, )+ }
            }

            /// Message d'affichage par défaut, sans aucune donnée (jamais la cause d'un refus d'accès à une personne).
            pub const fn message_neutre(self) -> &'static str {
                match self { $( CodeErreur::$variante => $neutre, )+ }
            }

            /// Retrouve un code depuis son texte.
            pub fn depuis_texte(texte: &str) -> Option<CodeErreur> {
                match texte { $( $texte => Some(CodeErreur::$variante), )+ _ => None }
            }
        }
    };
}

codes! {
    // --- Session et appareil (401, 403) -----------------------------------------------------------------------------
    /// Session absente ou expirée : l'appli purge ses baux et revient à la connexion.
    SessionExpiree => "session.expiree", 401, "Votre session a expiré. Reconnectez-vous.";
    /// Session révoquée (déconnexion, appareil perdu, accès fermé).
    SessionRevoquee => "session.revoquee", 401, "Votre session a été fermée. Reconnectez-vous.";
    /// Échec d'identification sur une route de connexion (aucune indication sur le compte).
    AuthRefusee => "auth.refusee", 401, "Identification impossible.";
    /// Action non permise sur une cible dont l'existence est déjà connue de la personne.
    DroitRefuse => "droit.refuse", 403, "Cette action n'est pas disponible.";
    /// Geste ★ sans signature de présence (lot MB4).
    PresenceRequise => "presence.requise", 403, "Une confirmation de votre présence est nécessaire.";
    /// Écriture sans signature d'appareil valide (lot MB4).
    AppareilNonLie => "appareil.non_lie", 403, "Cet appareil doit être de nouveau associé à votre compte.";

    // --- Requête mal formée (400) -----------------------------------------------------------------------------------
    RequeteInvalide => "requete.invalide", 400, "La requête est incorrecte.";
    ChampRequis => "champ.requis", 400, "Un champ obligatoire est manquant.";
    ChampInvalide => "champ.invalide", 400, "Une valeur est incorrecte.";
    /// `Idempotency-Key` absent sur une écriture authentifiée venant d'une appli.
    IdempotenceRequise => "idempotence.requise", 400, "L'identifiant d'idempotence de l'écriture est manquant.";
    /// `Idempotency-Key` qui n'est pas un UUID.
    IdempotenceCleInvalide => "idempotence.cle_invalide", 400, "L'identifiant d'idempotence de l'écriture est invalide.";

    // --- Objet introuvable ou en conflit ---------------------------------------------------------------------------
    /// Objet inexistant **ou hors des droits** de la personne : jamais d'oracle d'existence.
    ObjetIntrouvable => "objet.introuvable", 404, "Élément introuvable.";
    /// `If-Match` périmé.
    VersionConflit => "version.conflit", 409, "Cet élément a été modifié entre-temps. Rechargez-le.";
    /// Conflit d'état générique (unicité, état incompatible) sans code plus précis.
    ObjetConflit => "objet.conflit", 409, "L'opération entre en conflit avec l'état actuel.";
    /// Même clé d'idempotence, autre requête : erreur de programmation de l'appli.
    IdempotenceConflit => "idempotence.conflit", 409, "Cet identifiant d'idempotence a déjà servi pour une autre requête.";
    /// Même clé, première requête encore en cours de traitement.
    IdempotenceEnCours => "idempotence.en_cours", 409, "Cette opération est déjà en cours de traitement.";
    /// Même clé, première requête terminée mais sa réponse n'a pas pu être conservée.
    IdempotenceNonRejouable => "idempotence.non_rejouable", 409, "Cette opération a déjà été traitée ; sa réponse n'est plus disponible.";
    PreconditionEchouee => "precondition.echouee", 412, "Une précondition de la requête n'est plus vraie.";

    // --- Règles métier (422) et objets figés (423) ------------------------------------------------------------------
    /// Règle métier non respectée sans code plus précis (le message de la route est conservé).
    RegleNonRespectee => "regle.non_respectee", 422, "L'opération n'est pas permise dans l'état actuel.";
    /// Fenêtre de justification, de saisie ou de correction dépassée.
    FenetreDepassee => "fenetre.depassee", 422, "Le délai prévu pour cette opération est dépassé.";
    /// Période arrêtée (notes, bulletins).
    PeriodeArretee => "periode.arretee", 422, "La période est arrêtée.";
    /// Envoi refusé pendant la plage calme du destinataire.
    PlageCalme => "plage_calme.active", 422, "Cet envoi n'est pas possible pendant la plage calme.";
    /// Liste plus longue que le plafond serveur.
    ListeTropLongue => "liste.trop_longue", 422, "La liste demandée est trop longue.";
    /// Objet figé (appel clos hors fenêtre, bulletin publié).
    ObjetVerrouille => "objet.verrouille", 423, "Cet élément est en lecture seule.";

    // --- Plateforme -------------------------------------------------------------------------------------------------
    CorpsTropGros => "corps.trop_gros", 413, "Le contenu envoyé est trop volumineux.";
    DelaiDepasse => "delai.depasse", 408, "Le délai de traitement est dépassé.";
    /// Version de l'appli sous le minimum supporté.
    AppliTropAncienne => "appli.trop_ancienne", 426, "Mettez l'application à jour pour continuer.";
    DebitDepasse => "debit.depasse", 429, "Trop de requêtes. Réessayez dans un instant.";
    ServiceMaintenance => "service.maintenance", 503, "Service en maintenance. Réessayez plus tard.";
    /// Erreur interne : le détail n'existe que dans le journal, corrélé par `trace`.
    ErreurInterne => "erreur.interne", 500, "Erreur interne.";
}

impl CodeErreur {
    /// Code par défaut d'un statut d'erreur sans code explicite (routes existantes qui renvoient `(StatusCode, String)`).
    pub fn par_defaut(statut: u16) -> CodeErreur {
        match statut {
            400 => CodeErreur::RequeteInvalide,
            401 => CodeErreur::SessionExpiree,
            403 => CodeErreur::DroitRefuse,
            404 => CodeErreur::ObjetIntrouvable,
            408 => CodeErreur::DelaiDepasse,
            409 => CodeErreur::ObjetConflit,
            412 => CodeErreur::PreconditionEchouee,
            413 => CodeErreur::CorpsTropGros,
            422 => CodeErreur::RegleNonRespectee,
            423 => CodeErreur::ObjetVerrouille,
            426 => CodeErreur::AppliTropAncienne,
            429 => CodeErreur::DebitDepasse,
            503 => CodeErreur::ServiceMaintenance,
            s if s >= 500 => CodeErreur::ErreurInterne,
            _ => CodeErreur::RequeteInvalide,
        }
    }

    /// `true` si le message d'une route ne doit **jamais** être reproduit : un refus d'accès ou un objet introuvable
    /// ne dit pas pourquoi (pas d'oracle d'existence, pas de cause d'un refus visant une personne).
    pub const fn message_toujours_neutre(self) -> bool {
        matches!(
            self,
            CodeErreur::SessionExpiree
                | CodeErreur::SessionRevoquee
                | CodeErreur::AuthRefusee
                | CodeErreur::DroitRefuse
                | CodeErreur::PresenceRequise
                | CodeErreur::AppareilNonLie
                | CodeErreur::ObjetIntrouvable
                | CodeErreur::ErreurInterne
        )
    }
}

/// Champ en erreur d'une requête mal formée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErreurChamp {
    pub champ: String,
    pub code: CodeErreur,
}

/// Contenu de l'enveloppe d'erreur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErreurCodee {
    pub code: CodeErreur,
    pub message: String,
    #[serde(default)]
    pub champs: Vec<ErreurChamp>,
    /// Délai conseillé avant de réessayer (429, 503, opération en cours) ; `null` sinon.
    #[serde(default)]
    pub reessayer_apres_s: Option<u32>,
    /// Identifiant de requête (corrélé au journal serveur) : jamais une donnée personnelle.
    pub trace: String,
}

/// Corps JSON d'une erreur codée : `{ "erreur": { … } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnveloppeErreur {
    pub erreur: ErreurCodee,
}

impl EnveloppeErreur {
    pub fn nouvelle(
        code: CodeErreur,
        message: impl Into<String>,
        trace: impl Into<String>,
    ) -> Self {
        Self {
            erreur: ErreurCodee {
                code,
                message: message.into(),
                champs: Vec::new(),
                reessayer_apres_s: None,
                trace: trace.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_codes_sont_uniques_et_au_format_domaine_cause() {
        let mut vus = std::collections::BTreeSet::new();
        for c in CodeErreur::TOUS {
            let t = c.as_str();
            assert!(vus.insert(t), "code en double : {t}");
            let (d, cause) = t.split_once('.').expect("domaine.cause");
            for p in [d, cause] {
                assert!(
                    !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                    "{t}"
                );
            }
            assert!((400..600).contains(&c.statut_http()), "{t}");
            assert!(!c.message_neutre().is_empty(), "{t}");
        }
    }

    #[test]
    fn le_catalogue_exige_par_le_contrat_est_present() {
        for t in [
            "session.expiree",
            "session.revoquee",
            "droit.refuse",
            "objet.introuvable",
            "version.conflit",
            "idempotence.conflit",
            "objet.verrouille",
            "appli.trop_ancienne",
            "debit.depasse",
            "service.maintenance",
            "champ.requis",
            "presence.requise",
            "appareil.non_lie",
        ] {
            assert!(CodeErreur::depuis_texte(t).is_some(), "{t}");
        }
    }

    #[test]
    fn serialisation_en_texte_et_aller_retour() {
        for c in CodeErreur::TOUS {
            let j = serde_json::to_string(c).unwrap();
            assert_eq!(j, format!("\"{}\"", c.as_str()));
            assert_eq!(serde_json::from_str::<CodeErreur>(&j).unwrap(), *c);
            assert_eq!(CodeErreur::depuis_texte(c.as_str()), Some(*c));
        }
    }

    #[test]
    fn les_statuts_par_defaut_ont_un_code_de_meme_statut() {
        for s in [
            400u16, 401, 403, 404, 408, 409, 412, 413, 422, 423, 426, 429, 503, 500, 502,
        ] {
            let c = CodeErreur::par_defaut(s);
            if s == 502 {
                assert_eq!(c, CodeErreur::ErreurInterne);
            } else {
                assert_eq!(c.statut_http(), s, "{s}");
            }
        }
    }

    #[test]
    fn enveloppe_au_format_du_contrat() {
        let e = EnveloppeErreur::nouvelle(CodeErreur::DroitRefuse, "x", "t1");
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["erreur"]["code"], "droit.refuse");
        assert!(v["erreur"]["champs"].as_array().unwrap().is_empty());
        assert!(v["erreur"]["reessayer_apres_s"].is_null());
        assert_eq!(v["erreur"]["trace"], "t1");
    }
}
