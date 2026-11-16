//! Socle 42, lot V0d (restauration, journal de voyage, droit à l'image, reprise des séjours de Next.js)
//! contre la base locale de test. Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Défauts de Next.js fermés par V0d, chacun avec son test (`docs/socles/42_SORTIES_VOYAGES.md` §2.3) :
//!
//! * VD-06 `vd_06_…` destinataire de la restauration : la boîte de l'établissement, jamais une adresse client ;
//! * VD-10, VD-43 `vd_10_43_…` lien du journal : nominatif, révocable, expirant, limité en débit ;
//! * VD-11 `vd_11_…` photos publiées sans droit à l'image (et case « vraie par défaut » : R-SV40) ;
//! * VD-12 `vd_12_…` audience du journal recalculée (élève retiré = accès retiré) ;
//! * VD-16 `vd_16_…` aucun INE, aucun contact de parent chez la cuisine ni chez un prestataire ;
//! * VD-44 `vd_44_…` « qui mange » suit l'état des participants (un désisté perd son panier) ;
//! * R-SV37 `r_sv37_…` propriété unique : la reprise n'écrit jamais sur un séjour de ScolIA.

use super::*;
use crate::formulaires::{NouveauFormulaire, ReponseFormulaire};
use crate::tests_voyages_v0a::{compte, est_forbidden, est_introuvable, est_refus, Cadre};
use crate::voyages_courrier_ports::{adresse_expedition, CibleReponse, Reponse};
use crate::voyages_prestataires_repo::PrestataireSaisie;
use crate::voyages_journal_repo::{DistributionLien, PhotoSaisie, PublicationSaisie};
use crate::voyages_reprise_repo::{ParamsReprise, ParticipantImport, SejourImport};
use crate::voyages_repo::SortieLigne;
use crate::voyages_restauration_repo::{AttributionSaisie, ExterneSaisie, LigneSaisie, RepasSaisie};
use chrono::{Duration, TimeZone, Utc};
use ent_auth::Role;
use std::sync::Mutex;
use uuid::Uuid;

pub(crate) async fn cadre() -> Option<Cadre> {
    let mut c = Cadre::nouveau().await?;
    c.ent.install_mail_dry_run();
    Some(c)
}

pub(crate) fn recus_vers(c: &Cadre, adr: &str) -> usize {
    c.ent.mailer().receipts().iter().filter(|r| r.to == adr).count()
}

pub(crate) fn png() -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    v.extend((0..200u8).map(|i| i.wrapping_mul(7)));
    v
}

impl Cadre {
    /// Sortie sans nuitée autorisée (`ouverte`), avec les trois élèves inscrits.
    async fn ouverte_avec_eleves(&self, titre: &str) -> SortieLigne {
        let s = self.creer(titre, "sans_nuitee").await;
        self.jusqu_a_ouverte(s.id).await;
        let r = self
            .ent
            .voyage_participants_ajouter(&self.a(self.prof, Role::Prof), s.id, self.eleves.clone())
            .await
            .expect("participants");
        assert_eq!(r.ajoutes.len(), 3);
        s
    }

    pub(crate) async fn participant(&self, sortie: Uuid, eleve: Uuid) -> Uuid {
        sqlx::query_scalar("SELECT id FROM ent_sortie_participant WHERE sortie_id = $1 AND eleve_id = $2")
            .bind(sortie)
            .bind(eleve)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn encadrant(&self, sortie: Uuid) -> Uuid {
        sqlx::query_scalar("SELECT id FROM ent_sortie_encadrant WHERE sortie_id = $1 AND role = 'responsable'")
            .bind(sortie)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    pub(crate) fn prof_a(&self) -> Acteur<'static> {
        self.a(self.prof, Role::Prof)
    }

    pub(crate) fn dir_a(&self) -> Acteur<'static> {
        self.a(self.dir, Role::Direction)
    }

    /// Consentement au droit à l'image : formulaire de classe, puis décision du parent de l'élève.
    async fn formulaire_image(&self) -> Uuid {
        let f = self
            .ent
            .creer_formulaire(
                self.etab,
                self.dir,
                "direction",
                NouveauFormulaire {
                    type_formulaire: "consentement".into(),
                    objet: Some("droit_image".into()),
                    titre: "Droit à l'image".into(),
                    texte: "Autorisez-vous la publication des photos du voyage ?".into(),
                    audience: "classe".into(),
                    cible_ids: vec![self.classe],
                },
            )
            .await
            .expect("formulaire");
        self.ent.publier_formulaire(self.etab, self.dir, "direction", f.id).await.expect("publication");
        f.id
    }

    async fn decider_image(&self, formulaire: Uuid, parent: Uuid, eleve: Uuid, decision: &str) {
        self.ent
            .repondre_formulaire(
                self.etab,
                parent,
                formulaire,
                ReponseFormulaire { eleve_id: eleve, decision: decision.into(), commentaire: None },
            )
            .await
            .expect("décision");
    }

    /// Deuxième parent (avec compte) de l'élève `eleve`, autorité parentale par défaut.
    async fn parent_de(&self, eleve: Uuid) -> Uuid {
        let p = compte(&self.ent, self.etab, "parent").await;
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::new_v4())
            .bind(self.etab)
            .bind(eleve)
            .bind(p)
            .execute(self.ent.pool())
            .await
            .expect("lien");
        p
    }

    /// Titulaire SANS compte d'un élève (personne du socle 4, lien sans utilisateur).
    async fn titulaire_sans_compte(&self, eleve: Uuid) -> Uuid {
        let personne = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1,$2,'Titulaire Sans Compte','saisie')")
            .bind(personne)
            .bind(self.etab)
            .execute(self.ent.pool())
            .await
            .expect("personne");
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, autorite_parentale)
             VALUES ($1,$2,$3,NULL,$4,true)",
        )
        .bind(Uuid::new_v4())
        .bind(self.etab)
        .bind(eleve)
        .bind(personne)
        .execute(self.ent.pool())
        .await
        .expect("lien sans compte");
        personne
    }
}

// ═════════════════════════════ Restauration ═════════════════════════════

fn commande(c: &Cadre, rev: Option<i32>, standard: i32, vegetarien: i32) -> RepasSaisie {
    RepasSaisie {
        rev,
        livraison_heure: chrono::NaiveTime::from_hms_opt(8, 30, 0),
        livraison_lieu: Some("Portail principal".into()),
        lignes: vec![
            LigneSaisie { jour: c.jour, categorie: "panier".into(), variante: Some("standard".into()), quantite: standard },
            LigneSaisie { jour: c.jour, categorie: "panier".into(), variante: Some("vegetarien".into()), quantite: vegetarien },
            LigneSaisie { jour: c.jour, categorie: "boisson".into(), variante: None, quantite: 40 },
        ],
        externes: vec![],
    }
}

fn attribution(c: &Cadre, rev: i32, participant: Option<Uuid>, encadrant: Option<Uuid>, variante: &str) -> AttributionSaisie {
    AttributionSaisie {
        rev,
        jour: c.jour,
        participant_id: participant,
        encadrant_id: encadrant,
        variante: Some(variante.into()),
        etiquette_nominative: variante != "standard",
    }
}

