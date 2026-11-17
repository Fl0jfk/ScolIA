//! Rapprochement des **référentiels** (lot R3, T0 §6.1 à §6.3) : années scolaires, sites, classes.
//!
//! Règles automatiques (clés fortes, sans ressemblance) :
//! * année : mêmes dates de début **et** de fin, uniques des deux côtés ;
//! * classe : code de division SIECLE identique au code ScolIA, unique des deux côtés ;
//! * site, classe : équivalence de libellé déjà **validée** par l'établissement (table de synonymes relue
//!   une fois, T0 §6.3), rejouée.
//!
//! Tout le reste est proposé : libellé identique après normalisation (« Seconde B » = « 2nde B »), dates proches.
//! Une année passée absente de ScolIA n'est **pas** créée : ses données vont en archive (T0 §6.2).

use super::donnees::{
    CibleAnnee, CibleRapprochement, SourceAnnee, SourceClasse, SourceRapprochement, SourceSite,
};
use super::normalisation::{cle_classe, cle_nom};
use super::types::{classer, motif_de, Candidat, Confiance, Raison, Resume, Verdict};
use super::{Contexte, Verdicts};
use std::collections::BTreeMap;

pub const OBJET_ANNEE: &str = "annee_scolaire";
pub const OBJET_SITE: &str = "etablissement_site";
pub const OBJET_CLASSE: &str = "classe_site_mapping";
pub const REGLE_DATES: &str = "dates_identiques";
pub const REGLE_SIECLE: &str = "code_siecle_unique";
pub const REGLE_SYNONYME: &str = "synonyme_valide";
const MAX_CANDIDATS: usize = 5;
/// Écart de dates accepté « à quelques jours » (T0 §6.2), signalé et jamais appliqué seul.
const ECART_JOURS: i64 = 7;

fn resume_annee_source(a: &SourceAnnee) -> Resume {
    Resume {
        detail: Some(format!("{} → {}", a.debut, a.fin)),
        nom: Some(a.libelle.clone()).filter(|l| !l.is_empty()),
        ..Default::default()
    }
}

fn resume_annee_cible(a: &CibleAnnee) -> Resume {
    Resume {
        detail: Some(format!("{} → {}", a.debut, a.fin)),
        nom: Some(a.libelle.clone()),
        ..Default::default()
    }
}

pub fn rapprocher_annees(
    src: &[SourceAnnee],
    cible: &CibleRapprochement,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    let mut memes_dates_src: BTreeMap<(chrono::NaiveDate, chrono::NaiveDate), usize> =
        BTreeMap::new();
    for a in src {
        *memes_dates_src.entry((a.debut, a.fin)).or_default() += 1;
    }
    for a in src {
        if ctx.deja_connue(OBJET_ANNEE, &a.source_id) {
            continue;
        }
        let exactes: Vec<&CibleAnnee> = cible
            .annees
            .iter()
            .filter(|c| c.debut == a.debut && c.fin == a.fin)
            .collect();
        let cle = (OBJET_ANNEE.to_string(), a.source_id.clone());
        if let [c] = exactes.as_slice() {
            if memes_dates_src.get(&(a.debut, a.fin)) == Some(&1)
                && !ctx.cible_reliee(OBJET_ANNEE, &c.id)
            {
                verdicts.insert(
                    cle,
                    Verdict::Automatique {
                        cible_id: c.id.clone(),
                        regle: REGLE_DATES,
                        raisons: vec![Raison::DatesIdentiques],
                    },
                );
                continue;
            }
        }
        let mut candidats = Vec::new();
        for c in &cible.annees {
            let identiques = c.debut == a.debut && c.fin == a.fin;
            let proches = (c.debut - a.debut).num_days().abs() <= ECART_JOURS
                && (c.fin - a.fin).num_days().abs() <= ECART_JOURS;
            let libelle = !a.libelle.is_empty() && cle_nom(&a.libelle) == cle_nom(&c.libelle);
            let (conf, mut raisons) = if identiques {
                (Confiance::Forte, vec![Raison::DatesIdentiques])
            } else if proches {
                (Confiance::Forte, vec![Raison::DatesProches])
            } else if libelle {
                (Confiance::Moyenne, vec![])
            } else {
                continue;
            };
            if libelle {
                raisons.push(Raison::LibelleIdentique);
            }
            if ctx.cible_reliee(OBJET_ANNEE, &c.id) {
                raisons.push(Raison::CibleDejaReliee);
            }
            if exactes.len() > 1 && identiques {
                raisons.push(Raison::AmbiguiteDansLeCoeur);
            }
            candidats.push(Candidat {
                cible_id: c.id.clone(),
                confiance: conf,
                raisons,
                resume: resume_annee_cible(c),
            });
        }
        let cs = classer(candidats, MAX_CANDIDATS);
        if cs.is_empty() && !a.courante && a.fin < ctx.jour {
            verdicts.insert(
                cle,
                Verdict::Archive {
                    raison: "annee_passee_absente",
                },
            );
        } else {
            verdicts.insert(
                cle,
                Verdict::ADecider {
                    motif: motif_de(&cs),
                    candidats: cs,
                    resume_source: resume_annee_source(a),
                },
            );
        }
    }
}

