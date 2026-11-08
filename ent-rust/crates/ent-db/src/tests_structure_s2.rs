//! Socle 5, lot S2 : profil, paramètres, sites, bâtiments, salles, lieux extérieurs,
//! disponibilités, import en lot. Base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`,
//! ignorés en local et **en échec en CI**.
//!
//! Pour ne pas se marcher dessus, la plupart des scénarios travaillent dans un **établissement
//! neuf** (identifiant aléatoire) avec le rôle d'un persona du labo.

use super::*;
use crate::ref_commun::Acteur;
use crate::seed::LaboPersona;
use crate::structure_etab::{structure, ProfilSaisie};
use crate::structure_lieux::{
    structure_disponibilite, Cible, DisponibiliteSaisie, FiltreSalles, LieuSaisie, SalleSaisie,
    SiteModif,
};
use crate::tenant_tx::{RlsMode, TenantTx};
use crate::tests_d022_bis::labo;
use chrono::{Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use domain_eleve::DomainError;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

fn acteur_de(etab: Uuid, p: &LaboPersona) -> Acteur<'static> {
    Acteur {
        etablissement_id: etab,
        user_id: p.user_id,
        role: p.role.as_str(),
    }
}

fn sfx() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}

fn est_forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn est_domaine<T>(r: &Result<T, EntDbError>, d: DomainError) -> bool {
    matches!(r, Err(EntDbError::Domain(e)) if *e == d)
}

fn profil_saisie(nom: &str) -> ProfilSaisie {
    ProfilSaisie {
        nom_officiel: nom.into(),
        nom_usage: None,
        logo_document_id: None,
        uai: Some("0751234A".into()),
        secteur: "public".into(),
        contrat_type: None,
        degres: vec!["second".into()],
        types: vec!["college".into()],
        academie_code: None,
        departement_code: Some("75".into()),
        zone_calendrier: None,
        regime_local: None,
        tutelle_libelle: None,
        adresse: None,
        telephone: None,
        courriel_contact: None,
        jours_ouvres: None,
        version_attendue: None,
    }
}

async fn nouveau_site(ent: &PgEnt, a: &Acteur<'_>, code: &str) -> Uuid {
    ent.ref_creer_site(a, code, &format!("Site {code}"), None, None)
        .await
        .unwrap()
        .id
}

fn salle(code: &str, site: Uuid) -> SalleSaisie {
    SalleSaisie {
        code: code.into(),
        libelle: format!("Salle {code}"),
        site_id: site,
        batiment_id: None,
        lieu_id: None,
        etage: None,
        type_salle: "banalisee".into(),
        capacite: Some(30),
        equipements: vec![],
        accessible_pmr: None,
        reservable: None,
    }
}

// ───────────────────────── Fuseau (critère de fin : outre-mer) ─────────────────────────

/// Établissement d'outre-mer : l'heure locale est juste, le port `structure::fuseau` la fournit,
/// la métropole reste le défaut, un nom inconnu est refusé et un changement en cours d'année
/// exige une confirmation.
#[tokio::test]
async fn outre_mer_heure_locale_juste_et_port_fuseau() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let reunion = Uuid::new_v4();
    let a = acteur_de(reunion, &cast.direction);
    let metropole = Uuid::new_v4();

    // Défaut : Europe/Paris, sans rien écrire.
    let mut c = ent.pool().acquire().await.unwrap();
    assert_eq!(
        structure::fuseau(&mut c, reunion).await.unwrap(),
        "Europe/Paris"
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_etablissement_parametre WHERE etablissement_id = $1",
    )
    .bind(reunion)
    .fetch_one(&mut *c)
    .await
    .unwrap();
    assert_eq!(n, 0, "lire le fuseau n'écrit rien");

    assert!(est_domaine(
        &ent.structure_fuseau_definir(&a, "Mars/Phobos", false).await,
        DomainError::ValeurInvalide
    ));
    assert_eq!(
        ent.structure_fuseau_definir(&a, "Indian/Reunion", false)
            .await
            .unwrap(),
        "Indian/Reunion"
    );

    // 21 h 00 UTC le 10 novembre = 01 h 00 le 11 à La Réunion (UTC+4), 22 h 00 le 10 à Paris.
    let instant = Utc.with_ymd_and_hms(2026, 11, 10, 21, 0, 0).unwrap();
    let local = structure::heure_locale(&mut c, reunion, instant)
        .await
        .unwrap();
    assert_eq!(local.to_string(), "2026-11-11 01:00:00");
    let paris = structure::heure_locale(&mut c, metropole, instant)
        .await
        .unwrap();
    assert_eq!(paris.to_string(), "2026-11-10 22:00:00");
    assert_eq!(
        structure::fuseau(&mut c, reunion).await.unwrap(),
        "Indian/Reunion"
    );
    assert_eq!(
        structure::fuseau(&mut c, metropole).await.unwrap(),
        "Europe/Paris"
    );
    // Même source que les dates métier du socle 4 (`ent_aujourdhui_etab`).
    let jour_base: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(reunion)
        .bind(instant)
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert_eq!(jour_base.to_string(), "2026-11-11");

    // Journal : avant/après.
    let j = ent.structure_journal(&a, "fuseau", None, 10).await.unwrap();
    assert_eq!(j.len(), 1);
    assert_eq!(j[0].apres.as_ref().unwrap()["fuseau"], "Indian/Reunion");
    drop(c);

    // Changement en cours d'année : confirmation exigée (établissement du labo, année courante).
    let labo_a = acteur_de(cast.etablissement_id, &cast.direction);
    ent.referentiel_annee_courante_assuree(cast.etablissement_id)
        .await
        .unwrap();
    let avant = {
        let mut c = ent.pool().acquire().await.unwrap();
        structure::fuseau(&mut c, cast.etablissement_id)
            .await
            .unwrap()
    };
    let cible = if avant == "America/Cayenne" {
        "Europe/Paris"
    } else {
        "America/Cayenne"
    };
    assert!(matches!(
        ent.structure_fuseau_definir(&labo_a, cible, false).await,
        Err(EntDbError::Domain(DomainError::Conflit))
    ));
    ent.structure_fuseau_definir(&labo_a, cible, true)
        .await
        .unwrap();
    ent.structure_fuseau_definir(&labo_a, &avant, true)
        .await
        .unwrap();
}

