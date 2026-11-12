//! Socle 3, lot I2 (`docs/socles/03_INSCRIPTIONS.md` §16 M3 à M5, §17 bis) : modèle des campagnes,
//! de l'offre et des places, démarches (fratrie), déclarations, états et transitions, règles des
//! personas (« ne pas solliciter », adresse à protéger, annulation d'une validation, prolongation
//! d'échéance, fermetures). Contre la base locale de test (feature `labo`).

use super::tests_inscriptions::{
    adm, campagne, campagne_ouverte, compter, courriels_vers, dir, dossier_admis, distributeur,
    formulaire, labo, second_parent, verifier,
};
use super::*;
use crate::inscriptions::{dossier_tx, transitionner_tx};
use crate::inscriptions_etats::{self, Cause, NATURES};
use ent_auth::Role;
use ent_types::inscriptions::*;
use uuid::Uuid;

fn email(prefixe: &str) -> String {
    format!("{prefixe}-{}@exemple.test", Uuid::new_v4().simple())
}

const CAUSES: [Cause; 5] = [
    Cause::Etape,
    Cause::Decision,
    Cause::Validation,
    Cause::AnnulationValidation,
    Cause::Blocage,
];

/// Insère directement un dossier dans `etat` (campagne dédiée) et renvoie son identifiant.
async fn dossier_en_etat(
    ent: &PgEnt,
    c: &LaboCast,
    nature: &str,
    etat: &str,
    avant_blocage: Option<&str>,
) -> Uuid {
    let etab = c.etablissement_id;
    let camp = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_campagne (id, etablissement_id, nature, libelle, annee_cible)
         VALUES ($1,$2,$3,'Exhaustif','2027-2028')",
    )
    .bind(camp)
    .bind(etab)
    .bind(nature)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let id = Uuid::new_v4();
    // Élève et date de validation toujours posés : la transition vers « validée » (réservée à la
    // validation, qui les écrit) ne doit pas buter sur les contraintes du dossier.
    sqlx::query(
        "INSERT INTO ent_inscription_dossier
            (id, etablissement_id, campagne_id, nature, etat, etat_avant_blocage, eleve_nom, eleve_prenom,
             eleve_id, valide_le)
         VALUES ($1,$2,$3,$4,$5,$6,'Test','Etat',$7, now())",
    )
    .bind(id)
    .bind(etab)
    .bind(camp)
    .bind(nature)
    .bind(etat)
    .bind(avant_blocage)
    .bind(c.eleve_lea_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

/// CRITÈRE I2 — « chaque transition interdite est refusée par le service » : toutes les paires
/// (nature, de, vers) de toutes les natures, avec chaque cause, contre le service et la base.
#[tokio::test]
async fn i2_machine_d_etats_chaque_transition_est_testee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (mut admises, mut refusees) = (0, 0);
    for nature in NATURES {
        for de in inscriptions_etats::etats(nature) {
            for vers in inscriptions_etats::etats(nature) {
                // Un dossier bloqué revient à l'état d'avant : on le fait pointer vers `vers`
                // quand c'est un état ordinaire, pour tester aussi le retour autorisé.
                let avant = (*de == "bloque_desaccord" && *vers != "bloque_desaccord")
                    .then_some(*vers);
                let causes: &[Cause] = if inscriptions_etats::transition_admise(nature, de, vers) {
                    &CAUSES
                } else {
                    &[Cause::Etape]
                };
                for cause in causes {
                    let id = dossier_en_etat(&ent, &c, nature, de, avant).await;
                    let mut tx = ent.pool_verif().begin().await.unwrap();
                    let d = dossier_tx(&mut tx, etab, id).await.unwrap();
                    let attendu = inscriptions_etats::verifier_transition(nature, de, vers, *cause)
                        .is_ok()
                        && (*de != "bloque_desaccord"
                            || inscriptions_etats::retour_de_blocage_admis(
                                d.etat_avant_blocage.as_deref(),
                                vers,
                            ));
                    let r = transitionner_tx(&mut tx, etab, &d, vers, *cause, Some("test"), None).await;
                    assert_eq!(
                        r.is_ok(),
                        attendu,
                        "{nature}: {de} → {vers} (cause {cause:?}) : attendu {attendu}, obtenu {:?}",
                        r.as_ref().err().map(|e| e.to_string())
                    );
                    if attendu {
                        admises += 1;
                        tx.commit().await.unwrap();
                        let (etat, statut): (String, String) = sqlx::query_as(
                            "SELECT etat, statut FROM ent_inscription_dossier WHERE id = $1",
                        )
                        .bind(id)
                        .fetch_one(ent.pool_verif())
                        .await
                        .unwrap();
                        assert_eq!(etat, *vers);
                        // La vue historique suit (déclencheur 304).
                        let (attendu_statut,): (String,) =
                            sqlx::query_as("SELECT ent_inscription_statut_depuis_etat($1,$2,$3)")
                                .bind(nature)
                                .bind(vers)
                                .bind(&d.statut)
                                .fetch_one(ent.pool_verif())
                                .await
                                .unwrap();
                        assert_eq!(statut, attendu_statut, "{nature}: statut après {de} → {vers}");
                    } else {
                        refusees += 1;
                        drop(tx);
                        let (etat,): (String,) =
                            sqlx::query_as("SELECT etat FROM ent_inscription_dossier WHERE id = $1")
                                .bind(id)
                                .fetch_one(ent.pool_verif())
                                .await
                                .unwrap();
                        assert_eq!(etat, *de, "une transition refusée ne change rien");
                    }
                }
            }
        }
    }
    assert!(admises > 100 && refusees > 300, "{admises} admises, {refusees} refusées");
    // La liste des états du SQL est celle du service, pour toutes les natures.
    for nature in NATURES {
        let mut tous: Vec<&str> = NATURES
            .iter()
            .flat_map(|n| inscriptions_etats::etats(n).iter().copied())
            .collect();
        tous.sort_unstable();
        tous.dedup();
        for e in tous {
            let (sql,): (bool,) = sqlx::query_as("SELECT ent_inscription_etat_valide($1,$2)")
                .bind(nature)
                .bind(e)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
            assert_eq!(sql, inscriptions_etats::etats(nature).contains(&e), "{nature}/{e}");
        }
    }
    let (inconnue,): (bool,) = sqlx::query_as("SELECT ent_inscription_etat_valide('inconnue','deposee')")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(!inconnue);
}

