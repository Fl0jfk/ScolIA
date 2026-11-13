//! Socle 41 (internat), lots I0 et I1 : règles testées contre la base locale de test (feature
//! `labo`) ; sans `ENT_DATABASE_URL`, elles sont ignorées en local (échec explicite en CI).
//!
//! Chaque défaut du registre `ALERTES_NEXTJS_PROD.md` fermé par ces lots a au moins un test
//! nommé `nj_XX_…` (règle 14 de `PILOTAGE.md`, D-051) :
//!
//! | Défaut | Test(s) |
//! |---|---|
//! | NJ-16 (santé lisible par surveillants et secrétariat) | `nj_16_aucune_donnee_de_sante_dans_l_internat`, `nj_16_suivi_borne_et_sante_signalee` |
//! | NJ-21 (droits incohérents, appel pour n'importe quel élève) | `nj_21_droits_internat_refuses_hors_equipe_habilitee`, `nj_21_pointage_refuse_pour_un_eleve_non_heberge` |
//! | NJ-23 (deux appels divergents, écritures qui s'écrasent) | `nj_23_presence_en_ajout_seul_rien_ne_s_ecrase`, `nj_23_deux_surveillants_simultanes_conservent_leurs_pointages`, `nj_23_nuit_future_et_nuit_passee` |
//! | NJ-54 (affectation non atomique, sexe / régime / date non vérifiés) | `nj_54_affectation_refuse_hors_regime_public_et_capacite`, `nj_54_capacite_atomique_sous_concurrence`, `nj_54_interdiction_de_contact_et_derogation` |
//! | RI-03 (suivis privés lus par tout rôle qui lit les élèves) | `ri_03_suivis_prives_reserves_et_lecture_journalisee` |
//!
//! NJ-22 (jetons de sortie) et NJ-51 (confirmation par GET) relèvent des lots I3 et I8 : ils ne sont
//! **pas** fermés ici ; `ent-api` teste seulement qu'aucune route d'internat n'existe sans session.

use super::*;
use chrono::{Duration, NaiveDate};
use domain_eleve::{
    nuit_paris, AffecterEleveInternat, CreateEleve, CreerChambreInternat,
    CreerSuiviInternat, DomainError, PointerPresenceInternat, RegimeCode, CORRESPONDANCES_REGIME,
};
use ent_auth::Role;
use ent_types::internat::*;
use std::sync::Arc;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_| {}).await;
    Some((ent, cast))
}

fn alea() -> String {
    Uuid::new_v4().simple().to_string()[..6].replace(char::is_numeric, "x")
}

const CPE: &str = "cpe";
const DIR: &str = "direction";

