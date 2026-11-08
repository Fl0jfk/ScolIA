//! Logique d'écran pure de la fiche élève 360° (sans Leptos ni réseau), testée en natif.
//!
//! Aucune règle métier décisive ici : le serveur reste l'autorité (droits, parts = 100 %,
//! motif de suspension, minimisation). Ces fonctions ne font que préparer l'affichage et
//! éviter un aller-retour évident (champ vide, somme des parts).

use chrono::NaiveDate;
use ent_web_api::{
    ActionFicheDto, AjoutResponsableBody, ContactBody, EntreeChronologieDto, FicheEleveDto,
    LienBody, LienEnAttenteEleveDto, MasquageDto, PersonneBody, PersonneDto, ResponsableDto,
};
use uuid::Uuid;

/// Codes d'action du catalogue serveur utilisés par la fiche.
pub const ACTION_PERSONNES: &str = "personnes.gerer";
pub const ACTION_PAYEURS: &str = "payeurs.definir";
pub const ACTION_SUSPENDRE: &str = "liens.suspendre";

/// L'action est-elle proposée par le serveur pour cette fiche ?
pub fn peut(actions: &[ActionFicheDto], code: &str) -> bool {
    actions.iter().any(|a| a.code == code)
}

/// État d'un lien de responsabilité, pour l'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatLien {
    Actif,
    Suspendu,
    Termine,
    AVenir,
}

impl EtatLien {
    pub fn libelle(self) -> &'static str {
        match self {
            EtatLien::Actif => "Actif",
            EtatLien::Suspendu => "Suspendu",
            EtatLien::Termine => "Terminé",
            EtatLien::AVenir => "À venir",
        }
    }
}

/// État d'un lien : suspension d'abord, puis dates (le drapeau `actif` du serveur fait foi).
pub fn etat_lien(r: &ResponsableDto, jour: NaiveDate) -> EtatLien {
    if r.suspendu_le.is_some() {
        EtatLien::Suspendu
    } else if r.date_fin.is_some_and(|f| f < jour) {
        EtatLien::Termine
    } else if r.date_debut.is_some_and(|d| d > jour) {
        EtatLien::AVenir
    } else if r.actif {
        EtatLien::Actif
    } else {
        // Le serveur dit inactif sans raison visible (personne fusionnée…).
        EtatLien::Suspendu
    }
}

/// Message à afficher à la place d'un groupe de champs fermé (lot A1) : `None` si le groupe est
/// ouvert. Un champ vide n'est jamais ambigu : absent ≠ masqué.
pub fn message_masque(masque: &[MasquageDto], groupe: &str) -> Option<String> {
    masque
        .iter()
        .find(|m| m.groupe == groupe)
        .map(|m| match &m.action_a_demander {
            Some(a) => format!("{} (droit à demander : {a})", m.message),
            None => m.message.clone(),
        })
}

pub fn libelle_garde(g: Option<&str>) -> &'static str {
    match g {
        Some("exclusive") => "Garde exclusive",
        Some("alternee") => "Garde alternée",
        Some("aucune") => "Sans garde",
        _ => "Garde non renseignée",
    }
}

/// « Prénom NOM » d'une personne.
pub fn nom_personne(p: &PersonneDto) -> String {
    match p.prenom.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(pr) => format!("{pr} {}", p.nom.to_uppercase()),
        None => p.nom.to_uppercase(),
    }
}

/// Droits fins accordés par le lien, en clair.
pub fn droits_du_lien(r: &ResponsableDto) -> Vec<&'static str> {
    let mut d = Vec::new();
    if r.peut_voir_notes {
        d.push("notes");
    }
    if r.peut_voir_vs {
        d.push("vie scolaire");
    }
    if r.peut_justifier_absence {
        d.push("justifier");
    }
    if r.peut_autoriser_sorties {
        d.push("sorties");
    }
    d
}

