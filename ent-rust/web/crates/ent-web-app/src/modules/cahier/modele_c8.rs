//! Logique d'écran pure du lot C8 (tenue, visa, inspection, exports, clôture et archive) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : droits, référence figée du visa, bornes de l'accès d'inspection (durée, cours, période),
//! journalisation, empreinte des exports, irréversibilité de la clôture restent au serveur. Ce module formate ce que le serveur
//! annonce et prépare les corps de requête. Aucun nom d'élève n'existe dans ces écrans.

use chrono::{Duration, NaiveDate};
use ent_web_api::cahier_c8_types::*;
use uuid::Uuid;

use crate::noyau::temps::date_longue;

/// Étiquette d'état affichée avec un ton (`ok`, `info`, `avertissement`, `neutre`, `mal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ton {
    Ok,
    Info,
    Avertissement,
    Neutre,
    Mal,
}

/// Fenêtre par défaut du tableau de tenue : les six dernières semaines, jusqu'à aujourd'hui.
pub fn periode_par_defaut(auj: NaiveDate) -> (NaiveDate, NaiveDate) {
    (auj - Duration::days(42), auj)
}

/// Lecture d'un jour saisi (`AAAA-MM-JJ`).
pub fn lire_jour(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d").ok()
}

/// « 12 séances sur 16 (75 %) » ou « Aucune séance sur la période ».
pub fn phrase_taux(l: &TenueLigneDto) -> String {
    match l.taux_pct {
        Some(t) => format!(
            "{} séance(s) sur {} renseignée(s) ({t} %)",
            l.seances_renseignees, l.seances_passees
        ),
        None => "Aucune séance sur la période".to_string(),
    }
}

/// État d'une ligne du tableau : la priorité va à ce qui a changé depuis le visa, puis au retard de visa.
pub fn etat_ligne(l: &TenueLigneDto) -> (&'static str, Ton) {
    if l.modifies_apres_visa > 0 {
        ("Modifié après visa", Ton::Avertissement)
    } else if l.ajoutes_apres_visa > 0 {
        // M7 : ce qui est ajouté après un visa est aussi visible que ce qui est modifié.
        ("Complété après visa", Ton::Avertissement)
    } else if l.a_viser {
        ("À viser", Ton::Info)
    } else {
        ("Visé", Ton::Ok)
    }
}

/// Dernière entrée et dernier visa en une phrase (« dernière entrée le … · jamais visé »).
pub fn phrase_dates(l: &TenueLigneDto) -> String {
    let entree = l
        .derniere_entree
        .map_or("aucune entrée publiée".to_string(), |d| {
            format!("dernière entrée le {}", date_longue(d))
        });
    let visa = l.dernier_visa_le.map_or("jamais visé".to_string(), |d| {
        format!("visé le {}", date_longue(d.date_naive()))
    });
    format!("{entree} · {visa}")
}

/// Corps d'un visa de cours. Une observation vide n'est pas envoyée.
pub fn corps_visa(
    cours: Uuid,
    du: NaiveDate,
    au: NaiveDate,
    observation: &str,
    confirmer_vide: bool,
) -> VisaCahierBody {
    let o = observation.trim();
    VisaCahierBody {
        cible: VISA_COURS.into(),
        cours_id: Some(cours),
        classe_id: None,
        agent_id: None,
        du,
        au,
        observation: (!o.is_empty()).then(|| o.to_string()),
        confirmer_vide,
    }
}

/// Confirmation lue après un visa : ce qui a été figé, ou le constat d'un cahier vide.
pub fn phrase_visa(v: &VisaCahierDto) -> String {
    if v.vide {
        format!(
            "Visa enregistré : cahier vide sur la période du {} au {}.",
            date_longue(v.du),
            date_longue(v.au)
        )
    } else {
        format!(
            "Visa enregistré : {} élément(s) figés du {} au {}.",
            v.nb_elements,
            date_longue(v.du),
            date_longue(v.au)
        )
    }
}

/// Ligne d'une modification postérieure au visa.
pub fn phrase_modification(m: &ModificationDto) -> String {
    let motif = m
        .motif
        .as_deref()
        .filter(|x| !x.is_empty())
        .map_or(String::new(), |x| format!(" — {x}"));
    format!(
        "Version {} par {} le {}{motif}",
        m.version,
        m.par,
        date_longue(m.le.date_naive())
    )
}

/// Libellé et ton de l'état d'un accès d'inspection.
pub fn etat_acces(etat: &str) -> (&'static str, Ton) {
    match etat {
        ACCES_ACTIF => ("Actif", Ton::Ok),
        ACCES_A_VENIR => ("À venir", Ton::Info),
        ACCES_EXPIRE => ("Expiré", Ton::Neutre),
        ACCES_REVOQUE => ("Révoqué", Ton::Mal),
        _ => ("Inconnu", Ton::Neutre),
    }
}

