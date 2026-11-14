//! Socle 20, lot C3 : agenda, cahier de l'élève et de la famille, « ce que j'ai manqué », coche, résumé hebdomadaire,
//! abonnement au calendrier, contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`). Aucun résultat ne dépend de l'heure réelle : le « jour » est celui de
//! l'établissement et les échéances s'en déduisent ; la visibilité des publications se force par SQL ; les cas de
//! lien (séparés, suspendu, sans exercice, majeur, parti) se posent par SQL puis se rétablissent. Chaque test ne
//! vérifie que ses propres lignes.

use super::*;
use crate::cahier_c3::{decouper_jeton, lundi_de};
use chrono::DateTime;
use ent_types::cahier_c2::*;
use ent_types::cahier_c3::*;

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

fn corps_travail(cours: Vec<Uuid>, jour_echeance: NaiveDate) -> TravailC2Body {
    TravailC2Body {
        cours_ids: cours,
        seance_donnee_id: None,
        echeance_seance_id: None,
        echeance_prochaine_seance: false,
        echeance_jour: Some(jour_echeance),
        echeance_heure: NaiveTime::from_hms_opt(8, 0, 0),
        nature: "exercices".into(),
        titre: format!("Exercices {}", &Uuid::new_v4().to_string()[..12]),
        consigne: "p. 12 n° 3 à 5".into(),
        duree_estimee_min: Some(30),
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

async fn cours_de_service(fx: &Fx, service: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
        .bind(service)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Rend échues les publications de l'établissement (l'heure réelle du test est quelconque).
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

/// Premier jour ouvré (ni week-end, ni vacances, ni férié de l'établissement) à partir de `jour` : le serveur refuse une
/// échéance un jour fermé, et le jour réel du test est quelconque.
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

/// Crée un travail pour la classe A, visible, échéance le premier jour ouvré à partir du jour donné ; renvoie
/// (travail, cours, jour d'échéance).
async fn travail_a(fx: &Fx, a_partir_de: NaiveDate) -> (Uuid, Uuid, NaiveDate) {
    let cours = cours_de_service(fx, fx.service_a).await;
    let jour_echeance = ouvre(fx, a_partir_de).await;
    let cree = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(vec![cours], jour_echeance)).await.unwrap();
    rendre_visible(fx).await;
    (cree.travaux[0].id, cours, jour_echeance)
}

async fn agenda(fx: &Fx, a: &Acteur<'_>, eleve: Option<Uuid>, du: NaiveDate, au: NaiveDate) -> Result<AgendaDto, EntDbError> {
    fx.ent.cdt_agenda(a, eleve, du, au).await
}

fn contient(a: &AgendaDto, id: Uuid) -> bool {
    a.lignes.iter().any(|l| l.id == id)
}

/// Lien d'un compte parent à un élève (sert aux « parents séparés »).
async fn lier(fx: &Fx, parent: Uuid, eleve: Uuid) {
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, lien, autorite_parentale)
         VALUES ($1, $2, $3, $4, 'pere', true)",
    )
    .bind(Uuid::new_v4())
    .bind(fx.cast.etablissement_id)
    .bind(eleve)
    .bind(parent)
    .execute(fx.ent.pool())
    .await
    .unwrap();
}

async fn sql(fx: &Fx, requete: &str, eleve: Uuid, parent: Uuid) {
    sqlx::query(requete)
        .bind(fx.cast.etablissement_id)
        .bind(eleve)
        .bind(parent)
        .execute(fx.ent.pool())
        .await
        .unwrap();
}

// ───────────────────────── Agenda, coche, statut, évaluations ─────────────────────────

