//! Logique d'écran pure du registre des violations (sans Leptos ni réseau), testée en natif.
//!
//! Aucune règle métier ici : la grille de risque, l'échéance, ce qui manque pour clore et les droits viennent du
//! serveur. L'écran ne fait que dire ces faits en clair et préparer les corps à envoyer.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ent_web_api::rgpd_violations_types::{
    PersonnesBody, QualificationViolationBody, ViolationDto, ViolationLigneDto,
};
use uuid::Uuid;

use crate::noyau::temps::paris_vers_utc;

/// Catégories de données proposées à la qualification : un sous-ensemble des codes du catalogue de conservation
/// (`ent-db::conservation::CATALOGUE`, le serveur refuse un code inconnu), avec un libellé court. Les catégories particulières (santé, handicap, social, discipline…) pèsent
/// dans la grille du serveur.
pub const CATEGORIES: [(&str, &str); 16] = [
    ("scol.registre", "Identité et listes d'élèves"),
    ("scol.dossier", "Dossier scolaire"),
    ("scol.bulletin", "Bulletins et conseils de classe"),
    ("eva.note", "Notes et évaluations"),
    ("vs.appel", "Appel, absences et retards"),
    ("vs.justificatif", "Justificatifs d'absence"),
    ("disc.sanction", "Sanctions et incidents"),
    ("sante.dossier", "Santé (infirmerie, PAI)"),
    ("adm.personne", "Responsables et coordonnées"),
    (
        "adm.decision_justice",
        "Décisions de justice (autorité parentale)",
    ),
    ("fin.piece", "Factures et paiements"),
    ("fin.iban", "Coordonnées bancaires"),
    ("fin.bourse", "Dossiers de bourse"),
    ("msg.conversation", "Messagerie"),
    ("cpt.securite", "Comptes : connexions et sessions"),
    ("rh.dossier", "Dossiers des personnels"),
];

pub const RISQUES: [&str; 3] = ["faible", "avere", "eleve"];
pub const ETATS: [&str; 4] = ["ouverte", "qualifiee", "notifiee", "close"];

pub fn libelle_categorie(code: &str) -> String {
    CATEGORIES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| code.to_string())
}

pub fn libelle_risque(r: &str) -> &'static str {
    match r {
        "faible" => "Risque faible",
        "avere" => "Risque avéré",
        "eleve" => "Risque élevé",
        _ => "Risque non décidé",
    }
}

pub fn libelle_etat(e: &str) -> &'static str {
    match e {
        "ouverte" => "À qualifier",
        "qualifiee" => "Qualifiée",
        "notifiee" => "Notifiée à la CNIL",
        "close" => "Close",
        _ => "État inconnu",
    }
}

pub fn libelle_avancement(a: &str) -> &'static str {
    match a {
        "recue" => "Reçue, en attente de traitement",
        "en_cours" => "En cours de traitement",
        "close" => "Traitée",
        _ => "Avancement inconnu",
    }
}

pub fn libelle_origine(o: &str) -> &'static str {
    match o {
        "declaration" => "Déclarée par un personnel",
        "supervision" => "Détectée automatiquement",
        "plateforme" => "Signalée par l'éditeur de la plateforme",
        _ => "Origine inconnue",
    }
}

pub fn libelle_etape(e: &str) -> &'static str {
    match e {
        "declaree" => "Déclarée",
        "alerte_envoyee" => "Direction alertée",
        "qualifiee" => "Qualifiée",
        "categories" => "Catégories de données",
        "personnes_calculees" => "Personnes concernées calculées",
        "mesure" => "Mesure",
        "risque_propose" => "Risque proposé par la grille",
        "avis_dpo" => "Avis du DPO",
        "risque_decide" => "Risque décidé",
        "notifiee_cnil" => "Notifiée à la CNIL",
        "notification_complementaire" => "Notification complémentaire",
        "non_notification" => "Non-notification justifiée",
        "personnes_informees" => "Personnes informées",
        "personnes_non_informees" => "Exception à l'information des personnes",
        "echeance_alertee" => "Rappel d'échéance",
        "propagee" => "Reçue de la plateforme",
        "close" => "Close",
        _ => "Étape",
    }
}

