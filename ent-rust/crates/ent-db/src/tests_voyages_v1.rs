//! Socle 42, lot V1 (exigences et autorisations, guichet, désaccords, dérogations, « qui manque quoi »,
//! relances, ordres de mission, liste nominative à un prestataire) contre la base locale de test. Sans
//! `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Cas limites de la conception (`docs/socles/42_SORTIES_VOYAGES.md` §17.1) et défauts de Next.js fermés :
//!
//! * CV-04 `cv_04_…` désaccord déclaré : blocage jusqu'à la décision motivée de la direction ;
//! * CV-05/06 `cv_06_…` titulaire privé d'autorité après signature : « à revoir », l'autre est sollicité ;
//! * CV-07 `cv_07_…` élève majeur : il signe lui-même ; ses parents ne sont pas relancés à sa place ;
//! * CV-09 `cv_09_…` famille qui ne répond pas : relances J-21, J-10, J-5, liste « qui manque » ;
//! * CV-25/26 `cv_25_26_…` refus ou retrait du droit à l'image : l'élève participe, les photos sont filtrées ;
//! * VD-16 `vd_16_…` aucun contact de famille, aucun INE chez un prestataire ;
//! * R-SV6 `r_sv6_…` dérogation : non légale seulement, jamais un refus, jamais un consentement.

use super::*;
use crate::tests_voyages_v0a::{actions_journal, agent_de, compte, est_forbidden, est_introuvable, est_refus, personne_de, Cadre};
use crate::tests_voyages_v0d::{admin, cadre, message, pret_a_basculer, photo, recus_vers};
use crate::voyages_exigences_repo::{AttestationLigne, AttestationSaisie, CampagneSaisie, DecisionRecue, GuichetSaisie, Matrice, ScanSaisie};
use crate::voyages_missions_repo::TransmissionSaisie;
use crate::voyages_pdf::{rendre, Bloc};
use crate::voyages_prestataires_repo::PrestataireSaisie;
use crate::voyages_repo::SortieLigne;
use chrono::{Duration, NaiveDate};
use ent_auth::Role;
use uuid::Uuid;

fn pdf(t: &str) -> Vec<u8> {
    rendre("Coupon", &[Bloc::Paragraphe(t.into())])
}

/// Texte d'un PDF (relu par la table ToUnicode de la police incorporée : les octets ne contiennent pas le texte).
fn pdf_texte(octets: &[u8]) -> String {
    ent_pdf::releve::texte(octets).to_string()
}

/// Met à jour l'identité d'un élève comme le fait une projection (le garde-fou interdit l'écriture directe).
async fn maj_identite(c: &Cadre, eleve: Uuid, set_sql: &str) {
    let mut tx = c.ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)").execute(&mut *tx).await.unwrap();
    sqlx::query(&format!("UPDATE ent_eleve SET {set_sql} WHERE etablissement_id = $1 AND id = $2"))
        .bind(c.etab)
        .bind(eleve)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

impl Cadre {
    /// Sortie facultative autorisée (`ouverte`), trois élèves inscrits.
    async fn facultative(&self, titre: &str) -> SortieLigne {
        let s = self.creer(titre, "sans_nuitee").await;
        self.jusqu_a_ouverte(s.id).await;
        self.ent
            .voyage_participants_ajouter(&self.a(self.prof, Role::Prof), s.id, self.eleves.clone())
            .await
            .expect("participants");
        s
    }

    async fn personne(&self, user: Uuid) -> Uuid {
        personne_de(&self.ent, self.etab, user).await
    }

    async fn etat_participant(&self, sortie: Uuid, eleve: Uuid) -> String {
        sqlx::query_scalar("SELECT etat FROM ent_sortie_participant WHERE sortie_id = $1 AND eleve_id = $2")
            .bind(sortie)
            .bind(eleve)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn campagne(&self, id: Uuid, image: bool, soins: bool) -> Matrice {
        self.ent
            .voyage_campagne_ouvrir(&self.prof_a(), id, CampagneSaisie { avec_image: image, avec_soins: soins })
            .await
            .expect("campagne")
    }

    /// Autre titulaire (avec compte) de l'élève, autorité parentale par défaut.
    async fn titulaire_de(&self, eleve: Uuid) -> (Uuid, Uuid) {
        let u = compte(&self.ent, self.etab, "parent").await;
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::new_v4())
            .bind(self.etab)
            .bind(eleve)
            .bind(u)
            .execute(self.ent.pool())
            .await
            .expect("lien");
        (u, self.personne(u).await)
    }

    fn jour_coupon(&self, il_y_a: i64) -> NaiveDate {
        self.jour - Duration::days(3) - Duration::days(il_y_a)
    }

    fn guichet(&self, participant: Uuid, code: &str, decision: &str, signataire: Uuid) -> GuichetSaisie {
        GuichetSaisie {
            participant_id: participant,
            code: code.into(),
            decision: decision.into(),
            signataire_personne_id: signataire,
            signee_le: self.jour_coupon(0),
            scan: None,
        }
    }

    async fn cellule(&self, id: Uuid, acteur: &Acteur<'_>, eleve: Uuid, code: &str) -> Option<String> {
        let m = self.ent.voyage_exigences_matrice(acteur, id).await.unwrap();
        let p = self.participant(id, eleve).await;
        m.lignes.into_iter().find(|l| l.participant_id == p)?.cellules.into_iter().find(|c| c.code == code).map(|c| c.etat)
    }
}

// ═════════════════════════════ Campagne et exigences ═════════════════════════════

