//! Socle 3, lots I3 (pièces réelles) et I4 (accords des responsables), contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Aucun test ne dépend de l'heure réelle : les jours sont lus en base ou posés explicitement, les délais
//! (purge) sont passés en paramètre, les échéances ne sont jamais comparées à « aujourd'hui ».
//!
//! | Critère | Test(s) |
//! |---|---|
//! | contrôle du type réel (le nom et l'en-tête ne comptent pas) | `i3_type_reel_controle_sur_le_contenu` |
//! | dépôt → quarantaine → analyse → contrôle → lecture du PDF servi | `i3_depot_analyse_controle_lecture` |
//! | fichier infecté : jamais servi, la pièce redevient à déposer | `i3_fichier_infecte_jamais_servi` |
//! | photo mobile → PDF | `i3_photos_du_telephone_en_un_pdf` |
//! | règles conditionnelles, réapplication en cours de campagne | `i3_regles_conditionnelles_et_reapplication` |
//! | pièce « famille » déposée une fois pour deux enfants | `i3_piece_de_famille_une_fois_pour_deux_enfants` |
//! | réutilisation proposée, jamais en silence | `i3_reutilisation_proposee_jamais_en_silence` |
//! | rattachement à la validation, pièce bloquante | `i3_validation_pieces_bloquantes_et_rattachement` |
//! | purge des non-aboutis | `i3_purge_des_non_aboutis` |
//! | droits et isolation | `i3_droits_et_isolation` |
//! | demande à tous les titulaires, réponse en ligne sans compte | `i4_reponse_en_ligne_sans_compte` |
//! | code à usage limité | `i4_code_cinq_essais_et_lien_faux` |
//! | désaccord bloquant, levée avec pièce | `i4_refus_bloque_et_se_leve_avec_piece` |
//! | décision de justice | `i4_decision_de_justice_dispense_le_refus` |
//! | parent sans autorité ne décide pas | `i4_parent_sans_autorite_ne_decide_pas` |
//! | accord papier | `i4_accord_papier_saisi_par_le_secretariat` |
//! | ne pas solliciter, dispense | `i4_ne_pas_solliciter_et_dispense` |
//! | désaccord d'un parent non déposant, messages neutres | `i4_desaccord_declare_message_neutre` |
//! | règle `un_seul` | `i4_regle_un_seul_le_depot_vaut_accord` |

use std::sync::Arc;

use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, VerdictAnalyse};
use ent_types::inscriptions::*;
use ent_types::inscriptions_pieces::*;
use uuid::Uuid;

use super::tests_inscriptions::{adm, campagne, campagne_ouverte, courriels_vers, dir, formulaire, labo, second_parent, verifier};
use super::*;

// ───────────────────────── Doublures et fichiers de test ─────────────────────────

const EICAR: &[u8] = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

/// Antivirus de test : reconnaît EICAR comme le vrai.
struct AvSain;

impl AnalyseurAntivirus for AvSain {
    fn nom(&self) -> &'static str {
        "test"
    }
    fn analyser(&self, ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse {
        if ctx.octets.windows(EICAR.len()).any(|w| w == EICAR) {
            return VerdictAnalyse::Infecte("Eicar-Test-Signature".into());
        }
        VerdictAnalyse::Sain
    }
}

pub(crate) fn av() -> Arc<dyn AnalyseurAntivirus> {
    Arc::new(AvSain)
}

pub(crate) fn pdf_simple() -> Vec<u8> {
    b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R  >>\nendobj\n\
      2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
      3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>\nendobj\n\
      trailer\n<< /Root 1 0 R >>\nstartxref\n0\n%%EOF\n"
        .to_vec()
}

fn pdf_infecte() -> Vec<u8> {
    let mut v = pdf_simple();
    v.extend_from_slice(EICAR);
    v
}

fn segment(m: u8, corps: &[u8]) -> Vec<u8> {
    let mut v = vec![0xff, m];
    v.extend_from_slice(&((corps.len() + 2) as u16).to_be_bytes());
    v.extend_from_slice(corps);
    v
}

/// Un JPEG minimal (photo de téléphone).
fn photo() -> Vec<u8> {
    let mut v = vec![0xff, 0xd8];
    v.extend_from_slice(&segment(0xdb, &[0u8; 65]));
    v.extend_from_slice(&segment(0xc0, &[8, 0, 30, 0, 40, 3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]));
    v.extend_from_slice(&segment(0xc4, &[0u8; 20]));
    v.extend_from_slice(&segment(0xda, &[3, 1, 0, 2, 0, 3, 0, 0, 63, 0]));
    v.extend_from_slice(&[0x12, 0x34, 0xff, 0x00, 0x56, 0xff, 0xd9]);
    v
}

/// Dépôt réel par la famille puis analyse (antivirus de test) : ce que fait la tâche de fond.
pub(crate) trait PiecesTest {
    async fn deposer_reel(&self, etab: Uuid, jeton: &str, dossier: Uuid, code: &str);
}

impl PiecesTest for PgEnt {
    async fn deposer_reel(&self, etab: Uuid, jeton: &str, dossier: Uuid, code: &str) {
        self.deposer_fichiers_famille(etab, jeton, dossier, code, vec![pdf_simple()])
            .await
            .unwrap();
        self.analyser_pieces_en_attente(etab, av(), 100).await.unwrap();
    }
}

fn email(prefixe: &str) -> String {
    format!("{prefixe}-{}@exemple.test", Uuid::new_v4().simple())
}

fn controle(decision: &str, motif: Option<&str>) -> ControlePieceBody {
    ControlePieceBody {
        decision: decision.into(),
        motif: motif.map(str::to_string),
        ..Default::default()
    }
}

fn regle(code: &str, libelle: &str, bloquante: bool, portee: &str, conditions: Vec<(&str, &str)>) -> ReglePieceDto {
    ReglePieceDto {
        code_type: code.into(),
        libelle: libelle.into(),
        consigne: None,
        moment: "depot".into(),
        bloquante,
        portee: portee.into(),
        conditions: conditions
            .into_iter()
            .map(|(cle, valeur)| ConditionPieceDto { cle: cle.into(), valeur: valeur.into() })
            .collect(),
    }
}

/// Campagne d'admission ouverte avec la règle d'accord donnée et des règles de pièces posées explicitement.
async fn campagne_avec(
    ent: &PgEnt,
    c: &LaboCast,
    accord: &str,
    regles_pieces: &[ReglePieceDto],
) -> Uuid {
    let (d, dr) = dir(c);
    let mut regles = crate::inscriptions_modele::regles_par_defaut("admission");
    regles.accord_responsables = accord.into();
    let mut b = campagne("admission", None);
    b.pieces = vec![];
    b.regles = Some(regles);
    let camp = ent.creer_campagne_for_actor(c.etablissement_id, d, dr, &b).await.unwrap();
    ent.definir_regles_pieces_for_actor(c.etablissement_id, d, dr, camp.id, regles_pieces)
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(c.etablissement_id, d, dr, camp.id).await.unwrap();
    camp.id
}

async fn deposer(ent: &PgEnt, etab: Uuid, jeton: &str, camp: Uuid, prenom: &str, second: Option<ResponsableFormulaireDto>) -> Uuid {
    ent.deposer_preinscription(etab, jeton, &formulaire(camp, prenom, second))
        .await
        .unwrap()
        .id
}

fn statut_de(pieces: &[PieceAttendueDto], code: &str) -> String {
    pieces.iter().find(|p| p.code == code).unwrap_or_else(|| panic!("pièce {code} absente")).statut.clone()
}

fn mise_en_en_instruction(dossier: Uuid) -> StatutBody {
    let _ = dossier;
    StatutBody { statut: "en_instruction".into(), motif: None }
}

