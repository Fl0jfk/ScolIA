//! Socle 4, lot A1 : droits par groupe de champs (F0 à F4, R), raisons de masquage, conflit
//! d'intérêts (R-A18), droit d'accès d'un titulaire sans les données de l'autre, interdiction de
//! remise visible de l'accueil et de la vie scolaire. Contre la base locale de test (feature `labo`).
//! Un test par règle.

use super::tests_administratif_a0::{dir, eleve_neuf, labo, lien_saisie, personne_avec_compte};
use super::*;
use crate::dossier_personnes::{ContactSaisie, InterdictionSaisie, LienResultat, PersonneSaisie};
use ent_auth::Role;
use uuid::Uuid;

const TEL_B: &str = "0611223344";
const ADRESSE_B: &str = "8 impasse du Lilas, 31000 Toulouse";
const EMAIL_B: &str = "secret-b@exemple.fr";

/// Agent du personnel (compte + fiche personne) avec un profil de base.
async fn agent(ent: &PgEnt, etab: Uuid, role: Role, nom: &str) -> (Uuid, Uuid) {
    let u = Uuid::new_v4().simple().to_string();
    let (pers, user) = personne_avec_compte(ent, etab, nom, &format!("ag-{u}@exemple.fr")).await;
    sqlx::query("UPDATE ent_membership SET role = $2 WHERE user_id = $1")
        .bind(user)
        .bind(role.as_str())
        .execute(ent.pool())
        .await
        .unwrap();
    (pers, user)
}

/// Responsable avec compte, coordonnées et lien confirmé. Retourne (personne, user, lien).
#[allow(clippy::too_many_arguments)]
async fn responsable(
    ent: &PgEnt,
    etab: Uuid,
    eleve: Uuid,
    nom: &str,
    lien: &str,
    email: &str,
    tel: &str,
    adresse: &str,
) -> (Uuid, Uuid, Uuid) {
    let (d, r) = dir_cast(ent, etab).await;
    let (pers, user) = personne_avec_compte(ent, etab, nom, email).await;
    sqlx::query("UPDATE ent_personne SET telephone = $2, adresse = $3 WHERE id = $1")
        .bind(pers)
        .bind(tel)
        .bind(adresse)
        .execute(ent.pool())
        .await
        .unwrap();
    let mut saisie = lien_saisie(Some(pers), None);
    saisie.lien = lien.into();
    let LienResultat::EnAttente { attente_id, .. } = ent
        .lier_personne_for_actor(etab, d, r, eleve, &saisie)
        .await
        .unwrap()
    else {
        panic!("attendu : lien en attente");
    };
    ent.repondre_lien_en_attente(etab, user, attente_id, true).await.unwrap();
    let lien_id = ent
        .responsables_eleve_for_actor(etab, d, r, eleve)
        .await
        .unwrap()
        .into_iter()
        .find(|x| x.personne.id == pers)
        .unwrap()
        .lien_id;
    (pers, user, lien_id)
}

/// Acteur « direction » du labo pour un établissement donné.
async fn dir_cast(ent: &PgEnt, etab: Uuid) -> (Uuid, &'static str) {
    let (u,): (Uuid,) = sqlx::query_as(
        "SELECT user_id FROM ent_membership WHERE etablissement_id = $1 AND role = 'direction' LIMIT 1",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    (u, Role::Direction.as_str())
}

/// Dossier de deux parents séparés : A (mère, payeuse à 100 %) et B (père) ; retourne les ids.
struct Dossier {
    eleve: Uuid,
    pers_a: Uuid,
    user_a: Uuid,
    lien_a: Uuid,
    pers_b: Uuid,
    lien_b: Uuid,
}

async fn dossier_separe(ent: &PgEnt, etab: Uuid, nom: &str) -> Dossier {
    let u = Uuid::new_v4().simple().to_string();
    let eleve = eleve_neuf(ent, etab, nom).await;
    let (pers_a, user_a, lien_a) = responsable(
        ent, etab, eleve, "ParentA", "mère", &format!("a-{u}@exemple.fr"), "0600000001",
        "1 rue des Tilleuls, 75001 Paris",
    )
    .await;
    let (pers_b, _user_b, lien_b) = responsable(
        ent, etab, eleve, "ParentB", "père", &format!("b-{u}-{EMAIL_B}"), TEL_B, ADRESSE_B,
    )
    .await;
    let (d, r) = dir_cast(ent, etab).await;
    ent.definir_payeurs_for_actor(etab, d, r, eleve, &[(lien_a, 100)]).await.unwrap();
    Dossier { eleve, pers_a, user_a, lien_a, pers_b, lien_b }
}

fn de<'a>(rs: &'a [Responsable], pers: Uuid) -> &'a Responsable {
    rs.iter().find(|r| r.personne.id == pers).expect("responsable absent")
}

