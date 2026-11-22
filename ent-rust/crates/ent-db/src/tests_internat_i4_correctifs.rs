//! Socle 41, lot I4 — **correctifs de revue** (règles générales 1 à 6, D-069) testés contre la base locale de test (feature
//! `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés en local (échec explicite en CI).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande reçoit son instant ; les jours sont fixes (avril
//! 2026, Paris = UTC + 2 ; nuit du changement d'heure : 28 → 29 mars 2026). Chaque test crée son propre établissement.
//!
//! | Constat | Test(s) |
//! |---|---|
//! | E1 (élève parti qui ne rentre pas ; en activité à l'appel) | `e1_*` |
//! | E2 (l'accord annuel ne vaut que pour une soirée ; plusieurs jours : attendu seulement après départ) | `e2_*` |
//! | E3, M10 (la cause est une donnée de protection ; exports) | `e3_*`, `m10_*` |
//! | M1, F10 (tout fait se corrige par un fait) | `m1_*` |
//! | M2 (une seule fonction de titulaire) | `m2_*` |
//! | M3, M4, F1, F12 (événements) | `m3_*`, `m4_*`, `f1_*`, `f12_*` |
//! | M5 (fusion de fiches) | `m5_*` |
//! | M6, M7, M8, M9 (accompagnateurs, décision, protégés, étude) | `m6_*` … `m9_*` |
//! | F2 à F9, F11 | `f2_*` … `f11_*` |
//! | Concurrence | `concurrence_*` |

use super::tests_internat_i2::{
    banc, classes_des_deux_zones, est_interdit, refus_de, Banc, CPE, DIR,
};
use super::tests_internat_i3::{
    avril, desaccord_connu, interdire, interdire_objet, parent, paris, tiers, PARENT,
};
use super::tests_internat_i4::*;
use super::*;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_types::internat_vie::*;
use uuid::Uuid;

/// Heure locale (Paris, UTC + 2 en avril) sans dépasser minuit vers l'arrière : `paris` retranche deux heures.
fn local(jour: NaiveDate, h: u32, mi: u32) -> DateTime<Utc> {
    paris(jour, h + 2, mi) - Duration::hours(2)
}

/// Participe, répond et fait accepter une sortie pour un élève (un parent déjà créé).
async fn sortie_prete(b: &Banc, eleves: &[(Uuid, Uuid)]) -> SortieGroupeDto {
    let g = soumettre(
        b,
        brouillon(b, vec![b.prof], eleves.iter().map(|e| e.0).collect())
            .await
            .id,
    )
    .await;
    for (e, p) in eleves {
        repondre(b, *p, g.id, *e, true).await.unwrap();
    }
    accepter(b, g.id).await;
    g
}

async fn cas_de(
    b: &Banc,
    eleve: Uuid,
) -> Vec<(
    Uuid,
    String,
    Option<Uuid>,
    Option<DateTime<Utc>>,
    Option<String>,
)> {
    sqlx::query_as(
        "SELECT id, nature, sortie_groupe_id, heure_prevue, issue FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY ouvert_le, id",
    )
    .bind(b.etab)
    .bind(eleve)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap()
}

fn ouverture_soir() -> ent_types::internat_nuit::AppelOuvertureDto {
    ent_types::internat_nuit::AppelOuvertureDto {
        nuit: Some(avril(16)),
        type_appel: "soir".into(),
        perimetre: "internat".into(),
        batiment_id: None,
        zone_id: None,
        liste_id: None,
        complementaire: false,
    }
}

// ───────────────────────── E1 : un élève parti ne disparaît jamais ─────────────────────────

#[tokio::test]
async fn e1_parti_non_rentre_ouvre_un_cas_par_la_veille_et_le_retour_le_ferme() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    // Tolérance de l'établissement : 15 minutes par défaut (D-069) après l'heure de fin prévue.
    let reglages = b.ent.lire_reglages_vie_a(b.etab, b.cpe, CPE).await.unwrap();
    assert_eq!(reglages.groupe_tolerance_retour_minutes, 15);
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin() + Duration::minutes(14))
            .await
            .unwrap(),
        0,
        "dans la tolérance"
    );
    assert!(cas_de(&b, e).await.is_empty());
    let apres = fin() + Duration::minutes(16);
    assert_eq!(b.ent.veille_non_rentres_a(b.etab, apres).await.unwrap(), 1);
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres + Duration::minutes(1))
            .await
            .unwrap(),
        0,
        "idempotent : un seul cas"
    );
    let cas = cas_de(&b, e).await;
    assert_eq!(cas.len(), 1);
    assert_eq!(
        (cas[0].1.as_str(), cas[0].2, cas[0].3),
        ("non_rentre", Some(g.id), Some(fin()))
    );
    assert!(cas[0].4.is_none(), "cas ouvert");
    // La vue du cas dit qui joindre et l'heure prévue (procédure d'I3).
    let vue = b
        .ent
        .cas_vue_a(b.etab, b.cpe, CPE, cas[0].0, apres)
        .await
        .unwrap();
    let origine = vue.origine.expect("origine du cas");
    assert_eq!(origine.a_joindre, vec![b.prof]);
    assert_eq!(
        (origine.heure_prevue, origine.sortie_groupe_id),
        (fin(), Some(g.id))
    );
    // Le retour pointé ferme le cas (issue « retrouvé »).
    let r = b
        .ent
        .retour_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e],
                ..Default::default()
            },
            apres + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(resultat(&r, e), "ok");
    assert_eq!(cas_de(&b, e).await[0].4.as_deref(), Some("retrouve"));
    // Les réglages se modifient dans leurs bornes, par la gestion seulement.
    let r = ReglagesVieSaisieDto {
        groupe_tolerance_retour_minutes: Some(30),
        ..Default::default()
    };
    assert_eq!(
        b.ent
            .modifier_reglages_vie_a(b.etab, b.cpe, CPE, &r, t0())
            .await
            .unwrap()
            .groupe_tolerance_retour_minutes,
        30
    );
    let r = ReglagesVieSaisieDto {
        groupe_tolerance_retour_minutes: Some(241),
        ..Default::default()
    };
    assert!(refus_de(
        &b.ent
            .modifier_reglages_vie_a(b.etab, b.cpe, CPE, &r, t0())
            .await,
        "tolérance"
    ));
    assert!(est_interdit(
        &b.ent
            .modifier_reglages_vie_a(b.etab, b.prof, PROF, &ReglagesVieSaisieDto::default(), t0())
            .await
    ));
}

#[tokio::test]
async fn e1_sortie_qui_chevauche_l_appel_marque_en_activite_jamais_absent() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let g = sortie_prete(&b, &[(e1, p1), (e2, p2)]).await;
    // e2 part AVANT l'ouverture de l'appel du soir (marque posée à l'ouverture) ; e1 APRÈS (marque posée au départ).
    depart(&b, g.id, vec![e2]).await;
    let ouvert = b
        .ent
        .ouvrir_appel_a(b.etab, b.cpe, CPE, &ouverture_soir(), jour_j(12))
        .await
        .unwrap();
    depart_a(&b, g.id, vec![e1], 14).await;
    let vue = b
        .ent
        .vue_appel_a(b.etab, b.cpe, CPE, ouvert.appel.id, jour_j(15))
        .await
        .unwrap();
    for e in [e1, e2] {
        let l = vue.lignes.iter().find(|l| l.eleve_id == e).unwrap();
        assert_eq!(
            l.marque.as_deref(),
            Some("activite"),
            "en activité, pas absent"
        );
        assert_eq!(l.retour_attendu, Some(fin()));
    }
    assert!(vue
        .lignes
        .iter()
        .all(|l| l.marque.as_deref() != Some("absent")));
}

/// Départ à `mi` minutes après l'ouverture de la plage (accompagnateur pointé présent).
async fn depart_a(b: &Banc, id: Uuid, eleves: Vec<Uuid>, mi: i64) {
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.prof,
            PROF,
            id,
            &AppelGroupeDto {
                eleves: eleves.clone(),
                ..Default::default()
            },
            jour_j(mi),
        )
        .await
        .unwrap();
    assert!(r.iter().all(|x| x.resultat == "ok"), "{r:?}");
}

