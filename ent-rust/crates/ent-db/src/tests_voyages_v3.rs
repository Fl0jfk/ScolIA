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

/// V3b, Moyen 2 : un participant resté « parti » sur une sortie rentrée ne perd son retrait qu'à partir du jour de
/// passage en « rentrée » (`rentree_le`) ; les nuits déjà passées dehors restent retirées, les nuits à venir qu'il ne
/// passera pas dehors le laissent attendu.
#[tokio::test]
async fn ff_une_sortie_rentree_garde_les_nuits_passees_dehors_d_un_participant_reste_parti() {
    let Some(x) = suivi_v3().await else { return };
    // Séjour de quatre nuits (jours 0 à 3) : le cadre en pose deux.
    sqlx::query("UPDATE ent_sortie SET fin = fin + interval '2 days' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    x.sync().await;
    let (e0, e1) = (x.c.eleves[0], x.c.eleves[1]);
    interne_parti(&x, e0, x.p0).await;
    interne_parti(&x, e1, x.p1).await;
    x.c.ent
        .voyage_internat_synchroniser_a(&x.porteur(), x.s.id, instant_fixe())
        .await
        .unwrap();
    let avant = retraits_actifs(&x, e0).await;
    assert_eq!(
        (avant[0].0, avant[0].1),
        (jours(&x, 0), jours(&x, 3)),
        "quatre nuits retirées au départ"
    );
    // p1 est rentré avec le groupe le jour 1 à midi (fait daté) ; p0 est resté « parti » dans les données.
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
    // La sortie passe en « rentrée » le jour 2 à 14 h : instant explicite, posé tel quel à la transition.
    let rentree_le = jours(&x, 2).and_hms_opt(14, 0, 0).unwrap().and_utc();
    sqlx::query("UPDATE ent_sortie SET etat = 'rentree', rentree_le = $2 WHERE id = $1")
        .bind(x.s.id)
        .bind(rentree_le)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    x.c.ent
        .voyage_internat_synchroniser_a(&x.dir(), x.s.id, instant_fixe())
        .await
        .unwrap();
    // p0 : nuits 0 et 1 (passées dehors) restent retirées ; nuits 2 et 3 (à venir, jamais passées dehors) : attendu.
    let r0 = retraits_actifs(&x, e0).await;
    assert_eq!(r0.len(), 1, "{r0:?}");
    assert_eq!(
        (r0[0].0, r0[0].1),
        (jours(&x, 0), jours(&x, 1)),
        "jusqu'à la veille de la rentrée"
    );
    for (n, attendu) in [(0, "retire"), (1, "retire"), (2, "attendu"), (3, "attendu")] {
        assert_eq!(
            statut_nuit(&x, e0, jours(&x, n)).await.as_deref(),
            Some(attendu),
            "nuit {n} du participant resté parti"
        );
    }
    // p1 : rentré le jour 1 : nuit 0 retirée seulement.
    let r1 = retraits_actifs(&x, e1).await;
    assert_eq!(
        r1.len(),
        1,
        "le rentré garde ses nuits passées dehors : {r1:?}"
    );
    assert_eq!(r1[0].1, jours(&x, 0), "jusqu'à la veille de son retour");
    // L'instant de la rentrée est immuable et n'existe que pour une sortie rentrée ou close.
    sqlx::query("UPDATE ent_sortie SET rentree_le = rentree_le + interval '5 days' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    let relu: DateTime<Utc> = sqlx::query_scalar("SELECT rentree_le FROM ent_sortie WHERE id = $1")
        .bind(x.s.id)
        .fetch_one(x.c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(relu, rentree_le, "rentree_le est immuable");
    let r = sqlx::query("UPDATE ent_sortie SET rentree_le = now() WHERE etat <> 'rentree' AND etat <> 'close' AND id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(r.rows_affected(), 0);
    let hors: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie WHERE rentree_le IS NOT NULL AND etat NOT IN ('rentree', 'close')",
    )
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        hors, 0,
        "aucun instant de rentrée hors d'une sortie rentrée ou close"
    );
    // La base refuse une sortie « rentrée » sans instant, ou un instant sur une sortie non rentrée (23514 : contrainte).
    let r = sqlx::query("UPDATE ent_sortie SET etat = 'preparation' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await;
    assert!(r.is_ok(), "la réouverture efface l'instant : {r:?}");
    let efface: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT rentree_le FROM ent_sortie WHERE id = $1")
            .bind(x.s.id)
            .fetch_one(x.c.ent.pool_verif())
            .await
            .unwrap();
    assert!(efface.is_none());
    // Faible 7 : un instant de rentrée fourni en SQL direct est borné (pas au-delà de 7 jours dans l'avenir).
    let r = sqlx::query("UPDATE ent_sortie SET etat = 'rentree', rentree_le = now() + interval '30 days' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "rentrée dans l'avenir lointain"
    );
    let r = sqlx::query("UPDATE ent_sortie SET etat = 'rentree', rentree_le = now() - interval '2 days' WHERE id = $1")
        .bind(x.s.id)
        .execute(x.c.ent.pool_verif())
        .await;
    assert!(
        r.is_ok(),
        "un instant passé (rattrapage) reste accepté : {r:?}"
    );
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
    // 656 (Faible 1) : la fonction de 656 est celle de 655 SANS la branche directe « séjour scolaire » : aucune ligne
    // nouvelle, et les seules lignes retirées sont celles de cette branche (un retrait n'est lu que dans `ent_internat_retrait`).
    let m656 = include_str!("../../../migrations/656_sortie_voyages_v3b_suites.sql");
    let c = normaliser(corps(m656));
    let nouvelles: Vec<&String> = c.iter().filter(|l| !b.contains(l)).collect();
    assert!(
        nouvelles.is_empty(),
        "656 n'ajoute aucune ligne à 655 : {nouvelles:?}"
    );
    let retirees: Vec<&String> = b.iter().filter(|l| !c.contains(l)).collect();
    assert!(
        retirees
            .iter()
            .any(|l| l.contains("ent_sortie_retrait_effectif")),
        "la branche directe contenait le test d'effectivité : {retirees:?}"
    );
    assert!(
        !c.iter()
            .any(|l| l.contains("ent_sortie_internat_retrait")
                || l.contains("ent_sortie_participant")),
        "plus aucune lecture directe des participants d'une sortie dans l'attendu"
    );
    assert!(
        retirees.len() <= 26,
        "seule la branche directe est retirée : {retirees:?}"
    );
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
                lignes: vec![BordereauLigneSaisie {
                    participant_id: x.p1,
                    numero_cheque: "901".into(),
                    montant: "100".into(),
                }],
            },
        )
        .await
        .expect("bordereau");
    let l = bv.lignes[0].id;
    for (qui, acteur) in [("parent", &parent), ("payeur", &payeur_c)] {
        let e = ent
            .voyage_bordereau_remettre(acteur, x.s.id, bv.id, vec![l], None)
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "remise, {qui} : {e:?}");
    }
    let r = ent
        .voyage_bordereau_remettre(&compta_libre(&x).await, x.s.id, bv.id, vec![l], None)
        .await
        .expect("remise par une personne libre");
    assert!(r.remise.is_some());
}

// ═════════════════════════════ V3b : suites de la contre-revue des suites (656) ═════════════════════════════

/// Instant de référence des tests de contournement : l'horloge de la BASE (qui pose seule les horodatages), puis des
/// décalages en jours entiers : les expirations (30 jours) se jouent par rapport à ce que la base a écrit, jamais au jour UTC.
async fn instant_base(x: &Suivi) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT now()")
        .fetch_one(x.c.ent.pool_verif())
        .await
        .unwrap()
}

use crate::voyages_contournement::DemandeContournement;

fn dem(action: &str, motif: &str) -> DemandeContournement {
    DemandeContournement {
        action: action.into(),
        motif: motif.into(),
        cible_id: None,
        montant_max: None,
        prestataire_id: None,
    }
}

fn dem_creer(presta: Uuid, motif: &str) -> DemandeContournement {
    DemandeContournement {
        montant_max: Some("100".into()),
        prestataire_id: Some(presta),
        ..dem("acompte_creer", motif)
    }
}

fn dem_payer(acompte: Uuid, presta: Uuid, max: &str, motif: &str) -> DemandeContournement {
    DemandeContournement {
        cible_id: Some(acompte),
        montant_max: Some(max.into()),
        prestataire_id: Some(presta),
        ..dem("acompte_payer", motif)
    }
}

fn dem_annuler(acompte: Uuid, motif: &str) -> DemandeContournement {
    DemandeContournement {
        cible_id: Some(acompte),
        ..dem("acompte_annuler", motif)
    }
}

fn dem_collective(motif: &str) -> DemandeContournement {
    dem("annulation_collective", motif)
}

fn plus(t: DateTime<Utc>, jours: i64) -> DateTime<Utc> {
    t + chrono::Duration::days(jours)
}

async fn un_prestataire(x: &Suivi) -> Uuid {
    use crate::voyages_prestataires_repo::PrestataireSaisie;
    x.c.ent
        .prestataire_creer(
            &x.sec(),
            PrestataireSaisie {
                raison_sociale: format!("Cars V3b {}", Uuid::new_v4().simple()),
                siren: None,
                telephone: None,
                categories: vec!["transporteur".into()],
                adresses: vec![format!("v3b-{}@exemple-test.fr", Uuid::new_v4().simple())],
                version: None,
                actif: None,
            },
        )
        .await
        .unwrap()
        .id
}

fn acompte_de(presta: Uuid, jour: chrono::NaiveDate) -> crate::voyages_argent_flux::AcompteSaisie {
    crate::voyages_argent_flux::AcompteSaisie {
        prestataire_id: presta,
        commande_id: None,
        libelle: "Acompte".into(),
        montant: "100".into(),
        echeance: jour + chrono::Duration::days(10),
    }
}

async fn nb_refus(x: &Suivi, user: Uuid, action: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_argent_refus WHERE sortie_id = $1 AND user_id = $2 AND action = $3",
    )
    .bind(x.s.id)
    .bind(user)
    .bind(action)
    .fetch_one(x.c.ent.pool_verif())
    .await
    .unwrap()
}

