//! Socle 41, lot I4 — **passe 2** (contre-revue du pilote, D-069) testée contre la base locale de test (feature `labo`).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande, y compris la tâche planifiée
//! (`internat_sorties_tache_a`), reçoit son instant ; les jours sont fixes (avril 2026, Paris = UTC + 2).
//!
//! | Constat | Test(s) |
//! |---|---|
//! | É1 (l'élève en activité reste suivi : annulation, désinscription refusée, retour sur un temps annulé) | `e1_*` |
//! | M1 (un cas existant n'est jamais rouvert ; un cas de l'appel est rattaché) | `m1_*` |
//! | M2 (un point de reprise par ligne) | `m2_*` |
//! | M3 (étude : absent signalé ; avis aux responsables) | `m3_*` |
//! | M4 (accord annuel « soirée » : jamais dans la nuit) | `m4_*` |
//! | M5 (registre des données) | `m5_*` |
//! | M6 (branchement de la tâche, sortie à cheval sur minuit) | `m6_*` |
//! | M7 (protégés comptés, sans nom) | `m7_*` |
//! | F-a, F-b, F-c, F-g | `fa_*`, `fb_*`, `fc_*`, `fg_*` |

use super::tests_internat_i2::{banc, refus_de, Banc, CPE};
use super::tests_internat_i3::{avril, parent, paris};
use super::tests_internat_i4::*;
use super::tests_internat_i4_correctifs::{cas_de, local, sortie_de, sortie_prete};
use super::*;
use chrono::{DateTime, Duration, Utc};
use ent_auth::Action;
use ent_types::internat_nuit::CasIssueSaisieDto;
use ent_types::internat_vie::*;
use uuid::Uuid;

/// Une activité du soir de 20 h 30 à 21 h 30 (15 avril), encadrée par `b.prof`, où `eleves` sont inscrits et pointés
/// « en activité » à 20 h 40.
pub(crate) async fn activite_en_cours(b: &Banc, eleves: &[Uuid]) -> TempsDto {
    let mut a = etude(None);
    a.nature = "activite".into();
    a.libelle = "Atelier théâtre".into();
    a.encadrant_user_id = Some(b.prof);
    a.debut = paris(avril(15), 20, 30);
    a.fin = paris(avril(15), 21, 30);
    let act = b
        .ent
        .creer_temps_a(b.etab, b.cpe, CPE, &a, t0())
        .await
        .unwrap();
    for e in eleves {
        b.ent
            .inscrire_temps_a(
                b.etab,
                b.cpe,
                CPE,
                act.id,
                &InscriptionTempsDto { eleve_id: *e },
                mi_etude(1),
            )
            .await
            .unwrap();
        pointer_temps(b, act.id, *e, "present", paris(avril(15), 20, 40))
            .await
            .unwrap();
    }
    act
}

pub(crate) async fn pointer_temps(
    b: &Banc,
    temps: Uuid,
    e: Uuid,
    etat: &str,
    a: DateTime<Utc>,
) -> Result<(), EntDbError> {
    b.ent
        .pointer_temps_a(
            b.etab,
            b.prof,
            PROF,
            temps,
            &PointageTempsDto {
                eleve_id: e,
                etat: etat.into(),
            },
            a,
        )
        .await
        .map(|_| ())
}

/// Distribue les événements de l'établissement de test (consommateurs standard).
pub(crate) async fn distribuer(b: &Banc) {
    b.ent
        .distributeur(crate::outbox::ConfigDistribution {
            taille_lot: 100,
            max_tentatives: 3,
            delai_base: std::time::Duration::ZERO,
            delai_max: std::time::Duration::ZERO,
            intervalle: std::time::Duration::from_millis(20),
            etablissement: Some(b.etab),
        })
        .distribuer_tout()
        .await
        .unwrap();
}

pub(crate) fn fin_activite() -> DateTime<Utc> {
    paris(avril(15), 21, 30)
}

// ───────────────────────── É1 ─────────────────────────

