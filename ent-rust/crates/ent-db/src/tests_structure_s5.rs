//! Socle 5, lot S5 : matières, offre d'enseignements, règles de choix, enseignements suivis
//! (distincts des groupes), reprise des options historiques, port pour le socle 3, périmètre D-022.
//! Base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ignorés en local et **en échec
//! en CI**. La plupart des scénarios travaillent dans un **établissement neuf** (identifiant
//! aléatoire, années posées à la main) pour ne pas se marcher dessus.

use super::*;
use crate::ref_commun::Acteur;
use crate::seed::LaboPersona;
use crate::structure_enseignements::{Ajout, DecisionSaisie, Modification, Sortie};
use crate::structure_offre::offre::{self, ContexteChoix, Demande};
use crate::structure_offre::{
    IncompatibiliteSaisie, MatiereModif, MatiereSaisie, OffreModif, OffreSaisie, PrerequisSaisie,
    RegleSaisie,
};
use crate::tenant_tx::{RlsMode, TenantTx};
use crate::tests_d022_bis::{labo, monde};
use chrono::{Duration, NaiveDate, Utc};
use domain_eleve::{CreateEleve, DomainError};
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

fn aujourdhui() -> NaiveDate {
    Utc::now().date_naive()
}

fn est_forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn est_domaine<T>(r: &Result<T, EntDbError>, d: DomainError) -> bool {
    matches!(r, Err(EntDbError::Domain(e)) if *e == d)
}

struct Monde5 {
    etab: Uuid,
    annee: Uuid,
    annee_prec: Uuid,
}

