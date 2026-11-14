//! Socle 29, lot P3 : cartes d'action des socles 3, 18 et 19 (autorisation ou réponse, rendez-vous,
//! réinscription) : cardinalité « un seul titulaire suffit », neutralité entre foyers (R-PO7, R-PO38),
//! parent sans exercice (R-PO8), élève majeur (§8.8). Base locale de test (feature `labo`).
//!
//! Chaque test ne vérifie que ses propres lignes : tâches retrouvées par nature et par enfant.

use super::*;
use crate::afaire::{Cardinalite, NATURES};
use chrono::{Duration, Utc};
use ent_types::afaire::{AFaire, ChampAction, EtatAfaire, TacheAfaire, MESSAGE_DEJA_TRAITEE};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

fn eleve<'a>(c: &'a LaboCast, prenom: &str) -> &'a LaboEleve {
    c.eleves
        .iter()
        .find(|e| e.prenom == prenom)
        .expect("élève du cast")
}

/// Lève le délai de visibilité (R-PO31, testé à part) : ces scénarios ne dépendent pas de l'heure de la CI.
async fn sans_plage(ent: &PgEnt, etab: Uuid) {
    sqlx::query("UPDATE ent_afaire SET visible_a_partir_de = LEAST(visible_a_partir_de, now()) WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

async fn rapprocher(ent: &PgEnt, etab: Uuid) {
    ent.rapprocher_afaire(etab, Utc::now()).await.unwrap();
    sans_plage(ent, etab).await;
}

async fn liste(ent: &PgEnt, etab: Uuid, user: Uuid) -> AFaire {
    ent.a_faire_de(etab, user, "parent", Utc::now())
        .await
        .expect("à-faire")
}

fn de_nature<'a>(a: &'a AFaire, nature: &str, eleve: Uuid) -> Vec<&'a TacheAfaire> {
    a.taches
        .iter()
        .filter(|t| t.nature == nature && t.eleve_id == Some(eleve))
        .collect()
}

fn est_deja_traitee<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(m)) if m == MESSAGE_DEJA_TRAITEE)
}

/// Second parent d'un élève, dans un autre foyer, avec ou sans autorité parentale.
async fn second_parent(ent: &PgEnt, etab: Uuid, eleve: Uuid, autorite: bool) -> Uuid {
    let user = Uuid::new_v4();
    let p = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1, $2, 'x', 'Second', now(), now())",
    )
    .bind(user)
    .bind(format!("p3-{}@ent.lab", user.simple()))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'parent', now())")
        .bind(user)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let foyer = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, 'Autre foyer P3')")
        .bind(foyer)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let _ = p;
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien, autorite_parentale)
         VALUES ($1, $2, $3, $4, $5, 'pere', $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .bind(autorite)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    user
}

fn objet_aleatoire() -> String {
    format!("t29_{}", &Uuid::new_v4().simple().to_string()[..16])
}

async fn formulaire_publie(ent: &PgEnt, c: &LaboCast, titre: &str) -> Uuid {
    let (adm, role) = (c.direction.user_id, "direction");
    let f = ent
        .creer_formulaire(
            c.etablissement_id,
            adm,
            role,
            crate::formulaires::NouveauFormulaire {
                type_formulaire: "autorisation".into(),
                objet: Some(objet_aleatoire()),
                titre: titre.into(),
                texte: "Autorisez-vous cette activité ?".into(),
                audience: "etablissement".into(),
                cible_ids: vec![],
            },
        )
        .await
        .unwrap();
    ent.publier_formulaire(c.etablissement_id, adm, role, f.id)
        .await
        .unwrap();
    f.id
}

