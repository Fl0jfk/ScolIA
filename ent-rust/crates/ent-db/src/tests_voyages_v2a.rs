//! Socle 42, lot V2a (l'argent des sorties en mode suivi, R-SV44) contre la base locale de test. Sans
//! `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**. Aucun jour réel codé en dur : les dates viennent du
//! cadre (jour de l'établissement) ; la sortie part dans trois jours, donc le solde « à J-30 » est déjà échu.
//!
//! * `parts_…` : la part découle du budget validé, répartie par le **port des payeurs** du 24 ; état agrégé sans nom ;
//! * `versement_…` : pointage sans facture, ajout seul, annulation par écriture inverse motivée, bornes ;
//! * `plan_…` : plan long accordé, invisible de l'organisateur (« 1 plan en cours ») ;
//! * `bordereau_…` : chèques collectés par l'organisateur, remise le jour même, écart signalé, public interdit ;
//! * `acomptes_…` : fournisseurs à payer, bientôt, en retard, alerte une seule fois ;
//! * `avoirs_…` : désistement avant/après engagement, motif légitime, frais réels, baisse de prix, annulation
//!   collective : **montants proposés**, jamais une écriture comptable, jamais plus que le versé ;
//! * `obligatoire_…` (CV-81), `contournement_…` (NX-07), `droits_…`, `famille_…`, `export_…`, `base_…`, `outbox_…`.

use super::*;
use crate::tests_voyages_v0a::{est_conflit_detail, est_forbidden, est_introuvable, est_refus, Cadre};
use crate::tests_voyages_v0d::cadre;
use crate::voyages::Commande;
use crate::voyages_argent_flux::{AcompteSaisie, BordereauLigneSaisie, BordereauSaisie};
use crate::voyages_argent_repo::{FraisSaisie, VersementSaisie};
use crate::voyages_prestataires_repo::PrestataireSaisie;
use crate::voyages_repo::{ParamsCommande, SortieLigne};
use chrono::Duration;
use ent_auth::Role;
use ent_object::presign::Presigner;
use uuid::Uuid;

fn ligne_budget(sens: &str, poste: &str, libelle: &str, montant: &str) -> crate::voyages_budget_repo::LigneSaisie {
    crate::voyages_budget_repo::LigneSaisie {
        sens: sens.into(),
        poste: poste.into(),
        libelle: libelle.into(),
        montant: montant.into(),
        motif: Some("Devis de l'agence".into()),
        affectation: None,
    }
}

fn code_sql<T>(r: &Result<T, sqlx::Error>) -> Option<String> {
    match r {
        Err(sqlx::Error::Database(d)) => d.code().map(|x| x.to_string()),
        _ => None,
    }
}

/// Une sortie ouverte (budget validé : un car de 1 500 € pour trois payants, soit **500 €** par famille) avec quatre
/// payeurs réels : l'élève 0 a deux parents séparés (50 % chacun, deux foyers), l'élève 1 un seul payeur, l'élève 2
/// **aucun** payeur désigné.
struct Suivi {
    c: Cadre,
    s: SortieLigne,
    parent_a: Uuid,
    parent_b: Uuid,
    parent_c: Uuid,
    compte_a: Uuid,
    compte_b: Uuid,
    compte_c: Uuid,
    p0: Uuid,
    p1: Uuid,
}

async fn foyer(c: &Cadre) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, 'Foyer V2a')")
        .bind(id)
        .bind(c.etab)
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    id
}

async fn payeur(c: &Cadre, eleve: Uuid, user: Uuid, foyer: Uuid, part: i16) {
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien, est_payeur, part_paiement)
         VALUES ($1, $2, $3, $4, $5, 'mere', true, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(c.etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .bind(part)
    .execute(c.ent.pool_verif())
    .await
    .unwrap();
}

async fn compte_du_foyer(c: &Cadre, foyer: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT ent_fin_compte_du_foyer($1, $2, NULL, 'manuel')")
        .bind(c.etab)
        .bind(foyer)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap()
}

impl Cadre {
    /// Jour de l'établissement (jamais UTC : règle 33).
    async fn jour_etab(&self) -> chrono::NaiveDate {
        self.ent.jour_etablissement(self.etab).await.unwrap()
    }

    async fn voyage_a_budget(&self, titre: &str) -> SortieLigne {
        let s = self.creer(titre, "voyage").await;
        self.preparer_jusqu_a_soumise(s.id).await;
        self.cmd(self.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap();
        self.cmd(self.prof, Role::Prof, s.id, Commande::PasserAuxFinances).await.unwrap();
        self.budget_minimal(s.id).await;
        self.cmd(self.compta, Role::Compta, s.id, Commande::ValiderBudget).await.unwrap();
        self.cmd(self.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap()
    }
}

async fn suivi() -> Option<Suivi> {
    let c = cadre().await?;
    let s = c.voyage_a_budget("Voyage à Rome").await;
    c.ent.voyage_participants_ajouter(&c.a(c.prof, Role::Prof), s.id, c.eleves.clone()).await.expect("participants");
    let parent_a = crate::tests_voyages_v0a::compte(&c.ent, c.etab, "parent").await;
    let parent_b = crate::tests_voyages_v0a::compte(&c.ent, c.etab, "parent").await;
    let parent_c = crate::tests_voyages_v0a::compte(&c.ent, c.etab, "parent").await;
    let (fa, fb, fc) = (foyer(&c).await, foyer(&c).await, foyer(&c).await);
    payeur(&c, c.eleves[0], parent_a, fa, 50).await;
    payeur(&c, c.eleves[0], parent_b, fb, 50).await;
    payeur(&c, c.eleves[1], parent_c, fc, 100).await;
    let (compte_a, compte_b, compte_c) = (compte_du_foyer(&c, fa).await, compte_du_foyer(&c, fb).await, compte_du_foyer(&c, fc).await);
    let (p0, p1) = (c.participant(s.id, c.eleves[0]).await, c.participant(s.id, c.eleves[1]).await);
    Some(Suivi { c, s, parent_a, parent_b, parent_c, compte_a, compte_b, compte_c, p0, p1 })
}

impl Suivi {
    fn compta(&self) -> Acteur<'static> {
        self.c.a(self.c.compta, Role::Compta)
    }
    fn dir(&self) -> Acteur<'static> {
        self.c.a(self.c.dir, Role::Direction)
    }
    fn sec(&self) -> Acteur<'static> {
        self.c.a(self.c.sec, Role::Administratif)
    }
    fn porteur(&self) -> Acteur<'static> {
        self.c.a(self.c.prof, Role::Prof)
    }
    fn parent(&self, user: Uuid) -> Acteur<'static> {
        self.c.a(user, Role::Parent)
    }

    async fn verser(&self, participant: Uuid, compte: Uuid, moyen: &str, montant: &str) -> Result<crate::voyages_argent_repo::VersementVue, EntDbError> {
        self.c
            .ent
            .voyage_versement_pointer(
                &self.compta(),
                self.s.id,
                VersementSaisie {
                    participant_id: participant,
                    compte_id: compte,
                    moyen: moyen.into(),
                    montant: montant.into(),
                    date_valeur: None,
                    reference: Some("REF-1".into()),
                },
            )
            .await
    }

    async fn sync(&self) {
        self.c.ent.voyage_argent_synchroniser(&self.compta(), self.s.id).await.expect("parts");
    }
}

