//! Socle 42, lot V3 « argent relié » (`docs/socles/42_SORTIES_VOYAGES.md` §12.4, §12.5, §6.2 ; VOY-224, VOY-286 ;
//! D-068 : la comptabilité des voyages est épaissie et reliée au reste de l'ENT, pas refondue).
//!
//! Trois liens, tous dans la continuité du mode suivi de V2a (R-SV44, D-034 : aucune facture, aucun avoir comptable) :
//!
//! * **aides** (APEL, fonds social, collectivité, entreprise, don affecté) appliquées à la part d'**un** payeur : elles
//!   réduisent ce qui reste dû, elles ne sont **jamais** de l'argent versé par la famille — un avoir ne les rend donc
//!   pas au payeur (CF-69) ; une aide d'un participant parti est signalée « à restituer ou réaffecter » ;
//! * **reports** de versement d'un participant à un autre **du même payeur** (frère ou sœur, enfant remplacé depuis la
//!   liste d'attente) : l'argent ne change jamais de payeur ; l'avoir du partant se recalcule sur ce qu'il lui reste ;
//! * **internat** (socle 41) : un interne confirmé qui part en séjour est retiré de l'attendu des nuits concernées
//!   (motif « séjour scolaire »), et rétabli s'il se désiste ; aucune ressaisie.
//!
//! Tout mouvement d'argent est inscrit en **ajout seul** (annulation par écriture inverse motivée, vérifiée par la
//! base) ; les montants sont des centimes entiers venus d'un texte (`centimes_de_texte`), jamais d'un nombre du
//! navigateur ; aucun montant ni nom ne part dans l'outbox ; seule la comptabilité (`voyages.argent`) voit le détail.

