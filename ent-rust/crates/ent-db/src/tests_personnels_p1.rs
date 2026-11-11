//! Socle 6, lot P1 : agents et engagements (migrations 352, 353, 361), contre la base locale de test.
//!
//! * un agent par compte du personnel (déclencheur + reprise) ; fonction principale ;
//! * un professeur qui est aussi parent = une seule personne (socle 4) ;
//! * maître du privé sous contrat + salarié de l'OGEC = deux engagements simultanés (R-P1) ;
//! * disponibilité / congé = suspension d'engagement sans clôture (R-P4) ;
//! * ports `agent_du_compte` et `annuaire` ; matrice de droits ; isolation entre établissements, RLS ;
//! * journal et événements sans donnée personnelle ; fusion d'agents ; contact d'urgence confidentiel.
//!
//! Chaque test crée ses propres établissements (identifiants neufs) : pas de dépendance au labo.

use super::*;
use crate::dossier_personnes::{LienSaisie, PersonneSaisie};
use crate::personnels::{FiltreAnnuaire, NiveauAnnuaire, ResultatCreationAgent};
use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Portee};
use ent_types::personnels::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-P1-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    // Garde-fou : un test de reprise interrompu ne laisse jamais le déclencheur suspendu.
    sqlx::query("ALTER TABLE ent_membership ENABLE TRIGGER ent_membership_agent_sync_trg")
        .execute(ent.pool())
        .await
        .unwrap();
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p1-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(
        &email,
        MDP,
        Some(&format!(
            "Nom {role} {}",
            &Uuid::new_v4().simple().to_string()[..6]
        )),
        etab,
        role,
    )
    .await
    .expect("compte")
    .id
}

async fn email_de(ent: &PgEnt, user: Uuid) -> String {
    let (e,): (String,) = sqlx::query_as("SELECT email FROM ent_user WHERE id = $1")
        .bind(user)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    e
}

async fn connexion(ent: &PgEnt, etab: Uuid, user: Uuid) -> Result<SessionInfo, EntDbError> {
    let email = email_de(ent, user).await;
    ent.login(&email, MDP, etab).await
}

async fn agent_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let (a,): (Uuid,) = sqlx::query_as(
        "SELECT a.id FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("un agent pour ce compte");
    a
}

async fn compter(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql)
        .bind(a)
        .bind(b)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    n
}

fn d(y: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, j).unwrap()
}

fn engagement(nature: &str, employeur: &str, du: NaiveDate) -> EngagementBody {
    EngagementBody {
        du: Some(du),
        nature: nature.into(),
        employeur_type: employeur.into(),
        ..Default::default()
    }
}

fn est_conflit<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Refus(RefusMetier::Conflit, _)))
}
fn est_invalide<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Refus(RefusMetier::Invalide, _)))
}
fn est_interdit<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}
fn est_introuvable<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    )
}

