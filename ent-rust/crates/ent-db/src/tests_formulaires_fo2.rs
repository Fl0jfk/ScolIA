//! Socle 18, lot FO2 — création de formulaires par les personnels, contre la base locale de test
//! (`docs/socles/18_FORMULAIRES.md` §11, §21.2, §27, §31). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**. Avec
//! `ENT_RLS_MODE=strict`, le code testé tourne sous un rôle **non propriétaire, sans BYPASSRLS**.
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | droits `formulaires.creer` / `valider` / `gerer`, formulaire d'un autre introuvable, autre établissement | `fo2_droits_portee_et_cloisonnement` |
//! | audience dans la portée (professeur : ses classes), exclusions fondues, critères en lignes | `fo2_audience_portee_exclusions_et_apercu` |
//! | modèles prêts, types du §11.2, sensibilité (C3 refusée, C2 réservée), limites | `fo2_modeles_types_et_sensibilite` |
//! | conditions d'affichage sans boucle (code et base) | `fo2_conditions_sans_boucle_code_et_base` |
//! | publication, version figée par la base, nouvelle version, correction de forme tracée, cibles | `fo2_publication_versions_figees_et_correction` |
//! | validation à large audience, quatre yeux, refus | `fo2_validation_quatre_yeux_au_dela_du_seuil` |
//! | duplication, clôture, archivage, suppression d'un brouillon, co-auteurs | `fo2_cycle_de_vie_duplication_et_co_auteurs` |
//! | répondre : valeurs typées, unités, conditions, remplacement, clôture, répondant calculé | `fo2_repondre_valeurs_unites_et_conditions` |

use super::*;
use crate::frm_moteur::*;
use crate::tests_formulaires_fo0::{classe_4b, compte_eleve, labo, parent_en_plus, retirer_parent};
use uuid::Uuid;

const DIR: &str = "direction";

/// Vide ce que les lots FO2 et FO3 ont écrit (formulaires, réponses, désaccords, preuves) : tables en ajout seul, la purge
/// volontaire est le seul moyen. À appeler avant `vider_accords` de FO1 (clés étrangères vers les objets d'accord).
pub(crate) async fn vider_fo23(ent: &PgEnt, etab: Uuid) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.purge_etablissement', 'on', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    for t in [
        "ent_acc_preuve",
        "ent_frm_valeur",
        "ent_acc_decision_usage",
        "ent_acc_decision",
        "ent_frm_soumission",
        "ent_frm_cible",
        "ent_frm_condition",
        "ent_frm_option",
        "ent_frm_ligne",
        "ent_frm_correction",
        "ent_frm_question",
        "ent_frm_section",
        "ent_frm_audience_critere",
        "ent_frm_auteur",
        "ent_frm_version",
        "ent_frm_journal",
        "ent_frm_formulaire",
        "ent_acc_document_lien",
        "ent_acc_desaccord",
        "ent_acc_objet_source",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
}

pub(crate) fn nf(titre: &str, nature: &str, unite: &str) -> NouveauFormulaire {
    NouveauFormulaire {
        titre: titre.into(),
        description: None,
        nature: nature.into(),
        unite: unite.into(),
        confidentialite: None,
        destinataire_confidentiel_user_id: None,
        ouverture: None,
        echeance: None,
        relances: None,
    }
}

pub(crate) fn crit(mode: &str, t: &str, cible: Option<Uuid>) -> CritereAudience {
    CritereAudience {
        mode: mode.into(),
        type_critere: t.into(),
        cible_id: cible,
    }
}

pub(crate) fn nq(section: Uuid, t: &str, libelle: &str, obligatoire: bool) -> NouvelleQuestion {
    NouvelleQuestion {
        section_id: section,
        type_question: t.into(),
        libelle: libelle.into(),
        aide: None,
        obligatoire,
        sensibilite: None,
        min_nombre: None,
        max_nombre: None,
        longueur_max: None,
        options: vec![],
        lignes: vec![],
        objet_accord_code: None,
    }
}

pub(crate) async fn classe_par_code(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT c.id FROM ent_classe c JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.code = $2 AND c.archivee_le IS NULL ORDER BY y.courante DESC, y.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(code)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    id
}

/// Formulaire à une section et une question « oui / non » obligatoire, audience : la classe 4B, publié par la direction.
pub(crate) async fn publie_oui_non(
    ent: &PgEnt,
    etab: Uuid,
    dir: Uuid,
    unite: &str,
    titre: &str,
) -> (Uuid, StructureVue) {
    let f = ent
        .frm_creer(etab, dir, DIR, nf(titre, "collecte", unite))
        .await
        .unwrap();
    let s = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "Réponse".into(),
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
        nq(s, "oui_non", "Participez-vous ?", true),
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
    let b = ent.frm_publier(etab, dir, DIR, f.id, None).await.unwrap();
    assert_eq!(b.statut, "publie");
    let st = ent.frm_structure(etab, dir, DIR, f.id, None).await.unwrap();
    (f.id, st)
}

