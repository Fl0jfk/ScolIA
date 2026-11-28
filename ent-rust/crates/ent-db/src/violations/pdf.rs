//! **Fiches PDF du registre des violations** (socle 27, complément du palier G5 ; NX27-18, RGP-258).
//!
//! Deux documents, rendus **côté serveur** par la brique `ent-pdf` (D-062), à la demande, jamais stockés :
//! * la **fiche d'une violation** : tout ce que la fiche à l'écran montre (horloge, qualification, risque proposé puis
//!   décidé, notifications, information des personnes, mesures, élément de la plateforme, clôture, chronologie), pour
//!   le classeur du DPO, une réunion de direction ou un contrôle de la CNIL ;
//! * le **registre** filtré (art. 33.5 : toutes les violations, notifiées ou non), sous forme de tableau.
//!
//! Choix prudents (D-069), notés dans `docs/socles/27_G5_COMPLEMENT_LIVRAISON.md` :
//! * le gabarit d'établissement du socle 7 (logo, charte) n'est pas encore branché : en-tête textuel (nom de
//!   l'établissement), pied « document interne » ; le rendu passera au gabarit sans changer de contenu ;
//! * **minimisation** (E14, R-RG32) : ni le compte du déclarant, ni une personne nommée ; la chronologie dit le rôle,
//!   pas la personne. Le registre, comme l'export CSV, ne porte pas la description libre ;
//! * les instants sont écrits dans le **fuseau de l'établissement** (formatés par la base), la mise en page est pure et
//!   déterministe (même fiche, même date d'édition : mêmes octets), ce que les tests vérifient.

use super::{ViolationLigne, ViolationVue};
use chrono::{DateTime, Utc};
use ent_pdf::{Align, Colonne, Document, Ligne, Meta, Tableau};

/// Ce que la mise en page reçoit en plus des faits : l'établissement, l'instant d'édition (déjà écrit dans son
/// fuseau) et le formateur d'instants (fuseau de l'établissement, « 12/03/2027 à 14 h 05 »).
pub struct Edition<'a> {
    pub etablissement: &'a str,
    pub edite_le: &'a str,
    pub instant: &'a dyn Fn(DateTime<Utc>) -> String,
}

const PIED: &str =
    "Document interne : registre des violations de données (RGPD, art. 33.5). Ne pas diffuser.";

fn libelle_nature(n: &str) -> &'static str {
    match n {
        "violation" => "Violation de données personnelles",
        "incident_securite" => "Incident de sécurité sans atteinte aux données personnelles",
        _ => "Nature inconnue",
    }
}

fn libelle_origine(o: &str) -> &'static str {
    match o {
        "declaration" => "Déclarée par un personnel",
        "supervision" => "Détectée automatiquement",
        "plateforme" => "Signalée par l'éditeur de la plateforme",
        _ => "Origine inconnue",
    }
}

fn libelle_risque(r: Option<&str>) -> &'static str {
    match r {
        Some("faible") => "faible",
        Some("avere") => "avéré",
        Some("eleve") => "élevé",
        Some(_) => "inconnu",
        None => "non décidé",
    }
}

fn libelle_etat(e: &str) -> &'static str {
    match e {
        "ouverte" => "à qualifier",
        "qualifiee" => "qualifiée",
        "notifiee" => "notifiée à la CNIL",
        "close" => "close",
        _ => "inconnu",
    }
}

fn libelle_etape(e: &str) -> &'static str {
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

fn libelle_mesure(n: &str) -> &'static str {
    match n {
        "prise" => "prise",
        "envisagee" => "envisagée",
        _ => "mesure",
    }
}

fn libelle_categorie(code: &str) -> String {
    match crate::conservation::categorie(code) {
        Some(c) => format!("{} ({code})", c.libelle),
        None => code.to_string(),
    }
}

fn oui_non(b: bool) -> &'static str {
    if b {
        "oui"
    } else {
        "non"
    }
}