#[tokio::test]
async fn c3_agenda_de_l_eleve_coche_personnelle_et_famille_sans_coche() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let auj = jour(&fx).await;
    let (du, au) = (auj - Duration::days(1), auj + Duration::days(20));
    let (t, cours, echeance) = travail_a(&fx, auj + Duration::days(3)).await;

    let vu = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap();
    let l = vu.lignes.iter().find(|l| l.id == t).expect("le travail est dans l'agenda de l'élève");
    assert_eq!((l.genre.as_str(), l.jour, l.heure.as_deref()), ("travail", echeance, Some("08:00")));
    assert_eq!(l.cours_id, Some(cours));
    assert_eq!((l.nature.as_deref(), l.duree_estimee_min), (Some("Exercices"), Some(30)));
    assert!(l.consigne.as_deref().is_some_and(|c| c.contains("p. 12")));
    assert_eq!(l.fait, Some(false), "coche personnelle servie à l'élève");
    assert_eq!(vu.vue, "eleve");
    assert!(!l.auteur.clone().unwrap().is_empty());
    assert!(!l.auteur.clone().unwrap().contains('@'), "nom d'affichage, jamais l'adresse ni l'identifiant de compte");
    assert!(l.statut.is_none(), "aucune remise attendue : pas de statut");

    // Coche, puis décoche : outil personnel, sans effet sur ce que voit la famille.
    fx.ent.cdt_cocher(&fx.eleve(&fx.x), t, true).await.unwrap();
    fx.ent.cdt_cocher(&fx.eleve(&fx.x), t, true).await.unwrap(); // idempotent
    let l = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap().lignes.into_iter().find(|l| l.id == t).unwrap();
    assert_eq!(l.fait, Some(true));
    let famille = agenda(&fx, &fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap();
    assert_eq!(famille.vue, "famille");
    let lf = famille.lignes.iter().find(|l| l.id == t).expect("la famille lit l'agenda");
    assert_eq!(lf.fait, None, "R-C14 : la coche n'est jamais servie à un adulte");
    assert_eq!(lf.consigne, l.consigne);
    fx.ent.cdt_cocher(&fx.eleve(&fx.x), t, false).await.unwrap();
    let l = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap().lignes.into_iter().find(|l| l.id == t).unwrap();
    assert_eq!(l.fait, Some(false));

    // Seul l'élève coche : ni un responsable, ni un professeur ; et pas un travail qui n'est pas le sien.
    assert!(est_interdit(&fx.ent.cdt_cocher(&fx.parent(&fx.x), t, true).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_cocher(&fx.prof(), t, true).await.unwrap_err()));
    assert!(est_introuvable(&fx.ent.cdt_cocher(&fx.eleve(&fx.y), t, true).await.unwrap_err()), "classe B : le travail n'existe pas");
    assert!(est_introuvable(&fx.ent.cdt_cocher(&fx.eleve(&fx.x), Uuid::new_v4(), true).await.unwrap_err()));

    // L'élève de l'autre classe n'en voit rien ; hors période : rien.
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.y), None, du, au).await.unwrap(), t));
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.x), None, du, echeance - Duration::days(1)).await.unwrap(), t));
    // Aucune trace de lecture nominative n'est écrite (R-C42).
    let vu_le: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_travail_eleve WHERE etablissement_id = $1 AND vu_le IS NOT NULL")
        .bind(fx.cast.etablissement_id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(vu_le, 0);
}

#[tokio::test]
async fn c3_brouillon_programme_retire_et_public_restreint_invisibles() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let auj = jour(&fx).await;
    let (du, au) = (auj, auj + Duration::days(20));
    let cours = cours_de_service(&fx, fx.service_a).await;
    // Brouillon : jamais visible.
    let mut b = corps_travail(vec![cours], ouvre(&fx, auj + Duration::days(4)).await);
    b.publication = "brouillon".into();
    let brouillon = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id;
    // Public restreint à X : W (même classe) ne le voit pas.
    let mut r = corps_travail(vec![cours], ouvre(&fx, auj + Duration::days(5)).await);
    r.eleve_ids = vec![fx.x.id];
    let restreint = fx.ent.cdt_travaux_creer(&fx.prof(), r).await.unwrap().travaux[0].id;
    // Travail retiré.
    let retire = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(vec![cours], ouvre(&fx, auj + Duration::days(6)).await)).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;
    let avant = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(contient(&avant, retire) && contient(&avant, restreint) && !contient(&avant, brouillon));
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.w), None, du, au).await.unwrap(), restreint), "public restreint : W ne le voit pas");
    fx.ent.cdt_travail_retirer(&fx.prof(), retire, "erreur de saisie").await.unwrap();
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap(), retire), "un travail retiré disparaît de l'agenda");
    // Programmé pour plus tard : invisible tant que `visible_a` n'est pas échu.
    let prog = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(vec![cours], ouvre(&fx, auj + Duration::days(7)).await)).await.unwrap().travaux[0].id;
    sqlx::query("UPDATE ent_cdt_travail SET visible_a = now() + interval '2 days' WHERE etablissement_id = $1 AND id = $2")
        .bind(fx.cast.etablissement_id)
        .bind(prog)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap(), prog));
}

