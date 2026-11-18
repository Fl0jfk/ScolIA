//! Messagerie, lot M5 (`docs/socles/16_MESSAGERIE.md` §12, §26 ; R-M31 à R-M33 ; CM-57 à CM-60, D16) : pièces jointes,
//! contre la base locale de test (feature `labo`). Même discipline que M0a à M4 : un **témoin** précède chaque attaque
//! (la même action par un acteur légitime réussit), chaque scénario sème son propre établissement.
//!
//! **Fichiers réels** : les PDF, images et documents viennent de `fixtures/messagerie/` (reportlab, pypdf, Pillow,
//! python-docx ; voir le README du dossier) — jamais un faux en-tête tronqué.
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | quarantaine : rien n'est servi avant l'analyse ; PDF désarmé servi | `m5_quarantaine_puis_pdf_desarme_servi` |
//! | CM-57 photo : EXIF et GPS retirés ; PNG sans bloc texte | `m5_photo_sans_gps_png_sans_texte` |
//! | CM-58 pièce infectée : refusée, jamais servie, événement neutre | `m5_cm58_infectee_jamais_servie` |
//! | antivirus indisponible : quarantaine maintenue, la passe s'arrête | `m5_antivirus_indisponible_quarantaine_maintenue` |
//! | CM-59 PDF protégé refusé ; formats dangereux refusés à l'entrée | `m5_cm59_pdf_chiffre_et_formats_dangereux` |
//! | D16 : 20 Mio par pièce, 5 pièces par message | `m5_d16_taille_et_nombre` |
//! | URL pré-signée de 60 s, expirée refusée, journal de chaque téléchargement | `m5_url_60_secondes_expiree_refusee_et_journal` |
//! | participant retiré : plus rien ; tiers : 404 uniforme ; message retiré : plus servie | `m5_participant_retire_et_tiers`, `m5_r_m32_message_retire` |
//! | CM-60 carte document : droits du lecteur relus à la lecture | `m5_cm60_carte_document_droits_relus` |
//! | reprise des `BYTEA` : idempotente, rejouable, avec rapport | `m5_reprise_bytea_rejouable_avec_rapport` |
//! | isolation entre établissements | `m5_isolation_entre_etablissements` |

use super::tests_messagerie_m1::{labo, ouvrir, role};
use super::*;
use crate::msg_m5::{BilanAnalysePieces, IssueAnalyse};
use crate::reprise_bytea::OptionsReprise;
use domain_eleve::{AttacherPiece, DomainError, MsgPieceMeta};
use ent_auth::Role;
use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, VerdictAnalyse};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

// ───────────────────────── Fichiers réels ─────────────────────────

pub(crate) const PDF_REEL: &[u8] = include_bytes!("../fixtures/messagerie/certificat_medical.pdf");
pub(crate) const PDF_CHIFFRE: &[u8] = include_bytes!("../fixtures/messagerie/pdf_chiffre.pdf");
pub(crate) const PDF_JAVASCRIPT: &[u8] = include_bytes!("../fixtures/messagerie/pdf_javascript.pdf");
pub(crate) const PDF_OPEN_ACTION: &[u8] = include_bytes!("../fixtures/messagerie/pdf_open_action.pdf");
pub(crate) const PHOTO_GPS: &[u8] = include_bytes!("../fixtures/messagerie/photo_gps.jpg");
pub(crate) const CAPTURE_PNG: &[u8] = include_bytes!("../fixtures/messagerie/capture.png");
pub(crate) const LETTRE_DOCX: &[u8] = include_bytes!("../fixtures/messagerie/lettre.docx");
pub(crate) const LETTRE_MACRO: &[u8] = include_bytes!("../fixtures/messagerie/lettre_macro.docx");
pub(crate) const NOTE_TXT: &[u8] = include_bytes!("../fixtures/messagerie/note.txt");
pub(crate) const ARCHIVE_ZIP: &[u8] = include_bytes!("../fixtures/messagerie/archive.zip");
/// En-tête ELF construit ici (64 bits, x86-64, exécutable) : quelques octets suffisent, la détection est par signature.
/// Aucun binaire compilé hors du dépôt (invérifiable).
pub(crate) const EXECUTABLE: &[u8] = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x3e\x00\x01\x00\x00\x00\x78\x00\x40\x00\x00\x00\x00\x00\x40\x00\x00\x00\x00\x00\x00\x00";

/// Chaîne de test EICAR (inoffensive, reconnue par tous les antivirus).
pub(crate) const EICAR: &[u8] = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

/// Antivirus de test : reconnaît EICAR comme le vrai, et peut être « arrêté » (démon injoignable).
pub(crate) struct AvTest {
    arrete: AtomicBool,
}

