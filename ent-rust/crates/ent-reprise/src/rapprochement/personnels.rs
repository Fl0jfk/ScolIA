//! Rapprochement des **personnels**, des copies d'**enseignants** et des **comptes** (lot R3, T0 §6.5 et §6.6).
//!
//! L'e-mail professionnel est la clé la plus utile, mais il n'est pas une identité nationale : une adresse de
//! service (« direction@ ») ou un changement de personne la rend trompeuse. Elle donne donc une confiance
//! **forte** au plus, jamais un rapprochement automatique. L'e-mail personnel n'est qu'un indice (T0 §6.5).
//! Un compte « administrateur de la plateforme » du Next.js n'a pas d'équivalent dans l'établissement.

use super::donnees::{
    CibleAgent, CibleCompte, CibleRapprochement, SourceCompte, SourcePersonnel, SourceRapprochement,
};
use super::normalisation::{cle_nom, email_masque, email_normalise};
use super::types::{classer, motif_de, Candidat, Confiance, Raison, Resume, Verdict};
use super::{Contexte, Verdicts};
use std::collections::{BTreeMap, BTreeSet};

pub const OBJET_PERSONNEL: &str = "personnel";
pub const OBJET_ENSEIGNANT: &str = "enseignant";
pub const OBJET_COMPTE: &str = "user";
const MAX_CANDIDATS: usize = 5;

pub fn resume_source(p: &SourcePersonnel) -> Resume {
    Resume {
        nom: Some(p.nom.clone()).filter(|n| !n.is_empty()),
        prenom: Some(p.prenom.clone()).filter(|n| !n.is_empty()),
        contact_masque: p
            .email_pro
            .as_deref()
            .or(p.email.as_deref())
            .and_then(email_masque),
        detail: p.categorie.clone(),
        ..Default::default()
    }
}

pub fn resume_agent(a: &CibleAgent) -> Resume {
    Resume {
        nom: Some(a.nom.clone()),
        prenom: a.prenom.clone(),
        contact_masque: a.emails.first().and_then(|e| email_masque(e)),
        ..Default::default()
    }
}

fn emails_normalises(p: &SourcePersonnel) -> (Option<String>, Option<String>, Option<String>) {
    let n = |o: &Option<String>| o.as_deref().and_then(email_normalise);
    (n(&p.email_pro), n(&p.email), n(&p.email_perso))
}