/// Moyen 3 : contournement tracé de la règle « sortie entière » par une seconde personne habilitée et libre.
#[tokio::test]
async fn v3b_contournement_trace_de_la_sortie_entiere_par_une_seconde_personne_libre() {
    use crate::voyages::Commande;
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    // La seule personne habilitée est liée à l'élève 1 ; une autre, libre, est habilitée aussi ; une troisième est liée.
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let lie2 = compta_parent_de(&x, x.c.eleves[0]).await;
    let libre = x.compta();
    // Sans contournement : refus de la règle 3 (inchangé), tracé.
    let e = ent
        .voyage_acompte_creer_avec(&lie, x.s.id, acompte_de(presta, jour), None, t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // Une demande d'une personne libre : aucun contournement nécessaire (409). Une action inconnue : 422 (invalide).
    let e = ent
        .voyage_contournement_demander_a(&libre, x.s.id, dem_creer(presta, "Je suis libre"), t0)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)),
        "{e:?}"
    );
    let e = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem("tout_faire", "Motif"), t0)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    // Le motif est obligatoire ; le parent et le professeur n'ont aucune prise.
    for motif in ["", "   "] {
        assert!(ent
            .voyage_contournement_demander_a(&lie, x.s.id, dem_creer(presta, motif), t0)
            .await
            .is_err());
    }
    assert!(est_introuvable(
        &ent.voyage_contournement_demander_a(
            &x.parent(x.parent_c),
            x.s.id,
            dem_creer(presta, "Motif"),
            t0
        )
        .await
        .unwrap_err()
    ));
    assert!(est_forbidden(
        &ent.voyage_contournement_demander_a(&x.porteur(), x.s.id, dem_creer(presta, "Motif"), t0)
            .await
            .unwrap_err()
    ));
    // Demande valable ; un doublon ouvert est un conflit.
    let c = ent
        .voyage_contournement_demander_a(
            &lie,
            x.s.id,
            dem_creer(presta, "Seule comptable présente"),
            t0,
        )
        .await
        .unwrap();
    assert_eq!(c.statut, "en_attente");
    let e = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_creer(presta, "Encore"), t0)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    // Non décidée : l'usage reste refusé (même avec l'identifiant).
    let e = ent
        .voyage_acompte_creer_avec(&lie, x.s.id, acompte_de(presta, jour), Some(c.id), t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // Décision : pas la demandeuse, pas une personne liée à la sortie, pas sans droit ; tracé.
    for (qui, acteur) in [("demandeuse", &lie), ("liée", &lie2)] {
        let e = ent
            .voyage_contournement_decider_a(acteur, x.s.id, c.id, true, None, plus(t0, 0))
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "{qui} : {e:?}");
    }
    assert!(est_forbidden(
        &ent.voyage_contournement_decider_a(&x.porteur(), x.s.id, c.id, true, None, t0)
            .await
            .unwrap_err()
    ));
    assert_eq!(
        nb_refus(&x, lie.user_id, "voyage_contournement_decider_a").await,
        1
    );
    assert_eq!(
        nb_refus(&x, lie2.user_id, "voyage_contournement_decider_a").await,
        1
    );
    // Un refus sans motif est invalide ; avec motif, il clôt la demande (plus d'usage), une nouvelle est possible.
    assert!(ent
        .voyage_contournement_decider_a(&libre, x.s.id, c.id, false, None, t0)
        .await
        .is_err());
    let refusee = ent
        .voyage_contournement_decider_a(&libre, x.s.id, c.id, false, Some("Pas nécessaire"), t0)
        .await
        .unwrap();
    assert_eq!(refusee.statut, "refuse");
    let e = ent
        .voyage_acompte_creer_avec(&lie, x.s.id, acompte_de(presta, jour), Some(c.id), t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // Nouvelle demande, autorisée, utilisée UNE fois pour UNE écriture.
    let c = ent
        .voyage_contournement_demander_a(
            &lie,
            x.s.id,
            dem_creer(presta, "Seule comptable présente"),
            t0,
        )
        .await
        .unwrap();
    let ok = ent
        .voyage_contournement_decider_a(&libre, x.s.id, c.id, true, None, plus(t0, 1))
        .await
        .unwrap();
    assert_eq!(ok.statut, "autorise");
    // Une décision tardive ou doublée : conflit.
    let e = ent
        .voyage_contournement_decider_a(&libre, x.s.id, c.id, false, Some("Trop tard"), plus(t0, 1))
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    // Un contournement d'une autre action ou d'une autre personne ne sert pas.
    let ac_autre = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let autre = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_autre.id, "Autre action"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre, x.s.id, autre.id, true, None, t0)
        .await
        .unwrap();
    let e = ent
        .voyage_acompte_creer_avec(
            &lie,
            x.s.id,
            acompte_de(presta, jour),
            Some(autre.id),
            plus(t0, 1),
        )
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "autre action : {e:?}");
    let e = ent
        .voyage_acompte_creer_avec(
            &lie2,
            x.s.id,
            acompte_de(presta, jour),
            Some(c.id),
            plus(t0, 1),
        )
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "autre personne : {e:?}");
    // Usage : l'acompte est créé par la personne liée, et le contournement est consommé (cible = l'acompte).
    let ac = ent
        .voyage_acompte_creer_avec(
            &lie,
            x.s.id,
            acompte_de(presta, jour),
            Some(c.id),
            plus(t0, 2),
        )
        .await
        .expect("acompte par contournement");
    let cible: Uuid = sqlx::query_scalar(
        "SELECT cible_id FROM ent_sortie_argent_contournement_evt WHERE contournement_id = $1 AND evenement = 'utilise'",
    )
    .bind(c.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(cible, ac.id);
    let e = ent
        .voyage_acompte_creer_avec(
            &lie,
            x.s.id,
            acompte_de(presta, jour),
            Some(c.id),
            plus(t0, 2),
        )
        .await
        .unwrap_err();
    assert!(
        est_interdit_metier(&e),
        "un contournement ne sert qu'une fois : {e:?}"
    );
    // Le paiement par la personne liée : sans contournement refusé ; avec un contournement « payer » (même acompte, même
    // prestataire, sous le plafond) accepté.
    let pay = ent
        .voyage_contournement_demander_a(
            &lie,
            x.s.id,
            dem_payer(ac.id, presta, "100", "Paiement"),
            plus(t0, 2),
        )
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre, x.s.id, pay.id, true, None, plus(t0, 2))
        .await
        .unwrap();
    let e = ent
        .voyage_acompte_payer(&lie, x.s.id, ac.id, None, None, None)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    let paye = ent
        .voyage_acompte_payer_avec(
            &lie,
            x.s.id,
            ac.id,
            None,
            None,
            None,
            Some(pay.id),
            plus(t0, 2),
        )
        .await
        .expect("paiement par contournement");
    assert!(paye.paye_le.is_some());
    // Annulation d'un second acompte : l'auteur est gardé en base (Faible 6).
    let ac2 = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let e = ent
        .voyage_acompte_annuler(&lie, x.s.id, ac2.id, "Erreur")
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    let ann = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac2.id, "Doublon"), t0)
        .await
        .unwrap();
    // Expiration : une autorisation non utilisée expire (31 jours).
    ent.voyage_contournement_decider_a(&libre, x.s.id, ann.id, true, None, t0)
        .await
        .unwrap();
    let e = ent
        .voyage_acompte_annuler_avec(&lie, x.s.id, ac2.id, "Doublon", Some(ann.id), plus(t0, 31))
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "autorisation expirée : {e:?}");
    ent.voyage_acompte_annuler_avec(&lie, x.s.id, ac2.id, "Doublon", Some(ann.id), plus(t0, 29))
        .await
        .expect("annulation par contournement");
    let auteur: Option<Uuid> = sqlx::query_scalar(
        "SELECT annule_par_user_id FROM ent_sortie_acompte_fournisseur WHERE id = $1",
    )
    .bind(ac2.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        auteur,
        Some(lie.user_id),
        "auteur de l'annulation gardé en base"
    );
    // Une demande non décidée depuis plus de 30 jours expire : plus de décision possible.
    let vieille = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_collective("Sortie annulée"), t0)
        .await
        .unwrap();
    let e = ent
        .voyage_contournement_decider_a(&libre, x.s.id, vieille.id, true, None, plus(t0, 31))
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    let liste = ent
        .voyage_contournements_lister_a(&libre, x.s.id, plus(t0, 31))
        .await
        .unwrap();
    assert_eq!(
        liste.iter().find(|v| v.id == vieille.id).unwrap().statut,
        "expire"
    );
    assert!(liste.iter().any(|v| v.statut == "utilise"));
    assert!(est_introuvable(
        &ent.voyage_contournements_lister_a(&x.parent(x.parent_c), x.s.id, t0)
            .await
            .unwrap_err()
    ));
    // Annulation collective : sortie annulée avec un versement d'un participant dont la personne est parente.
    x.verser(x.p1, x.compte_c, "cheque", "50").await.unwrap();
    commande_a(&x, Commande::Annuler, None, t0).await.unwrap();
    let e = ent
        .voyage_avoirs_annulation_collective(&lie, x.s.id)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    // La demande « vieille » (expirée seulement dans l'avenir simulé) est encore ouverte à l'instant réel : une seconde
    // demande identique est un conflit (Faible 1), on décide donc celle-ci.
    let e = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_collective("Encore"), t0)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    ent.voyage_contournement_decider_a(&libre, x.s.id, vieille.id, true, None, t0)
        .await
        .unwrap();
    let n = ent
        .voyage_avoirs_annulation_collective_avec(&lie, x.s.id, Some(vieille.id), plus(t0, 1))
        .await
        .expect("annulation collective par contournement");
    assert!(n >= 1);
    // Chaque fait est en ajout seul (UPDATE, DELETE, TRUNCATE : 42501).
    for sql in [
        "UPDATE ent_sortie_argent_contournement SET motif = 'x'",
        "DELETE FROM ent_sortie_argent_contournement",
        "UPDATE ent_sortie_argent_contournement_evt SET motif = 'x'",
        "DELETE FROM ent_sortie_argent_contournement_evt",
        "TRUNCATE ent_sortie_argent_contournement_evt",
    ] {
        let r = sqlx::query(sql).execute(ent.pool_verif()).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "{sql}");
    }
}

/// Moyen 3 : la base garde elle-même le contournement (on ne passe pas par l'application).
#[tokio::test]
async fn v3b_la_base_n_accepte_un_acompte_d_une_personne_liee_que_avec_un_contournement_utilise_pour_cette_ecriture(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let lie2 = compta_parent_de(&x, x.c.eleves[0]).await;
    let libre = x.compta();
    let inserer = |id: Uuid, par: Uuid| {
        sqlx::query(
            "INSERT INTO ent_sortie_acompte_fournisseur (id, etablissement_id, sortie_id, prestataire_id, libelle, montant_centimes, echeance, cree_par_user_id)
             VALUES ($1,$2,$3,$4,'x',100,$5,$6)",
        )
        .bind(id)
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(presta)
        .bind(jour)
        .bind(par)
    };
    let r = inserer(Uuid::new_v4(), lie.user_id)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "sans contournement");
    let c = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_creer(presta, "Motif"), t0)
        .await
        .unwrap();
    // Les faits eux-mêmes sont gardés : « utilisé » sans autorisation, décision de la demandeuse ou d'une personne liée.
    let evt = |evenement: &str, par: Uuid, cible: Option<Uuid>| {
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement_evt (id, etablissement_id, contournement_id, evenement, par_user_id, cible_id, le)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(c.id)
        .bind(evenement.to_string())
        .bind(par)
        .bind(cible)
        .bind(t0)
    };
    for (nom, evenement, par, cible) in [
        (
            "utilisé sans autorisation",
            "utilise",
            lie.user_id,
            Some(Uuid::new_v4()),
        ),
        ("autorisé par la demandeuse", "autorise", lie.user_id, None),
        (
            "autorisé par une personne liée",
            "autorise",
            lie2.user_id,
            None,
        ),
        ("refusé par la demandeuse", "refuse", lie.user_id, None),
    ] {
        let r = evt(evenement, par, cible).execute(ent.pool_verif()).await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "{nom}");
    }
    let r = evt("utilise", libre.user_id, Some(Uuid::new_v4()))
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "utilisé par une autre personne que la demandeuse"
    );
    evt("autorise", libre.user_id, None)
        .execute(ent.pool_verif())
        .await
        .expect("autorisation par la personne libre");
    // Autorisée mais pas encore utilisée : l'écriture reste refusée ; utilisée pour UNE cible : cette écriture passe, pas une autre.
    let cible = Uuid::new_v4();
    let r = inserer(cible, lie.user_id).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "autorisé, non utilisé"
    );
    evt("utilise", lie.user_id, Some(cible))
        .execute(ent.pool_verif())
        .await
        .expect("utilisation");
    let r = inserer(Uuid::new_v4(), lie.user_id)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "une autre cible");
    let r = inserer(cible, lie2.user_id).execute(ent.pool_verif()).await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "une autre personne");
    inserer(cible, lie.user_id)
        .execute(ent.pool_verif())
        .await
        .expect("cette écriture, par cette personne");
    // L'usage est unique (index) : un second fait « utilisé » est refusé.
    let r = evt("utilise", lie.user_id, Some(Uuid::new_v4()))
        .execute(ent.pool_verif())
        .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23505"), "un seul usage");
}