// ═════════════════════════════ Parts et état agrégé ═════════════════════════════

#[tokio::test]
async fn parts_decoulent_du_budget_repartie_par_le_port_des_payeurs_etat_sans_nom() {
    let Some(x) = suivi().await else { return };
    let prix: i64 = sqlx::query_scalar("SELECT montant_centimes FROM ent_sortie_prix_fige WHERE sortie_id = $1 ORDER BY rang DESC LIMIT 1")
        .bind(x.s.id)
        .fetch_one(x.c.ent.pool())
        .await
        .unwrap();
    assert_eq!(prix, 50_000, "1 500 € pour trois payants");
    let e = x.c.ent.voyage_argent_synchroniser(&x.compta(), x.s.id).await.unwrap();
    assert_eq!((e.participants_payants, e.a_definir), (3, 1), "l'élève sans payeur désigné reste « à définir » (jamais au responsable 1)");
    // Deux parents séparés : 250 € chacun ; un seul payeur : 500 €.
    let lignes = x.c.ent.voyage_argent_paiements(&x.compta(), x.s.id).await.unwrap();
    assert_eq!(lignes.len(), 3);
    let part = |compte: Uuid| lignes.iter().find(|l| l.compte_id == compte).unwrap().part_centimes;
    assert_eq!((part(x.compte_a), part(x.compte_b), part(x.compte_c)), (25_000, 25_000, 50_000));
    assert_eq!(e.du_total_centimes, 100_000);
    // La synchronisation est idempotente : ni doublon de part, ni changement.
    x.sync().await;
    x.sync().await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_part WHERE sortie_id = $1").bind(x.s.id).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 3);
    // L'organisateur voit l'état agrégé (des nombres), et rien de nominatif.
    let ag = x.c.ent.voyage_argent_etat(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!(ag.participants_payants, 3);
    assert_eq!(ag.en_retard, 2, "la sortie part dans trois jours : le solde à J-30 est échu, rien n'est versé");
    let json = serde_json::to_string(&ag).unwrap();
    for interdit in ["Alpha", "Bravo", "Charlie", "nom", "eleve", "payeur"] {
        assert!(!json.contains(interdit), "l'état agrégé ne contient aucun nom : {json}");
    }
    // Le nominatif est fermé au porteur, à un autre professeur, à l'accueil, au secrétariat sans droit d'argent.
    for (acteur, nom) in [(x.porteur(), "porteur"), (x.c.a(x.c.prof2, Role::Prof), "prof"), (x.c.a(x.c.accueil, Role::Accueil), "accueil"), (x.sec(), "secrétariat")] {
        let e = x.c.ent.voyage_argent_paiements(&acteur, x.s.id).await.unwrap_err();
        assert!(est_introuvable(&e) || est_forbidden(&e), "{nom} : {e:?}");
    }
    // Chaque consultation nominative est journalisée.
    let acces: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_argent_acces WHERE sortie_id = $1 AND portee = 'liste_paiements' AND user_id = $2")
        .bind(x.s.id)
        .bind(x.c.compta)
        .fetch_one(x.c.ent.pool())
        .await
        .unwrap();
    assert_eq!(acces, 1);
    // Un autre établissement ne voit rien : « introuvable ».
    let Some(autre) = cadre().await else { return };
    let e = autre.ent.voyage_argent_etat(&autre.a(autre.compta, Role::Compta), x.s.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

// ═════════════════════════════ Versements ═════════════════════════════

#[tokio::test]
async fn versement_pointe_sans_facture_ajout_seul_et_annulation_inverse_motivee() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    // Moyens, montants et dates : rien d'ambigu n'entre.
    for (moyen, montant) in [("carte", "10"), ("cheque", "-5"), ("cheque", "12,345"), ("cheque", "0"), ("cheque", "abc"), ("cheque", "100 000,01")] {
        let e = x.verser(x.p1, x.compte_c, moyen, montant).await.unwrap_err();
        assert!(est_refus(&e) || matches!(e, EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide)), "{moyen} {montant} : {e:?}");
    }
    let e = x.verser(x.p1, x.compte_c, "cheques_vacances", "50").await.unwrap_err();
    assert!(est_refus(&e), "chèques-vacances : réglage désactivé par défaut (agrément ANCV)");
    // Une date de valeur future est refusée.
    let futur = x.c.jour + Duration::days(40);
    let e = x
        .c
        .ent
        .voyage_versement_pointer(&x.compta(), x.s.id, VersementSaisie { participant_id: x.p1, compte_id: x.compte_c, moyen: "cheque".into(), montant: "10".into(), date_valeur: Some(futur), reference: None })
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");

    let v = x.verser(x.p1, x.compte_c, "cheque", "200,50").await.expect("versement");
    assert_eq!((v.sens.as_str(), v.montant_centimes), ("versement", 20_050));
    // Un versement qui dépasse la part du payeur est refusé ; le versement de l'autre payeur n'est pas compté.
    let e = x.verser(x.p1, x.compte_c, "virement", "300").await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    x.verser(x.p0, x.compte_a, "especes", "250").await.expect("le payeur A solde sa part");
    let e = x.verser(x.p0, x.compte_a, "especes", "1").await.unwrap_err();
    assert!(est_refus(&e), "part soldée");
    // Un payeur qui n'est pas celui de l'élève : « introuvable ».
    let e = x.verser(x.p1, x.compte_a, "cheque", "10").await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    // Rôles : le porteur, un professeur, le secrétariat (réception de bordereaux seulement) ne pointent pas.
    for acteur in [x.porteur(), x.c.a(x.c.prof2, Role::Prof), x.sec(), x.c.a(x.c.accueil, Role::Accueil)] {
        let e = x
            .c
            .ent
            .voyage_versement_pointer(&acteur, x.s.id, VersementSaisie { participant_id: x.p1, compte_id: x.compte_c, moyen: "cheque".into(), montant: "1".into(), date_valeur: None, reference: None })
            .await
            .unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }

    // Annulation : une écriture INVERSE motivée ; l'original est inchangé.
    let e = x.c.ent.voyage_versement_annuler(&x.compta(), x.s.id, v.id, "  ").await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide)), "motif obligatoire : {e:?}");
    let a = x.c.ent.voyage_versement_annuler(&x.compta(), x.s.id, v.id, "Chèque rejeté par la banque").await.unwrap();
    assert_eq!((a.sens.as_str(), a.montant_centimes, a.annule_id), ("annulation", 20_050, Some(v.id)));
    let e = x.c.ent.voyage_versement_annuler(&x.compta(), x.s.id, v.id, "encore").await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(domain_eleve::DomainError::Conflit)), "une seule annulation : {e:?}");
    let e = x.c.ent.voyage_versement_annuler(&x.compta(), x.s.id, a.id, "annuler l'annulation").await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide)), "{e:?}");
    let l = x.c.ent.voyage_versements(&x.compta(), x.s.id).await.unwrap();
    assert!(l.iter().any(|y| y.id == v.id && y.sens == "versement" && y.montant_centimes == 20_050), "l'original est intact");
    // Le versé net revient à zéro pour ce payeur, qui peut verser de nouveau.
    let ligne = x.c.ent.voyage_argent_paiements(&x.compta(), x.s.id).await.unwrap().into_iter().find(|l| l.compte_id == x.compte_c).unwrap();
    assert_eq!(ligne.verse_centimes, 0);
    x.verser(x.p1, x.compte_c, "virement", "500").await.expect("nouveau versement après annulation");

    // Aucune facture : aucune table de facturation n'a bougé pour cette sortie.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_fac_facture WHERE etablissement_id = $1").bind(x.c.etab).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 0, "le mode suivi n'émet aucune facture");
}