// ───────────────────────── E2 : un accord ne vaut que pour ce qu'il couvre ─────────────────────────

fn sortie_de(debut: DateTime<Utc>, fin: DateTime<Utc>, b: &Banc, e: Uuid) -> SortieGroupeSaisieDto {
    SortieGroupeSaisieDto {
        titre: "Sortie".into(),
        lieu: "Ville".into(),
        debut,
        fin,
        encadrants: vec![b.prof],
        eleves: vec![e],
    }
}

#[tokio::test]
async fn e2_l_accord_annuel_ne_vaut_que_pour_une_soiree() {
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
    // Soirée : même jour, 6 h au plus, aucune nuit couverte → l'accord annuel vaut.
    assert_eq!(
        etat(paris(avril(16), 18, 0), paris(avril(16), 22, 0)).await,
        "autorise"
    );
    // 48 h : un accord dédié est exigé → la famille répond.
    assert_eq!(
        etat(paris(avril(17), 14, 0), paris(avril(19), 14, 0)).await,
        "en_attente"
    );
    // Plus longue que la durée maximale du type « soirée » (6 h), même jour.
    assert_eq!(
        etat(paris(avril(20), 8, 0), paris(avril(20), 18, 0)).await,
        "en_attente"
    );
    // Couvre minuit (pas le même jour) : la famille répond.
    assert_eq!(
        etat(paris(avril(21), 20, 0), local(avril(22), 1, 0)).await,
        "en_attente"
    );
}

#[tokio::test]
async fn e2_sortie_de_plusieurs_jours_retire_de_l_attendu_seulement_apres_depart_constate() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let deb = paris(avril(16), 14, 0);
    let mut s = sortie_de(deb, paris(avril(18), 14, 0), &b, e1);
    s.eleves = vec![e1, e2];
    let g = b
        .ent
        .creer_sortie_groupe_a(b.etab, b.cpe, CPE, &s, t0())
        .await
        .unwrap();
    let g = b
        .ent
        .soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, h(1))
        .await
        .unwrap();
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    repondre(&b, p2, g.id, e2, true).await.unwrap();
    accepter(&b, g.id).await;
    let statut = |e: Uuid| {
        let b = &b;
        async move {
            sqlx::query_as::<_, (String, Option<String>)>("SELECT statut, motif_code FROM ent_internat_attendu_calcule($1, $2) WHERE eleve_id = $3")
                .bind(b.etab)
                .bind(avril(16))
                .bind(e)
                .fetch_one(b.ent.pool_verif())
                .await
                .unwrap()
        }
    };
    // Avant le départ : attendus (un autorisé qui n'est pas parti n'est pas retiré).
    assert_eq!(statut(e1).await.0, "attendu");
    // e1 part, e2 reste : seul le départ CONSTATÉ retire.
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e1],
                ..Default::default()
            },
            deb - Duration::minutes(10),
        )
        .await
        .unwrap();
    assert_eq!(resultat(&r, e1), "ok");
    assert_eq!(
        statut(e1).await,
        ("retire".to_string(), Some("sortie".to_string()))
    );
    assert_eq!(statut(e2).await.0, "attendu");
}

// ───────────────────────── E3 : la cause d'un blocage est une donnée de protection ─────────────────────────

#[tokio::test]
async fn e3_un_accompagnateur_sans_droit_ne_voit_aucune_cause_dans_le_json() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let (p1, pers1) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let (p3, _) = parent(&b, e3, true).await;
    desaccord_connu(&b, e2, p2).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2, e3]).await.id).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    repondre(&b, p3, g.id, e3, true).await.unwrap();
    accepter(&b, g.id).await;
    interdire(&b, e1, Some(pers1)).await;
    // L'accompagnateur (professeur, sans droit d'internat) fait le départ : « non autorisé », jamais la cause.
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e1, e2, e3],
                ..Default::default()
            },
            jour_j(10),
        )
        .await
        .unwrap();
    assert_eq!(
        (resultat(&r, e1), resultat(&r, e2), resultat(&r, e3)),
        ("non_autorise".into(), "non_autorise".into(), "ok".into())
    );
    let vue = b
        .ent
        .sortie_groupe_a(b.etab, b.prof, PROF, g.id, jour_j(11))
        .await
        .unwrap();
    let json = serde_json::to_string(&vue).unwrap();
    for interdit in ["interdiction", "desaccord_connu", "accord_tombe"] {
        assert!(
            !json.contains(interdit),
            "{interdit} dans la vue de l'accompagnateur : {json}"
        );
    }
    assert!(vue.participants.iter().all(|p| p.motif.is_none()));
    assert_eq!(etat_de(&vue, e1), "non_autorise");
    // Le CPE (internat.lire / gérer) lit la cause.
    let cpe = detail(&b, g.id).await;
    let motifs: Vec<Option<String>> = [e1, e2]
        .iter()
        .map(|e| {
            cpe.participants
                .iter()
                .find(|p| p.eleve_id == *e)
                .unwrap()
                .motif
                .clone()
        })
        .collect();
    assert_eq!(
        motifs,
        vec![Some("interdiction".into()), Some("desaccord_connu".into())]
    );
}

// ───────────────────────── M1 / F10 : tout fait se corrige par un fait ─────────────────────────

#[tokio::test]
async fn m1_un_retour_pointe_par_erreur_s_annule_par_un_fait_motive_et_un_refus_telephone_se_saisit(
) {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (_p2, pers2) = parent(&b, e2, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    // Refus reçu par téléphone : il se saisit comme un accord hors ligne (auteur, date, canal) et refuse l'élève.
    let refus_tel = AccordHorsLigneDto {
        donne_par_personne_id: pers2,
        accord: false,
        recu_le: h(1),
        canal: "telephone".into(),
        piece_id: None,
    };
    let d = b
        .ent
        .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e2, &refus_tel, h(2))
        .await
        .unwrap();
    assert_eq!(etat_de(&d, e2), "refuse");
    let actes: Vec<String> = sqlx::query_scalar("SELECT acte FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 ORDER BY cree_le")
        .bind(b.etab).bind(g.id).bind(e2).fetch_all(b.ent.pool_verif()).await.unwrap();
    assert!(actes.contains(&"refus_hors_ligne".to_string()), "{actes:?}");
    // Un refus est définitif : un accord hors ligne ne le remplace pas.
    let accord_tel = AccordHorsLigneDto {
        accord: true,
        ..refus_tel.clone()
    };
    assert!(refus_de(
        &b.ent
            .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e2, &accord_tel, h(2))
            .await,
        "n'attend pas"
    ));
    accepter(&b, g.id).await;
    depart(&b, g.id, vec![e1]).await;
    let retour = |mi: i64| {
        let b = &b;
        async move {
            b.ent
                .retour_groupe_a(
                    b.etab,
                    b.prof,
                    PROF,
                    g.id,
                    &AppelGroupeDto {
                        eleves: vec![e1],
                        ..Default::default()
                    },
                    jour_j(mi),
                )
                .await
                .unwrap()
        }
    };
    assert_eq!(resultat(&retour(60).await, e1), "ok");
    // Retour pointé par erreur : un fait motivé remet l'élève « parti » ; sans motif codé, refusé.
    let annule = |motif: &str| {
        let b = &b;
        let motif = motif.to_string();
        async move {
            b.ent
                .annuler_retour_groupe_a(
                    b.etab,
                    b.prof,
                    PROF,
                    g.id,
                    &AnnulationRetourDto {
                        eleve_id: e1,
                        motif_code: motif,
                    },
                    jour_j(61),
                )
                .await
        }
    };
    assert!(refus_de(&annule("parce que").await, "motif"));
    annule("erreur_pointage").await.unwrap();
    assert_eq!(etat_de(&detail(&b, g.id).await, e1), "parti");
    let clore = b
        .ent
        .clore_sortie_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &ClotureGroupeDto {
                bilan_code: "sans_incident".into(),
            },
            jour_j(62),
        )
        .await;
    assert!(
        refus_de(&clore, "pas rentrés"),
        "le départ rétabli bloque la clôture"
    );
    assert!(refus_de(
        &annule("erreur_pointage").await,
        "pas pointé rentré"
    ));
    assert_eq!(resultat(&retour(63).await, e1), "ok");
    let actes: Vec<String> = sqlx::query_scalar("SELECT acte FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 ORDER BY cree_le")
        .bind(b.etab).bind(g.id).bind(e1).fetch_all(b.ent.pool_verif()).await.unwrap();
    assert_eq!(actes.iter().filter(|a| *a == "retour").count(), 2);
    assert!(actes.contains(&"retour_annule".to_string()));
}