/// V3b, Moyen 3 (revue) : le rattrapage de 656 (`rentree_le`, `annulee_le`) atteint les lignes **sous la RLS stricte** : il
/// énumère les établissements comme 2220 (`ent_etablissement UNION ent_membership`) et pose le contexte pour chacun. Sans
/// contexte, un `UPDATE` naïf ne verrait aucune ligne. Le réglage précédent est restauré.
#[cfg(feature = "labo")]
#[tokio::test]
async fn v3b_le_rattrapage_de_656_atteint_les_lignes_sous_la_rls_stricte() {
    let Some(x) = suivi_v3().await else { return };
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return;
    };
    let (proprietaire, strict) = crate::tests_rls_stricte::proprietaire_et_strict(&url).await;
    let sql = include_str!("../../../migrations/656_sortie_voyages_v3b_suites.sql");
    let debut = sql.find("-- DEBUT-DONNEES").expect("repère de début") + "-- DEBUT-DONNEES".len();
    let fin = sql.find("-- FIN-DONNEES").expect("repère de fin");
    for (etat, colonne) in [("rentree", "rentree_le"), ("annulee", "annulee_le")] {
        // État antérieur aux colonnes (cas rattrapé) : contraintes et déclencheurs de pose écartés le temps de le fabriquer.
        for ddl in [
            "ALTER TABLE ent_sortie DISABLE TRIGGER ent_sortie_rentree_le_tg".to_string(),
            "ALTER TABLE ent_sortie DISABLE TRIGGER ent_sortie_annulee_le_tg".to_string(),
            "ALTER TABLE ent_sortie DROP CONSTRAINT IF EXISTS ent_sortie_rentree_le_chk".to_string(),
            "ALTER TABLE ent_sortie DROP CONSTRAINT IF EXISTS ent_sortie_annulee_le_chk".to_string(),
            format!("UPDATE ent_sortie SET etat = '{etat}', rentree_le = NULL, annulee_le = NULL WHERE id = '{}'", x.s.id),
            "ALTER TABLE ent_sortie ENABLE TRIGGER ent_sortie_rentree_le_tg".to_string(),
            "ALTER TABLE ent_sortie ENABLE TRIGGER ent_sortie_annulee_le_tg".to_string(),
        ] {
            sqlx::query(&ddl).execute(proprietaire.pool()).await.unwrap();
        }
        // Préalable : sans contexte, un UPDATE ordinaire sous le rôle strict ne touche aucune ligne.
        let naif = sqlx::query(&format!(
            "UPDATE ent_sortie SET {colonne} = now() WHERE {colonne} IS NULL AND etat = '{etat}'"
        ))
        .execute(strict.pool())
        .await
        .expect("UPDATE sans contexte");
        assert_eq!(
            naif.rows_affected(),
            0,
            "préalable : la RLS stricte masque tout sans contexte ({etat})"
        );
        // Le bloc de données de la migration, exécuté par le rôle strict.
        sqlx::raw_sql(&sql[debut..fin])
            .execute(strict.pool())
            .await
            .expect("rattrapage sous rôle strict");
        let pose: bool = sqlx::query_scalar(&format!(
            "SELECT {colonne} IS NOT NULL FROM ent_sortie WHERE id = $1"
        ))
        .bind(x.s.id)
        .fetch_one(proprietaire.pool())
        .await
        .unwrap();
        assert!(pose, "{colonne} rattrapée sous la RLS stricte");
    }
    // Palier des faibles, lot 2 (précédent de M-4, contre-revue V3b suites 2) : ne remet QUE les deux contraintes
    // écartées. Un rejeu complet de 656 puis 657 remettait un instant (et, depuis 658, durablement) des fonctions d'avant
    // sous les tests voisins de la même base.
    for ddl in [
        "ALTER TABLE ent_sortie ADD CONSTRAINT ent_sortie_rentree_le_chk
            CHECK ((etat <> 'rentree' OR rentree_le IS NOT NULL) AND (rentree_le IS NULL OR etat IN ('rentree', 'close')))",
        "ALTER TABLE ent_sortie ADD CONSTRAINT ent_sortie_annulee_le_chk
            CHECK ((etat = 'annulee') = (annulee_le IS NOT NULL))",
    ] {
        sqlx::query(ddl)
            .execute(proprietaire.pool())
            .await
            .expect("contrainte rétablie");
    }
    crate::tenant_tx::forcer_mode_test(None);
}

/// Moyen 1 : pas d'autorisation « en blanc ». La demande porte sa portée (acompte visé, plafond, prestataire) ; une
/// autorisation ne couvre que cela, au service comme en SQL direct. Moyen 2 : expiration, habilitation de l'autorisatrice
/// (à l'autorisation ET à l'usage), horodatages posés par la base, garde de base de l'annulation collective.
#[tokio::test]
async fn v3b_une_autorisation_ne_couvre_que_ce_que_la_demande_porte_service_et_sql_direct() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let autre_presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    let ac_a = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let ac_b = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    // Portée absente ou incohérente : invalide ; cible inconnue : introuvable ; autre prestataire que l'acompte : invalide.
    for (nom, d) in [
        ("créer sans portée", dem("acompte_creer", "Motif")),
        ("payer sans cible", dem("acompte_payer", "Motif")),
        ("annuler sans cible", dem("acompte_annuler", "Motif")),
        (
            "collective avec plafond",
            DemandeContournement {
                montant_max: Some("10".into()),
                ..dem_collective("Motif")
            },
        ),
        ("plafond nul", dem_payer(ac_a.id, presta, "0", "Motif")),
        (
            "autre prestataire que l'acompte",
            dem_payer(ac_a.id, autre_presta, "100", "Motif"),
        ),
    ] {
        let e = ent
            .voyage_contournement_demander_a(&lie, x.s.id, d, t0)
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Domain(_)), "{nom} : {e:?}");
    }
    let e = ent
        .voyage_contournement_demander_a(
            &lie,
            x.s.id,
            dem_payer(Uuid::new_v4(), presta, "100", "Motif"),
            t0,
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Payer l'acompte A : plafond 100 €, prestataire P. L'autorisatrice voit la portée.
    let c = ent
        .voyage_contournement_demander_a(
            &lie,
            x.s.id,
            dem_payer(ac_a.id, presta, "100", "Paiement A"),
            t0,
        )
        .await
        .unwrap();
    assert_eq!(
        (c.cible_id, c.montant_max_centimes, c.prestataire_id),
        (Some(ac_a.id), Some(10_000), Some(presta))
    );
    let vue = ent
        .voyage_contournements_lister_a(&libre, x.s.id, t0)
        .await
        .unwrap();
    assert_eq!(
        vue.iter().find(|v| v.id == c.id).unwrap().cible_id,
        Some(ac_a.id)
    );
    ent.voyage_contournement_decider_a(&libre, x.s.id, c.id, true, None, t0)
        .await
        .unwrap();
    // Une demande identique encore ouverte (autorisée, non utilisée) : conflit (Faible 1).
    let e = ent
        .voyage_contournement_demander_a(
            &lie,
            x.s.id,
            dem_payer(ac_a.id, presta, "100", "Encore"),
            t0,
        )
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    // Service : un AUTRE acompte, un montant SUPÉRIEUR au plafond : refusés (tracés comme la règle 3).
    let e = ent
        .voyage_acompte_payer_avec(&lie, x.s.id, ac_b.id, None, None, None, Some(c.id), t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "autre acompte : {e:?}");
    let e = ent
        .voyage_acompte_payer_avec(
            &lie,
            x.s.id,
            ac_a.id,
            Some("150"),
            None,
            None,
            Some(c.id),
            t0,
        )
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "montant supérieur : {e:?}");
    // SQL direct : « utilisé » pour une autre cible que celle de la demande : 23514 ; pour la bonne : accepté.
    let utilise = |cid: Uuid, par: Uuid, cible: Uuid| {
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement_evt (id, etablissement_id, contournement_id, evenement, par_user_id, cible_id)
             VALUES ($1,$2,$3,'utilise',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(cid)
        .bind(par)
        .bind(cible)
    };
    let r = utilise(c.id, lie.user_id, ac_b.id)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "autre cible");
    utilise(c.id, lie.user_id, ac_a.id)
        .execute(ent.pool_verif())
        .await
        .expect("bonne cible");
    // Expiration en base : valable maintenant et 29 jours après l'autorisation, plus au-delà de 30 jours.
    let valide_a = |decalage: &'static str| {
        sqlx::query_scalar::<_, bool>(
            "SELECT ent_sortie_contournement_valide($1,$2,$3,'acompte_payer',$4,10000,$5, now() + $6::interval)",
        )
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(lie.user_id)
        .bind(ac_a.id)
        .bind(presta)
        .bind(decalage)
        .fetch_one(ent.pool_verif())
    };
    assert!(valide_a("0 days").await.unwrap());
    assert!(valide_a("29 days").await.unwrap());
    assert!(
        !valide_a("31 days").await.unwrap(),
        "expirée 30 jours après l'autorisation"
    );
    // SQL direct, paiement : autre acompte, montant supérieur, puis conforme.
    let payer = |acompte: Uuid, centimes: i64| {
        sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur SET paye_le = $3, paye_centimes = $4, paye_par_user_id = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(x.c.etab)
        .bind(acompte)
        .bind(jour)
        .bind(centimes)
        .bind(lie.user_id)
    };
    let r = payer(ac_b.id, 10_000).execute(ent.pool_verif()).await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "autre acompte");
    let r = payer(ac_a.id, 20_000).execute(ent.pool_verif()).await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "au-delà du plafond");
    payer(ac_a.id, 10_000)
        .execute(ent.pool_verif())
        .await
        .expect("dans la portée");
    // Créer : plafond 100 € et prestataire P. Autre prestataire ou montant supérieur : refusés (service puis SQL direct).
    let cc = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_creer(presta, "Créer"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre, x.s.id, cc.id, true, None, t0)
        .await
        .unwrap();
    let e = ent
        .voyage_acompte_creer_avec(
            &lie,
            x.s.id,
            acompte_de(autre_presta, jour),
            Some(cc.id),
            t0,
        )
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "autre prestataire : {e:?}");
    let gros = crate::voyages_argent_flux::AcompteSaisie {
        montant: "150".into(),
        ..acompte_de(presta, jour)
    };
    let e = ent
        .voyage_acompte_creer_avec(&lie, x.s.id, gros, Some(cc.id), t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "montant supérieur : {e:?}");
    let nid = Uuid::new_v4();
    utilise(cc.id, lie.user_id, nid)
        .execute(ent.pool_verif())
        .await
        .expect("utilisé pour la future cible");
    let inserer = |prest: Uuid, centimes: i64| {
        sqlx::query(
            "INSERT INTO ent_sortie_acompte_fournisseur (id, etablissement_id, sortie_id, prestataire_id, libelle, montant_centimes, echeance, cree_par_user_id)
             VALUES ($1,$2,$3,$4,'x',$5,$6,$7)",
        )
        .bind(nid)
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(prest)
        .bind(centimes)
        .bind(jour)
        .bind(lie.user_id)
    };
    let r = inserer(autre_presta, 10_000)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "autre prestataire (SQL direct)"
    );
    let r = inserer(presta, 20_000).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "au-delà du plafond (SQL direct)"
    );
    inserer(presta, 10_000)
        .execute(ent.pool_verif())
        .await
        .expect("dans la portée");
    // Moyen 2, horodatages posés par la base : un `le` ou un `demande_le` fournis sont ignorés.
    let cid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_argent_contournement (id, etablissement_id, sortie_id, action, demande_par_user_id, motif, demande_le)
         VALUES ($1,$2,$3,'annulation_collective',$4,'Motif','2000-01-01T00:00:00Z')",
    )
    .bind(cid)
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(lie.user_id)
    .execute(ent.pool_verif())
    .await
    .expect("demande directe");
    let decide = |par: Uuid| {
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement_evt (id, etablissement_id, contournement_id, evenement, par_user_id, le)
             VALUES ($1,$2,$3,'autorise',$4,'2000-01-01T00:00:00Z')",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(cid)
        .bind(par)
    };
    // Habilitation (Moyen 2) : un membre sans le profil comptable, même libre de la sortie, n'autorise pas.
    let r = decide(x.porteur().user_id).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "membre non habilité"
    );
    let r = decide(Uuid::new_v4()).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "pas membre de l'établissement"
    );
    // Contre-revue du pilote (B1) : un comptable libre dont l'accès est suspendu, fermé ou échu garde son rôle mais
    // n'autorise plus.
    for (etat, fin) in [
        ("suspendu", None),
        ("ferme", None),
        ("ouvert", Some("2000-01-01")),
    ] {
        let inactif = compta_libre(&x).await;
        sqlx::query(
            "UPDATE ent_membership SET etat = $3, date_fin = $4::date WHERE user_id = $1 AND etablissement_id = $2",
        )
        .bind(inactif.user_id)
        .bind(x.c.etab)
        .bind(etat)
        .bind(fin)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        let r = decide(inactif.user_id).execute(ent.pool_verif()).await;
        assert_eq!(
            code_sql(&r).as_deref(),
            Some("23514"),
            "accès {etat} / fin {fin:?} : n'autorise pas"
        );
    }
    decide(libre.user_id)
        .execute(ent.pool_verif())
        .await
        .expect("autorisation habilitée");
    // Contre-revue du pilote (M1) : un acompte inséré déjà payé (ou annulé) est refusé, quel que soit son auteur.
    for colonnes in [
        "paye_le, paye_centimes, paye_par_user_id) VALUES ($1,$2,$3,$4,'x',100,$5,$6, $5, 100000000, $6)",
        "annule_le, annule_motif) VALUES ($1,$2,$3,$4,'x',100,$5,$6, $5, 'Doublon')",
    ] {
        let r = sqlx::query(&format!(
            "INSERT INTO ent_sortie_acompte_fournisseur (id, etablissement_id, sortie_id, prestataire_id, libelle, montant_centimes, echeance, cree_par_user_id, {colonnes}"
        ))
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(presta)
        .bind(jour)
        .bind(libre.user_id)
        .execute(ent.pool_verif())
        .await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "acompte inséré payé ou annulé : {colonnes}");
    }
    let (d_le, e_le): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        "SELECT c.demande_le, e.le FROM ent_sortie_argent_contournement c
           JOIN ent_sortie_argent_contournement_evt e ON e.contournement_id = c.id WHERE c.id = $1",
    )
    .bind(cid)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        d_le > t0 - chrono::Duration::minutes(1) && e_le > t0 - chrono::Duration::minutes(1),
        "{d_le} {e_le}"
    );
    // Faible 2 : l'habilitation de l'autorisatrice est revérifiée à l'usage. Autorisation par `libre2`, puis retrait de
    // son profil : l'usage est refusé (SQL direct et service).
    let libre2 = compta_libre(&x).await;
    let c3 = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_b.id, "Doublon"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre2, x.s.id, c3.id, true, None, t0)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_membership SET role = 'prof' WHERE user_id = $1 AND etablissement_id = $2",
    )
    .bind(libre2.user_id)
    .bind(x.c.etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = utilise(c3.id, lie.user_id, ac_b.id)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "autorisatrice plus habilitée"
    );
    let e = ent
        .voyage_acompte_annuler_avec(&lie, x.s.id, ac_b.id, "Doublon", Some(c3.id), t0)
        .await
        .unwrap_err();
    assert!(
        est_interdit_metier(&e),
        "service, autorisatrice plus habilitée : {e:?}"
    );
    // Annulation collective : garde de base (avoir « annulation collective » proposé par une personne liée).
    x.verser(x.p1, x.compte_c, "cheque", "50").await.unwrap();
    let avoir = |par: Option<Uuid>| {
        sqlx::query(
            "INSERT INTO ent_sortie_avoir_propose (id, etablissement_id, sortie_id, participant_id, compte_id, motif_code, cle,
                    paye_net_centimes, montant_propose_centimes, propose_par_user_id)
             VALUES ($1,$2,$3,$4,$5,'annulation_collective',$6,5000,5000,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(x.p1)
        .bind(x.compte_c)
        .bind(format!("directe-{}", Uuid::new_v4().simple()))
        .bind(par)
    };
    let r = avoir(Some(lie.user_id)).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "annulation collective par une personne liée"
    );
    let r = avoir(None).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "annulation collective sans auteur"
    );
    avoir(Some(libre.user_id))
        .execute(ent.pool_verif())
        .await
        .expect("annulation collective par une personne libre");
}