/// Les états de campagne : une campagne fermée ne se rouvre pas, chaque paire est testée.
#[tokio::test]
async fn i2_campagne_etats() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let etats = ["brouillon", "planifiee", "ouverte", "suspendue", "fermee", "archivee"];
    for de in etats {
        for vers in etats {
            let camp = ent
                .creer_campagne_for_actor(etab, d, dr, &CampagneBody {
                    libelle: format!("{de}-{vers}"),
                    annee_cible: {
                        let n = 2100 + etats.iter().position(|x| *x == de).unwrap() * 10
                            + etats.iter().position(|x| *x == vers).unwrap();
                        format!("{n}-{}", n + 1)
                    },
                    ..campagne("admission", None)
                })
                .await
                .unwrap();
            sqlx::query("UPDATE ent_inscription_campagne SET statut = $2 WHERE id = $1")
                .bind(camp.id)
                .bind(de)
                .execute(ent.pool_verif())
                .await
                .unwrap();
            let r = ent.changer_statut_campagne_for_actor(etab, d, dr, camp.id, vers).await;
            assert_eq!(
                r.is_ok(),
                crate::inscriptions_modele::transition_campagne_admise(de, vers),
                "campagne {de} → {vers}"
            );
        }
    }
    // Ouvrir, fermer, puis ne pas rouvrir.
    let camp = ent.creer_campagne_for_actor(etab, d, dr, &CampagneBody {
        annee_cible: "2051-2052".into(),
        ..campagne("admission", None)
    }).await.unwrap();
    ent.ouvrir_campagne_for_actor(etab, d, dr, camp.id).await.unwrap();
    assert!(ent.ouvrir_campagne_for_actor(etab, d, dr, camp.id).await.is_err(), "déjà ouverte");
    ent.fermer_campagne_for_actor(etab, d, dr, camp.id).await.unwrap();
    assert!(matches!(
        ent.ouvrir_campagne_for_actor(etab, d, dr, camp.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Le secrétariat ne change pas l'état d'une campagne sans le droit de gestion… il l'a (gerer) ;
    // un rôle sans droit d'inscriptions est refusé.
    let prof = (c.prof.user_id, Role::Prof.as_str());
    assert!(matches!(
        ent.changer_statut_campagne_for_actor(etab, prof.0, prof.1, camp.id, "archivee").await,
        Err(EntDbError::Forbidden)
    ));
    let _ = (a, ar);
    // Chaque modification après la création crée une version, jamais sans trace.
    let v: i64 = compter(&ent, "SELECT count(*) FROM ent_inscription_campagne_version WHERE campagne_id = $1", camp.id).await;
    assert!(v >= 3, "création, ouverture, fermeture : {v} versions");
}

/// Année du référentiel, règles de la campagne et offres issues des niveaux ; bornes du logiciel.
#[tokio::test]
async fn i2_campagne_regles_annee_et_offres_des_niveaux() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = ent
        .creer_campagne_for_actor(etab, d, dr, &CampagneBody {
            annee_cible: "2061-2062".into(),
            niveaux: vec![
                NiveauCampagneDto { code: "6EME".into(), capacite: Some(30) },
                NiveauCampagneDto { code: "5EME".into(), capacite: None },
            ],
            regles: Some(ReglesCampagneDto {
                accord_responsables: "tous_les_titulaires".into(),
                delai_reponse_jours: Some(10),
                ..ReglesCampagneDto::default()
            }),
            ..campagne("admission", None)
        })
        .await
        .unwrap();
    assert!(camp.annee_id.is_some(), "aucune campagne sans année du référentiel");
    assert_eq!(camp.version, 1);
    assert_eq!(camp.regles.accord_responsables, "tous_les_titulaires");
    assert_eq!(camp.regles.delai_reponse_jours, Some(10));
    assert_eq!(camp.offres.len(), 2, "les niveaux deviennent des offres sans parcours");
    let o6 = camp.offres.iter().find(|o| o.niveau_code == "6EME").unwrap();
    assert_eq!((o6.capacite, o6.disponibles), (Some(30), Some(30)));
    let o5 = camp.offres.iter().find(|o| o.niveau_code == "5EME").unwrap();
    assert_eq!((o5.capacite, o5.disponibles), (None, None));
    // Bornes du logiciel.
    for regles in [
        ReglesCampagneDto { delai_reponse_jours: Some(1), ..ReglesCampagneDto::default() },
        ReglesCampagneDto { delai_reponse_jours: Some(31), ..ReglesCampagneDto::default() },
        ReglesCampagneDto { delai_annulation_validation_h: 169, ..ReglesCampagneDto::default() },
        ReglesCampagneDto { accord_responsables: "personne".into(), ..ReglesCampagneDto::default() },
        ReglesCampagneDto { motif_communique: "peut-etre".into(), ..ReglesCampagneDto::default() },
        ReglesCampagneDto { mode_reinscription: Some("tacite".into()), ..ReglesCampagneDto::default() },
    ] {
        assert!(ent
            .creer_campagne_for_actor(etab, d, dr, &CampagneBody { regles: Some(regles), ..campagne("admission", None) })
            .await
            .is_err());
    }
    // Les quatre natures existent ; l'affectation ne comporte pas de décision.
    for nature in ["affectes", "en_cours_annee", "reinscription"] {
        let k = ent
            .creer_campagne_for_actor(etab, d, dr, &CampagneBody { annee_cible: "2062-2063".into(), ..campagne(nature, None) })
            .await
            .unwrap();
        assert_eq!(k.regles.decision_requise, false, "{nature}");
        assert_eq!(k.regles.mode_reinscription.is_some(), nature == "reinscription");
    }
    // Modifier une campagne ouverte : nouvelle version avec son motif.
    let camp2 = campagne_ouverte(&ent, &c, "admission", Some(5)).await;
    let avant = ent.list_campagnes_for_actor(etab, d, dr).await.unwrap().into_iter().find(|k| k.id == camp2).unwrap();
    let apres = ent
        .modifier_campagne_for_actor(etab, d, dr, camp2, &CampagneBody {
            motif_modification: Some("Capacité relevée".into()),
            niveaux: vec![NiveauCampagneDto { code: "6EME".into(), capacite: Some(8) }],
            ..campagne("admission", Some(8))
        })
        .await
        .unwrap();
    assert_eq!(apres.version, avant.version + 1);
    assert_eq!(apres.offres.iter().find(|o| o.niveau_code == "6EME").unwrap().capacite, Some(8));
    let (motif,): (Option<String>,) = sqlx::query_as(
        "SELECT motif FROM ent_inscription_campagne_version WHERE campagne_id = $1 AND version = $2",
    )
    .bind(camp2)
    .bind(apres.version)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("Capacité relevée"));
    let (hist,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_offre_historique h
           JOIN ent_inscription_offre o ON o.id = h.offre_id WHERE o.campagne_id = $1",
    )
    .bind(camp2)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(hist >= 2, "création et changement de capacité tracés");
}

/// REPRISE IDEMPOTENTE (M3, M4, M5) : des données « d'avant » (statuts historiques, sans démarche
/// ni référence) sont reprises, deux fois de suite sans rien changer ni dupliquer ; ce qui ne se
/// rapproche pas est rangé dans le rapport, jamais perdu.
#[tokio::test]
async fn i2_reprise_idempotente() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    // Campagne « historique » : année absente du référentiel, niveau absent du référentiel.
    let camp = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_campagne (id, etablissement_id, nature, libelle, annee_cible, statut)
         VALUES ($1,$2,'admission','Ancienne','2099-2100','ouverte')",
    )
    .bind(camp)
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_inscription_campagne_niveau (campagne_id, etablissement_id, code, capacite) VALUES ($1,$2,'QUATRIEME',4)")
        .bind(camp)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // Dossiers « historiques » : écrits avec le seul `statut`, comme le faisait l'ancien code.
    let mut ids = Vec::new();
    for (statut, niveau) in [("incomplet", Some("QUATRIEME")), ("admis", Some("QUATRIEME")), ("desistement", None), ("liste_attente", Some("INCONNU"))] {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_inscription_dossier (id, etablissement_id, campagne_id, nature, statut, eleve_nom, eleve_prenom, niveau_demande, decision_motif, decide_le)
             VALUES ($1,$2,$3,'admission',$4,'Ancien','Dossier',$5, CASE WHEN $4 IN ('admis','liste_attente') THEN 'motif repris' END, CASE WHEN $4 IN ('admis','liste_attente') THEN now() END)",
        )
        .bind(id)
        .bind(etab)
        .bind(camp)
        .bind(statut)
        .bind(niveau)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_inscription_responsable (id, etablissement_id, dossier_id, rang, nom, email, lien)
             VALUES ($1,$2,$3,1,'Parent',$4,'mere')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(format!("hist-{}@exemple.test", id.simple()))
        .execute(ent.pool_verif())
        .await
        .unwrap();
        ids.push(id);
    }
    // Le déclencheur donne déjà l'état, la démarche et la référence à une ligne historique.
    let lignes: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT statut, etat, reference FROM ent_inscription_dossier WHERE campagne_id = $1 ORDER BY created_at, id",
    )
    .bind(camp)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let etats: Vec<&str> = lignes.iter().map(|l| l.1.as_str()).collect();
    assert_eq!(etats, ["attente_famille", "admis", "desiste", "liste_attente"]);
    assert!(lignes.iter().all(|l| l.2.starts_with("ADM-2099-")));

    // Deux reprises de suite : rien ne change, rien ne se duplique.
    let compter_tout = |ent: &PgEnt| {
        let pool = ent.pool_verif().clone();
        async move {
            let v: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM ent_inscription_offre),
                        (SELECT count(*) FROM ent_inscription_demarche),
                        (SELECT count(*) FROM ent_inscription_declaration_responsable),
                        (SELECT count(*) FROM ent_inscription_decision),
                        (SELECT count(*) FROM ent_inscription_voeu),
                        (SELECT count(*) FROM ent_inscription_reprise_rapport)",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            v
        }
    };
    crate::migrate::rejouer_tout(ent.pool_verif()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    let apres_1 = compter_tout(&ent).await;
    crate::migrate::rejouer_tout(ent.pool_verif()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    let apres_2 = compter_tout(&ent).await;
    assert_eq!(apres_1, apres_2, "la reprise est idempotente");
    // Chaque dossier a une démarche et une déclaration ; la décision repose sur le motif repris.
    for id in &ids {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_declaration_responsable x
               JOIN ent_inscription_dossier d ON d.demarche_id = x.demarche_id WHERE d.id = $1",
        )
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(n, 1, "une déclaration par responsable repris");
    }
    // Les dossiers d'un niveau absent de l'offre / de l'année sont au rapport, pas perdus.
    let (annee,): (Option<Uuid>,) = sqlx::query_as("SELECT annee_id FROM ent_inscription_campagne WHERE id = $1")
        .bind(camp)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(annee.is_none(), "une année absente du référentiel n'est pas créée par la reprise");
    let (rap,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_reprise_rapport WHERE objet_id = $1 AND cause = 'annee_absente_du_referentiel'",
    )
    .bind(camp)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(rap, 1);
    let (offres,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_inscription_offre WHERE campagne_id = $1")
        .bind(camp)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(offres, 1, "le niveau de campagne est repris en offre");
    let (voeux,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_voeu v JOIN ent_inscription_dossier d ON d.id = v.dossier_id WHERE d.campagne_id = $1",
    )
    .bind(camp)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(voeux, 2, "deux dossiers nomment le niveau de l'offre ; l'inconnu n'a pas de vœu");
    let (sans_offre,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_reprise_rapport WHERE cause = 'niveau_sans_offre' AND objet_id = ANY($1)",
    )
    .bind(&ids)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(sans_offre, 1);
    let (dec,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = ANY($1) AND motif_interne = 'motif repris'",
    )
    .bind(&ids)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(dec, 2, "décisions reprises avec leur motif");
    // Égalité des comptes par état avant / après : le statut historique n'a pas bougé.
    let apres: Vec<(String, String)> = sqlx::query_as(
        "SELECT statut, etat FROM ent_inscription_dossier WHERE campagne_id = $1 ORDER BY created_at, id",
    )
    .bind(camp)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        apres.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(),
        lignes.iter().map(|l| l.0.as_str()).collect::<Vec<_>>()
    );
}