/// Ce que l'horloge des 72 h dit, en clair. `None` : plus d'horloge (incident de sécurité, notifiée, non-notification
/// justifiée ou close).
pub fn phrase_horloge(heures_restantes: i64, en_retard: bool, reglee: bool) -> Option<String> {
    if reglee {
        return None;
    }
    Some(if en_retard || heures_restantes < 0 {
        let h = heures_restantes.unsigned_abs();
        format!(
            "Échéance CNIL dépassée de {h} h : notifiez dès que possible, en motivant le retard."
        )
    } else if heures_restantes == 0 {
        "Moins d'une heure avant l'échéance CNIL.".to_string()
    } else {
        format!("{heures_restantes} h avant l'échéance CNIL (72 h après la prise de connaissance).")
    })
}

/// L'échéance n'a plus de sens : rien à notifier, déjà notifiée, non-notification justifiée, ou close.
pub fn echeance_reglee(v: &ViolationDto) -> bool {
    v.nature_incident == "incident_securite"
        || v.notifiee_cnil_le.is_some()
        || v.non_notification_motif.is_some()
        || v.etat == "close"
}

pub fn echeance_reglee_ligne(l: &ViolationLigneDto) -> bool {
    l.nature_incident == "incident_securite"
        || l.notifiee_cnil_le.is_some()
        || l.non_notifiee
        || l.etat == "close"
}

/// Teinte de l'horloge : rouge en retard ou à moins de 6 h, orange à moins de 24 h.
pub fn urgence(heures_restantes: i64, en_retard: bool, reglee: bool) -> Urgence {
    if reglee {
        Urgence::Aucune
    } else if en_retard || heures_restantes < 6 {
        Urgence::Critique
    } else if heures_restantes < 24 {
        Urgence::Proche
    } else {
        Urgence::Normale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgence {
    Aucune,
    Normale,
    Proche,
    Critique,
}

/// Registre trié : les plus urgentes d'abord (horloge en cours, la plus proche), puis les autres, récentes d'abord.
pub fn trier_registre(lignes: &mut [ViolationLigneDto]) {
    lignes.sort_by(|a, b| {
        let ra = echeance_reglee_ligne(a);
        let rb = echeance_reglee_ligne(b);
        ra.cmp(&rb)
            .then_with(|| {
                if !ra && !rb {
                    a.heures_restantes.cmp(&b.heures_restantes)
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| b.connue_le.cmp(&a.connue_le))
    });
}

/// Compteurs du bandeau du registre.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compteurs {
    pub ouvertes: usize,
    pub horloge_en_cours: usize,
    pub en_retard: usize,
    pub closes: usize,
}

pub fn compter(lignes: &[ViolationLigneDto]) -> Compteurs {
    let mut c = Compteurs::default();
    for l in lignes {
        if l.etat == "close" {
            c.closes += 1;
        } else {
            c.ouvertes += 1;
        }
        if !echeance_reglee_ligne(l) {
            c.horloge_en_cours += 1;
        }
        if l.en_retard {
            c.en_retard += 1;
        }
    }
    c
}

/// Date et heure locales (heure de Paris, saisie du formulaire) → instant UTC. Vide → `Ok(None)`.
pub fn instant_saisi(date: &str, heure: &str) -> Result<Option<DateTime<Utc>>, &'static str> {
    let date = date.trim();
    if date.is_empty() {
        return Ok(None);
    }
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "Date illisible.")?;
    let h = match heure.trim() {
        "" => NaiveTime::from_hms_opt(0, 0, 0).expect("minuit"),
        t => NaiveTime::parse_from_str(t, "%H:%M").map_err(|_| "Heure illisible (HH:MM).")?,
    };
    Ok(Some(paris_vers_utc(NaiveDateTime::new(d, h))))
}

/// Contrôle local de la déclaration (le serveur revérifie) : une phrase de 3 à 200 caractères.
pub fn declaration_recevable(intitule: &str) -> Result<(), &'static str> {
    let n = intitule.trim().chars().count();
    if n < 3 {
        Err("Dites en une phrase ce qui s'est passé.")
    } else if n > 200 {
        Err("Une phrase suffit (200 caractères au plus) ; les détails vont dans les précisions.")
    } else {
        Ok(())
    }
}

