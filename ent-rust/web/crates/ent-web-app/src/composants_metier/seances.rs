//! Séances (pure, testée) : tri, moment par rapport à l'heure, prochain cours, lien d'appel.
//! Partagé par l'accueil, l'emploi du temps et l'appel.

use chrono::NaiveDate;
use chrono::NaiveTime;
use ent_web_api::Creneau;

use crate::noyau::navigation;

use crate::noyau::temps::lire_heure;

/// Séances triées par heure de début puis par classe.
pub fn trier(mut creneaux: Vec<Creneau>) -> Vec<Creneau> {
    creneaux.sort_by(|a, b| {
        a.heure_debut
            .cmp(&b.heure_debut)
            .then(a.classe.cmp(&b.classe))
    });
    creneaux
}

/// Moment d'une séance par rapport à l'heure courante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    EnCours,
    /// Commence dans tant de minutes.
    DansMinutes(i64),
    Terminee,
}

pub fn moment(c: &Creneau, maintenant: NaiveTime) -> Moment {
    let (Some(debut), Some(fin)) = (lire_heure(&c.heure_debut), lire_heure(&c.heure_fin)) else {
        return Moment::Terminee;
    };
    if maintenant >= fin {
        Moment::Terminee
    } else if maintenant >= debut {
        Moment::EnCours
    } else {
        Moment::DansMinutes((debut - maintenant).num_minutes().max(0))
    }
}

/// Séance en cours, sinon la prochaine de la journée (`None` si plus rien).
pub fn prochaine(creneaux: &[Creneau], maintenant: NaiveTime) -> Option<(Creneau, Moment)> {
    creneaux
        .iter()
        .map(|c| (c, moment(c, maintenant)))
        .find(|(_, m)| *m != Moment::Terminee)
        .map(|(c, m)| (c.clone(), m))
}

/// « en cours », « dans 12 min », « dans 1 h 05 ».
pub fn libelle_moment(m: Moment) -> String {
    match m {
        Moment::EnCours => "En cours".into(),
        Moment::DansMinutes(0) => "Commence maintenant".into(),
        Moment::DansMinutes(n) if n < 60 => format!("Dans {n} min"),
        Moment::DansMinutes(n) => format!("Dans {} h {:02}", n / 60, n % 60),
        Moment::Terminee => "Terminé".into(),
    }
}

/// Adresse de l'appel d'une séance.
pub fn lien_appel(c: &Creneau, jour: NaiveDate) -> String {
    format!("{}?jour={jour}&creneau={}", navigation::APPEL, c.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn seance(debut: &str, fin: &str, classe: &str) -> Creneau {
        Creneau {
            id: Uuid::from_u128(1),
            classe: classe.into(),
            matiere_libelle: "Maths".into(),
            prof_user_id: Uuid::nil(),
            prof_label: None,
            jour_semaine: 2,
            heure_debut: debut.into(),
            heure_fin: fin.into(),
            salle: None,
            semaine_ab: "toutes".into(),
        }
    }

    fn h(t: &str) -> NaiveTime {
        NaiveTime::parse_from_str(t, "%H:%M").unwrap()
    }

    #[test]
    fn prochain_cours_selon_l_heure() {
        let jour = trier(vec![
            seance("10:00", "11:00", "5A"),
            seance("08:00", "09:00", "4B"),
        ]);
        assert_eq!(jour[0].classe, "4B", "tri par heure");
        let (c, m) = prochaine(&jour, h("07:48")).unwrap();
        assert_eq!((c.classe.as_str(), m), ("4B", Moment::DansMinutes(12)));
        let (c, m) = prochaine(&jour, h("08:30")).unwrap();
        assert_eq!((c.classe.as_str(), m), ("4B", Moment::EnCours));
        let (c, _) = prochaine(&jour, h("09:00")).unwrap();
        assert_eq!(c.classe, "5A", "une séance finie n'est plus la prochaine");
        assert!(prochaine(&jour, h("11:00")).is_none());
        assert!(prochaine(&[], h("08:00")).is_none(), "journée sans cours");
    }

    #[test]
    fn libelles_de_moment() {
        assert_eq!(libelle_moment(Moment::DansMinutes(12)), "Dans 12 min");
        assert_eq!(libelle_moment(Moment::DansMinutes(65)), "Dans 1 h 05");
        assert_eq!(
            libelle_moment(Moment::DansMinutes(0)),
            "Commence maintenant"
        );
        assert_eq!(libelle_moment(Moment::EnCours), "En cours");
    }
}