// ───────────────────────── Salle ↔ site (critère de fin) ─────────────────────────

/// Une salle est toujours liée à un site de son établissement : par le service ET par la base ;
/// aucun site n'est créé à la volée.
#[tokio::test]
async fn salle_toujours_liee_a_un_site_de_son_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (ea, eb) = (Uuid::new_v4(), Uuid::new_v4());
    let (a, b) = (
        acteur_de(ea, &cast.administratif),
        acteur_de(eb, &cast.administratif),
    );
    let sa = nouveau_site(&ent, &a, "PRINC").await;
    let sb = nouveau_site(&ent, &b, "PRINC").await;
    let user = cast.admin.user_id;

    // Service : site d'un autre établissement = introuvable ; aucune salle créée.
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, salle("A1", sb)).await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, salle("A1", Uuid::new_v4()))
            .await,
        DomainError::NotFound
    ));
    assert!(ent
        .structure_salles(&a, &FiltreSalles::default())
        .await
        .unwrap()
        .is_empty());

    // Base : clé étrangère composite, même en contournant le service.
    let r = sqlx::query("INSERT INTO ent_salle (id, etablissement_id, code, libelle, created_by_user_id, site_id) VALUES ($1,$2,'X1','X1',$3,$4)")
        .bind(Uuid::new_v4()).bind(ea).bind(user).bind(sb)
        .execute(ent.pool()).await;
    assert!(
        r.is_err(),
        "site d'un autre établissement refusé par la base"
    );
    // Base : une salle neuve sans site (ni texte) est refusée.
    let r = sqlx::query("INSERT INTO ent_salle (id, etablissement_id, code, libelle, created_by_user_id) VALUES ($1,$2,'X2','X2',$3)")
        .bind(Uuid::new_v4()).bind(ea).bind(user)
        .execute(ent.pool()).await;
    assert!(r.is_err(), "salle sans site refusée");
    // Base : un site texte inconnu est refusé et ne crée AUCUN site.
    let r = sqlx::query("INSERT INTO ent_salle (id, etablissement_id, code, libelle, created_by_user_id, site) VALUES ($1,$2,'X3','X3',$3,'Inconnu')")
        .bind(Uuid::new_v4()).bind(ea).bind(user)
        .execute(ent.pool()).await;
    assert!(r.is_err(), "site texte inconnu refusé");
    let nb_sites: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_site WHERE etablissement_id = $1")
            .bind(ea)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(nb_sites, 1, "aucune création implicite de site");
    // Base : un site texte qui désigne exactement un site existant du MÊME établissement est résolu
    // (chemin historique), jamais celui d'un autre.
    sqlx::query("INSERT INTO ent_salle (id, etablissement_id, code, libelle, created_by_user_id, site) VALUES ($1,$2,'H1','H1',$3,'princ')")
        .bind(Uuid::new_v4()).bind(ea).bind(user)
        .execute(ent.pool()).await.unwrap();
    let (site_id, miroir): (Uuid, String) = sqlx::query_as(
        "SELECT site_id, site FROM ent_salle WHERE etablissement_id = $1 AND code = 'H1'",
    )
    .bind(ea)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((site_id, miroir.as_str()), (sa, "PRINC"));
    // Base : détacher une salle de son site est refusé.
    let r = sqlx::query(
        "UPDATE ent_salle SET site_id = NULL WHERE etablissement_id = $1 AND code = 'H1'",
    )
    .bind(ea)
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "salle détachée de son site refusée");

    // Service : salle créée, typée, avec équipements ; bâtiment d'un autre site refusé.
    let s2 = nouveau_site(&ent, &a, "ANNEXE").await;
    let bat = ent
        .structure_batiment_creer(&a, sa, "b", "Bâtiment B")
        .await
        .unwrap();
    assert_eq!(bat.code, "B");
    let mut s = salle("lab1", sa);
    s.batiment_id = Some(bat.id);
    s.type_salle = "laboratoire".into();
    s.equipements = vec!["paillasses".into(), "paillasses".into(), " hotte ".into()];
    s.accessible_pmr = Some(true);
    let cree = ent.structure_salle_creer(&a, s.clone()).await.unwrap();
    assert_eq!(
        (cree.code.as_str(), cree.type_salle.as_str()),
        ("LAB1", "laboratoire")
    );
    assert_eq!(cree.equipements, vec!["paillasses", "hotte"]);
    assert_eq!(cree.site_code.as_deref(), Some("PRINC"));
    assert!(!cree.reservable, "pas réservable par défaut");
    // Le bâtiment du site A ne va pas avec une salle de l'annexe.
    let mut autre = salle("LAB2", s2);
    autre.batiment_id = Some(bat.id);
    assert!(ent.structure_salle_creer(&a, autre).await.is_err());
    // Code déjà pris : conflit, jamais de doublon.
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, salle("lab1", sa)).await,
        DomainError::Conflit
    ));
    // Type inconnu / capacité hors borne / salle de lieu extérieur non « exterieur ».
    let mut t = salle("T1", sa);
    t.type_salle = "piscine".into();
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, t).await,
        DomainError::ValeurInvalide
    ));
    let mut c = salle("C1", sa);
    c.capacite = Some(501);
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, c).await,
        DomainError::ValeurInvalide
    ));

    // Le code ne change pas ; changer de site exige de redire le bâtiment (ici : retiré).
    let mut m = salle("LAB1", s2);
    m.type_salle = "laboratoire".into();
    let deplacee = ent.structure_salle_modifier(&a, cree.id, m).await.unwrap();
    assert_eq!(deplacee.site_code.as_deref(), Some("ANNEXE"));
    assert!(deplacee.batiment_id.is_none());
    assert!(est_domaine(
        &ent.structure_salle_modifier(&a, cree.id, salle("AUTRE", s2))
            .await,
        DomainError::ValeurInvalide
    ));
    // Établissement B ne voit ni ne modifie la salle de A.
    assert!(est_domaine(
        &ent.structure_salle_modifier(&b, cree.id, salle("LAB1", sb))
            .await,
        DomainError::NotFound
    ));
    assert!(ent
        .structure_salles(&b, &FiltreSalles::default())
        .await
        .unwrap()
        .is_empty());
}

