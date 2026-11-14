//! D3 — chiffrement applicatif des champs sensibles, contre une base locale de test.
//! Les tests base sont ignorés (skip) sans `ENT_DATABASE_URL`.

use super::*;
use crate::champs_chiffres::ChampsSensibles;
use chrono::{NaiveDate, Utc};
use domain_eleve::{CreateEleve, CreerMandatSepa, DomainError, UpsertDossierPsy, UpsertFicheSante};
use sqlx::Row;
use uuid::Uuid;

const IBAN: &str = "FR7630006000011234567890189";

async fn connect_local() -> Option<PgEnt> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(ent)
}

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    eleve: Uuid,
    user: Uuid,
}

async fn cadre() -> Option<Cadre> {
    let ent = connect_local().await?;
    let etab = Uuid::new_v4();
    let eleve = ent
        .create_eleve(CreateEleve {
            etablissement_id: etab,
            nom: "Chiffre".into(),
            prenom: "D3".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: Some("6A".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .expect("eleve");
    let user = ent
        .register_user(
            &format!("d3-{}@ent.local", Uuid::new_v4()),
            "DevLocalPass1!",
            None,
            etab,
            "direction",
        )
        .await
        .expect("user");
    Some(Cadre {
        ent,
        etab,
        eleve: eleve.id,
        user: user.id,
    })
}

async fn brut(ent: &PgEnt, table: &str, col: &str, where_col: &str, id: Uuid) -> Option<String> {
    sqlx::query(&format!(
        "SELECT {col} AS v FROM {table} WHERE {where_col} = $1"
    ))
    .bind(id)
    .fetch_optional(ent.pool())
    .await
    .expect("select brut")
    .and_then(|r| r.get::<Option<String>, _>("v"))
}

fn fiche(c: &Cadre, allergies: &str, note: &str) -> UpsertFicheSante {
    UpsertFicheSante {
        etablissement_id: c.etab,
        eleve_id: c.eleve,
        allergies: allergies.into(),
        traitements: "Ventoline 2 bouffées".into(),
        contre_indications: "Aspirine".into(),
        pai_actif: false,
        medecin_nom: Some("Dr Martin".into()),
        urgence_tel: Some("0612345678".into()),
        note_confidentielle: note.into(),
        actor_user_id: c.user,
    }
}

#[tokio::test]
async fn iban_chiffre_en_base_masque_dans_le_journal_et_relu_en_clair() {
    let Some(c) = cadre().await else {
        eprintln!("skip D3: ENT_DATABASE_URL");
        return;
    };
    let foyer = c.ent.create_foyer(c.etab, Some("Foyer D3")).await.unwrap();
    let m = c
        .ent
        .creer_mandat_sepa_for_actor(
            "direction",
            c.user,
            CreerMandatSepa {
                etablissement_id: c.etab,
                foyer_id: foyer,
                rum: format!("RUM{}", &Uuid::new_v4().simple().to_string()[..12]),
                iban: IBAN.into(),
                bic: Some("AGRIFRPP".into()),
                titulaire: "Lily Potter".into(),
                signe_at: Utc::now(),
                actor_user_id: c.user,
            },
        )
        .await
        .expect("mandat");
    // Retour de service : valeurs claires (usage métier / export).
    assert_eq!(m.iban, IBAN);
    // Base : aucune valeur en clair.
    for col in ["iban", "bic", "titulaire"] {
        let v = brut(&c.ent, "ent_fac_sepa_mandat", col, "id", m.id)
            .await
            .unwrap();
        assert!(v.starts_with("enc:v1:"), "{col} en clair : {v}");
        assert!(!v.contains("FR76") && !v.contains("Potter") && !v.contains("AGRIFRPP"));
    }
    let histo = c
        .ent
        .list_iban_histo_foyer_for_actor(c.etab, c.user, "direction", foyer, 10)
        .await
        .unwrap();
    assert_eq!(histo.len(), 1);
    assert_eq!(histo[0].iban, IBAN);
    let h_iban = brut(&c.ent, "ent_foyer_iban_histo", "iban", "id", histo[0].id)
        .await
        .unwrap();
    assert!(h_iban.starts_with("enc:v1:") && !h_iban.contains("FR76"));
    // Journal d'événements : aucun IBAN complet (masqué).
    let payload: String = sqlx::query(
        "SELECT payload::text AS p FROM ent_metier_event \
         WHERE etablissement_id = $1 AND aggregate_id = $2",
    )
    .bind(c.etab)
    .bind(histo[0].id)
    .fetch_one(c.ent.pool())
    .await
    .expect("événement")
    .get("p");
    assert!(
        !payload.contains("30006000"),
        "IBAN en clair dans l'événement : {payload}"
    );
    assert!(payload.contains("FR76 •••• 0189"), "{payload}");
    // Lecture par liste.
    let l = c
        .ent
        .list_mandats_sepa_for_actor(c.etab, c.user, "direction")
        .await
        .unwrap();
    assert!(l
        .iter()
        .any(|x| x.id == m.id && x.iban == IBAN && x.titulaire == "Lily Potter"));
}

#[tokio::test]
async fn fiche_sante_et_dossier_psy_chiffres_puis_relus_meme_apres_mise_a_jour() {
    let Some(c) = cadre().await else {
        eprintln!("skip D3: ENT_DATABASE_URL");
        return;
    };
    let f = c
        .ent
        .upsert_fiche_sante_for_actor("infirmerie", fiche(&c, "arachides", "note secrète"))
        .await
        .expect("fiche");
    for col in [
        "allergies",
        "traitements",
        "contre_indications",
        "medecin_nom",
        "urgence_tel",
        "note_confidentielle",
    ] {
        let v = brut(&c.ent, "ent_fiche_sante", col, "eleve_id", c.eleve)
            .await
            .unwrap();
        assert!(v.starts_with("enc:v1:"), "{col} en clair : {v}");
    }
    // Seconde écriture : l'id de ligne est conservé, la fiche reste lisible.
    let f2 = c
        .ent
        .upsert_fiche_sante_for_actor("infirmerie", fiche(&c, "arachides, kiwi", "note 2"))
        .await
        .expect("fiche 2");
    assert_eq!(f2.id, f.id);
    let relue = c
        .ent
        .get_fiche_sante_for_actor(c.etab, c.user, "infirmerie", c.eleve)
        .await
        .unwrap()
        .expect("fiche relue");
    assert_eq!(relue.allergies, "arachides, kiwi");
    assert_eq!(relue.note_confidentielle, "note 2");
    assert_eq!(relue.medecin_nom.as_deref(), Some("Dr Martin"));

    // Dossier psy.
    let d = |note: &str| UpsertDossierPsy {
        etablissement_id: c.etab,
        eleve_id: c.eleve,
        axes_suivi: "Anxiété de performance".into(),
        contacts_externes: "CMP Nord 0102030405".into(),
        note_confidentielle: note.into(),
        suivi_actif: true,
        signale_famille: false,
        actor_user_id: c.user,
    };
    c.ent
        .upsert_dossier_psy_for_actor("psychologue", d("n1"))
        .await
        .expect("dossier");
    c.ent
        .upsert_dossier_psy_for_actor("psychologue", d("n2"))
        .await
        .expect("dossier 2");
    for col in ["axes_suivi", "contacts_externes", "note_confidentielle"] {
        let v = brut(&c.ent, "ent_psy_dossier", col, "eleve_id", c.eleve)
            .await
            .unwrap();
        assert!(v.starts_with("enc:v1:"), "{col} en clair : {v}");
    }
    let dossier = c
        .ent
        .get_dossier_psy_for_actor(c.etab, c.user, "psychologue", c.eleve)
        .await
        .unwrap()
        .expect("dossier relu");
    assert_eq!(dossier.note_confidentielle, "n2");
    assert_eq!(dossier.contacts_externes, "CMP Nord 0102030405");
}

#[tokio::test]
async fn chiffre_copie_dans_une_autre_ligne_refuse_et_heritage_en_clair_rechiffre() {
    let Some(c) = cadre().await else {
        eprintln!("skip D3: ENT_DATABASE_URL");
        return;
    };
    c.ent
        .upsert_fiche_sante_for_actor("infirmerie", fiche(&c, "latex", "n"))
        .await
        .expect("fiche");
    // Autre élève (même établissement) avec sa propre fiche.
    let autre = c
        .ent
        .create_eleve(CreateEleve {
            etablissement_id: c.etab,
            nom: "Autre".into(),
            prenom: "Eleve".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: Some("6A".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .expect("autre");
    let mut cmd = fiche(&c, "rien", "x");
    cmd.eleve_id = autre.id;
    c.ent
        .upsert_fiche_sante_for_actor("infirmerie", cmd)
        .await
        .expect("fiche autre");

    // Attaque : recopie du chiffré d'une ligne dans l'autre → illisible (AAD = id de ligne).
    let vole = brut(&c.ent, "ent_fiche_sante", "allergies", "eleve_id", c.eleve)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_fiche_sante SET allergies = $1 WHERE eleve_id = $2")
        .bind(&vole)
        .bind(autre.id)
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(c
        .ent
        .get_fiche_sante_for_actor(c.etab, c.user, "infirmerie", autre.id)
        .await
        .is_err());
    // La ligne d'origine reste lisible.
    assert!(c
        .ent
        .get_fiche_sante_for_actor(c.etab, c.user, "infirmerie", c.eleve)
        .await
        .is_ok());

    // Héritage en clair : relu tel quel, puis rechiffré par la commande.
    sqlx::query(
        "UPDATE ent_fiche_sante SET allergies = 'ancienne valeur en clair', \
         note_confidentielle = 'ancienne note' WHERE eleve_id = $1",
    )
    .bind(autre.id)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let lue = c
        .ent
        .get_fiche_sante_for_actor(c.etab, c.user, "infirmerie", autre.id)
        .await
        .unwrap()
        .expect("fiche");
    assert_eq!(lue.allergies, "ancienne valeur en clair");
    let r = c
        .ent
        .rechiffrer_champs_sensibles()
        .await
        .expect("rechiffrement");
    assert!(r.champs_rechiffres >= 2, "{r:?}");
    let v = brut(&c.ent, "ent_fiche_sante", "allergies", "eleve_id", autre.id)
        .await
        .unwrap();
    assert!(v.starts_with("enc:v1:") && !v.contains("ancienne"));
    let relue = c
        .ent
        .get_fiche_sante_for_actor(c.etab, c.user, "infirmerie", autre.id)
        .await
        .unwrap()
        .expect("fiche");
    assert_eq!(relue.allergies, "ancienne valeur en clair");
    assert_eq!(relue.note_confidentielle, "ancienne note");
    // Idempotent.
    assert_eq!(
        c.ent
            .rechiffrer_champs_sensibles()
            .await
            .unwrap()
            .champs_rechiffres,
        0
    );
}

#[tokio::test]
async fn sans_cle_aucune_ecriture_en_clair() {
    let Some(c) = cadre().await else {
        eprintln!("skip D3: ENT_DATABASE_URL");
        return;
    };
    let repo = crate::infirmerie_repo::PgInfirmerieRepo::new(c.ent.pool().clone());
    let repo = repo.avec_champs(ChampsSensibles::sans_cle());
    let f = domain_eleve::FicheSante {
        id: Uuid::new_v4(),
        etablissement_id: c.etab,
        eleve_id: c.eleve,
        allergies: "gluten".into(),
        traitements: String::new(),
        contre_indications: String::new(),
        pai_actif: false,
        medecin_nom: None,
        urgence_tel: None,
        note_confidentielle: String::new(),
        updated_by_user_id: c.user,
        updated_at: Utc::now(),
    };
    assert_eq!(
        repo.upsert_fiche(&f).await.unwrap_err(),
        DomainError::Infrastructure
    );
    assert!(
        brut(&c.ent, "ent_fiche_sante", "allergies", "eleve_id", c.eleve)
            .await
            .is_none()
    );
}