// ═════════════════════════════ Plan de paiement et retard ═════════════════════════════

#[tokio::test]
async fn plan_long_accorde_invisible_de_l_organisateur_et_retard_sans_nom() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let comp = x.compta();
    // Plan long sans motif, ou hors bornes : refusé.
    assert!(est_refus(&x.c.ent.voyage_plan_accorder(&comp, x.s.id, x.p1, x.compte_c, 5, None).await.unwrap_err()));
    for n in [0, 11] {
        assert!(x.c.ent.voyage_plan_accorder(&comp, x.s.id, x.p1, x.compte_c, n, Some("x")).await.is_err(), "{n} échéances");
    }
    // Rôles : la comptabilité et la direction accordent, ni le porteur ni le secrétariat.
    assert!(est_forbidden(&x.c.ent.voyage_plan_accorder(&x.porteur(), x.s.id, x.p1, x.compte_c, 5, Some("x")).await.unwrap_err()));
    assert!(est_forbidden(&x.c.ent.voyage_plan_accorder(&x.sec(), x.s.id, x.p1, x.compte_c, 5, Some("x")).await.unwrap_err()));
    // Sans plan, la part de 500 € est échue : en retard.
    assert_eq!(x.c.ent.voyage_argent_etat(&x.porteur(), x.s.id).await.unwrap().en_retard, 2);
    x.c.ent.voyage_plan_accorder(&x.dir(), x.s.id, x.p1, x.compte_c, 5, Some("Famille en difficulté, étalement demandé")).await.unwrap();
    let l = x.c.ent.voyage_argent_paiements(&x.compta(), x.s.id).await.unwrap().into_iter().find(|l| l.compte_id == x.compte_c).unwrap();
    assert!(l.plan_long && l.nb_echeances == 5);
    assert_eq!(l.echeances.iter().map(|e| e.centimes).sum::<i64>(), 50_000, "l'échéancier somme exactement la part");
    assert_eq!(l.echeances[0].date, x.c.jour_etab().await, "la première échéance est celle de l'inscription");
    // Après la première échéance versée, le plan est « en cours » : l'organisateur ne voit que ce compteur.
    let premiere = l.echeances[0].centimes;
    x.verser(x.p1, x.compte_c, "cheque", &crate::voyages_argent::euros(premiere)).await.unwrap();
    let ag = x.c.ent.voyage_argent_etat(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!(ag.plans_en_cours, 1, "« 1 plan en cours »");
    assert_eq!(ag.en_retard, 1, "l'élève aux deux parents séparés reste en retard (compté par participant), sans nom");
    // Le plan n'a aucune trace côté organisateur : aucune route ne le lui rend.
    assert!(x.c.ent.voyage_argent_paiements(&x.porteur(), x.s.id).await.is_err());
}

// ═════════════════════════════ Bordereau de chèques ═════════════════════════════

fn ligne_b(p: Uuid, numero: &str, montant: &str) -> BordereauLigneSaisie {
    BordereauLigneSaisie { participant_id: p, numero_cheque: numero.into(), montant: montant.into() }
}

#[tokio::test]
async fn bordereau_de_cheques_jour_meme_ecart_signale_et_pointage() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let prof = x.porteur();
    let b = |nb: i32, total: &str, lignes: Vec<BordereauLigneSaisie>| BordereauSaisie { nb_cheques: nb, total: total.into(), lignes };
    // Le nombre ou le total annoncé doit être celui des lignes ; pas de numéro en double ; pas de bordereau vide.
    assert!(est_refus(&x.c.ent.voyage_bordereau_saisir(&prof, x.s.id, b(2, "300", vec![ligne_b(x.p0, "111", "100"), ligne_b(x.p1, "112", "200,01")])).await.unwrap_err()));
    assert!(est_refus(&x.c.ent.voyage_bordereau_saisir(&prof, x.s.id, b(3, "300", vec![ligne_b(x.p0, "111", "100"), ligne_b(x.p1, "112", "200")])).await.unwrap_err()));
    assert!(est_refus(&x.c.ent.voyage_bordereau_saisir(&prof, x.s.id, b(2, "300", vec![ligne_b(x.p0, "111", "100"), ligne_b(x.p1, "111", "200")])).await.unwrap_err()));
    assert!(x.c.ent.voyage_bordereau_saisir(&prof, x.s.id, b(0, "0", vec![])).await.is_err());
    // Seul le porteur saisit : un autre professeur, la comptabilité, le secrétariat non.
    for acteur in [x.c.a(x.c.prof2, Role::Prof), x.compta(), x.sec()] {
        let e = x.c.ent.voyage_bordereau_saisir(&acteur, x.s.id, b(1, "100", vec![ligne_b(x.p0, "113", "100")])).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }
    let bv = x.c.ent.voyage_bordereau_saisir(&prof, x.s.id, b(2, "300", vec![ligne_b(x.p0, "111", "100"), ligne_b(x.p1, "112", "200")])).await.expect("bordereau");
    assert_eq!((bv.nb_cheques, bv.total_centimes, bv.jour_collecte), (2, 30_000, x.c.jour_etab().await));
    assert!(bv.remise.is_none() && !bv.en_attente);
    // L'organisateur ne reçoit pas son propre bordereau (il n'a pas le droit) ; le secrétariat reçoit.
    assert!(est_forbidden(&x.c.ent.voyage_bordereau_remettre(&prof, x.s.id, bv.id, 2, "300", None).await.unwrap_err()));
    // Un écart sans note est refusé ; avec note il est enregistré et signalé.
    assert!(est_refus(&x.c.ent.voyage_bordereau_remettre(&x.sec(), x.s.id, bv.id, 1, "100", None).await.unwrap_err()));
    let r = x.c.ent.voyage_bordereau_remettre(&x.sec(), x.s.id, bv.id, 1, "100", Some("Un chèque manquant dans l'enveloppe")).await.unwrap();
    let rem = r.remise.unwrap();
    assert!(rem.ecart && !rem.en_retard, "même jour, mais un écart");
    // Remise unique.
    assert!(matches!(x.c.ent.voyage_bordereau_remettre(&x.sec(), x.s.id, bv.id, 2, "300", Some("bis")).await.unwrap_err(), EntDbError::Domain(domain_eleve::DomainError::Conflit)));
    // Pointage d'une ligne : le payeur est choisi par celui qui pointe ; une ligne ne se pointe qu'une fois.
    let lignes = x.c.ent.voyage_bordereaux(&x.sec(), x.s.id).await.unwrap().remove(0).lignes;
    let l0 = lignes.iter().find(|l| l.numero_cheque == "111").unwrap().id;
    let v = x.c.ent.voyage_bordereau_ligne_pointer(&x.sec(), x.s.id, l0, x.compte_a).await.unwrap();
    assert_eq!((v.moyen.as_str(), v.montant_centimes, v.reference.as_deref()), ("cheque", 10_000, Some("111")));
    assert!(matches!(x.c.ent.voyage_bordereau_ligne_pointer(&x.sec(), x.s.id, l0, x.compte_b).await.unwrap_err(), EntDbError::Domain(domain_eleve::DomainError::Conflit)));
    // L'organisateur voit ses bordereaux ; un autre professeur non.
    assert_eq!(x.c.ent.voyage_bordereaux(&prof, x.s.id).await.unwrap().len(), 1);
    assert!(x.c.ent.voyage_bordereaux(&x.c.a(x.c.prof2, Role::Prof), x.s.id).await.is_err());
    // Événement de signalement : un identifiant et un code, aucun montant ni nom.
    let payload: String = sqlx::query_scalar("SELECT m.payload::text FROM ent_outbox o JOIN ent_metier_event m ON m.id = o.event_id WHERE o.etablissement_id = $1 AND o.type = 'voyages.bordereau_signale'")
        .bind(x.c.etab)
        .fetch_one(x.c.ent.pool_verif())
        .await
        .unwrap();
    assert!(payload.contains("ecart") && !payload.contains("30000") && !payload.contains("centimes"), "{payload}");
}

