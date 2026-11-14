//! Socle 15, lot A3 — impact d'une absence, tableaux, files
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §10, R-AP23, R-AP25, R-AP45, R-AP46, R-AP48 ; ABP-150 à 157,
//! 159, 165 à 167, 170 à 174, 189, 272).
//!
//! **Niveau « organisation » (§18.1)** : rien ici ne lit la nature, le motif, un traitement ou une pièce ;
//! les DTO n'ont pas ces champs.
//!
//! **Calcul de l'impact.** À chaque changement d'une absence (et à chaque recalcul de l'emploi du temps,
//! tâche planifiée), le serveur recalcule les séances touchées en lisant le **port du socle 10**
//! (`edt_seances(Intervenant)`, séances de l'agent : titulaire, remplaçant) — le 15 n'écrit jamais dans les
//! tables du 10 — puis y applique les règles du 15 :
//!
//! * une plage **partielle** ne touche que les séances qu'elle recoupe (CAP-05) ; une absence **discontinue**
//!   touche l'union de ses plages (CAP-06) ;
//! * **co-enseignement** : la séance n'est touchée que si **aucun** autre intervenant en saisie n'est
//!   présent (R-AP25, CAP-25) ;
//! * une séance **neutralisée** par le 10 (sortie de toute la classe, examen, vacances) est `sans_objet`
//!   (CAP-26) ; pendant les **vacances** il n'y a pas de séance donc aucun impact (CAP-27) ;
//! * un remplacement ou une annulation déjà posés (modification du 10, ou exception historique
//!   `ent_edt_exception` liée à l'absence) donne l'état `remplace` / `annule` et une réponse reprise ;
//! * **nouvelle version d'EDT** (CAP-46) : le recalcul conserve l'état et la réponse des séances qui ont
//!   gardé leur identité ; celles qui ont disparu avec une réponse posée sont **signalées** (`a_revoir`),
//!   jamais perdues en silence ;
//! * R-AP23 : un impact encore à traiter (ou proposé, sollicité) quand l'heure de fin est passée devient
//!   `non_assure`.
//!
//! **Limites (ports absents)** : les postes de service (socle 6 P15), les engagements des socles 19, 23, 38,
//! 42 et les séances des élèves accompagnés par un AESH (socle 10 §8.3 bis) n'ont pas de port aujourd'hui :
//! le type d'impact existe, rien n'est calculé (ABP-152 à 154).

use std::collections::{HashMap, HashSet};

use super::*;
use crate::edt_ports::{CibleSeances, SeanceEdt};
use crate::ref_commun::Acteur;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

/// Une absence annulée ne touche plus l'organisation ; toutes les autres comptent, **refusée comprise** :
/// l'agent n'était pas là, les élèves n'ont pas leur cours (R-AP15). Une absence seulement signalée est
/// calculée en interne mais n'a aucun effet extérieur avant confirmation (R-AP46).
pub fn touche_l_organisation(etat: &str) -> bool {
    etat != "annulee"
}

/// États posés par le lot A4 (propositions, sollicitations, autres réponses) : conservés au recalcul tant que
/// l'emploi du temps n'affirme rien d'autre.
const ETATS_REPONSE_POSEE: &[&str] = &[
    "propose", "sollicite", "autre_cours", "deplace", "continuite", "supplee",
];

/// État d'un impact selon ce que dit l'emploi du temps (faits du socle 10 et exception historique).
pub fn etat_derive_de_la_seance(
    etat_seance: &str,
    remplacant_distinct: bool,
    exception_kind: Option<&str>,
    exception_remplacant_distinct: bool,
) -> Option<&'static str> {
    if etat_seance == "neutralisee" {
        return Some("sans_objet");
    }
    if etat_seance == "annulee" || exception_kind == Some("annulation") {
        return Some("annule");
    }
    if remplacant_distinct || (exception_kind == Some("remplacement") && exception_remplacant_distinct) {
        return Some("remplace");
    }
    None
}

/// État d'un impact après recalcul : le fait du 10 l'emporte ; sinon la réponse posée est conservée ;
/// sinon `a_traiter`. Puis R-AP23 : l'heure de fin passée sans réponse donne `non_assure`.
pub fn etat_apres_calcul(
    derive: Option<&'static str>,
    existant: Option<&str>,
    fin_passee: bool,
) -> &'static str {
    let etat = match derive {
        Some(d) => d,
        None => match existant {
            Some(e) => ETATS_REPONSE_POSEE.iter().find(|x| **x == e).copied().unwrap_or("a_traiter"),
            None => "a_traiter",
        },
    };
    if fin_passee && matches!(etat, "a_traiter" | "propose" | "sollicite") {
        "non_assure"
    } else {
        etat
    }
}

/// `[a_debut, a_fin)` recoupe `[s_debut, s_fin)`.
pub fn recoupe(
    a_debut: DateTime<Utc>,
    a_fin: DateTime<Utc>,
    s_debut: DateTime<Utc>,
    s_fin: DateTime<Utc>,
) -> bool {
    a_debut < s_fin && a_fin > s_debut
}

