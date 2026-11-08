//! Ports du socle 2 (l'élève) et du référentiel (socle 5) appelés par le socle 3 — lot I1
//! (`docs/socles/03_INSCRIPTIONS.md` §5.2, §5.3 et §16 M2).
//!
//! **Le module d'inscriptions n'écrit jamais lui-même** dans `ent_eleve`, `ent_eleve_scolarite_annee`
//! ni `ent_affectation` : il appelle ces commandes dans la transaction de la validation (ou de
//! l'import). Fait, journal et événement partent ensemble ou pas du tout ; une erreur remonte et
//! défait la transaction de l'appelant. Le test `tests_garde_inscriptions` échoue si le module
//! reprend une écriture directe.
//!
//! * [`creer_eleve_tx`] — ELV-01 : fiche élève, version d'identité initiale (par la base),
//!   journal, événement `eleve.created` **sans état civil**.
//! * [`enregistrer_inscription_annuelle_tx`] — ELV-09 : inscription annuelle **prévue** (année,
//!   niveau, parcours, régime). Sur le schéma actuel c'est une ligne `projetee` de
//!   `ent_eleve_scolarite_annee` ; le contrat ne changera pas quand le lot E4 (`ent_eleve_inscription`)
//!   arrivera.
//! * [`affecter_arrivee_en_cours_d_annee_tx`] — affectation initiale en classe, réservée aux
//!   arrivées en cours d'année (la constitution des classes relève du socle 5).

use chrono::{NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::EntDbError;

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("eleve_ports", err))
}

fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

/// Origine d'une création d'élève (colonne `source` de la version d'identité et de l'événement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEleve {
    /// Validation d'un dossier d'admission (socle 3).
    Admission,
    /// Import d'une liste d'élèves (cadre d'import S2-3, socle 9 à terme).
    Import,
}

impl SourceEleve {
    /// Valeur de `source` reconnue par `ent_eleve_identite`.
    fn code_identite(self) -> &'static str {
        match self {
            SourceEleve::Admission => "admission",
            SourceEleve::Import => "import",
        }
    }
    /// Valeur historique de la charge utile de `eleve.created`.
    fn code_evenement(self) -> &'static str {
        match self {
            SourceEleve::Admission => "inscription",
            SourceEleve::Import => "import",
        }
    }
}

/// Statut initial de la fiche (colonne `status` héritée, jusqu'au statut dérivé du lot E4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatutInitial {
    /// Candidat validé : pas encore dans une classe de l'année courante.
    Preinscrit,
    /// Élève déjà présent (import d'une liste d'élèves en place).
    Actif,
}

impl StatutInitial {
    fn code(self) -> &'static str {
        match self {
            StatutInitial::Preinscrit => "preinscrit",
            StatutInitial::Actif => "actif",
        }
    }
}

/// Valeurs de départ d'un élève. Elles viennent de la déclaration de la famille (dossier) ou
/// d'une ligne d'import, jamais d'une saisie directe dans les tables du socle 2.
#[derive(Debug, Clone)]
pub struct NouvelEleve<'a> {
    pub nom: &'a str,
    pub prenom: &'a str,
    pub ine: Option<&'a str>,
    pub sexe: Option<&'a str>,
    pub date_naissance: Option<NaiveDate>,
    pub regime: Option<&'a str>,
    pub statut: StatutInitial,
}

/// Référence de l'objet à l'origine du fait (dossier, lot d'import) : identifiant seul, jamais de
/// donnée personnelle.
#[derive(Debug, Clone, Copy)]
pub struct OrigineFait {
    pub source: SourceEleve,
    pub source_ref: Option<Uuid>,
}

/// ELV-01 — crée la fiche élève dans la transaction de l'appelant.
///
/// Le journal et l'événement ne portent aucun état civil (ni nom, ni date de naissance, ni INE).
pub async fn creer_eleve_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    e: &NouvelEleve<'_>,
    origine: OrigineFait,
    actor: Uuid,
) -> Result<Uuid, EntDbError> {
    if e.nom.trim().is_empty() || e.prenom.trim().is_empty() {
        return Err(invalide("nom et prénom de l'élève obligatoires"));
    }
    // Auteur et source de la version d'identité initiale (écrite par la base à l'insertion).
    sqlx::query("SELECT set_config('ent.acteur_id', $1, true), set_config('ent.identite_source', $2, true)")
        .bind(actor.to_string())
        .bind(origine.source.code_identite())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let eleve_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, ine, status, regime, sexe,
                                date_naissance, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, now(), now())",
    )
    .bind(eleve_id)
    .bind(etab)
    .bind(e.nom)
    .bind(e.prenom)
    .bind(e.ine)
    .bind(e.statut.code())
    .bind(e.regime)
    .bind(e.sexe)
    .bind(e.date_naissance)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif','eleve_cree','Dossier élève créé',$4,true,$5, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(json!({
        "eleve_id": eleve_id,
        "source": origine.source.code_evenement(),
        "source_ref": origine.source_ref,
    }))
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let mut payload = json!({
        "eleve_id": eleve_id,
        "statut": e.statut.code(),
        "source": origine.source.code_evenement(),
    });
    if let (Some(r), Some(o)) = (origine.source_ref, payload.as_object_mut()) {
        let cle = match origine.source {
            SourceEleve::Admission => "dossier_id",
            SourceEleve::Import => "lot_id",
        };
        o.insert(cle.to_string(), json!(r));
    }
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: types::ELEVE_CREATED.into(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve_id,
        eleve_id: Some(eleve_id),
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(eleve_id)
}

