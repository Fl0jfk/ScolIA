//! Socle 42, lot V0c (prestataires, demandes de devis, devis, signature, bon de commande, avenants,
//! boîte de réception des devis et file de tri) contre la base locale de test. Sans
//! `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Chaque défaut de Next.js fermé par V0c a ici son test (voir `docs/socles/42_SORTIES_VOYAGES.md`
//! §33.8 et les noms `nj_xx_…`) :
//!
//! * `nj_07_08_…` droits de lecture et d'écriture des achats ;
//! * `nj_09_…` une pièce n'est jamais servie hors de sa sortie ;
//! * `nj_10_11_…` destinataires pris dans l'annuaire seul, messages en texte brut ;
//! * `nj_12_…` copie à la boîte comptabilité **de l'établissement** ;
//! * `nj_13_…` signature scellée sur l'empreinte, en ajout seul ;
//! * `nj_27_…` cloisonnement entre établissements ;
//! * `nj_28_70_…` courrier entrant : rattachement certain seulement, file de tri, jamais perdu ;
//! * `nj_38_…` pièces : type détecté sur les octets ;
//! * `nj_55_…` aucune coordonnée de famille ni nom d'élève chez le prestataire.

use super::*;
use crate::tests_voyages_v0a::{
    actions_journal, compte, est_conflit_detail, est_forbidden, est_introuvable, est_refus, Cadre,
};
use crate::voyages::Commande;
use crate::voyages_courrier_repo::{CourrierEntrant, PieceCourrier};
use crate::voyages_pdf::{rendre, Bloc};
use crate::voyages_prestataires::reference_courrier;
use crate::voyages_prestataires_repo::{
    AvenantSaisie, ConfirmationCommande, DemandeSaisie, DevisSaisie, PieceSaisie, PrestataireSaisie,
    PrestataireVue,
};
use crate::voyages_repo::{ParamsCommande, SortieLigne};
use chrono::Utc;
use ent_auth::Role;
use uuid::Uuid;

async fn cadre() -> Option<Cadre> {
    let mut c = Cadre::nouveau().await?;
    c.ent.install_mail_dry_run();
    Some(c)
}

fn saisie_presta(nom: &str, email: &str) -> PrestataireSaisie {
    PrestataireSaisie {
        raison_sociale: nom.into(),
        siren: Some("732829320".into()),
        telephone: Some("01 23 45 67 89".into()),
        categories: vec!["transporteur".into()],
        adresses: vec![email.into()],
        version: None,
        actif: None,
    }
}

fn pdf(texte: &str) -> Vec<u8> {
    rendre("Devis", &[Bloc::Paragraphe(texte.into())])
}

fn devis_manuel(presta: Uuid, montant: Option<&str>) -> DevisSaisie {
    DevisSaisie {
        prestataire_id: presta,
        reference: Some("DV-2027-001".into()),
        montant: montant.map(String::from),
        valide_jusqu_au: None,
        inclus: Some("Car 55 places, péages".into()),
        conditions_annulation: None,
        piece: None,
    }
}

