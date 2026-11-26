//! Socle 27, **lot G4-1** : traces d'accès séparées du journal métier (§11, §17.5) et page « Consultations du dossier »
//! (RGP-191, RGP-192, RGP-310, R-RG8, R-RG28, R-RG29, R-RG31, R-RG46, R-RG49).
//!
//! * **Écriture** : `tracer_consultation_a` écrit `acc_trace` **dans la même transaction** que la ligne du journal et
//!   que la lecture qu'elle précède (R-RG8 : un échec d'écriture refuse la lecture). La ligne de journal et la trace
//!   partagent le même identifiant (la reprise de la migration 2546 se rejoue ainsi sans doublon). La fonction et le
//!   service de l'agent sont **figés** au moment de la consultation (R-RG31).
//! * **Hors périmètre** (RGP-191, R-RG49) : une consultation de la fiche d'un élève hors du périmètre de l'agent
//!   (classes, groupes, désignés du moteur de droits) demande un **motif** de la liste S-542, en un geste : le service
//!   répond « motif requis », l'écran propose la liste, la lecture est alors servie et tracée avec le motif. Un geste
//!   couvert par un **périmètre temporaire** (remplacement, conseil de classe, sortie : RGP-310) n'est pas « hors
//!   périmètre » ; la permanence n'est pas modélisée par ScolIA (valeur prudente, D-069).
//! * **Page « Consultations du dossier »** (§11.2, §11.4) : la famille (par enfant dont elle est titulaire **à la date de
//!   la consultation**), l'élève majeur, l'agent (son propre dossier) voient date, **fonction et service** — jamais le
//!   nom (§11.3, S-549) —, nature, section et motif, regroupés par jour, année scolaire en cours et précédente.
//!   Jamais : santé et suivi psychologique (R-RG29), lectures de la conformité, support de l'éditeur sans la mention
//!   « Support ScolIA, intervention autorisée le … », section protégée autrement que sous un code générique, et trace
//!   protégée pour un parent (R-RG46) autre que la personne protégée.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use domain_eleve::{GroupeChamps, PorteeGroupe};
use ent_auth::Role;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Message **fixe** de l'erreur « motif requis » (l'API le reconnaît, l'écran propose la liste S-542).
pub const MOTIF_REQUIS: &str = "motif_requis";

/// Motifs proposés **par défaut** pour une consultation hors périmètre (S-542). Un établissement peut renommer,
/// réordonner ou désactiver (`acc_motif`) ; jamais d'ajout de texte libre.
pub const MOTIFS_PAR_DEFAUT: &[(&str, &str)] = &[
    ("suivi_pedagogique", "Suivi pédagogique de l'élève"),
    ("vie_scolaire", "Vie scolaire ou discipline"),
    ("demande_famille", "Demande d'une famille"),
    (
        "gestion_administrative",
        "Gestion administrative du dossier",
    ),
    ("urgence", "Urgence"),
    ("autre_motif", "Autre motif justifié auprès de la direction"),
];

/// Libellé d'une section pour l'affichage (codes **génériques**, §11.4).
pub fn libelle_section(code: &str) -> &'static str {
    match code {
        "identite" => "Identité et identifiants",
        "coordonnees" => "Coordonnées",
        "coordonnees_protegees" => "Informations protégées",
        "section_protegee" => "Informations protégées",
        "scolarite" => "Scolarité",
        "internat" => "Internat",
        "dossier" => "Dossier de l'élève",
        "dossier_agent" => "Dossier de l'agent",
        _ => "Dossier de l'élève",
    }
}

pub fn libelle_nature(code: &str) -> &'static str {
    match code {
        "liste" => "Liste",
        "extrait" => "Extrait",
        "export" => "Export",
        _ => "Consultation",
    }
}

/// Un motif proposé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotifVue {
    pub code: String,
    pub libelle: String,
}

/// Où se situe l'agent par rapport à la fiche consultée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Perimetre {
    Dans,
    /// Couvert par un périmètre temporaire (code de la table `acc_trace`).
    Temporaire(&'static str),
    Hors,
}

