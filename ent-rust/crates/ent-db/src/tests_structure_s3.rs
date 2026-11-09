//! Socle 5, lot S3 : calendrier de l'année, fériés calculés, ponts, jours cibles, semaines A/B,
//! fermeture exceptionnelle (effets différenciés, levée programmée), port `jour()`, droits et
//! cloisonnement. Base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ignorés en
//! local et **en échec en CI**. Chaque scénario travaille dans un **établissement neuf**.
//! Les dates du calendrier officiel saisies ici sont des **données de labo**, pas des dates
//! officielles.

use super::*;
use crate::outbox::{ConfigDistribution, Consommateur, EvenementDistribue, FutureTraitement};
use crate::ref_commun::Acteur;
use crate::seed::LaboPersona;
use crate::structure_calendrier::{
    calendrier, Cible, CibleEvenement, EvenementModif, EvenementSaisie, FermetureModif,
    FermetureSaisie, LigneOfficielle, RegleSemaineSaisie,
};
use crate::structure_etab::{structure as port_etab, ProfilSaisie};
use crate::tenant_tx::{RlsMode, TenantTx};
use crate::tests_d022_bis::labo;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use domain_eleve::DomainError;
use serde_json::{json, Value};
use sqlx::{PgConnection, Row};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

fn acteur_de(etab: Uuid, p: &LaboPersona) -> Acteur<'static> {
    Acteur {
        etablissement_id: etab,
        user_id: p.user_id,
        role: p.role.as_str(),
    }
}

fn sfx() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_lowercase()
}

fn est_forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn est_domaine<T>(r: &Result<T, EntDbError>, e: DomainError) -> bool {
    matches!(r, Err(EntDbError::Domain(x)) if *x == e)
}

struct Monde3 {
    etab: Uuid,
    annee: Uuid,
}

/// Établissement neuf avec l'année de labo 2026-2027 (non courante : dates fixes, assertions exactes).
async fn etab_fixe(ent: &PgEnt) -> Monde3 {
    etab_avec_annee(ent, "2026-2027", d(2026, 9, 1), d(2027, 8, 31), false).await
}

/// Établissement neuf avec une année qui contient aujourd'hui (fermetures : « à partir d'aujourd'hui »).
async fn etab_courant(ent: &PgEnt) -> Monde3 {
    let t = Utc::now().date_naive();
    etab_avec_annee(
        ent,
        "courante",
        t - Duration::days(60),
        t + Duration::days(200),
        true,
    )
    .await
}

async fn etab_avec_annee(
    ent: &PgEnt,
    libelle: &str,
    debut: NaiveDate,
    fin: NaiveDate,
    courante: bool,
) -> Monde3 {
    let (etab, annee) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(annee)
    .bind(etab)
    .bind(libelle)
    .bind(debut)
    .bind(fin)
    .bind(courante)
    .execute(ent.pool())
    .await
    .unwrap();
    Monde3 { etab, annee }
}

fn profil(zone: Option<&str>, dep: &str) -> ProfilSaisie {
    ProfilSaisie {
        nom_officiel: "Collège de labo".into(),
        nom_usage: None,
        logo_document_id: None,
        uai: None,
        secteur: "public".into(),
        contrat_type: None,
        degres: vec!["second".into()],
        types: vec!["college".into()],
        academie_code: None,
        departement_code: Some(dep.into()),
        zone_calendrier: zone.map(str::to_string),
        regime_local: None,
        tutelle_libelle: None,
        adresse: None,
        telephone: None,
        courriel_contact: None,
        jours_ouvres: None,
        version_attendue: None,
    }
}

async fn niveau(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ($1,$2,$3,$3)",
    )
    .bind(id)
    .bind(etab)
    .bind(code)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn classe(ent: &PgEnt, m: &Monde3, code: &str, niveau_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau_id)
         VALUES ($1,$2,$3,$4,$4,$5)",
    )
    .bind(id)
    .bind(m.etab)
    .bind(m.annee)
    .bind(code)
    .bind(niveau_id)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn groupe(ent: &PgEnt, m: &Monde3, code: &str, classe_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle, classe_id)
         VALUES ($1,$2,$3,$4,$4,$5)",
    )
    .bind(id)
    .bind(m.etab)
    .bind(m.annee)
    .bind(code)
    .bind(classe_id)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

fn saisie(
    annee: Uuid,
    type_e: &str,
    libelle: &str,
    du: NaiveDate,
    au: NaiveDate,
) -> EvenementSaisie {
    EvenementSaisie {
        annee_id: annee,
        type_evenement: type_e.into(),
        libelle: libelle.into(),
        du,
        au,
        du_demi: None,
        au_demi: None,
        effet: None,
        cibles: vec![],
        publie_familles: false,
        publie_personnels: false,
        motif: None,
        jour_reference: None,
    }
}

fn officielle(
    zone: &str,
    type_l: &str,
    libelle: &str,
    du: NaiveDate,
    au: NaiveDate,
    v: &str,
) -> LigneOfficielle {
    LigneOfficielle {
        zone: zone.into(),
        annee_libelle: "2026-2027".into(),
        type_ligne: type_l.into(),
        libelle: libelle.into(),
        du,
        au,
        source: "données de labo (pas une source officielle)".into(),
        source_version: v.into(),
    }
}

/// Vacances de labo de l'année 2026-2027 (dates inventées pour le test).
fn vacances_labo(zone: &str, v: &str) -> Vec<LigneOfficielle> {
    vec![
        officielle(
            zone,
            "vacances",
            "Vacances de la Toussaint",
            d(2026, 10, 19),
            d(2026, 10, 30),
            v,
        ),
        officielle(
            zone,
            "vacances",
            "Vacances de Noël",
            d(2026, 12, 21),
            d(2027, 1, 1),
            v,
        ),
        officielle(
            zone,
            "vacances",
            "Vacances d'hiver",
            d(2027, 2, 8),
            d(2027, 2, 19),
            v,
        ),
        officielle(
            zone,
            "vacances",
            "Vacances de printemps",
            d(2027, 4, 5),
            d(2027, 4, 16),
            v,
        ),
        officielle(
            zone,
            "vacances",
            "Vacances d'été",
            d(2027, 7, 3),
            d(2027, 8, 31),
            v,
        ),
    ]
}

async fn jour(
    ent: &PgEnt,
    m: &Monde3,
    date: NaiveDate,
    cible: &Cible,
) -> crate::structure_calendrier::Jour {
    let mut c = ent.pool().acquire().await.unwrap();
    calendrier::jour(&mut c, m.etab, date, cible).await.unwrap()
}

async fn jour_24(ent: &PgEnt, m: &Monde3, c: Cible) -> crate::structure_calendrier::Jour {
    jour(ent, m, d(2026, 9, 24), &c).await
}

async fn nb_evenements_publies(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2")
        .bind(etab)
        .bind(type_name)
        .fetch_one(ent.pool())
        .await
        .unwrap()
        .get(0)
}

// ─── Critère de fin n° 1 : jour() exact sur une année de labo ───────────────────────────────

