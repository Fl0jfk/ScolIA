//! Socle 20, lot C2 : saisie et publication, contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`, séances, élèves). Aucun test n'utilise l'heure réelle pour
//! décider d'un résultat : la plage calme se teste avec des **instants explicites** (`publie_a` ou appel direct de
//! `ent_cdt_visible_a`) ; la visibilité des élèves se force par SQL (publication échue) quand l'heure réelle
//! pourrait tomber dans la plage calme. Chaque test ne vérifie que ses propres lignes.

use super::*;
use crate::cahier_c2::contient_html;
use chrono::{DateTime, TimeZone};
use ent_types::cahier_c2::*;

fn instant(a: i32, m: u32, j: u32, h: u32, mi: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(a, m, j, h, mi, 0).unwrap()
}

fn entree(contenu: &str, publication: &str) -> EntreeBody {
    EntreeBody {
        titre: None,
        contenu: contenu.into(),
        notions: vec![],
        publication: publication.into(),
        publie_a: None,
        publie_jour: None,
        publie_heure: None,
        sans_avis: false,
        motif: None,
    }
}

fn travail(cours: Vec<Uuid>) -> TravailC2Body {
    TravailC2Body {
        cours_ids: cours,
        seance_donnee_id: None,
        echeance_seance_id: None,
        echeance_prochaine_seance: false,
        echeance_jour: None,
        echeance_heure: None,
        nature: "exercices".into(),
        titre: format!("Exercices {}", &Uuid::new_v4().to_string()[..12]),
        consigne: "p. 12 n° 3 à 5".into(),
        duree_estimee_min: None,
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

async fn cours_de(fx: &Fx, service: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
        .bind(service)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Rend échues les publications différées de l'établissement (l'heure réelle du test est quelconque).
async fn rendre_visible(fx: &Fx) {
    let e = fx.cast.etablissement_id;
    for sql in [
        "UPDATE ent_cdt_entree SET visible_a = now() - interval '1 minute'
          WHERE etablissement_id = $1 AND ancien_id IS NULL AND etat IN ('programmee', 'publiee') AND visible_a > now()",
        "UPDATE ent_cdt_travail SET visible_a = now() - interval '1 minute'
          WHERE etablissement_id = $1 AND ancien_id IS NULL AND etat IN ('programme', 'publie') AND visible_a > now()",
    ] {
        sqlx::query(sql).bind(e).execute(fx.ent.pool()).await.unwrap();
    }
}

async fn visible_a_de(fx: &Fx, publie: DateTime<Utc>) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT ent_cdt_visible_a($1, $2)")
        .bind(fx.cast.etablissement_id)
        .bind(publie)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

async fn nb_evt(fx: &Fx, type_name: &str, agregat: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2 AND aggregate_id = $3")
        .bind(fx.cast.etablissement_id)
        .bind(type_name)
        .bind(agregat)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Rend visible un travail précis (publication échue), puis le projette dans l'ancien modèle.
async fn travail_visible(fx: &Fx, id: Uuid) {
    sqlx::query("UPDATE ent_cdt_travail SET visible_a = now() - interval '1 minute' WHERE etablissement_id = $1 AND id = $2 AND visible_a > now()")
        .bind(fx.cast.etablissement_id)
        .bind(id)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
}

// ───────────────────────── Plage calme (R-C5) : instants explicites ─────────────────────────

#[tokio::test]
async fn c2_plage_calme_22h_7h_et_fin_de_semaine_instants_explicites() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    // Hiver (UTC+1) : mardi 11 mars 2031.
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 11, 21, 0)).await, instant(2031, 3, 12, 6, 0), "mardi 22 h → mercredi 7 h");
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 11, 4, 0)).await, instant(2031, 3, 11, 6, 0), "mardi 5 h → mardi 7 h");
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 11, 6, 0)).await, instant(2031, 3, 11, 6, 0), "mardi 7 h pile : visible");
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 11, 5, 59)).await, instant(2031, 3, 11, 6, 0), "6 h 59 : encore calme");
    // Vendredi 19 h 59 : visible tout de suite ; 20 h : lundi 7 h (test d'or du §32.1).
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 14, 18, 59)).await, instant(2031, 3, 14, 18, 59));
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 14, 19, 0)).await, instant(2031, 3, 17, 6, 0), "vendredi 20 h → lundi 7 h");
    // Dimanche 21 h → lundi 7 h ; samedi midi → lundi 7 h.
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 16, 20, 0)).await, instant(2031, 3, 17, 6, 0));
    assert_eq!(visible_a_de(&fx, instant(2031, 3, 15, 11, 0)).await, instant(2031, 3, 17, 6, 0));
    // Été (UTC+2) : dimanche 13 juillet 22 h Paris = 20 h UTC → lundi 7 h Paris = 5 h UTC.
    assert_eq!(visible_a_de(&fx, instant(2031, 7, 13, 20, 0)).await, instant(2031, 7, 14, 5, 0));
    // Vacances (lundi 14 au dimanche 27 avril) : une publication en plein milieu attend la rentrée, 7 h.
    let dir = fx.cast.direction.user_id;
    let e = fx.cast.etablissement_id;
    sqlx::query("INSERT INTO ent_periode_vacances (id, etablissement_id, libelle, kind, debut, fin, created_by_user_id) VALUES ($1,$2,'Printemps','vacances','2031-04-14','2031-04-27',$3)")
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(dir)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(visible_a_de(&fx, instant(2031, 4, 16, 8, 0)).await, instant(2031, 4, 28, 5, 0), "mercredi en vacances → lundi de rentrée 7 h");
    assert_eq!(visible_a_de(&fx, instant(2031, 4, 11, 18, 30)).await, instant(2031, 4, 28, 5, 0), "vendredi 20 h 30 avant les vacances → rentrée");
    // Jour férié (lundi 5 mai) : la publication du vendredi soir attend mardi 7 h.
    sqlx::query("INSERT INTO ent_jour_calendrier (id, etablissement_id, jour, kind, libelle) VALUES ($1,$2,'2031-05-05','ferie','Pont')")
        .bind(Uuid::new_v4())
        .bind(e)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(visible_a_de(&fx, instant(2031, 5, 2, 19, 0)).await, instant(2031, 5, 6, 5, 0), "vendredi 21 h → mardi 7 h (lundi férié)");
}