/// Élève interne depuis longtemps (régime « interne » dès la création, période antérieure à toute
/// date de test) : la nuit courante peut être celle d'hier sans changer le résultat.
async fn eleve(ent: &PgEnt, etab: Uuid, sexe: &str, regime: &str) -> domain_eleve::Eleve {
    let jour = domain_eleve::horloge::jour_paris();
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Int{}", alea()),
        prenom: if sexe == "F" { "Zélie".into() } else { "Théo".into() },
        ine: None,
        sexe: Some(sexe.into()),
        date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: None,
        regime: Some(regime.into()),
        mef: None,
        site: None,
        date_entree: Some(jour - Duration::days(400)),
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

async fn chambre(ent: &PgEnt, etab: Uuid, cpe: Uuid, public: &str, capacite: i16) -> domain_eleve::ChambreInternat {
    ent.creer_chambre_internat_for_actor(
        CPE,
        CreerChambreInternat {
            etablissement_id: etab,
            batiment: "Tour test".into(),
            etage: 1,
            // 12 caractères aléatoires : sur la base partagée d'une tranche de CI, 6 caractères (7 lettres
            // possibles) entraient en collision de temps en temps (« Conflit » à la création).
            code: format!("T{}{}", alea(), alea()),
            capacite,
            genre: public.into(),
            actor_user_id: cpe,
        },
    )
    .await
    .unwrap()
}

async fn affecter(
    ent: &PgEnt,
    etab: Uuid,
    cpe: Uuid,
    eleve: Uuid,
    chambre: Uuid,
    debut: NaiveDate,
) -> Result<domain_eleve::AffectationInternat, EntDbError> {
    ent.affecter_eleve_internat_for_actor(
        CPE,
        AffecterEleveInternat {
            etablissement_id: etab,
            eleve_id: eleve,
            chambre_id: chambre,
            lit: None,
            debut,
            actor_user_id: cpe,
        },
    )
    .await
}

fn est(r: &Result<impl std::fmt::Debug, EntDbError>, attendu: DomainError) -> bool {
    matches!(r, Err(EntDbError::Domain(e)) if *e == attendu)
}

fn pointage(etab: Uuid, eleve: Uuid, jour: NaiveDate, statut: &str, par: Uuid) -> PointerPresenceInternat {
    PointerPresenceInternat {
        etablissement_id: etab,
        eleve_id: eleve,
        jour,
        statut: statut.into(),
        note: String::new(),
        actor_user_id: par,
        motif_correction_code: None,
        motif_correction_texte: None,
    }
}

// ───────────────────────── Régime codé, nuit métier ─────────────────────────

#[tokio::test]
async fn regime_code_table_de_correspondance_identique_au_code() {
    let Some((ent, _)) = labo().await else { return };
    let mut lignes: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT vocabulaire, valeur, code FROM ent_regime_correspondance ORDER BY vocabulaire, valeur",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    // Tri fait des deux côtés en Rust : la collation de la base ne doit pas décider de l'égalité.
    lignes.sort();
    let mut code: Vec<(String, String, String)> = CORRESPONDANCES_REGIME
        .iter()
        .map(|(v, x, c)| (v.to_string(), x.to_string(), c.as_str().to_string()))
        .collect();
    code.sort();
    assert_eq!(lignes, code, "la table SQL et RegimeCode::depuis_texte doivent coïncider (R-IT2)");
    for (_, valeur, c) in &code {
        let sql: Option<String> = sqlx::query_scalar("SELECT ent_regime_vers_code($1)")
            .bind(valeur)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        assert_eq!(sql.as_deref(), Some(c.as_str()), "{valeur}");
        assert_eq!(RegimeCode::depuis_texte(valeur).map(|r| r.as_str()), Some(c.as_str()));
    }
}

#[tokio::test]
async fn nuit_metier_sql_et_rust_identiques_aux_changements_d_heure() {
    use chrono::TimeZone;
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let instants = [
        (2026, 12, 10, 23, 30),
        (2026, 12, 11, 10, 59),
        (2026, 12, 11, 11, 0),
        (2027, 3, 27, 22, 0),
        (2027, 3, 28, 0, 30),
        (2027, 3, 28, 1, 30),
        (2027, 3, 28, 9, 59),
        (2027, 3, 28, 10, 0),
        (2027, 10, 30, 22, 30),
        (2027, 10, 31, 0, 30),
        (2027, 10, 31, 1, 30),
        (2027, 10, 31, 10, 59),
        (2027, 10, 31, 11, 0),
    ];
    for (y, m, d, h, mi) in instants {
        let t = chrono::Utc.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap();
        let sql: NaiveDate = sqlx::query_scalar("SELECT ent_nuit_etab($1, $2)")
            .bind(etab)
            .bind(t)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        assert_eq!(sql, nuit_paris(t), "{t}");
    }
}

// ───────────────────────── NJ-54 : affectation ─────────────────────────

#[tokio::test]
async fn nj_54_affectation_refuse_hors_regime_public_et_capacite() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let filles = chambre(&ent, etab, cpe, "filles", 1).await;

    // Régime : un demi-pensionnaire, un externe ne sont pas hébergés (R-IT1).
    let dp = eleve(&ent, etab, "F", "demi_pension").await;
    assert!(est(&affecter(&ent, etab, cpe, dp.id, filles.id, nuit).await, DomainError::InternatRegimeRequis));
    let ext = eleve(&ent, etab, "F", "externe").await;
    assert!(est(&affecter(&ent, etab, cpe, ext.id, filles.id, nuit).await, DomainError::InternatRegimeRequis));

    // Public : un garçon dans une chambre de filles est refusé (R-IT11) ; la direction peut déroger.
    let garcon = eleve(&ent, etab, "M", "interne").await;
    assert!(est(
        &affecter(&ent, etab, cpe, garcon.id, filles.id, nuit).await,
        DomainError::InternatPublicIncompatible
    ));
    // Seule la direction accorde la dérogation (le CPE est refusé).
    let derog = DerogationSaisieDto {
        eleve_id: garcon.id,
        nature: "public_chambre".into(),
        chambre_id: Some(filles.id),
        avec_eleve_id: None,
        du: nuit,
        au: None,
    };
    assert!(matches!(
        ent.accorder_derogation_internat_for_actor(etab, cpe, CPE, &derog).await,
        Err(EntDbError::Forbidden)
    ));
    ent.accorder_derogation_internat_for_actor(etab, cast.direction.user_id, DIR, &derog)
        .await
        .unwrap();
    let a = affecter(&ent, etab, cpe, garcon.id, filles.id, nuit).await.unwrap();
    assert!(a.lit_id.is_some() && a.sejour_id.is_some(), "lit attribué et séjour ouvert par le régime");

    // Capacité (1 lit) : une fille de plus est refusée.
    let fille = eleve(&ent, etab, "F", "interne").await;
    assert!(est(&affecter(&ent, etab, cpe, fille.id, filles.id, nuit).await, DomainError::InternatChambrePleine));

    // Une seule affectation ouverte par élève.
    let autre = chambre(&ent, etab, cpe, "garcons", 2).await;
    assert!(est(&affecter(&ent, etab, cpe, garcon.id, autre.id, nuit).await, DomainError::InternatDejaAffecte));

    // Chambre « mixte » : refusée à la création (R-IT11).
    let mixte = ent
        .creer_chambre_internat_for_actor(
            CPE,
            CreerChambreInternat {
                etablissement_id: etab,
                batiment: "X".into(),
                etage: 0,
                code: format!("M{}", alea()),
                capacite: 2,
                genre: "mixte".into(),
                actor_user_id: cpe,
            },
        )
        .await;
    assert!(est(&mixte, DomainError::InternatChambreInvalide));
}

#[tokio::test]
async fn nj_54_capacite_atomique_sous_concurrence() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 3).await;
    let mut ids = Vec::new();
    for _ in 0..10 {
        ids.push(eleve(&ent, etab, "M", "interne").await.id);
    }
    let ent = Arc::new(ent);
    let mut taches = Vec::new();
    for id in ids.clone() {
        let ent = ent.clone();
        taches.push(tokio::spawn(async move {
            affecter(&ent, etab, cpe, id, ch.id, nuit).await.map(|a| a.id)
        }));
    }
    let mut ok = 0;
    let mut pleines = 0;
    for t in taches {
        match t.await.unwrap() {
            Ok(_) => ok += 1,
            Err(EntDbError::Domain(DomainError::InternatChambrePleine))
            | Err(EntDbError::Domain(DomainError::InternatLitIndisponible)) => pleines += 1,
            Err(e) => panic!("erreur inattendue : {e:?}"),
        }
    }
    assert_eq!(ok, 3, "exactement la capacité de la chambre");
    assert_eq!(pleines, 7);
    let (ouvertes,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ent_internat_affectation WHERE etablissement_id = $1 AND chambre_id = $2 AND fin IS NULL",
    )
    .bind(etab)
    .bind(ch.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(ouvertes, 3);
    // Même hors service : la base refuse seule le dépassement (migration 2820).
    let intrus = eleve(&ent, etab, "M", "interne").await;
    let r = sqlx::query(
        "INSERT INTO ent_internat_affectation (id, etablissement_id, eleve_id, chambre_id, debut, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(intrus.id)
    .bind(ch.id)
    .bind(nuit)
    .bind(cpe)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "insertion directe au-delà de la capacité refusée par la base");
}

