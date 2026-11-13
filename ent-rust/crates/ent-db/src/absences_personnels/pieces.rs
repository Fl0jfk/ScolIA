//! Socle 15, lot A2 — pièces, fil, conservation
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §18, §21.3, §26, R-AP21, R-AP22, R-AP40 ; ABP-140 à 149, 160 à 164).
//!
//! Tout ici relève du niveau **RH** (§18.1) : l'agent lui-même, le valideur ou l'instructeur du circuit, la
//! direction et, selon l'employeur, le secrétariat (État) ou la comptabilité (OGEC) ; **personne d'autre,
//! administrateur technique compris**. Un acteur hors cercle reçoit « introuvable » (pas d'oracle).
//!
//! | Commande                                   | Qui                                                      | ABP        |
//! |--------------------------------------------|----------------------------------------------------------|------------|
//! | `deposer_piece_absence_for_actor`          | l'agent ou l'instructeur                                  | 140        |
//! | `relancer_piece_absence_for_actor`         | l'instructeur ; plafond S-1502                            | 141        |
//! | `lien_piece_absence_for_actor`             | le cercle ; lien 5 min au plus ; lecture journalisée      | 142        |
//! | `retirer_piece_absence_for_actor`          | l'agent avant traitement, l'instructeur                   | 143        |
//! | `lectures_pieces_absence_for_actor`        | l'agent seul : qui a lu mes pièces                        | 144        |
//! | `poster_message_` / `fil_absence_for_actor`| le cercle ; ajout seul, 4 000 caractères                   | 145, 146   |
//! | `echeances_absences_for_actor`             | l'instructeur : arrêts à recevoir, pièces manquantes      | 147        |
//! | `purger_echeances` (tâche)                 | système ; purge à la date, jamais sous gel                | 160        |
//! | `purger_piece_absence_for_actor`           | `conserver` ; motif codé                                  | 161        |
//! | `purges_a_venir_for_actor`                 | `conserver`                                               | 162        |
//! | `poser_` / `lever_gel_absence_for_actor`   | `conserver` (direction) ; signalé par événement           | 163        |
//! | `definir_confidentialite_renforcee_`       | `conserver`                                               | 164        |
//!
//! **Règles tenues** : R-AP21 (30 jours après la transmission pour l'État ; 3 ans après la fin de l'année
//! scolaire pour l'OGEC ; purges datées et journalisées ; gel possible), R-AP22 (jamais en pièce jointe d'un
//! mail ni d'une notification : seulement un lien pré-signé court), R-AP40 (aucune lecture par l'IA : ce
//! module n'expose rien à `ent-ia`), R-AP2 (aucun nom de fichier conservé, aucun DTO d'organisation ne porte
//! une pièce).
//!
//! **Limite assumée** : la purge d'une pièce est une suppression **logique et irréversible** dans
//! l'application (état `purgee`, ligne `ent_objet.supprime_le`, plus aucun lien possible). L'effacement
//! physique des octets chez le fournisseur n'a pas d'API dans `ent-object` (objets immuables, PRA E5, D-043) :
//! il relève du cycle de vie du seau, décision de sauvegarde et du DPO (Q-AP12) encore ouverte.

use super::*;
use crate::cahier::FichierDepot;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};
use ent_object::presign::{Disposition, LinkSpec, PresignError};
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;

/// Nombre maximal de pièces d'une absence (retirées comprises) et de messages de son fil.
pub const MAX_PIECES_ABSENCE: i64 = 20;
pub const MAX_MESSAGES_ABSENCE: i64 = 500;
/// Durée de vie maximale d'un lien vers une pièce RH (§18.2).
pub const TTL_MAX_LIEN_PIECE_SECS: u64 = 300;
const TYPES_PIECE: &[&str] = &["arret", "justificatif", "convocation", "autre"];
const MOTIFS_PURGE: &[&str] = &["demande_agent", "erreur_depot", "contentieux_clos", "autre"];

// ---------------------------------------------------------------------------------------------
// Dates de conservation (pures : testées sans base)
// ---------------------------------------------------------------------------------------------

/// Fin de l'année scolaire (31 août) qui contient `jour`, à défaut d'une année déclarée à l'établissement.
pub fn fin_annee_scolaire_par_defaut(jour: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    let annee = if jour.month() >= 9 { jour.year() + 1 } else { jour.year() };
    NaiveDate::from_ymd_opt(annee, 8, 31).expect("31 août existe")
}

/// `date` + `annees` années calendaires (le 29 février devient le 28).
pub fn ajouter_annees(date: NaiveDate, annees: u32) -> NaiveDate {
    use chrono::Datelike;
    let an = date.year() + annees as i32;
    NaiveDate::from_ymd_opt(an, date.month(), date.day())
        .or_else(|| NaiveDate::from_ymd_opt(an, date.month(), date.day() - 1))
        .expect("date valide")
}

