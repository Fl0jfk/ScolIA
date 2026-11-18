//! Tests du moteur étendu (socle 21, lot V2). Aucune horloge, aucune base : valeurs fixes.

use super::*;

fn rat(n: i128, d: i128) -> Rat {
    Rat::new(n, d).unwrap()
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

/// Évaluation à identifiant fixe (déterminisme des tris).
fn ev(n: u128, titre: &str, bareme: i64, coef: i64) -> EvalCalcul {
    EvalCalcul {
        id: id(n),
        titre: titre.to_string(),
        bareme_m: bareme * 1000,
        coef_m: coef * 1000,
        comptage: Comptage::Normal,
        hors_bareme_autorise: false,
        sous_matiere_id: None,
        cours_id: None,
        annulee: false,
    }
}

fn note(n: u128, bareme: i64, coef: i64, valeur: f64) -> EntreeCalcul {
    EntreeCalcul::note(ev(n, &format!("E{n}"), bareme, coef), valeur)
}

fn defaut() -> ReglesMoteur {
    ReglesMoteur::default()
}

fn exacte(m: &MoyenneCalculee) -> Option<Rat> {
    m.exacte
}

fn aff(m: &MoyenneCalculee) -> Option<i64> {
    m.affichee_m
}

// ── Rationnels et arrondis ─────────────────────────────────────────────────────────

#[test]
fn rationnels_exacts_et_ordre() {
    assert_eq!(rat(2, 4), rat(1, 2));
    assert_eq!(rat(-1, -2), rat(1, 2));
    assert_eq!(rat(1, 3).ajouter(rat(1, 6)).unwrap(), rat(1, 2));
    assert_eq!(rat(1, 2).retrancher(rat(1, 3)).unwrap(), rat(1, 6));
    assert_eq!(rat(2, 3).multiplier(rat(3, 4)).unwrap(), rat(1, 2));
    assert_eq!(rat(1, 2).diviser(rat(1, 4)).unwrap(), Rat::entier(2));
    assert!(rat(1, 3) < rat(1, 2));
    assert!(rat(-1, 2) < Rat::ZERO);
    assert_eq!(Rat::new(1, 0), Err(ErreurCalcul::DivisionParZero));
    assert_eq!(
        rat(1, 2).diviser(Rat::ZERO),
        Err(ErreurCalcul::DivisionParZero)
    );
}

#[test]
fn debordement_est_une_erreur_jamais_une_panique() {
    let grand = Rat::entier(i128::MAX / 2);
    assert_eq!(grand.multiplier(grand), Err(ErreurCalcul::Debordement));
    assert_eq!(
        grand.ajouter(grand).and_then(|x| x.ajouter(grand)),
        Err(ErreurCalcul::Debordement)
    );
    // La comparaison reste définie même quand le produit croisé déborde.
    let a = Rat::new(i128::MAX - 1, i128::MAX - 2).unwrap();
    let b = Rat::new(i128::MAX - 3, i128::MAX - 4).unwrap();
    assert_ne!(a.cmp(&b), Ordering::Equal);
}

#[test]
fn arrondi_demi_haut_a_0_005_pres() {
    let r = ArrondiRegle::new(Precision::Centieme, ModeArrondi::DemiHaut);
    // 12,495 → 12,50 ; 12,494999… → 12,49 ; 13,125 → 13,13 (conception §24).
    assert_eq!(r.en_milliemes(rat(12495, 1000)).unwrap(), 12_500);
    assert_eq!(r.en_milliemes(rat(12_494_999, 1_000_000)).unwrap(), 12_490);
    assert_eq!(r.en_milliemes(rat(13125, 1000)).unwrap(), 13_130);
    assert_eq!(r.en_milliemes(rat(12_005, 1000)).unwrap(), 12_010);
    assert_eq!(r.en_milliemes(rat(12_004, 1000)).unwrap(), 12_000);
}

#[test]
fn arrondi_les_cinq_modes_sur_le_demi_exact() {
    let x = rat(12345, 1000); // 12,345 : pile entre 12,34 et 12,35
    let c = |m| {
        ArrondiRegle::new(Precision::Centieme, m)
            .en_milliemes(x)
            .unwrap()
    };
    assert_eq!(c(ModeArrondi::DemiHaut), 12_350);
    assert_eq!(c(ModeArrondi::DemiBas), 12_340);
    assert_eq!(c(ModeArrondi::DemiPair), 12_340); // 1234 est pair
    assert_eq!(c(ModeArrondi::Superieur), 12_350);
    assert_eq!(c(ModeArrondi::Inferieur), 12_340);
    // Demi pair : 12,355 → 12,36 (1235 est impair, on monte).
    let y = rat(12355, 1000);
    assert_eq!(
        ArrondiRegle::new(Precision::Centieme, ModeArrondi::DemiPair)
            .en_milliemes(y)
            .unwrap(),
        12_360
    );
}

#[test]
fn arrondi_hors_demi_les_modes_directionnels() {
    let x = rat(12341, 1000);
    let c = |m| {
        ArrondiRegle::new(Precision::Centieme, m)
            .en_milliemes(x)
            .unwrap()
    };
    assert_eq!(c(ModeArrondi::DemiHaut), 12_340);
    assert_eq!(c(ModeArrondi::DemiBas), 12_340);
    assert_eq!(c(ModeArrondi::Superieur), 12_350);
    assert_eq!(c(ModeArrondi::Inferieur), 12_340);
    // Une valeur déjà sur le pas ne bouge pour aucun mode.
    for m in ModeArrondi::TOUS {
        assert_eq!(
            ArrondiRegle::new(Precision::Centieme, m)
                .en_milliemes(rat(1234, 100))
                .unwrap(),
            12_340
        );
    }
}

#[test]
fn arrondi_dixieme_demi_point_quart_unite() {
    let h = ModeArrondi::DemiHaut;
    assert_eq!(
        ArrondiRegle::new(Precision::Dixieme, h)
            .en_milliemes(rat(1225, 100))
            .unwrap(),
        12_300
    ); // 12,25 → 12,3
    assert_eq!(
        ArrondiRegle::new(Precision::Dixieme, h)
            .en_milliemes(rat(1224, 100))
            .unwrap(),
        12_200
    );
    assert_eq!(
        ArrondiRegle::new(Precision::DemiPoint, h)
            .en_milliemes(rat(1225, 100))
            .unwrap(),
        12_500
    ); // 12,25 → 12,5
    assert_eq!(
        ArrondiRegle::new(Precision::DemiPoint, h)
            .en_milliemes(rat(1224, 100))
            .unwrap(),
        12_000
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Quart, h)
            .en_milliemes(rat(1213, 100))
            .unwrap(),
        12_250
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Quart, h)
            .en_milliemes(rat(1211, 100))
            .unwrap(),
        12_000
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Unite, h)
            .en_milliemes(rat(125, 10))
            .unwrap(),
        13_000
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Unite, h)
            .en_milliemes(rat(124, 10))
            .unwrap(),
        12_000
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Millieme, h)
            .en_milliemes(rat(1, 3))
            .unwrap(),
        333
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Millieme, h)
            .en_milliemes(rat(2, 3))
            .unwrap(),
        667
    );
}

#[test]
fn arrondi_de_zero_et_des_negatifs() {
    for m in ModeArrondi::TOUS {
        assert_eq!(
            ArrondiRegle::new(Precision::Centieme, m)
                .en_milliemes(Rat::ZERO)
                .unwrap(),
            0
        );
    }
    // Plancher et plafond sont corrects aussi sous zéro (div_euclid).
    assert_eq!(
        ArrondiRegle::new(Precision::Unite, ModeArrondi::Inferieur)
            .en_milliemes(rat(-1, 2))
            .unwrap(),
        -1000
    );
    assert_eq!(
        ArrondiRegle::new(Precision::Unite, ModeArrondi::Superieur)
            .en_milliemes(rat(-1, 2))
            .unwrap(),
        0
    );
}

