//! Socle 20, lot C7 : adaptations (R-C8, §13), contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`). Critères de fin : **aucun motif visible** ; **un camarade ne détecte pas une
//! variante** (test de réponse identique : mêmes champs, mêmes versions, aucun indicateur). Le port du socle 35 est simulé par
//! `ent_cdt_amenagement_simule`. Aucun résultat ne dépend de l'heure réelle.

use super::*;
use chrono::{DateTime, Duration, Utc};
use ent_types::cahier_c2::*;
use ent_types::cahier_c5::*;
use ent_types::cahier_c7::*;

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

async fn ouvre(fx: &Fx, a_partir_de: NaiveDate) -> NaiveDate {
    sqlx::query_scalar(
        "SELECT d::date FROM generate_series($2::date, $2::date + 60, interval '1 day') AS d
          WHERE NOT ent_cdt_jour_ferme($1, d::date) ORDER BY 1 LIMIT 1",
    )
    .bind(fx.cast.etablissement_id)
    .bind(a_partir_de)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap()
}

async fn cours_a(fx: &Fx) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1").bind(fx.service_a).fetch_one(fx.ent.pool()).await.unwrap()
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

async fn travail(fx: &Fx, rendu: &str) -> Uuid {
    let cours = cours_a(fx).await;
    let j = ouvre(fx, jour(fx).await + Duration::days(5)).await;
    let id = fx
        .ent
        .cdt_travaux_creer(
            &fx.prof(),
            TravailC2Body {
                cours_ids: vec![cours],
                seance_donnee_id: None,
                echeance_seance_id: None,
                echeance_prochaine_seance: false,
                echeance_jour: Some(j),
                echeance_heure: NaiveTime::from_hms_opt(18, 0, 0),
                nature: "exercices".into(),
                titre: format!("Adapt {}", &Uuid::new_v4().to_string()[..12]),
                consigne: "Consigne commune à toute la classe.".into(),
                duree_estimee_min: Some(30),
                rendu: rendu.into(),
                retard_accepte: true,
                formats: vec![],
                eleve_ids: vec![],
                ia_autorisee: "non".into(),
                publication: "maintenant".into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
            },
        )
        .await
        .unwrap()
        .travaux[0]
        .id;
    rendre_visible(fx).await;
    id
}

fn variante(consigne: &str) -> VarianteBody {
    VarianteBody { libelle_interne: "Version allégée (interne)".into(), consigne: consigne.into(), duree_estimee_min: Some(10), echeance_jour: None, echeance_heure: None }
}

async fn agenda(fx: &Fx, a: &Acteur<'_>, eleve: Option<Uuid>) -> ent_types::cahier_c3::AgendaDto {
    let auj = jour(fx).await;
    fx.ent.cdt_agenda(a, eleve, auj, auj + Duration::days(30)).await.unwrap()
}

// ───────────────────────── variantes ─────────────────────────