/// Date de purge d'une pièce (R-AP21, §18.4). `None` tant qu'elle n'est pas connue : pour un agent de l'État
/// elle dépend de la transmission au rectorat, qui n'a peut-être pas encore eu lieu.
pub fn date_purge_piece(
    employeur: &str,
    transmise_le: Option<NaiveDate>,
    fin_annee: NaiveDate,
    jours_etat: u32,
    annees_ogec: u32,
) -> Option<NaiveDate> {
    if employeur == "etat" {
        transmise_le.map(|t| t + Duration::days(jours_etat as i64))
    } else {
        Some(ajouter_annees(fin_annee, annees_ogec))
    }
}

// ---------------------------------------------------------------------------------------------
// Accès : le cercle RH
// ---------------------------------------------------------------------------------------------

struct Acces {
    absence: AbsenceLigne,
    suis_absent: bool,
    valideur: bool,
    instructeur: bool,
    lecteur_nature: bool,
}

impl Acces {
    fn cercle(&self) -> bool {
        self.suis_absent || self.valideur || self.instructeur || self.lecteur_nature
    }
    fn role_affiche(&self) -> &'static str {
        if self.suis_absent {
            "agent"
        } else if self.instructeur {
            "instructeur"
        } else if self.valideur {
            "valideur"
        } else {
            "direction"
        }
    }
}

fn piece_dto(r: PieceLigne) -> PieceAbsenceDto {
    PieceAbsenceDto {
        id: r.id,
        absence_id: r.absence_id,
        type_piece: r.type_piece,
        etat: r.etat,
        taille_octets: r.taille_octets,
        mime: r.mime,
        deposee_par_user_id: r.deposee_par_user_id,
        deposee_le: r.deposee_le,
        retiree_le: r.retiree_le,
        purge_prevue_le: r.purge_prevue_le,
        purgee_le: r.purgee_le,
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PieceLigne {
    id: Uuid,
    absence_id: Uuid,
    type_piece: String,
    cle_objet: String,
    taille_octets: i64,
    mime: String,
    etat: String,
    deposee_par_user_id: Uuid,
    deposee_le: DateTime<Utc>,
    retiree_le: Option<DateTime<Utc>>,
    purge_prevue_le: Option<NaiveDate>,
    purgee_le: Option<DateTime<Utc>>,
}

const PIECE_COLONNES: &str = "id, absence_id, type_piece, cle_objet, taille_octets, mime, etat,
    deposee_par_user_id, deposee_le, retiree_le, purge_prevue_le, purgee_le";

async fn jour_etab(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    Ok(j)
}

/// Un gel actif suspend toute purge de l'absence (R-AP21).
async fn gel_actif(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    let (b,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_absp_gel WHERE etablissement_id = $1 AND absence_id = $2
                          AND leve_le IS NULL AND du <= $3 AND (au IS NULL OR au >= $3))",
    )
    .bind(etab)
    .bind(absence_id)
    .bind(jour)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(b)
}

/// Date de purge d'une pièce de cette absence, selon l'employeur et les paramètres de l'établissement.
async fn date_purge_calculee(
    conn: &mut PgConnection,
    etab: Uuid,
    a: &AbsenceLigne,
) -> Result<Option<NaiveDate>, EntDbError> {
    let (jours_etat, annees_ogec): (i16, i16) = sqlx::query_as(
        "SELECT COALESCE((SELECT purge_piece_etat_jours FROM ent_absp_param WHERE etablissement_id = $1), 30),
                COALESCE((SELECT purge_piece_ogec_annees FROM ent_absp_param WHERE etablissement_id = $1), 3)",
    )
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let transmise: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT ent_aujourdhui_etab($1, max(le)) FROM ent_absp_decision
          WHERE etablissement_id = $1 AND absence_id = $2 AND type = 'transmission'
            AND voie_transmission IN ('rectorat', 'dsden')",
    )
    .bind(etab)
    .bind(a.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let debut: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(a.debut)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let fin_annee: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
          ORDER BY date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(debut)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(date_purge_piece(
        &a.employeur,
        transmise,
        fin_annee.unwrap_or_else(|| fin_annee_scolaire_par_defaut(debut)),
        jours_etat as u32,
        annees_ogec as u32,
    ))
}

/// Purge (logique, irréversible) d'une pièce, dans la transaction de l'appelant : état `purgee`, objet
/// marqué supprimé au registre, journal. Le fichier n'est plus jamais servi.
async fn purger_piece_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    p: &PieceLigne,
    cause: &str,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_absp_piece SET etat = 'purgee', purgee_le = now(), purgee_cause = $3, purgee_par_user_id = $4
          WHERE etablissement_id = $1 AND id = $2 AND etat <> 'purgee'",
    )
    .bind(etab)
    .bind(p.id)
    .bind(cause)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_objet SET supprime_le = now()
          WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NULL",
    )
    .bind(etab)
    .bind(&p.cle_objet)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journal(conn, etab, p.absence_id, "piece_purgee", Some("cause"), Some(&p.type_piece), Some(cause), actor).await
}

