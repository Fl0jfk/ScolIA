//! Socle 2, lot S2-11 : santé et accompagnement contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Les textes « sentinelles » (`SENT_*`) servent à prouver la minimisation : ils sont
//! saisis dans les champs de santé puis recherchés dans les événements, la file
//! d'envoi, les notifications, le journal et les listes non santé.

use super::*;
use crate::accompagnement::{DefinirAmenagement, OuvrirDispositif};
use crate::outbox::ConfigDistribution;
use crate::sante_acc::{
    Audience, ClorePassage, EnregistrerProtocole, ExtrasPai, NouveauPassage, PublierContrainte,
};
use chrono::Duration;
use domain_eleve::{today_utc, CreerPassageInfirmerie, DomainError, PassageInfirmerieMotif, UpsertFicheSante, UpsertPai};
use ent_auth::Role;
use uuid::Uuid;

const SENT_MOTIF: &str = "SENT-motif-anaphylaxie-noix";
const SENT_MESURES: &str = "SENT-mesures-epipen-poche";
const SENT_PROTOCOLE: &str = "SENT-protocole-injecter-cuisse";
const SENT_NOTE: &str = "SENT-note-interne-medecin";
const SENT_ALLERGIE: &str = "SENT-allergie-arachide";
const SENT_TRAITEMENT: &str = "SENT-traitement-ventoline";
const SENT_SOINS: &str = "SENT-soins-glacage-cheville";
const SENT_CLINIQUE: &str = "SENT-clinique-entorse-grade2";
const SENT_PSY: &str = "SENT-psy-axes-anxiete";

const TOUS_SENTINELLES: [&str; 9] = [
    SENT_MOTIF,
    SENT_MESURES,
    SENT_PROTOCOLE,
    SENT_NOTE,
    SENT_ALLERGIE,
    SENT_TRAITEMENT,
    SENT_SOINS,
    SENT_CLINIQUE,
    SENT_PSY,
];

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn inf(cast: &LaboCast) -> (Uuid, &'static str) {
    (cast.infirmieres[0].user_id, Role::Infirmerie.as_str())
}

fn dir(cast: &LaboCast) -> (Uuid, &'static str) {
    (cast.direction.user_id, Role::Direction.as_str())
}

fn prof(cast: &LaboCast) -> (Uuid, &'static str) {
    (cast.prof.user_id, Role::Prof.as_str())
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    })
    .distribuer_tout()
    .await
    .unwrap();
}

fn pai_cmd(cast: &LaboCast, eleve: Uuid, actif: bool) -> UpsertPai {
    UpsertPai {
        etablissement_id: cast.etablissement_id,
        eleve_id: eleve,
        motif: SENT_MOTIF.into(),
        mesures_accueil: SENT_MESURES.into(),
        protocole_urgence: SENT_PROTOCOLE.into(),
        note_interne: SENT_NOTE.into(),
        date_debut: today_utc(),
        date_fin: None,
        actif,
        actor_user_id: cast.infirmieres[0].user_id,
    }
}

fn extras_trousse() -> ExtrasPai {
    ExtrasPai {
        date_revision: Some(today_utc() + Duration::days(10)),
        trousse_urgence: true,
        trousse_lieu: "Infirmerie, armoire rouge".into(),
        trousse_peremption: Some(today_utc() + Duration::days(200)),
    }
}

async fn brut(ent: &PgEnt, sql: &str, id: Uuid) -> String {
    let (v,): (String,) = sqlx::query_as(sql).bind(id).fetch_one(ent.pool()).await.unwrap();
    v
}

/// Texte de tout ce qui n'est pas la table santé : événements, file d'envoi,
/// notifications et journal de l'établissement.
async fn texte_non_sante(ent: &PgEnt, etab: Uuid) -> String {
    let mut out = String::new();
    for sql in [
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1",
        "SELECT coalesce(titre,'') || ' ' || coalesce(corps,'') FROM ent_notification WHERE etablissement_id = $1",
        "SELECT coalesce(libelle,'') || ' ' || payload::text FROM ent_journal_modif WHERE etablissement_id = $1",
    ] {
        let lignes: Vec<(String,)> = sqlx::query_as(sql).bind(etab).fetch_all(ent.pool()).await.unwrap();
        for (l,) in lignes {
            out.push_str(&l);
            out.push('\n');
        }
    }
    out
}

fn sans_sentinelle(texte: &str, ou: &str) {
    for s in TOUS_SENTINELLES {
        assert!(!texte.contains(s), "donnée de santé « {s} » trouvée dans {ou}");
    }
}

// ─── 1. PAI complet : version, signatures, révision, trousse ─────────────────