#[tokio::test]
async fn c2_entree_programmee_en_plage_calme_visible_a_la_fin_de_la_plage() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj + Duration::days(1)).await;
    // Programmée un mardi à 22 h (heure de Paris) : le serveur annonce « visible mercredi à 7 h ».
    let mut b = entree("Chapitre sur les fractions", "programmer");
    b.publie_a = Some(instant(2031, 3, 11, 21, 0));
    let e = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, b).await.unwrap();
    assert_eq!(e.etat, "programmee");
    assert_eq!(e.visible_a, Some(instant(2031, 3, 12, 6, 0)));
    assert_eq!(e.visible_jour.map(|d| d.to_string()), Some("2031-03-12".into()));
    assert_eq!(e.visible_heure.as_deref(), Some("07:00"));
    assert!(e.differee_par_plage_calme, "l'auteur est prévenu : publication différée par la plage calme");
    // Programmée à 12 h : pas de report.
    let seance2 = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj + Duration::days(2)).await;
    let mut b = entree("Chapitre suivant", "programmer");
    b.publie_a = Some(instant(2031, 3, 11, 11, 0));
    let e2 = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance2, b).await.unwrap();
    assert_eq!(e2.visible_a, Some(instant(2031, 3, 11, 11, 0)));
    assert!(!e2.differee_par_plage_calme);
    // Rien n'est visible des élèves tant que la publication n'est pas échue.
    let (du, au) = (auj, auj + Duration::days(5));
    let avant = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(avant.iter().all(|s| s.seance_id != seance || s.contenu.is_none()), "programmée : rien n'est visible");
    // Publication échue : l'entrée est projetée pour les lecteurs et l'avis part (une seule fois).
    rendre_visible(&fx).await;
    let vue = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    let s = vue.iter().find(|s| s.seance_id == seance).expect("séance de la classe");
    assert!(s.contenu.as_ref().is_some_and(|c| c.contenu.contains("fractions")));
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
    assert_eq!(nb_evt(&fx, "cahier.seance_renseignee", e.id).await, 1, "un seul avis par publication");
}

// ───────────────────────── Parcours « saisir en 3 gestes » ─────────────────────────

