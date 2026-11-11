//! Socle 39, lot ST1 (a) : dispositifs, paramètres, organismes (état, sites, contacts) et périodes
//! (`docs/socles/39_STAGES.md` §9, §10, §19.1 à §19.3). Contre la base locale de labo ; sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use chrono::NaiveDate;
use domain_eleve::{CreerEntreprise, StageErreur};
use ent_types::stages::*;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast, sqlx::PgPool)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    Some((ent, cast, pool))
}

fn est_refus<T: std::fmt::Debug>(r: &Result<T, EntDbError>, motif: StageErreur) -> bool {
    matches!(r, Err(EntDbError::Refus(_, m)) if *m == motif.message())
}

fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

/// Année scolaire lointaine et une classe, pour des périodes qui ne touchent pas l'année courante.
async fn annee_et_classe(pool: &sqlx::PgPool, etab: Uuid) -> (Uuid, Uuid) {
    let (annee, classe) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante) VALUES ($1,$2,'2098-2099',DATE '2098-09-01',DATE '2099-08-31',false)")
        .bind(annee)
        .bind(etab)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,'3A','3e A')")
        .bind(classe)
        .bind(etab)
        .bind(annee)
        .execute(pool)
        .await
        .unwrap();
    (annee, classe)
}

async fn organisme(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    ent.creer_entreprise_for_actor(
        "administratif",
        CreerEntreprise {
            etablissement_id: cast.etablissement_id,
            raison_sociale: "Ollivander".into(),
            siret: None,
            adresse: "Chemin de Traverse".into(),
            actor_user_id: cast.administratif.user_id,
        },
    )
    .await
    .expect("organisme")
    .id
}

#[tokio::test]
async fn st1a_dispositifs_actives_par_la_direction() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    // D-052 : ni PFMP ni apprentissage dans les dispositifs fournis.
    let liste = ent.dispositifs_stage_for_actor(etab, dir, "direction").await.unwrap();
    let codes: Vec<&str> = liste.iter().map(|x| x.code.as_str()).collect();
    assert_eq!(codes, ["observation_college", "seconde_gt", "immersion", "stage_individuel", "autre_encadre"]);
    assert!(liste.iter().all(|x| !x.actif && !x.allocation && !x.regime_at), "rien d'actif tant que la direction n'a pas choisi");
    assert!(liste[0].observation_seule && !liste[1].observation_seule);
    // Lecture : secrétariat oui, professeur / CPE non ; activation : direction seulement.
    assert!(ent.dispositifs_stage_for_actor(etab, cast.administratif.user_id, "administratif").await.is_ok());
    for (role, u) in [("prof", cast.prof.user_id), ("cpe", cast.cpe.user_id), ("compta", cast.compta.user_id)] {
        let r = ent.dispositifs_stage_for_actor(etab, u, role).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    let r = ent.activer_dispositif_stage_for_actor(etab, cast.administratif.user_id, "administratif", "observation_college", true).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent.activer_dispositif_stage_for_actor(etab, dir, "direction", "pfmp", true).await;
    assert!(est_refus(&r, StageErreur::DispositifInconnu), "{r:?}");
    ent.activer_dispositif_stage_for_actor(etab, dir, "direction", "observation_college", true).await.unwrap();
    let liste = ent.dispositifs_stage_for_actor(etab, dir, "direction").await.unwrap();
    assert!(liste[0].actif && liste.iter().filter(|x| x.actif).count() == 1);
    ent.activer_dispositif_stage_for_actor(etab, dir, "direction", "observation_college", false).await.unwrap();
    assert!(!ent.dispositifs_stage_for_actor(etab, dir, "direction").await.unwrap()[0].actif);
    // Autre établissement : rien d'activé chez lui.
    let autre = ent.seed_labo_cast().await.unwrap();
    assert!(ent.dispositifs_stage_for_actor(autre.etablissement_id, autre.direction.user_id, "direction").await.unwrap().iter().all(|x| !x.actif));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'stages.parametres_modifies'")
        .bind(etab)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 2);
}