#[tokio::test]
async fn campagne_exigences_calculees_obligatoire_sans_autorisation_et_droits() {
    let Some(c) = cadre().await else { return };
    // Sortie OBLIGATOIRE : gratuite, aucune autorisation parentale ; rien à recueillir hors consentements distincts.
    let mut sa = c.saisie("Visite du collège voisin", "sans_nuitee");
    sa.nature = "obligatoire".into();
    sa.destination = "proximite".into();
    let o = c.ent.voyage_creer(&c.prof_a(), sa).await.unwrap();
    c.preparer_jusqu_a_soumise(o.id).await;
    c.cmd(c.dir, Role::Direction, o.id, crate::voyages::Commande::ValiderPedagogie).await.unwrap();
    c.cmd(c.dir, Role::Direction, o.id, crate::voyages::Commande::Autoriser).await.unwrap();
    c.ent.voyage_participants_ajouter(&c.prof_a(), o.id, c.eleves.clone()).await.unwrap();
    let e = c.ent.voyage_campagne_ouvrir(&c.prof_a(), o.id, CampagneSaisie { avec_image: false, avec_soins: false }).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let m = c.ent.voyage_campagne_ouvrir(&c.prof_a(), o.id, CampagneSaisie { avec_image: true, avec_soins: false }).await.unwrap();
    assert_eq!(m.codes, vec!["droit_image".to_string()], "aucune autorisation de participation pour une sortie obligatoire");
    assert_eq!(m.prets, 3, "le droit à l'image ne bloque personne");

    // Sortie FACULTATIVE : l'autorisation de participation est bloquante ; en attente n'est pas « bloqué ».
    let s = c.facultative("Voyage à Rome").await;
    let accueil = c.a(c.accueil, Role::Accueil);
    // Droits : introuvable pour l'accueil, la famille, un autre établissement ; interdit pour un encadrant non porteur.
    let autre = cadre().await.unwrap();
    for (a, nom) in [(accueil, "accueil"), (c.a(c.parent, Role::Parent), "famille"), (autre.a(autre.dir, Role::Direction), "autre établissement")] {
        assert!(est_introuvable(&c.ent.voyage_campagne_ouvrir(&a, s.id, CampagneSaisie { avec_image: true, avec_soins: true }).await.unwrap_err()), "{nom}");
        assert!(est_introuvable(&c.ent.voyage_exigences_matrice(&a, s.id).await.unwrap_err()), "{nom}");
    }
    let e = c.ent.voyage_campagne_ouvrir(&c.a(c.prof2, Role::Prof), s.id, CampagneSaisie { avec_image: true, avec_soins: true }).await.unwrap_err();
    assert!(est_forbidden(&e), "encadrant non porteur : {e:?}");
    let m = c.campagne(s.id, true, true).await;
    assert_eq!(m.codes, vec!["autorisation_participation".to_string(), "autorisation_soins".to_string(), "droit_image".to_string()]);
    assert_eq!((m.manquants, m.bloques, m.prets), (3, 0, 0));
    assert!(m.lignes.iter().all(|l| l.etat == "inscrit"));
    // Une seule campagne par sortie ; événement sortant vers le socle 18, sans identité d'élève.
    assert!(c.ent.voyage_campagne_ouvrir(&c.prof_a(), s.id, CampagneSaisie { avec_image: true, avec_soins: true }).await.is_err());
    let payload: String = sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE type = 'voyages.campagne_ouverte' AND aggregate_id = $1")
        .bind(s.id)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert!(!payload.contains("Prénom") && !payload.contains("Alpha"));
    // Aucune assurance exigée (guide 2025) : la matrice n'a pas de colonne « assurance ».
    assert!(!m.codes.iter().any(|x| x.contains("assurance")));
}

#[tokio::test]
async fn guichet_papier_signataire_qualifie_date_du_coupon_scan_et_ajout_seul() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage").await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    let parent0 = c.personne(c.parent).await;
    // Sans campagne ouverte : refus.
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", parent0)).await.unwrap_err()));
    c.campagne(s.id, true, false).await;
    // Droits : porteur, secrétariat (guichet) et direction ; pas l'accueil ni la comptabilité sans détail.
    let g = || c.guichet(p0, "autorisation_participation", "accepte", parent0);
    assert!(est_introuvable(&c.ent.voyage_autorisation_guichet(&c.a(c.accueil, Role::Accueil), s.id, g()).await.unwrap_err()));
    assert!(est_introuvable(&c.ent.voyage_autorisation_guichet(&c.a(c.parent, Role::Parent), s.id, g()).await.unwrap_err()));
    // Signataire : un titulaire AVEC autorité de CET élève. Le professeur, ou le parent d'un autre élève : refusé.
    let prof_personne = c.personne(c.prof).await;
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", prof_personne)).await.unwrap_err()));
    let (_, autre_personne) = c.titulaire_de(c.eleves[1]).await;
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", autre_personne)).await.unwrap_err()));
    // Date du coupon : ni dans le futur ni plus ancienne qu'un an ; code non signable ; scan actif refusé.
    let mut futur = g();
    futur.signee_le = c.jour + Duration::days(40);
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, futur).await.unwrap_err()));
    let mut vieux = g();
    vieux.signee_le = c.jour - Duration::days(900);
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, vieux).await.unwrap_err()));
    assert!(c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "desaccord_titulaire", "accepte", parent0)).await.is_err());
    let mut piege = g();
    piege.scan = Some(ScanSaisie { nom: "coupon.pdf".into(), mime: None, contenu: b"<html><script>alert(1)</script></html>".to_vec() });
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, piege).await.unwrap_err()));
    // Un participant d'une autre sortie n'est pas atteignable (introuvable).
    let s2 = c.facultative("Autre sortie").await;
    let p_autre = c.participant(s2.id, c.eleves[0]).await;
    assert!(est_introuvable(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p_autre, "autorisation_participation", "accepte", parent0)).await.unwrap_err()));
    // Signature papier valide avec scan : la date est celle du coupon, la source « guichet », le scan en objet.
    let mut ok = g();
    ok.signee_le = c.jour_coupon(5);
    ok.scan = Some(ScanSaisie { nom: "coupon.pdf".into(), mime: Some("application/pdf".into()), contenu: pdf("coupon signé") });
    let m = c.ent.voyage_autorisation_guichet(&c.a(c.sec, Role::Administratif), s.id, ok.clone()).await.unwrap();
    assert_eq!(m.prets, 1);
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    let (source, signee, cree): (String, NaiveDate, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT f.type, f.signee_le, f.cree_le FROM ent_sortie_exigence_fait f JOIN ent_sortie_exigence x ON x.id = f.exigence_id
          WHERE x.sortie_id = $1 AND x.code = 'autorisation_participation' AND x.participant_id = $2",
    )
    .bind(s.id)
    .bind(p0)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((source.as_str(), signee), ("guichet", ok.signee_le));
    assert!(cree.date_naive() >= signee, "la date de saisie n'est pas la date de signature");
    let (cle, sha): (String, String) = sqlx::query_as("SELECT f.scan_cle_objet, f.scan_sha256 FROM ent_sortie_exigence_fait f JOIN ent_sortie_exigence x ON x.id = f.exigence_id WHERE x.sortie_id = $1 AND f.scan_cle_objet IS NOT NULL")
        .bind(s.id)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(ent_object::sha256_hex(&c.ent.lire_objet(c.etab, &cle).await.unwrap()), sha);
    // Les faits de signature sont en ajout seul (base) ; le journal de la sortie ne recopie aucun nom.
    let r = sqlx::query("UPDATE ent_sortie_exigence_fait SET decision = 'refuse' WHERE exigence_id IN (SELECT id FROM ent_sortie_exigence WHERE sortie_id = $1)")
        .bind(s.id)
        .execute(c.ent.pool())
        .await;
    assert!(matches!(&r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501")), "{r:?}");
    assert!(sqlx::query("DELETE FROM ent_sortie_exigence_fait").execute(c.ent.pool()).await.is_err());
    assert!(actions_journal(&c, s.id).await.contains(&"autorisation_guichet".to_string()));
}

#[tokio::test]
async fn cv_06_titulaire_prive_d_autorite_apres_signature_l_autre_est_sollicite() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage").await;
    c.campagne(s.id, false, false).await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    let (parent1_user, parent1) = (c.parent, c.personne(c.parent).await);
    let (_, parent2) = c.titulaire_de(c.eleves[0]).await;
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", parent1)).await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    // Le signataire perd l'autorité : sa signature est « à revoir », le participant est bloqué (jamais « prêt »).
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = false WHERE user_id = $1 AND eleve_id = $2")
        .bind(parent1_user)
        .bind(c.eleves[0])
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(c.cellule(s.id, &c.prof_a(), c.eleves[0], "autorisation_participation").await.as_deref(), Some("a_revoir"));
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "bloque");
    // L'ancien titulaire ne peut plus signer ; l'autre titulaire signe : remplie, prêt.
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", parent1)).await.unwrap_err()));
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", parent2)).await.unwrap();
    assert_eq!(c.cellule(s.id, &c.prof_a(), c.eleves[0], "autorisation_participation").await.as_deref(), Some("remplie"));
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    // Un seul refus suffit à bloquer (prudence) ; un refus ne se déroge pas ; le titulaire peut se raviser.
    let (_, parent3) = c.titulaire_de(c.eleves[0]).await;
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "refuse", parent3)).await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "bloque");
    let e = c.ent.voyage_exigence_deroger(&c.dir_a(), s.id, p0, "autorisation_participation", "on passe").await.unwrap_err();
    assert!(est_refus(&e), "participation d'une sortie facultative : légale, jamais dérogeable : {e:?}");
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", parent3)).await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret", "la dernière décision de chaque signataire fait foi");
}