#[tokio::test]
async fn bordereau_remis_un_autre_jour_est_en_retard_et_signale_en_attente() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    // Un bordereau collecté hier (inséré tel quel : la base garde la trace) et jamais remis.
    let hier = x.c.jour_etab().await - Duration::days(1);
    let bid = Uuid::new_v4();
    let mut tx = x.c.ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)").bind(x.c.etab.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO ent_sortie_bordereau (id, etablissement_id, sortie_id, jour_collecte, nb_cheques, total_centimes, saisi_par_user_id) VALUES ($1,$2,$3,$4,1,10000,$5)")
        .bind(bid).bind(x.c.etab).bind(x.s.id).bind(hier).bind(x.c.prof).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO ent_sortie_bordereau_ligne (id, etablissement_id, bordereau_id, participant_id, numero_cheque, montant_centimes) VALUES ($1,$2,$3,$4,'900',10000)")
        .bind(Uuid::new_v4()).bind(x.c.etab).bind(bid).bind(x.p1).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let attente = x.c.ent.voyages_bordereaux_en_attente(&x.sec()).await.unwrap();
    assert_eq!(attente.len(), 1, "à relancer : la collecte date d'hier");
    assert!(est_forbidden(&x.c.ent.voyages_bordereaux_en_attente(&x.porteur()).await.unwrap_err()));
    assert!(x.c.ent.voyage_bordereaux(&x.porteur(), x.s.id).await.unwrap()[0].en_attente);
    let r = x.c.ent.voyage_bordereau_remettre(&x.sec(), x.s.id, bid, 1, "100", None).await.unwrap();
    let rem = r.remise.unwrap();
    assert!(rem.en_retard && !rem.ecart, "remis un autre jour que la collecte : en retard, sans écart");
    assert!(x.c.ent.voyages_bordereaux_en_attente(&x.sec()).await.unwrap().is_empty());
}

#[tokio::test]
async fn bordereau_interdit_dans_le_public_et_quand_l_etablissement_le_desactive() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let saisie = || BordereauSaisie { nb_cheques: 1, total: "100".into(), lignes: vec![ligne_b(x.p0, "700", "100")] };
    // Réglage de l'établissement : la collecte par les organisateurs peut être coupée (réservé à la comptabilité).
    let mut r = x.c.ent.voyages_argent_reglages(&x.compta()).await.unwrap();
    assert!(r.collecte_organisateur && !r.cheques_vacances);
    assert!(est_forbidden(&x.c.ent.voyages_argent_reglages(&x.porteur()).await.unwrap_err()));
    r.collecte_organisateur = false;
    x.c.ent.voyages_argent_reglages_definir(&x.compta(), r.clone()).await.unwrap();
    assert!(est_refus(&x.c.ent.voyage_bordereau_saisir(&x.porteur(), x.s.id, saisie()).await.unwrap_err()));
    r.collecte_organisateur = true;
    r.alerte_acompte_jours = 0;
    assert!(x.c.ent.voyages_argent_reglages_definir(&x.compta(), r.clone()).await.is_err(), "délai d'alerte hors bornes");
    r.alerte_acompte_jours = 14;
    x.c.ent.voyages_argent_reglages_definir(&x.compta(), r).await.unwrap();
    x.c.ent.voyage_bordereau_saisir(&x.porteur(), x.s.id, saisie()).await.expect("privé : l'organisateur peut collecter");
    // Régime public : seul le régisseur manie les fonds, l'organisateur ne collecte pas.
    sqlx::query("UPDATE ent_fin_entite_facturation SET regime = 'public_eple' WHERE etablissement_id = $1").bind(x.c.etab).execute(x.c.ent.pool_verif()).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_fin_entite_facturation WHERE etablissement_id = $1 AND regime = 'public_eple'").bind(x.c.etab).fetch_one(x.c.ent.pool_verif()).await.unwrap();
    if n == 0 {
        // Entité de facturation absente dans ce cadre : on la crée comme le service (régime public).
        sqlx::query("INSERT INTO ent_fin_entite_facturation (id, etablissement_id, regime, libelle) VALUES ($1,$2,'public_eple','Collège public')")
            .bind(Uuid::new_v4()).bind(x.c.etab).execute(x.c.ent.pool_verif()).await.unwrap();
    }
    let e = x.c.ent.voyage_bordereau_saisir(&x.porteur(), x.s.id, BordereauSaisie { nb_cheques: 1, total: "100".into(), lignes: vec![ligne_b(x.p0, "701", "100")] }).await.unwrap_err();
    assert!(est_refus(&e), "public : l'organisateur ne collecte pas ({e:?})");
}

// ═════════════════════════════ Acomptes fournisseurs ═════════════════════════════