fn groupes_fermes(r: &Responsable) -> Vec<&'static str> {
    r.masque.iter().map(|m| m.groupe.code()).collect()
}

/// Groupes F0–F4 et payeurs : chaque profil lit ce que son modèle ouvre, rien d'autre ; chaque
/// champ fermé est expliqué (raison + droit à demander).
#[tokio::test]
async fn groupes_de_champs_par_profil_sur_donnees_reelles() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "Groupes").await;
    // Suspension (F3 : la mesure ; F4 : motif et référence).
    ent.suspendre_lien_for_actor(etab, d, r, dos.lien_b, "Ordonnance de protection du 12/03", Some("RG 24/118"))
        .await
        .unwrap();
    let (_, accueil) = agent(&ent, etab, Role::Accueil, "Accueil").await;

    // Direction : tout, motif de la suspension compris.
    let vue = ent.responsables_eleve_for_actor(etab, d, r, dos.eleve).await.unwrap();
    let b = de(&vue, dos.pers_b);
    assert_eq!(b.personne.telephone.as_deref(), Some(TEL_B));
    assert_eq!(b.personne.adresse.as_deref(), Some(ADRESSE_B));
    assert!(b.suspension_motif.is_some() && b.suspension_reference.is_some());
    assert!(b.masque.is_empty(), "direction : rien de masqué");
    assert!(de(&vue, dos.pers_a).est_payeur);

    // Administratif : F0–F3 et payeurs, jamais F4 (motif de suspension).
    let adm = cast.administratif.user_id;
    let vue = ent.responsables_eleve_for_actor(etab, adm, "administratif", dos.eleve).await.unwrap();
    let b = de(&vue, dos.pers_b);
    assert_eq!(b.personne.adresse.as_deref(), Some(ADRESSE_B));
    assert!(b.suspendu_le.is_some(), "F3 : la mesure est visible");
    assert!(b.suspension_motif.is_none() && b.suspension_reference.is_none(), "F4 fermé");
    assert_eq!(groupes_fermes(b), vec!["F4"]);
    assert!(de(&vue, dos.pers_a).est_payeur);

    // Vie scolaire : F0, F1, F3 ; ni adresse (F2), ni motif (F4), ni payeurs.
    let cpe = cast.cpe.user_id;
    let vue = ent.responsables_eleve_for_actor(etab, cpe, "cpe", dos.eleve).await.unwrap();
    let a = de(&vue, dos.pers_a);
    assert_eq!(a.personne.telephone.as_deref(), Some("0600000001"));
    assert!(a.personne.adresse.is_none() && a.foyer_id.is_none());
    assert!(a.autorite_parentale, "F3 ouvert pour la vie scolaire");
    assert!(!a.est_payeur && a.part_paiement.is_none(), "payeurs : suivent payeurs.definir");
    assert_eq!(groupes_fermes(a), vec!["F2", "F4"]);
    let f2 = a.masque.iter().find(|m| m.groupe.code() == "F2").unwrap();
    assert_eq!(f2.raison.code(), "droit_manquant");
    assert_eq!(
        f2.raison.action_a_demander().map(|x| x.code()),
        Some("responsables.adresses.lire")
    );

    // Comptabilité : F0–F2 et payeurs ; pas de situation juridique (F3).
    let compta = cast.compta.user_id;
    let vue = ent.responsables_eleve_for_actor(etab, compta, "compta", dos.eleve).await.unwrap();
    let a = de(&vue, dos.pers_a);
    assert!(a.personne.adresse.is_some() && a.est_payeur && a.part_paiement == Some(100));
    assert!(!a.autorite_parentale && a.garde.is_none() && !a.peut_voir_notes);
    assert_eq!(groupes_fermes(a), vec!["F3", "F4", "R"]);

    // Accueil : désignation et moyens de joindre seulement.
    let vue = ent.responsables_eleve_for_actor(etab, accueil, "accueil", dos.eleve).await.unwrap();
    let a = de(&vue, dos.pers_a);
    assert_eq!(a.personne.telephone.as_deref(), Some("0600000001"));
    assert!(a.personne.adresse.is_none() && !a.autorite_parentale && !a.est_payeur);
    assert_eq!(groupes_fermes(a), vec!["F2", "F3", "F4"]);

    // Professeur qui ne suit pas l'élève : rien. Psychologue : rien (aucun groupe).
    assert!(matches!(
        ent.responsables_eleve_for_actor(etab, cast.prof.user_id, "prof", dos.eleve).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.responsables_eleve_for_actor(etab, cast.psys[0].user_id, "psy", dos.eleve).await,
        Err(EntDbError::Forbidden)
    ));
}