#[tokio::test]
async fn e1_annuler_un_temps_garde_en_veille_les_eleves_en_activite_et_leur_retour_ferme_le_cas() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let act = activite_en_cours(&b, &[e1, e2]).await;
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            act.id,
            &InscriptionTempsDto { eleve_id: e3 },
            mi_etude(2),
        )
        .await
        .unwrap();
    // L'annulation reste possible (c'est le cas de l'urgence), avec des élèves en activité.
    let t = b
        .ent
        .annuler_temps_a(b.etab, b.cpe, CPE, act.id, paris(avril(15), 21, 0))
        .await
        .unwrap();
    assert_eq!(t.statut, "annule");
    // Un temps annulé ne reçoit plus de départ (e3 n'est pas parti)…
    assert!(refus_de(
        &pointer_temps(&b, act.id, e3, "present", paris(avril(15), 21, 5)).await,
        "annulé"
    ));
    // … mais la veille suit les deux élèves partis : un cas chacun.
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin_activite() + Duration::minutes(16))
            .await
            .unwrap(),
        2
    );
    for e in [e1, e2] {
        let cas = cas_de(&b, e).await;
        assert_eq!(
            cas.len(),
            1,
            "un cas pour l'élève en activité d'un temps annulé"
        );
        assert_eq!(cas[0].1, "non_rentre");
        assert!(cas[0].4.is_none());
    }
    // La marque « en activité » de l'appel suit aussi l'élève d'un temps annulé : à l'ouverture, ni absent ni cas de plus.
    // Le retour se pointe sur le temps annulé (sans fenêtre horaire) et ferme le cas ; seul le retour, pas un départ.
    pointer_temps(
        &b,
        act.id,
        e1,
        "rentre",
        fin_activite() + Duration::minutes(25),
    )
    .await
    .unwrap();
    assert_eq!(cas_de(&b, e1).await[0].4.as_deref(), Some("retrouve"));
    assert!(refus_de(
        &pointer_temps(
            &b,
            act.id,
            e1,
            "rentre",
            fin_activite() + Duration::minutes(26)
        )
        .await,
        "rentre"
    ));
    assert!(refus_de(
        &pointer_temps(
            &b,
            act.id,
            e1,
            "absent",
            fin_activite() + Duration::minutes(26)
        )
        .await,
        "annulé"
    ));
    // Longtemps après (aucune fenêtre) : e2 rentre encore.
    pointer_temps(
        &b,
        act.id,
        e2,
        "rentre",
        fin_activite() + Duration::hours(10),
    )
    .await
    .unwrap();
    assert_eq!(cas_de(&b, e2).await[0].4.as_deref(), Some("retrouve"));
    // Les deux sont rentrés : la veille ne rouvre rien.
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin_activite() + Duration::hours(11))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn e1_la_desinscription_d_un_eleve_en_activite_est_refusee_jusqu_a_son_retour() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let act = activite_en_cours(&b, &[e1]).await;
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            act.id,
            &InscriptionTempsDto { eleve_id: e2 },
            mi_etude(2),
        )
        .await
        .unwrap();
    let des = |e: Uuid, a: DateTime<Utc>| {
        let b = &b;
        async move {
            b.ent
                .desinscrire_temps_a(b.etab, b.cpe, CPE, act.id, e, a)
                .await
        }
    };
    // e1 est parti, pas encore rentré : refusé, avec un message explicite.
    let r = des(e1, paris(avril(15), 21, 0)).await;
    assert!(refus_de(&r, "parti en activité"), "{r:?}");
    // e2 (non pointé) se désinscrit normalement.
    assert!(des(e2, paris(avril(15), 21, 0)).await.is_ok());
    // Après son retour pointé, e1 se désinscrit.
    pointer_temps(&b, act.id, e1, "rentre", paris(avril(15), 21, 10))
        .await
        .unwrap();
    assert!(des(e1, paris(avril(15), 21, 15)).await.is_ok());
}

// ───────────────────────── M1 ─────────────────────────

pub(crate) fn apres_fin(mi: i64) -> DateTime<Utc> {
    fin() + Duration::minutes(mi)
}

