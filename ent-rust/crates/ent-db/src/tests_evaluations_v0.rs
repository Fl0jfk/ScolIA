//! Socle 21, lot V0 « correctifs urgents » : tests d'attaque et de parcours contre la base
//! locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, scénarios ignorés.
//!
//! Chaque scénario part d'un établissement neuf (`monde`, de `tests_evaluations`).

use super::*;
use crate::tests_evaluations::{devoir, labo, monde, r, statut, Monde};
use domain_eleve::{today_utc, DocConfidentialite, DocTiroir};
use ent_types::evaluations::*;
use sqlx::Row;
use uuid::Uuid;

/// Une classe de trois élèves dont Hannah (`eleves[0]`) a deux responsables : `parent_lea`
/// (voit les notes) et `parent_tom` (**privé** de l'accès aux notes par la direction).
async fn avec_devoir_note(ent: &PgEnt, cast: &LaboCast, m: &Monde) -> Uuid {
    let (etab, prof, a) = (m.etab, cast.prof.user_id, m.eleves[0]);
    let d = ent
        .creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "Contrôle", 1.0, 20.0))
        .await
        .unwrap();
    ent.saisir_grille_for_actor(etab, prof, "prof", d.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(14.0))] })
        .await
        .unwrap();
    ent.publier_devoir_for_actor(etab, prof, "prof", d.id).await.unwrap();
    d.id
}

/// Génère puis publie les bulletins de la classe (période fermée) ; rend l'id du document
/// « bulletin » déposé au dossier de Hannah.
async fn publier_bulletin(ent: &PgEnt, cast: &LaboCast, m: &Monde) -> Uuid {
    let dir = cast.direction.user_id;
    statut(ent, cast, m, "fermee").await;
    ent.generer_bulletins_for_actor(
        m.etab,
        dir,
        "direction",
        &GenererBulletinsBody { classe_id: m.classe, periode_id: m.periode, motif: None },
    )
    .await
    .unwrap();
    ent.publier_bulletins_for_actor(
        m.etab,
        dir,
        "direction",
        &PublierBulletinsBody { classe_id: m.classe, periode_id: m.periode },
    )
    .await
    .unwrap();
    let (doc,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_eleve_document
          WHERE etablissement_id = $1 AND eleve_id = $2 AND source = 'bulletin' AND confidentialite = 'standard'",
    )
    .bind(m.etab)
    .bind(m.eleves[0])
    .fetch_one(ent.pool())
    .await
    .unwrap();
    doc
}

// ─── D3 : fuites avant publication et sans droit aux notes ───────────────────

#[tokio::test]
async fn bulletin_illisible_pour_un_parent_prive_d_acces_aux_notes() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, a) = (m.etab, m.eleves[0]);
    let (lea, tom) = (cast.parent_lea.user_id, cast.parent_tom.user_id);
    avec_devoir_note(&ent, &cast, &m).await;
    let doc = publier_bulletin(&ent, &cast, &m).await;

    // Le parent qui voit les notes lit et télécharge le bulletin…
    let (d, octets) = ent.telecharger_document_bytes_for_actor(etab, lea, "parent", doc).await.unwrap();
    assert_eq!(d.tiroir, DocTiroir::Famille);
    assert_eq!(d.confidentialite, DocConfidentialite::Standard);
    assert!(!octets.is_empty());
    // … le parent privé d'accès aux notes, lui, est refusé (téléchargement et liste).
    assert!(
        ent.telecharger_document_bytes_for_actor(etab, tom, "parent", doc).await.is_err(),
        "un parent sans droit aux notes ne doit pas télécharger le bulletin"
    );
    let liste = ent.list_documents_meta_eleve_for_actor(etab, tom, "parent", a).await.unwrap();
    let (_, accessible) = liste.iter().find(|(d, _)| d.id == doc).expect("document listé");
    assert!(!accessible, "le bulletin n'est pas accessible au parent sans droit aux notes");
    // Un document « livret de compétences » suit la même règle.
    let livret = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, code, mime_type,
                                         storage_key, confidentialite, source, created_by_user_id)
         VALUES ($1,$2,$3,'famille','Livret','LIVRET_X','application/pdf',$4,'standard','manuel',$5)",
    )
    .bind(livret)
    .bind(etab)
    .bind(a)
    .bind(format!("k-{livret}"))
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(ent.telecharger_document_bytes_for_actor(etab, tom, "parent", livret).await.is_err());
    // Un document famille ordinaire reste lisible par ce même parent (pas de sur-blocage).
    let autre = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, code, mime_type,
                                         storage_key, confidentialite, source, created_by_user_id)
         VALUES ($1,$2,$3,'famille','Circulaire','CIRC','application/pdf',$4,'standard','manuel',$5)",
    )
    .bind(autre)
    .bind(etab)
    .bind(a)
    .bind(format!("k-{autre}"))
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let liste = ent.list_documents_meta_eleve_for_actor(etab, tom, "parent", a).await.unwrap();
    assert!(liste.iter().any(|(d, ok)| d.id == autre && *ok));
}