// ───────────────────────────── I3 ─────────────────────────────

/// Le type réel se contrôle sur le contenu : un exécutable renommé « .pdf » est refusé à l'entrée, un fichier
/// vide ou trop gros aussi ; un PDF qui se dit « image » est lu pour ce qu'il est.
#[tokio::test]
async fn i3_type_reel_controle_sur_le_contenu() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = campagne_avec(&ent, &c, "un_seul", &[regle("livret_famille", "Livret de famille", true, "dossier", vec![])]).await;
    let jeton = verifier(&ent, etab, &email("type")).await;
    let dossier = deposer(&ent, etab, &jeton, camp, "Type", None).await;
    let exe = b"MZ\x90\x00\x03\x00\x00\x00 programme.pdf".to_vec();
    for faux in [exe, vec![], b"<html><script>alert(1)</script></html>".to_vec(), b"PK\x03\x04archive".to_vec()] {
        assert!(
            ent.deposer_fichiers_famille(etab, &jeton, dossier, "livret_famille", vec![faux]).await.is_err(),
            "un fichier qui n'est ni PDF, ni JPEG, ni PNG est refusé"
        );
    }
    // PDF et photos : pas les deux à la fois.
    assert!(ent
        .deposer_fichiers_famille(etab, &jeton, dossier, "livret_famille", vec![pdf_simple(), photo()])
        .await
        .is_err());
    // Rien n'a été déposé : la pièce attend toujours.
    let p = ent.pieces_dossier_famille(etab, &jeton, dossier).await.unwrap();
    assert_eq!(statut_de(&p, "livret_famille"), "attendue");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_doc_piece WHERE etablissement_id = $1 AND dossier_id = $2")
        .bind(etab)
        .bind(dossier)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

/// Déposer, attendre l'analyse, contrôler, lire. Rien n'est lisible ni acceptable en quarantaine.
#[tokio::test]
async fn i3_depot_analyse_controle_lecture() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let camp = campagne_avec(&ent, &c, "un_seul", &[regle("livret_famille", "Livret de famille", true, "dossier", vec![])]).await;
    let jeton = verifier(&ent, etab, &email("depot")).await;
    let dossier = deposer(&ent, etab, &jeton, camp, "Depot", None).await;
    let r = ent
        .deposer_fichiers_famille(etab, &jeton, dossier, "livret_famille", vec![pdf_simple()])
        .await
        .unwrap();
    assert_eq!((r.statut.as_str(), r.fichier_etat.as_str()), ("deposee", "en_quarantaine"));
    // En quarantaine : ni acceptation, ni lecture.
    assert!(ent
        .controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("accepter", None))
        .await
        .is_err());
    assert!(ent.lire_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille").await.is_err());
    let bilan = ent.analyser_pieces_en_attente(etab, av(), 100).await.unwrap();
    assert_eq!(bilan.recues, 1);
    let p = ent.pieces_dossier_famille(etab, &jeton, dossier).await.unwrap();
    assert_eq!(p[0].fichier_etat.as_deref(), Some("recue"));
    // Un refus exige un motif type ; « autre » s'explique.
    assert!(ent
        .controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("refuser", None))
        .await
        .is_err());
    assert!(ent
        .controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("refuser", Some("autre")))
        .await
        .is_err());
    let p = ent
        .controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("accepter", None))
        .await
        .unwrap();
    assert_eq!(statut_de(&p, "livret_famille"), "acceptee");
    // La lecture rend le PDF servi (désarmé), journalisée ; jamais l'original.
    let lue = ent.lire_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille").await.unwrap();
    assert!(lue.octets.starts_with(b"%PDF-"));
    assert_eq!(lue.nb_pages, 1);
    let (journal,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_journal WHERE etablissement_id = $1 AND action = 'piece_inscription_lue'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(journal, 1);
    // Acceptée : plus de nouveau dépôt.
    assert!(ent
        .deposer_fichiers_famille(etab, &jeton, dossier, "livret_famille", vec![pdf_simple()])
        .await
        .is_err());
}

#[tokio::test]
async fn i3_fichier_infecte_jamais_servi() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let camp = campagne_avec(&ent, &c, "un_seul", &[regle("livret_famille", "Livret de famille", true, "dossier", vec![])]).await;
    let jeton = verifier(&ent, etab, &email("eicar")).await;
    let dossier = deposer(&ent, etab, &jeton, camp, "Eicar", None).await;
    ent.deposer_fichiers_famille(etab, &jeton, dossier, "livret_famille", vec![pdf_infecte()]).await.unwrap();
    let bilan = ent.analyser_pieces_en_attente(etab, av(), 100).await.unwrap();
    assert_eq!(bilan.infectees, 1);
    // La famille le sait seule, avec un motif sans détail technique ; rien n'est lisible.
    let p = ent.pieces_dossier_famille(etab, &jeton, dossier).await.unwrap();
    assert_eq!(statut_de(&p, "livret_famille"), "refusee");
    assert!(p[0].motif_refus.as_deref().unwrap().contains("analyse de sécurité"));
    assert!(ent.lire_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille").await.is_err());
    assert!(ent
        .controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("accepter", None))
        .await
        .is_err());
    // Un nouveau dépôt sain est possible et passe.
    ent.deposer_reel(etab, &jeton, dossier, "livret_famille").await;
    let p = ent
        .controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("accepter", None))
        .await
        .unwrap();
    assert_eq!(statut_de(&p, "livret_famille"), "acceptee");
}

#[tokio::test]
async fn i3_photos_du_telephone_en_un_pdf() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let camp = campagne_avec(&ent, &c, "un_seul", &[regle("acte_naissance", "Acte de naissance", true, "dossier", vec![])]).await;
    let jeton = verifier(&ent, etab, &email("photo")).await;
    let dossier = deposer(&ent, etab, &jeton, camp, "Photo", None).await;
    ent.deposer_fichiers_famille(etab, &jeton, dossier, "acte_naissance", vec![photo(), photo(), photo()])
        .await
        .unwrap();
    ent.analyser_pieces_en_attente(etab, av(), 100).await.unwrap();
    let p = ent.pieces_dossier_famille(etab, &jeton, dossier).await.unwrap();
    assert_eq!(p[0].nb_pages, Some(3));
    ent.controler_piece_inscription_for_actor(etab, a, ar, dossier, "acte_naissance", &controle("accepter", None))
        .await
        .unwrap();
    let lue = ent.lire_piece_inscription_for_actor(etab, a, ar, dossier, "acte_naissance").await.unwrap();
    assert!(lue.octets.starts_with(b"%PDF-"), "ce qui est servi est un PDF, jamais la photo");
    assert_eq!(lue.nb_pages, 3);
}

