//! Socle 42, lot V3 « argent relié » (aides, reports de versement, internat) contre la base locale de test. Sans
//! `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**. Aucun jour réel codé en dur : les dates viennent du
//! cadre (jour de l'établissement).
//!
//! * `aide_…` : une aide réduit le reste dû, n'est jamais rendue en avoir, ne se voit que de la comptabilité ;
//! * `report_…` : l'argent d'un participant passe à un autre **du même payeur**, l'avoir du partant se recalcule ;
//! * `internat_…` : un interne confirmé est retiré de l'attendu des nuits du séjour, rétabli s'il se désiste ;
//! * `ajout_seul_…`, `base_…`, `isolation_…` : gardes de la base et cloisonnement.

use super::*;
use crate::tests_voyages_v0a::{est_forbidden, est_introuvable, est_refus};
use crate::tests_voyages_v2a::{code_sql, foyer, payeur, suivi, Suivi};
use crate::voyages_argent_v3::{AideSaisie, ReportSaisie};
use ent_auth::Role;
use uuid::Uuid;

fn aide(p: Uuid, compte: Uuid, source: &str, montant: &str) -> AideSaisie {
    AideSaisie { participant_id: p, compte_id: compte, source: source.into(), montant: montant.into(), reference: Some("DEC-12".into()) }
}

fn report(de: Uuid, vers: Uuid, compte: Uuid, montant: &str) -> ReportSaisie {
    ReportSaisie { de_participant_id: de, vers_participant_id: vers, compte_id: compte, montant: montant.into(), motif: "Le frère prend la place".into() }
}

impl Suivi {
    /// Participant du troisième élève, rattaché au **même payeur** que le deuxième (deux enfants, un compte).
    async fn p2_meme_payeur(&self) -> Uuid {
        let foyer_c: Uuid = sqlx::query_scalar("SELECT foyer_id FROM ent_eleve_lien WHERE user_id = $1 AND foyer_id IS NOT NULL")
            .bind(self.parent_c)
            .fetch_one(self.c.ent.pool())
            .await
            .unwrap();
        payeur(&self.c, self.c.eleves[2], self.parent_c, foyer_c, 100).await;
        self.sync().await;
        self.c.participant(self.s.id, self.c.eleves[2]).await
    }

    async fn ligne(&self, p: Uuid, compte: Uuid) -> crate::voyages_argent_repo::LignePaiement {
        self.c
            .ent
            .voyage_argent_paiements(&self.compta(), self.s.id)
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.participant_id == p && l.compte_id == compte)
            .expect("ligne de paiement")
    }

    async fn avoirs_de(&self, p: Uuid, compte: Uuid) -> Vec<crate::voyages_argent_repo::AvoirVue> {
        self.c
            .ent
            .voyage_avoirs(&self.compta(), self.s.id)
            .await
            .unwrap()
            .into_iter()
            .filter(|a| a.participant_id == p && a.compte_id == compte)
            .collect()
    }
}

// ═════════════════════════════ Aides ═════════════════════════════

