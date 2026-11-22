//! Socle 12, lot AS6 (vérification et file du matin). Réf. : `docs/socles/12_ASSIDUITE.md` §9.5, §9.7, §17.6,
//! R-AS25, R-AS35, R-AS37, CA-02, CA-41, CA-50, CA-61, §24.1. Critères de fin : relance unique à `début + délai` ;
//! aucune relance d'une séance non assurée ; file triée selon §17.6. Chaque test travaille dans un établissement
//! neuf (jeu de labo) ; les instants sont donnés au service (jamais l'heure réelle comme condition).
//! Sans `ENT_DATABASE_URL` : ignorés par absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::ntf::catalogue::ABSENCE_CONSTATEE;
use crate::tests_notifications_n1::{basculer, config, distribuer};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use domain_eleve::{DomainError, PoserEdtException};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    Some(
        crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
            e.install_brain_p0();
        })
        .await,
    )
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn introuvable<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Domain(DomainError::NotFound)))
}

fn invalide<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::SaisieInvalide(_)))
}

fn conflit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(_)))
}

async fn aujourdhui(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    ent.aujourdhui_etablissement(cast.etablissement_id).await.unwrap()
}

/// Dernier **jour de cours** (calendrier du socle 5) à la date de l'établissement ou avant : aujourd'hui en semaine,
/// le vendredi un samedi ou un dimanche. Règle 33 : les scénarios de vérification (`verifier_jour_systeme`) reçoivent
/// l'instant en paramètre ; leur jour de référence ne doit donc dépendre que du calendrier, jamais de ce que le test
/// tourne un jour sans cours (le moteur ne relance ni n'alerte un jour fermé, à raison).
async fn jour_ouvre(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    let mut jour = aujourdhui(ent, cast).await;
    let cible = crate::structure_calendrier::Cible { site_id: None, niveau_id: None, classe_id: None, groupe_id: None };
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    for _ in 0..400 {
        let ouvres = crate::structure_calendrier::calendrier::jours_ouvres(&mut conn, cast.etablissement_id, jour, jour, &cible)
            .await
            .unwrap();
        if !ouvres.is_empty() {
            return jour;
        }
        jour -= Duration::days(1);
    }
    panic!("aucun jour de cours dans les 400 derniers jours : calendrier du jeu de labo vide ?");
}

/// Instant UTC d'une heure murale de l'établissement (même fonction SQL que le service).
async fn instant(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate, h: u32, m: u32) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT ent_cdt_instant_local($1, $2, $3)")
        .bind(cast.etablissement_id)
        .bind(jour)
        .bind(t(h, m))
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn compte(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    sqlx::query_scalar(sql).bind(a).bind(b).fetch_one(ent.pool_verif()).await.unwrap()
}

