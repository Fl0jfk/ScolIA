//! Rapprochement des **élèves** (lot R3, T0 §6.4).
//!
//! Ordre des règles :
//! 1. une correspondance déjà enregistrée (décision humaine ou reprise précédente) prime : elle n'est pas
//!    recalculée ici (c'est la simulation qui la rejoue) ;
//! 2. **INE** identique, unique dans l'export et unique dans ScolIA, sans contradiction de date de naissance
//!    ni d'identité → **seule règle automatique** ;
//! 3. nom + prénom + date de naissance identiques → **proposée** (confiance forte) : une personne habilitée
//!    confirme, à l'unité ou en lot ;
//! 4. variantes (nom de naissance, prénom secondaire, nom composé, une lettre d'écart) → proposées, confiance
//!    moyenne ou faible ;
//! 5. rien d'approximatif n'est jamais appliqué seul.
//!
//! Cas toujours présentés à un humain : INE identique avec date de naissance différente, INE en double
//! (deux fiches de l'export pour un seul élève de ScolIA, ou l'inverse), jumeaux possibles.

use super::donnees::{CibleEleve, CibleRapprochement, SourceEleve};
use super::normalisation::{cle_classe, cle_ine, cle_nom, distance_bornee, ine_fin, mots};
use super::types::{classer, motif_de, Candidat, Confiance, Raison, Resume, Verdict};
use super::{Contexte, Verdicts};
use std::collections::{BTreeMap, BTreeSet};

