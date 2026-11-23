//! Socle 41, lot I4 — **suites 2** (faibles de la contre-revue des suites, D-069) testées contre la base locale de test
//! (feature `labo`).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande reçoit son instant ; jours fixes d'avril 2026
//! (Paris = UTC + 2). Seule exception assumée : l'âge d'un événement (consommateur d'avis), posé **par la base** sur une
//! minute calculée une fois, sans lire l'horloge du test.
//!
//! | Point | Test(s) |
//! |---|---|
//! | F-g serrés : parent interdit, majeur, non-titulaire, `rentre`, `non_autorise`, second refus ; contenu de l'avis | `s4_fg_*` |
//! | Pointage du soir : autre temps, autre établissement | `s4_pointage_*` |
//! | D-069 : protégé pointé par l'encadrant ou le droit de confidentialité seulement | `s4_pointage_un_protege_*` |
//! | Avis de retour : sortie désignée, accompagnateurs | `s4_fg_un_parent_interdit_*` |
//! | Avis regroupé : le nombre est mis à jour | `s4_avis_regroupe_*` |

use super::tests_internat_i2::{banc, refus_de, Banc, CPE, DIR};
use super::tests_internat_i3::{avril, compte_eleve, interdire, parent, paris};
use super::tests_internat_i4::*;
use super::tests_internat_i4_correctifs::{cas_de, sortie_prete};
use super::tests_internat_i4_passe2::{apres_fin, distribuer};
use super::tests_internat_i4_passe3::proteger;
use super::*;
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_types::internat_vie::*;
use uuid::Uuid;

fn saisie(pers: Uuid, accord: bool) -> AccordHorsLigneDto {
    AccordHorsLigneDto {
        donne_par_personne_id: pers,
        accord,
        recu_le: h(1),
        canal: "telephone".into(),
        piece_id: None,
    }
}

async fn faits(b: &Banc, g: Uuid, e: Uuid, acte: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_internat_sortie_groupe_fait \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 AND acte = $4",
    )
    .bind(b.etab)
    .bind(g)
    .bind(e)
    .bind(acte)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap()
}

async fn nb_refus_hors_ligne(b: &Banc, g: Uuid, e: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_internat_sortie_groupe_accord_hors_ligne \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 AND accord = false",
    )
    .bind(b.etab)
    .bind(g)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap()
}

async fn refuser(b: &Banc, g: Uuid, e: Uuid, pers: Uuid) -> Result<SortieGroupeDto, EntDbError> {
    b.ent
        .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g, e, &saisie(pers, false), jour_j(40))
        .await
}

async fn avis_de(b: &Banc, user: Uuid, kind: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT corps FROM ent_notification \
          WHERE etablissement_id = $1 AND kind = $2 AND destinataire_user_id = $3 ORDER BY created_at",
    )
    .bind(b.etab)
    .bind(kind)
    .bind(user)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap()
}

// ───────────────────────── F-g : refus reçu après le départ, cas limites ─────────────────────────

#[tokio::test]
async fn s4_fg_un_parent_interdit_apres_le_depart_son_refus_est_trace_et_l_avis_designe_la_sortie_sans_nom(
) {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, pers) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    // Le parent devient interdit APRÈS le départ : son refus, plus restrictif, reste enregistré (la trace nomme l'auteur).
    interdire(&b, e, Some(pers)).await;
    refuser(&b, g.id, e, pers)
        .await
        .expect("le refus d'un parent interdit s'enregistre après le départ");
    assert_eq!(
        etat_de(&detail(&b, g.id).await, e),
        "parti",
        "le départ n'est pas annulé après coup"
    );
    assert_eq!(faits(&b, g.id, e, "refus_apres_depart").await, 1);
    assert_eq!(nb_refus_hors_ligne(&b, g.id, e).await, 1);
    assert_eq!(b.evenements("internat.retour_demande").await.len(), 1);
    distribuer(&b).await;
    // L'avis atteint le CPE ET l'accompagnateur de la sortie ; il désigne la sortie (titre, heure de fin), jamais l'élève.
    let (nom, prenom): (String, String) =
        sqlx::query_as("SELECT nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(b.etab)
            .bind(e)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap();
    for (qui, libelle) in [(b.cpe, "le responsable"), (b.prof, "l'accompagnateur")] {
        let avis = avis_de(&b, qui, "internat_retour_demande").await;
        assert_eq!(
            avis.len(),
            1,
            "{libelle} reçoit l'avis de retour : {avis:?}"
        );
        let corps = &avis[0];
        assert!(
            corps.contains("Sortie cinéma"),
            "le titre de la sortie : {corps}"
        );
        assert!(corps.contains("18h00"), "l'heure de fin (Paris) : {corps}");
        for interdit in [
            nom.as_str(),
            prenom.as_str(),
            &e.to_string(),
            &pers.to_string(),
        ] {
            assert!(
                !corps.contains(interdit),
                "aucun nom ni identifiant d'élève ou de parent : {corps}"
            );
        }
    }
    // Un compte du personnel qui n'est ni responsable ni accompagnateur ne reçoit rien.
    let tiers = crate::tests_voyages_v0a::compte(&b.ent, b.etab, PROF).await;
    assert!(avis_de(&b, tiers, "internat_retour_demande")
        .await
        .is_empty());
}