/// Une ligne de la page « Consultations du dossier ».
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LigneConsultation {
    pub le: DateTime<Utc>,
    /// Jour **de l'établissement** (fuseau de l'établissement) : celui du regroupement.
    pub jour: NaiveDate,
    /// Fonction et service **figés** ; « Support ScolIA » pour le support de l'éditeur.
    pub fonction: String,
    pub service: String,
    pub nature: String,
    pub section: String,
    pub motif: Option<String>,
    /// Mention du support de l'éditeur (« Support ScolIA, intervention autorisée le … »).
    pub mention: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JourConsultations {
    pub jour: NaiveDate,
    pub lignes: Vec<LigneConsultation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DossierConsulte {
    /// `eleve` ou `agent`.
    pub sujet_type: String,
    pub sujet_id: Uuid,
    /// Prénom de l'élève (jamais plus) ; vide pour le dossier de l'agent.
    pub libelle: String,
    pub total: i64,
    pub jours: Vec<JourConsultations>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsultationsDossier {
    pub depuis: NaiveDate,
    pub dossiers: Vec<DossierConsulte>,
}

/// Libellé du service (famille de fonction) affiché.
fn service_de_famille(famille: &str) -> &'static str {
    match famille {
        "direction" => "Direction",
        "enseignement" => "Enseignement",
        "education" => "Éducation",
        "administration" => "Administration",
        "sante_social" => "Santé et social",
        "accompagnement" => "Accompagnement",
        "service_agents" => "Services généraux",
        "technique" => "Technique",
        "intervention" => "Intervenants",
        _ => "Établissement",
    }
}

/// Fonction et service de repli quand l'agent n'a pas de fonction déclarée : déduits du profil de base.
fn repli_fonction(role: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match role {
        "admin" => (
            "administrateur",
            "Administrateur",
            "administration",
            "Administration",
        ),
        "direction" => ("direction", "Direction", "direction", "Direction"),
        "administratif" => (
            "administratif",
            "Personnel administratif",
            "administration",
            "Administration",
        ),
        "cpe" => (
            "cpe",
            "Conseiller principal d'éducation",
            "education",
            "Éducation",
        ),
        "compta" => (
            "comptabilite",
            "Comptabilité",
            "administration",
            "Administration",
        ),
        "prof" => ("enseignant", "Enseignant", "enseignement", "Enseignement"),
        "infirmerie" => (
            "infirmerie",
            "Infirmerie",
            "sante_social",
            "Santé et social",
        ),
        "psy" => (
            "psychologue",
            "Psychologue",
            "sante_social",
            "Santé et social",
        ),
        "accueil" => ("accueil", "Accueil", "administration", "Administration"),
        _ => ("personnel", "Personnel", "autre", "Établissement"),
    }
}

/// Fonction et service figés d'un agent à un jour donné (R-RG31).
pub(crate) struct FonctionFigee {
    pub fonction_code: String,
    pub fonction_libelle: String,
    pub service_code: String,
    pub service_libelle: String,
}

pub(crate) async fn fonction_figee(
    conn: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    role: &str,
    jour: NaiveDate,
) -> Result<FonctionFigee, EntDbError> {
    let l: Option<(String, String, String)> = sqlx::query_as(
        "SELECT fo.code, fo.libelle, fo.famille
           FROM ent_personne p
           JOIN ent_agent a ON a.personne_id = p.id AND a.etablissement_id = p.etablissement_id
           JOIN ent_agent_fonction af ON af.agent_id = a.id AND af.etablissement_id = a.etablissement_id
           JOIN ent_fonction fo ON fo.id = af.fonction_id
          WHERE p.etablissement_id = $1 AND p.user_id = $2
            AND af.annule_le IS NULL AND af.du <= $3 AND (af.au IS NULL OR af.au >= $3)
          ORDER BY af.principale DESC, af.du DESC
          LIMIT 1",
    )
    .bind(etab)
    .bind(acteur)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(match l {
        Some((code, libelle, famille)) => FonctionFigee {
            fonction_code: code,
            fonction_libelle: libelle,
            service_libelle: service_de_famille(&famille).to_string(),
            service_code: famille,
        },
        None => {
            let (c, l, sc, sl) = repli_fonction(role);
            FonctionFigee {
                fonction_code: c.to_string(),
                fonction_libelle: l.to_string(),
                service_code: sc.to_string(),
                service_libelle: sl.to_string(),
            }
        }
    })
}

impl PgEnt {
    /// Motifs actifs proposés pour une consultation hors périmètre (S-542) : liste par défaut, ajustée par
    /// `acc_motif` (renommage, ordre, désactivation).
    pub async fn acces_motifs(&self, etab: Uuid) -> Result<Vec<MotifVue>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        motifs_actifs(&mut tx, etab).await
    }

    /// Crée à l'avance les partitions de traces (mois courant et deux suivants), RLS posée sur chacune.
    pub async fn acces_partitions_assurer_a(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let mut conn = self.pool().acquire().await?;
        sqlx::query("SELECT public.acc_trace_assurer_partitions($1)")
            .bind(maintenant.date_naive())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn acces_partitions_assurer(&self) -> Result<(), EntDbError> {
        self.acces_partitions_assurer_a(Utc::now()).await
    }

    /// Périmètre de l'agent pour la fiche d'un élève : la portée de **l'annuaire** (moteur de droits existant : classes,
    /// groupes, désignés, missions, service ; le remplacement du jour y est déjà compris), puis les périmètres temporaires
    /// qu'il ne couvre pas (conseil de classe, sortie : RGP-310). Un lecteur sans portée restreinte (établissement entier)
    /// ou sans droit d'annuaire (la lecture est alors refusée par ailleurs, sans possibilité de motif) est toujours « dans »
    /// son périmètre. Les liens avec l'élève du moteur de droits sont ceux du jour réel de l'établissement.
    pub(crate) async fn perimetre_acteur(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<Perimetre, EntDbError> {
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Ok(Perimetre::Dans);
        }
        let profil = self
            .profil_champs_conn(&mut *conn, etab, acteur, role)
            .await?;
        let PorteeGroupe::Ensemble(ens) = profil.portee(GroupeChamps::Designation) else {
            return Ok(Perimetre::Dans);
        };
        let remplacant: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM ent_eleve e
                   JOIN ent_edt_creneau c ON c.etablissement_id = e.etablissement_id AND c.classe = e.classe
                   JOIN ent_edt_exception x ON x.etablissement_id = c.etablissement_id AND x.creneau_id = c.id
                  WHERE e.etablissement_id = $1 AND e.id = $2 AND x.remplacant_user_id = $3 AND x.jour = $4)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await?;
        if remplacant {
            return Ok(Perimetre::Temporaire("remplacement"));
        }
        if ens.contains(&eleve) {
            return Ok(Perimetre::Dans);
        }
        let conseil: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM ent_eleve e
                   JOIN ent_conseil_classe cc ON cc.etablissement_id = e.etablissement_id AND cc.classe = e.classe
                  WHERE e.etablissement_id = $1 AND e.id = $2 AND cc.jour = $4
                    AND cc.statut <> 'annule' AND cc.created_by_user_id = $3)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await?;
        if conseil {
            return Ok(Perimetre::Temporaire("conseil_classe"));
        }
        let sortie: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM ent_sortie_participant pa
                   JOIN ent_sortie s ON s.etablissement_id = pa.etablissement_id AND s.id = pa.sortie_id
                   JOIN ent_sortie_encadrant en ON en.etablissement_id = s.etablissement_id AND en.sortie_id = s.id
                   JOIN ent_personne p ON p.etablissement_id = en.etablissement_id AND p.id = en.personne_id
                  WHERE pa.etablissement_id = $1 AND pa.eleve_id = $2 AND p.user_id = $3
                    AND pa.etat IN ('inscrit', 'pret', 'parti')
                    AND $4 BETWEEN (s.debut::date - 1) AND (s.fin::date + 1))",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await?;
        if sortie {
            return Ok(Perimetre::Temporaire("sortie"));
        }
        Ok(Perimetre::Hors)
    }

    /// Exige, pour la fiche d'un élève hors du périmètre de l'agent, un motif de la liste S-542 (RGP-191). Rend le
    /// motif retenu (code) quand il est fourni et valable. `MOTIF_REQUIS` sinon.
    pub async fn acces_exiger_motif_fiche_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        motif: Option<&str>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Ok(());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_de(&mut tx, etab, maintenant).await?;
        // Un élève absent de l'établissement n'est pas évalué : la lecture répondra « introuvable ».
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await?;
        if !existe {
            return Ok(());
        }
        if self
            .perimetre_acteur(&mut tx, etab, acteur, role, eleve, jour)
            .await?
            == Perimetre::Hors
        {
            valider_motif(&mut tx, etab, motif)
                .await?
                .ok_or(EntDbError::ConflitDossier(MOTIF_REQUIS))?;
        }
        Ok(())
    }

    /// Page « Consultations du dossier » (RGP-192) : ce que l'appelant a le droit de voir de **ses** dossiers.
    pub async fn acces_consultations_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ConsultationsDossier, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_de(&mut tx, etab, maintenant).await?;
        let depuis = debut_periode_affichee(&mut tx, etab, jour).await?;
        let mut dossiers: Vec<DossierConsulte> = Vec::new();
        // Les éléments que l'appelant peut consulter : (type, identifiant, libellé).
        let mut cibles: Vec<(&'static str, Uuid, String)> = Vec::new();
        match parsed {
            Role::Parent => {
                let enfants: Vec<(Uuid, String)> = sqlx::query_as(
                    "SELECT DISTINCT e.id, e.prenom
                       FROM ent_eleve_lien l JOIN ent_eleve e ON e.etablissement_id = l.etablissement_id AND e.id = l.eleve_id
                      WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.autorite_parentale
                        AND l.suspendu_le IS NULL AND (l.date_debut IS NULL OR l.date_debut <= $3)
                      ORDER BY e.prenom, e.id",
                )
                .bind(etab)
                .bind(acteur)
                .bind(jour)
                .fetch_all(&mut *tx)
                .await?;
                cibles.extend(enfants.into_iter().map(|(id, p)| ("eleve", id, p)));
            }
            Role::Eleve => {
                let moi: Option<(Uuid, String)> = sqlx::query_as(
                    "SELECT e.id, e.prenom FROM ent_personne p
                       JOIN ent_eleve e ON e.etablissement_id = p.etablissement_id AND e.id = p.eleve_id
                      WHERE p.etablissement_id = $1 AND p.user_id = $2",
                )
                .bind(etab)
                .bind(acteur)
                .fetch_optional(&mut *tx)
                .await?;
                if let Some((id, prenom)) = moi {
                    // Seul l'élève **majeur** exerce ses droits seul (§9.3) ; un mineur passe par ses responsables.
                    if crate::droits_demandes::eleve_majeur_le(&mut tx, etab, id, jour).await?
                        == Some(true)
                    {
                        cibles.push(("eleve", id, prenom));
                    }
                }
            }
            _ => {
                let moi: Option<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
                )
                .bind(etab)
                .bind(acteur)
                .fetch_optional(&mut *tx)
                .await?;
                if let Some(p) = moi {
                    cibles.push(("agent", p, String::new()));
                }
            }
        }
        for (type_, id, libelle) in cibles {
            let lignes = lire_lignes(&mut tx, etab, acteur, type_, id, depuis, jour).await?;
            let total = i64::try_from(lignes.len()).unwrap_or(i64::MAX);
            let mut par_jour: BTreeMap<NaiveDate, Vec<LigneConsultation>> = BTreeMap::new();
            for l in lignes {
                par_jour.entry(l.jour).or_default().push(l);
            }
            let jours = par_jour
                .into_iter()
                .rev()
                .map(|(jour, mut lignes)| {
                    lignes.sort_by_key(|l| std::cmp::Reverse(l.le));
                    JourConsultations { jour, lignes }
                })
                .collect();
            dossiers.push(DossierConsulte {
                sujet_type: type_.to_string(),
                sujet_id: id,
                libelle,
                total,
                jours,
            });
        }
        Ok(ConsultationsDossier { depuis, dossiers })
    }

    pub async fn acces_consultations(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<ConsultationsDossier, EntDbError> {
        self.acces_consultations_a(etab, acteur, role, Utc::now())
            .await
    }
}