#[test]
fn formatage_francais() {
    assert_eq!(formater_fr(12_800), "12,8");
    assert_eq!(formater_fr(14_000), "14");
    assert_eq!(formater_fr(9_125), "9,125");
    assert_eq!(formater_fr(0), "0");
    assert_eq!(formater_fr(-500), "-0,5");
}

#[test]
fn les_codes_se_relisent() {
    for s in StatutCalcul::TOUS {
        assert_eq!(StatutCalcul::parse(s.as_str()), Some(s));
    }
    for p in Precision::TOUTES {
        assert_eq!(Precision::parse(p.as_str()), Some(p));
    }
    for m in ModeArrondi::TOUS {
        assert_eq!(ModeArrondi::parse(m.as_str()), Some(m));
    }
    for c in Comptage::TOUS {
        assert_eq!(Comptage::parse(c.as_str()), Some(c));
    }
    assert_eq!(StatutCalcul::parse("zero"), None);
    assert!(!StatutCalcul::NonConcerne.saisissable());
    assert!(!StatutCalcul::EnAttente.saisissable());
    assert!(StatutCalcul::NonRendu.saisissable());
}

// ── Exemple de référence (§8.8) ────────────────────────────────────────────────────

#[test]
fn exemple_de_reference_lea_12_80() {
    // DS 1 : 14/20 coef 2 ; interro 7/10 coef 1 ; DM non rendu ; DS 2 absent justifié rattrapé
    // 11/20 coef 2 ; évaluation diagnostique « ne compte pas » 4/20.
    let entrees = vec![
        note(1, 20, 2, 14.0),
        note(2, 10, 1, 7.0),
        EntreeCalcul::statut(ev(3, "DM", 20, 1), StatutCalcul::NonRendu),
        note(4, 20, 2, 11.0),
        EntreeCalcul::note(
            ev(5, "Diagnostique", 20, 1).avec_comptage(Comptage::NeComptePas),
            4.0,
        ),
    ];
    let m = moyenne_cours(&entrees, &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(rat(64, 5)));
    assert_eq!(aff(&m), Some(12_800));
    assert_eq!(m.nb_comptees(), 3);
    assert_eq!(m.nb_non_rendus(), 1);
    let effets: Vec<Effet> = m.lignes.iter().map(|l| l.effet).collect();
    assert_eq!(
        effets,
        vec![
            Effet::Comptee,
            Effet::Comptee,
            Effet::NonComptee(RaisonNonComptee::NonRendu),
            Effet::Comptee,
            Effet::NonComptee(RaisonNonComptee::NeComptePas),
        ]
    );
    let texte = m.explication_fr(&defaut());
    assert_eq!(texte.len(), 6);
    assert!(
        texte[1].contains("7 sur 10") && texte[1].contains("soit 14 sur 20"),
        "{}",
        texte[1]
    );
    assert!(texte[2].contains("non rendu, non compté"));
    assert!(texte[4].contains("ne compte pas"));
    assert_eq!(texte[5], "Moyenne : 12,8 sur 20");
}

#[test]
fn explication_dit_la_formule_et_reste_exacte() {
    let m = moyenne_cours(&[note(1, 20, 1, 10.0), note(2, 20, 1, 13.0)], &defaut()).unwrap();
    assert_eq!(m.formule, Formule::NotesRamenees);
    assert_eq!(exacte(&m), Some(rat(23, 2)));
    assert_eq!(aff(&m), Some(11_500));
}

// ── Cas limites de la moyenne de cours ─────────────────────────────────────────────

#[test]
fn aucune_note_aucune_moyenne() {
    let m = moyenne_cours(&[], &defaut()).unwrap();
    assert_eq!(exacte(&m), None);
    assert_eq!(aff(&m), None);
    assert!(m
        .explication_fr(&defaut())
        .last()
        .unwrap()
        .starts_with("Aucune moyenne"));
}

#[test]
fn tous_absents_aucune_moyenne_et_jamais_zero() {
    let entrees: Vec<EntreeCalcul> = (1..=6)
        .map(|n| EntreeCalcul::statut(ev(n, "DS", 20, 1), StatutCalcul::Absent))
        .collect();
    let m = moyenne_cours(&entrees, &defaut()).unwrap();
    assert_eq!(exacte(&m), None);
    assert!(m
        .lignes
        .iter()
        .all(|l| l.effet == Effet::NonComptee(RaisonNonComptee::Absent)));
}

#[test]
fn chaque_statut_non_compte_a_sa_raison() {
    let cas = [
        (StatutCalcul::Absent, RaisonNonComptee::Absent),
        (StatutCalcul::ARattraper, RaisonNonComptee::ARattraper),
        (StatutCalcul::Dispense, RaisonNonComptee::Dispense),
        (StatutCalcul::NonRendu, RaisonNonComptee::NonRendu),
        (StatutCalcul::NonNote, RaisonNonComptee::NonNote),
        (StatutCalcul::NonConcerne, RaisonNonComptee::NonConcerne),
        (StatutCalcul::Annule, RaisonNonComptee::Annule),
        (StatutCalcul::EnAttente, RaisonNonComptee::EnAttente),
    ];
    for (statut, raison) in cas {
        let m = moyenne_cours(
            &[
                note(1, 20, 1, 12.0),
                EntreeCalcul::statut(ev(2, "X", 20, 1), statut),
            ],
            &defaut(),
        )
        .unwrap();
        assert_eq!(
            exacte(&m),
            Some(Rat::entier(12)),
            "{statut:?} ne doit pas changer la moyenne"
        );
        assert_eq!(m.lignes[1].effet, Effet::NonComptee(raison));
    }
}

#[test]
fn une_seule_note() {
    let m = moyenne_cours(&[note(1, 40, 3, 30.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(15)));
}

#[test]
fn bareme_10_20_40_ramenes_a_20() {
    let m = moyenne_cours(
        &[
            note(1, 10, 1, 8.0),
            note(2, 20, 1, 8.0),
            note(3, 40, 1, 8.0),
        ],
        &defaut(),
    )
    .unwrap();
    // 16 + 8 + 4 = 28 / 3
    assert_eq!(exacte(&m), Some(rat(28, 3)));
    assert_eq!(aff(&m), Some(9_330));
}

#[test]
fn echelle_10_et_100() {
    let mut r = defaut();
    r.echelle_m = 10_000;
    let m = moyenne_cours(&[note(1, 20, 1, 15.0)], &r).unwrap();
    assert_eq!(exacte(&m), Some(rat(15, 2)));
    r.echelle_m = 100_000;
    let m = moyenne_cours(&[note(1, 20, 1, 15.0)], &r).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(75)));
}

#[test]
fn coefficient_zero_ne_pese_rien() {
    let m = moyenne_cours(&[note(1, 20, 0, 1.0), note(2, 20, 2, 15.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(15)));
    assert_eq!(
        m.lignes[0].effet,
        Effet::NonComptee(RaisonNonComptee::CoefficientNul)
    );
    // Que des coefficients nuls : aucune moyenne.
    let m = moyenne_cours(&[note(1, 20, 0, 1.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), None);
}

#[test]
fn coefficients_decimaux() {
    let mut a = ev(1, "A", 20, 1);
    a.coef_m = 500; // 0,5
    let mut b = ev(2, "B", 20, 1);
    b.coef_m = 1500; // 1,5
    let m = moyenne_cours(
        &[EntreeCalcul::note(a, 10.0), EntreeCalcul::note(b, 14.0)],
        &defaut(),
    )
    .unwrap();
    // (10×0,5 + 14×1,5) / 2 = 13
    assert_eq!(exacte(&m), Some(Rat::entier(13)));
}

#[test]
fn donnees_incoherentes_jamais_transformees_en_note() {
    // Note sans valeur ; valeur au-delà du barème ; barème nul ; valeur négative.
    let sans_valeur = EntreeCalcul {
        evaluation: ev(1, "A", 20, 1),
        statut: StatutCalcul::Note,
        valeur_m: None,
    };
    let hors = EntreeCalcul {
        evaluation: ev(2, "B", 20, 1),
        statut: StatutCalcul::Note,
        valeur_m: Some(21_000),
    };
    let mut b0 = ev(3, "C", 20, 1);
    b0.bareme_m = 0;
    let bareme_nul = EntreeCalcul {
        evaluation: b0,
        statut: StatutCalcul::Note,
        valeur_m: Some(1000),
    };
    let negative = EntreeCalcul {
        evaluation: ev(4, "D", 20, 1),
        statut: StatutCalcul::Note,
        valeur_m: Some(-1),
    };
    let m = moyenne_cours(
        &[
            note(9, 20, 1, 10.0),
            sans_valeur,
            hors,
            bareme_nul,
            negative,
        ],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(10)));
    for l in &m.lignes[1..] {
        assert_eq!(l.effet, Effet::NonComptee(RaisonNonComptee::DonneeInvalide));
    }
}

#[test]
fn hors_bareme_autorise_compte_et_est_plafonne_a_l_echelle() {
    let mut e = ev(1, "Bonus de sujet", 20, 1);
    e.hors_bareme_autorise = true;
    let m = moyenne_cours(&[EntreeCalcul::note(e.clone(), 22.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(20)));
    assert!(m.plafonnee);
    let mut r = defaut();
    r.plafonner_a_l_echelle = false;
    let m = moyenne_cours(&[EntreeCalcul::note(e, 22.0)], &r).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(22)));
    assert!(!m.plafonnee);
}

