//! Socle 3, lot I5 — **passe 3** (contre-revue de la passe 2), contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Aucun test ne dépend de l'heure réelle ni du jour UTC : les instants passés au service sont posés (2027).
//!
//! | Correctif | Test(s) |
//! |---|---|
//! | 1 ordre des verrous : refus de la famille / décision sur le suivant | `p3_verrous_refus_famille_et_decision_sur_le_suivant` |
//! | 1 ordre des verrous : deux déplacements croisés | `p3_verrous_deplacements_croises` |
//! | 1 ordre des verrous : deux réutilisations croisées | `p3_verrous_reutilisations_croisees` |
//! | 2 registre et export, lignes réellement exportables | `p3_export_ne_rend_que_le_publie` |
//! | 3 publication caduque | `p3_publication_caduque_dossier_annule_avant_l_echeance` |
//! | 4 séquence `ordre_attente` | `p3_sequence_ordre_attente_reprise_puis_nouvel_entrant` |
//! | 5 écarter une proposition | `p3_ecarter_seulement_une_designation_de_la_liste` |
//! | 6 réparation de chaîne, gardes à `search_path` fixé | `p3_314_repare_une_chaine_partielle` |
//! | D-069 F15 acceptation au guichet | `p3_guichet_refus_distincts_notification_et_delai` |
//! | faibles : motif masqué, filtre avant `LIMIT`, `liberations` | `p3_motif_masque_et_ecartes_avant_la_limite`, `p3_liberations_autant_que_de_places` |

use ent_types::inscriptions::*;
use ent_types::inscriptions_decisions::*;
use uuid::Uuid;

use super::tests_inscriptions::{adm, courriels_vers, dir, labo};
use super::tests_inscriptions_i3_i4::{demarche_deux_enfants, regle, PiecesTest};
use super::tests_inscriptions_i5::{
    campagne_cap, corps, decider, dossier, email, expedier, maintenant, proposition_ouverte, revoir,
};
use super::tests_inscriptions_i5_revue::{agent_direction, agent_role, nombre, rang_vu, sqlstate};
use super::*;

const ANNEES: [&str; 6] = [
    "2027-2028",
    "2028-2029",
    "2029-2030",
    "2030-2031",
    "2031-2032",
    "2032-2033",
];

fn jours(n: i64) -> chrono::Duration {
    chrono::Duration::days(n)
}

async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    nombre(ent, sql, id).await
}

// ───────────────────────────── 1. Ordre des verrous ─────────────────────────────

/// Refus de la famille sur X (la place se libère, le suivant Y est désigné) **pendant** une décision de la direction sur Y :
/// les deux se terminent, sans interblocage (40P01 est mappé en conflit : un conflit est donc aussi une défaillance ici).
#[tokio::test]
async fn p3_verrous_refus_famille_et_decision_sur_le_suivant() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    for annee in ANNEES {
        let (camp, offre) = campagne_cap(&ent, &c, Some(1), annee).await;
        let x = dossier(&ent, &c, camp, "RefusX", None).await;
        decider(&ent, &c, &x, &corps("admis", Some(offre), "Admis"))
            .await
            .unwrap();
        let y = dossier(&ent, &c, camp, "SuivantY", None).await;
        let z = dossier(&ent, &c, camp, "SuivantZ", None).await;
        for d in [&y, &z] {
            decider(&ent, &c, d, &corps("liste_attente", Some(offre), "Complet"))
                .await
                .unwrap();
        }
        let reponse = ReponsePropositionBody {
            reponse: "declinee".into(),
        };
        let refus_y = corps("refuse", Some(offre), "Refus du suivant");
        let refus = ent.repondre_proposition_famille(etab, &x.jeton, x.id, &reponse, maintenant());
        let decision = decider(&ent, &c, &y, &refus_y);
        let (r1, r2) = tokio::join!(refus, decision);
        assert!(r1.is_ok(), "{annee} : refus de la famille : {:?}", r1.err());
        assert!(
            r2.is_ok(),
            "{annee} : décision sur le suivant : {:?}",
            r2.err()
        );
        assert_eq!(
            super::tests_inscriptions_i5::etat(&ent, etab, &x).await,
            "desiste"
        );
        assert_eq!(
            super::tests_inscriptions_i5::etat(&ent, etab, &y).await,
            "refuse"
        );
        // Une seule place, un seul désigné : Z.
        assert_eq!(
            proposition_ouverte(&ent, etab, &z).await.map(|p| p.1),
            Some("designee".to_string()),
            "{annee} : la place libérée est désignée au suivant restant"
        );
        assert!(proposition_ouverte(&ent, etab, &y).await.is_none());
    }
}