/// Lieu extérieur : salle de type « exterieur » seulement ; fermeture refusée tant qu'une salle
/// ouverte en dépend ; disponibilité en fenêtres + indisponibilité (travaux de la mairie).
#[tokio::test]
async fn lieu_exterieur_disponibilites_et_indisponibilites() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.direction);
    let site = nouveau_site(&ent, &a, "PRINC").await;
    let lieu = ent
        .structure_lieu_creer(
            &a,
            LieuSaisie {
                code: "gym-mairie".into(),
                libelle: "Gymnase municipal".into(),
                adresse: Some("12 rue du Stade".into()),
                proprietaire: Some("Mairie".into()),
                convention_document_id: None,
                trajet_minutes: Some(12),
                mode_trajet: Some("pied".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(lieu.code, "GYM-MAIRIE");
    assert!(est_domaine(
        &ent.structure_lieu_creer(
            &a,
            LieuSaisie {
                code: "gym-mairie".into(),
                libelle: "x".into(),
                adresse: None,
                proprietaire: None,
                convention_document_id: None,
                trajet_minutes: None,
                mode_trajet: None
            }
        )
        .await,
        DomainError::Conflit
    ));
    assert!(ent
        .structure_lieu_creer(
            &a,
            LieuSaisie {
                code: "L2".into(),
                libelle: "x".into(),
                adresse: None,
                proprietaire: None,
                convention_document_id: None,
                trajet_minutes: Some(500),
                mode_trajet: None
            }
        )
        .await
        .is_err());

    // Salle de gymnase municipal : exige le type « exterieur ».
    let mut s = salle("GYM", site);
    s.lieu_id = Some(lieu.id);
    s.type_salle = "gymnase".into();
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, s.clone()).await,
        DomainError::ValeurInvalide
    ));
    s.type_salle = "exterieur".into();
    let gym = ent.structure_salle_creer(&a, s).await.unwrap();
    assert_eq!(gym.lieu_code.as_deref(), Some("GYM-MAIRIE"));
    assert!(est_domaine(
        &ent.structure_lieu_fermer(&a, lieu.id, NaiveDate::from_ymd_opt(2027, 7, 1).unwrap())
            .await,
        DomainError::Conflit
    ));

    // Fenêtres : mardi 08:00–12:00 seulement (conventionnées) ; le lieu fait foi pour sa salle.
    let mardi = |h0: u32, h1: u32| DisponibiliteSaisie {
        jour_semaine: 2,
        heure_debut: NaiveTime::from_hms_opt(h0, 0, 0).unwrap(),
        heure_fin: NaiveTime::from_hms_opt(h1, 0, 0).unwrap(),
        du: None,
        au: None,
        libelle: Some("créneau convention".into()),
    };
    assert!(ent
        .structure_disponibilite_ajouter(
            &a,
            Cible::Lieu(lieu.id),
            DisponibiliteSaisie {
                jour_semaine: 8,
                ..mardi(8, 12)
            }
        )
        .await
        .is_err());
    assert!(ent
        .structure_disponibilite_ajouter(&a, Cible::Lieu(lieu.id), mardi(12, 8))
        .await
        .is_err());
    assert!(ent
        .structure_disponibilite_ajouter(&a, Cible::Lieu(Uuid::new_v4()), mardi(8, 12))
        .await
        .is_err());
    // 2026-11-10 est un mardi ; heures UTC = heures de Paris −1 en novembre.
    let t = |h: u32, m: u32| Utc.with_ymd_and_hms(2026, 11, 10, h, m, 0).unwrap();
    let mut c = ent.pool().acquire().await.unwrap();
    let cible_salle = Cible::Salle(gym.id);
    assert!(
        structure_disponibilite::disponible(&mut c, ea, cible_salle, t(8, 0), t(9, 0))
            .await
            .unwrap(),
        "sans fenêtre : disponible"
    );
    let d = ent
        .structure_disponibilite_ajouter(&a, Cible::Lieu(lieu.id), mardi(8, 12))
        .await
        .unwrap();
    assert!(
        structure_disponibilite::disponible(&mut c, ea, cible_salle, t(8, 0), t(9, 0))
            .await
            .unwrap(),
        "09h–10h Paris : dans la fenêtre"
    );
    assert!(
        !structure_disponibilite::disponible(&mut c, ea, cible_salle, t(13, 0), t(14, 0))
            .await
            .unwrap(),
        "hors fenêtre"
    );
    let mercredi = Utc.with_ymd_and_hms(2026, 11, 11, 8, 0, 0).unwrap();
    assert!(
        !structure_disponibilite::disponible(
            &mut c,
            ea,
            cible_salle,
            mercredi,
            mercredi + Duration::hours(1)
        )
        .await
        .unwrap(),
        "autre jour"
    );
    // Travaux de la mairie sur le lieu : la salle du lieu devient indisponible.
    let rap = ent
        .structure_indisponibilite_declarer(
            &a,
            Cible::Lieu(lieu.id),
            t(0, 0),
            t(0, 0) + Duration::days(14),
            "travaux",
            Some("toiture"),
        )
        .await
        .unwrap();
    assert_eq!(rap.indisponibilites.len(), 1);
    assert!(
        !structure_disponibilite::disponible(&mut c, ea, cible_salle, t(8, 0), t(9, 0))
            .await
            .unwrap(),
        "travaux"
    );
    ent.structure_indisponibilite_annuler(&a, rap.indisponibilites[0].id, false)
        .await
        .unwrap();
    assert!(
        structure_disponibilite::disponible(&mut c, ea, cible_salle, t(8, 0), t(9, 0))
            .await
            .unwrap(),
        "annulée"
    );
    // Intervalle qui traverse minuit ou à l'envers : refusé, pas deviné.
    assert!(
        structure_disponibilite::disponible(&mut c, ea, cible_salle, t(22, 30), t(23, 30))
            .await
            .is_err()
    );
    assert!(
        structure_disponibilite::disponible(&mut c, ea, cible_salle, t(9, 0), t(8, 0))
            .await
            .is_err()
    );
    // Fenêtre annulée : la salle redevient libre de toute fenêtre.
    ent.structure_disponibilite_annuler(&a, d.id).await.unwrap();
    assert!(
        structure_disponibilite::disponible(&mut c, ea, cible_salle, t(13, 0), t(14, 0))
            .await
            .unwrap()
    );
    assert!(ent
        .structure_disponibilites(&a, Cible::Lieu(lieu.id))
        .await
        .unwrap()
        .is_empty());
    // Mauvaise période / motif / indisponibilité modifiée à la main : refus.
    assert!(ent
        .structure_indisponibilite_declarer(&a, cible_salle, t(9, 0), t(8, 0), "travaux", None)
        .await
        .is_err());
    assert!(ent
        .structure_indisponibilite_declarer(&a, cible_salle, t(8, 0), t(9, 0), "caprice", None)
        .await
        .is_err());
    let r = sqlx::query(
        "UPDATE ent_salle_indisponibilite SET motif_code = 'autre' WHERE etablissement_id = $1",
    )
    .bind(ea)
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "une indisponibilité ne se modifie pas");
    let r = sqlx::query("DELETE FROM ent_salle_indisponibilite WHERE etablissement_id = $1")
        .bind(ea)
        .execute(ent.pool())
        .await;
    assert!(r.is_err(), "une indisponibilité ne se supprime pas");

    // Fermeture du lieu une fois la salle fermée.
    ent.structure_salle_fermer(&a, gym.id, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
        .await
        .unwrap();
    ent.structure_lieu_fermer(&a, lieu.id, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
        .await
        .unwrap();
    assert!(
        !structure_disponibilite::disponible(
            &mut c,
            ea,
            cible_salle,
            Utc.with_ymd_and_hms(2026, 12, 8, 8, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 12, 8, 9, 0, 0).unwrap()
        )
        .await
        .unwrap(),
        "fermée"
    );
}

/// STR-110 : plusieurs salles indisponibles en un geste, tout ou rien, annulables ensemble ;
/// créneaux touchés signalés.
#[tokio::test]
async fn indisponibilite_de_plusieurs_salles_en_un_geste() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.administratif);
    let site = nouveau_site(&ent, &a, "PRINC").await;
    let mut ids = vec![];
    for c in ["S1", "S2", "S3"] {
        ids.push(
            ent.structure_salle_creer(&a, salle(c, site))
                .await
                .unwrap()
                .id,
        );
    }
    // Un créneau hebdomadaire du mardi occupe S1.
    sqlx::query("INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin, salle) VALUES ($1,$2,'6A','Maths',$3,2,'08:00','09:00','s1')")
        .bind(Uuid::new_v4()).bind(ea).bind(cast.prof.user_id)
        .execute(ent.pool()).await.unwrap();
    let du = Utc.with_ymd_and_hms(2026, 11, 10, 6, 0, 0).unwrap(); // un mardi
    let au = du + Duration::hours(10);
    // Une salle inconnue annule le lot entier.
    let mauvais = ent
        .structure_indisponibilite_lot(
            &a,
            vec![ids[0], Uuid::new_v4()],
            du,
            au,
            "examen",
            None,
            None,
        )
        .await;
    assert!(mauvais.is_err());
    let rien = ent
        .structure_indisponibilites(&a, Cible::Salle(ids[0]), du - Duration::days(1))
        .await
        .unwrap();
    assert!(rien.is_empty(), "tout ou rien");
    let rap = ent
        .structure_indisponibilite_lot(
            &a,
            ids.clone(),
            du,
            au,
            "examen",
            Some("brevet blanc"),
            Some(Uuid::new_v4()),
        )
        .await
        .unwrap();
    assert_eq!(rap.indisponibilites.len(), 3);
    let lot = rap.lot_id.unwrap();
    assert!(rap.indisponibilites.iter().all(|i| i.lot_id == Some(lot)));
    assert_eq!(
        rap.creneaux_touches, 1,
        "le créneau du mardi en S1 est signalé"
    );
    // Un autre jour (mercredi) : aucun créneau touché.
    let mer = ent
        .structure_indisponibilite_lot(
            &a,
            vec![ids[0]],
            du + Duration::days(1),
            au + Duration::days(1),
            "travaux",
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(mer.creneaux_touches, 0);
    // Annulation d'une seule, puis de tout le lot.
    assert_eq!(
        ent.structure_indisponibilite_annuler(&a, rap.indisponibilites[0].id, false)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        ent.structure_indisponibilite_annuler(&a, rap.indisponibilites[1].id, true)
            .await
            .unwrap(),
        2
    );
    // La salle ne peut pas être fermée tant qu'un créneau l'utilise.
    assert!(est_domaine(
        &ent.structure_salle_fermer(&a, ids[0], NaiveDate::from_ymd_opt(2027, 1, 1).unwrap())
            .await,
        DomainError::Conflit
    ));
    ent.structure_salle_fermer(&a, ids[1], NaiveDate::from_ymd_opt(2027, 1, 1).unwrap())
        .await
        .unwrap();
    assert!(
        ent.structure_salles(&a, &FiltreSalles::default())
            .await
            .unwrap()
            .iter()
            .all(|s| s.id != ids[1]),
        "fermée masquée par défaut"
    );
    assert_eq!(
        ent.structure_salles(
            &a,
            &FiltreSalles {
                site_id: None,
                inclure_fermees: true
            }
        )
        .await
        .unwrap()
        .len(),
        3
    );
}