/// Établissement neuf avec deux années : l'année courante et la précédente (non clôturée).
async fn etab_neuf(ent: &PgEnt) -> Monde5 {
    let etab = Uuid::new_v4();
    let t = aujourdhui();
    let (annee, annee_prec) = (Uuid::new_v4(), Uuid::new_v4());
    for (id, lib, d, f, courante) in [
        (
            annee_prec,
            "N-1",
            t - Duration::days(430),
            t - Duration::days(70),
            false,
        ),
        (
            annee,
            "N",
            t - Duration::days(60),
            t + Duration::days(200),
            true,
        ),
    ] {
        sqlx::query(
            "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(etab)
        .bind(lib)
        .bind(d)
        .bind(f)
        .bind(courante)
        .execute(ent.pool())
        .await
        .unwrap();
    }
    Monde5 {
        etab,
        annee,
        annee_prec,
    }
}

async fn eleve(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "S5".into(),
        ine: None,
        sexe: None,
        date_naissance: None,
        numero_badge: None,
        redoublant: false,
        classe: None,
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
    .id
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

/// Place l'élève dans une classe du niveau (affectation active).
async fn en_classe(ent: &PgEnt, m: &Monde5, eleve: Uuid, niveau: Uuid, code: &str) {
    let classe: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = $3",
    )
    .bind(m.etab)
    .bind(m.annee)
    .bind(code)
    .fetch_optional(ent.pool())
    .await
    .unwrap();
    let classe = match classe {
        Some(c) => c,
        None => {
            let c = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau_id)
                 VALUES ($1,$2,$3,$4,$4,$5)",
            )
            .bind(c)
            .bind(m.etab)
            .bind(m.annee)
            .bind(code)
            .bind(niveau)
            .execute(ent.pool())
            .await
            .unwrap();
            c
        }
    };
    sqlx::query(
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
         VALUES ($1,$2,$3,$4,$5,$6,'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(m.etab)
    .bind(m.annee)
    .bind(eleve)
    .bind(classe)
    .bind(aujourdhui() - Duration::days(50))
    .execute(ent.pool())
    .await
    .unwrap();
}

fn saisie_offre(annee: Uuid, code: &str, modalite: &str) -> OffreSaisie {
    OffreSaisie {
        annee_id: annee,
        code: code.into(),
        libelle: format!("Offre {code}"),
        matiere_id: None,
        modalite: modalite.into(),
        code_modalite_election: None,
        admission: None,
        hors_contrat: false,
        capacite: None,
        ouvert_aux_voeux: false,
        public_type: None,
        date_limite_choix: None,
        ordre: None,
        niveau_ids: vec![],
        mef_ids: vec![],
    }
}

async fn offre_creer(ent: &PgEnt, a: &Acteur<'_>, s: OffreSaisie) -> Uuid {
    ent.structure_offre_creer(a, s).await.unwrap().id
}

fn ajouts(ids: &[Uuid]) -> Modification {
    Modification {
        ajouter: ids
            .iter()
            .map(|o| Ajout {
                offre_id: *o,
                forcer_capacite: false,
            })
            .collect(),
        ..Modification::default()
    }
}

fn codes(r: &crate::structure_enseignements::ResultatModification) -> Vec<String> {
    let mut v: Vec<String> = r.violations.iter().map(|v| v.code.clone()).collect();
    v.sort();
    v
}

// ───────────────────────── Matières ─────────────────────────

#[tokio::test]
async fn matieres_codes_sous_matieres_et_archivage_garde() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let a = acteur_de(m.etab, &cast.direction);
    let saisie = |code: &str| MatiereSaisie {
        code: code.into(),
        libelle: format!("Matière {code}"),
        libelle_court: Some("Abr".into()),
        couleur: Some("#1A2b3C".into()),
        famille: Some("Langues".into()),
        parent_id: None,
        code_national: Some("020100".into()),
    };
    let fr = ent.structure_matiere_creer(&a, saisie("fr")).await.unwrap();
    assert_eq!(fr.code, "FR", "codes en majuscules");
    assert_eq!(fr.couleur.as_deref(), Some("#1A2b3C"));
    // Doublon sans tenir compte de la casse ; formes invalides.
    assert!(est_domaine(
        &ent.structure_matiere_creer(&a, saisie("FR")).await,
        DomainError::Conflit
    ));
    for mauvais in [
        MatiereSaisie {
            couleur: Some("rouge".into()),
            ..saisie("A1")
        },
        MatiereSaisie {
            code_national: Some("02 01".into()),
            ..saisie("A2")
        },
        MatiereSaisie {
            libelle_court: Some("x".repeat(21)),
            ..saisie("A3")
        },
        MatiereSaisie {
            code: "a b".into(),
            ..saisie("A4")
        },
    ] {
        assert!(est_domaine(
            &ent.structure_matiere_creer(&a, mauvais).await,
            DomainError::ValeurInvalide
        ));
    }
    // Sous-matière : un seul niveau.
    let oral = ent
        .structure_matiere_creer(
            &a,
            MatiereSaisie {
                parent_id: Some(fr.id),
                ..saisie("FR_ORAL")
            },
        )
        .await
        .unwrap();
    assert_eq!(oral.parent_id, Some(fr.id));
    let r = ent
        .structure_matiere_creer(
            &a,
            MatiereSaisie {
                parent_id: Some(oral.id),
                ..saisie("PETIT")
            },
        )
        .await;
    assert!(r.is_err(), "pas de sous-sous-matière");
    let r = ent
        .structure_matiere_modifier(
            &a,
            fr.id,
            MatiereModif {
                parent_id: Some(oral.id),
                ..Default::default()
            },
        )
        .await;
    assert!(r.is_err(), "un parent ne devient pas sous-matière");
    // Modification : champs vides effacés, code inchangé.
    let modif = ent
        .structure_matiere_modifier(
            &a,
            fr.id,
            MatiereModif {
                libelle: Some("Français".into()),
                famille: Some(String::new()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (
            modif.libelle.as_str(),
            modif.famille.clone(),
            modif.code.as_str()
        ),
        ("Français", None, "FR")
    );
    // Archivage refusé : sous-matière active, puis offre ouverte qui l'utilise.
    assert!(est_domaine(
        &ent.structure_matiere_archiver(&a, fr.id).await,
        DomainError::Conflit
    ));
    ent.structure_matiere_archiver(&a, oral.id).await.unwrap();
    let mut s = saisie_offre(m.annee, "FRANCAIS", "commun");
    s.matiere_id = Some(fr.id);
    let offre = offre_creer(&ent, &a, s).await;
    assert!(est_domaine(
        &ent.structure_matiere_archiver(&a, fr.id).await,
        DomainError::Conflit
    ));
    // Une offre ne se crée pas sur une matière archivée.
    let mut s = saisie_offre(m.annee, "ORAL", "commun");
    s.matiere_id = Some(oral.id);
    assert!(ent.structure_offre_creer(&a, s).await.is_err());
    // Fermée l'offre, l'archivage passe ; la matière archivée disparaît de la liste par défaut.
    ent.structure_offre_fermer(&a, offre, "plus proposée", None, true)
        .await
        .unwrap();
    ent.structure_matiere_archiver(&a, fr.id).await.unwrap();
    assert!(ent.structure_matieres(&a, false).await.unwrap().is_empty());
    assert_eq!(ent.structure_matieres(&a, true).await.unwrap().len(), 2);
    ent.structure_matiere_desarchiver(&a, fr.id).await.unwrap();
    // Journal : la création, la modification, les archivages.
    let objets: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet = 'matiere'",
    )
    .bind(m.etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(objets >= 6, "{objets}");
}

// ───────────────────────── Offre et règles de choix ─────────────────────────

#[tokio::test]
async fn regles_de_choix_incompatibilites_prerequis_capacite_et_cibles_appliquees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let a = acteur_de(m.etab, &cast.direction);
    let (n5, n4) = (
        niveau(&ent, m.etab, "5E").await,
        niveau(&ent, m.etab, "4E").await,
    );
    let ada = eleve(&ent, m.etab, "Ada").await;
    let bob = eleve(&ent, m.etab, "Bob").await;
    en_classe(&ent, &m, ada, n5, "5A").await;
    en_classe(&ent, &m, bob, n5, "5A").await;

    let ciblee = |code: &str, modalite: &str, niveaux: Vec<Uuid>| {
        let mut s = saisie_offre(m.annee, code, modalite);
        s.niveau_ids = niveaux;
        s
    };
    let esp = offre_creer(&ent, &a, ciblee("ESP", "langue_2", vec![n5])).await;
    let all = offre_creer(&ent, &a, ciblee("ALL", "langue_2", vec![n5])).await;
    let latin = offre_creer(&ent, &a, ciblee("LAT", "option_facultative", vec![n5])).await;
    let grec = offre_creer(&ent, &a, ciblee("GREC", "option_facultative", vec![n5])).await;
    let hors = offre_creer(
        &ent,
        &a,
        ciblee("SCIENCES4", "option_facultative", vec![n4]),
    )
    .await;
    let mut chin = ciblee("CHINOIS", "option_facultative", vec![]);
    chin.capacite = Some(1);
    let chin = offre_creer(&ent, &a, chin).await;
    let mut tard = ciblee("ATELIER", "option_facultative", vec![]);
    tard.date_limite_choix = Some(aujourdhui() - Duration::days(3));
    let tard = offre_creer(&ent, &a, tard).await;

    // Règle « LV2 : exactement 1 » ; incompatibilité latin / grec.
    let regle = ent
        .structure_regle_creer(
            &a,
            RegleSaisie {
                annee_id: m.annee,
                niveau_id: Some(n5),
                mef_id: None,
                libelle: "LV2 : exactement 1".into(),
                offre_ids: vec![esp, all],
                min: 1,
                max: Some(1),
                bloquant: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(regle.offre_ids.len(), 2);
    ent.structure_incompatibilite_declarer(
        &a,
        IncompatibiliteSaisie {
            offre_a: grec,
            offre_b: latin,
            motif: Some("même barrette".into()),
            bloquant: true,
        },
    )
    .await
    .unwrap();
    // Règle mal formée : min > nombre d'offres, offres d'ailleurs.
    let r = ent
        .structure_regle_creer(
            &a,
            RegleSaisie {
                annee_id: m.annee,
                niveau_id: None,
                mef_id: None,
                libelle: "x".into(),
                offre_ids: vec![esp],
                min: 2,
                max: None,
                bloquant: true,
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::ValeurInvalide));

    // Deux LV2 d'un coup : refusé, rien n'est écrit.
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[esp, all]))
        .await
        .unwrap();
    assert!(!r.ecrit);
    assert_eq!(codes(&r), vec!["regle_max".to_string()]);
    assert!(ent
        .structure_enseignements_eleve(&a, ada, Some(m.annee), true)
        .await
        .unwrap()
        .is_empty());
    // Aperçu : rien n'est écrit non plus, mais le résultat est le même qu'un vrai ajout.
    let mut ap = ajouts(&[esp]);
    ap.apercu = true;
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ap)
        .await
        .unwrap();
    assert!(!r.ecrit && r.apercu && r.violations.is_empty());
    assert!(ent
        .structure_enseignements_eleve(&a, ada, Some(m.annee), true)
        .await
        .unwrap()
        .is_empty());
    // Une LV2 : écrit, accepté tout de suite (offre libre).
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[esp]))
        .await
        .unwrap();
    assert!(r.ecrit);
    assert_eq!(r.enseignements.len(), 1);
    assert_eq!(r.enseignements[0].etat, "accepte");
    // Une deuxième LV2 : refusée ; changer de LV2 en un geste : accepté.
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[all]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["regle_max".to_string()]);
    let change = Modification {
        ajouter: vec![Ajout {
            offre_id: all,
            forcer_capacite: false,
        }],
        quitter: vec![Sortie {
            offre_id: esp,
            fin_nature: "changement_en_cours_annee".into(),
            motif: "choix de la famille".into(),
        }],
        ..Modification::default()
    };
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, change)
        .await
        .unwrap();
    assert!(r.ecrit, "{:?}", r.violations);
    // R-S16 : l'enseignement quitté garde sa nature et ses dates ; l'actif ne montre plus que l'allemand.
    let tous = ent
        .structure_enseignements_eleve(&a, ada, Some(m.annee), false)
        .await
        .unwrap();
    let q = tous.iter().find(|e| e.offre_id == esp).unwrap();
    assert_eq!(
        (q.etat.as_str(), q.fin_nature.as_deref()),
        ("quitte", Some("changement_en_cours_annee"))
    );
    assert!(q.quitte_le.is_some() && q.debut_le.is_some());
    assert_eq!(
        ent.structure_enseignements_eleve(&a, ada, Some(m.annee), true)
            .await
            .unwrap()
            .len(),
        1
    );
    // Historique en ajout seul : création puis quittance.
    let h = ent
        .structure_enseignement_historique(&a, q.id)
        .await
        .unwrap();
    assert_eq!(
        h.iter().map(|l| l.vers_etat.as_str()).collect::<Vec<_>>(),
        vec!["accepte", "quitte"]
    );
    // Un enseignement quitté ne bouge plus, ne se supprime pas.
    for sql in [
        "UPDATE ent_eleve_enseignement SET etat = 'accepte', fin_nature = NULL, quitte_le = NULL WHERE id = $1",
        "UPDATE ent_eleve_enseignement SET motif = 'autre' WHERE id = $1",
        "DELETE FROM ent_eleve_enseignement WHERE id = $1",
    ] {
        assert!(sqlx::query(sql).bind(q.id).execute(ent.pool()).await.is_err(), "{sql}");
    }
    assert!(
        sqlx::query("DELETE FROM ent_eleve_enseignement_histo WHERE enseignement_id = $1")
            .bind(q.id)
            .execute(ent.pool())
            .await
            .is_err()
    );
    // On peut rechoisir l'enseignement quitté : nouvelle ligne, l'historique reste.
    let r = ent
        .structure_enseignements_modifier(
            &a,
            ada,
            m.annee,
            Modification {
                ajouter: vec![Ajout {
                    offre_id: esp,
                    forcer_capacite: false,
                }],
                quitter: vec![Sortie {
                    offre_id: all,
                    fin_nature: "correction".into(),
                    motif: "erreur de saisie".into(),
                }],
                ..Modification::default()
            },
        )
        .await
        .unwrap();
    assert!(r.ecrit);
    assert_eq!(
        ent.structure_enseignements_eleve(&a, ada, Some(m.annee), false)
            .await
            .unwrap()
            .len(),
        3
    );

    // Incompatibilité latin / grec.
    assert!(
        ent.structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[latin]))
            .await
            .unwrap()
            .ecrit
    );
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[grec]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["incompatibilite".to_string()]);
    assert!(r.violations[0].message.contains("même barrette"));

    // Hors cible (offre de 4ᵉ pour un élève de 5ᵉ) ; offre inconnue (autre établissement).
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[hors]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["hors_cible".to_string()]);
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[Uuid::new_v4()]))
        .await;
    assert!(r.is_err() || !r.unwrap().ecrit);

    // Capacité : un seul chinois ; le deuxième est refusé, sauf dépassement explicite (tracé).
    assert!(
        ent.structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[chin]))
            .await
            .unwrap()
            .ecrit
    );
    let r = ent
        .structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[chin]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["capacite_atteinte".to_string()]);
    let force = Modification {
        ajouter: vec![Ajout {
            offre_id: chin,
            forcer_capacite: true,
        }],
        ..Modification::default()
    };
    let r = ent
        .structure_enseignements_modifier(&a, bob, m.annee, force)
        .await
        .unwrap();
    assert!(
        r.ecrit && codes(&r) == vec!["capacite_atteinte".to_string()],
        "{:?}",
        r.violations
    );
    let force_trace: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet = 'enseignements_suivis'
           AND apres::text LIKE '%\"capacite_forcee\": true%'",
    )
    .bind(m.etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(force_trace, 1, "dépassement de capacité tracé");
    let o = ent.structure_offre(&a, chin).await.unwrap();
    assert_eq!((o.nb_suivis, o.capacite), (2, Some(1)));

    // Date limite dépassée : avertissement, pas un refus.
    let r = ent
        .structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[tard]))
        .await
        .unwrap();
    assert!(r.ecrit && codes(&r) == vec!["date_limite_depassee".to_string()]);

    // Minimum : bob n'a aucune LV2 ; sans `exiger_complet` ce n'est qu'un avertissement, avec c'est bloquant.
    let r = ent
        .structure_enseignements_modifier(
            &a,
            bob,
            m.annee,
            Modification {
                exiger_complet: true,
                ..Modification::default()
            },
        )
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["regle_min".to_string()]);
    let c = ent
        .structure_enseignements_controle_regles(&a, m.annee, bob)
        .await
        .unwrap();
    assert!(c.iter().any(|v| v.code == "regle_min" && v.bloquant));
    assert!(ent
        .structure_enseignements_controle_regles(&a, m.annee, ada)
        .await
        .unwrap()
        .is_empty());

    // Offre fermée : refusée au choix ; une règle archivée ne s'applique plus.
    ent.structure_offre_fermer(&a, grec, "pas d'effectif", None, true)
        .await
        .unwrap();
    let r = ent
        .structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[grec]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r).contains(&"offre_fermee".to_string()));
    ent.structure_regle_archiver(&a, regle.id).await.unwrap();
    assert!(
        ent.structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[esp, all]))
            .await
            .unwrap()
            .ecrit
    );
    assert_eq!(
        ent.structure_regles_choix(&a, m.annee).await.unwrap().len(),
        1
    );
    // Ajouter deux fois la même offre, quitter ce qu'on ne suit pas : refus propres.
    assert!(est_domaine(
        &ent.structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[esp]))
            .await,
        DomainError::Conflit
    ));
    let r = ent
        .structure_enseignements_modifier(
            &a,
            bob,
            m.annee,
            Modification {
                quitter: vec![Sortie {
                    offre_id: latin,
                    fin_nature: "correction".into(),
                    motif: "x".into(),
                }],
                ..Modification::default()
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::NotFound));
    // `fermeture_offre` et `depart` ne se posent pas à la main… `depart` oui, `fermeture_offre` non.
    let r = ent
        .structure_enseignements_modifier(
            &a,
            bob,
            m.annee,
            Modification {
                quitter: vec![Sortie {
                    offre_id: esp,
                    fin_nature: "fermeture_offre".into(),
                    motif: "x".into(),
                }],
                ..Modification::default()
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::ValeurInvalide));
}