#[tokio::test]
async fn jour_exact_sur_une_annee_de_labo() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_fixe(&ent).await;
    let dir = acteur_de(m.etab, &cast.direction);
    let zone = format!("z{}", sfx());
    ent.structure_profil_definir(&dir, profil(Some(&zone), "75"))
        .await
        .unwrap();
    ent.calendrier_officiel_referencer(&vacances_labo(&zone, "labo-1"))
        .await
        .unwrap();

    // Aperçu : rien n'est écrit.
    let apercu = ent
        .structure_calendrier_adopter(&dir, m.annee, true, false)
        .await
        .unwrap();
    assert!(!apercu.ecrit && apercu.officiel_disponible);
    assert_eq!(apercu.ajoutes.len(), 5);
    assert_eq!(apercu.feries_ajoutes.len(), 11);
    assert!(!apercu.ponts_possibles.is_empty());
    assert!(ent
        .structure_calendrier_evenements(&dir, m.annee, None, true)
        .await
        .unwrap()
        .is_empty());
    // Adoption réelle puis rejeu : idempotent.
    let r = ent
        .structure_calendrier_adopter(&dir, m.annee, false, false)
        .await
        .unwrap();
    assert!(r.ecrit);
    let r2 = ent
        .structure_calendrier_adopter(&dir, m.annee, false, false)
        .await
        .unwrap();
    assert!(r2.ajoutes.is_empty() && r2.feries_ajoutes.is_empty() && r2.inchanges == 5);
    assert_eq!(
        ent.structure_calendrier_evenements(&dir, m.annee, None, false)
            .await
            .unwrap()
            .len(),
        16
    );

    // Pont, journée banalisée par niveau, rattrapage d'un samedi, types de semaine.
    let pont = ent
        .structure_calendrier_evenement_creer(
            &dir,
            saisie(
                m.annee,
                "pont",
                "Pont de l'Ascension",
                d(2027, 5, 7),
                d(2027, 5, 7),
            ),
        )
        .await
        .unwrap();
    assert!(pont.chevauchements.is_empty());
    let (n6, n5) = (
        niveau(&ent, m.etab, "6EME").await,
        niveau(&ent, m.etab, "5EME").await,
    );
    let (c6, c5) = (
        classe(&ent, &m, "6A", n6).await,
        classe(&ent, &m, "5A", n5).await,
    );
    let g6 = groupe(&ent, &m, "6A-G1", c6).await;
    let mut b = saisie(
        m.annee,
        "journee_banalisee",
        "Sortie des sixièmes",
        d(2026, 9, 24),
        d(2026, 9, 24),
    );
    b.cibles = vec![CibleEvenement {
        niveau_id: Some(n6),
        ..Default::default()
    }];
    ent.structure_calendrier_evenement_creer(&dir, b)
        .await
        .unwrap();
    let mut rat = saisie(
        m.annee,
        "rattrapage",
        "Samedi rattrapé",
        d(2026, 10, 10),
        d(2026, 10, 10),
    );
    rat.jour_reference = Some("lun".into());
    ent.structure_calendrier_evenement_creer(&dir, rat)
        .await
        .unwrap();
    ent.structure_semaines_definir(
        &dir,
        m.annee,
        RegleSemaineSaisie {
            etiquettes: vec!["a".into(), "B".into()],
            mode: "alternance_continue".into(),
            premiere_semaine: d(2026, 9, 7),
        },
    )
    .await
    .unwrap();

    let vide = Cible::default();
    // Mercredi 2 septembre : matin seulement, avant la première semaine étiquetée.
    let j = jour(&ent, &m, d(2026, 9, 2), &vide).await;
    assert!(j.ouvre && j.matin.cours && !j.apres_midi.cours && j.semaine_type.is_none());
    assert_eq!(j.type_jour, "cours");
    // Semaines A/B : 7/9 A, 14/9 B, 2/11 A (les vacances ne comptent pas), 14/12 A, 4/1 B.
    for (date, e) in [
        (d(2026, 9, 7), "A"),
        (d(2026, 9, 14), "B"),
        (d(2026, 11, 2), "A"),
        (d(2026, 12, 14), "A"),
        (d(2027, 1, 4), "B"),
    ] {
        assert_eq!(
            jour(&ent, &m, date, &vide).await.semaine_type.as_deref(),
            Some(e),
            "{date}"
        );
    }
    // Semaines de vacances : sans étiquette.
    for date in [
        d(2026, 10, 21),
        d(2026, 10, 28),
        d(2026, 12, 23),
        d(2026, 12, 30),
    ] {
        let j = jour(&ent, &m, date, &vide).await;
        assert!(!j.ouvre && j.semaine_type.is_none(), "{date}");
    }
    // Vacances, fériés (Toussaint dimanche, Armistice mercredi, Noël, Pâques, Ascension), pont.
    for (date, t) in [
        (d(2026, 10, 21), "vacances"),
        (d(2026, 11, 11), "ferie"),
        (d(2026, 12, 25), "ferie"), // vacances + férié : le férié prime dans le type
        (d(2027, 3, 29), "ferie"),
        (d(2027, 5, 6), "ferie"),
        (d(2027, 5, 7), "pont"),
        (d(2027, 5, 17), "ferie"),
        (d(2027, 9, 1), "hors_annee"),
    ] {
        let j = jour(&ent, &m, date, &vide).await;
        assert!(!j.ouvre, "{date}");
        assert_eq!(j.type_jour, t, "{date}");
    }
    // Cibles : la journée banalisée ne touche que les 6e (classe et groupe de 6e compris).
    assert!(
        !jour_24(
            &ent,
            &m,
            Cible {
                classe_id: Some(c6),
                ..Default::default()
            }
        )
        .await
        .ouvre
    );
    assert!(
        !jour_24(
            &ent,
            &m,
            Cible {
                groupe_id: Some(g6),
                ..Default::default()
            }
        )
        .await
        .ouvre
    );
    assert!(
        !jour_24(
            &ent,
            &m,
            Cible {
                niveau_id: Some(n6),
                ..Default::default()
            }
        )
        .await
        .ouvre
    );
    assert!(
        jour_24(
            &ent,
            &m,
            Cible {
                classe_id: Some(c5),
                ..Default::default()
            }
        )
        .await
        .ouvre
    );
    assert!(jour_24(&ent, &m, vide.clone()).await.ouvre);
    // Rattrapage : le samedi 10 octobre suit l'emploi du temps du lundi (matin et après-midi), semaine A.
    let j = jour(&ent, &m, d(2026, 10, 10), &vide).await;
    assert!(j.ouvre && j.matin.cours && j.apres_midi.cours);
    assert_eq!(
        (j.type_jour.as_str(), j.jour_reference.as_deref()),
        ("rattrapage", Some("lun"))
    );
    assert_eq!(j.semaine_type.as_deref(), Some("A"));
    // Jours de cours de septembre : 22.
    let mut c = ent.pool().acquire().await.unwrap();
    let n = calendrier::jours_ouvres(&mut c, m.etab, d(2026, 9, 1), d(2026, 9, 30), &vide)
        .await
        .unwrap();
    assert_eq!(n.len(), 22);
    let n6j = calendrier::jours_ouvres(
        &mut c,
        m.etab,
        d(2026, 9, 1),
        d(2026, 9, 30),
        &Cible {
            niveau_id: Some(n6),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(n6j.len(), 21);
    assert!(
        calendrier::jours(&mut c, m.etab, d(2026, 1, 1), d(2028, 6, 1), &vide)
            .await
            .is_err()
    );
    drop(c);

    // Pentecôte travaillée : on annule le férié calculé, le lundi redevient un jour de cours.
    let pent = ent
        .structure_calendrier_evenements(&dir, m.annee, None, false)
        .await
        .unwrap()
        .into_iter()
        .find(|e| e.libelle == "Lundi de Pentecôte")
        .unwrap();
    assert_eq!(
        (pent.du, pent.origine.as_str()),
        (d(2027, 5, 17), "calcule")
    );
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_modifier(
            &dir,
            pent.id,
            EvenementModif {
                libelle: Some("x".into()),
                ..Default::default()
            }
        )
        .await,
        DomainError::Conflit
    ));
    ent.structure_calendrier_evenement_annuler(&dir, pent.id, "journée de solidarité")
        .await
        .unwrap();
    let j = jour(&ent, &m, d(2027, 5, 17), &vide).await;
    assert!(j.ouvre && j.type_jour == "cours");
    // Rejouer l'adoption ne ressuscite pas le férié annulé.
    let r3 = ent
        .structure_calendrier_adopter(&dir, m.annee, false, false)
        .await
        .unwrap();
    assert!(r3.feries_ajoutes.is_empty());
    assert!(!jour(&ent, &m, d(2027, 5, 17), &vide)
        .await
        .type_jour
        .eq("ferie"));
    ent.structure_calendrier_evenement_retablir(&dir, pent.id)
        .await
        .unwrap();
    assert_eq!(
        jour(&ent, &m, d(2027, 5, 17), &vide).await.type_jour,
        "ferie"
    );

    // Semaine forcée puis libérée.
    ent.structure_semaine_forcer(
        &dir,
        m.annee,
        d(2026, 9, 14),
        Some("a".into()),
        "inversion décidée en conseil",
    )
    .await
    .unwrap();
    assert_eq!(
        jour(&ent, &m, d(2026, 9, 14), &vide)
            .await
            .semaine_type
            .as_deref(),
        Some("A")
    );
    assert!(est_domaine(
        &ent.structure_semaine_forcer(&dir, m.annee, d(2026, 9, 14), Some("Z".into()), "x")
            .await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_semaine_forcer(&dir, m.annee, d(2026, 9, 15), Some("A".into()), "x")
            .await,
        DomainError::ValeurInvalide
    ));
    ent.structure_semaine_liberer(&dir, m.annee, d(2026, 9, 14))
        .await
        .unwrap();
    assert_eq!(
        jour(&ent, &m, d(2026, 9, 14), &vide)
            .await
            .semaine_type
            .as_deref(),
        Some("B")
    );

    // Comparaison avec une autre année (STR-30) et journal.
    let cmp = ent
        .structure_calendrier_comparer(&dir, m.annee, m.annee)
        .await
        .unwrap();
    assert_eq!(cmp.ecart_jours_cours, 0);
    assert!(cmp.annee.jours_cours > 120 && cmp.annee.semaines_par_etiquette.contains_key("A"));
    for objet in [
        "calendrier_adoption",
        "calendrier_evenement",
        "semaines",
        "semaine",
    ] {
        let l = ent.structure_journal(&dir, objet, None, 200).await.unwrap();
        assert!(!l.is_empty(), "journal : {objet}");
    }
    assert!(nb_evenements_publies(&ent, m.etab, "referentiel.calendrier_adopte").await >= 2);
    assert!(nb_evenements_publies(&ent, m.etab, "referentiel.semaines_definies").await >= 3);
}