#[tokio::test]
async fn variante_substituee_pour_l_eleve_un_camarade_ne_detecte_rien_reponse_identique() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let t = travail(&fx, "aucun").await;
    let v = fx.ent.cdt_variante_creer(&fx.prof(), t, variante("Version courte : exercice 1 seulement.")).await.unwrap();
    assert_eq!(v.eleves.len(), 0);
    // Avant l'attribution, tout le monde lit la version commune.
    let avant_x = agenda(&fx, &fx.eleve(&fx.x), None).await.lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(avant_x.consigne.as_deref().unwrap().contains("commune"));
    let version_avant = avant_x.version;
    let v = fx.ent.cdt_variante_attribuer(&fx.prof(), v.id, AttributionBody { eleve_ids: vec![fx.x.id] }).await.unwrap();
    assert_eq!(v.eleves.len(), 1);
    // X lit **sa** version comme la seule ; son camarade W lit la version commune.
    let lx = agenda(&fx, &fx.eleve(&fx.x), None).await.lignes.into_iter().find(|l| l.id == t).unwrap();
    let lw = agenda(&fx, &fx.eleve(&fx.w), None).await.lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(lx.consigne.as_deref().unwrap().contains("Version courte"));
    assert_eq!(lx.duree_estimee_min, Some(10));
    assert!(lw.consigne.as_deref().unwrap().contains("commune"));
    assert_eq!(lw.duree_estimee_min, Some(30));
    // Test de réponse identique : mêmes champs, mêmes versions, aucun indicateur de variante (ni libellé interne).
    let (jx, jw) = (serde_json::to_value(&lx).unwrap(), serde_json::to_value(&lw).unwrap());
    let cles = |j: &serde_json::Value| j.as_object().unwrap().keys().cloned().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(cles(&jx), cles(&jw), "les champs servis sont les mêmes pour tous les élèves");
    assert_eq!((lx.version, lx.modifie_le), (version_avant, avant_x.modifie_le), "attribuer une variante ne touche pas la version du travail");
    assert_eq!(lx.version, lw.version);
    for j in [&jx, &jw] {
        let s = j.to_string().to_lowercase();
        assert!(!s.contains("variante") && !s.contains("allégée") && !s.contains("interne"), "{s}");
    }
    // Le responsable de X lit la version de X ; celui de W, la version commune.
    let px = agenda(&fx, &fx.parent(&fx.x), Some(fx.x.id)).await.lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(px.consigne.as_deref().unwrap().contains("Version courte"));
    // Seul le professeur en saisie lit les variantes et leurs élèves nommés.
    assert_eq!(fx.ent.cdt_variantes_lister(&fx.prof(), t).await.unwrap().len(), 1);
    for ac in [fx.eleve(&fx.x), fx.eleve(&fx.w), fx.parent(&fx.x), fx.prof2(), fx.cpe(), fx.direction()] {
        assert!(est_interdit(&fx.ent.cdt_variantes_lister(&ac, t).await.unwrap_err()), "{}", ac.role);
    }
    // Un élève hors du public ne reçoit pas de variante (R-C25) ; HTML refusé ; échéance passée refusée.
    assert!(est_refus(&fx.ent.cdt_variante_attribuer(&fx.prof(), v.id, AttributionBody { eleve_ids: vec![fx.y.id] }).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_refus(&fx.ent.cdt_variante_creer(&fx.prof(), t, variante("<script>alert(1)</script>")).await.unwrap_err(), RefusMetier::Invalide));
    let mut passee = variante("ok");
    passee.echeance_jour = Some(jour(&fx).await - Duration::days(2));
    passee.echeance_heure = NaiveTime::from_hms_opt(8, 0, 0);
    assert!(est_refus(&fx.ent.cdt_variante_creer(&fx.prof(), t, passee).await.unwrap_err(), RefusMetier::Invalide));
    // Une seule variante active par élève : la nouvelle attribution retire l'ancienne ; retirer ramène à la version commune.
    let v2 = fx.ent.cdt_variante_creer(&fx.prof(), t, variante("Autre version : exercices 2 et 3.")).await.unwrap();
    fx.ent.cdt_variante_attribuer(&fx.prof(), v2.id, AttributionBody { eleve_ids: vec![fx.x.id] }).await.unwrap();
    let lx2 = agenda(&fx, &fx.eleve(&fx.x), None).await.lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(lx2.consigne.as_deref().unwrap().contains("Autre version"));
    let listee = fx.ent.cdt_variantes_lister(&fx.prof(), t).await.unwrap();
    assert_eq!(listee.iter().map(|v| v.eleves.len()).sum::<usize>(), 1);
    fx.ent.cdt_variante_retirer(&fx.prof(), v2.id, AttributionBody { eleve_ids: vec![fx.x.id] }).await.unwrap();
    let lx3 = agenda(&fx, &fx.eleve(&fx.x), None).await.lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(lx3.consigne.as_deref().unwrap().contains("commune"));
}