/// Deux déplacements croisés (A passe de l'offre 1 à l'offre 2 pendant que B passe de 2 à 1) : même ordre de verrous.
#[tokio::test]
async fn p3_verrous_deplacements_croises() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (u, r) = dir(&c);
    let etab = c.etablissement_id;
    let (camp, offre1) = campagne_cap(&ent, &c, Some(2), "2027-2028").await;
    let offre2 = ent
        .creer_offre_for_actor(
            etab,
            u,
            r,
            camp,
            &OffreBody {
                niveau_code: "5EME".into(),
                parcours_code: Some("latin".into()),
                libelle_public: "5ᵉ latin".into(),
                capacite: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id;
    let a = dossier(&ent, &c, camp, "DeplaceA", None).await;
    let b = dossier(&ent, &c, camp, "DeplaceB", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre1), "Admis"))
        .await
        .unwrap();
    decider(&ent, &c, &b, &corps("admis", Some(offre2), "Admis"))
        .await
        .unwrap();
    let (mut oa, mut ob) = (offre1, offre2);
    for tour in 0..6 {
        let (vers_a, vers_b) = (ob, oa);
        let (corps_a, corps_b) = (
            corps("admis", Some(vers_a), "Déplacement A"),
            corps("admis", Some(vers_b), "Déplacement B"),
        );
        let (ra, rb) = tokio::join!(
            revoir(&ent, &c, &a, &corps_a),
            revoir(&ent, &c, &b, &corps_b)
        );
        assert!(ra.is_ok(), "tour {tour} : A : {:?}", ra.err());
        assert!(rb.is_ok(), "tour {tour} : B : {:?}", rb.err());
        (oa, ob) = (vers_a, vers_b);
    }
}

/// Deux réutilisations croisées de pièces de foyer (A reprend celle de B pendant que B reprend celle de A).
#[tokio::test]
async fn p3_verrous_reutilisations_croisees() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = super::tests_inscriptions_i3_i4::campagne_avec(
        &ent,
        &c,
        "un_seul",
        &[
            regle(
                "justificatif_domicile",
                "Justificatif de domicile",
                true,
                "dossier",
                vec![],
            ),
            regle(
                "livret_famille",
                "Livret de famille",
                true,
                "dossier",
                vec![],
            ),
        ],
    )
    .await;
    let jeton = super::tests_inscriptions::verifier(&ent, etab, &email("croise")).await;
    let dem = ent
        .deposer_demarche(etab, &jeton, &demarche_deux_enfants(camp, None))
        .await
        .unwrap();
    let (d1, d2) = (dem.dossiers[0].id, dem.dossiers[1].id);
    ent.deposer_reel(etab, &jeton, d1, "livret_famille").await;
    ent.deposer_reel(etab, &jeton, d2, "justificatif_domicile")
        .await;
    let (r1, r2) = tokio::join!(
        ent.reutiliser_piece_famille(etab, &jeton, d2, "livret_famille", d1),
        ent.reutiliser_piece_famille(etab, &jeton, d1, "justificatif_domicile", d2),
    );
    assert!(r1.is_ok(), "d2 reprend le livret de d1 : {:?}", r1.err());
    assert!(
        r2.is_ok(),
        "d1 reprend le justificatif de d2 : {:?}",
        r2.err()
    );
}

// ───────────────────────────── 2. Registre et export ─────────────────────────────

