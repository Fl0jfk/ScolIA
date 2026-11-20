//! Socle 41, lot I3, passe 3 — **« un acte tombe avec son auteur »** : la règle générale, vérifiée en UN SEUL endroit.
//!
//! Pour la sortie ou l'absence d'un **mineur**, au contrôle de départ et à chaque revue périodique : si **une seule** des
//! personnes qui ont posé un acte encore en vigueur sur elle est désormais visée (sur toute sa chaîne de fusions) par une
//! interdiction non levée et non terminée d'objet `remise`, `visite` ou `contact_eleve`, alors l'acte tombe.
//!
//! Les actes et leurs auteurs sont lus dans le **journal des faits** (`ent_internat_i3_fait`), pas seulement dans les
//! colonnes de la ligne :
//!
//! | Acte | Source de l'auteur |
//! |---|---|
//! | demande d'une sortie, annonce d'une absence | fait `demande` / `annonce` (qualité famille) ; colonne de la ligne |
//! | accord de la famille | colonne `accord_titulaire_user_id` (l'accord en vigueur) |
//! | désignation ou changement de la personne qui récupère | dernier fait `demande` / `accord_titulaire` / `reprise` qui porte une personne (`reprise` compris : un parent qui change la personne d'une sortie créée par le système en est l'auteur) |
//! | désignation du correspondant | colonne `designe_par_user_id` et fait `designation` |
//! | autorisation annuelle (socle 18) dont la sortie ou l'absence dépend | **tous** les décideurs qui l'ont accordée (`ent_acc_decision`) |
//!
//! Un acte est « en vigueur » tant qu'aucun fait `accord_remis_a_zero` ne l'a suivi. Une décision du CPE ou de la
//! direction **postérieure à la pose de l'interdiction** couvre l'acte (la revue ne défait pas ce qu'ils ont décidé).
//!
//! Effets (un seul chemin par objet, [`appliquer_sortie_tx`], [`appliquer_absence_tx`], [`appliquer_correspondant_tx`]) :
//! * sortie non partie : accord et décision du CPE retirés, personne désignée effacée, autorisation annuelle écartée,
//!   retour à « demandée » (la décision revient au CPE), fait `accord_remis_a_zero` de motif `interdiction_auteur`,
//!   événement `interdiction_auteur` ;
//! * absence non partie : personne désignée effacée, absence **bloquée** (motif `interdiction_auteur`), l'élève reste
//!   attendu, la direction tranche ;
//! * correspondant : suspendu (seul le CPE le reprend) ;
//! * déjà parti (absence ou sortie « partie ») : rien ne se défait, un **cas pour le CPE** s'ouvre (nature
//!   `interdiction_auteur`) et les nuits suivantes ne retirent plus l'élève de l'attendu tant qu'il n'a pas tranché.
//!
//! La fonction travaille **en lot** : une seule évaluation pour tous les objets d'un établissement (une requête par type
//! de lecture, jamais une par ligne), que l'appelant ait un objet (contrôle de départ) ou plusieurs (revue à la minute).

use super::*;

/// Bilan d'une revue : objets suspendus, bloqués ou remis à la décision, et objets dont la revue a échoué.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RevueBilan {
    pub suspendues: i32,
    pub echecs: i32,
}