#[tokio::test]
async fn appreciations_et_competences_invisibles_a_la_famille_avant_publication() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, a) = (m.etab, cast.prof.user_id, m.eleves[0]);
    let (lea, dir) = (cast.parent_lea.user_id, cast.direction.user_id);
    avec_devoir_note(&ent, &cast, &m).await;
    // Un compte élève lié à Hannah.
    let eleve = ent
        .register_login(&format!("v0-{}@ent.lab", Uuid::new_v4().simple()), ent_auth::Role::Eleve, "Hannah", etab)
        .await
        .unwrap();
    ent.link_eleve_compte(etab, a, eleve.user_id).await.unwrap();

    // Appréciation de matière et générale (période ouverte) + une compétence évaluée.
    for (matiere, texte) in [(Some(m.maths), "Très bon trimestre."), (None, "Élève sérieuse.")] {
        ent.saisir_appreciation_eva_for_actor(etab, prof, "prof", &AppreciationBody {
            eleve_id: a, periode_id: m.periode, matiere_id: matiere, texte: texte.into(),
        }).await.unwrap();
    }
    let item = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_competence_item (id, etablissement_id, domaine, code, libelle) VALUES ($1,$2,'D1',$3,'Lire')")
        .bind(item).bind(etab).bind(format!("C{}", &item.simple().to_string()[..6]))
        .execute(ent.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_competence_evaluation (id, etablissement_id, eleve_id, item_id, periode_id, niveau, auteur_user_id)
         VALUES ($1,$2,$3,$4,$5,'satisfaisante',$6)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(a).bind(item).bind(m.periode).bind(prof)
    .execute(ent.pool()).await.unwrap();

    // Avant publication : le personnel habilité les lit, la famille et l'élève non.
    let lire_app = |user: Uuid, role: &'static str| {
        let ent = &ent;
        async move { ent.list_appreciations_for_actor(etab, user, role, a, None).await.unwrap().len() }
    };
    assert_eq!(lire_app(dir, "direction").await, 2);
    assert_eq!(lire_app(lea, "parent").await, 0, "appréciations non publiées : invisibles de la famille");
    assert_eq!(lire_app(eleve.user_id, "eleve").await, 0, "… et de l'élève");
    let lire_comp = |user: Uuid, role: &'static str| {
        let ent = &ent;
        async move { ent.list_evaluations_eleve_for_actor(etab, user, role, a, None).await.unwrap().len() }
    };
    assert_eq!(lire_comp(dir, "direction").await, 1);
    assert_eq!(lire_comp(lea, "parent").await, 0, "compétences non publiées : invisibles");
    assert_eq!(lire_comp(eleve.user_id, "eleve").await, 0);
    // Même avec le filtre par période explicite.
    assert!(ent.list_appreciations_for_actor(etab, lea, "parent", a, Some(m.periode)).await.unwrap().is_empty());

    // Publication du bulletin : tout devient visible.
    publier_bulletin(&ent, &cast, &m).await;
    assert_eq!(lire_app(lea, "parent").await, 2);
    assert_eq!(lire_app(eleve.user_id, "eleve").await, 2);
    assert_eq!(lire_comp(lea, "parent").await, 1);
}