/// L'horloge des 72 h en clair, à l'instant de l'édition.
fn horloge(v: &ViolationVue) -> String {
    if v.nature_incident == "incident_securite" {
        return "sans objet (incident de sécurité sans atteinte aux données)".into();
    }
    if v.notifiee_cnil_le.is_some() {
        return "réglée : notifiée à la CNIL".into();
    }
    if v.non_notification_motif.is_some() {
        return "réglée : non-notification justifiée".into();
    }
    if v.etat == "close" {
        return "réglée : violation close".into();
    }
    if v.en_retard {
        return format!(
            "dépassée de {} h, notification encore possible (motif du retard exigé)",
            -v.heures_restantes
        );
    }
    format!("{} h restantes à l'édition", v.heures_restantes.max(0))
}

fn document(titre: &str, ed: &Edition<'_>) -> Document {
    Document::new(Meta {
        titre: titre.to_string(),
        auteur: "ScolIA".into(),
        creation: None,
    })
    .avec_en_tete(&format!(
        "{} — registre des violations de données",
        ed.etablissement
    ))
    .avec_pied(PIED)
}

fn ligne(d: &mut Document, gauche: &str, droite: &str) {
    d.ligne_droite(gauche, droite, false);
}

fn bloc_texte(d: &mut Document, libelle: &str, texte: Option<&str>) {
    if let Some(t) = texte.map(str::trim).filter(|t| !t.is_empty()) {
        d.texte(libelle, ent_pdf::Police::HelveticaGras, 10.5, Align::Gauche);
        d.paragraphe(t);
        d.espace(4.0);
    }
}