/// Libellé du motif d'un accès d'inspection.
pub fn libelle_motif(motif: &str) -> &'static str {
    match motif {
        "rdv_carriere" => "Rendez-vous de carrière",
        "inspection" => "Inspection",
        _ => "Autre",
    }
}

/// Accès : « jusqu'au 12 décembre · 2 lectures ».
pub fn phrase_validite(a: &AccesInspectionDto) -> String {
    format!(
        "Cahier du {} au {} · accès jusqu'au {} · {} lecture(s)",
        date_longue(a.du_cahier),
        date_longue(a.au_cahier),
        date_longue(a.valable_au.date_naive()),
        a.nb_consultations
    )
}

/// Corps d'octroi d'un accès d'inspection ; erreur lisible si un champ est illisible. La borne de durée (90 jours), le contrôle
/// du bénéficiaire et des cours sont ceux du serveur.
pub fn corps_acces(
    beneficiaire: &str,
    cours: Vec<Uuid>,
    du: &str,
    au: &str,
    duree: &str,
    motif: &str,
) -> Result<AccesInspectionBody, String> {
    let beneficiaire_user_id = Uuid::parse_str(beneficiaire.trim())
        .map_err(|_| "L'identifiant du compte de l'inspecteur est illisible.".to_string())?;
    if cours.is_empty() {
        return Err("Cochez au moins un cours à ouvrir.".into());
    }
    let du_cahier = lire_jour(du).ok_or("Début de période illisible.")?;
    let au_cahier = lire_jour(au).ok_or("Fin de période illisible.")?;
    let duree_jours = match duree.trim() {
        "" => None,
        t => Some(
            t.parse::<i32>()
                .map_err(|_| "Durée illisible.".to_string())?,
        ),
    };
    Ok(AccesInspectionBody {
        beneficiaire_user_id,
        cours_ids: cours,
        agent_id: None,
        du_cahier,
        au_cahier,
        duree_jours,
        motif: motif.into(),
    })
}

/// Points du contrôle de clôture, un par ligne ; vide si rien à signaler.
pub fn points_controle(c: &ControleClotureDto) -> Vec<String> {
    let mut v = Vec::new();
    if !c.cours_non_vises.is_empty() {
        v.push(format!(
            "{} cours avec du contenu publié n'ont aucun visa cette année.",
            c.cours_non_vises.len()
        ));
    }
    if c.entrees_brouillon > 0 {
        v.push(format!(
            "{} entrée(s) restent en brouillon ou programmées et ne seront jamais publiées.",
            c.entrees_brouillon
        ));
    }
    if c.travaux_brouillon > 0 {
        v.push(format!(
            "{} travail(aux) restent en brouillon ou programmés.",
            c.travaux_brouillon
        ));
    }
    if c.non_rendus > 0 {
        v.push(format!(
            "{} constat(s) « non rendu » : les élèves gardent leurs dépôts après la clôture.",
            c.non_rendus
        ));
    }
    v
}

/// Texte d'un export figé affiché en lecture : l'empreinte tronquée suffit pour reconnaître un document.
pub fn empreinte_courte(e: &str) -> String {
    e.chars().take(16).collect()
}

