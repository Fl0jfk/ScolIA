//! Socle 20, lot C4 : la charge de travail, contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`). Aucun résultat ne dépend de l'heure réelle : le « jour » est celui de
//! l'établissement, les échéances sont des jours ouvrés calculés par la base, les évaluations annoncées sont des devoirs
//! datés. Chaque test ne vérifie que ses propres lignes.

use super::*;
use crate::cahier_c4::*;
use chrono::DateTime;
use domain_eleve::CreateEleve;
use ent_types::cahier_c2::*;
use ent_types::cahier_c4::*;
use std::time::Instant;

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

/// Premier jour ouvré à partir de `jour`.
async fn ouvre(fx: &Fx, jour: NaiveDate) -> NaiveDate {
    sqlx::query_scalar(
        "SELECT d::date FROM generate_series($2::date, $2::date + 60, interval '1 day') AS d
          WHERE NOT ent_cdt_jour_ferme($1, d::date) ORDER BY 1 LIMIT 1",
    )
    .bind(fx.cast.etablissement_id)
    .bind(jour)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap()
}

/// Jour ouvré strictement après `jour`.
async fn ouvre_apres(fx: &Fx, jour: NaiveDate) -> NaiveDate {
    ouvre(fx, jour + Duration::days(1)).await
}

fn corps_travail(cours: Vec<Uuid>, echeance: NaiveDate, nature: &str, duree: Option<i16>) -> TravailC2Body {
    TravailC2Body {
        cours_ids: cours,
        seance_donnee_id: None,
        echeance_seance_id: None,
        echeance_prochaine_seance: false,
        echeance_jour: Some(echeance),
        echeance_heure: NaiveTime::from_hms_opt(8, 0, 0),
        nature: nature.into(),
        titre: format!("Travail {}", &Uuid::new_v4().to_string()[..12]),
        consigne: "à faire".into(),
        duree_estimee_min: duree,
        rendu: "aucun".into(),
        retard_accepte: true,
        formats: vec![],
        eleve_ids: vec![],
        ia_autorisee: "non".into(),
        publication: "maintenant".into(),
        publie_a: None,
        publie_jour: None,
        publie_heure: None,
    }
}