/// Moyen 4 : tests SQL directs (23514) des déclencheurs de 655 : frais réel, décision d'avoir (INSERT et UPDATE, y
/// compris l'exclusion de la décision système), paiement d'acompte (UPDATE), annulation d'acompte (656).
#[tokio::test]
async fn m4_les_declencheurs_de_655_refusent_en_base_frais_decision_d_avoir_et_paiement_d_acompte()
{
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let jour = x.c.jour_etab().await;
    let parent = compta_parent_de(&x, x.c.eleves[1]).await;
    let payeur_c = compta_payeur_de(&x, x.compte_c).await;
    let libre = compta_libre(&x).await;
    // Frais réel : INSERT direct par la personne liée ou payeuse refusé ; par une personne libre accepté.
    let frais = |par: Uuid| {
        sqlx::query(
            "INSERT INTO ent_sortie_frais_reel (id, etablissement_id, sortie_id, participant_id, nature, libelle, montant_centimes, saisi_par_user_id)
             VALUES ($1,$2,$3,$4,'autre','Billet',100,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(x.p1)
        .bind(par)
    };
    for (qui, a) in [("parent", &parent), ("payeur", &payeur_c)] {
        let r = frais(a.user_id).execute(ent.pool_verif()).await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "frais réel, {qui}");
    }
    frais(libre.user_id)
        .execute(ent.pool_verif())
        .await
        .expect("frais par une personne libre");
    // Avoir : un versement, puis le désistement du participant 1 → avoir proposé ; sa décision.
    x.verser(x.p1, x.compte_c, "cheque", "50").await.unwrap();
    ent.voyage_participant_desister(&x.porteur(), x.s.id, x.p1, "famille", None)
        .await
        .unwrap();
    let avoir = x
        .avoirs_de(x.p1, x.compte_c)
        .await
        .into_iter()
        .find(|a| a.statut == "propose")
        .expect("avoir proposé");
    let decider = |par: Option<Uuid>, statut: &str, auto: bool, id: Uuid| {
        sqlx::query(
            "UPDATE ent_sortie_avoir_propose
                SET statut = $3, montant_retenu_centimes = CASE WHEN $3 = 'retenu' THEN 0 END,
                    decide_par_user_id = $4, decide_le = now(), decision_motif = 'x', decision_auto = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(x.c.etab)
        .bind(id)
        .bind(statut.to_string())
        .bind(par)
        .bind(auto)
    };
    for (qui, a) in [("parent", &parent), ("payeur", &payeur_c)] {
        for statut in ["retenu", "ecarte"] {
            let r = decider(Some(a.user_id), statut, false, avoir.id)
                .execute(ent.pool_verif())
                .await;
            assert_eq!(
                code_sql(&r).as_deref(),
                Some("23514"),
                "décision {statut} (UPDATE), {qui}"
            );
        }
    }
    // INSERT direct d'un avoir déjà décidé par la personne liée : refusé aussi.
    let inserer = |par: Option<Uuid>, statut: &str, auto: bool| {
        sqlx::query(
            "INSERT INTO ent_sortie_avoir_propose
                 (id, etablissement_id, sortie_id, participant_id, compte_id, motif_code, cle, paye_net_centimes,
                  montant_propose_centimes, statut, montant_retenu_centimes, decide_par_user_id, decide_le, decision_auto)
             VALUES ($1,$2,$3,$4,$5,'motif_legitime',$6,0,0,$7, CASE WHEN $7 = 'retenu' THEN 0 END, $8, now(), $9)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(x.p1)
        .bind(x.compte_c)
        .bind(format!("direct-{}", Uuid::new_v4().simple()))
        .bind(statut.to_string())
        .bind(par)
        .bind(auto)
    };
    for (qui, a) in [("parent", &parent), ("payeur", &payeur_c)] {
        let r = inserer(Some(a.user_id), "retenu", false)
            .execute(ent.pool_verif())
            .await;
        assert_eq!(
            code_sql(&r).as_deref(),
            Some("23514"),
            "décision (INSERT), {qui}"
        );
    }
    // Exclusion : une décision « signée système » (sans acteur humain) n'exerce aucun pouvoir : l'INSERT passe ; une
    // proposition (sans décision) aussi.
    inserer(None, "ecarte", true)
        .execute(ent.pool_verif())
        .await
        .expect("décision système");
    // La même décision par une personne libre passe (UPDATE).
    decider(Some(libre.user_id), "retenu", false, avoir.id)
        .execute(ent.pool_verif())
        .await
        .expect("décision par une personne libre");
    // Acompte : UPDATE de paiement par la personne liée refusé (INSERT refusé déjà couvert) ; par une personne libre accepté.
    let presta = un_prestataire(&x).await;
    let ac = ent
        .voyage_acompte_creer(&x.compta(), x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let payer = |par: Uuid| {
        sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur SET paye_le = $3, paye_centimes = 100, paye_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(x.c.etab)
        .bind(ac.id)
        .bind(jour)
        .bind(par)
    };
    for (qui, a) in [("parent", &parent), ("payeur", &payeur_c)] {
        let r = payer(a.user_id).execute(ent.pool_verif()).await;
        assert_eq!(
            code_sql(&r).as_deref(),
            Some("23514"),
            "paiement d'acompte (UPDATE), {qui}"
        );
    }
    // Annulation d'acompte (656) : l'auteur est gardé ; refusée à la personne liée, sans auteur refusée par la contrainte.
    let ac2 = ent
        .voyage_acompte_creer(&x.compta(), x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let annuler = |par: Option<Uuid>| {
        sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur SET annule_le = $3, annule_motif = 'x', annule_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(x.c.etab)
        .bind(ac2.id)
        .bind(jour)
        .bind(par)
    };
    let r = annuler(Some(parent.user_id))
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "annulation d'acompte par la personne liée"
    );
    let r = annuler(None).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "annulation sans auteur (contrainte)"
    );
    annuler(Some(libre.user_id))
        .execute(ent.pool_verif())
        .await
        .expect("annulation par une personne libre");
    // Le paiement du premier acompte par une personne libre passe.
    payer(libre.user_id)
        .execute(ent.pool_verif())
        .await
        .expect("paiement par une personne libre");
}

/// Moyen 4 : `ent_sortie_issue_coherente` (655) : une aide ou un report reprend exactement sa demande.
#[tokio::test]
async fn m4_ent_sortie_issue_coherente_refuse_en_base_une_aide_ou_un_report_different_de_sa_demande(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let p2 = x.p2_meme_payeur().await;
    let d_aide = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "10"),
            instant_fixe(),
        )
        .await
        .unwrap()
        .demande_id;
    x.verser(x.p1, x.compte_c, "cheque", "100").await.unwrap();
    let d_report = ent
        .voyage_report_effectuer_a(
            &x.compta(),
            x.s.id,
            report(x.p1, p2, x.compte_c, "20"),
            instant_fixe(),
        )
        .await
        .unwrap()
        .demande_id;
    // Aide : source, nature de la demande (un report), sortie, compte.
    let aide_sql = |demande: Uuid, source: &str, participant: Uuid, compte: Uuid, montant: i64| {
        sqlx::query(
            "INSERT INTO ent_sortie_aide (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, motif, demande_id, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,'aide',$6,$7,'x',$8,$9, now())",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(participant)
        .bind(compte)
        .bind(source.to_string())
        .bind(montant)
        .bind(demande)
        .bind(x.c.dir)
    };
    for (nom, demande, source, participant, compte, montant) in [
        ("autre source", d_aide, "autre", x.p1, x.compte_c, 1000_i64),
        ("autre compte", d_aide, "apel", x.p1, x.compte_a, 1000),
        (
            "demande de report",
            d_report,
            "apel",
            x.p1,
            x.compte_c,
            2000,
        ),
        (
            "demande inconnue",
            Uuid::new_v4(),
            "apel",
            x.p1,
            x.compte_c,
            1000,
        ),
    ] {
        let r = aide_sql(demande, source, participant, compte, montant)
            .execute(ent.pool_verif())
            .await;
        assert!(
            matches!(code_sql(&r).as_deref(), Some("23514") | Some("23503")),
            "aide, {nom} : {r:?}"
        );
    }
    // Report : montant, participant d'origine, participant de destination, compte, nature (demande d'aide).
    let report_sql = |demande: Uuid, de: Uuid, vers: Uuid, compte: Uuid, montant: i64| {
        sqlx::query(
            "INSERT INTO ent_sortie_report (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, motif, demande_id, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,$6,'report',$7,'x',$8,$9, now())",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(de)
        .bind(vers)
        .bind(compte)
        .bind(montant)
        .bind(demande)
        .bind(x.c.dir)
    };
    for (nom, demande, de, vers, compte, montant) in [
        ("autre montant", d_report, x.p1, p2, x.compte_c, 2001_i64),
        ("autre origine", d_report, x.p0, p2, x.compte_c, 2000),
        ("autre destination", d_report, x.p1, x.p0, x.compte_c, 2000),
        ("autre compte", d_report, x.p1, p2, x.compte_a, 2000),
        ("demande d'aide", d_aide, x.p1, p2, x.compte_c, 1000),
    ] {
        let r = report_sql(demande, de, vers, compte, montant)
            .execute(ent.pool_verif())
            .await;
        assert_eq!(
            code_sql(&r).as_deref(),
            Some("23514"),
            "report, {nom} : {r:?}"
        );
    }
}

/// Faible 2 (a) : le seuil de seconde validation vaut aussi à l'annulation, sur le CUMUL engagé du participant et du
/// payeur : une aide fractionnée sous le seuil ne s'annule pas seule morceau par morceau.
/// Faible 2 (b) : une demande expirée (plus validable) ne compte plus dans le cumul.
#[tokio::test]
async fn v3b_le_seuil_vaut_a_l_annulation_sur_le_cumul_et_une_demande_expiree_ne_compte_plus() {
    let Some(x) = suivi().await else { return };
    regler_seconde_validation(&x, 15_000, false).await;
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_fixe();
    // Deux aides de 100 € sur la même part : la première passe seule, la seconde porte le cumul à 200 € (> 150 €).
    let a1 = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "100"),
            t0,
        )
        .await
        .unwrap();
    let aide1 = a1.aide.expect("première aide appliquée");
    let a2 = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "apel", "100"),
            t0,
        )
        .await
        .unwrap();
    assert!(
        a2.aide.is_none(),
        "la seconde dépasse le seuil en cumul : en attente"
    );
    ent.voyage_demande_valider_a(&x.dir(), x.s.id, a2.demande_id, t0)
        .await
        .expect("seconde personne");
    // Annuler la première (100 €, sous le seuil seule) : le cumul (200 €) la soumet à une seconde personne.
    let e = ent
        .voyage_aide_annuler_a(
            &x.compta(),
            x.s.id,
            aide1.id,
            "Erreur de saisie",
            plus(t0, 1),
        )
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    assert_eq!(
        nb_refus(&x, x.c.compta, "voyage_aide_annuler_a").await,
        1,
        "le refus de seconde personne est tracé (Faible 4)"
    );
    ent.voyage_aide_annuler_a(&x.dir(), x.s.id, aide1.id, "Erreur de saisie", plus(t0, 1))
        .await
        .expect("une autre personne annule");
    // (b) Une demande de 160 € reste en attente ; 31 jours plus tard elle est expirée et ne pèse plus sur le cumul.
    // Cumul net actuel : 100 € (la seconde aide) ; on repart d'un cadre neuf pour ne pas mêler les deux.
    let y = suivi().await.expect("cadre");
    regler_seconde_validation(&y, 15_000, false).await;
    y.sync().await;
    let attente =
        y.c.ent
            .voyage_aide_accorder_a(
                &y.compta(),
                y.s.id,
                aide(y.p1, y.compte_c, "apel", "160"),
                t0,
            )
            .await
            .unwrap();
    assert!(attente.aide.is_none(), "160 € > 150 € : en attente");
    let encore =
        y.c.ent
            .voyage_aide_accorder_a(
                &y.compta(),
                y.s.id,
                aide(y.p1, y.compte_c, "apel", "10"),
                plus(t0, 1),
            )
            .await
            .unwrap();
    assert!(
        encore.aide.is_none(),
        "10 € + 160 € en attente (non expirée) : en attente"
    );
    let apres =
        y.c.ent
            .voyage_aide_accorder_a(
                &y.compta(),
                y.s.id,
                aide(y.p1, y.compte_c, "apel", "10"),
                plus(t0, 31),
            )
            .await
            .unwrap();
    assert!(
        apres.aide.is_some(),
        "les demandes expirées ne comptent plus : 10 € passent seuls"
    );
}