#[test]
fn evaluation_annulee_jamais_comptee() {
    let mut e = ev(1, "Sujet diffusé", 20, 1);
    e.annulee = true;
    let m = moyenne_cours(
        &[EntreeCalcul::note(e, 5.0), note(2, 20, 1, 14.0)],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(14)));
    assert_eq!(
        m.lignes[0].effet,
        Effet::NonComptee(RaisonNonComptee::EvaluationAnnulee)
    );
}

#[test]
fn echelle_non_positive_refusee() {
    let mut r = defaut();
    r.echelle_m = 0;
    assert_eq!(
        moyenne_cours(&[note(1, 20, 1, 10.0)], &r).unwrap_err(),
        ErreurCalcul::Reglage
    );
    let mut r = defaut();
    r.bonus = TraitementBonus::PointsAuDessusDe { seuil_m: -1 };
    assert_eq!(moyenne_cours(&[], &r).unwrap_err(), ErreurCalcul::Reglage);
}

// ── Non rendu ──────────────────────────────────────────────────────────────────────

#[test]
fn non_rendu_exclu_par_defaut_zero_par_choix() {
    let entrees = vec![
        note(1, 20, 1, 14.0),
        EntreeCalcul::statut(ev(2, "DM", 20, 1), StatutCalcul::NonRendu),
    ];
    assert_eq!(
        exacte(&moyenne_cours(&entrees, &defaut()).unwrap()),
        Some(Rat::entier(14))
    );
    let mut r = defaut();
    r.non_rendu = TraitementNonRendu::Zero;
    let m = moyenne_cours(&entrees, &r).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(7)));
    assert_eq!(m.lignes[1].effet, Effet::CompteeZeroNonRendu);
    assert!(m.lignes[1].texte_fr(20_000).contains("compté 0"));
}

#[test]
fn une_absence_n_est_jamais_un_zero_quel_que_soit_le_reglage() {
    // Aucun réglage ne convertit une absence (R-V5) : on essaie toutes les combinaisons.
    let entrees = vec![
        note(1, 20, 1, 14.0),
        EntreeCalcul::statut(ev(2, "DS", 20, 1), StatutCalcul::Absent),
    ];
    for nr in [TraitementNonRendu::Exclu, TraitementNonRendu::Zero] {
        for p in [
            PonderationNotes::NotesRamenees,
            PonderationNotes::PointsCumules,
        ] {
            let mut r = defaut();
            r.non_rendu = nr;
            r.ponderation = p;
            assert_eq!(
                exacte(&moyenne_cours(&entrees, &r).unwrap()),
                Some(Rat::entier(14))
            );
        }
    }
}

#[test]
fn diagnostic_absents_zero_explique_un_ecart_sans_etre_une_moyenne_officielle() {
    let entrees = vec![
        note(1, 20, 1, 14.0),
        EntreeCalcul::statut(ev(2, "DS", 20, 1), StatutCalcul::Absent),
        EntreeCalcul::statut(ev(3, "DS bis", 20, 1), StatutCalcul::ARattraper),
    ];
    let officielle = moyenne_cours(&entrees, &defaut()).unwrap();
    let diag = diagnostic_absents_comptes_zero(&entrees, &defaut()).unwrap();
    assert_eq!(exacte(&officielle), Some(Rat::entier(14)));
    assert_eq!(exacte(&diag), Some(rat(14, 3)));
}

// ── Pondération des barèmes ────────────────────────────────────────────────────────

#[test]
fn points_cumules_different_des_notes_ramenees_quand_les_baremes_different() {
    // 10/10 coef 1 et 10/20 coef 1 : ramenées → (20 + 10)/2 = 15 ; points → 20/30×20 = 13,33.
    let entrees = vec![note(1, 10, 1, 10.0), note(2, 20, 1, 10.0)];
    let ramenees = moyenne_cours(&entrees, &defaut()).unwrap();
    assert_eq!(exacte(&ramenees), Some(Rat::entier(15)));
    let mut r = defaut();
    r.ponderation = PonderationNotes::PointsCumules;
    let points = moyenne_cours(&entrees, &r).unwrap();
    assert_eq!(points.formule, Formule::PointsCumules);
    assert_eq!(exacte(&points), Some(rat(40, 3)));
    assert_eq!(aff(&points), Some(13_330));
    // Même barème partout : les deux méthodes coïncident.
    let memes = vec![note(1, 20, 2, 10.0), note(2, 20, 1, 16.0)];
    assert_eq!(
        moyenne_cours(&memes, &defaut()).unwrap().exacte,
        moyenne_cours(&memes, &r).unwrap().exacte
    );
}

// ── « Compte si elle améliore » ────────────────────────────────────────────────────

fn si_ameliore(n: u128, valeur: f64) -> EntreeCalcul {
    EntreeCalcul::note(
        ev(n, &format!("SA{n}"), 20, 1).avec_comptage(Comptage::SiAmeliore),
        valeur,
    )
}