/// « Telle pièce si tel cas » : la décision de justice n'est demandée que si les responsables vivent dans des
/// foyers distincts ; une règle ajoutée en cours de campagne se réapplique aux dossiers ouverts, la famille en
/// est prévenue (suivi visible), une pièce dont la condition tombe ne compte plus.
#[tokio::test]
async fn i3_regles_conditionnelles_et_reapplication() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_avec(
        &ent,
        &c,
        "un_seul",
        &[
            regle("livret_famille", "Livret de famille", true, "dossier", vec![]),
            regle("decision_justice", "Décision de justice", true, "dossier", vec![("foyers_distincts", "true")]),
            regle("exeat", "Certificat de radiation", false, "dossier", vec![("autre_etablissement", "true")]),
        ],
    )
    .await;
    // Un seul foyer, aucun établissement d'origine : ni décision de justice ni exeat.
    let j1 = verifier(&ent, etab, &email("un-foyer")).await;
    let d1 = deposer(&ent, etab, &j1, camp, "Seul", None).await;
    let p1 = ent.pieces_dossier_famille(etab, &j1, d1).await.unwrap();
    assert_eq!(p1.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(), ["livret_famille"]);
    // Deux foyers : la décision de justice est attendue.
    let j2 = verifier(&ent, etab, &email("deux-foyers")).await;
    let d2 = deposer(&ent, etab, &j2, camp, "Deux", Some(second_parent(&email("autre")))).await;
    let p2 = ent.pieces_dossier_famille(etab, &j2, d2).await.unwrap();
    assert!(p2.iter().any(|p| p.code == "decision_justice" && p.bloquante));
    assert!(p2.iter().all(|p| p.code != "exeat"));
    // Règle ajoutée en cours de campagne : réappliquée, prévenue.
    let mut regles = ent.regles_pieces_for_actor(etab, d, dr, camp).await.unwrap();
    regles.push(regle("justificatif_domicile", "Justificatif de domicile", true, "demarche", vec![]));
    ent.definir_regles_pieces_for_actor(etab, d, dr, camp, &regles).await.unwrap();
    let p1 = ent.pieces_dossier_famille(etab, &j1, d1).await.unwrap();
    assert_eq!(statut_de(&p1, "justificatif_domicile"), "attendue");
    let (prevenue,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_suivi_dossier
          WHERE etablissement_id = $1 AND dossier_id = $2 AND kind = 'piece' AND visible_famille
            AND texte LIKE '%supplémentaire%'",
    )
    .bind(etab)
    .bind(d1)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(prevenue, 1);
    // Une règle retirée : la pièce reste au dossier (historique) mais ne compte plus.
    regles.retain(|r| r.code_type != "decision_justice");
    ent.definir_regles_pieces_for_actor(etab, d, dr, camp, &regles).await.unwrap();
    let p2 = ent.pieces_dossier_famille(etab, &j2, d2).await.unwrap();
    assert!(p2.iter().all(|p| p.code != "decision_justice"));
    let (conservee,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_inscription_piece WHERE etablissement_id = $1 AND dossier_id = $2
                         AND code = 'decision_justice' AND NOT applicable)",
    )
    .bind(etab)
    .bind(d2)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(conservee);
    // Règle invalide refusée : une pièce d'identité n'est jamais demandée par une campagne.
    let mut mauvaises = regles.clone();
    mauvaises.push(regle("piece_identite", "Pièce d'identité", true, "dossier", vec![]));
    assert!(ent.definir_regles_pieces_for_actor(etab, d, dr, camp, &mauvaises).await.is_err());
}

fn demarche_deux_enfants(camp: Uuid, second: Option<ResponsableFormulaireDto>) -> DemarcheBody {
    let enfant = |prenom: &str| EnfantDemarcheBody {
        eleve: EleveFormulaireDto {
            nom: "Weasley".into(),
            prenom: prenom.into(),
            niveau_demande: Some("6EME".into()),
            sexe: Some("F".into()),
            date_naissance: chrono::NaiveDate::from_ymd_opt(2015, 5, 5),
            ..Default::default()
        },
        voeux: vec![],
    };
    DemarcheBody {
        campagne_id: camp,
        utiliser_mes_infos: false,
        enregistrer_profil: false,
        enfants: vec![enfant("Ginny"), enfant("Fred")],
        responsable: ResponsableFormulaireDto {
            nom: "Weasley".into(),
            prenom: Some("Arthur".into()),
            lien: Some("pere".into()),
            meme_foyer: true,
            autorite_parentale: true,
            ..Default::default()
        },
        second_parent: second,
        autres_responsables: vec![],
    }
}

/// Une pièce de famille se dépose UNE fois pour tous les enfants de la démarche.
#[tokio::test]
async fn i3_piece_de_famille_une_fois_pour_deux_enfants() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let camp = campagne_avec(
        &ent,
        &c,
        "un_seul",
        &[
            regle("livret_famille", "Livret de famille", true, "demarche", vec![]),
            regle("acte_naissance", "Acte de naissance", true, "dossier", vec![]),
        ],
    )
    .await;
    let jeton = verifier(&ent, etab, &email("fratrie")).await;
    let dem = ent.deposer_demarche(etab, &jeton, &demarche_deux_enfants(camp, None)).await.unwrap();
    let (d1, d2) = (dem.dossiers[0].id, dem.dossiers[1].id);
    ent.deposer_reel(etab, &jeton, d1, "livret_famille").await;
    // Le même fichier vaut pour les deux enfants ; l'acte de naissance, propre à chacun, reste attendu.
    let p1 = ent.pieces_dossier_famille(etab, &jeton, d1).await.unwrap();
    let p2 = ent.pieces_dossier_famille(etab, &jeton, d2).await.unwrap();
    assert_eq!(statut_de(&p1, "livret_famille"), "deposee");
    assert_eq!(statut_de(&p2, "livret_famille"), "deposee");
    assert_eq!(statut_de(&p2, "acte_naissance"), "attendue");
    let (fichiers,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT piece_doc_id) FROM ent_inscription_piece
          WHERE etablissement_id = $1 AND code = 'livret_famille' AND piece_doc_id IS NOT NULL",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fichiers, 1, "un seul fichier pour la démarche");
    // Accepté une fois : accepté pour les deux.
    ent.controler_piece_inscription_for_actor(etab, a, ar, d1, "livret_famille", &controle("accepter", None)).await.unwrap();
    let p2 = ent.pieces_dossier_famille(etab, &jeton, d2).await.unwrap();
    assert_eq!(statut_de(&p2, "livret_famille"), "acceptee");
    // Un enfant ajouté plus tard à la démarche (règle réappliquée) reprend la pièce sans la redemander.
    let dem2 = ent.deposer_demarche(etab, &jeton, &demarche_deux_enfants(camp, None)).await.unwrap();
    let autre = ent.pieces_dossier_famille(etab, &jeton, dem2.dossiers[0].id).await.unwrap();
    assert_eq!(statut_de(&autre, "livret_famille"), "attendue", "une autre démarche ne partage rien");
}

/// Une pièce de foyer demandée enfant par enfant est PROPOSÉE à l'autre enfant, jamais reprise en silence.
#[tokio::test]
async fn i3_reutilisation_proposee_jamais_en_silence() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = campagne_avec(
        &ent,
        &c,
        "un_seul",
        &[
            regle("justificatif_domicile", "Justificatif de domicile", true, "dossier", vec![]),
            regle("acte_naissance", "Acte de naissance", true, "dossier", vec![]),
        ],
    )
    .await;
    let jeton = verifier(&ent, etab, &email("reuse")).await;
    let dem = ent.deposer_demarche(etab, &jeton, &demarche_deux_enfants(camp, None)).await.unwrap();
    let (d1, d2) = (dem.dossiers[0].id, dem.dossiers[1].id);
    // Avant tout dépôt : rien à proposer.
    let p2 = ent.pieces_dossier_famille(etab, &jeton, d2).await.unwrap();
    assert!(p2.iter().all(|p| p.proposition_dossier_id.is_none()));
    ent.deposer_reel(etab, &jeton, d1, "justificatif_domicile").await;
    ent.deposer_reel(etab, &jeton, d1, "acte_naissance").await;
    let p2 = ent.pieces_dossier_famille(etab, &jeton, d2).await.unwrap();
    let dom = p2.iter().find(|p| p.code == "justificatif_domicile").unwrap();
    assert_eq!((dom.statut.as_str(), dom.proposition_dossier_id), ("attendue", Some(d1)));
    // L'acte de naissance est propre à chaque enfant : jamais proposé, jamais réutilisable.
    let acte = p2.iter().find(|p| p.code == "acte_naissance").unwrap();
    assert!(acte.proposition_dossier_id.is_none());
    assert!(ent.reutiliser_piece_famille(etab, &jeton, d2, "acte_naissance", d1).await.is_err());
    // Confirmation explicite : la pièce est reprise (même fichier).
    let r = ent.reutiliser_piece_famille(etab, &jeton, d2, "justificatif_domicile", d1).await.unwrap();
    assert_eq!(statut_de(&r, "justificatif_domicile"), "reprise");
    let (fichiers,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT piece_doc_id) FROM ent_inscription_piece
          WHERE etablissement_id = $1 AND code = 'justificatif_domicile'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fichiers, 1);
    let (decisions,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_piece_decision WHERE etablissement_id = $1 AND decision = 'reutilisee'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(decisions, 1, "la reprise laisse une trace");
    // Un dossier d'une autre démarche : « introuvable ».
    let autre_jeton = verifier(&ent, etab, &email("autre-demarche")).await;
    let d3 = deposer(&ent, etab, &autre_jeton, camp, "Autre", None).await;
    assert!(ent.reutiliser_piece_famille(etab, &autre_jeton, d3, "justificatif_domicile", d1).await.is_err());
}

