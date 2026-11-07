//! Socle 2 (l'élève), lot E4 : scolarité (scolarisations datées, statut dérivé, inscriptions,
//! changements datés, capacité, majorité, mobilité, modalités, tâche quotidienne). Scénarios
//! contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use crate::scolarite::*;
use chrono::{Duration, NaiveDate, Utc};
use domain_eleve::{
    statut_derive, ChangeStatus, CodeModalite, CreateEleve, DomainError, EleveStatus,
    EtablissementScolarite, EtatInscription, MotifDerogationRegime, MotifEntree, MotifSortie,
    NatureCapacite, PeriodeScolarisation, SortirEleve, StatutApprenant, TypeEtablissement, TypeMobilite,
};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn aujourdhui() -> NaiveDate {
    Utc::now().date_naive()
}

fn alea() -> String {
    Uuid::new_v4().simple().to_string()[..6].replace(char::is_numeric, "x")
}

async fn eleve_le(
    ent: &PgEnt,
    etab: Uuid,
    entree: Option<NaiveDate>,
    naissance: Option<NaiveDate>,
    classe: Option<&str>,
) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Scol{}", alea()),
        prenom: "Zélie".into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: naissance.or(NaiveDate::from_ymd_opt(2010, 3, 14)),
        numero_badge: None,
        redoublant: false,
        classe: classe.map(String::from),
        regime: None,
        mef: None,
        site: None,
        date_entree: entree,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

async fn eleve(ent: &PgEnt, etab: Uuid) -> domain_eleve::Eleve {
    eleve_le(ent, etab, None, None, None).await
}

fn ouverture(etab: Uuid, eleve: Uuid, date: NaiveDate, motif: MotifEntree, acteur: Uuid) -> OuvertureScolarisation {
    OuvertureScolarisation {
        etablissement_id: etab,
        eleve_id: eleve,
        date_entree: date,
        motif,
        origine: EtablissementScolarite::default(),
        source: "saisie".into(),
        source_ref: None,
        acteur: Some(acteur),
    }
}

fn cloture(etab: Uuid, eleve: Uuid, date: NaiveDate, motif: MotifSortie, acteur: Uuid) -> ClotureScolarisation {
    ClotureScolarisation {
        etablissement_id: etab,
        eleve_id: eleve,
        date_sortie: date,
        motif,
        motif_libre: None,
        destination: EtablissementScolarite::default(),
        exeat_document_id: None,
        acteur: Some(acteur),
    }
}

async fn types_evenements(ent: &PgEnt, eleve: Uuid) -> Vec<String> {
    let l: Vec<(String,)> =
        sqlx::query_as("SELECT type FROM ent_metier_event WHERE eleve_id = $1 ORDER BY created_at, id")
            .bind(eleve)
            .fetch_all(ent.pool())
            .await
            .unwrap();
    l.into_iter().map(|(t,)| t).collect()
}

async fn statut_cache(ent: &PgEnt, eleve: Uuid) -> (Option<String>, String) {
    sqlx::query_as("SELECT statut, status FROM ent_eleve WHERE id = $1")
        .bind(eleve)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

async fn piece(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, mime_type, source)
         VALUES ($1,$2,$3,'administratif','Jugement','application/pdf','upload')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

const ADM: &str = "administratif";

/// Le statut est DÉRIVÉ : la base et la logique pure donnent le même résultat à toutes les
/// bornes, et le cache d'`ent_eleve` suit chaque commande ; jamais saisi.
#[tokio::test]
async fn statut_derive_base_et_logique_pure_concordent() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve(&ent, etab).await;
    let entree = aujourdhui() - Duration::days(30);
    let sortie = aujourdhui() - Duration::days(10);
    // Reprise du cadre : l'élève créé aujourd'hui a une scolarisation héritée ; on la remplace
    // par un scénario daté (sortie passée, retour futur).
    ent.cloturer_scolarisation_systeme(cloture(etab, e.id, aujourdhui(), MotifSortie::Autre, admin)).await.ok();
    sqlx::query("ALTER TABLE ent_eleve_scolarisation DISABLE TRIGGER ent_eleve_scolarisation_garde")
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve_scolarisation SET date_entree = $2, date_sortie = $3, motif_sortie = 'demenagement' WHERE eleve_id = $1")
        .bind(e.id)
        .bind(entree)
        .bind(sortie)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_eleve_scolarisation ENABLE TRIGGER ent_eleve_scolarisation_garde")
        .execute(ent.pool())
        .await
        .unwrap();
    let retour = aujourdhui() + Duration::days(60);
    ent.ouvrir_scolarisation(Role::Administratif.as_str(), ouverture(etab, e.id, retour, MotifEntree::Retour, admin))
        .await
        .unwrap();
    let periodes = vec![
        PeriodeScolarisation { date_entree: entree, date_sortie: Some(sortie), motif_sortie: Some(MotifSortie::Demenagement) },
        PeriodeScolarisation { date_entree: retour, date_sortie: None, motif_sortie: None },
    ];
    for delta in [-31i64, -30, -29, -11, -10, -9, 0, 59, 60, 61] {
        let jour = aujourdhui() + Duration::days(delta);
        let sql = ent.statut_eleve_a(etab, e.id, jour).await.unwrap();
        assert_eq!(sql, statut_derive(&periodes, jour).code(), "jour {jour}");
    }
    // Aujourd'hui : sorti (sortie passée, retour annoncé plus tard → « à venir »).
    assert_eq!(ent.statut_eleve_a(etab, e.id, aujourdhui()).await.unwrap(), "a_venir");
    assert_eq!(statut_cache(&ent, e.id).await.0.as_deref(), Some("a_venir"));
    // Le statut ne se saisit pas : une écriture directe du cache est corrigée par la tâche.
    sqlx::query("UPDATE ent_eleve SET statut = 'scolarise' WHERE id = $1").bind(e.id).execute(ent.pool()).await.unwrap();
    ent.tache_nocturne_eleves().await.unwrap();
    assert_eq!(statut_cache(&ent, e.id).await.0.as_deref(), Some("a_venir"));
}