#[tokio::test]
async fn prerequis_et_admission_sur_avis_avec_historique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let a = acteur_de(m.etab, &cast.administratif);
    let sx = sfx();
    let latin_m = ent
        .structure_matiere_creer(
            &a,
            MatiereSaisie {
                code: format!("LAT{sx}"),
                libelle: "Latin".into(),
                libelle_court: None,
                couleur: None,
                famille: None,
                parent_id: None,
                code_national: None,
            },
        )
        .await
        .unwrap()
        .id;
    let n5 = niveau(&ent, m.etab, "5E").await;
    let ada = eleve(&ent, m.etab, "Ada").await;
    let bob = eleve(&ent, m.etab, "Bob").await;
    for e in [ada, bob] {
        en_classe(&ent, &m, e, n5, "5A").await;
    }
    // Latin l'an dernier (année précédente), latin approfondi cette année : prérequis de matière.
    let mut s = saisie_offre(m.annee_prec, "LATIN_N1", "option_facultative");
    s.matiere_id = Some(latin_m);
    let latin_prec = offre_creer(&ent, &a, s).await;
    let mut s = saisie_offre(m.annee, "LATIN_APPRO", "option_facultative");
    s.matiere_id = Some(latin_m);
    s.admission = Some("sur_avis".into());
    let appro = offre_creer(&ent, &a, s).await;
    let section = offre_creer(
        &ent,
        &a,
        saisie_offre(m.annee, "SECTION", "option_facultative"),
    )
    .await;
    let lv3 = offre_creer(&ent, &a, saisie_offre(m.annee, "LV3", "langue_3")).await;
    let lv2 = offre_creer(&ent, &a, saisie_offre(m.annee, "LV2", "langue_2")).await;
    ent.structure_prerequis_ajouter(
        &a,
        appro,
        PrerequisSaisie {
            r#type: "matiere_annee_precedente".into(),
            prerequis_offre_id: None,
            prerequis_matiere_id: Some(latin_m),
            bloquant: true,
        },
    )
    .await
    .unwrap();
    let o = ent
        .structure_prerequis_ajouter(
            &a,
            lv3,
            PrerequisSaisie {
                r#type: "offre_meme_annee".into(),
                prerequis_offre_id: Some(lv2),
                prerequis_matiere_id: None,
                bloquant: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(o.prerequis.len(), 1);
    // Forme invalide ou prérequis de soi-même.
    for p in [
        PrerequisSaisie {
            r#type: "offre_meme_annee".into(),
            prerequis_offre_id: Some(lv3),
            prerequis_matiere_id: None,
            bloquant: true,
        },
        PrerequisSaisie {
            r#type: "matiere_annee_precedente".into(),
            prerequis_offre_id: Some(lv2),
            prerequis_matiere_id: None,
            bloquant: true,
        },
        PrerequisSaisie {
            r#type: "autre".into(),
            prerequis_offre_id: None,
            prerequis_matiere_id: None,
            bloquant: true,
        },
    ] {
        assert!(ent.structure_prerequis_ajouter(&a, lv3, p).await.is_err());
    }

    // Prérequis d'offre : LV3 sans LV2 refusé, avec LV2 dans le même geste accepté.
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[lv3]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["prerequis_manquant".to_string()]);
    assert!(
        ent.structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[lv3, lv2]))
            .await
            .unwrap()
            .ecrit
    );
    // Prérequis de matière : Ada n'a pas fait de latin l'an dernier, Bob oui.
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[appro]))
        .await
        .unwrap();
    assert!(!r.ecrit && codes(&r) == vec!["prerequis_manquant".to_string()]);
    assert!(
        ent.structure_enseignements_modifier(&a, bob, m.annee_prec, ajouts(&[latin_prec]))
            .await
            .unwrap()
            .ecrit
    );
    let r = ent
        .structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[appro]))
        .await
        .unwrap();
    assert!(r.ecrit, "{:?}", r.violations);
    // Admission sur avis : « choisi », pas encore accepté ; ne compte pas dans les places.
    let bob_appro = r
        .enseignements
        .iter()
        .find(|e| e.offre_id == appro)
        .unwrap();
    assert_eq!(bob_appro.etat, "choisi");
    assert_eq!(
        ent.structure_offre(&a, appro).await.unwrap().nb_en_attente,
        1
    );
    // Accepter sans avis favorable : refusé par la base (jamais d'acceptation sans avis).
    let r = ent
        .structure_enseignement_decider(
            &a,
            bob_appro.id,
            DecisionSaisie {
                decision: "accepter".into(),
                avis: None,
                effet: None,
                motif: None,
                forcer_capacite: false,
            },
        )
        .await;
    assert!(r.is_err());
    let r = ent
        .structure_enseignement_decider(
            &a,
            bob_appro.id,
            DecisionSaisie {
                decision: "accepter".into(),
                avis: Some("defavorable".into()),
                effet: None,
                motif: None,
                forcer_capacite: false,
            },
        )
        .await;
    assert!(r.is_err());
    let acc = ent
        .structure_enseignement_decider(
            &a,
            bob_appro.id,
            DecisionSaisie {
                decision: "accepter".into(),
                avis: Some("favorable".into()),
                effet: None,
                motif: None,
                forcer_capacite: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (acc.etat.as_str(), acc.avis.as_deref()),
        ("accepte", Some("favorable"))
    );
    assert!(acc.debut_le.is_some());
    // Une décision se prend une fois.
    let r = ent
        .structure_enseignement_decider(
            &a,
            bob_appro.id,
            DecisionSaisie {
                decision: "refuser".into(),
                avis: None,
                effet: None,
                motif: None,
                forcer_capacite: false,
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::Conflit));
    // Refus d'un autre candidat : terminal, motif conservé, historique complet.
    let cleo = eleve(&ent, m.etab, "Cléo").await;
    en_classe(&ent, &m, cleo, n5, "5A").await;
    assert!(
        ent.structure_enseignements_modifier(&a, cleo, m.annee_prec, ajouts(&[latin_prec]))
            .await
            .unwrap()
            .ecrit
    );
    let r = ent
        .structure_enseignements_modifier(&a, cleo, m.annee, ajouts(&[appro]))
        .await
        .unwrap();
    let id_cleo = r.enseignements[0].id;
    let refus = ent
        .structure_enseignement_decider(
            &a,
            id_cleo,
            DecisionSaisie {
                decision: "refuser".into(),
                avis: Some("defavorable".into()),
                effet: None,
                motif: Some("niveau insuffisant".into()),
                forcer_capacite: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(refus.etat, "refuse");
    let h = ent
        .structure_enseignement_historique(&a, id_cleo)
        .await
        .unwrap();
    assert_eq!(
        h.iter().map(|l| l.vers_etat.as_str()).collect::<Vec<_>>(),
        vec!["choisi", "refuse"]
    );
    // Une offre libre ne se « refuse » pas : la base le garantit.
    let r = ent
        .structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[section]))
        .await
        .unwrap();
    assert_eq!(
        r.enseignements
            .iter()
            .find(|e| e.offre_id == section)
            .unwrap()
            .etat,
        "accepte"
    );
    let r = sqlx::query("UPDATE ent_eleve_enseignement SET etat = 'refuse' WHERE offre_id = $1")
        .bind(section)
        .execute(ent.pool())
        .await;
    assert!(r.is_err());
    // Prérequis retiré : LV3 n'impose plus rien.
    let pid = ent.structure_offre(&a, lv3).await.unwrap().prerequis[0].id;
    ent.structure_prerequis_retirer(&a, pid).await.unwrap();
    assert!(est_domaine(
        &ent.structure_prerequis_retirer(&a, pid).await,
        DomainError::NotFound
    ));
}