pub fn rapprocher_personnels(
    src: &SourceRapprochement,
    cible: &CibleRapprochement,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    // Adresses portées par plusieurs personnes distinctes (une copie `enseignant` d'un `personnel` n'est pas
    // « une autre personne » : même nom et même prénom).
    let mut partage_src: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for p in &src.personnels {
        let (pro, mail, _) = emails_normalises(p);
        for e in [pro, mail].into_iter().flatten() {
            partage_src
                .entry(e)
                .or_default()
                .insert((cle_nom(&p.nom), cle_nom(&p.prenom)));
        }
    }
    let mut partage_cible: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for a in &cible.agents {
        for e in a.emails.iter().filter_map(|e| email_normalise(e)) {
            partage_cible.entry(e).or_default().insert(&a.id);
        }
    }
    let prises_p = ctx.cibles_reliees(OBJET_PERSONNEL);
    let prises_e = ctx.cibles_reliees(OBJET_ENSEIGNANT);
    let mut bruts: Vec<(&SourcePersonnel, Vec<Candidat>)> = Vec::new();
    for p in &src.personnels {
        if ctx.deja_connue(p.objet, &p.source_id) {
            continue;
        }
        let (pro, mail, perso) = emails_normalises(p);
        let (nom, prenom) = (cle_nom(&p.nom), cle_nom(&p.prenom));
        let mut candidats = Vec::new();
        for a in &cible.agents {
            let emails: BTreeSet<String> =
                a.emails.iter().filter_map(|e| email_normalise(e)).collect();
            let via_pro = pro.as_ref().is_some_and(|e| emails.contains(e));
            let via_mail = mail.as_ref().is_some_and(|e| emails.contains(e));
            let via_perso = perso.as_ref().is_some_and(|e| emails.contains(e));
            let an = cle_nom(&a.nom);
            let ap = cle_nom(a.prenom.as_deref().unwrap_or_default());
            let nom_id = !nom.is_empty() && nom == an;
            let prenom_id = !prenom.is_empty() && prenom == ap;
            if !(via_pro || via_mail || via_perso || (nom_id && prenom_id)) {
                continue;
            }
            let mut raisons = Vec::new();
            if via_pro {
                raisons.push(Raison::EmailProIdentique);
            }
            if via_mail {
                raisons.push(Raison::EmailIdentique);
            }
            if via_perso {
                raisons.push(Raison::EmailPersonnelIdentique);
            }
            if nom_id {
                raisons.push(Raison::NomIdentique);
            }
            if prenom_id {
                raisons.push(Raison::PrenomIdentique);
            }
            let par_email = via_pro || via_mail;
            if par_email && !nom.is_empty() && !an.is_empty() && !nom_id {
                raisons.push(Raison::NomDifferent);
            }
            let partage = [&pro, &mail].into_iter().flatten().any(|e| {
                partage_src.get(e).is_some_and(|s| s.len() > 1)
                    || partage_cible.get(e).is_some_and(|s| s.len() > 1)
            }) && par_email;
            if partage {
                raisons.push(Raison::EmailPartage);
            }
            let mut conf = if par_email && nom_id && (via_pro || prenom_id) {
                Confiance::Forte
            } else if par_email || (nom_id && prenom_id) {
                Confiance::Moyenne
            } else {
                Confiance::Faible
            };
            if partage || raisons.contains(&Raison::NomDifferent) {
                conf = conf.min(Confiance::Moyenne);
            }
            candidats.push(Candidat {
                cible_id: a.id.clone(),
                confiance: conf,
                raisons,
                resume: resume_agent(a),
            });
        }
        bruts.push((p, candidats));
    }
    // Un personnel et sa copie d'enseignant proposent légitimement le même agent : ce n'est pas un conflit.
    let mut sources_par_cible: BTreeMap<&str, BTreeSet<(String, String)>> = BTreeMap::new();
    for (p, cs) in &bruts {
        for c in cs {
            sources_par_cible
                .entry(&c.cible_id)
                .or_default()
                .insert((cle_nom(&p.nom), cle_nom(&p.prenom)));
        }
    }
    let conflits: BTreeSet<String> = sources_par_cible
        .iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(c, _)| (*c).to_string())
        .collect();
    for (p, mut cs) in bruts {
        let fortes = cs
            .iter()
            .filter(|c| c.confiance == Confiance::Forte)
            .count();
        let prises = if p.objet == OBJET_PERSONNEL {
            &prises_p
        } else {
            &prises_e
        };
        for c in &mut cs {
            if prises.contains(&c.cible_id) {
                c.raisons.push(Raison::CibleDejaReliee);
            }
            if conflits.contains(&c.cible_id) {
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
            (p.objet.to_string(), p.source_id.clone()),
            Verdict::ADecider {
                motif: motif_de(&cs),
                candidats: cs,
                resume_source: resume_source(p),
            },
        );
    }
}

pub fn resume_compte_source(c: &SourceCompte) -> Resume {
    Resume {
        nom: c.nom.clone(),
        prenom: c.prenom.clone(),
        contact_masque: c.email.as_deref().and_then(email_masque),
        ..Default::default()
    }
}

pub fn resume_compte_cible(c: &CibleCompte) -> Resume {
    Resume {
        contact_masque: email_masque(&c.email),
        ..Default::default()
    }
}

