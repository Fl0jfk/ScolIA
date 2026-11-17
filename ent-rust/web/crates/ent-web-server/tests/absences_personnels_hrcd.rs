//! Absences des personnels (socle 15, écrans A11d à A11f) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : le client typé de l'écran (`ent-web-api`) joue les heures et états (valider,
//! corriger, figer, signer, exporter, régulariser), le remplacement de courte durée (plan versionné, volontaires,
//! compteurs, remontée **sans nom**) et le paramétrage (seuil de la mention « répond rarement », circuit), avec la
//! **séparation** : un professeur ne lit jamais un motif ni un état d'heures, rien d'un autre établissement.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test absences_personnels_hrcd -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »). Chaque test ouvre son propre
//! établissement (identifiant neuf).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate};
use ent_types::absences_personnels::{
    CompteRenduRcdBody, ContesterHeureBody, CorrigerHeureBody, CreerRegleCircuitBody,
    EngagementRcdBody, FigerEtatBody, FiltreHeures, ModifierEngagementRcdBody, PlanRcdBody,
    ProcedurePlanRcdBody, SaisirHeureBody, ValiderHeuresBody,
};
use ent_web_api::{ApiClient, ApiError, Connexion, SuiteRemontee};
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"]
            .iter()
            .any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (app, _) = ent_web_server::routeur_complet()
        .await
        .expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some(format!("http://{addr}"))
}

struct Compte {
    api: ApiClient,
    user_id: Uuid,
}

async fn inscrire(base: &str, etab: Uuid, role: &str) -> Compte {
    let email = format!("a11h-{role}-{}@ent.local", Uuid::new_v4().simple());
    ApiClient::new(base, None)
        .post_json(
            "/auth/register",
            serde_json::json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
        )
        .await
        .unwrap_or_else(|e| panic!("inscription {role} : {e:?}"));
    match ApiClient::new(base, None)
        .connexion(&email, MDP, etab)
        .await
        .unwrap_or_else(|e| panic!("connexion {role} : {e:?}"))
    {
        Connexion::Ouverte(s) => Compte {
            api: ApiClient::new(base, Some(s.token.clone())),
            user_id: s.user_id,
        },
        Connexion::PasskeyRequise(_) => panic!("passkey requise pour {role}"),
    }
}

/// S'assure que l'établissement a une année scolaire courante (le plan, les engagements et l'état annuel en dépendent).
async fn assurer_annee(dir: &Compte) {
    // Créer une classe fait assurer l'année courante de l'établissement (comme `Cadre` dans `ent-db`).
    dir.api
        .post_json(
            "/referentiel/classes",
            serde_json::json!({"code": format!("A{}", &Uuid::new_v4().simple().to_string()[..5])}),
        )
        .await
        .expect("année courante de l'établissement");
}

/// L'agent (socle 6) d'un compte, lu dans l'annuaire par la direction.
async fn agent_de(dir: &Compte, user: Uuid) -> Uuid {
    dir.api
        .absp_annuaire()
        .await
        .expect("annuaire")
        .into_iter()
        .find(|a| a.user_id == Some(user))
        .unwrap_or_else(|| panic!("aucun agent pour le compte {user}"))
        .agent_id
}

fn premier(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap()
}

/// Un jour du mois **terminé** précédent (l'état ne se fige qu'après le dernier jour du mois).
fn jour_du_mois_passe() -> NaiveDate {
    premier(ent_db::jour_paris()) - Duration::days(10)
}

// ───────────────────────── Pages rendues par le serveur ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn les_pages_hrcd_sont_rendues_avec_csp_et_sans_style_en_ligne() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/personnel/mes-heures", "Mes heures"),
        ("/web/personnel/etats-heures", "Heures et états mensuels"),
        ("/web/personnel/rcd", "Remplacement de courte durée"),
        (
            "/web/personnel/parametrage-absences",
            "Réglages des absences",
        ),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP sur {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "titre « {titre} » sur {chemin}");
        assert!(
            html.contains("Chargement"),
            "chargement rendu côté serveur sur {chemin}"
        );
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne sur {chemin}"
        );
        assert!(
            !html.contains("<script>"),
            "aucun script en ligne sans nonce sur {chemin}"
        );
    }
}