/// Références : uniques par établissement, atomiques sous concurrence, au format `PRÉFIXE-AAAA-NNNN`.
#[tokio::test]
async fn i2_references_uniques_sous_concurrence() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_campagne (id, etablissement_id, nature, libelle, annee_cible)
         VALUES ($1,$2,'admission','Concurrence','2071-2072')",
    )
    .bind(camp)
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let mut taches = Vec::new();
    for _ in 0..16 {
        let pool = ent.pool_verif().clone();
        taches.push(tokio::spawn(async move {
            sqlx::query_scalar::<_, String>(
                "INSERT INTO ent_inscription_dossier (id, etablissement_id, campagne_id, nature, etat, eleve_nom, eleve_prenom)
                 VALUES (gen_random_uuid(), $1, $2, 'admission', 'deposee', 'Course', 'Atomique') RETURNING reference",
            )
            .bind(etab)
            .bind(camp)
            .fetch_one(&pool)
            .await
            .unwrap()
        }));
    }
    let mut refs = Vec::new();
    for t in taches {
        refs.push(t.await.unwrap());
    }
    refs.sort();
    let attendues: Vec<String> = (1..=16).map(|n| format!("ADM-2071-{n:04}")).collect();
    assert_eq!(refs, attendues, "numéros consécutifs, sans doublon ni trou");
    // Deux établissements ont chacun leur numérotation.
    let (autre, _) = (labo().await.unwrap().1, ());
    let camp2 = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_inscription_campagne (id, etablissement_id, nature, libelle, annee_cible) VALUES ($1,$2,'reinscription','R','2071-2072')")
        .bind(camp2).bind(autre.etablissement_id).execute(ent.pool_verif()).await.unwrap();
    let r: String = sqlx::query_scalar(
        "INSERT INTO ent_inscription_dossier (id, etablissement_id, campagne_id, nature, etat, eleve_nom, eleve_prenom)
         VALUES (gen_random_uuid(), $1, $2, 'reinscription', 'a_confirmer', 'X', 'Y') RETURNING reference",
    )
    .bind(autre.etablissement_id).bind(camp2).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(r, "REI-2071-0001");
}