#[test]
fn si_ameliore_compte_quand_elle_fait_monter() {
    let m = moyenne_cours(&[note(1, 20, 1, 10.0), si_ameliore(2, 16.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(13)));
    assert_eq!(m.lignes[1].effet, Effet::CompteeCarAmeliore);
}

#[test]
fn si_ameliore_ne_compte_pas_quand_elle_baisse_ou_egale() {
    let m = moyenne_cours(&[note(1, 20, 1, 12.0), si_ameliore(2, 6.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(12)));
    assert_eq!(
        m.lignes[1].effet,
        Effet::NonComptee(RaisonNonComptee::NonRetenueCarElleBaisse)
    );
    // Égalité : ne change rien, donc non retenue.
    let m = moyenne_cours(&[note(1, 20, 1, 12.0), si_ameliore(2, 12.0)], &defaut()).unwrap();
    assert_eq!(
        m.lignes[1].effet,
        Effet::NonComptee(RaisonNonComptee::NonRetenueCarElleBaisse)
    );
}

#[test]
fn si_ameliore_seule_compte_comme_unique_note() {
    let m = moyenne_cours(&[si_ameliore(1, 4.0)], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(4)));
    assert_eq!(m.lignes[0].effet, Effet::CompteeCarAmeliore);
}

#[test]
fn deux_si_ameliore_ne_dependent_pas_de_l_ordre_de_saisie() {
    // Base 10 ; deux évaluations « si améliore » : 18 (monte) puis 12 (monte encore la base
    // 10 mais pas la moyenne 14). L'ordre canonique les traite par note décroissante.
    let base = note(1, 20, 1, 10.0);
    let a = si_ameliore(2, 18.0);
    let b = si_ameliore(3, 12.0);
    let r1 = moyenne_cours(&[base.clone(), a.clone(), b.clone()], &defaut()).unwrap();
    let r2 = moyenne_cours(&[b, a, base], &defaut()).unwrap();
    assert_eq!(r1.exacte, r2.exacte);
    assert_eq!(exacte(&r1), Some(Rat::entier(14)));
    let effet = |m: &MoyenneCalculee, i: u128| {
        m.lignes
            .iter()
            .find(|l| l.evaluation_id == id(i))
            .unwrap()
            .effet
    };
    assert_eq!(effet(&r1, 3), effet(&r2, 3));
    assert_eq!(
        effet(&r1, 3),
        Effet::NonComptee(RaisonNonComptee::NonRetenueCarElleBaisse)
    );
}

#[test]
fn si_ameliore_ne_fait_jamais_baisser_la_moyenne() {
    // Propriété : ajouter une évaluation « si améliore » ne fait jamais baisser la moyenne.
    let base = vec![
        note(1, 20, 2, 9.0),
        note(2, 10, 1, 7.0),
        note(3, 40, 1, 31.0),
    ];
    let avant = exacte(&moyenne_cours(&base, &defaut()).unwrap()).unwrap();
    for v in 0..=40 {
        let mut e = base.clone();
        e.push(si_ameliore(9, f64::from(v) / 2.0));
        let apres = exacte(&moyenne_cours(&e, &defaut()).unwrap()).unwrap();
        assert!(
            apres >= avant,
            "valeur {} : {apres:?} < {avant:?}",
            f64::from(v) / 2.0
        );
    }
}

// ── Ne compte pas, bonus ──────────────────────────────────────────────────────────

#[test]
fn ajouter_une_evaluation_ne_compte_pas_ne_change_rien() {
    let base = vec![note(1, 20, 2, 9.0), note(2, 10, 1, 7.0)];
    let avant = moyenne_cours(&base, &defaut()).unwrap();
    for v in [0.0, 5.0, 20.0] {
        let mut e = base.clone();
        e.push(EntreeCalcul::note(
            ev(9, "Diag", 20, 5).avec_comptage(Comptage::NeComptePas),
            v,
        ));
        assert_eq!(moyenne_cours(&e, &defaut()).unwrap().exacte, avant.exacte);
    }
}

#[test]
fn note_bonus_seule_sans_base_ne_donne_pas_de_moyenne() {
    let bonus = EntreeCalcul::note(ev(1, "Bonus", 20, 1).avec_comptage(Comptage::Bonus), 18.0);
    // Ignorée par défaut.
    let m = moyenne_cours(std::slice::from_ref(&bonus), &defaut()).unwrap();
    assert_eq!(exacte(&m), None);
    assert_eq!(
        m.lignes[0].effet,
        Effet::NonComptee(RaisonNonComptee::BonusIgnore)
    );
    // « Points au-dessus du seuil » sans note de base : pas de moyenne non plus (jamais 0).
    let mut r = defaut();
    r.bonus = TraitementBonus::PointsAuDessusDe { seuil_m: 10_000 };
    let m = moyenne_cours(std::slice::from_ref(&bonus), &r).unwrap();
    assert_eq!(exacte(&m), None);
    // « Comme normale » : une note ordinaire.
    r.bonus = TraitementBonus::CommeNormale;
    assert_eq!(
        exacte(&moyenne_cours(&[bonus], &r).unwrap()),
        Some(Rat::entier(18))
    );
}

#[test]
fn bonus_points_au_dessus_du_seuil_s_ajoutent_puis_plafonnent() {
    let mut r = defaut();
    r.bonus = TraitementBonus::PointsAuDessusDe { seuil_m: 10_000 };
    let base = vec![note(1, 20, 1, 12.0), note(2, 20, 1, 14.0)]; // moyenne 13
    let bonus = EntreeCalcul::note(ev(3, "Bonus", 20, 1).avec_comptage(Comptage::Bonus), 16.0);
    let mut e = base.clone();
    e.push(bonus);
    let m = moyenne_cours(&e, &r).unwrap();
    // (16 − 10) × 1 / (1 + 1) = 3 points ajoutés : 13 + 3 = 16.
    assert_eq!(m.ajout_bonus, Some(Rat::entier(3)));
    assert_eq!(exacte(&m), Some(Rat::entier(16)));
    assert_eq!(m.lignes[2].effet, Effet::CompteeCommeBonus);
    // Bonus sous le seuil : aucun point.
    let mut e = base.clone();
    e.push(EntreeCalcul::note(
        ev(3, "Bonus", 20, 1).avec_comptage(Comptage::Bonus),
        8.0,
    ));
    assert_eq!(
        exacte(&moyenne_cours(&e, &r).unwrap()),
        Some(Rat::entier(13))
    );
    // Plafond à l'échelle.
    let mut e = vec![note(1, 20, 1, 19.0)];
    e.push(EntreeCalcul::note(
        ev(3, "Bonus", 20, 4).avec_comptage(Comptage::Bonus),
        20.0,
    ));
    let m = moyenne_cours(&e, &r).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(20)));
    assert!(m.plafonnee);
}

// ── Arrondis d'étape ───────────────────────────────────────────────────────────────

#[test]
fn par_defaut_aucun_arrondi_intermediaire() {
    // 1/3 + 2/3 : exact avant affichage.
    let m = moyenne_cours(&[note(1, 3, 1, 1.0), note(2, 3, 1, 2.0)], &defaut()).unwrap();
    // (20/3 + 40/3)/2 = 10
    assert_eq!(exacte(&m), Some(Rat::entier(10)));
}

#[test]
fn arrondi_de_chaque_note_ramenee_change_la_moyenne() {
    // 1/3 sur 3 ramené à 20 = 6,666… : arrondi à 6,67 avant pondération.
    let entrees = vec![note(1, 3, 1, 1.0), note(2, 20, 1, 7.0)];
    let exact = moyenne_cours(&entrees, &defaut()).unwrap();
    let mut r = defaut();
    r.arrondis.note_ramenee = Some(ArrondiRegle::new(
        Precision::Centieme,
        ModeArrondi::DemiHaut,
    ));
    let arrondie = moyenne_cours(&entrees, &r).unwrap();
    assert_ne!(exact.exacte, arrondie.exacte);
    // (6,67 + 7) / 2 = 6,835
    assert_eq!(arrondie.exacte, Some(rat(6835, 1000)));
    assert_eq!(arrondie.lignes[0].note_ramenee, Some(rat(667, 100)));
}

#[test]
fn arrondi_de_la_moyenne_de_matiere_avant_la_generale() {
    let (m1, m2) = (id(1), id(2));
    // Moyennes de matière : 10,004 (coef 1) et 11,996 (coef 1).
    let matieres = vec![
        MatiereMoyenne {
            matiere_id: m1,
            moyenne: Some(rat(10004, 1000)),
            coef_m: 1000,
            hors_moyenne_generale: false,
        },
        MatiereMoyenne {
            matiere_id: m2,
            moyenne: Some(rat(11996, 1000)),
            coef_m: 1000,
            hors_moyenne_generale: false,
        },
    ];
    let exacte_g = moyenne_generale(&matieres, &defaut()).unwrap();
    assert_eq!(exacte(&exacte_g), Some(Rat::entier(11)));
    let mut r = defaut();
    r.arrondis.moyenne_matiere = Some(ArrondiRegle::new(Precision::Dixieme, ModeArrondi::DemiHaut));
    let g = moyenne_generale(&matieres, &r).unwrap();
    // 10,0 et 12,0 → 11
    assert_eq!(exacte(&g), Some(Rat::entier(11)));
    // Cas où l'arrondi d'étape se voit : 10,04 et 10,06 → 10,0 et 10,1 → 10,05.
    let matieres = vec![
        MatiereMoyenne {
            matiere_id: m1,
            moyenne: Some(rat(1004, 100)),
            coef_m: 1000,
            hors_moyenne_generale: false,
        },
        MatiereMoyenne {
            matiere_id: m2,
            moyenne: Some(rat(1006, 100)),
            coef_m: 1000,
            hors_moyenne_generale: false,
        },
    ];
    assert_eq!(
        exacte(&moyenne_generale(&matieres, &defaut()).unwrap()),
        Some(rat(1005, 100))
    );
    assert_eq!(
        exacte(&moyenne_generale(&matieres, &r).unwrap()),
        Some(rat(1005, 100))
    );
    r.arrondis.moyenne_matiere = Some(ArrondiRegle::new(Precision::Unite, ModeArrondi::DemiHaut));
    assert_eq!(
        exacte(&moyenne_generale(&matieres, &r).unwrap()),
        Some(Rat::entier(10))
    );
}

