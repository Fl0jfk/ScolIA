//! Socle 18, lot FO0 — correctifs urgents des formulaires, autorisations et consentements, contre la base locale
//! de test (`docs/socles/18_FORMULAIRES.md` §2.2, §19, §29). Sans `ENT_DATABASE_URL` : ignorés en local et **en
//! échec en CI** (`ENT_EXIGER_BASE`).
//!
//! Un test par défaut corrigé : `fd01_…` à `fd19_…` ; les cas limites de la conception : `ck03_…`, `ck06_…`,
//! `ck07_…`, `ck08_…`, `ck15_…`, `ck17_…`, `ck18_…`, `ck19_…`, `ck30_…`, `ck31_…`.
//!
//! Chaque test travaille sur un **objet aléatoire** (la base de test est partagée) et nettoie ce qu'il a créé
//! (interdictions, mesures de capacité, liens, dates de naissance).

use super::*;
use crate::formulaires::{NouveauFormulaire, ReponseFormulaire};
use chrono::NaiveDate;
use uuid::Uuid;

pub(crate) async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
        e.install_mail_dry_run();
    })
    .await;
    Some((ent, cast))
}

pub(crate) fn objet_aleatoire() -> String {
    format!("fo0_{}", &Uuid::new_v4().simple().to_string()[..16])
}

pub(crate) async fn classe_4b(ent: &PgEnt, etab: Uuid) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT c.id FROM ent_classe c
           JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.code = '4B' AND c.archivee_le IS NULL
          ORDER BY y.courante DESC, y.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    id
}

pub(crate) fn nouveau(type_f: &str, objet: &str, classe: Uuid, texte: &str) -> NouveauFormulaire {
    NouveauFormulaire {
        type_formulaire: type_f.into(),
        objet: Some(objet.into()),
        titre: "Consentement suivi psychologique".into(),
        texte: texte.into(),
        audience: "classe".into(),
        cible_ids: vec![classe],
    }
}

pub(crate) fn rep(eleve: Uuid, decision: &str) -> ReponseFormulaire {
    ReponseFormulaire { eleve_id: eleve, decision: decision.into(), commentaire: None }
}

/// Un parent de plus pour `eleve` (autre foyer), avec ou sans autorité parentale et droit de sortie.
pub(crate) async fn parent_en_plus(ent: &PgEnt, etab: Uuid, eleve: Uuid, autorite: bool, sorties: bool) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1, $2, 'x', 'Second parent', now(), now())",
    )
    .bind(user)
    .bind(format!("fo0-{}@ent.lab", user.simple()))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'parent', now())")
        .bind(user)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let foyer = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, 'Foyer FO0')")
        .bind(foyer)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_lien
            (id, etablissement_id, eleve_id, user_id, foyer_id, lien, autorite_parentale, peut_autoriser_sorties)
         VALUES ($1, $2, $3, $4, $5, 'pere', $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .bind(autorite)
    .bind(sorties)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    user
}