impl AvTest {
    pub(crate) fn sain() -> Arc<AvTest> {
        Arc::new(AvTest { arrete: AtomicBool::new(false) })
    }
    fn arreter(&self, oui: bool) {
        self.arrete.store(oui, Ordering::SeqCst);
    }
}

impl AnalyseurAntivirus for AvTest {
    fn nom(&self) -> &'static str {
        "test"
    }
    fn analyser(&self, ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse {
        if self.arrete.load(Ordering::SeqCst) {
            return VerdictAnalyse::Indisponible;
        }
        if ctx.octets.windows(EICAR.len()).any(|w| w == EICAR) {
            return VerdictAnalyse::Infecte("Eicar-Test-Signature".into());
        }
        VerdictAnalyse::Sain
    }
}

fn contient(haystack: &[u8], aiguille: &[u8]) -> bool {
    haystack.windows(aiguille.len()).any(|w| w == aiguille)
}

// ───────────────────────── Mise en scène ─────────────────────────

pub(crate) async fn deposer(
    ent: &PgEnt,
    etab: Uuid,
    message: Uuid,
    auteur: Uuid,
    r: Role,
    nom: &str,
    mime: &str,
    contenu: &[u8],
) -> Result<MsgPieceMeta, EntDbError> {
    ent.attacher_piece_for_actor(
        role(r),
        AttacherPiece {
            etablissement_id: etab,
            message_id: message,
            nom: nom.into(),
            mime_type: mime.into(),
            contenu: contenu.to_vec(),
            actor_user_id: auteur,
        },
    )
    .await
}

/// Sort de quarantaine tout ce qui peut l'être (antivirus sain) : l'analyse **réelle**, pas un raccourci en base.
pub(crate) async fn lever_quarantaine(ent: &PgEnt, etab: Uuid) -> BilanAnalysePieces {
    ent.analyser_pieces_messagerie_etablissement(etab, AvTest::sain(), 200)
        .await
        .expect("analyse des pièces")
}

pub(crate) async fn etat_piece(ent: &PgEnt, piece: Uuid) -> String {
    sqlx::query_scalar("SELECT etat FROM ent_msg_piece WHERE id = $1")
        .bind(piece)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

pub(crate) async fn nb_acces(ent: &PgEnt, etab: Uuid, piece: Uuid, via: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_msg_piece_acces WHERE etablissement_id = $1 AND piece_id = $2 AND via = $3",
    )
    .bind(etab)
    .bind(piece)
    .bind(via)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

/// Un fil ouvert par la direction vers la famille de Léa ; rend (fil de Lily, message).
pub(crate) async fn fil_de_lily(ent: &PgEnt, cast: &LaboCast, sujet: &str) -> (Uuid, Uuid) {
    let fils = ouvrir(
        ent,
        cast.etablissement_id,
        Role::Direction,
        cast.direction.user_id,
        cast.eleve_lea_id,
        sujet,
    )
    .await;
    let f = fils
        .iter()
        .find(|f| f.fil.id != Uuid::nil())
        .expect("un fil pour le foyer de Léa");
    (f.fil.id, f.message.id)
}

pub(crate) fn est_introuvable(e: &EntDbError) -> bool {
    matches!(
        e,
        EntDbError::Domain(DomainError::MsgPieceIntrouvable | DomainError::NotFound | DomainError::MsgFilIntrouvable)
    )
}

pub(crate) fn est_conflit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(crate::service::RefusMetier::Conflit, _))
}

// ───────────────────────── Quarantaine et normalisation ─────────────────────────