async fn jour_de(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<NaiveDate, EntDbError> {
    Ok(sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(&mut *conn)
        .await?)
}

/// Début de la période affichée : début de l'année scolaire **précédente** (année en cours et précédente, §11.2) ;
/// sans année déclarée, deux ans avant la date.
async fn debut_periode_affichee(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
) -> Result<NaiveDate, EntDbError> {
    let courante: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
          ORDER BY date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(courante) = courante else {
        return Ok(jour
            .with_year(jour.year() - 2)
            .unwrap_or(jour - chrono::Duration::days(730)));
    };
    let precedente: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut < $2
          ORDER BY date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(courante)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(precedente.unwrap_or_else(|| courante.with_year(courante.year() - 1).unwrap_or(courante)))
}

/// Les lignes visibles par l'appelant pour un sujet (élève ou dossier d'agent).
async fn lire_lignes(
    conn: &mut PgConnection,
    etab: Uuid,
    viewer: Uuid,
    type_: &str,
    sujet: Uuid,
    depuis: NaiveDate,
    jour: NaiveDate,
) -> Result<Vec<LigneConsultation>, EntDbError> {
    let motifs: BTreeMap<String, String> = motifs_actifs(&mut *conn, etab)
        .await?
        .into_iter()
        .map(|m| (m.code, m.libelle))
        .collect();
    #[allow(clippy::type_complexity)]
    let lignes: Vec<(DateTime<Utc>, NaiveDate, String, String, String, String, Option<String>, bool, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT t.le, ent_aujourdhui_etab(t.etablissement_id, t.le), t.fonction_libelle, t.service_libelle, t.nature, t.section, t.motif_code,
                t.support_editeur, t.support_intervention_le
           FROM acc_trace t
          WHERE t.etablissement_id = $1
            AND (($2 = 'eleve' AND t.sujet_eleve_id = $3) OR ($2 = 'agent' AND t.sujet_personne_id = $3))
            AND t.le >= $4::date::timestamptz AND t.le < ($5::date + 1)::timestamptz
            AND t.affichable
            -- R-RG46 : une trace protégée n'est montrée qu'à la personne protégée elle-même.
            AND (NOT t.protegee OR EXISTS (
                   SELECT 1 FROM ent_personne vp
                    WHERE vp.etablissement_id = t.etablissement_id AND vp.user_id = $6
                      AND (EXISTS (SELECT 1 FROM ent_adresse_protegee a
                                    WHERE a.etablissement_id = vp.etablissement_id AND a.personne_id = vp.id)
                           OR EXISTS (SELECT 1 FROM ent_personne_reserve r
                                       WHERE r.etablissement_id = vp.etablissement_id AND r.personne_id = vp.id
                                         AND r.ne_pas_solliciter))))
            -- Une famille ne voit que les dates où elle était titulaire (lien daté, autorité parentale).
            AND ($2 <> 'eleve' OR EXISTS (
                   SELECT 1 FROM ent_eleve_lien l
                    WHERE l.etablissement_id = t.etablissement_id AND l.eleve_id = t.sujet_eleve_id
                      AND l.user_id = $6 AND l.autorite_parentale
                      AND (l.date_debut IS NULL OR l.date_debut <= t.le::date)
                      AND (l.date_fin IS NULL OR l.date_fin >= t.le::date))
                OR EXISTS (SELECT 1 FROM ent_personne mp
                            WHERE mp.etablissement_id = t.etablissement_id AND mp.user_id = $6
                              AND mp.eleve_id = t.sujet_eleve_id))
          ORDER BY t.le DESC
          LIMIT 5000",
    )
    .bind(etab)
    .bind(type_)
    .bind(sujet)
    .bind(depuis)
    .bind(jour)
    .bind(viewer)
    .fetch_all(&mut *conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(le, jour, fonction, service, nature, section, motif, support, intervention)| {
                let (fonction, service, mention) = if support {
                    let m = intervention
                        .map(|d| {
                            format!(
                                "Support ScolIA, intervention autorisée le {}",
                                d.format("%d/%m/%Y")
                            )
                        })
                        .unwrap_or_else(|| "Support ScolIA".to_string());
                    ("Support ScolIA".to_string(), String::new(), Some(m))
                } else {
                    (fonction, service, None)
                };
                LigneConsultation {
                    le,
                    jour,
                    fonction,
                    service,
                    nature: libelle_nature(&nature).to_string(),
                    section: libelle_section(&section).to_string(),
                    motif: motif.and_then(|c| motifs.get(&c).cloned()),
                    mention,
                }
            },
        )
        .collect())
}