// ─── Droits, cloisonnement ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fo2_droits_portee_et_cloisonnement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    // Ni une famille, ni la comptabilité, ni l'infirmerie ne créent un formulaire.
    for (u, r) in [
        (cast.parent_lea.user_id, "parent"),
        (cast.compta.user_id, "compta"),
    ] {
        let e = ent
            .frm_creer(etab, u, r, nf("X", "collecte", "eleve_un_titulaire"))
            .await;
        assert!(matches!(e, Err(EntDbError::Forbidden)), "{r} : {e:?}");
        assert!(
            matches!(
                ent.frm_modeles(etab, u, r).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
    }
    // Direction, secrétariat, vie scolaire, professeur : oui.
    let f_dir = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Réunion de rentrée", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    assert_eq!(
        (
            f_dir.statut.as_str(),
            f_dir.confidentialite.as_str(),
            f_dir.version_publiee,
            f_dir.brouillon
        ),
        ("brouillon", "nominatif", None, true)
    );
    let f_adm = ent
        .frm_creer(
            etab,
            cast.administratif.user_id,
            "administratif",
            nf("Fiche", "collecte", "personne"),
        )
        .await
        .unwrap();
    let f_cpe = ent
        .frm_creer(
            etab,
            cast.cpe.user_id,
            "cpe",
            nf("Enquête", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    let f_prof = ent
        .frm_creer(
            etab,
            cast.prof.user_id,
            "prof",
            nf("Coupon 4B", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    // Un professeur ne publie pas sans audience, et ne voit que les siens.
    let mes_prof = ent
        .frm_lister(etab, cast.prof.user_id, "prof")
        .await
        .unwrap();
    assert_eq!(
        mes_prof.iter().map(|f| f.id).collect::<Vec<_>>(),
        vec![f_prof.id]
    );
    // Le formulaire d'un autre est **introuvable** (indiscernable d'un identifiant inconnu), jamais « interdit ».
    for (u, r) in [
        (cast.prof.user_id, "prof"),
        (cast.compta.user_id, "compta"),
        (cast.parent_lea.user_id, "parent"),
    ] {
        let e = ent.frm_lire(etab, u, r, f_dir.id).await;
        assert!(
            matches!(
                e,
                Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
            ),
            "{r} : {e:?}"
        );
    }
    assert!(matches!(
        ent.frm_ajouter_section(
            etab,
            cast.prof.user_id,
            "prof",
            f_dir.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None
            }
        )
        .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // `formulaires.gerer` (direction, secrétariat, vie scolaire) lit tous les formulaires de l'établissement, sans les modifier.
    assert!(ent
        .frm_lire(etab, cast.cpe.user_id, "cpe", f_dir.id)
        .await
        .is_ok());
    assert!(ent
        .frm_lire(etab, cast.administratif.user_id, "administratif", f_dir.id)
        .await
        .is_ok());
    assert!(matches!(
        ent.frm_ajouter_section(
            etab,
            cast.cpe.user_id,
            "cpe",
            f_dir.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None
            }
        )
        .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    let tous = ent.frm_lister(etab, dir, DIR).await.unwrap();
    for id in [f_dir.id, f_adm.id, f_cpe.id, f_prof.id] {
        assert!(tous.iter().any(|f| f.id == id));
    }
    // Un autre établissement ne voit rien (filtre applicatif et RLS).
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let e = ent
        .frm_lire(
            autre.etablissement_id,
            autre.direction.user_id,
            DIR,
            f_dir.id,
        )
        .await;
    assert!(
        matches!(
            e,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{e:?}"
    );
    assert!(ent
        .frm_lister(autre.etablissement_id, autre.direction.user_id, DIR)
        .await
        .unwrap()
        .iter()
        .all(|f| f.id != f_dir.id));
    // Le rôle est celui de la session de l'établissement (les routes le posent) : un destinataire confidentiel d'un autre établissement
    // n'est pas membre de celui-ci.
    let confidentiel = NouveauFormulaire {
        confidentialite: Some("confidentiel".into()),
        destinataire_confidentiel_user_id: Some(autre.cpe.user_id),
        ..nf("X", "collecte", "personne")
    };
    assert!(matches!(
        ent.frm_creer(etab, dir, DIR, confidentiel).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    vider_fo23(&ent, etab).await;
}

// ─── Audience ────────────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fo2_audience_portee_exclusions_et_apercu() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    let (c4b, c5a) = (
        classe_4b(&ent, etab).await,
        classe_par_code(&ent, etab, "5A").await,
    );
    let f = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Sondage de niveau", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    // Critères invalides : refus nets.
    for c in [
        vec![],
        vec![crit("exclure", "classe", Some(c4b))],
        vec![crit("inclure", "classe", None)],
        vec![crit("inclure", "etablissement", Some(c4b))],
        vec![crit("inclure", "classe", Some(Uuid::new_v4()))],
    ] {
        assert!(
            matches!(
                ent.frm_definir_audience(etab, dir, DIR, f.id, c).await,
                Err(EntDbError::SaisieInvalide(_))
            ),
            "critères invalides"
        );
    }
    // La direction : une classe, puis l'établissement entier, puis l'établissement moins une classe (exclus fondus).
    let a4b = ent
        .frm_definir_audience(
            etab,
            dir,
            DIR,
            f.id,
            vec![crit("inclure", "classe", Some(c4b))],
        )
        .await
        .unwrap();
    assert!(a4b.total >= 1 && a4b.exclus == 0 && !a4b.validation_requise);
    let tout = ent
        .frm_definir_audience(
            etab,
            dir,
            DIR,
            f.id,
            vec![crit("inclure", "etablissement", None)],
        )
        .await
        .unwrap();
    assert!(tout.total > a4b.total, "l'établissement dépasse une classe");
    let sans_4b = ent
        .frm_definir_audience(
            etab,
            dir,
            DIR,
            f.id,
            vec![
                crit("inclure", "etablissement", None),
                crit("exclure", "classe", Some(c4b)),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        sans_4b.total + sans_4b.exclus,
        tout.total,
        "les exclus se retranchent"
    );
    assert_eq!(sans_4b.exclus, a4b.total);
    assert_eq!(
        ent.frm_apercu_audience(etab, dir, DIR, f.id)
            .await
            .unwrap()
            .total,
        sans_4b.total,
        "l'aperçu relit les critères enregistrés"
    );
    // Le professeur : ses classes seulement. Établissement entier et classes hors portée sont refusés, jamais tronqués.
    let prof = cast.prof.user_id;
    let fp = ent
        .frm_creer(
            etab,
            prof,
            "prof",
            nf("Coupon 4B", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    for c in [
        vec![crit("inclure", "etablissement", None)],
        vec![crit("inclure", "classe", Some(c5a))],
    ] {
        let e = ent.frm_definir_audience(etab, prof, "prof", fp.id, c).await;
        assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    }
    let portee = ent
        .eleves_autorises(etab, prof, "prof", ent_auth::Action::FormulairesCreer)
        .await
        .unwrap();
    if portee
        .as_ref()
        .is_some_and(|p| p.contains(&cast.eleve_lea_id))
    {
        let ok = ent
            .frm_definir_audience(
                etab,
                prof,
                "prof",
                fp.id,
                vec![crit("inclure", "classe", Some(c4b))],
            )
            .await
            .unwrap();
        assert!(ok.total >= 1);
    }
    // Un élève nommé hors de la portée du professeur est refusé aussi.
    if let Some(hors) = cast
        .eleves
        .iter()
        .map(|e| e.id)
        .find(|id| portee.as_ref().is_some_and(|p| !p.contains(id)))
    {
        let e = ent
            .frm_definir_audience(
                etab,
                prof,
                "prof",
                fp.id,
                vec![crit("inclure", "eleve", Some(hors))],
            )
            .await;
        assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    }
    // Une audience s'écrit en lignes : aucun JSON, des critères typés.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_audience_critere WHERE etablissement_id = $1 AND formulaire_id = $2")
        .bind(etab)
        .bind(f.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 2);
    // Une audience publiée ne se redéfinit plus (cibles figées).
    let (pid, _) = publie_oui_non(&ent, etab, dir, "eleve_un_titulaire", "Publié").await;
    assert!(matches!(
        ent.frm_definir_audience(
            etab,
            dir,
            DIR,
            pid,
            vec![crit("inclure", "classe", Some(c4b))]
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    vider_fo23(&ent, etab).await;
}

// ─── Modèles, types, sensibilité ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fo2_modeles_types_et_sensibilite() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    // Cinq modèles prêts.
    let modeles = ent.frm_modeles(etab, dir, DIR).await.unwrap();
    assert_eq!(modeles.len(), 5);
    let coupon = ent
        .frm_creer_depuis_modele(
            etab,
            dir,
            DIR,
            "coupon_reunion",
            Some("Réunion de rentrée".into()),
        )
        .await
        .unwrap();
    assert_eq!(
        (
            coupon.titre.as_str(),
            coupon.modele_code.as_deref(),
            coupon.unite.as_str()
        ),
        (
            "Réunion de rentrée",
            Some("coupon_reunion"),
            "eleve_un_titulaire"
        )
    );
    let s = ent
        .frm_structure(etab, dir, DIR, coupon.id, None)
        .await
        .unwrap();
    assert_eq!(
        (
            s.sections.len(),
            s.sections[0].questions.len(),
            s.numero,
            s.publiee
        ),
        (1, 3, 1, false)
    );
    let oui_non = &s.sections[0].questions[0];
    assert_eq!(
        oui_non
            .options
            .iter()
            .map(|o| o.libelle.as_str())
            .collect::<Vec<_>>(),
        vec!["Oui", "Non"],
        "options système"
    );
    assert!(matches!(
        ent.frm_creer_depuis_modele(etab, dir, DIR, "inconnu", None)
            .await,
        Err(EntDbError::Domain(_))
    ));
    // Le modèle « sortie » exige l'objet d'accord activé et publié (FO3).
    assert!(matches!(
        ent.frm_creer_depuis_modele(etab, dir, DIR, "sortie", None)
            .await,
        Err(EntDbError::ConflitDossier(_)) | Err(EntDbError::Domain(_))
    ));
    // Types du §11.2 : chacun créé avec ses contrôles.
    let f = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Tous les types", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    let sec = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "Tout".into(),
                aide: Some("Aide".into()),
            },
        )
        .await
        .unwrap();
    let ajouter = |q: NouvelleQuestion| ent.frm_ajouter_question(etab, dir, DIR, f.id, q);
    ajouter(nq(sec, "texte_court", "Prénom", true))
        .await
        .unwrap();
    ajouter(nq(sec, "texte_long", "Remarques", false))
        .await
        .unwrap();
    ajouter(NouvelleQuestion {
        options: vec!["A".into(), "B".into(), "C".into()],
        ..nq(sec, "choix_unique", "Atelier", false)
    })
    .await
    .unwrap();
    ajouter(NouvelleQuestion {
        options: vec!["A".into(), "B".into()],
        ..nq(sec, "choix_multiple", "Jours", false)
    })
    .await
    .unwrap();
    ajouter(NouvelleQuestion {
        options: vec!["A".into(), "B".into()],
        ..nq(sec, "liste", "Liste", false)
    })
    .await
    .unwrap();
    ajouter(nq(sec, "oui_non", "Oui ou non", false))
        .await
        .unwrap();
    ajouter(NouvelleQuestion {
        min_nombre: Some(0.0),
        max_nombre: Some(10.0),
        ..nq(sec, "nombre", "Combien", false)
    })
    .await
    .unwrap();
    ajouter(nq(sec, "date", "Quand", false)).await.unwrap();
    ajouter(nq(sec, "heure", "À quelle heure", false))
        .await
        .unwrap();
    ajouter(NouvelleQuestion {
        min_nombre: Some(1.0),
        max_nombre: Some(5.0),
        ..nq(sec, "echelle", "Note", false)
    })
    .await
    .unwrap();
    ajouter(NouvelleQuestion {
        options: vec!["Bien".into(), "Moyen".into()],
        lignes: vec!["Maths".into(), "Sport".into()],
        ..nq(sec, "grille", "Matières", false)
    })
    .await
    .unwrap();
    ajouter(NouvelleQuestion {
        options: vec!["1".into(), "2".into(), "3".into()],
        ..nq(sec, "classement", "Classez", false)
    })
    .await
    .unwrap();
    ajouter(nq(sec, "prise_connaissance", "J'ai lu le règlement", false))
        .await
        .unwrap();
    ajouter(nq(sec, "information", "Un texte affiché", false))
        .await
        .unwrap();
    // Types pas encore disponibles et types inconnus : refus nets.
    for t in ["fichier", "creneau", "choix_tarifie", "inconnu"] {
        assert!(
            matches!(
                ajouter(nq(sec, t, "X", false)).await,
                Err(EntDbError::SaisieInvalide(_))
            ),
            "{t}"
        );
    }
    // Sensibilité (R-FC22) : la santé est refusée, la religion est réservée à la direction et se classe en C2.
    assert!(
        matches!(ajouter(nq(sec, "texte_court", "Votre enfant a-t-il une allergie ?", false)).await, Err(EntDbError::SaisieInvalide(m)) if m.contains("santé"))
    );
    assert!(
        matches!(ajouter(nq(sec, "texte_court", "Quelle est votre religion ?", false)).await, Err(EntDbError::SaisieInvalide(m)) if m.contains("C2"))
    );
    let c2 = NouvelleQuestion {
        sensibilite: Some("C2".into()),
        ..nq(sec, "texte_court", "Quelle est votre religion ?", false)
    };
    ajouter(c2.clone()).await.unwrap();
    let fa = ent
        .frm_creer(
            etab,
            cast.administratif.user_id,
            "administratif",
            nf("Secrétariat", "collecte", "personne"),
        )
        .await
        .unwrap();
    let sa = ent
        .frm_ajouter_section(
            etab,
            cast.administratif.user_id,
            "administratif",
            fa.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    let e = ent
        .frm_ajouter_question(
            etab,
            cast.administratif.user_id,
            "administratif",
            fa.id,
            NouvelleQuestion {
                section_id: sa,
                ..c2
            },
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::SaisieInvalide(m)) if m.contains("direction")),
        "{e:?}"
    );
    // La question « accord » demande un formulaire « mixte » et un objet d'accord (FO3).
    assert!(matches!(
        ajouter(NouvelleQuestion {
            objet_accord_code: Some("droit_image".into()),
            ..nq(sec, "accord", "Droit à l'image", false)
        })
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Confidentiel : un destinataire du personnel, jamais une famille ; anonyme : pas encore, aucune fausse promesse.
    let confidentiel = |d: Option<Uuid>| NouveauFormulaire {
        confidentialite: Some("confidentiel".into()),
        destinataire_confidentiel_user_id: d,
        ..nf("Confidentiel", "collecte", "eleve_un_titulaire")
    };
    assert!(matches!(
        ent.frm_creer(etab, dir, DIR, confidentiel(None)).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_creer(etab, dir, DIR, confidentiel(Some(cast.parent_lea.user_id)))
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let c = ent
        .frm_creer(etab, dir, DIR, confidentiel(Some(cast.cpe.user_id)))
        .await
        .unwrap();
    assert_eq!(c.confidentialite, "confidentiel");
    let anonyme = NouveauFormulaire {
        confidentialite: Some("anonyme".into()),
        ..nf("Anonyme", "collecte", "personne")
    };
    assert!(
        matches!(ent.frm_creer(etab, dir, DIR, anonyme).await, Err(EntDbError::SaisieInvalide(m)) if m.contains("anonym"))
    );
    // Autres refus : nature, unité, titre, dates, relances.
    for n in [
        nf("", "collecte", "personne"),
        nf("X", "autre", "personne"),
        nf("X", "collecte", "autre"),
        NouveauFormulaire {
            relances: Some("jamais".into()),
            ..nf("X", "collecte", "personne")
        },
        NouveauFormulaire {
            ouverture: Some(chrono::Utc::now()),
            echeance: Some(chrono::Utc::now() - chrono::Duration::days(1)),
            ..nf("X", "collecte", "personne")
        },
    ] {
        assert!(matches!(
            ent.frm_creer(etab, dir, DIR, n).await,
            Err(EntDbError::SaisieInvalide(_))
        ));
    }
    // Aucune colonne JSON dans le moteur (D-036).
    let (json,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns WHERE table_name LIKE 'ent_frm_%' AND data_type IN ('json', 'jsonb')",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(json, 0);
    vider_fo23(&ent, etab).await;
}

// ─── Conditions d'affichage sans boucle ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn fo2_conditions_sans_boucle_code_et_base() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    let f = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Conditions", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    let s1 = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "Un".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    let q1 = ent
        .frm_ajouter_question(
            etab,
            dir,
            DIR,
            f.id,
            nq(s1, "oui_non", "Venez-vous ?", true),
        )
        .await
        .unwrap();
    let q2 = ent
        .frm_ajouter_question(
            etab,
            dir,
            DIR,
            f.id,
            nq(s1, "texte_court", "Combien de personnes ?", false),
        )
        .await
        .unwrap();
    let s2 = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "Deux".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    let q3 = ent
        .frm_ajouter_question(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleQuestion {
                options: vec!["Voiture".into(), "Bus".into()],
                ..nq(s2, "choix_unique", "Transport", false)
            },
        )
        .await
        .unwrap();
    let q4 = ent
        .frm_ajouter_question(etab, dir, DIR, f.id, nq(s2, "texte_court", "Plaque", false))
        .await
        .unwrap();
    let st = ent.frm_structure(etab, dir, DIR, f.id, None).await.unwrap();
    let opt = |q: Uuid, i: usize| {
        st.sections
            .iter()
            .flat_map(|s| &s.questions)
            .find(|x| x.id == q)
            .unwrap()
            .options[i]
            .id
    };
    let (oui, voiture) = (opt(q1, 0), opt(q3, 0));
    // q2 dépend de q1 (avant) : permis. q4 dépend de q3 : permis.
    ent.frm_ajouter_condition(
        etab,
        dir,
        DIR,
        f.id,
        NouvelleCondition {
            question_id: q2,
            depend_de_question_id: q1,
            option_id: oui,
            operateur: "egal".into(),
        },
    )
    .await
    .unwrap();
    ent.frm_ajouter_condition(
        etab,
        dir,
        DIR,
        f.id,
        NouvelleCondition {
            question_id: q4,
            depend_de_question_id: q3,
            option_id: voiture,
            operateur: "egal".into(),
        },
    )
    .await
    .unwrap();
    // Doublon exact : la base refuse.
    assert!(ent
        .frm_ajouter_condition(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleCondition {
                question_id: q2,
                depend_de_question_id: q1,
                option_id: oui,
                operateur: "egal".into()
            }
        )
        .await
        .is_err());
    // Vers l'arrière (q1 dépendrait de q3 qui est après) et vers soi-même : refus nets, par le code.
    for (q, dep, o) in [(q1, q3, voiture), (q1, q1, oui), (q3, q4, oui)] {
        let e = ent
            .frm_ajouter_condition(
                etab,
                dir,
                DIR,
                f.id,
                NouvelleCondition {
                    question_id: q,
                    depend_de_question_id: dep,
                    option_id: o,
                    operateur: "egal".into(),
                },
            )
            .await;
        assert!(matches!(e, Err(EntDbError::SaisieInvalide(_))), "{e:?}");
    }
    // Une question texte ne fonde pas une condition ; l'option d'une autre question non plus ; opérateur inconnu.
    assert!(matches!(
        ent.frm_ajouter_condition(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleCondition {
                question_id: q4,
                depend_de_question_id: q2,
                option_id: oui,
                operateur: "egal".into()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_ajouter_condition(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleCondition {
                question_id: q4,
                depend_de_question_id: q1,
                option_id: voiture,
                operateur: "egal".into()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_ajouter_condition(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleCondition {
                question_id: q4,
                depend_de_question_id: q1,
                option_id: oui,
                operateur: "peut-etre".into()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // **La base le refuse aussi** (le code n'est pas la seule garde) : condition vers l'arrière, et boucle à la publication.
    let r = sqlx::query(
        "INSERT INTO ent_frm_condition (id, etablissement_id, question_id, depend_de_question_id, option_id, operateur) VALUES ($1,$2,$3,$4,$5,'egal')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(q1)
    .bind(q3)
    .bind(voiture)
    .execute(ent.pool_verif())
    .await;
    assert!(
        r.is_err(),
        "le déclencheur refuse une condition vers l'arrière"
    );
    // Retirer une question retire ses conditions (et celles qui en dépendent).
    ent.frm_retirer_question(etab, dir, DIR, f.id, q3)
        .await
        .unwrap();
    let st = ent.frm_structure(etab, dir, DIR, f.id, None).await.unwrap();
    assert!(st
        .sections
        .iter()
        .flat_map(|s| &s.questions)
        .all(|q| q.conditions.iter().all(|c| c.depend_de_question_id != q3)));
    // La condition d'une question est lue dans la structure.
    let q2v = st
        .sections
        .iter()
        .flat_map(|s| &s.questions)
        .find(|q| q.id == q2)
        .unwrap();
    assert_eq!(q2v.conditions.len(), 1);
    vider_fo23(&ent, etab).await;
}

// ─── Publication, versions figées, correction de forme ───────────────────────────────────────────────

#[tokio::test]
async fn fo2_publication_versions_figees_et_correction() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    let f = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Choix d'atelier", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    // Un formulaire vide, sans audience, ne se publie pas.
    assert!(matches!(
        ent.frm_publier(etab, dir, DIR, f.id, None).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let s = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "Atelier".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(
            ent.frm_publier(etab, dir, DIR, f.id, None).await,
            Err(EntDbError::SaisieInvalide(_))
        ),
        "section vide"
    );
    let q = ent
        .frm_ajouter_question(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleQuestion {
                options: vec!["Théâtre".into(), "Échecs".into()],
                ..nq(s, "choix_unique", "Atelier souhaité", true)
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(
            ent.frm_publier(etab, dir, DIR, f.id, None).await,
            Err(EntDbError::SaisieInvalide(_))
        ),
        "audience vide"
    );
    let c4b = classe_4b(&ent, etab).await;
    let apercu = ent
        .frm_definir_audience(
            etab,
            dir,
            DIR,
            f.id,
            vec![crit("inclure", "classe", Some(c4b))],
        )
        .await
        .unwrap();
    let b = ent.frm_publier(etab, dir, DIR, f.id, None).await.unwrap();
    assert_eq!(
        (b.statut.as_str(), b.version_numero, b.cibles),
        ("publie", 1, apercu.total)
    );
    let (cibles,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_frm_cible WHERE etablissement_id = $1 AND formulaire_id = $2",
    )
    .bind(etab)
    .bind(f.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(cibles as usize, apercu.total);
    let vue = ent.frm_lire(etab, dir, DIR, f.id).await.unwrap();
    assert_eq!(
        (
            vue.statut.as_str(),
            vue.version_publiee,
            vue.brouillon,
            vue.audience_nb
        ),
        ("publie", Some(1), false, Some(apercu.total as i32))
    );
    let (empreinte, publiee_par): (String, Uuid) = sqlx::query_as("SELECT empreinte, publiee_par FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2 AND numero = 1")
        .bind(etab)
        .bind(f.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!((empreinte.len(), publiee_par), (64, dir));
    // Plus aucune modification du brouillon : il n'y en a plus, et la base fige la version publiée.
    assert!(matches!(
        ent.frm_ajouter_section(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleSection {
                titre: "X".into(),
                aide: None
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    for sql in [
        "UPDATE ent_frm_question SET libelle = 'Autre' WHERE etablissement_id = $1 AND id = $2",
        "DELETE FROM ent_frm_question WHERE etablissement_id = $1 AND id = $2",
        "UPDATE ent_frm_section SET titre = 'Autre' WHERE etablissement_id = $1 AND id = $2",
    ] {
        let id = if sql.contains("section") { s } else { q };
        assert!(
            sqlx::query(sql)
                .bind(etab)
                .bind(id)
                .execute(ent.pool_verif())
                .await
                .is_err(),
            "{sql}"
        );
    }
    assert!(sqlx::query("UPDATE ent_frm_version SET type_changement = 'fond' WHERE etablissement_id = $1 AND formulaire_id = $2").bind(etab).bind(f.id).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query(
        "DELETE FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2"
    )
    .bind(etab)
    .bind(f.id)
    .execute(ent.pool_verif())
    .await
    .is_err());
    assert!(sqlx::query("UPDATE ent_frm_formulaire SET nature = 'mixte' WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(f.id).execute(ent.pool_verif()).await.is_err(), "la nature ne change plus");
    assert!(
        sqlx::query("DELETE FROM ent_frm_formulaire WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(f.id)
            .execute(ent.pool_verif())
            .await
            .is_err(),
        "seul un brouillon jamais publié se supprime"
    );
    // Les réglages après publication : prolonger oui, raccourcir non, titre non.
    let t0 = chrono::Utc::now() + chrono::Duration::days(10);
    ent.frm_regler(etab, dir, DIR, f.id, None, None, None, Some(t0), None)
        .await
        .unwrap();
    assert!(matches!(
        ent.frm_regler(
            etab,
            dir,
            DIR,
            f.id,
            None,
            None,
            None,
            Some(t0 - chrono::Duration::days(1)),
            None
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.frm_regler(
            etab,
            dir,
            DIR,
            f.id,
            Some("Autre titre".into()),
            None,
            None,
            None,
            None
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.frm_regler(
        etab,
        dir,
        DIR,
        f.id,
        None,
        None,
        None,
        Some(t0 + chrono::Duration::days(2)),
        Some("j3_j1".into()),
    )
    .await
    .unwrap();
    // Nouvelle version : un brouillon copié ; un seul à la fois ; les réponses restent liées à leur version.
    let v2 = ent
        .frm_nouvelle_version(etab, dir, DIR, f.id)
        .await
        .unwrap();
    assert_eq!(
        (
            v2.numero,
            v2.publiee,
            v2.sections[0].questions.len(),
            v2.sections[0].questions[0].options.len()
        ),
        (2, false, 1, 2)
    );
    assert_ne!(v2.sections[0].questions[0].id, q, "identifiants neufs");
    assert!(matches!(
        ent.frm_nouvelle_version(etab, dir, DIR, f.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Le brouillon se modifie ; une version « de forme » dont la structure a changé est refusée.
    ent.frm_ajouter_question(
        etab,
        dir,
        DIR,
        f.id,
        nq(v2.sections[0].id, "texte_court", "Pourquoi ?", false),
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.frm_publier(etab, dir, DIR, f.id, Some("forme")).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.frm_publier(etab, dir, DIR, f.id, Some("autre")).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let b2 = ent.frm_publier(etab, dir, DIR, f.id, None).await.unwrap();
    assert_eq!((b2.version_numero, b2.statut.as_str()), (2, "publie"));
    let (chg,): (Option<String>, ) = sqlx::query_as("SELECT type_changement FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2 AND numero = 2")
        .bind(etab)
        .bind(f.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        chg.as_deref(),
        Some("fond"),
        "sans qualification, `fond` par prudence"
    );
    // La version 1 reste lisible telle qu'elle a été publiée.
    let v1 = ent
        .frm_structure(etab, dir, DIR, f.id, Some(1))
        .await
        .unwrap();
    assert_eq!((v1.numero, v1.sections[0].questions.len()), (1, 1));
    // Version « de forme » : mêmes questions, libellés retouchés (copie puis changement de libellé du brouillon).
    let v3 = ent
        .frm_nouvelle_version(etab, dir, DIR, f.id)
        .await
        .unwrap();
    let qv3 = v3.sections[0].questions[0].id;
    sqlx::query("UPDATE ent_frm_question SET libelle = 'Atelier souhaité (précisé)' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(qv3)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let b3 = ent
        .frm_publier(etab, dir, DIR, f.id, Some("forme"))
        .await
        .unwrap();
    assert_eq!(b3.version_numero, 3);
    // Correction de forme d'un libellé publié : tracée, ancienne valeur conservée, santé toujours refusée.
    ent.frm_corriger_libelle(
        etab,
        dir,
        DIR,
        f.id,
        qv3,
        "Atelier souhaité (corrigé)",
        Some("Une seule réponse"),
    )
    .await
    .unwrap();
    let (ancien, nouveau): (String, String) = sqlx::query_as("SELECT ancien_libelle, nouveau_libelle FROM ent_frm_correction WHERE etablissement_id = $1 AND question_id = $2")
        .bind(etab)
        .bind(qv3)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        (ancien.as_str(), nouveau.as_str()),
        ("Atelier souhaité (précisé)", "Atelier souhaité (corrigé)")
    );
    assert!(matches!(
        ent.frm_corriger_libelle(etab, dir, DIR, f.id, qv3, "Une allergie ?", None)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_corriger_libelle(etab, dir, DIR, f.id, qv3, "", None)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // La correction passe par le service seul : la base la refuse sans l'autorisation de transaction.
    assert!(sqlx::query(
        "UPDATE ent_frm_question SET libelle = 'Autre' WHERE etablissement_id = $1 AND id = $2"
    )
    .bind(etab)
    .bind(qv3)
    .execute(ent.pool_verif())
    .await
    .is_err());
    // La table des corrections et le journal sont en ajout seul.
    assert!(
        sqlx::query("DELETE FROM ent_frm_correction WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_frm_journal SET acte = 'cree' WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Le journal dit qui a publié quoi (codes courts, jamais une réponse).
    let (publie,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_journal WHERE etablissement_id = $1 AND formulaire_id = $2 AND acte IN ('publie', 'version')")
        .bind(etab)
        .bind(f.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(publie, 3);
    vider_fo23(&ent, etab).await;
}

// ─── Validation d'un formulaire à large audience (quatre yeux) ───────────────────────────────────────

#[tokio::test]
async fn fo2_validation_quatre_yeux_au_dela_du_seuil() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    // Le seuil est abaissé pour le test (200 par défaut) : toute audience non vide demande une validation.
    crate::frm_service::forcer_seuil_test(Some(0));
    let adm = cast.administratif.user_id;
    let f = ent
        .frm_creer(
            etab,
            adm,
            "administratif",
            nf("Enquête large", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    let s = ent
        .frm_ajouter_section(
            etab,
            adm,
            "administratif",
            f.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    ent.frm_ajouter_question(
        etab,
        adm,
        "administratif",
        f.id,
        nq(s, "oui_non", "Êtes-vous d'accord ?", true),
    )
    .await
    .unwrap();
    let c4b = classe_4b(&ent, etab).await;
    let a = ent
        .frm_definir_audience(
            etab,
            adm,
            "administratif",
            f.id,
            vec![crit("inclure", "classe", Some(c4b))],
        )
        .await
        .unwrap();
    assert!(a.validation_requise);
    // L'auteur publie : le formulaire passe en « soumis », rien n'est publié, aucune cible.
    let b = ent
        .frm_publier(etab, adm, "administratif", f.id, None)
        .await
        .unwrap();
    assert_eq!((b.statut.as_str(), b.cibles), ("soumis", a.total));
    let (publiee, cibles): (Option<chrono::DateTime<chrono::Utc>>, i64) = sqlx::query_as(
        "SELECT (SELECT publiee_le FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2),
                (SELECT count(*) FROM ent_frm_cible WHERE etablissement_id = $1 AND formulaire_id = $2)",
    )
    .bind(etab)
    .bind(f.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(publiee.is_none() && cibles == 0);
    // Un soumis ne se modifie pas.
    assert!(matches!(
        ent.frm_ajouter_section(
            etab,
            adm,
            "administratif",
            f.id,
            NouvelleSection {
                titre: "X".into(),
                aide: None
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Valider : seulement `formulaires.valider` (direction), jamais l'auteur, jamais un co-auteur.
    for (u, r) in [
        (adm, "administratif"),
        (cast.cpe.user_id, "cpe"),
        (cast.prof.user_id, "prof"),
        (cast.parent_lea.user_id, "parent"),
    ] {
        assert!(
            matches!(
                ent.frm_valider(etab, u, r, f.id, None).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
    }
    // Refus motivé : retour en brouillon, motif tracé en code court.
    assert!(matches!(
        ent.frm_refuser_validation(etab, dir, DIR, f.id, "Texte libre invalide")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.frm_refuser_validation(etab, dir, DIR, f.id, "audience_trop_large")
        .await
        .unwrap();
    assert_eq!(
        ent.frm_lire(etab, adm, "administratif", f.id)
            .await
            .unwrap()
            .statut,
        "brouillon"
    );
    assert!(matches!(
        ent.frm_refuser_validation(etab, dir, DIR, f.id, "autre_motif")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // L'auteur resoumet, puis la direction valide : la première version est publiée, les cibles écrites, la validation tracée.
    ent.frm_publier(etab, adm, "administratif", f.id, None)
        .await
        .unwrap();
    let b = ent.frm_valider(etab, dir, DIR, f.id, None).await.unwrap();
    assert_eq!((b.statut.as_str(), b.version_numero), ("publie", 1));
    let (valide_par, publie_par, nb): (Option<Uuid>, Uuid, i64) = sqlx::query_as(
        "SELECT valide_par, publie_par, (SELECT count(*) FROM ent_frm_cible WHERE etablissement_id = $1 AND formulaire_id = $2)
           FROM ent_frm_formulaire WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(f.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (valide_par, publie_par, nb as usize),
        (Some(dir), dir, a.total)
    );
    // Une nouvelle version d'un formulaire à large audience attend aussi une validation (pas l'auteur seul).
    ent.frm_nouvelle_version(etab, adm, "administratif", f.id)
        .await
        .unwrap();
    let st = ent
        .frm_structure(etab, adm, "administratif", f.id, None)
        .await
        .unwrap();
    ent.frm_ajouter_question(
        etab,
        adm,
        "administratif",
        f.id,
        nq(st.sections[0].id, "texte_court", "Pourquoi ?", false),
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.frm_publier(etab, adm, "administratif", f.id, None)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let b2 = ent.frm_valider(etab, dir, DIR, f.id, None).await.unwrap();
    assert_eq!(b2.version_numero, 2);
    // Un formulaire de la direction n'est pas validé par la direction elle-même (quatre yeux).
    let fd = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Par la direction", "collecte", "eleve_un_titulaire"),
        )
        .await
        .unwrap();
    let sd = ent
        .frm_ajouter_section(
            etab,
            dir,
            DIR,
            fd.id,
            NouvelleSection {
                titre: "S".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    ent.frm_ajouter_question(etab, dir, DIR, fd.id, nq(sd, "oui_non", "Q ?", true))
        .await
        .unwrap();
    ent.frm_definir_audience(
        etab,
        dir,
        DIR,
        fd.id,
        vec![crit("inclure", "classe", Some(c4b))],
    )
    .await
    .unwrap();
    assert_eq!(
        ent.frm_publier(etab, dir, DIR, fd.id, None)
            .await
            .unwrap()
            .statut,
        "soumis"
    );
    let e = ent.frm_valider(etab, dir, DIR, fd.id, None).await;
    assert!(
        matches!(
            e,
            Err(EntDbError::Refus(crate::service::RefusMetier::Interdit, _))
        ),
        "{e:?}"
    );
    crate::frm_service::forcer_seuil_test(None);
    vider_fo23(&ent, etab).await;
}

// ─── Duplication, clôture, archivage, suppression, co-auteurs ────────────────────────────────────────

#[tokio::test]
async fn fo2_cycle_de_vie_duplication_et_co_auteurs() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_fo23(&ent, etab).await;
    let (pid, st) = publie_oui_non(&ent, etab, dir, "eleve_un_titulaire", "À dupliquer").await;
    // Dupliquer : un brouillon neuf, sans audience, avec la structure ; l'original ne bouge pas.
    let copie = ent.frm_dupliquer(etab, dir, DIR, pid).await.unwrap();
    assert_eq!(
        (
            copie.statut.as_str(),
            copie.titre.as_str(),
            copie.version_publiee,
            copie.audience_nb
        ),
        ("brouillon", "Copie de À dupliquer", None, None)
    );
    let sc = ent
        .frm_structure(etab, dir, DIR, copie.id, None)
        .await
        .unwrap();
    assert_eq!((sc.sections.len(), sc.sections[0].questions.len()), (1, 1));
    assert_ne!(
        sc.sections[0].questions[0].id,
        st.sections[0].questions[0].id
    );
    assert!(
        matches!(
            ent.frm_publier(etab, dir, DIR, copie.id, None).await,
            Err(EntDbError::SaisieInvalide(_))
        ),
        "l'audience se redéfinit"
    );
    // Co-auteur : un membre du personnel qui peut créer ; un parent ou la comptabilité non ; il modifie le brouillon.
    assert!(matches!(
        ent.frm_ajouter_co_auteur(etab, dir, DIR, copie.id, cast.parent_lea.user_id)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_ajouter_co_auteur(etab, dir, DIR, copie.id, cast.compta.user_id)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.frm_ajouter_co_auteur(etab, dir, DIR, copie.id, cast.cpe.user_id)
        .await
        .unwrap();
    let s = ent
        .frm_ajouter_section(
            etab,
            cast.cpe.user_id,
            "cpe",
            copie.id,
            NouvelleSection {
                titre: "Ajoutée par le CPE".into(),
                aide: None,
            },
        )
        .await
        .unwrap();
    ent.frm_ajouter_question(
        etab,
        cast.cpe.user_id,
        "cpe",
        copie.id,
        nq(s, "texte_long", "Commentaire", false),
    )
    .await
    .unwrap();
    // Seul l'auteur gère les co-auteurs.
    assert!(ent
        .frm_ajouter_co_auteur(
            etab,
            cast.cpe.user_id,
            "cpe",
            copie.id,
            cast.administratif.user_id
        )
        .await
        .is_err());
    ent.frm_retirer_co_auteur(etab, dir, DIR, copie.id, cast.cpe.user_id)
        .await
        .unwrap();
    assert!(ent
        .frm_ajouter_section(
            etab,
            cast.cpe.user_id,
            "cpe",
            copie.id,
            NouvelleSection {
                titre: "X".into(),
                aide: None
            }
        )
        .await
        .is_err());
    // Brouillon jamais publié : la seule suppression permise, par l'auteur seulement.
    assert!(ent
        .frm_supprimer_brouillon(etab, cast.cpe.user_id, "cpe", copie.id)
        .await
        .is_err());
    ent.frm_supprimer_brouillon(etab, dir, DIR, copie.id)
        .await
        .unwrap();
    assert!(ent.frm_lire(etab, dir, DIR, copie.id).await.is_err());
    // Un formulaire publié ne se supprime pas ; il se clôt puis s'archive.
    assert!(matches!(
        ent.frm_supprimer_brouillon(etab, dir, DIR, pid).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(
        matches!(
            ent.frm_archiver(etab, dir, DIR, pid).await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "clore avant d'archiver"
    );
    ent.frm_clore(etab, dir, DIR, pid).await.unwrap();
    assert_eq!(
        ent.frm_lire(etab, dir, DIR, pid).await.unwrap().statut,
        "clos"
    );
    assert!(matches!(
        ent.frm_clore(etab, dir, DIR, pid).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.frm_rouvrir(etab, dir, DIR, pid).await.unwrap();
    ent.frm_clore(etab, dir, DIR, pid).await.unwrap();
    ent.frm_archiver(etab, dir, DIR, pid).await.unwrap();
    assert_eq!(
        ent.frm_lire(etab, dir, DIR, pid).await.unwrap().statut,
        "archive"
    );
    assert!(
        matches!(
            ent.frm_rouvrir(etab, dir, DIR, pid).await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "un archivé reste archivé"
    );
    assert!(sqlx::query(
        "UPDATE ent_frm_formulaire SET statut = 'publie' WHERE etablissement_id = $1 AND id = $2"
    )
    .bind(etab)
    .bind(pid)
    .execute(ent.pool_verif())
    .await
    .is_err());
    // La suppression est tracée au journal, sans identifiant de formulaire.
    let (supprimes,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_journal WHERE etablissement_id = $1 AND acte = 'supprime' AND formulaire_id IS NULL")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(supprimes, 1);
    vider_fo23(&ent, etab).await;
}

// ─── Répondre ────────────────────────────────────────────────────────────────────────────────────────

fn oui_non_saisie(q: &QuestionVue, oui: bool) -> ValeurSaisie {
    ValeurSaisie {
        question_id: q.id,
        option_ids: vec![q.options[usize::from(!oui)].id],
        ..Default::default()
    }
}

#[tokio::test]
async fn fo2_repondre_valeurs_unites_et_conditions() {
    use crate::frm_reponse::ReponseSaisie;
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_fo23(&ent, etab).await;
    let (pid, st) = publie_oui_non(&ent, etab, dir, "eleve_un_titulaire", "Coupon").await;
    let q = &st.sections[0].questions[0];
    let rep = |e: Uuid, v: Vec<ValeurSaisie>| ReponseSaisie {
        eleve_id: e,
        valeurs: v,
        accords: vec![],
    };
    // « À remplir » : Léa voit le formulaire pour Harry ; Tom, parent d'un autre élève de 5A, ne le voit pas.
    let a_remplir = ent.frm_a_repondre(etab, lea).await.unwrap();
    assert!(a_remplir
        .iter()
        .any(|a| a.formulaire_id == pid && a.eleve_id == harry && !a.deja_repondu));
    assert!(ent
        .frm_a_repondre(etab, cast.parent_tom.user_id)
        .await
        .unwrap()
        .iter()
        .all(|a| a.formulaire_id != pid));
    // Un parent qui n'est pas celui de l'élève, ou un élève hors audience : introuvable (jamais « interdit »).
    let e = ent
        .frm_repondre(
            etab,
            cast.parent_tom.user_id,
            pid,
            rep(harry, vec![oui_non_saisie(q, true)]),
        )
        .await;
    assert!(
        matches!(
            e,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{e:?}"
    );
    let e = ent
        .frm_repondre(
            etab,
            cast.parent_tom.user_id,
            pid,
            rep(cast.eleve_tom_id, vec![oui_non_saisie(q, true)]),
        )
        .await;
    assert!(
        matches!(
            e,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{e:?}"
    );
    assert!(matches!(
        ent.frm_formulaire_pour_repondre(etab, cast.parent_tom.user_id, pid, harry)
            .await,
        Err(EntDbError::Domain(_))
    ));
    assert_eq!(
        ent.frm_formulaire_pour_repondre(etab, lea, pid, harry)
            .await
            .unwrap()
            .sections[0]
            .questions
            .len(),
        1
    );
    // Question obligatoire sans réponse, option étrangère, réponse en double : refus nets.
    assert!(matches!(
        ent.frm_repondre(etab, lea, pid, rep(harry, vec![])).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let mut mauvaise = oui_non_saisie(q, true);
    mauvaise.option_ids = vec![Uuid::new_v4()];
    assert!(matches!(
        ent.frm_repondre(etab, lea, pid, rep(harry, vec![mauvaise]))
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_repondre(
            etab,
            lea,
            pid,
            rep(
                harry,
                vec![oui_non_saisie(q, true), oui_non_saisie(q, false)]
            )
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Réponse valable : accusé de **sa** réponse ; rien n'est écrit par les refus précédents.
    let (nb0,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_soumission WHERE etablissement_id = $1 AND formulaire_id = $2").bind(etab).bind(pid).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(nb0, 0);
    let r1 = ent
        .frm_repondre(etab, lea, pid, rep(harry, vec![oui_non_saisie(q, true)]))
        .await
        .unwrap();
    assert!(!r1.remplace_une_reponse && r1.decisions.is_empty());
    assert!(ent
        .frm_a_repondre(etab, lea)
        .await
        .unwrap()
        .iter()
        .any(|a| a.formulaire_id == pid && a.deja_repondu));
    // Unité « un titulaire » : la première réponse vaut ; l'autre titulaire est refusé sans contenu.
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    let e = ent
        .frm_repondre(etab, pere, pid, rep(harry, vec![oui_non_saisie(q, false)]))
        .await;
    assert!(
        matches!(e, Err(EntDbError::ConflitDossier(m)) if !m.contains("Oui") && !m.contains("Non")),
        "{e:?}"
    );
    // Léa corrige sa réponse : l'ancienne est « remplacée » et garde sa date.
    let r2 = ent
        .frm_repondre(etab, lea, pid, rep(harry, vec![oui_non_saisie(q, false)]))
        .await
        .unwrap();
    assert!(r2.remplace_une_reponse);
    let statuts: Vec<(String,)> = sqlx::query_as("SELECT statut FROM ent_frm_soumission WHERE etablissement_id = $1 AND formulaire_id = $2 ORDER BY envoyee_le, id").bind(etab).bind(pid).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(
        statuts.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        vec!["remplacee", "envoyee"]
    );
    // Une soumission est en ajout seul : ni suppression, ni réécriture des valeurs.
    assert!(
        sqlx::query("DELETE FROM ent_frm_soumission WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_frm_valeur SET rang = 1 WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Élève mineur : un formulaire « un titulaire » n'est pas le sien ; le compte de l'élève est introuvable ici.
    let enfant = compte_eleve(&ent, etab, harry).await;
    assert!(matches!(
        ent.frm_repondre(etab, enfant, pid, rep(harry, vec![oui_non_saisie(q, true)]))
            .await,
        Err(EntDbError::Domain(_))
    ));
    retirer_parent(&ent, etab, pere).await;
    // Clôture : plus de réponse ; échéance passée : plus de réponse ; réouverture : de nouveau.
    ent.frm_clore(etab, dir, DIR, pid).await.unwrap();
    assert!(matches!(
        ent.frm_repondre(etab, lea, pid, rep(harry, vec![oui_non_saisie(q, true)]))
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.frm_rouvrir(etab, dir, DIR, pid).await.unwrap();
    sqlx::query("UPDATE ent_frm_formulaire SET echeance = now() - interval '1 hour' WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(pid).execute(ent.pool_verif()).await.unwrap();
    assert!(matches!(
        ent.frm_repondre(etab, lea, pid, rep(harry, vec![oui_non_saisie(q, true)]))
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(
        ent.frm_a_repondre(etab, lea)
            .await
            .unwrap()
            .iter()
            .all(|a| a.formulaire_id != pid),
        "échéance passée : plus à remplir"
    );

    // Valeurs typées et conditions : un formulaire à tous les types, la question 2 n'apparaît que si « oui ».
    let f = ent
        .frm_creer(
            etab,
            dir,
            DIR,
            nf("Typé", "collecte", "eleve_chaque_titulaire"),
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
    let q1 = ent
        .frm_ajouter_question(etab, dir, DIR, f.id, nq(s, "oui_non", "Venez-vous ?", true))
        .await
        .unwrap();
    let q2 = ent
        .frm_ajouter_question(
            etab,
            dir,
            DIR,
            f.id,
            NouvelleQuestion {
                min_nombre: Some(1.0),
                max_nombre: Some(5.0),
                ..nq(s, "echelle", "Note", true)
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
            options: vec!["A".into(), "B".into(), "C".into()],
            ..nq(s, "classement", "Classez", false)
        },
    )
    .await
    .unwrap();
    ent.frm_ajouter_question(etab, dir, DIR, f.id, nq(s, "date", "Quand", false))
        .await
        .unwrap();
    ent.frm_ajouter_question(etab, dir, DIR, f.id, nq(s, "texte_court", "Prénom", false))
        .await
        .unwrap();
    let st2 = ent.frm_structure(etab, dir, DIR, f.id, None).await.unwrap();
    let oui = st2.sections[0].questions[0].options[0].id;
    ent.frm_ajouter_condition(
        etab,
        dir,
        DIR,
        f.id,
        NouvelleCondition {
            question_id: q2,
            depend_de_question_id: q1,
            option_id: oui,
            operateur: "egal".into(),
        },
    )
    .await
    .unwrap();
    let c4b = classe_4b(&ent, etab).await;
    ent.frm_definir_audience(
        etab,
        dir,
        DIR,
        f.id,
        vec![crit("inclure", "classe", Some(c4b))],
    )
    .await
    .unwrap();
    ent.frm_publier(etab, dir, DIR, f.id, None).await.unwrap();
    let st2 = ent.frm_structure(etab, dir, DIR, f.id, None).await.unwrap();
    let qs = &st2.sections[0].questions;
    let (non, oui) = (qs[0].options[1].id, qs[0].options[0].id);
    let val = |i: usize, f: &dyn Fn(&mut ValeurSaisie)| {
        let mut v = ValeurSaisie {
            question_id: qs[i].id,
            ..Default::default()
        };
        f(&mut v);
        v
    };
    // « Non » : la question 2 (obligatoire) est masquée : pas exigée, et une réponse donnée est refusée.
    ent.frm_repondre(
        etab,
        lea,
        f.id,
        rep(harry, vec![val(0, &|v| v.option_ids = vec![non])]),
    )
    .await
    .unwrap();
    let e = ent
        .frm_repondre(
            etab,
            lea,
            f.id,
            rep(
                harry,
                vec![
                    val(0, &|v| v.option_ids = vec![non]),
                    val(1, &|v| v.nombre = Some(3.0)),
                ],
            ),
        )
        .await;
    assert!(
        matches!(e, Err(EntDbError::SaisieInvalide(_))),
        "réponse à une question masquée : {e:?}"
    );
    // « Oui » : la question 2 est exigée, entière et dans les bornes ; le reste est facultatif.
    assert!(matches!(
        ent.frm_repondre(
            etab,
            lea,
            f.id,
            rep(harry, vec![val(0, &|v| v.option_ids = vec![oui])])
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.frm_repondre(
            etab,
            lea,
            f.id,
            rep(
                harry,
                vec![
                    val(0, &|v| v.option_ids = vec![oui]),
                    val(1, &|v| v.nombre = Some(9.0))
                ]
            )
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let o3 = qs[2].options.iter().map(|o| o.id).collect::<Vec<_>>();
    let ok = ent
        .frm_repondre(
            etab,
            lea,
            f.id,
            rep(
                harry,
                vec![
                    val(0, &|v| v.option_ids = vec![oui]),
                    val(1, &|v| v.nombre = Some(4.0)),
                    val(2, &|v| v.classement = vec![o3[2], o3[0], o3[1]]),
                    val(3, &|v| {
                        v.date = chrono::NaiveDate::from_ymd_opt(2026, 12, 3)
                    }),
                    val(4, &|v| v.texte = Some("  Harry ".into())),
                ],
            ),
        )
        .await
        .unwrap();
    assert!(ok.remplace_une_reponse);
    // Les valeurs sont typées : une colonne par type, une seule renseignée (la base le vérifie aussi).
    let lignes: Vec<(Option<f64>, Option<String>, Option<chrono::NaiveDate>, Option<i16>)> = sqlx::query_as(
        "SELECT valeur_nombre::float8, valeur_texte, valeur_date, rang FROM ent_frm_valeur WHERE etablissement_id = $1 AND soumission_id = $2",
    )
    .bind(etab)
    .bind(ok.soumission_id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes.len(), 1 + 1 + 3 + 1 + 1);
    assert!(
        lignes.iter().any(|l| l.0 == Some(4.0))
            && lignes.iter().any(|l| l.1.as_deref() == Some("Harry"))
            && lignes.iter().any(|l| l.2.is_some())
    );
    assert!(sqlx::query("INSERT INTO ent_frm_valeur (id, etablissement_id, soumission_id, version_id, question_id, valeur_texte, valeur_nombre) SELECT $1, etablissement_id, soumission_id, version_id, question_id, 'x', 1 FROM ent_frm_valeur WHERE soumission_id = $2 LIMIT 1")
        .bind(Uuid::new_v4())
        .bind(ok.soumission_id)
        .execute(ent.pool_verif())
        .await
        .is_err(), "deux valeurs scalaires dans une ligne : refusé par la base");
    // `eleve_chaque_titulaire` : le second titulaire répond aussi, sa réponse n'efface pas celle de la première.
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    ent.frm_repondre(
        etab,
        pere,
        f.id,
        rep(harry, vec![val(0, &|v| v.option_ids = vec![non])]),
    )
    .await
    .unwrap();
    let (en_cours,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_frm_soumission WHERE etablissement_id = $1 AND formulaire_id = $2 AND statut = 'envoyee'").bind(etab).bind(f.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(en_cours, 2);
    retirer_parent(&ent, etab, pere).await;
    vider_fo23(&ent, etab).await;
}