#[tokio::test]
async fn c2_saisir_en_trois_gestes_cours_contenu_travail() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let prof0 = fx.cast.profs[0].user_id;
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let aujourdhui = fx.seance("CDT-A", prof0, Some(fx.service_a), auj).await;
    let prochaine = fx.seance("CDT-A", prof0, Some(fx.service_a), auj + Duration::days(7)).await;

    // Geste 1 : je choisis mon cours (liste des cours où j'écris aujourd'hui).
    let mes = fx.ent.cdt_cours_saisie(&fx.prof()).await.unwrap();
    assert!(mes.iter().any(|c| c.cours_id == cours && c.cible == "CDT-A"));
    let seances = fx.ent.cdt_seances_du_cours(&fx.prof(), cours, auj, auj + Duration::days(10)).await.unwrap();
    assert!(seances.iter().any(|s| s.seance_id == aujourdhui && s.entree.is_none()), "entrée vide tant que rien n'est écrit");

    // Geste 2 : je publie le contenu de la séance.
    let e = fx
        .ent
        .cdt_entree_enregistrer(&fx.prof(), cours, aujourdhui, entree("Fractions : définition et exemples", "maintenant"))
        .await
        .unwrap();
    assert_eq!((e.etat.as_str(), e.version), ("publiee", 1));
    assert_eq!(e.auteur.is_empty(), false);

    // Geste 3 : je donne le travail pour la prochaine séance ; nature et durée proposées, échéance-séance.
    let mut t = travail(vec![cours]);
    t.seance_donnee_id = Some(aujourdhui);
    t.echeance_prochaine_seance = true;
    let cree = fx.ent.cdt_travaux_creer(&fx.prof(), t).await.unwrap();
    let t1 = &cree.travaux[0];
    assert_eq!(t1.echeance_mode, "seance");
    assert_eq!(t1.echeance_seance_id, Some(prochaine));
    assert_eq!(t1.duree_estimee_min, Some(30), "durée proposée par la nature « exercices »");
    assert_eq!((t1.ia_autorisee.as_str(), t1.public.as_str()), ("non", "cours"));
    assert_eq!(t1.echeance_jour, auj + Duration::days(7));
    assert_eq!(t1.echeance_heure, "08:00");

    // L'élève de la classe lit le cahier et l'agenda (publication échue), jamais un brouillon.
    rendre_visible(&fx).await;
    let (du, au) = (auj - Duration::days(1), auj + Duration::days(30));
    let cahier = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(cahier.iter().any(|s| s.seance_id == aujourdhui && s.contenu.as_ref().is_some_and(|c| c.contenu.contains("Fractions"))));
    let agenda = agenda_de(&fx, &fx.x, du, au).await;
    assert!(agenda.iter().any(|i| i.travail.id == t1.id), "le travail apparaît dans l'agenda de l'élève");
    // L'élève de l'autre classe n'en voit rien.
    assert!(agenda_de(&fx, &fx.y, du, au).await.iter().all(|i| i.travail.id != t1.id));
    // La séance porte son entrée et son travail dans l'écran 1 du professeur.
    let seances = fx.ent.cdt_seances_du_cours(&fx.prof(), cours, auj, auj + Duration::days(10)).await.unwrap();
    let s0 = seances.iter().find(|s| s.seance_id == aujourdhui).unwrap();
    assert_eq!(s0.travaux_donnes.len(), 1);
    let s1 = seances.iter().find(|s| s.seance_id == prochaine).unwrap();
    assert_eq!(s1.travaux_attendus.len(), 1, "le travail est attendu à la séance suivante");
}

#[tokio::test]
async fn c2_brouillon_invisible_des_eleves_et_publication_refusee_a_un_non_intervenant() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj + Duration::days(3)).await;
    let b = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Brouillon de préparation", "brouillon")).await.unwrap();
    assert_eq!((b.etat.as_str(), b.visible_a, b.publie_a), ("brouillon", None, None));
    rendre_visible(&fx).await;
    let (du, au) = (auj, auj + Duration::days(10));
    let vue = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(vue.iter().all(|s| s.seance_id != seance || s.contenu.is_none()), "le brouillon n'est jamais lu par l'élève");
    // Ni le professeur d'un autre cours, ni la direction, ni la compta ne saisissent.
    for acteur in [fx.prof2(), fx.direction(), fx.compta(), fx.cpe()] {
        let r = fx.ent.cdt_entree_enregistrer(&acteur, cours, seance, entree("intrusion", "maintenant")).await;
        assert!(r.is_err_and(|e| est_interdit(&e)), "seul l'intervenant en saisie écrit");
    }
    // La direction ne lit pas le brouillon.
    assert!(est_introuvable_ou_interdit(fx.ent.cdt_entree_lire(&fx.direction(), b.id).await));
    // Un brouillon publié devient visible ; une entrée publiée ne redevient pas brouillon.
    let p = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Version finale", "maintenant")).await.unwrap();
    assert_eq!((p.id, p.etat.as_str()), (b.id, "publiee"));
    let r = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Retour brouillon", "brouillon")).await;
    assert!(r.is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
}