// ─── Régimes locaux, outre-mer et fuseau ────────────────────────────────────────────────────

#[tokio::test]
async fn alsace_moselle_outre_mer_et_fuseau() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (am, re, metro) = (
        etab_fixe(&ent).await,
        etab_fixe(&ent).await,
        etab_fixe(&ent).await,
    );
    for (m, dep) in [(&am, "67"), (&re, "974"), (&metro, "75")] {
        let dir = acteur_de(m.etab, &cast.direction);
        ent.structure_profil_definir(&dir, profil(None, dep))
            .await
            .unwrap();
        let r = ent
            .structure_calendrier_adopter(&dir, m.annee, false, false)
            .await
            .unwrap();
        // Sans calendrier officiel : avertissement net, fériés calculés quand même.
        assert!(!r.officiel_disponible && !r.avertissements.is_empty());
    }
    let (dir_re, dir_am) = (
        acteur_de(re.etab, &cast.direction),
        acteur_de(am.etab, &cast.direction),
    );
    ent.structure_fuseau_definir(&dir_re, "Indian/Reunion", true)
        .await
        .unwrap();
    let vide = Cible::default();
    // Vendredi saint (26 mars 2027) et Saint-Étienne : Alsace-Moselle seulement.
    assert_eq!(
        jour(&ent, &am, d(2027, 3, 26), &vide).await.type_jour,
        "ferie"
    );
    assert_eq!(
        jour(&ent, &metro, d(2027, 3, 26), &vide).await.type_jour,
        "cours"
    );
    assert_eq!(
        jour(&ent, &am, d(2026, 12, 26), &vide).await.type_jour,
        "ferie"
    );
    // Abolition de l'esclavage à La Réunion (20 décembre) — férié local [NV : à valider].
    assert_eq!(
        jour(&ent, &re, d(2026, 12, 20), &vide).await.type_jour,
        "ferie"
    );
    assert!(jour(&ent, &metro, d(2026, 12, 20), &vide)
        .await
        .evenements
        .is_empty());
    // Les fériés nationaux valent partout.
    for m in [&am, &re, &metro] {
        assert_eq!(jour(&ent, m, d(2027, 5, 6), &vide).await.type_jour, "ferie");
    }
    let _ = dir_am;
    // Fuseau : 21 h UTC le 10 novembre = 01 h le 11 à La Réunion, 22 h le 10 à Paris.
    let instant = Utc.with_ymd_and_hms(2026, 11, 10, 21, 0, 0).unwrap();
    let mut c = ent.pool().acquire().await.unwrap();
    let h_re = port_etab::heure_locale(&mut c, re.etab, instant)
        .await
        .unwrap();
    let h_fr = port_etab::heure_locale(&mut c, metro.etab, instant)
        .await
        .unwrap();
    assert_eq!(
        (h_re.date(), h_fr.date()),
        (d(2026, 11, 11), d(2026, 11, 10))
    );
    // « Aujourd'hui » du port suit le fuseau de l'établissement.
    let aujourdhui = calendrier::aujourdhui(&mut c, re.etab, &vide)
        .await
        .unwrap()
        .date;
    assert_eq!(
        aujourdhui,
        port_etab::aujourdhui(&mut c, re.etab).await.unwrap()
    );
}

