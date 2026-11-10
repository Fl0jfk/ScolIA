//! Reprise des pièces `BYTEA` (messagerie, PAI) vers le magasin d'objets
//! (PRA_STOCKAGE_OBJETS §5, étapes 1 et 2). Données de labo uniquement ; sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés (échec en CI).
//!
//! Les pièces « historiques » sont insérées en SQL (seul moyen d'en produire : plus aucun chemin
//! applicatif n'écrit `contenu`). Les pièces nouvelles passent par les services.

use super::*;
use crate::reprise_bytea::{CauseEchec, OptionsReprise, TablePiece};
use chrono::Utc;
use domain_eleve::{AttacherOrdonnancePai, AttacherPiece, OuvrirFil, UpsertPai};
use ent_auth::Role;
use ent_object::provider::{FaultyProvider, MemoryProvider, Pannes};
use ent_object::{sha256_hex, ObjectStore};
use sqlx::Row;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use uuid::Uuid;

async fn ent_avec_pannes() -> Option<(PgEnt, Arc<Pannes>)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let (f, pannes) = FaultyProvider::new(Arc::new(MemoryProvider::new()));
    ent.installer_objets(ObjectStore::with_provider(Arc::new(f)));
    Some((ent, pannes))
}

/// Un établissement de labo prêt : un fil + message (direction), un PAI (infirmerie).
struct Monde {
    etab: Uuid,
    direction: Uuid,
    infirmiere: Uuid,
    eleve: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    pai_id: Uuid,
}

async fn monde(ent: &PgEnt) -> Monde {
    // Les graines de labo peuvent se heurter (`Reessayer`) quand les tests tournent en parallèle.
    let mut essai = 0;
    let cast = loop {
        match ent.seed_labo_cast().await {
            Ok(c) => break c,
            Err(EntDbError::Domain(
                domain_eleve::DomainError::Reessayer | domain_eleve::DomainError::Infrastructure,
            )) if essai < 8 => {
                essai += 1;
                tokio::time::sleep(std::time::Duration::from_millis(150 * essai)).await;
            }
            Err(e) => panic!("seed: {e:?}"),
        }
    };
    let etab = cast.etablissement_id;
    let direction = cast.direction.user_id;
    let infirmiere = cast.infirmieres[0].user_id;
    let eleve = cast.eleve_lea_id;
    let (fil, msg) = ent
        .ouvrir_fil_for_actor(
            Role::Direction.as_str(),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: eleve,
                sujet: "Fil de reprise".into(),
                premier_message: "Bonjour".into(),
                actor_user_id: direction,
            },
        )
        .await
        .expect("fil");
    let pai = ent
        .upsert_pai_for_actor(
            Role::Infirmerie.as_str(),
            UpsertPai {
                etablissement_id: etab,
                eleve_id: eleve,
                motif: "Asthme".into(),
                mesures_accueil: "Ventoline".into(),
                protocole_urgence: String::new(),
                note_interne: String::new(),
                date_debut: Utc::now().date_naive(),
                date_fin: None,
                actif: true,
                actor_user_id: infirmiere,
            },
        )
        .await
        .expect("pai");
    Monde {
        etab,
        direction,
        infirmiere,
        eleve,
        fil_id: fil.id,
        message_id: msg.id,
        pai_id: pai.id,
    }
}

fn octets(graine: u8, n: usize) -> Vec<u8> {
    let mut v = b"%PDF-1.4\n".to_vec();
    v.extend((0..n).map(|i| graine.wrapping_add((i * 7) as u8)));
    v.extend_from_slice(b"\n%%EOF\n");
    v
}

/// Pièce de messagerie HISTORIQUE : octets en base, aucune référence d'objet.
async fn hist_msg(ent: &PgEnt, m: &Monde, nom: &str, bytes: &[u8]) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_msg_piece (id, etablissement_id, fil_id, message_id, nom, mime_type,
                                    taille_octets, contenu, created_by_user_id)
         VALUES ($1,$2,$3,$4,$5,'application/pdf',$6,$7,$8)",
    )
    .bind(id)
    .bind(m.etab)
    .bind(m.fil_id)
    .bind(m.message_id)
    .bind(nom)
    .bind(bytes.len() as i32)
    .bind(bytes)
    .bind(m.direction)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