/// Rend la fiche d'une violation.
pub fn rendre_fiche(v: &ViolationVue, ed: &Edition<'_>) -> Vec<u8> {
    let quand = ed.instant;
    let mut d = document(&format!("Fiche de la violation {}", v.reference), ed);
    d.titre(&format!("Violation {} — {}", v.reference, v.intitule));
    d.paragraphe(&format!("Fiche éditée le {}.", ed.edite_le));
    d.espace(6.0);

    d.sous_titre("En bref");
    ligne(&mut d, "Nature", libelle_nature(&v.nature_incident));
    ligne(&mut d, "Origine", libelle_origine(&v.origine));
    ligne(&mut d, "État", libelle_etat(&v.etat));
    ligne(&mut d, "Prise de connaissance", &quand(v.connue_le));
    if let Some(c) = v.constatee_le {
        ligne(&mut d, "Constatée depuis", &quand(c));
    }
    if v.nature_incident == "violation" {
        ligne(
            &mut d,
            "Échéance de notification (72 h)",
            &quand(v.echeance_cnil_le),
        );
    }
    ligne(&mut d, "Horloge", &horloge(v));
    if let Some(s) = v.source_code.as_deref() {
        ligne(&mut d, "Détecteur", s);
    }
    bloc_texte(&mut d, "Ce qui s'est passé", v.description.as_deref());

    if let Some(p) = &v.plateforme_element {
        d.sous_titre("Signalement de l'éditeur de la plateforme");
        ligne(&mut d, "Référence de l'éditeur", &p.reference);
        ligne(
            &mut d,
            "Connue de l'éditeur le",
            &quand(p.connue_editeur_le),
        );
        ligne(&mut d, "Établissement informé le", &quand(p.informe_le));
        if let Some(n) = p.personnes_nombre {
            ligne(&mut d, "Personnes de l'établissement", &n.to_string());
        }
        ligne(
            &mut d,
            "Couche compte touchée (ScolIA notifie pour sa part)",
            oui_non(p.couche_compte),
        );
        if let Some(n) = p.notifiee_cnil_par_editeur_le {
            ligne(&mut d, "Notifiée à la CNIL par l'éditeur le", &quand(n));
        }
        bloc_texte(&mut d, "Ce que l'éditeur sait", Some(&p.description));
        bloc_texte(
            &mut d,
            "Éléments propres à l'établissement",
            Some(&p.elements),
        );
    }

    if v.nature_incident == "violation" {
        d.sous_titre("Qualification");
        match v.qualifiee_le {
            Some(q) => ligne(&mut d, "Qualifiée le", &quand(q)),
            None => d.paragraphe("Pas encore qualifiée."),
        }
        let atteintes: Vec<&str> = [
            (v.confidentialite, "confidentialité"),
            (v.integrite, "intégrité"),
            (v.disponibilite, "disponibilité"),
        ]
        .iter()
        .filter(|(b, _)| *b)
        .map(|(_, l)| *l)
        .collect();
        if v.qualifiee_le.is_some() {
            ligne(
                &mut d,
                "Atteinte",
                &if atteintes.is_empty() {
                    "non précisée".to_string()
                } else {
                    atteintes.join(", ")
                },
            );
            let personnes = match v.personnes_nombre {
                Some(n) if v.personnes_estime => format!("{n} (estimation)"),
                Some(n) => format!("{n} (calculé)"),
                None => "non renseigné".into(),
            };
            ligne(&mut d, "Personnes concernées", &personnes);
            ligne(&mut d, "Mineurs concernés", oui_non(v.personnes_mineurs));
            if v.categories.is_empty() {
                ligne(&mut d, "Catégories de données", "aucune renseignée");
            } else {
                d.texte(
                    "Catégories de données",
                    ent_pdf::Police::HelveticaGras,
                    10.5,
                    Align::Gauche,
                );
                for c in &v.categories {
                    d.paragraphe(&format!("• {}", libelle_categorie(c)));
                }
            }
            bloc_texte(&mut d, "Conséquences", v.consequences.as_deref());
        }

        d.sous_titre("Risque pour les personnes");
        ligne(
            &mut d,
            "Proposé par la grille",
            libelle_risque(v.risque_propose.as_deref()),
        );
        for r in &v.raisons_proposition {
            d.paragraphe(&format!("• {r}"));
        }
        match (&v.avis_dpo, v.avis_dpo_le) {
            (Some(a), Some(le)) => {
                bloc_texte(&mut d, &format!("Avis du DPO ({})", quand(le)), Some(a))
            }
            _ => d.paragraphe("Avis du DPO : pas encore donné."),
        }
        ligne(
            &mut d,
            "Décidé par la direction",
            libelle_risque(v.risque.as_deref()),
        );
        if let Some(le) = v.risque_decide_le {
            ligne(&mut d, "Décision le", &quand(le));
        }
        bloc_texte(&mut d, "Motif de la décision", v.risque_motif.as_deref());

        d.sous_titre("Notification à la CNIL (art. 33)");
        if v.notifications.is_empty() {
            match v.non_notification_motif.as_deref() {
                Some(m) => bloc_texte(&mut d, "Non-notification justifiée (art. 33.5)", Some(m)),
                None => d.paragraphe("Pas encore notifiée."),
            }
        } else {
            let mut t = Tableau::new(vec![
                Colonne::new("Rang", 0.6, Align::Gauche),
                Colonne::new("Nature", 1.3, Align::Gauche),
                Colonne::new("Notifiée le", 1.6, Align::Gauche),
                Colonne::new("Référence CNIL", 1.4, Align::Gauche),
                Colonne::new("Résumé", 2.6, Align::Gauche),
            ]);
            for n in &v.notifications {
                t.lignes.push(Ligne::new(vec![
                    n.rang.to_string(),
                    if n.nature == "initiale" {
                        "initiale".into()
                    } else {
                        n.nature.replace('_', " ")
                    },
                    quand(n.notifiee_le),
                    n.reference_cnil.clone(),
                    n.resume.clone().unwrap_or_default(),
                ]));
            }
            d.tableau(&t);
            bloc_texte(&mut d, "Motif du retard", v.retard_motif.as_deref());
        }

        d.sous_titre("Information des personnes (art. 34)");
        match (
            v.personnes_informees_le,
            v.personnes_non_information_motif.as_deref(),
        ) {
            (Some(le), _) => ligne(
                &mut d,
                &format!("Informées le {}", quand(le)),
                &format!(
                    "{} destinataires",
                    v.personnes_informees_nombre.unwrap_or(0)
                ),
            ),
            (None, Some(m)) => bloc_texte(&mut d, "Exception motivée (art. 34.3)", Some(m)),
            (None, None) if v.risque.as_deref() == Some("eleve") => {
                d.paragraphe("Pas encore informées : exigé pour un risque élevé.")
            }
            (None, None) => d.paragraphe("Non informées (exigé seulement pour un risque élevé)."),
        }
    }

    d.sous_titre("Mesures prises ou envisagées");
    if v.mesures.is_empty() {
        d.paragraphe("Aucune mesure consignée.");
    } else {
        let mut t = Tableau::new(vec![
            Colonne::new("Le", 1.5, Align::Gauche),
            Colonne::new("Mesure", 0.9, Align::Gauche),
            Colonne::new("Texte", 4.5, Align::Gauche),
        ]);
        for m in &v.mesures {
            t.lignes.push(Ligne::new(vec![
                quand(m.le),
                libelle_mesure(&m.nature).to_string(),
                m.texte.clone(),
            ]));
        }
        d.tableau(&t);
    }

    d.sous_titre("Clôture");
    match v.close_le {
        Some(le) => {
            ligne(&mut d, "Close le", &quand(le));
            if let Some(r) = v.revue_le {
                ligne(
                    &mut d,
                    "Revue du retour d'expérience",
                    &r.format("%d/%m/%Y").to_string(),
                );
            }
            bloc_texte(
                &mut d,
                "Retour d'expérience",
                v.retour_experience.as_deref(),
            );
        }
        None if v.manque_pour_clore.is_empty() => {
            d.paragraphe("Ouverte : la clôture est possible.")
        }
        None => {
            d.paragraphe("Ouverte. Reste à faire pour clore :");
            for m in &v.manque_pour_clore {
                d.paragraphe(&format!("• {m}"));
            }
        }
    }

    d.sous_titre("Chronologie");
    let mut t = Tableau::new(vec![
        Colonne::new("Le", 1.5, Align::Gauche),
        Colonne::new("Étape", 1.7, Align::Gauche),
        Colonne::new("Rôle", 0.9, Align::Gauche),
        Colonne::new("Détail", 3.0, Align::Gauche),
    ]);
    t.taille = 8.5;
    for e in &v.chronologie {
        t.lignes.push(Ligne::new(vec![
            quand(e.le),
            libelle_etape(&e.etape).to_string(),
            e.par_role.clone().unwrap_or_default(),
            e.texte.clone().unwrap_or_default(),
        ]));
    }
    if t.lignes.is_empty() {
        d.paragraphe("Aucune étape.");
    } else {
        d.tableau(&t);
    }
    d.octets()
}