/// Coordonnées affichables (seulement celles que le serveur a envoyées).
pub fn coordonnees(p: &PersonneDto) -> Vec<String> {
    [&p.email, &p.telephone, &p.telephone_2]
        .into_iter()
        .filter_map(|c| c.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .map(str::to_string)
        .collect()
}

/// Libellé d'un domaine de chronologie.
pub fn libelle_domaine(code: &str) -> &'static str {
    match code {
        "dossier" => "Dossier",
        "scolarite" => "Scolarité",
        "vie_scolaire" => "Vie scolaire",
        "notes" => "Évaluations",
        "documents" => "Documents",
        "accompagnement" => "Accompagnement",
        "repas" => "Restauration",
        "finances" => "Finances",
        "sante" => "Santé",
        "psy" => "Psychologie",
        _ => "Autre",
    }
}

/// Domaines présents dans la chronologie, dans un ordre stable, avec leur nombre.
pub fn domaines_presents(entrees: &[EntreeChronologieDto]) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = Vec::new();
    for e in entrees {
        match v.iter_mut().find(|(d, _)| *d == e.domaine) {
            Some((_, n)) => *n += 1,
            None => v.push((e.domaine.clone(), 1)),
        }
    }
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

pub fn filtrer_chronologie(
    entrees: &[EntreeChronologieDto],
    domaine: Option<&str>,
) -> Vec<EntreeChronologieDto> {
    entrees
        .iter()
        .filter(|e| domaine.is_none_or(|d| e.domaine == d))
        .cloned()
        .collect()
}

/// Section existante du dossier accessible depuis la fiche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionFiche {
    pub titre: &'static str,
    pub detail: String,
    /// Écran Leptos existant qui traite la section (sinon : consultation seule).
    pub lien: Option<&'static str>,
}

fn pluriel(n: u32, un: &str, plusieurs: &str) -> String {
    match n {
        0 => format!("aucun {un}"),
        1 => format!("1 {un}"),
        _ => format!("{n} {plusieurs}"),
    }
}

/// Sections visibles par le lecteur (selon le serveur) avec leur contenu résumé.
pub fn sections_accessibles(f: &FicheEleveDto, famille: bool) -> Vec<SectionFiche> {
    let s = &f.sections;
    let r = &f.resume;
    let mut v = Vec::new();
    if s.notes {
        v.push(SectionFiche {
            titre: "Évaluations",
            detail: pluriel(r.notes, "note", "notes"),
            lien: None,
        });
    }
    if s.vs {
        v.push(SectionFiche {
            titre: "Vie scolaire",
            detail: format!(
                "{}, {}, {}",
                pluriel(r.sanctions, "sanction", "sanctions"),
                pluriel(r.incidents, "incident", "incidents"),
                pluriel(r.carnet, "mot du carnet", "mots du carnet")
            ),
            lien: Some(if famille {
                "/web/absences"
            } else {
                "/web/vie-scolaire"
            }),
        });
    }
    if s.documents {
        v.push(SectionFiche {
            titre: "Documents",
            detail: pluriel(r.documents, "document", "documents"),
            lien: None,
        });
    }
    if s.options {
        v.push(SectionFiche {
            titre: "Options",
            detail: pluriel(r.options, "option", "options"),
            lien: None,
        });
    }
    if s.dispositifs {
        v.push(SectionFiche {
            titre: "Accompagnement",
            detail: pluriel(r.dispositifs, "dispositif", "dispositifs"),
            lien: None,
        });
    }
    if s.bourses {
        v.push(SectionFiche {
            titre: "Bourses",
            detail: pluriel(r.bourses, "bourse", "bourses"),
            lien: None,
        });
    }
    v
}

/// Écran existant correspondant à une action du catalogue (si la fiche peut y mener).
pub fn lien_action(code: &str) -> Option<&'static str> {
    match code {
        "absences.lire" | "absences.justifier" => Some("/web/vie-scolaire"),
        "messagerie.eleve" => Some("/web/messages/nouveau"),
        _ => None,
    }
}

/// Saisie d'un nouveau responsable (personne + lien).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieResponsable {
    pub nom: String,
    pub prenom: String,
    pub email: String,
    pub telephone: String,
    pub lien: String,
    pub garde: String,
    pub autorite: bool,
    pub voir_notes: bool,
    pub voir_vs: bool,
    pub justifier: bool,
}