#[tokio::test]
async fn c3_statut_constate_prolongation_individuelle_et_modification_visible() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let auj = jour(&fx).await;
    let (du, au) = (auj - Duration::days(10), auj + Duration::days(20));
    let cours = cours_de_service(&fx, fx.service_a).await;
    let mut b = corps_travail(vec![cours], ouvre(&fx, auj + Duration::days(2)).await);
    b.rendu = "en_ligne".into();
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;
    let statut = |a: AgendaDto| a.lignes.into_iter().find(|l| l.id == t).and_then(|l| l.statut);
    assert_eq!(statut(agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap()).as_deref(), Some("a_faire"));
    // L'échéance passée sans dépôt : « non rendu » (un fait, jamais un zéro).
    sqlx::query("UPDATE ent_cdt_travail SET echeance_le = now() - interval '2 days' WHERE etablissement_id = $1 AND id = $2")
        .bind(fx.cast.etablissement_id)
        .bind(t)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(statut(agenda(&fx, &fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap()).as_deref(), Some("non_rendu"));
    // Prolongation individuelle : l'échéance effective de X change, pas celle de W ; le motif n'est jamais servi.
    sqlx::query(
        "INSERT INTO ent_cdt_echeance_eleve (id, etablissement_id, travail_id, eleve_id, echeance_le, motif_code)
         VALUES ($1, $2, $3, $4, now() + interval '3 days', 'amenagement')",
    )
    .bind(Uuid::new_v4())
    .bind(fx.cast.etablissement_id)
    .bind(t)
    .bind(fx.x.id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    let lx = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap().lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(lx.echeance_prolongee);
    assert_eq!(lx.statut.as_deref(), Some("a_faire"));
    let lw = agenda(&fx, &fx.eleve(&fx.w), None, du, au).await.unwrap().lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(!lw.echeance_prolongee && lw.jour < lx.jour);
    let json = serde_json::to_string(&lx).unwrap();
    assert!(!json.contains("amenagement"), "le motif d'une prolongation n'est jamais servi : {json}");
    // Constat du professeur : le plus récent fait foi.
    for s in ["rendu_papier", "excuse"] {
        sqlx::query("INSERT INTO ent_cdt_statut (id, etablissement_id, travail_id, eleve_id, statut) VALUES ($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4())
            .bind(fx.cast.etablissement_id)
            .bind(t)
            .bind(fx.w.id)
            .bind(s)
            .execute(fx.ent.pool())
            .await
            .unwrap();
    }
    assert_eq!(statut(agenda(&fx, &fx.eleve(&fx.w), None, du, au).await.unwrap()).as_deref(), Some("excuse"));
    // Modification : « modifié le » est lisible et la version est servie (R-C29, jamais caché).
    let avant = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap().lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(avant.modifie_le.is_none() && avant.version == 1);
    let mut m = ModifierTravailC2Body::default();
    m.consigne = Some("Consigne précisée".into());
    m.sans_avis = true;
    fx.ent.cdt_travail_modifier(&fx.prof(), t, m).await.unwrap();
    rendre_visible(&fx).await;
    let apres = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap().lignes.into_iter().find(|l| l.id == t).unwrap();
    assert!(apres.modifie_le.is_some() && apres.version > avant.version, "une modification sans avis reste visible : {apres:?}");
}

#[tokio::test]
async fn c3_evaluation_annoncee_port_du_21_sorties_et_comptage_unique() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let (du, au) = (auj, auj + Duration::days(20));
    // Devoir de l'existant (socle 21 minimal) pour la classe A, dans 4 jours.
    let periode = Uuid::new_v4();
    let code = Uuid::new_v4().simple().to_string();
    sqlx::query("INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin) VALUES ($1,$2,$3,'Période C3',$4,$5)")
        .bind(periode)
        .bind(e)
        .bind(&code[..12])
        .bind(auj - Duration::days(60))
        .bind(auj + Duration::days(60))
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let matiere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1,$2,$3,'Histoire')")
        .bind(matiere)
        .bind(e)
        .bind(&code[12..24])
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let devoir = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, classe_id, titre, date_devoir, coef, sur)
         VALUES ($1,$2,$3,$4,'CDT-A',$5,'Contrôle chapitre 4',$6, 3, 20)",
    )
    .bind(devoir)
    .bind(e)
    .bind(periode)
    .bind(matiere)
    .bind(fx.classe_a)
    .bind(auj + Duration::days(4))
    .execute(fx.ent.pool())
    .await
    .unwrap();
    let x = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap();
    let l = x.lignes.iter().find(|l| l.id == devoir).expect("évaluation annoncée dans l'agenda");
    assert_eq!((l.genre.as_str(), l.matiere.as_str(), l.titre.as_str()), ("evaluation", "Histoire", "Contrôle chapitre 4"));
    assert!(l.consigne.is_none() && l.fait.is_none() && l.statut.is_none());
    let json = serde_json::to_string(l).unwrap();
    assert!(!json.contains("coef") && !json.contains("\"sur\""), "jamais de coefficient ni de barème : {json}");
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.y), None, du, au).await.unwrap(), devoir), "classe B : rien");
    // Le travail qui annonce la même évaluation n'est pas compté deux fois (CC-55).
    let (t, _, _) = travail_a(&fx, auj + Duration::days(4)).await;
    sqlx::query("UPDATE ent_cdt_travail SET evaluation_id = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(e)
        .bind(t)
        .bind(devoir)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let x = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(contient(&x, t) && !contient(&x, devoir));
    assert_eq!(x.lignes.iter().filter(|l| l.genre == "evaluation").count(), 1);
    // Sortie : participant inscrit d'une sortie ouverte, titre seulement.
    let sortie = Uuid::new_v4();
    let annee = fx.ent.referentiel_annee_courante_assuree(e).await.unwrap();
    let ref_sortie = format!("SC3-{}", &code[..10]);
    let porteur: Uuid = sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 ORDER BY id LIMIT 1")
        .bind(e)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_sortie (id, etablissement_id, annee_id, reference, titre, nature, forme, destination, debut, fin, etat, porteur_personne_id)
         VALUES ($1,$2,$3,$4,'Musée de la ville','obligatoire','sans_nuitee','proximite',$5,$6,'ouverte',$7)",
    )
    .bind(sortie)
    .bind(e)
    .bind(annee.id)
    .bind(&ref_sortie)
    .bind(ent_instant(&fx, auj + Duration::days(5), "09:00").await)
    .bind(ent_instant(&fx, auj + Duration::days(5), "16:00").await)
    .bind(porteur)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_sortie_participant (id, etablissement_id, sortie_id, eleve_id, etat) VALUES ($1,$2,$3,$4,'inscrit')")
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(sortie)
        .bind(fx.x.id)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let x = agenda(&fx, &fx.eleve(&fx.x), None, du, au).await.unwrap();
    let s = x.lignes.iter().find(|l| l.id == sortie).expect("la sortie est dans l'agenda");
    assert_eq!((s.genre.as_str(), s.jour, s.heure.as_deref()), ("sortie", auj + Duration::days(5), Some("09:00")));
    assert!(!contient(&agenda(&fx, &fx.eleve(&fx.w), None, du, au).await.unwrap(), sortie), "W n'est pas inscrit");
}