/// Les lignes **réellement exportables** (la condition SQL de `export_si` exécutée) après une décision d'admission
/// programmée, donc non publiée : aucune décision, aucune proposition, aucune condition, aucune ligne interne du suivi.
/// Après la publication, elles apparaissent.
#[tokio::test]
async fn p3_export_ne_rend_que_le_publie() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    ent.definir_conditions_campagne_for_actor(
        etab,
        u,
        r,
        camp,
        &ConditionsCampagneBody {
            codes: vec!["reponse_famille".into(), "contrat_signe".into()],
        },
    )
    .await
    .unwrap();
    let d = dossier(&ent, &c, camp, "Export", None).await;
    let mut b = corps(
        "admis_sous_reserve",
        Some(offre),
        "Admission avec réserve interne",
    );
    b.reserve = Some("Réserve confidentielle de la commission".into());
    b.publier_le = Some(maintenant() + jours(7));
    ent.decider_complete_for_actor(etab, u, r, d.id, &b, false, maintenant())
        .await
        .unwrap();

    let get = |nom: &str| {
        crate::registre_donnees::REGISTRE
            .iter()
            .find(|t| t.nom == nom)
            .unwrap()
    };
    let exportables = |nom: &'static str| {
        let classe = get(nom);
        let cond = classe.export_si.unwrap_or("true");
        format!("SELECT count(*) FROM {nom} x WHERE x.dossier_id = $1 AND ({cond})")
    };
    for nom in [
        "ent_inscription_decision",
        "ent_inscription_proposition_place",
        "ent_inscription_condition",
    ] {
        assert!(
            get(nom).export_si.is_some(),
            "{nom} : une règle de ligne est déclarée"
        );
        assert_eq!(
            compter(&ent, &exportables(nom), d.id).await,
            0,
            "{nom} : rien d'exportable tant que la décision n'est pas publiée"
        );
        // Les lignes existent bel et bien : c'est la règle qui les retient.
        assert!(
            compter(
                &ent,
                &format!("SELECT count(*) FROM {nom} WHERE dossier_id = $1"),
                d.id
            )
            .await
                > 0,
            "{nom} : des lignes existent"
        );
    }
    // Le suivi : jamais une ligne interne, jamais une ligne rattachée à une décision non publiée.
    let suivi = get("ent_inscription_suivi_dossier");
    let cond = suivi.export_si.unwrap();
    assert_eq!(
        compter(
            &ent,
            &format!(
                "SELECT count(*) FROM ent_inscription_suivi_dossier x WHERE x.dossier_id = $1 AND ({cond})
                    AND (NOT x.visible_famille OR (x.decision_id IS NOT NULL AND NOT EXISTS (
                         SELECT 1 FROM ent_inscription_decision d WHERE d.id = x.decision_id AND d.publiee_le IS NOT NULL)))"
            ),
            d.id
        )
        .await,
        0
    );
    // Les colonnes retirées de l'export existent réellement et les conditions s'exécutent : voir `p4_export_chaque_table_condition_executee`
    // (passe 4, M3) et `r_m4_…` ; aucune vérification « la liste contient le nom » ici.
    // Publication à l'échéance : les lignes deviennent exportables.
    assert!(
        ent.publier_decisions_echues_tache(maintenant() + jours(8))
            .await
            .unwrap()
            >= 1
    );
    assert_eq!(
        compter(&ent, &exportables("ent_inscription_decision"), d.id).await,
        1
    );
    assert_eq!(
        compter(
            &ent,
            &exportables("ent_inscription_proposition_place"),
            d.id
        )
        .await,
        1
    );
    let conditions = compter(
        &ent,
        "SELECT count(*) FROM ent_inscription_condition WHERE dossier_id = $1",
        d.id,
    )
    .await;
    assert!(conditions >= 2);
    assert_eq!(
        compter(&ent, &exportables("ent_inscription_condition"), d.id).await,
        conditions
    );
}

// ───────────────────────────── 3. Publication caduque ─────────────────────────────