/// Déclarations : les trois natures du lot sont par autorité (un seul titulaire suffit, A11), pour
/// la famille (plage calme), et chacune a un texte de gabarit neutre (aucun motif, aucun montant).
#[test]
fn p3_declarations_des_natures_famille() {
    for code in ["form.repondre", "rdv.reserver", "insc.reinscrire"] {
        let n = NATURES.iter().find(|n| n.code == code).expect(code);
        assert_eq!(n.cardinalite, Cardinalite::ParAutorite, "{code}");
        assert!(n.public_famille && !n.urgente, "{code}");
        assert!(n.derivee, "{code} : le rapprochement doit pouvoir la rebâtir");
    }
    crate::afaire::verifier_declarations(NATURES).expect("déclarations valides");
}

/// Carte « autorisation » (socle 18) : une réponse par enfant, un seul titulaire suffit, la
/// commande du propriétaire écrit, l'autre foyer voit « déjà traitée » (R-PO7, R-PO38), un
/// responsable sans exercice n'a aucune tâche de décision (R-PO8).
#[tokio::test]
async fn p3_carte_autorisation_neutralite_et_sans_exercice() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    let autre = second_parent(&ent, etab, harry.id, true).await;
    let sans_exercice = second_parent(&ent, etab, harry.id, false).await;
    let titre = format!("Sortie {}", Uuid::new_v4().simple());
    let formulaire = formulaire_publie(&ent, &c, &titre).await;
    rapprocher(&ent, etab).await;

    // Témoin : les deux titulaires ont la tâche (même identifiant), le sans-exercice non.
    let a = liste(&ent, etab, harry.parent.user_id).await;
    let b = liste(&ent, etab, autre).await;
    let (ta, tb) = (
        de_nature(&a, "form.repondre", harry.id),
        de_nature(&b, "form.repondre", harry.id),
    );
    assert_eq!((ta.len(), tb.len()), (1, 1), "un seul titulaire suffit : la même tâche, deux portes");
    assert_eq!(ta[0].id, tb[0].id);
    assert!(ta[0].titre.contains("Harry") && !ta[0].titre.contains(&titre), "titre neutre");
    let tache = ta[0].id;
    let s = liste(&ent, etab, sans_exercice).await;
    assert!(
        de_nature(&s, "form.repondre", harry.id).is_empty(),
        "R-PO8 : aucune tâche de décision sans exercice"
    );

    // Aperçu : la carte demande une réponse parmi une liste fermée ; rien n'est écrit.
    let ap = ent
        .afaire_apercu(etab, harry.parent.user_id, "parent", tache, Utc::now())
        .await
        .unwrap();
    let champ: &ChampAction = ap.champs.iter().find(|c| c.code == "decision").unwrap();
    assert!(champ.obligatoire && champ.choix == ["accepte", "refuse"]);
    let ecrites = |ent: &PgEnt| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND formulaire_id = $2")
                .bind(etab)
                .bind(formulaire)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    assert_eq!(ecrites(&ent).await, 0, "l'aperçu n'écrit rien");

    // Sans confirmation, sans réponse, ou avec une réponse hors liste : refus, toujours rien d'écrit.
    for (confirme, decision) in [(false, Some("accepte")), (true, None), (true, Some("retire"))] {
        let r = ent
            .afaire_executer_avec(
                etab, harry.parent.user_id, "parent", tache, confirme, None,
                decision.map(String::from), Utc::now(),
            )
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{confirme} {decision:?}");
    }
    assert_eq!(ecrites(&ent).await, 0);

    // Le responsable répond : la commande du propriétaire écrit (qualité « responsable »).
    let r = ent
        .afaire_executer_avec(
            etab, harry.parent.user_id, "parent", tache, true, None,
            Some("accepte".into()), Utc::now(),
        )
        .await
        .unwrap();
    assert!(r.fait);
    assert_eq!(ecrites(&ent).await, 1);
    let (qualite, decision): (String, String) = sqlx::query_as(
        "SELECT qualite, decision FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND formulaire_id = $2",
    )
    .bind(etab)
    .bind(formulaire)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((qualite.as_str(), decision.as_str()), ("responsable", "accepte"));

    // L'autre foyer : plus rien « à faire », la carte reste affichée sans nom ni date (R-PO38),
    // et toute action répond « déjà traitée » (même erreur qu'une clôture par l'établissement).
    let b = liste(&ent, etab, autre).await;
    let carte = de_nature(&b, "form.repondre", harry.id);
    assert_eq!(carte.len(), 1);
    assert_eq!(carte[0].etat, EtatAfaire::Faite);
    assert!(!format!("{b:?}").contains(&harry.parent.user_id.to_string()));
    let r = ent
        .afaire_executer_avec(
            etab, autre, "parent", tache, true, None, Some("refuse".into()), Utc::now(),
        )
        .await;
    assert!(est_deja_traitee(r));
    assert!(est_deja_traitee(
        ent.afaire_apercu(etab, autre, "parent", tache, Utc::now()).await
    ));
    assert_eq!(ecrites(&ent).await, 1, "la seconde tentative n'écrit rien");
    // Au prochain point neutre, la carte disparaît chez l'autre foyer.
    let plus_tard = ent
        .a_faire_de(etab, autre, "parent", Utc::now() + Duration::hours(25))
        .await
        .unwrap();
    assert!(de_nature(&plus_tard, "form.repondre", harry.id).is_empty());

    // Un parent d'un autre enfant ne voit rien, et son identifiant forgé est introuvable.
    let hermione = eleve(&c, "Hermione").clone();
    let tiers = liste(&ent, etab, hermione.parent.user_id).await;
    assert!(de_nature(&tiers, "form.repondre", harry.id).is_empty());
    let r = ent
        .afaire_apercu(etab, hermione.parent.user_id, "parent", tache, Utc::now())
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(_))), "introuvable, pas « déjà traitée » : {r:?}");
}