#[tokio::test]
async fn nj_54_interdiction_de_contact_et_derogation() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe, dir) = (cast.etablissement_id, cast.cpe.user_id, cast.direction.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 3).await;
    let (a, b) = (eleve(&ent, etab, "M", "interne").await, eleve(&ent, etab, "M", "interne").await);
    affecter(&ent, etab, cpe, a.id, ch.id, nuit).await.unwrap();
    // Seule la direction pose l'interdiction (port provisoire du socle 13).
    let interdit = InterdictionContactSaisieDto { eleve_a_id: a.id, eleve_b_id: b.id, du: nuit, au: None };
    assert!(matches!(
        ent.poser_interdiction_contact_internat_for_actor(etab, cpe, CPE, &interdit).await,
        Err(EntDbError::Forbidden)
    ));
    ent.poser_interdiction_contact_internat_for_actor(etab, dir, DIR, &interdit).await.unwrap();
    assert!(est(&affecter(&ent, etab, cpe, b.id, ch.id, nuit).await, DomainError::InternatInterdictionContact));
    // Autre chambre : possible.
    let autre = chambre(&ent, etab, cpe, "garcons", 2).await;
    affecter(&ent, etab, cpe, b.id, autre.id, nuit).await.unwrap();
    // Dérogation de la direction : pour b dans la chambre de a.
    let c = eleve(&ent, etab, "M", "interne").await;
    ent.poser_interdiction_contact_internat_for_actor(
        etab,
        dir,
        DIR,
        &InterdictionContactSaisieDto { eleve_a_id: a.id, eleve_b_id: c.id, du: nuit, au: None },
    )
    .await
    .unwrap();
    assert!(est(&affecter(&ent, etab, cpe, c.id, ch.id, nuit).await, DomainError::InternatInterdictionContact));
    ent.accorder_derogation_internat_for_actor(
        etab,
        dir,
        DIR,
        &DerogationSaisieDto {
            eleve_id: c.id,
            nature: "interdiction_contact".into(),
            chambre_id: None,
            avec_eleve_id: Some(a.id),
            du: nuit,
            au: None,
        },
    )
    .await
    .unwrap();
    affecter(&ent, etab, cpe, c.id, ch.id, nuit).await.unwrap();
}

// ───────────────────────── NJ-23 : présence en faits ─────────────────────────

#[tokio::test]
async fn nj_23_presence_en_ajout_seul_rien_ne_s_ecrase() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 2).await;
    let e = eleve(&ent, etab, "M", "interne").await;
    affecter(&ent, etab, cpe, e.id, ch.id, nuit - Duration::days(3)).await.unwrap();

    let p1 = ent
        .pointer_presence_internat_for_actor(CPE, pointage(etab, e.id, nuit, "present", cpe))
        .await
        .unwrap();
    assert_eq!(p1.statut.as_str(), "present");
    // Deuxième pointage de la même nuit : un NOUVEAU fait (l'ancien n'est pas écrasé).
    let p2 = ent
        .pointer_presence_internat_for_actor(CPE, pointage(etab, e.id, nuit, "absent", cpe))
        .await
        .unwrap();
    assert_eq!(p2.statut.as_str(), "absent", "la valeur courante est le fait le plus récent");
    let faits = ent.historique_presence_internat_for_actor(etab, CPE, e.id, nuit).await.unwrap();
    assert_eq!(faits.len(), 2, "les deux faits subsistent");
    assert_eq!(faits[0].statut.as_str(), "present");
    assert_eq!(faits[1].statut.as_str(), "absent");
    let courantes = ent.list_presences_internat_jour_for_actor(etab, CPE, nuit).await.unwrap();
    assert_eq!(courantes.iter().filter(|p| p.eleve_id == e.id).count(), 1);

    // La base refuse toute modification ou suppression d'un fait (ajout seul).
    for sql in [
        "UPDATE ent_internat_presence_fait SET marque = 'present' WHERE etablissement_id = $1",
        "DELETE FROM ent_internat_presence_fait WHERE etablissement_id = $1",
    ] {
        let r = sqlx::query(sql).bind(etab).execute(ent.pool_verif()).await;
        let code = match &r {
            Err(sqlx::Error::Database(d)) => d.code().map(|c| c.to_string()),
            _ => None,
        };
        assert_eq!(code.as_deref(), Some("42501"), "{sql} : {r:?}");
    }
    // L'ancienne table n'accepte plus aucune écriture.
    let r = sqlx::query(
        "INSERT INTO ent_internat_presence (id, etablissement_id, eleve_id, affectation_id, jour, statut, actor_user_id)
         SELECT gen_random_uuid(), etablissement_id, eleve_id, id, $2, 'present', created_by_user_id
           FROM ent_internat_affectation WHERE etablissement_id = $1 AND eleve_id = $3 LIMIT 1",
    )
    .bind(etab)
    .bind(nuit)
    .bind(e.id)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "ent_internat_presence est en lecture seule");
}

#[tokio::test]
async fn nj_23_deux_surveillants_simultanes_conservent_leurs_pointages() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 2).await;
    let e = eleve(&ent, etab, "M", "interne").await;
    affecter(&ent, etab, cpe, e.id, ch.id, nuit).await.unwrap();
    let ent = Arc::new(ent);
    let (a, b) = {
        let (e1, e2) = (ent.clone(), ent.clone());
        let t1 = tokio::spawn(async move {
            e1.pointer_presence_internat_for_actor(CPE, pointage(etab, e.id, nuit, "present", cpe)).await
        });
        let t2 = tokio::spawn(async move {
            e2.pointer_presence_internat_for_actor(CPE, pointage(etab, e.id, nuit, "retard", cpe)).await
        });
        (t1.await.unwrap(), t2.await.unwrap())
    };
    a.unwrap();
    b.unwrap();
    let faits = ent.historique_presence_internat_for_actor(etab, CPE, e.id, nuit).await.unwrap();
    assert_eq!(faits.len(), 2, "aucun pointage perdu (corrige l'écrasement de NJ-23)");
}