fn est_introuvable_ou_interdit<T>(r: Result<T, EntDbError>) -> bool {
    r.is_err_and(|e| est_interdit(&e) || est_introuvable(&e))
}

// ───────────────────────── Modification : versions, avis, échéance avancée ─────────────────────────

#[tokio::test]
async fn c2_modification_d_entree_versionnee_visible_des_lecteurs_avis_seulement_si_voulu() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj).await;
    let e1 = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Fractoins : définition", "maintenant")).await.unwrap();
    rendre_visible(&fx).await;
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
    // Faute de frappe : modification mineure, sans avis ; la version précédente reste lisible.
    let mut b = entree("Fractions : définition", "maintenant");
    b.sans_avis = true;
    b.motif = Some("faute de frappe".into());
    let e2 = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, b).await.unwrap();
    assert_eq!((e2.id, e2.version), (e1.id, 2));
    // Contenu utile : avec avis.
    let e3 = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Fractions : définition et propriétés", "maintenant")).await.unwrap();
    assert_eq!(e3.version, 3);
    let versions = fx.ent.cdt_entree_versions(&fx.prof(), e1.id).await.unwrap();
    assert_eq!(versions.iter().map(|v| v.version).collect::<Vec<_>>(), vec![2, 1]);
    assert!(versions[1].contenu.contains("Fractoins"), "l'ancien texte reste lisible");
    assert!(versions[1].sans_avis, "la correction de la faute n'a pas prévenu");
    assert!(!versions[0].sans_avis, "la modification utile a prévenu");
    assert_eq!(nb_evt(&fx, "cahier.seance_modifiee", e1.id).await, 2);
    // Les lecteurs voient le texte courant (projection) ; l'ancienne route lit aussi la dernière version.
    let vue = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, auj, auj + Duration::days(1)).await.unwrap();
    assert!(vue.iter().any(|s| s.contenu.as_ref().is_some_and(|c| c.contenu.contains("propriétés"))));
}

#[tokio::test]
async fn c2_echeance_avancee_demande_confirmation_et_previent_toujours() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj + Duration::days(10));
    b.echeance_heure = Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
    // Le 10ᵉ jour peut tomber un week-end : on prend le premier jour ouvré (lun.–ven.) à partir de là.
    while matches!(b.echeance_jour.unwrap().format("%a").to_string().as_str(), "Sat" | "Sun") {
        b.echeance_jour = Some(b.echeance_jour.unwrap() + Duration::days(1));
    }
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux.remove(0);
    travail_visible(&fx, t.id).await;
    let tard = t.echeance_jour;

    // Échéance avancée sans confirmation : refus (409), rien n'est modifié.
    let avance = ModifierTravailC2Body {
        echeance_jour: Some(tard - Duration::days(2)),
        echeance_heure: Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
        ..Default::default()
    };
    let r = fx.ent.cdt_travail_modifier(&fx.prof(), t.id, avance.clone()).await;
    // (le jour reculé peut être un week-end : le refus pertinent reste un refus métier)
    assert!(r.is_err_and(|e| est_refus(&e, RefusMetier::Conflit) || est_refus(&e, RefusMetier::Invalide)));
    // Date avancée à un jour ouvré : on cherche le jour ouvré précédent.
    let mut j = tard - Duration::days(1);
    while matches!(j.format("%a").to_string().as_str(), "Sat" | "Sun") {
        j -= Duration::days(1);
    }
    let avance = ModifierTravailC2Body {
        echeance_jour: Some(j),
        echeance_heure: Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
        ..Default::default()
    };
    let r = fx.ent.cdt_travail_modifier(&fx.prof(), t.id, avance.clone()).await;
    assert!(r.is_err_and(|e| est_refus(&e, RefusMetier::Conflit)), "échéance avancée sans confirmation");
    assert_eq!(fx.ent.cdt_travail_lire(&fx.prof(), t.id).await.unwrap().echeance_jour, tard, "rien n'a changé");
    // Confirmée : avis émis, même si l'auteur coche « sans avis ».
    let confirme = ModifierTravailC2Body { confirme_echeance_avancee: true, sans_avis: true, ..avance };
    let m = fx.ent.cdt_travail_modifier(&fx.prof(), t.id, confirme).await.unwrap();
    assert!(m.echeance_avancee && m.avis_emis, "l'échéance avancée prévient toujours");
    assert_eq!(m.travail.echeance_jour, j);
    assert_eq!(nb_evt(&fx, "cahier.travail_modifie", t.id).await, 1);
    // Échéance reculée : aucune confirmation, avis seulement si voulu.
    let recule = ModifierTravailC2Body {
        echeance_jour: Some(tard),
        echeance_heure: Some(chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
        sans_avis: true,
        ..Default::default()
    };
    let m = fx.ent.cdt_travail_modifier(&fx.prof(), t.id, recule).await.unwrap();
    assert!(!m.echeance_avancee && !m.avis_emis);
    // Faute de frappe dans le titre, sans avis ; avec avis ensuite.
    let m = fx
        .ent
        .cdt_travail_modifier(&fx.prof(), t.id, ModifierTravailC2Body { titre: Some("Titre corrigé".into()), sans_avis: true, ..Default::default() })
        .await
        .unwrap();
    assert!(!m.avis_emis);
    let m = fx
        .ent
        .cdt_travail_modifier(&fx.prof(), t.id, ModifierTravailC2Body { consigne: Some("Nouvelle consigne".into()), ..Default::default() })
        .await
        .unwrap();
    assert!(m.avis_emis);
    // Versions lisibles : quatre modifications, quatre versions antérieures.
    let v = fx.ent.cdt_travail_versions(&fx.prof(), t.id).await.unwrap();
    assert_eq!(v.len(), 4);
    assert_eq!(v.iter().filter(|x| x.sans_avis).count(), 2, "deux modifications silencieuses");
    // Un autre professeur ne modifie pas ce travail.
    assert!(fx
        .ent
        .cdt_travail_modifier(&fx.prof2(), t.id, ModifierTravailC2Body { titre: Some("pirate".into()), ..Default::default() })
        .await
        .is_err_and(|e| est_interdit(&e)));
}

