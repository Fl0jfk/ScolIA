//! Socle 42, lot V3 « argent relié » (aides, reports de versement, internat) contre la base locale de test. Sans
//! `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**. Aucun jour réel codé en dur : les dates viennent du
//! cadre (jour de l'établissement).
//!
//! * `aide_…` : une aide réduit le reste dû, n'est jamais rendue en avoir, ne se voit que de la comptabilité ;
//! * `report_…` : l'argent d'un participant passe à un autre **du même payeur**, l'avoir du partant se recalcule ;
//! * `internat_…` : un interne confirmé est retiré de l'attendu des nuits du séjour, rétabli s'il se désiste ;
//! * `ajout_seul_…`, `base_…`, `isolation_…` : gardes de la base et cloisonnement.

use crate::ref_commun::Acteur;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tests_voyages_v0a::{est_forbidden, est_introuvable, est_refus};
use crate::tests_voyages_v2a::{code_sql, foyer, payeur, suivi, Suivi};
use crate::voyages_argent_v3::{AideSaisie, AideVue, ReportSaisie, ReportVue};
use chrono::{DateTime, Utc};
use ent_auth::Role;
use uuid::Uuid;

/// Instant fixe des écritures de ces tests (F4) : jamais l'heure réelle.
fn instant_fixe() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2031-03-12T11:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

/// Appels « à plat » : la décision appliquée tout de suite rend l'écriture ; une demande en attente est un refus ici.
trait DecisionsPlates {
    async fn voyage_aide_accorder_t(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: AideSaisie,
    ) -> Result<AideVue, EntDbError>;
    async fn voyage_report_effectuer_t(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: ReportSaisie,
    ) -> Result<ReportVue, EntDbError>;
}

impl DecisionsPlates for PgEnt {
    async fn voyage_aide_accorder_t(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: AideSaisie,
    ) -> Result<AideVue, EntDbError> {
        let d = self
            .voyage_aide_accorder_a(a, id, v, instant_fixe())
            .await?;
        d.aide.ok_or(EntDbError::Refus(
            RefusMetier::Conflit,
            "en attente de la seconde validation",
        ))
    }
    async fn voyage_report_effectuer_t(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: ReportSaisie,
    ) -> Result<ReportVue, EntDbError> {
        let d = self
            .voyage_report_effectuer_a(a, id, v, instant_fixe())
            .await?;
        d.report.ok_or(EntDbError::Refus(
            RefusMetier::Conflit,
            "en attente de la seconde validation",
        ))
    }
}

/// Règle les deux paramètres de seconde validation de l'établissement du cadre.
async fn regler_seconde_validation(x: &Suivi, seuil_centimes: i64, fonds_social: bool) {
    for (cle, type_param, entier, booleen) in [
        (
            crate::voyages_argent_v3::CLE_SEUIL_SECONDE_VALIDATION,
            "entier",
            Some(seuil_centimes),
            None,
        ),
        (
            crate::voyages_argent_v3::CLE_FONDS_SOCIAL_SECONDE_VALIDATION,
            "booleen",
            None,
            Some(fonds_social),
        ),
    ] {
        sqlx::query(
            "INSERT INTO ent_parametre (etablissement_id, cle, type_param, valeur_entier, valeur_booleen, version, socle_proprietaire)
             VALUES ($1, $2, $3, $4, $5, 1, 42)
             ON CONFLICT (etablissement_id, cle) DO UPDATE
                SET valeur_entier = EXCLUDED.valeur_entier, valeur_booleen = EXCLUDED.valeur_booleen",
        )
        .bind(x.c.etab)
        .bind(cle)
        .bind(type_param)
        .bind(entier)
        .bind(booleen)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    }
}

/// Le cadre des tests d'argent de V2a, avec la seconde validation **levée** (seuil très haut, fonds social libre) : les
/// tests de V3 qui éprouvent les montants n'ont pas besoin de deux personnes. Les tests de la règle 3 repartent du
/// défaut (`suivi()`).
async fn suivi_v3() -> Option<Suivi> {
    let x = suivi().await?;
    regler_seconde_validation(&x, 10_000_000, false).await;
    Some(x)
}

fn aide(p: Uuid, compte: Uuid, source: &str, montant: &str) -> AideSaisie {
    AideSaisie {
        participant_id: p,
        compte_id: compte,
        source: source.into(),
        montant: montant.into(),
        reference: Some("DEC-12".into()),
        motif: "Décision de la commission".into(),
        cle: format!("aide-{}", Uuid::new_v4().simple()),
    }
}

fn report(de: Uuid, vers: Uuid, compte: Uuid, montant: &str) -> ReportSaisie {
    ReportSaisie {
        de_participant_id: de,
        vers_participant_id: vers,
        compte_id: compte,
        montant: montant.into(),
        motif: "Le frère prend la place".into(),
        cle: format!("report-{}", Uuid::new_v4().simple()),
    }
}

impl Suivi {
    /// Participant du troisième élève, rattaché au **même payeur** que le deuxième (deux enfants, un compte).
    async fn p2_meme_payeur(&self) -> Uuid {
        let foyer_c: Uuid = sqlx::query_scalar(
            "SELECT foyer_id FROM ent_eleve_lien WHERE user_id = $1 AND foyer_id IS NOT NULL",
        )
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
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    // Rien d'ambigu n'entre : source inconnue, montant nul, négatif, à trois décimales, ou plus que la part.
    for (source, montant) in [
        ("loterie", "10"),
        ("apel", "0"),
        ("apel", "-5"),
        ("apel", "12,345"),
        ("apel", "abc"),
    ] {
        let e = x
            .c
            .ent
            .voyage_aide_accorder_t(&x.compta(), x.s.id, aide(x.p1, x.compte_c, source, montant))
            .await
            .unwrap_err();
        assert!(
            est_refus(&e)
                || matches!(
                    e,
                    EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide)
                ),
            "{source} {montant} : {e:?}"
        );
    }
    let e =
        x.c.ent
            .voyage_aide_accorder_t(
                &x.compta(),
                x.s.id,
                aide(x.p1, x.compte_c, "apel", "500,01"),
            )
            .await
            .unwrap_err();
    assert!(est_refus(&e), "une aide ne dépasse pas la part : {e:?}");
    // Un payeur qui n'est pas celui de l'élève : « introuvable ».
    assert!(est_introuvable(
        &x.c.ent
            .voyage_aide_accorder_t(&x.compta(), x.s.id, aide(x.p1, x.compte_a, "apel", "10"))
            .await
            .unwrap_err()
    ));

    let a =
        x.c.ent
            .voyage_aide_accorder_t(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "100"))
            .await
            .expect("aide");
    assert_eq!(
        (a.sens.as_str(), a.montant_centimes, a.a_restituer),
        ("aide", 10_000, false)
    );
    let l = x.ligne(x.p1, x.compte_c).await;
    assert_eq!(
        (
            l.part_centimes,
            l.verse_centimes,
            l.aides_centimes,
            l.reste_centimes
        ),
        (50_000, 0, 10_000, 40_000)
    );
    // Le versement ne peut plus dépasser ce qui reste dû : 400 € au plus.
    assert!(est_refus(
        &x.verser(x.p1, x.compte_c, "cheque", "400,01")
            .await
            .unwrap_err()
    ));
    x.verser(x.p1, x.compte_c, "cheque", "400")
        .await
        .expect("le reste dû");
    let l = x.ligne(x.p1, x.compte_c).await;
    assert_eq!(
        (
            l.verse_centimes,
            l.aides_centimes,
            l.reste_centimes,
            l.etat.as_str()
        ),
        (40_000, 10_000, 0, "solde")
    );
    // Une aide ne s'ajoute pas au-delà de la part.
    assert!(est_refus(
        &x.c.ent
            .voyage_aide_accorder_t(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "1"))
            .await
            .unwrap_err()
    ));

    // L'organisateur ne voit que des nombres : aucune trace de l'aide, de sa source, ni du fonds social.
    let ag =
        x.c.ent
            .voyage_argent_etat(&x.porteur(), x.s.id)
            .await
            .unwrap();
    assert_eq!(ag.soldes, 1);
    let json = serde_json::to_string(&ag).unwrap();
    for interdit in ["aide", "fonds", "apel", "social"] {
        assert!(!json.contains(interdit), "{json}");
    }
    // Rôles : seules la comptabilité et la direction inscrivent, lisent et annulent une aide.
    let attendu = |e: &EntDbError| est_forbidden(e) || est_introuvable(e);
    for acteur in [
        x.porteur(),
        x.c.a(x.c.prof2, Role::Prof),
        x.sec(),
        x.c.a(x.c.accueil, Role::Accueil),
        x.parent(x.parent_c),
    ] {
        assert!(attendu(
            &x.c.ent
                .voyage_aide_accorder_t(&acteur, x.s.id, aide(x.p0, x.compte_a, "apel", "1"))
                .await
                .unwrap_err()
        ));
        assert!(attendu(
            &x.c.ent.voyage_aides(&acteur, x.s.id).await.unwrap_err()
        ));
        assert!(attendu(
            &x.c.ent
                .voyage_aide_annuler(&acteur, x.s.id, a.id, "x")
                .await
                .unwrap_err()
        ));
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
    x.c.ent
        .voyage_participant_desister(
            &x.c.a(x.c.prof, Role::Prof),
            x.s.id,
            x.p1,
            "decision_personnelle",
            None,
        )
        .await
        .unwrap();
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    assert_eq!(avoirs.len(), 1);
    assert_eq!(
        (
            avoirs[0].paye_net_centimes,
            avoirs[0].montant_propose_centimes
        ),
        (40_000, 40_000),
        "l'aide n'est pas de l'argent du payeur (CF-69)"
    );
    let liste = x.c.ent.voyage_aides(&x.compta(), x.s.id).await.unwrap();
    assert!(
        liste[0].a_restituer,
        "aide d'un participant parti : à restituer à sa source ou à réaffecter"
    );

    // Annulation : écriture inverse motivée, exacte, une seule fois ; l'original reste intact.
    assert!(
        x.c.ent
            .voyage_aide_annuler(&x.compta(), x.s.id, a.id, "  ")
            .await
            .is_err(),
        "motif obligatoire"
    );
    let inv =
        x.c.ent
            .voyage_aide_annuler(
                &x.compta(),
                x.s.id,
                a.id,
                "Aide retournée à l'APEL après désistement",
            )
            .await
            .unwrap();
    assert_eq!(
        (inv.sens.as_str(), inv.montant_centimes, inv.annule_id),
        ("annulation", 10_000, Some(a.id))
    );
    assert!(matches!(
        x.c.ent
            .voyage_aide_annuler(&x.compta(), x.s.id, a.id, "encore")
            .await
            .unwrap_err(),
        EntDbError::Domain(domain_eleve::DomainError::Conflit)
    ));
    assert!(x
        .c
        .ent
        .voyage_aide_annuler(&x.compta(), x.s.id, inv.id, "annuler l'annulation")
        .await
        .is_err());
    let liste = x.c.ent.voyage_aides(&x.compta(), x.s.id).await.unwrap();
    assert!(liste
        .iter()
        .any(|l| l.id == a.id && l.sens == "aide" && l.montant_centimes == 10_000));
    assert!(
        liste.iter().all(|l| !l.a_restituer),
        "l'aide annulée n'est plus à restituer"
    );
    // Aucune facture : le mode suivi n'écrit pas dans la comptabilité.
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_fac_facture WHERE etablissement_id = $1")
            .bind(x.c.etab)
            .fetch_one(x.c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn aide_famille_voit_son_aide_sans_la_source_et_le_responsable_voit_la_part_couverte() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    x.c.ent
        .voyage_aide_accorder_t(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "fonds_social", "80"),
        )
        .await
        .unwrap();
    x.verser(x.p0, x.compte_a, "cheque", "50").await.unwrap();
    // Le parent A voit sa part (250 €), son aide (80 €), son versé (50 €), le reste (120 €) — jamais la source.
    let a =
        x.c.ent
            .voyage_argent_famille(&x.parent(x.parent_a), x.s.id)
            .await
            .unwrap();
    assert_eq!(
        (
            a[0].part_centimes,
            a[0].aides_centimes,
            a[0].verse_centimes,
            a[0].reste_centimes
        ),
        (25_000, 8_000, 5_000, 12_000)
    );
    let json = serde_json::to_string(&a).unwrap();
    for interdit in ["fonds_social", "social", "apel", "DEC-12"] {
        assert!(
            !json.contains(interdit),
            "la famille ne voit jamais d'où vient l'aide : {json}"
        );
    }
    // L'autre payeur du même élève ne voit rien de cette aide.
    let b =
        x.c.ent
            .voyage_argent_famille(&x.parent(x.parent_b), x.s.id)
            .await
            .unwrap();
    assert_eq!(
        (
            b[0].aides_centimes,
            b[0].verse_centimes,
            b[0].reste_centimes
        ),
        (0, 0, 25_000)
    );
    // Le responsable désigné voit ce qui couvre la part (130 €), sans savoir qu'une partie est une aide.
    x.c.ent
        .voyage_nominatif_ouvrir(&x.dir(), x.s.id, "Relances des familles par le responsable")
        .await
        .unwrap();
    let vue =
        x.c.ent
            .voyage_argent_paiements_vue(&x.porteur(), x.s.id)
            .await
            .unwrap();
    let json = serde_json::to_string(&vue).unwrap();
    assert!(!json.contains("aides"), "{json}");
    match vue {
        crate::voyages_argent_repo::PaiementsVue::Responsable(v) => {
            let l = v.iter().find(|l| l.compte_id == x.compte_a).unwrap();
            assert_eq!((l.verse_centimes, l.reste_centimes), (13_000, 12_000));
        }
        crate::voyages_argent_repo::PaiementsVue::Complet(_) => {
            panic!("le porteur ne reçoit jamais la vue complète")
        }
    }
}