#[tokio::test]
async fn pai_complet_signatures_par_version_et_revision() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (i, ri) = inf(&cast);
    let (d, rd) = dir(&cast);
    let parent = (cast.parent_lea.user_id, "parent");

    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), Some(extras_trousse()))
        .await
        .unwrap();
    let p = ent.pai_complet_for_actor(etab, i, ri, lea).await.unwrap().unwrap();
    // Le jeu de labo peut déjà porter un PAI pour cet élève : la version se compare au départ.
    let v0 = p.version;
    assert!(v0 >= 1);
    assert!(p.trousse_urgence && !p.trousse_perimee);
    assert!(p.revision_due, "révision dans 10 jours : à préparer");
    assert!(!p.complet && p.parties_manquantes.len() == 4);
    assert_eq!(p.pai.motif, SENT_MOTIF, "relu en clair après chiffrement");

    // Trousse sans lieu : refusé.
    let mut mauvais = extras_trousse();
    mauvais.trousse_lieu = "  ".into();
    assert!(ent
        .enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), Some(mauvais))
        .await
        .is_err());

    // Signatures : médecin et infirmier consignés par l'infirmerie, chef par la
    // direction, famille par le responsable lié.
    ent.signer_pai_for_actor(etab, i, ri, lea, "medecin", Some("Dr Chourave")).await.unwrap();
    ent.signer_pai_for_actor(etab, i, ri, lea, "infirmier", Some("Mme Pomfresh")).await.unwrap();
    // L'infirmière ne signe pas pour le chef d'établissement.
    assert!(matches!(
        ent.signer_pai_for_actor(etab, i, ri, lea, "chef_etablissement", Some("Dumbledore")).await,
        Err(EntDbError::Forbidden)
    ));
    ent.signer_pai_for_actor(etab, d, rd, lea, "chef_etablissement", Some("A. Dumbledore"))
        .await
        .unwrap();
    // La direction ne signe pas « infirmier » à la place de l'infirmière.
    assert!(matches!(
        ent.signer_pai_for_actor(etab, d, rd, lea, "infirmier", Some("Faux nom")).await,
        Err(EntDbError::Forbidden)
    ));
    // Signature d'une version périmée : conflit, rien d'enregistré.
    assert!(matches!(
        ent.signer_pai_version_for_actor(etab, parent.0, parent.1, lea, "famille", None, Some(v0 + 99)).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Un parent sans lien avec Léa ne signe pas.
    assert!(matches!(
        ent.signer_pai_for_actor(etab, cast.parent_tom.user_id, "parent", lea, "famille", None).await,
        Err(EntDbError::Forbidden)
    ));
    // Un parent ne signe que pour la famille.
    assert!(matches!(
        ent.signer_pai_for_actor(etab, parent.0, parent.1, lea, "medecin", Some("X")).await,
        Err(EntDbError::Forbidden)
    ));
    let avant = ent.pai_complet_for_actor(etab, i, ri, lea).await.unwrap().unwrap();
    assert!(!avant.complet && avant.parties_manquantes == vec!["famille".to_string()]);
    let signe = ent
        .signer_pai_version_for_actor(etab, parent.0, parent.1, lea, "famille", None, Some(v0))
        .await
        .unwrap();
    assert!(signe.complet && signe.signatures.len() == 4);
    // Signature en double : refusée.
    assert!(matches!(
        ent.signer_pai_for_actor(etab, parent.0, parent.1, lea, "famille", None).await,
        Err(EntDbError::Domain(DomainError::PaiDejaSigne))
    ));
    // La famille voit le PAI complet de son enfant mais jamais la note interne.
    let vue = ent.pai_complet_for_actor(etab, parent.0, parent.1, lea).await.unwrap().unwrap();
    assert!(vue.pai.note_interne.is_empty() && vue.complet);

    // Toute modification invalide les signatures (nouvelle version).
    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), None).await.unwrap();
    let apres = ent.pai_complet_for_actor(etab, i, ri, lea).await.unwrap().unwrap();
    assert_eq!(apres.version, v0 + 1);
    assert!(apres.signatures.is_empty() && !apres.complet);
    assert!(apres.trousse_urgence, "les compléments sont conservés quand ils ne sont pas fournis");

    // À réviser : liste de l'infirmerie (échéance dans 10 jours).
    let liste = ent.pai_a_reviser_for_actor(etab, i, ri, 30).await.unwrap();
    assert!(liste.iter().any(|p| p.eleve_id == lea && p.jours_restants <= 10));
    assert!(ent.pai_a_reviser_for_actor(etab, i, ri, 5).await.unwrap().iter().all(|p| p.eleve_id != lea));

    // Droits : ni le professeur ni la famille n'écrivent un PAI.
    assert!(matches!(
        ent.enregistrer_pai_for_actor("prof", pai_cmd(&cast, lea, true), None).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.pai_a_reviser_for_actor(etab, cast.prof.user_id, "prof", 30).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.pai_complet_for_actor(etab, cast.prof.user_id, "prof", lea).await.is_err());
}

// ─── 2. Chiffrement conservé ─────────────────────────────────────────────────

