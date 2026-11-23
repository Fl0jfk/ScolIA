//! Socle 3, lot I5 — **correctifs de revue** (D-069), contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Aucun test ne dépend de l'heure réelle : les instants passés au service sont posés (2027) ; les comparaisons
//! de chaînes de décisions se font par la chaîne `remplace_decision_id`, jamais par l'heure.
//!
//! | Correctif | Test(s) |
//! |---|---|
//! | E1 règle 1 : une décision non publiée n'existe pas pour la famille, ni hier ni demain | `r_e1_refus_non_publie_jamais_visible` |
//! | E2/M1/M10 règle 2 : membre lié écarté d'office, toutes les routes à identifiant de dossier | `r_e2_membre_lie_ecarte_de_toutes_les_routes` |
//! | E2 : en séance, seul un membre de la commission décide ; F3 membres validés, plafonds, publication bornée | `r_e2_seance_membres_et_plafonds` |
//! | M2 : actions de la famille sur l'état visible | `r_m2_actions_famille_sur_etat_visible` |
//! | M3 règle 3 : validation après décision publiée et place acceptée | `r_m3_validation_suit_decision_publiee_et_acceptee` |
//! | M4/M9 règle 4 : chaîne, concurrence, attaques SQL en base | `r_m4_chaine_et_concurrence`, `r_m9_ajout_seul_attaques_sql` |
//! | M5 : admission directe sur une place désignée | `r_m5_admission_directe_sur_place_designee` |
//! | M6/M7 : proposition envoyée jamais réécrite, levée journalisée | `r_m6_m7_proposition_envoyee_et_levee_journalisee` |
//! | M8 : motif interne réservé à `inscriptions.decider` | `r_m8_motif_interne_reserve_au_decideur` |
//! | M10 : deux envois, acceptation contre expiration | `r_m10_concurrence_envois_et_reponse` |
//! | M11 : passer son tour ne vaut que pour la place concernée | `r_m11_passer_son_tour_une_place` |
//! | F2 : rang de la famille sur les seules décisions publiées | `r_f2_rang_famille_decisions_publiees` |
//! | F6 : aucune erreur technique dans le récapitulatif de lot | `r_f6_recapitulatif_sans_erreur_technique` |

use domain_eleve::DomainError;
use ent_types::inscriptions::*;
use ent_types::inscriptions_decisions::*;
use ent_types::inscriptions_pieces::*;
use uuid::Uuid;

use super::tests_inscriptions::{adm, dir, labo};
use super::tests_inscriptions_i5::{
    campagne_cap, corps, decider, dossier, email, expedier, maintenant, proposition_ouverte,
    revoir, t,
};
use super::*;

/// Refus **exact** : « introuvable » (le dossier n'existe pas pour le membre écarté), avec un compte qui a le droit.
macro_rules! refus_introuvable {
    ($nom:expr, $e:expr) => {{
        let r = $e.await;
        assert!(
            matches!(&r, Err(EntDbError::Domain(DomainError::NotFound))),
            "{} : refus exact attendu (introuvable), obtenu {:?}",
            $nom,
            r.as_ref().err()
        );
    }};
}

/// Membre du personnel (rôle direction : tous les droits d'inscription) créé pour le test, avec cette adresse.
pub(super) async fn agent_direction(ent: &PgEnt, etab: Uuid, mail: &str) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1,$2,'x','Agent de test', now(), now())",
    )
    .bind(user)
    .bind(mail)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1,$2,'direction', now())",
    )
    .bind(user)
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    user
}

pub(super) async fn nombre(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// Code SQLSTATE d'un refus de la base (D-069 règle 3 : un refus se prouve par son code exact, jamais par « une erreur »).
pub(super) fn sqlstate<T>(r: &Result<T, sqlx::Error>) -> String {
    match r {
        Err(sqlx::Error::Database(d)) => d.code().map(|c| c.to_string()).unwrap_or_default(),
        Err(autre) => format!("autre erreur : {autre}"),
        Ok(_) => "aucune erreur".to_string(),
    }
}

/// `check_violation` : refus des gardes de la base (écart, ajout seul).
const CHECK_VIOLATION: &str = "23514";
/// `unique_violation` : une racine par dossier, un seul successeur par décision.
const UNIQUE_VIOLATION: &str = "23505";

fn conflit_exact(e: &EntDbError, attendu: &str) -> bool {
    matches!(e, EntDbError::ConflitDossier(m) if *m == attendu)
}

// ───────────────────────────── E1 ─────────────────────────────

/// Un refus pris mais jamais publié, puis revu en « admis » et publié : la famille ne voit jamais le refus, ni son
/// motif, ni la ligne de suivi qu'il a écrite.
#[tokio::test]
async fn r_e1_refus_non_publie_jamais_visible() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Refusrevu", None).await;
    let mut b = corps("refuse", Some(offre), "Dossier incomplet de fond");
    b.motif_communique = Some("MOTIF-DU-REFUS".into());
    b.publier_le = Some(t("2027-03-10T09:00:00Z"));
    decider(&ent, &c, &d, &b).await.unwrap();
    let mut b2 = corps("admis", Some(offre), "Revu : place trouvée");
    b2.motif_communique = Some("Bienvenue".into());
    revoir(&ent, &c, &d, &b2).await.unwrap();

    let vue = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    assert_eq!(vue.sens.as_deref(), Some("admis"));
    let json =
        serde_json::to_string(&ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap()).unwrap();
    assert!(
        !json.contains("MOTIF-DU-REFUS") && !json.contains("\"refuse\""),
        "la famille ne voit jamais le refus : {json}"
    );
    let espace =
        serde_json::to_string(&ent.espace_preinscription(etab, &d.jeton).await.unwrap()).unwrap();
    assert!(!espace.contains("\"refuse\""), "{espace}");
    // Les lignes de suivi écrites par la décision non publiée restent rattachées à cette décision.
    let rattachees = nombre(
        &ent,
        "SELECT count(*) FROM ent_inscription_suivi_dossier s
          WHERE s.dossier_id = $1 AND s.decision_id IS NOT NULL AND s.statut_apres = 'refuse'",
        d.id,
    )
    .await;
    assert_eq!(rattachees, 1);
}

// ───────────────────────────── E2 / M1 / M10 ─────────────────────────────