fn synonyme<'a>(cible: &'a CibleRapprochement, domaine: &str, cle: &str) -> Option<&'a String> {
    cible.synonymes.get(&(domaine.to_string(), cle.to_string()))
}

pub fn rapprocher_sites(
    src: &[SourceSite],
    cible: &CibleRapprochement,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    for s in src {
        if ctx.deja_connue(OBJET_SITE, &s.source_id) {
            continue;
        }
        let cle = (OBJET_SITE.to_string(), s.source_id.clone());
        let k_lib = cle_nom(&s.libelle);
        if let Some(id) =
            synonyme(cible, "site", &k_lib).filter(|id| cible.sites.iter().any(|c| &c.id == *id))
        {
            verdicts.insert(
                cle,
                Verdict::Automatique {
                    cible_id: id.clone(),
                    regle: REGLE_SYNONYME,
                    raisons: vec![Raison::SynonymeValide],
                },
            );
            continue;
        }
        let k_id = cle_nom(&s.source_id);
        let mut candidats = Vec::new();
        for c in &cible.sites {
            let par_code = !k_id.is_empty() && k_id == cle_nom(&c.code);
            let par_libelle = !k_lib.is_empty() && k_lib == cle_nom(&c.libelle);
            if !(par_code || par_libelle) {
                continue;
            }
            let mut raisons = Vec::new();
            if par_code {
                raisons.push(Raison::CodeIdentique);
            }
            if par_libelle {
                raisons.push(Raison::LibelleIdentique);
            }
            if ctx.cible_reliee(OBJET_SITE, &c.id) {
                raisons.push(Raison::CibleDejaReliee);
            }
            candidats.push(Candidat {
                cible_id: c.id.clone(),
                confiance: Confiance::Forte,
                raisons,
                resume: Resume {
                    nom: Some(c.libelle.clone()),
                    detail: Some(c.code.clone()),
                    ..Default::default()
                },
            });
        }
        let cs = classer(candidats, MAX_CANDIDATS);
        verdicts.insert(
            cle,
            Verdict::ADecider {
                motif: motif_de(&cs),
                candidats: cs,
                resume_source: Resume {
                    nom: Some(s.libelle.clone()).filter(|l| !l.is_empty()),
                    ..Default::default()
                },
            },
        );
    }
}

