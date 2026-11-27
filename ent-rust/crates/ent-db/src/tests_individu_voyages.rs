//! Transverse « la deuxième personne fondée sur la personne physique », lot 2 (migration 659) : les voyages.
//! Contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Faible 9 de la contre-revue V3 : un même individu à deux comptes n'était pas reconnu. Ici : la règle 3 voit le lien
//! familial porté par une autre fiche de l'individu (avec ou sans compte), la seconde validation et l'annulation au-dessus
//! du seuil refusent un second compte du demandeur, le contournement refuse un second compte de la demandeuse, le
//! bordereau n'est pas remis par un second compte de la personne qui l'a saisi. Fichier séparé de `tests_voyages_v3.rs`.

use crate::ref_commun::Acteur;
use crate::service::{EntDbError, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::tests_individu::{fiche_du_compte, fiche_sans_compte};
use crate::tests_voyages_v2a::{suivi, Suivi};
use crate::voyages_argent_v3::{
    AideSaisie, StatutDemande, MSG_REGLE3, MSG_SECONDE_ANNULATION, MSG_SECONDE_VALIDATION,
};
use chrono::{DateTime, Utc};
use ent_auth::Role;
use uuid::Uuid;

fn instant_fixe() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2031-03-12T11:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn aide(p: Uuid, compte: Uuid, montant: &str) -> AideSaisie {
    AideSaisie {
        participant_id: p,
        compte_id: compte,
        source: "apel".into(),
        montant: montant.into(),
        reference: Some("DEC-IND".into()),
        motif: "Décision de la commission".into(),
        cle: format!("aide-{}", Uuid::new_v4().simple()),
    }
}

fn refus_msg(e: &EntDbError, m: &str) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Interdit, x) if *x == m)
}

async fn nouveau(x: &Suivi, role: &str) -> Uuid {
    crate::tests_voyages_v0a::compte(&x.c.ent, x.c.etab, role).await
}

fn acteur(x: &Suivi, u: Uuid, role: Role) -> Acteur<'static> {
    x.c.a(u, role)
}

/// La direction relie deux fiches.
async fn relier(x: &Suivi, a: Uuid, b: Uuid) {
    x.c.ent
        .individu_relier_for_actor(x.c.etab, x.c.dir, "direction", a, b, "deux_comptes", false)
        .await
        .expect("relier");
}

async fn decideur_interdit(x: &Suivi, u: Uuid, compte: Uuid, eleve: Uuid) -> bool {
    let mut tx = TenantTx::begin(x.c.ent.pool(), x.c.etab).await.unwrap();
    sqlx::query_scalar("SELECT ent_sortie_argent_decideur_interdit($1, $2, $3, ARRAY[$4]::uuid[])")
        .bind(x.c.etab)
        .bind(u)
        .bind(compte)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .unwrap()
}

/// Règle 3 : la comptable dont l'adresse personnelle est le compte d'un parent payeur de l'élève n'aide plus cet élève
/// depuis son compte professionnel, dès que les deux fiches sont reliées.
#[tokio::test]
async fn regle3_voit_le_second_compte_parent_de_la_comptable() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let compta = x.compta();
    // Avant le lien : deux individus, l'aide passe (sous le seuil).
    assert!(!decideur_interdit(&x, compta.user_id, x.compte_c, x.c.eleves[1]).await);
    let d = ent
        .voyage_aide_accorder_a(&compta, x.s.id, aide(x.p1, x.compte_c, "10"), instant_fixe())
        .await
        .unwrap();
    assert_eq!(d.statut, StatutDemande::Appliquee);
    // La fiche de la comptable et celle du parent C (payeur de l'élève 1) sont reliées.
    let f_compta = fiche_du_compte(ent, x.c.etab, compta.user_id).await;
    let f_parent = fiche_du_compte(ent, x.c.etab, x.parent_c).await;
    relier(&x, f_compta, f_parent).await;
    assert!(decideur_interdit(&x, compta.user_id, x.compte_c, x.c.eleves[1]).await, "la base voit l'individu");
    // Ni par le compte payeur, ni par l'élève : refus de la règle 3.
    assert!(decideur_interdit(&x, compta.user_id, Uuid::nil(), x.c.eleves[1]).await, "lien familial de l'autre fiche");
    let e = ent
        .voyage_aide_accorder_a(&compta, x.s.id, aide(x.p1, x.compte_c, "10"), instant_fixe())
        .await
        .unwrap_err();
    assert!(refus_msg(&e, MSG_REGLE3), "{e:?}");
    // L'élève d'un autre foyer reste aidable par elle.
    let d = ent
        .voyage_aide_accorder_a(&compta, x.s.id, aide(x.p0, x.compte_a, "10"), instant_fixe())
        .await
        .unwrap();
    assert_eq!(d.statut, StatutDemande::Appliquee);
    // Lien levé : de nouveau deux individus.
    let lien = ent
        .individu_liens_for_actor(x.c.etab, x.c.dir, "direction")
        .await
        .unwrap()[0]
        .id;
    let dir2 = nouveau(&x, "direction").await;
    ent.individu_lever_for_actor(x.c.etab, dir2, "direction", lien, "erreur").await.unwrap();
    assert!(!decideur_interdit(&x, compta.user_id, x.compte_c, x.c.eleves[1]).await);
}