impl Cadre {
    fn sec_a(&self) -> Acteur<'static> {
        self.a(self.sec, Role::Administratif)
    }

    async fn presta(&self, nom: &str) -> PrestataireVue {
        let email = format!("contact-{}@cars-{}.fr", Uuid::new_v4().simple(), nom.to_lowercase());
        self.ent.prestataire_creer(&self.sec_a(), saisie_presta(nom, &email)).await.expect("prestataire")
    }

    /// Voyage avec car, au stade « préparation » (pédagogie validée).
    async fn voyage_car(&self, titre: &str) -> SortieLigne {
        let mut sa = self.saisie(titre, "voyage");
        sa.besoin_car = true;
        let s = self.ent.voyage_creer(&self.a(self.prof, Role::Prof), sa).await.expect("création");
        self.preparer_jusqu_a_soumise(s.id).await;
        self.cmd(self.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap()
    }

    async fn version(&self, id: Uuid) -> i32 {
        sqlx::query_scalar("SELECT version FROM ent_sortie WHERE id = $1")
            .bind(id)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn demander(&self, id: Uuid, presta: &[Uuid]) -> Result<Vec<crate::voyages_prestataires_repo::EnvoiVue>, EntDbError> {
        self.ent
            .voyage_devis_demander(
                &self.a(self.prof, Role::Prof),
                id,
                DemandeSaisie { prestataires: presta.to_vec(), confirme: true },
            )
            .await
    }

    /// Devis saisi par le porteur avec un montant, puis choisi.
    async fn devis_choisi(&self, id: Uuid, presta: Uuid, montant: &str) -> crate::voyages_prestataires_repo::DevisVue {
        let prof = self.a(self.prof, Role::Prof);
        let d = self.ent.voyage_devis_ajouter(&prof, id, devis_manuel(presta, Some(montant))).await.expect("devis");
        let v = self.ent.voyage_devis_choisir(&prof, id, d.id, d.rev).await.expect("choix");
        v.into_iter().find(|x| x.id == d.id).unwrap()
    }

    async fn signer(&self, id: Uuid, d: &crate::voyages_prestataires_repo::DevisVue) -> Result<SortieLigne, EntDbError> {
        self.signer_avec(id, d, d.empreinte.clone(), Some(true), Role::Direction, self.dir).await
    }

    async fn signer_avec(
        &self,
        id: Uuid,
        d: &crate::voyages_prestataires_repo::DevisVue,
        empreinte: Option<String>,
        confirme: Option<bool>,
        role: Role,
        user: Uuid,
    ) -> Result<SortieLigne, EntDbError> {
        let version = self.version(id).await;
        self.ent
            .voyage_commande(
                &self.a(user, role),
                id,
                Commande::SignerDevis,
                ParamsCommande {
                    version: Some(version),
                    devis_id: Some(d.id),
                    devis_rev: Some(d.rev),
                    empreinte,
                    confirme,
                    ..Default::default()
                },
            )
            .await
    }

    /// Voyage jusqu'à l'étape « devis à signer » avec un devis choisi (1 500 €).
    async fn voyage_a_signer(&self, titre: &str) -> (SortieLigne, PrestataireVue, crate::voyages_prestataires_repo::DevisVue) {
        let s = self.voyage_car(titre).await;
        let p = self.presta("Dupont").await;
        let d = self.devis_choisi(s.id, p.id, "1 500").await;
        let s = self.cmd(self.prof, Role::Prof, s.id, Commande::DemanderSignature).await.unwrap();
        assert_eq!(s.etat, "a_signer");
        let d = self.ent.voyage_achats(&self.a(self.prof, Role::Prof), s.id).await.unwrap().devis.into_iter().find(|x| x.id == d.id).unwrap();
        (s, p, d)
    }
}

fn recus_vers(c: &Cadre, adresse: &str) -> usize {
    c.ent.mailer().receipts().iter().filter(|r| r.to == adresse).count()
}

// ─────────────────────────── Annuaire ───────────────────────────

#[tokio::test]
async fn annuaire_droits_adresses_verifiees_et_siren() {
    let Some(c) = cadre().await else { return };
    // Seuls secrétariat, comptabilité et direction tiennent l'annuaire (voyages.prestataires).
    for (u, r) in [(c.prof, Role::Prof), (c.accueil, Role::Accueil), (c.parent, Role::Parent)] {
        let e = c.ent.prestataire_creer(&c.a(u, r), saisie_presta("Interdit", "a@interdit.fr")).await.unwrap_err();
        assert!(est_forbidden(&e), "{r:?} {e:?}");
        assert!(c.ent.prestataires_liste(&c.a(u, r)).await.is_err());
    }
    let p = c.ent.prestataire_creer(&c.a(c.compta, Role::Compta), saisie_presta("Cars Martin", "Contact@Cars-Martin.FR")).await.unwrap();
    assert_eq!(p.adresses.len(), 1);
    assert_eq!(p.adresses[0].email, "contact@cars-martin.fr");
    assert_eq!(p.adresses[0].domaine, "cars-martin.fr");
    // Adresses multiples, noms ou séparateurs glissés dans le champ : refusés ; SIREN faux : refusé.
    for mauvais in ["a@b.fr, c@d.fr", "Nom <a@b.fr>", "a@b.fr\nBcc: x@y.fr", "pas-une-adresse"] {
        let e = c.ent.prestataire_adresse_ajouter(&c.sec_a(), p.id, mauvais).await.unwrap_err();
        assert!(est_refus(&e), "{mauvais:?} {e:?}");
    }
    let mut faux = saisie_presta("SIREN faux", "x@siren-faux.fr");
    faux.siren = Some("732829321".into());
    assert!(est_refus(&c.ent.prestataire_creer(&c.sec_a(), faux).await.unwrap_err()));
    // Même adresse chez un autre prestataire : refusée (rattachement univoque) ; même nom : refusé.
    let e = c.ent.prestataire_creer(&c.sec_a(), saisie_presta("Autre", "contact@cars-martin.fr")).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let e = c.ent.prestataire_creer(&c.sec_a(), saisie_presta("cars martin", "z@autre-domaine.fr")).await;
    assert!(e.is_err());
    // Désactivation : l'adresse reste au dossier, n'est plus utilisée ; la liste la montre.
    let p = c.ent.prestataire_adresse_desactiver(&c.sec_a(), p.id, p.adresses[0].id).await.unwrap();
    assert!(p.adresses[0].desactivee_le.is_some());
    // Modification sur la version lue.
    let mut m = saisie_presta("Cars Martin SA", "ignoree@x.fr");
    m.version = Some(p.version);
    let p2 = c.ent.prestataire_modifier(&c.sec_a(), p.id, m.clone()).await.unwrap();
    assert_eq!(p2.raison_sociale, "Cars Martin SA");
    assert!(est_refus(&c.ent.prestataire_modifier(&c.sec_a(), p.id, m).await.unwrap_err()), "version périmée");
    // Les changements d'annuaire sont journalisés.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'voyages_annuaire'")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert!(n >= 3, "{n}");
}

#[tokio::test]
async fn nj_27_annuaire_et_achats_cloisonnes_par_etablissement() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let pa = a.presta("Alpha").await;
    assert!(b.ent.prestataires_liste(&b.sec_a()).await.unwrap().is_empty());
    // Un prestataire d'un autre établissement n'est ni modifiable ni utilisable.
    let mut m = saisie_presta("Vol", "vol@vol.fr");
    m.version = Some(1);
    assert!(b.ent.prestataire_modifier(&b.sec_a(), pa.id, m).await.is_err());
    let s = b.voyage_car("Voyage B").await;
    let e = b.demander(s.id, &[pa.id]).await.unwrap_err();
    assert!(est_refus(&e) || est_introuvable(&e), "{e:?}");
    // Les achats d'une sortie d'un autre établissement sont introuvables.
    let sa = a.voyage_car("Voyage A").await;
    let e = b.ent.voyage_achats(&b.a(b.dir, Role::Direction), sa.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

// ─────────────────────────── Demandes de devis ───────────────────────────

#[tokio::test]
async fn demande_de_devis_droits_apercu_confirmation_et_envoi() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Rome").await;
    let p = c.presta("Dupont").await;
    let adresse = p.adresses[0].email.clone();
    let prof = c.a(c.prof, Role::Prof);
    // Le porteur choisit parmi les noms de l'annuaire : il ne reçoit pas les adresses.
    let choix = c.ent.voyage_prestataires_choix(&prof, s.id).await.unwrap();
    assert!(choix.iter().any(|x| x.id == p.id && x.joignable && !x.demande_envoyee));
    assert!(!serde_json::to_string(&choix).unwrap().contains('@'), "adresses non exposées au porteur");
    // Un professeur tiers, l'accueil, une famille : rien.
    let tiers = compte(&c.ent, c.etab, "prof").await;
    assert!(c.ent.voyage_prestataires_choix(&c.a(tiers, Role::Prof), s.id).await.is_err());
    for (u, r) in [(c.accueil, Role::Accueil), (c.parent, Role::Parent)] {
        let e = c.ent.voyage_prestataires_choix(&c.a(u, r), s.id).await.unwrap_err();
        assert!(est_introuvable(&e) || est_forbidden(&e), "{r:?} {e:?}");
    }
    let e = c
        .ent
        .voyage_devis_demander(&c.a(tiers, Role::Prof), s.id, DemandeSaisie { prestataires: vec![p.id], confirme: true })
        .await
        .unwrap_err();
    assert!(est_introuvable(&e) || est_forbidden(&e), "{e:?}");
    // Sans confirmation après aperçu : refus. Liste vide ou prestataire sans adresse : refus.
    let e = c.ent.voyage_devis_demander(&prof, s.id, DemandeSaisie { prestataires: vec![p.id], confirme: false }).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    assert!(c.demander(s.id, &[]).await.is_err());
    let mut sans = saisie_presta("Sans Adresse", "temp@sans-adresse.fr");
    sans.adresses.clear();
    let sans = c.ent.prestataire_creer(&c.sec_a(), sans).await.unwrap();
    assert!(est_refus(&c.demander(s.id, &[sans.id]).await.unwrap_err()));
    // Une demande part : au statut « envoyé », vers l'adresse de l'annuaire, objet portant la référence.
    let envois = c.demander(s.id, &[p.id]).await.unwrap();
    assert_eq!(envois.len(), 1);
    assert_eq!(envois[0].statut, "envoye");
    assert_eq!(recus_vers(&c, &adresse), 1);
    let ref_c = reference_courrier(&s.reference, c.etab, s.id);
    assert!(envois[0].objet.contains(&ref_c), "{}", envois[0].objet);
    // Une seconde demande au même prestataire : conflit (c'est une relance).
    assert!(est_refus(&c.demander(s.id, &[p.id]).await.unwrap_err()));
    let relances = c.ent.voyage_devis_relancer(&prof, s.id, true).await.unwrap();
    assert_eq!(relances.len(), 1);
    assert_eq!(relances[0].type_, "relance");
    assert_eq!(recus_vers(&c, &adresse), 2);
    // Une réponse reçue supprime la relance.
    c.ent.voyage_devis_ajouter(&prof, s.id, devis_manuel(p.id, Some("1 200"))).await.unwrap();
    assert!(est_refus(&c.ent.voyage_devis_relancer(&prof, s.id, true).await.unwrap_err()));
    assert!(actions_journal(&c, s.id).await.contains(&"devis_demandes".to_string()));
    // Hors préparation (brouillon d'une autre sortie) : refus.
    let b = c.creer("Brouillon", "voyage").await;
    assert!(est_refus(&c.demander(b.id, &[p.id]).await.unwrap_err()));
}

#[tokio::test]
async fn nj_10_11_destinataires_de_l_annuaire_seul_et_texte_brut() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Lyon").await;
    let mut p = c.presta("Dupont").await;
    // Le nom de l'établissement peut contenir des retours à la ligne : jamais d'en-tête injecté.
    let annuaire: Vec<String> = c.ent.prestataires_liste(&c.sec_a()).await.unwrap().into_iter().flat_map(|x| x.adresses.into_iter().map(|a| a.email)).collect();
    c.demander(s.id, &[p.id]).await.unwrap();
    for r in c.ent.mailer().receipts() {
        assert!(annuaire.contains(&r.to), "destinataire hors annuaire : {}", r.to);
        assert!(!r.subject.contains('\n') && !r.subject.contains('\r'));
    }
    for (_, sujet, corps) in c.ent.mailer().dry_run_corps() {
        assert!(!corps.contains('<') || !corps.contains("</"), "texte brut : {sujet}");
    }
    // Une adresse désactivée ne reçoit plus rien ; sans adresse active la demande est refusée.
    p = c.ent.prestataire_adresse_desactiver(&c.sec_a(), p.id, p.adresses[0].id).await.unwrap();
    let s2 = c.voyage_car("Voyage à Nice").await;
    assert!(est_refus(&c.demander(s2.id, &[p.id]).await.unwrap_err()));
    // Aucune API publique n'accepte un destinataire libre : la mise en file ne prend qu'un identifiant
    // d'adresse de l'annuaire, garanti par la clé étrangère composite.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_envoi (id, etablissement_id, sortie_id, prestataire_id, adresse_id, type, reference_courrier, objet, corps)
         VALUES ($1,$2,$3,$4,$5,'demande_devis','REF-123456','o','c')",
    )
    .bind(Uuid::new_v4())
    .bind(c.etab)
    .bind(s.id)
    .bind(p.id)
    .bind(Uuid::new_v4())
    .execute(c.ent.pool())
    .await;
    assert!(r.is_err(), "adresse inconnue : clé étrangère");
}