/// Saisie de la qualification, telle que l'écran la tient.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieQualification {
    pub incident_securite: bool,
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    pub categories: Vec<String>,
    pub personnes: String,
    pub personnes_estime: bool,
    pub mineurs: bool,
    pub consequences: String,
}

pub fn corps_qualification(
    s: &SaisieQualification,
) -> Result<QualificationViolationBody, &'static str> {
    if !s.incident_securite && !(s.confidentialite || s.integrite || s.disponibilite) {
        return Err("Cochez ce qui est atteint : confidentialité, intégrité ou disponibilité.");
    }
    let personnes_nombre = match s.personnes.trim() {
        "" => None,
        t => Some(
            t.replace([' ', '\u{202f}', '\u{a0}'], "")
                .parse::<i32>()
                .ok()
                .filter(|n| *n >= 0)
                .ok_or("Nombre de personnes : un nombre entier.")?,
        ),
    };
    Ok(QualificationViolationBody {
        nature_incident: if s.incident_securite {
            "incident_securite"
        } else {
            "violation"
        }
        .to_string(),
        confidentialite: s.confidentialite,
        integrite: s.integrite,
        disponibilite: s.disponibilite,
        categories: s.categories.clone(),
        personnes_nombre,
        personnes_estime: s.personnes_estime,
        personnes_mineurs: s.mineurs,
        consequences: Some(s.consequences.trim().to_string()).filter(|c| !c.is_empty()),
    })
}

/// Saisie reprise de la fiche (pour corriger une qualification).
pub fn saisie_depuis(v: &ViolationDto) -> SaisieQualification {
    SaisieQualification {
        incident_securite: v.nature_incident == "incident_securite",
        confidentialite: v.confidentialite,
        integrite: v.integrite,
        disponibilite: v.disponibilite,
        categories: v.categories.clone(),
        personnes: v
            .personnes_nombre
            .map(|n| n.to_string())
            .unwrap_or_default(),
        personnes_estime: v.personnes_estime,
        mineurs: v.personnes_mineurs,
        consequences: v.consequences.clone().unwrap_or_default(),
    }
}

/// Origine des personnes concernées : une liste d'identifiants collés (un par ligne ou séparés par des virgules),
/// ou une classe.
pub fn corps_personnes(origine: &str, texte: &str) -> Result<PersonnesBody, &'static str> {
    let ids: Result<Vec<Uuid>, _> = texte
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(Uuid::parse_str)
        .collect();
    let ids = ids.map_err(|_| "Un identifiant est illisible.")?;
    match origine {
        "classe" => match ids.as_slice() {
            [c] => Ok(PersonnesBody::Classe { classe_id: *c }),
            _ => Err("Indiquez l'identifiant d'une seule classe."),
        },
        "eleves" if !ids.is_empty() => Ok(PersonnesBody::Eleves { eleve_ids: ids }),
        "comptes" if !ids.is_empty() => Ok(PersonnesBody::Comptes { compte_ids: ids }),
        _ => Err("Indiquez au moins un identifiant."),
    }
}

/// Un retard de notification se motive (art. 33.1) : l'écran le demande avant d'envoyer.
pub fn retard_a_motiver(echeance: DateTime<Utc>, notifiee_le: DateTime<Utc>) -> bool {
    notifiee_le > echeance
}