// ───────────────────────── Sites ─────────────────────────

#[tokio::test]
async fn site_modification_validee_et_fermeture_gardee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.direction);
    let site = nouveau_site(&ent, &a, "PRINC").await;
    let m = SiteModif {
        libelle: Some("Collège principal".into()),
        uai: Some("0751234a".into()),
        degres: Some(vec!["second".into()]),
        couleur: Some("#336699".into()),
        fuseau_horaire: Some("Pacific/Noumea".into()),
        adresse: Some("1 rue de l'École".into()),
    };
    let s = ent.structure_site_modifier(&a, site, m).await.unwrap();
    assert_eq!(
        s.uai.as_deref(),
        Some("0751234A"),
        "UAI normalisé en majuscules"
    );
    assert_eq!(s.fuseau_horaire.as_deref(), Some("Pacific/Noumea"));
    for mauvais in [
        SiteModif {
            uai: Some("123".into()),
            ..Default::default()
        },
        SiteModif {
            couleur: Some("rouge".into()),
            ..Default::default()
        },
        SiteModif {
            degres: Some(vec!["maternelle".into()]),
            ..Default::default()
        },
        SiteModif {
            fuseau_horaire: Some("Mars/Phobos".into()),
            ..Default::default()
        },
        SiteModif {
            libelle: Some("  ".into()),
            ..Default::default()
        },
    ] {
        assert!(est_domaine(
            &ent.structure_site_modifier(&a, site, mauvais).await,
            DomainError::ValeurInvalide
        ));
    }
    // L'UAI vide efface ; absent conserve.
    let s = ent
        .structure_site_modifier(
            &a,
            site,
            SiteModif {
                adresse: Some(String::new()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(s.adresse.is_none());
    assert_eq!(s.uai.as_deref(), Some("0751234A"));
    // Fermeture : refusée tant qu'une salle ouverte ou une classe ouverte y est.
    let salle_id = ent
        .structure_salle_creer(&a, salle("S1", site))
        .await
        .unwrap()
        .id;
    let jour = NaiveDate::from_ymd_opt(2027, 7, 1).unwrap();
    assert!(est_domaine(
        &ent.structure_site_fermer(&a, site, jour).await,
        DomainError::Conflit
    ));
    ent.structure_salle_fermer(&a, salle_id, jour)
        .await
        .unwrap();
    let ferme = ent.structure_site_fermer(&a, site, jour).await.unwrap();
    assert_eq!((ferme.actif, ferme.ferme_le), (false, Some(jour)));
    // On ne crée plus de salle ni de bâtiment dans un site fermé.
    assert!(est_domaine(
        &ent.structure_salle_creer(&a, salle("S2", site)).await,
        DomainError::Conflit
    ));
    assert!(est_domaine(
        &ent.structure_batiment_creer(&a, site, "B", "B").await,
        DomainError::Conflit
    ));
    ent.structure_site_rouvrir(&a, site).await.unwrap();
    // Journal du site : modification, modification, fermeture, réouverture.
    let j = ent
        .structure_journal(&a, "site", Some(&site.to_string()), 50)
        .await
        .unwrap();
    assert_eq!(j.len(), 4);
}

// ───────────────────────── Import en lot (STR-08, STR-110) ─────────────────────────

#[tokio::test]
async fn import_de_salles_apercu_rejets_et_idempotence() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.administratif);
    let site = nouveau_site(&ent, &a, "PRINC").await;
    ent.structure_batiment_creer(&a, site, "A", "Bâtiment A")
        .await
        .unwrap();
    let csv =
        "code;libelle;site;batiment;etage;type;capacite;equipements;accessible_pmr;reservable\n\
               a101;Salle A101;PRINC;A;1;banalisee;30;vidéoprojecteur|tableau blanc;oui;non\n\
               A102;Labo de physique;princ;A;1;laboratoire;24;paillasses;non;\n\
               A101;Doublon;PRINC;;;;;;;\n\
               B1;Salle sans site connu;AILLEURS;;;;;;;\n\
               B2;Salle bâtiment faux;PRINC;Z;;;;;;\n\
               B3;Type faux;PRINC;;;piscine;;;;\n\
               B4;Capacité folle;PRINC;;;;999;;;\n\
               B5;Oui non faux;PRINC;;;;;;peut-être;\n\
               ;Sans code;PRINC;;;;;;;\n\
               B6;Cellule \u{7}piégée;PRINC;;;;;;;\n";
    // Aperçu : rapport complet, aucune écriture.
    let ap = ent.structure_salles_importer(&a, csv, true).await.unwrap();
    assert!(ap.apercu);
    assert_eq!(
        (ap.lignes, ap.creees, ap.modifiees, ap.inchangees),
        (10, 2, 0, 0)
    );
    let motifs: Vec<(usize, String)> = ap
        .rejets
        .iter()
        .map(|r| (r.ligne, r.raison.clone()))
        .collect();
    assert_eq!(ap.rejets.len(), 8, "{motifs:?}");
    assert!(ap
        .rejets
        .iter()
        .any(|r| r.ligne == 4 && r.raison.contains("déjà présent")));
    assert!(ap
        .rejets
        .iter()
        .any(|r| r.ligne == 5 && r.raison.contains("site inconnu")));
    assert!(
        ent.structure_salles(&a, &FiltreSalles::default())
            .await
            .unwrap()
            .is_empty(),
        "l'aperçu n'écrit rien"
    );
    let nb_journal = ent
        .structure_journal(&a, "import_salles", None, 10)
        .await
        .unwrap()
        .len();
    assert_eq!(nb_journal, 0);

    // Import réel.
    let r = ent.structure_salles_importer(&a, csv, false).await.unwrap();
    assert!(!r.apercu);
    assert_eq!(
        (r.creees, r.modifiees, r.inchangees, r.rejets.len()),
        (2, 0, 0, 8)
    );
    let salles = ent
        .structure_salles(&a, &FiltreSalles::default())
        .await
        .unwrap();
    assert_eq!(salles.len(), 2);
    let a101 = salles.iter().find(|s| s.code == "A101").unwrap();
    assert_eq!(
        (
            a101.batiment_code.as_deref(),
            a101.accessible_pmr,
            a101.reservable
        ),
        (Some("A"), Some(true), false)
    );
    assert_eq!(a101.equipements, vec!["vidéoprojecteur", "tableau blanc"]);
    assert_eq!(
        ent.structure_sites(&a).await.unwrap().len(),
        1,
        "aucun site créé par l'import"
    );
    assert_eq!(
        ent.structure_journal(&a, "import_salles", None, 10)
            .await
            .unwrap()
            .len(),
        1
    );

    // Idempotent : second passage = inchangées ; une cellule vide garde la valeur ; une mise à jour compte.
    let r2 = ent.structure_salles_importer(&a, csv, false).await.unwrap();
    assert_eq!((r2.creees, r2.modifiees, r2.inchangees), (0, 0, 2));
    let maj = "code,libelle,site,capacite\nA101,,PRINC,40\n";
    let r3 = ent.structure_salles_importer(&a, maj, false).await.unwrap();
    assert_eq!(
        (r3.creees, r3.modifiees, r3.inchangees, r3.rejets.len()),
        (0, 1, 0, 0)
    );
    let a101 = ent
        .structure_salles(&a, &FiltreSalles::default())
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.code == "A101")
        .unwrap();
    assert_eq!(
        (
            a101.capacite,
            a101.libelle.as_str(),
            a101.batiment_code.as_deref()
        ),
        (Some(40), "Salle A101", Some("A"))
    );

    // Fichier entier refusé : colonne inconnue, colonne obligatoire absente, trop gros, guillemet ouvert.
    for mauvais in [
        "code;libelle;site;couleur\nA;B;PRINC;rouge\n",
        "code;libelle\nA;B\n",
        "",
        "code;libelle;site\n\"A;B;PRINC\n",
    ] {
        assert!(
            est_domaine(
                &ent.structure_salles_importer(&a, mauvais, true).await,
                DomainError::ValeurInvalide
            ),
            "{mauvais:?}"
        );
    }
    let enorme = format!("code;libelle;site\n{}", "X;Y;PRINC\n".repeat(2001));
    assert!(est_domaine(
        &ent.structure_salles_importer(&a, &enorme, true).await,
        DomainError::ValeurInvalide
    ));
    // Un établissement voisin ne peut ni voir ni modifier ces salles par l'import (autre site, même code).
    let eb = Uuid::new_v4();
    let b = acteur_de(eb, &cast.administratif);
    nouveau_site(&ent, &b, "PRINC").await;
    let rb = ent
        .structure_salles_importer(&b, "code;libelle;site\nA101;Chez B;PRINC\n", false)
        .await
        .unwrap();
    assert_eq!(rb.creees, 1, "le code A101 est libre chez B");
    let chez_a = ent
        .structure_salles(&a, &FiltreSalles::default())
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.code == "A101")
        .unwrap();
    assert_eq!(chez_a.libelle, "Salle A101");
}

