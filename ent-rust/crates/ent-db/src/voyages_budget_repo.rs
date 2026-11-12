//! Socle 42, lot V0b : budget d'une sortie — **commandes et lectures**
//! (`docs/socles/42_SORTIES_VOYAGES.md` §12.1, §19.9 ; R-SV1, R-SV15 à R-SV17, R-SV27 à R-SV29).
//!
//! * **versions numérotées**, en centimes, une seule en brouillon à la fois ; la comptabilité valide
//!   **une version** (celle qu'elle a lue : numéro et compteur d'écritures), qui est ensuite figée
//!   par la base elle-même (déclencheurs) ; une modification crée la version n+1, motivée ;
//! * **budget équilibré** pour être validé : sinon refus, avec les causes du déficit ;
//! * **prix annoncé figé** à l'autorisation, en ajout seul : une baisse suit la version validée, une
//!   hausse exige une décision motivée de la direction (et un nouvel avis aux familles, proposé) ;
//! * **PDF côté serveur** (fiche budget, circulaire), déposés par le `StorageProvider`
//!   d'`ent-object` sous une clé opaque, relus avec leur empreinte, jamais d'URL publique ;
//! * **synthèse APEL** annuelle, corrigée (états retenus, année du socle 5, montants agrégés) ;
//! * événements `voyages.*` **sans identité d'élève**.

use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::MetierEvent;
use ent_object::ObjectStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::voyages::{champ_csv, est_modifiable};
use crate::voyages_budget::{
    calculer, comparer_prix, euros, libelle_poste, marge_prereglee, montant_en_centimes,
    poste_valide, Calcul, EcartPrix, LigneCalc, AFFECTATIONS, POSTES_DEPENSE, POSTES_RECETTE,
};
use crate::voyages_pdf::{rendre, Bloc};
use crate::voyages_repo::{
    charger_avec_acces, interdit, journaliser, publier, refus, texte, texte_opt, tx, Acces, Fait,
    SortieLigne,
};

const LIGNES_MAX: i64 = 100;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BudgetLigneVue {
    pub id: Uuid,
    pub sens: String,
    pub poste: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub source: String,
    pub motif: Option<String>,
    pub affectation: Option<String>,
    pub rang: i32,
}

#[derive(Debug, Clone, FromRow)]
struct VersionRow {
    id: Uuid,
    numero: i32,
    etat: String,
    rev: i32,
    nb_payants: i32,
    marge_pct: i32,
    marge_motif: Option<String>,
    motif_revision: Option<String>,
    part_familles_centimes: Option<i64>,
    cree_le: DateTime<Utc>,
    valide_le: Option<DateTime<Utc>>,
}

const COLS_VERSION: &str = "id, numero, etat, rev, nb_payants, marge_pct, marge_motif, motif_revision, \
     part_familles_centimes, cree_le, valide_le";