// ─── Critère de fin n° 2 : une fermeture émet un événement consommé ────────────────────────

#[derive(Default)]
struct Collecteur {
    vus: Mutex<Vec<(String, Value, usize)>>,
}

impl Consommateur for Collecteur {
    fn nom(&self) -> &'static str {
        "test.fermetures_s3"
    }
    fn types(&self) -> &'static [&'static str] {
        &[
            "referentiel.fermeture_exceptionnelle",
            "referentiel.fermeture_levee",
        ]
    }
    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(async move {
            // Un consommateur réel (socles 10, 12, 40, 41) lit le port dans sa transaction.
            let du: NaiveDate = serde_json::from_value(evt.event().payload["du"].clone())
                .map_err(|e| e.to_string())?;
            let j = calendrier::jour(tx, evt.etablissement_id(), du, &Cible::default())
                .await
                .map_err(|e| e.to_string())?;
            self.vus.lock().unwrap().push((
                evt.event().type_name.clone(),
                evt.event().payload.clone(),
                j.evenements.len(),
            ));
            Ok(())
        })
    }
}

fn config_test(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 50,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

fn fermeture(du: NaiveDate, au: NaiveDate) -> FermetureSaisie {
    FermetureSaisie {
        libelle: Some("Alerte météo".into()),
        du,
        au,
        du_demi: None,
        au_demi: None,
        services_touches: None,
        internes_presents: false,
        cibles: vec![],
        motif: "Alerte orange de la préfecture".into(),
        conditionnelle: false,
        levee_prevue_le: None,
        publie_familles: true,
        publie_personnels: false,
    }
}

async fn aujourdhui(ent: &PgEnt, etab: Uuid) -> NaiveDate {
    let mut c = ent.pool().acquire().await.unwrap();
    port_etab::aujourdhui(&mut c, etab).await.unwrap()
}

#[tokio::test]
async fn fermeture_exceptionnelle_effets_evenement_consomme_et_historique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_courant(&ent).await;
    let dir = acteur_de(m.etab, &cast.direction);
    let adm = acteur_de(m.etab, &cast.administratif);
    let t = aujourdhui(&ent, m.etab).await;

    // Seule la direction (ou l'administrateur) déclare : l'administratif écrit au calendrier, pas ici.
    let mut s = fermeture(t, t + Duration::days(2));
    s.internes_presents = true;
    assert!(est_forbidden(
        &ent.structure_fermeture_declarer(&adm, s.clone(), true)
            .await
    ));
    assert!(est_forbidden(
        &ent.structure_fermeture_declarer(&acteur_de(m.etab, &cast.prof), s.clone(), true)
            .await
    ));
    // Passé : rien à fermer. Motif obligatoire. Mélange service / portée : refusé.
    assert!(est_domaine(
        &ent.structure_fermeture_declarer(
            &dir,
            fermeture(t - Duration::days(3), t - Duration::days(1)),
            true
        )
        .await,
        DomainError::ValeurInvalide
    ));
    let mut sans_motif = s.clone();
    sans_motif.motif = "  ".into();
    assert!(est_domaine(
        &ent.structure_fermeture_declarer(&dir, sans_motif, true)
            .await,
        DomainError::ValeurInvalide
    ));

    // Aperçu : internes présents → internat et restauration restent ouverts (R-S1) ; rien d'écrit.
    let ap = ent
        .structure_fermeture_declarer(&dir, s.clone(), true)
        .await
        .unwrap();
    assert!(!ap.ecrit && ap.evenement.is_none());
    assert_eq!(ap.apercu.effet, "sans_cours");
    assert!(ap
        .apercu
        .services_maintenus
        .contains(&"internat".to_string()));
    assert!(ap
        .apercu
        .services_maintenus
        .contains(&"restauration".to_string()));
    assert!(!ap.apercu.avertissements.is_empty() && !ap.apercu.non_calculable.is_empty());
    let attendus: Vec<NaiveDate> = {
        let mut c = ent.pool().acquire().await.unwrap();
        calendrier::jours_ouvres(&mut c, m.etab, t, t + Duration::days(2), &Cible::default())
            .await
            .unwrap()
    };
    assert_eq!(ap.apercu.jours_cours_touches, attendus);
    assert!(ent
        .structure_calendrier_evenements(&dir, m.annee, None, true)
        .await
        .unwrap()
        .is_empty());

    // Déclaration réelle ; doublon identique refusé (un seul message).
    let r = ent
        .structure_fermeture_declarer(&dir, s.clone(), false)
        .await
        .unwrap();
    let f = r.evenement.unwrap();
    assert_eq!(
        (f.effet.as_str(), f.internes_presents),
        ("sans_cours", true)
    );
    assert!(est_domaine(
        &ent.structure_fermeture_declarer(&dir, s.clone(), false)
            .await,
        DomainError::Conflit
    ));
    let j = jour(&ent, &m, t, &Cible::default()).await;
    assert!(j
        .evenements
        .iter()
        .any(|e| e.type_evenement == "fermeture_exceptionnelle"));
    assert!(!j.ouvre && j.services_fermes.contains(&"cours".to_string()));
    assert!(!j.services_fermes.contains(&"internat".to_string()));

    // Prolongation puis changement de services : un événement par changement.
    let p = ent
        .structure_fermeture_modifier(
            &dir,
            f.id,
            FermetureModif {
                au: Some(t + Duration::days(4)),
                motif: "décision préfectorale".into(),
                version_attendue: Some(f.version),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let f2 = p.evenement.unwrap();
    assert_eq!(f2.au, t + Duration::days(4));
    assert!(est_domaine(
        &ent.structure_fermeture_modifier(
            &dir,
            f.id,
            FermetureModif {
                au: Some(t + Duration::days(4)),
                motif: "x".into(),
                ..Default::default()
            }
        )
        .await,
        DomainError::Conflit
    ));
    // Version périmée : conflit.
    assert!(est_domaine(
        &ent.structure_fermeture_modifier(
            &dir,
            f.id,
            FermetureModif {
                au: Some(t + Duration::days(5)),
                motif: "x".into(),
                version_attendue: Some(f.version),
                ..Default::default()
            }
        )
        .await,
        DomainError::Conflit
    ));
    ent.structure_fermeture_modifier(
        &dir,
        f.id,
        FermetureModif {
            services_touches: Some(vec!["cours".into(), "transports".into()]),
            motif: "cantine maintenue".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    // Levée à partir de t + 2 : les jours déjà fermés le restent.
    let levee = ent
        .structure_fermeture_lever(&dir, f.id, "alerte terminée", Some(t + Duration::days(2)))
        .await
        .unwrap();
    assert_eq!(levee.au, t + Duration::days(1));
    assert!(est_domaine(
        &ent.structure_fermeture_lever(&dir, f.id, "encore", Some(t + Duration::days(3)))
            .await,
        DomainError::Conflit
    ));
    let hist = ent
        .structure_fermeture_historique(&dir, f.id)
        .await
        .unwrap();
    assert_eq!(
        hist.iter()
            .map(|h| h.changement.as_str())
            .collect::<Vec<_>>(),
        ["declaree", "prolongee", "services_modifies", "levee"]
    );
    assert_eq!(
        nb_evenements_publies(&ent, m.etab, "referentiel.fermeture_exceptionnelle").await,
        3
    );
    assert_eq!(
        nb_evenements_publies(&ent, m.etab, "referentiel.fermeture_levee").await,
        1
    );

    // Consommateur de test : reçoit les 4 faits, lit le port, et ne voit ni motif ni libellé.
    let mut dist = ent.distributeur(config_test(m.etab));
    let col = Arc::new(Collecteur::default());
    dist.enregistrer(col.clone()).unwrap();
    dist.distribuer_tout().await.unwrap();
    let vus = col.vus.lock().unwrap().clone();
    assert_eq!(vus.len(), 4);
    let (_, charge, nb_evts_jour) = &vus[0];
    assert!(
        *nb_evts_jour >= 1,
        "le consommateur lit la fermeture par le port"
    );
    assert_eq!(
        charge["services_touches"],
        json!(["accueil_eleves", "administration", "cours", "transports"])
    );
    for (_, c, _) in &vus {
        assert!(c.get("motif").is_none() && c.get("libelle").is_none());
        assert!(c["evenement_id"] == json!(f.id));
        assert!(c.get("historique_id").is_some());
    }
    assert!(!charge["services_touches"]
        .as_array()
        .unwrap()
        .contains(&json!("internat")));
    assert_eq!(vus[3].0, "referentiel.fermeture_levee");

    // Garde-fous de la base : suppression et réouverture d'une fermeture impossibles.
    let r = sqlx::query("DELETE FROM ent_calendrier_evenement WHERE id = $1")
        .bind(f.id)
        .execute(ent.pool())
        .await;
    assert!(r.is_err());
    let r = sqlx::query("UPDATE ent_fermeture_historique SET motif = 'x' WHERE evenement_id = $1")
        .bind(f.id)
        .execute(ent.pool())
        .await;
    assert!(r.is_err(), "historique en ajout seul");
    // Levée entière d'une autre fermeture : annulée, jamais rouverte.
    let g = ent
        .structure_fermeture_declarer(
            &dir,
            fermeture(t + Duration::days(10), t + Duration::days(11)),
            false,
        )
        .await
        .unwrap()
        .evenement
        .unwrap();
    let g2 = ent
        .structure_fermeture_lever(&dir, g.id, "fausse alerte", Some(t + Duration::days(10)))
        .await
        .unwrap();
    assert!(g2.annule_le.is_some());
    let r = sqlx::query("UPDATE ent_calendrier_evenement SET annule_le = NULL, annule_par = NULL, annule_motif = NULL WHERE id = $1")
        .bind(g.id)
        .execute(ent.pool())
        .await;
    assert!(r.is_err(), "une fermeture levée ne se rouvre pas");
    // Les commandes du calendrier ordinaire refusent les fermetures.
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_annuler(&dir, f.id, "x")
            .await,
        DomainError::Conflit
    ));
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(
            &dir,
            saisie(m.annee, "fermeture_exceptionnelle", "x", t, t)
        )
        .await,
        DomainError::ValeurInvalide
    ));
}