// ───────────────────────── Profil, préréglages, paramètres ─────────────────────────

#[tokio::test]
async fn profil_versionne_journalise_et_valide() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.direction);
    // Rien n'existe, rien n'est créé en lisant.
    assert!(ent.structure_profil(&a).await.unwrap().is_none());
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_etablissement_profil WHERE etablissement_id = $1",
    )
    .bind(ea)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 0);

    let p1 = ent
        .structure_profil_definir(&a, profil_saisie("Collège Victor Hugo"))
        .await
        .unwrap();
    assert_eq!(
        (p1.version, p1.fuseau.as_str(), p1.secteur.as_str()),
        (1, "Europe/Paris", "public")
    );
    assert!(
        p1.jours_ouvres["mer"].as_array().unwrap().len() == 1,
        "semaine type par défaut : mercredi matin"
    );
    let mut s2 = profil_saisie("Collège Victor Hugo");
    s2.nom_usage = Some("Hugo".into());
    s2.version_attendue = Some(1);
    let p2 = ent.structure_profil_definir(&a, s2.clone()).await.unwrap();
    assert_eq!(p2.version, 2);
    // Version périmée : conflit, rien n'est écrasé.
    assert!(est_domaine(
        &ent.structure_profil_definir(&a, s2).await,
        DomainError::Conflit
    ));
    // Validations.
    let mut s = profil_saisie("X");
    s.uai = Some("12".into());
    assert!(est_domaine(
        &ent.structure_profil_definir(&a, s).await,
        DomainError::ValeurInvalide
    ));
    let mut s = profil_saisie("X");
    s.contrat_type = Some("simple".into()); // public + contrat : incohérent
    assert!(est_domaine(
        &ent.structure_profil_definir(&a, s).await,
        DomainError::ValeurInvalide
    ));
    let mut s = profil_saisie("X");
    s.secteur = "prive_sous_contrat".into();
    s.contrat_type = Some("association".into());
    s.jours_ouvres = Some(json!({"lun": ["matin"]}));
    let prive = ent.structure_profil_definir(&a, s).await.unwrap();
    assert_eq!(prive.contrat_type.as_deref(), Some("association"));
    for mauvais in [
        ProfilSaisie {
            secteur: "associatif".into(),
            ..profil_saisie("X")
        },
        ProfilSaisie {
            degres: vec!["maternelle".into()],
            ..profil_saisie("X")
        },
        ProfilSaisie {
            nom_officiel: "  ".into(),
            ..profil_saisie("X")
        },
        ProfilSaisie {
            regime_local: Some("bretagne".into()),
            ..profil_saisie("X")
        },
        ProfilSaisie {
            jours_ouvres: Some(json!({})),
            ..profil_saisie("X")
        },
        ProfilSaisie {
            departement_code: Some("7".into()),
            ..profil_saisie("X")
        },
    ] {
        assert!(est_domaine(
            &ent.structure_profil_definir(&a, mauvais).await,
            DomainError::ValeurInvalide
        ));
    }
    let j = ent.structure_journal(&a, "profil", None, 10).await.unwrap();
    assert_eq!(j.len(), 3);
    assert!(j[2].avant.is_none() && j[2].apres.is_some());
    assert_eq!(j[0].avant.as_ref().unwrap()["version"], 2);
    // Le journal est en ajout seul, même pour le propriétaire de la base.
    let r = sqlx::query("DELETE FROM ent_structure_journal WHERE etablissement_id = $1")
        .bind(ea)
        .execute(ent.pool())
        .await;
    assert!(r.is_err());
    let r =
        sqlx::query("UPDATE ent_structure_journal SET action = 'x' WHERE etablissement_id = $1")
            .bind(ea)
            .execute(ent.pool())
            .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn preregles_apercu_puis_une_seule_application_sans_creation() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.administratif);
    // Pas d'identité : on ne devine pas, conflit.
    assert!(est_domaine(
        &ent.structure_prereglage_appliquer(&a, "college_public", true)
            .await,
        DomainError::Conflit
    ));
    ent.structure_profil_definir(&a, profil_saisie("Collège"))
        .await
        .unwrap();
    assert!(est_domaine(
        &ent.structure_prereglage_appliquer(&a, "inexistant", true)
            .await,
        DomainError::NotFound
    ));
    // Un paramètre déjà personnalisé n'est jamais écrasé.
    ent.structure_parametre_definir(&a, "structure.duree_seance_defaut_min", json!(50), None)
        .await
        .unwrap();
    let ap = ent
        .structure_prereglage_appliquer(&a, "etablissement_prive_sous_contrat", true)
        .await
        .unwrap();
    assert!(!ap.applique);
    assert_eq!(
        ap.parametres_conserves,
        vec!["structure.duree_seance_defaut_min"]
    );
    assert!(
        ent.structure_profil(&a)
            .await
            .unwrap()
            .unwrap()
            .profil_preregle
            .is_none(),
        "aperçu sans effet"
    );
    let r = ent
        .structure_prereglage_appliquer(&a, "etablissement_prive_sous_contrat", false)
        .await
        .unwrap();
    assert!(r.applique);
    let p = ent.structure_profil(&a).await.unwrap().unwrap();
    assert_eq!(
        (p.secteur.as_str(), p.profil_preregle.as_deref()),
        (
            "prive_sous_contrat",
            Some("etablissement_prive_sous_contrat")
        )
    );
    assert!(p.jours_ouvres["sam"].is_array(), "samedi matin du privé");
    let v = ent.structure_parametres(&a).await.unwrap();
    assert_eq!(
        v.iter()
            .find(|x| x.definition.cle == "structure.duree_seance_defaut_min")
            .unwrap()
            .valeur,
        json!(50)
    );
    // Jamais réappliqué.
    assert!(est_domaine(
        &ent.structure_prereglage_appliquer(&a, "college_public", false)
            .await,
        DomainError::Conflit
    ));
    // Aucune création implicite de site, salle, bâtiment, lieu.
    for t in [
        "ent_site",
        "ent_salle",
        "ent_batiment",
        "ent_lieu",
        "ent_classe",
    ] {
        let n: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {t} WHERE etablissement_id = $1"
        ))
        .bind(ea)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert_eq!(n, 0, "{t}");
    }
}