#[tokio::test]
async fn conseils_de_l_eleve_soumis_au_droit_du_lien() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, a) = (m.etab, m.eleves[0]);
    let (lea, tom) = (cast.parent_lea.user_id, cast.parent_tom.user_id);
    assert!(ent.list_conseils_eleve_for_actor(etab, lea, "parent", a).await.is_ok());
    let r = ent.list_conseils_eleve_for_actor(etab, tom, "parent", a).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

// ─── D2 : réouverture d'une période, motif et journal ────────────────────────

#[tokio::test]
async fn rouvrir_une_periode_exige_un_motif_et_laisse_une_trace() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, dir) = (m.etab, cast.direction.user_id);
    let chg = |statut: &'static str, motif: Option<&'static str>| {
        let (ent, m) = (&ent, &m);
        async move { ent.changer_statut_periode_for_actor(etab, dir, "direction", m.periode, statut, motif).await }
    };
    chg("fermee", None).await.unwrap();
    // Rouvrir sans motif, ou avec un motif vide : refusé, et la période reste fermée.
    assert!(matches!(chg("ouverte", None).await, Err(EntDbError::SaisieInvalide(_))));
    assert!(matches!(chg("ouverte", Some("  ")).await, Err(EntDbError::SaisieInvalide(_))));
    assert!(matches!(chg("partielle_appreciations", Some("ab")).await, Err(EntDbError::SaisieInvalide(_))));
    assert_eq!(ent.periodes_notes(etab).await.unwrap().iter().find(|p| p.id == m.periode).unwrap().statut, "fermee");
    // Avec motif : accepté, journalisé.
    chg("ouverte", Some("Erreur de saisie signalée par le professeur de maths")).await.unwrap();
    chg("fermee", None).await.unwrap(); // refermer : pas de motif requis
    let j = ent.journal_periode_for_actor(etab, dir, "direction", m.periode).await.unwrap();
    let apres: Vec<(&str, &str, Option<&str>)> = j
        .iter()
        .rev()
        .map(|l| (l.statut_avant.as_str(), l.statut_apres.as_str(), l.motif.as_deref()))
        .collect();
    assert_eq!(
        apres,
        vec![
            ("ouverte", "fermee", None),
            ("fermee", "ouverte", Some("Erreur de saisie signalée par le professeur de maths")),
            ("ouverte", "fermee", None),
        ]
    );
    assert!(j.iter().all(|l| l.acteur_user_id == dir));
    // Le journal est en ajout seul, même pour le propriétaire des tables.
    let e = sqlx::query("UPDATE ent_note_periode_journal SET motif = 'effacé' WHERE etablissement_id = $1")
        .bind(etab).execute(ent.pool()).await;
    assert!(e.is_err(), "modification du journal interdite");
    let e = sqlx::query("DELETE FROM ent_note_periode_journal WHERE etablissement_id = $1")
        .bind(etab).execute(ent.pool()).await;
    assert!(e.is_err(), "suppression du journal interdite");

    // Matrice de droits : ni l'administratif, ni le professeur, ni la famille ne rouvrent,
    // ni ne lisent le journal.
    let (adm, prof, lea) = (cast.administratif.user_id, cast.prof.user_id, cast.parent_lea.user_id);
    for (u, role) in [(adm, "administratif"), (prof, "prof"), (lea, "parent")] {
        assert!(matches!(
            ent.changer_statut_periode_for_actor(etab, u, role, m.periode, "ouverte", Some("tentative de réouverture")).await,
            Err(EntDbError::Forbidden)
        ), "{role}");
        assert!(matches!(
            ent.journal_periode_for_actor(etab, u, role, m.periode).await,
            Err(EntDbError::Forbidden)
        ), "{role}");
    }
    assert_eq!(ent.periodes_notes(etab).await.unwrap().iter().find(|p| p.id == m.periode).unwrap().statut, "fermee");
    // Cloisonnement : la direction d'un autre établissement ne touche pas cette période.
    let autre = ent.seed_labo_cast().await.unwrap();
    assert!(ent
        .changer_statut_periode_for_actor(autre.etablissement_id, autre.direction.user_id, "direction", m.periode, "ouverte", Some("intrusion"))
        .await
        .is_err());
    assert!(ent.journal_periode_for_actor(autre.etablissement_id, autre.direction.user_id, "direction", m.periode).await.map(|l| l.is_empty()).unwrap_or(true));
}

