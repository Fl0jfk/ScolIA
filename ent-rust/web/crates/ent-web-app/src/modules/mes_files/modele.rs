//! Logique d'écran pure de « Mes files » (lot P4 du socle 29 ; sans Leptos ni réseau, testée en natif).
//! Aucune règle métier : membres, prises, expirations et droits restent au serveur.

use chrono::{DateTime, Duration, Utc};
use ent_types::afaire::{EtatAfaire, NiveauPrise, OrigineFile, TacheFile, TuileFile};

/// Résumé d'une tuile : des nombres, jamais de contenu. Le sens ne repose jamais sur la couleur seule.
pub fn resume_tuile(t: &TuileFile) -> String {
    let mut parts = vec![match t.ouvertes {
        0 => "Rien à traiter".to_string(),
        1 => "1 à traiter".to_string(),
        n => format!("{n} à traiter"),
    }];
    match t.en_retard {
        Some(0) | None => {}
        Some(1) => parts.push("1 en retard".into()),
        Some(n) => parts.push(format!("{n} en retard")),
    }
    if t.prises_par_moi > 0 {
        parts.push(format!("{} prise(s) par vous", t.prises_par_moi));
    }
    if t.prises_par_autres > 0 {
        parts.push(format!("{} prise(s) par un collègue", t.prises_par_autres));
    }
    parts.join(" · ")
}

/// Adresse où s'ouvre la tuile : le contenu d'une file de l'index est dans cet écran ; une file du
/// socle N1 s'ouvre chez son propriétaire (si une adresse existe).
pub fn lien_tuile(t: &TuileFile) -> Option<String> {
    match t.origine {
        OrigineFile::Afaire => Some(format!("/web/mes-files/{}", t.code)),
        OrigineFile::Demandes => t.lien.clone(),
    }
}

/// Texte de l'ancienneté : relatif à `maintenant`, passé par l'appelant (pas d'horloge ici).
pub fn libelle_plus_ancienne(
    d: Option<DateTime<Utc>>,
    maintenant: DateTime<Utc>,
) -> Option<String> {
    let d = d?;
    if d < maintenant - Duration::hours(1) {
        Some(format!("Depuis le {}", d.format("%d/%m/%Y")))
    } else {
        Some("Récent".into())
    }
}

/// État d'une tâche dans la file, dit en toutes lettres.
pub fn libelle_etat(t: &TacheFile) -> String {
    match (t.etat, t.prises.first(), &t.prepare_par) {
        (EtatAfaire::Prise, Some(p), _) if p.par_moi => match p.niveau {
            NiveauPrise::Preparer => "Vous la préparez".to_string(),
            NiveauPrise::Decider => "Prise par vous".to_string(),
        },
        (EtatAfaire::Prise, Some(p), _) => match p.niveau {
            NiveauPrise::Preparer => format!("En préparation par {}", p.par),
            NiveauPrise::Decider => format!("Prise par {}", p.par),
        },
        (EtatAfaire::EnAttente, _, Some(par)) => format!("Préparée par {par}, à décider"),
        (EtatAfaire::EnAttente, _, None) => "En attente".to_string(),
        _ => "À prendre".to_string(),
    }
}

/// Une prise « décider » libre : la tâche n'est pas prise par un collègue (le serveur tranche en cas de course).
pub fn peut_prendre(t: &TacheFile, niveau: NiveauPrise) -> bool {
    !t.prises.iter().any(|p| p.niveau == niveau)
}

/// Une prise de moi, à ce niveau.
pub fn prise_par_moi(t: &TacheFile, niveau: NiveauPrise) -> bool {
    t.prises.iter().any(|p| p.par_moi && p.niveau == niveau)
}