#[tokio::test]
async fn cv_07_eleve_majeur_signe_lui_meme_et_ses_parents_ne_sont_pas_relances() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage").await;
    // L'élève 0 est majeur ; son parent (compte) a l'autorité mais ne signe pas à sa place.
    maj_identite(&c, c.eleves[0], "date_naissance = (ent_aujourdhui_etab(etablissement_id) - interval '19 years')::date").await;
    c.campagne(s.id, false, false).await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    let parent = c.personne(c.parent).await;
    assert!(est_refus(&c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", parent)).await.unwrap_err()));
    // La fiche de l'élève (personne rattachée) est le signataire recevable.
    let mut eleve_personne: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2").bind(c.etab).bind(c.eleves[0]).fetch_optional(c.ent.pool()).await.unwrap();
    if eleve_personne.is_none() {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source, eleve_id) VALUES ($1,$2,'Majeur','saisie',$3)").bind(id).bind(c.etab).bind(c.eleves[0]).execute(c.ent.pool()).await.unwrap();
        eleve_personne = Some(id);
    }
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", eleve_personne.unwrap())).await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    // Relance : l'élève majeur n'a pas de compte ici ; son parent n'est PAS relancé à sa place (CV-07).
    let ap = c.ent.voyage_relance_apercu(&c.prof_a(), s.id).await.unwrap();
    assert_eq!(ap.participants_concernes, 2, "les deux mineurs seulement");
}

#[tokio::test]
async fn port_du_socle_18_decision_recue_revalidee_par_le_serveur() {
    let (Some(c), Some(autre)) = (cadre().await, cadre().await) else { return };
    let s = c.facultative("Voyage").await;
    c.campagne(s.id, true, false).await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    let parent = c.personne(c.parent).await;
    let d = |code: &str, decision: &str, signataire: Uuid, jour: NaiveDate| DecisionRecue {
        sortie_id: s.id,
        participant_id: p0,
        code: code.into(),
        decision: decision.into(),
        signataire_personne_id: signataire,
        signee_le: jour,
        reference_externe: Some("FRM-18-001".into()),
    };
    let aujourdhui = c.jour - Duration::days(3);
    // Revalidé : signataire non titulaire, date future, code inconnu, autre établissement (sortie introuvable).
    let prof = c.personne(c.prof).await;
    assert!(est_refus(&c.ent.voyages_autorisation_decision_recue(c.etab, d("autorisation_participation", "accepte", prof, aujourdhui)).await.unwrap_err()));
    assert!(c.ent.voyages_autorisation_decision_recue(c.etab, d("autorisation_participation", "accepte", parent, c.jour + Duration::days(30))).await.is_err());
    assert!(c.ent.voyages_autorisation_decision_recue(c.etab, d("assurance", "accepte", parent, aujourdhui)).await.is_err(), "l'assurance n'est jamais une exigence");
    assert!(est_introuvable(&autre.ent.voyages_autorisation_decision_recue(autre.etab, d("autorisation_participation", "accepte", parent, aujourdhui)).await.unwrap_err()));
    // Décision valable : état de l'exigence rendu, participant prêt, source « formulaire ».
    assert_eq!(c.ent.voyages_autorisation_decision_recue(c.etab, d("autorisation_participation", "accepte", parent, aujourdhui)).await.unwrap(), "remplie");
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    // CV-25 : l'élève dont le titulaire REFUSE le droit à l'image participe normalement ; la décision est consignée.
    assert_eq!(c.ent.voyages_autorisation_decision_recue(c.etab, d("droit_image", "refuse", parent, aujourdhui)).await.unwrap(), "refusee");
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret", "le droit à l'image ne bloque jamais la participation");
    // CV-26 : retrait pendant le séjour : la décision la plus récente fait foi.
    assert_eq!(c.ent.voyages_autorisation_decision_recue(c.etab, d("droit_image", "accepte", parent, aujourdhui)).await.unwrap(), "remplie");
    assert_eq!(c.ent.voyages_autorisation_decision_recue(c.etab, d("droit_image", "retire", parent, aujourdhui)).await.unwrap(), "refusee");
    // Campagne close : plus aucune décision entrante.
    c.ent.voyage_campagne_clore(&c.prof_a(), s.id).await.unwrap();
    assert!(est_refus(&c.ent.voyages_autorisation_decision_recue(c.etab, d("droit_image", "accepte", parent, aujourdhui)).await.unwrap_err()));
}