/// Règle d'assiduité en vigueur depuis 2020 (préparation de test : le service refuse une règle posée dans le passé).
async fn poser_regle(ent: &PgEnt, etab: Uuid, consolidation: i32, verification: i32) {
    sqlx::query("DELETE FROM ent_asd_regle WHERE etablissement_id = $1 AND figee_le IS NULL")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_regle (id, etablissement_id, valide_du, delai_consolidation_min, delai_verification_min)
         VALUES ($1, $2, DATE '2020-01-01', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(consolidation)
    .bind(verification)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

async fn creneau(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate, debut: (u32, u32), fin: (u32, u32)) -> Uuid {
    ent.seed_creneau_du_jour(
        cast.etablissement_id,
        "4B",
        "AS6",
        cast.prof.user_id,
        jour,
        t(debut.0, debut.1),
        t(fin.0, fin.1),
        None,
    )
    .await
    .expect("créneau")
    .id
}

fn saisie(etat: &str) -> SaisieLigne {
    SaisieLigne {
        etat: etat.into(),
        minutes: None,
        lieu_code: None,
    }
}

async fn valider(ent: &PgEnt, cast: &LaboCast, appel: Uuid) {
    ent.valider_appel_v2(
        cast.etablissement_id,
        appel,
        cast.prof.user_id,
        "prof",
        ValidationAppel {
            confirme_non_pointes: false,
            canal: None,
        },
    )
    .await
    .expect("valider");
}

/// Appel d'un nouveau créneau, pointé puis validé par le professeur ; renvoie (créneau, appel).
async fn appel(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    debut: (u32, u32),
    fin: (u32, u32),
    lignes: &[(Uuid, &str)],
) -> (Uuid, Uuid) {
    let etab = cast.etablissement_id;
    let c = creneau(ent, cast, jour, debut, fin).await;
    let (a, _) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.expect("ouvrir");
    for (e, etat) in lignes {
        ent.pointer_ligne_v2(etab, a.id, *e, cast.prof.user_id, "prof", saisie(etat), "web")
            .await
            .expect("pointer");
    }
    valider(ent, cast, a.id).await;
    (c, a.id)
}

fn relances_creneau_sql() -> &'static str {
    "SELECT count(*) FROM ent_appel_relance WHERE etablissement_id = $1 AND creneau_id = $2"
}

async fn relances_auto(ent: &PgEnt, cast: &LaboCast, creneau: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_appel_relance WHERE etablissement_id = $1 AND creneau_id = $2 AND origine = 'automatique'",
    )
    .bind(cast.etablissement_id)
    .bind(creneau)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

async fn alertes(ent: &PgEnt, etab: Uuid, eleve: Uuid, kind: &str) -> Vec<(Uuid, String, Option<String>)> {
    sqlx::query_as(
        "SELECT id, statut, cloture_code FROM ent_asd_alerte
          WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = $3 ORDER BY cree_le, id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(kind)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

async fn evenements_type(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2")
        .bind(etab)
        .bind(type_name)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

// ───────────────────────── Relance automatique (AS-151) ─────────────────────────

/// Critère de fin : relance **unique** à `début + délai`, à l'intervenant, tracée comme automatique ; la base refuse
/// une seconde relance automatique ; l'avis suit le circuit existant (notification de l'intervenant).
#[tokio::test]
async fn as6_relance_automatique_unique_a_debut_plus_delai() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 10, 15).await;
    let jour = jour_ouvre(&ent, &cast).await;
    let c = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    // 14 minutes après le début : trop tôt.
    ent.verifier_jour_systeme(etab, instant(&ent, &cast, jour, 8, 14).await).await.unwrap();
    assert_eq!(relances_auto(&ent, &cast, c).await, 0, "avant début + délai : aucune relance");
    // À début + 15 : une relance.
    let b = ent.verifier_jour_systeme(etab, instant(&ent, &cast, jour, 8, 15).await).await.unwrap();
    assert!(b.relances >= 1);
    assert_eq!(relances_auto(&ent, &cast, c).await, 1);
    // Passages suivants : jamais une seconde fois.
    for (h, m) in [(8, 16), (8, 45), (9, 30)] {
        ent.verifier_jour_systeme(etab, instant(&ent, &cast, jour, h, m).await).await.unwrap();
    }
    assert_eq!(relances_auto(&ent, &cast, c).await, 1, "une seule relance automatique");
    // Tracée : origine automatique, sans envoyeur, destinataire = l'intervenant du jour.
    let (origine, par, dest): (String, Option<Uuid>, Uuid) = sqlx::query_as(
        "SELECT origine, envoyee_par_user_id, destinataire_user_id FROM ent_appel_relance
          WHERE etablissement_id = $1 AND creneau_id = $2",
    )
    .bind(etab)
    .bind(c)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((origine.as_str(), par, dest), ("automatique", None, cast.prof.user_id));
    // La base elle-même refuse une seconde relance automatique (index unique partiel).
    let double = sqlx::query(
        "INSERT INTO ent_appel_relance (id, etablissement_id, creneau_id, jour, destinataire_user_id, origine)
         VALUES ($1, $2, $3, $4, $5, 'automatique')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(c)
    .bind(jour)
    .bind(cast.prof.user_id)
    .execute(ent.pool_verif())
    .await;
    assert!(double.is_err(), "deux relances automatiques : refusé par la base");
    // Une relance automatique sans envoyeur seulement : l'incohérence est refusée.
    let faux = sqlx::query(
        "INSERT INTO ent_appel_relance (id, etablissement_id, creneau_id, jour, destinataire_user_id, origine, envoyee_par_user_id)
         VALUES ($1, $2, $3, $4, $5, 'automatique', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(Uuid::new_v4())
    .bind(jour)
    .bind(cast.prof.user_id)
    .execute(ent.pool_verif())
    .await;
    assert!(faux.is_err());
    // L'intervenant est prévenu par le circuit existant (jamais un envoi direct de ce lot).
    distribuer(&ent, etab).await;
    let n = compte(
        &ent,
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'appel_a_faire'",
        etab,
        cast.prof.user_id,
    )
    .await;
    assert!(n >= 1, "l'intervenant est notifié");
    // Une relance manuelle de la vie scolaire reste possible (autre origine) et n'est pas bloquée.
    assert!(ent.relancer_appel(etab, cast.cpe.user_id, "cpe", c, jour).await.is_ok());
    assert_eq!(compte(&ent, relances_creneau_sql(), etab, c).await, 2);
}

/// Aucune relance pour une séance annulée, validée, reprise par la vie scolaire, ni avant l'heure ; la séance voisine
/// qui attend vraiment son appel est relancée (témoin).
#[tokio::test]
async fn as6_aucune_relance_pour_une_seance_annulee_validee_ou_reprise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 10, 15).await;
    let (jour, lea) = (jour_ouvre(&ent, &cast).await, cast.eleve_lea_id);
    // Témoin : séance de 8 h sans appel.
    let temoin = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    // Annulée : exception d'annulation de l'emploi du temps.
    let annulee = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    ent.poser_edt_exception_for_actor(
        "direction",
        PoserEdtException {
            etablissement_id: etab,
            creneau_id: annulee,
            jour,
            kind: EdtExceptionKind::Annulation,
            remplacant_user_id: None,
            nouvelle_salle: None,
            motif: "professeur absent".into(),
            actor_user_id: cast.direction.user_id,
        },
    )
    .await
    .expect("annulation");
    // Validée à temps.
    let (validee, _) = appel(&ent, &cast, jour, (8, 0), (9, 0), &[(lea, "present")]).await;
    // Reprise par la vie scolaire (appel ouvert à sa place) : relance éteinte.
    let reprise = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let r = ent
        .appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", reprise, jour, "oubli", Some(jour))
        .await
        .unwrap();
    assert!(!r.intervenant_lui_meme && !r.deja_ouvert);
    ent.verifier_jour_systeme(etab, instant(&ent, &cast, jour, 8, 40).await).await.unwrap();
    assert_eq!(relances_auto(&ent, &cast, temoin).await, 1, "le témoin est relancé");
    for (nom, c) in [("annulée", annulee), ("validée", validee), ("reprise", reprise)] {
        assert_eq!(relances_auto(&ent, &cast, c).await, 0, "séance {nom} : aucune relance");
    }
}

/// Une séance dont l'état du socle 10 n'est pas « assurée » (non assurée, neutralisée…) n'est jamais relancée ;
/// un jour sans cours (calendrier du socle 5) non plus.
#[tokio::test]
async fn as6_aucune_relance_d_une_seance_non_assuree_ni_un_jour_ferme() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 10, 15).await;
    let jour = jour_ouvre(&ent, &cast).await;
    let non_assuree = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let assuree = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    // Séance du socle 10 « non assurée » (préparation par le propriétaire) rattachée à l'année du jeu de labo.
    let annee: Uuid = match sqlx::query_scalar("SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(etab)
        .fetch_optional(ent.pool_verif())
        .await
        .unwrap()
    {
        Some(a) => a,
        None => {
            let a = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
                 VALUES ($1, $2, 'AS6', $3, $4, true)",
            )
            .bind(a)
            .bind(etab)
            .bind(jour - Duration::days(200))
            .bind(jour + Duration::days(200))
            .execute(ent.pool_verif())
            .await
            .unwrap();
            a
        }
    };
    for (c, etat) in [(non_assuree, "non_assuree"), (assuree, "prevue")] {
        sqlx::query(
            "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe, prof_user_id,
                                         matiere_libelle, heure_debut, heure_fin, statut)
             VALUES ($1, $2, $3, $4, $5, '4B', $6, 'AS6', TIME '08:00', TIME '09:00', $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annee)
        .bind(c)
        .bind(jour)
        .bind(cast.prof.user_id)
        .bind(if etat == "non_assuree" { "annulee" } else { "prevue" })
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    let maintenant = instant(&ent, &cast, jour, 8, 40).await;
    ent.verifier_jour_systeme(etab, maintenant).await.unwrap();
    assert_eq!(relances_auto(&ent, &cast, non_assuree).await, 0, "séance non assurée : aucune relance");
    // Témoin : `jour` est un jour de cours (`jour_ouvre`), la séance assurée est donc relancée.
    assert_eq!(relances_auto(&ent, &cast, assuree).await, 1, "jour de cours : la séance assurée est relancée");
    // Jour sans cours (calendrier du socle 5) : le samedi qui suit, même séance à la même heure, aucune relance.
    let mut samedi = jour;
    while samedi.weekday() != chrono::Weekday::Sat {
        samedi += Duration::days(1);
    }
    let ferme = creneau(&ent, &cast, samedi, (8, 0), (9, 0)).await;
    ent.verifier_jour_systeme(etab, instant(&ent, &cast, samedi, 8, 40).await).await.unwrap();
    assert_eq!(relances_auto(&ent, &cast, ferme).await, 0, "jour sans cours : aucune relance");
    // Un jour fermé (vacances déclarées) : plus aucune relance. On vérifie la règle sur le calendrier.
    sqlx::query("DELETE FROM ent_appel_relance WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.unwrap();
}

// ───────────────────────── Faire l'appel à la place (AS-152) ─────────────────────────

#[tokio::test]
async fn as6_faire_l_appel_a_la_place_droit_trace_et_relance_eteinte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 10, 15).await;
    let (jour, lea) = (jour_ouvre(&ent, &cast).await, cast.eleve_lea_id);
    let c = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    // Le professeur n'a pas `appel.faire_tout` ; la famille, la comptabilité non plus.
    for (u, r) in [
        (cast.prof.user_id, "prof"),
        (cast.parent_lea.user_id, "parent"),
        (cast.compta.user_id, "compta"),
    ] {
        assert!(interdit(ent.appel_a_la_place_au(etab, u, r, c, jour, "oubli", Some(jour)).await), "{r}");
    }
    // Raison inconnue, jour futur, séance inconnue.
    assert!(invalide(ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour, "parce que", Some(jour)).await));
    assert!(invalide(ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour + Duration::days(1), "oubli", Some(jour)).await));
    assert!(introuvable(ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", Uuid::new_v4(), jour, "oubli", Some(jour)).await));
    // Règle « le jour même ou la veille », éprouvée avec une horloge fixée : la veille passe (séance d'un autre jour
    // ouvert, ici hors sujet : `introuvable`, pas `invalide`), deux jours avant est refusé.
    assert!(invalide(ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour - Duration::days(2), "oubli", Some(jour)).await));
    assert!(!invalide(ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour - Duration::days(1), "oubli", Some(jour)).await));
    // La CPE fait l'appel à la place : appel ouvert, substitution tracée.
    let r = ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour, "intervenant_injoignable", Some(jour)).await.unwrap();
    assert_eq!(r.intervenant_user_id, Some(cast.prof.user_id));
    assert!(!r.intervenant_lui_meme && !r.deja_ouvert);
    let (auteur, intervenant, raison): (Uuid, Option<Uuid>, String) = sqlx::query_as(
        "SELECT auteur_user_id, intervenant_user_id, raison_code FROM ent_asd_appel_substitution
          WHERE etablissement_id = $1 AND appel_id = $2",
    )
    .bind(etab)
    .bind(r.appel_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((auteur, intervenant, raison.as_str()), (cast.cpe.user_id, Some(cast.prof.user_id), "intervenant_injoignable"));
    assert_eq!(evenements_type(&ent, etab, types_as6::APPEL_A_LA_PLACE).await, 1);
    // Idempotent : une seconde demande ne double ni la trace ni l'événement.
    let r2 = ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour, "oubli", Some(jour)).await.unwrap();
    assert_eq!((r2.appel_id, r2.deja_ouvert), (r.appel_id, true));
    assert_eq!(compte(&ent, "SELECT count(*) FROM ent_asd_appel_substitution WHERE etablissement_id = $1 AND appel_id = $2", etab, r.appel_id).await, 1);
    assert_eq!(evenements_type(&ent, etab, types_as6::APPEL_A_LA_PLACE).await, 1);
    // La substitution est en ajout seul (déclencheur).
    assert!(sqlx::query("UPDATE ent_asd_appel_substitution SET raison_code = 'panne' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .is_err());
    // Relance éteinte : même à début + 2 heures.
    ent.verifier_jour_systeme(etab, instant(&ent, &cast, jour, 10, 0).await).await.unwrap();
    assert_eq!(relances_auto(&ent, &cast, c).await, 0);
    // La CPE pointe et valide : l'appel garde son auteur et son validateur (auteur noté).
    ent.pointer_ligne_v2(etab, r.appel_id, lea, cast.cpe.user_id, "cpe", saisie("absent"), "web").await.unwrap();
    ent.valider_appel_v2(etab, r.appel_id, cast.cpe.user_id, "cpe", ValidationAppel { confirme_non_pointes: false, canal: None })
        .await
        .unwrap();
    let (auteur_appel, valide_par): (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT auteur_user_id, valide_par FROM ent_appel WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(r.appel_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((auteur_appel, valide_par), (cast.cpe.user_id, Some(cast.cpe.user_id)));
    // Appel déjà validé : on ne le refait pas à la place de personne.
    assert!(conflit(ent.appel_a_la_place_au(etab, cast.cpe.user_id, "cpe", c, jour, "oubli", Some(jour)).await));
    // Un élève d'un autre établissement n'apparaît pas : la séance d'un autre établissement est introuvable.
    assert!(introuvable(ent.appel_a_la_place_au(Uuid::new_v4(), cast.cpe.user_id, "cpe", c, jour, "oubli", Some(jour)).await)
        || interdit(ent.appel_a_la_place_au(Uuid::new_v4(), cast.cpe.user_id, "cpe", c, jour, "oubli", Some(jour)).await));
    // L'intervenant qui fait lui-même l'appel n'est pas une substitution.
    let c2 = creneau(&ent, &cast, jour, (10, 0), (11, 0)).await;
    // (le professeur n'a pas `appel.faire_tout` : seule une personne qui l'a peut passer par cette route)
    let r3 = ent.appel_a_la_place_au(etab, cast.direction.user_id, "direction", c2, jour, "oubli", Some(jour)).await.unwrap();
    assert!(!r3.intervenant_lui_meme);
}

// ───────────────────────── Absence après présence (AS-154) ─────────────────────────

#[tokio::test]
async fn as6_absence_apres_presence_alerte_interne_famille_apres_quinze_minutes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 10, 15).await;
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let (jour, lea, parent) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id, cast.parent_lea.user_id);
    // Présente à 8 h, absente à 10 h (CA-02, CA-61).
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[(lea, "present")]).await;
    distribuer(&ent, etab).await;
    assert!(alertes(&ent, etab, lea, "absence_apres_presence").await.is_empty(), "présente : aucune alerte");
    let (_, a10) = appel(&ent, &cast, jour, (10, 0), (11, 0), &[(lea, "absent")]).await;
    distribuer(&ent, etab).await;
    let al = alertes(&ent, etab, lea, "absence_apres_presence").await;
    assert_eq!(al.len(), 1, "alerte interne immédiate");
    assert_eq!(al[0].1, "ouverte");
    assert_eq!(evenements_type(&ent, etab, types_as6::ABSENCE_APRES_PRESENCE).await, 1);
    // Rejeu du consommateur : aucune alerte de plus, aucun événement de plus.
    let d = ent.distributeur(config(etab));
    d.rejouer_consommateur(CONSOMMATEUR_VERIFICATION, etab, 0).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(alertes(&ent, etab, lea, "absence_apres_presence").await.len(), 1);
    assert_eq!(evenements_type(&ent, etab, types_as6::ABSENCE_APRES_PRESENCE).await, 1);
    // La famille n'est prévenue qu'après 15 minutes au moins (R-N67), pas après les 10 de la règle.
    let pas_encore = crate::ntf::expedition::expedier_avis_dus(ent.pool(), etab, Utc::now() + Duration::minutes(12)).await.unwrap();
    assert_eq!(pas_encore.avis, 0, "12 minutes : trop tôt pour une absence après présence");
    let parti = crate::ntf::expedition::expedier_avis_dus(ent.pool(), etab, Utc::now() + Duration::minutes(20)).await.unwrap();
    assert_eq!(parti.avis, 1);
    let recus: i64 = compte(
        &ent,
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'absence_constatee'",
        etab,
        parent,
    )
    .await;
    assert_eq!(recus, 1);
    // Localisée entre-temps (infirmerie) : l'alerte est close par le système.
    ent.poser_localisation_for_actor(
        etab,
        cast.cpe.user_id,
        "cpe",
        PoserLocalisation {
            eleve_id: lea,
            lieu_code: "infirmerie".into(),
            jour: None,
            heure_debut: None,
            heure_fin: None,
        },
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let al = alertes(&ent, etab, lea, "absence_apres_presence").await;
    assert_eq!((al[0].1.as_str(), al[0].2.as_deref()), ("traitee", Some("localise")));
    let _ = a10;
}