// ───────────────────────── Fermeture d'une offre (STR-43) ─────────────────────────

#[tokio::test]
async fn fermeture_d_offre_apercu_puis_effets_tracees_direction_seule() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let dir = acteur_de(m.etab, &cast.direction);
    let adm = acteur_de(m.etab, &cast.administratif);
    let libre = offre_creer(
        &ent,
        &dir,
        saisie_offre(m.annee, "THEATRE", "option_facultative"),
    )
    .await;
    let mut s = saisie_offre(m.annee, "EURO", "option_facultative");
    s.admission = Some("sur_dossier".into());
    let euro = offre_creer(&ent, &dir, s).await;
    let (ada, bob) = (
        eleve(&ent, m.etab, "Ada").await,
        eleve(&ent, m.etab, "Bob").await,
    );
    ent.structure_enseignements_modifier(&adm, ada, m.annee, ajouts(&[libre]))
        .await
        .unwrap();
    ent.structure_enseignements_modifier(&adm, bob, m.annee, ajouts(&[libre, euro]))
        .await
        .unwrap();
    // Aperçu (sans `confirmer`) : liste les élèves, n'écrit rien.
    let ap = ent
        .structure_offre_fermer(&dir, libre, "pas d'effectif", None, false)
        .await
        .unwrap();
    assert_eq!((ap.nb_acceptes, ap.nb_en_attente, ap.ferme), (2, 0, false));
    assert_eq!(ap.eleve_ids.len(), 2);
    assert!(ent
        .structure_offre(&dir, libre)
        .await
        .unwrap()
        .ferme_le
        .is_none());
    // L'administratif gère l'offre mais ne la ferme pas (direction / administrateur seulement).
    assert!(est_forbidden(
        &ent.structure_offre_fermer(&adm, libre, "x", None, true)
            .await
    ));
    // Le motif est obligatoire.
    assert!(ent
        .structure_offre_fermer(&dir, libre, "  ", None, true)
        .await
        .is_err());
    let r = ent
        .structure_offre_fermer(&dir, libre, "pas d'effectif", None, true)
        .await
        .unwrap();
    assert!(r.ferme);
    for e in [ada, bob] {
        let v = ent
            .structure_enseignements_eleve(&dir, e, Some(m.annee), false)
            .await
            .unwrap();
        let t = v.iter().find(|x| x.offre_id == libre).unwrap();
        assert_eq!(
            (t.etat.as_str(), t.fin_nature.as_deref()),
            ("quitte", Some("fermeture_offre"))
        );
        assert!(t.quitte_le.is_some());
    }
    // Choix en attente : retiré à la fermeture de l'offre, jamais « quitté ».
    ent.structure_offre_fermer(&dir, euro, "plus de dossier", None, true)
        .await
        .unwrap();
    let v = ent
        .structure_enseignements_eleve(&dir, bob, Some(m.annee), false)
        .await
        .unwrap();
    assert_eq!(
        v.iter().find(|x| x.offre_id == euro).unwrap().etat,
        "retire"
    );
    // Déjà fermée : conflit ; plus de modification ; rouverte, elle se modifie à nouveau.
    assert!(est_domaine(
        &ent.structure_offre_fermer(&dir, libre, "x", None, true)
            .await,
        DomainError::Conflit
    ));
    assert!(ent
        .structure_offre_modifier(
            &dir,
            libre,
            OffreModif {
                libelle: Some("Y".into()),
                ..Default::default()
            }
        )
        .await
        .is_err());
    ent.structure_offre_rouvrir(&dir, libre).await.unwrap();
    let o = ent
        .structure_offre_modifier(
            &dir,
            libre,
            OffreModif {
                libelle: Some("Théâtre".into()),
                capacite: Some(20),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!((o.libelle.as_str(), o.capacite), ("Théâtre", Some(20)));
    // L'événement de fermeture ne porte aucune identité d'élève.
    let payloads: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'referentiel.offre_fermee'",
    )
    .bind(m.etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(payloads.len(), 2);
    for (p,) in payloads {
        let s = p.to_string();
        assert!(
            !s.contains(&ada.to_string()) && !s.contains(&bob.to_string()),
            "{s}"
        );
    }
}

// ───────────────────────── Choix ≠ groupe ─────────────────────────

#[tokio::test]
async fn controle_choix_sans_groupe() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let a = acteur_de(m.etab, &cast.administratif);
    let offre = offre_creer(&ent, &a, saisie_offre(m.annee, "ALLEMAND", "langue_2")).await;
    let (ada, bob, cleo) = (
        eleve(&ent, m.etab, "Ada").await,
        eleve(&ent, m.etab, "Bob").await,
        eleve(&ent, m.etab, "Cléo").await,
    );
    let groupe = |code: &str| {
        let id = Uuid::new_v4();
        let code = code.to_string();
        let (etab, annee) = (m.etab, m.annee);
        let pool = ent.pool().clone();
        async move {
            sqlx::query("INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,$4,$4)")
                .bind(id).bind(etab).bind(annee).bind(code).execute(&pool).await.unwrap();
            id
        }
    };
    let affecter = |eleve: Uuid, groupe: Uuid| {
        let pool = ent.pool().clone();
        let (etab, annee) = (m.etab, m.annee);
        async move {
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, source)
                 VALUES ($1,$2,$3,$4,$5,$6,'saisie')",
            )
            .bind(Uuid::new_v4()).bind(etab).bind(annee).bind(eleve).bind(groupe)
            .bind(aujourdhui() - Duration::days(30))
            .execute(&pool).await.unwrap();
        }
    };
    ent.structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[offre]))
        .await
        .unwrap();
    ent.structure_enseignements_modifier(&a, bob, m.annee, ajouts(&[offre]))
        .await
        .unwrap();
    let liste = |causes: Vec<crate::structure_enseignements::Incoherence>| {
        let mut v: Vec<(String, Option<Uuid>)> =
            causes.into_iter().map(|c| (c.cause, c.eleve_id)).collect();
        v.sort();
        v
    };
    // Offre par groupe, aucun groupe associé : signalé une fois (les élèves ne sont pas accusés).
    let c = ent
        .structure_enseignements_incoherences(&a, m.annee, None)
        .await
        .unwrap();
    assert_eq!(liste(c), vec![("offre_sans_groupe".to_string(), None)]);
    // Groupes associés : les deux élèves suivent l'allemand sans être dans un groupe.
    let (g1, g2) = (groupe("ALL1").await, groupe("ALL2").await);
    ent.structure_offre_groupe_associer(&a, offre, g1)
        .await
        .unwrap();
    ent.structure_offre_groupe_associer(&a, offre, g2)
        .await
        .unwrap();
    let c = ent
        .structure_enseignements_incoherences(&a, m.annee, None)
        .await
        .unwrap();
    let mut attendu = vec![
        ("sans_groupe".to_string(), Some(ada)),
        ("sans_groupe".to_string(), Some(bob)),
    ];
    attendu.sort();
    assert_eq!(liste(c), attendu);
    // Ada dans un groupe : en règle. Bob dans deux : « plusieurs groupes ». Cléo dans un groupe sans suivre l'offre.
    affecter(ada, g1).await;
    affecter(bob, g1).await;
    affecter(bob, g2).await;
    affecter(cleo, g2).await;
    let c = ent
        .structure_enseignements_incoherences(&a, m.annee, None)
        .await
        .unwrap();
    let mut attendu = vec![
        ("plusieurs_groupes".to_string(), Some(bob)),
        ("groupe_sans_choix".to_string(), Some(cleo)),
    ];
    attendu.sort();
    assert_eq!(liste(c), attendu);
    // Cléo suit l'enseignement : plus d'écart pour elle. Ada l'a quitté : elle est de nouveau « groupe sans choix »
    // à partir du lendemain de la sortie (suivi jusqu'à la date de fin incluse).
    ent.structure_enseignements_modifier(&a, cleo, m.annee, ajouts(&[offre]))
        .await
        .unwrap();
    ent.structure_enseignements_modifier(
        &a,
        ada,
        m.annee,
        Modification {
            quitter: vec![Sortie {
                offre_id: offre,
                fin_nature: "abandon_fin_premiere".into(),
                motif: "abandon".into(),
            }],
            ..Modification::default()
        },
    )
    .await
    .unwrap();
    let demain = aujourdhui() + Duration::days(1);
    let c = ent
        .structure_enseignements_incoherences(&a, m.annee, Some(aujourdhui()))
        .await
        .unwrap();
    assert!(
        !c.iter().any(|x| x.eleve_id == Some(ada)),
        "suivi jusqu'à la date de fin incluse"
    );
    let c = ent
        .structure_enseignements_incoherences(&a, m.annee, Some(demain))
        .await
        .unwrap();
    assert!(c
        .iter()
        .any(|x| x.eleve_id == Some(ada) && x.cause == "groupe_sans_choix"));
    // Offre organisée par classe entière : aucun groupe attendu.
    let mut s = saisie_offre(m.annee, "CLASSE_ENTIERE", "commun");
    s.public_type = Some("classe".into());
    let ce = offre_creer(&ent, &a, s).await;
    ent.structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[ce]))
        .await
        .unwrap();
    let c = ent
        .structure_enseignements_incoherences(&a, m.annee, Some(aujourdhui()))
        .await
        .unwrap();
    assert!(!c.iter().any(|x| x.offre_id == ce));
    // Un groupe d'une autre année ne s'associe pas à l'offre ; un groupe ne porte qu'une offre.
    let g_prec = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,'GP','GP')")
        .bind(g_prec).bind(m.etab).bind(m.annee_prec).execute(ent.pool()).await.unwrap();
    assert!(ent
        .structure_offre_groupe_associer(&a, offre, g_prec)
        .await
        .is_err());
    let autre = offre_creer(
        &ent,
        &a,
        saisie_offre(m.annee, "AUTRE", "option_facultative"),
    )
    .await;
    assert!(ent
        .structure_offre_groupe_associer(&a, autre, g1)
        .await
        .is_err());
    // Les écarts ne sont lisibles que par les gestionnaires.
    let prof = acteur_de(m.etab, &cast.prof);
    assert!(est_forbidden(
        &ent.structure_enseignements_incoherences(&prof, m.annee, None)
            .await
    ));
}

