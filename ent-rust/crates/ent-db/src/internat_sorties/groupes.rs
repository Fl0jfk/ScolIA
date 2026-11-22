//! Socle 41, lot I4 — **sorties de groupe** (§10.3, INT-170 à INT-179 ; R-IT21, R-IT24, R-IT25).
//!
//! Circuit : l'organisateur (`internat.gerer`) crée un brouillon (accompagnateurs = comptes du personnel, internes
//! invités) ; la soumission interroge les familles ; la direction (`internat.sorties.decider`, jamais l'auteur de la
//! sortie) accepte ou refuse ; le jour J, un accompagnateur fait l'appel de départ puis de retour ; la clôture exige que
//! tout élève parti soit rentré.
//!
//! Règles générales de protection appliquées ici (voir `docs/socles/41_INTERNAT_I4_LIVRAISON.md`) :
//! * **refus par défaut** : seul un élève `autorise` peut partir ; une réponse manquante, un doute ou une erreur de lecture
//!   ne l'autorisent jamais ;
//! * **un refus ou un blocage n'exclut que l'élève concerné** (R-IT21) ; **le plus restrictif l'emporte** entre titulaires ;
//! * **un acte tombe avec son auteur** : au départ, l'accord d'un titulaire désormais visé par une interdiction (ou qui n'a
//!   plus l'autorité), l'accord hors ligne d'une personne interdite, l'accord annuel dont les décideurs sont interdits sont
//!   écartés — l'élève ne part pas, la cause (`interdiction_auteur`) n'est jamais relayée aux familles ;
//! * **pas de départ constaté = élève attendu** : une sortie de groupe ne retire JAMAIS un élève de l'attendu de la nuit
//!   (l'appel du soir le pointe « en activité » avec son retour attendu) ;
//! * **R-IT24** un accord hors ligne nomme la personne qui l'a donné, la date de réception, le canal et le compte qui l'a
//!   reçu ; jamais un « autorisé par l'équipe » anonyme ;
//! * **R-IT25** aucun accompagnateur en texte libre : des comptes du personnel de l'établissement ;
//! * **R-IT32** une famille ne voit que SON enfant, jamais un autre participant ni le nombre d'inscrits.
//!
//! Chaque fonction `*_a` reçoit l'instant (`maintenant`) : aucun test ne dépend de l'heure réelle.

use super::*;
use ent_types::internat_vie::*;

const MOTIFS_REFUS_DIRECTION: [&str; 5] = [
    "securite",
    "encadrement",
    "calendrier",
    "reglement",
    "autre",
];
const MOTIFS_ANNULATION: [&str; 5] = ["meteo", "encadrement", "organisateur", "securite", "autre"];
const BILANS: [&str; 3] = ["sans_incident", "retard", "incident"];
const MOTIFS_ANNULATION_RETOUR: [&str; 3] = ["erreur_pointage", "retour_non_effectif", "autre"];
const CANAUX: [&str; 3] = ["telephone", "papier", "en_personne"];
/// L'appel de départ s'ouvre 30 minutes avant l'heure prévue.
const DEPART_AVANCE_MINUTES: i64 = 30;
/// Un accord reçu hors ligne se date au plus de 90 jours en arrière (et jamais dans le futur).
const HORS_LIGNE_ANCIENNETE_JOURS: i64 = 90;
/// Une sortie se prévoit un an à l'avance au plus.
pub(super) const GROUPE_HORIZON_JOURS: i64 = 366;
/// Au plus 200 élèves par sortie, 20 accompagnateurs.
const GROUPE_MAX_ELEVES: usize = 200;
const GROUPE_MAX_ENCADRANTS: usize = 20;
/// Un accompagnateur lit sa sortie jusqu'à 7 jours après sa fin (F4).
const ENCADRANT_LECTURE_JOURS: i64 = 7;

pub(super) struct GroupeLigne {
    pub(super) id: Uuid,
    pub(super) titre: String,
    pub(super) lieu: String,
    pub(super) debut: DateTime<Utc>,
    pub(super) fin: DateTime<Utc>,
    pub(super) statut: String,
    pub(super) cree_par: Uuid,
    pub(super) decide_par: Option<Uuid>,
    pub(super) decide_le: Option<DateTime<Utc>>,
    pub(super) decision_motif: Option<String>,
    pub(super) annule_motif: Option<String>,
    pub(super) bilan: Option<String>,
    pub(super) modele_id: Option<Uuid>,
}

type LigneBrute = (
    Uuid,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
    String,
    Uuid,
    Option<Uuid>,
    Option<DateTime<Utc>>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<Uuid>,
);

const COLONNES_GROUPE: &str =
    "id, titre, lieu, debut, fin, statut, cree_par_user_id, decide_par_user_id, decide_le, \
     decision_motif_code, annule_motif_code, bilan_code, modele_id";

fn groupe_de(r: LigneBrute) -> GroupeLigne {
    GroupeLigne {
        id: r.0,
        titre: r.1,
        lieu: r.2,
        debut: r.3,
        fin: r.4,
        statut: r.5,
        cree_par: r.6,
        decide_par: r.7,
        decide_le: r.8,
        decision_motif: r.9,
        annule_motif: r.10,
        bilan: r.11,
        modele_id: r.12,
    }
}

pub(super) async fn groupe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<GroupeLigne, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_GROUPE} FROM ent_internat_sortie_groupe WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    let r: Option<LigneBrute> = sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
    r.map(groupe_de).ok_or_else(|| DomainError::NotFound.into())
}

/// Réglages de vie quotidienne (créés à la première lecture avec les valeurs prudentes de la migration).
async fn parametres_vie_tx(conn: &mut PgConnection, etab: Uuid) -> Result<(i16, i16), EntDbError> {
    sqlx::query("INSERT INTO ent_internat_parametres_vie (etablissement_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(etab)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    sqlx::query_as("SELECT groupe_eleves_par_encadrant, groupe_preavis_heures FROM ent_internat_parametres_vie WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

async fn fait_groupe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    eleve: Option<Uuid>,
    acte: &str,
    par: Option<Uuid>,
    a: DateTime<Utc>,
    motif: Option<&str>,
) -> Result<(), EntDbError> {
    fait_groupe_autre_tx(conn, etab, sortie, eleve, acte, par, None, a, motif).await
}

/// Fait de la sortie qui vise aussi un compte du personnel (accompagnateur présent, remplacé) : jamais un nom.
#[allow(clippy::too_many_arguments)]
async fn fait_groupe_autre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    eleve: Option<Uuid>,
    acte: &str,
    par: Option<Uuid>,
    autre: Option<Uuid>,
    a: DateTime<Utc>,
    motif: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_internat_sortie_groupe_fait \
         (id, etablissement_id, sortie_groupe_id, eleve_id, acte, par_user_id, autre_user_id, a, motif_code) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(sortie)
    .bind(eleve)
    .bind(acte)
    .bind(par)
    .bind(autre)
    .bind(a)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Événement de la sortie (sans élève). Charge : identifiants, dates, codes ; jamais un titre ni un lieu.
async fn evt_groupe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Option<Uuid>,
    type_name: &str,
    g: &GroupeLigne,
    statut: &str,
    cause: &str,
) -> Result<(), EntDbError> {
    publier(
        conn,
        etab,
        type_name,
        AggregateKind::Internat,
        g.id,
        None,
        json!({ "debut": g.debut.to_rfc3339(), "fin": g.fin.to_rfc3339(), "statut": statut, "cause": cause }),
        actor,
        Causalite::default(),
    )
    .await?;
    Ok(())
}

/// Événement visant un élève de la sortie (réponse attendue, accord, blocage). Toute cause `interdiction_*` ne se relaie
/// JAMAIS aux familles (contrat des causes d'interdiction, §22).
async fn evt_participant_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Option<Uuid>,
    g: &GroupeLigne,
    eleve: Uuid,
    etat: &str,
    cause: &str,
) -> Result<(), EntDbError> {
    publier(
        conn,
        etab,
        types::INTERNAT_SORTIE_GROUPE_AUTORISATION,
        AggregateKind::Internat,
        g.id,
        Some(eleve),
        json!({ "debut": g.debut.to_rfc3339(), "fin": g.fin.to_rfc3339(), "etat": etat, "cause": cause }),
        actor,
        Causalite::default(),
    )
    .await?;
    Ok(())
}

pub(super) async fn est_interne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_sejour s JOIN ent_eleve e \
                          ON e.etablissement_id = s.etablissement_id AND e.id = s.eleve_id \
                         WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.du <= $3 \
                           AND (s.au IS NULL OR s.au >= $3) AND (e.date_sortie IS NULL OR e.date_sortie > $3))",
    )
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Compte du personnel de l'établissement (adhésion ouverte, ni parent ni élève) : R-IT25.
pub(super) async fn est_personnel_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_membership m WHERE m.etablissement_id = $1 AND m.user_id = $2 \
                          AND m.etat = 'ouvert' AND m.role NOT IN ('parent', 'eleve') \
                          AND (m.date_fin IS NULL OR m.date_fin >= $3))",
    )
    .bind(etab)
    .bind(user)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Un accompagnateur lit et fait l'appel de SA sortie tant qu'il est encore du personnel de l'établissement (adhésion
/// ouverte, F4 : l'inscription dans la table ne suffit pas) et jusqu'à `ENCADRANT_LECTURE_JOURS` jours après la fin.
pub(super) async fn est_encadrant_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    user: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let jour = jour_local_tx(conn, etab, maintenant).await?;
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_sortie_groupe_encadrant e \
                          JOIN ent_internat_sortie_groupe g ON g.etablissement_id = e.etablissement_id AND g.id = e.sortie_groupe_id \
                         WHERE e.etablissement_id = $1 AND e.sortie_groupe_id = $2 AND e.user_id = $3 \
                           AND g.fin + make_interval(days => $4::int) > $5 \
                           AND EXISTS (SELECT 1 FROM ent_membership m WHERE m.etablissement_id = e.etablissement_id \
                                         AND m.user_id = e.user_id AND m.etat = 'ouvert' AND m.role NOT IN ('parent', 'eleve') \
                                         AND (m.date_fin IS NULL OR m.date_fin >= $6)))",
    )
    .bind(etab)
    .bind(sortie)
    .bind(user)
    .bind(ENCADRANT_LECTURE_JOURS as i32)
    .bind(maintenant)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

async fn encadrants_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT user_id FROM ent_internat_sortie_groupe_encadrant \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 ORDER BY user_id",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

