//! Transverse « la deuxième personne fondée sur la personne physique », lot 3 (migration 2421) : la réouverture du cahier
//! clôturé est validée par un autre **individu** que le demandeur, dans l'application comme dans la base. Sous-module de
//! `tests_cahier` (fixture `Fx`, un établissement neuf par test). Aucune heure réelle.

use super::*;
use crate::tests_individu::fiche_du_compte;
use ent_types::cahier_c8::*;

#[tokio::test]
async fn individu_reouverture_pas_validee_par_un_second_compte_du_demandeur() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let annee = fx.ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    fx.ent
        .cdt_c8_cloturer(&fx.direction(), ClotureBody { annee_id: annee.id, confirmer: true })
        .await
        .unwrap();
    let motif = "Réouverture motivée par une erreur de saisie";
    fx.ent
        .cdt_c8_rouvrir(&fx.direction(), ReouvrirBody { annee_id: annee.id, motif: motif.into() })
        .await
        .unwrap();
    // Les deux comptes de direction sont reliés : un seul humain.
    let (d1, d2) = (fx.direction().user_id, fx.direction2().user_id);
    let f1 = fiche_du_compte(&fx.ent, etab, d1).await;
    let f2 = fiche_du_compte(&fx.ent, etab, d2).await;
    fx.ent
        .individu_relier_for_actor(etab, d1, "direction", f1, f2, "deux_comptes", false)
        .await
        .unwrap();
    // Application : 403.
    let e = fx
        .ent
        .cdt_c8_rouvrir_valider(&fx.direction2(), ValiderReouvertureBody { annee_id: annee.id })
        .await
        .unwrap_err();
    assert!(est_interdit(&e), "{e:?}");
    // Base (`ent_cdt_cloture_coherence` recréée par 2421) : même refus en SQL direct.
    let e = sqlx::query(
        "INSERT INTO ent_cdt_cloture (id, etablissement_id, annee_id, action, motif, par_user_id, demandeur_user_id)
         VALUES ($1, $2, $3, 'reouverture', $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee.id)
    .bind(motif)
    .bind(d2)
    .bind(d1)
    .execute(fx.ent.pool())
    .await
    .unwrap_err();
    assert!(e.to_string().contains("d'une autre personne"), "{e}");
}