// ═════════════════════════════ Reports de versement ═════════════════════════════

#[tokio::test]
async fn report_passe_l_argent_au_meme_payeur_et_recalcule_l_avoir_du_partant() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "virement", "500").await.unwrap();
    x.c.ent
        .voyage_participant_desister(
            &x.c.a(x.c.prof, Role::Prof),
            x.s.id,
            x.p1,
            "decision_personnelle",
            None,
        )
        .await
        .unwrap();
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    assert_eq!(
        (
            avoirs.len(),
            avoirs[0].montant_propose_centimes,
            avoirs[0].statut.as_str()
        ),
        (1, 50_000, "propose")
    );

    // Refus : soi-même, un autre payeur, plus que le versé, montant ambigu, motif absent, rôle sans droit.
    assert!(x
        .c
        .ent
        .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, x.p1, x.compte_c, "10"))
        .await
        .is_err());
    assert!(
        est_refus(
            &x.c.ent
                .voyage_report_effectuer_t(
                    &x.compta(),
                    x.s.id,
                    report(x.p1, x.p0, x.compte_c, "10")
                )
                .await
                .unwrap_err()
        ),
        "p0 n'a pas ce payeur"
    );
    assert!(
        est_refus(
            &x.c.ent
                .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p0, p2, x.compte_a, "10"))
                .await
                .unwrap_err()
        ),
        "p2 n'a pas ce payeur"
    );
    assert!(est_refus(
        &x.c.ent
            .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "500,01"))
            .await
            .unwrap_err()
    ));
    assert!(x
        .c
        .ent
        .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "12,345"))
        .await
        .is_err());
    let mut sans_motif = report(x.p1, p2, x.compte_c, "10");
    sans_motif.motif = "  ".into();
    assert!(x
        .c
        .ent
        .voyage_report_effectuer_t(&x.compta(), x.s.id, sans_motif)
        .await
        .is_err());
    for acteur in [
        x.porteur(),
        x.sec(),
        x.c.a(x.c.accueil, Role::Accueil),
        x.parent(x.parent_c),
    ] {
        let e =
            x.c.ent
                .voyage_report_effectuer_t(&acteur, x.s.id, report(x.p1, p2, x.compte_c, "10"))
                .await
                .unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }

    let r =
        x.c.ent
            .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "300"))
            .await
            .expect("report");
    assert_eq!((r.sens.as_str(), r.montant_centimes), ("report", 30_000));
    // Le receveur a 300 € de versé et 200 € à payer ; le partant n'a plus que 200 € à se faire rendre.
    let l = x.ligne(p2, x.compte_c).await;
    assert_eq!(
        (l.part_centimes, l.verse_centimes, l.reste_centimes),
        (50_000, 30_000, 20_000)
    );
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    let ouverts: Vec<_> = avoirs.iter().filter(|a| a.statut == "propose").collect();
    assert_eq!(
        ouverts.len(),
        1,
        "l'ancienne proposition est écartée, une nouvelle est proposée"
    );
    assert_eq!(
        (
            ouverts[0].paye_net_centimes,
            ouverts[0].montant_propose_centimes
        ),
        (20_000, 20_000)
    );
    assert!(
        avoirs
            .iter()
            .any(|a| a.statut == "ecarte" && a.montant_propose_centimes == 50_000),
        "l'ancienne est écartée, pas supprimée"
    );
    // On ne reporte pas plus que ce qui reste chez le partant (200 €), ni plus que le reste dû du receveur.
    assert!(est_refus(
        &x.c.ent
            .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "200,01"))
            .await
            .unwrap_err()
    ));
    // Le versement d'un receveur ne peut pas non plus dépasser son reste.
    assert!(est_refus(
        &x.verser(p2, x.compte_c, "cheque", "200,01")
            .await
            .unwrap_err()
    ));
    x.verser(p2, x.compte_c, "cheque", "200")
        .await
        .expect("le reste dû du receveur");

    // Famille : le payeur voit les reports de SES enfants ; l'autre famille n'en voit aucun.
    let c_ =
        x.c.ent
            .voyage_argent_famille(&x.parent(x.parent_c), x.s.id)
            .await
            .unwrap();
    let ligne_p2 = c_.iter().find(|m| m.participant_id == p2).unwrap();
    assert_eq!(
        (ligne_p2.verse_centimes, ligne_p2.reste_centimes),
        (50_000, 0)
    );
    assert_eq!(ligne_p2.reports.len(), 1);
    assert!(!ligne_p2.reports[0].cede);
    let a_ =
        x.c.ent
            .voyage_argent_famille(&x.parent(x.parent_a), x.s.id)
            .await
            .unwrap();
    assert!(a_.iter().all(|m| m.reports.is_empty()));

    // Annulation : écriture inverse motivée, une seule fois ; le receveur a encore l'argent reporté ; l'avoir du partant se recalcule.
    assert!(x
        .c
        .ent
        .voyage_report_annuler(&x.compta(), x.s.id, r.id, " ")
        .await
        .is_err());
    let inv =
        x.c.ent
            .voyage_report_annuler(
                &x.compta(),
                x.s.id,
                r.id,
                "Erreur de saisie : mauvais enfant",
            )
            .await
            .unwrap();
    assert_eq!(
        (inv.sens.as_str(), inv.annule_id, inv.montant_centimes),
        ("annulation", Some(r.id), 30_000)
    );
    assert!(matches!(
        x.c.ent
            .voyage_report_annuler(&x.compta(), x.s.id, r.id, "encore")
            .await
            .unwrap_err(),
        EntDbError::Domain(domain_eleve::DomainError::Conflit)
    ));
    assert!(x
        .c
        .ent
        .voyage_report_annuler(&x.compta(), x.s.id, inv.id, "annuler l'annulation")
        .await
        .is_err());
    let l = x.ligne(p2, x.compte_c).await;
    assert_eq!(
        (l.verse_centimes, l.reste_centimes),
        (20_000, 30_000),
        "seuls les 200 € versés restent chez le receveur"
    );
    let avoirs = x.avoirs_de(x.p1, x.compte_c).await;
    assert_eq!(
        avoirs
            .iter()
            .filter(|a| a.statut == "propose")
            .map(|a| a.montant_propose_centimes)
            .collect::<Vec<_>>(),
        vec![50_000]
    );
    // Lecture nominative journalisée.
    assert_eq!(
        x.c.ent
            .voyage_reports(&x.compta(), x.s.id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(x.c.ent.voyage_reports(&x.porteur(), x.s.id).await.is_err());
}

#[tokio::test]
async fn report_refuse_quand_l_argent_n_est_plus_chez_le_receveur_ou_quand_l_avoir_est_decide() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "virement", "500").await.unwrap();
    // Report complet vers p2, qui se désiste ensuite : son avoir porte les 500 reportés.
    let r =
        x.c.ent
            .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "500"))
            .await
            .unwrap();
    x.c.ent
        .voyage_participant_desister(
            &x.c.a(x.c.prof, Role::Prof),
            x.s.id,
            p2,
            "decision_personnelle",
            None,
        )
        .await
        .unwrap();
    let a2 = x.avoirs_de(p2, x.compte_c).await;
    assert_eq!(
        (a2.len(), a2[0].paye_net_centimes),
        (1, 50_000),
        "l'argent reporté est bien celui du payeur"
    );
    // Le receveur a un avoir en cours : annuler le report le rend « à revoir », sans rien détruire.
    x.c.ent
        .voyage_report_annuler(&x.compta(), x.s.id, r.id, "Le report était une erreur")
        .await
        .unwrap();
    assert!(x
        .avoirs_de(p2, x.compte_c)
        .await
        .iter()
        .all(|a| a.statut == "a_revoir"));
    // Un avoir RETENU pour le partant interdit de lui retirer de l'argent par un report.
    let y = suivi_v3().await;
    let Some(y) = y else { return };
    y.sync().await;
    let q2 = y.p2_meme_payeur().await;
    y.verser(y.p1, y.compte_c, "virement", "500").await.unwrap();
    y.c.ent
        .voyage_participant_desister(
            &y.c.a(y.c.prof, Role::Prof),
            y.s.id,
            y.p1,
            "decision_personnelle",
            None,
        )
        .await
        .unwrap();
    let av = y.avoirs_de(y.p1, y.compte_c).await.remove(0);
    y.c.ent
        .voyage_avoir_decider(&y.compta(), y.s.id, av.id, true, None, None, "Avoir total")
        .await
        .unwrap();
    let e =
        y.c.ent
            .voyage_report_effectuer_t(&y.compta(), y.s.id, report(y.p1, q2, y.compte_c, "100"))
            .await
            .unwrap_err();
    assert!(
        est_refus(&e),
        "un avoir déjà retenu : la décision se règle d'abord dans la comptabilité : {e:?}"
    );
}

// ═════════════════════════════ Internat (D-069, règles 1 et 2) ═════════════════════════════