#[tokio::test]
async fn vd_06_16_44_commande_qui_mange_plafonnee_exacte_et_boite_de_l_etablissement() {
    let Some(c) = cadre().await else { return };
    let s = c.ouverte_avec_eleves("Sortie musée").await;
    let prof = c.prof_a();
    // Création de la commande : 2 paniers standard et 1 végétarien pour 3 élèves.
    let v = c.ent.voyage_repas_ecrire(&prof, s.id, commande(&c, None, 2, 1)).await.unwrap();
    let rev = v.commande.as_ref().unwrap().rev;
    assert_eq!(v.commande.as_ref().unwrap().statut, "brouillon");
    // Une seconde création « à blanc » (sans la révision lue) est refusée : les écritures concurrentes ne s'écrasent pas.
    assert!(c.ent.voyage_repas_ecrire(&prof, s.id, commande(&c, None, 9, 9)).await.is_err());
    assert!(c.ent.voyage_repas_ecrire(&prof, s.id, commande(&c, Some(rev + 7), 9, 9)).await.is_err());
    // Attributions : alpha et bravo standard, charlie végétarien, plus l'adulte responsable pour le panier standard.
    let (p0, p1, p2) = (c.participant(s.id, c.eleves[0]).await, c.participant(s.id, c.eleves[1]).await, c.participant(s.id, c.eleves[2]).await);
    let enc = c.encadrant(s.id).await;
    let mut rev = rev;
    let v = c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, Some(p0), None, "standard")).await.unwrap();
    rev = v.commande.as_ref().unwrap().rev;
    let v = c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, Some(p1), None, "standard")).await.unwrap();
    rev = v.commande.as_ref().unwrap().rev;
    // Plafond : un troisième panier standard n'existe pas dans la commande.
    let e = c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, None, Some(enc), "standard")).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // Un même élève ne mange pas deux paniers le même jour ; un panier est pour un participant OU un adulte.
    assert!(c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, Some(p0), None, "standard")).await.is_err());
    assert!(c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, Some(p0), Some(enc), "standard")).await.is_err());
    // Pas encore exacte (il manque le panier végétarien) : la commande ne part pas.
    let v = c.ent.voyage_repas_lire(&prof, s.id).await.unwrap();
    assert_eq!(v.ecarts.len(), 1);
    rev = v.commande.as_ref().unwrap().rev;
    assert!(est_refus(&c.ent.voyage_repas_passer(&prof, s.id, rev, true).await.unwrap_err()));
    let v = c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, Some(p2), None, "vegetarien")).await.unwrap();
    rev = v.commande.as_ref().unwrap().rev;
    assert!(v.ecarts.is_empty());
    // Aperçu = rendu réel : sans INE, sans coordonnée de parent ; noms, classes et variantes seulement.
    let ap = c.ent.voyage_repas_apercu(&prof, s.id).await.unwrap();
    assert!(!ap.objet.contains('\n') && ap.objet.contains(&s.reference));
    assert!(ap.corps.contains("Paniers repas (standard) : 2") && ap.corps.contains("végétarien"));
    assert!(ap.corps.contains("Prénom0") && ap.corps.contains("(à étiqueter au nom de l'élève)"));
    for interdit in ["INE", "@", "téléphone", "motif"] {
        assert!(!ap.corps.contains(interdit), "l'avis ne doit pas contenir « {interdit} » : {}", ap.corps);
    }
    // Sans confirmation expresse : refus. Sans boîte de restauration : l'avis attend, rien ne part.
    assert!(c.ent.voyage_repas_passer(&prof, s.id, rev, false).await.is_err());
    let avant = c.ent.mailer().receipts().len();
    let v = c.ent.voyage_repas_passer(&prof, s.id, rev, true).await.unwrap();
    assert_eq!(v.commande.as_ref().unwrap().statut, "passee");
    assert_eq!(v.avis.len(), 1);
    // Sans boîte réglée, l'envoi est tenté après le geste et reste « sans destinataire ».
    let v = c.ent.voyage_repas_lire(&prof, s.id).await.unwrap();
    assert_eq!(v.avis[0].statut, "sans_destinataire");
    assert_eq!(c.ent.mailer().receipts().len(), avant, "aucun envoi sans boîte réglée");
    // VD-06 : la boîte est celle de l'établissement, réglée par la direction seule, adresse unique et valide.
    let boite = format!("cuisine-{}@lycee.fr", Uuid::new_v4().simple());
    for (a, attendu_forbidden) in [(c.a(c.sec, Role::Administratif), true), (c.a(c.compta, Role::Compta), true), (prof, true)] {
        let e = c.ent.voyages_reglage_restauration_ecrire(&a, Some(&boite)).await.unwrap_err();
        assert_eq!(est_forbidden(&e), attendu_forbidden, "{e:?}");
    }
    for mauvais in ["a@b.fr, c@d.fr", "a@b.fr;c@d.fr", "pas une adresse", "x@y.fr\nBcc: z@y.fr"] {
        assert!(c.ent.voyages_reglage_restauration_ecrire(&c.dir_a(), Some(mauvais)).await.is_err(), "{mauvais:?}");
    }
    c.ent.voyages_reglage_restauration_ecrire(&c.dir_a(), Some(&boite)).await.unwrap();
    assert_eq!(recus_vers(&c, &boite), 1, "l'avis en attente part dès que la boîte est réglée");
    let v = c.ent.voyage_repas_lire(&c.prof_a(), s.id).await.unwrap();
    assert_eq!(v.avis[0].statut, "envoye");
    // Le message part du seul destinataire réglé, avec exactement le texte composé par le serveur.
    let recu = c.ent.mailer().receipts().into_iter().find(|r| r.to == boite).unwrap();
    assert_eq!(recu.subject, v.avis[0].objet);
    // Le contenu d'un avis envoyé est figé par la base (ajout seul) et un avis ne se supprime pas.
    let r = sqlx::query("UPDATE ent_sortie_repas_avis SET corps = 'autre' WHERE commande_id = $1")
        .bind(v.commande.as_ref().unwrap().id)
        .execute(c.ent.pool())
        .await;
    assert!(matches!(&r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501")), "{r:?}");
    assert!(sqlx::query("DELETE FROM ent_sortie_repas_avis").execute(c.ent.pool()).await.is_err());
    // Journal : trace des gestes, sans nom ni motif.
    let j = crate::tests_voyages_v0a::actions_journal(&c, s.id).await;
    assert!(j.contains(&"repas_commande_passee".to_string()), "{j:?}");
}

#[tokio::test]
async fn vd_44_un_desiste_perd_son_panier_et_l_avenant_annule_et_remplace() {
    let Some(c) = cadre().await else { return };
    let s = c.ouverte_avec_eleves("Sortie plage").await;
    let prof = c.prof_a();
    let boite = format!("cuisine-{}@lycee.fr", Uuid::new_v4().simple());
    c.ent.voyages_reglage_restauration_ecrire(&c.dir_a(), Some(&boite)).await.unwrap();
    let mut rev = c.ent.voyage_repas_ecrire(&prof, s.id, commande(&c, None, 3, 0)).await.unwrap().commande.unwrap().rev;
    for e in &c.eleves {
        let p = c.participant(s.id, *e).await;
        rev = c.ent.voyage_repas_attribuer(&prof, s.id, attribution(&c, rev, Some(p), None, "standard")).await.unwrap().commande.unwrap().rev;
    }
    c.ent.voyage_repas_passer(&prof, s.id, rev, true).await.unwrap();
    assert_eq!(recus_vers(&c, &boite), 1);
    // Une commande passée n'est pas rouverte comme une première commande : l'avenant est la seule voie.
    let v = c.ent.voyage_repas_lire(&prof, s.id).await.unwrap();
    rev = v.commande.as_ref().unwrap().rev;
    assert!(!v.a_transmettre);
    assert!(est_refus(&c.ent.voyage_repas_avenant_emettre(&prof, s.id, rev, "rien", true).await.unwrap_err()), "aucun changement à transmettre");
    // Désistement d'un élève : son panier n'est plus compté et un avenant est proposé (sans nom).
    let p2 = c.participant(s.id, c.eleves[2]).await;
    c.ent.voyage_participant_desister(&prof, s.id, p2, "famille", None).await.unwrap();
    let v = c.ent.voyage_repas_lire(&prof, s.id).await.unwrap();
    assert!(v.a_transmettre);
    assert_eq!(v.propositions.len(), 1);
    assert!(!v.propositions[0].contains("Prénom"));
    assert!(v.attributions.iter().any(|x| x.ignoree));
    // Un avenant tant que les paniers ne sont plus exacts (3 commandés, 2 qui partent) est refusé.
    rev = v.commande.as_ref().unwrap().rev;
    assert!(est_refus(&c.ent.voyage_repas_avenant_emettre(&prof, s.id, rev, "Désistement", true).await.unwrap_err()));
    // On retire l'attribution du désisté et on réduit la commande à 2 : exact, avenant n° 1.
    let att = v.attributions.iter().find(|x| x.ignoree).unwrap().id;
    let v = c.ent.voyage_repas_attribution_retirer(&prof, s.id, att, rev).await.unwrap();
    rev = v.commande.as_ref().unwrap().rev;
    let v = c.ent.voyage_repas_ecrire(&prof, s.id, commande(&c, Some(rev), 2, 0)).await.unwrap();
    rev = v.commande.as_ref().unwrap().rev;
    assert!(v.a_transmettre);
    assert!(c.ent.voyage_repas_avenant_emettre(&prof, s.id, rev, "  ", true).await.is_err(), "motif obligatoire");
    assert!(c.ent.voyage_repas_avenant_emettre(&prof, s.id, rev, "Désistement", false).await.is_err(), "confirmation obligatoire");
    let v = c.ent.voyage_repas_avenant_emettre(&prof, s.id, rev, "Désistement d'un élève", true).await.unwrap();
    assert_eq!(v.commande.as_ref().unwrap().numero_avis, 1);
    assert_eq!(v.avis.len(), 2);
    assert_eq!(v.avis[1].type_, "avenant");
    assert!(v.avis[1].objet.contains("avenant n° 1") && v.avis[1].objet.contains("annule et remplace"));
    assert!(!v.a_transmettre);
    assert_eq!(recus_vers(&c, &boite), 2);
    // Le même avenant ne se renvoie pas (révision déjà transmise).
    let rev2 = v.commande.as_ref().unwrap().rev;
    assert!(c.ent.voyage_repas_avenant_emettre(&prof, s.id, rev2, "Encore", true).await.is_err());
    // Convives externes : un libellé et un nombre, aucune donnée personnelle ; un jour hors sortie est refusé.
    let mut s2 = commande(&c, Some(rev2), 2, 0);
    s2.externes = vec![ExterneSaisie { jour: c.jour, libelle: "Parents accompagnateurs".into(), nombre: 1, variante: None }];
    let e = c.ent.voyage_repas_ecrire(&prof, s.id, s2).await.unwrap_err();
    assert!(est_refus(&e), "un convive externe de plus dépasse le plafond de paniers : {e:?}");
    let mut hors = commande(&c, Some(rev2), 2, 0);
    hors.lignes[0].jour = c.jour + Duration::days(30);
    assert!(est_refus(&c.ent.voyage_repas_ecrire(&prof, s.id, hors).await.unwrap_err()));
}

#[tokio::test]
async fn restauration_droits_et_cloisonnement_entre_etablissements() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let s = a.ouverte_avec_eleves("Sortie A").await;
    let rev = a.ent.voyage_repas_ecrire(&a.prof_a(), s.id, commande(&a, None, 2, 1)).await.unwrap().commande.unwrap().rev;
    // Lisent : le porteur et la direction. Ne savent même pas que la commande existe : encadrant non porteur,
    // comptabilité, accueil, famille, et tout profil d'un autre établissement.
    assert!(a.ent.voyage_repas_lire(&a.dir_a(), s.id).await.is_ok());
    for (acteur, nom) in [
        (a.a(a.prof2, Role::Prof), "encadrant non porteur"),
        (a.a(a.compta, Role::Compta), "comptabilité"),
        (a.a(a.accueil, Role::Accueil), "accueil"),
        (a.a(a.parent, Role::Parent), "famille"),
        (b.a(b.dir, Role::Direction), "autre établissement"),
        (b.a(b.prof, Role::Prof), "professeur d'un autre établissement"),
    ] {
        let e = a.ent.voyage_repas_lire(&acteur, s.id).await.unwrap_err();
        assert!(est_introuvable(&e), "{nom} : {e:?}");
        let e = a.ent.voyage_repas_ecrire(&acteur, s.id, commande(&a, Some(rev), 9, 9)).await.unwrap_err();
        assert!(est_introuvable(&e), "{nom} (écriture) : {e:?}");
    }
    // La commande de A est invisible de B jusque dans la base (RLS), et les avis de B ne partent jamais chez A.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_repas_commande WHERE etablissement_id = $1").bind(b.etab).fetch_one(b.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_tables_de_la_restauration_et_des_reponses() {
    crate::tests_rls_lots::verifier_tables(&[
        "ent_sortie_repas_commande",
        "ent_sortie_repas_ligne",
        "ent_sortie_repas_attribution",
        "ent_sortie_repas_externe",
        "ent_sortie_repas_avis",
        "ent_sortie_reponse_cle",
    ])
    .await;
}