/// Motifs actifs d'un établissement (défaut du code + ajustements de `acc_motif`).
pub(crate) async fn motifs_actifs(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<Vec<MotifVue>, EntDbError> {
    let lignes: Vec<(String, String, i16, bool)> = sqlx::query_as(
        "SELECT code, libelle, rang, actif FROM acc_motif WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await?;
    let mut v: Vec<(i16, MotifVue, bool)> = MOTIFS_PAR_DEFAUT
        .iter()
        .enumerate()
        .map(|(i, (c, l))| {
            (
                i16::try_from(i).unwrap_or(0),
                MotifVue {
                    code: (*c).to_string(),
                    libelle: (*l).to_string(),
                },
                true,
            )
        })
        .collect();
    for (code, libelle, rang, actif) in lignes {
        match v.iter_mut().find(|(_, m, _)| m.code == code) {
            Some(e) => *e = (rang, MotifVue { code, libelle }, actif),
            None => v.push((rang, MotifVue { code, libelle }, actif)),
        }
    }
    v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.code.cmp(&b.1.code)));
    Ok(v.into_iter()
        .filter(|(_, _, actif)| *actif)
        .map(|(_, m, _)| m)
        .collect())
}

/// `Some(code)` si le motif fourni est dans la liste active ; `None` s'il est absent ; refus s'il est inconnu.
pub(crate) async fn valider_motif(
    conn: &mut PgConnection,
    etab: Uuid,
    motif: Option<&str>,
) -> Result<Option<String>, EntDbError> {
    let Some(m) = motif.map(str::trim).filter(|m| !m.is_empty()) else {
        return Ok(None);
    };
    if motifs_actifs(&mut *conn, etab)
        .await?
        .iter()
        .any(|x| x.code == m)
    {
        Ok(Some(m.to_string()))
    } else {
        Err(EntDbError::SaisieInvalide("motif de consultation inconnu"))
    }
}

