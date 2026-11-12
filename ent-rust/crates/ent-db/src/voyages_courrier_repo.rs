//! Socle 42, lot V0c : boîte de réception des devis par **établissement** et file de tri
//! (`docs/socles/42_SORTIES_VOYAGES.md` §13.3 ; VOY-163, VOY-164, NX-46, NX-47, NX-70).
//!
//! * Le dépôt d'un courrier est une **fonction de service** appelée par le relevé de la boîte de
//!   l'établissement (le choix de l'outil de relevé, Q-V7, reste ouvert) : elle n'est exposée par
//!   **aucune route HTTP** (corrige NJ-28 : « n'importe qui peut écrire à la boîte »).
//! * Rattachement **automatique** seulement si la référence **exacte** de la sortie (somme de contrôle
//!   comprise) **et** un expéditeur de l'annuaire à qui une demande a été envoyée pour **cette** sortie
//!   sont réunis ; tout le reste va à la file de tri. Jamais de devinette, jamais entre établissements
//!   (chaque lecture est cloisonnée par l'établissement de la boîte et par la RLS).
//! * Un courrier n'est **jamais** marqué traité sans rattachement confirmé : « ignoré » est un geste
//!   humain tracé (corrige VD-45 : perte silencieuse).
//! * Le contenu n'est transmis à **aucune IA** et ne part jamais en copie (corrige NJ-28, NJ-70) ;
//!   le montant TTC n'est que **proposé** à partir du texte, un humain le confirme.
//! * Les pièces passent par la même politique de dépôt que les autres fichiers (type détecté sur les
//!   octets, plafond, analyse antivirus) ; les liens du message ne sont jamais suivis.

use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use serde::Serialize;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{db, introuvable, Acteur};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages::est_terminal;
use crate::voyages_prestataires::{
    adresse_valide, domaine_voisin, decider_rattachement, ligne_propre, montant_ttc_propose,
    references_candidates, reference_courrier, Rattachement,
};
use crate::voyages_prestataires_repo::{PrestataireSaisie, PrestataireVue};
use crate::voyages_repo::{interdit, journaliser, publier, refus, texte_opt, tx, Fait};

const PIECES_MAX: usize = 10;

#[derive(Debug, Clone)]
pub struct PieceCourrier {
    pub nom: String,
    pub mime: Option<String>,
    pub contenu: Vec<u8>,
}