/// DN-18 : l'assurance est un paramètre de l'établissement, jamais codée en dur ; verrou optimiste.
#[tokio::test]
async fn st1a_parametres_assurance_bornes_et_version() {
    let Some((ent, cast, _pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let p = ent.parametres_stage_for_actor(etab, dir, "direction").await.unwrap();
    assert_eq!((p.version, p.assurance_renseignee, p.assureur_nom.clone()), (0, false, None), "aucun assureur par défaut");
    assert_eq!((p.relance_ouverture_j, p.relance_sans_lieu_j, p.conservation_contacts_annees), (60, 30, 3));
    // Secrétariat : lecture oui, écriture non.
    assert!(ent.parametres_stage_for_actor(etab, cast.administratif.user_id, "administratif").await.is_ok());
    let r = ent
        .modifier_parametres_stage_for_actor(etab, cast.administratif.user_id, "administratif", ModifierParametresStageBody::default())
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let p = ent
        .modifier_parametres_stage_for_actor(
            etab,
            dir,
            "direction",
            ModifierParametresStageBody {
                assurance: Some(Some(AssuranceBody { assureur_nom: "MAE".into(), contrat: "C-42".into() })),
                deliberation: Some(Some(DeliberationBody { reference: "CA n° 12".into(), date: d(2026, 10, 14) })),
                relance_ouverture_j: Some(45),
                max_stagiaires_referent: Some(Some(8)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!((p.version, p.assurance_renseignee, p.relance_ouverture_j, p.max_stagiaires_referent), (1, true, 45, Some(8)));
    assert_eq!(p.deliberation_le, Some(d(2026, 10, 14)));
    // Version périmée : refusée.
    let r = ent
        .modifier_parametres_stage_for_actor(etab, dir, "direction", ModifierParametresStageBody { version: Some(0), relance_sans_lieu_j: Some(20), ..Default::default() })
        .await;
    assert!(est_refus(&r, StageErreur::ConflitDeVersion), "{r:?}");
    // Hors bornes (S-3901 : J-90 à J-15) ; assurance incomplète.
    let r = ent
        .modifier_parametres_stage_for_actor(etab, dir, "direction", ModifierParametresStageBody { version: Some(1), relance_ouverture_j: Some(120), ..Default::default() })
        .await;
    assert!(est_refus(&r, StageErreur::ParametreInvalide), "{r:?}");
    let r = ent
        .modifier_parametres_stage_for_actor(etab, dir, "direction", ModifierParametresStageBody {
            version: Some(1),
            assurance: Some(Some(AssuranceBody { assureur_nom: "MAE".into(), contrat: " ".into() })),
            ..Default::default()
        })
        .await;
    assert!(est_refus(&r, StageErreur::AssuranceIncomplete), "{r:?}");
    // Effacement explicite (null) du quota ; le reste est inchangé.
    let p = ent
        .modifier_parametres_stage_for_actor(etab, dir, "direction", ModifierParametresStageBody { version: Some(1), max_stagiaires_referent: Some(None), ..Default::default() })
        .await
        .unwrap();
    assert_eq!((p.version, p.max_stagiaires_referent, p.relance_ouverture_j, p.assurance_renseignee), (2, None, 45, true));
    // Cloisonnement : l'autre établissement garde les valeurs par défaut.
    let autre = ent.seed_labo_cast().await.unwrap();
    let q = ent.parametres_stage_for_actor(autre.etablissement_id, autre.direction.user_id, "direction").await.unwrap();
    assert_eq!((q.version, q.assurance_renseignee), (0, false));
}

#[tokio::test]
async fn st1a_organisme_etat_motif_code_et_interdit_reserve_a_la_direction() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (adm, dir) = (cast.administratif.user_id, cast.direction.user_id);
    let org = organisme(&ent, &cast).await;
    let o = ent
        .modifier_organisme_stage_for_actor(etab, adm, "administratif", org, ModifierOrganismeBody {
            version: 1,
            type_organisme: Some("association".into()),
            code_postal: Some(Some("75005".into())),
            commune: Some(Some("Londres".into())),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!((o.type_organisme.as_str(), o.version, o.commune.as_deref()), ("association", 2, Some("Londres")));
    let r = ent
        .modifier_organisme_stage_for_actor(etab, adm, "administratif", org, ModifierOrganismeBody { version: 1, commune: Some(None), ..Default::default() })
        .await;
    assert!(est_refus(&r, StageErreur::ConflitDeVersion), "{r:?}");
    let etat = |version, etat: &str, motif: Option<&str>, texte: Option<&str>| ChangerEtatOrganismeBody {
        version,
        etat: etat.into(),
        motif_code: motif.map(Into::into),
        motif_texte: texte.map(Into::into),
    };
    let r = ent.changer_etat_organisme_stage_for_actor(etab, adm, "administratif", org, etat(2, "deconseille", None, None)).await;
    assert!(est_refus(&r, StageErreur::MotifEtatRequis), "{r:?}");
    let o = ent
        .changer_etat_organisme_stage_for_actor(etab, adm, "administratif", org, etat(2, "deconseille", Some("securite"), Some("Machine sans protection")))
        .await
        .unwrap();
    assert_eq!((o.etat.as_str(), o.etat_motif_code.as_deref(), o.version), ("deconseille", Some("securite"), 3));
    // « Interdit » : le secrétariat ne le peut pas (ce n'est pas la direction) ; la direction oui.
    let r = ent.changer_etat_organisme_stage_for_actor(etab, adm, "administratif", org, etat(3, "interdit", Some("securite"), None)).await;
    assert!(est_refus(&r, StageErreur::InterditReserveDirection), "{r:?}");
    let o = ent.changer_etat_organisme_stage_for_actor(etab, dir, "direction", org, etat(3, "interdit", Some("securite"), None)).await.unwrap();
    assert_eq!(o.etat, "interdit");
    // Réactivé : le motif disparaît.
    let o = ent.changer_etat_organisme_stage_for_actor(etab, dir, "direction", org, etat(4, "actif", Some("securite"), Some("x"))).await.unwrap();
    assert_eq!((o.etat.as_str(), o.etat_motif_code, o.etat_motif_texte), ("actif", None, None));
    // Droits : professeur et CPE ne gèrent ni ne lisent le détail ; un autre établissement ne le voit pas.
    for (role, u) in [("prof", cast.prof.user_id), ("cpe", cast.cpe.user_id)] {
        let r = ent.changer_etat_organisme_stage_for_actor(etab, u, role, org, etat(5, "ferme", None, None)).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
        let r = ent.organisme_stage_for_actor(etab, u, role, org).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    let autre = ent.seed_labo_cast().await.unwrap();
    let r = ent.organisme_stage_for_actor(autre.etablissement_id, autre.administratif.user_id, "administratif", org).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    // L'événement d'état ne porte ni le texte libre ni le nom de l'organisme.
    let (payloads,): (String,) = sqlx::query_as("SELECT string_agg(payload::text, ' ') FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'stages.organisme_etat'")
        .bind(etab)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!payloads.contains("Machine") && !payloads.contains("Ollivander"), "{payloads}");
}

/// DN-09 : les contacts d'un organisme (personnes extérieures) ne sont lisibles qu'avec
/// `stages.organismes.gerer` ; l'événement ne porte aucune coordonnée.
#[tokio::test]
async fn st1a_sites_et_contacts_droits_version_et_minimisation() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let adm = cast.administratif.user_id;
    let org = organisme(&ent, &cast).await;
    let site = |version, commune: &str| SiteStageBody {
        version,
        libelle: "Boutique".into(),
        adresse_ligne1: "1 Chemin de Traverse".into(),
        adresse_ligne2: None,
        code_postal: "75005".into(),
        commune: commune.into(),
        pays: None,
    };
    let s = ent.creer_site_stage_for_actor(etab, adm, "administratif", org, site(None, "Paris")).await.unwrap();
    assert_eq!((s.pays.as_str(), s.version, s.actif), ("FR", 1, true));
    let r = ent.creer_site_stage_for_actor(etab, adm, "administratif", org, site(None, " ")).await;
    assert!(est_refus(&r, StageErreur::SiteInvalide), "{r:?}");
    let s2 = ent.modifier_site_stage_for_actor(etab, adm, "administratif", s.id, site(Some(1), "Lyon")).await.unwrap();
    assert_eq!((s2.commune.as_str(), s2.version), ("Lyon", 2));
    let r = ent.modifier_site_stage_for_actor(etab, adm, "administratif", s.id, site(Some(1), "Nice")).await;
    assert!(est_refus(&r, StageErreur::ConflitDeVersion), "{r:?}");
    assert_eq!(ent.sites_stage_for_actor(etab, adm, "administratif", org).await.unwrap().len(), 1);
    // Un site ne se crée pas sur un organisme d'un autre établissement ni inexistant.
    let r = ent.creer_site_stage_for_actor(etab, adm, "administratif", Uuid::new_v4(), site(None, "Paris")).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");

    let contact = |version, email: Option<&str>, signataire: bool| ContactStageBody {
        version,
        nom: "Weasley".into(),
        prenom: "Fred".into(),
        fonction: Some("Gérant".into()),
        email: email.map(Into::into),
        telephone: Some("0102030405".into()),
        est_tuteur: true,
        est_signataire: signataire,
        est_rh: false,
    };
    let r = ent.creer_contact_stage_for_actor(etab, adm, "administratif", org, contact(None, None, true)).await;
    assert!(est_refus(&r, StageErreur::SignataireSansCourriel), "{r:?}");
    let c = ent.creer_contact_stage_for_actor(etab, adm, "administratif", org, contact(None, Some("fred@wwws.example"), true)).await.unwrap();
    assert_eq!((c.version, c.actif, c.informe_le), (1, true, None));
    let c2 = ent.modifier_contact_stage_for_actor(etab, adm, "administratif", c.id, contact(Some(1), Some("fred2@wwws.example"), true)).await.unwrap();
    assert_eq!((c2.version, c2.email.as_deref()), (2, Some("fred2@wwws.example")));
    let r = ent.modifier_contact_stage_for_actor(etab, adm, "administratif", c.id, contact(Some(1), Some("x@y.example"), true)).await;
    assert!(est_refus(&r, StageErreur::ConflitDeVersion), "{r:?}");
    // DN-09 : professeur, CPE, comptabilité ne lisent pas les contacts.
    for (role, u) in [("prof", cast.prof.user_id), ("cpe", cast.cpe.user_id), ("compta", cast.compta.user_id)] {
        let r = ent.contacts_stage_for_actor(etab, u, role, org).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
        let r = ent.creer_contact_stage_for_actor(etab, u, role, org, contact(None, Some("a@b.example"), false)).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Autre établissement : rien (404) et aucune lecture croisée.
    let autre = ent.seed_labo_cast().await.unwrap();
    let r = ent.modifier_contact_stage_for_actor(autre.etablissement_id, autre.administratif.user_id, "administratif", c.id, contact(Some(2), Some("z@z.example"), false)).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    assert!(ent.contacts_stage_for_actor(autre.etablissement_id, autre.administratif.user_id, "administratif", org).await.unwrap().is_empty());
    // Désactivation : le contact disparaît de la liste, l'historique (ligne) reste.
    ent.desactiver_contact_stage_for_actor(etab, adm, "administratif", c.id, 2).await.unwrap();
    assert!(ent.contacts_stage_for_actor(etab, adm, "administratif", org).await.unwrap().is_empty());
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_entreprise_contact WHERE id = $1 AND NOT actif AND desactive_le IS NOT NULL")
        .bind(c.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
    // Événements sans nom ni coordonnée.
    let (payloads,): (String,) = sqlx::query_as("SELECT string_agg(payload::text, ' ') FROM ent_metier_event WHERE etablissement_id = $1 AND type IN ('stages.contact_modifie', 'stages.site_modifie')")
        .bind(etab)
        .fetch_one(&pool)
        .await
        .unwrap();
    for interdit in ["Weasley", "Fred", "wwws", "0102", "Chemin"] {
        assert!(!payloads.contains(interdit), "{interdit} dans {payloads}");
    }
    // Base : un signataire sans courriel est refusé aussi hors service.
    let r = sqlx::query("UPDATE ent_entreprise_contact SET est_signataire = true, email = NULL WHERE id = $1")
        .bind(c.id)
        .execute(&pool)
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn st1a_periodes_cycle_de_vie_et_controles() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, adm) = (cast.direction.user_id, cast.administratif.user_id);
    let (annee, classe) = annee_et_classe(&pool, etab).await;
    let nouvelle = |dispositif: &str, du, au, publics: Vec<PublicPeriodeDto>| CreerPeriodeStageBody {
        annee_id: annee,
        dispositif_code: dispositif.into(),
        libelle: "Stage de 3e".into(),
        du,
        au,
        individuelle: false,
        motif: None,
        publics,
    };
    let classe3a = || vec![PublicPeriodeDto { classe_id: Some(classe), ..Default::default() }];
    // Dispositif non activé : refusé ; secrétariat sans `stages.periodes.gerer` : refusé.
    let r = ent.creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2098, 12, 7), d(2098, 12, 11), classe3a())).await;
    assert!(est_refus(&r, StageErreur::DispositifInactif), "{r:?}");
    let r = ent.creer_periode_stage_for_actor(etab, adm, "administratif", nouvelle("observation_college", d(2098, 12, 7), d(2098, 12, 11), classe3a())).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    ent.activer_dispositif_stage_for_actor(etab, dir, "direction", "observation_college", true).await.unwrap();
    // Hors année, dates inversées, dispositif inconnu.
    let r = ent.creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2099, 8, 25), d(2099, 9, 3), classe3a())).await;
    assert!(est_refus(&r, StageErreur::PeriodeHorsAnnee), "{r:?}");
    let r = ent.creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2098, 12, 11), d(2098, 12, 7), classe3a())).await;
    assert!(est_refus(&r, StageErreur::PeriodeInvalide), "{r:?}");
    let r = ent.creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("pfmp", d(2098, 12, 7), d(2098, 12, 11), classe3a())).await;
    assert!(est_refus(&r, StageErreur::DispositifInconnu), "{r:?}");
    // Public d'un autre établissement : refusé (clé composée) ; public ambigu : refusé.
    let autre = ent.seed_labo_cast().await.unwrap();
    let (_, classe_autre) = annee_et_classe(&pool, autre.etablissement_id).await;
    let r = ent
        .creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2098, 12, 7), d(2098, 12, 11), vec![PublicPeriodeDto { classe_id: Some(classe_autre), ..Default::default() }]))
        .await;
    assert!(est_refus(&r, StageErreur::PeriodePublicInvalide), "{r:?}");
    let r = ent
        .creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2098, 12, 7), d(2098, 12, 11), vec![PublicPeriodeDto { classe_id: Some(classe), eleve_id: Some(cast.eleve_lea_id), ..Default::default() }]))
        .await;
    assert!(est_refus(&r, StageErreur::PeriodePublicInvalide), "{r:?}");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_stage_periode WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "un refus ne laisse aucune période (transaction)");
    // Création valide, puis publication.
    let p = ent.creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2098, 12, 7), d(2098, 12, 11), classe3a())).await.unwrap();
    assert_eq!((p.etat.as_str(), p.version, p.publics.len(), p.type_periode.as_str()), ("prevue", 1, 1, "collective"));
    // Période individuelle : motif obligatoire, élèves seulement (rattrapage).
    let mut indiv = nouvelle("observation_college", d(2099, 3, 1), d(2099, 3, 5), vec![PublicPeriodeDto { eleve_id: Some(cast.eleve_lea_id), ..Default::default() }]);
    indiv.individuelle = true;
    let r = ent.creer_periode_stage_for_actor(etab, dir, "direction", indiv.clone()).await;
    assert!(est_refus(&r, StageErreur::PeriodeMotifRequis), "{r:?}");
    indiv.motif = Some("Élève absent pour maladie".into());
    let i = ent.creer_periode_stage_for_actor(etab, dir, "direction", indiv.clone()).await.unwrap();
    assert_eq!(i.type_periode, "individuelle");
    indiv.publics = classe3a();
    let r = ent.creer_periode_stage_for_actor(etab, dir, "direction", indiv).await;
    assert!(est_refus(&r, StageErreur::PeriodePublicInvalide), "{r:?}");

    // Publier sans public : refusé ; avec public et bonne version : ok ; mauvaise version : conflit.
    let sans = ent.creer_periode_stage_for_actor(etab, dir, "direction", nouvelle("observation_college", d(2099, 1, 11), d(2099, 1, 15), vec![])).await.unwrap();
    let r = ent.publier_periode_stage_for_actor(etab, dir, "direction", sans.id, 1).await;
    assert!(est_refus(&r, StageErreur::PeriodeSansPublic), "{r:?}");
    let r = ent.publier_periode_stage_for_actor(etab, dir, "direction", p.id, 9).await;
    assert!(est_refus(&r, StageErreur::ConflitDeVersion), "{r:?}");
    let pub_ = ent.publier_periode_stage_for_actor(etab, dir, "direction", p.id, 1).await.unwrap();
    assert_eq!((pub_.etat.as_str(), pub_.version), ("publiee", 2));
    let r = ent.publier_periode_stage_for_actor(etab, dir, "direction", p.id, 2).await;
    assert!(est_refus(&r, StageErreur::EtatPeriodeInvalide), "{r:?}");
    // Modification d'une période publiée : dates déplacées, publics remplacés (jamais vides).
    let m = ent
        .modifier_periode_stage_for_actor(etab, dir, "direction", p.id, ModifierPeriodeStageBody {
            version: 2,
            libelle: "Stage de 3e (décalé)".into(),
            du: d(2098, 12, 14),
            au: d(2098, 12, 18),
            motif: None,
            publics: classe3a(),
        })
        .await
        .unwrap();
    assert_eq!((m.du, m.version, m.etat.as_str()), (d(2098, 12, 14), 3, "publiee"));
    let r = ent
        .modifier_periode_stage_for_actor(etab, dir, "direction", p.id, ModifierPeriodeStageBody { version: 3, libelle: "x".into(), du: d(2098, 12, 14), au: d(2098, 12, 18), motif: None, publics: vec![] })
        .await;
    assert!(est_refus(&r, StageErreur::PeriodeSansPublic), "{r:?}");
    // Liste filtrée, cloisonnée ; lecture secrétariat oui, professeur non.
    let liste = ent.periodes_stage_for_actor(etab, adm, "administratif", &FiltrePeriodesStage { annee_id: Some(annee), etat: Some("publiee".into()) }).await.unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(ent.periodes_stage_for_actor(etab, dir, "direction", &FiltrePeriodesStage::default()).await.unwrap().len(), 3);
    assert!(ent.periodes_stage_for_actor(autre.etablissement_id, autre.direction.user_id, "direction", &FiltrePeriodesStage::default()).await.unwrap().is_empty());
    let r = ent.periodes_stage_for_actor(etab, cast.prof.user_id, "prof", &FiltrePeriodesStage::default()).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Fermeture, puis plus aucune modification.
    let close = ent.fermer_periode_stage_for_actor(etab, dir, "direction", p.id, 3).await.unwrap();
    assert_eq!(close.etat, "close");
    let r = ent.fermer_periode_stage_for_actor(etab, dir, "direction", p.id, 4).await;
    assert!(est_refus(&r, StageErreur::EtatPeriodeInvalide), "{r:?}");
    let r = ent
        .modifier_periode_stage_for_actor(etab, dir, "direction", p.id, ModifierPeriodeStageBody { version: 4, libelle: "x".into(), du: d(2098, 12, 14), au: d(2098, 12, 18), motif: None, publics: classe3a() })
        .await;
    assert!(est_refus(&r, StageErreur::PeriodeDansLeTemps), "{r:?}");
    // Un autre établissement ne touche pas la période (404).
    let r = ent.fermer_periode_stage_for_actor(autre.etablissement_id, autre.direction.user_id, "direction", i.id, 1).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    // Événements : identifiants, dispositif et dates seulement.
    let (payloads,): (String,) = sqlx::query_as("SELECT string_agg(payload::text, ' ') FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'stages.periode_%'")
        .bind(etab)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!payloads.contains("maladie") && !payloads.contains("décalé"), "{payloads}");
    // La base refuse aussi l'incohérence (période individuelle sans motif).
    let r = sqlx::query("UPDATE ent_stage_periode SET motif = NULL WHERE id = $1").bind(i.id).execute(&pool).await;
    assert!(r.is_err());
}
