//! Socle 42, lot V2a : bordereau de remise de chèques (VOY-266), échéancier des acomptes fournisseurs (VOY-270),
//! réglages d'établissement de l'argent en mode suivi (R-SV45, S-438 ; `42_SORTIES_VOYAGES.md` §12.3, §32.5).
//!
//! * **Bordereau** : l'organisateur (le porteur) saisit les chèques qu'il a collectés (élève, numéro, montant), avec le
//!   nombre et le total qu'il annonce — le serveur vérifie qu'ils s'additionnent — et le **secrétariat** les reçoit
//!   **le jour même** (jour de l'établissement). L'écart entre bordereau et remise, et le retard, sont calculés par la
//!   base et **signalés** à la comptabilité. **Dans le public**, seul le régisseur manie les fonds : l'organisateur ne
//!   collecte pas. Un porteur ne reçoit pas son propre bordereau.
//! * **Acomptes fournisseurs** : à payer, bientôt, en retard, payé, annulé ; l'alerte avant échéance part **une seule
//!   fois** à la comptabilité, par un passage idempotent.
//! * **Événements** : identifiants et codes seulement, aucun montant, aucun nom.

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEvent, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages_argent::{centimes_de_texte, statut_acompte, VERSEMENT_MAX};
use crate::voyages_argent_repo::{
    jour_etab, peut_argent, peut_remise, pointer_tx, regime_public, reglages, verrou_sortie, Reglages, VersementSaisie,
    VersementVue,
};
use crate::voyages_repo::{charger_avec_acces, interdit, publier, refus, texte, texte_opt, tx};