/// Ouvre le savepoint de la revue d'UN objet, juste avant sa première écriture (jamais pour un objet sans écriture).
pub(super) async fn ouvrir_savepoint_revue(conn: &mut PgConnection, ouvert: &mut bool) -> Result<(), EntDbError> {
    if !*ouvert {
        sqlx::query("SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
        *ouvert = true;
    }
    Ok(())
}

/// Passe 4 (R7) : ferme le savepoint ouvert (`SAVEPOINT revue_objet`) autour de la revue d'UN objet. Succès : le travail
/// de l'objet est conservé (`vrai` = l'objet a été suspendu, bloqué ou remis à la décision). Échec : seul le travail de
/// cet objet est annulé, l'échec est compté et journalisé (sans donnée nominative) ; les autres objets sont revus. Seule
/// une erreur du savepoint lui-même (connexion perdue) remonte.
pub(super) async fn fin_savepoint_revue(
    conn: &mut PgConnection,
    ouvert: bool,
    resultat: Result<bool, EntDbError>,
    comptes: &mut i32,
    echecs: &mut i32,
) -> Result<(), EntDbError> {
    if !ouvert {
        // Aucune écriture n'a été tentée pour cet objet (le plus grand nombre : pas de requête de plus par ligne) : une
        // simple lecture qui échoue fait échouer la transaction, comme avant la passe 4.
        *comptes += i32::from(resultat?);
        return Ok(());
    }
    match resultat {
        Ok(vrai) => {
            sqlx::query("RELEASE SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
            if vrai {
                *comptes += 1;
            }
        }
        Err(e) => {
            sqlx::query("ROLLBACK TO SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
            sqlx::query("RELEASE SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
            *echecs += 1;
            tracing::warn!(nature = %nature_erreur(&e), "revue d'internat : un objet n'a pas pu être revu (les autres l'ont été)");
        }
    }
    Ok(())
}

/// Qualités d'un acte posé par la famille (les actes du personnel ne sont jamais visés par une interdiction).
const FAMILLE: [&str; 3] = ["titulaire", "eleve_majeur", "eleve"];

/// Ce que l'auteur a posé.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Portee {
    /// Demande de la sortie, annonce de l'absence.
    Demande,
    /// Accord de la famille sur la sortie.
    Accord,
    /// Désignation (ou changement) de la personne qui récupère l'élève.
    Personne,
    /// Désignation du correspondant.
    Designation,
    /// Autorisation annuelle (socle 18) dont l'objet dépend.
    Annuel,
}

/// Résultat de l'évaluation d'un objet : les auteurs désormais visés (et non couverts par une décision postérieure).
#[derive(Debug, Default, Clone)]
pub(super) struct Chute {
    pub auteurs: Vec<(Uuid, Portee)>,
    /// Déjà parti : la personne à qui l'élève a été remis est désormais visée.
    pub remise_visee: bool,
    /// Date de pose de l'interdiction la plus récente en cause.
    pub depuis: Option<DateTime<Utc>>,
}

impl Chute {
    pub(super) fn tombe(&self) -> bool {
        !self.auteurs.is_empty() || self.remise_visee
    }

    pub(super) fn touche(&self, p: Portee) -> bool {
        self.auteurs.iter().any(|(_, q)| *q == p)
    }
}

/// Un correspondant désigné et en vigueur.
pub(super) struct CorrespondantAuteur {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub designe_par: Uuid,
}

/// Les objets qui portent des actes.
pub(super) enum Objet<'a> {
    Sortie(&'a SortieLigne),
    Absence(&'a AbsenceLigne),
    Correspondant(&'a CorrespondantAuteur),
}

impl Objet<'_> {
    fn id(&self) -> Uuid {
        match self {
            Objet::Sortie(s) => s.id,
            Objet::Absence(a) => a.id,
            Objet::Correspondant(c) => c.id,
        }
    }

    fn eleve(&self) -> Uuid {
        match self {
            Objet::Sortie(s) => s.eleve_id,
            Objet::Absence(a) => a.eleve_id,
            Objet::Correspondant(c) => c.eleve_id,
        }
    }

    fn parti(&self) -> bool {
        match self {
            Objet::Sortie(s) => s.statut == "partie",
            Objet::Absence(a) => a.depart_effectif.is_some(),
            Objet::Correspondant(_) => false,
        }
    }
}

#[derive(sqlx::FromRow)]
struct FaitLu {
    sortie_id: Option<Uuid>,
    absence_id: Option<Uuid>,
    correspondant_id: Option<Uuid>,
    acte: String,
    qualite: String,
    par_user_id: Option<Uuid>,
    a: DateTime<Utc>,
    personne_id: Option<Uuid>,
    cree_le: DateTime<Utc>,
    motif_code: Option<String>,
    motif_texte: Option<String>,
}

impl FaitLu {
    /// R2 : une décision du CPE ou de la direction ne couvre la chute que si elle PORTE SUR elle : levée du blocage
    /// (`tranchee_levee`, direction) ou reprise du correspondant (CPE) dont le motif est la chute elle-même
    /// (`interdiction_auteur` : interdiction OU perte de droit de l'auteur), avec un motif écrit. Une acceptation, une
    /// autorisation de routine, une levée pour une autre cause (désaccord, par exemple) ne couvrent jamais.
    fn couvre_la_chute(&self, correspondant: bool) -> bool {
        let motif_ecrit = self.motif_texte.as_deref().is_some_and(|t| !t.trim().is_empty());
        let motif_chute = matches!(self.motif_code.as_deref(), Some("interdiction_auteur"));
        motif_ecrit
            && motif_chute
            && if correspondant {
                self.acte == "reprise" && self.qualite == "cpe"
            } else {
                self.acte == "tranchee_levee" && self.qualite == "direction"
            }
    }

    fn cible(&self) -> Option<Uuid> {
        self.sortie_id.or(self.absence_id).or(self.correspondant_id)
    }

    fn famille(&self) -> bool {
        FAMILLE.contains(&self.qualite.as_str()) && self.par_user_id.is_some()
    }
}

/// Décideurs (qui ont **accordé**, dernière décision de chacun) de l'autorisation annuelle de sortie pour un usage, par
/// élève. Une requête par usage pour tous les élèves.
pub(super) async fn deciders_annuel_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    usage: &str,
    eleves: &[Uuid],
) -> Result<HashMap<Uuid, Vec<Uuid>>, EntDbError> {
    if eleves.is_empty() {
        return Ok(HashMap::new());
    }
    let lignes: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT DISTINCT ON (x.sujet_eleve_id, x.decideur_user_id) x.sujet_eleve_id, x.decideur_user_id, u.accorde \
           FROM ent_acc_decision x \
           JOIN ent_acc_objet o ON o.etablissement_id = x.etablissement_id AND o.id = x.objet_id AND o.code = $2 \
           JOIN ent_acc_decision_usage u ON u.decision_id = x.id AND u.etablissement_id = x.etablissement_id \
           JOIN ent_acc_usage us ON us.id = u.usage_id AND us.etablissement_id = u.etablissement_id AND us.code = $3 \
          WHERE x.etablissement_id = $1 AND x.sujet_eleve_id = ANY($4) AND x.decision <> 'annulee_par_etablissement' \
            AND NOT EXISTS (SELECT 1 FROM ent_acc_decision a \
                             WHERE a.etablissement_id = x.etablissement_id AND a.annule_decision_id = x.id) \
          ORDER BY x.sujet_eleve_id, x.decideur_user_id, x.decide_le DESC, x.id DESC",
    )
    .bind(etab)
    .bind(OBJET_ACCORD_SORTIE)
    .bind(usage)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut par_eleve: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (eleve, decideur, accorde) in lignes {
        if accorde {
            par_eleve.entry(eleve).or_default().push(decideur);
        }
    }
    Ok(par_eleve)
}

/// Date de pose de l'interdiction qui vise chaque couple (élève, compte), `None` si le compte n'est pas visé. Une requête
/// pour tous les couples.
pub(super) async fn comptes_vises_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    couples: &[(Uuid, Uuid)],
    jour: NaiveDate,
) -> Result<HashMap<(Uuid, Uuid), DateTime<Utc>>, EntDbError> {
    if couples.is_empty() {
        return Ok(HashMap::new());
    }
    let eleves: Vec<Uuid> = couples.iter().map(|c| c.0).collect();
    let users: Vec<Uuid> = couples.iter().map(|c| c.1).collect();
    let lignes: Vec<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT k.e, k.u, ent_internat_user_vise($1, k.e, k.u, $4) \
           FROM unnest($2::uuid[], $3::uuid[]) AS k(e, u)",
    )
    .bind(etab)
    .bind(&eleves)
    .bind(&users)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(lignes.into_iter().filter_map(|(e, u, d)| d.map(|d| ((e, u), d))).collect())
}

