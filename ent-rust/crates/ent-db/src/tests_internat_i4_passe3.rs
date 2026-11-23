//! Socle 41, lot I4 — **passe 3** (suites de la contre-revue de la passe 2, D-069) testée contre la base locale de test
//! (feature `labo`).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande reçoit son instant ; jours fixes d'avril 2026
//! (Paris = UTC + 2). Seule exception assumée : l'âge d'un événement (consommateur d'avis), réglé en base par rapport à
//! `now()` de la base, sans lire l'horloge du test.
//!
//! | Constat | Test(s) |
//! |---|---|
//! | Contrainte `motif_code` nommée (2830) | `s3_le_rejeu_de_2830_*` |
//! | Élève protégé inscrit pointé par l'encadrant | `s3_un_encadrant_*` |
//! | É1 : marque « en activité » sur un temps annulé | `s3_e1_*` |
//! | M1 : pas de réouverture d'une nuit à l'autre | `s3_m1_*` |
//! | F-g : refus reçu après le départ | `s3_fg_*` |
//! | Avis regroupés, borne d'âge | `s3_avis_*` |
//! | Marquage « protégé » identique | `s3_le_marquage_*` |

use super::tests_internat_i2::{banc, refus_de, Banc, CPE};
use super::tests_internat_i3::{accorder_annuellement, avril, parent, paris};
use super::tests_internat_i4::*;
use super::tests_internat_i4_correctifs::{cas_de, sortie_prete};
use super::tests_internat_i4_passe2::{
    activite_en_cours, apres_fin, distribuer, fin_activite, pointer_temps,
};
use chrono::{DateTime, Duration, Utc};
use ent_types::internat_nuit::{AppelOuvertureDto, CasIssueSaisieDto};
use ent_types::internat_vie::*;
use uuid::Uuid;

/// Rend l'élève « protégé » (mesure de confidentialité) : l'encadrant sans droit ne le voit plus nulle part ailleurs.
async fn proteger(b: &Banc, e: Uuid) {
    sqlx::query("INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du) VALUES ($1,$2,$3,'enc:v1:xx',ARRAY['adresse_protegee'],'2026-01-01')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(e)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
}

// ───────────────────────── 2830 : contrainte nommée ─────────────────────────

#[tokio::test]
async fn s3_le_rejeu_de_2830_remplace_l_ancien_check_et_l_accord_annuel_tombe_sans_23514() {
    let Some(b) = banc().await else { return };
    let p = b.ent.pool_verif();
    // Une base migrée par l'ancienne 2828 : CHECK en ligne (nom automatique), sans `accord_annuel_tombe`. NOT VALID : la base
    // partagée des tests porte déjà des lignes de cette valeur ; la contrainte s'applique à toute nouvelle écriture.
    for sql in [
        "ALTER TABLE ent_internat_sortie_groupe_participant DROP CONSTRAINT IF EXISTS ent_internat_sg_part_motif_code_chk",
        "ALTER TABLE ent_internat_sortie_groupe_participant ADD CONSTRAINT ent_internat_sortie_groupe_participant_motif_code_check \
         CHECK (motif_code IS NULL OR motif_code IN ('refus_famille', 'desaccord_connu', 'interdiction', 'plus_interne', \
         'retire_par_organisateur', 'majeur_refuse', 'majorite', 'chevauchement')) NOT VALID",
    ] {
        sqlx::query(sql).execute(p).await.unwrap();
    }
    crate::migrate::rejouer_tout(p).await.unwrap();
    let (anciens, nommes): (i64, String) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE conname = 'ent_internat_sortie_groupe_participant_motif_code_check'), \
                coalesce(max(pg_get_constraintdef(oid)) FILTER (WHERE conname = 'ent_internat_sg_part_motif_code_chk'), '') \
           FROM pg_constraint WHERE conrelid = 'ent_internat_sortie_groupe_participant'::regclass",
    )
    .fetch_one(p)
    .await
    .unwrap();
    assert_eq!(anciens, 0, "l'ancien CHECK en ligne est remplacé");
    assert!(
        nommes.contains("accord_annuel_tombe") && nommes.contains("chevauchement"),
        "{nommes}"
    );
    // Le rejeu est idempotent (une seconde fois : même résultat).
    crate::migrate::rejouer_tout(p).await.unwrap();
    // L'accord annuel « tombe » sans violer le CHECK (23514).
    let e = b.a[0];
    let (par, _) = parent(&b, e, true).await;
    accorder_annuellement(&b, par, e, &["sortie_soiree"]).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    accepter(&b, g.id).await;
    b.ent
        .accords_decider(
            b.etab,
            par,
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
}