#[tokio::test]
async fn m5_quarantaine_puis_pdf_desarme_servi() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Certificat").await;
    // Un PDF à JavaScript est refusé dès l'entrée (politique de dépôt) ; un PDF à action d'ouverture est accepté puis
    // désarmé par l'analyse.
    let e = deposer(&ent, etab, msg, dir, Role::Direction, "js.pdf", "application/pdf", PDF_JAVASCRIPT)
        .await
        .expect_err("JavaScript refusé à l'entrée");
    assert!(matches!(e, EntDbError::Fichier(_)), "{e:?}");
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_OPEN_ACTION)
        .await
        .expect("dépôt d'un vrai PDF à action d'ouverture");
    assert_eq!(meta.etat, "en_quarantaine", "toute pièce naît en quarantaine");
    // Quarantaine : rien n'est servi, ni à la famille ni à l'auteur, ni par lien ni en direct.
    for (qui, r) in [(lily, Role::Parent), (dir, Role::Direction)] {
        let e = ent
            .telecharger_piece_for_actor(etab, qui, role(r), meta.id)
            .await
            .expect_err("jamais servie en quarantaine");
        assert!(est_conflit(&e), "{e:?}");
    }
    // Rien n'est écrit au journal des téléchargements pour un téléchargement refusé.
    assert_eq!(nb_acces(&ent, etab, meta.id, "telechargement").await, 0);

    let bilan = lever_quarantaine(&ent, etab).await;
    assert_eq!(bilan.saines, 1, "{bilan:?}");
    assert_eq!(etat_piece(&ent, meta.id).await, "saine");

    // Témoin : la famille télécharge, et reçoit le PDF **désarmé** (jamais l'original).
    let servie = ent
        .telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id)
        .await
        .expect("témoin : pièce saine");
    assert!(contient(PDF_OPEN_ACTION, b"/OpenAction") && contient(PDF_OPEN_ACTION, b"/URI"));
    assert!(!contient(&servie.contenu, b"/OpenAction") && !contient(&servie.contenu, b"/URI"), "action neutralisée");
    assert_ne!(servie.contenu, PDF_OPEN_ACTION, "ce n'est pas l'original");
    assert!(servie.contenu.starts_with(b"%PDF-"));
    assert_eq!(servie.mime_type, "application/pdf");
    assert_eq!(servie.sha256.as_deref(), Some(ent_object::sha256_hex(&servie.contenu).as_str()));
    // L'original reste scellé, distinct du fichier servi.
    let (cle_orig, cle_servie): (String, String) =
        sqlx::query_as("SELECT storage_key, cle_objet_servi FROM ent_msg_piece WHERE id = $1")
            .bind(meta.id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_ne!(cle_orig, cle_servie);
    assert!(cle_servie.starts_with(&format!("{etab}/messages-servis/")), "clé opaque : {cle_servie}");
    assert_eq!(ent.lire_objet(etab, &cle_orig).await.unwrap(), PDF_OPEN_ACTION, "l'original est intact");
    // Rejouable : la seconde passe ne retraite rien.
    let encore = lever_quarantaine(&ent, etab).await;
    assert_eq!(encore, BilanAnalysePieces::default());
    assert_eq!(
        ent.analyser_piece_message(etab, meta.id, AvTest::sain()).await.unwrap(),
        IssueAnalyse::DejaTraitee
    );
}

#[tokio::test]
async fn m5_photo_sans_gps_png_sans_texte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Photos").await;
    let photo = deposer(&ent, etab, msg, dir, Role::Direction, "IMG_0001.jpg", "image/jpeg", PHOTO_GPS)
        .await
        .unwrap();
    let capture = deposer(&ent, etab, msg, dir, Role::Direction, "capture.png", "image/png", CAPTURE_PNG)
        .await
        .unwrap();
    // Les fixtures portent bien ce qu'on prétend retirer (sinon le test ne prouverait rien).
    assert!(contient(PHOTO_GPS, b"Exif"), "la photo de départ porte un EXIF");
    assert!(contient(CAPTURE_PNG, b"tEXt"), "le PNG de départ porte un bloc texte");
    assert_eq!(lever_quarantaine(&ent, etab).await.saines, 2);
    let p = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), photo.id).await.unwrap();
    assert!(p.contenu.starts_with(&[0xff, 0xd8]), "toujours un JPEG");
    assert!(!contient(&p.contenu, b"Exif"), "EXIF (dont la position GPS) retiré");
    assert_eq!(p.mime_type, "image/jpeg");
    let c = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), capture.id).await.unwrap();
    assert!(c.contenu.starts_with(&[0x89, b'P', b'N', b'G']), "toujours un PNG");
    assert!(!contient(&c.contenu, b"tEXt") && !contient(&c.contenu, b"localisation-secrete"), "bloc texte retiré");
    assert!(c.contenu.len() < CAPTURE_PNG.len(), "plus petit : blocs annexes jetés");
    // Un document bureautique ouvert et un texte passent tels quels (téléchargement en `attachment`).
    let docx = deposer(&ent, etab, msg, dir, Role::Direction, "lettre.docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document", LETTRE_DOCX).await.unwrap();
    let txt = deposer(&ent, etab, msg, dir, Role::Direction, "note.txt", "text/plain", NOTE_TXT).await.unwrap();
    assert_eq!(lever_quarantaine(&ent, etab).await.saines, 2);
    assert_eq!(
        ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), docx.id).await.unwrap().contenu,
        LETTRE_DOCX
    );
    assert_eq!(
        ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), txt.id).await.unwrap().contenu,
        NOTE_TXT
    );
}