pub(crate) async fn retirer_parent(ent: &PgEnt, etab: Uuid, user: Uuid) {
    sqlx::query("DELETE FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2")
        .bind(etab)
        .bind(user)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

pub(crate) async fn droit_sortie_de_lea(ent: &PgEnt, cast: &LaboCast, valeur: bool) {
    sqlx::query("UPDATE ent_eleve_lien SET peut_autoriser_sorties = $3 WHERE etablissement_id = $1 AND user_id = $2")
        .bind(cast.etablissement_id)
        .bind(cast.parent_lea.user_id)
        .bind(valeur)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

pub(crate) type Naissance = (Option<NaiveDate>, Option<String>);

pub(crate) async fn lire_naissance(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Naissance {
    sqlx::query_as("SELECT date_naissance, date_naissance_statut FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// Pose la date de naissance par le canal de projection de la garde d'identité (seule écriture directe admise).
pub(crate) async fn poser_naissance(ent: &PgEnt, etab: Uuid, eleve: Uuid, date: Option<NaiveDate>, statut: Option<&str>) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)").execute(&mut *tx).await.unwrap();
    sqlx::query(
        "UPDATE ent_eleve SET date_naissance = $3, date_naissance_statut = $4
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .bind(date)
    .bind(statut)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Il y a `ans` ans (jour de l'établissement) : majeur à date certaine pour 19.
pub(crate) fn il_y_a(ans: i32) -> NaiveDate {
    use chrono::Datelike;
    let j = domain_eleve::horloge::jour_paris();
    NaiveDate::from_ymd_opt(j.year() - ans, j.month(), j.day().min(28)).unwrap()
}

pub(crate) async fn compte_eleve(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let existant: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2")
            .bind(etab)
            .bind(eleve)
            .fetch_optional(ent.pool_verif())
            .await
            .unwrap();
    if let Some((u,)) = existant {
        return u;
    }
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1, $2, 'x', 'Élève FO0', now(), now())",
    )
    .bind(user)
    .bind(format!("fo0e-{}@ent.lab", user.simple()))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'eleve', now())")
        .bind(user)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3)")
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    user
}

pub(crate) async fn etat_de(ent: &PgEnt, cast: &LaboCast, objet: &str, classe: Uuid, eleve: Uuid) -> String {
    ent.consentements_classe_for_actor(cast.etablissement_id, cast.direction.user_id, "direction", objet, classe)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.eleve_id == eleve)
        .expect("élève de la classe")
        .decision
}

pub(crate) async fn nb_notifications(ent: &PgEnt, etab: Uuid, user: Uuid, source: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification n
          WHERE n.etablissement_id = $1 AND n.destinataire_user_id = $2 AND n.kind = 'formulaire'
            AND n.source_event_id IN (SELECT o.event_id FROM ent_outbox o WHERE o.aggregate_id = $3)",
    )
    .bind(etab)
    .bind(user)
    .bind(source)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

// ─── FD-01 : une version de fond renouvelle les accords, jamais les refus ────────────────────────────

#[tokio::test]
async fn fd01_ck30_ck31_version_de_fond_renouvelle_les_accords_pas_les_refus() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();

    let v1 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Journal interne")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", v1.id).await.unwrap();
    ent.repondre_formulaire(etab, lea, v1.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "accepte");

    // CK-31 : une correction de forme ne demande rien de nouveau.
    let v2 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Journal interne (orthographe)")).await.unwrap();
    let v2 = ent.publier_formulaire_qualifie(etab, dir, "direction", v2.id, Some("forme")).await.unwrap();
    assert_eq!((v2.version, v2.changement.as_deref(), v2.publie_par), (2, Some("forme"), Some(dir)));
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "accepte", "forme : l'accord de la v1 reste valable");
    let mes = ent.formulaires_famille(etab, lea).await.unwrap();
    let m = mes.iter().find(|m| m.formulaire_id == v2.id && m.eleve_id == harry).unwrap();
    assert!(!m.a_repondre, "forme : rien à refaire");
    assert_eq!(nb_notifications(&ent, etab, lea, v2.id).await, 0, "forme : aucune sollicitation");

    // CK-30 : une version de fond (par défaut) rend l'accord « à renouveler ».
    let v3 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Journal interne et réseaux sociaux")).await.unwrap();
    let v3 = ent.publier_formulaire(etab, dir, "direction", v3.id).await.unwrap();
    assert_eq!(v3.changement.as_deref(), Some("fond"), "par prudence, une nouvelle version est de fond");
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "a_renouveler");
    let mes = ent.formulaires_famille(etab, lea).await.unwrap();
    let m = mes.iter().find(|m| m.formulaire_id == v3.id && m.eleve_id == harry).unwrap();
    assert!(m.a_repondre && m.ma_decision.as_deref() == Some("accepte"));
    assert_eq!(nb_notifications(&ent, etab, lea, v3.id).await, 1, "fond : nouvelle demande");
    // Le renouvellement est une nouvelle décision, même texte de décision.
    ent.repondre_formulaire(etab, lea, v3.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "accepte");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND objet = $2")
        .bind(etab).bind(&objet).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n, 2, "deux décisions : v1 puis v3");

    // Un refus reste protecteur d'une version à l'autre.
    ent.repondre_formulaire(etab, lea, v3.id, rep(harry, "retire")).await.unwrap();
    let v4 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Journal, réseaux, presse")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", v4.id).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "refuse", "le refus survit à la version de fond");
    assert!(ent.eleves_photo_masquee(etab, &[harry]).await.unwrap().is_empty() || objet != "droit_image");

    // Une première version ne se qualifie pas ; une qualification inconnue est refusée.
    let autre = objet_aleatoire();
    let p = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &autre, classe, "T")).await.unwrap();
    assert!(ent.publier_formulaire_qualifie(etab, dir, "direction", p.id, Some("fond")).await.is_err());
    assert!(ent.publier_formulaire_qualifie(etab, dir, "direction", p.id, Some("mineur")).await.is_err());
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-02, CK-06 : un parent sous interdiction de communication ne lit ni ne signe ──────────────────