// ─── D10, R-V5 : une absence n'est jamais un zéro ────────────────────────────

#[tokio::test]
async fn une_absence_n_est_jamais_un_zero_et_le_rattrapage_reste_possible() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, dir) = (m.etab, cast.prof.user_id, cast.direction.user_id);
    let (a, b, c) = (m.eleves[0], m.eleves[1], m.eleves[2]);
    let d1 = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "C1", 1.0, 20.0)).await.unwrap();
    let d2 = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "C2", 1.0, 20.0)).await.unwrap();
    // Hannah : 12 puis absente ; Susan : absente « à rattraper » ; Michael : 8 puis absent.
    ent.saisir_grille_for_actor(etab, prof, "prof", d1.id, &SaisieGrilleBody {
        resultats: vec![r(a, "note", Some(12.0)), r(b, "a_rattraper", None), r(c, "note", Some(8.0))],
    }).await.unwrap();
    let rap = ent.saisir_grille_for_actor(etab, prof, "prof", d2.id, &SaisieGrilleBody {
        resultats: vec![r(a, "absent", None), r(b, "a_rattraper", None), r(c, "absent", None)],
    }).await.unwrap();
    assert_eq!((rap.enregistres, rap.refuses.len()), (3, 0));
    // « absent » avec une valeur est incohérent : refusé (l'absence ne porte jamais de note).
    let rap = ent.saisir_grille_for_actor(etab, prof, "prof", d2.id, &SaisieGrilleBody {
        resultats: vec![r(a, "absent", Some(0.0))],
    }).await.unwrap();
    assert_eq!((rap.enregistres, rap.refuses.len()), (0, 1));

    // Réglage « absent = 0 » : refusé, même par la direction ; la règle se lit comme fausse.
    assert!(matches!(
        ent.definir_regles_calcul_for_actor(etab, dir, "direction", &ReglesCalculDto { arrondi: "centieme".into(), absent_compte_zero: true }).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Même forcé en base (ancienne donnée) : la contrainte l'interdit, et la lecture reste fausse.
    let forcer = sqlx::query(
        "INSERT INTO ent_eva_regle (etablissement_id, arrondi, absent_compte_zero) VALUES ($1, 'centieme', true)
         ON CONFLICT (etablissement_id) DO UPDATE SET absent_compte_zero = true",
    )
    .bind(etab)
    .execute(ent.pool())
    .await;
    assert!(forcer.is_err(), "la base refuse absent_compte_zero = true");
    assert!(!ent.regles_calcul_for_actor(etab, "direction").await.unwrap().absent_compte_zero);

    // Calculs : Hannah 12 (l'absence au 2ᵉ devoir ne compte pas), Susan aucune moyenne
    // (jamais 0), Michael 8.
    let v = ent.vue_classe_for_actor(etab, dir, "direction", m.classe, m.periode).await.unwrap();
    let moy = |id: Uuid| v.lignes.iter().find(|l| l.eleve_id == id).unwrap().moyenne_generale;
    assert_eq!(moy(a), Some(12.0));
    assert_eq!(moy(b), None, "des absences seules ne donnent jamais une moyenne de 0");
    assert_eq!(moy(c), Some(8.0));
    // Le rattrapage reste possible : Susan passe le devoir plus tard.
    ent.saisir_grille_for_actor(etab, prof, "prof", d1.id, &SaisieGrilleBody { resultats: vec![r(b, "note", Some(15.0))] }).await.unwrap();
    let v = ent.vue_classe_for_actor(etab, dir, "direction", m.classe, m.periode).await.unwrap();
    assert_eq!(v.lignes.iter().find(|l| l.eleve_id == b).unwrap().moyenne_generale, Some(15.0));
    // Aucune absence n'est stockée comme une note de 0 en base.
    let (zeros,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_note WHERE etablissement_id = $1 AND statut <> 'note' AND valeur <> 0",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(zeros, 0);
}

// ─── D8 : unicité d'un résultat par (devoir, élève) ──────────────────────────

#[tokio::test]
async fn un_resultat_par_devoir_et_par_eleve() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, a) = (m.etab, cast.prof.user_id, m.eleves[0]);
    let d = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "C1", 1.0, 20.0)).await.unwrap();
    ent.saisir_grille_for_actor(etab, prof, "prof", d.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(10.0))] }).await.unwrap();
    let doublon = sqlx::query(
        "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note, auteur_user_id, devoir_id, periode_id)
         VALUES ($1,$2,$3,$4,18,20,ent_aujourdhui(),$5,$6,$7)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(a).bind(m.maths).bind(prof).bind(d.id).bind(m.periode)
    .execute(ent.pool())
    .await;
    assert!(doublon.is_err(), "un second résultat pour le même (devoir, élève) est refusé par la base");
    // Les notes hors devoir (sans devoir_id) restent libres.
    for v in [11.0_f64, 12.0] {
        sqlx::query(
            "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note, auteur_user_id)
             VALUES ($1,$2,$3,$4,$5,20,ent_aujourdhui(),$6)",
        )
        .bind(Uuid::new_v4()).bind(etab).bind(a).bind(m.maths).bind(v).bind(prof)
        .execute(ent.pool())
        .await
        .unwrap();
    }
}