fn texte(s: &str) -> Option<String> {
    Some(s.trim().to_string()).filter(|t| !t.is_empty())
}

impl SaisieResponsable {
    /// Personne à créer : nom et nature du lien obligatoires ; e-mail facultatif (D-020).
    pub fn personne(&self) -> Result<PersonneBody, String> {
        let nom = texte(&self.nom).ok_or("Le nom est obligatoire.")?;
        if texte(&self.lien).is_none() {
            return Err("Indiquez la nature du lien (mère, père, tuteur…).".into());
        }
        if let Some(e) = texte(&self.email) {
            if !e.contains('@') {
                return Err("L'adresse e-mail n'est pas valide.".into());
            }
        }
        Ok(PersonneBody {
            nom,
            prenom: texte(&self.prenom),
            email: texte(&self.email),
            telephone: texte(&self.telephone),
            ..Default::default()
        })
    }

    /// Commande unique : nouvelle personne + lien dans une seule requête.
    pub fn ajout(&self) -> Result<AjoutResponsableBody, String> {
        let mut lien = self.lien(Uuid::nil());
        lien.personne_id = None;
        Ok(AjoutResponsableBody {
            nouvelle_personne: Some(self.personne()?),
            lien,
        })
    }

    pub fn lien(&self, personne_id: Uuid) -> LienBody {
        LienBody {
            personne_id: Some(personne_id),
            foyer_id: None,
            lien: self.lien.trim().to_string(),
            autorite_parentale: self.autorite,
            garde: texte(&self.garde),
            date_debut: None,
            date_fin: None,
            peut_voir_notes: self.voir_notes,
            peut_voir_vs: self.voir_vs,
            peut_justifier_absence: self.justifier,
            peut_autoriser_sorties: false,
        }
    }
}

/// Saisie d'un contact d'urgence (nouvelle personne).
pub fn contact_depuis(
    nom: &str,
    prenom: &str,
    telephone: &str,
    relation: &str,
    peut_recuperer: bool,
) -> Result<ContactBody, String> {
    let nom = texte(nom).ok_or("Le nom du contact est obligatoire.")?;
    let telephone = texte(telephone).ok_or("Un contact d'urgence doit avoir un téléphone.")?;
    let relation = texte(relation).ok_or("Indiquez la relation avec l'élève.")?;
    Ok(ContactBody {
        personne_id: None,
        nouvelle_personne: Some(PersonneBody {
            nom,
            prenom: texte(prenom),
            telephone: Some(telephone),
            ..Default::default()
        }),
        relation,
        ordre: 1,
        peut_recuperer,
        commentaire: None,
    })
}

/// Motif de suspension : obligatoire (3 à 500 caractères).
pub fn valider_motif(motif: &str) -> Result<String, String> {
    let m = motif.trim();
    let n = m.chars().count();
    if !(3..=500).contains(&n) {
        return Err("Le motif est obligatoire (décision de justice, référence…).".into());
    }
    Ok(m.to_string())
}

/// Parts de paiement saisies (texte par lien) → liste à envoyer. Champs vides ignorés ;
/// la somme doit faire 100 % s'il y a au moins un payeur.
pub fn parts_payeurs(saisies: &[(Uuid, String)]) -> Result<Vec<(Uuid, i16)>, String> {
    let mut v = Vec::new();
    for (id, t) in saisies {
        let t = t.trim().trim_end_matches('%').trim();
        if t.is_empty() || t == "0" {
            continue;
        }
        let p: i16 = t
            .parse()
            .map_err(|_| format!("« {t} » n'est pas un pourcentage entier."))?;
        if !(1..=100).contains(&p) {
            return Err("Chaque part est comprise entre 1 et 100 %.".into());
        }
        v.push((*id, p));
    }
    let somme: i32 = v.iter().map(|(_, p)| i32::from(*p)).sum();
    if !v.is_empty() && somme != 100 {
        return Err(format!(
            "La somme des parts fait {somme} % au lieu de 100 %."
        ));
    }
    Ok(v)
}