#[test]
fn le_mode_d_affichage_est_reglable() {
    let entrees = vec![note(1, 20, 1, 12.0), note(2, 20, 1, 12.69)]; // 12,345
    let mut r = defaut();
    assert_eq!(aff(&moyenne_cours(&entrees, &r).unwrap()), Some(12_350));
    r.affichage = ArrondiRegle::new(Precision::Centieme, ModeArrondi::DemiPair);
    assert_eq!(aff(&moyenne_cours(&entrees, &r).unwrap()), Some(12_340));
    r.affichage = ArrondiRegle::new(Precision::Dixieme, ModeArrondi::Inferieur);
    assert_eq!(aff(&moyenne_cours(&entrees, &r).unwrap()), Some(12_300));
}

// ── Sous-matières et plusieurs cours ───────────────────────────────────────────────

fn en_sous_matiere(mut e: EntreeCalcul, sm: u128) -> EntreeCalcul {
    e.evaluation.sous_matiere_id = Some(id(sm));
    e
}

#[test]
fn sous_matieres_moyenne_ponderee_des_moyennes() {
    // Grammaire : 10 et 12 → 11 (coef sous-matière 1) ; Rédaction : 18 (coef 3) → (11 + 54)/4.
    let entrees = vec![
        en_sous_matiere(note(1, 20, 1, 10.0), 100),
        en_sous_matiere(note(2, 20, 1, 12.0), 100),
        en_sous_matiere(note(3, 20, 1, 18.0), 200),
    ];
    let coefs = [
        CoefSousMatiere {
            sous_matiere_id: Some(id(100)),
            coef_m: 1000,
        },
        CoefSousMatiere {
            sous_matiere_id: Some(id(200)),
            coef_m: 3000,
        },
    ];
    let m = moyenne_matiere(&entrees, &coefs, &defaut()).unwrap();
    assert_eq!(m.formule, Formule::Composantes);
    assert_eq!(exacte(&m), Some(rat(65, 4)));
    assert_eq!(m.composantes.len(), 2);
    assert_eq!(m.lignes.len(), 3);
}

#[test]
fn sous_matieres_a_plat_ignore_les_coefficients_de_sous_matiere() {
    let entrees = vec![
        en_sous_matiere(note(1, 20, 1, 10.0), 100),
        en_sous_matiere(note(2, 20, 1, 12.0), 100),
        en_sous_matiere(note(3, 20, 1, 18.0), 200),
    ];
    let mut r = defaut();
    r.sous_matieres = MethodeSousMatieres::ToutesLesNotes;
    let m = moyenne_matiere(&entrees, &[], &r).unwrap();
    assert_eq!(
        exacte(&m),
        Some(Rat::entier(40).diviser(Rat::entier(3)).unwrap())
    );
}

#[test]
fn sous_matiere_sans_moyenne_est_ignoree() {
    let entrees = vec![
        en_sous_matiere(note(1, 20, 1, 10.0), 100),
        en_sous_matiere(
            EntreeCalcul::statut(ev(2, "Absente", 20, 1), StatutCalcul::Absent),
            200,
        ),
    ];
    let m = moyenne_matiere(&entrees, &[], &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(10)));
    let ignoree = m
        .composantes
        .iter()
        .find(|c| c.niveau == Niveau::SousMatiere(Some(id(200))))
        .unwrap();
    assert!(!ignoree.compte);
    assert_eq!(ignoree.raison, Some(RaisonNonComptee::EnAttente));
}

#[test]
fn evaluations_directes_et_sous_matieres_se_melangent_par_groupe() {
    let entrees = vec![
        note(1, 20, 1, 8.0),
        en_sous_matiere(note(2, 20, 1, 14.0), 100),
    ];
    let m = moyenne_matiere(&entrees, &[], &defaut()).unwrap();
    // Deux groupes de coefficient 1 : (8 + 14) / 2.
    assert_eq!(exacte(&m), Some(Rat::entier(11)));
}

#[test]
fn sans_sous_matiere_matiere_et_cours_coincident() {
    let entrees = vec![
        note(1, 20, 2, 9.0),
        note(2, 10, 1, 7.0),
        note(3, 20, 1, 15.5),
    ];
    for methode in [
        MethodeSousMatieres::MoyenneDesMoyennes,
        MethodeSousMatieres::ToutesLesNotes,
    ] {
        let mut r = defaut();
        r.sous_matieres = methode;
        assert_eq!(
            moyenne_matiere(&entrees, &[], &r).unwrap().exacte,
            moyenne_cours(&entrees, &r).unwrap().exacte
        );
    }
}

#[test]
fn matiere_sur_deux_cours_garde_les_notes_des_deux_groupes() {
    // CE-11 : un élève passé du groupe 1 au groupe 2 de maths : les notes des deux comptent.
    let mut e1 = ev(1, "DS groupe 1", 20, 2);
    e1.cours_id = Some(id(901));
    let mut e2 = ev(2, "DS groupe 2", 20, 1);
    e2.cours_id = Some(id(902));
    let m = moyenne_matiere(
        &[EntreeCalcul::note(e1, 10.0), EntreeCalcul::note(e2, 13.0)],
        &[],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(11)));
}

#[test]
fn coefficient_de_sous_matiere_nul_ecarte_la_sous_matiere() {
    let entrees = vec![
        en_sous_matiere(note(1, 20, 1, 10.0), 100),
        en_sous_matiere(note(2, 20, 1, 20.0), 200),
    ];
    let coefs = [CoefSousMatiere {
        sous_matiere_id: Some(id(200)),
        coef_m: 0,
    }];
    let m = moyenne_matiere(&entrees, &coefs, &defaut()).unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(10)));
}

#[test]
fn arrondi_de_sous_matiere_avant_ponderation() {
    let entrees = vec![
        en_sous_matiere(note(1, 3, 1, 1.0), 100), // 6,666…
        en_sous_matiere(note(2, 20, 1, 10.0), 200),
    ];
    let mut r = defaut();
    let exact = moyenne_matiere(&entrees, &[], &r).unwrap();
    r.arrondis.moyenne_sous_matiere =
        Some(ArrondiRegle::new(Precision::Dixieme, ModeArrondi::DemiHaut));
    let arrondie = moyenne_matiere(&entrees, &[], &r).unwrap();
    assert_ne!(exact.exacte, arrondie.exacte);
    assert_eq!(arrondie.exacte, Some(rat(835, 100))); // (6,7 + 10)/2
}

// ── Moyenne générale ───────────────────────────────────────────────────────────────

fn mat(n: u128, moyenne: Option<Rat>, coef: i64) -> MatiereMoyenne {
    MatiereMoyenne {
        matiere_id: id(n),
        moyenne,
        coef_m: coef * 1000,
        hors_moyenne_generale: false,
    }
}

#[test]
fn generale_ponderee_par_coefficient_de_matiere() {
    let g = moyenne_generale(
        &[
            mat(1, Some(Rat::entier(12)), 3),
            mat(2, Some(Rat::entier(8)), 1),
        ],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&g), Some(Rat::entier(11)));
}

#[test]
fn generale_non_ponderee_ignore_les_coefficients() {
    let mut r = defaut();
    r.generale = PonderationGenerale::NonPonderee;
    let g = moyenne_generale(
        &[
            mat(1, Some(Rat::entier(12)), 3),
            mat(2, Some(Rat::entier(8)), 1),
        ],
        &r,
    )
    .unwrap();
    assert_eq!(exacte(&g), Some(Rat::entier(10)));
}