/// Plafond de la lecture du registre (`LIMIT` de `violations_registre_a`) : au-delà, le PDF le dit.
pub const REGISTRE_LIMITE: usize = 1000;

/// Rend le registre (lignes déjà filtrées et triées par la lecture). `filtre` dit en clair ce qui est retenu.
pub fn rendre_registre(lignes: &[ViolationLigne], filtre: &str, ed: &Edition<'_>) -> Vec<u8> {
    let quand = ed.instant;
    let mut d = document("Registre des violations de données", ed);
    d.titre("Registre des violations de données");
    d.paragraphe(&format!(
        "{} — édité le {}. {filtre}. Toutes les violations y figurent, notifiées ou non (art. 33.5) ; la fiche de \
         chacune porte le détail.",
        ed.etablissement, ed.edite_le
    ));
    if lignes.len() >= REGISTRE_LIMITE {
        d.paragraphe(&format!(
            "Attention : seules les {REGISTRE_LIMITE} plus récentes sont listées. Affinez le filtre (année) pour \
             éditer le reste du registre."
        ));
    }
    let violations = lignes
        .iter()
        .filter(|l| l.nature_incident == "violation")
        .count();
    let notifiees = lignes
        .iter()
        .filter(|l| l.notifiee_cnil_le.is_some())
        .count();
    let justifiees = lignes.iter().filter(|l| l.non_notifiee).count();
    let ouvertes = lignes.iter().filter(|l| l.etat != "close").count();
    let retard = lignes.iter().filter(|l| l.en_retard).count();
    d.espace(4.0);
    ligne(&mut d, "Lignes", &lignes.len().to_string());
    ligne(
        &mut d,
        "Dont violations de données",
        &violations.to_string(),
    );
    ligne(&mut d, "Notifiées à la CNIL", &notifiees.to_string());
    ligne(
        &mut d,
        "Non-notifications justifiées",
        &justifiees.to_string(),
    );
    ligne(&mut d, "Encore ouvertes", &ouvertes.to_string());
    ligne(
        &mut d,
        "Échéance dépassée sans notification",
        &retard.to_string(),
    );
    d.espace(6.0);
    if lignes.is_empty() {
        d.paragraphe("Aucune violation ne répond à ce filtre.");
        return d.octets();
    }
    let mut t = Tableau::new(vec![
        Colonne::new("Référence", 1.1, Align::Gauche),
        Colonne::new("Intitulé", 2.6, Align::Gauche),
        Colonne::new("Connue le", 1.4, Align::Gauche),
        Colonne::new("Risque", 0.8, Align::Gauche),
        Colonne::new("CNIL", 1.4, Align::Gauche),
        Colonne::new("Pers.", 0.6, Align::Droite),
        Colonne::new("État", 1.1, Align::Gauche),
    ]);
    t.taille = 8.0;
    for l in lignes {
        let cnil = if l.nature_incident == "incident_securite" {
            "sans objet".to_string()
        } else if let Some(n) = l.notifiee_cnil_le {
            quand(n)
        } else if l.non_notifiee {
            "non notifiée (justifiée)".into()
        } else if l.en_retard {
            "à notifier, échéance dépassée".into()
        } else {
            "à notifier".into()
        };
        let intitule = if l.nature_incident == "incident_securite" {
            format!("{} (incident de sécurité)", l.intitule)
        } else {
            l.intitule.clone()
        };
        t.lignes.push(Ligne::new(vec![
            l.reference.clone(),
            intitule,
            quand(l.connue_le),
            if l.nature_incident == "violation" {
                libelle_risque(l.risque.as_deref()).to_string()
            } else {
                "—".into()
            },
            cnil,
            l.personnes_nombre
                .map(|n| n.to_string())
                .unwrap_or_default(),
            libelle_etat(&l.etat).to_string(),
        ]));
    }
    d.tableau(&t);
    d.octets()
}