#[tokio::test]
async fn s4_fg_un_eleve_majeur_repond_lui_meme_son_refus_hors_ligne_est_refuse_sans_trace() {
    let Some(b) = banc().await else { return };
    let m = b.a[0];
    compte_eleve(&b, m, NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    let (_, pm) = parent(&b, m, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![m]).await.id).await;
    let r = refuser(&b, g.id, m, pm).await;
    assert!(refus_de(&r, "majeur répond lui-même"), "{r:?}");
    assert_eq!(
        nb_refus_hors_ligne(&b, g.id, m).await,
        0,
        "rien n'est enregistré"
    );
    assert_eq!(faits(&b, g.id, m, "refus_apres_depart").await, 0);
    assert_eq!(faits(&b, g.id, m, "refus_hors_ligne").await, 0);
}

#[tokio::test]
async fn s4_fg_une_personne_non_titulaire_ne_peut_rien_enregistrer_et_l_etat_ne_bouge_pas() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
    // Une personne de l'établissement qui n'a aucun lien avec l'élève.
    let etrangere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, source) VALUES ($1,$2,'Voisin','Paul','saisie')")
        .bind(etrangere)
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let r = refuser(&b, g.id, e, etrangere).await;
    assert!(refus_de(&r, "ne peut pas être enregistré"), "{r:?}");
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "parti");
    assert_eq!(nb_refus_hors_ligne(&b, g.id, e).await, 0);
    assert_eq!(faits(&b, g.id, e, "refus_apres_depart").await, 0);
    assert!(
        b.evenements("internat.retour_demande").await.is_empty(),
        "aucune demande de retour"
    );
}

#[tokio::test]
async fn s4_fg_un_refus_pour_un_eleve_rentre_est_une_trace_sans_changer_l_etat_ni_demander_de_retour(
) {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, pers) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    depart(&b, g.id, vec![e]).await;
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
    refuser(&b, g.id, e, pers)
        .await
        .expect("la trace s'enregistre");
    assert_eq!(
        etat_de(&detail(&b, g.id).await, e),
        "rentre",
        "l'état ne change pas"
    );
    assert_eq!(faits(&b, g.id, e, "refus_apres_depart").await, 1);
    assert!(
        b.evenements("internat.retour_demande").await.is_empty(),
        "un élève rentré n'a aucun retour à demander"
    );
}

#[tokio::test]
async fn s4_fg_un_refus_pour_un_eleve_non_autorise_est_une_trace_sans_changer_l_etat() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, pers) = parent(&b, e, true).await;
    let (_, pers2) = parent(&b, e, true).await;
    let g = sortie_prete(&b, &[(e, p)]).await;
    interdire(&b, e, Some(pers)).await;
    assert_eq!(
        resultat(&depart_cause(&b, g.id, vec![e]).await, e),
        "accord_tombe"
    );
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "non_autorise");
    refuser(&b, g.id, e, pers2)
        .await
        .expect("la trace s'enregistre");
    assert_eq!(
        etat_de(&detail(&b, g.id).await, e),
        "non_autorise",
        "l'état ne change pas"
    );
    assert_eq!(faits(&b, g.id, e, "refus_apres_depart").await, 1);
    assert!(b.evenements("internat.retour_demande").await.is_empty());
}