#[tokio::test]
async fn variante_avec_son_echeance_et_modification_prevenant_les_seuls_eleves_concernes() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let t = travail(&fx, "en_ligne").await;
    let base: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    let j = ouvre(&fx, jour(&fx).await + Duration::days(12)).await;
    let mut b = variante("Version avec un délai propre.");
    b.echeance_jour = Some(j);
    b.echeance_heure = NaiveTime::from_hms_opt(18, 0, 0);
    let v = fx.ent.cdt_variante_creer(&fx.prof(), t, b.clone()).await.unwrap();
    assert!(v.echeance_le.unwrap() > base);
    fx.ent.cdt_variante_attribuer(&fx.prof(), v.id, AttributionBody { eleve_ids: vec![fx.x.id] }).await.unwrap();
    // L'échéance effective de X est celle de sa variante (rendu en ligne comprise) ; celle de W reste commune.
    let rx = fx.ent.cdt_remise_lire(&fx.eleve(&fx.x), t, None).await.unwrap();
    let rw = fx.ent.cdt_remise_lire(&fx.eleve(&fx.w), t, None).await.unwrap();
    assert_eq!(rx.echeance_le, v.echeance_le.unwrap());
    assert_eq!(rw.echeance_le, base);
    let (jx, jw) = (serde_json::to_value(&rx).unwrap(), serde_json::to_value(&rw).unwrap());
    let cles = |j: &serde_json::Value| j.as_object().unwrap().keys().cloned().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(cles(&jx), cles(&jw), "la situation d'un élève a les mêmes champs, variante ou non");
    // Modifier la variante prévient X seul : un événement, une version.
    let avant = fx.nb_evenements("cahier.travail_modifie").await;
    let v2 = fx.ent.cdt_variante_modifier(&fx.prof(), v.id, VarianteBody { consigne: "Version retouchée.".into(), ..b }).await.unwrap();
    assert_eq!(v2.version, v.version + 1);
    assert_eq!(fx.nb_evenements("cahier.travail_modifie").await, avant + 1);
    // Une variante ne contourne pas la clôture ni un autre cours : un autre professeur ne la modifie pas.
    assert!(est_interdit(&fx.ent.cdt_variante_modifier(&fx.prof2(), v.id, variante("x")).await.unwrap_err()));
}

// ───────────────────────── aménagements appliqués ─────────────────────────

#[tokio::test]
async fn pastilles_et_majoration_du_delai_apercu_puis_application_motif_jamais_servi() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (e, cours, auj) = (fx.cast.etablissement_id, cours_a(&fx).await, jour(&fx).await);
    for (code, pct) in [("temps_majore", Some(50)), ("police_agrandie", None)] {
        sqlx::query("INSERT INTO ent_cdt_amenagement_simule (id, etablissement_id, eleve_id, code, valeur_pct, du) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(fx.x.id)
            .bind(code)
            .bind(pct)
            .bind(auj - Duration::days(10))
            .execute(fx.ent.pool())
            .await
            .unwrap();
    }
    let p = fx.ent.cdt_pastilles_cours(&fx.prof(), cours).await.unwrap();
    let px = p.eleves.iter().find(|x| x.eleve_id == fx.x.id).unwrap();
    assert_eq!(px.pastilles, vec!["Police agrandie".to_string(), "Temps majoré (+50 %)".to_string()]);
    assert!(p.eleves.iter().filter(|x| x.eleve_id != fx.x.id).all(|x| x.pastilles.is_empty()));
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("PAP") && !json.contains("PPS") && !json.contains("diagnostic"), "{json}");
    // Les pastilles ne sont ni pour l'élève, ni pour la famille, ni pour un autre professeur.
    for ac in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.prof2(), fx.cpe()] {
        assert!(est_interdit(&fx.ent.cdt_pastilles_cours(&ac, cours).await.unwrap_err()), "{}", ac.role);
    }
    // Travail donné 2 jours avant l'échéance : +50 % = +24 h pour X seul.
    let t = travail(&fx, "en_ligne").await;
    sqlx::query("UPDATE ent_cdt_travail SET visible_a = echeance_le - interval '2 days' WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    let base: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    let apercu = fx.ent.cdt_amenagements_appliquer(&fx.prof(), t, false).await.unwrap();
    assert!(!apercu.applique);
    assert_eq!(apercu.eleves.len(), 1);
    assert_eq!(apercu.eleves[0].echeance_le, base + Duration::hours(24));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_echeance_eleve WHERE travail_id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n, 0, "l'aperçu n'écrit rien");
    let fait = fx.ent.cdt_amenagements_appliquer(&fx.prof(), t, true).await.unwrap();
    assert_eq!((fait.appliques, fait.applique), (1, true));
    let rx = fx.ent.cdt_remise_lire(&fx.eleve(&fx.x), t, None).await.unwrap();
    let rw = fx.ent.cdt_remise_lire(&fx.eleve(&fx.w), t, None).await.unwrap();
    assert_eq!(rx.echeance_le, base + Duration::hours(24));
    assert_eq!(rw.echeance_le, base);
    // Aucun motif visible de l'élève : « délai accordé », jamais « aménagement ».
    let s = serde_json::to_string(&rx).unwrap().to_lowercase();
    assert!(rx.delai_accorde && !s.contains("amenagement") && !s.contains("majoré") && !s.contains("temps_majore"), "{s}");
    let motif: String = sqlx::query_scalar("SELECT motif_code FROM ent_cdt_echeance_eleve WHERE travail_id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(motif, "amenagement");
    // Idempotent ; un autre professeur n'applique rien ; un travail non publié n'est pas concerné.
    assert_eq!(fx.ent.cdt_amenagements_appliquer(&fx.prof(), t, true).await.unwrap().appliques, 0);
    assert!(est_interdit(&fx.ent.cdt_amenagements_appliquer(&fx.prof2(), t, true).await.unwrap_err()));
}