#[tokio::test]
async fn m5_cm58_infectee_jamais_servie() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, msg) = fil_de_lily(&ent, &cast, "Document").await;
    // Un vrai PDF auquel est accolée la chaîne EICAR (signature d'un malware de test).
    let mut infecte = PDF_REEL.to_vec();
    infecte.extend_from_slice(b"\n%");
    infecte.extend_from_slice(EICAR);
    infecte.push(b'\n');
    let sain = deposer(&ent, etab, msg, dir, Role::Direction, "ok.pdf", "application/pdf", PDF_REEL).await.unwrap();
    let mal = deposer(&ent, etab, msg, dir, Role::Direction, "piege.pdf", "application/pdf", &infecte).await.unwrap();
    let b = lever_quarantaine(&ent, etab).await;
    assert_eq!((b.saines, b.infectees), (1, 1), "{b:?}");
    // Témoin : la saine se sert.
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), sain.id).await.is_ok());
    // L'infectée n'est jamais servie : ni en direct, ni par lien, ni à son auteur ; elle est « refusée ».
    for (qui, r) in [(lily, Role::Parent), (dir, Role::Direction)] {
        let e = ent.telecharger_piece_for_actor(etab, qui, role(r), mal.id).await.expect_err("refusée");
        assert!(est_conflit(&e), "{e:?}");
    }
    assert_eq!(etat_piece(&ent, mal.id).await, "infectee");
    let (cle_servi,): (Option<String>,) = sqlx::query_as("SELECT cle_objet_servi FROM ent_msg_piece WHERE id = $1")
        .bind(mal.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(cle_servi.is_none(), "aucun fichier servi, jamais");
    // La base interdit de la « réhabiliter » (état terminal, fichier servi interdit hors saine).
    let r = sqlx::query("UPDATE ent_msg_piece SET etat = 'saine' WHERE id = $1").bind(mal.id).execute(ent.pool()).await;
    assert!(r.is_err(), "une pièce analysée ne change plus d'état");
    // Les deux côtés voient l'état dans le fil (meta) : « refusée », sans motif ni nom de menace.
    let metas = ent.messages.list_pieces_meta_fil(etab, fil).await.unwrap();
    assert!(metas.iter().any(|m| m.id == mal.id && m.etat == "infectee"));
    // L'événement est neutre : identifiants et état, ni nom de fichier ni nom de menace (R-M14).
    let charge: String = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'messagerie.piece_analysee'
           AND payload->>'piece_id' = $2",
    )
    .bind(etab)
    .bind(mal.id.to_string())
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(charge.contains("infectee"));
    for interdit in ["Eicar", "piege", ".pdf", "menace"] {
        assert!(!charge.contains(interdit), "événement non neutre ({interdit}) : {charge}");
    }
}

#[tokio::test]
async fn m5_antivirus_indisponible_quarantaine_maintenue() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Antivirus").await;
    let mut ids = Vec::new();
    for i in 0..4 {
        let m = deposer(&ent, etab, msg, dir, Role::Direction, &format!("p{i}.pdf"), "application/pdf", PDF_REEL)
            .await
            .unwrap();
        ids.push(m.id);
    }
    let av = AvTest::sain();
    av.arreter(true);
    let b = ent.analyser_pieces_messagerie_etablissement(etab, av.clone(), 50).await.unwrap();
    assert!(b.antivirus_indisponible, "la passe s'arrête après trois indisponibilités : {b:?}");
    assert_eq!((b.saines, b.en_quarantaine), (0, 3));
    for id in &ids {
        assert_eq!(etat_piece(&ent, *id).await, "en_quarantaine", "fermé par défaut : rien ne sort");
        assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), *id).await.is_err());
    }
    // Le démon revient : tout sort, dans l'ordre, sans doublon ; les tentatives sont comptées.
    av.arreter(false);
    let b = ent.analyser_pieces_messagerie_etablissement(etab, av, 50).await.unwrap();
    assert_eq!(b.saines, 4, "{b:?}");
    let (tentatives,): (i32,) = sqlx::query_as("SELECT analyse_tentatives FROM ent_msg_piece WHERE id = $1")
        .bind(ids[0])
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(tentatives, 2, "une indisponibilité puis l'analyse réussie");
}