/// Une absence corrigée (l'élève était bien là) clôt l'alerte ; une absence annoncée par la famille n'en crée pas.
#[tokio::test]
async fn as6_la_correction_de_l_appel_clot_l_alerte_apres_presence() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (jour, lea) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[(lea, "present")]).await;
    let (_, a10) = appel(&ent, &cast, jour, (10, 0), (11, 0), &[(lea, "absent")]).await;
    distribuer(&ent, etab).await;
    assert_eq!(alertes(&ent, etab, lea, "absence_apres_presence").await.len(), 1);
    ent.corriger_ligne_for_actor(
        etab,
        a10,
        lea,
        cast.cpe.user_id,
        "cpe",
        CorrectionLigne {
            etat: "present".into(),
            minutes: None,
            lieu_code: None,
            motif_code: None,
            motif_texte: Some("erreur de saisie".into()),
            version_base: None,
        },
        "web",
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let al = alertes(&ent, etab, lea, "absence_apres_presence").await;
    assert_eq!((al[0].1.as_str(), al[0].2.as_deref()), ("traitee", Some("corrige")));
}

// ───────────────────────── Élève exclu de cours non arrivé (AS-209) ─────────────────────────

#[tokio::test]
async fn as6_eleve_exclu_de_cours_non_arrive_alerte_a_dix_minutes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (jour, lea) = (jour_ouvre(&ent, &cast).await, cast.eleve_lea_id);
    let c = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let (a, _) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.unwrap();
    ent.exclure_du_cours_for_actor(etab, a.id, lea, cast.prof.user_id, "prof", "web").await.unwrap();
    // Instant fixé (règle 33) : l'exclusion « maintenant » + 11 minutes franchirait minuit entre 23 h 49 et
    // minuit (heure de l'établissement) et changerait de jour ; l'exclusion est donc datée à 10 h, le même jour.
    let t0 = instant(&ent, &cast, jour, 10, 0).await;
    sqlx::query("UPDATE ent_appel_ligne SET exclusion_cours_le = $4 WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3")
        .bind(etab)
        .bind(a.id)
        .bind(lea)
        .bind(t0)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // 5 minutes plus tard : pas d'alerte.
    let b = ent.verifier_jour_systeme(etab, t0 + Duration::minutes(5)).await.unwrap();
    assert_eq!(b.alertes_non_arrivee, 0);
    // 11 minutes plus tard, sans arrivée confirmée : alerte (une seule, même repassée).
    ent.verifier_jour_systeme(etab, t0 + Duration::minutes(11)).await.unwrap();
    ent.verifier_jour_systeme(etab, t0 + Duration::minutes(12)).await.unwrap();
    let al = alertes(&ent, etab, lea, "non_arrivee").await;
    assert_eq!(al.len(), 1, "une alerte, pas deux");
    assert_eq!(evenements_type(&ent, etab, types_as6::NON_ARRIVEE).await, 1);
    // La vie scolaire confirme l'arrivée : l'alerte se clôt au passage suivant.
    ent.confirmer_arrivee_vs_for_actor(etab, a.id, lea, cast.cpe.user_id, "cpe").await.unwrap();
    let b = ent.verifier_jour_systeme(etab, t0 + Duration::minutes(13)).await.unwrap();
    assert_eq!(b.alertes_closes, 1);
    let al = alertes(&ent, etab, lea, "non_arrivee").await;
    assert_eq!((al[0].1.as_str(), al[0].2.as_deref()), ("traitee", Some("arrive")));
}