/// Faible 4 : traces des refus de seconde personne ; un `23514` de course devient le refus de la règle 3 ; plafond de débit.
#[tokio::test]
async fn v3b_les_refus_de_seconde_personne_sont_traces_la_course_23514_devient_un_refus_et_le_debit_est_plafonne(
) {
    let Some(x) = suivi().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_fixe();
    // Seconde personne : le demandeur ne valide pas sa propre demande (fonds social, par défaut > seuil : en attente).
    let d = ent
        .voyage_aide_accorder_a(
            &x.compta(),
            x.s.id,
            aide(x.p1, x.compte_c, "fonds_social", "10"),
            t0,
        )
        .await
        .unwrap();
    assert!(d.aide.is_none());
    let e = ent
        .voyage_demande_valider_a(&x.compta(), x.s.id, d.demande_id, t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    assert_eq!(
        nb_refus(&x, x.c.compta, "voyage_demande_valider_a").await,
        1
    );
    // Course : un lien posé entre le contrôle de l'application et l'écriture ferait refuser l'INSERT par le déclencheur ;
    // ce `23514` est reconnu comme le refus de la règle 3 (et se trace comme lui), jamais comme une erreur 500.
    let parent = compta_parent_de(&x, x.c.eleves[1]).await;
    let r = sqlx::query(
        "INSERT INTO ent_sortie_plan (id, etablissement_id, sortie_id, participant_id, compte_id, rang, nb_echeances, accorde_par_user_id)
         VALUES ($1,$2,$3,$4,$5,98,2,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(x.p1)
    .bind(x.compte_c)
    .bind(parent.user_id)
    .execute(ent.pool_verif())
    .await;
    let e = crate::voyages_argent_v3::db_pouvoir(r.unwrap_err());
    assert!(crate::voyages_argent_v3::est_refus_regle3(&e), "{e:?}");
    assert!(crate::voyages_argent_v3::est_refus_a_tracer(&e));
    // Un autre 23514 (contrainte de forme) reste une erreur de saisie, pas un refus de la règle 3.
    let r = sqlx::query("UPDATE ent_sortie_plan SET nb_echeances = 0 WHERE sortie_id = $1")
        .bind(x.s.id)
        .execute(ent.pool_verif())
        .await;
    if let Err(e) = r {
        assert!(!crate::voyages_argent_v3::est_refus_regle3(
            &crate::voyages_argent_v3::db_pouvoir(e)
        ));
    }
    // Plafond de débit : 35 refus de la même personne laissent au plus PLAFOND traces par heure.
    let spammeur = compta_parent_de(&x, x.c.eleves[1]).await;
    for _ in 0..35 {
        let e = ent
            .voyage_plan_accorder(&spammeur, x.s.id, x.p1, x.compte_c, 2, None)
            .await
            .unwrap_err();
        assert!(est_interdit_metier(&e), "{e:?}");
    }
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_argent_refus WHERE sortie_id = $1 AND user_id = $2 AND action = $3 AND NOT plafond_atteint",
    )
    .bind(x.s.id)
    .bind(spammeur.user_id)
    .bind("voyage_plan_accorder")
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        n,
        crate::voyages_argent_v3::PLAFOND_TRACES_REFUS_PAR_HEURE,
        "plafond de traces"
    );
    // Faible 5 : au-delà du plafond, UNE ligne « plafond atteint » signale le dépassement (jamais davantage).
    let marqueurs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_argent_refus WHERE sortie_id = $1 AND user_id = $2 AND action = $3 AND plafond_atteint",
    )
    .bind(x.s.id)
    .bind(spammeur.user_id)
    .bind("voyage_plan_accorder")
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(marqueurs, 1, "une seule trace agrégée au-delà du plafond");
    // Le plafond est par (personne, action) : le flot d'une action ne masque pas le refus d'une autre.
    let e = ent
        .voyage_acompte_creer(
            &spammeur,
            x.s.id,
            acompte_de(un_prestataire(&x).await, x.c.jour_etab().await),
        )
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    assert_eq!(
        nb_refus(&x, spammeur.user_id, "voyage_acompte_creer").await,
        1
    );
}

// ═════════════════════════════ Suites V3b, 2 (contre-revue des correctifs : M2, M3, faibles 1 et 5) ═════════════════════════════

fn message_sql<T>(r: &Result<T, sqlx::Error>) -> String {
    match r {
        Err(sqlx::Error::Database(d)) => d.message().to_string(),
        _ => String::new(),
    }
}

/// Une demande de contournement **antidatée** de `jours` jours (horloge de la base) : le déclencheur d'horodatage est écarté
/// le temps de l'écriture, dans une transaction (même technique que `forcer_etat_seance`).
async fn demande_antidatee(
    x: &Suivi,
    par: Uuid,
    action: &str,
    cible: Option<Uuid>,
    max: Option<i64>,
    presta: Option<Uuid>,
    jours: i32,
) -> Uuid {
    let id = Uuid::new_v4();
    let mut tx = x.c.ent.pool_verif().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_sortie_argent_contournement DISABLE TRIGGER ent_sortie_argent_contournement_horodate_tg")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_sortie_argent_contournement
             (id, etablissement_id, sortie_id, action, demande_par_user_id, motif, cible_id, montant_max_centimes, prestataire_id, demande_le)
         VALUES ($1,$2,$3,$4,$5,'Motif',$6,$7,$8, now() - make_interval(days => $9))",
    )
    .bind(id)
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(action)
    .bind(par)
    .bind(cible)
    .bind(max)
    .bind(presta)
    .bind(jours)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE ent_sortie_argent_contournement ENABLE TRIGGER ent_sortie_argent_contournement_horodate_tg")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

