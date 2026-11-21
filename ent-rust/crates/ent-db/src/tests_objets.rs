//! Stockage d'objets (F-013, PRA E5) : registre `ent_objet`, empreinte et taille,
//! immuabilité, vérification à la lecture, et **aucun échec ignoré**.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés (échec en CI).

use super::*;
use ent_object::provider::{FaultyProvider, MemoryProvider, Pannes};
use ent_object::{sha256_hex, ObjectStore};
use sqlx::Row;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use uuid::Uuid;

const PDF: &[u8] = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n";

async fn ent_avec_pannes() -> Option<(PgEnt, Arc<Pannes>)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let (f, pannes) = FaultyProvider::new(Arc::new(MemoryProvider::new()));
    ent.installer_objets(ObjectStore::with_provider(Arc::new(f)));
    Some((ent, pannes))
}

async fn ligne(ent: &PgEnt, key: &str) -> Option<(i64, String, Option<String>)> {
    sqlx::query("SELECT taille, sha256, mime FROM ent_objet WHERE cle_objet = $1")
        .bind(key)
        .fetch_optional(ent.pool())
        .await
        .unwrap()
        .map(|r| (r.get(0), r.get(1), r.get(2)))
}

#[tokio::test]
async fn objet_enregistre_avec_empreinte_et_relu_verifie() {
    let Some((ent, pannes)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast = ent.seed_labo_cast().await.expect("seed");
    let etab = cast.etablissement_id;
    let key = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    let info = ent
        .stocker_objet(etab, &key, Some("application/pdf"), PDF)
        .await
        .unwrap();
    let (taille, sha, mime) = ligne(&ent, &key).await.expect("ligne de registre");
    assert_eq!(taille as usize, PDF.len());
    assert_eq!(sha, sha256_hex(PDF));
    assert_eq!(sha, info.sha256);
    assert_eq!(mime.as_deref(), Some("application/pdf"));
    assert_eq!(ent.lire_objet(etab, &key).await.unwrap(), PDF);

    // Corruption au repos côté fournisseur : la lecture échoue, elle ne sert pas d'octets altérés.
    pannes.lecture_corrompue.store(true, SeqCst);
    let e = ent.lire_objet(etab, &key).await.unwrap_err();
    assert!(
        matches!(e, EntDbError::Object(ObjectStoreError::Integrity)),
        "{e:?}"
    );
    pannes.lecture_corrompue.store(false, SeqCst);
    assert_eq!(ent.objects().metrics().integrite_echec, 1);
}

#[tokio::test]
async fn contenu_different_de_la_base_est_refuse() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast = ent.seed_labo_cast().await.expect("seed");
    let etab = cast.etablissement_id;
    // L'objet existe chez le fournisseur mais le registre dit autre chose : remplacement détecté.
    let key = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    ent.objects().put(&key, b"contenu remplace").unwrap();
    crate::objets::enregistrer_objet(
        ent.pool(),
        etab,
        &ent_object::ObjectInfo {
            key: key.clone(),
            taille: 16,
            sha256: sha256_hex(b"contenu original"),
        },
        None,
    )
    .await
    .unwrap();
    let e = ent.lire_objet(etab, &key).await.unwrap_err();
    assert!(
        matches!(e, EntDbError::Object(ObjectStoreError::Integrity)),
        "{e:?}"
    );
}

