//! Socle 2, lot S2-7 : une période complète de l'évaluation au bulletin, contre la
//! base locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, scénarios ignorés.
//!
//! Chaque scénario part d'un établissement neuf (`seed_labo_cast`) et y crée sa propre
//! classe, ses matières, ses services, ses élèves et sa période de notes.

use super::*;
use crate::outbox::ConfigDistribution;
use crate::referentiel_repo::CreerService;
use chrono::Duration;
use domain_eleve::{today_utc, CreateEleve, CreerNotePeriode, DomainError};
use ent_types::evaluations::*;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        taille_lot: 200,
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

async fn notifs(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> usize {
    ent.mes_notifications(etab, user, 200)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == kind)
        .count()
}

struct Monde {
    etab: Uuid,
    classe: Uuid,
    maths: Uuid,
    francais: Uuid,
    /// Service de maths du professeur du labo, professeur principal de la classe.
    s_maths: Uuid,
    /// Service de français d'un autre professeur.
    s_fr: Uuid,
    autre_prof: Uuid,
    periode: Uuid,
    eleves: Vec<Uuid>,
}

const ADMIN: &str = "admin";

async fn monde(ent: &PgEnt, cast: &LaboCast) -> Monde {
    let etab = cast.etablissement_id;
    let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    let suffixe = &Uuid::new_v4().simple().to_string()[..6];
    let classe = ent
        .referentiel_creer_classe(ADMIN, etab, Some(annee.id), &format!("S27-{suffixe}"), Some("4e S2-7"), Some("4EME"))
        .await
        .unwrap();
    let matiere = |code: &str, lib: &str| {
        let (code, lib) = (format!("{code}{suffixe}"), lib.to_string());
        async move {
            ent.create_matiere(
                CreateMatiere {
                    etablissement_id: etab,
                    code,
                    libelle: lib,
                    source_nomenclature: None,
                },
                None,
            )
            .await
            .unwrap()
            .id
        }
    };
    let maths = matiere("M", "Mathématiques").await;
    let francais = matiere("F", "Français").await;
    let autre_prof = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|u| *u != cast.prof.user_id)
        .expect("un second professeur dans le labo");
    let s_maths = ent
        .referentiel_creer_service(
            ADMIN,
            etab,
            CreerService {
                prof_user_id: cast.prof.user_id,
                matiere_id: Some(maths),
                classe_id: Some(classe.id),
                groupe_id: None,
                professeur_principal: true,
            },
        )
        .await
        .unwrap()
        .id;
    let s_fr = ent
        .referentiel_creer_service(
            ADMIN,
            etab,
            CreerService {
                prof_user_id: autre_prof,
                matiere_id: Some(francais),
                classe_id: Some(classe.id),
                groupe_id: None,
                professeur_principal: false,
            },
        )
        .await
        .unwrap()
        .id;
    let mut eleves = Vec::new();
    for (nom, prenom) in [("Abbott", "Hannah"), ("Bones", "Susan"), ("Corner", "Michael")] {
        let e = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: nom.into(),
                prenom: prenom.into(),
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
            .unwrap();
        ent.referentiel_affecter_eleve_classe(ADMIN, etab, classe.id, e.id, None)
            .await
            .unwrap();
        eleves.push(e.id);
    }
    // INE renseigné pour deux élèves seulement (anomalie d'export attendue pour le 3ᵉ).
    for (i, ine) in [(0usize, "111111111AA"), (1, "222222222BB")] {
        sqlx::query("UPDATE ent_eleve SET ine = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleves[i])
            .bind(ine)
            .execute(ent.pool())
            .await
            .unwrap();
    }
    // Hannah : parent_lea voit les notes, parent_tom non.
    for (user, voit) in [(cast.parent_lea.user_id, true), (cast.parent_tom.user_id, false)] {
        ent.link_parent(crate::foyer_repo::CreateLien {
            etablissement_id: etab,
            eleve_id: eleves[0],
            user_id: user,
            foyer_id: None,
            lien: "responsable".into(),
            peut_voir_notes: voit,
            peut_voir_vs: true,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
        })
        .await
        .unwrap();
    }
    let t = today_utc();
    let periode = ent
        .creer_note_periode(CreerNotePeriode {
            etablissement_id: etab,
            code: format!("T{suffixe}"),
            libelle: "Trimestre S2-7".into(),
            date_debut: (t - Duration::days(10)).max(annee.date_debut),
            date_fin: (t + Duration::days(30)).min(annee.date_fin),
            actor_user_id: None,
        })
        .await
        .unwrap()
        .id;
    Monde {
        etab,
        classe: classe.id,
        maths,
        francais,
        s_maths,
        s_fr,
        autre_prof,
        periode,
        eleves,
    }
}