/// Un fait **antidaté** de `jours` jours, garde d'insertion écartée le temps de l'écriture (l'horodatage `le` est alors celui
/// fourni, comme le ferait une base dont l'horloge a avancé depuis).
async fn fait_antidate(
    x: &Suivi,
    cid: Uuid,
    evenement: &str,
    par: Uuid,
    cible: Option<Uuid>,
    jours: i32,
) {
    let mut tx = x.c.ent.pool_verif().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_sortie_argent_contournement_evt DISABLE TRIGGER ent_sortie_argent_contournement_evt_garde_tg")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_sortie_argent_contournement_evt (id, etablissement_id, contournement_id, evenement, par_user_id, cible_id, le)
         VALUES ($1,$2,$3,$4,$5,$6, now() - make_interval(days => $7))",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(cid)
    .bind(evenement.to_string())
    .bind(par)
    .bind(cible)
    .bind(jours)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE ent_sortie_argent_contournement_evt ENABLE TRIGGER ent_sortie_argent_contournement_evt_garde_tg")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

/// Fait inséré par la voie normale (garde active).
fn fait(
    x: &Suivi,
    cid: Uuid,
    evenement: &str,
    par: Uuid,
    cible: Option<Uuid>,
) -> sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments> {
    sqlx::query(
        "INSERT INTO ent_sortie_argent_contournement_evt (id, etablissement_id, contournement_id, evenement, par_user_id, cible_id)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(x.c.etab)
    .bind(cid)
    .bind(evenement.to_string())
    .bind(par)
    .bind(cible)
}

fn nouvel_acompte_sql(
    x: &Suivi,
    id: Uuid,
    presta: Uuid,
    centimes: i64,
    jour: chrono::NaiveDate,
    par: Uuid,
) -> sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments> {
    sqlx::query(
        "INSERT INTO ent_sortie_acompte_fournisseur (id, etablissement_id, sortie_id, prestataire_id, libelle, montant_centimes, echeance, cree_par_user_id)
         VALUES ($1,$2,$3,$4,'x',$5,$6,$7)",
    )
    .bind(id)
    .bind(x.c.etab)
    .bind(x.s.id)
    .bind(presta)
    .bind(centimes)
    .bind(jour)
    .bind(par)
}

/// M3 : l'expiration se joue **à la décision** et **à l'usage** par les déclencheurs, sur l'horloge de la base (faits
/// antidatés, déclencheur d'horodatage écarté dans une transaction de test : aucune heure réelle dans les assertions).
#[tokio::test]
async fn v3b2_l_expiration_se_joue_a_la_decision_et_a_l_usage_par_les_declencheurs_sur_l_horloge_de_la_base(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    // Décision : une demande de 31 jours ne se décide plus (déclencheur), une de 29 jours oui.
    let vieille = demande_antidatee(
        &x,
        lie.user_id,
        "annulation_collective",
        None,
        None,
        None,
        31,
    )
    .await;
    let r = fait(&x, vieille, "autorise", libre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "décision d'une demande de 31 jours"
    );
    assert!(message_sql(&r).contains("expiré"), "{}", message_sql(&r));
    let recente = demande_antidatee(
        &x,
        lie.user_id,
        "annulation_collective",
        None,
        None,
        None,
        29,
    )
    .await;
    fait(&x, recente, "autorise", libre.user_id, None)
        .execute(ent.pool_verif())
        .await
        .expect("décision d'une demande de 29 jours");
    // Usage, garde du fait « utilisé » : autorisation de 31 jours refusée, de 29 jours acceptée.
    let u31 = demande_antidatee(
        &x,
        lie.user_id,
        "annulation_collective",
        None,
        None,
        None,
        31,
    )
    .await;
    fait_antidate(&x, u31, "autorise", libre.user_id, None, 31).await;
    let r = fait(&x, u31, "utilise", lie.user_id, Some(x.s.id))
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "usage d'une autorisation de 31 jours"
    );
    assert!(message_sql(&r).contains("expiré"), "{}", message_sql(&r));
    let u29 = demande_antidatee(
        &x,
        lie.user_id,
        "annulation_collective",
        None,
        None,
        None,
        29,
    )
    .await;
    fait_antidate(&x, u29, "autorise", libre.user_id, None, 29).await;
    fait(&x, u29, "utilise", lie.user_id, Some(x.s.id))
        .execute(ent.pool_verif())
        .await
        .expect("usage d'une autorisation de 29 jours");
    // Usage, déclencheur d'écriture : un « utilisé » dont l'autorisation a vieilli ne couvre plus l'écriture (acompte).
    // 31 jours : refusée ; 29 jours : acceptée (faits posés avec la garde écartée, pour isoler le déclencheur de pouvoir).
    for (jours, attendu) in [(31, false), (29, true)] {
        let cid = demande_antidatee(
            &x,
            lie.user_id,
            "acompte_creer",
            None,
            Some(10_000),
            Some(presta),
            jours,
        )
        .await;
        let nid = Uuid::new_v4();
        fait_antidate(&x, cid, "autorise", libre.user_id, None, jours).await;
        fait_antidate(&x, cid, "utilise", lie.user_id, Some(nid), jours).await;
        let r = nouvel_acompte_sql(&x, nid, presta, 10_000, jour, lie.user_id)
            .execute(ent.pool_verif())
            .await;
        if attendu {
            r.expect("autorisation de 29 jours : l'écriture passe");
        } else {
            assert_eq!(
                code_sql(&r).as_deref(),
                Some("23514"),
                "autorisation de 31 jours : l'écriture est refusée"
            );
        }
    }
}

/// M3 : `demande_le` et `le` sont en ajout seul : UPDATE et DELETE refusés en SQL direct (les deux tables).
#[tokio::test]
async fn v3b2_les_horodatages_de_contournement_ne_se_modifient_ni_ne_se_suppriment_en_sql_direct() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let c = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_collective("Motif"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&x.compta(), x.s.id, c.id, true, None, t0)
        .await
        .unwrap();
    for (nom, sql) in [
        ("UPDATE demande_le", "UPDATE ent_sortie_argent_contournement SET demande_le = now() - interval '40 days' WHERE id = $1"),
        ("DELETE demande", "DELETE FROM ent_sortie_argent_contournement WHERE id = $1"),
        ("UPDATE le", "UPDATE ent_sortie_argent_contournement_evt SET le = now() - interval '40 days' WHERE contournement_id = $1"),
        ("DELETE fait", "DELETE FROM ent_sortie_argent_contournement_evt WHERE contournement_id = $1"),
    ] {
        let r = sqlx::query(sql).bind(c.id).execute(ent.pool_verif()).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "{nom} : ajout seul");
    }
    let (d, n): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_sortie_argent_contournement WHERE id = $1),
                (SELECT count(*) FROM ent_sortie_argent_contournement_evt WHERE contournement_id = $1)",
    )
    .bind(c.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((d, n), (1, 1), "rien n'a bougé");
}

/// M3 : `portee_chk` en SQL direct (plafond, prestataire ou cible NULL ou en trop, selon l'action).
#[tokio::test]
async fn v3b2_la_contrainte_de_portee_refuse_en_sql_direct_plafond_prestataire_ou_cible_manquants_ou_en_trop(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let ac = ent
        .voyage_acompte_creer(&x.compta(), x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let inserer = |action: &'static str, cible: Option<Uuid>, max: Option<i64>, p: Option<Uuid>| {
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement
                 (id, etablissement_id, sortie_id, action, demande_par_user_id, motif, cible_id, montant_max_centimes, prestataire_id)
             VALUES ($1,$2,$3,$4,$5,'Motif',$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(action)
        .bind(lie.user_id)
        .bind(cible)
        .bind(max)
        .bind(p)
    };
    let (a, m, p) = (Some(ac.id), Some(10_000_i64), Some(presta));
    for (nom, action, cible, max, pr) in [
        ("créer sans plafond", "acompte_creer", None, None, p),
        ("créer sans prestataire", "acompte_creer", None, m, None),
        ("créer avec une cible", "acompte_creer", a, m, p),
        ("payer sans cible", "acompte_payer", None, m, p),
        ("payer sans plafond", "acompte_payer", a, None, p),
        ("payer sans prestataire", "acompte_payer", a, m, None),
        ("annuler sans cible", "acompte_annuler", None, None, None),
        ("annuler avec un plafond", "acompte_annuler", a, m, None),
        ("annuler avec un prestataire", "acompte_annuler", a, None, p),
        (
            "collective avec une cible",
            "annulation_collective",
            a,
            None,
            None,
        ),
        (
            "collective avec un plafond",
            "annulation_collective",
            None,
            m,
            None,
        ),
        (
            "collective avec un prestataire",
            "annulation_collective",
            None,
            None,
            p,
        ),
        ("plafond nul", "acompte_creer", None, Some(0), p),
    ] {
        let r = inserer(action, cible, max, pr)
            .execute(ent.pool_verif())
            .await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "{nom}");
    }
    for (nom, action, cible, max, pr) in [
        ("créer", "acompte_creer", None, m, p),
        ("payer", "acompte_payer", a, m, p),
        ("annuler", "acompte_annuler", a, None, None),
        ("collective", "annulation_collective", None, None, None),
    ] {
        inserer(action, cible, max, pr)
            .execute(ent.pool_verif())
            .await
            .unwrap_or_else(|e| panic!("{nom} bien formée : {e}"));
    }
}

/// M3 : annulation d'acompte hors cible en SQL direct ; autrice réellement membre d'un autre établissement.
#[tokio::test]
async fn v3b2_annulation_d_acompte_hors_cible_en_sql_direct_et_autrice_membre_d_un_autre_etablissement(
) {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    let ac_a = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let ac_b = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let c = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_a.id, "Doublon"), t0)
        .await
        .unwrap();
    // Autrice membre d'un AUTRE établissement (comptable réelle de cet autre établissement) : la décision est refusée.
    let Some(autre) = crate::tests_voyages_v0d::cadre().await else {
        return;
    };
    let r = fait(&x, c.id, "autorise", autre.compta, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "autrice d'un autre établissement"
    );
    assert!(
        message_sql(&r).contains("membre habilité"),
        "{}",
        message_sql(&r)
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_argent_contournement_evt WHERE contournement_id = $1",
    )
    .bind(c.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0, "aucun fait n'a été écrit");
    ent.voyage_contournement_decider_a(&libre, x.s.id, c.id, true, None, t0)
        .await
        .unwrap();
    fait(&x, c.id, "utilise", lie.user_id, Some(ac_a.id))
        .execute(ent.pool_verif())
        .await
        .expect("utilisé pour A");
    // SQL direct : annuler B (hors cible) refusé ; annuler A passe.
    let annuler = |acompte: Uuid| {
        sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur SET annule_le = now(), annule_motif = 'Doublon', annule_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(x.c.etab)
        .bind(acompte)
        .bind(lie.user_id)
    };
    let r = annuler(ac_b.id).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "annuler un autre acompte que la cible"
    );
    annuler(ac_a.id)
        .execute(ent.pool_verif())
        .await
        .expect("annuler la cible");
}