// ───────────────────────── M2 : une seule fonction pour la règle de titulaire ─────────────────────────

#[tokio::test]
async fn m2_l_accord_hors_ligne_applique_la_regle_m_a_et_la_chaine_de_fusions() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    // Interdiction SANS personne identifiée, posée AVANT la création du lien (règle M-A, D25) : le lien créé ensuite ni
    // n'agit ni ne répond, y compris hors ligne (même fonction que la réponse en ligne).
    interdire(&b, e, None).await;
    let (_, nouvelle) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    let s = AccordHorsLigneDto {
        donne_par_personne_id: nouvelle,
        accord: true,
        recu_le: h(1),
        canal: "papier".into(),
        piece_id: None,
    };
    assert!(refus_de(
        &b.ent
            .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &s, h(2))
            .await,
        "ne peut pas être enregistré"
    ));
    // Chaîne de fusions : une interdiction posée sur une fiche fusionnée dans la personne vise la même personne réelle.
    let b2 = banc().await.unwrap();
    let e2 = b2.a[0];
    let (_, pers) = parent(&b2, e2, true).await;
    let doublon = tiers(&b2, "Doublon").await;
    sqlx::query(
        "UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(b2.etab)
    .bind(doublon)
    .bind(pers)
    .execute(b2.ent.pool_verif())
    .await
    .unwrap();
    interdire(&b2, e2, Some(doublon)).await;
    let g2 = soumettre(&b2, brouillon(&b2, vec![b2.prof], vec![e2]).await.id).await;
    let s2 = AccordHorsLigneDto {
        donne_par_personne_id: pers,
        accord: true,
        recu_le: h(1),
        canal: "papier".into(),
        piece_id: None,
    };
    assert!(refus_de(
        &b2.ent
            .accord_hors_ligne_groupe_a(b2.etab, b2.cpe, CPE, g2.id, e2, &s2, h(2))
            .await,
        "ne peut pas être enregistré"
    ));
    // Un accord hors ligne valide, puis une interdiction sur la fiche fusionnée : il tombe au départ.
    let b3 = banc().await.unwrap();
    let e3 = b3.a[0];
    let (_, pers3) = parent(&b3, e3, true).await;
    let doublon3 = tiers(&b3, "Doublon").await;
    sqlx::query(
        "UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(b3.etab)
    .bind(doublon3)
    .bind(pers3)
    .execute(b3.ent.pool_verif())
    .await
    .unwrap();
    let g3 = soumettre(&b3, brouillon(&b3, vec![b3.prof], vec![e3]).await.id).await;
    let s3 = AccordHorsLigneDto {
        donne_par_personne_id: pers3,
        accord: true,
        recu_le: h(1),
        canal: "papier".into(),
        piece_id: None,
    };
    b3.ent
        .accord_hors_ligne_groupe_a(b3.etab, b3.cpe, CPE, g3.id, e3, &s3, h(2))
        .await
        .unwrap();
    accepter(&b3, g3.id).await;
    interdire(&b3, e3, Some(doublon3)).await;
    assert_eq!(
        resultat(&depart_cause(&b3, g3.id, vec![e3]).await, e3),
        "accord_tombe"
    );
}

#[tokio::test]
async fn m2_interdictions_visite_et_contact_eleve_font_tomber_l_accord() {
    let Some(b) = banc().await else { return };
    for objet in ["visite", "contact_eleve"] {
        let e = b.a[if objet == "visite" { 0 } else { 1 }];
        let (p, pers) = parent(&b, e, true).await;
        let g = sortie_prete(&b, &[(e, p)]).await;
        interdire_objet(&b, e, pers, objet, None).await;
        assert_eq!(
            resultat(&depart_cause(&b, g.id, vec![e]).await, e),
            "accord_tombe",
            "{objet}"
        );
        // Et la famille visée ne répond plus (acteur écarté).
        assert!(
            est_interdit(&repondre(&b, p, g.id, e, true).await),
            "{objet}"
        );
    }
}

// ───────────────────────── F2, F3 : gardes en base ─────────────────────────

#[tokio::test]
async fn f2_transitions_de_participant_gardees_en_base() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let (_p3, _) = parent(&b, e3, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2, e3]).await.id).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    repondre(&b, p2, g.id, e2, false).await.unwrap();
    let pool = b.ent.pool_verif();
    let maj = |sql: &str, e: Uuid| {
        let q = format!("UPDATE ent_internat_sortie_groupe_participant SET {sql} WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3");
        let (etab, sortie) = (b.etab, g.id);
        async move {
            sqlx::query(&q)
                .bind(etab)
                .bind(sortie)
                .bind(e)
                .execute(pool)
                .await
        }
    };
    // `parti` seulement depuis `autorise` : un élève en attente (e3) ou refusé (e2) ne part pas, même par SQL.
    let parti = "etat = 'parti', parti_le = now(), parti_par_user_id = (SELECT id FROM ent_user LIMIT 1), reponse_qualite = 'titulaire', reponse_le = now(), reponse_user_id = (SELECT id FROM ent_user LIMIT 1)";
    assert!(maj(parti, e3).await.is_err(), "en_attente → parti");
    assert!(maj(parti, e2).await.is_err(), "refuse → parti");
    // `refuse` est terminal : pas de retour à `autorise` ni à `en_attente`.
    assert!(maj("etat = 'autorise'", e2).await.is_err());
    assert!(maj("etat = 'en_attente'", e2).await.is_err());
    // Une réponse ne s'écrit pas sur un élève qui n'attend plus (autorise → en_attente).
    assert!(maj("etat = 'en_attente'", e1).await.is_err());
    accepter(&b, g.id).await;
    depart(&b, g.id, vec![e1]).await;
    // Pas d'annulation d'une sortie avec des partis, même par SQL.
    let r = sqlx::query("UPDATE ent_internat_sortie_groupe SET statut = 'annulee', annule_motif_code = 'meteo' WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab).bind(g.id).execute(pool).await;
    assert!(r.is_err(), "annulation d'une sortie avec des partis");
    // Un blocage est terminal.
    let (e4, p4) = (b.a[3], parent(&b, b.a[3], true).await.0);
    let g2 = soumettre(&b, brouillon(&b, vec![b.prof], vec![e4]).await.id).await;
    desaccord_connu(&b, e4, p4).await;
    sqlx::query("UPDATE ent_internat_sortie_groupe_participant SET etat = 'non_autorise', motif_code = 'desaccord_connu' WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3")
        .bind(b.etab).bind(g2.id).bind(e4).execute(pool).await.unwrap();
    let r = sqlx::query("UPDATE ent_internat_sortie_groupe_participant SET etat = 'autorise' WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3")
        .bind(b.etab).bind(g2.id).bind(e4).execute(pool).await;
    assert!(r.is_err(), "non_autorise → autorise");
}

#[tokio::test]
async fn f3_cles_etrangeres_composites_et_piece_validee() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (_, pers) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    // Pièce inconnue : refusée par le service.
    let s = AccordHorsLigneDto {
        donne_par_personne_id: pers,
        accord: true,
        recu_le: h(1),
        canal: "papier".into(),
        piece_id: Some(Uuid::new_v4()),
    };
    assert!(refus_de(
        &b.ent
            .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &s, h(2))
            .await,
        "pièce"
    ));
    // Une personne d'un AUTRE établissement ne se référence pas (clé composite), ni par SQL.
    let autre = banc().await.unwrap();
    let (_, pers_autre) = parent(&autre, autre.a[0], true).await;
    let r = sqlx::query("INSERT INTO ent_internat_sortie_groupe_accord_hors_ligne (id, etablissement_id, sortie_groupe_id, eleve_id, donne_par_personne_id, recu_le, canal, recu_par_user_id) VALUES ($1,$2,$3,$4,$5,now(),'papier',$6)")
        .bind(Uuid::new_v4()).bind(b.etab).bind(g.id).bind(e).bind(pers_autre).bind(b.cpe).execute(b.ent.pool_verif()).await;
    assert!(r.is_err(), "personne d'un autre établissement");
    // `liaison.temps_id` : une séance d'un autre établissement ne se référence pas.
    b.ent
        .lister_codes_liaison_a(b.etab, b.cpe, CPE)
        .await
        .unwrap(); // le catalogue existe (clé étrangère du code)
    let t_autre = creer_etude(&autre, None).await;
    let r = sqlx::query("INSERT INTO ent_internat_liaison (id, etablissement_id, eleve_id, code, nuit, par_user_id, pour_le_jour, temps_id) SELECT $1,$2,$3,'absence_etude','2026-04-15',$4,true,$5 WHERE true")
        .bind(Uuid::new_v4()).bind(b.etab).bind(e).bind(b.cpe).bind(t_autre.id).execute(b.ent.pool_verif()).await;
    assert!(r.is_err(), "séance d'un autre établissement");
}

