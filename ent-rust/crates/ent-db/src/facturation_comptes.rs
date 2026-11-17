//! Socle 24, lots F1 et F2 — commandes du service unique sur les **comptes débiteurs** : vue guichet,
//! journal, création d'organismes payeurs, reprise (MIG-4), désignation de payeurs, conséquences d'un
//! changement de payeur, élève majeur payeur, prises en charge, tâches.
//!
//! Chaque commande : droit fin (`ent_auth::Action::Facturation*`) → transaction d'établissement (RLS) →
//! écriture → **journal** (`ent_fin_journal`, même transaction) → événement `facturation.*` sans montant.
//! Toute cible absente ou d'un autre établissement répond « introuvable » (jamais un succès).

use chrono::NaiveDate;
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind};
use ent_types::finances::{CompteFoyerDto, FactureResumeDto};
use ent_types::finances_comptes::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::facturation_payeurs::{
    self as payeurs, joindre_piece_tx, ouvrir_tache, proposer_tx, repondre_tx, PieceRepartition,
    PropositionDesignation, CATEGORIES_DESIGNATION,
};
use crate::facturation_service::{journaliser_tx, Acte, Chemin};
use crate::finances::{aujourd_hui, compte_tx, conflit, db, introuvable, invalide, publier, texte};
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt, RefusMetier};

// ---------------------------------------------------------------------------
// Fonctions pures
// ---------------------------------------------------------------------------

/// SIRET valide : 14 chiffres et clé de Luhn (les établissements de La Poste, dont le SIREN est
/// 356 000 000, suivent une autre règle : somme des chiffres multiple de 5).
pub fn siret_valide(s: &str) -> bool {
    if s.len() != 14 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let chiffres: Vec<u32> = s.bytes().map(|b| u32::from(b - b'0')).collect();
    if s.starts_with("356000000") {
        return chiffres.iter().sum::<u32>() % 5 == 0;
    }
    let somme: u32 = chiffres
        .iter()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 1 {
                let d = c * 2;
                if d > 9 { d - 9 } else { d }
            } else {
                *c
            }
        })
        .sum();
    somme.is_multiple_of(10)
}

/// Une prise en charge est-elle à renouveler (échéance dans les 30 jours) ou échue ? `jour` est un
/// **paramètre** : le calcul ne lit jamais l'horloge (règle 33). Renvoie `(a_renouveler, echue)`.
pub fn etat_prise_en_charge(au: NaiveDate, jour: NaiveDate) -> (bool, bool) {
    let echue = au < jour;
    let a_renouveler = !echue && (au - jour).num_days() <= 30;
    (a_renouveler, echue)
}

fn quotas_acteur(role: &str) -> Option<Role> {
    Role::parse(role)
}

// ---------------------------------------------------------------------------
// Lectures partagées
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct CompteRow {
    id: Uuid,
    foyer_id: Uuid,
    type_debiteur: String,
    personne_id: Option<Uuid>,
    libelle_facturation: String,
    siret: Option<String>,
    reference_client: Option<String>,
    organisme_public: bool,
    chorus_pro: bool,
    etat: String,
    a_verifier: bool,
    a_verifier_motif: Option<String>,
    entite_facturation_id: Uuid,
    origine: String,
}

const SELECT_COMPTE: &str = "SELECT id, foyer_id, type_debiteur, personne_id, libelle_facturation, siret,
        reference_client, organisme_public, chorus_pro, etat, a_verifier, a_verifier_motif,
        entite_facturation_id, origine FROM ent_fin_compte WHERE etablissement_id = $1";

impl From<CompteRow> for CompteDto {
    fn from(r: CompteRow) -> Self {
        Self {
            id: r.id,
            foyer_id: r.foyer_id,
            type_debiteur: r.type_debiteur,
            personne_id: r.personne_id,
            libelle_facturation: r.libelle_facturation,
            siret: r.siret,
            reference_client: r.reference_client,
            organisme_public: r.organisme_public,
            chorus_pro: r.chorus_pro,
            etat: r.etat,
            a_verifier: r.a_verifier,
            a_verifier_motif: r.a_verifier_motif,
            entite_facturation_id: r.entite_facturation_id,
            origine: r.origine,
        }
    }
}

