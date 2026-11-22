//! Socle 41, lot I4 — **vie quotidienne** : liaison codée jour / nuit, événements de nuit, études et activités du soir,
//! préoccupation (§11, INT-180 à INT-194, INT-247 ; R-IT7, R-IT8, R-IT9, R-IT29).
//!
//! Règles générales de protection appliquées ici (détail : `docs/socles/41_INTERNAT_I4_LIVRAISON.md`) :
//! * **aucune donnée de santé dans le 41** (R-IT6) : tout texte est contrôlé ; un texte qui évoque la santé est refusé avec
//!   l'invite d'utiliser le mouvement du socle 34 ; un « malaise » n'a aucun texte libre et émet un signal codé vers le 34 ;
//! * **refus par défaut** (R-IT8) : la liaison et les événements ne sont lus que par l'équipe d'internat, le CPE et la
//!   direction (droits `internat.liaison.lire`, portée vérifiée au serveur, R-IT30) ; la vie scolaire de jour ne voit que
//!   les faits « pour le jour », jamais leur texte ; ni professeur, ni infirmerie, ni famille, ni élève ;
//! * **aucune visibilité famille sans publication** (R-IT9) : ce lot ne donne AUCUNE lecture de la liaison aux familles
//!   (la publication dans le bilan de la semaine relève du lot I7) ;
//! * **R-IT3** rien ne se supprime : masquage tracé, complément ajouté, annulation datée, retrait daté ;
//! * **R-IT29** la préoccupation n'enregistre aucun contenu : un adulte, un élève, un instant ;
//! * **aucun nom, aucun texte libre dans l'outbox** : identifiants, dates et codes ;
//! * **R-IT38** chaque lecture de liaison ou d'événement est journalisée.
//!
//! Chaque fonction `*_a` reçoit l'instant (`maintenant`) : aucun test ne dépend de l'heure réelle.

use super::groupes::{
    est_interne_tx, est_personnel_tx, famille_dto_tx, groupe_tx, GROUPE_HORIZON_JOURS,
};
use super::*;
use ent_types::internat_vie::*;

const NATURES_EVENEMENT: [&str; 7] = [
    "incident",
    "comportement",
    "intrusion",
    "alarme",
    "malaise",
    "degradation",
    "objet_dangereux",
];
const NIVEAUX: [&str; 3] = ["information", "important", "critique"];
const MESURES: [&str; 8] = [
    "aucune",
    "adultes_sur_place",
    "direction_prevenue",
    "secours_appeles",
    "chambre_separee",
    "objet_confisque",
    "famille_prevenue",
    "autre",
];
const ROLES_EVENEMENT: [&str; 4] = ["auteur_presume", "victime", "temoin", "present"];
const MOTIFS_MASQUAGE: [&str; 5] = [
    "erreur",
    "donnee_sensible",
    "hors_perimetre",
    "doublon",
    "autre",
];
/// Un fait de liaison se date de la nuit courante ou de l'une des 7 précédentes (on rattrape, on ne réécrit pas).
const LIAISON_ANTIDATE_NUITS: i64 = 7;
/// Un événement se déclare au plus 48 h après les faits, jamais dans le futur (5 minutes d'écart d'horloge tolérées).
const EVENEMENT_ANTIDATE_HEURES: i64 = 48;
const EVENEMENT_FUTUR_TOLERE_MINUTES: i64 = 5;
const MAX_PARTICIPANTS_EVENEMENT: usize = 60;

/// Catalogue par défaut (§11.1) : (code, libellé, pour le jour, valorisation, ordre). Aucune catégorie de santé.
const CODES_DEFAUT: [(&str, &str, bool, bool, i16); 10] = [
    ("fatigue", "Fatigué, couché tôt", false, false, 1),
    (
        "conflit_regle",
        "Conflit dans la chambre, réglé",
        false,
        false,
        2,
    ),
    ("demande_cpe", "A demandé à parler au CPE", true, false, 3),
    (
        "travail_non_fait",
        "Travail non fait à l'étude",
        true,
        false,
        4,
    ),
    (
        "comportement_remarquable",
        "Comportement remarquable",
        false,
        true,
        5,
    ),
    ("objet_perdu", "Objet perdu", true, false, 6),
    (
        "rappeler_famille",
        "À rappeler à la famille",
        true,
        false,
        7,
    ),
    (
        "demande_intervention",
        "Demande d'intervention technique",
        false,
        false,
        8,
    ),
    ("exercice_fait", "Exercice fait", false, false, 9),
    ("absence_etude", "Absence à l'étude", true, false, 10),
];

/// Texte court d'un fait de liaison : bornes du réglage, aucune donnée de santé (R-IT7) — message d'invite vers le 34.
fn texte_liaison(t: Option<&str>, max: i16) -> Result<Option<String>, EntDbError> {
    let Some(t) = t.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if t.chars().count() > max as usize {
        return Err(refus("texte trop long (300 caractères au plus)"));
    }
    if evoque_la_sante(t) {
        return Err(refus(
            "ce texte évoque la santé : utiliser le mouvement du socle santé, jamais la liaison",
        ));
    }
    Ok(Some(t.to_string()))
}

async fn codes_defaut_tx(conn: &mut PgConnection, etab: Uuid) -> Result<(), EntDbError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_internat_liaison_code WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if n > 0 {
        return Ok(());
    }
    for (code, libelle, jour, valo, ordre) in CODES_DEFAUT {
        sqlx::query(
            "INSERT INTO ent_internat_liaison_code (etablissement_id, code, libelle, pour_le_jour, valorisation, ordre) \
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
        )
        .bind(etab)
        .bind(code)
        .bind(libelle)
        .bind(jour)
        .bind(valo)
        .bind(ordre)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

async fn codes_tx(conn: &mut PgConnection, etab: Uuid) -> Result<Vec<LiaisonCodeDto>, EntDbError> {
    codes_defaut_tx(conn, etab).await?;
    let rows: Vec<(String, String, bool, bool, i16, bool)> = sqlx::query_as(
        "SELECT code, libelle, pour_le_jour, valorisation, ordre, actif FROM ent_internat_liaison_code \
          WHERE etablissement_id = $1 ORDER BY ordre, code",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| LiaisonCodeDto {
            code: r.0,
            libelle: r.1,
            pour_le_jour: r.2,
            valorisation: r.3,
            ordre: r.4,
            actif: r.5,
        })
        .collect())
}

fn libelle_code(l: &str) -> Result<String, EntDbError> {
    let l = l.trim();
    if l.is_empty() || l.chars().count() > 80 {
        return Err(refus("libellé du code : 1 à 80 caractères"));
    }
    if evoque_la_sante(l) {
        return Err(refus(
            "aucune catégorie de santé dans le catalogue de liaison",
        ));
    }
    Ok(l.to_string())
}

fn code_valide(c: &str) -> bool {
    let b = c.as_bytes();
    (2..=40).contains(&b.len())
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || *x == b'_')
}