pub(crate) async fn nuit_de(b: &Banc, a: DateTime<Utc>) -> chrono::NaiveDate {
    let mut tx = crate::tenant_tx::TenantTx::begin(b.ent.pool(), b.etab)
        .await
        .unwrap();
    let n = crate::internat_nuit::nuit_a_tx(&mut tx, b.etab, a)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    n
}

async fn ouvrir_cas_appel(b: &Banc, e: Uuid, a: DateTime<Utc>) -> Uuid {
    let nuit = nuit_de(b, apres_fin(16)).await;
    let mut tx = crate::tenant_tx::TenantTx::begin(b.ent.pool(), b.etab)
        .await
        .unwrap();
    let id = b
        .ent
        .ouvrir_cas_tx(
            &mut tx,
            b.etab,
            Some(b.cpe),
            None,
            e,
            nuit,
            "non_retrouve",
            a,
        )
        .await
        .unwrap()
        .expect("cas ouvert par l'appel");
    tx.commit().await.unwrap();
    id
}

#[tokio::test]
async fn m1_un_cas_existant_n_est_jamais_rouvert_et_un_cas_de_l_appel_est_rattache() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let (p3, _) = parent(&b, e3, true).await;
    let g = sortie_prete(&b, &[(e1, p1), (e2, p2), (e3, p3)]).await;
    depart(&b, g.id, vec![e1, e2, e3]).await;
    // e2 : cas OUVERT par l'appel avant la veille ; e3 : cas ouvert puis FERMÉ par l'appel (élève pointé présent).
    let cas2 = ouvrir_cas_appel(&b, e2, apres_fin(5)).await;
    let cas3 = ouvrir_cas_appel(&b, e3, apres_fin(5)).await;
    sqlx::query(
        "UPDATE ent_internat_cas SET issue = 'retrouve', issue_le = $3, issue_par_user_id = $4, issue_lieu_code = 'chambre' \
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(b.etab)
    .bind(cas3)
    .bind(apres_fin(8))
    .bind(b.cpe)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    // La veille n'ouvre un cas que pour e1.
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(16))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        cas_de(&b, e3).await.len(),
        1,
        "cas fermé par l'appel : pas de nouveau cas"
    );
    let c2 = cas_de(&b, e2).await;
    assert_eq!(c2.len(), 1, "cas ouvert par l'appel : pas de doublon");
    assert_eq!(
        (c2[0].0, c2[0].2, c2[0].3),
        (cas2, Some(g.id), Some(fin())),
        "rattaché à la sortie et à l'heure prévue"
    );
    // e1 : le CPE ferme le cas ; aux minutes suivantes, rien n'est rouvert (aucun bruit).
    let cas1 = cas_de(&b, e1).await;
    assert_eq!(cas1.len(), 1);
    b.ent
        .cas_issue_a(
            b.etab,
            b.cpe,
            CPE,
            cas1[0].0,
            &CasIssueSaisieDto {
                issue: "annule_erreur".into(),
                lieu_code: None,
                confirme_par_personne_id: None,
                numero_intervention: None,
                note: Some("ouvert à tort".into()),
                a: Some(apres_fin(18)),
            },
            apres_fin(18),
        )
        .await
        .unwrap();
    for mi in [19, 20, 21] {
        assert_eq!(
            b.ent
                .veille_non_rentres_a(b.etab, apres_fin(mi))
                .await
                .unwrap(),
            0,
            "minute {mi}"
        );
    }
    assert_eq!(cas_de(&b, e1).await.len(), 1);
    assert_eq!(
        b.evenements("internat.cas_ouvert").await.len(),
        3,
        "e1 (veille), e2 et e3 (appel) : jamais de bruit répété"
    );
    // Le retour de e2 ferme, grâce au rattachement, le cas que l'appel avait ouvert.
    b.ent
        .retour_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e2],
                ..Default::default()
            },
            apres_fin(30),
        )
        .await
        .unwrap();
    assert_eq!(cas_de(&b, e2).await[0].4.as_deref(), Some("retrouve"));
}

// ───────────────────────── M2 ─────────────────────────