#[tokio::test]
async fn parametres_connus_valides_versionnes_et_journalises() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let a = acteur_de(ea, &cast.direction);
    // Lecture : défauts du registre, aucune ligne écrite.
    let tous = ent.structure_parametres(&a).await.unwrap();
    assert!(tous.iter().all(|p| !p.personnalise && p.version == 0));
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_parametre WHERE etablissement_id = $1")
            .bind(ea)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 0);
    let cle = "structure.duree_seance_defaut_min";
    // Clé inconnue, type faux, hors bornes.
    for (c, v) in [
        ("structure.inconnue", json!(1)),
        (cle, json!("long")),
        (cle, json!(5)),
        (cle, json!(241)),
        ("structure.demi_journee_coupure", json!("midi")),
    ] {
        assert!(
            est_domaine(
                &ent.structure_parametre_definir(&a, c, v, None).await,
                DomainError::ValeurInvalide
            ),
            "{c}"
        );
    }
    let p1 = ent
        .structure_parametre_definir(&a, cle, json!(50), Some(0))
        .await
        .unwrap();
    assert_eq!((p1.version, p1.valeur.clone()), (1, json!(50)));
    let p2 = ent
        .structure_parametre_definir(&a, cle, json!(60), Some(1))
        .await
        .unwrap();
    assert_eq!(p2.version, 2);
    // Modification concurrente (version périmée) : conflit.
    assert!(est_domaine(
        &ent.structure_parametre_definir(&a, cle, json!(45), Some(1))
            .await,
        DomainError::Conflit
    ));
    let mut c = ent.pool().acquire().await.unwrap();
    assert_eq!(
        structure::parametre(&mut c, ea, cle).await.unwrap(),
        json!(60)
    );
    assert_eq!(
        structure::parametre(&mut c, ea, "structure.trajet_defaut_min")
            .await
            .unwrap(),
        json!(0),
        "défaut"
    );
    assert!(structure::parametre(&mut c, ea, "structure.inconnue")
        .await
        .is_err());
    let j = ent
        .structure_journal(&a, "parametre", Some(cle), 10)
        .await
        .unwrap();
    assert_eq!(j.len(), 2);
    assert_eq!(j[0].avant.as_ref().unwrap()["valeur"], 50);
    assert_eq!(j[0].apres.as_ref().unwrap()["version"], 2);
    // Valeur par défaut « réservable » lue à la création d'une salle.
    let site = nouveau_site(&ent, &a, "PRINC").await;
    ent.structure_parametre_definir(
        &a,
        "structure.salles_reservables_par_defaut",
        json!(true),
        None,
    )
    .await
    .unwrap();
    assert!(
        ent.structure_salle_creer(&a, salle("R1", site))
            .await
            .unwrap()
            .reservable
    );
    let mut non = salle("R2", site);
    non.reservable = Some(false);
    assert!(!ent.structure_salle_creer(&a, non).await.unwrap().reservable);
}