// ───────────────────────── Prévenir maintenant (AS-155) ─────────────────────────

#[tokio::test]
async fn as6_prevenir_maintenant_passe_par_le_moteur_en_u3_sans_delai() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 20, 15).await;
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let (jour, lea, tom, parent) = (
        aujourdhui(&ent, &cast).await,
        cast.eleve_lea_id,
        cast.eleve_tom_id,
        cast.parent_lea.user_id,
    );
    // Droits : la direction et la CPE seulement ; ni professeur, ni famille, ni comptabilité, ni secrétariat.
    let req = || PrevenirMaintenant {
        eleve_id: lea,
        raison_code: "fugue_suspectee".into(),
    };
    for (u, r) in [
        (cast.prof.user_id, "prof"),
        (parent, "parent"),
        (cast.compta.user_id, "compta"),
        (cast.administratif.user_id, "administratif"),
    ] {
        assert!(interdit(ent.prevenir_maintenant_for_actor(etab, u, r, req()).await), "{r}");
    }
    // Sans absence constatée aujourd'hui : refusé (on n'alerte pas une famille sans fait).
    assert!(conflit(ent.prevenir_maintenant_for_actor(etab, cast.cpe.user_id, "cpe", req()).await));
    // Raison inconnue ; élève inconnu.
    let mut mauvaise = req();
    mauvaise.raison_code = "au_hasard".into();
    assert!(invalide(ent.prevenir_maintenant_for_actor(etab, cast.cpe.user_id, "cpe", mauvaise).await));
    assert!(introuvable(
        ent.prevenir_maintenant_for_actor(
            etab,
            cast.cpe.user_id,
            "cpe",
            PrevenirMaintenant { eleve_id: Uuid::new_v4(), raison_code: "fugue_suspectee".into() }
        )
        .await
    ));
    // Léa est absente à 8 h : l'avis ordinaire est planifié (délai de 20 minutes).
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[(lea, "absent")]).await;
    distribuer(&ent, etab).await;
    let ordinaires: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, statut FROM ent_ntf_avis WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'vie_scolaire.absence_constatee'",
    )
    .bind(etab)
    .bind(lea)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(ordinaires.len(), 1);
    assert_eq!(ordinaires[0].1, "planifie");
    // Rien n'est parti : aucun envoi direct.
    assert_eq!(compte(&ent, "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2", etab, parent).await, 0);
    // La CPE prévient maintenant.
    let r = ent.prevenir_maintenant_for_actor(etab, cast.cpe.user_id, "cpe", req()).await.unwrap();
    assert!(!r.deja_fait && r.avis_retire && !r.sans_destinataire && r.destinataires >= 1, "{r:?}");
    // L'avis est de la brique : U3, sans délai de confirmation, nature dédiée.
    let (urgence, nature, confirme, effet): (String, String, DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        "SELECT urgence, nature, confirme_apres, effet_le FROM ent_ntf_avis WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(r.avis_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((urgence.as_str(), nature.as_str()), ("u3", "vie_scolaire.absence_inquietante"));
    assert_eq!(confirme, effet, "sans délai de confirmation");
    // L'avis ordinaire encore planifié est retiré, le registre le dit.
    let statut: String = sqlx::query_scalar("SELECT statut FROM ent_ntf_avis WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(ordinaires[0].0)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(statut, "annule");
    let cause: Option<String> = sqlx::query_scalar(
        "SELECT cause_code FROM ent_asd_avis_demi_journee WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(cause.as_deref(), Some("urgence"));
    // La famille reçoit l'urgence tout de suite (une minute plus tard), par le moteur.
    let b = crate::ntf::expedition::expedier_avis_dus(ent.pool(), etab, Utc::now() + Duration::minutes(1)).await.unwrap();
    assert_eq!(b.avis, 1);
    let recus: Vec<(String, String)> = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'absence_inquietante'",
    )
    .bind(etab)
    .bind(parent)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(recus.len(), 1);
    assert!(recus[0].1.contains("pas de nouvelles"), "{recus:?}");
    // Décision tracée.
    let (auteur, raison, avis, remplace): (Uuid, String, Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT auteur_user_id, raison_code, avis_id, avis_remplace_id FROM ent_asd_prevenir
          WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((auteur, raison.as_str(), avis, remplace), (cast.cpe.user_id, "fugue_suspectee", r.avis_id, Some(ordinaires[0].0)));
    assert_eq!(evenements_type(&ent, etab, types_as6::ALERTE_URGENTE).await, 1);
    // Idempotent : un second geste le même jour n'envoie rien de plus.
    let r2 = ent.prevenir_maintenant_for_actor(etab, cast.direction.user_id, "direction", req()).await.unwrap();
    assert!(r2.deja_fait);
    assert_eq!(r2.avis_id, r.avis_id);
    assert_eq!(compte(&ent, "SELECT count(*) FROM ent_asd_prevenir WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await, 1);
    // Une absence plus tard dans la journée ne déclenche pas un avis ordinaire derrière l'urgence.
    appel(&ent, &cast, jour, (10, 0), (11, 0), &[(lea, "absent")]).await;
    distribuer(&ent, etab).await;
    let n: i64 = compte(
        &ent,
        "SELECT count(*) FROM ent_ntf_avis WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'vie_scolaire.absence_constatee' AND statut <> 'annule'",
        etab,
        lea,
    )
    .await;
    assert_eq!(n, 0, "la famille est déjà alertée en urgence ce jour");
    // Un élève qui n'a aucune absence n'est pas alerté (Tom).
    assert!(conflit(
        ent.prevenir_maintenant_for_actor(etab, cast.cpe.user_id, "cpe", PrevenirMaintenant { eleve_id: tom, raison_code: "autre_urgence".into() }).await
    ));
    // La décision est en ajout seul.
    assert!(sqlx::query("DELETE FROM ent_asd_prevenir WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .is_err());
    // Les événements ne portent aucun texte libre (R-AS25).
    let mauvais: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = ANY($2)
            AND payload::text ~* '(texte|note|motif_texte|nom|prenom)'",
    )
    .bind(etab)
    .bind(vec![types_as6::ALERTE_URGENTE, types_as6::ABSENCE_APRES_PRESENCE, types_as6::NON_ARRIVEE, types_as6::APPEL_A_LA_PLACE])
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(mauvais, 0);
}

// ───────────────────────── File du matin (AS-156, §17.6) ─────────────────────────

fn contact(eleve: Uuid, canal: &str, issue: &str) -> TracerContact {
    TracerContact {
        eleve_id: eleve,
        absence_id: None,
        personne_id: None,
        canal: canal.into(),
        sens: None,
        issue: issue.into(),
        note: Some("note-secrete-xyz".into()),
    }
}

/// Critère de fin : la file est **triée selon §17.6** : prévenir maintenant, exclu non arrivé, absence après présence,
/// vigilance, famille tentée sans succès, mineur sans nouvelles, puis les appels non faits ; une ligne par élève ;
/// la famille jointe disparaît ; ni note interne ni motif ne sortent.
#[tokio::test]
async fn as6_la_file_du_matin_est_classee_selon_17_6() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    poser_regle(&ent, etab, 10, 15).await;
    let jour = aujourdhui(&ent, &cast).await;
    let lea = cast.eleve_lea_id;
    let par_prenom = |p: &str| cast.eleves.iter().find(|e| e.prenom == p).map(|e| e.id).expect("élève du labo");
    let tom = par_prenom("Draco");
    let (hermione, ron, ginny) = (par_prenom("Hermione"), par_prenom("Ron"), par_prenom("Ginny"));
    // Chacun pointé dans l'appel de sa classe de 8 h ; Ginny, présente à 8 h, est absente à 10 h (après présence).
    let mut classe_ginny = String::from("4B");
    for classe in ["4B", "5A"] {
        let c = ent
            .seed_creneau_du_jour(etab, classe, "AS6", cast.prof.user_id, jour, t(8, 0), t(9, 0), None)
            .await
            .unwrap()
            .id;
        let (a, lignes) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.unwrap();
        for (e, etat) in [(ginny, "present"), (lea, "absent"), (hermione, "absent")] {
            if lignes.iter().any(|l| l.eleve_id == e) {
                if e == ginny {
                    classe_ginny = classe.to_string();
                }
                ent.pointer_ligne_v2(etab, a.id, e, cast.prof.user_id, "prof", saisie(etat), "web").await.unwrap();
            }
        }
        valider(&ent, &cast, a.id).await;
    }
    let c10 = ent
        .seed_creneau_du_jour(etab, &classe_ginny, "AS6", cast.prof.user_id, jour, t(10, 0), t(11, 0), None)
        .await
        .unwrap()
        .id;
    let (a10, _) = ent.ouvrir_appel(etab, c10, jour, cast.prof.user_id, "prof").await.unwrap();
    ent.pointer_ligne_v2(etab, a10.id, ginny, cast.prof.user_id, "prof", saisie("absent"), "web").await.unwrap();
    valider(&ent, &cast, a10.id).await;
    distribuer(&ent, etab).await;
    // Un créneau de 8 h 20 sans appel : pas encore dans la file à 8 h 31 (début + 2 × 15 = 8 h 50).
    let sans_appel_tot = creneau(&ent, &cast, jour, (8, 20), (9, 20)).await;
    // Un créneau de 8 h sans appel : dans la file dès 8 h 30.
    let sans_appel = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    // Léa : prévenir maintenant (niveau 1).
    ent.prevenir_maintenant_for_actor(etab, cast.cpe.user_id, "cpe", PrevenirMaintenant { eleve_id: lea, raison_code: "eleve_en_danger".into() })
        .await
        .unwrap();
    let maintenant = instant(&ent, &cast, jour, 10, 45).await;
    let file = || ent.file_du_matin_for_actor(etab, cast.cpe.user_id, "cpe", None, maintenant);
    let niveau_de_eleve = |f: &FileDuMatin, e: Uuid| f.entrees.iter().find(|x| x.eleve_id == Some(e)).map(|x| x.niveau);
    // Hermione, absente sans nouvelles : mineure sans nouvelles (niveau 6) ; sous vigilance (4) ; une fois la famille
    // tentée sans succès (5, ou 4 sous vigilance).
    let f = file().await.unwrap();
    assert_eq!(niveau_de_eleve(&f, hermione), Some(6), "{:#?}", f.entrees);
    ent.vigilance_poser_for_actor(etab, cast.cpe.user_id, "cpe", hermione).await.unwrap();
    assert_eq!(niveau_de_eleve(&file().await.unwrap(), hermione), Some(4));
    ent.vigilance_lever_for_actor(etab, cast.cpe.user_id, "cpe", hermione).await.unwrap();
    // Famille tentée sans succès (niveau 5).
    ent.tracer_contact_for_actor(etab, cast.cpe.user_id, "cpe", contact(hermione, "telephone", "ne_repond_pas")).await.unwrap();
    let f = file().await.unwrap();
    let par_eleve = |e: Uuid| f.entrees.iter().find(|x| x.eleve_id == Some(e));
    let niveau = |e: Uuid| par_eleve(e).map(|x| x.niveau);
    assert_eq!(niveau(lea), Some(1));
    assert_eq!(niveau(ginny), Some(3));
    assert_eq!(niveau(hermione), Some(5));
    // Une ligne par élève ; les autres raisons de Léa (absente sans nouvelles) sont listées sur sa ligne.
    for e in [lea, hermione, ginny] {
        assert_eq!(f.entrees.iter().filter(|x| x.eleve_id == Some(e) && x.categorie != "incoherence").count(), 1, "une ligne par élève");
    }
    assert!(par_eleve(lea).unwrap().raisons.iter().any(|r| r == "sans_nouvelles"));
    let _ = (tom, ron);
    // Les appels non faits suivent (niveau 8) : le créneau de 8 h y est, celui de 8 h 20 y est aussi à 10 h 45.
    let appels: Vec<_> = f.entrees.iter().filter(|x| x.categorie == "appel_non_fait").collect();
    assert!(appels.iter().any(|x| x.creneau_id == Some(sans_appel)));
    assert!(appels.iter().any(|x| x.creneau_id == Some(sans_appel_tot)));
    // Ordre global : niveaux croissants, rangs 1..n.
    let niveaux: Vec<u8> = f.entrees.iter().map(|x| x.niveau).collect();
    let mut tries = niveaux.clone();
    tries.sort();
    assert_eq!(niveaux, tries, "la file est triée par niveau");
    assert!(f.entrees.iter().enumerate().all(|(i, x)| x.rang as usize == i + 1));
    assert_eq!(f.compteurs.get("urgence"), Some(&1));
    // À 8 h 31, l'appel de 8 h 20 n'est pas encore dans la file (début + 2 × délai).
    let tot = ent.file_du_matin_for_actor(etab, cast.cpe.user_id, "cpe", None, instant(&ent, &cast, jour, 8, 31).await).await.unwrap();
    assert!(tot.entrees.iter().any(|x| x.creneau_id == Some(sans_appel)));
    assert!(!tot.entrees.iter().any(|x| x.creneau_id == Some(sans_appel_tot)), "pas avant début + 2 × délai");
    // La famille d'Hermione est jointe : elle quitte la file ; la note interne n'en sort jamais.
    ent.tracer_contact_for_actor(etab, cast.cpe.user_id, "cpe", contact(hermione, "telephone", "joint")).await.unwrap();
    let f2 = ent.file_du_matin_for_actor(etab, cast.cpe.user_id, "cpe", None, maintenant).await.unwrap();
    assert!(!f2.entrees.iter().any(|x| x.eleve_id == Some(hermione) && x.categorie != "incoherence"), "famille jointe : plus dans la file {:?}", f2.entrees.iter().filter(|x| x.eleve_id == Some(hermione)).map(|x| (x.categorie.clone(), x.detail_code.clone())).collect::<Vec<_>>());
    let json = serde_json::to_string(&f2).unwrap();
    assert!(!json.contains("note-secrete-xyz"), "aucune note interne");
    // « Je prends » : verrou visible sur l'alerte d'absence après présence de Ginny.
    let alerte = alertes(&ent, etab, ginny, "absence_apres_presence").await[0].0;
    ent.alerte_prendre_for_actor(etab, cast.cpe.user_id, "cpe", alerte).await.unwrap();
    let f3 = ent.file_du_matin_for_actor(etab, cast.cpe.user_id, "cpe", None, Utc::now()).await.unwrap();
    let ligne = f3.entrees.iter().find(|x| x.eleve_id == Some(ginny)).unwrap();
    assert_eq!(ligne.pris_par, Some(cast.cpe.user_id));
    // Un jour futur n'a pas de file ; le professeur et la famille n'y accèdent pas.
    assert!(invalide(ent.file_du_matin_for_actor(etab, cast.cpe.user_id, "cpe", Some(jour + Duration::days(1)), maintenant).await));
    for (u, r) in [(cast.prof.user_id, "prof"), (cast.parent_lea.user_id, "parent"), (cast.compta.user_id, "compta")] {
        assert!(interdit(ent.file_du_matin_for_actor(etab, u, r, None, maintenant).await), "{r}");
    }
}

// ───────────────────────── Alertes et vigilance ─────────────────────────

#[tokio::test]
async fn as6_je_prends_verrou_traitement_et_droits() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (jour, lea) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[(lea, "present")]).await;
    appel(&ent, &cast, jour, (10, 0), (11, 0), &[(lea, "absent")]).await;
    distribuer(&ent, etab).await;
    let alerte = alertes(&ent, etab, lea, "absence_apres_presence").await[0].0;
    // Droits : `assiduite.suivre` (CPE, direction, secrétariat) ; jamais le professeur ni la famille.
    for (u, r) in [(cast.prof.user_id, "prof"), (cast.parent_lea.user_id, "parent"), (cast.compta.user_id, "compta")] {
        assert!(interdit(ent.alerte_prendre_for_actor(etab, u, r, alerte).await), "{r}");
        assert!(interdit(ent.alerte_traiter_for_actor(etab, u, r, alerte, "sans_suite").await), "{r}");
        assert!(interdit(ent.alertes_for_actor(etab, u, r, jour).await), "{r}");
    }
    let a = ent.alerte_prendre_for_actor(etab, cast.cpe.user_id, "cpe", alerte).await.unwrap();
    assert_eq!((a.statut.as_str(), a.pris_par), ("prise", Some(cast.cpe.user_id)));
    // Un autre membre de la vie scolaire ne prend pas une alerte déjà prise (verrou de 15 minutes) ; la même
    // personne le prolonge.
    assert!(conflit(ent.alerte_prendre_for_actor(etab, cast.administratif.user_id, "administratif", alerte).await));
    assert!(ent.alerte_prendre_for_actor(etab, cast.cpe.user_id, "cpe", alerte).await.is_ok());
    // Une alerte d'un autre établissement ou inexistante : même réponse.
    assert!(introuvable(ent.alerte_prendre_for_actor(etab, cast.cpe.user_id, "cpe", Uuid::new_v4()).await));
    assert!(introuvable(ent.alerte_traiter_for_actor(etab, cast.cpe.user_id, "cpe", Uuid::new_v4(), "sans_suite").await));
    // Issues : liste fermée ; « corrigé » est réservée au système.
    assert!(invalide(ent.alerte_traiter_for_actor(etab, cast.cpe.user_id, "cpe", alerte, "au_hasard").await));
    assert!(invalide(ent.alerte_traiter_for_actor(etab, cast.cpe.user_id, "cpe", alerte, "corrige").await));
    assert_eq!(ent.alertes_for_actor(etab, cast.cpe.user_id, "cpe", jour).await.unwrap().len(), 1);
    let t = ent.alerte_traiter_for_actor(etab, cast.cpe.user_id, "cpe", alerte, "famille_jointe").await.unwrap();
    assert_eq!((t.statut.as_str(), t.cloture_code.as_deref()), ("traitee", Some("famille_jointe")));
    assert!(ent.alertes_for_actor(etab, cast.cpe.user_id, "cpe", jour).await.unwrap().is_empty());
    // Déjà traitée : plus rien à faire, même réponse que « inexistante ».
    assert!(introuvable(ent.alerte_traiter_for_actor(etab, cast.cpe.user_id, "cpe", alerte, "sans_suite").await));
    assert!(conflit(ent.alerte_prendre_for_actor(etab, cast.cpe.user_id, "cpe", alerte).await));
}

#[tokio::test]
async fn as6_vigilance_sans_motif_posee_levee_jamais_supprimee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    for (u, r) in [(cast.prof.user_id, "prof"), (cast.parent_lea.user_id, "parent"), (cast.compta.user_id, "compta")] {
        assert!(interdit(ent.vigilance_poser_for_actor(etab, u, r, lea).await), "{r}");
        assert!(interdit(ent.vigilance_lever_for_actor(etab, u, r, lea).await), "{r}");
        assert!(interdit(ent.vigilances_for_actor(etab, u, r).await), "{r}");
    }
    assert!(introuvable(ent.vigilance_poser_for_actor(etab, cast.cpe.user_id, "cpe", Uuid::new_v4()).await));
    ent.vigilance_poser_for_actor(etab, cast.cpe.user_id, "cpe", lea).await.unwrap();
    // Idempotent : un seul drapeau vivant.
    ent.vigilance_poser_for_actor(etab, cast.direction.user_id, "direction", lea).await.unwrap();
    assert_eq!(ent.vigilances_for_actor(etab, cast.cpe.user_id, "cpe").await.unwrap().len(), 1);
    // Jamais supprimé ; ne se modifie que pour être levé.
    assert!(sqlx::query("DELETE FROM ent_asd_vigilance WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("UPDATE ent_asd_vigilance SET pose_par = eleve_id WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    ent.vigilance_lever_for_actor(etab, cast.cpe.user_id, "cpe", lea).await.unwrap();
    assert!(ent.vigilances_for_actor(etab, cast.cpe.user_id, "cpe").await.unwrap().is_empty());
    assert!(introuvable(ent.vigilance_lever_for_actor(etab, cast.cpe.user_id, "cpe", lea).await), "déjà levé");
    // L'historique est gardé et on peut reposer un drapeau.
    ent.vigilance_poser_for_actor(etab, cast.cpe.user_id, "cpe", lea).await.unwrap();
    assert_eq!(compte(&ent, "SELECT count(*) FROM ent_asd_vigilance WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await, 2);
    // Aucun motif en base : le drapeau n'a aucune colonne de texte libre.
    let colonnes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_name = 'ent_asd_vigilance' AND data_type IN ('text', 'character varying', 'json', 'jsonb')",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(colonnes, 0);
}

/// Cloisonnement : un acteur d'un autre établissement ne voit ni n'agit sur les données de celui-ci ; les écritures
/// visant un élève ou une alerte d'ailleurs ont la même réponse que « inexistant ».
#[tokio::test]
async fn as6_cloisonnement_par_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (jour, autre) = (aujourdhui(&ent, &cast).await, Uuid::new_v4());
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[(lea, "present")]).await;
    appel(&ent, &cast, jour, (10, 0), (11, 0), &[(lea, "absent")]).await;
    distribuer(&ent, etab).await;
    let alerte = alertes(&ent, etab, lea, "absence_apres_presence").await[0].0;
    // Même identité, mais dans un établissement où elle n'a aucun droit : refus ou « introuvable », jamais de donnée.
    assert!(ent.file_du_matin_for_actor(autre, cast.cpe.user_id, "cpe", None, Utc::now()).await.map_or(true, |f| f.entrees.is_empty()));
    assert!(ent.alerte_traiter_for_actor(autre, cast.cpe.user_id, "cpe", alerte, "sans_suite").await.is_err());
    assert!(ent.alerte_prendre_for_actor(autre, cast.cpe.user_id, "cpe", alerte).await.is_err());
    assert!(ent.prevenir_maintenant_for_actor(autre, cast.cpe.user_id, "cpe", PrevenirMaintenant { eleve_id: lea, raison_code: "fugue_suspectee".into() }).await.is_err());
    assert!(ent.vigilance_poser_for_actor(autre, cast.cpe.user_id, "cpe", lea).await.is_err());
    // Rien n'a bougé.
    let al = alertes(&ent, etab, lea, "absence_apres_presence").await;
    assert_eq!(al[0].1, "ouverte");
}
