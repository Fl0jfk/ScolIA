//! Tests du rapprochement R3 intégré à la simulation, sur l'export fictif décodable (aucune donnée réelle,
//! aucune horloge : le jour de référence est fixé).

mod commun;

use chrono::{NaiveDate, TimeZone, Utc};
use commun::*;
use ent_reprise::correspondance::{Existantes, Statut};
use ent_reprise::decodage::{decoder_export, Decodage, OptionsDecodage};
use ent_reprise::jeu_fictif;
use ent_reprise::rapprochement::donnees::{CibleEleve, CiblePersonne, SourceRapprochement};
use ent_reprise::rapprochement::{
    existante_de_decision, valider_decision, ActionDecision, CibleRapprochement, Confiance,
    DecisionDemandee, Motif, PropositionFile, Raison,
};
use ent_reprise::simulation::{correspondances_a_enregistrer, simuler_avec_rapprochement, texte};
use std::path::PathBuf;

const ETAB: &str = "5c011a00-0000-4000-8000-000000000028";

fn jour() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn jeu(nom: &str) -> (PathBuf, Decodage) {
    let d = tmp(nom);
    jeu_fictif::generer_decodable(&d, 3).unwrap();
    let r = decoder_export(&OptionsDecodage {
        dump: d.join("dump.sql"),
        objets_s3: Some(d.join("objets_s3.tsv")),
        contenus_s3: Some(d.join("contenus_s3")),
        maintenant: Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0).unwrap(),
    })
    .unwrap();
    (d, r)
}

/// ScolIA fictif : deux des trois élèves de l'export y existent (l'un avec le même INE, l'autre sans INE),
/// le troisième n'y est pas.
fn cible_depuis(src: &SourceRapprochement) -> CibleRapprochement {
    let e = &src.eleves;
    assert!(e.len() >= 3, "l'export fictif doit porter trois élèves");
    CibleRapprochement {
        eleves: vec![
            CibleEleve {
                id: "11111111-1111-4111-8111-111111111111".into(),
                nom: e[0].nom.clone(),
                nom_naissance: None,
                prenom: e[0].prenom.clone(),
                prenoms: vec![],
                ine: e[0].ine.clone(),
                date_naissance: e[0].date_naissance,
                classe: None,
                actif: true,
            },
            CibleEleve {
                id: "22222222-2222-4222-8222-222222222222".into(),
                nom: e[1].nom.to_uppercase(),
                nom_naissance: None,
                prenom: e[1].prenom.clone(),
                prenoms: vec![],
                ine: None,
                date_naissance: e[1].date_naissance,
                classe: None,
                actif: true,
            },
        ],
        personnes: vec![CiblePersonne {
            id: "33333333-3333-4333-8333-333333333333".into(),
            nom: "Autre".into(),
            prenom: Some("Personne".into()),
            email: None,
            telephone: None,
            eleves: vec![],
        }],
        ..Default::default()
    }
}

#[test]
fn simulation_avec_rapprochement_invariant_deterministe_et_sans_ecriture() {
    let (_, d) = jeu("r3-inv");
    let src = SourceRapprochement::extraire(&d);
    let cible = cible_depuis(&src);
    let a = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    println!("{}", texte(&a));
    assert!(a.invariants_ok(), "invariant (rapprochées comprises) violé");
    assert_eq!(a.ecritures_metier, 0);
    let b = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    assert_eq!(texte(&a), texte(&b));
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap(),
        "rapport déterministe"
    );
    // Les lignes « à décider » du rapport sont celles de la file ou du hors-R3 : rien n'est perdu.
    let hors = a.rapprochement.hors_r3;
    assert_eq!(a.a_decider.len() as u64, a.total.a_decider);
    assert_eq!(a.file.len() as u64 + hors, a.total.a_decider);
    // Aucune ligne n'a un motif « générique de R2 » pour un objet traité par R3.
    assert!(a
        .a_decider
        .iter()
        .filter(|x| ent_reprise::rapprochement::est_objet_r3(&x.objet))
        .all(|x| x.motif != "rapprochement_a_decider"));
}