/// Fait échouer toute insertion de cas pour `eleve` (déclencheur de test, retiré ensuite) : simule une ligne en erreur.
async fn piege_cas(b: &Banc, eleve: Uuid, pose: bool) {
    let p = b.ent.pool_verif();
    if pose {
        sqlx::query(
            "CREATE OR REPLACE FUNCTION ent_test_i4p2_echec() RETURNS trigger LANGUAGE plpgsql AS \
             $$ BEGIN RAISE EXCEPTION 'échec simulé'; END $$",
        )
        .execute(p)
        .await
        .unwrap();
        sqlx::query(&format!(
            "CREATE TRIGGER ent_test_i4p2_{} BEFORE INSERT ON ent_internat_cas FOR EACH ROW \
             WHEN (NEW.eleve_id = '{eleve}') EXECUTE FUNCTION ent_test_i4p2_echec()",
            eleve.simple()
        ))
        .execute(p)
        .await
        .unwrap();
    } else {
        sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS ent_test_i4p2_{} ON ent_internat_cas",
            eleve.simple()
        ))
        .execute(p)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn m2_une_ligne_en_erreur_n_empeche_pas_les_autres_de_la_veille() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let g = sortie_prete(&b, &[(e1, p1), (e2, p2)]).await;
    depart(&b, g.id, vec![e1, e2]).await;
    piege_cas(&b, e1, true).await;
    let r = b
        .ent
        .veille_non_rentres_bilan_a(b.etab, apres_fin(16))
        .await;
    piege_cas(&b, e1, false).await;
    assert_eq!(
        r.unwrap(),
        (1, 1),
        "un cas ouvert, une ligne en échec comptée"
    );
    assert!(cas_de(&b, e1).await.is_empty());
    assert_eq!(cas_de(&b, e2).await.len(), 1);
    // Le piège levé, la minute suivante reprend la ligne en échec.
    assert_eq!(
        b.ent
            .veille_non_rentres_bilan_a(b.etab, apres_fin(17))
            .await
            .unwrap(),
        (1, 0)
    );
    assert_eq!(cas_de(&b, e1).await.len(), 1);
}

#[tokio::test]
async fn m2_la_veille_de_l_etude_a_aussi_un_point_de_reprise_par_inscrit() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let t = creer_etude(&b, None).await;
    for e in [e1, e2] {
        b.ent
            .inscrire_temps_a(
                b.etab,
                b.cpe,
                CPE,
                t.id,
                &InscriptionTempsDto { eleve_id: e },
                mi_etude(1),
            )
            .await
            .unwrap();
    }
    let p = b.ent.pool_verif();
    sqlx::query("CREATE OR REPLACE FUNCTION ent_test_i4p2_echec_etude() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'échec simulé'; END $$")
        .execute(p)
        .await
        .unwrap();
    let nom = format!("ent_test_i4p2_etude_{}", e1.simple());
    sqlx::query(&format!(
        "CREATE TRIGGER {nom} BEFORE UPDATE OF signale_le ON ent_internat_temps_participant FOR EACH ROW \
         WHEN (NEW.eleve_id = '{e1}' AND NEW.signale_le IS NOT NULL) EXECUTE FUNCTION ent_test_i4p2_echec_etude()"
    ))
    .execute(p)
    .await
    .unwrap();
    let debut_etude = paris(avril(15), 19, 0);
    let r = b
        .ent
        .veille_etudes_bilan_a(b.etab, debut_etude + Duration::minutes(16))
        .await;
    sqlx::query(&format!(
        "DROP TRIGGER IF EXISTS {nom} ON ent_internat_temps_participant"
    ))
    .execute(p)
    .await
    .unwrap();
    assert_eq!(r.unwrap(), (1, 1));
    assert_eq!(
        b.ent
            .veille_etudes_bilan_a(b.etab, debut_etude + Duration::minutes(17))
            .await
            .unwrap(),
        (1, 0)
    );
}

// ───────────────────────── M3 ─────────────────────────