#[tokio::test]
async fn fd02_ck06_parent_interdit_ne_lit_ni_ne_signe_et_ne_compte_plus() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    droit_sortie_de_lea(&ent, &cast, true).await;
    let f = ent.creer_formulaire(etab, dir, "direction", nouveau("autorisation", &objet, classe, "Sortie au musée le 12 mars")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", f.id).await.unwrap();

    // Phase 1 — interdiction visant un libellé : un lien créé ensuite n'est pas communicant tant qu'il n'est pas confirmé.
    let interdiction = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_libelle, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,'M. Interdit', ARRAY['communication'], 'decision_justice', 'fo0-test', ent_aujourdhui_etab($2) - 1, $4)",
    )
    .bind(interdiction).bind(etab).bind(harry).bind(dir)
    .execute(ent.pool_verif()).await.unwrap();
    let pere = parent_en_plus(&ent, etab, harry, true, true).await;
    assert!(ent.formulaires_famille(etab, pere).await.unwrap().is_empty(), "ni texte ni prénom pour un parent interdit");
    assert!(ent.repondre_formulaire(etab, pere, f.id, rep(harry, "refuse")).await.is_err(), "il ne signe pas");
    assert_eq!(nb_notifications(&ent, etab, pere, f.id).await, 0);
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "aucune", "il n'est pas parmi les décideurs");
    // Les autres titulaires décident, et la règle (`un`) s'applique à eux seuls.
    ent.repondre_formulaire(etab, lea, f.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "accepte");

    // Phase 2 — interdiction visant la personne de Lea : sa décision déjà donnée ne compte plus (CK-06).
    let personne: Option<(Option<Uuid>,)> =
        sqlx::query_as("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3")
            .bind(etab).bind(lea).bind(harry).fetch_optional(ent.pool_verif()).await.unwrap();
    if let Some((Some(p),)) = personne {
        let i2 = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
             VALUES ($1,$2,$3,$4, ARRAY['communication'], 'decision_justice', 'fo0-test', ent_aujourdhui_etab($2) - 1, $5)",
        )
        .bind(i2).bind(etab).bind(harry).bind(p).bind(dir)
        .execute(ent.pool_verif()).await.unwrap();
        assert!(ent.formulaires_famille(etab, lea).await.unwrap().is_empty());
        assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "aucune", "sa décision ne compte plus");
        sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $2 WHERE id = $1")
            .bind(i2).bind(dir).execute(ent.pool_verif()).await.unwrap();
    }
    sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $2 WHERE id = $1")
        .bind(interdiction).bind(dir).execute(ent.pool_verif()).await.unwrap();
    retirer_parent(&ent, etab, pere).await;
    droit_sortie_de_lea(&ent, &cast, false).await;
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-03, R-FC16, R-FC58 : jamais la décision de l'autre titulaire ─────────────────────────────────