/// Courrier relevé dans la boîte de l'établissement.
#[derive(Debug, Clone)]
pub struct CourrierEntrant {
    /// Identifiant du message côté serveur de courrier : le dépôt est **idempotent**.
    pub message_externe_id: String,
    pub recu_le: DateTime<Utc>,
    pub expediteur: String,
    pub objet: String,
    /// Texte brut du message (jamais du HTML interprété).
    pub corps: String,
    pub pieces: Vec<PieceCourrier>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourrierRecu {
    pub id: Uuid,
    pub doublon: bool,
    pub statut: String,
    pub sortie_id: Option<Uuid>,
    pub raison: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CourrierVue {
    pub id: Uuid,
    pub recu_le: DateTime<Utc>,
    pub expediteur_email: String,
    pub objet: String,
    pub extrait: String,
    pub nb_pieces: i64,
    pub pieces_refusees: i32,
    pub alerte_domaine_voisin: bool,
    pub raison: Option<String>,
    /// Prestataire de l'annuaire dont l'adresse est celle de l'expéditeur, s'il y en a un.
    pub prestataire_connu_id: Option<Uuid>,
    pub prestataire_connu: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PieceCourrierVue {
    pub id: Uuid,
    pub nom: String,
    pub taille: i32,
}

async fn prestataire_de_adresse(conn: &mut PgConnection, etab: Uuid, email: &str) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT prestataire_id FROM ent_prestataire_adresse
          WHERE etablissement_id = $1 AND email = $2 AND desactivee_le IS NULL",
    )
    .bind(etab)
    .bind(email)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// Crée le devis issu d'un courrier : montant **proposé** (jamais retenu), pièce PDF reprise du courrier.
async fn devis_depuis_courrier(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Option<Uuid>,
    sortie: Uuid,
    prestataire: Uuid,
    courrier: Uuid,
    propose: Option<i64>,
) -> Result<Uuid, EntDbError> {
    let piece: Option<(String, String, String, i32, String)> = sqlx::query_as(
        "SELECT cle_objet, nom, sha256, taille, mime FROM ent_courrier_piece
          WHERE etablissement_id = $1 AND courrier_id = $2 AND mime = 'application/pdf' ORDER BY cree_le, id LIMIT 1",
    )
    .bind(etab)
    .bind(courrier)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let document = match piece {
        Some((cle, nom, sha, taille, _)) => {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_sortie_document (id, etablissement_id, sortie_id, type, cle_objet, nom, taille, sha256, emis_par_user_id)
                 VALUES ($1,$2,$3,'devis',$4,$5,$6,$7,$8)",
            )
            .bind(id)
            .bind(etab)
            .bind(sortie)
            .bind(cle)
            .bind(nom)
            .bind(taille)
            .bind(sha)
            .bind(user)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            Some(id)
        }
        None => None,
    };
    let commande: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_sortie_commande WHERE etablissement_id = $1 AND sortie_id = $2 AND statut <> 'annulee')",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_devis
             (id, etablissement_id, sortie_id, prestataire_id, origine, courrier_id, document_id,
              montant_propose_centimes, statut, cree_par_user_id)
         VALUES ($1,$2,$3,$4,'courrier',$5,$6,$7,$8,$9)",
    )
    .bind(id)
    .bind(etab)
    .bind(sortie)
    .bind(prestataire)
    .bind(courrier)
    .bind(document)
    .bind(propose)
    .bind(if commande { "apres_commande" } else { "a_confirmer" })
    .bind(user)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(id)
}