// ───────────────────────── Reprise des options historiques ─────────────────────────

#[tokio::test]
async fn options_historiques_reprises_sans_perte_et_sans_creation_d_annee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let a = acteur_de(m.etab, &cast.administratif);
    let t = aujourdhui();
    let (e1, e2, e3) = (
        eleve(&ent, m.etab, "Ada").await,
        eleve(&ent, m.etab, "Bob").await,
        eleve(&ent, m.etab, "Cléo").await,
    );
    // Un groupe homonyme existe déjà pour LATIN : l'offre reprise s'y associe.
    let groupe = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,'LATIN','Latin')")
        .bind(groupe).bind(m.etab).bind(m.annee).execute(ent.pool()).await.unwrap();
    let annees_avant: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1")
            .bind(m.etab)
            .fetch_one(ent.pool())
            .await
            .unwrap();

    let opt = |eleve: Uuid, code: &str, debut: NaiveDate, fin: Option<NaiveDate>| {
        let pool = ent.pool().clone();
        let (etab, code) = (m.etab, code.to_string());
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_eleve_option (id, etablissement_id, eleve_id, nature, code, libelle, date_debut, date_fin)
                 VALUES ($1,$2,$3,'academique',$4,$4,$5,$6)",
            )
            .bind(id).bind(etab).bind(eleve).bind(code).bind(debut).bind(fin)
            .execute(&pool).await.unwrap();
            id
        }
    };
    let opt_annee = |eleve: Uuid, annee: &str, code: &str, statut: &str| {
        let pool = ent.pool().clone();
        let (etab, annee, code, statut) = (
            m.etab,
            annee.to_string(),
            code.to_string(),
            statut.to_string(),
        );
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_eleve_option_annee (id, etablissement_id, eleve_id, annee_scolaire, nature, code, libelle, statut)
                 VALUES ($1,$2,$3,$4,'interne',$5,$5,$6)",
            )
            .bind(id).bind(etab).bind(eleve).bind(annee).bind(code).bind(statut)
            .execute(&pool).await.unwrap();
            id
        }
    };
    // Ancienne option en cours, ancienne option terminée en cours d'année, option sur une date sans
    // année, option au code vide, doublon du même code pour le même élève.
    let o_latin = opt(e1, "LATIN", t - Duration::days(40), None).await;
    let o_fin = opt(
        e2,
        "CHORALE",
        t - Duration::days(40),
        Some(t - Duration::days(10)),
    )
    .await;
    // Une option à une date sans année : la ligne n'est PAS stockée (le pont historique du lot E4 la
    // refuse, et sa reprise rejouée à chaque migration s'y casserait) ; on soumet l'enregistrement
    // directement à la fonction de reprise, comme le ferait le pont pour une écriture historique.
    let o_sans_annee = Uuid::new_v4();
    let phantome: Option<Uuid> = sqlx::query_scalar(
        "SELECT ent_eleve_enseignement_depuis_option(ROW($1, $2, $3, 'academique', 'RUGBY', 'RUGBY', $4::date, NULL::date, now())::ent_eleve_option)",
    )
    .bind(o_sans_annee).bind(m.etab).bind(e3).bind(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap())
    .fetch_one(ent.pool()).await.unwrap();
    assert!(phantome.is_none());
    let o_vide = opt(e3, "   ", t - Duration::days(40), None).await;
    let o_doublon = opt(e1, "LATIN", t - Duration::days(35), None).await;
    // Propositions par année : retenue, proposée, retirée, et une année inconnue.
    let a_ret = opt_annee(e1, "N", "ESCRIME", "retenue").await;
    let a_prop = opt_annee(e2, "N", "ESCRIME", "proposee").await;
    let a_retiree = opt_annee(e3, "N", "ESCRIME", "retiree").await;
    let a_inconnue = opt_annee(e3, "2090-2091", "ESCRIME", "retenue").await;

    // 1. Le pont reprend à l'écriture ; 2. la reprise rejouée ne change rien (idempotente).
    let rejouee: i64 = sqlx::query_scalar("SELECT ent_eleve_enseignement_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(rejouee, 0, "tout est déjà reprise ou rangé à traiter");
    // Aucune option perdue : héritées = reprises + à traiter.
    let c = ent
        .structure_enseignements_comparaison_reprise(&a)
        .await
        .unwrap();
    // 8 lignes stockées + la ligne « sans année » soumise directement : héritées = reprises + à traiter.
    assert_eq!(c.heritees + 1, 9);
    assert_eq!(c.heritees + 1, c.reprises + c.a_traiter, "{c:?}");
    assert_eq!(
        c.a_traiter, 3,
        "code vide, date sans année, année inconnue : rangés, jamais perdus"
    );
    let cause = |id: Uuid| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, String>(
                "SELECT cause FROM ent_eleve_enseignement_a_traiter WHERE option_id = $1",
            )
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(cause(o_sans_annee).await.as_deref(), Some("annee_inconnue"));
    assert_eq!(cause(o_vide).await.as_deref(), Some("code_vide"));
    assert_eq!(cause(a_inconnue).await.as_deref(), Some("annee_inconnue"));
    // Aucune année créée implicitement.
    let annees_apres: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1")
            .bind(m.etab)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        annees_apres, annees_avant,
        "la reprise ne crée jamais d'année"
    );
    let offre_rugby: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_offre_enseignement WHERE etablissement_id = $1 AND code = 'RUGBY'")
        .bind(m.etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(offre_rugby, 0);

    // Enseignements obtenus.
    let v1 = ent
        .structure_enseignements_eleve(&a, e1, Some(m.annee), false)
        .await
        .unwrap();
    assert_eq!(v1.len(), 2, "latin (doublon fusionné) + escrime");
    let latin = v1.iter().find(|e| e.offre_code == "LATIN").unwrap();
    assert_eq!(
        (latin.etat.as_str(), latin.source.as_str()),
        ("accepte", "reprise_option")
    );
    assert_eq!(latin.debut_le, Some(t - Duration::days(40)));
    let escrime = v1.iter().find(|e| e.offre_code == "ESCRIME").unwrap();
    assert_eq!(escrime.etat, "accepte");
    let v2 = ent
        .structure_enseignements_eleve(&a, e2, Some(m.annee), false)
        .await
        .unwrap();
    let chorale = v2.iter().find(|e| e.offre_code == "CHORALE").unwrap();
    assert_eq!(
        (chorale.etat.as_str(), chorale.fin_nature.as_deref()),
        ("quitte", Some("changement_en_cours_annee"))
    );
    assert_eq!(chorale.quitte_le, Some(t - Duration::days(10)));
    assert_eq!(
        v2.iter().find(|e| e.offre_code == "ESCRIME").unwrap().etat,
        "choisi",
        "proposée = choisi"
    );
    let v3 = ent
        .structure_enseignements_eleve(&a, e3, Some(m.annee), false)
        .await
        .unwrap();
    assert_eq!(
        v3.iter().find(|e| e.offre_code == "ESCRIME").unwrap().etat,
        "retire",
        "retirée = retiré"
    );
    // L'offre latin a repris le groupe homonyme ; les offres reprises sont marquées comme telles.
    let o = ent.structure_offres(&a, Some(m.annee)).await.unwrap();
    let l = o.iter().find(|x| x.code == "LATIN").unwrap();
    assert_eq!(
        (l.groupe_ids.clone(), l.source.as_str(), l.ouvert_aux_voeux),
        (vec![groupe], "reprise_option", false)
    );
    assert_eq!(l.modalite, "option_facultative");
    // Une mise à jour de la ligne historique est suivie (proposition acceptée, option terminée plus tard).
    sqlx::query("UPDATE ent_eleve_option_annee SET statut = 'retenue' WHERE id = $1")
        .bind(a_prop)
        .execute(ent.pool())
        .await
        .unwrap();
    let v2 = ent
        .structure_enseignements_eleve(&a, e2, Some(m.annee), false)
        .await
        .unwrap();
    assert_eq!(
        v2.iter().find(|e| e.offre_code == "ESCRIME").unwrap().etat,
        "accepte"
    );
    sqlx::query("UPDATE ent_eleve_option SET date_fin = $2 WHERE id = $1")
        .bind(o_latin)
        .bind(t - Duration::days(5))
        .execute(ent.pool())
        .await
        .unwrap();
    let v1 = ent
        .structure_enseignements_eleve(&a, e1, Some(m.annee), false)
        .await
        .unwrap();
    assert_eq!(
        v1.iter().find(|e| e.offre_code == "LATIN").unwrap().etat,
        "quitte"
    );
    // Un doublon n'a pas créé de seconde ligne active pour le même couple élève / offre.
    let actifs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_eleve_enseignement WHERE eleve_id = $1 AND offre_id = $2 AND etat IN ('choisi','accepte')",
    )
    .bind(e1).bind(l.id).fetch_one(ent.pool()).await.unwrap();
    assert!(actifs <= 1, "{actifs}");
    let _ = (o_fin, o_doublon, a_ret, a_retiree);
    // Chaque ligne reprise a sa correspondance (traçabilité de la reprise).
    let map: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_eleve_enseignement_reprise WHERE etablissement_id = $1",
    )
    .bind(m.etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(map, 6);
}

