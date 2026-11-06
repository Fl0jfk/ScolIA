//! Bulletins périodiques versionnés et exports de livrets (socle 2, lot S2-7 :
//! EVA-10, EVA-12, EVA-13, EVA-14).
//!
//! - Génération : période **fermée** ; une version = un instantané JSON figé + un PDF
//!   dérivé ; une nouvelle version n'est créée que si le fond a changé (empreinte),
//!   avec un **motif** obligatoire à partir de la version 2 ; la base interdit de
//!   modifier ou supprimer une version (déclencheur `ent_trg_bulletin_immuable`).
//! - Publication : la version remplace la précédente publiée, est **archivée** dans le
//!   dossier de l'élève (tiroir famille) et l'événement `note.bulletin_publie` part dans
//!   la même transaction (outbox) ; le consommateur `d6.evaluations` notifie les
//!   responsables autorisés (`peut_voir_notes`, lien actif) et l'élève.
//! - Lecture : personnel avec `notes.lire` sur l'élève (toutes les versions) ; familles et
//!   élève : la version **publiée** seulement.
//! - Exports LSU / LSL : format **documenté** non validé contre le XSD officiel [NV] ;
//!   chaque export est tracé (`ent_eva_export`, événement `export.realise`).

use std::collections::HashMap;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use domain_eleve::evaluation::{
    format_fr, rendre_lsl_xml, rendre_lsu_xml, xml_bien_forme, LivretBilanCycle, LivretEleve,
    LivretMatiere, LivretPeriode, AVERTISSEMENT_LIVRETS,
};
use domain_eleve::{rendre_pdf_lignes, NotePeriodeStatut};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_object::ObjectStore;
use ent_types::evaluations::{
    BulletinConseilDto, BulletinContenuDto, BulletinDetailDto, BulletinDto, BulletinLigneDto,
    ExportLivretDto, GenererBulletinsBody, PublierBulletinsBody, RapportBulletinsDto,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::evaluations::{
    conflit, db, introuvable, refus, statut_periode_tx, texte_borne, types_eva, CalculClasse,
};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};

fn empreinte(octets: &[u8]) -> String {
    let h = Sha256::digest(octets);
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn moy(x: Option<f64>) -> String {
    x.map(format_fr).unwrap_or_else(|| "—".into())
}

fn libelle_mention(m: &str) -> &'static str {
    match m {
        "felicitations" => "Félicitations",
        "complement" => "Compliments",
        "encouragements" => "Encouragements",
        "avertissement_travail" => "Avertissement (travail)",
        "avertissement_conduite" => "Avertissement (conduite)",
        _ => "",
    }
}

fn libelle_decision(d: &str) -> &'static str {
    match d {
        "passage" => "Passage",
        "redoublement" => "Redoublement",
        "orientation" => "Orientation",
        "reserve" => "Décision réservée",
        _ => "",
    }
}

