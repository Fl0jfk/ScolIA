//! Socle 2 (l'élève), lot E8 : catalogue d'indicateurs, consommateur D0 de l'outbox, synthèse de
//! la fiche et contrat des tuiles. Scénarios contre la base locale de test (feature `labo`).
//!
//! Aucune dépendance à l'heure réelle : les pastilles qui dépendent de la date sont recalculées à
//! un jour **explicite** (`reconstruire_indicateurs(.., Some(jour))`).

use super::*;
use crate::eleve_indicateurs::{EtatTuile, CONSOMMATEUR_INDICATEURS};
use crate::identifiants::AjoutIdentifiant;
use crate::outbox::ConfigDistribution;
use crate::scolarite::ChangementRegimeSite;
use crate::seed::LaboCast;
use chrono::{Duration, NaiveDate};
use domain_eleve::{
    AttributKind, CreateEleve, ProvenanceIdentifiant, RaisonTuile, SystemeIdentifiant, UpsertPai,
    CATALOGUE_INDICATEURS, CONTRATS_TUILES,
};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_e| {}).await;
    Some((ent, cast))
}

fn lettres(n: usize) -> String {
    Uuid::new_v4().as_bytes().iter().take(n).map(|b| (b'a' + b % 26) as char).collect()
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str, naissance: NaiveDate) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Pastille{}", lettres(6)),
        prenom: prenom.into(),
        ine: None,
        sexe: Some("M".into()),
        date_naissance: Some(naissance),
        numero_badge: None,
        redoublant: false,
        classe: Some("6A".into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

/// Distribue tout ce que l'outbox de l'établissement contient (le consommateur D0 y répond).
async fn distribuer(ent: &PgEnt, etab: Uuid) {
    let d = ent.distributeur(ConfigDistribution { etablissement: Some(etab), ..Default::default() });
    d.distribuer_tout().await.unwrap();
}

/// Codes des pastilles de la projection (lecture directe, sans droit : la vérité de la base).
async fn projection(ent: &PgEnt, eleve: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT code FROM ent_eleve_indicateur WHERE eleve_id = $1 ORDER BY code")
        .bind(eleve)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap()
}

fn user(cast: &LaboCast, r: Role) -> (Uuid, &'static str) {
    let p = match r {
        Role::Direction => &cast.direction,
        Role::Administratif => &cast.administratif,
        Role::Cpe => &cast.cpe,
        Role::Prof => &cast.prof,
        Role::Compta => &cast.compta,
        _ => unreachable!(),
    };
    (p.user_id, r.as_str())
}

fn naissance_mineur() -> NaiveDate {
    NaiveDate::from_ymd_opt(2012, 6, 1).unwrap()
}

/// La table du catalogue est égale à la constante du logiciel (la migration la sème, le code la lit).
#[tokio::test]
async fn le_catalogue_sql_est_egal_au_catalogue_du_logiciel() {
    let Some((ent, _cast)) = labo().await else { return };
    let lignes: Vec<(String, String, String, String, String, Vec<String>, bool)> = sqlx::query_as(
        "SELECT code, domaine, libelle, icone, gravite, actions_requises, depend_du_temps
           FROM ent_catalogue_indicateur ORDER BY code",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let mut attendu: Vec<_> = CATALOGUE_INDICATEURS
        .iter()
        .map(|d| {
            (
                d.code.to_string(),
                d.domaine.to_string(),
                d.libelle.to_string(),
                d.icone.to_string(),
                d.gravite.code().to_string(),
                d.actions_requises.iter().map(|a| (*a).to_string()).collect::<Vec<_>>(),
                d.depend_du_temps,
            )
        })
        .collect();
    attendu.sort();
    assert_eq!(lignes, attendu);
    // Chaque action désignée existe au catalogue des droits : une faute de frappe serait une pastille publique.
    for d in CATALOGUE_INDICATEURS {
        for a in d.actions_requises {
            assert!(ent_auth::Action::parse(a).is_some(), "{} : action {a} inconnue", d.code);
        }
    }
    for c in CONTRATS_TUILES {
        for a in c.actions_requises {
            assert!(ent_auth::Action::parse(a).is_some(), "tuile {} : action {a} inconnue", c.domaine);
        }
    }
    // Le consommateur est enregistré sous son nom stable.
    let d = ent.distributeur(ConfigDistribution::default());
    assert!(d.noms_consommateurs().contains(&CONSOMMATEUR_INDICATEURS));
}

/// Les trois pastilles de référence, alimentées PAR ÉVÉNEMENTS : sans INE (création, identifiant),
/// interne (régime), PAI (infirmerie) — posées puis retirées.
#[tokio::test]
async fn interne_pai_et_sans_ine_suivent_les_evenements() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let admin = cast.administratif.user_id;
    let infirmiere = cast.infirmieres[0].clone();
    let e = eleve(&ent, etab, "Paul", naissance_mineur()).await;
    distribuer(&ent, etab).await;
    // Création : sans INE (événement `eleve.created`).
    assert_eq!(projection(&ent, e.id).await, ["sans_ine"]);

    // INE ajouté : la pastille disparaît (événement `eleve.identifiant_change`).
    let ine = format!("1234{}", Uuid::new_v4().simple().to_string()[..7].replace(|c: char| !c.is_ascii_digit(), "5"));
    ent.ajouter_identifiant(
        Role::Administratif.as_str(),
        AjoutIdentifiant {
            etablissement_id: etab,
            eleve_id: e.id,
            systeme: SystemeIdentifiant::Ine,
            valeur: ine,
            provenance: ProvenanceIdentifiant::Saisie,
            valide_du: None,
            preuve_document_id: None,
            acteur: Some(admin),
        },
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    assert!(projection(&ent, e.id).await.is_empty(), "INE saisi : plus de pastille");

    // Régime interne (événement `eleve.regime_changed`) : « interne », valeur = le régime.
    let jour = ent.jour_etablissement(etab).await.unwrap();
    let chg = |valeur: &str| ChangementRegimeSite {
        etablissement_id: etab,
        eleve_id: e.id,
        kind: AttributKind::Regime,
        valeur: valeur.into(),
        date_effet: jour - Duration::days(3),
        derogation: None,
        acteur: admin,
    };
    ent.changer_regime_ou_site(Role::Administratif.as_str(), chg("interne")).await.unwrap();
    distribuer(&ent, etab).await;
    assert_eq!(projection(&ent, e.id).await, ["interne"]);
    let (v,): (Option<String>,) = sqlx::query_as("SELECT valeur_code FROM ent_eleve_indicateur WHERE eleve_id = $1 AND code = 'interne'")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(v.as_deref(), Some("interne"));

    // PAI (événement `infirmerie.pai`) : « PAI en cours » ; PAI levé : la pastille disparaît.
    let pai = |actif: bool| UpsertPai {
        etablissement_id: etab,
        eleve_id: e.id,
        motif: "allergie".into(),
        mesures_accueil: "trousse".into(),
        protocole_urgence: "appeler le 15".into(),
        note_interne: String::new(),
        date_debut: jour - Duration::days(10),
        date_fin: None,
        actif,
        actor_user_id: infirmiere.user_id,
    };
    ent.enregistrer_pai_for_actor(Role::Infirmerie.as_str(), pai(true), None).await.unwrap();
    distribuer(&ent, etab).await;
    assert_eq!(projection(&ent, e.id).await, ["interne", "pai_actif"]);
    ent.enregistrer_pai_for_actor(Role::Infirmerie.as_str(), pai(false), None).await.unwrap();
    distribuer(&ent, etab).await;
    assert_eq!(projection(&ent, e.id).await, ["interne"]);

    // Retour au régime demi-pensionnaire : plus d'interne.
    ent.changer_regime_ou_site(Role::Administratif.as_str(), ChangementRegimeSite { date_effet: jour - Duration::days(1), ..chg("demi-pensionnaire") })
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    assert!(projection(&ent, e.id).await.is_empty());

    // Rejeu du consommateur : même état (idempotent, aucune ligne en double).
    let d = ent.distributeur(ConfigDistribution { etablissement: Some(etab), ..Default::default() });
    d.rejouer_consommateur(CONSOMMATEUR_INDICATEURS, etab, 0).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert!(projection(&ent, e.id).await.is_empty());
    // La reconstruction depuis les tables sources donne la même chose (la projection est un cache).
    ent.reconstruire_indicateurs(etab, None, None).await.unwrap();
    assert!(projection(&ent, e.id).await.is_empty());
}

/// Filtrage par les droits : la même fiche, des pastilles différentes selon l'acteur ; une tuile
/// masquée dit pourquoi et ne laisse rien filtrer ; un élève protégé est absent pour un tiers.
#[tokio::test]
async fn les_pastilles_et_les_tuiles_suivent_les_droits_de_l_acteur() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    let (prof, rprof) = user(&cast, Role::Prof);
    let infirmiere = cast.infirmieres[0].clone();
    let e = eleve(&ent, etab, "Tom", naissance_mineur()).await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    ent.changer_regime_ou_site(
        radm,
        ChangementRegimeSite {
            etablissement_id: etab,
            eleve_id: e.id,
            kind: AttributKind::Regime,
            valeur: "interne".into(),
            date_effet: jour - Duration::days(3),
            derogation: None,
            acteur: adm,
        },
    )
    .await
    .unwrap();
    ent.enregistrer_pai_for_actor(
        Role::Infirmerie.as_str(),
        UpsertPai {
            etablissement_id: etab,
            eleve_id: e.id,
            motif: "asthme".into(),
            mesures_accueil: "trousse".into(),
            protocole_urgence: "appeler le 15".into(),
            note_interne: String::new(),
            date_debut: jour - Duration::days(5),
            date_fin: None,
            actif: true,
            actor_user_id: infirmiere.user_id,
        },
        None,
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let codes = |s: &crate::eleve_indicateurs::SyntheseEleve| s.pastilles.iter().map(|p| p.code).collect::<Vec<_>>();
    let tuile = |s: &crate::eleve_indicateurs::SyntheseEleve, d: &str| s.tuiles.iter().find(|t| t.domaine == d).cloned().unwrap();

    // Direction : sans INE, interne, mais pas le PAI (santé : `sante.lire`, ni G6 sans mesure).
    let s = ent.synthese_eleve_pour_acteur(rdir, etab, dir, e.id).await.unwrap();
    assert!(codes(&s).contains(&"sans_ine") && codes(&s).contains(&"interne"), "{:?}", codes(&s));
    // Alerte d'abord : le tri place les gravités fortes en tête.
    assert!(s.pastilles.windows(2).all(|w| w[0].gravite <= w[1].gravite));
    // Infirmerie : le PAI, pas le « sans INE » (G3).
    let s = ent.synthese_eleve_pour_acteur(Role::Infirmerie.as_str(), etab, infirmiere.user_id, e.id).await.unwrap();
    assert!(codes(&s).contains(&"pai_actif") && !codes(&s).contains(&"sans_ine"), "{:?}", codes(&s));
    assert_eq!(tuile(&s, "sante").etat, EtatTuile::Visible);
    assert_eq!(tuile(&s, "sante").pastilles.len(), 1);
    // La lecture d'une pastille de santé est journalisée.
    let c = ent.consultations_pour_acteur(rdir, etab, dir, e.id, 50).await.unwrap();
    assert!(c.iter().any(|x| x.acteur == Some(infirmiere.user_id) && x.action == "pastilles_sensibles"), "{c:?}");

    // Secrétariat : pas de PAI ; la tuile santé est masquée AVEC sa raison, sans contenu.
    let s = ent.synthese_eleve_pour_acteur(radm, etab, adm, e.id).await.unwrap();
    assert!(!codes(&s).contains(&"pai_actif"));
    let t = tuile(&s, "sante");
    assert!(matches!(t.etat, EtatTuile::Masquee(RaisonTuile::DroitManquant | RaisonTuile::ReserveAuPersonnel | RaisonTuile::HorsPortee)), "{:?}", t.etat);
    assert!(t.pastilles.is_empty());
    assert!(t.lien.contains(&e.id.to_string()));
    // Les tuiles sont rendues dans l'ordre du contrat, une par domaine.
    assert_eq!(s.tuiles.iter().map(|t| t.domaine).collect::<Vec<_>>(), {
        let mut c: Vec<_> = CONTRATS_TUILES.iter().collect();
        c.sort_by_key(|c| c.rang);
        c.iter().map(|c| c.domaine).collect::<Vec<_>>()
    });

    // Un professeur qui ne suit pas cet élève n'obtient rien (portée de l'annuaire) ; la famille non plus.
    let r = ent.synthese_eleve_pour_acteur(rprof, etab, prof, e.id).await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)) | Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent.synthese_eleve_pour_acteur(Role::Parent.as_str(), etab, cast.parent_lea.user_id, cast.eleve_lea_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Élève absent ou d'un autre établissement : 404.
    let r = ent.synthese_eleve_pour_acteur(rdir, etab, dir, Uuid::new_v4()).await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");
    let ailleurs = eleve(&ent, Uuid::new_v4(), "Ailleurs", naissance_mineur()).await;
    let r = ent.synthese_eleve_pour_acteur(rdir, etab, dir, ailleurs.id).await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");

    // Confidentialité renforcée : la pastille « confidentialité » (G6) pour la direction ; l'élève
    // disparaît pour le secrétariat (404), même avec ses droits d'annuaire.
    ent.poser_confidentialite(
        rdir,
        crate::eleve_confidentialite::PoserConfidentialite {
            etablissement_id: etab,
            eleve_id: e.id,
            motif: "Protection".into(),
            mesures: vec![],
            du: None,
            au: None,
            acteur: dir,
        },
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let s = ent.synthese_eleve_pour_acteur(rdir, etab, dir, e.id).await.unwrap();
    assert!(codes(&s).contains(&"confidentialite_renforcee"));
    assert_eq!(tuile(&s, "protection").etat, EtatTuile::Visible);
    let r = ent.synthese_eleve_pour_acteur(radm, etab, adm, e.id).await;
    assert!(matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{r:?}");
}

/// Listes de travail et compteurs : « sans INE » pour le secrétariat ; refus sans le droit ;
/// l'élève protégé n'est ni dans la liste ni dans le compte d'un tiers.
#[tokio::test]
async fn listes_et_compteurs_par_pastille() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    let (cpe, rcpe) = user(&cast, Role::Cpe);
    let a = eleve(&ent, etab, "Ana", naissance_mineur()).await;
    let b = eleve(&ent, etab, "Bob", naissance_mineur()).await;
    distribuer(&ent, etab).await;
    let l = ent.eleves_par_pastille_pour_acteur(radm, etab, adm, "sans_ine", 5000).await.unwrap();
    assert!(l.contains(&a.id) && l.contains(&b.id));
    // La CPE n'a pas G3 : refus. Pastille inconnue : saisie invalide.
    assert!(matches!(ent.eleves_par_pastille_pour_acteur(rcpe, etab, cpe, "sans_ine", 10).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.eleves_par_pastille_pour_acteur(radm, etab, adm, "inventee", 10).await, Err(EntDbError::SaisieInvalide(_))));
    let avant = ent.compteurs_pastilles_pour_acteur(radm, etab, adm).await.unwrap();
    let n_avant = avant.iter().find(|(c, _)| *c == "sans_ine").unwrap().1;
    assert!(n_avant >= 2);
    // Protégé : disparaît de la liste et du compte du secrétariat, reste pour la direction.
    ent.poser_confidentialite(
        rdir,
        crate::eleve_confidentialite::PoserConfidentialite {
            etablissement_id: etab,
            eleve_id: a.id,
            motif: "Protection".into(),
            mesures: vec![],
            du: None,
            au: None,
            acteur: dir,
        },
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let l = ent.eleves_par_pastille_pour_acteur(radm, etab, adm, "sans_ine", 5000).await.unwrap();
    assert!(!l.contains(&a.id) && l.contains(&b.id));
    let apres = ent.compteurs_pastilles_pour_acteur(radm, etab, adm).await.unwrap();
    assert_eq!(apres.iter().find(|(c, _)| *c == "sans_ine").unwrap().1, n_avant - 1);
    let l = ent.eleves_par_pastille_pour_acteur(rdir, etab, dir, "sans_ine", 5000).await.unwrap();
    assert!(l.contains(&a.id));
}

/// Pastilles qui dépendent de la date : recalculées à un jour EXPLICITE (majorité, échéance d'un PAI).
#[tokio::test]
async fn pastilles_dependant_du_temps_a_un_jour_explicite() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let infirmiere = cast.infirmieres[0].clone();
    // Né le 29 février 2008 : majeur le 28 février 2026 (règle du calendrier).
    let e = eleve(&ent, etab, "Léap", NaiveDate::from_ymd_opt(2008, 2, 29).unwrap()).await;
    let jour = |a, m, j| NaiveDate::from_ymd_opt(a, m, j).unwrap();
    ent.enregistrer_pai_for_actor(
        Role::Infirmerie.as_str(),
        UpsertPai {
            etablissement_id: etab,
            eleve_id: e.id,
            motif: "allergie".into(),
            mesures_accueil: "trousse".into(),
            protocole_urgence: "appeler le 15".into(),
            note_interne: String::new(),
            date_debut: jour(2025, 9, 1),
            date_fin: Some(jour(2026, 6, 30)),
            actif: true,
            actor_user_id: infirmiere.user_id,
        },
        None,
    )
    .await
    .unwrap();
    ent.reconstruire_indicateurs(etab, None, Some(jour(2026, 2, 27))).await.unwrap();
    assert_eq!(projection(&ent, e.id).await, ["pai_actif", "sans_ine"], "veille de la majorité");
    ent.reconstruire_indicateurs(etab, None, Some(jour(2026, 2, 28))).await.unwrap();
    assert_eq!(projection(&ent, e.id).await, ["majeur", "pai_actif", "sans_ine"], "majeur le 28 février 2026");
    ent.reconstruire_indicateurs(etab, None, Some(jour(2026, 7, 1))).await.unwrap();
    assert_eq!(projection(&ent, e.id).await, ["majeur", "sans_ine"], "PAI échu le 30 juin");
    // Idempotence : un second passage ne change rien.
    assert_eq!(ent.reconstruire_indicateurs(etab, None, Some(jour(2026, 7, 1))).await.unwrap(), 0);
    // La tâche quotidienne s'exécute sans erreur (jour réel) et recalcule les pastilles datées.
    ent.recalcul_nocturne_indicateurs().await.unwrap();
}

/// Un élève fusionné (absorbé) n'a plus de pastille ; la fiche conservée est recalculée.
#[tokio::test]
async fn un_eleve_sorti_ou_fusionne_n_a_plus_de_pastille() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Sam", naissance_mineur()).await;
    distribuer(&ent, etab).await;
    assert_eq!(projection(&ent, e.id).await, ["sans_ine"]);
    // Statut « sorti » (sortie datée) : les pastilles de suivi cessent (plus à relancer).
    sqlx::query("UPDATE ent_eleve SET statut = 'sorti' WHERE id = $1")
        .bind(e.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    ent.reconstruire_indicateurs(etab, None, None).await.unwrap();
    assert!(projection(&ent, e.id).await.is_empty());
    // Absorbé par une fusion : plus de pastille non plus.
    sqlx::query("UPDATE ent_eleve SET statut = 'scolarise' WHERE id = $1").bind(e.id).execute(ent.pool_verif()).await.unwrap();
    ent.reconstruire_indicateurs(etab, None, None).await.unwrap();
    assert_eq!(projection(&ent, e.id).await, ["sans_ine"]);
    let autre = eleve(&ent, etab, "Tim", naissance_mineur()).await;
    sqlx::query("UPDATE ent_eleve SET fusionne_dans = $2 WHERE id = $1").bind(e.id).bind(autre.id).execute(ent.pool_verif()).await.unwrap();
    ent.reconstruire_indicateurs(etab, None, None).await.unwrap();
    assert!(projection(&ent, e.id).await.is_empty());
}