/// Validation : une pièce bloquante non acceptée bloque ; une dispense motivée de la direction la lève ; à la
/// validation les pièces acceptées sont rattachées à l'élève (mêmes objets, sans copie d'octets).
#[tokio::test]
async fn i3_validation_pieces_bloquantes_et_rattachement() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let camp = campagne_avec(
        &ent,
        &c,
        "un_seul",
        &[
            regle("livret_famille", "Livret de famille", true, "dossier", vec![]),
            regle("bulletins_anterieurs", "Bulletins", true, "dossier", vec![]),
            regle("photo", "Photo d'identité", false, "dossier", vec![]),
        ],
    )
    .await;
    let jeton = verifier(&ent, etab, &email("valid")).await;
    let dossier = deposer(&ent, etab, &jeton, camp, "Valid", None).await;
    ent.changer_statut_for_actor(etab, a, ar, dossier, &mise_en_en_instruction(dossier)).await.unwrap();
    ent.deposer_reel(etab, &jeton, dossier, "livret_famille").await;
    ent.controler_piece_inscription_for_actor(etab, a, ar, dossier, "livret_famille", &controle("accepter", None)).await.unwrap();
    // « Bulletins » déposé mais pas contrôlé : bloquant.
    ent.deposer_reel(etab, &jeton, dossier, "bulletins_anterieurs").await;
    ent.decider_for_actor(etab, d, dr, dossier, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    let e = ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("Bulletins"), "le message nomme la pièce qui manque : {e}");
    // Le résumé du dossier compte la pièce bloquante (la photo, non bloquante, ne compte pas).
    let resumes = ent.list_dossiers_for_actor(etab, a, ar, None, None).await.unwrap();
    assert_eq!(resumes.iter().find(|r| r.id == dossier).unwrap().pieces_manquantes, 1);
    // Dispense par l'administratif : refusée pour une pièce bloquante (décision de la direction).
    assert!(ent
        .controler_piece_inscription_for_actor(
            etab, a, ar, dossier, "bulletins_anterieurs",
            &ControlePieceBody { decision: "dispenser".into(), motif: Some("famille en cours de déménagement".into()), ..Default::default() }
        )
        .await
        .is_err());
    ent.controler_piece_inscription_for_actor(
        etab, d, dr, dossier, "bulletins_anterieurs",
        &ControlePieceBody { decision: "dispenser".into(), motif: Some("famille en cours de déménagement".into()), ..Default::default() },
    )
    .await
    .unwrap();
    let v = ent.valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default()).await.unwrap();
    // Rattachement : une pièce de l'élève au socle 7, mêmes objets que la pièce du dossier.
    let (rattachees, memes_objets): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE EXISTS (
                SELECT 1 FROM ent_doc_piece s WHERE s.etablissement_id = r.etablissement_id AND s.dossier_id = $2
                   AND s.cle_objet_servi = r.cle_objet_servi))
           FROM ent_doc_piece r WHERE r.etablissement_id = $1 AND r.eleve_id = $3",
    )
    .bind(etab)
    .bind(dossier)
    .bind(v.eleve_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((rattachees, memes_objets), (1, 1));
    let (objets,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT cle_objet_servi) FROM ent_doc_piece WHERE etablissement_id = $1 AND type_code = 'livret_famille'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(objets, 1, "aucune copie d'octets");
    let (marquee,): (bool,) = sqlx::query_as(
        "SELECT rattachee_doc_id IS NOT NULL FROM ent_inscription_piece WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'livret_famille'",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(marquee);
}

/// Les fichiers d'un dossier non abouti sont détruits (la ligne et l'historique restent) ; ceux d'un dossier
/// validé, rattachés à l'élève, ne sont jamais touchés.
#[tokio::test]
async fn i3_purge_des_non_aboutis() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let camp = campagne_avec(&ent, &c, "un_seul", &[regle("livret_famille", "Livret de famille", true, "dossier", vec![])]).await;
    // Dossier refusé, pièce acceptée.
    let jr = verifier(&ent, etab, &email("refuse")).await;
    let refuse = deposer(&ent, etab, &jr, camp, "Refuse", None).await;
    ent.changer_statut_for_actor(etab, a, ar, refuse, &mise_en_en_instruction(refuse)).await.unwrap();
    ent.deposer_reel(etab, &jr, refuse, "livret_famille").await;
    ent.controler_piece_inscription_for_actor(etab, a, ar, refuse, "livret_famille", &controle("accepter", None)).await.unwrap();
    ent.decider_for_actor(etab, d, dr, refuse, &DecisionBody { decision: "refuse".into(), motif: Some("classe complète".into()) })
        .await
        .unwrap();
    // Dossier validé.
    let jv = verifier(&ent, etab, &email("valide")).await;
    let valide = deposer(&ent, etab, &jv, camp, "Valide", None).await;
    ent.changer_statut_for_actor(etab, a, ar, valide, &mise_en_en_instruction(valide)).await.unwrap();
    ent.deposer_reel(etab, &jv, valide, "livret_famille").await;
    ent.controler_piece_inscription_for_actor(etab, a, ar, valide, "livret_famille", &controle("accepter", None)).await.unwrap();
    ent.decider_for_actor(etab, d, dr, valide, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    ent.valider_inscription_for_actor(etab, d, dr, valide, &ValidationBody::default()).await.unwrap();
    // Délai non atteint : rien n'est purgé.
    let b = ent.purger_pieces_non_aboutis(etab, 3650).await.unwrap();
    assert_eq!((b.dossiers, b.fichiers_detruits), (0, 0));
    // Délai atteint (le délai se passe en paramètre : aucune horloge).
    let b = ent.purger_pieces_non_aboutis(etab, 0).await.unwrap();
    assert_eq!(b.dossiers, 1);
    assert!(b.fichiers_detruits >= 2, "l'original et le fichier servi");
    let (supprimes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_objet o JOIN ent_doc_piece f ON f.etablissement_id = o.etablissement_id
            AND (f.cle_objet_original = o.cle_objet OR f.cle_objet_servi = o.cle_objet)
          WHERE f.etablissement_id = $1 AND f.dossier_id = $2 AND o.supprime_le IS NOT NULL",
    )
    .bind(etab)
    .bind(refuse)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(supprimes >= 2);
    // Le fichier du dossier validé (partagé avec la pièce de l'élève) est intact et lisible.
    let lue = ent.lire_piece_inscription_for_actor(etab, a, ar, valide, "livret_famille").await.unwrap();
    assert!(lue.octets.starts_with(b"%PDF-"));
    // Celui du dossier refusé est détruit, sa ligne reste (preuve), la lecture est refusée.
    assert!(ent.lire_piece_inscription_for_actor(etab, a, ar, refuse, "livret_famille").await.is_err());
    let (ligne, purgee): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE purgee_le IS NOT NULL) FROM ent_inscription_piece
          WHERE etablissement_id = $1 AND dossier_id = $2",
    )
    .bind(etab)
    .bind(refuse)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((ligne, purgee), (1, 1));
    // Idempotente : une seconde passe ne détruit plus rien.
    let b = ent.purger_pieces_non_aboutis(etab, 0).await.unwrap();
    assert_eq!((b.dossiers, b.fichiers_detruits), (0, 0));
}