/// Pièces d'identité acceptées au guichet : (code de l'API, libellé). Seul le TYPE est enregistré
/// par le serveur : l'écran ne demande ni numéro, ni copie.
pub const PIECES_IDENTITE: [(&str, &str); 6] = [
    ("carte_identite", "Carte nationale d'identité"),
    ("passeport", "Passeport"),
    ("titre_sejour", "Titre de séjour"),
    ("permis_conduire", "Permis de conduire"),
    ("livret_famille", "Livret de famille"),
    ("autre", "Autre pièce officielle"),
];

/// Libellé d'une pièce à partir de son code (le code inconnu est affiché tel quel).
pub fn libelle_piece(code: &str) -> String {
    PIECES_IDENTITE
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| code.to_string())
}

/// Phrase de la case de confirmation du guichet.
pub const CASE_VERIFICATION: &str =
    "J'ai vérifié une pièce d'identité en cours de validité de cette personne";

/// Contrôle local avant l'envoi : une pièce de la liste fermée et la case cochée. Renvoie le code
/// à transmettre. Le serveur revérifie tout.
pub fn piece_a_envoyer(piece: &str, verifiee: bool) -> Result<String, String> {
    if !PIECES_IDENTITE.iter().any(|(c, _)| *c == piece) {
        return Err("Choisissez le type de pièce d'identité présentée.".into());
    }
    if !verifiee {
        return Err(
            "Confirmez que vous avez vérifié une pièce d'identité en cours de validité.".into(),
        );
    }
    Ok(piece.to_string())
}

/// Texte d'une demande en attente : « NOM Prénom — lien (origine) ».
pub fn libelle_demande(l: &LienEnAttenteEleveDto) -> String {
    let nom = match l.prenom.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => format!("{p} {}", l.nom.to_uppercase()),
        None => l.nom.to_uppercase(),
    };
    let origine = match l.source.as_str() {
        "inscription" => "issue d'une inscription",
        _ => "saisie par l'établissement",
    };
    if l.lien.trim().is_empty() {
        format!("{nom} ({origine})")
    } else {
        format!("{nom} · {} ({origine})", l.lien.trim())
    }
}

/// Demandes les plus anciennes d'abord.
pub fn trier_demandes(mut v: Vec<LienEnAttenteEleveDto>) -> Vec<LienEnAttenteEleveDto> {
    v.sort_by(|a, b| {
        a.demande_le
            .cmp(&b.demande_le)
            .then_with(|| a.attente_id.cmp(&b.attente_id))
    });
    v
}