// ═════════════════════════════ Désaccord et dérogation ═════════════════════════════

#[tokio::test]
async fn cv_04_desaccord_bloque_jusqu_a_la_decision_motivee_de_la_direction() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage").await;
    c.campagne(s.id, false, false).await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    let mere = c.personne(c.parent).await;
    let (pere_user, pere) = c.titulaire_de(c.eleves[0]).await;
    // La mère signe ; le père déclare un désaccord connu de l'établissement.
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "accepte", mere)).await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    assert!(est_refus(&c.ent.voyage_desaccord_declarer(&c.prof_a(), s.id, p0, c.personne(c.prof).await).await.unwrap_err()), "le désaccord vient d'un titulaire");
    c.ent.voyage_desaccord_declarer(&c.prof_a(), s.id, p0, pere).await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "bloque");
    // L'organisateur voit « bloqué » SANS la cause : la colonne désaccord est réservée à la direction.
    let m = c.ent.voyage_exigences_matrice(&c.prof_a(), s.id).await.unwrap();
    assert!(!m.codes.contains(&"desaccord_titulaire".to_string()));
    assert_eq!(m.bloques, 1);
    let md = c.ent.voyage_exigences_matrice(&c.dir_a(), s.id).await.unwrap();
    assert!(md.codes.contains(&"desaccord_titulaire".to_string()));
    // Le journal ne dit pas qui est en désaccord ; aucun titulaire n'est notifié de quoi que ce soit.
    let j = actions_journal(&c, s.id).await;
    assert!(j.contains(&"desaccord_declare".to_string()));
    let notifs: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = ANY($1)").bind(vec![c.parent, pere_user]).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(notifs, 0, "jamais d'information d'un titulaire sur la position de l'autre");
    // Décision : direction seule, motivée ; l'organisateur (même porteur) ne peut pas la prendre.
    assert!(est_forbidden(&c.ent.voyage_desaccord_decider(&c.prof_a(), s.id, p0, "accord présumé").await.unwrap_err()));
    assert!(est_introuvable(&c.ent.voyage_desaccord_decider(&c.a(c.accueil, Role::Accueil), s.id, p0, "x").await.unwrap_err()));
    assert!(c.ent.voyage_desaccord_decider(&c.dir_a(), s.id, p0, "  ").await.is_err(), "motif obligatoire");
    c.ent.voyage_desaccord_decider(&c.dir_a(), s.id, p0, "Accord présumé de l'autre titulaire, décision motivée au dossier").await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[0]).await, "pret");
    // La décision n'est pas l'autorisation : sans la signature de la mère, l'élève ne serait pas prêt (autre élève).
    let p1 = c.participant(s.id, c.eleves[1]).await;
    let (_, tit1) = c.titulaire_de(c.eleves[1]).await;
    c.ent.voyage_desaccord_declarer(&c.prof_a(), s.id, p1, tit1).await.unwrap();
    c.ent.voyage_desaccord_decider(&c.dir_a(), s.id, p1, "Levée du désaccord").await.unwrap();
    assert_eq!(c.etat_participant(s.id, c.eleves[1]).await, "inscrit", "pas d'autorisation : pas prêt, mais plus bloqué");
    // Une décision déjà prise ne se rejoue pas.
    assert!(c.ent.voyage_desaccord_decider(&c.dir_a(), s.id, p1, "encore").await.is_err());
    // Le motif de la direction n'est jamais montré à l'organisateur.
    let texte = serde_json::to_string(&m).unwrap();
    assert!(!texte.contains("Accord présumé"));
}

#[tokio::test]
async fn r_sv6_derogation_non_legale_motivee_direction_seule_et_jamais_un_consentement() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage").await;
    c.campagne(s.id, true, true).await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    // Droits : direction seule (`voyages.autoriser`) ; ni l'organisateur, ni le secrétariat, ni l'accueil.
    for (a, ok) in [(c.prof_a(), false), (c.a(c.sec, Role::Administratif), false), (c.a(c.compta, Role::Compta), false), (c.dir_a(), true)] {
        let r = c.ent.voyage_exigence_deroger(&a, s.id, p0, "autorisation_soins", "famille injoignable").await;
        assert_eq!(r.is_ok(), ok, "{r:?}");
    }
    assert!(est_introuvable(&c.ent.voyage_exigence_deroger(&c.a(c.accueil, Role::Accueil), s.id, p0, "droit_image", "x").await.unwrap_err()));
    // Légales : jamais (participation, désaccord) ; motif obligatoire ; code inconnu refusé.
    for code in ["autorisation_participation", "desaccord_titulaire", "assurance", "inconnu"] {
        assert!(c.ent.voyage_exigence_deroger(&c.dir_a(), s.id, p0, code, "motif").await.is_err(), "{code}");
    }
    assert!(c.ent.voyage_exigence_deroger(&c.dir_a(), s.id, p0, "droit_image", " ").await.is_err());
    // La dérogation arrête la relance de cette exigence mais ne vaut JAMAIS consentement à l'image :
    // une photo où l'élève est marqué reste masquée pour les familles (R-SV40).
    c.ent.voyage_exigence_deroger(&c.dir_a(), s.id, p0, "droit_image", "Famille injoignable").await.unwrap();
    assert_eq!(c.cellule(s.id, &c.dir_a(), c.eleves[0], "droit_image").await.as_deref(), Some("derogee"));
    c.ent.voyage_journal_activer(&c.prof_a(), s.id).await.unwrap();
    c.ent.voyage_journal_publier(&c.prof_a(), s.id, message("Départ", vec![photo(vec![c.eleves[0]])], false)).await.unwrap();
    let vue = c.ent.voyages_journal_famille(&c.a(c.parent, Role::Parent)).await.unwrap();
    assert_eq!(vue[0].publications[0].photos.len(), 0, "dérogation ≠ consentement : photo masquée");
    // L'organisateur voit l'état « dérogée », jamais le motif.
    let m = c.ent.voyage_exigences_matrice(&c.prof_a(), s.id).await.unwrap();
    assert!(!serde_json::to_string(&m).unwrap().contains("injoignable"));
    // Un refus ne se déroge pas : le titulaire refuse les soins, la dérogation est refusée.
    let (_, tit) = c.titulaire_de(c.eleves[1]).await;
    let p1 = c.participant(s.id, c.eleves[1]).await;
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p1, "autorisation_soins", "refuse", tit)).await.unwrap();
    assert!(est_refus(&c.ent.voyage_exigence_deroger(&c.dir_a(), s.id, p1, "autorisation_soins", "on passe outre").await.unwrap_err()));
}