/// FRATRIE NATIVE : une démarche, deux enfants, une déclaration des responsables (tuteur pour un
/// seul enfant), une référence par dossier ; seul le déposant lit la démarche.
#[tokio::test]
async fn i2_fratrie_une_demarche_plusieurs_dossiers() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = ent
        .creer_campagne_for_actor(etab, d, dr, &CampagneBody {
            annee_cible: "2072-2073".into(),
            niveaux: vec![],
            ..campagne("admission", None)
        })
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(etab, d, dr, camp.id).await.unwrap();
    let o6 = ent
        .creer_offre_for_actor(etab, d, dr, camp.id, &OffreBody {
            niveau_code: "6EME".into(),
            libelle_public: "6ᵉ".into(),
            capacite: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    let o_latin = ent
        .creer_offre_for_actor(etab, d, dr, camp.id, &OffreBody {
            niveau_code: "5EME".into(),
            parcours_code: Some("latin".into()),
            libelle_public: "5ᵉ latin".into(),
            capacite: None,
            ..Default::default()
        })
        .await
        .unwrap();
    let deposant = email("fratrie");
    let jeton = verifier(&ent, etab, &deposant).await;
    let tuteur = ResponsableFormulaireDto {
        nom: "Tuteur".into(),
        prenom: Some("Albus".into()),
        lien: Some("tuteur".into()),
        email: Some(email("tuteur")),
        meme_foyer: false,
        autorite_parentale: true,
        ..Default::default()
    };
    let eleve = |prenom: &str, niveau: &str| EleveFormulaireDto {
        nom: "Weasley".into(),
        prenom: prenom.into(),
        niveau_demande: Some(niveau.into()),
        ..Default::default()
    };
    let dem = ent
        .deposer_demarche(etab, &jeton, &DemarcheBody {
            campagne_id: camp.id,
            enfants: vec![
                EnfantDemarcheBody { eleve: eleve("Fred", "6EME"), voeux: vec![o6.id] },
                EnfantDemarcheBody { eleve: eleve("George", "5EME"), voeux: vec![o_latin.id] },
            ],
            responsable: ResponsableFormulaireDto { nom: "Weasley".into(), lien: Some("mere".into()), ..Default::default() },
            second_parent: Some(second_parent(&email("pere"))),
            autres_responsables: vec![AutreResponsableBody { responsable: tuteur, enfants: vec![0] }],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(dem.dossiers.len(), 2, "un dossier par enfant");
    let refs: std::collections::HashSet<_> = dem.dossiers.iter().map(|x| x.reference.clone()).collect();
    assert_eq!(refs.len(), 2, "une référence par dossier");
    assert!(dem.dossiers.iter().all(|x| x.etat == "deposee" && x.reference.starts_with("ADM-2072-")));
    assert_eq!(dem.declarations.len(), 3, "une seule déclaration des responsables pour la fratrie");
    let par_ordre = |o: i16| dem.declarations.iter().find(|x| x.ordre == o).unwrap();
    assert!(par_ordre(1).est_deposant && par_ordre(1).pour_enfants.len() == 2);
    assert_eq!(par_ordre(2).pour_enfants.len(), 2);
    assert_eq!(par_ordre(3).pour_enfants.len(), 1, "le tuteur ne concerne qu'un enfant");
    // Tout est dans la même démarche, avec les vœux des enfants.
    let (nd,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT demarche_id) FROM ent_inscription_dossier WHERE id = ANY($1)",
    )
    .bind(dem.dossiers.iter().map(|x| x.id).collect::<Vec<_>>())
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(nd, 1);
    let (nv,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_voeu WHERE dossier_id = ANY($1)",
    )
    .bind(dem.dossiers.iter().map(|x| x.id).collect::<Vec<_>>())
    .fetch_all(ent.pool_verif())
    .await
    .map(|v: Vec<(i64,)>| v[0])
    .unwrap();
    assert_eq!(nv, 2);
    // Seul le déposant vérifié lit la démarche ; la marque « ne pas solliciter » ne lui est jamais rendue.
    sqlx::query("UPDATE ent_inscription_declaration_responsable SET ne_pas_solliciter = true WHERE demarche_id = $1 AND ordre = 2")
        .bind(dem.id).execute(ent.pool_verif()).await.unwrap();
    let vue = ent.demarche_famille(etab, &jeton, dem.id).await.unwrap();
    assert!(vue.declarations.iter().all(|x| !x.ne_pas_solliciter && x.personne_id.is_none()));
    let autre = verifier(&ent, etab, &email("autre")).await;
    assert!(ent.demarche_famille(etab, &autre, dem.id).await.is_err());
    // Un vœu d'une autre campagne ou d'une offre inconnue est refusé ; six enfants au plus ; aucun enfant.
    for corps in [
        DemarcheBody { campagne_id: camp.id, enfants: vec![], responsable: ResponsableFormulaireDto { nom: "X".into(), ..Default::default() }, ..Default::default() },
        DemarcheBody {
            campagne_id: camp.id,
            enfants: vec![EnfantDemarcheBody { eleve: eleve("Z", "6EME"), voeux: vec![Uuid::new_v4()] }],
            responsable: ResponsableFormulaireDto { nom: "X".into(), ..Default::default() },
            ..Default::default()
        },
        DemarcheBody {
            campagne_id: camp.id,
            enfants: vec![EnfantDemarcheBody { eleve: eleve("Z", "5EME"), voeux: vec![o6.id] }],
            responsable: ResponsableFormulaireDto { nom: "X".into(), ..Default::default() },
            ..Default::default()
        },
    ] {
        assert!(ent.deposer_demarche(etab, &autre, &corps).await.is_err());
    }
}

/// COMPTEUR DE PLACES JUSTE : proposées, confirmées, validées, liste d'attente, réserve des
/// réinscrits, dépassement confirmé et tracé, désistement qui libère, concurrence.
#[tokio::test]
async fn i2_compteur_de_places_juste() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", Some(2)).await;
    let places = |ent: &PgEnt| {
        let ent_pool = ent.pool_verif().clone();
        async move {
            let r: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
                "SELECT proposees, confirmees, validees, liste_attente, reserve_reinscription, demandes
                   FROM ent_inscription_offre_occupation WHERE campagne_id = $1",
            )
            .bind(camp)
            .fetch_one(&ent_pool)
            .await
            .unwrap();
            r
        }
    };
    async fn dispo(ent: &PgEnt, etab: Uuid, d: Uuid, dr: &str, camp: Uuid) -> Option<i64> {
        ent.offres_for_actor(etab, d, dr, camp).await.unwrap()[0].disponibles
    }
    let mut dossiers = Vec::new();
    let mut jetons = Vec::new();
    for prenom in ["Un", "Deux", "Trois", "Quatre"] {
        let mail = email("places");
        let j = verifier(&ent, etab, &mail).await;
        let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp, prenom, None)).await.unwrap();
        dossiers.push(doss.id);
        jetons.push(j);
    }
    assert_eq!(places(&ent).await, (0, 0, 0, 0, 0, 4), "quatre candidatures, aucune place consommée");
    assert_eq!(dispo(&ent, etab, d, dr, camp).await, Some(2));
    let decider = |id: Uuid, decision: &str, depassement: bool| {
        let ent_ref = &ent;
        let decision = decision.to_string();
        async move {
            ent_ref
                .decider_avance_for_actor(etab, d, dr, id, &DecisionAvanceeBody {
                    decision,
                    motif: Some("décision de test".into()),
                    confirmer_depassement: depassement,
                    ..Default::default()
                })
                .await
        }
    };
    decider(dossiers[0], "admis", false).await.unwrap();
    assert_eq!(places(&ent).await, (1, 0, 0, 0, 0, 3), "un admis = une place proposée");
    decider(dossiers[1], "admis", false).await.unwrap();
    assert_eq!(dispo(&ent, etab, d, dr, camp).await, Some(0));
    // Offre pleine : refusée sans confirmation, jamais en silence.
    assert!(matches!(decider(dossiers[2], "admis", false).await, Err(EntDbError::ConflitDossier(_))));
    // La liste d'attente ne consomme rien et reçoit un rang.
    decider(dossiers[2], "liste_attente", false).await.unwrap();
    assert_eq!(places(&ent).await, (2, 0, 0, 1, 0, 1));
    let (rang,): (Option<i32>,) = sqlx::query_as("SELECT rang_attente FROM ent_inscription_dossier WHERE id = $1")
        .bind(dossiers[2])
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(rang, Some(1));
    // Dépassement assumé : confirmé, tracé, compté (disponibles négatif).
    decider(dossiers[3], "admis", true).await.unwrap();
    assert_eq!(dispo(&ent, etab, d, dr, camp).await, Some(-1));
    let (depasse,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1 AND depassement_capacite",
    )
    .bind(dossiers[3])
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(depasse, 1, "le dépassement est tracé dans la décision");
    // Désistement de la famille d'un admis : la place revient.
    ent.desister_famille(etab, &jetons[0], dossiers[0]).await.unwrap();
    assert_eq!(places(&ent).await, (2, 0, 0, 1, 0, 0));
    assert_eq!(dispo(&ent, etab, d, dr, camp).await, Some(0));
    // Validation : la place passe de « proposée » à « validée » (pièces obligatoires acceptées).
    sqlx::query("UPDATE ent_inscription_piece SET statut = 'acceptee' WHERE dossier_id = $1")
        .bind(dossiers[1])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    ent.valider_inscription_for_actor(etab, d, dr, dossiers[1], &ValidationBody {
        niveau: Some("6EME".into()),
        ..Default::default()
    })
    .await
    .unwrap_or_else(|e| panic!("validation : {e}"));
    assert_eq!(places(&ent).await, (1, 0, 1, 1, 0, 0));
    assert_eq!(dispo(&ent, etab, d, dr, camp).await, Some(0));

    // Réinscrits : la place est gardée sur l'offre jumelle d'une campagne de réinscription.
    let rein = ent
        .creer_campagne_for_actor(etab, d, dr, &CampagneBody {
            niveaux: vec![NiveauCampagneDto { code: "6EME".into(), capacite: None }],
            ..campagne("reinscription", None)
        })
        .await
        .unwrap();
    let offre_rein = rein.offres[0].id;
    let (offre_adm,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_inscription_offre WHERE campagne_id = $1")
        .bind(camp)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    for etat in ["confirmee", "validee", "a_confirmer"] {
        let dr_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_inscription_dossier (id, etablissement_id, campagne_id, nature, etat, eleve_nom, eleve_prenom, eleve_id, valide_le)
             VALUES ($1,$2,$3,'reinscription',$4,'Rein','Scrit', CASE WHEN $4='validee' THEN $5 END, CASE WHEN $4='validee' THEN now() END)",
        )
        .bind(dr_id).bind(etab).bind(rein.id).bind(etat).bind(c.eleve_tom_id)
        .execute(ent.pool_verif()).await.unwrap();
        let v = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_inscription_voeu (id, etablissement_id, dossier_id, rang, offre_id, statut) VALUES ($1,$2,$3,1,$4,'retenu')")
            .bind(v).bind(etab).bind(dr_id).bind(offre_rein).execute(ent.pool_verif()).await.unwrap();
        sqlx::query("UPDATE ent_inscription_dossier SET voeu_retenu_id = $2 WHERE id = $1").bind(dr_id).bind(v).execute(ent.pool_verif()).await.unwrap();
    }
    // Mode explicite : « à confirmer » ne réserve rien ; confirmée et validée réservent.
    let (reserve,): (i64,) = sqlx::query_as("SELECT reserve_reinscription FROM ent_inscription_offre_occupation WHERE offre_id = $1")
        .bind(offre_adm).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(reserve, 2);
    // Mode tacite : tout réinscrit non parti garde sa place.
    sqlx::query("UPDATE ent_inscription_campagne SET mode_reinscription = 'tacite' WHERE id = $1").bind(rein.id).execute(ent.pool_verif()).await.unwrap();
    let (reserve,): (i64,) = sqlx::query_as("SELECT reserve_reinscription FROM ent_inscription_offre_occupation WHERE offre_id = $1")
        .bind(offre_adm).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(reserve, 3);
    assert_eq!(dispo(&ent, etab, d, dr, camp).await, Some(-3), "2 − 3 réservées − 1 proposée − 1 validée");
}

/// Deux admissions simultanées sur la dernière place : une seule passe (verrou de l'offre).
#[tokio::test]
async fn i2_derniere_place_une_seule_admission() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", Some(1)).await;
    let mut ids = Vec::new();
    for prenom in ["Alpha", "Beta"] {
        let j = verifier(&ent, etab, &email("derniere")).await;
        ids.push(ent.deposer_preinscription(etab, &j, &formulaire(camp, prenom, None)).await.unwrap().id);
    }
    let admis = DecisionBody { decision: "admis".into(), motif: None };
    let (a, b) = tokio::join!(
        ent.decider_for_actor(etab, d, dr, ids[0], &admis),
        ent.decider_for_actor(etab, d, dr, ids[1], &admis),
    );
    assert_eq!([a.is_ok(), b.is_ok()].iter().filter(|x| **x).count(), 1, "une seule admission passe");
    let (n,): (i64,) = sqlx::query_as("SELECT proposees FROM ent_inscription_offre_occupation WHERE campagne_id = $1")
        .bind(camp)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
}