fn nouvelle(nom: &str, prenom: &str, email: Option<&str>) -> CreerAgentBody {
    CreerAgentBody {
        personne: Some(PersonneAgentBody {
            nom: nom.into(),
            prenom: Some(prenom.into()),
            email: email.map(str::to_string),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn creer(ent: &PgEnt, etab: Uuid, dir: Uuid, body: &CreerAgentBody) -> AgentCreeDto {
    match ent
        .creer_agent_for_actor(etab, dir, "direction", body)
        .await
        .expect("création")
    {
        ResultatCreationAgent::Cree(c) => c,
        ResultatCreationAgent::AConfirmer(c) => panic!("correspondances inattendues : {c:?}"),
    }
}

// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn un_agent_par_compte_du_personnel_avec_fonction_principale() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let attendu = [
        ("admin", "a_preciser"),
        ("direction", "chef_etablissement"),
        ("administratif", "secretaire_scolarite"),
        ("cpe", "cpe"),
        ("compta", "comptable"),
        ("prof", "enseignant"),
        ("infirmerie", "infirmier"),
        ("psy", "psychologue"),
        ("accueil", "agent_accueil"),
    ];
    for (role, fonction) in attendu {
        let u = compte(&ent, etab, role).await;
        let a = agent_de(&ent, etab, u).await;
        // Une seule personne, un seul agent, un engagement à préciser, la fonction du profil.
        assert_eq!(
            compter(
                &ent,
                "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
                etab,
                u
            )
            .await,
            1,
            "{role}"
        );
        assert_eq!(compter(&ent, "SELECT count(*) FROM ent_agent_engagement WHERE etablissement_id = $1 AND agent_id = $2 AND nature = 'a_preciser'", etab, a).await, 1);
        let (code,): (String,) = sqlx::query_as(
            "SELECT f.code FROM ent_agent_fonction af JOIN ent_fonction f ON f.id = af.fonction_id
              WHERE af.etablissement_id = $1 AND af.agent_id = $2 AND af.principale",
        )
        .bind(etab)
        .bind(a)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert_eq!(code, fonction, "{role}");
        // Le téléphone du compte n'est pas copié sur la personne.
        let (tel,): (Option<String>,) = sqlx::query_as(
            "SELECT telephone FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(u)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert!(tel.is_none());
    }
    // Familles et élèves : aucun agent. Un parent promu personnel en reçoit un (une seule fois).
    let parent = compte(&ent, etab, "parent").await;
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id WHERE a.etablissement_id = $1 AND p.user_id = $2", etab, parent).await, 0);
    sqlx::query(
        "UPDATE ent_membership SET role = 'prof' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(parent)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE ent_membership SET role = 'cpe' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(parent)
    .execute(ent.pool())
    .await
    .unwrap();
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id WHERE a.etablissement_id = $1 AND p.user_id = $2", etab, parent).await, 1);
}

#[tokio::test]
async fn professeur_parent_une_seule_personne() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    let (personne_id,): (Uuid,) = sqlx::query_as("SELECT personne_id FROM ent_agent WHERE id = $1")
        .bind(agent)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let eleve = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at) VALUES ($1,$2,'Enfant','Test','preinscrit', now(), now())")
        .bind(eleve)
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    // Le secrétariat lie la personne du professeur à son enfant : la MÊME personne (pas de doublon).
    let r = ent
        .lier_personne_for_actor(
            etab,
            dir,
            "direction",
            eleve,
            &LienSaisie {
                personne_id: Some(personne_id),
                foyer_id: None,
                lien: "pere".into(),
                autorite_parentale: true,
                garde: None,
                date_debut: None,
                date_fin: None,
                peut_voir_notes: true,
                peut_voir_vs: true,
                peut_justifier_absence: true,
                peut_autoriser_sorties: false,
            },
        )
        .await
        .expect("lien");
    match r {
        crate::dossier_personnes::LienResultat::EnAttente { personne_id: p, .. } => {
            assert_eq!(p, personne_id)
        }
        crate::dossier_personnes::LienResultat::Actif(l) => assert_eq!(l.personne.id, personne_id),
    }
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
            etab,
            prof
        )
        .await,
        1
    );
    // Il reste agent, avec sa fonction.
    let actif = ent
        .agent_du_compte(
            etab,
            prof,
            ent.aujourdhui_etablissement(etab).await.unwrap(),
        )
        .await
        .unwrap()
        .expect("agent actif");
    assert_eq!(actif.agent_id, agent);
    assert_eq!(actif.fonction_principale.as_deref(), Some("enseignant"));

    // Sens inverse (PRS-01) : un parent connu (sans compte) devient agent avec SA personne.
    let parent_seul = ent
        .creer_personne_for_actor(
            etab,
            dir,
            "direction",
            &PersonneSaisie {
                nom: "Durand".into(),
                prenom: Some("Alice".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    ent.lier_personne_for_actor(
        etab,
        dir,
        "direction",
        eleve,
        &LienSaisie {
            personne_id: Some(parent_seul.id),
            foyer_id: None,
            lien: "mere".into(),
            autorite_parentale: true,
            garde: None,
            date_debut: None,
            date_fin: None,
            peut_voir_notes: true,
            peut_voir_vs: true,
            peut_justifier_absence: true,
            peut_autoriser_sorties: false,
        },
    )
    .await
    .unwrap();
    let c = creer(
        &ent,
        etab,
        dir,
        &CreerAgentBody {
            personne_id: Some(parent_seul.id),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(c.personne_id, parent_seul.id);
    assert!(c
        .avertissements
        .contains(&"personne_responsable".to_string()));
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
            etab,
            parent_seul.id
        )
        .await,
        1
    );
    // Une personne n'est agent qu'une fois.
    let deux = ent
        .creer_agent_for_actor(
            etab,
            dir,
            "direction",
            &CreerAgentBody {
                personne_id: Some(parent_seul.id),
                ..Default::default()
            },
        )
        .await;
    assert!(est_conflit(&deux));
}

#[tokio::test]
async fn creation_cherche_les_correspondances_avant_de_creer_une_personne() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let dir_a = compte(&ent, a, "direction").await;
    let dir_b = compte(&ent, b, "direction").await;
    let martin = ent
        .creer_personne_for_actor(
            a,
            dir_a,
            "direction",
            &PersonneSaisie {
                nom: "Martin".into(),
                prenom: Some("Paul".into()),
                email: Some("paul.martin@example.org".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let avant = compter(
        &ent,
        "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND $2 = $2",
        a,
        a,
    )
    .await;
    match ent
        .creer_agent_for_actor(a, dir_a, "direction", &nouvelle("Martin", "Paul", None))
        .await
        .unwrap()
    {
        ResultatCreationAgent::AConfirmer(c) => {
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].personne_id, martin.id);
            assert!(!c[0].est_agent);
            // Jamais l'adresse en clair.
            assert!(c[0]
                .email_masque
                .as_deref()
                .is_none_or(|m| !m.contains("paul.martin@example.org")));
        }
        autre => panic!("correspondance attendue : {autre:?}"),
    }
    // Aucune écriture tant que la direction n'a pas tranché.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND $2 = $2",
            a,
            a
        )
        .await,
        avant
    );
    // Même adresse : proposée aussi.
    assert!(matches!(
        ent.creer_agent_for_actor(
            a,
            dir_a,
            "direction",
            &nouvelle("Autre", "Nom", Some("paul.martin@example.org"))
        )
        .await
        .unwrap(),
        ResultatCreationAgent::AConfirmer(_)
    ));
    // Cloisonnement : l'autre établissement ne voit pas cette personne et crée sans question.
    assert!(matches!(
        ent.creer_agent_for_actor(
            b,
            dir_b,
            "direction",
            &nouvelle("Martin", "Paul", Some("paul.martin@example.org"))
        )
        .await
        .unwrap(),
        ResultatCreationAgent::Cree(_)
    ));
    // Confirmation explicite : une seconde personne est créée.
    let mut confirme = nouvelle("Martin", "Paul", None);
    confirme.confirmer_nouvelle = true;
    let c = creer(&ent, a, dir_a, &confirme).await;
    assert_ne!(c.personne_id, martin.id);
    // Corps ambigu ou vide : refusé.
    assert!(est_invalide(
        &ent.creer_agent_for_actor(a, dir_a, "direction", &CreerAgentBody::default())
            .await
    ));
    assert!(est_invalide(
        &ent.creer_agent_for_actor(
            a,
            dir_a,
            "direction",
            &CreerAgentBody {
                personne_id: Some(martin.id),
                personne: Some(PersonneAgentBody {
                    nom: "X".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }
        )
        .await
    ));
    // Une personne d'un autre établissement : 404 uniforme.
    assert!(est_introuvable(
        &ent.creer_agent_for_actor(
            b,
            dir_b,
            "direction",
            &CreerAgentBody {
                personne_id: Some(martin.id),
                ..Default::default()
            }
        )
        .await
    ));
}

#[tokio::test]
async fn maitre_sous_contrat_et_salarie_ogec_deux_engagements_simultanes() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    // L'engagement « à préciser » de la reprise est une saisie à corriger.
    let fiche = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap();
    assert_eq!(fiche.engagements.len(), 1);
    assert!(fiche.engagements[0]
        .alertes
        .contains(&"nature_a_preciser".to_string()));
    ent.corriger_engagement_for_actor(etab, dir, "direction", fiche.engagements[0].id)
        .await
        .unwrap();

    let ogec = Uuid::new_v4();
    let mut maitre = engagement("maitre_contractuel", "etat", d(2025, 9, 1));
    maitre.quotite_pct = Some(80.0);
    maitre.heures_dues_hebdo = Some(15.0);
    let e1 = ent
        .ajouter_engagement_for_actor(etab, dir, "direction", agent, &maitre)
        .await
        .expect("maître sous contrat");
    let mut surveillance = engagement("salarie_organisme", "organisme_gestion", d(2025, 9, 1));
    surveillance.employeur_organisme_id = Some(ogec);
    surveillance.quotite_pct = Some(20.0);
    let e2 = ent
        .ajouter_engagement_for_actor(etab, dir, "direction", agent, &surveillance)
        .await
        .expect("salarié OGEC, simultané");

    // Un seul agent, deux engagements courants (le port les rend tous deux).
    let actif = ent
        .agent_du_compte(
            etab,
            prof,
            ent.aujourdhui_etablissement(etab).await.unwrap(),
        )
        .await
        .unwrap()
        .expect("agent");
    assert_eq!(actif.agent_id, agent);
    let mut natures: Vec<_> = actif.engagements.iter().map(|e| e.nature.clone()).collect();
    natures.sort();
    assert_eq!(natures, vec!["maitre_contractuel", "salarie_organisme"]);
    assert_eq!(
        actif
            .engagements
            .iter()
            .map(|e| e.employeur_type.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    assert!(e1.alertes.is_empty() && e2.alertes.is_empty());

    // Même employeur, même nature, période qui se recouvre : refusé (et rien n'est écrit).
    let doublon = ent
        .ajouter_engagement_for_actor(etab, dir, "direction", agent, &maitre)
        .await;
    assert!(est_conflit(&doublon));
    let mut autre_ogec = surveillance.clone();
    autre_ogec.employeur_organisme_id = Some(Uuid::new_v4());
    // Autre organisme de gestion : un autre employeur, donc permis.
    ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &autre_ogec)
        .await
        .expect("autre organisme");
    // Même employeur que le maître (l'État) mais autre nature : permis, avec alerte de qualité.
    let contractuel = ent
        .ajouter_engagement_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &engagement("vacataire", "etat", d(2025, 9, 1)),
        )
        .await
        .expect("même employeur, autre nature");
    assert!(contractuel
        .alertes
        .contains(&"meme_employeur_simultane".to_string()));
    // Périodes successives de même nature : permis (renouvellement).
    let mut ancien = engagement("maitre_contractuel", "etat", d(2020, 9, 1));
    ancien.au = Some(d(2025, 8, 31));
    ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &ancien)
        .await
        .expect("période antérieure");
    // Valeurs refusées.
    let mut mauvaise = engagement("roi", "etat", d(2025, 9, 1));
    assert!(est_invalide(
        &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &mauvaise)
            .await
    ));
    mauvaise = engagement("titulaire", "etat", d(2025, 9, 1));
    mauvaise.quotite_pct = Some(120.0);
    assert!(est_invalide(
        &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &mauvaise)
            .await
    ));
    mauvaise.quotite_pct = None;
    mauvaise.jours_travailles = Some(vec![0, 7]);
    assert!(est_invalide(
        &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &mauvaise)
            .await
    ));
    mauvaise.jours_travailles = None;
    mauvaise.au = Some(d(2025, 1, 1));
    assert!(est_invalide(
        &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &mauvaise)
            .await
    ));
    // Correction : la ligne erronée est annulée, jamais supprimée, et libère la période.
    ent.corriger_engagement_for_actor(etab, dir, "direction", e1.id)
        .await
        .unwrap();
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_agent_engagement WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NOT NULL", etab, e1.id).await, 1);
    ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &maitre)
        .await
        .expect("période libérée");
}