// ═════════════════════════════ Qui manque quoi et relances ═════════════════════════════

#[tokio::test]
async fn cv_09_relance_groupee_destinataires_calcules_et_texte_fixe() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage Rome").await;
    let p0 = c.participant(s.id, c.eleves[0]).await;
    let (u1, t1) = c.titulaire_de(c.eleves[1]).await;
    let (u2, t2) = c.titulaire_de(c.eleves[2]).await;
    let sans_compte = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1,$2,'Sans Compte','saisie')").bind(sans_compte).bind(c.etab).execute(c.ent.pool()).await.unwrap();
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id) VALUES ($1,$2,$3,NULL,$4)").bind(Uuid::new_v4()).bind(c.etab).bind(c.eleves[1]).bind(sans_compte).execute(c.ent.pool()).await.unwrap();
    // Sans campagne : pas de relance ; aperçu sans exigence.
    assert!(est_refus(&c.ent.voyage_relancer_manquants(&c.prof_a(), s.id, true).await.unwrap_err()));
    c.campagne(s.id, false, false).await;
    // Un titulaire privé de communication (interdiction) n'est PAS relancé ; celui qui a déjà refusé non plus.
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4,ARRAY['communication'],'decision_etablissement','Aucun contact', ent_aujourdhui_etab($2) - 1, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(c.etab)
    .bind(c.eleves[2])
    .bind(t2)
    .bind(c.dir)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let ap = c.ent.voyage_relance_apercu(&c.prof_a(), s.id).await.unwrap();
    // Élève 0 (parent avec compte) + élève 1 (titulaire avec compte et titulaire sans compte) ; l'élève 2 n'a plus
    // de titulaire sollicitable.
    assert_eq!((ap.destinataires_comptes, ap.destinataires_sans_compte), (2, 1), "{ap:?}");
    // Confirmation obligatoire ; droits comme la matrice.
    assert!(c.ent.voyage_relancer_manquants(&c.prof_a(), s.id, false).await.is_err());
    assert!(est_introuvable(&c.ent.voyage_relancer_manquants(&c.a(c.accueil, Role::Accueil), s.id, true).await.unwrap_err()));
    // Le titulaire de l'élève 1 signe : plus relancé. Celui de l'élève 0 le reste.
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(c.participant(s.id, c.eleves[1]).await, "autorisation_participation", "accepte", t1)).await.unwrap();
    let r = c.ent.voyage_relancer_manquants(&c.prof_a(), s.id, true).await.unwrap();
    assert_eq!(r.destinataires_comptes, 1);
    let notifs: Vec<(Uuid, String, String)> = sqlx::query_as("SELECT destinataire_user_id, titre, corps FROM ent_notification WHERE kind = 'voyages_autorisation_relance' AND etablissement_id = $1")
        .bind(c.etab)
        .fetch_all(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(notifs.len(), 1);
    assert_eq!(notifs[0].0, c.parent, "seul le titulaire de l'élève 0 est relancé");
    assert!(!notifs[0].2.contains("Prénom") && !notifs[0].2.contains("Alpha"), "aucun nom d'élève dans l'avis : {}", notifs[0].2);
    assert!(![u1, u2].contains(&notifs[0].0));
    // Pas deux relances manuelles en moins de douze heures.
    assert!(est_refus(&c.ent.voyage_relancer_manquants(&c.prof_a(), s.id, true).await.unwrap_err()));
    // Un titulaire qui REFUSE n'est plus relancé (il a répondu) : le participant n'est plus « concerné ».
    c.ent.voyage_autorisation_guichet(&c.prof_a(), s.id, c.guichet(p0, "autorisation_participation", "refuse", c.personne(c.parent).await)).await.unwrap();
    let ap = c.ent.voyage_relance_apercu(&c.prof_a(), s.id).await.unwrap();
    assert_eq!(ap.participants_concernes, 1, "l'élève 2 seul reste à relancer (mais sans titulaire sollicitable)");
    let m = c.ent.voyage_exigences_matrice(&c.prof_a(), s.id).await.unwrap();
    assert_eq!((m.bloques, m.prets), (1, 1));
}

#[tokio::test]
async fn relances_automatiques_j21_j10_j5_idempotentes_et_rattrapees_sans_rafale() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage").await;
    c.campagne(s.id, false, false).await;
    // La sortie est dans trois jours (cadre de test) : on se réveille après J-21, J-10 et J-5.
    assert_eq!(c.ent.voyages_relances_traiter(c.etab).await.unwrap(), 1, "un compte (le parent de l'élève 0), une seule relance");
    let lignes: Vec<(String, bool)> = sqlx::query_as("SELECT seuil, omise FROM ent_sortie_relance WHERE sortie_id = $1 ORDER BY seuil").bind(s.id).fetch_all(c.ent.pool()).await.unwrap();
    assert_eq!(lignes, vec![("J-10".into(), true), ("J-21".into(), true), ("J-5".into(), false)], "J-21 et J-10 omises, J-5 envoyée");
    // Idempotent : rejouer le même jour n'envoie rien de plus.
    assert_eq!(c.ent.voyages_relances_traiter(c.etab).await.unwrap(), 0);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_notification WHERE kind = 'voyages_autorisation_relance' AND etablissement_id = $1").bind(c.etab).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    // Les relances sont en ajout seul (base).
    assert!(sqlx::query("DELETE FROM ent_sortie_relance").execute(c.ent.pool()).await.is_err());
    // Une sortie lointaine n'est pas relancée ; une campagne close non plus.
    let loin = c.facultative("Voyage lointain").await;
    c.campagne(loin.id, false, false).await;
    sqlx::query("UPDATE ent_sortie SET debut = debut + interval '60 days', fin = fin + interval '60 days' WHERE id = $1").bind(loin.id).execute(c.ent.pool()).await.unwrap();
    assert_eq!(c.ent.voyages_relances_traiter(c.etab).await.unwrap(), 0);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_relance WHERE sortie_id = $1").bind(loin.id).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
}