#[tokio::test]
async fn aide_reduit_le_reste_n_est_jamais_rendue_en_avoir_et_reste_chez_la_comptabilite() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    // Rien d'ambigu n'entre : source inconnue, montant nul, négatif, à trois décimales, ou plus que la part.
    for (source, montant) in [("loterie", "10"), ("apel", "0"), ("apel", "-5"), ("apel", "12,345"), ("apel", "abc")] {
        let e = x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_c, source, montant)).await.unwrap_err();
        assert!(est_refus(&e) || matches!(e, EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide)), "{source} {montant} : {e:?}");
    }
    let e = x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "500,01")).await.unwrap_err();
    assert!(est_refus(&e), "une aide ne dépasse pas la part : {e:?}");
    // Un payeur qui n'est pas celui de l'élève : « introuvable ».
    assert!(est_introuvable(&x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_a, "apel", "10")).await.unwrap_err()));

    let a = x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "fonds_social", "100")).await.expect("aide");
    assert_eq!((a.sens.as_str(), a.montant_centimes, a.a_restituer), ("aide", 10_000, false));
    let l = x.ligne(x.p1, x.compte_c).await;
    assert_eq!((l.part_centimes, l.verse_centimes, l.aides_centimes, l.reste_centimes), (50_000, 0, 10_000, 40_000));
    // Le versement ne peut plus dépasser ce qui reste dû : 400 € au plus.
    assert!(est_refus(&x.verser(x.p1, x.compte_c, "cheque", "400,01").await.unwrap_err()));
    x.verser(x.p1, x.compte_c, "cheque", "400").await.expect("le reste dû");
    let l = x.ligne(x.p1, x.compte_c).await;
    assert_eq!((l.verse_centimes, l.aides_centimes, l.reste_centimes, l.etat.as_str()), (40_000, 10_000, 0, "solde"));
    // Une aide ne s'ajoute pas au-delà de la part.
    assert!(est_refus(&x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "1")).await.unwrap_err()));

    // L'organisateur ne voit que des nombres : aucune trace de l'aide, de sa source, ni du fonds social.
    let ag = x.c.ent.voyage_argent_etat(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!(ag.soldes, 1);
    let json = serde_json::to_string(&ag).unwrap();
    for interdit in ["aide", "fonds", "apel", "social"] {
        assert!(!json.contains(interdit), "{json}");
    }
    // Rôles : seules la comptabilité et la direction inscrivent, lisent et annulent une aide.
    let attendu = |e: &EntDbError| est_forbidden(e) || est_introuvable(e);
    for acteur in [x.porteur(), x.c.a(x.c.prof2, Role::Prof), x.sec(), x.c.a(x.c.accueil, Role::Accueil), x.parent(x.parent_c)] {
        assert!(attendu(&x.c.ent.voyage_aide_accorder(&acteur, x.s.id, aide(x.p0, x.compte_a, "apel", "1")).await.unwrap_err()));
        assert!(attendu(&x.c.ent.voyage_aides(&acteur, x.s.id).await.unwrap_err()));
        assert!(attendu(&x.c.ent.voyage_aide_annuler(&acteur, x.s.id, a.id, "x").await.unwrap_err()));
    }
    // La consultation nominative des aides est journalisée.
    let liste = x.c.ent.voyage_aides(&x.dir(), x.s.id).await.unwrap();
    assert_eq!(liste.len(), 1);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_argent_acces WHERE sortie_id = $1 AND user_id = $2 AND portee = 'liste_paiements'")
        .bind(x.s.id)
        .bind(x.c.dir)
        .fetch_one(x.c.ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 1);

    // Désistement : l'avoir ne rend que l'argent versé par la famille (400 €), jamais l'aide ; l'aide est signalée.
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p1, "decision_personnelle", None).await.unwrap();
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    assert_eq!(avoirs.len(), 1);
    assert_eq!((avoirs[0].paye_net_centimes, avoirs[0].montant_propose_centimes), (40_000, 40_000), "l'aide n'est pas de l'argent du payeur (CF-69)");
    let liste = x.c.ent.voyage_aides(&x.compta(), x.s.id).await.unwrap();
    assert!(liste[0].a_restituer, "aide d'un participant parti : à restituer à sa source ou à réaffecter");

    // Annulation : écriture inverse motivée, exacte, une seule fois ; l'original reste intact.
    assert!(x.c.ent.voyage_aide_annuler(&x.compta(), x.s.id, a.id, "  ").await.is_err(), "motif obligatoire");
    let inv = x.c.ent.voyage_aide_annuler(&x.compta(), x.s.id, a.id, "Aide retournée à l'APEL après désistement").await.unwrap();
    assert_eq!((inv.sens.as_str(), inv.montant_centimes, inv.annule_id), ("annulation", 10_000, Some(a.id)));
    assert!(matches!(
        x.c.ent.voyage_aide_annuler(&x.compta(), x.s.id, a.id, "encore").await.unwrap_err(),
        EntDbError::Domain(domain_eleve::DomainError::Conflit)
    ));
    assert!(x.c.ent.voyage_aide_annuler(&x.compta(), x.s.id, inv.id, "annuler l'annulation").await.is_err());
    let liste = x.c.ent.voyage_aides(&x.compta(), x.s.id).await.unwrap();
    assert!(liste.iter().any(|l| l.id == a.id && l.sens == "aide" && l.montant_centimes == 10_000));
    assert!(liste.iter().all(|l| !l.a_restituer), "l'aide annulée n'est plus à restituer");
    // Aucune facture : le mode suivi n'écrit pas dans la comptabilité.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_fac_facture WHERE etablissement_id = $1").bind(x.c.etab).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn aide_famille_voit_son_aide_sans_la_source_et_le_responsable_voit_la_part_couverte() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p0, x.compte_a, "fonds_social", "80")).await.unwrap();
    x.verser(x.p0, x.compte_a, "cheque", "50").await.unwrap();
    // Le parent A voit sa part (250 €), son aide (80 €), son versé (50 €), le reste (120 €) — jamais la source.
    let a = x.c.ent.voyage_argent_famille(&x.parent(x.parent_a), x.s.id).await.unwrap();
    assert_eq!((a[0].part_centimes, a[0].aides_centimes, a[0].verse_centimes, a[0].reste_centimes), (25_000, 8_000, 5_000, 12_000));
    let json = serde_json::to_string(&a).unwrap();
    for interdit in ["fonds_social", "social", "apel", "DEC-12"] {
        assert!(!json.contains(interdit), "la famille ne voit jamais d'où vient l'aide : {json}");
    }
    // L'autre payeur du même élève ne voit rien de cette aide.
    let b = x.c.ent.voyage_argent_famille(&x.parent(x.parent_b), x.s.id).await.unwrap();
    assert_eq!((b[0].aides_centimes, b[0].verse_centimes, b[0].reste_centimes), (0, 0, 25_000));
    // Le responsable désigné voit ce qui couvre la part (130 €), sans savoir qu'une partie est une aide.
    x.c.ent.voyage_nominatif_ouvrir(&x.dir(), x.s.id, "Relances des familles par le responsable").await.unwrap();
    let vue = x.c.ent.voyage_argent_paiements_vue(&x.porteur(), x.s.id).await.unwrap();
    let json = serde_json::to_string(&vue).unwrap();
    assert!(!json.contains("aides"), "{json}");
    match vue {
        crate::voyages_argent_repo::PaiementsVue::Responsable(v) => {
            let l = v.iter().find(|l| l.compte_id == x.compte_a).unwrap();
            assert_eq!((l.verse_centimes, l.reste_centimes), (13_000, 12_000));
        }
        crate::voyages_argent_repo::PaiementsVue::Complet(_) => panic!("le porteur ne reçoit jamais la vue complète"),
    }
}

