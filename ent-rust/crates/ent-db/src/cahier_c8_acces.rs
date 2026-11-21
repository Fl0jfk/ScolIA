//! Socle 20, lot C8 (2/2) : **accès d'inspection, exports figés, clôture et archive annuelle** (`20_CAHIER_DE_TEXTES.md`
//! §14.2, §14.3, R-C18, R-C19, R-C28, CC-59, CC-60, CC-61, CC-63).
//!
//! * L'**inspecteur** voit exactement ce qu'on lui ouvre (cours, période du cahier, durée ≤ 90 jours) : entrées et consignes
//!   publiées, jamais de remise, de retour, de variante, de nom d'élève. Chaque lecture est journalisée **avant** d'être servie
//!   (si la trace ne s'écrit pas, rien n'est servi) ; le journal est lu de la direction et du professeur concerné.
//! * Un **export** est un texte figé, sans donnée d'élève, dont l'empreinte SHA-256 est recalculée par la base (CHECK).
//!   Il ne contient ni heure de production ni auteur : deux exports du même cahier ont la même empreinte.
//! * La **clôture** fige le cahier d'une année (plus aucune écriture), produit une archive par cours portant du contenu et
//!   journalise l'acte. Elle est irréversible sauf **réouverture motivée** de la direction, elle aussi journalisée ; une année
//!   close au socle 5 reste figée quoi qu'on fasse ici.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::types;
use ent_types::cahier_c8::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::cahier_c2::{conflit, invalide, texte_borne};
use crate::cahier_c8::{cours_de_agent, nom, periode_valide};
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

const MAX_COURS_PAR_ACCES: usize = 40;
const MAX_JOURNAL: i64 = 500;

// ───────────────────────────── contenu figé d'un cahier ─────────────────────────────

/// Ce qu'on exporte : les cours (contenu publié seulement) ou tout ce qu'un auteur a signé (brouillons compris).
pub(crate) enum Perimetre {
    Cours(Vec<Uuid>),
    Auteur(Uuid),
}