// ─── Levée programmée : fuseau de l'établissement, un seul message ────────────────────────

#[tokio::test]
async fn levee_programmee_fuseau_et_un_seul_message() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (paris, re) = (etab_courant(&ent).await, etab_courant(&ent).await);
    ent.structure_fuseau_definir(&acteur_de(re.etab, &cast.direction), "Indian/Reunion", true)
        .await
        .unwrap();
    // Levée programmée le soir (UTC) du jour X : 22 h/21 h à Paris (après-midi : le matin reste
    // fermé), 00 h 30 le lendemain à La Réunion (la veille est la dernière journée fermée).
    let x = Utc::now().date_naive() + Duration::days(2);
    let instant: DateTime<Utc> = Utc.from_utc_datetime(&x.and_hms_opt(20, 30, 0).unwrap());
    let mut ids = Vec::new();
    for m in [&paris, &re] {
        let dir = acteur_de(m.etab, &cast.direction);
        let t = aujourdhui(&ent, m.etab).await;
        let mut s = fermeture(t, t + Duration::days(6));
        s.conditionnelle = true;
        s.levee_prevue_le = Some(instant);
        // Une levée sans fermeture conditionnelle est refusée.
        let mut seche = s.clone();
        seche.conditionnelle = false;
        assert!(est_domaine(
            &ent.structure_fermeture_declarer(&dir, seche, true).await,
            DomainError::ValeurInvalide
        ));
        let f = ent
            .structure_fermeture_declarer(&dir, s, false)
            .await
            .unwrap()
            .evenement
            .unwrap();
        assert_eq!(f.levee_prevue_le, Some(instant));
        ids.push(f.id);
    }
    // Avant l'heure : rien.
    assert_eq!(
        ent.structure_fermetures_levees_executer(paris.etab, instant - Duration::minutes(1))
            .await
            .unwrap(),
        0
    );
    for (m, id) in [&paris, &re].into_iter().zip(&ids) {
        let n = ent
            .structure_fermetures_levees_executer(m.etab, instant + Duration::minutes(1))
            .await
            .unwrap();
        assert_eq!(n, 1);
        // Rejouer ne produit rien de plus.
        assert_eq!(
            ent.structure_fermetures_levees_executer(m.etab, instant + Duration::hours(2))
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            nb_evenements_publies(&ent, m.etab, "referentiel.fermeture_levee").await,
            1
        );
        let dir = acteur_de(m.etab, &cast.direction);
        let e = ent.structure_calendrier_evenement(&dir, *id).await.unwrap();
        assert!(e.levee_prevue_le.is_none());
        let h = ent.structure_fermeture_historique(&dir, *id).await.unwrap();
        let derniere = h.last().unwrap();
        assert_eq!(
            (
                derniere.changement.as_str(),
                derniere.origine.as_str(),
                derniere.par
            ),
            ("levee", "programme", None)
        );
        if m.etab == paris.etab {
            assert_eq!(
                (e.au, e.au_demi.as_deref()),
                (x, Some("matin")),
                "Paris : levée l'après-midi"
            );
        } else {
            assert_eq!(
                (e.au, e.au_demi.as_deref()),
                (x, None),
                "La Réunion : levée le lendemain matin"
            );
        }
    }
    // Charge de l'événement de levée : programmée, identifiants et dates seulement.
    let c: Value = sqlx::query_scalar(
        "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'referentiel.fermeture_levee'",
    )
    .bind(re.etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (c["origine"].as_str(), c["complete"].as_bool()),
        (Some("programme"), Some(false))
    );
    assert!(c.get("motif").is_none());
}

// ─── Services administratifs (STR-93, R-S2) ─────────────────────────────────────────────