async fn rendre_visible(fx: &Fx) {
    sqlx::query(
        "UPDATE ent_cdt_travail SET visible_a = now() - interval '1 minute'
          WHERE etablissement_id = $1 AND ancien_id IS NULL AND etat IN ('programme', 'publie') AND visible_a > now()",
    )
    .bind(fx.cast.etablissement_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
}

async fn cours_de_service(fx: &Fx, service: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
        .bind(service)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Devoir de l'existant (socle 21 minimal) pour une classe, daté `jour`.
async fn devoir(fx: &Fx, periode: Uuid, classe: Uuid, code_classe: &str, jour: NaiveDate, matiere: &str, titre: &str) {
    let e = fx.cast.etablissement_id;
    let mat = Uuid::new_v4();
    let code = Uuid::new_v4().simple().to_string();
    sqlx::query("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1,$2,$3,$4)")
        .bind(mat)
        .bind(e)
        .bind(&code[..12])
        .bind(matiere)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, classe_id, titre, date_devoir)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(periode)
    .bind(mat)
    .bind(code_classe)
    .bind(classe)
    .bind(titre)
    .bind(jour)
    .execute(fx.ent.pool())
    .await
    .unwrap();
}

async fn periode(fx: &Fx, auj: NaiveDate) -> Uuid {
    let id = Uuid::new_v4();
    let code = Uuid::new_v4().simple().to_string();
    sqlx::query("INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin) VALUES ($1,$2,$3,'Période C4',$4,$5)")
        .bind(id)
        .bind(fx.cast.etablissement_id)
        .bind(&code[..12])
        .bind(auj - Duration::days(90))
        .bind(auj + Duration::days(90))
        .execute(fx.ent.pool())
        .await
        .unwrap();
    id
}

fn tout_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

// ───────────────────────── Groupe transversal de 4 classes, 35 élèves ─────────────────────────

#[tokio::test]
async fn c4_groupe_de_4_classes_nombres_justes_aucune_matiere_hors_classe_alerte_dates_et_performance() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let dir = Role::Direction.as_str();
    let auj = jour(&fx).await;
    let annee = fx.ent.referentiel_annee_courante_assuree(e).await.unwrap();
    let prof0 = fx.cast.profs[0].user_id;

    // Quatre classes de 9, 9, 9 et 8 élèves (35), toutes dans un même groupe transversal (sans classe de rattachement).
    let mut classes = Vec::new();
    for k in 0..4 {
        let code = format!("K{k}-{}", &Uuid::new_v4().simple().to_string()[..8]);
        let c = fx.ent.referentiel_creer_classe(dir, e, Some(annee.id), &code, None, None).await.unwrap();
        classes.push((c.id, code));
    }
    let groupe = fx.ent.referentiel_creer_groupe(dir, e, Some(annee.id), &format!("G4-{}", &Uuid::new_v4().simple().to_string()[..8]), None, None).await.unwrap();
    let tailles = [9usize, 9, 9, 8];
    let mut par_classe: Vec<Vec<Uuid>> = vec![Vec::new(); 4];
    for (k, n) in tailles.iter().enumerate() {
        for i in 0..*n {
            let eleve = fx
                .ent
                .create_eleve(CreateEleve {
                    etablissement_id: e,
                    nom: format!("Charge{k}"),
                    prenom: format!("Eleve{i}"),
                    ine: None,
                    sexe: None,
                    date_naissance: NaiveDate::from_ymd_opt(2012, 5, 5),
                    numero_badge: None,
                    redoublant: false,
                    classe: None,
                    regime: None,
                    mef: None,
                    site: None,
                    date_entree: Some(auj - Duration::days(120)),
                    date_sortie: None,
                    motif_sortie: None,
                    actor_user_id: Some(fx.cast.admin.user_id.to_string()),
                })
                .await
                .unwrap();
            fx.ent.referentiel_affecter_eleve_classe(dir, e, classes[k].0, eleve.id, None).await.unwrap();
            fx.ent.referentiel_affecter_eleve_groupe(dir, e, groupe.id, eleve.id, None).await.unwrap();
            par_classe[k].push(eleve.id);
        }
    }
    let service_g = fx
        .ent
        .referentiel_creer_service(
            dir,
            e,
            crate::referentiel_repo::CreerService {
                prof_user_id: prof0,
                matiere_id: None,
                classe_id: None,
                groupe_id: Some(groupe.id),
                professeur_principal: false,
            },
        )
        .await
        .unwrap();
    let cours_g = cours_de_service(&fx, service_g.id).await;
    let mut cours_classes = Vec::new();
    for (id, _) in &classes {
        let s = fx
            .ent
            .referentiel_creer_service(
                dir,
                e,
                crate::referentiel_repo::CreerService {
                    prof_user_id: prof0,
                    matiere_id: None,
                    classe_id: Some(*id),
                    groupe_id: None,
                    professeur_principal: false,
                },
            )
            .await
            .unwrap();
        cours_classes.push(cours_de_service(&fx, s.id).await);
    }

    // Évaluations annoncées le jour J : 1 pour la classe 0, 2 pour la classe 1, 0 pour la classe 2, 3 pour la classe 3.
    let j = ouvre(&fx, auj + Duration::days(5)).await;
    let p = periode(&fx, auj).await;
    let secrets = ["MatiereSecreteHistoire", "MatiereSecreteMaths", "MatiereSecretePhysique", "MatiereSecreteAnglais", "MatiereSecreteFrancais", "MatiereSecreteSVT"];
    for (k, m, t) in [(0, 0, "TitreSecretA"), (1, 1, "TitreSecretB"), (1, 2, "TitreSecretC"), (3, 3, "TitreSecretD"), (3, 4, "TitreSecretE"), (3, 5, "TitreSecretF")] {
        devoir(&fx, p, classes[k].0, &classes[k].1, j, secrets[m], t).await;
    }

    // Aperçu du groupe transversal : 35 élèves réels, nombres justes, jamais une matière ni un titre.
    let body = ApercuBody {
        cours_ids: vec![cours_g],
        echeance_jour: Some(j),
        prochaine_seance: false,
        nature: Some("evaluation".into()),
        duree_estimee_min: None,
    };
    let debut = Instant::now();
    let a = fx.ent.cdt_charge_apercu(&fx.prof(), body.clone()).await.unwrap();
    let duree_ms = debut.elapsed().as_millis();
    assert_eq!(a.nb_eleves, 35, "les élèves réels des quatre classes, pas la charge d'une classe");
    assert_eq!(
        a.evaluations_ce_jour.iter().map(|d| (d.valeur, d.nb_eleves)).collect::<Vec<_>>(),
        vec![(0, 9), (1, 9), (2, 9), (3, 8)],
        "distribution exacte des évaluations déjà annoncées ce jour-là"
    );
    let eval = a.alertes.iter().find(|x| x.repere == "evaluations").expect("alerte sur les évaluations du jour");
    assert_eq!((eval.nb_eleves, eval.valeur_max, eval.seuil), (9 + 8, 4, 2), "classes 1 et 3 passent au-dessus de 2");
    assert!(a.alerte && a.alertes.iter().all(|x| x.repere != "minutes"));
    // Autres échéances : natures et nombres seulement, la matière jamais (groupe transversal, §11.1).
    let ev = a.autres_echeances.iter().find(|x| x.genre == "evaluation").unwrap();
    assert_eq!((ev.nb_eleves, ev.matiere.as_deref()), (9 + 9 + 8, None), "26 élèves ont déjà une évaluation ce jour-là");
    assert!(a.autres_echeances.iter().all(|x| x.matiere.is_none()));
    let json = tout_json(&a);
    for secret in secrets.iter().chain(["TitreSecret", "Charge0", "Eleve0", "Cours à préciser"].iter()) {
        assert!(!json.contains(secret), "{secret} ne sort jamais de l'aperçu d'un groupe transversal : {json}");
    }
    // Deux dates alternatives calculées : des jours ouvrés distincts, plus légers pour ces élèves.
    assert_eq!(a.dates_proposees.len(), 2, "{:?}", a.dates_proposees);
    for d in &a.dates_proposees {
        assert!(d.jour > auj && d.jour != j && ouvre(&fx, d.jour).await == d.jour, "jour ouvré après aujourd'hui");
        assert!(d.nb_eleves_au_dessus < eval.nb_eleves, "la date proposée est plus légère");
    }
    // Performance : moins de 200 ms pour 35 élèves (meilleur de trois passages, base locale).
    let mut meilleur = duree_ms;
    for _ in 0..2 {
        let t = Instant::now();
        fx.ent.cdt_charge_apercu(&fx.prof(), body.clone()).await.unwrap();
        meilleur = meilleur.min(t.elapsed().as_millis());
    }
    assert!(meilleur < 200, "aperçu de 35 élèves en {meilleur} ms (critère : moins de 200 ms)");

    // Aperçu d'un cours de CLASSE (classe 1) : la matière des échéances de sa classe est montrée, jamais celle d'un autre
    // cours (le travail du groupe transversal n'a pas de classe : nature seulement).
    let t_groupe = corps_travail(vec![cours_g], j, "exercices", Some(20));
    fx.ent.cdt_travaux_creer(&fx.prof(), t_groupe).await.unwrap();
    rendre_visible(&fx).await;
    let a1 = fx
        .ent
        .cdt_charge_apercu(&fx.prof(), ApercuBody { cours_ids: vec![cours_classes[1]], echeance_jour: Some(j), prochaine_seance: false, nature: Some("exercices".into()), duree_estimee_min: Some(10) })
        .await
        .unwrap();
    assert_eq!(a1.nb_eleves, 9);
    let matieres: Vec<Option<&str>> = a1.autres_echeances.iter().filter(|x| x.genre == "evaluation").map(|x| x.matiere.as_deref()).collect();
    assert!(matieres.contains(&Some("MatiereSecreteMaths")) && matieres.contains(&Some("MatiereSecretePhysique")), "la classe voit ses matières : {matieres:?}");
    let travail_groupe = a1.autres_echeances.iter().find(|x| x.genre == "travail").expect("le travail du groupe est compté");
    assert_eq!((travail_groupe.nb_eleves, travail_groupe.matiere.as_deref()), (9, None), "un cours hors classe : nature seulement");
    assert!(!tout_json(&a1).contains("TitreSecret"));
    assert!(!tout_json(&a1).contains("MatiereSecreteAnglais"), "aucune matière d'une autre classe");
}