// ───────────────────────── Droits ─────────────────────────

/// Matrice : lecture pour tout le personnel, écriture pour administrateur / direction /
/// administratif ; les familles n'ont rien. Contrôlé côté serveur, sur chaque famille d'action.
#[tokio::test]
async fn matrice_de_droits_structure() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ea = Uuid::new_v4();
    let gest = acteur_de(ea, &cast.administratif);
    let site = nouveau_site(&ent, &gest, "PRINC").await;
    let salle_id = ent
        .structure_salle_creer(&gest, salle("S1", site))
        .await
        .unwrap()
        .id;
    let jour = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
    let du = Utc.with_ymd_and_hms(2026, 11, 10, 8, 0, 0).unwrap();

    let personas: [(&str, &LaboPersona, bool, bool); 7] = [
        ("admin", &cast.admin, true, true),
        ("direction", &cast.direction, true, true),
        ("administratif", &cast.administratif, true, true),
        ("compta", &cast.compta, true, false),
        ("prof", &cast.prof, true, false),
        ("cpe", &cast.cpe, true, false),
        ("parent", &cast.parent_lea, false, false),
    ];
    for (nom, p, lit, ecrit) in personas {
        let x = acteur_de(ea, p);
        assert_eq!(
            !est_forbidden(&ent.structure_sites(&x).await),
            lit,
            "lire sites : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_salles(&x, &FiltreSalles::default()).await),
            lit,
            "lire salles : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_lieux(&x).await),
            lit,
            "lire lieux : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_profil(&x).await),
            lit,
            "lire profil : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_parametres(&x).await),
            lit,
            "lire paramètres : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_journal(&x, "profil", None, 5).await),
            lit,
            "lire journal : {nom}"
        );
        // Écritures : jamais « Forbidden » pour un gestionnaire, toujours pour les autres.
        let ecritures = [
            ent.structure_profil_definir(&x, profil_saisie("Collège"))
                .await
                .map(|_| ()),
            ent.structure_fuseau_definir(&x, "Europe/Paris", false)
                .await
                .map(|_| ()),
            ent.structure_parametre_definir(&x, "structure.trajet_defaut_min", json!(5), None)
                .await
                .map(|_| ()),
            ent.structure_prereglage_appliquer(&x, "college_public", true)
                .await
                .map(|_| ()),
            ent.structure_site_modifier(&x, site, SiteModif::default())
                .await
                .map(|_| ()),
            ent.structure_site_fermer(&x, site, jour).await.map(|_| ()),
            ent.structure_batiment_creer(&x, site, &format!("B{}", sfx()), "Bât")
                .await
                .map(|_| ()),
            ent.structure_salle_creer(&x, salle(&format!("Z{}", sfx()), site))
                .await
                .map(|_| ()),
            ent.structure_salle_modifier(&x, salle_id, salle("S1", site))
                .await
                .map(|_| ()),
            ent.structure_salle_fermer(&x, Uuid::new_v4(), jour)
                .await
                .map(|_| ()),
            ent.structure_salles_importer(&x, "code;libelle;site\nQ1;Q;PRINC\n", true)
                .await
                .map(|_| ()),
            ent.structure_lieu_creer(
                &x,
                LieuSaisie {
                    code: format!("L{}", sfx()),
                    libelle: "L".into(),
                    adresse: None,
                    proprietaire: None,
                    convention_document_id: None,
                    trajet_minutes: None,
                    mode_trajet: None,
                },
            )
            .await
            .map(|_| ()),
            ent.structure_indisponibilite_declarer(
                &x,
                Cible::Salle(salle_id),
                du,
                du + Duration::hours(1),
                "travaux",
                None,
            )
            .await
            .map(|_| ()),
            ent.structure_indisponibilite_lot(
                &x,
                vec![salle_id],
                du + Duration::days(3),
                du + Duration::days(3) + Duration::hours(1),
                "examen",
                None,
                None,
            )
            .await
            .map(|_| ()),
        ];
        for (i, r) in ecritures.iter().enumerate() {
            assert_eq!(!est_forbidden(r), ecrit, "écriture n°{i} : {nom} ({r:?})");
        }
    }
    // L'écriture refusée n'a rien laissé : une seule salle (celle du gestionnaire) + celles des gestionnaires.
    let n = ent
        .structure_salles(&gest, &FiltreSalles::default())
        .await
        .unwrap()
        .len();
    assert_eq!(
        n,
        1 + 3,
        "S1 + une salle par gestionnaire (admin, direction, administratif)"
    );
}