pub fn rapprocher_classes(
    src: &[SourceClasse],
    cible: &CibleRapprochement,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    let mut siecle_src: BTreeMap<String, usize> = BTreeMap::new();
    for c in src {
        if let Some(k) = c
            .code_siecle
            .as_deref()
            .map(cle_nom)
            .filter(|k| !k.is_empty())
        {
            *siecle_src.entry(k).or_default() += 1;
        }
    }
    for s in src {
        if ctx.deja_connue(OBJET_CLASSE, &s.source_id) {
            continue;
        }
        let cle = (OBJET_CLASSE.to_string(), s.source_id.clone());
        let k_classe = cle_classe(&s.nom);
        if let Some(id) = synonyme(cible, "classe", &k_classe)
            .filter(|id| cible.classes.iter().any(|c| &c.id == *id))
        {
            verdicts.insert(
                cle,
                Verdict::Automatique {
                    cible_id: id.clone(),
                    regle: REGLE_SYNONYME,
                    raisons: vec![Raison::SynonymeValide],
                },
            );
            continue;
        }
        let k_siecle = s
            .code_siecle
            .as_deref()
            .map(cle_nom)
            .filter(|k| !k.is_empty());
        if let Some(k) = &k_siecle {
            let memes: Vec<_> = cible
                .classes
                .iter()
                .filter(|c| cle_nom(&c.code) == *k)
                .collect();
            if let [c] = memes.as_slice() {
                if siecle_src.get(k) == Some(&1) && !ctx.cible_reliee(OBJET_CLASSE, &c.id) {
                    verdicts.insert(
                        cle,
                        Verdict::Automatique {
                            cible_id: c.id.clone(),
                            regle: REGLE_SIECLE,
                            raisons: vec![Raison::CodeSiecleIdentique],
                        },
                    );
                    continue;
                }
            }
        }
        let mut candidats = Vec::new();
        for c in &cible.classes {
            let par_siecle = k_siecle.as_ref().is_some_and(|k| *k == cle_nom(&c.code));
            let par_libelle = !k_classe.is_empty()
                && (k_classe == cle_classe(&c.libelle) || k_classe == cle_classe(&c.code));
            if !(par_siecle || par_libelle) {
                continue;
            }
            let mut raisons = Vec::new();
            if par_siecle {
                raisons.push(Raison::CodeSiecleIdentique);
            }
            if par_libelle {
                raisons.push(Raison::LibelleIdentique);
            }
            if ctx.cible_reliee(OBJET_CLASSE, &c.id) {
                raisons.push(Raison::CibleDejaReliee);
            }
            if siecle_src
                .get(k_siecle.as_deref().unwrap_or_default())
                .copied()
                .unwrap_or(0)
                > 1
                && par_siecle
            {
                raisons.push(Raison::AmbiguiteDansLeCoeur);
            }
            candidats.push(Candidat {
                cible_id: c.id.clone(),
                confiance: Confiance::Forte,
                raisons,
                resume: Resume {
                    nom: Some(c.libelle.clone()),
                    detail: Some(c.code.clone()),
                    ..Default::default()
                },
            });
        }
        let cs = classer(candidats, MAX_CANDIDATS);
        verdicts.insert(
            cle,
            Verdict::ADecider {
                motif: motif_de(&cs),
                candidats: cs,
                resume_source: Resume {
                    nom: Some(s.nom.clone()).filter(|l| !l.is_empty()),
                    detail: s.code_siecle.clone(),
                    ..Default::default()
                },
            },
        );
    }
}

/// Accès simple aux sources de ce fichier depuis le module parent.
pub fn tout(
    src: &SourceRapprochement,
    cible: &CibleRapprochement,
    ctx: &Contexte,
    v: &mut Verdicts,
) {
    rapprocher_annees(&src.annees, cible, ctx, v);
    rapprocher_sites(&src.sites, cible, ctx, v);
    rapprocher_classes(&src.classes, cible, ctx, v);
}