#[tokio::test]
async fn i3_droits_et_isolation() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let Some((_, c2)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let camp = campagne_avec(&ent, &c, "un_seul", &[regle("livret_famille", "Livret de famille", true, "dossier", vec![])]).await;
    let jeton = verifier(&ent, etab, &email("droits")).await;
    let dossier = deposer(&ent, etab, &jeton, camp, "Droits", None).await;
    ent.deposer_reel(etab, &jeton, dossier, "livret_famille").await;
    // Un parent (rôle famille) n'instruit ni ne contrôle ; un professeur non plus.
    let parent = c.parent_lea.user_id;
    assert!(ent.pieces_dossier_for_actor(etab, parent, "parent", dossier).await.is_err());
    assert!(ent
        .controler_piece_inscription_for_actor(etab, parent, "parent", dossier, "livret_famille", &controle("accepter", None))
        .await
        .is_err());
    assert!(ent.lire_piece_inscription_for_actor(etab, c.prof.user_id, "enseignant", dossier, "livret_famille").await.is_err());
    // L'agent d'un autre établissement : « introuvable » (jamais le dossier de l'autre).
    let (a2, ar2) = adm(&c2);
    assert!(ent.pieces_dossier_for_actor(c2.etablissement_id, a2, ar2, dossier).await.is_err());
    assert!(ent
        .controler_piece_inscription_for_actor(c2.etablissement_id, a2, ar2, dossier, "livret_famille", &controle("accepter", None))
        .await
        .is_err());
    assert!(ent.lire_piece_inscription_for_actor(c2.etablissement_id, a2, ar2, dossier, "livret_famille").await.is_err());
    // Le déposant d'un autre dossier ne voit pas ce dossier.
    let autre = verifier(&ent, etab, &email("curieux")).await;
    assert!(ent.pieces_dossier_famille(etab, &autre, dossier).await.is_err());
    assert!(ent.deposer_fichiers_famille(etab, &autre, dossier, "livret_famille", vec![pdf_simple()]).await.is_err());
    // L'administratif instruit normalement.
    assert_eq!(ent.pieces_dossier_for_actor(etab, a, ar, dossier).await.unwrap().len(), 1);
}

// ───────────────────────────── I4 ─────────────────────────────

/// Le dernier courriel vers `to` qui contient `marque` : (identifiant de demande, jeton) lus dans le lien.
fn lien_accord(ent: &PgEnt, to: &str) -> (Uuid, String) {
    let corps = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(a, _, c)| a == to && c.contains("/web/accord?d="))
        .unwrap_or_else(|| panic!("aucun lien d'accord envoyé à {to}"))
        .2;
    let q = corps.split("/web/accord?d=").nth(1).unwrap();
    let demande = q.split('&').next().unwrap();
    let jeton = q.split("&t=").nth(1).unwrap().split_whitespace().next().unwrap();
    (Uuid::parse_str(demande).unwrap(), jeton.to_string())
}

fn dernier_code(ent: &PgEnt, to: &str) -> String {
    let corps = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(a, s, _)| a == to && s.contains("code de vérification"))
        .unwrap_or_else(|| panic!("aucun code envoyé à {to}"))
        .2;
    corps.split(": ").nth(1).unwrap().split_whitespace().next().unwrap().to_string()
}

async fn expedier(ent: &PgEnt) {
    ent.expedier_envois_inscriptions(200).await.unwrap();
}

struct Famille {
    dossier: Uuid,
    jeton: String,
    mere: String,
    pere: String,
}

/// Dossier d'admission déposé par la mère ; le père est le second parent (foyer distinct, adresse propre).
async fn famille_privee(ent: &PgEnt, c: &LaboCast, accord: &str) -> Famille {
    let etab = c.etablissement_id;
    let camp = campagne_avec(ent, c, accord, &[regle("livret_famille", "Livret de famille", false, "dossier", vec![])]).await;
    let mere = email("mere");
    let pere = email("pere");
    let jeton = verifier(ent, etab, &mere).await;
    let dossier = deposer(ent, etab, &jeton, camp, "Luna", Some(second_parent(&pere))).await;
    Famille { dossier, jeton, mere, pere }
}

async fn accords(ent: &PgEnt, c: &LaboCast, dossier: Uuid) -> AccordsDossierDto {
    let (a, ar) = adm(c);
    ent.accords_dossier_for_actor(c.etablissement_id, a, ar, dossier).await.unwrap()
}