/// Ce que la trace doit porter en plus de l'action (périmètre, motif).
pub(crate) struct ContexteTrace<'a> {
    pub perimetre: Perimetre,
    pub motif: Option<&'a str>,
}

/// Écrit la ligne `acc_trace` dans la transaction **de l'appelant** (R-RG8). `id` est celui de la ligne de journal.
pub(crate) async fn ecrire_trace_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    acteur: Uuid,
    role: &str,
    eleve: Option<Uuid>,
    action: &str,
    ctx: &ContexteTrace<'_>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    // La partition du mois de la trace (et les deux suivantes) existe avant l'insertion.
    sqlx::query("SELECT public.acc_trace_assurer_partitions($1)")
        .bind(maintenant.date_naive())
        .execute(&mut *conn)
        .await?;
    let jour = jour_de(&mut *conn, etab, maintenant).await?;
    let f = fonction_figee(&mut *conn, etab, acteur, role, jour).await?;
    let (nature, section, affichable, protegee): (String, String, bool, bool) = sqlx::query_as(
        "SELECT public.acc_trace_nature($1),
                public.acc_trace_section_finale($1, $2, $3),
                public.acc_trace_affichable(public.acc_trace_section($1)),
                public.acc_trace_protegee($2, $3, public.acc_trace_section($1))",
    )
    .bind(action)
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut *conn)
    .await?;
    let (hors, temporaire) = match ctx.perimetre {
        Perimetre::Dans => (false, None),
        Perimetre::Temporaire(t) => (false, Some(t)),
        Perimetre::Hors => (true, None),
    };
    sqlx::query(
        "INSERT INTO acc_trace (id, etablissement_id, le, acteur_compte_id, acteur_role, fonction_code, fonction_libelle,
                                service_code, service_libelle, nature, section, action, sujet_eleve_id, hors_perimetre,
                                perimetre_temporaire, motif_code, affichable, protegee)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, left($12, 120), $13, $14, $15, $16, $17, $18)",
    )
    .bind(id)
    .bind(etab)
    .bind(maintenant)
    .bind(acteur)
    .bind(role)
    .bind(&f.fonction_code)
    .bind(&f.fonction_libelle)
    .bind(&f.service_code)
    .bind(&f.service_libelle)
    .bind(nature)
    .bind(section)
    .bind(action)
    .bind(eleve)
    .bind(hors)
    .bind(temporaire)
    .bind(if hors { ctx.motif } else { None })
    .bind(affichable)
    .bind(protegee)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