#[tokio::test]
async fn s4_fg_un_second_refus_pour_un_eleve_deja_refuse_est_une_trace_de_plus() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (_, pers) = parent(&b, e, true).await;
    let (_, pers2) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    // Premier refus : l'élève passe à « refusé ».
    b.ent
        .accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &saisie(pers, false), h(2))
        .await
        .unwrap();
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "refuse");
    assert_eq!(faits(&b, g.id, e, "refus_hors_ligne").await, 1);
    // Second refus (un autre titulaire) : une trace de plus, l'état ne bouge pas, aucun retour à demander.
    refuser(&b, g.id, e, pers2)
        .await
        .expect("la trace s'enregistre");
    assert_eq!(etat_de(&detail(&b, g.id).await, e), "refuse");
    assert_eq!(faits(&b, g.id, e, "refus_hors_ligne").await, 1);
    assert_eq!(faits(&b, g.id, e, "refus_apres_depart").await, 1);
    assert_eq!(nb_refus_hors_ligne(&b, g.id, e).await, 2);
    assert!(b.evenements("internat.retour_demande").await.is_empty());
}

// ───────────────────────── Pointage au temps du soir ─────────────────────────

async fn pointer_par(
    b: &Banc,
    acteur: Uuid,
    role: &str,
    temps: Uuid,
    e: Uuid,
) -> Result<TempsDto, EntDbError> {
    b.ent
        .pointer_temps_a(
            b.etab,
            acteur,
            role,
            temps,
            &PointageTempsDto {
                eleve_id: e,
                etat: "present".into(),
            },
            paris(avril(15), 19, 5),
        )
        .await
}