#[tokio::test]
async fn jours_ouvres_administratifs_et_fermeture_de_service() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_fixe(&ent).await;
    let dir = acteur_de(m.etab, &cast.direction);
    ent.structure_profil_definir(&dir, profil(None, "75"))
        .await
        .unwrap();
    ent.structure_calendrier_adopter(&dir, m.annee, false, false)
        .await
        .unwrap();
    let mut ete = saisie(
        m.annee,
        "fermeture",
        "Fermeture estivale du secrétariat",
        d(2027, 7, 20),
        d(2027, 8, 20),
    );
    ete.cibles = vec![CibleEvenement {
        service: Some("secretariat".into()),
        ..Default::default()
    }];
    ent.structure_calendrier_evenement_creer(&dir, ete)
        .await
        .unwrap();
    let mut vac = saisie(
        m.annee,
        "vacances",
        "Vacances de la Toussaint",
        d(2026, 10, 19),
        d(2026, 10, 30),
    );
    vac.effet = Some("sans_cours".into());
    ent.structure_calendrier_evenement_creer(&dir, vac)
        .await
        .unwrap();

    let s = ent
        .structure_calendrier_jour_service(&dir, d(2027, 8, 4), "secretariat")
        .await
        .unwrap();
    assert_eq!(
        (s.ouvert, s.raison.as_deref()),
        (false, Some("service_ferme"))
    );
    assert!(
        ent.structure_calendrier_jour_service(&dir, d(2027, 8, 4), "intendance")
            .await
            .unwrap()
            .ouvert
    );
    // Les vacances ne ferment pas le secrétariat ; un férié si.
    assert!(
        ent.structure_calendrier_jour_service(&dir, d(2026, 10, 21), "secretariat")
            .await
            .unwrap()
            .ouvert
    );
    assert_eq!(
        ent.structure_calendrier_jour_service(&dir, d(2026, 11, 11), "secretariat")
            .await
            .unwrap()
            .raison
            .as_deref(),
        Some("ferie")
    );
    let mut c = ent.pool().acquire().await.unwrap();
    let v = calendrier::jours_ouvres_administratifs(
        &mut c,
        m.etab,
        d(2026, 10, 19),
        d(2026, 10, 30),
        "secretariat",
    )
    .await
    .unwrap();
    assert_eq!(v.len(), 10);
    let v = calendrier::jours_ouvres_administratifs(
        &mut c,
        m.etab,
        d(2026, 11, 9),
        d(2026, 11, 13),
        "secretariat",
    )
    .await
    .unwrap();
    assert_eq!(v.len(), 4);
    // Délai de 3 jours ouvrés depuis le vendredi 6 novembre : lundi, mardi, (mercredi férié), jeudi.
    let e = calendrier::echeance_jours_ouvres(&mut c, m.etab, d(2026, 11, 6), 3, "secretariat")
        .await
        .unwrap();
    assert_eq!(e, d(2026, 11, 12));
    assert!(
        calendrier::jour_service(&mut c, m.etab, d(2026, 11, 6), "cantine")
            .await
            .is_err()
    );
    drop(c);

    // Un service ne se mélange pas avec une portée ; un service inconnu est refusé.
    let n = niveau(&ent, m.etab, "4EME").await;
    let mut mix = saisie(m.annee, "fermeture", "x", d(2027, 8, 1), d(2027, 8, 2));
    mix.cibles = vec![
        CibleEvenement {
            service: Some("cdi".into()),
            ..Default::default()
        },
        CibleEvenement {
            niveau_id: Some(n),
            ..Default::default()
        },
    ];
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(&dir, mix).await,
        DomainError::ValeurInvalide
    ));
    let mut inconnu = saisie(m.annee, "fermeture", "x", d(2027, 8, 1), d(2027, 8, 2));
    inconnu.cibles = vec![CibleEvenement {
        service: Some("cantine".into()),
        ..Default::default()
    }];
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(&dir, inconnu)
            .await,
        DomainError::ValeurInvalide
    ));
}

// ─── Versions du calendrier officiel : différences à accepter ───────────────────────────────

#[tokio::test]
async fn nouvelle_version_officielle_donne_des_differences_a_accepter() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_fixe(&ent).await;
    let dir = acteur_de(m.etab, &cast.direction);
    let zone = format!("z{}", sfx());
    ent.structure_profil_definir(&dir, profil(Some(&zone.to_uppercase()), "75"))
        .await
        .unwrap();
    let r = ent
        .structure_calendrier_adopter(&dir, m.annee, true, false)
        .await
        .unwrap();
    assert!(!r.officiel_disponible && r.ajoutes.is_empty());
    assert!(r
        .avertissements
        .iter()
        .any(|a| a.contains("saisie manuelle")));
    ent.calendrier_officiel_referencer(&vacances_labo(&zone, "v1"))
        .await
        .unwrap();
    ent.structure_calendrier_adopter(&dir, m.annee, false, false)
        .await
        .unwrap();
    // Nouvelle version : les vacances d'hiver sont décalées d'une semaine.
    let mut v2 = vacances_labo(&zone, "v2");
    v2[2] = officielle(
        &zone,
        "vacances",
        "Vacances d'hiver",
        d(2027, 2, 15),
        d(2027, 2, 26),
        "v2",
    );
    ent.calendrier_officiel_referencer(&v2).await.unwrap();
    let ap = ent
        .structure_calendrier_adopter(&dir, m.annee, true, false)
        .await
        .unwrap();
    assert_eq!(ap.differences.len(), 1);
    assert_eq!(
        (ap.differences[0].du_actuel, ap.differences[0].du_officiel),
        (d(2027, 2, 8), d(2027, 2, 15))
    );
    // Sans acceptation : les dates ne bougent pas.
    ent.structure_calendrier_adopter(&dir, m.annee, false, false)
        .await
        .unwrap();
    let vide = Cible::default();
    assert_eq!(
        jour(&ent, &m, d(2027, 2, 9), &vide).await.type_jour,
        "vacances"
    );
    let ac = ent
        .structure_calendrier_adopter(&dir, m.annee, false, true)
        .await
        .unwrap();
    assert!(ac.differences_acceptees);
    assert_eq!(
        jour(&ent, &m, d(2027, 2, 9), &vide).await.type_jour,
        "cours"
    );
    assert_eq!(
        jour(&ent, &m, d(2027, 2, 16), &vide).await.type_jour,
        "vacances"
    );
    assert!(ent
        .structure_calendrier_adopter(&dir, m.annee, true, false)
        .await
        .unwrap()
        .differences
        .is_empty());
    // Saisie manuelle : modèle documenté.
    let modele = crate::structure_calendrier::modele_saisie(Some("a"));
    assert_eq!(modele.periodes.len(), 5);
}

