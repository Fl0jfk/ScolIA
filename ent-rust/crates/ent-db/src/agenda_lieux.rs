//! Réservation de lieux (socle 19, lot AG3) et disponibilités (lot AG2, calculées).
//!
//! Toutes les salles du référentiel du socle 5, **salles de classe comprises** (D-058), sont réservables
//! selon un réglage **par salle** (`toujours`, `sur_validation`, `jamais`). La disponibilité vient des ports
//! du socle 10 (`edt_occupation_lieu`, `edt_occupe`, `edt_salles_liberees`), **jamais d'une copie** : un cours
//! supprimé libère la salle ; si un remplaçant est nommé ou le cours rétabli, la réservation **tombe** et le
//! demandeur est prévenu (motif public, salles de rechange). Le motif d'absence n'apparaît jamais.
//!
//! Défauts du Next.js (module réservation de salles) corrigés ici, un test chacun (`tests_agenda_ag3.rs`) :
//! NJ-62 / ND-07 lecture de toutes les réservations sans borne (ici : plage de 42 jours, projection minimale) ;
//! NJ-76 / ND-08 n'importe quel compte annule ou modifie la réservation d'un autre (ici : demandeur,
//! bénéficiaire ou gestionnaire, sinon 404) ; ND-01 double réservation (contrainte d'exclusion en base) ;
//! ND-02 succès partiel silencieux d'une série (rapport par occurrence) ; ND-03 dates passées, heures et
//! horizon non contrôlés ; ND-04 fuseau ; ND-05 HTML dans les e-mails (texte brut) ; ND-06 destinataire fourni
//! par le navigateur (résolu côté serveur) ; ND-09 chemin IA qui ne vérifie ni « réservable » ni l'EDT (une
//! seule commande serveur) ; ND-10 salle supprimée (clé composite) ; ND-11 faux conflits par sous-chaîne
//! (salle par identifiant) ; ND-12 écran plus restrictif que le serveur, auteur d'annulation en texte (fil de
//! faits en ajout seul).

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::agenda::*;
use serde_json::json;
use uuid::Uuid;

use crate::agenda::{instants, journaliser, nettoyer, tz_sql};
use crate::edt_ports::RessourcePlan;
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Plage maximale d'une lecture de planning (NJ-62 : aucune lecture sans borne).
pub const PLANNING_MAX_JOURS: i64 = 42;
/// Occurrences maximales d'une série de réservations.
pub const SERIE_RESERVATION_MAX: usize = 60;

fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

const CATEGORIES: [&str; 4] = ["enseignement", "reunion", "evenement", "autre"];

// ───────────────────────────── Logique pure ─────────────────────────────

/// Intersection de plages libres de plusieurs personnes (AG2, créneau commun) : plages d'au moins `duree`.
pub fn intersection_plages(
    listes: &[Vec<(NaiveTime, NaiveTime)>],
    duree: Duration,
) -> Vec<(NaiveTime, NaiveTime)> {
    let mut it = listes.iter();
    let Some(premiere) = it.next() else {
        return vec![];
    };
    let mut courant: Vec<(NaiveTime, NaiveTime)> = premiere.clone();
    for liste in it {
        let mut suivant = Vec::new();
        for (a, b) in &courant {
            for (c, d) in liste {
                let (debut, fin) = ((*a).max(*c), (*b).min(*d));
                if fin > debut && fin - debut >= duree {
                    suivant.push((debut, fin));
                }
            }
        }
        courant = suivant;
    }
    courant.sort();
    courant
}

/// Dates d'une série hebdomadaire (jours 1..7, intervalle en semaines, jusqu'à `au` inclus).
pub fn dates_serie(
    premier: NaiveDate,
    jours: &[i16],
    intervalle: i16,
    au: NaiveDate,
) -> Vec<NaiveDate> {
    let jours: BTreeSet<i16> = jours.iter().copied().collect();
    let lundi0 = premier - Duration::days(i64::from(premier.weekday().num_days_from_monday()));
    let mut out = Vec::new();
    let mut d = premier;
    while d <= au && out.len() <= SERIE_RESERVATION_MAX {
        let lundi = d - Duration::days(i64::from(d.weekday().num_days_from_monday()));
        let semaine = (lundi - lundi0).num_days() / 7;
        if jours.contains(&(d.weekday().number_from_monday() as i16))
            && semaine % i64::from(intervalle.max(1)) == 0
        {
            out.push(d);
        }
        d += Duration::days(1);
    }
    out
}

// ───────────────────────────── Accès ─────────────────────────────

struct InfoSalle {
    libelle: String,
}

#[derive(sqlx::FromRow)]
struct LigneReservation {
    id: Uuid,
    salle_id: Uuid,
    salle_libelle: String,
    jour: NaiveDate,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    intitule: String,
    motif_categorie: String,
    demandeur_user_id: Uuid,
    pour_user_id: Option<Uuid>,
    demandeur_nom: Option<String>,
    pour_nom: Option<String>,
    statut: String,
    effectif: Option<i32>,
    mise_en_place: Option<String>,
    commentaire: Option<String>,
    serie_id: Option<Uuid>,
    motif_decision: Option<String>,
}

fn select_reservation(tz: &str) -> String {
    format!(
        "SELECT r.id, r.salle_id, s.libelle AS salle_libelle,
                (r.debut AT TIME ZONE {tz})::date AS jour,
                (r.debut AT TIME ZONE {tz})::time AS heure_debut,
                (r.fin AT TIME ZONE {tz})::time AS heure_fin,
                r.intitule, r.motif_categorie, r.demandeur_user_id, r.pour_user_id,
                ud.display_name AS demandeur_nom, up.display_name AS pour_nom,
                r.statut, r.effectif, r.mise_en_place, r.commentaire, r.serie_id, r.motif_decision
           FROM ent_agenda_reservation r
           JOIN ent_salle s ON s.etablissement_id = r.etablissement_id AND s.id = r.salle_id
           JOIN ent_user ud ON ud.id = r.demandeur_user_id
           LEFT JOIN ent_user up ON up.id = r.pour_user_id"
    )
}