/// Pièce de PAI HISTORIQUE (santé).
async fn hist_pai(ent: &PgEnt, m: &Monde, nom: &str, bytes: &[u8]) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_pai_piece (id, etablissement_id, eleve_id, pai_id, nom, mime_type,
                                    taille_octets, contenu, created_by_user_id)
         VALUES ($1,$2,$3,$4,$5,'application/pdf',$6,$7,$8)",
    )
    .bind(id)
    .bind(m.etab)
    .bind(m.eleve)
    .bind(m.pai_id)
    .bind(nom)
    .bind(bytes.len() as i32)
    .bind(bytes)
    .bind(m.infirmiere)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

struct Ligne {
    octets_en_base: i64,
    storage_key: Option<String>,
    sha256: Option<String>,
    repris: bool,
}

async fn ligne(ent: &PgEnt, t: TablePiece, id: Uuid) -> Ligne {
    let r = sqlx::query(&format!(
        "SELECT COALESCE(octet_length(contenu), 0)::bigint AS n, storage_key, sha256,
                repris_le IS NOT NULL AS repris FROM {} WHERE id = $1",
        t.table()
    ))
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    Ligne {
        octets_en_base: r.get("n"),
        storage_key: r.get("storage_key"),
        sha256: r.get("sha256"),
        repris: r.get("repris"),
    }
}

