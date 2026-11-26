//! Logique d'écran **pure** du guichet des demandes de droits (socle 27, lot G3a) : libellés, tri, filtres,
//! contrôles de saisie locaux. Aucune dépendance à Leptos ni au réseau ; testée en natif.
//!
//! Aucune **règle métier** ici (`FRONTEND_LEPTOS.md` §2) : les droits, les délais et les décisions sont
//! tranchés par `ent-api` / `ent-db`. Ce fichier ne fait que rendre lisible ce que le serveur a décidé, et
//! éviter d'envoyer une requête qu'on sait incomplète.

use ent_web_api::rgpd_droits_types::{DemandeDto, DemandeLigneDto, SuiviDemandeDto};

/// Libellé d'un état de demande, en clair.
pub fn libelle_etat(etat: &str) -> &'static str {
    match etat {
        "recue" => "Reçue",
        "identite_a_verifier" => "Identité à vérifier",
        "en_cours" => "En cours",
        "a_relire" => "À relire",
        "prete" => "Prête",
        "remise" => "Remise",
        "close" => "Close",
        "refusee" => "Refusée",
        _ => "État inconnu",
    }
}

/// Libellé d'un droit exercé.
pub fn libelle_droit(droit: &str) -> &'static str {
    match droit {
        "acces" => "Accès",
        "rectification" => "Rectification",
        "effacement" => "Effacement",
        "limitation" => "Limitation",
        "opposition" => "Opposition",
        "portabilite" => "Portabilité",
        "consultations" => "Qui a consulté",
        _ => "Droit inconnu",
    }
}

/// Libellé d'une décision prise sur un droit.
pub fn libelle_decision(decision: &str) -> &'static str {
    match decision {
        "en_examen" => "En examen",
        "accorde" => "Accordé",
        "partiel" => "Partiel",
        "refuse" => "Refusé",
        "sans_objet" => "Sans objet",
        _ => "Décision inconnue",
    }
}

/// Libellé d'un canal d'arrivée.
pub fn libelle_canal(canal: &str) -> &'static str {
    match canal {
        "en_ligne" => "En ligne",
        "courrier" => "Courrier",
        "courriel" => "Courriel",
        "guichet" => "Au guichet",
        "dpo" => "Transmise par le DPO",
        "cnil" => "Transmise par la CNIL",
        _ => "Canal inconnu",
    }
}

/// Libellé d'une qualité invoquée par le demandeur.
pub fn libelle_qualite(q: &str) -> &'static str {
    match q {
        "soi_meme" => "Pour elle-même",
        "eleve_mineur" => "Élève mineur, pour lui-même",
        "titulaire_autorite" => "Titulaire de l'autorité parentale",
        "parent_sans_autorite" => "Parent sans exercice de l'autorité",
        "representant_majeur_protege" => "Représentant d'un majeur protégé",
        "mandataire" => "Mandataire (mandat exigé)",
        "ancien_eleve" => "Ancien élève",
        "personnel" => "Personnel de l'établissement",
        "tiers" => "Tiers",
        _ => "Qualité inconnue",
    }
}

/// Libellé d'une qualification.
pub fn libelle_qualification(q: &str) -> &'static str {
    match q {
        "a_qualifier" => "À qualifier",
        "rgpd" => "RGPD",
        "cada" => "Document administratif (CADA)",
        "document" => "Simple demande de document",
        _ => "Qualification inconnue",
    }
}

/// Libellé d'un mode de vérification d'identité.
pub fn libelle_mode_identite(m: &str) -> &'static str {
    match m {
        "session_forte" => "Connexion de la personne",
        "code_courriel" => "Code envoyé au courriel du dossier",
        "piece" => "Pièce d'identité consultée",
        "mandat" => "Mandat",
        _ => "Mode inconnu",
    }
}

/// Libellé de la qualité constatée par le socle 4.
pub fn libelle_qualite_constatee(q: &str) -> &'static str {
    match q {
        "confirmee" => "Qualité confirmée",
        "confirmee_sans_autorite" => "Lien confirmé, sans exercice de l'autorité",
        "restreinte" => "Protection au dossier : arbitrage de la direction",
        "absente" => "Aucun lien à la date de la demande",
        "non_verifiable" => "Qualité non vérifiable : pièce ou mandat à apprécier",
        "sans_objet" => "Sans objet (la personne demande pour elle-même)",
        _ => "Qualité inconnue",
    }
}