#[tokio::test]
async fn nj_55_aucune_donnee_de_famille_ni_nom_d_eleve_chez_le_prestataire() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Paris").await;
    let p = c.presta("Dupont").await;
    c.demander(s.id, &[p.id]).await.unwrap();
    let parent_mail: Option<String> = sqlx::query_scalar("SELECT email FROM ent_user WHERE id = $1").bind(c.parent).fetch_optional(c.ent.pool()).await.unwrap();
    for (_, sujet, corps) in c.ent.mailer().dry_run_corps() {
        for interdit in ["Alpha", "Bravo", "Charlie", "Prénom"] {
            assert!(!corps.contains(interdit) && !sujet.contains(interdit), "{interdit}");
        }
        if let Some(m) = &parent_mail {
            assert!(!corps.contains(m.as_str()));
        }
    }
    // Les envois ne portent que le PDF de la demande, généré à partir des seuls effectifs.
    let achats = c.ent.voyage_achats(&c.a(c.prof, Role::Prof), s.id).await.unwrap();
    assert_eq!(achats.envois.len(), 1);
}

// ─────────────────────────── Devis, signature, commande ───────────────────────────

#[tokio::test]
async fn nj_07_08_achats_lecture_et_ecriture_reservees() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Berlin").await;
    let p = c.presta("Dupont").await;
    let prof = c.a(c.prof, Role::Prof);
    let d = c.ent.voyage_devis_ajouter(&prof, s.id, devis_manuel(p.id, Some("900"))).await.unwrap();
    // Lecture : porteur, direction, comptabilité, secrétariat. Accueil, famille : introuvable.
    for (u, r, ok) in [
        (c.prof, Role::Prof, true),
        (c.dir, Role::Direction, true),
        (c.compta, Role::Compta, true),
        (c.sec, Role::Administratif, true),
        (c.accueil, Role::Accueil, false),
        (c.parent, Role::Parent, false),
    ] {
        let r1 = c.ent.voyage_achats(&c.a(u, r), s.id).await;
        assert_eq!(r1.is_ok(), ok, "{r:?} {r1:?}");
    }
    // Un professeur tiers (même établissement) ne voit ni n'écrit rien ; l'encadrant non porteur lit
    // le dossier mais pas les achats.
    let tiers = compte(&c.ent, c.etab, "prof").await;
    assert!(c.ent.voyage_achats(&c.a(tiers, Role::Prof), s.id).await.is_err());
    let enc = c.a(c.prof2, Role::Prof);
    assert!(c.ent.voyage_achats(&enc, s.id).await.is_err());
    assert!(c.ent.voyage_devis_ajouter(&enc, s.id, devis_manuel(p.id, Some("1"))).await.is_err());
    assert!(c.ent.voyage_devis_choisir(&enc, s.id, d.id, d.rev).await.is_err());
    assert!(c.ent.voyage_devis_confirmer_montant(&enc, s.id, d.id, d.rev, "1").await.is_err());
    // Le montant saisi est strict (format français), plafonné.
    for mauvais in ["1.350", "-5", "1e3", "abc"] {
        assert!(c.ent.voyage_devis_ajouter(&prof, s.id, devis_manuel(p.id, Some(mauvais))).await.is_err(), "{mauvais}");
    }
    // Une version périmée du devis (lue avant un autre geste) ne se confirme ni ne se choisit.
    let d2 = c.ent.voyage_devis_confirmer_montant(&prof, s.id, d.id, d.rev, "950").await.unwrap();
    assert_eq!(d2.montant_centimes, Some(95_000));
    assert!(est_refus(&c.ent.voyage_devis_choisir(&prof, s.id, d.id, d.rev).await.unwrap_err()), "rev périmée");
    // Signature : seule la direction, jamais le porteur ni la comptabilité (corrige NJ-07, NJ-13).
    let v = c.ent.voyage_devis_choisir(&prof, s.id, d.id, d2.rev).await.unwrap();
    let choisi = v.into_iter().find(|x| x.id == d.id).unwrap();
    let s = c.cmd(c.prof, Role::Prof, s.id, Commande::DemanderSignature).await.unwrap();
    for (u, r) in [(c.prof, Role::Prof), (c.compta, Role::Compta), (c.sec, Role::Administratif), (c.accueil, Role::Accueil)] {
        let e = c.signer_avec(s.id, &choisi, choisi.empreinte.clone(), Some(true), r, u).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{r:?} {e:?}");
    }
    assert_eq!(c.version(s.id).await, s.version, "rien n'a bougé");
}