/// Fait de l'élève un interne (séjour d'internat ouvert, présent toutes les nuits).
async fn interne(x: &Suivi, eleve: Uuid) {
    let sejour = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_internat_sejour (id, etablissement_id, eleve_id, du, au, origine) VALUES ($1, $2, $3, $4, NULL, 'manuel_direction')",
    )
    .bind(sejour)
    .bind(x.c.etab)
    .bind(eleve)
    .bind(x.c.jour - chrono::Duration::days(60))
    .execute(x.c.ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_internat_rythme (id, etablissement_id, sejour_id, du, dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi)
         VALUES ($1, $2, $3, $4, true, true, true, true, true, true, true)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(sejour)
    .bind(x.c.jour - chrono::Duration::days(60))
    .execute(x.c.ent.pool_verif())
    .await
    .unwrap();
}

/// Constate (en base, V3b n'existant pas encore) l'état du participant.
async fn etat_sql(x: &Suivi, participant: Uuid, etat: &str) {
    sqlx::query("UPDATE ent_sortie_participant SET etat = $2 WHERE id = $1")
        .bind(participant)
        .bind(etat)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
}

async fn interne_parti(x: &Suivi, eleve: Uuid, participant: Uuid) {
    interne(x, eleve).await;
    etat_sql(x, participant, "parti").await;
}

/// Statut de l'élève à l'attendu de la nuit (`attendu` ou `retire`), lu par la fonction SQL de l'internat.
async fn statut_nuit(x: &Suivi, eleve: Uuid, nuit: chrono::NaiveDate) -> Option<String> {
    sqlx::query_scalar(
        "SELECT statut FROM ent_internat_attendu_calcule($1, $2) WHERE eleve_id = $3",
    )
    .bind(x.c.etab)
    .bind(nuit)
    .bind(eleve)
    .fetch_optional(x.c.ent.pool_verif())
    .await
    .unwrap()
}

async fn retraits_actifs(
    x: &Suivi,
    eleve: Uuid,
) -> Vec<(chrono::NaiveDate, chrono::NaiveDate, String, String)> {
    sqlx::query_as(
        "SELECT du_nuit, au_nuit, motif_code, source FROM ent_internat_retrait WHERE eleve_id = $1 AND annule_le IS NULL ORDER BY du_nuit",
    )
    .bind(eleve)
    .fetch_all(x.c.ent.pool_verif())
    .await
    .unwrap()
}

/// Dernière cause de fin de retrait de voyage tracée par la synchronisation.
async fn causes_de_fin(x: &Suivi) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT cause FROM ent_sortie_internat_retrait_fin WHERE sortie_id = $1 ORDER BY le, id",
    )
    .bind(x.s.id)
    .fetch_all(x.c.ent.pool_verif())
    .await
    .unwrap()
}

async fn commande(
    x: &Suivi,
    cmd: crate::voyages::Commande,
    vers: Option<&str>,
) -> Result<crate::voyages_repo::SortieLigne, EntDbError> {
    commande_a(x, cmd, vers, Utc::now()).await
}

/// Comme `commande`, à l'instant fixé (F-e : jamais l'heure réelle quand le test en dépend).
async fn commande_a(
    x: &Suivi,
    cmd: crate::voyages::Commande,
    vers: Option<&str>,
    maintenant: DateTime<Utc>,
) -> Result<crate::voyages_repo::SortieLigne, EntDbError> {
    let version: i32 = sqlx::query_scalar("SELECT version FROM ent_sortie WHERE id = $1")
        .bind(x.s.id)
        .fetch_one(x.c.ent.pool())
        .await
        .unwrap();
    x.c.ent
        .voyage_commande_a(
            &x.dir(),
            x.s.id,
            cmd,
            crate::voyages_repo::ParamsCommande {
                version: Some(version),
                motif: Some("Décision de la direction".into()),
                vers: vers.map(str::to_string),
                ..Default::default()
            },
            maintenant,
        )
        .await
}

fn jours(x: &Suivi, n: i64) -> chrono::NaiveDate {
    x.c.jour + chrono::Duration::days(n)
}

#[tokio::test]
async fn internat_pas_de_depart_constate_eleve_attendu_puis_retire_puis_attendu_au_retour_anticipe()
{
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let (e0, e1) = (x.c.eleves[0], x.c.eleves[1]);
    interne(&x, e0).await;
    interne(&x, e1).await;
    // Règle 1 — « prêt » n'est pas « parti » : l'élève reste attendu et appelé la nuit du départ.
    etat_sql(&x, x.p0, "pret").await;
    etat_sql(&x, x.p1, "pret").await;
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!(
        (r.poses, r.annules, r.respectes),
        (0, 0, 0),
        "aucun retrait avant le départ constaté"
    );
    assert!(retraits_actifs(&x, e0).await.is_empty());
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu"),
        "prêt, non parti : attendu"
    );
    // Rôles : le porteur et la direction synchronisent ; un autre professeur, l'accueil, un parent non.
    for acteur in [
        x.c.a(x.c.prof2, Role::Prof),
        x.c.a(x.c.accueil, Role::Accueil),
        x.parent(x.parent_a),
    ] {
        let e =
            x.c.ent
                .voyage_internat_synchroniser_a(&acteur, x.s.id, instant_fixe())
                .await
                .unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }
    // Départ constaté de e0 seulement : le retrait est posé, effectif aux deux nuits du séjour (J, J+1), pas à J+2.
    etat_sql(&x, x.p0, "parti").await;
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!(
        (r.poses, r.annules, r.inchanges, r.nuits),
        (1, 0, 0, 2),
        "départ J 08 h, retour J+2 : nuits J et J+1"
    );
    let retraits = retraits_actifs(&x, e0).await;
    assert_eq!(retraits.len(), 1);
    assert_eq!((retraits[0].0, retraits[0].1), (jours(&x, 0), jours(&x, 1)));
    assert_eq!(
        (retraits[0].2.as_str(), retraits[0].3.as_str()),
        ("sejour_scolaire", "voyage")
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("retire"),
        "parti : non attendu"
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 1)).await.as_deref(),
        Some("retire")
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 2)).await.as_deref(),
        Some("attendu"),
        "la nuit du retour : attendu"
    );
    assert_eq!(
        statut_nuit(&x, e1, jours(&x, 0)).await.as_deref(),
        Some("attendu"),
        "e1 est encore « prêt » : attendu"
    );
    // Idempotent : rejouer ne change rien, une seule trace, un événement sans nom ni montant.
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.dir(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!((r.poses, r.annules, r.inchanges), (0, 0, 1));
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_sortie_internat_retrait WHERE sortie_id = $1")
            .bind(x.s.id)
            .fetch_one(x.c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 1, "une seule trace");
    let payload: serde_json::Value = sqlx::query_scalar(
        "SELECT m.payload::jsonb FROM ent_outbox o JOIN ent_metier_event m ON m.id = o.event_id
          WHERE o.etablissement_id = $1 AND o.type = 'internat.attendu_modifie' ORDER BY m.id DESC LIMIT 1",
    )
    .bind(x.c.etab)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(payload["motif_code"], "sejour_scolaire");
    // Retour anticipé (règle 2) : e0 rentre le jour J+1 à midi ; la nuit J+1 le retrouve attendu, dans la même transaction.
    let retour = (jours(&x, 1)).and_hms_opt(12, 0, 0).unwrap().and_utc();
    for acteur in [x.c.a(x.c.prof2, Role::Prof), x.parent(x.parent_a)] {
        let e =
            x.c.ent
                .voyage_participant_retour_anticipe_a(&acteur, x.s.id, x.p0, "sante", None, retour)
                .await
                .unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }
    let e =
        x.c.ent
            .voyage_participant_retour_anticipe_a(&x.porteur(), x.s.id, x.p1, "sante", None, retour)
            .await
            .unwrap_err();
    assert!(
        est_refus(&e),
        "un participant « prêt » n'est pas parti : il ne rentre pas : {e:?}"
    );
    let r =
        x.c.ent
            .voyage_participant_retour_anticipe_a(&x.porteur(), x.s.id, x.p0, "sante", None, retour)
            .await
            .unwrap();
    assert_eq!(
        (r.poses, r.annules),
        (1, 1),
        "ancien retrait annulé, nouveau retrait jusqu'à la veille du retour"
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("retire"),
        "la nuit passée dehors reste retirée"
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 1)).await.as_deref(),
        Some("attendu"),
        "revenu plus tôt : attendu à nouveau"
    );
    assert_eq!(causes_de_fin(&x).await, vec!["retour_anticipe".to_string()]);
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!(
        (r.poses, r.annules, r.inchanges),
        (0, 0, 1),
        "la resynchronisation est stable"
    );
    // Un participant déjà rentré ne rentre pas une seconde fois.
    let e =
        x.c.ent
            .voyage_participant_retour_anticipe_a(&x.porteur(), x.s.id, x.p0, "sante", None, retour)
            .await
            .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
}

#[tokio::test]
async fn internat_desistement_et_retrait_d_un_participant_retablissent_l_attendu_dans_la_meme_transaction(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("retire")
    );
    x.c.ent
        .voyage_participant_desister(
            &x.c.a(x.c.prof, Role::Prof),
            x.s.id,
            x.p0,
            "decision_personnelle",
            None,
        )
        .await
        .unwrap();
    assert!(retraits_actifs(&x, e0).await.is_empty());
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu"),
        "rétabli sans nouvelle synchronisation"
    );
    let total: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_internat_retrait WHERE eleve_id = $1")
            .bind(e0)
            .fetch_one(x.c.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(
        total, 1,
        "le retrait annulé reste dans l'historique du socle 41"
    );
    assert_eq!(causes_de_fin(&x).await, vec!["participant".to_string()]);
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!((r.poses, r.annules), (0, 0), "un désisté n'est plus retiré");
    // Retrait d'un participant par l'établissement : même effet.
    let e1 = x.c.eleves[1];
    interne_parti(&x, e1, x.p1).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(
        statut_nuit(&x, e1, jours(&x, 0)).await.as_deref(),
        Some("retire")
    );
    x.c.ent
        .voyage_participant_retirer(
            &x.c.a(x.c.prof, Role::Prof),
            x.s.id,
            x.p1,
            "decision_etablissement",
            Some("Décision de la direction"),
        )
        .await
        .unwrap();
    assert_eq!(
        statut_nuit(&x, e1, jours(&x, 0)).await.as_deref(),
        Some("attendu")
    );
}

#[tokio::test]
async fn internat_annulation_ou_reouverture_de_la_sortie_retablit_l_attendu_sans_synchronisation_manuelle(
) {
    use crate::voyages::Commande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("retire")
    );
    // Réouverture d'une étape amont : la sortie n'est plus confirmée, l'élève redevient attendu.
    commande(&x, Commande::Rouvrir, Some("a_autoriser"))
        .await
        .unwrap();
    assert!(retraits_actifs(&x, e0).await.is_empty());
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu")
    );
    // Refus ensuite : toujours aucun retrait, rien n'est reposé.
    commande(&x, Commande::Refuser, None).await.unwrap();
    assert!(retraits_actifs(&x, e0).await.is_empty());
    assert_eq!(causes_de_fin(&x).await, vec!["sortie".to_string()]);
}

#[tokio::test]
async fn internat_sortie_annulee_en_cours_ne_garde_que_les_nuits_deja_passees_dehors() {
    use crate::voyages::Commande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    // Annulée avant le départ réel (le séjour est dans le futur du cadre) : plus aucune nuit passée dehors, aucun retrait.
    commande(&x, Commande::Annuler, None).await.unwrap();
    assert!(retraits_actifs(&x, e0).await.is_empty());
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu")
    );
    assert_eq!(causes_de_fin(&x).await, vec!["sortie".to_string()]);
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.dir(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!(
        (r.poses, r.annules),
        (0, 0),
        "une sortie annulée ne repose rien"
    );
}