// ─── Validations et garde-fous ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn validations_chevauchements_et_annee_cloturee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_fixe(&ent).await;
    let autre = etab_fixe(&ent).await;
    let dir = acteur_de(m.etab, &cast.direction);
    let ok = saisie(
        m.annee,
        "journee_pedagogique",
        "Journée pédagogique",
        d(2026, 11, 20),
        d(2026, 11, 20),
    );
    let e = ent
        .structure_calendrier_evenement_creer(&dir, ok.clone())
        .await
        .unwrap();
    assert_eq!(e.evenement.effet, "sans_eleves", "effet par défaut du type");
    // Chevauchement expliqué, jamais bloquant.
    let c = ent
        .structure_calendrier_evenement_creer(
            &dir,
            saisie(
                m.annee,
                "journee_banalisee",
                "Autre",
                d(2026, 11, 20),
                d(2026, 11, 21),
            ),
        )
        .await
        .unwrap();
    assert_eq!(c.chevauchements.len(), 1);
    // Refus de saisies invalides.
    let invalides: Vec<EvenementSaisie> = vec![
        saisie(m.annee, "inconnu", "x", d(2026, 11, 1), d(2026, 11, 1)),
        saisie(m.annee, "pont", "x", d(2026, 11, 3), d(2026, 11, 2)),
        saisie(m.annee, "pont", "   ", d(2026, 11, 2), d(2026, 11, 2)),
        EvenementSaisie {
            effet: Some("magique".into()),
            ..saisie(m.annee, "pont", "x", d(2026, 11, 2), d(2026, 11, 2))
        },
        EvenementSaisie {
            du_demi: Some("soir".into()),
            ..saisie(m.annee, "pont", "x", d(2026, 11, 2), d(2026, 11, 2))
        },
        EvenementSaisie {
            publie_familles: true,
            publie_personnels: true,
            ..saisie(m.annee, "evenement", "x", d(2026, 11, 2), d(2026, 11, 2))
        },
        saisie(
            m.annee,
            "rattrapage",
            "sans référence",
            d(2026, 11, 2),
            d(2026, 11, 2),
        ),
        EvenementSaisie {
            jour_reference: Some("lun".into()),
            ..saisie(m.annee, "pont", "x", d(2026, 11, 2), d(2026, 11, 2))
        },
        saisie(m.annee, "pont", "hors année", d(2028, 1, 1), d(2028, 1, 1)),
    ];
    for s in invalides {
        let lib = s.libelle.clone();
        assert!(
            est_domaine(
                &ent.structure_calendrier_evenement_creer(&dir, s).await,
                DomainError::ValeurInvalide
            ),
            "{lib}"
        );
    }
    // R-S19 : publié aux personnels seulement.
    let mut p = saisie(
        m.annee,
        "evenement",
        "Conseil pédagogique",
        d(2026, 11, 24),
        d(2026, 11, 24),
    );
    p.publie_personnels = true;
    assert!(
        ent.structure_calendrier_evenement_creer(&dir, p)
            .await
            .unwrap()
            .evenement
            .publie_personnels
    );
    // Cible d'un autre établissement : introuvable (aucun oracle d'existence).
    let n_autre = niveau(&ent, autre.etab, "3EME").await;
    let mut x = saisie(m.annee, "pont", "x", d(2026, 11, 2), d(2026, 11, 2));
    x.cibles = vec![CibleEvenement {
        niveau_id: Some(n_autre),
        ..Default::default()
    }];
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(&dir, x).await,
        DomainError::NotFound
    ));
    // Année d'un autre établissement.
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(
            &dir,
            saisie(autre.annee, "pont", "x", d(2026, 11, 2), d(2026, 11, 2))
        )
        .await,
        DomainError::NotFound
    ));
    // Concurrence optimiste.
    let r = ent
        .structure_calendrier_evenement_modifier(
            &dir,
            e.evenement.id,
            EvenementModif {
                libelle: Some("Renommé".into()),
                version_attendue: Some(1),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.evenement.version, 2);
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_modifier(
            &dir,
            e.evenement.id,
            EvenementModif {
                libelle: Some("Encore".into()),
                version_attendue: Some(1),
                ..Default::default()
            }
        )
        .await,
        DomainError::Conflit
    ));
    // Année clôturée : le calendrier est gelé.
    sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE id = $1")
        .bind(m.annee)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(&dir, ok).await,
        DomainError::Conflit
    ));
    assert!(est_domaine(
        &ent.structure_calendrier_adopter(&dir, m.annee, false, false)
            .await,
        DomainError::Conflit
    ));
    // La lecture reste possible.
    assert!(
        ent.structure_calendrier_evenements(&dir, m.annee, None, true)
            .await
            .unwrap()
            .len()
            >= 3
    );
    // Les règles de semaines refusent les cas absurdes.
    let m2 = etab_fixe(&ent).await;
    let dir2 = acteur_de(m2.etab, &cast.direction);
    for (et, mode, lundi) in [
        (vec!["A"], "alternance_continue", d(2026, 9, 7)),
        (vec!["A", "A"], "alternance_continue", d(2026, 9, 7)),
        (vec!["A", "B"], "toutes_les_semaines", d(2026, 9, 7)),
        (vec!["A", "B"], "alternance_continue", d(2026, 9, 8)),
        (vec!["A", "B"], "alternance_continue", d(2030, 9, 2)),
    ] {
        let r = ent
            .structure_semaines_definir(
                &dir2,
                m2.annee,
                RegleSemaineSaisie {
                    etiquettes: et.iter().map(|s| s.to_string()).collect(),
                    mode: mode.into(),
                    premiere_semaine: lundi,
                },
            )
            .await;
        assert!(
            est_domaine(&r, DomainError::ValeurInvalide),
            "{et:?} {mode} {lundi}"
        );
    }
    // Forcer sans règle : conflit.
    assert!(est_domaine(
        &ent.structure_semaine_forcer(&dir2, m2.annee, d(2026, 9, 14), Some("A".into()), "x")
            .await,
        DomainError::Conflit
    ));
}