#[tokio::test]
async fn nj_23_nuit_future_et_nuit_passee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 2).await;
    let e = eleve(&ent, etab, "M", "interne").await;
    affecter(&ent, etab, cpe, e.id, ch.id, nuit - Duration::days(5)).await.unwrap();

    // Futur : refusé.
    let r = ent
        .pointer_presence_internat_for_actor(CPE, pointage(etab, e.id, nuit + Duration::days(1), "present", cpe))
        .await;
    assert!(est(&r, DomainError::InternatNuitFuture), "{r:?}");
    // Passé sans motif : refusé ; avec motif codé : accepté ; motif inconnu : refusé.
    let passe = nuit - Duration::days(2);
    let r = ent.pointer_presence_internat_for_actor(CPE, pointage(etab, e.id, passe, "present", cpe)).await;
    assert!(est(&r, DomainError::InternatMotifRequis), "{r:?}");
    let mut avec = pointage(etab, e.id, passe, "present", cpe);
    avec.motif_correction_code = Some("oubli".into());
    avec.motif_correction_texte = Some("Appel du soir oublié".into());
    let p = ent.pointer_presence_internat_for_actor(CPE, avec).await.unwrap();
    assert_eq!(p.motif_correction_code.as_deref(), Some("oubli"));
    let mut faux = pointage(etab, e.id, passe, "present", cpe);
    faux.motif_correction_code = Some("n_importe_quoi".into());
    assert!(est(&ent.pointer_presence_internat_for_actor(CPE, faux).await, DomainError::InternatPresenceInvalide));
    // La base garde aussi la règle (insertion directe d'une nuit future).
    let r = sqlx::query(
        "INSERT INTO ent_internat_presence_fait (id, etablissement_id, eleve_id, nuit, marque, par_user_id)
         VALUES ($1, $2, $3, $4, 'present', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(e.id)
    .bind(nuit + Duration::days(2))
    .bind(cpe)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "nuit future refusée par la base");
}

#[tokio::test]
async fn nj_21_pointage_refuse_pour_un_eleve_non_heberge() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    // L'appel typé acceptait « une marque pour n'importe quel élève de l'établissement » (DI-19).
    let ext = eleve(&ent, etab, "M", "externe").await;
    let r = ent.pointer_presence_internat_for_actor(CPE, pointage(etab, ext.id, nuit, "present", cpe)).await;
    assert!(est(&r, DomainError::InternatAffectationInvalide), "{r:?}");
    // Un interne dont l'affectation a pris fin n'est plus pointable.
    let ch = chambre(&ent, etab, cpe, "garcons", 2).await;
    let e = eleve(&ent, etab, "M", "interne").await;
    let a = affecter(&ent, etab, cpe, e.id, ch.id, nuit - Duration::days(4)).await.unwrap();
    ent.clore_affectation_internat_for_actor(etab, CPE, a.id, nuit - Duration::days(2), cpe).await.unwrap();
    let mut p = pointage(etab, e.id, nuit, "present", cpe);
    p.motif_correction_code = None;
    assert!(est(&ent.pointer_presence_internat_for_actor(CPE, p).await, DomainError::InternatAffectationInvalide));
}

// ───────────────────────── NJ-21, RI-03, NJ-16 : droits et santé ─────────────────────────

#[tokio::test]
async fn nj_21_droits_internat_refuses_hors_equipe_habilitee() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let e = eleve(&ent, etab, "M", "interne").await;
    let hors_equipe = [
        (Role::Prof, cast.prof.user_id),
        (Role::Infirmerie, cast.infirmieres[0].user_id),
        (Role::Psy, cast.psys[0].user_id),
        (Role::Compta, cast.compta.user_id),
        (Role::Accueil, Uuid::new_v4()),
        (Role::Personnel, Uuid::new_v4()),
        (Role::Parent, cast.parent_lea.user_id),
    ];
    for (role, user) in hors_equipe {
        let r = role.as_str();
        assert!(!PgEnt::role_can_gerer_internat(r), "{r}");
        assert!(matches!(
            ent.list_chambres_internat_for_actor(etab, r).await,
            Err(EntDbError::Forbidden)
        ), "{r} : liste des chambres");
        assert!(matches!(
            ent.list_presences_internat_jour_for_actor(etab, r, nuit).await,
            Err(EntDbError::Forbidden)
        ), "{r} : présences");
        assert!(matches!(
            ent.list_affectations_internat_jour_for_actor(etab, r, nuit).await,
            Err(EntDbError::Forbidden)
        ), "{r} : affectations");
        assert!(matches!(
            ent.pointer_presence_internat_for_actor(r, pointage(etab, e.id, nuit, "present", user)).await,
            Err(EntDbError::Forbidden)
        ), "{r} : pointage");
        assert!(matches!(
            ent.creer_suivi_internat_for_actor(
                r,
                CreerSuiviInternat {
                    etablissement_id: etab,
                    eleve_id: e.id,
                    jour: nuit,
                    texte: "x".into(),
                    visible_famille: false,
                    actor_user_id: user,
                },
            )
            .await,
            Err(EntDbError::Forbidden)
        ), "{r} : suivi");
        // Hébergement (lot I1) : mêmes refus.
        assert!(matches!(ent.plan_internat_for_actor(etab, user, r, nuit).await, Err(EntDbError::Forbidden)), "{r} : plan");
        assert!(matches!(ent.lister_zones_internat_for_actor(etab, user, r).await, Err(EntDbError::Forbidden)), "{r} : zones");
        assert!(matches!(ent.parametres_internat_for_actor(etab, user, r).await, Err(EntDbError::Forbidden)), "{r} : paramètres");
        assert!(matches!(
            ent.modifier_parametres_internat_for_actor(etab, user, r, &ParametresInternatModifDto::default()).await,
            Err(EntDbError::Forbidden)
        ), "{r} : modifier paramètres");
        assert!(matches!(
            ent.lister_sejours_internat_for_actor(etab, user, r, e.id).await,
            Err(EntDbError::Forbidden)
        ), "{r} : séjours");
    }
    // Le CPE et la direction, eux, accèdent.
    assert!(ent.list_chambres_internat_for_actor(etab, CPE).await.is_ok());
    assert!(ent.plan_internat_for_actor(etab, cast.cpe.user_id, CPE, nuit).await.is_ok());
    // Les paramètres sont réservés à la direction : le CPE lit, ne modifie pas.
    assert!(ent.parametres_internat_for_actor(etab, cast.cpe.user_id, CPE).await.is_ok());
    assert!(matches!(
        ent.modifier_parametres_internat_for_actor(etab, cast.cpe.user_id, CPE, &ParametresInternatModifDto::default()).await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn ri_03_suivis_prives_reserves_et_lecture_journalisee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let eleve_id = cast.eleve_lea_id;
    // Le CPE note un suivi ; même demandé « visible famille », il reste privé (RI-09).
    let s = ent
        .creer_suivi_internat_for_actor(
            CPE,
            CreerSuiviInternat {
                etablissement_id: etab,
                eleve_id,
                jour: nuit,
                texte: "Calme, a aidé un camarade".into(),
                visible_famille: true,
                actor_user_id: cpe,
            },
        )
        .await
        .unwrap();
    assert!(!s.visible_famille, "plus de case « visible famille » à la saisie");
    // Équipe habilitée : lit le suivi, la lecture est journalisée.
    let avant: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'internat' AND action = 'lecture_suivis_prives'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let recap = ent.get_recap_internat_for_actor(etab, cpe, CPE, eleve_id, nuit).await.unwrap();
    assert!(recap.suivis_famille.iter().any(|x| x.id == s.id), "le CPE lit les suivis privés");
    let apres: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'internat' AND action = 'lecture_suivis_prives'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(apres, avant + 1, "chaque lecture sensible laisse une trace (R-IT38)");
    // Professeur, infirmerie, psychologue, comptabilité, accueil, personnel : aucun accès.
    for (role, user) in [
        (Role::Prof, cast.prof.user_id),
        (Role::Infirmerie, cast.infirmieres[0].user_id),
        (Role::Psy, cast.psys[0].user_id),
        (Role::Compta, cast.compta.user_id),
        (Role::Accueil, Uuid::new_v4()),
        (Role::Personnel, Uuid::new_v4()),
    ] {
        let r = ent.get_recap_internat_for_actor(etab, user, role.as_str(), eleve_id, nuit).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{} doit être refusé : {r:?}", role.as_str());
    }
    // Le parent de l'élève ne voit pas le suivi privé.
    let r = ent
        .get_recap_internat_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), eleve_id, nuit)
        .await
        .unwrap();
    assert!(r.suivis_famille.is_empty(), "suivi privé invisible de la famille");
    // Le parent d'un autre élève est refusé.
    let r = ent
        .get_recap_internat_for_actor(etab, cast.parent_tom.user_id, Role::Parent.as_str(), eleve_id, nuit)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Le journal est en ajout seul.
    let r = sqlx::query("UPDATE ent_journal_modif SET libelle = 'x' WHERE etablissement_id = $1 AND domaine = 'internat'")
        .bind(etab)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn nj_16_suivi_borne_et_sante_signalee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let mk = |texte: String| CreerSuiviInternat {
        etablissement_id: etab,
        eleve_id: cast.eleve_lea_id,
        jour: nuit,
        texte,
        visible_famille: false,
        actor_user_id: cpe,
    };
    assert!(est(
        &ent.creer_suivi_internat_for_actor(CPE, mk("x".repeat(301))).await,
        DomainError::InternatSuiviInvalide
    ));
    assert!(ent.creer_suivi_internat_for_actor(CPE, mk("x".repeat(300))).await.is_ok());
    // La base garde la borne (insertion directe).
    let r = sqlx::query(
        "INSERT INTO ent_internat_suivi (id, etablissement_id, eleve_id, jour, texte, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(nuit)
    .bind("y".repeat(301))
    .bind(cpe)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "suivi de plus de 300 caractères refusé par la base");
}