// ═════════════════════════════ Reports de versement ═════════════════════════════

#[tokio::test]
async fn report_passe_l_argent_au_meme_payeur_et_recalcule_l_avoir_du_partant() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "virement", "500").await.unwrap();
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p1, "decision_personnelle", None).await.unwrap();
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    assert_eq!((avoirs.len(), avoirs[0].montant_propose_centimes, avoirs[0].statut.as_str()), (1, 50_000, "propose"));

    // Refus : soi-même, un autre payeur, plus que le versé, montant ambigu, motif absent, rôle sans droit.
    assert!(x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, x.p1, x.compte_c, "10")).await.is_err());
    assert!(est_refus(&x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, x.p0, x.compte_c, "10")).await.unwrap_err()), "p0 n'a pas ce payeur");
    assert!(est_refus(&x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p0, p2, x.compte_a, "10")).await.unwrap_err()), "p2 n'a pas ce payeur");
    assert!(est_refus(&x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "500,01")).await.unwrap_err()));
    assert!(x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "12,345")).await.is_err());
    let mut sans_motif = report(x.p1, p2, x.compte_c, "10");
    sans_motif.motif = "  ".into();
    assert!(x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, sans_motif).await.is_err());
    for acteur in [x.porteur(), x.sec(), x.c.a(x.c.accueil, Role::Accueil), x.parent(x.parent_c)] {
        let e = x.c.ent.voyage_report_effectuer(&acteur, x.s.id, report(x.p1, p2, x.compte_c, "10")).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }

    let r = x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "300")).await.expect("report");
    assert_eq!((r.sens.as_str(), r.montant_centimes), ("report", 30_000));
    // Le receveur a 300 € de versé et 200 € à payer ; le partant n'a plus que 200 € à se faire rendre.
    let l = x.ligne(p2, x.compte_c).await;
    assert_eq!((l.part_centimes, l.verse_centimes, l.reste_centimes), (50_000, 30_000, 20_000));
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    let ouverts: Vec<_> = avoirs.iter().filter(|a| a.statut == "propose").collect();
    assert_eq!(ouverts.len(), 1, "l'ancienne proposition est écartée, une nouvelle est proposée");
    assert_eq!((ouverts[0].paye_net_centimes, ouverts[0].montant_propose_centimes), (20_000, 20_000));
    assert!(avoirs.iter().any(|a| a.statut == "ecarte" && a.montant_propose_centimes == 50_000), "l'ancienne est écartée, pas supprimée");
    // On ne reporte pas plus que ce qui reste chez le partant (200 €), ni plus que le reste dû du receveur.
    assert!(est_refus(&x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "200,01")).await.unwrap_err()));
    // Le versement d'un receveur ne peut pas non plus dépasser son reste.
    assert!(est_refus(&x.verser(p2, x.compte_c, "cheque", "200,01").await.unwrap_err()));
    x.verser(p2, x.compte_c, "cheque", "200").await.expect("le reste dû du receveur");

    // Famille : le payeur voit les reports de SES enfants ; l'autre famille n'en voit aucun.
    let c_ = x.c.ent.voyage_argent_famille(&x.parent(x.parent_c), x.s.id).await.unwrap();
    let ligne_p2 = c_.iter().find(|m| m.participant_id == p2).unwrap();
    assert_eq!((ligne_p2.verse_centimes, ligne_p2.reste_centimes), (50_000, 0));
    assert_eq!(ligne_p2.reports.len(), 1);
    assert!(!ligne_p2.reports[0].cede);
    let a_ = x.c.ent.voyage_argent_famille(&x.parent(x.parent_a), x.s.id).await.unwrap();
    assert!(a_.iter().all(|m| m.reports.is_empty()));

    // Annulation : écriture inverse motivée, une seule fois ; le receveur a encore l'argent reporté ; l'avoir du partant se recalcule.
    assert!(x.c.ent.voyage_report_annuler(&x.compta(), x.s.id, r.id, " ").await.is_err());
    let inv = x.c.ent.voyage_report_annuler(&x.compta(), x.s.id, r.id, "Erreur de saisie : mauvais enfant").await.unwrap();
    assert_eq!((inv.sens.as_str(), inv.annule_id, inv.montant_centimes), ("annulation", Some(r.id), 30_000));
    assert!(matches!(
        x.c.ent.voyage_report_annuler(&x.compta(), x.s.id, r.id, "encore").await.unwrap_err(),
        EntDbError::Domain(domain_eleve::DomainError::Conflit)
    ));
    assert!(x.c.ent.voyage_report_annuler(&x.compta(), x.s.id, inv.id, "annuler l'annulation").await.is_err());
    let l = x.ligne(p2, x.compte_c).await;
    assert_eq!((l.verse_centimes, l.reste_centimes), (20_000, 30_000), "seuls les 200 € versés restent chez le receveur");
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    assert_eq!(avoirs.iter().filter(|a| a.statut == "propose").map(|a| a.montant_propose_centimes).collect::<Vec<_>>(), vec![50_000]);
    // Lecture nominative journalisée.
    assert_eq!(x.c.ent.voyage_reports(&x.compta(), x.s.id).await.unwrap().len(), 2);
    assert!(x.c.ent.voyage_reports(&x.porteur(), x.s.id).await.is_err());
}