#[tokio::test]
async fn m3_un_inscrit_pointe_absent_est_signale_et_les_responsables_recoivent_un_avis() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let t = creer_etude(&b, None).await;
    for e in [e1, e2] {
        b.ent
            .inscrire_temps_a(
                b.etab,
                b.cpe,
                CPE,
                t.id,
                &InscriptionTempsDto { eleve_id: e },
                mi_etude(1),
            )
            .await
            .unwrap();
    }
    let debut_etude = paris(avril(15), 19, 0);
    pointer_temps(&b, t.id, e1, "absent", debut_etude + Duration::minutes(5))
        .await
        .unwrap();
    assert_eq!(
        b.ent
            .veille_etudes_a(b.etab, debut_etude + Duration::minutes(16))
            .await
            .unwrap(),
        2,
        "absent et non pointé"
    );
    assert_eq!(
        b.ent
            .veille_etudes_a(b.etab, debut_etude + Duration::minutes(17))
            .await
            .unwrap(),
        0,
        "une seule fois"
    );
    // Les événements sont distribués : le CPE et la direction reçoivent un avis, sans nom d'élève.
    distribuer(&b).await;
    let avis: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT destinataire_user_id, titre, corps FROM ent_notification WHERE etablissement_id = $1 AND kind = 'internat_etude_non_pointee'",
    )
    .bind(b.etab)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    let dest: std::collections::HashSet<Uuid> = avis.iter().map(|a| a.0).collect();
    assert!(dest.contains(&b.cpe) && dest.contains(&b.dir), "{avis:?}");
    assert!(
        !dest.contains(&b.prof),
        "un professeur sans droit de gestion ne reçoit rien"
    );
    let nom: String =
        sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(b.etab)
            .bind(e1)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap();
    assert!(!avis.is_empty());
    for (_, titre, corps) in &avis {
        assert!(
            !titre.contains(&nom) && !corps.contains(&nom) && !corps.contains(&e1.to_string()),
            "aucun nom ni identifiant d'élève dans l'avis : {titre} / {corps}"
        );
    }
}

#[tokio::test]
async fn m3_un_cas_de_nuit_ouvert_par_la_veille_previent_les_responsables() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(16))
            .await
            .unwrap(),
        1
    );
    distribuer(&b).await;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND kind = 'internat_cas_ouvert' AND destinataire_user_id = $2",
    )
    .bind(b.etab)
    .bind(b.cpe)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "le CPE de l'internat reçoit l'avis");
}

// ───────────────────────── M4 ─────────────────────────

#[tokio::test]
async fn m4_l_accord_annuel_soiree_ne_couvre_aucune_sortie_qui_recoupe_la_nuit() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    super::tests_internat_i3::accorder_annuellement(&b, p, e, &["sortie_soiree"]).await;
    let etat = |debut: DateTime<Utc>, fin: DateTime<Utc>| {
        let b = &b;
        async move {
            let g = b
                .ent
                .creer_sortie_groupe_a(b.etab, b.cpe, CPE, &sortie_de(debut, fin, b, e), t0())
                .await
                .unwrap();
            let g = b
                .ent
                .soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, h(1))
                .await
                .unwrap();
            etat_de(&g, e)
        }
    };
    // 00 h 30 → 02 h 00 : un seul jour local, mais en pleine nuit : la famille répond.
    assert_eq!(
        etat(local(avril(23), 0, 30), local(avril(23), 2, 0)).await,
        "en_attente"
    );
    // 05 h 30 → 07 h 00 : commence dans la nuit.
    assert_eq!(
        etat(local(avril(24), 5, 30), local(avril(24), 7, 0)).await,
        "en_attente"
    );
    // 21 h → 23 h : une soirée qui rentre avant minuit reste couverte par l'accord annuel.
    assert_eq!(
        etat(local(avril(24), 21, 0), local(avril(24), 23, 0)).await,
        "autorise"
    );
    // 18 h → 22 h (cas de référence).
    assert_eq!(
        etat(local(avril(25), 18, 0), local(avril(25), 22, 0)).await,
        "autorise"
    );
}

// ───────────────────────── M5 ─────────────────────────