/// Majorité (§8.8) : la décision bascule vers l'élève, qui répond en son nom (qualité « eleve_majeur ») ;
/// les parents n'ont plus la tâche.
#[tokio::test]
async fn p3_carte_autorisation_majeur_bascule_vers_l_eleve() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let ron = eleve(&c, "Ron").clone();
    let Some(compte) = ron.compte.clone() else {
        eprintln!("skip : Ron n'a pas de compte élève dans ce jeu de labo");
        return;
    };
    {
        // Jeu de données : la date de naissance passe par la projection d'identité (garde SQL).
        let mut tx = ent.pool_verif().begin().await.unwrap();
        sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE ent_eleve SET date_naissance = (ent_aujourdhui_etab(etablissement_id) - INTERVAL '19 years')::date WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(ron.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    formulaire_publie(&ent, &c, &format!("Majeur {}", Uuid::new_v4().simple())).await;
    rapprocher(&ent, etab).await;
    let parent = liste(&ent, etab, ron.parent.user_id).await;
    assert!(de_nature(&parent, "form.repondre", ron.id).is_empty(), "le parent n'a plus la décision");
    let lui = ent
        .a_faire_de(etab, compte.user_id, "eleve", Utc::now())
        .await
        .unwrap();
    let t = de_nature(&lui, "form.repondre", ron.id);
    assert_eq!(t.len(), 1, "l'élève majeur a la tâche");
    ent.afaire_executer_avec(
        etab, compte.user_id, "eleve", t[0].id, true, None, Some("refuse".into()), Utc::now(),
    )
    .await
    .unwrap();
    let (q,): (String,) = sqlx::query_as(
        "SELECT qualite FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND eleve_id = $2 AND signataire_user_id = $3",
    )
    .bind(etab)
    .bind(ron.id)
    .bind(compte.user_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(q, "eleve_majeur");
}

/// Rendez-vous (socle 19) et réinscription (socle 3) : cartes à lien vers l'écran du propriétaire,
/// même cardinalité et même neutralité ; la tâche se ferme quand le propriétaire dit « fait ».
#[tokio::test]
async fn p3_rendez_vous_et_reinscription_par_lien_du_proprietaire() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    let autre = second_parent(&ent, etab, harry.id, true).await;
    let sans_exercice = second_parent(&ent, etab, harry.id, false).await;

    // Rencontre ouverte pour la classe de Harry (jeu de données du propriétaire).
    let (classe,): (Uuid,) = sqlx::query_as(
        "SELECT classe_id FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2
          AND classe_id IS NOT NULL
          AND date_debut <= ent_aujourdhui_etab($1) AND (date_fin IS NULL OR date_fin >= ent_aujourdhui_etab($1)) LIMIT 1",
    )
    .bind(etab)
    .bind(harry.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let rencontre = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_rencontre (id, etablissement_id, titre, classe_id, statut, cree_par, ouverte_le) VALUES ($1,$2,'Réunion P3',$3,'ouverte',$4, now())")
        .bind(rencontre)
        .bind(etab)
        .bind(classe)
        .bind(c.direction.user_id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let creneau = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_creneau_rdv (id, etablissement_id, rencontre_id, enseignant_user_id, debut, fin) VALUES ($1,$2,$3,$4, now() + interval '3 days', now() + interval '3 days 15 minutes')")
        .bind(creneau)
        .bind(etab)
        .bind(rencontre)
        .bind(c.prof.user_id)
        .execute(ent.pool_verif())
        .await
        .unwrap();

    // Réinscription : campagne ouverte, dossier brouillon préparé pour chaque élève actif.
    let (d, r) = crate::tests_inscriptions::dir(&c);
    let camp = ent
        .creer_campagne_for_actor(
            etab, d, r,
            &ent_types::inscriptions::CampagneBody {
                annee_cible: "2073-2074".into(),
                ..crate::tests_inscriptions::campagne("reinscription", None)
            },
        )
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(etab, d, r, camp.id).await.unwrap();
    rapprocher(&ent, etab).await;

    let a = liste(&ent, etab, harry.parent.user_id).await;
    let b = liste(&ent, etab, autre).await;
    for (nature, lien) in [("rdv.reserver", "/web/rencontres"), ("insc.reinscrire", "/web/reinscription")] {
        let (ta, tb) = (de_nature(&a, nature, harry.id), de_nature(&b, nature, harry.id));
        assert_eq!((ta.len(), tb.len()), (1, 1), "{nature} : un seul titulaire suffit");
        assert_eq!(ta[0].id, tb[0].id, "{nature}");
        match ta[0].action.as_ref().unwrap() {
            ent_types::afaire::ActionTache::Lien { lien: l, .. } => assert_eq!(l, lien),
            autre => panic!("{nature} : lien attendu, pas {autre:?}"),
        }
        // Le 29 n'exécute rien lui-même : pas de carte à confirmer, pas de raccourci (R-PO1).
        let r = ent
            .afaire_executer_avec(etab, harry.parent.user_id, "parent", ta[0].id, true, None, None, Utc::now())
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{nature} : {r:?}");
        let s = liste(&ent, etab, sans_exercice).await;
        assert!(de_nature(&s, nature, harry.id).is_empty(), "{nature} : R-PO8");
    }

    // Le propriétaire dit « fait » : réservation d'un créneau (commande du socle 19) puis confirmation
    // de la réinscription (socle 3) ; l'index se met à jour par la relecture des sources.
    sqlx::query("UPDATE ent_creneau_rdv SET statut = 'reserve', reserve_par_user_id = $3, eleve_id = $4, reserve_le = now() WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(creneau)
        .bind(harry.parent.user_id)
        .bind(harry.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let (dossier,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_inscription_dossier WHERE etablissement_id = $1 AND campagne_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(camp.id)
    .bind(harry.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    ent.confirmer_reinscription_famille(
        etab,
        harry.parent.user_id,
        "parent",
        dossier,
        &ent_types::inscriptions::ConfirmationReinscriptionBody::default(),
    )
    .await
    .unwrap();
    rapprocher(&ent, etab).await;
    let b = liste(&ent, etab, autre).await;
    for nature in ["rdv.reserver", "insc.reinscrire"] {
        assert!(
            de_nature(&b, nature, harry.id).iter().all(|t| !t.etat.est_ouvert()),
            "{nature} : plus rien à faire chez l'autre foyer"
        );
    }
}
