//! Rapprochement des **responsables** (lot R3, T0 §6.6) : une fiche `foyer_responsable` du Next.js est-elle
//! déjà une personne de ScolIA ?
//!
//! Aucune règle automatique : un responsable n'a pas d'identifiant national. Les raisons sont l'élève
//! (le foyer du responsable contient un élève déjà rapproché auquel la personne est liée dans ScolIA), l'adresse
//! e-mail, le téléphone, le nom et le prénom. Une adresse **partagée** par plusieurs personnes (fréquent entre
//! deux parents) ne désigne personne : elle plafonne la confiance et interdit la décision en lot.

use super::donnees::{CiblePersonne, CibleRapprochement, SourceRapprochement, SourceResponsable};
use super::normalisation::{cle_nom, cle_telephone, email_masque, email_normalise};
use super::types::{classer, motif_de, Candidat, Confiance, Raison, Resume, Verdict};
use super::{Contexte, Verdicts};
use std::collections::{BTreeMap, BTreeSet};

pub const OBJET: &str = "foyer_responsable";
const MAX_CANDIDATS: usize = 5;

pub fn resume_source(r: &SourceResponsable) -> Resume {
    Resume {
        nom: Some(r.nom.clone()).filter(|n| !n.is_empty()),
        prenom: Some(r.prenom.clone()).filter(|n| !n.is_empty()),
        naissance: None,
        classe: None,
        ine_fin: None,
        contact_masque: r.email.as_deref().and_then(email_masque),
        detail: None,
    }
}

pub fn resume_cible(p: &CiblePersonne) -> Resume {
    Resume {
        nom: Some(p.nom.clone()),
        prenom: p.prenom.clone(),
        naissance: None,
        classe: None,
        ine_fin: None,
        contact_masque: p.email.as_deref().and_then(email_masque),
        detail: None,
    }
}

fn identite(nom: &str, prenom: &str) -> (String, String) {
    (cle_nom(nom), cle_nom(prenom))
}