#[tokio::test]
async fn m5_les_colonnes_qui_portent_la_cause_d_un_blocage_ne_sortent_pas_de_l_export() {
    let Some(b) = banc().await else { return };
    use crate::registre_donnees::classer;
    for (table, attendues) in [
        (
            "ent_internat_sortie_groupe_participant",
            vec!["motif_code", "reponse_user_id"],
        ),
        ("ent_internat_cas", vec!["nature", "issue_note"]),
    ] {
        let c = classer(table).unwrap();
        assert!(
            c.export_portabilite,
            "{table} reste exporté (sauf ces colonnes)"
        );
        for col in &attendues {
            assert!(
                c.colonnes_hors_export.contains(col),
                "{table} : {col} doit être hors export"
            );
        }
        // Chaque colonne déclarée existe réellement (une faute de frappe ne protégerait rien).
        for col in c.colonnes_hors_export {
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2)",
            )
            .bind(table)
            .bind(col)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap();
            assert!(existe, "{table}.{col} n'existe pas");
        }
    }
    // La nature « interdiction_auteur » et le motif « interdiction » sont bien portés par ces colonnes.
    let nature_cas: bool = sqlx::query_scalar(
        "SELECT pg_get_constraintdef(c.oid) LIKE '%interdiction_auteur%' FROM pg_constraint c \
          WHERE c.conrelid = 'ent_internat_cas'::regclass AND c.conname = 'ent_internat_cas_nature_check'",
    )
    .fetch_one(b.ent.pool_verif())
    .await
    .expect("la contrainte `ent_internat_cas_nature_check` existe");
    assert!(nature_cas);
}

// ───────────────────────── M6 ─────────────────────────

#[tokio::test]
async fn m6_la_tache_planifiee_lance_les_deux_nouvelles_veilles() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let g = sortie_prete(&b, &[(e1, p1)]).await;
    depart(&b, g.id, vec![e1]).await;
    let t = creer_etude(&b, None).await;
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            t.id,
            &InscriptionTempsDto { eleve_id: e2 },
            mi_etude(1),
        )
        .await
        .unwrap();
    // Un seul instant, fixe : 16 minutes après la fin de la sortie (veille des non-rentrés). L'étude du 15 avril est
    // vérifiée à son propre instant, avec la même tâche.
    // La tâche est lancée pour CET établissement seulement : elle n'agit pas sur ceux des tests voisins (base partagée).
    b.ent
        .internat_sorties_tache_etab_a(b.etab, &|| apres_fin(16))
        .await;
    assert_eq!(
        cas_de(&b, e1).await.len(),
        1,
        "la tâche lance la veille des élèves non rentrés"
    );
    b.ent
        .internat_sorties_tache_etab_a(b.etab, &|| paris(avril(15), 19, 16))
        .await;
    let signale: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT signale_le FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(b.etab)
    .bind(e2)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        signale,
        Some(paris(avril(15), 19, 16)),
        "la tâche lance la veille des études non pointées"
    );
}

#[tokio::test]
async fn m6_une_sortie_a_cheval_sur_minuit_ouvre_son_cas_apres_minuit_dans_la_bonne_nuit() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let (debut, fin) = (paris(avril(17), 22, 0), local(avril(18), 1, 0));
    let g = b
        .ent
        .creer_sortie_groupe_a(b.etab, b.cpe, CPE, &sortie_de(debut, fin, &b, e), t0())
        .await
        .unwrap();
    let g = b
        .ent
        .soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, h(1))
        .await
        .unwrap();
    repondre(&b, p, g.id, e, true).await.unwrap();
    accepter(&b, g.id).await;
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e],
                ..Default::default()
            },
            debut - Duration::minutes(10),
        )
        .await
        .unwrap();
    assert!(r.iter().all(|x| x.resultat == "ok"), "{r:?}");
    // Avant minuit, puis dans la tolérance : rien. Après fin + tolérance (1 h 16, le 18 avril) : un cas, dans la nuit du 17.
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, paris(avril(17), 23, 30))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin + Duration::minutes(14))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin + Duration::minutes(16))
            .await
            .unwrap(),
        1
    );
    let nuit: chrono::NaiveDate = sqlx::query_scalar(
        "SELECT nuit FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(b.etab)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(nuit, avril(17), "la nuit du 17 au 18 avril");
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin + Duration::minutes(17))
            .await
            .unwrap(),
        0
    );
}