#[tokio::test]
async fn report_refuse_quand_l_argent_n_est_plus_chez_le_receveur_ou_quand_l_avoir_est_decide() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "virement", "500").await.unwrap();
    // Report complet vers p2, qui se désiste ensuite : son avoir porte les 500 reportés.
    let r = x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "500")).await.unwrap();
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, p2, "decision_personnelle", None).await.unwrap();
    let a2 = x.avoirs_de(p2, x.compte_c).await;
    assert_eq!((a2.len(), a2[0].paye_net_centimes), (1, 50_000), "l'argent reporté est bien celui du payeur");
    // Le receveur a un avoir en cours : annuler le report le rend « à revoir », sans rien détruire.
    x.c.ent.voyage_report_annuler(&x.compta(), x.s.id, r.id, "Le report était une erreur").await.unwrap();
    assert!(x.avoirs_de(p2, x.compte_c).await.iter().all(|a| a.statut == "a_revoir"));
    // Un avoir RETENU pour le partant interdit de lui retirer de l'argent par un report.
    let y = suivi().await;
    let Some(y) = y else { return };
    y.sync().await;
    let q2 = y.p2_meme_payeur().await;
    y.verser(y.p1, y.compte_c, "virement", "500").await.unwrap();
    y.c.ent.voyage_participant_desister(&y.c.a(y.c.prof, Role::Prof), y.s.id, y.p1, "decision_personnelle", None).await.unwrap();
    let av = y.avoirs_de(y.p1, y.compte_c).await.remove(0);
    y.c.ent.voyage_avoir_decider(&y.compta(), y.s.id, av.id, true, None, None, "Avoir total").await.unwrap();
    let e = y.c.ent.voyage_report_effectuer(&y.compta(), y.s.id, report(y.p1, q2, y.compte_c, "100")).await.unwrap_err();
    assert!(est_refus(&e), "un avoir déjà retenu : la décision se règle d'abord dans la comptabilité : {e:?}");
}