// ───────────────────────── Contrôles de l'échéance (§9.1, §8.3) ─────────────────────────

async fn creer(fx: &Fx, b: TravailC2Body) -> Result<TravauxCreesDto, EntDbError> {
    fx.ent.cdt_travaux_creer(&fx.prof(), b).await
}

#[tokio::test]
async fn c2_controles_de_l_echeance_passee_ferme_plage_calme_et_fin_d_annee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    // Échéance dans le passé.
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj - Duration::days(2));
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    // Échéance pendant la plage calme (23 h 59 en ligne) : refusée par défaut.
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj + Duration::days(20));
    b.echeance_heure = Some(chrono::NaiveTime::from_hms_opt(23, 59, 0).unwrap());
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj + Duration::days(20));
    b.echeance_heure = Some(chrono::NaiveTime::from_hms_opt(6, 30, 0).unwrap());
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)), "6 h 30 : encore la plage calme");
    // Jour fermé (vacances, fin de semaine).
    let mut samedi = auj + Duration::days(14);
    while samedi.format("%a").to_string() != "Sat" {
        samedi += Duration::days(1);
    }
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(samedi);
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    // Au-delà de la fin de l'année scolaire.
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj + Duration::days(900));
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    // Pas d'échéance du tout, ou deux à la fois.
    assert!(creer(&fx, travail(vec![cours])).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    let mut b = travail(vec![cours]);
    b.echeance_prochaine_seance = true;
    b.echeance_jour = Some(auj + Duration::days(9));
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    // Données invalides : nature inconnue, formats sur un rendu hors ligne, usage de l'IA, HTML.
    let mut b = travail(vec![cours]);
    b.echeance_prochaine_seance = true;
    b.nature = "inconnue".into();
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    let mut b = travail(vec![cours]);
    b.echeance_prochaine_seance = true;
    b.formats = vec!["pdf".into()];
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    let mut b = travail(vec![cours]);
    b.echeance_prochaine_seance = true;
    b.consigne = "<script>alert(1)</script>".into();
    assert!(creer(&fx, b).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    assert!(contient_html("<b>gras</b>") && contient_html("</p>") && !contient_html("si a < b alors") && !contient_html("1 <2 et 3> 2"));
}

#[tokio::test]
async fn c2_echeance_apres_la_plage_calme_alerte_moins_de_24h_et_publication_differee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    // Travail programmé un mardi à 22 h pour le lendemain 18 h : publication différée à 7 h, donné moins de 24 h avant.
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj + Duration::days(3));
    while matches!(b.echeance_jour.unwrap().format("%a").to_string().as_str(), "Sat" | "Sun") {
        b.echeance_jour = Some(b.echeance_jour.unwrap() + Duration::days(1));
    }
    b.publication = "programmer".into();
    b.publie_a = Some(instant(2031, 3, 11, 21, 0));
    let r = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap();
    let t = &r.travaux[0];
    assert_eq!(t.etat, "programme");
    assert_eq!(t.visible_a, Some(instant(2031, 3, 12, 6, 0)));
    assert!(t.differee_par_plage_calme);
    assert!(r.avertissements.contains(&"differe_par_plage_calme".to_string()));
    // L'échéance est dans l'année : mais la publication de 2031 est postérieure à l'échéance → alerte (moins de 24 h
    // avant, ici négative) : l'alerte ne bloque jamais.
    assert!(r.avertissements.contains(&"donne_moins_de_24h".to_string()));
    // Non visible tant que la publication n'est pas échue : l'élève ne le voit pas.
    let agenda = agenda_de(&fx, &fx.x, auj - Duration::days(1), auj + Duration::days(20)).await;
    assert!(agenda.iter().all(|i| i.travail.id != t.id));
}