#[tokio::test]
async fn textes_cliniques_chiffres_en_base() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, hermione) = (cast.etablissement_id, cast.eleves[2].id);
    let (i, ri) = inf(&cast);

    let pai = ent
        .enregistrer_pai_for_actor(ri, pai_cmd(&cast, hermione, true), None)
        .await
        .unwrap();
    for col in ["motif", "mesures_accueil", "protocole_urgence", "note_interne"] {
        let v = brut(&ent, &format!("SELECT {col} FROM ent_pai WHERE id = $1"), pai.id).await;
        assert!(v.starts_with("enc:v1:"), "ent_pai.{col} doit être chiffré");
        assert!(!v.contains("SENT-"));
    }
    // Un chiffré recopié dans une autre ligne ne s'ouvre pas (AAD = id de ligne).
    let autre = ent
        .enregistrer_pai_for_actor(ri, pai_cmd(&cast, cast.eleves[3].id, true), None)
        .await
        .unwrap();
    let chiffre = brut(&ent, "SELECT motif FROM ent_pai WHERE id = $1", pai.id).await;
    sqlx::query("UPDATE ent_pai SET motif = $2 WHERE id = $1")
        .bind(autre.id)
        .bind(&chiffre)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(ent.pai_complet_for_actor(etab, i, ri, cast.eleves[3].id).await.is_err());

    let p = ent
        .enregistrer_passage_for_actor(
            i,
            ri,
            NouveauPassage {
                etablissement_id: etab,
                eleve_id: hermione,
                jour: today_utc(),
                heure_arrivee: None,
                motif: "soins".into(),
                note_soins: SENT_CLINIQUE.into(),
                soins: SENT_SOINS.into(),
                visible_famille: true,
                issue: "retour_classe".into(),
                informer_famille: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(p.note_soins, SENT_CLINIQUE);
    for col in ["note_soins", "soins"] {
        let v = brut(&ent, &format!("SELECT {col} FROM ent_infirmerie_passage WHERE id = $1"), p.id).await;
        assert!(v.starts_with("enc:v1:"), "passage.{col} doit être chiffré");
    }
    // Passage créé par l'ancienne route : chiffré lui aussi.
    let legacy = ent
        .creer_passage_infirmerie_for_actor(
            ri,
            CreerPassageInfirmerie {
                etablissement_id: etab,
                eleve_id: hermione,
                jour: today_utc(),
                motif: PassageInfirmerieMotif::Soins,
                note_soins: SENT_CLINIQUE.into(),
                visible_famille: false,
                actor_user_id: i,
            },
        )
        .await
        .unwrap();
    let v = brut(&ent, "SELECT note_soins FROM ent_infirmerie_passage WHERE id = $1", legacy.id).await;
    assert!(v.starts_with("enc:v1:"));
    let relus = ent.list_passages_infirmerie_for_actor(etab, i, ri, Some(hermione), None).await.unwrap();
    assert!(relus.iter().any(|p| p.note_soins == SENT_CLINIQUE));
}

// ─── 3. Minimisation : aucune donnée de santé hors des tables de santé ───────

#[tokio::test]
async fn minimisation_aucune_donnee_de_sante_dans_evenements_notifications_journal() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (i, ri) = inf(&cast);
    let parent = (cast.parent_lea.user_id, "parent");
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(lea).bind(parent.0).execute(ent.pool()).await.unwrap();

    // Saisie de tous les textes cliniques par tous les chemins.
    ent.upsert_fiche_sante_for_actor(
        ri,
        UpsertFicheSante {
            etablissement_id: etab,
            eleve_id: lea,
            allergies: SENT_ALLERGIE.into(),
            traitements: SENT_TRAITEMENT.into(),
            contre_indications: SENT_ALLERGIE.into(),
            pai_actif: true,
            medecin_nom: Some("Dr Test".into()),
            urgence_tel: Some("0600000000".into()),
            note_confidentielle: SENT_NOTE.into(),
            actor_user_id: i,
        },
    )
    .await
    .unwrap();
    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), Some(extras_trousse())).await.unwrap();
    ent.signer_pai_for_actor(etab, parent.0, parent.1, lea, "famille", None).await.unwrap();
    ent.upsert_dossier_psy_for_actor(
        Role::Psy.as_str(),
        domain_eleve::UpsertDossierPsy {
            etablissement_id: etab,
            eleve_id: lea,
            axes_suivi: SENT_PSY.into(),
            contacts_externes: SENT_PSY.into(),
            note_confidentielle: SENT_PSY.into(),
            suivi_actif: true,
            signale_famille: false,
            actor_user_id: cast.psys[0].user_id,
        },
    )
    .await
    .unwrap();
    ent.creer_passage_infirmerie_for_actor(
        ri,
        CreerPassageInfirmerie {
            etablissement_id: etab,
            eleve_id: lea,
            jour: today_utc(),
            motif: PassageInfirmerieMotif::Accident,
            note_soins: SENT_CLINIQUE.into(),
            visible_famille: true,
            actor_user_id: i,
        },
    )
    .await
    .unwrap();
    // Passage « accident » : la règle informe la famille sans détail.
    let p = ent
        .enregistrer_passage_for_actor(
            i,
            ri,
            NouveauPassage {
                etablissement_id: etab,
                eleve_id: lea,
                jour: today_utc(),
                heure_arrivee: None,
                motif: "accident".into(),
                note_soins: SENT_CLINIQUE.into(),
                soins: SENT_SOINS.into(),
                visible_famille: true,
                issue: "retour_classe".into(),
                informer_famille: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(p.famille_information, "informee");
    ent.publier_contrainte_for_actor(
        i,
        ri,
        PublierContrainte {
            etablissement_id: etab,
            eleve_id: lea,
            code: "repas_sans_arachide".into(),
            audiences: vec![Audience::Restauration],
            date_fin: None,
        },
    )
    .await
    .unwrap();
    // Consultations (journal).
    ent.get_fiche_sante_for_actor(etab, i, ri, lea).await.unwrap();
    ent.resume_fiche_sante_pdf_for_actor(etab, i, ri, lea).await.unwrap();
    ent.fiche_urgence_for_actor(etab, i, ri, lea).await.unwrap();

    distribuer(&ent, etab).await;
    let texte = texte_non_sante(&ent, etab).await;
    sans_sentinelle(&texte, "les événements, notifications ou le journal");
    // Les événements du lot ne portent pas non plus les codes de contrainte.
    let evts = texte_evenements_sante(&ent, etab).await;
    assert!(!evts.contains("repas_sans_arachide") && !evts.contains("Infirmerie, armoire rouge"));

    // La notification de famille existe et ne dit rien du motif ni des soins.
    let notifs = ent.mes_notifications(etab, parent.0, 50).await.unwrap();
    let n = notifs.iter().find(|n| n.kind == "infirmerie_passage").expect("notification famille");
    assert!(n.corps.contains("infirmerie") && !n.corps.to_lowercase().contains("accident"));
    assert!(!n.titre.to_lowercase().contains("accident"));

    // Listes non santé : le dossier de l'élève (direction) ne contient aucune sentinelle.
    let fiche = ent.fiche_eleve_for_actor(etab, cast.direction.user_id, "direction", lea).await.unwrap();
    let rendu = format!("{fiche:?}");
    sans_sentinelle(&rendu, "la fiche 360° (liste non santé)");
    let chrono = rendu.to_lowercase();
    assert!(!chrono.contains("anaphylaxie") && !chrono.contains("arachide"));
}

async fn texte_evenements_sante(ent: &PgEnt, etab: Uuid) -> String {
    let l: Vec<(String,)> = sqlx::query_as(
        "SELECT payload::text FROM ent_metier_event
          WHERE etablissement_id = $1
            AND (type LIKE 'infirmerie.%' OR type LIKE 'sante.%' OR type LIKE 'psy.%'
                 OR type LIKE 'accompagnement.%')",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(!l.is_empty(), "des événements de santé étaient attendus");
    l.into_iter().map(|(t,)| t).collect::<Vec<_>>().join("\n")
}

// ─── 4. Contraintes publiées : la contrainte, jamais le diagnostic ───────────

#[tokio::test]
async fn contraintes_publiees_par_audience_et_droits() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea, draco) = (cast.etablissement_id, cast.eleve_lea_id, cast.eleve_tom_id);
    let (i, ri) = inf(&cast);
    let (pr, rp) = prof(&cast);
    let (cp, rc) = (cast.compta.user_id, Role::Compta.as_str());

    // La trousse d'urgence du PAI se publie d'elle-même (enseignants + sorties).
    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), Some(extras_trousse())).await.unwrap();
    let pub_ = |code: &str, a: Vec<Audience>, eleve| PublierContrainte {
        etablissement_id: etab,
        eleve_id: eleve,
        code: code.into(),
        audiences: a,
        date_fin: None,
    };
    ent.publier_contrainte_for_actor(i, ri, pub_("repas_sans_arachide", vec![Audience::Restauration], lea)).await.unwrap();
    ent.publier_contrainte_for_actor(i, ri, pub_("ne_pas_courir", vec![Audience::Enseignants, Audience::Sorties], lea)).await.unwrap();
    ent.publier_contrainte_for_actor(i, ri, pub_("repas_sans_lait", vec![Audience::Restauration], draco)).await.unwrap();

    // Hors catalogue ou audience non prévue : refus (aucun texte libre).
    for (code, a) in [
        ("asthme_severe", vec![Audience::Enseignants]),
        ("repas_sans_arachide", vec![Audience::Enseignants]),
        ("ne_pas_courir", vec![Audience::Restauration]),
        ("ne_pas_courir", vec![]),
    ] {
        assert!(
            ent.publier_contrainte_for_actor(i, ri, pub_(code, a, lea)).await.is_err(),
            "{code} aurait dû être refusé"
        );
    }
    // Le professeur ne publie pas.
    assert!(matches!(
        ent.publier_contrainte_for_actor(pr, rp, pub_("ne_pas_courir", vec![Audience::Enseignants], lea)).await,
        Err(EntDbError::Forbidden)
    ));

    // Restauration (comptabilité = repas.gerer) : seulement l'audience restauration, sans source.
    let cantine = ent.contraintes_liste_for_actor(etab, cp, rc, Audience::Restauration).await.unwrap();
    let codes: Vec<_> = cantine.iter().flat_map(|e| e.contraintes.iter().map(|c| c.code.as_str())).collect();
    assert!(codes.contains(&"repas_sans_arachide") && codes.contains(&"repas_sans_lait"));
    assert!(!codes.contains(&"ne_pas_courir") && !codes.contains(&"trousse_urgence"));
    assert!(cantine.iter().all(|e| e.contraintes.iter().all(|c| c.source.is_none())));
    let rendu = serde_json::to_string(&cantine).unwrap();
    assert!(!rendu.contains("SENT-") && !rendu.contains("pai"));
    // La comptabilité n'a pas l'audience « enseignants ».
    assert!(matches!(
        ent.contraintes_liste_for_actor(etab, cp, rc, Audience::Enseignants).await,
        Err(EntDbError::Forbidden)
    ));

    // Professeur : ses élèves (Harry), audience enseignants seulement ; jamais le repas.
    let mes = ent.contraintes_liste_for_actor(etab, pr, rp, Audience::Enseignants).await.unwrap();
    assert!(mes.iter().all(|e| e.eleve_id != draco), "Draco n'est pas son élève");
    let l = mes.iter().find(|e| e.eleve_id == lea).expect("Harry");
    let c: Vec<_> = l.contraintes.iter().map(|c| c.code.as_str()).collect();
    assert!(c.contains(&"ne_pas_courir") && c.contains(&"trousse_urgence") && !c.contains(&"repas_sans_arachide"));
    assert!(matches!(
        ent.contraintes_liste_for_actor(etab, pr, rp, Audience::Restauration).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.contraintes_eleve_for_actor(etab, pr, rp, draco, Some(Audience::Enseignants)).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.contraintes_eleve_for_actor(etab, pr, rp, lea, None).await,
        Err(EntDbError::Forbidden)
    ));
    // La famille ne passe pas par cette voie.
    assert!(ent
        .contraintes_eleve_for_actor(etab, cast.parent_lea.user_id, "parent", lea, Some(Audience::Enseignants))
        .await
        .is_err());
    // Personnel de santé : tout, avec la source.
    let toutes = ent.contraintes_eleve_for_actor(etab, i, ri, lea, None).await.unwrap();
    assert!(toutes.len() >= 3 && toutes.iter().any(|c| c.source.as_deref() == Some("pai")));

    // Retrait : disparaît des listes ; PAI désactivé retire ce qu'il a dérivé.
    let arachide = toutes.iter().find(|c| c.code == "repas_sans_arachide").unwrap().id;
    ent.retirer_contrainte_for_actor(etab, i, ri, arachide).await.unwrap();
    assert!(matches!(
        ent.retirer_contrainte_for_actor(etab, i, ri, arachide).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    // Une publication manuelle du même code ne détache pas la contrainte du PAI.
    ent.publier_contrainte_for_actor(i, ri, pub_("trousse_urgence", vec![Audience::Enseignants], lea)).await.unwrap();
    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, false), None).await.unwrap();
    let apres = ent.contraintes_eleve_for_actor(etab, i, ri, lea, None).await.unwrap();
    assert!(apres.iter().all(|c| c.code != "trousse_urgence" && c.code != "repas_sans_arachide"));
    assert!(apres.iter().any(|c| c.code == "ne_pas_courir"), "la contrainte manuelle reste");
}