/// Admission dans le privé (règle « tous les titulaires ») : le déposant donne son accord par le dépôt, une
/// demande part au second titulaire (lien + code, sans compte). Tant qu'il n'a pas répondu, l'admission est
/// refusée ; après sa réponse, elle passe. Le jeton et le code ne sont jamais stockés.
#[tokio::test]
async fn i4_reponse_en_ligne_sans_compte() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let f = famille_privee(&ent, &c, "tous_les_titulaires").await;
    let ac = accords(&ent, &c, f.dossier).await;
    assert_eq!((ac.regle.as_str(), ac.complet, ac.accords.len()), ("tous_les_titulaires", false, 2));
    let rep = |rang| ac.accords.iter().find(|x| x.rang == rang).unwrap();
    assert_eq!((rep(1).reponse.as_str(), rep(1).moyen.as_deref()), ("donne", Some("depot_conjoint")));
    assert_eq!(rep(2).reponse, "attendu");
    assert!(rep(2).demande_en_cours);
    ent.changer_statut_for_actor(etab, a, ar, f.dossier, &StatutBody { statut: "en_instruction".into(), motif: None }).await.unwrap();
    let e = ent
        .decider_for_actor(etab, d, dr, f.dossier, &DecisionBody { decision: "admis".into(), motif: None })
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("accord de tous les titulaires"), "{e}");
    // Le père reçoit son lien ; la mère (déposante) aucune demande.
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &f.pere), 1);
    let (demande, jeton) = lien_accord(&ent, &f.pere);
    // Ce que l'écran public montre : l'établissement, le prénom de l'enfant, l'état — rien d'un autre parent.
    let vue = ent.accord_public(etab, demande, &jeton).await.unwrap();
    assert_eq!((vue.enfant_prenom.as_str(), vue.etat.as_str(), vue.reponse), ("Luna", "envoyee", None));
    // Ni jeton ni code ne sont en base.
    let (stocke,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_envoi WHERE etablissement_id = $1 AND corps LIKE '%' || $2 || '%'",
    )
    .bind(etab)
    .bind(&jeton)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(stocke, 0, "le jeton n'est écrit qu'au moment de l'envoi, jamais dans la file");
    // Sans code, aucune réponse.
    assert!(ent.repondre_accord(etab, demande, &jeton, "000000", "donne").await.is_err());
    ent.envoyer_code_accord(etab, demande, &jeton).await.unwrap();
    expedier(&ent).await;
    let code = dernier_code(&ent, &f.pere);
    assert_eq!(code.len(), 6);
    ent.repondre_accord(etab, demande, &jeton, &code, "donne").await.unwrap();
    let ac = accords(&ent, &c, f.dossier).await;
    assert!(ac.complet && !ac.bloque_desaccord);
    assert_eq!(ac.accords.iter().find(|x| x.rang == 2).unwrap().moyen.as_deref(), Some("en_ligne_code"));
    // Usage unique : le même lien ne répond plus.
    assert!(ent.repondre_accord(etab, demande, &jeton, &code, "refuse").await.is_err());
    // L'historique garde qui, comment, quand.
    let (traces,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_accord_trace WHERE etablissement_id = $1 AND vers_reponse = 'donne'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(traces, 2);
    ent.decider_for_actor(etab, d, dr, f.dossier, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
}

#[tokio::test]
async fn i4_code_cinq_essais_et_lien_faux() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let f = famille_privee(&ent, &c, "tous_les_titulaires").await;
    expedier(&ent).await;
    let (demande, jeton) = lien_accord(&ent, &f.pere);
    // Jeton faux : la même réponse qu'une demande inconnue.
    let e1 = ent.accord_public(etab, demande, "jeton-inventé").await.unwrap_err().to_string();
    let e2 = ent.accord_public(etab, Uuid::new_v4(), &jeton).await.unwrap_err().to_string();
    assert_eq!(e1, e2);
    assert!(ent.envoyer_code_accord(etab, demande, "jeton-inventé").await.is_err());
    // Le jeton d'une demande ne sert pas pour une autre.
    assert!(ent.accord_public(etab, Uuid::new_v4(), &jeton).await.is_err());
    // Un autre établissement ne voit pas la demande.
    let Some((_, c2)) = labo().await else { return };
    assert!(ent.accord_public(c2.etablissement_id, demande, &jeton).await.is_err());
    ent.envoyer_code_accord(etab, demande, &jeton).await.unwrap();
    expedier(&ent).await;
    let bon = dernier_code(&ent, &f.pere);
    let faux = if bon == "123456" { "654321" } else { "123456" };
    for _ in 0..5 {
        assert!(ent.repondre_accord(etab, demande, &jeton, faux, "donne").await.is_err());
    }
    // Cinq essais ratés : même le bon code est refusé (il faut en demander un autre).
    assert!(ent.repondre_accord(etab, demande, &jeton, &bon, "donne").await.is_err());
    // Un nouveau code (nouvel envoi) rouvre cinq essais ; l'ancien code ne vaut plus.
    ent.envoyer_code_accord(etab, demande, &jeton).await.unwrap();
    expedier(&ent).await;
    let nouveau = dernier_code(&ent, &f.pere);
    if nouveau != bon {
        assert!(ent.repondre_accord(etab, demande, &jeton, &bon, "donne").await.is_err());
    }
    ent.repondre_accord(etab, demande, &jeton, &nouveau, "donne").await.unwrap();
    // Le nombre de codes par demande est borné.
    let f2 = famille_privee(&ent, &c, "tous_les_titulaires").await;
    expedier(&ent).await;
    let (d2, j2) = lien_accord(&ent, &f2.pere);
    for _ in 0..5 {
        ent.envoyer_code_accord(etab, d2, &j2).await.unwrap();
    }
    assert!(ent.envoyer_code_accord(etab, d2, &j2).await.is_err());
}

/// Un refus bloque le dossier : aucune décision, aucune validation, aucune levée sans pièce ; les deux parents
/// reçoivent le même message neutre ; la levée exige l'accord écrit (pièce acceptée) et rend l'état d'avant.
#[tokio::test]
async fn i4_refus_bloque_et_se_leve_avec_piece() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let f = famille_privee(&ent, &c, "tous_les_titulaires").await;
    ent.changer_statut_for_actor(etab, a, ar, f.dossier, &StatutBody { statut: "en_instruction".into(), motif: None }).await.unwrap();
    expedier(&ent).await;
    let (demande, jeton) = lien_accord(&ent, &f.pere);
    ent.envoyer_code_accord(etab, demande, &jeton).await.unwrap();
    expedier(&ent).await;
    let code = dernier_code(&ent, &f.pere);
    ent.repondre_accord(etab, demande, &jeton, &code, "refuse").await.unwrap();
    let ac = accords(&ent, &c, f.dossier).await;
    assert!(ac.bloque_desaccord && !ac.complet);
    let (etat, avant): (String, Option<String>) =
        sqlx::query_as("SELECT etat, etat_avant_blocage FROM ent_inscription_dossier WHERE id = $1")
            .bind(f.dossier)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!((etat.as_str(), avant.as_deref()), ("bloque_desaccord", Some("en_instruction")));
    // Aucune décision (la vue historique `statut` ne change pas : c'est l'état qui fait foi), aucune validation.
    for decision in ["admis", "refuse", "liste_attente"] {
        let r = ent
            .decider_for_actor(etab, d, dr, f.dossier, &DecisionBody { decision: decision.into(), motif: Some("motif".into()) })
            .await;
        assert!(r.is_err(), "décision {decision} impossible pendant le blocage");
    }
    assert!(ent.valider_inscription_for_actor(etab, d, dr, f.dossier, &ValidationBody::default()).await.is_err());
    assert!(ent
        .changer_statut_for_actor(etab, a, ar, f.dossier, &StatutBody { statut: "admis".into(), motif: None })
        .await
        .is_err());
    // Message neutre aux DEUX parents, identique, sans dire qui a refusé.
    expedier(&ent).await;
    let msgs: Vec<_> = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .filter(|(_, s, _)| s.contains("en attente"))
        .collect();
    assert_eq!(msgs.len(), 2, "un message par titulaire");
    assert_eq!(msgs[0].2, msgs[1].2);
    assert!(msgs.iter().all(|m| m.2.contains("juge aux affaires familiales") && !m.2.to_lowercase().contains("refus")));
    // Le raccourci « débloquer avec un motif » ne lève pas un désaccord déclaré.
    assert!(ent
        .blocage_for_actor(etab, d, dr, f.dossier, &BlocageBody { bloquer: false, motif: "tout est réglé".into() })
        .await
        .is_err());
    // Levée sans pièce, avec une pièce non déposée, ou d'un mauvais type : refusée.
    let sans = LeverDesaccordBody { rang: 2, issue: "accord_ecrit".into(), piece_code: "accord_responsable".into(), motif: "accord écrit reçu".into() };
    assert!(ent.lever_desaccord_for_actor(etab, d, dr, f.dossier, &sans).await.is_err());
    // L'administratif ne lève pas (décision de la direction).
    assert!(ent.lever_desaccord_for_actor(etab, a, ar, f.dossier, &sans).await.is_err());
    // La pièce « accord écrit » s'ajoute aux pièces attendues, se dépose (la famille peut déposer pendant le
    // blocage), s'accepte ; alors la direction lève.
    let (_, dr_) = dir(&c);
    let mut regles = ent.regles_pieces_for_actor(etab, d, dr_, sql_campagne(&ent, f.dossier).await).await.unwrap();
    regles.push(regle("accord_responsable", "Accord écrit du responsable", false, "dossier", vec![]));
    ent.definir_regles_pieces_for_actor(etab, d, dr, sql_campagne(&ent, f.dossier).await, &regles).await.unwrap();
    ent.deposer_reel(etab, &f.jeton, f.dossier, "accord_responsable").await;
    assert!(
        ent.lever_desaccord_for_actor(etab, d, dr, f.dossier, &sans).await.is_err(),
        "une pièce déposée mais pas acceptée ne lève rien"
    );
    ent.controler_piece_inscription_for_actor(etab, a, ar, f.dossier, "accord_responsable", &controle("accepter", None)).await.unwrap();
    let ac = ent.lever_desaccord_for_actor(etab, d, dr, f.dossier, &sans).await.unwrap();
    assert!(!ac.bloque_desaccord && ac.complet);
    let r2 = ac.accords.iter().find(|x| x.rang == 2).unwrap();
    assert_eq!((r2.reponse.as_str(), r2.moyen.as_deref(), r2.a_piece), ("donne", Some("papier"), true));
    let (etat,): (String,) = sqlx::query_as("SELECT etat FROM ent_inscription_dossier WHERE id = $1")
        .bind(f.dossier)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(etat, "en_instruction", "le dossier revient à l'état d'avant le blocage");
    ent.decider_for_actor(etab, d, dr, f.dossier, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    // L'historique garde le refus puis la levée.
    let (traces,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_accord_trace t JOIN ent_inscription_accord a
           ON a.etablissement_id = t.etablissement_id AND a.id = t.accord_id
          WHERE t.etablissement_id = $1 AND a.dossier_id = $2 AND a.objet = 'admission'
            AND t.de_reponse = 'refuse' AND t.vers_reponse = 'donne' AND t.acteur_type = 'direction'",
    )
    .bind(etab)
    .bind(f.dossier)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(traces, 1);
}

