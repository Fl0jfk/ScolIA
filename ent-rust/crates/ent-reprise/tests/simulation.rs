//! Tests de la simulation et de la correspondance (lot R2), sur l'export fictif décodable.

mod commun;

use chrono::{TimeZone, Utc};
use commun::*;
use ent_reprise::correspondance::{id_derive, Existante, Existantes, Statut, SOURCE_NEXT};
use ent_reprise::decodage::{decoder_export, Decodage, OptionsDecodage};
use ent_reprise::jeu_fictif;
use ent_reprise::simulation::{correspondances_a_enregistrer, simuler, texte};
use std::path::PathBuf;

const ETAB: &str = "5c011a00-0000-4000-8000-000000000028";

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

#[test]
fn simulation_invariant_et_aucune_ecriture() {
    let (_, d) = jeu("sim-inv");
    let r = simuler(ETAB, &d, &Existantes::new());
    println!("{}", texte(&r));
    assert!(r.invariants_ok());
    assert_eq!(r.ecritures_metier, 0);
    // Sans correspondance : des créations (modules), rien de mis à jour ni d'inchangé.
    assert!(r.total.creees > 0);
    assert_eq!(r.total.mises_a_jour + r.total.inchangees, 0);
    // Le rapprochement n'invente aucune fiche du cœur : tout est « à décider ».
    assert!(r.par_objet["eleve"].a_decider > 0);
    assert_eq!(r.par_objet["eleve"].creees, 0);
    assert_eq!(r.a_decider.len() as u64, r.total.a_decider);
    // Archives : archivées. Santé : refusée.
    assert!(r.par_objet["vs_appel"].archivees > 0);
    assert_eq!(r.par_objet["sante_pai"].refusees, r.par_objet["sante_pai"].lues);
    // Les lignes lues du décodage sont toutes accountées.
    let lues: u64 = d.rapport.tables.values().map(|o| o.lues).sum::<u64>()
        + d.rapport.s3.values().map(|o| o.lues).sum::<u64>();
    assert_eq!(r.total.lues, lues);
}

#[test]
fn identifiants_cibles_stables_et_distincts() {
    let (_, d) = jeu("sim-ids");
    let a = simuler(ETAB, &d, &Existantes::new());
    let b = simuler(ETAB, &d, &Existantes::new());
    let ids = |r: &ent_reprise::simulation::RapportSimulation| r.propositions.iter().map(|p| p.cible_id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&a), ids(&b));
    let mut u = ids(&a);
    u.sort();
    u.dedup();
    assert_eq!(u.len(), a.propositions.len(), "collision d'identifiants cibles");
    let p = &a.propositions[0];
    assert_eq!(p.cible_id, id_derive(ETAB, SOURCE_NEXT, &p.objet, &p.source_id));
    // Autre établissement cible : autres identifiants.
    let c = simuler("5c011a00-0000-4000-8000-000000000029", &d, &Existantes::new());
    assert_ne!(ids(&a), ids(&c));
}

#[test]
fn rejouer_apres_application_ne_change_rien() {
    let (_, d) = jeu("sim-idem");
    let vide = Existantes::new();
    let r1 = simuler(ETAB, &d, &vide);
    let apres = correspondances_a_enregistrer(&r1, &vide);
    let r2 = simuler(ETAB, &d, &apres);
    assert!(r2.invariants_ok());
    // Rien à créer ni à mettre à jour : tout est inchangé ; le « à décider » reste à décider.
    assert_eq!(r2.total.creees, 0);
    assert_eq!(r2.total.mises_a_jour, 0);
    assert_eq!(r2.total.inchangees, r1.total.creees);
    assert_eq!(r2.total.a_decider, r1.total.a_decider);
    assert_eq!(r2.propositions.len(), 0);
    // Réenregistrer ne change pas la table.
    assert_eq!(correspondances_a_enregistrer(&r2, &apres), apres);
}

