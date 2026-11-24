//! Socle 42, lot V0d : reprise des séjours de Next.js — commandes et lectures
//! (`docs/socles/42_SORTIES_VOYAGES.md` §26, §32.1 à §32.4 ; VOY-258, VOY-259, VOY-263 ; R-SV37 à R-SV40, R-SV46).
//!
//! * **aucune connexion à la production** : la reprise charge un **export** fourni par Florian (anonymisé pour
//!   les essais) sous forme d'enregistrements typés ; le dossier aplati de Next.js n'est jamais conservé ;
//! * **droit** : `voyages.reprise` (administrateur, direction) — la reprise voit des séjours entiers ;
//! * **propriété unique** : un séjour est modifiable dans un seul outil ; la reprise **n'écrit jamais** sur
//!   un séjour dont ScolIA est propriétaire, elle **signale** l'écart (R-SV37) ;
//! * **idempotence** par `source_id` : rejouer la même reprise ne duplique rien ;
//! * **rapprochement** des élèves : identifiant connu, puis INE unique, puis file humaine ; **jamais**
//!   d'approximation ; l'identité de l'export est effacée dès le rapprochement (minimisation) ;
//! * **montants** repris en centimes ; « 1.350 » est signalé, jamais deviné ; la part annoncée avant la
//!   bascule est conservée telle quelle (R-SV46), l'écart avec la règle de ScolIA est signalé ;
//! * **aides individuelles nominatives** : non reprises, signalées (VD-37, Q-V29) ;
//! * **bascule** : tableau tout vert (ou dérogation motivée sur les contrôles dérogeables) ; elle crée alors
//!   la sortie ScolIA et rend la propriété à ScolIA ; **aucune écriture** n'est faite dans Next.js.

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages::reference;
use crate::voyages_budget::montant_en_centimes;
use crate::voyages_repo::{annee_de, journaliser, publier, refus, tx, Fait};
use crate::voyages_reprise::{
    basculable, controles, etat_cible, ine_normalise, rapprocher, regime, Controle, FaitsBascule, Rapprochement,
    STATUTS_SOURCE,
};

const SEJOURS_MAX: usize = 500;
const PARTICIPANTS_MAX: usize = 400;

// ───────────────────────── Types ─────────────────────────