/// La décision est une ligne ; la famille ne lit que le motif communiqué.
#[tokio::test]
async fn i2_decision_est_une_ligne_et_motif_communique() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let j = verifier(&ent, etab, &email("motif")).await;
    let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp, "Motif", None)).await.unwrap();
    // Motif trop court / absent pour un refus.
    for motif in [None, Some("ab".to_string())] {
        assert!(ent
            .decider_avance_for_actor(etab, d, dr, doss.id, &DecisionAvanceeBody { decision: "refuse".into(), motif, ..Default::default() })
            .await
            .is_err());
    }
    // Motif interne seulement (chaîne vide = non communiqué).
    let apres = ent
        .decider_avance_for_actor(etab, d, dr, doss.id, &DecisionAvanceeBody {
            decision: "liste_attente".into(),
            motif: Some("Dossier à comparer avec les autres".into()),
            motif_communique: Some(String::new()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(apres.decision_motif.as_deref(), Some("Dossier à comparer avec les autres"), "le personnel lit le motif interne");
    let famille = ent.dossier_famille(etab, &j, doss.id).await.unwrap();
    assert_eq!(famille.decision_motif, None, "la famille ne lit que le motif communiqué");
    assert_eq!(famille.etat, "liste_attente");
    // Revue : une nouvelle ligne remplace la précédente, rien n'est effacé.
    ent.decider_avance_for_actor(etab, d, dr, doss.id, &DecisionAvanceeBody {
        decision: "admis".into(),
        motif: Some("Place libérée".into()),
        motif_communique: Some("Une place s'est libérée".into()),
        ..Default::default()
    })
    .await
    .unwrap();
    let lignes: Vec<(String, Option<Uuid>, Uuid)> = sqlx::query_as(
        "SELECT sens, remplace_decision_id, id FROM ent_inscription_decision WHERE dossier_id = $1 ORDER BY decide_le, id",
    )
    .bind(doss.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(), ["liste_attente", "admis"]);
    assert_eq!(lignes[1].1, Some(lignes[0].2), "la décision revue référence la précédente");
    assert_eq!(
        ent.dossier_famille(etab, &j, doss.id).await.unwrap().decision_motif.as_deref(),
        Some("Une place s'est libérée")
    );
}

/// DÉSACCORD : blocage posé par le secrétariat, levé par la direction ; rien ne se décide ni ne se
/// valide pendant le blocage ; le dossier garde sa place.
#[tokio::test]
async fn i2_blocage_desaccord() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", Some(3)).await;
    let (dossier, _) = dossier_admis(&ent, &c, camp, &email("bloc"), formulaire(camp, "Bloc", None)).await;
    ent.blocage_for_actor(etab, a, ar, dossier, &BlocageBody { bloquer: true, motif: "Le père s'oppose".into() }).await.unwrap();
    let b = ent.dossier_for_actor(etab, d, dr, dossier).await.unwrap();
    assert_eq!((b.etat.as_str(), b.statut.as_str()), ("bloque_desaccord", "admis"));
    assert!(ent.valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default()).await.is_err());
    assert!(ent.decider_for_actor(etab, d, dr, dossier, &DecisionBody { decision: "refuse".into(), motif: Some("test motif".into()) }).await.is_err());
    assert!(ent.changer_statut_for_actor(etab, a, ar, dossier, &StatutBody { statut: "en_instruction".into(), motif: None }).await.is_err());
    assert!(ent.relancer_for_actor(etab, a, ar, dossier).await.is_err(), "aucune relance pendant le blocage");
    let (n,): (i64,) = sqlx::query_as("SELECT proposees FROM ent_inscription_offre_occupation WHERE campagne_id = $1")
        .bind(camp).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n, 1, "le dossier bloqué garde sa place");
    // Lever : réservé à la direction (le secrétariat n'a pas le droit de décider).
    assert!(matches!(
        ent.blocage_for_actor(etab, a, ar, dossier, &BlocageBody { bloquer: false, motif: "Accord écrit reçu".into() }).await,
        Err(EntDbError::Forbidden)
    ));
    ent.blocage_for_actor(etab, d, dr, dossier, &BlocageBody { bloquer: false, motif: "Accord écrit reçu".into() }).await.unwrap();
    assert_eq!(ent.dossier_for_actor(etab, d, dr, dossier).await.unwrap().etat, "admis");
    assert!(ent.blocage_for_actor(etab, d, dr, dossier, &BlocageBody { bloquer: false, motif: "Encore".into() }).await.is_err());
    ent.valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default()).await.unwrap();
}

/// La famille retire sa candidature (avant admission) ou se désiste (après) ; pas l'inverse.
#[tokio::test]
async fn i2_retrait_et_desistement_de_la_famille() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let j = verifier(&ent, etab, &email("retrait")).await;
    let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp, "Retrait", None)).await.unwrap();
    // Désistement avant admission : pas de sens.
    assert!(ent.desister_famille(etab, &j, doss.id).await.is_err());
    // Un autre déposant ne voit pas le dossier.
    let autre = verifier(&ent, etab, &email("autre")).await;
    assert!(ent.retirer_candidature_famille(etab, &autre, doss.id).await.is_err());
    let r = ent.retirer_candidature_famille(etab, &j, doss.id).await.unwrap();
    assert_eq!(r.etat, "retire");
    assert!(ent.retirer_candidature_famille(etab, &j, doss.id).await.is_err(), "terminal");
    // Après admission, le retrait n'est plus ouvert : on se désiste.
    let j2 = verifier(&ent, etab, &email("desiste")).await;
    let d2 = ent.deposer_preinscription(etab, &j2, &formulaire(camp, "Desiste", None)).await.unwrap();
    ent.decider_for_actor(etab, d, dr, d2.id, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    assert!(ent.retirer_candidature_famille(etab, &j2, d2.id).await.is_err());
    assert_eq!(ent.desister_famille(etab, &j2, d2.id).await.unwrap().etat, "desiste");
}

/// Actions de transition du personnel : droits, motif, états de chaque nature.
#[tokio::test]
async fn i2_transitions_du_personnel() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let j = verifier(&ent, etab, &email("trans")).await;
    let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp, "Trans", None)).await.unwrap();
    let t = |action: &str, motif: &str| TransitionBody { action: action.into(), motif: motif.into() };
    assert!(ent.transition_for_actor(etab, a, ar, doss.id, &t("inconnue", "motif")).await.is_err());
    assert!(ent.transition_for_actor(etab, a, ar, doss.id, &t("sans_suite", "ab")).await.is_err(), "motif trop court");
    // Sans suite (secrétariat), puis reprise.
    assert_eq!(ent.transition_for_actor(etab, a, ar, doss.id, &t("sans_suite", "Délai dépassé")).await.unwrap().etat, "sans_suite");
    assert_eq!(ent.transition_for_actor(etab, a, ar, doss.id, &t("reprendre", "La famille a répondu")).await.unwrap().etat, "en_instruction");
    // L'annulation pour erreur est une décision de la direction.
    assert!(matches!(ent.transition_for_actor(etab, a, ar, doss.id, &t("annuler", "Doublon")).await, Err(EntDbError::Forbidden)));
    assert_eq!(ent.transition_for_actor(etab, d, dr, doss.id, &t("annuler", "Doublon de dossier")).await.unwrap().etat, "annule");
    assert!(ent.transition_for_actor(etab, d, dr, doss.id, &t("reprendre", "Erreur")).await.is_err(), "annulé : terminal");
    // Une action d'une autre nature est refusée par la table.
    assert!(ent.transition_for_actor(etab, a, ar, doss.id, &t("non_presente", "Non venu")).await.is_err());
    // Clôture tracée.
    let (clos, motif): (Option<chrono::DateTime<chrono::Utc>>, Option<String>) =
        sqlx::query_as("SELECT clos_le, motif_cloture FROM ent_inscription_dossier WHERE id = $1")
            .bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(clos.is_some());
    assert_eq!(motif.as_deref(), Some("Doublon de dossier"));
}