#[tokio::test]
async fn acomptes_fournisseurs_statuts_paiement_et_alerte_une_seule_fois() {
    let Some(x) = suivi().await else { return };
    let presta = x
        .c
        .ent
        .prestataire_creer(
            &x.sec(),
            PrestataireSaisie {
                raison_sociale: format!("Cars Dupont {}", Uuid::new_v4().simple()),
                siren: None,
                telephone: None,
                categories: vec!["transporteur".into()],
                adresses: vec![format!("cars-{}@exemple-test.fr", Uuid::new_v4().simple())],
                version: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    let jour = x.c.jour_etab().await;
    let acompte = |libelle: &str, montant: &str, echeance| AcompteSaisie { prestataire_id: presta.id, commande_id: None, libelle: libelle.into(), montant: montant.into(), echeance };
    let retard = x.c.ent.voyage_acompte_creer(&x.compta(), x.s.id, acompte("Acompte car", "450", jour - Duration::days(2))).await.unwrap();
    let bientot = x.c.ent.voyage_acompte_creer(&x.compta(), x.s.id, acompte("Acompte hôtel", "900", jour + Duration::days(5))).await.unwrap();
    let loin = x.c.ent.voyage_acompte_creer(&x.dir(), x.s.id, acompte("Solde hôtel", "1 800".replace(' ', "").as_str(), jour + Duration::days(60))).await.unwrap();
    assert_eq!((retard.statut.as_str(), bientot.statut.as_str(), loin.statut.as_str()), ("en_retard", "bientot", "a_payer"));
    // Droits : ni le porteur, ni le secrétariat, ni un autre professeur.
    for acteur in [x.porteur(), x.sec(), x.c.a(x.c.prof2, Role::Prof)] {
        let e = x.c.ent.voyage_acompte_creer(&acteur, x.s.id, acompte("x", "10", jour)).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
        assert!(x.c.ent.voyage_acomptes(&acteur, x.s.id).await.is_err());
    }
    // Un prestataire d'un autre établissement : « introuvable ».
    let Some(autre) = cadre().await else { return };
    let etranger = autre.ent.prestataire_creer(&autre.a(autre.sec, Role::Administratif), PrestataireSaisie { raison_sociale: format!("Autre {}", Uuid::new_v4().simple()), siren: None, telephone: None, categories: vec!["transporteur".into()], adresses: vec![format!("a-{}@exemple-test.fr", Uuid::new_v4().simple())], version: None, actif: None }).await.unwrap();
    let e = x.c.ent.voyage_acompte_creer(&x.compta(), x.s.id, AcompteSaisie { prestataire_id: etranger.id, commande_id: None, libelle: "x".into(), montant: "10".into(), echeance: jour }).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // L'alerte part UNE fois pour l'acompte en retard et celui de la fenêtre ; pas pour le lointain.
    assert_eq!(x.c.ent.voyages_acomptes_alertes_declencher(&x.compta()).await.unwrap(), 2);
    assert_eq!(x.c.ent.voyages_acomptes_alertes_declencher(&x.compta()).await.unwrap(), 0, "idempotent");
    assert!(est_forbidden(&x.c.ent.voyages_acomptes_alertes_declencher(&x.sec()).await.unwrap_err()));
    // Paiement : une seule fois ; une annulation motivée ; jamais de modification du montant dû.
    let p = x.c.ent.voyage_acompte_payer(&x.compta(), x.s.id, retard.id, Some("450,00"), None, Some("VIR-42")).await.unwrap();
    assert_eq!((p.statut.as_str(), p.paye_centimes), ("paye", Some(45_000)));
    assert!(matches!(x.c.ent.voyage_acompte_payer(&x.compta(), x.s.id, retard.id, None, None, None).await.unwrap_err(), EntDbError::Domain(domain_eleve::DomainError::Conflit)));
    assert!(x.c.ent.voyage_acompte_annuler(&x.compta(), x.s.id, loin.id, " ").await.is_err(), "motif obligatoire");
    x.c.ent.voyage_acompte_annuler(&x.compta(), x.s.id, loin.id, "Hébergement changé").await.unwrap();
    let liste = x.c.ent.voyage_acomptes(&x.dir(), x.s.id).await.unwrap();
    let st = |id: Uuid| liste.iter().find(|a| a.id == id).unwrap().statut.clone();
    assert_eq!((st(retard.id).as_str(), st(bientot.id).as_str(), st(loin.id).as_str()), ("paye", "bientot", "annule"));
    // Ni suppression ni modification du montant par la base.
    let r = sqlx::query("DELETE FROM ent_sortie_acompte_fournisseur WHERE id = $1").bind(bientot.id).execute(x.c.ent.pool_verif()).await;
    assert_eq!(code_sql(&r).as_deref(), Some("42501"));
    let r = sqlx::query("UPDATE ent_sortie_acompte_fournisseur SET montant_centimes = 1 WHERE id = $1").bind(bientot.id).execute(x.c.ent.pool_verif()).await;
    assert_eq!(code_sql(&r).as_deref(), Some("42501"));
}

// ═════════════════════════════ Avoirs proposés, frais réels ═════════════════════════════

#[tokio::test]
async fn avoirs_proposes_au_payeur_qui_a_paye_frais_reels_et_decision_de_la_comptabilite() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    // L'élève 0 a deux payeurs : A a versé 250 €, B 100 €.
    x.verser(x.p0, x.compte_a, "cheque", "250").await.unwrap();
    x.verser(x.p0, x.compte_b, "virement", "100").await.unwrap();
    // Désistement non légitime avant engagement des dépenses : avoir total, par payeur, jamais plus que le versé.
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p0, "decision_personnelle", None).await.unwrap();
    let avoirs = x.c.ent.voyage_avoirs(&x.compta(), x.s.id).await.unwrap();
    let de = |compte: Uuid| avoirs.iter().find(|a| a.compte_id == compte && a.statut == "propose").unwrap().clone();
    assert_eq!((de(x.compte_a).montant_propose_centimes, de(x.compte_b).montant_propose_centimes), (25_000, 10_000));
    assert_eq!(de(x.compte_a).motif_code, "desistement_avant_engagement");
    // Plus aucun versement ne se pointe pour ce participant.
    assert!(est_refus(&x.verser(x.p0, x.compte_a, "cheque", "1").await.unwrap_err()));
    // VOY-273 : un billet nominatif non remboursable de 120 € se répartit 60 / 60 ; l'avoir est recalculé
    // (les anciennes propositions sont écartées, pas supprimées).
    let frais = x.c.ent.voyage_frais_saisir(&x.compta(), x.s.id, FraisSaisie { participant_id: x.p0, nature: "billet_nominatif".into(), libelle: "Billet d'avion nominatif".into(), montant: "120".into() }).await.unwrap();
    let avoirs = x.c.ent.voyage_avoirs(&x.compta(), x.s.id).await.unwrap();
    let ouverts: Vec<_> = avoirs.iter().filter(|a| a.statut == "propose").collect();
    assert_eq!(ouverts.len(), 2);
    let montant = |compte: Uuid| ouverts.iter().find(|a| a.compte_id == compte).unwrap().montant_propose_centimes;
    assert_eq!((montant(x.compte_a), montant(x.compte_b)), (19_000, 4_000), "250 − 60 et 100 − 60 : jamais négatif, jamais au-delà du versé");
    assert_eq!(avoirs.iter().filter(|a| a.statut == "ecarte").count(), 2, "les anciennes propositions restent, écartées");
    assert!(ouverts.iter().all(|a| a.motif_code == "desistement_apres_engagement" && a.frais_retenus_centimes == 6_000));
    // Un frais se corrige par une annulation motivée, et l'avoir est recalculé.
    x.c.ent.voyage_frais_annuler(&x.compta(), x.s.id, frais.id, "Billet finalement remboursé par la compagnie").await.unwrap();
    let avoirs = x.c.ent.voyage_avoirs(&x.compta(), x.s.id).await.unwrap();
    let a_ouvert = avoirs.iter().find(|a| a.statut == "propose" && a.compte_id == x.compte_a).unwrap().clone();
    assert_eq!(a_ouvert.montant_propose_centimes, 25_000);
    // Frais pour un participant qui n'est pas parti : refusé.
    assert!(est_refus(&x.c.ent.voyage_frais_saisir(&x.compta(), x.s.id, FraisSaisie { participant_id: x.p1, nature: "autre".into(), libelle: "x".into(), montant: "10".into() }).await.unwrap_err()));
    // Décision : retenir plus que le versé est refusé ; retenu une fois, plus de recalcul.
    assert!(est_refus(&x.c.ent.voyage_avoir_decider(&x.compta(), x.s.id, a_ouvert.id, true, Some("300"), "ok").await.unwrap_err()));
    let d = x.c.ent.voyage_avoir_decider(&x.compta(), x.s.id, a_ouvert.id, true, None, "Validé en comptabilité").await.unwrap();
    assert_eq!((d.statut.as_str(), d.montant_retenu_centimes), ("retenu", Some(25_000)));
    assert!(matches!(x.c.ent.voyage_avoir_decider(&x.compta(), x.s.id, a_ouvert.id, false, None, "non").await.unwrap_err(), EntDbError::Domain(domain_eleve::DomainError::Conflit)));
    assert!(est_refus(&x.c.ent.voyage_frais_saisir(&x.compta(), x.s.id, FraisSaisie { participant_id: x.p0, nature: "autre".into(), libelle: "Pénalité".into(), montant: "10".into() }).await.unwrap_err()), "avoir déjà retenu");
    // Les avoirs sont des montants proposés : aucune pièce comptable ni écriture ne naît.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_fac_facture WHERE etablissement_id = $1").bind(x.c.etab).fetch_one(x.c.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 0);
    // Droits : le porteur et le secrétariat ne voient pas les avoirs.
    for acteur in [x.porteur(), x.sec()] {
        assert!(x.c.ent.voyage_avoirs(&acteur, x.s.id).await.is_err());
    }
}