/// Admission programmée à J+7, dossier annulé à J+3 : à J+7 rien n'est publié ni notifié, la caducité est tracée, aucun
/// événement `ADMISSION_DECIDEE` n'est émis.
#[tokio::test]
async fn p3_publication_caduque_dossier_annule_avant_l_echeance() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Caduque", None).await;
    let mut b = corps("admis", Some(offre), "Admission programmée");
    b.publier_le = Some(maintenant() + jours(7));
    let decision = ent
        .decider_complete_for_actor(etab, u, r, d.id, &b, false, maintenant())
        .await
        .unwrap();
    // J+3 : le dossier est annulé.
    ent.transition_for_actor(
        etab,
        u,
        r,
        d.id,
        &TransitionBody {
            action: "annuler".into(),
            motif: "Annulation demandée".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        super::tests_inscriptions_i5::etat(&ent, etab, &d).await,
        "annule"
    );
    expedier(&ent).await;
    let avant = courriels_vers(&ent, &d.email);
    // J+7 : la tâche de fond.
    let publiees = ent
        .publier_decisions_echues_tache(maintenant() + jours(7))
        .await
        .unwrap();
    assert_eq!(publiees, 0, "rien n'est publié");
    let (publiee_le, caduque_le, motif): (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT publiee_le, caduque_le, caduque_motif FROM ent_inscription_decision WHERE id = $1",
    )
    .bind(decision.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(publiee_le.is_none(), "jamais publiée");
    assert_eq!(
        caduque_le,
        Some(maintenant() + jours(7)),
        "la caducité est datée par l'instant de la tâche"
    );
    assert!(
        motif.as_deref().is_some_and(|m| m.contains("annule")),
        "motif tracé : {motif:?}"
    );
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_outbox WHERE aggregate_id = $1 AND type = 'admission.decidee'",
            d.id
        )
        .await,
        0,
        "aucun événement ADMISSION_DECIDEE"
    );
    expedier(&ent).await;
    assert_eq!(
        courriels_vers(&ent, &d.email),
        avant,
        "la famille n'est pas notifiée"
    );
    let vue = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    assert!(!vue.publiee, "la famille ne lit rien de cette décision");
    // Une seconde passe ne la reprend pas, et la publication manuelle est refusée (caduque).
    assert_eq!(
        ent.publier_decisions_echues_tache(maintenant() + jours(8))
            .await
            .unwrap(),
        0
    );
    let manuelle = ent
        .publier_decision_for_actor(
            etab,
            u,
            r,
            decision.id,
            &PublierBody::default(),
            maintenant() + jours(8),
        )
        .await;
    assert!(
        matches!(manuelle, Err(EntDbError::ConflitDossier(_))),
        "{:?}",
        manuelle.err()
    );
}

// ───────────────────────────── 4. Séquence de l'ordre d'attente ─────────────────────────────