// ───────────────────────── Élève protégé inscrit au temps du soir ─────────────────────────

#[tokio::test]
async fn s3_un_encadrant_sans_droit_de_confidentialite_pointe_un_protege_inscrit_et_le_non_rentre_se_ferme(
) {
    let Some(b) = banc().await else { return };
    let (prot, autre) = (b.a[0], b.a[1]);
    // Activité : le protégé est inscrit puis protégé ; l'encadrant (b.prof, sans droit propre) le pointe.
    let act = activite_en_cours(&b, &[prot]).await;
    // `activite_en_cours` a pointé « present » AVANT la protection ; on protège, puis on vérifie le retour.
    proteger(&b, prot).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin_activite() + Duration::minutes(16))
            .await
            .unwrap(),
        1,
        "le protégé en activité non rentré ouvre un cas"
    );
    pointer_temps(
        &b,
        act.id,
        prot,
        "rentre",
        fin_activite() + Duration::minutes(20),
    )
    .await
    .expect("l'encadrant pointe le retour d'un protégé inscrit");
    assert_eq!(cas_de(&b, prot).await[0].4.as_deref(), Some("retrouve"));
    // Un protégé inscrit à une étude : son absence se pointe aussi.
    let et = creer_etude(&b, None).await;
    let e2 = b.a[2];
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            et.id,
            &InscriptionTempsDto { eleve_id: e2 },
            mi_etude(1),
        )
        .await
        .unwrap();
    proteger(&b, e2).await;
    pointer_temps(&b, et.id, e2, "absent", paris(avril(15), 19, 5))
        .await
        .expect("l'encadrant pointe l'absence d'un protégé inscrit");
    // Envers un tiers NON inscrit, rien ne fuit : « n'est pas inscrit » (jamais « introuvable » ni autre détail).
    proteger(&b, autre).await;
    let r = pointer_temps(&b, act.id, autre, "present", paris(avril(15), 20, 45)).await;
    assert!(refus_de(&r, "n'est pas inscrit"), "{r:?}");
}

#[tokio::test]
async fn s3_le_marquage_protege_est_identique_dans_les_sorties_et_les_temps_du_soir() {
    let Some(b) = banc().await else { return };
    let (visible, cache) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, visible, true).await;
    let (p2, _) = parent(&b, cache, true).await;
    let g = sortie_prete(&b, &[(visible, p1), (cache, p2)]).await;
    depart(&b, g.id, vec![visible, cache]).await;
    proteger(&b, cache).await;
    let d = b
        .ent
        .sortie_groupe_a(b.etab, b.prof, PROF, g.id, jour_j(30))
        .await
        .unwrap();
    for p in &d.participants {
        assert_eq!(
            p.protege,
            p.eleve_id == cache,
            "marqué protégé ssi masqué : {p:?}"
        );
        assert_eq!(p.nom.is_none(), p.eleve_id == cache);
    }
}

// ───────────────────────── É1 : temps annulé et appel ─────────────────────────

fn ouverture_soir_15() -> AppelOuvertureDto {
    AppelOuvertureDto {
        nuit: Some(avril(15)),
        type_appel: "soir".into(),
        perimetre: "internat".into(),
        batiment_id: None,
        zone_id: None,
        liste_id: None,
        complementaire: false,
    }
}