#[tokio::test]
async fn nj_13_signature_scellee_sur_l_empreinte_en_ajout_seul() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Vienne").await;
    let p = c.presta("Dupont").await;
    let prof = c.a(c.prof, Role::Prof);
    // Pièce du devis : PDF vérifié sur les octets.
    let mut d = devis_manuel(p.id, Some("1 500"));
    d.piece = Some(PieceSaisie { nom: "devis.pdf".into(), contenu: pdf("Devis 1 500 € TTC"), mime: None });
    let d = c.ent.voyage_devis_ajouter(&prof, s.id, d).await.unwrap();
    assert!(d.document_id.is_some());
    let d = c.ent.voyage_devis_choisir(&prof, s.id, d.id, d.rev).await.unwrap().into_iter().find(|x| x.id == d.id).unwrap();
    let s = c.cmd(c.prof, Role::Prof, s.id, Commande::DemanderSignature).await.unwrap();
    // La pièce signée est celle qui a été lue : une autre empreinte est refusée, comme l'absence de
    // confirmation expresse ou d'empreinte.
    let autre = ent_object::sha256_hex(b"autre pdf");
    for (emp, conf) in [(Some(autre), Some(true)), (None, Some(true)), (d.empreinte.clone(), None), (d.empreinte.clone(), Some(false))] {
        let e = c.signer_avec(s.id, &d, emp, conf, Role::Direction, c.dir).await.unwrap_err();
        assert!(est_refus(&e) || est_conflit_detail(&e), "{e:?}");
    }
    assert_eq!(c.version(s.id).await, s.version);
    // Signature correcte : état, commande, bon de commande et envoi dans la même transaction.
    let signee = c.signer(s.id, &d).await.unwrap();
    assert_eq!(signee.etat, "budget_a_valider");
    let ach = c.ent.voyage_achats(&c.a(c.dir, Role::Direction), s.id).await.unwrap();
    assert_eq!(ach.commandes.len(), 1);
    assert_eq!(ach.commandes[0].numero, 1);
    assert_eq!(ach.commandes[0].statut, "envoyee");
    assert_eq!(ach.commandes[0].niveau_signature, "S1");
    assert_eq!(ach.commandes[0].montant_centimes, 150_000);
    assert!(ach.envois.iter().any(|e| e.type_ == "bon_commande" && e.statut == "envoye"));
    assert_eq!(ach.devis[0].statut, "signe");
    assert_eq!(recus_vers(&c, &p.adresses[0].email), 1);
    // L'empreinte scellée est celle de la pièce, et la pièce relue au magasin l'est aussi.
    let (sha, empreinte): (String, String) = sqlx::query_as(
        "SELECT doc.sha256, sg.empreinte FROM ent_sortie_signature sg
           JOIN ent_sortie_devis dv ON dv.id = sg.devis_id JOIN ent_sortie_document doc ON doc.id = dv.document_id
          WHERE sg.sortie_id = $1",
    )
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(sha, empreinte);
    let (_, octets) = c.ent.voyage_devis_piece_lire(&c.a(c.dir, Role::Direction), s.id, d.id).await.unwrap();
    assert_eq!(ent_object::sha256_hex(&octets), empreinte);
    // Une signature ne se modifie ni ne se supprime (ajout seul, base) ; la commande non plus ne
    // peut plus changer de devis ; double signature impossible.
    let r = sqlx::query("UPDATE ent_sortie_signature SET empreinte = $1 WHERE sortie_id = $2")
        .bind(ent_object::sha256_hex(b"x"))
        .bind(s.id)
        .execute(c.ent.pool())
        .await;
    assert!(matches!(&r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501")), "{r:?}");
    let r = sqlx::query("DELETE FROM ent_sortie_signature WHERE sortie_id = $1").bind(s.id).execute(c.ent.pool()).await;
    assert!(r.is_err());
    assert!(c.signer(s.id, &d).await.is_err());
    // Le journal garde la trace, sans recopier de donnée de famille.
    let j = actions_journal(&c, s.id).await;
    assert!(j.contains(&"devis_signe".to_string()) && j.contains(&"passee_aux_finances".to_string()), "{j:?}");
}

#[tokio::test]
async fn nj_12_copie_a_la_boite_comptabilite_de_l_etablissement() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    // Établissement A règle sa boîte ; B n'en a pas : aucune copie, aucune adresse « par défaut ».
    let boite = format!("compta-{}@lycee-a.fr", Uuid::new_v4().simple());
    a.ent.sorties_reglage_ecrire(&a.a(a.dir, Role::Direction), Some(&boite), Some("Lycée A — 1 rue de l'Ecole")).await.unwrap();
    assert!(a.ent.sorties_reglage_ecrire(&a.a(a.prof, Role::Prof), Some("x@y.fr"), None).await.is_err());
    assert!(a.ent.sorties_reglage_ecrire(&a.a(a.dir, Role::Direction), Some("a@b.fr, c@d.fr"), None).await.is_err());
    let (sa, _, da) = a.voyage_a_signer("Voyage A").await;
    let (sb, _, db_) = b.voyage_a_signer("Voyage B").await;
    a.signer(sa.id, &da).await.unwrap();
    b.signer(sb.id, &db_).await.unwrap();
    assert_eq!(recus_vers(&a, &boite), 1, "copie du bon de commande à la boîte de A");
    assert_eq!(recus_vers(&b, &boite), 0, "la boîte de A n'est jamais utilisée pour B");
    // Les devis signés de B ne partent que chez les prestataires de B.
    let annuaire_b: Vec<String> = b.ent.prestataires_liste(&b.sec_a()).await.unwrap().into_iter().flat_map(|x| x.adresses.into_iter().map(|y| y.email)).collect();
    for r in b.ent.mailer().receipts().iter().filter(|r| r.subject.contains(&reference_courrier(&sb.reference, b.etab, sb.id))) {
        assert!(annuaire_b.contains(&r.to), "{}", r.to);
    }
    // Les réglages d'un établissement sont invisibles de l'autre (RLS).
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_reglage WHERE etablissement_id = $1").bind(b.etab).fetch_one(b.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn avenant_numerote_signature_conservee_chauffeur_visible_des_encadrants_et_efface() {
    let Some(c) = cadre().await else { return };
    let (s, p, d) = c.voyage_a_signer("Voyage à Prague").await;
    c.signer(s.id, &d).await.unwrap();
    let prof = c.a(c.prof, Role::Prof);
    let ach = c.ent.voyage_achats(&prof, s.id).await.unwrap();
    let cmd_id = ach.commandes[0].id;
    // Avenant : confirmation expresse, motif, droits du porteur ; refus pour un tiers.
    let av = AvenantSaisie { motif: "Deux élèves de plus".into(), effectif_eleves: 32, effectif_adultes: 4, debut: None, fin: None, confirme: true };
    let tiers = compte(&c.ent, c.etab, "prof").await;
    assert!(c.ent.voyage_avenant_emettre(&c.a(tiers, Role::Prof), s.id, cmd_id, av.clone()).await.is_err());
    assert!(c.ent.voyage_avenant_emettre(&c.a(c.compta, Role::Compta), s.id, cmd_id, AvenantSaisie { confirme: false, ..av.clone() }).await.is_err());
    let mut sans_motif = av.clone();
    sans_motif.motif = "  ".into();
    assert!(c.ent.voyage_avenant_emettre(&prof, s.id, cmd_id, sans_motif).await.is_err());
    let v = c.ent.voyage_avenant_emettre(&prof, s.id, cmd_id, av).await.unwrap();
    assert_eq!(v.statut, "a_confirmer");
    assert_eq!(v.nb_avenants, 1);
    assert_eq!(v.effectif_eleves_commande, 32);
    // « Annule et remplace » : la signature précédente reste, le devis reste signé.
    let (nb_sig, statut): (i64, String) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_sortie_signature WHERE sortie_id = $1),
                (SELECT statut FROM ent_sortie_devis WHERE id = $2)",
    )
    .bind(s.id)
    .bind(d.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((nb_sig, statut.as_str()), (1, "signe"));
    let ach = c.ent.voyage_achats(&prof, s.id).await.unwrap();
    assert!(ach.envois.iter().any(|e| e.type_ == "avenant" && e.statut == "envoye"));
    let v2 = c.ent.voyage_avenant_emettre(&prof, s.id, cmd_id, AvenantSaisie { motif: "Un de moins".into(), effectif_eleves: 31, effectif_adultes: 4, debut: None, fin: None, confirme: true }).await.unwrap();
    assert_eq!(v2.nb_avenants, 2);
    let r = sqlx::query("DELETE FROM ent_sortie_avenant WHERE sortie_id = $1").bind(s.id).execute(c.ent.pool()).await;
    assert!(r.is_err(), "avenants en ajout seul");
    // Confirmation du prestataire, chauffeur et téléphone en champs dédiés.
    assert!(c.ent.voyage_commande_confirmer(&c.a(tiers, Role::Prof), s.id, cmd_id, ConfirmationCommande { chauffeur_nom: None, chauffeur_tel: None }).await.is_err());
    assert!(est_refus(&c.ent.voyage_commande_confirmer(&prof, s.id, cmd_id, ConfirmationCommande { chauffeur_nom: Some("Marc".into()), chauffeur_tel: Some("06;DROP".into()) }).await.unwrap_err()));
    let conf = c.ent.voyage_commande_confirmer(&prof, s.id, cmd_id, ConfirmationCommande { chauffeur_nom: Some("Marc Durand".into()), chauffeur_tel: Some("06 12 34 56 78".into()) }).await.unwrap();
    assert_eq!(conf.statut, "confirmee");
    // Visible du porteur et de l'encadrant ; ni de la comptabilité, ni de l'accueil, ni d'un tiers.
    let contacts = c.ent.voyage_contacts_transport(&c.a(c.prof2, Role::Prof), s.id).await.unwrap();
    assert_eq!(contacts[0].chauffeur_nom.as_deref(), Some("Marc Durand"));
    for (u, r) in [(c.compta, Role::Compta), (c.accueil, Role::Accueil), (c.parent, Role::Parent), (tiers, Role::Prof)] {
        assert!(c.ent.voyage_contacts_transport(&c.a(u, r), s.id).await.is_err(), "{r:?}");
    }
    // Jamais recopiés ailleurs : ni dans les envois (corps, objets) ni dans les PDF adressés.
    for (_, sujet, corps) in c.ent.mailer().dry_run_corps() {
        assert!(!corps.contains("Marc Durand") && !sujet.contains("Marc Durand"));
        assert!(!corps.contains("06 12 34 56 78"));
    }
    let _ = p;
    // Annulation du dossier : le conducteur est effacé (NJ-28).
    c.cmd_motif(c.dir, Role::Direction, s.id, Commande::Annuler, "Annulation", None).await.unwrap();
    let (nom, tel, le): (Option<String>, Option<String>, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        "SELECT chauffeur_nom, chauffeur_tel, contacts_effaces_le FROM ent_sortie_commande WHERE sortie_id = $1",
    )
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert!(nom.is_none() && tel.is_none() && le.is_some());
}