/// Texte déterministe d'un cahier sur une période : aucune heure de production, aucun auteur de l'export, aucun nom d'élève.
/// Retourne (texte, nombre d'entrées, nombre de travaux).
pub(crate) async fn contenu_cahier(
    tx: &mut PgConnection,
    etab: Uuid,
    titre: &str,
    du: NaiveDate,
    au: NaiveDate,
    perimetre: &Perimetre,
) -> Result<(String, i32, i32), EntDbError> {
    let (cours_ids, auteur, brouillons): (Vec<Uuid>, Option<Uuid>, bool) = match perimetre {
        Perimetre::Cours(c) => (c.clone(), None, false),
        Perimetre::Auteur(u) => (Vec::new(), Some(*u), true),
    };
    #[allow(clippy::type_complexity)]
    let entrees: Vec<(Uuid, String, String, NaiveDate, String, String, Option<String>, String, i32, String, Uuid)> = sqlx::query_as(
        "SELECT e.id, c.libelle, COALESCE(cl.code, g.code, ''), s.jour, to_char(s.heure_debut, 'HH24:MI') || '-' || to_char(s.heure_fin, 'HH24:MI'),
                e.etat, e.titre, e.contenu, e.version, ent_cdt_nom_affichage(e.etablissement_id, e.auteur_user_id), c.id
           FROM ent_cdt_entree e
           JOIN ent_cours c ON c.etablissement_id = e.etablissement_id AND c.id = e.cours_id
           JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
           LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
           LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
          WHERE e.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
            AND (($4::uuid IS NULL AND e.cours_id = ANY($5) AND e.etat = 'publiee')
              OR ($4::uuid IS NOT NULL AND e.auteur_user_id = $4 AND e.etat <> 'retiree'))
          ORDER BY c.libelle, c.id, s.jour, s.heure_debut, e.id",
    )
    .bind(etab)
    .bind(du)
    .bind(au)
    .bind(auteur)
    .bind(&cours_ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    #[allow(clippy::type_complexity)]
    let travaux: Vec<(Uuid, String, String, NaiveDate, NaiveDate, String, String, String, Option<i16>, i32, String, String, Uuid)> = sqlx::query_as(
        "SELECT t.id, c.libelle, COALESCE(cl.code, g.code, ''), t.donne_le,
                (t.echeance_le AT TIME ZONE COALESCE((SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = t.etablissement_id), 'Europe/Paris'))::date,
                n.libelle, t.titre, t.consigne, t.duree_estimee_min, t.version, t.etat,
                ent_cdt_nom_affichage(t.etablissement_id, t.auteur_user_id), c.id
           FROM ent_cdt_travail t
           JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id
           JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
           LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
           LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
          WHERE t.etablissement_id = $1 AND t.donne_le BETWEEN $2 AND $3
            AND (($4::uuid IS NULL AND t.cours_id = ANY($5) AND t.etat = 'publie')
              OR ($4::uuid IS NOT NULL AND t.auteur_user_id = $4 AND t.etat <> 'retire'))
          ORDER BY c.libelle, c.id, t.donne_le, t.titre, t.id",
    )
    .bind(etab)
    .bind(du)
    .bind(au)
    .bind(auteur)
    .bind(&cours_ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let mut out = format!("CAHIER DE TEXTES — {titre}\nPériode : du {du} au {au}\n");
    out.push_str(if brouillons {
        "Export professionnel : vos entrées et travaux (brouillons compris, signalés). Aucune donnée d'élève.\n"
    } else {
        "Contenu publié uniquement. Aucune donnée d'élève (ni remise, ni retour, ni variante).\n"
    });
    let mut ordre: Vec<(Uuid, String, String)> = Vec::new();
    for e in &entrees {
        if !ordre.iter().any(|(c, _, _)| *c == e.10) {
            ordre.push((e.10, e.1.clone(), e.2.clone()));
        }
    }
    for t in &travaux {
        if !ordre.iter().any(|(c, _, _)| *c == t.12) {
            ordre.push((t.12, t.1.clone(), t.2.clone()));
        }
    }
    ordre.sort_by(|a, b| (&a.1, a.0).cmp(&(&b.1, b.0)));
    for (cours, libelle, public) in &ordre {
        out.push_str(&format!("\n== {libelle} ({public}) ==\n"));
        for e in entrees.iter().filter(|e| e.10 == *cours) {
            let marque = if e.5 == "publiee" {
                String::new()
            } else {
                format!(" [{}]", e.5)
            };
            out.push_str(&format!(
                "-- {} {} · Séance · {} (v{}, {}){}\n{}\n",
                e.3,
                e.4,
                e.6.as_deref().unwrap_or("(sans titre)"),
                e.8,
                e.9,
                marque,
                e.7
            ));
            let notions: Vec<(String,)> =
                sqlx::query_as("SELECT libelle FROM ent_cdt_entree_notion WHERE etablissement_id = $1 AND entree_id = $2 ORDER BY ordre")
                    .bind(etab)
                    .bind(e.0)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
            if !notions.is_empty() {
                out.push_str(&format!(
                    "Notions : {}\n",
                    notions
                        .into_iter()
                        .map(|(n,)| n)
                        .collect::<Vec<_>>()
                        .join(" ; ")
                ));
            }
        }
        for t in travaux.iter().filter(|t| t.12 == *cours) {
            let marque = if t.10 == "publie" {
                String::new()
            } else {
                format!(" [{}]", t.10)
            };
            let duree = t.8.map(|d| format!(" · {d} min")).unwrap_or_default();
            out.push_str(&format!(
                "-- Travail donné le {} pour le {} · {} · {}{} (v{}, {}){}\n{}\n",
                t.3, t.4, t.5, t.6, duree, t.9, t.11, marque, t.7
            ));
        }
    }
    Ok((out, entrees.len() as i32, travaux.len() as i32))
}

async fn inserer_export(
    tx: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    cours: Option<Uuid>,
    agent: Option<Uuid>,
    annee: Option<Uuid>,
    (du, au): (NaiveDate, NaiveDate),
    titre: &str,
    motif: Option<&str>,
    par: Uuid,
    (contenu, ne, nt): (String, i32, i32),
) -> Result<(Uuid, String), EntDbError> {
    let id = Uuid::new_v4();
    let empreinte: String =
        sqlx::query_scalar("SELECT encode(sha256(convert_to($1, 'UTF8')), 'hex')")
            .bind(&contenu)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_cdt_export (id, etablissement_id, nature, cours_id, agent_id, annee_id, du, au, titre, contenu, empreinte,
                nb_entrees, nb_travaux, motif, produit_par_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(id)
    .bind(etab)
    .bind(nature)
    .bind(cours)
    .bind(agent)
    .bind(annee)
    .bind(du)
    .bind(au)
    .bind(titre)
    .bind(&contenu)
    .bind(&empreinte)
    .bind(ne)
    .bind(nt)
    .bind(motif)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok((id, empreinte))
}

async fn export_dto(tx: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<ExportDto, EntDbError> {
    #[allow(clippy::type_complexity)]
    let r: (String, String, NaiveDate, NaiveDate, i32, i32, String, DateTime<Utc>, Uuid) = sqlx::query_as(
        "SELECT nature, titre, du, au, nb_entrees, nb_travaux, empreinte, cree_le, produit_par_user_id
           FROM ent_cdt_export WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let produit_par = nom(tx, etab, Some(r.8)).await?;
    Ok(ExportDto {
        id,
        nature: r.0,
        titre: r.1,
        du: r.2,
        au: r.3,
        nb_entrees: r.4,
        nb_travaux: r.5,
        empreinte: r.6,
        produit_le: r.7,
        produit_par,
    })
}

impl PgEnt {
    // ───────────────────────────── CDT-217 : exports figés ─────────────────────────────

    /// CDT-217, CC-61 : export figé du cahier d'un cours ou d'un professeur sur une période (`cahier.exporter`). Sans donnée
    /// d'élève : valable pour un litige comme pour une inspection.
    pub async fn cdt_c8_export_produire(
        &self,
        a: &Acteur<'_>,
        b: ExportBody,
    ) -> Result<ExportDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        periode_valide(b.du, b.au)?;
        if ![EXPORT_LITIGE, EXPORT_INSPECTION, EXPORT_AUTRE].contains(&b.nature.as_str()) {
            return Err(invalide("nature de l'export : litige, inspection ou autre"));
        }
        let motif = match b.motif.as_deref() {
            Some(m) if !m.trim().is_empty() => Some(texte_borne(
                m,
                500,
                true,
                "motif trop long (500 caractères au plus)",
            )?),
            _ => None,
        };
        if b.nature == EXPORT_LITIGE && motif.is_none() {
            return Err(invalide("un export pour litige exige un motif"));
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (cours_ids, titre, cours, agent) = match (b.cours_id, b.agent_id) {
            (Some(c), None) => {
                let r = self.c8_cours_ref(etab, c).await?.ok_or_else(introuvable)?;
                (
                    vec![c],
                    format!("{} ({})", r.libelle, r.public),
                    Some(c),
                    None,
                )
            }
            (None, Some(ag)) => {
                let u: Option<(Option<Uuid>,)> = sqlx::query_as(
                    "SELECT pe.user_id FROM ent_agent g JOIN ent_personne pe ON pe.etablissement_id = g.etablissement_id AND pe.id = g.personne_id
                      WHERE g.etablissement_id = $1 AND g.id = $2",
                )
                .bind(etab)
                .bind(ag)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let Some((Some(user),)) = u else {
                    return Err(introuvable());
                };
                let l = cours_de_agent(&mut tx, etab, ag, user, b.du, b.au).await?;
                let n = nom(&mut tx, etab, Some(user)).await?;
                (l, format!("professeur {n}"), None, Some(ag))
            }
            _ => return Err(invalide("indiquez un cours ou un professeur, pas les deux")),
        };
        let contenu = contenu_cahier(
            &mut tx,
            etab,
            &titre,
            b.du,
            b.au,
            &Perimetre::Cours(cours_ids),
        )
        .await?;
        let (id, empreinte) = inserer_export(
            &mut tx,
            etab,
            &b.nature,
            cours,
            agent,
            None,
            (b.du, b.au),
            &format!("Cahier — {titre}"),
            motif.as_deref(),
            a.user_id,
            contenu,
        )
        .await?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_EXPORT_PRODUIT,
            id,
            None,
            json!({ "nature": b.nature, "empreinte": empreinte }),
        )
        .await?;
        let dto = export_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Exports produits (hors exports personnels de professeurs), récents d'abord.
    pub async fn cdt_c8_exports_lister(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<ExportDto>, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_cdt_export WHERE etablissement_id = $1 AND nature NOT IN ('professeur', 'archive') ORDER BY cree_le DESC, id LIMIT 200",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(export_dto(&mut tx, etab, id).await?);
        }
        Ok(out)
    }

    /// Lecture d'un export : direction (`cahier.exporter`) ; un export personnel de professeur n'est lu que de son auteur.
    pub async fn cdt_c8_export_lire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<ExportContenuDto, EntDbError> {
        let direction = self.cdt_droit_etab(a, Action::CahierExporter).await?;
        if !direction && !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(String, Uuid, String)> =
            sqlx::query_as("SELECT nature, produit_par_user_id, contenu FROM ent_cdt_export WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let Some((nature, par, contenu)) = r else {
            return Err(introuvable());
        };
        let autorise = if nature == "professeur" {
            par == a.user_id
        } else {
            direction
        };
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        Ok(ExportContenuDto {
            export: export_dto(&mut tx, etab, id).await?,
            contenu,
        })
    }

    /// CDT-223 : le professeur exporte **son** cahier de l'année (entrées et travaux qu'il a signés, sans élève).
    pub async fn cdt_c8_export_professeur(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<ExportContenuDto, EntDbError> {
        if !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let agent = self.c8_agent_du_compte(a).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let an: Option<(String, NaiveDate, NaiveDate)> =
            sqlx::query_as("SELECT libelle, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(annee_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let Some((libelle, du, au)) = an else {
            return Err(introuvable());
        };
        let n = nom(&mut tx, etab, Some(a.user_id)).await?;
        let titre = format!("{n}, année {libelle}");
        let contenu =
            contenu_cahier(&mut tx, etab, &titre, du, au, &Perimetre::Auteur(a.user_id)).await?;
        let (id, _) = inserer_export(
            &mut tx,
            etab,
            "professeur",
            None,
            agent,
            Some(annee_id),
            (du, au),
            &format!("Mon cahier — {libelle}"),
            None,
            a.user_id,
            contenu.clone(),
        )
        .await?;
        let dto = export_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(ExportContenuDto {
            export: dto,
            contenu: contenu.0,
        })
    }

    // ───────────────────────────── CDT-215 : accès d'inspection ─────────────────────────────

    /// CDT-215 : accorde un accès d'inspection (`cahier.acces_inspection`) : un compte invité, des cours, une période du cahier,
    /// une durée (30 jours par défaut, 90 au plus). Le professeur concerné est prévenu (événement de l'outbox).
    pub async fn cdt_c8_acces_accorder(
        &self,
        a: &Acteur<'_>,
        b: AccesInspectionBody,
    ) -> Result<AccesInspectionDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierAccesInspection)
            .await?;
        periode_valide(b.du_cahier, b.au_cahier)?;
        if !["rdv_carriere", "inspection", "autre"].contains(&b.motif.as_str()) {
            return Err(invalide(
                "motif de l'accès : rdv_carriere, inspection ou autre",
            ));
        }
        let duree = b.duree_jours.unwrap_or(ACCES_DUREE_DEFAUT_JOURS);
        if !(1..=ACCES_DUREE_MAX_JOURS).contains(&duree) {
            return Err(invalide("durée de l'accès : 1 à 90 jours"));
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Le bénéficiaire est un compte de l'établissement, jamais un élève ni un responsable.
        let m: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(b.beneficiaire_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((role,)) = m else {
            return Err(introuvable());
        };
        if matches!(
            ent_auth::Role::parse(&role),
            Some(ent_auth::Role::Eleve | ent_auth::Role::Parent)
        ) {
            return Err(invalide(
                "un élève ou un responsable ne peut pas recevoir un accès d'inspection",
            ));
        }
        let (agent_cible, mut cours_ids) = match b.agent_id {
            Some(ag) => {
                let u: Option<(Option<Uuid>,)> = sqlx::query_as(
                    "SELECT pe.user_id FROM ent_agent g JOIN ent_personne pe ON pe.etablissement_id = g.etablissement_id AND pe.id = g.personne_id
                      WHERE g.etablissement_id = $1 AND g.id = $2",
                )
                .bind(etab)
                .bind(ag)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let Some((Some(user),)) = u else {
                    return Err(introuvable());
                };
                (
                    Some(ag),
                    cours_de_agent(&mut tx, etab, ag, user, b.du_cahier, b.au_cahier).await?,
                )
            }
            None => (None, Vec::new()),
        };
        for c in &b.cours_ids {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(c)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
            if !cours_ids.contains(c) {
                cours_ids.push(*c);
            }
        }
        if cours_ids.is_empty() {
            return Err(conflit("aucun cours à ouvrir : indiquez des cours ou un professeur qui en a sur la période"));
        }
        if cours_ids.len() > MAX_COURS_PAR_ACCES {
            return Err(invalide("quarante cours au plus par accès"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_cdt_acces_inspection (id, etablissement_id, beneficiaire_user_id, agent_cible_id, du_cahier, au_cahier,
                    valable_du, valable_au, motif, accorde_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, now(), now() + make_interval(days => $7), $8, $9)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.beneficiaire_user_id)
        .bind(agent_cible)
        .bind(b.du_cahier)
        .bind(b.au_cahier)
        .bind(duree)
        .bind(&b.motif)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for c in &cours_ids {
            sqlx::query("INSERT INTO ent_cdt_acces_inspection_cours (etablissement_id, acces_id, cours_id) VALUES ($1, $2, $3)")
                .bind(etab)
                .bind(id)
                .bind(c)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        historiser(&mut tx, etab, id, "accorde", a.user_id).await?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_ACCES_INSPECTION_ACCORDE,
            id,
            None,
            json!({ "acces_id": id, "cours": cours_ids.len() }),
        )
        .await?;
        let dto = acces_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-215 : prolonge un accès non révoqué (la durée totale ne dépasse jamais 90 jours : au-delà, un nouvel accès).
    pub async fn cdt_c8_acces_prolonger(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        b: ProlongerBody,
    ) -> Result<AccesInspectionDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierAccesInspection)
            .await?;
        if !(1..=ACCES_DUREE_MAX_JOURS).contains(&b.jours) {
            return Err(invalide("prolongation : 1 à 90 jours"));
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(DateTime<Utc>, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT valable_du, valable_au, revoque_le FROM ent_cdt_acces_inspection WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((du, au, revoque)) = r else {
            return Err(introuvable());
        };
        if revoque.is_some() {
            return Err(conflit("accès révoqué : accordez-en un nouveau"));
        }
        let nouvelle = au.max(Utc::now()) + Duration::days(b.jours as i64);
        if nouvelle - du > Duration::days(ACCES_DUREE_MAX_JOURS as i64) {
            return Err(conflit(
                "durée maximale de 90 jours atteinte : accordez un nouvel accès",
            ));
        }
        sqlx::query("UPDATE ent_cdt_acces_inspection SET valable_au = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(nouvelle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        historiser(&mut tx, etab, id, "prolonge", a.user_id).await?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_ACCES_INSPECTION_PROLONGE,
            id,
            None,
            json!({ "acces_id": id }),
        )
        .await?;
        let dto = acces_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-215 : révoque un accès (effet immédiat, trace conservée).
    pub async fn cdt_c8_acces_revoquer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<AccesInspectionDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierAccesInspection)
            .await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(Option<DateTime<Utc>>,)> =
            sqlx::query_as("SELECT revoque_le FROM ent_cdt_acces_inspection WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                .bind(etab)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let Some((revoque,)) = r else {
            return Err(introuvable());
        };
        if revoque.is_some() {
            return Err(conflit("accès déjà révoqué"));
        }
        sqlx::query("UPDATE ent_cdt_acces_inspection SET revoque_le = now(), revoque_par_user_id = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        historiser(&mut tx, etab, id, "revoque", a.user_id).await?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_ACCES_INSPECTION_REVOQUE,
            id,
            None,
            json!({ "acces_id": id }),
        )
        .await?;
        let dto = acces_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Accès accordés : la direction les voit tous ; un professeur ne voit que ceux qui le concernent (visés par son nom, ou sur
    /// un de ses cours) ; personne d'autre.
    pub async fn cdt_c8_acces_lister(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<AccesInspectionDto>, EntDbError> {
        let direction = self
            .cdt_droit_etab(a, Action::CahierAccesInspection)
            .await?;
        if !direction && !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let agent = self.c8_agent_du_compte(a).await?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT x.id FROM ent_cdt_acces_inspection x
              WHERE x.etablissement_id = $1
                AND ($2 OR x.agent_cible_id = $3
                     OR EXISTS (SELECT 1 FROM ent_cdt_acces_inspection_cours ac
                                 WHERE ac.etablissement_id = x.etablissement_id AND ac.acces_id = x.id
                                   AND ent_prof_sur_cours($1, ac.cours_id, $4, $5, false)))
              ORDER BY x.cree_le DESC, x.id LIMIT 200",
        )
        .bind(etab)
        .bind(direction)
        .bind(agent)
        .bind(a.user_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(acces_dto(&mut tx, etab, id).await?);
        }
        Ok(out)
    }

    /// Accès dont le compte est bénéficiaire (l'inspecteur) : aucun droit du moteur n'est nécessaire, l'octroi fait foi.
    pub async fn cdt_c8_mes_acces(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<AccesInspectionDto>, EntDbError> {
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_cdt_acces_inspection WHERE etablissement_id = $1 AND beneficiaire_user_id = $2 ORDER BY cree_le DESC, id LIMIT 50",
        )
        .bind(etab)
        .bind(a.user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(acces_dto(&mut tx, etab, id).await?);
        }
        Ok(out)
    }

    /// CDT-216 : l'inspecteur lit un cours de son accès. Chaque lecture est journalisée avant d'être servie ; l'accès expiré,
    /// révoqué, à venir, un cours hors accès ou l'accès d'un autre : 403 (CC-59).
    pub async fn cdt_c8_inspection_lire(
        &self,
        a: &Acteur<'_>,
        acces_id: Uuid,
        cours_id: Uuid,
    ) -> Result<CahierInspectionDto, EntDbError> {
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // `FOR SHARE` : une révocation concurrente attend la fin de la lecture, ou la précède.
        let r: Option<(NaiveDate, NaiveDate, DateTime<Utc>)> = sqlx::query_as(
            "SELECT du_cahier, au_cahier, valable_au FROM ent_cdt_acces_inspection
              WHERE etablissement_id = $1 AND id = $2 AND beneficiaire_user_id = $3
                AND revoque_le IS NULL AND valable_du <= now() AND valable_au > now() FOR SHARE",
        )
        .bind(etab)
        .bind(acces_id)
        .bind(a.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((du, au, valable_au)) = r else {
            return Err(EntDbError::Forbidden);
        };
        let (ouvert,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_acces_inspection_cours WHERE etablissement_id = $1 AND acces_id = $2 AND cours_id = $3)",
        )
        .bind(etab)
        .bind(acces_id)
        .bind(cours_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !ouvert {
            return Err(EntDbError::Forbidden);
        }
        // Journal d'abord : sans trace, rien n'est servi.
        sqlx::query(
            "INSERT INTO ent_cdt_consultation (id, etablissement_id, qui_user_id, nature, objet_id, acces_id, cours_id)
             VALUES ($1, $2, $3, 'inspection', $4, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(a.user_id)
        .bind(acces_id)
        .bind(cours_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let cours = {
            let r: (Uuid, String, String) = sqlx::query_as(
                "SELECT c.id, c.libelle, COALESCE(cl.code, g.code, '') FROM ent_cours c
                   LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
                   LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
                  WHERE c.etablissement_id = $1 AND c.id = $2",
            )
            .bind(etab)
            .bind(cours_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            CoursRefDto {
                cours_id: r.0,
                libelle: r.1,
                public: r.2,
            }
        };
        #[allow(clippy::type_complexity)]
        let entrees_l: Vec<(Uuid, NaiveDate, chrono::NaiveTime, chrono::NaiveTime, Option<String>, String, i32, String)> = sqlx::query_as(
            "SELECT e.id, s.jour, s.heure_debut, s.heure_fin, e.titre, e.contenu, e.version, ent_cdt_nom_affichage(e.etablissement_id, e.auteur_user_id)
               FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat = 'publiee' AND s.jour BETWEEN $3 AND $4
              ORDER BY s.jour, s.heure_debut, e.id",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut entrees = Vec::new();
        for (id, jour, hd, hf, titre, contenu, version, auteur) in entrees_l {
            let notions: Vec<(String,)> =
                sqlx::query_as("SELECT libelle FROM ent_cdt_entree_notion WHERE etablissement_id = $1 AND entree_id = $2 ORDER BY ordre")
                    .bind(etab)
                    .bind(id)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
            entrees.push(EntreeInspectionDto {
                jour,
                heure_debut: hd,
                heure_fin: hf,
                titre,
                contenu,
                notions: notions.into_iter().map(|(n,)| n).collect(),
                version,
                auteur,
            });
        }
        #[allow(clippy::type_complexity)]
        let travaux_l: Vec<(NaiveDate, NaiveDate, String, String, String, Option<i16>, i32, String)> = sqlx::query_as(
            "SELECT t.donne_le,
                    (t.echeance_le AT TIME ZONE COALESCE((SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = t.etablissement_id), 'Europe/Paris'))::date,
                    t.titre, t.consigne, n.libelle, t.duree_estimee_min, t.version, ent_cdt_nom_affichage(t.etablissement_id, t.auteur_user_id)
               FROM ent_cdt_travail t JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
              WHERE t.etablissement_id = $1 AND t.cours_id = $2 AND t.etat = 'publie' AND t.donne_le BETWEEN $3 AND $4
              ORDER BY t.donne_le, t.titre, t.id",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let travaux = travaux_l
            .into_iter()
            .map(
                |(donne_le, echeance_jour, titre, consigne, nature, duree, version, auteur)| {
                    TravailInspectionDto {
                        donne_le,
                        echeance_jour,
                        titre,
                        consigne,
                        nature,
                        duree_estimee_min: duree,
                        version,
                        auteur,
                    }
                },
            )
            .collect();
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_CONSULTE_INSPECTION,
            acces_id,
            None,
            json!({ "acces_id": acces_id, "cours_id": cours_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(CahierInspectionDto {
            acces_id,
            cours,
            du,
            au,
            valable_au,
            entrees,
            travaux,
        })
    }

    /// Journal des consultations d'un accès : la direction et le professeur concerné (visé, ou intervenant d'un cours ouvert).
    pub async fn cdt_c8_acces_journal(
        &self,
        a: &Acteur<'_>,
        acces_id: Uuid,
    ) -> Result<Vec<ConsultationDto>, EntDbError> {
        let direction = self
            .cdt_droit_etab(a, Action::CahierAccesInspection)
            .await?;
        if !direction && !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let agent = self.c8_agent_du_compte(a).await?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (existe,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_cdt_acces_inspection WHERE etablissement_id = $1 AND id = $2)")
            .bind(etab)
            .bind(acces_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let (concerne,): (bool,) = sqlx::query_as(
            "SELECT $3 OR EXISTS (SELECT 1 FROM ent_cdt_acces_inspection x WHERE x.etablissement_id = $1 AND x.id = $2 AND x.agent_cible_id = $4)
                OR EXISTS (SELECT 1 FROM ent_cdt_acces_inspection_cours ac
                            WHERE ac.etablissement_id = $1 AND ac.acces_id = $2 AND ent_prof_sur_cours($1, ac.cours_id, $5, $6, false))",
        )
        .bind(etab)
        .bind(acces_id)
        .bind(direction)
        .bind(agent)
        .bind(a.user_id)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !concerne {
            return Err(EntDbError::Forbidden);
        }
        let l: Vec<(DateTime<Utc>, Uuid, Option<String>)> = sqlx::query_as(
            "SELECT k.cree_le, k.qui_user_id, c.libelle FROM ent_cdt_consultation k
               LEFT JOIN ent_cours c ON c.etablissement_id = k.etablissement_id AND c.id = k.cours_id
              WHERE k.etablissement_id = $1 AND k.acces_id = $2 AND k.nature = 'inspection'
              ORDER BY k.cree_le DESC, k.id LIMIT $3",
        )
        .bind(etab)
        .bind(acces_id)
        .bind(MAX_JOURNAL)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (le, qui, cours) in l {
            out.push(ConsultationDto {
                le,
                qui: nom(&mut tx, etab, Some(qui)).await?,
                cours,
            });
        }
        Ok(out)
    }

    // ───────────────────────────── CDT-222 : clôture et archive ─────────────────────────────

    /// Contrôle de clôture (port `ControleCloture` du socle 5) : cours non visés, brouillons jamais publiés, non-rendus.
    pub async fn cdt_c8_cloture_controle(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<ControleClotureDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        controle(&mut tx, etab, annee_id).await
    }

    /// CDT-222, CC-60 : clôture le cahier de l'année (plus aucune écriture), une archive figée par cours portant du contenu.
    /// Possible malgré les points du contrôle seulement avec `confirmer` ; irréversible sauf réouverture motivée.
    pub async fn cdt_c8_cloturer(
        &self,
        a: &Acteur<'_>,
        b: ClotureBody,
    ) -> Result<ClotureDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Verrou de l'année : deux clôtures simultanées se suivent.
        let an: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(b.annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin)) = an else {
            return Err(introuvable());
        };
        let c = controle(&mut tx, etab, b.annee_id).await?;
        if c.deja_fige {
            return Err(conflit("le cahier de cette année est déjà figé"));
        }
        if !b.confirmer
            && (!c.cours_non_vises.is_empty()
                || c.entrees_brouillon > 0
                || c.travaux_brouillon > 0
                || c.non_rendus > 0)
        {
            return Err(conflit("le contrôle de clôture signale des points à régler (cours non visés, brouillons, non-rendus) : confirmez pour clôturer malgré tout"));
        }
        let cours: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT c.id FROM ent_cours c
              WHERE c.etablissement_id = $1 AND c.annee_id = $2
                AND (EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.etablissement_id = c.etablissement_id AND e.cours_id = c.id AND e.etat = 'publiee')
                  OR EXISTS (SELECT 1 FROM ent_cdt_travail t WHERE t.etablissement_id = c.etablissement_id AND t.cours_id = c.id AND t.etat = 'publie'))
              ORDER BY c.libelle, c.id",
        )
        .bind(etab)
        .bind(b.annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut n = 0i64;
        for (cours_id,) in cours {
            let r: (String, String) = {
                let x = sqlx::query_as::<_, (String, String)>(
                    "SELECT c.libelle, COALESCE(cl.code, g.code, '') FROM ent_cours c
                       LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
                       LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
                      WHERE c.etablissement_id = $1 AND c.id = $2",
                )
                .bind(etab)
                .bind(cours_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                x
            };
            let titre = format!("{} ({}) — archive {}", r.0, r.1, c.annee_libelle);
            let contenu = contenu_cahier(
                &mut tx,
                etab,
                &titre,
                debut,
                fin,
                &Perimetre::Cours(vec![cours_id]),
            )
            .await?;
            let (export_id, empreinte) = inserer_export(
                &mut tx,
                etab,
                "archive",
                Some(cours_id),
                None,
                Some(b.annee_id),
                (debut, fin),
                &format!("Archive — {titre}"),
                None,
                a.user_id,
                contenu,
            )
            .await?;
            // Conservation : lecture jusqu'à la fin de l'année suivante, puis cinq ans (R-06, R-C28, à valider par le DPO).
            sqlx::query(
                "INSERT INTO ent_cdt_archive (id, etablissement_id, annee_id, cours_id, export_id, empreinte, produite_le, version, echeance_archive)
                 VALUES ($1, $2, $3, $4, $5, $6, now(),
                         COALESCE((SELECT max(version) FROM ent_cdt_archive WHERE etablissement_id = $2 AND annee_id = $3 AND cours_id = $4), 0) + 1,
                         ($7::date + interval '6 years')::date)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(b.annee_id)
            .bind(cours_id)
            .bind(export_id)
            .bind(&empreinte)
            .bind(fin)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            n += 1;
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_cdt_cloture (id, etablissement_id, annee_id, action, nb_archives, par_user_id) VALUES ($1, $2, $3, 'cloture', $4, $5)")
            .bind(id)
            .bind(etab)
            .bind(b.annee_id)
            .bind(n as i32)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_ANNEE_FIGEE,
            id,
            None,
            json!({ "annee_id": b.annee_id, "archives": n }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ClotureDto {
            annee_id: b.annee_id,
            archives_produites: n,
            fige: true,
        })
    }

    /// CC-63 : réouverture motivée du cahier d'une année (procédure tracée de la direction). Impossible tant que l'année est
    /// close au socle 5 : c'est cette année qu'il faut rouvrir d'abord.
    pub async fn cdt_c8_rouvrir(
        &self,
        a: &Acteur<'_>,
        b: ReouvrirBody,
    ) -> Result<ClotureDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        let motif = texte_borne(
            &b.motif,
            500,
            true,
            "motif obligatoire (500 caractères au plus)",
        )?;
        if motif.chars().count() < 10 {
            return Err(invalide("motif trop court : 10 caractères au moins"));
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let an: Option<(bool,)> = sqlx::query_as(
            "SELECT cloturee_le IS NOT NULL FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(b.annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((close5,)) = an else {
            return Err(introuvable());
        };
        let (fige,): (bool,) = sqlx::query_as("SELECT ent_cdt_cahier_fige($1, $2)")
            .bind(etab)
            .bind(b.annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !fige {
            return Err(conflit("le cahier de cette année n'est pas clôturé"));
        }
        if close5 {
            return Err(conflit("l'année scolaire est close : rouvrez d'abord l'année (structure) ; le cahier reste figé"));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_cdt_cloture (id, etablissement_id, annee_id, action, motif, par_user_id) VALUES ($1, $2, $3, 'reouverture', $4, $5)")
            .bind(id)
            .bind(etab)
            .bind(b.annee_id)
            .bind(&motif)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_ANNEE_ROUVERTE,
            id,
            None,
            json!({ "annee_id": b.annee_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ClotureDto {
            annee_id: b.annee_id,
            archives_produites: 0,
            fige: false,
        })
    }

    /// Archives d'une année (ou toutes) : `cahier.exporter`.
    pub async fn cdt_c8_archives(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
    ) -> Result<Vec<ArchiveDto>, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        #[allow(clippy::type_complexity)]
        let l: Vec<(Uuid, Uuid, Uuid, String, String, DateTime<Utc>, String, i32, NaiveDate, String, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT r.id, r.annee_id, c.id, c.libelle, COALESCE(cl.code, g.code, ''), r.produite_le, r.empreinte, r.version,
                    r.echeance_archive, r.decision, r.decide_le
               FROM ent_cdt_archive r
               JOIN ent_cours c ON c.etablissement_id = r.etablissement_id AND c.id = r.cours_id
               LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
               LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
              WHERE r.etablissement_id = $1 AND ($2::uuid IS NULL OR r.annee_id = $2)
              ORDER BY r.produite_le DESC, c.libelle, r.version DESC LIMIT 500",
        )
        .bind(etab)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(l.into_iter()
            .map(
                |(
                    id,
                    annee_id,
                    cid,
                    libelle,
                    public,
                    produite_le,
                    empreinte,
                    version,
                    echeance_archive,
                    decision,
                    decide_le,
                )| ArchiveDto {
                    id,
                    annee_id,
                    cours: CoursRefDto {
                        cours_id: cid,
                        libelle,
                        public,
                    },
                    produite_le,
                    empreinte,
                    version,
                    echeance_archive,
                    decision,
                    decide_le,
                },
            )
            .collect())
    }

    /// Texte figé d'une archive (direction, `cahier.exporter`).
    pub async fn cdt_c8_archive_contenu(
        &self,
        a: &Acteur<'_>,
        archive_id: Uuid,
    ) -> Result<ExportContenuDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        let etab = a.etablissement_id;
        let e: Option<(Option<Uuid>,)> = sqlx::query_as(
            "SELECT export_id FROM ent_cdt_archive WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(archive_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let Some((Some(export_id),)) = e else {
            return Err(introuvable());
        };
        self.cdt_c8_export_lire(a, export_id).await
    }

    /// CDT-245 : décision du chef d'établissement sur une archive arrivée à échéance (versée aux archives de l'établissement
    /// ou à détruire). La décision est tracée et définitive ; la purge effective relève du socle 27.
    pub async fn cdt_c8_archive_decider(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        b: ArchiveDecisionBody,
    ) -> Result<ArchiveDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierExporter).await?;
        if !["versee", "detruite"].contains(&b.decision.as_str()) {
            return Err(invalide("décision : versee ou detruite"));
        }
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(String, NaiveDate, Uuid)> =
            sqlx::query_as("SELECT decision, echeance_archive, annee_id FROM ent_cdt_archive WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                .bind(etab)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let Some((decision, echeance, annee)) = r else {
            return Err(introuvable());
        };
        if decision != "a_decider" {
            return Err(conflit("décision déjà prise"));
        }
        if echeance > jour {
            return Err(conflit(
                "l'archive est conservée jusqu'à son échéance : aucune décision avant",
            ));
        }
        sqlx::query("UPDATE ent_cdt_archive SET decision = $3, decide_par_user_id = $4, decide_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(&b.decision)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_ARCHIVE_DECIDEE,
            id,
            None,
            json!({ "decision": b.decision }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.cdt_c8_archives(a, Some(annee))
            .await?
            .into_iter()
            .find(|x| x.id == id)
            .ok_or_else(introuvable)
    }
}

async fn historiser(
    tx: &mut PgConnection,
    etab: Uuid,
    acces: Uuid,
    action: &str,
    par: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_cdt_acces_inspection_historique (id, etablissement_id, acces_id, action, valable_au, par_user_id)
         VALUES ($1, $2, $3, $4, (SELECT valable_au FROM ent_cdt_acces_inspection WHERE etablissement_id = $2 AND id = $3), $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(acces)
    .bind(action)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

async fn acces_dto(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<AccesInspectionDto, EntDbError> {
    #[allow(clippy::type_complexity)]
    let r: (Uuid, Option<Uuid>, NaiveDate, NaiveDate, DateTime<Utc>, DateTime<Utc>, String, Uuid, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT beneficiaire_user_id, agent_cible_id, du_cahier, au_cahier, valable_du, valable_au, motif, accorde_par_user_id, revoque_le
           FROM ent_cdt_acces_inspection WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let (benef, agent, du_c, au_c, v_du, v_au, motif, par, revoque) = r;
    let agent_cible = match agent {
        Some(ag) => {
            let u: Option<(Option<Uuid>,)> = sqlx::query_as(
                "SELECT pe.user_id FROM ent_agent g JOIN ent_personne pe ON pe.etablissement_id = g.etablissement_id AND pe.id = g.personne_id
                  WHERE g.etablissement_id = $1 AND g.id = $2",
            )
            .bind(etab)
            .bind(ag)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match u {
                Some((Some(user),)) => Some(nom(tx, etab, Some(user)).await?),
                _ => Some("Professeur".to_string()),
            }
        }
        None => None,
    };
    let cours: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT c.id, c.libelle, COALESCE(cl.code, g.code, '')
           FROM ent_cdt_acces_inspection_cours ac
           JOIN ent_cours c ON c.etablissement_id = ac.etablissement_id AND c.id = ac.cours_id
           LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
           LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
          WHERE ac.etablissement_id = $1 AND ac.acces_id = $2 ORDER BY c.libelle, c.id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let (nb,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cdt_consultation WHERE etablissement_id = $1 AND acces_id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let maintenant = Utc::now();
    let etat = if revoque.is_some() {
        ACCES_REVOQUE
    } else if v_au <= maintenant {
        ACCES_EXPIRE
    } else if v_du > maintenant {
        ACCES_A_VENIR
    } else {
        ACCES_ACTIF
    };
    Ok(AccesInspectionDto {
        id,
        beneficiaire: nom(tx, etab, Some(benef)).await?,
        motif,
        agent_cible,
        cours: cours
            .into_iter()
            .map(|(cours_id, libelle, public)| CoursRefDto {
                cours_id,
                libelle,
                public,
            })
            .collect(),
        du_cahier: du_c,
        au_cahier: au_c,
        valable_du: v_du,
        valable_au: v_au,
        etat: etat.to_string(),
        accorde_par: nom(tx, etab, Some(par)).await?,
        revoque_le: revoque,
        nb_consultations: nb,
    })
}

async fn controle(
    tx: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<ControleClotureDto, EntDbError> {
    let an: Option<(String, NaiveDate)> = sqlx::query_as("SELECT libelle, date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
    let Some((annee_libelle, debut)) = an else {
        return Err(introuvable());
    };
    let (deja_fige,): (bool,) = sqlx::query_as("SELECT ent_cdt_cahier_fige($1, $2)")
        .bind(etab)
        .bind(annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
    let (cours_total,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cours WHERE etablissement_id = $1 AND annee_id = $2",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let non_vises: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT c.id, c.libelle, COALESCE(cl.code, g.code, '')
           FROM ent_cours c
           LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
           LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
          WHERE c.etablissement_id = $1 AND c.annee_id = $2
            AND (EXISTS (SELECT 1 FROM ent_cdt_entree e WHERE e.etablissement_id = c.etablissement_id AND e.cours_id = c.id AND e.etat = 'publiee')
              OR EXISTS (SELECT 1 FROM ent_cdt_travail t WHERE t.etablissement_id = c.etablissement_id AND t.cours_id = c.id AND t.etat = 'publie'))
            AND NOT EXISTS (
                SELECT 1 FROM ent_cdt_visa v
                 WHERE v.etablissement_id = c.etablissement_id AND v.au >= $3
                   AND (v.cours_id = c.id OR (v.cible = 'classe' AND v.classe_id = c.classe_id)
                        OR EXISTS (SELECT 1 FROM ent_cdt_visa_element ve
                                     LEFT JOIN ent_cdt_entree e ON ve.objet = 'entree' AND e.id = ve.objet_id
                                     LEFT JOIN ent_cdt_travail t ON ve.objet = 'travail' AND t.id = ve.objet_id
                                    WHERE ve.etablissement_id = v.etablissement_id AND ve.visa_id = v.id AND COALESCE(e.cours_id, t.cours_id) = c.id)))
          ORDER BY c.libelle, c.id",
    )
    .bind(etab)
    .bind(annee_id)
    .bind(debut)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let (entrees_brouillon,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cdt_entree WHERE etablissement_id = $1 AND annee_id = $2 AND etat IN ('brouillon', 'programmee')",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let (travaux_brouillon,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_cdt_travail WHERE etablissement_id = $1 AND annee_id = $2 AND etat IN ('brouillon', 'programme')")
            .bind(etab)
            .bind(annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
    let (non_rendus,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM (
            SELECT DISTINCT ON (s.travail_id, s.eleve_id) s.statut
              FROM ent_cdt_statut s JOIN ent_cdt_travail t ON t.etablissement_id = s.etablissement_id AND t.id = s.travail_id
             WHERE s.etablissement_id = $1 AND t.annee_id = $2
             ORDER BY s.travail_id, s.eleve_id, s.cree_le DESC, s.id DESC) x
          WHERE x.statut = 'non_rendu'",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let (archives_existantes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cdt_archive WHERE etablissement_id = $1 AND annee_id = $2",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    Ok(ControleClotureDto {
        annee_id,
        annee_libelle,
        deja_fige,
        cours_total,
        cours_non_vises: non_vises
            .into_iter()
            .map(|(cours_id, libelle, public)| CoursRefDto {
                cours_id,
                libelle,
                public,
            })
            .collect(),
        entrees_brouillon,
        travaux_brouillon,
        non_rendus,
        archives_existantes,
    })
}
