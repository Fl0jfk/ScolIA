//! Socle 2 (l'élève), lot E3 — identifiants externes de l'élève : dépôt et commandes.
//!
//! * ELV-07 [`PgEnt::ajouter_identifiant`], [`PgEnt::confirmer_identifiant`],
//!   [`PgEnt::contester_identifiant`], [`PgEnt::revoquer_identifiant`] ;
//! * ELV-41 [`PgEnt::declarer_badge_perdu`] ;
//! * lectures : [`PgEnt::identifiants_pour_acteur`] (G3), [`PgEnt::identifiants_contestes`]
//!   (qualité des données).
//!
//! Chaque commande = UNE transaction (`TenantTx`) : identifiant (fonction SQL unique
//! `ent_eleve_identifiant_enregistrer`) + événement + journal. **Aucune** valeur
//! d'identifiant dans un événement ni dans le journal : système, statuts, provenance.
//!
//! L'unicité ne joue qu'à l'intérieur de l'établissement (D-013) : toutes les requêtes
//! portent `etablissement_id`, aucune ne regarde ailleurs, et le résultat d'un ajout est
//! identique que la même valeur existe ou non dans un autre établissement.

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    motif_identifiant_valide, normaliser_identifiant, transition_identifiant_permise, DomainError,
    ErreurIdentifiant, ProvenanceIdentifiant, StatutIdentifiant, SystemeIdentifiant,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

fn aujourdhui() -> NaiveDate {
    domain_eleve::horloge::jour_paris()
}

/// Un identifiant de l'élève, tel que stocké.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IdentifiantEleve {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub systeme: String,
    pub valeur: String,
    pub statut: String,
    pub provenance: String,
    pub preuve_document_id: Option<Uuid>,
    pub valide_du: NaiveDate,
    pub valide_au: Option<NaiveDate>,
    pub confirme_le: Option<DateTime<Utc>>,
    pub confirme_par_source: Option<String>,
    pub motif: Option<String>,
    pub enregistre_le: DateTime<Utc>,
    pub enregistre_par: Option<Uuid>,
}

const COLONNES: &str = "id, etablissement_id, eleve_id, systeme, valeur, statut, provenance,
    preuve_document_id, valide_du, valide_au, confirme_le, confirme_par_source, motif,
    enregistre_le, enregistre_par";

/// Un identifiant contesté de l'établissement (écran de qualité des données).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IdentifiantConteste {
    pub identifiant_id: Uuid,
    pub eleve_id: Uuid,
    pub systeme: String,
    pub motif: Option<String>,
    /// Autre élève de l'établissement qui porte (actif) la même valeur : souvent un doublon.
    pub autre_eleve_id: Option<Uuid>,
}

/// Ajout d'un identifiant (ELV-07).
#[derive(Debug, Clone)]
pub struct AjoutIdentifiant {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub systeme: SystemeIdentifiant,
    pub valeur: String,
    pub provenance: ProvenanceIdentifiant,
    pub valide_du: Option<NaiveDate>,
    pub preuve_document_id: Option<Uuid>,
    pub acteur: Option<Uuid>,
}

/// Résultat d'une commande d'identifiant.
#[derive(Debug, Clone)]
pub struct ResultatIdentifiant {
    pub identifiant: IdentifiantEleve,
    /// La valeur est déjà portée par un autre élève : l'identifiant est enregistré `conteste`.
    pub collision: bool,
    pub collision_eleve_id: Option<Uuid>,
    /// Identifiant actif remplacé (révoqué) par cet ajout.
    pub remplace: Option<Uuid>,
    /// L'élève portait déjà cette valeur : rien n'a été écrit (idempotent).
    pub existant: bool,
}