async fn compte_row(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<CompteRow, EntDbError> {
    sqlx::query_as::<_, CompteRow>(&format!("{SELECT_COMPTE} AND id = $2"))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Comptes dont une personne connectée est le payeur : ses foyers payeurs (liens actifs) et, pour un
/// **élève majeur payeur**, le compte de sa fiche « personne » (FD-15 : la fiche n'a pas de compte
/// utilisateur, l'accès passe par le compte de l'élève).
pub(crate) async fn comptes_du_user(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
) -> Result<Vec<(Uuid, Uuid)>, EntDbError> {
    sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT c.id, c.foyer_id FROM ent_fin_compte c
          WHERE c.etablissement_id = $1 AND c.etat = 'actif'
            AND (EXISTS (SELECT 1 FROM ent_eleve_lien_actif l
                          WHERE l.etablissement_id = c.etablissement_id AND l.foyer_id = c.foyer_id
                            AND l.user_id = $2 AND l.est_payeur)
              OR EXISTS (SELECT 1 FROM ent_eleve_compte ec
                           JOIN ent_personne p ON p.etablissement_id = ec.etablissement_id AND p.eleve_id = ec.eleve_id
                           JOIN ent_eleve e ON e.etablissement_id = ec.etablissement_id AND e.id = ec.eleve_id
                          WHERE ec.etablissement_id = c.etablissement_id AND ec.user_id = $2
                            AND p.id = c.personne_id
                            -- M5 : l'élève n'accède au compte que s'il est majeur **aujourd'hui** et qu'une
                            -- désignation à son nom a pris effet (date d'effet atteinte).
                            AND e.date_naissance IS NOT NULL AND e.date_naissance <= ($3::date - interval '18 years')::date
                            AND EXISTS (SELECT 1 FROM ent_fin_designation d
                                         WHERE d.etablissement_id = c.etablissement_id AND d.compte_id = c.id
                                           AND d.statut IN ('effective', 'en_attente') AND d.du <= $3)))
          ORDER BY c.created_at, c.id",
    )
    .bind(etab)
    .bind(user)
    .bind(aujourd_hui())
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

/// Foyers d'un élève majeur payeur (pour les routes historiques de la famille).
pub(crate) async fn foyers_eleve_majeur_payeur(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
) -> Result<Vec<Uuid>, EntDbError> {
    let r: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT c.foyer_id FROM ent_fin_compte c
           JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id
           JOIN ent_eleve_compte ec ON ec.etablissement_id = p.etablissement_id AND ec.eleve_id = p.eleve_id
           JOIN ent_eleve e ON e.etablissement_id = ec.etablissement_id AND e.id = ec.eleve_id
          WHERE c.etablissement_id = $1 AND ec.user_id = $2 AND c.etat = 'actif'
            AND e.date_naissance IS NOT NULL AND e.date_naissance <= ($3::date - interval '18 years')::date
            AND EXISTS (SELECT 1 FROM ent_fin_designation d
                         WHERE d.etablissement_id = c.etablissement_id AND d.compte_id = c.id
                           AND d.statut IN ('effective', 'en_attente') AND d.du <= $3)",
    )
    .bind(etab)
    .bind(user)
    .bind(aujourd_hui())
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.into_iter().map(|x| x.0).collect())
}

// ---------------------------------------------------------------------------
// Commandes
// ---------------------------------------------------------------------------

fn dto_part(p: &payeurs::PartPayeur) -> PartPayeurDto {
    PartPayeurDto {
        compte_id: p.compte_id,
        libelle: p.libelle.clone(),
        part_bps: p.part_bps as i32,
        rang: p.rang,
    }
}

fn categorie_valide(c: &str) -> Result<(), EntDbError> {
    if CATEGORIES_DESIGNATION.contains(&c) {
        Ok(())
    } else {
        Err(invalide("catégorie : toutes, scolarite, restauration, internat, activite, frais, voyages, fournitures"))
    }
}

impl PgEnt {
    // -- F1 : journal -------------------------------------------------------

    /// FAC-203 : le journal financier. `facturation.journal` voit tout ; un autre membre du personnel
    /// de facturation ne voit que **ses propres** gestes (M-676). Ajout seul : aucune route d'écriture.
    pub async fn journal_financier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        limite: i64,
    ) -> Result<Vec<JournalLigneDto>, EntDbError> {
        let tout = self
            .a_action_etablissement(etab, actor, role, Action::FacturationJournal)
            .await?;
        if !tout {
            // Ses propres gestes : réservé à qui travaille dans la facturation.
            self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        }
        let limite = limite.clamp(1, 500);
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>, Option<Uuid>, String, String, String, String,
                       Option<Uuid>, Option<Uuid>, Option<Uuid>, Option<i64>, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT id, le, acteur_user_id, casquette, chemin, action, objet_type, objet_id, compte_id,
                        eleve_id, montant_centimes, motif, detail
                   FROM ent_fin_journal
                  WHERE etablissement_id = $1 AND ($2 OR acteur_user_id = $3)
                  ORDER BY le DESC, id LIMIT $4",
            )
            .bind(etab)
            .bind(tout)
            .bind(actor)
            .bind(limite)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| JournalLigneDto {
                id: r.0,
                le: r.1,
                acteur_user_id: r.2,
                casquette: r.3,
                chemin: r.4,
                action: r.5,
                objet_type: r.6,
                objet_id: r.7,
                compte_id: r.8,
                eleve_id: r.9,
                montant_centimes: r.10,
                motif: r.11,
                detail: r.12,
            })
            .collect())
    }

    // -- F1 : vue guichet (R-F37) --------------------------------------------

    /// Quota quotidien d'actions au guichet par agent (E3) : recherches et ouvertures de comptes confondues,
    /// jour de Paris. Au-delà : refus, ligne de journal et alerte à la direction (tâche `guichet_quota`,
    /// une seule ouverte par agent) — l'alerte est validée même si la demande est refusée.
    async fn controler_quota_guichet(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        let mut tx = self.fin_tx(etab).await?;
        let (quota,): (i32,) = sqlx::query_as(
            "SELECT COALESCE((SELECT guichet_quota_jour FROM ent_fin_parametres WHERE etablissement_id = $1), 30)",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (utilise,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_fin_journal
              WHERE etablissement_id = $1 AND acteur_user_id = $2
                AND action IN ('guichet.rechercher', 'guichet.consulter')
                AND le >= (date_trunc('day', now() AT TIME ZONE 'Europe/Paris') AT TIME ZONE 'Europe/Paris')",
        )
        .bind(etab)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if utilise < i64::from(quota) {
            return Ok(());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Guichet, "guichet.quota_depasse", "compte")
                .detail(&format!("quota quotidien de {quota} atteint")),
        )
        .await?;
        ouvrir_tache(
            &mut tx,
            etab,
            "guichet_quota",
            None,
            None,
            Some(&format!("u:{actor}")),
            aujourd_hui(),
            &format!("un agent a atteint le quota quotidien d'actions au guichet ({quota}) : vérifier l'usage"),
            Some(actor),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Err(EntDbError::Refus(
            RefusMetier::Interdit,
            "quota quotidien du guichet atteint : la direction est alertée",
        ))
    }

    /// Règle le quota quotidien d'actions au guichet de l'établissement (1 à 1 000 ; 30 par défaut).
    pub async fn definir_quota_guichet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        par_jour: i32,
    ) -> Result<i32, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationParametrer).await?;
        if !(1..=1000).contains(&par_jour) {
            return Err(invalide("quota : 1 à 1000 actions par jour"));
        }
        let mut tx = self.fin_tx(etab).await?;
        sqlx::query(
            "INSERT INTO ent_fin_parametres (etablissement_id, guichet_quota_jour) VALUES ($1, $2)
             ON CONFLICT (etablissement_id) DO UPDATE SET guichet_quota_jour = EXCLUDED.guichet_quota_jour",
        )
        .bind(etab)
        .bind(par_jour)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "guichet.quota_regle", "parametres")
                .detail(&format!("{par_jour} par jour")),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(par_jour)
    }

    /// Cherche le payeur présent par ses **nom et prénom complets** (correspondance exacte, sans
    /// casse) : 5 résultats au plus, aucune adresse ni aucun solde. Soumise au quota du guichet et
    /// journalisée (le texte cherché ne l'est pas). Une recherche partielle est refusée (E3).
    pub async fn guichet_rechercher_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nom: &str,
        prenom: &str,
        _auth: AuthentificationGuichet,
    ) -> Result<Vec<GuichetPayeurDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGuichet).await?;
        let (nom, prenom) = (nom.trim(), prenom.trim());
        let valide = |t: &str| t.chars().count() >= 2 && t.chars().count() <= 80 && !t.contains(['%', '_']);
        if !valide(nom) || !valide(prenom) {
            return Err(invalide("nom et prénom complets exigés"));
        }
        self.controler_quota_guichet(etab, actor, role).await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT c.id, c.libelle_facturation FROM ent_fin_compte c
               JOIN ent_eleve_lien l ON l.etablissement_id = c.etablissement_id AND l.foyer_id = c.foyer_id
                                    AND l.est_payeur AND l.suspendu_le IS NULL
               JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
              WHERE c.etablissement_id = $1 AND c.etat = 'actif'
                AND lower(p.nom) = lower($2) AND lower(p.prenom) = lower($3)
              ORDER BY c.libelle_facturation, c.id LIMIT 5",
        )
        .bind(etab)
        .bind(nom)
        .bind(prenom)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Guichet, "guichet.rechercher", "compte")
                .detail(&format!("{} résultat(s)", rows.len())),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(compte_id, libelle)| GuichetPayeurDto { compte_id, libelle })
            .collect())
    }

    /// La vue guichet d'**un** compte : solde, échéances, documents (R-F37). Sans preuve d'identité
    /// (présentiel, espace connecté ou rappel au numéro enregistré), aucune donnée (CF-95). L'accueil
    /// n'a pas les documents. Chaque consultation est journalisée (R-F36).
    pub async fn guichet_compte_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        compte: Uuid,
        auth: AuthentificationGuichet,
    ) -> Result<GuichetCompteDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGuichet).await?;
        self.controler_quota_guichet(etab, actor, role).await?;
        let mut tx = self.fin_tx(etab).await?;
        let c = compte_row(&mut tx, etab, compte).await?;
        let vue: CompteFoyerDto = compte_tx(&mut tx, etab, c.foyer_id, false).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Guichet, "guichet.consulter", "compte")
                .objet(compte)
                .compte(compte)
                .detail(match auth {
                    AuthentificationGuichet::Presentiel => "payeur présent au guichet",
                    AuthentificationGuichet::EspaceConnecte => "payeur authentifié dans l'espace famille",
                    AuthentificationGuichet::RappelNumeroEnregistre => "rappel au numéro enregistré",
                }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        // F1 : liste blanche — les documents ne sont vus que par l'Administratif ou un détenteur de
        // `facturation.lire` ; tout autre rôle du guichet (accueil, personnel…) n'a que solde et échéances.
        let voit_documents = quotas_acteur(role).is_some_and(|r| matches!(r, Role::Administratif))
            || self.a_action_etablissement(etab, actor, role, Action::FacturationLire).await?;
        let sans_documents = !voit_documents;
        let documents = (!sans_documents).then(|| {
            vue.factures
                .iter()
                .map(|f: &FactureResumeDto| GuichetDocumentDto {
                    facture_id: f.id,
                    numero: f.numero.clone(),
                    type_piece: f.type_piece.clone(),
                    statut: f.statut.clone(),
                    total_centimes: f.total_centimes,
                })
                .collect()
        });
        Ok(GuichetCompteDto {
            compte_id: compte,
            libelle: c.libelle_facturation,
            solde_centimes: vue.solde_centimes,
            echeances: vue
                .echeances
                .iter()
                .map(|e| GuichetEcheanceDto {
                    piece_numero: e.facture_numero.clone(),
                    date_echeance: e.echeance.date_echeance,
                    montant_centimes: e.echeance.montant_centimes,
                    regle_centimes: e.echeance.couvert_centimes,
                })
                .collect(),
            documents,
        })
    }

    // -- F2 : comptes -------------------------------------------------------

    pub async fn comptes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        filtre: &ComptesFiltre,
    ) -> Result<Vec<CompteDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        if let Some(t) = &filtre.type_debiteur {
            if !["personne", "organisme", "foyer"].contains(&t.as_str()) {
                return Err(invalide("type : personne, organisme ou foyer"));
            }
        }
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<CompteRow> = sqlx::query_as(&format!(
            "{SELECT_COMPTE} AND ($2::boolean IS NULL OR a_verifier = $2)
                AND ($3::text IS NULL OR type_debiteur = $3)
              ORDER BY libelle_facturation, id LIMIT 500"
        ))
        .bind(etab)
        .bind(filtre.a_verifier)
        .bind(&filtre.type_debiteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "comptes.lister", "compte")
                .detail(&format!("{} compte(s)", rows.len())),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(rows.into_iter().map(CompteDto::from).collect())
    }

    pub async fn compte_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<CompteDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        let mut tx = self.fin_tx(etab).await?;
        let c = compte_row(&mut tx, etab, id).await?;
        // R-F36 : toute lecture nominative d'un compte par le personnel est journalisée.
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "compte.consulter", "compte")
                .objet(id)
                .compte(id),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(c.into())
    }

    /// FAC-256 : crée le compte d'un **organisme payeur** (employeur, comité d'entreprise, aide sociale à
    /// l'enfance…). Un foyer technique porte le compte (compatibilité des pièces, Q-A1) ; aucune donnée
    /// scolaire n'est attachée à l'organisme.
    pub async fn creer_compte_organisme_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CreerCompteOrganismeBody,
    ) -> Result<CompteDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let libelle = texte(&b.libelle, 200, "nom de l'organisme obligatoire (200 caractères)")?;
        let siret = match b.siret.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) if siret_valide(s) => Some(s.to_string()),
            Some(_) => return Err(invalide("SIRET invalide (14 chiffres, clé de contrôle)")),
            None => None,
        };
        if b.chorus_pro && !b.organisme_public {
            return Err(invalide("Chorus Pro ne concerne qu'un organisme public"));
        }
        if b.chorus_pro && siret.is_none() {
            return Err(invalide("Chorus Pro exige le SIRET de l'organisme"));
        }
        let reference = b
            .reference_client
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(80).collect::<String>());
        let mut tx = self.fin_tx(etab).await?;
        let foyer = crate::dossier_personnes::creer_foyer_tx(&mut tx, etab, &libelle).await?;
        let entite: (Uuid,) = sqlx::query_as("SELECT ent_fin_entite_par_defaut($1)")
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_compte (id, etablissement_id, entite_facturation_id, type_debiteur, foyer_id,
                    libelle_facturation, siret, reference_client, organisme_public, chorus_pro, origine, cree_par)
             VALUES ($1,$2,$3,'organisme',$4,$5,$6,$7,$8,$9,'organisme',$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(entite.0)
        .bind(foyer)
        .bind(&libelle)
        .bind(&siret)
        .bind(&reference)
        .bind(b.organisme_public)
        .bind(b.chorus_pro)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.as_database_error().and_then(|d| d.code()).is_some_and(|c| c == "23505") {
                conflit("un compte porte déjà ce SIRET")
            } else {
                db(e)
            }
        })?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "compte.creer_organisme", "compte")
                .objet(id)
                .compte(id)
                .foyer(foyer)
                .detail("compte d'organisme payeur créé"),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::FACTURATION_COMPTE_CREE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "compte_id": id, "type": "organisme" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let c = compte_row(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(c.into())
    }

    /// MIG-4 rejouable : une fonction SQL unique (`ent_fin_reprendre_comptes`) crée les comptes des foyers
    /// qui ont des pièces ou sont payeurs, rattache les pièces et reprend les désignations ; les foyers à
    /// plusieurs payeurs (ou sans payeur) sont **signalés**, jamais éclatés (CF-09).
    pub async fn reprendre_comptes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<RapportRepriseDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationReprise).await?;
        let mut tx = self.fin_tx(etab).await?;
        let r: (i32, i32, i32, i32, i32) =
            sqlx::query_as("SELECT comptes_crees, pieces_rattachees, foyers_a_verifier, designations_creees, eleves_a_verifier FROM ent_fin_reprendre_comptes($1, $2)")
                .bind(etab)
                .bind(actor)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "comptes.reprendre", "compte").detail(&format!(
                "{} comptes créés, {} pièces rattachées, {} foyers à vérifier, {} désignations reprises, {} élèves à vérifier",
                r.0, r.1, r.2, r.3, r.4
            )),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::FACTURATION_REPRISE_COMPTES,
            AggregateKind::Facture,
            etab,
            None,
            json!({ "comptes": r.0, "pieces": r.1, "a_verifier": r.2, "designations": r.3, "eleves_a_verifier": r.4 }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RapportRepriseDto {
            comptes_crees: r.0,
            pieces_rattachees: r.1,
            foyers_a_verifier: r.2,
            designations_creees: r.3,
            eleves_a_verifier: r.4,
        })
    }

    // -- F2 : payeurs désignés ---------------------------------------------

    /// Qui paie `categorie` pour `eleve` à `date` ? (le port du socle 4, par adaptateur).
    pub async fn payeurs_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        categorie: &str,
        date: NaiveDate,
    ) -> Result<PayeursDto, EntDbError> {
        if !self.a_action_etablissement(etab, actor, role, Action::FacturationEngagements).await? {
            self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        }
        categorie_valide(categorie)?;
        let mut tx = self.fin_tx(etab).await?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let lus = payeurs::payeurs_a(&mut tx, etab, eleve, categorie, date, false).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "payeurs.consulter", "designation").eleve(eleve),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(PayeursDto {
            eleve_id: eleve,
            categorie: categorie.to_string(),
            date,
            complet: lus.complete(),
            origine: lus.origine.code().to_string(),
            parts: lus.parts.iter().map(dto_part).collect(),
        })
    }

    /// FAC-217, FAC-218 : désigne une répartition de payeurs à une date. Sans pièce, elle reste **en
    /// attente** de l'acceptation de chaque payeur désigné (R-F30) ; avec pièce (jugement, convention
    /// homologuée, accord des deux), elle est opposable aussitôt (R-F31).
    pub async fn designer_payeurs_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        b: &DesignerPayeursBody,
    ) -> Result<ResultatDesignationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        categorie_valide(&b.categorie)?;
        let parts: Vec<(Uuid, i64, i16)> = b
            .parts
            .iter()
            .map(|p| (p.compte_id, i64::from(p.part_bps), p.rang.unwrap_or(1)))
            .collect();
        let prop = PropositionDesignation {
            eleve_id: eleve,
            categorie: b.categorie.clone(),
            parts,
            date_effet: b.date_effet,
            piece: b.piece.as_ref().map(|p| PieceRepartition {
                nature: p.nature.clone(),
                reference: p.reference.clone(),
                document_id: p.document_id,
            }),
            motif: b.motif.clone(),
            source: "finances",
        };
        let mut tx = self.fin_tx(etab).await?;
        let r = proposer_tx(&mut tx, etab, actor, role, &prop).await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatDesignationDto {
            groupe_id: r.groupe_id,
            statut: r.statut.to_string(),
            consequences_proposees: r.consequences as i32,
        })
    }

    /// Réponse du **payeur désigné** (R-F30) : seul un payeur du compte visé peut accepter ou refuser ;
    /// toute autre personne reçoit « introuvable » (jamais l'existence de la proposition).
    pub async fn repondre_designation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        groupe: Uuid,
        accepte: bool,
    ) -> Result<ResultatDesignationDto, EntDbError> {
        let mut tx = self.fin_tx(etab).await?;
        let mes: Vec<Uuid> = comptes_du_user(&mut tx, etab, actor).await?.into_iter().map(|c| c.0).collect();
        let r = repondre_tx(&mut tx, etab, actor, role, groupe, &mes, accepte).await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatDesignationDto {
            groupe_id: groupe,
            statut: match (accepte, r) {
                (false, _) => "refusee",
                (true, Some(_)) => "effective",
                (true, None) => "en_attente",
            }
            .to_string(),
            consequences_proposees: r.unwrap_or(0) as i32,
        })
    }

    /// FAC-217 : joint la pièce de répartition à une proposition en attente.
    pub async fn joindre_piece_repartition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        groupe: Uuid,
        b: &JoindrePieceBody,
    ) -> Result<ResultatDesignationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let piece = PieceRepartition {
            nature: b.piece.nature.clone(),
            reference: b.piece.reference.clone(),
            document_id: b.piece.document_id,
        };
        let mut tx = self.fin_tx(etab).await?;
        let n = joindre_piece_tx(&mut tx, etab, actor, role, groupe, &piece).await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatDesignationDto {
            groupe_id: groupe,
            statut: if n.is_some() { "effective" } else { "en_attente" }.into(),
            consequences_proposees: n.unwrap_or(0) as i32,
        })
    }

    /// CF-75 : un parent seul demande un changement de répartition → une **tâche** pour le secrétariat ;
    /// rien ne change, l'autre parent n'en est pas informé par cette route (demande d'acceptation neutre
    /// plus tard). Seul un payeur de l'élève peut demander.
    pub async fn demander_repartition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &DemandeRepartitionBody,
    ) -> Result<AccuseDemandeDto, EntDbError> {
        categorie_valide(&b.categorie)?;
        let mut tx = self.fin_tx(etab).await?;
        let mes = comptes_du_user(&mut tx, etab, actor).await?;
        // Le compte du demandeur : celui des payeurs de l'élève qui lui appartient (une demande par demandeur).
        let mon_compte: Option<Uuid> = if mes.is_empty() {
            None
        } else {
            let lus = payeurs::payeurs_a(&mut tx, etab, b.eleve_id, &b.categorie, aujourd_hui(), false).await?;
            lus.parts
                .iter()
                .find(|p| mes.iter().any(|m| m.0 == p.compte_id))
                .map(|p| p.compte_id)
        };
        let Some(mon_compte) = mon_compte else {
            // Même réponse pour un élève inconnu, d'un autre établissement ou sans lien de paiement.
            return Err(introuvable());
        };
        let commentaire = b.commentaire.as_deref().unwrap_or("").trim().chars().take(200).collect::<String>();
        // E1 : la tâche porte le compte du demandeur (clé d'idempotence par demandeur) ; la réponse à la
        // famille est un accusé neutre, jamais la tâche ni le commentaire d'un autre parent.
        ouvrir_tache(
            &mut tx,
            etab,
            "demande_repartition",
            Some(b.eleve_id),
            Some(mon_compte),
            Some(&b.categorie),
            aujourd_hui(),
            &format!("demande d'un payeur : changement de répartition ({commentaire}) ; rien ne change sans acceptation ou pièce"),
            Some(actor),
        )
        .await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Famille, "repartition.demander", "tache")
                .eleve(b.eleve_id)
                .compte(mon_compte),
        )
        .await?;
        let accuse: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, statut FROM ent_fin_tache
              WHERE etablissement_id = $1 AND nature = 'demande_repartition' AND eleve_id = $2
                AND categorie = $3 AND compte_id = $4 AND statut = 'ouverte'
              ORDER BY created_at DESC LIMIT 1",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .bind(&b.categorie)
        .bind(mon_compte)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        accuse
            .map(|(id, statut)| AccuseDemandeDto { id, statut })
            .ok_or_else(introuvable)
    }

    /// M1 : réactive une prise en charge **échue** (nouvelle date de fin postérieure à aujourd'hui, motif
    /// obligatoire, journalisé). Une prise en charge annulée ou encore active ne se réactive pas.
    pub async fn reactiver_prise_en_charge_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ReactiverPriseBody,
    ) -> Result<PriseEnChargeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let motif = b.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 200 {
            return Err(invalide("motif : 3 à 200 caractères"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let r: Option<(Uuid, Uuid, String, NaiveDate, String)> = sqlx::query_as(
            "SELECT compte_id, eleve_id, reference, du, statut FROM ent_fin_prise_en_charge
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((compte, eleve, _reference, du, statut)) = r else { return Err(introuvable()) };
        if statut != "echue" {
            return Err(conflit("seule une prise en charge échue se réactive"));
        }
        if b.au <= aujourd_hui() || b.au < du {
            return Err(invalide("nouvelle fin : postérieure à aujourd'hui"));
        }
        sqlx::query("UPDATE ent_fin_prise_en_charge SET statut = 'active', au = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(b.au)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_fin_tache SET statut = 'resolue', resolue_le = now(), resolue_par = $3
              WHERE etablissement_id = $1 AND prise_en_charge_id = $2 AND statut = 'ouverte'",
        )
        .bind(etab)
        .bind(id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "prise_en_charge.reactiver", "prise_en_charge")
                .objet(id)
                .compte(compte)
                .eleve(eleve)
                .motif(motif),
        )
        .await?;
        let p = prise_en_charge_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    /// FAC-253, CF-84 : le payeur passe des parents à l'**élève majeur** à sa demande. L'élève doit être
    /// majeur (date de naissance), avoir sa fiche « personne » (socle 4) et accepter (R-F30) ; les
    /// pièces passées restent aux parents (aucune pièce émise n'est touchée), les conséquences sur les
    /// mois à venir sont proposées.
    pub async fn transferer_payeur_majeur_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        b: &PayeurMajeurBody,
    ) -> Result<ResultatDesignationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let categories: Vec<String> = if b.categories.is_empty() {
            vec!["toutes".into()]
        } else {
            b.categories.clone()
        };
        for c in &categories {
            categorie_valide(c)?;
        }
        let mut tx = self.fin_tx(etab).await?;
        let r: Option<(Option<chrono::NaiveDate>,)> = sqlx::query_as(
            "SELECT date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (naissance,) = r.ok_or_else(introuvable)?;
        let majeur = naissance.is_some_and(|n| {
            use chrono::Datelike;
            let d = b.date_effet;
            let age = d.year() - n.year() - i32::from((d.month(), d.day()) < (n.month(), n.day()));
            age >= 18
        });
        if !majeur {
            return Err(invalide("l'élève n'est pas majeur à la date d'effet"));
        }
        let personne: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((personne,)) = personne else {
            return Err(conflit("la fiche « personne » de l'élève majeur est à créer au dossier (secrétariat)"));
        };
        // Compte de l'élève : le sien, ou un compte créé sur un foyer technique à son nom.
        let compte: Uuid = match sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM ent_fin_compte WHERE etablissement_id = $1 AND personne_id = $2",
        )
        .bind(etab)
        .bind(personne)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        {
            Some((id,)) => id,
            None => {
                let (nom, prenom): (String, Option<String>) = sqlx::query_as(
                    "SELECT nom, prenom FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(personne)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                let libelle = format!("{} {}", prenom.unwrap_or_default(), nom).trim().to_string();
                let foyer = crate::dossier_personnes::creer_foyer_tx(&mut tx, etab, &libelle).await?;
                let entite: (Uuid,) = sqlx::query_as("SELECT ent_fin_entite_par_defaut($1)")
                    .bind(etab)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_fin_compte (id, etablissement_id, entite_facturation_id, type_debiteur,
                            personne_id, foyer_id, libelle_facturation, origine, cree_par)
                     VALUES ($1,$2,$3,'personne',$4,$5,$6,'majeur',$7)",
                )
                .bind(id)
                .bind(etab)
                .bind(entite.0)
                .bind(personne)
                .bind(foyer)
                .bind(&libelle)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                publier(
                    &mut tx,
                    etab,
                    types::FACTURATION_COMPTE_CREE,
                    AggregateKind::Facture,
                    id,
                    Some(eleve),
                    json!({ "compte_id": id, "type": "personne", "eleve_majeur": true }),
                    Some(actor),
                    Causalite::default(),
                )
                .await?;
                id
            }
        };
        let mut dernier = None;
        for c in &categories {
            let prop = PropositionDesignation {
                eleve_id: eleve,
                categorie: c.clone(),
                parts: vec![(compte, payeurs::PART_ENTIERE, 1)],
                date_effet: b.date_effet,
                piece: None,
                motif: b.motif.clone(),
                source: "majeur",
            };
            dernier = Some(proposer_tx(&mut tx, etab, actor, role, &prop).await?);
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "payeur.transferer_majeur", "designation")
                .eleve(eleve)
                .compte(compte)
                .detail(&format!("à compter du {} : acceptation de l'élève attendue", b.date_effet)),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::FACTURATION_CHANGEMENT_PAYEUR_APPLIQUE,
            AggregateKind::Eleve,
            eleve,
            Some(eleve),
            json!({ "eleve_majeur": true, "date_effet": b.date_effet, "statut": "en_attente" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        let r = dernier.ok_or_else(introuvable)?;
        Ok(ResultatDesignationDto {
            groupe_id: r.groupe_id,
            statut: r.statut.to_string(),
            consequences_proposees: r.consequences as i32,
        })
    }

    // -- F2 : conséquences ---------------------------------------------------

    pub async fn consequences_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Option<Uuid>,
    ) -> Result<Vec<ConsequenceDto>, EntDbError> {
        if !self.a_action_etablissement(etab, actor, role, Action::FacturationAvoirs).await? {
            self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        }
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, String, NaiveDate, String, Uuid, Uuid, Uuid, i32, i32, i64, String)> =
            sqlx::query_as(
                "SELECT id, designation_groupe_id, eleve_id, categorie, date_effet, sens, compte_id, ligne_id,
                        facture_id, mois_total, mois_apres, montant_centimes, statut
                   FROM ent_fin_consequence
                  WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR eleve_id = $2)
                  ORDER BY created_at DESC, sens, id LIMIT 500",
            )
            .bind(etab)
            .bind(eleve)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| ConsequenceDto {
                id: r.0,
                designation_groupe_id: r.1,
                eleve_id: r.2,
                categorie: r.3,
                date_effet: r.4,
                sens: r.5,
                compte_id: r.6,
                ligne_id: r.7,
                facture_id: r.8,
                mois_total: r.9,
                mois_apres: r.10,
                montant_centimes: r.11,
                statut: r.12,
            })
            .collect())
    }

    /// FAC-176 ★ : accepte (ou refuse) les conséquences proposées d'un changement de payeur. Accepter
    /// crée des **brouillons** (avoir au payeur sortant, ligne au payeur entrant) ; l'émission reste un
    /// geste humain (`facturation.emettre`). Aucune pièce émise n'est modifiée.
    pub async fn decider_consequences_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        groupe: Uuid,
        b: &DecisionConsequencesBody,
    ) -> Result<ResultatConsequencesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationAvoirs).await?;
        let mut tx = self.fin_tx(etab).await?;
        let r = crate::facturation_consequences::decider_tx(&mut tx, etab, actor, role, groupe, b).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    // -- F2 : prises en charge ------------------------------------------------

    pub async fn creer_prise_en_charge_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &PriseEnChargeBody,
    ) -> Result<PriseEnChargeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let reference = texte(&b.reference, 80, "référence de prise en charge obligatoire (80 caractères)")?;
        if b.au < b.du {
            return Err(invalide("période : fin avant début"));
        }
        if b.categories.is_empty() || b.categories.len() > 8 {
            return Err(invalide("1 à 8 catégories"));
        }
        for c in &b.categories {
            categorie_valide(c)?;
        }
        // F7 : catégories dédoublonnées (ordre conservé).
        let mut categories: Vec<String> = Vec::new();
        for c in &b.categories {
            if !categories.contains(c) {
                categories.push(c.clone());
            }
        }
        if b.plafond_centimes.is_some_and(|p| p <= 0 || p > 1_000_000_000) {
            return Err(invalide("plafond : un montant positif en centimes (10 millions d'euros au plus), ou aucun"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let c = compte_row(&mut tx, etab, b.compte_id).await?;
        if c.type_debiteur != "organisme" {
            return Err(conflit("une prise en charge ne vise qu'un compte d'organisme"));
        }
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_prise_en_charge (id, etablissement_id, compte_id, eleve_id, reference, du, au,
                    categories, plafond_centimes, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.compte_id)
        .bind(b.eleve_id)
        .bind(&reference)
        .bind(b.du)
        .bind(b.au)
        .bind(&categories)
        .bind(b.plafond_centimes)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "prise_en_charge.creer", "prise_en_charge")
                .objet(id)
                .compte(b.compte_id)
                .eleve(b.eleve_id)
                .detail(&format!("du {} au {}", b.du, b.au)),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::FACTURATION_PRISE_EN_CHARGE,
            AggregateKind::Facture,
            id,
            Some(b.eleve_id),
            json!({ "prise_en_charge_id": id, "compte_id": b.compte_id }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(PriseEnChargeDto {
            id,
            compte_id: b.compte_id,
            eleve_id: b.eleve_id,
            reference,
            du: b.du,
            au: b.au,
            categories: b.categories.clone(),
            plafond_centimes: b.plafond_centimes,
            statut: "active".into(),
        })
    }

    pub async fn prises_en_charge_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<PriseEnChargeDto>, EntDbError> {
        let engagements = self.a_action_etablissement(etab, actor, role, Action::FacturationEngagements).await?;
        if !engagements {
            self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        }
        // F2 : la référence d'une prise en charge (numéro de dossier de l'organisme) n'est lue que par qui a
        // `facturation.lire` ; le secrétariat (engagements seuls) la voit masquée.
        let voit_reference = self.a_action_etablissement(etab, actor, role, Action::FacturationLire).await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, String, NaiveDate, NaiveDate, Vec<String>, Option<i64>, String)> =
            sqlx::query_as(
                "SELECT id, compte_id, eleve_id, reference, du, au, categories, plafond_centimes, statut
                   FROM ent_fin_prise_en_charge WHERE etablissement_id = $1 ORDER BY au, id LIMIT 500",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| PriseEnChargeDto {
                id: r.0,
                compte_id: r.1,
                eleve_id: r.2,
                reference: if voit_reference { r.3 } else { "(masquée)".to_string() },
                du: r.4,
                au: r.5,
                categories: r.6,
                plafond_centimes: r.7,
                statut: r.8,
            })
            .collect())
    }

    /// FAC-257 : alerte 30 jours avant l'échéance d'une prise en charge (tâche de renouvellement) ; une
    /// prise en charge **échue** passe `echue` et n'est plus jamais facturée à l'organisme ni reportée
    /// sur la famille sans décision (CF-83). Idempotent ; `jour` est un paramètre (jamais l'horloge).
    pub async fn alerter_prises_en_charge_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<RapportAlertesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, NaiveDate, String)> = sqlx::query_as(
            "SELECT id, compte_id, eleve_id, au, statut FROM ent_fin_prise_en_charge
              WHERE etablissement_id = $1 AND statut IN ('active', 'echue') ORDER BY au, id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (mut a_renouveler, mut echues) = (0, 0);
        for (id, compte, eleve, au, statut) in rows {
            let (proche, echue) = etat_prise_en_charge(au, jour);
            if echue && statut == "active" {
                sqlx::query("UPDATE ent_fin_prise_en_charge SET statut = 'echue' WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
            if proche || echue {
                sqlx::query(
                    "INSERT INTO ent_fin_tache (id, etablissement_id, nature, eleve_id, compte_id, prise_en_charge_id,
                            depuis, detail, cree_par)
                     VALUES ($1,$2,'prise_en_charge_a_renouveler',$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(eleve)
                .bind(compte)
                .bind(id)
                .bind(au)
                .bind(if echue {
                    "prise en charge échue : plus aucune facture à l'organisme, rien n'est reporté sur la famille sans décision"
                } else {
                    "prise en charge à renouveler dans les 30 jours"
                })
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            if echue {
                echues += 1;
            } else if proche {
                a_renouveler += 1;
            }
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "prises_en_charge.alerter", "prise_en_charge")
                .detail(&format!("{a_renouveler} à renouveler, {echues} échue(s) au {jour}")),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RapportAlertesDto { a_renouveler, echues })
    }

    // -- F2 : tâches ----------------------------------------------------------

    pub async fn taches_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        filtre: &TachesFiltre,
    ) -> Result<Vec<TacheDto>, EntDbError> {
        if !self.a_action_etablissement(etab, actor, role, Action::FacturationEngagements).await? {
            self.fin_exiger(etab, actor, role, Action::FacturationLire).await?;
        }
        let statut = filtre.statut.as_deref().unwrap_or("ouverte");
        if !["ouverte", "resolue", "abandonnee"].contains(&statut) {
            return Err(invalide("statut : ouverte, resolue ou abandonnee"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String, Option<Uuid>, Option<Uuid>, Option<String>, NaiveDate, Option<String>, String)> =
            sqlx::query_as(
                "SELECT id, nature, eleve_id, compte_id, categorie, depuis, detail, statut FROM ent_fin_tache
                  WHERE etablissement_id = $1 AND statut = $2 ORDER BY depuis, created_at, id LIMIT 500",
            )
            .bind(etab)
            .bind(statut)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| TacheDto {
                id: r.0,
                nature: r.1,
                eleve_id: r.2,
                compte_id: r.3,
                categorie: r.4,
                depuis: r.5,
                detail: r.6,
                statut: r.7,
            })
            .collect())
    }

    /// Abandonne une tâche (situation traitée hors logiciel) : motif obligatoire, journalisé.
    pub async fn abandonner_tache_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationEngagements).await?;
        let motif = texte(motif, 300, "motif obligatoire (300 caractères)")?;
        let mut tx = self.fin_tx(etab).await?;
        let r = sqlx::query(
            "UPDATE ent_fin_tache SET statut = 'abandonnee', resolue_le = now(), resolue_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'ouverte'",
        )
        .bind(etab)
        .bind(id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "tache.abandonner", "tache")
                .objet(id)
                .motif(&motif),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// FD-15 : les comptes débiteurs de la personne connectée (payeur d'un foyer, ou élève majeur payeur).
    pub async fn mes_comptes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
    ) -> Result<Vec<MonCompteDto>, EntDbError> {
        let mut tx = self.fin_tx(etab).await?;
        let mes = comptes_du_user(&mut tx, etab, actor).await?;
        let mut out = Vec::new();
        for (id, _) in mes {
            let c = compte_row(&mut tx, etab, id).await?;
            out.push(MonCompteDto {
                id: c.id,
                foyer_id: c.foyer_id,
                type_debiteur: c.type_debiteur,
                libelle_facturation: c.libelle_facturation,
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facturation_payeurs::{categorie_de_designation, NATURES_PIECE};

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn siret_cle_de_luhn() {
        // SIRET d'exemple public (établissement siège d'une administration) : clé valide.
        assert!(siret_valide("73282932000074"));
        assert!(!siret_valide("73282932000075"), "clé fausse");
        assert!(!siret_valide("7328293200007"), "13 chiffres");
        assert!(!siret_valide("7328293200007a"), "lettre");
        assert!(!siret_valide(""));
        // La Poste : somme des chiffres multiple de 5.
        assert!(siret_valide("35600000000047"));
        assert!(!siret_valide("35600000000048"));
    }

    #[test]
    fn prise_en_charge_alerte_a_trente_jours_et_echue() {
        let jour = d(2027, 1, 1);
        assert_eq!(etat_prise_en_charge(d(2027, 1, 31), jour), (true, false), "30 jours : à renouveler");
        assert_eq!(etat_prise_en_charge(d(2027, 2, 1), jour), (false, false), "31 jours : pas encore");
        assert_eq!(etat_prise_en_charge(d(2027, 1, 1), jour), (true, false), "échéance aujourd'hui : dernier jour");
        assert_eq!(etat_prise_en_charge(d(2026, 12, 31), jour), (false, true), "échue hier");
    }

    #[test]
    fn categories_valides_listees() {
        assert!(categorie_valide("toutes").is_ok());
        assert!(categorie_valide("restauration").is_ok());
        assert!(categorie_valide("n'importe quoi").is_err());
        assert_eq!(categorie_de_designation("internat"), "internat");
        assert!(NATURES_PIECE.contains(&"jugement"));
    }
}

async fn prise_en_charge_dto(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<PriseEnChargeDto, EntDbError> {
    let r: (Uuid, Uuid, Uuid, String, NaiveDate, NaiveDate, Vec<String>, Option<i64>, String) = sqlx::query_as(
        "SELECT id, compte_id, eleve_id, reference, du, au, categories, plafond_centimes, statut
           FROM ent_fin_prise_en_charge WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(PriseEnChargeDto {
        id: r.0,
        compte_id: r.1,
        eleve_id: r.2,
        reference: r.3,
        du: r.4,
        au: r.5,
        categories: r.6,
        plafond_centimes: r.7,
        statut: r.8,
    })
}