async fn nb_objets(ent: &PgEnt, etab: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_objet WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

async fn lire_msg(ent: &PgEnt, m: &Monde, id: Uuid) -> Vec<u8> {
    ent.telecharger_piece_for_actor(m.etab, m.direction, Role::Direction.as_str(), id)
        .await
        .expect("lecture pièce")
        .contenu
}

async fn lire_pai(ent: &PgEnt, m: &Monde, id: Uuid) -> Vec<u8> {
    ent.telecharger_pai_piece_for_actor(m.etab, m.infirmiere, Role::Infirmerie.as_str(), id)
        .await
        .expect("lecture pièce PAI")
        .contenu
}

fn reel(vider: bool) -> OptionsReprise {
    OptionsReprise {
        vider,
        taille_lot: 2,
        ..Default::default()
    }
}

// ───────────── Étape 1 : écritures nouvelles, plus aucun BYTEA ─────────────

#[tokio::test]
async fn ecritures_nouvelles_vont_dans_le_magasin_et_jamais_en_bytea() {
    let Some((ent, pannes)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let pdf = octets(1, 300);

    // Messagerie.
    let meta = ent
        .attacher_piece_for_actor(
            Role::Direction.as_str(),
            AttacherPiece {
                etablissement_id: m.etab,
                message_id: m.message_id,
                nom: "courrier.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: pdf.clone(),
                actor_user_id: m.direction,
            },
        )
        .await
        .expect("attacher");
    let l = ligne(&ent, TablePiece::Messagerie, meta.id).await;
    assert_eq!(l.octets_en_base, 0, "aucun octet de fichier en base");
    let cle = l.storage_key.expect("clé d'objet");
    assert_eq!(l.sha256.as_deref(), Some(sha256_hex(&pdf).as_str()));
    assert!(!l.repris, "écrite directement dans le magasin : pas une reprise");
    assert!(cle.starts_with(&format!("{}/messages/{}.", m.etab, meta.id)));
    assert!(!cle.contains("courrier"), "E14 : le nom du fichier n'est pas dans la clé");
    assert_eq!(lire_msg(&ent, &m, meta.id).await, pdf);

    // PAI (santé).
    let ordo = ent
        .attacher_ordonnance_pai_for_actor(
            Role::Infirmerie.as_str(),
            AttacherOrdonnancePai {
                etablissement_id: m.etab,
                eleve_id: m.eleve,
                nom: "ordonnance-asthme.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: pdf.clone(),
                actor_user_id: m.infirmiere,
            },
        )
        .await
        .expect("ordonnance");
    let l = ligne(&ent, TablePiece::Pai, ordo.id).await;
    assert_eq!(l.octets_en_base, 0);
    let cle = l.storage_key.expect("clé");
    assert_eq!(l.sha256.as_deref(), Some(sha256_hex(&pdf).as_str()));
    assert!(!cle.contains("ordonnance") && !cle.contains("asthme"), "E14 : clé opaque");
    assert_eq!(lire_pai(&ent, &m, ordo.id).await, pdf);

    // Échec du magasin : l'erreur remonte, AUCUNE ligne n'est créée, jamais de repli en BYTEA.
    pannes.echec_ecriture.store(true, SeqCst);
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1")
        .bind(m.etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    let e = ent
        .attacher_piece_for_actor(
            Role::Direction.as_str(),
            AttacherPiece {
                etablissement_id: m.etab,
                message_id: m.message_id,
                nom: "autre.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: octets(2, 50),
                actor_user_id: m.direction,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Object(_)), "{e:?}");
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1")
        .bind(m.etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(avant, apres, "pas de ligne sans objet");
}

// ───────────── Lecture : l'objet d'abord, le BYTEA pour l'historique seulement ─────────────

#[tokio::test]
async fn lecture_prefere_l_objet_et_retombe_sur_bytea_pour_l_historique() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let original = octets(3, 200);
    let hm = hist_msg(&ent, &m, "histo.pdf", &original).await;
    let hp = hist_pai(&ent, &m, "histo-sante.pdf", &original).await;
    // Historique non repris : servi depuis le BYTEA.
    assert_eq!(lire_msg(&ent, &m, hm).await, original);
    assert_eq!(lire_pai(&ent, &m, hp).await, original);

    // Copie vérifiée SANS vidage : l'objet existe ; on altère le BYTEA pour prouver que la lecture
    // sert bien l'objet.
    for t in TablePiece::TOUTES {
        let r = ent
            .reprendre_pieces_etablissement(t, m.etab, &reel(false))
            .await
            .unwrap();
        assert!(r.en_echec.is_empty(), "{}", r.resume());
        assert_eq!(r.copiees, 1);
        assert_eq!(r.videes, 0);
    }
    for (t, id) in [(TablePiece::Messagerie, hm), (TablePiece::Pai, hp)] {
        sqlx::query(&format!("UPDATE {} SET contenu = '\\x00ff'::bytea WHERE id = $1", t.table()))
            .bind(id)
            .execute(ent.pool())
            .await
            .unwrap();
    }
    assert_eq!(lire_msg(&ent, &m, hm).await, original, "l'objet est préféré au BYTEA");
    assert_eq!(lire_pai(&ent, &m, hp).await, original);
}

// ───────────── Étape 2 : reprise ─────────────

#[tokio::test]
async fn reprise_sur_deux_etablissements_octet_pour_octet() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (a, b) = (monde(&ent).await, monde(&ent).await);
    assert_ne!(a.etab, b.etab);

    let mut attendu: Vec<(TablePiece, Uuid, Vec<u8>, bool)> = Vec::new(); // (table, id, octets, etab_a)
    for (i, (m, est_a)) in [(&a, true), (&b, false)].into_iter().enumerate() {
        for k in 0..3u8 {
            let by = octets(10 + k + 40 * i as u8, 120 + 37 * k as usize);
            attendu.push((TablePiece::Messagerie, hist_msg(&ent, m, &format!("n{k}.pdf"), &by).await, by, est_a));
            let by = octets(90 + k + 40 * i as u8, 80 + 11 * k as usize);
            attendu.push((TablePiece::Pai, hist_pai(&ent, m, &format!("s{k}.pdf"), &by).await, by, est_a));
        }
    }
    let objets_a = nb_objets(&ent, a.etab).await;
    let ecritures = ent.objects().put_count();

    // Simulation : n'écrit rien (ni objet, ni registre, ni ligne) mais voit tout.
    let sim = ent
        .reprendre_pieces(&TablePiece::TOUTES, Some(a.etab), &OptionsReprise { simulation: true, vider: true, ..Default::default() })
        .await
        .unwrap();
    assert!(sim.iter().all(|r| r.simulation && r.en_echec.is_empty() && r.videes == 0));
    assert_eq!(sim.iter().map(|r| r.copiees).sum::<u64>(), 6);
    assert_eq!(ent.objects().put_count(), ecritures, "aucune écriture en simulation");
    assert_eq!(nb_objets(&ent, a.etab).await, objets_a);
    for (t, id, by, _) in &attendu {
        let l = ligne(&ent, *t, *id).await;
        assert_eq!(l.octets_en_base, by.len() as i64);
        assert!(l.storage_key.is_none() && !l.repris);
    }

    // Reprise réelle de l'établissement A seulement, avec vidage.
    let rap = ent
        .reprendre_pieces(&TablePiece::TOUTES, Some(a.etab), &reel(true))
        .await
        .unwrap();
    for r in &rap {
        assert_eq!(r.etablissement_id, a.etab);
        assert!(r.en_echec.is_empty(), "{}", r.resume());
        assert_eq!((r.examinees, r.copiees, r.videes, r.restantes), (3, 3, 3, 0));
        assert!(r.octets_copies > 0);
    }
    for (t, id, by, est_a) in &attendu {
        let l = ligne(&ent, *t, *id).await;
        if *est_a {
            assert_eq!(l.octets_en_base, 0, "BYTEA vidé après vérification");
            assert!(l.repris);
            assert_eq!(l.sha256.as_deref(), Some(sha256_hex(by).as_str()));
            let cle = l.storage_key.unwrap();
            assert!(cle.starts_with(&format!("{}/", a.etab)), "clé de l'établissement A");
            assert!(!cle.contains("/n"), "E14 : clé opaque");
            let relu = match t {
                TablePiece::Messagerie => lire_msg(&ent, &a, *id).await,
                TablePiece::Pai => lire_pai(&ent, &a, *id).await,
            };
            assert_eq!(&relu, by, "lecture après reprise identique octet pour octet");
        } else {
            assert_eq!(l.octets_en_base, by.len() as i64, "l'établissement B n'a pas été touché");
            assert!(l.storage_key.is_none());
        }
    }
    assert_eq!(nb_objets(&ent, a.etab).await, objets_a + 6);

    // Tous les établissements : B est repris, A est « déjà repris » (idempotence).
    let tous = ent.reprendre_pieces(&TablePiece::TOUTES, None, &reel(true)).await.unwrap();
    let pour = |etab: Uuid, t: TablePiece| tous.iter().find(|r| r.etablissement_id == etab && r.table == t).unwrap();
    for t in TablePiece::TOUTES {
        // A n'a plus d'octets en base : inutile de le parcourir (idempotence).
        assert!(tous.iter().all(|r| r.etablissement_id != a.etab || r.examinees == 0));
        let ra = ent.reprendre_pieces_etablissement(t, a.etab, &reel(true)).await.unwrap();
        assert_eq!((ra.examinees, ra.copiees, ra.deja_reprises), (0, 0, 3), "{}", ra.resume());
        let rb = pour(b.etab, t);
        assert_eq!((rb.examinees, rb.copiees, rb.videes, rb.deja_reprises), (3, 3, 3, 0), "{}", rb.resume());
    }
    for (t, id, by, est_a) in &attendu {
        let m = if *est_a { &a } else { &b };
        let relu = match t {
            TablePiece::Messagerie => lire_msg(&ent, m, *id).await,
            TablePiece::Pai => lire_pai(&ent, m, *id).await,
        };
        assert_eq!(&relu, by);
        assert_eq!(ligne(&ent, *t, *id).await.octets_en_base, 0);
    }
    // Passage suivant sur A et B : rien à faire, aucune écriture. (La base de test est partagée :
    // d'autres scénarios y laissent des pièces en échec volontaire ; on ne regarde que A et B.)
    let n = ent.objects().put_count();
    for etab in [a.etab, b.etab] {
        let encore = ent.reprendre_pieces(&TablePiece::TOUTES, Some(etab), &reel(true)).await.unwrap();
        assert!(encore.iter().all(|r| r.examinees == 0 && r.en_echec.is_empty() && r.restantes == 0));
    }
    assert_eq!(ent.objects().put_count(), n);
}

#[tokio::test]
async fn interruption_au_milieu_puis_reprise_sans_doublon_ni_perte() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let mut pieces = Vec::new();
    for k in 0..5u8 {
        let by = octets(k, 64 + k as usize);
        pieces.push((hist_msg(&ent, &m, "x.pdf", &by).await, by));
    }
    let avant = nb_objets(&ent, m.etab).await;
    // Arrêt volontaire après 2 pièces.
    let r1 = ent
        .reprendre_pieces_etablissement(
            TablePiece::Messagerie,
            m.etab,
            &OptionsReprise { vider: true, taille_lot: 2, limite_pieces: Some(2), ..Default::default() },
        )
        .await
        .unwrap();
    assert!(r1.interrompue);
    assert_eq!((r1.examinees, r1.videes, r1.restantes), (2, 2, 3), "{}", r1.resume());
    // La reprise repart du reste, sans retraiter ni dupliquer.
    let r2 = ent
        .reprendre_pieces_etablissement(TablePiece::Messagerie, m.etab, &reel(true))
        .await
        .unwrap();
    assert!(!r2.interrompue && r2.en_echec.is_empty());
    assert_eq!((r2.deja_reprises, r2.examinees, r2.videes, r2.restantes), (2, 3, 3, 0), "{}", r2.resume());
    assert_eq!(nb_objets(&ent, m.etab).await, avant + 5, "un objet par pièce, aucun doublon");
    for (id, by) in &pieces {
        assert_eq!(&lire_msg(&ent, &m, *id).await, by);
    }
}

#[tokio::test]
async fn panne_du_stockage_ne_vide_jamais_contenu_puis_reprise_apres_retablissement() {
    let Some((ent, pannes)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let by = octets(5, 400);
    let id = hist_msg(&ent, &m, "sante-confidentiel.pdf", &by).await;
    let idp = hist_pai(&ent, &m, "ordonnance-secrete.pdf", &by).await;
    let objets = nb_objets(&ent, m.etab).await;

    let intact = |t: TablePiece, id: Uuid| {
        let ent = &ent;
        async move {
            let l = ligne(ent, t, id).await;
            assert_eq!(l.octets_en_base, 400 + 16, "contenu intact");
            assert!(l.storage_key.is_none() && l.sha256.is_none() && !l.repris);
        }
    };

    // 1. Écriture refusée.
    pannes.echec_ecriture.store(true, SeqCst);
    for (t, attendues) in [(TablePiece::Messagerie, 1), (TablePiece::Pai, 1)] {
        let r = ent.reprendre_pieces_etablissement(t, m.etab, &reel(true)).await.unwrap();
        assert_eq!(r.en_echec.len(), attendues, "{}", r.resume());
        assert!(r.en_echec.iter().all(|e| e.cause == CauseEchec::StockageIndisponible));
        assert_eq!((r.copiees, r.videes, r.restantes), (0, 0, attendues as u64));
        assert!(!r.resume().contains("secrete") && !r.resume().contains("confidentiel"));
    }
    intact(TablePiece::Messagerie, id).await;
    intact(TablePiece::Pai, idp).await;
    pannes.echec_ecriture.store(false, SeqCst);

    // 2. Relecture corrompue côté stockage : le BYTEA reste la seule copie fiable.
    pannes.lecture_corrompue.store(true, SeqCst);
    let r = ent.reprendre_pieces_etablissement(TablePiece::Pai, m.etab, &reel(true)).await.unwrap();
    assert_eq!(r.en_echec[0].cause, CauseEchec::RelectureDifferente, "{}", r.resume());
    intact(TablePiece::Pai, idp).await;
    pannes.lecture_corrompue.store(false, SeqCst);
    assert_eq!(nb_objets(&ent, m.etab).await, objets, "aucune ligne de registre pour un échec");

    // 3. Stockage rétabli : la même commande aboutit.
    let rap = ent.reprendre_pieces(&TablePiece::TOUTES, Some(m.etab), &reel(true)).await.unwrap();
    assert!(rap.iter().all(|r| r.en_echec.is_empty() && r.videes == 1), "{rap:?}");
    assert_eq!(lire_msg(&ent, &m, id).await, by);
    assert_eq!(lire_pai(&ent, &m, idp).await, by);
}

#[tokio::test]
async fn incoherences_refusees_sans_toucher_la_ligne() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let by = octets(6, 100);

    // Taille annoncée différente des octets.
    let id_t = hist_msg(&ent, &m, "a.pdf", &by).await;
    sqlx::query("UPDATE ent_msg_piece SET taille_octets = 5 WHERE id = $1").bind(id_t).execute(ent.pool()).await.unwrap();
    // Objet déjà présent sous la clé de reprise avec un AUTRE contenu (immuable : refusé).
    let id_o = hist_msg(&ent, &m, "b.pdf", &by).await;
    let cle_o = format!("{}/messages/{id_o}.pdf", m.etab);
    ent.objects().put(&cle_o, b"un autre contenu").unwrap();
    // Registre annonçant une autre empreinte pour la clé.
    let id_r = hist_msg(&ent, &m, "c.pdf", &by).await;
    let cle_r = format!("{}/messages/{id_r}.pdf", m.etab);
    crate::objets::enregistrer_objet(
        ent.pool(),
        m.etab,
        &ent_object::ObjectInfo { key: cle_r, taille: 3, sha256: sha256_hex(b"autre") },
        None,
    )
    .await
    .unwrap();
    // Ligne dont l'empreinte annoncée n'est pas celle des octets.
    let id_s = hist_msg(&ent, &m, "d.pdf", &by).await;
    sqlx::query("UPDATE ent_msg_piece SET sha256 = $2 WHERE id = $1")
        .bind(id_s)
        .bind(sha256_hex(b"autre chose"))
        .execute(ent.pool())
        .await
        .unwrap();

    let r = ent.reprendre_pieces_etablissement(TablePiece::Messagerie, m.etab, &reel(true)).await.unwrap();
    let cause = |id: Uuid| r.en_echec.iter().find(|e| e.piece_id == id).map(|e| e.cause);
    assert_eq!(cause(id_t), Some(CauseEchec::TailleIncoherente));
    assert_eq!(cause(id_o), Some(CauseEchec::ObjetDejaPresentDifferent));
    assert_eq!(cause(id_r), Some(CauseEchec::RegistreIncoherent));
    assert_eq!(cause(id_s), Some(CauseEchec::RegistreIncoherent));
    assert_eq!((r.videes, r.restantes), (0, 4), "{}", r.resume());
    for id in [id_t, id_o, id_r, id_s] {
        assert!(ligne(&ent, TablePiece::Messagerie, id).await.octets_en_base > 0);
    }
}

#[tokio::test]
async fn objet_deja_copie_a_l_identique_est_adopte_et_la_reprise_en_deux_temps_fonctionne() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let by = octets(7, 150);
    let base = nb_objets(&ent, m.etab).await;
    let id = hist_pai(&ent, &m, "o.pdf", &by).await;
    // Objet recopié auparavant par un lien court (même clé, même contenu) : adopté, pas dupliqué.
    let cle = format!("{}/pai/{id}.pdf", m.etab);
    ent.objects().put(&cle, &by).unwrap();

    // Temps 1 : copie vérifiée sans vidage (avant le point de sauvegarde).
    let r1 = ent.reprendre_pieces_etablissement(TablePiece::Pai, m.etab, &reel(false)).await.unwrap();
    assert_eq!((r1.copiees, r1.videes, r1.restantes), (1, 0, 1), "{}", r1.resume());
    let l = ligne(&ent, TablePiece::Pai, id).await;
    assert_eq!(l.storage_key.as_deref(), Some(cle.as_str()));
    assert_eq!(l.octets_en_base, by.len() as i64, "BYTEA conservé tant que le vidage n'est pas demandé");
    // Temps 2 : sauvegarde vérifiée → revérification depuis le stockage puis vidage.
    let r2 = ent.reprendre_pieces_etablissement(TablePiece::Pai, m.etab, &reel(true)).await.unwrap();
    assert_eq!((r2.reverifiees, r2.copiees, r2.videes, r2.restantes), (1, 0, 1, 0), "{}", r2.resume());
    assert_eq!(ligne(&ent, TablePiece::Pai, id).await.octets_en_base, 0);
    assert_eq!(lire_pai(&ent, &m, id).await, by);
    assert_eq!(nb_objets(&ent, m.etab).await, base + 1, "un seul objet : adopté, pas dupliqué");
}