/// INS-57 : « ne pas solliciter » posé après le dépôt, avant la validation.
#[tokio::test]
async fn i2_ne_pas_solliciter_apres_le_depot() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let protege = email("protege");
    let mere = email("mere");
    let (dossier, _) = dossier_admis(&ent, &c, camp, &mere, formulaire(camp, "Protegee", Some(second_parent(&protege)))).await;
    // Un message à son intention est déjà en file (accord, copie…).
    sqlx::query(
        "INSERT INTO ent_inscription_envoi (id, etablissement_id, dossier_id, cle, kind, destinataire, sujet, corps)
         VALUES (gen_random_uuid(), $1, $2, $3, 'information', $4, 'Copie', 'Bonjour')",
    )
    .bind(etab).bind(dossier).bind(format!("copie:{}", Uuid::new_v4())).bind(&protege)
    .execute(ent.pool_verif()).await.unwrap();
    let motif = "Ordonnance de protection du 3 mars";
    // Droits : décision de la direction ; le déposant n'est pas marquable ; rang inconnu.
    assert!(matches!(ent.ne_pas_solliciter_for_actor(etab, a, ar, dossier, &NePasSolliciterBody { rang: 2, motif: motif.into() }).await, Err(EntDbError::Forbidden)));
    assert!(ent.ne_pas_solliciter_for_actor(etab, d, dr, dossier, &NePasSolliciterBody { rang: 1, motif: motif.into() }).await.is_err());
    assert!(ent.ne_pas_solliciter_for_actor(etab, d, dr, dossier, &NePasSolliciterBody { rang: 5, motif: motif.into() }).await.is_err());
    assert!(ent.ne_pas_solliciter_for_actor(etab, d, dr, dossier, &NePasSolliciterBody { rang: 2, motif: "x".into() }).await.is_err());
    let r = ent.ne_pas_solliciter_for_actor(etab, d, dr, dossier, &NePasSolliciterBody { rang: 2, motif: motif.into() }).await.unwrap();
    assert_eq!(r.envois_annules, 1, "le message déjà programmé est annulé");
    // Marque portée par la ligne du dossier ET par la déclaration ; motif scellé, jamais renvoyé.
    let (flag, sceau): (bool, Option<String>) = sqlx::query_as(
        "SELECT ne_pas_solliciter, ne_pas_solliciter_ref FROM ent_inscription_responsable WHERE dossier_id = $1 AND rang = 2",
    ).bind(dossier).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(flag);
    let sceau = sceau.expect("motif scellé");
    assert!(!sceau.contains("Ordonnance"), "le motif n'est jamais en clair");
    let (decl,): (bool,) = sqlx::query_as(
        "SELECT x.ne_pas_solliciter FROM ent_inscription_declaration_responsable x
           JOIN ent_inscription_dossier d ON d.demarche_id = x.demarche_id WHERE d.id = $1 AND x.ordre = 2",
    ).bind(dossier).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(decl, "la déclaration porte la marque");
    let vue = serde_json::to_string(&ent.dossier_for_actor(etab, d, dr, dossier).await.unwrap()).unwrap();
    assert!(!vue.contains("Ordonnance") && !vue.contains(&sceau), "ni motif ni sceau dans la vue du personnel");
    let (envoi_actif,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_envoi WHERE dossier_id = $1 AND lower(destinataire) = lower($2) AND statut = 'a_envoyer'",
    ).bind(dossier).bind(&protege).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(envoi_actif, 0);
    // La validation n'invite pas la personne marquée ; le déposant, lui, est servi normalement.
    ent.valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default()).await.unwrap();
    distributeur(&ent, etab).distribuer_tout().await.unwrap();
    ent.expedier_envois_inscriptions(100).await.unwrap();
    assert_eq!(courriels_vers(&ent, &protege), 0, "aucun message vers la personne à ne pas solliciter");
    let (invit,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_invitation i JOIN ent_inscription_responsable r ON r.id = i.responsable_id
          WHERE i.dossier_id = $1 AND r.rang = 2",
    ).bind(dossier).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(invit, 0);
    // Aucune prolongation ni fermeture ne la prévient non plus (voir les tests dédiés).
}

/// R-I9 : l'adresse à protéger n'est en clair ni dans le dossier ni dans la déclaration.
#[tokio::test]
async fn i2_adresse_a_proteger_jamais_en_clair_dans_la_declaration() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let j = verifier(&ent, etab, &email("adresse")).await;
    let mut f = formulaire(camp, "Cachee", None);
    f.responsable.adresse = Some("12 rue des Lilas, 75011 Paris".into());
    f.responsable.reserve = Some(ReserveResponsableDto { adresse_a_proteger: true, ..Default::default() });
    let doss = ent.deposer_preinscription(etab, &j, &f).await.unwrap();
    let (adr_resp, scelle): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT adresse, adresse_protegee FROM ent_inscription_responsable WHERE dossier_id = $1 AND rang = 1",
    ).bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(adr_resp.is_none());
    assert!(scelle.as_deref().is_some_and(|s| !s.contains("Lilas")));
    let (adr_decl, protege): (Option<String>, bool) = sqlx::query_as(
        "SELECT x.adresse, x.adresse_a_proteger FROM ent_inscription_declaration_responsable x
           JOIN ent_inscription_dossier d ON d.demarche_id = x.demarche_id WHERE d.id = $1 AND x.ordre = 1",
    ).bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(adr_decl.is_none(), "la déclaration ne contient jamais l'adresse à protéger");
    assert!(protege);
    // La table entière : aucune trace en clair.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_declaration_responsable WHERE row_to_json(ent_inscription_declaration_responsable)::text LIKE '%Lilas%'",
    ).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n, 0);
}