// ───────────────────────── AESH ─────────────────────────

#[tokio::test]
async fn aesh_voit_les_seuls_eleves_accompagnes_ajoute_une_note_ne_lit_ni_copies_ni_retours() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (e, cours, auj) = (fx.cast.etablissement_id, cours_a(&fx).await, jour(&fx).await);
    // Le compte du second professeur de la fixture joue l'AESH (il n'enseigne rien dans ce cours) : un agent sans saisie.
    let aesh = fx.cast.profs[1].user_id;
    let affecter = |du: NaiveDate, fin: Option<NaiveDate>| {
        let pool = fx.ent.pool().clone();
        let direction = fx.cast.direction.user_id;
        let eleve = fx.x.id;
        async move {
            sqlx::query("INSERT INTO ent_aesh_affectation (id, etablissement_id, eleve_id, aesh_nom, aesh_user_id, date_debut, date_fin, affecte_par_user_id) VALUES ($1, $2, $3, 'Accompagnante', $4, $5, $6, $7)")
                .bind(Uuid::new_v4())
                .bind(e)
                .bind(eleve)
                .bind(aesh)
                .bind(du)
                .bind(fin)
                .bind(direction)
                .execute(&pool)
                .await
                .unwrap();
        }
    };
    affecter(auj - Duration::days(5), None).await;
    let t = travail(&fx, "en_ligne").await;
    let v = fx.ent.cdt_variante_creer(&fx.prof(), t, variante("Version de X.")).await.unwrap();
    fx.ent.cdt_variante_attribuer(&fx.prof(), v.id, AttributionBody { eleve_ids: vec![fx.x.id] }).await.unwrap();
    // L'AESH voit ses élèves accompagnés, leur agenda (avec **leur** version) et le cahier des cours concernés.
    let mes = fx.ent.cdt_aesh_eleves(&fx.prof2()).await.unwrap();
    assert_eq!(mes.iter().map(|m| m.eleve_id).collect::<Vec<_>>(), vec![fx.x.id]);
    let ag = fx.ent.cdt_aesh_agenda(&fx.prof2(), fx.x.id, auj, auj + Duration::days(30)).await.unwrap();
    let l = ag.lignes.iter().find(|l| l.id == t).unwrap();
    assert!(l.consigne.as_deref().unwrap().contains("Version de X"));
    assert!(l.fait.is_none(), "la coche n'est jamais lue d'un adulte (R-C14)");
    assert!(fx.ent.cdt_aesh_cahier(&fx.prof2(), fx.x.id, cours).await.is_ok());
    // Un élève non accompagné : rien (agenda, cahier, note).
    assert!(est_interdit(&fx.ent.cdt_aesh_agenda(&fx.prof2(), fx.w.id, auj, auj + Duration::days(30)).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_aesh_cahier(&fx.prof2(), fx.w.id, cours).await.unwrap_err()));
    let note = |eleve: Uuid, texte: &str| NoteAccompagnementBody { travail_id: Some(t), entree_id: None, eleve_id: eleve, texte: texte.into() };
    assert!(est_interdit(&fx.ent.cdt_note_accompagnement_ajouter(&fx.prof2(), note(fx.w.id, "x")).await.unwrap_err()));
    // La note d'accompagnement : texte simple, un travail ou une entrée, de son élève.
    let n = fx.ent.cdt_note_accompagnement_ajouter(&fx.prof2(), note(fx.x.id, "Consigne reformulée en classe, exercice 1 fait à l'oral.")).await.unwrap();
    assert!(n.texte.contains("reformulée"));
    assert!(est_refus(&fx.ent.cdt_note_accompagnement_ajouter(&fx.prof2(), note(fx.x.id, "<b>x</b>")).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_refus(&fx.ent.cdt_note_accompagnement_ajouter(&fx.prof2(), NoteAccompagnementBody { travail_id: Some(t), entree_id: Some(Uuid::new_v4()), eleve_id: fx.x.id, texte: "x".into() }).await.unwrap_err(), RefusMetier::Invalide));
    // Lue des professeurs du cours ; ni de l'élève, ni de la famille, ni de la CPE.
    let lues = fx.ent.cdt_notes_accompagnement_lire(&fx.prof(), Some(t), None).await.unwrap();
    assert_eq!(lues.len(), 1);
    assert!(!lues[0].auteur.contains('@'));
    for ac in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.cpe()] {
        assert!(est_interdit(&fx.ent.cdt_notes_accompagnement_lire(&ac, Some(t), None).await.unwrap_err()), "{}", ac.role);
    }
    // Ni copies, ni retours, ni suivi : l'AESH n'a aucune saisie sur le cours.
    let ech: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    let accuse = fx.ent.cdt_deposer(&fx.eleve(&fx.x), t, crate::cahier_c5::DepotEntree { texte: Some("ma copie".into()), commentaire: None, fichiers: vec![pdf("a.pdf")] }, ech - Duration::hours(1)).await.unwrap();
    assert!(est_interdit(&fx.ent.cdt_suivi(&fx.prof2(), t).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_piece_lien(&fx.prof2(), accuse.pieces[0].id, None).await.unwrap_err()));
    // Hors période d'accompagnement : plus rien.
    sqlx::query("UPDATE ent_aesh_affectation SET date_fin = ent_aujourdhui_etab(etablissement_id) - 1 WHERE etablissement_id = $1").bind(e).execute(fx.ent.pool()).await.unwrap();
    assert!(fx.ent.cdt_aesh_eleves(&fx.prof2()).await.unwrap().is_empty());
    assert!(est_interdit(&fx.ent.cdt_aesh_agenda(&fx.prof2(), fx.x.id, auj, auj + Duration::days(30)).await.unwrap_err()));
}