#[tokio::test]
async fn internat_annulee_apres_le_depart_les_nuits_passees_restent_retirees() {
    use crate::voyages::Commande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    // Le séjour a commencé il y a trois jours et dure encore deux jours, PAR RAPPORT À L'INSTANT FIXE du test (F-e : ni
    // `now()` ni l'heure réelle, qui feraient vaciller le test autour de minuit à Paris).
    let t = instant_fixe();
    sqlx::query("UPDATE ent_sortie SET debut = $2 - interval '3 days', fin = $2 + interval '2 days' WHERE id = $1")
        .bind(x.s.id)
        .bind(t)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, t)
        .await
        .unwrap();
    commande_a(&x, Commande::Annuler, None, t).await.unwrap();
    // Dernière nuit retirée = la veille du jour de l'annulation (jour de l'établissement, lu dans `annulee_le`, M-4).
    let veille: chrono::NaiveDate = sqlx::query_scalar(
        "SELECT ent_aujourdhui_etab($1, annulee_le) - 1 FROM ent_sortie WHERE id = $2",
    )
    .bind(x.c.etab)
    .bind(x.s.id)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    let retraits = retraits_actifs(&x, e0).await;
    assert_eq!(retraits.len(), 1, "{retraits:?}");
    assert_eq!(
        retraits[0].1, veille,
        "seules les nuits déjà passées dehors restent retirées"
    );
    assert_eq!(statut_nuit(&x, e0, veille).await.as_deref(), Some("retire"));
    assert_eq!(
        statut_nuit(&x, e0, veille + chrono::Duration::days(1))
            .await
            .as_deref(),
        Some("attendu")
    );
}

#[tokio::test]
async fn internat_dates_modifiees_par_l_api_deplacent_les_retraits_sans_synchronisation_manuelle() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(retraits_actifs(&x, e0).await[0].1, jours(&x, 1));
    // Le retour est avancé à la veille au soir (J 08 h → J+1 14 h) : une seule nuit, par la route de modification.
    let mut m = x.c.saisie("Voyage à Rome", "voyage");
    m.fin = m.debut + chrono::Duration::hours(30);
    let s =
        x.c.ent
            .voyage_modifier(&x.porteur(), x.s.id, x.s.version, m)
            .await
            .expect("modification");
    assert_eq!(s.fin, s.debut + chrono::Duration::hours(30));
    let retraits = retraits_actifs(&x, e0).await;
    assert_eq!(retraits.len(), 1);
    assert_eq!(
        (retraits[0].0, retraits[0].1),
        (jours(&x, 0), jours(&x, 0)),
        "une seule nuit"
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 1)).await.as_deref(),
        Some("attendu")
    );
    assert_eq!(causes_de_fin(&x).await, vec!["dates".to_string()]);
    // Une sortie d'une journée n'a aucune nuit : plus aucun retrait.
    let mut m = x.c.saisie("Voyage à Rome", "voyage");
    m.fin = m.debut + chrono::Duration::hours(6);
    x.c.ent
        .voyage_modifier(&x.porteur(), x.s.id, s.version, m)
        .await
        .expect("modification");
    assert!(retraits_actifs(&x, e0).await.is_empty());
}

#[tokio::test]
async fn internat_annulation_du_cpe_est_memorisee_et_respectee_a_la_synchronisation_suivante() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    let retrait: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_internat_retrait WHERE eleve_id = $1 AND annule_le IS NULL",
    )
    .bind(e0)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    // Le CPE voit l'enfant à l'internat : il annule le retrait côté internat (même chemin que le socle 41).
    x.c.ent
        .annuler_retrait_a(x.c.etab, x.c.dir, "direction", retrait, instant_fixe())
        .await
        .expect("annulation par la direction de l'internat");
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu")
    );
    // La synchronisation suivante ne le retire PAS de nouveau, même quand les dates changent ; elle le dit.
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!((r.poses, r.annules, r.respectes), (0, 0, 1));
    assert!(retraits_actifs(&x, e0).await.is_empty());
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu"),
        "le CPE a vu l'élève : il reste attendu"
    );
    // Même résultat par la voie « live » de l'attendu quand la sortie est prête ou en cours : l'annulation prime.
    sqlx::query("UPDATE ent_sortie SET etat = 'en_cours' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu")
    );
}

// ═════════════════════════════ Règle 3 : seconde validation, lien avec l'élève (M3, F1, F2) ═════════════════════════════

fn est_interdit_metier(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Interdit, _))
}

/// Un second compte du personnel « compta », rattaché à un élève (lien familial) : un parent qui travaille à la comptabilité.
async fn compta_parent_de(x: &Suivi, eleve: Uuid) -> Acteur<'static> {
    let u = crate::tests_voyages_v0a::compte(&x.c.ent, x.c.etab, "compta").await;
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, lien) VALUES ($1, $2, $3, $4, 'pere')")
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(eleve)
        .bind(u)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    x.c.a(u, Role::Compta)
}