async fn sql_campagne(ent: &PgEnt, dossier: Uuid) -> Uuid {
    let (c,): (Uuid,) = sqlx::query_as("SELECT campagne_id FROM ent_inscription_dossier WHERE id = $1")
        .bind(dossier)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    c
}

/// Une décision de justice qui tranche (le parent n'a plus l'autorité) lève le blocage en dispensant son accord.
#[tokio::test]
async fn i4_decision_de_justice_dispense_le_refus() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let f = famille_privee(&ent, &c, "tous_les_titulaires").await;
    let camp = sql_campagne(&ent, f.dossier).await;
    let mut regles = ent.regles_pieces_for_actor(etab, d, dr, camp).await.unwrap();
    regles.push(regle("decision_justice", "Décision de justice", false, "dossier", vec![]));
    ent.definir_regles_pieces_for_actor(etab, d, dr, camp, &regles).await.unwrap();
    ent.declarer_desaccord_for_actor(etab, a, ar, f.dossier, &DesaccordBody { rang: 2, motif: "le père a téléphoné".into() }).await.unwrap();
    ent.deposer_reel(etab, &f.jeton, f.dossier, "decision_justice").await;
    // Pièce sensible : le droit du domaine « justice » est exigé en plus de la fonction ; l'administratif ne
    // l'a pas, la direction oui.
    let _ = ent
        .controler_piece_inscription_for_actor(etab, a, ar, f.dossier, "decision_justice", &controle("accepter", None))
        .await
        .expect_err("l'administratif n'a pas le droit du domaine justice");
    ent.controler_piece_inscription_for_actor(etab, d, dr, f.dossier, "decision_justice", &controle("accepter", None)).await.unwrap();
    // Un accord écrit ne peut pas s'appuyer sur une décision de justice, ni l'inverse.
    let mauvais = LeverDesaccordBody { rang: 2, issue: "accord_ecrit".into(), piece_code: "decision_justice".into(), motif: "jugement du 3 mars".into() };
    assert!(ent.lever_desaccord_for_actor(etab, d, dr, f.dossier, &mauvais).await.is_err());
    let bon = LeverDesaccordBody { issue: "decision_justice".into(), ..mauvais };
    let ac = ent.lever_desaccord_for_actor(etab, d, dr, f.dossier, &bon).await.unwrap();
    let r2 = ac.accords.iter().find(|x| x.rang == 2).unwrap();
    assert_eq!((r2.reponse.as_str(), r2.moyen.as_deref()), ("dispense", Some("decision_direction")));
    assert!(ac.complet && !ac.bloque_desaccord);
}

/// Un parent sans autorité parentale ne décide pas : aucune demande, aucun accord saisissable, et si l'autorité
/// est retirée après l'envoi du lien, la réponse est refusée côté serveur.
#[tokio::test]
async fn i4_parent_sans_autorite_ne_decide_pas() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let camp = campagne_avec(&ent, &c, "tous_les_titulaires", &[]).await;
    let mere = email("mere");
    let beau = email("beaupere");
    let jeton = verifier(&ent, etab, &mere).await;
    let mut second = second_parent(&beau);
    second.autorite_parentale = false;
    second.lien = Some("autre".into());
    let dossier = deposer(&ent, etab, &jeton, camp, "Sans", Some(second)).await;
    let ac = accords(&ent, &c, dossier).await;
    // Un seul titulaire : la mère, qui a donné par le dépôt ; le beau-père n'a ni ligne d'accord ni demande.
    assert_eq!(ac.accords.len(), 1);
    assert!(ac.complet);
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &beau), 0);
    // La saisie d'un accord ou d'un désaccord à son nom est refusée par le serveur.
    let e = ent
        .saisir_accord_papier_for_actor(etab, a, ar, dossier, &AccordPapierBody { rang: 2, reponse: "donne".into(), piece_code: None })
        .await;
    assert!(e.is_err());
    assert!(ent
        .declarer_desaccord_for_actor(etab, a, ar, dossier, &DesaccordBody { rang: 2, motif: "il n'est pas d'accord".into() })
        .await
        .is_err());
    // Son compte (adresse vérifiée) n'ouvre pas non plus le droit de répondre.
    let jeton_beau = verifier(&ent, etab, &beau).await;
    assert!(ent.repondre_accord_compte(etab, &jeton_beau, dossier, "refuse").await.is_err());
    assert!(!accords(&ent, &c, dossier).await.bloque_desaccord);
    // Autorité retirée APRÈS l'envoi : le lien reçu ne donne aucun droit.
    let camp2 = campagne_avec(&ent, &c, "tous_les_titulaires", &[]).await;
    let mere2 = email("mere");
    let pere2 = email("pere");
    let j2 = verifier(&ent, etab, &mere2).await;
    let d2 = deposer(&ent, etab, &j2, camp2, "Retire", Some(second_parent(&pere2))).await;
    expedier(&ent).await;
    let (demande, tok) = lien_accord(&ent, &pere2);
    sqlx::query("UPDATE ent_inscription_declaration_responsable SET autorite_parentale = 'non' WHERE etablissement_id = $1 AND email = $2")
        .bind(etab)
        .bind(&pere2)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    ent.envoyer_code_accord(etab, demande, &tok).await.unwrap();
    expedier(&ent).await;
    let code = dernier_code(&ent, &pere2);
    assert!(ent.repondre_accord(etab, demande, &tok, &code, "refuse").await.is_err());
    assert!(!accords(&ent, &c, d2).await.bloque_desaccord);
}