#[cfg(test)]
mod tests {
    use super::super::donnees::{CibleClasse, CibleSite};
    use super::*;
    use crate::correspondance::Existantes;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    fn ctx<'a>(e: &'a Existantes) -> Contexte<'a> {
        Contexte::new(d(2026, 11, 16), e)
    }

    fn annee_s(id: &str, debut: NaiveDate, fin: NaiveDate, courante: bool) -> SourceAnnee {
        SourceAnnee {
            source_id: id.into(),
            libelle: "2025-2026".into(),
            debut,
            fin,
            courante,
        }
    }

    fn annee_c(id: &str, debut: NaiveDate, fin: NaiveDate) -> CibleAnnee {
        CibleAnnee {
            id: id.into(),
            libelle: "2025-2026".into(),
            debut,
            fin,
        }
    }

    #[test]
    fn annee_aux_memes_dates_est_automatique() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let cible = CibleRapprochement {
            annees: vec![annee_c("a1", d(2025, 9, 1), d(2026, 7, 4))],
            ..Default::default()
        };
        rapprocher_annees(
            &[annee_s("s1", d(2025, 9, 1), d(2026, 7, 4), false)],
            &cible,
            &ctx(&e),
            &mut v,
        );
        assert!(matches!(
            &v[&(OBJET_ANNEE.to_string(), "s1".to_string())],
            Verdict::Automatique { regle, .. } if *regle == REGLE_DATES
        ));
    }

    #[test]
    fn annee_a_quelques_jours_est_proposee_et_signalee() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let cible = CibleRapprochement {
            annees: vec![annee_c("a1", d(2025, 9, 2), d(2026, 7, 5))],
            ..Default::default()
        };
        rapprocher_annees(
            &[annee_s("s1", d(2025, 9, 1), d(2026, 7, 4), true)],
            &cible,
            &ctx(&e),
            &mut v,
        );
        let Verdict::ADecider { candidats, .. } = &v[&(OBJET_ANNEE.to_string(), "s1".to_string())]
        else {
            panic!()
        };
        assert!(candidats[0].raisons.contains(&Raison::DatesProches));
    }

    #[test]
    fn annee_passee_absente_va_en_archive_jamais_creee() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        rapprocher_annees(
            &[
                annee_s("s1", d(2023, 9, 1), d(2024, 7, 4), false),
                annee_s("s2", d(2026, 9, 1), d(2027, 7, 4), true),
            ],
            &CibleRapprochement::default(),
            &ctx(&e),
            &mut v,
        );
        assert!(matches!(
            &v[&(OBJET_ANNEE.to_string(), "s1".to_string())],
            Verdict::Archive { .. }
        ));
        // L'année courante absente reste à décider : on ne l'archive pas en silence.
        assert!(matches!(
            &v[&(OBJET_ANNEE.to_string(), "s2".to_string())],
            Verdict::ADecider { .. }
        ));
    }

    #[test]
    fn deux_annees_de_l_export_aux_memes_dates_ne_sont_pas_automatiques() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let cible = CibleRapprochement {
            annees: vec![annee_c("a1", d(2025, 9, 1), d(2026, 7, 4))],
            ..Default::default()
        };
        rapprocher_annees(
            &[
                annee_s("s1", d(2025, 9, 1), d(2026, 7, 4), false),
                annee_s("s2", d(2025, 9, 1), d(2026, 7, 4), false),
            ],
            &cible,
            &ctx(&e),
            &mut v,
        );
        assert!(v.values().all(|x| matches!(x, Verdict::ADecider { .. })));
    }

    fn classe_c(id: &str, code: &str, libelle: &str) -> CibleClasse {
        CibleClasse {
            id: id.into(),
            code: code.into(),
            libelle: libelle.into(),
        }
    }

    fn classe_s(id: &str, nom: &str, siecle: Option<&str>) -> SourceClasse {
        SourceClasse {
            source_id: id.into(),
            nom: nom.into(),
            code_siecle: siecle.map(str::to_string),
        }
    }

    #[test]
    fn classe_par_code_siecle_unique_est_automatique() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let cible = CibleRapprochement {
            classes: vec![classe_c("k1", "2NDE B", "Seconde B")],
            ..Default::default()
        };
        rapprocher_classes(
            &[classe_s("s1", "2nde B", Some("2nde b"))],
            &cible,
            &ctx(&e),
            &mut v,
        );
        assert!(matches!(
            &v[&(OBJET_CLASSE.to_string(), "s1".to_string())],
            Verdict::Automatique { regle, .. } if *regle == REGLE_SIECLE
        ));
    }

    #[test]
    fn classe_par_libelle_equivalent_est_proposee_pas_appliquee() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let cible = CibleRapprochement {
            classes: vec![classe_c("k1", "SEC-B", "Seconde B")],
            ..Default::default()
        };
        rapprocher_classes(
            &[classe_s("s1", "2nde B", None), classe_s("s2", "2B", None)],
            &cible,
            &ctx(&e),
            &mut v,
        );
        let Verdict::ADecider { candidats, .. } = &v[&(OBJET_CLASSE.to_string(), "s1".to_string())]
        else {
            panic!()
        };
        assert_eq!(candidats[0].cible_id, "k1");
        assert!(candidats[0].raisons.contains(&Raison::LibelleIdentique));
        // « 2B » n'est jamais déduit : aucun candidat.
        let Verdict::ADecider { candidats, .. } = &v[&(OBJET_CLASSE.to_string(), "s2".to_string())]
        else {
            panic!()
        };
        assert!(candidats.is_empty());
    }

    #[test]
    fn synonyme_valide_est_rejoue_automatiquement() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let mut cible = CibleRapprochement {
            classes: vec![classe_c("k1", "SEC-B", "Seconde B")],
            ..Default::default()
        };
        cible
            .synonymes
            .insert(("classe".into(), cle_classe("2B")), "k1".into());
        rapprocher_classes(&[classe_s("s2", "2B", None)], &cible, &ctx(&e), &mut v);
        assert!(matches!(
            &v[&(OBJET_CLASSE.to_string(), "s2".to_string())],
            Verdict::Automatique { regle, raisons, .. } if *regle == REGLE_SYNONYME && raisons == &[Raison::SynonymeValide]
        ));
        // Un synonyme dont la cible a disparu n'est pas appliqué.
        let mut v = Verdicts::new();
        cible.classes.clear();
        rapprocher_classes(&[classe_s("s2", "2B", None)], &cible, &ctx(&e), &mut v);
        assert!(matches!(
            &v[&(OBJET_CLASSE.to_string(), "s2".to_string())],
            Verdict::ADecider { .. }
        ));
    }

    #[test]
    fn site_par_libelle_ou_code_propose_et_synonyme_rejoue() {
        let e = Existantes::new();
        let mut v = Verdicts::new();
        let mut cible = CibleRapprochement {
            sites: vec![CibleSite {
                id: "t1".into(),
                code: "COLLEGE".into(),
                libelle: "Collège Les Brumes".into(),
            }],
            ..Default::default()
        };
        let s = [
            SourceSite {
                source_id: "college".into(),
                libelle: "Autre libellé".into(),
            },
            SourceSite {
                source_id: "x".into(),
                libelle: "Collège les brumes".into(),
            },
            SourceSite {
                source_id: "y".into(),
                libelle: "Lycée".into(),
            },
        ];
        rapprocher_sites(&s, &cible, &ctx(&e), &mut v);
        let g = |id: &str| match &v[&(OBJET_SITE.to_string(), id.to_string())] {
            Verdict::ADecider { candidats, .. } => candidats.len(),
            _ => usize::MAX,
        };
        assert_eq!((g("college"), g("x"), g("y")), (1, 1, 0));
        cible
            .synonymes
            .insert(("site".into(), cle_nom("Lycée")), "t1".into());
        let mut v = Verdicts::new();
        rapprocher_sites(&s, &cible, &ctx(&e), &mut v);
        assert!(matches!(
            &v[&(OBJET_SITE.to_string(), "y".to_string())],
            Verdict::Automatique { .. }
        ));
    }
}