// ───────────────────────── Événements : M3, M4, F1, F12 ─────────────────────────

#[tokio::test]
async fn m3_un_malaise_exige_un_eleve_non_temoin_et_le_signal_est_accuse() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    // Sans élève, ou seulement des témoins : refusé (422), aucun signal ne part.
    let mut s = evt("malaise");
    s.zone_id = Some(b.zone_a);
    assert!(refus_de(
        &b.ent
            .declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1))
            .await,
        "au moins un élève"
    ));
    s.participants = vec![pe(e1, "temoin")];
    assert!(refus_de(
        &b.ent
            .declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(2))
            .await,
        "au moins un élève"
    ));
    assert!(b.evenements("internat.malaise_signale").await.is_empty());
    // Un élève concerné + un témoin : un signal pour l'élève concerné, accusé dans la réponse.
    s.participants = vec![pe(e1, "victime"), pe(e2, "temoin")];
    let d = b
        .ent
        .declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(3))
        .await
        .unwrap();
    assert_eq!(d.signaux_sante, 1);
    assert_eq!(b.evenements("internat.malaise_signale").await.len(), 1);
    assert_eq!(
        d.avertissement.as_deref(),
        Some(AVERTISSEMENT_TEXTE_LIBRE),
        "F12 : rappel à la saisie"
    );
}

#[tokio::test]
async fn m4_les_ecritures_sur_un_evenement_rendent_un_accuse_minimal_sans_droit_de_lecture() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.b[0]);
    // Déclarant sans droit de lecture de la liaison, ni portée sur e2.
    let u = personnel_avec(&b, &[Action::InternatEvenementDeclarer]).await;
    let mut s = evt("incident");
    s.participants = vec![pe(e1, "auteur_presume"), pe(e2, "victime")];
    s.complement = Some("Bousculade dans le couloir".into());
    let d = b
        .ent
        .declarer_evenement_a(b.etab, u, PERSONNEL, &s, soir_14(1))
        .await
        .unwrap();
    assert!(
        d.accuse_minimal && d.participants.is_empty() && d.complements.is_empty(),
        "pas de droit de lecture : accusé minimal"
    );
    assert_eq!(d.nature, "incident");
    let c = b
        .ent
        .completer_evenement_a(
            b.etab,
            u,
            PERSONNEL,
            d.id,
            &ComplementSaisieDto {
                texte: "Calme revenu".into(),
            },
            soir_14(2),
        )
        .await
        .unwrap();
    assert!(c.accuse_minimal && c.complements.is_empty());
    // Le CPE (lecture + portée d'établissement) reçoit l'événement complet ; sa lecture est journalisée par élève.
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND objet = 'registre'")
        .bind(b.etab).fetch_one(b.ent.pool_verif()).await.unwrap();
    let c = b
        .ent
        .completer_evenement_a(
            b.etab,
            b.cpe,
            CPE,
            d.id,
            &ComplementSaisieDto {
                texte: "Rencontre prévue".into(),
            },
            soir_14(3),
        )
        .await
        .unwrap();
    assert!(!c.accuse_minimal && c.participants.len() == 2 && c.complements.len() == 3);
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND objet = 'registre'")
        .bind(b.etab).fetch_one(b.ent.pool_verif()).await.unwrap();
    assert!(
        apres > avant,
        "la lecture rendue par une écriture est journalisée"
    );
    // Un lecteur dont la portée ne couvre pas tous les participants reçoit l'accusé minimal (même sur une demande d'intervention).
    let classes = classes_des_deux_zones(&b).await;
    let restreint = personnel_portee(
        &b,
        &[
            Action::InternatEvenementDeclarer,
            Action::InternatLiaisonLire,
        ],
        ent_auth::Portee::Designes,
        vec![classes[0]],
    )
    .await;
    let mut s2 = evt("degradation");
    s2.zone_id = Some(b.zone_a);
    s2.participants = vec![pe(e1, "present"), pe(e2, "present")];
    let d2 = b
        .ent
        .declarer_evenement_a(b.etab, b.cpe, CPE, &s2, soir_14(5))
        .await
        .unwrap();
    let r = b
        .ent
        .demander_intervention_a(b.etab, restreint, PERSONNEL, d2.id, soir_14(6))
        .await
        .unwrap();
    assert!(
        r.accuse_minimal && r.participants.is_empty(),
        "participant hors portée"
    );
}

#[tokio::test]
async fn f1_le_droit_est_controle_avant_l_existence() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    // Réponse sur une sortie inexistante par un parent d'un AUTRE élève : interdit, pas « introuvable ».
    let (autre, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(
        &repondre(&b, autre, Uuid::new_v4(), e, true).await
    ));
    // Le vrai titulaire apprend, lui, que la sortie n'existe pas.
    assert!(matches!(
        repondre(&b, p, Uuid::new_v4(), e, true).await,
        Err(EntDbError::Domain(_))
    ));
    // Compléter un événement inexistant sans aucun droit : interdit.
    let r = b
        .ent
        .completer_evenement_a(
            b.etab,
            b.prof,
            PROF,
            Uuid::new_v4(),
            &ComplementSaisieDto { texte: "x".into() },
            soir_14(1),
        )
        .await;
    assert!(est_interdit(&r), "{r:?}");
}

#[tokio::test]
async fn f12_la_saisie_de_liaison_rappelle_de_ne_pas_saisir_de_sante() {
    let Some(b) = banc().await else { return };
    let d = b
        .ent
        .ecrire_liaison_a(
            b.etab,
            b.cpe,
            CPE,
            &fait(b.a[0], "fatigue", Some("Couché tôt")),
            soir_14(1),
        )
        .await
        .unwrap();
    assert_eq!(d.avertissement.as_deref(), Some(AVERTISSEMENT_TEXTE_LIBRE));
    // La liste de mots reste une aide : le rappel accompagne chaque saisie, même quand le texte passe.
    assert!(AVERTISSEMENT_TEXTE_LIBRE.contains("aide"));
}

// ───────────────────────── M5 : fusion de fiches ─────────────────────────