// ───────────────────────── Port pour le socle 3 ─────────────────────────

#[tokio::test]
async fn offre_lue_par_le_port_pour_le_socle_3() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let a = acteur_de(m.etab, &cast.direction);
    let (n6, n5) = (
        niveau(&ent, m.etab, "6E").await,
        niveau(&ent, m.etab, "5E").await,
    );
    let ouverte = |code: &str, niveaux: Vec<Uuid>, cap: Option<i32>| {
        let mut s = saisie_offre(m.annee, code, "option_facultative");
        s.niveau_ids = niveaux;
        s.capacite = cap;
        s.ouvert_aux_voeux = true;
        s
    };
    let latin = offre_creer(&ent, &a, ouverte("LATIN", vec![n5], Some(2))).await;
    let _tous = offre_creer(&ent, &a, ouverte("ANGLAIS_RENFORCE", vec![], None)).await;
    let _six = offre_creer(&ent, &a, ouverte("BILANGUE", vec![n6], None)).await;
    let _fermee = offre_creer(&ent, &a, ouverte("FERMEE", vec![n5], None)).await;
    let fermee_id = ent
        .structure_offres(&a, Some(m.annee))
        .await
        .unwrap()
        .iter()
        .find(|o| o.code == "FERMEE")
        .unwrap()
        .id;
    ent.structure_offre_fermer(&a, fermee_id, "x", None, true)
        .await
        .unwrap();
    let mut non_voeu = saisie_offre(m.annee, "INTERNE", "option_facultative");
    non_voeu.niveau_ids = vec![n5];
    offre_creer(&ent, &a, non_voeu).await;

    let mut t = TenantTx::begin(ent.pool(), m.etab).await.unwrap();
    let liste = offre::offres_ouvertes_aux_voeux(&mut t, m.etab, m.annee, Some(n5), None)
        .await
        .unwrap();
    let mut c: Vec<_> = liste.iter().map(|o| o.code.clone()).collect();
    c.sort();
    assert_eq!(
        c,
        vec!["ANGLAIS_RENFORCE", "LATIN"],
        "ni fermée, ni non ouverte aux vœux, ni autre niveau"
    );
    let l = liste.iter().find(|o| o.code == "LATIN").unwrap();
    assert_eq!((l.capacite, l.places_restantes), (Some(2), Some(2)));
    // Sans niveau : toutes les offres ouvertes aux vœux.
    let toutes = offre::offres_ouvertes_aux_voeux(&mut t, m.etab, m.annee, None, None)
        .await
        .unwrap();
    assert_eq!(toutes.len(), 3);
    // Un autre établissement ne lit rien de cette offre (RLS de la transaction + filtre).
    t.commit().await.unwrap();
    let autre = Uuid::new_v4();
    let mut t = TenantTx::begin(ent.pool(), autre).await.unwrap();
    assert!(
        offre::offres_ouvertes_aux_voeux(&mut t, autre, m.annee, Some(n5), None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(offre::lire(&mut t, autre, latin).await.unwrap().is_none());
    t.commit().await.unwrap();
    // Les places se réduisent avec les enseignements acceptés.
    let ada = eleve(&ent, m.etab, "Ada").await;
    ent.structure_enseignements_modifier(&a, ada, m.annee, ajouts(&[latin]))
        .await
        .unwrap();
    let mut t = TenantTx::begin(ent.pool(), m.etab).await.unwrap();
    assert_eq!(
        offre::lire(&mut t, m.etab, latin)
            .await
            .unwrap()
            .unwrap()
            .places_restantes,
        Some(1)
    );
    // Le moteur de règles est le même pour le socle 3 : vœux incompatibles refusés (élève inconnu).
    let v = offre::verifier_choix(
        &mut t,
        m.etab,
        &Demande {
            annee_id: m.annee,
            contexte: ContexteChoix {
                niveau_id: Some(n6),
                mef_id: None,
            },
            eleve_id: None,
            ensemble: &[latin],
            nouveaux: &[latin],
            jour: aujourdhui(),
            complet: false,
            forcer_capacite: false,
        },
    )
    .await
    .unwrap();
    assert!(v.iter().any(|x| x.code == "hors_cible" && x.bloquant));
    t.commit().await.unwrap();
}

#[tokio::test]
async fn socle_3_lit_l_offre_d_enseignements_d_une_campagne() {
    use crate::tests_inscriptions::{campagne, dir};
    use ent_types::inscriptions::*;
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (d, dr) = dir(&cast);
    let code = format!("N{}", sfx());
    let a = acteur_de(etab, &cast.direction);
    // Niveau du référentiel rapproché par son code, campagne sur une année propre à ce test.
    let niv = niveau(&ent, etab, &code).await;
    let camp = ent
        .creer_campagne_for_actor(
            etab,
            d,
            dr,
            &CampagneBody {
                annee_cible: "2081-2082".into(),
                niveaux: vec![NiveauCampagneDto {
                    code: code.clone(),
                    capacite: Some(30),
                }],
                ..campagne("admission", None)
            },
        )
        .await
        .unwrap();
    let annee = camp.annee_id.expect("année du référentiel");
    let offre_ins = camp
        .offres
        .iter()
        .find(|o| o.niveau_code == code)
        .expect("offre d'inscription");
    assert_eq!(
        offre_ins.niveau_id,
        Some(niv),
        "niveau rapproché du référentiel"
    );
    let mut s = saisie_offre(annee, &format!("LATIN_{code}"), "option_facultative");
    s.niveau_ids = vec![niv];
    s.ouvert_aux_voeux = true;
    offre_creer(&ent, &a, s).await;
    let mut s = saisie_offre(annee, &format!("NON_OUVERTE_{code}"), "option_facultative");
    s.niveau_ids = vec![niv];
    offre_creer(&ent, &a, s).await;
    let l = ent
        .enseignements_proposes_for_actor(etab, d, dr, camp.id, offre_ins.id)
        .await
        .unwrap();
    assert_eq!(l.len(), 1);
    assert!(l[0].code.starts_with("LATIN_"));
    // Une offre d'une autre campagne ou inconnue : introuvable ; les familles n'accèdent pas à ce port.
    assert!(ent
        .enseignements_proposes_for_actor(etab, d, dr, camp.id, Uuid::new_v4())
        .await
        .is_err());
    assert!(ent
        .enseignements_proposes_for_actor(
            etab,
            cast.parent_lea.user_id,
            "parent",
            camp.id,
            offre_ins.id
        )
        .await
        .is_err());
}

// ───────────────────────── Droits, périmètre D-022, cloisonnement ─────────────────────────

#[tokio::test]
async fn professeur_ne_voit_que_les_enseignements_de_son_perimetre_d022() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let mo = monde(&ent, &cast).await;
    let annee = ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let adm = acteur_de(etab, &cast.administratif);
    let sx = sfx();
    let offre = offre_creer(
        &ent,
        &adm,
        saisie_offre(annee, &format!("OPT{sx}"), "option_facultative"),
    )
    .await;
    // Un enseignement avec un motif sensible pour chacun des quatre élèves de la classe mixte.
    for e in mo.all.iter().chain(mo.esp.iter()) {
        ent.structure_enseignements_modifier(&adm, *e, annee, ajouts(&[offre]))
            .await
            .unwrap();
        ent.structure_enseignements_modifier(
            &adm,
            *e,
            annee,
            Modification {
                quitter: vec![Sortie {
                    offre_id: offre,
                    fin_nature: "correction".into(),
                    motif: "raison de santé".into(),
                }],
                ..Modification::default()
            },
        )
        .await
        .unwrap();
    }
    let prof_all = Acteur {
        etablissement_id: etab,
        user_id: mo.prof_all,
        role: "prof",
    };
    let prof_esp = Acteur {
        etablissement_id: etab,
        user_id: mo.prof_esp,
        role: "prof",
    };
    let prof_principal = Acteur {
        etablissement_id: etab,
        user_id: mo.prof_principal,
        role: "prof",
    };
    // Le professeur d'allemand lit les siens (sans le motif libre), pas ceux d'espagnol.
    for e in mo.all {
        let v = ent
            .structure_enseignements_eleve(&prof_all, e, Some(annee), false)
            .await
            .unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].motif.is_none(), "motif réservé aux gestionnaires");
        let h = ent
            .structure_enseignement_historique(&prof_all, v[0].id)
            .await
            .unwrap();
        assert!(h.iter().all(|l| l.motif.is_none()));
    }
    for e in mo.esp {
        assert!(
            est_forbidden(
                &ent.structure_enseignements_eleve(&prof_all, e, Some(annee), false)
                    .await
            ),
            "allemand ↛ espagnol"
        );
        // L'historique d'un hors-périmètre n'est pas distinguable d'un identifiant inconnu.
        let id: Uuid = sqlx::query_scalar(
            "SELECT id FROM ent_eleve_enseignement WHERE eleve_id = $1 AND offre_id = $2",
        )
        .bind(e)
        .bind(offre)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert!(est_domaine(
            &ent.structure_enseignement_historique(&prof_all, id).await,
            DomainError::NotFound
        ));
    }
    for e in mo.esp {
        assert!(ent
            .structure_enseignements_eleve(&prof_esp, e, Some(annee), false)
            .await
            .is_ok());
    }
    for e in mo.all {
        assert!(est_forbidden(
            &ent.structure_enseignements_eleve(&prof_esp, e, Some(annee), false)
                .await
        ));
    }
    // Le professeur principal de la classe entière voit les quatre.
    for e in mo.all.iter().chain(mo.esp.iter()) {
        assert!(ent
            .structure_enseignements_eleve(&prof_principal, *e, Some(annee), false)
            .await
            .is_ok());
    }
    // Un élève hors de toute classe de ce professeur (élève du labo) : refusé.
    let hors = eleve(&ent, etab, "HorsPerimetre").await;
    assert!(est_forbidden(
        &ent.structure_enseignements_eleve(&prof_all, hors, Some(annee), false)
            .await
    ));
    // Un professeur ne modifie rien, même dans son périmètre.
    let r = ent
        .structure_enseignements_modifier(&prof_all, mo.all[0], annee, ajouts(&[offre]))
        .await;
    assert!(est_forbidden(&r));
    // Gestionnaire : motif visible. Autre établissement : élève introuvable.
    let v = ent
        .structure_enseignements_eleve(&adm, mo.all[0], Some(annee), false)
        .await
        .unwrap();
    assert_eq!(v[0].motif.as_deref(), Some("raison de santé"));
    let etranger = acteur_de(Uuid::new_v4(), &cast.direction);
    assert!(est_domaine(
        &ent.structure_enseignements_eleve(&etranger, mo.all[0], Some(annee), false)
            .await,
        DomainError::NotFound
    ));
    // …et l'offre d'un établissement n'existe pas pour un autre.
    assert!(est_domaine(
        &ent.structure_offre(&etranger, offre).await,
        DomainError::NotFound
    ));
    assert!(ent
        .structure_offre_modifier(&etranger, offre, OffreModif::default())
        .await
        .is_err());
}