#[tokio::test]
async fn avoirs_motif_legitime_total_et_annulation_collective() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    x.verser(x.p1, x.compte_c, "cheque", "500").await.unwrap();
    // Motif légitime (maladie) : avoir total proposé.
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p1, "sante", None).await.unwrap();
    let a = x.c.ent.voyage_avoirs(&x.compta(), x.s.id).await.unwrap();
    assert_eq!((a.len(), a[0].motif_code.as_str(), a[0].montant_propose_centimes), (1, "motif_legitime", 50_000));
    // Annulation collective : seulement sur une sortie annulée ; avoir total à chaque payeur ayant versé.
    x.verser(x.p0, x.compte_b, "especes", "80").await.unwrap();
    assert!(est_refus(&x.c.ent.voyage_avoirs_annulation_collective(&x.compta(), x.s.id).await.unwrap_err()));
    x.c.ent.voyage_commande(&x.dir(), x.s.id, Commande::Annuler, ParamsCommande { motif: Some("Grève des transports".into()), ..Default::default() }).await.unwrap();
    assert_eq!(x.c.ent.voyage_avoirs_annulation_collective(&x.compta(), x.s.id).await.unwrap(), 1);
    assert_eq!(x.c.ent.voyage_avoirs_annulation_collective(&x.compta(), x.s.id).await.unwrap(), 0, "idempotent");
    let a = x.c.ent.voyage_avoirs(&x.compta(), x.s.id).await.unwrap();
    assert!(a.iter().any(|y| y.motif_code == "annulation_collective" && y.compte_id == x.compte_b && y.montant_propose_centimes == 8_000));
}

#[tokio::test]
async fn avoirs_baisse_de_prix_rend_le_trop_verse_au_payeur_qui_a_paye() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    x.verser(x.p1, x.compte_c, "virement", "500").await.unwrap();
    x.verser(x.p0, x.compte_a, "cheque", "100").await.unwrap();
    // Révision du budget : le car coûte 1 200 € (part 400 €) ; validée par la comptabilité.
    let compta = x.compta();
    let v = x.c.ent.voyage_budget_version_creer(&compta, x.s.id, Some("Nouveau devis moins cher")).await.unwrap();
    let b = v.brouillon.unwrap();
    let v = x.c.ent.voyage_budget_ligne_modifier(&compta, x.s.id, b.numero, b.rev, b.lignes[0].id, ligne_budget("depense", "transport", "Car", "1 200")).await.unwrap();
    let b = v.brouillon.unwrap();
    x.c.ent.voyage_budget_valider_revision(&compta, x.s.id, b.numero, b.rev).await.unwrap();
    let avoirs = x.c.ent.voyage_avoirs(&compta, x.s.id).await.unwrap();
    assert_eq!(avoirs.len(), 1, "A a versé 100 € pour une part ramenée à 200 € : rien à rendre ; C a versé 500 € pour 400 €");
    assert_eq!((avoirs[0].motif_code.as_str(), avoirs[0].compte_id, avoirs[0].montant_propose_centimes), ("baisse_prix", x.compte_c, 10_000));
    // Les parts suivent le nouveau prix ; l'ancienne ligne reste (ajout seul).
    let lignes = x.c.ent.voyage_argent_paiements(&compta, x.s.id).await.unwrap();
    assert_eq!(lignes.iter().find(|l| l.compte_id == x.compte_c).unwrap().part_centimes, 40_000);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_part WHERE sortie_id = $1 AND participant_id = $2").bind(x.s.id).bind(x.p1).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 2);
}

// ═════════════════════════════ Sortie obligatoire, contournement du devis ═════════════════════════════

#[tokio::test]
async fn obligatoire_part_zero_imposee_cv_81() {
    let Some(c) = cadre().await else { return };
    let mut sa = c.saisie("Visite du collège voisin", "sans_nuitee");
    sa.nature = "obligatoire".into();
    sa.destination = "proximite".into();
    let s = c.ent.voyage_creer(&c.prof_a(), sa).await.unwrap();
    c.preparer_jusqu_a_soumise(s.id).await;
    c.cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap();
    c.cmd(c.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap();
    c.ent.voyage_participants_ajouter(&c.prof_a(), s.id, c.eleves.clone()).await.unwrap();
    let compta = c.a(c.compta, Role::Compta);
    let e = c.ent.voyage_argent_synchroniser(&compta, s.id).await.unwrap();
    assert!(e.gratuite && e.du_total_centimes == 0 && e.a_definir == 0, "{e:?}");
    let p = c.participant(s.id, c.eleves[0]).await;
    let (f, u) = (foyer(&c).await, crate::tests_voyages_v0a::compte(&c.ent, c.etab, "parent").await);
    payeur(&c, c.eleves[0], u, f, 100).await;
    let cpt = compte_du_foyer(&c, f).await;
    let err = c
        .ent
        .voyage_versement_pointer(&compta, s.id, VersementSaisie { participant_id: p, compte_id: cpt, moyen: "cheque".into(), montant: "10".into(), date_valeur: None, reference: None })
        .await
        .unwrap_err();
    assert!(est_refus(&err), "sortie obligatoire : rien n'est dû par les familles ({err:?})");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_part WHERE sortie_id = $1").bind(s.id).fetch_one(c.ent.pool()).await.unwrap();
    assert_eq!(n, 0, "aucune part pour une sortie obligatoire");
}

async fn code_et_motif_de(c: &Cadre, id: Uuid) -> (Option<String>, Option<String>) {
    sqlx::query_as::<_, (Option<String>, Option<String>)>("SELECT contournement_devis_code, contournement_devis_motif FROM ent_sortie WHERE id = $1")
        .bind(id)
        .fetch_one(c.ent.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn contournement_du_devis_motifs_codes() {
    let Some(c) = cadre().await else { return };
    let essai = |code: Option<&str>, texte: Option<&str>| ParamsCommande { motif_code: code.map(Into::into), motif: texte.map(Into::into), ..Default::default() };
    let vers_a_signer = || async {
        let mut sa = c.saisie("Sortie au château", "sans_nuitee");
        sa.besoin_car = true;
        let s = c.ent.voyage_creer(&c.prof_a(), sa).await.unwrap();
        c.preparer_jusqu_a_soumise(s.id).await;
        c.cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap();
        c.cmd(c.dir, Role::Direction, s.id, Commande::DemanderSignature).await.unwrap();
        s
    };
    let dir = c.a(c.dir, Role::Direction);
    let code_et_motif = |id: Uuid| code_et_motif_de(&c, id);
    // « Transport en commun » : aucune justification supplémentaire.
    let s = vers_a_signer().await;
    c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(Some("transport_en_commun"), None)).await.unwrap();
    assert_eq!(code_et_motif(s.id).await, (Some("transport_en_commun".into()), Some("Transport en commun".into())));
    // « Autre » sans texte, ou un code inconnu : refusés ; « autre » avec texte : accepté.
    let s = vers_a_signer().await;
    assert!(est_refus(&c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(Some("autre"), None)).await.unwrap_err()));
    assert!(est_refus(&c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(Some("fusee"), Some("x"))).await.unwrap_err()));
    assert!(est_refus(&c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(None, None)).await.unwrap_err()));
    c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(Some("autre"), Some("Devis oral du prestataire"))).await.unwrap();
    assert_eq!(code_et_motif(s.id).await, (Some("autre".into()), Some("Devis oral du prestataire".into())));
    // Compatibilité : un texte seul vaut « autre » ; « à pied » et « véhicule de l'établissement » sont des codes.
    let s = vers_a_signer().await;
    c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(None, Some("Devis oral"))).await.unwrap();
    assert_eq!(code_et_motif(s.id).await.0.as_deref(), Some("autre"));
    for code in ["a_pied", "vehicule_etablissement"] {
        let s = vers_a_signer().await;
        c.ent.voyage_commande(&dir, s.id, Commande::PasserAuxFinances, essai(Some(code), None)).await.unwrap();
        assert_eq!(code_et_motif(s.id).await.0.as_deref(), Some(code));
    }
    // Seule la direction contourne.
    let s = vers_a_signer().await;
    let e = c.ent.voyage_commande(&c.prof_a(), s.id, Commande::PasserAuxFinances, essai(Some("a_pied"), None)).await.unwrap_err();
    assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
}