#[derive(Debug, Clone, Serialize)]
pub struct BudgetVersionVue {
    pub numero: i32,
    pub etat: String,
    /// Compteur d'écritures : à renvoyer avec chaque écriture et avec la validation.
    pub rev: i32,
    pub nb_payants: i32,
    pub marge_pct: i32,
    pub marge_motif: Option<String>,
    pub motif_revision: Option<String>,
    /// Résultat scellé à la validation (`None` pour un brouillon).
    pub part_familles_centimes: Option<i64>,
    pub cree_le: DateTime<Utc>,
    pub valide_le: Option<DateTime<Utc>>,
    pub lignes: Vec<BudgetLigneVue>,
    pub calcul: Calcul,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PrixFigeVue {
    pub rang: i32,
    pub montant_centimes: i64,
    pub cause: String,
    pub motif: Option<String>,
    pub fige_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionResume {
    pub numero: i32,
    pub etat: String,
    pub valide_le: Option<DateTime<Utc>>,
    pub part_familles_centimes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BudgetVue {
    pub sortie_id: Uuid,
    /// Faux pour une sortie sans voyage ni car : il n'y a pas de budget à valider.
    pub a_budget: bool,
    pub obligatoire: bool,
    pub marge_prereglee: i32,
    pub peut_ecrire: bool,
    pub peut_valider: bool,
    pub peut_decider_hausse: bool,
    pub brouillon: Option<BudgetVersionVue>,
    pub validee: Option<BudgetVersionVue>,
    pub historique: Vec<VersionResume>,
    pub prix_fige: Vec<PrixFigeVue>,
    pub prix_courant_centimes: Option<i64>,
    /// La version validée dépasse le prix annoncé : décision de la direction attendue (R-SV17).
    pub hausse_en_attente: bool,
}

/// Saisie d'une ligne : le montant est **un texte** au format français, jamais un nombre venu du
/// navigateur (R-SV16).
#[derive(Debug, Clone, Deserialize)]
pub struct LigneSaisie {
    pub sens: String,
    pub poste: String,
    pub libelle: String,
    pub montant: String,
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub affectation: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ParamsBudget {
    #[serde(default)]
    pub nb_payants: Option<i32>,
    #[serde(default)]
    pub marge_pct: Option<i32>,
    #[serde(default)]
    pub marge_motif: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DocumentVue {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_: String,
    pub nom: String,
    pub taille: i32,
    pub sha256: String,
    pub emis_le: DateTime<Utc>,
    pub version_budget: Option<i32>,
    /// Le plus récent de son type : il « remplace » les précédents.
    pub actuel: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct LigneApel {
    pub sortie_id: Uuid,
    pub reference: String,
    pub titre: String,
    pub debut: DateTime<Utc>,
    /// Jour de départ en heure de Paris (JJ/MM/AAAA, R-SV29).
    pub jour: String,
    pub etat: String,
    pub apel_centimes: i64,
    pub aides_centimes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyntheseApel {
    pub annee_id: Uuid,
    pub annee_libelle: String,
    pub lignes: Vec<LigneApel>,
    pub total_apel_centimes: i64,
    pub total_aides_centimes: i64,
    pub total_centimes: i64,
}

// ───────────────────────── Droits ─────────────────────────

/// Lecture : le porteur, la comptabilité, la direction, le secrétariat. Un encadrant, un professeur
/// non porteur, l'accueil et les familles n'ont pas accès au budget (R-SV18, §21.4).
fn peut_lire_budget(acces: &Acces) -> bool {
    acces.porteur
        || acces.a(Action::VoyagesBudgetValider)
        || acces.a(Action::VoyagesValider)
        || acces.a(Action::VoyagesAutoriser)
        || acces.a(Action::VoyagesModifier)
}

/// Émission des PDF de la fiche budget : comptabilité et direction (VOY-219).
fn peut_emettre_budget(acces: &Acces) -> bool {
    acces.a(Action::VoyagesBudgetValider)
        || acces.a(Action::VoyagesValider)
        || acces.a(Action::VoyagesAutoriser)
}

/// Étapes où le porteur (ou le secrétariat) prépare encore le budget prévisionnel.
fn etape_de_preparation(etat: &str) -> bool {
    matches!(etat, "brouillon" | "a_completer" | "preparation" | "a_signer")
}

/// Écriture du budget : la comptabilité tant que le dossier est modifiable ; le porteur et ceux qui
/// gèrent la sortie pendant la préparation seulement (ensuite le budget est entre les mains de la
/// comptabilité). Sans budget à valider (ni voyage ni car), aucune écriture.
fn garde_ecriture(s: &SortieLigne, acces: &Acces) -> Result<(), EntDbError> {
    let compta = acces.a(Action::VoyagesBudgetValider);
    if !(compta || acces.gere(Action::VoyagesModifier)) {
        return Err(interdit());
    }
    if !s.contexte().a_budget() {
        return Err(refus(
            RefusMetier::Conflit,
            "Cette sortie n'a pas de budget à valider (ni voyage ni car).",
        ));
    }
    if !est_modifiable(&s.etat) {
        return Err(refus(
            RefusMetier::Conflit,
            "Le dossier n'est plus modifiable : le budget ne peut plus changer.",
        ));
    }
    if !compta && !etape_de_preparation(&s.etat) {
        return Err(refus(
            RefusMetier::Conflit,
            "Le budget est à l'étape de la comptabilité : demandez-lui la modification.",
        ));
    }
    Ok(())
}

// ───────────────────────── Lectures internes ─────────────────────────

async fn versions(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<Vec<VersionRow>, EntDbError> {
    sqlx::query_as::<_, VersionRow>(&format!(
        "SELECT {COLS_VERSION} FROM ent_sortie_budget_version
          WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY numero"
    ))
    .bind(etab)
    .bind(sortie)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn lignes_de(
    conn: &mut PgConnection,
    etab: Uuid,
    version: Uuid,
) -> Result<Vec<BudgetLigneVue>, EntDbError> {
    sqlx::query_as::<_, BudgetLigneVue>(
        "SELECT id, sens, poste, libelle, montant_centimes, source, motif, affectation, rang
           FROM ent_sortie_budget_ligne
          WHERE etablissement_id = $1 AND version_id = $2
          ORDER BY sens, rang, id",
    )
    .bind(etab)
    .bind(version)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn prix_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<Vec<PrixFigeVue>, EntDbError> {
    sqlx::query_as::<_, PrixFigeVue>(
        "SELECT rang, montant_centimes, cause, motif, fige_le FROM ent_sortie_prix_fige
          WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY rang",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(conn)
    .await
    .map_err(db)
}

fn calcul_de(lignes: &[BudgetLigneVue], v: &VersionRow, obligatoire: bool) -> Calcul {
    let l: Vec<LigneCalc<'_>> = lignes
        .iter()
        .map(|x| LigneCalc {
            sens: &x.sens,
            poste: &x.poste,
            montant_centimes: x.montant_centimes,
            affectation: x.affectation.as_deref(),
        })
        .collect();
    calculer(&l, v.nb_payants as i64, v.marge_pct as i64, obligatoire)
}

async fn vue_version(
    conn: &mut PgConnection,
    etab: Uuid,
    v: &VersionRow,
    obligatoire: bool,
) -> Result<BudgetVersionVue, EntDbError> {
    let lignes = lignes_de(conn, etab, v.id).await?;
    let calcul = calcul_de(&lignes, v, obligatoire);
    Ok(BudgetVersionVue {
        numero: v.numero,
        etat: v.etat.clone(),
        rev: v.rev,
        nb_payants: v.nb_payants,
        marge_pct: v.marge_pct,
        marge_motif: v.marge_motif.clone(),
        motif_revision: v.motif_revision.clone(),
        part_familles_centimes: v.part_familles_centimes,
        cree_le: v.cree_le,
        valide_le: v.valide_le,
        lignes,
        calcul,
    })
}

async fn assembler(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    acces: &Acces,
) -> Result<BudgetVue, EntDbError> {
    let rows = versions(conn, a.etablissement_id, s.id).await?;
    let obligatoire = s.nature == "obligatoire";
    let mut brouillon = None;
    let mut validee = None;
    if let Some(v) = rows.iter().find(|v| v.etat == "brouillon") {
        brouillon = Some(vue_version(conn, a.etablissement_id, v, obligatoire).await?);
    }
    if let Some(v) = rows.iter().find(|v| v.etat == "validee") {
        validee = Some(vue_version(conn, a.etablissement_id, v, obligatoire).await?);
    }
    let prix = prix_de(conn, a.etablissement_id, s.id).await?;
    let courant = prix.last().map(|p| p.montant_centimes);
    let hausse = match (&validee, courant) {
        (Some(v), Some(c)) => v.part_familles_centimes.unwrap_or(0) > c,
        _ => false,
    };
    Ok(BudgetVue {
        sortie_id: s.id,
        a_budget: s.contexte().a_budget(),
        obligatoire,
        marge_prereglee: marge_prereglee(&s.forme, &s.destination),
        peut_ecrire: garde_ecriture(s, acces).is_ok(),
        peut_valider: acces.a(Action::VoyagesBudgetValider),
        peut_decider_hausse: acces.a(Action::VoyagesAutoriser),
        brouillon,
        validee,
        historique: rows
            .iter()
            .map(|v| VersionResume {
                numero: v.numero,
                etat: v.etat.clone(),
                valide_le: v.valide_le,
                part_familles_centimes: v.part_familles_centimes,
            })
            .collect(),
        prix_fige: prix,
        prix_courant_centimes: courant,
        hausse_en_attente: hausse,
    })
}

/// Le brouillon désigné, verrouillé, à la version d'écritures lue par l'appelant.
async fn brouillon_verrouille(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    numero: i32,
    rev: i32,
) -> Result<VersionRow, EntDbError> {
    let v = sqlx::query_as::<_, VersionRow>(&format!(
        "SELECT {COLS_VERSION} FROM ent_sortie_budget_version
          WHERE etablissement_id = $1 AND sortie_id = $2 AND numero = $3 FOR UPDATE"
    ))
    .bind(etab)
    .bind(sortie)
    .bind(numero)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    if v.etat != "brouillon" {
        return Err(refus(
            RefusMetier::Conflit,
            "Cette version est validée : elle est figée. Créez la version suivante pour la modifier.",
        ));
    }
    if v.rev != rev {
        return Err(refus(
            RefusMetier::Conflit,
            "Le budget a été modifié entre-temps : rechargez-le avant de recommencer.",
        ));
    }
    Ok(v)
}

async fn bump(conn: &mut PgConnection, etab: Uuid, version: Uuid) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_sortie_budget_version SET rev = rev + 1, modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(version)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

// ───────────────────────── Lecture publique ─────────────────────────

impl PgEnt {
    /// VOY-210 : vue complète du budget (versions, lignes, calcul, prix annoncé).
    pub async fn voyage_budget(&self, a: &Acteur<'_>, id: Uuid) -> Result<BudgetVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_lire_budget(&acces) {
            return Err(interdit());
        }
        let v = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }
}

// ───────────────────────── Écritures : versions et lignes ─────────────────────────

struct LigneValide {
    sens: String,
    poste: String,
    libelle: String,
    montant: i64,
    motif: String,
    affectation: Option<String>,
}

fn valider_ligne(l: &LigneSaisie) -> Result<LigneValide, EntDbError> {
    if !poste_valide(&l.sens, &l.poste) {
        return Err(refus(RefusMetier::Invalide, "Poste inconnu pour ce type de ligne."));
    }
    let libelle = texte(&l.libelle, 200)?;
    let montant = montant_en_centimes(&l.montant).map_err(|m| refus(RefusMetier::Invalide, m))?;
    let motif = texte_opt(l.motif.as_deref(), 1000)?.ok_or_else(|| {
        refus(
            RefusMetier::Invalide,
            "Un motif est obligatoire : d'où vient ce montant (devis, estimation, décision) ?",
        )
    })?;
    let affectation = if l.sens == "depense" {
        if l.affectation.is_some() {
            return Err(refus(
                RefusMetier::Invalide,
                "L'affectation ne concerne que les recettes.",
            ));
        }
        None
    } else {
        match l.affectation.as_deref() {
            Some(x) if AFFECTATIONS.contains(&x) => Some(x.to_string()),
            Some(_) => {
                return Err(refus(RefusMetier::Invalide, "Affectation inconnue."));
            }
            None if l.poste == "don_affecte" => {
                return Err(refus(
                    RefusMetier::Invalide,
                    "Un don affecté déclare son affectation (encadrement ou général).",
                ));
            }
            None => Some("general".to_string()),
        }
    };
    Ok(LigneValide {
        sens: l.sens.clone(),
        poste: l.poste.clone(),
        libelle,
        montant,
        motif,
        affectation,
    })
}

fn resume_ligne(l: &LigneValide) -> String {
    format!("{} : {}", l.libelle, euros(l.montant))
}

impl PgEnt {
    /// VOY-210, VOY-215 : crée la **version suivante** du budget (brouillon). La première reprend
    /// l'effectif déclaré et la marge préréglée du contexte ; une révision recopie la version
    /// validée et exige un **motif** (« une modification crée une version n+1, motivée »).
    pub async fn voyage_budget_version_creer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: Option<&str>,
    ) -> Result<BudgetVue, EntDbError> {
        let motif = texte_opt(motif, 1000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_ecriture(&s, &acces)?;
        let rows = versions(&mut t, a.etablissement_id, id).await?;
        if rows.iter().any(|v| v.etat == "brouillon") {
            return Err(refus(
                RefusMetier::Conflit,
                "Une version est déjà en cours d'édition : terminez-la ou abandonnez-la.",
            ));
        }
        let validee = rows.iter().find(|v| v.etat == "validee");
        if validee.is_some() && motif.is_none() {
            return Err(refus(
                RefusMetier::Invalide,
                "Une révision du budget est motivée : indiquez pourquoi il change.",
            ));
        }
        let numero = rows.iter().map(|v| v.numero).max().unwrap_or(0) + 1;
        let (nb, marge) = match validee {
            Some(v) => (v.nb_payants, v.marge_pct),
            None => (
                s.effectif_eleves_declare.or(s.places).unwrap_or(0).clamp(0, 2000),
                marge_prereglee(&s.forme, &s.destination),
            ),
        };
        let vid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_budget_version
                 (id, etablissement_id, sortie_id, numero, nb_payants, marge_pct, marge_motif,
                  motif_revision, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(vid)
        .bind(a.etablissement_id)
        .bind(id)
        .bind(numero)
        .bind(nb)
        .bind(marge)
        .bind(validee.and_then(|v| v.marge_motif.clone()))
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        if let Some(v) = validee {
            sqlx::query(
                "INSERT INTO ent_sortie_budget_ligne
                     (id, etablissement_id, version_id, sens, poste, libelle, montant_centimes,
                      source, motif, affectation, rang, cree_par_user_id)
                 SELECT gen_random_uuid(), etablissement_id, $3, sens, poste, libelle, montant_centimes,
                        source, motif, affectation, rang, $4
                   FROM ent_sortie_budget_ligne
                  WHERE etablissement_id = $1 AND version_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(v.id)
            .bind(vid)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some("budget_version"),
                apres: Some(numero.to_string()),
                motif: motif.as_deref(),
                ..Fait::simple("budget_version_creee")
            },
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "voyages.budget_version",
            id,
            json!({ "numero": numero, "revision": validee.is_some() }),
        )
        .await?;
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(vue)
    }

    /// Abandonne un brouillon (une révision qu'on ne veut plus) : la version validée reste.
    pub async fn voyage_budget_version_abandonner(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        numero: i32,
        rev: i32,
    ) -> Result<BudgetVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_ecriture(&s, &acces)?;
        let v = brouillon_verrouille(&mut t, a.etablissement_id, id, numero, rev).await?;
        sqlx::query("DELETE FROM ent_sortie_budget_ligne WHERE etablissement_id = $1 AND version_id = $2")
            .bind(a.etablissement_id)
            .bind(v.id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM ent_sortie_budget_version WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(v.id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some("budget_version"),
                avant: Some(numero.to_string()),
                ..Fait::simple("budget_version_abandonnee")
            },
        )
        .await?;
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// NX-53 : effectif payant et marge de sécurité. Une marge différente du préréglage du contexte
    /// (0 / 5 / 7 / 10 %) demande un **motif**.
    pub async fn voyage_budget_parametrer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        numero: i32,
        rev: i32,
        p: ParamsBudget,
    ) -> Result<BudgetVue, EntDbError> {
        let marge_motif = texte_opt(p.marge_motif.as_deref(), 1000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_ecriture(&s, &acces)?;
        let v = brouillon_verrouille(&mut t, a.etablissement_id, id, numero, rev).await?;
        let nb = p.nb_payants.unwrap_or(v.nb_payants);
        let marge = p.marge_pct.unwrap_or(v.marge_pct);
        if !(0..=2000).contains(&nb) || !(0..=30).contains(&marge) {
            return Err(refus(
                RefusMetier::Invalide,
                "Effectif payant (0 à 2000) ou marge (0 à 30 %) hors limites.",
            ));
        }
        let prereglee = marge_prereglee(&s.forme, &s.destination);
        let motif_marge = if marge == prereglee {
            None
        } else {
            Some(
                marge_motif
                    .or_else(|| v.marge_motif.clone().filter(|_| marge == v.marge_pct))
                    .ok_or_else(|| {
                        refus(
                            RefusMetier::Invalide,
                            "Une marge différente du préréglage du séjour demande un motif.",
                        )
                    })?,
            )
        };
        sqlx::query(
            "UPDATE ent_sortie_budget_version
                SET nb_payants = $3, marge_pct = $4, marge_motif = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(v.id)
        .bind(nb)
        .bind(marge)
        .bind(&motif_marge)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        bump(&mut t, a.etablissement_id, v.id).await?;
        for (champ, avant, apres) in [
            ("nb_payants", v.nb_payants, nb),
            ("marge_pct", v.marge_pct, marge),
        ] {
            if avant != apres {
                journaliser(
                    &mut t,
                    a,
                    id,
                    Fait {
                        champ: Some(champ),
                        avant: Some(avant.to_string()),
                        apres: Some(apres.to_string()),
                        motif: motif_marge.as_deref().filter(|_| champ == "marge_pct"),
                        ..Fait::simple("budget_parametre")
                    },
                )
                .await?;
            }
        }
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// NX-51 : ajoute une dépense ou une recette (saisie motivée) au brouillon.
    pub async fn voyage_budget_ligne_ajouter(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        numero: i32,
        rev: i32,
        saisie: LigneSaisie,
    ) -> Result<BudgetVue, EntDbError> {
        let l = valider_ligne(&saisie)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_ecriture(&s, &acces)?;
        let v = brouillon_verrouille(&mut t, a.etablissement_id, id, numero, rev).await?;
        let (n, rang): (i64, i32) = sqlx::query_as(
            "SELECT count(*), coalesce(max(rang), 0)::int FROM ent_sortie_budget_ligne
              WHERE etablissement_id = $1 AND version_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(v.id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if n >= LIGNES_MAX {
            return Err(refus(
                RefusMetier::Conflit,
                "Cent lignes au plus par version de budget : regroupez-en.",
            ));
        }
        sqlx::query(
            "INSERT INTO ent_sortie_budget_ligne
                 (id, etablissement_id, version_id, sens, poste, libelle, montant_centimes, motif,
                  affectation, rang, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(v.id)
        .bind(&l.sens)
        .bind(&l.poste)
        .bind(&l.libelle)
        .bind(l.montant)
        .bind(&l.motif)
        .bind(&l.affectation)
        .bind(rang + 1)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        bump(&mut t, a.etablissement_id, v.id).await?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some(l.poste.as_str()),
                apres: Some(resume_ligne(&l)),
                motif: Some(l.motif.as_str()),
                ..Fait::simple("budget_ligne_ajoutee")
            },
        )
        .await?;
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// NX-51 : modifie une ligne du brouillon (le sens d'une ligne ne change pas).
    pub async fn voyage_budget_ligne_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        numero: i32,
        rev: i32,
        ligne_id: Uuid,
        saisie: LigneSaisie,
    ) -> Result<BudgetVue, EntDbError> {
        let l = valider_ligne(&saisie)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_ecriture(&s, &acces)?;
        let v = brouillon_verrouille(&mut t, a.etablissement_id, id, numero, rev).await?;
        let avant: Option<(String, String, i64)> = sqlx::query_as(
            "SELECT sens, libelle, montant_centimes FROM ent_sortie_budget_ligne
              WHERE etablissement_id = $1 AND version_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(v.id)
        .bind(ligne_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (sens_avant, libelle_avant, montant_avant) = avant.ok_or_else(introuvable)?;
        if sens_avant != l.sens {
            return Err(refus(
                RefusMetier::Invalide,
                "Une dépense ne devient pas une recette : supprimez la ligne et recréez-la.",
            ));
        }
        sqlx::query(
            "UPDATE ent_sortie_budget_ligne
                SET poste = $4, libelle = $5, montant_centimes = $6, motif = $7, affectation = $8
              WHERE etablissement_id = $1 AND version_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(v.id)
        .bind(ligne_id)
        .bind(&l.poste)
        .bind(&l.libelle)
        .bind(l.montant)
        .bind(&l.motif)
        .bind(&l.affectation)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        bump(&mut t, a.etablissement_id, v.id).await?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some(l.poste.as_str()),
                avant: Some(format!("{libelle_avant} : {}", euros(montant_avant))),
                apres: Some(resume_ligne(&l)),
                motif: Some(l.motif.as_str()),
                ..Fait::simple("budget_ligne_modifiee")
            },
        )
        .await?;
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// NX-51 : supprime une ligne du brouillon. (L'exclusion durable d'une ligne issue d'un devis,
    /// pour qu'une synchronisation ne la recrée pas, arrive avec les devis au lot V0c.)
    pub async fn voyage_budget_ligne_supprimer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        numero: i32,
        rev: i32,
        ligne_id: Uuid,
    ) -> Result<BudgetVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_ecriture(&s, &acces)?;
        let v = brouillon_verrouille(&mut t, a.etablissement_id, id, numero, rev).await?;
        let supprimee: Option<(String, String, i64)> = sqlx::query_as(
            "DELETE FROM ent_sortie_budget_ligne
              WHERE etablissement_id = $1 AND version_id = $2 AND id = $3
              RETURNING poste, libelle, montant_centimes",
        )
        .bind(a.etablissement_id)
        .bind(v.id)
        .bind(ligne_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (poste, libelle, montant) = supprimee.ok_or_else(introuvable)?;
        bump(&mut t, a.etablissement_id, v.id).await?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some(poste.as_str()),
                avant: Some(format!("{libelle} : {}", euros(montant))),
                ..Fait::simple("budget_ligne_supprimee")
            },
        )
        .await?;
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }
}

// ───────────────────────── Validation et prix figé ─────────────────────────

/// Valide le brouillon `numero` lu à la version d'écritures `rev` : il doit être **équilibré**.
/// L'ancienne version validée devient « remplacée » ; la nouvelle est scellée (part, totaux).
async fn valider_brouillon(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    numero: i32,
    rev: i32,
) -> Result<(VersionRow, MetierEvent), EntDbError> {
    let v = brouillon_verrouille(conn, a.etablissement_id, s.id, numero, rev).await?;
    let lignes = lignes_de(conn, a.etablissement_id, v.id).await?;
    let c = calcul_de(&lignes, &v, s.nature == "obligatoire");
    if !c.equilibre {
        let detail = if c.causes.is_empty() {
            "Le budget n'est pas équilibré.".to_string()
        } else {
            c.causes.join(" ; ")
        };
        return Err(EntDbError::ConflitDossierDetail(detail));
    }
    let part = c.part_familles.unwrap_or(0);
    sqlx::query(
        "UPDATE ent_sortie_budget_version SET etat = 'remplacee'
          WHERE etablissement_id = $1 AND sortie_id = $2 AND etat = 'validee'",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_sortie_budget_version
            SET etat = 'validee', part_familles_centimes = $3, total_depenses_centimes = $4,
                total_recettes_centimes = $5, valide_le = now(), valide_par_user_id = $6,
                rev = rev + 1, modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(v.id)
    .bind(part)
    .bind(c.depenses_total)
    .bind(c.recettes_total)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journaliser(
        conn,
        a,
        s.id,
        Fait {
            champ: Some("budget_version"),
            apres: Some(format!("{numero} : part des familles {}", euros(part))),
            motif: v.motif_revision.as_deref(),
            ..Fait::simple("budget_version_validee")
        },
    )
    .await?;
    let ev = publier(
        conn,
        a,
        "voyages.budget_version_validee",
        s.id,
        json!({ "numero": numero, "part_familles_centimes": part }),
    )
    .await?;
    let mut v = v;
    v.part_familles_centimes = Some(part);
    Ok((v, ev))
}

/// Commande `valider-budget` : on valide **la version lue**. Sans brouillon, la version validée
/// peut être reconfirmée (dossier rouvert à l'étape du budget, rien n'a changé).
pub(crate) async fn valider_pour_commande(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    numero: Option<i32>,
    rev: Option<i32>,
) -> Result<Option<MetierEvent>, EntDbError> {
    let (Some(numero), Some(rev)) = (numero, rev) else {
        return Err(refus(
            RefusMetier::Invalide,
            "Une décision porte sur la version du budget que vous avez lue : rechargez-la.",
        ));
    };
    let rows = versions(conn, a.etablissement_id, s.id).await?;
    if let Some(b) = rows.iter().find(|v| v.etat == "brouillon") {
        if b.numero != numero {
            return Err(refus(
                RefusMetier::Conflit,
                "Une version plus récente du budget existe : rechargez-la.",
            ));
        }
        let (_, ev) = valider_brouillon(conn, a, s, numero, rev).await?;
        return Ok(Some(ev));
    }
    match rows.iter().find(|v| v.etat == "validee") {
        Some(v) if v.numero == numero && v.rev == rev => Ok(None),
        Some(_) => Err(refus(
            RefusMetier::Conflit,
            "Le budget a été modifié entre-temps : rechargez-le avant de recommencer.",
        )),
        None => Err(refus(
            RefusMetier::Conflit,
            "Aucune version de budget à valider : la comptabilité doit d'abord en saisir une.",
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecisionPrix {
    Aucune,
    Fige,
    Baisse,
    HausseDecidee,
    HausseEnAttente,
}

async fn prix_courant(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<Option<(i32, i64)>, EntDbError> {
    sqlx::query_as(
        "SELECT rang, montant_centimes FROM ent_sortie_prix_fige
          WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY rang DESC LIMIT 1",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

async fn version_validee(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<Option<VersionRow>, EntDbError> {
    sqlx::query_as::<_, VersionRow>(&format!(
        "SELECT {COLS_VERSION} FROM ent_sortie_budget_version
          WHERE etablissement_id = $1 AND sortie_id = $2 AND etat = 'validee'"
    ))
    .bind(etab)
    .bind(sortie)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// Aligne le prix annoncé sur la version validée (NX-55, R-SV17) : le premier est figé à
/// l'autorisation ; une **baisse** suit toujours (avoirs à proposer, V2) ; une **hausse** n'est
/// posée que sur décision motivée de la direction, sinon elle reste « en attente ».
async fn aligner_prix(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    v: &VersionRow,
    decideur: bool,
    motif: Option<&str>,
) -> Result<(DecisionPrix, Option<MetierEvent>), EntDbError> {
    let part = v.part_familles_centimes.unwrap_or(0);
    let courant = prix_courant(conn, a.etablissement_id, s.id).await?;
    let (cause, decision) = match comparer_prix(part, courant.map(|c| c.1)) {
        EcartPrix::Identique => return Ok((DecisionPrix::Aucune, None)),
        EcartPrix::Premier => ("ouverture", DecisionPrix::Fige),
        EcartPrix::Baisse => ("baisse", DecisionPrix::Baisse),
        EcartPrix::Hausse if decideur => {
            if motif.is_none() {
                return Err(refus(
                    RefusMetier::Invalide,
                    "Augmenter le prix annoncé aux familles demande une décision motivée de la direction.",
                ));
            }
            ("hausse_decidee", DecisionPrix::HausseDecidee)
        }
        EcartPrix::Hausse => return Ok((DecisionPrix::HausseEnAttente, None)),
    };
    let rang = courant.map_or(1, |c| c.0 + 1);
    sqlx::query(
        "INSERT INTO ent_sortie_prix_fige
             (id, etablissement_id, sortie_id, rang, version_id, montant_centimes, cause, motif,
              fige_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(s.id)
    .bind(rang)
    .bind(v.id)
    .bind(part)
    .bind(cause)
    .bind(motif)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let (action, note) = match decision {
        DecisionPrix::Fige => ("prix_fige", None),
        DecisionPrix::Baisse => (
            "prix_baisse",
            Some("avoirs à proposer aux payeurs (socle 24, lot V2)"),
        ),
        _ => ("prix_hausse_decidee", Some("nouvel avis aux familles à proposer")),
    };
    journaliser(
        conn,
        a,
        s.id,
        Fait {
            champ: Some("prix_annonce"),
            avant: courant.map(|c| euros(c.1)),
            apres: Some(euros(part)),
            motif: motif.or(note),
            ..Fait::simple(action)
        },
    )
    .await?;
    let ev = publier(
        conn,
        a,
        "voyages.prix_fige",
        s.id,
        json!({ "montant_centimes": part, "cause": cause, "rang": rang }),
    )
    .await?;
    Ok((decision, Some(ev)))
}

/// À l'autorisation (NX-08, NX-55) : un budget validé est exigé ; le prix annoncé est figé.
pub(crate) async fn figer_a_l_autorisation(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    motif: Option<&str>,
) -> Result<Option<MetierEvent>, EntDbError> {
    let Some(v) = version_validee(conn, a.etablissement_id, s.id).await? else {
        return Err(EntDbError::ConflitDossierDetail(
            "Aucune version de budget validée : la comptabilité doit valider le budget avant l'autorisation."
                .to_string(),
        ));
    };
    Ok(aligner_prix(conn, a, s, &v, true, motif).await?.1)
}

impl PgEnt {
    /// VOY-215 : valide une **révision** (version n+1) d'un budget déjà validé, une fois le dossier
    /// à l'étape d'autorisation ou après. Une baisse du prix annoncé suit ; une hausse reste en
    /// attente de la décision de la direction ([`PgEnt::voyage_prix_hausser`]).
    pub async fn voyage_budget_valider_revision(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        numero: i32,
        rev: i32,
    ) -> Result<BudgetVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesBudgetValider) {
            return Err(interdit());
        }
        if !matches!(s.etat.as_str(), "a_autoriser" | "ouverte" | "prete") {
            return Err(refus(
                RefusMetier::Conflit,
                "Une révision se valide après le budget : à l'étape « budget à valider », utilisez « Valider le budget ».",
            ));
        }
        let (v, ev1) = valider_brouillon(&mut t, a, &s, numero, rev).await?;
        let mut evs = vec![ev1];
        if prix_courant(&mut t, a.etablissement_id, id).await?.is_some() {
            if let (_, Some(ev)) = aligner_prix(&mut t, a, &s, &v, false, None).await? {
                evs.push(ev);
            }
        }
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evs);
        Ok(vue)
    }

    /// NX-55, R-SV17 : **augmente** le prix annoncé jusqu'à la part de la version validée, sur
    /// décision motivée de la direction. Un nouvel avis aux familles est proposé (jamais envoyé seul).
    pub async fn voyage_prix_hausser(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: &str,
    ) -> Result<BudgetVue, EntDbError> {
        let motif = texte(motif, 1000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesAutoriser) {
            return Err(interdit());
        }
        if !matches!(s.etat.as_str(), "ouverte" | "prete") {
            return Err(refus(
                RefusMetier::Conflit,
                "Le prix annoncé n'est figé qu'à partir de l'autorisation de la sortie.",
            ));
        }
        let v = version_validee(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(conflit)?;
        let courant = prix_courant(&mut t, a.etablissement_id, id).await?;
        if comparer_prix(v.part_familles_centimes.unwrap_or(0), courant.map(|c| c.1))
            != EcartPrix::Hausse
        {
            return Err(refus(
                RefusMetier::Conflit,
                "La version validée ne dépasse pas le prix annoncé : aucune hausse à décider.",
            ));
        }
        let (_, ev) = aligner_prix(&mut t, a, &s, &v, true, Some(&motif)).await?;
        let vue = assembler(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        if let Some(ev) = ev {
            self.diffuser_apres_commit(&[ev]);
        }
        Ok(vue)
    }
}

// ───────────────────────── Documents PDF ─────────────────────────

/// Dates d'un document, **toutes en heure de Paris** (R-SV29), calculées par la base.
struct Dates {
    debut: String,
    fin: String,
    valide: Option<String>,
    prix: Vec<String>,
}

async fn jour_paris(conn: &mut PgConnection, t: DateTime<Utc>) -> Result<String, EntDbError> {
    sqlx::query_scalar("SELECT to_char($1::timestamptz AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY')")
        .bind(t)
        .fetch_one(conn)
        .await
        .map_err(db)
}

async fn dates_paris(
    conn: &mut PgConnection,
    s: &SortieLigne,
    valide: Option<DateTime<Utc>>,
    prix: &[PrixFigeVue],
) -> Result<Dates, EntDbError> {
    let (debut, fin): (String, String) = sqlx::query_as(
        "SELECT to_char($1::timestamptz AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY à HH24:MI'),
                to_char($2::timestamptz AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY à HH24:MI')",
    )
    .bind(s.debut)
    .bind(s.fin)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let valide = match valide {
        Some(t) => Some(jour_paris(&mut *conn, t).await?),
        None => None,
    };
    let mut jours = Vec::new();
    for p in prix {
        jours.push(jour_paris(&mut *conn, p.fige_le).await?);
    }
    Ok(Dates { debut, fin, valide, prix: jours })
}

fn ligne(gauche: impl Into<String>, droite: impl Into<String>, gras: bool) -> Bloc {
    Bloc::Ligne { gauche: gauche.into(), droite: droite.into(), gras }
}

fn blocs_fiche_budget(
    s: &SortieLigne,
    d: &Dates,
    v: &BudgetVersionVue,
    prix: &[PrixFigeVue],
) -> Vec<Bloc> {
    let c = &v.calcul;
    let mut b = vec![
        Bloc::Titre("Fiche budget".into()),
        Bloc::Paragraphe(format!("{} — réf. {}", s.titre, s.reference)),
        Bloc::Paragraphe(format!(
            "Du {} au {}{}",
            d.debut,
            d.fin,
            s.destination_libelle
                .as_deref()
                .map(|x| format!(" — {x}"))
                .unwrap_or_default()
        )),
        Bloc::Paragraphe(format!(
            "Sortie {} — {}",
            if s.nature == "obligatoire" { "obligatoire" } else { "facultative" },
            if s.forme == "voyage" { "voyage avec nuitée" } else { "sans nuitée" }
        )),
        Bloc::SousTitre(format!(
            "Version {} — validée{}",
            v.numero,
            d.valide.as_ref().map(|j| format!(" le {j}")).unwrap_or_default()
        )),
    ];
    if let Some(m) = &v.motif_revision {
        b.push(Bloc::Paragraphe(format!("Motif de la révision : {m}")));
    }
    for (sens, titre, postes, total, nom_total) in [
        ("depense", "Dépenses", &POSTES_DEPENSE[..], c.depenses_total, "Total des dépenses"),
        ("recette", "Recettes (hors participation des familles)", &POSTES_RECETTE[..], c.recettes_total, "Total des recettes"),
    ] {
        b.push(Bloc::SousTitre(titre.into()));
        let mut vide = true;
        for p in postes {
            for l in v.lignes.iter().filter(|l| l.sens == sens && l.poste == *p) {
                vide = false;
                let aff = if l.affectation.as_deref() == Some("encadrement") {
                    " (affectée à l'encadrement)"
                } else {
                    ""
                };
                b.push(ligne(
                    format!("{} — {}{aff}", libelle_poste(p), l.libelle),
                    euros(l.montant_centimes),
                    false,
                ));
            }
        }
        if vide {
            b.push(Bloc::Paragraphe("Aucune ligne.".into()));
        }
        b.push(ligne(nom_total, euros(total), true));
    }
    b.push(Bloc::SousTitre("Part des familles".into()));
    b.push(ligne(
        "Dépenses hors encadrement et hors surcoût handicap",
        euros(c.depenses_familles),
        false,
    ));
    b.push(ligne(
        "Recettes réduisant la part des familles",
        euros(c.recettes_reduisant_la_part),
        false,
    ));
    b.push(ligne(format!("Marge de sécurité ({} %)", c.marge_pct), euros(c.marge), false));
    b.push(ligne("Participants payants", c.nb_payants.to_string(), false));
    if let Some(cout) = c.cout_reel_par_eleve {
        b.push(ligne("Coût réel par élève", euros(cout), false));
    }
    b.push(ligne(
        "Part des familles par élève (arrondie à l'euro inférieur)",
        euros(c.part_familles.unwrap_or(0)),
        true,
    ));
    b.push(ligne("Participation totale attendue des familles", euros(c.total_familles), false));
    if c.arrondi_pris_en_charge > 0 {
        b.push(ligne(
            "Arrondi pris en charge par l'établissement",
            euros(c.arrondi_pris_en_charge),
            false,
        ));
    }
    if c.excedent > 0 {
        b.push(ligne("Excédent prévisionnel", euros(c.excedent), false));
    }
    if !prix.is_empty() {
        b.push(Bloc::SousTitre("Prix annoncé aux familles (figé)".into()));
        for (p, jour) in prix.iter().zip(&d.prix) {
            let cause = match p.cause.as_str() {
                "ouverture" => "figé à l'autorisation",
                "baisse" => "baisse",
                _ => "hausse décidée par la direction",
            };
            b.push(ligne(
                format!("{jour} — {cause}"),
                euros(p.montant_centimes),
                false,
            ));
        }
    }
    b.push(Bloc::Espace);
    b.push(Bloc::Paragraphe(if c.equilibre {
        "Budget équilibré (validé par la comptabilité).".into()
    } else {
        "Budget non équilibré.".into()
    }));
    b
}

/// Corps de la circulaire aux familles (texte), composé **côté serveur** depuis les faits du dossier
/// (VD-22, VD-35) : dates, lieu, rendez-vous, objectifs, programme, consignes, et la **participation
/// annoncée** quand elle est figée (R-SV8 : aucune participation pour une sortie obligatoire).
pub(crate) async fn composer_circulaire(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
) -> Result<(String, String), EntDbError> {
    let d = dates_paris(conn, s, None, &[]).await?;
    let mut corps = format!(
        "Madame, Monsieur,\n\nNous organisons la sortie « {} ».\n\n",
        s.titre
    );
    corps.push_str(&format!("Départ : {}\nRetour : {}\n", d.debut, d.fin));
    if let Some(x) = &s.destination_libelle {
        corps.push_str(&format!("Destination : {x}\n"));
    }
    if let Some(x) = &s.point_rendez_vous {
        corps.push_str(&format!("Rendez-vous : {x}\n"));
    }
    if let Some(x) = &s.objectifs {
        corps.push_str(&format!("\nObjectifs : {x}\n"));
    }
    if let Some(x) = &s.programme_libelle {
        corps.push_str(&format!("\nProgramme : {x}\n"));
    }
    if let Some(x) = &s.consignes_transport {
        corps.push_str(&format!("\nConsignes : {x}\n"));
    }
    if s.nature == "obligatoire" {
        corps.push_str(
            "\nAucune participation financière n'est demandée aux familles pour cette sortie obligatoire.\n",
        );
    } else if let Some((_, p)) = prix_courant(conn, a.etablissement_id, s.id).await? {
        corps.push_str(&format!(
            "\nParticipation demandée aux familles : {} par élève (montant fixe annoncé).\n",
            euros(p)
        ));
    }
    if s.nature == "facultative" {
        corps.push_str(
            "\nCette sortie est facultative : l'autorisation d'un responsable légal est demandée.\n",
        );
    }
    corps.push_str("\nNous restons à votre disposition.\n");
    Ok((format!("Circulaire — {}", s.titre), corps))
}

#[allow(clippy::too_many_arguments)]
/// Dépose le PDF (clé opaque, empreinte et taille au registre des objets, dans la transaction), puis
/// inscrit le document en ajout seul.
async fn emettre_document(
    e: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    type_: &str,
    version: Option<Uuid>,
    nom: &str,
    pdf: &[u8],
) -> Result<(DocumentVue, MetierEvent), EntDbError> {
    let doc = Uuid::new_v4();
    let cle = ObjectStore::key_for(a.etablissement_id, "voyages", doc, "pdf");
    let info = e.objet_put(&cle, pdf).await?;
    crate::objets::enregistrer_objet(&mut *conn, a.etablissement_id, &info, Some("application/pdf"))
        .await?;
    sqlx::query(
        "INSERT INTO ent_sortie_document
             (id, etablissement_id, sortie_id, type, version_id, cle_objet, nom, taille, sha256,
              emis_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(doc)
    .bind(a.etablissement_id)
    .bind(s.id)
    .bind(type_)
    .bind(version)
    .bind(&cle)
    .bind(nom)
    .bind(pdf.len() as i32)
    .bind(&info.sha256)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journaliser(
        conn,
        a,
        s.id,
        Fait {
            champ: Some(type_),
            apres: Some(info.sha256.chars().take(12).collect()),
            ..Fait::simple("document_emis")
        },
    )
    .await?;
    let ev = publier(conn, a, "voyages.document_emis", s.id, json!({ "type": type_ })).await?;
    let v = documents_de(conn, a.etablissement_id, s.id, Some(doc)).await?;
    Ok((v.into_iter().next().ok_or_else(introuvable)?, ev))
}

async fn documents_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    seul: Option<Uuid>,
) -> Result<Vec<DocumentVue>, EntDbError> {
    sqlx::query_as::<_, DocumentVue>(
        "SELECT d.id, d.type, d.nom, d.taille, d.sha256, d.emis_le, bv.numero AS version_budget,
                (d.id = (SELECT d2.id FROM ent_sortie_document d2
                          WHERE d2.etablissement_id = d.etablissement_id AND d2.sortie_id = d.sortie_id
                            AND d2.type = d.type ORDER BY d2.emis_le DESC, d2.id DESC LIMIT 1)) AS actuel
           FROM ent_sortie_document d
           LEFT JOIN ent_sortie_budget_version bv
             ON bv.etablissement_id = d.etablissement_id AND bv.id = d.version_id
          WHERE d.etablissement_id = $1 AND d.sortie_id = $2 AND ($3::uuid IS NULL OR d.id = $3)
          ORDER BY d.emis_le DESC, d.id DESC",
    )
    .bind(etab)
    .bind(sortie)
    .bind(seul)
    .fetch_all(conn)
    .await
    .map_err(db)
}

fn peut_voir_document(acces: &Acces, type_: &str) -> bool {
    match type_ {
        "budget" => peut_lire_budget(acces),
        _ => acces.gere(Action::VoyagesModifier)
            || acces.a(Action::VoyagesValider)
            || acces.a(Action::VoyagesAutoriser),
    }
}

impl PgEnt {
    /// VOY-219 (NX-57) : rend la fiche budget d'une **version validée** en PDF, côté serveur.
    pub async fn voyage_budget_pdf(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<DocumentVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_emettre_budget(&acces) {
            return Err(interdit());
        }
        let v = version_validee(&mut t, a.etablissement_id, id).await?.ok_or_else(|| {
            EntDbError::ConflitDossierDetail(
                "La fiche budget se rend à partir d'une version validée : aucune n'existe.".to_string(),
            )
        })?;
        let vue = vue_version(&mut t, a.etablissement_id, &v, s.nature == "obligatoire").await?;
        let prix = prix_de(&mut t, a.etablissement_id, id).await?;
        let d = dates_paris(&mut t, &s, v.valide_le, &prix).await?;
        let blocs = blocs_fiche_budget(&s, &d, &vue, &prix);
        let pdf = rendre(&format!("ScolIA — fiche budget — {} — version {}", s.reference, v.numero), &blocs);
        let (doc, ev) = emettre_document(
            self,
            &mut t,
            a,
            &s,
            "budget",
            Some(v.id),
            &format!("budget-{}-v{}.pdf", s.reference, v.numero),
            &pdf,
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(doc)
    }

    /// VOY-170 (NX-59) : rend la circulaire aux familles en PDF, côté serveur. Mêmes gardes que le
    /// texte : porteur ou droit de modifier, sortie `ouverte` ou `prete` (R-SV5). Une nouvelle
    /// circulaire remplace l'ancienne (la plus récente fait foi) sans l'effacer.
    pub async fn voyage_circulaire_pdf(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<DocumentVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !acces.gere(Action::VoyagesModifier) {
            return Err(interdit());
        }
        if !matches!(s.etat.as_str(), "ouverte" | "prete") {
            return Err(refus(
                RefusMetier::Conflit,
                "La circulaire aux familles se prépare une fois la sortie autorisée.",
            ));
        }
        let (titre, corps) = composer_circulaire(&mut t, a, &s).await?;
        let mut blocs = vec![Bloc::Titre(titre), Bloc::Espace];
        blocs.extend(corps.split("\n\n").map(|p| Bloc::Paragraphe(p.trim_end().to_string())));
        let pdf = rendre(&format!("ScolIA — circulaire — {}", s.reference), &blocs);
        let (doc, ev) = emettre_document(
            self,
            &mut t,
            a,
            &s,
            "circulaire",
            None,
            &format!("circulaire-{}.pdf", s.reference),
            &pdf,
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(doc)
    }

    /// Documents émis pour la sortie, limités à ceux que l'appelant a le droit de voir.
    pub async fn voyage_documents(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<DocumentVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let v = documents_de(&mut t, a.etablissement_id, id, None).await?;
        t.commit().await.map_err(db)?;
        Ok(v.into_iter().filter(|d| peut_voir_document(&acces, &d.type_)).collect())
    }

    /// Relit un document (nom, octets) : droit contrôlé, empreinte et taille vérifiées contre le
    /// registre des objets. Aucune URL publique (R-SV28).
    pub async fn voyage_document_lire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        doc: Uuid,
    ) -> Result<(String, Vec<u8>), EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let r: Option<(String, String, String, String)> = sqlx::query_as(
            "SELECT type, cle_objet, nom, sha256 FROM ent_sortie_document
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(doc)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        let (type_, cle, nom, sha) = r.ok_or_else(introuvable)?;
        if !peut_voir_document(&acces, &type_) {
            return Err(introuvable());
        }
        let octets = self
            .lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha))
            .await?;
        Ok((nom, octets))
    }
}

// ───────────────────────── Synthèse APEL (NX-58) ─────────────────────────

fn montant_csv(c: i64) -> String {
    format!("{},{:02}", c / 100, c % 100)
}

impl PgEnt {
    /// VOY-226 (NX-58, corrigé) : synthèse APEL annuelle, **comptabilité seule**. Ne compte que les
    /// sorties `prete`, `en_cours`, `rentree` ou `close` (corrige VD-36 : Next.js comptait aussi les
    /// brouillons) de l'**année scolaire du socle 5**, sur leur version de budget **validée** ;
    /// montants **agrégés** par sortie (APEL collectif et total des aides individuelles, aucun nom).
    pub async fn voyages_synthese_apel(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
    ) -> Result<SyntheseApel, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesBudgetValider).await?;
        let mut t = tx(self, a).await?;
        let (annee, libelle): (Uuid, String) = match annee_id {
            Some(x) => sqlx::query_as(
                "SELECT id, libelle FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(x),
            None => sqlx::query_as(
                "SELECT id, libelle FROM ent_annee_scolaire
                  WHERE etablissement_id = $1 AND date_debut <= ent_aujourdhui_etab(etablissement_id) AND date_fin >= ent_aujourdhui_etab(etablissement_id)
                  ORDER BY date_debut DESC LIMIT 1",
            )
            .bind(a.etablissement_id),
        }
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let lignes = sqlx::query_as::<_, LigneApel>(
            "SELECT s.id AS sortie_id, s.reference, s.titre, s.debut,
                    to_char(s.debut AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY') AS jour, s.etat,
                    coalesce(sum(l.montant_centimes) FILTER (WHERE l.poste = 'apel'), 0)::bigint AS apel_centimes,
                    coalesce(sum(l.montant_centimes) FILTER (WHERE l.poste = 'aides_individuelles'), 0)::bigint AS aides_centimes
               FROM ent_sortie s
               JOIN ent_sortie_budget_version v
                 ON v.etablissement_id = s.etablissement_id AND v.sortie_id = s.id AND v.etat = 'validee'
               JOIN ent_sortie_budget_ligne l
                 ON l.etablissement_id = v.etablissement_id AND l.version_id = v.id
                AND l.sens = 'recette' AND l.poste IN ('apel', 'aides_individuelles')
              WHERE s.etablissement_id = $1 AND s.annee_id = $2
                AND s.etat IN ('prete', 'en_cours', 'rentree', 'close')
              GROUP BY s.id, s.reference, s.titre, s.debut, s.etat
             HAVING sum(l.montant_centimes) > 0
              ORDER BY s.debut, s.id",
        )
        .bind(a.etablissement_id)
        .bind(annee)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        let total_apel: i64 = lignes.iter().map(|l| l.apel_centimes).sum();
        let total_aides: i64 = lignes.iter().map(|l| l.aides_centimes).sum();
        Ok(SyntheseApel {
            annee_id: annee,
            annee_libelle: libelle,
            lignes,
            total_apel_centimes: total_apel,
            total_aides_centimes: total_aides,
            total_centimes: total_apel + total_aides,
        })
    }

    /// Même synthèse en CSV (séparateur `;`, virgule décimale, aucune formule interprétable).
    pub async fn voyages_synthese_apel_csv(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
    ) -> Result<String, EntDbError> {
        let s = self.voyages_synthese_apel(a, annee_id).await?;
        let mut o = String::from(
            "Référence;Titre;Départ;État;APEL collectif (€);Aides individuelles (€);Total (€)\n",
        );
        for l in &s.lignes {
            o.push_str(&format!(
                "{};{};{};{};{};{};{}\n",
                champ_csv(&l.reference),
                champ_csv(&l.titre),
                l.jour,
                champ_csv(&l.etat),
                montant_csv(l.apel_centimes),
                montant_csv(l.aides_centimes),
                montant_csv(l.apel_centimes + l.aides_centimes),
            ));
        }
        o.push_str(&format!(
            "Total;;;;{};{};{}\n",
            montant_csv(s.total_apel_centimes),
            montant_csv(s.total_aides_centimes),
            montant_csv(s.total_centimes)
        ));
        Ok(o)
    }
}