#[tokio::test]
async fn regle3_le_decideur_n_est_jamais_lie_a_l_eleve_ni_payeur_et_le_motif_est_obligatoire() {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    // Motif vide : refusé, aide comme report (le motif est obligatoire et historisé).
    for motif in ["", "   "] {
        let mut a = aide(x.p1, x.compte_c, "apel", "10");
        a.motif = motif.into();
        assert!(x
            .c
            .ent
            .voyage_aide_accorder_a(&x.compta(), x.s.id, a, instant_fixe())
            .await
            .is_err());
        let mut r = report(x.p1, p2, x.compte_c, "10");
        r.motif = motif.into();
        assert!(x
            .c
            .ent
            .voyage_report_effectuer_a(&x.compta(), x.s.id, r, instant_fixe())
            .await
            .is_err());
    }
    // Clé d'idempotence trop courte ou à caractères libres : refusée.
    for cle in ["court", "avec des espaces !"] {
        let mut a = aide(x.p1, x.compte_c, "apel", "10");
        a.cle = cle.into();
        assert!(
            x.c.ent
                .voyage_aide_accorder_a(&x.compta(), x.s.id, a, instant_fixe())
                .await
                .is_err(),
            "{cle}"
        );
    }
    // Décideur parent de l'élève aidé : interdit, aide, report et annulation.
    let parent_de_p1 = compta_parent_de(&x, x.c.eleves[1]).await;
    let e =
        x.c.ent
            .voyage_aide_accorder_a(
                &parent_de_p1,
                x.s.id,
                aide(x.p1, x.compte_c, "apel", "10"),
                instant_fixe(),
            )
            .await
            .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // Il l'est aussi pour un report qui touche son enfant (côté receveur comme côté cédant).
    let e =
        x.c.ent
            .voyage_report_effectuer_a(
                &parent_de_p1,
                x.s.id,
                report(x.p1, p2, x.compte_c, "10"),
                instant_fixe(),
            )
            .await
            .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    let e =
        x.c.ent
            .voyage_report_effectuer_a(
                &parent_de_p1,
                x.s.id,
                report(p2, x.p1, x.compte_c, "10"),
                instant_fixe(),
            )
            .await
            .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // Rien n'a été écrit.
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_sortie_argent_demande WHERE sortie_id = $1")
            .bind(x.s.id)
            .fetch_one(x.c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 0);
    // Le même parent peut décider pour l'enfant d'un AUTRE foyer (aide sur p0, compte A) : seul son enfant est protégé.
    let ok =
        x.c.ent
            .voyage_aide_accorder_a(
                &parent_de_p1,
                x.s.id,
                aide(x.p0, x.compte_a, "apel", "10"),
                instant_fixe(),
            )
            .await
            .expect("aide pour un autre enfant");
    let aide_a = ok.aide.expect("sous le seuil : appliquée").id;
    // Payeur du compte : interdit (personne qui paie pour ce compte, même sans lien avec l'élève).
    let foyer_c: Uuid = sqlx::query_scalar("SELECT foyer_id FROM ent_fin_compte WHERE id = $1")
        .bind(x.compte_c)
        .fetch_one(x.c.ent.pool_verif())
        .await
        .unwrap();
    let payeur_compta = {
        let u = crate::tests_voyages_v0a::compte(&x.c.ent, x.c.etab, "compta").await;
        payeur(&x.c, x.c.eleves[2], u, foyer_c, 0).await;
        x.c.a(u, Role::Compta)
    };
    let e =
        x.c.ent
            .voyage_aide_accorder_a(
                &payeur_compta,
                x.s.id,
                aide(x.p1, x.compte_c, "apel", "10"),
                instant_fixe(),
            )
            .await
            .unwrap_err();
    assert!(est_interdit_metier(&e), "payeur du compte : {e:?}");
    // L'annulation d'une aide obéit à la même règle : le parent de p0 ne l'annule pas.
    let parent_de_p0 = compta_parent_de(&x, x.c.eleves[0]).await;
    let e =
        x.c.ent
            .voyage_aide_annuler_a(
                &parent_de_p0,
                x.s.id,
                aide_a,
                "Erreur de saisie",
                instant_fixe(),
            )
            .await
            .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    x.c.ent
        .voyage_aide_annuler_a(
            &x.compta(),
            x.s.id,
            aide_a,
            "Erreur de saisie",
            instant_fixe(),
        )
        .await
        .expect("une autre personne annule");
    // La base elle-même refuse le contournement : une aide insérée au nom d'un parent de l'élève est rejetée.
    let demande_a: Uuid =
        sqlx::query_scalar("SELECT demande_id FROM ent_sortie_aide WHERE id = $1")
            .bind(aide_a)
            .fetch_one(x.c.ent.pool_verif())
            .await
            .unwrap();
    let r = sqlx::query(
        "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, motif, demande_id, decide_par_user_id, decide_le)
         VALUES ($1,$2,$3,$4,$5,'aide','apel',100,'x',$6,$7, now())",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p0)
    .bind(x.compte_a)
    .bind(demande_a)
    .bind(parent_de_p0.user_id)
    .execute(x.c.ent.pool_verif())
    .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"));
}

#[tokio::test]
async fn regle3_au_dela_du_seuil_ou_pour_le_fonds_social_une_seconde_personne_valide() {
    use crate::voyages_argent_v3::StatutDemande;
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    let ent = &x.c.ent;
    // Défaut prudent : 150 € ne déclenchent pas la seconde validation (strictement au-dessus), 150,01 € oui.
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "150"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        d.statut,
        StatutDemande::Appliquee,
        "150,00 € : sous le seuil"
    );
    assert!(
        d.avertissement_motif.contains("situation sociale"),
        "F2 : avertissement à la saisie du motif"
    );
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "apel", "150,01"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        (d.statut, d.aide.is_none()),
        (StatutDemande::EnAttente, true),
        "150,01 € : seconde personne"
    );
    let demande_apel = d.demande_id;
    // Fonds social : toujours une seconde personne par défaut, même pour 1 €.
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "fonds_social", "1"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(d.statut, StatutDemande::EnAttente);
    let demande_fonds = d.demande_id;
    // En attente : rien n'est encore dû en moins (seuls les 150 € appliqués comptent).
    assert_eq!(x.ligne(x.p1, x.compte_c).await.aides_centimes, 15_000);
    assert_eq!(x.ligne(x.p0, x.compte_a).await.aides_centimes, 0);
    // F1 : le demandeur ne se valide pas lui-même.
    let e = ent
        .voyage_demande_valider_a(&x.compta(), x.s.id, demande_apel, instant_fixe())
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // Ni un parent de l'élève, ni un non-comptable ne valident.
    let parent_de_p0 = compta_parent_de(&x, x.c.eleves[0]).await;
    let e = ent
        .voyage_demande_valider_a(&parent_de_p0, x.s.id, demande_apel, instant_fixe())
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    for acteur in [x.porteur(), x.sec(), x.parent(x.parent_c)] {
        let e = ent
            .voyage_demande_valider_a(&acteur, x.s.id, demande_apel, instant_fixe())
            .await
            .unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{e:?}");
    }
    // La direction (autre personne, libre de tout lien) valide : l'aide est écrite, au nom du valideur.
    let v = ent
        .voyage_demande_valider_a(&x.dir(), x.s.id, demande_apel, instant_fixe())
        .await
        .expect("seconde validation");
    assert_eq!(v.statut, StatutDemande::Appliquee);
    let aide_valide = v.aide.expect("aide écrite");
    assert_eq!(aide_valide.montant_centimes, 15_001);
    let (decide_par, demande_par): (Uuid, Uuid) = sqlx::query_as(
        "SELECT a.decide_par_user_id, d.demande_par_user_id FROM ent_sortie_aide a JOIN ent_sortie_argent_demande d ON d.id = a.demande_id WHERE a.id = $1",
    )
    .bind(aide_valide.id)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (decide_par, demande_par),
        (x.c.dir, x.c.compta),
        "valideur et demandeur sont tracés, distincts"
    );
    assert_eq!(x.ligne(x.p0, x.compte_a).await.aides_centimes, 15_001);
    // Compare-et-écrit : une demande déjà tranchée ne se retranche pas (double clic = une seule écriture).
    let e = ent
        .voyage_demande_valider_a(&x.dir(), x.s.id, demande_apel, instant_fixe())
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::Conflit)),
        "{e:?}"
    );
    // Refus : motif obligatoire ; le demandeur peut seulement retirer sa propre demande.
    assert!(ent
        .voyage_demande_refuser_a(&x.dir(), x.s.id, demande_fonds, "  ", instant_fixe())
        .await
        .is_err());
    let st = ent
        .voyage_demande_refuser_a(
            &x.dir(),
            x.s.id,
            demande_fonds,
            "Hors critères de la commission",
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(st, StatutDemande::Refusee);
    let e = ent
        .voyage_demande_valider_a(&x.dir(), x.s.id, demande_fonds, instant_fixe())
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::Conflit)),
        "{e:?}"
    );
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "fonds_social", "2"),
            instant_fixe(),
        )
        .await
        .unwrap();
    let st = ent
        .voyage_demande_refuser_a(
            &x.compta(),
            x.s.id,
            d.demande_id,
            "Saisie en double",
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(st, StatutDemande::Retiree, "le demandeur retire sa demande");
    // Report au-delà du seuil : même circuit.
    x.verser(x.p1, x.compte_c, "cheque", "300").await.unwrap();
    let d = ent
        .voyage_report_effectuer_a(
            &x.compta(),
            x.s.id,
            report(x.p1, p2, x.compte_c, "200"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        (d.statut, d.report.is_none()),
        (StatutDemande::EnAttente, true)
    );
    let v = ent
        .voyage_demande_valider_a(&x.dir(), x.s.id, d.demande_id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(v.report.expect("report écrit").montant_centimes, 20_000);
    // La liste des demandes porte les statuts, la consultation est journalisée.
    let l = ent.voyage_demandes(&x.dir(), x.s.id).await.unwrap();
    let statuts: Vec<&str> = l.iter().map(|d| d.statut.as_str()).collect();
    assert_eq!(statuts.iter().filter(|s| **s == "appliquee").count(), 3);
    assert_eq!(statuts.iter().filter(|s| **s == "refusee").count(), 1);
    assert_eq!(statuts.iter().filter(|s| **s == "retiree").count(), 1);
    assert!(
        l.iter().all(|d| !d.motif.is_empty()),
        "motif obligatoire partout"
    );
    // Seuil paramétrable par établissement : relevé, les 200 € passent sans seconde personne ; fonds social libéré.
    regler_seconde_validation(&x, 30_000, false).await;
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "fonds_social", "3"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        d.statut,
        StatutDemande::Appliquee,
        "fonds social libéré par le paramètre"
    );
    // Et abaissé à zéro : toute somme demande une seconde personne.
    regler_seconde_validation(&x, 0, true).await;
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "apel", "1"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(d.statut, StatutDemande::EnAttente);
}

#[tokio::test]
async fn f1_un_double_clic_ne_produit_qu_une_ecriture_et_une_cle_reutilisee_a_tort_est_un_conflit()
{
    use crate::voyages_argent_v3::StatutDemande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    let saisie = aide(x.p1, x.compte_c, "apel", "40");
    let compta = x.compta();
    let (a, b) = tokio::join!(
        x.c.ent
            .voyage_aide_accorder_a(&compta, x.s.id, saisie.clone(), instant_fixe()),
        x.c.ent
            .voyage_aide_accorder_a(&compta, x.s.id, saisie.clone(), instant_fixe()),
    );
    let (a, b) = (
        a.expect("premier clic"),
        b.expect("second clic : même résultat"),
    );
    assert_eq!(
        (a.statut, b.statut),
        (StatutDemande::Appliquee, StatutDemande::Appliquee)
    );
    assert_eq!(a.demande_id, b.demande_id);
    assert_eq!(
        a.aide.as_ref().map(|x| x.id),
        b.aide.as_ref().map(|x| x.id),
        "la même aide"
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_aide WHERE sortie_id = $1 AND sens = 'aide'",
    )
    .bind(x.s.id)
    .fetch_one(x.c.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "une seule écriture");
    assert_eq!(x.ligne(x.p1, x.compte_c).await.aides_centimes, 4_000);
    // Même clé, montant différent : conflit, rien n'est écrit.
    let mut autre = saisie.clone();
    autre.montant = "41".into();
    let e =
        x.c.ent
            .voyage_aide_accorder_a(&x.compta(), x.s.id, autre, instant_fixe())
            .await
            .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::Conflit)),
        "{e:?}"
    );
    // Report : même garantie.
    let r = report(x.p1, p2, x.compte_c, "30");
    let (a, b) = tokio::join!(
        x.c.ent
            .voyage_report_effectuer_a(&compta, x.s.id, r.clone(), instant_fixe()),
        x.c.ent
            .voyage_report_effectuer_a(&compta, x.s.id, r.clone(), instant_fixe()),
    );
    let (a, b) = (a.expect("premier clic"), b.expect("second clic"));
    assert_eq!(
        a.report.as_ref().map(|x| x.id),
        b.report.as_ref().map(|x| x.id)
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_report WHERE sortie_id = $1 AND sens = 'report'",
    )
    .bind(x.s.id)
    .fetch_one(x.c.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "un seul report");
    // L'annulation (compare-et-écrit) : un second clic est un conflit, jamais une seconde écriture inverse.
    let id = a.report.unwrap().id;
    x.c.ent
        .voyage_report_annuler_a(&x.compta(), x.s.id, id, "Report inutile", instant_fixe())
        .await
        .unwrap();
    let e =
        x.c.ent
            .voyage_report_annuler_a(&x.compta(), x.s.id, id, "Report inutile", instant_fixe())
            .await
            .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::Conflit)),
        "{e:?}"
    );
}

#[tokio::test]
async fn m1_annuler_un_versement_dont_l_argent_a_ete_reporte_demande_d_annuler_d_abord_le_report() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    let v = x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    let d =
        x.c.ent
            .voyage_report_effectuer_a(
                &x.compta(),
                x.s.id,
                report(x.p1, p2, x.compte_c, "100"),
                instant_fixe(),
            )
            .await
            .unwrap();
    let rid = d.report.expect("report").id;
    let e =
        x.c.ent
            .voyage_versement_annuler(&x.compta(), x.s.id, v.id, "Chèque rejeté")
            .await
            .unwrap_err();
    assert!(
        matches!(e, EntDbError::Refus(RefusMetier::Conflit, m) if m.contains("annule d'abord le report")),
        "{e:?}"
    );
    // Le report défait, le versement s'annule.
    x.c.ent
        .voyage_report_annuler_a(
            &x.compta(),
            x.s.id,
            rid,
            "Retour au premier élève",
            instant_fixe(),
        )
        .await
        .unwrap();
    x.c.ent
        .voyage_versement_annuler(&x.compta(), x.s.id, v.id, "Chèque rejeté")
        .await
        .expect("net suffisant");
}

#[tokio::test]
async fn m4_aide_report_et_pointage_refuses_sur_une_sortie_close_annulee_ou_refusee() {
    use crate::voyages::Commande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    for etat in ["annulee", "refusee", "close"] {
        // Retour à l'état ouvert (SQL), puis l'état visé : les trois états terminaux de la sortie.
        sqlx::query("UPDATE ent_sortie SET etat = 'ouverte' WHERE id = $1")
            .bind(x.s.id)
            .execute(x.c.ent.pool_verif())
            .await
            .unwrap();
        if etat == "annulee" {
            commande(&x, Commande::Annuler, None).await.unwrap();
        } else {
            sqlx::query("UPDATE ent_sortie SET etat = $2 WHERE id = $1")
                .bind(x.s.id)
                .bind(etat)
                .execute(x.c.ent.pool_verif())
                .await
                .unwrap();
        }
        let e =
            x.c.ent
                .voyage_aide_accorder_a(
                    &x.compta(),
                    x.s.id,
                    aide(x.p1, x.compte_c, "apel", "10"),
                    instant_fixe(),
                )
                .await
                .unwrap_err();
        assert!(
            matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)),
            "aide, {etat} : {e:?}"
        );
        let e =
            x.c.ent
                .voyage_report_effectuer_a(
                    &x.compta(),
                    x.s.id,
                    report(x.p1, p2, x.compte_c, "10"),
                    instant_fixe(),
                )
                .await
                .unwrap_err();
        assert!(
            matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)),
            "report, {etat} : {e:?}"
        );
        let e = x
            .verser(x.p1, x.compte_c, "cheque", "10")
            .await
            .unwrap_err();
        assert!(
            matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)),
            "pointage, {etat} : {e:?}"
        );
    }
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_sortie_argent_demande WHERE sortie_id = $1")
            .bind(x.s.id)
            .fetch_one(x.c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 0, "rien n'a été demandé");
}

#[tokio::test]
async fn f5_l_annulation_d_un_report_qui_remet_des_avoirs_a_revoir_trace_l_acteur() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    let d =
        x.c.ent
            .voyage_report_effectuer_a(
                &x.compta(),
                x.s.id,
                report(x.p1, p2, x.compte_c, "100"),
                instant_fixe(),
            )
            .await
            .unwrap();
    let rid = d.report.expect("report").id;
    // Le receveur se désiste : un avoir est proposé sur l'argent reçu.
    x.c.ent
        .voyage_participant_desister(
            &x.c.a(x.c.prof, Role::Prof),
            x.s.id,
            p2,
            "decision_personnelle",
            None,
        )
        .await
        .unwrap();
    assert!(
        !x.avoirs_de(p2, x.compte_c).await.is_empty(),
        "avoir proposé au receveur"
    );
    x.c.ent
        .voyage_report_annuler_a(
            &x.dir(),
            x.s.id,
            rid,
            "Le report était une erreur",
            instant_fixe(),
        )
        .await
        .unwrap();
    let (acteur, apres): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT acteur_user_id, apres FROM ent_sortie_journal WHERE sortie_id = $1 AND action = 'avoirs_a_revoir'",
    )
    .bind(x.s.id)
    .fetch_one(x.c.ent.pool())
    .await
    .unwrap();
    assert_eq!(acteur, x.c.dir, "l'acteur est tracé");
    let apres = apres.unwrap();
    assert!(apres.contains("avoir"), "{apres}");
    assert!(
        !apres.contains('€'),
        "aucun montant dans le journal de l'organisateur"
    );
}