/// Raisons de masquage : la fiche renvoie, pour chaque groupe fermé, la raison et le droit à demander.
#[tokio::test]
async fn la_fiche_explique_chaque_groupe_ferme() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dos = dossier_separe(&ent, etab, "Raisons").await;
    let fiche = ent
        .fiche_eleve_for_actor(etab, cast.cpe.user_id, "cpe", dos.eleve)
        .await
        .unwrap()
        .unwrap();
    let codes: Vec<_> = fiche.masquages.iter().map(|m| m.groupe.code()).collect();
    assert_eq!(codes, vec!["F2", "F4"]);
    assert!(fiche.masquages.iter().all(|m| !m.raison.message().is_empty()));
    assert!(!fiche.responsables.is_empty());
    // La direction n'a aucun masquage.
    let (d, r) = dir(&cast);
    let fiche = ent.fiche_eleve_for_actor(etab, d, r, dos.eleve).await.unwrap().unwrap();
    assert!(fiche.masquages.is_empty());
}

/// R-A18 — un agent responsable d'un élève ne lit pas, avec ses droits professionnels, F2 à F4 ni
/// les payeurs de l'autre parent ; F0, F1 et R restent lisibles, avec trace ; toute écriture
/// passe par un collègue ; sa casquette de parent est inchangée.
#[tokio::test]
async fn conflit_d_interet_agent_parent_de_l_eleve() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "Conflit").await;
    // Sylvie, secrétaire (droits « administratif »), est aussi la mère A : même compte.
    sqlx::query("UPDATE ent_membership SET role = 'administratif' WHERE user_id = $1")
        .bind(dos.user_a)
        .execute(ent.pool())
        .await
        .unwrap();
    let sylvie = dos.user_a;
    assert_eq!(
        ent.conflit_interet_eleve(etab, sylvie, "administratif", dos.eleve).await.unwrap(),
        Some(NatureConflit::Responsable)
    );
    let traces_avant = traces(&ent, etab, sylvie).await;

    // Casquette professionnelle : l'autre parent est réduit à sa désignation et à ses moyens de joindre.
    let vue = ent.responsables_eleve_for_actor(etab, sylvie, "administratif", dos.eleve).await.unwrap();
    let b = de(&vue, dos.pers_b);
    assert_eq!(b.personne.telephone.as_deref(), Some(TEL_B), "F1 reste lisible");
    assert!(b.personne.adresse.is_none() && b.foyer_id.is_none(), "F2 fermé");
    assert!(!b.autorite_parentale && !b.est_payeur, "F3 et payeurs fermés");
    assert!(b.suspension_motif.is_none(), "F4 fermé");
    assert!(b.masque.iter().any(|m| m.groupe.code() == "F2" && m.raison.code() == "conflit_interet"));
    assert!(b.masque.iter().any(|m| m.groupe.code() == "F4" && m.raison.code() == "conflit_interet"));
    assert!(traces(&ent, etab, sylvie).await > traces_avant, "accès tracé comme sensible");

    // La recherche et la lecture d'une personne ne contournent pas la règle (adresse = F2).
    let trouvees = ent.rechercher_personnes_for_actor(etab, sylvie, "administratif", "ParentB").await.unwrap();
    let b_trouvee = trouvees.iter().find(|t| t.personne.id == dos.pers_b).unwrap();
    assert!(b_trouvee.personne.adresse.is_none() && b_trouvee.personne.telephone.is_some());
    let b_lue = ent.get_personne_for_actor(etab, sylvie, "administratif", dos.pers_b).await.unwrap();
    assert!(b_lue.adresse.is_none());
    // Sa propre fiche reste lisible en entier.
    assert!(ent.get_personne_for_actor(etab, sylvie, "administratif", dos.pers_a).await.unwrap().adresse.is_some());
    let (_, collegue_lecteur) = agent(&ent, etab, Role::Administratif, "Lecteur").await;
    assert!(ent
        .get_personne_for_actor(etab, collegue_lecteur, "administratif", dos.pers_b)
        .await
        .unwrap()
        .adresse
        .is_some());

    // Les contacts d'urgence (R) restent lisibles.
    assert!(ent.contacts_eleve_for_actor(etab, sylvie, "administratif", dos.eleve).await.is_ok());

    // Écritures : refusées pour le dossier de l'élève, ou une personne qui lui est liée.
    fn refus<T>(e: Result<T, EntDbError>) -> bool {
        matches!(e, Err(EntDbError::ConflitDossier(m)) if m.contains("collègue"))
    }
    let saisie = PersonneSaisie {
        nom: "ParentB".into(),
        telephone: Some("0699999999".into()),
        ..Default::default()
    };
    assert!(refus(ent.modifier_personne_for_actor(etab, sylvie, "administratif", dos.pers_b, &saisie).await));
    assert!(refus(
        ent.modifier_lien_for_actor(etab, sylvie, "administratif", dos.lien_b, &lien_saisie(None, None))
            .await
    ));
    assert!(refus(
        ent.definir_payeurs_for_actor(etab, sylvie, "administratif", dos.eleve, &[(dos.lien_b, 100)])
            .await
    ));
    assert!(refus(
        ent.ajouter_contact_for_actor(
            etab,
            sylvie,
            "administratif",
            dos.eleve,
            &ContactSaisie {
                personne_id: None,
                nouvelle_personne: Some(PersonneSaisie {
                    nom: "Voisine".into(),
                    telephone: Some("0644444444".into()),
                    ..Default::default()
                }),
                relation: "voisine".into(),
                ordre: 1,
                peut_recuperer: false,
                commentaire: None,
            },
        )
        .await
    ));
    // Un collègue, lui, peut modifier : la règle vise l'agent lié, pas le dossier.
    let (_, collegue) = agent(&ent, etab, Role::Administratif, "Collegue").await;
    ent.modifier_personne_for_actor(etab, collegue, "administratif", dos.pers_b, &saisie).await.unwrap();

    // Casquette de parent inchangée : elle voit SES données en entier, et de B la désignation seulement.
    let vue = ent.responsables_eleve_for_actor(etab, sylvie, "parent", dos.eleve).await.unwrap();
    let a = de(&vue, dos.pers_a);
    assert!(a.personne.adresse.is_some() && a.est_payeur && a.autorite_parentale);
    assert!(de(&vue, dos.pers_b).personne.telephone.is_none());

    // La direction voit l'accès tracé dans son tableau.
    let tableau = ent.conflits_interet_for_actor(etab, d, r).await.unwrap();
    assert!(tableau.iter().any(|l| l.user_id == sylvie && l.eleve_id == dos.eleve && l.acces_30j >= 1));
}