/// Faible B1, à l'usage : une autorisation donnée PUIS dont l'autrice est suspendue ne sert plus (fait « utilisé » et
/// écriture), tant que l'accès n'est pas rétabli.
#[tokio::test]
async fn v3b2_une_autorisation_dont_l_autrice_est_suspendue_apres_coup_ne_sert_plus() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    let ac_1 = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let ac_2 = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let etat = |user: Uuid, etat: &'static str| {
        sqlx::query(
            "UPDATE ent_membership SET etat = $3 WHERE user_id = $1 AND etablissement_id = $2",
        )
        .bind(user)
        .bind(x.c.etab)
        .bind(etat)
        .execute(ent.pool_verif())
    };
    // (1) Suspendue AVANT l'usage : le fait « utilisé » est refusé.
    let aut1 = compta_libre(&x).await;
    let c1 = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_1.id, "Doublon 1"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&aut1, x.s.id, c1.id, true, None, t0)
        .await
        .unwrap();
    etat(aut1.user_id, "suspendu").await.unwrap();
    let r = fait(&x, c1.id, "utilise", lie.user_id, Some(ac_1.id))
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "autrice suspendue : l'usage est refusé"
    );
    etat(aut1.user_id, "ouvert").await.unwrap();
    fait(&x, c1.id, "utilise", lie.user_id, Some(ac_1.id))
        .execute(ent.pool_verif())
        .await
        .expect("accès rétabli : l'usage passe");
    // (2) Suspendue APRÈS l'usage, avant l'écriture : l'écriture est refusée, puis passe une fois l'accès rétabli.
    let aut2 = compta_libre(&x).await;
    let c2 = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_2.id, "Doublon 2"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&aut2, x.s.id, c2.id, true, None, t0)
        .await
        .unwrap();
    fait(&x, c2.id, "utilise", lie.user_id, Some(ac_2.id))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    etat(aut2.user_id, "suspendu").await.unwrap();
    let annuler = |acompte: Uuid| {
        sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur SET annule_le = now(), annule_motif = 'Doublon', annule_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(x.c.etab)
        .bind(acompte)
        .bind(lie.user_id)
    };
    let r = annuler(ac_2.id).execute(ent.pool_verif()).await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "autrice suspendue : l'écriture est refusée"
    );
    etat(aut2.user_id, "ouvert").await.unwrap();
    annuler(ac_2.id)
        .execute(ent.pool_verif())
        .await
        .expect("accès rétabli : l'écriture passe");
}

/// Faible 5 : l'autrice d'une autorisation la révoque (ajout seul « revoque ») tant qu'elle n'a pas servi.
#[tokio::test]
async fn v3b2_l_autrice_revoque_son_autorisation_tant_qu_elle_n_a_pas_servi() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    let autre = compta_libre(&x).await;
    let ac_r = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let ac_u = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    // Une demande non autorisée ne se révoque pas.
    let c = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_r.id, "Doublon"), t0)
        .await
        .unwrap();
    let e = ent
        .voyage_contournement_revoquer_a(&libre, x.s.id, c.id, t0)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "non autorisée : {e:?}");
    let r = fait(&x, c.id, "revoque", libre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "base : non autorisée"
    );
    ent.voyage_contournement_decider_a(&libre, x.s.id, c.id, true, None, t0)
        .await
        .unwrap();
    // Une autre personne, même habilitée et libre, ne révoque pas ; la demandeuse non plus (service et base).
    let e = ent
        .voyage_contournement_revoquer_a(&autre, x.s.id, c.id, t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "autre personne : {e:?}");
    let e = ent
        .voyage_contournement_revoquer_a(&lie, x.s.id, c.id, t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "demandeuse : {e:?}");
    for (nom, par) in [
        ("autre personne", autre.user_id),
        ("demandeuse", lie.user_id),
    ] {
        let r = fait(&x, c.id, "revoque", par, None)
            .execute(ent.pool_verif())
            .await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "base : {nom}");
    }
    // L'autrice révoque : statut « revoque », une seule fois ; l'autorisation ne sert plus (fait « utilisé » refusé).
    let v = ent
        .voyage_contournement_revoquer_a(&libre, x.s.id, c.id, t0)
        .await
        .unwrap();
    assert_eq!(v.statut, "revoque");
    let e = ent
        .voyage_contournement_revoquer_a(&libre, x.s.id, c.id, t0)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "déjà révoquée : {e:?}");
    let r = fait(&x, c.id, "revoque", libre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23505"),
        "une seule révocation"
    );
    let r = fait(&x, c.id, "utilise", lie.user_id, Some(ac_r.id))
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "révoquée : ne s'utilise pas"
    );
    assert!(message_sql(&r).contains("révoqué"), "{}", message_sql(&r));
    let e = ent
        .voyage_acompte_annuler_avec(&lie, x.s.id, ac_r.id, "Doublon", Some(c.id), t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "service : révoquée : {e:?}");
    // Une autorisation utilisée ne se révoque plus ; une autorisation révoquée après usage n'est pas possible.
    let cu = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac_u.id, "Doublon U"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre, x.s.id, cu.id, true, None, t0)
        .await
        .unwrap();
    fait(&x, cu.id, "utilise", lie.user_id, Some(ac_u.id))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let e = ent
        .voyage_contournement_revoquer_a(&libre, x.s.id, cu.id, t0)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "déjà utilisée : {e:?}");
    let r = fait(&x, cu.id, "revoque", libre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "base : déjà utilisée"
    );
    // Une autorisation révoquée n'ouvre plus rien : fonction de validité fausse.
    let ouvert: bool = sqlx::query_scalar(
        "SELECT ent_sortie_contournement_ouvert($1,$2,$3,$4,'acompte_annuler',$5,NULL,NULL, now())",
    )
    .bind(x.c.etab)
    .bind(c.id)
    .bind(x.s.id)
    .bind(lie.user_id)
    .bind(ac_r.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!ouvert, "révoquée : jamais ouverte");
}

/// Moyen 2 (revue) : un avoir collectif dont le couple (participant, compte) n'a rien versé, ou dont `paye_net` dépasse
/// l'argent versé, est refusé en base. Faible 1 : la garde porte sur le participant et le payeur de la ligne, et la décision
/// portée par la ligne est contrôlée même dans la branche collective.
#[tokio::test]
async fn v3b2_un_avoir_collectif_ne_depasse_pas_l_argent_verse_et_sa_decision_est_gardee() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let libre = x.compta();
    let libre2 = compta_libre(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let payeur_c = compta_payeur_de(&x, x.compte_c).await;
    x.verser(x.p1, x.compte_c, "cheque", "50").await.unwrap();
    let avoir = |part: Uuid, compte: Uuid, net: i64, par: Uuid, decision: Option<Uuid>| {
        sqlx::query(
            "INSERT INTO ent_sortie_avoir_propose (id, etablissement_id, sortie_id, participant_id, compte_id, motif_code, cle,
                    paye_net_centimes, montant_propose_centimes, propose_par_user_id,
                    statut, decide_par_user_id, decide_le, montant_retenu_centimes)
             VALUES ($1,$2,$3,$4,$5,'annulation_collective',$6,$7,$7,$8,
                     CASE WHEN $9::uuid IS NULL THEN 'propose' ELSE 'retenu' END, $9::uuid,
                     CASE WHEN $9::uuid IS NULL THEN NULL ELSE now() END,
                     CASE WHEN $9::uuid IS NULL THEN NULL ELSE 1000 END)",
        )
        .bind(Uuid::new_v4())
        .bind(x.c.etab)
        .bind(x.s.id)
        .bind(part)
        .bind(compte)
        .bind(format!("directe-{}", Uuid::new_v4().simple()))
        .bind(net)
        .bind(par)
        .bind(decision)
    };
    for (nom, part, compte, net) in [
        ("au-delà de l'argent versé", x.p1, x.compte_c, 5_001),
        ("montant nul", x.p1, x.compte_c, 0),
        ("participant sans versement", x.p0, x.compte_a, 100),
        (
            "payeur sans versement pour ce participant",
            x.p1,
            x.compte_a,
            100,
        ),
    ] {
        let r = avoir(part, compte, net, libre.user_id, None)
            .execute(ent.pool_verif())
            .await;
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "{nom}");
        assert!(
            message_sql(&r).contains("ne dépasse pas l'argent versé"),
            "{nom} : {}",
            message_sql(&r)
        );
    }
    avoir(x.p1, x.compte_c, 5_000, libre.user_id, None)
        .execute(ent.pool_verif())
        .await
        .expect("l'argent versé, ni plus ni moins");
    // Contre-revue du pilote (M-1) : la borne vaut pour le cumul : un second avoir pour le même payeur et le même
    // participant est refusé, même inférieur au versé.
    let r = avoir(x.p1, x.compte_c, 3_000, libre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(code_sql(&r).as_deref(), Some("23514"), "cumul au-delà du versé");
    assert!(message_sql(&r).contains("déjà couvert"), "{}", message_sql(&r));
    // Faible 1 : la décision portée par la ligne est contrôlée (parent de l'élève, payeur du compte : refusés ; libre : ok).
    for (nom, qui) in [
        ("parent de l'élève", lie.user_id),
        ("payeur du compte", payeur_c.user_id),
    ] {
        let r = avoir(x.p1, x.compte_c, 2_000, libre.user_id, Some(qui))
            .execute(ent.pool_verif())
            .await;
        assert_eq!(
            code_sql(&r).as_deref(),
            Some("23514"),
            "décision par le {nom}"
        );
    }
    // Un autre couple (participant, payeur) encore non couvert : les gardes de personne restent discriminantes.
    x.verser(x.p0, x.compte_a, "cheque", "50").await.unwrap();
    let payeur_a = compta_payeur_de(&x, x.compte_a).await;
    let lie0 = compta_parent_de(&x, x.c.eleves[0]).await;
    // La proposition elle-même reste gardée sur le participant et le payeur de la ligne.
    let r = avoir(x.p0, x.compte_a, 1_000, lie0.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "proposée par le parent de l'élève"
    );
    let r = avoir(x.p0, x.compte_a, 1_000, payeur_a.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert_eq!(
        code_sql(&r).as_deref(),
        Some("23514"),
        "proposée par le payeur du compte"
    );
    avoir(x.p0, x.compte_a, 2_000, libre.user_id, Some(libre2.user_id))
        .execute(ent.pool_verif())
        .await
        .expect("décision par une personne libre");
}

/// Moyen 3 (revue) : le rattrapage de 656 restaure `app.etablissement_id` et est idempotent (second passage sans effet).
#[cfg(feature = "labo")]
#[tokio::test]
async fn v3b2_le_rattrapage_restaure_le_reglage_de_l_etablissement_et_se_rejoue_sans_effet() {
    let Some(x) = suivi_v3().await else { return };
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return;
    };
    let (proprietaire, strict) = crate::tests_rls_stricte::proprietaire_et_strict(&url).await;
    let sql = include_str!("../../../migrations/656_sortie_voyages_v3b_suites.sql");
    let debut = sql.find("-- DEBUT-DONNEES").expect("repère de début") + "-- DEBUT-DONNEES".len();
    let fin = sql.find("-- FIN-DONNEES").expect("repère de fin");
    let bloc = &sql[debut..fin];
    // Palier des faibles n° 2 (F-8) : deux autres lignes du même établissement, copies de la sortie : une sortie annulée
    // sans `annulee_le` (rattrapée comme `rentree_le`) et une annulée dont l'instant est déjà posé (jamais modifiée).
    let (annulee_vide, annulee_posee) = (Uuid::new_v4(), Uuid::new_v4());
    let posee_le: DateTime<Utc> = "2030-01-15T08:00:00Z".parse().unwrap();
    let copie = |nid: Uuid, instant: Option<DateTime<Utc>>| {
        format!(
            "INSERT INTO ent_sortie SELECT (jsonb_populate_record(NULL::ent_sortie, to_jsonb(s) || jsonb_build_object(
                 'id', '{nid}', 'reference', 'F8-{nid}', 'etat', 'annulee', 'rentree_le', NULL, 'annulee_le', {}))).*
               FROM ent_sortie s WHERE s.id = '{}'",
            instant.map_or("NULL".to_string(), |t| format!("'{}'", t.to_rfc3339())),
            x.s.id
        )
    };
    for ddl in [
        "ALTER TABLE ent_sortie DISABLE TRIGGER ent_sortie_rentree_le_tg".to_string(),
        "ALTER TABLE ent_sortie DISABLE TRIGGER ent_sortie_annulee_le_tg".to_string(),
        "ALTER TABLE ent_sortie DROP CONSTRAINT IF EXISTS ent_sortie_rentree_le_chk".to_string(),
        "ALTER TABLE ent_sortie DROP CONSTRAINT IF EXISTS ent_sortie_annulee_le_chk".to_string(),
        copie(annulee_vide, None),
        copie(annulee_posee, Some(posee_le)),
        format!(
            "UPDATE ent_sortie SET etat = 'rentree', rentree_le = NULL WHERE id = '{}'",
            x.s.id
        ),
        "ALTER TABLE ent_sortie ENABLE TRIGGER ent_sortie_rentree_le_tg".to_string(),
        "ALTER TABLE ent_sortie ENABLE TRIGGER ent_sortie_annulee_le_tg".to_string(),
    ] {
        sqlx::query(&ddl)
            .execute(proprietaire.pool())
            .await
            .unwrap();
    }
    let modifie_avant: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT modifie_le FROM ent_sortie WHERE id = $1")
            .bind(annulee_vide)
            .fetch_one(proprietaire.pool())
            .await
            .unwrap();
    let sentinelle = Uuid::new_v4().to_string();
    // Réglage précédent posé : restauré à l'identique après le bloc ; la ligne masquée sans contexte est rattrapée.
    let mut tx = strict.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(&sentinelle)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql(bloc)
        .execute(&mut *tx)
        .await
        .expect("rattrapage");
    let apres: Option<String> =
        sqlx::query_scalar("SELECT current_setting('app.etablissement_id', true)")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(
        apres.as_deref(),
        Some(sentinelle.as_str()),
        "réglage précédent restauré"
    );
    tx.commit().await.unwrap();
    let premier: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT rentree_le FROM ent_sortie WHERE id = $1")
            .bind(x.s.id)
            .fetch_one(proprietaire.pool())
            .await
            .unwrap();
    assert!(premier.is_some(), "rentree_le rattrapée");
    // F-8 : l'état de toutes les sorties de l'établissement après le premier passage (instants de rentrée et
    // d'annulation) ; le second passage doit le laisser identique, ligne par ligne.
    let etat_des_sorties = || async {
        sqlx::query_as::<_, (Uuid, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>)>(
            "SELECT id, rentree_le, annulee_le, modifie_le FROM ent_sortie WHERE etablissement_id = $1 ORDER BY id",
        )
        .bind(x.c.etab)
        .fetch_all(proprietaire.pool())
        .await
        .unwrap()
    };
    let apres_premier = etat_des_sorties().await;
    let ligne = |id: Uuid| {
        apres_premier
            .iter()
            .find(|l| l.0 == id)
            .copied()
            .expect("ligne présente")
    };
    assert_eq!(
        ligne(annulee_vide).2,
        modifie_avant,
        "annulee_le rattrapée sur l'instant de modification d'avant le passage"
    );
    assert_eq!(
        ligne(annulee_posee).2,
        Some(posee_le),
        "un instant d'annulation déjà posé n'est jamais modifié"
    );
    // Sans réglage préalable : rien ne reste posé après le bloc (vide ou absent).
    let mut tx = strict.pool().begin().await.unwrap();
    sqlx::raw_sql(bloc)
        .execute(&mut *tx)
        .await
        .expect("second passage");
    let apres: Option<String> =
        sqlx::query_scalar("SELECT current_setting('app.etablissement_id', true)")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(
        apres.unwrap_or_default().is_empty(),
        "aucun établissement ne reste posé"
    );
    tx.commit().await.unwrap();
    // Idempotence : le second passage ne change rien (même instant, aucune ligne de plus touchée).
    let second: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT rentree_le FROM ent_sortie WHERE id = $1")
            .bind(x.s.id)
            .fetch_one(proprietaire.pool())
            .await
            .unwrap();
    assert_eq!(premier, second, "second passage sans effet");
    assert_eq!(
        etat_des_sorties().await,
        apres_premier,
        "F-8 : second passage sans effet sur aucune ligne (rentrée, annulation, modification)"
    );
    // Contre-revue du pilote (M-4) : ne remet QUE la contrainte écartée, sans rejouer 656 ni 657 sur la base commune (le
    // rejeu de 656 remettait un instant les fonctions d'avant 657 sous les tests voisins).
    sqlx::query(
        "ALTER TABLE ent_sortie ADD CONSTRAINT ent_sortie_rentree_le_chk
            CHECK ((etat <> 'rentree' OR rentree_le IS NOT NULL) AND (rentree_le IS NULL OR etat IN ('rentree', 'close')))",
    )
    .execute(proprietaire.pool())
    .await
    .expect("contrainte rétablie");
    sqlx::query(
        "ALTER TABLE ent_sortie ADD CONSTRAINT ent_sortie_annulee_le_chk CHECK ((etat = 'annulee') = (annulee_le IS NOT NULL))",
    )
    .execute(proprietaire.pool())
    .await
    .expect("contrainte d'annulation rétablie");
    crate::tenant_tx::forcer_mode_test(None);
}