fn devoir(service: Uuid, periode: Uuid, titre: &str, coef: f64, sur: f64) -> CreerDevoirGrilleBody {
    CreerDevoirGrilleBody {
        service_id: service,
        periode_id: periode,
        titre: titre.into(),
        date_devoir: today_utc(),
        coef,
        sur,
    }
}

fn r(eleve: Uuid, statut: &str, valeur: Option<f64>) -> SaisieResultatBody {
    SaisieResultatBody {
        eleve_id: eleve,
        statut: statut.into(),
        valeur,
    }
}

async fn statut(ent: &PgEnt, cast: &LaboCast, m: &Monde, s: &str) {
    ent.changer_statut_periode_for_actor(m.etab, cast.direction.user_id, "direction", m.periode, s)
        .await
        .unwrap();
}

// ─── 1. Une période complète ─────────────────────────────────────────────────

#[tokio::test]
async fn periode_complete_de_la_grille_au_bulletin_et_a_l_export() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, [a, b, c]) = (m.etab, cast.prof.user_id, [m.eleves[0], m.eleves[1], m.eleves[2]]);

    // Services évaluables : le professeur voit le sien, pas celui de français.
    let services = ent.services_evaluables_for_actor(etab, prof, "prof").await.unwrap();
    assert!(services.iter().any(|s| s.service_id == m.s_maths));
    assert!(services.iter().all(|s| s.service_id != m.s_fr));

    // Grille : deux devoirs (coef 1 sur 20, coef 2 sur 10), résultats non numériques.
    let d1 = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "Contrôle 1", 1.0, 20.0)).await.unwrap();
    let d2 = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "Devoir maison", 2.0, 10.0)).await.unwrap();
    let rap = ent
        .saisir_grille_for_actor(etab, prof, "prof", d1.id, &SaisieGrilleBody {
            resultats: vec![r(a, "note", Some(15.0)), r(b, "absent", None), r(c, "note", Some(9.5))],
        })
        .await
        .unwrap();
    assert_eq!((rap.enregistres, rap.inchanges, rap.refuses.len()), (3, 0, 0));
    let rap = ent
        .saisir_grille_for_actor(etab, prof, "prof", d2.id, &SaisieGrilleBody {
            resultats: vec![
                r(a, "note", Some(8.0)),
                r(b, "dispense", None),
                r(c, "non_note", None),
                r(Uuid::new_v4(), "note", Some(5.0)), // hors effectif
                r(a, "note", Some(9.0)),               // doublon
            ],
        })
        .await
        .unwrap();
    assert_eq!(rap.enregistres, 3);
    assert_eq!(rap.refuses.len(), 2);
    // Re-saisie identique : inchangée ; valeur hors barème refusée.
    let rap = ent
        .saisir_grille_for_actor(etab, prof, "prof", d2.id, &SaisieGrilleBody {
            resultats: vec![r(a, "note", Some(8.0)), r(c, "note", Some(10.5))],
        })
        .await
        .unwrap();
    assert_eq!((rap.enregistres, rap.inchanges, rap.refuses.len()), (0, 1, 1));

    let g = ent.grille_for_actor(etab, prof, "prof", m.s_maths, m.periode).await.unwrap();
    assert!(g.saisie_ouverte && g.appreciations_ouvertes);
    let moy = |id: Uuid| g.lignes.iter().find(|l| l.eleve_id == id).unwrap().moyenne;
    // Hannah : (15 × 1 + 16 × 2) / 3 = 15,67 ; Susan : absent + dispensé → pas de moyenne ;
    // Michael : 9,5 (le non noté est exclu).
    assert_eq!(moy(a), Some(15.67));
    assert_eq!(moy(b), None);
    assert_eq!(moy(c), Some(9.5));
    assert_eq!(g.stats.effectif, 2);
    assert_eq!((g.stats.min, g.stats.max), (Some(9.5), Some(15.67)));
    assert_eq!(g.stats.moyenne, Some(12.58)); // (15,666… + 9,5) / 2 = 12,5833…
    let st1 = &g.devoirs.iter().find(|d| d.id == d1.id).unwrap().stats;
    assert_eq!((st1.effectif, st1.moyenne), (2, Some(12.25)));

    // Publication : seule Hannah a un parent qui voit les notes → une notification ;
    // avant publication, la famille ne voit pas l'évaluation.
    let lea = cast.parent_lea.user_id;
    let rel = ent.releve_eleve_for_actor(etab, lea, "parent", a, m.periode).await.unwrap();
    assert!(rel.matieres.iter().all(|x| x.evaluations.is_empty() && x.moyenne.is_none()));
    assert!(ent.notes_for_eleve(etab, a).await.unwrap().iter().all(|n| n.devoir_id != Some(d1.id)));
    ent.publier_devoir_for_actor(etab, prof, "prof", d1.id).await.unwrap();
    ent.publier_devoir_for_actor(etab, prof, "prof", d1.id).await.unwrap(); // idempotent
    // Ancien chemin de saisie : refusé sur un devoir de grille (service, effectif, verrou).
    assert!(matches!(
        ent.saisir_note_for_actor("prof", crate::note_repo::SaisirNote {
            etablissement_id: etab, eleve_id: a, matiere_id: m.maths, valeur: 3.0, sur: 20.0,
            date_note: today_utc(), auteur_user_id: prof, libelle: None, devoir_id: Some(d1.id), periode_id: None,
        }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Moyenne historique : le devoir non publié (d2) n'y entre pas.
    let hist = ent.moyennes_eleve_periode(etab, a, m.periode).await.unwrap();
    assert_eq!(hist.iter().find(|x| x.matiere_id == m.maths).map(|x| x.moyenne_sur_20), Some(15.0));
    distribuer(&ent, etab).await;
    assert_eq!(notifs(&ent, etab, lea, "evaluation_publiee").await, 1);
    assert_eq!(notifs(&ent, etab, cast.parent_tom.user_id, "evaluation_publiee").await, 0);
    let rel = ent.releve_eleve_for_actor(etab, lea, "parent", a, m.periode).await.unwrap();
    let rm = rel.matieres.iter().find(|x| x.matiere_id == m.maths).unwrap();
    assert_eq!(rm.evaluations.len(), 1, "seul le devoir publié est visible");
    assert_eq!(rm.moyenne, Some(15.0));
    assert!(matches!(
        ent.releve_eleve_for_actor(etab, cast.parent_tom.user_id, "parent", a, m.periode).await,
        Err(EntDbError::Forbidden)
    ));

    // Verrou partiel : notes fermées, appréciations ouvertes.
    statut(&ent, &cast, &m, "partielle_appreciations").await;
    assert!(matches!(
        ent.saisir_grille_for_actor(etab, prof, "prof", d1.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(1.0))] }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "Tardif", 1.0, 20.0)).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.saisir_appreciation_eva_for_actor(etab, prof, "prof", &AppreciationBody {
        eleve_id: a, periode_id: m.periode, matiere_id: Some(m.maths), texte: "Très bon trimestre.".into(),
    }).await.unwrap();
    // Le professeur de français n'écrit pas l'appréciation de maths.
    assert!(matches!(
        ent.saisir_appreciation_eva_for_actor(etab, m.autre_prof, "prof", &AppreciationBody {
            eleve_id: a, periode_id: m.periode, matiere_id: Some(m.maths), texte: "x".into(),
        }).await,
        Err(EntDbError::Forbidden)
    ));
    // Appréciation générale : le professeur principal ; alerte des matières sans appréciation.
    let gen = ent.saisir_appreciation_eva_for_actor(etab, prof, "prof", &AppreciationBody {
        eleve_id: c, periode_id: m.periode, matiere_id: None, texte: "Des efforts à poursuivre.".into(),
    }).await.unwrap();
    assert_eq!(gen.matieres_sans_appreciation, vec!["Mathématiques".to_string()]);
    assert!(matches!(
        ent.saisir_appreciation_eva_for_actor(etab, m.autre_prof, "prof", &AppreciationBody {
            eleve_id: c, periode_id: m.periode, matiere_id: None, texte: "x".into(),
        }).await,
        Err(EntDbError::Forbidden)
    ));

    // Conseil de classe : préparation, décision, avis sur un vœu.
    let conseil = ent.creer_conseil_eva_for_actor(etab, cast.direction.user_id, "direction", &CreerConseilBody {
        classe_id: m.classe, periode_id: m.periode, jour: today_utc(), heure: chrono::NaiveTime::from_hms_opt(17, 0, 0).unwrap(), lieu: "Salle 12".into(),
    }).await.unwrap();
    let prep = ent.preparation_conseil_for_actor(etab, prof, "prof", conseil.id).await.unwrap();
    assert!(prep.peut_decider, "professeur principal");
    assert_eq!(prep.eleves.len(), 3);
    let pa = prep.eleves.iter().find(|e| e.eleve_id == a).unwrap();
    assert_eq!(pa.moyenne_generale, Some(15.67));
    assert!(pa.appreciations_manquantes.is_empty());
    let pc = prep.eleves.iter().find(|e| e.eleve_id == c).unwrap();
    assert_eq!(pc.appreciations_manquantes, vec!["Mathématiques".to_string()]);
    assert_eq!(prep.appreciations_manquantes, 1); // Susan n'a pas de moyenne
    assert_eq!(prep.appreciations_generales_manquantes, 2);
    // Un professeur de la classe prépare mais ne décide pas.
    let prep2 = ent.preparation_conseil_for_actor(etab, m.autre_prof, "prof", conseil.id).await.unwrap();
    assert!(!prep2.peut_decider);
    let decision = DecisionConseilBody {
        eleve_id: a,
        appreciation: "Félicitations du conseil.".into(),
        mention: Some("felicitations".into()),
        decision_code: Some("passage".into()),
        avis_orientation: Some("Seconde générale".into()),
    };
    assert!(matches!(
        ent.decider_conseil_eva_for_actor(etab, m.autre_prof, "prof", conseil.id, &decision).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.decider_conseil_eva_for_actor(etab, prof, "prof", conseil.id, &DecisionConseilBody { mention: Some("bravo".into()), ..decision.clone() }).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.decider_conseil_eva_for_actor(etab, cast.direction.user_id, "direction", conseil.id, &decision).await.unwrap();
    // Vœu d'orientation (ORI-01 existant) puis avis du conseil (ORI-02).
    let filiere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_filiere (id, etablissement_id, code, libelle, created_by_user_id) VALUES ($1,$2,$3,'Seconde générale',$4)")
        .bind(filiere).bind(etab).bind(format!("2GT{}", &filiere.simple().to_string()[..4])).bind(cast.admin.user_id)
        .execute(ent.pool()).await.unwrap();
    let voeu = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_voeu_inscription (id, etablissement_id, eleve_id, filiere_id, rang, annee_scolaire, created_by_user_id) VALUES ($1,$2,$3,$4,1,'2026-2027',$5)")
        .bind(voeu).bind(etab).bind(a).bind(filiere).bind(lea)
        .execute(ent.pool()).await.unwrap();
    let avis = ent.avis_voeu_for_actor(etab, prof, "prof", conseil.id, &AvisVoeuBody {
        voeu_id: voeu, avis: "favorable".into(), motivation: "Résultats solides".into(),
    }).await.unwrap();
    let avis_id = avis.avis_id.unwrap();
    // Avant publication du bulletin, la famille ne voit pas l'avis et ne peut y répondre.
    let o = ent.orientation_eleve_for_actor(etab, lea, "parent", a).await.unwrap();
    assert!(o[0].avis.is_none());
    assert!(matches!(
        ent.repondre_avis_for_actor(etab, lea, "parent", avis_id, "accord").await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));

    // Bulletin : impossible tant que la période n'est pas fermée.
    let gen_body = GenererBulletinsBody { classe_id: m.classe, periode_id: m.periode, motif: None };
    let dir = cast.direction.user_id;
    assert!(matches!(
        ent.generer_bulletins_for_actor(etab, dir, "direction", &gen_body).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    statut(&ent, &cast, &m, "fermee").await;
    assert!(matches!(
        ent.saisir_appreciation_eva_for_actor(etab, prof, "prof", &AppreciationBody {
            eleve_id: a, periode_id: m.periode, matiere_id: Some(m.maths), texte: "Trop tard".into(),
        }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.generer_bulletins_for_actor(etab, prof, "prof", &gen_body).await,
        Err(EntDbError::Forbidden)
    ));
    let v1 = ent.generer_bulletins_for_actor(etab, dir, "direction", &gen_body).await.unwrap();
    assert_eq!((v1.bulletins.len(), v1.inchanges), (3, 0));
    assert!(v1.bulletins.iter().all(|x| x.version == 1 && x.statut == "genere"));
    // Rien n'a changé : aucune nouvelle version.
    let again = ent.generer_bulletins_for_actor(etab, dir, "direction", &gen_body).await.unwrap();
    assert_eq!((again.bulletins.len(), again.inchanges), (0, 3));
    let bul_a = v1.bulletins.iter().find(|x| x.eleve_id == a).unwrap().id;
    let detail = ent.bulletin_detail_for_actor(etab, dir, "direction", bul_a).await.unwrap();
    assert_eq!(detail.contenu.moyenne_generale, Some(15.67));
    assert_eq!(detail.contenu.conseil.as_ref().unwrap().mention.as_deref(), Some("felicitations"));
    assert_eq!(detail.contenu.lignes.iter().find(|l| l.matiere == "Mathématiques").unwrap().appreciation.as_deref(), Some("Très bon trimestre."));
    // Version non publiée : invisible pour la famille.
    assert!(ent.bulletins_eleve_for_actor(etab, lea, "parent", a).await.unwrap().is_empty());
    assert!(matches!(
        ent.bulletin_pdf_for_actor(etab, lea, "parent", bul_a).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));

    // Publication : archivage au dossier, notification, consultation famille.
    let pub1 = ent.publier_bulletins_for_actor(etab, dir, "direction", &PublierBulletinsBody { classe_id: m.classe, periode_id: m.periode }).await.unwrap();
    assert_eq!(pub1.bulletins.len(), 3);
    distribuer(&ent, etab).await;
    assert_eq!(notifs(&ent, etab, lea, "bulletin_publie").await, 1);
    assert_eq!(notifs(&ent, etab, cast.parent_tom.user_id, "bulletin_publie").await, 0);
    let fam = ent.bulletins_eleve_for_actor(etab, lea, "parent", a).await.unwrap();
    assert_eq!(fam.len(), 1);
    let (nom, pdf) = ent.bulletin_pdf_for_actor(etab, lea, "parent", bul_a).await.unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4") && nom.ends_with("-v1.pdf"));
    let (docs,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_document WHERE etablissement_id = $1 AND eleve_id = $2 AND source = 'bulletin' AND tiroir = 'famille'")
        .bind(etab).bind(a).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(docs, 1, "bulletin archivé au dossier");
    // Avis désormais visible ; la famille répond.
    let o = ent.orientation_eleve_for_actor(etab, lea, "parent", a).await.unwrap();
    assert_eq!(o[0].avis.as_deref(), Some("favorable"));
    ent.repondre_avis_for_actor(etab, lea, "parent", avis_id, "accord").await.unwrap();
    // Une seule réponse ; l'avis communiqué ne se modifie plus.
    assert!(matches!(
        ent.repondre_avis_for_actor(etab, lea, "parent", avis_id, "desaccord").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.avis_voeu_for_actor(etab, prof, "prof", conseil.id, &AvisVoeuBody {
            voeu_id: voeu, avis: "defavorable".into(), motivation: String::new(),
        }).await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Immuabilité : la base refuse de modifier ou supprimer une version.
    assert!(sqlx::query("UPDATE ent_bulletin SET contenu = '{}'::jsonb WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(bul_a).execute(ent.pool()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_bulletin WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(bul_a).execute(ent.pool()).await.is_err());

    // Correction après fermeture (EVA-14) : direction seule, motif obligatoire.
    let corr = CorrigerNoteBody { eleve_id: c, statut: "note".into(), valeur: Some(12.0), motif: "Erreur de report".into() };
    assert!(matches!(
        ent.corriger_note_for_actor(etab, prof, "prof", d1.id, &corr).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.corriger_note_for_actor(etab, dir, "direction", d1.id, &CorrigerNoteBody { motif: " ".into(), ..corr.clone() }).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Un élève hors de l'effectif du devoir est refusé.
    let autre_eleve = cast.eleve_lea_id;
    assert!(matches!(
        ent.corriger_note_for_actor(etab, dir, "direction", d1.id, &CorrigerNoteBody { eleve_id: autre_eleve, ..corr.clone() }).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.corriger_note_for_actor(etab, dir, "direction", d1.id, &corr).await.unwrap();
    // Nouvelle version : motif obligatoire, seule la classe… seul l'élève changé est versionné.
    assert!(matches!(
        ent.generer_bulletins_for_actor(etab, dir, "direction", &gen_body).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v2 = ent.generer_bulletins_for_actor(etab, dir, "direction", &GenererBulletinsBody { motif: Some("Correction d'une note".into()), ..gen_body.clone() }).await.unwrap();
    // Michael change (moyenne), Hannah aussi (moyenne de classe sur sa ligne), Susan aussi.
    assert!(v2.bulletins.iter().all(|x| x.version == 2));
    let v2c = v2.bulletins.iter().find(|x| x.eleve_id == c).unwrap();
    let d2c = ent.bulletin_detail_for_actor(etab, dir, "direction", v2c.id).await.unwrap();
    assert_eq!(d2c.contenu.moyenne_generale, Some(12.0));
    assert_eq!(d2c.contenu.motif.as_deref(), Some("Correction d'une note"));
    ent.publier_bulletins_for_actor(etab, dir, "direction", &PublierBulletinsBody { classe_id: m.classe, periode_id: m.periode }).await.unwrap();
    let versions = ent.bulletins_eleve_for_actor(etab, dir, "direction", c).await.unwrap();
    // L'ancienne version publiée reste archivée mais sort du tiroir famille.
    let (visibles,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_document WHERE etablissement_id = $1 AND eleve_id = $2 AND source = 'bulletin' AND confidentialite = 'standard'")
        .bind(etab).bind(c).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(visibles, 1);
    // Période rouverte : publication refusée tant qu'elle n'est pas refermée.
    statut(&ent, &cast, &m, "partielle_appreciations").await;
    assert!(matches!(
        ent.publier_bulletins_for_actor(etab, dir, "direction", &PublierBulletinsBody { classe_id: m.classe, periode_id: m.periode }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    statut(&ent, &cast, &m, "fermee").await;
    assert_eq!(versions.len(), 2);
    assert!(versions.iter().any(|v| v.version == 1 && v.statut == "remplace"));
    assert!(versions.iter().any(|v| v.version == 2 && v.statut == "publie"));
    let fam_c: Vec<_> = ent.bulletins_eleve_for_actor(etab, lea, "parent", a).await.unwrap();
    assert_eq!(fam_c.len(), 1, "la famille ne voit que la version publiée en cours");
    assert_eq!(fam_c[0].version, 2);

    // Export LSU / LSL : format documenté, anomalies signalées, trace d'export.
    assert!(matches!(
        ent.exporter_livret_for_actor(etab, prof, "prof", "lsu", m.classe, m.periode).await,
        Err(EntDbError::Forbidden)
    ));
    let lsu = ent.exporter_livret_for_actor(etab, cast.administratif.user_id, "administratif", "lsu", m.classe, m.periode).await.unwrap();
    assert_eq!(lsu.nb_eleves, 3);
    assert!(lsu.avertissement.contains("[NV]"));
    assert!(lsu.anomalies.iter().any(|x| x.contains("INE manquant : Michael Corner")));
    assert!(lsu.contenu.contains("id-be=\"111111111AA\""));
    assert!(lsu.contenu.contains("moyenne-eleve=\"15.67/20\""));
    domain_eleve::evaluation::xml_bien_forme(&lsu.contenu).unwrap();
    let lsl = ent.exporter_livret_for_actor(etab, cast.administratif.user_id, "administratif", "lsl", m.classe, m.periode).await.unwrap();
    assert!(lsl.contenu.contains("<scolia-lsl"));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eva_export WHERE etablissement_id = $1")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 2);
    assert!(matches!(
        ent.exporter_livret_for_actor(etab, cast.administratif.user_id, "administratif", "csv", m.classe, m.periode).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let _ = m.francais;
}

// ─── 2. Droits ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn professeur_hors_classe_et_familles_refuses() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof) = (m.etab, cast.prof.user_id);
    let d = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "C1", 1.0, 20.0)).await.unwrap();
    // Le professeur de français (même classe) ne crée pas de devoir de maths, ne saisit pas.
    assert!(matches!(
        ent.creer_devoir_grille_for_actor(etab, m.autre_prof, "prof", &devoir(m.s_maths, m.periode, "X", 1.0, 20.0)).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.saisir_grille_for_actor(etab, m.autre_prof, "prof", d.id, &SaisieGrilleBody { resultats: vec![r(m.eleves[0], "note", Some(1.0))] }).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(ent.publier_devoir_for_actor(etab, m.autre_prof, "prof", d.id).await, Err(EntDbError::Forbidden)));
    // Un professeur sans aucun élève de la classe ne lit pas la grille.
    let hors = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|u| *u != prof && *u != m.autre_prof)
        .expect("un troisième professeur");
    assert!(matches!(ent.grille_for_actor(etab, hors, "prof", m.s_maths, m.periode).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.vue_classe_for_actor(etab, hors, "prof", m.classe, m.periode).await, Err(EntDbError::Forbidden)));
    // Famille et élève : aucune route de saisie ni de vue de classe.
    let lea = cast.parent_lea.user_id;
    assert!(matches!(ent.grille_for_actor(etab, lea, "parent", m.s_maths, m.periode).await, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.saisir_grille_for_actor(etab, lea, "parent", d.id, &SaisieGrilleBody { resultats: vec![] }).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(ent.vue_classe_for_actor(etab, lea, "parent", m.classe, m.periode).await, Err(EntDbError::Forbidden)));
    // Verrou et règles : direction seulement.
    assert!(matches!(
        ent.changer_statut_periode_for_actor(etab, prof, "prof", m.periode, "fermee").await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.definir_regles_calcul_for_actor(etab, cast.cpe.user_id, "cpe", &ReglesCalculDto { arrondi: "dixieme".into(), absent_compte_zero: false }).await,
        Err(EntDbError::Forbidden)
    ));
    // Le CPE lit la vue de classe (notes.lire établissement) mais ne saisit pas.
    assert!(ent.vue_classe_for_actor(etab, cast.cpe.user_id, "cpe", m.classe, m.periode).await.is_ok());
    assert!(matches!(
        ent.saisir_grille_for_actor(etab, cast.cpe.user_id, "cpe", d.id, &SaisieGrilleBody { resultats: vec![r(m.eleves[0], "note", Some(1.0))] }).await,
        Err(EntDbError::Forbidden)
    ));
    // Rôle paramétré : « notes.saisir » sur tout l'établissement ouvre la saisie au CPE.
    let role = ent
        .creer_role_for_actor(etab, cast.admin.user_id, "admin", crate::droits_repo::RoleSaisie {
            nom: format!("Saisie notes {}", &Uuid::new_v4().simple().to_string()[..4]),
            description: None,
            modele: None,
            droits: vec![crate::droits_repo::Droit {
                action: ent_auth::Action::NotesSaisir,
                portee: ent_auth::Portee::Etablissement,
                cibles: vec![],
            }],
        })
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, cast.admin.user_id, "admin", role.id, cast.cpe.user_id, None)
        .await
        .unwrap();
    let rap = ent
        .saisir_grille_for_actor(etab, cast.cpe.user_id, "cpe", d.id, &SaisieGrilleBody { resultats: vec![r(m.eleves[0], "note", Some(11.0))] })
        .await
        .unwrap();
    assert_eq!(rap.enregistres, 1);
    // Bilan de cycle : saisie par le professeur de la classe, validation par le PP
    // seulement quand les 8 composantes sont positionnées.
    let mut comps: Vec<ComposanteNiveauDto> = domain_eleve::evaluation::COMPOSANTES_SOCLE[..7]
        .iter()
        .map(|(c, _)| ComposanteNiveauDto { composante: c.to_string(), niveau: 3 })
        .collect();
    let b = ent.saisir_bilan_cycle_for_actor(etab, m.autre_prof, "prof", &BilanCycleBody {
        eleve_id: m.eleves[0], cycle: 4, composantes: comps.clone(), synthese: "En bonne voie".into(),
    }).await.unwrap();
    assert!(matches!(ent.valider_bilan_cycle_for_actor(etab, m.autre_prof, "prof", b.id).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.valider_bilan_cycle_for_actor(etab, prof, "prof", b.id).await, Err(EntDbError::SaisieInvalide(_))));
    comps.push(ComposanteNiveauDto { composante: "REP_MND".into(), niveau: 4 });
    ent.saisir_bilan_cycle_for_actor(etab, prof, "prof", &BilanCycleBody {
        eleve_id: m.eleves[0], cycle: 4, composantes: comps, synthese: "Socle acquis".into(),
    }).await.unwrap();
    assert!(ent.bilans_cycle_eleve_for_actor(etab, lea, "parent", m.eleves[0]).await.unwrap().is_empty(), "brouillon invisible des familles");
    let v = ent.valider_bilan_cycle_for_actor(etab, prof, "prof", b.id).await.unwrap();
    assert_eq!((v.statut.as_str(), v.composantes.len()), ("valide", 8));
    assert_eq!(ent.bilans_cycle_eleve_for_actor(etab, lea, "parent", m.eleves[0]).await.unwrap().len(), 1);
    assert!(matches!(
        ent.saisir_bilan_cycle_for_actor(etab, prof, "prof", &BilanCycleBody { eleve_id: m.eleves[0], cycle: 4, composantes: vec![], synthese: String::new() }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

// ─── 3. Isolation ────────────────────────────────────────────────────────────

#[tokio::test]
async fn isolation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let d = ent.creer_devoir_grille_for_actor(m.etab, cast.prof.user_id, "prof", &devoir(m.s_maths, m.periode, "C1", 1.0, 20.0)).await.unwrap();
    let autre = ent.seed_labo_cast().await.unwrap();
    let (b, dir_b) = (autre.etablissement_id, autre.direction.user_id);
    let nf = |r: Result<_, EntDbError>| matches!(r, Err(EntDbError::Domain(DomainError::NotFound)));
    assert!(nf(ent.grille_for_actor(b, dir_b, "direction", m.s_maths, m.periode).await.map(|_| ())));
    assert!(nf(ent.saisir_grille_for_actor(b, dir_b, "direction", d.id, &SaisieGrilleBody { resultats: vec![] }).await.map(|_| ())));
    assert!(nf(ent.publier_devoir_for_actor(b, dir_b, "direction", d.id).await.map(|_| ())));
    assert!(nf(ent.changer_statut_periode_for_actor(b, dir_b, "direction", m.periode, "fermee").await.map(|_| ())));
    assert!(nf(ent.vue_classe_for_actor(b, dir_b, "direction", m.classe, m.periode).await.map(|_| ())));
    assert!(nf(ent.generer_bulletins_for_actor(b, dir_b, "direction", &GenererBulletinsBody { classe_id: m.classe, periode_id: m.periode, motif: None }).await.map(|_| ())));
    // Le service de A n'apparaît pas dans B ; B ne crée rien sur un service de A.
    assert!(ent.services_evaluables_for_actor(b, dir_b, "direction").await.unwrap().iter().all(|s| s.service_id != m.s_maths));
    assert!(nf(ent.creer_devoir_grille_for_actor(b, dir_b, "direction", &devoir(m.s_maths, m.periode, "X", 1.0, 20.0)).await.map(|_| ())));
    // Bulletin de A introuvable depuis B (même avec son identifiant).
    statut(&ent, &cast, &m, "fermee").await;
    let v = ent.generer_bulletins_for_actor(m.etab, cast.direction.user_id, "direction", &GenererBulletinsBody { classe_id: m.classe, periode_id: m.periode, motif: None }).await.unwrap();
    let id = v.bulletins[0].id;
    assert!(nf(ent.bulletin_detail_for_actor(b, dir_b, "direction", id).await.map(|_| ())));
    assert!(nf(ent.bulletin_pdf_for_actor(b, dir_b, "direction", id).await.map(|_| ())));
    assert!(ent.bulletins_eleve_for_actor(b, dir_b, "direction", m.eleves[0]).await.map(|v| v.is_empty()).unwrap_or(true));
    // Un événement forgé dans B citant un élève de A ne notifie personne : depuis la clé
    // composée d'E0 (migration 240), la base refuse déjà la publication elle-même.
    let mut tx = ent.pool().begin().await.unwrap();
    use crate::outbox::Publier;
    let forge = tx
        .publier(ent_events::MetierEventDraft {
        etablissement_id: b,
        type_name: ent_events::types::NOTE_BULLETIN_PUBLIE.to_string(),
        aggregate: ent_events::AggregateKind::Eleve,
        aggregate_id: m.eleves[0],
        eleve_id: Some(m.eleves[0]),
        payload: serde_json::json!({ "bulletin_id": id }),
        actor_user_id: None,
    })
    .await;
    assert!(forge.is_err(), "événement inter-établissements accepté");
    drop(tx);
    distribuer(&ent, b).await;
    assert_eq!(notifs(&ent, b, autre.parent_lea.user_id, "bulletin_publie").await, 0);
    assert_eq!(notifs(&ent, m.etab, cast.parent_lea.user_id, "bulletin_publie").await, 0);
}

// ─── 4. Calculs : coefficients de matière, règles de l'établissement ────────

#[tokio::test]
async fn coefficients_de_matiere_et_regles_de_calcul() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, a) = (m.etab, cast.prof.user_id, m.eleves[0]);
    let dm = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "M", 1.0, 20.0)).await.unwrap();
    let df = ent.creer_devoir_grille_for_actor(etab, m.autre_prof, "prof", &devoir(m.s_fr, m.periode, "F", 1.0, 20.0)).await.unwrap();
    let dabs = ent.creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "M2", 1.0, 20.0)).await.unwrap();
    ent.saisir_grille_for_actor(etab, prof, "prof", dm.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(16.0))] }).await.unwrap();
    ent.saisir_grille_for_actor(etab, m.autre_prof, "prof", df.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(10.25))] }).await.unwrap();
    ent.saisir_grille_for_actor(etab, prof, "prof", dabs.id, &SaisieGrilleBody { resultats: vec![r(a, "absent", None)] }).await.unwrap();
    let dir = cast.direction.user_id;
    let vue = |v: VueClasseDto| v.lignes.into_iter().find(|l| l.eleve_id == a).unwrap().moyenne_generale;
    // Coefficient 1 / 1 : (16 + 10,25) / 2 = 13,125 → 13,13 (demi vers le haut).
    assert_eq!(vue(ent.vue_classe_for_actor(etab, dir, "direction", m.classe, m.periode).await.unwrap()), Some(13.13));
    // Maths coefficient 3 : (48 + 10,25) / 4 = 14,5625 → 14,56.
    ent.definir_coef_matiere_for_actor(etab, cast.administratif.user_id, "administratif", &CoefMatiereBody { classe_id: m.classe, matiere_id: m.maths, coef: 3.0 }).await.unwrap();
    assert_eq!(vue(ent.vue_classe_for_actor(etab, dir, "direction", m.classe, m.periode).await.unwrap()), Some(14.56));
    assert!(matches!(
        ent.definir_coef_matiere_for_actor(etab, prof, "prof", &CoefMatiereBody { classe_id: m.classe, matiere_id: m.maths, coef: 2.0 }).await,
        Err(EntDbError::Forbidden)
    ));
    // Règle « absent = 0 » : maths (16 + 0) / 2 = 8 → générale (24 + 10,25) / 4 = 8,5625 ;
    // arrondi au dixième → 8,6.
    ent.definir_regles_calcul_for_actor(etab, dir, "direction", &ReglesCalculDto { arrondi: "dixieme".into(), absent_compte_zero: true }).await.unwrap();
    let v = ent.vue_classe_for_actor(etab, dir, "direction", m.classe, m.periode).await.unwrap();
    assert_eq!(v.arrondi, "dixieme");
    let l = v.lignes.iter().find(|l| l.eleve_id == a).unwrap();
    let idx = v.matieres.iter().position(|x| x.matiere_id == m.maths).unwrap();
    assert_eq!(l.moyennes[idx], Some(8.0));
    assert_eq!(l.moyenne_generale, Some(8.6));
    // Arrondi inconnu refusé.
    assert!(matches!(
        ent.definir_regles_calcul_for_actor(etab, dir, "direction", &ReglesCalculDto { arrondi: "entier".into(), absent_compte_zero: false }).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Effacer une cellule (période ouverte) : statut « vide ».
    let rap = ent.saisir_grille_for_actor(etab, prof, "prof", dabs.id, &SaisieGrilleBody { resultats: vec![r(a, "vide", None)] }).await.unwrap();
    assert_eq!(rap.enregistres, 1);
    let g = ent.grille_for_actor(etab, prof, "prof", m.s_maths, m.periode).await.unwrap();
    assert_eq!(g.lignes.iter().find(|l| l.eleve_id == a).unwrap().resultats.len(), 1);
    assert_eq!(g.arrondi, "dixieme");
}