async fn traces(ent: &PgEnt, etab: Uuid, user: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif
          WHERE etablissement_id = $1 AND actor_user_id = $2 AND domaine = 'conflit_interet'",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    n
}

/// R-A18 — l'ex-conjoint : aucun lien visible dans le schéma, donc une **déclaration** (par l'agent
/// ou par la direction) ferme F2–F4 pour cet élève ; seule la direction lève, jamais l'agent.
#[tokio::test]
async fn conflit_d_interet_declare_ex_conjoint() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "ExConjoint").await;
    let (_, agent_user) = agent(&ent, etab, Role::Administratif, "Carole").await;
    let role = "administratif";
    // Avant déclaration : lecture complète.
    let vue = ent.responsables_eleve_for_actor(etab, agent_user, role, dos.eleve).await.unwrap();
    assert!(de(&vue, dos.pers_b).personne.adresse.is_some());

    // Déclaration par l'agent lui-même.
    let id = ent
        .declarer_conflit_interet_for_actor(etab, agent_user, role, None, dos.eleve, "ex_conjoint")
        .await
        .unwrap();
    // Idempotente.
    assert_eq!(
        ent.declarer_conflit_interet_for_actor(etab, agent_user, role, None, dos.eleve, "ex_conjoint")
            .await
            .unwrap(),
        id
    );
    let vue = ent.responsables_eleve_for_actor(etab, agent_user, role, dos.eleve).await.unwrap();
    let b = de(&vue, dos.pers_b);
    assert!(b.personne.adresse.is_none() && !b.autorite_parentale);
    assert_eq!(b.personne.telephone.as_deref(), Some(TEL_B));
    // L'agent ne lève pas sa propre déclaration ; il n'a d'ailleurs pas le droit de gérer.
    assert!(ent.lever_conflit_interet_for_actor(etab, agent_user, role, id).await.is_err());
    // La direction la voit et la lève : la lecture complète revient.
    let tableau = ent.conflits_interet_for_actor(etab, d, r).await.unwrap();
    assert!(tableau.iter().any(|l| l.declaration_id == Some(id) && l.nature.as_deref() == Some("ex_conjoint")));
    ent.lever_conflit_interet_for_actor(etab, d, r, id).await.unwrap();
    let vue = ent.responsables_eleve_for_actor(etab, agent_user, role, dos.eleve).await.unwrap();
    assert!(de(&vue, dos.pers_b).personne.adresse.is_some());

    // La direction déclare pour un autre agent ; un agent ne déclare pas pour un autre.
    let (_, autre) = agent(&ent, etab, Role::Cpe, "Autre").await;
    assert!(ent
        .declarer_conflit_interet_for_actor(etab, agent_user, role, Some(autre), dos.eleve, "proche")
        .await
        .is_err());
    ent.declarer_conflit_interet_for_actor(etab, d, r, Some(autre), dos.eleve, "proche").await.unwrap();
    let vue = ent.responsables_eleve_for_actor(etab, autre, "cpe", dos.eleve).await.unwrap();
    assert!(!de(&vue, dos.pers_a).autorite_parentale, "F3 fermé par la déclaration de la direction");
    // Nature inconnue, ou cible hors personnel : refusées.
    assert!(ent
        .declarer_conflit_interet_for_actor(etab, d, r, Some(dos.user_a), dos.eleve, "proche")
        .await
        .is_err());
    assert!(ent
        .declarer_conflit_interet_for_actor(etab, agent_user, role, None, dos.eleve, "invente")
        .await
        .is_err());
}