/// Libellé d'un événement du journal.
pub fn libelle_evenement(e: &str) -> &'static str {
    match e {
        "recue" => "Demande reçue",
        "qualifiee" => "Qualifiée",
        "identite_demandee" => "Précision d'identité demandée",
        "identite_verifiee" => "Identité vérifiée",
        "identite_echouee" => "Identité non établie",
        "affectee" => "Affectation",
        "prolongee" => "Délai prolongé",
        "droit_ajoute" => "Droit ajouté",
        "droit_decide" => "Décision sur un droit",
        "titulaires_informes" => "Titulaires de l'autorité",
        "close" => "Clôturée",
        "refusee" => "Refusée",
        "rouverte" => "Rouverte",
        "piece_destruction_demandee" => "Destruction de la pièce demandée",
        "piece_destruction_en_attente" => "Destruction de la pièce en attente",
        "piece_detruite" => "Pièce d'identité détruite",
        "numero_attribue" => "Numéro attribué",
        "sujet_rapproche" => "Rapprochement d'un dossier",
        "identite_a_arbitrer" => "Qualité à arbitrer",
        "identite_arbitree" => "Qualité arbitrée par la direction",
        "decision_sans_identite" => "Décision prise sans identité établie (direction)",
        "accuse_envoye" => "Accusé de réception envoyé",
        "echeance_alertee" => "Alerte d'échéance",
        _ => "Événement",
    }
}

/// Teinte d'une pastille d'échéance : rouge en retard, orange à l'approche, neutre sinon. Le seuil
/// d'approche vient du serveur (S-544) ; la fonction ne décide rien, elle colore.
pub fn teinte_echeance(jours_restants: i64, en_retard: bool, seuil: i64) -> &'static str {
    if en_retard {
        "mal"
    } else if jours_restants <= seuil {
        "avert"
    } else {
        "neutre"
    }
}

/// Phrase d'échéance pour le guichet : « en retard de 3 jours », « 10 jours », « aujourd'hui ».
pub fn phrase_echeance(jours_restants: i64, en_retard: bool) -> String {
    match (en_retard, jours_restants) {
        (true, j) => {
            let n = -j;
            if n == 1 {
                "en retard d'un jour".into()
            } else {
                format!("en retard de {n} jours")
            }
        }
        (false, 0) => "échéance aujourd'hui".into(),
        (false, 1) => "demain".into(),
        (false, j) => format!("dans {j} jours"),
    }
}

/// Filtres de la file du guichet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filtre {
    #[default]
    Toutes,
    EnRetard,
    EcheanceProche,
    IdentiteAVerifier,
    AQualifier,
    Closes,
}

impl Filtre {
    pub fn code(self) -> &'static str {
        match self {
            Self::Toutes => "toutes",
            Self::EnRetard => "retard",
            Self::EcheanceProche => "proche",
            Self::IdentiteAVerifier => "identite",
            Self::AQualifier => "qualifier",
            Self::Closes => "closes",
        }
    }

    pub fn depuis_code(c: &str) -> Self {
        match c {
            "retard" => Self::EnRetard,
            "proche" => Self::EcheanceProche,
            "identite" => Self::IdentiteAVerifier,
            "qualifier" => Self::AQualifier,
            "closes" => Self::Closes,
            _ => Self::Toutes,
        }
    }
}

fn est_close(l: &DemandeLigneDto) -> bool {
    matches!(l.etat.as_str(), "close" | "refusee")
}

/// Applique un filtre et, si elle n'est pas vide, une recherche sur la **référence** de la demande (jamais
/// sur un nom : la file n'en porte pas, R-RG32).
pub fn filtrer(
    lignes: &[DemandeLigneDto],
    filtre: Filtre,
    recherche: &str,
    seuil: i64,
) -> Vec<DemandeLigneDto> {
    let q = recherche.trim().to_lowercase();
    lignes
        .iter()
        .filter(|l| match filtre {
            Filtre::Toutes => !est_close(l),
            Filtre::EnRetard => l.en_retard && !est_close(l),
            Filtre::EcheanceProche => !est_close(l) && !l.en_retard && l.jours_restants <= seuil,
            Filtre::IdentiteAVerifier => l.etat == "identite_a_verifier",
            Filtre::AQualifier => l.qualification == "a_qualifier" && !est_close(l),
            Filtre::Closes => est_close(l),
        })
        .filter(|l| q.is_empty() || l.reference.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

/// Les droits restés sans décision : la clôture les exige (contrôle local, le serveur refuse de toute
/// façon).
pub fn droits_en_examen(d: &DemandeDto) -> Vec<&str> {
    d.droits
        .iter()
        .filter(|x| x.decision == "en_examen")
        .map(|x| x.droit.as_str())
        .collect()
}

/// Pourquoi la clôture n'est pas encore possible, en une phrase ; `None` quand elle l'est.
pub fn obstacle_cloture(d: &DemandeDto) -> Option<String> {
    if matches!(d.etat.as_str(), "close" | "refusee") {
        return Some("Cette demande est close.".into());
    }
    let restants = droits_en_examen(d);
    if restants.is_empty() {
        return None;
    }
    let liste = restants
        .iter()
        .map(|c| libelle_droit(c))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "Chaque droit exercé reçoit une décision avant la clôture. Reste : {liste}."
    ))
}

