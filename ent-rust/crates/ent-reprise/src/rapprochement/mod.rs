//! **Rapprochement** des référentiels et des personnes entre l'export Next.js décodé (R2) et ScolIA
//! (socle 28, lot R3 = MS4 d). Fonctions **pures** : le rapprochement lit un instantané de ScolIA fourni par
//! `ent-db` et l'extraction du décodage ; il n'écrit rien et ne connaît ni base ni horloge (le jour de
//! référence est un paramètre).
//!
//! Principes (T0 §6, R-MSE22) :
//! * **le cœur fait foi** : aucune fiche n'est créée sans décision ;
//! * **explicable** : chaque proposition porte ses raisons et un niveau de confiance ;
//! * **jamais de fusion sur une ressemblance** : seul un INE unique des deux côtés et sans contradiction, une
//!   clé de référentiel (dates d'année, code de division SIECLE) ou une équivalence validée par une personne
//!   rapproche sans attendre ; tout le reste va dans la file « à décider ».

pub mod decision;
pub mod donnees;
pub mod eleves;
pub mod normalisation;
pub mod personnels;
pub mod referentiels;
pub mod responsables;
pub mod types;

pub use decision::{
    cle_synonyme, eligible_lot, existante_de_decision, valider_decision, ActionDecision,
    DecisionDemandee, PropositionFile, RefusDecision,
};
pub use donnees::{CibleRapprochement, SourceRapprochement, Synonymes};
pub use types::{Candidat, Confiance, Motif, Polarite, Raison, Resume, Verdict};

use crate::correspondance::{Existantes, Statut};
use chrono::NaiveDate;
use std::collections::{BTreeMap, BTreeSet};

/// Objets de l'export dont le rapprochement est traité par ce lot.
pub const OBJETS_R3: &[&str] = &[
    referentiels::OBJET_ANNEE,
    referentiels::OBJET_SITE,
    referentiels::OBJET_CLASSE,
    eleves::OBJET,
    responsables::OBJET,
    personnels::OBJET_PERSONNEL,
    personnels::OBJET_ENSEIGNANT,
    personnels::OBJET_COMPTE,
];

pub fn est_objet_r3(objet: &str) -> bool {
    OBJETS_R3.contains(&objet)
}

/// Verdicts par (objet source, identifiant source).
pub type Verdicts = BTreeMap<(String, String), Verdict>;

/// Contexte d'un rapprochement : le jour de référence (jamais l'horloge) et les correspondances déjà posées.
#[derive(Debug, Clone, Copy)]
pub struct Contexte<'a> {
    pub jour: NaiveDate,
    pub existantes: &'a Existantes,
}

impl<'a> Contexte<'a> {
    pub fn new(jour: NaiveDate, existantes: &'a Existantes) -> Contexte<'a> {
        Contexte { jour, existantes }
    }

    /// Une décision ou une reprise précédente existe : la simulation la rejoue, le rapprochement ne la
    /// recalcule pas. Une ligne simplement « à décider » reste, elle, à rapprocher.
    pub fn deja_connue(&self, objet: &str, source_id: &str) -> bool {
        self.existantes
            .get(&(objet.to_string(), source_id.to_string()))
            .is_some_and(|e| e.statut != Statut::ADecider)
    }

    /// Fiches ScolIA déjà reliées (rapprochées) à une fiche de l'export, pour cet objet.
    pub fn cibles_reliees(&self, objet: &str) -> BTreeSet<String> {
        self.existantes
            .iter()
            .filter(|((o, _), e)| o == objet && e.statut == Statut::Rapprochee)
            .map(|(_, e)| e.cible_id.clone())
            .collect()
    }

    pub fn cible_reliee(&self, objet: &str, cible_id: &str) -> bool {
        self.existantes.iter().any(|((o, _), e)| {
            o == objet && e.statut == Statut::Rapprochee && e.cible_id == cible_id
        })
    }
}

/// Rapproche toutes les fiches d'objets R3 de l'export qui n'ont pas encore de décision.
pub fn rapprocher(
    src: &SourceRapprochement,
    cible: &CibleRapprochement,
    ctx: &Contexte,
) -> Verdicts {
    let mut v = Verdicts::new();
    referentiels::tout(src, cible, ctx, &mut v);
    eleves::rapprocher(&src.eleves, cible, ctx, &mut v);
    // Élèves déjà rapprochés (décision ou règle forte) : ils servent à reconnaître les responsables.
    let mut resolutions: BTreeMap<String, String> = ctx
        .existantes
        .iter()
        .filter(|((o, _), e)| o == eleves::OBJET && e.statut == Statut::Rapprochee)
        .map(|((_, s), e)| (s.clone(), e.cible_id.clone()))
        .collect();
    for ((o, s), verdict) in &v {
        if let (true, Verdict::Automatique { cible_id, .. }) = (o == eleves::OBJET, verdict) {
            resolutions.insert(s.clone(), cible_id.clone());
        }
    }
    responsables::rapprocher(src, cible, &resolutions, ctx, &mut v);
    personnels::rapprocher_personnels(src, cible, ctx, &mut v);
    personnels::rapprocher_comptes(src, cible, ctx, &mut v);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondance::Existante;

    fn j() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
    }