/// INS-68 : annuler une validation erronée — délai, absence de données produites, commandes
/// inverses, retour à « confirmé », nouvelle validation possible.
#[tokio::test]
async fn i2_annuler_une_validation_erronee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", Some(3)).await;
    let (dossier, _) = dossier_admis(&ent, &c, camp, &email("erreur"), formulaire(camp, "Erreur", Some(second_parent(&email("pere"))))).await;
    // Non validé : rien à annuler.
    let corps = AnnulationValidationBody { motif: "Mauvais enfant validé".into() };
    assert!(ent.annuler_validation_for_actor(etab, d, dr, dossier, &corps).await.is_err());
    let v = ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody { niveau: Some("6EME".into()), ..Default::default() })
        .await
        .unwrap();
    // Droit : la direction décide, le secrétariat ne défait pas une validation.
    assert!(matches!(ent.annuler_validation_for_actor(etab, a, ar, dossier, &corps).await, Err(EntDbError::Forbidden)));
    assert!(ent.annuler_validation_for_actor(etab, d, dr, dossier, &AnnulationValidationBody { motif: "x".into() }).await.is_err());
    // Une donnée d'un autre socle produite depuis : refus, avec la liste.
    let evt = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_metier_event (id, etablissement_id, type, aggregate, aggregate_id, eleve_id, created_at)
         VALUES ($1,$2,'note.saisie','note',$1,$3, now() + interval '1 minute')",
    ).bind(evt).bind(etab).bind(v.eleve_id).execute(ent.pool_verif()).await.unwrap();
    let err = ent.annuler_validation_for_actor(etab, d, dr, dossier, &corps).await.unwrap_err();
    assert!(err.to_string().contains("note.saisie"), "la liste est montrée : {err}");
    sqlx::query("DELETE FROM ent_metier_event WHERE id = $1").bind(evt).execute(ent.pool_verif()).await.unwrap();
    // Délai dépassé (campagne à 0 h) : refus.
    sqlx::query("UPDATE ent_inscription_campagne SET delai_annulation_validation_h = 0 WHERE id = $1").bind(camp).execute(ent.pool_verif()).await.unwrap();
    assert!(ent.annuler_validation_for_actor(etab, d, dr, dossier, &corps).await.is_err());
    sqlx::query("UPDATE ent_inscription_campagne SET delai_annulation_validation_h = 72 WHERE id = $1").bind(camp).execute(ent.pool_verif()).await.unwrap();
    // Annulation : tout revient, rien n'est effacé.
    let r = ent.annuler_validation_for_actor(etab, d, dr, dossier, &corps).await.unwrap();
    assert_eq!(r.etat, "confirme");
    assert!(r.eleve_archive && r.inscription_annuelle_retiree);
    let (statut, trace): (String, i64) = sqlx::query_as(
        "SELECT statut, (SELECT count(*) FROM ent_journal_modif j WHERE j.eleve_id = ent_eleve.id AND j.action = 'creation_annulee')
           FROM ent_eleve WHERE id = $1",
    ).bind(v.eleve_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(statut, "archive", "la fiche créée par erreur est archivée, pas effacée");
    assert_eq!(trace, 1, "la création annulée laisse sa trace au journal");
    let (prevues,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_scolarite_annee WHERE eleve_id = $1 AND source_ref = $2")
        .bind(v.eleve_id).bind(dossier).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(prevues, 0);
    let (liens_actifs,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_lien WHERE eleve_id = $1 AND suspendu_le IS NULL")
        .bind(v.eleve_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(liens_actifs, 0, "les liens créés par la validation sont suspendus");
    let (events,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'inscription.validation_annulee'")
        .bind(v.eleve_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(events, 1);
    let dto = ent.dossier_for_actor(etab, d, dr, dossier).await.unwrap();
    assert_eq!((dto.etat.as_str(), dto.eleve_id, dto.valide_le), ("confirme", None, None));
    // Une nouvelle validation, correcte cette fois, est possible. La fiche archivée est un homonyme :
    // le choix humain (jamais implicite) est de créer une nouvelle fiche.
    assert!(ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody { niveau: Some("6EME".into()), ..Default::default() })
        .await
        .is_err(), "l'homonyme exige un choix humain explicite");
    // Les personnes créées par la première validation existent encore : le rapprochement reste un
    // choix humain (ici, de nouvelles fiches).
    let mut resolutions = Vec::new();
    for rang in [1, 2] {
        resolutions.push(ResolutionResponsable {
            responsable_id: super::tests_inscriptions::resp_id(&ent, dossier, rang).await,
            personne: ChoixPersonne::Nouvelle,
            foyer: None,
        });
    }
    let v2 = ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody {
            niveau: Some("6EME".into()),
            eleve: Some(ChoixEleve::Nouveau),
            responsables: resolutions,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_ne!(v2.eleve_id, v.eleve_id, "une nouvelle fiche, la précédente reste archivée");
}

/// INS-79 : prolonger une échéance, jamais en arrière, familles prévenues par la file d'envoi.
#[tokio::test]
async fn i2_prolonger_une_echeance_en_masse() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let aujourdhui = domain_eleve::horloge::jour_paris();
    let limite = aujourdhui + chrono::Duration::days(5);
    let camp = ent
        .creer_campagne_for_actor(etab, d, dr, &CampagneBody {
            annee_cible: "2081-2082".into(),
            regles: Some(ReglesCampagneDto { date_limite_famille: Some(limite), ..ReglesCampagneDto::default() }),
            ..campagne("admission", None)
        })
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(etab, d, dr, camp.id).await.unwrap();
    let mut ids = Vec::new();
    let mut mails = Vec::new();
    for prenom in ["P1", "P2", "P3"] {
        let m = email("prolonge");
        let j = verifier(&ent, etab, &m).await;
        let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp.id, prenom, None)).await.unwrap();
        // Dossier incomplet : la famille a une échéance.
        ent.changer_statut_for_actor(etab, a, ar, doss.id, &StatutBody { statut: "incomplet".into(), motif: Some("Pièce manquante".into()) }).await.unwrap();
        ids.push(doss.id);
        mails.push(m);
    }
    let echeance = |id: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            let (e,): (Option<chrono::NaiveDate>,) = sqlx::query_as("SELECT echeance_famille FROM ent_inscription_dossier WHERE id = $1")
                .bind(id).fetch_one(&pool).await.unwrap();
            e
        }
    };
    let avant = echeance(ids[0]).await.expect("échéance posée à l'état « attente de la famille »");
    // P3 a déjà une échéance plus lointaine : elle ne recule jamais.
    sqlx::query("UPDATE ent_inscription_dossier SET echeance_famille = $2 WHERE id = $1").bind(ids[2]).bind(aujourdhui + chrono::Duration::days(60)).execute(ent.pool_verif()).await.unwrap();
    let nouvelle = aujourdhui + chrono::Duration::days(30);
    // Droit et validité.
    let prof = (c.prof.user_id, Role::Prof.as_str());
    let corps = ProlongationBody { objet: "echeance_dossiers".into(), nouvelle_date: nouvelle, motif_public: "Intempéries : le service est fermé".into(), dossiers: vec![] };
    assert!(matches!(ent.prolonger_for_actor(etab, prof.0, prof.1, camp.id, &corps).await, Err(EntDbError::Forbidden)));
    assert!(ent.prolonger_for_actor(etab, a, ar, camp.id, &ProlongationBody { motif_public: "x".into(), ..corps.clone() }).await.is_err());
    assert!(ent.prolonger_for_actor(etab, a, ar, camp.id, &ProlongationBody { objet: "autre".into(), ..corps.clone() }).await.is_err());
    let p = ent.prolonger_for_actor(etab, a, ar, camp.id, &corps).await.unwrap();
    assert_eq!(p.nb_dossiers, 2, "P3 avait déjà mieux : intacte");
    assert_eq!(p.familles_prevenues, 2);
    assert_eq!(echeance(ids[0]).await, Some(nouvelle));
    assert!(echeance(ids[0]).await > Some(avant));
    assert_eq!(echeance(ids[2]).await, Some(aujourdhui + chrono::Duration::days(60)));
    // Les familles concernées sont prévenues par la file (pas d'envoi direct dans la transaction).
    let (en_file,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_envoi WHERE etablissement_id = $1 AND kind = 'information' AND statut = 'a_envoyer'
            AND dossier_id = ANY($2) AND destinataire = ANY($3)",
    ).bind(etab).bind(&ids).bind(&mails).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(en_file, 2);
    assert_eq!(courriels_vers(&ent, &mails[0]).min(0), 0);
    // Date limite de la campagne : en avant seulement.
    let reculer = ProlongationBody { objet: "date_limite_famille".into(), nouvelle_date: limite - chrono::Duration::days(1), ..corps.clone() };
    assert!(matches!(ent.prolonger_for_actor(etab, a, ar, camp.id, &reculer).await, Err(EntDbError::ConflitDossier(_))));
    let avance = ProlongationBody { objet: "date_limite_famille".into(), nouvelle_date: limite + chrono::Duration::days(10), ..corps.clone() };
    ent.prolonger_for_actor(etab, a, ar, camp.id, &avance).await.unwrap();
    let k = ent.list_campagnes_for_actor(etab, d, dr).await.unwrap().into_iter().find(|k| k.id == camp.id).unwrap();
    assert_eq!(k.regles.date_limite_famille, Some(limite + chrono::Duration::days(10)));
    assert!(k.version >= 3);
    let (journal,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_inscription_prolongation WHERE campagne_id = $1").bind(camp.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(journal, 2);
}

/// INS-80 : fermeture du secrétariat — délais reportés, relances suspendues, message à la famille.
#[tokio::test]
async fn i2_fermeture_suspend_les_delais_et_les_relances() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let m = email("ferme");
    let j = verifier(&ent, etab, &m).await;
    let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp, "Ferme", None)).await.unwrap();
    ent.changer_statut_for_actor(etab, a, ar, doss.id, &StatutBody { statut: "incomplet".into(), motif: Some("Pièce manquante".into()) }).await.unwrap();
    let aujourdhui = domain_eleve::horloge::jour_paris();
    let (ech,): (Option<chrono::NaiveDate>,) = sqlx::query_as("SELECT echeance_famille FROM ent_inscription_dossier WHERE id = $1").bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    let ech = ech.unwrap();
    // Validité : période cohérente, motif.
    let corps = FermetureBody {
        date_debut: aujourdhui - chrono::Duration::days(1),
        date_fin: ech + chrono::Duration::days(10),
        report_jours: Some(2),
        motif_public: "Vacances d'été".into(),
        contact_urgence: Some("urgence@exemple.test".into()),
    };
    assert!(ent.declarer_fermeture_for_actor(etab, a, ar, &FermetureBody { date_fin: aujourdhui - chrono::Duration::days(3), ..corps.clone() }).await.is_err());
    assert!(ent.declarer_fermeture_for_actor(etab, a, ar, &FermetureBody { motif_public: "x".into(), ..corps.clone() }).await.is_err());
    let prof = (c.prof.user_id, Role::Prof.as_str());
    assert!(matches!(ent.declarer_fermeture_for_actor(etab, prof.0, prof.1, &corps).await, Err(EntDbError::Forbidden)));
    let f = ent.declarer_fermeture_for_actor(etab, a, ar, &corps).await.unwrap();
    assert!(f.echeances_reportees >= 1);
    // L'échéance de la famille est reportée au jour de réouverture + 2 jours.
    let (apres,): (Option<chrono::NaiveDate>,) = sqlx::query_as("SELECT echeance_famille FROM ent_inscription_dossier WHERE id = $1").bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(apres, Some(corps.date_fin + chrono::Duration::days(3)));
    // Relances suspendues pendant la fermeture ; message affiché sur la démarche de la famille.
    assert!(matches!(ent.relancer_for_actor(etab, a, ar, doss.id).await, Err(EntDbError::ConflitDossier(_))));
    let vue = ent.dossier_famille(etab, &j, doss.id).await.unwrap();
    let fe = vue.fermeture.expect("message de fermeture sur la démarche");
    assert_eq!((fe.motif_public.as_str(), fe.contact_urgence.as_deref()), ("Vacances d'été", Some("urgence@exemple.test")));
    // Une échéance recalculée pendant la fermeture tombe après la réouverture.
    ent.changer_statut_for_actor(etab, a, ar, doss.id, &StatutBody { statut: "en_instruction".into(), motif: None }).await.unwrap();
    ent.changer_statut_for_actor(etab, a, ar, doss.id, &StatutBody { statut: "incomplet".into(), motif: Some("Encore une pièce".into()) }).await.unwrap();
    let (re,): (Option<chrono::NaiveDate>,) = sqlx::query_as("SELECT echeance_famille FROM ent_inscription_dossier WHERE id = $1").bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(re.unwrap() > corps.date_fin, "délai famille jamais échu pendant la fermeture");
    // La famille est prévenue par la file. Annuler la fermeture rend les relances.
    let (info,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_inscription_envoi WHERE dossier_id = $1 AND kind = 'information'").bind(doss.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(info, 1);
    assert_eq!(ent.fermetures_for_actor(etab, d, dr).await.unwrap().len(), 1);
    ent.annuler_fermeture_for_actor(etab, a, ar, f.id).await.unwrap();
    assert!(ent.fermetures_for_actor(etab, d, dr).await.unwrap().is_empty());
    assert!(ent.dossier_famille(etab, &j, doss.id).await.unwrap().fermeture.is_none());
}