/// Règle 3 : une fiche parent **sans compte** (saisie par le secrétariat) reliée à l'agent suffit.
#[tokio::test]
async fn regle3_voit_la_fiche_parent_sans_compte_reliee_a_l_agent() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let compta2 = nouveau(&x, "compta").await;
    let a2 = acteur(&x, compta2, Role::Compta);
    let f_sans = fiche_sans_compte(ent, x.c.etab, "Mère sans compte", None).await;
    {
        let mut tx = TenantTx::begin(ent.pool(), x.c.etab).await.unwrap();
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien) VALUES ($1, $2, $3, $4, 'mere')")
            .bind(Uuid::new_v4())
            .bind(x.c.etab)
            .bind(x.c.eleves[0])
            .bind(f_sans)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert!(!decideur_interdit(&x, compta2, x.compte_a, x.c.eleves[0]).await);
    let f_compta2 = fiche_du_compte(ent, x.c.etab, compta2).await;
    relier(&x, f_sans, f_compta2).await;
    assert!(decideur_interdit(&x, compta2, Uuid::nil(), x.c.eleves[0]).await);
    let e = ent
        .voyage_aide_accorder_a(&a2, x.s.id, aide(x.p0, x.compte_a, "10"), instant_fixe())
        .await
        .unwrap_err();
    assert!(refus_msg(&e, MSG_REGLE3), "{e:?}");
}

/// Seconde validation et annulation au-dessus du seuil : un second compte du demandeur ne vaut pas une seconde personne,
/// dans l'application comme dans la base.
#[tokio::test]
async fn seconde_validation_et_annulation_refusent_un_second_compte_du_demandeur() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let compta = x.compta();
    // Un second compte « comptabilité » du même humain (ancien compte, autre adresse), relié par la direction.
    let bis = nouveau(&x, "compta").await;
    let a_bis = acteur(&x, bis, Role::Compta);
    let f1 = fiche_du_compte(ent, x.c.etab, compta.user_id).await;
    let f2 = fiche_du_compte(ent, x.c.etab, bis).await;
    relier(&x, f1, f2).await;
    // 200 € : au-dessus du seuil par défaut (150 €), la demande attend une seconde personne.
    let d = ent
        .voyage_aide_accorder_a(&compta, x.s.id, aide(x.p0, x.compte_a, "200"), instant_fixe())
        .await
        .unwrap();
    assert_eq!(d.statut, StatutDemande::EnAttente);
    let e = ent
        .voyage_demande_valider_a(&a_bis, x.s.id, d.demande_id, instant_fixe())
        .await
        .unwrap_err();
    assert!(refus_msg(&e, MSG_SECONDE_VALIDATION), "{e:?}");
    // La base refuse aussi l'issue « appliquée » posée par le second compte (avant toute autre vérification).
    {
        let mut tx = TenantTx::begin(ent.pool(), x.c.etab).await.unwrap();
        let e = sqlx::query(
            "INSERT INTO ent_sortie_argent_issue (id, etablissement_id, demande_id, issue, aide_id, par_user_id, le)
             VALUES (gen_random_uuid(), $1, $2, 'appliquee', gen_random_uuid(), $3, now())",
        )
        .bind(x.c.etab)
        .bind(d.demande_id)
        .bind(bis)
        .execute(&mut *tx)
        .await
        .unwrap_err();
        assert!(format!("{e}").contains("une seconde personne valide"), "{e}");
    }
    // Une autre personne valide.
    let autre = acteur(&x, nouveau(&x, "compta").await, Role::Compta);
    let v = ent
        .voyage_demande_valider_a(&autre, x.s.id, d.demande_id, instant_fixe())
        .await
        .unwrap();
    let aide_id = v.aide.expect("aide appliquée").id;
    // Annuler cette aide au-dessus du seuil : ni la demandeuse, ni son second compte.
    for qui in [&compta, &a_bis] {
        let e = ent
            .voyage_aide_annuler_a(qui, x.s.id, aide_id, "Erreur de saisie", instant_fixe())
            .await
            .unwrap_err();
        assert!(refus_msg(&e, MSG_SECONDE_ANNULATION), "{e:?}");
    }
}