/// Un membre lié au candidat (adresse du déposant, ou d'un second responsable) est écarté **d'office** : sur chaque
/// route à identifiant de dossier, le refus est exactement « introuvable », avec un compte qui a le droit.
#[tokio::test]
async fn r_e2_membre_lie_ecarte_de_toutes_les_routes() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let second = email("second");
    let d = dossier(&ent, &c, camp, "Lie", Some(&second)).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    // Le lien : deux membres du personnel, l'un à l'adresse du déposant, l'autre à celle du second responsable.
    for (variante, mail) in [
        ("deposant", d.email.clone()),
        ("second responsable", second.clone()),
    ] {
        let u = agent_direction(&ent, etab, &mail).await;
        let r = "direction";
        let id = d.id;
        let nom = |route: &str| format!("{route} ({variante})");

        refus_introuvable!(nom("dossier"), ent.dossier_for_actor(etab, u, r, id));
        refus_introuvable!(
            nom("statut"),
            ent.changer_statut_for_actor(
                etab,
                u,
                r,
                id,
                &StatutBody {
                    statut: "en_instruction".into(),
                    motif: None
                }
            )
        );
        refus_introuvable!(
            nom("piece"),
            ent.decider_piece_for_actor(etab, u, r, id, "livret", &PieceDecisionBody::default())
        );
        refus_introuvable!(
            nom("commentaire"),
            ent.commenter_for_actor(
                etab,
                u,
                r,
                id,
                &CommentaireBody {
                    texte: "note".into(),
                    visible_famille: false
                }
            )
        );
        refus_introuvable!(nom("relancer"), ent.relancer_for_actor(etab, u, r, id));
        refus_introuvable!(
            nom("decision (ancienne route)"),
            ent.decider_for_actor(
                etab,
                u,
                r,
                id,
                &DecisionBody {
                    decision: "admis".into(),
                    ..Default::default()
                }
            )
        );
        refus_introuvable!(
            nom("decision avancée"),
            ent.decider_avance_for_actor(
                etab,
                u,
                r,
                id,
                &DecisionAvanceeBody {
                    decision: "refuse".into(),
                    motif: Some("Motif".into()),
                    ..Default::default()
                }
            )
        );
        refus_introuvable!(
            nom("valider"),
            ent.valider_inscription_for_actor(etab, u, r, id, &ValidationBody::default())
        );
        refus_introuvable!(
            nom("transition"),
            ent.transition_for_actor(
                etab,
                u,
                r,
                id,
                &TransitionBody {
                    action: "sans_suite".into(),
                    motif: "Motif de test".into()
                }
            )
        );
        refus_introuvable!(
            nom("blocage"),
            ent.blocage_for_actor(etab, u, r, id, &BlocageBody::default())
        );
        refus_introuvable!(
            nom("ne pas solliciter"),
            ent.ne_pas_solliciter_for_actor(etab, u, r, id, &NePasSolliciterBody::default())
        );
        refus_introuvable!(
            nom("annuler la validation"),
            ent.annuler_validation_for_actor(etab, u, r, id, &AnnulationValidationBody::default())
        );
        refus_introuvable!(
            nom("correspondances"),
            ent.correspondances_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("accords"),
            ent.accords_dossier_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("demander accords"),
            ent.demander_accords_for_actor(etab, u, r, id, &DemanderAccordsBody::default())
        );
        refus_introuvable!(
            nom("accord papier"),
            ent.saisir_accord_papier_for_actor(
                etab,
                u,
                r,
                id,
                &AccordPapierBody {
                    rang: 1,
                    reponse: "donne".into(),
                    piece_code: None
                }
            )
        );
        refus_introuvable!(
            nom("désaccord"),
            ent.declarer_desaccord_for_actor(
                etab,
                u,
                r,
                id,
                &DesaccordBody {
                    rang: 1,
                    motif: "Motif".into()
                }
            )
        );
        refus_introuvable!(
            nom("dispenser"),
            ent.dispenser_accord_for_actor(
                etab,
                u,
                r,
                id,
                &DispenseAccordBody {
                    rang: 1,
                    motif: "Motif".into(),
                    piece_code: None
                }
            )
        );
        refus_introuvable!(
            nom("lever désaccord"),
            ent.lever_desaccord_for_actor(
                etab,
                u,
                r,
                id,
                &LeverDesaccordBody {
                    rang: 1,
                    issue: "accord_ecrit".into(),
                    piece_code: "accord_responsable".into(),
                    motif: "Motif".into()
                }
            )
        );
        refus_introuvable!(
            nom("pièces attendues"),
            ent.pieces_dossier_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("contrôle d'une pièce"),
            ent.controler_piece_inscription_for_actor(
                etab,
                u,
                r,
                id,
                "livret",
                &ControlePieceBody::default()
            )
        );
        refus_introuvable!(
            nom("lecture d'une pièce"),
            ent.lire_piece_inscription_for_actor(etab, u, r, id, "livret")
        );
        refus_introuvable!(
            nom("dépôt au guichet"),
            ent.deposer_fichiers_guichet_for_actor(etab, u, r, id, "livret", vec![])
        );
        refus_introuvable!(
            nom("réutiliser une pièce"),
            ent.reutiliser_piece_dossier_for_actor(etab, u, r, id, "livret", Uuid::new_v4())
        );
        refus_introuvable!(
            nom("décisions"),
            ent.decisions_dossier_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("décider"),
            ent.decider_complete_for_actor(
                etab,
                u,
                r,
                id,
                &corps("admis", Some(offre), "Admis"),
                true,
                maintenant()
            )
        );
        refus_introuvable!(
            nom("avis (donner)"),
            ent.donner_avis_for_actor(
                etab,
                u,
                r,
                id,
                &AvisBody {
                    contexte: "entretien".into(),
                    sens: "favorable".into(),
                    texte: "Très bien".into()
                }
            )
        );
        refus_introuvable!(
            nom("avis (lire)"),
            ent.avis_dossier_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("déport (état)"),
            ent.deport_etat_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("déport (se déporter)"),
            ent.se_deporter_for_actor(
                etab,
                u,
                r,
                id,
                &DeportBody {
                    motif: "Parent du candidat".into()
                }
            )
        );
        refus_introuvable!(
            nom("conditions"),
            ent.conditions_dossier_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("réévaluer"),
            ent.reevaluer_conditions_for_actor(etab, u, r, id)
        );
        refus_introuvable!(
            nom("remplir une condition"),
            ent.remplir_condition_for_actor(
                etab,
                u,
                r,
                id,
                "contrat_signe",
                &RemplirConditionBody::default(),
                maintenant()
            )
        );
        refus_introuvable!(
            nom("lever une condition"),
            ent.lever_condition_for_actor(
                etab,
                u,
                r,
                id,
                "contrat_signe",
                &MotifBody {
                    motif: "Motif".into()
                },
                maintenant()
            )
        );
        // Une décision rattachée au dossier (courrier), une proposition désignée.
        let decision_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM ent_inscription_decision WHERE dossier_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        refus_introuvable!(
            nom("courrier"),
            ent.courrier_decision_for_actor(etab, u, r, decision_id)
        );
        refus_introuvable!(
            nom("publier"),
            ent.publier_decision_for_actor(
                etab,
                u,
                r,
                decision_id,
                &PublierBody::default(),
                maintenant()
            )
        );
        // Et la lecture de la liste ne le montre pas non plus.
        let liste = ent
            .list_dossiers_for_actor(etab, u, r, Some(camp), None)
            .await
            .unwrap();
        assert!(
            liste.iter().all(|x| x.id != id),
            "{variante} : le dossier est absent de ses listes"
        );
        // Le second verrou, en base : ni avis ni décision d'un membre lié, même par SQL direct.
        let avis = sqlx::query(
            "INSERT INTO ent_inscription_avis (id, etablissement_id, dossier_id, auteur_id, contexte, sens, texte)
             VALUES ($1,$2,$3,$4,'entretien','favorable','Texte')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(u)
        .execute(ent.pool_verif())
        .await;
        assert_eq!(
            sqlstate(&avis),
            CHECK_VIOLATION,
            "{variante} : avis refusé en base (garde d'écart)"
        );
        let decision = sqlx::query(
            "INSERT INTO ent_inscription_decision
                (id, etablissement_id, dossier_id, sens, motif_interne, decide_par, remplace_decision_id)
             VALUES ($1,$2,$3,'refuse','Motif de test',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(u)
        .bind(decision_id)
        .execute(ent.pool_verif())
        .await;
        assert_eq!(
            sqlstate(&decision),
            CHECK_VIOLATION,
            "{variante} : décision refusée en base (garde d'écart)"
        );
    }
    // Un membre sans lien, lui, accède au dossier (le refus n'est pas général).
    let (u, r) = dir(&c);
    assert!(ent.dossier_for_actor(etab, u, r, d.id).await.is_ok());
    // Un déport déclaré marque les avis déjà donnés.
    let (a, ar) = adm(&c);
    ent.donner_avis_for_actor(
        etab,
        a,
        ar,
        d.id,
        &AvisBody {
            contexte: "entretien".into(),
            sens: "reserve".into(),
            texte: "Avis avant déport".into(),
        },
    )
    .await
    .unwrap();
    ent.se_deporter_for_actor(
        etab,
        a,
        ar,
        d.id,
        &DeportBody {
            motif: "Connaissance".into(),
        },
    )
    .await
    .unwrap();
    let avis = ent.avis_dossier_for_actor(etab, u, r, d.id).await.unwrap();
    assert!(avis.iter().any(|x| x.auteur_id == a && x.auteur_deporte));
}

/// En séance, seul un membre de la commission décide ; les membres sont des personnels de l'établissement ;
/// plafonds et publication bornés (F3).
#[tokio::test]
async fn r_e2_seance_membres_et_plafonds() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Seance", None).await;
    // Un membre qui n'est pas du personnel (compte de famille) : refusé à la création.
    let (_, parent) =
        super::tests_administratif_a0::personne_avec_compte(&ent, etab, "Parent", &email("parent"))
            .await;
    let base = CommissionBody {
        campagne_id: camp,
        intitule: "Commission de test".into(),
        seance_le: t("2027-03-20T14:00:00Z"),
        membres: vec![parent],
        dossier_ids: vec![d.id],
    };
    assert!(matches!(
        ent.creer_commission_admission_for_actor(etab, u, r, &base)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Un membre inconnu de l'établissement : refusé aussi, du même refus exact.
    assert!(matches!(
        ent.creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                membres: vec![Uuid::new_v4()],
                ..base.clone()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // La direction n'est pas membre de cette séance : elle n'y décide pas (refus exact, droit présent).
    let autre = agent_direction(&ent, etab, &email("membre")).await;
    let commission = ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                membres: vec![autre],
                ..base.clone()
            },
        )
        .await
        .unwrap();
    let lot = DecisionsEnLotBody {
        decisions: vec![DecisionEnLotItem {
            dossier_id: d.id,
            decision: corps("liste_attente", Some(offre), "Complet"),
        }],
    };
    let refus = ent
        .decider_en_lot_for_actor(etab, u, r, commission.id, &lot, maintenant())
        .await;
    assert!(
        matches!(refus, Err(EntDbError::Forbidden)),
        "{:?}",
        refus.err()
    );
    // Le membre désigné, lui, décide.
    let recap = ent
        .decider_en_lot_for_actor(etab, autre, "direction", commission.id, &lot, maintenant())
        .await
        .unwrap();
    assert_eq!((recap.reussies, recap.refusees), (1, 0), "{recap:?}");
    // Plafonds : 501 dossiers d'un coup, publication à plus de 90 jours.
    let trop: Vec<Uuid> = (0..501).map(|_| Uuid::new_v4()).collect();
    assert!(matches!(
        ent.modifier_dossiers_commission_for_actor(
            etab,
            u,
            r,
            commission.id,
            &CommissionDossiersBody {
                ajouter: trop,
                retirer: vec![]
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let d2 = dossier(&ent, &c, camp, "Programme", None).await;
    let mut b = corps("admis", Some(offre), "Admis");
    b.publier_le = Some(t("2027-12-01T09:00:00Z"));
    assert!(matches!(
        decider(&ent, &c, &d2, &b).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ───────────────────────────── M2 ─────────────────────────────

/// Tant qu'une décision plus récente que la dernière publiée existe, aucune action de la famille ne révèle ni
/// n'exécute cet état : le refus est le message neutre d'une modification concurrente.
#[tokio::test]
async fn r_m2_actions_famille_sur_etat_visible() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Famille", None).await;
    let mut b = corps("refuse", Some(offre), "Refus non publié");
    b.motif_communique = Some("MOTIF-SECRET".into());
    b.publier_le = Some(t("2027-03-10T09:00:00Z"));
    decider(&ent, &c, &d, &b).await.unwrap();
    let neutre = |e: &EntDbError| {
        conflit_exact(
            e,
            "le dossier est en cours de modification : réessayez dans quelques instants",
        )
    };
    let e = ent
        .retirer_candidature_famille(etab, &d.jeton, d.id)
        .await
        .unwrap_err();
    assert!(neutre(&e), "{e:?}");
    let e = ent
        .desister_famille(etab, &d.jeton, d.id)
        .await
        .unwrap_err();
    assert!(neutre(&e), "{e:?}");
    let e = ent
        .deposer_piece_famille(etab, &d.jeton, d.id, "livret", "livret.pdf")
        .await
        .unwrap_err();
    assert!(neutre(&e), "{e:?}");
    let e = ent
        .repondre_proposition_famille(
            etab,
            &d.jeton,
            d.id,
            &ReponsePropositionBody {
                reponse: "acceptee".into(),
            },
            maintenant(),
        )
        .await
        .unwrap_err();
    assert!(neutre(&e), "{e:?}");
    let message = e.to_string();
    assert!(
        !message.to_lowercase().contains("refus") && !message.contains("MOTIF-SECRET"),
        "{message}"
    );
    // Rien n'a été exécuté : l'état réel est celui de la décision non publiée.
    assert_eq!(
        super::tests_inscriptions_i5::etat(&ent, etab, &d).await,
        "refuse"
    );
}

// ───────────────────────────── M3 ─────────────────────────────

/// La validation exige la dernière décision publiée et, si une place a été proposée, son acceptation.
#[tokio::test]
async fn r_m3_validation_suit_decision_publiee_et_acceptee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Valide", None).await;
    let mut b = corps("admis", Some(offre), "Admis");
    b.publier_le = Some(t("2027-03-10T09:00:00Z"));
    let dec = decider(&ent, &c, &d, &b).await.unwrap();
    // Non publiée : refus de valider.
    let e = ent
        .valider_inscription_for_actor(etab, a, ar, d.id, &ValidationBody::default())
        .await
        .unwrap_err();
    assert!(
        e.to_string().contains("décision d'admission publiée"),
        "{e}"
    );
    // Publiée : la proposition est envoyée, la famille n'a pas répondu : refus de valider.
    ent.publier_decision_for_actor(
        etab,
        u,
        r,
        dec.id,
        &PublierBody::default(),
        t("2027-03-11T09:00:00Z"),
    )
    .await
    .unwrap();
    assert_eq!(
        proposition_ouverte(&ent, etab, &d).await.unwrap().1,
        "envoyee"
    );
    let e = ent
        .valider_inscription_for_actor(etab, a, ar, d.id, &ValidationBody::default())
        .await
        .unwrap_err();
    assert!(e.to_string().contains("pas encore accepté"), "{e}");
    // Acceptée : le refus n'est plus celui-là.
    ent.repondre_proposition_famille(
        etab,
        &d.jeton,
        d.id,
        &ReponsePropositionBody {
            reponse: "acceptee".into(),
        },
        t("2027-03-12T09:00:00Z"),
    )
    .await
    .unwrap();
    let suite = ent
        .valider_inscription_for_actor(etab, a, ar, d.id, &ValidationBody::default())
        .await;
    if let Err(e) = suite {
        let m = e.to_string();
        assert!(
            !m.contains("pas encore accepté") && !m.contains("décision d'admission publiée"),
            "{m}"
        );
    }
}

// ───────────────────────────── M4 / M9 ─────────────────────────────

/// Deux décisions concurrentes sur un même dossier : une seule passe, l'autre est un conflit (jamais une erreur
/// technique). La chaîne a une racine et un successeur par décision, imposés en base.
#[tokio::test]
async fn r_m4_chaine_et_concurrence() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Concur", None).await;
    let b1 = corps("refuse", Some(offre), "Refus");
    let b2 = corps("admis", Some(offre), "Admis");
    let (r1, r2) = tokio::join!(decider(&ent, &c, &d, &b1), decider(&ent, &c, &d, &b2));
    let (ok, ko): (Vec<_>, Vec<_>) = [r1, r2].into_iter().partition(Result::is_ok);
    assert_eq!((ok.len(), ko.len()), (1, 1));
    assert!(matches!(
        ko.into_iter().next().unwrap(),
        Err(EntDbError::ConflitDossier(_))
    ));
    // Un seul racine, une seule feuille.
    let racines = nombre(
        &ent,
        "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1 AND remplace_decision_id IS NULL",
        d.id,
    )
    .await;
    assert_eq!(racines, 1);
    // Deux revues concurrentes : la chaîne reste linéaire (un successeur par décision).
    let (ba, bb) = (
        corps("admis", Some(offre), "Revu A"),
        corps("liste_attente", Some(offre), "Revu B"),
    );
    let (r3, r4) = tokio::join!(revoir(&ent, &c, &d, &ba), revoir(&ent, &c, &d, &bb));
    // Assertion exacte (passe 3) : au moins une revue passe, chaque refus est un conflit de dossier (jamais une erreur
    // technique), et la chaîne compte exactement la décision initiale (une seule des deux décisions concurrentes passe) et chaque revue réussie.
    let revues_reussies = [&r3, &r4].iter().filter(|r| r.is_ok()).count();
    assert!(revues_reussies >= 1, "au moins une revue passe : {r3:?} / {r4:?}");
    for r in [&r3, &r4] {
        if let Err(e) = r {
            assert!(matches!(e, EntDbError::ConflitDossier(_)), "refus exact attendu (conflit de dossier) : {e:?}");
        }
    }
    let longueur_chaine = nombre(&ent, "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1", d.id).await;
    assert_eq!(longueur_chaine, 1 + i64::try_from(revues_reussies).unwrap());
    let doubles = nombre(
        &ent,
        "SELECT count(*) FROM (SELECT remplace_decision_id FROM ent_inscription_decision
                                WHERE dossier_id = $1 AND remplace_decision_id IS NOT NULL
                                GROUP BY remplace_decision_id HAVING count(*) > 1) x",
        d.id,
    )
    .await;
    assert_eq!(doubles, 0);
    // L'historique suit la chaîne : chaque décision remplace la précédente de la liste.
    let (u, r) = dir(&c);
    let h = ent
        .decisions_dossier_for_actor(etab, u, r, d.id)
        .await
        .unwrap();
    assert!(h[0].remplace_decision_id.is_none());
    for w in h.windows(2) {
        assert_eq!(w[1].remplace_decision_id, Some(w[0].id));
    }
    // En base : une seconde racine, ou un second successeur, est refusé.
    let racine = sqlx::query(
        "INSERT INTO ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne)
         VALUES ($1,$2,$3,'ajourne','Seconde racine')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(
        sqlstate(&racine),
        UNIQUE_VIOLATION,
        "une seule racine par dossier"
    );
    let successeur = sqlx::query(
        "INSERT INTO ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne, remplace_decision_id)
         VALUES ($1,$2,$3,'ajourne','Second successeur',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(h[0].id)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(
        sqlstate(&successeur),
        UNIQUE_VIOLATION,
        "un seul successeur par décision"
    );
}

/// Attaques SQL directes sur les historiques en ajout seul : modification, suppression, troncature refusées ; la
/// purge n'est permise que par le chemin du dépôt (`ent.purge_etablissement`).
#[tokio::test]
async fn r_m9_ajout_seul_attaques_sql() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Attaque", None).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    ent.donner_avis_for_actor(
        etab,
        a,
        ar,
        d.id,
        &AvisBody {
            contexte: "entretien".into(),
            sens: "favorable".into(),
            texte: "Avis de test".into(),
        },
    )
    .await
    .unwrap();
    ent.se_deporter_for_actor(
        etab,
        a,
        ar,
        d.id,
        &DeportBody {
            motif: "Connaissance".into(),
        },
    )
    .await
    .unwrap();
    let (u, r) = dir(&c);
    ent.lever_condition_for_actor(
        etab,
        u,
        r,
        d.id,
        "reserve",
        &MotifBody {
            motif: "Test".into(),
        },
        maintenant(),
    )
    .await
    .ok();
    sqlx::query(
        "INSERT INTO ent_inscription_condition_levee (id, etablissement_id, dossier_id, code, motif, auteur_id)
         VALUES ($1,$2,$3,'contrat_signe','Levée de test',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(u)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    for (table, colonne) in [
        ("ent_inscription_avis", "texte"),
        ("ent_inscription_deport", "motif"),
        ("ent_inscription_decision", "motif_interne"),
        ("ent_inscription_condition_levee", "motif"),
    ] {
        for attaque in [
            format!(
                "UPDATE {table} SET {colonne} = 'falsifié' WHERE dossier_id = '{}'",
                d.id
            ),
            format!("DELETE FROM {table} WHERE dossier_id = '{}'", d.id),
            format!("TRUNCATE {table} CASCADE"),
        ] {
            // Dans une transaction annulée : même si une garde manquait, rien ne serait perdu.
            let mut tx = ent.pool_verif().begin().await.unwrap();
            let res = sqlx::query(&attaque).execute(&mut *tx).await;
            tx.rollback().await.unwrap();
            assert_eq!(
                sqlstate(&res),
                CHECK_VIOLATION,
                "attaque non refusée par la garde d'ajout seul : {attaque}"
            );
        }
    }
    // Le chemin de purge unique : le dépôt le pose (`SET LOCAL`), la cascade d'un dossier passe.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL ent.purge_etablissement = 'on'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(d.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_avis WHERE dossier_id = $1",
            d.id
        )
        .await,
        0
    );
}

// ───────────────────────────── M5 / M11 ─────────────────────────────

/// Une admission directe ne consomme pas la place déjà désignée au premier de la liste sans confirmation
/// explicite, motivée et tracée ; la désignation reprise est annulée, le dossier garde son rang.
#[tokio::test]
async fn r_m5_admission_directe_sur_place_designee() {
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
    let premier = dossier(&ent, &c, camp, "Premier", None).await;
    decider(
        &ent,
        &c,
        &premier,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    // La place se libère : le premier est désigné.
    revoir(
        &ent,
        &c,
        &plein,
        &corps("refuse", Some(offre), "Désistement constaté"),
    )
    .await
    .unwrap();
    assert_eq!(
        proposition_ouverte(&ent, etab, &premier).await.unwrap().1,
        "designee"
    );
    // Admission directe d'un autre dossier : refusée sans confirmation explicite.
    let direct = dossier(&ent, &c, camp, "Direct", None).await;
    let e = decider(
        &ent,
        &c,
        &direct,
        &corps("admis", Some(offre), "Admission directe"),
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("désignée"), "{e}");
    assert_eq!(
        proposition_ouverte(&ent, etab, &premier).await.unwrap().1,
        "designee"
    );
    // Confirmation sans motif : refusée aussi.
    let mut b = corps("admis", Some(offre), "Admission directe");
    b.confirmer_passer_devant = true;
    let e = decider(&ent, &c, &direct, &b).await.unwrap_err();
    assert!(
        matches!(e, EntDbError::ConflitDossier(_)),
        "refus exact attendu (conflit) : {e:?}"
    );
    // Confirmation motivée : la désignation est annulée (tracée), le premier garde son rang.
    b.motif_passer_devant = Some("Frère d'un élève de l'établissement".into());
    decider(&ent, &c, &direct, &b).await.unwrap();
    assert!(proposition_ouverte(&ent, etab, &premier).await.is_none());
    let motif: String = sqlx::query_scalar(
        "SELECT motif FROM ent_inscription_proposition_place WHERE dossier_id = $1 AND statut = 'annulee'",
    )
    .bind(premier.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(motif.contains("Frère d'un élève"), "{motif}");
    assert_eq!(
        super::tests_inscriptions_i5::etat(&ent, etab, &premier).await,
        "liste_attente"
    );
    let traces = nombre(
        &ent,
        "SELECT count(*) FROM ent_inscription_suivi_dossier WHERE dossier_id = $1 AND kind = 'liste_attente'
            AND texte LIKE '%reprise par une admission directe%'",
        premier.id,
    )
    .await;
    assert_eq!(traces, 1);
}

/// « Passer son tour » ne vaut que pour la place concernée : à une place libérée plus tard, le dossier écarté
/// redevient désignable à son rang.
#[tokio::test]
async fn r_m11_passer_son_tour_une_place() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let p1 = dossier(&ent, &c, camp, "Place1", None).await;
    decider(&ent, &c, &p1, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let a = dossier(&ent, &c, camp, "RangA", None).await;
    decider(
        &ent,
        &c,
        &a,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    let b = dossier(&ent, &c, camp, "RangB", None).await;
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    revoir(&ent, &c, &p1, &corps("refuse", Some(offre), "Désistement"))
        .await
        .unwrap();
    let (prop_a, statut) = proposition_ouverte(&ent, etab, &a).await.unwrap();
    assert_eq!(statut, "designee");
    // A passe son tour : B est désigné pour cette place ; A n'est pas re-désigné pour elle.
    ent.ecarter_proposition_for_actor(
        etab,
        u,
        r,
        prop_a,
        &MotifBody {
            motif: "Famille injoignable".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    assert!(proposition_ouverte(&ent, etab, &a).await.is_none());
    assert_eq!(
        proposition_ouverte(&ent, etab, &b).await.unwrap().1,
        "designee"
    );
    // B décline… la place revient : une nouvelle place libérée plus tard est de nouveau proposée à A (son rang).
    let (prop_b, _) = proposition_ouverte(&ent, etab, &b).await.unwrap();
    ent.ecarter_proposition_for_actor(
        etab,
        u,
        r,
        prop_b,
        &MotifBody {
            motif: "Souhaite attendre".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    // Capacité augmentée **par la route de l'offre** : une place de plus (nouveau numéro de place), désignation
    // explicite par la direction.
    let (niveau, libelle, regime): (String, String, Option<String>) = sqlx::query_as(
        "SELECT niveau_code, libelle_public, regime FROM ent_inscription_offre WHERE id = $1",
    )
    .bind(offre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    ent.modifier_offre_for_actor(
        etab,
        u,
        r,
        offre,
        &OffreBody {
            niveau_code: niveau,
            regime,
            libelle_public: libelle,
            capacite: Some(2),
            motif: Some("Ouverture d'une place".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    ent.designer_suivants_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    assert_eq!(
        proposition_ouverte(&ent, etab, &a).await.map(|p| p.1),
        Some("designee".to_string()),
        "A redevient désignable à son rang pour une place ultérieure"
    );
}

// ───────────────────────────── M6 / M7 ─────────────────────────────

/// Une proposition `envoyee` n'est jamais réécrite : annulation tracée et nouvelle proposition ; la réponse de la
/// famille est réinitialisée ; la levée d'une condition reste au journal en ajout seul.
#[tokio::test]
async fn r_m6_m7_proposition_envoyee_et_levee_journalisee() {
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
    let d = dossier(&ent, &c, camp, "Envoyee", None).await;
    let d1 = decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let (p1, statut) = proposition_ouverte(&ent, etab, &d).await.unwrap();
    assert_eq!(statut, "envoyee");
    // La direction lève le contrat (motif) : journalisé.
    ent.lever_condition_for_actor(
        etab,
        u,
        r,
        d.id,
        "contrat_signe",
        &MotifBody {
            motif: "Contrat reçu au guichet".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    assert_eq!(
        nombre(&ent, "SELECT count(*) FROM ent_inscription_condition_levee WHERE dossier_id = $1 AND motif = 'Contrat reçu au guichet'", d.id).await,
        1
    );
    // Revue : « admis sous réserve ». La proposition envoyée n'est pas réécrite : annulée, une nouvelle est créée.
    let mut b = corps("admis_sous_reserve", Some(offre), "Réserve ajoutée");
    b.reserve = Some("Bulletin du 2e trimestre".into());
    b.publier_le = Some(t("2027-03-10T09:00:00Z"));
    revoir(&ent, &c, &d, &b).await.unwrap();
    let (statut_p1, decision_p1): (String, Option<Uuid>) = sqlx::query_as(
        "SELECT statut, decision_id FROM ent_inscription_proposition_place WHERE id = $1",
    )
    .bind(p1)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(statut_p1, "annulee");
    assert_eq!(
        decision_p1,
        Some(d1.id),
        "l'ancienne proposition garde sa décision d'origine"
    );
    let (p2, statut2) = proposition_ouverte(&ent, etab, &d).await.unwrap();
    assert_ne!(p1, p2);
    assert_eq!(
        statut2, "designee",
        "la nouvelle proposition ne part qu'à la publication"
    );
    // La proposition annulée est tracée au suivi interne (jamais visible de la famille).
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_suivi_dossier
              WHERE dossier_id = $1 AND kind = 'liste_attente' AND NOT visible_famille
                AND texte LIKE 'Proposition de place envoyée annulée%'",
            d.id
        )
        .await,
        1
    );
    // La levée reste au journal : une condition réinitialisée ne l'efface pas.
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_condition_levee WHERE dossier_id = $1",
            d.id
        )
        .await,
        1,
        "la levée n'est jamais effacée"
    );
    expedier(&ent).await;
}

// ───────────────────────────── M8 ─────────────────────────────

/// Le motif interne est réservé à `inscriptions.decider` : le secrétariat ne le lit ni dans l'historique des
/// décisions, ni dans le dossier.
#[tokio::test]
async fn r_m8_motif_interne_reserve_au_decideur() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Motif", None).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "MOTIF-INTERNE-SECRET"),
    )
    .await
    .unwrap();
    let direction = ent
        .decisions_dossier_for_actor(etab, u, r, d.id)
        .await
        .unwrap();
    assert_eq!(
        direction[0].motif_interne.as_deref(),
        Some("MOTIF-INTERNE-SECRET")
    );
    let secretariat = ent
        .decisions_dossier_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert!(secretariat.iter().all(|x| x.motif_interne.is_none()));
    let dossier_secretariat = ent.dossier_for_actor(etab, a, ar, d.id).await.unwrap();
    assert!(!serde_json::to_string(&dossier_secretariat)
        .unwrap()
        .contains("MOTIF-INTERNE-SECRET"));
    let dossier_direction = ent.dossier_for_actor(etab, u, r, d.id).await.unwrap();
    assert!(serde_json::to_string(&dossier_direction)
        .unwrap()
        .contains("MOTIF-INTERNE-SECRET"));
}

// ───────────────────────────── M10 ─────────────────────────────

/// Deux envois concurrents de la même place : un seul passe. Acceptation contre expiration : un seul gagne, l'état
/// final est cohérent.
#[tokio::test]
async fn r_m10_concurrence_envois_et_reponse() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Libere", None).await;
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
    let (prop, _) = proposition_ouverte(&ent, etab, &suivant).await.unwrap();
    let envoi = EnvoyerPropositionBody {
        confirmer: true,
        ..Default::default()
    };
    let (e1, e2) = tokio::join!(
        ent.envoyer_proposition_for_actor(etab, u, r, prop, &envoi, t("2027-03-03T09:00:00Z")),
        ent.envoyer_proposition_for_actor(etab, u, r, prop, &envoi, t("2027-03-03T09:00:01Z"))
    );
    let (ok, ko): (Vec<_>, Vec<_>) = [e1, e2].into_iter().partition(Result::is_ok);
    assert_eq!((ok.len(), ko.len()), (1, 1), "un seul envoi");
    assert!(matches!(
        ko.into_iter().next().unwrap(),
        Err(EntDbError::ConflitDossier(_))
    ));
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1",
            suivant.id
        )
        .await,
        2
    );
    // Acceptation (famille) contre expiration constatée (direction), après le délai de réponse.
    let apres = t("2027-04-30T09:00:00Z");
    let acceptee = ReponsePropositionBody {
        reponse: "acceptee".into(),
    };
    let (acc, exp) = tokio::join!(
        ent.repondre_proposition_famille(
            etab,
            &suivant.jeton,
            suivant.id,
            &acceptee,
            t("2027-03-04T09:00:00Z")
        ),
        ent.constater_expiration_for_actor(etab, u, r, prop, apres)
    );
    // Exactement une des deux passe ; l'autre est refusée par le conflit classé (jamais une erreur technique).
    assert_eq!(
        usize::from(acc.is_ok()) + usize::from(exp.is_ok()),
        1,
        "exactement une des deux passe : {:?} / {:?}",
        acc.as_ref().err(),
        exp.as_ref().err()
    );
    for perdante in [acc.as_ref().err(), exp.as_ref().err()]
        .into_iter()
        .flatten()
    {
        assert!(
            matches!(perdante, EntDbError::ConflitDossier(_)),
            "refus exact attendu (conflit) : {perdante:?}"
        );
    }
    let statut: String =
        sqlx::query_scalar("SELECT statut FROM ent_inscription_proposition_place WHERE id = $1")
            .bind(prop)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    let etat_final = super::tests_inscriptions_i5::etat(&ent, etab, &suivant).await;
    match statut.as_str() {
        "acceptee" => assert!(
            matches!(etat_final.as_str(), "admis" | "confirme"),
            "{etat_final}"
        ),
        "expiree" => assert_eq!(etat_final, "sans_suite"),
        autre => panic!("statut final incohérent : {autre} / {etat_final}"),
    }
}

// ───────────────────────────── F2 / F6 ─────────────────────────────

/// Le rang lu par la famille ne dépend que des décisions publiées : un autre dossier dont la décision n'est pas
/// encore publiée ne le décale pas.
#[tokio::test]
async fn r_f2_rang_famille_decisions_publiees() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Occupe", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let non_publie = dossier(&ent, &c, camp, "NonPublie", None).await;
    let mut b = corps("liste_attente", Some(offre), "Complet");
    b.publier_le = Some(t("2027-03-10T09:00:00Z"));
    decider(&ent, &c, &non_publie, &b).await.unwrap();
    let publie = dossier(&ent, &c, camp, "Publie", None).await;
    decider(
        &ent,
        &c,
        &publie,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    let vue = ent
        .decision_famille(etab, &publie.jeton, publie.id)
        .await
        .unwrap();
    assert_eq!(
        vue.position_attente,
        Some(1),
        "le dossier non publié, pourtant devant, ne décale pas le rang lu par la famille"
    );
}

/// Aucune erreur technique (SQL, base) dans le récapitulatif d'un lot ; seulement ce que la personne peut corriger.
#[tokio::test]
async fn r_f6_recapitulatif_sans_erreur_technique() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Lot", None).await;
    let commission = ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                campagne_id: camp,
                intitule: "Lot de test".into(),
                seance_le: t("2027-03-20T14:00:00Z"),
                membres: vec![u],
                dossier_ids: vec![d.id],
            },
        )
        .await
        .unwrap();
    let mauvais = DecisionCompleteBody {
        sens: "admis_sous_reserve".into(),
        motif_interne: "Motif".into(),
        offre_id: Some(Uuid::new_v4()),
        ..Default::default()
    };
    let recap = ent
        .decider_en_lot_for_actor(
            etab,
            u,
            r,
            commission.id,
            &DecisionsEnLotBody {
                decisions: vec![
                    DecisionEnLotItem {
                        dossier_id: d.id,
                        decision: mauvais,
                    },
                    DecisionEnLotItem {
                        dossier_id: Uuid::new_v4(),
                        decision: corps("refuse", Some(offre), "Motif"),
                    },
                ],
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(recap.refusees, 2, "{recap:?}");
    for res in &recap.resultats {
        let m = res.erreur.clone().unwrap().to_lowercase();
        assert!(
            !m.contains("sql")
                && !m.contains("database")
                && !m.contains("violates")
                && !m.contains("constraint"),
            "{m}"
        );
    }
}

// ═════════════════════════════ Passe 2 (contre-revue du pilote) ═════════════════════════════
//
// | Correctif | Test(s) |
// |---|---|
// | M-A, F3 : rang famille = décisions publiées (sortie de liste, séance) ; `dossier_famille` = `decision_famille` | `p2_ma_rang_famille_sortie_de_liste_non_publiee`, `p2_f3_decisions_de_seance_non_publiees` |
// | M-B : droit avant écart (transition) | `p2_mb_droit_avant_ecart` |
// | M-C : passer son tour, même place | `p2_mc_passer_son_tour_une_seule_place` |
// | M-D : registre, export de la personne | `p2_md_registre_export_publie_seulement` |
// | F4 : masquage par la chaîne | `p2_f4_masquage_par_la_chaine` |
// | F7 : déport idempotent | `p2_f7_se_deporter_idempotent` |
// | F9, F10, F11 : commissions et dossiers écartés | `p2_f9_f10_commission_et_dossiers_ecartes` |
// | F14, F15 : constat et acceptation au guichet journalisés | `p2_f14_f15_constat_et_acceptation_au_guichet` |
// | F16 : 313 rejouable sur une base de l'ancienne version | `p2_f16_313_rejouable_sur_ancienne_version` |

/// Membre du personnel créé pour le test, avec ce rôle et cette adresse.
pub(super) async fn agent_role(ent: &PgEnt, etab: Uuid, mail: &str, role: &str) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1,$2,'x','Agent de test', now(), now())",
    )
    .bind(user)
    .bind(mail)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1,$2,$3, now())",
    )
    .bind(user)
    .bind(etab)
    .bind(role)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    user
}

/// Rang vu par la famille par les deux lectures : `dossier_famille` et `decision_famille` (un seul calcul).
pub(super) async fn rang_vu(ent: &PgEnt, etab: Uuid, d: &super::tests_inscriptions_i5::Dos) -> Option<i64> {
    let dossier = ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap();
    let decision = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    assert_eq!(
        dossier.rang_attente.map(i64::from),
        decision.position_attente,
        "dossier_famille et decision_famille donnent le même rang"
    );
    decision.position_attente
}

async fn decision_id_de(ent: &PgEnt, etab: Uuid, u: Uuid, r: &str, d: Uuid) -> Uuid {
    ent.decisions_dossier_for_actor(etab, u, r, d)
        .await
        .unwrap()
        .last()
        .unwrap()
        .id
}

/// M-A : une sortie de liste par décision **non publiée** ne décale pas le rang des autres ; une fois publiée, oui.
#[tokio::test]
async fn p2_ma_rang_famille_sortie_de_liste_non_publiee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Occupe", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let a = dossier(&ent, &c, camp, "RangA", None).await;
    let b = dossier(&ent, &c, camp, "RangB", None).await;
    let cc = dossier(&ent, &c, camp, "RangC", None).await;
    for x in [&a, &b, &cc] {
        decider(&ent, &c, x, &corps("liste_attente", Some(offre), "Complet"))
            .await
            .unwrap();
    }
    assert_eq!(rang_vu(&ent, etab, &a).await, Some(1));
    assert_eq!(rang_vu(&ent, etab, &b).await, Some(2));
    assert_eq!(rang_vu(&ent, etab, &cc).await, Some(3));

    // B sort de la liste par un refus **non publié** (programmé) : le rang réel de C passe à 2, celui de la famille non.
    let mut refus = corps("refuse", Some(offre), "Refus programmé");
    refus.publier_le = Some(t("2027-03-10T09:00:00Z"));
    revoir(&ent, &c, &b, &refus).await.unwrap();
    let reel: Option<i32> =
        sqlx::query_scalar("SELECT rang_attente FROM ent_inscription_dossier WHERE id = $1")
            .bind(cc.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(reel, Some(2), "le rang réel (personnel) a bougé");
    assert_eq!(
        rang_vu(&ent, etab, &cc).await,
        Some(3),
        "la famille ne voit pas la sortie de B"
    );
    assert_eq!(rang_vu(&ent, etab, &a).await, Some(1));
    // B lui-même : rien ne se lit (décision non publiée), il compte toujours dans la file des autres.
    assert_eq!(rang_vu(&ent, etab, &b).await, None);
    assert_eq!(
        ent.dossier_famille(etab, &b.jeton, b.id)
            .await
            .unwrap()
            .etat,
        "liste_attente"
    );

    // Un dossier dont l'entrée en liste n'est pas publiée ne compte pas encore pour les autres.
    let nouveau = dossier(&ent, &c, camp, "RangD", None).await;
    let mut entree = corps("liste_attente", Some(offre), "Complet");
    entree.publier_le = Some(t("2027-03-10T09:00:00Z"));
    decider(&ent, &c, &nouveau, &entree).await.unwrap();
    assert_eq!(rang_vu(&ent, etab, &nouveau).await, None);
    assert_eq!(rang_vu(&ent, etab, &cc).await, Some(3));

    // La sortie de B publiée : C avance, pour les deux lectures.
    let id = decision_id_de(&ent, etab, u, r, b.id).await;
    ent.publier_decision_for_actor(etab, u, r, id, &PublierBody::default(), maintenant())
        .await
        .unwrap();
    assert_eq!(rang_vu(&ent, etab, &cc).await, Some(2));
    assert_eq!(rang_vu(&ent, etab, &a).await, Some(1));
}

/// F3 : les décisions prises **en séance** restent non publiées jusqu'à la publication de la séance ; ni l'état ni le
/// rang vus par la famille ne bougent avant, les deux lectures concordent.
#[tokio::test]
async fn p2_f3_decisions_de_seance_non_publiees() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Occupe", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let a = dossier(&ent, &c, camp, "SeanceA", None).await;
    let b = dossier(&ent, &c, camp, "SeanceB", None).await;
    let cc = dossier(&ent, &c, camp, "SeanceC", None).await;
    for x in [&a, &b, &cc] {
        decider(&ent, &c, x, &corps("liste_attente", Some(offre), "Complet"))
            .await
            .unwrap();
    }
    let commission = ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                campagne_id: camp,
                intitule: "Séance de test".into(),
                seance_le: t("2027-03-20T14:00:00Z"),
                membres: vec![u],
                dossier_ids: vec![b.id],
            },
        )
        .await
        .unwrap();
    let recap = ent
        .decider_en_lot_for_actor(
            etab,
            u,
            r,
            commission.id,
            &DecisionsEnLotBody {
                decisions: vec![DecisionEnLotItem {
                    dossier_id: b.id,
                    decision: corps("refuse", Some(offre), "Refus de séance"),
                }],
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!((recap.reussies, recap.refusees), (1, 0), "{recap:?}");
    assert_eq!(
        rang_vu(&ent, etab, &cc).await,
        Some(3),
        "décision de séance non publiée : rien ne bouge"
    );
    assert_eq!(rang_vu(&ent, etab, &b).await, None);
    assert_eq!(
        ent.dossier_famille(etab, &b.jeton, b.id)
            .await
            .unwrap()
            .etat,
        "liste_attente"
    );
    let p = ent
        .publier_commission_for_actor(
            etab,
            u,
            r,
            commission.id,
            &PublierBody::default(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!((p.publiees, p.ignorees), (1, 0));
    assert_eq!(rang_vu(&ent, etab, &cc).await, Some(2));
    assert_eq!(
        ent.dossier_famille(etab, &b.jeton, b.id)
            .await
            .unwrap()
            .etat,
        "refuse"
    );
}

/// M-B : le droit précède la garde d'écart. Un compte **sans** le droit et lié au candidat reçoit « interdit » (403),
/// un compte **avec** le droit et lié reçoit « introuvable » ; une action inconnue n'est signalée qu'après le droit.
#[tokio::test]
async fn p2_mb_droit_avant_ecart() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let second = email("second");
    let d = dossier(&ent, &c, camp, "DroitEcart", Some(&second)).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    // Lié (adresse du second responsable) mais sans le droit de décider : le secrétariat (« administratif »).
    let sans_droit = agent_role(&ent, etab, &second, "administratif").await;
    let annuler = TransitionBody {
        action: "annuler".into(),
        motif: "Motif de test".into(),
    };
    let r = ent
        .transition_for_actor(etab, sans_droit, "administratif", d.id, &annuler)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "sans droit et lié : 403 exact, obtenu {:?}",
        r.err()
    );
    let r = ent
        .decider_avance_for_actor(
            etab,
            sans_droit,
            "administratif",
            d.id,
            &DecisionAvanceeBody {
                decision: "refuse".into(),
                motif: Some("Motif".into()),
                ..Default::default()
            },
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "décision sans droit et lié : 403 exact, obtenu {:?}",
        r.err()
    );
    // Lié **avec** le droit : introuvable.
    let avec_droit = agent_direction(&ent, etab, &d.email).await;
    refus_introuvable!(
        "transition (avec droit, lié)",
        ent.transition_for_actor(etab, avec_droit, "direction", d.id, &annuler)
    );
    // Action inconnue : le droit d'abord, puis la forme de la demande (jamais l'existence du dossier).
    let r = ent
        .transition_for_actor(
            etab,
            avec_droit,
            "direction",
            d.id,
            &TransitionBody {
                action: "inconnue".into(),
                motif: "Motif de test".into(),
            },
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "forme refusée avant l'existence : {:?}",
        r.err()
    );
}

/// M-C : « passer son tour » — A, B écartent la même place : ni la désignation automatique ni l'appel sans exclusion ne
/// la reproposent à A ; une place libérée **ensuite** redevient proposable à A à son rang.
#[tokio::test]
async fn p2_mc_passer_son_tour_une_seule_place() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let plein = dossier(&ent, &c, camp, "Libere", None).await;
    decider(&ent, &c, &plein, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let a = dossier(&ent, &c, camp, "TourA", None).await;
    let b = dossier(&ent, &c, camp, "TourB", None).await;
    for x in [&a, &b] {
        decider(&ent, &c, x, &corps("liste_attente", Some(offre), "Complet"))
            .await
            .unwrap();
    }
    revoir(
        &ent,
        &c,
        &plein,
        &corps("refuse", Some(offre), "Désistement"),
    )
    .await
    .unwrap();
    let motif = MotifBody {
        motif: "Famille injoignable".into(),
    };
    // A est désigné, passe son tour : B est désigné, passe son tour.
    let (pa, sa) = proposition_ouverte(&ent, etab, &a)
        .await
        .expect("A désigné");
    assert_eq!(sa, "designee");
    ent.ecarter_proposition_for_actor(etab, u, r, pa, &motif, maintenant())
        .await
        .unwrap();
    let (pb, sb) = proposition_ouverte(&ent, etab, &b)
        .await
        .expect("B désigné à la suite");
    assert_eq!(sb, "designee");
    ent.ecarter_proposition_for_actor(etab, u, r, pb, &motif, maintenant())
        .await
        .unwrap();
    // A/B/A : la même place n'est pas reproposée à A, ni à B.
    assert!(
        proposition_ouverte(&ent, etab, &a).await.is_none(),
        "A n'est pas re-désigné pour la même place"
    );
    assert!(proposition_ouverte(&ent, etab, &b).await.is_none());
    // L'appel sans exclusion (désignation explicite) ne la reprend pas non plus.
    ent.designer_suivants_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    assert!(
        proposition_ouverte(&ent, etab, &a).await.is_none(),
        "appel sans exclusion : toujours pas A"
    );
    assert!(proposition_ouverte(&ent, etab, &b).await.is_none());
    // Une place libérée ensuite : A redevient désignable, à son rang.
    let y = dossier(&ent, &c, camp, "Nouveau", None).await;
    decider(&ent, &c, &y, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    revoir(&ent, &c, &y, &corps("refuse", Some(offre), "Désistement"))
        .await
        .unwrap();
    let (_, s) = proposition_ouverte(&ent, etab, &a)
        .await
        .expect("A désigné pour la nouvelle place");
    assert_eq!(s, "designee");
    assert!(
        proposition_ouverte(&ent, etab, &b).await.is_none(),
        "une seule place : une seule désignation"
    );
}

/// M-D : l'export de la personne (registre des données) ne porte ni motif ni auteur de décision, ni état interne, ni rang
/// brut ; il se limite aux décisions publiées ; chaque colonne retirée existe, la condition de ligne est du SQL valide.
#[tokio::test]
async fn p2_md_registre_export_publie_seulement() {
    let Some((ent, _c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let get = |nom: &str| {
        crate::registre_donnees::REGISTRE
            .iter()
            .find(|c| c.nom == nom)
            .unwrap()
    };
    let dossier = get("ent_inscription_dossier");
    for col in [
        "decision_motif",
        "decide_par",
        "etat",
        "etat_avant_blocage",
        "rang_attente",
        "ordre_attente",
    ] {
        assert!(
            dossier.colonnes_hors_export.contains(&col),
            "dossier : {col} jamais exporté"
        );
    }
    let decision = get("ent_inscription_decision");
    assert!(decision.colonnes_hors_export.contains(&"motif_interne"));
    assert!(decision.colonnes_hors_export.contains(&"decide_par"));
    assert_eq!(
        decision.export_si,
        Some("x.publiee_le IS NOT NULL"),
        "décisions publiées seulement"
    );
    let suivi = get("ent_inscription_suivi_dossier");
    assert!(
        suivi
            .export_si
            .is_some_and(|s| s.contains("publiee_le IS NOT NULL")),
        "suivi d'une décision non publiée exclu"
    );
    for classe in [dossier, decision, suivi] {
        for col in classe.colonnes_hors_export {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM information_schema.columns
                  WHERE table_schema = current_schema() AND table_name = $1 AND column_name = $2",
            )
            .bind(classe.nom)
            .bind(*col)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
            assert_eq!(
                n, 1,
                "{}.{col} : la colonne retirée de l'export existe",
                classe.nom
            );
        }
        if let Some(cond) = classe.export_si {
            sqlx::query(&format!(
                "SELECT count(*) FROM {} x WHERE {cond}",
                classe.nom
            ))
            .fetch_one(ent.pool_verif())
            .await
            .unwrap_or_else(|e| panic!("{} : condition d'export invalide : {e}", classe.nom));
        }
    }
}

/// F4 : le suivi d'une décision non publiée se masque par la **chaîne** (rattachement à la décision), jamais par
/// une date : même antidaté, il ne se lit pas avant la publication, et se lit après.
#[tokio::test]
async fn p2_f4_masquage_par_la_chaine() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Chaine", None).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    let mut refus = corps("refuse", Some(offre), "Refus programmé");
    refus.publier_le = Some(t("2027-03-10T09:00:00Z"));
    revoir(&ent, &c, &d, &refus).await.unwrap();
    // Un commentaire visible de la famille, écrit pendant la décision non publiée : rattaché à elle.
    ent.commenter_for_actor(
        etab,
        a,
        ar,
        d.id,
        &CommentaireBody {
            texte: "COMMENTAIRE-PENDANT".into(),
            visible_famille: true,
        },
    )
    .await
    .unwrap();
    // Antidatage de toutes les lignes du suivi : le masquage ne doit pas dépendre des dates.
    sqlx::query("UPDATE ent_inscription_suivi_dossier SET created_at = '2020-01-01T00:00:00Z' WHERE dossier_id = $1")
        .bind(d.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let json =
        serde_json::to_string(&ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap()).unwrap();
    assert!(
        !json.contains("\"refuse\"") && !json.contains("COMMENTAIRE-PENDANT"),
        "{json}"
    );
    let id = decision_id_de(&ent, etab, u, r, d.id).await;
    ent.publier_decision_for_actor(etab, u, r, id, &PublierBody::default(), maintenant())
        .await
        .unwrap();
    let json =
        serde_json::to_string(&ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap()).unwrap();
    assert!(
        json.contains("COMMENTAIRE-PENDANT") && json.contains("\"refuse\""),
        "{json}"
    );
}

/// F7 : se déporter deux fois est idempotent (même réponse, une seule ligne, une seule trace).
#[tokio::test]
async fn p2_f7_se_deporter_idempotent() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (camp, _offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Deport", None).await;
    let b = DeportBody {
        motif: "Connaissance de la famille".into(),
    };
    let premier = ent
        .se_deporter_for_actor(etab, a, ar, d.id, &b)
        .await
        .unwrap();
    let second = ent
        .se_deporter_for_actor(etab, a, ar, d.id, &b)
        .await
        .unwrap();
    assert_eq!(premier, second);
    assert!(second.deporte);
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_deport WHERE dossier_id = $1",
            d.id
        )
        .await,
        1
    );
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_suivi_dossier WHERE dossier_id = $1 AND kind = 'deport'",
            d.id
        )
        .await,
        1
    );
}

/// F9, F10, F11 : une commission n'accepte pas un dossier dont l'acteur est écarté ; la publication d'une séance ignore
/// (et signale) ces dossiers au lieu d'échouer en bloc ; les listes écartent les dossiers en une requête groupée.
#[tokio::test]
async fn p2_f9_f10_commission_et_dossiers_ecartes() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d1 = dossier(&ent, &c, camp, "Libre", None).await;
    let d2 = dossier(&ent, &c, camp, "Lie", None).await;
    let lie = agent_direction(&ent, etab, &d2.email).await;
    let base = CommissionBody {
        campagne_id: camp,
        intitule: "Séance écartée".into(),
        seance_le: t("2027-03-20T14:00:00Z"),
        membres: vec![u],
        dossier_ids: vec![d2.id],
    };
    // F9 : création avec un dossier dont l'acteur est écarté : introuvable.
    refus_introuvable!(
        "créer",
        ent.creer_commission_admission_for_actor(etab, lie, "direction", &base)
    );
    let commission = ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                dossier_ids: vec![d1.id],
                ..base.clone()
            },
        )
        .await
        .unwrap();
    refus_introuvable!(
        "ajouter",
        ent.modifier_dossiers_commission_for_actor(
            etab,
            lie,
            "direction",
            commission.id,
            &CommissionDossiersBody {
                ajouter: vec![d2.id],
                retirer: vec![]
            }
        )
    );
    ent.modifier_dossiers_commission_for_actor(
        etab,
        u,
        r,
        commission.id,
        &CommissionDossiersBody {
            ajouter: vec![d2.id],
            retirer: vec![],
        },
    )
    .await
    .unwrap();
    // La liste de la séance ne montre pas le dossier lié à l'agent lié (F11 : un seul filtre groupé).
    let vue = ent
        .commission_admission_for_actor(etab, lie, "direction", commission.id)
        .await
        .unwrap();
    assert!(
        vue.dossiers.iter().all(|x| x.dossier_id != d2.id)
            && vue.dossiers.iter().any(|x| x.dossier_id == d1.id)
    );
    // Décisions de séance (non publiées) sur les deux dossiers.
    let lot = DecisionsEnLotBody {
        decisions: [&d1, &d2]
            .iter()
            .map(|x| DecisionEnLotItem {
                dossier_id: x.id,
                decision: corps("liste_attente", Some(offre), "Complet"),
            })
            .collect(),
    };
    let recap = ent
        .decider_en_lot_for_actor(etab, u, r, commission.id, &lot, maintenant())
        .await
        .unwrap();
    assert_eq!((recap.reussies, recap.refusees), (2, 0), "{recap:?}");
    // F10 : l'agent lié à d2 publie la séance : d1 est publié, d2 ignoré et signalé, aucun échec en bloc.
    let p = ent
        .publier_commission_for_actor(
            etab,
            lie,
            "direction",
            commission.id,
            &PublierBody::default(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!((p.publiees, p.ignorees), (1, 1), "{p:?}");
    let publiee = |d: Uuid| {
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1 AND publiee_le IS NOT NULL",
            d,
        )
    };
    assert_eq!(publiee(d1.id).await, 1);
    assert_eq!(
        publiee(d2.id).await,
        0,
        "le dossier écarté n'est pas publié à sa place"
    );
    let p = ent
        .publier_commission_for_actor(
            etab,
            u,
            r,
            commission.id,
            &PublierBody::default(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!((p.publiees, p.ignorees), (1, 0));
}

/// F14, F15 : un constat de condition est journalisé en ajout seul ; le secrétariat enregistre l'acceptation de la
/// famille au guichet, tracée à part (motif obligatoire, journal, auteur) et jamais confondue avec sa réponse.
#[tokio::test]
async fn p2_f14_f15_constat_et_acceptation_au_guichet() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
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
    let d = dossier(&ent, &c, camp, "Guichet", None).await;
    decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    // F14 : le constat du contrat est journalisé (genre `remplie`), comme la levée.
    ent.remplir_condition_for_actor(
        etab,
        a,
        ar,
        d.id,
        "contrat_signe",
        &RemplirConditionBody::default(),
        maintenant(),
    )
    .await
    .unwrap();
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_condition_levee
              WHERE dossier_id = $1 AND code = 'contrat_signe' AND genre = 'remplie'",
            d.id
        )
        .await,
        1
    );
    let attaque = sqlx::query(
        "UPDATE ent_inscription_condition_levee SET motif = 'falsifié' WHERE dossier_id = $1",
    )
    .bind(d.id)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(sqlstate(&attaque), CHECK_VIOLATION, "journal en ajout seul");
    // F15 : acceptation au guichet. Le motif est obligatoire.
    let (prop, statut) = proposition_ouverte(&ent, etab, &d).await.unwrap();
    assert_eq!(statut, "envoyee");
    let sans = ent
        .accepter_proposition_au_guichet_for_actor(
            etab,
            a,
            ar,
            prop,
            &MotifBody { motif: "  ".into() },
            maintenant(),
        )
        .await;
    assert!(
        matches!(sans, Err(EntDbError::SaisieInvalide(_))),
        "{:?}",
        sans.err()
    );
    let dto = ent
        .accepter_proposition_au_guichet_for_actor(
            etab,
            a,
            ar,
            prop,
            &MotifBody {
                motif: "Parent venu au guichet, signature sur place".into(),
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(dto.statut, "acceptee");
    let (source, par): (Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT source, remplie_par FROM ent_inscription_condition WHERE dossier_id = $1 AND code = 'reponse_famille'",
    )
    .bind(d.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (source.as_deref(), par),
        (Some("personnel"), Some(a)),
        "tracée comme saisie du personnel"
    );
    assert_eq!(
        nombre(
            &ent,
            "SELECT count(*) FROM ent_inscription_condition_levee
              WHERE dossier_id = $1 AND code = 'reponse_famille' AND genre = 'guichet'",
            d.id
        )
        .await,
        1
    );
    // Une seule fois.
    let encore = ent
        .accepter_proposition_au_guichet_for_actor(
            etab,
            a,
            ar,
            prop,
            &MotifBody {
                motif: "Deuxième saisie".into(),
            },
            maintenant(),
        )
        .await;
    assert!(
        matches!(encore, Err(EntDbError::ConflitDossier(_))),
        "{:?}",
        encore.err()
    );
}

/// F16 : 313 rejouable sur une base où l'**ancienne** version a tourné (objets de la passe 1 absents, historique de
/// décisions sans chaîne). Tout se passe dans une transaction annulée : la base de test n'est pas modifiée.
#[tokio::test]
async fn p2_f16_313_rejouable_sur_ancienne_version() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Ancienne", None).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    let mut tx = ent.pool_verif().begin().await.unwrap();
    // L'état de l'ancienne version : ni la chaîne, ni les gardes de lien, ni le journal des levées, ni les colonnes de la passe 2.
    for ordre in [
        "DROP TRIGGER IF EXISTS ent_inscription_decision_chaine_trg ON ent_inscription_decision",
        "DROP TRIGGER IF EXISTS ent_inscription_decision_ecarte_trg ON ent_inscription_decision",
        "DROP TRIGGER IF EXISTS ent_inscription_decision_truncate_trg ON ent_inscription_decision",
        "DROP TRIGGER IF EXISTS ent_inscription_avis_truncate_trg ON ent_inscription_avis",
        "DROP TRIGGER IF EXISTS ent_inscription_avis_ecarte_trg ON ent_inscription_avis",
        "DROP TRIGGER IF EXISTS ent_inscription_deport_truncate_trg ON ent_inscription_deport",
        "DROP INDEX IF EXISTS ent_inscription_decision_racine_uq",
        "DROP INDEX IF EXISTS ent_inscription_decision_successeur_uq",
        "DROP TABLE IF EXISTS ent_inscription_condition_levee",
        "ALTER TABLE ent_inscription_suivi_dossier DROP COLUMN IF EXISTS decision_id",
        "ALTER TABLE ent_inscription_offre DROP COLUMN IF EXISTS liberations",
        "ALTER TABLE ent_inscription_proposition_place DROP COLUMN IF EXISTS place_no",
        "ALTER TABLE ent_inscription_dossier DROP COLUMN IF EXISTS ordre_attente, DROP COLUMN IF EXISTS offre_attente_id",
        "DROP FUNCTION IF EXISTS ent_inscription_en_liste_visible(uuid, uuid, text)",
        "DROP SEQUENCE IF EXISTS ent_inscription_attente_ordre_seq",
        // Un historique **sans chaîne** : deux racines pour le même dossier (ce que l'ancien code pouvait laisser).
        "ALTER TABLE ent_inscription_decision DISABLE TRIGGER ent_inscription_decision_ajout_seul_trg",
    ] {
        sqlx::query(ordre).execute(&mut *tx).await.unwrap_or_else(|e| panic!("{ordre} : {e}"));
    }
    sqlx::query(
        "INSERT INTO ent_inscription_decision (id, etablissement_id, dossier_id, sens, motif_interne, decide_le)
         VALUES ($1,$2,$3,'refuse','Seconde racine héritée', now() + interval '1 minute')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE ent_inscription_decision ENABLE TRIGGER ent_inscription_decision_ajout_seul_trg")
        .execute(&mut *tx)
        .await
        .unwrap();
    // Rejeu du fichier courant.
    let sql = include_str!("../../../migrations/313_inscription_decision_i5.sql");
    for ordre in crate::migrate::ordres(sql) {
        sqlx::query(&ordre)
            .execute(&mut *tx)
            .await
            .unwrap_or_else(|e| panic!("rejeu de 313 refusé : {e}\n{ordre}"));
    }
    // L'historique est chaîné (une racine, un successeur) et les objets sont revenus.
    let racines: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1 AND remplace_decision_id IS NULL",
    )
    .bind(d.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        racines, 1,
        "l'historique sans chaîne est chaîné par le rejeu"
    );
    for objet in [
        "ent_inscription_condition_levee",
        "ent_inscription_decision_racine_uq",
        "ent_inscription_attente_ordre_seq",
    ] {
        let n: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(objet)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert!(n.is_some(), "{objet} recréé par le rejeu");
    }
    tx.rollback().await.unwrap();
}