// ═════════════════════════════ Base ═════════════════════════════

/// Rejoue une instruction que PostgreSQL a choisie pour casser un interblocage (`40P01`) : `TRUNCATE … CASCADE` verrouille
/// toutes les tables liées pendant que d'autres tests de la même base y écrivent. Le refus attendu est celui du déclencheur.
async fn sans_interblocage(
    pool: &sqlx::PgPool,
    sql: &str,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    let mut r = sqlx::query(sql).execute(pool).await;
    for _ in 0..40 {
        if code_sql(&r).as_deref() != Some("40P01") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        r = sqlx::query(sql).execute(pool).await;
    }
    r
}

#[tokio::test]
async fn ajout_seul_et_gardes_de_la_base_pour_les_aides_et_les_reports() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    let a =
        x.c.ent
            .voyage_aide_accorder_t(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "20"))
            .await
            .unwrap();
    x.c.ent
        .voyage_report_effectuer_t(&x.compta(), x.s.id, report(x.p1, p2, x.compte_c, "30"))
        .await
        .unwrap();
    x.c.ent
        .voyage_participant_desister(&x.c.a(x.c.prof, Role::Prof), x.s.id, x.p1, "sante", None)
        .await
        .ok();
    let pool = x.c.ent.pool_verif();
    for t in [
        "ent_sortie_aide",
        "ent_sortie_report",
        "ent_sortie_internat_retrait",
    ] {
        if t == "ent_sortie_internat_retrait" {
            interne_parti(&x, x.c.eleves[0], x.p0).await;
            x.c.ent
                .voyage_internat_synchroniser(&x.dir(), x.s.id)
                .await
                .unwrap();
        }
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {t}"))
            .fetch_one(pool)
            .await
            .unwrap();
        assert!(n > 0, "{t} doit contenir une ligne");
        let r = sqlx::query(&format!(
            "UPDATE {t} SET etablissement_id = etablissement_id"
        ))
        .execute(pool)
        .await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "UPDATE {t}");
        let r = sqlx::query(&format!("DELETE FROM {t}")).execute(pool).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "DELETE {t}");
        let r = sans_interblocage(pool, &format!("TRUNCATE {t} CASCADE")).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "TRUNCATE {t}");
    }
    // Une annulation d'aide qui ne reprend pas exactement l'aide est refusée par la base.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, annule_id, motif, decide_par_user_id, decide_le)
         VALUES ($1,$2,$3,$4,$5,'annulation','apel',1999,$6,'x',$7, now())",
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
    let demande_report: Uuid = sqlx::query_scalar(
        "SELECT demande_id FROM ent_sortie_report WHERE demande_id IS NOT NULL LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    // Un report vers un participant qui n'a pas ce payeur est refusé par la base, comme un montant hors bornes.
    for (vers, montant, attendu) in [
        (x.p0, 10_i64, "23514"),
        (p2, 0, "23514"),
        (p2, 10_000_001, "23514"),
    ] {
        let r = sqlx::query(
            "INSERT INTO ent_sortie_report (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, motif, decide_par_user_id, demande_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,$6,'report',$7,'x',$8,$9, now())",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(x.p1)
        .bind(vers)
        .bind(x.compte_c)
        .bind(montant)
        .bind(x.c.compta)
        .bind(demande_report)
        .execute(pool)
        .await;
        assert_eq!(
            code_sql(&r).as_deref(),
            Some(attendu),
            "vers {vers} montant {montant}"
        );
    }
    // Une aide ne vise qu'une part de la sortie : un payeur sans part est refusé.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, motif, demande_id, decide_par_user_id, decide_le)
         VALUES ($1,$2,$3,$4,$5,'aide','apel',100,'x',$7,$6, now())",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p1)
    .bind(x.compte_a)
    .bind(x.c.compta)
    .bind(demande_report)
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
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let aide_id =
        x.c.ent
            .voyage_aide_accorder_t(&x.compta(), x.s.id, aide(x.p1, x.compte_c, "apel", "10"))
            .await
            .unwrap()
            .id;
    let Some(autre) = crate::tests_voyages_v0d::cadre().await else {
        return;
    };
    let a = autre.a(autre.compta, Role::Compta);
    assert!(est_introuvable(
        &autre.ent.voyage_aides(&a, x.s.id).await.unwrap_err()
    ));
    assert!(est_introuvable(
        &autre.ent.voyage_reports(&a, x.s.id).await.unwrap_err()
    ));
    assert!(est_introuvable(
        &autre
            .ent
            .voyage_aide_annuler(&a, x.s.id, aide_id, "x")
            .await
            .unwrap_err()
    ));
    assert!(est_introuvable(
        &autre
            .ent
            .voyage_aide_accorder_t(&a, x.s.id, aide(x.p1, x.compte_c, "apel", "1"))
            .await
            .unwrap_err()
    ));
    assert!(est_introuvable(
        &autre
            .ent
            .voyage_report_effectuer_t(&a, x.s.id, report(x.p1, x.p0, x.compte_c, "1"))
            .await
            .unwrap_err()
    ));
    let e = autre
        .ent
        .voyage_internat_synchroniser(&autre.a(autre.dir, Role::Direction), x.s.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

// ═════════════════════════════ Suites de la contre-revue de V3 (M-1 à M-4, F-a à F-j) ═════════════════════════════

/// Un compte « compta » payeur d'un compte donné (sans lien avec l'élève aidé lui-même).
async fn compta_payeur_de(x: &Suivi, compte: Uuid) -> Acteur<'static> {
    let foyer_c: Uuid = sqlx::query_scalar("SELECT foyer_id FROM ent_fin_compte WHERE id = $1")
        .bind(compte)
        .fetch_one(x.c.ent.pool_verif())
        .await
        .unwrap();
    let u = crate::tests_voyages_v0a::compte(&x.c.ent, x.c.etab, "compta").await;
    payeur(&x.c, x.c.eleves[2], u, foyer_c, 0).await;
    x.c.a(u, Role::Compta)
}

/// Un troisième compte « compta » libre de tout lien.
async fn compta_libre(x: &Suivi) -> Acteur<'static> {
    let u = crate::tests_voyages_v0a::compte(&x.c.ent, x.c.etab, "compta").await;
    x.c.a(u, Role::Compta)
}

#[tokio::test]
async fn m1_un_depart_ecrit_sans_resynchronisation_laisse_l_eleve_attendu_puis_le_retire_et_le_cpe_peut_annuler(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    // La sortie est « en cours » : l'ancienne branche directe retirait déjà l'élève, sans aucune ligne de retrait.
    sqlx::query("UPDATE ent_sortie SET etat = 'en_cours' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    assert!(retraits_actifs(&x, e0).await.is_empty());
    for n in [0, 1] {
        assert_eq!(
            statut_nuit(&x, e0, jours(&x, n)).await.as_deref(),
            Some("attendu"),
            "M-1 : « parti » écrit directement, sans resynchronisation : l'élève reste attendu (nuit {n})"
        );
    }
    // Après `internat_resynchroniser` : retiré, avec une ligne de retrait que le CPE peut annuler.
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
            .await
            .unwrap();
    assert_eq!((r.poses, r.annules), (1, 0));
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("retire")
    );
    let retrait: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_internat_retrait WHERE eleve_id = $1 AND annule_le IS NULL",
    )
    .bind(e0)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    x.c.ent
        .annuler_retrait_a(x.c.etab, x.c.dir, "direction", retrait, instant_fixe())
        .await
        .expect("le CPE peut annuler le retrait");
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("attendu")
    );
}

