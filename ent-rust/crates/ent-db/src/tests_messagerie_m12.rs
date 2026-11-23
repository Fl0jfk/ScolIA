//! Messagerie M12 : flux « quelque chose a changé » — aucun événement pour un non-participant, plus rien après un
//! retrait, aucun contenu. Les instants viennent de la base (règle 33 : aucune heure réelle de l'appelant).

use super::tests_messagerie_m1::{ecrit, labo};
use super::tests_messagerie_m5::fil_de_lily;
use ent_auth::Role;
use uuid::Uuid;

async fn depart(ent: &super::PgEnt, etab: Uuid) -> String {
    ent.messages.position_depart_flux(etab).await.expect("départ").encoder()
}

async fn ids(ent: &super::PgEnt, etab: Uuid, user: Uuid, r: Role, apres: &str) -> Vec<Uuid> {
    ent.flux_messagerie_for_actor(etab, user, r.as_str(), Some(apres), 0)
        .await
        .expect("flux")
        .fils
        .into_iter()
        .map(|f| f.fil_id)
        .collect()
}

#[tokio::test]
async fn m12_flux_participants_seulement_sans_contenu_et_retrait() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily, cpe) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
        cast.cpe.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Flux").await;
    let avant = depart(&ent, etab).await;
    // Rien de nouveau depuis « maintenant » : flux vide, curseur rendu.
    assert!(ids(&ent, etab, lily, Role::Parent, &avant).await.is_empty());

    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Message secret du flux").await);
    // La famille participante est avertie ; un personnel non participant ne reçoit rien (R-M2).
    assert_eq!(ids(&ent, etab, lily, Role::Parent, &avant).await, vec![fil]);
    assert!(ids(&ent, etab, cpe, Role::Cpe, &avant).await.is_empty());
    // Aucun contenu : seul un identifiant de fil et un instant sont sérialisés.
    let flux = ent.flux_messagerie_for_actor(etab, lily, Role::Parent.as_str(), Some(&avant), 0).await.unwrap();
    let json = serde_json::to_string(&flux).unwrap();
    assert!(!json.contains("secret") && !json.contains("Flux"), "{json}");
    // Sans curseur : aucun événement, seulement le curseur de départ.
    let depart = ent.flux_messagerie_for_actor(etab, lily, Role::Parent.as_str(), None, 0).await.unwrap();
    assert!(depart.fils.is_empty());
    // Le curseur rendu après lecture : plus rien (rien n'est servi deux fois).
    let apres_lecture = ent.flux_messagerie_for_actor(etab, lily, Role::Parent.as_str(), Some(&avant), 0).await.unwrap().curseur;
    assert!(ids(&ent, etab, lily, Role::Parent, &apres_lecture).await.is_empty());

    // Participant retiré : le flux se tait aussitôt (la vue est relue à chaque appel).
    sqlx::query(
        "UPDATE ent_msg_participant SET au = now(), motif_sortie = 'sortie_volontaire'
          WHERE etablissement_id = $1 AND fil_id = $2 AND cote = 'famille' AND au IS NULL",
    )
    .bind(etab)
    .bind(fil)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    assert!(ids(&ent, etab, lily, Role::Parent, &avant).await.is_empty());
}

#[tokio::test]
async fn m12_flux_message_differe_invisible_avant_remise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Flux différé").await;
    let avant = depart(&ent, etab).await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Plus tard").await);
    sqlx::query(
        "UPDATE ent_msg SET remise_prevue_le = now() + interval '1 hour'
          WHERE etablissement_id = $1 AND fil_id = $2 AND corps = 'Plus tard'",
    )
    .bind(etab)
    .bind(fil)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    assert!(ids(&ent, etab, lily, Role::Parent, &avant).await.is_empty(), "un message différé ne fait aucun bruit avant sa remise");
}