/// Message de retour après une confirmation au guichet.
pub fn message_guichet(lien_cree: bool, nom: &str) -> String {
    if lien_cree {
        format!("Lien confirmé au guichet : {nom} est maintenant responsable de l'élève.")
    } else {
        format!("{nom} était déjà liée à l'élève : la demande est close.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::{ResumeSectionsDto, SectionsDto};

    fn personne(prenom: Option<&str>) -> PersonneDto {
        PersonneDto {
            id: Uuid::from_u128(1),
            nom: "Potter".into(),
            prenom: prenom.map(Into::into),
            civilite: None,
            email: Some("lily@exemple.fr".into()),
            telephone: Some(" ".into()),
            telephone_2: None,
            adresse: None,
            a_compte: true,
            represente_eleve: false,
            fusionnee_dans: None,
        }
    }

    fn resp() -> ResponsableDto {
        ResponsableDto {
            lien_id: Uuid::from_u128(2),
            eleve_id: Uuid::from_u128(3),
            personne: personne(Some("Lily")),
            foyer_id: None,
            foyer_libelle: None,
            lien: "mère".into(),
            autorite_parentale: true,
            garde: None,
            date_debut: None,
            date_fin: None,
            suspendu_le: None,
            suspension_motif: None,
            suspension_reference: None,
            actif: true,
            peut_voir_notes: true,
            peut_voir_vs: false,
            peut_justifier_absence: true,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
            masque: vec![],
        }
    }

    fn jour() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 5).unwrap()
    }

    #[test]
    fn message_masque_explique_le_groupe_ferme() {
        let m = vec![
            MasquageDto {
                groupe: "F2".into(),
                libelle: "Adresses des responsables".into(),
                raison: "droit_manquant".into(),
                message: "Votre rôle n'ouvre pas cette information.".into(),
                action_a_demander: Some("responsables.adresses.lire".into()),
            },
            MasquageDto {
                groupe: "F4".into(),
                libelle: "Données protégées".into(),
                raison: "conflit_interet".into(),
                message: "Réservé à un collègue.".into(),
                action_a_demander: None,
            },
        ];
        let f2 = message_masque(&m, "F2").unwrap();
        assert!(f2.contains("Votre rôle") && f2.contains("responsables.adresses.lire"));
        assert_eq!(
            message_masque(&m, "F4").as_deref(),
            Some("Réservé à un collègue.")
        );
        assert_eq!(message_masque(&m, "F1"), None);
    }

    #[test]
    fn etat_des_liens() {
        let mut r = resp();
        assert_eq!(etat_lien(&r, jour()), EtatLien::Actif);
        r.date_fin = Some(NaiveDate::from_ymd_opt(2026, 11, 4).unwrap());
        r.actif = false;
        assert_eq!(etat_lien(&r, jour()), EtatLien::Termine);
        r.date_fin = None;
        r.date_debut = Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert_eq!(etat_lien(&r, jour()), EtatLien::AVenir);
        r.suspendu_le = Some(chrono::DateTime::from_timestamp(0, 0).unwrap());
        assert_eq!(etat_lien(&r, jour()), EtatLien::Suspendu);
    }

    #[test]
    fn libelles_et_coordonnees() {
        assert_eq!(nom_personne(&personne(Some("Lily"))), "Lily POTTER");
        assert_eq!(nom_personne(&personne(None)), "POTTER");
        assert_eq!(
            coordonnees(&personne(None)),
            vec!["lily@exemple.fr".to_string()]
        );
        assert_eq!(droits_du_lien(&resp()), vec!["notes", "justifier"]);
        assert_eq!(libelle_garde(Some("alternee")), "Garde alternée");
        assert_eq!(libelle_domaine("inconnu"), "Autre");
    }

    #[test]
    fn parts_des_payeurs() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(
            parts_payeurs(&[(a, "".into()), (b, "".into())]).unwrap(),
            vec![]
        );
        assert_eq!(
            parts_payeurs(&[(a, "60".into()), (b, " 40 % ".into())]).unwrap(),
            vec![(a, 60), (b, 40)]
        );
        assert!(parts_payeurs(&[(a, "60".into()), (b, "30".into())])
            .unwrap_err()
            .contains("90 %"));
        assert!(parts_payeurs(&[(a, "abc".into())]).is_err());
        assert!(parts_payeurs(&[(a, "150".into())]).is_err());
    }

    #[test]
    fn saisies_valides_et_invalides() {
        let mut s = SaisieResponsable {
            nom: " Potter ".into(),
            lien: "père".into(),
            ..Default::default()
        };
        let p = s.personne().unwrap();
        assert_eq!(p.nom, "Potter");
        assert_eq!(p.email, None, "parent sans e-mail accepté");
        s.email = "pas-une-adresse".into();
        assert!(s.personne().is_err());
        s.email = String::new();
        s.lien = " ".into();
        assert!(s.personne().is_err());
        assert!(valider_motif("  ").is_err());
        assert_eq!(valider_motif(" JAF 2026 ").unwrap(), "JAF 2026");
        assert!(contact_depuis("Dursley", "", "", "tante", true).is_err());
        let c = contact_depuis("Dursley", "Pétunia", "01 02", "tante", true).unwrap();
        assert!(c.peut_recuperer && c.personne_id.is_none());
    }

    #[test]
    fn chronologie_et_sections() {
        let e = |d: &str| EntreeChronologieDto {
            quand: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            source: "evenement".into(),
            domaine: d.into(),
            titre: "x".into(),
        };
        let l = vec![e("vie_scolaire"), e("dossier"), e("vie_scolaire")];
        assert_eq!(
            domaines_presents(&l),
            vec![("dossier".into(), 1), ("vie_scolaire".into(), 2)]
        );
        assert_eq!(filtrer_chronologie(&l, Some("dossier")).len(), 1);
        assert_eq!(filtrer_chronologie(&l, None).len(), 3);

        let mut f = FicheEleveDto {
            eleve_id: Uuid::nil(),
            nom: "Potter".into(),
            prenom: "Harry".into(),
            classe: None,
            regime: None,
            statut: "actif".into(),
            ine: None,
            date_naissance: None,
            age_ans: None,
            majeur: false,
            date_entree: None,
            date_sortie: None,
            sections: SectionsDto::default(),
            resume: ResumeSectionsDto::default(),
            responsables: vec![],
            contacts: vec![],
            actions: vec![],
            alertes: vec![],
            masques: vec![],
            masquages: vec![],
            interdictions: vec![],
        };
        assert!(sections_accessibles(&f, false).is_empty());
        f.sections.vs = true;
        f.resume.sanctions = 2;
        let s = sections_accessibles(&f, true);
        assert_eq!(s[0].lien, Some("/web/absences"));
        assert!(s[0].detail.contains("2 sanctions"));
        assert_eq!(
            sections_accessibles(&f, false)[0].lien,
            Some("/web/vie-scolaire")
        );
        assert!(!peut(&f.actions, ACTION_SUSPENDRE));
    }

    #[test]
    fn pieces_codes_et_libelles() {
        let codes: Vec<&str> = PIECES_IDENTITE.iter().map(|(c, _)| *c).collect();
        assert_eq!(
            codes,
            [
                "carte_identite",
                "passeport",
                "titre_sejour",
                "permis_conduire",
                "livret_famille",
                "autre"
            ]
        );
        assert_eq!(libelle_piece("passeport"), "Passeport");
        assert_eq!(libelle_piece("inconnu"), "inconnu");
        assert!(PIECES_IDENTITE.iter().all(|(_, l)| !l.trim().is_empty()));
    }

    #[test]
    fn guichet_exige_une_piece_de_la_liste_et_la_case() {
        assert!(piece_a_envoyer("", true).is_err());
        assert!(piece_a_envoyer("passeport", false).is_err());
        assert!(piece_a_envoyer("n'importe quoi", true).is_err());
        assert_eq!(piece_a_envoyer("passeport", true).unwrap(), "passeport");
    }

    #[test]
    fn la_confirmation_ne_transporte_que_le_type_de_piece() {
        // Aucun champ numéro ni copie : le corps ne contient que `piece`.
        let corps = serde_json::to_value(ent_types::dossier::ConfirmationGuichetBody {
            piece: "carte_identite".into(),
            confirmation_direction: false,
        })
        .unwrap();
        let champs: Vec<&String> = corps.as_object().unwrap().keys().collect();
        assert_eq!(champs, ["piece"]);
        assert!(PIECES_IDENTITE
            .iter()
            .all(|(c, l)| !c.contains("numero") && !l.to_lowercase().contains("numéro")));
        assert!(!CASE_VERIFICATION.to_lowercase().contains("numéro"));
    }

    #[test]
    fn demandes_en_attente_libelle_tri_et_message() {
        let d = |n: u128, t: i64, prenom: Option<&str>, source: &str| LienEnAttenteEleveDto {
            attente_id: Uuid::from_u128(n),
            personne_id: Uuid::from_u128(n),
            nom: "Martin".into(),
            prenom: prenom.map(Into::into),
            a_un_compte: false,
            lien: "mère".into(),
            source: source.into(),
            demande_le: chrono::DateTime::from_timestamp(t, 0).unwrap(),
        };
        assert_eq!(
            libelle_demande(&d(1, 0, Some("Léa"), "inscription")),
            "Léa MARTIN · mère (issue d'une inscription)"
        );
        assert_eq!(
            libelle_demande(&d(1, 0, None, "saisie")),
            "MARTIN · mère (saisie par l'établissement)"
        );
        let v = trier_demandes(vec![d(1, 20, None, "saisie"), d(2, 10, None, "saisie")]);
        assert_eq!(v[0].attente_id, Uuid::from_u128(2));
        assert!(message_guichet(true, "Léa").contains("confirmé"));
        assert!(message_guichet(false, "Léa").contains("déjà"));
    }
}