// ───────────────────────── Public restreint, plusieurs cours ─────────────────────────

#[tokio::test]
async fn c2_public_restreint_aux_eleves_du_cours_et_jamais_servi_a_la_classe_entiere() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let mut b = travail(vec![cours]);
    b.echeance_jour = Some(auj + Duration::days(6));
    while matches!(b.echeance_jour.unwrap().format("%a").to_string().as_str(), "Sat" | "Sun") {
        b.echeance_jour = Some(b.echeance_jour.unwrap() + Duration::days(1));
    }
    b.echeance_heure = Some(chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    // Un élève d'une autre classe ne peut jamais être ajouté (R-C25).
    b.eleve_ids = vec![fx.x.id, fx.y.id];
    assert!(fx.ent.cdt_travaux_creer(&fx.prof(), b.clone()).await.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    // Deux élèves de la classe : public restreint, jamais projeté pour les anciens lecteurs (qui le montreraient à toute la classe).
    b.eleve_ids = vec![fx.x.id, fx.w.id];
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), b.clone()).await.unwrap().travaux.remove(0);
    assert_eq!((t.public.as_str(), t.nb_eleves_cibles), ("restreint", 2));
    travail_visible(&fx, t.id).await;
    let (du, au) = (auj - Duration::days(1), auj + Duration::days(20));
    for e in [&fx.x, &fx.w, &fx.z] {
        assert!(agenda_de(&fx, e, du, au).await.iter().all(|i| i.travail.id != t.id), "pas de fuite vers l'ancien agenda");
    }
    // Plusieurs cours : refus d'un public restreint multi-cours.
    let mut multi = b.clone();
    multi.cours_ids = vec![cours, Uuid::new_v4()];
    assert!(fx.ent.cdt_travaux_creer(&fx.prof(), multi).await.is_err());
}

#[tokio::test]
async fn c2_un_travail_pour_plusieurs_cours_reliés_et_droits_par_cours() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let dir = Role::Direction.as_str();
    let prof0 = fx.cast.profs[0].user_id;
    let service_b = fx
        .ent
        .referentiel_creer_service(
            dir,
            e,
            CreerService {
                prof_user_id: prof0,
                matiere_id: None,
                classe_id: Some(fx.classe_b),
                groupe_id: None,
                professeur_principal: false,
            },
        )
        .await
        .unwrap();
    let (ca, cb) = (cours_de(&fx, fx.service_a).await, cours_de(&fx, service_b.id).await);
    let auj = jour(&fx).await;
    fx.seance("CDT-A", prof0, Some(fx.service_a), auj + Duration::days(7)).await;
    fx.seance("CDT-B", prof0, Some(service_b.id), auj + Duration::days(8)).await;
    // Un geste, deux cours : un travail par cours, reliés, chacun avec sa « prochaine séance ».
    let mut b = travail(vec![ca, cb]);
    b.echeance_prochaine_seance = true;
    let r = fx.ent.cdt_travaux_creer(&fx.prof(), b.clone()).await.unwrap();
    assert_eq!(r.travaux.len(), 2);
    assert_eq!(r.travaux[0].relie_id, None);
    assert_eq!(r.travaux[1].relie_id, Some(r.travaux[0].id));
    assert_ne!(r.travaux[0].echeance_seance_id, r.travaux[1].echeance_seance_id);
    // Un cours qui n'est pas le sien : tout est refusé (rien n'est créé).
    let mut mauvais = b.clone();
    let autre_cours: Uuid = sqlx::query_scalar("SELECT id FROM ent_cours WHERE etablissement_id = $1 AND id <> $2 AND id <> $3 LIMIT 1")
        .bind(e)
        .bind(ca)
        .bind(cb)
        .fetch_optional(fx.ent.pool())
        .await
        .unwrap()
        .unwrap_or_else(Uuid::new_v4);
    mauvais.cours_ids = vec![ca, autre_cours];
    mauvais.titre = "Ne doit pas exister".into();
    assert!(fx.ent.cdt_travaux_creer(&fx.prof(), mauvais).await.is_err());
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_travail WHERE etablissement_id = $1 AND titre = 'Ne doit pas exister'")
        .bind(e)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

