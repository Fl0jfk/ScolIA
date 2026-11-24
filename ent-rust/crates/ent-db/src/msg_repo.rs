use chrono::{DateTime, Utc};
use domain_eleve::{
    DomainError, Message, MsgAccuse, MsgFil, MsgModele, MsgPiece, MsgPieceMeta, MsgPriorite,
};
use sqlx::PgPool;

use crate::tenant_tx::TenantTx;
use uuid::Uuid;

/// Issue de l'annulation d'un message différé (E1) : totale, ou refusée avec une raison exploitable par l'API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnulationDiffere {
    /// Message supprimé (avec ses versions, cartes, accès, lectures, accusés et pièces) ; clés d'objets à détruire.
    Annule(Vec<String>),
    /// Le message est déjà remis (ou absent, ou d'un autre auteur) : l'annulation n'a pas lieu.
    DejaRemis,
    /// La base a refusé la suppression (clé étrangère ou garde en ajout seul) : rien n'est supprimé, le message part
    /// normalement. L'appelant le dit à l'auteur ; jamais d'erreur technique opaque.
    Refusee,
}

/// Issue d'un dépôt de pièce sous verrou (F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionPiece {
    Inseree,
    /// La limite de pièces par message est atteinte.
    Plein,
    MessageRetire,
    MessageDiffere,
}

/// Fenêtre maximale d'un curseur du flux M12 (secondes) : au-delà, le client doit resynchroniser tout.
pub const FLUX_FENETRE_SECONDES: u64 = 600;
/// M-2 : écart maximal toléré entre l'horloge de la base et l'`emis` d'un curseur (deux nœuds de lecture).
/// M-2 : nombre d'identifiants de transaction au plus qu'un curseur légitime peut avoir de retard sur la borne
/// courante (la fenêtre de 600 s à plusieurs milliers de transactions par seconde reste bien en dessous).
pub const FLUX_MARGE_XID: u64 = 50_000_000;
pub const FLUX_TOLERANCE_HORLOGE_SECONDES: i64 = 5;

/// Position de reprise du flux M12 (M1). **Opaque pour le client** : il la rend telle quelle (`apres`).
///
/// Deux flux d'événements, chacun avec sa propre position, parce qu'ils n'ont pas la même cause :
/// - **immédiats** (message remis à l'écriture, retrait) : ordre `(xid, fil, genre)` où `xid` est l'identifiant (`xid8`)
///   de la transaction qui a écrit la ligne. Tout est servi sous la borne `pg_snapshot_xmin(pg_current_snapshot())` :
///   une transaction d'identifiant inférieur est terminée, donc rien ne peut plus apparaître sous la borne (aucune
///   horloge, aucune lecture du catalogue). Position « rattrapée » : `(borne, nil, "")` ;
/// - **différés** (message dont la remise est programmée) : l'événement naît de l'**échéance** `remise_prevue_le`, pas
///   d'une validation ; ordre `(instant, fil)`, la position est celle du dernier événement servi.
///
/// `emis` (heure de la base à l'émission) ne sert qu'à borner l'âge du curseur (resynchronisation), jamais à ordonner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionFlux {
    pub xid: u64,
    pub fil: Option<Uuid>,
    /// `nouveau` ou `retire` (avec `fil`) ; `None` : position rattrapée.
    pub genre: Option<String>,
    pub differe: DateTime<Utc>,
    pub differe_fil: Option<Uuid>,
    pub emis: DateTime<Utc>,
    /// F-1 : identité du cluster PostgreSQL qui a émis le curseur (`system_identifier`). Les `xid` sont propres à un
    /// cluster : après une restauration logique (`pg_dump`/`pg_restore`) ou un clone, ceux d'un curseur ancien n'ont
    /// plus de sens ; un curseur d'un autre cluster est ramené au point de resynchronisation.
    pub cluster: i64,
}

impl PositionFlux {
    const PREFIXE: &'static str = "v3";

    /// Forme transmise au client (opaque) : `v3.<xid>.<fil|->.<n|r|->.<µs>.<fil|->.<µs>.<cluster>`.
    pub fn encoder(&self) -> String {
        let fil = |f: &Option<Uuid>| f.map_or_else(|| "-".to_string(), |u| u.to_string());
        let genre = match self.genre.as_deref() {
            Some("nouveau") => "n",
            Some("retire") => "r",
            _ => "-",
        };
        format!(
            "{}.{}.{}.{}.{}.{}.{}.{}",
            Self::PREFIXE,
            self.xid,
            fil(&self.fil),
            genre,
            self.differe.timestamp_micros(),
            fil(&self.differe_fil),
            self.emis.timestamp_micros(),
            self.cluster
        )
    }

    /// Lit un curseur ; `None` si le format est inconnu (ancien format, valeur fabriquée) : le client resynchronise.
    pub fn decoder(texte: &str) -> Option<Self> {
        let p: Vec<&str> = texte.split('.').collect();
        if p.len() != 8 || p[0] != Self::PREFIXE {
            return None;
        }
        let fil = |t: &str| -> Option<Option<Uuid>> {
            if t == "-" { Some(None) } else { Uuid::parse_str(t).ok().map(Some) }
        };
        let genre = match p[3] {
            "n" => Some("nouveau".to_string()),
            "r" => Some("retire".to_string()),
            "-" => None,
            _ => return None,
        };
        let instant = |t: &str| DateTime::<Utc>::from_timestamp_micros(t.parse::<i64>().ok()?);
        Some(Self {
            xid: p[1].parse().ok()?,
            fil: fil(p[2])?,
            genre,
            differe: instant(p[4])?,
            differe_fil: fil(p[5])?,
            emis: instant(p[6])?,
            cluster: p[7].parse().ok()?,
        })
    }

    /// Position de départ : tout ce qui est antérieur à la borne `xid` et à `maintenant` est considéré comme connu.
    pub fn depart(xid: u64, maintenant: DateTime<Utc>, cluster: i64) -> Self {
        Self { xid, fil: None, genre: None, differe: maintenant, differe_fil: None, emis: maintenant, cluster }
    }
}

/// Résultat brut du flux M12 (avant le jugement de lecture) ; voir [`PgMsgRepo::evenements_depuis`].
#[derive(Debug, Clone)]
pub struct EvenementsFlux {
    /// Position à rendre au prochain appel.
    pub position: PositionFlux,
    pub tronque: bool,
    /// Curseur plus ancien que la fenêtre maximale ou illisible : rien n'est rendu, le client recharge tout.
    pub resynchro: bool,
    /// `(fil, instant, genre)`, genre = `nouveau` ou `retire`.
    pub evenements: Vec<(Uuid, DateTime<Utc>, String)>,
}

/// Identité du cluster (`pg_control_system().system_identifier`, exécutable par tout rôle), lue **une fois** par
/// processus puis gardée : elle ne change qu'avec un autre cluster (restauration logique, `pg_upgrade`), jamais en cours
/// de vie d'un processus branché sur la même base ; une lecture par appel du flux était superflue.
static CLUSTER_ID: std::sync::OnceLock<i64> = std::sync::OnceLock::new();