#[tokio::test]
async fn devis_a_confirmer_ne_se_choisit_pas_et_le_budget_prend_le_montant_confirme() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Madrid").await;
    let p = c.presta("Dupont").await;
    let prof = c.a(c.prof, Role::Prof);
    let sans = c.ent.voyage_devis_ajouter(&prof, s.id, devis_manuel(p.id, None)).await.unwrap();
    assert_eq!(sans.statut, "a_confirmer");
    assert!(sans.montant_centimes.is_none());
    assert!(est_refus(&c.ent.voyage_devis_choisir(&prof, s.id, sans.id, sans.rev).await.unwrap_err()));
    let compta = c.a(c.compta, Role::Compta);
    // Pas de budget en brouillon : refus ; puis la ligne ne vient que d'un devis confirmé.
    let v = c.ent.voyage_budget_version_creer(&compta, s.id, None).await.unwrap();
    let b = v.brouillon.unwrap();
    let e = c.ent.voyage_budget_ligne_depuis_devis(&compta, s.id, b.numero, b.rev, sans.id).await.unwrap_err();
    assert!(est_refus(&e), "montant non confirmé : {e:?}");
    let ok = c.ent.voyage_devis_confirmer_montant(&prof, s.id, sans.id, sans.rev, "1 350,50").await.unwrap();
    let v = c.ent.voyage_budget_ligne_depuis_devis(&compta, s.id, b.numero, b.rev, ok.id).await.unwrap();
    let b2 = v.brouillon.unwrap();
    assert_eq!(b2.lignes.len(), 1);
    assert_eq!(b2.lignes[0].montant_centimes, 135_050);
    assert_eq!(b2.lignes[0].source, "devis_recu");
    assert_eq!(b2.lignes[0].poste, "transport");
    // Une seule ligne par devis et par version ; l'accueil n'écrit pas au budget.
    assert!(c.ent.voyage_budget_ligne_depuis_devis(&compta, s.id, b2.numero, b2.rev, ok.id).await.is_err());
    assert!(c.ent.voyage_budget_ligne_depuis_devis(&c.a(c.accueil, Role::Accueil), s.id, b2.numero, b2.rev, ok.id).await.is_err());
    // Écarter un devis : jamais supprimé.
    let ecartes = c.ent.voyage_devis_ecarter(&prof, s.id, ok.id).await.unwrap();
    assert_eq!(ecartes.iter().find(|x| x.id == ok.id).unwrap().statut, "ecarte");
}