#[test]
fn generale_ignore_matieres_sans_moyenne_hors_moyenne_et_coefficient_nul() {
    let mut hors = mat(3, Some(Rat::entier(20)), 2);
    hors.hors_moyenne_generale = true;
    let g = moyenne_generale(
        &[
            mat(1, Some(Rat::entier(12)), 2),
            mat(2, None, 5),
            hors,
            mat(4, Some(Rat::entier(0)), 0),
        ],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&g), Some(Rat::entier(12)));
    let raison = |n: u128| {
        g.composantes
            .iter()
            .find(|c| c.niveau == Niveau::Matiere(id(n)))
            .unwrap()
            .raison
    };
    assert_eq!(raison(2), Some(RaisonNonComptee::EnAttente));
    assert_eq!(raison(3), Some(RaisonNonComptee::NeComptePas));
    assert_eq!(raison(4), Some(RaisonNonComptee::CoefficientNul));
}

#[test]
fn generale_sans_aucune_matiere_aucune_moyenne() {
    assert_eq!(exacte(&moyenne_generale(&[], &defaut()).unwrap()), None);
    assert_eq!(
        exacte(&moyenne_generale(&[mat(1, None, 1)], &defaut()).unwrap()),
        None
    );
    // Une matière à moyenne 0 est une moyenne (0 ≠ « pas de moyenne »).
    assert_eq!(
        exacte(&moyenne_generale(&[mat(1, Some(Rat::ZERO), 1)], &defaut()).unwrap()),
        Some(Rat::ZERO)
    );
}

// ── Moyenne annuelle ───────────────────────────────────────────────────────────────

fn per(n: u128, m: Option<Rat>, poids: i64) -> PeriodeMoyenne {
    PeriodeMoyenne {
        periode_id: id(n),
        moyenne: m,
        poids_m: poids * 1000,
    }
}

#[test]
fn annuelle_moyenne_des_periodes_poids_egaux() {
    let m = moyenne_annuelle_periodes(
        &[
            per(1, Some(Rat::entier(10)), 1),
            per(2, Some(Rat::entier(14)), 1),
            per(3, Some(Rat::entier(15)), 1),
        ],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&m), Some(rat(13, 1)));
}

#[test]
fn annuelle_ignore_la_periode_sans_moyenne_et_pese_les_poids() {
    let m = moyenne_annuelle_periodes(
        &[
            per(1, Some(Rat::entier(10)), 1),
            per(2, None, 1),
            per(3, Some(Rat::entier(16)), 2),
        ],
        &defaut(),
    )
    .unwrap();
    assert_eq!(exacte(&m), Some(Rat::entier(14)));
    assert!(
        !m.composantes
            .iter()
            .find(|c| c.niveau == Niveau::Periode(id(2)))
            .unwrap()
            .compte
    );
    // Aucune période : aucune moyenne.
    assert_eq!(
        exacte(&moyenne_annuelle_periodes(&[per(1, None, 1)], &defaut()).unwrap()),
        None
    );
}

#[test]
fn les_deux_methodes_annuelles_divergent_quand_les_periodes_sont_inegales() {
    // T1 : une note à 20 ; T2 : quatre notes à 10. Moyenne des périodes = 15 ; toutes les
    // évaluations = (20 + 4×10) / 5 = 12.
    let t1 = vec![note(1, 20, 1, 20.0)];
    let t2: Vec<EntreeCalcul> = (2..=5).map(|n| note(n, 20, 1, 10.0)).collect();
    let r = defaut();
    let m1 = moyenne_cours(&t1, &r).unwrap().exacte;
    let m2 = moyenne_cours(&t2, &r).unwrap().exacte;
    let par_periodes = moyenne_annuelle_periodes(&[per(1, m1, 1), per(2, m2, 1)], &r).unwrap();
    assert_eq!(exacte(&par_periodes), Some(Rat::entier(15)));
    let toutes: Vec<EntreeCalcul> = t1.into_iter().chain(t2).collect();
    let par_evals = moyenne_annuelle_evaluations(&toutes, &[], &r).unwrap();
    assert_eq!(exacte(&par_evals), Some(Rat::entier(12)));
}

#[test]
fn arrondi_de_periode_avant_l_annuelle() {
    let periodes = [
        per(1, Some(rat(1004, 100)), 1),
        per(2, Some(rat(1006, 100)), 1),
    ];
    let mut r = defaut();
    assert_eq!(
        exacte(&moyenne_annuelle_periodes(&periodes, &r).unwrap()),
        Some(rat(1005, 100))
    );
    r.arrondis.moyenne_periode = Some(ArrondiRegle::new(Precision::Unite, ModeArrondi::DemiHaut));
    assert_eq!(
        exacte(&moyenne_annuelle_periodes(&periodes, &r).unwrap()),
        Some(Rat::entier(10))
    );
}

// ── Statistiques ───────────────────────────────────────────────────────────────────

#[test]
fn statistiques_moyenne_min_max_mediane() {
    let v: Vec<Rat> = [8, 12, 14, 9, 17].iter().map(|n| Rat::entier(*n)).collect();
    let s = statistiques(&v, &defaut()).unwrap().unwrap();
    assert_eq!(s.effectif, 5);
    assert_eq!(s.moyenne, rat(60, 5));
    assert_eq!(
        (s.min, s.max, s.mediane),
        (Rat::entier(8), Rat::entier(17), Rat::entier(12))
    );
}

#[test]
fn mediane_d_un_effectif_pair_selon_le_reglage() {
    let v: Vec<Rat> = [10, 12, 14, 20].iter().map(|n| Rat::entier(*n)).collect();
    let mut r = defaut();
    assert_eq!(
        statistiques(&v, &r).unwrap().unwrap().mediane,
        Rat::entier(13)
    );
    r.mediane = MedianePaire::Inferieure;
    assert_eq!(
        statistiques(&v, &r).unwrap().unwrap().mediane,
        Rat::entier(12)
    );
    r.mediane = MedianePaire::Superieure;
    assert_eq!(
        statistiques(&v, &r).unwrap().unwrap().mediane,
        Rat::entier(14)
    );
}

#[test]
fn mediane_avec_egalites() {
    let v: Vec<Rat> = [12, 12, 12, 8, 15, 12]
        .iter()
        .map(|n| Rat::entier(*n))
        .collect();
    let s = statistiques(&v, &defaut()).unwrap().unwrap();
    assert_eq!(s.mediane, Rat::entier(12));
    assert_eq!(s.min, Rat::entier(8));
    assert_eq!(s.max, Rat::entier(15));
    // Toutes égales.
    let v = vec![Rat::entier(10); 7];
    let s = statistiques(&v, &defaut()).unwrap().unwrap();
    assert_eq!(
        (s.moyenne, s.min, s.max, s.mediane),
        (
            Rat::entier(10),
            Rat::entier(10),
            Rat::entier(10),
            Rat::entier(10)
        )
    );
}

#[test]
fn statistiques_d_un_seul_eleve_et_d_aucun() {
    let s = statistiques(&[Rat::entier(9)], &defaut()).unwrap().unwrap();
    assert_eq!(
        (s.effectif, s.moyenne, s.mediane),
        (1, Rat::entier(9), Rat::entier(9))
    );
    assert_eq!(statistiques(&[], &defaut()).unwrap(), None);
}

#[test]
fn la_moyenne_de_classe_est_la_moyenne_des_moyennes_pas_des_notes() {
    // Élève A : 1 note à 20 ; élève B : 4 notes à 10. Moyenne des moyennes = 15 ;
    // moyenne de toutes les notes = 12 (non retenue).
    let moyennes = [Rat::entier(20), Rat::entier(10)];
    assert_eq!(
        statistiques(&moyennes, &defaut()).unwrap().unwrap().moyenne,
        Rat::entier(15)
    );
}

#[test]
fn seuil_de_confidentialite_cinq_eleves() {
    let r = defaut();
    for n in 1..=4usize {
        let v: Vec<Rat> = (0..n).map(|i| Rat::entier(10 + i as i128)).collect();
        let s = statistiques(&v, &r).unwrap().unwrap();
        assert!(
            s.pour_famille(&r).is_none(),
            "{n} élèves : statistiques masquées"
        );
    }
    let v: Vec<Rat> = (0..5).map(|i| Rat::entier(10 + i)).collect();
    let s = statistiques(&v, &r).unwrap().unwrap();
    let f = s.pour_famille(&r).expect("5 élèves : visibles");
    assert_eq!(f.effectif(), 5);
    let mut strict = defaut();
    strict.seuil_effectif_stats = 10;
    assert!(s.pour_famille(&strict).is_none());
}