// ═════════════════════════════ Internat ═════════════════════════════

/// Fait de l'élève un interne (séjour d'internat ouvert) et du participant un partant confirmé.
async fn interne_pret(x: &Suivi, eleve: Uuid, participant: Uuid) {
    sqlx::query(
        "INSERT INTO ent_internat_sejour (id, etablissement_id, eleve_id, du, au, origine) VALUES ($1, $2, $3, $4, NULL, 'manuel_direction')",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(eleve)
    .bind(x.c.jour - chrono::Duration::days(60))
    .execute(x.c.ent.pool_verif())
    .await
    .unwrap();
    pret(x, participant).await;
}

async fn pret(x: &Suivi, participant: Uuid) {
    sqlx::query("UPDATE ent_sortie_participant SET etat = 'pret' WHERE id = $1").bind(participant).execute(x.c.ent.pool_verif()).await.unwrap();
}

async fn retraits_actifs(x: &Suivi, eleve: Uuid) -> Vec<(chrono::NaiveDate, chrono::NaiveDate, String, String)> {
    sqlx::query_as(
        "SELECT du_nuit, au_nuit, motif_code, source FROM ent_internat_retrait WHERE eleve_id = $1 AND annule_le IS NULL ORDER BY du_nuit",
    )
    .bind(eleve)
    .fetch_all(x.c.ent.pool_verif())
    .await
    .unwrap()
}

#[tokio::test]
async fn internat_un_interne_confirme_est_retire_des_nuits_du_sejour_et_retabli_s_il_se_desiste() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let (e0, e1) = (x.c.eleves[0], x.c.eleves[1]);
    interne_pret(&x, e0, x.p0).await;
    pret(&x, x.p1).await; // confirmé, mais pas interne : aucun retrait
    let nuit_depart = x.c.jour; // le départ est fixé au jour du cadre (08 h)
    // Rôles : le porteur et la direction synchronisent ; un autre professeur, l'accueil, un parent non.
    for acteur in [x.c.a(x.c.prof2, Role::Prof), x.c.a(x.c.accueil, Role::Accueil), x.parent(x.parent_a)] {
        let e = x.c.ent.voyage_internat_synchroniser(&acteur, x.s.id).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }
    assert!(retraits_actifs(&x, e0).await.is_empty(), "rien n'est posé tant que personne n'a synchronisé");
    let r = x.c.ent.voyage_internat_synchroniser(&x.porteur(), x.s.id).await.expect("synchronisation");
    assert_eq!((r.poses, r.annules, r.inchanges, r.nuits), (1, 0, 0, 2), "départ J 08 h, retour J+2 : les nuits de J et J+1");
    let retraits = retraits_actifs(&x, e0).await;
    assert_eq!(retraits.len(), 1);
    assert_eq!((retraits[0].0, retraits[0].1), (nuit_depart, nuit_depart + chrono::Duration::days(1)));
    assert_eq!((retraits[0].2.as_str(), retraits[0].3.as_str()), ("sejour_scolaire", "voyage"));
    assert!(retraits_actifs(&x, e1).await.is_empty(), "un externe n'est jamais retiré de l'internat");
    // Idempotent : rejouer ne change rien.
    let r = x.c.ent.voyage_internat_synchroniser(&x.dir(), x.s.id).await.unwrap();
    assert_eq!((r.poses, r.annules, r.inchanges), (0, 0, 1));
    assert_eq!(retraits_actifs(&x, e0).await.len(), 1);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_internat_retrait WHERE sortie_id = $1").bind(x.s.id).fetch_one(x.c.ent.pool()).await.unwrap();
    assert_eq!(n, 1, "une seule trace");
    // L'événement ne porte ni nom ni montant : des dates et un code.
    let payload: serde_json::Value = sqlx::query_scalar(
        "SELECT m.payload::jsonb FROM ent_outbox o JOIN ent_metier_event m ON m.id = o.event_id
          WHERE o.etablissement_id = $1 AND o.type = 'internat.attendu_modifie' ORDER BY m.id DESC LIMIT 1",
    )
    .bind(x.c.etab)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(payload["motif_code"], "sejour_scolaire");
    // Le désistement rétablit l'interne dans l'attendu : le retrait est annulé (daté, signé), jamais supprimé.
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p0, "decision_personnelle", None).await.unwrap();
    assert!(retraits_actifs(&x, e0).await.is_empty());
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_retrait WHERE eleve_id = $1").bind(e0).fetch_one(x.c.ent.pool_verif()).await.unwrap();
    assert_eq!(total, 1, "le retrait annulé reste dans l'historique du socle 41");
    let r = x.c.ent.voyage_internat_synchroniser(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!((r.poses, r.annules), (0, 0), "un désisté n'est plus retiré");
}