use chrono::{DateTime, NaiveDate, Utc};
use ent_events::{types, AggregateKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::outbox::Causalite;
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::voyages_argent::{centimes_de_texte, VERSEMENT_MAX};
use crate::voyages_argent_repo::{
    aides_nettes, paye_net, peut_argent, recalculer_depart, synchroniser, verrou_sortie, ETATS_PAYANTS,
};
use crate::voyages_repo::{charger_avec_acces, interdit, journaliser, refus, texte, texte_opt, tx, Fait, SortieLigne};

/// Sources d'une aide (codes ; le libellé est l'affaire de l'écran).
pub const SOURCES_AIDE: [&str; 6] = ["apel", "fonds_social", "collectivite", "entreprise", "don_affecte", "autre"];

/// États d'un participant qui part effectivement : seuls ceux-là sont retirés de l'attendu de l'internat.
pub(crate) const ETATS_PARTANTS: [&str; 2] = ["pret", "parti"];

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AideSaisie {
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub source: String,
    /// Texte au format français (`80,00`) : jamais un nombre venu du navigateur.
    pub montant: String,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AideVue {
    pub id: Uuid,
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub sens: String,
    pub source: String,
    pub montant_centimes: i64,
    pub reference: Option<String>,
    pub annule_id: Option<Uuid>,
    pub motif: Option<String>,
    pub decide_le: DateTime<Utc>,
    /// Aide encore active d'un participant qui n'est plus inscrit : à restituer à sa source ou à réaffecter.
    pub a_restituer: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportSaisie {
    pub de_participant_id: Uuid,
    pub vers_participant_id: Uuid,
    pub compte_id: Uuid,
    pub montant: String,
    pub motif: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ReportVue {
    pub id: Uuid,
    pub de_participant_id: Uuid,
    pub vers_participant_id: Uuid,
    pub compte_id: Uuid,
    pub sens: String,
    pub montant_centimes: i64,
    pub annule_id: Option<Uuid>,
    pub motif: String,
    pub decide_le: DateTime<Utc>,
}

/// Résultat de la synchronisation avec l'internat : des nombres, aucun nom.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InternatSynchro {
    pub poses: usize,
    pub annules: usize,
    pub inchanges: usize,
    /// Nuits passées hors de l'établissement : 0 pour un séjour d'une seule journée (rien à retirer).
    pub nuits: i64,
}

// ───────────────────────── Aides internes ─────────────────────────

/// Part courante (centimes) de ce payeur pour ce participant ; `None` si ce payeur n'en porte aucune.
async fn part_courante(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
    compte: Uuid,
) -> Result<Option<i64>, EntDbError> {
    sqlx::query_scalar(
        "SELECT montant_centimes FROM ent_sortie_part
          WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
          ORDER BY prix_rang DESC LIMIT 1",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .bind(compte)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// État du participant de cette sortie, ligne verrouillée.
async fn etat_participant(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
) -> Result<String, EntDbError> {
    sqlx::query_scalar(
        "SELECT etat FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn lire_aide(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<AideVue, EntDbError> {
    sqlx::query_as(
        "SELECT x.id, x.participant_id, x.compte_id, x.sens, x.source, x.montant_centimes, x.reference, x.annule_id,
                x.motif, x.decide_le,
                (x.sens = 'aide'
                 AND NOT EXISTS (SELECT 1 FROM ent_sortie_aide y WHERE y.etablissement_id = x.etablissement_id AND y.annule_id = x.id)
                 AND NOT EXISTS (SELECT 1 FROM ent_sortie_participant p
                                  WHERE p.etablissement_id = x.etablissement_id AND p.id = x.participant_id
                                    AND p.etat = ANY($3))) AS a_restituer
           FROM ent_sortie_aide x WHERE x.etablissement_id = $1 AND x.id = $2",
    )
    .bind(etab)
    .bind(id)
    .bind(ETATS_PAYANTS.iter().map(|e| (*e).to_string()).collect::<Vec<_>>())
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn lire_report(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<ReportVue, EntDbError> {
    sqlx::query_as(
        "SELECT id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, annule_id, motif, decide_le
           FROM ent_sortie_report WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

/// Le participant est-il parti de la sortie (désisté ou retiré) ?
fn est_parti(etat: &str) -> bool {
    matches!(etat, "desiste" | "retire")
}

/// Libère l'internat d'un participant qui se désiste ou est retiré : ses retraits de séjour sont annulés (ils restent
/// dans le socle 41, datés et signés, jamais supprimés). Appelé dans la transaction du changement d'état.
pub(crate) async fn internat_liberer_participant(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    participant: Uuid,
) -> Result<usize, EntDbError> {
    let liens: Vec<(Uuid, Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT r.id, p.eleve_id, r.du_nuit, r.au_nuit
           FROM ent_sortie_internat_retrait l
           JOIN ent_internat_retrait r ON r.etablissement_id = l.etablissement_id AND r.id = l.retrait_id
           JOIN ent_sortie_participant p ON p.etablissement_id = l.etablissement_id AND p.id = l.participant_id
          WHERE l.etablissement_id = $1 AND l.participant_id = $2 AND r.annule_le IS NULL",
    )
    .bind(a.etablissement_id)
    .bind(participant)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (retrait, eleve, du, au) in &liens {
        annuler_retrait(conn, a, *retrait, *eleve, *du, *au).await?;
    }
    Ok(liens.len())
}

async fn annuler_retrait(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    retrait: Uuid,
    eleve: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<(), EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_internat_retrait SET annule_le = now(), annule_par_user_id = $3
          WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL",
    )
    .bind(a.etablissement_id)
    .bind(retrait)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() == 1 {
        crate::finances::publier(
            conn,
            a.etablissement_id,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            retrait,
            Some(eleve),
            json!({ "du": du.to_string(), "au": au.to_string(), "cause": "retrait_annule", "motif_code": "sejour_scolaire" }),
            Some(a.user_id),
            Causalite::default(),
        )
        .await?;
    }
    Ok(())
}

impl PgEnt {
    // ───────────────────────── Aides (VOY-224) ─────────────────────────

    /// Inscrit une aide sur la part d'**un** payeur d'un participant. Comptabilité et direction (`voyages.argent`).
    /// L'aide ne dépasse jamais ce qui reste dû sur cette part (versé et aides déjà inscrites déduits).
    pub async fn voyage_aide_accorder(&self, a: &Acteur<'_>, id: Uuid, v: AideSaisie) -> Result<AideVue, EntDbError> {
        if !SOURCES_AIDE.contains(&v.source.as_str()) {
            return Err(invalide());
        }
        let montant = centimes_de_texte(&v.montant).filter(|m| (1..=VERSEMENT_MAX).contains(m)).ok_or_else(invalide)?;
        let reference = texte_opt(v.reference.as_deref(), 40)?;
        let mut t = tx(self, a).await?;
        // M1 : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        if s.nature == "obligatoire" {
            return Err(refus(RefusMetier::Conflit, "Sortie obligatoire : rien n'est dû par les familles, aucune aide à inscrire."));
        }
        synchroniser(&mut t, a, &s).await?;
        let etat = etat_participant(&mut t, a.etablissement_id, s.id, v.participant_id).await?;
        if !ETATS_PAYANTS.contains(&etat.as_str()) {
            return Err(refus(RefusMetier::Conflit, "Ce participant n'est plus inscrit : aucune aide ne s'inscrit."));
        }
        let part = part_courante(&mut t, a.etablissement_id, s.id, v.participant_id, v.compte_id)
            .await?
            .ok_or_else(introuvable)?;
        let verse = paye_net(&mut t, a.etablissement_id, s.id, v.participant_id, v.compte_id).await?;
        let aides = aides_nettes(&mut t, a.etablissement_id, s.id)
            .await?
            .get(&(v.participant_id, v.compte_id))
            .copied()
            .unwrap_or(0);
        let couvert = verse.checked_add(aides).and_then(|x| x.checked_add(montant)).ok_or_else(invalide)?;
        if couvert > part {
            return Err(refus(RefusMetier::Conflit, "Cette aide dépasse ce qui reste dû sur la part de ce payeur."));
        }
        let aid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_aide
                 (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, reference,
                  decide_par_user_id)
             VALUES ($1,$2,$3,$4,$5,'aide',$6,$7,$8,$9)",
        )
        .bind(aid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(v.participant_id)
        .bind(v.compte_id)
        .bind(&v.source)
        .bind(montant)
        .bind(&reference)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let vue = lire_aide(&mut t, a.etablissement_id, aid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Annule une aide par une écriture **inverse motivée** (l'aide d'origine n'est jamais modifiée).
    pub async fn voyage_aide_annuler(&self, a: &Acteur<'_>, id: Uuid, aide_id: Uuid, motif: &str) -> Result<AideVue, EntDbError> {
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let o = lire_aide(&mut t, a.etablissement_id, aide_id).await.map_err(|_| introuvable())?;
        let de_cette_sortie: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_aide WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(aide_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !de_cette_sortie {
            return Err(introuvable());
        }
        if o.sens != "aide" {
            return Err(invalide());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_aide WHERE etablissement_id = $1 AND annule_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(aide_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_aide
                 (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, reference,
                  annule_id, motif, decide_par_user_id)
             VALUES ($1,$2,$3,$4,$5,'annulation',$6,$7,$8,$9,$10,$11)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.participant_id)
        .bind(o.compte_id)
        .bind(&o.source)
        .bind(o.montant_centimes)
        .bind(&o.reference)
        .bind(aide_id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let vue = lire_aide(&mut t, a.etablissement_id, nid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Liste **nominative** des aides d'une sortie : comptabilité et direction seulement, **consultation journalisée**.
    pub async fn voyage_aides(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<AideVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v: Vec<AideVue> = sqlx::query_as(
            "SELECT x.id, x.participant_id, x.compte_id, x.sens, x.source, x.montant_centimes, x.reference, x.annule_id,
                    x.motif, x.decide_le,
                    (x.sens = 'aide'
                     AND NOT EXISTS (SELECT 1 FROM ent_sortie_aide y WHERE y.etablissement_id = x.etablissement_id AND y.annule_id = x.id)
                     AND NOT EXISTS (SELECT 1 FROM ent_sortie_participant p
                                      WHERE p.etablissement_id = x.etablissement_id AND p.id = x.participant_id
                                        AND p.etat = ANY($3))) AS a_restituer
               FROM ent_sortie_aide x WHERE x.etablissement_id = $1 AND x.sortie_id = $2 ORDER BY x.decide_le, x.id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(ETATS_PAYANTS.iter().map(|e| (*e).to_string()).collect::<Vec<_>>())
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", v.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Reports de versement (VOY-286, écart 9 de V2a) ─────────────────────────

    /// Reporte un montant déjà versé d'un participant à un autre **du même payeur**. Jamais plus que ce que ce payeur a
    /// versé pour le premier, jamais plus que ce qui reste dû pour le second ; si le premier est parti, son avoir
    /// proposé se recalcule sur ce qu'il lui reste.
    pub async fn voyage_report_effectuer(&self, a: &Acteur<'_>, id: Uuid, v: ReportSaisie) -> Result<ReportVue, EntDbError> {
        let montant = centimes_de_texte(&v.montant).filter(|m| (1..=VERSEMENT_MAX).contains(m)).ok_or_else(invalide)?;
        let motif = texte(&v.motif, 500)?;
        if v.de_participant_id == v.vers_participant_id {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        if s.nature == "obligatoire" {
            return Err(refus(RefusMetier::Conflit, "Sortie obligatoire : rien n'est dû par les familles."));
        }
        synchroniser(&mut t, a, &s).await?;
        // Ordre déterministe des verrous de lignes.
        let (premier, second) = if v.de_participant_id < v.vers_participant_id {
            (v.de_participant_id, v.vers_participant_id)
        } else {
            (v.vers_participant_id, v.de_participant_id)
        };
        let e1 = etat_participant(&mut t, a.etablissement_id, s.id, premier).await?;
        let e2 = etat_participant(&mut t, a.etablissement_id, s.id, second).await?;
        let (etat_de, etat_vers) = if premier == v.de_participant_id { (e1, e2) } else { (e2, e1) };
        if !ETATS_PAYANTS.contains(&etat_vers.as_str()) {
            return Err(refus(RefusMetier::Conflit, "Le participant qui reçoit le report n'est plus inscrit."));
        }
        let part_vers = part_courante(&mut t, a.etablissement_id, s.id, v.vers_participant_id, v.compte_id).await?;
        let part_de = part_courante(&mut t, a.etablissement_id, s.id, v.de_participant_id, v.compte_id).await?;
        let (Some(part_vers), Some(_)) = (part_vers, part_de) else {
            return Err(refus(
                RefusMetier::Conflit,
                "Un report reste chez le même payeur : ce payeur doit porter une part des deux participants.",
            ));
        };
        let verse_de = paye_net(&mut t, a.etablissement_id, s.id, v.de_participant_id, v.compte_id).await?;
        if montant > verse_de {
            return Err(refus(RefusMetier::Conflit, "Ce report dépasse ce que ce payeur a versé pour ce participant."));
        }
        let verse_vers = paye_net(&mut t, a.etablissement_id, s.id, v.vers_participant_id, v.compte_id).await?;
        let aides_vers = aides_nettes(&mut t, a.etablissement_id, s.id)
            .await?
            .get(&(v.vers_participant_id, v.compte_id))
            .copied()
            .unwrap_or(0);
        let couvert = verse_vers.checked_add(aides_vers).and_then(|x| x.checked_add(montant)).ok_or_else(invalide)?;
        if couvert > part_vers {
            return Err(refus(RefusMetier::Conflit, "Ce report dépasse ce qui reste dû pour le participant qui le reçoit."));
        }
        let rid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_report
                 (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens,
                  montant_centimes, motif, decide_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,'report',$7,$8,$9)",
        )
        .bind(rid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(v.de_participant_id)
        .bind(v.vers_participant_id)
        .bind(v.compte_id)
        .bind(montant)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        if est_parti(&etat_de) {
            recalculer_depart(&mut t, a, &s, v.de_participant_id).await?;
        }
        let vue = lire_report(&mut t, a.etablissement_id, rid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Défait un report par une écriture inverse motivée. Refusé si l'argent reporté n'est plus disponible chez le
    /// receveur (déjà remboursé ou reporté ailleurs) : la comptabilité règle d'abord ce mouvement.
    pub async fn voyage_report_annuler(&self, a: &Acteur<'_>, id: Uuid, report_id: Uuid, motif: &str) -> Result<ReportVue, EntDbError> {
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let o: ReportVue = sqlx::query_as(
            "SELECT id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, annule_id, motif, decide_le
               FROM ent_sortie_report WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(report_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if o.sens != "report" {
            return Err(invalide());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_report WHERE etablissement_id = $1 AND annule_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(report_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        let verse_vers = paye_net(&mut t, a.etablissement_id, s.id, o.vers_participant_id, o.compte_id).await?;
        if verse_vers < o.montant_centimes {
            return Err(refus(
                RefusMetier::Conflit,
                "L'argent reporté n'est plus disponible chez le participant qui l'a reçu : régler d'abord ce mouvement.",
            ));
        }
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_report
                 (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens,
                  montant_centimes, annule_id, motif, decide_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,'annulation',$7,$8,$9,$10)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.de_participant_id)
        .bind(o.vers_participant_id)
        .bind(o.compte_id)
        .bind(o.montant_centimes)
        .bind(report_id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        // Les avoirs déjà proposés ou retenus du receveur reposent sur un versé qui baisse : « à revoir » (M5).
        sqlx::query(
            "UPDATE ent_sortie_avoir_propose SET statut = 'a_revoir'
              WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
                AND statut IN ('propose', 'retenu')",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.vers_participant_id)
        .bind(o.compte_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let etat_de: String = etat_participant(&mut t, a.etablissement_id, s.id, o.de_participant_id).await?;
        if est_parti(&etat_de) {
            recalculer_depart(&mut t, a, &s, o.de_participant_id).await?;
        }
        let vue = lire_report(&mut t, a.etablissement_id, nid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Liste des reports d'une sortie : comptabilité et direction, consultation journalisée.
    pub async fn voyage_reports(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<ReportVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v: Vec<ReportVue> = sqlx::query_as(
            "SELECT id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, annule_id, motif, decide_le
               FROM ent_sortie_report WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY decide_le, id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", v.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Internat (socle 41) ─────────────────────────

    /// Aligne l'attendu de l'internat sur le séjour : chaque **interne** confirmé (prêt ou parti) est retiré des nuits
    /// passées hors de l'établissement (de la nuit du départ à la veille du retour), motif « séjour scolaire » ; un
    /// participant qui n'est plus confirmé est rétabli. Idempotent : rejouer ne change rien. Le porteur, la direction
    /// ou un droit de modification de sortie.
    pub async fn voyage_internat_synchroniser(&self, a: &Acteur<'_>, id: Uuid) -> Result<InternatSynchro, EntDbError> {
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(ent_auth::Action::VoyagesModifier) {
            return Err(interdit());
        }
        let r = internat_synchroniser_tx(&mut t, a, &s).await?;
        if r.poses + r.annules > 0 {
            journaliser(&mut t, a, s.id, Fait { champ: Some("internat"), ..Fait::simple("internat_synchronise") }).await?;
        }
        t.commit().await.map_err(db)?;
        Ok(r)
    }
}

/// Cœur de la synchronisation, dans la transaction de l'appelant.
pub(crate) async fn internat_synchroniser_tx(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
) -> Result<InternatSynchro, EntDbError> {
    let etab = a.etablissement_id;
    // Jours de l'établissement (jamais UTC) : nuit du départ à la veille du retour.
    let (du, retour): (NaiveDate, NaiveDate) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2), ent_aujourdhui_etab($1, $3)")
        .bind(etab)
        .bind(s.debut)
        .bind(s.fin)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let au = retour - chrono::Duration::days(1);
    let nuits = if au >= du { (au - du).num_days() + 1 } else { 0 };
    if nuits > 400 {
        return Err(refus(RefusMetier::Invalide, "Un séjour de plus de 400 nuits ne se retire pas de l'internat en une fois."));
    }
    // Internes confirmés dont le séjour d'internat couvre au moins une des nuits.
    let voulus: Vec<(Uuid, Uuid)> = if nuits == 0 {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT sp.id, sp.eleve_id FROM ent_sortie_participant sp
              WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2 AND sp.etat = ANY($3)
                AND EXISTS (SELECT 1 FROM ent_internat_sejour j
                             WHERE j.etablissement_id = sp.etablissement_id AND j.eleve_id = sp.eleve_id
                               AND daterange(j.du, j.au, '[]') && daterange($4, $5, '[]'))
              ORDER BY sp.id",
        )
        .bind(etab)
        .bind(s.id)
        .bind(ETATS_PARTANTS.iter().map(|e| (*e).to_string()).collect::<Vec<_>>())
        .bind(du)
        .bind(au)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
    };
    // Retraits de séjour encore actifs posés pour cette sortie.
    let existants: Vec<(Uuid, Uuid, Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT r.id, l.participant_id, r.eleve_id, r.du_nuit, r.au_nuit
           FROM ent_sortie_internat_retrait l
           JOIN ent_internat_retrait r ON r.etablissement_id = l.etablissement_id AND r.id = l.retrait_id
          WHERE l.etablissement_id = $1 AND l.sortie_id = $2 AND r.annule_le IS NULL",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let (mut poses, mut annules, mut inchanges) = (0usize, 0usize, 0usize);
    for (retrait, participant, eleve, rdu, rau) in &existants {
        let garde = nuits > 0 && *rdu == du && *rau == au && voulus.iter().any(|(p, _)| p == participant);
        if garde {
            inchanges += 1;
        } else {
            annuler_retrait(conn, a, *retrait, *eleve, *rdu, *rau).await?;
            annules += 1;
        }
    }
    for (participant, eleve) in &voulus {
        let deja = existants.iter().any(|(_, p, _, rdu, rau)| p == participant && *rdu == du && *rau == au);
        if deja {
            continue;
        }
        let retrait = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_retrait
                 (id, etablissement_id, eleve_id, du_nuit, au_nuit, motif_code, source, par_user_id)
             VALUES ($1,$2,$3,$4,$5,'sejour_scolaire','voyage',$6)",
        )
        .bind(retrait)
        .bind(etab)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .bind(a.user_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_sortie_internat_retrait
                 (id, etablissement_id, sortie_id, participant_id, retrait_id, du_nuit, au_nuit, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(s.id)
        .bind(participant)
        .bind(retrait)
        .bind(du)
        .bind(au)
        .bind(a.user_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        crate::finances::publier(
            conn,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            retrait,
            Some(*eleve),
            json!({ "du": du.to_string(), "au": au.to_string(), "cause": "retrait", "motif_code": "sejour_scolaire" }),
            Some(a.user_id),
            Causalite::default(),
        )
        .await?;
        poses += 1;
    }
    Ok(InternatSynchro { poses, annules, inchanges, nuits })
}