/// Un refus ou un partiel exige un article **et** un texte : évite un aller-retour inutile.
pub fn decision_complete(decision: &str, motif_code: &str, motif_texte: &str) -> bool {
    if !matches!(decision, "refuse" | "partiel") {
        return !decision.is_empty();
    }
    !motif_code.trim().is_empty() && motif_texte.trim().len() >= 3
}

/// Longueur minimale d'un motif accepté par le serveur.
pub const MOTIF_MIN: usize = 3;

/// Le motif saisi est-il recevable localement ?
pub fn motif_recevable(motif: &str) -> bool {
    motif.trim().chars().count() >= MOTIF_MIN
}

/// Une saisie de dépôt est-elle complète ? Au moins un droit, et l'élève désigné quand la demande porte
/// sur **l'enfant d'un titulaire** (le serveur ne peut pas le deviner). Pour une qualité « pour moi-même »
/// (`soi_meme`, `eleve_mineur`, `personnel`, `ancien_eleve`), le serveur résout le sujet depuis le compte :
/// l'usager n'a aucun identifiant interne à saisir.
pub fn depot_complet(qualite: &str, sujet_id_renseigne: bool, droits: &[String]) -> bool {
    if droits.is_empty() {
        return false;
    }
    match qualite {
        "" => false,
        "titulaire_autorite" => sujet_id_renseigne,
        _ => true,
    }
}

/// Référence de la pièce d'identité saisie au guichet (F7) : vide → aucune ; un identifiant valide →
/// la pièce ; autre chose → `Err` (on n'envoie pas une référence qu'on sait fausse). Le serveur valide
/// ensuite que c'est bien une pièce d'identité de l'établissement.
pub fn piece_depuis_texte(texte: &str) -> Result<Option<uuid::Uuid>, uuid::Error> {
    let t = texte.trim();
    if t.is_empty() {
        return Ok(None);
    }
    uuid::Uuid::parse_str(t).map(Some)
}

/// Un dépôt **public** (sans compte) est-il complet ? Nom, moyen de contact, personne concernée et au moins
/// un droit : le serveur borne ensuite les tailles et les qualités.
pub fn depot_public_complet(nom: &str, contact: &str, personne: &str, droits: &[String]) -> bool {
    nom.trim().chars().count() >= 2
        && contact.trim().chars().count() >= 3
        && personne.trim().chars().count() >= 2
        && !droits.is_empty()
}

/// Journal du plus récent au plus ancien (le guichet lit d'abord ce qui vient de se passer).
pub fn journal_recent_d_abord<T>(v: &[T]) -> Vec<T>
where
    T: Clone + JournalHorodate,
{
    let mut l = v.to_vec();
    l.sort_by_key(|a| std::cmp::Reverse(a.instant()));
    l
}

/// Ce dont le tri du journal a besoin : son instant.
pub trait JournalHorodate {
    fn instant(&self) -> chrono::DateTime<chrono::Utc>;
}

impl JournalHorodate for ent_web_api::rgpd_droits_types::JournalDto {
    fn instant(&self) -> chrono::DateTime<chrono::Utc> {
        self.le
    }
}