/// Comptes : clé = e-mail normalisé (T0 §6.6), jamais automatique.
pub fn rapprocher_comptes(
    src: &SourceRapprochement,
    cible: &CibleRapprochement,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    let mut doubles: BTreeMap<String, usize> = BTreeMap::new();
    for c in &src.comptes {
        if let Some(e) = c.email.as_deref().and_then(email_normalise) {
            *doubles.entry(e).or_default() += 1;
        }
    }
    let mut par_email: BTreeMap<String, Vec<&CibleCompte>> = BTreeMap::new();
    for c in &cible.comptes {
        if let Some(e) = email_normalise(&c.email) {
            par_email.entry(e).or_default().push(c);
        }
    }
    let prises = ctx.cibles_reliees(OBJET_COMPTE);
    for c in &src.comptes {
        if ctx.deja_connue(OBJET_COMPTE, &c.source_id) {
            continue;
        }
        let email = c.email.as_deref().and_then(email_normalise);
        let mut candidats = Vec::new();
        if let Some(e) = &email {
            for k in par_email.get(e).map(Vec::as_slice).unwrap_or_default() {
                let mut raisons = vec![Raison::EmailIdentique];
                let mut conf = Confiance::Forte;
                if doubles.get(e).copied().unwrap_or(0) > 1 || par_email[e].len() > 1 {
                    raisons.push(Raison::EmailPartage);
                    conf = Confiance::Moyenne;
                }
                if c.administrateur_plateforme {
                    raisons.push(Raison::AdministrateurPlateforme);
                    conf = Confiance::Faible;
                }
                if prises.contains(&k.id) {
                    raisons.push(Raison::CibleDejaReliee);
                    conf = conf.min(Confiance::Moyenne);
                }
                candidats.push(Candidat {
                    cible_id: k.id.clone(),
                    confiance: conf,
                    raisons,
                    resume: resume_compte_cible(k),
                });
            }
        }
        let cs = classer(candidats, MAX_CANDIDATS);
        verdicts.insert(
            (OBJET_COMPTE.to_string(), c.source_id.clone()),
            Verdict::ADecider {
                motif: motif_de(&cs),
                candidats: cs,
                resume_source: resume_compte_source(c),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondance::Existantes;
    use chrono::NaiveDate;

    fn pers(
        objet: &'static str,
        id: &str,
        nom: &str,
        prenom: &str,
        pro: Option<&str>,
        mail: Option<&str>,
    ) -> SourcePersonnel {
        SourcePersonnel {
            objet,
            source_id: id.into(),
            nom: nom.into(),
            prenom: prenom.into(),
            email_pro: pro.map(str::to_string),
            email: mail.map(str::to_string),
            email_perso: None,
            categorie: Some("enseignant".into()),
        }
    }

    fn agent(id: &str, nom: &str, prenom: &str, emails: &[&str]) -> CibleAgent {
        CibleAgent {
            id: id.into(),
            nom: nom.into(),
            prenom: Some(prenom.into()),
            emails: emails.iter().map(|e| e.to_string()).collect(),
        }
    }

    fn lancer(ps: Vec<SourcePersonnel>, agents: Vec<CibleAgent>) -> Verdicts {
        let src = SourceRapprochement {
            personnels: ps,
            ..Default::default()
        };
        let cible = CibleRapprochement {
            agents,
            ..Default::default()
        };
        let existantes = Existantes::new();
        let ctx = Contexte::new(NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(), &existantes);
        let mut v = Verdicts::new();
        rapprocher_personnels(&src, &cible, &ctx, &mut v);
        v
    }

    fn cands<'a>(v: &'a Verdicts, objet: &str, id: &str) -> &'a [Candidat] {
        match &v[&(objet.to_string(), id.to_string())] {
            Verdict::ADecider { candidats, .. } => candidats,
            autre => panic!("jamais automatique : {autre:?}"),
        }
    }

    #[test]
    fn email_pro_et_nom_donnent_forte_jamais_automatique() {
        let v = lancer(
            vec![pers(
                "personnel",
                "p1",
                "Bernard",
                "Claire",
                Some("C.Bernard@college.test"),
                None,
            )],
            vec![agent(
                "a1",
                "Bernard",
                "Claire",
                &["c.bernard@college.test"],
            )],
        );
        let c = cands(&v, "personnel", "p1");
        assert_eq!(c[0].confiance, Confiance::Forte);
        assert!(c[0].raisons.contains(&Raison::EmailProIdentique));
    }

    #[test]
    fn email_personnel_seul_n_est_qu_un_indice_faible() {
        let mut p = pers("personnel", "p1", "Bernard", "Claire", None, None);
        p.email_perso = Some("claire@example.test".into());
        let v = lancer(
            vec![p],
            vec![agent("a1", "Autre", "Personne", &["claire@example.test"])],
        );
        let c = cands(&v, "personnel", "p1");
        assert_eq!(c[0].confiance, Confiance::Faible);
        assert!(c[0].raisons.contains(&Raison::EmailPersonnelIdentique));
    }

    #[test]
    fn adresse_de_service_partagee_plafonne() {
        let v = lancer(
            vec![
                pers(
                    "personnel",
                    "p1",
                    "Bernard",
                    "Claire",
                    Some("direction@college.test"),
                    None,
                ),
                pers(
                    "personnel",
                    "p2",
                    "Petit",
                    "Luc",
                    Some("direction@college.test"),
                    None,
                ),
            ],
            vec![agent(
                "a1",
                "Bernard",
                "Claire",
                &["direction@college.test"],
            )],
        );
        for id in ["p1", "p2"] {
            let c = cands(&v, "personnel", id);
            assert!(c[0].confiance <= Confiance::Moyenne, "{id}");
            assert!(c[0].raisons.contains(&Raison::EmailPartage));
        }
    }

    #[test]
    fn copie_enseignant_et_personnel_proposent_la_meme_fiche_sans_conflit() {
        let v = lancer(
            vec![
                pers(
                    "personnel",
                    "p1",
                    "Bernard",
                    "Claire",
                    Some("c.bernard@college.test"),
                    None,
                ),
                pers(
                    "enseignant",
                    "e1",
                    "Bernard",
                    "Claire",
                    Some("c.bernard@college.test"),
                    None,
                ),
            ],
            vec![agent(
                "a1",
                "Bernard",
                "Claire",
                &["c.bernard@college.test"],
            )],
        );
        for (o, id) in [("personnel", "p1"), ("enseignant", "e1")] {
            let c = cands(&v, o, id);
            assert!(
                !c[0].raisons.contains(&Raison::CiblePlusieursSources),
                "{o}"
            );
            assert!(!c[0].raisons.contains(&Raison::EmailPartage), "{o}");
            assert_eq!(c[0].confiance, Confiance::Forte);
        }
    }

    #[test]
    fn nom_et_prenom_sans_email_restent_a_decider_en_moyenne() {
        let v = lancer(
            vec![pers("personnel", "p1", "Bernard", "Claire", None, None)],
            vec![agent("a1", "Bernard", "Claire", &[])],
        );
        assert_eq!(
            cands(&v, "personnel", "p1")[0].confiance,
            Confiance::Moyenne
        );
    }

    #[test]
    fn email_pro_avec_nom_different_est_contradictoire() {
        let v = lancer(
            vec![pers(
                "personnel",
                "p1",
                "Bernard",
                "Claire",
                Some("x@college.test"),
                None,
            )],
            vec![agent("a1", "Petit", "Luc", &["x@college.test"])],
        );
        let c = cands(&v, "personnel", "p1");
        assert!(c[0].raisons.contains(&Raison::NomDifferent));
        assert!(c[0].confiance <= Confiance::Moyenne);
    }

    #[test]
    fn comptes_par_email_normalise_jamais_automatiques_et_admin_plateforme_signale() {
        let src = SourceRapprochement {
            comptes: vec![
                SourceCompte {
                    source_id: "u1".into(),
                    email: Some("Claire@College.test".into()),
                    nom: Some("Bernard".into()),
                    prenom: Some("Claire".into()),
                    administrateur_plateforme: false,
                },
                SourceCompte {
                    source_id: "u2".into(),
                    email: Some("root@scolia.test".into()),
                    nom: None,
                    prenom: None,
                    administrateur_plateforme: true,
                },
                SourceCompte {
                    source_id: "u3".into(),
                    email: Some("inconnu@college.test".into()),
                    nom: None,
                    prenom: None,
                    administrateur_plateforme: false,
                },
            ],
            ..Default::default()
        };
        let cible = CibleRapprochement {
            comptes: vec![
                CibleCompte {
                    id: "k1".into(),
                    email: "claire@college.test".into(),
                },
                CibleCompte {
                    id: "k2".into(),
                    email: "root@scolia.test".into(),
                },
            ],
            ..Default::default()
        };
        let existantes = Existantes::new();
        let ctx = Contexte::new(NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(), &existantes);
        let mut v = Verdicts::new();
        rapprocher_comptes(&src, &cible, &ctx, &mut v);
        assert_eq!(cands(&v, "user", "u1")[0].confiance, Confiance::Forte);
        let c2 = cands(&v, "user", "u2");
        assert!(c2[0].raisons.contains(&Raison::AdministrateurPlateforme));
        assert_eq!(c2[0].confiance, Confiance::Faible);
        assert!(cands(&v, "user", "u3").is_empty());
    }
}