#[tokio::test]
async fn m2_le_seuil_de_seconde_validation_porte_sur_le_cumul_pas_sur_chaque_demande() {
    use crate::voyages_argent_v3::StatutDemande;
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let d1 = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "149,99"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        d1.statut,
        StatutDemande::Appliquee,
        "149,99 € : sous le seuil"
    );
    // Fractionnement : même personne, même participant, même payeur, source étiquetée « autre ».
    let d2 = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "autre", "149,99"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        (d2.statut, d2.aide.is_none()),
        (StatutDemande::EnAttente, true),
        "299,98 € cumulés : une seconde personne"
    );
    // Même une toute petite demande suivante attend : le cumul (appliqué + en attente) dépasse déjà le seuil.
    let d3 = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "0,01"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(d3.statut, StatutDemande::EnAttente);
    // Un autre participant (et un autre payeur) a son propre cumul.
    let d4 = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "apel", "149,99"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(d4.statut, StatutDemande::Appliquee);
    assert_eq!(
        x.ligne(x.p1, x.compte_c).await.aides_centimes,
        14_999,
        "seule l'aide appliquée compte comme aide"
    );
    // La seconde personne valide la demande fractionnée : l'aide s'écrit alors.
    let v = ent
        .voyage_demande_valider_a(&x.dir(), x.s.id, d2.demande_id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(v.statut, StatutDemande::Appliquee);
    assert_eq!(x.ligne(x.p1, x.compte_c).await.aides_centimes, 29_998);
    // Le même raisonnement vaut pour les reports (cumul par cédant et payeur).
    let p2 = x.p2_meme_payeur().await;
    x.verser(x.p1, x.compte_c, "cheque", "200").await.unwrap();
    let r1 = ent
        .voyage_report_effectuer_a(
            &x.compta(),
            x.s.id,
            report(x.p1, p2, x.compte_c, "100"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(r1.statut, StatutDemande::Appliquee);
    let r2 = ent
        .voyage_report_effectuer_a(
            &x.compta(),
            x.s.id,
            report(x.p1, p2, x.compte_c, "100"),
            instant_fixe(),
        )
        .await
        .unwrap();
    assert_eq!(
        r2.statut,
        StatutDemande::EnAttente,
        "200 € de reports cumulés > 150 €"
    );
}

#[tokio::test]
async fn m3_la_regle_3_couvre_plans_versements_frais_avoirs_et_acomptes_parent_et_payeur_refuses() {
    use crate::voyages::Commande;
    use crate::voyages_argent_flux::AcompteSaisie;
    use crate::voyages_argent_repo::{FraisSaisie, VersementSaisie};
    use crate::voyages_prestataires_repo::PrestataireSaisie;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let parent = compta_parent_de(&x, x.c.eleves[1]).await;
    let payeur_c = compta_payeur_de(&x, x.compte_c).await;
    let versement = |p: Uuid, c: Uuid| VersementSaisie {
        participant_id: p,
        compte_id: c,
        moyen: "cheque".into(),
        montant: "10".into(),
        date_valeur: None,
        reference: None,
    };
    let frais = |p: Uuid| FraisSaisie {
        participant_id: p,
        nature: "autre".into(),
        libelle: "Billet".into(),
        montant: "10".into(),
    };
    // Plan et pointage (et frais réel) : le parent de l'élève et le payeur du compte sont refusés.
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_plan_accorder(acteur, x.s.id, x.p1, x.compte_c, 2, None)
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "plan, {qui} : {e:?}");
        let e = ent
            .voyage_versement_pointer(acteur, x.s.id, versement(x.p1, x.compte_c))
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "pointage, {qui} : {e:?}");
        let e = ent
            .voyage_frais_saisir(acteur, x.s.id, frais(x.p1))
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "frais réel, {qui} : {e:?}");
    }
    // Une autre personne les accomplit : la règle ne bloque que les personnes liées.
    ent.voyage_plan_accorder(&x.compta(), x.s.id, x.p1, x.compte_c, 2, None)
        .await
        .expect("plan par la comptabilité");
    let v = x
        .verser(x.p1, x.compte_c, "cheque", "50")
        .await
        .expect("pointage par la comptabilité");
    // Annulation d'un versement.
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_versement_annuler(acteur, x.s.id, v.id, "Erreur de saisie")
            .await
            .unwrap_err();
        assert!(
            est_interdit_metier(&e),
            "annulation de versement, {qui} : {e:?}"
        );
    }
    // Désistement : frais réel, puis avoir proposé.
    ent.voyage_participant_desister(&x.porteur(), x.s.id, x.p1, "famille", None)
        .await
        .unwrap();
    let f = ent
        .voyage_frais_saisir(&x.compta(), x.s.id, frais(x.p1))
        .await
        .expect("frais par la comptabilité");
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_frais_annuler(acteur, x.s.id, f.id, "Erreur de saisie")
            .await
            .unwrap_err();
        assert!(
            est_interdit_metier(&e),
            "annulation de frais, {qui} : {e:?}"
        );
    }
    let avoir = x
        .avoirs_de(x.p1, x.compte_c)
        .await
        .into_iter()
        .find(|a| a.statut == "propose")
        .expect("avoir proposé");
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_avoir_decider(
                acteur,
                x.s.id,
                avoir.id,
                true,
                None,
                None,
                "Décision de la comptabilité",
            )
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "décision d'avoir, {qui} : {e:?}");
    }
    // Acomptes fournisseurs : la sortie où l'enfant part (parent) ou dont le compte paie (payeur).
    let presta = ent
        .prestataire_creer(
            &x.sec(),
            PrestataireSaisie {
                raison_sociale: format!("Cars M3 {}", Uuid::new_v4().simple()),
                siren: None,
                telephone: None,
                categories: vec!["transporteur".into()],
                adresses: vec![format!("m3-{}@exemple-test.fr", Uuid::new_v4().simple())],
                version: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    let jour = x.c.jour_etab().await;
    let acompte = || AcompteSaisie {
        prestataire_id: presta.id,
        commande_id: None,
        libelle: "Acompte".into(),
        montant: "100".into(),
        echeance: jour + chrono::Duration::days(10),
    };
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_acompte_creer(acteur, x.s.id, acompte())
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "acompte, création, {qui} : {e:?}");
    }
    let ac = ent
        .voyage_acompte_creer(&x.compta(), x.s.id, acompte())
        .await
        .expect("acompte par la comptabilité");
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_acompte_payer(acteur, x.s.id, ac.id, None, None, None)
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "acompte, paiement, {qui} : {e:?}");
        let e = ent
            .voyage_acompte_annuler(acteur, x.s.id, ac.id, "Erreur")
            .await
            .unwrap_err();
        assert!(
            est_interdit_metier(&e),
            "acompte, annulation, {qui} : {e:?}"
        );
    }
    // Annulation collective : sortie annulée ; le parent d'un participant qui a versé et le payeur de son compte sont refusés.
    commande_a(&x, Commande::Annuler, None, instant_fixe())
        .await
        .unwrap();
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_avoirs_annulation_collective(acteur, x.s.id)
            .await
            .unwrap_err();
        assert!(
            est_interdit_metier(&e),
            "annulation collective, {qui} : {e:?}"
        );
    }
    // F-c : chacun de ces refus a laissé une trace (qui, quelle action, quelle sortie), sans nom ni montant.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_argent_refus WHERE sortie_id = $1 AND user_id = $2",
    )
    .bind(x.s.id)
    .bind(parent.user_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        n, 10,
        "un refus tracé par pouvoir refusé au parent (dix pouvoirs)"
    );
    // La base refuse aussi le contournement direct (déclencheurs 655) : une écriture au nom du parent est rejetée.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_plan (id, etablissement_id, sortie_id, participant_id, compte_id, rang, nb_echeances, accorde_par_user_id)
         VALUES ($1,$2,$3,$4,$5,99,2,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p1)
    .bind(x.compte_c)
    .bind(parent.user_id)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "plan : refus de la base"
    );
    let r = sqlx::query(
        "INSERT INTO ent_sortie_versement (id, etablissement_id, sortie_id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, saisi_par_user_id)
         VALUES ($1,$2,$3,$4,$5,'versement','cheque',100,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p1)
    .bind(x.compte_c)
    .bind(jour)
    .bind(payeur_c.user_id)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "versement : refus de la base (payeur)"
    );
    let r = sqlx::query(
        "INSERT INTO ent_sortie_acompte_fournisseur (id, etablissement_id, sortie_id, prestataire_id, libelle, montant_centimes, echeance, cree_par_user_id)
         VALUES ($1,$2,$3,$4,'x',100,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(presta.id)
    .bind(jour)
    .bind(parent.user_id)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "acompte : refus de la base"
    );
}

#[tokio::test]
async fn m4_la_date_d_annulation_ne_se_lit_plus_dans_modifie_le() {
    use crate::voyages::Commande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let e0 = x.c.eleves[0];
    interne_parti(&x, e0, x.p0).await;
    let t = instant_fixe();
    sqlx::query("UPDATE ent_sortie SET debut = $2 - interval '3 days', fin = $2 + interval '2 days' WHERE id = $1")
        .bind(x.s.id)
        .bind(t)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, t)
        .await
        .unwrap();
    commande_a(&x, Commande::Annuler, None, t).await.unwrap();
    let avant = retraits_actifs(&x, e0).await;
    assert_eq!(avant.len(), 1, "{avant:?}");
    let annulee_le: DateTime<Utc> =
        sqlx::query_scalar("SELECT annulee_le FROM ent_sortie WHERE id = $1")
            .bind(x.s.id)
            .fetch_one(x.c.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(
        annulee_le, t,
        "l'instant d'annulation est celui de la commande"
    );
    // `modifie_le` est déplacé APRÈS l'annulation (un futur UPDATE de la sortie, par exemple) : rien ne change.
    sqlx::query("UPDATE ent_sortie SET modifie_le = modifie_le + interval '2 days' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    let r =
        x.c.ent
            .voyage_internat_synchroniser_a(&x.dir(), x.s.id, t)
            .await
            .unwrap();
    assert_eq!(
        (r.poses, r.annules, r.inchanges),
        (0, 0, 1),
        "les nuits retirées ne bougent pas"
    );
    assert_eq!(retraits_actifs(&x, e0).await, avant);
    // L'instant d'annulation est immuable : la base le garde.
    sqlx::query("UPDATE ent_sortie SET annulee_le = annulee_le + interval '5 days' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    let encore: DateTime<Utc> =
        sqlx::query_scalar("SELECT annulee_le FROM ent_sortie WHERE id = $1")
            .bind(x.s.id)
            .fetch_one(x.c.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(encore, t);
    // Et la colonne n'existe que pour les sorties annulées.
    let autre: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie WHERE etat <> 'annulee' AND annulee_le IS NOT NULL",
    )
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(autre, 0);
}

#[tokio::test]
async fn fa_personnel_puis_droit_puis_existence_aucune_route_ne_revele_ni_ne_verrouille_avant() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let inconnue = Uuid::new_v4();
    // Non-personnel (parent, élève) : « introuvable », sortie existante ou non.
    for acteur in [x.parent(x.parent_a), x.c.a(x.c.prof2, Role::Eleve)] {
        for id in [x.s.id, inconnue] {
            let e = ent
                .voyage_aide_accorder_a(
                    &acteur,
                    id,
                    aide(x.p1, x.compte_c, "apel", "10"),
                    instant_fixe(),
                )
                .await
                .unwrap_err();
            assert!(est_introuvable(&e), "{e:?}");
            let e = ent
                .voyage_internat_synchroniser_a(&acteur, id, instant_fixe())
                .await
                .unwrap_err();
            assert!(est_introuvable(&e), "{e:?}");
            let e = ent
                .voyage_participant_retour_anticipe_a(
                    &acteur,
                    id,
                    x.p0,
                    "sante",
                    None,
                    instant_fixe(),
                )
                .await
                .unwrap_err();
            assert!(est_introuvable(&e), "{e:?}");
        }
    }
    // Personnel sans le droit d'argent : « interdit » AVANT de regarder si la sortie existe (aucune fuite d'existence).
    let prof = x.c.a(x.c.prof2, Role::Prof);
    for id in [x.s.id, inconnue] {
        let e = ent
            .voyage_aide_accorder_a(
                &prof,
                id,
                aide(x.p1, x.compte_c, "apel", "10"),
                instant_fixe(),
            )
            .await
            .unwrap_err();
        assert!(est_forbidden(&e), "{e:?}");
        let e = ent
            .voyage_demande_valider_a(&prof, id, Uuid::new_v4(), instant_fixe())
            .await
            .unwrap_err();
        assert!(est_forbidden(&e), "{e:?}");
        let e = ent
            .voyage_demande_refuser_a(&prof, id, Uuid::new_v4(), "Non", instant_fixe())
            .await
            .unwrap_err();
        assert!(est_forbidden(&e), "{e:?}");
    }
    // Le droit suffit pour arriver à l'existence : sortie inconnue → « introuvable ».
    let e = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            inconnue,
            aide(x.p1, x.compte_c, "apel", "10"),
            instant_fixe(),
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let e = ent
        .voyage_internat_synchroniser_a(&x.dir(), inconnue, instant_fixe())
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

#[tokio::test]
async fn fb_retour_anticipe_borne_trace_et_refuse_sur_une_sortie_terminale() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let (e0, e1) = (x.c.eleves[0], x.c.eleves[1]);
    interne_parti(&x, e0, x.p0).await;
    interne_parti(&x, e1, x.p1).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    let midi = |n| jours(&x, n).and_hms_opt(12, 0, 0).unwrap().and_utc();
    // Saisie le jour J+3 pour un retour effectif le jour J+1 : la date de retour est bornée (≤ maintenant, ≥ départ).
    let e =
        x.c.ent
            .voyage_participant_retour_anticipe_a(
                &x.porteur(),
                x.s.id,
                x.p0,
                "sante",
                Some(midi(4)),
                midi(3),
            )
            .await
            .unwrap_err();
    assert!(est_refus(&e), "un retour dans le futur est refusé : {e:?}");
    let e =
        x.c.ent
            .voyage_participant_retour_anticipe_a(
                &x.porteur(),
                x.s.id,
                x.p0,
                "sante",
                Some(midi(-30)),
                midi(3),
            )
            .await
            .unwrap_err();
    assert!(
        est_refus(&e),
        "un retour avant le départ est refusé : {e:?}"
    );
    x.c.ent
        .voyage_participant_retour_anticipe_a(
            &x.porteur(),
            x.s.id,
            x.p0,
            "sante",
            Some(midi(1)),
            midi(3),
        )
        .await
        .expect("saisie tardive");
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 0)).await.as_deref(),
        Some("retire"),
        "la nuit passée dehors reste retirée"
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 1)).await.as_deref(),
        Some("attendu"),
        "l'élève est rentré le jour J+1 : pas retiré jusqu'à l'instant de la saisie"
    );
    let (a, trace): (DateTime<Utc>, Option<String>) = sqlx::query_as(
        "SELECT a, motif_texte FROM ent_sortie_participant_fait WHERE participant_id = $1 AND etat_apres = 'rentre'",
    )
    .bind(x.p0)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(a, midi(1));
    assert!(
        trace.unwrap_or_default().contains("saisi le"),
        "l'instant de la saisie reste tracé"
    );
    // Une sortie rentrée, close ou annulée n'accepte plus de retour anticipé.
    for etat in ["rentree", "close", "annulee"] {
        sqlx::query("UPDATE ent_sortie SET etat = $2 WHERE id = $1")
            .bind(x.s.id)
            .bind(etat)
            .execute(x.c.ent.pool_verif())
            .await
            .unwrap();
        let e =
            x.c.ent
                .voyage_participant_retour_anticipe_a(
                    &x.porteur(),
                    x.s.id,
                    x.p1,
                    "sante",
                    None,
                    midi(3),
                )
                .await
                .unwrap_err();
        assert!(est_refus(&e), "{etat} : {e:?}");
    }
}