/// M6 : plancher dur de 3, quel que soit le réglage ; un réglage à 0 ou 1 ne montre pas un groupe de 2.
#[test]
fn le_seuil_des_statistiques_familles_ne_descend_jamais_sous_trois() {
    let mut r = defaut();
    for seuil in [0usize, 1, 2, 3] {
        r.seuil_effectif_stats = seuil;
        let deux = statistiques(&[Rat::entier(8), Rat::entier(12)], &r).unwrap().unwrap();
        assert!(deux.pour_famille(&r).is_none(), "seuil {seuil} : 2 élèves masqués");
        let trois = statistiques(&[Rat::entier(8), Rat::entier(12), Rat::entier(10)], &r)
            .unwrap()
            .unwrap();
        assert!(trois.pour_famille(&r).is_some(), "seuil {seuil} : 3 élèves visibles");
    }
}

/// M6 : réglages `stats_familles` et `mediane_familles` respectés ; cas min = max.
#[test]
fn statistiques_familles_respectent_les_reglages_et_le_cas_min_egal_max() {
    let mut r = defaut();
    let tous_egaux: Vec<Rat> = (0..5).map(|_| Rat::entier(10)).collect();
    let s = statistiques(&tous_egaux, &r).unwrap().unwrap();
    assert_eq!((s.min, s.max, s.moyenne, s.mediane), (Rat::entier(10), Rat::entier(10), Rat::entier(10), Rat::entier(10)));
    let f = s.pour_famille(&r).expect("min = max, effectif suffisant : affichable");
    assert_eq!((f.min(), f.max(), f.moyenne()), (Rat::entier(10), Rat::entier(10), Rat::entier(10)));
    // La médiane n'est pas montrée par défaut.
    assert_eq!(f.mediane(), None);
    r.mediane_familles = true;
    assert_eq!(s.pour_famille(&r).unwrap().mediane(), Some(Rat::entier(10)));
    // Le réglage peut couper toute statistique.
    r.stats_familles = false;
    assert!(s.pour_famille(&r).is_none());
    // Même min = max, un groupe sous le plancher reste masqué.
    r.stats_familles = true;
    let deux = statistiques(&[Rat::entier(10), Rat::entier(10)], &r).unwrap().unwrap();
    assert!(deux.pour_famille(&r).is_none());
}

#[test]
fn statistiques_affichees_arrondies_selon_la_regle() {
    let s = statistiques(&[rat(10, 3), rat(11, 3), Rat::entier(4)], &defaut())
        .unwrap()
        .unwrap();
    let a = s.affichees(&defaut()).unwrap();
    assert_eq!(a.effectif, 3);
    assert_eq!(a.min_m, 3_330);
    assert_eq!(a.mediane_m, 3_670);
    assert_eq!(a.max_m, 4_000);
}

#[test]
fn statistiques_d_une_evaluation_ecartent_les_statuts() {
    let r = [
        (StatutCalcul::Note, Some(10_000)),
        (StatutCalcul::Note, Some(5_000)),
        (StatutCalcul::Absent, None),
        (StatutCalcul::Dispense, None),
        (StatutCalcul::NonRendu, None),
    ];
    let e10 = EvalCalcul::simple("DS", 10);
    let s = statistiques_evaluation(&e10, &r, &defaut()).unwrap().unwrap();
    assert_eq!(s.effectif, 2);
    assert_eq!(s.moyenne, Rat::entier(15)); // (20 + 10) / 2
    assert_eq!(s.min, Rat::entier(10));
    assert_eq!(
        statistiques_evaluation(&EvalCalcul::simple("DS", 20), &[(StatutCalcul::Absent, None)], &defaut())
            .unwrap(),
        None
    );
    let mut sans_bareme = EvalCalcul::simple("DS", 10);
    sans_bareme.bareme_m = 0;
    assert_eq!(statistiques_evaluation(&sans_bareme, &r, &defaut()).unwrap(), None);
}

/// M7 : mêmes règles de classement que la moyenne. 10, 10, 10, 10 et 23 sur 20 (hors barème non autorisé) :
/// la note de 23 est écartée, comme dans la moyenne ; moyenne 10 et maximum 10.
#[test]
fn statistiques_d_une_evaluation_utilisent_le_meme_classement_que_la_moyenne() {
    let e = EvalCalcul::simple("DS", 20);
    let res: Vec<(StatutCalcul, Option<i64>)> = [10_000, 10_000, 10_000, 10_000, 23_000]
        .iter()
        .map(|v| (StatutCalcul::Note, Some(*v)))
        .collect();
    let s = statistiques_evaluation(&e, &res, &defaut()).unwrap().unwrap();
    assert_eq!(s.effectif, 4);
    assert_eq!(s.moyenne, Rat::entier(10));
    assert_eq!(s.max, Rat::entier(10));
    // La moyenne de l'élève à 23 donne la même exclusion : cohérence avec `moyenne_cours`.
    let m = moyenne_cours(&[EntreeCalcul::note(e.clone(), 23.0)], &defaut()).unwrap();
    assert_eq!(m.exacte, None);
    // Hors barème AUTORISÉ : la note entre dans la statistique.
    let mut ok = e.clone();
    ok.hors_bareme_autorise = true;
    assert_eq!(statistiques_evaluation(&ok, &res, &defaut()).unwrap().unwrap().max, Rat::entier(23));
    // Évaluation annulée : aucune statistique.
    let mut annulee = e.clone();
    annulee.annulee = true;
    assert_eq!(statistiques_evaluation(&annulee, &res, &defaut()).unwrap(), None);
    // Non rendu : exclu par défaut, compté 0 si l'établissement le décide (comme la moyenne).
    let avec_nr = [(StatutCalcul::Note, Some(10_000)), (StatutCalcul::NonRendu, None)];
    assert_eq!(statistiques_evaluation(&e, &avec_nr, &defaut()).unwrap().unwrap().effectif, 1);
    let mut zero = defaut();
    zero.non_rendu = TraitementNonRendu::Zero;
    let s = statistiques_evaluation(&e, &avec_nr, &zero).unwrap().unwrap();
    assert_eq!((s.effectif, s.moyenne), (2, Rat::entier(5)));
}

/// F1 : une ligne de bonus sans base n'est pas dite « comptée ».
#[test]
fn un_bonus_sans_base_est_etiquete_non_compte() {
    let mut r = defaut();
    r.bonus = TraitementBonus::PointsAuDessusDe { seuil_m: 10_000 };
    let b = EvalCalcul::simple("Bonus", 20).avec_comptage(Comptage::Bonus);
    let m = moyenne_cours(&[EntreeCalcul::note(b, 18.0)], &r).unwrap();
    assert_eq!(m.exacte, None);
    assert_eq!(
        m.lignes[0].effet,
        Effet::NonComptee(RaisonNonComptee::BonusSansBase)
    );
    assert!(m.lignes[0].texte_fr(20_000).contains("aucune autre évaluation"));
}

// ── Validation de saisie ───────────────────────────────────────────────────────────

#[test]
fn saisie_hors_bareme_refusee_sauf_derogation() {
    use ErreurSaisieMoteur as E;
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(23_000), 20_000, false, 10),
        Err(E::HorsBareme)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(23_000), 20_000, true, 10),
        Ok(())
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(20_000), 20_000, false, 10),
        Ok(())
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(-10), 20_000, false, 10),
        Err(E::Negative)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, None, 20_000, false, 10),
        Err(E::Incoherent)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Absent, Some(1000), 20_000, false, 10),
        Err(E::Incoherent)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Absent, None, 20_000, false, 10),
        Ok(())
    );
    assert_eq!(
        valider_saisie(StatutCalcul::NonConcerne, None, 20_000, false, 10),
        Err(E::StatutNonSaisissable)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::EnAttente, None, 20_000, false, 10),
        Err(E::StatutNonSaisissable)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(1000), 0, false, 10),
        Err(E::Bareme)
    );
}