// ───────────────────────── Palier des faibles regroupés, lot 2 (contre-revue V3b suites 2) ─────────────────────────

/// M-2 (658) : en SQL direct, une révocation et un usage concurrents d'une même autorisation ne passent plus tous les deux.
/// La garde des faits prend le verrou de la sortie : la seconde transaction attend la première, puis lit son fait.
#[tokio::test]
async fn faibles_l2_m2_revocation_et_usage_concurrents_en_sql_direct_un_seul_passe() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    for (premier, second, attendu) in [
        ("revoque", "utilise", "révoqué"),
        ("utilise", "revoque", "déjà utilisé"),
    ] {
        let ac = ent
            .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
            .await
            .unwrap();
        let c = ent
            .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac.id, "Course"), t0)
            .await
            .unwrap();
        ent.voyage_contournement_decider_a(&libre, x.s.id, c.id, true, None, t0)
            .await
            .unwrap();
        let qui = |ev: &str| if ev == "revoque" { libre.user_id } else { lie.user_id };
        let cible = |ev: &str| (ev == "utilise").then_some(ac.id);
        let mut t1 = ent.pool_verif().begin().await.unwrap();
        fait(&x, c.id, premier, qui(premier), cible(premier))
            .execute(&mut *t1)
            .await
            .unwrap_or_else(|e| panic!("{premier} : {e}"));
        // La seconde transaction démarre pendant que la première tient le verrou de la sortie. Palier des faibles n° 2 :
        // plus de délai fixe ; on attend que la connexion de la seconde soit **en attente d'un verrou** (catalogue
        // `pg_stat_activity`, connexion identifiée par son pid), ce qui prouve le blocage sans dépendre de la machine.
        let mut c2 = ent.pool_verif().acquire().await.unwrap().detach();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut c2)
            .await
            .unwrap();
        let q = fait(&x, c.id, second, qui(second), cible(second));
        let h = tokio::spawn(async move {
            let r = q.execute(&mut c2).await;
            drop(c2);
            r
        });
        let mut bloquee = false;
        for _ in 0..400 {
            let attente: Option<String> = sqlx::query_scalar(
                "SELECT wait_event_type FROM pg_stat_activity WHERE pid = $1",
            )
            .bind(pid)
            .fetch_optional(ent.pool_verif())
            .await
            .unwrap()
            .flatten();
            if attente.as_deref() == Some("Lock") {
                bloquee = true;
                break;
            }
            assert!(
                !h.is_finished(),
                "{second} a fini sans attendre le verrou de la sortie tenu par {premier}"
            );
            tokio::task::yield_now().await;
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(
            bloquee && !h.is_finished(),
            "{second} attend le verrou de la sortie tenu par {premier}"
        );
        t1.commit().await.unwrap();
        let r = h.await.unwrap();
        assert_eq!(code_sql(&r).as_deref(), Some("23514"), "{premier} puis {second}");
        assert!(message_sql(&r).contains(attendu), "{}", message_sql(&r));
    }
}

/// F-4 : une demande expirée qui n'a jamais été autorisée n'a rien à révoquer : conflit d'état (409), pas un refus de
/// personne (403). F-6 : le refus de révocation par une autre personne que l'autrice est tracé.
#[tokio::test]
async fn faibles_l2_f4_f6_revocation_d_une_demande_expiree_et_refus_trace() {
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    let autre = compta_libre(&x).await;
    let ac = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let c = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac.id, "Expirée"), t0)
        .await
        .unwrap();
    for qui in [&libre, &autre] {
        let e = ent
            .voyage_contournement_revoquer_a(qui, x.s.id, c.id, plus(t0, 31))
            .await
            .unwrap_err();
        assert!(
            matches!(e, EntDbError::Domain(domain_eleve::DomainError::Conflit)),
            "expirée sans autorisation : conflit, pas un refus de personne : {e:?}"
        );
    }
    // Une autorisation : une autre personne que l'autrice ne révoque pas, et son refus laisse une trace.
    let ac2 = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let c2 = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac2.id, "Autorisée"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre, x.s.id, c2.id, true, None, t0)
        .await
        .unwrap();
    let avant = nb_refus(&x, autre.user_id, "voyage_contournement_revoquer_a").await;
    let e = ent
        .voyage_contournement_revoquer_a(&autre, x.s.id, c2.id, t0)
        .await
        .unwrap_err();
    assert!(est_interdit_metier(&e), "{e:?}");
    assert_eq!(
        nb_refus(&x, autre.user_id, "voyage_contournement_revoquer_a").await,
        avant + 1,
        "refus de révocation tracé"
    );
}

/// F-3 et F-5 : les refus de cycle de vie d'un contournement venus de la base (course) sont des conflits d'état, pas des
/// interdictions de la règle 3 ; la révocation par une autre personne reste une interdiction ; un refus de portée
/// (« ne couvre pas cette cible ») reste celui de la règle 3.
#[tokio::test]
async fn faibles_l2_f3_f5_refus_de_la_base_traduits_en_refus_metier() {
    use crate::voyages_argent_v3::{db_pouvoir, est_refus_regle3};
    let Some(x) = suivi_v3().await else { return };
    x.sync().await;
    let ent = &x.c.ent;
    let t0 = instant_base(&x).await;
    let jour = x.c.jour_etab().await;
    let presta = un_prestataire(&x).await;
    let lie = compta_parent_de(&x, x.c.eleves[1]).await;
    let libre = x.compta();
    let autre = compta_libre(&x).await;
    let ac = ent
        .voyage_acompte_creer(&libre, x.s.id, acompte_de(presta, jour))
        .await
        .unwrap();
    let c = ent
        .voyage_contournement_demander_a(&lie, x.s.id, dem_annuler(ac.id, "Traduction"), t0)
        .await
        .unwrap();
    ent.voyage_contournement_decider_a(&libre, x.s.id, c.id, true, None, t0)
        .await
        .unwrap();
    // Portée : une autre cible reste un refus de la règle 3.
    let r = fait(&x, c.id, "utilise", lie.user_id, Some(Uuid::new_v4()))
        .execute(ent.pool_verif())
        .await;
    assert!(est_refus_regle3(&db_pouvoir(r.unwrap_err())));
    // Révocation par une autre personne : interdiction (tracée par le service).
    let r = fait(&x, c.id, "revoque", autre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    let e = db_pouvoir(r.unwrap_err());
    assert!(est_interdit_metier(&e) && !est_refus_regle3(&e), "{e:?}");
    // Révoquée, puis utilisée : conflit d'état.
    fait(&x, c.id, "revoque", libre.user_id, None)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let r = fait(&x, c.id, "utilise", lie.user_id, Some(ac.id))
        .execute(ent.pool_verif())
        .await;
    let e = db_pouvoir(r.unwrap_err());
    assert!(
        matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)),
        "révoquée puis utilisée : {e:?}"
    );
    // Une seconde révocation (index d'unicité) reste un conflit ordinaire.
    let r = fait(&x, c.id, "revoque", libre.user_id, None)
        .execute(ent.pool_verif())
        .await;
    assert!(matches!(
        db_pouvoir(r.unwrap_err()),
        EntDbError::Domain(domain_eleve::DomainError::Conflit)
    ));
}