/// États d'une sortie où un organisateur peut avoir collecté des chèques.
const ETATS_COLLECTE: [&str; 3] = ["ouverte", "prete", "en_cours"];
const CHEQUES_MAX: usize = 500;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct BordereauLigneSaisie {
    pub participant_id: Uuid,
    pub numero_cheque: String,
    pub montant: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BordereauSaisie {
    /// Le nombre et le total annoncés par l'organisateur : le serveur vérifie qu'ils correspondent aux lignes.
    pub nb_cheques: i32,
    pub total: String,
    pub lignes: Vec<BordereauLigneSaisie>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BordereauVue {
    pub id: Uuid,
    pub sortie_id: Uuid,
    pub jour_collecte: NaiveDate,
    pub nb_cheques: i32,
    pub total_centimes: i64,
    pub saisi_le: chrono::DateTime<chrono::Utc>,
    #[sqlx(skip)]
    pub remise: Option<RemiseVue>,
    /// Non remis le jour même (ou jamais) : à signaler.
    #[sqlx(skip)]
    pub en_attente: bool,
    #[sqlx(skip)]
    pub lignes: Vec<BordereauLigneVue>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RemiseVue {
    pub nb_recus: i32,
    pub total_recu_centimes: i64,
    pub jour_remise: NaiveDate,
    pub ecart: bool,
    pub en_retard: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BordereauLigneVue {
    pub id: Uuid,
    pub participant_id: Uuid,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub numero_cheque: String,
    pub montant_centimes: i64,
    /// Un versement a-t-il été pointé pour cette ligne ?
    pub pointee: bool,
    /// Réception à la remise : `Some(true)` reçue, `Some(false)` manquante, `None` pas encore remise (M4).
    pub recue: Option<bool>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BordereauAttente {
    pub bordereau_id: Uuid,
    pub sortie_id: Uuid,
    pub titre: String,
    pub jour_collecte: NaiveDate,
    pub nb_cheques: i32,
    pub total_centimes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcompteSaisie {
    pub prestataire_id: Uuid,
    #[serde(default)]
    pub commande_id: Option<Uuid>,
    pub libelle: String,
    pub montant: String,
    pub echeance: NaiveDate,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcompteVue {
    pub id: Uuid,
    pub prestataire_id: Uuid,
    pub raison_sociale: String,
    pub commande_id: Option<Uuid>,
    pub libelle: String,
    pub montant_centimes: i64,
    pub echeance: NaiveDate,
    pub statut: String,
    pub paye_le: Option<NaiveDate>,
    pub paye_centimes: Option<i64>,
    pub alerte_le: Option<NaiveDate>,
}

#[derive(FromRow)]
struct AcompteRow {
    id: Uuid,
    prestataire_id: Uuid,
    raison_sociale: String,
    commande_id: Option<Uuid>,
    libelle: String,
    montant_centimes: i64,
    echeance: NaiveDate,
    paye_le: Option<NaiveDate>,
    paye_centimes: Option<i64>,
    alerte_le: Option<NaiveDate>,
    annule_le: Option<NaiveDate>,
}

fn vue_acompte(r: AcompteRow, jour: NaiveDate, alerte_jours: i64) -> AcompteVue {
    let st = statut_acompte(r.echeance, r.paye_le.is_some(), r.annule_le.is_some(), jour, alerte_jours);
    AcompteVue {
        id: r.id,
        prestataire_id: r.prestataire_id,
        raison_sociale: r.raison_sociale,
        commande_id: r.commande_id,
        libelle: r.libelle,
        montant_centimes: r.montant_centimes,
        echeance: r.echeance,
        statut: st.code().to_string(),
        paye_le: r.paye_le,
        paye_centimes: r.paye_centimes,
        alerte_le: r.alerte_le,
    }
}

const SELECT_ACOMPTE: &str = "SELECT f.id, f.prestataire_id, p.raison_sociale, f.commande_id, f.libelle, f.montant_centimes,
        f.echeance, f.paye_le, f.paye_centimes, f.alerte_le, f.annule_le
   FROM ent_sortie_acompte_fournisseur f
   JOIN ent_prestataire p ON p.etablissement_id = f.etablissement_id AND p.id = f.prestataire_id";

// ───────────────────────── Réglages ─────────────────────────

impl PgEnt {
    /// Réglages de l'argent des sorties (collecte par l'organisateur, chèques-vacances, délai d'alerte).
    pub async fn voyages_argent_reglages(&self, a: &Acteur<'_>) -> Result<Reglages, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesArgent).await?;
        let mut t = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r = reglages(&mut t, a.etablissement_id).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    pub async fn voyages_argent_reglages_definir(&self, a: &Acteur<'_>, r: Reglages) -> Result<Reglages, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesArgent).await?;
        if !(1..=90).contains(&r.alerte_acompte_jours) {
            return Err(invalide());
        }
        let mut t = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_sortie_argent_reglage
                 (etablissement_id, collecte_organisateur, cheques_vacances, alerte_acompte_jours, modifie_par_user_id)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET collecte_organisateur = EXCLUDED.collecte_organisateur,
                    cheques_vacances = EXCLUDED.cheques_vacances,
                    alerte_acompte_jours = EXCLUDED.alerte_acompte_jours,
                    modifie_le = now(), modifie_par_user_id = EXCLUDED.modifie_par_user_id",
        )
        .bind(a.etablissement_id)
        .bind(r.collecte_organisateur)
        .bind(r.cheques_vacances)
        .bind(r.alerte_acompte_jours)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let nouveau = reglages(&mut t, a.etablissement_id).await?;
        t.commit().await.map_err(db)?;
        Ok(nouveau)
    }

    // ───────────────────────── Bordereaux (VOY-266) ─────────────────────────

    /// L'organisateur saisit un bordereau de chèques collectés. Refusé dans le **public** (régie seule) et quand
    /// l'établissement a désactivé la collecte par les organisateurs. Seul le **porteur** de la sortie saisit.
    pub async fn voyage_bordereau_saisir(&self, a: &Acteur<'_>, id: Uuid, b: BordereauSaisie) -> Result<BordereauVue, EntDbError> {
        if b.lignes.is_empty() || b.lignes.len() > CHEQUES_MAX {
            return Err(invalide());
        }
        let mut lignes = Vec::with_capacity(b.lignes.len());
        let mut total = 0i64;
        for l in &b.lignes {
            let m = centimes_de_texte(&l.montant).filter(|m| (1..=VERSEMENT_MAX).contains(m)).ok_or_else(invalide)?;
            lignes.push((l.participant_id, texte(&l.numero_cheque, 30)?, m));
            total = total.checked_add(m).ok_or_else(invalide)?;
        }
        // Double saisie : le nombre et le total annoncés doivent être ceux des lignes.
        let annonce = centimes_de_texte(&b.total).ok_or_else(invalide)?;
        if usize::try_from(b.nb_cheques).ok() != Some(lignes.len()) || annonce != total {
            return Err(refus(
                RefusMetier::Invalide,
                "Le nombre ou le total annoncé ne correspond pas aux chèques saisis : recomptez le bordereau.",
            ));
        }
        let mut numeros: Vec<&str> = lignes.iter().map(|l| l.1.as_str()).collect();
        numeros.sort_unstable();
        numeros.dedup();
        if numeros.len() != lignes.len() {
            return Err(refus(RefusMetier::Invalide, "Deux chèques portent le même numéro."));
        }
        // F-a (contre-revue du pilote) : parent et élève écartés avant de prendre le verrou de la sortie.
        self.avant_verrou(a, None).await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.porteur {
            return Err(interdit());
        }
        match regime_public(&mut t, a.etablissement_id).await? {
            // M6 : sans entité de facturation, on ne sait pas si l'établissement est public : par sécurité, refusé.
            None => {
                return Err(refus(
                    RefusMetier::Interdit,
                    "Aucune entité de facturation n'est configurée : renseignez le régime de l'établissement (privé sous contrat ou public) dans les réglages de facturation avant d'autoriser la collecte de chèques par un organisateur.",
                ))
            }
            Some(true) => {
                return Err(refus(
                    RefusMetier::Interdit,
                    "Dans un établissement public, seul le régisseur manie les fonds : l'organisateur ne collecte pas.",
                ))
            }
            Some(false) => {}
        }
        if !reglages(&mut t, a.etablissement_id).await?.collecte_organisateur {
            return Err(refus(RefusMetier::Interdit, "Cet établissement ne confie pas la collecte de chèques aux organisateurs."));
        }
        if s.nature == "obligatoire" || !ETATS_COLLECTE.contains(&s.etat.as_str()) {
            return Err(refus(RefusMetier::Conflit, "Aucune collecte n'est attendue à cette étape de la sortie."));
        }
        // Chaque chèque vient d'un participant de CETTE sortie, encore inscrit.
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.0).collect();
        let ok: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT id) FROM ent_sortie_participant
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = ANY($3) AND etat = ANY($4)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(&ids)
        .bind(crate::voyages_argent_repo::ETATS_PAYANTS.iter().map(|x| x.to_string()).collect::<Vec<_>>())
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let distincts = {
            let mut d = ids.clone();
            d.sort_unstable();
            d.dedup();
            d.len() as i64
        };
        if ok != distincts {
            return Err(introuvable());
        }
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let bid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_bordereau (id, etablissement_id, sortie_id, jour_collecte, nb_cheques, total_centimes, saisi_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(bid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(jour)
        .bind(lignes.len() as i32)
        .bind(total)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        for (participant, numero, montant) in &lignes {
            sqlx::query(
                "INSERT INTO ent_sortie_bordereau_ligne (id, etablissement_id, bordereau_id, participant_id, numero_cheque, montant_centimes)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(bid)
            .bind(participant)
            .bind(numero)
            .bind(montant)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let vue = lire_bordereau(&mut t, a, bid, true, jour).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Le secrétariat (ou la comptabilité) reçoit le bordereau en **cochant chaque chèque reçu** (M4) : les lignes
    /// non cochées sont « manquantes ». Le nombre et le total reçus sont ceux des lignes cochées ; l'écart et le retard
    /// sont calculés par la base ; s'il y en a un, la comptabilité est **prévenue** (événement sans montant).
    pub async fn voyage_bordereau_remettre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        bordereau_id: Uuid,
        recues: Vec<Uuid>,
        note: Option<&str>,
    ) -> Result<BordereauVue, EntDbError> {
        let note = texte_opt(note, 500)?;
        self.avant_verrou(a, None).await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_remise(&acces) {
            return Err(interdit());
        }
        let saisi_par: Uuid = sqlx::query_scalar(
            "SELECT saisi_par_user_id FROM ent_sortie_bordereau
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(bordereau_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        // Séparation des tâches : on ne reçoit pas son propre bordereau.
        // Transverse « individu » (659) : ni la personne qui a saisi, ni un autre de ses comptes.
        if crate::individu::meme_individu(&mut t, a.etablissement_id, saisi_par, a.user_id).await? {
            return Err(refus(RefusMetier::Interdit, "Le bordereau est remis à une autre personne que celle qui l'a saisi."));
        }
        let lignes: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT id, montant_centimes FROM ent_sortie_bordereau_ligne
              WHERE etablissement_id = $1 AND bordereau_id = $2 ORDER BY id",
        )
        .bind(a.etablissement_id)
        .bind(bordereau_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        // M-3 (contre-revue du pilote, D-069) : recevoir un bordereau est un pouvoir sur l'argent (il fixe les écarts et
        // les alertes) : la personne qui reçoit n'est liée à aucun élève ni payeuse d'aucun compte des chèques du bordereau.
        let participants: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT participant_id FROM ent_sortie_bordereau_ligne WHERE etablissement_id = $1 AND bordereau_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(bordereau_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        crate::voyages_argent_v3::exiger_libre_des_participants(&mut t, a.etablissement_id, a.user_id, &participants).await?;
        let mut cochees = recues.clone();
        cochees.sort_unstable();
        cochees.dedup();
        if cochees.len() != recues.len() || cochees.iter().any(|c| !lignes.iter().any(|(l, _)| l == c)) {
            return Err(introuvable());
        }
        let mut total = 0i64;
        for (l, m) in &lignes {
            if cochees.contains(l) {
                total = total.checked_add(*m).ok_or_else(invalide)?;
            }
        }
        let nb_recus = cochees.len() as i32;
        let ecart = nb_recus as usize != lignes.len();
        if ecart && note.is_none() {
            return Err(refus(RefusMetier::Invalide, "Un chèque manquant se documente : ajoutez une note."));
        }
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let r = sqlx::query(
            "INSERT INTO ent_sortie_bordereau_remise
                 (id, etablissement_id, bordereau_id, nb_recus, total_recu_centimes, jour_remise, note, recu_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(bordereau_id)
        .bind(nb_recus)
        .bind(total)
        .bind(jour)
        .bind(&note)
        .bind(a.user_id)
        .execute(&mut *t)
        .await;
        match r {
            Ok(_) => {}
            Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23505") => return Err(conflit()),
            Err(e) => return Err(db(e)),
        }
        for (l, _) in &lignes {
            sqlx::query(
                "INSERT INTO ent_sortie_bordereau_reception (id, etablissement_id, bordereau_id, ligne_id, recue)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(bordereau_id)
            .bind(l)
            .bind(cochees.contains(l))
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let vue = lire_bordereau(&mut t, a, bordereau_id, true, jour).await?;
        let mut evs: Vec<MetierEvent> = Vec::new();
        if let Some(rem) = &vue.remise {
            if rem.ecart || rem.en_retard {
                evs.push(
                    publier(
                        &mut t,
                        a,
                        "voyages.bordereau_signale",
                        s.id,
                        json!({ "bordereau_id": bordereau_id, "code": if rem.ecart { "ecart" } else { "en_retard" } }),
                    )
                    .await?,
                );
            }
        }
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evs);
        Ok(vue)
    }

    /// Pointe une ligne de bordereau reçue : crée le versement (chèque) sur la part du **payeur choisi** par la
    /// personne qui pointe (un élève peut avoir deux payeurs : le chèque va au bon). Une ligne ne se pointe qu'une fois.
    pub async fn voyage_bordereau_ligne_pointer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        ligne_id: Uuid,
        compte_id: Uuid,
    ) -> Result<VersementVue, EntDbError> {
        let r = self.voyage_bordereau_ligne_pointer_corps(a, id, ligne_id, compte_id).await;
        self.tracer_refus_regle3(a, Some(id), "voyage_bordereau_ligne_pointer", r).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_bordereau_ligne_pointer_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        ligne_id: Uuid,
        compte_id: Uuid,
    ) -> Result<VersementVue, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, None).await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_remise(&acces) {
            return Err(interdit());
        }
        let l: (Uuid, String, i64, NaiveDate) = sqlx::query_as(
            "SELECT l.participant_id, l.numero_cheque, l.montant_centimes, r.jour_remise
               FROM ent_sortie_bordereau_ligne l
               JOIN ent_sortie_bordereau b ON b.etablissement_id = l.etablissement_id AND b.id = l.bordereau_id
               JOIN ent_sortie_bordereau_remise r ON r.etablissement_id = b.etablissement_id AND r.bordereau_id = b.id
               JOIN ent_sortie_bordereau_reception x ON x.etablissement_id = l.etablissement_id AND x.ligne_id = l.id AND x.recue
              WHERE l.etablissement_id = $1 AND l.id = $2 AND b.sortie_id = $3",
        )
        .bind(a.etablissement_id)
        .bind(ligne_id)
        .bind(s.id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let saisie = VersementSaisie {
            participant_id: l.0,
            compte_id,
            moyen: "cheque".into(),
            montant: crate::voyages_argent::euros(l.2),
            date_valeur: Some(l.3),
            reference: Some(l.1),
        };
        let v = pointer_tx(&mut t, a, &s, &saisie, Some(ligne_id)).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Bordereaux d'une sortie : l'organisateur voit les siens (avec leurs lignes), le secrétariat et la comptabilité
    /// tous. Un bordereau est « en attente » s'il n'a pas été remis (le jour même ou plus tard).
    pub async fn voyage_bordereaux(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<BordereauVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.porteur || peut_remise(&acces)) {
            return Err(introuvable());
        }
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_sortie_bordereau WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY saisi_le, id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for b in ids {
            out.push(lire_bordereau(&mut t, a, b, true, jour).await?);
        }
        self.acces_nominatif(&mut t, a, Some(s.id), "bordereaux", out.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Tous les bordereaux de l'établissement non remis alors que leur jour de collecte est passé : à relancer.
    pub async fn voyages_bordereaux_en_attente(&self, a: &Acteur<'_>) -> Result<Vec<BordereauAttente>, EntDbError> {
        let droits = self.voyages_droits(a).await?;
        if !(droits.contains(&Action::VoyagesArgent) || droits.contains(&Action::VoyagesRemise)) {
            return Err(interdit());
        }
        let mut t = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let v = sqlx::query_as::<_, BordereauAttente>(
            "SELECT b.id AS bordereau_id, b.sortie_id, s.titre, b.jour_collecte, b.nb_cheques, b.total_centimes
               FROM ent_sortie_bordereau b
               JOIN ent_sortie s ON s.etablissement_id = b.etablissement_id AND s.id = b.sortie_id
              WHERE b.etablissement_id = $1 AND b.jour_collecte < $2
                AND NOT EXISTS (SELECT 1 FROM ent_sortie_bordereau_remise r
                                 WHERE r.etablissement_id = b.etablissement_id AND r.bordereau_id = b.id)
              ORDER BY b.jour_collecte, b.id",
        )
        .bind(a.etablissement_id)
        .bind(jour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, None, "bordereaux", v.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Acomptes fournisseurs (VOY-270) ─────────────────────────

    pub async fn voyage_acompte_creer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        f: AcompteSaisie,
    ) -> Result<AcompteVue, EntDbError> {
        self.voyage_acompte_creer_avec(a, id, f, None, Utc::now()).await
    }

    /// V3b (Moyen 3) : avec un contournement autorisé par une seconde personne libre (`voyages_contournement`), consommé
    /// par cette écriture seule. Horloge passée en paramètre.
    pub async fn voyage_acompte_creer_avec(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        f: AcompteSaisie,
        contournement: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<AcompteVue, EntDbError> {
        let r = self.voyage_acompte_creer_corps(a, id, f, contournement, maintenant).await;
        self.tracer_refus_regle3(a, Some(id), "voyage_acompte_creer", r).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_acompte_creer_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        f: AcompteSaisie,
        contournement: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<AcompteVue, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent)).await?;
        let montant = centimes_de_texte(&f.montant).filter(|m| *m > 0 && *m <= 100_000_000_000).ok_or_else(invalide)?;
        let libelle = texte(&f.libelle, 200)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        // M-3 : un acompte fournisseur engage l'argent de la sortie : jamais pour une sortie où son propre enfant part.
        let nid = Uuid::new_v4();
        crate::voyages_contournement::exiger_libre_ou_contourne(&mut t, a, s.id, "acompte_creer", contournement, nid, Some(montant), Some(f.prestataire_id), maintenant).await?;
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        if f.echeance < jour - chrono::Duration::days(366) || f.echeance > jour + chrono::Duration::days(3 * 366) {
            return Err(invalide());
        }
        sqlx::query(
            "INSERT INTO ent_sortie_acompte_fournisseur
                 (id, etablissement_id, sortie_id, prestataire_id, commande_id, libelle, montant_centimes, echeance, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(f.prestataire_id)
        .bind(f.commande_id)
        .bind(&libelle)
        .bind(montant)
        .bind(f.echeance)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(|e| match e {
            // Prestataire ou commande d'un autre établissement ou inexistant : « introuvable », jamais de fuite.
            sqlx::Error::Database(d) if d.code().as_deref() == Some("23503") => introuvable(),
            e => crate::voyages_argent_v3::db_pouvoir(e),
        })?;
        let alerte = i64::from(reglages(&mut t, a.etablissement_id).await?.alerte_acompte_jours);
        let r: AcompteRow = sqlx::query_as(&format!("{SELECT_ACOMPTE} WHERE f.etablissement_id = $1 AND f.id = $2"))
            .bind(a.etablissement_id)
            .bind(nid)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(vue_acompte(r, jour, alerte))
    }

    pub async fn voyage_acomptes(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<AcompteVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let alerte = i64::from(reglages(&mut t, a.etablissement_id).await?.alerte_acompte_jours);
        let rows: Vec<AcompteRow> = sqlx::query_as(&format!(
            "{SELECT_ACOMPTE} WHERE f.etablissement_id = $1 AND f.sortie_id = $2 ORDER BY f.echeance, f.id"
        ))
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows.into_iter().map(|r| vue_acompte(r, jour, alerte)).collect())
    }

    /// Marque un acompte payé (une fois). Le montant payé peut différer du montant dû (frais, change) : il est écrit.
    pub async fn voyage_acompte_payer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        acompte_id: Uuid,
        montant: Option<&str>,
        date: Option<NaiveDate>,
        reference: Option<&str>,
    ) -> Result<AcompteVue, EntDbError> {
        self.voyage_acompte_payer_avec(a, id, acompte_id, montant, date, reference, None, Utc::now()).await
    }

    /// V3b (Moyen 3) : variante avec contournement tracé (voir `voyage_acompte_creer_avec`).
    #[allow(clippy::too_many_arguments)]
    pub async fn voyage_acompte_payer_avec(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        acompte_id: Uuid,
        montant: Option<&str>,
        date: Option<NaiveDate>,
        reference: Option<&str>,
        contournement: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<AcompteVue, EntDbError> {
        let r = self
            .voyage_acompte_payer_corps(a, id, acompte_id, montant, date, reference, contournement, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_acompte_payer", r).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_acompte_payer_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        acompte_id: Uuid,
        montant: Option<&str>,
        date: Option<NaiveDate>,
        reference: Option<&str>,
        contournement: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<AcompteVue, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent)).await?;
        let reference = texte_opt(reference, 60)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        // M-3 : payer un acompte est un pouvoir sur l'argent de la sortie.
        crate::voyages_contournement::exiger_libre_ou_contourne(&mut t, a, s.id, "acompte_payer", contournement, acompte_id, montant.and_then(centimes_de_texte), None, maintenant).await?;
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let (du, paye, annule): (i64, Option<NaiveDate>, Option<NaiveDate>) = sqlx::query_as(
            "SELECT montant_centimes, paye_le, annule_le FROM ent_sortie_acompte_fournisseur
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(acompte_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if paye.is_some() || annule.is_some() {
            return Err(conflit());
        }
        let m = match montant {
            Some(x) => centimes_de_texte(x).filter(|m| *m > 0 && *m <= 100_000_000_000).ok_or_else(invalide)?,
            None => du,
        };
        let date = date.unwrap_or(jour);
        if date > jour {
            return Err(refus(RefusMetier::Invalide, "La date d'un paiement ne peut pas être dans le futur."));
        }
        sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur
                SET paye_le = $3, paye_centimes = $4, paye_reference = $5, paye_par_user_id = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(acompte_id)
        .bind(date)
        .bind(m)
        .bind(&reference)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(crate::voyages_argent_v3::db_pouvoir)?;
        let alerte = i64::from(reglages(&mut t, a.etablissement_id).await?.alerte_acompte_jours);
        let r: AcompteRow = sqlx::query_as(&format!("{SELECT_ACOMPTE} WHERE f.etablissement_id = $1 AND f.id = $2"))
            .bind(a.etablissement_id)
            .bind(acompte_id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(vue_acompte(r, jour, alerte))
    }

    /// Annule un acompte non payé, avec motif (jamais de suppression).
    pub async fn voyage_acompte_annuler(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        acompte_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.voyage_acompte_annuler_avec(a, id, acompte_id, motif, None, Utc::now()).await
    }

    /// V3b (Moyen 3) : variante avec contournement tracé (voir `voyage_acompte_creer_avec`).
    pub async fn voyage_acompte_annuler_avec(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        acompte_id: Uuid,
        motif: &str,
        contournement: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let r = self.voyage_acompte_annuler_corps(a, id, acompte_id, motif, contournement, maintenant).await;
        self.tracer_refus_regle3(a, Some(id), "voyage_acompte_annuler", r).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_acompte_annuler_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        acompte_id: Uuid,
        motif: &str,
        contournement: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent)).await?;
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        // M-3 : annuler un acompte est un pouvoir sur l'argent de la sortie ; l'auteur est gardé en base (656, `annule_par_user_id`).
        crate::voyages_contournement::exiger_libre_ou_contourne(&mut t, a, s.id, "acompte_annuler", contournement, acompte_id, None, None, maintenant).await?;
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let r = sqlx::query(
            "UPDATE ent_sortie_acompte_fournisseur SET annule_le = $4, annule_motif = $5, annule_par_user_id = $6
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 AND paye_le IS NULL AND annule_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(acompte_id)
        .bind(jour)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(crate::voyages_argent_v3::db_pouvoir)?;
        if r.rows_affected() == 0 {
            return Err(conflit());
        }
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Passage d'alerte pour UN établissement : les acomptes à payer bientôt ou en retard, **pas encore signalés**,
    /// le sont (une seule fois) à la comptabilité. Idempotent ; retourne le nombre d'alertes émises.
    pub async fn voyages_acomptes_alertes_traiter(&self, etab: Uuid) -> Result<usize, EntDbError> {
        self.voyages_acomptes_alertes_traiter_au(etab, None).await
    }

    /// Même passage pour un jour donné (planificateur et tests : aucune heure réelle) ; `None` : le jour de
    /// l'établissement.
    pub async fn voyages_acomptes_alertes_traiter_au(
        &self,
        etab: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<usize, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = match jour {
            Some(j) => j,
            None => jour_etab(&mut t, etab).await?,
        };
        let fenetre = i64::from(reglages(&mut t, etab).await?.alerte_acompte_jours);
        let rows: Vec<(Uuid, Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT id, sortie_id, echeance FROM ent_sortie_acompte_fournisseur
              WHERE etablissement_id = $1 AND paye_le IS NULL AND annule_le IS NULL AND alerte_le IS NULL
                AND echeance <= $2 + ($3::int * interval '1 day')::interval
              ORDER BY echeance, id FOR UPDATE",
        )
        .bind(etab)
        .bind(jour)
        .bind(fenetre as i32)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut evs = Vec::new();
        for (id, sortie, echeance) in &rows {
            let r = sqlx::query(
                "UPDATE ent_sortie_acompte_fournisseur SET alerte_le = $3
                  WHERE etablissement_id = $1 AND id = $2 AND alerte_le IS NULL",
            )
            .bind(etab)
            .bind(id)
            .bind(jour)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                let code = if *echeance < jour { "en_retard" } else { "bientot" };
                evs.push(
                    t.publier(MetierEventDraft {
                        etablissement_id: etab,
                        type_name: "voyages.acompte_echeance".to_string(),
                        aggregate: AggregateKind::Voyage,
                        aggregate_id: *sortie,
                        eleve_id: None,
                        payload: json!({ "acompte_id": id, "code": code }),
                        actor_user_id: None,
                    })
                    .await?,
                );
            }
        }
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evs);
        Ok(evs.len())
    }

    /// Passage manuel par la comptabilité ou la direction, sur son établissement.
    pub async fn voyages_acomptes_alertes_declencher(&self, a: &Acteur<'_>) -> Result<usize, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesArgent).await?;
        self.voyages_acomptes_alertes_traiter(a.etablissement_id).await
    }

    /// Passage planifié sur **tous** les établissements (une fois par jour, à côté des rappels).
    pub async fn voyages_acomptes_alertes_traiter_tous(&self) -> Result<usize, EntDbError> {
        let etabs = crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("voyages_acomptes"),
        )
        .await
        .map_err(db)?;
        let mut n = 0;
        for e in etabs {
            n += self.voyages_acomptes_alertes_traiter(e).await?;
        }
        Ok(n)
    }
}

async fn lire_bordereau(
    conn: &mut sqlx::PgConnection,
    a: &Acteur<'_>,
    id: Uuid,
    avec_lignes: bool,
    jour: NaiveDate,
) -> Result<BordereauVue, EntDbError> {
    let mut b: BordereauVue = sqlx::query_as(
        "SELECT id, sortie_id, jour_collecte, nb_cheques, total_centimes, saisi_le
           FROM ent_sortie_bordereau WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    b.remise = sqlx::query_as(
        "SELECT nb_recus, total_recu_centimes, jour_remise, ecart, en_retard, note
           FROM ent_sortie_bordereau_remise WHERE etablissement_id = $1 AND bordereau_id = $2",
    )
    .bind(a.etablissement_id)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    b.en_attente = b.remise.is_none() && b.jour_collecte < jour;
    if avec_lignes {
        b.lignes = sqlx::query_as(
            "SELECT l.id, l.participant_id, e.nom AS eleve_nom, e.prenom AS eleve_prenom, l.numero_cheque, l.montant_centimes,
                    EXISTS (SELECT 1 FROM ent_sortie_versement v
                             WHERE v.etablissement_id = l.etablissement_id AND v.bordereau_ligne_id = l.id) AS pointee,
                    (SELECT r.recue FROM ent_sortie_bordereau_reception r
                      WHERE r.etablissement_id = l.etablissement_id AND r.ligne_id = l.id) AS recue
               FROM ent_sortie_bordereau_ligne l
               JOIN ent_sortie_participant sp ON sp.etablissement_id = l.etablissement_id AND sp.id = l.participant_id
               JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
              WHERE l.etablissement_id = $1 AND l.bordereau_id = $2 ORDER BY l.numero_cheque, l.id",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(b)
}