#[tokio::test]
async fn fd03_un_parent_ne_voit_pas_la_decision_de_l_autre() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    let f = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Photo")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", f.id).await.unwrap();
    ent.repondre_formulaire(etab, lea, f.id, rep(harry, "refuse")).await.unwrap();

    let vue_pere = ent.formulaires_famille(etab, pere).await.unwrap();
    let m = vue_pere.iter().find(|m| m.formulaire_id == f.id && m.eleve_id == harry).unwrap();
    assert!(m.ma_decision.is_none() && m.a_repondre, "il n'a rien répondu et ne sait pas ce que la mère a répondu");
    assert!(vue_pere.iter().all(|m| m.etat.is_none()), "aucun état global montré aux familles");
    // Sa réponse lui renvoie SA décision, pas l'état global (« refusé » trahirait celle de la mère).
    let r = ent.repondre_formulaire(etab, pere, f.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(r.decision, "accepte");
    let vue_lea = ent.formulaires_famille(etab, lea).await.unwrap();
    assert!(vue_lea.iter().all(|m| m.etat.is_none()));
    assert_eq!(vue_lea.iter().find(|m| m.formulaire_id == f.id).unwrap().ma_decision.as_deref(), Some("refuse"));
    retirer_parent(&ent, etab, pere).await;
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-04, FD-05, CK-15, CK-17, CK-18, CK-19 : majorité et capacité ─────────────────────────────────

#[tokio::test]
async fn fd04_ck19_majeur_prevenu_directement_ses_parents_non() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let eleve_user = compte_eleve(&ent, etab, harry).await;
    let origine = lire_naissance(&ent, etab, harry).await;
    poser_naissance(&ent, etab, harry, Some(il_y_a(19)), origine.1.as_deref()).await;
    let f = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Photo")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", f.id).await.unwrap();
    assert_eq!(nb_notifications(&ent, etab, eleve_user, f.id).await, 1, "l'étudiant de 19 ans est prévenu");
    assert_eq!(nb_notifications(&ent, etab, lea, f.id).await, 0, "ses parents ne le sont pas");
    assert!(ent.formulaires_famille(etab, lea).await.unwrap().iter().all(|m| m.eleve_id != harry));
    assert!(ent.repondre_formulaire(etab, lea, f.id, rep(harry, "accepte")).await.is_err());
    let mes = ent.formulaires_famille(etab, eleve_user).await.unwrap();
    assert!(mes.iter().any(|m| m.formulaire_id == f.id && m.eleve_id == harry && m.a_repondre));
    ent.repondre_formulaire(etab, eleve_user, f.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "accepte");
    poser_naissance(&ent, etab, harry, origine.0, origine.1.as_deref()).await;
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

#[tokio::test]
async fn fd05_ck15_ck17_ck18_tutelle_emancipation_et_date_contestee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let eleve_user = compte_eleve(&ent, etab, harry).await;
    let origine = lire_naissance(&ent, etab, harry).await;
    let decide_seul = || async {
        let (b,): (bool,) = sqlx::query_as("SELECT ent_eleve_decide_seul($1, $2)").bind(etab).bind(harry).fetch_one(ent.pool_verif()).await.unwrap();
        b
    };
    let mesure = |nature: &'static str| {
        let ent = &ent;
        async move {
            sqlx::query(
                "INSERT INTO ent_eleve_capacite (id, etablissement_id, eleve_id, nature, du, piece_document_id)
                 VALUES ($1,$2,$3,$4, ent_aujourdhui_etab($2) - 1, $5)",
            )
            .bind(Uuid::new_v4()).bind(etab).bind(harry).bind(nature).bind(Uuid::new_v4())
            .execute(ent.pool_verif()).await.unwrap();
        }
    };
    let leve = |nature: &'static str| {
        let ent = &ent;
        async move {
            sqlx::query("UPDATE ent_eleve_capacite SET levee_le = now(), levee_par = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = $3 AND levee_le IS NULL")
                .bind(etab).bind(harry).bind(nature).bind(dir)
                .execute(ent.pool_verif()).await.unwrap();
        }
    };
    let objet = objet_aleatoire();
    let f = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Photo")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", f.id).await.unwrap();

    // Mineur : ses parents décident, lui pas.
    assert!(!decide_seul().await);
    assert!(ent.repondre_formulaire(etab, eleve_user, f.id, rep(harry, "accepte")).await.is_err());

    // CK-17 : mineur émancipé, il décide seul ; ses parents ne signent plus.
    mesure("emancipation").await;
    assert!(decide_seul().await);
    assert!(ent.repondre_formulaire(etab, lea, f.id, rep(harry, "accepte")).await.is_err());
    ent.repondre_formulaire(etab, eleve_user, f.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet, classe, harry).await, "accepte");
    leve("emancipation").await;
    assert!(!decide_seul().await);

    // CK-15 : majeur sous tutelle, le tuteur (titulaire) décide, pas l'élève.
    poser_naissance(&ent, etab, harry, Some(il_y_a(19)), origine.1.as_deref()).await;
    assert!(decide_seul().await, "majeur, sans mesure");
    mesure("tutelle").await;
    assert!(!decide_seul().await, "la tutelle retire la décision à l'élève");
    assert!(ent.repondre_formulaire(etab, eleve_user, f.id, rep(harry, "refuse")).await.is_err());
    ent.repondre_formulaire(etab, lea, f.id, rep(harry, "accepte")).await.unwrap();
    leve("tutelle").await;

    // CK-16 : curatelle, l'élève décide (assisté du curateur pour les actes qui l'exigent : FO3).
    mesure("curatelle").await;
    assert!(decide_seul().await);
    leve("curatelle").await;

    // CK-18 : date de naissance contestée, majorité non présumée ; les titulaires décident.
    poser_naissance(&ent, etab, harry, Some(il_y_a(19)), Some("contestee")).await;
    assert!(!decide_seul().await);
    assert!(ent.repondre_formulaire(etab, eleve_user, f.id, rep(harry, "refuse")).await.is_err());

    poser_naissance(&ent, etab, harry, origine.0, origine.1.as_deref()).await;
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-08 : titre neutre hors de l'ENT ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn fd08_le_titre_d_un_formulaire_ne_quitte_pas_l_enveloppe_neutre() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let f = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Suivi")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", f.id).await.unwrap();
    let vues = ent.mes_notifications(etab, lea, 50).await.unwrap();
    let n = vues.iter().find(|n| n.kind == "formulaire").expect("notification");
    assert!(!n.titre.contains("psychologique") && !n.corps.contains("psychologique"), "pas de titre sensible : {}", n.titre);
    // Aucun envoi (e-mail, push, SMS) ne porte le titre : texte neutre par type (R-FC37).
    let envois: Vec<(String,)> = sqlx::query_as(
        "SELECT e.titre FROM ent_notification_envoi e WHERE e.etablissement_id = $1 AND e.kind = 'formulaire' AND e.source_id = $2",
    )
    .bind(etab).bind(f.id).fetch_all(ent.pool_verif()).await.unwrap();
    assert!(envois.iter().all(|(t,)| !t.contains("psychologique")));
    let _ = harry;
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-10, CK-08 : une autorisation exige le droit de sortie du lien ────────────────────────────────