#[tokio::test]
async fn m5_la_fusion_deplace_les_vigilances_actives_les_inscriptions_et_les_sorties_non_closes() {
    let Some(b) = banc().await else { return };
    let (absorbe, conserve) = (b.b[0], b.b[1]);
    // Vigilances : une active (suit), une levée (reste sur l'absorbé).
    let vig = |code: &str, au: Option<NaiveDate>| VigilanceSaisieDto {
        eleve_id: absorbe,
        code: code.into(),
        referent_user_id: None,
        du: Some(avril(14)),
        au,
    };
    let active = b
        .ent
        .poser_vigilance_a(
            b.etab,
            b.cpe,
            CPE,
            &vig(
                "suivi_rapproche",
                Some(NaiveDate::from_ymd_opt(2028, 3, 1).unwrap()),
            ),
            t0(),
        )
        .await
        .unwrap();
    let levee = b
        .ent
        .poser_vigilance_a(b.etab, b.cpe, CPE, &vig("premier_mois", None), t0())
        .await
        .unwrap();
    b.ent
        .lever_vigilance_a(b.etab, b.cpe, CPE, levee.id, t0())
        .await
        .unwrap();
    // Inscriptions : T1 seule (suit), T2 où le conservé est déjà inscrit (conflit : le conservé l'emporte).
    let mut t1 = etude(None);
    t1.encadrant_user_id = Some(b.prof);
    t1.debut = paris(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap(), 19, 0);
    t1.fin = paris(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap(), 20, 0);
    let mut t2 = t1.clone();
    t2.debut = paris(NaiveDate::from_ymd_opt(2027, 1, 6).unwrap(), 19, 0);
    t2.fin = paris(NaiveDate::from_ymd_opt(2027, 1, 6).unwrap(), 20, 0);
    let (t1, t2) = (
        b.ent
            .creer_temps_a(b.etab, b.cpe, CPE, &t1, t0() + Duration::hours(1))
            .await,
        b.ent
            .creer_temps_a(b.etab, b.cpe, CPE, &t2, t0() + Duration::hours(1))
            .await,
    );
    let (t1, t2) = (t1.unwrap(), t2.unwrap());
    let ins = |t: Uuid, e: Uuid| {
        let b = &b;
        async move {
            b.ent
                .inscrire_temps_a(
                    b.etab,
                    b.cpe,
                    CPE,
                    t,
                    &InscriptionTempsDto { eleve_id: e },
                    t0() + Duration::hours(2),
                )
                .await
                .unwrap()
        }
    };
    ins(t1.id, absorbe).await;
    ins(t2.id, absorbe).await;
    ins(t2.id, conserve).await;
    // Sorties : S1 non close (suit), S3 où les deux sont invités (conflit), S2 annulée (reste).
    let g_cree = |eleves: Vec<Uuid>, debut: DateTime<Utc>| {
        let b = &b;
        async move {
            let mut s = saisie_groupe(vec![b.prof], eleves);
            s.debut = debut;
            s.fin = debut + Duration::hours(3);
            b.ent
                .creer_sortie_groupe_a(b.etab, b.cpe, CPE, &s, t0())
                .await
                .unwrap()
        }
    };
    let s1 = g_cree(vec![absorbe], paris(avril(20), 14, 0)).await;
    let s2 = g_cree(vec![absorbe], paris(avril(21), 14, 0)).await;
    b.ent
        .annuler_sortie_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            s2.id,
            &AnnulationGroupeDto {
                motif_code: "meteo".into(),
            },
            h(1),
        )
        .await
        .unwrap();
    let s3 = g_cree(vec![absorbe, conserve], paris(avril(22), 14, 0)).await;
    // Fusion (tous les conflits de l'aperçu sont regardés et acquittés : la ligne du conservé l'emporte).
    let ap = b
        .ent
        .apercu_fusion_pour_acteur(DIR, b.etab, b.dir, absorbe, conserve)
        .await
        .unwrap();
    let mut d = crate::eleve_fusion::DemandeFusion {
        etablissement_id: b.etab,
        absorbe_id: absorbe,
        conserve_id: conserve,
        motif_code: "doublon_saisie".into(),
        motif: None,
        conflits_acquittes: ap.conflits,
        acteur: b.dir,
    };
    d.conflits_acquittes
        .sort_by_key(|c| (c.table.clone(), c.cle.to_string()));
    let f = match b.ent.fusionner_eleves_pour_acteur(DIR, d).await.unwrap() {
        crate::eleve_fusion::ResultatFusion::Faite(f) => f,
        crate::eleve_fusion::ResultatFusion::ConflitsAResoudre(c) => {
            panic!("conflits inattendus : {c:?}")
        }
    };
    let proprietaire = |table: &'static str, col: &'static str, id: Uuid, eleve: Uuid| {
        let b = &b;
        async move {
            let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE etablissement_id = $1 AND {col} = $2 AND eleve_id = $3"))
                .bind(b.etab).bind(id).bind(eleve).fetch_one(b.ent.pool_verif()).await.unwrap();
            n
        }
    };
    let vigilance = |id: Uuid| {
        let b = &b;
        async move {
            sqlx::query_scalar::<_, Uuid>("SELECT eleve_id FROM ent_internat_vigilance WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(id).fetch_one(b.ent.pool_verif()).await.unwrap()
        }
    };
    assert_eq!(
        vigilance(active.id).await,
        conserve,
        "vigilance active déplacée"
    );
    assert_eq!(
        vigilance(levee.id).await,
        absorbe,
        "vigilance levée : historique, reste"
    );
    assert_eq!(
        proprietaire(
            "ent_internat_temps_participant",
            "temps_id",
            t1.id,
            conserve
        )
        .await,
        1,
        "inscription en cours déplacée"
    );
    assert_eq!(
        proprietaire("ent_internat_temps_participant", "temps_id", t2.id, absorbe).await,
        1,
        "conflit : celle du conservé l'emporte"
    );
    assert_eq!(
        proprietaire(
            "ent_internat_temps_participant",
            "temps_id",
            t2.id,
            conserve
        )
        .await,
        1
    );
    assert_eq!(
        proprietaire(
            "ent_internat_sortie_groupe_participant",
            "sortie_groupe_id",
            s1.id,
            conserve
        )
        .await,
        1,
        "sortie non close déplacée"
    );
    assert_eq!(
        proprietaire(
            "ent_internat_sortie_groupe_participant",
            "sortie_groupe_id",
            s2.id,
            absorbe
        )
        .await,
        1,
        "sortie annulée : reste"
    );
    assert_eq!(
        proprietaire(
            "ent_internat_sortie_groupe_participant",
            "sortie_groupe_id",
            s3.id,
            absorbe
        )
        .await,
        1,
        "conflit : celle du conservé l'emporte"
    );
    // Défusion : tout revient à l'identique.
    let r = b
        .ent
        .defusionner_eleves_pour_acteur(DIR, b.etab, b.dir, f.id)
        .await
        .unwrap();
    assert!(
        matches!(r, crate::eleve_fusion::ResultatDefusion::Faite(_)),
        "{r:?}"
    );
    assert_eq!(vigilance(active.id).await, absorbe);
    assert_eq!(
        proprietaire("ent_internat_temps_participant", "temps_id", t1.id, absorbe).await,
        1
    );
    assert_eq!(
        proprietaire(
            "ent_internat_sortie_groupe_participant",
            "sortie_groupe_id",
            s1.id,
            absorbe
        )
        .await,
        1
    );
}

// ───────────────────────── M6, M7, M8, M9 ─────────────────────────

#[tokio::test]
async fn m6_remplacement_pendant_la_sortie_presence_au_depart_et_encadrant_du_temps() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let remplacant = personnel_avec(&b, &[]).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    // Le départ exige au moins un accompagnateur pointé présent (le CPE n'en est pas un).
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e],
                ..Default::default()
            },
            jour_j(10),
        )
        .await;
    assert!(refus_de(&r, "aucun accompagnateur pointé"));
    // Un accompagnateur qui n'est pas de la sortie ne se pointe pas présent.
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e],
                encadrants_presents: vec![remplacant],
            },
            jour_j(10),
        )
        .await;
    assert!(refus_de(&r, "accompagnateur de cette sortie"));
    // Remplacement avant le départ (tracé), puis le remplaçant est pointé présent.
    let d = b
        .ent
        .remplacer_encadrant_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &RemplacementEncadrantDto {
                ancien_user_id: b.prof,
                nouveau_user_id: remplacant,
            },
            jour_j(5),
        )
        .await
        .unwrap();
    assert_eq!(d.encadrants, vec![remplacant]);
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e],
                encadrants_presents: vec![remplacant],
            },
            jour_j(10),
        )
        .await
        .unwrap();
    assert_eq!(resultat(&r, e), "ok");
    let actes: Vec<String> = sqlx::query_scalar("SELECT acte FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND acte IN ('encadrant_remplace', 'encadrant_present') ORDER BY cree_le")
        .bind(b.etab).bind(g.id).fetch_all(b.ent.pool_verif()).await.unwrap();
    assert_eq!(actes, vec!["encadrant_remplace", "encadrant_present"]);
    // Pendant la sortie (après le départ) : remplacement encore possible.
    let encore = personnel_avec(&b, &[]).await;
    b.ent
        .remplacer_encadrant_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &RemplacementEncadrantDto {
                ancien_user_id: remplacant,
                nouveau_user_id: encore,
            },
            jour_j(60),
        )
        .await
        .unwrap();
    // Un parent n'est pas un accompagnateur de remplacement (R-IT25).
    let (parent_u, _) = parent(&b, b.a[2], true).await;
    assert!(refus_de(
        &b.ent
            .remplacer_encadrant_groupe_a(
                b.etab,
                b.cpe,
                CPE,
                g.id,
                &RemplacementEncadrantDto {
                    ancien_user_id: encore,
                    nouveau_user_id: parent_u
                },
                jour_j(61)
            )
            .await,
        "personnel"
    ));
    // Un temps du soir exige un encadrant.
    let mut t = etude(None);
    t.encadrant_user_id = None;
    assert!(refus_de(
        &b.ent.creer_temps_a(b.etab, b.cpe, CPE, &t, t0()).await,
        "encadrant"
    ));
}