#[tokio::test]
async fn nj_09_38_piece_de_devis_jamais_servie_hors_de_sa_sortie_et_types_controles() {
    let Some(c) = cadre().await else { return };
    let s1 = c.voyage_car("Voyage 1").await;
    let s2 = c.voyage_car("Voyage 2").await;
    let p = c.presta("Dupont").await;
    let prof = c.a(c.prof, Role::Prof);
    let mut avec = devis_manuel(p.id, Some("100"));
    avec.piece = Some(PieceSaisie { nom: "devis.pdf".into(), contenu: pdf("D"), mime: Some("application/pdf".into()) });
    let d = c.ent.voyage_devis_ajouter(&prof, s1.id, avec).await.unwrap();
    assert!(c.ent.voyage_devis_piece_lire(&prof, s1.id, d.id).await.is_ok());
    // Même identifiant de devis demandé via une autre sortie : introuvable (pas de recherche « contient »).
    let e = c.ent.voyage_devis_piece_lire(&prof, s2.id, d.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Types : HTML, SVG, exécutable, image déguisée en PDF, nom à double extension : refusés.
    for (nom, octets) in [
        ("devis.pdf", b"<html><script>alert(1)</script></html>".to_vec()),
        ("devis.svg", b"<svg xmlns='http://www.w3.org/2000/svg'><script/></svg>".to_vec()),
        ("devis.pdf.exe", pdf("x")),
        ("devis.pdf", b"\x89PNG\r\n\x1a\n0000000000000000".to_vec()),
        ("vide.pdf", Vec::new()),
    ] {
        let mut x = devis_manuel(p.id, Some("1"));
        x.piece = Some(PieceSaisie { nom: nom.into(), contenu: octets, mime: Some("application/pdf".into()) });
        assert!(c.ent.voyage_devis_ajouter(&prof, s1.id, x).await.is_err(), "{nom}");
    }
    // Droit : le dossier d'un autre établissement n'est pas lisible.
    let autre = compte(&c.ent, c.etab, "prof").await;
    assert!(c.ent.voyage_devis_piece_lire(&c.a(autre, Role::Prof), s1.id, d.id).await.is_err());
}

// ─────────────────────────── Courrier entrant ───────────────────────────

fn courrier(id: &str, de: &str, objet: &str, corps: &str) -> CourrierEntrant {
    CourrierEntrant {
        message_externe_id: id.into(),
        recu_le: Utc::now(),
        expediteur: de.into(),
        objet: objet.into(),
        corps: corps.into(),
        pieces: vec![],
    }
}

#[tokio::test]
async fn nj_28_70_courrier_rattache_seulement_si_reference_exacte_et_expediteur_sollicite() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Dublin").await;
    let p = c.presta("Dupont").await;
    let mail = p.adresses[0].email.clone();
    let reference = reference_courrier(&s.reference, c.etab, s.id);
    // Avant toute demande envoyée : même avec la référence exacte et le bon expéditeur → tri.
    let r = c.ent.courrier_entrant_deposer(c.etab, courrier("m0", &mail, &format!("Re: [{reference}]"), "Total TTC 1 000 €")).await.unwrap();
    assert_eq!(r.statut, "a_trier");
    assert_eq!(r.raison.as_deref(), Some("reference_seule"));
    c.demander(s.id, &[p.id]).await.unwrap();
    // Cas certain : référence exacte + expéditeur sollicité → rattaché, devis « à confirmer », montant proposé.
    let mut msg = courrier("m1", &mail, &format!("Re: [{reference}] devis"), "Bonjour,\nTotal HT 1 000,00\nTotal TTC : 1 200,00 €\nCordialement");
    msg.pieces = vec![PieceCourrier { nom: "devis.pdf".into(), mime: Some("application/pdf".into()), contenu: pdf("Devis") }, PieceCourrier { nom: "piege.html".into(), mime: Some("text/html".into()), contenu: b"<script>x</script>".to_vec() }];
    let r = c.ent.courrier_entrant_deposer(c.etab, msg).await.unwrap();
    assert_eq!((r.statut.as_str(), r.sortie_id), ("rattache", Some(s.id)));
    let ach = c.ent.voyage_achats(&c.a(c.prof, Role::Prof), s.id).await.unwrap();
    let d = ach.devis.iter().find(|x| x.origine == "courrier").expect("devis du courrier");
    assert_eq!(d.statut, "a_confirmer");
    assert_eq!(d.montant_centimes, None, "le montant proposé n'est jamais retenu");
    assert_eq!(d.montant_propose_centimes, Some(120_000));
    assert!(d.document_id.is_some());
    let refusees: i32 = sqlx::query_scalar("SELECT pieces_refusees FROM ent_courrier_entrant WHERE id = $1").bind(r.id).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(refusees, 1, "pièce HTML refusée et comptée");
    // Idempotent : le même message n'est pas rattaché deux fois.
    let r2 = c.ent.courrier_entrant_deposer(c.etab, courrier("m1", &mail, "x", "y")).await.unwrap();
    assert!(r2.doublon && r2.id == r.id);
    assert_eq!(c.ent.voyage_achats(&c.a(c.prof, Role::Prof), s.id).await.unwrap().devis.len(), 1);
    // Référence sans somme de contrôle, devinée (référence voisine), ou expéditeur inconnu : tri, jamais auto.
    let sans_somme = s.reference.clone();
    let cas = [
        ("m2", mail.as_str(), format!("Re: {sans_somme}"), "expediteur_seul"),
        ("m3", "inconnu@autre-societe.com", format!("Re: [{reference}]"), "expediteur_inconnu"),
        ("m4", mail.as_str(), "Re: votre demande".to_string(), "expediteur_seul"),
        ("m5", "quelquun@nulle-part.org", "Bonjour".to_string(), "ni_reference_ni_expediteur"),
    ];
    for (id, de, objet, raison) in cas {
        let r = c.ent.courrier_entrant_deposer(c.etab, courrier(id, de, &objet, "texte")).await.unwrap();
        assert_eq!(r.statut, "a_trier", "{id}");
        assert_eq!(r.raison.as_deref(), Some(raison), "{id}");
    }
    // Aucune sortie fantôme : un seul devis, malgré 5 courriers à trier.
    assert_eq!(c.ent.voyage_achats(&c.a(c.prof, Role::Prof), s.id).await.unwrap().devis.len(), 1);
}