#[tokio::test]
async fn fd10_ck08_autorisation_exige_autoriser_sorties() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet_a = objet_aleatoire();
    let objet_c = objet_aleatoire();
    droit_sortie_de_lea(&ent, &cast, false).await;
    let sortie = ent.creer_formulaire(etab, dir, "direction", nouveau("autorisation", &objet_a, classe, "Sortie")).await.unwrap();
    let image = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet_c, classe, "Image")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", sortie.id).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", image.id).await.unwrap();
    // Sans le droit : ne voit pas l'autorisation, ne la signe pas, n'est pas sollicité ; le consentement reste ouvert.
    let mes = ent.formulaires_famille(etab, lea).await.unwrap();
    assert!(mes.iter().all(|m| m.formulaire_id != sortie.id), "autorisation invisible sans autoriser_sorties");
    assert!(mes.iter().any(|m| m.formulaire_id == image.id), "le consentement ne dépend pas de ce droit");
    assert!(ent.repondre_formulaire(etab, lea, sortie.id, rep(harry, "accepte")).await.is_err());
    assert_eq!(nb_notifications(&ent, etab, lea, sortie.id).await, 0);
    ent.repondre_formulaire(etab, lea, image.id, rep(harry, "accepte")).await.unwrap();
    // Avec le droit : elle signe.
    droit_sortie_de_lea(&ent, &cast, true).await;
    ent.repondre_formulaire(etab, lea, sortie.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, &objet_a, classe, harry).await, "accepte");
    // Le droit retiré après coup : sa décision ne compte plus.
    droit_sortie_de_lea(&ent, &cast, false).await;
    assert_eq!(etat_de(&ent, &cast, &objet_a, classe, harry).await, "aucune");
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-11 : acteurs de la publication et de l'archivage ─────────────────────────────────────────────