/// Lignes imprimées du bulletin (le PDF ne contient que l'instantané).
pub(crate) fn lignes_bulletin(c: &BulletinContenuDto) -> Vec<String> {
    let mut l = vec![
        format!("{} — Bulletin scolaire", c.etablissement),
        format!(
            "{} — {} au {}",
            c.periode_libelle,
            c.date_debut.format("%d/%m/%Y"),
            c.date_fin.format("%d/%m/%Y")
        ),
        format!("Eleve : {} {} · Classe : {}", c.eleve_prenom, c.eleve_nom, c.classe),
        format!("Version {}", c.version),
    ];
    if let Some(m) = &c.motif {
        l.push(format!("Motif de la nouvelle version : {m}"));
    }
    l.push(String::new());
    l.push("Matiere (coef.) : eleve | classe (min - max)".into());
    l.push("--------------------------------------------".into());
    for x in &c.lignes {
        l.push(format!(
            "{} ({}) : {} | {} ({} - {})",
            x.matiere,
            format_fr(x.coef),
            moy(x.moyenne),
            moy(x.moyenne_classe),
            moy(x.min),
            moy(x.max)
        ));
        if let Some(a) = &x.appreciation {
            for morceau in decouper(a, 95) {
                l.push(format!("   {morceau}"));
            }
        }
    }
    l.push(String::new());
    l.push(format!(
        "Moyenne generale : {} (classe : {})",
        moy(c.moyenne_generale),
        moy(c.moyenne_generale_classe)
    ));
    l.push(format!(
        "Vie scolaire : {} absence(s) dont {} non justifiee(s), {} retard(s)",
        c.absences, c.absences_non_justifiees, c.retards
    ));
    if let Some(a) = &c.appreciation_generale {
        l.push(String::new());
        l.push("Appreciation generale :".into());
        for morceau in decouper(a, 95) {
            l.push(format!("   {morceau}"));
        }
    }
    if let Some(cs) = &c.conseil {
        l.push(String::new());
        l.push("Conseil de classe :".into());
        if !cs.appreciation.is_empty() {
            for morceau in decouper(&cs.appreciation, 95) {
                l.push(format!("   {morceau}"));
            }
        }
        if let Some(m) = cs.mention.as_deref().map(libelle_mention).filter(|s| !s.is_empty()) {
            l.push(format!("   Mention : {m}"));
        }
        if let Some(d) = cs.decision_code.as_deref().map(libelle_decision).filter(|s| !s.is_empty()) {
            l.push(format!("   Decision : {d}"));
        }
        if let Some(a) = &cs.avis_orientation {
            l.push(format!("   Avis d'orientation : {a}"));
        }
    }
    l.push(String::new());
    l.push(format!(
        "Moyennes arrondies ({}) ; absent, dispense et non note exclus du calcul.",
        c.arrondi
    ));
    l
}