/// Heure de la base et borne du curseur : `pg_snapshot_xmin(pg_current_snapshot())`, l'identifiant de la plus ancienne
/// transaction **encore ouverte** (toute transaction d'identifiant inférieur est terminée). Les deux dans la même
/// transaction : c'est la base (`TenantTx`, donc le contexte d'établissement), aucune lecture du catalogue sauf
/// l'identité du cluster (première lecture du processus).
async fn lire_borne(tx: &mut TenantTx<'_>) -> Result<(DateTime<Utc>, u64, i64), DomainError> {
    let (maintenant, xmin, xmax): (DateTime<Utc>, String, String) = sqlx::query_as(
        "SELECT now(), pg_snapshot_xmin(pg_current_snapshot())::text,
                pg_snapshot_xmax(pg_current_snapshot())::text",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx)?;
    let cluster = match CLUSTER_ID.get() {
        Some(c) => *c,
        None => {
            let (c,): (i64,) = sqlx::query_as("SELECT system_identifier FROM pg_control_system()")
                .fetch_one(&mut **tx)
                .await
                .map_err(map_sqlx)?;
            *CLUSTER_ID.get_or_init(|| c)
        }
    };
    let xmin = xmin.parse::<u64>().map_err(|_| DomainError::ReferenceInconnue)?;
    if let Ok(xmax) = xmax.parse::<u64>() {
        crate::msg_xmin_gele::surveiller(xmin, xmax, maintenant);
    }
    Ok((maintenant, xmin, cluster))
}

/// M-2 : un curseur reçu est **borné** avant tout usage. Hors bornes, il n'est pas cru : il est ramené au point de
/// resynchronisation (le client recharge tout et repart d'un curseur sain), jamais exécuté. Un refus 400 aurait
/// distingué « faux » de « trop ancien » pour rien : le client, dans les deux cas, doit recharger.
/// - cluster : celui de la base courante (F-1) ;
/// - `emis` : ni dans le futur (tolérance [`FLUX_TOLERANCE_HORLOGE_SECONDES`]) ni plus ancien que la fenêtre ;
/// - `xid` : dans `[xmin − FLUX_MARGE_XID, xmin]` (la borne ne recule jamais : un curseur légitime est ≤ xmin) ;
/// - `differe` : dans `[maintenant − 2 × fenêtre, maintenant]`.
pub(crate) fn curseur_borne(apres: &PositionFlux, maintenant: DateTime<Utc>, xmin: u64, cluster: i64) -> bool {
    let fenetre = chrono::Duration::seconds(FLUX_FENETRE_SECONDES as i64);
    let tolerance = chrono::Duration::seconds(FLUX_TOLERANCE_HORLOGE_SECONDES);
    apres.cluster == cluster
        && apres.emis <= maintenant + tolerance
        && apres.emis >= maintenant - fenetre
        && apres.xid <= xmin
        && apres.xid >= xmin.saturating_sub(FLUX_MARGE_XID)
        && apres.differe <= maintenant + tolerance
        && apres.differe >= maintenant - fenetre * 2
}

/// Prédicat de lisibilité d'un fil pour un personnel (alias `f`) ; paramètres `$3` = compte,
/// `$4` = `reserve_ok`, `$5` = `foyer_ok` (voir [`PgMsgRepo::fils_lisibles_personnel`]).
///
/// M1 (R-M1) : **lire = participer**. Le premier terme est la participation
/// (`ent_msg_fil_lisible`, l'unique vue de lecture) ; elle couvre l'ouvreur et tout personnel qui a
/// écrit. Les trois termes suivants sont le **pont des rôles réservés**, provisoire jusqu'à M3
/// (boîtes de service) : un personnel qui détient le droit lit encore, sans être participant, les
/// fils de vie scolaire, les relances (`foyer_ok`) et les fils réservés (`reserve_ok`). Dès qu'il y
/// écrit, il devient participant. M3 : **jamais** un fil de boîte (`nature = 'boite'`) ni un fil C3 par ce
/// pont : un fil de boîte n'est lu que par ses membres (R-M1, R-M11), la supervision motivée est M10.
const PREDICAT_LISIBLE_PERSONNEL: &str = "(
       f.id IN (SELECT l.fil_id FROM ent_msg_fil_lisible($1, $3) l)
    OR (f.nature <> 'boite' AND f.sensibilite <> 'C3' AND (
           (f.foyer_id IS NULL AND f.vie_scolaire)
        OR (f.foyer_id IS NOT NULL AND $5)
        OR (f.foyer_id IS NULL AND NOT f.vie_scolaire AND $4))))";

/// Échappe `\`, `%` et `_` d'une recherche libre pour `ILIKE ... ESCAPE '\'`.
fn echapper_like(r: &str) -> String {
    r.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Curseur de pagination de la boîte : position du dernier fil rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurseurFils {
    /// 0 = priorité haute, 1 = normale.
    pub rang: i16,
    pub dernier_message: DateTime<Utc>,
    pub fil_id: Uuid,
}

impl CurseurFils {
    pub fn de_fil(f: &MsgFil) -> Self {
        Self {
            rang: f.priorite.rang(),
            dernier_message: f.last_message_at,
            fil_id: f.id,
        }
    }

    /// Forme texte opaque pour l'API : `rang|horodatage|id`.
    pub fn encoder(&self) -> String {
        format!(
            "{}|{}|{}",
            self.rang,
            self.dernier_message.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            self.fil_id
        )
    }

    pub fn decoder(s: &str) -> Option<Self> {
        let mut p = s.split('|');
        let rang: i16 = p.next()?.parse().ok()?;
        let dernier_message = DateTime::parse_from_rfc3339(p.next()?).ok()?.with_timezone(&Utc);
        let fil_id = Uuid::parse_str(p.next()?).ok()?;
        if p.next().is_some() || !(0..=1).contains(&rang) {
            return None;
        }
        Some(Self { rang, dernier_message, fil_id })
    }
}

/// Filtres d'une page de la boîte d'un personnel.
#[derive(Debug, Clone, Default)]
pub struct FiltreFils<'a> {
    pub eleve_id: Option<Uuid>,
    /// Périmètre d'un professeur (`None` = tout l'établissement).
    pub eleves: Option<&'a [Uuid]>,
    pub avec_archives: bool,
    pub dossier: Option<&'a str>,
    pub recherche: Option<&'a str>,
    pub apres: Option<CurseurFils>,
    pub limite: i64,
}

/// Fil en attente de réponse du personnel (MSG-329).
#[derive(Debug, Clone)]
pub struct FilNonTraite {
    pub fil: MsgFil,
    pub en_attente_depuis: DateTime<Utc>,
}

pub struct PgMsgRepo {
    pool: PgPool,
}

/// Portée de lecture d'un fil à sa création (M0a, D3 et D5).
#[derive(Debug, Clone, Copy)]
pub struct PorteeFil {
    pub prive_user_id: Option<Uuid>,
    pub foyer_id: Option<Uuid>,
    /// `responsables` (défaut), `famille` ou `eleve`.
    pub audience: &'static str,
    pub vie_scolaire: bool,
    /// M0b (D22) : foyer lecteur d'un fil du personnel « aux responsables » (un fil par foyer).
    /// Distinct de `foyer_id` (payeurs d'une relance).
    pub foyer_lecteur_id: Option<Uuid>,
}

impl PorteeFil {
    /// Fil de l'établissement aux responsables, invisible de l'élève.
    pub const fn responsables() -> Self {
        Self {
            prive_user_id: None,
            foyer_id: None,
            audience: "responsables",
            vie_scolaire: false,
            foyer_lecteur_id: None,
        }
    }

    /// Fil du personnel aux responsables d'**un** foyer (D22) : lu des seuls liens communicants
    /// de ce foyer, jamais de l'autre foyer.
    pub const fn responsables_du_foyer(foyer_lecteur_id: Uuid) -> Self {
        Self { foyer_lecteur_id: Some(foyer_lecteur_id), ..Self::responsables() }
    }

    /// Fil du personnel à l'élève majeur (D23) : l'élève le lit, jamais un parent.
    pub const fn eleve_majeur() -> Self {
        Self { audience: "eleve", ..Self::responsables() }
    }

    /// Fil de vie scolaire (carnet, alerte d'absences) aux responsables ayant le droit.
    pub const fn vie_scolaire() -> Self {
        Self { vie_scolaire: true, ..Self::responsables() }
    }

    /// Fil de relance de facture, réservé aux payeurs du foyer.
    pub const fn foyer(foyer_id: Uuid) -> Self {
        Self { foyer_id: Some(foyer_id), ..Self::responsables() }
    }

    /// Fil ouvert par un responsable : privé entre lui et l'établissement.
    pub const fn prive_parent(user_id: Uuid) -> Self {
        Self { prive_user_id: Some(user_id), ..Self::responsables() }
    }