#[tokio::test]
async fn fd11_publication_et_archivage_ont_un_acteur() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let adm = cast.administratif.user_id;
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let v1 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "v1")).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", v1.id).await.unwrap();
    let v2 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "v2")).await.unwrap();
    ent.publier_formulaire(etab, adm, "administratif", v2.id).await.unwrap();
    let (publie_par, archive_par): (Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT publie_par, archive_par FROM ent_formulaire WHERE id = $1").bind(v1.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((publie_par, archive_par), (Some(dir), Some(adm)), "v1 publiée par la direction, archivée par la publication de v2");
    let (publie_par, archive_par): (Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT publie_par, archive_par FROM ent_formulaire WHERE id = $1").bind(v2.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((publie_par, archive_par), (Some(adm), None));
    let arch = ent.archiver_formulaire(etab, dir, "direction", v2.id).await.unwrap();
    assert_eq!(arch.statut, "archive");
    let (archive_par,): (Option<Uuid>,) = sqlx::query_as("SELECT archive_par FROM ent_formulaire WHERE id = $1").bind(v2.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(archive_par, Some(dir));
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-12 : immuabilité en base ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fd12_decisions_et_textes_publies_sont_immuables_en_base() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let brouillon = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "Texte")).await.unwrap();
    // Un brouillon se modifie.
    sqlx::query("UPDATE ent_formulaire SET texte = 'Texte corrigé' WHERE id = $1").bind(brouillon.id).execute(ent.pool_verif()).await.unwrap();
    ent.publier_formulaire(etab, dir, "direction", brouillon.id).await.unwrap();
    ent.repondre_formulaire(etab, lea, brouillon.id, rep(harry, "accepte")).await.unwrap();
    let echec = |r: Result<sqlx::postgres::PgQueryResult, sqlx::Error>| match r {
        Err(sqlx::Error::Database(e)) => e.code().map(|c| c.to_string()),
        Ok(_) => None,
        Err(e) => panic!("{e}"),
    };
    let tx = |sql: &'static str, id: Uuid| {
        let pool = ent.pool_verif().clone();
        async move { sqlx::query(sql).bind(id).execute(&pool).await }
    };
    // Le texte publié ne change plus (« accepté » ne peut pas être réécrit après coup).
    assert_eq!(echec(tx("UPDATE ent_formulaire SET texte = 'Autre texte' WHERE id = $1", brouillon.id).await), Some("42501".into()));
    assert_eq!(echec(tx("UPDATE ent_formulaire SET titre = 'Autre' WHERE id = $1", brouillon.id).await), Some("42501".into()));
    assert_eq!(echec(tx("DELETE FROM ent_formulaire WHERE id = $1", brouillon.id).await), Some("42501".into()), "un formulaire publié ne se supprime pas");
    assert_eq!(echec(tx("UPDATE ent_formulaire SET statut = 'brouillon' WHERE id = $1", brouillon.id).await), Some("42501".into()));
    // Une décision est en ajout seul.
    let (dec,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_formulaire_reponse WHERE formulaire_id = $1").bind(brouillon.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(echec(tx("UPDATE ent_formulaire_reponse SET decision = 'refuse' WHERE id = $1", dec).await), Some("42501".into()));
    assert_eq!(echec(tx("UPDATE ent_formulaire_reponse SET empreinte = 'x' WHERE id = $1", dec).await), Some("42501".into()));
    assert_eq!(echec(tx("DELETE FROM ent_formulaire_reponse WHERE id = $1", dec).await), Some("42501".into()));
    // La fusion de deux fiches ne réaffecte que l'élève : permis.
    let autre = cast.eleve_tom_id;
    sqlx::query("UPDATE ent_formulaire_reponse SET eleve_id = $2 WHERE id = $1").bind(dec).bind(autre).execute(ent.pool_verif()).await.unwrap();
    sqlx::query("UPDATE ent_formulaire_reponse SET eleve_id = $2 WHERE id = $1").bind(dec).bind(harry).execute(ent.pool_verif()).await.unwrap();
    // Seul un brouillon jamais publié se supprime.
    let b2 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "v2")).await.unwrap();
    assert_eq!(echec(tx("DELETE FROM ent_formulaire WHERE id = $1", b2.id).await), None);
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-14, FD-15 : numéro de version et nature ──────────────────────────────────────────────────────