#[tokio::test]
async fn s3_e1_un_appel_ouvert_apres_l_annulation_du_temps_marque_en_activite_l_eleve_sans_retour()
{
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let act = activite_en_cours(&b, &[e]).await;
    b.ent
        .annuler_temps_a(b.etab, b.cpe, CPE, act.id, paris(avril(15), 21, 0))
        .await
        .unwrap();
    let ouvert = b
        .ent
        .ouvrir_appel_a(
            b.etab,
            b.cpe,
            CPE,
            &ouverture_soir_15(),
            paris(avril(15), 21, 5),
        )
        .await
        .unwrap();
    let vue = b
        .ent
        .vue_appel_a(b.etab, b.cpe, CPE, ouvert.appel.id, paris(avril(15), 21, 6))
        .await
        .unwrap();
    let l = vue.lignes.iter().find(|l| l.eleve_id == e).unwrap();
    assert_eq!(
        l.marque.as_deref(),
        Some("activite"),
        "l'élève parti d'un temps annulé est marqué en activité, jamais absent"
    );
    assert_eq!(l.retour_attendu, Some(fin_activite()));
}

// ───────────────────────── M1 : pas de réouverture d'une nuit à l'autre ─────────────────────────

#[tokio::test]
async fn s3_m1_un_cas_clos_n_est_pas_rouvert_la_nuit_suivante() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    let nuit1 = nuit_de_veille(&b, apres_fin(16)).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(16))
            .await
            .unwrap(),
        1
    );
    let cas = cas_de(&b, e).await;
    b.ent
        .cas_issue_a(
            b.etab,
            b.cpe,
            CPE,
            cas[0].0,
            &CasIssueSaisieDto {
                issue: "annule_erreur".into(),
                lieu_code: None,
                confirme_par_personne_id: None,
                numero_intervention: None,
                note: Some("ouvert à tort".into()),
                a: Some(apres_fin(20)),
            },
            apres_fin(20),
        )
        .await
        .unwrap();
    // La nuit change à midi : le lendemain après-midi (nuit suivante), l'élève n'est toujours pas pointé rentré.
    let lendemain = paris(avril(17), 13, 0);
    assert_ne!(
        nuit_de_veille(&b, lendemain).await,
        nuit1,
        "c'est bien une autre nuit"
    );
    for mi in [0, 1, 60] {
        assert_eq!(
            b.ent
                .veille_non_rentres_a(b.etab, lendemain + Duration::minutes(mi))
                .await
                .unwrap(),
            0,
            "aucune réouverture, ni la nuit suivante ni plus tard"
        );
    }
    assert_eq!(cas_de(&b, e).await.len(), 1);
}

async fn nuit_de_veille(b: &Banc, a: DateTime<Utc>) -> chrono::NaiveDate {
    super::tests_internat_i4_passe2::nuit_de(b, a).await
}

// ───────────────────────── F-g : refus reçu après le départ ─────────────────────────