#[tokio::test]
async fn nj_16_aucune_donnee_de_sante_dans_l_internat() {
    let Some((ent, _)) = labo().await else { return };
    // R-IT6 : aucune colonne de santé dans les tables d'internat (le Next.js stockait allergies, PAI,
    // traitements et notes médicales dans la fiche de l'interne).
    let colonnes: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name LIKE 'ent\\_internat\\_%'
            AND (column_name ~* '(allerg|medic|traitement|diagnost|pai|sante|maladie|ordonnance|handicap)')",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(colonnes.is_empty(), "colonnes de santé interdites dans l'internat : {colonnes:?}");
}

// ───────────────────────── Isolation entre établissements ─────────────────────────

#[tokio::test]
async fn isolation_entre_etablissements_matrice() {
    let Some((ent, cast_a)) = labo().await else { return };
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let (a, b) = (cast_a.etablissement_id, cast_b.etablissement_id);
    assert_ne!(a, b);
    let nuit = ent.nuit_courante_internat(a).await.unwrap();
    let ch_a = chambre(&ent, a, cast_a.cpe.user_id, "garcons", 2).await;
    let el_a = eleve(&ent, a, "M", "interne").await;
    let el_b = eleve(&ent, b, "M", "interne").await;
    let ch_b = chambre(&ent, b, cast_b.cpe.user_id, "garcons", 2).await;
    // Lectures : chacun ne voit que ses chambres.
    let la = ent.list_chambres_internat_for_actor(a, CPE).await.unwrap();
    assert!(la.iter().any(|c| c.id == ch_a.id) && !la.iter().any(|c| c.id == ch_b.id));
    let lb = ent.list_chambres_internat_for_actor(b, CPE).await.unwrap();
    assert!(lb.iter().any(|c| c.id == ch_b.id) && !lb.iter().any(|c| c.id == ch_a.id));
    // Écritures croisées : élève de B dans la chambre de A (depuis A), et inversement.
    assert!(affecter(&ent, a, cast_a.cpe.user_id, el_b.id, ch_a.id, nuit).await.is_err());
    assert!(affecter(&ent, a, cast_a.cpe.user_id, el_a.id, ch_b.id, nuit).await.is_err());
    // Plan, zones, paramètres : strictement par établissement.
    let plan_b = ent.plan_internat_for_actor(b, cast_b.cpe.user_id, CPE, nuit).await.unwrap();
    assert!(!plan_b.chambres_sans_zone.iter().any(|c| c.id == ch_a.id));
    // Clés composites (E2) : la base refuse une affectation dont l'élève et la chambre
    // appartiennent à deux établissements différents, même par insertion directe.
    let r = sqlx::query(
        "INSERT INTO ent_internat_affectation (id, etablissement_id, eleve_id, chambre_id, debut, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(a)
    .bind(el_b.id)
    .bind(ch_a.id)
    .bind(nuit)
    .bind(cast_a.cpe.user_id)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "FK composite (etablissement_id, eleve_id) violée");
    // Pointage d'un élève de B depuis A : refusé.
    let r = ent
        .pointer_presence_internat_for_actor(CPE, pointage(a, el_b.id, nuit, "present", cast_a.cpe.user_id))
        .await;
    assert!(r.is_err());
    // Récapitulatif d'un élève d'un autre établissement : refusé.
    let r = ent.get_recap_internat_for_actor(a, cast_a.cpe.user_id, CPE, el_b.id, nuit).await;
    assert!(r.is_err());
}