/// Données de labo : le statut en cache de TOUS les élèves est celui que dérivent leurs
/// scolarisations (statut dérivé = statut attendu).
#[tokio::test]
async fn statut_de_tous_les_eleves_du_labo_est_celui_attendu() {
    let Some((ent, _cast)) = labo().await else { return };
    let (nb, ecarts): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE statut IS DISTINCT FROM
            ent_eleve_statut_a(etablissement_id, id, ent_jour_etablissement(etablissement_id)))
         FROM ent_eleve",
    )
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(nb > 0);
    assert_eq!(ecarts, 0, "statut en cache ≠ statut dérivé pour {ecarts} élève(s) sur {nb}");
    // Et le statut texte hérité reste cohérent avec le statut dérivé (hors « à venir »).
    let (incoherents,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve WHERE (statut = 'scolarise' AND status <> 'actif')
                                            OR (statut = 'sorti' AND status <> 'sorti')",
    )
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(incoherents, 0);
}

/// ELV-08 / ELV-11 / ELV-12 : entrée, sortie, réintégration sur la MÊME fiche ; événements
/// réservés enfin émis ; non-chevauchement (logique ET base).
#[tokio::test]
async fn entree_sortie_reintegration_et_non_chevauchement() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(100)), None, Some("6A")).await;
    // Une scolarisation héritée est ouverte : en ouvrir une autre est refusé.
    let deja = ent
        .ouvrir_scolarisation(role, ouverture(etab, e.id, aujourdhui(), MotifEntree::Reprise, admin))
        .await;
    assert!(matches!(deja, Err(EntDbError::ConflitDossier(_))), "{deja:?}");
    // ELV-11 : sortie le jour même ; affectations fermées ; statut recalculé.
    let sortie = aujourdhui();
    let r = ent
        .cloturer_scolarisation(role, cloture(etab, e.id, sortie, MotifSortie::Demenagement, admin))
        .await
        .unwrap();
    assert_eq!(r.statut, "sorti");
    assert!(r.statut_change);
    assert_eq!(r.scolarisation.motif_sortie.as_deref(), Some("demenagement"));
    let (statut, status) = statut_cache(&ent, e.id).await;
    assert_eq!((statut.as_deref(), status.as_str()), (Some("sorti"), "sorti"));
    let (ds, ms): (Option<NaiveDate>, Option<String>) =
        sqlx::query_as("SELECT date_sortie, motif_sortie FROM ent_eleve WHERE id = $1").bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((ds, ms.as_deref()), (Some(sortie), Some("demenagement")), "projection héritée");
    // Clôturer deux fois : refusé. Motif décès par la voie ordinaire : refusé.
    assert!(matches!(
        ent.cloturer_scolarisation(role, cloture(etab, e.id, sortie, MotifSortie::Abandon, admin)).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.cloturer_scolarisation(role, cloture(etab, e.id, sortie, MotifSortie::Deces, admin)).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Entrée AVANT la sortie précédente : chevauchement refusé par la logique.
    let chevauche = ent
        .reintegrer_eleve(role, ouverture(etab, e.id, sortie - Duration::days(1), MotifEntree::Retour, admin))
        .await;
    assert!(matches!(chevauche, Err(EntDbError::ConflitDossier(_))), "{chevauche:?}");
    // ELV-12 : réintégration le lendemain, sur la même fiche.
    let retour = sortie + Duration::days(1);
    let mut cmd = ouverture(etab, e.id, retour, MotifEntree::Retour, admin);
    cmd.origine = EtablissementScolarite {
        uai: Some("0751234a".into()),
        libelle: Some("Collège voisin".into()),
        type_: Some(TypeEtablissement::Public),
    };
    let r2 = ent.reintegrer_eleve(role, cmd).await.unwrap();
    assert_eq!(r2.scolarisation.eleve_id, e.id);
    assert_eq!(r2.scolarisation.motif_entree, "retour");
    assert_eq!(r2.scolarisation.origine_uai.as_deref(), Some("0751234A"));
    assert_eq!(r2.statut, "a_venir", "le retour est demain");
    let (scol, _) = {
        let (_, s, i) = ent.scolarite_pour_acteur(etab, admin, role, e.id).await.unwrap();
        (s, i)
    };
    assert_eq!(scol.len(), 2, "deux séjours sur une seule fiche");
    // Événements : réservés enfin émis, sans donnée d'identité.
    let evs = types_evenements(&ent, e.id).await;
    for t in ["scolarite.closed", "eleve.sortie", "scolarite.opened", "eleve.reintegre", "eleve.status_changed"] {
        assert!(evs.iter().any(|x| x == t), "événement manquant : {t} dans {evs:?}");
    }
    let (payloads,): (serde_json::Value,) = sqlx::query_as(
        "SELECT jsonb_agg(payload) FROM ent_metier_event WHERE eleve_id = $1 AND type LIKE 'scolarite.%'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(!payloads.to_string().contains("Zélie") && !payloads.to_string().contains("Collège voisin"));
    // Filet de la base : chevauchement et double ouverture refusés même par SQL direct.
    let direct = sqlx::query(
        "INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree, date_sortie)
         VALUES (gen_random_uuid(), $1, $2, $3, 'reprise', $4)",
    )
    .bind(etab)
    .bind(e.id)
    .bind(sortie - Duration::days(5))
    .bind(sortie + Duration::days(5))
    .execute(ent.pool())
    .await;
    assert!(direct.is_err(), "chevauchement accepté par la base");
    let ouvertes = sqlx::query(
        "INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree)
         VALUES (gen_random_uuid(), $1, $2, $3, 'reprise')",
    )
    .bind(etab)
    .bind(e.id)
    .bind(retour + Duration::days(400))
    .execute(ent.pool())
    .await;
    assert!(ouvertes.is_err(), "deux scolarisations ouvertes acceptées par la base");
    // Une scolarisation ne se supprime pas.
    let suppr = sqlx::query("DELETE FROM ent_eleve_scolarisation WHERE eleve_id = $1").bind(e.id).execute(ent.pool()).await;
    assert!(suppr.is_err());
}

/// Droits : la scolarité s'écrit avec `eleves.scolarite.gerer`, la sortie avec `eleves.sortie`
/// (jamais le CPE, le professeur ni la famille) ; lecture G1 réservée.
#[tokio::test]
async fn droits_scolarite_par_role() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab).await;
    for (u, r) in [
        (cast.cpe.user_id, Role::Cpe),
        (cast.prof.user_id, Role::Prof),
        (cast.compta.user_id, Role::Compta),
        (cast.parent_lea.user_id, Role::Parent),
    ] {
        let o = ent.ouvrir_scolarisation(r.as_str(), ouverture(etab, e.id, aujourdhui(), MotifEntree::Reprise, u)).await;
        assert!(matches!(o, Err(EntDbError::Forbidden)), "{r:?}: {o:?}");
        let c = ent.cloturer_scolarisation(r.as_str(), cloture(etab, e.id, aujourdhui(), MotifSortie::Autre, u)).await;
        assert!(matches!(c, Err(EntDbError::Forbidden)), "{r:?}: {c:?}");
        let d = ent.enregistrer_deces(r.as_str(), etab, e.id, aujourdhui(), u).await;
        assert!(matches!(d, Err(EntDbError::Forbidden)), "{r:?}: {d:?}");
        let l = ent.scolarite_pour_acteur(etab, u, r.as_str(), e.id).await;
        assert!(matches!(l, Err(EntDbError::Forbidden)), "{r:?}: {l:?}");
    }
    // Rien n'a été écrit par ces refus.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_scolarisation WHERE eleve_id = $1").bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    // Autre établissement : élève introuvable, jamais d'information.
    let autre = Uuid::new_v4();
    let ea = eleve(&ent, autre).await;
    let x = ent.scolarite_pour_acteur(etab, cast.administratif.user_id, ADM_ROLE, ea.id).await;
    assert!(matches!(x, Err(EntDbError::Domain(DomainError::NotFound)) | Err(EntDbError::Forbidden)), "{x:?}");
    let y = ent
        .cloturer_scolarisation(ADM_ROLE, cloture(etab, ea.id, aujourdhui(), MotifSortie::Autre, cast.administratif.user_id))
        .await;
    assert!(y.is_err());
}