// ───────────────────────── Retrait, copie, contribution, rétroactif ─────────────────────────

#[tokio::test]
async fn c2_retrait_avec_motif_versions_gardees_et_remises_conservees() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj).await;
    let e = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Contenu par erreur", "maintenant")).await.unwrap();
    rendre_visible(&fx).await;
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
    // Motif obligatoire ; la compta et le professeur d'un autre cours ne retirent pas.
    assert!(fx.ent.cdt_entree_retirer(&fx.prof(), e.id, "  ").await.is_err_and(|x| est_refus(&x, RefusMetier::Invalide)));
    assert!(fx.ent.cdt_entree_retirer(&fx.compta(), e.id, "erreur de saisie").await.is_err_and(|x| est_interdit(&x)));
    assert!(fx.ent.cdt_entree_retirer(&fx.prof2(), e.id, "erreur de saisie").await.is_err_and(|x| est_interdit(&x)));
    // La direction retire (avis à l'auteur) mais n'écrit pas.
    let r = fx.ent.cdt_entree_retirer(&fx.direction(), e.id, "contenu inapproprié").await.unwrap();
    assert_eq!((r.etat.as_str(), r.retire_motif.as_deref()), ("retiree", Some("contenu inapproprié")));
    // L'élève qui l'avait vue lit « retiré » ; les versions sont gardées.
    let vue = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, auj, auj + Duration::days(1)).await.unwrap();
    assert!(vue.iter().filter(|s| s.seance_id == seance).all(|s| s.contenu.as_ref().is_none_or(|c| c.contenu.contains("retiré"))));
    assert!(fx.ent.cdt_entree_versions(&fx.prof(), e.id).await.unwrap().iter().any(|v| v.contenu.contains("par erreur")));
    assert!(fx.ent.cdt_entree_retirer(&fx.prof(), e.id, "déjà retirée").await.is_err_and(|x| est_refus(&x, RefusMetier::Conflit)));
    // Travail avec remise : retrait de la diffusion, la copie de l'élève reste.
    let mut b = travail(vec![cours]);
    b.rendu = "en_ligne".into();
    b.echeance_jour = Some(auj + Duration::days(6));
    while matches!(b.echeance_jour.unwrap().format("%a").to_string().as_str(), "Sat" | "Sun") {
        b.echeance_jour = Some(b.echeance_jour.unwrap() + Duration::days(1));
    }
    b.echeance_heure = Some(chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux.remove(0);
    travail_visible(&fx, t.id).await;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("copie.pdf"))).await.unwrap();
    let r = fx.ent.cdt_travail_retirer(&fx.direction(), t.id, "consigne erronée").await.unwrap();
    assert_eq!((r.etat.as_str(), r.retire_motif.as_deref()), ("retire", Some("consigne erronée")));
    let (remises,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_travail_remise WHERE etablissement_id = $1 AND travail_id = $2")
        .bind(fx.cast.etablissement_id)
        .bind(t.id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(remises, 1, "la remise reste à l'élève");
    assert!(fx.ent.cdt_travail_modifier(&fx.prof(), t.id, ModifierTravailC2Body { titre: Some("x".into()), ..Default::default() })
        .await
        .is_err_and(|x| est_refus(&x, RefusMetier::Conflit)));
    assert_eq!(nb_evt(&fx, "cahier.travail_supprime", t.id).await, 1);
}

#[tokio::test]
async fn c2_copie_de_seance_retroactif_annee_close_et_contribution_signee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let dir = Role::Direction.as_str();
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let service_b = fx
        .ent
        .referentiel_creer_service(
            dir,
            e,
            CreerService { prof_user_id: prof0, matiere_id: None, classe_id: Some(fx.classe_b), groupe_id: None, professeur_principal: false },
        )
        .await
        .unwrap();
    let (ca, cb) = (cours_de(&fx, fx.service_a).await, cours_de(&fx, service_b.id).await);
    let auj = jour(&fx).await;
    let hier = fx.seance("CDT-A", prof0, Some(fx.service_a), auj - Duration::days(1)).await;
    let sb = fx.seance("CDT-B", prof0, Some(service_b.id), auj + Duration::days(1)).await;
    // Saisie rétroactive d'une séance passée : « renseignée le … ».
    let mut b = entree("Séance de hier, saisie oubliée", "maintenant");
    b.notions = vec!["fractions".into(), "".into(), "proportions".into()];
    let retro = fx.ent.cdt_entree_enregistrer(&fx.prof(), ca, hier, b).await.unwrap();
    assert!(retro.renseignee_le.is_some(), "saisie après coup tracée");
    assert_eq!(retro.notions, vec!["fractions".to_string(), "proportions".to_string()]);
    // Copie vers un autre cours : brouillon, même contenu, référence à l'origine ; jamais deux fois.
    let copies = fx.ent.cdt_entree_copier(&fx.prof(), retro.id, &[CopieVersBody { cours_id: cb, seance_id: sb }]).await.unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!((copies[0].etat.as_str(), copies[0].copie_de_id), ("brouillon", Some(retro.id)));
    assert_eq!(copies[0].contenu, retro.contenu);
    assert!(fx.ent.cdt_entree_copier(&fx.prof(), retro.id, &[CopieVersBody { cours_id: cb, seance_id: sb }]).await
        .is_err_and(|x| est_refus(&x, RefusMetier::Conflit)));
    // Un autre professeur ne copie pas ce qu'il ne peut pas lire.
    assert!(fx.ent.cdt_entree_copier(&fx.prof2(), retro.id, &[CopieVersBody { cours_id: cb, seance_id: sb }]).await
        .is_err_and(|x| est_interdit(&x)));
    // Contribution signée : seul un co-intervenant du cours (suppléant nommé) contribue.
    assert!(fx.ent.cdt_contribuer(&fx.prof2(), retro.id, "Ma remarque").await.is_err_and(|x| est_interdit(&x)));
    let agent2 = agent_du_compte(&fx, prof2).await;
    fx.ent
        .nommer_suppleant_for_actor(e, fx.cast.direction.user_id, "direction", fx.service_a, &NommerSuppleantBody { agent_id: agent2, du: auj, au: None, origine: None, origine_ref: None })
        .await
        .unwrap();
    let c = fx.ent.cdt_contribuer(&fx.prof2(), retro.id, "Rattrapage proposé jeudi").await.unwrap();
    assert!(!c.auteur.is_empty());
    let lu = fx.ent.cdt_entree_lire(&fx.prof(), retro.id).await.unwrap();
    assert_eq!(lu.contributions.len(), 1);
    assert_eq!(lu.contributions[0].contenu, "Rattrapage proposé jeudi");
    // Année close : plus rien ne se modifie (R-C28).
    let annee: Uuid = sqlx::query_scalar("SELECT annee_id FROM ent_edt_seance WHERE id = $1").bind(hier).fetch_one(fx.ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE id = $1").bind(annee).execute(fx.ent.pool()).await.unwrap();
    let r = fx.ent.cdt_entree_enregistrer(&fx.prof(), ca, hier, entree("Trop tard", "maintenant")).await;
    sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = NULL WHERE id = $1").bind(annee).execute(fx.ent.pool()).await.unwrap();
    assert!(r.is_err_and(|x| est_refus(&x, RefusMetier::Conflit)));
}

#[tokio::test]
async fn c2_une_correction_de_l_ancienne_route_n_est_pas_ecrasee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj).await;
    let e = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seance, entree("Texte de C2", "maintenant")).await.unwrap();
    rendre_visible(&fx).await;
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
    // L'application mobile (ancienne route) corrige le contenu.
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Texte corrigé depuis le mobile", "").await.unwrap();
    // C2 relit l'ancienne ligne avant de modifier : la correction n'est ni perdue ni écrasée sans trace.
    let r = fx.ent.cdt_entree_retirer(&fx.prof(), e.id, "test de relecture").await.unwrap();
    assert_eq!(r.etat, "retiree");
    let versions = fx.ent.cdt_entree_versions(&fx.prof(), e.id).await.unwrap();
    assert!(versions.iter().any(|v| v.contenu.contains("corrigé depuis le mobile")), "la correction mobile est gardée en version");
    assert!(versions.iter().any(|v| v.contenu.contains("Texte de C2")));
}