/// R-A18 — contact d'urgence : un agent contact d'urgence de l'élève est aussi en conflit.
#[tokio::test]
async fn conflit_d_interet_contact_d_urgence() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "ContactConflit").await;
    let (pers, user) = agent(&ent, etab, Role::Cpe, "Tante").await;
    ent.ajouter_contact_for_actor(
        etab,
        d,
        r,
        dos.eleve,
        &ContactSaisie {
            personne_id: Some(pers),
            nouvelle_personne: None,
            relation: "tante".into(),
            ordre: 2,
            peut_recuperer: false,
            commentaire: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        ent.conflit_interet_eleve(etab, user, "cpe", dos.eleve).await.unwrap(),
        Some(NatureConflit::Contact)
    );
    let vue = ent.responsables_eleve_for_actor(etab, user, "cpe", dos.eleve).await.unwrap();
    assert!(!de(&vue, dos.pers_a).autorite_parentale);
    // Un agent sans lien n'est jamais en conflit.
    assert_eq!(
        ent.conflit_interet_eleve(etab, cast.cpe.user_id, "cpe", dos.eleve).await.unwrap(),
        None
    );
}

/// Droit d'accès d'un titulaire — il voit ses données et la désignation de l'autre responsable ;
/// rien d'autre sur lui (coordonnées, adresse, situation juridique, payeurs), jamais F4, jamais
/// une interdiction ; les contacts déclarés par l'autre ne lui sont pas montrés.
#[tokio::test]
async fn droit_d_acces_d_un_parent_sans_les_donnees_de_l_autre() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "DroitAcces").await;
    // B déclare un contact (grand-mère) ; A en déclare un autre ; le secrétariat en saisit un.
    let user_b: Uuid = sqlx::query_scalar("SELECT user_id FROM ent_personne WHERE id = $1")
        .bind(dos.pers_b)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let contact = |nom: &str, tel: &str| ContactSaisie {
        personne_id: None,
        nouvelle_personne: Some(PersonneSaisie {
            nom: nom.into(),
            telephone: Some(tel.into()),
            ..Default::default()
        }),
        relation: "proche".into(),
        ordre: 1,
        peut_recuperer: false,
        commentaire: None,
    };
    ent.ajouter_contact_for_actor(etab, user_b, "parent", dos.eleve, &contact("GrandMereDeB", "0655555555"))
        .await
        .unwrap();
    ent.ajouter_contact_for_actor(etab, dos.user_a, "parent", dos.eleve, &contact("OncleDeA", "0666666666"))
        .await
        .unwrap();
    ent.ajouter_contact_for_actor(etab, d, r, dos.eleve, &contact("VoisinEcole", "0677777777"))
        .await
        .unwrap();
    // Une interdiction visant A, posée par la direction.
    ent.poser_interdiction_for_actor(
        etab,
        d,
        r,
        dos.eleve,
        &InterdictionSaisie {
            personne_id: Some(dos.pers_a),
            objet: vec!["remise".into()],
            fondement: "decision_justice".into(),
            motif: Some("Jugement du 3 mai : motif sensible".into()),
            consigne: "Ne pas remettre ; prévenir la direction".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let a = dos.user_a;
    let vue = ent.responsables_eleve_for_actor(etab, a, "parent", dos.eleve).await.unwrap();
    let moi = de(&vue, dos.pers_a);
    assert!(moi.personne.adresse.is_some() && moi.est_payeur && moi.autorite_parentale);
    let autre = de(&vue, dos.pers_b);
    assert_eq!(autre.personne.nom, "ParentB", "F0 : la désignation");
    assert_eq!(autre.lien, "père");
    assert!(autre.personne.email.is_none() && autre.personne.telephone.is_none());
    assert!(autre.personne.adresse.is_none() && autre.foyer_id.is_none());
    assert!(!autre.autorite_parentale && autre.garde.is_none() && !autre.peut_voir_notes);
    assert!(!autre.est_payeur && autre.part_paiement.is_none());
    assert!(autre.suspension_motif.is_none() && autre.suspension_reference.is_none());
    assert!(autre.masque.iter().any(|m| m.groupe.code() == "F1" && m.raison.code() == "autre_responsable"));
    assert!(autre.masque.iter().any(|m| m.groupe.code() == "F4" && m.raison.code() == "reserve_etablissement"));

    // Contacts : les siens et ceux de l'établissement, pas ceux de B.
    let noms: Vec<String> = ent
        .contacts_eleve_for_actor(etab, a, "parent", dos.eleve)
        .await
        .unwrap()
        .into_iter()
        .map(|c| c.personne.nom)
        .collect();
    assert!(noms.contains(&"OncleDeA".to_string()) && noms.contains(&"VoisinEcole".to_string()));
    assert!(!noms.contains(&"GrandMereDeB".to_string()));

    // Fiche : aucune donnée de B ni interdiction, nulle part dans la réponse.
    let fiche = ent.fiche_eleve_for_actor(etab, a, "parent", dos.eleve).await.unwrap().unwrap();
    assert!(fiche.interdictions.is_empty(), "jamais une interdiction qui le vise");
    let texte = format!("{:?} {:?}", fiche.responsables, fiche.contacts);
    for secret in [TEL_B, ADRESSE_B, EMAIL_B, "GrandMereDeB", "motif sensible", "Jugement"] {
        assert!(!texte.contains(secret), "fuite vers l'autre parent : {secret}");
    }
    // L'interdiction n'est pas non plus lisible par la route dédiée.
    assert!(matches!(
        ent.interdictions_eleve_for_actor(etab, a, "parent", dos.eleve).await,
        Err(EntDbError::Forbidden)
    ));
}

/// Interdiction de remise — la consigne est visible de l'accueil et de la vie scolaire, le
/// fondement avec F3, le motif (scellé) avec F4 seulement ; invisible de la personne visée ; pose
/// et levée réservées à `remise.interdire` ; motif jamais en clair en base ni au journal.
#[tokio::test]
async fn interdiction_de_remise_visible_accueil_et_vie_scolaire() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "Interdit").await;
    let (_, accueil) = agent(&ent, etab, Role::Accueil, "Josiane").await;
    let motif = "Violences conjugales, ordonnance de protection du 12 mars";
    let id = ent
        .poser_interdiction_for_actor(
            etab,
            d,
            r,
            dos.eleve,
            &InterdictionSaisie {
                personne_id: Some(dos.pers_b),
                objet: vec!["remise".into(), "visite".into()],
                fondement: "ordonnance_protection".into(),
                motif: Some(motif.into()),
                consigne: "Ne pas remettre l'enfant ; prévenir la direction ; appeler la mère".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Accueil : personne visée, objets, consigne ; ni fondement ni motif.
    let v = ent.interdictions_eleve_for_actor(etab, accueil, "accueil", dos.eleve).await.unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].personne_libelle, "Avec ParentB");
    assert_eq!(v[0].objet, vec!["remise".to_string(), "visite".to_string()]);
    assert!(v[0].consigne.starts_with("Ne pas remettre"));
    assert!(v[0].bloquante);
    assert!(v[0].fondement.is_none() && v[0].motif.is_none());

    // Vie scolaire : consigne et fondement (F3), jamais le motif.
    let v = ent.interdictions_eleve_for_actor(etab, cast.cpe.user_id, "cpe", dos.eleve).await.unwrap();
    assert_eq!(v[0].fondement.as_deref(), Some("ordonnance_protection"));
    assert!(v[0].motif.is_none());
    // Secrétariat : idem. Direction : motif ouvert (et la consultation est tracée).
    let v = ent
        .interdictions_eleve_for_actor(etab, cast.administratif.user_id, "administratif", dos.eleve)
        .await
        .unwrap();
    assert!(v[0].motif.is_none());
    let avant: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'interdiction_motif'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let v = ent.interdictions_eleve_for_actor(etab, d, r, dos.eleve).await.unwrap();
    assert_eq!(v[0].motif.as_deref(), Some(motif));
    let apres: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'interdiction_motif'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(apres, avant + 1);

    // Sans `remise.lire` (comptabilité, psychologue, professeur qui ne suit pas l'élève) : refusé.
    for (u, role) in [
        (cast.compta.user_id, "compta"),
        (cast.psys[0].user_id, "psy"),
        (cast.prof.user_id, "prof"),
    ] {
        assert!(
            matches!(
                ent.interdictions_eleve_for_actor(etab, u, role, dos.eleve).await,
                Err(EntDbError::Forbidden)
            ),
            "{role}"
        );
    }

    // La personne visée, même agent d'accueil, ne lit pas l'interdiction qui la vise.
    let (pers_visee, visee) = agent(&ent, etab, Role::Accueil, "Visee").await;
    ent.poser_interdiction_for_actor(
        etab,
        d,
        r,
        dos.eleve,
        &InterdictionSaisie {
            personne_id: Some(pers_visee),
            objet: vec!["contact_eleve".into()],
            fondement: "decision_etablissement".into(),
            consigne: "Aucun contact avec l'élève".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let v = ent.interdictions_eleve_for_actor(etab, visee, "accueil", dos.eleve).await.unwrap();
    assert_eq!(v.len(), 1, "elle ne voit que l'interdiction de B, pas celle qui la vise");
    let v = ent.interdictions_eleve_for_actor(etab, accueil, "accueil", dos.eleve).await.unwrap();
    assert_eq!(v.len(), 2);

    // Pose et levée : `remise.interdire` seulement (ni accueil, ni vie scolaire, ni secrétariat).
    let saisie = InterdictionSaisie {
        personne_libelle: Some("Inconnu".into()),
        objet: vec!["remise".into()],
        fondement: "demande_titulaire".into(),
        consigne: "À vérifier".into(),
        ..Default::default()
    };
    for (u, role) in [
        (accueil, "accueil"),
        (cast.cpe.user_id, "cpe"),
        (cast.administratif.user_id, "administratif"),
    ] {
        assert!(matches!(
            ent.poser_interdiction_for_actor(etab, u, role, dos.eleve, &saisie).await,
            Err(EntDbError::Forbidden)
        ));
        assert!(matches!(
            ent.lever_interdiction_for_actor(etab, u, role, id, "motif").await,
            Err(EntDbError::Forbidden)
        ));
    }
    // `demande_titulaire` : enregistrée, non bloquante par défaut.
    ent.poser_interdiction_for_actor(etab, d, r, dos.eleve, &saisie).await.unwrap();
    let v = ent.interdictions_eleve_for_actor(etab, accueil, "accueil", dos.eleve).await.unwrap();
    assert!(v.iter().any(|i| i.personne_libelle == "Inconnu" && !i.bloquante));

    // Le motif n'est jamais en clair : ni en base, ni au journal.
    let scelle: String = sqlx::query_scalar("SELECT motif_scelle FROM ent_eleve_interdiction WHERE id = $1")
        .bind(id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(scelle.starts_with("enc:v1:") && !scelle.contains("Violences"));
    let fuite: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND payload::text LIKE '%Violences%'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(fuite, 0);

    // Levée : motif obligatoire, puis l'interdiction disparaît pour tous ; double levée refusée.
    assert!(ent.lever_interdiction_for_actor(etab, d, r, id, "  ").await.is_err());
    ent.lever_interdiction_for_actor(etab, d, r, id, "Décision d'appel du 2 juin").await.unwrap();
    let v = ent.interdictions_eleve_for_actor(etab, accueil, "accueil", dos.eleve).await.unwrap();
    assert!(v.iter().all(|i| i.id != id));
    assert!(ent.lever_interdiction_for_actor(etab, d, r, id, "encore").await.is_err());
}

/// Validation d'une interdiction et dates : objet, fondement, consigne, cible, période.
#[tokio::test]
async fn interdiction_validation_et_periode() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "Validation").await;
    let (_, accueil) = agent(&ent, etab, Role::Accueil, "Josiane").await;
    let ok = InterdictionSaisie {
        personne_id: Some(dos.pers_b),
        objet: vec!["remise".into()],
        fondement: "decision_justice".into(),
        consigne: "Ne pas remettre".into(),
        ..Default::default()
    };
    let invalide = |mutation: fn(&mut InterdictionSaisie)| {
        let mut s = ok.clone();
        mutation(&mut s);
        s
    };
    for s in [
        invalide(|s| s.objet = vec![]),
        invalide(|s| s.objet = vec!["inconnu".into()]),
        invalide(|s| s.fondement = "caprice".into()),
        invalide(|s| s.consigne = "   ".into()),
        invalide(|s| s.consigne = "x".repeat(301)),
        invalide(|s| s.personne_id = None),
        invalide(|s| s.personne_id = Some(Uuid::new_v4())),
    ] {
        assert!(ent.poser_interdiction_for_actor(etab, d, r, dos.eleve, &s).await.is_err(), "{s:?}");
    }
    // Période : future (pas encore active) puis échue (plus active) ; fin antérieure au début refusée.
    let demain = chrono::Utc::now().date_naive() + chrono::Duration::days(3);
    let hier = chrono::Utc::now().date_naive() - chrono::Duration::days(3);
    let mut future = ok.clone();
    future.du = Some(demain);
    ent.poser_interdiction_for_actor(etab, d, r, dos.eleve, &future).await.unwrap();
    let mut echue = ok.clone();
    echue.du = Some(hier - chrono::Duration::days(10));
    echue.au = Some(hier);
    ent.poser_interdiction_for_actor(etab, d, r, dos.eleve, &echue).await.unwrap();
    let mut inverse = ok.clone();
    inverse.du = Some(demain);
    inverse.au = Some(hier);
    assert!(ent.poser_interdiction_for_actor(etab, d, r, dos.eleve, &inverse).await.is_err());
    let v = ent.interdictions_eleve_for_actor(etab, accueil, "accueil", dos.eleve).await.unwrap();
    assert!(v.is_empty(), "ni la future ni l'échue ne sont actives : {v:?}");
}