// ─── 5. Passages, information de la famille, renvoi au domicile ──────────────

#[tokio::test]
async fn passages_famille_informee_sans_detail_et_renvoi_controle() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (i, ri) = inf(&cast);
    let (pr, rp) = prof(&cast);
    let parent = (cast.parent_lea.user_id, "parent");
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(lea).bind(parent.0).execute(ent.pool()).await.unwrap();
    let nouveau = |motif: &str, issue: &str, visible: bool| NouveauPassage {
        etablissement_id: etab,
        eleve_id: lea,
        jour: today_utc(),
        heure_arrivee: None,
        motif: motif.into(),
        note_soins: SENT_CLINIQUE.into(),
        soins: SENT_SOINS.into(),
        visible_famille: visible,
        issue: issue.into(),
        informer_famille: false,
    };

    // Soins banals : famille non informée par la règle.
    let banal = ent.enregistrer_passage_for_actor(i, ri, nouveau("soins", "retour_classe", false)).await.unwrap();
    assert_eq!(banal.famille_information, "non");
    // À la demande : informée une seule fois.
    let info = ent.informer_famille_passage_for_actor(etab, i, ri, banal.id).await.unwrap();
    assert_eq!(info.famille_information, "informee");
    let encore = ent.informer_famille_passage_for_actor(etab, i, ri, banal.id).await.unwrap();
    assert_eq!(encore.famille_informee_at, info.famille_informee_at);
    // Accident : informée automatiquement. Renvoi au domicile refusé à la création.
    let acc = ent.enregistrer_passage_for_actor(i, ri, nouveau("accident", "non_renseignee", true)).await.unwrap();
    assert_eq!(acc.famille_information, "informee");
    assert!(ent.enregistrer_passage_for_actor(i, ri, nouveau("soins", "renvoi_domicile", false)).await.is_err());
    assert!(ent.enregistrer_passage_for_actor(i, ri, nouveau("pas_un_motif", "retour_classe", false)).await.is_err());

    distribuer(&ent, etab).await;
    // Rejouer la distribution ne duplique pas les notifications.
    distribuer(&ent, etab).await;
    let n = ent.mes_notifications(etab, parent.0, 100).await.unwrap();
    let infos: Vec<_> = n.iter().filter(|n| n.kind == "infirmerie_passage").collect();
    assert_eq!(infos.len(), 2, "une notification par passage informé");
    for x in &infos {
        for mot in ["accident", "soins", "SENT-", "retour"] {
            assert!(!x.corps.contains(mot) && !x.titre.contains(mot), "« {mot} » dans la notification");
        }
    }
    // Un parent d'un autre élève n'est pas notifié.
    let autre = ent.mes_notifications(etab, cast.parent_tom.user_id, 100).await.unwrap();
    assert!(autre.iter().all(|n| n.kind != "infirmerie_passage"));

    // Vue famille : seulement les passages visibles, sans note ni soins.
    let fam = ent.passages_sante_for_actor(etab, parent.0, parent.1, Some(lea), None).await.unwrap();
    // Le jeu de labo porte déjà des passages : on vérifie les nôtres et la règle générale.
    assert!(fam.iter().any(|p| p.id == acc.id), "passage visible absent de la vue famille");
    assert!(fam.iter().all(|p| p.id != banal.id && p.visible_famille), "passage non visible exposé");
    assert!(fam.iter().all(|p| p.note_soins.is_empty() && p.soins.is_empty() && p.recupere_par_nom.is_none()));
    // La famille ne lit pas le tableau du jour ; le professeur non plus.
    assert!(ent.passages_sante_for_actor(etab, parent.0, parent.1, None, None).await.is_err());
    assert!(matches!(
        ent.passages_sante_for_actor(etab, pr, rp, None, None).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.passages_sante_for_actor(etab, cast.parent_tom.user_id, "parent", Some(lea), None).await.is_err());
    // Tableau du jour pour l'infirmerie.
    let jour = ent.passages_sante_for_actor(etab, i, ri, None, None).await.unwrap();
    assert!(jour.iter().any(|p| p.id == acc.id && p.note_soins == SENT_CLINIQUE));

    // Renvoi au domicile : exige une personne habilitée (responsable actif / contact validé).
    let sans = ClorePassage {
        etablissement_id: etab,
        passage_id: banal.id,
        heure_sortie: None,
        issue: "renvoi_domicile".into(),
        soins: None,
        recupere_par_personne_id: None,
    };
    assert!(ent.cloturer_passage_for_actor(i, ri, sans.clone()).await.is_err());
    let inconnue = ClorePassage { recupere_par_personne_id: Some(Uuid::new_v4()), ..sans.clone() };
    assert!(ent.cloturer_passage_for_actor(i, ri, inconnue).await.is_err());
    let resp = ent.responsables_eleve_for_actor(etab, cast.direction.user_id, "direction", lea).await.unwrap();
    let r = resp.iter().find(|r| r.actif).expect("un responsable actif");
    let ok = ent
        .cloturer_passage_for_actor(i, ri, ClorePassage { recupere_par_personne_id: Some(r.personne.id), ..sans })
        .await
        .unwrap();
    assert_eq!(ok.issue, "renvoi_domicile");
    assert_eq!(ok.recupere_par_nom.as_deref(), Some(r.personne.libelle().as_str()));
    let evts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'infirmerie.renvoi_domicile'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(evts, 1);
    // Le professeur ne clôture pas un passage.
    assert!(matches!(
        ent.cloturer_passage_for_actor(pr, rp, ClorePassage {
            etablissement_id: etab,
            passage_id: banal.id,
            heure_sortie: None,
            issue: "retour_classe".into(),
            soins: None,
            recupere_par_personne_id: None,
        })
        .await,
        Err(EntDbError::Forbidden)
    ));
}