/// **La fonction unique** : pour chaque objet, les auteurs désormais visés dont l'acte tombe. Les objets absents du
/// résultat n'ont rien à défaire.
pub(super) async fn actes_qui_tombent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    objets: &[Objet<'_>],
    jour: NaiveDate,
) -> Result<HashMap<Uuid, Chute>, EntDbError> {
    if objets.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<Uuid> = objets.iter().map(Objet::id).collect();
    // 1. Le journal des faits de tous les objets, en une requête.
    let faits: Vec<FaitLu> = sqlx::query_as(
        "SELECT sortie_id, absence_id, correspondant_id, acte, qualite, par_user_id, a, personne_id, cree_le, \
                motif_code, motif_texte \
           FROM ent_internat_i3_fait \
          WHERE etablissement_id = $1 \
            AND (sortie_id = ANY($2) OR absence_id = ANY($2) OR correspondant_id = ANY($2)) \
            AND acte IN ('annonce', 'demande', 'regularisation_demandee', 'accord_titulaire', 'reprise', \
                         'accord_remis_a_zero', 'acceptation', 'tranchee_levee', 'depart', 'designation') \
          ORDER BY cree_le",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut par_cible: HashMap<Uuid, Vec<&FaitLu>> = HashMap::new();
    for f in &faits {
        if let Some(c) = f.cible() {
            par_cible.entry(c).or_default().push(f);
        }
    }
    // 2. Usage d'accord de chaque type de sortie, et majorité des élèves dont une absence part seule.
    let usages: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT code, usage_accord_code FROM ent_internat_sortie_type WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .into_iter()
    .collect();
    let eleves_absence: Vec<Uuid> = objets
        .iter()
        .filter_map(|o| match o {
            Objet::Absence(a) if a.recupere_par_personne_id.is_none() && a.annonce_qualite != "personnel" => Some(a.eleve_id),
            _ => None,
        })
        .collect();
    let mineurs: HashSet<Uuid> = if eleves_absence.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_as::<_, (Uuid, bool)>(
            "SELECT e, ent_internat_eleve_majeur($1, e, $3) FROM unnest($2::uuid[]) AS e",
        )
        .bind(etab)
        .bind(&eleves_absence)
        .bind(jour)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .filter_map(|(e, majeur)| (!majeur).then_some(e))
        .collect()
    };
    // 3. Décideurs de l'autorisation annuelle, par usage (une requête par usage).
    let mut annuel_par_usage: HashMap<String, Vec<Uuid>> = HashMap::new();
    for o in objets {
        match o {
            Objet::Sortie(s) if s.par_autorisation_annuelle => {
                if let Some(u) = usages.get(&s.type_code) {
                    annuel_par_usage.entry(u.clone()).or_default().push(s.eleve_id);
                }
            }
            Objet::Absence(a) if mineurs.contains(&a.eleve_id) && a.recupere_par_personne_id.is_none() => {
                if let Some(u) = usages.get("week_end") {
                    annuel_par_usage.entry(u.clone()).or_default().push(a.eleve_id);
                }
            }
            _ => {}
        }
    }
    let mut deciders: HashMap<(String, Uuid), Vec<Uuid>> = HashMap::new();
    for (usage, eleves) in annuel_par_usage {
        for (eleve, ds) in deciders_annuel_tx(conn, etab, &usage, &eleves).await? {
            deciders.insert((usage.clone(), eleve), ds);
        }
    }
    // 4. Les auteurs de chaque objet.
    let mut auteurs: HashMap<Uuid, Vec<(Uuid, Portee)>> = HashMap::new();
    let mut remis: HashMap<Uuid, Uuid> = HashMap::new();
    for o in objets {
        let faits_o: &[&FaitLu] = par_cible.get(&o.id()).map(Vec::as_slice).unwrap_or(&[]);
        let mut v: Vec<(Uuid, Portee)> = Vec::new();
        match o {
            Objet::Sortie(s) => {
                let reset = faits_o.iter().filter(|f| f.acte == "accord_remis_a_zero").map(|f| f.cree_le).max();
                let en_vigueur = |f: &FaitLu| reset.is_none_or(|r| f.cree_le > r);
                for f in faits_o.iter().filter(|f| f.acte == "demande" && f.famille() && en_vigueur(f)) {
                    if let Some(u) = f.par_user_id {
                        v.push((u, Portee::Demande));
                    }
                }
                if let (Some(_), Some(u)) = (s.accord_titulaire_le, s.accord_titulaire_user_id) {
                    v.push((u, Portee::Accord));
                }
                if s.avec_personne_id.is_some() {
                    let dernier = faits_o
                        .iter()
                        .rfind(|f| matches!(f.acte.as_str(), "demande" | "accord_titulaire" | "reprise") && f.personne_id.is_some());
                    if let Some(f) = dernier.filter(|f| f.famille()) {
                        if let Some(u) = f.par_user_id {
                            v.push((u, Portee::Personne));
                        }
                    }
                }
                if s.par_autorisation_annuelle {
                    if let Some(ds) = usages.get(&s.type_code).and_then(|u| deciders.get(&(u.clone(), s.eleve_id))) {
                        v.extend(ds.iter().map(|d| (*d, Portee::Annuel)));
                    }
                }
                if s.statut == "partie" {
                    if let Some(p) = faits_o.iter().filter(|f| f.acte == "depart").filter_map(|f| f.personne_id).next_back() {
                        remis.insert(s.id, p);
                    }
                }
            }
            Objet::Absence(ab) => {
                if ab.annonce_qualite != "personnel" {
                    v.push((ab.annonce_par_user_id, Portee::Demande));
                    if ab.recupere_par_personne_id.is_some() {
                        v.push((ab.annonce_par_user_id, Portee::Personne));
                    }
                }
                for f in faits_o.iter().filter(|f| matches!(f.acte.as_str(), "annonce" | "demande" | "regularisation_demandee") && f.famille()) {
                    if let Some(u) = f.par_user_id {
                        v.push((u, Portee::Demande));
                    }
                }
                if let Some(ds) = usages.get("week_end").and_then(|u| deciders.get(&(u.clone(), ab.eleve_id))) {
                    v.extend(ds.iter().map(|d| (*d, Portee::Annuel)));
                }
                if let Some(p) = ab.depart_remis_a_personne_id {
                    remis.insert(ab.id, p);
                }
            }
            Objet::Correspondant(c) => {
                v.push((c.designe_par, Portee::Designation));
                for f in faits_o.iter().filter(|f| f.acte == "designation" && f.famille()) {
                    if let Some(u) = f.par_user_id {
                        v.push((u, Portee::Designation));
                    }
                }
            }
        }
        v.sort_by_key(|(u, p)| (*u, *p as u8));
        v.dedup();
        auteurs.insert(o.id(), v);
    }
    // 5. Qui est visé : une requête pour tous les couples (élève, compte), une pour toutes les personnes remises.
    let mut couples: Vec<(Uuid, Uuid)> = Vec::new();
    for o in objets {
        for (u, _) in auteurs.get(&o.id()).map(Vec::as_slice).unwrap_or(&[]) {
            couples.push((o.eleve(), *u));
        }
    }
    couples.sort_unstable();
    couples.dedup();
    let vises = comptes_vises_tx(conn, etab, &couples, jour).await?;
    let mut remis_vises: HashMap<Uuid, DateTime<Utc>> = HashMap::new();
    let a_verifier: Vec<(Uuid, Uuid, Uuid)> = objets
        .iter()
        .filter(|o| o.parti())
        .filter_map(|o| remis.get(&o.id()).map(|p| (o.id(), o.eleve(), *p)))
        .collect();
    if !a_verifier.is_empty() {
        let (cibles, (eleves, personnes)): (Vec<Uuid>, (Vec<Uuid>, Vec<Uuid>)) =
            a_verifier.iter().map(|(c, e, p)| (*c, (*e, *p))).unzip();
        let lignes: Vec<(Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT k.c, ent_internat_personne_vise($1, k.e, k.p, $5) \
               FROM unnest($2::uuid[], $3::uuid[], $4::uuid[]) AS k(c, e, p)",
        )
        .bind(etab)
        .bind(&cibles)
        .bind(&eleves)
        .bind(&personnes)
        .bind(jour)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        remis_vises = lignes.into_iter().filter_map(|(c, d)| d.map(|d| (c, d))).collect();
    }
    // 6. Les chutes, hors celles qu'une décision du CPE ou de la direction postérieure à l'interdiction couvre.
    let mut sortie: HashMap<Uuid, Chute> = HashMap::new();
    for o in objets {
        let faits_o: &[&FaitLu] = par_cible.get(&o.id()).map(Vec::as_slice).unwrap_or(&[]);
        let couvert_apres = |depuis: DateTime<Utc>| {
            faits_o.iter().any(|f| f.a >= depuis && f.couvre_la_chute(matches!(o, Objet::Correspondant(_))))
        };
        let mut chute = Chute::default();
        for (u, portee) in auteurs.get(&o.id()).map(Vec::as_slice).unwrap_or(&[]) {
            let Some(depuis) = vises.get(&(o.eleve(), *u)).copied() else { continue };
            if couvert_apres(depuis) {
                continue;
            }
            chute.auteurs.push((*u, *portee));
            chute.depuis = chute.depuis.max(Some(depuis));
        }
        if let Some(depuis) = remis_vises.get(&o.id()).copied() {
            if !couvert_apres(depuis) {
                chute.remise_visee = true;
                chute.depuis = chute.depuis.max(Some(depuis));
            }
        }
        if chute.tombe() {
            sortie.insert(o.id(), chute);
        }
    }
    Ok(sortie)
}