// ─── D12 : un seul moteur de calcul (exact) ──────────────────────────────────

#[tokio::test]
async fn les_lectures_historiques_utilisent_le_moteur_exact() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, a) = (m.etab, cast.prof.user_id, m.eleves[0]);
    // 3 notes /20 coefficient 1 : 10,005 n'existe pas (deux décimales), mais 10,01 + 10,02 + 10,02
    // donne 10,0166… ; l'ancien calcul (f64 arrondi au centième) et le moteur exact doivent
    // rendre la même moyenne affichée 10,02, calculée sans arrondi intermédiaire.
    for v in [10.01_f64, 10.02, 10.02] {
        let d = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "C", 1.0, 20.0)).await.unwrap();
        ent.saisir_grille_for_actor(etab, prof, "prof", d.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(v))] }).await.unwrap();
        ent.publier_devoir_for_actor(etab, prof, "prof", d.id).await.unwrap();
    }
    let hist = ent.moyennes_eleve_periode(etab, a, m.periode).await.unwrap();
    let exacte = hist.iter().find(|x| x.matiere_id == m.maths).unwrap().moyenne_sur_20;
    let rel = ent.releve_eleve_for_actor(etab, cast.parent_lea.user_id, "parent", a, m.periode).await.unwrap();
    let rm = rel.matieres.iter().find(|x| x.matiere_id == m.maths).unwrap();
    assert_eq!(rm.moyenne, Some(exacte), "relevé et lecture historique : la même moyenne");
    assert_eq!(exacte, 10.02);
}

// ─── D11 : mentions « mise en garde » ────────────────────────────────────────