async fn inscrire(b: &Banc, temps: Uuid, e: Uuid) {
    b.ent
        .inscrire_temps_a(
            b.etab,
            b.cpe,
            CPE,
            temps,
            &InscriptionTempsDto { eleve_id: e },
            mi_etude(1),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn s4_pointage_un_encadrant_d_un_autre_temps_est_refuse_et_un_autre_etablissement_ne_trouve_rien(
) {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let t1 = creer_etude(&b, None).await;
    inscrire(&b, t1.id, e).await;
    // Un second enseignant encadre un AUTRE temps : il n'a aucun droit sur celui-ci.
    let autre = crate::tests_voyages_v0a::compte(&b.ent, b.etab, PROF).await;
    let mut s2 = etude(None);
    s2.libelle = "Étude de la salle 14".into();
    s2.encadrant_user_id = Some(autre);
    b.ent
        .creer_temps_a(b.etab, b.cpe, CPE, &s2, t0())
        .await
        .unwrap();
    let r = pointer_par(&b, autre, PROF, t1.id, e).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // L'encadrant du temps, lui, pointe.
    pointer_par(&b, b.prof, PROF, t1.id, e).await.unwrap();
    // Un autre établissement : le temps et l'élève n'existent pas pour lui.
    let b2 = banc().await.unwrap();
    let r = pointer_par(&b2, b2.cpe, CPE, t1.id, e).await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
}

#[tokio::test]
async fn s4_pointage_un_protege_n_est_pointe_que_par_l_encadrant_ou_le_droit_de_confidentialite() {
    let Some(b) = banc().await else { return };
    let (prot, libre) = (b.a[0], b.a[1]);
    let t = creer_etude(&b, None).await;
    inscrire(&b, t.id, prot).await;
    inscrire(&b, t.id, libre).await;
    // Second temps (même encadrant) où le protégé est inscrit AVANT d'être protégé (ensuite le CPE ne le voit plus).
    let t2 = {
        let mut s = etude(None);
        s.libelle = "Étude bis".into();
        s.encadrant_user_id = Some(b.prof);
        s.debut = paris(avril(15), 21, 0);
        s.fin = paris(avril(15), 22, 0);
        b.ent
            .creer_temps_a(b.etab, b.cpe, CPE, &s, t0())
            .await
            .unwrap()
    };
    inscrire(&b, t2.id, prot).await;
    proteger(&b, prot).await;
    // Un détenteur de portée (internat.appel sur l'établissement) SANS droit de confidentialité : « n'est pas inscrit »,
    // comme un tiers ; un élève non protégé, lui, se pointe (la portée est bien en vigueur).
    let porteur = personnel_avec(&b, &[Action::InternatAppel]).await;
    let r = pointer_par(&b, porteur, PERSONNEL, t.id, prot).await;
    assert!(refus_de(&r, "n'est pas inscrit"), "{r:?}");
    pointer_par(&b, porteur, PERSONNEL, t.id, libre)
        .await
        .expect("la portée suffit pour un élève non protégé");
    // Le détenteur du droit de confidentialité (direction) pointe le protégé.
    pointer_par(&b, b.dir, DIR, t.id, prot)
        .await
        .expect("le droit de confidentialité pointe un protégé inscrit");
    // L'encadrant du temps pointe aussi un protégé inscrit (autre temps, même règle).
    b.ent
        .pointer_temps_a(
            b.etab,
            b.prof,
            PROF,
            t2.id,
            &PointageTempsDto {
                eleve_id: prot,
                etat: "present".into(),
            },
            paris(avril(15), 21, 5),
        )
        .await
        .expect("l'encadrant du temps pointe son protégé inscrit");
    // Garde-fou : le pointage du protégé par un tiers n'a rien écrit.
    let pointe_par: Option<Uuid> = sqlx::query_scalar(
        "SELECT pointe_par_user_id FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND temps_id = $2 AND eleve_id = $3",
    )
    .bind(b.etab)
    .bind(t.id)
    .bind(prot)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        pointe_par,
        Some(b.dir),
        "seul le détenteur du droit a pointé le protégé"
    );
}

// ───────────────────────── Avis regroupé : le nombre est mis à jour ─────────────────────────

#[tokio::test]
async fn s4_avis_regroupe_un_second_evenement_de_la_minute_met_a_jour_l_avis_non_lu() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    // Une minute pleine posée UNE fois par la base (jamais deux lectures de l'horloge : pas de bascule de minute).
    let minute: DateTime<Utc> = sqlx::query_scalar("SELECT date_trunc('minute', now())")
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    let poser = |eleve: Uuid| {
        let (b, minute) = (&b, minute);
        async move {
            sqlx::query(
                "UPDATE ent_metier_event SET created_at = $3 \
                  WHERE etablissement_id = $1 AND type = 'internat.cas_ouvert' AND eleve_id = $2",
            )
            .bind(b.etab)
            .bind(eleve)
            .bind(minute + chrono::Duration::seconds(10))
            .execute(b.ent.pool_verif())
            .await
            .unwrap();
        }
    };
    // Premier cas : avis « simple » (un seul élève).
    let (p1, _) = parent(&b, e1, true).await;
    let g1 = sortie_prete(&b, &[(e1, p1)]).await;
    depart(&b, g1.id, vec![e1]).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(16))
            .await
            .unwrap(),
        1
    );
    poser(e1).await;
    distribuer(&b).await;
    let avant = avis_de(&b, b.cpe, "internat_cas_ouvert").await;
    assert_eq!(avant.len(), 1, "{avant:?}");
    assert!(
        !avant[0].contains("élèves concernés"),
        "un seul élève : pas de nombre : {avant:?}"
    );
    // Second cas dans la même minute, après la première distribution : l'avis non lu est mis à jour (pas de second avis).
    let (p2, _) = parent(&b, e2, true).await;
    let g2 = sortie_prete(&b, &[(e2, p2)]).await;
    depart(&b, g2.id, vec![e2]).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(30))
            .await
            .unwrap(),
        1
    );
    poser(e2).await;
    distribuer(&b).await;
    let apres = avis_de(&b, b.cpe, "internat_cas_ouvert").await;
    assert_eq!(apres.len(), 1, "toujours un seul avis : {apres:?}");
    assert!(
        apres[0].contains("2 élèves"),
        "le nombre est à jour : {apres:?}"
    );
    // Un avis déjà LU ne change plus sous les yeux de son lecteur.
    sqlx::query("UPDATE ent_notification SET lu_le = $3 WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'internat_cas_ouvert'")
        .bind(b.etab)
        .bind(b.cpe)
        .bind(minute)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let (p3, _) = parent(&b, e3, true).await;
    let g3 = sortie_prete(&b, &[(e3, p3)]).await;
    depart(&b, g3.id, vec![e3]).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(50))
            .await
            .unwrap(),
        1
    );
    poser(e3).await;
    distribuer(&b).await;
    let lu = avis_de(&b, b.cpe, "internat_cas_ouvert").await;
    assert_eq!(lu, apres, "l'avis lu garde son texte");
    // Les cas existent bien tous les trois (le nombre vient des événements, pas d'un compteur).
    assert_eq!(cas_de(&b, e3).await.len(), 1);
}