// ─── Droits et cloisonnement ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn matrice_de_droits_et_isolation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (m, autre) = (etab_courant(&ent).await, etab_courant(&ent).await);
    let t = aujourdhui(&ent, m.etab).await;
    // (nom, persona, lit, écrit au calendrier, déclare une fermeture)
    let personas: [(&str, &LaboPersona, bool, bool, bool); 7] = [
        ("admin", &cast.admin, true, true, true),
        ("direction", &cast.direction, true, true, true),
        ("administratif", &cast.administratif, true, true, false),
        ("compta", &cast.compta, true, false, false),
        ("prof", &cast.prof, true, false, false),
        ("cpe", &cast.cpe, true, false, false),
        ("parent", &cast.parent_lea, false, false, false),
    ];
    for (nom, p, lit, ecrit, ferme) in personas {
        let x = acteur_de(m.etab, p);
        assert_eq!(
            !est_forbidden(
                &ent.structure_calendrier_evenements(&x, m.annee, None, false)
                    .await
            ),
            lit,
            "lire : {nom}"
        );
        assert_eq!(
            !est_forbidden(
                &ent.structure_calendrier_jour(&x, t, &Cible::default())
                    .await
            ),
            lit,
            "jour : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_semaines(&x, m.annee).await),
            lit,
            "semaines : {nom}"
        );
        assert_eq!(
            !est_forbidden(
                &ent.structure_calendrier_comparer(&x, m.annee, m.annee)
                    .await
            ),
            lit,
            "comparer : {nom}"
        );
        let c = ent
            .structure_calendrier_evenement_creer(
                &x,
                saisie(
                    m.annee,
                    "evenement",
                    &format!("E {nom}"),
                    t + Duration::days(20),
                    t + Duration::days(20),
                ),
            )
            .await;
        assert_eq!(!est_forbidden(&c), ecrit, "créer : {nom}");
        let adopt = ent
            .structure_calendrier_adopter(&x, m.annee, true, false)
            .await;
        assert_eq!(!est_forbidden(&adopt), ecrit, "adopter : {nom}");
        let sem = ent
            .structure_semaine_forcer(&x, m.annee, d(2030, 1, 7), None, "x")
            .await;
        assert_eq!(!est_forbidden(&sem), ecrit, "semaines : {nom}");
        let f = ent
            .structure_fermeture_declarer(
                &x,
                fermeture(t + Duration::days(30), t + Duration::days(31)),
                true,
            )
            .await;
        assert_eq!(!est_forbidden(&f), ferme, "fermeture : {nom}");
    }
    // Isolation : l'établissement voisin ne voit, ne lit ni ne modifie rien de m.
    let dir = acteur_de(m.etab, &cast.direction);
    let dir_b = acteur_de(autre.etab, &cast.direction);
    let ev = ent
        .structure_calendrier_evenement_creer(
            &dir,
            saisie(
                m.annee,
                "pont",
                "Pont de m",
                t + Duration::days(40),
                t + Duration::days(40),
            ),
        )
        .await
        .unwrap()
        .evenement;
    let f = ent
        .structure_fermeture_declarer(
            &dir,
            fermeture(t + Duration::days(50), t + Duration::days(51)),
            false,
        )
        .await
        .unwrap()
        .evenement
        .unwrap();
    assert!(ent
        .structure_calendrier_evenements(&dir_b, m.annee, None, true)
        .await
        .unwrap()
        .is_empty());
    for r in [
        ent.structure_calendrier_evenement(&dir_b, ev.id)
            .await
            .map(|_| ()),
        ent.structure_calendrier_evenement_annuler(&dir_b, ev.id, "x")
            .await
            .map(|_| ()),
        ent.structure_calendrier_evenement_modifier(&dir_b, ev.id, EvenementModif::default())
            .await
            .map(|_| ()),
        ent.structure_fermeture_lever(&dir_b, f.id, "x", None)
            .await
            .map(|_| ()),
        ent.structure_fermeture_modifier(
            &dir_b,
            f.id,
            FermetureModif {
                au: Some(t + Duration::days(60)),
                motif: "x".into(),
                ..Default::default()
            },
        )
        .await
        .map(|_| ()),
    ] {
        assert!(est_domaine(&r, DomainError::NotFound), "{r:?}");
    }
    assert!(ent
        .structure_fermeture_historique(&dir_b, f.id)
        .await
        .unwrap()
        .is_empty());
    assert!(est_domaine(
        &ent.structure_calendrier_evenement_creer(&dir_b, saisie(m.annee, "pont", "x", t, t))
            .await,
        DomainError::NotFound
    ));
    // Le port de l'établissement voisin ne voit pas les fermetures de m.
    let mut c = ent.pool().acquire().await.unwrap();
    let j = calendrier::jour(
        &mut c,
        autre.etab,
        t + Duration::days(50),
        &Cible::default(),
    )
    .await
    .unwrap();
    assert!(j.evenements.is_empty());
    // Et l'événement publié reste dans son établissement.
    assert_eq!(
        nb_evenements_publies(&ent, autre.etab, "referentiel.fermeture_exceptionnelle").await,
        0
    );
}

#[tokio::test]
async fn rls_strict_tables_du_lot_s3() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let admin = ent.pool().clone();
    const ROLE: &str = "ent_rls_app";
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(&admin)
    .await
    .unwrap();
    let tables =
        "ent_calendrier_evenement, ent_calendrier_cible, ent_semaine_type_regle, ent_semaine, \
                  ent_fermeture_historique, ent_structure_journal";
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON {tables} TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap();

    let (m1, m2) = (etab_courant(&ent).await, etab_courant(&ent).await);
    let t = aujourdhui(&ent, m1.etab).await;
    for m in [&m1, &m2] {
        let dir = acteur_de(m.etab, &cast.direction);
        ent.structure_calendrier_evenement_creer(
            &dir,
            saisie(
                m.annee,
                "pont",
                "Pont",
                t + Duration::days(40),
                t + Duration::days(40),
            ),
        )
        .await
        .unwrap();
        ent.structure_fermeture_declarer(
            &dir,
            fermeture(t + Duration::days(50), t + Duration::days(51)),
            false,
        )
        .await
        .unwrap();
        ent.structure_semaines_definir(
            &dir,
            m.annee,
            RegleSemaineSaisie {
                etiquettes: vec!["A".into(), "B".into()],
                mode: "alternance_continue".into(),
                premiere_semaine: lundi_apres(t),
            },
        )
        .await
        .unwrap();
    }
    for table in [
        "ent_calendrier_evenement",
        "ent_semaine_type_regle",
        "ent_semaine",
        "ent_fermeture_historique",
    ] {
        let mut tx = TenantTx::begin_mode(&app, m1.etab, RlsMode::Strict)
            .await
            .unwrap();
        let n: i64 = sqlx::query(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap()
            .get(0);
        let autres: i64 = sqlx::query(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id <> $1"
        ))
        .bind(m1.etab)
        .fetch_one(&mut *tx)
        .await
        .unwrap()
        .get(0);
        assert_eq!(
            autres, 0,
            "{table} : lignes d'un autre établissement visibles"
        );
        assert!(n >= 1, "{table}");
        if table == "ent_calendrier_evenement" {
            let r = sqlx::query(
                "INSERT INTO ent_calendrier_evenement (id, etablissement_id, annee_id, type, libelle, du, au, effet)
                 VALUES ($1,$2,$3,'pont','x',$4,$4,'sans_cours')",
            )
            .bind(Uuid::new_v4())
            .bind(m2.etab)
            .bind(m2.annee)
            .bind(t)
            .execute(&mut *tx)
            .await;
            let err = r.expect_err("insertion inter-établissement refusée");
            assert_eq!(
                err.as_database_error().and_then(|d| d.code()).as_deref(),
                Some("42501")
            );
        }
        drop(tx);
    }
    for table in ["ent_calendrier_evenement", "ent_semaine"] {
        let n: i64 = sqlx::query(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&app)
            .await
            .unwrap()
            .get(0);
        assert_eq!(n, 0, "{table} sans TenantTx");
    }
}

/// Premier lundi strictement après `t` (règle de semaines valable dans l'année courante du test).
fn lundi_apres(t: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    t + Duration::days(7 - i64::from(t.weekday().num_days_from_monday()))
}