// ───────────────────────── Chambres, lits, zones ─────────────────────────

#[tokio::test]
async fn chambre_modifiable_jamais_sous_l_occupation_ri_07() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 3).await;
    for _ in 0..2 {
        let e = eleve(&ent, etab, "M", "interne").await;
        affecter(&ent, etab, cpe, e.id, ch.id, nuit).await.unwrap();
    }
    let lits = ent.lister_lits_internat_for_actor(etab, cpe, CPE, ch.id).await.unwrap();
    assert_eq!(lits.len(), 3, "un lit par place");
    // Réduire à 1 lit (< 2 occupants) : refusé.
    let reduire = |c: i16| ChambreModifDto { capacite: Some(c), ..Default::default() };
    let r = ent.modifier_chambre_internat_for_actor(etab, cpe, CPE, ch.id, &reduire(1)).await;
    assert!(est(&r, DomainError::InternatCapaciteSousOccupation), "{r:?}");
    // Réduire à 2 : accepté, le lit libre (n° 3) disparaît ; remonter à 4 : un lit de plus.
    ent.modifier_chambre_internat_for_actor(etab, cpe, CPE, ch.id, &reduire(2)).await.unwrap();
    assert_eq!(ent.lister_lits_internat_for_actor(etab, cpe, CPE, ch.id).await.unwrap().len(), 2);
    ent.modifier_chambre_internat_for_actor(etab, cpe, CPE, ch.id, &reduire(4)).await.unwrap();
    assert_eq!(ent.lister_lits_internat_for_actor(etab, cpe, CPE, ch.id).await.unwrap().len(), 4);
    // Fermer ou retirer une chambre occupée : refusé ; changer son public : refusé.
    let fermer = ChambreModifDto { etat: Some("fermee".into()), ..Default::default() };
    assert!(est(
        &ent.modifier_chambre_internat_for_actor(etab, cpe, CPE, ch.id, &fermer).await,
        DomainError::InternatChambreOccupee
    ));
    let public = ChambreModifDto { public: Some("filles".into()), ..Default::default() };
    assert!(est(
        &ent.modifier_chambre_internat_for_actor(etab, cpe, CPE, ch.id, &public).await,
        DomainError::InternatChambreOccupee
    ));
    // Chambre vide : fermable, puis plus affectable.
    let vide = chambre(&ent, etab, cpe, "garcons", 2).await;
    let f = ent.modifier_chambre_internat_for_actor(etab, cpe, CPE, vide.id, &fermer).await.unwrap();
    assert_eq!(f.etat, "fermee");
    let e = eleve(&ent, etab, "M", "interne").await;
    assert!(est(&affecter(&ent, etab, cpe, e.id, vide.id, nuit).await, DomainError::InternatChambreFermee));
}