#[tokio::test]
async fn les_anciennes_mentions_avertissement_deviennent_des_mises_en_garde() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, dir, a) = (m.etab, cast.direction.user_id, m.eleves[0]);
    let conseil = ent.creer_conseil_eva_for_actor(etab, dir, "direction", &CreerConseilBody {
        classe_id: m.classe, periode_id: m.periode, jour: today_utc(),
        heure: chrono::NaiveTime::from_hms_opt(17, 0, 0).unwrap(), lieu: "Salle 3".into(),
    }).await.unwrap();
    let decision = |mention: &str| DecisionConseilBody {
        eleve_id: a, appreciation: "Des efforts à poursuivre.".into(), mention: Some(mention.into()),
        decision_code: None, avis_orientation: None,
    };
    ent.decider_conseil_eva_for_actor(etab, dir, "direction", conseil.id, &decision("avertissement_travail")).await.unwrap();
    let stockee = |ent: &PgEnt| {
        let q = sqlx::query("SELECT mention FROM ent_conseil_classe_dossier WHERE etablissement_id = $1 AND conseil_id = $2 AND eleve_id = $3")
            .bind(etab).bind(conseil.id).bind(a);
        let pool = ent.pool().clone();
        async move { q.fetch_one(&pool).await.unwrap().get::<Option<String>, _>(0) }
    };
    assert_eq!(stockee(&ent).await.as_deref(), Some("mise_en_garde_travail"));
    ent.decider_conseil_eva_for_actor(etab, dir, "direction", conseil.id, &decision("mise_en_garde_conduite")).await.unwrap();
    assert_eq!(stockee(&ent).await.as_deref(), Some("mise_en_garde_conduite"));
    // La base n'accepte plus le code « avertissement » (sanction disciplinaire, R. 511-13).
    let e = sqlx::query("UPDATE ent_conseil_classe_dossier SET mention = 'avertissement_travail' WHERE etablissement_id = $1")
        .bind(etab).execute(ent.pool()).await;
    assert!(e.is_err());
}

// ─── D14 : un seul droit d'export officiel ───────────────────────────────────

#[tokio::test]
async fn export_livret_reserve_aux_administratifs_avec_mention_non_officielle() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let etab = m.etab;
    for (u, role) in [(cast.prof.user_id, "prof"), (cast.parent_lea.user_id, "parent"), (cast.cpe.user_id, "cpe")] {
        assert!(matches!(
            ent.exporter_livret_for_actor(etab, u, role, "lsu", m.classe, m.periode).await,
            Err(EntDbError::Forbidden)
        ), "{role} n'exporte jamais de livret");
    }
    let ok = ent
        .exporter_livret_for_actor(etab, cast.administratif.user_id, "administratif", "lsu", m.classe, m.periode)
        .await
        .unwrap();
    assert!(ok.avertissement.contains("NON OFFICIEL"));
}

// ─── D9 : RLS sur les tables d'évaluation (isolation entre établissements) ───

#[tokio::test]
async fn rls_sur_les_tables_d_evaluation_isole_les_etablissements() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let m = monde(&ent, &cast).await;
    let (etab, prof, a) = (m.etab, cast.prof.user_id, m.eleves[0]);
    ent.saisir_appreciation_eva_for_actor(etab, prof, "prof", &AppreciationBody {
        eleve_id: a, periode_id: m.periode, matiere_id: Some(m.maths), texte: "RLS".into(),
    }).await.unwrap();
    let d = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "C", 1.0, 20.0)).await.unwrap();
    ent.saisir_grille_for_actor(etab, prof, "prof", d.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(9.0))] }).await.unwrap();

    // Rôle applicatif non propriétaire, sans BYPASSRLS, en mode strict.
    const ROLE: &str = "ent_eva_v0_app";
    for sql in [
        format!("DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; EXCEPTION WHEN duplicate_object THEN NULL; END $$"),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT ON ent_note, ent_note_appreciation, ent_devoir, ent_note_periode, ent_bulletin, ent_competence_evaluation, ent_conseil_classe_dossier TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(ent.pool()).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();
    for table in ["ent_note", "ent_note_appreciation", "ent_devoir", "ent_note_periode"] {
        let compter = |etab_pose: Option<Uuid>| {
            let app = app.clone();
            async move {
                let mut tx = app.begin().await.unwrap();
                if let Some(e) = etab_pose {
                    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)").bind(e.to_string()).execute(&mut *tx).await.unwrap();
                }
                let n: i64 = sqlx::query(&format!("SELECT count(*) FROM {table}")).fetch_one(&mut *tx).await.unwrap().get(0);
                n
            }
        };
        let (propres, etranger, sans) = (
            compter(Some(etab)).await,
            compter(Some(Uuid::new_v4())).await,
            compter(None).await,
        );
        assert!(propres >= 1, "{table} : l'établissement voit ses lignes");
        assert_eq!(etranger, 0, "{table} : un autre établissement ne voit rien");
        assert_eq!(sans, 0, "{table} : mode strict sans établissement posé : aucune ligne");
    }
}