// ═════════════════════════════ Famille, nominatif, export, base, outbox ═════════════════════════════

#[tokio::test]
async fn famille_ne_voit_que_sa_part_et_ses_versements_meme_parents_separes() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    x.verser(x.p0, x.compte_a, "cheque", "100").await.unwrap();
    x.verser(x.p0, x.compte_b, "virement", "40").await.unwrap();
    let a = x.c.ent.voyage_argent_famille(&x.parent(x.parent_a), x.s.id).await.unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!((a[0].compte_id, a[0].part_centimes, a[0].verse_centimes), (x.compte_a, 25_000, 10_000));
    assert_eq!(a[0].versements.len(), 1, "jamais le versement de l'autre payeur du même élève");
    let json = serde_json::to_string(&a).unwrap();
    assert!(!json.contains(&x.compte_b.to_string()) && !json.contains("4000"), "{json}");
    let b = x.c.ent.voyage_argent_famille(&x.parent(x.parent_b), x.s.id).await.unwrap();
    assert_eq!((b[0].compte_id, b[0].verse_centimes), (x.compte_b, 4_000));
    // Un parent sans rapport avec la sortie, un élève, un autre établissement : « introuvable ».
    let autre_parent = crate::tests_voyages_v0a::compte(&x.c.ent, x.c.etab, "parent").await;
    assert!(est_introuvable(&x.c.ent.voyage_argent_famille(&x.parent(autre_parent), x.s.id).await.unwrap_err()));
    assert!(est_introuvable(&x.c.ent.voyage_argent_famille(&x.c.a(x.c.prof, Role::Prof), x.s.id).await.unwrap_err()));
    let c_ = x.c.ent.voyage_argent_famille(&x.parent(x.parent_c), x.s.id).await.unwrap();
    assert_eq!(c_[0].compte_id, x.compte_c);
    let Some(autre) = cadre().await else { return };
    assert!(est_introuvable(&autre.ent.voyage_argent_famille(&autre.a(x.parent_a, Role::Parent), x.s.id).await.unwrap_err()));
}

#[tokio::test]
async fn nominatif_ouvert_par_la_direction_au_seul_responsable_journalise() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    assert!(est_introuvable(&x.c.ent.voyage_argent_paiements(&x.porteur(), x.s.id).await.unwrap_err()));
    // Seule la direction ouvre, avec un motif.
    assert!(est_forbidden(&x.c.ent.voyage_nominatif_ouvrir(&x.compta(), x.s.id, "x").await.unwrap_err()));
    assert!(est_forbidden(&x.c.ent.voyage_nominatif_ouvrir(&x.porteur(), x.s.id, "x").await.unwrap_err()));
    assert!(x.c.ent.voyage_nominatif_ouvrir(&x.dir(), x.s.id, " ").await.is_err());
    x.c.ent.voyage_nominatif_ouvrir(&x.dir(), x.s.id, "Relances des familles par le responsable").await.unwrap();
    assert!(matches!(x.c.ent.voyage_nominatif_ouvrir(&x.dir(), x.s.id, "bis").await.unwrap_err(), EntDbError::Domain(domain_eleve::DomainError::Conflit)));
    // Le responsable (porteur) lit ; un autre professeur et l'accueil non ; chaque lecture est journalisée.
    assert_eq!(x.c.ent.voyage_argent_paiements(&x.porteur(), x.s.id).await.unwrap().len(), 3);
    assert!(x.c.ent.voyage_argent_paiements(&x.c.a(x.c.prof2, Role::Prof), x.s.id).await.is_err());
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_argent_acces WHERE sortie_id = $1 AND user_id = $2").bind(x.s.id).bind(x.c.prof).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    x.c.ent.voyage_nominatif_fermer(&x.dir(), x.s.id).await.unwrap();
    assert!(est_introuvable(&x.c.ent.voyage_argent_paiements(&x.porteur(), x.s.id).await.unwrap_err()), "refermé : le porteur ne voit plus rien");
}