/// R-A18 — la pose d'une interdiction sur son propre dossier passe par un collègue.
#[tokio::test]
async fn interdiction_sur_son_propre_dossier_par_un_collegue() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let dos = dossier_separe(&ent, etab, "PropreDossier").await;
    // Une directrice adjointe (profil direction) est la mère A.
    sqlx::query("UPDATE ent_membership SET role = 'direction' WHERE user_id = $1")
        .bind(dos.user_a)
        .execute(ent.pool())
        .await
        .unwrap();
    let saisie = InterdictionSaisie {
        personne_id: Some(dos.pers_b),
        objet: vec!["remise".into()],
        fondement: "decision_etablissement".into(),
        consigne: "Ne pas remettre".into(),
        ..Default::default()
    };
    assert!(matches!(
        ent.poser_interdiction_for_actor(etab, dos.user_a, "direction", dos.eleve, &saisie).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.poser_interdiction_for_actor(etab, d, r, dos.eleve, &saisie).await.unwrap();
}

/// Les droits ajoutés par A1 sont dans le catalogue, avec leurs inclusions explicites : détenir
/// `personnes.gerer` ouvre F0–F2, `liens.juridique.gerer` ouvre F3, etc.
#[tokio::test]
async fn inclusions_explicites_du_catalogue() {
    use ent_auth::Action::*;
    assert!(ResponsablesLire.accordee_par().contains(&PersonnesGerer));
    assert!(ResponsablesAdressesLire.accordee_par().contains(&PersonnesGerer));
    assert!(ResponsablesJuridiqueLire.accordee_par().contains(&LiensJuridiqueGerer));
    assert!(RemiseLire.accordee_par().contains(&RemiseGerer));
    assert!(ResponsablesProtegeLire.accordee_par().contains(&ResponsablesProtegeGerer));
    // Aucune autre inclusion implicite : gérer les personnes n'ouvre ni F3, ni F4, ni R.
    assert!(!ResponsablesJuridiqueLire.accordee_par().contains(&PersonnesGerer));
    assert!(ResponsablesProtegeLire.accordee_par() != [PersonnesGerer]);
    assert!(RemiseLire.accordee_par() != [PersonnesGerer]);
}