async fn ent_instant(fx: &Fx, jour: NaiveDate, heure: &str) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT ent_cdt_instant_local($1, $2, $3::time)")
        .bind(fx.cast.etablissement_id)
        .bind(jour)
        .bind(heure)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

// ───────────────────────── Matrice de séparation de la famille ─────────────────────────

#[tokio::test]
async fn c3_matrice_de_separation_famille_separes_suspendu_sans_exercice_majeur_parti() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let (du, au) = (auj, auj + Duration::days(10));
    let (t, _, _) = travail_a(&fx, auj + Duration::days(2)).await;
    let px = fx.parent(&fx.x);
    macro_rules! lit {
        ($a:expr, $e:expr) => {
            agenda(&fx, &$a, $e, du, au)
        };
    }
    // Responsable habilité : lit l'agenda, le résumé et « ce que j'ai manqué » de son enfant.
    assert!(contient(&lit!(fx.parent(&fx.x), Some(fx.x.id)).await.unwrap(), t));
    fx.ent.cdt_resume_semaine(&fx.parent(&fx.x), Some(fx.x.id), None).await.unwrap();
    fx.ent.cdt_manque(&fx.parent(&fx.x), Some(fx.x.id), auj - Duration::days(5), auj).await.unwrap();
    // L'enfant n'est pas désigné : refus de saisie ; l'enfant d'un autre : interdit.
    assert!(est_refus(&lit!(fx.parent(&fx.x), None).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_interdit(&lit!(fx.parent(&fx.y), Some(fx.x.id)).await.unwrap_err()), "le parent de Y ne lit pas X");
    assert!(est_interdit(&lit!(fx.parent(&fx.x), Some(fx.y.id)).await.unwrap_err()), "le parent de X ne lit pas Y");
    // Élève : le sien seulement.
    assert!(est_interdit(&lit!(fx.eleve(&fx.x), Some(fx.w.id)).await.unwrap_err()));
    // Personnel (professeur, direction, CPE, comptabilité) : refus, la lecture passe par les routes du lot C2.
    for a in [fx.prof(), fx.direction(), fx.cpe(), fx.compta()] {
        assert!(est_interdit(&lit!(a, Some(fx.x.id)).await.unwrap_err()));
    }

    // Sans exercice de l'autorité parentale : refus ; rétabli : accès.
    sql(&fx, "UPDATE ent_eleve_lien SET autorite_parentale = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3", fx.x.id, px.user_id).await;
    assert!(est_interdit(&lit!(fx.parent(&fx.x), Some(fx.x.id)).await.unwrap_err()), "sans exercice de l'autorité parentale");
    sql(&fx, "UPDATE ent_eleve_lien SET autorite_parentale = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3", fx.x.id, px.user_id).await;
    // Lien suspendu : refus.
    sql(&fx, "UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'suspension de test' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3", fx.x.id, px.user_id).await;
    assert!(est_interdit(&lit!(fx.parent(&fx.x), Some(fx.x.id)).await.unwrap_err()), "lien suspendu");
    sql(&fx, "UPDATE ent_eleve_lien SET suspendu_le = NULL, suspension_motif = NULL WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3", fx.x.id, px.user_id).await;
    assert!(contient(&lit!(fx.parent(&fx.x), Some(fx.x.id)).await.unwrap(), t));

    // Parents séparés : un second responsable (celui de Y) est lié à X ; une interdiction de communication visant le
    // premier le ferme, le second garde son accès. L'élève lit toujours son propre agenda.
    lier(&fx, fx.y.parent.user_id, fx.x.id).await;
    assert!(contient(&lit!(fx.parent(&fx.y), Some(fx.x.id)).await.unwrap(), t), "second responsable habilité");
    let personne: Uuid = sqlx::query_scalar("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(e)
        .bind(fx.x.id)
        .bind(px.user_id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4,ARRAY['communication'],'decision_justice','ne pas communiquer',$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(fx.x.id)
    .bind(personne)
    .bind(auj - Duration::days(1))
    .bind(fx.cast.direction.user_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    assert!(est_interdit(&lit!(fx.parent(&fx.x), Some(fx.x.id)).await.unwrap_err()), "parent visé par une interdiction de communication");
    assert!(contient(&lit!(fx.parent(&fx.y), Some(fx.x.id)).await.unwrap(), t), "l'autre parent n'est pas visé");
    assert!(contient(&lit!(fx.eleve(&fx.x), None).await.unwrap(), t));
    let ab = fx.ent.cdt_abonnement_creer(&fx.parent(&fx.y), Some(fx.x.id)).await;
    assert!(ab.is_ok());
    assert!(est_interdit(&fx.ent.cdt_abonnement_creer(&fx.parent(&fx.x), Some(fx.x.id)).await.unwrap_err()));

    // Élève majeur : les responsables sont fermés, l'élève lit.
    {
        // L'identité n'est pas modifiable à la main : le procédé de test des autres lots (projection locale à la transaction).
        let mut tx = fx.ent.pool().begin().await.unwrap();
        sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)").execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE ent_eleve SET date_naissance = (ent_aujourdhui_etab(etablissement_id) - INTERVAL '19 years')::date WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(fx.w.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert!(est_interdit(&lit!(fx.parent(&fx.w), Some(fx.w.id)).await.unwrap_err()), "élève majeur : responsable fermé");
    assert!(lit!(fx.eleve(&fx.w), None).await.is_ok(), "l'élève majeur lit son agenda");
    // Élève parti : ni lui, ni sa famille (C-D5).
    sqlx::query("UPDATE ent_eleve SET date_sortie = ent_aujourdhui_etab($1) - 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(e)
        .bind(fx.z.id)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert!(est_interdit(&lit!(fx.eleve(&fx.z), None).await.unwrap_err()));
    assert!(est_interdit(&lit!(fx.parent(&fx.z), Some(fx.z.id)).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_cocher(&fx.eleve(&fx.z), t, true).await.unwrap_err()));
}

// ───────────────────────── Ce que j'ai manqué ─────────────────────────

#[tokio::test]
async fn c3_ce_que_j_ai_manque_seances_absentes_contenu_publie_et_travail_donne() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let (du, au) = (auj - Duration::days(6), auj);
    let prof0 = fx.cast.profs[0].user_id;
    let cours = cours_de_service(&fx, fx.service_a).await;
    // Les affectations du labo commencent aujourd'hui : l'historique de X remonte avant les séances manquées.
    sqlx::query("UPDATE ent_affectation SET date_debut = $3 WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(e)
        .bind(fx.x.id)
        .bind(auj - Duration::days(60))
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let manquee = fx.seance("CDT-A", prof0, Some(fx.service_a), auj - Duration::days(2)).await;
    let autre = fx.seance("CDT-A", prof0, Some(fx.service_a), auj - Duration::days(3)).await;
    let sans_contenu = fx.seance("CDT-A", prof0, Some(fx.service_a), auj - Duration::days(4)).await;
    let mut entree = EntreeBody {
        titre: Some("Fractions".into()),
        contenu: "Définition et exemples".into(),
        notions: vec!["Fraction".into(), "Numérateur".into()],
        publication: "maintenant".into(),
        publie_a: None,
        publie_jour: None,
        publie_heure: None,
        sans_avis: false,
        motif: None,
    };
    fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, manquee, entree.clone()).await.unwrap();
    entree.contenu = "Séance où X était présent".into();
    fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, autre, entree).await.unwrap();
    let echeance_manque = ouvre(&fx, auj + Duration::days(5)).await;
    let mut b = corps_travail(vec![cours], echeance_manque);
    b.seance_donnee_id = Some(manquee);
    let travail = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;
    // Absences de X : à la séance `manquee` (par le créneau) et à `sans_contenu` ; pas à `autre`.
    for s in [manquee, sans_contenu] {
        let (creneau, jour_s): (Uuid, NaiveDate) = sqlx::query_as("SELECT creneau_id, jour FROM ent_edt_seance WHERE id = $1")
            .bind(s)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
        let absence = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification, source) VALUES ($1,$2,$3,$4,$4,'a_qualifier','en_attente','reprise')")
            .bind(absence)
            .bind(e)
            .bind(fx.x.id)
            .bind(jour_s)
            .execute(fx.ent.pool())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, seance_ref, heure_debut, heure_fin, source)
             VALUES ($1,$2,$3,$4,$5,'seance',$6,'08:00','09:00','reprise')",
        )
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(absence)
        .bind(fx.x.id)
        .bind(jour_s)
        .bind(creneau)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    }
    let m = fx.ent.cdt_manque(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert_eq!(m.seances.len(), 1, "seule la séance manquée qui a du contenu publié : {:?}", m.seances.iter().map(|s| s.jour).collect::<Vec<_>>());
    let s = &m.seances[0];
    assert_eq!(s.seance_id, manquee);
    assert_eq!(s.contenu.as_deref(), Some("Définition et exemples"));
    assert_eq!(s.notions, vec!["Fraction".to_string(), "Numérateur".to_string()]);
    assert_eq!((s.heure_debut.as_str(), s.heure_fin.as_str()), ("08:00", "09:00"));
    assert_eq!(s.travaux.len(), 1);
    assert_eq!((s.travaux[0].id, s.travaux[0].echeance_jour), (travail, echeance_manque));
    // La famille lit la même chose ; W (présent) n'a rien manqué ; un élève ne lit pas celui d'un autre.
    assert_eq!(fx.ent.cdt_manque(&fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap().seances.len(), 1);
    assert!(fx.ent.cdt_manque(&fx.eleve(&fx.w), None, du, au).await.unwrap().seances.is_empty());
    assert!(est_interdit(&fx.ent.cdt_manque(&fx.eleve(&fx.w), Some(fx.x.id), du, au).await.unwrap_err()));
    // Une absence retirée (correction) ne compte plus ; une entrée retirée disparaît.
    sqlx::query("UPDATE ent_asd_absence_seance SET retiree_le = now(), retiree_cause = 'correction' WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3")
        .bind(e)
        .bind(fx.x.id)
        .bind(auj - Duration::days(2))
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert!(fx.ent.cdt_manque(&fx.eleve(&fx.x), None, du, au).await.unwrap().seances.is_empty());
}

// ───────────────────────── Résumé hebdomadaire et avis ─────────────────────────

#[tokio::test]
async fn c3_resume_hebdomadaire_decomptes_envoi_unique_et_avis_sans_contenu() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let lundi = lundi_de(auj) + Duration::days(7);
    let (t, _, _) = travail_a(&fx, lundi + Duration::days(2)).await;
    let r = fx.ent.cdt_resume_semaine(&fx.parent(&fx.x), Some(fx.x.id), Some(lundi + Duration::days(3))).await.unwrap();
    assert_eq!((r.semaine_debut, r.semaine_fin), (lundi, lundi + Duration::days(6)));
    assert!(r.lignes.iter().any(|l| l.id == t) && r.nb_travaux >= 1);
    assert!(r.duree_estimee_min >= 30);
    assert!(r.lignes.iter().all(|l| l.fait.is_none()), "jamais de coche dans le résumé d'une famille");
    let re = fx.ent.cdt_resume_semaine(&fx.eleve(&fx.x), None, Some(lundi)).await.unwrap();
    assert!(re.lignes.iter().all(|l| l.fait.is_some() || l.genre != "travail"));
    // Envoi : un seul avis par élève et par semaine, même rejoué ; décomptes seulement ; pas pour un lundi faux.
    assert!(est_refus(&fx.ent.cdt_resumes_preparer(e, lundi + Duration::days(1)).await.unwrap_err(), RefusMetier::Invalide));
    let n1 = fx.ent.cdt_resumes_preparer(e, lundi).await.unwrap();
    assert!(n1 >= 1);
    let n2 = fx.ent.cdt_resumes_preparer(e, lundi).await.unwrap();
    assert_eq!(n2, 0, "rejouable sans doublon");
    let evts: Vec<(Uuid, String)> = sqlx::query_as("SELECT o.aggregate_id, m.payload::text FROM ent_outbox o JOIN ent_metier_event m ON m.id = o.event_id WHERE o.etablissement_id = $1 AND o.type = 'cahier.resume_hebdomadaire'")
        .bind(e)
        .fetch_all(fx.ent.pool())
        .await
        .unwrap();
    let (_, payload) = evts.iter().find(|(a, _)| *a == fx.x.id).expect("un avis pour X");
    assert_eq!(evts.iter().filter(|(a, _)| *a == fx.x.id).count(), 1);
    for interdit in ["consigne", "titre", "p. 12", "note", "@"] {
        assert!(!payload.contains(interdit), "l'avis ne porte que des décomptes : {payload}");
    }
    assert!(!evts.iter().any(|(a, _)| *a == fx.y.id), "Y n'a rien cette semaine-là");
    let envoi: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_resume_envoi WHERE etablissement_id = $1 AND semaine_debut = $2")
        .bind(e)
        .bind(lundi)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(envoi as usize, n1);
}