/// Élèves « comptés » pour l'encadrement : invités, en attente ou autorisés (un refus ou un retrait n'y entre pas).
async fn nb_comptes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_internat_sortie_groupe_participant \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND etat IN ('invite', 'en_attente', 'autorise', 'parti')",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Encadrement suffisant : au moins un accompagnateur et pas plus d'élèves par accompagnateur que le réglage.
fn encadrement_suffisant(nb_eleves: i64, nb_encadrants: usize, par_encadrant: i16) -> bool {
    nb_eleves == 0
        || (nb_encadrants > 0 && nb_eleves <= nb_encadrants as i64 * i64::from(par_encadrant))
}

/// Ligne participant (brute).
type PartBrute = (
    Uuid,
    String,
    Option<String>,
    Option<Uuid>,
    Option<Uuid>,
    Option<DateTime<Utc>>,
    Option<String>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

const COLONNES_PART: &str =
    "eleve_id, etat, reponse_qualite, reponse_user_id, reponse_personne_id, reponse_le, \
     motif_code, parti_le, rentre_le";

async fn participants_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<Vec<PartBrute>, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_PART} FROM ent_internat_sortie_groupe_participant \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND etat <> 'retire' ORDER BY eleve_id"
    );
    sqlx::query_as(&sql)
        .bind(etab)
        .bind(sortie)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)
}

async fn participant_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    eleve: Uuid,
) -> Result<Option<PartBrute>, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_PART} FROM ent_internat_sortie_groupe_participant \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3 FOR UPDATE"
    );
    sqlx::query_as(&sql)
        .bind(etab)
        .bind(sortie)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)
}

/// Une sortie de groupe est-elle dans les bornes du type « soirée » ? (règle générale 2) : durée au plus égale à
/// `duree_max_heures` du type, même jour local, aucune nuit couverte. Hors de ces bornes, l'accord annuel « sortie de
/// soirée » ne vaut pas : la famille répond explicitement.
async fn dans_les_bornes_soiree_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    g: &GroupeLigne,
) -> Result<bool, EntDbError> {
    let t = type_tx(conn, etab, "soiree").await?;
    if !t.actif || g.fin - g.debut > Duration::hours(i64::from(t.duree_max_heures)) {
        return Ok(false);
    }
    let meme_jour: bool = sqlx::query_scalar(
        "SELECT ($2::timestamptz AT TIME ZONE ent_internat_fuseau($1))::date \
              = ($3::timestamptz AT TIME ZONE ent_internat_fuseau($1))::date",
    )
    .bind(etab)
    .bind(g.debut)
    .bind(g.fin)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(meme_jour && nuits_couvertes_tx(conn, etab, g.debut, g.fin).await? == 0)
}

/// Un élève ne peut pas être à deux endroits à la fois (F9) : un temps du soir où il est inscrit, ou une autre sortie de
/// groupe qu'il a acceptée ou qui attend sa famille, chevauche-t-il `[debut, fin)` ? **Sous verrou** : un verrou
/// consultatif par élève (pris avant la lecture) sérialise les deux sens (inscription à un temps / invitation à une
/// sortie), sans quoi deux demandes simultanées passeraient chacune le contrôle.
pub(super) async fn chevauchement_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    ignorer_temps: Option<Uuid>,
    ignorer_groupe: Option<Uuid>,
) -> Result<bool, EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('internat.i4.eleve:' || $1::text || ':' || $2::text, 0))")
        .bind(etab)
        .bind(eleve)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_temps_participant p JOIN ent_internat_temps t \
                          ON t.etablissement_id = p.etablissement_id AND t.id = p.temps_id \
                         WHERE p.etablissement_id = $1 AND p.eleve_id = $2 AND p.retire_le IS NULL \
                           AND t.statut = 'prevu' AND ($5::uuid IS NULL OR t.id <> $5) AND t.debut < $4 AND t.fin > $3) \
             OR EXISTS (SELECT 1 FROM ent_internat_sortie_groupe_participant p JOIN ent_internat_sortie_groupe g \
                          ON g.etablissement_id = p.etablissement_id AND g.id = p.sortie_groupe_id \
                         WHERE p.etablissement_id = $1 AND p.eleve_id = $2 AND p.etat IN ('en_attente', 'autorise', 'parti') \
                           AND g.statut IN ('soumise', 'acceptee') AND ($6::uuid IS NULL OR g.id <> $6) \
                           AND g.debut < $4 AND g.fin > $3)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(debut)
    .bind(fin)
    .bind(ignorer_temps)
    .bind(ignorer_groupe)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// État d'un participant à l'invitation (au moment de la soumission, ou d'un ajout après). **Refus par défaut** : tout
/// ce qui n'est pas un accord annuel vérifié, DANS LES BORNES d'une soirée, attend la famille. Retourne (état, qualité,
/// motif).
async fn evaluer_invitation_tx(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    g: &GroupeLigne,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<(&'static str, Option<&'static str>, Option<&'static str>), EntDbError> {
    if !est_interne_tx(conn, etab, eleve, jour).await? {
        return Ok(("non_autorise", None, Some("plus_interne")));
    }
    if desaccord_connu_tx(conn, etab, eleve).await? {
        return Ok(("non_autorise", None, Some("desaccord_connu")));
    }
    if chevauchement_tx(conn, etab, eleve, g.debut, g.fin, None, Some(g.id)).await? {
        return Ok(("non_autorise", None, Some("chevauchement")));
    }
    if dans_les_bornes_soiree_tx(conn, etab, g).await?
        && verdict_annuel(ent, conn, etab, eleve, "sortie_soiree", jour).await == "autorise"
    {
        return Ok(("autorise", Some("annuelle"), None));
    }
    Ok(("en_attente", None, None))
}

/// Pose l'état d'un participant invité (écriture + fait + événement).
#[allow(clippy::too_many_arguments)]
async fn appliquer_invitation_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    g: &GroupeLigne,
    eleve: Uuid,
    etat: &str,
    qualite: Option<&str>,
    motif: Option<&str>,
    actor: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_internat_sortie_groupe_participant SET etat = $4, reponse_qualite = $5, \
                reponse_le = CASE WHEN $5 IS NULL THEN NULL ELSE $6 END, motif_code = $7, modifie_le = $6 \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(g.id)
    .bind(eleve)
    .bind(etat)
    .bind(qualite)
    .bind(maintenant)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let acte = match (etat, motif) {
        ("autorise", _) => "accord_annuel",
        ("non_autorise", Some("desaccord_connu")) => "blocage_desaccord",
        // F11 : chaque cause porte son propre fait (jamais « interdiction » pour un élève qui n'est plus interne).
        ("non_autorise", Some("plus_interne")) => "blocage_plus_interne",
        ("non_autorise", Some("majorite")) => "blocage_majorite",
        ("non_autorise", Some("chevauchement")) => "blocage_chevauchement",
        ("non_autorise", _) => "blocage_interdiction",
        _ => "invitation",
    };
    fait_groupe_tx(
        conn,
        etab,
        g.id,
        Some(eleve),
        acte,
        Some(actor),
        maintenant,
        motif,
    )
    .await?;
    let cause = motif.unwrap_or(if etat == "autorise" {
        "accord_annuel"
    } else {
        "reponse_attendue"
    });
    evt_participant_tx(conn, etab, Some(actor), g, eleve, etat, cause).await
}

fn participant_dto(
    p: &PartBrute,
    noms: &HashMap<Uuid, (String, String)>,
    proteges: &HashSet<Uuid>,
    caches: &HashSet<Uuid>,
    voit_la_cause: bool,
) -> ParticipantGroupeDto {
    let cache = caches.contains(&p.0);
    ParticipantGroupeDto {
        eleve_id: p.0,
        nom: (!cache)
            .then(|| noms.get(&p.0).map(|n| n.0.clone()))
            .flatten(),
        prenom: (!cache)
            .then(|| noms.get(&p.0).map(|n| n.1.clone()))
            .flatten(),
        protege: proteges.contains(&p.0),
        etat: p.1.clone(),
        reponse_qualite: p.2.clone(),
        reponse_le: p.5,
        // E3 : la cause d'un blocage est une donnée de protection (interdiction, désaccord connu, accord tombé) : un
        // accompagnateur sans droit de lecture ne voit que l'état opérationnel « non autorisé ».
        motif: if voit_la_cause { p.6.clone() } else { None },
        parti_le: p.7,
        rentre_le: p.8,
    }
}

/// Titre, lieu et horaires d'une sortie : bornes, aucun texte de santé, créée avant son début.
fn valider_horaires(
    titre: &str,
    lieu: &str,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    maintenant: DateTime<Utc>,
) -> Result<(String, String), EntDbError> {
    let (titre, lieu) = (titre.trim(), lieu.trim());
    if titre.is_empty()
        || titre.chars().count() > 120
        || lieu.is_empty()
        || lieu.chars().count() > 120
    {
        return Err(refus("titre et lieu : 1 à 120 caractères"));
    }
    if evoque_la_sante(titre) || evoque_la_sante(lieu) {
        return Err(refus(
            "aucune donnée de santé dans l'internat : le texte évoque la santé",
        ));
    }
    if fin <= debut || fin > debut + Duration::days(14) {
        return Err(refus(
            "horaires invalides (fin après le début, 14 jours au plus)",
        ));
    }
    if debut <= maintenant {
        return Err(refus("une sortie se crée avant son début"));
    }
    if debut > maintenant + Duration::days(GROUPE_HORIZON_JOURS) {
        return Err(refus("une sortie se prévoit un an à l'avance au plus"));
    }
    Ok((titre.to_string(), lieu.to_string()))
}