impl LigneReservation {
    /// Projection selon le lecteur (NJ-62) : le commentaire n'est lisible que du demandeur, du
    /// bénéficiaire et du gestionnaire.
    fn vue(self, lecteur: Uuid, gestionnaire: bool) -> ReservationVue {
        let partie = self.demandeur_user_id == lecteur || self.pour_user_id == Some(lecteur);
        ReservationVue {
            id: self.id,
            salle_id: self.salle_id,
            salle_libelle: self.salle_libelle,
            jour: self.jour,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            intitule: self.intitule,
            motif_categorie: self.motif_categorie,
            demandeur_nom: self.demandeur_nom,
            pour_nom: self.pour_nom,
            statut: self.statut,
            effectif: self.effectif,
            mise_en_place: self.mise_en_place,
            commentaire: if partie || gestionnaire {
                self.commentaire
            } else {
                None
            },
            je_suis_demandeur: self.demandeur_user_id == lecteur,
            serie_id: self.serie_id,
            motif_decision: self.motif_decision,
        }
    }
}

fn acteur_systeme(etab: Uuid) -> Acteur<'static> {
    // Les ports du socle 10 ne demandent que « personnel » : le job planifié se présente comme tel.
    Acteur {
        etablissement_id: etab,
        user_id: Uuid::nil(),
        role: "admin",
    }
}

impl PgEnt {
    async fn exiger_lieu(&self, a: &Acteur<'_>, action: Action) -> Result<(), EntDbError> {
        self.exiger_action_etab(a.etablissement_id, a.user_id, a.role, action)
            .await
    }

    async fn gestionnaire_lieux(&self, a: &Acteur<'_>) -> Result<bool, EntDbError> {
        self.a_action_etablissement(
            a.etablissement_id,
            a.user_id,
            a.role,
            Action::AgendaLieuxGerer,
        )
        .await
    }