#[tokio::test]
async fn echec_d_ecriture_remonte_et_rien_n_est_enregistre() {
    let Some((ent, pannes)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast = ent.seed_labo_cast().await.expect("seed");
    let etab = cast.etablissement_id;
    pannes.echec_ecriture.store(true, SeqCst);
    let key = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    let e = ent.stocker_objet(etab, &key, None, PDF).await.unwrap_err();
    assert!(
        matches!(e, EntDbError::Object(ObjectStoreError::Backend(_))),
        "{e:?}"
    );
    assert!(
        ligne(&ent, &key).await.is_none(),
        "aucune ligne pour un objet non écrit"
    );
    assert_eq!(ent.objects().metrics().ecritures_echec, 1);
    // Écriture tronquée silencieusement : détectée, rien n'est enregistré.
    pannes.echec_ecriture.store(false, SeqCst);
    pannes.ecriture_tronquee.store(true, SeqCst);
    let key2 = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    let e = ent.stocker_objet(etab, &key2, None, PDF).await.unwrap_err();
    assert!(
        matches!(e, EntDbError::Object(ObjectStoreError::Integrity)),
        "{e:?}"
    );
    assert!(ligne(&ent, &key2).await.is_none());
}

#[tokio::test]
async fn depot_de_document_echoue_si_le_stockage_echoue() {
    let Some((ent, pannes)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast = ent.seed_labo_cast().await.expect("seed");
    let etab = cast.etablissement_id;
    let cmd = || domain_eleve::DeposerDocument {
        etablissement_id: etab,
        eleve_id: cast.eleve_lea_id,
        tiroir: domain_eleve::DocTiroir::Scolaire,
        titre: format!("Justificatif {}", Uuid::new_v4().simple()),
        code: None,
        mime_type: Some("application/pdf".into()),
        storage_key: None,
        confidentialite: domain_eleve::DocConfidentialite::Restreint,
        actor_user_id: cast.direction.user_id,
    };
    pannes.echec_ecriture.store(true, SeqCst);
    let e = ent
        .deposer_document_with_contenu(cmd(), Some(PDF))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Object(ObjectStoreError::Backend(_))),
        "{e:?}"
    );
    pannes.echec_ecriture.store(false, SeqCst);

    let doc = ent
        .deposer_document_with_contenu(cmd(), Some(PDF))
        .await
        .unwrap();
    let key = doc.storage_key.clone().expect("clé objet");
    assert!(key.starts_with(&format!("{etab}/documents/")));
    assert!(
        ligne(&ent, &key).await.is_some(),
        "objet enregistré dans le registre"
    );
    let (_, octets) = ent
        .telecharger_document_bytes_for_actor(etab, cast.direction.user_id, "direction", doc.id)
        .await
        .unwrap();
    assert_eq!(octets, PDF);
    // Lecture altérée : erreur visible, jamais le « stub » de repli.
    pannes.lecture_corrompue.store(true, SeqCst);
    let e = ent
        .telecharger_document_bytes_for_actor(etab, cast.direction.user_id, "direction", doc.id)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Object(ObjectStoreError::Integrity)),
        "{e:?}"
    );
}

#[tokio::test]
async fn registre_immuable_en_base() {
    let Some((ent, _)) = ent_avec_pannes().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast = ent.seed_labo_cast().await.expect("seed");
    let etab = cast.etablissement_id;
    let key = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    ent.stocker_objet(etab, &key, None, PDF).await.unwrap();
    // Aucune modification du contenu enregistré, aucun DELETE.
    for sql in [
        "UPDATE ent_objet SET sha256 = repeat('0', 64) WHERE cle_objet = $1",
        "UPDATE ent_objet SET taille = 1 WHERE cle_objet = $1",
        "DELETE FROM ent_objet WHERE cle_objet = $1",
    ] {
        let r = sqlx::query(sql).bind(&key).execute(ent.pool()).await;
        assert!(r.is_err(), "doit être refusé : {sql}");
    }
    // La clé doit commencer par l'établissement (cloisonnement).
    let autre = ObjectStore::key_for(Uuid::new_v4(), "documents", Uuid::new_v4(), "pdf");
    let r = crate::objets::enregistrer_objet(
        ent.pool(),
        etab,
        &ent_object::ObjectInfo {
            key: autre,
            taille: 1,
            sha256: sha256_hex(b"x"),
        },
        None,
    )
    .await;
    assert!(r.is_err(), "clé d'un autre établissement refusée");
    // Suppression logique datée : permise une fois, puis figée.
    sqlx::query("UPDATE ent_objet SET supprime_le = now() WHERE cle_objet = $1")
        .bind(&key)
        .execute(ent.pool())
        .await
        .unwrap();
    let r = sqlx::query("UPDATE ent_objet SET supprime_le = NULL WHERE cle_objet = $1")
        .bind(&key)
        .execute(ent.pool())
        .await;
    assert!(r.is_err());
}