// ═════════════════════════════ Attestation de reprise ═════════════════════════════

#[tokio::test]
async fn attestation_en_masse_des_autorisations_papier_d_un_sejour_repris() {
    let Some(c) = cadre().await else { return };
    let adm = admin(&c).await;
    // Séjour hors reprise : l'attestation en masse est refusée.
    let normale = c.facultative("Voyage ScolIA").await;
    c.campagne(normale.id, false, false).await;
    let att = |l: Vec<AttestationLigne>, confirme: bool| AttestationSaisie { lignes: l, signee_le: c.jour_coupon(10), confirme };
    assert!(est_refus(&c.ent.voyage_attestations_masse(&c.prof_a(), normale.id, att(vec![AttestationLigne { participant_id: c.participant(normale.id, c.eleves[0]).await, signataire_personne_id: c.personne(c.parent).await }], true)).await.unwrap_err()));
    // Séjour repris de Next.js, basculé : participants rapprochés (élèves 0 et 1).
    let id = pret_a_basculer(&c, &adm, "NX-V1", "FINALISE_DIR_ATTENTE_ELEVES", 20).await;
    let sortie = c.ent.voyages_reprise_basculer(&adm, id, true).await.unwrap();
    c.ent.voyage_campagne_ouvrir(&c.dir_a(), sortie, CampagneSaisie { avec_image: false, avec_soins: false }).await.unwrap();
    // Next.js n'a jamais recueilli d'autorisation : aucune présomption ; sans attestation, l'exigence reste attendue.
    assert_eq!(c.ent.voyages_reprise_tableau(&adm).await.unwrap().iter().find(|l| l.source_id == "NX-V1").unwrap().attestations, Some(0));
    assert_eq!(c.etat_participant(sortie, c.eleves[0]).await, "inscrit");
    let l = vec![
        AttestationLigne { participant_id: c.participant(sortie, c.eleves[0]).await, signataire_personne_id: c.personne(c.parent).await },
    ];
    // Confirmation obligatoire ; signataire recevable ; rien n'est écrit en cas d'erreur (tout ou rien).
    assert!(est_refus(&c.ent.voyage_attestations_masse(&c.dir_a(), sortie, att(l.clone(), false)).await.unwrap_err()));
    let mauvaise = vec![l[0].clone(), AttestationLigne { participant_id: c.participant(sortie, c.eleves[1]).await, signataire_personne_id: c.personne(c.parent).await }];
    assert!(c.ent.voyage_attestations_masse(&c.dir_a(), sortie, att(mauvaise, true)).await.is_err());
    assert_eq!(c.etat_participant(sortie, c.eleves[0]).await, "inscrit", "tout ou rien");
    c.ent.voyage_attestations_masse(&c.dir_a(), sortie, att(l, true)).await.unwrap();
    assert_eq!(c.etat_participant(sortie, c.eleves[0]).await, "pret");
    let t = c.ent.voyages_reprise_tableau(&adm).await.unwrap();
    assert_eq!(t.iter().find(|l| l.source_id == "NX-V1").unwrap().attestations, Some(1));
    let (src, jour): (String, NaiveDate) = sqlx::query_as(
        "SELECT x.source, x.signee_le FROM ent_sortie_exigence x WHERE x.sortie_id = $1 AND x.code = 'autorisation_participation' AND x.signee_le IS NOT NULL",
    )
    .bind(sortie)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((src.as_str(), jour), ("attestation_reprise", c.jour_coupon(10)));
}

// ═════════════════════════════ Ordres de mission ═════════════════════════════

#[tokio::test]
async fn ordres_de_mission_et_autorisations_des_adultes_direction_seule_et_remis_a_l_adulte() {
    let Some(c) = cadre().await else { return };
    let brouillon = c.creer("Brouillon", "sans_nuitee").await;
    let s = c.facultative("Voyage Rome").await;
    let prof3 = compte(&c.ent, c.etab, "prof").await;
    c.ent.voyage_encadrant_ajouter(&c.prof_a(), s.id, agent_de(&c.ent, c.etab, prof3).await, "encadrant").await.unwrap();
    // Direction seule ; l'organisateur (détail) est interdit, l'accueil et les autres établissements : introuvable.
    assert!(est_forbidden(&c.ent.voyage_ordres_mission_emettre(&c.prof_a(), s.id, None).await.unwrap_err()));
    assert!(est_introuvable(&c.ent.voyage_ordres_mission_emettre(&c.a(c.accueil, Role::Accueil), s.id, None).await.unwrap_err()));
    // Pas avant l'autorisation de la sortie.
    assert!(est_refus(&c.ent.voyage_ordres_mission_emettre(&c.dir_a(), brouillon.id, None).await.unwrap_err()));
    let docs = c.ent.voyage_ordres_mission_emettre(&c.dir_a(), s.id, Some("Frais réels sur justificatifs")).await.unwrap();
    assert_eq!(docs.len(), 4, "un ordre de mission et une autorisation écrite pour chacun des deux adultes");
    assert_eq!(docs.iter().filter(|d| d.type_ == "ordre_mission").count(), 2);
    // Contenu : dates, destination, prise en charge ; aucune donnée d'élève.
    let (_, octets) = c.ent.voyage_ordre_mission_lire(&c.dir_a(), s.id, docs.iter().find(|d| d.type_ == "ordre_mission").unwrap().id).await.unwrap();
    assert!(octets.starts_with(b"%PDF"));
    let t = pdf_texte(&octets);
    assert!(t.contains("Ordre de mission") && t.contains("Frais réels sur justificatifs") && t.contains(&s.reference), "{t}");
    assert!(t.contains("Destination") && t.contains("Prise en charge"), "{t}");
    assert!(!t.contains("Alpha") && !t.contains("Charlie"), "aucun élève dans un ordre de mission");
    // Chaque adulte voit et lit les siens seulement ; l'autre adulte, le porteur non encadrant, la famille : rien.
    let moi = c.a(c.prof2, Role::Prof);
    let siens = c.ent.voyage_ordres_mission_lister(&moi, s.id).await.unwrap();
    assert_eq!(siens.len(), 2);
    let adulte2 = c.personne(c.prof2).await;
    assert!(siens.iter().all(|d| d.personne_id == adulte2));
    assert!(c.ent.voyage_ordre_mission_lire(&moi, s.id, siens[0].id).await.is_ok());
    let de_l_autre = docs.iter().find(|d| d.personne_id != adulte2).unwrap();
    assert!(est_introuvable(&c.ent.voyage_ordre_mission_lire(&moi, s.id, de_l_autre.id).await.unwrap_err()));
    assert!(est_introuvable(&c.ent.voyage_ordre_mission_lire(&c.a(c.parent, Role::Parent), s.id, de_l_autre.id).await.unwrap_err()));
    assert_eq!(c.ent.voyage_ordres_mission_lister(&c.dir_a(), s.id).await.unwrap().len(), 4);
    // Un document d'adulte ne sort jamais par l'identifiant d'un document d'une autre sortie.
    assert!(est_introuvable(&c.ent.voyage_ordre_mission_lire(&c.dir_a(), brouillon.id, de_l_autre.id).await.unwrap_err()));
    // Réémission : de nouveaux documents s'ajoutent (ajout seul), les précédents restent.
    c.ent.voyage_ordres_mission_emettre(&c.dir_a(), s.id, None).await.unwrap();
    assert_eq!(c.ent.voyage_ordres_mission_lister(&c.dir_a(), s.id).await.unwrap().len(), 8);
    assert!(actions_journal(&c, s.id).await.contains(&"ordres_mission_emis".to_string()));
}