/// Prochaine étape d'une absence : qui doit agir (ABP-172). `externe` : l'employeur (rectorat, DSDEN) décide.
pub fn prochaine_etape(etat: &str, decision_externe: bool) -> (&'static str, &'static str) {
    match etat {
        "signalee" => ("agent", "confirmer"),
        "declaree" => ("valideur", "valider"),
        "en_instruction" => ("valideur", "valider"),
        "prise_en_compte" => ("employeur", "decision_externe"),
        "decision_employeur" => ("instructeur", "traiter"),
        "validee" if decision_externe => ("employeur", "decision_externe"),
        "validee" => ("instructeur", "traiter"),
        "traitee" => ("valideur", "clore"),
        _ => ("personne", "rien"),
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ImpactLigne {
    id: Uuid,
    absence_id: Uuid,
    type_impact: String,
    seance_id: Option<Uuid>,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    etat: String,
    a_revoir: bool,
    creneau_id: Option<Uuid>,
    jour: Option<NaiveDate>,
    heure_debut: Option<NaiveTime>,
    heure_fin: Option<NaiveTime>,
    classe: Option<String>,
    matiere_libelle: Option<String>,
    salle: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ImpactJourLigne {
    #[sqlx(flatten)]
    base: ImpactLigne,
    agent_label: Option<String>,
    renforcee: bool,
}

const IMPACT_SELECT: &str = "SELECT i.id, i.absence_id, i.type AS type_impact, i.seance_id, i.debut, i.fin, i.etat, i.a_revoir,
        s.creneau_id, s.jour, s.heure_debut, s.heure_fin, s.classe, s.matiere_libelle, s.salle
   FROM ent_absp_impact i
   LEFT JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id";

fn impact_dto(r: ImpactLigne) -> ImpactDto {
    let seance = match (r.seance_id, r.creneau_id, r.jour, r.heure_debut, r.heure_fin, r.classe, r.matiere_libelle) {
        (Some(seance_id), Some(creneau_id), Some(jour), Some(hd), Some(hf), Some(classe), Some(matiere)) => {
            Some(SeanceImpactDto {
                seance_id,
                creneau_id,
                jour,
                heure_debut: hd,
                heure_fin: hf,
                classe,
                matiere_libelle: matiere,
                salle: r.salle,
            })
        }
        _ => None,
    };
    ImpactDto {
        id: r.id,
        absence_id: r.absence_id,
        type_impact: r.type_impact,
        seance,
        debut: r.debut,
        fin: r.fin,
        etat: r.etat,
        a_revoir: r.a_revoir,
    }
}

/// Une séance touchée, avec ce que l'emploi du temps en dit.
struct Touchee {
    seance_id: Uuid,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    derive: Option<&'static str>,
    exception: Option<(Uuid, String, Option<Uuid>, Uuid)>,
}

impl PgEnt {
    // ───────────────────────────── Calcul ─────────────────────────────

    /// Recalcule l'impact d'une absence dans la transaction de l'appelant (idempotent). Renvoie le nombre
    /// d'impacts et le nombre de réponses à revoir.
    pub(crate) async fn recalculer_impact_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        actor: Uuid,
    ) -> Result<(u32, u32), EntDbError> {
        let a = charger(conn, etab, absence_id, false).await?;
        let plages: Vec<(DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT debut, fin FROM ent_absp_plage
              WHERE etablissement_id = $1 AND absence_id = $2 AND retiree_le IS NULL ORDER BY debut",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let (maintenant,): (DateTime<Utc>,) = sqlx::query_as("SELECT now()")
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
        let mut touchees: Vec<Touchee> = Vec::new();
        if let (true, Some(user), false) = (touche_l_organisation(&a.etat), a.agent_user_id, plages.is_empty()) {
            touchees = self.seances_touchees_par(conn, etab, absence_id, a.agent_id, user, &plages).await?;
        }
        // Existant.
        let existants: Vec<(Uuid, Option<Uuid>, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, seance_id, etat, reponse_active_id FROM ent_absp_impact
              WHERE etablissement_id = $1 AND absence_id = $2 AND type = 'seance'",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let par_seance: HashMap<Uuid, (Uuid, String, Option<Uuid>)> = existants
            .iter()
            .filter_map(|(id, s, e, r)| s.map(|s| (s, (*id, e.clone(), *r))))
            .collect();
        let mut vues: HashSet<Uuid> = HashSet::new();
        for t in &touchees {
            vues.insert(t.seance_id);
            let existant = par_seance.get(&t.seance_id);
            let etat = etat_apres_calcul(t.derive, existant.map(|e| e.1.as_str()), t.fin <= maintenant);
            let impact_id = match existant {
                Some((id, ..)) => {
                    sqlx::query(
                        "UPDATE ent_absp_impact SET etat = $3, debut = $4, fin = $5, a_revoir = false, calcule_le = now()
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(etat)
                    .bind(t.debut)
                    .bind(t.fin)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                    *id
                }
                None => {
                    let id = Uuid::new_v4();
                    sqlx::query(
                        "INSERT INTO ent_absp_impact (id, etablissement_id, absence_id, type, seance_id, debut, fin, etat)
                         VALUES ($1,$2,$3,'seance',$4,$5,$6,$7)",
                    )
                    .bind(id)
                    .bind(etab)
                    .bind(absence_id)
                    .bind(t.seance_id)
                    .bind(t.debut)
                    .bind(t.fin)
                    .bind(etat)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                    id
                }
            };
            self.synchroniser_reponse_historique(conn, etab, impact_id, t, actor).await?;
        }
        // Séances qui ne sont plus touchées (nouvelle version d'EDT, plage retirée, absence annulée).
        let mut a_revoir = 0u32;
        for (id, seance, etat, reponse) in &existants {
            let Some(seance) = seance else { continue };
            if vues.contains(seance) {
                continue;
            }
            let (a_reponse,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_absp_reponse WHERE etablissement_id = $1 AND impact_id = $2)",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if reponse.is_none() && !a_reponse && matches!(etat.as_str(), "a_traiter" | "non_assure" | "sans_objet") {
                sqlx::query("DELETE FROM ent_absp_impact WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
            } else if reponse.is_none() {
                sqlx::query(
                    "UPDATE ent_absp_impact SET etat = 'sans_objet', a_revoir = false, calcule_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            } else {
                sqlx::query(
                    "UPDATE ent_absp_impact SET a_revoir = true, calcule_le = now() WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                a_revoir += 1;
            }
        }
        // Lot A5 (CAP-36) : une absence qui a une demande ou une suppléance voit ses nouvelles séances passer
        // `supplee` à la date, et son état de remplacement suivre les faits du socle 6.
        let (a_remplacement,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_service_intervenant
                             WHERE etablissement_id = $1 AND nature = 'suppleant' AND origine = 'absence'
                               AND origine_ref = $2 AND annule_le IS NULL)
                 OR EXISTS (SELECT 1 FROM ent_absp_demande_suppleance WHERE etablissement_id = $1 AND absence_id = $2)",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if a_remplacement {
            self.synchroniser_remplacement_tx(conn, etab, absence_id, Some(actor)).await?;
        }
        Ok((touchees.len() as u32, a_revoir))
    }

    /// Recalcul de l'impact dans la transaction d'une commande **sans jamais la faire échouer** : l'absence est
    /// un fait RH, l'impact se rattrape (lecture, tâche planifiée). Le recalcul est protégé par un point de
    /// reprise : une erreur de base n'abandonne pas la transaction de la commande.
    pub(crate) async fn recalculer_impact_tolerant(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        actor: Uuid,
    ) {
        if sqlx::query("SAVEPOINT impact_absence").execute(&mut *conn).await.is_err() {
            return;
        }
        match self.recalculer_impact_tx(conn, etab, absence_id, actor).await {
            Ok(_) => {
                let _ = sqlx::query("RELEASE SAVEPOINT impact_absence").execute(&mut *conn).await;
            }
            Err(e) => {
                tracing::warn!(absence = %absence_id, "recalcul d'impact différé : {e}");
                let _ = sqlx::query("ROLLBACK TO SAVEPOINT impact_absence").execute(&mut *conn).await;
            }
        }
    }

    /// Les séances que l'absence touche, avec les règles du 15 (plages, co-enseignement, faits du 10).
    async fn seances_touchees_par(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        agent_id: Uuid,
        user: Uuid,
        plages: &[(DateTime<Utc>, DateTime<Utc>)],
    ) -> Result<Vec<Touchee>, EntDbError> {
        // Fenêtre en jours locaux de l'établissement ; le port lit 62 jours au plus par appel.
        let (premier, dernier): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT ent_aujourdhui_etab($1, $2), ent_aujourdhui_etab($1, $3 - interval '1 microsecond')",
        )
        .bind(etab)
        .bind(plages.iter().map(|p| p.0).min().expect("plages non vides"))
        .bind(plages.iter().map(|p| p.1).max().expect("plages non vides"))
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        let dernier = dernier.min(premier + Duration::days(366));
        // L'agent lit ses propres séances (D-022) : le calcul n'emprunte aucun rôle de gestion.
        let soi = Acteur { etablissement_id: etab, user_id: user, role: "prof" };
        let mut seances: Vec<SeanceEdt> = Vec::new();
        let mut d = premier;
        while d <= dernier {
            let f = (d + Duration::days(60)).min(dernier);
            seances.extend(self.edt_seances(&soi, CibleSeances::Intervenant(user), d, f).await?);
            d = f + Duration::days(1);
        }
        // Co-enseignant en saisie : le port ne retourne que titulaires et remplaçants ; les séances où l'agent
        // est un intervenant ajouté (socle 10, `ent_edt_seance_intervenant`) se lisent en plus, en lecture seule.
        let ids_co: Vec<Uuid> = sqlx::query_scalar(
            "SELECT s.id FROM ent_edt_seance_intervenant i
               JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id
              WHERE i.etablissement_id = $1 AND i.agent_id = $2 AND i.nature = 'ajoute' AND i.saisie
                AND s.jour BETWEEN $3 AND $4 AND s.etat <> 'retiree'",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(premier)
        .bind(dernier)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let deja: HashSet<Uuid> = seances.iter().map(|s| s.id).collect();
        let ids_co: Vec<Uuid> = ids_co.into_iter().filter(|i| !deja.contains(i)).collect();
        seances.extend(self.edt_seances_par_ids(etab, &ids_co).await?);
        if seances.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = seances.iter().map(|s| s.id).collect();
        // Intervenants figés du 10 : les co-enseignants (`ajoute`, en saisie) et les retirés.
        let figes: Vec<(Uuid, Uuid, Option<Uuid>, String, bool)> = sqlx::query_as(
            "SELECT i.seance_id, i.agent_id, p.user_id, i.nature, i.saisie
               FROM ent_edt_seance_intervenant i
               JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE i.etablissement_id = $1 AND i.seance_id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        // Exceptions historiques (remplacement, annulation) posées sur ces créneaux et jours.
        let exceptions: Vec<(Uuid, NaiveDate, Uuid, String, Option<Uuid>, Uuid)> = sqlx::query_as(
            "SELECT creneau_id, jour, id, kind, remplacant_user_id, created_by_user_id FROM ent_edt_exception
              WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3",
        )
        .bind(etab)
        .bind(premier)
        .bind(dernier)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut sortie: Vec<Touchee> = Vec::new();
        for s in seances {
            if matches!(s.etat.as_str(), "deplacee" | "retiree") {
                continue;
            }
            let (debut, fin) = match (s.debut_utc, s.fin_utc) {
                (Some(d), Some(f)) => (d, f),
                _ => (
                    instant(conn, etab, s.jour, Some(s.heure_debut)).await?,
                    instant(conn, etab, s.jour, Some(s.heure_fin)).await?,
                ),
            };
            if !plages.iter().any(|p| recoupe(p.0, p.1, debut, fin)) {
                continue;
            }
            let est_principal = s.prof_user_id == user || s.remplacant_user_id == Some(user);
            let mes: Vec<&(Uuid, Uuid, Option<Uuid>, String, bool)> =
                figes.iter().filter(|f| f.0 == s.id).collect();
            let retires: HashSet<Uuid> = mes.iter().filter(|f| f.3 == "retire").map(|f| f.1).collect();
            let ajoutes: Vec<(Uuid, Option<Uuid>)> = mes
                .iter()
                .filter(|f| f.3 == "ajoute" && f.4 && !retires.contains(&f.1))
                .map(|f| (f.1, f.2))
                .collect();
            let est_ajoute = ajoutes.iter().any(|(ag, _)| *ag == agent_id);
            if !est_principal && !est_ajoute {
                continue;
            }
            // R-AP25 : un autre intervenant en saisie présent suffit, la séance n'est pas touchée.
            let mut autres: Vec<Uuid> = ajoutes.iter().filter(|(ag, _)| *ag != agent_id).map(|(ag, _)| *ag).collect();
            if est_ajoute && !est_principal {
                if let Some(p) = principal_agent(conn, etab, s.remplacant_user_id.unwrap_or(s.prof_user_id)).await? {
                    autres.push(p);
                }
            }
            let mut quelqu_un_present = false;
            for autre in autres {
                if !agent_absent_sur(conn, etab, autre, absence_id, debut, fin).await? {
                    quelqu_un_present = true;
                    break;
                }
            }
            if quelqu_un_present {
                continue;
            }
            let exc = exceptions.iter().find(|e| e.0 == s.creneau_id && e.1 == s.jour);
            let remplacant_distinct = s.remplacant_user_id.is_some_and(|r| r != user) && s.prof_user_id == user;
            let derive = etat_derive_de_la_seance(
                &s.etat,
                remplacant_distinct,
                exc.map(|e| e.3.as_str()),
                exc.is_some_and(|e| e.4.is_some_and(|r| r != user)),
            );
            sortie.push(Touchee {
                seance_id: s.id,
                debut,
                fin,
                derive,
                exception: exc.map(|e| (e.2, e.3.clone(), e.4, e.5)),
            });
        }
        Ok(sortie)
    }

    /// Reprise des liens historiques : une exception `remplacement` ou `annulation` sur la séance devient la
    /// réponse active de l'impact ; si elle a disparu, la réponse reprise est levée.
    async fn synchroniser_reponse_historique(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        impact_id: Uuid,
        t: &Touchee,
        actor: Uuid,
    ) -> Result<(), EntDbError> {
        let active: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, modification_edt_ref FROM ent_absp_reponse
              WHERE etablissement_id = $1 AND impact_id = $2 AND etat = 'active'",
        )
        .bind(etab)
        .bind(impact_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let voulue = t.exception.as_ref().filter(|e| matches!(e.1.as_str(), "remplacement" | "annulation"));
        match (voulue, active) {
            (Some(e), Some((_, Some(reference)))) if reference == e.0 => {}
            (Some(e), ancienne) => {
                if let Some((rid, _)) = ancienne {
                    lever_reponse(conn, etab, impact_id, rid, actor).await?;
                }
                let remplacant = match e.2 {
                    Some(u) if e.1 == "remplacement" => agent_du_user_tx(conn, etab, u).await?,
                    _ => None,
                };
                let rid = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_absp_reponse (id, etablissement_id, impact_id, type, remplacant_agent_id,
                            modification_edt_ref, posee_par_user_id)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(rid)
                .bind(etab)
                .bind(impact_id)
                .bind(if e.1 == "annulation" { "annulation" } else { "remplacement" })
                .bind(remplacant)
                .bind(e.0)
                .bind(e.3)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                sqlx::query("UPDATE ent_absp_impact SET reponse_active_id = $3 WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(impact_id)
                    .bind(rid)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
            }
            (None, Some((rid, Some(_)))) => {
                // Réponse reprise d'une exception qui n'existe plus : levée (la séance redevient à traiter).
                lever_reponse(conn, etab, impact_id, rid, actor).await?;
            }
            (None, _) => {}
        }
        Ok(())
    }

    /// Recalcul à la demande d'une absence (ABP-151) : transaction propre.
    pub async fn recalculer_impact_absence(
        &self,
        etab: Uuid,
        actor: Uuid,
        absence_id: Uuid,
    ) -> Result<(u32, u32), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = self.recalculer_impact_tx(&mut tx, etab, absence_id, actor).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Tâche planifiée (ABP-151, ABP-160 voisin) : recalcule l'impact des absences à venir ou en cours de
    /// chaque établissement. C'est ce qui rattrape une nouvelle version d'EDT (CAP-46). Une absence en
    /// échec n'arrête pas les autres.
    pub async fn recalculer_impacts_a_venir(&self) -> Result<BilanImpactDto, EntDbError> {
        let mut bilan = BilanImpactDto::default();
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("absences_personnels_impact"))
            .await
            .map_err(db)?
        {
            let ids: Vec<(Uuid, Uuid)> = sqlx::query_as(
                "SELECT DISTINCT a.id, a.saisie_par_user_id FROM ent_absp_absence a
                   JOIN ent_absp_plage pl ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id
                                         AND pl.retiree_le IS NULL
                  WHERE a.etablissement_id = $1 AND a.cause = 'rh' AND a.etat <> 'annulee'
                    AND pl.fin > now() - interval '1 day'
                  LIMIT 2000",
            )
            .bind(etab)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
            .await
            .map_err(db)?;
            for (id, par) in ids {
                match self.recalculer_impact_absence(etab, par, id).await {
                    Ok((n, r)) => {
                        bilan.absences += 1;
                        bilan.impacts += n;
                        bilan.a_revoir += r;
                    }
                    Err(e) => tracing::warn!(absence = %id, "recalcul d'impact : {e}"),
                }
            }
        }
        Ok(bilan)
    }

    // ───────────────────────────── Lectures ─────────────────────────────

    pub(super) async fn voit_renforcees(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<bool, EntDbError> {
        Ok(matches!(Role::parse(role), Some(Role::Direction))
            || self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider).await?)
    }

    pub(super) async fn peut_lire_organisation(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<bool, EntDbError> {
        Ok(self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireOrganisation).await?
            || self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature).await?)
    }

    /// ABP-150 : l'impact d'une absence (séances), sans nature. L'agent voit le sien ; l'organisation (adjoint,
    /// direction, vie scolaire) ; une absence en confidentialité renforcée n'est vue que de la direction, des
    /// valideurs et de l'agent. Tout autre profil : « introuvable ». Le recalcul est fait avant la lecture.
    pub async fn impact_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<ImpactDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let organisation = self.peut_lire_organisation(etab, actor, role).await?;
        let voit_renforcees = self.voit_renforcees(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, absence_id, false).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let (renforcee,): (bool,) = sqlx::query_as(
            "SELECT confidentialite_renforcee FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !(suis_absent || (organisation && (!renforcee || voit_renforcees))) {
            return Err(introuvable());
        }
        self.recalculer_impact_tx(&mut tx, etab, absence_id, actor).await?;
        let rows: Vec<ImpactLigne> = sqlx::query_as(&format!(
            "{IMPACT_SELECT} WHERE i.etablissement_id = $1 AND i.absence_id = $2 ORDER BY i.debut, i.id"
        ))
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows.into_iter().map(impact_dto).collect())
    }

    /// ABP-165, 166 : tableau « ce matin » / « demain ». Les impacts du jour, par heure et par état, et les
    /// absents (sans nature). Une absence en confidentialité renforcée y figure comme « séance à couvrir » sans
    /// nom pour un profil qui n'y a pas droit. Fenêtre : de la veille à 14 jours.
    pub async fn tableau_jour_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<TableauJourDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self.peut_lire_organisation(etab, actor, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let voit = self.voit_renforcees(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let aujourdhui = jour_du_jour(&mut tx, etab).await?;
        if jour < aujourdhui - Duration::days(1) || jour > aujourdhui + Duration::days(14) {
            return Err(invalide("Jour hors fenêtre (de la veille à 14 jours)."));
        }
        // « Mis à jour en continu » : les absences qui couvrent ce jour sont recalculées avant la lecture.
        let ids: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT a.id, a.saisie_par_user_id FROM ent_absp_absence a
               JOIN ent_absp_plage pl ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id
                                     AND pl.retiree_le IS NULL
              WHERE a.etablissement_id = $1 AND a.cause = 'rh' AND a.etat <> 'annulee'
                AND ent_aujourdhui_etab($1, pl.debut) <= $2 AND ent_aujourdhui_etab($1, pl.fin - interval '1 microsecond') >= $2
              LIMIT 300",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, par) in ids {
            self.recalculer_impact_tx(&mut tx, etab, id, par).await?;
        }
        let rows: Vec<ImpactJourLigne> = sqlx::query_as(
            "SELECT i.id, i.absence_id, i.type AS type_impact, i.seance_id, i.debut, i.fin, i.etat, i.a_revoir,
                    s.creneau_id, s.jour, s.heure_debut, s.heure_fin, s.classe, s.matiere_libelle, s.salle,
                    btrim(coalesce(p.prenom, '') || ' ' || p.nom) AS agent_label,
                    a.confidentialite_renforcee AS renforcee
               FROM ent_absp_impact i
               JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id
               JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
               JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE i.etablissement_id = $1 AND s.jour = $2 AND a.etat <> 'annulee'
              ORDER BY s.heure_debut, s.classe, i.id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut compteurs = CompteursImpactDto::default();
        let mut lignes: Vec<LigneTableauDto> = Vec::new();
        for j in rows {
            let (label, renforcee, r) = (j.agent_label, j.renforcee, j.base);
            compteurs.ajouter(&r.etat);
            let heure = r.heure_debut.expect("séance jointe");
            let agent_label = if renforcee && !voit { None } else { label };
            let item = ImpactTableauDto { impact: impact_dto(r), agent_label };
            match lignes.last_mut() {
                Some(l) if l.heure_debut == heure => l.impacts.push(item),
                _ => lignes.push(LigneTableauDto { heure_debut: heure, impacts: vec![item] }),
            }
        }
        let absents = self.absences_organisation_for_actor(etab, actor, role, jour, jour).await?;
        Ok(TableauJourDto { jour, absents, lignes, compteurs })
    }

    /// ABP-174 : compteurs du tableau de bord (à valider, à traiter, séances à couvrir aujourd'hui), selon les
    /// droits de l'acteur (un compteur sans le droit correspondant vaut 0).
    pub async fn compteurs_tableau_de_bord_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<CompteursTableauDeBordDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let valider = self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider).await?;
        let instruire = self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesInstruire).await?;
        let organisation = self.peut_lire_organisation(etab, actor, role).await?;
        if !(valider || instruire || organisation) {
            return Err(EntDbError::Forbidden);
        }
        let voit = self.voit_renforcees(etab, actor, role).await?;
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let employeur: Option<&str> = match r {
            Role::Administratif => Some("etat"),
            Role::Compta => Some("ogec"),
            _ => None,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a_valider: i64 = if valider {
            sqlx::query_scalar(
                "SELECT count(*) FROM ent_absp_absence WHERE etablissement_id = $1 AND cause = 'rh'
                    AND etat IN ('declaree', 'en_instruction')",
            )
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        } else {
            0
        };
        let a_traiter: i64 = if instruire {
            sqlx::query_scalar(
                "SELECT count(*) FROM ent_absp_absence WHERE etablissement_id = $1 AND cause = 'rh'
                    AND etat IN ('validee', 'prise_en_compte', 'decision_employeur') AND ($2::text IS NULL OR employeur = $2)",
            )
            .bind(etab)
            .bind(employeur)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        } else {
            0
        };
        let a_couvrir: i64 = if organisation {
            let jour = jour_du_jour(&mut tx, etab).await?;
            sqlx::query_scalar(
                "SELECT count(*) FROM ent_absp_impact i
                   JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id
                   JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
                  WHERE i.etablissement_id = $1 AND s.jour = $2 AND i.etat IN ('a_traiter', 'non_assure')
                    AND a.etat <> 'annulee' AND (NOT a.confidentialite_renforcee OR $3)",
            )
            .bind(etab)
            .bind(jour)
            .bind(voit)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        } else {
            0
        };
        Ok(CompteursTableauDeBordDto {
            a_valider: a_valider as u32,
            a_traiter: a_traiter as u32,
            seances_a_couvrir_aujourdhui: a_couvrir as u32,
        })
    }

    /// ABP-171 : file « à traiter » de l'instructeur (secrétariat : État ; comptabilité : OGEC ; direction :
    /// tout), par date de début. Cercle RH : la fiche porte la nature, comme en A1.
    pub async fn file_a_traiter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<AbsenceDto>, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesInstruire).await? {
            return Err(EntDbError::Forbidden);
        }
        let employeur: Option<&str> = match r {
            Role::Administratif => Some("etat"),
            Role::Compta => Some("ogec"),
            _ => None,
        };
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_absence WHERE etablissement_id = $1 AND cause = 'rh'
                AND etat IN ('validee', 'prise_en_compte', 'decision_employeur') AND ($2::text IS NULL OR employeur = $2)
              ORDER BY debut, id LIMIT 200",
        )
        .bind(etab)
        .bind(employeur)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push(self.absence_for_actor(etab, actor, role, id).await?);
        }
        Ok(out)
    }

    /// ABP-172 : qui doit agir sur cette absence, et quoi. Cercle RH (comme la fiche).
    pub async fn prochaine_etape_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<ProchaineEtapeDto, EntDbError> {
        let fiche = self.absence_for_actor(etab, actor, role, absence_id).await?;
        let (acteur, action) = prochaine_etape(&fiche.etat, fiche.decision == "externe");
        Ok(ProchaineEtapeDto { acteur: acteur.to_string(), action: action.to_string() })
    }

    /// ABP-159 (partiel) : au retour, l'agent voit ce que sont devenues ses séances pendant l'absence (état de
    /// chaque impact). Le travail donné et les incidents de la classe relèvent des socles 12, 13 et 20.
    pub async fn retour_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<ImpactDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, absence_id, false).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        if !(mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor)) {
            return Err(introuvable());
        }
        let rows: Vec<ImpactLigne> = sqlx::query_as(&format!(
            "{IMPACT_SELECT} WHERE i.etablissement_id = $1 AND i.absence_id = $2 ORDER BY i.debut, i.id"
        ))
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(impact_dto).collect())
    }

    // ───────────────────────────── ABP-189 : sans objet ─────────────────────────────

    /// Marque un impact « sans objet » (la séance n'avait pas lieu pour une autre raison). Droit d'organiser
    /// l'emploi du temps (`edt.gerer`) ET lecture d'organisation : l'administrateur technique n'a pas accès.
    /// Réservé aux impacts encore à traiter ; journalisé.
    pub async fn marquer_impact_sans_objet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
    ) -> Result<ImpactDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !(self.a_droit_abs(etab, actor, role, Action::EdtGerer).await?
            && self.peut_lire_organisation(etab, actor, role).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let voit = self.voit_renforcees(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (absence_id, etat, renforcee): (Uuid, String, bool) = sqlx::query_as(
            "SELECT i.absence_id, i.etat, a.confidentialite_renforcee FROM ent_absp_impact i
               JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
              WHERE i.etablissement_id = $1 AND i.id = $2 FOR UPDATE OF i",
        )
        .bind(etab)
        .bind(impact_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if renforcee && !voit {
            return Err(introuvable());
        }
        if !matches!(etat.as_str(), "a_traiter" | "non_assure") {
            return Err(conflit("seul un impact à traiter peut être marqué sans objet"));
        }
        sqlx::query("UPDATE ent_absp_impact SET etat = 'sans_objet', calcule_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(impact_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journal(&mut tx, etab, absence_id, "impact_sans_objet", Some("impact"), Some(&etat), Some("sans_objet"), actor).await?;
        let r: ImpactLigne = sqlx::query_as(&format!("{IMPACT_SELECT} WHERE i.etablissement_id = $1 AND i.id = $2"))
            .bind(etab)
            .bind(impact_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(impact_dto(r))
    }

    // ───────────────────────────── ABP-155 : présent ? ─────────────────────────────

    /// Savoir si un agent est présent : `oui`, `non` ou `inconnu`. Jamais la nature ni le lieu. Réservé à
    /// l'accueil et au personnel qui organise (R-AP48) ; une consultation hors accueil est journalisée. Une
    /// absence seulement signalée (non confirmée) ne donne pas « non » : `inconnu` (R-AP46). Limite : les
    /// professeurs qui partagent une séance avec l'agent (R-AP48) et la limitation de débit ne sont pas faits.
    pub async fn present_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
    ) -> Result<PresenceDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        let accueil = r == Role::Accueil;
        if !(accueil
            || self.peut_lire_organisation(etab, actor, role).await?
            || self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesSignaler).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        // Absences à effet : confirmées (états comptants hors « signalée »), en cours maintenant.
        let (non, incertain): (bool, bool) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_absp_absence a JOIN ent_absp_plage pl
                              ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id AND pl.retiree_le IS NULL
                             WHERE a.etablissement_id = $1 AND a.agent_id = $2 AND a.etat = ANY($3)
                               AND (a.etat <> 'signalee' OR a.confirmee_le IS NOT NULL)
                               AND pl.debut <= now() AND pl.fin > now()),
                    EXISTS (SELECT 1 FROM ent_absp_absence a JOIN ent_absp_plage pl
                              ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id AND pl.retiree_le IS NULL
                             WHERE a.etablissement_id = $1 AND a.agent_id = $2 AND a.etat = 'signalee'
                               AND a.confirmee_le IS NULL AND pl.debut <= now() AND pl.fin > now())",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(ETATS_COMPTANT.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let statut = if non {
            "non"
        } else if incertain {
            "inconnu"
        } else {
            "oui"
        };
        if !accueil {
            self.tracer_acces_sensible(
                etab,
                actor,
                role,
                None,
                "consultation",
                "absence_personnel_present",
                "Consultation de la présence d'un agent",
            )
            .await;
        }
        Ok(PresenceDto { agent_id, statut: statut.to_string() })
    }

    // ───────────────────────────── ABP-272 : interrupteur « EDT de référence » ─────────────────────────────

    pub async fn edt_reference_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<EdtReferenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !(self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesParametrer).await?
            || self.peut_lire_organisation(etab, actor, role).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let row: Option<(bool, Option<DateTime<Utc>>, Option<Uuid>)> = sqlx::query_as(
            "SELECT edt_reference, edt_reference_pose_le, edt_reference_pose_par_user_id FROM ent_absp_param
              WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        let (actif, pose_le, pose_par_user_id) = row.unwrap_or((false, None, None));
        Ok(EdtReferenceDto { actif, pose_le, pose_par_user_id })
    }

    /// Pose ou retire l'interrupteur « EDT de référence » : conditionne tout effet vers les familles et les
    /// élèves (A4). La direction (droit `parametrer`) ; à la pose, l'emploi du temps doit être « à jour » —
    /// contrôle approché : au moins une séance prévue dans les 14 prochains jours (le contrôle propre du socle
    /// 10 n'existe pas encore). Date et auteur conservés ; journalisé au niveau de l'établissement.
    pub async fn definir_edt_reference_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        actif: bool,
    ) -> Result<EdtReferenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesParametrer).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if actif {
            let jour = jour_du_jour(&mut tx, etab).await?;
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3
                    AND etat IN ('prevue', 'modifiee', 'ajoutee')",
            )
            .bind(etab)
            .bind(jour)
            .bind(jour + Duration::days(14))
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n == 0 {
                return Err(conflit("emploi du temps non à jour : aucune séance prévue dans les 14 prochains jours"));
            }
        }
        sqlx::query(
            "INSERT INTO ent_absp_param (etablissement_id, edt_reference, edt_reference_pose_le, edt_reference_pose_par_user_id)
             VALUES ($1, $2, now(), $3)
             ON CONFLICT (etablissement_id) DO UPDATE SET edt_reference = $2, edt_reference_pose_le = now(),
                    edt_reference_pose_par_user_id = $3, modifie_le = now(), modifie_par_user_id = $3",
        )
        .bind(etab)
        .bind(actif)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.edt_reference_for_actor(etab, actor, role).await
    }
}