/// Origine d'une inscription annuelle (colonne `source` de `ent_eleve_scolarite_annee`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceInscription {
    Admission,
    Reinscription,
}

impl SourceInscription {
    fn code(self) -> &'static str {
        match self {
            SourceInscription::Admission => "admission",
            SourceInscription::Reinscription => "reinscription",
        }
    }
}

/// Inscription annuelle à enregistrer **à l'état prévu** (niveau, parcours, régime).
#[derive(Debug, Clone)]
pub struct InscriptionAnnuellePrevue<'a> {
    pub eleve_id: Uuid,
    /// Année scolaire, au format `2026-2027` (celui des campagnes).
    pub annee_scolaire: &'a str,
    pub niveau: Option<&'a str>,
    pub parcours: Option<&'a str>,
    pub regime: Option<&'a str>,
    pub source: SourceInscription,
    /// Dossier d'origine.
    pub source_ref: Option<Uuid>,
}

fn borne(v: Option<&str>, max: usize, msg: &'static str) -> Result<Option<String>, EntDbError> {
    match v.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() <= max => Ok(Some(s.to_string())),
        Some(_) => Err(invalide(msg)),
    }
}

/// ELV-09 — enregistre (ou met à jour) l'inscription annuelle **prévue** de l'élève.
///
/// * Idempotent : une validation rejouée met à jour la même ligne.
/// * Une année déjà `courante` ou `historique` n'est jamais réécrite (conflit) : l'élève est déjà
///   inscrit, ou l'a été, pour cette année.
/// * Une confirmation déjà donnée par la famille (`reinscription`) garde son statut.
///
/// Renvoie l'identifiant de l'inscription annuelle.
pub async fn enregistrer_inscription_annuelle_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    i: &InscriptionAnnuellePrevue<'_>,
    actor: Uuid,
) -> Result<Uuid, EntDbError> {
    let annee = borne(Some(i.annee_scolaire), 20, "année scolaire invalide")?
        .ok_or(invalide("année scolaire obligatoire"))?;
    let niveau = borne(i.niveau, 40, "niveau trop long")?;
    let parcours = borne(i.parcours, 120, "parcours trop long")?;
    let regime = borne(i.regime, 40, "régime trop long")?;
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(etab)
    .bind(i.eleve_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !ok {
        return Err(EntDbError::Domain(DomainError::ReferenceInconnue));
    }
    // RP2-10 : la scolarisation à la date de rentrée existe avant l'inscription annuelle, pour
    // que le pont vers `ent_eleve_inscription` (modèle du socle 2, E4) puisse écrire la prévue.
    assurer_scolarisation_prevue_tx(conn, etab, i.eleve_id, &annee, i.source, i.source_ref, actor).await?;
    let existante: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, statut FROM ent_eleve_scolarite_annee
          WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_scolaire = $3 FOR UPDATE",
    )
    .bind(etab)
    .bind(i.eleve_id)
    .bind(&annee)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let id = match existante {
        Some((_, statut)) if matches!(statut.as_str(), "courante" | "historique") => {
            return Err(conflit(
                "l'élève est déjà inscrit (ou l'a été) pour cette année scolaire",
            ))
        }
        Some((id, _)) => {
            sqlx::query(
                "UPDATE ent_eleve_scolarite_annee
                    SET niveau = COALESCE($3, niveau), parcours = COALESCE($4, parcours),
                        regime = COALESCE($5, regime), source = $6, source_ref = $7, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(&niveau)
            .bind(&parcours)
            .bind(&regime)
            .bind(i.source.code())
            .bind(i.source_ref)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            id
        }
        None => {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_eleve_scolarite_annee
                     (id, etablissement_id, eleve_id, annee_scolaire, niveau, parcours, regime,
                      statut, source, source_ref, passage_juridique, redoublement_possible)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'projetee',$8,$9,false,true)",
            )
            .bind(id)
            .bind(etab)
            .bind(i.eleve_id)
            .bind(&annee)
            .bind(&niveau)
            .bind(&parcours)
            .bind(&regime)
            .bind(i.source.code())
            .bind(i.source_ref)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            id
        }
    };
    // Journal sans donnée personnelle : année, niveau et origine seulement.
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif','inscription_annuelle_prevue',
                 'Inscription annuelle prévue',$4,true,$5, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(i.eleve_id)
    .bind(json!({
        "inscription_annuelle_id": id, "annee": annee, "niveau": niveau,
        "source": i.source.code(), "source_ref": i.source_ref,
    }))
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// RP2-10 — ouvre, si l'élève n'en a pas, la scolarisation qui commence à la rentrée de l'année
/// visée (ou aujourd'hui si l'année a déjà commencé). Idempotent, dans la transaction de
/// l'appelant, journalisé. Sans effet si l'année n'existe pas encore dans le référentiel (la
/// ligne attend alors dans `ent_eleve_inscription_a_traiter`, reprise à la préparation de l'année)
/// ou si l'élève est déjà scolarisé (ou le sera) à cette date. Motif « retour » pour un ancien élève.
async fn assurer_scolarisation_prevue_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    annee_libelle: &str,
    source: SourceInscription,
    source_ref: Option<Uuid>,
    actor: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let annee: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND libelle = $2",
    )
    .bind(etab)
    .bind(annee_libelle)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((debut, fin)) = annee else {
        return Ok(None);
    };
    let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_jour_etablissement($1)")
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let entree = std::cmp::max(debut, jour);
    if entree >= fin {
        return Ok(None);
    }
    // L'élève est-il déjà scolarisé (ou le sera-t-il) à partir de cette date ?
    let (couvert, anterieure): (bool, bool) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve_scolarisation
                         WHERE etablissement_id = $1 AND eleve_id = $2
                           AND (date_sortie IS NULL OR date_sortie > $3)),
                EXISTS (SELECT 1 FROM ent_eleve_scolarisation
                         WHERE etablissement_id = $1 AND eleve_id = $2 AND date_entree < $3)",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(entree)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if couvert {
        return Ok(None);
    }
    sqlx::query("SELECT set_config('ent.acteur_id', $1, true)")
        .bind(actor.to_string())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let id = Uuid::new_v4();
    let motif = if anterieure { "retour" } else { "premiere_inscription" };
    sqlx::query(
        "INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree,
                                              source, source_ref, decide_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve_id)
    .bind(entree)
    .bind(motif)
    .bind(source.code())
    .bind(source_ref)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif','scolarisation_ouverte','Scolarisation ouverte',$4,true,$5, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(json!({ "scolarisation_id": id, "motif": motif, "source": source.code(), "source_ref": source_ref }))
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Some(id))
}

