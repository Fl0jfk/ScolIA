//! Transverse « la deuxième personne fondée sur la personne physique », lot 3 : les autres socles (migrations 2316 et
//! 2421, conversions de l'application). Contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL` :
//! ignorés en local et **en échec en CI**.
//!
//! Un second compte relié ne vaut pas une autre personne : absences des personnels (R-AP10, service et base), accès
//! d'inspection du cahier (base), conflit d'intérêts R-A18 (fiche parent reliée), délégation (fiches d'agent reliées).
//! Une garde statique vérifie qu'aucune égalité de comptes ne reste dans les refus convertis.

use crate::individu::agents_meme_individu;
use crate::service::{EntDbError, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::tests_individu::{compte, connecter, fiche_du_compte, fiche_sans_compte};
use crate::tests_voyages_v2a::suivi;
use crate::NatureConflitInteret as NatureConflit;
use crate::PgEnt;
use chrono::NaiveDate;
use domain_eleve::DeclarerAbsenceRh;
use uuid::Uuid;

fn jour(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2031, m, d).unwrap()
}

async fn agent_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    crate::personnels::agent_du_user_tx(&mut tx, etab, user)
        .await
        .unwrap()
        .expect("agent du compte")
}

/// R-AP10 : un second compte « direction » de l'absent, relié par le secrétariat, ne valide ni ne refuse son absence,
/// ni dans l'application, ni dans la base (projection, décision, valideur du circuit). Une autre personne valide.
#[tokio::test]
async fn r_ap10_un_second_compte_de_l_absent_ne_decide_pas() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (bis, _) = compte(&ent, etab, "direction").await;
    let (autre, _) = compte(&ent, etab, "direction").await;
    let (sec, _) = compte(&ent, etab, "administratif").await;
    let a = ent
        .declarer_absence_rh_for_actor(
            "direction",
            DeclarerAbsenceRh {
                etablissement_id: etab,
                user_id: dir,
                debut: jour(3, 12),
                fin: jour(3, 12),
                motif: "rdv".into(),
                actor_user_id: dir,
            },
        )
        .await
        .expect("déclaration");
    let f_dir = fiche_du_compte(&ent, etab, dir).await;
    let f_bis = fiche_du_compte(&ent, etab, bis).await;
    ent.individu_relier_for_actor(etab, sec, "administratif", f_dir, f_bis, "doublon_agent", false)
        .await
        .expect("relier");

    for e in [
        ent.valider_absence_rh_for_actor(etab, "direction", a.id, bis).await.unwrap_err(),
        ent.refuser_absence_rh_for_actor(etab, "direction", a.id, bis).await.unwrap_err(),
        ent.valider_absence_for_actor(etab, bis, "direction", a.id, &Default::default())
            .await
            .unwrap_err(),
    ] {
        assert!(matches!(e, EntDbError::Refus(RefusMetier::Interdit, _)), "{e:?}");
    }
    // En base.
    let r = sqlx::query("UPDATE ent_absence_rh SET valide_par_user_id = $2 WHERE id = $1")
        .bind(a.id)
        .bind(bis)
        .execute(ent.pool())
        .await;
    assert!(r.is_err(), "ent_absence_rh : un second compte de l'absent valide");
    let agent_bis = agent_de(&ent, etab, bis).await;
    let r = sqlx::query(
        "INSERT INTO ent_absp_decision (id, etablissement_id, absence_id, type, auteur_agent_id, auteur_user_id)
         VALUES ($1,$2,$3,'validation',$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(a.id)
    .bind(agent_bis)
    .bind(bis)
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "ent_absp_decision : un second compte de l'absent décide");
    let r = sqlx::query(
        "INSERT INTO ent_absp_circuit (id, etablissement_id, absence_id, role, agent_id, source)
         VALUES ($1,$2,$3,'valideur',$4,'regle')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(a.id)
    .bind(agent_bis)
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "ent_absp_circuit : une autre fiche d'agent de l'absent est valideur");
    // Une autre personne valide.
    let v = ent
        .valider_absence_rh_for_actor(etab, "direction", a.id, autre)
        .await
        .expect("une autre personne valide");
    assert_eq!(v.statut, "validee");
}

/// Délégation : jamais à une autre fiche d'agent du même individu (port des délégations).
#[tokio::test]
async fn delegation_pas_a_une_autre_fiche_d_agent_du_meme_individu() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (p1, _) = compte(&ent, etab, "prof").await;
    let (p2, _) = compte(&ent, etab, "prof").await;
    let (g1, g2) = (agent_de(&ent, etab, p1).await, agent_de(&ent, etab, p2).await);
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert!(agents_meme_individu(&mut tx, etab, g1, g1).await.unwrap());
    assert!(!agents_meme_individu(&mut tx, etab, g1, g2).await.unwrap());
    drop(tx);
    let (f1, f2) = (fiche_du_compte(&ent, etab, p1).await, fiche_du_compte(&ent, etab, p2).await);
    ent.individu_relier_for_actor(etab, dir, "direction", f1, f2, "doublon_agent", false)
        .await
        .unwrap();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert!(agents_meme_individu(&mut tx, etab, g1, g2).await.unwrap());
    assert!(agents_meme_individu(&mut tx, etab, g2, g1).await.unwrap());
    let d = jour(3, 12);
    let e = crate::personnels_delegations::controles_delegataire_tx(&mut tx, etab, g1, g2, d, d, d)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Invalide, _)), "{e:?}");
}