#[tokio::test]
async fn export_csv_lisible_lien_court_apres_controle_journalise_et_sans_ecriture_comptable() {
    let Some(mut x) = suivi().await else { return };
    x.c.ent.installer_presigner(Presigner::local(b"cle-de-test-v2a-0123456789abcdef!!", 60).unwrap());
    x.sync().await;
    x.verser(x.p1, x.compte_c, "cheque", "500").await.unwrap();
    x.verser(x.p0, x.compte_a, "cheque", "100,50").await.unwrap();
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p0, "famille", None).await.unwrap();
    // Droits : comptabilité et direction ; ni le porteur, ni le secrétariat, ni l'accueil.
    for acteur in [x.porteur(), x.sec(), x.c.a(x.c.accueil, Role::Accueil)] {
        let e = x.c.ent.voyage_argent_export(&acteur, x.s.id).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }
    let ex = x.c.ent.voyage_argent_export(&x.compta(), x.s.id).await.unwrap();
    assert!(ex.lien.contains("http") || ex.lien.starts_with('/'), "{}", ex.lien);
    let (cle, nb): (String, i32) = sqlx::query_as("SELECT cle_objet, nb_lignes FROM ent_sortie_argent_export WHERE sortie_id = $1").bind(x.s.id).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(nb as usize, ex.nb_lignes);
    let octets = x.c.ent.objet_get(&cle).await.unwrap();
    let texte = String::from_utf8(octets).unwrap();
    assert!(texte.starts_with('\u{feff}') && texte.contains("type;sortie;eleve_nom;eleve_prenom;payeur;date;moyen;reference;montant;etat;motif\r\n"));
    assert!(texte.contains("versement;") && texte.contains(";100,50;") && texte.contains("avoir_propose;") && texte.contains(";500,00;"), "{texte}");
    assert!(!texte.contains("facture") && !texte.contains("ecriture"), "un export de suivi n'est pas une pièce comptable");
    // Une cellule qui commencerait par = serait une formule dans un tableur : neutralisée.
    assert!(!texte.lines().any(|l| l.split(';').any(|c| c.starts_with('='))), "{texte}");
    // Journalisé : l'export et l'accès nominatif.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_argent_acces WHERE sortie_id = $1 AND portee = 'export' AND user_id = $2").bind(x.s.id).bind(x.c.compta).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    // Un autre établissement : « introuvable ».
    let Some(autre) = cadre().await else { return };
    let e = autre.ent.voyage_argent_export(&autre.a(autre.compta, Role::Compta), x.s.id).await.unwrap_err();
    assert!(est_introuvable(&e) || matches!(e, EntDbError::LiensIndisponibles), "{e:?}");
}

#[tokio::test]
async fn base_ajout_seul_bornes_et_somme_du_bordereau() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let v = x.verser(x.p1, x.compte_c, "cheque", "10").await.unwrap();
    let pool = x.c.ent.pool_verif();
    // UPDATE, DELETE, TRUNCATE : refusés par la base elle-même.
    for sql in [
        "UPDATE ent_sortie_versement SET montant_centimes = 1 WHERE id = $1",
        "DELETE FROM ent_sortie_versement WHERE id = $1",
        "UPDATE ent_sortie_part SET montant_centimes = 1 WHERE sortie_id = $1",
        "DELETE FROM ent_sortie_part WHERE sortie_id = $1",
    ] {
        let id = if sql.contains("part") { x.s.id } else { v.id };
        let r = sqlx::query(sql).bind(id).execute(pool).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "{sql}");
    }
    let r = sqlx::query("TRUNCATE ent_sortie_versement").execute(pool).await;
    assert!(r.is_err(), "TRUNCATE refusé");
    // Bornes : montant nul, négatif, au-delà de 100 000 € ; date de valeur future ; annulation qui ne reprend pas l'original.
    let ins = |montant: i64, date: chrono::NaiveDate| {
        let (etab, sortie, p, cpt, user) = (x.c.etab, x.s.id, x.p1, x.compte_c, x.c.compta);
        async move {
            sqlx::query("INSERT INTO ent_sortie_versement (id, etablissement_id, sortie_id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, saisi_par_user_id) VALUES ($1,$2,$3,$4,$5,'versement','cheque',$6,$7,$8)")
                .bind(Uuid::new_v4()).bind(etab).bind(sortie).bind(p).bind(cpt).bind(montant).bind(date).bind(user)
                .execute(pool).await
        }
    };
    let jour = x.c.jour_etab().await;
    for m in [0i64, -1, 10_000_001] {
        assert_eq!(code_sql(&ins(m, jour).await).as_deref(), Some("23514"), "{m}");
    }
    assert_eq!(code_sql(&ins(100, jour + Duration::days(2)).await).as_deref(), Some("23514"), "date future");
    let r = sqlx::query("INSERT INTO ent_sortie_versement (id, etablissement_id, sortie_id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, annule_id, motif, saisi_par_user_id) VALUES ($1,$2,$3,$4,$5,'annulation','cheque',999,$6,$7,'x',$8)")
        .bind(Uuid::new_v4()).bind(x.c.etab).bind(x.s.id).bind(x.p1).bind(x.compte_c).bind(jour).bind(v.id).bind(x.c.compta)
        .execute(pool).await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "une annulation reprend exactement le versement");
    // Un bordereau dont les lignes ne s'additionnent pas n'existe pas (contrôle au COMMIT).
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)").bind(x.c.etab.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO ent_sortie_bordereau (id, etablissement_id, sortie_id, jour_collecte, nb_cheques, total_centimes, saisi_par_user_id) VALUES ($1,$2,$3,$4,2,500,$5)")
        .bind(Uuid::new_v4()).bind(x.c.etab).bind(x.s.id).bind(jour).bind(x.c.prof).execute(&mut *tx).await.unwrap();
    let r = tx.commit().await;
    assert!(r.is_err(), "bordereau sans lignes : refusé au commit");
    // Isolation : toutes les tables d'argent sont sous RLS.
    for t in ["ent_sortie_part", "ent_sortie_plan", "ent_sortie_versement", "ent_sortie_frais_reel", "ent_sortie_avoir_propose", "ent_sortie_bordereau", "ent_sortie_bordereau_ligne", "ent_sortie_bordereau_remise", "ent_sortie_acompte_fournisseur", "ent_sortie_argent_export", "ent_sortie_nominatif_ouverture", "ent_sortie_argent_acces", "ent_sortie_argent_reglage"] {
        let rls: bool = sqlx::query_scalar("SELECT relrowsecurity AND relforcerowsecurity FROM pg_class WHERE relname = $1").bind(t).fetch_one(pool).await.unwrap();
        assert!(rls, "{t} : RLS forcée");
    }
}

#[tokio::test]
async fn outbox_sans_montant_ni_nom_pour_les_evenements_de_l_argent() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    x.verser(x.p1, x.compte_c, "cheque", "123,45").await.unwrap();
    let presta = x
        .c
        .ent
        .prestataire_creer(&x.sec(), PrestataireSaisie { raison_sociale: format!("Hotel {}", Uuid::new_v4().simple()), siren: None, telephone: None, categories: vec!["hebergeur".into()], adresses: vec![format!("h-{}@exemple-test.fr", Uuid::new_v4().simple())], version: None, actif: None })
        .await
        .unwrap();
    let jour = x.c.jour_etab().await;
    x.c.ent.voyage_acompte_creer(&x.compta(), x.s.id, AcompteSaisie { prestataire_id: presta.id, commande_id: None, libelle: "Acompte 987,65".into(), montant: "987,65".into(), echeance: jour }).await.unwrap();
    x.c.ent.voyages_acomptes_alertes_declencher(&x.compta()).await.unwrap();
    let payloads: Vec<String> = sqlx::query_scalar("SELECT m.payload::text FROM ent_outbox o JOIN ent_metier_event m ON m.id = o.event_id WHERE o.etablissement_id = $1 AND o.type IN ('voyages.acompte_echeance','voyages.bordereau_signale')")
        .bind(x.c.etab)
        .fetch_all(x.c.ent.pool_verif())
        .await
        .unwrap();
    assert!(!payloads.is_empty());
    for p in payloads {
        for interdit in ["98765", "987,65", "12345", "centimes", "montant", "Alpha", "Bravo", "Hotel"] {
            assert!(!p.contains(interdit), "{interdit} dans {p}");
        }
    }
    // Aucun événement de versement ni de part : seuls des signalements (identifiants et codes) existent.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type LIKE 'voyages.versement%'").bind(x.c.etab).fetch_one(x.c.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 0);
    let _ = est_conflit_detail;
}