// ───────────────────────── Effets ─────────────────────────

/// Sortie **non partie** dont l'acte tombe : accord et décision du CPE retirés, personne effacée, autorisation annuelle
/// écartée, retour à « demandée » (la décision revient au CPE ; un autre titulaire peut répondre). Tracé et signalé.
pub(super) async fn appliquer_sortie_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    chute: &Chute,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let efface = chute.touche(Portee::Personne) && s.avec_personne_id.is_some();
    let n = sqlx::query(
        "UPDATE ent_internat_sortie SET \
                statut = CASE WHEN statut IN ('autorisee', 'suspendue') THEN 'demandee' ELSE statut END, \
                avant_blocage = CASE WHEN statut = 'bloquee' AND avant_blocage IN ('autorisee', 'suspendue') \
                                     THEN 'demandee' ELSE avant_blocage END, \
                accord_titulaire_le = NULL, accord_titulaire_user_id = NULL, \
                decision_cpe_le = NULL, decision_cpe_user_id = NULL, \
                avec_personne_id = CASE WHEN $3 THEN NULL ELSE avec_personne_id END, \
                par_autorisation_annuelle = CASE WHEN $4 THEN false ELSE par_autorisation_annuelle END, \
                modifie_le = $5 \
          WHERE etablissement_id = $1 AND id = $2 AND statut IN ('demandee', 'autorisee', 'suspendue', 'bloquee')",
    )
    .bind(etab)
    .bind(s.id)
    .bind(efface)
    .bind(chute.touche(Portee::Annuel))
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(erreur_sortie)?
    .rows_affected();
    if n == 0 {
        // Partie entre-temps : le cas pour le CPE est ouvert par la revue des départs.
        return Ok(false);
    }
    fait_tx(
        conn,
        etab,
        Cible::Sortie(s.id),
        "accord_remis_a_zero",
        "systeme",
        None,
        maintenant,
        Some("interdiction_auteur"),
        None,
        if efface { s.avec_personne_id } else { None },
        None,
    )
    .await?;
    evenements_sortie_tx(conn, etab, None, s, types::INTERNAT_SORTIE_DECIDEE, "demandee", "interdiction_auteur", s.statut == "autorisee")
        .await?;
    Ok(true)
}