#[tokio::test]
async fn m7_des_eleves_ajoutes_apres_l_acceptation_remettent_la_sortie_a_la_decision() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let g = sortie_prete(&b, &[(e1, p1)]).await;
    assert_eq!(detail(&b, g.id).await.statut, "acceptee");
    let d = b
        .ent
        .inviter_eleves_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            g.id,
            &ElevesAjoutDto { eleves: vec![e2] },
            h(4),
        )
        .await
        .unwrap();
    assert_eq!(d.statut, "soumise", "nouvelle décision de la direction");
    assert_eq!(
        (etat_de(&d, e1), etat_de(&d, e2)),
        ("autorise".into(), "en_attente".into())
    );
    // Pas de départ tant que la direction n'a pas décidé de nouveau.
    let r = b
        .ent
        .depart_groupe_a(
            b.etab,
            b.prof,
            PROF,
            g.id,
            &AppelGroupeDto {
                eleves: vec![e1],
                ..Default::default()
            },
            jour_j(10),
        )
        .await;
    assert!(refus_de(&r, "acceptée"));
    repondre(&b, p2, g.id, e2, true).await.unwrap();
    accepter(&b, g.id).await;
    let r = depart(&b, g.id, vec![e1, e2]).await;
    assert_eq!(
        (resultat(&r, e1), resultat(&r, e2)),
        ("ok".into(), "ok".into())
    );
    let actes: Vec<String> = sqlx::query_scalar("SELECT acte FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND acte IN ('acceptation', 'nouvelle_decision') ORDER BY cree_le")
        .bind(b.etab).bind(g.id).fetch_all(b.ent.pool_verif()).await.unwrap();
    assert_eq!(
        actes,
        vec!["acceptation", "nouvelle_decision", "acceptation"]
    );
}

#[tokio::test]
async fn m8_l_inscription_a_un_temps_ne_revele_pas_un_eleve_protege() {
    let Some(b) = banc().await else { return };
    let (visible, protege) = (b.a[0], b.a[1]);
    let t = creer_etude(&b, None).await;
    let u = personnel_avec(&b, &[Action::InternatGerer]).await;
    let ins = |e: Uuid, qui: Uuid, role: &'static str| {
        let b = &b;
        async move {
            b.ent
                .inscrire_temps_a(
                    b.etab,
                    qui,
                    role,
                    t.id,
                    &InscriptionTempsDto { eleve_id: e },
                    mi_etude(1),
                )
                .await
        }
    };
    assert!(ins(visible, u, PERSONNEL).await.is_ok());
    ins(protege, b.cpe, CPE).await.unwrap(); // inscrit AVANT d'être protégé
                                             // Élève protégé (confidentialité renforcée) : introuvable pour un agent sans droit sur la confidentialité.
    sqlx::query("INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du) VALUES ($1,$2,$3,'enc:v1:xx',ARRAY['adresse_protegee'],'2026-01-01')")
        .bind(Uuid::new_v4()).bind(b.etab).bind(protege).execute(b.ent.pool_verif()).await.unwrap();
    let autre = b.a[2];
    assert!(ins(autre, u, PERSONNEL).await.is_ok());
    let r = ins(protege, u, PERSONNEL).await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "protégé introuvable : {r:?}"
    );
    let r = b
        .ent
        .desinscrire_temps_a(b.etab, u, PERSONNEL, t.id, protege, mi_etude(2))
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    // Le protégé n'apparaît jamais dans la liste des inscrits du personnel sans droit sur la confidentialité.
    let liste = b
        .ent
        .lister_temps_a(b.etab, u, PERSONNEL, avril(15), avril(15))
        .await
        .unwrap();
    assert!(!liste[0].inscrits.contains(&protege) && liste[0].inscrits.contains(&visible));
}

#[tokio::test]
async fn m9_etude_non_pointee_signalee_activite_non_rentree_cas_de_nuit() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let t = creer_etude(&b, None).await;
    for e in [e1, e2, e3] {
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
    // Avant le délai (15 min par défaut) : rien. e1 présent, e2 absent (pointés) ; e3 non pointé → signalé une seule fois.
    assert_eq!(
        b.ent
            .veille_etudes_a(b.etab, debut_etude + Duration::minutes(10))
            .await
            .unwrap(),
        0
    );
    let pointer = |e: Uuid, etat: &str, mi: i64| {
        let b = &b;
        let etat = etat.to_string();
        async move {
            b.ent
                .pointer_temps_a(
                    b.etab,
                    b.prof,
                    PROF,
                    t.id,
                    &PointageTempsDto { eleve_id: e, etat },
                    debut_etude + Duration::minutes(mi),
                )
                .await
        }
    };
    pointer(e1, "present", 11).await.unwrap();
    pointer(e2, "absent", 12).await.unwrap();
    assert_eq!(
        b.ent
            .veille_etudes_a(b.etab, debut_etude + Duration::minutes(16))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        b.ent
            .veille_etudes_a(b.etab, debut_etude + Duration::minutes(17))
            .await
            .unwrap(),
        0,
        "signalé une seule fois"
    );
    let evts = b.evenements("internat.etude_non_pointee").await;
    assert_eq!(evts.len(), 1);
    let vue = b
        .ent
        .lister_temps_a(b.etab, b.cpe, CPE, avril(15), avril(15))
        .await
        .unwrap();
    let signale: Vec<Uuid> = vue[0]
        .pointages
        .iter()
        .filter(|p| p.signale)
        .map(|p| p.eleve_id)
        .collect();
    assert_eq!(signale, vec![e3]);
    // Un inscrit pointé deux fois : refusé ; pointage d'un non-inscrit : refusé ; sans droit ni encadrement : interdit.
    assert!(refus_de(&pointer(e1, "absent", 13).await, "déjà pointé"));
    assert!(est_interdit(
        &b.ent
            .pointer_temps_a(
                b.etab,
                b.cpe,
                "parent",
                t.id,
                &PointageTempsDto {
                    eleve_id: e1,
                    etat: "present".into()
                },
                debut_etude + Duration::minutes(14)
            )
            .await
    ));
    // Activité : « en activité » puis non rentré à fin + tolérance → cas de nuit ; le retour le ferme.
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
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            act.id,
            &InscriptionTempsDto { eleve_id: e1 },
            mi_etude(2),
        )
        .await
        .unwrap();
    let t_act = paris(avril(15), 20, 40);
    b.ent
        .pointer_temps_a(
            b.etab,
            b.prof,
            PROF,
            act.id,
            &PointageTempsDto {
                eleve_id: e1,
                etat: "present".into(),
            },
            t_act,
        )
        .await
        .unwrap();
    let fin_act = paris(avril(15), 21, 30);
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin_act + Duration::minutes(10))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, fin_act + Duration::minutes(20))
            .await
            .unwrap(),
        1
    );
    let cas = cas_de(&b, e1).await;
    assert_eq!((cas[0].1.as_str(), cas[0].3), ("non_rentre", Some(fin_act)));
    b.ent
        .pointer_temps_a(
            b.etab,
            b.prof,
            PROF,
            act.id,
            &PointageTempsDto {
                eleve_id: e1,
                etat: "rentre".into(),
            },
            fin_act + Duration::minutes(25),
        )
        .await
        .unwrap();
    assert_eq!(cas_de(&b, e1).await[0].4.as_deref(), Some("retrouve"));
}

// ───────────────────────── M10 : exports de portabilité ─────────────────────────