impl PgEnt {
    async fn groupe_dto_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        g: &GroupeLigne,
        portee: Option<&HashSet<Uuid>>,
        ecartes: Vec<EcarteDto>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        let mut parts = participants_tx(conn, etab, g.id).await?;
        if let Some(p) = portee {
            parts.retain(|x| p.contains(&x.0));
        }
        let ids: Vec<Uuid> = parts.iter().map(|p| p.0).collect();
        let (noms, proteges, caches) = self
            .noms_pour_personnel(etab, actor, role, conn, &ids)
            .await?;
        let voit_la_cause = self.voit_la_cause(etab, actor, role).await?;
        Ok(SortieGroupeDto {
            id: g.id,
            titre: g.titre.clone(),
            lieu: g.lieu.clone(),
            debut: g.debut,
            fin: g.fin,
            statut: g.statut.clone(),
            cree_par_user_id: g.cree_par,
            decide_par_user_id: g.decide_par,
            decide_le: g.decide_le,
            decision_motif_code: g.decision_motif.clone(),
            annule_motif_code: g.annule_motif.clone(),
            bilan_code: g.bilan.clone(),
            modele_id: g.modele_id,
            encadrants: encadrants_tx(conn, etab, g.id).await?,
            participants: parts
                .iter()
                .map(|p| participant_dto(p, &noms, &proteges, &caches, voit_la_cause))
                .collect(),
            ecartes,
        })
    }

    /// Règle générale 3 : la CAUSE d'un blocage (interdiction, désaccord connu, accord tombé) est une donnée de protection ;
    /// seuls `internat.lire`, `internat.gerer` et `internat.sorties.decider` la voient.
    pub(super) async fn voit_la_cause(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<bool, EntDbError> {
        match self
            .nuit_portee(
                etab,
                actor,
                role,
                &[
                    Action::InternatLire,
                    Action::InternatGerer,
                    Action::InternatSortiesDecider,
                ],
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(EntDbError::Forbidden) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Droit d'organiser : `internat.gerer` d'établissement (INT-170, INT-173, INT-177).
    async fn groupe_organiser(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await
    }

    /// Droit de faire l'appel d'UNE sortie : un de ses accompagnateurs, ou `internat.appel` / `internat.gerer` d'établissement.
    async fn groupe_appel_exiger(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        sortie: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        if est_encadrant_tx(conn, etab, sortie, actor, maintenant).await? {
            return Ok(());
        }
        for a in [Action::InternatAppel, Action::InternatGerer] {
            if self.a_action_etablissement(etab, actor, role, a).await? {
                return Ok(());
            }
        }
        Err(EntDbError::Forbidden)
    }

    /// Valide une liste d'invités : élèves de l'établissement, visibles de l'acteur ; les autres sont écartés avec un
    /// motif codé (jamais en silence). Retourne les invités retenus et les écartés.
    async fn groupe_trier_invites(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
        eleves: &[Uuid],
    ) -> Result<(Vec<Uuid>, Vec<EcarteDto>), EntDbError> {
        let mut retenus: Vec<Uuid> = Vec::new();
        let mut ecartes: Vec<EcarteDto> = Vec::new();
        let mut vus: HashSet<Uuid> = HashSet::new();
        for e in eleves {
            if !vus.insert(*e) {
                continue;
            }
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(e)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if !existe || self.eleve_cache_au_tiers(etab, actor, role, *e).await? {
                ecartes.push(EcarteDto {
                    eleve_id: *e,
                    motif: "inconnu".into(),
                });
                continue;
            }
            if !est_interne_tx(conn, etab, *e, jour).await? {
                ecartes.push(EcarteDto {
                    eleve_id: *e,
                    motif: "plus_interne".into(),
                });
                continue;
            }
            retenus.push(*e);
        }
        Ok((retenus, ecartes))
    }

    async fn groupe_ajouter_encadrants(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        g: &GroupeLigne,
        users: &[Uuid],
        actor: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let jour = jour_local_tx(conn, etab, maintenant).await?;
        for u in users {
            if !est_personnel_tx(conn, etab, *u, jour).await? {
                return Err(refus(
                    "un accompagnateur est un compte du personnel de l'établissement",
                ));
            }
            let n = sqlx::query(
                "INSERT INTO ent_internat_sortie_groupe_encadrant (etablissement_id, sortie_groupe_id, user_id) \
                 VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            )
            .bind(etab)
            .bind(g.id)
            .bind(u)
            .execute(&mut *conn)
            .await
            .map_err(db)?
            .rows_affected();
            if n > 0 {
                fait_groupe_tx(
                    conn,
                    etab,
                    g.id,
                    None,
                    "encadrant_ajoute",
                    Some(actor),
                    maintenant,
                    None,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn groupe_inviter(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        g: &GroupeLigne,
        eleves: &[Uuid],
        actor: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        for e in eleves {
            let n = sqlx::query(
                "INSERT INTO ent_internat_sortie_groupe_participant (id, etablissement_id, sortie_groupe_id, eleve_id) \
                 VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(g.id)
            .bind(e)
            .execute(&mut *conn)
            .await
            .map_err(db)?
            .rows_affected();
            if n > 0 {
                fait_groupe_tx(
                    conn,
                    etab,
                    g.id,
                    Some(*e),
                    "invitation",
                    Some(actor),
                    maintenant,
                    None,
                )
                .await?;
            }
        }
        Ok(())
    }

    /// INT-170 / INT-178 : crée une sortie de groupe (brouillon). Les élèves qui ne sont plus internes ou qui sont
    /// inconnus sont écartés avec un motif, jamais invités.
    pub async fn creer_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &SortieGroupeSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        if b.eleves.len() > GROUPE_MAX_ELEVES || b.encadrants.len() > GROUPE_MAX_ENCADRANTS {
            return Err(refus("trop d'élèves ou d'accompagnateurs pour une sortie"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (titre, lieu) = valider_horaires(&b.titre, &b.lieu, b.debut, b.fin, maintenant)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_sortie_groupe (id, etablissement_id, titre, lieu, debut, fin, cree_par_user_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(&titre)
        .bind(&lieu)
        .bind(b.debut)
        .bind(b.fin)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            "creation",
            Some(actor),
            maintenant,
            None,
        )
        .await?;
        self.groupe_ajouter_encadrants(&mut tx, etab, &g, &b.encadrants, actor, maintenant)
            .await?;
        let jour = nuit_a_tx(&mut tx, etab, b.debut).await?;
        let (retenus, ecartes) = self
            .groupe_trier_invites(&mut tx, etab, actor, role, jour, &b.eleves)
            .await?;
        self.groupe_inviter(&mut tx, etab, &g, &retenus, actor, maintenant)
            .await?;
        evt_groupe_tx(
            &mut tx,
            etab,
            Some(actor),
            types::INTERNAT_SORTIE_GROUPE_CREEE,
            &g,
            "brouillon",
            "creation",
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, ecartes)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Modification d'un brouillon (titre, lieu, horaires) ; après la soumission, annuler et recréer.
    pub async fn modifier_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &SortieGroupeSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "brouillon" {
            return Err(conflit(
                "une sortie soumise ne se modifie plus : l'annuler et la recréer",
            ));
        }
        let (titre, lieu) = valider_horaires(&b.titre, &b.lieu, b.debut, b.fin, maintenant)?;
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe SET titre = $3, lieu = $4, debut = $5, fin = $6, modifie_le = $7 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&titre)
        .bind(&lieu)
        .bind(b.debut)
        .bind(b.fin)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            "modification",
            Some(actor),
            maintenant,
            None,
        )
        .await?;
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Ajoute des accompagnateurs (comptes du personnel) à une sortie non terminée et non commencée.
    pub async fn ajouter_encadrant_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        user: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "brouillon" | "soumise" | "acceptee")
            || g.debut <= maintenant
        {
            return Err(conflit(
                "cette sortie ne peut plus changer d'accompagnateurs",
            ));
        }
        if encadrants_tx(&mut tx, etab, id).await?.len() >= GROUPE_MAX_ENCADRANTS {
            return Err(refus("trop d'accompagnateurs"));
        }
        self.groupe_ajouter_encadrants(&mut tx, etab, &g, &[user], actor, maintenant)
            .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Retire un accompagnateur ; l'encadrement doit rester suffisant pour les élèves comptés.
    pub async fn retirer_encadrant_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        user: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "brouillon" | "soumise" | "acceptee")
            || g.debut <= maintenant
        {
            return Err(conflit(
                "cette sortie ne peut plus changer d'accompagnateurs",
            ));
        }
        let (par_encadrant, _) = parametres_vie_tx(&mut tx, etab).await?;
        let restants = encadrants_tx(&mut tx, etab, id)
            .await?
            .into_iter()
            .filter(|u| *u != user)
            .count();
        if g.statut != "brouillon"
            && !encadrement_suffisant(
                nb_comptes_tx(&mut tx, etab, id).await?,
                restants,
                par_encadrant,
            )
        {
            return Err(conflit("l'encadrement deviendrait insuffisant"));
        }
        let n = sqlx::query(
            "DELETE FROM ent_internat_sortie_groupe_encadrant WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND user_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            "encadrant_retire",
            Some(actor),
            maintenant,
            None,
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Invite d'autres internes à une sortie non terminée et non commencée. Après la soumission, chaque nouvel invité est
    /// aussitôt évalué (accord annuel, désaccord, attente de la famille) et l'encadrement est revérifié.
    pub async fn inviter_eleves_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ElevesAjoutDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        if b.eleves.len() > GROUPE_MAX_ELEVES {
            return Err(refus("trop d'élèves en une fois"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "brouillon" | "soumise" | "acceptee")
            || g.debut <= maintenant
        {
            return Err(conflit("cette sortie n'accepte plus d'invités"));
        }
        let jour = nuit_a_tx(&mut tx, etab, g.debut).await?;
        let (retenus, ecartes) = self
            .groupe_trier_invites(&mut tx, etab, actor, role, jour, &b.eleves)
            .await?;
        // Un élève déjà retiré ne revient pas : le trigger l'interdit (retiré = définitif pour cette sortie).
        for e in &retenus {
            if let Some(p) = participant_tx(&mut tx, etab, id, *e).await? {
                if p.1 == "retire" {
                    return Err(conflit("un élève retiré de cette sortie n'y revient pas : créer une nouvelle sortie"));
                }
            }
        }
        self.groupe_inviter(&mut tx, etab, &g, &retenus, actor, maintenant)
            .await?;
        if g.statut != "brouillon" {
            let (par_encadrant, _) = parametres_vie_tx(&mut tx, etab).await?;
            let nb_enc = encadrants_tx(&mut tx, etab, id).await?.len();
            let nouveaux: Vec<Uuid> = participants_tx(&mut tx, etab, id)
                .await?
                .into_iter()
                .filter(|p| p.1 == "invite")
                .map(|p| p.0)
                .collect();
            if !encadrement_suffisant(
                nb_comptes_tx(&mut tx, etab, id).await?,
                nb_enc,
                par_encadrant,
            ) {
                return Err(conflit("l'encadrement serait insuffisant pour ces élèves"));
            }
            let a_des_nouveaux = !nouveaux.is_empty();
            for e in nouveaux {
                let (etat, qualite, motif) =
                    evaluer_invitation_tx(self, &mut tx, etab, &g, e, jour).await?;
                appliquer_invitation_tx(
                    &mut tx, etab, &g, e, etat, qualite, motif, actor, maintenant,
                )
                .await?;
            }
            // M7 : des élèves ajoutés après l'acceptation remettent la sortie à la décision de la direction (la
            // décision précédente ne couvrait pas ces élèves) ; l'acceptation reste dans le journal des faits.
            if g.statut == "acceptee" && a_des_nouveaux {
                sqlx::query(
                    "UPDATE ent_internat_sortie_groupe SET statut = 'soumise', decide_par_user_id = NULL, decide_le = NULL, \
                            decision_motif_code = NULL, modifie_le = $3 WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_groupe_tx(
                    &mut tx,
                    etab,
                    id,
                    None,
                    "nouvelle_decision",
                    Some(actor),
                    maintenant,
                    None,
                )
                .await?;
                let g2 = groupe_tx(&mut tx, etab, id, false).await?;
                evt_groupe_tx(
                    &mut tx,
                    etab,
                    Some(actor),
                    types::INTERNAT_SORTIE_GROUPE_CREEE,
                    &g2,
                    "soumise",
                    "nouvelle_decision",
                )
                .await?;
            }
        }
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, ecartes)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Retire un élève de la sortie (motif « retiré par l'organisateur ») : définitif, tracé, sans suppression.
    pub async fn retirer_eleve_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "brouillon" | "soumise" | "acceptee") {
            return Err(conflit("cette sortie est terminée"));
        }
        let Some(p) = participant_tx(&mut tx, etab, id, eleve).await? else {
            return Err(DomainError::NotFound.into());
        };
        if matches!(p.1.as_str(), "parti" | "rentre") {
            return Err(conflit(
                "cet élève est déjà parti : le retour se pointe, il ne se retire pas",
            ));
        }
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe_participant SET etat = 'retire', motif_code = 'retire_par_organisateur', \
                    modifie_le = $4 WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(eleve)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            Some(eleve),
            "retrait",
            Some(actor),
            maintenant,
            Some("retire_par_organisateur"),
        )
        .await?;
        if g.statut != "brouillon" {
            evt_participant_tx(
                &mut tx,
                etab,
                Some(actor),
                &g,
                eleve,
                "retire",
                "retire_par_organisateur",
            )
            .await?;
        }
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-172 : soumet la sortie à la direction et interroge les familles. Exige un encadrement suffisant, au moins un
    /// invité, le préavis de l'établissement. Chaque invité est évalué (refus par défaut).
    pub async fn soumettre_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "brouillon" {
            return Err(conflit("cette sortie est déjà soumise"));
        }
        let (par_encadrant, preavis) = parametres_vie_tx(&mut tx, etab).await?;
        if g.debut < maintenant + Duration::hours(i64::from(preavis)) {
            return Err(conflit("le préavis de l'établissement n'est pas respecté : les familles doivent avoir le temps de répondre"));
        }
        let nb = nb_comptes_tx(&mut tx, etab, id).await?;
        let nb_enc = encadrants_tx(&mut tx, etab, id).await?.len();
        if nb == 0 {
            return Err(refus("aucun élève invité"));
        }
        if !encadrement_suffisant(nb, nb_enc, par_encadrant) {
            return Err(refus(
                "encadrement insuffisant : ajouter des accompagnateurs",
            ));
        }
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe SET statut = 'soumise', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            "soumission",
            Some(actor),
            maintenant,
            None,
        )
        .await?;
        let jour = nuit_a_tx(&mut tx, etab, g.debut).await?;
        for p in participants_tx(&mut tx, etab, id).await? {
            if p.1 == "invite" {
                let (etat, qualite, motif) =
                    evaluer_invitation_tx(self, &mut tx, etab, &g, p.0, jour).await?;
                appliquer_invitation_tx(
                    &mut tx, etab, &g, p.0, etat, qualite, motif, actor, maintenant,
                )
                .await?;
            }
        }
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        evt_groupe_tx(
            &mut tx,
            etab,
            Some(actor),
            types::INTERNAT_SORTIE_GROUPE_CREEE,
            &g,
            "soumise",
            "soumission",
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-171 : décision de la direction. Jamais l'auteur de la sortie (séparation des rôles) ; une direction de
    /// formation (portée désignée, R-IT30) ne décide que si TOUS les élèves comptés sont dans sa portée.
    pub async fn decider_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &DecisionGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatSortiesDecider])
            .await?;
        let motif = b.motif_code.as_deref();
        if !b.accepte && !motif.is_some_and(|m| MOTIFS_REFUS_DIRECTION.contains(&m)) {
            return Err(refus("un refus s'accompagne d'un motif codé (securite, encadrement, calendrier, reglement, autre)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "soumise" {
            return Err(conflit("cette sortie n'attend pas de décision"));
        }
        if g.cree_par == actor {
            return Err(refus("l'auteur d'une sortie ne la décide pas : une autre personne de la direction décide"));
        }
        let parts = participants_tx(&mut tx, etab, id).await?;
        if let Some(p) = &portee {
            if parts.iter().any(|x| !p.contains(&x.0)) {
                return Err(EntDbError::Forbidden);
            }
        }
        if b.accepte {
            let (par_encadrant, _) = parametres_vie_tx(&mut tx, etab).await?;
            let nb_enc = encadrants_tx(&mut tx, etab, id).await?.len();
            if !encadrement_suffisant(
                nb_comptes_tx(&mut tx, etab, id).await?,
                nb_enc,
                par_encadrant,
            ) {
                return Err(conflit(
                    "encadrement insuffisant : la sortie ne peut pas être acceptée",
                ));
            }
            if g.fin <= maintenant {
                return Err(conflit("cette sortie est terminée"));
            }
        }
        let nouveau = if b.accepte { "acceptee" } else { "refusee" };
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe SET statut = $3, decide_par_user_id = $4, decide_le = $5, \
                    decision_motif_code = $6, modifie_le = $5 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouveau)
        .bind(actor)
        .bind(maintenant)
        .bind(if b.accepte { None } else { motif })
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            if b.accepte {
                "acceptation"
            } else {
                "refus_direction"
            },
            Some(actor),
            maintenant,
            if b.accepte { None } else { motif },
        )
        .await?;
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        evt_groupe_tx(
            &mut tx,
            etab,
            Some(actor),
            types::INTERNAT_SORTIE_GROUPE_DECIDEE,
            &g,
            nouveau,
            if b.accepte { "acceptee" } else { "refusee" },
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, portee.as_ref(), Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-173 : annule une sortie (brouillon, soumise ou acceptée) tant qu'aucun élève n'est parti.
    pub async fn annuler_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AnnulationGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        if !MOTIFS_ANNULATION.contains(&b.motif_code.as_str()) {
            return Err(refus("motif d'annulation inconnu"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "brouillon" | "soumise" | "acceptee") {
            return Err(conflit("cette sortie est terminée"));
        }
        let parts = participants_tx(&mut tx, etab, id).await?;
        if parts
            .iter()
            .any(|p| matches!(p.1.as_str(), "parti" | "rentre"))
        {
            return Err(conflit(
                "des élèves sont partis : les faire rentrer puis clore la sortie",
            ));
        }
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe SET statut = 'annulee', annule_motif_code = $3, modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&b.motif_code)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            "annulation",
            Some(actor),
            maintenant,
            Some(&b.motif_code),
        )
        .await?;
        if g.statut != "brouillon" {
            // Chaque famille interrogée est prévenue, par un événement par élève (identifiants et code seulement).
            for p in parts
                .iter()
                .filter(|p| matches!(p.1.as_str(), "en_attente" | "autorise"))
            {
                evt_participant_tx(
                    &mut tx,
                    etab,
                    Some(actor),
                    &g,
                    p.0,
                    "annulee",
                    &b.motif_code,
                )
                .await?;
            }
        }
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        evt_groupe_tx(
            &mut tx,
            etab,
            Some(actor),
            types::INTERNAT_SORTIE_GROUPE_ANNULEE,
            &g,
            "annulee",
            &b.motif_code,
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-177 : duplique une sortie récurrente (piscine du mardi) : nouveau brouillon aux nouveaux horaires, mêmes
    /// accompagnateurs et mêmes invités ; AUCUN accord n'est repris (chaque sortie a ses propres réponses).
    pub async fn dupliquer_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &DuplicationGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let src = groupe_tx(&mut tx, etab, id, false).await?;
        let (titre, lieu) = valider_horaires(&src.titre, &src.lieu, b.debut, b.fin, maintenant)?;
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_sortie_groupe (id, etablissement_id, titre, lieu, debut, fin, cree_par_user_id, modele_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(nid)
        .bind(etab)
        .bind(&titre)
        .bind(&lieu)
        .bind(b.debut)
        .bind(b.fin)
        .bind(actor)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let g = groupe_tx(&mut tx, etab, nid, true).await?;
        fait_groupe_tx(
            &mut tx,
            etab,
            nid,
            None,
            "duplication",
            Some(actor),
            maintenant,
            None,
        )
        .await?;
        let enc: Vec<Uuid> = encadrants_tx(&mut tx, etab, id).await?;
        let enc_ok = {
            let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
            let mut v = Vec::new();
            for u in enc {
                if est_personnel_tx(&mut tx, etab, u, jour).await? {
                    v.push(u);
                }
            }
            v
        };
        self.groupe_ajouter_encadrants(&mut tx, etab, &g, &enc_ok, actor, maintenant)
            .await?;
        let invites: Vec<Uuid> = participants_tx(&mut tx, etab, id)
            .await?
            .into_iter()
            .map(|p| p.0)
            .collect();
        let jour = nuit_a_tx(&mut tx, etab, b.debut).await?;
        let (retenus, ecartes) = self
            .groupe_trier_invites(&mut tx, etab, actor, role, jour, &invites)
            .await?;
        self.groupe_inviter(&mut tx, etab, &g, &retenus, actor, maintenant)
            .await?;
        evt_groupe_tx(
            &mut tx,
            etab,
            Some(actor),
            types::INTERNAT_SORTIE_GROUPE_CREEE,
            &g,
            "brouillon",
            "duplication",
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, ecartes)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Réponse d'un titulaire (ou de l'élève majeur) pour UN élève. Le plus restrictif l'emporte : un refus est définitif
    /// et l'emporte sur un accord d'un autre titulaire ; un accord ne remplace jamais un refus ni un blocage.
    pub async fn repondre_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        eleve: Uuid,
        b: &ReponseGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeFamilleDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self
            .famille_tx(&mut tx, etab, actor, role, eleve, jour)
            .await?;
        let qualite = match famille {
            Some(
                f @ Famille {
                    qualite: Qualite::Titulaire | Qualite::EleveMajeur,
                    ..
                },
            ) if f.peut_autoriser => f.qualite,
            Some(Famille {
                qualite: Qualite::Titulaire,
                ..
            }) => {
                return Err(refus(
                    "votre lien avec l'élève ne vous permet pas d'autoriser une sortie",
                ))
            }
            _ => return Err(EntDbError::Forbidden),
        };
        // F1 : le droit (la qualité de titulaire de CET élève) est contrôlé avant l'existence de la sortie.
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "soumise" | "acceptee") {
            return Err(conflit("cette sortie n'attend pas de réponse"));
        }
        if maintenant >= g.debut {
            return Err(conflit("cette sortie a commencé"));
        }
        let Some(p) = participant_tx(&mut tx, etab, id, eleve).await? else {
            return Err(DomainError::NotFound.into());
        };
        match p.1.as_str() {
            "en_attente" | "autorise" => {}
            "refuse" if b.accord => {
                return Err(conflit(
                    "un refus de la famille est définitif pour cette sortie",
                ))
            }
            "refuse" => {}
            _ => return Err(conflit("cet élève ne peut pas répondre à cette sortie")),
        }
        if b.accord {
            if p.1 == "autorise" {
                // Idempotent : le premier accord reste (le plus restrictif l'emporte, un second accord n'ajoute rien).
            } else if desaccord_connu_tx(&mut tx, etab, eleve).await? {
                sqlx::query(
                    "UPDATE ent_internat_sortie_groupe_participant SET etat = 'non_autorise', motif_code = 'desaccord_connu', \
                            modifie_le = $4 WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
                )
                .bind(etab)
                .bind(id)
                .bind(eleve)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_groupe_tx(
                    &mut tx,
                    etab,
                    id,
                    Some(eleve),
                    "blocage_desaccord",
                    Some(actor),
                    maintenant,
                    Some("desaccord_connu"),
                )
                .await?;
                evt_participant_tx(
                    &mut tx,
                    etab,
                    Some(actor),
                    &g,
                    eleve,
                    "non_autorise",
                    "desaccord_connu",
                )
                .await?;
            } else {
                sqlx::query(
                    "UPDATE ent_internat_sortie_groupe_participant SET etat = 'autorise', reponse_qualite = $4, \
                            reponse_user_id = $5, reponse_le = $6, motif_code = NULL, modifie_le = $6 \
                      WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
                )
                .bind(etab)
                .bind(id)
                .bind(eleve)
                .bind(qualite.code())
                .bind(actor)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_groupe_tx(
                    &mut tx,
                    etab,
                    id,
                    Some(eleve),
                    "accord_famille",
                    Some(actor),
                    maintenant,
                    None,
                )
                .await?;
                evt_participant_tx(
                    &mut tx,
                    etab,
                    Some(actor),
                    &g,
                    eleve,
                    "autorise",
                    "accord_famille",
                )
                .await?;
            }
        } else if p.1 != "refuse" {
            let motif = if qualite == Qualite::EleveMajeur {
                "majeur_refuse"
            } else {
                "refus_famille"
            };
            sqlx::query(
                "UPDATE ent_internat_sortie_groupe_participant SET etat = 'refuse', reponse_qualite = $4, \
                        reponse_user_id = $5, reponse_le = $6, motif_code = $7, modifie_le = $6 \
                  WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(eleve)
            .bind(qualite.code())
            .bind(actor)
            .bind(maintenant)
            .bind(motif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            fait_groupe_tx(
                &mut tx,
                etab,
                id,
                Some(eleve),
                "refus_famille",
                Some(actor),
                maintenant,
                Some(motif),
            )
            .await?;
            evt_participant_tx(&mut tx, etab, Some(actor), &g, eleve, "refuse", motif).await?;
        }
        let dto = famille_dto_tx(&mut tx, etab, &g, eleve, true).await?;
        tx.commit().await.map_err(db)?;
        dto.ok_or_else(|| DomainError::NotFound.into())
    }

    /// INT-174 : enregistre un accord — ou un REFUS — reçu hors ligne, **avec auteur et date** (R-IT24, règle 5). La
    /// personne qui l'a donné doit être un titulaire de l'élève (autorité parentale, droit d'autoriser les sorties), non
    /// visée par une interdiction : la qualité de titulaire passe par **la même fonction** que la réponse en ligne
    /// (`titulaire_pour_tx`, règle M-A comprise). Un élève majeur répond lui-même (jamais d'accord hors ligne à sa place).
    pub async fn accord_hors_ligne_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        eleve: Uuid,
        b: &AccordHorsLigneDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        // Ordre des contrôles (règle 6) : personnel et droit, puis élève protégé introuvable, puis le reste.
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatSortiesDecider])
            .await?;
        if portee.as_ref().is_some_and(|p| !p.contains(&eleve)) {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, actor, role, eleve).await?;
        if !CANAUX.contains(&b.canal.as_str()) {
            return Err(refus("canal inconnu (telephone, papier, en_personne)"));
        }
        if b.recu_le > maintenant
            || b.recu_le < maintenant - Duration::days(HORS_LIGNE_ANCIENNETE_JOURS)
        {
            return Err(refus(
                "la date de réception de l'accord est invalide (ni future, ni de plus de 90 jours)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "soumise" | "acceptee") || maintenant >= g.debut {
            return Err(conflit("cette sortie n'attend plus d'accord"));
        }
        let Some(p) = participant_tx(&mut tx, etab, id, eleve).await? else {
            return Err(DomainError::NotFound.into());
        };
        // Un accord vaut pour un élève qui attend ; un refus (le plus restrictif l'emporte) aussi pour un élève déjà
        // autorisé par un autre titulaire.
        let admis = if b.accord {
            p.1 == "en_attente"
        } else {
            matches!(p.1.as_str(), "en_attente" | "autorise")
        };
        if !admis {
            return Err(conflit(
                "cet élève n'attend pas de réponse (déjà répondu, bloqué ou retiré)",
            ));
        }
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        if eleve_majeur_tx(&mut tx, etab, eleve, jour).await? {
            return Err(refus(
                "un élève majeur répond lui-même : aucun accord hors ligne à sa place",
            ));
        }
        if b.accord && desaccord_connu_tx(&mut tx, etab, eleve).await? {
            return Err(conflit(
                "désaccord connu entre les titulaires : renvoyé au CPE",
            ));
        }
        // Une seule fonction pour la qualité de titulaire. Refus par défaut ; la cause (non titulaire, interdite) n'est
        // jamais détaillée.
        let titulaire = self
            .titulaire_pour_tx(
                &mut tx,
                etab,
                QuiTitulaire::Personne(b.donne_par_personne_id),
                eleve,
                jour,
            )
            .await?;
        if !titulaire.is_some_and(|peut| peut) {
            return Err(refus("cet accord ne peut pas être enregistré"));
        }
        // F3 : la pièce justificative existe, dans cet établissement, et concerne cet élève ou cette personne.
        if let Some(piece) = b.piece_id {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2 \
                                  AND (eleve_id = $3 OR personne_id = $4))",
            )
            .bind(etab)
            .bind(piece)
            .bind(eleve)
            .bind(b.donne_par_personne_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(refus("pièce justificative inconnue pour cet élève"));
            }
        }
        sqlx::query(
            "INSERT INTO ent_internat_sortie_groupe_accord_hors_ligne \
             (id, etablissement_id, sortie_groupe_id, eleve_id, donne_par_personne_id, accord, recu_le, canal, \
              recu_par_user_id, piece_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(eleve)
        .bind(b.donne_par_personne_id)
        .bind(b.accord)
        .bind(b.recu_le)
        .bind(&b.canal)
        .bind(actor)
        .bind(b.piece_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if b.accord {
            sqlx::query(
                "UPDATE ent_internat_sortie_groupe_participant SET etat = 'autorise', reponse_qualite = 'hors_ligne', \
                        reponse_user_id = $4, reponse_personne_id = $5, reponse_le = $6, motif_code = NULL, modifie_le = $6 \
                  WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(eleve)
            .bind(actor)
            .bind(b.donne_par_personne_id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            sqlx::query(
                "UPDATE ent_internat_sortie_groupe_participant SET etat = 'refuse', reponse_qualite = 'hors_ligne', \
                        reponse_user_id = $4, reponse_personne_id = $5, reponse_le = $6, motif_code = 'refus_famille', \
                        modifie_le = $6 \
                  WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(eleve)
            .bind(actor)
            .bind(b.donne_par_personne_id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            Some(eleve),
            if b.accord {
                "accord_hors_ligne"
            } else {
                "refus_hors_ligne"
            },
            Some(actor),
            maintenant,
            Some(&b.canal),
        )
        .await?;
        evt_participant_tx(
            &mut tx,
            etab,
            Some(actor),
            &g,
            eleve,
            if b.accord { "autorise" } else { "refuse" },
            if b.accord {
                "accord_hors_ligne"
            } else {
                "refus_hors_ligne"
            },
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, portee.as_ref(), Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// L'accord en vigueur d'un participant tient-il toujours à l'instant du départ ? **Un acte tombe avec son auteur** :
    /// l'accord d'un titulaire n'est valable que si ce titulaire a encore l'autorité et n'est pas visé par une
    /// interdiction ; l'accord hors ligne que si la personne est encore titulaire et non interdite (même fonction) ;
    /// l'accord annuel que si le verdict du socle 18 est toujours « autorisé » ET que la sortie reste dans les bornes
    /// d'une soirée (règle 2). **Un accord de titulaire tombe à la majorité de l'élève** (F8) : l'élève répond alors
    /// lui-même, avant la sortie. Tout doute vaut refus. `None` : l'accord tient ; `Some(motif)` : il est tombé.
    async fn accord_tient_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        p: &PartBrute,
        g: &GroupeLigne,
        jour: NaiveDate,
    ) -> Result<Option<&'static str>, EntDbError> {
        let tombe = Some("interdiction");
        match p.2.as_deref() {
            Some(q @ ("titulaire" | "hors_ligne")) => {
                if eleve_majeur_tx(conn, etab, eleve, jour).await? {
                    return Ok(Some("majorite"));
                }
                let qui = match (q, p.3, p.4) {
                    ("titulaire", Some(user), _) => QuiTitulaire::Compte(user),
                    ("hors_ligne", _, Some(pers)) => QuiTitulaire::Personne(pers),
                    _ => return Ok(tombe),
                };
                let tient = self
                    .titulaire_pour_tx(conn, etab, qui, eleve, jour)
                    .await?
                    .is_some_and(|peut| peut);
                Ok(if tient { None } else { tombe })
            }
            Some("eleve_majeur") => Ok(if eleve_majeur_tx(conn, etab, eleve, jour).await? {
                None
            } else {
                tombe
            }),
            Some("annuelle") => {
                let tient = dans_les_bornes_soiree_tx(conn, etab, g).await?
                    && verdict_annuel(self, conn, etab, eleve, "sortie_soiree", jour).await
                        == "autorise";
                Ok(if tient { None } else { tombe })
            }
            _ => Ok(tombe),
        }
    }

    /// INT-176 : appel de DÉPART. Seuls les élèves `autorise` partent, et seulement si leur accord tient encore ; les
    /// autres reçoivent un résultat codé, jamais un départ silencieux. **Au moins un accompagnateur est pointé présent**
    /// (fait tracé) et l'encadrement se compte sur les accompagnateurs présents (M6). Le départ est raisonné en NUIT, pas
    /// en jour civil (F6). Un élève parti est aussitôt marqué « en activité » dans les appels ouverts de la nuit (règle 1).
    pub async fn depart_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AppelGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ResultatAppelDto>, EntDbError> {
        if b.eleves.len() > GROUPE_MAX_ELEVES {
            return Err(refus("trop d'élèves en un appel"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.groupe_appel_exiger(&mut tx, etab, actor, role, id, maintenant)
            .await?;
        let voit_la_cause = self.voit_la_cause(etab, actor, role).await?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "acceptee" {
            return Err(conflit(
                "seule une sortie acceptée par la direction donne lieu à un départ",
            ));
        }
        if maintenant < g.debut - Duration::minutes(DEPART_AVANCE_MINUTES) || maintenant >= g.fin {
            return Err(conflit("hors de la plage de départ de cette sortie"));
        }
        // F6 : la nuit (jour de la nuit en cours), jamais le jour civil : un départ à 00 h 30 appartient à la nuit d'hier.
        let nuit = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let jour_civil = jour_local_tx(&mut tx, etab, maintenant).await?;
        let (par_encadrant, _) = parametres_vie_tx(&mut tx, etab).await?;
        let nb_presents = self
            .pointer_encadrants_presents(
                &mut tx,
                etab,
                &g,
                actor,
                &b.encadrants_presents,
                jour_civil,
                maintenant,
            )
            .await?;
        let capacite = nb_presents * i64::from(par_encadrant);
        let mut deja_partis: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_sortie_groupe_participant \
              WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND etat = 'parti'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut vus: HashSet<Uuid> = HashSet::new();
        let mut resultats: Vec<ResultatAppelDto> = Vec::new();
        for e in &b.eleves {
            if !vus.insert(*e) {
                continue;
            }
            let res = |r: &str| ResultatAppelDto {
                eleve_id: *e,
                resultat: r.to_string(),
            };
            let Some(p) = participant_tx(&mut tx, etab, id, *e).await? else {
                resultats.push(res("pas_invite"));
                continue;
            };
            match p.1.as_str() {
                "parti" | "rentre" => {
                    resultats.push(res("deja_parti"));
                    continue;
                }
                "autorise" => {}
                _ => {
                    resultats.push(res("non_autorise"));
                    continue;
                }
            }
            if !est_interne_tx(&mut tx, etab, *e, nuit).await? {
                bloquer_participant_tx(
                    &mut tx,
                    etab,
                    &g,
                    *e,
                    "plus_interne",
                    "blocage_plus_interne",
                    actor,
                    maintenant,
                )
                .await?;
                resultats.push(res("non_autorise"));
                continue;
            }
            if desaccord_connu_tx(&mut tx, etab, *e).await? {
                bloquer_participant_tx(
                    &mut tx,
                    etab,
                    &g,
                    *e,
                    "desaccord_connu",
                    "blocage_desaccord",
                    actor,
                    maintenant,
                )
                .await?;
                resultats.push(res("non_autorise"));
                continue;
            }
            if let Some(motif) = self
                .accord_tient_tx(&mut tx, etab, *e, &p, &g, nuit)
                .await?
            {
                let acte = if motif == "majorite" {
                    "blocage_majorite"
                } else {
                    "accord_tombe"
                };
                bloquer_participant_tx(&mut tx, etab, &g, *e, motif, acte, actor, maintenant)
                    .await?;
                // E3 : seule la direction, le CPE et l'internat voient pourquoi ; l'accompagnateur lit « non autorisé ».
                resultats.push(res(if voit_la_cause {
                    "accord_tombe"
                } else {
                    "non_autorise"
                }));
                continue;
            }
            if deja_partis + 1 > capacite {
                resultats.push(res("encadrement_insuffisant"));
                continue;
            }
            sqlx::query(
                "UPDATE ent_internat_sortie_groupe_participant SET etat = 'parti', parti_le = $4, parti_par_user_id = $5, \
                        modifie_le = $4 WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(e)
            .bind(maintenant)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            deja_partis += 1;
            fait_groupe_tx(
                &mut tx,
                etab,
                id,
                Some(*e),
                "depart",
                Some(actor),
                maintenant,
                None,
            )
            .await?;
            publier(
                &mut tx,
                etab,
                types::INTERNAT_SORTIE_GROUPE_PRESENCE,
                AggregateKind::Internat,
                id,
                Some(*e),
                json!({ "fin": g.fin.to_rfc3339(), "etat": "parti" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
            // Règle 1 : l'appel du soir marque l'élève « en activité » (retour attendu), jamais « absent ».
            self.marquer_en_activite_tx(
                &mut tx,
                etab,
                actor,
                *e,
                g.fin,
                "sortie de groupe",
                maintenant,
            )
            .await?;
            resultats.push(res("ok"));
        }
        tx.commit().await.map_err(db)?;
        Ok(resultats)
    }

    /// M6 : pointe la présence des accompagnateurs au départ (fait tracé par accompagnateur, une fois) et renvoie le nombre
    /// d'accompagnateurs présents. L'acteur, s'il est accompagnateur, est présent d'office ; au moins un est exigé.
    #[allow(clippy::too_many_arguments)]
    async fn pointer_encadrants_presents(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        g: &GroupeLigne,
        actor: Uuid,
        listes: &[Uuid],
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<i64, EntDbError> {
        let encadrants = encadrants_tx(conn, etab, g.id).await?;
        let mut presents: Vec<Uuid> = listes.to_vec();
        if encadrants.contains(&actor) {
            presents.push(actor);
        }
        presents.sort();
        presents.dedup();
        for u in &presents {
            if !encadrants.contains(u) || !est_personnel_tx(conn, etab, *u, jour).await? {
                return Err(refus(
                    "un accompagnateur pointé présent est un accompagnateur de cette sortie, encore du personnel",
                ));
            }
        }
        if presents.is_empty() {
            return Err(conflit(
                "aucun accompagnateur pointé présent : pointer au moins un accompagnateur avant le départ",
            ));
        }
        for u in presents {
            let deja: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 \
                                  AND sortie_groupe_id = $2 AND acte = 'encadrant_present' AND autre_user_id = $3)",
            )
            .bind(etab)
            .bind(g.id)
            .bind(u)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if !deja {
                fait_groupe_autre_tx(
                    conn,
                    etab,
                    g.id,
                    None,
                    "encadrant_present",
                    Some(actor),
                    Some(u),
                    maintenant,
                    None,
                )
                .await?;
            }
        }
        // Présents = pointés présents ET toujours accompagnateurs de la sortie (un remplacé ne compte plus).
        sqlx::query_scalar(
            "SELECT count(DISTINCT f.autre_user_id) FROM ent_internat_sortie_groupe_fait f \
               JOIN ent_internat_sortie_groupe_encadrant e ON e.etablissement_id = f.etablissement_id \
                    AND e.sortie_groupe_id = f.sortie_groupe_id AND e.user_id = f.autre_user_id \
              WHERE f.etablissement_id = $1 AND f.sortie_groupe_id = $2 AND f.acte = 'encadrant_present'",
        )
        .bind(etab)
        .bind(g.id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
    }

    /// Appel de RETOUR : seuls les élèves partis rentrent. Le retour ferme le cas de nuit ouvert pour cette sortie.
    pub async fn retour_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AppelGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ResultatAppelDto>, EntDbError> {
        if b.eleves.len() > GROUPE_MAX_ELEVES {
            return Err(refus("trop d'élèves en un appel"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.groupe_appel_exiger(&mut tx, etab, actor, role, id, maintenant)
            .await?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "acceptee" {
            return Err(conflit("cette sortie n'est pas en cours"));
        }
        let mut vus: HashSet<Uuid> = HashSet::new();
        let mut resultats: Vec<ResultatAppelDto> = Vec::new();
        for e in &b.eleves {
            if !vus.insert(*e) {
                continue;
            }
            let res = |r: &str| ResultatAppelDto {
                eleve_id: *e,
                resultat: r.to_string(),
            };
            match participant_tx(&mut tx, etab, id, *e).await? {
                None => resultats.push(res("pas_invite")),
                Some(p) if p.1 == "parti" => {
                    sqlx::query(
                        "UPDATE ent_internat_sortie_groupe_participant SET etat = 'rentre', rentre_le = $4, \
                                rentre_par_user_id = $5, modifie_le = $4 \
                          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(e)
                    .bind(maintenant)
                    .bind(actor)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    fait_groupe_tx(
                        &mut tx,
                        etab,
                        id,
                        Some(*e),
                        "retour",
                        Some(actor),
                        maintenant,
                        None,
                    )
                    .await?;
                    publier(
                        &mut tx,
                        etab,
                        types::INTERNAT_SORTIE_GROUPE_PRESENCE,
                        AggregateKind::Internat,
                        id,
                        Some(*e),
                        json!({ "fin": g.fin.to_rfc3339(), "etat": "rentre" }),
                        Some(actor),
                        Causalite::default(),
                    )
                    .await?;
                    self.clore_cas_origine_tx(&mut tx, etab, actor, *e, Some(id), None, maintenant)
                        .await?;
                    resultats.push(res("ok"));
                }
                Some(p) if p.1 == "rentre" => resultats.push(res("deja_rentre")),
                Some(_) => resultats.push(res("pas_parti")),
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(resultats)
    }

    /// Règle 5 (tout fait se corrige par un fait) : un retour pointé par erreur s'annule par un fait **motivé** qui remet
    /// l'élève « parti » ; l'élève redevient « en activité » dans les appels ouverts et, s'il n'est toujours pas rentré à
    /// l'heure prévue, la tâche de veille rouvre son cas.
    pub async fn annuler_retour_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AnnulationRetourDto,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ResultatAppelDto>, EntDbError> {
        if !MOTIFS_ANNULATION_RETOUR.contains(&b.motif_code.as_str()) {
            return Err(refus(
                "un retour s'annule avec un motif codé (erreur_pointage, retour_non_effectif, autre)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.groupe_appel_exiger(&mut tx, etab, actor, role, id, maintenant)
            .await?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "acceptee" {
            return Err(conflit("cette sortie n'est pas en cours"));
        }
        let Some(p) = participant_tx(&mut tx, etab, id, b.eleve_id).await? else {
            return Err(DomainError::NotFound.into());
        };
        if p.1 != "rentre" {
            return Err(conflit("cet élève n'est pas pointé rentré"));
        }
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe_participant SET etat = 'parti', rentre_le = NULL, rentre_par_user_id = NULL, \
                    modifie_le = $4 WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(b.eleve_id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            Some(b.eleve_id),
            "retour_annule",
            Some(actor),
            maintenant,
            Some(&b.motif_code),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_SORTIE_GROUPE_PRESENCE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "fin": g.fin.to_rfc3339(), "etat": "parti" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        self.marquer_en_activite_tx(
            &mut tx,
            etab,
            actor,
            b.eleve_id,
            g.fin,
            "sortie de groupe",
            maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(vec![ResultatAppelDto {
            eleve_id: b.eleve_id,
            resultat: "parti".into(),
        }])
    }

    /// M6 : remplace un accompagnateur, y compris pendant la sortie (maladie, imprévu). L'ancien sort de la liste, le
    /// nouveau (compte du personnel) y entre ; un fait trace qui remplace qui. L'encadrement ne change pas.
    pub async fn remplacer_encadrant_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &RemplacementEncadrantDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        self.groupe_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if !matches!(g.statut.as_str(), "brouillon" | "soumise" | "acceptee")
            || maintenant >= g.fin + Duration::hours(6)
        {
            return Err(conflit(
                "cette sortie ne peut plus changer d'accompagnateurs",
            ));
        }
        if b.ancien_user_id == b.nouveau_user_id {
            return Err(refus(
                "l'accompagnateur de remplacement est une autre personne",
            ));
        }
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        if !est_personnel_tx(&mut tx, etab, b.nouveau_user_id, jour).await? {
            return Err(refus(
                "un accompagnateur est un compte du personnel de l'établissement",
            ));
        }
        let encadrants = encadrants_tx(&mut tx, etab, id).await?;
        if !encadrants.contains(&b.ancien_user_id) {
            return Err(DomainError::NotFound.into());
        }
        if encadrants.contains(&b.nouveau_user_id) {
            return Err(conflit("cet accompagnateur est déjà de la sortie"));
        }
        sqlx::query(
            "DELETE FROM ent_internat_sortie_groupe_encadrant WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND user_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(b.ancien_user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_internat_sortie_groupe_encadrant (etablissement_id, sortie_groupe_id, user_id) VALUES ($1,$2,$3)",
        )
        .bind(etab)
        .bind(id)
        .bind(b.nouveau_user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Le fait garde le NOUVEAU (autre_user_id) et l'auteur du remplacement ; l'ancien se lit sur le fait d'ajout.
        fait_groupe_autre_tx(
            &mut tx,
            etab,
            id,
            None,
            "encadrant_remplace",
            Some(actor),
            Some(b.nouveau_user_id),
            maintenant,
            None,
        )
        .await?;
        fait_groupe_autre_tx(
            &mut tx,
            etab,
            id,
            None,
            "encadrant_retire",
            Some(actor),
            Some(b.ancien_user_id),
            maintenant,
            Some("remplace"),
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-179 : clôture. Impossible tant qu'un élève parti n'est pas rentré (un élève jamais rentré n'est pas « oublié »).
    pub async fn clore_sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ClotureGroupeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        if !BILANS.contains(&b.bilan_code.as_str()) {
            return Err(refus("bilan inconnu (sans_incident, retard, incident)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.groupe_appel_exiger(&mut tx, etab, actor, role, id, maintenant)
            .await?;
        let g = groupe_tx(&mut tx, etab, id, true).await?;
        if g.statut != "acceptee" {
            return Err(conflit("seule une sortie acceptée se clôt"));
        }
        if maintenant < g.debut {
            return Err(conflit("cette sortie n'a pas commencé"));
        }
        let partis: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_sortie_groupe_participant \
              WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND etat = 'parti'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if partis > 0 {
            return Err(conflit(
                "des élèves partis ne sont pas rentrés : pointer leur retour avant de clore",
            ));
        }
        sqlx::query(
            "UPDATE ent_internat_sortie_groupe SET statut = 'close', close_le = $3, close_par_user_id = $4, \
                    bilan_code = $5, modifie_le = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(actor)
        .bind(&b.bilan_code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_groupe_tx(
            &mut tx,
            etab,
            id,
            None,
            "cloture",
            Some(actor),
            maintenant,
            Some(&b.bilan_code),
        )
        .await?;
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        evt_groupe_tx(
            &mut tx,
            etab,
            Some(actor),
            types::INTERNAT_SORTIE_GROUPE_CLOSE,
            &g,
            "close",
            &b.bilan_code,
        )
        .await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, None, Vec::new())
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Détail d'une sortie pour le personnel : l'organisateur, la direction, `internat.lire`, ou un accompagnateur de
    /// CETTE sortie. La lecture des élèves est bornée à la portée du droit (R-IT30) et journalisée (R-IT38).
    pub async fn sortie_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieGroupeDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let portee = self
            .groupe_lecture_portee(&mut tx, etab, actor, role, id, maintenant)
            .await?;
        let g = groupe_tx(&mut tx, etab, id, false).await?;
        let dto = self
            .groupe_dto_tx(&mut tx, etab, actor, role, &g, portee.as_ref(), Vec::new())
            .await?;
        // F5 : le journal de lecture nomme chaque élève lu (une ligne, la liste des identifiants), pas « une sortie ».
        let lus: Vec<Uuid> = dto.participants.iter().map(|p| p.eleve_id).collect();
        crate::internat_nuit::journaliser_lecture_eleves_tx(
            &mut tx, etab, actor, "sorties", None, None, &lus, maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Droit de lire UNE sortie : accompagnateur (voit tous les élèves de sa sortie), sinon portée du droit.
    async fn groupe_lecture_portee(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Option<HashSet<Uuid>>, EntDbError> {
        if est_encadrant_tx(conn, etab, id, actor, maintenant).await? {
            return Ok(None);
        }
        self.nuit_portee(
            etab,
            actor,
            role,
            &[
                Action::InternatLire,
                Action::InternatGerer,
                Action::InternatAppel,
            ],
        )
        .await
    }

    /// INT-175 : sorties d'une période (par NUIT : une sortie qui commence à 00 h 30 est celle de la nuit d'hier, F6), sans
    /// noms d'élèves.
    pub async fn lister_sorties_groupe_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        du: NaiveDate,
        au: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<SortieGroupeResumeDto>, EntDbError> {
        if au < du || (au - du).num_days() > 366 {
            return Err(refus("période invalide (366 jours au plus)"));
        }
        let a_le_droit = self
            .nuit_portee(
                etab,
                actor,
                role,
                &[
                    Action::InternatLire,
                    Action::InternatGerer,
                    Action::InternatAppel,
                ],
            )
            .await
            .is_ok();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let rows: Vec<(Uuid, String, String, DateTime<Utc>, DateTime<Utc>, String, i64, i64, i64, bool)> = sqlx::query_as(
            "SELECT g.id, g.titre, g.lieu, g.debut, g.fin, g.statut, \
                    count(p.*) FILTER (WHERE p.etat <> 'retire'), \
                    count(p.*) FILTER (WHERE p.etat IN ('autorise', 'parti', 'rentre')), \
                    count(p.*) FILTER (WHERE p.etat = 'en_attente'), \
                    EXISTS (SELECT 1 FROM ent_internat_sortie_groupe_encadrant e \
                             WHERE e.etablissement_id = g.etablissement_id AND e.sortie_groupe_id = g.id AND e.user_id = $4 \
                               AND g.fin + make_interval(days => $6::int) > $5 \
                               AND EXISTS (SELECT 1 FROM ent_membership m WHERE m.etablissement_id = e.etablissement_id \
                                             AND m.user_id = e.user_id AND m.etat = 'ouvert' AND m.role NOT IN ('parent', 'eleve') \
                                             AND (m.date_fin IS NULL OR m.date_fin >= $7))) \
               FROM ent_internat_sortie_groupe g \
               LEFT JOIN ent_internat_sortie_groupe_participant p \
                 ON p.etablissement_id = g.etablissement_id AND p.sortie_groupe_id = g.id \
              WHERE g.etablissement_id = $1 \
                AND ent_nuit_etab($1, g.debut) BETWEEN $2 AND $3 \
              GROUP BY g.id ORDER BY g.debut, g.id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(actor)
        .bind(maintenant)
        .bind(ENCADRANT_LECTURE_JOURS as i32)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        // Le droit voit tout (résumés, sans nom) ; un accompagnateur sans droit ne voit que SES sorties ; sans droit
        // ni sortie à accompagner : refusé.
        let lignes: Vec<SortieGroupeResumeDto> = rows
            .into_iter()
            .filter(|r| a_le_droit || r.9)
            .map(|r| SortieGroupeResumeDto {
                id: r.0,
                titre: r.1,
                lieu: r.2,
                debut: r.3,
                fin: r.4,
                statut: r.5,
                nb_participants: r.6,
                nb_autorises: r.7,
                nb_en_attente: r.8,
            })
            .collect();
        if lignes.is_empty() && !a_le_droit {
            return Err(EntDbError::Forbidden);
        }
        Ok(lignes)
    }

    /// Ce que voit une famille (ou l'élève lui-même) : SON enfant, jamais un autre participant (R-IT32). Un parent
    /// interdit de remise (C-1), ou sans droit de lecture, ne lit rien.
    pub async fn sorties_groupe_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<SortieGroupeFamilleDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let Some(f) = self
            .famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour)
            .await?
        else {
            return Err(EntDbError::Forbidden);
        };
        let peut =
            f.peut_autoriser && matches!(f.qualite, Qualite::Titulaire | Qualite::EleveMajeur);
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT g.id FROM ent_internat_sortie_groupe g \
               JOIN ent_internat_sortie_groupe_participant p ON p.etablissement_id = g.etablissement_id AND p.sortie_groupe_id = g.id \
              WHERE g.etablissement_id = $1 AND p.eleve_id = $2 AND g.statut <> 'brouillon' \
                AND p.etat NOT IN ('invite', 'retire') AND g.fin > $3 - interval '30 days' \
              ORDER BY g.debut, g.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for id in ids {
            let g = groupe_tx(&mut tx, etab, id, false).await?;
            if let Some(mut d) = famille_dto_tx(&mut tx, etab, &g, eleve, peut).await? {
                d.peut_repondre = d.peut_repondre && maintenant < g.debut;
                out.push(d);
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }
}

impl PgEnt {
    /// Règle 1 : marque « en activité » (retour attendu) un élève parti, dans chaque appel OUVERT de la nuit courante qui
    /// l'attend (jamais « absent » : l'élève est sous la responsabilité d'un accompagnateur). Un élève déjà marqué « en
    /// activité » n'est pas remarqué ; un élève hors liste de l'appel est ignoré.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn marquer_en_activite_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        eleve: Uuid,
        retour: DateTime<Utc>,
        note: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let nuit = nuit_a_tx(conn, etab, maintenant).await?;
        let appels: Vec<Uuid> = sqlx::query_scalar(
            "SELECT a.id FROM ent_internat_appel a JOIN ent_internat_appel_attendu x \
                    ON x.etablissement_id = a.etablissement_id AND x.appel_id = a.id \
              WHERE a.etablissement_id = $1 AND a.nuit = $2 AND a.etat = 'ouvert' AND x.eleve_id = $3 \
                AND x.statut IN ('attendu', 'ajoute') ORDER BY a.id",
        )
        .bind(etab)
        .bind(nuit)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for id in appels {
            let appel = crate::internat_nuit::appel_tx(conn, etab, id, "FOR UPDATE").await?;
            let deja = crate::internat_nuit::marques_courantes_tx(conn, etab, id)
                .await?
                .get(&eleve)
                .is_some_and(|m| m.marque == "activite");
            if deja {
                continue;
            }
            self.pointer_tx(
                conn,
                etab,
                actor,
                &appel,
                &ent_types::internat_nuit::PointageSaisieDto {
                    eleve_id: eleve,
                    marque: "activite".into(),
                    note: Some(note.to_string()),
                    retour_attendu: Some(retour),
                    a: None,
                    appareil_id: None,
                    motif_correction_code: None,
                    motif_correction_texte: None,
                },
                maintenant,
            )
            .await?;
        }
        Ok(())
    }

    /// Règle 1, à l'ouverture d'un appel : les élèves attendus qui sont PARTIS (sortie de groupe acceptée, activité du soir
    /// pointée « en activité ») sont marqués « en activité » avec leur retour attendu : l'appel ne les compte pas absents.
    pub(crate) async fn marquer_en_activite_ouverture_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        appel_id: Uuid,
        eleves: &[Uuid],
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        if eleves.is_empty() {
            return Ok(());
        }
        let lignes: Vec<(Uuid, DateTime<Utc>, String)> = sqlx::query_as(
            "SELECT DISTINCT ON (eleve_id) eleve_id, retour, note FROM ( \
               SELECT p.eleve_id, g.fin AS retour, 'sortie de groupe'::text AS note \
                 FROM ent_internat_sortie_groupe_participant p \
                 JOIN ent_internat_sortie_groupe g ON g.etablissement_id = p.etablissement_id AND g.id = p.sortie_groupe_id \
                WHERE p.etablissement_id = $1 AND p.etat = 'parti' AND g.statut = 'acceptee' AND p.eleve_id = ANY($2) \
               UNION ALL \
               SELECT tp.eleve_id, t.fin, 'activité du soir'::text \
                 FROM ent_internat_temps_participant tp \
                 JOIN ent_internat_temps t ON t.etablissement_id = tp.etablissement_id AND t.id = tp.temps_id \
                WHERE tp.etablissement_id = $1 AND tp.pointage = 'present' AND tp.rentre_le IS NULL AND tp.retire_le IS NULL \
                  AND t.nature = 'activite' AND t.statut = 'prevu' AND tp.eleve_id = ANY($2)) x \
              ORDER BY eleve_id, retour",
        )
        .bind(etab)
        .bind(eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        if lignes.is_empty() {
            return Ok(());
        }
        let appel = crate::internat_nuit::appel_tx(conn, etab, appel_id, "FOR UPDATE").await?;
        for (eleve, retour, note) in lignes {
            self.pointer_tx(
                conn,
                etab,
                actor,
                &appel,
                &ent_types::internat_nuit::PointageSaisieDto {
                    eleve_id: eleve,
                    marque: "activite".into(),
                    note: Some(note),
                    retour_attendu: Some(retour),
                    a: None,
                    appareil_id: None,
                    motif_correction_code: None,
                    motif_correction_texte: None,
                },
                maintenant,
            )
            .await?;
        }
        Ok(())
    }

    /// Le retour d'un élève parti ferme le cas de nuit que son absence avait ouvert (issue « retrouvé », lieu extérieur).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn clore_cas_origine_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        eleve: Uuid,
        sortie: Option<Uuid>,
        temps: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let cas: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "UPDATE ent_internat_cas SET issue = 'retrouve', issue_le = $5, issue_par_user_id = $6, \
                    issue_lieu_code = 'exterieur', modifie_le = now() \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND issue IS NULL \
                AND (($3::uuid IS NOT NULL AND sortie_groupe_id = $3) OR ($4::uuid IS NOT NULL AND temps_id = $4)) \
          RETURNING id, nuit",
        )
        .bind(etab)
        .bind(eleve)
        .bind(sortie)
        .bind(temps)
        .bind(maintenant)
        .bind(actor)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((cas, nuit)) = cas {
            publier(
                conn,
                etab,
                types::INTERNAT_CAS_CLOS,
                AggregateKind::Internat,
                cas,
                Some(eleve),
                json!({ "nuit": nuit.to_string(), "issue": "retrouve" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        Ok(())
    }

    /// Règle 1 (tâche périodique, pas un point de lecture) : tout élève « parti » d'une sortie de groupe ou en activité du
    /// soir et non rentré à l'heure de fin prévue + tolérance de l'établissement ouvre **automatiquement** un cas de nuit
    /// (procédure d'I3, nature « non rentré ») qui garde la sortie ou le temps d'origine et l'heure prévue ; les
    /// accompagnateurs à joindre se lisent sur la sortie. Idempotent (un seul cas ouvert par élève et par nuit). Renvoie
    /// le nombre de cas ouverts.
    pub async fn veille_non_rentres_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let tolerance: i16 = {
            let _ = parametres_vie_tx(&mut tx, etab).await?;
            sqlx::query_scalar(
                "SELECT groupe_tolerance_retour_minutes FROM ent_internat_parametres_vie WHERE etablissement_id = $1",
            )
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        };
        let limite = maintenant - Duration::minutes(i64::from(tolerance));
        // (élève, sortie, temps, heure prévue) : sorties de groupe acceptées dont un élève est encore « parti », activités
        // du soir où l'élève est pointé « en activité » sans retour.
        let lignes: Vec<(Uuid, Option<Uuid>, Option<Uuid>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT p.eleve_id, g.id, NULL::uuid, g.fin FROM ent_internat_sortie_groupe_participant p \
               JOIN ent_internat_sortie_groupe g ON g.etablissement_id = p.etablissement_id AND g.id = p.sortie_groupe_id \
              WHERE p.etablissement_id = $1 AND p.etat = 'parti' AND g.statut = 'acceptee' AND g.fin < $2 \
             UNION ALL \
             SELECT tp.eleve_id, NULL::uuid, t.id, t.fin FROM ent_internat_temps_participant tp \
               JOIN ent_internat_temps t ON t.etablissement_id = tp.etablissement_id AND t.id = tp.temps_id \
              WHERE tp.etablissement_id = $1 AND tp.pointage = 'present' AND tp.rentre_le IS NULL AND tp.retire_le IS NULL \
                AND t.nature = 'activite' AND t.statut = 'prevu' AND t.fin < $2 \
              ORDER BY 4, 1",
        )
        .bind(etab)
        .bind(limite)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let nuit = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let mut ouverts = 0;
        for (eleve, sortie, temps, prevue) in lignes {
            if let Some(cas) = self
                .ouvrir_cas_tx(
                    &mut tx,
                    etab,
                    None,
                    None,
                    eleve,
                    nuit,
                    "non_rentre",
                    maintenant,
                )
                .await?
            {
                sqlx::query(
                    "UPDATE ent_internat_cas SET sortie_groupe_id = $3, temps_id = $4, heure_prevue = $5 \
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(cas)
                .bind(sortie)
                .bind(temps)
                .bind(prevue)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                ouverts += 1;
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(ouverts)
    }
}

/// Un participant bloqué au départ : écriture + fait + événement. La cause `interdiction` n'est jamais relayée.
#[allow(clippy::too_many_arguments)]
async fn bloquer_participant_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    g: &GroupeLigne,
    eleve: Uuid,
    motif: &str,
    acte: &str,
    actor: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_internat_sortie_groupe_participant SET etat = 'non_autorise', motif_code = $4, modifie_le = $5 \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(g.id)
    .bind(eleve)
    .bind(motif)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    fait_groupe_tx(
        conn,
        etab,
        g.id,
        Some(eleve),
        acte,
        Some(actor),
        maintenant,
        Some(motif),
    )
    .await?;
    let cause = if motif == "interdiction" {
        "interdiction_auteur"
    } else {
        motif
    };
    evt_participant_tx(conn, etab, Some(actor), g, eleve, "non_autorise", cause).await
}

/// Vue famille d'un participant (jamais la cause d'un blocage, jamais les autres participants).
pub(super) async fn famille_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    g: &GroupeLigne,
    eleve: Uuid,
    peut: bool,
) -> Result<Option<SortieGroupeFamilleDto>, EntDbError> {
    let etat: Option<String> = sqlx::query_scalar(
        "SELECT etat FROM ent_internat_sortie_groupe_participant \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(g.id)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(etat) = etat.filter(|e| !matches!(e.as_str(), "invite" | "retire")) else {
        return Ok(None);
    };
    let attend = matches!(etat.as_str(), "en_attente" | "autorise")
        && matches!(g.statut.as_str(), "soumise" | "acceptee");
    Ok(Some(SortieGroupeFamilleDto {
        sortie_id: g.id,
        eleve_id: eleve,
        titre: g.titre.clone(),
        lieu: g.lieu.clone(),
        debut: g.debut,
        fin: g.fin,
        statut: g.statut.clone(),
        etat,
        peut_repondre: peut && attend,
    }))
}

#[cfg(test)]
mod tests_unitaires {
    use super::encadrement_suffisant;

    #[test]
    fn l_encadrement_se_calcule_par_accompagnateur() {
        assert!(encadrement_suffisant(0, 0, 12));
        assert!(
            !encadrement_suffisant(1, 0, 12),
            "un élève sans accompagnateur : insuffisant"
        );
        assert!(encadrement_suffisant(12, 1, 12));
        assert!(!encadrement_suffisant(13, 1, 12));
        assert!(encadrement_suffisant(24, 2, 12));
    }
}