// ═════════════════════════════ Liste nominative à un prestataire (VD-16) ═════════════════════════════

#[tokio::test]
async fn vd_16_liste_nominative_champs_fermes_validation_direction_aucun_contact_de_famille() {
    let Some(c) = cadre().await else { return };
    let s = c.facultative("Voyage Londres").await;
    let adresse = format!("contact-{}@cars-test.fr", Uuid::new_v4().simple());
    let presta = c
        .ent
        .prestataire_creer(
            &c.a(c.sec, Role::Administratif),
            PrestataireSaisie {
                raison_sociale: format!("Cars {}", Uuid::new_v4().simple()),
                siren: None,
                telephone: None,
                categories: vec!["transporteur".into()],
                adresses: vec![adresse.clone()],
                version: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    maj_identite(&c, c.eleves[0], "ine = '12345678AB1', date_naissance = '2012-04-01'").await;
    let email_parent: String = sqlx::query_scalar("SELECT email FROM ent_user WHERE id = $1").bind(c.parent).fetch_one(c.ent.pool()).await.unwrap();
    let saisie = |champs: &[&str], pays: &str, hors: bool| TransmissionSaisie {
        prestataire_id: presta.id,
        champs: champs.iter().map(|x| x.to_string()).collect(),
        motif: "Liste exigée par le contrat de transport".into(),
        pays_destinataire: pays.into(),
        confirme_hors_ue: hors,
    };
    // Champs : liste FERMÉE. Jamais d'INE, de contact de famille, de santé ; jamais vide.
    for mauvais in [vec!["ine"], vec!["nom", "email_parent"], vec!["telephone"], vec!["telephone_parent"], vec!["pai"], vec![]] {
        let e = c.ent.voyage_transmission_demander(&c.prof_a(), s.id, saisie(&mauvais, "France", false)).await.unwrap_err();
        assert!(est_refus(&e), "{mauvais:?} : {e:?}");
    }
    // La base refuse aussi, en direct, tout champ hors liste (garde-fou indépendant du code).
    let t = c.ent.voyage_transmission_demander(&c.prof_a(), s.id, saisie(&["nom", "prenom", "classe"], "France", false)).await.unwrap();
    assert_eq!((t.statut.as_str(), t.hors_ue, t.nb_lignes), ("a_valider", false, 3));
    for interdit in ["ine", "email", "telephone", "adresse", "sante"] {
        let r = sqlx::query("INSERT INTO ent_sortie_transmission_champ (id, etablissement_id, transmission_id, champ) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::new_v4())
            .bind(c.etab)
            .bind(t.id)
            .bind(interdit)
            .execute(c.ent.pool())
            .await;
        assert!(r.is_err(), "{interdit}");
    }
    // Destinataire hors UE : signalé, confirmation expresse (VOY-289) ; pays et motif obligatoires.
    assert!(est_refus(&c.ent.voyage_transmission_demander(&c.prof_a(), s.id, saisie(&["nom"], "Royaume-Uni", false)).await.unwrap_err()));
    let hors = c.ent.voyage_transmission_demander(&c.prof_a(), s.id, saisie(&["nom", "prenom"], "Royaume-Uni", true)).await.unwrap();
    assert!(hors.hors_ue);
    let mut sans_pays = saisie(&["nom"], "France", false);
    sans_pays.pays_destinataire = " ".into();
    assert!(c.ent.voyage_transmission_demander(&c.prof_a(), s.id, sans_pays).await.is_err());
    let mut sans_motif = saisie(&["nom"], "France", false);
    sans_motif.motif = "".into();
    assert!(c.ent.voyage_transmission_demander(&c.prof_a(), s.id, sans_motif).await.is_err());
    // Droits : introuvable pour l'accueil et la famille ; un prestataire d'un autre établissement n'existe pas.
    assert!(est_introuvable(&c.ent.voyage_transmission_demander(&c.a(c.accueil, Role::Accueil), s.id, saisie(&["nom"], "France", false)).await.unwrap_err()));
    let autre = cadre().await.unwrap();
    let e = autre.ent.voyage_transmission_demander(&autre.a(autre.dir, Role::Direction), s.id, saisie(&["nom"], "France", false)).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Décision : la DIRECTION seule ; refus motivé sans envoi.
    assert!(est_forbidden(&c.ent.voyage_transmission_decider(&c.prof_a(), s.id, t.id, true, None).await.unwrap_err()));
    let refusee = c.ent.voyage_transmission_decider(&c.dir_a(), s.id, hors.id, false, Some("Destinataire non couvert par le contrat")).await.unwrap();
    assert_eq!(refusee.statut, "refusee");
    assert!(c.ent.voyage_transmission_decider(&c.dir_a(), s.id, hors.id, true, None).await.is_err(), "déjà décidée");
    assert_eq!(recus_vers(&c, &adresse), 0);
    // Un participant désisté n'est pas dans la liste.
    let p2 = c.participant(s.id, c.eleves[2]).await;
    c.ent.voyage_participant_desister(&c.prof_a(), s.id, p2, "famille", None).await.unwrap();
    // Validation : liste composée côté serveur, envoyée à l'adresse DE L'ANNUAIRE, sans contact ni INE.
    let ok = c.ent.voyage_transmission_decider(&c.dir_a(), s.id, t.id, true, None).await.unwrap();
    assert_eq!(ok.statut, "validee");
    assert_eq!(recus_vers(&c, &adresse), 1);
    let cle: String = sqlx::query_scalar("SELECT cle_objet FROM ent_sortie_document WHERE sortie_id = $1 AND type = 'liste_nominative'").bind(s.id).fetch_one(c.ent.pool()).await.unwrap();
    let octets = c.ent.lire_objet(c.etab, &cle).await.unwrap();
    assert!(octets.starts_with(b"%PDF"));
    let liste = pdf_texte(&octets);
    assert!(liste.contains("Alpha") && liste.contains("Bravo"), "les participants qui partent figurent dans la liste : {liste}");
    assert!(!liste.contains("Charlie"), "le désisté n'est pas transmis");
    for interdit in ["12345678AB1", email_parent.as_str(), "@ent.local", "01/04/2012", "INE"] {
        assert!(!liste.contains(interdit) && !octets.windows(interdit.len()).any(|w| w == interdit.as_bytes()), "la liste ne doit pas contenir « {interdit} » (VD-16)");
    }
    // Le message accompagne le document sans nom d'élève ; la trace dit qui, quand, quels champs, quel pays.
    let corps: String = sqlx::query_scalar("SELECT corps FROM ent_sortie_envoi WHERE sortie_id = $1 AND type = 'liste_nominative'").bind(s.id).fetch_one(c.ent.pool()).await.unwrap();
    assert!(!corps.contains("Alpha") && corps.contains("confidentiel"));
    let (champs, pays, decide): (Vec<String>, String, Option<Uuid>) = sqlx::query_as(
        "SELECT (SELECT array_agg(champ ORDER BY champ) FROM ent_sortie_transmission_champ WHERE transmission_id = x.id), x.pays_destinataire, x.decide_par_user_id
           FROM ent_sortie_transmission x WHERE x.id = $1",
    )
    .bind(t.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((champs, pays.as_str(), decide), (vec!["classe".into(), "nom".into(), "prenom".into()], "France", Some(c.dir)));
    assert!(actions_journal(&c, s.id).await.contains(&"liste_nominative_validee".to_string()));
    // Lecture des transmissions : organisateur et direction seulement.
    assert_eq!(c.ent.voyage_transmissions_lire(&c.dir_a(), s.id).await.unwrap().len(), 2);
    assert!(est_introuvable(&c.ent.voyage_transmissions_lire(&c.a(c.parent, Role::Parent), s.id).await.unwrap_err()));
}

// ═════════════════════════════ Séparation entre établissements ═════════════════════════════

#[tokio::test]
async fn exigences_et_transmissions_cloisonnees_entre_etablissements() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let s = a.facultative("Voyage A").await;
    a.campagne(s.id, true, true).await;
    let p0 = a.participant(s.id, a.eleves[0]).await;
    for acteur in [b.a(b.dir, Role::Direction), b.a(b.prof, Role::Prof), b.a(b.sec, Role::Administratif)] {
        assert!(est_introuvable(&a.ent.voyage_exigences_matrice(&acteur, s.id).await.unwrap_err()));
        assert!(a.ent.voyage_autorisation_guichet(&acteur, s.id, a.guichet(p0, "autorisation_participation", "accepte", a.personne(a.parent).await)).await.is_err());
        assert!(a.ent.voyage_exigence_deroger(&acteur, s.id, p0, "droit_image", "x").await.is_err());
        assert!(a.ent.voyage_desaccord_decider(&acteur, s.id, p0, "x").await.is_err());
        assert!(a.ent.voyage_ordres_mission_emettre(&acteur, s.id, None).await.is_err());
        assert!(a.ent.voyage_relancer_manquants(&acteur, s.id, true).await.is_err());
        assert!(a.ent.voyage_transmissions_lire(&acteur, s.id).await.is_err());
    }
    // Le port d'entrée ne traverse pas non plus : la sortie de A n'existe pas pour B.
    let e = b.ent.voyages_autorisation_decision_recue(b.etab, DecisionRecue {
        sortie_id: s.id,
        participant_id: p0,
        code: "droit_image".into(),
        decision: "accepte".into(),
        signataire_personne_id: a.personne(a.parent).await,
        signee_le: a.jour - Duration::days(3),
        reference_externe: None,
    }).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Les relances automatiques de B ne touchent jamais la campagne de A.
    assert_eq!(b.ent.voyages_relances_traiter(b.etab).await.unwrap(), 0);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_relance WHERE sortie_id = $1").bind(s.id).fetch_one(a.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_tables_des_exigences_et_des_transmissions() {
    crate::tests_rls_lots::verifier_tables(&[
        "ent_sortie_campagne",
        "ent_sortie_exigence",
        "ent_sortie_exigence_fait",
        "ent_sortie_relance",
        "ent_sortie_transmission",
        "ent_sortie_transmission_champ",
    ])
    .await;
}

#[tokio::test]
async fn schema_v1_sans_json_ni_bytea_et_avec_cles_composites() {
    let Some(c) = cadre().await else { return };
    let tables: Vec<String> = ["ent_sortie_campagne", "ent_sortie_exigence", "ent_sortie_exigence_fait", "ent_sortie_relance", "ent_sortie_transmission", "ent_sortie_transmission_champ"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let interdits: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name = ANY($1) AND data_type IN ('json', 'jsonb', 'bytea', 'ARRAY')",
    )
    .bind(&tables)
    .fetch_all(c.ent.pool())
    .await
    .unwrap();
    assert!(interdits.is_empty(), "{interdits:?}");
    for t in &tables[..5] {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_constraint WHERE conrelid = ('public.' || $1)::regclass AND contype = 'u' AND pg_get_constraintdef(oid) LIKE '%etablissement_id, id%'")
            .bind(t)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
        assert!(n >= 1, "{t} : clé composite absente");
    }
}