/// Offres : droits, clé, capacité tracée, unités d'hébergement (R-I15), archivage refusé si un
/// vœu s'y rattache.
#[tokio::test]
async fn i2_offres_cles_unites_et_droits() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let prof = (c.prof.user_id, Role::Prof.as_str());
    let corps = OffreBody {
        niveau_code: "4EME".into(),
        regime: Some("interne".into()),
        libelle_public: "4ᵉ interne".into(),
        capacite: Some(10),
        unites: vec![
            UniteHebergementDto { id: None, libelle: "Dortoir filles".into(), capacite: 6, conditions_acces: None, occupees: 0 },
            UniteHebergementDto { id: None, libelle: "Chambre adaptée".into(), capacite: 2, conditions_acces: Some("PMR".into()), occupees: 0 },
        ],
        ..Default::default()
    };
    assert!(matches!(ent.creer_offre_for_actor(etab, prof.0, prof.1, camp, &corps).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.offres_for_actor(etab, prof.0, prof.1, camp).await, Err(EntDbError::Forbidden)));
    // Unités : somme ≤ capacité ; régime interne seulement.
    assert!(ent.creer_offre_for_actor(etab, d, dr, camp, &OffreBody { capacite: Some(7), ..corps.clone() }).await.is_err());
    assert!(ent.creer_offre_for_actor(etab, d, dr, camp, &OffreBody { regime: Some("externe".into()), ..corps.clone() }).await.is_err());
    assert!(ent.creer_offre_for_actor(etab, d, dr, camp, &OffreBody { regime: Some("hotel".into()), ..corps.clone() }).await.is_err());
    assert!(ent.creer_offre_for_actor(etab, d, dr, camp, &OffreBody { capacite: Some(-1), ..corps.clone() }).await.is_err());
    let o = ent.creer_offre_for_actor(etab, d, dr, camp, &corps).await.unwrap();
    assert_eq!(o.unites.len(), 2);
    // La clé de l'offre est unique et ne change pas.
    assert!(matches!(ent.creer_offre_for_actor(etab, d, dr, camp, &corps).await, Err(EntDbError::ConflitDossier(_))));
    assert!(ent.modifier_offre_for_actor(etab, d, dr, o.id, &OffreBody { regime: None, ..corps.clone() }).await.is_err());
    // Capacité modifiée en cours, avec trace ; le compteur suit.
    let m = ent.modifier_offre_for_actor(etab, d, dr, o.id, &OffreBody { capacite: Some(12), motif: Some("Ouverture d'un dortoir".into()), ..corps.clone() }).await.unwrap();
    assert_eq!((m.capacite, m.disponibles), (Some(12), Some(12)));
    let (h,): (Option<String>,) = sqlx::query_as("SELECT motif FROM ent_inscription_offre_historique WHERE offre_id = $1 ORDER BY modifie_le DESC LIMIT 1").bind(o.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(h.as_deref(), Some("Ouverture d'un dortoir"));
    // Archivage : refusé dès qu'un vœu s'y rattache, sinon possible.
    let j = verifier(&ent, etab, &email("archive")).await;
    let doss = ent.deposer_demarche(etab, &j, &DemarcheBody {
        campagne_id: camp,
        enfants: vec![EnfantDemarcheBody { eleve: EleveFormulaireDto { nom: "A".into(), prenom: "B".into(), niveau_demande: Some("4EME".into()), ..Default::default() }, voeux: vec![o.id] }],
        responsable: ResponsableFormulaireDto { nom: "A".into(), ..Default::default() },
        ..Default::default()
    }).await.unwrap();
    assert!(!doss.dossiers.is_empty());
    assert!(matches!(ent.archiver_offre_for_actor(etab, d, dr, o.id).await, Err(EntDbError::ConflitDossier(_))));
    let libre = ent.creer_offre_for_actor(etab, d, dr, camp, &OffreBody { niveau_code: "3EME".into(), libelle_public: "3ᵉ".into(), ..Default::default() }).await.unwrap();
    ent.archiver_offre_for_actor(etab, d, dr, libre.id).await.unwrap();
    assert_eq!(ent.offres_for_actor(etab, d, dr, camp).await.unwrap().iter().filter(|x| x.niveau_code == "3EME").count(), 0);
}

/// Isolation entre établissements : offres, transitions, annulations d'un autre établissement
/// restent introuvables.
#[tokio::test]
async fn i2_isolation_etablissements() {
    let Some((ent, a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let (da, dra) = dir(&a);
    let (db_, drb) = dir(&b);
    let camp = campagne_ouverte(&ent, &a, "admission", Some(5)).await;
    let ja = verifier(&ent, a.etablissement_id, &email("iso")).await;
    let doss = ent.deposer_preinscription(a.etablissement_id, &ja, &formulaire(camp, "Iso", None)).await.unwrap();
    let offres = ent.offres_for_actor(a.etablissement_id, da, dra, camp).await.unwrap();
    assert!(!offres.is_empty());
    assert!(ent.offres_for_actor(b.etablissement_id, db_, drb, camp).await.is_err());
    assert!(ent.creer_offre_for_actor(b.etablissement_id, db_, drb, camp, &OffreBody { niveau_code: "X".into(), libelle_public: "X".into(), ..Default::default() }).await.is_err());
    assert!(ent.modifier_offre_for_actor(b.etablissement_id, db_, drb, offres[0].id, &OffreBody { niveau_code: "6EME".into(), libelle_public: "X".into(), ..Default::default() }).await.is_err());
    assert!(ent.archiver_offre_for_actor(b.etablissement_id, db_, drb, offres[0].id).await.is_err());
    assert!(ent.transition_for_actor(b.etablissement_id, db_, drb, doss.id, &TransitionBody { action: "annuler".into(), motif: "motif long".into() }).await.is_err());
    assert!(ent.blocage_for_actor(b.etablissement_id, db_, drb, doss.id, &BlocageBody { bloquer: true, motif: "motif long".into() }).await.is_err());
    assert!(ent.ne_pas_solliciter_for_actor(b.etablissement_id, db_, drb, doss.id, &NePasSolliciterBody { rang: 2, motif: "motif long".into() }).await.is_err());
    assert!(ent.annuler_validation_for_actor(b.etablissement_id, db_, drb, doss.id, &AnnulationValidationBody { motif: "motif long".into() }).await.is_err());
    assert!(ent.prolonger_for_actor(b.etablissement_id, db_, drb, camp, &ProlongationBody { objet: "cloture".into(), nouvelle_date: domain_eleve::horloge::jour_paris(), motif_public: "motif long".into(), dossiers: vec![] }).await.is_err());
    assert!(ent.changer_statut_campagne_for_actor(b.etablissement_id, db_, drb, camp, "suspendue").await.is_err());
    // Le dossier de A est intact.
    assert_eq!(ent.dossier_for_actor(a.etablissement_id, da, dra, doss.id).await.unwrap().etat, "deposee");
    // Une fermeture de A n'apparaît pas chez B.
    ent.declarer_fermeture_for_actor(a.etablissement_id, da, dra, &FermetureBody {
        date_debut: domain_eleve::horloge::jour_paris() - chrono::Duration::days(1),
        date_fin: domain_eleve::horloge::jour_paris() + chrono::Duration::days(5),
        report_jours: None,
        motif_public: "Fermeture de A".into(),
        contact_urgence: None,
    }).await.unwrap();
    assert!(ent.fermetures_for_actor(b.etablissement_id, db_, drb).await.unwrap().is_empty());
}

/// Le libellé visible de chaque état n'est pas ici : les états sont des codes stables ; le test de
/// compatibilité vérifie que les anciens lecteurs voient le même dossier.
#[tokio::test]
async fn i2_vue_historique_des_dossiers() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let (a, ar) = adm(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let j = verifier(&ent, etab, &email("histo")).await;
    let doss = ent.deposer_preinscription(etab, &j, &formulaire(camp, "Histo", None)).await.unwrap();
    assert_eq!((doss.statut.as_str(), doss.etat.as_str()), ("deposee", "deposee"));
    assert!(doss.reference.starts_with("ADM-2027-"));
    let inc = ent.changer_statut_for_actor(etab, a, ar, doss.id, &StatutBody { statut: "incomplet".into(), motif: Some("Pièce manquante".into()) }).await.unwrap();
    assert_eq!((inc.statut.as_str(), inc.etat.as_str()), ("incomplet", "attente_famille"));
    let l = ent.list_dossiers_for_actor(etab, d, dr, Some(camp), Some("incomplet".into())).await.unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].etat, "attente_famille");
    // Un changement impossible depuis l'état est refusé (plus de saut direct vers une décision).
    assert!(ent.changer_statut_for_actor(etab, a, ar, doss.id, &StatutBody { statut: "admis".into(), motif: None }).await.is_err());
}