#[tokio::test]
async fn fc_demande_expiree_annulation_soumise_au_seuil_et_refus_trace() {
    use crate::voyages_argent_v3::StatutDemande;
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_fixe();
    // Expiration : une demande en attente depuis plus de 30 jours ne se valide plus (on la refuse ou on la retire).
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "apel", "200"),
            t0,
        )
        .await
        .unwrap();
    assert_eq!(d.statut, StatutDemande::EnAttente);
    let e = ent
        .voyage_demande_valider_a(
            &x.dir(),
            x.s.id,
            d.demande_id,
            t0 + chrono::Duration::days(31),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)),
        "{e:?}"
    );
    let v = ent.voyage_demandes(&x.compta(), x.s.id).await.unwrap();
    assert_eq!(
        v.iter().find(|y| y.id == d.demande_id).unwrap().statut,
        "en_attente",
        "la liste lit l'horloge réelle : la demande de 2031 n'est pas encore expirée"
    );
    ent.voyage_demande_refuser_a(
        &x.dir(),
        x.s.id,
        d.demande_id,
        "Expirée",
        t0 + chrono::Duration::days(31),
    )
    .await
    .expect("on peut encore la refuser");
    // La même demande, validée à temps par une autre personne, s'applique ; son annulation (200 € > seuil) ne se fait pas seul.
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "apel", "200"),
            t0,
        )
        .await
        .unwrap();
    let v = ent
        .voyage_demande_valider_a(
            &x.dir(),
            x.s.id,
            d.demande_id,
            t0 + chrono::Duration::days(29),
        )
        .await
        .expect("à temps");
    let aide_id = v.aide.unwrap().id;
    for (qui, acteur) in [
        ("le demandeur", x.compta()),
        ("celui qui a décidé", x.dir()),
    ] {
        let e = ent
            .voyage_aide_annuler_a(&acteur, x.s.id, aide_id, "Erreur", t0)
            .await
            .unwrap_err();
        assert!(
            est_interdit_metier(&e),
            "{qui} n'annule pas seul une aide au-dessus du seuil : {e:?}"
        );
    }
    ent.voyage_aide_annuler_a(
        &compta_libre(&x).await,
        x.s.id,
        aide_id,
        "Erreur de saisie",
        t0,
    )
    .await
    .expect("une troisième personne annule");
    // Sous le seuil : l'annulation reste simple.
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p0, x.compte_a, "apel", "20"),
            t0,
        )
        .await
        .unwrap();
    ent.voyage_aide_annuler_a(
        &x.compta(),
        x.s.id,
        d.aide.unwrap().id,
        "Erreur de saisie",
        t0,
    )
    .await
    .expect("sous le seuil");
    // Refus tracé : le parent refusé laisse une ligne (action, sortie, personne), sans montant.
    let parent = compta_parent_de(&x, x.c.eleves[1]).await;
    let e = ent
        .voyage_aide_accorder_a(&parent, x.s.id, aide(x.p1, x.compte_c, "apel", "10"), t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM ent_sortie_argent_refus WHERE sortie_id = $1 AND user_id = $2",
    )
    .bind(x.s.id)
    .bind(parent.user_id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(actions, vec!["voyage_aide_accorder_a".to_string()]);
    // Ajout seul : la trace ne se modifie ni ne s'efface.
    let r = sqlx::query("DELETE FROM ent_sortie_argent_refus WHERE sortie_id = $1")
        .bind(x.s.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn fd_la_base_garde_la_coherence_des_aides_reports_demandes_et_fins_de_retrait() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    // Les contraintes de cohérence existent.
    for (table, nom) in [
        (
            "ent_sortie_internat_retrait_fin",
            "ent_sortie_internat_retrait_fin_lien_fk",
        ),
        (
            "ent_sortie_argent_demande",
            "ent_sortie_argent_demande_participant_sortie_fk",
        ),
        (
            "ent_sortie_argent_demande",
            "ent_sortie_argent_demande_vers_sortie_fk",
        ),
        (
            "ent_sortie_participant",
            "ent_sortie_participant_etab_sortie_id_uq",
        ),
    ] {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_constraint WHERE conrelid = $1::regclass AND conname = $2",
        )
        .bind(table)
        .bind(nom)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(n, 1, "{nom}");
    }
    // Une fin de retrait vise un retrait de séjour posé.
    let r = sqlx::query(
        "INSERT INTO ent_sortie_internat_retrait_fin (id, etablissement_id, sortie_id, retrait_id, cause, par_user_id, le)
         VALUES ($1,$2,$3,$4,'dates',$5, now())",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(Uuid::new_v4())
    .bind(x.c.dir)
    .execute(ent.pool_verif())
    .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23503"));
    // Une aide reprend exactement sa demande : un autre montant, un autre participant ou un autre payeur est rejeté.
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "10"),
            instant_fixe(),
        )
        .await
        .unwrap();
    let demande_id = d.demande_id;
    let (autre_p, autre_c) = (x.p0, x.compte_a);
    for (participant, compte, montant) in [(x.p1, x.compte_c, 999_i64), (autre_p, autre_c, 1000)] {
        let r = sqlx::query(
            "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, motif, demande_id, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,'aide','apel',$6,'x',$7,$8, now())",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(participant)
        .bind(compte)
        .bind(montant)
        .bind(demande_id)
        .bind(x.c.dir)
        .execute(ent.pool_verif())
        .await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "{montant}");
    }
}

#[tokio::test]
async fn ff_une_sortie_rentree_ne_garde_pas_un_participant_reste_parti_retire() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let (e0, e1) = (x.c.eleves[0], x.c.eleves[1]);
    interne_parti(&x, e0, x.p0).await;
    interne_parti(&x, e1, x.p1).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    assert_eq!(retraits_actifs(&x, e0).await.len(), 1);
    // p1 est rentré avec le groupe (fait daté) ; p0 est resté « parti » dans les données : incohérence.
    sqlx::query("UPDATE ent_sortie_participant SET etat = 'rentre' WHERE id = $1")
        .bind(x.p1)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_sortie_participant_fait (id, etablissement_id, participant_id, a, type, etat_avant, etat_apres)
         VALUES ($1,$2,$3,$4,'etat','parti','rentre')",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.p1)
    .bind(jours(&x, 1).and_hms_opt(12, 0, 0).unwrap().and_utc())
    .execute(x.c.ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("UPDATE ent_sortie SET etat = 'rentree' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    x.c.ent
        .voyage_internat_synchroniser_a(&x.dir(), x.s.id, instant_fixe())
        .await
        .unwrap();
    assert!(
        retraits_actifs(&x, e0).await.is_empty(),
        "resté « parti » sur une sortie rentrée : plus retiré"
    );
    assert_eq!(
        statut_nuit(&x, e0, jours(&x, 1)).await.as_deref(),
        Some("attendu")
    );
    let r1 = retraits_actifs(&x, e1).await;
    assert_eq!(
        r1.len(),
        1,
        "le rentré garde ses nuits passées dehors : {r1:?}"
    );
    assert_eq!(r1[0].1, jours(&x, 0), "jusqu'à la veille de son retour");
    assert!(causes_de_fin(&x).await.contains(&"sortie".to_string()));
}

#[tokio::test]
async fn fi_la_fonction_d_attendu_de_655_ne_differe_de_celle_de_2829_que_par_la_condition_commentee_655_et_les_public(
) {
    // Le corps de la fonction recréée en 655 est celui de 2829 (dernière définition avant elle), aux préfixes `public.` près
    // et hors le seul bloc commenté « 655 » : une divergence future est détectée ici, pas écrasée en silence.
    let m655 = include_str!("../../../migrations/655_sortie_voyages_v3_suites.sql");
    let m2829 = include_str!("../../../migrations/2829_internat_i4_vie_quotidienne.sql");
    let corps = |s: &str| -> String {
        let debut = s
            .find("CREATE OR REPLACE FUNCTION ent_internat_attendu_calcule")
            .expect("définition");
        let fin = s[debut..].find("\n$$;").expect("fin") + debut;
        s[debut..fin].to_string()
    };
    let normaliser = |s: String| -> Vec<String> {
        s.replace("public.", "")
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with("--"))
            .collect()
    };
    let (a, b) = (normaliser(corps(m2829)), normaliser(corps(m655)));
    // Les lignes ajoutées par 655 sont exactement le bloc « EXISTS … retrait_effectif » de M-1.
    let ajoutees: Vec<&String> = b.iter().filter(|l| !a.contains(l)).collect();
    let manquantes: Vec<&String> = a.iter().filter(|l| !b.contains(l)).collect();
    assert!(
        manquantes.is_empty(),
        "lignes de 2829 absentes de 655 : {manquantes:?}"
    );
    assert_eq!(
        ajoutees.len(),
        4,
        "seul le bloc M-1 (4 lignes de code) est ajouté : {ajoutees:?}"
    );
    assert!(ajoutees
        .iter()
        .any(|l| l.contains("ent_sortie_retrait_effectif")));
    // Même contrôle pour 654, dont les deux conditions « 654 » sont déjà dans 2829.
    let m654 = include_str!("../../../migrations/654_sortie_internat_lien.sql");
    let c654 = normaliser(corps(m654));
    for l in c654.iter().filter(|l| l.contains("sejour_scolaire")) {
        assert!(a.contains(l), "{l}");
    }
}

/// Contre-revue du pilote (Moyen 1, D-069) : recevoir un bordereau est un pouvoir sur l'argent. La personne liée à un
/// élève du bordereau, ou payeuse d'un de ses comptes, ne le reçoit pas ; une personne libre le reçoit.
#[tokio::test]
async fn m3_la_remise_d_un_bordereau_est_refusee_au_parent_et_au_payeur() {
    use crate::voyages_argent_flux::{BordereauLigneSaisie, BordereauSaisie};
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let parent = compta_parent_de(&x, x.c.eleves[1]).await;
    let payeur_c = compta_payeur_de(&x, x.compte_c).await;
    let bv = ent
        .voyage_bordereau_saisir(
            &x.porteur(),
            x.s.id,
            BordereauSaisie {
                nb_cheques: 1,
                total: "100".into(),
                lignes: vec![BordereauLigneSaisie { participant_id: x.p1, numero_cheque: "901".into(), montant: "100".into() }],
            },
        )
        .await
        .expect("bordereau");
    let l = bv.lignes[0].id;
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent.voyage_bordereau_remettre(acteur, x.s.id, bv.id, vec![l], None).await.unwrap_err();
        assert!(est_interdit_metier(&e), "remise, {qui} : {e:?}");
    }
    let r = ent
        .voyage_bordereau_remettre(&compta_libre(&x).await, x.s.id, bv.id, vec![l], None)
        .await
        .expect("remise par une personne libre");
    assert!(r.remise.is_some());
}