#[tokio::test]
async fn matrice_de_droits_offre_et_enseignements() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_neuf(&ent).await;
    let gest = acteur_de(m.etab, &cast.administratif);
    let offre = offre_creer(
        &ent,
        &gest,
        saisie_offre(m.annee, "THEATRE", "option_facultative"),
    )
    .await;
    let ada = eleve(&ent, m.etab, "Ada").await;
    ent.structure_enseignements_modifier(&gest, ada, m.annee, ajouts(&[offre]))
        .await
        .unwrap();

    // (nom, persona, lit le catalogue, écrit, lit les enseignements d'un élève)
    let personas: [(&str, &LaboPersona, bool, bool, bool); 7] = [
        ("admin", &cast.admin, true, true, true),
        ("direction", &cast.direction, true, true, true),
        ("administratif", &cast.administratif, true, true, true),
        ("compta", &cast.compta, true, false, false),
        ("prof", &cast.prof, true, false, false), // aucun élève dans son périmètre ici
        ("cpe", &cast.cpe, true, false, true),
        ("parent", &cast.parent_lea, false, false, false),
    ];
    for (nom, p, lit, ecrit, lit_eleve) in personas {
        let x = acteur_de(m.etab, p);
        assert_eq!(
            !est_forbidden(&ent.structure_matieres(&x, false).await),
            lit,
            "matières : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_offres(&x, Some(m.annee)).await),
            lit,
            "offre : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_regles_choix(&x, m.annee).await),
            lit,
            "règles : {nom}"
        );
        assert_eq!(
            !est_forbidden(&ent.structure_incompatibilites(&x, m.annee).await),
            lit,
            "incompatibilités : {nom}"
        );
        let e = ent
            .structure_enseignements_eleve(&x, ada, Some(m.annee), false)
            .await;
        assert_eq!(
            e.is_ok(),
            lit_eleve,
            "enseignements d'un élève : {nom} → {e:?}"
        );
        // Écritures : gestionnaires seulement.
        let o = ent
            .structure_offre_creer(&x, saisie_offre(m.annee, &format!("X{}", sfx()), "autre"))
            .await;
        assert_eq!(!est_forbidden(&o), ecrit, "créer une offre : {nom}");
        let mat = ent
            .structure_matiere_creer(
                &x,
                MatiereSaisie {
                    code: format!("M{}", sfx()),
                    libelle: "M".into(),
                    libelle_court: None,
                    couleur: None,
                    famille: None,
                    parent_id: None,
                    code_national: None,
                },
            )
            .await;
        assert_eq!(!est_forbidden(&mat), ecrit, "créer une matière : {nom}");
        let ch = ent
            .structure_enseignements_modifier(
                &x,
                ada,
                m.annee,
                Modification {
                    apercu: true,
                    ..ajouts(&[offre])
                },
            )
            .await;
        assert_eq!(
            !est_forbidden(&ch),
            ecrit,
            "modifier les enseignements : {nom}"
        );
        assert_eq!(
            !est_forbidden(
                &ent.structure_enseignements_incoherences(&x, m.annee, None)
                    .await
            ),
            ecrit,
            "file de travail : {nom}"
        );
    }
}