async fn jour_du_jour(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    Ok(j)
}

/// Agent (socle 6) d'un compte, s'il en a un.
async fn principal_agent(conn: &mut PgConnection, etab: Uuid, user: Uuid) -> Result<Option<Uuid>, EntDbError> {
    agent_du_user_tx(conn, etab, user).await
}

/// L'agent est-il lui-même absent (absence à effet, autre que celle qu'on calcule) sur `[debut, fin)` ?
async fn agent_absent_sur(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    sauf_absence: Uuid,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let (b,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_absp_absence a JOIN ent_absp_plage pl
                          ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id AND pl.retiree_le IS NULL
                         WHERE a.etablissement_id = $1 AND a.agent_id = $2 AND a.id <> $3 AND a.etat = ANY($6)
                           AND pl.debut < $5 AND pl.fin > $4)",
    )
    .bind(etab)
    .bind(agent_id)
    .bind(sauf_absence)
    .bind(debut)
    .bind(fin)
    .bind(ETATS_COMPTANT.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(b)
}

async fn lever_reponse(
    conn: &mut PgConnection,
    etab: Uuid,
    impact_id: Uuid,
    reponse_id: Uuid,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query("UPDATE ent_absp_impact SET reponse_active_id = NULL WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(impact_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    sqlx::query(
        "UPDATE ent_absp_reponse SET etat = 'levee', levee_le = now(), levee_par_user_id = $3
          WHERE etablissement_id = $1 AND id = $2 AND etat = 'active'",
    )
    .bind(etab)
    .bind(reponse_id)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[cfg(test)]
mod tests_purs {
    use super::*;

    fn h(heure: u32) -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 11, 16, heure, 0, 0).unwrap()
    }

    #[test]
    fn plage_partielle_recoupe_seulement_les_seances_concernees() {
        // CAP-05 : absence l'après-midi (13 h 30 – 17 h) ; cours à 9 h, 14 h, 17 h.
        let (a, b) = (h(13) + Duration::minutes(30), h(17));
        assert!(!recoupe(a, b, h(9), h(10)));
        assert!(recoupe(a, b, h(14), h(15)));
        assert!(!recoupe(a, b, h(17), h(18)), "la fin est exclusive");
        assert!(recoupe(a, b, h(13), h(14)), "une séance à cheval est touchée");
    }

    #[test]
    fn etat_derive_des_faits_du_socle_10() {
        // CAP-26 : séance neutralisée (sortie de toute la classe) → sans objet.
        assert_eq!(etat_derive_de_la_seance("neutralisee", false, None, false), Some("sans_objet"));
        assert_eq!(etat_derive_de_la_seance("annulee", false, None, false), Some("annule"));
        assert_eq!(etat_derive_de_la_seance("prevue", false, Some("annulation"), false), Some("annule"));
        assert_eq!(etat_derive_de_la_seance("modifiee", true, None, false), Some("remplace"));
        assert_eq!(etat_derive_de_la_seance("prevue", false, Some("remplacement"), true), Some("remplace"));
        // Un « remplaçant » qui est l'absent lui-même ne couvre rien.
        assert_eq!(etat_derive_de_la_seance("prevue", false, Some("remplacement"), false), None);
        assert_eq!(etat_derive_de_la_seance("prevue", false, None, false), None);
    }

    #[test]
    fn etat_apres_calcul_conserve_les_reponses_et_applique_r_ap23() {
        // Rien d'affirmé par l'EDT : la réponse posée est conservée ; un état dérivé du 10 l'emporte.
        assert_eq!(etat_apres_calcul(None, Some("sollicite"), false), "sollicite");
        assert_eq!(etat_apres_calcul(None, Some("remplace"), false), "a_traiter", "le 10 ne dit plus remplacé");
        assert_eq!(etat_apres_calcul(Some("remplace"), Some("a_traiter"), false), "remplace");
        assert_eq!(etat_apres_calcul(None, None, false), "a_traiter");
        // R-AP23 : l'heure de fin passée sans réponse → non assuré ; une séance remplacée le reste.
        assert_eq!(etat_apres_calcul(None, Some("propose"), true), "non_assure");
        assert_eq!(etat_apres_calcul(None, None, true), "non_assure");
        assert_eq!(etat_apres_calcul(Some("remplace"), None, true), "remplace");
        assert_eq!(etat_apres_calcul(Some("sans_objet"), None, true), "sans_objet");
    }

    #[test]
    fn prochaine_etape_par_etat() {
        assert_eq!(prochaine_etape("signalee", false), ("agent", "confirmer"));
        assert_eq!(prochaine_etape("declaree", false), ("valideur", "valider"));
        assert_eq!(prochaine_etape("validee", false), ("instructeur", "traiter"));
        assert_eq!(prochaine_etape("validee", true), ("employeur", "decision_externe"));
        assert_eq!(prochaine_etape("prise_en_compte", true), ("employeur", "decision_externe"));
        assert_eq!(prochaine_etape("traitee", false), ("valideur", "clore"));
        assert_eq!(prochaine_etape("close", false), ("personne", "rien"));
        assert_eq!(prochaine_etape("annulee", false), ("personne", "rien"));
    }

    #[test]
    fn seule_une_absence_annulee_ne_touche_plus() {
        assert!(touche_l_organisation("signalee"));
        assert!(touche_l_organisation("validee"));
        assert!(touche_l_organisation("refusee"), "l'agent n'était pas là (R-AP15)");
        assert!(!touche_l_organisation("annulee"));
    }
}