// ───────────────────────── Abonnement au calendrier ─────────────────────────

#[tokio::test]
async fn c3_abonnement_calendrier_jeton_hache_revocable_et_droits_relus() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let (t, _, _) = travail_a(&fx, auj + Duration::days(3)).await;
    let cree = fx.ent.cdt_abonnement_creer(&fx.eleve(&fx.x), None).await.unwrap();
    let jeton = cree.chemin.strip_prefix("/cahier/c3/calendrier/").unwrap().to_string();
    let (etab, secret) = decouper_jeton(&jeton).expect("jeton bien formé");
    assert_eq!(etab, e);
    // Seule l'empreinte est stockée : jamais le secret.
    let stocke: String = sqlx::query_scalar("SELECT jeton_hash FROM ent_cdt_abonnement WHERE etablissement_id = $1 AND id = $2")
        .bind(e)
        .bind(cree.abonnement.id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_ne!(stocke, secret);
    assert_eq!(stocke.len(), 64);
    // Flux : la ligne du travail, sans consigne ni auteur ; jeton inconnu ou mal formé : introuvable.
    let (_, lignes) = fx.ent.cdt_calendrier_par_jeton(&jeton).await.unwrap();
    let l = lignes.iter().find(|l| l.id == t).expect("travail dans le flux");
    assert!(l.consigne.is_none() && l.auteur.is_none() && l.fait.is_none());
    for faux in [
        format!("{}-{}", e.simple(), "0".repeat(64)),
        format!("{}-{}", Uuid::new_v4().simple(), secret),
        "n-importe-quoi".to_string(),
    ] {
        assert!(est_introuvable(&fx.ent.cdt_calendrier_par_jeton(&faux).await.unwrap_err()), "{faux}");
    }
    // Liste : sans jeton ; un autre compte ne voit ni ne révoque l'abonnement.
    let liste = fx.ent.cdt_abonnements(&fx.eleve(&fx.x)).await.unwrap();
    assert!(liste.iter().any(|a| a.id == cree.abonnement.id));
    assert!(fx.ent.cdt_abonnements(&fx.eleve(&fx.w)).await.unwrap().is_empty());
    assert!(est_introuvable(&fx.ent.cdt_abonnement_revoquer(&fx.eleve(&fx.w), cree.abonnement.id).await.unwrap_err()));
    // Un responsable perd son droit : le flux ne donne plus rien (droits relus à chaque lecture).
    let cp = fx.ent.cdt_abonnement_creer(&fx.parent(&fx.x), Some(fx.x.id)).await.unwrap();
    let jp = cp.chemin.strip_prefix("/cahier/c3/calendrier/").unwrap().to_string();
    assert!(fx.ent.cdt_calendrier_par_jeton(&jp).await.is_ok());
    sql(&fx, "UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'suspension de test' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3", fx.x.id, fx.parent(&fx.x).user_id).await;
    assert!(est_introuvable(&fx.ent.cdt_calendrier_par_jeton(&jp).await.unwrap_err()));
    // Révocation : le flux cesse aussitôt.
    fx.ent.cdt_abonnement_revoquer(&fx.eleve(&fx.x), cree.abonnement.id).await.unwrap();
    assert!(est_introuvable(&fx.ent.cdt_calendrier_par_jeton(&jeton).await.unwrap_err()));
    assert!(fx.ent.cdt_abonnements(&fx.eleve(&fx.x)).await.unwrap().iter().all(|a| a.id != cree.abonnement.id));
    // Plafond d'abonnements actifs.
    for _ in 0..5 {
        fx.ent.cdt_abonnement_creer(&fx.eleve(&fx.x), None).await.unwrap();
    }
    assert!(est_refus(&fx.ent.cdt_abonnement_creer(&fx.eleve(&fx.x), None).await.unwrap_err(), RefusMetier::Conflit));
}