// ─── 6. Accompagnement : PAP/PPS, aménagements, vue professeur ───────────────

#[tokio::test]
async fn accompagnement_amenagements_visibles_du_professeur_sans_diagnostic() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea, draco) = (cast.etablissement_id, cast.eleve_lea_id, cast.eleve_tom_id);
    let (d, rd) = dir(&cast);
    let (pr, rp) = prof(&cast);
    let (i, ri) = inf(&cast);

    // PAP ouvert par la direction ; PPS refusé à l'infirmerie, PAP autorisé.
    let ouvrir = |eleve, kind: &str| OuvrirDispositif {
        etablissement_id: etab,
        eleve_id: eleve,
        kind: kind.into(),
        libelle: Some("Suivi renforcé".into()),
        document_id: None,
        date_debut: None,
        date_revision: Some(today_utc() + Duration::days(20)),
    };
    let pap = ent.ouvrir_dispositif_for_actor(d, rd, ouvrir(lea, "pap")).await.unwrap();
    assert!(pap.revision_due && pap.dispositif.statut == "actif");
    assert!(matches!(
        ent.ouvrir_dispositif_for_actor(i, ri, ouvrir(lea, "pps")).await,
        Err(EntDbError::Forbidden)
    ));
    ent.ouvrir_dispositif_for_actor(i, ri, ouvrir(draco, "pap")).await.unwrap();
    assert!(ent.ouvrir_dispositif_for_actor(d, rd, ouvrir(lea, "pai")).await.is_err(), "le PAI a son propre parcours");
    assert!(matches!(
        ent.ouvrir_dispositif_for_actor(pr, rp, ouvrir(lea, "pap")).await,
        Err(EntDbError::Forbidden)
    ));

    // Aménagements : catalogue fermé.
    let def = |eleve, code: &str, dispo| DefinirAmenagement {
        etablissement_id: etab,
        eleve_id: eleve,
        dispositif_id: dispo,
        code: code.into(),
        portee: "les_deux".into(),
        date_fin: None,
    };
    ent.definir_amenagement_for_actor(d, rd, def(lea, "tiers_temps", Some(pap.dispositif.id))).await.unwrap();
    ent.definir_amenagement_for_actor(d, rd, def(lea, "police_adaptee", Some(pap.dispositif.id))).await.unwrap();
    assert!(ent.definir_amenagement_for_actor(d, rd, def(lea, "dyslexie_severe", None)).await.is_err());
    assert!(ent
        .definir_amenagement_for_actor(d, rd, DefinirAmenagement { portee: "partout".into(), ..def(lea, "tiers_temps", None) })
        .await
        .is_err());
    assert!(ent.definir_amenagement_for_actor(d, rd, def(lea, "tiers_temps", Some(Uuid::new_v4()))).await.is_err());
    assert!(matches!(
        ent.definir_amenagement_for_actor(pr, rp, def(lea, "calculatrice", None)).await,
        Err(EntDbError::Forbidden)
    ));
    ent.definir_amenagement_for_actor(d, rd, def(draco, "salle_separee", None)).await.unwrap();

    // Vue professeur : ses élèves, aménagements seulement.
    let vus = ent.amenagements_eleve_for_actor(etab, pr, rp, lea).await.unwrap();
    let codes: Vec<_> = vus.iter().map(|a| a.code.as_str()).collect();
    assert_eq!(codes, vec!["police_adaptee", "tiers_temps"]);
    assert!(matches!(
        ent.amenagements_eleve_for_actor(etab, pr, rp, draco).await,
        Err(EntDbError::Forbidden)
    ));
    let mes = ent.mes_amenagements_for_actor(etab, pr, rp).await.unwrap();
    assert!(mes.iter().all(|e| e.eleve_id != draco) && mes.iter().any(|e| e.eleve_id == lea));
    let json = serde_json::to_string(&mes).unwrap().to_lowercase();
    for interdit in ["pap", "pps", "dispositif", "kind", "libelle\":\"suivi", "suivi renforcé", "pai"] {
        assert!(!json.contains(interdit), "« {interdit} » dans la vue professeur : {json}");
    }
    // Le professeur ne lit ni le dossier d'accompagnement, ni la liste à réviser.
    assert!(matches!(
        ent.dossier_accompagnement_for_actor(etab, pr, rp, lea).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.dispositifs_a_reviser_for_actor(etab, pr, rp, 30).await,
        Err(EntDbError::Forbidden)
    ));
    // Famille : les aménagements de SON enfant seulement.
    assert_eq!(ent.amenagements_eleve_for_actor(etab, cast.parent_lea.user_id, "parent", lea).await.unwrap().len(), 2);
    assert!(ent.amenagements_eleve_for_actor(etab, cast.parent_tom.user_id, "parent", lea).await.is_err());
    // Comptabilité : rien.
    assert!(matches!(
        ent.amenagements_eleve_for_actor(etab, cast.compta.user_id, "compta", lea).await,
        Err(EntDbError::Forbidden)
    ));

    // Un dispositif d'un autre élève ne se lie pas (AESH, réunion).
    let autre_dispo = ent.ouvrir_dispositif_for_actor(d, rd, ouvrir(draco, "pps")).await.unwrap();
    assert!(ent
        .affecter_aesh_for_actor(d, rd, crate::accompagnement::AffecterAesh {
            etablissement_id: etab,
            eleve_id: lea,
            dispositif_id: Some(autre_dispo.dispositif.id),
            aesh_nom: "X".into(),
            heures_hebdo: None,
            date_debut: today_utc(),
            date_fin: None,
        })
        .await
        .is_err());
    // AESH et équipe éducative ; compte rendu chiffré et réservé à la gestion.
    let aesh = ent
        .affecter_aesh_for_actor(
            d,
            rd,
            crate::accompagnement::AffecterAesh {
                etablissement_id: etab,
                eleve_id: lea,
                dispositif_id: Some(pap.dispositif.id),
                aesh_nom: "Mme Lupin".into(),
                heures_hebdo: Some(12),
                date_debut: today_utc(),
                date_fin: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(aesh.heures_hebdo, Some(12));
    let reunion = ent
        .planifier_reunion_for_actor(
            d,
            rd,
            crate::accompagnement::PlanifierReunion {
                etablissement_id: etab,
                eleve_id: lea,
                dispositif_id: None,
                kind: "equipe_suivi".into(),
                date_reunion: today_utc() + Duration::days(7),
            },
        )
        .await
        .unwrap();
    ent.consigner_reunion_for_actor(etab, d, rd, reunion.id, SENT_PSY).await.unwrap();
    let brut_cr = brut(&ent, "SELECT compte_rendu FROM ent_equipe_reunion WHERE id = $1", reunion.id).await;
    assert!(brut_cr.starts_with("enc:v1:"));
    let dossier = ent.dossier_accompagnement_for_actor(etab, d, rd, lea).await.unwrap();
    assert_eq!(dossier.reunions[0].compte_rendu, SENT_PSY);
    let vue_inf = ent.dossier_accompagnement_for_actor(etab, i, ri, lea).await.unwrap();
    assert!(vue_inf.reunions[0].compte_rendu.is_empty(), "compte rendu réservé à la gestion");

    // Révision puis clôture : les aménagements du dispositif sont retirés.
    let a_reviser = ent.dispositifs_a_reviser_for_actor(etab, d, rd, 30).await.unwrap();
    assert!(a_reviser.iter().any(|x| x.dispositif_id == pap.dispositif.id));
    let revise = ent
        .reviser_dispositif_for_actor(etab, d, rd, pap.dispositif.id, Some(today_utc() + Duration::days(300)), false)
        .await
        .unwrap();
    assert!(!revise.revision_due);
    ent.reviser_dispositif_for_actor(etab, d, rd, pap.dispositif.id, None, true).await.unwrap();
    assert!(ent.amenagements_eleve_for_actor(etab, pr, rp, lea).await.unwrap().is_empty());
    assert!(matches!(
        ent.reviser_dispositif_for_actor(etab, d, rd, pap.dispositif.id, None, true).await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Événements : identifiants et états seulement.
    let evts = texte_evenements_sante(&ent, etab).await;
    for interdit in ["tiers_temps", "police_adaptee", "pap", "Mme Lupin", "SENT-"] {
        assert!(!evts.contains(interdit), "« {interdit} » dans un événement d'accompagnement");
    }
}

// ─── 7. Fiche d'urgence : niveaux, droits, journal de consultation ───────────

#[tokio::test]
async fn fiche_d_urgence_niveaux_droits_et_journal() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea, draco) = (cast.etablissement_id, cast.eleve_lea_id, cast.eleve_tom_id);
    let (i, ri) = inf(&cast);
    let (pr, rp) = prof(&cast);
    ent.upsert_fiche_sante_for_actor(
        ri,
        UpsertFicheSante {
            etablissement_id: etab,
            eleve_id: lea,
            allergies: SENT_ALLERGIE.into(),
            traitements: SENT_TRAITEMENT.into(),
            contre_indications: String::new(),
            pai_actif: true,
            medecin_nom: Some("Dr Rogue".into()),
            urgence_tel: Some("0611223344".into()),
            note_confidentielle: String::new(),
            actor_user_id: i,
        },
    )
    .await
    .unwrap();
    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), Some(extras_trousse())).await.unwrap();
    ent.publier_contrainte_for_actor(
        i,
        ri,
        PublierContrainte {
            etablissement_id: etab,
            eleve_id: lea,
            code: "ne_pas_courir".into(),
            audiences: vec![Audience::Enseignants, Audience::Sorties],
            date_fin: None,
        },
    )
    .await
    .unwrap();

    ent.publier_contrainte_for_actor(
        i,
        ri,
        PublierContrainte {
            etablissement_id: etab,
            eleve_id: lea,
            code: "repas_sans_arachide".into(),
            audiences: vec![Audience::Restauration],
            date_fin: None,
        },
    )
    .await
    .unwrap();
    let complete = ent.fiche_urgence_for_actor(etab, i, ri, lea).await.unwrap();
    assert!(complete.contraintes.iter().any(|c| c.code == "repas_sans_arachide"));
    assert_eq!(complete.niveau, "complete");
    assert_eq!(complete.protocole_urgence.as_deref(), Some(SENT_PROTOCOLE));
    assert_eq!(complete.medecin_nom.as_deref(), Some("Dr Rogue"));
    assert!(complete.trousse_urgence && complete.trousse_lieu.contains("armoire"));
    assert!(!complete.responsables.is_empty());

    // Encadrant (professeur de l'élève) : minimale — contacts, contraintes, trousse ;
    // ni protocole, ni médecin, ni diagnostic, ni traitement.
    let min = ent.fiche_urgence_for_actor(etab, pr, rp, lea).await.unwrap();
    assert_eq!(min.niveau, "minimale");
    assert!(min.protocole_urgence.is_none() && min.medecin_nom.is_none() && min.urgence_tel.is_none());
    assert!(min.contraintes.iter().any(|c| c.code == "ne_pas_courir"));
    // Niveau minimal : jamais les contraintes réservées à la restauration (allergie alimentaire).
    assert!(min.contraintes.iter().all(|c| c.code != "repas_sans_arachide"));
    assert!(min.trousse_urgence);
    let rendu = serde_json::to_string(&min).unwrap();
    for s in TOUS_SENTINELLES {
        assert!(!rendu.contains(s), "« {s} » dans la fiche d'urgence minimale");
    }
    assert!(!rendu.to_lowercase().contains("anaphylaxie"));
    assert!(min.consigne_generale.contains("15"));

    // Hors périmètre : refus (professeur sur un élève qu'il n'enseigne pas, comptabilité, famille).
    assert!(matches!(ent.fiche_urgence_for_actor(etab, pr, rp, draco).await, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.fiche_urgence_for_actor(etab, cast.compta.user_id, "compta", lea).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.fiche_urgence_for_actor(etab, cast.parent_lea.user_id, "parent", lea).await,
        Err(EntDbError::Forbidden)
    ));

    // Journal de consultation : un enregistrement par accès, avec l'acteur réel.
    let (n_compl, n_min): (i64, i64) = (
        sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND eleve_id = $2 AND action = 'fiche_urgence' AND actor_user_id = $3 AND libelle LIKE '%complète%'")
            .bind(etab).bind(lea).bind(i).fetch_one(ent.pool()).await.unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND eleve_id = $2 AND action = 'fiche_urgence' AND actor_user_id = $3 AND libelle LIKE '%minimale%'")
            .bind(etab).bind(lea).bind(pr).fetch_one(ent.pool()).await.unwrap(),
    );
    assert_eq!((n_compl, n_min), (1, 1));
    // Lecture du PAI et des passages journalisée aussi.
    ent.pai_complet_for_actor(etab, i, ri, lea).await.unwrap();
    ent.passages_sante_for_actor(etab, i, ri, Some(lea), None).await.unwrap();
    for action in ["pai", "passages_infirmerie"] {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = $2 AND actor_user_id = $3",
        )
        .bind(etab)
        .bind(action)
        .bind(i)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert!(n >= 1, "consultation « {action} » non journalisée");
    }
}