#[cfg(test)]
mod tests {
    use super::super::{EtapeVue, MesureVue, NotificationVue};
    use super::*;
    use ent_pdf::releve::{texte, texte_des_pages, verifier_structure};

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn utc(i: DateTime<Utc>) -> String {
        i.format("%d/%m/%Y à %H h %M").to_string()
    }

    fn edition(f: &dyn Fn(DateTime<Utc>) -> String) -> Edition<'_> {
        Edition {
            etablissement: "Collège Poudlard",
            edite_le: "02/03/2027 à 10 h 00",
            instant: f,
        }
    }

    fn fiche() -> ViolationVue {
        ViolationVue {
            id: uuid::Uuid::nil(),
            reference: "V-2027-003".into(),
            nature_incident: "violation".into(),
            origine: "declaration".into(),
            plateforme: false,
            source_code: None,
            declaree_par_compte_id: Some(uuid::Uuid::from_u128(42)),
            intitule: "Clé USB avec les notes de 4e B perdue".into(),
            description: Some("Perdue dans le bus scolaire, non chiffrée.".into()),
            constatee_le: None,
            connue_le: t("2027-03-01T08:00:00Z"),
            echeance_cnil_le: t("2027-03-04T08:00:00Z"),
            heures_restantes: 46,
            en_retard: false,
            confidentialite: true,
            integrite: false,
            disponibilite: false,
            categories: vec!["eva.note".into(), "scol.registre".into()],
            personnes_nombre: Some(28),
            personnes_estime: false,
            personnes_mineurs: true,
            consequences: Some("Notes connues d'un tiers.".into()),
            qualifiee_le: Some(t("2027-03-01T09:00:00Z")),
            risque_propose: Some("avere".into()),
            raisons_proposition: vec!["des mineurs sont concernés par une divulgation"],
            avis_dpo: Some("Notifier, risque avéré.".into()),
            avis_dpo_le: Some(t("2027-03-01T10:00:00Z")),
            risque: Some("avere".into()),
            risque_motif: Some("Conforme à l'avis.".into()),
            risque_decide_le: Some(t("2027-03-01T11:00:00Z")),
            notifiee_cnil_le: Some(t("2027-03-02T08:00:00Z")),
            reference_cnil: Some("CNIL-123".into()),
            retard_motif: None,
            non_notification_motif: None,
            personnes_informees_le: None,
            personnes_informees_nombre: None,
            personnes_non_information_motif: None,
            etat: "notifiee".into(),
            close_le: None,
            retour_experience: None,
            revue_le: None,
            mesures: vec![MesureVue {
                nature: "prise".into(),
                texte: "Mots de passe des comptes enseignants renouvelés".into(),
                le: t("2027-03-01T12:00:00Z"),
            }],
            notifications: vec![NotificationVue {
                rang: 1,
                nature: "initiale".into(),
                notifiee_le: t("2027-03-02T08:00:00Z"),
                reference_cnil: "CNIL-123".into(),
                resume: None,
            }],
            chronologie: vec![EtapeVue {
                etape: "declaree".into(),
                texte: None,
                par_role: Some("prof".into()),
                le: t("2027-03-01T08:00:00Z"),
            }],
            manque_pour_clore: vec![],
            plateforme_element: None,
        }
    }

    #[test]
    fn fiche_complete_lisible_et_deterministe() {
        let f = utc;
        let v = fiche();
        let a = rendre_fiche(&v, &edition(&f));
        let b = rendre_fiche(&v, &edition(&f));
        assert_eq!(a, b, "rendu déterministe");
        verifier_structure(&a);
        let txt = texte(&a);
        for attendu in [
            "Violation V-2027-003",
            "Collège Poudlard",
            "Prise de connaissance",
            "01/03/2027 à 08 h 00",
            "04/03/2027 à 08 h 00",
            "réglée : notifiée à la CNIL",
            "confidentialité",
            "28 (calculé)",
            "notes hors bulletin, devoirs, évaluations (eva.note)",
            "avéré",
            "Avis du DPO",
            "CNIL-123",
            "Mots de passe des comptes enseignants renouvelés",
            "Déclarée",
            "Ne pas diffuser",
        ] {
            assert!(txt.contains(attendu), "« {attendu} » absent : {txt}");
        }
        // Minimisation : le compte du déclarant n'est jamais écrit.
        assert!(!txt.contains(&uuid::Uuid::from_u128(42).to_string()));
    }

    #[test]
    fn fiche_dit_ce_qui_manque_et_le_retard() {
        let f = utc;
        let mut v = fiche();
        v.notifications.clear();
        v.notifiee_cnil_le = None;
        v.reference_cnil = None;
        v.etat = "qualifiee".into();
        v.en_retard = true;
        v.heures_restantes = -5;
        v.risque = Some("eleve".into());
        v.manque_pour_clore =
            vec!["notifier la CNIL, ou justifier la non-notification (risque faible seulement)"];
        let txt = texte(&rendre_fiche(&v, &edition(&f)));
        assert!(txt.contains("dépassée de 5 h"), "{txt}");
        assert!(txt.contains("Pas encore notifiée"));
        assert!(txt.contains("exigé pour un risque élevé"));
        assert!(txt.contains("Reste à faire pour clore"));
    }

    #[test]
    fn incident_de_securite_sans_horloge_ni_cnil() {
        let f = utc;
        let mut v = fiche();
        v.nature_incident = "incident_securite".into();
        let txt = texte(&rendre_fiche(&v, &edition(&f)));
        assert!(txt.contains("sans objet"));
        assert!(!txt.contains("Notification à la CNIL"), "{txt}");
        assert!(!txt.contains("Échéance de notification"));
    }

    fn ligne_registre(i: i32, etat: &str) -> ViolationLigne {
        ViolationLigne {
            id: uuid::Uuid::from_u128(i as u128),
            reference: format!("V-2027-{i:03}"),
            nature_incident: if i % 7 == 0 {
                "incident_securite".into()
            } else {
                "violation".into()
            },
            origine: "declaration".into(),
            intitule: format!("Incident numéro {i}"),
            connue_le: t("2027-03-01T08:00:00Z"),
            echeance_cnil_le: t("2027-03-04T08:00:00Z"),
            heures_restantes: 10,
            en_retard: i % 5 == 0,
            risque: Some("faible".into()),
            notifiee_cnil_le: (i % 2 == 0).then(|| t("2027-03-02T08:00:00Z")),
            non_notifiee: i % 3 == 0,
            personnes_nombre: Some(i),
            etat: etat.into(),
            close_le: None,
        }
    }

    #[test]
    fn registre_en_tableau_sur_plusieurs_pages() {
        let f = utc;
        let lignes: Vec<ViolationLigne> = (1..=120).map(|i| ligne_registre(i, "close")).collect();
        let pdf = rendre_registre(&lignes, "Année 2027", &edition(&f));
        verifier_structure(&pdf);
        let pages = texte_des_pages(&pdf);
        assert!(pages.len() >= 3, "120 lignes : {} pages", pages.len());
        let n = pages.len();
        for (i, p) in pages.iter().enumerate() {
            assert!(p.contains(&format!("Page {} sur {n}", i + 1)));
            assert!(
                p.contains("Référence"),
                "en-tête de tableau répété page {}",
                i + 1
            );
        }
        let txt = texte(&pdf);
        assert!(txt.contains("V-2027-001") && txt.contains("V-2027-120"));
        assert!(txt.contains("Année 2027"));
        // La cellule peut passer à la ligne : on lit le texte sans ses retours.
        let plat = txt.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            plat.contains("(incident de sécurité)") || plat.contains("(incident de"),
            "{plat:.400}"
        );
        assert!(
            !txt.contains("seules les"),
            "120 lignes : pas de plafond annoncé"
        );
    }

    #[test]
    fn registre_plafonne_le_dit() {
        let f = utc;
        let lignes: Vec<ViolationLigne> = (1..=REGISTRE_LIMITE as i32)
            .map(|i| ligne_registre(i, "close"))
            .collect();
        let txt = texte(&rendre_registre(&lignes, "Toutes les années", &edition(&f)));
        assert!(txt.contains("seules les 1000 plus récentes sont listées"));
    }

    #[test]
    fn registre_vide() {
        let f = utc;
        let txt = texte(&rendre_registre(&[], "Toutes les années", &edition(&f)));
        assert!(txt.contains("Aucune violation ne répond à ce filtre"));
    }
}