#[tokio::test]
async fn m5_cm59_pdf_chiffre_et_formats_dangereux() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Formats").await;
    // Refusés **à l'entrée** : exécutable réel, archive, DOCX à macro, même déguisés en PDF ou en texte.
    for (nom, mime, octets) in [
        ("installer.pdf", "application/pdf", EXECUTABLE),
        ("photos.zip", "application/zip", ARCHIVE_ZIP),
        ("lettre.pdf", "application/pdf", LETTRE_MACRO),
        ("lettre.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", LETTRE_MACRO),
    ] {
        let e = deposer(&ent, etab, msg, dir, Role::Direction, nom, mime, octets)
            .await
            .expect_err(nom);
        assert!(
            matches!(e, EntDbError::Fichier(_) | EntDbError::Domain(DomainError::MsgPieceInvalide)),
            "{nom} : {e:?}"
        );
    }
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1 AND message_id = $2")
        .bind(etab)
        .bind(msg)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0, "aucune ligne, aucun objet pour un refus à l'entrée");
    // Un PDF protégé par mot de passe passe l'entrée (c'est un PDF) mais est **refusé à l'analyse** (CM-59).
    let chiffre = deposer(&ent, etab, msg, dir, Role::Direction, "protege.pdf", "application/pdf", PDF_CHIFFRE)
        .await
        .expect("accepté à l'entrée");
    let b = lever_quarantaine(&ent, etab).await;
    assert_eq!((b.rejetees, b.saines), (1, 0), "{b:?}");
    assert_eq!(etat_piece(&ent, chiffre.id).await, "rejetee");
    for (qui, r) in [(lily, Role::Parent), (dir, Role::Direction)] {
        let e = ent.telecharger_piece_for_actor(etab, qui, role(r), chiffre.id).await.expect_err("jamais servie");
        assert!(est_conflit(&e), "{e:?}");
    }
    // Témoin : le même fichier sans protection se sert.
    let ok = deposer(&ent, etab, msg, dir, Role::Direction, "ok.pdf", "application/pdf", PDF_REEL).await.unwrap();
    lever_quarantaine(&ent, etab).await;
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), ok.id).await.is_ok());
}