/// Cahier de textes (2421) : nul ne s'accorde un accès d'inspection depuis un autre de ses comptes, en base.
#[tokio::test]
async fn cahier_acces_d_inspection_pas_a_un_second_compte() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (bis, _) = compte(&ent, etab, "prof").await;
    let (autre, _) = compte(&ent, etab, "prof").await;
    let (f1, f2) = (fiche_du_compte(&ent, etab, dir).await, fiche_du_compte(&ent, etab, bis).await);
    ent.individu_relier_for_actor(etab, dir, "direction", f1, f2, "deux_comptes", false)
        .await
        .unwrap();
    let inserer = |beneficiaire: Uuid| {
        let ent = &ent;
        async move {
            sqlx::query(
                "INSERT INTO ent_cdt_acces_inspection (id, etablissement_id, beneficiaire_user_id, du_cahier, au_cahier,
                        valable_du, valable_au, motif, accorde_par_user_id)
                 VALUES ($1, $2, $3, DATE '2031-01-01', DATE '2031-06-30',
                         TIMESTAMPTZ '2031-03-12 10:00Z', TIMESTAMPTZ '2031-04-11 10:00Z', 'inspection', $4)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(beneficiaire)
            .bind(dir)
            .execute(ent.pool())
            .await
        }
    };
    let e = inserer(bis).await.expect_err("un accès à son second compte");
    assert!(e.to_string().contains("ne s'accorde pas à soi-même"), "{e}");
    inserer(autre).await.expect("un accès à une autre personne");
}

/// Conflit d'intérêts (R-A18) : une fiche parent **sans compte**, reliée à la fiche de l'agent, ouvre le conflit.
#[tokio::test]
async fn conflit_d_interet_voit_la_fiche_parent_reliee() {
    let Some(x) = suivi().await else { return };
    let ent = &x.c.ent;
    let etab = x.c.etab;
    let (agent, _) = compte(ent, etab, "cpe").await;
    let f_sans = fiche_sans_compte(ent, etab, "Père sans compte", None).await;
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien) VALUES ($1, $2, $3, $4, 'pere')")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(x.c.eleves[0])
            .bind(f_sans)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert_eq!(ent.conflit_interet_eleve(etab, agent, "cpe", x.c.eleves[0]).await.unwrap(), None);
    let f_agent = fiche_du_compte(ent, etab, agent).await;
    ent.individu_relier_for_actor(etab, x.c.dir, "direction", f_sans, f_agent, "fiche_sans_compte", false)
        .await
        .unwrap();
    assert_eq!(
        ent.conflit_interet_eleve(etab, agent, "cpe", x.c.eleves[0]).await.unwrap(),
        Some(NatureConflit::Responsable)
    );
    assert_eq!(ent.conflit_interet_eleve(etab, agent, "cpe", x.c.eleves[1]).await.unwrap(), None, "un autre élève");
}

/// Garde statique : les refus convertis ne comparent plus deux comptes par égalité (conception §5, lot 3).
#[test]
fn garde_statique_aucune_egalite_de_comptes_dans_les_refus_convertis() {
    let interdits: &[(&str, &str, &str)] = &[
        ("voyages_argent_v3.rs", include_str!("voyages_argent_v3.rs"), "d.seconde_validation && d.demande_par_user_id == a.user_id"),
        ("voyages_argent_v3.rs", include_str!("voyages_argent_v3.rs"), "user == decide || demande == Some(user)"),
        ("voyages_contournement.rs", include_str!("voyages_contournement.rs"), "c.demande_par_user_id == a.user_id"),
        ("voyages_argent_flux.rs", include_str!("voyages_argent_flux.rs"), "saisi_par == a.user_id"),
        ("frm_service.rs", include_str!("frm_service.rs"), "f.auteur_user_id == user"),
        ("cahier_c8_acces.rs", include_str!("cahier_c8_acces.rs"), "beneficiaire_user_id == a.user_id"),
        ("cahier_c8_acces.rs", include_str!("cahier_c8_acces.rs"), "beneficiaire == a.user_id"),
        ("cahier_c8_acces.rs", include_str!("cahier_c8_acces.rs"), "demandeur == a.user_id"),
        ("internat_sorties/groupes.rs", include_str!("internat_sorties/groupes.rs"), "g.cree_par == actor"),
        ("absences_rh.rs", include_str!("absences_rh.rs"), "actor == current.user_id"),
        ("documents_signature_signer.rs", include_str!("documents_signature_signer.rs"), ".await? == Some(a.user_id)"),
        ("personnels_delegations.rs", include_str!("personnels_delegations.rs"), "delegataire == delegant"),
    ];
    // Restent sur le compte (principe 1, ce qu'ils accordent) : la demandeuse retire sa propre demande
    // (`voyages_argent_v3.rs`, « retirée »), l'absent annule la sienne, la signataire signe la sienne.
    for (fichier, src, motif) in interdits {
        assert!(!src.contains(motif), "{fichier} compare encore deux comptes : « {motif} »");
    }
    // Absences des personnels : chaque refus R-AP10 passe par `absent_individu`, jamais par le seul compte.
    let abs = include_str!("absences_personnels.rs");
    for motif in ["valider_droit && !suis_absent", "instructeur) && !suis_absent", "let ok = !suis_absent", "if suis_absent ||"] {
        assert!(!abs.contains(motif), "absences_personnels.rs : refus encore sur le compte : « {motif} »");
    }
}