    /// Fil ouvert par un élève : privé à l'élève et au personnel, jamais un parent.
    pub const fn eleve(user_id: Uuid) -> Self {
        Self { prive_user_id: Some(user_id), audience: "eleve", ..Self::responsables() }
    }
}

impl PgMsgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insère un fil avec sa portée de lecture (D-014, RP1-11 ; M0a D3 et D5) :
    /// `prive_user_id` = fil privé entre ce compte et l'établissement ; `foyer_id` = fil réservé
    /// aux payeurs de ce foyer (relance de facture) ; `audience` et `vie_scolaire` : voir
    /// [`PorteeFil`].
    pub async fn insert_fil_portee(
        &self,
        fil: &MsgFil,
        portee: PorteeFil,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, fil.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg_fil (
                id, etablissement_id, eleve_id, sujet, created_by_user_id,
                created_at, last_message_at, archive, modele_id, priorite, dossier,
                prive_user_id, foyer_id, audience, vie_scolaire, foyer_lecteur_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
            "#,
        )
        .bind(fil.id)
        .bind(fil.etablissement_id)
        .bind(fil.eleve_id)
        .bind(&fil.sujet)
        .bind(fil.created_by_user_id)
        .bind(fil.created_at)
        .bind(fil.last_message_at)
        .bind(fil.archive)
        .bind(fil.modele_id)
        .bind(fil.priorite.as_str())
        .bind(fil.dossier.as_deref())
        .bind(portee.prive_user_id)
        .bind(portee.foyer_id)
        .bind(portee.audience)
        .bind(portee.vie_scolaire)
        .bind(portee.foyer_lecteur_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // M1 : nature, origine et participation du fil, dans la même transaction (R-M1, R-M14).
        // `reprise_m1_le` : un fil créé après M1 n'est jamais retraité par la reprise (483).
        sqlx::query(
            r#"
            UPDATE ent_msg_fil f SET
                nature = CASE WHEN f.audience = 'eleve' THEN 'eleve'
                              WHEN f.foyer_id IS NOT NULL OR f.vie_scolaire THEN 'systeme'
                              ELSE 'famille' END,
                origine = CASE WHEN (SELECT mb.role FROM ent_membership mb
                                      WHERE mb.user_id = f.created_by_user_id
                                        AND mb.etablissement_id = f.etablissement_id) = 'eleve' THEN 'eleve'
                               WHEN (SELECT mb.role FROM ent_membership mb
                                      WHERE mb.user_id = f.created_by_user_id
                                        AND mb.etablissement_id = f.etablissement_id) = 'parent' THEN 'famille'
                               WHEN f.foyer_id IS NOT NULL OR f.vie_scolaire THEN 'systeme'
                               ELSE 'personnel' END,
                ouvert_par_personne_id = (SELECT pe.id FROM ent_personne pe
                                           WHERE pe.etablissement_id = f.etablissement_id
                                             AND pe.user_id = f.created_by_user_id
                                             AND pe.fusionnee_dans IS NULL),
                reprise_m1_le = now()
             WHERE f.etablissement_id = $1 AND f.id = $2
            "#,
        )
        .bind(fil.etablissement_id)
        .bind(fil.id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO ent_msg_fil_eleve (etablissement_id, fil_id, eleve_id)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(fil.etablissement_id)
        .bind(fil.id)
        .bind(fil.eleve_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // Famille et élève selon la portée ; puis l'ouvreur (personnel : participant « auteur »).
        sqlx::query("SELECT ent_msg_peupler_famille($1)")
            .bind(fil.id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        sqlx::query("SELECT ent_msg_ajouter_participant($1, $2, 'auteur', 'auteur')")
            .bind(fil.id)
            .bind(fil.created_by_user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    /// Parmi `fil_ids`, ceux qu'un compte **famille** peut lire (D-014, RP1-11 ; M0a D1, D3, D5) :
    /// - jamais le fil privé d'un autre compte ;
    /// - un fil de relance (`foyer_id`) seulement pour les payeurs actifs de ce foyer, jamais
    ///   pour un élève ;
    /// - un parent : lien actif **non visé par une interdiction de communication** ; un fil de
    ///   vie scolaire exige `peut_voir_vs` et l'autorité parentale ; jamais un fil `eleve` ;
    /// - un élève : seulement les fils d'audience `eleve` ou `famille`.
    pub async fn fils_lisibles_famille(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        est_parent: bool,
        fil_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(Vec::new());
        }
        // M1 (R-M1, R-M2) : un compte famille ne lit que les fils où il participe, vus par l'unique
        // fonction `ent_msg_fil_lisible` (participation active, lien communicant relu, vie scolaire,
        // payeur, remise différée). `est_parent` n'intervient plus : le côté du participant décide.
        let _ = est_parent;
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT l.fil_id FROM ent_msg_fil_lisible($1, $3) l WHERE l.fil_id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(fil_ids)
        .bind(user_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Parmi `fil_ids`, ceux qu'un **personnel** peut lire (M0a D2, M0b D21 et D22).
    ///
    /// Règle commune ([`PREDICAT_LISIBLE_PERSONNEL`]) :
    /// - un fil de vie scolaire (carnet, alerte d'absences) reste lisible du personnel suivant
    ///   l'élève ;
    /// - un fil de relance (`foyer_id`) : `foyer_ok` (direction, secrétariat, comptabilité) ;
    /// - tout autre fil (privé d'un parent, d'un élève, du personnel aux responsables d'un foyer
    ///   ou à un élève majeur) : `reserve_ok` (droit `messagerie.fils_reserves.lire`) ;
    /// - dans tous les cas, son auteur et tout personnel qui y a écrit.
    ///
    /// Le lien d'enseignement seul (`messagerie.eleve` d'un professeur) ne suffit plus : il ouvre
    /// l'écriture d'un nouveau fil, pas la lecture de ceux des autres.
    pub async fn fils_lisibles_personnel(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        reserve_ok: bool,
        foyer_ok: bool,
        fil_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT f.id FROM ent_msg_fil f
              WHERE f.etablissement_id = $1 AND f.id = ANY($2) AND {PREDICAT_LISIBLE_PERSONNEL}"
        );
        let rows: Vec<(Uuid,)> = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(fil_ids)
            .bind(user_id)
            .bind(reserve_ok)
            .bind(foyer_ok)
            .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Une page de la boîte d'un personnel (M0b, D12) : filtrage de lisibilité **dans la
    /// requête** (une page ne se vide pas après coup), recherche sur le sujet, curseur stable
    /// `(priorité, dernier message, id)`. Retourne au plus `filtre.limite + 1` fils : le
    /// dernier, s'il est présent, signale une page suivante.
    pub async fn page_fils_personnel(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        reserve_ok: bool,
        foyer_ok: bool,
        filtre: &FiltreFils<'_>,
    ) -> Result<Vec<MsgFil>, DomainError> {
        let sql = format!(
            "SELECT f.id, f.etablissement_id, f.eleve_id, f.sujet, f.created_by_user_id,
                    f.created_at, f.last_message_at, f.archive, f.modele_id, f.priorite, f.dossier
               FROM ent_msg_fil f
              WHERE f.etablissement_id = $1 AND $2::uuid[] IS NOT NULL AND {PREDICAT_LISIBLE_PERSONNEL}
                AND ($6::uuid IS NULL OR f.eleve_id = $6)
                AND ($7::uuid[] IS NULL OR f.eleve_id = ANY($7))
                AND ($8 OR NOT f.archive)
                AND ($9::text IS NULL OR f.dossier = $9)
                AND ($10::text IS NULL OR f.sujet ILIKE '%' || $10 || '%' ESCAPE '\\')
                AND ($11::smallint IS NULL
                     OR (CASE f.priorite WHEN 'haute' THEN 0 ELSE 1 END) > $11
                     OR ((CASE f.priorite WHEN 'haute' THEN 0 ELSE 1 END) = $11
                         AND (f.last_message_at < $12
                              OR (f.last_message_at = $12 AND f.id < $13))))
              ORDER BY CASE f.priorite WHEN 'haute' THEN 0 ELSE 1 END, f.last_message_at DESC, f.id DESC
              LIMIT $14"
        );
        let dossier = filtre.dossier.map(str::trim).filter(|d| !d.is_empty());
        let recherche = filtre
            .recherche
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(echapper_like);
        let rows = sqlx::query_as::<_, FilRow>(&sql)
            .bind(etablissement_id)
            .bind(Vec::<Uuid>::new()) // $2 (réservé au prédicat commun)
            .bind(user_id)
            .bind(reserve_ok)
            .bind(foyer_ok)
            .bind(filtre.eleve_id)
            .bind(filtre.eleves)
            .bind(filtre.avec_archives || filtre.eleve_id.is_some())
            .bind(dossier)
            .bind(recherche)
            .bind(filtre.apres.map(|c| c.rang))
            .bind(filtre.apres.map(|c| c.dernier_message))
            .bind(filtre.apres.map(|c| c.fil_id))
            .bind(filtre.limite.clamp(1, 200) + 1)
            .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    /// File « messages des familles non traités » (MSG-329, M0b-1) : fils privés d'un parent,
    /// d'un élève ou du personnel à un foyer dont le **dernier message remis** est de la famille
    /// (aucune réponse du personnel depuis), les plus anciens d'abord. `eleves` borne le
    /// périmètre (`None` = tout l'établissement). Ne retourne que des fils lisibles avec
    /// `reserve_ok` : l'appelant exige `messagerie.fils_reserves.lire`.
    pub async fn fils_non_traites(
        &self,
        etablissement_id: Uuid,
        eleves: Option<&[Uuid]>,
        limite: i64,
    ) -> Result<Vec<FilNonTraite>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct Ligne {
            #[sqlx(flatten)]
            fil: FilRow,
            en_attente_depuis: DateTime<Utc>,
        }
        let rows = sqlx::query_as::<_, Ligne>(
            r#"
            SELECT f.id, f.etablissement_id, f.eleve_id, f.sujet, f.created_by_user_id,
                   f.created_at, f.last_message_at, f.archive, f.modele_id, f.priorite, f.dossier,
                   d.created_at AS en_attente_depuis
              FROM ent_msg_fil f
              JOIN LATERAL (
                    SELECT m.auteur_user_id, m.created_at
                      FROM ent_msg m
                     WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id
                       AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now())
                     ORDER BY m.created_at DESC, m.id DESC LIMIT 1) d ON true
             WHERE f.etablissement_id = $1
               AND NOT f.archive AND f.foyer_id IS NULL AND NOT f.vie_scolaire
               AND f.nature <> 'boite' AND f.sensibilite <> 'C3'
               AND (f.prive_user_id IS NOT NULL OR f.foyer_lecteur_id IS NOT NULL
                    OR f.audience = 'eleve')
               AND EXISTS (SELECT 1 FROM ent_membership mb
                            WHERE mb.etablissement_id = f.etablissement_id
                              AND mb.user_id = d.auteur_user_id
                              AND mb.role IN ('parent', 'eleve'))
               AND ($2::uuid[] IS NULL OR f.eleve_id = ANY($2))
             ORDER BY d.created_at ASC, f.id ASC
             LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleves)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(|l| {
                Some(FilNonTraite { fil: l.fil.into_fil()?, en_attente_depuis: l.en_attente_depuis })
            })
            .collect())
    }

    /// Rôle de base de chaque compte (membres de l'établissement).
    pub async fn roles_de(
        &self,
        etablissement_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if user_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT user_id, role FROM ent_membership
              WHERE etablissement_id = $1 AND user_id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(user_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    /// Personnels à prévenir quand une famille écrit (M0b, D13) : ceux qui ont déjà écrit dans le
    /// fil (hors `sauf`) ; à défaut, les titulaires de `messagerie.fils_reserves.lire` à
    /// l'échelle de l'établissement (rôle de base `bases` ou rôle paramétré actif).
    pub async fn personnels_a_prevenir(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        sauf: Uuid,
        bases: &[&str],
    ) -> Result<Vec<Uuid>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        // M1 : les personnels participants du fil (ils ont ouvert le fil ou y ont écrit), hors `sauf`.
        let ont_ecrit: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT pe.user_id
               FROM ent_msg_participant p
               JOIN ent_personne pe ON pe.etablissement_id = p.etablissement_id AND pe.id = p.personne_id
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.suspendu_le IS NULL
                AND p.cote = 'etablissement' AND pe.user_id IS NOT NULL AND pe.user_id <> $3",
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(sauf)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if !ont_ecrit.is_empty() {
            return Ok(ont_ecrit.into_iter().map(|(u,)| u).collect());
        }
        let bases: Vec<String> = bases.iter().map(|b| (*b).to_string()).collect();
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT m.user_id FROM ent_membership m
              WHERE m.etablissement_id = $1 AND m.role = ANY($2) AND m.user_id <> $3
             UNION
             SELECT a.user_id
               FROM ent_role_attribution a
               JOIN ent_role_droit d ON d.role_id = a.role_id
                                    AND d.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.clos_le IS NULL AND a.user_id <> $3
                AND a.debut <= ent_aujourdhui_etab($1) AND (a.fin IS NULL OR a.fin >= ent_aujourdhui_etab($1))
                AND d.action = 'messagerie.fils_reserves.lire' AND d.portee = 'etablissement'",
        )
        .bind(etablissement_id)
        .bind(&bases)
        .bind(sauf)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(u,)| u).collect())
    }

    /// Comptes responsables à notifier pour ce fil : `None` = tous les liens notifiables de
    /// l'élève ; `Some(ids)` = seulement ceux-là (fil privé, de foyer, d'élève ou de vie
    /// scolaire). Le filtre des liens interdits ou « ne pas solliciter » est appliqué ensuite.
    pub async fn destinataires_famille_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Option<Vec<Uuid>>, DomainError> {
        // M1 (R-M1) : les destinataires d'un avis sont les participants **famille** actifs du fil,
        // jamais « tous les liens de l'élève » ; un fil d'élève (côté `eleve`) n'est jamais annoncé
        // à un parent. L'interdiction, « ne pas solliciter » et la suspension sont relues ensuite
        // par l'appelant (`list_liens_eleve_actifs`).
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT pe.user_id
               FROM ent_msg_participant p
               JOIN ent_personne pe ON pe.etablissement_id = p.etablissement_id AND pe.id = p.personne_id
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.suspendu_le IS NULL
                AND p.cote = 'famille' AND pe.user_id IS NOT NULL",
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(Some(ids.into_iter().map(|(u,)| u).collect()))
    }

    /// Peut-on écrire dans ce fil ? (R-M2 : `peut_ecrire_fil`, volet participation.) Faux si le fil
    /// est clos ou gelé, ou si la participation active du compte lui retire l'écriture. Un compte
    /// sans participation (personnel venu par le pont des rôles réservés) peut écrire : il devient
    /// participant à son premier message. La lecture reste jugée par `ent_msg_fil_lisible`.
    pub async fn peut_ecrire_fil(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        fil_id: Uuid,
    ) -> Result<bool, DomainError> {
        let ok: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT f.etat = 'ouvert'
                   AND COALESCE((
                        SELECT bool_and(p.peut_ecrire)
                          FROM ent_msg_participant p
                         WHERE p.etablissement_id = f.etablissement_id AND p.fil_id = f.id
                           AND p.au IS NULL AND p.suspendu_le IS NULL
                           AND (p.personne_id IN (SELECT pe.id FROM ent_personne pe
                                                   WHERE pe.etablissement_id = f.etablissement_id
                                                     AND pe.user_id = $3)
                                OR p.eleve_id IN (SELECT c.eleve_id FROM ent_eleve_compte c
                                                   WHERE c.etablissement_id = f.etablissement_id
                                                     AND c.user_id = $3))), true)
              FROM ent_msg_fil f
             WHERE f.etablissement_id = $1 AND f.id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(user_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(ok.is_some_and(|(b,)| b))
    }

    /// Nombre de participants actifs d'un fil par côté `(famille, établissement, élève)` : charge
    /// des événements `messagerie.*` (R-M14 : des comptes, jamais de contenu ni de nom).
    pub async fn compter_participants(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<(i64, i64, i64), DomainError> {
        let (f, e, l): (i64, i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE cote = 'famille'),
                    count(*) FILTER (WHERE cote = 'etablissement'),
                    count(*) FILTER (WHERE cote = 'eleve')
               FROM ent_msg_participant
              WHERE etablissement_id = $1 AND fil_id = $2 AND au IS NULL",
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok((f, e, l))
    }

    /// Marque `fil_id` comme jumeau de `premier` (R-M3 : un fil par foyer pour un même envoi).
    pub async fn marquer_jumeau(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        premier: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            "UPDATE ent_msg_fil SET jumeau_de = $3
              WHERE etablissement_id = $1 AND id = $2 AND $2 <> $3",
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(premier)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_fil_archive(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        archive: bool,
        archived_at: Option<DateTime<Utc>>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            UPDATE ent_msg_fil
            SET archive = $3, archived_at = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(archive)
        .bind(archived_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_fil_boite(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        priorite: MsgPriorite,
        dossier: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            UPDATE ent_msg_fil
            SET priorite = $3, dossier = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(priorite.as_str())
        .bind(dossier)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn insert_modele(&self, m: &MsgModele) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, m.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg_modele (
                id, etablissement_id, titre, sujet, corps, actif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(&m.titre)
        .bind(&m.sujet)
        .bind(&m.corps)
        .bind(m.actif)
        .bind(m.created_by_user_id)
        .bind(m.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_modele(
        &self,
        etablissement_id: Uuid,
        modele_id: Uuid,
    ) -> Result<Option<MsgModele>, DomainError> {
        let row = sqlx::query_as::<_, ModeleRow>(
            r#"
            SELECT id, etablissement_id, titre, sujet, corps, actif,
                   created_by_user_id, created_at
            FROM ent_msg_modele
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(modele_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(ModeleRow::into_modele))
    }

    pub async fn list_modeles(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<MsgModele>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = if actifs_seulement {
            sqlx::query_as::<_, ModeleRow>(
                r#"
                SELECT id, etablissement_id, titre, sujet, corps, actif,
                       created_by_user_id, created_at
                FROM ent_msg_modele
                WHERE etablissement_id = $1 AND actif = true AND portee = 'etablissement'
                ORDER BY titre ASC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, ModeleRow>(
                r#"
                SELECT id, etablissement_id, titre, sujet, corps, actif,
                       created_by_user_id, created_at
                FROM ent_msg_modele
                WHERE etablissement_id = $1 AND portee = 'etablissement'
                ORDER BY titre ASC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(ModeleRow::into_modele).collect())
    }

    pub async fn insert_message(&self, msg: &Message) -> Result<(), DomainError> {
        self.insert_message_remise(msg, None, None).await.map(|_| ())
    }

    /// Insère un message ; `remise` = instant de remise différée par la plage calme (M0b, D24),
    /// `derogation` = (motif, auteur) d'une remise immédiate demandée par la direction.
    pub async fn insert_message_remise(
        &self,
        msg: &Message,
        remise: Option<DateTime<Utc>>,
        derogation: Option<(&str, Uuid)>,
    ) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, msg.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_msg (
                id, etablissement_id, fil_id, auteur_user_id, corps, created_at,
                remise_prevue_le, derogation_motif, derogation_par
            ) VALUES ($1,$2,$3,$4,$5,clock_timestamp(),$6,$7,$8)
            "#,
        )
        .bind(msg.id)
        .bind(msg.etablissement_id)
        .bind(msg.fil_id)
        .bind(msg.auteur_user_id)
        .bind(&msg.corps)
        .bind(remise)
        .bind(derogation.map(|d| d.0))
        .bind(derogation.map(|d| d.1))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // M1 : qui écrit dans un fil y participe (un personnel venu par le pont des rôles réservés
        // devient participant) ; la fiche personne de l'auteur est posée sur le message.
        let (nouveau,): (bool,) =
            sqlx::query_as("SELECT ent_msg_ajouter_participant($1, $2, 'ajoute', 'a_ecrit')")
                .bind(msg.fil_id)
                .bind(msg.auteur_user_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx)?;
        sqlx::query(
            "UPDATE ent_msg m SET auteur_personne_id = pe.id
               FROM ent_personne pe
              WHERE m.etablissement_id = $1 AND m.id = $2
                AND pe.etablissement_id = m.etablissement_id AND pe.user_id = m.auteur_user_id
                AND pe.fusionnee_dans IS NULL",
        )
        .bind(msg.etablissement_id)
        .bind(msg.id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(nouveau)
    }

    pub async fn touch_fil_last(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            UPDATE ent_msg_fil SET last_message_at = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Option<MsgFil>, DomainError> {
        let row = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(FilRow::into_fil))
    }

    /// Fils par identifiants, en **une** requête (les inconnus sont absents). Lecture par identifiant : l'appelant
    /// applique ensuite le jugement de lecture groupé (`actor_can_access_fils_lot`).
    pub async fn get_fils_lot(
        &self,
        etablissement_id: Uuid,
        fil_ids: &[Uuid],
    ) -> Result<Vec<MsgFil>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    pub async fn list_fils_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<MsgFil>, DomainError> {
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    pub async fn list_fils_eleves(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
    ) -> Result<Vec<MsgFil>, DomainError> {
        if eleve_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND eleve_id = ANY($2)
            ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    /// Messages remis du fil, plus ceux que `lecteur` a lui-même écrits et dont la remise est
    /// différée par la plage calme (M0b, D24 : invisibles de tout autre jusqu'à la remise).
    pub async fn list_messages(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        lecteur: Uuid,
    ) -> Result<Vec<Message>, DomainError> {
        let rows = sqlx::query_as::<_, MsgRow>(
            r#"
            SELECT id, etablissement_id, fil_id, auteur_user_id,
                   CASE WHEN retire_le IS NOT NULL THEN 'Message retiré par son auteur.' ELSE corps END AS corps,
                   created_at
            FROM ent_msg
            WHERE etablissement_id = $1 AND fil_id = $2
              AND (remise_prevue_le IS NULL OR remise_prevue_le <= now() OR auteur_user_id = $3)
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(lecteur)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(MsgRow::into_msg).collect())
    }

    /// Insert accusés ; ignore les doublons (message_id, lecteur_user_id).
    pub async fn upsert_accuses(&self, accuses: &[MsgAccuse]) -> Result<usize, DomainError> {
        let mut inserted = 0usize;
        for a in accuses {
            let mut tx = TenantTx::begin(&self.pool, a.etablissement_id).await.map_err(map_sqlx)?;
            let res = sqlx::query(
                r#"
                INSERT INTO ent_msg_accuse (
                    id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at
                ) VALUES ($1,$2,$3,$4,$5,$6)
                ON CONFLICT (message_id, lecteur_user_id) DO NOTHING
                "#,
            )
            .bind(a.id)
            .bind(a.etablissement_id)
            .bind(a.fil_id)
            .bind(a.message_id)
            .bind(a.lecteur_user_id)
            .bind(a.lu_at)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            tx.commit().await.map_err(map_sqlx)?;
            inserted += res.rows_affected() as usize;
        }
        Ok(inserted)
    }

    pub async fn list_accuses_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<MsgAccuse>, DomainError> {
        let rows = sqlx::query_as::<_, AccuseRow>(
            r#"
            SELECT id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at
            FROM ent_msg_accuse
            WHERE etablissement_id = $1 AND fil_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AccuseRow::into_accuse).collect())
    }

    /// Position de départ du flux : borne `xid` courante et heure de la base.
    pub async fn position_depart_flux(&self, etablissement_id: Uuid) -> Result<PositionFlux, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let (maintenant, xmin, cluster) = lire_borne(&mut tx).await?;
        Ok(PositionFlux::depart(xmin, maintenant, cluster))
    }

    /// M12 (flux temps réel) : candidats du flux. Seuls les fils où le compte **participe** (vue
    /// `ent_msg_fil_lisible`, relue à chaque appel) entrent ici ; l'appelant applique ensuite le **jugement de
    /// lecture** complet (`actor_can_access_fil`, avec le rôle de la session) : le flux ne révèle jamais plus que la
    /// lecture (règle 1). Aucun contenu : identifiant du fil, instant, genre (`nouveau` ou `retire`).
    ///
    /// Les messages **du compte lui-même** n'apparaissent pas (il les connaît) ; un message **retiré** est signalé
    /// (`retire`, sans contenu) sauf s'il est retiré par ce compte. Un message différé reste muet avant sa remise.
    ///
    /// Positions (M1) : voir [`PositionFlux`]. Les événements immédiats sont servis **sous la borne `xmin`** et
    /// dans l'ordre `(xid, fil, genre)` : un message validé tard (transaction longue) porte un `xid` inférieur ou égal
    /// à celui de ses voisins, donc n'est jamais dépassé par le curseur. Les événements différés sont servis par
    /// échéance `(remise, fil)`. Chaque flux est limité à `limite` événements ; `tronque` si l'un des deux l'est, et
    /// la position de ce flux est alors celle du dernier événement rendu.
    pub async fn evenements_depuis(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        apres: &PositionFlux,
        limite: i64,
    ) -> Result<EvenementsFlux, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let (maintenant, xmin, cluster) = lire_borne(&mut tx).await?;
        if !curseur_borne(apres, maintenant, xmin, cluster) {
            return Ok(EvenementsFlux {
                position: PositionFlux::depart(xmin, maintenant, cluster),
                tronque: false,
                resynchro: true,
                evenements: Vec::new(),
            });
        }
        let limite = limite.max(0);
        let mut immediats = self
            .evenements_immediats(&mut tx, etablissement_id, user_id, apres, xmin, limite + 1)
            .await?;
        let tronque_i = immediats.len() as i64 > limite;
        immediats.truncate(limite as usize);
        let mut differes = self
            .evenements_differes(&mut tx, etablissement_id, user_id, apres, limite + 1)
            .await?;
        let tronque_d = differes.len() as i64 > limite;
        differes.truncate(limite as usize);

        let mut position = PositionFlux { emis: maintenant, ..apres.clone() };
        if tronque_i {
            if let Some((fil, _, genre, xid)) = immediats.last() {
                (position.xid, position.fil, position.genre) = (*xid, Some(*fil), Some(genre.clone()));
            }
        } else {
            // Rattrapé : tout ce qui est sous la borne est servi ; la suite est strictement au-delà.
            (position.xid, position.fil, position.genre) = (xmin.max(apres.xid), None, None);
        }
        if tronque_d {
            if let Some((fil, instant, _)) = differes.last() {
                (position.differe, position.differe_fil) = (*instant, Some(*fil));
            }
        } else {
            // Rattrapé : tout ce qui est échu (`<= maintenant`) vient d'être servi. Le curseur avance à `maintenant`,
            // sans quoi, par temps calme, il resterait à son point de départ et sortirait de la borne de M-2.
            (position.differe, position.differe_fil) = (maintenant.max(apres.differe), None);
        }
        let mut evenements: Vec<(Uuid, DateTime<Utc>, String)> =
            immediats.into_iter().map(|(f, i, g, _)| (f, i, g)).collect();
        evenements.extend(differes);
        Ok(EvenementsFlux { position, tronque: tronque_i || tronque_d, resynchro: false, evenements })
    }

    /// Événements **immédiats** strictement après `(xid, fil, genre)` et sous `xmin` : ordre `(xid, fil, genre)`.
    async fn evenements_immediats(
        &self,
        tx: &mut TenantTx<'_>,
        etablissement_id: Uuid,
        user_id: Uuid,
        apres: &PositionFlux,
        xmin: u64,
        limite: i64,
    ) -> Result<Vec<(Uuid, DateTime<Utc>, String, u64)>, DomainError> {
        let rows: Vec<(Uuid, DateTime<Utc>, String, String)> = sqlx::query_as(
            r#"
            WITH ev AS (
                SELECT m.fil_id, m.created_at AS instant, m.msg_xid AS xid, 'nouveau' AS genre
                  FROM ent_msg_fil_lisible($1, $2) l
                  JOIN ent_msg m ON m.etablissement_id = $1 AND m.fil_id = l.fil_id
                 WHERE m.auteur_user_id IS DISTINCT FROM $2
                   AND m.retire_le IS NULL
                   AND m.remise_prevue_le IS NULL
                   AND m.msg_xid >= $3::text::xid8 AND m.msg_xid < $7::text::xid8
                UNION ALL
                SELECT m.fil_id, m.retire_le AS instant, coalesce(m.retire_xid, m.msg_xid) AS xid, 'retire' AS genre
                  FROM ent_msg_fil_lisible($1, $2) l
                  JOIN ent_msg m ON m.etablissement_id = $1 AND m.fil_id = l.fil_id
                 WHERE m.retire_le IS NOT NULL
                   AND m.retire_par IS DISTINCT FROM $2
                   AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now())
                   AND coalesce(m.retire_xid, m.msg_xid) >= $3::text::xid8
                   AND coalesce(m.retire_xid, m.msg_xid) < $7::text::xid8
            ), agg AS (
                SELECT fil_id, genre, max(xid) AS xid, max(instant) AS instant FROM ev GROUP BY fil_id, genre
            )
            SELECT fil_id, instant, genre, xid::text FROM agg
             WHERE (xid, fil_id, genre) > ($3::text::xid8, $4, $6)
             ORDER BY xid ASC, fil_id ASC, genre ASC
             LIMIT $5
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(apres.xid.to_string())
        .bind(apres.fil.unwrap_or_else(Uuid::nil))
        .bind(limite)
        .bind(apres.genre.as_deref().unwrap_or(""))
        .bind(xmin.to_string())
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter()
            .map(|(f, i, g, x)| x.parse::<u64>().map(|x| (f, i, g, x)).map_err(|_| DomainError::ReferenceInconnue))
            .collect()
    }

    /// Événements **différés** : messages dont l'échéance est passée, strictement après `(instant, fil)`.
    async fn evenements_differes(
        &self,
        tx: &mut TenantTx<'_>,
        etablissement_id: Uuid,
        user_id: Uuid,
        apres: &PositionFlux,
        limite: i64,
    ) -> Result<Vec<(Uuid, DateTime<Utc>, String)>, DomainError> {
        sqlx::query_as(
            r#"
            SELECT m.fil_id, max(m.remise_prevue_le) AS instant, 'nouveau' AS genre
              FROM ent_msg_fil_lisible($1, $2) l
              JOIN ent_msg m ON m.etablissement_id = $1 AND m.fil_id = l.fil_id
             WHERE m.auteur_user_id IS DISTINCT FROM $2
               AND m.retire_le IS NULL
               AND m.remise_prevue_le IS NOT NULL
               AND m.remise_prevue_le >= $3 AND m.remise_prevue_le <= now()
             GROUP BY m.fil_id
            HAVING (max(m.remise_prevue_le), m.fil_id) > ($3, $4)
             ORDER BY max(m.remise_prevue_le) ASC, m.fil_id ASC
             LIMIT $5
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(apres.differe)
        .bind(apres.differe_fil.unwrap_or_else(Uuid::nil))
        .bind(limite)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx)
    }

    /// Messages non accusés par le lecteur (hors ses propres messages), par fil.
    pub async fn count_non_lus_par_fils(
        &self,
        etablissement_id: Uuid,
        lecteur_user_id: Uuid,
        fil_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, i64>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT m.fil_id, COUNT(*)::bigint
            FROM ent_msg m
            WHERE m.etablissement_id = $1
              AND m.fil_id = ANY($2)
              AND m.auteur_user_id <> $3
              AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now())
              AND NOT EXISTS (
                SELECT 1 FROM ent_msg_accuse a
                WHERE a.message_id = m.id AND a.lecteur_user_id = $3
              )
            GROUP BY m.fil_id
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_ids)
        .bind(lecteur_user_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    /// Insère une pièce dont le fichier est **déjà** dans le magasin d'objets (clé + empreinte).
    /// Plus aucune écriture de BYTEA : `contenu` reste vide (PRA_STOCKAGE_OBJETS §5, étape 1).
    /// Insère une pièce **sous verrou** du message (F1) : le message est verrouillé (`FOR UPDATE`), puis la limite de
    /// `max` pièces et l'état du message (ni retiré, ni différé d'un autre auteur) sont contrôlés dans la même transaction que
    /// l'insertion : deux dépôts simultanés ne dépassent pas la limite.
    pub async fn insert_piece(&self, piece: &MsgPiece, max: i64) -> Result<InsertionPiece, DomainError> {
        let (Some(cle), Some(sha)) = (piece.storage_key.as_deref(), piece.sha256.as_deref()) else {
            return Err(DomainError::MsgPieceInvalide);
        };
        let mut tx = TenantTx::begin(&self.pool, piece.etablissement_id)
            .await
            .map_err(map_sqlx)?;
        let etat: Option<(bool, bool, Option<Uuid>)> = sqlx::query_as(
            "SELECT retire_le IS NOT NULL, coalesce(remise_prevue_le > now(), false), auteur_user_id
               FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(piece.etablissement_id)
        .bind(piece.message_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let Some((retire, differe, auteur)) = etat else {
            return Err(DomainError::NotFound);
        };
        if retire {
            return Ok(InsertionPiece::MessageRetire);
        }
        // Un message différé n'est invisible de tout autre : son **auteur** peut y joindre une pièce avant la remise
        // (elle suit le message : quarantaine, antivirus, puis remise) ; personne d'autre.
        if differe && auteur != Some(piece.created_by_user_id) {
            return Ok(InsertionPiece::MessageDiffere);
        }
        let (nb,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1 AND message_id = $2",
        )
        .bind(piece.etablissement_id)
        .bind(piece.message_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if nb >= max {
            return Ok(InsertionPiece::Plein);
        }
        sqlx::query(
            r#"
            INSERT INTO ent_msg_piece (
                id, etablissement_id, fil_id, message_id, nom, mime_type,
                taille_octets, contenu, storage_key, sha256, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,'\x'::bytea,$8,$9,$10,$11)
            "#,
        )
        .bind(piece.id)
        .bind(piece.etablissement_id)
        .bind(piece.fil_id)
        .bind(piece.message_id)
        .bind(&piece.nom)
        .bind(&piece.mime_type)
        .bind(piece.taille_octets)
        .bind(cle)
        .bind(sha)
        .bind(piece.created_by_user_id)
        .bind(piece.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(InsertionPiece::Inseree)
    }

    pub async fn list_pieces_meta_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<MsgPieceMeta>, DomainError> {
        let rows = sqlx::query_as::<_, PieceMetaRow>(
            r#"
            SELECT id, message_id, nom, mime_type, taille_octets,
                   created_by_user_id, created_at, etat
            FROM ent_msg_piece
            WHERE etablissement_id = $1 AND fil_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(PieceMetaRow::into_meta).collect())
    }

    /// Liens communicants de l'élève groupés par foyer (`None` = lien sans foyer) : un fil du
    /// personnel est créé par groupe (M0b, D22). Ordre stable (foyers par identifiant).
    ///
    /// M2 (R-M7) : avec `sollicitables_seulement`, les responsables « à ne pas solliciter » sont
    /// exclus (aucun fil ouvert par l'établissement vers eux) ; sans, tous les liens communicants
    /// (lecture et réponse dans les fils qu'ils ont eux-mêmes ouverts).
    pub async fn groupes_communicants(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        sollicitables_seulement: bool,
    ) -> Result<Vec<(Option<Uuid>, Vec<Uuid>)>, DomainError> {
        let vue = if sollicitables_seulement {
            "ent_eleve_lien_sollicitable"
        } else {
            "ent_eleve_lien_communicant"
        };
        let rows: Vec<(Option<Uuid>, Uuid)> = sqlx::query_as(&format!(
            "SELECT DISTINCT foyer_id, user_id FROM {vue}
              WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id IS NOT NULL
              ORDER BY foyer_id NULLS LAST, user_id"
        ))
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        let mut groupes: Vec<(Option<Uuid>, Vec<Uuid>)> = Vec::new();
        for (foyer, user) in rows {
            match groupes.last_mut() {
                Some((f, users)) if *f == foyer => users.push(user),
                _ => groupes.push((foyer, vec![user])),
            }
        }
        Ok(groupes)
    }

    /// Supprime un message dont la remise est encore à venir (R-M48), s'il est de `auteur` : **totalement ou pas du
    /// tout** (E1). Voir [`AnnulationDiffere`] ; les clés d'objets des pièces supprimées (original et fichier servi)
    /// sont rendues : l'appelant détruit les objets **après** le commit.
    ///
    /// Tout se passe dans **une** transaction. Les cartes « document », le journal d'accès aux pièces et les versions
    /// d'un message corrigé sont en ajout seul : la suppression n'est permise que par l'exception bornée du
    /// déclencheur (jeton de transaction `ent_msg_annulation_garde` posé pour ce message, jamais remis, migrations 490 à 492). Un message
    /// jamais remis n'a pas de lecteur : ses cartes, accès, versions et lectures disparaissent avec lui. Si la base
    /// refuse une suppression (23503 clé étrangère, 23514 ou 42501 garde), la transaction est défaite en entier.
    pub async fn annuler_message_differe(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        auteur: Uuid,
    ) -> Result<AnnulationDiffere, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let ok: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_msg
              WHERE etablissement_id = $1 AND id = $2 AND auteur_user_id = $3
                AND remise_prevue_le > now() FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .bind(auteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if ok.is_none() {
            return Ok(AnnulationDiffere::DejaRemis);
        }
        let refus_base = |e: &sqlx::Error| {
            matches!(
                e.as_database_error().and_then(|d| d.code()).as_deref(),
                Some("23503" | "23514" | "42501")
            )
        };
        // M-3 : jeton lié à CETTE transaction (migration 492), plus un réglage libre que n'importe quelle session pose.
        match sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2, $3)")
            .bind(etablissement_id)
            .bind(message_id)
            .bind(auteur)
            .execute(&mut *tx)
            .await
        {
            Ok(_) => {}
            Err(e) if refus_base(&e) => return Ok(AnnulationDiffere::Refusee),
            Err(e) => return Err(map_sqlx(e)),
        }
        for requete in [
            "DELETE FROM ent_msg_carte_document WHERE etablissement_id = $1 AND message_id = $2",
            "DELETE FROM ent_msg_piece_acces WHERE etablissement_id = $1
               AND piece_id IN (SELECT id FROM ent_msg_piece WHERE etablissement_id = $1 AND message_id = $2)",
            "DELETE FROM ent_msg_lecture WHERE etablissement_id = $1 AND lu_jusqua_message_id = $2",
        ] {
            match sqlx::query(requete).bind(etablissement_id).bind(message_id).execute(&mut *tx).await {
                Ok(_) => {}
                Err(e) if refus_base(&e) => return Ok(AnnulationDiffere::Refusee),
                Err(e) => return Err(map_sqlx(e)),
            }
        }
        let cles: Vec<(Option<String>, Option<String>)> = match sqlx::query_as(
            "DELETE FROM ent_msg_piece WHERE etablissement_id = $1 AND message_id = $2
             RETURNING storage_key, cle_objet_servi",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .fetch_all(&mut *tx)
        .await
        {
            Ok(c) => c,
            Err(e) if refus_base(&e) => return Ok(AnnulationDiffere::Refusee),
            Err(e) => return Err(map_sqlx(e)),
        };
        // M-1 : l'avis planifié de ce message (`en_attente`) et celui que le distributeur a déjà pris (`envoi`) sont
        // annulés **dans cette transaction**, avant la suppression du message : tout ou rien. Un échec ou une panne
        // plus loin défait aussi l'annulation de l'avis ; la ligne d'avis étant verrouillée par cette mise à jour, le
        // distributeur (`FOR UPDATE SKIP LOCKED`) ne la prend pas avant le commit.
        if let Err(e) = sqlx::query(
            "UPDATE ent_notification_envoi SET statut = 'annule', motif = 'message_annule', traite_le = now()
              WHERE etablissement_id = $1 AND source_type = 'systeme' AND source_id = $2
                AND statut IN ('en_attente', 'envoi')",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .execute(&mut *tx)
        .await
        {
            return if refus_base(&e) { Ok(AnnulationDiffere::Refusee) } else { Err(map_sqlx(e)) };
        }
        // Versions d'un message corrigé avant sa remise, puis accusés : dans la même transaction, avant le message.
        for requete in [
            "DELETE FROM ent_msg_version WHERE etablissement_id = $1 AND message_id = $2",
            "DELETE FROM ent_msg_accuse WHERE etablissement_id = $1 AND message_id = $2",
            "DELETE FROM ent_msg WHERE etablissement_id = $1 AND id = $2",
        ] {
            match sqlx::query(requete).bind(etablissement_id).bind(message_id).execute(&mut *tx).await {
                Ok(_) => {}
                Err(e) if refus_base(&e) => return Ok(AnnulationDiffere::Refusee),
                Err(e) => return Err(map_sqlx(e)),
            }
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(AnnulationDiffere::Annule(cles.into_iter().flat_map(|(a, b)| [a, b]).flatten().collect()))
    }

    pub async fn count_pieces_message(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
    ) -> Result<i64, DomainError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1 AND message_id = $2",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    /// Le message est-il remis, ou écrit par `lecteur` lui-même (M0b, D24) ?
    pub async fn message_remis_pour(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        lecteur: Uuid,
    ) -> Result<bool, DomainError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT coalesce((SELECT remise_prevue_le IS NULL OR remise_prevue_le <= now()
                                     OR auteur_user_id = $3
                                FROM ent_msg WHERE etablissement_id = $1 AND id = $2), false)",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .bind(lecteur)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(ok)
    }

    /// F6 : observation de la borne `xmin` **hors du flux** (alerte de gel, `msg_xmin_gele`). Le flux n'observe la borne
    /// que quand un client l'interroge : sans client connecté, un gel passerait inaperçu. Le distributeur d'avis l'appelle
    /// à chaque passage ; l'échec de la lecture est journalisé (`warn!`) et n'interrompt rien. Aucune donnée n'est lue
    /// (la borne est un compteur du cluster).
    pub(crate) async fn observer_borne_hors_flux(&self, etablissement_id: Uuid) {
        match TenantTx::begin(&self.pool, etablissement_id).await {
            Ok(mut tx) => {
                if let Err(e) = lire_borne(&mut tx).await {
                    // Passe 5, faible 4 : une base indisponible ne journalise pas toutes les 5 s (une ligne par minute).
                    if crate::journal_limite::autoriser_global("xmin_hors_flux_lecture") {
                        tracing::warn!(error = %e, "observation hors flux de la borne xmin : lecture impossible");
                    }
                }
            }
            Err(e) => {
                if crate::journal_limite::autoriser_global("xmin_hors_flux_transaction") {
                    tracing::warn!(error = %e, "observation hors flux de la borne xmin : transaction impossible");
                }
            }
        }
    }

    /// Le message existe-t-il encore ? Lu `FOR KEY SHARE` : une annulation en cours (`FOR UPDATE` du message) est
    /// attendue, puis la ligne a disparu. Prédicat d'existence pour le distributeur d'avis (M-1) : aucun contenu.
    ///
    /// `attente` borne l'attente de ce verrou (`lock_timeout`, côté base) : au-delà, erreur (55P03) et l'appelant reporte.
    pub(crate) async fn message_existe_sous_verrou(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        attente: std::time::Duration,
    ) -> Result<bool, DomainError> {
        self.message_existe_sous_verrou_brut(etablissement_id, message_id, attente)
            .await
            .map_err(map_sqlx)
    }

    /// Même contrôle, erreur **brute** : le code SQLSTATE reste lisible (`Database::code`) alors que le dépôt le réduit
    /// à `Infrastructure`. Passe 5, faible 2 : les tests exigent 42501 (rôle sans droit) ou 55P03 (verrou tenu) ; la
    /// réponse HTTP n'en est pas changée (l'API n'appelle que la version de domaine ci-dessus).
    pub(crate) async fn message_existe_sous_verrou_brut(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        attente: std::time::Duration,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await?;
        sqlx::query("SELECT set_config('lock_timeout', $1, true)")
            .bind(format!("{}ms", attente.as_millis().max(1)))
            .execute(&mut *tx)
            .await?;
        let r: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR KEY SHARE")
                .bind(etablissement_id)
                .bind(message_id)
                .fetch_optional(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(r.is_some())
    }

    /// Vrai si ce message est **différé et pas encore remis** et que `acteur` n'en est pas l'auteur : pour tout autre
    /// que l'auteur il n'existe pas encore (l'appelant répond 404, comme pour un identifiant inconnu).
    pub async fn message_differe_d_autrui(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
        acteur: Uuid,
    ) -> Result<bool, DomainError> {
        let r: Option<(bool,)> = sqlx::query_as(
            "SELECT coalesce(remise_prevue_le > now(), false) AND auteur_user_id IS DISTINCT FROM $3
               FROM ent_msg WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(message_id)
        .bind(acteur)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(r.is_some_and(|(b,)| b))
    }

    pub async fn get_message(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Message>, DomainError> {
        let row = sqlx::query_as::<_, MsgRow>(
            r#"
            SELECT id, etablissement_id, fil_id, auteur_user_id,
                   CASE WHEN retire_le IS NOT NULL THEN 'Message retiré par son auteur.' ELSE corps END AS corps,
                   created_at
            FROM ent_msg
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(message_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(MsgRow::into_msg))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FilRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    sujet: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    last_message_at: DateTime<Utc>,
    archive: bool,
    modele_id: Option<Uuid>,
    priorite: String,
    dossier: Option<String>,
}

impl FilRow {
    fn into_fil(self) -> Option<MsgFil> {
        let priorite = MsgPriorite::parse(&self.priorite)?;
        Some(MsgFil {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            sujet: self.sujet,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            last_message_at: self.last_message_at,
            archive: self.archive,
            modele_id: self.modele_id,
            priorite,
            dossier: self.dossier,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ModeleRow {
    id: Uuid,
    etablissement_id: Uuid,
    titre: String,
    sujet: String,
    corps: String,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ModeleRow {
    fn into_modele(self) -> MsgModele {
        MsgModele {
            id: self.id,
            etablissement_id: self.etablissement_id,
            titre: self.titre,
            sujet: self.sujet,
            corps: self.corps,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MsgRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    auteur_user_id: Uuid,
    corps: String,
    created_at: DateTime<Utc>,
}

impl MsgRow {
    fn into_msg(self) -> Message {
        Message {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            auteur_user_id: self.auteur_user_id,
            corps: self.corps,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AccuseRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    lecteur_user_id: Uuid,
    lu_at: DateTime<Utc>,
}

impl AccuseRow {
    fn into_accuse(self) -> MsgAccuse {
        MsgAccuse {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            message_id: self.message_id,
            lecteur_user_id: self.lecteur_user_id,
            lu_at: self.lu_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PieceMetaRow {
    id: Uuid,
    message_id: Uuid,
    nom: String,
    mime_type: String,
    taille_octets: i32,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    etat: String,
}

impl PieceMetaRow {
    fn into_meta(self) -> MsgPieceMeta {
        MsgPieceMeta {
            id: self.id,
            message_id: self.message_id,
            nom: self.nom,
            mime_type: self.mime_type,
            taille_octets: self.taille_octets,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            etat: self.etat,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("msg_repo", err)
}