#[tokio::test]
async fn m5_d16_taille_et_nombre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Taille").await;
    // Un PDF réel de plus de 3 Mio (le plafond historique de 256 Kio tenait au stockage en base) : une page
    // supplémentaire par ajout incrémental n'est pas nécessaire, un commentaire de fin de fichier suffit ici car
    // seule la taille est en jeu à l'entrée.
    let mut gros = PDF_REEL.to_vec();
    gros.extend(std::iter::repeat_n(b'%', 3 * 1024 * 1024));
    gros.push(b'\n');
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "scan.pdf", "application/pdf", &gros)
        .await
        .expect("3 Mio acceptés (D16)");
    assert!(meta.taille_octets > 3 * 1024 * 1024);
    // 20 Mio + 1 octet : refus de taille (413), jamais une erreur 500.
    let mut trop = PDF_REEL.to_vec();
    trop.extend(std::iter::repeat_n(b'%', 20 * 1024 * 1024));
    match deposer(&ent, etab, msg, dir, Role::Direction, "enorme.pdf", "application/pdf", &trop).await {
        Err(EntDbError::Fichier(e)) => assert!(e.est_taille(), "{e:?}"),
        autre => panic!("attendu un refus de taille, obtenu {autre:?}"),
    }
    // Au plus 5 pièces par message (S-138).
    for i in 0..4 {
        deposer(&ent, etab, msg, dir, Role::Direction, &format!("p{i}.pdf"), "application/pdf", PDF_REEL)
            .await
            .expect("pièces 2 à 5");
    }
    assert!(matches!(
        deposer(&ent, etab, msg, dir, Role::Direction, "de-trop.pdf", "application/pdf", PDF_REEL).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ───────────────────────── Liens, journal, droits ─────────────────────────

#[tokio::test]
async fn m5_url_60_secondes_expiree_refusee_et_journal() {
    let Some((mut ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let secret = [7u8; 32];
    ent.installer_presigner(ent_object::presign::Presigner::local(&secret, 60).unwrap());
    let verif = ent_object::presign::Presigner::local(&secret, 60).unwrap();
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Lien").await;
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL)
        .await
        .unwrap();
    // En quarantaine : aucun lien n'est émis (et la tentative ne laisse pas de ligne de journal).
    let e = ent.lien_piece_message_for_actor(etab, lily, role(Role::Parent), meta.id).await.expect_err("quarantaine");
    assert!(est_conflit(&e), "{e:?}");
    assert_eq!(nb_acces(&ent, etab, meta.id, "lien").await, 0);
    lever_quarantaine(&ent, etab).await;

    let avant = ent_object::presign::now_unix();
    let lien = ent
        .lien_piece_message_for_actor(etab, lily, role(Role::Parent), meta.id)
        .await
        .expect("témoin : lien après contrôle des droits");
    assert!(lien.expires_at - avant <= 61, "60 s : {} s", lien.expires_at - avant);
    assert!(lien.expires_at > avant);
    let token = lien.url.trim_start_matches("/fichiers/");
    let claims = verif.verify_local(token, lien.expires_at - 1).expect("valide jusqu'à l'expiration");
    assert_eq!(claims.user_id, lily, "le lien est celui du lecteur");
    assert_eq!(claims.etablissement_id, etab);
    assert!(claims.key.contains("/messages-servis/"), "le lien vise le fichier servi, jamais l'original");
    assert!(verif.verify_local(token, lien.expires_at + 1).is_err(), "une URL expirée est refusée");
    // Un jeton falsifié est refusé.
    let mut faux = token.to_string();
    faux.push('x');
    assert!(verif.verify_local(&faux, lien.expires_at - 1).is_err());
    // Journal : un lien, un téléchargement direct = une ligne chacun, ni nom ni clé dans la ligne.
    ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id).await.unwrap();
    assert_eq!(nb_acces(&ent, etab, meta.id, "lien").await, 1);
    assert_eq!(nb_acces(&ent, etab, meta.id, "telechargement").await, 1);
    let colonnes: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'ent_msg_piece_acces'",
    )
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(
        !colonnes.iter().any(|c| c.contains("nom") || c.contains("cle") || c.contains("storage")),
        "le journal ne porte ni nom ni clé : {colonnes:?}"
    );
    // Ajout seul : la ligne ne se modifie pas.
    assert!(sqlx::query("UPDATE ent_msg_piece_acces SET via = 'lien' WHERE piece_id = $1")
        .bind(meta.id)
        .execute(ent.pool())
        .await
        .is_err());
}

#[tokio::test]
async fn m5_participant_retire_et_tiers() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let tom = cast.parent_tom.user_id;
    let (fil, msg) = fil_de_lily(&ent, &cast, "Retrait").await;
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL)
        .await
        .unwrap();
    lever_quarantaine(&ent, etab).await;
    // Témoin : Lily, participante, télécharge.
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id).await.is_ok());
    // Un tiers (parent d'un autre élève) : 404 **uniforme** — la même réponse qu'une pièce qui n'existe pas.
    let inexistante = Uuid::new_v4();
    let e_tiers = ent
        .telecharger_piece_for_actor(etab, tom, role(Role::Parent), meta.id)
        .await
        .expect_err("tiers");
    let e_vide = ent
        .telecharger_piece_for_actor(etab, tom, role(Role::Parent), inexistante)
        .await
        .expect_err("inexistante");
    assert!(est_introuvable(&e_tiers), "{e_tiers:?}");
    assert_eq!(format!("{e_tiers:?}"), format!("{e_vide:?}"), "aucun indice sur l'existence de la pièce");
    let l_tiers = ent.lien_piece_message_for_actor(etab, tom, role(Role::Parent), meta.id).await;
    assert!(
        matches!(&l_tiers, Err(e) if est_introuvable(e) || matches!(e, EntDbError::LiensIndisponibles)),
        "{l_tiers:?}"
    );
    // Il ne dépose pas non plus dans ce fil (même 404 qu'un message inexistant).
    let e = deposer(&ent, etab, msg, tom, Role::Parent, "x.pdf", "application/pdf", PDF_REEL).await.expect_err("tiers");
    assert!(est_introuvable(&e), "{e:?}");

    // Lily est **retirée** du fil (sortie volontaire) : plus rien, ni en direct ni par lien, 404 uniforme.
    let r = sqlx::query(
        "UPDATE ent_msg_participant SET au = now(), motif_sortie = 'sortie_volontaire'
          WHERE etablissement_id = $1 AND fil_id = $2 AND cote = 'famille' AND au IS NULL",
    )
    .bind(etab)
    .bind(fil)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(r.rows_affected() >= 1, "Lily était bien participante");
    let e = ent
        .telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id)
        .await
        .expect_err("participant retiré");
    assert!(est_introuvable(&e), "{e:?}");
    assert_eq!(format!("{e:?}"), format!("{e_vide:?}"));
    assert!(ent.lien_piece_message_for_actor(etab, lily, role(Role::Parent), meta.id).await.is_err());
    // L'auteur, lui, garde l'accès.
    assert!(ent.telecharger_piece_for_actor(etab, dir, role(Role::Direction), meta.id).await.is_ok());
}

#[tokio::test]
async fn m5_r_m32_message_retire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Message retiré").await;
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL)
        .await
        .unwrap();
    lever_quarantaine(&ent, etab).await;
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id).await.is_ok(), "témoin");
    // Le message est retiré (R-M28 : masquage tracé, aucune suppression) : la pièce est conservée, plus servie.
    sqlx::query(
        "UPDATE ent_msg SET retire_le = now(), retire_par = $2, retrait_par = 'auteur' WHERE etablissement_id = $1 AND id = $3",
    )
    .bind(etab)
    .bind(dir)
    .bind(msg)
    .execute(ent.pool())
    .await
    .unwrap();
    for (qui, r) in [(lily, Role::Parent), (dir, Role::Direction)] {
        let e = ent.telecharger_piece_for_actor(etab, qui, role(r), meta.id).await.expect_err("retirée");
        assert!(est_introuvable(&e), "{e:?}");
    }
    let (n, etat): (i64, String) = sqlx::query_as(
        "SELECT count(*), max(etat) FROM ent_msg_piece WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(meta.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((n, etat.as_str()), (1, "saine"), "conservée (R-M32)");
}