    fn ex(statut: Statut, cible: &str) -> Existante {
        Existante {
            cible_id: cible.into(),
            empreinte_source: None,
            modifie_depuis_reprise: false,
            statut,
        }
    }

    #[test]
    fn une_ligne_a_decider_n_est_pas_consideree_comme_connue() {
        let mut e = Existantes::new();
        e.insert(("eleve".into(), "a".into()), ex(Statut::ADecider, "x"));
        e.insert(("eleve".into(), "b".into()), ex(Statut::Rapprochee, "y"));
        e.insert(("eleve".into(), "c".into()), ex(Statut::Ignoree, "z"));
        e.insert(("eleve".into(), "d".into()), ex(Statut::Creee, "w"));
        let c = Contexte::new(j(), &e);
        assert!(!c.deja_connue("eleve", "a"));
        assert!(
            c.deja_connue("eleve", "b")
                && c.deja_connue("eleve", "c")
                && c.deja_connue("eleve", "d")
        );
        assert!(!c.deja_connue("eleve", "absent"));
        assert_eq!(
            c.cibles_reliees("eleve").into_iter().collect::<Vec<_>>(),
            vec!["y".to_string()]
        );
        assert!(
            c.cible_reliee("eleve", "y")
                && !c.cible_reliee("eleve", "x")
                && !c.cible_reliee("foyer", "y")
        );
    }

    #[test]
    fn le_rapprochement_reconnait_les_responsables_par_les_eleves_rapproches_par_ine() {
        use donnees::*;
        let jour = NaiveDate::from_ymd_opt(2012, 3, 4);
        let mut src = SourceRapprochement::default();
        src.eleves.push(SourceEleve {
            source_id: "e1".into(),
            nom: "Martin".into(),
            prenom: "Léa".into(),
            ine: Some("1234567890A".into()),
            date_naissance: jour,
            classe: None,
            date_sortie: None,
            statut: None,
        });
        src.responsables.push(SourceResponsable {
            source_id: "r1".into(),
            foyer_id: Some("f1".into()),
            nom: "Martin".into(),
            prenom: "Anne".into(),
            email: None,
            telephone: None,
        });
        src.foyers_eleves
            .insert("f1".into(), ["e1".to_string()].into_iter().collect());
        let cible = CibleRapprochement {
            eleves: vec![CibleEleve {
                id: "c1".into(),
                nom: "Martin".into(),
                nom_naissance: None,
                prenom: "Léa".into(),
                prenoms: vec![],
                ine: Some("1234567890A".into()),
                date_naissance: jour,
                classe: None,
                actif: true,
            }],
            personnes: vec![CiblePersonne {
                id: "p1".into(),
                nom: "Martin".into(),
                prenom: Some("Anne".into()),
                email: None,
                telephone: None,
                eleves: vec!["c1".into()],
            }],
            ..Default::default()
        };
        let e = Existantes::new();
        let v = rapprocher(&src, &cible, &Contexte::new(j(), &e));
        assert!(matches!(
            &v[&("eleve".to_string(), "e1".to_string())],
            Verdict::Automatique { .. }
        ));
        let Verdict::ADecider { candidats, .. } =
            &v[&("foyer_responsable".to_string(), "r1".to_string())]
        else {
            panic!()
        };
        assert!(candidats[0].raisons.contains(&Raison::MemeEleve));
        assert_eq!(candidats[0].confiance, Confiance::Forte);
    }

    #[test]
    fn deterministe() {
        let src = SourceRapprochement::default();
        let cible = CibleRapprochement::default();
        let e = Existantes::new();
        assert_eq!(
            rapprocher(&src, &cible, &Contexte::new(j(), &e)),
            rapprocher(&src, &cible, &Contexte::new(j(), &e))
        );
    }
}