// ───────────────────────── M7 ─────────────────────────

#[tokio::test]
async fn m7_un_eleve_protege_est_compte_sans_nom_dans_la_sortie_et_le_temps_du_soir() {
    let Some(b) = banc().await else { return };
    let (visible, protege) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, visible, true).await;
    let (p2, _) = parent(&b, protege, true).await;
    let g = sortie_prete(&b, &[(visible, p1), (protege, p2)]).await;
    depart(&b, g.id, vec![visible, protege]).await;
    let t = creer_etude(&b, None).await;
    for e in [visible, protege] {
        b.ent
            .inscrire_temps_a(
                b.etab,
                b.cpe,
                CPE,
                t.id,
                &InscriptionTempsDto { eleve_id: e },
                mi_etude(1),
            )
            .await
            .unwrap_or_else(|x| panic!("{x:?}"));
    }
    sqlx::query("INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du) VALUES ($1,$2,$3,'enc:v1:xx',ARRAY['adresse_protegee'],'2026-01-01')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(protege)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    // L'accompagnateur (sans droit sur la confidentialité) voit ses deux élèves : le protégé sans nom, avec son état.
    let d = b
        .ent
        .sortie_groupe_a(b.etab, b.prof, PROF, g.id, jour_j(30))
        .await
        .unwrap();
    assert_eq!(d.participants.len(), 2);
    let p = d
        .participants
        .iter()
        .find(|p| p.eleve_id == protege)
        .unwrap();
    assert!(p.protege && p.nom.is_none() && p.prenom.is_none());
    assert_eq!(p.etat, "parti");
    assert!(p.parti_le.is_some());
    let v = d
        .participants
        .iter()
        .find(|p| p.eleve_id == visible)
        .unwrap();
    assert!(v.nom.is_some() && !v.protege);
    // Les événements ne portent jamais son nom.
    let evts = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1",
    )
    .bind(b.etab)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    let nom: String =
        sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(b.etab)
            .bind(protege)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap();
    assert!(evts.iter().all(|e| !e.to_string().contains(&nom)));
    // Le temps du soir suit la même règle (vérifié en détail par `m8_*`) : le protégé est compté et marqué.
    let u = personnel_avec(&b, &[Action::InternatGerer]).await;
    let liste = b
        .ent
        .lister_temps_a(b.etab, u, PERSONNEL, avril(15), avril(15))
        .await
        .unwrap();
    let pt = liste[0]
        .pointages
        .iter()
        .find(|x| x.eleve_id == protege)
        .expect("protégé compté");
    assert!(pt.protege);
    assert_eq!(liste[0].inscrits.len(), 2);
}

// ───────────────────────── F-b ─────────────────────────

#[tokio::test]
async fn fb_un_eleve_declare_victime_et_present_ne_recoit_qu_un_signal_sante() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let mut s = evt("malaise");
    s.participants = vec![pe(e, "victime"), pe(e, "present")];
    s.zone_id = Some(b.zone_a);
    let r = b
        .ent
        .declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(2))
        .await
        .unwrap();
    assert_eq!(
        r.signaux_sante, 1,
        "un signal par élève, pas par couple (élève, rôle)"
    );
    assert_eq!(b.evenements("internat.malaise_signale").await.len(), 1);
}

// ───────────────────────── F-a, F-c, F-g ─────────────────────────