// ───────────────────────── A11d : heures, états, régularisation ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heures_etat_mensuel_signature_export_et_regularisation() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let dir = inscrire(&base, etab, "direction").await;
    let sec = inscrire(&base, etab, "administratif").await;
    let compta = inscrire(&base, etab, "compta").await;
    let prof = inscrire(&base, etab, "prof").await;
    let collegue = inscrire(&base, etab, "prof").await;
    let parent = inscrire(&base, etab, "parent").await;
    assurer_annee(&dir).await;
    let agent = agent_de(&dir, prof.user_id).await;
    let le = jour_du_mois_passe();
    let mois = premier(le);

    // R-AP28 : le remplacement de courte durée et le rattrapage dû ne se saisissent pas ; pas d'avance ; pas un prof.
    for nature in ["rcd", "rattrapage_du"] {
        let e = dir
            .api
            .absp_saisir_heure(&SaisirHeureBody {
                agent_id: agent,
                nature: nature.into(),
                duree_minutes: 60,
                le,
            })
            .await;
        assert!(matches!(e, Err(ApiError::Invalide(_))), "{nature} : {e:?}");
    }
    let futur = ent_db::jour_paris() + Duration::days(3);
    let e = dir
        .api
        .absp_saisir_heure(&SaisirHeureBody {
            agent_id: agent,
            nature: "hse".into(),
            duree_minutes: 60,
            le: futur,
        })
        .await;
    assert!(matches!(e, Err(ApiError::Invalide(_))), "{e:?}");
    let e = prof
        .api
        .absp_saisir_heure(&SaisirHeureBody {
            agent_id: agent,
            nature: "hse".into(),
            duree_minutes: 60,
            le,
        })
        .await;
    assert!(matches!(e, Err(ApiError::Interdit(_))), "{e:?}");

    // La direction saisit une HSE hors séance : constatée, preuve « saisie ».
    let h = dir
        .api
        .absp_saisir_heure(&SaisirHeureBody {
            agent_id: agent,
            nature: "hse".into(),
            duree_minutes: 90,
            le,
        })
        .await
        .expect("saisie");
    assert_eq!(
        (h.etat.as_str(), h.preuve.as_str(), h.duree_minutes),
        ("constatee", "saisie", 90)
    );
    let employeur = h.employeur.clone();
    assert!(["etat", "ogec"].contains(&employeur.as_str()));

    // « Mes heures » : la mienne, jamais celle d'un autre ; rien ne manque (aucune séance remplacée).
    let mes = prof.api.absp_mes_heures(None).await.expect("mes heures");
    assert!(mes.heures.iter().any(|x| x.id == h.id));
    assert!(mes.a_attester.is_empty());
    assert!(collegue
        .api
        .absp_mes_heures(None)
        .await
        .unwrap()
        .heures
        .is_empty());
    assert!(prof.api.absp_mon_etat_annuel().await.is_ok());
    // Les écrans de gestion sont fermés au professeur, au collègue et au parent.
    let filtre = FiltreHeures {
        mois: Some(mois),
        ..Default::default()
    };
    for api in [&prof.api, &collegue.api, &parent.api] {
        assert!(matches!(
            api.absp_heures(&filtre).await,
            Err(ApiError::Interdit(_))
        ));
    }
    // Périmètre d'employeur : la direction voit l'heure ; l'autre service ne la voit pas (liste sans elle, ou refus).
    assert!(dir
        .api
        .absp_heures(&filtre)
        .await
        .unwrap()
        .iter()
        .any(|x| x.id == h.id));
    let autre_service = if employeur == "etat" { &compta } else { &sec };
    match autre_service.api.absp_heures(&filtre).await {
        Ok(l) => assert!(
            !l.iter().any(|x| x.id == h.id),
            "l'heure d'un autre employeur ne se lit pas"
        ),
        Err(e) => assert!(matches!(e, ApiError::Interdit(_)), "{e:?}"),
    }

    // Contestation par l'agent (motif codé), une seule fois ; la validation la laisse à régler.
    let c = prof
        .api
        .absp_contester_heure(
            h.id,
            &ContesterHeureBody {
                raison_code: "duree_erronee".into(),
                note: Some("Je crois 1 h".into()),
            },
        )
        .await
        .expect("contestation");
    assert_eq!(c.etat, "contestee");
    assert!(prof
        .api
        .absp_contester_heure(
            h.id,
            &ContesterHeureBody {
                raison_code: "autre".into(),
                note: None
            }
        )
        .await
        .is_err());
    assert!(collegue
        .api
        .absp_contester_heure(
            h.id,
            &ContesterHeureBody {
                raison_code: "autre".into(),
                note: None
            }
        )
        .await
        .is_err());
    let bilan = dir
        .api
        .absp_valider_heures(&ValiderHeuresBody {
            mois,
            agent_id: None,
            employeur: Some(employeur.clone()),
        })
        .await
        .expect("validation");
    assert_eq!(bilan.contestees_restantes, 1, "{bilan:?}");
    // La correction règle la contestation : l'heure est validée à la durée juste.
    let v = dir
        .api
        .absp_corriger_heure(
            h.id,
            &CorrigerHeureBody {
                duree_minutes: 60,
                raison_code: "duree_erronee".into(),
                note: None,
            },
        )
        .await
        .expect("correction");
    assert_eq!((v.etat.as_str(), v.duree_minutes), ("validee", 60));
    // Une raison inconnue est refusée.
    assert!(matches!(
        dir.api
            .absp_corriger_heure(
                h.id,
                &CorrigerHeureBody {
                    duree_minutes: 45,
                    raison_code: "caprice".into(),
                    note: None
                }
            )
            .await,
        Err(ApiError::Invalide(_))
    ));

    // Figer, signer, exporter : seulement le bon cercle ; le professeur n'a aucun accès.
    assert!(prof
        .api
        .absp_figer_etat(&FigerEtatBody {
            mois,
            employeur: employeur.clone()
        })
        .await
        .is_err());
    let e = dir
        .api
        .absp_figer_etat(&FigerEtatBody {
            mois,
            employeur: employeur.clone(),
        })
        .await
        .expect("figer");
    assert_eq!((e.etat.as_str(), e.total_minutes), ("fige", 60), "{e:?}");
    assert!(e.lignes.iter().all(|l| l.agent_id == agent));
    // L'état se fige une fois.
    assert!(dir
        .api
        .absp_figer_etat(&FigerEtatBody {
            mois,
            employeur: employeur.clone()
        })
        .await
        .is_err());
    for api in [&prof.api, &collegue.api, &parent.api] {
        assert!(
            api.absp_etat_mensuel(e.id).await.is_err(),
            "l'état n'est lisible ni du professeur ni d'un collègue"
        );
        assert!(api.absp_signer_etat(e.id).await.is_err());
        assert!(api.absp_etat_csv(e.id).await.is_err());
    }
    let signe = dir.api.absp_signer_etat(e.id).await.expect("signature");
    assert_eq!(signe.etat, "signe");
    let csv = dir.api.absp_etat_csv(e.id).await.expect("export");
    assert!(!csv.is_empty());
    let csv = String::from_utf8_lossy(&csv).to_lowercase();
    for interdit in ["maladie", "formation", "motif", "nature_absence"] {
        assert!(
            !csv.contains(interdit),
            "l'export des heures ne dit rien d'une absence : {interdit}"
        );
    }

    // L'heure est figée : on ne la corrige plus, on la **régularise** sur l'état suivant ; l'agent la voit figée.
    let mes = prof.api.absp_mes_heures(None).await.unwrap();
    let figee = mes.heures.iter().find(|x| x.id == h.id).expect("heure");
    assert!(
        matches!(figee.etat.as_str(), "figee" | "exportee"),
        "{}",
        figee.etat
    );
    assert_eq!(figee.etat_mensuel_id, Some(e.id));
    assert!(dir
        .api
        .absp_corriger_heure(
            h.id,
            &CorrigerHeureBody {
                duree_minutes: 30,
                raison_code: "oubli".into(),
                note: None
            }
        )
        .await
        .is_err());
    assert!(prof
        .api
        .absp_contester_heure(
            h.id,
            &ContesterHeureBody {
                raison_code: "autre".into(),
                note: None
            }
        )
        .await
        .is_err());
    let r = dir
        .api
        .absp_regulariser_heure(
            h.id,
            &CorrigerHeureBody {
                duree_minutes: 45,
                raison_code: "duree_erronee".into(),
                note: None,
            },
        )
        .await
        .expect("régularisation");
    assert_eq!(
        (r.duree_avant, r.duree_apres, r.delta_minutes),
        (60, 45, -15)
    );
    let apres = dir.api.absp_heures(&filtre).await.unwrap();
    let ligne = apres.iter().find(|x| x.id == h.id).unwrap();
    assert_eq!(ligne.regularisation_minutes, -15);
    assert_eq!(
        ligne.duree_minutes, 60,
        "la durée figée n'est jamais réécrite"
    );

    // Paie et DSN : réservées à l'OGEC (comptabilité, direction) ; ni le secrétariat ni le professeur.
    assert!(matches!(
        sec.api.absp_elements_paie(mois).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(prof.api.absp_elements_paie(mois).await.is_err());
    assert!(compta.api.absp_elements_paie(mois).await.is_ok());
    assert!(compta.api.absp_dsn().await.is_ok());
    assert!(prof.api.absp_dsn().await.is_err());
}

// ───────────────────────── A11e : plan RCD, volontaires, remontée sans nom ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plan_rcd_volontaires_compteurs_et_remontee_sans_nom() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let dir = inscrire(&base, etab, "direction").await;
    let prof = inscrire(&base, etab, "prof").await;
    let collegue = inscrire(&base, etab, "prof").await;
    let parent = inscrire(&base, etab, "parent").await;
    assurer_annee(&dir).await;
    let agent_prof = agent_de(&dir, prof.user_id).await;

    // Sans plan : rien, lisible du personnel ; un parent est refusé.
    assert!(prof.api.absp_plan_courant().await.expect("plan").is_none());
    assert!(matches!(
        parent.api.absp_plan_courant().await,
        Err(ApiError::Interdit(_))
    ));
    // Un professeur ne crée ni ne lit les versions du plan.
    assert!(matches!(
        prof.api.absp_creer_plan(&PlanRcdBody::default()).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.api.absp_plans_rcd().await,
        Err(ApiError::Interdit(_))
    ));

    // Plan : brouillon (invisible d'un professeur), délai borné, procédure, version.
    assert!(matches!(
        dir.api
            .absp_creer_plan(&PlanRcdBody {
                delai_prevenance_heures: Some(200),
                ..Default::default()
            })
            .await,
        Err(ApiError::Invalide(_))
    ));
    let p = dir
        .api
        .absp_creer_plan(&PlanRcdBody {
            objectifs: Some("Assurer la continuité des cours".into()),
            delai_prevenance_heures: Some(48),
            continuite_numerique: Some(true),
            ..Default::default()
        })
        .await
        .expect("plan");
    assert_eq!(
        (p.etat.as_str(), p.version, p.delai_prevenance_heures),
        ("brouillon", 1, 48)
    );
    assert!(
        prof.api.absp_plan_courant().await.unwrap().is_none(),
        "un brouillon n'est lisible que de la direction"
    );
    assert!(dir.api.absp_plan_courant().await.unwrap().is_some());
    let p = dir
        .api
        .absp_modifier_plan(
            p.id,
            &PlanRcdBody {
                delai_prevenance_heures: Some(24),
                ..Default::default()
            },
        )
        .await
        .expect("modification du brouillon");
    assert_eq!(p.delai_prevenance_heures, 24);
    let jour = ent_db::jour_paris();
    let p = dir
        .api
        .absp_procedure_plan(
            p.id,
            &ProcedurePlanRcdBody {
                avis_cp_le: Some(jour),
                presente_le: Some(jour),
                instance: Some("instance_etablissement".into()),
                transmis_le: None,
            },
        )
        .await
        .expect("procédure");
    assert_eq!(p.etat, "presente");
    assert!(
        prof.api.absp_plan_courant().await.unwrap().is_some(),
        "présenté : lisible du personnel"
    );
    assert!(prof
        .api
        .absp_procedure_plan(
            p.id,
            &ProcedurePlanRcdBody {
                avis_cp_le: Some(jour),
                presente_le: None,
                instance: None,
                transmis_le: None
            }
        )
        .await
        .is_err());
    // Un plan présenté ne se modifie plus : on en tire une version.
    assert!(dir
        .api
        .absp_modifier_plan(p.id, &PlanRcdBody::default())
        .await
        .is_err());
    let v2 = dir
        .api
        .absp_versionner_plan(
            p.id,
            &PlanRcdBody {
                delai_prevenance_heures: Some(12),
                ..Default::default()
            },
        )
        .await
        .expect("version");
    assert_eq!(v2.version, 2);
    let versions = dir.api.absp_plans_rcd().await.unwrap();
    assert!(
        versions.iter().any(|x| x.id == p.id && x.etat == "archive"),
        "{versions:?}"
    );
    // Campagne : une seule fois par version.
    let c = dir.api.absp_campagne_rcd(v2.id).await.expect("campagne");
    assert!(c.campagne_lancee_le.is_some());
    assert!(dir.api.absp_campagne_rcd(v2.id).await.is_err());

    // Volontaire : le professeur s'engage lui-même ; son compteur ne porte que ses chiffres et sa position.
    assert!(prof.api.absp_mon_compteur_rcd().await.unwrap().is_none());
    let du = jour - Duration::days(30);
    let au = jour + Duration::days(200);
    let e = prof
        .api
        .absp_engager_rcd(&EngagementRcdBody {
            agent_id: None,
            volume_heures: 18.0,
            statut: "pacte".into(),
            du,
            au,
        })
        .await
        .expect("engagement");
    assert_eq!((e.volume_heures, e.etat.as_str()), (18.0, "actif"));
    assert!(
        prof.api
            .absp_engager_rcd(&EngagementRcdBody {
                agent_id: None,
                volume_heures: 5.0,
                statut: "hse".into(),
                du,
                au
            })
            .await
            .is_err(),
        "un seul engagement actif par agent et par année"
    );
    // Engager un collègue demande le droit de gérer.
    assert!(matches!(
        collegue
            .api
            .absp_engager_rcd(&EngagementRcdBody {
                agent_id: Some(agent_prof),
                volume_heures: 5.0,
                statut: "hse".into(),
                du,
                au
            })
            .await,
        Err(ApiError::Interdit(_))
    ));
    let mon = prof
        .api
        .absp_mon_compteur_rcd()
        .await
        .unwrap()
        .expect("compteur");
    assert_eq!(
        (mon.engagement.id, mon.heures_restantes, mon.sollicitees),
        (e.id, 18.0, 0)
    );
    assert!(["en_dessous", "dans_la_mediane", "au_dessus"].contains(&mon.position.as_str()));
    // Le vivier (nominatif) n'est pas pour un professeur ; la direction le lit.
    assert!(matches!(
        prof.api.absp_vivier_rcd().await,
        Err(ApiError::Interdit(_))
    ));
    let vivier = dir.api.absp_vivier_rcd().await.expect("vivier");
    assert_eq!((vivier.volontaires, vivier.volume_engage), (1, 18.0));
    // Mettre fin à son engagement.
    let fin = prof
        .api
        .absp_modifier_engagement_rcd(
            e.id,
            &ModifierEngagementRcdBody {
                terminer: true,
                ..Default::default()
            },
        )
        .await
        .expect("fin d'engagement");
    assert_eq!(fin.etat, "termine");
    assert!(prof
        .api
        .absp_modifier_engagement_rcd(
            e.id,
            &ModifierEngagementRcdBody {
                volume_heures: Some(3.0),
                ..Default::default()
            }
        )
        .await
        .is_err());

    // Remontée : préparée (lundi de la semaine passée), contrôlée, exportée ; aucun nom, aucune nature fine.
    let lundi = {
        let j = ent_db::jour_paris() - Duration::days(7);
        j - Duration::days(i64::from(j.weekday().num_days_from_monday()))
    };
    assert!(matches!(
        prof.api.absp_remontees_rcd().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(prof
        .api
        .absp_preparer_remontee_rcd(Some(lundi))
        .await
        .is_err());
    let r = dir
        .api
        .absp_preparer_remontee_rcd(Some(lundi))
        .await
        .expect("préparation");
    assert_eq!((r.etat.as_str(), r.semaine), ("preparee", lundi));
    // Une remontée non contrôlée ne s'exporte pas.
    assert!(dir.api.absp_remontee_csv(r.id).await.is_err());
    let r = dir
        .api
        .absp_suite_remontee_rcd(r.id, SuiteRemontee::Controle)
        .await
        .expect("contrôle");
    assert_eq!(r.etat, "controlee");
    let csv = dir.api.absp_remontee_csv(r.id).await.expect("export");
    let csv = String::from_utf8_lossy(&csv).to_lowercase();
    for interdit in ["nom", "agent", "maladie", "motif_libre", "@"] {
        assert!(
            !csv.contains(interdit),
            "la remontée ne porte aucun nom : {interdit}"
        );
    }
    let liste = dir.api.absp_remontees_rcd().await.unwrap();
    let json = serde_json::to_value(&liste).unwrap();
    for rem in json.as_array().unwrap() {
        for cle in rem.as_object().unwrap().keys() {
            assert!(
                [
                    "id",
                    "semaine",
                    "etat",
                    "preparee_le",
                    "controlee_le",
                    "transmise_le",
                    "lignes"
                ]
                .contains(&cle.as_str()),
                "champ inattendu dans une remontée : {cle}"
            );
        }
        for l in rem["lignes"].as_array().unwrap() {
            for cle in l.as_object().unwrap().keys() {
                assert!(
                    [
                        "discipline_code",
                        "niveau_code",
                        "categorie_nationale",
                        "mode_continuite",
                        "heures_prevues",
                        "heures_absence",
                        "heures_remplacees",
                        "effectif"
                    ]
                    .contains(&cle.as_str()),
                    "champ inattendu dans une ligne de remontée : {cle}"
                );
            }
        }
    }
    let t = dir
        .api
        .absp_suite_remontee_rcd(r.id, SuiteRemontee::Transmission)
        .await
        .expect("transmission");
    assert_eq!(t.etat, "transmise");
    // Compte rendu depuis la remontée.
    let cr = dir
        .api
        .absp_compte_rendu_rcd(
            v2.id,
            &CompteRenduRcdBody {
                periode_du: lundi,
                periode_au: lundi + Duration::days(6),
            },
        )
        .await
        .expect("compte rendu");
    assert_eq!(cr.remontees, 1);
    let plans = dir.api.absp_plans_rcd().await.unwrap();
    assert_eq!(
        plans.iter().find(|x| x.id == v2.id).unwrap().comptes_rendus,
        1
    );

    // Cloisonnement : un autre établissement ne voit rien de ce plan ni de ces remontées.
    let autre = Uuid::new_v4();
    let dir_b = inscrire(&base, autre, "direction").await;
    assurer_annee(&dir_b).await;
    assert!(dir_b.api.absp_plan_courant().await.unwrap().is_none());
    assert!(dir_b.api.absp_remontees_rcd().await.unwrap().is_empty());
    assert!(dir_b.api.absp_campagne_rcd(v2.id).await.is_err());
    assert_eq!(dir_b.api.absp_vivier_rcd().await.unwrap().volontaires, 0);
}

// ───────────────────────── A11f : paramétrage ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seuil_edt_de_reference_et_circuit_se_reglent_avec_le_droit_parametrer() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let dir = inscrire(&base, etab, "direction").await;
    let admin = inscrire(&base, etab, "admin").await;
    let sec = inscrire(&base, etab, "administratif").await;
    let cpe = inscrire(&base, etab, "cpe").await;
    let prof = inscrire(&base, etab, "prof").await;
    let parent = inscrire(&base, etab, "parent").await;
    let agent_sec = agent_de(&dir, sec.user_id).await;

    // Seuil : valeur d'origine, lisible de la direction et de qui organise ; jamais d'un professeur ni d'un parent.
    let p = dir.api.absp_parametres().await.expect("paramètres");
    assert_eq!(
        (
            p.seuil_sans_reponse_min,
            p.seuil_sans_reponse_defaut,
            p.regle
        ),
        (3, 3, false)
    );
    assert!(
        cpe.api.absp_parametres().await.is_ok(),
        "qui organise lit le seuil"
    );
    for api in [&prof.api, &parent.api] {
        assert!(matches!(
            api.absp_parametres().await,
            Err(ApiError::Interdit(_))
        ));
    }
    // Écriture : direction et administration seulement ; bornes de 1 à 20.
    for c in [&prof, &cpe, &sec, &parent] {
        assert!(matches!(
            c.api.absp_definir_parametres(5).await,
            Err(ApiError::Interdit(_))
        ));
    }
    for hors in [0, 21, -3] {
        assert!(
            matches!(
                dir.api.absp_definir_parametres(hors).await,
                Err(ApiError::Invalide(_))
            ),
            "{hors}"
        );
    }
    let p = dir.api.absp_definir_parametres(5).await.expect("réglage");
    assert_eq!((p.seuil_sans_reponse_min, p.regle), (5, true));
    assert_eq!(
        cpe.api
            .absp_parametres()
            .await
            .unwrap()
            .seuil_sans_reponse_min,
        5
    );
    assert_eq!(
        admin
            .api
            .absp_definir_parametres(2)
            .await
            .unwrap()
            .seuil_sans_reponse_min,
        2
    );
    // Un autre établissement n'est pas touché.
    let autre = Uuid::new_v4();
    let dir_b = inscrire(&base, autre, "direction").await;
    let pb = dir_b.api.absp_parametres().await.unwrap();
    assert_eq!((pb.seuil_sans_reponse_min, pb.regle), (3, false));

    // Emploi du temps de référence : éteint par défaut ; seule la direction l'écrit.
    assert!(!dir.api.absp_edt_reference().await.unwrap().actif);
    assert!(matches!(
        prof.api.absp_definir_edt_reference(true).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.api.absp_edt_reference().await,
        Err(ApiError::Interdit(_))
    ));

    // Circuit : règle ajoutée, listée, retirée ; refusée à un professeur ; rang et employeur contrôlés.
    let corps = CreerRegleCircuitBody {
        employeur: "etat".into(),
        famille: Some("sante".into()),
        role: "instructeur".into(),
        agent_id: agent_sec,
        rang: Some(1),
    };
    assert!(matches!(
        prof.api.absp_creer_regle_circuit(&corps).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.api.absp_regles_circuit().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        dir.api
            .absp_creer_regle_circuit(&CreerRegleCircuitBody {
                employeur: "rectorat".into(),
                ..corps.clone()
            })
            .await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        dir.api
            .absp_creer_regle_circuit(&CreerRegleCircuitBody {
                rang: Some(9),
                ..corps.clone()
            })
            .await,
        Err(ApiError::Invalide(_))
    ));
    let regle = dir
        .api
        .absp_creer_regle_circuit(&corps)
        .await
        .expect("règle");
    assert!(dir
        .api
        .absp_regles_circuit()
        .await
        .unwrap()
        .iter()
        .any(|r| r.id == regle.id));
    assert!(
        dir_b.api.absp_regles_circuit().await.unwrap().is_empty(),
        "rien d'un autre établissement"
    );
    assert!(
        dir_b
            .api
            .absp_retirer_regle_circuit(regle.id)
            .await
            .is_err(),
        "ni retirer la règle d'un autre"
    );
    dir.api
        .absp_retirer_regle_circuit(regle.id)
        .await
        .expect("retrait");
    assert!(!dir
        .api
        .absp_regles_circuit()
        .await
        .unwrap()
        .iter()
        .any(|r| r.id == regle.id));
}