#[tokio::test]
async fn zones_lits_plan_sans_nom() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    // Un bâtiment du socle 5 (jeu de labo) pour y déclarer la zone.
    let batiment: Uuid = match sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM ent_batiment WHERE etablissement_id = $1 LIMIT 1",
    )
    .bind(etab)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap()
    {
        Some(b) => b,
        None => {
            let site: Uuid = sqlx::query_scalar("SELECT id FROM ent_site WHERE etablissement_id = $1 LIMIT 1")
                .bind(etab)
                .fetch_optional(ent.pool_verif())
                .await
                .unwrap()
                .unwrap_or_else(Uuid::new_v4);
            sqlx::query("INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, 'Site test') ON CONFLICT DO NOTHING")
                .bind(site)
                .bind(etab)
                .bind(format!("S{}", alea()))
                .execute(ent.pool_verif())
                .await
                .unwrap();
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_batiment (id, etablissement_id, site_id, code, libelle) VALUES ($1, $2, $3, $4, 'Bâtiment test')")
                .bind(id)
                .bind(etab)
                .bind(site)
                .bind(format!("B{}", alea()))
                .execute(ent.pool_verif())
                .await
                .unwrap();
            id
        }
    };
    let z = ent
        .creer_zone_internat_for_actor(
            etab,
            cpe,
            CPE,
            &ZoneSaisieDto { batiment_id: batiment, code: format!("Z{}", alea()), libelle: "Aile des garçons".into(), public: "garcons".into(), ordre: Some(1) },
        )
        .await
        .unwrap();
    let commune = ent
        .creer_zone_internat_for_actor(
            etab,
            cpe,
            CPE,
            &ZoneSaisieDto { batiment_id: batiment, code: format!("C{}", alea()), libelle: "Foyer".into(), public: "commun".into(), ordre: Some(2) },
        )
        .await
        .unwrap();
    let saisie = |zone: Option<Uuid>, public: &str| ChambreSaisieDto {
        zone_id: zone,
        batiment: "B".into(),
        etage: 1,
        code: format!("Z{}", alea()),
        capacite: 2,
        public: public.into(),
        accessible_pmr: true,
        douche: true,
        lavabo: false,
        bureau: true,
        notes_entretien: None,
    };
    // Cohérence zone / public (R-IT11) ; aucune chambre en partie commune.
    assert!(matches!(
        ent.creer_chambre_etendue_for_actor(etab, cpe, CPE, &saisie(Some(z.id), "filles")).await,
        Err(EntDbError::Domain(DomainError::InternatPublicIncompatible))
    ));
    assert!(matches!(
        ent.creer_chambre_etendue_for_actor(etab, cpe, CPE, &saisie(Some(commune.id), "garcons")).await,
        Err(EntDbError::Domain(DomainError::InternatZoneInvalide))
    ));
    let ch = ent.creer_chambre_etendue_for_actor(etab, cpe, CPE, &saisie(Some(z.id), "garcons")).await.unwrap();
    assert!(ch.accessible_pmr && ch.douche && ch.bureau);
    // Lit neutralisé : n'est plus attribué ; le plan le montre, sans aucun nom.
    let lits = ent.lister_lits_internat_for_actor(etab, cpe, CPE, ch.id).await.unwrap();
    ent.neutraliser_lit_internat_for_actor(
        etab,
        cpe,
        CPE,
        lits[0].id,
        &NeutraliserLitDto { du: nuit - Duration::days(1), au: None, motif_code: "casse".into() },
    )
    .await
    .unwrap();
    let e1 = eleve(&ent, etab, "M", "interne").await;
    let a1 = ent
        .affecter_lit_internat_for_actor(
            etab,
            cpe,
            CPE,
            &AffectationSaisieDto { eleve_id: e1.id, chambre_id: ch.id, lit_id: None, debut: nuit, motif_code: Some("rentree".into()), besoin_accessibilite: true },
        )
        .await
        .unwrap();
    assert_eq!(a1.lit_numero, Some(2), "le lit n° 1 est neutralisé : l'élève reçoit le n° 2");
    // Lit neutralisé demandé explicitement : refusé.
    let e2 = eleve(&ent, etab, "M", "interne").await;
    let r = ent
        .affecter_lit_internat_for_actor(
            etab,
            cpe,
            CPE,
            &AffectationSaisieDto { eleve_id: e2.id, chambre_id: ch.id, lit_id: Some(lits[0].id), debut: nuit, motif_code: None, besoin_accessibilite: false },
        )
        .await;
    assert!(est(&r, DomainError::InternatLitIndisponible), "{r:?}");
    // Un lit occupé ne se neutralise pas.
    let r = ent
        .neutraliser_lit_internat_for_actor(
            etab,
            cpe,
            CPE,
            a1.lit_id.unwrap(),
            &NeutraliserLitDto { du: nuit, au: None, motif_code: "travaux".into() },
        )
        .await;
    assert!(est(&r, DomainError::InternatLitIndisponible), "{r:?}");
    // Chambre non accessible refusée à un élève qui en a besoin.
    let sans_pmr = chambre(&ent, etab, cpe, "garcons", 2).await;
    let e3 = eleve(&ent, etab, "M", "interne").await;
    let r = ent
        .affecter_lit_internat_for_actor(
            etab,
            cpe,
            CPE,
            &AffectationSaisieDto { eleve_id: e3.id, chambre_id: sans_pmr.id, lit_id: None, debut: nuit, motif_code: None, besoin_accessibilite: true },
        )
        .await;
    assert!(est(&r, DomainError::InternatAccessibiliteRequise), "{r:?}");
    // Plan : lits libres / occupés / neutralisés ; identifiants seulement.
    let plan = ent.plan_internat_for_actor(etab, cpe, CPE, nuit).await.unwrap();
    let pz = plan.zones.iter().find(|p| p.zone.id == z.id).expect("zone au plan");
    let pc = pz.chambres.iter().find(|c| c.id == ch.id).expect("chambre au plan");
    let etats: Vec<&str> = pc.lits.iter().map(|l| l.etat.as_str()).collect();
    assert_eq!(etats, vec!["neutralise", "occupe"]);
    assert_eq!(pc.lits[1].eleve_id, Some(e1.id));
    let json = serde_json::to_string(&plan).unwrap();
    assert!(!json.contains(&e1.nom) && !json.contains("Théo"), "le plan ne porte aucun nom (R-IT32)");
    // Fermer la zone signale les occupants à reloger ; une zone fermée n'accueille plus.
    let fermee = ent
        .modifier_zone_internat_for_actor(
            etab,
            cpe,
            CPE,
            z.id,
            &ZoneModifDto { ferme_du: Some(nuit), ..Default::default() },
        )
        .await
        .unwrap();
    assert!(fermee.occupants_a_reloger >= 1);
    let e4 = eleve(&ent, etab, "M", "interne").await;
    let r = ent
        .affecter_lit_internat_for_actor(
            etab,
            cpe,
            CPE,
            &AffectationSaisieDto { eleve_id: e4.id, chambre_id: ch.id, lit_id: None, debut: nuit, motif_code: None, besoin_accessibilite: false },
        )
        .await;
    assert!(est(&r, DomainError::InternatChambreFermee), "{r:?}");
}

// ───────────────────────── Séjours, rythmes, régime ─────────────────────────

