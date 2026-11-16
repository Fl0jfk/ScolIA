//! Socle 12, lot AS14b-bis (tableau de bord de la vie scolaire en panneaux) : exclus du jour, heure de bascule,
//! disposition personnelle. Chaque test travaille dans un établissement neuf (jeu de labo) ; les instants sont
//! donnés au service. Sans `ENT_DATABASE_URL` : ignorés par absence de base (le test échoue en CI).

use super::*;
use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use domain_eleve::DomainError;

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

fn invalide<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::SaisieInvalide(_)) | Err(EntDbError::Domain(DomainError::ValeurInvalide)))
}

async fn aujourdhui(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    ent.aujourdhui_etablissement(cast.etablissement_id).await.unwrap()
}

fn pref(ordre: &[&str]) -> Vec<PreferencePanneau> {
    ordre
        .iter()
        .map(|p| PreferencePanneau {
            panneau: (*p).into(),
            replie: *p == "incoherences",
            masque: *p == "familles",
        })
        .collect()
}

const ORDRE: [&str; 5] = ["exclus", "appels", "absents", "familles", "incoherences"];

#[tokio::test]
async fn tableau_exclus_du_jour_droits_urgence_et_arrivee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (jour, lea) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let c = ent
        .seed_creneau_du_jour(etab, "4B", "TDB", cast.prof.user_id, jour, t(8, 0), t(9, 0), None)
        .await
        .expect("créneau")
        .id;
    let (a, _) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.unwrap();
    ent.exclure_du_cours_for_actor(etab, a.id, lea, cast.prof.user_id, "prof", "web").await.unwrap();
    let t0 = Utc::now();

    // Droits : la vie scolaire (comme la file du matin) ; ni le professeur, ni une famille.
    assert!(interdit(ent.exclus_du_jour_for_actor(etab, cast.prof.user_id, "prof", t0).await));
    assert!(interdit(ent.exclus_du_jour_for_actor(etab, cast.parent_lea.user_id, "parent", t0).await));

    // Fraîchement exclu : dans le panneau, pas encore urgent.
    let r = ent.exclus_du_jour_for_actor(etab, cast.cpe.user_id, "cpe", t0 + Duration::minutes(2)).await.unwrap();
    assert_eq!(r.exclus.len(), 1);
    assert_eq!((r.exclus[0].eleve_id, r.exclus[0].appel_id), (lea, a.id));
    assert!(!r.exclus[0].urgent);
    // Dix minutes après : urgent (c'est aussi une ligne du bandeau de la file du matin).
    let r = ent.exclus_du_jour_for_actor(etab, cast.cpe.user_id, "cpe", t0 + Duration::minutes(11)).await.unwrap();
    assert!(r.exclus[0].urgent && r.exclus[0].minutes_depuis >= 10);
    // Arrivée confirmée : l'élève quitte le panneau.
    ent.confirmer_arrivee_vs_for_actor(etab, a.id, lea, cast.cpe.user_id, "cpe").await.unwrap();
    let r = ent.exclus_du_jour_for_actor(etab, cast.cpe.user_id, "cpe", t0 + Duration::minutes(12)).await.unwrap();
    assert!(r.exclus.is_empty());
}

#[tokio::test]
async fn tableau_heure_de_bascule_defaut_reglage_et_bornes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    // Défaut : 11 h, sans disposition (automatique).
    let c = ent.config_tableau_for_actor(etab, cast.cpe.user_id, "cpe").await.unwrap();
    assert_eq!((c.heure_bascule, c.preference), (t(11, 0), None));
    // Droits d'accès : ni professeur ni famille.
    assert!(interdit(ent.config_tableau_for_actor(etab, cast.prof.user_id, "prof").await));
    assert!(interdit(ent.config_tableau_for_actor(etab, cast.parent_lea.user_id, "parent").await));
    // Régler l'heure : direction seulement (comme la règle d'assiduité) ; ni la CPE, ni le professeur.
    assert!(interdit(ent.regler_heure_bascule_for_actor(etab, cast.cpe.user_id, "cpe", t(10, 0)).await));
    assert!(interdit(ent.regler_heure_bascule_for_actor(etab, cast.prof.user_id, "prof", t(10, 0)).await));
    // Bornes : 6 h à 18 h.
    for h in [t(5, 59), t(18, 1), t(0, 0)] {
        assert!(invalide(ent.regler_heure_bascule_for_actor(etab, cast.direction.user_id, "direction", h).await));
    }
    assert_eq!(ent.regler_heure_bascule_for_actor(etab, cast.direction.user_id, "direction", t(10, 30)).await.unwrap(), t(10, 30));
    // Idempotent et modifiable : une seule ligne par établissement.
    ent.regler_heure_bascule_for_actor(etab, cast.direction.user_id, "direction", t(12, 0)).await.unwrap();
    let c = ent.config_tableau_for_actor(etab, cast.cpe.user_id, "cpe").await.unwrap();
    assert_eq!(c.heure_bascule, t(12, 0));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_asd_tableau_reglage WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn tableau_disposition_personnelle_par_utilisateur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (cpe, dir) = (cast.cpe.user_id, cast.direction.user_id);
    // Droits : réservée à ceux qui voient le tableau.
    assert!(interdit(ent.poser_preference_tableau_for_actor(etab, cast.prof.user_id, "prof", pref(&ORDRE)).await));
    assert!(interdit(ent.effacer_preference_tableau_for_actor(etab, cast.parent_lea.user_id, "parent").await));
    // Valeurs refusées : panneau manquant, doublon, panneau inconnu (le bandeau Urgent n'est pas rangeable).
    assert!(invalide(ent.poser_preference_tableau_for_actor(etab, cpe, "cpe", pref(&ORDRE[..4])).await));
    assert!(invalide(ent.poser_preference_tableau_for_actor(etab, cpe, "cpe", pref(&["exclus", "exclus", "appels", "absents", "familles"])).await));
    assert!(invalide(ent.poser_preference_tableau_for_actor(etab, cpe, "cpe", pref(&["urgent", "appels", "absents", "familles", "incoherences"])).await));
    assert!(ent.config_tableau_for_actor(etab, cpe, "cpe").await.unwrap().preference.is_none(), "un refus n'écrit rien");
    // Épingler : l'ordre, le repli et le masquage reviennent tels quels, pour cet utilisateur seulement.
    ent.poser_preference_tableau_for_actor(etab, cpe, "cpe", pref(&ORDRE)).await.unwrap();
    assert_eq!(ent.config_tableau_for_actor(etab, cpe, "cpe").await.unwrap().preference, Some(pref(&ORDRE)));
    assert_eq!(ent.config_tableau_for_actor(etab, dir, "direction").await.unwrap().preference, None);
    // Remplacer (sans doublon de rang), puis revenir à « automatique » (idempotent).
    let autre = ["absents", "exclus", "appels", "incoherences", "familles"];
    ent.poser_preference_tableau_for_actor(etab, cpe, "cpe", pref(&autre)).await.unwrap();
    assert_eq!(ent.config_tableau_for_actor(etab, cpe, "cpe").await.unwrap().preference, Some(pref(&autre)));
    ent.effacer_preference_tableau_for_actor(etab, cpe, "cpe").await.unwrap();
    ent.effacer_preference_tableau_for_actor(etab, cpe, "cpe").await.unwrap();
    assert_eq!(ent.config_tableau_for_actor(etab, cpe, "cpe").await.unwrap().preference, None);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_asd_tableau_pref WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
}