/// Dates de la bascule (S-436, décidées par Florian) : `gel` et dernier jour de départ « en vol ».
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ParamsReprise {
    pub gel: NaiveDate,
    pub fin_vol: NaiveDate,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParticipantImport {
    pub source_id: String,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    #[serde(default)]
    pub ine: Option<String>,
    #[serde(default)]
    pub classe: Option<String>,
    /// Identifiant ScolIA déjà connu (export enrichi) ; vérifié dans l'établissement.
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SejourImport {
    pub source_id: String,
    pub statut_source: String,
    #[serde(default)]
    pub statut_precedent: Option<String>,
    /// `SIMPLE` ou `COMPLEX`.
    pub type_source: String,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub nature: Option<String>,
    #[serde(default)]
    pub nuitees: bool,
    #[serde(default)]
    pub besoin_car: bool,
    #[serde(default)]
    pub classes_texte: Option<String>,
    #[serde(default)]
    pub porteur_personne_id: Option<Uuid>,
    /// Montants en TEXTE (format français strict) : « 1.350 » est signalé, jamais lu 1,35 €.
    #[serde(default)]
    pub budget_total: Option<String>,
    #[serde(default)]
    pub part_annoncee: Option<String>,
    /// Part recalculée par les règles de ScolIA (si la comptabilité l'a produite) : écart signalé à la direction.
    #[serde(default)]
    pub part_recalculee: Option<String>,
    #[serde(default)]
    pub pieces_attendues: i32,
    /// Nombre d'aides nominatives de la fiche budget : non reprises, signalées à la comptabilité.
    #[serde(default)]
    pub aides_nominatives: i32,
    #[serde(default)]
    pub participants: Vec<ParticipantImport>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RapportReprise {
    pub crees: usize,
    pub mis_a_jour: usize,
    /// Séjours dont ScolIA est propriétaire : jamais réécrits ; un écart a pu être signalé.
    pub intacts_scolia: usize,
    pub participants_rapproches: usize,
    pub participants_a_rapprocher: usize,
    pub ecarts: usize,
    pub statuts_inconnus: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LigneBascule {
    pub id: Uuid,
    pub source_id: String,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub proprietaire: String,
    pub regime: String,
    pub statut_source: String,
    pub etat_cible: String,
    pub participants_total: i64,
    pub participants_a_rapprocher: i64,
    pub ecarts_non_traites: i64,
    pub pieces_attendues: i32,
    pub pieces_copiees: i32,
    pub budget_total_centimes: Option<i64>,
    pub part_annoncee_centimes: Option<i64>,
    pub derogation: bool,
    pub controles: Vec<Controle>,
    pub basculable: bool,
    /// Autorisations papier attestées et versements importés : suivis dès les lots V1 et V2a.
    pub attestations: Option<i64>,
    pub versements: Option<i64>,
    pub sortie_id: Option<Uuid>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct EcartVue {
    pub id: Uuid,
    pub code: String,
    pub detail: String,
    pub traite_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ParticipantReprise {
    pub id: Uuid,
    pub source_id: String,
    pub nom_source: Option<String>,
    pub prenom_source: Option<String>,
    pub classe_source: Option<String>,
    pub statut: String,
    pub eleve_id: Option<Uuid>,
}

// ───────────────────────── Aides ─────────────────────────

#[derive(FromRow)]
struct SejourLigne {
    proprietaire: String,
    regime: String,
    statut_source: String,
    etat_cible: String,
    etat_retour_cible: Option<String>,
    titre: String,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    nature: String,
    forme: String,
    besoin_car: bool,
    porteur_personne_id: Option<Uuid>,
    budget_total_centimes: Option<i64>,
    part_annoncee_centimes: Option<i64>,
    pieces_attendues: i32,
    pieces_copiees: i32,
    derogation_motif: Option<String>,
    sortie_id: Option<Uuid>,
    source_id: String,
}

async fn charger(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<SejourLigne, EntDbError> {
    let sql = format!(
        "SELECT id, proprietaire, regime, statut_source, etat_cible, etat_retour_cible, titre, debut, fin, nature, forme,
                besoin_car, porteur_personne_id, budget_total_centimes, part_annoncee_centimes, pieces_attendues,
                pieces_copiees, derogation_motif, sortie_id, source_id
           FROM ent_sortie_reprise_sejour WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as(&sql).bind(etab).bind(id).fetch_optional(conn).await.map_err(db)?.ok_or_else(introuvable)
}

/// Jour de l'établissement d'un instant (Europe/Paris, comme le reste du socle).
async fn jour_de(conn: &mut PgConnection, instant: DateTime<Utc>) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE 'Europe/Paris')::date")
        .bind(instant)
        .fetch_one(conn)
        .await
        .map_err(db)
}

async fn aujourdhui(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)").bind(etab).fetch_one(conn).await.map_err(db)
}

async fn ecart(conn: &mut PgConnection, etab: Uuid, sejour: Uuid, code: &str, detail: &str) -> Result<bool, EntDbError> {
    let n = sqlx::query(
        "INSERT INTO ent_sortie_reprise_ecart (id, etablissement_id, sejour_id, code, detail)
         VALUES ($1,$2,$3,$4,$5) ON CONFLICT (sejour_id, code, detail) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(sejour)
    .bind(code)
    .bind(detail.chars().take(300).collect::<String>())
    .execute(conn)
    .await
    .map_err(db)?
    .rows_affected();
    Ok(n > 0)
}

fn net(s: &Option<String>, max: usize) -> Option<String> {
    s.as_deref().map(str::trim).filter(|x| !x.is_empty()).map(|x| x.chars().take(max).collect())
}

/// Montant d'un export : `Ok(None)` si absent, `Err(brut)` si douteux (jamais deviné).
fn montant(s: &Option<String>) -> Result<Option<i64>, String> {
    match s.as_deref().map(str::trim).filter(|x| !x.is_empty()) {
        None => Ok(None),
        Some(t) => montant_en_centimes(t).map(Some).map_err(|_| t.chars().take(40).collect()),
    }
}

async fn rapprocher_participants(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sejour: Uuid,
    ps: &[ParticipantImport],
    rapport: &mut RapportReprise,
) -> Result<(), EntDbError> {
    for p in ps {
        let src = p.source_id.trim();
        if src.is_empty() || src.chars().count() > 100 {
            return Err(invalide());
        }
        let deja: Option<(String,)> = sqlx::query_as(
            "SELECT statut FROM ent_sortie_reprise_participant WHERE etablissement_id = $1 AND sejour_id = $2 AND source_id = $3",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(src)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((st,)) = deja {
            // Rejeu : un participant déjà rapproché ou exclu n'est pas retouché ; un autre peut l'être.
            if st != "a_rapprocher" {
                if st == "rapproche" {
                    rapport.participants_rapproches += 1;
                }
                continue;
            }
        }
        let connu: bool = match p.eleve_id {
            None => false,
            Some(e) => sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)")
                .bind(a.etablissement_id)
                .bind(e)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?,
        };
        let ine = p.ine.as_deref().and_then(ine_normalise);
        let (eleve_par_ine, nb): (Option<Uuid>, usize) = match &ine {
            None => (None, 0),
            Some(i) => {
                let v: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND upper(ine) = $2 LIMIT 2",
                )
                .bind(a.etablissement_id)
                .bind(i)
                .fetch_all(&mut *conn)
                .await
                .map_err(db)?;
                (v.first().copied(), v.len())
            }
        };
        // Un même élève ne figure qu'une fois par séjour : si déjà pris, la file humaine tranche.
        let decision = rapprocher(connu, ine.is_some(), nb);
        let (eleve, methode) = match decision {
            Rapprochement::EleveId => (p.eleve_id, Some("eleve_id")),
            Rapprochement::Ine => (eleve_par_ine, Some("ine")),
            Rapprochement::ARapprocher => (None, None),
        };
        let pris: bool = match eleve {
            None => false,
            Some(e) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_reprise_participant
                                 WHERE etablissement_id = $1 AND sejour_id = $2 AND eleve_id = $3 AND source_id <> $4)",
            )
            .bind(a.etablissement_id)
            .bind(sejour)
            .bind(e)
            .bind(src)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?,
        };
        let (eleve, methode) = if pris { (None, None) } else { (eleve, methode) };
        if let (Some(e), Some(m)) = (eleve, methode) {
            sqlx::query(
                "INSERT INTO ent_sortie_reprise_participant
                     (id, etablissement_id, sejour_id, source_id, eleve_id, statut, methode, rapproche_le)
                 VALUES ($1,$2,$3,$4,$5,'rapproche',$6, now())
                 ON CONFLICT (sejour_id, source_id) DO UPDATE
                    SET eleve_id = EXCLUDED.eleve_id, statut = 'rapproche', methode = EXCLUDED.methode,
                        rapproche_le = now(), nom_source = NULL, prenom_source = NULL, ine_source = NULL",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(sejour)
            .bind(src)
            .bind(e)
            .bind(m)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            rapport.participants_rapproches += 1;
        } else {
            sqlx::query(
                "INSERT INTO ent_sortie_reprise_participant
                     (id, etablissement_id, sejour_id, source_id, nom_source, prenom_source, ine_source, classe_source)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
                 ON CONFLICT (sejour_id, source_id) DO UPDATE
                    SET nom_source = EXCLUDED.nom_source, prenom_source = EXCLUDED.prenom_source,
                        ine_source = EXCLUDED.ine_source, classe_source = EXCLUDED.classe_source",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(sejour)
            .bind(src)
            .bind(net(&p.nom, 100))
            .bind(net(&p.prenom, 100))
            .bind(&ine)
            .bind(net(&p.classe, 50))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            rapport.participants_a_rapprocher += 1;
        }
    }
    Ok(())
}

async fn journal_reprise(conn: &mut PgConnection, a: &Acteur<'_>, action: &str, libelle: &str) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle, actor_user_id)
         VALUES ($1,$2,NULL,'voyages_reprise',$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(action)
    .bind(libelle)
    .bind(a.user_id)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

// ───────────────────────── Commandes ─────────────────────────

impl PgEnt {
    /// VOY-259 : charge (ou recharge, différentiel et idempotent) des séjours de l'export de Next.js dans
    /// la zone de reprise. Un séjour dont ScolIA est propriétaire n'est **jamais** réécrit : un écart
    /// constaté est signalé (R-SV37).
    pub async fn voyages_reprise_importer(
        &self,
        a: &Acteur<'_>,
        params: ParamsReprise,
        sejours: Vec<SejourImport>,
    ) -> Result<RapportReprise, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        if sejours.len() > SEJOURS_MAX || params.fin_vol < params.gel {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let jour = aujourdhui(&mut t, a.etablissement_id).await?;
        let mut rapport = RapportReprise::default();
        for s in sejours {
            let source = s.source_id.trim().to_string();
            if source.is_empty() || source.chars().count() > 100 || s.participants.len() > PARTICIPANTS_MAX || s.fin <= s.debut {
                return Err(invalide());
            }
            if !STATUTS_SOURCE.contains(&s.statut_source.as_str()) {
                rapport.statuts_inconnus.push(s.statut_source.chars().take(40).collect());
                continue;
            }
            if !matches!(s.type_source.as_str(), "SIMPLE" | "COMPLEX") {
                return Err(invalide());
            }
            let titre = net(&Some(s.titre.clone()), 200).ok_or_else(invalide)?;
            let (d_jour, f_jour) = (jour_de(&mut t, s.debut).await?, jour_de(&mut t, s.fin).await?);
            let (cible, retour, avert) = etat_cible(&s.statut_source, s.statut_precedent.as_deref(), f_jour, jour)
                .ok_or_else(invalide)?;
            let reg = regime(d_jour, f_jour, params.gel, params.fin_vol, jour);
            let forme = if s.type_source == "COMPLEX" && s.nuitees { "voyage" } else { "sans_nuitee" };
            let nature = match s.nature.as_deref() {
                Some("obligatoire") => "obligatoire",
                _ => "facultative",
            };
            let (budget, part, recalculee) = (montant(&s.budget_total), montant(&s.part_annoncee), montant(&s.part_recalculee));
            let existant: Option<(Uuid, String)> = sqlx::query_as(
                "SELECT id, proprietaire FROM ent_sortie_reprise_sejour WHERE etablissement_id = $1 AND source_id = $2 FOR UPDATE",
            )
            .bind(a.etablissement_id)
            .bind(&source)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            let id = match existant {
                Some((id, prop)) if prop == "scolia" => {
                    // Propriété à ScolIA : on compare, on signale, on n'écrit pas.
                    let (t0, d0, f0, st0): (String, DateTime<Utc>, DateTime<Utc>, String) = sqlx::query_as(
                        "SELECT titre, debut, fin, statut_source FROM ent_sortie_reprise_sejour WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(a.etablissement_id)
                    .bind(id)
                    .fetch_one(&mut *t)
                    .await
                    .map_err(db)?;
                    let mut diff = Vec::new();
                    if t0 != titre {
                        diff.push("titre");
                    }
                    if d0 != s.debut || f0 != s.fin {
                        diff.push("dates");
                    }
                    if st0 != s.statut_source {
                        diff.push("statut");
                    }
                    if !diff.is_empty() && ecart(&mut t, a.etablissement_id, id, "ecart_apres_bascule", &format!("modifié dans Next.js après la bascule : {}", diff.join(", "))).await? {
                        rapport.ecarts += 1;
                    }
                    rapport.intacts_scolia += 1;
                    continue;
                }
                Some((id, _)) => {
                    sqlx::query(
                        "UPDATE ent_sortie_reprise_sejour
                            SET statut_source = $3, etat_cible = $4, etat_retour_cible = $5, regime = $6, type_source = $7,
                                titre = $8, debut = $9, fin = $10, nature = $11, forme = $12, besoin_car = $13,
                                classes_texte = $14, porteur_personne_id = COALESCE($15, porteur_personne_id),
                                budget_total_centimes = $16, part_annoncee_centimes = $17,
                                pieces_attendues = $18, modifie_le = now(), importe_le = now(), importe_par_user_id = $19
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(a.etablissement_id)
                    .bind(id)
                    .bind(&s.statut_source)
                    .bind(cible)
                    .bind(retour)
                    .bind(reg)
                    .bind(&s.type_source)
                    .bind(&titre)
                    .bind(s.debut)
                    .bind(s.fin)
                    .bind(nature)
                    .bind(forme)
                    .bind(s.besoin_car)
                    .bind(net(&s.classes_texte, 500))
                    .bind(s.porteur_personne_id)
                    .bind(budget.clone().ok().flatten())
                    .bind(part.clone().ok().flatten())
                    .bind(s.pieces_attendues.max(0))
                    .bind(a.user_id)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                    rapport.mis_a_jour += 1;
                    id
                }
                None => {
                    let id = Uuid::new_v4();
                    sqlx::query(
                        "INSERT INTO ent_sortie_reprise_sejour
                             (id, etablissement_id, source_id, statut_source, etat_cible, etat_retour_cible, regime, type_source,
                              titre, debut, fin, nature, forme, besoin_car, classes_texte, porteur_personne_id,
                              budget_total_centimes, part_annoncee_centimes, pieces_attendues, importe_par_user_id)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
                    )
                    .bind(id)
                    .bind(a.etablissement_id)
                    .bind(&source)
                    .bind(&s.statut_source)
                    .bind(cible)
                    .bind(retour)
                    .bind(reg)
                    .bind(&s.type_source)
                    .bind(&titre)
                    .bind(s.debut)
                    .bind(s.fin)
                    .bind(nature)
                    .bind(forme)
                    .bind(s.besoin_car)
                    .bind(net(&s.classes_texte, 500))
                    .bind(s.porteur_personne_id)
                    .bind(budget.clone().ok().flatten())
                    .bind(part.clone().ok().flatten())
                    .bind(s.pieces_attendues.max(0))
                    .bind(a.user_id)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                    rapport.crees += 1;
                    id
                }
            };
            // Écarts du rapport de reprise.
            let mut nouveaux = Vec::new();
            if let Some(w) = avert {
                nouveaux.push(("statut_herite", w.to_string()));
            }
            for (nom, m) in [("budget_total", &budget), ("part_annoncee", &part), ("part_recalculee", &recalculee)] {
                if let Err(brut) = m {
                    nouveaux.push(("montant_douteux", format!("{nom} : « {brut} » non reconnu")));
                }
            }
            if let (Ok(Some(x)), Ok(Some(y))) = (&part, &recalculee) {
                if x != y {
                    nouveaux.push((
                        "part_annoncee_differente",
                        format!("annoncée {},{:02} € ; règle ScolIA {},{:02} €", x / 100, x % 100, y / 100, y % 100),
                    ));
                }
            }
            if s.aides_nominatives > 0 {
                nouveaux.push(("aide_nominative_retiree", format!("{} aide(s) nominative(s) non reprise(s) : à la comptabilité", s.aides_nominatives)));
            }
            if s.porteur_personne_id.is_none() {
                nouveaux.push(("porteur_a_designer", "aucun porteur rapproché".to_string()));
            }
            for (code, detail) in nouveaux {
                if ecart(&mut t, a.etablissement_id, id, code, &detail).await? {
                    rapport.ecarts += 1;
                }
            }
            rapprocher_participants(&mut t, a, id, &s.participants, &mut rapport).await?;
        }
        journal_reprise(&mut t, a, "reprise_importee", &format!("Reprise de {} séjour(s) de Next.js", rapport.crees + rapport.mis_a_jour)).await?;
        t.commit().await.map_err(db)?;
        Ok(rapport)
    }

    /// Rapprochement humain d'un participant de l'export avec un élève de l'établissement. L'identité de
    /// l'export est effacée dans le même geste.
    pub async fn voyages_reprise_rapprocher(
        &self,
        a: &Acteur<'_>,
        sejour: Uuid,
        participant_source_id: &str,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a.etablissement_id, sejour, true).await?;
        if s.proprietaire == "scolia" {
            return Err(refus(RefusMetier::Conflit, "Ce séjour est déjà à ScolIA : le rapprochement est clos."));
        }
        let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)")
            .bind(a.etablissement_id)
            .bind(eleve_id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let pris: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_reprise_participant
                             WHERE etablissement_id = $1 AND sejour_id = $2 AND eleve_id = $3 AND source_id <> $4)",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(eleve_id)
        .bind(participant_source_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if pris {
            return Err(conflit());
        }
        let n = sqlx::query(
            "UPDATE ent_sortie_reprise_participant
                SET eleve_id = $4, statut = 'rapproche', methode = 'humain', rapproche_le = now(), rapproche_par_user_id = $5,
                    nom_source = NULL, prenom_source = NULL, ine_source = NULL
              WHERE etablissement_id = $1 AND sejour_id = $2 AND source_id = $3 AND statut <> 'exclu'",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(participant_source_id)
        .bind(eleve_id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journal_reprise(&mut t, a, "reprise_participant_rapproche", "Participant rapproché à la main").await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Désigne le porteur (une personne de l'établissement) d'un séjour repris.
    pub async fn voyages_reprise_porteur_definir(&self, a: &Acteur<'_>, sejour: Uuid, personne: Uuid) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a.etablissement_id, sejour, true).await?;
        if s.proprietaire == "scolia" {
            return Err(refus(RefusMetier::Conflit, "Ce séjour est déjà à ScolIA."));
        }
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND fusionnee_dans IS NULL)",
        )
        .bind(a.etablissement_id)
        .bind(personne)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        sqlx::query("UPDATE ent_sortie_reprise_sejour SET porteur_personne_id = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(sejour)
            .bind(personne)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_sortie_reprise_ecart SET traite_le = now(), traite_par_user_id = $3
              WHERE etablissement_id = $1 AND sejour_id = $2 AND code = 'porteur_a_designer' AND traite_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journal_reprise(&mut t, a, "reprise_porteur_defini", "Porteur désigné").await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Décision humaine tracée sur un écart (« traité ») : il ne se supprime pas.
    pub async fn voyages_reprise_ecart_traiter(&self, a: &Acteur<'_>, ecart_id: Uuid) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let n = sqlx::query(
            "UPDATE ent_sortie_reprise_ecart SET traite_le = now(), traite_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2 AND traite_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(ecart_id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journal_reprise(&mut t, a, "reprise_ecart_traite", "Écart de reprise traité").await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Pièces d'un séjour : nombre attendu et nombre déjà copié (avec contrôle d'empreinte) vers le dépôt du
    /// socle 7. La copie elle-même est faite par l'outil de reprise ; ici, le compte et son contrôle.
    pub async fn voyages_reprise_pieces_definir(&self, a: &Acteur<'_>, sejour: Uuid, copiees: i32) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a.etablissement_id, sejour, true).await?;
        if s.proprietaire == "scolia" || copiees < 0 || copiees > s.pieces_attendues {
            return Err(invalide());
        }
        sqlx::query("UPDATE ent_sortie_reprise_sejour SET pieces_copiees = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(sejour)
            .bind(copiees)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Dérogation écrite (motivée) pour basculer malgré des contrôles dérogeables rouges (écarts, pièces).
    /// Elle ne lève jamais « en vol », « déjà basculé », un participant non rapproché ni l'absence de porteur.
    pub async fn voyages_reprise_deroger(&self, a: &Acteur<'_>, sejour: Uuid, motif: &str) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let motif = motif.trim();
        if motif.is_empty() || motif.chars().count() > 1000 {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a.etablissement_id, sejour, true).await?;
        if s.proprietaire == "scolia" {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_sortie_reprise_sejour SET derogation_motif = $3, derogation_par_user_id = $4, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journal_reprise(&mut t, a, "reprise_derogation", "Dérogation de bascule accordée").await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// VOY-263 : tableau de bascule, une ligne par séjour repris.
    pub async fn voyages_reprise_tableau(&self, a: &Acteur<'_>) -> Result<Vec<LigneBascule>, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_sortie_reprise_sejour WHERE etablissement_id = $1 ORDER BY debut, source_id",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for id in ids {
            out.push(ligne(&mut t, a.etablissement_id, id).await?);
        }
        t.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Écarts d'un séjour repris.
    pub async fn voyages_reprise_ecarts(&self, a: &Acteur<'_>, sejour: Uuid) -> Result<Vec<EcartVue>, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let v = sqlx::query_as(
            "SELECT id, code, detail, traite_le FROM ent_sortie_reprise_ecart
              WHERE etablissement_id = $1 AND sejour_id = $2 ORDER BY cree_le, code",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Participants d'un séjour repris encore à rapprocher (identité de l'export, pour la file humaine).
    pub async fn voyages_reprise_a_rapprocher(&self, a: &Acteur<'_>, sejour: Uuid) -> Result<Vec<ParticipantReprise>, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let v = sqlx::query_as(
            "SELECT id, source_id, nom_source, prenom_source, classe_source, statut, eleve_id
               FROM ent_sortie_reprise_participant
              WHERE etablissement_id = $1 AND sejour_id = $2 AND statut = 'a_rapprocher' ORDER BY nom_source, source_id",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Bascule d'un séjour : crée la sortie ScolIA et rend la propriété à ScolIA. Tableau tout vert, ou
    /// dérogation motivée sur les contrôles dérogeables. Rien n'est écrit dans Next.js.
    pub async fn voyages_reprise_basculer(&self, a: &Acteur<'_>, sejour: Uuid, confirme: bool) -> Result<Uuid, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        if !confirme {
            return Err(refus(RefusMetier::Invalide, "Confirmez la bascule après avoir lu le tableau."));
        }
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a.etablissement_id, sejour, true).await?;
        let l = ligne(&mut t, a.etablissement_id, sejour).await?;
        if !l.basculable {
            return Err(refus(RefusMetier::Conflit, "Le séjour n'est pas basculable : voir le tableau de bascule."));
        }
        let porteur = s.porteur_personne_id.ok_or_else(invalide)?;
        let annee = annee_de(&mut t, a, s.debut).await?;
        let sortie = Uuid::new_v4();
        let confirmee = matches!(s.etat_cible.as_str(), "ouverte" | "close");
        sqlx::query(
            "INSERT INTO ent_sortie
                 (id, etablissement_id, annee_id, reference, titre, nature, forme, destination, debut, fin, porteur_personne_id,
                  etat, etat_retour, besoin_car, liste_confirmee_le, liste_confirmee_par_user_id, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'france',$8,$9,$10,$11,$12,$13,
                     CASE WHEN $14 THEN now() END, CASE WHEN $14 THEN $15 END, $15)",
        )
        .bind(sortie)
        .bind(a.etablissement_id)
        .bind(annee)
        .bind(reference(s.debut, sortie))
        .bind(&s.titre)
        .bind(&s.nature)
        .bind(&s.forme)
        .bind(s.debut)
        .bind(s.fin)
        .bind(porteur)
        .bind(&s.etat_cible)
        .bind(&s.etat_retour_cible)
        .bind(s.besoin_car)
        .bind(confirmee)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let etat_participant = if s.etat_cible == "close" { "rentre" } else { "inscrit" };
        sqlx::query(
            "INSERT INTO ent_sortie_participant (id, etablissement_id, sortie_id, eleve_id, etat, cree_par_user_id)
             SELECT gen_random_uuid(), etablissement_id, $3, eleve_id, $4, $5
               FROM ent_sortie_reprise_participant
              WHERE etablissement_id = $1 AND sejour_id = $2 AND statut = 'rapproche'",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(sortie)
        .bind(etat_participant)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_sortie_reprise_sejour
                SET proprietaire = 'scolia', sortie_id = $3, basculee_le = now(), basculee_par_user_id = $4, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .bind(sortie)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            sortie,
            Fait { etat_apres: Some(&s.etat_cible), motif: Some("repris de Next.js"), ..Fait::simple("sortie_reprise") },
        )
        .await?;
        let ev = publier(&mut t, a, "voyages.reprise_basculee", sortie, json!({ "etat": s.etat_cible })).await?;
        journal_reprise(&mut t, a, "reprise_basculee", "Séjour basculé de Next.js vers ScolIA").await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(sortie)
    }

    /// Retour arrière d'un séjour **sans écriture métier** dans ScolIA : Next.js redevient propriétaire et la
    /// sortie créée est annulée (jamais supprimée : son journal est en ajout seul). Refusé si la sortie a déjà reçu des écritures (la ressaisie se fait alors
    /// séjour par séjour, §32.4).
    pub async fn voyages_reprise_retour_arriere(&self, a: &Acteur<'_>, sejour: Uuid) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesReprise).await?;
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a.etablissement_id, sejour, true).await?;
        let Some(sortie) = s.sortie_id else {
            return Err(conflit());
        };
        // Écritures métier : tout ce qui n'est pas le journal de reprise lui-même.
        let ecritures: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM ent_sortie_journal WHERE etablissement_id = $1 AND sortie_id = $2 AND action <> 'sortie_reprise')
                  + (SELECT count(*) FROM ent_sortie_participant_fait pf JOIN ent_sortie_participant sp
                         ON sp.etablissement_id = pf.etablissement_id AND sp.id = pf.participant_id
                      WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2)
                  + (SELECT count(*) FROM ent_sortie_budget_version WHERE etablissement_id = $1 AND sortie_id = $2)
                  + (SELECT count(*) FROM ent_sortie_devis WHERE etablissement_id = $1 AND sortie_id = $2)
                  + (SELECT count(*) FROM ent_sortie_repas_commande WHERE etablissement_id = $1 AND sortie_id = $2)
                  + (SELECT count(*) FROM ent_sortie_journal_voyage WHERE etablissement_id = $1 AND sortie_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(sortie)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if ecritures > 0 {
            return Err(refus(
                RefusMetier::Conflit,
                "Des écritures ont été faites dans ScolIA sur ce séjour : retour arrière séjour par séjour, avec ressaisie.",
            ));
        }
        // La contrainte de bascule impose de lever la propriété AVANT de retirer la sortie.
        sqlx::query(
            "UPDATE ent_sortie_reprise_sejour
                SET proprietaire = 'nextjs', sortie_id = NULL, basculee_le = NULL, basculee_par_user_id = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(sejour)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        // Le journal de la sortie est en ajout seul : la sortie créée par la bascule est ANNULÉE (motif tracé),
        // jamais supprimée ; Next.js redevient propriétaire et une nouvelle bascule créera une nouvelle sortie.
        // F-g : « une seule fonction » aligne l'attendu de l'internat sur l'état de la sortie, ici aussi (verrou de la sortie
        // d'abord, même ordre que partout).
        crate::voyages_argent_repo::verrou_sortie(&mut t, sortie).await?;
        sqlx::query(
            "UPDATE ent_sortie SET etat = 'annulee', etat_retour = NULL, version = version + 1, modifie_le = now(),
                    motif_decision = 'Retour arrière de la reprise : Next.js redevient propriétaire'
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(sortie)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        crate::voyages_argent_v3::internat_resynchroniser(&mut t, a, sortie, chrono::Utc::now()).await?;
        journaliser(&mut t, a, sortie, Fait { etat_apres: Some("annulee"), ..Fait::simple("retour_arriere_reprise") }).await?;
        journal_reprise(&mut t, a, "reprise_retour_arriere", "Retour arrière : Next.js redevient propriétaire").await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }
}

async fn ligne(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<LigneBascule, EntDbError> {
    let s = charger(conn, etab, id, false).await?;
    let (total, a_rapprocher): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE statut <> 'exclu'), count(*) FILTER (WHERE statut = 'a_rapprocher')
           FROM ent_sortie_reprise_participant WHERE etablissement_id = $1 AND sejour_id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let non_traites: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_reprise_ecart WHERE etablissement_id = $1 AND sejour_id = $2 AND traite_le IS NULL",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let (titre, debut, fin) = (s.titre.clone(), s.debut, s.fin);
    let c = controles(&FaitsBascule {
        proprietaire_scolia: s.proprietaire == "scolia",
        regime: s.regime.clone(),
        participants_a_rapprocher: a_rapprocher,
        porteur_designe: s.porteur_personne_id.is_some(),
        ecarts_non_traites: non_traites,
        pieces_attendues: s.pieces_attendues,
        pieces_copiees: s.pieces_copiees,
        etat_cible: s.etat_cible.clone(),
    });
    let derogation = s.derogation_motif.is_some();
    let ok = basculable(&c, derogation);
    // Autorisations papier attestées (lot V1, VOY-260) : suivies une fois la sortie ScolIA créée par la bascule.
    let attestations: Option<i64> = match s.sortie_id {
        None => None,
        Some(sortie) => Some(
            sqlx::query_scalar(
                "SELECT count(*) FROM ent_sortie_exigence WHERE etablissement_id = $1 AND sortie_id = $2
                    AND code = 'autorisation_participation' AND source = 'attestation_reprise'",
            )
            .bind(etab)
            .bind(sortie)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?,
        ),
    };
    Ok(LigneBascule {
        id,
        source_id: s.source_id,
        titre,
        debut,
        fin,
        proprietaire: s.proprietaire,
        regime: s.regime,
        statut_source: s.statut_source,
        etat_cible: s.etat_cible,
        participants_total: total,
        participants_a_rapprocher: a_rapprocher,
        ecarts_non_traites: non_traites,
        pieces_attendues: s.pieces_attendues,
        pieces_copiees: s.pieces_copiees,
        budget_total_centimes: s.budget_total_centimes,
        part_annoncee_centimes: s.part_annoncee_centimes,
        derogation,
        controles: c,
        basculable: ok,
        attestations,
        versements: None,
        sortie_id: s.sortie_id,
    })
}

impl PgEnt {
    /// Séjours repris dont ScolIA est propriétaire : lecture seule de leur état d'origine (aide à la recette).
    pub async fn voyages_reprise_proprietaire(&self, etab: Uuid, source_id: &str) -> Result<Option<String>, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = sqlx::query_scalar(
            "SELECT proprietaire FROM ent_sortie_reprise_sejour WHERE etablissement_id = $1 AND source_id = $2",
        )
        .bind(etab)
        .bind(source_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(p)
    }
}