#[tokio::test]
async fn sejour_ne_a_l_affectation_rythme_versionne_et_regime_ferme_l_hebergement() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let ch = chambre(&ent, etab, cpe, "garcons", 2).await;
    let e = eleve(&ent, etab, "M", "interne").await;
    // R-IT1 : aucun séjour tant qu'on n'a rien fait ; la lecture n'écrit jamais.
    assert!(ent.lister_sejours_internat_for_actor(etab, cpe, CPE, e.id).await.unwrap().is_empty());
    let a = affecter(&ent, etab, cpe, e.id, ch.id, nuit).await.unwrap();
    let sejours = ent.lister_sejours_internat_for_actor(etab, cpe, CPE, e.id).await.unwrap();
    assert_eq!(sejours.len(), 1);
    assert_eq!(sejours[0].origine, "regime");
    assert_eq!(Some(sejours[0].id), a.sejour_id);
    assert_eq!(sejours[0].rythmes.len(), 1, "rythme initial = rythme par défaut (S-4100)");
    assert!(sejours[0].rythmes[0].jours.dimanche && !sejours[0].rythmes[0].jours.vendredi);
    // Resynchroniser est idempotent (aucun doublon de séjour).
    ent.synchroniser_sejours_internat_for_actor(etab, cpe, CPE, e.id).await.unwrap();
    assert_eq!(ent.lister_sejours_internat_for_actor(etab, cpe, CPE, e.id).await.unwrap().len(), 1);
    // R-IT4 : le rythme est versionné, on ajoute une version, on ne modifie pas l'ancienne.
    let sid = sejours[0].id;
    let jours = |mercredi: bool| JoursRythmeDto { dimanche: true, lundi: true, mardi: true, mercredi, jeudi: true, vendredi: false, samedi: false };
    let s2 = ent
        .definir_rythme_internat_for_actor(etab, cpe, CPE, sid, &RythmeSaisieDto { du: nuit + Duration::days(7), jours: jours(false) })
        .await
        .unwrap();
    assert_eq!(s2.rythmes.len(), 2);
    assert!(s2.rythmes[0].jours.mercredi && !s2.rythmes[1].jours.mercredi);
    // Même date : refusé (une version par date) ; aucun soir attendu : refusé.
    assert!(ent
        .definir_rythme_internat_for_actor(etab, cpe, CPE, sid, &RythmeSaisieDto { du: nuit + Duration::days(7), jours: jours(true) })
        .await
        .is_err());
    let aucun = JoursRythmeDto { dimanche: false, lundi: false, mardi: false, mercredi: false, jeudi: false, vendredi: false, samedi: false };
    assert!(est(
        &ent.definir_rythme_internat_for_actor(etab, cpe, CPE, sid, &RythmeSaisieDto { du: nuit + Duration::days(9), jours: aucun }).await,
        DomainError::InternatRythmeInvalide
    ));
    let r = sqlx::query("UPDATE ent_internat_rythme SET lundi = false WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "les versions de rythme sont en ajout seul");

    // RI-05 / INT-112 : le passage à demi-pensionnaire (événement du socle 2) ferme l'affectation
    // la veille de la date d'effet et le séjour ; avis publié ; rejouable sans doublon.
    let effet = nuit + Duration::days(1);
    ent.changer_regime_ou_site(
        "administratif",
        crate::scolarite::ChangementRegimeSite {
            etablissement_id: etab,
            eleve_id: e.id,
            kind: domain_eleve::AttributKind::Regime,
            valeur: "demi_pension".into(),
            date_effet: effet,
            derogation: None,
            acteur: cast.administratif.user_id,
        },
    )
    .await
    .unwrap();
    let cfg = crate::outbox::ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    };
    ent.distributeur(cfg.clone()).distribuer_tout().await.unwrap();
    ent.distributeur(cfg).distribuer_tout().await.unwrap();
    let (fin,): (Option<NaiveDate>,) = sqlx::query_as(
        "SELECT fin FROM ent_internat_affectation WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(a.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fin, Some(effet - Duration::days(1)), "affectation close la veille de la date d'effet");
    let sj = ent.lister_sejours_internat_for_actor(etab, cpe, CPE, e.id).await.unwrap();
    assert_eq!(sj.len(), 1);
    assert_eq!(sj[0].au, Some(effet - Duration::days(1)), "séjour clos, jamais supprimé (R-IT3)");
    let avis: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'internat.regime_effet' AND aggregate_id = $2",
    )
    .bind(etab)
    .bind(a.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(avis, 1, "un seul avis, même si l'événement est rejoué");
    // Suppression d'une affectation : impossible.
    let r = sqlx::query("DELETE FROM ent_internat_affectation WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(a.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "R-IT3 : on clôt, on ne supprime pas");
}

#[tokio::test]
async fn changement_de_chambre_atomique_et_parametres_de_la_direction() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, cpe, dir) = (cast.etablissement_id, cast.cpe.user_id, cast.direction.user_id);
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let c1 = chambre(&ent, etab, cpe, "garcons", 2).await;
    let c2 = chambre(&ent, etab, cpe, "garcons", 1).await;
    let c_pleine = chambre(&ent, etab, cpe, "garcons", 1).await;
    let e = eleve(&ent, etab, "M", "interne").await;
    let autre = eleve(&ent, etab, "M", "interne").await;
    affecter(&ent, etab, cpe, autre.id, c_pleine.id, nuit - Duration::days(10)).await.unwrap();
    affecter(&ent, etab, cpe, e.id, c1.id, nuit - Duration::days(10)).await.unwrap();
    // Changement vers une chambre pleine : refusé, et l'ancienne affectation reste ouverte (atomique).
    let chg = |chambre: Uuid| ChangementChambreDto { chambre_id: chambre, lit_id: None, date_effet: nuit, motif_code: "demande_eleve".into(), besoin_accessibilite: false };
    let r = ent.changer_chambre_internat_for_actor(etab, cpe, CPE, e.id, &chg(c_pleine.id)).await;
    assert!(est(&r, DomainError::InternatChambrePleine), "{r:?}");
    let ouvertes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ent_internat_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND fin IS NULL AND chambre_id = $3",
    )
    .bind(etab)
    .bind(e.id)
    .bind(c1.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(ouvertes, 1, "rien n'a été fermé : l'interne n'est jamais sans lit");
    // Vers une chambre libre : l'ancienne ferme la veille, la nouvelle commence à la date d'effet.
    let n = ent.changer_chambre_internat_for_actor(etab, cpe, CPE, e.id, &chg(c2.id)).await.unwrap();
    assert_eq!(n.debut, nuit);
    assert_eq!(n.motif_code.as_deref(), Some("demande_eleve"));
    let (fin,): (Option<NaiveDate>,) = sqlx::query_as(
        "SELECT fin FROM ent_internat_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND chambre_id = $3",
    )
    .bind(etab)
    .bind(e.id)
    .bind(c1.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fin, Some(nuit - Duration::days(1)));
    // Motif codé obligatoire.
    let mauvais = ChangementChambreDto { motif_code: "parce_que".into(), ..chg(c1.id) };
    assert!(ent.changer_chambre_internat_for_actor(etab, cpe, CPE, e.id, &mauvais).await.is_err());

    // Paramètres : défauts de §28, modifiables par la direction, bornés.
    let p = ent.parametres_internat_for_actor(etab, cpe, CPE).await.unwrap();
    assert_eq!(p.seuil_retard_minutes, 30);
    assert_eq!(p.majeur_info_parents, "si_a_charge");
    let m = ParametresInternatModifDto { seuil_retard_minutes: Some(45), jour_bilan: Some(5), ..Default::default() };
    let p2 = ent.modifier_parametres_internat_for_actor(etab, dir, DIR, &m).await.unwrap();
    assert_eq!((p2.seuil_retard_minutes, p2.jour_bilan), (45, 5));
    assert_eq!(ent.parametres_internat_for_actor(etab, cpe, CPE).await.unwrap().seuil_retard_minutes, 45);
    for mauvais in [
        ParametresInternatModifDto { seuil_retard_minutes: Some(9999), ..Default::default() },
        ParametresInternatModifDto { jour_bilan: Some(9), ..Default::default() },
        ParametresInternatModifDto { majeur_info_parents: Some("peut_etre".into()), ..Default::default() },
    ] {
        assert!(ent.modifier_parametres_internat_for_actor(etab, dir, DIR, &mauvais).await.is_err());
    }
}