#[tokio::test]
async fn contraintes_de_la_base_cloisonnement_et_gel_d_annee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m1 = etab_neuf(&ent).await;
    let m2 = etab_neuf(&ent).await;
    let a1 = acteur_de(m1.etab, &cast.direction);
    let a2 = acteur_de(m2.etab, &cast.direction);
    let n2 = niveau(&ent, m2.etab, "6E").await;
    // Cible d'un autre établissement : refusée à la création par le service, et par la clé composite.
    let mut s = saisie_offre(m1.annee, "X", "commun");
    s.niveau_ids = vec![n2];
    assert!(est_domaine(
        &ent.structure_offre_creer(&a1, s).await,
        DomainError::NotFound
    ));
    let o1 = offre_creer(&ent, &a1, saisie_offre(m1.annee, "O1", "commun")).await;
    let r = sqlx::query("INSERT INTO ent_offre_cible (id, etablissement_id, offre_id, niveau_id) VALUES ($1,$2,$3,$4)")
        .bind(Uuid::new_v4()).bind(m1.etab).bind(o1).bind(n2).execute(ent.pool()).await;
    assert!(r.is_err(), "clé composite (niveau, établissement)");
    // Une offre d'une année qui n'est pas la sienne.
    let mut s = saisie_offre(m2.annee, "AILLEURS", "commun");
    s.annee_id = m1.annee;
    assert!(ent.structure_offre_creer(&a2, s).await.is_err());
    // Règle sur des offres de deux établissements / deux années : refusée.
    let o2 = offre_creer(&ent, &a2, saisie_offre(m2.annee, "O2", "commun")).await;
    let r = ent
        .structure_regle_creer(
            &a1,
            RegleSaisie {
                annee_id: m1.annee,
                niveau_id: None,
                mef_id: None,
                libelle: "mixte".into(),
                offre_ids: vec![o1, o2],
                min: 0,
                max: Some(1),
                bloquant: true,
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::NotFound));
    let o1_prec = offre_creer(&ent, &a1, saisie_offre(m1.annee_prec, "O1", "commun")).await;
    let r = ent
        .structure_incompatibilite_declarer(
            &a1,
            IncompatibiliteSaisie {
                offre_a: o1,
                offre_b: o1_prec,
                motif: None,
                bloquant: true,
            },
        )
        .await;
    assert!(
        est_domaine(&r, DomainError::ValeurInvalide),
        "deux années : {r:?}"
    );
    assert!(est_domaine(
        &ent.structure_incompatibilite_declarer(
            &a1,
            IncompatibiliteSaisie {
                offre_a: o1,
                offre_b: o2,
                motif: None,
                bloquant: true
            }
        )
        .await,
        DomainError::NotFound
    ));
    // Un code d'offre est unique dans l'année ; il est libre dans une autre année.
    assert!(est_domaine(
        &ent.structure_offre_creer(&a1, saisie_offre(m1.annee, "O1", "commun"))
            .await,
        DomainError::Conflit
    ));
    // Une offre ne se supprime pas.
    assert!(
        sqlx::query("DELETE FROM ent_offre_enseignement WHERE id = $1")
            .bind(o1)
            .execute(ent.pool())
            .await
            .is_err()
    );
    // Valeurs invalides.
    for mauvaise in [
        OffreSaisie {
            modalite: "n_importe_quoi".into(),
            ..saisie_offre(m1.annee, "A", "commun")
        },
        OffreSaisie {
            admission: Some("tirage_au_sort".into()),
            ..saisie_offre(m1.annee, "B", "commun")
        },
        OffreSaisie {
            public_type: Some("ecole".into()),
            ..saisie_offre(m1.annee, "C", "commun")
        },
        OffreSaisie {
            capacite: Some(-1),
            ..saisie_offre(m1.annee, "D", "commun")
        },
        OffreSaisie {
            code_modalite_election: Some("trop long trop".into()),
            ..saisie_offre(m1.annee, "E", "commun")
        },
        saisie_offre(m1.annee, "", "commun"),
    ] {
        assert!(est_domaine(
            &ent.structure_offre_creer(&a1, mauvaise).await,
            DomainError::ValeurInvalide
        ));
    }
    // Année clôturée : l'offre est gelée (création, modification, enseignements).
    let ada = eleve(&ent, m1.etab, "Ada").await;
    ent.structure_enseignements_modifier(&a1, ada, m1.annee_prec, ajouts(&[o1_prec]))
        .await
        .unwrap();
    sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE id = $1")
        .bind(m1.annee_prec)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(est_domaine(
        &ent.structure_offre_creer(&a1, saisie_offre(m1.annee_prec, "NOUVELLE", "commun"))
            .await,
        DomainError::Conflit
    ));
    assert!(ent
        .structure_offre_modifier(
            &a1,
            o1_prec,
            OffreModif {
                libelle: Some("Z".into()),
                ..Default::default()
            }
        )
        .await
        .is_err());
    let r = ent
        .structure_enseignements_modifier(
            &a1,
            ada,
            m1.annee_prec,
            Modification {
                quitter: vec![Sortie {
                    offre_id: o1_prec,
                    fin_nature: "correction".into(),
                    motif: "x".into(),
                }],
                ..Modification::default()
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::Conflit));
    // Les enseignements d'une année close restent lisibles (historique).
    let v = ent
        .structure_enseignements_eleve(&a1, ada, Some(m1.annee_prec), false)
        .await
        .unwrap();
    assert_eq!(v.len(), 1);
}

#[tokio::test]
async fn rls_strict_tables_du_lot_s5() {
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
    let tables = "ent_offre_enseignement, ent_offre_cible, ent_offre_groupe, ent_offre_regle_choix, \
                  ent_offre_regle_choix_offre, ent_offre_incompatibilite, ent_offre_prerequis, \
                  ent_eleve_enseignement, ent_eleve_enseignement_histo, ent_eleve_enseignement_reprise, \
                  ent_eleve_enseignement_a_traiter, ent_structure_journal";
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

    let (m1, m2) = (etab_neuf(&ent).await, etab_neuf(&ent).await);
    let (a, b) = (
        acteur_de(m1.etab, &cast.direction),
        acteur_de(m2.etab, &cast.direction),
    );
    let (o_a, o_b) = (
        offre_creer(&ent, &a, saisie_offre(m1.annee, "O", "commun")).await,
        offre_creer(&ent, &b, saisie_offre(m2.annee, "O", "commun")).await,
    );
    for (x, mm) in [(&a, &m1), (&b, &m2)] {
        let e = eleve(&ent, mm.etab, "E").await;
        let o = if x.etablissement_id == m1.etab {
            o_a
        } else {
            o_b
        };
        ent.structure_enseignements_modifier(x, e, mm.annee, ajouts(&[o]))
            .await
            .unwrap();
    }
    for (table, attendu) in [
        ("ent_offre_enseignement", 1i64),
        ("ent_eleve_enseignement", 1),
        ("ent_eleve_enseignement_histo", 1),
    ] {
        let mut t = TenantTx::begin_mode(&app, m1.etab, RlsMode::Strict)
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
        .bind(m1.etab)
        .fetch_one(&mut *t)
        .await
        .unwrap()
        .get(0);
        assert_eq!(
            autres, 0,
            "{table} : lignes d'un autre établissement visibles"
        );
        assert_eq!(n, attendu, "{table}");
        if table == "ent_offre_enseignement" {
            let r = sqlx::query(
                "INSERT INTO ent_offre_enseignement (id, etablissement_id, annee_id, code, libelle, modalite)
                 VALUES ($1,$2,$3,'Z','Z','commun')",
            )
            .bind(Uuid::new_v4()).bind(m2.etab).bind(m2.annee).execute(&mut *t).await;
            let err = r.expect_err("insertion inter-établissement refusée");
            assert_eq!(
                err.as_database_error().and_then(|d| d.code()).as_deref(),
                Some("42501")
            );
        }
        drop(t);
    }
    // Sans TenantTx, le rôle strict ne voit rien.
    for table in ["ent_offre_enseignement", "ent_eleve_enseignement"] {
        let n: i64 = sqlx::query(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&app)
            .await
            .unwrap()
            .get(0);
        assert_eq!(n, 0, "{table}");
    }
}