#[tokio::test]
async fn m5_isolation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Isolation").await;
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL)
        .await
        .unwrap();
    lever_quarantaine(&ent, etab).await;
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id).await.is_ok(), "témoin");
    // Un autre établissement ne voit ni la pièce, ni ses lignes de journal, ne l'analyse pas, ne la reprend pas.
    let (autre, _) = labo().await.expect("second établissement");
    let autre_etab = autre.seed_labo_cast().await.unwrap().etablissement_id;
    assert_ne!(autre_etab, etab);
    let e = ent
        .telecharger_piece_for_actor(autre_etab, lily, role(Role::Parent), meta.id)
        .await
        .expect_err("autre établissement");
    assert!(est_introuvable(&e), "{e:?}");
    assert_eq!(
        ent.analyser_piece_message(autre_etab, meta.id, AvTest::sain()).await.unwrap(),
        IssueAnalyse::DejaTraitee,
        "l'analyse d'un autre établissement ne touche rien"
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_msg_piece_acces WHERE etablissement_id = $1")
        .bind(autre_etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

// ───────────────────────── Reprise des BYTEA ─────────────────────────

/// Pièce de messagerie HISTORIQUE : octets en base, aucune référence d'objet (état par défaut : quarantaine).
async fn historique(ent: &PgEnt, etab: Uuid, fil: Uuid, message: Uuid, auteur: Uuid, nom: &str, mime: &str, octets: &[u8]) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_msg_piece (id, etablissement_id, fil_id, message_id, nom, mime_type, taille_octets, contenu,
                                    created_by_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(id)
    .bind(etab)
    .bind(fil)
    .bind(message)
    .bind(nom)
    .bind(mime)
    .bind(octets.len() as i32)
    .bind(octets)
    .bind(auteur)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn m5_reprise_bytea_rejouable_avec_rapport() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, msg) = fil_de_lily(&ent, &cast, "Historique").await;
    let pdf = historique(&ent, etab, fil, msg, dir, "ancien.pdf", "application/pdf", PDF_REEL).await;
    let jpg = historique(&ent, etab, fil, msg, dir, "ancienne.jpg", "image/jpeg", PHOTO_GPS).await;
    let mal = historique(&ent, etab, fil, msg, dir, "vieux.pdf", "application/pdf", PDF_CHIFFRE).await;
    let opts = OptionsReprise { vider: true, taille_lot: 2, ..Default::default() };
    let av = AvTest::sain();

    // Avant : en quarantaine, donc jamais servies (le BYTEA n'est plus un chemin de lecture).
    for id in [pdf, jpg, mal] {
        assert_eq!(etat_piece(&ent, id).await, "en_quarantaine");
        assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), id).await.is_err());
    }
    // Simulation : ne change rien.
    let simul = OptionsReprise { simulation: true, ..opts.clone() };
    let r = ent.reprendre_et_analyser_pieces_messagerie(etab, &simul, Some(av.clone()), 50).await.unwrap();
    assert_eq!(r.restent_en_quarantaine, 3, "la simulation n'écrit rien");
    assert_eq!(r.analyse, BilanAnalysePieces::default());
    let (en_base,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1 AND octet_length(contenu) > 0",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(en_base, 3);

    // Reprise réelle : tout sort du BYTEA, puis l'analyse a posteriori tranche (saine, saine, rejetée).
    let r = ent.reprendre_et_analyser_pieces_messagerie(etab, &opts, Some(av.clone()), 50).await.unwrap();
    assert_eq!(r.reprise.copiees, 3, "{:?}", r.reprise.resume());
    assert!(r.reprise.en_echec.is_empty());
    assert_eq!(r.reprise.restantes, 0, "plus aucun octet en base");
    assert_eq!((r.analyse.saines, r.analyse.rejetees), (2, 1), "{:?}", r.analyse);
    assert_eq!(r.restent_en_quarantaine, 0);
    assert_eq!(etat_piece(&ent, pdf).await, "saine");
    assert_eq!(etat_piece(&ent, jpg).await, "saine");
    assert_eq!(etat_piece(&ent, mal).await, "rejetee");
    // Servies depuis le stockage objet, normalisées.
    let p = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), jpg).await.unwrap();
    assert!(!contient(&p.contenu, b"Exif"), "l'ancienne photo perd son EXIF à la reprise");
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), pdf).await.is_ok());
    assert!(ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), mal).await.is_err());

    // Rejouable : une seconde exécution ne fait rien (ni copie, ni analyse, ni doublon d'objet).
    let (objets,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_objet WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let r2 = ent.reprendre_et_analyser_pieces_messagerie(etab, &opts, Some(av), 50).await.unwrap();
    assert_eq!(r2.reprise.copiees, 0);
    assert_eq!(r2.analyse, BilanAnalysePieces::default());
    let (objets2,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_objet WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(objets, objets2, "idempotente : aucun objet de plus");
}

// ───────────────────────── Carte « document » (R-M33, CM-60) ─────────────────────────

#[tokio::test]
async fn m5_cm60_carte_document_droits_relus() {
    use crate::tests_documents_d4::W;
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = w.m.etab;
    let (dir, lily, tom) = (w.cast.direction.user_id, w.cast.parent_lea.user_id, w.cast.parent_tom.user_id);
    // Un élève, sa famille (Lily) et un certificat de scolarité émis dans son coffre.
    let eleve = w.eleve("Henry", "Maël").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.certificat(eleve).await;
    let fils = ouvrir(&w.ent, etab, Role::Direction, dir, eleve, "Certificat").await;
    let (fil, msg) = (fils[0].fil.id, fils[0].message.id);

    // CM-60 : un personnel ne joint pas un document du coffre en fichier ; il joint une carte.
    let e = w
        .ent
        .attacher_piece_nature_for_actor(
            role(Role::Direction),
            AttacherPiece {
                etablissement_id: etab,
                message_id: msg,
                nom: "certificat.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: PDF_REEL.to_vec(),
                actor_user_id: dir,
            },
            Some("eleve.certificat_scolarite"),
        )
        .await
        .expect_err("nature du coffre refusée en fichier");
    assert!(matches!(e, EntDbError::Refus(crate::service::RefusMetier::Interdit, _)), "{e:?}");
    // Un tiers ne joint rien (404) ; un participant qui n'est pas l'auteur du message non plus (403).
    let e = w.ent.joindre_carte_document_for_actor(etab, tom, role(Role::Parent), msg, doc.document_id).await.expect_err("tiers");
    assert!(est_introuvable(&e), "{e:?}");
    let e = w.ent.joindre_carte_document_for_actor(etab, lily, role(Role::Parent), msg, doc.document_id).await.expect_err("pas l'auteur");
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");

    // Témoin : l'auteur (qui peut lire le document) joint la carte ; un document inexistant : même 404.
    let carte = w
        .ent
        .joindre_carte_document_for_actor(etab, dir, role(Role::Direction), msg, doc.document_id)
        .await
        .expect("carte jointe");
    assert!(carte.lisible);
    let e = w.ent.joindre_carte_document_for_actor(etab, dir, role(Role::Direction), msg, Uuid::new_v4()).await.expect_err("inconnu");
    assert!(est_introuvable(&e), "{e:?}");

    // Lily lit la carte avec **ses** droits (le document est dans son coffre) ; le titre n'est donné qu'à qui peut lire.
    let vue = w.ent.cartes_du_fil_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    assert_eq!(vue.len(), 1);
    assert!(vue[0].lisible && vue[0].titre.is_some());
    assert!(w.ent.ouvrir_carte_document_for_actor(etab, lily, role(Role::Parent), carte.id).await.is_ok(), "ouverture : témoin");
    // Un tiers n'a pas accès au fil (404) ni à la carte.
    assert!(est_introuvable(&w.ent.cartes_du_fil_for_actor(etab, tom, role(Role::Parent), fil).await.unwrap_err()));
    assert!(est_introuvable(&w.ent.ouvrir_carte_document_for_actor(etab, tom, role(Role::Parent), carte.id).await.unwrap_err()));

    // Le droit de Lily est relu **à la lecture** : le document quitte son coffre (retrait de la mise à disposition),
    // la carte devient « document non disponible » sans titre, et ne s'ouvre plus.
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = false WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(eleve)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    let vue = w.ent.cartes_du_fil_for_actor(etab, lily, role(Role::Parent), fil).await;
    match vue {
        Ok(v) => {
            assert!(!v[0].lisible && v[0].titre.is_none(), "droit retiré : plus de titre ni d'accès");
            assert!(w.ent.ouvrir_carte_document_for_actor(etab, lily, role(Role::Parent), carte.id).await.is_err());
        }
        // Le fil lui-même peut se fermer avec le droit : 404 uniforme, tout aussi correct.
        Err(e) => assert!(est_introuvable(&e), "{e:?}"),
    }
}
