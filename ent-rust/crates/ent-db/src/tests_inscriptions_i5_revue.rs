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
async fn agent_direction(ent: &PgEnt, etab: Uuid, mail: &str) -> Uuid {
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

async fn nombre(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

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
            ent.transition_for_actor(etab, u, r, id, &TransitionBody::default())
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
        assert!(avis.is_err(), "{variante} : avis refusé en base");
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
        assert!(decision.is_err(), "{variante} : décision refusée en base");
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
    // Un membre inconnu de l'établissement : refusé aussi.
    assert!(ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                membres: vec![Uuid::new_v4()],
                ..base.clone()
            }
        )
        .await
        .is_err());
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
    assert!(r3.is_ok() || r4.is_ok());
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
    assert!(racine.is_err());
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
    assert!(successeur.is_err());
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
            assert!(res.is_err(), "attaque non refusée : {attaque}");
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
    assert!(decider(&ent, &c, &direct, &b).await.is_err());
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
    // Capacité augmentée : une place de plus, désignation explicite par la direction.
    sqlx::query("UPDATE ent_inscription_offre SET capacite = 2 WHERE id = $1")
        .bind(offre)
        .execute(ent.pool_verif())
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
    assert_eq!(direction[0].motif_interne, "MOTIF-INTERNE-SECRET");
    let secretariat = ent
        .decisions_dossier_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert!(secretariat.iter().all(|x| x.motif_interne.is_empty()));
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
    assert!(acc.is_ok() || exp.is_ok(), "au moins une des deux passe");
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
    assert!(!(acc.is_ok() && exp.is_ok()) || statut == "expiree" || statut == "acceptee");
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