pub const OBJET: &str = "eleve";
pub const REGLE_INE: &str = "ine_unique";
const MAX_CANDIDATS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nom {
    Exact,
    Naissance,
    Partiel,
    Proche,
    Non,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prenom {
    Exact,
    Secondaire,
    Proche,
    Non,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Naissance {
    Egale,
    Differente,
    Inconnue,
}

fn compare_nom(s: &str, c: &CibleEleve) -> Nom {
    let s = cle_nom(s);
    if s.is_empty() {
        return Nom::Non;
    }
    let cn = cle_nom(&c.nom);
    if s == cn {
        return Nom::Exact;
    }
    let naissance = c
        .nom_naissance
        .as_deref()
        .map(cle_nom)
        .filter(|n| !n.is_empty());
    if naissance.as_deref() == Some(s.as_str()) {
        return Nom::Naissance;
    }
    let ms = mots(&s);
    let inclus = |autre: &str| {
        let ma = mots(autre);
        !ma.is_empty() && (ms.iter().all(|m| ma.contains(m)) || ma.iter().all(|m| ms.contains(m)))
    };
    if inclus(&cn) || naissance.as_deref().is_some_and(inclus) {
        return Nom::Partiel;
    }
    if s.chars().count() >= 5 && distance_bornee(&s, &cn, 1).is_some() {
        return Nom::Proche;
    }
    Nom::Non
}

fn compare_prenom(s: &str, c: &CibleEleve) -> Prenom {
    let s = cle_nom(s);
    if s.is_empty() {
        return Prenom::Non;
    }
    let cp = cle_nom(&c.prenom);
    if s == cp {
        return Prenom::Exact;
    }
    let autres: Vec<String> = c.prenoms.iter().map(|p| cle_nom(p)).collect();
    let mut tous: BTreeSet<&str> = mots(&cp).into_iter().collect();
    for a in &autres {
        tous.extend(mots(a));
    }
    let ms = mots(&s);
    if autres.contains(&s)
        || (!ms.is_empty() && ms.iter().all(|m| tous.contains(m)))
        || mots(&cp).iter().all(|m| ms.contains(m))
    {
        return Prenom::Secondaire;
    }
    if s.chars().count() >= 5 && distance_bornee(&s, &cp, 1).is_some() {
        return Prenom::Proche;
    }
    Prenom::Non
}

fn compare_naissance(s: &SourceEleve, c: &CibleEleve) -> Naissance {
    match (s.date_naissance, c.date_naissance) {
        (Some(a), Some(b)) if a == b => Naissance::Egale,
        (Some(_), Some(_)) => Naissance::Differente,
        _ => Naissance::Inconnue,
    }
}

/// Deux libellés de classe désignent-ils la même classe ? Synonymes validés d'abord, puis libellés
/// normalisés (« Seconde B » = « 2nde B »).
fn meme_classe(s: Option<&str>, c: Option<&str>, cible: &CibleRapprochement) -> bool {
    let (Some(s), Some(c)) = (s, c) else {
        return false;
    };
    let (ks, kc) = (cle_classe(s), cle_classe(c));
    if ks.is_empty() || kc.is_empty() {
        return false;
    }
    if ks == kc {
        return true;
    }
    cible
        .synonymes
        .get(&("classe".to_string(), ks))
        .is_some_and(|id| {
            cible
                .classes
                .iter()
                .any(|k| &k.id == id && (cle_classe(&k.libelle) == kc || cle_classe(&k.code) == kc))
        })
}

pub fn resume_source(s: &SourceEleve) -> Resume {
    Resume {
        nom: Some(s.nom.clone()).filter(|n| !n.is_empty()),
        prenom: Some(s.prenom.clone()).filter(|n| !n.is_empty()),
        naissance: s.date_naissance.map(|d| d.to_string()),
        classe: s.classe.clone(),
        ine_fin: s.ine.as_deref().and_then(ine_fin),
        contact_masque: None,
        detail: None,
    }
}

pub fn resume_cible(c: &CibleEleve) -> Resume {
    Resume {
        nom: Some(c.nom.clone()),
        prenom: Some(c.prenom.clone()),
        naissance: c.date_naissance.map(|d| d.to_string()),
        classe: c.classe.clone(),
        ine_fin: c.ine.as_deref().and_then(ine_fin),
        contact_masque: None,
        detail: (!c.actif).then(|| "élève sorti".to_string()),
    }
}

/// Élève sorti de l'établissement d'après l'export. Un statut explicite l'emporte sur la date : un élève
/// « actif » à date de sortie passée est une **incohérence** de l'export, qu'une personne regarde (il n'est
/// jamais archivé en silence).
fn est_sorti(s: &SourceEleve, ctx: &Contexte) -> bool {
    let statut = s.statut.as_deref().map(cle_nom);
    let sorti = matches!(
        statut.as_deref(),
        Some("sorti" | "sortie" | "inactif" | "inactive" | "archive" | "archived" | "left")
    );
    let actif = matches!(
        statut.as_deref(),
        Some("actif" | "active" | "inscrit" | "en cours")
    );
    sorti || (!actif && s.date_sortie.is_some_and(|d| d <= ctx.jour))
}

/// Candidat « composé » (sans INE) pour une paire, ou `None` si la paire ne mérite pas d'être proposée.
fn candidat_compose(
    s: &SourceEleve,
    c: &CibleEleve,
    cible: &CibleRapprochement,
) -> Option<Candidat> {
    let nom = compare_nom(&s.nom, c);
    let prenom = compare_prenom(&s.prenom, c);
    let naiss = compare_naissance(s, c);
    let classe = meme_classe(s.classe.as_deref(), c.classe.as_deref(), cible);
    let mut raisons = Vec::new();
    let nom_fort = matches!(nom, Nom::Exact);
    let confiance = match (nom, prenom, naiss) {
        (Nom::Exact, Prenom::Exact, Naissance::Egale) => Confiance::Forte,
        (
            Nom::Exact | Nom::Naissance | Nom::Partiel,
            Prenom::Exact | Prenom::Secondaire,
            Naissance::Egale,
        ) => Confiance::Moyenne,
        (Nom::Exact, Prenom::Exact, Naissance::Inconnue) if classe => Confiance::Moyenne,
        (Nom::Exact, Prenom::Exact, Naissance::Inconnue | Naissance::Differente) => {
            Confiance::Faible
        }
        (Nom::Exact | Nom::Proche, Prenom::Exact | Prenom::Proche, Naissance::Egale) => {
            raisons.push(Raison::NomPrenomProches);
            Confiance::Faible
        }
        _ => return None,
    };
    if nom_fort {
        raisons.push(Raison::NomIdentique);
    }
    if nom == Nom::Naissance {
        raisons.push(Raison::NomNaissanceIdentique);
    }
    if nom == Nom::Partiel {
        raisons.push(Raison::NomPartiel);
    }
    match prenom {
        Prenom::Exact => raisons.push(Raison::PrenomIdentique),
        Prenom::Secondaire => raisons.push(Raison::PrenomSecondaire),
        _ => {}
    }
    match naiss {
        Naissance::Egale => raisons.push(Raison::NaissanceIdentique),
        Naissance::Differente => raisons.push(Raison::NaissanceDifferente),
        Naissance::Inconnue => raisons.push(Raison::NaissanceInconnue),
    }
    if classe {
        raisons.push(Raison::MemeClasse);
    }
    // Deux INE valides et différents : un des deux est peut-être mal saisi, on le dit.
    if let (Some(a), Some(b)) = (
        s.ine.as_deref().and_then(cle_ine),
        c.ine.as_deref().and_then(cle_ine),
    ) {
        if a != b {
            raisons.push(Raison::IneDifferent);
        }
    }
    Some(Candidat {
        cible_id: c.id.clone(),
        confiance,
        raisons,
        resume: resume_cible(c),
    })
}

enum Brut {
    Auto(String, Vec<Raison>),
    Candidats(Vec<Candidat>),
}

pub fn rapprocher(
    src: &[SourceEleve],
    cible: &CibleRapprochement,
    ctx: &Contexte,
    verdicts: &mut Verdicts,
) {
    // INE : combien de fois chaque INE valide apparaît des deux côtés.
    let mut ine_src: BTreeMap<String, usize> = BTreeMap::new();
    for s in src {
        if let Some(i) = s.ine.as_deref().and_then(cle_ine) {
            *ine_src.entry(i).or_default() += 1;
        }
    }
    let mut ine_cible: BTreeMap<String, Vec<&CibleEleve>> = BTreeMap::new();
    for c in &cible.eleves {
        if let Some(i) = c.ine.as_deref().and_then(cle_ine) {
            ine_cible.entry(i).or_default().push(c);
        }
    }
    // Jumeaux possibles dans l'export : même nom et même naissance, prénoms différents.
    let mut fratries: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for s in src {
        if let Some(n) = s.date_naissance {
            fratries
                .entry((cle_nom(&s.nom), n.to_string()))
                .or_default()
                .insert(cle_nom(&s.prenom));
        }
    }
    let mut prises: BTreeSet<String> = ctx.cibles_reliees(OBJET);
    let mut bruts: Vec<(&SourceEleve, Brut)> = Vec::new();
    for s in src {
        if ctx.deja_connue(OBJET, &s.source_id) {
            continue;
        }
        let ine = s.ine.as_deref().and_then(cle_ine);
        let mut candidats: Vec<Candidat> = Vec::new();
        let mut ine_pris: BTreeSet<&str> = BTreeSet::new();
        if let Some(ine) = &ine {
            let memes = ine_cible.get(ine).map(Vec::as_slice).unwrap_or_default();
            let unique = memes.len() == 1 && ine_src.get(ine) == Some(&1);
            for c in memes {
                let nom = compare_nom(&s.nom, c);
                let prenom = compare_prenom(&s.prenom, c);
                let naiss = compare_naissance(s, c);
                let mut raisons = vec![Raison::IneIdentique];
                let mut contre = false;
                if naiss == Naissance::Differente {
                    raisons.push(Raison::NaissanceDifferente);
                    contre = true;
                }
                if nom == Nom::Non && matches!(prenom, Prenom::Non | Prenom::Proche) {
                    raisons.push(Raison::IdentiteDiscordante);
                    contre = true;
                }
                if memes.len() > 1 {
                    raisons.push(Raison::IneEnDoubleCoeur);
                    contre = true;
                }
                if ine_src.get(ine).copied().unwrap_or(0) > 1 {
                    raisons.push(Raison::IneEnDoubleNext);
                    contre = true;
                }
                if naiss == Naissance::Egale {
                    raisons.push(Raison::NaissanceIdentique);
                }
                if naiss == Naissance::Inconnue {
                    raisons.push(Raison::NaissanceInconnue);
                }
                if matches!(nom, Nom::Exact | Nom::Naissance) {
                    raisons.push(Raison::NomIdentique);
                }
                if prenom == Prenom::Exact {
                    raisons.push(Raison::PrenomIdentique);
                }
                if unique && !contre {
                    raisons.push(Raison::IneUniqueDeuxCotes);
                    candidats.clear();
                    ine_pris.insert(&c.id);
                    candidats.push(Candidat {
                        cible_id: c.id.clone(),
                        confiance: Confiance::Certaine,
                        raisons,
                        resume: resume_cible(c),
                    });
                    break;
                }
                ine_pris.insert(&c.id);
                candidats.push(Candidat {
                    cible_id: c.id.clone(),
                    confiance: if contre {
                        Confiance::Moyenne
                    } else {
                        Confiance::Forte
                    },
                    raisons,
                    resume: resume_cible(c),
                });
            }
        }
        if let [c] = candidats.as_slice() {
            if c.confiance == Confiance::Certaine {
                let raisons = c.raisons.clone();
                prises.insert(c.cible_id.clone());
                bruts.push((s, Brut::Auto(c.cible_id.clone(), raisons)));
                continue;
            }
        }
        for c in &cible.eleves {
            if ine_pris.contains(c.id.as_str()) {
                continue;
            }
            if let Some(mut cand) = candidat_compose(s, c, cible) {
                let jumeaux = fratries
                    .get(&(
                        cle_nom(&s.nom),
                        s.date_naissance.map(|d| d.to_string()).unwrap_or_default(),
                    ))
                    .is_some_and(|p| p.len() > 1);
                if jumeaux {
                    cand.raisons.push(Raison::JumeauxPossibles);
                }
                candidats.push(cand);
            }
        }
        bruts.push((s, Brut::Candidats(candidats)));
    }
    // Deuxième passe : conflits entre fiches (fiche ScolIA déjà prise, proposée plusieurs fois, ambiguïté).
    let mut demandes: BTreeMap<String, usize> = BTreeMap::new();
    for (_, b) in &bruts {
        if let Brut::Candidats(cs) = b {
            for c in cs {
                *demandes.entry(c.cible_id.clone()).or_default() += 1;
            }
        }
    }
    for (s, b) in bruts {
        let cle = (OBJET.to_string(), s.source_id.clone());
        match b {
            Brut::Auto(cible_id, raisons) => {
                verdicts.insert(
                    cle,
                    Verdict::Automatique {
                        cible_id,
                        regle: REGLE_INE,
                        raisons,
                    },
                );
            }
            Brut::Candidats(mut cs) => {
                let fortes = cs
                    .iter()
                    .filter(|c| c.confiance == Confiance::Forte)
                    .count();
                for c in &mut cs {
                    let deja_prise = prises.contains(&c.cible_id);
                    if deja_prise {
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
                if cs.is_empty() && est_sorti(s, ctx) {
                    verdicts.insert(
                        cle,
                        Verdict::Archive {
                            raison: "sorti_absent_du_coeur",
                        },
                    );
                } else {
                    verdicts.insert(
                        cle,
                        Verdict::ADecider {
                            motif: motif_de(&cs),
                            candidats: cs,
                            resume_source: resume_source(s),
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondance::Existantes;
    use chrono::NaiveDate;

    pub(crate) fn jour(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn src(
        id: &str,
        nom: &str,
        prenom: &str,
        ine: Option<&str>,
        n: Option<NaiveDate>,
    ) -> SourceEleve {
        SourceEleve {
            source_id: id.into(),
            nom: nom.into(),
            prenom: prenom.into(),
            ine: ine.map(str::to_string),
            date_naissance: n,
            classe: None,
            date_sortie: None,
            statut: None,
        }
    }

    fn cib(
        id: &str,
        nom: &str,
        prenom: &str,
        ine: Option<&str>,
        n: Option<NaiveDate>,
    ) -> CibleEleve {
        CibleEleve {
            id: id.into(),
            nom: nom.into(),
            nom_naissance: None,
            prenom: prenom.into(),
            prenoms: vec![],
            ine: ine.map(str::to_string),
            date_naissance: n,
            classe: None,
            actif: true,
        }
    }

    fn lancer(s: Vec<SourceEleve>, c: Vec<CibleEleve>) -> Verdicts {
        let existantes = Existantes::new();
        let ctx = Contexte::new(jour(2026, 11, 16), &existantes);
        let cible = CibleRapprochement {
            eleves: c,
            ..Default::default()
        };
        let mut v = Verdicts::new();
        rapprocher(&s, &cible, &ctx, &mut v);
        v
    }

    fn v<'a>(r: &'a Verdicts, id: &str) -> &'a Verdict {
        &r[&(OBJET.to_string(), id.to_string())]
    }

    #[test]
    fn ine_unique_des_deux_cotes_est_automatique_et_porte_ses_raisons() {
        let r = lancer(
            vec![src(
                "n1",
                "Martin",
                "Léa",
                Some("1234567890a"),
                Some(jour(2012, 3, 4)),
            )],
            vec![cib(
                "c1",
                "MARTIN",
                "Lea",
                Some("1234567890A"),
                Some(jour(2012, 3, 4)),
            )],
        );
        match v(&r, "n1") {
            Verdict::Automatique {
                cible_id,
                regle,
                raisons,
            } => {
                assert_eq!(cible_id, "c1");
                assert_eq!(*regle, REGLE_INE);
                assert!(raisons.contains(&Raison::IneIdentique));
                assert!(raisons.contains(&Raison::IneUniqueDeuxCotes));
            }
            autre => panic!("attendu automatique : {autre:?}"),
        }
    }

    #[test]
    fn ine_identique_mais_naissance_differente_va_a_decider() {
        let r = lancer(
            vec![src(
                "n1",
                "Martin",
                "Léa",
                Some("1234567890A"),
                Some(jour(2012, 3, 4)),
            )],
            vec![cib(
                "c1",
                "Martin",
                "Léa",
                Some("1234567890A"),
                Some(jour(2011, 3, 4)),
            )],
        );
        match v(&r, "n1") {
            Verdict::ADecider {
                candidats, motif, ..
            } => {
                assert_eq!(candidats.len(), 1);
                assert!(candidats[0].raisons.contains(&Raison::NaissanceDifferente));
                assert!(candidats[0].a_contre());
                assert_ne!(candidats[0].confiance, Confiance::Certaine);
                assert_eq!(*motif, super::super::types::Motif::Contradiction);
            }
            autre => panic!("attendu à décider : {autre:?}"),
        }
    }

    #[test]
    fn ine_identique_mais_identite_discordante_va_a_decider() {
        let r = lancer(
            vec![src("n1", "Martin", "Léa", Some("1234567890A"), None)],
            vec![cib("c1", "Dupont", "Hugo", Some("1234567890A"), None)],
        );
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!()
        };
        assert!(candidats[0].raisons.contains(&Raison::IdentiteDiscordante));
    }

    #[test]
    fn ine_en_double_dans_l_export_jamais_automatique() {
        let r = lancer(
            vec![
                src(
                    "n1",
                    "Martin",
                    "Léa",
                    Some("1234567890A"),
                    Some(jour(2012, 3, 4)),
                ),
                src(
                    "n2",
                    "Martin",
                    "Lea",
                    Some("1234567890A"),
                    Some(jour(2012, 3, 4)),
                ),
            ],
            vec![cib(
                "c1",
                "Martin",
                "Léa",
                Some("1234567890A"),
                Some(jour(2012, 3, 4)),
            )],
        );
        for id in ["n1", "n2"] {
            let Verdict::ADecider { candidats, .. } = v(&r, id) else {
                panic!("{id}")
            };
            assert!(candidats[0].raisons.contains(&Raison::IneEnDoubleNext));
        }
    }

    #[test]
    fn ine_en_double_dans_le_coeur_jamais_automatique() {
        let r = lancer(
            vec![src("n1", "Martin", "Léa", Some("1234567890A"), None)],
            vec![
                cib("c1", "Martin", "Léa", Some("1234567890A"), None),
                cib("c2", "Martin", "Léa", Some("1234567890A"), None),
            ],
        );
        let Verdict::ADecider {
            candidats, motif, ..
        } = v(&r, "n1")
        else {
            panic!()
        };
        assert_eq!(candidats.len(), 2);
        assert!(candidats
            .iter()
            .all(|c| c.raisons.contains(&Raison::IneEnDoubleCoeur)));
        assert_ne!(*motif, super::super::types::Motif::AucunCandidat);
    }

    #[test]
    fn ine_illisible_ne_rapproche_rien() {
        let r = lancer(
            vec![src("n1", "Martin", "Léa", Some("???"), None)],
            vec![cib("c1", "Dupont", "Hugo", Some("???"), None)],
        );
        let Verdict::ADecider {
            candidats, motif, ..
        } = v(&r, "n1")
        else {
            panic!()
        };
        assert!(candidats.is_empty());
        assert_eq!(*motif, super::super::types::Motif::AucunCandidat);
    }

    #[test]
    fn nom_prenom_naissance_exacts_sont_proposes_jamais_appliques() {
        let r = lancer(
            vec![src(
                "n1",
                "Éloïse-Marie",
                "Camille",
                None,
                Some(jour(2011, 5, 6)),
            )],
            vec![cib(
                "c1",
                "Eloise Marie",
                "camille",
                None,
                Some(jour(2011, 5, 6)),
            )],
        );
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!("jamais automatique")
        };
        assert_eq!(candidats[0].confiance, Confiance::Forte);
        assert_eq!(
            candidats[0].raisons,
            vec![
                Raison::NomIdentique,
                Raison::PrenomIdentique,
                Raison::NaissanceIdentique
            ]
        );
    }

    #[test]
    fn faute_de_frappe_est_proposee_en_confiance_faible() {
        let r = lancer(
            vec![src(
                "n1",
                "Martinez",
                "Camille",
                None,
                Some(jour(2011, 5, 6)),
            )],
            vec![cib(
                "c1",
                "Martines",
                "Camille",
                None,
                Some(jour(2011, 5, 6)),
            )],
        );
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!()
        };
        assert_eq!(candidats[0].confiance, Confiance::Faible);
        assert!(candidats[0].raisons.contains(&Raison::NomPrenomProches));
    }

    #[test]
    fn homonymes_de_naissance_differente_restent_faibles_avec_contradiction() {
        let r = lancer(
            vec![src("n1", "Martin", "Léa", None, Some(jour(2012, 3, 4)))],
            vec![cib("c1", "Martin", "Léa", None, Some(jour(2010, 1, 1)))],
        );
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!()
        };
        assert_eq!(candidats[0].confiance, Confiance::Faible);
        assert!(candidats[0].a_contre());
    }

    #[test]
    fn prenom_secondaire_et_nom_de_naissance_donnent_une_confiance_moyenne() {
        let mut c = cib("c1", "Durand-Petit", "Jean", None, Some(jour(2010, 2, 2)));
        c.nom_naissance = Some("Petit".into());
        c.prenoms = vec!["Jean".into(), "Pierre".into()];
        let r = lancer(
            vec![src("n1", "Petit", "Pierre", None, Some(jour(2010, 2, 2)))],
            vec![c],
        );
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!()
        };
        assert_eq!(candidats[0].confiance, Confiance::Moyenne);
        assert!(candidats[0]
            .raisons
            .contains(&Raison::NomNaissanceIdentique));
        assert!(candidats[0].raisons.contains(&Raison::PrenomSecondaire));
    }

    #[test]
    fn jumeaux_ne_sont_pas_confondus_et_sont_signales() {
        let r = lancer(
            vec![
                src("n1", "Roux", "Paul", None, Some(jour(2012, 1, 1))),
                src("n2", "Roux", "Pierre", None, Some(jour(2012, 1, 1))),
            ],
            vec![
                cib("c1", "Roux", "Paul", None, Some(jour(2012, 1, 1))),
                cib("c2", "Roux", "Pierre", None, Some(jour(2012, 1, 1))),
            ],
        );
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!()
        };
        assert_eq!(candidats.len(), 1, "le jumeau n'est pas proposé pour Paul");
        assert_eq!(candidats[0].cible_id, "c1");
        assert!(candidats[0].raisons.contains(&Raison::JumeauxPossibles));
        let Verdict::ADecider { candidats, .. } = v(&r, "n2") else {
            panic!()
        };
        assert_eq!(candidats[0].cible_id, "c2");
    }

    #[test]
    fn deux_fiches_de_l_export_pour_une_fiche_du_coeur_sont_signalees() {
        let r = lancer(
            vec![
                src("n1", "Roux", "Paul", None, Some(jour(2012, 1, 1))),
                src("n2", "Roux", "Paul", None, Some(jour(2012, 1, 1))),
            ],
            vec![cib("c1", "Roux", "Paul", None, Some(jour(2012, 1, 1)))],
        );
        for id in ["n1", "n2"] {
            let Verdict::ADecider { candidats, .. } = v(&r, id) else {
                panic!()
            };
            assert!(candidats[0]
                .raisons
                .contains(&Raison::CiblePlusieursSources));
            assert_eq!(
                candidats[0].confiance,
                Confiance::Moyenne,
                "pas de lot possible"
            );
        }
    }

    #[test]
    fn fiche_deja_prise_par_une_ine_est_signalee_aux_autres_propositions() {
        let r = lancer(
            vec![
                src(
                    "n1",
                    "Roux",
                    "Paul",
                    Some("1234567890A"),
                    Some(jour(2012, 1, 1)),
                ),
                src("n2", "Roux", "Paul", None, Some(jour(2012, 1, 1))),
            ],
            vec![cib(
                "c1",
                "Roux",
                "Paul",
                Some("1234567890A"),
                Some(jour(2012, 1, 1)),
            )],
        );
        assert!(matches!(v(&r, "n1"), Verdict::Automatique { .. }));
        let Verdict::ADecider { candidats, .. } = v(&r, "n2") else {
            panic!()
        };
        assert!(candidats[0].raisons.contains(&Raison::CibleDejaReliee));
    }

    #[test]
    fn eleve_sans_candidat_attend_une_decision_le_coeur_fait_foi() {
        let r = lancer(vec![src("n1", "Nouveau", "Élève", None, None)], vec![]);
        let Verdict::ADecider {
            candidats,
            motif,
            resume_source,
        } = v(&r, "n1")
        else {
            panic!()
        };
        assert!(candidats.is_empty());
        assert_eq!(*motif, super::super::types::Motif::AucunCandidat);
        assert_eq!(resume_source.nom.as_deref(), Some("Nouveau"));
    }

    #[test]
    fn eleve_sorti_absent_du_coeur_est_archive() {
        let mut s = src("n1", "Ancien", "Élève", None, None);
        s.date_sortie = Some(jour(2025, 7, 4));
        let mut s2 = src("n2", "Parti", "Élève", None, None);
        s2.statut = Some("Sorti".into());
        let mut s3 = src("n3", "Futur", "Sortant", None, None);
        s3.date_sortie = Some(jour(2027, 7, 4));
        let mut s4 = src("n4", "Incoherent", "Statut", None, None);
        s4.date_sortie = Some(jour(2025, 7, 4));
        s4.statut = Some("actif".into());
        let r = lancer(vec![s, s2, s3, s4], vec![]);
        assert!(matches!(v(&r, "n1"), Verdict::Archive { .. }));
        assert!(matches!(v(&r, "n2"), Verdict::Archive { .. }));
        // Sortie future : toujours élève actuel, donc à décider (jamais archivé en silence).
        assert!(matches!(v(&r, "n3"), Verdict::ADecider { .. }));
        // Statut « actif » mais date de sortie passée : incohérence de l'export, jamais archivée en silence.
        assert!(matches!(v(&r, "n4"), Verdict::ADecider { .. }));
    }

    #[test]
    fn resume_minimise_l_ine_et_ne_porte_pas_de_contact() {
        let s = src("n1", "Martin", "Léa", Some("1234567890A"), None);
        let res = resume_source(&s);
        assert_eq!(res.ine_fin.as_deref(), Some("…890A"));
        assert!(res.contact_masque.is_none());
        assert!(!format!("{res:?}").contains("1234567890A"));
    }

    #[test]
    fn meme_classe_par_libelles_equivalents_renforce_un_candidat_sans_naissance() {
        let mut s = src("n1", "Roux", "Paul", None, None);
        s.classe = Some("Seconde B".into());
        let mut c = cib("c1", "Roux", "Paul", None, None);
        c.classe = Some("2nde B".into());
        let r = lancer(vec![s], vec![c]);
        let Verdict::ADecider { candidats, .. } = v(&r, "n1") else {
            panic!()
        };
        assert_eq!(candidats[0].confiance, Confiance::Moyenne);
        assert!(candidats[0].raisons.contains(&Raison::MemeClasse));
    }

    #[test]
    fn correspondance_existante_n_est_pas_recalculee() {
        let mut existantes = Existantes::new();
        existantes.insert(
            (OBJET.into(), "n1".into()),
            crate::correspondance::Existante {
                cible_id: "c9".into(),
                empreinte_source: None,
                modifie_depuis_reprise: false,
                statut: crate::correspondance::Statut::Rapprochee,
            },
        );
        let ctx = Contexte::new(jour(2026, 11, 16), &existantes);
        let cible = CibleRapprochement {
            eleves: vec![cib("c1", "Roux", "Paul", None, None)],
            ..Default::default()
        };
        let mut r = Verdicts::new();
        rapprocher(
            &[src("n1", "Roux", "Paul", None, None)],
            &cible,
            &ctx,
            &mut r,
        );
        assert!(r.is_empty());
    }

    #[test]
    fn candidats_bornes_et_ordre_deterministe() {
        let cibles: Vec<CibleEleve> = (0..9)
            .map(|i| {
                cib(
                    &format!("c{i}"),
                    "Roux",
                    "Paul",
                    None,
                    Some(jour(2012, 1, 1)),
                )
            })
            .collect();
        let r = lancer(
            vec![src("n1", "Roux", "Paul", None, Some(jour(2012, 1, 1)))],
            cibles.clone(),
        );
        let Verdict::ADecider {
            candidats, motif, ..
        } = v(&r, "n1")
        else {
            panic!()
        };
        assert_eq!(candidats.len(), MAX_CANDIDATS);
        assert_eq!(*motif, super::super::types::Motif::Ambigu);
        let r2 = lancer(
            vec![src("n1", "Roux", "Paul", None, Some(jour(2012, 1, 1)))],
            cibles,
        );
        assert_eq!(v(&r, "n1"), v(&r2, "n1"));
    }
}