const ADM_ROLE: &str = "administratif";

/// ELV-34 : le décès clôt la scolarisation (motif `deces`), publie `eleve.decede` sans aucun
/// détail, et interdit la réintégration.
#[tokio::test]
async fn deces_ferme_la_scolarisation_sans_detail_et_interdit_le_retour() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.direction.user_id);
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(50)), None, None).await;
    let r = ent.enregistrer_deces(Role::Direction.as_str(), etab, e.id, aujourdhui(), admin).await.unwrap();
    assert_eq!(r.scolarisation.motif_sortie.as_deref(), Some("deces"));
    assert_eq!(r.statut, "sorti");
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.decede'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(payload.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["scolarisation_id"]);
    let retour = ent
        .reintegrer_eleve(Role::Direction.as_str(), ouverture(etab, e.id, aujourdhui() + Duration::days(1), MotifEntree::Retour, admin))
        .await;
    assert!(matches!(retour, Err(EntDbError::ConflitDossier(_))), "{retour:?}");
    // Même par une entrée ordinaire, ou par l'ancien chemin (réactivation) : refusé.
    let ordinaire = ent
        .ouvrir_scolarisation(Role::Direction.as_str(), ouverture(etab, e.id, aujourdhui() + Duration::days(1), MotifEntree::ChangementEtablissement, admin))
        .await;
    assert!(matches!(ordinaire, Err(EntDbError::ConflitDossier(_))), "{ordinaire:?}");
    let ancien = ent
        .change_status(ChangeStatus { etablissement_id: etab, eleve_id: e.id, nouveau_status: EleveStatus::Actif, actor_user_id: None })
        .await;
    assert!(ancien.is_err(), "réactivation d'un décédé par l'ancien chemin");
    // Le décès n'apparaît ni dans les événements de scolarité ni dans la colonne héritée.
    let (ev,): (serde_json::Value,) = sqlx::query_as(
        "SELECT jsonb_agg(payload) FROM ent_metier_event WHERE eleve_id = $1 AND type IN ('scolarite.closed','eleve.sortie')",
    ).bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert!(!ev.to_string().contains("deces"), "{ev}");
    let (col,): (Option<String>,) = sqlx::query_as("SELECT motif_sortie FROM ent_eleve WHERE id = $1").bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(col, None);
    // Le journal ne montre pas le décès à la famille.
    let (visible,): (bool,) = sqlx::query_as(
        "SELECT visible_famille FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'eleve_decede'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(!visible);
}

/// Sortie future : l'élève reste scolarisé jusqu'à la date ; la tâche quotidienne publie le
/// changement de statut quand la date est atteinte (simulée en rendant le cache périmé).
#[tokio::test]
async fn sortie_future_puis_tache_quotidienne() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(20)), None, None).await;
    let r = ent
        .cloturer_scolarisation(ADM_ROLE, cloture(etab, e.id, aujourdhui() + Duration::days(7), MotifSortie::FinDeCycle, admin))
        .await
        .unwrap();
    assert_eq!(r.statut, "scolarise", "pas encore sorti");
    assert!(!r.statut_change);
    // La date de sortie « arrive » : on avance la sortie à hier (avec la garde levée le temps du test).
    sqlx::query("UPDATE ent_eleve SET statut = 'scolarise' WHERE id = $1").bind(e.id).execute(ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_eleve_scolarisation SET date_sortie = $2 WHERE eleve_id = $1")
        .bind(e.id)
        .bind(aujourdhui() - Duration::days(1))
        .execute(ent.pool())
        .await
        .unwrap();
    // Le trigger a déjà projeté ; on remet un cache périmé pour exercer la tâche.
    sqlx::query("UPDATE ent_eleve SET statut = 'scolarise' WHERE id = $1").bind(e.id).execute(ent.pool()).await.unwrap();
    let bilan = ent.tache_nocturne_eleves().await.unwrap();
    assert!(bilan.statuts_changes >= 1, "{bilan:?}");
    assert!(bilan.repercussions_sortie >= 1, "{bilan:?}");
    assert_eq!(statut_cache(&ent, e.id).await.0.as_deref(), Some("sorti"));
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.status_changed' AND payload->>'cause' = 'tache_quotidienne'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Idempotente : une seconde exécution ne republie rien pour cet élève.
    ent.tache_nocturne_eleves().await.unwrap();
    let (n2,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.status_changed' AND payload->>'cause' = 'tache_quotidienne'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n2, 1);
}

/// Anciens chemins (`sortir`, `change_status`) : convertis en changement de la scolarisation,
/// le statut dérivé suit.
#[tokio::test]
async fn pont_des_chemins_herites_sortir_et_change_status() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(40)), None, Some("5A")).await;
    ent.sortir(SortirEleve {
        etablissement_id: etab,
        eleve_id: e.id,
        date_sortie: aujourdhui(),
        motif_sortie: Some("demenagement".into()),
        actor_user_id: None,
    })
    .await
    .unwrap();
    let (s, sort): (Option<String>, Option<NaiveDate>) =
        sqlx::query_as("SELECT statut, date_sortie FROM ent_eleve WHERE id = $1").bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(s.as_deref(), Some("sorti"));
    assert_eq!(sort, Some(aujourdhui()));
    let (motif, nb): (Option<String>, i64) = sqlx::query_as(
        "SELECT max(motif_sortie), count(*) FROM ent_eleve_scolarisation WHERE eleve_id = $1",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((motif.as_deref(), nb), (Some("demenagement"), 1));
    // Réactivation par l'ancien chemin : la scolarisation est rouverte.
    ent.change_status(ChangeStatus {
        etablissement_id: etab,
        eleve_id: e.id,
        nouveau_status: EleveStatus::Actif,
        actor_user_id: None,
    })
    .await
    .unwrap();
    let (s, ds): (Option<String>, Option<NaiveDate>) =
        sqlx::query_as("SELECT statut, date_sortie FROM ent_eleve WHERE id = $1").bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((s.as_deref(), ds), (Some("scolarise"), None));
}