#[tokio::test]
async fn disponibilite_ou_conge_suspend_l_engagement_sans_le_clore() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let admin = compte(&ent, etab, "admin").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let fiche = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap();
    let eng = fiche.engagements[0].id;
    ent.modifier_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng,
        &ModifierEngagementBody {
            nature: "titulaire".into(),
            employeur_type: "etat".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Suspension future : l'accès reste ouvert aujourd'hui.
    let future = SuspendreEngagementBody {
        du: jour + Duration::days(30),
        au: None,
        motif_code: "disponibilite".into(),
        acces_suspendu: true,
    };
    ent.suspendre_engagement_for_actor(etab, dir, "direction", eng, &future)
        .await
        .expect("suspension future");
    let s0 = connexion(&ent, etab, prof)
        .await
        .expect("accès ouvert avant la date");
    assert_eq!(
        ent.agent_du_compte(etab, prof, jour)
            .await
            .unwrap()
            .unwrap()
            .etat,
        "actif"
    );
    // Deux suspensions ne se recouvrent pas.
    assert!(est_conflit(
        &ent.suspendre_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &SuspendreEngagementBody {
                du: jour + Duration::days(40),
                au: None,
                motif_code: "autre".into(),
                acces_suspendu: true
            }
        )
        .await
    ));
    // Levée avant le début : la suspension est annulée.
    ent.reprendre_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng,
        &ReprendreEngagementBody {
            date_reprise: jour + Duration::days(1),
        },
    )
    .await
    .expect("annulation avant le début");

    // Disponibilité qui commence aujourd'hui : accès coupé, session invalide, engagement NON clos.
    ent.suspendre_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng,
        &SuspendreEngagementBody {
            du: jour,
            au: None,
            motif_code: "disponibilite".into(),
            acces_suspendu: true,
        },
    )
    .await
    .expect("disponibilité");
    assert!(
        connexion(&ent, etab, prof).await.is_err(),
        "connexion refusée pendant la disponibilité"
    );
    assert!(
        ent.resolve_session(&format!("Bearer {}", s0.token))
            .await
            .is_err(),
        "session ouverte avant : invalide"
    );
    let fiche = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap();
    assert_eq!(fiche.etat, "suspendu");
    let e = &fiche.engagements[0];
    assert!(
        e.au.is_none() && e.motif_fin.is_none(),
        "l'engagement n'est pas clos"
    );
    assert!(e.suspendu);
    assert_eq!(
        e.suspensions[0].motif_code.as_deref(),
        Some("disponibilite")
    );
    // L'agent suspendu est absent de l'annuaire courant, présent à la demande de la direction.
    let courant = ent
        .annuaire_for_actor(etab, dir, "direction", &FiltreAnnuaire::default())
        .await
        .unwrap();
    assert!(courant.iter().all(|a| a.agent_id != agent));
    let tous = ent
        .annuaire_for_actor(
            etab,
            dir,
            "direction",
            &FiltreAnnuaire {
                etats: vec!["suspendu".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(tous.iter().any(|a| a.agent_id == agent));
    // …mais seuls ceux qui lisent la fiche d'organisation voient les agents non actifs.
    assert!(est_interdit(
        &ent.annuaire_for_actor(
            etab,
            admin,
            "prof",
            &FiltreAnnuaire {
                etats: vec!["suspendu".into()],
                ..Default::default()
            }
        )
        .await
    ));
    let a = ent
        .agent_du_compte(etab, prof, jour)
        .await
        .unwrap()
        .expect("l'engagement est courant, suspendu");
    assert_eq!(a.etat, "suspendu");

    // Reprise à la date de début de la suspension : annulée, l'accès revient, SANS nouvelle invitation.
    let invitations_avant = compter(
        &ent,
        "SELECT count(*) FROM ent_password_reset WHERE user_id = $1 AND $2 = $2",
        prof,
        prof,
    )
    .await;
    ent.reprendre_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng,
        &ReprendreEngagementBody { date_reprise: jour },
    )
    .await
    .expect("reprise");
    connexion(&ent, etab, prof).await.expect("accès rétabli");
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_password_reset WHERE user_id = $1 AND $2 = $2",
            prof,
            prof
        )
        .await,
        invitations_avant
    );
    assert_eq!(
        ent.agent_du_compte(etab, prof, jour)
            .await
            .unwrap()
            .unwrap()
            .etat,
        "actif"
    );

    // Reprise à date : la suspension s'arrête la veille de la reprise (bornes incluses), levée tracée.
    ent.suspendre_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng,
        &SuspendreEngagementBody {
            du: jour,
            au: None,
            motif_code: "conge_parental".into(),
            acces_suspendu: false,
        },
    )
    .await
    .unwrap();
    // Congé qui garde l'accès (choix de la direction) : connexion possible, état de l'agent « suspendu ».
    connexion(&ent, etab, prof)
        .await
        .expect("accès conservé par choix");
    let reprise = ent
        .reprendre_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &ReprendreEngagementBody {
                date_reprise: jour + Duration::days(10),
            },
        )
        .await
        .unwrap();
    let s = reprise
        .suspensions
        .iter()
        .find(|s| s.levee)
        .expect("suspension levée");
    assert_eq!(s.au, Some(jour + Duration::days(9)));
    // Rien à lever.
    assert!(est_conflit(
        &ent.reprendre_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &ReprendreEngagementBody {
                date_reprise: jour + Duration::days(20)
            }
        )
        .await
    ));
    // Une suspension déborde-t-elle de l'engagement ?
    ent.terminer_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng,
        &TerminerEngagementBody {
            au: jour + Duration::days(100),
            motif_fin: "fin_contrat".into(),
        },
    )
    .await
    .unwrap();
    assert!(est_invalide(
        &ent.suspendre_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &SuspendreEngagementBody {
                du: jour + Duration::days(90),
                au: None,
                motif_code: "autre".into(),
                acces_suspendu: true
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.suspendre_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &SuspendreEngagementBody {
                du: jour,
                au: None,
                motif_code: "vacances".into(),
                acces_suspendu: true
            }
        )
        .await
    ));
}

#[tokio::test]
async fn suspension_d_un_engagement_le_maitre_garde_l_acces_par_son_autre_engagement() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let a_preciser = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap()
        .engagements[0]
        .id;
    ent.corriger_engagement_for_actor(etab, dir, "direction", a_preciser)
        .await
        .unwrap();
    let maitre = ent
        .ajouter_engagement_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &engagement("maitre_contractuel", "etat", d(2025, 9, 1)),
        )
        .await
        .unwrap();
    let mut ogec = engagement("salarie_organisme", "organisme_gestion", d(2025, 9, 1));
    ogec.employeur_organisme_id = Some(Uuid::new_v4());
    ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &ogec)
        .await
        .unwrap();
    ent.suspendre_engagement_for_actor(
        etab,
        dir,
        "direction",
        maitre.id,
        &SuspendreEngagementBody {
            du: jour,
            au: None,
            motif_code: "disponibilite".into(),
            acces_suspendu: true,
        },
    )
    .await
    .unwrap();
    // L'agent est toujours engagé à l'OGEC : l'accès reste ouvert, l'état aussi.
    connexion(&ent, etab, prof)
        .await
        .expect("accès conservé par l'engagement OGEC");
    assert_eq!(
        ent.agent_du_compte(etab, prof, jour)
            .await
            .unwrap()
            .unwrap()
            .etat,
        "actif"
    );
}

#[tokio::test]
async fn suspension_refusee_si_elle_prive_l_etablissement_de_son_dernier_administrateur() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let admin = compte(&ent, etab, "admin").await;
    let dir = compte(&ent, etab, "direction").await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let eng = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent_de(&ent, etab, admin).await)
        .await
        .unwrap()
        .engagements[0]
        .id;
    let corps = SuspendreEngagementBody {
        du: jour,
        au: None,
        motif_code: "conge_long".into(),
        acces_suspendu: true,
    };
    assert!(est_conflit(
        &ent.suspendre_engagement_for_actor(etab, dir, "direction", eng, &corps)
            .await
    ));
    let second = compte(&ent, etab, "admin").await;
    ent.suspendre_engagement_for_actor(etab, dir, "direction", eng, &corps)
        .await
        .expect("un autre administrateur reste");
    // Le premier administrateur est désormais suspendu : le second ne peut plus être fermé (SQL
    // direct compris) sans laisser l'établissement sans administrateur utilisable.
    let direct = sqlx::query(
        "UPDATE ent_membership SET etat = 'ferme' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(second)
    .execute(ent.pool())
    .await;
    assert!(
        direct.is_err(),
        "le dernier administrateur utilisable est protégé"
    );
    let (ouvert,): (bool,) = sqlx::query_as("SELECT ent_acces_ouvert($1, $2)")
        .bind(second)
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(ouvert);
}

#[tokio::test]
async fn motifs_de_fin_codes_natures_limitees_et_prolongation() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    let referent = agent_de(&ent, etab, dir).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let eng = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap()
        .engagements[0]
        .id;

    // Motif libre refusé ; motif de R-P13 accepté ; jamais de texte.
    let t = |motif: &str| TerminerEngagementBody {
        au: jour + Duration::days(5),
        motif_fin: motif.into(),
    };
    assert!(est_invalide(
        &ent.terminer_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &t("il est parti en pleurant")
        )
        .await
    ));
    let fini = ent
        .terminer_engagement_for_actor(etab, dir, "direction", eng, &t("rupture_conventionnelle"))
        .await
        .unwrap();
    assert_eq!(fini.motif_fin.as_deref(), Some("rupture_conventionnelle"));
    assert_eq!(fini.au, Some(jour + Duration::days(5)));
    // Prolongation : la fin est décalée, la ligne et l'historique restent.
    let prolonge = ent
        .prolonger_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &ProlongerEngagementBody {
                au: Some(jour + Duration::days(60)),
            },
        )
        .await
        .unwrap();
    assert_eq!(prolonge.au, Some(jour + Duration::days(60)));
    let sans_fin = ent
        .prolonger_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &ProlongerEngagementBody { au: None },
        )
        .await
        .unwrap();
    assert!(sans_fin.au.is_none() && sans_fin.motif_fin.is_none());
    // La fin ne précède pas le début.
    assert!(est_invalide(
        &ent.terminer_engagement_for_actor(
            etab,
            dir,
            "direction",
            eng,
            &TerminerEngagementBody {
                au: d(2000, 1, 1),
                motif_fin: "autre".into()
            }
        )
        .await
    ));

    // Natures limitées : fin (un an au plus) et référent obligatoires.
    let mut interv = engagement("intervenant", "aucun", jour);
    assert!(
        est_invalide(
            &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &interv)
                .await
        ),
        "sans fin"
    );
    interv.au = Some(jour + Duration::days(30));
    assert!(
        est_invalide(
            &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &interv)
                .await
        ),
        "sans référent"
    );
    interv.referent_agent_id = Some(agent);
    assert!(
        est_invalide(
            &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &interv)
                .await
        ),
        "son propre référent"
    );
    interv.referent_agent_id = Some(referent);
    interv.au = Some(jour + Duration::days(400));
    assert!(
        est_invalide(
            &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &interv)
                .await
        ),
        "plus d'un an"
    );
    interv.au = Some(jour + Duration::days(300));
    ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &interv)
        .await
        .expect("intervenant daté avec référent");
    // Référent d'un autre établissement : refusé.
    let autre = Uuid::new_v4();
    let dir_autre = compte(&ent, autre, "direction").await;
    let agent_autre = agent_de(&ent, autre, dir_autre).await;
    interv.referent_agent_id = Some(agent_autre);
    let mut autre_interv = interv.clone();
    autre_interv.nature = "benevole".into();
    assert!(est_invalide(
        &ent.ajouter_engagement_for_actor(etab, dir, "direction", agent, &autre_interv)
            .await
    ));
}