/// `resolutions` : élève source → fiche ScolIA, **seulement** pour les élèves déjà rapprochés.
pub fn rapprocher(
    src: &SourceRapprochement,
    cible: &CibleRapprochement,
    resolutions: &BTreeMap<String, String>,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    // Combien de personnes distinctes portent chaque adresse, dans l'export et dans ScolIA.
    let mut email_src: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for r in &src.responsables {
        if let Some(e) = r.email.as_deref().and_then(email_normalise) {
            email_src
                .entry(e)
                .or_default()
                .insert(identite(&r.nom, &r.prenom));
        }
    }
    let mut email_cible: BTreeMap<String, Vec<&CiblePersonne>> = BTreeMap::new();
    for p in &cible.personnes {
        if let Some(e) = p.email.as_deref().and_then(email_normalise) {
            email_cible.entry(e).or_default().push(p);
        }
    }
    let prises = ctx.cibles_reliees(OBJET);
    let mut bruts: Vec<(&SourceResponsable, Vec<Candidat>)> = Vec::new();
    for r in &src.responsables {
        if ctx.deja_connue(OBJET, &r.source_id) {
            continue;
        }
        let eleves_cibles: BTreeSet<&String> = r
            .foyer_id
            .as_ref()
            .and_then(|f| src.foyers_eleves.get(f))
            .into_iter()
            .flatten()
            .filter_map(|e| resolutions.get(e))
            .collect();
        let email = r.email.as_deref().and_then(email_normalise);
        let tel = r.telephone.as_deref().and_then(cle_telephone);
        let (nom, prenom) = identite(&r.nom, &r.prenom);
        let partage_source = email
            .as_ref()
            .and_then(|e| email_src.get(e))
            .is_some_and(|s| s.len() > 1);
        let mut candidats = Vec::new();
        for p in &cible.personnes {
            let meme_eleve = p.eleves.iter().any(|e| eleves_cibles.contains(e));
            let meme_email =
                email.is_some() && p.email.as_deref().and_then(email_normalise) == email;
            let meme_tel = tel.is_some() && p.telephone.as_deref().and_then(cle_telephone) == tel;
            let (pn, pp) = identite(&p.nom, p.prenom.as_deref().unwrap_or_default());
            let nom_id = !nom.is_empty() && nom == pn;
            let prenom_id = !prenom.is_empty() && prenom == pp;
            if !(meme_eleve || meme_email || meme_tel || (nom_id && prenom_id)) {
                continue;
            }
            // Un lien d'élève ou un contact commun sans aucun autre rapport avec le nom n'est pas une raison.
            if !(nom_id || prenom_id || meme_email || meme_tel) {
                continue;
            }
            let mut raisons = Vec::new();
            if meme_eleve {
                raisons.push(Raison::MemeEleve);
            }
            if meme_email {
                raisons.push(Raison::EmailIdentique);
            }
            if meme_tel {
                raisons.push(Raison::TelephoneIdentique);
            }
            if nom_id {
                raisons.push(Raison::NomIdentique);
            }
            if prenom_id {
                raisons.push(Raison::PrenomIdentique);
            }
            if (meme_email || meme_tel) && !nom.is_empty() && !pn.is_empty() && !nom_id {
                raisons.push(Raison::NomDifferent);
            }
            let partage = meme_email
                && (partage_source
                    || email
                        .as_ref()
                        .and_then(|e| email_cible.get(e))
                        .is_some_and(|v| v.len() > 1));
            if partage {
                raisons.push(Raison::EmailPartage);
            }
            let contact = meme_email || meme_tel;
            let mut conf = match (meme_eleve, contact, nom_id, prenom_id) {
                (true, _, true, true) | (_, true, true, true) | (true, true, true, _) => {
                    Confiance::Forte
                }
                (true, _, true, false) | (false, true, true, false) | (true, true, false, _) => {
                    Confiance::Moyenne
                }
                _ => Confiance::Faible,
            };
            if partage || raisons.contains(&Raison::NomDifferent) {
                conf = conf.min(Confiance::Moyenne);
            }
            candidats.push(Candidat {
                cible_id: p.id.clone(),
                confiance: conf,
                raisons,
                resume: resume_cible(p),
            });
        }
        bruts.push((r, candidats));
    }
    let mut demandes: BTreeMap<String, usize> = BTreeMap::new();
    for (_, cs) in &bruts {
        for c in cs {
            *demandes.entry(c.cible_id.clone()).or_default() += 1;
        }
    }
    for (r, mut cs) in bruts {
        // Deux parents d'un même foyer qui proposent la même fiche : à regarder de près.
        let fortes = cs
            .iter()
            .filter(|c| c.confiance == Confiance::Forte)
            .count();
        for c in &mut cs {
            if prises.contains(&c.cible_id) {
                c.raisons.push(Raison::CibleDejaReliee);
            }
            if demandes.get(&c.cible_id).copied().unwrap_or(0) > 1 {
                c.raisons.push(Raison::CiblePlusieursSources);
            }
            if fortes > 1 && c.confiance == Confiance::Forte {
                c.raisons.push(Raison::AmbiguiteDansLeCoeur);
            }
            if c.a_contre() && c.confiance == Confiance::Forte {
                c.confiance = Confiance::Moyenne;
            }
        }
        let cs = classer(cs, MAX_CANDIDATS);
        verdicts.insert(
            (OBJET.to_string(), r.source_id.clone()),
            Verdict::ADecider {
                motif: motif_de(&cs),
                candidats: cs,
                resume_source: resume_source(r),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondance::Existantes;
    use chrono::NaiveDate;

    fn resp(
        id: &str,
        foyer: &str,
        nom: &str,
        prenom: &str,
        email: Option<&str>,
    ) -> SourceResponsable {
        SourceResponsable {
            source_id: id.into(),
            foyer_id: Some(foyer.into()),
            nom: nom.into(),
            prenom: prenom.into(),
            email: email.map(str::to_string),
            telephone: None,
        }
    }

    fn pers(
        id: &str,
        nom: &str,
        prenom: &str,
        email: Option<&str>,
        eleves: &[&str],
    ) -> CiblePersonne {
        CiblePersonne {
            id: id.into(),
            nom: nom.into(),
            prenom: Some(prenom.into()),
            email: email.map(str::to_string),
            telephone: None,
            eleves: eleves.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn lancer(
        resps: Vec<SourceResponsable>,
        cibles: Vec<CiblePersonne>,
        res: &[(&str, &str)],
    ) -> Verdicts {
        let mut src = SourceRapprochement {
            responsables: resps,
            ..Default::default()
        };
        src.foyers_eleves
            .insert("f1".into(), ["e1".to_string()].into_iter().collect());
        let cible = CibleRapprochement {
            personnes: cibles,
            ..Default::default()
        };
        let existantes = Existantes::new();
        let ctx = Contexte::new(NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(), &existantes);
        let resolutions = res
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        let mut v = Verdicts::new();
        rapprocher(&src, &cible, &resolutions, &ctx, &mut v);
        v
    }

    fn cands<'a>(v: &'a Verdicts, id: &str) -> &'a [Candidat] {
        match &v[&(OBJET.to_string(), id.to_string())] {
            Verdict::ADecider { candidats, .. } => candidats,
            autre => panic!("jamais automatique : {autre:?}"),
        }
    }

    #[test]
    fn meme_eleve_meme_nom_et_email_donne_forte_jamais_automatique() {
        let v = lancer(
            vec![resp(
                "r1",
                "f1",
                "Dupont",
                "Anne",
                Some("Anne@Example.test"),
            )],
            vec![pers(
                "p1",
                "DUPONT",
                "anne",
                Some("anne@example.test"),
                &["c1"],
            )],
            &[("e1", "c1")],
        );
        let c = cands(&v, "r1");
        assert_eq!(c[0].confiance, Confiance::Forte);
        assert!(c[0].raisons.contains(&Raison::MemeEleve));
        assert!(c[0].raisons.contains(&Raison::EmailIdentique));
    }

    #[test]
    fn email_partage_par_deux_parents_plafonne_la_confiance() {
        let v = lancer(
            vec![
                resp("r1", "f1", "Dupont", "Anne", Some("famille@example.test")),
                resp("r2", "f1", "Dupont", "Paul", Some("famille@example.test")),
            ],
            vec![
                pers(
                    "p1",
                    "Dupont",
                    "Anne",
                    Some("famille@example.test"),
                    &["c1"],
                ),
                pers("p2", "Dupont", "Paul", None, &["c1"]),
            ],
            &[("e1", "c1")],
        );
        for id in ["r1", "r2"] {
            let c = cands(&v, id);
            assert!(
                c.iter().any(|x| x.raisons.contains(&Raison::EmailPartage)),
                "{id}"
            );
            // Aucun candidat appuyé sur l'adresse partagée ne dépasse « moyenne ».
            assert!(c
                .iter()
                .filter(|x| x.raisons.contains(&Raison::EmailPartage))
                .all(|x| x.confiance <= Confiance::Moyenne));
        }
    }

    #[test]
    fn meme_email_mais_nom_different_est_contradictoire() {
        let v = lancer(
            vec![resp("r1", "f1", "Dupont", "Anne", Some("a@example.test"))],
            vec![pers("p1", "Martin", "Julie", Some("a@example.test"), &[])],
            &[],
        );
        let c = cands(&v, "r1");
        assert!(c[0].raisons.contains(&Raison::NomDifferent));
        assert!(c[0].confiance <= Confiance::Moyenne);
    }

    #[test]
    fn eleve_non_rapproche_le_lien_n_est_pas_une_raison() {
        // L'élève du foyer n'est pas encore rapproché : le lien d'élève ne compte pas, ni le nom différent.
        let v = lancer(
            vec![resp("r1", "f1", "Dupont", "Anne", None)],
            vec![pers("p1", "Martin", "Julie", None, &["c1"])],
            &[],
        );
        assert!(cands(&v, "r1").is_empty());
    }

    #[test]
    fn meme_nom_et_prenom_sans_autre_indice_reste_faible() {
        let v = lancer(
            vec![resp("r1", "f1", "Dupont", "Anne", None)],
            vec![pers("p1", "Dupont", "Anne", None, &[])],
            &[],
        );
        assert_eq!(cands(&v, "r1")[0].confiance, Confiance::Faible);
    }

    #[test]
    fn aucun_candidat_laisse_la_creation_a_une_decision() {
        let v = lancer(
            vec![resp("r1", "f1", "Inconnu", "Parent", None)],
            vec![],
            &[],
        );
        assert!(cands(&v, "r1").is_empty());
        let Verdict::ADecider {
            motif,
            resume_source,
            ..
        } = &v[&(OBJET.to_string(), "r1".to_string())]
        else {
            panic!()
        };
        assert_eq!(*motif, super::super::types::Motif::AucunCandidat);
        assert_eq!(resume_source.nom.as_deref(), Some("Inconnu"));
    }

    #[test]
    fn resume_masque_l_adresse() {
        let r = resp(
            "r1",
            "f1",
            "Dupont",
            "Anne",
            Some("anne.dupont@example.test"),
        );
        let res = resume_source(&r);
        assert_eq!(res.contact_masque.as_deref(), Some("a***@example.test"));
        assert!(!format!("{res:?}").contains("anne.dupont"));
    }

    #[test]
    fn telephone_identique_est_une_raison_avec_le_nom() {
        let mut r = resp("r1", "f1", "Dupont", "Anne", None);
        r.telephone = Some("06 12 34 56 78".into());
        let mut p = pers("p1", "Dupont", "Anne", None, &[]);
        p.telephone = Some("+33612345678".into());
        let v = lancer(vec![r], vec![p], &[]);
        let c = cands(&v, "r1");
        assert!(c[0].raisons.contains(&Raison::TelephoneIdentique));
        assert_eq!(c[0].confiance, Confiance::Forte);
    }
}