/// Libellé d'une décision d'archive.
pub fn libelle_decision(d: &str) -> &'static str {
    match d {
        "versee" => "Versée aux archives de l'établissement",
        "detruite" => "À détruire (décision prise)",
        _ => "À décider à l'échéance",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn j(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    fn ligne(taux: Option<i32>, modifies: i64, a_viser: bool) -> TenueLigneDto {
        TenueLigneDto {
            cours_id: Uuid::nil(),
            libelle: "Maths".into(),
            public: "6A".into(),
            intervenants: vec![],
            seances_passees: 16,
            seances_renseignees: 12,
            taux_pct: taux,
            travaux_donnes: 3,
            derniere_entree: Some(j(11, 12)),
            dernier_visa_le: None,
            modifies_apres_visa: modifies,
            ajoutes_apres_visa: 0,
            a_viser,
        }
    }

    #[test]
    fn la_fenetre_par_defaut_couvre_six_semaines() {
        let (du, au) = periode_par_defaut(j(11, 20));
        assert_eq!(au, j(11, 20));
        assert_eq!((au - du).num_days(), 42);
    }

    #[test]
    fn un_cours_modifie_apres_visa_prime_sur_le_retard_de_visa() {
        assert_eq!(
            etat_ligne(&ligne(Some(75), 2, true)).0,
            "Modifié après visa"
        );
        assert_eq!(etat_ligne(&ligne(Some(75), 0, true)).0, "À viser");
        assert_eq!(etat_ligne(&ligne(Some(75), 0, false)), ("Visé", Ton::Ok));
    }

    #[test]
    fn le_taux_se_lit_en_toutes_lettres_et_sans_seance_on_ne_calcule_rien() {
        assert!(phrase_taux(&ligne(Some(75), 0, false)).contains("12 séance(s) sur 16"));
        assert_eq!(
            phrase_taux(&ligne(None, 0, false)),
            "Aucune séance sur la période"
        );
        let d = phrase_dates(&ligne(Some(75), 0, true));
        assert!(
            d.contains("dernière entrée le") && d.contains("jamais visé"),
            "{d}"
        );
    }

    #[test]
    fn une_observation_vide_n_est_pas_envoyee_et_le_visa_vide_exige_sa_confirmation() {
        let b = corps_visa(Uuid::nil(), j(10, 1), j(11, 1), "   ", false);
        assert!(b.observation.is_none() && !b.confirmer_vide);
        assert_eq!(b.cible, VISA_COURS);
        let b = corps_visa(Uuid::nil(), j(10, 1), j(11, 1), "  Détaillez.  ", true);
        assert_eq!(b.observation.as_deref(), Some("Détaillez."));
        assert!(b.confirmer_vide);
    }

    #[test]
    fn la_confirmation_du_visa_dit_ce_qui_a_ete_fige() {
        let mut v = VisaCahierDto {
            id: Uuid::nil(),
            cible: VISA_COURS.into(),
            libelle_cible: "Maths".into(),
            du: j(10, 1),
            au: j(11, 1),
            vise_le: Utc.with_ymd_and_hms(2026, 11, 2, 12, 0, 0).unwrap(),
            vise_par: "Direction".into(),
            nb_elements: 14,
            vide: false,
            modifies_apres_visa: 0,
            observations: vec![],
        };
        assert!(phrase_visa(&v).contains("14 élément(s) figés"));
        v.vide = true;
        assert!(phrase_visa(&v).contains("cahier vide"));
    }

    #[test]
    fn un_acces_se_decrit_par_son_etat_et_sa_validite() {
        assert_eq!(etat_acces(ACCES_ACTIF), ("Actif", Ton::Ok));
        assert_eq!(etat_acces(ACCES_REVOQUE).1, Ton::Mal);
        assert_eq!(etat_acces("inconnu").0, "Inconnu");
        assert_eq!(libelle_motif("rdv_carriere"), "Rendez-vous de carrière");
    }

    #[test]
    fn l_octroi_exige_un_compte_lisible_un_cours_et_des_dates() {
        let c = Uuid::from_u128(9);
        let u = Uuid::from_u128(1).to_string();
        assert!(corps_acces(
            "pas-un-uuid",
            vec![c],
            "2026-10-01",
            "2026-11-01",
            "",
            "inspection"
        )
        .is_err());
        assert!(corps_acces(&u, vec![], "2026-10-01", "2026-11-01", "", "inspection").is_err());
        assert!(corps_acces(&u, vec![c], "demain", "2026-11-01", "", "inspection").is_err());
        assert!(corps_acces(
            &u,
            vec![c],
            "2026-10-01",
            "2026-11-01",
            "beaucoup",
            "inspection"
        )
        .is_err());
        let b = corps_acces(&u, vec![c], "2026-10-01", "2026-11-01", "", "rdv_carriere").unwrap();
        assert_eq!(
            b.duree_jours, None,
            "la durée par défaut est celle du serveur"
        );
        let b = corps_acces(&u, vec![c], "2026-10-01", "2026-11-01", "45", "inspection").unwrap();
        assert_eq!(b.duree_jours, Some(45));
    }

    #[test]
    fn le_controle_de_cloture_liste_chaque_point_et_se_tait_quand_tout_est_regle() {
        let mut c = ControleClotureDto {
            annee_id: Uuid::nil(),
            annee_libelle: "2026-2027".into(),
            deja_fige: false,
            cours_total: 10,
            cours_non_vises: vec![],
            entrees_brouillon: 0,
            travaux_brouillon: 0,
            non_rendus: 0,
            archives_existantes: 0,
            reouverture_en_attente: None,
        };
        assert!(points_controle(&c).is_empty());
        c.cours_non_vises = vec![CoursRefDto {
            cours_id: Uuid::nil(),
            libelle: "Maths".into(),
            public: "6A".into(),
        }];
        c.entrees_brouillon = 2;
        c.travaux_brouillon = 1;
        c.non_rendus = 4;
        assert_eq!(points_controle(&c).len(), 4);
    }

    #[test]
    fn l_empreinte_est_tronquee_pour_la_lecture() {
        assert_eq!(empreinte_courte(&"a".repeat(64)).len(), 16);
        assert_eq!(
            libelle_decision("versee"),
            "Versée aux archives de l'établissement"
        );
        assert_eq!(libelle_decision("a_decider"), "À décider à l'échéance");
    }
}