#[tokio::test]
async fn matrice_de_droits_serveur_et_roles_parametres() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let admin = compte(&ent, etab, "admin").await;
    let dir = compte(&ent, etab, "direction").await;
    let agent = agent_de(&ent, etab, dir).await;
    let corps = nouvelle("Nouveau", "Agent", None);
    // Seuls Admin et Direction créent (comportement actuel de `provisionner_personnel` conservé).
    for role in [
        "administratif",
        "cpe",
        "compta",
        "prof",
        "infirmerie",
        "psy",
        "accueil",
    ] {
        let u = compte(&ent, etab, role).await;
        assert!(
            est_interdit(&ent.creer_agent_for_actor(etab, u, role, &corps).await),
            "{role} ne crée pas d'agent"
        );
        assert!(
            est_interdit(
                &ent.ajouter_engagement_for_actor(
                    etab,
                    u,
                    role,
                    agent,
                    &engagement("titulaire", "etat", d(2025, 9, 1))
                )
                .await
            ),
            "{role}"
        );
        assert!(est_interdit(
            &ent.definir_identite_restreinte_for_actor(
                etab,
                u,
                role,
                agent,
                &IdentiteRestreinteBody {
                    restreinte: true,
                    motif: Some("menace".into())
                }
            )
            .await
        ));
        // L'annuaire est ouvert à tout le personnel (comme `/personnel/search`).
        assert!(
            !ent.annuaire_for_actor(etab, u, role, &FiltreAnnuaire::default())
                .await
                .unwrap()
                .is_empty(),
            "{role}"
        );
        // Fiche d'organisation : Administratif, Direction, Admin seulement.
        let lit = ent.fiche_agent_for_actor(etab, u, role, agent).await;
        assert_eq!(lit.is_ok(), role == "administratif", "{role}");
    }
    // Familles : aucun droit.
    for role in ["parent", "eleve"] {
        let u = compte(&ent, etab, role).await;
        assert!(est_interdit(
            &ent.annuaire_for_actor(etab, u, role, &FiltreAnnuaire::default())
                .await
        ));
    }
    creer(&ent, etab, dir, &corps).await;
    ent.creer_agent_for_actor(etab, admin, "admin", &nouvelle("Autre", "Agent", None))
        .await
        .expect("admin crée");

    // Un rôle paramétré peut ouvrir la gestion des agents au secrétariat (Q-P23), au niveau établissement.
    let secretaire = compte(&ent, etab, "administratif").await;
    let role = ent
        .creer_role_for_actor(
            etab,
            dir,
            "direction",
            RoleSaisie {
                nom: format!(
                    "Gestion des agents {}",
                    &Uuid::new_v4().simple().to_string()[..6]
                ),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::PersonnelsGerer,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    assert!(est_interdit(
        &ent.creer_agent_for_actor(
            etab,
            secretaire,
            "administratif",
            &nouvelle("Via", "Role", None)
        )
        .await
    ));
    ent.attribuer_role_for_actor(etab, dir, "direction", role.id, secretaire, None)
        .await
        .unwrap();
    ent.creer_agent_for_actor(
        etab,
        secretaire,
        "administratif",
        &nouvelle("Via", "Role", None),
    )
    .await
    .expect("rôle paramétré");
    // La portée « mes classes » n'a aucun sens pour un agent : refusée par le catalogue.
    assert!(ent
        .creer_role_for_actor(
            etab,
            dir,
            "direction",
            RoleSaisie {
                nom: "Rogue".into(),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::PersonnelsGerer,
                    portee: Portee::MesClasses,
                    cibles: vec![]
                }]
            }
        )
        .await
        .is_err());
    // Anti-escalade : on ne peut accorder `personnels.gerer` que si on le détient.
    let cpe = compte(&ent, etab, "cpe").await;
    assert!(ent
        .creer_role_for_actor(
            etab,
            cpe,
            "cpe",
            RoleSaisie {
                nom: "Escalade".into(),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::PersonnelsGerer,
                    portee: Portee::Etablissement,
                    cibles: vec![]
                }]
            }
        )
        .await
        .is_err());
    // L'identité d'un agent : `personnels.gerer` OU `personnes.gerer`.
    let identite = ModifierIdentiteBody {
        nom: "Corrigé".into(),
        prenom: Some("Faute".into()),
        ..Default::default()
    };
    ent.modifier_identite_agent_for_actor(etab, dir, "direction", agent, &identite)
        .await
        .expect("direction");
    let adm_scol = compte(&ent, etab, "administratif").await;
    ent.modifier_identite_agent_for_actor(etab, adm_scol, "administratif", agent, &identite)
        .await
        .expect("secrétariat (personnes.gerer)");
    let prof = compte(&ent, etab, "prof").await;
    assert!(est_interdit(
        &ent.modifier_identite_agent_for_actor(etab, prof, "prof", agent, &identite)
            .await
    ));
}