// ───────────────────────── Charge d'un élève, repères, calendrier de classe ─────────────────────────

#[tokio::test]
async fn c4_charge_de_l_eleve_famille_reperes_a_trois_positions_et_calendrier_de_classe() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let auj = jour(&fx).await;
    let cours = cours_de_service(&fx, fx.service_a).await;
    let j = ouvre(&fx, auj + Duration::days(4)).await;
    let veille = sqlx::query_scalar::<_, NaiveDate>(
        "SELECT d::date FROM generate_series($2::date - 10, $2::date - 1, interval '1 day') AS d
          WHERE NOT ent_cdt_jour_ferme($1, d::date) ORDER BY 1 DESC LIMIT 1",
    )
    .bind(fx.cast.etablissement_id)
    .bind(j)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    let p = periode(&fx, auj).await;
    // Deux évaluations annoncées pour la classe A le jour J ; un travail de 50 minutes (la veille ouvrée porte 50 min).
    devoir(&fx, p, fx.classe_a, "CDT-A", j, "Chimie", "Contrôle de chimie").await;
    devoir(&fx, p, fx.classe_a, "CDT-A", j, "Latin", "Version latine").await;
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(vec![cours], j, "exercices", Some(50))).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;
    let (du, au) = (j - Duration::days(1), j + Duration::days(1));

    let c = fx.ent.cdt_charge_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    let cj = c.jours.iter().find(|x| x.jour == j).expect("le jour J figure");
    assert_eq!((cj.nb_evaluations, cj.nb_travaux), (2, 1));
    assert_eq!(c.position, "normal");
    assert_eq!((c.repere.groupe.as_str(), c.repere.evaluations_jour, c.repere.minutes_soir), ("4e-3e", 2, 60), "niveau inconnu : colonne intermédiaire");
    assert_eq!(cj.minutes_soir_precedent, 50, "la veille ouvrée de {j} est {veille}");
    assert!(cj.au_dessus.is_empty(), "2 évaluations et 50 min : au repère, pas au-dessus");
    // La famille lit la même charge ; le parent d'un autre enfant, le personnel : refusés.
    let cf = fx.ent.cdt_charge_eleve(&fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap();
    assert_eq!(cf.jours, c.jours);
    assert!(est_interdit(&fx.ent.cdt_charge_eleve(&fx.parent(&fx.y), Some(fx.x.id), du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_charge_eleve(&fx.prof(), Some(fx.x.id), du, au).await.unwrap_err()));
    // L'élève de l'autre classe n'a rien ce jour-là.
    let cy = fx.ent.cdt_charge_eleve(&fx.eleve(&fx.y), None, du, au).await.unwrap();
    assert!(cy.jours.iter().all(|x| x.jour != j));
    let json = tout_json(&cf);
    assert!(!json.contains("Chimie") && !json.contains("Latin") && !json.contains("Contrôle"), "nombres seulement : {json}");

    // Repères à trois positions : la direction règle, le professeur non ; « strict » : 1 évaluation par jour.
    assert_eq!(fx.ent.cdt_reperes(&fx.eleve(&fx.x)).await.unwrap().position, "normal");
    assert!(est_interdit(&fx.ent.cdt_reperes_regler(&fx.prof(), "strict").await.unwrap_err()));
    assert!(est_refus(&fx.ent.cdt_reperes_regler(&fx.direction(), "extreme").await.unwrap_err(), RefusMetier::Invalide));
    let strict = fx.ent.cdt_reperes_regler(&fx.direction(), "strict").await.unwrap();
    assert_eq!(strict.position, "strict");
    assert_eq!(strict.groupes.iter().map(|g| g.evaluations_jour).collect::<Vec<_>>(), vec![1, 1, 1]);
    let c = fx.ent.cdt_charge_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    let cj = c.jours.iter().find(|x| x.jour == j).unwrap();
    assert_eq!(c.position, "strict");
    assert!(cj.au_dessus.contains(&"evaluations".to_string()), "2 évaluations dépassent le repère strict : {cj:?}");
    assert!(cj.au_dessus.contains(&"minutes".to_string()), "50 min dépassent 45 min (strict, 4e-3e) : {cj:?}");
    let souple = fx.ent.cdt_reperes_regler(&fx.direction(), "souple").await.unwrap();
    assert_eq!(souple.groupes.iter().map(|g| g.minutes_soir).collect::<Vec<_>>(), vec![60, 90, 150]);
    fx.ent.cdt_reperes_regler(&fx.direction(), "normal").await.unwrap();

    // Calendrier de la classe A : lignes avec matières pour l'équipe ; élèves au-dessus des repères agrégés, sans nom.
    let cal = fx.ent.cdt_calendrier_classe(&fx.prof(), fx.classe_a, du, au).await.unwrap();
    let jc = cal.jours.iter().find(|x| x.jour == j).expect("jour J dans le calendrier");
    let mats: Vec<&str> = jc.lignes.iter().map(|l| l.matiere.as_str()).collect();
    assert!(mats.contains(&"Chimie") && mats.contains(&"Latin"), "l'équipe de la classe voit les matières : {mats:?}");
    assert_eq!(jc.evaluations_max_par_eleve, 2);
    assert!(cal.nb_eleves >= 3);
    // Un professeur qui n'est pas de la classe, l'élève, la famille : refusés ; la direction lit.
    assert!(est_interdit(&fx.ent.cdt_calendrier_classe(&fx.prof2(), fx.classe_a, du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_calendrier_classe(&fx.eleve(&fx.x), fx.classe_a, du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_calendrier_classe(&fx.parent(&fx.x), fx.classe_a, du, au).await.unwrap_err()));
    assert!(fx.ent.cdt_calendrier_classe(&fx.direction(), fx.classe_a, du, au).await.is_ok());
    assert!(est_interdit(&fx.ent.cdt_calendrier_classe(&fx.direction(), Uuid::new_v4(), du, au).await.unwrap_err()), "classe inconnue : refusée");
    let _ = t;
}

// ───────────────────────── Alertes ignorées comptées ─────────────────────────

#[tokio::test]
async fn c4_alerte_ignoree_recalculee_par_le_serveur_comptee_et_jamais_nominative() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let cours = cours_de_service(&fx, fx.service_a).await;
    let j = ouvre(&fx, auj + Duration::days(2)).await;
    // Un premier travail de 40 minutes ; le second (30 min) fait passer le soir de 40 à 70 min, au-dessus de 60 (normal).
    let t0 = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(vec![cours], j, "exercices", Some(40))).await.unwrap().travaux[0].id;
    let c1 = corps_travail(vec![cours], j, "exercices", Some(30));
    let titre1 = c1.titre.clone();
    let t1 = fx.ent.cdt_travaux_creer(&fx.prof(), c1).await.unwrap().travaux[0].id;
    // Un travail sans alerte : un autre jour, 10 minutes.
    let autre_jour = ouvre_apres(&fx, ouvre_apres(&fx, j).await).await;
    let t2 = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(vec![cours], autre_jour, "exercices", Some(10))).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;

    // Le bandeau avertit avant : un repère de minutes dépassé par ce travail.
    let apercu = fx
        .ent
        .cdt_charge_apercu(&fx.prof(), ApercuBody { cours_ids: vec![cours], echeance_jour: Some(j), prochaine_seance: false, nature: Some("exercices".into()), duree_estimee_min: Some(30) })
        .await
        .unwrap();
    assert!(apercu.alertes.iter().any(|a| a.repere == "minutes" && a.seuil == 60 && a.valeur_max == 100), "{:?}", apercu.alertes);

    // Passer outre : le serveur recalcule (il ne croit pas le client) et compte, une fois par repère.
    let al = fx.ent.cdt_alerte_ignorer(&fx.prof(), t1).await.unwrap();
    assert!(al.iter().any(|a| a.repere == "minutes"), "{al:?}");
    fx.ent.cdt_alerte_ignorer(&fx.prof(), t1).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_alerte_charge WHERE etablissement_id = $1 AND travail_id = $2 AND ignoree")
        .bind(e)
        .bind(t1)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(n as usize, al.len(), "rejouable sans doublon");
    // Sans alerte réelle : rien n'est compté. Un autre professeur, un élève, la direction : refusés.
    assert!(est_refus(&fx.ent.cdt_alerte_ignorer(&fx.prof(), t2).await.unwrap_err(), RefusMetier::Conflit));
    assert!(est_interdit(&fx.ent.cdt_alerte_ignorer(&fx.prof2(), t1).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_alerte_ignorer(&fx.direction(), t1).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_alerte_ignorer(&fx.eleve(&fx.x), t1).await.unwrap_err()));
    let _ = t0;

    // Comptage : agrégé par classe pour la direction, sans titre ni auteur ; le professeur ne retrouve que les siennes.
    let (du, au) = (auj - Duration::days(1), auj + Duration::days(1));
    let dir = fx.ent.cdt_alertes_ignorees(&fx.direction(), du, au).await.unwrap();
    assert_eq!(dir.total, 1);
    assert_eq!(dir.par_classe.iter().map(|c| (c.classe.as_str(), c.nb)).collect::<Vec<_>>(), vec![("CDT-A", 1)]);
    assert!(dir.mes_alertes.is_empty(), "la direction n'est pas l'auteur : aucune alerte nominative");
    let json = tout_json(&dir);
    assert!(!json.contains(&titre1) && !json.contains(&fx.cast.profs[0].user_id.to_string()), "agrégé, jamais nominatif : {json}");
    let moi = fx.ent.cdt_alertes_ignorees(&fx.prof(), du, au).await.unwrap();
    assert_eq!(moi.total, 1);
    assert!(moi.par_classe.is_empty());
    let m = moi.mes_alertes.iter().find(|m| m.travail_id == t1 && m.repere == "minutes").expect("ses propres alertes");
    assert_eq!((m.titre.as_str(), m.seuil), (titre1.as_str(), 60));
    assert!(fx.ent.cdt_alertes_ignorees(&fx.prof2(), du, au).await.is_err() || fx.ent.cdt_alertes_ignorees(&fx.prof2(), du, au).await.unwrap().mes_alertes.is_empty());
    assert!(est_interdit(&fx.ent.cdt_alertes_ignorees(&fx.eleve(&fx.x), du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_alertes_ignorees(&fx.parent(&fx.x), du, au).await.unwrap_err()));
    // Droits de l'aperçu : seul qui saisit dans le cours ; un cours d'un autre établissement n'existe pas.
    let b = ApercuBody { cours_ids: vec![cours], echeance_jour: Some(j), prochaine_seance: false, nature: None, duree_estimee_min: None };
    assert!(est_interdit(&fx.ent.cdt_charge_apercu(&fx.prof2(), b.clone()).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_charge_apercu(&fx.eleve(&fx.x), b.clone()).await.unwrap_err()));
    assert!(est_introuvable(&fx.ent.cdt_charge_apercu(&fx.prof(), ApercuBody { cours_ids: vec![Uuid::new_v4()], ..b.clone() }).await.unwrap_err()));
    assert!(est_refus(&fx.ent.cdt_charge_apercu(&fx.prof(), ApercuBody { cours_ids: vec![], ..b.clone() }).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_refus(&fx.ent.cdt_charge_apercu(&fx.prof(), ApercuBody { echeance_jour: Some(auj), ..b }).await.unwrap_err(), RefusMetier::Invalide));
    let _: Option<DateTime<Utc>> = None;
}