/// M5 : la reprise est idempotente ; un élève « sorti » sans date reçoit la date de dernière
/// modification et un écart rapporté ; un « préinscrit » sans date n'a pas de scolarisation.
#[tokio::test]
async fn reprise_des_scolarisations_existantes() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (sorti, preinscrit) = (Uuid::new_v4(), Uuid::new_v4());
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_eleve DISABLE TRIGGER ent_eleve_scolarite_pont").execute(&mut *tx).await.unwrap();
    for (id, status) in [(sorti, "sorti"), (preinscrit, "preinscrit")] {
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at)
             VALUES ($1,$2,$3,'Hugo',$4, now() - interval '90 days', now() - interval '10 days')",
        )
        .bind(id)
        .bind(etab)
        .bind(format!("Reprise{}", alea()))
        .bind(status)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    // La création de la version d'identité (E2) touche `updated_at` : on le remet à l'état « ancien ».
    sqlx::query("UPDATE ent_eleve SET updated_at = now() - interval '10 days' WHERE id = ANY($1)")
        .bind(vec![sorti, preinscrit])
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_eleve ENABLE TRIGGER ent_eleve_scolarite_pont").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let (n1,): (i64,) = sqlx::query_as("SELECT ent_eleve_scolarisation_reprendre()").fetch_one(ent.pool()).await.unwrap();
    assert!(n1 >= 1);
    let (n2,): (i64,) = sqlx::query_as("SELECT ent_eleve_scolarisation_reprendre()").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n2, 0, "reprise idempotente");
    let (s, motif, d): (String, Option<String>, Option<NaiveDate>) = sqlx::query_as(
        "SELECT e.statut, s.motif_sortie, s.date_sortie FROM ent_eleve e
         JOIN ent_eleve_scolarisation s ON s.eleve_id = e.id WHERE e.id = $1",
    )
    .bind(sorti)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((s.as_str(), motif.as_deref()), ("sorti", Some("autre")));
    assert_eq!(d, Some(aujourdhui() - Duration::days(10)));
    let (ecart,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_scolarite_ecart WHERE eleve_id = $1 AND cause = 'sortie_sans_date'")
        .bind(sorti)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(ecart, 1);
    let (nb, st): (i64, Option<String>) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_eleve_scolarisation WHERE eleve_id = $1), (SELECT statut FROM ent_eleve WHERE id = $1)",
    )
    .bind(preinscrit)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((nb, st.as_deref()), (0, Some("a_venir")));
    ent.migrate().await.unwrap();
}