#[tokio::test]
async fn isolation_entre_etablissements_commandes_lectures_et_rls() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let dir_a = compte(&ent, a, "direction").await;
    let dir_b = compte(&ent, b, "direction").await;
    let prof_a = compte(&ent, a, "prof").await;
    let agent_a = agent_de(&ent, a, prof_a).await;
    let eng_a = ent
        .fiche_agent_for_actor(a, dir_a, "direction", agent_a)
        .await
        .unwrap()
        .engagements[0]
        .id;
    let jour = ent.aujourdhui_etablissement(a).await.unwrap();

    // L'autre direction ne lit, n'écrit, ne suspend, ne corrige rien chez A : 404 uniforme.
    assert!(est_introuvable(
        &ent.fiche_agent_for_actor(b, dir_b, "direction", agent_a)
            .await
    ));
    assert!(est_introuvable(
        &ent.corriger_engagement_for_actor(b, dir_b, "direction", eng_a)
            .await
    ));
    assert!(est_introuvable(
        &ent.suspendre_engagement_for_actor(
            b,
            dir_b,
            "direction",
            eng_a,
            &SuspendreEngagementBody {
                du: jour,
                au: None,
                motif_code: "autre".into(),
                acces_suspendu: true
            }
        )
        .await
    ));
    assert!(est_introuvable(
        &ent.ajouter_engagement_for_actor(
            b,
            dir_b,
            "direction",
            agent_a,
            &engagement("titulaire", "etat", d(2025, 9, 1))
        )
        .await
    ));
    assert!(est_introuvable(
        &ent.ajouter_identifiant_agent_for_actor(
            b,
            dir_b,
            "direction",
            agent_a,
            &IdentifiantAgentBody {
                systeme: "sts".into(),
                valeur: "X1".into()
            }
        )
        .await
    ));
    assert!(est_introuvable(
        &ent.definir_identite_restreinte_for_actor(
            b,
            dir_b,
            "direction",
            agent_a,
            &IdentiteRestreinteBody {
                restreinte: true,
                motif: Some("menace".into())
            }
        )
        .await
    ));
    // L'agent de A n'apparaît pas dans l'annuaire de B ; le compte de A n'a pas d'agent chez B.
    let annuaire_b = ent
        .annuaire_for_actor(
            b,
            dir_b,
            "direction",
            &FiltreAnnuaire {
                q: Some("Nom prof".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(annuaire_b.iter().all(|x| x.agent_id != agent_a));
    assert!(ent
        .agent_du_compte(b, prof_a, jour)
        .await
        .unwrap()
        .is_none());
    // Fonction propre à B introuvable en A ; fonction fournie visible partout.
    ent.creer_fonction_for_actor(
        b,
        dir_b,
        "direction",
        &CreerFonctionBody {
            code: "pastorale".into(),
            libelle: "Animateur en pastorale".into(),
            famille: "intervention".into(),
        },
    )
    .await
    .unwrap();
    let cat_a = ent
        .fonctions_catalogue_for_actor(a, dir_a, "direction")
        .await
        .unwrap();
    assert!(
        cat_a.iter().all(|f| f.code != "pastorale")
            && cat_a
                .iter()
                .any(|f| f.code == "enseignant" && !f.propre_etablissement)
    );
    assert!(est_invalide(
        &ent.ajouter_fonction_agent_for_actor(
            a,
            dir_a,
            "direction",
            agent_a,
            &AgentFonctionBody {
                fonction_code: "pastorale".into(),
                principale: false,
                du: None,
                au: None
            }
        )
        .await
    ));
    // Un code fourni ne se redéfinit pas.
    assert!(est_conflit(
        &ent.creer_fonction_for_actor(
            b,
            dir_b,
            "direction",
            &CreerFonctionBody {
                code: "enseignant".into(),
                libelle: "Prof".into(),
                famille: "enseignement".into()
            }
        )
        .await
    ));
    // Identifiant externe : unique par établissement ; la même valeur ailleurs est un autre agent.
    let prof_b = compte(&ent, b, "prof").await;
    let agent_b = agent_de(&ent, b, prof_b).await;
    let idf = IdentifiantAgentBody {
        systeme: "sts".into(),
        valeur: "STS-0042".into(),
    };
    ent.ajouter_identifiant_agent_for_actor(a, dir_a, "direction", agent_a, &idf)
        .await
        .unwrap();
    ent.ajouter_identifiant_agent_for_actor(b, dir_b, "direction", agent_b, &idf)
        .await
        .expect("autre établissement");
    assert!(est_conflit(
        &ent.ajouter_identifiant_agent_for_actor(
            a,
            dir_a,
            "direction",
            agent_de(&ent, a, dir_a).await,
            &idf
        )
        .await
    ));
    assert_eq!(
        ent.agent_par_identifiant(a, "sts", "STS-0042")
            .await
            .unwrap(),
        Some(agent_a)
    );
    assert_eq!(
        ent.agent_par_identifiant(b, "sts", "STS-0042")
            .await
            .unwrap(),
        Some(agent_b)
    );

    // RLS : rôle applicatif non propriétaire, sans BYPASSRLS.
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let admin = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let tables = "ent_agent, ent_agent_engagement, ent_agent_engagement_suspension, ent_agent_fonction, ent_agent_discipline, \
                  ent_agent_qualification, ent_agent_contact_urgence, ent_agent_identifiant, ent_fonction, ent_personne";
    for sql in [
        "DO $$ BEGIN CREATE ROLE ent_p1_app LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; EXCEPTION WHEN duplicate_object THEN NULL; END $$".to_string(),
        "GRANT USAGE ON SCHEMA public TO ent_p1_app".to_string(),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON {tables} TO ent_p1_app"),
        "ALTER ROLE ent_p1_app SET app.rls_strict = 'on'".to_string(),
    ] {
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username("ent_p1_app").unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap();
    // Sans TenantTx : fail-closed.
    let (rien,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_agent")
        .fetch_one(&app)
        .await
        .unwrap();
    assert_eq!(rien, 0);
    for table in [
        "ent_agent",
        "ent_agent_engagement",
        "ent_agent_fonction",
        "ent_agent_identifiant",
    ] {
        let mut tx =
            crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict)
                .await
                .unwrap();
        let (chez_b,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
        ))
        .bind(b)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        let (chez_a,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
        ))
        .bind(a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(chez_b, 0, "{table} : lignes de B visibles depuis A");
        assert!(chez_a > 0, "{table} : lignes de A invisibles de A");
        // Écrire pour un autre établissement est refusé par la politique.
        if table == "ent_agent_identifiant" {
            let ecriture = sqlx::query("INSERT INTO ent_agent_identifiant (id, etablissement_id, agent_id, systeme, valeur, du) VALUES ($1,$2,$3,'gar','z', CURRENT_DATE)")
                .bind(Uuid::new_v4()).bind(b).bind(agent_b).execute(&mut *tx).await;
            assert!(ecriture.is_err(), "écriture inter-établissement refusée");
        }
    }
    // Catalogue : les fonctions fournies restent lisibles, celles de B non.
    let mut tx = crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict)
        .await
        .unwrap();
    let (fournies,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_fonction WHERE etablissement_id IS NULL")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let (de_b,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_fonction WHERE etablissement_id = $1")
            .bind(b)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(fournies > 20);
    assert_eq!(de_b, 0);
}

#[tokio::test]
async fn annuaire_parite_avec_l_existant_et_vue_des_familles() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let mut comptes = vec![dir];
    for role in ["admin", "prof", "prof", "cpe", "administratif", "accueil"] {
        comptes.push(compte(&ent, etab, role).await);
    }
    // Un accès fermé par la direction (P0) sort de l'annuaire, comme de la recherche historique.
    let parti = compte(&ent, etab, "prof").await;
    let admin = comptes[1];
    ent.acces_fermer_for_actor(etab, admin, "admin", parti, "depart")
        .await
        .unwrap();

    let ancien: std::collections::BTreeSet<Uuid> = ent
        .search_staff(etab, "", 100)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.user_id)
        .collect();
    let nouveau = ent
        .annuaire_for_actor(
            etab,
            dir,
            "direction",
            &FiltreAnnuaire {
                limite: Some(100),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let nouveau_users: std::collections::BTreeSet<Uuid> =
        nouveau.iter().filter_map(|a| a.user_id).collect();
    assert_eq!(
        ancien, nouveau_users,
        "mêmes personnes que `/personnel/search`"
    );
    assert!(!nouveau_users.contains(&parti));
    // Jamais de téléphone : le type n'en porte pas ; une personne avec téléphone ne le fait pas apparaître.
    sqlx::query("UPDATE ent_personne SET telephone = '06 11 22 33 44' WHERE etablissement_id = $1 AND user_id = $2").bind(etab).bind(comptes[2]).execute(ent.pool()).await.unwrap();
    let json = serde_json::to_string(
        &ent.annuaire_for_actor(etab, dir, "direction", &FiltreAnnuaire::default())
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(!json.contains("06 11 22 33 44"));
    // Recherche par fonction et par nom ; les jokers SQL sont des caractères ordinaires.
    let profs = ent
        .annuaire_for_actor(
            etab,
            dir,
            "direction",
            &FiltreAnnuaire {
                fonction_code: Some("enseignant".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(profs.len(), 2);
    assert!(ent
        .annuaire_for_actor(
            etab,
            dir,
            "direction",
            &FiltreAnnuaire {
                q: Some("%".into()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .is_empty());

    // Vue des familles (port) : identité restreinte masquée, mineur exclu, pas d'e-mail ni de compte.
    let agent_prof = agent_de(&ent, etab, comptes[2]).await;
    let agent_cpe = agent_de(&ent, etab, comptes[4]).await;
    ent.modifier_identite_agent_for_actor(
        etab,
        dir,
        "direction",
        agent_prof,
        &ModifierIdentiteBody {
            nom: "Rogue".into(),
            prenom: Some("Severus".into()),
            civilite: Some("m".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    ent.definir_identite_restreinte_for_actor(
        etab,
        dir,
        "direction",
        agent_prof,
        &IdentiteRestreinteBody {
            restreinte: true,
            motif: Some("menace".into()),
        },
    )
    .await
    .unwrap();
    ent.definir_personnel_mineur_for_actor(
        etab,
        dir,
        "direction",
        agent_cpe,
        &PersonnelMineurBody {
            personnel_mineur: true,
        },
    )
    .await
    .unwrap();
    let mut conn = ent.pool().acquire().await.unwrap();
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let famille = crate::annuaire_tx(
        &mut conn,
        etab,
        NiveauAnnuaire::Famille,
        &FiltreAnnuaire {
            limite: Some(100),
            ..Default::default()
        },
        jour,
    )
    .await
    .unwrap();
    let rogue = famille
        .iter()
        .find(|a| a.agent_id == agent_prof)
        .expect("visible, masqué");
    assert_eq!(rogue.nom_affiche, "M. R.");
    assert!(rogue.email.is_none() && rogue.user_id.is_none() && !rogue.identite_restreinte);
    assert!(
        famille.iter().all(|a| a.agent_id != agent_cpe),
        "personnel mineur exclu"
    );
    assert!(famille
        .iter()
        .all(|a| a.email.is_none() && a.user_id.is_none()));
    // Collègues : nom complet.
    let interne = ent
        .annuaire_for_actor(
            etab,
            dir,
            "direction",
            &FiltreAnnuaire {
                limite: Some(100),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        interne
            .iter()
            .find(|a| a.agent_id == agent_prof)
            .unwrap()
            .nom_affiche,
        "Severus Rogue"
    );
    // Choix de l'agent pour les familles.
    assert_eq!(
        crate::nom_pour_lecteur(
            NiveauAnnuaire::Famille,
            "Chourave",
            Some("Pomona"),
            Some("mme"),
            None,
            false,
            None
        ),
        "Mme Chourave"
    );
    assert_eq!(
        crate::nom_pour_lecteur(
            NiveauAnnuaire::Famille,
            "Chourave",
            Some("Pomona"),
            Some("mme"),
            Some("Mme P. Chourave"),
            false,
            None
        ),
        "Mme P. Chourave"
    );
    assert_eq!(
        crate::nom_pour_lecteur(
            NiveauAnnuaire::Eleve,
            "X",
            None,
            None,
            None,
            true,
            Some("Professeur")
        ),
        "Professeur"
    );
}

#[tokio::test]
async fn fonctions_disciplines_qualifications_et_historique() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    // Une seule fonction principale à la fois.
    let r = ent
        .ajouter_fonction_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentFonctionBody {
                fonction_code: "documentaliste".into(),
                principale: true,
                du: None,
                au: None,
            },
        )
        .await;
    assert!(est_conflit(&r));
    let sec = ent
        .ajouter_fonction_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentFonctionBody {
                fonction_code: "documentaliste".into(),
                principale: false,
                du: None,
                au: None,
            },
        )
        .await
        .unwrap();
    assert!(!sec.principale);
    // Même fonction deux fois sur la même période : refusé. Fonction inconnue : refusée.
    assert!(est_conflit(
        &ent.ajouter_fonction_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentFonctionBody {
                fonction_code: "documentaliste".into(),
                principale: false,
                du: None,
                au: None
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.ajouter_fonction_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentFonctionBody {
                fonction_code: "grand_vizir".into(),
                principale: false,
                du: None,
                au: None
            }
        )
        .await
    ));
    ent.terminer_fonction_agent_for_actor(
        etab,
        dir,
        "direction",
        sec.id,
        &TerminerLigneBody {
            au: jour + Duration::days(10),
        },
    )
    .await
    .unwrap();
    assert!(est_invalide(
        &ent.terminer_fonction_agent_for_actor(
            etab,
            dir,
            "direction",
            sec.id,
            &TerminerLigneBody { au: d(2000, 1, 1) }
        )
        .await
    ));
    // Discipline : matière ou code national.
    let matiere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1,$2,'HG','Histoire-géographie')").bind(matiere).bind(etab).execute(ent.pool()).await.unwrap();
    let disc = ent
        .ajouter_discipline_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentDisciplineBody {
                matiere_id: Some(matiere),
                code_discipline_poste: None,
                principale: true,
                du: None,
                au: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(disc.matiere_id, Some(matiere));
    assert!(est_invalide(
        &ent.ajouter_discipline_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentDisciplineBody {
                matiere_id: None,
                code_discipline_poste: None,
                principale: false,
                du: None,
                au: None
            }
        )
        .await
    ));
    // Matière d'un autre établissement : refusée (clé composée).
    let autre = Uuid::new_v4();
    let matiere_autre = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1,$2,'MA','Maths')",
    )
    .bind(matiere_autre)
    .bind(autre)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(ent
        .ajouter_discipline_agent_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &AgentDisciplineBody {
                matiere_id: Some(matiere_autre),
                code_discipline_poste: None,
                principale: false,
                du: None,
                au: None
            }
        )
        .await
        .is_err());
    // Qualifications (R-P14) : expiration lue à la date, correction par annulation.
    let q = ent
        .enregistrer_qualification_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &QualificationBody {
                code: "secourisme".into(),
                precision: Some("PSC1".into()),
                obtenue_le: Some(d(2023, 1, 10)),
                valable_jusqu_au: Some(d(2024, 1, 10)),
            },
        )
        .await
        .unwrap();
    assert!(q.expiree);
    ent.enregistrer_qualification_for_actor(
        etab,
        dir,
        "direction",
        agent,
        &QualificationBody {
            code: "sst".into(),
            precision: None,
            obtenue_le: None,
            valable_jusqu_au: Some(jour + Duration::days(200)),
        },
    )
    .await
    .unwrap();
    assert!(est_invalide(
        &ent.enregistrer_qualification_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &QualificationBody {
                code: "magie".into(),
                precision: None,
                obtenue_le: None,
                valable_jusqu_au: None
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.enregistrer_qualification_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &QualificationBody {
                code: "permis_b".into(),
                precision: None,
                obtenue_le: Some(d(2024, 1, 1)),
                valable_jusqu_au: Some(d(2023, 1, 1))
            }
        )
        .await
    ));
    ent.annuler_qualification_for_actor(etab, dir, "direction", q.id)
        .await
        .unwrap();
    let fiche = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap();
    assert_eq!(fiche.qualifications.len(), 1);
    assert_eq!(fiche.fonctions.len(), 2);
    assert_eq!(fiche.disciplines.len(), 1);
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_agent_qualification WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NOT NULL", etab, q.id).await, 1);
}

#[tokio::test]
async fn journal_et_evenements_sans_donnee_personnelle() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let c = creer(
        &ent,
        etab,
        dir,
        &nouvelle(
            "Zorglub-Identifiable",
            "Hyacinthe",
            Some("hyacinthe.z@example.org"),
        ),
    )
    .await;
    let eng = ent
        .ajouter_engagement_for_actor(
            etab,
            dir,
            "direction",
            c.agent_id,
            &engagement("contractuel", "etat", d(2025, 9, 1)),
        )
        .await
        .unwrap();
    ent.suspendre_engagement_for_actor(
        etab,
        dir,
        "direction",
        eng.id,
        &SuspendreEngagementBody {
            du: d(2026, 2, 1),
            au: Some(d(2026, 3, 1)),
            motif_code: "conge_parental".into(),
            acces_suspendu: true,
        },
    )
    .await
    .unwrap();
    ent.definir_identite_restreinte_for_actor(
        etab,
        dir,
        "direction",
        c.agent_id,
        &IdentiteRestreinteBody {
            restreinte: true,
            motif: Some("menace".into()),
        },
    )
    .await
    .unwrap();
    ent.ajouter_identifiant_agent_for_actor(
        etab,
        dir,
        "direction",
        c.agent_id,
        &IdentifiantAgentBody {
            systeme: "sts".into(),
            valeur: "SECRET-STS-77".into(),
        },
    )
    .await
    .unwrap();

    let (journal,): (String,) = sqlx::query_as("SELECT COALESCE(string_agg(action || ' ' || libelle || ' ' || payload::text, ' | '), '') FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'personnels'")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    let (evenements,): (String,) = sqlx::query_as("SELECT COALESCE(string_agg(type || ' ' || payload::text, ' | '), '') FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'personnel.%'")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    for etat in [&journal, &evenements] {
        for interdit in [
            "Zorglub",
            "Hyacinthe",
            "example.org",
            "SECRET-STS-77",
            "conge_parental",
            "menace",
        ] {
            assert!(
                !etat.contains(interdit),
                "« {interdit} » ne doit figurer ni au journal ni dans un événement : {etat}"
            );
        }
    }
    for attendu in [
        "agent_cree",
        "engagement_ajoute",
        "engagement_suspendu",
        "identite_restreinte",
        "identifiant_ajoute",
    ] {
        assert!(journal.contains(attendu), "journal : {attendu}");
    }
    for attendu in [
        "personnel.cree",
        "personnel.engagement_ajoute",
        "personnel.engagement_suspendu",
        "personnel.identite_restreinte",
    ] {
        assert!(evenements.contains(attendu), "événement : {attendu}");
    }
    // Les noms d'événements consommés s'alignent sur ceux du socle 5 (R-P26).
    assert_eq!(
        ent_events::types::REFERENTIEL_ANNEE_OUVERTE,
        "referentiel.annee_ouverte"
    );
    assert_eq!(
        ent_events::types::REFERENTIEL_CLASSE_ARCHIVEE,
        "referentiel.classe_archivee"
    );
}

#[tokio::test]
async fn contact_d_urgence_et_choix_de_l_agent_confidentiels() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let secretaire = compte(&ent, etab, "administratif").await;
    let prof = compte(&ent, etab, "prof").await;
    let collegue = compte(&ent, etab, "prof").await;
    let agent = agent_de(&ent, etab, prof).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let contact = ContactUrgenceAgentBody {
        ordre: 1,
        nom: "Conjoint-Secret".into(),
        prenom: Some("Albert".into()),
        lien: Some("conjoint".into()),
        telephone: "06 99 88 77 66".into(),
    };
    // Seul l'agent déclare son contact (pas la direction à sa place) ; un parent n'est pas agent.
    assert!(est_interdit(
        &ent.definir_contact_urgence_for_actor(
            etab,
            compte(&ent, etab, "parent").await,
            "parent",
            &contact
        )
        .await
    ));
    ent.definir_contact_urgence_for_actor(etab, prof, "prof", &contact)
        .await
        .unwrap();
    assert!(est_invalide(
        &ent.definir_contact_urgence_for_actor(
            etab,
            prof,
            "prof",
            &ContactUrgenceAgentBody {
                ordre: 4,
                ..contact.clone()
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.definir_contact_urgence_for_actor(
            etab,
            prof,
            "prof",
            &ContactUrgenceAgentBody {
                telephone: "abc".into(),
                ..contact.clone()
            }
        )
        .await
    ));
    // Lecture : l'agent, la direction (tracée) ; pas le secrétariat, pas un collègue.
    let soi = ent
        .fiche_agent_for_actor(etab, prof, "prof", agent)
        .await
        .unwrap();
    assert_eq!(soi.contacts_urgence.as_ref().unwrap().len(), 1);
    assert!(est_interdit(
        &ent.fiche_agent_for_actor(etab, collegue, "prof", agent)
            .await
    ));
    let vue_secretariat = ent
        .fiche_agent_for_actor(etab, secretaire, "administratif", agent)
        .await
        .unwrap();
    assert!(
        vue_secretariat.contacts_urgence.is_none(),
        "contact d'urgence : direction seule"
    );
    assert!(vue_secretariat.identite_restreinte_motif.is_none());
    let vue_dir = ent
        .fiche_agent_for_actor(etab, dir, "direction", agent)
        .await
        .unwrap();
    assert_eq!(
        vue_dir.contacts_urgence.unwrap()[0].telephone,
        "06 99 88 77 66"
    );
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'contact_urgence_lu' AND $2 = $2", etab, etab).await, 1);
    // Remplacement le même jour sur place ; retrait.
    ent.definir_contact_urgence_for_actor(
        etab,
        prof,
        "prof",
        &ContactUrgenceAgentBody {
            nom: "Nouveau".into(),
            ..contact.clone()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        ent.fiche_agent_for_actor(etab, prof, "prof", agent)
            .await
            .unwrap()
            .contacts_urgence
            .unwrap()[0]
            .nom,
        "Nouveau"
    );
    ent.retirer_contact_urgence_for_actor(etab, prof, "prof", 1)
        .await
        .unwrap();
    assert!(ent
        .fiche_agent_for_actor(etab, prof, "prof", agent)
        .await
        .unwrap()
        .contacts_urgence
        .unwrap()
        .is_empty());
    // Aucun nom ni numéro au journal.
    let (journal,): (String,) = sqlx::query_as("SELECT COALESCE(string_agg(payload::text || libelle, ' '), '') FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'personnels'").bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert!(!journal.contains("Conjoint-Secret") && !journal.contains("06 99 88 77 66"));

    // Pause des notifications (au plus un an) ; photo : consentement daté.
    ent.definir_pause_notifications_for_actor(
        etab,
        prof,
        "prof",
        &PauseNotificationsBody {
            jusqu_au: Some(jour + Duration::days(30)),
        },
    )
    .await
    .unwrap();
    assert!(est_invalide(
        &ent.definir_pause_notifications_for_actor(
            etab,
            prof,
            "prof",
            &PauseNotificationsBody {
                jusqu_au: Some(jour + Duration::days(900))
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.definir_pause_notifications_for_actor(
            etab,
            prof,
            "prof",
            &PauseNotificationsBody {
                jusqu_au: Some(jour - Duration::days(1))
            }
        )
        .await
    ));
    ent.definir_photo_agent_for_actor(
        etab,
        prof,
        "prof",
        &PhotoAgentBody {
            visible_collegues: true,
            visible_eleves: false,
            visible_familles: false,
        },
    )
    .await
    .unwrap();
    let f = ent
        .fiche_agent_for_actor(etab, prof, "prof", agent)
        .await
        .unwrap();
    assert_eq!(
        f.notifications_en_pause_jusqu_au,
        Some(jour + Duration::days(30))
    );
    assert!(f.photo_visible_collegues && !f.photo_visible_eleves && !f.photo_visible_familles);
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_agent WHERE etablissement_id = $1 AND id = $2 AND photo_consentement_le IS NOT NULL", etab, agent).await, 1);
    // Identité restreinte : motif codé obligatoire, lu de la direction seule.
    assert!(est_invalide(
        &ent.definir_identite_restreinte_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &IdentiteRestreinteBody {
                restreinte: true,
                motif: None
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.definir_identite_restreinte_for_actor(
            etab,
            dir,
            "direction",
            agent,
            &IdentiteRestreinteBody {
                restreinte: true,
                motif: Some("jalousie".into())
            }
        )
        .await
    ));
    ent.definir_identite_restreinte_for_actor(
        etab,
        dir,
        "direction",
        agent,
        &IdentiteRestreinteBody {
            restreinte: true,
            motif: Some("menace".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        ent.fiche_agent_for_actor(etab, dir, "direction", agent)
            .await
            .unwrap()
            .identite_restreinte_motif
            .as_deref(),
        Some("menace")
    );
    assert!(ent
        .fiche_agent_for_actor(etab, secretaire, "administratif", agent)
        .await
        .unwrap()
        .identite_restreinte_motif
        .is_none());
    ent.definir_identite_restreinte_for_actor(
        etab,
        dir,
        "direction",
        agent,
        &IdentiteRestreinteBody {
            restreinte: false,
            motif: None,
        },
    )
    .await
    .unwrap();
    assert!(
        !ent.fiche_agent_for_actor(etab, dir, "direction", agent)
            .await
            .unwrap()
            .identite_restreinte
    );
}

#[tokio::test]
async fn fusion_de_deux_agents_en_double() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let admin = compte(&ent, etab, "admin").await;
    // Doublon : deux fiches pour la même personne, l'une avec compte, l'autre créée à la main.
    let prof = compte(&ent, etab, "prof").await;
    let garde = agent_de(&ent, etab, prof).await;
    let a_corriger = ent
        .fiche_agent_for_actor(etab, dir, "direction", garde)
        .await
        .unwrap()
        .engagements[0]
        .id;
    ent.corriger_engagement_for_actor(etab, dir, "direction", a_corriger)
        .await
        .unwrap();
    let doublon = creer(&ent, etab, dir, &nouvelle("Doublon", "Dora", None)).await;
    ent.ajouter_engagement_for_actor(
        etab,
        dir,
        "direction",
        doublon.agent_id,
        &engagement("vacataire", "association", d(2025, 9, 1)),
    )
    .await
    .unwrap();
    ent.enregistrer_qualification_for_actor(
        etab,
        dir,
        "direction",
        doublon.agent_id,
        &QualificationBody {
            code: "bafa".into(),
            precision: None,
            obtenue_le: None,
            valable_jusqu_au: None,
        },
    )
    .await
    .unwrap();
    ent.ajouter_identifiant_agent_for_actor(
        etab,
        dir,
        "direction",
        doublon.agent_id,
        &IdentifiantAgentBody {
            systeme: "edt_logiciel".into(),
            valeur: "DORA".into(),
        },
    )
    .await
    .unwrap();
    ent.definir_identite_restreinte_for_actor(
        etab,
        dir,
        "direction",
        doublon.agent_id,
        &IdentiteRestreinteBody {
            restreinte: true,
            motif: Some("autre".into()),
        },
    )
    .await
    .unwrap();

    // La fusion de personnes du socle 4 refuse deux agents sans passer par les personnels.
    let (p_src,): (Uuid,) = sqlx::query_as("SELECT personne_id FROM ent_agent WHERE id = $1")
        .bind(doublon.agent_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let (p_cible,): (Uuid,) = sqlx::query_as("SELECT personne_id FROM ent_agent WHERE id = $1")
        .bind(garde)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let direct = ent
        .fusionner_personnes_for_actor(etab, dir, "direction", p_src, p_cible, "doublon de saisie")
        .await;
    assert!(matches!(direct, Err(EntDbError::ConflitDossier(m)) if m.contains("agents")));
    // Moins de droits, motif vide, soi-même, agent d'un autre établissement : refusés.
    let corps = FusionnerAgentsBody {
        source_agent_id: doublon.agent_id,
        cible_agent_id: garde,
        motif: "doublon de saisie".into(),
    };
    assert!(est_interdit(
        &ent.fusionner_agents_for_actor(etab, compte(&ent, etab, "prof").await, "prof", &corps)
            .await
    ));
    assert!(est_invalide(
        &ent.fusionner_agents_for_actor(
            etab,
            dir,
            "direction",
            &FusionnerAgentsBody {
                motif: " ".into(),
                ..corps.clone()
            }
        )
        .await
    ));
    assert!(est_invalide(
        &ent.fusionner_agents_for_actor(
            etab,
            dir,
            "direction",
            &FusionnerAgentsBody {
                source_agent_id: garde,
                ..corps.clone()
            }
        )
        .await
    ));
    let (autre, autre_agent) = {
        let e = Uuid::new_v4();
        let u = compte(&ent, e, "direction").await;
        (e, agent_de(&ent, e, u).await)
    };
    let _ = autre;
    assert!(est_introuvable(
        &ent.fusionner_agents_for_actor(
            etab,
            dir,
            "direction",
            &FusionnerAgentsBody {
                cible_agent_id: autre_agent,
                ..corps.clone()
            }
        )
        .await
    ));

    let r = ent
        .fusionner_agents_for_actor(etab, dir, "direction", &corps)
        .await
        .expect("fusion");
    assert_eq!(r.engagements_deplaces, 1);
    let fiche = ent
        .fiche_agent_for_actor(etab, dir, "direction", garde)
        .await
        .unwrap();
    assert_eq!(fiche.engagements.len(), 1);
    assert_eq!(fiche.engagements[0].nature, "vacataire");
    assert_eq!(fiche.qualifications.len(), 1);
    assert_eq!(fiche.identifiants.len(), 1);
    assert!(
        fiche.identite_restreinte,
        "la protection de la source l'emporte"
    );
    // Le doublon sort de l'annuaire ; le compte et l'agent conservés restent.
    let liste = ent
        .annuaire_for_actor(
            etab,
            dir,
            "direction",
            &FiltreAnnuaire {
                limite: Some(100),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(liste.iter().all(|a| a.agent_id != doublon.agent_id));
    assert!(liste.iter().any(|a| a.agent_id == garde));
    // Déjà fusionné : on ne refusionne pas ; la défusion des personnes est liée à la fusion d'agents.
    assert!(est_conflit(
        &ent.fusionner_agents_for_actor(etab, dir, "direction", &corps)
            .await
    ));
    let (fusion_id,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_personne_fusion WHERE etablissement_id = $1 AND source_id = $2",
    )
    .bind(etab)
    .bind(p_src)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(matches!(
        ent.defusionner_personnes_for_actor(etab, admin, "admin", fusion_id)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Cas simple : un seul agent, côté source → il suit la personne conservée, et revient à la défusion.
    let seule = ent
        .creer_personne_for_actor(
            etab,
            dir,
            "direction",
            &PersonneSaisie {
                nom: "Conservee".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let agent_seul = creer(&ent, etab, dir, &nouvelle("Source-Agent", "Sam", None)).await;
    let f = ent
        .fusionner_personnes_for_actor(
            etab,
            dir,
            "direction",
            agent_seul.personne_id,
            seule.id,
            "même personne",
        )
        .await
        .expect("fusion de personnes");
    let (p_agent,): (Uuid,) = sqlx::query_as("SELECT personne_id FROM ent_agent WHERE id = $1")
        .bind(agent_seul.agent_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(p_agent, seule.id, "l'agent suit la personne conservée");
    ent.defusionner_personnes_for_actor(etab, dir, "direction", f.id)
        .await
        .expect("défusion");
    let (p_agent,): (Uuid,) = sqlx::query_as("SELECT personne_id FROM ent_agent WHERE id = $1")
        .bind(agent_seul.agent_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(
        p_agent, agent_seul.personne_id,
        "défusion : l'agent revient à sa personne"
    );
}

#[tokio::test]
async fn reprise_de_l_existant_comptes_ferme_professeur_parent_et_rejeu() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    // Comptes « d'avant » : le déclencheur est suspendu le temps de les créer, comme avant la migration.
    sqlx::query("ALTER TABLE ent_membership DISABLE TRIGGER ent_membership_agent_sync_trg")
        .execute(ent.pool())
        .await
        .unwrap();
    let prof = compte(&ent, etab, "prof").await;
    let parti = compte(&ent, etab, "psy").await;
    let futur = compte(&ent, etab, "cpe").await;
    let parent = compte(&ent, etab, "parent").await;
    sqlx::query("UPDATE ent_membership SET created_at = '2024-09-02 08:00+00' WHERE etablissement_id = $1 AND user_id IN ($2, $3, $4)").bind(etab).bind(prof).bind(parti).bind(futur).execute(ent.pool()).await.unwrap();
    // Un accès fermé le 30 juin 2025, un accès avec date de fin AVANT sa création (donnée aberrante).
    sqlx::query("UPDATE ent_membership SET etat = 'ferme', etat_depuis = '2025-06-30 18:00+00', etat_motif = 'depart' WHERE etablissement_id = $1 AND user_id = $2").bind(etab).bind(parti).execute(ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_membership SET date_fin = '2024-01-01' WHERE etablissement_id = $1 AND user_id = $2").bind(etab).bind(futur).execute(ent.pool()).await.unwrap();
    // Le professeur est aussi parent : sa personne existe déjà (lien de responsabilité).
    let pid = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, user_id, source) VALUES ($1,$2,'Parent','Prof',$3,'saisie')").bind(pid).bind(etab).bind(prof).execute(ent.pool()).await.unwrap();
    sqlx::query("ALTER TABLE ent_membership ENABLE TRIGGER ent_membership_agent_sync_trg")
        .execute(ent.pool())
        .await
        .unwrap();
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
            etab,
            prof
        )
        .await,
        1
    );

    // Rejeu de la migration (idempotente) : la reprise crée ce qui manque, une seule fois.
    crate::migrate::rejouer_tout(ent.pool()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    crate::migrate::rejouer_tout(ent.pool()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    for u in [prof, parti, futur] {
        assert_eq!(
            compter(
                &ent,
                "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
                etab,
                u
            )
            .await,
            1
        );
    }
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
            etab,
            parent
        )
        .await,
        0,
        "un parent sans lien n'a pas de personne"
    );
    let a_prof = agent_de(&ent, etab, prof).await;
    let (personne_agent, a_separer, source): (Uuid, bool, String) = sqlx::query_as(
        "SELECT personne_id, identite_a_separer, source FROM ent_agent WHERE id = $1",
    )
    .bind(a_prof)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        personne_agent, pid,
        "professeur-parent : la personne existante est réutilisée"
    );
    assert!(!a_separer);
    assert_eq!(source, "reprise");
    let (nom, prenom): (String, Option<String>) =
        sqlx::query_as("SELECT nom, prenom FROM ent_personne WHERE id = $1")
            .bind(pid)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        (nom.as_str(), prenom.as_deref()),
        ("Parent", Some("Prof")),
        "la personne saisie n'est pas écrasée"
    );
    // Engagement à préciser, du jour de création de l'accès (date de l'établissement).
    let (nature, du, au): (String, NaiveDate, Option<NaiveDate>) =
        sqlx::query_as("SELECT nature, du, au FROM ent_agent_engagement WHERE agent_id = $1")
            .bind(a_prof)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        (nature.as_str(), du, au),
        ("a_preciser", d(2024, 9, 2), None)
    );
    // Accès fermé : l'engagement s'arrête à la fermeture ; l'agent est « parti ».
    let a_parti = agent_de(&ent, etab, parti).await;
    let (au,): (Option<NaiveDate>,) =
        sqlx::query_as("SELECT au FROM ent_agent_engagement WHERE agent_id = $1")
            .bind(a_parti)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(au, Some(d(2025, 6, 30)));
    let (etat,): (String,) = sqlx::query_as("SELECT ent_agent_etat($1, $2, '2026-01-15')")
        .bind(etab)
        .bind(a_parti)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(etat, "parti");
    // Date de fin antérieure au début : bornée, la migration ne casse pas.
    let a_futur = agent_de(&ent, etab, futur).await;
    let (du, au): (NaiveDate, Option<NaiveDate>) =
        sqlx::query_as("SELECT du, au FROM ent_agent_engagement WHERE agent_id = $1")
            .bind(a_futur)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(au, Some(du));
    // Fonction principale pour tous : « aucun agent sans fonction principale ».
    let (sans,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_agent a WHERE a.etablissement_id = $1
            AND NOT EXISTS (SELECT 1 FROM ent_agent_fonction f WHERE f.agent_id = a.id AND f.principale AND f.annule_le IS NULL)",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(sans, 0);
    // Rien ne s'est dupliqué au rejeu.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_agent_engagement WHERE etablissement_id = $1 AND $2 = $2",
            etab,
            etab
        )
        .await,
        3
    );
}