// ─── 8. Isolation entre établissements et RLS ────────────────────────────────

#[tokio::test]
async fn isolation_des_etablissements_et_rls_des_nouvelles_tables() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (i, ri) = inf(&cast);
    ent.enregistrer_pai_for_actor(ri, pai_cmd(&cast, lea, true), Some(extras_trousse())).await.unwrap();
    ent.publier_contrainte_for_actor(
        i,
        ri,
        PublierContrainte {
            etablissement_id: etab,
            eleve_id: lea,
            code: "repas_sans_oeuf".into(),
            audiences: vec![Audience::Restauration],
            date_fin: None,
        },
    )
    .await
    .unwrap();

    // Autre établissement : son infirmière ne voit rien, ne peut rien écrire sur cet élève.
    let autre = ent.seed_labo_cast().await.unwrap();
    assert_ne!(autre.etablissement_id, etab);
    let (ai, ar) = (autre.infirmieres[0].user_id, "infirmerie");
    assert!(ent.fiche_urgence_for_actor(autre.etablissement_id, ai, ar, lea).await.is_err());
    assert!(matches!(
        ent.pai_complet_for_actor(autre.etablissement_id, ai, ar, lea).await,
        Err(_) | Ok(None)
    ));
    assert!(ent
        .publier_contrainte_for_actor(
            ai,
            ar,
            PublierContrainte {
                etablissement_id: autre.etablissement_id,
                eleve_id: lea,
                code: "repas_sans_oeuf".into(),
                audiences: vec![Audience::Restauration],
                date_fin: None,
            },
        )
        .await
        .is_err());
    assert!(ent
        .enregistrer_pai_for_actor(ar, UpsertPai { etablissement_id: autre.etablissement_id, eleve_id: lea, actor_user_id: ai, ..pai_cmd(&cast, lea, true) }, None)
        .await
        .is_err());
    let liste = ent
        .contraintes_liste_for_actor(autre.etablissement_id, autre.compta.user_id, "compta", Audience::Restauration)
        .await
        .unwrap();
    assert!(liste.iter().all(|e| e.eleve_id != lea));
    // Id d'une contrainte d'un autre établissement : introuvable.
    let id: Uuid = sqlx::query_scalar("SELECT id FROM ent_contrainte_publiee WHERE etablissement_id = $1 LIMIT 1")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(ent.retirer_contrainte_for_actor(autre.etablissement_id, ai, ar, id).await.is_err());

    // RLS : activée, forcée, avec politique sur toutes les nouvelles tables.
    for t in [
        "ent_pai_signature",
        "ent_contrainte_publiee",
        "ent_amenagement",
        "ent_aesh_affectation",
        "ent_equipe_reunion",
        "ent_sante_protocole",
        "ent_pai",
        "ent_infirmerie_passage",
        "ent_fiche_sante",
    ] {
        let (actif, force): (bool, bool) =
            sqlx::query_as("SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE relname = $1")
                .bind(t)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        assert!(actif && force, "RLS non forcée sur {t}");
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_policies WHERE tablename = $1 AND policyname = 'ent_rls_etablissement'")
            .bind(t)
            .fetch_one(ent.pool())
            .await
            .unwrap();
        assert_eq!(n, 1, "politique RLS absente sur {t}");
    }
}