#[tokio::test]
async fn m10_les_exports_ne_contiennent_ni_texte_libre_ni_autre_eleve() {
    let Some(b) = banc().await else { return };
    use crate::registre_donnees::classer;
    for table in [
        "ent_internat_evenement",
        "ent_internat_evenement_complement",
    ] {
        assert!(
            !classer(table).unwrap().export_portabilite,
            "{table} : ni circonstances ni texte libre dans l'export"
        );
    }
    let liaison = classer("ent_internat_liaison").unwrap();
    assert!(liaison.export_portabilite);
    assert_eq!(liaison.colonnes_hors_export, ["texte"]);
    for col in liaison.colonnes_hors_export {
        let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ent_internat_liaison' AND column_name = $1)")
            .bind(col).fetch_one(b.ent.pool_verif()).await.unwrap();
        assert!(existe, "colonne déclarée hors export : {col}");
    }
}

// ───────────────────────── F4, F5, F6, F7, F9, F11 ─────────────────────────

#[tokio::test]
async fn f4_un_accompagnateur_sorti_du_personnel_ou_apres_la_sortie_ne_lit_plus() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    assert!(b
        .ent
        .sortie_groupe_a(b.etab, b.prof, PROF, g.id, jour_j(1))
        .await
        .is_ok());
    // Lecture bornée dans le temps : 7 jours après la fin, l'accès tombe.
    let tard = fin() + Duration::days(8);
    assert!(est_interdit(
        &b.ent
            .sortie_groupe_a(b.etab, b.prof, PROF, g.id, tard)
            .await
    ));
    // Adhésion terminée : l'inscription dans la table ne suffit pas.
    sqlx::query(
        "UPDATE ent_membership SET etat = 'ferme' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(b.etab)
    .bind(b.prof)
    .execute(b.ent.pool_verif())
    .await
    .ok();
    let r = b
        .ent
        .sortie_groupe_a(b.etab, b.prof, PROF, g.id, jour_j(2))
        .await;
    assert!(est_interdit(&r), "{r:?}");
}

#[tokio::test]
async fn f5_la_lecture_d_une_sortie_est_journalisee_par_eleve() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let g = brouillon(&b, vec![b.prof], vec![e1, e2]).await;
    b.ent
        .sortie_groupe_a(b.etab, b.cpe, CPE, g.id, jour_j(1))
        .await
        .unwrap();
    let lus: Vec<Vec<Uuid>> = sqlx::query_scalar("SELECT eleve_ids FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND objet = 'sorties' AND user_id = $2 AND eleve_ids IS NOT NULL")
        .bind(b.etab).bind(b.cpe).fetch_all(b.ent.pool_verif()).await.unwrap();
    assert!(
        lus.iter().any(|l| l.contains(&e1) && l.contains(&e2)),
        "{lus:?}"
    );
}

#[tokio::test]
async fn f6_le_depart_apres_minuit_se_raisonne_en_nuit_y_compris_au_changement_d_heure() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    // Sortie qui commence à 00 h 30 (heure d'été) : c'est la NUIT du 15 → 16 avril, pas le jour civil du 16.
    let deb = local(avril(16), 0, 30);
    let mut s = saisie_groupe(vec![b.prof], vec![e]);
    s.debut = deb;
    s.fin = deb + Duration::hours(3);
    let g = b
        .ent
        .creer_sortie_groupe_a(b.etab, b.cpe, CPE, &s, t0())
        .await
        .unwrap();
    let g = b
        .ent
        .soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, h(1))
        .await
        .unwrap();
    repondre(&b, p, g.id, e, true).await.unwrap();
    accepter(&b, g.id).await;
    // La liste « du jour » se lit par nuit : la sortie de 00 h 30 est de la nuit du 15.
    let nuit_15 = b
        .ent
        .lister_sorties_groupe_a(b.etab, b.cpe, CPE, avril(15), avril(15), t0())
        .await
        .unwrap();
    let nuit_16 = b
        .ent
        .lister_sorties_groupe_a(b.etab, b.cpe, CPE, avril(16), avril(16), t0())
        .await
        .unwrap();
    assert!(nuit_15.iter().any(|x| x.id == g.id) && !nuit_16.iter().any(|x| x.id == g.id));
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
            deb + Duration::minutes(5),
        )
        .await
        .unwrap();
    assert_eq!(resultat(&r, e), "ok");
    // Changement d'heure (29 mars 2026, 2 h → 3 h) : un départ à 01 h 30 UTC (03 h 30 locale) reste de la nuit du 28.
    let nuit = |instant: DateTime<Utc>| {
        let b = &b;
        async move {
            sqlx::query_scalar::<_, NaiveDate>("SELECT ent_nuit_etab($1, $2)")
                .bind(b.etab)
                .bind(instant)
                .fetch_one(b.ent.pool_verif())
                .await
                .unwrap()
        }
    };
    let mars28 = NaiveDate::from_ymd_opt(2026, 3, 28).unwrap();
    let utc = |h: u32, m: u32| {
        NaiveDate::from_ymd_opt(2026, 3, 29)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
            .and_utc()
    };
    for instant in [utc(0, 30), utc(1, 30), utc(2, 0)] {
        assert_eq!(nuit(instant).await, mars28, "{instant}");
    }
    // Une sortie qui chevauche le changement d'heure garde sa durée réelle (3 h : 00 h 30 UTC → 03 h 30 UTC).
    let mut s2 = saisie_groupe(vec![b.prof], vec![b.a[1]]);
    s2.debut = utc(0, 30);
    s2.fin = utc(3, 30);
    let r = b
        .ent
        .creer_sortie_groupe_a(
            b.etab,
            b.cpe,
            CPE,
            &s2,
            NaiveDate::from_ymd_opt(2026, 3, 1)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap()
                .and_utc(),
        )
        .await
        .unwrap();
    assert_eq!((r.fin - r.debut).num_minutes(), 180);
    let liste = b
        .ent
        .lister_sorties_groupe_a(b.etab, b.cpe, CPE, mars28, mars28, t0())
        .await
        .unwrap();
    assert!(liste.iter().any(|x| x.id == r.id), "nuit du 28 mars");
}

#[tokio::test]
async fn f7_un_eleve_ne_voit_ni_le_nombre_d_inscrits_ni_les_places() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let u =
        super::tests_internat_i3::compte_eleve(&b, e, NaiveDate::from_ymd_opt(2010, 1, 1).unwrap())
            .await;
    let t = creer_etude(&b, Some(10)).await;
    let d = b
        .ent
        .inscrire_temps_a(
            b.etab,
            u,
            "eleve",
            t.id,
            &InscriptionTempsDto { eleve_id: e },
            mi_etude(1),
        )
        .await
        .unwrap();
    assert_eq!((d.nb_inscrits, d.places), (None, None));
    let json = serde_json::to_string(&d).unwrap();
    assert!(!json.contains("\"places\":10"), "{json}");
    let personnel = b
        .ent
        .lister_temps_a(b.etab, b.cpe, CPE, avril(15), avril(15))
        .await
        .unwrap();
    assert_eq!(
        (personnel[0].nb_inscrits, personnel[0].places),
        (Some(1), Some(10))
    );
}

#[tokio::test]
async fn f9_chevauchement_temps_et_sortie_dans_les_deux_sens() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    // Sens 1 : élève déjà inscrit à un temps → écarté de la sortie (jamais invité sur un créneau qu'il occupe).
    let mut etude_soir = etude(None);
    etude_soir.encadrant_user_id = Some(b.prof);
    etude_soir.debut = debut() + Duration::minutes(30);
    etude_soir.fin = debut() + Duration::hours(2);
    let t = b
        .ent
        .creer_temps_a(b.etab, b.cpe, CPE, &etude_soir, t0())
        .await
        .unwrap();
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            t.id,
            &InscriptionTempsDto { eleve_id: e1 },
            h(1),
        )
        .await
        .unwrap();
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    assert_eq!(
        etat_de(&g, e1),
        "non_autorise",
        "chevauche le temps où il est inscrit"
    );
    let motif = g
        .participants
        .iter()
        .find(|p| p.eleve_id == e1)
        .unwrap()
        .motif
        .clone();
    assert_eq!(motif.as_deref(), Some("chevauchement"));
    assert_eq!(etat_de(&g, e2), "en_attente");
    // Sens 2 : élève en attente de réponse (ou autorisé) à la sortie → ne s'inscrit pas au temps qui chevauche.
    let r = b
        .ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            t.id,
            &InscriptionTempsDto { eleve_id: e2 },
            h(2),
        )
        .await;
    assert!(refus_de(&r, "déjà un temps ou une sortie"));
    repondre(&b, p2, g.id, e2, true).await.unwrap();
    assert!(refus_de(
        &b.ent
            .inscrire_temps_a(
                b.etab,
                b.cpe,
                CPE,
                t.id,
                &InscriptionTempsDto { eleve_id: e2 },
                h(3)
            )
            .await,
        "déjà un temps ou une sortie"
    ));
    let _ = p1;
}