#[test]
fn pas_de_saisie_demi_point_refuse_13_3() {
    use ErreurSaisieMoteur as E;
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(13_300), 20_000, false, 500),
        Err(E::HorsPas)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(13_500), 20_000, false, 500),
        Ok(())
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(13_005), 20_000, false, 10),
        Err(E::HorsPas)
    );
    assert_eq!(
        valider_saisie(StatutCalcul::Note, Some(13_000), 20_000, false, 10),
        Ok(())
    );
}

// ── Propriétés : ordre des saisies, monotonie ──────────────────────────────────────

/// Mélange déterministe (congruentiel) : aucune dépendance à un générateur aléatoire.
fn melange<T: Clone>(v: &[T], graine: u64) -> Vec<T> {
    let mut v = v.to_vec();
    let mut s = graine;
    for i in (1..v.len()).rev() {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (s >> 33) as usize % (i + 1);
        v.swap(i, j);
    }
    v
}

fn jeu_riche() -> Vec<EntreeCalcul> {
    vec![
        note(1, 20, 2, 14.0),
        note(2, 10, 1, 7.5),
        note(3, 40, 1, 27.0),
        EntreeCalcul::statut(ev(4, "Abs", 20, 1), StatutCalcul::Absent),
        EntreeCalcul::statut(ev(5, "NR", 20, 1), StatutCalcul::NonRendu),
        si_ameliore(6, 19.0),
        si_ameliore(7, 11.0),
        si_ameliore(8, 3.0),
        EntreeCalcul::note(
            ev(9, "Diag", 20, 1).avec_comptage(Comptage::NeComptePas),
            1.0,
        ),
        EntreeCalcul::note(ev(10, "Bonus", 20, 1).avec_comptage(Comptage::Bonus), 17.0),
        en_sous_matiere(note(11, 20, 1, 9.0), 100),
        en_sous_matiere(note(12, 20, 3, 16.0), 200),
    ]
}

#[test]
fn la_moyenne_ne_depend_pas_de_l_ordre_des_saisies() {
    let jeu = jeu_riche();
    let mut regles = vec![defaut()];
    let mut r = defaut();
    r.ponderation = PonderationNotes::PointsCumules;
    r.bonus = TraitementBonus::PointsAuDessusDe { seuil_m: 10_000 };
    r.non_rendu = TraitementNonRendu::Zero;
    regles.push(r);
    let mut r = defaut();
    r.sous_matieres = MethodeSousMatieres::ToutesLesNotes;
    r.arrondis.note_ramenee = Some(ArrondiRegle::new(Precision::Dixieme, ModeArrondi::DemiPair));
    regles.push(r);
    for r in &regles {
        let ref_c = moyenne_cours(&jeu, r).unwrap().exacte;
        let ref_m = moyenne_matiere(&jeu, &[], r).unwrap().exacte;
        for graine in 0..40u64 {
            let m = melange(&jeu, graine);
            assert_eq!(
                moyenne_cours(&m, r).unwrap().exacte,
                ref_c,
                "cours, graine {graine}"
            );
            assert_eq!(
                moyenne_matiere(&m, &[], r).unwrap().exacte,
                ref_m,
                "matière, graine {graine}"
            );
        }
    }
}

#[test]
fn la_moyenne_generale_ne_depend_pas_de_l_ordre_des_matieres() {
    let matieres = vec![
        mat(1, Some(rat(1234, 100)), 3),
        mat(2, Some(rat(777, 100)), 2),
        mat(3, None, 1),
        mat(4, Some(rat(19, 1)), 1),
    ];
    let reference = moyenne_generale(&matieres, &defaut()).unwrap().exacte;
    for graine in 0..30u64 {
        assert_eq!(
            moyenne_generale(&melange(&matieres, graine), &defaut())
                .unwrap()
                .exacte,
            reference
        );
    }
}

#[test]
fn la_moyenne_reste_entre_la_plus_petite_et_la_plus_grande_note_ramenee() {
    // Propriété d'encadrement : 100 jeux déterministes.
    for graine in 0..100u64 {
        let n = 1 + (graine % 7) as u128;
        let mut entrees = Vec::new();
        let mut mini = Rat::entier(i128::MAX / 4);
        let mut maxi = Rat::ZERO;
        let mut s = graine + 1;
        for i in 0..n {
            s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let bareme = [5, 10, 20, 40][(s >> 40) as usize % 4];
            let valeur = ((s >> 20) % (bareme as u64 * 100 + 1)) as f64 / 100.0;
            let coef = 1 + ((s >> 50) % 4) as i64;
            let e = note(i + 1, bareme, coef, valeur);
            let ramenee = Rat::milliemes(e.valeur_m.unwrap())
                .multiplier(Rat::entier(20))
                .unwrap()
                .diviser(Rat::entier(i128::from(bareme)))
                .unwrap();
            mini = mini.min(ramenee);
            if ramenee > maxi {
                maxi = ramenee;
            }
            entrees.push(e);
        }
        let m = exacte(&moyenne_cours(&entrees, &defaut()).unwrap()).unwrap();
        assert!(m >= mini && m <= maxi, "graine {graine}");
    }
}

#[test]
fn debordement_remonte_en_erreur() {
    // Barèmes premiers entre eux et nombreux : le dénominateur grandit ; on doit obtenir un
    // résultat ou `Debordement`, jamais une panique.
    let premiers = [
        7_001, 7_011, 7_013, 7_019, 7_027, 7_039, 7_043, 7_057, 7_069, 7_079, 7_103, 7_109,
    ];
    let entrees: Vec<EntreeCalcul> = premiers
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let mut e = ev(i as u128 + 1, "E", 1, 1);
            e.bareme_m = *b;
            EntreeCalcul {
                evaluation: e,
                statut: StatutCalcul::Note,
                valeur_m: Some(b - 1),
            }
        })
        .collect();
    let r = moyenne_cours(&entrees, &defaut());
    assert!(matches!(r, Ok(_) | Err(ErreurCalcul::Debordement)));
}

#[test]
fn conversion_depuis_le_statut_historique() {
    use crate::evaluation::StatutResultat as S;
    assert_eq!(StatutCalcul::from(S::Note), StatutCalcul::Note);
    assert_eq!(StatutCalcul::from(S::ARattraper), StatutCalcul::ARattraper);
    assert_eq!(StatutCalcul::from(S::NonNote), StatutCalcul::NonNote);
}

#[test]
fn coherence_avec_le_moteur_historique_sur_le_cas_nominal() {
    // Le nouveau moteur et celui de V0 donnent la même moyenne sur les cas que V0 sait faire.
    use crate::evaluation::{
        moyenne_matiere as ancien, ReglesCalcul, ResultatCalcul, StatutResultat,
    };
    let anciens = vec![
        ResultatCalcul {
            statut: StatutResultat::Note,
            valeur: 14.0,
            sur: 20.0,
            coef: 2.0,
        },
        ResultatCalcul {
            statut: StatutResultat::Note,
            valeur: 7.0,
            sur: 10.0,
            coef: 1.0,
        },
        ResultatCalcul {
            statut: StatutResultat::Absent,
            valeur: 0.0,
            sur: 20.0,
            coef: 1.0,
        },
        ResultatCalcul {
            statut: StatutResultat::Note,
            valeur: 11.0,
            sur: 20.0,
            coef: 2.0,
        },
    ];
    let ancien_moy = ancien(&anciens, &ReglesCalcul::default()).unwrap();
    let nouveaux = vec![
        note(1, 20, 2, 14.0),
        note(2, 10, 1, 7.0),
        EntreeCalcul::statut(ev(3, "A", 20, 1), StatutCalcul::Absent),
        note(4, 20, 2, 11.0),
    ];
    let n = moyenne_cours(&nouveaux, &defaut()).unwrap();
    assert_eq!(
        f64::from(i32::try_from(aff(&n).unwrap()).unwrap()) / 1000.0,
        ancien_moy.arrondie(crate::evaluation::Arrondi::Centieme)
    );
}