/// Un dossier **repris** d'un ordre élevé, puis un nouvel entrant : le nouvel entrant passe derrière, côté famille
/// (`ordre_attente`) comme côté désignation (`rang_attente`).
#[tokio::test]
async fn p3_sequence_ordre_attente_reprise_puis_nouvel_entrant() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Plein", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let a = dossier(&ent, &c, camp, "Reprise", None).await;
    decider(
        &ent,
        &c,
        &a,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    // Le dossier A est « repris » avec un ordre bien au-dessus de la séquence (ce que 313 recopie depuis le rang).
    let (courant,): (i64,) =
        sqlx::query_as("SELECT last_value FROM ent_inscription_attente_ordre_seq")
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    let repris = courant + 40;
    sqlx::query("UPDATE ent_inscription_dossier SET ordre_attente = $2 WHERE id = $1")
        .bind(a.id)
        .bind(repris)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // L'instruction de 314 qui avance la séquence (la seule, rejouée seule : la séquence ne recule jamais).
    let sql = include_str!("../../../migrations/314_inscription_decision_i5_passe3.sql");
    let setval = crate::migrate::ordres(sql)
        .into_iter()
        .find(|o| o.contains("setval('ent_inscription_attente_ordre_seq'"))
        .expect("314 avance la séquence");
    sqlx::query(&setval)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let b = dossier(&ent, &c, camp, "Entrant", None).await;
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    let ordre_b: i64 =
        sqlx::query_scalar("SELECT ordre_attente FROM ent_inscription_dossier WHERE id = $1")
            .bind(b.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(
        ordre_b > repris,
        "le nouvel entrant ({ordre_b}) passe derrière le dossier repris ({repris})"
    );
    // Côté famille.
    assert_eq!(rang_vu(&ent, etab, &a).await, Some(1));
    assert_eq!(rang_vu(&ent, etab, &b).await, Some(2));
    // Côté désignation : la place libérée va à A, jamais au nouvel entrant.
    revoir(
        &ent,
        &c,
        &plein,
        &corps("refuse", Some(offre), "Désistement"),
    )
    .await
    .unwrap();
    assert_eq!(
        proposition_ouverte(&ent, etab, &a).await.map(|p| p.1),
        Some("designee".to_string())
    );
    assert!(proposition_ouverte(&ent, etab, &b).await.is_none());
}

// ───────────────────────────── 5. Écarter une proposition ─────────────────────────────

/// On n'écarte que la désignation de la liste d'attente sans décision : une place rattachée à une décision se défait
/// par une nouvelle décision (refus exact, variante explicite).
#[tokio::test]
async fn p3_ecarter_seulement_une_designation_de_la_liste() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Rattachee", None).await;
    let mut b = corps("admis", Some(offre), "Admission programmée");
    b.publier_le = Some(maintenant() + jours(7));
    let decision = ent
        .decider_complete_for_actor(etab, u, r, d.id, &b, false, maintenant())
        .await
        .unwrap();
    let (prop, statut) = proposition_ouverte(&ent, etab, &d)
        .await
        .expect("proposition désignée de la décision");
    assert_eq!(statut, "designee");
    let motif = MotifBody {
        motif: "Veut passer son tour".into(),
    };
    let refus = ent
        .ecarter_proposition_for_actor(etab, u, r, prop, &motif, maintenant())
        .await;
    match &refus {
        Err(EntDbError::ConflitDossier(m)) => assert_eq!(
            *m,
            "cette place est rattachée à une décision : passer par une nouvelle décision pour la modifier"
        ),
        autre => panic!("refus exact attendu (conflit de dossier), obtenu {autre:?}"),
    }
    // Rien n'a bougé : la décision reste publiable et sa proposition aussi.
    let (statut_apres,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_inscription_proposition_place WHERE id = $1")
            .bind(prop)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut_apres, "designee");
    assert!(
        ent.publier_decisions_echues_tache(maintenant() + jours(8))
            .await
            .unwrap()
            >= 1
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_inscription_proposition_place WHERE dossier_id = $1 AND statut = 'envoyee'",
            d.id
        )
        .await,
        1,
        "la décision publiée a bien sa proposition envoyée ({})",
        decision.id
    );
}

// ───────────────────────────── 6. Réparation de chaîne, gardes ─────────────────────────────