async fn param_texte_max_tx(conn: &mut PgConnection, etab: Uuid) -> Result<i16, EntDbError> {
    sqlx::query("INSERT INTO ent_internat_parametres_vie (etablissement_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(etab)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    sqlx::query_scalar(
        "SELECT liaison_texte_max FROM ent_internat_parametres_vie WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

type LiaisonBrute = (
    Uuid,
    Uuid,
    String,
    String,
    Option<String>,
    NaiveDate,
    Uuid,
    bool,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

const COLONNES_LIAISON: &str =
    "l.id, l.eleve_id, l.code, c.libelle, l.texte, l.nuit, l.par_user_id, l.pour_le_jour, \
     l.masque_le, l.a_revoir_le, l.signale_pp_le, l.cree_le";

const JOINTURE_LIAISON: &str = "FROM ent_internat_liaison l JOIN ent_internat_liaison_code c \
     ON c.etablissement_id = l.etablissement_id AND c.code = l.code";

impl PgEnt {
    fn liaison_dto(
        &self,
        r: &LiaisonBrute,
        noms: &HashMap<Uuid, (String, String)>,
        proteges: &HashSet<Uuid>,
        caches: &HashSet<Uuid>,
        voir_texte: bool,
        voir_masque_texte: bool,
    ) -> LiaisonDto {
        let cache = caches.contains(&r.1);
        let masque = r.8.is_some();
        LiaisonDto {
            id: r.0,
            eleve_id: r.1,
            nom: (!cache)
                .then(|| noms.get(&r.1).map(|n| n.0.clone()))
                .flatten(),
            prenom: (!cache)
                .then(|| noms.get(&r.1).map(|n| n.1.clone()))
                .flatten(),
            protege: proteges.contains(&r.1),
            code: r.2.clone(),
            libelle: r.3.clone(),
            texte: if voir_texte && (!masque || voir_masque_texte) {
                r.4.clone()
            } else {
                None
            },
            nuit: r.5,
            par_user_id: r.6,
            pour_le_jour: r.7,
            masque,
            a_revoir: r.9.is_some(),
            signale_pp: r.10.is_some(),
            cree_le: r.11,
        }
    }

    // ───────────────────────── Catalogue (INT-247) ─────────────────────────

    /// Catalogue des faits de liaison de l'établissement (créé à la première lecture). Lecture : tout porteur d'un
    /// droit de liaison (écrire, lire, « pour le jour ») ou de gestion.
    pub async fn lister_codes_liaison_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<LiaisonCodeDto>, EntDbError> {
        self.nuit_portee(
            etab,
            actor,
            role,
            &[
                Action::InternatLiaisonEcrire,
                Action::InternatLiaisonLire,
                Action::InternatLiaisonJour,
                Action::InternatGerer,
            ],
        )
        .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = codes_tx(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        Ok(c)
    }

    /// Ajoute un code propre à l'établissement : jamais une catégorie de santé.
    pub async fn creer_code_liaison_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &LiaisonCodeCreationDto,
        maintenant: DateTime<Utc>,
    ) -> Result<LiaisonCodeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        if !code_valide(&b.code) {
            return Err(refus(
                "code invalide (minuscules, chiffres et soulignés, 2 à 40 caractères)",
            ));
        }
        if evoque_la_sante(&b.code.replace('_', " ")) {
            return Err(refus(
                "aucune catégorie de santé dans le catalogue de liaison",
            ));
        }
        let libelle = libelle_code(&b.libelle)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        codes_defaut_tx(&mut tx, etab).await?;
        let n = sqlx::query(
            "INSERT INTO ent_internat_liaison_code (etablissement_id, code, libelle, pour_le_jour, ordre, modifie_le) \
             VALUES ($1,$2,$3,$4, COALESCE((SELECT max(ordre) + 1 FROM ent_internat_liaison_code WHERE etablissement_id = $1), 1), $5) \
             ON CONFLICT DO NOTHING",
        )
        .bind(etab)
        .bind(&b.code)
        .bind(&libelle)
        .bind(b.pour_le_jour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("ce code existe déjà"));
        }
        let c = codes_tx(&mut tx, etab)
            .await?
            .into_iter()
            .find(|c| c.code == b.code);
        tx.commit().await.map_err(db)?;
        c.ok_or_else(|| DomainError::NotFound.into())
    }

    /// Modifie un code (libellé, « pour le jour », actif, ordre). Un changement de « pour le jour » ne s'applique qu'aux
    /// faits écrits ensuite (la valeur est figée sur chaque fait).
    pub async fn modifier_code_liaison_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        code: &str,
        b: &LiaisonCodeSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<LiaisonCodeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        let libelle = b.libelle.as_deref().map(libelle_code).transpose()?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        codes_defaut_tx(&mut tx, etab).await?;
        let n = sqlx::query(
            "UPDATE ent_internat_liaison_code SET libelle = COALESCE($3, libelle), pour_le_jour = COALESCE($4, pour_le_jour), \
                    actif = COALESCE($5, actif), ordre = COALESCE($6, ordre), modifie_le = $7 \
              WHERE etablissement_id = $1 AND code = $2",
        )
        .bind(etab)
        .bind(code)
        .bind(libelle)
        .bind(b.pour_le_jour)
        .bind(b.actif)
        .bind(b.ordre)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        let c = codes_tx(&mut tx, etab)
            .await?
            .into_iter()
            .find(|c| c.code == code);
        tx.commit().await.map_err(db)?;
        c.ok_or_else(|| DomainError::NotFound.into())
    }

    // ───────────────────────── Faits de liaison ─────────────────────────

    /// INT-180 : écrit un fait de liaison codé sur un interne de son périmètre (R-IT30). L'élève doit être interne la
    /// nuit du fait ; la nuit est la courante ou l'une des 7 précédentes ; le texte est court et sans santé.
    pub async fn ecrire_liaison_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &LiaisonSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<LiaisonDto, EntDbError> {
        self.personnel_exiger(
            etab,
            actor,
            role,
            &[Action::InternatLiaisonEcrire],
            b.eleve_id,
        )
        .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let max = param_texte_max_tx(&mut tx, etab).await?;
        let texte = texte_liaison(b.texte.as_deref(), max)?;
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let nuit = b.nuit.unwrap_or(courante);
        if nuit > courante || nuit < courante - Duration::days(LIAISON_ANTIDATE_NUITS) {
            return Err(refus(
                "la nuit d'un fait de liaison est la nuit courante ou l'une des 7 précédentes",
            ));
        }
        codes_defaut_tx(&mut tx, etab).await?;
        let c: Option<(bool, bool)> = sqlx::query_as(
            "SELECT pour_le_jour, actif FROM ent_internat_liaison_code WHERE etablissement_id = $1 AND code = $2",
        )
        .bind(etab)
        .bind(&b.code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((pour_le_jour, actif)) = c else {
            return Err(refus("code de liaison inconnu"));
        };
        if !actif {
            return Err(refus("ce code de liaison n'est plus utilisé"));
        }
        if !est_interne_tx(&mut tx, etab, b.eleve_id, nuit).await? {
            return Err(refus("cet élève n'est pas interne cette nuit-là"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_liaison (id, etablissement_id, eleve_id, code, texte, nuit, par_user_id, pour_le_jour, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(&b.code)
        .bind(texte.as_deref())
        .bind(nuit)
        .bind(actor)
        .bind(pour_le_jour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_LIAISON_ECRITE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "code": b.code, "nuit": nuit.to_string(), "pour_le_jour": pour_le_jour }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = self
            .liaison_lire_une_tx(&mut tx, etab, actor, role, id, true)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    async fn liaison_lire_une_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        voir_texte: bool,
    ) -> Result<LiaisonDto, EntDbError> {
        let sql = format!("SELECT {COLONNES_LIAISON} {JOINTURE_LIAISON} WHERE l.etablissement_id = $1 AND l.id = $2");
        let r: LiaisonBrute = sqlx::query_as(&sql)
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let (noms, proteges, caches) = self
            .noms_pour_personnel(etab, actor, role, conn, &[r.1])
            .await?;
        Ok(self.liaison_dto(&r, &noms, &proteges, &caches, voir_texte, true))
    }

    /// INT-184 : chronologie de liaison d'un interne (équipe, CPE, direction ; portée vérifiée). Journalisée (R-IT38).
    /// Un fait masqué reste visible comme tel, son texte n'est lu que par la gestion (`internat.gerer`).
    pub async fn liaison_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<LiaisonDto>, EntDbError> {
        self.personnel_exiger(etab, actor, role, &[Action::InternatLiaisonLire], eleve)
            .await?;
        let gestion = self
            .a_action_etablissement(etab, actor, role, Action::InternatGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        crate::internat_nuit::journaliser_lecture_tx(
            &mut tx,
            etab,
            actor,
            "liaison",
            Some(eleve),
            None,
            maintenant,
        )
        .await?;
        let sql = format!(
            "SELECT {COLONNES_LIAISON} {JOINTURE_LIAISON} WHERE l.etablissement_id = $1 AND l.eleve_id = $2 \
              ORDER BY l.nuit DESC, l.cree_le DESC, l.id LIMIT 500"
        );
        let rows: Vec<LiaisonBrute> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(eleve)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let (noms, proteges, caches) = self
            .noms_pour_personnel(etab, actor, role, &mut tx, &[eleve])
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .iter()
            .map(|r| self.liaison_dto(r, &noms, &proteges, &caches, true, gestion))
            .collect())
    }

    /// Faits de liaison d'une nuit. `internat.liaison.lire` : tous les faits de sa portée avec leur texte. Seul
    /// `internat.liaison.jour` (vie scolaire de jour, le matin) : les faits « pour le jour » non masqués, **sans texte**
    /// (R-IT8). Sans l'un ni l'autre : refusé.
    pub async fn liaison_nuit_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<LiaisonDto>, EntDbError> {
        let complet = self
            .nuit_portee(etab, actor, role, &[Action::InternatLiaisonLire])
            .await;
        let (portee, voir_texte) = match complet {
            Ok(p) => (p, true),
            Err(EntDbError::Forbidden) => (
                self.nuit_portee(etab, actor, role, &[Action::InternatLiaisonJour])
                    .await?,
                false,
            ),
            Err(e) => return Err(e),
        };
        let gestion = voir_texte
            && self
                .a_action_etablissement(etab, actor, role, Action::InternatGerer)
                .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let sql = format!(
            "SELECT {COLONNES_LIAISON} {JOINTURE_LIAISON} WHERE l.etablissement_id = $1 AND l.nuit = $2 \
               AND ($3 OR (l.pour_le_jour AND l.masque_le IS NULL)) ORDER BY l.cree_le, l.id LIMIT 1000"
        );
        let mut rows: Vec<LiaisonBrute> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(nuit)
            .bind(voir_texte)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(p) = &portee {
            rows.retain(|r| p.contains(&r.1));
        }
        let ids: Vec<Uuid> = rows.iter().map(|r| r.1).collect();
        crate::internat_nuit::journaliser_lecture_eleves_tx(
            &mut tx, etab, actor, "liaison", None, None, &ids, maintenant,
        )
        .await?;
        let (noms, proteges, caches) = self
            .noms_pour_personnel(etab, actor, role, &mut tx, &ids)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .iter()
            .map(|r| self.liaison_dto(r, &noms, &proteges, &caches, voir_texte, gestion))
            .collect())
    }

    /// INT-182, INT-183 : gestes du CPE sur un fait (masquer avec motif, « à revoir », signaler au professeur principal
    /// SANS texte). Chaque geste est tracé (auteur, instant) ; masquer et signaler sont définitifs.
    pub async fn geste_liaison_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &LiaisonGesteDto,
        maintenant: DateTime<Utc>,
    ) -> Result<LiaisonDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        let (acte, motif) = match b.geste.as_str() {
            "masquer" => {
                let m = b
                    .motif_code
                    .as_deref()
                    .filter(|m| MOTIFS_MASQUAGE.contains(m));
                if m.is_none() {
                    return Err(refus("masquer un fait exige un motif codé (erreur, donnee_sensible, hors_perimetre, doublon, autre)"));
                }
                ("masquage", m)
            }
            "a_revoir" => ("a_revoir", None),
            "lever_a_revoir" => ("a_revoir_leve", None),
            "signaler_pp" => ("signal_pp", None),
            _ => {
                return Err(refus(
                    "geste inconnu (masquer, a_revoir, lever_a_revoir, signaler_pp)",
                ))
            }
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(
            Uuid,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        )> = sqlx::query_as(
            "SELECT eleve_id, masque_le, a_revoir_le, signale_pp_le FROM ent_internat_liaison \
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((eleve, masque, a_revoir, pp)) = r else {
            return Err(DomainError::NotFound.into());
        };
        self.exiger_eleve_visible(etab, actor, role, eleve).await?;
        if masque.is_some() {
            return Err(conflit("ce fait est masqué : plus aucun geste"));
        }
        let colonnes = match acte {
            "masquage" => "masque_le = $3, masque_par_user_id = $4",
            "a_revoir" if a_revoir.is_some() => return Err(conflit("ce fait est déjà à revoir")),
            "a_revoir" => "a_revoir_le = $3, a_revoir_par_user_id = $4",
            "a_revoir_leve" if a_revoir.is_none() => {
                return Err(conflit("ce fait n'est pas à revoir"))
            }
            "a_revoir_leve" => "a_revoir_le = NULL, a_revoir_par_user_id = NULL",
            _ if pp.is_some() => {
                return Err(conflit("ce fait est déjà signalé au professeur principal"))
            }
            _ => "signale_pp_le = $3, signale_pp_par_user_id = $4",
        };
        let sql = format!(
            "UPDATE ent_internat_liaison SET {colonnes} WHERE etablissement_id = $1 AND id = $2"
        );
        sqlx::query(&sql)
            .bind(etab)
            .bind(id)
            .bind(maintenant)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_internat_liaison_fait (id, etablissement_id, liaison_id, acte, motif_code, par_user_id, a) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(acte)
        .bind(motif)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if matches!(acte, "masquage" | "signal_pp") {
            let nom = if acte == "masquage" {
                types::INTERNAT_LIAISON_MASQUEE
            } else {
                types::INTERNAT_SIGNAL_PP
            };
            publier(
                &mut tx,
                etab,
                nom,
                AggregateKind::Internat,
                id,
                Some(eleve),
                json!({ "acte": acte, "motif": motif }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let dto = self
            .liaison_lire_une_tx(&mut tx, etab, actor, role, id, true)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // ───────────────────────── Événements de nuit ─────────────────────────

    /// INT-185 : déclare un événement de nuit. Les participants élèves doivent tous être dans la portée du déclarant
    /// (R-IT30) ; un « malaise » n'a aucun texte libre et émet un signal codé vers le socle santé ; un niveau
    /// « critique » émet l'alerte d'urgence avec la seule position (zone, chambre).
    pub async fn declarer_evenement_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &EvenementSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<EvenementDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatEvenementDeclarer])
            .await?;
        if !NATURES_EVENEMENT.contains(&b.nature.as_str()) {
            return Err(refus("nature d'événement inconnue"));
        }
        let niveau = b.niveau.as_deref().unwrap_or("information");
        if !NIVEAUX.contains(&niveau) {
            return Err(refus("niveau inconnu (information, important, critique)"));
        }
        if b.mesures_code
            .as_deref()
            .is_some_and(|m| !MESURES.contains(&m))
        {
            return Err(refus("mesure inconnue"));
        }
        if b.participants.len() > MAX_PARTICIPANTS_EVENEMENT {
            return Err(refus("trop de participants"));
        }
        let a = b.a.unwrap_or(maintenant);
        if a > maintenant + Duration::minutes(EVENEMENT_FUTUR_TOLERE_MINUTES)
            || a < maintenant - Duration::hours(EVENEMENT_ANTIDATE_HEURES)
        {
            return Err(refus("l'heure de l'événement est invalide (jamais dans le futur, 48 heures au plus en arrière)"));
        }
        let complement = match (b.nature.as_str(), b.complement.as_deref().map(str::trim).filter(|c| !c.is_empty())) {
            ("malaise", Some(_)) => {
                return Err(refus("un malaise n'a pas de texte libre : le signal part vers le mouvement du socle santé"))
            }
            (_, Some(c)) => {
                if c.chars().count() > 1000 {
                    return Err(refus("complément trop long (1000 caractères)"));
                }
                if evoque_la_sante(c) {
                    return Err(refus("ce texte évoque la santé : utiliser le mouvement du socle santé"));
                }
                Some(c.to_string())
            }
            _ => None,
        };
        // Participants : exactement un élève ou un adulte ; pas de doublon.
        let mut vus_e: HashSet<(Uuid, String)> = HashSet::new();
        let mut vus_u: HashSet<Uuid> = HashSet::new();
        for p in &b.participants {
            if !ROLES_EVENEMENT.contains(&p.role.as_str()) {
                return Err(refus("rôle de participant inconnu"));
            }
            match (p.eleve_id, p.user_id) {
                (Some(e), None) => {
                    if !vus_e.insert((e, p.role.clone())) {
                        return Err(refus("participant en double"));
                    }
                    if portee.as_ref().is_some_and(|s| !s.contains(&e)) {
                        return Err(EntDbError::Forbidden);
                    }
                    self.exiger_eleve_visible(etab, actor, role, e).await?;
                }
                (None, Some(u)) => {
                    if !matches!(p.role.as_str(), "temoin" | "present") {
                        return Err(refus("un adulte est témoin ou présent"));
                    }
                    if !vus_u.insert(u) {
                        return Err(refus("participant en double"));
                    }
                }
                _ => {
                    return Err(refus(
                        "un participant est un élève ou un adulte, jamais les deux",
                    ))
                }
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        for u in &vus_u {
            if !est_personnel_tx(&mut tx, etab, *u, jour).await? {
                return Err(refus(
                    "un adulte présent est un compte du personnel de l'établissement",
                ));
            }
        }
        for (e, _) in &vus_e {
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(e)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(DomainError::NotFound.into());
            }
        }
        for (table, id) in [
            ("ent_internat_zone", b.zone_id),
            ("ent_internat_chambre", b.chambre_id),
        ] {
            if let Some(i) = id {
                let sql = format!(
                    "SELECT EXISTS (SELECT 1 FROM {table} WHERE etablissement_id = $1 AND id = $2)"
                );
                let ok: bool = sqlx::query_scalar(&sql)
                    .bind(etab)
                    .bind(i)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if !ok {
                    return Err(refus("zone ou chambre inconnue"));
                }
            }
        }
        let nuit = nuit_a_tx(&mut tx, etab, a).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_evenement \
             (id, etablissement_id, nature, niveau, a, nuit, zone_id, chambre_id, mesures_code, declare_par_user_id, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(id)
        .bind(etab)
        .bind(&b.nature)
        .bind(niveau)
        .bind(a)
        .bind(nuit)
        .bind(b.zone_id)
        .bind(b.chambre_id)
        .bind(b.mesures_code.as_deref())
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for p in &b.participants {
            sqlx::query(
                "INSERT INTO ent_internat_evenement_participant (etablissement_id, evenement_id, eleve_id, user_id, role) \
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(etab)
            .bind(id)
            .bind(p.eleve_id)
            .bind(p.user_id)
            .bind(&p.role)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        if let Some(c) = &complement {
            sqlx::query(
                "INSERT INTO ent_internat_evenement_complement (id, etablissement_id, evenement_id, texte, par_user_id, cree_le) \
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(c)
            .bind(actor)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let nb_eleves = vus_e.iter().map(|(e, _)| *e).collect::<HashSet<_>>().len();
        publier(
            &mut tx,
            etab,
            types::INTERNAT_EVENEMENT_DECLARE,
            AggregateKind::Internat,
            id,
            None,
            json!({ "nature": b.nature, "niveau": niveau, "nuit": nuit.to_string(), "zone_id": b.zone_id,
                    "chambre_id": b.chambre_id, "nb_eleves": nb_eleves }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        if b.nature == "malaise" {
            // Signal codé vers le mouvement du socle santé (34) : identifiants et position seulement.
            for (e, _) in vus_e.iter().filter(|(_, r)| r != "temoin") {
                publier(
                    &mut tx,
                    etab,
                    types::INTERNAT_MALAISE_SIGNALE,
                    AggregateKind::Internat,
                    id,
                    Some(*e),
                    json!({ "nuit": nuit.to_string(), "zone_id": b.zone_id, "chambre_id": b.chambre_id }),
                    Some(actor),
                    Causalite::default(),
                )
                .await?;
            }
        }
        if niveau == "critique" {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ALERTE,
                AggregateKind::Internat,
                id,
                None,
                json!({ "niveau": niveau, "zone_id": b.zone_id, "chambre_id": b.chambre_id }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let dto = self
            .evenement_dto_tx(&mut tx, etab, actor, role, id, None)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    async fn evenement_dto_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        portee: Option<&HashSet<Uuid>>,
    ) -> Result<EvenementDto, EntDbError> {
        type Evt = (
            String,
            String,
            DateTime<Utc>,
            NaiveDate,
            Option<Uuid>,
            Option<Uuid>,
            Option<String>,
            Uuid,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        );
        let e: Evt = sqlx::query_as(
            "SELECT nature, niveau, a, nuit, zone_id, chambre_id, mesures_code, declare_par_user_id, transmis_13_le, \
                    intervention_demandee_le FROM ent_internat_evenement WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let mut parts: Vec<(Option<Uuid>, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT eleve_id, user_id, role FROM ent_internat_evenement_participant \
              WHERE etablissement_id = $1 AND evenement_id = $2 ORDER BY role, eleve_id, user_id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        if let Some(p) = portee {
            parts.retain(|x| x.0.is_none_or(|e| p.contains(&e)));
        }
        let ids: Vec<Uuid> = parts.iter().filter_map(|p| p.0).collect();
        let (noms, proteges, caches) = self
            .noms_pour_personnel(etab, actor, role, conn, &ids)
            .await?;
        let complements: Vec<(Uuid, String, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, texte, par_user_id, cree_le FROM ent_internat_evenement_complement \
              WHERE etablissement_id = $1 AND evenement_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(EvenementDto {
            id,
            nature: e.0,
            niveau: e.1,
            a: e.2,
            nuit: e.3,
            zone_id: e.4,
            chambre_id: e.5,
            mesures_code: e.6,
            declare_par_user_id: e.7,
            transmis_13_le: e.8,
            intervention_demandee_le: e.9,
            participants: parts
                .into_iter()
                .map(|(el, us, r)| {
                    let cache = el.is_some_and(|x| caches.contains(&x));
                    ParticipantEvenementDto {
                        eleve_id: el,
                        nom: el
                            .filter(|_| !cache)
                            .and_then(|x| noms.get(&x).map(|n| n.0.clone())),
                        prenom: el
                            .filter(|_| !cache)
                            .and_then(|x| noms.get(&x).map(|n| n.1.clone())),
                        protege: el.is_some_and(|x| proteges.contains(&x)),
                        user_id: us,
                        role: r,
                    }
                })
                .collect(),
            complements: complements
                .into_iter()
                .map(|(i, t, p, c)| ComplementDto {
                    id: i,
                    texte: t,
                    par_user_id: p,
                    cree_le: c,
                })
                .collect(),
        })
    }

    /// Événements d'une nuit (équipe, CPE, direction ; portée vérifiée). Un événement dont un participant élève sort de
    /// la portée du lecteur n'est pas montré (R-IT30). Lecture journalisée (R-IT38).
    pub async fn evenements_nuit_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<EvenementDto>, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLiaisonLire])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_internat_evenement WHERE etablissement_id = $1 AND nuit = $2 ORDER BY a, id LIMIT 500",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        let mut concernes: Vec<Uuid> = Vec::new();
        for id in ids {
            let d = self
                .evenement_dto_tx(&mut tx, etab, actor, role, id, None)
                .await?;
            let eleves: Vec<Uuid> = d.participants.iter().filter_map(|p| p.eleve_id).collect();
            if portee
                .as_ref()
                .is_some_and(|p| eleves.iter().any(|e| !p.contains(e)))
            {
                continue;
            }
            concernes.extend(eleves);
            out.push(d);
        }
        crate::internat_nuit::journaliser_lecture_eleves_tx(
            &mut tx,
            etab,
            actor,
            "evenement",
            None,
            None,
            &concernes,
            maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Un événement : mêmes règles de portée et de journal.
    pub async fn evenement_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<EvenementDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLiaisonLire])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = self
            .evenement_dto_tx(&mut tx, etab, actor, role, id, None)
            .await?;
        let eleves: Vec<Uuid> = d.participants.iter().filter_map(|p| p.eleve_id).collect();
        if portee
            .as_ref()
            .is_some_and(|p| eleves.iter().any(|e| !p.contains(e)))
        {
            return Err(DomainError::NotFound.into());
        }
        crate::internat_nuit::journaliser_lecture_eleves_tx(
            &mut tx,
            etab,
            actor,
            "evenement",
            None,
            None,
            &eleves,
            maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// INT-186 : complète un événement par un fait ajouté (jamais de modification ni de suppression). L'auteur de la
    /// déclaration (qui garde le droit de déclarer) ou la gestion de l'internat.
    pub async fn completer_evenement_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ComplementSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<EvenementDto, EntDbError> {
        let texte = b.texte.trim();
        if texte.is_empty() || texte.chars().count() > 1000 {
            return Err(refus("complément : 1 à 1000 caractères"));
        }
        if evoque_la_sante(texte) {
            return Err(refus(
                "ce texte évoque la santé : utiliser le mouvement du socle santé",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT declare_par_user_id, nature FROM ent_internat_evenement WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((auteur, nature)) = e else {
            return Err(DomainError::NotFound.into());
        };
        let est_auteur = auteur == actor
            && self
                .a_action_etablissement(etab, actor, role, Action::InternatEvenementDeclarer)
                .await?;
        if !est_auteur
            && !self
                .a_action_etablissement(etab, actor, role, Action::InternatGerer)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if nature == "malaise" {
            return Err(refus(
                "un malaise n'a pas de texte libre : passer par le mouvement du socle santé",
            ));
        }
        sqlx::query(
            "INSERT INTO ent_internat_evenement_complement (id, etablissement_id, evenement_id, texte, par_user_id, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(texte)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_EVENEMENT_COMPLETE,
            AggregateKind::Internat,
            id,
            None,
            json!({ "nature": nature }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = self
            .evenement_dto_tx(&mut tx, etab, actor, role, id, None)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-187 : transmet l'événement au socle 13 pour qualification (une seule fois). Jamais pour un malaise (santé)
    /// ni pour une alarme sans élève.
    pub async fn transmettre_evenement_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<EvenementDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e: Option<(String, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT nature, transmis_13_le FROM ent_internat_evenement WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((nature, deja)) = e else {
            return Err(DomainError::NotFound.into());
        };
        if matches!(nature.as_str(), "malaise" | "alarme") {
            return Err(refus(
                "ce type d'événement ne se transmet pas à la discipline",
            ));
        }
        if deja.is_some() {
            return Err(conflit("cet événement est déjà transmis"));
        }
        sqlx::query(
            "UPDATE ent_internat_evenement SET transmis_13_le = $3, transmis_13_par_user_id = $4 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_EVENEMENT_TRANSMIS,
            AggregateKind::Internat,
            id,
            None,
            json!({ "nature": nature, "vers": "discipline" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = self
            .evenement_dto_tx(&mut tx, etab, actor, role, id, None)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-188 : demande d'intervention technique sur une chambre ou une zone (socle 44) **sans nom d'élève**. Une fois
    /// par événement ; l'événement doit porter une position.
    pub async fn demander_intervention_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<EvenementDto, EntDbError> {
        self.nuit_portee(
            etab,
            actor,
            role,
            &[Action::InternatEvenementDeclarer, Action::InternatGerer],
        )
        .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e: Option<(Option<Uuid>, Option<Uuid>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT zone_id, chambre_id, intervention_demandee_le FROM ent_internat_evenement \
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((zone, chambre, deja)) = e else {
            return Err(DomainError::NotFound.into());
        };
        if zone.is_none() && chambre.is_none() {
            return Err(refus(
                "une demande d'intervention exige une chambre ou une zone",
            ));
        }
        if deja.is_some() {
            return Err(conflit(
                "une intervention est déjà demandée pour cet événement",
            ));
        }
        sqlx::query(
            "UPDATE ent_internat_evenement SET intervention_demandee_le = $3, intervention_demandee_par_user_id = $4 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_INTERVENTION_DEMANDEE,
            AggregateKind::Internat,
            id,
            None,
            json!({ "zone_id": zone, "chambre_id": chambre }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = self
            .evenement_dto_tx(&mut tx, etab, actor, role, id, None)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-189 : ouvre le circuit de préoccupation du socle 36. **Aucun contenu** : l'adulte, l'élève, l'instant (R-IT29).
    pub async fn ouvrir_preoccupation_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &PreoccupationDto,
        maintenant: DateTime<Utc>,
    ) -> Result<PreoccupationAccuseDto, EntDbError> {
        self.personnel_exiger(
            etab,
            actor,
            role,
            &[Action::InternatEvenementDeclarer],
            b.eleve_id,
        )
        .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_internat_preoccupation (id, etablissement_id, eleve_id, par_user_id, a) VALUES ($1,$2,$3,$4,$5)")
            .bind(id)
            .bind(etab)
            .bind(b.eleve_id)
            .bind(actor)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_PREOCCUPATION_SIGNALEE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "a": maintenant.to_rfc3339() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(PreoccupationAccuseDto { id, a: maintenant })
    }

    // ───────────────────────── Temps du soir : études et activités ─────────────────────────

    /// INT-190, INT-192 : crée une étude ou une activité du soir (`internat.gerer`).
    pub async fn creer_temps_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &TempsSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<TempsDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        if !matches!(b.nature.as_str(), "etude" | "activite") {
            return Err(refus("nature inconnue (etude, activite)"));
        }
        let libelle = b.libelle.trim();
        let lieu = b.lieu.as_deref().map(str::trim).filter(|l| !l.is_empty());
        if libelle.is_empty()
            || libelle.chars().count() > 120
            || lieu.is_some_and(|l| l.chars().count() > 80)
        {
            return Err(refus("libellé (1 à 120) ou lieu (80) invalide"));
        }
        if evoque_la_sante(libelle) || lieu.is_some_and(evoque_la_sante) {
            return Err(refus(
                "aucune donnée de santé dans l'internat : le texte évoque la santé",
            ));
        }
        if b.fin <= b.debut || b.fin > b.debut + Duration::hours(12) {
            return Err(refus("horaires invalides (12 heures au plus)"));
        }
        if b.debut < maintenant - Duration::hours(1)
            || b.debut > maintenant + Duration::days(GROUPE_HORIZON_JOURS)
        {
            return Err(refus(
                "un temps du soir se crée à l'avance (un an au plus), pas rétroactivement",
            ));
        }
        if b.places.is_some_and(|p| !(1..=500).contains(&p)) {
            return Err(refus("nombre de places invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(u) = b.encadrant_user_id {
            let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
            if !est_personnel_tx(&mut tx, etab, u, jour).await? {
                return Err(refus(
                    "l'encadrant est un compte du personnel de l'établissement",
                ));
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_temps (id, etablissement_id, nature, libelle, lieu, debut, fin, encadrant_user_id, places, \
                    cree_par_user_id, cree_le, modifie_le) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11)",
        )
        .bind(id)
        .bind(etab)
        .bind(&b.nature)
        .bind(libelle)
        .bind(lieu)
        .bind(b.debut)
        .bind(b.fin)
        .bind(b.encadrant_user_id)
        .bind(b.places)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_TEMPS_CREE,
            AggregateKind::Internat,
            id,
            None,
            json!({ "nature": b.nature, "debut": b.debut.to_rfc3339(), "fin": b.fin.to_rfc3339() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = temps_dto_tx(&mut tx, etab, id, true, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Temps du soir d'une période (personnel : droit de lecture ou de gestion). Les inscrits sont bornés à la portée.
    pub async fn lister_temps_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<TempsDto>, EntDbError> {
        if au < du || (au - du).num_days() > 366 {
            return Err(refus("période invalide (366 jours au plus)"));
        }
        let portee = self
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
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_internat_temps WHERE etablissement_id = $1 \
                AND (debut AT TIME ZONE ent_internat_fuseau($1))::date BETWEEN $2 AND $3 ORDER BY debut, id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for id in ids {
            out.push(temps_dto_tx(&mut tx, etab, id, true, portee.as_ref()).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Annule un temps du soir (daté, jamais supprimé) ; les inscrits sont prévenus par événement.
    pub async fn annuler_temps_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<TempsDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (statut, debut, fin) = temps_ligne_tx(&mut tx, etab, id, true).await?;
        if statut != "prevu" {
            return Err(conflit("ce temps est déjà annulé"));
        }
        sqlx::query("UPDATE ent_internat_temps SET statut = 'annule', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let inscrits: Vec<Uuid> = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND temps_id = $2 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for e in inscrits {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_TEMPS_ANNULE,
                AggregateKind::Internat,
                id,
                Some(e),
                json!({ "debut": debut.to_rfc3339(), "fin": fin.to_rfc3339() }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let dto = temps_dto_tx(&mut tx, etab, id, true, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-191 : inscrit un interne à un temps du soir. Le personnel (`internat.gerer`) jusqu'à la fin du temps, l'élève
    /// lui-même (mineur ou majeur) avant son début. Places contrôlées sous verrou ; pas de chevauchement avec un autre
    /// temps ni avec une sortie de groupe où l'élève est autorisé ou parti.
    pub async fn inscrire_temps_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &InscriptionTempsDto,
        maintenant: DateTime<Utc>,
    ) -> Result<TempsDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let propre = matches!(
            self.famille_tx(&mut tx, etab, actor, role, b.eleve_id, jour)
                .await?,
            Some(Famille {
                qualite: Qualite::Eleve | Qualite::EleveMajeur,
                ..
            })
        );
        let gestion = !propre
            && self
                .a_action_etablissement(etab, actor, role, Action::InternatGerer)
                .await?;
        if !propre && !gestion {
            return Err(EntDbError::Forbidden);
        }
        let (statut, debut, fin) = temps_ligne_tx(&mut tx, etab, id, true).await?;
        if statut != "prevu" {
            return Err(conflit("ce temps est annulé"));
        }
        if (propre && maintenant >= debut) || maintenant >= fin {
            return Err(conflit("les inscriptions sont closes"));
        }
        let jour_temps = jour_local_tx(&mut tx, etab, debut).await?;
        if !est_interne_tx(&mut tx, etab, b.eleve_id, jour_temps).await? {
            return Err(refus("cet élève n'est pas interne ce jour-là"));
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND temps_id = $2 \
                              AND eleve_id = $3 AND retire_le IS NULL)",
        )
        .bind(etab)
        .bind(id)
        .bind(b.eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit("cet élève est déjà inscrit"));
        }
        let places: Option<i16> = sqlx::query_scalar(
            "SELECT places FROM ent_internat_temps WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(p) = places {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND temps_id = $2 AND retire_le IS NULL",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n >= i64::from(p) {
                return Err(conflit("plus de place"));
            }
        }
        let chevauche: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_temps_participant p JOIN ent_internat_temps t \
                              ON t.etablissement_id = p.etablissement_id AND t.id = p.temps_id \
                             WHERE p.etablissement_id = $1 AND p.eleve_id = $2 AND p.retire_le IS NULL \
                               AND t.statut = 'prevu' AND t.id <> $3 AND t.debut < $5 AND t.fin > $4) \
                 OR EXISTS (SELECT 1 FROM ent_internat_sortie_groupe_participant p JOIN ent_internat_sortie_groupe g \
                              ON g.etablissement_id = p.etablissement_id AND g.id = p.sortie_groupe_id \
                             WHERE p.etablissement_id = $1 AND p.eleve_id = $2 AND p.etat IN ('autorise', 'parti') \
                               AND g.statut IN ('soumise', 'acceptee') AND g.debut < $5 AND g.fin > $4)",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .bind(id)
        .bind(debut)
        .bind(fin)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if chevauche {
            return Err(conflit(
                "cet élève a déjà un temps ou une sortie à ce moment-là",
            ));
        }
        sqlx::query(
            "INSERT INTO ent_internat_temps_participant (id, etablissement_id, temps_id, eleve_id, inscrit_le, inscrit_par_user_id) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(b.eleve_id)
        .bind(maintenant)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_TEMPS_INSCRIPTION,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "debut": debut.to_rfc3339(), "fin": fin.to_rfc3339(), "etat": "inscrit" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = temps_dto_tx(&mut tx, etab, id, !propre, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Désinscription (datée, jamais supprimée) : le personnel, ou l'élève avant le début.
    pub async fn desinscrire_temps_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<TempsDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let propre = matches!(
            self.famille_tx(&mut tx, etab, actor, role, eleve, jour)
                .await?,
            Some(Famille {
                qualite: Qualite::Eleve | Qualite::EleveMajeur,
                ..
            })
        );
        let gestion = !propre
            && self
                .a_action_etablissement(etab, actor, role, Action::InternatGerer)
                .await?;
        if !propre && !gestion {
            return Err(EntDbError::Forbidden);
        }
        let (_, debut, fin) = temps_ligne_tx(&mut tx, etab, id, true).await?;
        if (propre && maintenant >= debut) || maintenant >= fin {
            return Err(conflit("les inscriptions sont closes"));
        }
        let n = sqlx::query(
            "UPDATE ent_internat_temps_participant SET retire_le = $4, retire_par_user_id = $5 \
              WHERE etablissement_id = $1 AND temps_id = $2 AND eleve_id = $3 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(eleve)
        .bind(maintenant)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_TEMPS_INSCRIPTION,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "debut": debut.to_rfc3339(), "fin": fin.to_rfc3339(), "etat": "retire" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = temps_dto_tx(&mut tx, etab, id, !propre, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-194 : absence à l'étude = fait de liaison (jamais une absence de cours), rattaché à la séance.
    pub async fn absence_etude_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &InscriptionTempsDto,
        maintenant: DateTime<Utc>,
    ) -> Result<LiaisonDto, EntDbError> {
        self.personnel_exiger(
            etab,
            actor,
            role,
            &[Action::InternatLiaisonEcrire],
            b.eleve_id,
        )
        .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT nature, statut, debut FROM ent_internat_temps WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((nature, statut, debut)) = t else {
            return Err(DomainError::NotFound.into());
        };
        if nature != "etude" || statut != "prevu" {
            return Err(refus(
                "l'absence à l'étude se signale sur une étude non annulée",
            ));
        }
        if debut > maintenant {
            return Err(conflit("cette étude n'a pas commencé"));
        }
        let inscrit: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND temps_id = $2 \
                              AND eleve_id = $3 AND retire_le IS NULL)",
        )
        .bind(etab)
        .bind(id)
        .bind(b.eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !inscrit {
            return Err(refus("cet élève n'est pas inscrit à cette étude"));
        }
        let nuit = nuit_a_tx(&mut tx, etab, debut).await?;
        codes_defaut_tx(&mut tx, etab).await?;
        let pour_le_jour: bool = sqlx::query_scalar(
            "SELECT pour_le_jour FROM ent_internat_liaison_code WHERE etablissement_id = $1 AND code = 'absence_etude'",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .unwrap_or(false);
        let lid = Uuid::new_v4();
        let ins = sqlx::query(
            "INSERT INTO ent_internat_liaison (id, etablissement_id, eleve_id, code, nuit, par_user_id, pour_le_jour, temps_id, cree_le) \
             SELECT $1,$2,$3,'absence_etude',$4,$5,$6,$7,$8 \
              WHERE EXISTS (SELECT 1 FROM ent_internat_liaison_code WHERE etablissement_id = $2 AND code = 'absence_etude' AND actif)",
        )
        .bind(lid)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(nuit)
        .bind(actor)
        .bind(pour_le_jour)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if ins == 0 {
            return Err(refus(
                "le code « absence à l'étude » n'est pas actif dans le catalogue",
            ));
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_LIAISON_ECRITE,
            AggregateKind::Internat,
            lid,
            Some(b.eleve_id),
            json!({ "code": "absence_etude", "nuit": nuit.to_string(), "pour_le_jour": pour_le_jour }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = self
            .liaison_lire_une_tx(&mut tx, etab, actor, role, lid, true)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Planning d'un élève (lu par lui, ses titulaires, ou le personnel de sa portée) : études, activités, sorties de
    /// groupe des 30 prochains jours. **Jamais** le nom d'un autre élève, jamais le nombre d'inscrits (R-IT32).
    pub async fn planning_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<PlanningEleveDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self
            .famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour)
            .await?;
        let peut_repondre = famille.as_ref().is_some_and(|f| {
            f.peut_autoriser && matches!(f.qualite, Qualite::Titulaire | Qualite::EleveMajeur)
        });
        if famille.is_none() {
            tx.commit().await.map_err(db)?;
            self.personnel_exiger(
                etab,
                actor,
                role,
                &[Action::InternatLire, Action::InternatGerer],
                eleve,
            )
            .await?;
            tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        }
        let temps: Vec<(Uuid, String, String, Option<String>, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT t.id, t.nature, t.libelle, t.lieu, t.debut, t.fin FROM ent_internat_temps t \
               JOIN ent_internat_temps_participant p ON p.etablissement_id = t.etablissement_id AND p.temps_id = t.id \
              WHERE t.etablissement_id = $1 AND p.eleve_id = $2 AND p.retire_le IS NULL AND t.statut = 'prevu' \
                AND t.fin > $3 AND t.debut < $3 + interval '30 days' ORDER BY t.debut, t.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT g.id FROM ent_internat_sortie_groupe g \
               JOIN ent_internat_sortie_groupe_participant p ON p.etablissement_id = g.etablissement_id AND p.sortie_groupe_id = g.id \
              WHERE g.etablissement_id = $1 AND p.eleve_id = $2 AND g.statut IN ('soumise', 'acceptee') \
                AND p.etat NOT IN ('invite', 'retire') AND g.fin > $3 AND g.debut < $3 + interval '30 days' \
              ORDER BY g.debut, g.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut sorties = Vec::new();
        for gid in ids {
            let g = groupe_tx(&mut tx, etab, gid, false).await?;
            if let Some(mut d) = famille_dto_tx(&mut tx, etab, &g, eleve, peut_repondre).await? {
                d.peut_repondre = d.peut_repondre && maintenant < g.debut;
                sorties.push(d);
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(PlanningEleveDto {
            eleve_id: eleve,
            etudes_et_activites: temps
                .into_iter()
                .map(|t| PlanningTempsDto {
                    temps_id: t.0,
                    nature: t.1,
                    libelle: t.2,
                    lieu: t.3,
                    debut: t.4,
                    fin: t.5,
                })
                .collect(),
            sorties_groupe: sorties,
        })
    }
}

const CODES_VIGILANCE: [&str; 3] = ["suivi_rapproche", "premier_mois", "accompagner_coucher"];
/// Une vigilance se revoit chaque semaine.
const VIGILANCE_REVUE_JOURS: i64 = 7;

type VigilanceBrute = (
    Uuid,
    Uuid,
    String,
    Option<Uuid>,
    NaiveDate,
    Option<NaiveDate>,
    DateTime<Utc>,
);

fn vigilance_dto(v: VigilanceBrute, jour: NaiveDate, maintenant: DateTime<Utc>) -> VigilanceDto {
    VigilanceDto {
        id: v.0,
        eleve_id: v.1,
        code: v.2,
        referent_user_id: v.3,
        du: v.4,
        au: v.5,
        revue_le: v.6,
        a_reviser: v.6 < maintenant - Duration::days(VIGILANCE_REVUE_JOURS),
        active: v.4 <= jour && v.5.is_none_or(|a| a >= jour),
    }
}

impl PgEnt {
    /// INT-257 : pose une vigilance codée (et un référent d'internat) sur un interne. `internat.gerer`, élève visible ;
    /// aucun texte. Deux vigilances de même code ne se chevauchent pas.
    pub async fn poser_vigilance_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &VigilanceSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<VigilanceDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        self.exiger_eleve_visible(etab, actor, role, b.eleve_id)
            .await?;
        if !CODES_VIGILANCE.contains(&b.code.as_str()) {
            return Err(refus(
                "code de vigilance inconnu (suivi_rapproche, premier_mois, accompagner_coucher)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let du = b.du.unwrap_or(jour);
        if du < jour - Duration::days(30)
            || du > jour + Duration::days(366)
            || b.au.is_some_and(|a| a < du || a > du + Duration::days(730))
        {
            return Err(refus("dates de vigilance invalides"));
        }
        if let Some(r) = b.referent_user_id {
            if !est_personnel_tx(&mut tx, etab, r, jour).await? {
                return Err(refus(
                    "le référent est un compte du personnel de l'établissement",
                ));
            }
        }
        if !est_interne_tx(&mut tx, etab, b.eleve_id, du.max(jour)).await? {
            return Err(refus("cet élève n'est pas interne"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_vigilance (id, etablissement_id, eleve_id, code, referent_user_id, du, au, revue_le, \
                    revue_par_user_id, pose_par_user_id, cree_le) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$9,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(&b.code)
        .bind(b.referent_user_id)
        .bind(du)
        .bind(b.au)
        .bind(maintenant)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23P01") => conflit("une vigilance de ce type est déjà en vigueur sur cette période"),
            e => db(e),
        })?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_VIGILANCE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "code": b.code, "du": du.to_string(), "au": b.au.map(|a| a.to_string()), "acte": "posee" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let v = vigilance_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(vigilance_dto(v, jour, maintenant))
    }

    /// Revue hebdomadaire : date et auteur de la revue (une vigilance non revue depuis 7 jours est signalée).
    pub async fn reviser_vigilance_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<VigilanceDto, EntDbError> {
        self.vigilance_maj(etab, actor, role, id, maintenant, false)
            .await
    }

    /// Lève la vigilance : fin datée d'aujourd'hui (jamais supprimée).
    pub async fn lever_vigilance_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<VigilanceDto, EntDbError> {
        self.vigilance_maj(etab, actor, role, id, maintenant, true)
            .await
    }

    async fn vigilance_maj(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
        lever: bool,
    ) -> Result<VigilanceDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let v = vigilance_tx(&mut tx, etab, id).await?;
        self.exiger_eleve_visible(etab, actor, role, v.1).await?;
        if v.5.is_some_and(|a| a < jour) {
            return Err(conflit("cette vigilance est terminée"));
        }
        if lever {
            sqlx::query("UPDATE ent_internat_vigilance SET au = $3, revue_le = $4, revue_par_user_id = $5 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(jour.max(v.4))
                .bind(maintenant)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            publier(
                &mut tx,
                etab,
                types::INTERNAT_VIGILANCE,
                AggregateKind::Internat,
                id,
                Some(v.1),
                json!({ "code": v.2, "acte": "levee" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        } else {
            sqlx::query("UPDATE ent_internat_vigilance SET revue_le = $3, revue_par_user_id = $4 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(maintenant)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let v = vigilance_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(vigilance_dto(v, jour, maintenant))
    }

    /// Vigilances d'un interne (équipe d'internat, CPE, direction ; portée vérifiée) ; lecture journalisée.
    pub async fn vigilances_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<VigilanceDto>, EntDbError> {
        self.personnel_exiger(etab, actor, role, &[Action::InternatLiaisonLire], eleve)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        crate::internat_nuit::journaliser_lecture_tx(
            &mut tx,
            etab,
            actor,
            "liaison",
            Some(eleve),
            None,
            maintenant,
        )
        .await?;
        let rows: Vec<VigilanceBrute> = sqlx::query_as(
            "SELECT id, eleve_id, code, referent_user_id, du, au, revue_le FROM ent_internat_vigilance \
              WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY du DESC, id",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|v| vigilance_dto(v, jour, maintenant))
            .collect())
    }
}

async fn vigilance_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<VigilanceBrute, EntDbError> {
    sqlx::query_as(
        "SELECT id, eleve_id, code, referent_user_id, du, au, revue_le FROM ent_internat_vigilance \
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(|| DomainError::NotFound.into())
}

async fn temps_ligne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<(String, DateTime<Utc>, DateTime<Utc>), EntDbError> {
    let sql = format!(
        "SELECT statut, debut, fin FROM ent_internat_temps WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound.into())
}

async fn temps_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    avec_inscrits: bool,
    portee: Option<&HashSet<Uuid>>,
) -> Result<TempsDto, EntDbError> {
    type T = (
        String,
        String,
        Option<String>,
        DateTime<Utc>,
        DateTime<Utc>,
        Option<Uuid>,
        Option<i16>,
        String,
    );
    let t: T = sqlx::query_as(
        "SELECT nature, libelle, lieu, debut, fin, encadrant_user_id, places, statut FROM ent_internat_temps \
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    let mut inscrits: Vec<Uuid> = sqlx::query_scalar(
        "SELECT eleve_id FROM ent_internat_temps_participant WHERE etablissement_id = $1 AND temps_id = $2 \
            AND retire_le IS NULL ORDER BY eleve_id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let nb = inscrits.len() as i64;
    if let Some(p) = portee {
        inscrits.retain(|e| p.contains(e));
    }
    Ok(TempsDto {
        id,
        nature: t.0,
        libelle: t.1,
        lieu: t.2,
        debut: t.3,
        fin: t.4,
        encadrant_user_id: t.5,
        places: t.6,
        statut: t.7,
        nb_inscrits: nb,
        inscrits: if avec_inscrits { inscrits } else { Vec::new() },
    })
}