// ═════════════════════════════ Journal de voyage ═════════════════════════════

pub(crate) fn message(texte: &str, photos: Vec<PhotoSaisie>, aucun: bool) -> PublicationSaisie {
    PublicationSaisie { type_: "message".into(), nature_point: None, texte: texte.into(), aucun_eleve_identifiable: aucun, photos }
}

pub(crate) fn photo(eleves: Vec<Uuid>) -> PhotoSaisie {
    PhotoSaisie { nom: "groupe.png".into(), mime: Some("image/png".into()), contenu: png(), eleves }
}

#[tokio::test]
async fn vd_11_12_journal_photos_filtrees_par_le_droit_a_l_image_et_audience_recalculee() {
    let Some(c) = cadre().await else { return };
    let s = c.ouverte_avec_eleves("Voyage en Alsace").await;
    let prof = c.prof_a();
    let parent0 = c.parent; // titulaire de l'élève 0
    let parent1 = c.parent_de(c.eleves[1]).await;
    // Le journal ne s'ouvre pas deux fois ; avant l'ouverture, aucune publication.
    assert!(c.ent.voyage_journal_publier(&prof, s.id, message("Bonjour", vec![], false)).await.is_err());
    let j = c.ent.voyage_journal_activer(&prof, s.id).await.unwrap();
    assert!(j.actif);
    assert!(c.ent.voyage_journal_activer(&prof, s.id).await.is_err());
    // Une photo sans élève marqué exige la déclaration « aucun élève identifiable » ; marquer ET déclarer est incohérent.
    assert!(est_refus(&c.ent.voyage_journal_publier(&prof, s.id, message("Paysage", vec![photo(vec![])], false)).await.unwrap_err()));
    assert!(est_refus(&c.ent.voyage_journal_publier(&prof, s.id, message("Paysage", vec![photo(vec![c.eleves[0]])], true)).await.unwrap_err()));
    // Un élève marqué qui ne participe pas est refusé (autre établissement : même réponse).
    assert!(est_refus(&c.ent.voyage_journal_publier(&prof, s.id, message("X", vec![photo(vec![Uuid::new_v4()])], false)).await.unwrap_err()));
    // Fichier actif refusé (type détecté sur les octets, jamais sur le nom).
    let mut piege = photo(vec![]);
    piege.contenu = b"<svg onload=alert(1)></svg>".to_vec();
    assert!(est_refus(&c.ent.voyage_journal_publier(&prof, s.id, message("Piège", vec![piege], true)).await.unwrap_err()));
    // Publication : photo de l'élève 0 seul, puis de 0 et 1 ensemble, puis paysage déclaré sans élève.
    let _seul = c.ent.voyage_journal_publier(&prof, s.id, message("Départ !", vec![photo(vec![c.eleves[0]])], false)).await.unwrap();
    let duo = c.ent.voyage_journal_publier(&prof, s.id, message("Au sommet", vec![photo(vec![c.eleves[0], c.eleves[1]])], false)).await.unwrap();
    let paysage = c.ent.voyage_journal_publier(&prof, s.id, message("Le lac", vec![photo(vec![])], true)).await.unwrap();
    // R-SV40 : sans aucune décision, une photo avec élève est MASQUÉE (« inconnu » n'est pas un consentement).
    let f0 = c.ent.voyages_journal_famille(&c.a(parent0, Role::Parent)).await.unwrap();
    assert_eq!(f0.len(), 1);
    let vues = |f: &[crate::voyages_journal_repo::JournalFamille]| -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> = f[0].publications.iter().map(|p| (p.texte.clone(), p.photos.len())).collect();
        v.sort();
        v
    };
    assert_eq!(vues(&f0), vec![("Au sommet".into(), 0), ("Départ !".into(), 0), ("Le lac".into(), 1)]);
    assert_eq!(f0[0].auteur, "L'équipe du voyage", "aucun nom de personnel");
    // Le texte reste, la photo non montrée : côté auteur, le nombre de photos masquées est connu.
    let g = c.ent.voyage_journal_lire(&prof, s.id).await.unwrap();
    assert_eq!(g.publications.iter().map(|p| p.photos_masquees.unwrap()).sum::<i64>(), 2);
    // Consentement du parent 0 : la photo de l'élève 0 seul apparaît ; le duo reste masqué (l'élève 1 n'a rien décidé).
    let f = c.formulaire_image().await;
    c.decider_image(f, parent0, c.eleves[0], "accepte").await;
    let f0 = c.ent.voyages_journal_famille(&c.a(parent0, Role::Parent)).await.unwrap();
    assert_eq!(vues(&f0), vec![("Au sommet".into(), 0), ("Départ !".into(), 1), ("Le lac".into(), 1)]);
    // Le parent 1 consent aussi : le duo apparaît. Un refus ultérieur le fait disparaître (un seul refus suffit).
    c.decider_image(f, parent1, c.eleves[1], "accepte").await;
    let f0 = c.ent.voyages_journal_famille(&c.a(parent0, Role::Parent)).await.unwrap();
    assert_eq!(vues(&f0), vec![("Au sommet".into(), 1), ("Départ !".into(), 1), ("Le lac".into(), 1)]);
    c.decider_image(f, parent1, c.eleves[1], "retire").await;
    let f0 = c.ent.voyages_journal_famille(&c.a(parent0, Role::Parent)).await.unwrap();
    assert_eq!(vues(&f0), vec![("Au sommet".into(), 0), ("Départ !".into(), 1), ("Le lac".into(), 1)]);
    // Photo : servie à qui la voit, jamais par simple identifiant (photo masquée, autre sortie, autre famille).
    let f0 = c.ent.voyages_journal_famille(&c.a(parent0, Role::Parent)).await.unwrap();
    let photo_seul = f0[0].publications.iter().find(|p| p.texte == "Départ !").unwrap().photos[0].id;
    let (mime, octets) = c.ent.voyage_journal_photo(&c.a(parent0, Role::Parent), s.id, photo_seul).await.unwrap();
    assert_eq!((mime.as_str(), octets), ("image/png", png()));
    let e = c.ent.voyage_journal_photo(&c.a(parent0, Role::Parent), s.id, duo.photos.first().map(|p| p.id).unwrap_or_else(Uuid::new_v4)).await;
    assert!(e.is_err(), "photo masquée : jamais servie");
    // Audience : la famille de l'élève 2 (aucune décision, aucun lien) ne voit rien du journal des autres ;
    // celle de l'élève 0 le voit tant que l'élève 0 part encore.
    let autre = c.parent_de(c.eleves[2]).await;
    assert_eq!(c.ent.voyages_journal_famille(&c.a(autre, Role::Parent)).await.unwrap().len(), 1, "ses publications (texte)");
    let inconnu = compte(&c.ent, c.etab, "parent").await;
    assert!(c.ent.voyages_journal_famille(&c.a(inconnu, Role::Parent)).await.unwrap().is_empty());
    assert!(est_introuvable(&c.ent.voyage_journal_photo(&c.a(inconnu, Role::Parent), s.id, paysage.photos[0].id).await.unwrap_err()));
    let p0 = c.participant(s.id, c.eleves[0]).await;
    c.ent.voyage_participant_desister(&prof, s.id, p0, "famille", None).await.unwrap();
    assert!(c.ent.voyages_journal_famille(&c.a(parent0, Role::Parent)).await.unwrap().is_empty(), "élève désisté = accès retiré");
    assert!(est_introuvable(&c.ent.voyage_journal_photo(&c.a(parent0, Role::Parent), s.id, paysage.photos[0].id).await.unwrap_err()));
    // Retrait d'une publication par son auteur : disparue des familles, présente côté gestion (marquée retirée).
    c.ent.voyage_journal_publication_retirer(&prof, s.id, paysage.id).await.unwrap();
    let f2 = c.ent.voyages_journal_famille(&c.a(autre, Role::Parent)).await.unwrap();
    assert!(f2[0].publications.iter().all(|p| p.texte != "Le lac"));
    assert!(c.ent.voyage_journal_lire(&prof, s.id).await.unwrap().publications.iter().any(|p| p.retiree == Some(true)));
    // Fermeture : plus aucune lecture famille ; plus de publication.
    c.ent.voyage_journal_fermer(&prof, s.id).await.unwrap();
    assert!(c.ent.voyages_journal_famille(&c.a(autre, Role::Parent)).await.unwrap().is_empty());
    assert!(c.ent.voyage_journal_publier(&prof, s.id, message("Trop tard", vec![], false)).await.is_err());
}