impl PgEnt {
    /// Dépose un courrier relevé dans la boîte de **cet** établissement. Rattachement automatique
    /// seulement dans le cas certain (référence exacte et expéditeur sollicité) ; sinon file de tri.
    /// Idempotent sur l'identifiant du message.
    pub async fn courrier_entrant_deposer(&self, etab: Uuid, c: CourrierEntrant) -> Result<CourrierRecu, EntDbError> {
        let message = ligne_propre(&c.message_externe_id, 300);
        if message.is_empty() {
            return Err(refus(RefusMetier::Invalide, "Identifiant de message manquant."));
        }
        let brut = c.expediteur.trim().to_lowercase();
        let (expediteur, domaine) = match adresse_valide(&brut) {
            Some(x) => x,
            None => (
                ligne_propre(&brut, 254),
                "invalide".to_string(),
            ),
        };
        let objet = ligne_propre(&c.objet, 300);
        let extrait: String = c.corps.chars().map(|ch| if ch.is_control() && ch != '\n' { ' ' } else { ch }).take(2000).collect();
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some((id, statut, sortie, raison)) = sqlx::query_as::<_, (Uuid, String, Option<Uuid>, Option<String>)>(
            "SELECT id, statut, sortie_id, raison FROM ent_courrier_entrant WHERE etablissement_id = $1 AND message_externe_id = $2",
        )
        .bind(etab)
        .bind(&message)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        {
            t.commit().await.map_err(db)?;
            return Ok(CourrierRecu { id, doublon: true, statut, sortie_id: sortie, raison });
        }
        // Sortie visée par la référence exacte (cette boîte, cet établissement).
        let mut par_reference: Option<(Uuid, String)> = None;
        for (base, somme) in references_candidates(&format!("{objet}\n{}", c.corps)) {
            let s: Option<(Uuid, String)> = sqlx::query_as(
                "SELECT id, etat FROM ent_sortie WHERE etablissement_id = $1 AND upper(reference) = $2",
            )
            .bind(etab)
            .bind(&base)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            if let (Some((sid, etat)), Some(somme)) = (s, somme) {
                let complete = reference_courrier(&base, etab, sid);
                if complete.ends_with(&format!("-{somme}")) && !est_terminal(&etat) {
                    par_reference = Some((sid, etat));
                    break;
                }
            }
        }
        let expediteur_presta = prestataire_de_adresse(&mut t, etab, &expediteur).await?;
        let demande_envoyee = match (&par_reference, expediteur_presta) {
            (Some((sid, _)), Some(p)) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_envoi WHERE etablissement_id = $1 AND sortie_id = $2
                                 AND prestataire_id = $3 AND type = 'demande_devis' AND statut = 'envoye')",
            )
            .bind(etab)
            .bind(sid)
            .bind(p)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?,
            _ => false,
        };
        let decision = decider_rattachement(par_reference.as_ref().map(|x| x.0), expediteur_presta, demande_envoyee);
        // Alerte d'hameçonnage : domaine voisin d'un domaine connu de l'annuaire.
        let mut alerte = false;
        if expediteur_presta.is_none() {
            let connus: Vec<String> = sqlx::query_scalar(
                "SELECT DISTINCT domaine FROM ent_prestataire_adresse WHERE etablissement_id = $1 AND desactivee_le IS NULL",
            )
            .bind(etab)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            alerte = connus.iter().any(|d| domaine_voisin(&domaine, d));
        }
        let id = Uuid::new_v4();
        let (statut, sortie, presta, mode, raison): (&str, Option<Uuid>, Option<Uuid>, Option<&str>, &str) = match decision {
            Rattachement::Auto { sortie, prestataire } => ("rattache", Some(sortie), Some(prestataire), Some("auto"), "reference_et_expediteur"),
            Rattachement::ATrier { raison } => ("a_trier", None, expediteur_presta, None, raison),
        };
        sqlx::query(
            "INSERT INTO ent_courrier_entrant
                 (id, etablissement_id, message_externe_id, recu_le, expediteur_email, expediteur_domaine, objet, extrait,
                  statut, sortie_id, prestataire_id, mode, raison, alerte_domaine_voisin)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(id)
        .bind(etab)
        .bind(&message)
        .bind(c.recu_le)
        .bind(&expediteur)
        .bind(&domaine)
        .bind(&objet)
        .bind(&extrait)
        .bind(statut)
        .bind(sortie)
        .bind(presta)
        .bind(mode)
        .bind(raison)
        .bind(alerte)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        // Pièces : politique de dépôt commune ; une pièce refusée est comptée, jamais ouverte.
        let mut refusees = 0;
        for (i, p) in c.pieces.iter().enumerate() {
            if i >= PIECES_MAX {
                refusees += 1;
                continue;
            }
            match self
                .politique_fichiers()
                .valider(UsageFichier::Document, &p.nom, p.mime.as_deref(), &p.contenu)
            {
                Ok(v) => {
                    let pid = Uuid::new_v4();
                    let cle = ObjectStore::key_for(etab, "courrier", pid, v.extension);
                    let info = self.objects().put(&cle, &p.contenu)?;
                    crate::objets::enregistrer_objet(&mut *t, etab, &info, Some(v.mime)).await?;
                    sqlx::query(
                        "INSERT INTO ent_courrier_piece (id, etablissement_id, courrier_id, nom, mime, taille, sha256, cle_objet)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
                    )
                    .bind(pid)
                    .bind(etab)
                    .bind(id)
                    .bind(ligne_propre(&v.nom, 200))
                    .bind(v.mime)
                    .bind(p.contenu.len() as i32)
                    .bind(&info.sha256)
                    .bind(&cle)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                }
                Err(_) => refusees += 1,
            }
        }
        if refusees > 0 {
            sqlx::query("UPDATE ent_courrier_entrant SET pieces_refusees = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(refusees)
                .execute(&mut *t)
                .await
                .map_err(db)?;
        }
        let mut events = Vec::new();
        if let (Some(s), Some(p)) = (sortie, presta) {
            let propose = montant_ttc_propose(&c.corps);
            devis_depuis_courrier(&mut t, etab, None, s, p, id, propose).await?;
            sqlx::query(
                "INSERT INTO ent_sortie_journal (id, etablissement_id, sortie_id, acteur_user_id, action, apres)
                 VALUES ($1,$2,$3,NULL,'courrier_rattache',$4)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(s)
            .bind("auto : référence exacte et expéditeur sollicité")
            .execute(&mut *t)
            .await
            .map_err(db)?;
            events.push(
                (&mut *t as &mut PgConnection)
                    .publier(MetierEventDraft {
                        etablissement_id: etab,
                        type_name: "voyages.courrier_rattache".into(),
                        aggregate: AggregateKind::Voyage,
                        aggregate_id: s,
                        eleve_id: None,
                        payload: json!({ "mode": "auto" }),
                        actor_user_id: None,
                    })
                    .await?,
            );
        }
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        Ok(CourrierRecu { id, doublon: false, statut: statut.to_string(), sortie_id: sortie, raison: Some(raison.to_string()) })
    }

    /// NX-47 (VOY-163) : file de tri de l'établissement (droit d'établissement `voyages.prestataires`).
    pub async fn courrier_tri_liste(&self, a: &Acteur<'_>) -> Result<Vec<CourrierVue>, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let v = sqlx::query_as::<_, CourrierVue>(
            "SELECT c.id, c.recu_le, c.expediteur_email, c.objet, left(c.extrait, 500) AS extrait,
                    (SELECT count(*) FROM ent_courrier_piece p WHERE p.etablissement_id = c.etablissement_id AND p.courrier_id = c.id) AS nb_pieces,
                    c.pieces_refusees, c.alerte_domaine_voisin, c.raison, c.prestataire_id AS prestataire_connu_id,
                    pr.raison_sociale AS prestataire_connu
               FROM ent_courrier_entrant c
               LEFT JOIN ent_prestataire pr ON pr.etablissement_id = c.etablissement_id AND pr.id = c.prestataire_id
              WHERE c.etablissement_id = $1 AND c.statut = 'a_trier'
              ORDER BY c.recu_le, c.id LIMIT 200",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Nombre de courriers à trier (badge).
    pub async fn courrier_tri_compte(&self, a: &Acteur<'_>) -> Result<i64, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_courrier_entrant WHERE etablissement_id = $1 AND statut = 'a_trier'",
        )
        .bind(a.etablissement_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(n)
    }

    /// NX-46 (VOY-164) : rattache **à la main** un courrier à une sortie et à un prestataire de l'annuaire.
    pub async fn courrier_rattacher(
        &self,
        a: &Acteur<'_>,
        courrier: Uuid,
        sortie: Uuid,
        prestataire: Uuid,
    ) -> Result<Uuid, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let c: Option<(String, String)> = sqlx::query_as(
            "SELECT statut, extrait FROM ent_courrier_entrant WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (statut, extrait) = c.ok_or_else(introuvable)?;
        if statut != "a_trier" {
            return Err(refus(RefusMetier::Conflit, "Ce courrier est déjà traité."));
        }
        let etat: Option<String> =
            sqlx::query_scalar("SELECT etat FROM ent_sortie WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(sortie)
                .fetch_optional(&mut *t)
                .await
                .map_err(db)?;
        let etat = etat.ok_or_else(introuvable)?;
        if est_terminal(&etat) {
            return Err(refus(RefusMetier::Conflit, "Cette sortie est close."));
        }
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_prestataire WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(prestataire)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !ok {
            return Err(introuvable());
        }
        sqlx::query(
            "UPDATE ent_courrier_entrant SET statut = 'rattache', sortie_id = $3, prestataire_id = $4, mode = 'humain',
                    raison = 'tri_humain', trie_par_user_id = $5, trie_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .bind(sortie)
        .bind(prestataire)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let devis = devis_depuis_courrier(
            &mut t,
            a.etablissement_id,
            Some(a.user_id),
            sortie,
            prestataire,
            courrier,
            montant_ttc_propose(&extrait),
        )
        .await?;
        journaliser(&mut t, a, sortie, Fait { apres: Some("tri humain".into()), ..Fait::simple("courrier_rattache") }).await?;
        let ev = publier(&mut t, a, "voyages.courrier_rattache", sortie, json!({ "mode": "humain" })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(devis)
    }

    /// Ignore un courrier : geste humain, tracé, avec motif (jamais un état par défaut).
    pub async fn courrier_ignorer(&self, a: &Acteur<'_>, courrier: Uuid, motif: &str) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let motif = texte_opt(Some(motif), 500)?
            .ok_or_else(|| refus(RefusMetier::Invalide, "Un motif est obligatoire pour ignorer un courrier."))?;
        let mut t = tx(self, a).await?;
        let n = sqlx::query(
            "UPDATE ent_courrier_entrant SET statut = 'ignore', raison = 'tri_humain', trie_par_user_id = $3,
                    trie_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'a_trier'",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle, actor_user_id)
             VALUES ($1,$2,NULL,'voyages_annuaire','courrier_ignore',$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(format!("Courrier ignoré : {motif}"))
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// « Nouveau prestataire » depuis un courrier : l'adresse de l'expéditeur entre dans l'annuaire
    /// **parce qu'un humain l'a confirmée ici** (jamais automatiquement).
    pub async fn courrier_nouveau_prestataire(
        &self,
        a: &Acteur<'_>,
        courrier: Uuid,
        mut s: PrestataireSaisie,
    ) -> Result<PrestataireVue, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let email: Option<String> = sqlx::query_scalar(
            "SELECT expediteur_email FROM ent_courrier_entrant WHERE etablissement_id = $1 AND id = $2 AND statut = 'a_trier'",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        let email = email.ok_or_else(introuvable)?;
        s.adresses = vec![email];
        let v = self.prestataire_creer(a, s).await?;
        let mut t = tx(self, a).await?;
        sqlx::query(
            "UPDATE ent_courrier_entrant SET prestataire_id = $3, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'a_trier'",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .bind(v.id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Pièces d'un courrier à trier ou rattaché (droit d'établissement).
    pub async fn courrier_pieces(&self, a: &Acteur<'_>, courrier: Uuid) -> Result<Vec<PieceCourrierVue>, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await?;
        let mut t = tx(self, a).await?;
        let v = sqlx::query_as::<_, PieceCourrierVue>(
            "SELECT id, nom, taille FROM ent_courrier_piece WHERE etablissement_id = $1 AND courrier_id = $2 ORDER BY cree_le, id",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Lit une pièce de courrier : droit d'établissement, empreinte vérifiée, aucune URL publique.
    pub async fn courrier_piece_lire(
        &self,
        a: &Acteur<'_>,
        courrier: Uuid,
        piece: Uuid,
    ) -> Result<(String, String, Vec<u8>), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPrestataires).await.map_err(|_| interdit())?;
        let mut t = tx(self, a).await?;
        let r: Option<(String, String, String, String)> = sqlx::query_as(
            "SELECT cle_objet, nom, sha256, mime FROM ent_courrier_piece
              WHERE etablissement_id = $1 AND courrier_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(courrier)
        .bind(piece)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        let (cle, nom, sha, mime) = r.ok_or_else(introuvable)?;
        let o = self.lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha)).await?;
        Ok((nom, mime, o))
    }
}