fn erreur(e: ErreurIdentifiant) -> EntDbError {
    match e {
        ErreurIdentifiant::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurIdentifiant::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

/// Une contrainte ou la fonction SQL devient un refus lisible.
fn traduire(e: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &e {
        match d.code().as_deref() {
            Some("55000") => {
                return EntDbError::ConflitDossier(
                    "un identifiant confirmé existe : le révoquer avant d'en déclarer un autre",
                )
            }
            Some("23505") => {
                return EntDbError::ConflitDossier("cet identifiant est déjà enregistré")
            }
            Some("23503") => return EntDbError::SaisieInvalide("référence inconnue"),
            Some("23514") => return EntDbError::SaisieInvalide("valeur refusée par les règles"),
            Some("42501") => return EntDbError::Forbidden,
            _ => {}
        }
    }
    EntDbError::Sqlx(e)
}

struct PgIdentifiantRepo;

impl PgIdentifiantRepo {
    async fn verrouiller_eleve(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(r.is_some())
    }

    async fn lire(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        id: Uuid,
    ) -> Result<Option<IdentifiantEleve>, sqlx::Error> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_identifiant
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3"
        );
        sqlx::query_as::<_, IdentifiantEleve>(&sql)
            .bind(etab)
            .bind(eleve)
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
    }

    async fn de_l_eleve(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<IdentifiantEleve>, sqlx::Error> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_identifiant
             WHERE etablissement_id = $1 AND eleve_id = $2
             ORDER BY systeme, enregistre_le DESC, id"
        );
        sqlx::query_as::<_, IdentifiantEleve>(&sql)
            .bind(etab)
            .bind(eleve)
            .fetch_all(&mut *conn)
            .await
    }

    /// La pièce est-elle un document administratif ou familial de CET élève (jamais santé) ?
    async fn piece_de_l_eleve(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        document: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_document
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3
               AND tiroir IN ('administratif', 'famille') AND confidentialite <> 'sante'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(document)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(r.is_some())
    }

    #[allow(clippy::too_many_arguments)]
    async fn enregistrer(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        systeme: &str,
        valeur: &str,
        provenance: &str,
        statut: &str,
        valide_du: NaiveDate,
        acteur: Option<Uuid>,
        preuve: Option<Uuid>,
    ) -> Result<(Uuid, String, Option<Uuid>, Option<Uuid>, bool), sqlx::Error> {
        sqlx::query_as(
            "SELECT id, statut, collision_eleve, remplace, existant
             FROM ent_eleve_identifiant_enregistrer($1,$2,$3,$4,$5,$6,$7,$8,$9,false)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(systeme)
        .bind(valeur)
        .bind(provenance)
        .bind(statut)
        .bind(valide_du)
        .bind(acteur)
        .bind(preuve)
        .fetch_one(&mut *conn)
        .await
    }

    async fn changer_statut(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        id: Uuid,
        statut: &str,
        motif: Option<&str>,
        confirme_par: Option<&str>,
        jour: NaiveDate,
    ) -> Result<(), sqlx::Error> {
        let r = sqlx::query(
            "UPDATE ent_eleve_identifiant SET
                statut = $4,
                motif = coalesce($5, motif),
                valide_au = CASE WHEN $4 = 'revoque' THEN greatest(valide_du, $7) ELSE valide_au END,
                confirme_le = CASE WHEN $4 = 'confirme' THEN now() ELSE confirme_le END,
                confirme_par_source = CASE WHEN $4 = 'confirme' THEN $6 ELSE confirme_par_source END
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3",
        )
        .bind(etab)
        .bind(eleve)
        .bind(id)
        .bind(statut)
        .bind(motif)
        .bind(confirme_par)
        .bind(jour)
        .execute(&mut *conn)
        .await?;
        if r.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn projeter(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        systeme: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT ent_eleve_identifiant_projeter($1,$2,$3)")
            .bind(etab)
            .bind(eleve)
            .bind(systeme)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    /// Collision avec un autre élève de CET établissement (jamais ailleurs).
    async fn collision(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        systeme: &str,
        valeur: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let r: (Option<Uuid>,) =
            sqlx::query_as("SELECT ent_eleve_identifiant_collision($1,$2,$3,$4)")
                .bind(etab)
                .bind(eleve)
                .bind(systeme)
                .bind(valeur)
                .fetch_one(&mut *conn)
                .await?;
        Ok(r.0)
    }
}

/// Événement et journal d'une commande : jamais la valeur de l'identifiant.
async fn tracer(
    tx: &mut TenantTx<'static>,
    action: &'static str,
    id: &IdentifiantEleve,
    avant: Option<&str>,
    collision: bool,
    acteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    tx.publier(MetierEventDraft {
        etablissement_id: id.etablissement_id,
        type_name: types::ELEVE_IDENTIFIANT_CHANGE.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: id.eleve_id,
        eleve_id: Some(id.eleve_id),
        payload: json!({
            "identifiant_id": id.id,
            "systeme": id.systeme,
            "action": action,
            "statut_avant": avant,
            "statut_apres": id.statut,
            "provenance": id.provenance,
            "collision": collision,
        }),
        actor_user_id: acteur.map(|a| a.to_string()),
    })
    .await?;
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif',$4,$5,$6,false,$7, now())",
    )
    .bind(Uuid::new_v4())
    .bind(id.etablissement_id)
    .bind(id.eleve_id)
    .bind(format!("identifiant_{action}"))
    .bind("Identifiant de l'élève modifié")
    .bind(json!({ "identifiant_id": id.id, "systeme": id.systeme, "statut_apres": id.statut }))
    .bind(acteur)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl PgEnt {
    /// Droit d'écriture sur ce système : `eleves.identifiants.gerer` (tout système) ou, pour
    /// le seul badge, `eleves.badge.gerer` (vie scolaire, ELV-41). Portée établissement.
    async fn exiger_ecriture_identifiant(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        systeme: SystemeIdentifiant,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etab, acteur, role, Action::ElevesIdentifiantsGerer)
            .await?
        {
            return Ok(());
        }
        if systeme.est_badge()
            && self
                .a_action_etablissement(etab, acteur, role, Action::ElevesBadgeGerer)
                .await?
        {
            return Ok(());
        }
        Err(EntDbError::Forbidden)
    }

    async fn ouvrir_commande_identifiant(
        &self,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<TenantTx<'static>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if !PgIdentifiantRepo::verrouiller_eleve(&mut tx, etab, eleve).await? {
            return Err(DomainError::NotFound.into());
        }
        Ok(tx)
    }

    /// Cœur de l'ajout, dans la transaction de l'appelant (sans commit).
    pub(crate) async fn ecrire_identifiant(
        tx: &mut TenantTx<'static>,
        cmd: &AjoutIdentifiant,
    ) -> Result<ResultatIdentifiant, EntDbError> {
        let valeur = normaliser_identifiant(cmd.systeme, &cmd.valeur).map_err(erreur)?;
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        if let Some(p) = cmd.preuve_document_id {
            if !PgIdentifiantRepo::piece_de_l_eleve(&mut *tx, etab, eleve, p).await? {
                return Err(EntDbError::SaisieInvalide(
                    "pièce justificative inconnue ou non admise pour cet élève",
                ));
            }
        }
        let statut = cmd.provenance.statut_initial();
        let (id, statut_final, collision, remplace, existant) = PgIdentifiantRepo::enregistrer(
            &mut *tx,
            etab,
            eleve,
            cmd.systeme.code(),
            &valeur,
            cmd.provenance.code(),
            statut.code(),
            cmd.valide_du.unwrap_or_else(aujourdhui),
            cmd.acteur,
            cmd.preuve_document_id,
        )
        .await
        .map_err(traduire)?;
        let ligne = PgIdentifiantRepo::lire(&mut *tx, etab, eleve, id)
            .await?
            .ok_or(DomainError::NotFound)?;
        debug_assert_eq!(ligne.statut, statut_final);
        if !existant {
            tracer(tx, "ajoute", &ligne, None, collision.is_some(), cmd.acteur).await?;
        }
        Ok(ResultatIdentifiant {
            identifiant: ligne,
            collision: collision.is_some(),
            collision_eleve_id: collision,
            remplace,
            existant,
        })
    }

    /// ELV-07 : ajoute un identifiant saisi à la main (INE d'un bulletin, SIECLE…).
    /// Une collision n'échoue pas : l'identifiant est enregistré `conteste`.
    pub async fn ajouter_identifiant(
        &self,
        role: &str,
        cmd: AjoutIdentifiant,
    ) -> Result<ResultatIdentifiant, EntDbError> {
        let acteur = cmd.acteur.ok_or(EntDbError::Forbidden)?;
        self.exiger_ecriture_identifiant(cmd.etablissement_id, acteur, role, cmd.systeme)
            .await?;
        // Le badge a sa propre provenance ; les autres systèmes n'acceptent à la main que
        // une saisie, un document de la famille ou un dossier d'ancien établissement.
        let mut cmd = cmd;
        if cmd.systeme.est_badge() {
            cmd.provenance = ProvenanceIdentifiant::Badge;
        } else if !cmd.provenance.saisissable_a_la_main() {
            return Err(EntDbError::SaisieInvalide(
                "provenance réservée aux flux officiels",
            ));
        }
        let peut_lire = self
            .a_action_etablissement(cmd.etablissement_id, acteur, role, Action::ElevesIdentifiantsLire)
            .await?;
        let mut tx = self
            .ouvrir_commande_identifiant(cmd.etablissement_id, cmd.eleve_id)
            .await?;
        // Lot E5 : un identifiant géré par une source externe (INE sous SIECLE…) ne se saisit
        // pas localement ; un champ verrouillé non plus.
        if let Some(champ) = domain_eleve::champ_catalogue_du_systeme(cmd.systeme) {
            Self::exiger_champs_modifiables(&mut tx, cmd.etablissement_id, &[champ]).await?;
        }
        let mut r = Self::ecrire_identifiant(&mut tx, &cmd).await?;
        tx.commit().await?;
        // Un rôle qui ne lit pas les identifiants (badge seul) n'apprend pas quel autre élève
        // porte la valeur : il sait seulement qu'il y a collision.
        if !peut_lire {
            r.collision_eleve_id = None;
        }
        Ok(r)
    }

    /// Chemin **système** (socle 3, imports IOP, reprise) : sans contrôle de rôle ni
    /// restriction de provenance — l'appelant est un traitement du serveur, jamais une route.
    pub async fn ajouter_identifiant_systeme(
        &self,
        cmd: AjoutIdentifiant,
    ) -> Result<ResultatIdentifiant, EntDbError> {
        // Lot E5 : une donnée venue d'un flux officiel (import AAF/BEE/ONDE, retour BEE,
        // ÉduConnect) n'écrit jamais directement dans la fiche : elle passe par une proposition
        // (`deposer_proposition_systeme`), appliquée par une règle explicite ou une personne.
        if matches!(
            cmd.provenance,
            ProvenanceIdentifiant::ImportAaf
                | ProvenanceIdentifiant::ImportBee
                | ProvenanceIdentifiant::ImportOnde
                | ProvenanceIdentifiant::RetourBee
                | ProvenanceIdentifiant::Educonnect
                | ProvenanceIdentifiant::RepriseLogiciel
        ) {
            return Err(EntDbError::SaisieInvalide(
                "un flux officiel passe par une proposition (deposer_proposition_systeme)",
            ));
        }
        let mut tx = self
            .ouvrir_commande_identifiant(cmd.etablissement_id, cmd.eleve_id)
            .await?;
        // Le verrou et la source qui fait foi s'appliquent aussi aux traitements du serveur
        // (ex. INE déclaré par la famille à l'inscription, socle 3), sauf le badge.
        if let Some(champ) = domain_eleve::champ_catalogue_du_systeme(cmd.systeme) {
            Self::exiger_champs_modifiables(&mut tx, cmd.etablissement_id, &[champ]).await?;
        }
        let r = Self::ecrire_identifiant(&mut tx, &cmd).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Transition de statut (confirmer, contester, révoquer) dans une transaction.
    #[allow(clippy::too_many_arguments)]
    async fn transition_identifiant(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        id: Uuid,
        acteur: Uuid,
        vers: StatutIdentifiant,
        motif: Option<&str>,
        source: Option<&str>,
        action: &'static str,
    ) -> Result<IdentifiantEleve, EntDbError> {
        let mut tx = self.ouvrir_commande_identifiant(etab, eleve).await?;
        let actuel = PgIdentifiantRepo::lire(&mut tx, etab, eleve, id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let systeme = SystemeIdentifiant::parse(&actuel.systeme).ok_or(DomainError::NotFound)?;
        self.exiger_ecriture_identifiant(etab, acteur, role, systeme)
            .await?;
        let de = StatutIdentifiant::parse(&actuel.statut).ok_or(DomainError::NotFound)?;
        transition_identifiant_permise(de, vers).map_err(erreur)?;
        // Lot E5 : confirmer ou révoquer un identifiant géré par une source externe (ou sur un
        // champ verrouillé) est une écriture locale ; contester n'en est pas une (c'est un doute).
        if matches!(vers, StatutIdentifiant::Confirme | StatutIdentifiant::Revoque) {
            if let Some(champ) = domain_eleve::champ_catalogue_du_systeme(systeme) {
                Self::exiger_champs_modifiables(&mut tx, etab, &[champ]).await?;
            }
        }
        if vers == StatutIdentifiant::Confirme {
            // Un identifiant contesté ne redevient confirmé que si la collision a disparu.
            if systeme.unique_par_etablissement()
                && PgIdentifiantRepo::collision(&mut tx, etab, eleve, &actuel.systeme, &actuel.valeur)
                    .await?
                    .is_some()
            {
                return Err(EntDbError::ConflitDossier(
                    "un autre élève porte cette valeur : traiter le doublon d'abord",
                ));
            }
            // Un identifiant confirmé ne remplace pas un autre actif du même système :
            // l'ancien reste actif tant qu'il n'est pas révoqué (index d'unicité).
        }
        PgIdentifiantRepo::changer_statut(
            &mut tx,
            etab,
            eleve,
            id,
            vers.code(),
            motif,
            source,
            aujourdhui(),
        )
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                EntDbError::ConflitDossier("l'identifiant a changé, réessayer")
            }
            e => traduire(e),
        })?;
        PgIdentifiantRepo::projeter(&mut tx, etab, eleve, &actuel.systeme).await?;
        let apres = PgIdentifiantRepo::lire(&mut tx, etab, eleve, id)
            .await?
            .ok_or(DomainError::NotFound)?;
        tracer(&mut tx, action, &apres, Some(&actuel.statut), false, Some(acteur)).await?;
        tx.commit().await?;
        Ok(apres)
    }

    /// ELV-07 : confirme un identifiant (`source` : vérification, flux officiel…).
    pub async fn confirmer_identifiant(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        id: Uuid,
        acteur: Uuid,
        source: &str,
    ) -> Result<IdentifiantEleve, EntDbError> {
        let source = source.trim();
        if source.is_empty() || source.chars().count() > 80 {
            return Err(EntDbError::SaisieInvalide(
                "source de confirmation obligatoire (80 caractères au plus)",
            ));
        }
        self.transition_identifiant(
            role,
            etab,
            eleve,
            id,
            acteur,
            StatutIdentifiant::Confirme,
            None,
            Some(source),
            "confirme",
        )
        .await
    }

    /// ELV-07 : conteste un identifiant (motif obligatoire).
    pub async fn contester_identifiant(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        id: Uuid,
        acteur: Uuid,
        motif: Option<&str>,
    ) -> Result<IdentifiantEleve, EntDbError> {
        let motif = motif_identifiant_valide(motif).map_err(erreur)?;
        self.transition_identifiant(
            role,
            etab,
            eleve,
            id,
            acteur,
            StatutIdentifiant::Conteste,
            Some(&motif),
            None,
            "conteste",
        )
        .await
    }

    /// ELV-07 : révoque un identifiant (motif obligatoire ; la ligne est conservée).
    pub async fn revoquer_identifiant(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        id: Uuid,
        acteur: Uuid,
        motif: Option<&str>,
    ) -> Result<IdentifiantEleve, EntDbError> {
        let motif = motif_identifiant_valide(motif).map_err(erreur)?;
        self.transition_identifiant(
            role,
            etab,
            eleve,
            id,
            acteur,
            StatutIdentifiant::Revoque,
            Some(&motif),
            None,
            "revoque",
        )
        .await
    }

    /// ELV-41 : badge perdu — l'ancien est révoqué tout de suite (cantine, portail) et, si
    /// fourni, le nouveau est actif dans la même transaction.
    pub async fn declarer_badge_perdu(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        acteur: Uuid,
        nouveau: Option<&str>,
    ) -> Result<Option<ResultatIdentifiant>, EntDbError> {
        self.exiger_ecriture_identifiant(etab, acteur, role, SystemeIdentifiant::Badge)
            .await?;
        let mut tx = self.ouvrir_commande_identifiant(etab, eleve).await?;
        let actifs: Vec<IdentifiantEleve> = PgIdentifiantRepo::de_l_eleve(&mut tx, etab, eleve)
            .await?
            .into_iter()
            .filter(|i| {
                i.systeme == "BADGE"
                    && i.valide_au.is_none()
                    && StatutIdentifiant::parse(&i.statut).is_some_and(|s| s != StatutIdentifiant::Revoque)
            })
            .collect();
        if actifs.is_empty() && nouveau.is_none() {
            return Err(DomainError::NotFound.into());
        }
        for a in actifs {
            PgIdentifiantRepo::changer_statut(
                &mut tx,
                etab,
                eleve,
                a.id,
                "revoque",
                Some("badge perdu"),
                None,
                aujourdhui(),
            )
            .await
            .map_err(traduire)?;
            PgIdentifiantRepo::projeter(&mut tx, etab, eleve, "BADGE").await?;
            let apres = PgIdentifiantRepo::lire(&mut tx, etab, eleve, a.id)
                .await?
                .ok_or(DomainError::NotFound)?;
            tracer(&mut tx, "revoque", &apres, Some(&a.statut), false, Some(acteur)).await?;
        }
        let resultat = match nouveau {
            Some(valeur) => Some(
                Self::ecrire_identifiant(
                    &mut tx,
                    &AjoutIdentifiant {
                        etablissement_id: etab,
                        eleve_id: eleve,
                        systeme: SystemeIdentifiant::Badge,
                        valeur: valeur.to_string(),
                        provenance: ProvenanceIdentifiant::Badge,
                        valide_du: None,
                        preuve_document_id: None,
                        acteur: Some(acteur),
                    },
                )
                .await?,
            ),
            None => None,
        };
        tx.commit().await?;
        // Sans droit de lecture des identifiants, on ne dit pas QUEL autre élève porte la valeur.
        let mut resultat = resultat;
        if let Some(r) = resultat.as_mut() {
            if !self
                .a_action_etablissement(etab, acteur, role, Action::ElevesIdentifiantsLire)
                .await?
            {
                r.collision_eleve_id = None;
            }
        }
        Ok(resultat)
    }

    /// G3 : identifiants d'un élève (historique compris : révoqués, contestés). Il faut
    /// `eleves.identifiants.lire` sur l'élève ; la lecture est journalisée (acteur et objet,
    /// jamais la valeur).
    pub async fn identifiants_pour_acteur(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<IdentifiantEleve>, EntDbError> {
        if !self
            .autoriser_eleve(etab, acteur, role, Action::ElevesIdentifiantsLire, eleve)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let existe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *conn)
                .await?;
        if existe.is_none() {
            return Err(DomainError::NotFound.into());
        }
        let l = PgIdentifiantRepo::de_l_eleve(&mut conn, etab, eleve).await?;
        drop(conn);
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            Some(eleve),
            "identifiants",
            "consultation",
            "Consultation des identifiants de l'élève",
        )
        .await;
        Ok(l)
    }

    /// Qualité des données : identifiants contestés de l'établissement (collisions…),
    /// avec l'autre élève concerné. `eleves.identifiants.lire` sur l'établissement entier.
    pub async fn identifiants_contestes(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<IdentifiantConteste>, EntDbError> {
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ElevesIdentifiantsLire)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let l = sqlx::query_as::<_, IdentifiantConteste>(
            "SELECT i.id AS identifiant_id, i.eleve_id, i.systeme, i.motif,
                    ent_eleve_identifiant_collision(i.etablissement_id, i.eleve_id, i.systeme, i.valeur) AS autre_eleve_id
             FROM ent_eleve_identifiant i
             WHERE i.etablissement_id = $1 AND i.statut = 'conteste' AND i.valide_au IS NULL
             ORDER BY i.enregistre_le DESC, i.id
             LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            None,
            "identifiants",
            "qualite",
            "Consultation des identifiants contestés",
        )
        .await;
        Ok(l)
    }
}