/// Suivi du demandeur : la plus récente d'abord, les demandes closes après les ouvertes.
pub fn trier_suivi(mut v: Vec<SuiviDemandeDto>) -> Vec<SuiviDemandeDto> {
    v.sort_by(|a, b| {
        let close = |s: &SuiviDemandeDto| s.close_le.is_some();
        close(a)
            .cmp(&close(b))
            .then(b.recue_le.cmp(&a.recue_le))
            .then(b.reference.cmp(&a.reference))
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// Identifiant stable de test : la feature `v4` d'`uuid` n'est pas activée côté frontend
    /// (`FRONTEND_LEPTOS.md` §11), et un identifiant figé rend les assertions reproductibles.
    fn ident(n: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(n)
    }

    fn ligne(reference: &str, etat: &str, qualification: &str, jours: i64) -> DemandeLigneDto {
        DemandeLigneDto {
            id: ident(jours.unsigned_abs() as u128 + 1),
            reference: reference.into(),
            canal: "courrier".into(),
            recue_le: NaiveDate::from_ymd_opt(2027, 3, 3).unwrap(),
            echeance_effective: NaiveDate::from_ymd_opt(2027, 4, 3).unwrap(),
            jours_restants: jours,
            en_retard: jours < 0,
            etat: etat.into(),
            qualification: qualification.into(),
            qualite: "titulaire_autorite".into(),
            sujet_type: "eleve".into(),
            gestionnaire_personne_id: None,
            droits: vec!["acces".into()],
            saisie_tardive: false,
        }
    }

    fn jeu() -> Vec<DemandeLigneDto> {
        vec![
            ligne("2027-001", "en_cours", "rgpd", -4),
            ligne("2027-002", "en_cours", "rgpd", 5),
            ligne("2027-003", "identite_a_verifier", "a_qualifier", 20),
            ligne("2027-004", "close", "rgpd", 40),
            ligne("2027-005", "refusee", "rgpd", 40),
        ]
    }

    #[test]
    fn libelles_sans_trou() {
        for e in [
            "recue",
            "identite_a_verifier",
            "en_cours",
            "a_relire",
            "prete",
            "remise",
            "close",
            "refusee",
        ] {
            assert_ne!(libelle_etat(e), "État inconnu", "{e}");
        }
        for d in [
            "acces",
            "rectification",
            "effacement",
            "limitation",
            "opposition",
            "portabilite",
            "consultations",
        ] {
            assert_ne!(libelle_droit(d), "Droit inconnu", "{d}");
        }
        for c in ["en_ligne", "courrier", "courriel", "guichet", "dpo", "cnil"] {
            assert_ne!(libelle_canal(c), "Canal inconnu", "{c}");
        }
        for q in [
            "soi_meme",
            "eleve_mineur",
            "titulaire_autorite",
            "parent_sans_autorite",
            "representant_majeur_protege",
            "mandataire",
            "ancien_eleve",
            "personnel",
            "tiers",
        ] {
            assert_ne!(libelle_qualite(q), "Qualité inconnue", "{q}");
        }
        // Un code que le serveur ajouterait plus tard ne casse pas l'écran.
        assert_eq!(libelle_etat("inventé"), "État inconnu");
    }

    #[test]
    fn filtre_toutes_exclut_les_closes() {
        let l = filtrer(&jeu(), Filtre::Toutes, "", 10);
        assert_eq!(l.len(), 3);
        assert!(!l.iter().any(|x| x.etat == "close" || x.etat == "refusee"));
    }

    #[test]
    fn filtres_par_etat_et_echeance() {
        let j = jeu();
        assert_eq!(filtrer(&j, Filtre::EnRetard, "", 10).len(), 1);
        let proches = filtrer(&j, Filtre::EcheanceProche, "", 10);
        assert_eq!(proches.len(), 1, "seule 2027-002 est à 5 jours");
        assert_eq!(proches[0].reference, "2027-002");
        assert_eq!(filtrer(&j, Filtre::IdentiteAVerifier, "", 10).len(), 1);
        assert_eq!(filtrer(&j, Filtre::AQualifier, "", 10).len(), 1);
        assert_eq!(filtrer(&j, Filtre::Closes, "", 10).len(), 2);
        // Le seuil vient du serveur : à 30 jours, la demande à 20 jours devient « proche ».
        assert_eq!(filtrer(&j, Filtre::EcheanceProche, "", 30).len(), 2);
    }

    #[test]
    fn recherche_sur_la_reference_seulement() {
        let j = jeu();
        assert_eq!(filtrer(&j, Filtre::Toutes, "2027-002", 10).len(), 1);
        assert_eq!(filtrer(&j, Filtre::Toutes, "  2027-00  ", 10).len(), 3);
        assert_eq!(filtrer(&j, Filtre::Toutes, "granger", 10).len(), 0);
    }

    #[test]
    fn codes_de_filtre_aller_retour() {
        for f in [
            Filtre::Toutes,
            Filtre::EnRetard,
            Filtre::EcheanceProche,
            Filtre::IdentiteAVerifier,
            Filtre::AQualifier,
            Filtre::Closes,
        ] {
            assert_eq!(Filtre::depuis_code(f.code()), f);
        }
        assert_eq!(Filtre::depuis_code("n'importe quoi"), Filtre::Toutes);
    }

    #[test]
    fn teintes_et_phrases_d_echeance() {
        assert_eq!(teinte_echeance(-1, true, 10), "mal");
        assert_eq!(teinte_echeance(3, false, 10), "avert");
        assert_eq!(teinte_echeance(30, false, 10), "neutre");
        assert_eq!(phrase_echeance(-1, true), "en retard d'un jour");
        assert_eq!(phrase_echeance(-5, true), "en retard de 5 jours");
        assert_eq!(phrase_echeance(0, false), "échéance aujourd'hui");
        assert_eq!(phrase_echeance(1, false), "demain");
        assert_eq!(phrase_echeance(12, false), "dans 12 jours");
    }

    fn demande(etat: &str, decisions: &[(&str, &str)]) -> DemandeDto {
        use ent_web_api::rgpd_droits_types::DroitDecisionDto;
        DemandeDto {
            id: uuid::Uuid::nil(),
            reference: "2027-001".into(),
            canal: "courrier".into(),
            recue_le: NaiveDate::from_ymd_opt(2027, 3, 3).unwrap(),
            saisie_le: chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
            saisie_tardive: false,
            echeance_le: NaiveDate::from_ymd_opt(2027, 4, 3).unwrap(),
            prolongee_jusqu_au: None,
            prolongation_motif: None,
            echeance_effective: NaiveDate::from_ymd_opt(2027, 4, 3).unwrap(),
            jours_restants: 10,
            en_retard: false,
            qualite: "titulaire_autorite".into(),
            demandeur_nom: Some("Famille Granger".into()),
            demandeur_a_un_compte: false,
            sujet_type: "eleve".into(),
            sujet_id: None,
            personne_concernee: None,
            qualification: "rgpd".into(),
            qualification_motif: None,
            perimetre_texte: None,
            perimetre_du: None,
            perimetre_au: None,
            etat: etat.into(),
            gestionnaire_personne_id: None,
            identite_demandee_le: None,
            identite_fournie_le: None,
            identite_attente_jours: 0,
            reouvertures: 0,
            close_le: None,
            reponse_resume: None,
            refus_motif: None,
            titulaires_informes_le: None,
            titulaires_non_informes_motif: None,
            droits: decisions
                .iter()
                .map(|(d, dec)| DroitDecisionDto {
                    droit: (*d).into(),
                    decision: (*dec).into(),
                    motif_code: None,
                    motif_texte: None,
                    decide_le: None,
                })
                .collect(),
            identites: vec![],
            journal: vec![],
            piece_a_detruire: false,
            destruction_en_attente: false,
            identite_arbitrage_code: None,
            titulaires_a_decider: false,
        }
    }

    #[test]
    fn reference_de_piece_saisie() {
        assert_eq!(piece_depuis_texte("  "), Ok(None));
        let id = uuid::Uuid::from_u128(0x1234_5678);
        assert_eq!(piece_depuis_texte(&format!(" {id} ")), Ok(Some(id)));
        assert!(piece_depuis_texte("pas un identifiant").is_err());
    }

    #[test]
    fn depot_public_exige_nom_contact_personne_et_droit() {
        let d = vec!["acces".to_string()];
        assert!(depot_public_complet(
            "Marie Dupont",
            "marie@exemple.test",
            "Léa Dupont",
            &d
        ));
        assert!(!depot_public_complet(
            "M",
            "marie@exemple.test",
            "Léa Dupont",
            &d
        ));
        assert!(!depot_public_complet("Marie Dupont", "", "Léa Dupont", &d));
        assert!(!depot_public_complet(
            "Marie Dupont",
            "marie@exemple.test",
            " ",
            &d
        ));
        assert!(!depot_public_complet(
            "Marie Dupont",
            "marie@exemple.test",
            "Léa Dupont",
            &[]
        ));
    }

    #[test]
    fn libelles_des_evenements_du_correctif() {
        for e in [
            "piece_destruction_demandee",
            "piece_destruction_en_attente",
            "numero_attribue",
            "sujet_rapproche",
            "identite_a_arbitrer",
            "identite_arbitree",
            "decision_sans_identite",
        ] {
            assert_ne!(libelle_evenement(e), "Événement", "{e}");
        }
        assert_ne!(
            libelle_qualite_constatee("non_verifiable"),
            "Qualité inconnue"
        );
    }

    #[test]
    fn obstacle_cloture_nomme_les_droits_restants() {
        let d = demande(
            "en_cours",
            &[("acces", "accorde"), ("effacement", "en_examen")],
        );
        let m = obstacle_cloture(&d).expect("un obstacle");
        assert!(m.contains("Effacement"), "{m}");
        assert!(!m.contains("Accès"), "{m}");
        let ok = demande("en_cours", &[("acces", "accorde")]);
        assert!(obstacle_cloture(&ok).is_none());
        let close = demande("close", &[("acces", "accorde")]);
        assert!(obstacle_cloture(&close).is_some());
    }

    #[test]
    fn decision_refusee_exige_article_et_texte() {
        assert!(decision_complete("accorde", "", ""));
        assert!(!decision_complete("", "", ""));
        assert!(!decision_complete("refuse", "", "parce que"));
        assert!(!decision_complete("refuse", "art17_3_b", "x"));
        assert!(decision_complete(
            "refuse",
            "art17_3_b",
            "conservation imposée par un texte"
        ));
        assert!(decision_complete(
            "partiel",
            "partiel",
            "une partie seulement"
        ));
    }

    #[test]
    fn motif_borne_comme_le_serveur() {
        assert!(!motif_recevable("  "));
        assert!(!motif_recevable("ab"));
        assert!(motif_recevable("abc"));
        assert!(motif_recevable("  un motif  "));
    }

    #[test]
    fn depot_exige_un_droit_et_un_sujet_quand_il_faut() {
        let acces = vec!["acces".to_string()];
        assert!(!depot_complet("titulaire_autorite", true, &[]));
        assert!(!depot_complet("", true, &acces));
        assert!(!depot_complet("titulaire_autorite", false, &acces));
        assert!(depot_complet("titulaire_autorite", true, &acces));
        // Pour soi-même, aucun identifiant à saisir : le serveur résout le sujet depuis le compte.
        assert!(depot_complet("eleve_mineur", false, &acces));
        assert!(depot_complet("soi_meme", false, &acces));
        assert!(depot_complet("personnel", false, &acces));
        assert!(depot_complet("ancien_eleve", false, &acces));
    }

    #[test]
    fn journal_du_plus_recent_au_plus_ancien() {
        use ent_web_api::rgpd_droits_types::JournalDto;
        let l = |s: i64, e: &str| JournalDto {
            evenement: e.into(),
            detail: None,
            par_role: None,
            le: chrono::DateTime::from_timestamp(s, 0).unwrap(),
        };
        let v = vec![l(100, "recue"), l(300, "close"), l(200, "qualifiee")];
        let t = journal_recent_d_abord(&v);
        assert_eq!(
            t.iter().map(|x| x.evenement.as_str()).collect::<Vec<_>>(),
            vec!["close", "qualifiee", "recue"]
        );
    }

    #[test]
    fn suivi_trie_ouvertes_d_abord_puis_recentes() {
        let s = |r: &str, j: u32, close: bool| SuiviDemandeDto {
            id: ident(u128::from(j)),
            reference: r.into(),
            recue_le: NaiveDate::from_ymd_opt(2027, 3, j).unwrap(),
            echeance_effective: NaiveDate::from_ymd_opt(2027, 4, j).unwrap(),
            etat: if close {
                "close".into()
            } else {
                "en_cours".into()
            },
            avancement: String::new(),
            droits: vec![],
            prolongee_jusqu_au: None,
            prolongation_motif: None,
            close_le: close.then(|| chrono::DateTime::from_timestamp(1, 0).unwrap()),
            reponse_resume: None,
        };
        let t = trier_suivi(vec![
            s("2027-001", 1, true),
            s("2027-002", 2, false),
            s("2027-003", 5, false),
        ]);
        assert_eq!(
            t.iter().map(|x| x.reference.as_str()).collect::<Vec<_>>(),
            vec!["2027-003", "2027-002", "2027-001"]
        );
    }
}