#[tokio::test]
async fn journal_points_de_situation_codes_et_droits_de_publication() {
    let Some(c) = cadre().await else { return };
    let s = c.ouverte_avec_eleves("Voyage").await;
    let prof = c.prof_a();
    c.ent.voyage_journal_activer(&prof, s.id).await.unwrap();
    // Point de situation codé : jamais de texte libre long ni de photo (donc jamais d'incident ni de nom).
    let point = |nature: &str, texte: &str| PublicationSaisie {
        type_: "point_de_situation".into(),
        nature_point: Some(nature.into()),
        texte: texte.into(),
        aucun_eleve_identifiable: false,
        photos: vec![],
    };
    assert!(c.ent.voyage_journal_publier(&prof, s.id, point("arrivee", "Bien arrivés.")).await.is_ok());
    assert!(c.ent.voyage_journal_publier(&prof, s.id, point("incident_medical", "")).await.is_err());
    assert!(c.ent.voyage_journal_publier(&prof, s.id, point("depart", &"x".repeat(301))).await.is_err());
    let mut avec_photo = point("depart", "");
    avec_photo.photos = vec![photo(vec![])];
    assert!(c.ent.voyage_journal_publier(&prof, s.id, avec_photo).await.is_err());
    // Droits : l'encadrant non délégué, la comptabilité, l'accueil, la famille ne publient pas (« introuvable »).
    for (acteur, nom) in [
        (c.a(c.prof2, Role::Prof), "encadrant non délégué"),
        (c.a(c.compta, Role::Compta), "comptabilité"),
        (c.a(c.accueil, Role::Accueil), "accueil"),
        (c.a(c.parent, Role::Parent), "famille"),
    ] {
        let e = c.ent.voyage_journal_publier(&acteur, s.id, message("Intrus", vec![], false)).await.unwrap_err();
        assert!(est_introuvable(&e), "{nom} : {e:?}");
    }
    // Délégué : un encadrant DE LA SORTIE (jamais un nom libre) ; il publie alors, et seulement alors.
    let enc = c.encadrant(s.id).await;
    assert!(est_refus(&c.ent.voyage_journal_delegues_definir(&prof, s.id, vec![Uuid::new_v4()]).await.unwrap_err()));
    assert!(est_forbidden(&c.ent.voyage_journal_delegues_definir(&c.a(c.prof2, Role::Prof), s.id, vec![enc]).await.unwrap_err()), "l'encadrant connaît la sortie : interdit, pas introuvable");
    let j = c.ent.voyage_journal_delegues_definir(&prof, s.id, vec![enc]).await.unwrap();
    assert_eq!(j.delegues.len(), 1);
    let p = c.ent.voyage_journal_publier(&c.a(c.prof2, Role::Prof), s.id, message("De la part du délégué", vec![], false)).await.unwrap();
    // Un délégué ne retire que ses propres publications ; l'organisateur retire toutes.
    assert!(c.ent.voyage_journal_publication_retirer(&c.a(c.prof2, Role::Prof), s.id, p.id).await.is_ok());
    // La direction (voyages.modifier) gère le journal même sans en être l'organisatrice.
    assert!(c.ent.voyage_journal_lire(&c.dir_a(), s.id).await.is_ok());
}

struct Collecte(Mutex<Vec<(Uuid, String)>>);

impl DistributionLien for Collecte {
    fn distribuer(&self, _etab: Uuid, _sortie: Uuid, personne: Uuid, jeton: &str) {
        self.0.lock().unwrap().push((personne, jeton.to_string()));
    }
}