/// Délai médian en clair (jamais une précision trompeuse).
pub fn libelle_delai(minutes: Option<i64>) -> String {
    match minutes {
        None => "—".to_string(),
        Some(m) if m < 60 => format!("{m} min"),
        Some(m) if m < 24 * 60 => format!("{} h", m / 60),
        Some(m) => format!("{} j", m / (24 * 60)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::afaire::{PrioriteAfaire, PriseVue};
    use uuid::Uuid;

    fn t0() -> DateTime<Utc> {
        "2026-11-16T10:00:00Z".parse().unwrap()
    }

    fn tuile(origine: OrigineFile) -> TuileFile {
        TuileFile {
            code: "vs.justificatifs".into(),
            libelle: "Vie scolaire".into(),
            origine,
            ouvertes: 3,
            en_retard: Some(1),
            prises_par_moi: 1,
            prises_par_autres: 0,
            plus_ancienne: None,
            lien: Some("/web/absences".into()),
        }
    }

    fn tache(etat: EtatAfaire, prises: Vec<PriseVue>, prepare_par: Option<&str>) -> TacheFile {
        TacheFile {
            id: Uuid::nil(),
            nature: "vs.trancher_justificatif".into(),
            titre: "Justificatif à trancher — Léa".into(),
            eleve_prenom: Some("Léa".into()),
            priorite: PrioriteAfaire::AFaire,
            etat,
            echeance_le: None,
            prises,
            prepare_par: prepare_par.map(str::to_string),
            action: None,
        }
    }

    fn prise(par_moi: bool, niveau: NiveauPrise) -> PriseVue {
        PriseVue {
            par_moi,
            par: "Camille D.".into(),
            niveau,
            depuis: t0(),
            jusqu_a: t0() + Duration::days(2),
        }
    }

    #[test]
    fn le_resume_donne_des_nombres_au_singulier_et_au_pluriel() {
        let mut t = tuile(OrigineFile::Afaire);
        assert_eq!(
            resume_tuile(&t),
            "3 à traiter · 1 en retard · 1 prise(s) par vous"
        );
        t.ouvertes = 0;
        t.en_retard = Some(0);
        t.prises_par_moi = 0;
        assert_eq!(resume_tuile(&t), "Rien à traiter");
        // Une file de demandes ne publie pas de retard : rien n'est affirmé, jamais un faux zéro.
        t.ouvertes = 1;
        t.en_retard = None;
        assert_eq!(resume_tuile(&t), "1 à traiter");
    }

    #[test]
    fn une_file_de_l_index_s_ouvre_ici_une_file_de_demandes_chez_son_proprietaire() {
        assert_eq!(
            lien_tuile(&tuile(OrigineFile::Afaire)).as_deref(),
            Some("/web/mes-files/vs.justificatifs")
        );
        assert_eq!(
            lien_tuile(&tuile(OrigineFile::Demandes)).as_deref(),
            Some("/web/absences")
        );
        let mut sans = tuile(OrigineFile::Demandes);
        sans.lien = None;
        assert_eq!(lien_tuile(&sans), None);
    }

    #[test]
    fn l_etat_de_la_tache_nomme_qui_l_a_prise_et_a_quel_niveau() {
        assert_eq!(
            libelle_etat(&tache(EtatAfaire::Ouverte, vec![], None)),
            "À prendre"
        );
        assert_eq!(
            libelle_etat(&tache(
                EtatAfaire::Prise,
                vec![prise(true, NiveauPrise::Decider)],
                None
            )),
            "Prise par vous"
        );
        assert_eq!(
            libelle_etat(&tache(
                EtatAfaire::Prise,
                vec![prise(false, NiveauPrise::Decider)],
                None
            )),
            "Prise par Camille D."
        );
        assert_eq!(
            libelle_etat(&tache(
                EtatAfaire::Prise,
                vec![prise(false, NiveauPrise::Preparer)],
                None
            )),
            "En préparation par Camille D."
        );
        assert_eq!(
            libelle_etat(&tache(EtatAfaire::EnAttente, vec![], Some("Camille D."))),
            "Préparée par Camille D., à décider"
        );
    }

    /// Anomalie évitée : le bouton « Prendre » n'est pas proposé sur ce qu'un collègue traite déjà.
    #[test]
    fn on_ne_propose_pas_de_prendre_ce_qui_est_deja_pris() {
        let libre = tache(EtatAfaire::Ouverte, vec![], None);
        assert!(peut_prendre(&libre, NiveauPrise::Decider));
        let prise_autre = tache(
            EtatAfaire::Prise,
            vec![prise(false, NiveauPrise::Decider)],
            None,
        );
        assert!(!peut_prendre(&prise_autre, NiveauPrise::Decider));
        assert!(!prise_par_moi(&prise_autre, NiveauPrise::Decider));
        // Une préparation en cours n'empêche pas la décision d'un autre niveau.
        let preparation = tache(
            EtatAfaire::Prise,
            vec![prise(true, NiveauPrise::Preparer)],
            None,
        );
        assert!(peut_prendre(&preparation, NiveauPrise::Decider));
        assert!(prise_par_moi(&preparation, NiveauPrise::Preparer));
    }

    #[test]
    fn le_delai_median_est_dit_en_clair() {
        assert_eq!(libelle_delai(None), "—");
        assert_eq!(libelle_delai(Some(45)), "45 min");
        assert_eq!(libelle_delai(Some(180)), "3 h");
        assert_eq!(libelle_delai(Some(3 * 24 * 60 + 5)), "3 j");
    }

    #[test]
    fn l_anciennete_est_relative_a_l_instant_fourni() {
        assert_eq!(libelle_plus_ancienne(None, t0()), None);
        assert_eq!(
            libelle_plus_ancienne(Some(t0() - Duration::minutes(5)), t0()).as_deref(),
            Some("Récent")
        );
        assert_eq!(
            libelle_plus_ancienne(Some(t0() - Duration::days(3)), t0()).as_deref(),
            Some("Depuis le 13/11/2026")
        );
    }
}