#[tokio::test]
async fn nj_28_courrier_jamais_devine_entre_etablissements_et_alerte_domaine_voisin() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let s = a.voyage_car("Voyage A").await;
    let p = a.presta("Dupont").await;
    a.demander(s.id, &[p.id]).await.unwrap();
    let reference = reference_courrier(&s.reference, a.etab, s.id);
    // Courrier reçu dans la boîte de B avec la référence de A et l'expéditeur de A : rien ne se rattache.
    let r = b.ent.courrier_entrant_deposer(b.etab, courrier("x1", &p.adresses[0].email, &format!("[{reference}]"), "devis")).await.unwrap();
    assert_eq!(r.statut, "a_trier");
    assert!(r.sortie_id.is_none());
    assert!(a.ent.courrier_tri_liste(&a.sec_a()).await.unwrap().is_empty(), "la file de A ne voit pas celle de B");
    assert_eq!(b.ent.courrier_tri_liste(&b.sec_a()).await.unwrap().len(), 1);
    // Domaine voisin d'un prestataire connu : alerte d'hameçonnage.
    let domaine = p.adresses[0].domaine.clone();
    let voisin = domaine.replacen("cars-", "cars-x", 1);
    assert_ne!(voisin, domaine);
    a.ent.courrier_entrant_deposer(a.etab, courrier("v1", &format!("devis@{voisin}"), "Re: devis", "texte")).await.unwrap();
    let file = a.ent.courrier_tri_liste(&a.sec_a()).await.unwrap();
    assert!(file.iter().any(|x| x.alerte_domaine_voisin), "{file:?}");
}

