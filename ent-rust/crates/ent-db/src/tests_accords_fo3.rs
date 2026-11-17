//! Socle 18, lot FO3 — accords sur objets : question « accord » dans un formulaire, règle `tous` / `un` par objet (FD-17), désaccord
//! remonté au socle 4, objet créé par un autre socle, dossier de preuve par la brique de signature du socle 7 (D6), contre la base
//! locale de test (`docs/socles/18_FORMULAIRES.md` §6–§10, §21.1, §31). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec
//! en CI**. Aucun test ne dépend de l'heure : les jours viennent de la base, les expirations s'écrivent à l'instant voulu.
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | FD-17 : l'image exige `tous` (création, relèvement, ligne ancienne, calcul) | `fd17_l_image_exige_la_regle_tous` |
//! | FRM-124 : question « accord » liée à un objet existant, règle de l'objet, décisions dans la transaction de la réponse | `frm124_question_accord_dans_un_formulaire` |
//! | FRM-111, FRM-112 : désaccord remonté au dossier de l'élève, `desaccord` pour la famille d'objets, levée par pièce | `frm111_frm112_desaccord_remonte_bloque_et_se_leve_par_piece` |
//! | `creer_pour` : objet d'une sortie, idempotent, source tracée | `creer_pour_l_objet_d_une_sortie_est_idempotent_et_utilisable` |
//! | D6 : dossier de preuve, S2 jamais simulé, signature d'un autre refusée, horodatage honnête | `d6_dossier_de_preuve_d_une_decision_sans_simuler` |
//! | cloisonnement par établissement et immuabilité | `fo3_separation_et_immuabilite` |

use super::*;
use crate::accords::*;
use crate::accords_fo3::ObjetPourSource;
use crate::documents::*;
use crate::documents_pdf::RenduPdfAttestation;
use crate::documents_repo::*;
use crate::documents_signature::*;
use crate::documents_signature_repo::*;
use crate::documents_signature_signer::*;
use crate::frm_moteur::*;
use crate::frm_reponse::{AccordSaisie, ReponseSaisie};
use crate::ref_commun::Acteur;
use crate::service::RefusMetier;
use crate::tests_demandes::{fiche, monde, Acteur as ActeurTest};
use crate::tests_formulaires_fo0::{
    classe_4b, labo, objet_aleatoire, parent_en_plus, retirer_parent,
};
use crate::tests_formulaires_fo2::{crit, nf, nq, vider_fo23};
use chrono::{Duration, NaiveDate, Utc};
use std::sync::Mutex;
use uuid::Uuid;

const DIR: &str = "direction";

async fn vider(ent: &PgEnt, etab: Uuid) {
    vider_fo23(ent, etab).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.purge_etablissement', 'on', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    for t in [
        "ent_acc_decision_usage",
        "ent_acc_decision",
        "ent_acc_version_usage",
        "ent_acc_version",
        "ent_acc_usage",
        "ent_acc_journal",
        "ent_acc_objet",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
}

fn choix(codes: &[&str], accorde: bool) -> Vec<ChoixUsage> {
    codes
        .iter()
        .map(|c| ChoixUsage {
            code: (*c).into(),
            accorde,
        })
        .collect()
}

fn saisie(objet: &str, eleve: Uuid, codes: &[&str], accorde: bool) -> DecisionSaisie {
    DecisionSaisie {
        objet_code: objet.into(),
        eleve_id: Some(eleve),
        usages: choix(codes, accorde),
    }
}

async fn verdict(ent: &PgEnt, etab: Uuid, objet: &str, usage: &str, eleve: Uuid) -> Verdict {
    let v = ent
        .accords_verifier(
            etab,
            &DemandeVerification {
                objet_code: objet.into(),
                usage_code: usage.into(),
                sujets: vec![Sujet::Eleve(eleve)],
                jour: None,
            },
        )
        .await;
    v[0].verdict
}

fn objet_propre(
    code: &str,
    nature: &str,
    regle: &str,
    droit: Option<&str>,
    usages: &[&str],
    desaccord: Option<&str>,
) -> NouvelObjet {
    NouvelObjet {
        code: code.into(),
        libelle: format!("Objet {code}"),
        nature: nature.into(),
        regle: regle.into(),
        avis_eleve_des_ans: if nature == "consentement" {
            Some(11)
        } else {
            None
        },
        droit_lien_code: droit.map(String::from),
        preuve_min: "S1".into(),
        validite: "annee_scolaire".into(),
        valide_jusqu_au: None,
        evenement_fin: None,
        sujet_type: "eleve".into(),
        desaccord_objet: desaccord.map(String::from),
        usages: usages
            .iter()
            .map(|u| NouvelUsage {
                code: (*u).into(),
                libelle: format!("Usage {u}"),
                mission: false,
            })
            .collect(),
        texte: "Texte d'accord de test".into(),
    }
}

async fn objet_publie(ent: &PgEnt, etab: Uuid, dir: Uuid, n: NouvelObjet) {
    let code = n.code.clone();
    ent.accords_creer_objet(etab, dir, DIR, n).await.unwrap();
    ent.accords_publier_version(etab, dir, DIR, &code, None)
        .await
        .unwrap();
}