#[tokio::test]
async fn internat_une_sortie_sans_nuit_ne_retire_personne_et_une_date_changee_se_corrige() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    interne_pret(&x, x.c.eleves[0], x.p0).await;
    x.c.ent.voyage_internat_synchroniser(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!(retraits_actifs(&x, x.c.eleves[0]).await.len(), 1);
    // Le retour est avancé à la veille : une seule nuit ; l'ancien retrait est annulé, le nouveau posé.
    sqlx::query("UPDATE ent_sortie SET fin = debut + interval '30 hours' WHERE id = $1").bind(x.s.id).execute(x.c.ent.pool_verif()).await.unwrap();
    let r = x.c.ent.voyage_internat_synchroniser(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!((r.poses, r.annules, r.nuits), (1, 1, 1));
    let retraits = retraits_actifs(&x, x.c.eleves[0]).await;
    assert_eq!(retraits.len(), 1);
    assert_eq!(retraits[0].0, retraits[0].1, "une seule nuit");
    // Une sortie d'une journée n'a aucune nuit : plus aucun retrait.
    sqlx::query("UPDATE ent_sortie SET fin = debut + interval '6 hours' WHERE id = $1").bind(x.s.id).execute(x.c.ent.pool_verif()).await.unwrap();
    let r = x.c.ent.voyage_internat_synchroniser(&x.porteur(), x.s.id).await.unwrap();
    assert_eq!((r.poses, r.annules, r.nuits), (0, 1, 0));
    assert!(retraits_actifs(&x, x.c.eleves[0]).await.is_empty());
}

// ═════════════════════════════ Base ═════════════════════════════

#[tokio::test]
async fn ajout_seul_et_gardes_de_la_base_pour_les_aides_et_les_reports() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    let a = x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "20")).await.unwrap();
    x.c.ent.voyage_report_effectuer(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "30")).await.unwrap();
    x.c.ent.voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p1, "sante", None).await.ok();
    let pool = x.c.ent.pool_verif();
    for t in ["ent_sortie_aide", "ent_sortie_report", "ent_sortie_internat_retrait"] {
        if t == "ent_sortie_internat_retrait" {
            interne_pret(&x, x.c.eleves[0], x.p0).await;
            x.c.ent.voyage_internat_synchroniser(&x.dir(), x.s.id).await.unwrap();
        }
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {t}")).fetch_one(pool).await.unwrap();
        assert!(n > 0, "{t} doit contenir une ligne");
        let r = sqlx::query(&format!("UPDATE {t} SET etablissement_id = etablissement_id")).execute(pool).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "UPDATE {t}");
        let r = sqlx::query(&format!("DELETE FROM {t}")).execute(pool).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "DELETE {t}");
        let r = sqlx::query(&format!("TRUNCATE {t} CASCADE")).execute(pool).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "TRUNCATE {t}");
    }
    // Une annulation d'aide qui ne reprend pas exactement l'aide est refusée par la base.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, annule_id, motif, decide_par_user_id)
         VALUES ($1,$2,$3,$4,$5,'annulation','apel',1999,$6,'x',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p1)
    .bind(x.compte_c)
    .bind(a.id)
    .bind(x.c.compta)
    .execute(pool)
    .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"));
    // Un report vers un participant qui n'a pas ce payeur est refusé par la base, comme un montant hors bornes.
    for (vers, montant, attendu) in [(x.p0, 10_i64, "23514"), (p2, 0, "23514"), (p2, 10_000_001, "23514")] {
        let r = sqlx::query(
            "INSERT INTO ent_sortie_report (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, motif, decide_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,'report',$7,'x',$8)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(x.p1)
        .bind(vers)
        .bind(x.compte_c)
        .bind(montant)
        .bind(x.c.compta)
        .execute(pool)
        .await;
        assert_eq!(code_sql(&r).as_deref(), Some(attendu), "vers {vers} montant {montant}");
    }
    // Une aide ne vise qu'une part de la sortie : un payeur sans part est refusé.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, decide_par_user_id)
         VALUES ($1,$2,$3,$4,$5,'aide','apel',100,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p1)
    .bind(x.compte_a)
    .bind(x.c.compta)
    .execute(pool)
    .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"));
    // La source « voyage » des retraits d'internat est admise ; une source inventée ne l'est pas.
    let r = sqlx::query(
        "INSERT INTO ent_internat_retrait (id, etablissement_id, eleve_id, du_nuit, au_nuit, motif_code, source, par_user_id)
         VALUES ($1,$2,$3,$4,$4,'sejour_scolaire','inconnue',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.c.eleves[2])
    .bind(x.c.jour)
    .bind(x.c.dir)
    .execute(pool)
    .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"));
    let _ = foyer; // (garde l'import : le fixture de foyer est partagé avec V2a)
}

#[tokio::test]
async fn isolation_un_autre_etablissement_ne_voit_ni_aides_ni_reports_ni_synchronisation() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let aide_id = x.c.ent.voyage_aide_accorder(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "10")).await.unwrap().id;
    let Some(autre) = crate::tests_voyages_v0d::cadre().await else { return };
    let a = autre.a(autre.compta, Role::Compta);
    assert!(est_introuvable(&autre.ent.voyage_aides(&a, x.s.id).await.unwrap_err()));
    assert!(est_introuvable(&autre.ent.voyage_reports(&a, x.s.id).await.unwrap_err()));
    assert!(est_introuvable(&autre.ent.voyage_aide_annuler(&a, x.s.id, aide_id, "x").await.unwrap_err()));
    assert!(est_introuvable(&autre.ent.voyage_aide_accorder(&a, x.s.id, aide(x.p1, x.compte_c, "apel", "1")).await.unwrap_err()));
    assert!(est_introuvable(&autre.ent.voyage_report_effectuer(&a, x.s.id, report(x.p1, x.p0, x.compte_c, "1")).await.unwrap_err()));
    let e = autre.ent.voyage_internat_synchroniser(&autre.a(autre.dir, Role::Direction), x.s.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}