#[tokio::test]
async fn vd_10_43_lien_nominatif_revocable_expirant_limite_et_jamais_conserve() {
    let (Some(c), Some(autre)) = (cadre().await, cadre().await) else { return };
    let s = c.ouverte_avec_eleves("Voyage Londres").await;
    let prof = c.prof_a();
    c.ent.voyage_journal_activer(&prof, s.id).await.unwrap();
    c.ent.voyage_journal_publier(&prof, s.id, message("Bonne arrivée", vec![], false)).await.unwrap();
    let personne1 = c.titulaire_sans_compte(c.eleves[1]).await;
    let personne2 = c.titulaire_sans_compte(c.eleves[2]).await;
    // Seul un titulaire SANS compte reçoit un lien (celui qui a un compte lit dans son espace famille).
    let port = Collecte(Mutex::new(vec![]));
    assert!(est_introuvable(&c.ent.voyage_journal_liens_emettre(&c.a(c.accueil, Role::Accueil), s.id, &port).await.unwrap_err()));
    assert_eq!(c.ent.voyage_journal_liens_emettre(&prof, s.id, &port).await.unwrap(), 2);
    assert_eq!(c.ent.voyage_journal_liens_emettre(&prof, s.id, &port).await.unwrap(), 0, "jamais de doublon d'un lien actif");
    let liens = port.0.lock().unwrap().clone();
    assert_eq!(liens.len(), 2);
    let jeton1 = liens.iter().find(|l| l.0 == personne1).unwrap().1.clone();
    let jeton2 = liens.iter().find(|l| l.0 == personne2).unwrap().1.clone();
    assert_ne!(jeton1, jeton2);
    // Le jeton n'est jamais conservé : ni en clair dans la base, ni dans les journaux.
    let en_clair: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_journal_lien WHERE jeton_hash = $1 OR jeton_hash = $2")
        .bind(&jeton1)
        .bind(&jeton2)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(en_clair, 0);
    let journal = crate::tests_voyages_v0a::actions_journal(&c, s.id).await.join(",");
    assert!(!journal.contains(&jeton1));
    // Lecture : le journal, sans nom de personnel ni liste d'élèves ; le lien d'une famille ne mène pas ailleurs.
    let j = c.ent.voyage_journal_par_lien(&jeton1).await.unwrap();
    assert_eq!(j.sortie_id, s.id);
    assert_eq!(j.auteur, "L'équipe du voyage");
    assert_eq!(j.publications.len(), 1);
    assert!(j.publications[0].retiree.is_none() && j.publications[0].photos_masquees.is_none());
    // Jetons mal formés, inconnus, ou d'un autre établissement : même réponse « introuvable ».
    for mauvais in ["".to_string(), "abc".to_string(), format!("{}.{}", c.etab.simple(), "0".repeat(64)), format!("{}.{}", autre.etab.simple(), jeton1.split_once('.').unwrap().1)] {
        let e = c.ent.voyage_journal_par_lien(&mauvais).await.unwrap_err();
        assert!(est_introuvable(&e), "{mauvais} : {e:?}");
        assert!(est_introuvable(&autre.ent.voyage_journal_par_lien(&mauvais).await.unwrap_err()));
    }
    // Limitation de débit : au plus 30 lectures par fenêtre de 10 minutes, ensuite refus.
    let mut refuse = false;
    for _ in 0..40 {
        if c.ent.voyage_journal_par_lien(&jeton2).await.is_err() {
            refuse = true;
            break;
        }
    }
    assert!(refuse, "le lien est limité en débit");
    // Révocation par titulaire : le lien de l'autre vit encore ; le lien révoqué ne lit plus.
    assert_eq!(c.ent.voyage_journal_liens_revoquer(&prof, s.id, Some(personne1)).await.unwrap(), 1);
    assert!(est_introuvable(&c.ent.voyage_journal_par_lien(&jeton1).await.unwrap_err()));
    // Perte d'autorité parentale du titulaire : le lien ne lit plus (CV-69), sans rien révoquer à la main.
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = false WHERE personne_id = $1").bind(personne2).execute(c.ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_sortie_journal_lien SET fenetre_nombre = 0, fenetre_debut = NULL WHERE personne_id = $1").bind(personne2).execute(c.ent.pool()).await.unwrap();
    assert!(est_introuvable(&c.ent.voyage_journal_par_lien(&jeton2).await.unwrap_err()));
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = true WHERE personne_id = $1").bind(personne2).execute(c.ent.pool()).await.unwrap();
    assert!(c.ent.voyage_journal_par_lien(&jeton2).await.is_ok());
    // Élève désisté : accès retiré. Expiration (fin du séjour + 15 jours) : accès retiré.
    let p2 = c.participant(s.id, c.eleves[2]).await;
    c.ent.voyage_participant_desister(&prof, s.id, p2, "famille", None).await.unwrap();
    assert!(est_introuvable(&c.ent.voyage_journal_par_lien(&jeton2).await.unwrap_err()));
    sqlx::query("UPDATE ent_sortie_journal_lien SET expire_le = now() - interval '1 minute' WHERE personne_id = $1").bind(personne2).execute(c.ent.pool()).await.unwrap();
    assert!(est_introuvable(&c.ent.voyage_journal_par_lien(&jeton2).await.unwrap_err()));
    // Un lien émis expire par construction à la fin du séjour + 15 jours.
    let (fin, expire): (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
        "SELECT s.fin, l.expire_le FROM ent_sortie_journal_lien l JOIN ent_sortie s ON s.id = l.sortie_id WHERE l.personne_id = $1",
    )
    .bind(personne1)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(expire - fin, Duration::days(15));
}

#[tokio::test]
async fn journal_purge_a_fin_plus_15_jours_sauf_la_selection_du_bilan() {
    let Some(c) = cadre().await else { return };
    let s = c.ouverte_avec_eleves("Voyage Rome").await;
    let prof = c.prof_a();
    c.ent.voyage_journal_activer(&prof, s.id).await.unwrap();
    let gardee = c.ent.voyage_journal_publier(&prof, s.id, message("À garder", vec![photo(vec![])], true)).await.unwrap();
    let perdue = c.ent.voyage_journal_publier(&prof, s.id, message("À détruire", vec![photo(vec![c.eleves[0]])], false)).await.unwrap();
    // La sélection du bilan n'accepte aucune photo d'un élève non consentant.
    assert!(c.ent.voyage_journal_publication_garder(&prof, s.id, perdue.id, true).await.is_err());
    c.ent.voyage_journal_publication_garder(&prof, s.id, gardee.id, true).await.unwrap();
    // Avant l'échéance : rien n'est détruit.
    assert_eq!(c.ent.voyages_journal_purger(c.etab).await.unwrap(), 0);
    // Fin du séjour il y a 16 jours.
    sqlx::query("UPDATE ent_sortie SET debut = now() - interval '20 days', fin = now() - interval '16 days' WHERE id = $1").bind(s.id).execute(c.ent.pool()).await.unwrap();
    assert_eq!(c.ent.voyages_journal_purger(c.etab).await.unwrap(), 1);
    let j = c.ent.voyage_journal_lire(&prof, s.id).await.unwrap();
    assert_eq!(j.publications.len(), 1);
    assert_eq!(j.publications[0].texte, "À garder");
    assert!(!j.actif && j.ferme_le.is_some());
    // Les photos détruites sont supprimées logiquement au registre des objets (jamais de DELETE) et leur marquage effacé.
    let marques: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_publication_photo_eleve pe JOIN ent_sortie_publication_photo ph ON ph.id = pe.photo_id JOIN ent_sortie_publication p ON p.id = ph.publication_id WHERE p.sortie_id = $1")
        .bind(s.id)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(marques, 0);
    let supprimes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_objet WHERE etablissement_id = $1 AND cle_objet LIKE '%/journal_voyage/%' AND supprime_le IS NOT NULL")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(supprimes, 1);
    // Rejouer la purge ne détruit plus rien.
    assert_eq!(c.ent.voyages_journal_purger(c.etab).await.unwrap(), 0);
    assert!(crate::tests_voyages_v0a::actions_journal(&c, s.id).await.contains(&"journal_purge".to_string()));
}

#[tokio::test]
async fn journal_cloisonne_entre_etablissements() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let s = a.ouverte_avec_eleves("Voyage A").await;
    let prof = a.prof_a();
    a.ent.voyage_journal_activer(&prof, s.id).await.unwrap();
    let p = a.ent.voyage_journal_publier(&prof, s.id, message("Privé", vec![photo(vec![])], true)).await.unwrap();
    for acteur in [b.a(b.dir, Role::Direction), b.a(b.prof, Role::Prof), b.a(b.parent, Role::Parent)] {
        assert!(est_introuvable(&a.ent.voyage_journal_lire(&acteur, s.id).await.unwrap_err()));
        assert!(est_introuvable(&a.ent.voyage_journal_photo(&acteur, s.id, p.photos[0].id).await.unwrap_err()));
        assert!(a.ent.voyage_journal_publier(&acteur, s.id, message("Intrus", vec![], false)).await.is_err());
        assert!(a.ent.voyage_journal_fermer(&acteur, s.id).await.is_err());
    }
    assert!(b.ent.voyages_journal_famille(&b.a(b.parent, Role::Parent)).await.unwrap().is_empty());
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_tables_du_journal_de_voyage() {
    crate::tests_rls_lots::verifier_tables(&[
        "ent_sortie_journal_voyage",
        "ent_sortie_journal_delegue",
        "ent_sortie_publication",
        "ent_sortie_publication_photo",
        "ent_sortie_publication_photo_eleve",
        "ent_sortie_journal_lien",
    ])
    .await;
}

// ═════════════════════════════ Reprise des séjours de Next.js ═════════════════════════════

pub(crate) fn params(c: &Cadre) -> ParamsReprise {
    // Gel : dans trois jours moins un ; fin de la période « en vol » : dix jours après.
    ParamsReprise { gel: c.jour - Duration::days(1), fin_vol: c.jour + Duration::days(10) }
}

pub(crate) fn sejour(c: &Cadre, source: &str, statut: &str, jours: i64, participants: Vec<ParticipantImport>) -> SejourImport {
    SejourImport {
        source_id: source.into(),
        statut_source: statut.into(),
        statut_precedent: None,
        type_source: "SIMPLE".into(),
        titre: format!("Séjour {source}"),
        debut: Utc.from_utc_datetime(&(c.jour + Duration::days(jours)).and_hms_opt(8, 0, 0).unwrap()),
        fin: Utc.from_utc_datetime(&(c.jour + Duration::days(jours)).and_hms_opt(16, 0, 0).unwrap()),
        nature: None,
        nuitees: false,
        besoin_car: false,
        classes_texte: Some("4A".into()),
        porteur_personne_id: None,
        budget_total: Some("1 350,50".into()),
        part_annoncee: Some("45".into()),
        part_recalculee: None,
        pieces_attendues: 0,
        aides_nominatives: 0,
        participants,
    }
}