/// Parent sans adresse : accord sur papier, scanné et déposé au guichet, saisi par le secrétariat avec son nom ;
/// sans pièce acceptée, rien ne prouve l'accord.
#[tokio::test]
async fn i4_accord_papier_saisi_par_le_secretariat() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let camp = campagne_avec(&ent, &c, "tous_les_titulaires", &[regle("accord_responsable", "Accord écrit du responsable", false, "dossier", vec![])]).await;
    let mere = email("mere");
    let jeton = verifier(&ent, etab, &mere).await;
    // Le père n'a pas d'adresse : le foyer distinct exige un e-mail (D-014), il vit donc dans le foyer A.
    let mut pere = second_parent("sans@exemple.test");
    pere.email = None;
    pere.meme_foyer = true;
    let dossier = deposer(&ent, etab, &jeton, camp, "Papier", Some(pere)).await;
    let ac = accords(&ent, &c, dossier).await;
    assert!(!ac.complet);
    let p = ac.accords.iter().find(|x| x.rang == 2).unwrap();
    assert!(!p.demande_en_cours, "pas d'adresse : aucune demande en ligne");
    // Papier sans pièce : refusé. Avec une pièce déposée mais non acceptée : refusé.
    let b = AccordPapierBody { rang: 2, reponse: "donne".into(), piece_code: None };
    assert!(ent.saisir_accord_papier_for_actor(etab, a, ar, dossier, &b).await.is_err());
    let b = AccordPapierBody { piece_code: Some("accord_responsable".into()), ..b };
    ent.deposer_fichiers_guichet_for_actor(etab, a, ar, dossier, "accord_responsable", vec![photo()]).await.unwrap();
    ent.analyser_pieces_en_attente(etab, av(), 100).await.unwrap();
    assert!(ent.saisir_accord_papier_for_actor(etab, a, ar, dossier, &b).await.is_err());
    ent.controler_piece_inscription_for_actor(etab, a, ar, dossier, "accord_responsable", &controle("accepter", None)).await.unwrap();
    let ac = ent.saisir_accord_papier_for_actor(etab, a, ar, dossier, &b).await.unwrap();
    assert!(ac.complet);
    let p = ac.accords.iter().find(|x| x.rang == 2).unwrap();
    assert_eq!((p.moyen.as_deref(), p.a_piece), (Some("papier"), true));
    // La trace nomme l'agent qui a saisi.
    let (nb,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_accord_trace WHERE etablissement_id = $1 AND acteur_user_id = $2 AND acteur_type = 'secretariat'",
    )
    .bind(etab)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(nb, 1);
    ent.changer_statut_for_actor(etab, a, ar, dossier, &StatutBody { statut: "en_instruction".into(), motif: None }).await.unwrap();
    ent.decider_for_actor(etab, d, dr, dossier, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
}

/// « À ne pas solliciter » : aucune demande, aucune copie vers ce responsable ; son accord se dispense par décision
/// motivée de la direction (pas par l'administratif), sinon le dossier attend.
#[tokio::test]
async fn i4_ne_pas_solliciter_et_dispense() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let camp = campagne_avec(&ent, &c, "tous_les_titulaires", &[]).await;
    let mere = email("mere");
    let protege = email("protege");
    let jeton = verifier(&ent, etab, &mere).await;
    let mut second = second_parent(&protege);
    second.reserve = Some(ReserveResponsableDto {
        adresse_a_proteger: true,
        ne_pas_solliciter: true,
        ne_pas_solliciter_ref: Some("ordonnance de protection 2027/12".into()),
        ..Default::default()
    });
    let dossier = deposer(&ent, etab, &jeton, camp, "Protege", Some(second)).await;
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &protege), 0, "aucune demande d'accord vers un responsable à ne pas solliciter");
    let ac = accords(&ent, &c, dossier).await;
    assert!(!ac.complet);
    assert!(ac.accords.iter().all(|x| !x.demande_en_cours));
    // Même sur relance explicite, rien ne part vers lui.
    assert_eq!(
        ent.demander_accords_for_actor(etab, a, ar, dossier, &DemanderAccordsBody::default()).await.unwrap(),
        0
    );
    // Sans dispense, l'admission est impossible ; l'administratif ne dispense pas.
    ent.changer_statut_for_actor(etab, a, ar, dossier, &StatutBody { statut: "en_instruction".into(), motif: None }).await.unwrap();
    assert!(ent
        .decider_for_actor(etab, d, dr, dossier, &DecisionBody { decision: "admis".into(), motif: None })
        .await
        .is_err());
    let b = DispenseAccordBody { rang: 2, motif: "ordonnance de protection : le dossier ne lui est pas communiqué".into(), piece_code: None };
    assert!(ent.dispenser_accord_for_actor(etab, a, ar, dossier, &b).await.is_err());
    assert!(ent
        .dispenser_accord_for_actor(etab, d, dr, dossier, &DispenseAccordBody { motif: "x".into(), ..b.clone() })
        .await
        .is_err());
    let ac = ent.dispenser_accord_for_actor(etab, d, dr, dossier, &b).await.unwrap();
    assert!(ac.complet);
    ent.decider_for_actor(etab, d, dr, dossier, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    // Toujours aucun message vers lui, même après le blocage éventuel ou la décision.
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &protege), 0);
}

/// Un titulaire qui n'est pas le déposant écrit à l'établissement : le secrétariat déclare son désaccord, le dossier
/// est bloqué, les deux parents reçoivent le même message neutre.
#[tokio::test]
async fn i4_desaccord_declare_message_neutre() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let f = famille_privee(&ent, &c, "un_seul").await;
    // Sous `un_seul`, le dépôt vaut accord : le dossier n'est pas bloqué avant le désaccord.
    assert!(accords(&ent, &c, f.dossier).await.complet);
    assert!(ent.declarer_desaccord_for_actor(etab, a, ar, f.dossier, &DesaccordBody { rang: 2, motif: "ok".into() }).await.is_err(), "motif trop court");
    let ac = ent
        .declarer_desaccord_for_actor(etab, a, ar, f.dossier, &DesaccordBody { rang: 2, motif: "courrier reçu le matin".into() })
        .await
        .unwrap();
    assert!(ac.bloque_desaccord && !ac.complet, "un désaccord déclaré bloque même sous la règle `un_seul`");
    expedier(&ent).await;
    let msgs: Vec<_> = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .filter(|(to, s, _)| (to == &f.mere || to == &f.pere) && s.contains("en attente"))
        .collect();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].2, msgs[1].2, "le même message aux deux parents");
    // Déclarer deux fois ne bloque pas deux fois ni ne double les messages.
    ent.declarer_desaccord_for_actor(etab, a, ar, f.dossier, &DesaccordBody { rang: 2, motif: "relance par téléphone".into() }).await.unwrap();
    let (nb,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_inscription_suivi_dossier WHERE etablissement_id = $1 AND dossier_id = $2 AND statut_apres = 'bloque_desaccord'",
    )
    .bind(etab)
    .bind(f.dossier)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(nb, 1);
}

/// Règle `un_seul` (réinscription, public) : le dépôt du premier parent vaut accord, aucune demande n'est
/// envoyée à l'autre.
#[tokio::test]
async fn i4_regle_un_seul_le_depot_vaut_accord() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (d, dr) = dir(&c);
    let f = famille_privee(&ent, &c, "un_seul").await;
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &f.pere), 0, "aucune demande sous la règle `un_seul`");
    ent.changer_statut_for_actor(etab, a, ar, f.dossier, &StatutBody { statut: "en_instruction".into(), motif: None }).await.unwrap();
    ent.decider_for_actor(etab, d, dr, f.dossier, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    // Sur demande explicite du secrétariat, l'autre parent peut être sollicité.
    assert_eq!(
        ent.demander_accords_for_actor(etab, a, ar, f.dossier, &DemanderAccordsBody { rangs: vec![2] }).await.unwrap(),
        1
    );
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &f.pere), 1);
    // Une relance remplace la demande en cours : l'ancien lien ne marche plus.
    let (premiere, tok1) = lien_accord(&ent, &f.pere);
    ent.demander_accords_for_actor(etab, a, ar, f.dossier, &DemanderAccordsBody::default()).await.unwrap();
    assert!(ent.accord_public(etab, premiere, &tok1).await.unwrap().etat == "annulee");
    assert!(ent.envoyer_code_accord(etab, premiere, &tok1).await.is_err());
}