#[tokio::test]
async fn s3_fg_un_refus_apres_le_depart_est_un_fait_trace_et_demande_le_retour() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, pers) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "parti");
    let saisie = |accord: bool| AccordHorsLigneDto {
        donne_par_personne_id: pers,
        accord,
        recu_le: h(1),
        canal: "telephone".into(),
        piece_id: None,
    };
    // Un ACCORD reçu après le départ reste refusé.
    let r = b
        .ent
        .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &saisie(true), jour_j(40))
        .await;
    assert!(refus_de(&r, "n'attend plus") || r.is_err(), "{r:?}");
    // Le REFUS s'enregistre : fait tracé, l'état reste « parti » (le départ n'est pas annulé après coup), retour demandé.
    b.ent
        .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &saisie(false), jour_j(40))
        .await
        .unwrap();
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "parti");
    let (faits, par): (i64, Option<Uuid>) = sqlx::query_as(
        "SELECT count(*), min(par_user_id::text)::uuid FROM ent_internat_sortie_groupe_fait \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 AND acte = 'refus_apres_depart'",
    )
    .bind(b.etab)
    .bind(g.id)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((faits, par), (1, Some(b.cpe)), "fait tracé avec son auteur");
    let traces: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_internat_sortie_groupe_accord_hors_ligne WHERE etablissement_id = $1 \
          AND sortie_groupe_id = $2 AND eleve_id = $3 AND accord = false AND canal = 'telephone'",
    )
    .bind(b.etab)
    .bind(g.id)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        traces, 1,
        "le refus est daté, nommé (auteur), avec son canal"
    );
    let evts = b.evenements("internat.retour_demande").await;
    assert_eq!(evts.len(), 1, "l'avis de retour est émis : {evts:?}");
    assert!(
        !evts[0].to_string().contains(&pers.to_string()),
        "ni l'identité du parent ni un nom dans la charge"
    );
    // L'avis atteint les responsables, sans nom d'élève.
    distribuer(&b).await;
    let avis: Vec<String> = sqlx::query_scalar(
        "SELECT corps FROM ent_notification WHERE etablissement_id = $1 AND kind = 'internat_retour_demande' AND destinataire_user_id = $2",
    )
    .bind(b.etab)
    .bind(b.cpe)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(avis.len(), 1, "{avis:?}");
    // Le retour de l'élève reste possible et normal.
    b.ent
        .retour_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e],
                ..Default::default()
            },
            apres_fin(30),
        )
        .await
        .unwrap();
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "rentre");
}

// ───────────────────────── Avis regroupés, borne d'âge ─────────────────────────

#[tokio::test]
async fn s3_avis_une_rafale_de_cas_donne_un_seul_avis_avec_le_nombre_et_un_vieil_evenement_n_en_donne_aucun(
) {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let (p3, _) = parent(&b, e3, true).await;
    let g = sortie_prete(&b, &[(e1, p1), (e2, p2), (e3, p3)]).await;
    depart(&b, g.id, vec![e1, e2, e3]).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(16))
            .await
            .unwrap(),
        3
    );
    // Les trois événements du même établissement tombent dans la même minute (instant posé par la base : aucune lecture
    // de l'horloge du test), l'un d'eux est « ancien » : il n'en sort aucun avis.
    sqlx::query(
        "UPDATE ent_metier_event SET created_at = date_trunc('minute', now()) + interval '10 seconds' \
          WHERE etablissement_id = $1 AND type = 'internat.cas_ouvert'",
    )
    .bind(b.etab)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    distribuer(&b).await;
    let avis: Vec<(String, String)> = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE etablissement_id = $1 AND kind = 'internat_cas_ouvert' AND destinataire_user_id = $2",
    )
    .bind(b.etab)
    .bind(b.cpe)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(avis.len(), 1, "un seul avis pour la rafale : {avis:?}");
    assert!(avis[0].1.contains("3 élèves"), "{avis:?}");
    for e in [e1, e2, e3] {
        assert!(!avis[0].1.contains(&e.to_string()));
    }
    // Un événement vieux de deux jours (historique en attente au premier passage) ne produit aucun avis.
    let e4 = b.a[3];
    let (p4, _) = parent(&b, e4, true).await;
    let g2 = sortie_prete(&b, &[(e4, p4)]).await;
    depart(&b, g2.id, vec![e4]).await;
    b.ent
        .veille_non_rentres_a(b.etab, apres_fin(40))
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_metier_event SET created_at = now() - interval '2 days' \
          WHERE etablissement_id = $1 AND type = 'internat.cas_ouvert' AND eleve_id = $2",
    )
    .bind(b.etab)
    .bind(e4)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    distribuer(&b).await;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND kind = 'internat_cas_ouvert' AND destinataire_user_id = $2",
    )
    .bind(b.etab)
    .bind(b.cpe)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "l'événement ancien ne déclenche aucun nouvel avis");
}
