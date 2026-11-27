//! Lot A1 du socle 4 (`docs/socles/04_ADMINISTRATIF.md` §12.2, §13, R-A18) : droits par groupe de
//! champs du dossier administratif, appliqués **en un seul point** côté serveur.
//!
//! * [`PgEnt::vue_groupes_eleve`] : quels groupes (F0 à F4, R) l'acteur lit sur **cet élève**,
//!   et pourquoi les autres sont fermés (fonction pure `ent_auth::vue_personnel` / `vue_titulaire`).
//! * [`PgEnt::lecture_dossier_admin`] : responsables, contacts et interdictions filtrés d'après
//!   cette vue — aucune autre règle de minimisation ailleurs.
//! * **Conflit d'intérêts (R-A18)** : un agent qui est lui-même responsable, contact ou payeur de
//!   l'élève, ou qui a déclaré (ou dont la direction a déclaré) un lien (ex-conjoint, ancien membre
//!   du foyer, proche), ne lit pas F2 à F4 des autres personnes avec ses droits professionnels ;
//!   F0, F1 et R restent lisibles, tracés comme accès sensible ; toute écriture sur ce dossier
//!   passe par un collègue. Sa casquette de parent (session famille) est inchangée.
//! * **Droit d'accès d'un titulaire** : ses données, ses enfants, le nom et la nature du lien des
//!   autres responsables ; rien d'autre sur eux (ni coordonnées, adresse, situation juridique,
//!   payeurs), jamais F4, jamais une interdiction.
//! * **Interdictions** (`ent_eleve_interdiction`) : consigne lisible avec `remise.lire` (accueil,
//!   vie scolaire…), fondement avec F3, motif scellé avec F4 ; invisible pour la personne visée.

use super::*;
use crate::champs_chiffres::ChampsSensibles;
use ent_auth::{vue_personnel, vue_titulaire, DroitsPersonnel, Masquage, VueGroupes};
use crate::tenant_tx::TenantTx;

const TABLE_INTERDICTION: &str = "ent_eleve_interdiction";

/// Objets possibles d'une interdiction.
pub const OBJETS_INTERDICTION: [&str; 4] = ["remise", "communication", "visite", "contact_eleve"];
/// Fondements possibles d'une interdiction.
pub const FONDEMENTS_INTERDICTION: [&str; 5] = [
    "decision_justice",
    "ordonnance_protection",
    "decision_ase",
    "demande_titulaire",
    "decision_etablissement",
];
/// Natures d'un conflit d'intérêts déclaré.
pub const NATURES_CONFLIT: [&str; 4] = ["ex_conjoint", "ancien_foyer", "proche", "autre"];

/// Origine d'un conflit d'intérêts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatureConflit {
    /// L'agent est lui-même responsable de l'élève (lien, même ancien ou suspendu) ou payeur.
    Responsable,
    /// L'agent est contact d'urgence de l'élève.
    Contact,
    /// Conflit déclaré (par l'agent ou par la direction).
    Declare,
}

/// Interdiction active, telle que lue par un lecteur (selon ses groupes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterdictionVue {
    pub id: Uuid,
    pub personne_id: Option<Uuid>,
    /// Nom de la personne visée (fiche) ou texte de la décision si non enregistrée.
    pub personne_libelle: String,
    pub objet: Vec<String>,
    /// Bloquante : tout fondement sauf `demande_titulaire` (non bloquante par défaut, §8.8).
    pub bloquante: bool,
    /// Ce que doit faire l'accueil.
    pub consigne: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// F3 : fondement (décision de justice, ordonnance…).
    pub fondement: Option<String>,
    /// F4 : motif scellé, ouvert.
    pub motif: Option<String>,
}

/// Saisie d'une interdiction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterdictionSaisie {
    pub personne_id: Option<Uuid>,
    pub personne_libelle: Option<String>,
    pub objet: Vec<String>,
    pub fondement: String,
    pub motif: Option<String>,
    pub consigne: String,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
}

/// Lecture du dossier administratif d'un élève par un acteur : une seule vue, déjà minimisée.
#[derive(Debug, Clone)]
pub struct LectureDossierAdmin {
    pub vue: VueGroupes,
    pub masquages: Vec<Masquage>,
    pub responsables: Vec<Responsable>,
    pub contacts: Vec<ContactUrgence>,
    pub interdictions: Vec<InterdictionVue>,
}