/// Message aux personnes (art. 34) : 20 à 4 000 caractères, comme le serveur.
pub fn message_information_recevable(m: &str) -> bool {
    (20..=4000).contains(&m.trim().chars().count())
}

/// Lien du registre en PDF avec le filtre de l'écran (même paramètres que `GET /rgpd/violations`). L'état vient d'une
/// liste fermée ; il est quand même vérifié, un état inconnu étant omis.
pub fn lien_registre_pdf(etat: &str, a_notifier: bool) -> String {
    let mut params = Vec::new();
    if ETATS.contains(&etat) {
        params.push(format!("etat={etat}"));
    }
    if a_notifier {
        params.push("a_notifier=true".to_string());
    }
    if params.is_empty() {
        "/rgpd/violations/registre.pdf".to_string()
    } else {
        format!("/rgpd/violations/registre.pdf?{}", params.join("&"))
    }
}

/// Lien de la fiche PDF d'une violation.
pub fn lien_fiche_pdf(id: Uuid) -> String {
    format!("/rgpd/violations/{id}/fiche.pdf")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn liens_pdf_avec_le_filtre() {
        assert_eq!(
            lien_registre_pdf("", false),
            "/rgpd/violations/registre.pdf"
        );
        assert_eq!(
            lien_registre_pdf("close", true),
            "/rgpd/violations/registre.pdf?etat=close&a_notifier=true"
        );
        assert_eq!(
            lien_registre_pdf("x&y=1", false),
            "/rgpd/violations/registre.pdf",
            "état hors liste omis"
        );
        assert_eq!(
            lien_fiche_pdf(Uuid::nil()),
            "/rgpd/violations/00000000-0000-0000-0000-000000000000/fiche.pdf"
        );
    }

    fn ligne(etat: &str, heures: i64, notifiee: bool, connue_jour: u32) -> ViolationLigneDto {
        ViolationLigneDto {
            // Identifiant sans `Uuid::new_v4` (fonctionnalité absente du navigateur, FRONTEND_LEPTOS.md §17).
            id: Uuid::from_u128(u128::from(connue_jour) << 8 | heures.unsigned_abs() as u128),
            reference: format!("V-2027-{connue_jour:03}"),
            nature_incident: "violation".into(),
            origine: "declaration".into(),
            intitule: "x".into(),
            connue_le: Utc.with_ymd_and_hms(2027, 3, connue_jour, 8, 0, 0).unwrap(),
            echeance_cnil_le: Utc
                .with_ymd_and_hms(2027, 3, connue_jour + 3, 8, 0, 0)
                .unwrap(),
            heures_restantes: heures,
            en_retard: heures < 0 && !notifiee,
            risque: None,
            notifiee_cnil_le: notifiee.then(|| Utc.with_ymd_and_hms(2027, 3, 2, 8, 0, 0).unwrap()),
            non_notifiee: false,
            personnes_nombre: None,
            etat: etat.into(),
            close_le: None,
        }
    }

    #[test]
    fn horloge_en_clair() {
        assert_eq!(phrase_horloge(5, false, true), None);
        assert!(phrase_horloge(47, false, false)
            .unwrap()
            .starts_with("47 h avant"));
        assert!(phrase_horloge(0, false, false)
            .unwrap()
            .starts_with("Moins d'une heure"));
        assert!(phrase_horloge(-3, true, false)
            .unwrap()
            .contains("dépassée de 3 h"));
        assert_eq!(urgence(30, false, false), Urgence::Normale);
        assert_eq!(urgence(23, false, false), Urgence::Proche);
        assert_eq!(urgence(5, false, false), Urgence::Critique);
        assert_eq!(urgence(-1, true, true), Urgence::Aucune);
    }

    #[test]
    fn registre_urgentes_d_abord() {
        let mut l = vec![
            ligne("notifiee", 10, true, 5),
            ligne("ouverte", 40, false, 6),
            ligne("ouverte", 3, false, 4),
            ligne("close", 0, true, 7),
        ];
        trier_registre(&mut l);
        let ordre: Vec<i64> = l.iter().map(|x| x.heures_restantes).collect();
        assert_eq!(ordre[..2], [3, 40]);
        // Puis les réglées, récentes d'abord.
        assert_eq!(l[2].etat, "close");
        let c = compter(&l);
        assert_eq!(
            c,
            Compteurs {
                ouvertes: 3,
                horloge_en_cours: 2,
                en_retard: 0,
                closes: 1
            }
        );
    }

    #[test]
    fn instant_saisi_en_heure_de_paris() {
        assert_eq!(instant_saisi("", "10:00"), Ok(None));
        // 1er mars : heure d'hiver, UTC+1.
        assert_eq!(
            instant_saisi("2027-03-01", "09:30").unwrap().unwrap(),
            Utc.with_ymd_and_hms(2027, 3, 1, 8, 30, 0).unwrap()
        );
        // 1er juin : heure d'été, UTC+2.
        assert_eq!(
            instant_saisi("2027-06-01", "09:30").unwrap().unwrap(),
            Utc.with_ymd_and_hms(2027, 6, 1, 7, 30, 0).unwrap()
        );
        assert!(instant_saisi("01/03/2027", "").is_err());
        assert!(instant_saisi("2027-03-01", "9h").is_err());
    }

    #[test]
    fn declaration_et_qualification() {
        assert!(declaration_recevable("  a ").is_err());
        assert!(declaration_recevable("clé USB perdue").is_ok());
        assert!(declaration_recevable(&"x".repeat(201)).is_err());
        let mut s = SaisieQualification::default();
        assert!(corps_qualification(&s).is_err(), "rien d'atteint");
        s.confidentialite = true;
        s.personnes = "1 200".into();
        s.categories = vec!["eva.note".into()];
        let b = corps_qualification(&s).unwrap();
        assert_eq!(b.personnes_nombre, Some(1200));
        assert_eq!(b.nature_incident, "violation");
        assert_eq!(b.consequences, None);
        s.personnes = "beaucoup".into();
        assert!(corps_qualification(&s).is_err());
        // Un incident de sécurité sans atteinte n'exige pas de case.
        let i = SaisieQualification {
            incident_securite: true,
            ..Default::default()
        };
        assert_eq!(
            corps_qualification(&i).unwrap().nature_incident,
            "incident_securite"
        );
    }

    #[test]
    fn personnes_par_origine() {
        let a = "00000000-0000-0000-0000-000000000001";
        let b = "00000000-0000-0000-0000-000000000002";
        assert!(matches!(
            corps_personnes("classe", a),
            Ok(PersonnesBody::Classe { .. })
        ));
        assert!(corps_personnes("classe", &format!("{a},{b}")).is_err());
        match corps_personnes("eleves", &format!("{a}\n{b}; ")) {
            Ok(PersonnesBody::Eleves { eleve_ids }) => assert_eq!(eleve_ids.len(), 2),
            r => panic!("{r:?}"),
        }
        assert!(corps_personnes("comptes", "").is_err());
        assert!(corps_personnes("eleves", "pas-un-id").is_err());
    }

    #[test]
    fn libelles_et_listes_fermees() {
        assert_eq!(libelle_risque("eleve"), "Risque élevé");
        assert_eq!(libelle_etat("ouverte"), "À qualifier");
        assert_eq!(libelle_etape("propagee"), "Reçue de la plateforme");
        assert_eq!(libelle_categorie("inconnue"), "inconnue");
        assert!(!message_information_recevable("trop court"));
        let e = Utc.with_ymd_and_hms(2027, 3, 4, 8, 0, 0).unwrap();
        assert!(retard_a_motiver(e, e + chrono::Duration::minutes(1)));
        assert!(!retard_a_motiver(e, e));
    }
}