#[test]
fn le_rapprochement_ne_cree_jamais_de_fiche_et_chaque_proposition_porte_ses_raisons() {
    let (_, d) = jeu("r3-raisons");
    let src = SourceRapprochement::extraire(&d);
    let cible = cible_depuis(&src);
    let r = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    // Élèves : rien n'est créé.
    assert_eq!(r.par_objet["eleve"].creees, 0);
    // L'élève 2 n'a aucun candidat : seule une décision pourra le créer.
    let sans: Vec<_> = r
        .file
        .iter()
        .filter(|l| l.objet == "eleve" && l.candidats.is_empty())
        .collect();
    assert!(!sans.is_empty());
    assert!(sans.iter().all(|l| l.motif == Motif::AucunCandidat));
    // Toute proposition avec candidat porte au moins une raison « pour » et une confiance.
    for l in r.file.iter().filter(|l| !l.candidats.is_empty()) {
        for c in &l.candidats {
            assert!(!c.raisons.is_empty(), "{} sans raison", l.source_ref);
            assert!(c.confiance <= Confiance::Forte || c.confiance == Confiance::Certaine);
        }
    }
    // Un candidat au nom/prénom/naissance exacts sans INE n'est jamais appliqué seul.
    assert!(r.file.iter().any(|l| l.objet == "eleve"
        && l.candidats
            .iter()
            .any(|c| c.raisons.contains(&Raison::NaissanceIdentique))));
}

#[test]
fn le_rapport_ne_contient_aucune_donnee_personnelle() {
    let (_, d) = jeu("r3-rgpd");
    let src = SourceRapprochement::extraire(&d);
    let cible = cible_depuis(&src);
    let r = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    let json = serde_json::to_string(&r).unwrap();
    let lisible = texte(&r);
    for e in &src.eleves {
        for valeur in [
            Some(e.nom.as_str()),
            Some(e.prenom.as_str()),
            e.ine.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|v| v.len() > 4)
        {
            assert!(
                !json.contains(valeur),
                "valeur de donnée dans le rapport JSON : {valeur}"
            );
            assert!(
                !lisible.contains(valeur),
                "valeur de donnée dans le rapport lisible : {valeur}"
            );
        }
    }
    for x in &src.responsables {
        if let Some(m) = x.email.as_deref() {
            assert!(!json.contains(m) && !lisible.contains(m));
        }
    }
}

#[test]
fn une_decision_rejouee_sort_la_ligne_de_la_file_et_le_rejeu_est_idempotent() {
    let (_, d) = jeu("r3-decision");
    let src = SourceRapprochement::extraire(&d);
    let cible = cible_depuis(&src);
    let r1 = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    let ligne = r1
        .file
        .iter()
        .find(|l| l.objet == "eleve" && !l.candidats.is_empty())
        .expect("une proposition d'élève avec candidat");
    let prop = PropositionFile {
        objet: ligne.objet.clone(),
        source_id: ligne.source_id.clone(),
        candidats: ligne.candidats.clone(),
    };
    let cible_id = prop.candidats[0].cible_id.clone();
    let dem = DecisionDemandee {
        objet: ligne.objet.clone(),
        source_id: ligne.source_id.clone(),
        action: ActionDecision::Rattacher,
        cible_id: Some(cible_id.clone()),
        motif: Some("vérifié avec le secrétariat".into()),
    };
    valider_decision(&dem, &prop).expect("décision valide");
    let mut existantes = Existantes::new();
    existantes.insert(
        (dem.objet.clone(), dem.source_id.clone()),
        existante_de_decision(&dem, ETAB),
    );
    let r2 = simuler_avec_rapprochement(ETAB, &d, &existantes, &cible, jour());
    assert!(r2.invariants_ok());
    assert_eq!(
        r2.par_objet["eleve"].rapprochees,
        r1.par_objet["eleve"].rapprochees + 1
    );
    assert_eq!(
        r2.par_objet["eleve"].a_decider,
        r1.par_objet["eleve"].a_decider - 1
    );
    assert!(!r2
        .file
        .iter()
        .any(|l| l.source_id == dem.source_id && l.objet == dem.objet));
    assert!(r2
        .rapprochees
        .iter()
        .any(|x| x.source_id == dem.source_id && x.cible_id == cible_id));
    // Enregistrement puis rejeu : la décision devient « inchangée », rien n'est recréé.
    let apres = correspondances_a_enregistrer(&r2, &existantes);
    let r3 = simuler_avec_rapprochement(ETAB, &d, &apres, &cible, jour());
    assert!(r3.invariants_ok());
    assert_eq!(
        r3.par_objet["eleve"].rapprochees, 0,
        "plus aucun lien nouveau à poser"
    );
    assert!(r3.par_objet["eleve"].inchangees >= 1);
    assert_eq!(r3.par_objet["eleve"].creees, 0);
    assert_eq!(
        apres[&(dem.objet.clone(), dem.source_id.clone())].statut,
        Statut::Rapprochee
    );
}