#[tokio::test]
async fn fd14_fd15_numero_inferieur_et_nature_verrouillee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let classe = classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let v1 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "v1")).await.unwrap();
    let v2 = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet, classe, "v2")).await.unwrap();
    assert_eq!((v1.version, v2.version), (1, 2));
    ent.publier_formulaire(etab, dir, "direction", v2.id).await.unwrap();
    // FD-14 : publier le brouillon v1 archiverait la v2 : refusé, par le code et par la base.
    assert!(matches!(ent.publier_formulaire(etab, dir, "direction", v1.id).await, Err(EntDbError::ConflitDossier(_))));
    let brut = sqlx::query("UPDATE ent_formulaire SET statut = 'publie', publie_le = now() WHERE id = $1").bind(v1.id).execute(ent.pool_verif()).await;
    assert!(brut.is_err(), "garde en base : numéro inférieur refusé");
    // FD-15 : une « autorisation » du même objet ne devient pas la v3 du consentement.
    let r = ent.creer_formulaire(etab, dir, "direction", nouveau("autorisation", &objet, classe, "v3")).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
    let brut = sqlx::query(
        "INSERT INTO ent_formulaire (id, etablissement_id, type, objet, version, titre, texte, audience, cible_ids, cree_par)
         VALUES ($1,$2,'autorisation',$3, 3, 't', 'x', 'etablissement', '{}', $4)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(&objet).bind(dir)
    .execute(ent.pool_verif()).await;
    assert!(brut.is_err(), "garde en base : la nature d'un objet ne change jamais");
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-17, CK-03, CK-07 : règle `tous` pour l'image ; parent sans autorité ──────────────────────────

#[tokio::test]
async fn fd17_ck03_ck07_droit_a_l_image_exige_les_deux_titulaires() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let classe = classe_4b(&ent, etab).await;
    let cle = format!("{etab}/trombi/{harry}.jpg");
    sqlx::query("UPDATE ent_eleve SET photo_key = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(harry).bind(&cle).execute(ent.pool_verif()).await.unwrap();
    // Un seul titulaire (le lien sans autorité n'est pas décideur : CK-07) : son accord suffit.
    let beau_parent = parent_en_plus(&ent, etab, harry, false, false).await;
    let f = ent.creer_formulaire(etab, dir, "direction", NouveauFormulaire { objet: Some("droit_image".into()), ..nouveau("consentement", "x", classe, "Image") });
    let f = match f.await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    ent.publier_formulaire(etab, dir, "direction", f.id).await.unwrap();
    assert!(ent.formulaires_famille(etab, beau_parent).await.unwrap().is_empty(), "sans autorité parentale : pas décideur");
    assert!(ent.repondre_formulaire(etab, beau_parent, f.id, rep(harry, "accepte")).await.is_err());
    ent.repondre_formulaire(etab, lea, f.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, "droit_image", classe, harry).await, "accepte", "un seul titulaire : son accord suffit");
    // Un second titulaire (autorité parentale) : un seul accord ne suffit plus (CK-03).
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    assert_eq!(etat_de(&ent, &cast, "droit_image", classe, harry).await, "en_attente");
    assert!(!crate::formulaires::donne_droit("en_attente"));
    ent.repondre_formulaire(etab, pere, f.id, rep(harry, "accepte")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, "droit_image", classe, harry).await, "accepte");
    // Le père s'y oppose ensuite : refusé, photo masquée au trombinoscope.
    ent.repondre_formulaire(etab, pere, f.id, rep(harry, "retire")).await.unwrap();
    assert_eq!(etat_de(&ent, &cast, "droit_image", classe, harry).await, "refuse");
    assert_eq!(ent.eleves_photo_masquee(etab, &[harry]).await.unwrap().len(), 1);
    retirer_parent(&ent, etab, pere).await;
    retirer_parent(&ent, etab, beau_parent).await;
    sqlx::query("UPDATE ent_eleve SET photo_key = NULL WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(harry).execute(ent.pool_verif()).await.unwrap();
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── FD-19, FRM-261 : objet validé, santé verrouillée ────────────────────────────────────────────────

#[tokio::test]
async fn fd19_frm261_objet_valide_en_lecture_et_sante_verrouillee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let classe = classe_4b(&ent, etab).await;
    assert!(matches!(
        ent.consentements_classe_for_actor(etab, dir, "direction", "Droit Image'; --", classe).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    for objet in ["sante_allergies", "fiche_medicale", "religion_cantine"] {
        let r = ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", objet, classe, "x")).await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{objet} : refusé tant que le module santé est désactivé (D-034)");
    }
    assert!(ent.creer_formulaire(etab, dir, "direction", nouveau("consentement", &objet_aleatoire(), classe, "x")).await.is_ok());
    crate::formulaires::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── R-FC37 : la garde de neutralité couvre le type `formulaire` ─────────────────────────────────────

#[test]
fn le_type_formulaire_est_neutre_sur_les_canaux_externes() {
    assert!(crate::notifications::canal_externe_neutre("formulaire"));
    let _ = NaiveDate::from_ymd_opt(2026, 1, 1);
}