/// Un PUT S3 est atomique : une écriture tronquée n'existe qu'en simulation. Si pareil objet
/// restait sous sa clé immuable, la reprise le détecte, ne vide RIEN et laisse la pièce en échec
/// (intervention humaine), plutôt que de faire confiance à un objet dont l'empreinte ne colle pas.
#[tokio::test]
async fn ecriture_tronquee_detectee_et_jamais_suivie_d_un_vidage() {
    let Some((ent, pannes)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let by = octets(8, 300);
    let id = hist_pai(&ent, &m, "t.pdf", &by).await;
    let objets = nb_objets(&ent, m.etab).await;
    pannes.ecriture_tronquee.store(true, SeqCst);
    let r = ent.reprendre_pieces_etablissement(TablePiece::Pai, m.etab, &reel(true)).await.unwrap();
    assert_eq!(r.en_echec[0].cause, CauseEchec::RelectureDifferente, "{}", r.resume());
    pannes.ecriture_tronquee.store(false, SeqCst);
    let l = ligne(&ent, TablePiece::Pai, id).await;
    assert_eq!(l.octets_en_base, by.len() as i64);
    assert!(l.storage_key.is_none() && l.sha256.is_none());
    assert_eq!(nb_objets(&ent, m.etab).await, objets);
    assert_eq!(lire_pai(&ent, &m, id).await, by, "l'historique reste lisible depuis le BYTEA");
}