async fn droit_sortie(ent: &PgEnt, etab: Uuid, user: Uuid, valeur: bool) {
    sqlx::query("UPDATE ent_eleve_lien SET peut_autoriser_sorties = $3 WHERE etablissement_id = $1 AND user_id = $2")
        .bind(etab)
        .bind(user)
        .bind(valeur)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

async fn nb_evenements(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2")
        .bind(etab)
        .bind(type_name)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

// ─── FD-17 ───────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn fd17_regle_effective_et_objets_d_image_sans_base() {
    assert!(objet_exige_tous("droit_image", None) && objet_exige_tous("voyage_etranger", None));
    assert!(
        objet_exige_tous("mon_objet_photos", Some("image")),
        "un objet propre de la famille image"
    );
    assert!(!objet_exige_tous("sortie_facultative", Some("sortie")));
    assert_eq!(
        regle_effective("un", "droit_image", Some("image"), "eleve"),
        "tous",
        "une ligne ancienne à `un` se calcule à `tous`"
    );
    assert_eq!(
        regle_effective("un", "mon_objet", Some("image"), "eleve"),
        "tous"
    );
    assert_eq!(
        regle_effective("un", "sortie_facultative", Some("sortie"), "eleve"),
        "un"
    );
    assert_eq!(
        regle_effective("soi", "liste_representants", None, "personne"),
        "soi"
    );
    assert_eq!(
        regle_effective("eleve_et_titulaires", "droit_image", Some("image"), "eleve"),
        "eleve_et_titulaires"
    );
}

#[tokio::test]
async fn fd17_l_image_exige_la_regle_tous() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider(&ent, etab).await;
    // Création : un objet de la famille « image » à la règle `un` est refusé, à `tous` accepté.
    let code = objet_aleatoire();
    let e = ent
        .accords_creer_objet(
            etab,
            dir,
            DIR,
            objet_propre(
                &code,
                "consentement",
                "un",
                Some("signer_documents"),
                &["fil_classe"],
                Some("image"),
            ),
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::ConflitDossier(m)) if m.contains("image")),
        "{e:?}"
    );
    let e = ent
        .accords_creer_objet(
            etab,
            dir,
            DIR,
            objet_propre(&code, "consentement", "un", None, &["x"], Some("inconnue")),
        )
        .await;
    assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &code,
            "consentement",
            "tous",
            Some("signer_documents"),
            &["fil_classe"],
            Some("image"),
        ),
    )
    .await;
    // Le catalogue (`droit_image`) est déjà à `tous`, et ne baisse pas.
    ent.accords_activer_objet(etab, dir, DIR, "droit_image")
        .await
        .unwrap();
    for r in ["un", "eleve_seul", "soi"] {
        assert!(
            ent.accords_relever_regle(etab, dir, DIR, "droit_image", r)
                .await
                .is_err(),
            "{r}"
        );
    }
    // Une ligne **ancienne** (reprise, objet créé avant la correction) à `un` : le calcul la traite comme `tous`.
    let ancien = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &ancien,
            "consentement",
            "tous",
            Some("signer_documents"),
            &["fil_classe"],
            Some("image"),
        ),
    )
    .await;
    sqlx::query("UPDATE ent_acc_objet SET regle = 'un' WHERE etablissement_id = $1 AND code = $2")
        .bind(etab)
        .bind(&ancien)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    ent.accords_decider(etab, lea, saisie(&ancien, harry, &["fil_classe"], true))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &ancien, "fil_classe", harry).await,
        Verdict::EnAttente,
        "FD-17 : un seul accord ne suffit pas pour l'image, même si la ligne dit `un`"
    );
    ent.accords_decider(etab, pere, saisie(&ancien, harry, &["fil_classe"], true))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &ancien, "fil_classe", harry).await,
        Verdict::Autorise
    );
    retirer_parent(&ent, etab, pere).await;
    // Relever une règle : on monte, jamais l'inverse ; seule la paramétrage (direction) le fait ; le journal le trace.
    let sortie = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &sortie,
            "autorisation",
            "un",
            Some("autoriser_sorties"),
            &["participation"],
            Some("sortie"),
        ),
    )
    .await;
    assert!(matches!(
        ent.accords_relever_regle(
            etab,
            cast.administratif.user_id,
            "administratif",
            &sortie,
            "tous"
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(
        matches!(
            ent.accords_relever_regle(etab, dir, DIR, &sortie, "un")
                .await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "déjà `un`"
    );
    assert!(matches!(
        ent.accords_relever_regle(etab, dir, DIR, &sortie, "soi")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.accords_relever_regle(etab, dir, DIR, &sortie, "inconnue")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let o = ent
        .accords_relever_regle(etab, dir, DIR, &sortie, "tous")
        .await
        .unwrap();
    assert_eq!(o.regle, "tous");
    assert!(
        matches!(ent.accords_relever_regle(etab, dir, DIR, &sortie, "un").await, Err(EntDbError::ConflitDossier(m)) if m.contains("monter"))
    );
    let (releves,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_journal WHERE etablissement_id = $1 AND acte = 'regle_relevee'").bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(releves, 1);
    // Une information ne se refuse pas : `tous` interdit pour elle.
    let info = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &info,
            "information",
            "un",
            Some("signer_quotidien"),
            &["lecture"],
            None,
        ),
    )
    .await;
    assert!(matches!(
        ent.accords_relever_regle(etab, dir, DIR, &info, "tous")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    vider(&ent, etab).await;
}

// ─── FRM-124 ─────────────────────────────────────────────────────────────────────────────────────────

/// Formulaire « mixte » à une question « accord » sur l'objet `code`, audience : la classe 4B, publié par la direction.
async fn publie_accord(
    ent: &PgEnt,
    etab: Uuid,
    dir: Uuid,
    unite: &str,
    code: &str,
) -> (Uuid, StructureVue) {
    let f = ent
        .frm_creer(etab, dir, DIR, nf("Autorisation", "mixte", unite))
        .await
        .unwrap();
    let s = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "Autorisation".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    ent.frm_ajouter_question(
        etab,
        dir,
        DIR,
        f.id,
        NouvelleQuestion {
            objet_accord_code: Some(code.into()),
            ..nq(s, "accord", "Votre autorisation", true)
        },
    )
    .await
    .unwrap();
    ent.frm_ajouter_question(
        etab,
        dir,
        DIR,
        f.id,
        nq(s, "texte_court", "Un mot (facultatif)", false),
    )
    .await
    .unwrap();
    let c4b = classe_4b(ent, etab).await;
    ent.frm_definir_audience(
        etab,
        dir,
        DIR,
        f.id,
        vec![crit("inclure", "classe", Some(c4b))],
    )
    .await
    .unwrap();
    assert_eq!(
        ent.frm_publier(etab, dir, DIR, f.id, None)
            .await
            .unwrap()
            .statut,
        "publie"
    );
    (
        f.id,
        ent.frm_structure(etab, dir, DIR, f.id, None).await.unwrap(),
    )
}

#[tokio::test]
async fn frm124_question_accord_dans_un_formulaire() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider(&ent, etab).await;
    droit_sortie(&ent, etab, lea, true).await;
    let pere = parent_en_plus(&ent, etab, harry, true, true).await;
    let image = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &image,
            "consentement",
            "tous",
            Some("signer_documents"),
            &["fil_classe", "site_public"],
            Some("image"),
        ),
    )
    .await;
    let sortie = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &sortie,
            "autorisation",
            "un",
            Some("autoriser_sorties"),
            &["participation"],
            Some("sortie"),
        ),
    )
    .await;

    // Une question « accord » exige un formulaire « mixte » et un objet existant, actif et publié.
    let f = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Mal formé", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    let s = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    let accord = |code: &str| NouvelleQuestion {
        objet_accord_code: Some(code.into()),
        ..nq(s, "accord", "Accord", true)
    };
    assert!(
        matches!(
            ent.frm_ajouter_question(etab, dir, DIR, f.id, accord(&sortie))
                .await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "formulaire non mixte"
    );
    let fm = ent
        .frm_creer(etab, dir, DIR, nf("Mixte", "mixte", "eleve_un_titulaire"))
        .await
        .unwrap();
    let sm = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            fm.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    let accord_m = |code: &str| NouvelleQuestion {
        objet_accord_code: Some(code.into()),
        ..nq(sm, "accord", "Accord", true)
    };
    assert!(matches!(
        ent.frm_ajouter_question(etab, dir, DIR, fm.id, accord_m("objet_inexistant"))
            .await,
        Err(EntDbError::Domain(_))
    ));
    assert!(
        ent.frm_ajouter_question(etab, dir, DIR, fm.id, accord_m(&image))
            .await
            .is_ok(),
        "l'image à `tous` est utilisable"
    );
    // Une ligne ancienne d'image à `un` est refusée comme question (FD-17) : on relève la règle d'abord.
    let ancien = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &ancien,
            "consentement",
            "tous",
            Some("signer_documents"),
            &["fil_classe"],
            Some("image"),
        ),
    )
    .await;
    sqlx::query("UPDATE ent_acc_objet SET regle = 'un' WHERE etablissement_id = $1 AND code = $2")
        .bind(etab)
        .bind(&ancien)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(
        matches!(ent.frm_ajouter_question(etab, dir, DIR, fm.id, accord_m(&ancien)).await, Err(EntDbError::ConflitDossier(m)) if m.contains("image"))
    );
    // Un objet sans version publiée ou désactivé est refusé.
    ent.accords_activer_objet(etab, dir, DIR, "voyage_france")
        .await
        .unwrap();
    assert!(
        matches!(
            ent.frm_ajouter_question(etab, dir, DIR, fm.id, accord_m("voyage_france"))
                .await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "version non publiée"
    );
    // Un formulaire « mixte » sans accord, ou « collecte » avec un accord, ne se publie pas.
    let c4b = classe_4b(&ent, etab).await;
    ent.frm_ajouter_question(etab, dir, DIR, f.id, nq(s, "texte_court", "Mot", false))
        .await
        .unwrap();
    ent.frm_definir_audience(
        etab,
        dir,
        DIR,
        f.id,
        vec![crit("inclure", "classe", Some(c4b))],
    )
    .await
    .unwrap();
    // Sujet de l'objet et unité de réponse : une liste « personne » ne va pas avec une unité « un titulaire ».
    let liste = objet_aleatoire();
    let mut p = objet_propre(&liste, "consentement", "soi", None, &["liste_classe"], None);
    p.sujet_type = "personne".into();
    p.avis_eleve_des_ans = None;
    objet_publie(&ent, etab, dir, p).await;
    assert!(matches!(
        ent.frm_ajouter_question(etab, dir, DIR, fm.id, accord_m(&liste))
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Formulaire publié : sortie (règle `un`), les deux parents répondent.
    let (pid, st) = publie_accord(&ent, etab, dir, "eleve_chaque_titulaire", &sortie).await;
    let qa = st.sections[0]
        .questions
        .iter()
        .find(|q| q.type_question == "accord")
        .unwrap();
    assert_eq!(qa.objet_accord_code.as_deref(), Some(sortie.as_str()));
    let reponse = |accorde: bool| ReponseSaisie {
        eleve_id: harry,
        valeurs: vec![],
        accords: vec![AccordSaisie {
            question_id: qa.id,
            usages: choix(&["participation"], accorde),
        }],
    };
    // Question obligatoire sans accord : refus, rien n'est écrit.
    assert!(matches!(
        ent.frm_repondre(
            etab,
            lea,
            pid,
            ReponseSaisie {
                eleve_id: harry,
                valeurs: vec![],
                accords: vec![]
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert_eq!(
        verdict(&ent, etab, &sortie, "participation", harry).await,
        Verdict::EnAttente
    );
    // La mère répond : une soumission **et** une décision, liées, dans la même transaction.
    let r = ent
        .frm_repondre(etab, lea, pid, reponse(true))
        .await
        .unwrap();
    assert_eq!(
        (
            r.decisions.len(),
            r.decisions[0].decision.as_str(),
            r.decisions[0].qualite.as_str()
        ),
        (1, "accorde", "titulaire")
    );
    let (liee,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_decision WHERE etablissement_id = $1 AND formulaire_soumission_id = $2").bind(etab).bind(r.soumission_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(liee, 1);
    assert_eq!(
        verdict(&ent, etab, &sortie, "participation", harry).await,
        Verdict::Autorise,
        "règle `un` : un accord suffit"
    );
    // Jamais la décision de l'autre : la réponse du père ne contient que **sa** décision.
    let r2 = ent
        .frm_repondre(etab, pere, pid, reponse(false))
        .await
        .unwrap();
    assert_eq!(r2.decisions.len(), 1);
    assert_eq!(r2.decisions[0].decision, "refuse");
    assert_eq!(
        verdict(&ent, etab, &sortie, "participation", harry).await,
        Verdict::Desaccord,
        "règle `un` : refus après accord = désaccord"
    );
    // Même réponse renvoyée : la décision est « inchangée », aucune ligne en plus.
    let (avant,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_acc_decision WHERE etablissement_id = $1 AND sujet_eleve_id = $2",
    )
    .bind(etab)
    .bind(harry)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let r3 = ent
        .frm_repondre(etab, pere, pid, reponse(false))
        .await
        .unwrap();
    assert_eq!(r3.decisions[0].decision, "inchangee");
    let (apres,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_acc_decision WHERE etablissement_id = $1 AND sujet_eleve_id = $2",
    )
    .bind(etab)
    .bind(harry)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(avant, apres);
    // Tout ou rien : une erreur de décision (usage inconnu) n'écrit pas la soumission.
    let (nb_soum,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_soumission WHERE etablissement_id = $1 AND formulaire_id = $2").bind(etab).bind(pid).fetch_one(ent.pool_verif()).await.unwrap();
    let r_ko = ReponseSaisie {
        eleve_id: harry,
        valeurs: vec![],
        accords: vec![AccordSaisie {
            question_id: qa.id,
            usages: choix(&["usage_inconnu"], true),
        }],
    };
    assert!(ent.frm_repondre(etab, lea, pid, r_ko).await.is_err());
    let (nb_soum2,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_soumission WHERE etablissement_id = $1 AND formulaire_id = $2").bind(etab).bind(pid).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(nb_soum, nb_soum2);
    // Un père sans droit de sortie ne décide pas d'une sortie (droit du lien exigé par l'objet).
    droit_sortie(&ent, etab, pere, false).await;
    let e = ent.frm_repondre(etab, pere, pid, reponse(true)).await;
    assert!(
        e.is_err(),
        "sans `autoriser_sorties`, le décideur n'est pas qualifié : {e:?}"
    );
    droit_sortie(&ent, etab, pere, true).await;

    // Image, règle `tous`, unité « chaque titulaire » : en attente tant qu'un titulaire manque, autorisé quand les deux ont accordé.
    let (iid, ist) = publie_accord(&ent, etab, dir, "eleve_chaque_titulaire", &image).await;
    let qi = ist.sections[0]
        .questions
        .iter()
        .find(|q| q.type_question == "accord")
        .unwrap();
    let reponse_i = |accorde: bool, codes: &[&str]| ReponseSaisie {
        eleve_id: harry,
        valeurs: vec![],
        accords: vec![AccordSaisie {
            question_id: qi.id,
            usages: choix(codes, accorde),
        }],
    };
    ent.frm_repondre(
        etab,
        lea,
        iid,
        reponse_i(true, &["fil_classe", "site_public"]),
    )
    .await
    .unwrap();
    assert_eq!(
        verdict(&ent, etab, &image, "fil_classe", harry).await,
        Verdict::EnAttente,
        "FD-17 : un seul accord ne suffit pas"
    );
    ent.frm_repondre(etab, pere, iid, reponse_i(true, &["fil_classe"]))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &image, "fil_classe", harry).await,
        Verdict::Autorise
    );
    assert_eq!(
        verdict(&ent, etab, &image, "site_public", harry).await,
        Verdict::EnAttente,
        "le père n'a pas accordé le site public"
    );
    ent.frm_repondre(etab, pere, iid, reponse_i(false, &["fil_classe"]))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &image, "fil_classe", harry).await,
        Verdict::Refuse,
        "un refus d'un titulaire refuse (règle `tous`)"
    );
    // L'objet d'accord se désactive : le formulaire ne se republie pas avec une question vers un accord inactif.
    ent.accords_desactiver_objet(etab, dir, DIR, &sortie)
        .await
        .unwrap();
    let st2 = ent.frm_nouvelle_version(etab, dir, DIR, pid).await.unwrap();
    assert_eq!(st2.numero, 2);
    assert!(
        matches!(
            ent.frm_publier(etab, dir, DIR, pid, None).await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "accord désactivé"
    );
    retirer_parent(&ent, etab, pere).await;
    droit_sortie(&ent, etab, lea, false).await;
    vider(&ent, etab).await;
}

// ─── FRM-111, FRM-112 ────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn frm111_frm112_desaccord_remonte_bloque_et_se_leve_par_piece() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider(&ent, etab).await;
    droit_sortie(&ent, etab, lea, true).await;
    let pere = parent_en_plus(&ent, etab, harry, true, true).await;
    let (s1, s2) = (objet_aleatoire(), objet_aleatoire());
    for c in [&s1, &s2] {
        objet_publie(
            &ent,
            etab,
            dir,
            objet_propre(
                c,
                "autorisation",
                "un",
                Some("autoriser_sorties"),
                &["participation"],
                Some("sortie"),
            ),
        )
        .await;
    }
    // La mère accorde les deux sorties. Le père refuse la première : désaccord (règle `un`), **remonté** pour la famille « sortie ».
    ent.accords_decider(etab, lea, saisie(&s1, harry, &["participation"], true))
        .await
        .unwrap();
    ent.accords_decider(etab, lea, saisie(&s2, harry, &["participation"], true))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &s2, "participation", harry).await,
        Verdict::Autorise
    );
    ent.accords_decider(etab, pere, saisie(&s1, harry, &["participation"], false))
        .await
        .unwrap();
    let ouverts: Vec<(String, String)> = sqlx::query_as("SELECT objet, source FROM ent_acc_desaccord WHERE etablissement_id = $1 AND sujet_eleve_id = $2 AND leve_le IS NULL")
        .bind(etab)
        .bind(harry)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(ouverts, vec![("sortie".to_string(), "accord".to_string())]);
    assert!(
        nb_evenements(&ent, etab, "eleve.desaccord_declare").await >= 1,
        "publié pour le socle 4"
    );
    // Un désaccord né d'un refus vise **cette** sortie (objet visé) : les autres sorties de l'élève ne sont pas bloquées.
    assert_eq!(
        verdict(&ent, etab, &s1, "participation", harry).await,
        Verdict::Desaccord
    );
    assert!(!verdict(&ent, etab, &s1, "participation", harry)
        .await
        .permet());
    assert_eq!(
        verdict(&ent, etab, &s2, "participation", harry).await,
        Verdict::Autorise,
        "l'autre sortie reste autorisée"
    );
    // Un autre élève n'est pas concerné ; un autre objet (famille « image ») non plus.
    let img = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &img,
            "consentement",
            "tous",
            Some("signer_documents"),
            &["fil_classe"],
            Some("image"),
        ),
    )
    .await;
    assert_ne!(
        verdict(&ent, etab, &img, "fil_classe", harry).await,
        Verdict::Desaccord
    );
    // Un second refus sur le même objet ne rouvre rien (un seul désaccord ouvert par objet visé).
    ent.accords_decider(etab, pere, saisie(&s1, harry, &["participation"], true))
        .await
        .unwrap();
    ent.accords_decider(etab, pere, saisie(&s1, harry, &["participation"], false))
        .await
        .unwrap();
    let (n_ouverts,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_desaccord WHERE etablissement_id = $1 AND sujet_eleve_id = $2 AND leve_le IS NULL").bind(etab).bind(harry).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n_ouverts, 1);
    // Déclaration d'un désaccord **de famille** par un titulaire (pour son enfant seulement) et par l'établissement : sans doublon.
    let d = ent
        .accords_declarer_desaccord(etab, lea, "parent", harry, "sortie")
        .await
        .unwrap();
    assert_eq!(
        (d.objet.as_str(), d.source.as_str()),
        ("sortie", "declaration")
    );
    assert!(
        matches!(
            ent.accords_declarer_desaccord(etab, lea, "parent", harry, "sortie")
                .await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "déjà ouvert"
    );
    assert!(
        matches!(
            ent.accords_declarer_desaccord(
                etab,
                cast.parent_tom.user_id,
                "parent",
                harry,
                "sortie"
            )
            .await,
            Err(EntDbError::Domain(_))
        ),
        "pas son enfant : introuvable"
    );
    assert!(matches!(
        ent.accords_declarer_desaccord(etab, lea, "parent", harry, "inconnue")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Désormais **toutes** les sorties de l'élève sont « désaccord » : jamais autorisées.
    assert_eq!(
        verdict(&ent, etab, &s2, "participation", harry).await,
        Verdict::Desaccord,
        "la famille « sortie » est bloquée"
    );
    assert!(!verdict(&ent, etab, &s2, "participation", harry)
        .await
        .permet());
    // Lecture : direction et secrétariat seulement, journalisée.
    for (u, r) in [
        (lea, "parent"),
        (cast.cpe.user_id, "cpe"),
        (cast.prof.user_id, "prof"),
    ] {
        assert!(
            matches!(
                ent.accords_desaccords(etab, u, r, harry).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
    }
    let liste = ent
        .accords_desaccords(etab, cast.administratif.user_id, "administratif", harry)
        .await
        .unwrap();
    assert_eq!(liste.len(), 2);
    let precis = liste
        .iter()
        .find(|l| l.objet_code.as_deref() == Some(s1.as_str()))
        .expect("le désaccord de l'objet");
    assert_eq!(
        (precis.source.as_str(), precis.leve_le.is_none()),
        ("accord", true)
    );
    let (lectures,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_journal WHERE etablissement_id = $1 AND acte = 'lecture_detaillee' AND detail = 'desaccords'").bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(lectures, 1);
    // Levée : une **pièce** du dossier de l'élève et un motif ; jamais sans pièce, jamais par une famille.
    let id = d.id;
    let w = pieces(&ent, &cast, harry).await;
    assert!(matches!(
        ent.accords_lever_desaccord(etab, lea, "parent", id, w.document_id, "accord écrit")
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(
        matches!(
            ent.accords_lever_desaccord(
                etab,
                cast.administratif.user_id,
                "administratif",
                id,
                Uuid::new_v4(),
                "accord écrit"
            )
            .await,
            Err(EntDbError::Refus(RefusMetier::Invalide, _))
        ),
        "pièce inconnue"
    );
    assert!(matches!(
        ent.accords_lever_desaccord(
            etab,
            cast.administratif.user_id,
            "administratif",
            id,
            w.document_id,
            "  "
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // La pièce d'un autre élève ne lève pas ce désaccord.
    let autre_doc = pieces(&ent, &cast, cast.eleve_tom_id).await;
    assert!(matches!(
        ent.accords_lever_desaccord(
            etab,
            cast.administratif.user_id,
            "administratif",
            id,
            autre_doc.document_id,
            "accord écrit"
        )
        .await,
        Err(EntDbError::Refus(RefusMetier::Invalide, _))
    ));
    ent.accords_lever_desaccord(
        etab,
        cast.administratif.user_id,
        "administratif",
        id,
        w.document_id,
        "Accord écrit des deux parents du 12 mars",
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.accords_lever_desaccord(
            etab,
            cast.administratif.user_id,
            "administratif",
            id,
            w.document_id,
            "encore"
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(nb_evenements(&ent, etab, "eleve.desaccord_leve").await >= 1);
    assert_eq!(
        verdict(&ent, etab, &s2, "participation", harry).await,
        Verdict::Autorise,
        "désaccord de famille levé : l'autre sortie reprend"
    );
    assert_eq!(
        verdict(&ent, etab, &s1, "participation", harry).await,
        Verdict::Desaccord,
        "celui de la sortie contestée reste ouvert (et le refus du père aussi)"
    );
    // La levée d'un désaccord est en ajout seul : plus de modification ni de suppression.
    assert!(sqlx::query(
        "UPDATE ent_acc_desaccord SET leve_motif = 'autre' WHERE etablissement_id = $1"
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .is_err());
    assert!(
        sqlx::query("DELETE FROM ent_acc_desaccord WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // L'établissement enregistre un désaccord « général » (courrier, décision de justice) : source distincte.
    let g = ent
        .accords_declarer_desaccord(etab, dir, DIR, harry, "general")
        .await
        .unwrap();
    assert_eq!(g.source, "etablissement");
    // « image » déclaré : la famille image est bloquée (ce que la décision « sans l'autorisation de l'autre » demande).
    ent.accords_declarer_desaccord(etab, lea, "parent", harry, "image")
        .await
        .unwrap();
    ent.accords_decider(etab, lea, saisie(&img, harry, &["fil_classe"], true))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &img, "fil_classe", harry).await,
        Verdict::Desaccord
    );
    retirer_parent(&ent, etab, pere).await;
    droit_sortie(&ent, etab, lea, false).await;
    vider(&ent, etab).await;
}

// ─── creer_pour ──────────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn creer_pour_l_objet_d_une_sortie_est_idempotent_et_utilisable() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider(&ent, etab).await;
    droit_sortie(&ent, etab, lea, true).await;
    let source = Uuid::new_v4();
    let fin = Utc::now() + Duration::days(30);
    let pour = |catalogue: &str| ObjetPourSource {
        catalogue_code: catalogue.into(),
        source_socle: 42,
        source_type: "sortie".into(),
        source_id: source,
        libelle: "Sortie au musée du 12 mars".into(),
        evenement_fin: Some(fin),
        texte: None,
    };
    // Le socle appelant a contrôlé son droit : la brique exige un membre du personnel, jamais une famille ni un compte inconnu.
    assert!(matches!(
        ent.accords_creer_pour(etab, lea, pour("sortie_facultative"))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.accords_creer_pour(etab, Uuid::new_v4(), pour("sortie_facultative"))
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Catalogues permis : sortie et voyages seulement.
    assert!(matches!(
        ent.accords_creer_pour(etab, cast.prof.user_id, pour("droit_image"))
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.accords_creer_pour(
            etab,
            cast.prof.user_id,
            ObjetPourSource {
                libelle: " ".into(),
                ..pour("sortie_facultative")
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // L'organisateur (un professeur) crée l'objet : règle, droit du lien, preuve et validité viennent du catalogue.
    let o = ent
        .accords_creer_pour(etab, cast.prof.user_id, pour("sortie_facultative"))
        .await
        .unwrap();
    assert!(
        o.code.starts_with("sortie_") && o.code.len() <= 40,
        "{}",
        o.code
    );
    assert_eq!(
        (
            o.nature.as_str(),
            o.regle.as_str(),
            o.droit_lien_code.as_deref()
        ),
        ("autorisation", "un", Some("autoriser_sorties"))
    );
    let (validite, preuve, fin_objet, publiee): (String, String, Option<chrono::DateTime<Utc>>, i64) = sqlx::query_as(
        "SELECT validite, preuve_min, evenement_fin, (SELECT count(*) FROM ent_acc_version v WHERE v.etablissement_id = o.etablissement_id AND v.objet_id = o.id AND v.publiee_le IS NOT NULL)
           FROM ent_acc_objet o WHERE etablissement_id = $1 AND code = $2",
    )
    .bind(etab)
    .bind(&o.code)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (validite.as_str(), preuve.as_str(), publiee),
        ("evenement", "S1", 1)
    );
    assert_eq!(fin_objet.map(|f| f.timestamp()), Some(fin.timestamp()));
    let (socle, type_s): (i16, String) = sqlx::query_as("SELECT source_socle, source_type FROM ent_acc_objet_source WHERE etablissement_id = $1 AND source_id = $2").bind(etab).bind(source).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((socle, type_s.as_str()), (42, "sortie"));
    // Idempotent : le même appel rend le même objet, sans doublon ; la fin suit l'événement reporté.
    let encore = ent
        .accords_creer_pour(etab, cast.prof.user_id, pour("sortie_facultative"))
        .await
        .unwrap();
    assert_eq!(encore.id, o.id);
    let report = fin + Duration::days(7);
    ent.accords_creer_pour(
        etab,
        cast.prof.user_id,
        ObjetPourSource {
            evenement_fin: Some(report),
            ..pour("sortie_facultative")
        },
    )
    .await
    .unwrap();
    let (fin_apres,): (Option<chrono::DateTime<Utc>>,) = sqlx::query_as(
        "SELECT evenement_fin FROM ent_acc_objet WHERE etablissement_id = $1 AND code = $2",
    )
    .bind(etab)
    .bind(&o.code)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fin_apres.map(|f| f.timestamp()), Some(report.timestamp()));
    let (n_objets,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_acc_objet_source WHERE etablissement_id = $1 AND source_id = $2",
    )
    .bind(etab)
    .bind(source)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n_objets, 1);
    // Une autre source : un autre objet. Un voyage à l'étranger exige `tous` (catalogue).
    let autre = ent
        .accords_creer_pour(
            etab,
            dir,
            ObjetPourSource {
                source_id: Uuid::new_v4(),
                source_type: "voyage".into(),
                ..pour("voyage_etranger")
            },
        )
        .await
        .unwrap();
    assert_ne!(autre.id, o.id);
    assert_eq!(autre.regle, "tous");
    // Utilisable tout de suite : décision de la mère, verdict autorisé ; après la fin de l'événement : expiré (CK-36).
    assert_eq!(
        verdict(&ent, etab, &o.code, "participation", harry).await,
        Verdict::EnAttente
    );
    ent.accords_decider(etab, lea, saisie(&o.code, harry, &["participation"], true))
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &o.code, "participation", harry).await,
        Verdict::Autorise
    );
    sqlx::query("UPDATE ent_acc_objet SET evenement_fin = now() - interval '1 hour' WHERE etablissement_id = $1 AND code = $2").bind(etab).bind(&o.code).execute(ent.pool_verif()).await.unwrap();
    assert_eq!(
        verdict(&ent, etab, &o.code, "participation", harry).await,
        Verdict::Expire
    );
    // La source est en ajout seul.
    assert!(
        sqlx::query("DELETE FROM ent_acc_objet_source WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Un objet de source se met dans un formulaire comme n'importe quel objet actif et publié.
    let q = ent
        .frm_creer(etab, dir, DIR, nf("Sortie", "mixte", "eleve_un_titulaire"))
        .await
        .unwrap();
    let sq = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            q.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    ent.frm_ajouter_question(
        etab,
        dir,
        DIR,
        q.id,
        NouvelleQuestion {
            objet_accord_code: Some(autre.code.clone()),
            ..nq(sq, "accord", "Autorisation", true)
        },
    )
    .await
    .unwrap();
    droit_sortie(&ent, etab, lea, false).await;
    vider(&ent, etab).await;
}

// ─── Dossier de preuve D6 ────────────────────────────────────────────────────────────────────────────

struct Cachet;

impl CachetEtablissement for Cachet {
    fn sceller(&self, _: Uuid, pdf: &[u8]) -> Result<Scelle, ErreurCachet> {
        let mut o = pdf.to_vec();
        o.extend_from_slice(b"\n%SCELLE-TEST\n");
        Ok(Scelle {
            octets: o,
            cle_ref: "cle-test-1".into(),
        })
    }
}

#[derive(Default)]
struct Sms {
    vus: Mutex<Vec<(String, String)>>,
}

impl Sms {
    fn dernier_code(&self) -> String {
        let vus = self.vus.lock().unwrap();
        let (_, texte) = vus.last().expect("un SMS a été envoyé");
        texte
            .split(|c: char| !c.is_ascii_digit())
            .find(|m| m.len() == 6)
            .expect("code à six chiffres")
            .to_string()
    }
}

impl EnvoiCodeSignature for Sms {
    fn envoyer<'a>(&'a self, t: &'a str, c: &'a str) -> FutureEnvoi<'a> {
        Box::pin(async move {
            self.vus
                .lock()
                .unwrap()
                .push((t.to_string(), c.to_string()));
            Ok(())
        })
    }
}

struct Piece {
    document_id: Uuid,
}

/// Un document réellement émis pour l'élève (pièce du dossier, ou texte à signer).
async fn pieces(ent: &PgEnt, cast: &LaboCast, eleve: Uuid) -> Piece {
    let c = Cachet;
    let m = Moteur {
        rendu: &RenduPdfAttestation,
        cachet: &c,
        url_verification: "https://test.scolia.fr/verifier",
    };
    let a = Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.administratif.user_id,
        role: "administratif",
    };
    // L'attestation d'inscription exige un élève inscrit : on l'inscrit si besoin (idempotent).
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_scolarite_annee WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(cast.etablissement_id)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    if n == 0 {
        sqlx::query(
            "INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, niveau, classe, statut, source)
             VALUES ($1, $2, $3, '2026-2027', '4e', '4B', 'courante', 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(cast.etablissement_id)
        .bind(eleve)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    let e = ent
        .emettre_attestation_inscription_for_actor(&a, eleve, &m)
        .await
        .expect("document émis");
    Piece {
        document_id: e.document_id,
    }
}

async fn preparer(ent: &PgEnt, sql: &str, binds: &[Uuid]) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut q = sqlx::query(sql);
    for b in binds {
        q = q.bind(*b);
    }
    q.execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

async fn mobile(ent: &PgEnt, etab: Uuid, personne: Uuid, tel: &str) {
    sqlx::query("UPDATE ent_personne_coordonnee SET au = now() WHERE etablissement_id = $1 AND personne_id = $2 AND au IS NULL AND type IN ('mobile', 'fixe', 'professionnel')")
        .bind(etab)
        .bind(personne)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_personne_coordonnee (id, etablissement_id, personne_id, type, valeur, principal, source) VALUES ($1, $2, $3, 'mobile', $4, true, 'saisie')")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(personne)
        .bind(tel)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

async fn signer_par_code(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    sms: &Sms,
    signataire: Uuid,
) -> Result<SignatureFaite, EntDbError> {
    let c = Cachet;
    let h = HorodatageAucun;
    let ctx = ContexteSignature {
        moteur: Moteur {
            rendu: &RenduPdfAttestation,
            cachet: &c,
            url_verification: "https://test.scolia.fr/verifier",
        },
        horodatage: &h,
    };
    let a = Acteur {
        etablissement_id: etab,
        user_id: user,
        role: "parent",
    };
    let d = ent
        .demander_code_signature_for_actor(&a, sms, signataire)
        .await?;
    ent.signer_code_for_actor(&a, &ctx, d.defi_id, signataire, &sms.dernier_code())
        .await
}

async fn signataire_de(ent: &PgEnt, etab: Uuid, circuit: Uuid, personne: Uuid) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_doc_signataire WHERE etablissement_id = $1 AND circuit_id = $2 AND personne_id = $3 AND etat <> 'remplace'")
        .bind(etab)
        .bind(circuit)
        .bind(personne)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn d6_dossier_de_preuve_d_une_decision_sans_simuler() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let dir = cast.direction.user_id;
    vider(&ent, etab).await;
    let lily: ActeurTest = fiche(&ent, etab, cast.parent_lea.user_id, "parent", false).await;
    let tom: ActeurTest = fiche(&ent, etab, cast.parent_tom.user_id, "parent", false).await;
    mobile(&ent, etab, lily.personne, "0612345678").await;
    mobile(&ent, etab, tom.personne, "0687654321").await;
    // Un élève neuf, deux titulaires (Lily et Tom), droit de sortie pour les deux.
    let eleve = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, classe, date_naissance, date_entree, created_at, updated_at) VALUES ($1, $2, 'Dupont', 'Laura', 'actif', '4B', $3, $4, now(), now())")
        .bind(eleve)
        .bind(etab)
        .bind(NaiveDate::from_ymd_opt(2012, 3, 10).unwrap())
        .bind(NaiveDate::from_ymd_opt(2020, 9, 1).unwrap())
        .execute(ent.pool_verif())
        .await
        .unwrap();
    for p in [&lily, &tom] {
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, peut_autoriser_sorties) VALUES ($1, $2, $3, $4, $5, 'parent', true, true)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(eleve)
            .bind(p.user)
            .bind(p.personne)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    let sms = Sms::default();
    let objet = ent
        .accords_creer_pour(
            etab,
            dir,
            ObjetPourSource {
                catalogue_code: "sortie_facultative".into(),
                source_socle: 42,
                source_type: "sortie".into(),
                source_id: Uuid::new_v4(),
                libelle: "Sortie au théâtre".into(),
                evenement_fin: Some(Utc::now() + Duration::days(30)),
                texte: None,
            },
        )
        .await
        .unwrap();
    let code = objet.code.clone();
    // Le texte à signer : un document émis (socle 7), de nature « autorisation parentale », rattaché à la version de l'accord.
    let doc = pieces(&ent, &cast, eleve).await;
    preparer(&ent, "UPDATE ent_doc_document SET nature_code = 'voyages.autorisation_parentale' WHERE etablissement_id = $1 AND id = $2", &[etab, doc.document_id]).await;
    let autre_doc = pieces(&ent, &cast, eleve).await;
    preparer(&ent, "UPDATE ent_doc_document SET nature_code = 'voyages.autorisation_parentale' WHERE etablissement_id = $1 AND id = $2", &[etab, autre_doc.document_id]).await;
    // Le rattachement est un droit de gestion : une famille ne rattache rien, un document d'un autre élève est refusé.
    assert!(matches!(
        ent.accords_lier_document(
            etab,
            cast.parent_lea.user_id,
            "parent",
            &code,
            eleve,
            doc.document_id
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    let autre_eleve = pieces(&ent, &cast, cast.eleve_tom_id).await;
    assert!(matches!(
        ent.accords_lier_document(etab, dir, DIR, &code, eleve, autre_eleve.document_id)
            .await,
        Err(EntDbError::Refus(RefusMetier::Invalide, _))
    ));
    ent.accords_lier_document(etab, dir, DIR, &code, eleve, doc.document_id)
        .await
        .unwrap();
    ent.accords_lier_document(etab, dir, DIR, &code, eleve, doc.document_id)
        .await
        .unwrap();
    let adm = Acteur {
        etablissement_id: etab,
        user_id: cast.administratif.user_id,
        role: "administratif",
    };
    let lancer = |d: Uuid| {
        ent.lancer_circuit_for_actor(
            &adm,
            DemandeCircuit {
                document_id: d,
                ordre: None,
                echeance: None,
                titulaires_de_l_eleve: true,
                signataires: vec![],
            },
        )
    };
    let circuit = lancer(doc.document_id).await.expect("circuit lancé");
    let circuit_autre = lancer(autre_doc.document_id)
        .await
        .expect("circuit sur un document non rattaché");
    let (s_lily, s_tom) = (
        signataire_de(&ent, etab, circuit.circuit_id, lily.personne).await,
        signataire_de(&ent, etab, circuit.circuit_id, tom.personne).await,
    );

    // Avant toute signature : une décision signée est refusée (signature inconnue), la voie sans preuve reste celle du niveau S1.
    assert!(matches!(
        ent.accords_decider_signe(
            etab,
            lily.user,
            saisie(&code, eleve, &["participation"], true),
            Uuid::new_v4()
        )
        .await,
        Err(EntDbError::Refus(RefusMetier::Invalide, _))
    ));
    // Lily signe par code à usage unique (D6) : la signature prouve **sa** décision, et seulement la sienne.
    let sig = signer_par_code(&ent, etab, lily.user, &sms, s_lily)
        .await
        .expect("signature");
    assert_eq!(sig.niveau, crate::documents_signature::Niveau::S1);
    // Tom ne peut pas se servir de la signature de Lily.
    let e = ent
        .accords_decider_signe(
            etab,
            tom.user,
            saisie(&code, eleve, &["participation"], true),
            sig.signature_id,
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::Refus(RefusMetier::Interdit, _))),
        "{e:?}"
    );
    // La signature d'un document non rattaché à cet accord ne prouve rien.
    let sig_autre_s = signataire_de(&ent, etab, circuit_autre.circuit_id, lily.personne).await;
    let sig_autre = signer_par_code(&ent, etab, lily.user, &sms, sig_autre_s)
        .await
        .expect("signature");
    let e = ent
        .accords_decider_signe(
            etab,
            lily.user,
            saisie(&code, eleve, &["participation"], true),
            sig_autre.signature_id,
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::Refus(RefusMetier::Invalide, _))),
        "{e:?}"
    );
    // Un compte qui n'est pas titulaire ne décide pas, signature ou non.
    assert!(ent
        .accords_decider_signe(
            etab,
            cast.compta.user_id,
            saisie(&code, eleve, &["participation"], true),
            sig.signature_id
        )
        .await
        .is_err());
    // La décision signée : canal « code », preuve rattachée, verdict autorisé (règle `un`).
    let m1 = ent
        .accords_decider_signe(
            etab,
            lily.user,
            saisie(&code, eleve, &["participation"], true),
            sig.signature_id,
        )
        .await
        .unwrap();
    assert_eq!(
        (m1.decision.as_str(), m1.qualite.as_str()),
        ("accorde", "titulaire")
    );
    assert_eq!(
        verdict(&ent, etab, &code, "participation", eleve).await,
        Verdict::Autorise
    );
    let (canal, preuve_id, niveau, methode): (String, Option<Uuid>, String, String) = sqlx::query_as(
        "SELECT x.canal, x.preuve_signature_id, p.niveau, p.methode FROM ent_acc_decision x JOIN ent_acc_preuve p ON p.etablissement_id = x.etablissement_id AND p.decision_id = x.id
          WHERE x.etablissement_id = $1 AND x.id = $2",
    )
    .bind(etab)
    .bind(m1.decision_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (canal.as_str(), preuve_id, niveau.as_str(), methode.as_str()),
        ("code", Some(sig.signature_id), "S1", "code_sms")
    );
    // Une signature ne prouve qu'une fois.
    let e = ent
        .accords_decider_signe(
            etab,
            lily.user,
            saisie(&code, eleve, &["participation"], false),
            sig.signature_id,
        )
        .await;
    assert!(
        e.is_ok(),
        "un refus ne demande aucune signature (il protège)"
    );
    // Dossier de preuve : direction et secrétariat seulement, lecture journalisée.
    for (u, r) in [
        (lily.user, "parent"),
        (cast.cpe.user_id, "cpe"),
        (cast.prof.user_id, "prof"),
    ] {
        assert!(
            matches!(
                ent.accords_dossier_preuve(etab, u, r, m1.decision_id).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
    }
    let dossier = ent
        .accords_dossier_preuve(
            etab,
            cast.administratif.user_id,
            "administratif",
            m1.decision_id,
        )
        .await
        .unwrap();
    assert_eq!(
        (
            dossier.decision.as_str(),
            dossier.canal.as_str(),
            dossier.objet_code.as_str()
        ),
        ("accorde", "code", code.as_str())
    );
    let p = dossier.preuve.as_ref().expect("la preuve D6");
    assert_eq!(
        (
            p.niveau.as_str(),
            p.methode.as_str(),
            p.circuit_etat.as_str()
        ),
        ("S1", "code_sms", "complet")
    );
    assert_eq!(p.circuit_id, circuit.circuit_id);
    assert_eq!(p.empreinte_signee.len(), 64);
    // **Honnêteté** (limites de D6) : l'horodatage n'est jamais présenté comme vérifié ; ici, il n'a pas été demandé.
    assert!(p.horodatage.is_none());
    assert!(
        dossier
            .mentions
            .iter()
            .any(|t| t.contains("Aucun horodatage")),
        "{:?}",
        dossier.mentions
    );
    assert!(dossier.mentions.iter().all(
        |t| !t.to_lowercase().contains("qualifié vérifié") && !t.contains("horodatage vérifié")
    ));
    sqlx::query("INSERT INTO ent_doc_horodatage (id, etablissement_id, circuit_id, empreinte_horodatee, mode, etat, jeton_hex, obtenu_le) VALUES ($1, $2, $3, $4, 'mutualise', 'recu_non_verifie', 'ab', now())")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(circuit.circuit_id)
        .bind("a".repeat(64))
        .execute(ent.pool_verif())
        .await
        .ok();
    let dossier = ent
        .accords_dossier_preuve(etab, dir, DIR, m1.decision_id)
        .await
        .unwrap();
    if let Some(h) = dossier
        .preuve
        .as_ref()
        .and_then(|p| p.horodatage.as_deref())
    {
        assert_eq!(h, "recu_non_verifie");
        assert!(
            dossier.mentions.iter().any(|t| t.contains("non vérifié")),
            "{:?}",
            dossier.mentions
        );
    }
    let (lectures,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_journal WHERE etablissement_id = $1 AND acte = 'dossier_preuve_lu'").bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(lectures >= 2);
    // Une décision sans signature D6 dit qu'elle n'a pas de preuve renforcée.
    let m2 = ent
        .accords_decider(
            etab,
            tom.user,
            saisie(&code, eleve, &["participation"], true),
        )
        .await
        .unwrap();
    let sans = ent
        .accords_dossier_preuve(etab, dir, DIR, m2.decision_id)
        .await
        .unwrap();
    assert!(
        sans.preuve.is_none()
            && sans.canal == "en_ligne"
            && sans
                .mentions
                .iter()
                .any(|t| t.contains("sans preuve renforcée"))
    );
    // S2 n'est **jamais simulé** : un objet de preuve S2 refuse la voie sans brique, et refuse une signature d'un niveau inférieur (code SMS = S1).
    let s2 = objet_aleatoire();
    let mut n = objet_propre(
        &s2,
        "autorisation",
        "un",
        Some("autoriser_sorties"),
        &["participation"],
        Some("sortie"),
    );
    n.preuve_min = "S2".into();
    objet_publie(&ent, etab, dir, n).await;
    let e = ent
        .accords_decider(
            etab,
            lily.user,
            saisie(&s2, eleve, &["participation"], true),
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::ConflitDossier(m)) if m.contains("S2")),
        "{e:?}"
    );
    ent.accords_lier_document(etab, dir, DIR, &s2, eleve, doc.document_id)
        .await
        .unwrap();
    let e = ent
        .accords_decider_signe(
            etab,
            lily.user,
            saisie(&s2, eleve, &["participation"], true),
            sig.signature_id,
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::Refus(RefusMetier::Invalide, _))),
        "S1 ne vaut pas S2 : {e:?}"
    );
    assert_eq!(
        verdict(&ent, etab, &s2, "participation", eleve).await,
        Verdict::EnAttente
    );
    // Un circuit **annulé** : la signature qui prouvait la décision ne compte plus (D6 : le document signé ne vaut plus). Nature à règle
    // « tous » : la signature de Lily laisse le circuit ouvert, on peut donc l'annuler ensuite.
    let doc3 = pieces(&ent, &cast, eleve).await;
    preparer(&ent, "UPDATE ent_doc_document SET nature_code = 'inscriptions.finalisation_dossier' WHERE etablissement_id = $1 AND id = $2", &[etab, doc3.document_id]).await;
    let objet3 = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &objet3,
            "autorisation",
            "un",
            Some("autoriser_sorties"),
            &["participation"],
            Some("sortie"),
        ),
    )
    .await;
    ent.accords_lier_document(etab, dir, DIR, &objet3, eleve, doc3.document_id)
        .await
        .unwrap();
    let c3 = lancer(doc3.document_id).await.expect("circuit");
    let s3 = signataire_de(&ent, etab, c3.circuit_id, lily.personne).await;
    let sg3 = signer_par_code(&ent, etab, lily.user, &sms, s3)
        .await
        .expect("signature");
    assert!(
        !sg3.circuit_complet,
        "règle « tous » : Tom n'a pas encore signé"
    );
    ent.accords_decider_signe(
        etab,
        lily.user,
        saisie(&objet3, eleve, &["participation"], true),
        sg3.signature_id,
    )
    .await
    .unwrap();
    assert_eq!(
        verdict(&ent, etab, &objet3, "participation", eleve).await,
        Verdict::Autorise
    );
    ent.annuler_circuit_for_actor(&adm, c3.circuit_id, "annulation de test")
        .await
        .unwrap();
    assert_eq!(
        verdict(&ent, etab, &objet3, "participation", eleve).await,
        Verdict::Inconnu,
        "le circuit est annulé : la décision n'est plus prouvée, jamais « autorisé » par défaut"
    );
    assert!(!verdict(&ent, etab, &objet3, "participation", eleve)
        .await
        .permet());
    // Un circuit complet, lui, ne s'annule plus (document signé immuable) : la preuve reste.
    assert!(ent
        .annuler_circuit_for_actor(&adm, circuit.circuit_id, "trop tard")
        .await
        .is_err());
    let _ = (s_tom, m);
    // La preuve est en ajout seul.
    assert!(
        sqlx::query("DELETE FROM ent_acc_preuve WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM ent_acc_document_lien WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Nettoyage : ces lignes sont immuables, la purge volontaire est le seul moyen.
    sqlx::query("DELETE FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(eleve)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    vider(&ent, etab).await;
}

// ─── Cloisonnement et immuabilité ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fo3_separation_et_immuabilite() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let harry = cast.eleve_lea_id;
    vider(&ent, etab).await;
    let sortie = objet_aleatoire();
    objet_publie(
        &ent,
        etab,
        dir,
        objet_propre(
            &sortie,
            "autorisation",
            "un",
            Some("autoriser_sorties"),
            &["participation"],
            Some("sortie"),
        ),
    )
    .await;
    ent.accords_declarer_desaccord(etab, dir, DIR, harry, "sortie")
        .await
        .unwrap();
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let (e2, d2) = (autre.etablissement_id, autre.direction.user_id);
    // L'autre établissement ne voit ni le désaccord, ni l'objet ; ses références croisées sont refusées.
    assert!(
        ent.accords_desaccords(e2, d2, DIR, harry)
            .await
            .unwrap()
            .is_empty(),
        "pas de fuite d'un établissement à l'autre"
    );
    assert!(
        matches!(
            ent.accords_declarer_desaccord(e2, d2, DIR, harry, "image")
                .await,
            Err(EntDbError::Domain(_))
        ),
        "élève d'un autre établissement : introuvable"
    );
    assert!(matches!(
        ent.accords_relever_regle(e2, d2, DIR, &sortie, "tous")
            .await,
        Err(EntDbError::Domain(_))
    ));
    assert!(matches!(
        ent.accords_dossier_preuve(e2, d2, DIR, Uuid::new_v4())
            .await,
        Err(EntDbError::Domain(_))
    ));
    assert!(matches!(
        ent.accords_creer_pour(
            e2,
            autre.parent_lea.user_id,
            ObjetPourSource {
                catalogue_code: "sortie_facultative".into(),
                source_socle: 42,
                source_type: "sortie".into(),
                source_id: Uuid::new_v4(),
                libelle: "X".into(),
                evenement_fin: None,
                texte: None
            }
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Un compte de l'établissement A n'est pas membre de B : `creer_pour` refuse.
    assert!(matches!(
        ent.accords_creer_pour(
            e2,
            cast.prof.user_id,
            ObjetPourSource {
                catalogue_code: "sortie_facultative".into(),
                source_socle: 42,
                source_type: "sortie".into(),
                source_id: Uuid::new_v4(),
                libelle: "X".into(),
                evenement_fin: None,
                texte: None
            }
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Une réponse à un formulaire de A depuis B : introuvable.
    let (pid, st) = crate::tests_formulaires_fo2::publie_oui_non(
        &ent,
        etab,
        dir,
        "eleve_un_titulaire",
        "Coupon A",
    )
    .await;
    let q = &st.sections[0].questions[0];
    let rep = ReponseSaisie {
        eleve_id: harry,
        valeurs: vec![ValeurSaisie {
            question_id: q.id,
            option_ids: vec![q.options[0].id],
            ..Default::default()
        }],
        accords: vec![],
    };
    assert!(matches!(
        ent.frm_repondre(e2, autre.parent_lea.user_id, pid, rep)
            .await,
        Err(EntDbError::Domain(_))
    ));
    // Immuabilité : une soumission et une cible ne se suppriment pas, une décision en ajout seul.
    assert!(
        sqlx::query("DELETE FROM ent_frm_cible WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_frm_cible SET eleve_id = $2 WHERE etablissement_id = $1")
            .bind(etab)
            .bind(Uuid::new_v4())
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    vider(&ent, etab).await;
}