fn decouper(texte: &str, largeur: usize) -> Vec<String> {
    let mut out = Vec::new();
    for para in texte.lines() {
        let mut cur = String::new();
        for mot in para.split_whitespace() {
            if !cur.is_empty() && cur.chars().count() + 1 + mot.chars().count() > largeur {
                out.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(mot);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

type BulletinTuple = (
    Uuid,
    Uuid,
    String,
    String,
    Uuid,
    i32,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<String>,
);

const SELECT_BULLETIN: &str = "SELECT b.id, b.eleve_id, e.nom, e.prenom, b.periode_id, b.version, b.statut,
        b.empreinte, b.genere_le, b.publie_le, b.motif
   FROM ent_bulletin b
   JOIN ent_eleve e ON e.id = b.eleve_id AND e.etablissement_id = b.etablissement_id
  WHERE b.etablissement_id = $1";

fn bulletin_de(t: BulletinTuple) -> BulletinDto {
    BulletinDto {
        id: t.0,
        eleve_id: t.1,
        eleve_nom: t.2,
        eleve_prenom: t.3,
        periode_id: t.4,
        version: t.5,
        statut: t.6,
        empreinte: t.7,
        genere_le: t.8,
        publie_le: t.9,
        motif: t.10,
    }
}

/// Compteurs de vie scolaire sur la période : (absences, dont non justifiées, retards).
type Compteurs = HashMap<Uuid, (i64, i64, i64)>;

impl PgEnt {
    pub(crate) async fn eva_compteurs_vs(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Compteurs, EntDbError> {
        let absences: Vec<(Uuid, i64, i64)> = sqlx::query_as(
            "SELECT eleve_id, count(*)::bigint,
                    count(*) FILTER (WHERE statut <> 'justifiee')::bigint
               FROM ent_absence
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND debut <= $4 AND fin >= $3
                AND statut <> 'annulee_present'
              GROUP BY eleve_id",
        )
        .bind(etab)
        .bind(eleves)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let retards: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT eleve_id, count(*)::bigint FROM ent_retard
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND jour BETWEEN $3 AND $4
              GROUP BY eleve_id",
        )
        .bind(etab)
        .bind(eleves)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out: Compteurs = HashMap::new();
        for (e, n, nj) in absences {
            out.entry(e).or_default().0 = n;
            out.entry(e).or_default().1 = nj;
        }
        for (e, r) in retards {
            out.entry(e).or_default().2 = r;
        }
        Ok(out)
    }

    async fn eva_nom_etablissement(&self, etab: Uuid) -> Result<String, EntDbError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT nom FROM ent_etablissement WHERE id = $1")
            .bind(etab)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
        Ok(row.map(|(n,)| n).unwrap_or_else(|| "Établissement".into()))
    }

    /// Contenus (version 0, sans motif) de tous les élèves de la classe.
    async fn eva_contenus_bulletins(
        &self,
        etab: Uuid,
        calc: &CalculClasse,
    ) -> Result<Vec<(Uuid, BulletinContenuDto)>, EntDbError> {
        let a = calc.regles.arrondi;
        let ids: Vec<Uuid> = calc.eleves.iter().map(|e| e.0).collect();
        let apprs: Vec<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT eleve_id, matiere_id, texte FROM ent_note_appreciation
              WHERE etablissement_id = $1 AND periode_id = $2 AND eleve_id = ANY($3)",
        )
        .bind(etab)
        .bind(calc.periode.id)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        // Décisions du conseil le plus récent (non annulé) de la classe pour la période.
        type DecisionTuple = (Uuid, String, Option<String>, Option<String>, Option<String>);
        let decisions: Vec<DecisionTuple> = sqlx::query_as(
            "SELECT d.eleve_id, d.appreciation, d.mention, d.decision_code, d.avis_orientation
               FROM ent_conseil_classe_dossier d
              WHERE d.etablissement_id = $1 AND d.conseil_id = (
                    SELECT c.id FROM ent_conseil_classe c
                     WHERE c.etablissement_id = $1 AND c.classe_id = $2 AND c.periode_id = $3
                       AND c.statut <> 'annule'
                     ORDER BY c.jour DESC, c.created_at DESC LIMIT 1)",
        )
        .bind(etab)
        .bind(calc.classe_id)
        .bind(calc.periode.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let compteurs = self
            .eva_compteurs_vs(etab, &ids, calc.periode.date_debut, calc.periode.date_fin)
            .await?;
        let nom_etab = self.eva_nom_etablissement(etab).await?;
        let mut out = Vec::new();
        for (id, nom, prenom) in &calc.eleves {
            let lignes = calc
                .matieres
                .iter()
                .map(|(m, lib, _, coef)| {
                    let st = calc.stats_matiere.get(m);
                    BulletinLigneDto {
                        matiere: lib.clone(),
                        coef: *coef,
                        moyenne: calc.moyennes.get(&(*id, *m)).map(|x| x.arrondie(a)),
                        moyenne_classe: st.map(|s| s.moyenne.arrondie(a)),
                        min: st.map(|s| s.min.arrondie(a)),
                        max: st.map(|s| s.max.arrondie(a)),
                        appreciation: apprs
                            .iter()
                            .find(|(e, mm, _)| e == id && *mm == Some(*m))
                            .map(|(_, _, t)| t.clone()),
                    }
                })
                .collect();
            let conseil = decisions
                .iter()
                .find(|d| d.0 == *id)
                .map(|(_, appreciation, mention, decision_code, avis)| BulletinConseilDto {
                    appreciation: appreciation.clone(),
                    mention: mention.clone().filter(|m| m != "aucune"),
                    decision_code: decision_code.clone().filter(|d| d != "aucune"),
                    avis_orientation: avis.clone(),
                });
            let (abs, abs_nj, ret) = compteurs.get(id).copied().unwrap_or_default();
            out.push((
                *id,
                BulletinContenuDto {
                    etablissement: nom_etab.clone(),
                    eleve_nom: nom.clone(),
                    eleve_prenom: prenom.clone(),
                    classe: calc.classe_code.clone(),
                    periode_code: calc.periode.code.clone(),
                    periode_libelle: calc.periode.libelle.clone(),
                    date_debut: calc.periode.date_debut,
                    date_fin: calc.periode.date_fin,
                    arrondi: a.as_str().to_string(),
                    lignes,
                    moyenne_generale: calc.generales.get(id).map(|g| g.arrondie(a)),
                    moyenne_generale_classe: calc.stats_generale.map(|s| s.moyenne.arrondie(a)),
                    appreciation_generale: apprs
                        .iter()
                        .find(|(e, m, _)| e == id && m.is_none())
                        .map(|(_, _, t)| t.clone()),
                    conseil,
                    absences: abs,
                    absences_non_justifiees: abs_nj,
                    retards: ret,
                    version: 0,
                    motif: None,
                },
            ));
        }
        Ok(out)
    }

    async fn exiger_action_bulletins(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if self
            .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// EVA-10 / EVA-14 : génère les bulletins d'une classe (nouvelle version si le fond a changé).
    pub async fn generer_bulletins_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &GenererBulletinsBody,
    ) -> Result<RapportBulletinsDto, EntDbError> {
        self.exiger_action_bulletins(etab, acteur, role).await?;
        let periode = self.eva_periode(etab, body.periode_id).await?;
        if periode.statut != NotePeriodeStatut::Fermee {
            return Err(conflit("période non fermée : bulletin impossible"));
        }
        let motif = match body.motif.as_deref() {
            Some(m) => Some(texte_borne(m, 500)?).filter(|m| !m.is_empty()),
            None => None,
        };
        let calc = self
            .eva_calculer_classe(etab, body.classe_id, body.periode_id, false)
            .await?;
        let contenus = self.eva_contenus_bulletins(etab, &calc).await?;
        // Première passe : ce qui change, sans rien écrire.
        let mut a_creer = Vec::new();
        let mut inchanges = 0u32;
        for (eleve, contenu) in contenus {
            let fond = serde_json::to_vec(&contenu).map_err(|_| refus("bulletin non sérialisable"))?;
            let emp = empreinte(&fond);
            let dernier: Option<(i32, String)> = sqlx::query_as(
                "SELECT version, empreinte FROM ent_bulletin
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND periode_id = $3
                  ORDER BY version DESC LIMIT 1",
            )
            .bind(etab)
            .bind(eleve)
            .bind(body.periode_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            match dernier {
                Some((_, e)) if e == emp => inchanges += 1,
                Some((v, _)) => {
                    if motif.as_deref().is_none_or(|m| m.chars().count() < 3) {
                        return Err(refus("motif obligatoire pour une nouvelle version de bulletin"));
                    }
                    a_creer.push((eleve, contenu, emp, v + 1));
                }
                None => a_creer.push((eleve, contenu, emp, 1)),
            }
        }
        let mut crees = Vec::new();
        let mut tx = self.pool().begin().await.map_err(db)?;
        if statut_periode_tx(&mut tx, etab, body.periode_id).await? != NotePeriodeStatut::Fermee {
            return Err(conflit("période non fermée : bulletin impossible"));
        }
        for (eleve, mut contenu, emp, version) in a_creer {
            contenu.version = version;
            contenu.motif = if version > 1 { motif.clone() } else { None };
            let id = Uuid::new_v4();
            let pdf = rendre_pdf_lignes(&lignes_bulletin(&contenu));
            let key = ObjectStore::key_for(etab, "bulletins", id, "pdf");
            self.objects().put(&key, &pdf)?;
            let json = serde_json::to_value(&contenu).map_err(|_| refus("bulletin non sérialisable"))?;
            sqlx::query(
                "UPDATE ent_bulletin SET statut = 'remplace'
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND periode_id = $3 AND statut = 'genere'",
            )
            .bind(etab)
            .bind(eleve)
            .bind(body.periode_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_bulletin (id, etablissement_id, eleve_id, periode_id, version, contenu, empreinte,
                                           storage_key, octets, motif, genere_par_user_id)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            )
            .bind(id)
            .bind(etab)
            .bind(eleve)
            .bind(body.periode_id)
            .bind(version)
            .bind(&json)
            .bind(&emp)
            .bind(&key)
            .bind(pdf.len() as i32)
            .bind(&contenu.motif)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_eva::BULLETIN_GENERE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve,
                eleve_id: Some(eleve),
                payload: json!({ "periode_id": body.periode_id, "bulletin_id": id, "version": version }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
            crees.push(id);
        }
        tx.commit().await.map_err(db)?;
        let bulletins = self.eva_bulletins_par_ids(etab, &crees).await?;
        Ok(RapportBulletinsDto { bulletins, inchanges })
    }

    async fn eva_bulletins_par_ids(&self, etab: Uuid, ids: &[Uuid]) -> Result<Vec<BulletinDto>, EntDbError> {
        let rows: Vec<BulletinTuple> = sqlx::query_as(&format!(
            "{SELECT_BULLETIN} AND b.id = ANY($2) ORDER BY e.nom, e.prenom, b.version"
        ))
        .bind(etab)
        .bind(ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(bulletin_de).collect())
    }

    /// Publie la dernière version générée de chaque élève de la classe : archivage au
    /// dossier (tiroir famille) et événement `note.bulletin_publie` (outbox).
    pub async fn publier_bulletins_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &PublierBulletinsBody,
    ) -> Result<RapportBulletinsDto, EntDbError> {
        self.exiger_action_bulletins(etab, acteur, role).await?;
        let periode = self.eva_periode(etab, body.periode_id).await?;
        if periode.statut != NotePeriodeStatut::Fermee {
            return Err(conflit("période rouverte : régénérer après fermeture avant de publier"));
        }
        let effectif = self
            .eva_effectif(etab, Some(body.classe_id), None, periode.date_debut, periode.date_fin)
            .await?;
        let ids: Vec<Uuid> = effectif.iter().map(|e| e.0).collect();
        let a_publier: Vec<(Uuid, Uuid, i32, String)> = sqlx::query_as(
            "SELECT id, eleve_id, version, storage_key FROM ent_bulletin
              WHERE etablissement_id = $1 AND periode_id = $2 AND eleve_id = ANY($3) AND statut = 'genere'
              ORDER BY eleve_id, version DESC",
        )
        .bind(etab)
        .bind(body.periode_id)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let total = ids.len() as u32;
        let mut publies = Vec::new();
        let mut tx = self.pool().begin().await.map_err(db)?;
        if statut_periode_tx(&mut tx, etab, body.periode_id).await? != NotePeriodeStatut::Fermee {
            return Err(conflit("période rouverte : régénérer après fermeture avant de publier"));
        }
        let mut vus = std::collections::HashSet::new();
        for (id, eleve, version, key) in a_publier {
            if !vus.insert(eleve) {
                continue; // seule la plus récente version générée
            }
            // L'ancienne version publiée reste archivée au dossier, mais n'est plus
            // visible des familles (confidentialité « restreint »).
            sqlx::query(
                "UPDATE ent_eleve_document SET confidentialite = 'restreint'
                  WHERE etablissement_id = $1 AND id IN (
                        SELECT b.document_id FROM ent_bulletin b
                         WHERE b.etablissement_id = $1 AND b.eleve_id = $2 AND b.periode_id = $3
                           AND b.statut = 'publie' AND b.document_id IS NOT NULL)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(body.periode_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_bulletin SET statut = 'remplace'
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND periode_id = $3 AND statut = 'publie'",
            )
            .bind(etab)
            .bind(eleve)
            .bind(body.periode_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let doc_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, code, mime_type,
                                                 storage_key, confidentialite, source, created_by_user_id)
                 VALUES ($1,$2,$3,'famille',$4,$5,'application/pdf',$6,'standard','bulletin',$7)",
            )
            .bind(doc_id)
            .bind(etab)
            .bind(eleve)
            .bind(format!("Bulletin {} (version {version})", periode.libelle))
            .bind(format!("BULLETIN_{}_V{version}", periode.code))
            .bind(&key)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_bulletin SET statut = 'publie', publie_le = now(), publie_par_user_id = $3,
                                         document_id = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(acteur)
            .bind(doc_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::NOTE_BULLETIN_PUBLIE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve,
                eleve_id: Some(eleve),
                payload: json!({
                    "periode_id": body.periode_id,
                    "bulletin_id": id,
                    "version": version,
                    "document_id": doc_id,
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
            publies.push(id);
        }
        tx.commit().await.map_err(db)?;
        let bulletins = self.eva_bulletins_par_ids(etab, &publies).await?;
        Ok(RapportBulletinsDto {
            inchanges: total.saturating_sub(bulletins.len() as u32),
            bulletins,
        })
    }

    /// Versions des bulletins d'une classe sur une période (direction, ou lecture des
    /// notes de l'établissement).
    pub async fn bulletins_classe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
        periode_id: Uuid,
    ) -> Result<Vec<BulletinDto>, EntDbError> {
        Self::exiger_staff(role)?;
        if !(self
            .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
            .await?
            || self
                .a_action_etablissement(etab, acteur, role, Action::NotesLire)
                .await?
            || self.eva_est_pp(etab, acteur, classe_id).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let periode = self.eva_periode(etab, periode_id).await?;
        let ids: Vec<Uuid> = self
            .eva_effectif(etab, Some(classe_id), None, periode.date_debut, periode.date_fin)
            .await?
            .into_iter()
            .map(|e| e.0)
            .collect();
        let rows: Vec<BulletinTuple> = sqlx::query_as(&format!(
            "{SELECT_BULLETIN} AND b.periode_id = $2 AND b.eleve_id = ANY($3)
              ORDER BY e.nom, e.prenom, b.version DESC"
        ))
        .bind(etab)
        .bind(periode_id)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(bulletin_de).collect())
    }

    /// `true` : personnel (toutes versions) ; `false` : famille ou élève (publiée seule).
    async fn eva_acces_bulletins(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        if !self.actor_can_read_notes(etab, acteur, role, eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(Role::parse(role).is_some_and(|r| r.is_staff()))
    }

    pub async fn bulletins_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<BulletinDto>, EntDbError> {
        let staff = self.eva_acces_bulletins(etab, acteur, role, eleve_id).await?;
        let rows: Vec<BulletinTuple> = sqlx::query_as(&format!(
            "{SELECT_BULLETIN} AND b.eleve_id = $2 AND ($3 OR b.statut = 'publie')
              ORDER BY b.genere_le DESC, b.version DESC"
        ))
        .bind(etab)
        .bind(eleve_id)
        .bind(staff)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(bulletin_de).collect())
    }

    async fn eva_bulletin_accessible(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        bulletin_id: Uuid,
    ) -> Result<(BulletinDto, serde_json::Value, String), EntDbError> {
        let row: Option<(Uuid, String, serde_json::Value, String)> = sqlx::query_as(
            "SELECT eleve_id, statut, contenu, storage_key FROM ent_bulletin
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(bulletin_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (eleve_id, statut, contenu, key) = row.ok_or_else(introuvable)?;
        let staff = match self.eva_acces_bulletins(etab, acteur, role, eleve_id).await {
            Ok(s) => s,
            // Une famille sans droit ne doit pas apprendre l'existence du bulletin.
            Err(EntDbError::Forbidden) if !Role::parse(role).is_some_and(|r| r.is_staff()) => {
                return Err(introuvable())
            }
            Err(e) => return Err(e),
        };
        if !staff && statut != "publie" {
            return Err(introuvable());
        }
        let dto = self
            .eva_bulletins_par_ids(etab, &[bulletin_id])
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        Ok((dto, contenu, key))
    }

    pub async fn bulletin_detail_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        bulletin_id: Uuid,
    ) -> Result<BulletinDetailDto, EntDbError> {
        let (bulletin, contenu, _) = self.eva_bulletin_accessible(etab, acteur, role, bulletin_id).await?;
        let contenu: BulletinContenuDto =
            serde_json::from_value(contenu).map_err(|_| refus("bulletin illisible"))?;
        Ok(BulletinDetailDto { bulletin, contenu })
    }

    /// PDF d'une version (nom de fichier, octets).
    pub async fn bulletin_pdf_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        bulletin_id: Uuid,
    ) -> Result<(String, Vec<u8>), EntDbError> {
        let (b, _, key) = self.eva_bulletin_accessible(etab, acteur, role, bulletin_id).await?;
        let octets = self.objects().get(&key)?;
        let nom = format!(
            "bulletin-{}-{}-v{}.pdf",
            nettoyer_nom(&b.eleve_nom),
            nettoyer_nom(&b.eleve_prenom),
            b.version
        );
        Ok((nom, octets))
    }

    // ───────────────────────────── Exports LSU / LSL ─────────────────────────────

    /// EVA-12 : export d'une classe et d'une période au format documenté [NV].
    pub async fn exporter_livret_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        format: &str,
        classe_id: Uuid,
        periode_id: Uuid,
    ) -> Result<ExportLivretDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::LivretsExporter)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if !matches!(format, "lsu" | "lsl") {
            return Err(refus("format attendu : lsu ou lsl"));
        }
        let calc = self.eva_calculer_classe(etab, classe_id, periode_id, false).await?;
        let a = calc.regles.arrondi;
        let ids: Vec<Uuid> = calc.eleves.iter().map(|e| e.0).collect();
        let ines: Vec<(Uuid, Option<String>)> =
            sqlx::query_as("SELECT id, ine FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)")
                .bind(etab)
                .bind(&ids)
                .fetch_all(self.pool())
                .await
                .map_err(db)?;
        let ines: HashMap<Uuid, String> = ines
            .into_iter()
            .map(|(id, ine)| (id, ine.unwrap_or_default().trim().to_string()))
            .collect();
        let apprs: Vec<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT eleve_id, matiere_id, texte FROM ent_note_appreciation
              WHERE etablissement_id = $1 AND periode_id = $2 AND eleve_id = ANY($3)",
        )
        .bind(etab)
        .bind(periode_id)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let compteurs = self
            .eva_compteurs_vs(etab, &ids, calc.periode.date_debut, calc.periode.date_fin)
            .await?;
        type BilanTuple = (Uuid, Uuid, i16, String);
        let bilans: Vec<BilanTuple> = sqlx::query_as(
            "SELECT b.id, b.eleve_id, b.cycle, b.synthese FROM ent_bilan_cycle b
              WHERE b.etablissement_id = $1 AND b.eleve_id = ANY($2) AND b.statut = 'valide'
                AND b.annee_id IN (SELECT y.id FROM ent_annee_scolaire y
                                    WHERE y.etablissement_id = $1 AND y.date_debut <= $3 AND y.date_fin >= $3)
              ORDER BY b.valide_le DESC",
        )
        .bind(etab)
        .bind(&ids)
        .bind(calc.periode.date_debut)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let comps: Vec<(Uuid, String, i16)> = sqlx::query_as(
            "SELECT bilan_id, composante, niveau FROM ent_bilan_cycle_composante
              WHERE etablissement_id = $1 AND bilan_id = ANY($2) ORDER BY composante",
        )
        .bind(etab)
        .bind(bilans.iter().map(|b| b.0).collect::<Vec<_>>())
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let uai: Option<(String,)> = sqlx::query_as(
            "SELECT uai FROM ent_site WHERE etablissement_id = $1 AND uai IS NOT NULL AND btrim(uai) <> ''
              ORDER BY code LIMIT 1",
        )
        .bind(etab)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        // Rang de la période dans son année scolaire.
        let soeurs: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT p.id FROM ent_note_periode p
              WHERE p.etablissement_id = $1
                AND p.date_debut >= (SELECT y.date_debut FROM ent_annee_scolaire y
                                      WHERE y.etablissement_id = $1 AND y.date_debut <= $2 AND y.date_fin >= $2
                                      ORDER BY y.date_debut DESC LIMIT 1)
                AND p.date_debut <= (SELECT y.date_fin FROM ent_annee_scolaire y
                                      WHERE y.etablissement_id = $1 AND y.date_debut <= $2 AND y.date_fin >= $2
                                      ORDER BY y.date_debut DESC LIMIT 1)
              ORDER BY p.date_debut, p.code",
        )
        .bind(etab)
        .bind(calc.periode.date_debut)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let indice = soeurs.iter().position(|(id,)| *id == periode_id).map(|i| i + 1).unwrap_or(1);
        let mut anomalies = Vec::new();
        if uai.is_none() {
            anomalies.push("UAI de l'établissement absent (référentiel : sites)".to_string());
        }
        let millesime = calc.periode.date_debut.year()
            - if calc.periode.date_debut.month() < 8 { 1 } else { 0 };
        let mut eleves = Vec::new();
        for (id, nom, prenom) in &calc.eleves {
            let ine = ines.get(id).cloned().unwrap_or_default();
            if ine.is_empty() {
                anomalies.push(format!("INE manquant : {prenom} {nom}"));
            }
            let (abs, abs_nj, ret) = compteurs.get(id).copied().unwrap_or_default();
            let bilan = bilans.iter().find(|b| b.1 == *id).map(|b| LivretBilanCycle {
                cycle: b.2,
                composantes: comps
                    .iter()
                    .filter(|c| c.0 == b.0)
                    .map(|c| (c.1.clone(), c.2))
                    .collect(),
                synthese: b.3.clone(),
            });
            eleves.push(LivretEleve {
                ine,
                nom: nom.clone(),
                prenom: prenom.clone(),
                division: calc.classe_code.clone(),
                matieres: calc
                    .matieres
                    .iter()
                    .map(|(m, lib, code, _)| LivretMatiere {
                        code: code.clone(),
                        libelle: lib.clone(),
                        moyenne: calc.moyennes.get(&(*id, *m)).map(|x| x.arrondie(a)),
                        moyenne_classe: calc.stats_matiere.get(m).map(|s| s.moyenne.arrondie(a)),
                        appreciation: apprs
                            .iter()
                            .find(|(e, mm, _)| e == id && *mm == Some(*m))
                            .map(|(_, _, t)| t.clone()),
                    })
                    .collect(),
                appreciation_generale: apprs
                    .iter()
                    .find(|(e, mm, _)| e == id && mm.is_none())
                    .map(|(_, _, t)| t.clone()),
                absences_justifiees: abs - abs_nj,
                absences_injustifiees: abs_nj,
                retards: ret,
                bilan_cycle: if format == "lsu" { bilan } else { None },
            });
        }
        let livret = LivretPeriode {
            uai: uai.map(|(u,)| u).unwrap_or_default(),
            millesime,
            periode_code: calc.periode.code.clone(),
            periode_indice: indice as u32,
            nb_periodes: soeurs.len().max(1) as u32,
            eleves,
        };
        let contenu = if format == "lsu" {
            rendre_lsu_xml(&livret)
        } else {
            rendre_lsl_xml(&livret)
        };
        xml_bien_forme(&contenu).map_err(|_| refus("export XML mal formé"))?;
        let emp = empreinte(contenu.as_bytes());
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_eva_export (id, etablissement_id, format, periode_id, classe_id, nb_eleves, empreinte,
                                         conforme_verifie, exporte_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,false,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(format)
        .bind(periode_id)
        .bind(classe_id)
        .bind(livret.eleves.len() as i32)
        .bind(&emp)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::EXPORT_REALISE.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: periode_id,
            eleve_id: None,
            payload: json!({ "export_id": id, "format": format, "classe_id": classe_id, "nb_eleves": livret.eleves.len() }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ExportLivretDto {
            id,
            format: format.to_string(),
            nom_fichier: format!(
                "{format}-{}-{}.xml",
                nettoyer_nom(&calc.classe_code),
                nettoyer_nom(&calc.periode.code)
            ),
            contenu,
            avertissement: AVERTISSEMENT_LIVRETS.to_string(),
            anomalies,
            nb_eleves: livret.eleves.len() as u32,
            empreinte: emp,
        })
    }
}

/// Nom de fichier sûr : lettres ASCII, chiffres et tirets.
fn nettoyer_nom(s: &str) -> String {
    let n: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let n = n.trim_matches('-').to_string();
    if n.is_empty() {
        "x".into()
    } else {
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoupage_et_noms() {
        assert_eq!(decouper("un deux trois", 7), vec!["un deux", "trois"]);
        assert_eq!(nettoyer_nom("Élève/../4B"), "l-ve----4b");
        assert_eq!(nettoyer_nom("///"), "x");
        assert_eq!(empreinte(b"abc").len(), 64);
    }
}