pub(crate) fn part(source: &str, nom: &str, ine: Option<&str>, eleve: Option<Uuid>) -> ParticipantImport {
    ParticipantImport {
        source_id: source.into(),
        nom: Some(nom.into()),
        prenom: Some("Prénom".into()),
        ine: ine.map(String::from),
        classe: Some("4A".into()),
        eleve_id: eleve,
    }
}

pub(crate) async fn admin(c: &Cadre) -> Acteur<'static> {
    let u = compte(&c.ent, c.etab, "admin").await;
    c.a(u, Role::Admin)
}

async fn ine_de(c: &Cadre, eleve: Uuid, ine: &str) {
    sqlx::query("UPDATE ent_eleve SET ine = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(c.etab)
        .bind(eleve)
        .bind(ine)
        .execute(c.ent.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn reprise_droits_import_idempotent_et_rapprochement_sans_approximation() {
    let Some(c) = cadre().await else { return };
    let adm = admin(&c).await;
    ine_de(&c, c.eleves[1], "12345678AB1").await;
    // Un INE porté par deux élèves ne rapproche personne (file humaine).
    ine_de(&c, c.eleves[0], "77777777DD7").await;
    ine_de(&c, c.eleves[2], "77777777DD7").await;
    let ps = vec![
        part("p-id", "Dupont", None, Some(c.eleves[0])),
        part("p-ine", "Martin", Some("12345678ab1"), None),
        part("p-inconnu", "Inconnu", Some("00000000XX0"), None),
        part("p-sans", "Sansine", None, None),
        part("p-ineko", "Mauvais", Some("123"), None),
        part("p-dup", "Doublon", Some("77777777dd7"), None),
    ];
    let s = sejour(&c, "NX-1", "PROF_LOGISTICS", 20, ps.clone());
    // Droits : l'administrateur et la direction seulement ; ni secrétariat, ni professeur, ni famille.
    for (a, nom) in [(c.a(c.sec, Role::Administratif), "secrétariat"), (c.prof_a(), "professeur"), (c.a(c.compta, Role::Compta), "comptabilité"), (c.a(c.parent, Role::Parent), "famille")] {
        let e = c.ent.voyages_reprise_importer(&a, params(&c), vec![s.clone()]).await.unwrap_err();
        assert!(est_forbidden(&e), "{nom} : {e:?}");
        assert!(est_forbidden(&c.ent.voyages_reprise_tableau(&a).await.unwrap_err()), "{nom}");
    }
    let r = c.ent.voyages_reprise_importer(&adm, params(&c), vec![s.clone()]).await.unwrap();
    assert_eq!((r.crees, r.mis_a_jour), (1, 0));
    assert_eq!(r.participants_rapproches, 2, "identifiant connu et INE unique");
    assert_eq!(r.participants_a_rapprocher, 4, "inconnu, sans INE, INE invalide, INE partagé par deux élèves");
    // Idempotent : rejouer ne crée rien, ne double rien, ne perd rien.
    let r2 = c.ent.voyages_reprise_importer(&adm, params(&c), vec![s.clone()]).await.unwrap();
    assert_eq!((r2.crees, r2.mis_a_jour, r2.participants_rapproches, r2.participants_a_rapprocher), (0, 1, 2, 4));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_reprise_participant WHERE etablissement_id = $1").bind(c.etab).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(n, 6);
    // Minimisation : l'identité de l'export est effacée dès le rapprochement ; la contrainte de base la garantit.
    let identites: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_reprise_participant WHERE etablissement_id = $1 AND statut = 'rapproche' AND (nom_source IS NOT NULL OR ine_source IS NOT NULL)")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(identites, 0);
    let r = sqlx::query("UPDATE ent_sortie_reprise_participant SET nom_source = 'X' WHERE statut = 'rapproche' AND etablissement_id = $1").bind(c.etab).execute(c.ent.pool()).await;
    assert!(r.is_err(), "un participant rapproché ne garde pas l'identité de l'export");
    // Montants en centimes ; l'écart « porteur à désigner » est signalé.
    let t = c.ent.voyages_reprise_tableau(&adm).await.unwrap();
    assert_eq!(t[0].budget_total_centimes, Some(135_050));
    assert_eq!(t[0].part_annoncee_centimes, Some(4_500));
    assert!(!t[0].basculable);
    // File humaine : un élève ne figure qu'une fois par séjour ; un élève d'un autre établissement n'existe pas.
    let id = t[0].id;
    assert!(c.ent.voyages_reprise_rapprocher(&adm, id, "p-sans", c.eleves[0]).await.is_err(), "déjà rapproché dans ce séjour");
    assert!(est_introuvable(&c.ent.voyages_reprise_rapprocher(&adm, id, "p-sans", Uuid::new_v4()).await.unwrap_err()));
    c.ent.voyages_reprise_rapprocher(&adm, id, "p-sans", c.eleves[2]).await.unwrap();
    assert_eq!(c.ent.voyages_reprise_a_rapprocher(&adm, id).await.unwrap().len(), 3);
}

#[tokio::test]
async fn reprise_montants_douteux_aides_et_part_annoncee_signales_jamais_devines() {
    let Some(c) = cadre().await else { return };
    let adm = admin(&c).await;
    let mut s = sejour(&c, "NX-2", "EN_ATTENTE_COMPTA", 20, vec![]);
    s.budget_total = Some("1.350".into()); // « 1.350 » n'est PAS 1,35 €
    s.part_annoncee = Some("45".into());
    s.part_recalculee = Some("43,50".into());
    s.aides_nominatives = 2;
    let r = c.ent.voyages_reprise_importer(&adm, params(&c), vec![s]).await.unwrap();
    assert_eq!(r.crees, 1);
    let t = c.ent.voyages_reprise_tableau(&adm).await.unwrap();
    assert_eq!(t[0].budget_total_centimes, None, "montant douteux jamais deviné");
    assert_eq!(t[0].part_annoncee_centimes, Some(4_500), "la part annoncée est conservée telle quelle (R-SV46)");
    assert_eq!(t[0].etat_cible, "budget_a_valider");
    let ec = c.ent.voyages_reprise_ecarts(&adm, t[0].id).await.unwrap();
    let codes: Vec<&str> = ec.iter().map(|e| e.code.as_str()).collect();
    for attendu in ["montant_douteux", "part_annoncee_differente", "aide_nominative_retiree", "porteur_a_designer"] {
        assert!(codes.contains(&attendu), "{attendu} manque : {codes:?}");
    }
    // Les aides nominatives ne sont pas reprises : seul leur nombre est signalé, aucun nom de personne.
    assert!(ec.iter().any(|e| e.code == "aide_nominative_retiree" && e.detail.starts_with("2 aide(s)")), "{ec:?}");
    // Statut inconnu : refusé et listé, jamais deviné.
    let mut inconnu = sejour(&c, "NX-3", "STATUT_BIZARRE", 21, vec![]);
    inconnu.statut_source = "STATUT_BIZARRE".into();
    let r = c.ent.voyages_reprise_importer(&adm, params(&c), vec![inconnu]).await.unwrap();
    assert_eq!((r.crees, r.statuts_inconnus.len()), (0, 1));
}

pub(crate) async fn pret_a_basculer(c: &Cadre, adm: &Acteur<'_>, source: &str, statut: &str, jours: i64) -> Uuid {
    let ps = vec![part("a", "Alpha", None, Some(c.eleves[0])), part("b", "Bravo", None, Some(c.eleves[1]))];
    let mut s = sejour(c, source, statut, jours, ps);
    s.budget_total = Some("1 000".into());
    s.part_annoncee = None;
    c.ent.voyages_reprise_importer(adm, params(c), vec![s]).await.unwrap();
    let t = c.ent.voyages_reprise_tableau(adm).await.unwrap();
    let l = t.into_iter().find(|l| l.source_id == source).unwrap();
    // Le porteur est désigné par un humain (levée de l'écart « porteur à désigner »).
    let porteur = crate::tests_voyages_v0a::personne_de(&c.ent, c.etab, c.prof).await;
    c.ent.voyages_reprise_porteur_definir(adm, l.id, porteur).await.unwrap();
    l.id
}

#[tokio::test]
async fn r_sv37_bascule_tableau_derogation_et_propriete_unique() {
    let Some(c) = cadre().await else { return };
    let adm = admin(&c).await;
    let id = pret_a_basculer(&c, &adm, "NX-10", "FINALISE_DIR_ATTENTE_ELEVES", 20).await;
    // Tout vert sauf rien : basculable. Confirmation expresse obligatoire.
    let l = c.ent.voyages_reprise_tableau(&adm).await.unwrap().into_iter().find(|l| l.id == id).unwrap();
    assert!(l.basculable, "{:?}", l.controles);
    assert_eq!(l.attestations, None, "les attestations papier se suivent une fois le séjour basculé (lot V1)");
    assert!(c.ent.voyages_reprise_basculer(&adm, id, false).await.is_err());
    // Le droit seul ne suffit pas à basculer un séjour qui a des participants à rapprocher.
    let mut rouge = sejour(&c, "NX-11", "VALIDE", 22, vec![part("z", "Zoé", None, None)]);
    rouge.porteur_personne_id = Some(crate::tests_voyages_v0a::personne_de(&c.ent, c.etab, c.prof).await);
    c.ent.voyages_reprise_importer(&adm, params(&c), vec![rouge]).await.unwrap();
    let l_rouge = c.ent.voyages_reprise_tableau(&adm).await.unwrap().into_iter().find(|l| l.source_id == "NX-11").unwrap();
    assert!(!l_rouge.basculable);
    assert!(c.ent.voyages_reprise_basculer(&adm, l_rouge.id, true).await.is_err());
    // La dérogation motivée ne lève JAMAIS un participant non rapproché.
    c.ent.voyages_reprise_deroger(&adm, l_rouge.id, "décision de Florian").await.unwrap();
    assert!(c.ent.voyages_reprise_basculer(&adm, l_rouge.id, true).await.is_err());
    assert!(c.ent.voyages_reprise_deroger(&adm, l_rouge.id, "   ").await.is_err(), "motif obligatoire");
    // Bascule du séjour vert : la sortie ScolIA existe, en l'état de la correspondance (§32.3).
    let sortie = c.ent.voyages_reprise_basculer(&adm, id, true).await.unwrap();
    let (etat, nb, conf): (String, i64, bool) = sqlx::query_as(
        "SELECT s.etat, (SELECT count(*) FROM ent_sortie_participant WHERE sortie_id = s.id), s.liste_confirmee_le IS NOT NULL
           FROM ent_sortie s WHERE s.id = $1",
    )
    .bind(sortie)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((etat.as_str(), nb, conf), ("ouverte", 2, true));
    assert!(crate::tests_voyages_v0a::actions_journal(&c, sortie).await.contains(&"sortie_reprise".to_string()));
    // Propriété unique : un séjour déjà à ScolIA ne rebascule pas, et la reprise différentielle n'y écrit JAMAIS ;
    // un écart constaté est signalé, pas appliqué.
    assert!(c.ent.voyages_reprise_basculer(&adm, id, true).await.is_err());
    let mut modifie = sejour(&c, "NX-10", "ANNULE", 25, vec![]);
    modifie.titre = "Titre modifié dans Next.js".into();
    let r = c.ent.voyages_reprise_importer(&adm, params(&c), vec![modifie]).await.unwrap();
    assert_eq!((r.crees, r.mis_a_jour, r.intacts_scolia, r.ecarts), (0, 0, 1, 1));
    let titre: String = sqlx::query_scalar("SELECT titre FROM ent_sortie WHERE id = $1").bind(sortie).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(titre, "Séjour NX-10");
    assert_eq!(c.ent.voyages_reprise_proprietaire(c.etab, "NX-10").await.unwrap().as_deref(), Some("scolia"));
    let ec = c.ent.voyages_reprise_ecarts(&adm, id).await.unwrap();
    assert!(ec.iter().any(|e| e.code == "ecart_apres_bascule"));
    // Retour arrière sans écriture métier : Next.js redevient propriétaire, la sortie est annulée (jamais supprimée).
    c.ent.voyages_reprise_retour_arriere(&adm, id).await.unwrap();
    assert_eq!(c.ent.voyages_reprise_proprietaire(c.etab, "NX-10").await.unwrap().as_deref(), Some("nextjs"));
    let etat: String = sqlx::query_scalar("SELECT etat FROM ent_sortie WHERE id = $1").bind(sortie).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(etat, "annulee");
    // Nouvelle bascule : l'écart signalé après la première doit d'abord être TRAITÉ par un humain (jamais ignoré).
    assert!(c.ent.voyages_reprise_basculer(&adm, id, true).await.is_err());
    for e in c.ent.voyages_reprise_ecarts(&adm, id).await.unwrap().iter().filter(|e| e.traite_le.is_none()) {
        c.ent.voyages_reprise_ecart_traiter(&adm, e.id).await.unwrap();
    }
    let sortie2 = c.ent.voyages_reprise_basculer(&adm, id, true).await.unwrap();
    assert_ne!(sortie, sortie2);
    c.ent.voyage_participants_ajouter(&c.prof_a(), sortie2, vec![c.eleves[2]]).await.ok();
    c.ent.voyage_journal_activer(&c.dir_a(), sortie2).await.unwrap();
    assert!(c.ent.voyages_reprise_retour_arriere(&adm, id).await.is_err(), "des écritures ont eu lieu dans ScolIA");
}

#[tokio::test]
async fn reprise_sejours_en_vol_et_archives_cv_65() {
    let Some(c) = cadre().await else { return };
    let adm = admin(&c).await;
    let porteur = crate::tests_voyages_v0a::personne_de(&c.ent, c.etab, c.prof).await;
    // Parti après le gel, avant la fin de la période : « en vol » ; terminé avant le gel : archive ; plus tard : normal.
    let mut vol = sejour(&c, "NX-20", "VALIDE", 2, vec![]);
    vol.porteur_personne_id = Some(porteur);
    let mut passe = sejour(&c, "NX-21", "VALIDE", -30, vec![]);
    passe.porteur_personne_id = Some(porteur);
    let mut normal = sejour(&c, "NX-22", "VALIDE", 40, vec![]);
    normal.porteur_personne_id = Some(porteur);
    c.ent.voyages_reprise_importer(&adm, params(&c), vec![vol, passe, normal]).await.unwrap();
    let t = c.ent.voyages_reprise_tableau(&adm).await.unwrap();
    let rg = |s: &str| t.iter().find(|l| l.source_id == s).unwrap().clone();
    assert_eq!(rg("NX-20").regime, "en_vol");
    assert_eq!(rg("NX-21").regime, "archive");
    assert_eq!(rg("NX-21").etat_cible, "close", "terminé : repris en archive");
    assert_eq!(rg("NX-22").regime, "normal");
    // Un séjour en vol ne bascule pas, même avec une dérogation (il finit dans Next.js).
    let id = rg("NX-20").id;
    assert!(!rg("NX-20").basculable);
    c.ent.voyages_reprise_deroger(&adm, id, "urgence").await.unwrap();
    assert!(c.ent.voyages_reprise_basculer(&adm, id, true).await.is_err());
    // L'archive bascule en `close`, ses participants « rentrés » (lecture seule).
    let sortie = c.ent.voyages_reprise_basculer(&adm, rg("NX-21").id, true).await.unwrap();
    let etat: String = sqlx::query_scalar("SELECT etat FROM ent_sortie WHERE id = $1").bind(sortie).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(etat, "close");
}

#[tokio::test]
async fn reprise_cloisonnee_entre_etablissements() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let (adm_a, adm_b) = (admin(&a).await, admin(&b).await);
    a.ent.voyages_reprise_importer(&adm_a, params(&a), vec![sejour(&a, "NX-30", "VALIDE", 20, vec![])]).await.unwrap();
    // Même identifiant source dans un autre établissement : un autre séjour, jamais le même.
    b.ent.voyages_reprise_importer(&adm_b, params(&b), vec![sejour(&b, "NX-30", "BROUILLON", 20, vec![])]).await.unwrap();
    assert_eq!(a.ent.voyages_reprise_tableau(&adm_a).await.unwrap().len(), 1);
    let ta = a.ent.voyages_reprise_tableau(&adm_a).await.unwrap();
    let id_a = ta[0].id;
    // Les commandes de B ne touchent pas un séjour de A (RLS : il n'existe pas pour B).
    assert!(b.ent.voyages_reprise_ecarts(&adm_b, id_a).await.unwrap().is_empty());
    assert!(est_introuvable(&b.ent.voyages_reprise_deroger(&adm_b, id_a, "x").await.unwrap_err()));
    assert!(est_introuvable(&b.ent.voyages_reprise_rapprocher(&adm_b, id_a, "p", Uuid::new_v4()).await.unwrap_err()));
    assert!(est_introuvable(&b.ent.voyages_reprise_basculer(&adm_b, id_a, true).await.unwrap_err()));
    let tb = b.ent.voyages_reprise_tableau(&adm_b).await.unwrap();
    assert_eq!((ta.len(), tb.len()), (1, 1));
    assert_ne!(ta[0].id, tb[0].id);
    assert_eq!((ta[0].statut_source.as_str(), tb[0].statut_source.as_str()), ("VALIDE", "BROUILLON"), "le même identifiant source reste deux séjours");
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_tables_de_la_reprise() {
    crate::tests_rls_lots::verifier_tables(&["ent_sortie_reprise_sejour", "ent_sortie_reprise_participant", "ent_sortie_reprise_ecart"]).await;
}

/// Hygiène du schéma V0d : aucune colonne JSON ni BYTEA (D-036) ; clés composites ; RLS posée.
#[tokio::test]
async fn schema_v0d_sans_json_ni_bytea_et_avec_rls() {
    let Some(c) = cadre().await else { return };
    let tables: Vec<String> = [
        "ent_sortie_repas_commande", "ent_sortie_repas_ligne", "ent_sortie_repas_attribution", "ent_sortie_repas_externe",
        "ent_sortie_repas_avis", "ent_sortie_reponse_cle", "ent_sortie_journal_voyage", "ent_sortie_journal_delegue",
        "ent_sortie_publication", "ent_sortie_publication_photo", "ent_sortie_publication_photo_eleve",
        "ent_sortie_journal_lien", "ent_sortie_reprise_sejour", "ent_sortie_reprise_participant", "ent_sortie_reprise_ecart",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let interdits: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name = ANY($1) AND data_type IN ('json', 'jsonb', 'bytea')",
    )
    .bind(&tables)
    .fetch_all(c.ent.pool())
    .await
    .unwrap();
    assert!(interdits.is_empty(), "{interdits:?}");
    for t in &tables {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_constraint WHERE conrelid = ('public.' || $1)::regclass AND contype = 'u' AND pg_get_constraintdef(oid) LIKE '%etablissement_id, id%'")
            .bind(t)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
        assert!(n >= 1, "{t} : clé composite (etablissement_id, id) absente");
    }
}

// ═════════════════════════════ Ports du courrier sortant (D-065) ═════════════════════════════

async fn poser_uai(c: &Cadre) -> String {
    let uai = format!("{:07}{}", Uuid::new_v4().as_u128() % 10_000_000, 'Z');
    sqlx::query("INSERT INTO ent_etablissement_profil (etablissement_id, nom_officiel, uai) VALUES ($1, 'Collège de test', $2)")
        .bind(c.etab)
        .bind(&uai)
        .execute(c.ent.pool())
        .await
        .expect("profil");
    uai
}

#[tokio::test]
async fn d_065_adresse_de_reponse_a_usage_unique_double_controle_et_expediteur_deduit() {
    let (Some(a), Some(b)) = (cadre().await, cadre().await) else { return };
    let (uai_a, uai_b) = (poser_uai(&a).await, poser_uai(&b).await);
    // Expéditeur : déduit du profil de l'établissement du dossier, jamais fourni ; jamais le domaine d'un autre.
    let exp_a = a.ent.voyages_courrier_expediteur(a.etab).await.unwrap().unwrap();
    let exp_b = b.ent.voyages_courrier_expediteur(b.etab).await.unwrap().unwrap();
    assert_eq!(Some(exp_a.clone()), adresse_expedition(&uai_a));
    assert_ne!(exp_a, exp_b);
    assert!(exp_a.ends_with(&format!("@{}.scolia.fr", uai_a.to_lowercase())) && !exp_a.contains(&uai_b.to_lowercase()));
    assert_eq!(a.ent.voyages_courrier_expediteur(Uuid::new_v4()).await.unwrap(), None, "sans profil : aucun envoi possible");
    // Un prestataire de A et un dossier de A.
    let presta = a
        .ent
        .prestataire_creer(
            &a.a(a.sec, Role::Administratif),
            PrestataireSaisie {
                raison_sociale: format!("Cars {}", Uuid::new_v4().simple()),
                siren: None,
                telephone: None,
                categories: vec!["transporteur".into()],
                adresses: vec![format!("contact-{}@cars.fr", Uuid::new_v4().simple())],
                version: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    let s = a.creer("Sortie", "sans_nuitee").await;
    let cible = CibleReponse::Prestataire { prestataire_id: presta.id, adresse_id: presta.adresses[0].id };
    // Droits : porteur ou voyages.prestataires ; l'accueil, un encadrant non porteur, une famille ne peuvent rien émettre.
    for (acteur, nom) in [(a.a(a.accueil, Role::Accueil), "accueil"), (a.a(a.parent, Role::Parent), "famille")] {
        let e = a.ent.voyage_courrier_reponse_emettre(&acteur, s.id, cible.clone()).await.unwrap_err();
        assert!(est_introuvable(&e) || est_forbidden(&e), "{nom} : {e:?}");
    }
    // Une adresse d'un autre prestataire (ou inventée) n'est jamais acceptée pour cette sortie.
    let faux = CibleReponse::Prestataire { prestataire_id: presta.id, adresse_id: Uuid::new_v4() };
    assert!(est_introuvable(&a.ent.voyage_courrier_reponse_emettre(&a.prof_a(), s.id, faux).await.unwrap_err()));
    let adr = a.ent.voyage_courrier_reponse_emettre(&a.prof_a(), s.id, cible.clone()).await.unwrap().unwrap();
    assert!(adr.starts_with("r-") && adr.ends_with(&format!("@{}.scolia.fr", uai_a.to_lowercase())), "{adr}");
    // La clé n'est jamais conservée en clair.
    let cle = adr.strip_prefix("r-").unwrap().split('@').next().unwrap().to_string();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_reponse_cle WHERE cle_hash = $1").bind(&cle).fetch_one(a.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
    // Double contrôle : l'adresse de A présentée au sous-domaine de B est rejetée (file de tri), sans rien consommer ;
    // la même clé réécrite sur le sous-domaine de B ne mène pas non plus au dossier de A.
    assert_eq!(b.ent.voyage_courrier_reponse_resoudre(b.etab, &adr).await.unwrap(), Reponse::ATrier("sous_domaine_autre_etablissement"));
    let reecrite = format!("r-{cle}@{}.scolia.fr", uai_b.to_lowercase());
    assert_eq!(b.ent.voyage_courrier_reponse_resoudre(b.etab, &reecrite).await.unwrap(), Reponse::ATrier("cle_inconnue"));
    assert_eq!(a.ent.voyage_courrier_reponse_resoudre(a.etab, "n'importe quoi").await.unwrap(), Reponse::ATrier("adresse_mal_formee"));
    // Résolution par l'établissement de A : rattachée au dossier et au prestataire ; USAGE UNIQUE.
    assert_eq!(
        a.ent.voyage_courrier_reponse_resoudre(a.etab, &adr.to_uppercase()).await.unwrap(),
        Reponse::Rattachee { sortie_id: s.id, cible: cible.clone() }
    );
    assert_eq!(a.ent.voyage_courrier_reponse_resoudre(a.etab, &adr).await.unwrap(), Reponse::ATrier("cle_deja_utilisee"));
    // Expiration et révocation.
    let adr2 = a.ent.voyage_courrier_reponse_emettre(&a.prof_a(), s.id, cible.clone()).await.unwrap().unwrap();
    sqlx::query("UPDATE ent_sortie_reponse_cle SET expire_le = now() - interval '1 minute' WHERE sortie_id = $1 AND utilisee_le IS NULL")
        .bind(s.id)
        .execute(a.ent.pool())
        .await
        .unwrap();
    assert_eq!(a.ent.voyage_courrier_reponse_resoudre(a.etab, &adr2).await.unwrap(), Reponse::ATrier("cle_expiree"));
    let adr3 = a.ent.voyage_courrier_reponse_emettre(&a.prof_a(), s.id, CibleReponse::Restauration).await.unwrap().unwrap();
    assert_eq!(a.ent.voyage_courrier_reponses_revoquer(&a.prof_a(), s.id).await.unwrap(), 1);
    assert_eq!(a.ent.voyage_courrier_reponse_resoudre(a.etab, &adr3).await.unwrap(), Reponse::ATrier("cle_revoquee"));
    // Sans UAI : aucune adresse n'est émise (aucun repli sur un domaine commun).
    let sans = b.creer("Autre", "sans_nuitee").await;
    sqlx::query("UPDATE ent_etablissement_profil SET uai = NULL WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool()).await.unwrap();
    assert_eq!(b.ent.voyage_courrier_reponse_emettre(&b.prof_a(), sans.id, CibleReponse::Restauration).await.unwrap(), None);
}