/// Ligne du tableau de bord « conflits d'intérêts » de la direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflitInteretVu {
    pub declaration_id: Option<Uuid>,
    pub user_id: Uuid,
    pub eleve_id: Uuid,
    /// `agent`, `direction` ou `detecte` (lien ou contact vu à la lecture).
    pub origine: String,
    pub nature: Option<String>,
    pub acces_30j: i64,
    pub dernier_acces: Option<DateTime<Utc>>,
}

fn coffre() -> ChampsSensibles {
    ChampsSensibles::depuis_env()
}

/// Applique une vue de groupes à un responsable (**unique** endroit où un champ est masqué).
fn appliquer_vue(mut r: Responsable, v: &VueGroupes) -> Responsable {
    if !v.f1 {
        r.personne.email = None;
        r.personne.telephone = None;
        r.personne.telephone_2 = None;
    }
    if !v.f2 {
        r.personne.adresse = None;
        r.foyer_id = None;
        r.foyer_libelle = None;
    }
    if !v.f3 {
        r.autorite_parentale = false;
        r.garde = None;
        r.date_debut = None;
        r.date_fin = None;
        r.suspendu_le = None;
        r.peut_voir_notes = false;
        r.peut_voir_vs = false;
        r.peut_justifier_absence = false;
        r.peut_autoriser_sorties = false;
    }
    if !v.f4 {
        r.suspension_motif = None;
        r.suspension_reference = None;
    }
    if !v.payeurs {
        r.est_payeur = false;
        r.part_paiement = None;
    }
    r.masque = v.masquages.clone();
    r
}

/// Même application de la vue que pour les lectures, pour les réponses des écritures : une
/// écriture ne renvoie jamais plus que ce que l'acteur lirait.
pub(crate) fn minimiser_responsable(r: Responsable, v: &VueGroupes) -> Responsable {
    appliquer_vue(r, v)
}

/// RP2-01 : filtre le tableau `liens` de la fiche historique (`GET /eleves/{id}/dossier`)
/// d'après les responsables **déjà minimisés** par [`PgEnt::lecture_dossier_admin`] (même vue,
/// conflit d'intérêts par personne compris ; aucune seconde règle). Un lien absent de cette lecture
/// (F0 fermé, lien inactif pour une famille) est retiré ; sinon un champ dont le groupe est fermé
/// est neutralisé : coordonnées et compte (F1), foyer (F2), droits du lien (F3), payeurs.
pub(crate) fn minimiser_liens_dossier(
    liens: Vec<domain_eleve::LienDossier>,
    lus: &[Responsable],
) -> Vec<domain_eleve::LienDossier> {
    liens
        .into_iter()
        .filter_map(|mut l| {
            let r = lus.iter().find(|r| r.lien_id == l.lien_id)?;
            let ferme = |g: ent_auth::Groupe| r.masque.iter().any(|m| m.groupe == g);
            if ferme(ent_auth::Groupe::F1) {
                l.email = None;
                l.telephone = None;
                l.user_id = None;
            }
            if ferme(ent_auth::Groupe::F2) {
                l.foyer_id = None;
                l.foyer_libelle = None;
            }
            if ferme(ent_auth::Groupe::F3) {
                l.peut_voir_notes = false;
                l.peut_voir_vs = false;
                l.peut_justifier_absence = false;
                l.peut_autoriser_sorties = false;
            }
            l.est_payeur = l.est_payeur && r.est_payeur;
            if r.part_paiement.is_none() {
                l.part_paiement = None;
            }
            Some(l)
        })
        .collect()
}

impl PgEnt {
    /// Minimise un responsable renvoyé par une écriture d'après les groupes de l'acteur.
    pub(crate) async fn minimiser_pour_acteur(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        r: Responsable,
    ) -> Result<Responsable, EntDbError> {
        let (vue, _) = self.vue_groupes_eleve(etab, actor, role, r.eleve_id).await?;
        Ok(appliquer_vue(r, &vue))
    }