/// Affectation initiale en classe — **réservée à l'arrivée en cours d'année** (§7.3 et M10 :
/// avant la rentrée, la constitution des classes relève du socle 5). Idempotente.
///
/// Renvoie `true` si l'affectation a été posée (ou existait déjà pour une classe de l'année
/// courante), `false` si la classe n'existe pas dans l'établissement. Pour une classe de l'année
/// courante, la classe de la fiche est alignée et l'élève devient actif.
pub async fn affecter_arrivee_en_cours_d_annee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    classe_id: Uuid,
    source: &'static str,
) -> Result<bool, EntDbError> {
    let classe: Option<(Uuid, bool, NaiveDate)> = sqlx::query_as(
        "SELECT c.annee_id, a.courante, a.date_debut FROM ent_classe c
           JOIN ent_annee_scolaire a ON a.id = c.annee_id AND a.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.id = $2 AND c.archivee_le IS NULL",
    )
    .bind(etab)
    .bind(classe_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((annee_id, courante, debut_annee)) = classe else {
        return Ok(false);
    };
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !ok {
        return Ok(false);
    }
    let debut = std::cmp::max(debut_annee, Utc::now().date_naive());
    let pose = sqlx::query(
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
         VALUES ($1,$2,$3,$4,$5,$6,$7)
         ON CONFLICT (etablissement_id, eleve_id, annee_id)
            WHERE classe_id IS NOT NULL AND date_fin IS NULL DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee_id)
    .bind(eleve_id)
    .bind(classe_id)
    .bind(debut)
    .bind(source)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Une affectation déjà posée (même classe ou autre) n'est jamais remplacée ici : la fiche
    // n'est alignée que si l'affectation vient d'être créée (sinon fiche et affectation divergent).
    let deja_a_cette_classe = pose.rows_affected() == 0
        && sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM ent_affectation
                             WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3
                               AND classe_id = $4 AND date_fin IS NULL)",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(annee_id)
        .bind(classe_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    if courante && (pose.rows_affected() == 1 || deja_a_cette_classe) {
        // Classe courante de l'élève (le déclencheur `classe_id` aligne le texte).
        sqlx::query(
            "UPDATE ent_eleve SET classe_id = $3, status = 'actif', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(classe_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(true)
}

/// La classe est-elle une classe de l'année **courante** de l'établissement ? (arrivée en cours
/// d'année : seul cas où la validation d'un dossier affecte en classe.)
pub async fn classe_de_l_annee_courante_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    classe_id: Uuid,
) -> Result<bool, EntDbError> {
    let (oui,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_classe c
                          JOIN ent_annee_scolaire a ON a.id = c.annee_id AND a.etablissement_id = c.etablissement_id
                         WHERE c.etablissement_id = $1 AND c.id = $2 AND a.courante)",
    )
    .bind(etab)
    .bind(classe_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(oui)
}

/// ELV-02 (commande inverse d'ELV-01, appelée par l'annulation d'une validation erronée, INS-68) :
/// annule la fiche créée **par erreur**. Refusée si la fiche a déjà une vie (classe, scolarisation,
/// sortie). Rien n'est effacé : la fiche passe à `archive` (statut dérivé), la trace reste au journal ; la trace reste au journal et par l'événement `eleve.creation_annulee`
/// (sans donnée personnelle). Renvoie `false` si la fiche n'existe pas ou n'est plus annulable.
pub async fn annuler_creation_eleve_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    dossier_id: Uuid,
    actor: Uuid,
) -> Result<bool, EntDbError> {
    // La fiche créée par la validation n'a ni classe ni vie propre. Depuis RP2-10, la validation
    // ouvre la scolarisation « prévue » (source = ce dossier) : elle seule est admise ; toute autre
    // scolarisation (saisie, autre dossier, sortie) rend l'annulation impossible. Une scolarisation
    // ne se supprime jamais : celle du dossier est clôturée à durée nulle (sortie = entrée, motif
    // « autre »), puis la fiche est marquée `archive`. Le statut hérité (`status`) n'est pas touché
    // par l'archivage lui-même.
    let annulable: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve e
                         WHERE e.etablissement_id = $1 AND e.id = $2 AND e.classe_id IS NULL
                           AND e.status IN ('preinscrit', 'actif') AND e.date_sortie IS NULL
                           AND NOT EXISTS (
                               SELECT 1 FROM ent_eleve_scolarisation s
                                WHERE s.etablissement_id = $1 AND s.eleve_id = $2
                                  AND NOT (s.source IN ('admission', 'reinscription') AND s.source_ref = $3
                                           AND s.date_sortie IS NULL)))",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(dossier_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !annulable {
        return Ok(false);
    }
    sqlx::query("SELECT set_config('ent.acteur_id', $1, true)")
        .bind(actor.to_string())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    sqlx::query(
        "UPDATE ent_eleve_scolarisation
            SET date_sortie = date_entree, motif_sortie = 'autre',
                motif_sortie_libre = 'Validation du dossier annulée', cloture_le = now(), cloture_par = $4
          WHERE etablissement_id = $1 AND eleve_id = $2 AND source_ref = $3 AND date_sortie IS NULL",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(dossier_id)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let r = sqlx::query(
        "UPDATE ent_eleve SET statut = 'archive', updated_at = now()
          WHERE etablissement_id = $1 AND id = $2 AND classe_id IS NULL",
    )
    .bind(etab)
    .bind(eleve_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif','creation_annulee','Création de la fiche annulée',$4,false,$5, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(json!({ "eleve_id": eleve_id, "dossier_id": dossier_id }))
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: types::ELEVE_CREATION_ANNULEE.into(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve_id,
        eleve_id: Some(eleve_id),
        payload: json!({ "eleve_id": eleve_id, "dossier_id": dossier_id }),
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(true)
}

/// Commande inverse d'ELV-09 : retire l'inscription annuelle **prévue** (ligne `projetee`) créée par
/// un dossier. Une inscription courante, historique ou confirmée par la famille n'est jamais
/// retirée. Renvoie `true` si une ligne a été retirée.
pub async fn retirer_inscription_annuelle_prevue_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    dossier_id: Uuid,
) -> Result<bool, EntDbError> {
    let r = sqlx::query(
        "DELETE FROM ent_eleve_scolarite_annee
          WHERE etablissement_id = $1 AND eleve_id = $2 AND source_ref = $3 AND statut = 'projetee'",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(dossier_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected() > 0)
}