    async fn info_salle(
        &self,
        conn: &mut sqlx::PgConnection,
        etab: Uuid,
        salle: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<InfoSalle, EntDbError> {
        let r: Option<(String, String, Option<i32>, String, Option<String>)> = sqlx::query_as(
            "SELECT code, libelle, capacite, type, site FROM ent_salle
              WHERE etablissement_id = $1 AND id = $2 AND actif
                AND (ferme_le IS NULL OR $3::date IS NULL OR ferme_le > $3::date)",
        )
        .bind(etab)
        .bind(salle)
        .bind(jour)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let (_code, libelle, _cap, _type, _site) = r.ok_or_else(introuvable)?;
        Ok(InfoSalle { libelle })
    }

    async fn regle_de(
        &self,
        conn: &mut sqlx::PgConnection,
        etab: Uuid,
        salle: Uuid,
    ) -> Result<RegleLieu, EntDbError> {
        let r: Option<(String, Option<Uuid>, i32, i32, i32)> = sqlx::query_as(
            "SELECT mode, valideur_user_id, delai_min_heures, duree_max_minutes, horizon_jours
               FROM ent_agenda_lieu_regle WHERE etablissement_id = $1 AND salle_id = $2",
        )
        .bind(etab)
        .bind(salle)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        Ok(r.map(
            |(mode, valideur_user_id, delai_min_heures, duree_max_minutes, horizon_jours)| {
                RegleLieu {
                    mode,
                    valideur_user_id,
                    delai_min_heures,
                    duree_max_minutes,
                    horizon_jours,
                }
            },
        )
        .unwrap_or_default())
    }

    // ───── Règles d'une salle (AGD-242) ───────────────────────────────────────────────────────

    pub async fn agenda_regle_lieu(
        &self,
        a: &Acteur<'_>,
        salle: Uuid,
    ) -> Result<RegleLieu, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxLire).await?;
        let mut c = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        self.info_salle(&mut c, a.etablissement_id, salle, None)
            .await?;
        self.regle_de(&mut c, a.etablissement_id, salle).await
    }

    pub async fn agenda_parametrer_lieu(
        &self,
        a: &Acteur<'_>,
        salle: Uuid,
        r: RegleLieu,
    ) -> Result<RegleLieu, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxParametrer).await?;
        if !MODES_LIEU.contains(&r.mode.as_str()) {
            return Err(invalide("mode : toujours, sur_validation ou jamais"));
        }
        if !(0..=720).contains(&r.delai_min_heures)
            || !(15..=1440).contains(&r.duree_max_minutes)
            || !(1..=400).contains(&r.horizon_jours)
        {
            return Err(invalide("délai, durée maximale ou horizon hors bornes"));
        }
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        self.info_salle(&mut tx, e, salle, None).await?;
        match (r.mode.as_str(), r.valideur_user_id) {
            ("sur_validation", None) => {
                return Err(invalide("une salle sur validation désigne son valideur"))
            }
            (_, Some(v)) => {
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2
                                     AND etat = 'ouvert' AND role NOT IN ('parent', 'eleve'))",
                )
                .bind(e)
                .bind(v)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !ok {
                    return Err(invalide(
                        "valideur inconnu : un compte du personnel de l'établissement",
                    ));
                }
            }
            _ => {}
        }
        sqlx::query(
            "INSERT INTO ent_agenda_lieu_regle
                (etablissement_id, salle_id, mode, valideur_user_id, delai_min_heures, duree_max_minutes,
                 horizon_jours, modifie_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT (etablissement_id, salle_id) DO UPDATE
                SET mode = $3, valideur_user_id = $4, delai_min_heures = $5, duree_max_minutes = $6,
                    horizon_jours = $7, modifie_par_user_id = $8, updated_at = now()",
        )
        .bind(e)
        .bind(salle)
        .bind(&r.mode)
        .bind(r.valideur_user_id)
        .bind(r.delai_min_heures)
        .bind(r.duree_max_minutes)
        .bind(r.horizon_jours)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            e,
            "type",
            salle,
            "regle_lieu",
            Some(a.user_id),
            None,
            Some(format!("{} / {}", r.mode, r.horizon_jours)),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    // ───── Recherche d'une salle libre (AGD-231) ───────────────────────────────────────────────

    /// Salles libres sur un créneau : règle de la salle, **emploi du temps réel** (port du socle 10 :
    /// cours, indisponibilités), réservations en cours. Une salle de classe dont le cours est supprimé
    /// est proposée avec sa raison (D-058).
    pub async fn agenda_rechercher_salles(
        &self,
        a: &Acteur<'_>,
        q: RechercheSalle,
    ) -> Result<Vec<SalleDispo>, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxLire).await?;
        if q.heure_fin <= q.heure_debut {
            return Err(invalide("la fin doit suivre le début"));
        }
        let e = a.etablissement_id;
        let mut c = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let (debut, fin, _) = instants(
            &mut c,
            e,
            q.jour,
            Some(q.heure_debut),
            q.jour,
            Some(q.heure_fin),
        )
        .await?;
        let salles: Vec<(Uuid, String, String, Option<i32>, String, Option<String>)> = sqlx::query_as(
            "SELECT s.id, s.code, s.libelle, s.capacite, s.type, s.site FROM ent_salle s
              WHERE s.etablissement_id = $1 AND s.actif AND (s.ferme_le IS NULL OR s.ferme_le > $2)
                AND ($3::int IS NULL OR s.capacite IS NULL OR s.capacite >= $3)
                AND ($4::text IS NULL OR s.type = $4)
                AND NOT EXISTS (SELECT 1 FROM ent_agenda_reservation r
                                 WHERE r.etablissement_id = s.etablissement_id AND r.salle_id = s.id
                                   AND r.statut IN ('demandee', 'confirmee') AND r.debut < $6 AND r.fin > $5)
              ORDER BY s.capacite NULLS LAST, s.code LIMIT 300",
        )
        .bind(e)
        .bind(q.jour)
        .bind(q.capacite_min)
        .bind(q.type_salle.as_deref())
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
        let liberees = self
            .edt_salles_liberees(a, q.jour, q.jour)
            .await
            .unwrap_or_default();
        let mut out = Vec::new();
        for (id, code, libelle, capacite, type_salle, site) in salles {
            let regle = self.regle_de(&mut c, e, id).await?;
            if regle.mode == "jamais" {
                continue;
            }
            if self
                .edt_occupe(a, id, q.jour, q.heure_debut, q.heure_fin)
                .await?
            {
                continue;
            }
            let raison_libre = liberees
                .iter()
                .find(|l| {
                    l.lieu_id == id && l.heure_debut < q.heure_fin && q.heure_debut < l.heure_fin
                })
                .map(|l| format!("cours de {} supprimé", l.classe));
            out.push(SalleDispo {
                salle_id: id,
                code,
                libelle,
                capacite,
                type_salle,
                site,
                mode: regle.mode,
                raison_libre,
            });
        }
        Ok(out)
    }

    // ───── Planning d'une salle (AGD-232, NJ-62) ───────────────────────────────────────────────

    /// Planning d'une salle sur une plage **bornée** : réservations (qui, intitulé ; commentaire réservé
    /// aux parties et au gestionnaire), cours (classe seule) et créneaux libérés par l'EDT. Aucun élève.
    pub async fn agenda_planning_salle(
        &self,
        a: &Acteur<'_>,
        salle: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<PlanningSalle, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxLire).await?;
        if au < du || (au - du).num_days() >= PLANNING_MAX_JOURS {
            return Err(invalide("plage invalide : 42 jours au plus"));
        }
        let e = a.etablissement_id;
        let gestionnaire = self.gestionnaire_lieux(a).await?;
        let mut c = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let info = self.info_salle(&mut c, e, salle, None).await?;
        let (debut, fin, _) = instants(&mut c, e, du, None, au, None).await?;
        let tz = tz_sql("$1");
        let lignes: Vec<LigneReservation> = sqlx::query_as(&format!(
            "{} WHERE r.etablissement_id = $1 AND r.salle_id = $2 AND r.statut IN ('demandee', 'confirmee')
                AND r.debut < $4 AND r.fin > $3 ORDER BY r.debut LIMIT 500",
            select_reservation(&tz)
        ))
        .bind(e)
        .bind(salle)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
        drop(c);
        let reservations = lignes
            .into_iter()
            .map(|l| l.vue(a.user_id, gestionnaire))
            .collect();
        let cours = self
            .edt_occupation_lieu(a, salle, du, au)
            .await?
            .into_iter()
            .map(|o| CoursSalleVue {
                jour: o.jour,
                heure_debut: o.heure_debut,
                heure_fin: o.heure_fin,
                classe: o.classe,
            })
            .collect();
        let liberees = self
            .edt_salles_liberees(a, du, au)
            .await?
            .into_iter()
            .filter(|l| l.lieu_id == salle)
            .map(|l| LibereeVue {
                jour: l.jour,
                heure_debut: l.heure_debut,
                heure_fin: l.heure_fin,
                raison: format!("cours de {} supprimé", l.classe),
            })
            .collect();
        Ok(PlanningSalle {
            salle_id: salle,
            salle_libelle: info.libelle,
            du,
            au,
            reservations,
            cours,
            liberees,
        })
    }

    /// Mes réservations à venir (demandeur ou bénéficiaire), plage bornée.
    pub async fn agenda_mes_reservations(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<ReservationVue>, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxLire).await?;
        if au < du || (au - du).num_days() > 120 {
            return Err(invalide("plage invalide : 120 jours au plus"));
        }
        let e = a.etablissement_id;
        let mut c = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let (debut, fin, _) = instants(&mut c, e, du, None, au, None).await?;
        let tz = tz_sql("$1");
        let lignes: Vec<LigneReservation> = sqlx::query_as(&format!(
            "{} WHERE r.etablissement_id = $1 AND (r.demandeur_user_id = $2 OR r.pour_user_id = $2)
                AND r.debut < $4 AND r.fin > $3 AND r.statut <> 'annulee' ORDER BY r.debut LIMIT 300",
            select_reservation(&tz)
        ))
        .bind(e)
        .bind(a.user_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|l| l.vue(a.user_id, false))
            .collect())
    }

    // ───── Réserver (AGD-233, AGD-234, AGD-235) ────────────────────────────────────────────────

    /// Contrôles d'une occurrence, à la date : renvoie la raison du conflit, ou `None` si possible.
    async fn conflit_occurrence(
        &self,
        a: &Acteur<'_>,
        conn: &mut sqlx::PgConnection,
        salle: Uuid,
        regle: &RegleLieu,
        jour: NaiveDate,
        hd: NaiveTime,
        hf: NaiveTime,
        debut: DateTime<Utc>,
        fin: DateTime<Utc>,
        gestionnaire: bool,
    ) -> Result<Option<String>, EntDbError> {
        let e = a.etablissement_id;
        let maintenant = Utc::now();
        if debut <= maintenant {
            return Ok(Some("ce créneau est passé".into()));
        }
        if !gestionnaire && debut < maintenant + Duration::hours(i64::from(regle.delai_min_heures))
        {
            return Ok(Some(format!(
                "délai minimal de {} h non respecté",
                regle.delai_min_heures
            )));
        }
        let aujourdhui = self.aujourdhui_etablissement(e).await?;
        if !gestionnaire && (jour - aujourdhui).num_days() > i64::from(regle.horizon_jours) {
            return Ok(Some(format!(
                "au-delà de l'horizon de {} jours",
                regle.horizon_jours
            )));
        }
        if (fin - debut).num_minutes() > i64::from(regle.duree_max_minutes) {
            return Ok(Some(format!(
                "durée maximale de {} minutes dépassée",
                regle.duree_max_minutes
            )));
        }
        // Emploi du temps réel (port du socle 10) : qui occupe, par la classe, jamais l'intervenant.
        let occ = self.edt_occupation_lieu(a, salle, jour, jour).await?;
        if let Some(o) = occ.iter().find(|o| o.heure_debut < hf && hd < o.heure_fin) {
            return Ok(Some(format!(
                "occupée par un cours de {} ({}–{})",
                o.classe,
                o.heure_debut.format("%H:%M"),
                o.heure_fin.format("%H:%M")
            )));
        }
        if self.edt_occupe(a, salle, jour, hd, hf).await? {
            return Ok(Some(
                "salle indisponible (travaux, examen ou fermeture)".into(),
            ));
        }
        // Autre réservation : on dit qui (nom et intitulé, comme dans le module d'origine).
        let autre: Option<(Option<String>, String)> = sqlx::query_as(
            "SELECT u.display_name, r.intitule FROM ent_agenda_reservation r
               JOIN ent_user u ON u.id = r.demandeur_user_id
              WHERE r.etablissement_id = $1 AND r.salle_id = $2 AND r.statut IN ('demandee', 'confirmee')
                AND r.debut < $4 AND r.fin > $3 LIMIT 1",
        )
        .bind(e)
        .bind(salle)
        .bind(debut)
        .bind(fin)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((nom, intitule)) = autre {
            return Ok(Some(format!(
                "déjà réservée par {} : {}",
                nom.unwrap_or_else(|| "un collègue".into()),
                intitule
            )));
        }
        Ok(None)
    }

    /// Réserve une salle (unitaire ou en série). Les conflits sont **dits par occurrence** ; une série en
    /// conflit n'écrit rien tant que l'auteur ne confirme pas (`confirmer_partiel`). Les mêmes contrôles
    /// valent pour l'écran, l'assistant IA et tout appel : il n'y a qu'une seule commande.
    pub async fn agenda_reserver_lieu(
        &self,
        a: &Acteur<'_>,
        n: NouvelleReservation,
    ) -> Result<RapportReservation, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxReserver).await?;
        let intitule = nettoyer(&n.intitule, 200, "intitulé : 1 à 200 caractères")?;
        if n.heure_fin <= n.heure_debut {
            return Err(invalide("l'heure de fin doit suivre l'heure de début"));
        }
        let categorie = n.motif_categorie.clone().unwrap_or_else(|| "autre".into());
        if !CATEGORIES.contains(&categorie.as_str()) {
            return Err(invalide("catégorie inconnue"));
        }
        if n.effectif.is_some_and(|x| !(1..=2000).contains(&x))
            || n.mise_en_place
                .as_deref()
                .is_some_and(|t| t.chars().count() > 500)
            || n.commentaire
                .as_deref()
                .is_some_and(|t| t.chars().count() > 1000)
        {
            return Err(invalide(
                "effectif, mise en place ou commentaire hors bornes",
            ));
        }
        let dates: Vec<NaiveDate> = match &n.serie {
            None => vec![n.jour],
            Some(s) => {
                let jours: BTreeSet<i16> = s.jours_semaine.iter().copied().collect();
                let inter = s.intervalle_semaines.unwrap_or(1);
                if jours.is_empty()
                    || jours.iter().any(|j| !(1..=7).contains(j))
                    || !(1..=8).contains(&inter)
                {
                    return Err(invalide("série : jours 1 à 7, intervalle 1 à 8 semaines"));
                }
                if s.au < n.jour || s.au > n.jour + Duration::days(400) {
                    return Err(invalide(
                        "la série se termine après son début, dans l'année",
                    ));
                }
                let d = dates_serie(n.jour, &s.jours_semaine, inter, s.au);
                if d.is_empty() || d.len() > SERIE_RESERVATION_MAX {
                    return Err(invalide("série : de 1 à 60 occurrences"));
                }
                d
            }
        };
        let e = a.etablissement_id;
        let gestionnaire = self.gestionnaire_lieux(a).await?;
        let mut tx = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let info = self
            .info_salle(&mut tx, e, n.salle_id, Some(n.jour))
            .await?;
        let regle = self.regle_de(&mut tx, e, n.salle_id).await?;
        if regle.mode == "jamais" {
            return Err(EntDbError::ConflitDossier(
                "cette salle n'est pas réservable",
            ));
        }
        if let Some(pour) = n.pour_user_id.filter(|p| *p != a.user_id) {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2
                                 AND etat = 'ouvert' AND role NOT IN ('parent', 'eleve'))",
            )
            .bind(e)
            .bind(pour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide(
                    "bénéficiaire inconnu : un compte du personnel de l'établissement",
                ));
            }
        }
        // Une réservation à la fois par salle : le verrou précède les contrôles et l'écriture.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 12))")
            .bind(n.salle_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let mut occurrences: Vec<(NaiveDate, DateTime<Utc>, DateTime<Utc>, Option<String>)> =
            Vec::new();
        for jour in &dates {
            let (debut, fin, _) = instants(
                &mut tx,
                e,
                *jour,
                Some(n.heure_debut),
                *jour,
                Some(n.heure_fin),
            )
            .await?;
            let conflit = self
                .conflit_occurrence(
                    a,
                    &mut tx,
                    n.salle_id,
                    &regle,
                    *jour,
                    n.heure_debut,
                    n.heure_fin,
                    debut,
                    fin,
                    gestionnaire,
                )
                .await?;
            occurrences.push((*jour, debut, fin, conflit));
        }
        let nb_conflits = occurrences.iter().filter(|o| o.3.is_some()).count();
        if n.serie.is_none() && nb_conflits > 0 {
            let msg = occurrences[0].3.clone().unwrap_or_default();
            return Err(EntDbError::ConflitDossierDetail(format!(
                "{} : {msg}",
                info.libelle
            )));
        }
        let mut rapport: Vec<OccurrenceReservation> = occurrences
            .iter()
            .map(|(j, _, _, c)| OccurrenceReservation {
                jour: *j,
                reservation_id: None,
                statut: None,
                conflit: c.clone(),
            })
            .collect();
        if nb_conflits > 0 && !n.confirmer_partiel {
            return Ok(RapportReservation {
                ecrit: false,
                occurrences: rapport,
            });
        }
        let direct = regle.mode != "sur_validation"
            || gestionnaire
            || regle.valideur_user_id == Some(a.user_id);
        let statut = if direct { "confirmee" } else { "demandee" };
        let serie_id = n.serie.as_ref().map(|_| Uuid::new_v4());
        if let (Some(sid), Some(s)) = (serie_id, &n.serie) {
            sqlx::query(
                "INSERT INTO ent_agenda_serie (id, etablissement_id, frequence, intervalle, du, au, cree_par_user_id)
                 VALUES ($1,$2,'hebdomadaire',$3,$4,$5,$6)",
            )
            .bind(sid)
            .bind(e)
            .bind(s.intervalle_semaines.unwrap_or(1))
            .bind(n.jour)
            .bind(s.au)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            for j in s.jours_semaine.iter().copied().collect::<BTreeSet<i16>>() {
                sqlx::query("INSERT INTO ent_agenda_serie_jour (etablissement_id, serie_id, jour_semaine) VALUES ($1,$2,$3)")
                    .bind(e)
                    .bind(sid)
                    .bind(j)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
        }
        let mut premier: Option<(Uuid, DateTime<Utc>)> = None;
        for (i, (jour, debut, fin, conflit)) in occurrences.iter().enumerate() {
            if conflit.is_some() {
                continue;
            }
            let id = Uuid::new_v4();
            let r = sqlx::query(
                "INSERT INTO ent_agenda_reservation
                    (id, etablissement_id, salle_id, debut, fin, demandeur_user_id, pour_user_id, intitule,
                     motif_categorie, effectif, mise_en_place, commentaire, serie_id, statut)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
            )
            .bind(id)
            .bind(e)
            .bind(n.salle_id)
            .bind(debut)
            .bind(fin)
            .bind(a.user_id)
            .bind(n.pour_user_id.filter(|p| *p != a.user_id))
            .bind(&intitule)
            .bind(&categorie)
            .bind(n.effectif)
            .bind(n.mise_en_place.as_deref().map(str::trim).filter(|t| !t.is_empty()))
            .bind(n.commentaire.as_deref().map(str::trim).filter(|t| !t.is_empty()))
            .bind(serie_id)
            .bind(statut)
            .execute(&mut *tx)
            .await;
            if let Err(err) = r {
                // Filet de sécurité : la contrainte d'exclusion (ND-01) tranche une course que les contrôles
                // n'auraient pas vue.
                if let sqlx::Error::Database(d) = &err {
                    if d.code().as_deref() == Some("23P01") {
                        return Err(EntDbError::ConflitDossier(
                            "cette salle vient d'être réservée",
                        ));
                    }
                }
                return Err(db(err));
            }
            sqlx::query(
                "INSERT INTO ent_agenda_reservation_fait (id, etablissement_id, reservation_id, fait, par_user_id)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(id)
            .bind(if direct { "confirmee" } else { "demande" })
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            rapport[i].reservation_id = Some(id);
            rapport[i].statut = Some(statut.to_string());
            premier.get_or_insert((id, *debut));
            let _ = jour;
        }
        if let Some((id, debut)) = premier {
            let evt = tx
                .publier(MetierEventDraft {
                    etablissement_id: e,
                    type_name: if direct {
                        "agenda.lieu_reserve"
                    } else {
                        "agenda.lieu_demande"
                    }
                    .to_string(),
                    aggregate: AggregateKind::AgendaReservation,
                    aggregate_id: id,
                    eleve_id: None,
                    payload: json!({ "reservation_id": id, "salle_id": n.salle_id }),
                    actor_user_id: Some(a.user_id.to_string()),
                })
                .await?;
            let tz = tz_sql("$1");
            let (quand,): (String,) = sqlx::query_as(&format!(
                "SELECT to_char($2::timestamptz AT TIME ZONE {tz}, 'DD/MM/YYYY \"à\" HH24:MI')"
            ))
            .bind(e)
            .bind(debut)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let nb = rapport
                .iter()
                .filter(|o| o.reservation_id.is_some())
                .count();
            let suite = if nb > 1 {
                format!(" (et {} autres dates)", nb - 1)
            } else {
                String::new()
            };
            let maintenant = Utc::now();
            if direct {
                if let Some(pour) = n.pour_user_id.filter(|p| *p != a.user_id) {
                    let corps = format!(
                        "{} est réservée pour vous le {quand}{suite} : {intitule}.",
                        info.libelle
                    );
                    notifier_personne(
                        &mut tx,
                        EnvoiAPlanifier {
                            etablissement_id: e,
                            user_id: pour,
                            kind: "agenda_lieu_reserve",
                            titre: "Salle réservée pour vous",
                            corps: &corps,
                            eleve_id: None,
                            notification_id: None,
                            source_type: "evenement",
                            source_id: id,
                            cle_dedup: format!("lieu:{id}:reserve"),
                            droit_requis: None,
                            regroupable: false,
                            canaux_voulus: &["email", "push"],
                        },
                        evt.id,
                        maintenant,
                    )
                    .await?;
                }
            } else if let Some(valideur) = regle.valideur_user_id {
                let corps = format!(
                    "Demande pour {} le {quand}{suite} : {intitule}.",
                    info.libelle
                );
                notifier_personne(
                    &mut tx,
                    EnvoiAPlanifier {
                        etablissement_id: e,
                        user_id: valideur,
                        kind: "agenda_lieu_demande",
                        titre: "Demande de salle à valider",
                        corps: &corps,
                        eleve_id: None,
                        notification_id: None,
                        source_type: "evenement",
                        source_id: id,
                        cle_dedup: format!("lieu:{id}:demande"),
                        droit_requis: None,
                        regroupable: false,
                        canaux_voulus: &["push"],
                    },
                    evt.id,
                    maintenant,
                )
                .await?;
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(RapportReservation {
            ecrit: true,
            occurrences: rapport,
        })
    }

    // ───── Annuler (AGD-237, AGD-238, NJ-76) ───────────────────────────────────────────────────

    /// Annule une réservation (ou les occurrences à venir de sa série). **Seuls** le demandeur, le
    /// bénéficiaire et le gestionnaire le peuvent : tout autre compte reçoit 404, comme pour une réservation
    /// inexistante (NJ-76). Le gestionnaire qui annule celle d'un autre doit donner un motif, communiqué.
    pub async fn agenda_annuler_reservation(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        an: AnnulationReservation,
    ) -> Result<usize, EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxLire).await?;
        let e = a.etablissement_id;
        let gestionnaire = self.gestionnaire_lieux(a).await?;
        let motif = an
            .motif
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(str::to_string);
        if motif.as_deref().is_some_and(|m| m.chars().count() > 300) {
            return Err(invalide("motif trop long"));
        }
        let mut tx = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let r: Option<(
            Uuid,
            Option<Uuid>,
            String,
            Option<Uuid>,
            Uuid,
            DateTime<Utc>,
            String,
        )> = sqlx::query_as(
            "SELECT demandeur_user_id, pour_user_id, statut, serie_id, salle_id, debut, intitule
               FROM ent_agenda_reservation WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (demandeur, pour, statut, serie, salle, debut, intitule) = r.ok_or_else(introuvable)?;
        let partie = demandeur == a.user_id || pour == Some(a.user_id);
        if !partie && !gestionnaire {
            return Err(introuvable());
        }
        if !matches!(statut.as_str(), "demandee" | "confirmee") {
            return Err(EntDbError::ConflitDossier(
                "cette réservation n'est plus active",
            ));
        }
        if !partie && motif.is_none() {
            return Err(invalide(
                "un motif est obligatoire pour annuler la réservation d'un collègue",
            ));
        }
        if !gestionnaire && debut <= Utc::now() {
            return Err(EntDbError::ConflitDossier("cette réservation est passée"));
        }
        let cibles: Vec<(Uuid, DateTime<Utc>)> = if an.portee.as_deref() == Some("serie") {
            let Some(sid) = serie else {
                return Err(invalide("cette réservation ne fait pas partie d'une série"));
            };
            sqlx::query_as(
                "SELECT id, debut FROM ent_agenda_reservation
                  WHERE etablissement_id = $1 AND serie_id = $2 AND statut IN ('demandee', 'confirmee')
                    AND debut >= $3 AND ($4 OR demandeur_user_id = $5 OR pour_user_id = $5)
                  ORDER BY debut FOR UPDATE",
            )
            .bind(e)
            .bind(sid)
            .bind(debut)
            .bind(gestionnaire)
            .bind(a.user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        } else {
            vec![(id, debut)]
        };
        let (info_libelle,): (String,) =
            sqlx::query_as("SELECT libelle FROM ent_salle WHERE etablissement_id = $1 AND id = $2")
                .bind(e)
                .bind(salle)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        for (cid, _) in &cibles {
            sqlx::query(
                "UPDATE ent_agenda_reservation SET statut = 'annulee', motif_decision = $3,
                        version = version + 1, updated_at = now() WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(e)
            .bind(cid)
            .bind(&motif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_agenda_reservation_fait (id, etablissement_id, reservation_id, fait, par_user_id, motif_public)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(cid)
            .bind(if partie { "annulee_demandeur" } else { "annulee_gestionnaire" })
            .bind(a.user_id)
            .bind(&motif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: e,
                type_name: "agenda.lieu_annule".to_string(),
                aggregate: AggregateKind::AgendaReservation,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({ "reservation_id": id, "nb": cibles.len() }),
                actor_user_id: Some(a.user_id.to_string()),
            })
            .await?;
        // Le destinataire est résolu côté serveur (ND-06) : l'autre partie de la réservation.
        let tz = tz_sql("$1");
        let (quand,): (String,) = sqlx::query_as(&format!(
            "SELECT to_char($2::timestamptz AT TIME ZONE {tz}, 'DD/MM/YYYY \"à\" HH24:MI')"
        ))
        .bind(e)
        .bind(debut)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let a_prevenir: BTreeSet<Uuid> = [Some(demandeur), pour]
            .into_iter()
            .flatten()
            .filter(|u| *u != a.user_id)
            .collect();
        let suite = if cibles.len() > 1 {
            format!(" (et {} autres dates)", cibles.len() - 1)
        } else {
            String::new()
        };
        for u in a_prevenir {
            let corps =
                format!(
                "La réservation de {info_libelle} le {quand}{suite} ({intitule}) est annulée.{}",
                motif.as_deref().map(|m| format!(" Motif : {m}.")).unwrap_or_default()
            );
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: e,
                    user_id: u,
                    kind: "agenda_lieu_annule",
                    titre: "Réservation annulée",
                    corps: &corps,
                    eleve_id: None,
                    notification_id: None,
                    source_type: "evenement",
                    source_id: id,
                    cle_dedup: format!("lieu:{id}:annule:{evt_id}", evt_id = evt.id),
                    droit_requis: None,
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                evt.id,
                Utc::now(),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(cibles.len())
    }

    // ───── Valider ou refuser (AGD-236) ────────────────────────────────────────────────────────

    pub async fn agenda_decider_reservation(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        d: DecisionReservation,
    ) -> Result<(), EntDbError> {
        self.exiger_lieu(a, Action::AgendaLieuxLire).await?;
        let e = a.etablissement_id;
        let motif = d
            .motif
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(str::to_string);
        if !d.accepter && motif.is_none() {
            return Err(invalide(
                "un motif est obligatoire pour refuser une demande",
            ));
        }
        if motif.as_deref().is_some_and(|m| m.chars().count() > 300) {
            return Err(invalide("motif trop long"));
        }
        let valideur_general = self
            .a_action_etablissement(e, a.user_id, a.role, Action::AgendaLieuxValider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let r: Option<(Uuid, String, Uuid)> = sqlx::query_as(
            "SELECT demandeur_user_id, statut, salle_id FROM ent_agenda_reservation
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (demandeur, statut, salle) = r.ok_or_else(introuvable)?;
        let regle = self.regle_de(&mut tx, e, salle).await?;
        if regle.valideur_user_id != Some(a.user_id) && !valideur_general {
            return Err(introuvable());
        }
        if statut != "demandee" {
            return Err(EntDbError::ConflitDossier("cette demande est déjà traitée"));
        }
        let nouveau = if d.accepter { "confirmee" } else { "refusee" };
        sqlx::query(
            "UPDATE ent_agenda_reservation SET statut = $3, decide_par_user_id = $4, decide_le = now(),
                    motif_decision = $5, version = version + 1, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(id)
        .bind(nouveau)
        .bind(a.user_id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_agenda_reservation_fait (id, etablissement_id, reservation_id, fait, par_user_id, motif_public)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(id)
        .bind(nouveau)
        .bind(a.user_id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: e,
                type_name: "agenda.lieu_decide".to_string(),
                aggregate: AggregateKind::AgendaReservation,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({ "reservation_id": id, "accepter": d.accepter }),
                actor_user_id: Some(a.user_id.to_string()),
            })
            .await?;
        let corps = format!(
            "Votre demande de salle est {}.{}",
            if d.accepter { "acceptée" } else { "refusée" },
            motif
                .as_deref()
                .map(|m| format!(" Motif : {m}."))
                .unwrap_or_default()
        );
        notifier_personne(
            &mut tx,
            EnvoiAPlanifier {
                etablissement_id: e,
                user_id: demandeur,
                kind: "agenda_lieu_decide",
                titre: "Décision sur votre demande de salle",
                corps: &corps,
                eleve_id: None,
                notification_id: None,
                source_type: "evenement",
                source_id: id,
                cle_dedup: format!("lieu:{id}:decide"),
                droit_requis: None,
                regroupable: false,
                canaux_voulus: &["email", "push"],
            },
            evt.id,
            Utc::now(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───── D-058 : le cours prime ─────────────────────────────────────────────────────────────

    /// Les réservations à venir dont le créneau est **de nouveau occupé par un cours** (remplaçant nommé,
    /// cours rétabli) **tombent** : statut `tombee`, fait au fil, demandeur prévenu avec le motif public
    /// (« le cours de 4B est rétabli ») et des salles de rechange. Idempotent ; lancé avec les rappels.
    /// Retourne le nombre de réservations tombées.
    pub async fn agenda_reviser_reservations_edt(
        &self,
        maintenant: DateTime<Utc>,
        etablissement: Option<Uuid>,
    ) -> Result<usize, EntDbError> {
        let etabs = match etablissement {
            Some(e) => vec![e],
            None => crate::tenant_tx::etablissements_systeme(
                self.pool(),
                crate::tenant_tx::ContexteSysteme::JobPlanifie("reservations_lieux"),
            )
            .await
            .map_err(db)?,
        };
        let mut total = 0;
        for e in etabs {
            let sys = acteur_systeme(e);
            let tz = tz_sql("$1");
            let mut tx = TenantTx::begin(self.pool(), e).await.map_err(db)?;
            let lignes: Vec<(
                Uuid,
                Uuid,
                Uuid,
                Option<Uuid>,
                NaiveDate,
                NaiveTime,
                NaiveTime,
                String,
            )> = sqlx::query_as(&format!(
                "SELECT r.id, r.salle_id, r.demandeur_user_id, r.pour_user_id,
                        (r.debut AT TIME ZONE {tz})::date, (r.debut AT TIME ZONE {tz})::time,
                        (r.fin AT TIME ZONE {tz})::time, s.libelle
                   FROM ent_agenda_reservation r
                   JOIN ent_salle s ON s.etablissement_id = r.etablissement_id AND s.id = r.salle_id
                  WHERE r.etablissement_id = $1 AND r.statut IN ('demandee', 'confirmee')
                    AND r.debut > $2 AND r.debut < $2 + interval '62 days'
                  ORDER BY r.debut LIMIT 2000"
            ))
            .bind(e)
            .bind(maintenant)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            drop(tx);
            for (id, salle, demandeur, pour, jour, hd, hf, libelle) in lignes {
                let occ = self.edt_occupation_lieu(&sys, salle, jour, jour).await?;
                let Some(o) = occ.iter().find(|o| o.heure_debut < hf && hd < o.heure_fin) else {
                    continue;
                };
                let motif = format!("le cours de {} est rétabli dans cette salle", o.classe);
                let alternatives: Vec<String> = self
                    .agenda_rechercher_salles(
                        &sys,
                        RechercheSalle {
                            jour,
                            heure_debut: hd,
                            heure_fin: hf,
                            capacite_min: None,
                            type_salle: None,
                        },
                    )
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|s| s.salle_id != salle)
                    .take(3)
                    .map(|s| s.libelle)
                    .collect();
                let mut tx = TenantTx::begin(self.pool(), e).await.map_err(db)?;
                let n = sqlx::query(
                    "UPDATE ent_agenda_reservation SET statut = 'tombee', motif_decision = $3,
                            version = version + 1, updated_at = now()
                      WHERE etablissement_id = $1 AND id = $2 AND statut IN ('demandee', 'confirmee')",
                )
                .bind(e)
                .bind(id)
                .bind(&motif)
                .execute(&mut *tx)
                .await
                .map_err(db)?
                .rows_affected();
                if n == 0 {
                    continue;
                }
                sqlx::query(
                    "INSERT INTO ent_agenda_reservation_fait (id, etablissement_id, reservation_id, fait, motif_public)
                     VALUES ($1,$2,$3,'tombee',$4)",
                )
                .bind(Uuid::new_v4())
                .bind(e)
                .bind(id)
                .bind(&motif)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                let evt = tx
                    .publier(MetierEventDraft {
                        etablissement_id: e,
                        type_name: "agenda.lieu_en_conflit".to_string(),
                        aggregate: AggregateKind::AgendaReservation,
                        aggregate_id: id,
                        eleve_id: None,
                        payload: json!({ "reservation_id": id }),
                        actor_user_id: None,
                    })
                    .await?;
                let corps = format!(
                    "Votre réservation de {libelle} le {} de {} à {} est annulée : {motif}.{}",
                    jour.format("%d/%m/%Y"),
                    hd.format("%H:%M"),
                    hf.format("%H:%M"),
                    if alternatives.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " Salles libres sur ce créneau : {}.",
                            alternatives.join(", ")
                        )
                    }
                );
                for u in [Some(demandeur), pour]
                    .into_iter()
                    .flatten()
                    .collect::<BTreeSet<_>>()
                {
                    notifier_personne(
                        &mut tx,
                        EnvoiAPlanifier {
                            etablissement_id: e,
                            user_id: u,
                            kind: "agenda_lieu_tombee",
                            titre: "Réservation annulée : le cours est rétabli",
                            corps: &corps,
                            eleve_id: None,
                            notification_id: None,
                            source_type: "evenement",
                            source_id: id,
                            cle_dedup: format!("lieu:{id}:tombee"),
                            droit_requis: None,
                            regroupable: false,
                            canaux_voulus: &["email", "push"],
                        },
                        evt.id,
                        maintenant,
                    )
                    .await?;
                }
                tx.commit().await.map_err(db)?;
                total += 1;
            }
        }
        Ok(total)
    }

    // ───── AG2 : disponibilités calculées (libre / occupé), créneau commun ─────────────────────

    /// Libre / occupé de personnes sur un créneau, **sans motif** (R-AG6). Les personnels de santé et du
    /// social ne sont jamais détaillés (R-AG37 : l'occupation désignerait des élèves) : `non_communique`.
    /// L'occupation vient de l'emploi du temps réel (port du socle 10), jamais d'une copie.
    pub async fn agenda_libre_occupe(
        &self,
        a: &Acteur<'_>,
        personnes: &[Uuid],
        jour: NaiveDate,
        debut: NaiveTime,
        fin: NaiveTime,
    ) -> Result<Vec<DisponibilitePersonne>, EntDbError> {
        self.exiger_action_etab(
            a.etablissement_id,
            a.user_id,
            a.role,
            Action::AgendaLireStructure,
        )
        .await?;
        if fin <= debut {
            return Err(invalide("la fin doit suivre le début"));
        }
        if personnes.is_empty() || personnes.len() > 60 {
            return Err(invalide("de 1 à 60 personnes"));
        }
        let e = a.etablissement_id;
        let mut c = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let rows: Vec<(Uuid, Option<String>, String)> = sqlx::query_as(
            "SELECT m.user_id, u.display_name, m.role FROM ent_membership m JOIN ent_user u ON u.id = m.user_id
              WHERE m.etablissement_id = $1 AND m.user_id = ANY($2) AND m.etat = 'ouvert'
                AND m.role NOT IN ('parent', 'eleve')",
        )
        .bind(e)
        .bind(personnes)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
        drop(c);
        let mut out = Vec::new();
        for (u, nom, role) in rows {
            if matches!(Role::parse(&role), Some(Role::Infirmerie | Role::Psy)) {
                out.push(DisponibilitePersonne {
                    user_id: u,
                    nom,
                    etat: "non_communique".into(),
                });
                continue;
            }
            let libres = self
                .edt_plages_libres(
                    a,
                    RessourcePlan::Intervenant(u),
                    jour,
                    jour,
                    (fin - debut).num_minutes().max(1),
                )
                .await?;
            let libre = libres.iter().any(|p| p.debut <= debut && fin <= p.fin);
            out.push(DisponibilitePersonne {
                user_id: u,
                nom,
                etat: if libre { "libre" } else { "occupe" }.into(),
            });
        }
        out.sort_by(|x, y| x.nom.cmp(&y.nom));
        Ok(out)
    }

    /// Créneau commun : plages où **toutes** les personnes (hors santé et social) sont libres d'après
    /// l'emploi du temps réel, d'au moins `duree_min` minutes, sur une plage bornée.
    pub async fn agenda_creneau_commun(
        &self,
        a: &Acteur<'_>,
        personnes: &[Uuid],
        du: NaiveDate,
        au: NaiveDate,
        duree_min: i64,
    ) -> Result<Vec<PlageCommune>, EntDbError> {
        self.exiger_action_etab(
            a.etablissement_id,
            a.user_id,
            a.role,
            Action::AgendaLireStructure,
        )
        .await?;
        if personnes.len() < 2 || personnes.len() > 20 || !(5..=480).contains(&duree_min) {
            return Err(invalide("de 2 à 20 personnes, durée de 5 à 480 minutes"));
        }
        if au < du || (au - du).num_days() >= 28 {
            return Err(invalide("plage invalide : 28 jours au plus"));
        }
        let e = a.etablissement_id;
        let mut c = TenantTx::begin(self.pool(), e).await.map_err(db)?;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_membership WHERE etablissement_id = $1 AND user_id = ANY($2)
                AND etat = 'ouvert' AND role NOT IN ('parent', 'eleve', 'infirmerie', 'psy')",
        )
        .bind(e)
        .bind(personnes)
        .fetch_one(&mut *c)
        .await
        .map_err(db)?;
        drop(c);
        if n as usize != personnes.iter().collect::<BTreeSet<_>>().len() {
            return Err(invalide(
                "personnes inconnues ou dont l'occupation n'est pas communicable",
            ));
        }
        let mut par_personne: Vec<Vec<crate::edt_ports::PlageLibre>> = Vec::new();
        for u in personnes {
            par_personne.push(
                self.edt_plages_libres(a, RessourcePlan::Intervenant(*u), du, au, duree_min)
                    .await?,
            );
        }
        let mut out = Vec::new();
        let mut j = du;
        while j <= au {
            let listes: Vec<Vec<(NaiveTime, NaiveTime)>> = par_personne
                .iter()
                .map(|v| {
                    v.iter()
                        .filter(|p| p.jour == j)
                        .map(|p| (p.debut, p.fin))
                        .collect()
                })
                .collect();
            for (debut, fin) in intersection_plages(&listes, Duration::minutes(duree_min)) {
                out.push(PlageCommune {
                    jour: j,
                    debut,
                    fin,
                });
            }
            j += Duration::days(1);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn intersection_des_plages_libres() {
        let a = vec![(t(8, 0), t(12, 0)), (t(14, 0), t(17, 0))];
        let b = vec![(t(9, 0), t(10, 0)), (t(11, 0), t(15, 0))];
        let r = intersection_plages(&[a.clone(), b.clone()], Duration::minutes(30));
        assert_eq!(
            r,
            vec![
                (t(9, 0), t(10, 0)),
                (t(11, 0), t(12, 0)),
                (t(14, 0), t(15, 0))
            ]
        );
        // Durée minimale : la plage de 14 h à 15 h tient 60 minutes, pas 90.
        assert_eq!(intersection_plages(&[a, b], Duration::minutes(90)), vec![]);
        assert_eq!(intersection_plages(&[], Duration::minutes(5)), vec![]);
    }

    #[test]
    fn dates_de_serie_par_jours_et_intervalle() {
        let lundi = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        let d = dates_serie(lundi, &[1, 3], 1, lundi + Duration::days(13));
        assert_eq!(d.len(), 4, "{d:?}");
        let toutes_les_deux = dates_serie(lundi, &[1], 2, lundi + Duration::days(28));
        assert_eq!(
            toutes_les_deux,
            vec![
                lundi,
                lundi + Duration::days(14),
                lundi + Duration::days(28)
            ]
        );
        assert!(
            dates_serie(lundi, &[1], 1, lundi + Duration::days(900)).len() > SERIE_RESERVATION_MAX
        );
    }
}