impl PgEnt {
    /// Ouvre la transaction et contrôle l'appartenance au cercle RH : « introuvable » sinon.
    async fn ouvrir_acces_rh(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<(TenantTx<'static>, Acces), EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, absence_id, false).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let valideur = match mon_agent {
            Some(m) => {
                let (b,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_absp_circuit WHERE etablissement_id = $1 AND absence_id = $2
                                     AND agent_id = $3 AND role = 'valideur')",
                )
                .bind(etab)
                .bind(absence_id)
                .bind(m)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                b
            }
            None => false,
        };
        let instructeur = self.peut_instruire_abs(&mut tx, etab, actor, role, &a).await?;
        let lecteur_nature = lire_nature && (matches!(r, Role::Direction) || instruit(r, &a.employeur));
        let acces = Acces { absence: a, suis_absent, valideur, instructeur, lecteur_nature };
        if !acces.cercle() {
            return Err(introuvable());
        }
        Ok((tx, acces))
    }

    async fn piece_de_absence(
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        piece_id: Uuid,
        verrou: bool,
    ) -> Result<PieceLigne, EntDbError> {
        sqlx::query_as::<_, PieceLigne>(&format!(
            "SELECT {PIECE_COLONNES} FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2 AND id = $3 {}",
            if verrou { "FOR UPDATE" } else { "" }
        ))
        .bind(etab)
        .bind(absence_id)
        .bind(piece_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    // ───────────────────────────── ABP-140 : déposer une pièce ─────────────────────────────

    /// Dépose une pièce : l'agent pour lui-même, ou l'instructeur. Type et taille contrôlés par signature
    /// binaire (D2) ; le fichier part dans `ent-object` (objet immuable, empreinte) ; **aucun nom de fichier
    /// n'est conservé**. Pour un arrêt, l'agent atteste n'avoir déposé que les volets 2 et 3 (R-08).
    pub async fn deposer_piece_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        type_piece: &str,
        volet_1_exclu: bool,
        fichier: FichierDepot,
    ) -> Result<PieceAbsenceDto, EntDbError> {
        if !TYPES_PIECE.contains(&type_piece) {
            return Err(invalide("type de pièce inconnu"));
        }
        if type_piece == "arret" && !volet_1_exclu {
            return Err(invalide(
                "pour un arrêt, ne déposez que les volets 2 et 3, jamais le volet 1 (il porte le diagnostic)",
            ));
        }
        let (mut tx, acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        if !(acces.suis_absent || acces.instructeur) {
            return Err(EntDbError::Forbidden);
        }
        if acces.absence.etat == "annulee" {
            return Err(conflit("absence annulée : aucune pièce ne peut être déposée"));
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n >= MAX_PIECES_ABSENCE {
            return Err(conflit("nombre maximal de pièces atteint pour cette absence"));
        }
        let valide = self
            .politique_fichiers()
            .valider(UsageFichier::Document, &fichier.nom, Some(&fichier.mime_type), &fichier.contenu)
            .map_err(EntDbError::Fichier)?;
        let id = Uuid::new_v4();
        let cle = ObjectStore::key_for(etab, "absence_personnel", id, valide.extension);
        let purge = date_purge_calculee(&mut tx, etab, &acces.absence).await?;
        // Ligne d'abord, objet ensuite, dans la même transaction : un échec du stockage annule la ligne.
        sqlx::query(
            "INSERT INTO ent_absp_piece (id, etablissement_id, absence_id, type_piece, cle_objet, sha256,
                    taille_octets, mime, deposee_par_user_id, purge_prevue_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(absence_id)
        .bind(type_piece)
        .bind(&cle)
        .bind(ent_object::sha256_hex(&fichier.contenu))
        .bind(fichier.contenu.len() as i64)
        .bind(valide.mime)
        .bind(actor)
        .bind(purge)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let info = self.objet_put(&cle, &fichier.contenu).await?;
        crate::objets::enregistrer_objet(&mut *tx, etab, &info, Some(valide.mime)).await?;
        journal(&mut tx, etab, absence_id, "piece_deposee", Some("type_piece"), None, Some(type_piece), actor).await?;
        let r = Self::piece_de_absence(&mut tx, etab, absence_id, id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(piece_dto(r))
    }

    /// Les pièces d'une absence (états compris, sans contenu ni nom de fichier).
    pub async fn pieces_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<PieceAbsenceDto>, EntDbError> {
        let (mut tx, _acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        let rows: Vec<PieceLigne> = sqlx::query_as(&format!(
            "SELECT {PIECE_COLONNES} FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2
              ORDER BY deposee_le, id"
        ))
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(piece_dto).collect())
    }

    // ───────────────────────────── ABP-142 : lire une pièce ─────────────────────────────

    /// Lien pré-signé court vers une pièce, après contrôle du droit ; **la lecture est journalisée** avant
    /// l'émission du lien (pas de lien sans trace). Jamais en pièce jointe d'un mail (R-AP22).
    pub async fn lien_piece_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        piece_id: Uuid,
    ) -> Result<LienPieceDto, EntDbError> {
        if !self.presigner().is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        // Un lien RH ne dépasse jamais 5 minutes : un réglage plus long du fournisseur de liens le refuse.
        if self.presigner().ttl_secs() > TTL_MAX_LIEN_PIECE_SECS {
            return Err(EntDbError::LiensIndisponibles);
        }
        let (mut tx, _acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        let p = Self::piece_de_absence(&mut tx, etab, absence_id, piece_id, false).await?;
        if p.etat != "deposee" {
            return Err(EntDbError::FichierAbsent);
        }
        sqlx::query(
            "INSERT INTO ent_absp_piece_lecture (id, etablissement_id, piece_id, lecteur_user_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(piece_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (ext,) = (p.cle_objet.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin").to_string(),);
        let link = self
            .presigner()
            .issue(
                &LinkSpec {
                    key: p.cle_objet.clone(),
                    user_id: actor,
                    etablissement_id: etab,
                    disposition: Disposition::Attachment,
                    // Nom générique : le nom d'origine n'est jamais conservé (minimisation).
                    filename: format!("piece-{}.{ext}", p.type_piece),
                    mime: p.mime.clone(),
                },
                ent_object::presign::now_unix(),
            )
            .map_err(|e| match e {
                PresignError::Disabled => EntDbError::LiensIndisponibles,
                _ => EntDbError::FichierAbsent,
            })?;
        if !self.objet_existe(&p.cle_objet).await? {
            return Err(EntDbError::FichierAbsent);
        }
        tx.commit().await.map_err(db)?;
        Ok(LienPieceDto { url: link.url, expires_at: link.expires_at })
    }

    // ───────────────────────────── ABP-143 : retirer une pièce ─────────────────────────────

    /// Retire une pièce déposée par erreur : l'agent tant que l'absence n'est pas traitée, l'instructeur
    /// ensuite. La pièce passe `retiree` (jamais supprimée de la base) et sa purge est planifiée au prochain
    /// cycle (sauf gel).
    pub async fn retirer_piece_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        piece_id: Uuid,
    ) -> Result<PieceAbsenceDto, EntDbError> {
        let (mut tx, acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        let avant_traitement = !matches!(acces.absence.etat.as_str(), "traitee" | "close" | "annulee");
        if !(acces.instructeur || (acces.suis_absent && avant_traitement)) {
            return Err(EntDbError::Forbidden);
        }
        let p = Self::piece_de_absence(&mut tx, etab, absence_id, piece_id, true).await?;
        if p.etat != "deposee" {
            return Err(conflit("cette pièce n'est plus déposée"));
        }
        let jour = jour_etab(&mut tx, etab).await?;
        sqlx::query(
            "UPDATE ent_absp_piece SET etat = 'retiree', retiree_le = now(), retiree_par_user_id = $3,
                    purge_prevue_le = LEAST(COALESCE(purge_prevue_le, $4), $4)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(piece_id)
        .bind(actor)
        .bind(jour)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(&mut tx, etab, absence_id, "piece_retiree", Some("type_piece"), Some(&p.type_piece), None, actor).await?;
        let r = Self::piece_de_absence(&mut tx, etab, absence_id, piece_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(piece_dto(r))
    }

    // ───────────────────────────── ABP-141 : relancer la pièce ─────────────────────────────

    /// Relance l'agent pour une pièce attendue qui manque. Plafonnée (S-1502) ; l'événement ne porte que des
    /// identifiants (la notification envoie un lien, jamais la nature). Renvoie le nombre de relances faites.
    pub async fn relancer_piece_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<u32, EntDbError> {
        let (mut tx, acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        if !acces.instructeur {
            return Err(EntDbError::Forbidden);
        }
        let a = &acces.absence;
        let attendue: Option<String> = match a.nature_id {
            Some(n) => sqlx::query_scalar(
                "SELECT piece_attendue FROM ent_absp_nature
                  WHERE id = $1 AND (etablissement_id IS NULL OR etablissement_id = $2)",
            )
                .bind(n)
                .bind(etab)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?,
            None => None,
        };
        let attendue = attendue.filter(|t| t != "aucune").ok_or_else(|| conflit("aucune pièce n'est attendue pour cette absence"))?;
        let (deposee,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2 AND etat = 'deposee')",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deposee {
            return Err(conflit("une pièce est déjà déposée"));
        }
        let (max,): (i16,) = sqlx::query_as(
            "SELECT COALESCE((SELECT relances_piece_max FROM ent_absp_param WHERE etablissement_id = $1), 3)",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_absp_journal WHERE etablissement_id = $1 AND objet = 'absence'
                AND objet_id = $2 AND action = 'piece_relancee'",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja >= max as i64 {
            return Err(conflit("plafond de relances atteint"));
        }
        journal(&mut tx, etab, absence_id, "piece_relancee", Some("type_piece"), None, Some(&attendue), actor).await?;
        let ev = evenement(
            &mut tx,
            etab,
            "absence_personnel.piece_relancee",
            absence_id,
            a.agent_id,
            json!({ "relance": deja + 1 }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok((deja + 1) as u32)
    }

    // ───────────────────────────── ABP-144 : qui a lu mes pièces ─────────────────────────────

    /// L'agent voit qui a lu ses pièces, et quand (CAP-60). Lui seul : le reste du cercle ne lit pas ce
    /// journal. Les lectures survivent à la purge de la pièce.
    pub async fn lectures_pieces_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<LecturePieceDto>, EntDbError> {
        let (mut tx, acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        if !acces.suis_absent {
            return Err(EntDbError::Forbidden);
        }
        let rows: Vec<(Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT l.piece_id, l.lecteur_user_id, l.le FROM ent_absp_piece_lecture l
               JOIN ent_absp_piece p ON p.etablissement_id = l.etablissement_id AND p.id = l.piece_id
              WHERE l.etablissement_id = $1 AND p.absence_id = $2 ORDER BY l.le, l.id",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(piece_id, lecteur_user_id, le)| LecturePieceDto { piece_id, lecteur_user_id, le })
            .collect())
    }

    // ───────────────────────────── ABP-145, 146 : le fil ─────────────────────────────

    /// Écrit dans le fil de l'absence (ajout seul, 4 000 caractères, jamais de pièce jointe). Le rôle affiché
    /// est calculé par le serveur.
    pub async fn poster_message_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        texte: &str,
    ) -> Result<MessageAbsenceDto, EntDbError> {
        let texte = texte.trim();
        if texte.is_empty() {
            return Err(invalide("message vide"));
        }
        if texte.chars().count() > 4000 {
            return Err(invalide("message trop long (4 000 caractères au plus)"));
        }
        let (mut tx, acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        if acces.absence.etat == "annulee" {
            return Err(conflit("absence annulée : le fil est fermé"));
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_absp_message WHERE etablissement_id = $1 AND absence_id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n >= MAX_MESSAGES_ABSENCE {
            return Err(conflit("fil plein"));
        }
        let (id, le): (Uuid, DateTime<Utc>) = sqlx::query_as(
            "INSERT INTO ent_absp_message (id, etablissement_id, absence_id, auteur_user_id, role_affiche, texte)
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id, le",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(absence_id)
        .bind(actor)
        .bind(acces.role_affiche())
        .bind(texte)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        journal(&mut tx, etab, absence_id, "message_poste", Some("role"), None, Some(acces.role_affiche()), actor).await?;
        tx.commit().await.map_err(db)?;
        Ok(MessageAbsenceDto {
            id,
            auteur_user_id: actor,
            role_affiche: acces.role_affiche().to_string(),
            texte: texte.to_string(),
            le,
        })
    }

    pub async fn fil_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<MessageAbsenceDto>, EntDbError> {
        let (mut tx, _acces) = self.ouvrir_acces_rh(etab, actor, role, absence_id).await?;
        let rows: Vec<(Uuid, Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, auteur_user_id, role_affiche, texte, le FROM ent_absp_message
              WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY le, id",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, auteur_user_id, role_affiche, texte, le)| MessageAbsenceDto {
                id,
                auteur_user_id,
                role_affiche,
                texte,
                le,
            })
            .collect())
    }

    // ───────────────────────────── ABP-147 : échéances ─────────────────────────────

    /// Échéances à suivre par l'instructeur : arrêts non encore reçus (48 h, R-AP16) et pièces attendues
    /// manquantes (délai de la nature). Cloisonné par employeur comme l'instruction (§23.3). Les créneaux et
    /// la DSN (J+5) relèvent des lots suivants.
    pub async fn echeances_absences_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<EcheanceAbsenceDto>, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesInstruire).await? {
            return Err(EntDbError::Forbidden);
        }
        let employeur: Option<&str> = match r {
            Role::Administratif => Some("etat"),
            Role::Compta => Some("ogec"),
            _ => None,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut sortie: Vec<EcheanceAbsenceDto> = Vec::new();
        let arrets: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT a.absence_id, a.echeance_le FROM ent_absp_arret a
               JOIN ent_absp_absence x ON x.etablissement_id = a.etablissement_id AND x.id = a.absence_id
              WHERE a.etablissement_id = $1 AND a.recu_le IS NULL AND a.echeance_le IS NOT NULL
                AND x.etat NOT IN ('annulee', 'refusee', 'close')
                AND ($2::text IS NULL OR x.employeur = $2) ORDER BY a.echeance_le LIMIT 200",
        )
        .bind(etab)
        .bind(employeur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (absence_id, echeance_le) in arrets {
            let relances = relances_de(&mut tx, etab, absence_id).await?;
            sortie.push(EcheanceAbsenceDto { absence_id, genre: "arret_a_recevoir".into(), echeance_le, relances });
        }
        let manquantes: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT x.id, x.cree_le + make_interval(hours => n.delai_piece_heures)
               FROM ent_absp_absence x JOIN ent_absp_nature n ON n.id = x.nature_id
                    AND (n.etablissement_id IS NULL OR n.etablissement_id = x.etablissement_id)
              WHERE x.etablissement_id = $1 AND n.piece_attendue <> 'aucune' AND n.delai_piece_heures IS NOT NULL
                AND x.etat NOT IN ('annulee', 'refusee', 'close', 'signalee')
                AND NOT EXISTS (SELECT 1 FROM ent_absp_piece p WHERE p.etablissement_id = x.etablissement_id
                                  AND p.absence_id = x.id AND p.etat = 'deposee')
                AND ($2::text IS NULL OR x.employeur = $2)
              ORDER BY 2 LIMIT 200",
        )
        .bind(etab)
        .bind(employeur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (absence_id, echeance_le) in manquantes {
            let relances = relances_de(&mut tx, etab, absence_id).await?;
            sortie.push(EcheanceAbsenceDto { absence_id, genre: "piece_manquante".into(), echeance_le, relances });
        }
        sortie.sort_by_key(|e| e.echeance_le);
        Ok(sortie)
    }

    // ───────────────────────────── ABP-163 : gel de conservation ─────────────────────────────

    async fn exiger_conserver(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesConserver).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    /// Pose un gel (contentieux) : suspend la purge des pièces et du fil de cette absence. Motif obligatoire,
    /// journalisé ; l'événement (identifiants seulement) signale le gel au DPO.
    pub async fn poser_gel_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        motif: &str,
        du: NaiveDate,
        au: Option<NaiveDate>,
    ) -> Result<GelDto, EntDbError> {
        self.exiger_conserver(etab, actor, role).await?;
        let motif = motif.trim();
        if motif.chars().count() < 3 {
            return Err(invalide("le motif du gel est obligatoire"));
        }
        if motif.chars().count() > 500 {
            return Err(invalide("motif trop long (500 caractères au plus)"));
        }
        if let Some(fin) = au {
            if fin < du {
                return Err(invalide("la fin du gel précède son début"));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, absence_id, true).await?;
        let (id, pose_le): (Uuid, DateTime<Utc>) = sqlx::query_as(
            "INSERT INTO ent_absp_gel (id, etablissement_id, absence_id, motif, du, au, pose_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id, pose_le",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(absence_id)
        .bind(motif)
        .bind(du)
        .bind(au)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        journal(&mut tx, etab, absence_id, "gel_pose", Some("du"), None, Some(&du.to_string()), actor).await?;
        let ev = evenement(&mut tx, etab, "absence_personnel.gel_pose", absence_id, a.agent_id, json!({ "gel_id": id }), actor).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(GelDto {
            id,
            absence_id,
            motif: motif.to_string(),
            du,
            au,
            pose_par_user_id: actor,
            pose_le,
            leve_le: None,
        })
    }

    pub async fn lever_gel_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        gel_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_conserver(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_absp_gel SET leve_le = now(), leve_par_user_id = $4
              WHERE etablissement_id = $1 AND absence_id = $2 AND id = $3 AND leve_le IS NULL",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(gel_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journal(&mut tx, etab, absence_id, "gel_leve", None, None, None, actor).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn gels_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<GelDto>, EntDbError> {
        self.exiger_conserver(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String, NaiveDate, Option<NaiveDate>, Uuid, DateTime<Utc>, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                "SELECT id, motif, du, au, pose_par_user_id, pose_le, leve_le FROM ent_absp_gel
                  WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY pose_le, id",
            )
            .bind(etab)
            .bind(absence_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, motif, du, au, pose_par_user_id, pose_le, leve_le)| GelDto {
                id,
                absence_id,
                motif,
                du,
                au,
                pose_par_user_id,
                pose_le,
                leve_le,
            })
            .collect())
    }

    // ───────────────────────────── ABP-161, 162 : purge à la demande, purges à venir ─────────────────────────────

    /// Purges à venir (pièces non purgées dont la date est connue et ≤ `jusqu_au`, ou inconnue), avec la
    /// mention d'un gel actif.
    pub async fn purges_a_venir_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jusqu_au: NaiveDate,
    ) -> Result<Vec<PurgeAVenirDto>, EntDbError> {
        self.exiger_conserver(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_etab(&mut tx, etab).await?;
        let rows: Vec<(Uuid, Uuid, String, String, Option<NaiveDate>, bool)> = sqlx::query_as(
            "SELECT p.absence_id, p.id, p.type_piece, p.etat, p.purge_prevue_le,
                    EXISTS (SELECT 1 FROM ent_absp_gel g WHERE g.etablissement_id = p.etablissement_id
                              AND g.absence_id = p.absence_id AND g.leve_le IS NULL AND g.du <= $3
                              AND (g.au IS NULL OR g.au >= $3))
               FROM ent_absp_piece p
              WHERE p.etablissement_id = $1 AND p.etat <> 'purgee'
                AND (p.purge_prevue_le IS NULL OR p.purge_prevue_le <= $2)
              ORDER BY p.purge_prevue_le NULLS LAST, p.id LIMIT 500",
        )
        .bind(etab)
        .bind(jusqu_au)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(absence_id, piece_id, type_piece, etat, purge_prevue_le, gelee)| PurgeAVenirDto {
                absence_id,
                piece_id,
                type_piece,
                etat,
                purge_prevue_le,
                gelee,
            })
            .collect())
    }

    /// Purge une pièce à la demande (DPO, direction) : motif **codé**, refusée sous gel actif. Irréversible.
    pub async fn purger_piece_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        piece_id: Uuid,
        motif_code: &str,
    ) -> Result<PieceAbsenceDto, EntDbError> {
        self.exiger_conserver(etab, actor, role).await?;
        if !MOTIFS_PURGE.contains(&motif_code) {
            return Err(invalide("motif de purge inconnu"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let _a = charger(&mut tx, etab, absence_id, true).await?;
        let p = Self::piece_de_absence(&mut tx, etab, absence_id, piece_id, true).await?;
        if p.etat == "purgee" {
            return Err(conflit("pièce déjà purgée"));
        }
        let jour = jour_etab(&mut tx, etab).await?;
        if gel_actif(&mut tx, etab, absence_id, jour).await? {
            return Err(conflit("gel de conservation actif : la purge est suspendue"));
        }
        purger_piece_tx(&mut tx, etab, &p, "demande", actor).await?;
        journal(&mut tx, etab, absence_id, "purge_demandee", Some("motif"), None, Some(motif_code), actor).await?;
        let r = Self::piece_de_absence(&mut tx, etab, absence_id, piece_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(piece_dto(r))
    }

    // ───────────────────────────── ABP-160 : purge programmée ─────────────────────────────

    /// Tâche planifiée (une fois par heure, idempotente) : pour chaque établissement, calcule les dates de
    /// purge manquantes, purge les pièces échues (jamais sous gel) puis supprime le fil des absences dont
    /// toutes les pièces sont purgées. `jour` force la date (tests d'horloge) ; sinon le jour de chaque
    /// établissement.
    pub async fn purger_echeances(&self, jour: Option<NaiveDate>) -> Result<BilanPurgeDto, EntDbError> {
        let mut total = BilanPurgeDto::default();
        for etab in etablissements_systeme(
            self.pool(),
            ContexteSysteme::JobPlanifie("absences_personnels_purge"),
        )
        .await
        .map_err(db)?
        {
            let b = self.purger_echeances_etab(etab, jour).await?;
            total.dates_calculees += b.dates_calculees;
            total.pieces_purgees += b.pieces_purgees;
            total.messages_supprimes += b.messages_supprimes;
            total.pieces_gelees += b.pieces_gelees;
        }
        Ok(total)
    }

    pub async fn purger_echeances_etab(
        &self,
        etab: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<BilanPurgeDto, EntDbError> {
        let mut bilan = BilanPurgeDto::default();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = match jour {
            Some(j) => j,
            None => jour_etab(&mut tx, etab).await?,
        };
        // Acteur « système » du journal : le déposant de la pièce (aucun compte système n'existe).
        // 1. Dates manquantes (agent de l'État : connue seulement après la transmission).
        let sans_date: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, absence_id FROM ent_absp_piece
              WHERE etablissement_id = $1 AND etat = 'deposee' AND purge_prevue_le IS NULL",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (piece_id, absence_id) in sans_date {
            let a = charger(&mut tx, etab, absence_id, false).await?;
            if let Some(d) = date_purge_calculee(&mut tx, etab, &a).await? {
                sqlx::query("UPDATE ent_absp_piece SET purge_prevue_le = $3 WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(piece_id)
                    .bind(d)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                bilan.dates_calculees += 1;
            }
        }
        // 2. Pièces échues.
        let dues: Vec<PieceLigne> = sqlx::query_as(&format!(
            "SELECT {PIECE_COLONNES} FROM ent_absp_piece
              WHERE etablissement_id = $1 AND etat <> 'purgee' AND purge_prevue_le <= $2
              ORDER BY absence_id, id FOR UPDATE"
        ))
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut absences_touchees: Vec<(Uuid, Uuid)> = Vec::new();
        for p in dues {
            if gel_actif(&mut tx, etab, p.absence_id, jour).await? {
                bilan.pieces_gelees += 1;
                continue;
            }
            purger_piece_tx(&mut tx, etab, &p, "echeance", p.deposee_par_user_id).await?;
            bilan.pieces_purgees += 1;
            if !absences_touchees.iter().any(|(a, _)| *a == p.absence_id) {
                absences_touchees.push((p.absence_id, p.deposee_par_user_id));
            }
        }
        // 3. Fil : purgé avec les pièces (§18.4), quand l'absence n'a plus aucune pièce non purgée.
        for (absence_id, auteur) in absences_touchees {
            let (reste,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2 AND etat <> 'purgee')",
            )
            .bind(etab)
            .bind(absence_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if reste || gel_actif(&mut tx, etab, absence_id, jour).await? {
                continue;
            }
            sqlx::query("SELECT set_config('ent.absp_purge', 'on', true)")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            let n = sqlx::query("DELETE FROM ent_absp_message WHERE etablissement_id = $1 AND absence_id = $2")
                .bind(etab)
                .bind(absence_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?
                .rows_affected();
            sqlx::query("SELECT set_config('ent.absp_purge', 'off', true)")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            if n > 0 {
                journal(&mut tx, etab, absence_id, "fil_purge", Some("messages"), None, Some(&n.to_string()), auteur).await?;
                bilan.messages_supprimes += n as u32;
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(bilan)
    }

    // ───────────────────────────── ABP-164 : confidentialité renforcée ─────────────────────────────

    /// Marque (ou démarque) une absence « confidentialité renforcée » : la vue d'organisation est alors
    /// réduite à la direction et aux valideurs (la vie scolaire ne voit plus l'agent). L'agent en fait la
    /// demande par le fil ; la décision est celle de la direction (droit `conserver`).
    pub async fn definir_confidentialite_renforcee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        renforcee: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_conserver(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let _a = charger(&mut tx, etab, absence_id, true).await?;
        sqlx::query(
            "UPDATE ent_absp_absence SET confidentialite_renforcee = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(renforcee)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            absence_id,
            if renforcee { "confidentialite_renforcee" } else { "confidentialite_normale" },
            None,
            None,
            None,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
}

async fn relances_de(conn: &mut PgConnection, etab: Uuid, absence_id: Uuid) -> Result<u32, EntDbError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_absp_journal WHERE etablissement_id = $1 AND objet = 'absence'
            AND objet_id = $2 AND action = 'piece_relancee'",
    )
    .bind(etab)
    .bind(absence_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(n as u32)
}

#[cfg(test)]
mod tests_purs {
    use super::*;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    #[test]
    fn fin_d_annee_scolaire_par_defaut() {
        assert_eq!(fin_annee_scolaire_par_defaut(d(2026, 9, 1)), d(2027, 8, 31));
        assert_eq!(fin_annee_scolaire_par_defaut(d(2026, 11, 13)), d(2027, 8, 31));
        assert_eq!(fin_annee_scolaire_par_defaut(d(2027, 8, 31)), d(2027, 8, 31));
        assert_eq!(fin_annee_scolaire_par_defaut(d(2027, 1, 5)), d(2027, 8, 31));
    }

    #[test]
    fn ajouter_des_annees_gere_le_29_fevrier() {
        assert_eq!(ajouter_annees(d(2027, 8, 31), 3), d(2030, 8, 31));
        assert_eq!(ajouter_annees(d(2024, 2, 29), 1), d(2025, 2, 28));
    }

    #[test]
    fn purge_d_une_piece_selon_l_employeur() {
        // État : 30 jours après la transmission ; inconnue tant qu'elle n'a pas eu lieu.
        assert_eq!(date_purge_piece("etat", Some(d(2026, 11, 10)), d(2027, 8, 31), 30, 3), Some(d(2026, 12, 10)));
        assert_eq!(date_purge_piece("etat", None, d(2027, 8, 31), 30, 3), None);
        // OGEC : fin de l'année scolaire + 3 ans, indépendamment de toute transmission.
        assert_eq!(date_purge_piece("ogec", None, d(2027, 8, 31), 30, 3), Some(d(2030, 8, 31)));
    }
}