#[test]
fn source_plus_recente_met_a_jour_et_ajoute() {
    let (_, d) = jeu("sim-diff");
    let r1 = simuler(ETAB, &d, &Existantes::new());
    let mut ex = correspondances_a_enregistrer(&r1, &Existantes::new());
    // Une ligne déjà reprise change à la source (empreinte différente) ; une autre n'existait pas encore.
    let cle = ex.iter().find(|((o, _), e)| o == "travel" && e.statut == Statut::Creee).map(|(k, _)| k.clone()).unwrap();
    ex.get_mut(&cle).unwrap().empreinte_source = Some("ancienne".into());
    let cle2 = ex.iter().find(|((o, _), e)| o == "tarif" && e.statut == Statut::Creee).map(|(k, _)| k.clone()).unwrap();
    ex.remove(&cle2);
    let r2 = simuler(ETAB, &d, &ex);
    assert_eq!(r2.par_objet["travel"].mises_a_jour, 1);
    assert_eq!(r2.par_objet["tarif"].creees, 1);
    assert_eq!(r2.total.mises_a_jour, 1);
    assert_eq!(r2.total.creees, 1);
    assert!(r2.invariants_ok());
}

#[test]
fn objet_modifie_depuis_la_reprise_n_est_jamais_ecrase() {
    let (_, d) = jeu("sim-modif");
    let r1 = simuler(ETAB, &d, &Existantes::new());
    let mut ex = correspondances_a_enregistrer(&r1, &Existantes::new());
    let cle = ex.iter().find(|((o, _), e)| o == "travel" && e.statut == Statut::Creee).map(|(k, _)| k.clone()).unwrap();
    {
        let e = ex.get_mut(&cle).unwrap();
        e.empreinte_source = Some("ancienne".into());
        e.modifie_depuis_reprise = true;
    }
    let r2 = simuler(ETAB, &d, &ex);
    assert_eq!(r2.par_objet["travel"].a_decider, 1);
    assert_eq!(r2.par_objet["travel"].mises_a_jour, 0);
    assert!(r2.a_decider.iter().any(|a| a.motif == "modifie_depuis_reprise" && a.objet == "travel"));
}

#[test]
fn rapprochement_deja_decide_suit_la_correspondance_enregistree() {
    let (_, d) = jeu("sim-decide");
    let r1 = simuler(ETAB, &d, &Existantes::new());
    let a = r1.a_decider.iter().find(|a| a.objet == "eleve").unwrap();
    let mut ex = Existantes::new();
    ex.insert(
        ("eleve".into(), a.source_id.clone()),
        Existante {
            cible_id: "22222222-2222-4222-8222-222222222222".into(),
            empreinte_source: Some(a.empreinte.clone()),
            modifie_depuis_reprise: false,
            statut: Statut::Rapprochee,
        },
    );
    let r2 = simuler(ETAB, &d, &ex);
    assert_eq!(r2.par_objet["eleve"].inchangees, 1);
    assert_eq!(r2.par_objet["eleve"].a_decider, r1.par_objet["eleve"].a_decider - 1);
    // Une correspondance « ignorée » par décision humaine est comptée ignorée.
    ex.get_mut(&("eleve".into(), a.source_id.clone())).unwrap().statut = Statut::Ignoree;
    let r3 = simuler(ETAB, &d, &ex);
    assert_eq!(r3.par_objet["eleve"].ignorees, 1);
    assert!(r3.invariants_ok());
}

#[test]
fn le_rapport_ne_contient_aucun_identifiant_s3_brut_ni_donnee() {
    let (_, d) = jeu("sim-confid");
    let r = simuler(ETAB, &d, &Existantes::new());
    let json = serde_json::to_string(&r).unwrap() + &texte(&r);
    // Les clés S3 de dépôts en attente contiennent un jeton : jamais dans le rapport.
    assert!(!json.contains("jeton-valide") && !json.contains("demande-1"));
    for s in jeu_fictif::SENTINELLES {
        assert!(!json.contains(s));
    }
    assert!(!json.contains("example.test"));
}

#[test]
fn les_depots_s3_en_attente_sont_simules() {
    let (_, d) = jeu("sim-s3");
    let r = simuler(ETAB, &d, &Existantes::new());
    assert_eq!(r.par_objet["s3:RentreeEnAttente"].creees, 1);
    assert_eq!(r.par_objet["s3:RentreeEnAttente"].ignorees, 1); // expiré
    assert_eq!(r.par_objet["s3:DemandeEnAttente"].creees, 1);
    assert_eq!(r.par_objet["s3:Fichier"].ignorees, r.par_objet["s3:Fichier"].lues);
    assert_eq!(r.par_objet["s3:Secret"].refusees, 2);
}