#[tokio::test]
async fn f11_un_eleve_qui_n_est_plus_interne_est_bloque_avec_son_propre_fait() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    // L'élève quitte l'internat avant le départ (séjour clos la veille).
    sqlx::query("UPDATE ent_internat_sejour SET au = '2026-04-15' WHERE etablissement_id = $1 AND eleve_id = $2").bind(b.etab).bind(e).execute(b.ent.pool_verif()).await.unwrap();
    let r = depart(&b, g.id, vec![e]).await;
    assert_eq!(resultat(&r, e), "non_autorise");
    let acte: String = sqlx::query_scalar("SELECT acte FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 AND motif_code = 'plus_interne'")
        .bind(b.etab).bind(g.id).bind(e).fetch_one(b.ent.pool_verif()).await.unwrap();
    assert_eq!(
        acte, "blocage_plus_interne",
        "jamais « blocage_interdiction »"
    );
}

#[tokio::test]
async fn f8_l_accord_d_un_titulaire_tombe_a_la_majorite_de_l_eleve() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    // Mineur au moment de la réponse (17 ans le 14 avril 2026), majeur le jour de la sortie (18 ans le 15 avril).
    crate::tests_formulaires_fo0::poser_naissance(
        &b.ent,
        b.etab,
        e,
        NaiveDate::from_ymd_opt(2008, 4, 15),
        Some("declaree"),
    )
    .await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    let r = depart_cause(&b, g.id, vec![e]).await;
    assert_eq!(resultat(&r, e), "accord_tombe");
    let d = detail(&b, g.id).await;
    assert_eq!(d.participants[0].motif.as_deref(), Some("majorite"));
}

// ───────────────────────── Concurrence ─────────────────────────

#[tokio::test]
async fn concurrence_deux_surveillants_sur_un_depart_un_seul_depart_est_constate() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    let appel = |qui: Uuid, role: &'static str| {
        let b = &b;
        async move {
            b.ent
                .depart_groupe_a(
                    b.etab,
                    qui,
                    role,
                    g.id,
                    &AppelGroupeDto {
                        eleves: vec![e],
                        encadrants_presents: vec![b.prof],
                    },
                    jour_j(10),
                )
                .await
        }
    };
    let (r1, r2) = tokio::join!(appel(b.cpe, CPE), appel(b.prof, PROF));
    let mut res = vec![resultat(&r1.unwrap(), e), resultat(&r2.unwrap(), e)];
    res.sort();
    assert_eq!(res, vec!["deja_parti", "ok"], "un seul départ constaté");
    let departs: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND acte = 'depart'")
        .bind(b.etab).bind(g.id).fetch_one(b.ent.pool_verif()).await.unwrap();
    assert_eq!(departs, 1);
}

#[tokio::test]
async fn concurrence_reponse_et_depart_simultanes_ne_laissent_pas_partir_sans_accord() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    accepter(&b, g.id).await;
    // La famille répond « non » au moment où l'on appelle le départ : l'élève ne part jamais avec un refus enregistré.
    let appel = AppelGroupeDto {
        eleves: vec![e],
        ..Default::default()
    };
    let (rep, dep) = tokio::join!(
        b.ent.repondre_sortie_groupe_a(
            b.etab,
            p,
            PARENT,
            g.id,
            e,
            &ReponseGroupeDto { accord: false },
            debut() - Duration::minutes(31)
        ),
        b.ent
            .depart_groupe_a(b.etab, b.prof, PROF, g.id, &appel, jour_j(10)),
    );
    let _ = rep;
    let etat = etat_de(&detail(&b, g.id).await, e);
    let dep = dep.unwrap();
    assert!(
        matches!(etat.as_str(), "refuse") && resultat(&dep, e) == "non_autorise",
        "état {etat}, départ {dep:?}"
    );
}

#[tokio::test]
async fn concurrence_double_inscription_un_seul_inscrit_et_chevauchement_sous_verrou() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let t = creer_etude(&b, None).await;
    let ins = || {
        let b = &b;
        async move {
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
        }
    };
    let (r1, r2) = tokio::join!(ins(), ins());
    assert_eq!(r1.is_ok() as u8 + r2.is_ok() as u8, 1, "{r1:?} / {r2:?}");
    // Deux temps qui se chevauchent, deux inscriptions simultanées du même élève : une seule passe.
    let mut autre = etude(None);
    autre.encadrant_user_id = Some(b.prof);
    autre.debut = paris(avril(15), 19, 30);
    autre.fin = paris(avril(15), 21, 0);
    let t2 = b
        .ent
        .creer_temps_a(b.etab, b.cpe, CPE, &autre, t0())
        .await
        .unwrap();
    let e2 = b.a[1];
    let a = || {
        let b = &b;
        async move {
            b.ent
                .inscrire_temps_a(
                    b.etab,
                    b.cpe,
                    CPE,
                    t.id,
                    &InscriptionTempsDto { eleve_id: e2 },
                    mi_etude(2),
                )
                .await
        }
    };
    let c = || {
        let b = &b;
        async move {
            b.ent
                .inscrire_temps_a(
                    b.etab,
                    b.cpe,
                    CPE,
                    t2.id,
                    &InscriptionTempsDto { eleve_id: e2 },
                    mi_etude(2),
                )
                .await
        }
    };
    let (ra, rc) = tokio::join!(a(), c());
    assert_eq!(
        ra.is_ok() as u8 + rc.is_ok() as u8,
        1,
        "un seul créneau : {ra:?} / {rc:?}"
    );
}

// ───────────────────────── Rejeu des migrations avec des données d'I4 ─────────────────────────

#[tokio::test]
async fn migrations_le_rejeu_avec_des_donnees_d_i4_ne_viole_aucune_contrainte() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    b.ent
        .sortie_groupe_a(b.etab, b.cpe, CPE, g.id, jour_j(20))
        .await
        .unwrap();
    b.ent
        .veille_non_rentres_a(b.etab, fin() + Duration::minutes(20))
        .await
        .unwrap();
    let mut s = evt("incident");
    s.participants = vec![pe(e, "victime")];
    b.ent
        .declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1))
        .await
        .unwrap();
    // Le journal ne porte que des objets déjà permis par 2824 / 2826 (qui recréent leur contrainte au rejeu).
    let hors: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE objet NOT IN ('fiche_nuit','liaison','presents','registre','cas','appel','appel_pdf','attendu','liste_appels','procedure','effectif_repas','listes','journal','liste_remise','sorties','correspondants')")
        .fetch_one(b.ent.pool_verif()).await.unwrap();
    assert_eq!(hors, 0);
    crate::migrate::rejouer_tout(b.ent.pool_verif())
        .await
        .unwrap();
    // Après le rejeu : la dernière définition de l'attendu est celle d'I4 (branche des sorties de groupe), les gardes tiennent.
    let def: String = sqlx::query_scalar(
        "SELECT pg_get_functiondef('ent_internat_attendu_calcule(uuid, date)'::regprocedure)",
    )
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert!(
        def.contains("ent_internat_sortie_groupe_participant"),
        "attendu d'I4 écrasé par le rejeu de 2826"
    );
    let r = sqlx::query("UPDATE ent_internat_sortie_groupe_participant SET etat = 'autorise' WHERE etablissement_id = $1 AND eleve_id = $2").bind(b.etab).bind(e).execute(b.ent.pool_verif()).await;
    assert!(r.is_err(), "garde de transition rétablie");
}