// ───────────────────────── réglages d'affichage ─────────────────────────

#[tokio::test]
async fn reglages_d_affichage_de_l_eleve_seul_jamais_lus_d_un_adulte() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let a = fx.eleve(&fx.x);
    assert_eq!(fx.ent.cdt_affichage_lire(&a).await.unwrap(), AffichageDto::default());
    let r = AffichageDto { taille_police: "grande".into(), interligne: "large".into(), lecture_audio: true, animations_reduites: true };
    assert_eq!(fx.ent.cdt_affichage_regler(&a, r.clone()).await.unwrap(), r);
    assert_eq!(fx.ent.cdt_affichage_lire(&a).await.unwrap(), r);
    // Chaque élève a les siens ; un réglage inconnu est refusé.
    assert_eq!(fx.ent.cdt_affichage_lire(&fx.eleve(&fx.w)).await.unwrap(), AffichageDto::default());
    assert!(est_refus(&fx.ent.cdt_affichage_regler(&a, AffichageDto { taille_police: "geante".into(), ..r.clone() }).await.unwrap_err(), RefusMetier::Invalide));
    // Mise à jour idempotente (une seule ligne par élève).
    fx.ent.cdt_affichage_regler(&a, AffichageDto { lecture_audio: false, ..r }).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_affichage WHERE eleve_id = $1").bind(fx.x.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    // Aucun adulte : ni famille, ni professeur, ni vie scolaire, ni direction.
    for ac in [fx.parent(&fx.x), fx.prof(), fx.cpe(), fx.direction()] {
        assert!(est_interdit(&fx.ent.cdt_affichage_lire(&ac).await.unwrap_err()), "{}", ac.role);
        assert!(est_interdit(&fx.ent.cdt_affichage_regler(&ac, AffichageDto::default()).await.unwrap_err()), "{}", ac.role);
    }
}