// ───────────────────────── RLS (rôle non propriétaire) ─────────────────────────

/// Les tables du lot sont cloisonnées par la base elle-même : un rôle sans privilège de
/// contournement, en mode strict, ne voit et n'écrit que son établissement.
#[tokio::test]
async fn rls_strict_tables_du_lot_s2() {
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
    .execute(&admin).await.unwrap();
    let tables = "ent_etablissement_profil, ent_parametre, ent_structure_journal, ent_batiment, ent_lieu, ent_disponibilite, ent_salle_indisponibilite";
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

    let (ea, eb) = (Uuid::new_v4(), Uuid::new_v4());
    let (a, b) = (
        acteur_de(ea, &cast.direction),
        acteur_de(eb, &cast.direction),
    );
    ent.structure_profil_definir(&a, profil_saisie("A"))
        .await
        .unwrap();
    ent.structure_profil_definir(&b, profil_saisie("B"))
        .await
        .unwrap();
    ent.structure_parametre_definir(&a, "structure.trajet_defaut_min", json!(1), None)
        .await
        .unwrap();
    ent.structure_parametre_definir(&b, "structure.trajet_defaut_min", json!(2), None)
        .await
        .unwrap();
    for x in [&a, &b] {
        let site = nouveau_site(&ent, x, "PRINC").await;
        ent.structure_batiment_creer(x, site, "B", "B")
            .await
            .unwrap();
        ent.structure_lieu_creer(
            x,
            LieuSaisie {
                code: "L".into(),
                libelle: "L".into(),
                adresse: None,
                proprietaire: None,
                convention_document_id: None,
                trajet_minutes: None,
                mode_trajet: None,
            },
        )
        .await
        .unwrap();
    }
    for (table, attendu) in [
        ("ent_etablissement_profil", 1i64),
        ("ent_parametre", 1),
        ("ent_structure_journal", 4), // profil, paramètre, bâtiment, lieu (le site créé par `ref_creer_site` n'est pas journalisé ici)
        ("ent_batiment", 1),
        ("ent_lieu", 1),
    ] {
        let mut t = TenantTx::begin_mode(&app, ea, RlsMode::Strict)
            .await
            .unwrap();
        let n: i64 = sqlx::query(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *t)
            .await
            .unwrap()
            .get(0);
        let autres: i64 = sqlx::query(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id <> $1"
        ))
        .bind(ea)
        .fetch_one(&mut *t)
        .await
        .unwrap()
        .get(0);
        assert_eq!(
            autres, 0,
            "{table} : lignes d'un autre établissement visibles"
        );
        assert_eq!(n, attendu, "{table}");
        // Écrire dans l'autre établissement : refusé.
        if table == "ent_lieu" {
            let r = sqlx::query(
                "INSERT INTO ent_lieu (id, etablissement_id, code, libelle) VALUES ($1,$2,'Z','Z')",
            )
            .bind(Uuid::new_v4())
            .bind(eb)
            .execute(&mut *t)
            .await;
            let err = r.expect_err("insertion inter-établissement refusée");
            assert_eq!(
                err.as_database_error().and_then(|d| d.code()).as_deref(),
                Some("42501")
            );
        }
        drop(t);
    }
    // Sans TenantTx, le rôle strict ne voit rien (fail-closed).
    let n: i64 = sqlx::query("SELECT count(*) FROM ent_lieu")
        .fetch_one(&app)
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 0);
}