/// Absence **non partie** dont l'acte tombe : personne désignée effacée, absence bloquée (motif `interdiction_auteur`) ;
/// l'élève reste attendu et la direction tranche. Une absence déjà bloquée le reste (seule la personne est effacée).
pub(super) async fn appliquer_absence_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    ab: &AbsenceLigne,
    chute: &Chute,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    if chute.touche(Portee::Personne) && ab.recupere_par_personne_id.is_some() {
        sqlx::query(
            "UPDATE ent_internat_absence SET recupere_par_personne_id = NULL, modifie_le = $3 \
              WHERE etablissement_id = $1 AND id = $2 AND depart_effectif IS NULL",
        )
        .bind(etab)
        .bind(ab.id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        fait_tx(conn, etab, Cible::Absence(ab.id), "accord_remis_a_zero", "systeme", None, maintenant, Some("interdiction_auteur"), None, ab.recupere_par_personne_id, None)
            .await?;
    }
    if matches!(ab.statut.as_str(), "demandee" | "annoncee" | "acceptee") {
        return bloquer_absence_systeme_tx(conn, etab, ab, "interdiction_auteur", maintenant).await;
    }
    Ok(false)
}

/// Correspondant désigné par un auteur désormais visé : suspendu (seul le CPE le reprend), tracé et signalé.
pub(super) async fn appliquer_correspondant_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    c: &CorrespondantAuteur,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let n = sqlx::query(
        "UPDATE ent_internat_correspondant SET statut = 'suspendu', modifie_le = $3 \
          WHERE etablissement_id = $1 AND id = $2 AND statut = 'valide'",
    )
    .bind(etab)
    .bind(c.id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?
    .rows_affected();
    if n == 0 {
        return Ok(false);
    }
    fait_tx(conn, etab, Cible::Correspondant(c.id), "suspension", "systeme", None, maintenant, Some("interdiction_auteur"), None, None, None)
        .await?;
    publier(
        conn,
        etab,
        types::INTERNAT_CORRESPONDANT,
        AggregateKind::Internat,
        c.id,
        Some(c.eleve_id),
        json!({ "action": "suspendu", "cause": "interdiction_auteur" }),
        None,
        Causalite::default(),
    )
    .await?;
    Ok(true)
}