/// 314 rechaîne un historique partiel (d1 racine, d2 remplace d1, d3 racine) dans l'ordre (`decide_le`, puis `id`), pose
/// les index uniques, et ses fonctions de garde ont un `search_path` fixé. Tout dans une transaction annulée.
#[tokio::test]
async fn p3_314_repare_une_chaine_partielle() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, _offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Partielle", None).await;
    let verrou = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    for ordre in [
        "ALTER TABLE ent_inscription_decision DISABLE TRIGGER ent_inscription_decision_ajout_seul_trg",
        "DROP TRIGGER IF EXISTS ent_inscription_decision_ecarte_trg ON ent_inscription_decision",
        "DROP INDEX IF EXISTS ent_inscription_decision_racine_uq",
        "DROP INDEX IF EXISTS ent_inscription_decision_successeur_uq",
    ] {
        sqlx::query(ordre).execute(&mut *tx).await.unwrap_or_else(|e| panic!("{ordre} : {e}"));
    }
    // Fixture d'ancien état : la sentinelle (316) n'existait pas ; elle est neutralisée le temps de poser l'historique.
    sqlx::query("ALTER TABLE ent_inscription_decision DISABLE TRIGGER ent_inscription_sentinelle_decision_trg")
        .execute(&mut *tx)
        .await
        .ok();
    let (d1, d2, d3) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    for (id, remplace, minute) in [(d1, None, 1), (d2, Some(d1), 2), (d3, None, 3)] {
        sqlx::query(
            "INSERT INTO ent_inscription_decision
                (id, etablissement_id, dossier_id, sens, motif_interne, remplace_decision_id, decide_le, publiee_le)
             VALUES ($1,$2,$3,'ajourne','Historique partiel',$4, '2027-01-01T10:00:00Z'::timestamptz + make_interval(mins => $5), now())",
        )
        .bind(id)
        .bind(etab)
        .bind(d.id)
        .bind(remplace)
        .bind(minute)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("ALTER TABLE ent_inscription_decision ENABLE TRIGGER ent_inscription_decision_ajout_seul_trg")
        .execute(&mut *tx)
        .await
        .unwrap();
    let sql = include_str!("../../../migrations/314_inscription_decision_i5_passe3.sql");
    for ordre in crate::migrate::ordres(sql) {
        sqlx::query(&ordre)
            .execute(&mut *tx)
            .await
            .unwrap_or_else(|e| panic!("314 refusée : {e}\n{ordre}"));
    }
    // Ordre de chaîne vérifié : d1 racine, d2 remplace d1, d3 remplace d2 — et les historiques déjà chaînés d'avant
    // (la décision du dossier de test, seule du dossier voisin) ne sont pas touchés.
    let lignes: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT id, remplace_decision_id FROM ent_inscription_decision WHERE dossier_id = $1 ORDER BY decide_le, id",
    )
    .bind(d.id)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(lignes, vec![(d1, None), (d2, Some(d1)), (d3, Some(d2))]);
    // Les index uniques sont revenus (et tiennent).
    for index in [
        "ent_inscription_decision_racine_uq",
        "ent_inscription_decision_successeur_uq",
    ] {
        let n: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(index)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert!(n.is_some(), "{index} posé par 314");
    }
    // Les fonctions de garde : `search_path` fixé, table temporaire homonyme sans effet.
    let sans_chemin: Vec<String> = sqlx::query_scalar(
        "SELECT p.proname::text FROM pg_proc p
          WHERE p.proname IN ('ent_inscription_ajout_seul', 'ent_inscription_decision_chaine', 'ent_inscription_membre_lie',
                              'ent_inscription_ecarte_garde', 'ent_inscription_en_liste_visible', 'ent_inscription_purge_voulue')
            AND NOT COALESCE(p.proconfig::text[] @> ARRAY['search_path=pg_catalog, public, pg_temp'], false)",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert!(
        sans_chemin.is_empty(),
        "fonctions sans search_path fixé : {sans_chemin:?}"
    );
    sqlx::query("CREATE TEMP TABLE ent_inscription_decision (id uuid, dossier_id uuid, etablissement_id uuid, remplace_decision_id uuid)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let nouvelle = sqlx::query(
        "INSERT INTO public.ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne, remplace_decision_id)
         VALUES ($1,$2,$3,'ajourne','Hors du dossier',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(Uuid::new_v4())
    .execute(&mut *tx)
    .await;
    // `ent_inscription_decision_chaine` doit lire la **vraie** table : le prédécesseur inconnu est refusé (check_violation),
    // la table temporaire homonyme ne la trompe pas.
    assert_eq!(
        sqlstate(&nouvelle),
        "23514",
        "garde de chaîne : {nouvelle:?}"
    );
    tx.rollback().await.unwrap();
    verrou.liberer().await;
}

// ───────────────────────────── D-069 F15 : acceptation au guichet ─────────────────────────────

/// Acceptation au guichet : droit (403), refus distincts selon la proposition (`designee`, `expiree`, `annulee`,
/// `declinee`, déjà répondue), acteur écarté (introuvable), refus après `repondre_avant` tant que l'expiration n'est pas
/// constatée ; l'acceptation passe la proposition à `acceptee` (plus d'action « répondre » côté famille) et notifie.
#[tokio::test]
async fn p3_guichet_refus_distincts_notification_et_delai() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let motif = || MotifBody {
        motif: "La famille est venue au guichet".into(),
    };
    let conflit_exact = |res: &Result<PropositionDto, EntDbError>, attendu: &str| match res {
        Err(EntDbError::ConflitDossier(m)) => assert_eq!(*m, attendu),
        autre => panic!("refus exact attendu « {attendu} », obtenu {autre:?}"),
    };
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    // — Proposition `designee` (place libérée, pas encore envoyée).
    let plein = dossier(&ent, &c, camp, "Plein", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let suivant = dossier(&ent, &c, camp, "Suivant", None).await;
    decider(
        &ent,
        &c,
        &suivant,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    revoir(
        &ent,
        &c,
        &plein,
        &corps("refuse", Some(offre), "Désistement"),
    )
    .await
    .unwrap();
    let (p_designee, statut) = proposition_ouverte(&ent, etab, &suivant).await.unwrap();
    assert_eq!(statut, "designee");
    // 403 sans droit (un professeur n'a pas `inscriptions.gerer`).
    let prof = agent_role(&ent, etab, &email("prof"), "prof").await;
    let r403 = ent
        .accepter_proposition_au_guichet_for_actor(
            etab,
            prof,
            "prof",
            p_designee,
            &motif(),
            maintenant(),
        )
        .await;
    assert!(
        matches!(r403, Err(EntDbError::Forbidden)),
        "403 exact : {:?}",
        r403.err()
    );
    let res = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_designee, &motif(), maintenant())
        .await;
    conflit_exact(
        &res,
        "place désignée, pas encore envoyée à la famille : rien à accepter",
    );
    // Acteur écarté : introuvable (même avec le droit).
    let lie = agent_direction(&ent, etab, &suivant.email).await;
    let res = ent
        .accepter_proposition_au_guichet_for_actor(
            etab,
            lie,
            "direction",
            p_designee,
            &motif(),
            maintenant(),
        )
        .await;
    assert!(
        matches!(
            &res,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{:?}",
        res.err()
    );
    // — Envoyée : après `repondre_avant`, refus tant que l'expiration n'est pas constatée.
    ent.envoyer_proposition_for_actor(
        etab,
        u,
        r,
        p_designee,
        &EnvoyerPropositionBody {
            confirmer: true,
            ..Default::default()
        },
        maintenant(),
    )
    .await
    .unwrap();
    let repondre_avant: chrono::NaiveDate = sqlx::query_scalar(
        "SELECT repondre_avant FROM ent_inscription_proposition_place WHERE id = $1",
    )
    .bind(p_designee)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let apres_delai = repondre_avant
        .succ_opt()
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    let res = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_designee, &motif(), apres_delai)
        .await;
    conflit_exact(
        &res,
        "le délai de réponse est dépassé : l'expiration est à constater avant toute autre suite",
    );
    // — Expiration constatée : `expiree`.
    ent.constater_expiration_for_actor(etab, u, r, p_designee, apres_delai)
        .await
        .unwrap();
    let res = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_designee, &motif(), apres_delai)
        .await;
    conflit_exact(
        &res,
        "proposition expirée : le délai de réponse a été constaté dépassé",
    );
    // — `declinee` : la famille décline.
    let b = dossier(&ent, &c, camp, "Decline", None).await;
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    ent.designer_suivants_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    let (p_b, _) = proposition_ouverte(&ent, etab, &b)
        .await
        .expect("désigné après l'expiration");
    ent.envoyer_proposition_for_actor(
        etab,
        u,
        r,
        p_b,
        &EnvoyerPropositionBody {
            confirmer: true,
            ..Default::default()
        },
        maintenant(),
    )
    .await
    .unwrap();
    ent.repondre_proposition_famille(
        etab,
        &b.jeton,
        b.id,
        &ReponsePropositionBody {
            reponse: "declinee".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    let res = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_b, &motif(), maintenant())
        .await;
    conflit_exact(&res, "proposition déjà répondue");
    // — `annulee` : la place désignée au suivant est reprise par une admission directe.
    let e = dossier(&ent, &c, camp, "Annulee", None).await;
    decider(
        &ent,
        &c,
        &e,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    ent.designer_suivants_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    let (p_e, _) = proposition_ouverte(&ent, etab, &e).await.expect("désigné");
    let mut tx = super::tests_inscriptions::tx_sous_verrou_offres(&ent, &[offre]).await;
    sqlx::query("UPDATE ent_inscription_proposition_place SET statut = 'annulee' WHERE id = $1")
        .bind(p_e)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let res = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_e, &motif(), maintenant())
        .await;
    conflit_exact(&res, "proposition annulée ou écartée : rien à accepter");
    // — Acceptation réussie : `acceptee`, plus d'action « répondre », famille notifiée, motif masqué sans le droit de décider.
    let f = dossier(&ent, &c, camp, "Guichet", None).await;
    let mut tx = super::tests_inscriptions::tx_sous_verrou_offres(&ent, &[offre]).await;
    sqlx::query("UPDATE ent_inscription_offre SET capacite = 5 WHERE id = $1")
        .bind(offre)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    decider(&ent, &c, &f, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let (p_f, statut) = proposition_ouverte(&ent, etab, &f).await.unwrap();
    assert_eq!(statut, "envoyee");
    let avant = courriels_vers(&ent, &f.email);
    let vue_avant = ent.decision_famille(etab, &f.jeton, f.id).await.unwrap();
    assert!(vue_avant.actions.contains(&"repondre".to_string()));
    let dto = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_f, &motif(), maintenant())
        .await
        .unwrap();
    assert_eq!(dto.statut, "acceptee");
    assert!(
        dto.motif.is_none(),
        "le secrétariat sans `inscriptions.decider` ne lit pas le motif de la proposition"
    );
    let vue_apres = ent.decision_famille(etab, &f.jeton, f.id).await.unwrap();
    assert!(
        !vue_apres.actions.contains(&"repondre".to_string()),
        "plus d'action « répondre » côté famille"
    );
    assert_eq!(
        vue_apres.proposition.map(|p| p.statut),
        Some("acceptee".to_string())
    );
    expedier(&ent).await;
    assert!(
        courriels_vers(&ent, &f.email) > avant,
        "la famille est prévenue"
    );
    let encore = ent
        .accepter_proposition_au_guichet_for_actor(etab, a, ar, p_f, &motif(), maintenant())
        .await;
    conflit_exact(&encore, "proposition déjà répondue");
}

// ───────────────────────────── Faibles ─────────────────────────────

/// Le motif d'une proposition n'est lu qu'avec `inscriptions.decider` ; les dossiers écartés sont retirés **avant** la limite.
#[tokio::test]
async fn p3_motif_masque_et_ecartes_avant_la_limite() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Plein", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let x = dossier(&ent, &c, camp, "Tour", None).await;
    decider(
        &ent,
        &c,
        &x,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    revoir(
        &ent,
        &c,
        &plein,
        &corps("refuse", Some(offre), "Désistement"),
    )
    .await
    .unwrap();
    let (p, _) = proposition_ouverte(&ent, etab, &x).await.unwrap();
    ent.ecarter_proposition_for_actor(
        etab,
        u,
        r,
        p,
        &MotifBody {
            motif: "Tour passé, motif interne".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    let vue_decideur = ent
        .propositions_for_actor(etab, u, r, Some(camp), Some("ecartee".into()), maintenant())
        .await
        .unwrap();
    assert!(vue_decideur
        .iter()
        .any(|p| p.motif.as_deref() == Some("Tour passé, motif interne")));
    let vue_secretariat = ent
        .propositions_for_actor(
            etab,
            a,
            ar,
            Some(camp),
            Some("ecartee".into()),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(vue_secretariat.len(), vue_decideur.len());
    assert!(
        vue_secretariat.iter().all(|p| p.motif.is_none()),
        "motif masqué hors `inscriptions.decider`"
    );
    // Un membre lié au candidat (adresse du déposant) ne voit pas sa proposition, et ne la compte pas dans la limite.
    let lie = agent_direction(&ent, etab, &x.email).await;
    let vue_lie = ent
        .propositions_for_actor(etab, lie, "direction", Some(camp), None, maintenant())
        .await
        .unwrap();
    assert!(
        vue_lie.iter().all(|p| p.dossier_id != x.id),
        "dossier écarté : aucune ligne"
    );
}

/// `liberations` augmente du nombre de places ajoutées : +3 de capacité = 3 numéros de place.
#[tokio::test]
async fn p3_liberations_autant_que_de_places() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let lire = || async {
        sqlx::query_scalar::<_, i64>("SELECT liberations FROM ent_inscription_offre WHERE id = $1")
            .bind(offre)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap()
    };
    let avant = lire().await;
    let _ = camp;
    ent.modifier_offre_for_actor(
        etab,
        u,
        r,
        offre,
        &OffreBody {
            niveau_code: "6EME".into(),
            libelle_public: "6ᵉ".into(),
            capacite: Some(4),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        lire().await,
        avant + 3,
        "+3 places : trois numéros de place"
    );
}