#[tokio::test]
async fn fa_un_accord_annuel_retire_tombe_avec_sa_cause_exacte_pas_une_interdiction() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    super::tests_internat_i3::accorder_annuellement(&b, p, e, &["sortie_soiree"]).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "autorise");
    accepter(&b, g.id).await;
    // Le titulaire retire son accord annuel : à l'appel de départ, l'accord tombe, sans qu'il y ait d'interdiction.
    b.ent
        .accords_decider(
            b.etab,
            p,
            crate::accords::DecisionSaisie {
                objet_code: "sortie_internat".into(),
                eleve_id: Some(e),
                usages: vec![crate::accords::ChoixUsage {
                    code: "sortie_soiree".into(),
                    accorde: false,
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        resultat(&depart_cause(&b, g.id, vec![e]).await, e),
        "accord_tombe"
    );
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT motif_code FROM ent_internat_sortie_groupe_participant WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
    )
    .bind(b.etab)
    .bind(g.id)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("accord_annuel_tombe"));
    let evts = b.evenements("internat.sortie_groupe_autorisation").await;
    assert!(
        evts.iter().all(|x| !x.to_string().contains("interdiction")),
        "{evts:?}"
    );
}

#[tokio::test]
async fn fc_inviter_apres_un_depart_est_un_conflit_lisible() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let g = sortie_prete(&b, &[(e1, p1)]).await;
    depart(&b, g.id, vec![e1]).await;
    let r = b
        .ent
        .inviter_eleves_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &ElevesAjoutDto { eleves: vec![e2] },
            debut() - Duration::minutes(5),
        )
        .await;
    assert!(refus_de(&r, "déjà partis"), "{r:?}");
}

#[tokio::test]
async fn fg_un_refus_hors_ligne_s_enregistre_meme_pour_un_parent_interdit() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (_, pers) = parent(&b, e, true).await;
    super::tests_internat_i3::interdire(&b, e, Some(pers)).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    let saisie = |accord: bool| AccordHorsLigneDto {
        donne_par_personne_id: pers,
        accord,
        recu_le: h(1),
        canal: "telephone".into(),
        piece_id: None,
    };
    // Un accord du parent interdit reste refusé ; son refus, plus restrictif, s'enregistre et est tracé.
    assert!(refus_de(
        &b.ent
            .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &saisie(true), h(2))
            .await,
        "ne peut pas être enregistré"
    ));
    b.ent
        .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &saisie(false), h(2))
        .await
        .unwrap();
    let (n, par): (i64, Uuid) = sqlx::query_as(
        "SELECT count(*), min(recu_par_user_id::text)::uuid FROM ent_internat_sortie_groupe_accord_hors_ligne \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 AND accord = false",
    )
    .bind(b.etab)
    .bind(g.id)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n, par), (1, b.cpe), "refus tracé avec son auteur");
    assert_eq!(
        etat_de(&detail(&b, g.id).await, e),
        "refuse",
        "le refus du parent interdit s'applique : l'élève ne part pas"
    );
}

// ───────────────────────── Fusion d'ENT : l'attendu porte les deux ajouts ─────────────────────────

#[tokio::test]
async fn m6_l_attendu_garde_la_branche_des_sorties_de_groupe_et_les_regles_du_voyage_de_654() {
    // La migration 654 (socle 42) et 2829 (socle 41, I4) redéfinissent toutes deux `ent_internat_attendu_calcule` : la
    // dernière (2829) doit porter les deux ajouts, sans quoi l'ordre des migrations en effacerait un.
    let Some(b) = banc().await else { return };
    let def: String = sqlx::query_scalar(
        "SELECT pg_get_functiondef('ent_internat_attendu_calcule(uuid, date)'::regprocedure)",
    )
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert!(
        def.contains("ent_internat_sortie_groupe_participant"),
        "branche des sorties de groupe (I4)"
    );
    assert!(
        def.contains("ent_sortie_retrait_effectif"),
        "retrait de voyage seulement après le départ constaté (654)"
    );
    // 656 (Faible 1) : la branche directe « séjour scolaire » est supprimée ; l'annulation du CPE (retrait annulé) est
    // respectée parce qu'un retrait annulé n'est plus lu par le CTE des retraits.
    assert!(
        !def.contains("ent_sortie_participant"),
        "plus de lecture directe des participants d'une sortie dans l'attendu (656)"
    );
    assert!(
        def.contains("r.annule_le IS NULL"),
        "un retrait annulé (par le CPE) ne retire plus l'élève"
    );
}