    /// Droits du personnel sur cet élève (portées résolues par le moteur de droits).
    async fn droits_personnel_eleve(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<DroitsPersonnel, EntDbError> {
        let mut d = DroitsPersonnel::default();
        d.responsables_lire = self
            .autoriser_eleve(etab, actor, role, Action::ResponsablesLire, eleve_id)
            .await?;
        d.adresses_lire = self
            .autoriser_eleve(etab, actor, role, Action::ResponsablesAdressesLire, eleve_id)
            .await?;
        d.juridique_lire = self
            .autoriser_eleve(etab, actor, role, Action::ResponsablesJuridiqueLire, eleve_id)
            .await?;
        d.protege_lire = self
            .autoriser_eleve(etab, actor, role, Action::ResponsablesProtegeLire, eleve_id)
            .await?;
        d.remise_lire = self
            .autoriser_eleve(etab, actor, role, Action::RemiseLire, eleve_id)
            .await?;
        d.payeurs_definir = self
            .a_action_etablissement(etab, actor, role, Action::PayeursDefinir)
            .await?;
        Ok(d)
    }

    /// Conflit d'intérêts de cet agent vis-à-vis de cet élève (R-A18). `None` pour une famille ou
    /// un élève (leurs droits viennent du lien, jamais d'un rôle professionnel).
    pub async fn conflit_interet_eleve(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<NatureConflit>, EntDbError> {
        if !Role::parse(role).map(Role::is_staff).unwrap_or(false) {
            return Ok(None);
        }
        let (n,): (i32,) = sqlx::query_as(
            "SELECT CASE
                 WHEN EXISTS (SELECT 1 FROM ent_personne p
                                JOIN ent_eleve_lien l
                                  ON l.personne_id = p.id AND l.etablissement_id = p.etablissement_id
                               WHERE p.etablissement_id = $1 AND p.user_id = $2 AND l.eleve_id = $3) THEN 1
                 WHEN EXISTS (SELECT 1 FROM ent_personne p
                                JOIN ent_eleve_contact_urgence c
                                  ON c.personne_id = p.id AND c.etablissement_id = p.etablissement_id
                               WHERE p.etablissement_id = $1 AND p.user_id = $2 AND c.eleve_id = $3) THEN 2
                 WHEN EXISTS (SELECT 1 FROM ent_conflit_interet
                               WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3
                                 AND leve_le IS NULL) THEN 3
                 ELSE 0 END",
        )
        .bind(etab)
        .bind(actor)
        .bind(eleve_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(match n {
            1 => Some(NatureConflit::Responsable),
            2 => Some(NatureConflit::Contact),
            3 => Some(NatureConflit::Declare),
            _ => None,
        })
    }

    /// Groupes ouverts pour l'acteur sur cet élève, et le lien du titulaire s'il en est un.
    pub async fn vue_groupes_eleve(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(VueGroupes, Option<Uuid>), EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok((vue_personnel(DroitsPersonnel::default(), false), None));
        };
        if parsed == Role::Parent {
            return Ok(match self.lien_parent_eleve(etab, actor, eleve_id).await? {
                Some(l) => (vue_titulaire(false), Some(l.id)),
                None => (vue_personnel(DroitsPersonnel::default(), false), None),
            });
        }
        if !parsed.is_staff() {
            return Ok((vue_personnel(DroitsPersonnel::default(), false), None));
        }
        let droits = self.droits_personnel_eleve(etab, actor, role, eleve_id).await?;
        let conflit = self.conflit_interet_eleve(etab, actor, role, eleve_id).await?;
        Ok((vue_personnel(droits, conflit.is_some()), None))
    }

    /// Lecture minimisée du dossier administratif de l'élève (responsables, contacts,
    /// interdictions). Une famille ne voit que ses propres données ; jamais une interdiction.
    pub async fn lecture_dossier_admin(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<LectureDossierAdmin, EntDbError> {
        let (vue, mon_lien) = self.vue_groupes_eleve(etab, actor, role, eleve_id).await?;
        // Les codes de droits à demander n'ont de sens que pour le personnel et pour un titulaire :
        // un élève (ou un compte sans lien) ne reçoit aucune explication de masquage.
        let sait_expliquer = mon_lien.is_some()
            || Role::parse(role).map(Role::is_staff).unwrap_or(false);
        let masquages = if sait_expliquer { vue.masquages.clone() } else { Vec::new() };
        let mut sortie = LectureDossierAdmin {
            vue: vue.clone(),
            masquages,
            responsables: Vec::new(),
            contacts: Vec::new(),
            interdictions: Vec::new(),
        };
        if !vue.quelque_chose() {
            return Ok(sortie);
        }
        let mut mon_personne: Option<Uuid> = None;
        if vue.f0 {
            let tous = self.responsables_bruts(etab, eleve_id).await?;
            // R-A18 : une personne liée à un AUTRE élève avec lequel l'agent est en conflit
            // (l'ex-conjoint, responsable d'un autre enfant) reste fermée ici aussi.
            let vue_conflit = if mon_lien.is_none() && !vue.conflit_interet && vue.quelque_chose() {
                let ids: Vec<Uuid> = tous.iter().map(|r| r.personne.id).collect();
                let en_conflit = self.personnes_en_conflit(etab, actor, role, &ids).await?;
                if en_conflit.is_empty() {
                    None
                } else {
                    let d = self.droits_personnel_eleve(etab, actor, role, eleve_id).await?;
                    Some((en_conflit, vue_personnel(d, true)))
                }
            } else {
                None
            };
            sortie.responsables = match mon_lien {
                Some(mien) => {
                    mon_personne = tous
                        .iter()
                        .find(|r| r.lien_id == mien)
                        .map(|r| r.personne.id);
                    tous.into_iter()
                        .filter(|r| r.actif)
                        .map(|r| {
                            let v = vue_titulaire(r.lien_id == mien);
                            appliquer_vue(r, &v)
                        })
                        .collect()
                }
                // RP1-21 / RF-16 : un lien suspendu ou fermé n'est visible que d'un lecteur dont
                // la vue ouvre F3 (statut et dates des liens) ; sinon liens actifs seulement.
                None => tous
                    .into_iter()
                    .filter_map(|r| {
                        let v = match &vue_conflit {
                            Some((ids, v)) if ids.contains(&r.personne.id) => v,
                            _ => &vue,
                        };
                        (r.actif || v.f3).then(|| appliquer_vue(r, v))
                    })
                    .collect(),
            };
            if let Some((_, v)) = &vue_conflit {
                self.tracer_lecture_conflit(etab, actor, role, eleve_id, v).await;
            }
        } else if let Some(mien) = mon_lien {
            mon_personne = self.responsables_bruts(etab, eleve_id).await?.iter()
                .find(|r| r.lien_id == mien)
                .map(|r| r.personne.id);
        }
        if vue.r {
            let mut contacts = self.contacts_bruts(etab, eleve_id).await?;
            if mon_lien.is_some() {
                // Un titulaire voit ses propres contacts et ceux saisis par l'établissement.
                contacts.retain(|c| {
                    c.declare_par_personne_id.is_none() || c.declare_par_personne_id == mon_personne
                });
            }
            if let Some(mien) = mon_lien {
                // Un titulaire ne reçoit ni le commentaire interne d'un contact qu'il n'a pas
                // déclaré, ni les coordonnées d'un contact qui est en fait l'autre responsable.
                let autres: Vec<Uuid> = self
                    .responsables_bruts(etab, eleve_id)
                    .await?
                    .into_iter()
                    .filter(|r| r.lien_id != mien)
                    .map(|r| r.personne.id)
                    .collect();
                for c in &mut contacts {
                    if c.declare_par_personne_id != mon_personne {
                        c.commentaire = None;
                    }
                    if autres.contains(&c.personne.id) {
                        c.personne = c.personne.clone().sans_coordonnees();
                    }
                }
            }
            if !vue.f2 {
                for c in &mut contacts {
                    c.personne.adresse = None;
                }
            }
            sortie.contacts = contacts;
            if mon_lien.is_none() {
                sortie.interdictions = self
                    .interdictions_actives(etab, actor, eleve_id, vue.f3, vue.f4)
                    .await?;
            }
        }
        // Un motif ou une référence protégés ouverts (F4) sont une consultation sensible tracée.
        if mon_lien.is_none()
            && (sortie.interdictions.iter().any(|i| i.motif.is_some())
                || sortie.responsables.iter().any(|r| r.suspension_motif.is_some()))
        {
            self.tracer_acces_sensible(
                etab,
                actor,
                role,
                Some(eleve_id),
                "consultation",
                "motif_protege",
                "Consultation d'un motif protégé du dossier administratif (F4)",
            )
            .await;
        }
        Ok(sortie)
    }

    /// Trace d'accès sensible quand un agent lié à l'élève lit son dossier (R-A18).
    pub(crate) async fn tracer_lecture_conflit(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        vue: &VueGroupes,
    ) {
        if vue.acces_a_tracer() {
            self.tracer_acces_sensible(
                etab,
                actor,
                role,
                Some(eleve_id),
                "conflit_interet",
                "lecture_dossier_admin",
                "Lecture du dossier administratif par un agent lié à l'élève",
            )
            .await;
        }
    }

    /// Une écriture professionnelle sur ce dossier est refusée à l'agent en conflit d'intérêts :
    /// elle passe par un collègue (R-A18).
    pub(crate) async fn exiger_pas_de_conflit(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        if self
            .conflit_interet_eleve(etab, actor, role, eleve_id)
            .await?
            .is_some()
        {
            return Err(EntDbError::ConflitDossier(
                "conflit d'intérêts : cette modification doit être faite par un collègue",
            ));
        }
        Ok(())
    }

    /// Même règle pour une écriture sur une **personne** : refusée si elle touche un élève avec
    /// lequel l'agent est en conflit (ex-conjoint, enfants de l'agent…).
    pub(crate) async fn exiger_pas_de_conflit_personne(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
    ) -> Result<(), EntDbError> {
        if !Role::parse(role).map(Role::is_staff).unwrap_or(false) {
            return Ok(());
        }
        let eleves: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND personne_id = $2
             UNION
             SELECT eleve_id FROM ent_eleve_contact_urgence
              WHERE etablissement_id = $1 AND personne_id = $2",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        for (e,) in eleves {
            self.exiger_pas_de_conflit(etab, actor, role, e).await?;
        }
        Ok(())
    }

    /// Parmi `ids`, les personnes liées (responsable ou contact) à un élève avec lequel l'agent est
    /// en conflit d'intérêts (R-A18) ; sa propre fiche n'en fait pas partie. Sert à fermer F2 dans
    /// la recherche et la lecture des personnes, qui contournent sinon la lecture par élève.
    pub(crate) async fn personnes_en_conflit(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ids: &[Uuid],
    ) -> Result<std::collections::HashSet<Uuid>, EntDbError> {
        if ids.is_empty() || !Role::parse(role).map(Role::is_staff).unwrap_or(false) {
            return Ok(Default::default());
        }
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "WITH conflits AS (
                SELECT l.eleve_id FROM ent_personne p
                  JOIN ent_eleve_lien l ON l.personne_id = p.id AND l.etablissement_id = p.etablissement_id
                 WHERE p.etablissement_id = $1 AND p.user_id = $2
                UNION
                SELECT c.eleve_id FROM ent_personne p
                  JOIN ent_eleve_contact_urgence c ON c.personne_id = p.id AND c.etablissement_id = p.etablissement_id
                 WHERE p.etablissement_id = $1 AND p.user_id = $2
                UNION
                SELECT eleve_id FROM ent_conflit_interet
                 WHERE etablissement_id = $1 AND user_id = $2 AND leve_le IS NULL)
             SELECT DISTINCT x.personne_id FROM (
                SELECT personne_id, eleve_id FROM ent_eleve_lien WHERE etablissement_id = $1
                UNION ALL
                SELECT personne_id, eleve_id FROM ent_eleve_contact_urgence WHERE etablissement_id = $1) x
              WHERE x.eleve_id IN (SELECT eleve_id FROM conflits) AND x.personne_id = ANY($3)
                AND NOT EXISTS (SELECT 1 FROM ent_personne q
                                 WHERE q.etablissement_id = $1 AND q.id = x.personne_id AND q.user_id = $2)",
        )
        .bind(etab)
        .bind(actor)
        .bind(ids)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ───────────────────────────── Interdictions ─────────────────────────────

    async fn interdictions_actives(
        &self,
        etab: Uuid,
        lecteur: Uuid,
        eleve_id: Uuid,
        avec_fondement: bool,
        avec_motif: bool,
    ) -> Result<Vec<InterdictionVue>, EntDbError> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            Uuid,
            Option<Uuid>,
            String,
            Vec<String>,
            String,
            Option<String>,
            String,
            NaiveDate,
            Option<NaiveDate>,
        )> = sqlx::query_as(
            "SELECT i.id, i.personne_id,
                    COALESCE(NULLIF(btrim(concat_ws(' ', p.prenom, p.nom)), ''), i.personne_libelle, ''),
                    i.objet, i.fondement, i.motif_scelle, i.consigne, i.du, i.au
               FROM ent_eleve_interdiction i
               LEFT JOIN ent_personne p
                 ON p.id = i.personne_id AND p.etablissement_id = i.etablissement_id
              WHERE i.etablissement_id = $1 AND i.eleve_id = $2 AND i.levee_le IS NULL
                AND i.du <= ent_aujourdhui_etab($1, now())
                AND (i.au IS NULL OR i.au >= ent_aujourdhui_etab($1, now()))
                -- la personne visée ne lit jamais l'interdiction qui la vise, même agent
                AND NOT EXISTS (SELECT 1 FROM ent_personne q
                                 WHERE q.etablissement_id = i.etablissement_id AND q.user_id = $3
                                   AND (q.id = i.personne_id
                                        OR q.id = (SELECT s.fusionnee_dans FROM ent_personne s
                                                    WHERE s.etablissement_id = i.etablissement_id
                                                      AND s.id = i.personne_id)))
              ORDER BY i.decide_le, i.id",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(lecteur)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for (id, personne_id, libelle, objet, fondement, motif_scelle, consigne, du, au) in rows {
            let motif = match (avec_motif, motif_scelle) {
                (true, Some(m)) => Some(
                    coffre()
                        .ouvrir(TABLE_INTERDICTION, "motif_scelle", id, &m)
                        .map_err(EntDbError::Domain)?,
                ),
                _ => None,
            };
            out.push(InterdictionVue {
                id,
                personne_id,
                personne_libelle: libelle,
                objet,
                bloquante: fondement != "demande_titulaire",
                consigne,
                du,
                au,
                fondement: avec_fondement.then_some(fondement),
                motif,
            });
        }
        Ok(out)
    }

    /// Interdictions actives de l'élève, selon les groupes du lecteur (consigne : `remise.lire` ;
    /// fondement : F3 ; motif : F4). Jamais pour une famille. Une consultation avec motif ouvert
    /// est tracée.
    pub async fn interdictions_eleve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<InterdictionVue>, EntDbError> {
        let _ = self.dossier_exiger_eleve(etab, eleve_id).await?;
        let lecture = self.lecture_dossier_admin(etab, actor, role, eleve_id).await?;
        if !lecture.vue.r || Role::parse(role) == Some(Role::Parent) {
            return Err(EntDbError::Forbidden);
        }
        self.tracer_lecture_conflit(etab, actor, role, eleve_id, &lecture.vue).await;
        if lecture.interdictions.iter().any(|i| i.motif.is_some()) {
            self.tracer_acces_sensible(
                etab,
                actor,
                role,
                Some(eleve_id),
                "consultation",
                "interdiction_motif",
                "Consultation d'un motif d'interdiction (donnée protégée)",
            )
            .await;
        }
        Ok(lecture.interdictions)
    }

    /// Pose une interdiction (`remise.interdire`, établissement). Le motif est scellé ; le journal
    /// ne contient ni motif ni nom.
    pub async fn poser_interdiction_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        saisie: &InterdictionSaisie,
    ) -> Result<Uuid, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::RemiseInterdire).await?;
        let _ = self.dossier_exiger_eleve(etab, eleve_id).await?;
        self.exiger_pas_de_conflit(etab, actor, role, eleve_id).await?;
        let mut objet: Vec<String> = Vec::new();
        for o in &saisie.objet {
            let o = o.trim();
            if !OBJETS_INTERDICTION.contains(&o) {
                return Err(invalide("objet d'interdiction inconnu"));
            }
            if !objet.iter().any(|x| x == o) {
                objet.push(o.to_string());
            }
        }
        if objet.is_empty() {
            return Err(invalide("au moins un objet d'interdiction est obligatoire"));
        }
        if !FONDEMENTS_INTERDICTION.contains(&saisie.fondement.trim()) {
            return Err(invalide("fondement d'interdiction inconnu"));
        }
        let consigne = saisie.consigne.trim();
        if consigne.is_empty() || consigne.chars().count() > 300 {
            return Err(invalide("consigne obligatoire (300 caractères au plus)"));
        }
        let libelle = saisie
            .personne_libelle
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(l) = libelle {
            if l.chars().count() > 120 {
                return Err(invalide("nom de la personne visée trop long (120 caractères au plus)"));
            }
        }
        if saisie.personne_id.is_none() && libelle.is_none() {
            return Err(invalide("personne visée obligatoire (fiche ou nom)"));
        }
        let motif = saisie
            .motif
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if motif.map(|m| m.chars().count() > 2000).unwrap_or(false) {
            return Err(invalide("motif trop long (2 000 caractères au plus)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let du = match saisie.du {
            Some(d) => d,
            None => {
                let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())")
                    .bind(etab)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                j
            }
        };
        if saisie.au.map(|a| a < du).unwrap_or(false) {
            return Err(invalide("date de fin de l'interdiction antérieure à son début"));
        }
        let id = Uuid::new_v4();
        let scelle = match motif {
            Some(m) => Some(
                coffre()
                    .sceller(TABLE_INTERDICTION, "motif_scelle", id, m)
                    .map_err(EntDbError::Domain)?,
            ),
            None => None,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(pid) = saisie.personne_id {
            let p = personne_tx(&mut tx, etab, pid, false)
                .await?
                .ok_or(EntDbError::Domain(DomainError::NotFound))?;
            if p.fusionnee_dans.is_some() {
                return Err(invalide("personne fusionnée : viser la fiche conservée"));
            }
        }
        sqlx::query(
            "INSERT INTO ent_eleve_interdiction
                (id, etablissement_id, eleve_id, personne_id, personne_libelle, objet, fondement,
                 motif_scelle, consigne, du, au, decide_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve_id)
        .bind(saisie.personne_id)
        .bind(libelle)
        .bind(&objet)
        .bind(saisie.fondement.trim())
        .bind(scelle)
        .bind(consigne)
        .bind(du)
        .bind(saisie.au)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            Some(eleve_id),
            "interdiction_posee",
            "Interdiction posée",
            json!({ "interdiction_id": id, "objet": objet, "fondement": saisie.fondement.trim() }),
            false,
            actor,
        )
        .await?;
        commit(tx).await?;
        Ok(id)
    }

    /// Lève une interdiction (`remise.interdire`) : jamais de suppression, date et auteur gardés.
    pub async fn lever_interdiction_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        interdiction_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::RemiseInterdire).await?;
        let motif = motif.trim();
        if motif.is_empty() || motif.chars().count() > 300 {
            return Err(invalide("motif de levée obligatoire (300 caractères au plus)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let row: Option<(Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT eleve_id, levee_le FROM ent_eleve_interdiction
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(interdiction_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve_id, levee) = row.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if levee.is_some() {
            return Err(EntDbError::ConflitDossier("interdiction déjà levée"));
        }
        self.exiger_pas_de_conflit(etab, actor, role, eleve_id).await?;
        sqlx::query(
            "UPDATE ent_eleve_interdiction
                SET levee_le = now(), levee_par = $3, levee_motif = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(interdiction_id)
        .bind(actor)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            Some(eleve_id),
            "interdiction_levee",
            "Interdiction levée",
            json!({ "interdiction_id": interdiction_id }),
            false,
            actor,
        )
        .await?;
        commit(tx).await
    }

    // ───────────────────────── Conflits d'intérêts déclarés ─────────────────────────

    /// Déclare un conflit d'intérêts. L'agent déclare pour lui-même (`agent_user_id` absent ou égal
    /// à l'acteur : tout membre du personnel) ; la direction (`responsables.protege.gerer`)
    /// déclare pour un autre agent. Idempotent.
    pub async fn declarer_conflit_interet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_user_id: Option<Uuid>,
        eleve_id: Uuid,
        nature: &str,
    ) -> Result<Uuid, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let cible = agent_user_id.unwrap_or(actor);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let origine = if cible == actor {
            // RP2-14 : l'agent ne déclare que pour lui-même, sur un élève de son périmètre
            // (un professeur : un élève qu'il suit) ; sinon refus (pas d'oracle d'existence).
            if parsed == Role::Prof && !self.prof_suit_eleve(etab, actor, eleve_id).await? {
                return Err(EntDbError::Forbidden);
            }
            "agent"
        } else {
            self.dossier_exiger_action(etab, actor, role, Action::ResponsablesProtegeGerer)
                .await?;
            let membre: Option<(String,)> = sqlx::query_as(
                "SELECT role FROM ent_membership WHERE user_id = $1 AND etablissement_id = $2",
            )
            .bind(cible)
            .bind(etab)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match membre.and_then(|(r,)| Role::parse(&r)) {
                Some(r) if r.is_staff() => {}
                _ => return Err(invalide("l'agent désigné n'est pas membre du personnel")),
            }
            "direction"
        };
        let nature = nature.trim();
        if !NATURES_CONFLIT.contains(&nature) {
            return Err(invalide("nature de conflit d'intérêts inconnue"));
        }
        let _ = self.dossier_exiger_eleve(etab, eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        let inseree: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO ent_conflit_interet
                (id, etablissement_id, user_id, eleve_id, nature, origine, declare_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (etablissement_id, user_id, eleve_id) WHERE leve_le IS NULL DO NOTHING
             RETURNING id",
        )
        .bind(id)
        .bind(etab)
        .bind(cible)
        .bind(eleve_id)
        .bind(nature)
        .bind(origine)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((id,)) = inseree else {
            let (existant,): (Uuid,) = sqlx::query_as(
                "SELECT id FROM ent_conflit_interet
                  WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3 AND leve_le IS NULL",
            )
            .bind(etab)
            .bind(cible)
            .bind(eleve_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            return Ok(existant);
        };
        journal_tx(
            &mut tx,
            etab,
            Some(eleve_id),
            "conflit_interet_declare",
            "Conflit d'intérêts déclaré",
            json!({ "declaration_id": id, "user_id": cible, "nature": nature, "origine": origine }),
            false,
            actor,
        )
        .await?;
        commit(tx).await?;
        Ok(id)
    }

    /// Lève une déclaration (direction seulement ; un agent ne lève jamais sa propre déclaration).
    pub async fn lever_conflit_interet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        declaration_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::ResponsablesProtegeGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let row: Option<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT user_id, eleve_id, leve_le FROM ent_conflit_interet
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(declaration_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (user_id, eleve_id, leve) = row.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if user_id == actor {
            return Err(EntDbError::Forbidden);
        }
        if leve.is_some() {
            return Err(EntDbError::ConflitDossier("déclaration déjà levée"));
        }
        sqlx::query(
            "UPDATE ent_conflit_interet SET leve_le = now(), leve_par = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(declaration_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            Some(eleve_id),
            "conflit_interet_leve",
            "Conflit d'intérêts levé",
            json!({ "declaration_id": declaration_id, "user_id": user_id }),
            false,
            actor,
        )
        .await?;
        commit(tx).await
    }

    /// Tableau de la direction : déclarations actives et accès tracés des 30 derniers jours
    /// (agents liés à un élève dont ils ont lu le dossier). Action `responsables.protege.lire`.
    pub async fn conflits_interet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ConflitInteretVu>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::ResponsablesProtegeLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        #[allow(clippy::type_complexity)]
        let declarations: Vec<(Uuid, Uuid, Uuid, String, String)> = sqlx::query_as(
            "SELECT id, user_id, eleve_id, nature, origine FROM ent_conflit_interet
              WHERE etablissement_id = $1 AND leve_le IS NULL ORDER BY created_at, id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let acces: Vec<(Uuid, Uuid, i64, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT actor_user_id, eleve_id, count(*), max(created_at)
               FROM ent_journal_modif
              WHERE etablissement_id = $1 AND domaine = 'conflit_interet'
                AND created_at >= now() - interval '30 days'
                AND actor_user_id IS NOT NULL AND eleve_id IS NOT NULL
              GROUP BY actor_user_id, eleve_id ORDER BY max(created_at) DESC",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out: Vec<ConflitInteretVu> = declarations
            .into_iter()
            .map(|(id, user_id, eleve_id, nature, origine)| ConflitInteretVu {
                declaration_id: Some(id),
                user_id,
                eleve_id,
                origine,
                nature: Some(nature),
                acces_30j: 0,
                dernier_acces: None,
            })
            .collect();
        for (user_id, eleve_id, n, dernier) in acces {
            if let Some(l) = out.iter_mut().find(|l| l.user_id == user_id && l.eleve_id == eleve_id) {
                l.acces_30j = n;
                l.dernier_acces = dernier;
            } else {
                out.push(ConflitInteretVu {
                    declaration_id: None,
                    user_id,
                    eleve_id,
                    origine: "detecte".into(),
                    nature: None,
                    acces_30j: n,
                    dernier_acces: dernier,
                });
            }
        }
        Ok(out)
    }
}