#[test]
fn creer_et_ignorer_sont_des_decisions_rejouees() {
    let (_, d) = jeu("r3-creer");
    let src = SourceRapprochement::extraire(&d);
    let cible = cible_depuis(&src);
    let r1 = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    let sans: Vec<_> = r1
        .file
        .iter()
        .filter(|l| l.objet == "eleve" && l.candidats.is_empty())
        .collect();
    assert!(!sans.is_empty());
    let mut existantes = Existantes::new();
    let a_creer = sans[0];
    let dem = DecisionDemandee {
        objet: a_creer.objet.clone(),
        source_id: a_creer.source_id.clone(),
        action: ActionDecision::Creer,
        cible_id: None,
        motif: None,
    };
    existantes.insert(
        (dem.objet.clone(), dem.source_id.clone()),
        existante_de_decision(&dem, ETAB),
    );
    let r2 = simuler_avec_rapprochement(ETAB, &d, &existantes, &cible, jour());
    assert!(r2.invariants_ok());
    assert_eq!(
        r2.par_objet["eleve"].creees, 1,
        "la création décidée est proposée, pas faite"
    );
    assert_eq!(r2.ecritures_metier, 0);
    if let Some(autre) = sans.get(1) {
        existantes.insert(
            (autre.objet.clone(), autre.source_id.clone()),
            existante_de_decision(
                &DecisionDemandee {
                    objet: autre.objet.clone(),
                    source_id: autre.source_id.clone(),
                    action: ActionDecision::Ignorer,
                    cible_id: None,
                    motif: Some("fiche de test".into()),
                },
                ETAB,
            ),
        );
        let r3 = simuler_avec_rapprochement(ETAB, &d, &existantes, &cible, jour());
        assert!(r3.invariants_ok());
        assert_eq!(
            r3.par_objet["eleve"].ignorees,
            r1.par_objet["eleve"].ignorees + 1
        );
    }
}

#[test]
fn un_jour_de_reference_different_ne_change_que_ce_qui_en_depend() {
    // Aucun test ne lit l'horloge : le jour est un paramètre. Deux jours différents, mêmes décisions
    // pour un export dont aucune fiche n'a de date de sortie.
    let (_, d) = jeu("r3-jour");
    let src = SourceRapprochement::extraire(&d);
    let cible = cible_depuis(&src);
    let a = simuler_avec_rapprochement(ETAB, &d, &Existantes::new(), &cible, jour());
    let b = simuler_avec_rapprochement(
        ETAB,
        &d,
        &Existantes::new(),
        &cible,
        NaiveDate::from_ymd_opt(2026, 11, 17).unwrap(),
    );
    assert_eq!(a.total, b.total);
}