/// Contournement : un second compte de la demandeuse ne décide pas de sa demande.
#[tokio::test]
async fn contournement_un_second_compte_de_la_demandeuse_ne_decide_pas() {
    use crate::voyages_contournement::{DemandeContournement, MSG_DEMANDEUSE_DECIDE};
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    // La demandeuse : comptable, parent de l'élève 1 (liée à la sortie).
    let lie = nouveau(&x, "compta").await;
    {
        let mut tx = TenantTx::begin(ent.pool(), x.c.etab).await.unwrap();
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, lien) VALUES ($1, $2, $3, $4, 'pere')")
            .bind(Uuid::new_v4())
            .bind(x.c.etab)
            .bind(x.c.eleves[1])
            .bind(lie)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let a_lie = acteur(&x, lie, Role::Compta);
    let presta = {
        use crate::voyages_prestataires_repo::PrestataireSaisie;
        ent.prestataire_creer(
            &x.sec(),
            PrestataireSaisie {
                raison_sociale: format!("Cars individu {}", Uuid::new_v4().simple()),
                siren: None,
                telephone: None,
                categories: vec!["transporteur".into()],
                adresses: vec![format!("ind-{}@exemple-test.fr", Uuid::new_v4().simple())],
                version: None,
                actif: None,
            },
        )
        .await
        .unwrap()
        .id
    };
    let t0: DateTime<Utc> = sqlx::query_scalar("SELECT now()").fetch_one(ent.pool_verif()).await.unwrap();
    let c = ent
        .voyage_contournement_demander_a(
            &a_lie,
            x.s.id,
            DemandeContournement {
                action: "acompte_creer".into(),
                motif: "Seule comptable présente".into(),
                cible_id: None,
                montant_max: Some("100".into()),
                prestataire_id: Some(presta),
            },
            t0,
        )
        .await
        .unwrap();
    // Son second compte, relié : refusé comme la demandeuse elle-même.
    let bis = nouveau(&x, "compta").await;
    let f1 = fiche_du_compte(ent, x.c.etab, lie).await;
    let f2 = fiche_du_compte(ent, x.c.etab, bis).await;
    relier(&x, f1, f2).await;
    let e = ent
        .voyage_contournement_decider_a(&acteur(&x, bis, Role::Compta), x.s.id, c.id, true, None, t0)
        .await
        .unwrap_err();
    assert!(refus_msg(&e, MSG_DEMANDEUSE_DECIDE), "{e:?}");
    // Une personne libre décide.
    let ok = ent
        .voyage_contournement_decider_a(&x.compta(), x.s.id, c.id, true, None, t0)
        .await
        .unwrap();
    assert_eq!(ok.statut, "autorise");
    // L'autorisation ne vaut que si l'autorisatrice est un autre individu que la demandeuse (fonction de la base).
    let mut tx = TenantTx::begin(ent.pool(), x.c.etab).await.unwrap();
    let ouvert: bool = sqlx::query_scalar(
        "SELECT ent_sortie_contournement_ouvert($1, $2, $3, $4, 'acompte_creer', NULL, 100, $5, now())",
    )
    .bind(x.c.etab)
    .bind(c.id)
    .bind(x.s.id)
    .bind(lie)
    .bind(presta)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(ouvert, "autorisée par une personne libre : ouverte");
    drop(tx);
    // Relier ensuite la décideuse à la demandeuse ferme l'autorisation.
    let f3 = fiche_du_compte(ent, x.c.etab, x.c.compta).await;
    relier(&x, f2, f3).await;
    let mut tx = TenantTx::begin(ent.pool(), x.c.etab).await.unwrap();
    let ouvert: bool = sqlx::query_scalar(
        "SELECT ent_sortie_contournement_ouvert($1, $2, $3, $4, 'acompte_creer', NULL, 100, $5, now())",
    )
    .bind(x.c.etab)
    .bind(c.id)
    .bind(x.s.id)
    .bind(lie)
    .bind(presta)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(!ouvert, "autorisatrice et demandeuse : le même individu");
}

/// Bordereau : la personne qui l'a saisi ne le reçoit pas, ni par un second compte.
#[tokio::test]
async fn bordereau_pas_remis_par_un_second_compte_de_la_saisie() {
    use crate::voyages_argent_flux::{BordereauLigneSaisie, BordereauSaisie};
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let bv = ent
        .voyage_bordereau_saisir(
            &x.porteur(),
            x.s.id,
            BordereauSaisie {
                nb_cheques: 1,
                total: "10".into(),
                lignes: vec![BordereauLigneSaisie { participant_id: x.p0, numero_cheque: "901".into(), montant: "10".into() }],
            },
        )
        .await
        .unwrap();
    let ligne = bv.lignes[0].id;
    // Le porteur a aussi un compte au secrétariat (autre adresse), relié.
    let sec2 = nouveau(&x, "administratif").await;
    let f1 = fiche_du_compte(ent, x.c.etab, x.c.prof).await;
    let f2 = fiche_du_compte(ent, x.c.etab, sec2).await;
    relier(&x, f1, f2).await;
    let e = ent
        .voyage_bordereau_remettre(&acteur(&x, sec2, Role::Administratif), x.s.id, bv.id, vec![ligne], None)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Interdit, _)), "{e:?}");
    ent.voyage_bordereau_remettre(&x.sec(), x.s.id, bv.id, vec![ligne], None).await.unwrap();
}