/// ELV-09 : une inscription par année, rattachée à la scolarisation qui couvre l'année ;
/// pont bidirectionnel avec l'ancienne table ; année hors scolarisation refusée.
#[tokio::test]
async fn inscription_annuelle_et_pont_avec_l_ancienne_table() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(30)), None, None).await;
    let (annee,): (Uuid,) = sqlx::query_as("SELECT ent_assurer_annee($1, current_date)").bind(etab).fetch_one(ent.pool()).await.unwrap();
    let cmd = |etat: EtatInscription| InscriptionAnnuelle {
        etablissement_id: etab,
        eleve_id: e.id,
        annee_id: annee,
        niveau_id: None,
        mef_id: None,
        statut_apprenant: StatutApprenant::Scolaire,
        etat,
        redoublement: Some(false),
        source: "saisie".into(),
        acteur: Some(admin),
    };
    let i1 = ent.enregistrer_inscription_annuelle(ADM_ROLE, cmd(EtatInscription::Confirmee)).await.unwrap();
    let i2 = ent.enregistrer_inscription_annuelle(ADM_ROLE, cmd(EtatInscription::EnCours)).await.unwrap();
    assert_eq!(i1.id, i2.id, "une seule inscription par année");
    assert_eq!(i2.etat, "en_cours");
    // Pont vers l'ancienne table (les lecteurs hérités la voient).
    let (label,): (String,) = sqlx::query_as("SELECT libelle FROM ent_annee_scolaire WHERE id = $1").bind(annee).fetch_one(ent.pool()).await.unwrap();
    let (st,): (String,) = sqlx::query_as(
        "SELECT statut FROM ent_eleve_scolarite_annee WHERE eleve_id = $1 AND annee_scolaire = $2",
    )
    .bind(e.id)
    .bind(&label)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(st, "courante");
    // Ancienne table → nouvelle : une ligne héritée devient une inscription (année suivante).
    let y = aujourdhui().format("%Y").to_string().parse::<i32>().unwrap() + 1;
    let suivante = format!("{y}-{}", y + 1);
    let e2 = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(30)), None, None).await;
    sqlx::query(
        "INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, niveau, statut, source)
         VALUES (gen_random_uuid(), $1, $2, $3, 'INCONNU', 'projetee', 'montee_rapide')",
    )
    .bind(etab)
    .bind(e2.id)
    .bind(&label)
    .execute(ent.pool())
    .await
    .unwrap();
    let (etat, source): (String, String) = sqlx::query_as(
        "SELECT etat, source FROM ent_eleve_inscription WHERE eleve_id = $1",
    )
    .bind(e2.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((etat.as_str(), source.as_str()), ("prevue", "montee"));
    let (a_traiter,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_inscription_a_traiter WHERE eleve_id = $1 AND cause = 'niveau_inconnu'")
        .bind(e2.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(a_traiter, 1, "niveau inconnu rapporté, ligne reprise quand même");
    // Année texte illisible : rangée « à traiter », rien de supprimé.
    sqlx::query(
        "INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, statut, source)
         VALUES (gen_random_uuid(), $1, $2, 'cette annee', 'historique', 'saisie')",
    )
    .bind(etab)
    .bind(e2.id)
    .execute(ent.pool())
    .await
    .unwrap();
    let (illisible,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_inscription_a_traiter WHERE eleve_id = $1 AND cause = 'annee_illisible'")
        .bind(e2.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(illisible, 1);
    let (n,): (i64,) = sqlx::query_as("SELECT ent_eleve_inscription_reprendre()").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 0, "reprise idempotente");
    // Année où l'élève n'est pas scolarisé : refusée.
    let (passee,): (Uuid,) = sqlx::query_as("SELECT ent_assurer_annee($1, (current_date - interval '5 years')::date)")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let hors = ent.enregistrer_inscription_annuelle(ADM_ROLE, InscriptionAnnuelle { annee_id: passee, ..cmd(EtatInscription::Prevue) }).await;
    assert!(matches!(hors, Err(EntDbError::ConflitDossier(_))), "{hors:?}");
    let _ = suivante;
}

/// ELV-09 : changement de classe daté (période + affectation du référentiel) ; classe d'un
/// autre établissement ou hors année refusée ; options = groupes (ajout, retrait) et pont des
/// anciennes options.
#[tokio::test]
async fn classe_et_options_par_affectations_du_referentiel() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let (c1, c2, annee): (Uuid, Uuid, Uuid) = {
        let code1 = format!("T{}", alea().to_uppercase());
        let code2 = format!("U{}", alea().to_uppercase());
        let (a,): (Uuid,) = sqlx::query_as("SELECT ent_assurer_annee($1, current_date)").bind(etab).fetch_one(ent.pool()).await.unwrap();
        let (x,): (Option<Uuid>,) = sqlx::query_as("SELECT ent_resoudre_classe($1, $2, current_date)").bind(etab).bind(&code1).fetch_one(ent.pool()).await.unwrap();
        let (y,): (Option<Uuid>,) = sqlx::query_as("SELECT ent_resoudre_classe($1, $2, current_date)").bind(etab).bind(&code2).fetch_one(ent.pool()).await.unwrap();
        (x.unwrap(), y.unwrap(), a)
    };
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(5)), None, None).await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    ent.changer_classe(ADM_ROLE, ChangementClasse { etablissement_id: etab, eleve_id: e.id, classe_id: c1, date_effet: jour - Duration::days(3), acteur: admin })
        .await
        .unwrap();
    ent.changer_classe(ADM_ROLE, ChangementClasse { etablissement_id: etab, eleve_id: e.id, classe_id: c2, date_effet: jour, acteur: admin })
        .await
        .unwrap();
    let aff: Vec<(Uuid, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT classe_id, date_debut, date_fin FROM ent_affectation
         WHERE eleve_id = $1 AND classe_id IS NOT NULL ORDER BY date_debut",
    )
    .bind(e.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(aff.len(), 2, "{aff:?}");
    assert_eq!((aff[0].0, aff[0].2), (c1, Some(jour - Duration::days(1))));
    assert_eq!((aff[1].0, aff[1].2), (c2, None));
    let (classe_txt,): (Option<String>,) = sqlx::query_as("SELECT classe FROM ent_eleve WHERE id = $1").bind(e.id).fetch_one(ent.pool()).await.unwrap();
    assert!(classe_txt.is_some());
    assert!(types_evenements(&ent, e.id).await.iter().any(|t| t == "scolarite.classe_changed"));
    // Classe inconnue ou d'un autre établissement : refusée.
    let inconnue = ent
        .changer_classe(ADM_ROLE, ChangementClasse { etablissement_id: etab, eleve_id: e.id, classe_id: Uuid::new_v4(), date_effet: jour, acteur: admin })
        .await;
    assert!(matches!(inconnue, Err(EntDbError::SaisieInvalide(_))), "{inconnue:?}");
    // Date hors de l'année de la classe : refusée.
    let hors = ent
        .changer_classe(ADM_ROLE, ChangementClasse { etablissement_id: etab, eleve_id: e.id, classe_id: c1, date_effet: jour - Duration::days(900), acteur: admin })
        .await;
    assert!(matches!(hors, Err(EntDbError::SaisieInvalide(_))), "{hors:?}");
    // — Options = groupes du référentiel —
    let (g1,): (Uuid,) = sqlx::query_as("SELECT ent_eleve_option_groupe_assurer($1, $2, $3, 'Latin')")
        .bind(etab)
        .bind(annee)
        .bind(format!("LAT{}", alea().to_uppercase()))
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let opts = |ajouter: Vec<Uuid>, retirer: Vec<Uuid>, d: NaiveDate| ChangementOptions {
        etablissement_id: etab,
        eleve_id: e.id,
        annee_id: annee,
        ajouter,
        retirer,
        date_effet: d,
        acteur: admin,
    };
    ent.changer_options(ADM_ROLE, opts(vec![g1], vec![], jour - Duration::days(2))).await.unwrap();
    ent.changer_options(ADM_ROLE, opts(vec![g1], vec![], jour - Duration::days(2))).await.unwrap(); // idempotent
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_affectation WHERE eleve_id = $1 AND groupe_id = $2").bind(e.id).bind(g1).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    ent.changer_options(ADM_ROLE, opts(vec![], vec![g1], jour)).await.unwrap();
    let (fin,): (Option<NaiveDate>,) = sqlx::query_as("SELECT date_fin FROM ent_affectation WHERE eleve_id = $1 AND groupe_id = $2").bind(e.id).bind(g1).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(fin, Some(jour - Duration::days(1)));
    let vide = ent.changer_options(ADM_ROLE, opts(vec![], vec![], jour)).await;
    assert!(matches!(vide, Err(EntDbError::SaisieInvalide(_))));
    let inconnu = ent.changer_options(ADM_ROLE, opts(vec![Uuid::new_v4()], vec![], jour)).await;
    assert!(matches!(inconnu, Err(EntDbError::SaisieInvalide(_))));
    // Pont : une ancienne option devient une affectation de groupe ; la reprise est idempotente.
    let opt_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_option (id, etablissement_id, eleve_id, nature, code, libelle, date_debut)
         VALUES ($1,$2,$3,'academique',$4,'Théâtre', current_date)",
    )
    .bind(opt_id)
    .bind(etab)
    .bind(e.id)
    .bind(format!("THE{}", alea().to_uppercase()))
    .execute(ent.pool())
    .await
    .unwrap();
    let (lien,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_option_groupe WHERE source = 'option' AND option_id = $1").bind(opt_id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(lien, 1);
    let (r,): (i64,) = sqlx::query_as("SELECT ent_eleve_option_reprendre()").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(r, 0);
    let (herit, reprises, a_traiter): (i64, i64, i64) =
        sqlx::query_as("SELECT heritees, reprises, a_traiter FROM ent_eleve_option_comparaison($1)").bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert!(herit > 0 && reprises + a_traiter >= herit, "comparaison avant/après : {herit} {reprises} {a_traiter}");
}

/// ELV-10 / R-E5 : régime et site datés ; dates autorisées par l'établissement, dérogation
/// motivée sinon (tracée sans détail libre).
#[tokio::test]
async fn regime_dates_autorisees_et_derogation() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(60)), None, None).await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    let chg = |valeur: &str, d: NaiveDate, der: Option<MotifDerogationRegime>| ChangementRegimeSite {
        etablissement_id: etab,
        eleve_id: e.id,
        kind: domain_eleve::AttributKind::Regime,
        valeur: valeur.into(),
        date_effet: d,
        derogation: der,
        acteur: admin,
    };
    // Par défaut : libre.
    ent.changer_regime_ou_site(ADM_ROLE, chg("demi-pensionnaire", jour - Duration::days(30), None)).await.unwrap();
    // Règle « liste » : seules les dates listées sont autorisées sans dérogation.
    let autorisee = jour + Duration::days(20);
    sqlx::query("INSERT INTO ent_param_eleve_regime (etablissement_id, dates_changement, dates) VALUES ($1, 'liste', ARRAY[$2::date])
                 ON CONFLICT (etablissement_id) DO UPDATE SET dates_changement = 'liste', dates = ARRAY[$2::date]")
        .bind(etab)
        .bind(autorisee)
        .execute(ent.pool())
        .await
        .unwrap();
    assert_eq!(ent.prochaine_date_changement_regime(etab, jour).await.unwrap(), Some(autorisee));
    let refus = ent.changer_regime_ou_site(ADM_ROLE, chg("interne", jour, None)).await;
    assert!(matches!(refus, Err(EntDbError::ConflitDossier(_))), "{refus:?}");
    ent.changer_regime_ou_site(ADM_ROLE, chg("interne", jour, Some(MotifDerogationRegime::Demenagement))).await.unwrap();
    let (motif,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'regime_derogation'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(motif["motif"], "demenagement");
    // Date autorisée : sans dérogation.
    ent.changer_regime_ou_site(ADM_ROLE, chg("externe", autorisee, None)).await.unwrap();
    let periodes: Vec<(String, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT valeur, date_debut, date_fin FROM ent_eleve_periode WHERE eleve_id = $1 AND kind = 'regime' ORDER BY date_debut",
    )
    .bind(e.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(periodes.len(), 3, "{periodes:?}");
    assert_eq!(periodes[2], ("externe".to_string(), autorisee, None));
    assert!(types_evenements(&ent, e.id).await.iter().any(|t| t == "eleve.regime_changed"));
    // Fin de trimestre : le changement prend effet le lendemain de la fin du trimestre.
    sqlx::query("UPDATE ent_param_eleve_regime SET dates_changement = 'fin_de_trimestre' WHERE etablissement_id = $1").bind(etab).execute(ent.pool()).await.unwrap();
    let (an,): (Uuid,) = sqlx::query_as("SELECT ent_assurer_annee($1, $2)").bind(etab).bind(jour).fetch_one(ent.pool()).await.unwrap();
    let fin_t = jour + Duration::days(40);
    sqlx::query("INSERT INTO ent_periode (id, etablissement_id, annee_id, code, libelle, kind, date_debut, date_fin) VALUES (gen_random_uuid(), $1, $2, $3, 'T', 'trimestre', $4, $5)")
        .bind(etab).bind(an).bind(format!("T{}", alea())).bind(jour - Duration::days(50)).bind(fin_t).execute(ent.pool()).await.unwrap();
    assert!(ent.prochaine_date_changement_regime(etab, jour).await.unwrap().is_some());
    // Site : jamais soumis aux dates de régime ; valeur vide ou trop longue refusée.
    let site = ChangementRegimeSite { kind: domain_eleve::AttributKind::Site, ..chg("annexe", jour, None) };
    ent.changer_regime_ou_site(ADM_ROLE, site).await.unwrap();
    let vide = ent.changer_regime_ou_site(ADM_ROLE, chg("  ", jour, None)).await;
    assert!(matches!(vide, Err(EntDbError::SaisieInvalide(_))));
    // Rôle insuffisant.
    let cpe = ent.changer_regime_ou_site(Role::Cpe.as_str(), ChangementRegimeSite { acteur: cast.cpe.user_id, ..chg("externe", jour, None) }).await;
    assert!(matches!(cpe, Err(EntDbError::Forbidden)));
    sqlx::query("DELETE FROM ent_param_eleve_regime WHERE etablissement_id = $1").bind(etab).execute(ent.pool()).await.unwrap();
}

/// ELV-13 : capacité juridique avec pièce obligatoire, une mesure active par nature, levée
/// conservée, mesure immuable ; jamais de motif en clair dans les traces.
#[tokio::test]
async fn capacite_juridique() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve(&ent, etab).await;
    let autre = eleve(&ent, etab).await;
    let p = piece(&ent, etab, e.id).await;
    let decl = |nature: NatureCapacite, piece_id: Uuid| DeclarationCapacite {
        etablissement_id: etab,
        eleve_id: e.id,
        nature,
        du: aujourdhui(),
        piece_document_id: piece_id,
        motif: Some("jugement du tribunal".into()),
        acteur: admin,
    };
    // Pièce d'un AUTRE élève : refusée.
    let p_autre = piece(&ent, etab, autre.id).await;
    let x = ent.declarer_capacite(ADM_ROLE, decl(NatureCapacite::Emancipation, p_autre)).await;
    assert!(matches!(x, Err(EntDbError::SaisieInvalide(_))), "{x:?}");
    let c = ent.declarer_capacite(ADM_ROLE, decl(NatureCapacite::Emancipation, p)).await.unwrap();
    assert!(c.levee_le.is_none());
    let doublon = ent.declarer_capacite(ADM_ROLE, decl(NatureCapacite::Emancipation, p)).await;
    assert!(matches!(doublon, Err(EntDbError::ConflitDossier(_))), "{doublon:?}");
    // Autre nature : possible.
    ent.declarer_capacite(ADM_ROLE, decl(NatureCapacite::Curatelle, p)).await.unwrap();
    // Mesure immuable, jamais supprimée.
    assert!(sqlx::query("UPDATE ent_eleve_capacite SET nature = 'tutelle' WHERE id = $1").bind(c.id).execute(ent.pool()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_eleve_capacite WHERE id = $1").bind(c.id).execute(ent.pool()).await.is_err());
    let levee = ent.lever_capacite(ADM_ROLE, etab, e.id, c.id, None, admin).await.unwrap();
    assert!(levee.levee_le.is_some() && levee.au.is_some());
    assert!(matches!(ent.lever_capacite(ADM_ROLE, etab, e.id, c.id, None, admin).await, Err(EntDbError::ConflitDossier(_))));
    // Après levée, la même nature peut être redéclarée.
    ent.declarer_capacite(ADM_ROLE, decl(NatureCapacite::Emancipation, p)).await.unwrap();
    // Traces sans motif.
    let (t,): (serde_json::Value,) = sqlx::query_as(
        "SELECT jsonb_agg(payload) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.capacite_changee'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(!t.to_string().contains("tribunal"));
    let (j,): (bool,) = sqlx::query_as("SELECT bool_or(visible_famille) FROM ent_journal_modif WHERE eleve_id = $1 AND action LIKE 'capacite_%'")
        .bind(e.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(!j);
    // Droits : le CPE ne déclare rien.
    let cpe = ent.declarer_capacite(Role::Cpe.as_str(), DeclarationCapacite { acteur: cast.cpe.user_id, ..decl(NatureCapacite::Tutelle, p) }).await;
    assert!(matches!(cpe, Err(EntDbError::Forbidden)));
}

/// ELV-35 / ELV-36 : mobilité (la scolarisation n'est PAS close) et modalités sans chevauchement.
#[tokio::test]
async fn mobilite_et_modalites() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let e = eleve_le(&ent, etab, Some(aujourdhui() - Duration::days(40)), None, None).await;
    let du = aujourdhui() - Duration::days(2);
    let m = ent.ouvrir_mobilite(ADM_ROLE, etab, e.id, TypeMobilite::Etranger, du, Some(aujourdhui() + Duration::days(200)), admin).await.unwrap();
    assert_eq!(m.type_, "etranger");
    assert_eq!(statut_cache(&ent, e.id).await.0.as_deref(), Some("scolarise"), "la mobilité ne clôt rien");
    let chevauche = ent.ouvrir_mobilite(ADM_ROLE, etab, e.id, TypeMobilite::Soins, aujourdhui(), None, admin).await;
    assert!(matches!(chevauche, Err(EntDbError::ConflitDossier(_))), "{chevauche:?}");
    ent.fermer_mobilite(ADM_ROLE, etab, e.id, m.id, aujourdhui(), admin).await.unwrap();
    assert!(matches!(ent.fermer_mobilite(ADM_ROLE, etab, e.id, m.id, aujourdhui(), admin).await, Err(EntDbError::ConflitDossier(_))));
    // Mobilité à une date où l'élève n'est pas scolarisé : refusée.
    let avant = ent.ouvrir_mobilite(ADM_ROLE, etab, e.id, TypeMobilite::Cned, aujourdhui() - Duration::days(300), None, admin).await;
    assert!(matches!(avant, Err(EntDbError::ConflitDossier(_))), "{avant:?}");
    // Modalités : deux codes différents en même temps, mais pas deux fois le même.
    let u = ent.enregistrer_modalite(ADM_ROLE, etab, e.id, CodeModalite::Ulis, aujourdhui(), None, None, admin).await.unwrap();
    ent.enregistrer_modalite(ADM_ROLE, etab, e.id, CodeModalite::TempsPartiel, aujourdhui(), None, None, admin).await.unwrap();
    let double = ent.enregistrer_modalite(ADM_ROLE, etab, e.id, CodeModalite::Ulis, aujourdhui() + Duration::days(1), None, None, admin).await;
    assert!(matches!(double, Err(EntDbError::ConflitDossier(_))), "{double:?}");
    ent.terminer_modalite(ADM_ROLE, etab, e.id, u.id, aujourdhui(), admin).await.unwrap();
    // « sans détail » : le code de la modalité n'est ni dans l'événement ni dans le journal.
    let (ev, jr): (Option<serde_json::Value>, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT (SELECT jsonb_agg(payload) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'scolarite.modalite_changee'),
                (SELECT jsonb_agg(payload) FROM ent_journal_modif WHERE eleve_id = $1 AND action LIKE 'modalite_%')",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    for t in [ev.unwrap().to_string(), jr.unwrap().to_string()] {
        assert!(!t.contains("ulis") && !t.contains("temps_partiel"), "{t}");
    }
}

/// ELV-14 / ELV-39 : `eleve.majeur` le jour des 18 ans, `eleve.majorite_proche` à J-30, une
/// seule fois (marqueur), sans date de naissance dans la charge utile ; majorité jamais
/// calculée sur une date imprécise ou contestée ; rattrapage borné.
#[tokio::test]
async fn majorite_evenements_uniques_sans_date() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    let dix_huit = |d: NaiveDate| NaiveDate::from_ymd_opt(d.year() - 18, d.month(), d.day().min(28)).unwrap();
    use chrono::Datelike;
    let aujourd_hui = eleve_le(&ent, etab, Some(jour - Duration::days(400)), Some(dix_huit(jour)), None).await;
    let dans_10_jours = eleve_le(&ent, etab, Some(jour - Duration::days(400)), Some(dix_huit(jour + Duration::days(10))), None).await;
    let loin = eleve_le(&ent, etab, Some(jour - Duration::days(400)), Some(dix_huit(jour + Duration::days(200))), None).await;
    let ancien_majeur = eleve_le(&ent, etab, Some(jour - Duration::days(400)), Some(dix_huit(jour - Duration::days(300))), None).await;
    let b1 = ent.tache_nocturne_eleves().await.unwrap();
    assert!(b1.majeurs >= 1 && b1.majorites_proches >= 1, "{b1:?}");
    let compte = |id: Uuid, t: &'static str| {
        let pool = ent.pool().clone();
        async move {
            let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE eleve_id = $1 AND type = $2")
                .bind(id)
                .bind(t)
                .fetch_one(&pool)
                .await
                .unwrap();
            n
        }
    };
    assert_eq!(compte(aujourd_hui.id, "eleve.majeur").await, 1);
    assert_eq!(compte(dans_10_jours.id, "eleve.majorite_proche").await, 1);
    assert_eq!(compte(dans_10_jours.id, "eleve.majeur").await, 0);
    assert_eq!(compte(loin.id, "eleve.majorite_proche").await, 0);
    // Un élève majeur depuis longtemps : marqueur silencieux, aucun événement (pas d'inondation).
    assert_eq!(compte(ancien_majeur.id, "eleve.majeur").await, 0);
    let (silencieux,): (bool,) = sqlx::query_as("SELECT publie FROM ent_eleve_majorite_evenement WHERE eleve_id = $1 AND sorte = 'atteinte'")
        .bind(ancien_majeur.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(!silencieux);
    // Une seconde exécution ne republie rien.
    ent.tache_nocturne_eleves().await.unwrap();
    assert_eq!(compte(aujourd_hui.id, "eleve.majeur").await, 1);
    assert_eq!(compte(dans_10_jours.id, "eleve.majorite_proche").await, 1);
    // Aucune date dans la charge utile (elle révélerait la date de naissance).
    let (p,): (serde_json::Value,) = sqlx::query_as("SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.majeur'")
        .bind(aujourd_hui.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(p.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["date_presumee"]);
    // R-E1 : la fonction ne calcule rien sur une date imprécise ou contestée.
    for (prec, st) in [("annee", "declaree"), ("mois", "presumee"), ("jour", "contestee"), ("inconnue", "declaree")] {
        let (m,): (Option<NaiveDate>,) = sqlx::query_as("SELECT ent_eleve_majorite_le('2008-03-14', $1, $2)").bind(prec).bind(st).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(m, None, "{prec}/{st}");
    }
    let (m,): (Option<NaiveDate>,) = sqlx::query_as("SELECT ent_eleve_majorite_le('2008-03-14', 'jour', 'presumee')").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(m, NaiveDate::from_ymd_opt(2026, 3, 14));
    let (b,): (Option<NaiveDate>,) = sqlx::query_as("SELECT ent_eleve_majorite_le('2008-02-29', 'jour', 'declaree')").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(b, domain_eleve::date_majorite(NaiveDate::from_ymd_opt(2008, 2, 29), "jour", "declaree"));
}

/// R-E15 : « aujourd'hui » est le jour de l'ÉTABLISSEMENT. Deux établissements aux fuseaux
/// extrêmes (UTC+14 et UTC-11) ne voient jamais le même jour : le même élève devient majeur
/// chez l'un et seulement « proche » chez l'autre.
#[tokio::test]
async fn fuseau_de_l_etablissement_pour_la_majorite() {
    let Some((ent, _cast)) = labo().await else { return };
    let (est, ouest) = (Uuid::new_v4(), Uuid::new_v4());
    for (etab, fuseau) in [(est, "Pacific/Kiritimati"), (ouest, "Pacific/Pago_Pago")] {
        sqlx::query("INSERT INTO ent_param_eleve_fuseau (etablissement_id, fuseau) VALUES ($1, $2)")
            .bind(etab)
            .bind(fuseau)
            .execute(ent.pool())
            .await
            .unwrap();
    }
    let mauvais = sqlx::query("INSERT INTO ent_param_eleve_fuseau (etablissement_id, fuseau) VALUES ($1, 'Mars/Olympus')")
        .bind(Uuid::new_v4())
        .execute(ent.pool())
        .await;
    assert!(mauvais.is_err(), "fuseau inconnu accepté");
    let j_est = ent.jour_etablissement(est).await.unwrap();
    let j_ouest = ent.jour_etablissement(ouest).await.unwrap();
    assert!(j_est > j_ouest, "{j_est} {j_ouest}");
    use chrono::Datelike;
    let naissance = NaiveDate::from_ymd_opt(j_est.year() - 18, j_est.month(), j_est.day().min(28)).unwrap();
    // Le jour de l'établissement de l'est tombe le 28 au plus tard : pas de 29 février à gérer.
    let a = eleve_le(&ent, est, Some(j_est - Duration::days(500)), Some(naissance), None).await;
    let b = eleve_le(&ent, ouest, Some(j_ouest - Duration::days(500)), Some(naissance), None).await;
    let dues: Vec<(Uuid, String)> = sqlx::query_as("SELECT eleve_id, sorte FROM ent_eleve_majorites_a_traiter() WHERE eleve_id = ANY($1)")
        .bind(vec![a.id, b.id])
        .fetch_all(ent.pool())
        .await
        .unwrap();
    // `scolarise` est calculé au jour de chaque établissement (entrée il y a 500 jours).
    if j_est.day() <= 28 {
        assert!(dues.contains(&(a.id, "atteinte".to_string())), "{dues:?}");
        assert!(dues.contains(&(b.id, "proche".to_string())), "{dues:?}");
    }
}