/// Les correspondants valides désignés par un compte désormais visé sont suspendus (d'un élève, ou de tous). Nombre de
/// correspondants suspendus. Appelé au contrôle de départ et à la revue, avant celle des sorties.
pub(super) async fn suspendre_correspondants_auteur_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Option<Uuid>,
    jour: NaiveDate,
    maintenant: DateTime<Utc>,
    echecs: &mut i32,
) -> Result<i32, EntDbError> {
    let lignes: Vec<(Uuid, Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, eleve_id, designe_par_user_id FROM ent_internat_correspondant \
          WHERE etablissement_id = $1 AND statut = 'valide' AND ($2::uuid IS NULL OR eleve_id = $2) ORDER BY id FOR UPDATE",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let corr: Vec<CorrespondantAuteur> =
        lignes.into_iter().map(|(id, eleve_id, designe_par)| CorrespondantAuteur { id, eleve_id, designe_par }).collect();
    let objets: Vec<Objet<'_>> = corr.iter().map(Objet::Correspondant).collect();
    let chutes = actes_qui_tombent_tx(conn, etab, &objets, jour).await?;
    let mut n = 0;
    for c in corr.iter().filter(|c| chutes.contains_key(&c.id)) {
        sqlx::query("SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
        let r = appliquer_correspondant_tx(conn, etab, c, maintenant).await;
        fin_savepoint_revue(conn, true, r, &mut n, echecs).await?;
    }
    Ok(n)
}

// ───────────────────────── Cas pour le CPE (déjà parti) ─────────────────────────

/// Élèves pour lesquels un cas `interdiction_auteur` a déjà été ouvert (ouvert ou clos) depuis la date donnée : le CPE a
/// été saisi, la revue ne rouvre pas un cas clos chaque minute.
pub(super) async fn eleves_deja_signales_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
) -> Result<HashMap<Uuid, DateTime<Utc>>, EntDbError> {
    if eleves.is_empty() {
        return Ok(HashMap::new());
    }
    let lignes: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT eleve_id, max(ouvert_le) FROM ent_internat_cas \
          WHERE etablissement_id = $1 AND nature = 'interdiction_auteur' AND eleve_id = ANY($2) GROUP BY eleve_id",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(lignes.into_iter().collect())
}

impl PgEnt {
    /// Déjà parti (absence ou sortie « partie ») dont un auteur ou la personne remise est devenu visé : un cas pour le CPE
    /// s'ouvre (alerte) ; les nuits suivantes ne retirent plus l'élève de l'attendu tant qu'il n'a pas tranché. Rien
    /// n'est défait : l'élève est parti. Un cas déjà ouvert ou clos depuis la pose de l'interdiction n'est pas rouvert.
    /// Renvoie le nombre de cas ouverts.
    pub(super) async fn signaler_departs_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Option<Uuid>,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        echecs: &mut i32,
    ) -> Result<i32, EntDbError> {
        let nuit = nuit_a_tx(conn, etab, maintenant).await?;
        let sorties: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie \
              WHERE etablissement_id = $1 AND statut = 'partie' AND retour_effectif IS NULL \
                AND ($2::uuid IS NULL OR eleve_id = $2) ORDER BY debut, id"
        ))
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let absences: Vec<AbsenceLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_ABSENCE} FROM ent_internat_absence \
              WHERE etablissement_id = $1 AND statut IN ('demandee', 'annoncee', 'acceptee') AND depart_effectif IS NOT NULL \
                AND au_nuit >= $3 AND ($2::uuid IS NULL OR eleve_id = $2) ORDER BY du_nuit, id"
        ))
        .bind(etab)
        .bind(eleve)
        .bind(nuit)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let objets: Vec<Objet<'_>> = sorties.iter().map(Objet::Sortie).chain(absences.iter().map(Objet::Absence)).collect();
        let chutes = actes_qui_tombent_tx(conn, etab, &objets, jour).await?;
        if chutes.is_empty() {
            return Ok(0);
        }
        let concernes: Vec<Uuid> = objets.iter().filter(|o| chutes.contains_key(&o.id())).map(Objet::eleve).collect();
        let signales = eleves_deja_signales_tx(conn, etab, &concernes).await?;
        let mut ouverts = 0;
        for o in &objets {
            let Some(chute) = chutes.get(&o.id()) else { continue };
            let eleve = o.eleve();
            if let (Some(depuis), Some(deja)) = (chute.depuis, signales.get(&eleve)) {
                if *deja >= depuis {
                    continue;
                }
            }
            sqlx::query("SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
            let r: Result<bool, EntDbError> = async {
                if self.ouvrir_cas_tx(conn, etab, None, None, eleve, nuit, "interdiction_auteur", maintenant).await?.is_none() {
                    return Ok(false);
                }
                let cible = match o {
                    Objet::Sortie(s) => Cible::Sortie(s.id),
                    Objet::Absence(a) => Cible::Absence(a.id),
                    Objet::Correspondant(c) => Cible::Correspondant(c.id),
                };
                fait_tx(conn, etab, cible, "signalement", "systeme", None, maintenant, Some("interdiction_auteur"), None, None, None).await?;
                Ok(true)
            }
            .await;
            fin_savepoint_revue(conn, true, r, &mut ouverts, echecs).await?;
        }
        Ok(ouverts)
    }
}