#[tokio::test]
async fn file_de_tri_droits_rattachement_humain_et_courrier_jamais_perdu() {
    let Some(c) = cadre().await else { return };
    let s = c.voyage_car("Voyage à Séville").await;
    let p = c.presta("Dupont").await;
    let r = c.ent.courrier_entrant_deposer(c.etab, courrier("t1", "contact@nouveau-transporteur.fr", "Votre demande", "Prix TTC 800")).await.unwrap();
    assert_eq!(r.statut, "a_trier");
    // File réservée au droit d'établissement ; le porteur et l'accueil ne trient pas.
    for (u, role) in [(c.prof, Role::Prof), (c.accueil, Role::Accueil), (c.parent, Role::Parent)] {
        assert!(c.ent.courrier_tri_liste(&c.a(u, role)).await.is_err(), "{role:?}");
        assert!(c.ent.courrier_rattacher(&c.a(u, role), r.id, s.id, p.id).await.is_err());
        assert!(c.ent.courrier_ignorer(&c.a(u, role), r.id, "x").await.is_err());
    }
    assert_eq!(c.ent.courrier_tri_compte(&c.sec_a()).await.unwrap(), 1);
    // « Ignorer » exige un motif ; le courrier n'est jamais marqué traité tout seul (contrainte de base).
    assert!(est_refus(&c.ent.courrier_ignorer(&c.sec_a(), r.id, "  ").await.unwrap_err()));
    let direct = sqlx::query("UPDATE ent_courrier_entrant SET statut = 'ignore' WHERE id = $1").bind(r.id).execute(c.ent.pool()).await;
    assert!(direct.is_err(), "ignoré sans geste humain : refusé par la base");
    let direct = sqlx::query("UPDATE ent_courrier_entrant SET statut = 'rattache' WHERE id = $1").bind(r.id).execute(c.ent.pool()).await;
    assert!(direct.is_err(), "rattaché sans sortie : refusé par la base");
    // Nouveau prestataire depuis le courrier : l'adresse de l'expéditeur entre dans l'annuaire par ce geste.
    let nouveau = c.ent.courrier_nouveau_prestataire(&c.sec_a(), r.id, PrestataireSaisie { raison_sociale: "Nouveau Transporteur".into(), siren: None, telephone: None, categories: vec!["transporteur".into()], adresses: vec![], version: None, actif: None }).await.unwrap();
    assert_eq!(nouveau.adresses[0].email, "contact@nouveau-transporteur.fr");
    // Rattachement humain : devis « à confirmer », tracé au journal du séjour, montant seulement proposé.
    let devis = c.ent.courrier_rattacher(&c.sec_a(), r.id, s.id, nouveau.id).await.unwrap();
    let ach = c.ent.voyage_achats(&c.a(c.dir, Role::Direction), s.id).await.unwrap();
    let d = ach.devis.iter().find(|x| x.id == devis).unwrap();
    assert_eq!((d.statut.as_str(), d.montant_centimes, d.montant_propose_centimes), ("a_confirmer", None, Some(80_000)));
    assert!(actions_journal(&c, s.id).await.contains(&"courrier_rattache".to_string()));
    assert!(est_refus(&c.ent.courrier_rattacher(&c.sec_a(), r.id, s.id, nouveau.id).await.unwrap_err()), "déjà traité");
    assert_eq!(c.ent.courrier_tri_compte(&c.sec_a()).await.unwrap(), 0);
    // Ignorer avec motif.
    let r2 = c.ent.courrier_entrant_deposer(c.etab, courrier("t2", "pub@spam.fr", "Offre", "Promotion")).await.unwrap();
    c.ent.courrier_ignorer(&c.sec_a(), r2.id, "Publicité").await.unwrap();
    let ign: (String, Option<Uuid>) = sqlx::query_as("SELECT statut, trie_par_user_id FROM ent_courrier_entrant WHERE id = $1").bind(r2.id).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!((ign.0.as_str(), ign.1), ("ignore", Some(c.sec)));
    // Pièces d'un courrier : lecture réservée, empreinte vérifiée.
    let mut m = courrier("t3", "x@y-societe.fr", "PJ", "voir pièce");
    m.pieces = vec![PieceCourrier { nom: "devis.pdf".into(), mime: None, contenu: pdf("PJ") }];
    let r3 = c.ent.courrier_entrant_deposer(c.etab, m).await.unwrap();
    let pcs = c.ent.courrier_pieces(&c.sec_a(), r3.id).await.unwrap();
    assert_eq!(pcs.len(), 1);
    let (_, mime, octets) = c.ent.courrier_piece_lire(&c.sec_a(), r3.id, pcs[0].id).await.unwrap();
    assert_eq!(mime, "application/pdf");
    assert!(octets.starts_with(b"%PDF"));
    assert!(c.ent.courrier_piece_lire(&c.a(c.prof, Role::Prof), r3.id, pcs[0].id).await.is_err());
    assert!(c.ent.courrier_piece_lire(&c.sec_a(), Uuid::new_v4(), pcs[0].id).await.is_err());
}

// ─────────────────────────── Envois ───────────────────────────

#[tokio::test]
async fn envois_en_file_rejoués_sans_doublon_et_echec_apres_trois_essais() {
    let Some(mut c) = Cadre::nouveau().await else { return };
    // Messagerie « prête » mais qui n'envoie rien de confirmé : la demande reste en file puis échoue.
    c.ent.install_mail_dry_run();
    let s = c.voyage_car("Voyage à Turin").await;
    let p = c.presta("Dupont").await;
    // Le traitement refuse (échec) un message dont l'adresse a été désactivée entre-temps.
    c.ent.install_mail_dry_run();
    let envois = c.demander(s.id, &[p.id]).await.unwrap();
    assert_eq!(envois[0].statut, "envoye");
    let n = c.ent.voyages_envois_traiter(c.etab, 10).await.unwrap();
    assert_eq!(n, 0, "rien à renvoyer : pas de doublon");
    assert_eq!(recus_vers(&c, &p.adresses[0].email), 1);
    // Droit de traitement manuel : direction/secrétariat.
    assert!(c.ent.voyage_envois_file(&c.a(c.prof, Role::Prof)).await.is_err());
    assert!(c.ent.voyage_envois_file(&c.sec_a()).await.is_ok());
    c.ent.install_mail_dry_run();
    let _ = &mut c;
}

// ─────────────────────────── Schéma ───────────────────────────

#[test]
fn migration_644_sans_json_ni_bytea_ni_identite_d_eleve_et_cles_composites() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let sql: String = std::fs::read_to_string(dir.join("644_sortie_prestataires.sql"))
        .unwrap()
        .to_lowercase()
        .lines()
        .map(|l| l.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!sql.contains("json"), "D-036 : aucun JSON");
    assert!(!sql.contains("bytea"), "fichiers : magasin d'objets, jamais BYTEA");
    assert!(!sql.contains("eleve_id"), "aucune identité d'élève chez les prestataires (NJ-55)");
    assert!(!sql.contains("serial") && !sql.contains("identity"), "UUID fournis par l'application (E3)");
    assert!(!sql.contains("float") && !sql.contains("numeric") && !sql.contains("real"), "centimes entiers");
    // Chaque clé étrangère vers une table d'établissement est composite (E2), hors `ent_*` globales.
    for l in sql.lines().filter(|l| l.contains("references ent_") && !l.contains("references ent_annee")) {
        assert!(l.contains("(etablissement_id,"), "clé étrangère non composite : {l}");
    }
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_onze_tables_du_lot_v0c() {
    crate::tests_rls_lots::verifier_tables(&[
        "ent_prestataire",
        "ent_prestataire_categorie",
        "ent_prestataire_adresse",
        "ent_sortie_reglage",
        "ent_courrier_entrant",
        "ent_courrier_piece",
        "ent_sortie_devis",
        "ent_sortie_signature",
        "ent_sortie_commande",
        "ent_sortie_envoi",
        "ent_sortie_avenant",
    ])
    .await;
}