// ───────────────────────── Isolation entre établissements ─────────────────────────

#[tokio::test]
async fn c3_un_autre_etablissement_ne_lit_rien() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let auj = jour(&fx).await;
    let (t, _, _) = travail_a(&fx, auj + Duration::days(3)).await;
    // Un second établissement (autre jeu de labo) : son élève n'a ni ce travail ni cet élève.
    let autre = fx.ent.seed_labo_cast().await.unwrap();
    let a = Acteur {
        etablissement_id: autre.etablissement_id,
        user_id: autre.eleves[0].compte.as_ref().unwrap().user_id,
        role: Role::Eleve.as_str(),
    };
    let r = fx.ent.cdt_agenda(&a, None, auj, auj + Duration::days(10)).await.unwrap();
    assert!(!contient(&r, t));
    assert!(est_introuvable(&fx.ent.cdt_cocher(&a, t, true).await.unwrap_err()));
    // Un acteur de B qui désigne un élève de A : jamais lisible (le responsable de B n'a aucun lien dans A).
    let p = Acteur {
        etablissement_id: autre.etablissement_id,
        user_id: autre.eleves[0].parent.user_id,
        role: Role::Parent.as_str(),
    };
    assert!(est_interdit(&fx.ent.cdt_agenda(&p, Some(fx.x.id), auj, auj + Duration::days(10)).await.unwrap_err()));
}