// ─── 9. Registre des protocoles ──────────────────────────────────────────────

#[tokio::test]
async fn registre_des_protocoles_versions_droits_et_evenement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (i, ri) = inf(&cast);
    let (pr, rp) = prof(&cast);
    let titre = format!("Protocole test {}", Uuid::new_v4());
    let p = ent
        .enregistrer_protocole_for_actor(
            i,
            ri,
            EnregistrerProtocole {
                etablissement_id: etab,
                id: None,
                titre: titre.clone(),
                categorie: "urgence".into(),
                contenu: "Conduite à tenir type.".into(),
                actif: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(p.version, 1);
    let p2 = ent
        .enregistrer_protocole_for_actor(
            i,
            ri,
            EnregistrerProtocole {
                etablissement_id: etab,
                id: Some(p.id),
                titre: titre.clone(),
                categorie: "urgence".into(),
                contenu: "Conduite à tenir type, révisée.".into(),
                actif: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(p2.version, 2);
    // Titre déjà pris, catégorie inconnue : refusés.
    let dup = EnregistrerProtocole {
        etablissement_id: etab,
        id: None,
        titre: titre.clone(),
        categorie: "urgence".into(),
        contenu: "x".into(),
        actif: true,
    };
    assert!(ent.enregistrer_protocole_for_actor(i, ri, dup.clone()).await.is_err());
    assert!(ent
        .enregistrer_protocole_for_actor(i, ri, EnregistrerProtocole { titre: "Autre".into(), categorie: "zzz".into(), ..dup.clone() })
        .await
        .is_err());
    // Le professeur ne lit ni n'écrit le registre.
    assert!(matches!(ent.protocoles_for_actor(etab, pr, rp).await, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.enregistrer_protocole_for_actor(pr, rp, EnregistrerProtocole { titre: "Autre".into(), ..dup }).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.protocoles_for_actor(etab, i, ri).await.unwrap().iter().any(|x| x.id == p.id));
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'sante.protocole_mis_a_jour'
            AND aggregate_id = $2",
    )
    .bind(etab)
    .bind(p.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 2);
}
