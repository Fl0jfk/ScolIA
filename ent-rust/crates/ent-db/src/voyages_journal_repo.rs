//! Socle 42, lot V0d : journal de voyage pour les familles (`docs/socles/42_SORTIES_VOYAGES.md` §11.4 ;
//! NX-33, NX-34, NX-80 ; VOY-176 à VOY-179, VOY-254, VOY-265 ; R-SV40, R-SV42). Corrige VD-10, VD-11, VD-12,
//! VD-43 et la publication de photos sans droit à l'image.
//!
//! * **audience recalculée à chaque lecture** : les titulaires (et l'élève majeur) des participants qui
//!   partent encore ; un élève retiré ou désisté, un titulaire dont le lien a pris fin ou que l'établissement
//!   ne peut plus solliciter : accès retiré ;
//! * **droit à l'image** : une photo n'est montrée que si **chaque** élève marqué dessus a un consentement
//!   effectif `accepte` au socle 2 (`droit_image`). L'absence de décision n'est pas un consentement (R-SV40) ;
//!   une photo sans élève marqué exige la déclaration « aucun élève identifiable » ; le marquage reste
//!   **interne** : une famille ne voit jamais le nom d'un autre élève (R-SV42) ;
//! * **familles sans compte** : lien nominatif par (participant, titulaire), jeton jamais conservé (empreinte
//!   seule), révocable, expirant à la fin du séjour + 15 jours, avec limitation de débit ; la distribution
//!   des jetons passe par le port [`DistributionLien`] (l'envoi réel attend le socle 8, lot N6) ;
//! * **points de situation codés** (arrivée, départ, changement de programme) : jamais de texte libre
//!   long, donc jamais d'incident médical ni de nom d'élève ;
//! * **purge** : à la fin du séjour + 15 jours, publications et photos sont détruites, sauf la sélection
//!   gardée pour le bilan (sans élève non consentant).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use serde::Serialize;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::formulaires::{etats_effectifs, OBJET_DROIT_IMAGE};
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages_prestataires::ligne_propre;
use crate::voyages_repo::{
    charger_avec_acces, interdit, journaliser, personne_de, publier, refus, tx, Acces, Fait, SortieLigne,
};

const PHOTOS_MAX: usize = 10;
const ELEVES_PAR_PHOTO_MAX: usize = 60;
/// Limitation de débit d'un lien nominatif : au plus 30 lectures par fenêtre de 10 minutes.
const LIEN_LECTURES_MAX: i32 = 30;
const ETATS_PUBLIABLES: [&str; 4] = ["ouverte", "prete", "en_cours", "rentree"];

// ───────────────────────── Port de distribution ─────────────────────────

/// Port de distribution d'un lien nominatif à un titulaire sans compte. L'implémentation réelle (courriel
/// ou SMS par le socle 8, lot N6, depuis le sous-domaine de l'établissement) n'existe pas encore : un test
/// ou le futur consommateur fournit la sienne. Le jeton n'est remis **qu'ici**, une seule fois.
pub trait DistributionLien: Send + Sync {
    fn distribuer(&self, etablissement_id: Uuid, sortie_id: Uuid, personne_id: Uuid, jeton: &str);
}

/// Distribution « rien » : les liens émis ne sont remis à personne (utile pour compter ou révoquer).
pub struct SansDistribution;

impl DistributionLien for SansDistribution {
    fn distribuer(&self, _: Uuid, _: Uuid, _: Uuid, _: &str) {}
}

// ───────────────────────── Types ─────────────────────────

/// Photo à publier. Le contenu passe par la politique de dépôt commune (type détecté sur les octets).
#[derive(Debug, Clone)]
pub struct PhotoSaisie {
    pub nom: String,
    pub mime: Option<String>,
    pub contenu: Vec<u8>,
    /// Élèves visibles sur la photo (`eleve_id` de participants de la sortie). Vide = « aucun identifiable ».
    pub eleves: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct PublicationSaisie {
    /// `message` ou `point_de_situation`.
    pub type_: String,
    /// `arrivee`, `depart` ou `changement_programme` pour un point de situation.
    pub nature_point: Option<String>,
    pub texte: String,
    /// Déclaration de l'auteur : les photos de cette publication ne montrent aucun élève identifiable.
    pub aucun_eleve_identifiable: bool,
    pub photos: Vec<PhotoSaisie>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhotoVue {
    pub id: Uuid,
    pub nom: String,
    pub taille: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicationVue {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub type_: String,
    pub nature_point: Option<String>,
    pub texte: String,
    pub publiee_le: DateTime<Utc>,
    /// Vue de gestion seulement : les familles ne voient ni auteur ni publication retirée.
    pub retiree: Option<bool>,
    pub gardee_pour_bilan: Option<bool>,
    pub photos: Vec<PhotoVue>,
    /// Vue de gestion seulement : photos non montrées faute de consentement ou de marquage.
    pub photos_masquees: Option<i64>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DelegueVue {
    pub encadrant_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JournalVue {
    pub actif: bool,
    pub ferme_le: Option<DateTime<Utc>>,
    pub delegues: Vec<DelegueVue>,
    pub publications: Vec<PublicationVue>,
    pub liens_actifs: i64,
    pub peut_publier: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JournalFamille {
    pub sortie_id: Uuid,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// Toujours « L'équipe du voyage » : aucun nom de personnel (§11.4).
    pub auteur: &'static str,
    pub publications: Vec<PublicationVue>,
}

// ───────────────────────── Aides ─────────────────────────

#[derive(FromRow)]
struct PubLigne {
    id: Uuid,
    #[sqlx(rename = "type")]
    type_: String,
    nature_point: Option<String>,
    texte: String,
    aucun_eleve_identifiable: bool,
    publiee_le: DateTime<Utc>,
    retiree_le: Option<DateTime<Utc>>,
    gardee_pour_bilan: bool,
}

#[derive(FromRow)]
struct PhotoLigne {
    id: Uuid,
    publication_id: Uuid,
    nom: String,
    taille: i32,
}

/// Une photo est montrée si chaque élève marqué a un consentement effectif ; sans marquage, seulement si
/// l'auteur a déclaré « aucun élève identifiable ».
fn photo_montrable(marques: &[Uuid], aucun: bool, consentis: &HashSet<Uuid>) -> bool {
    if marques.is_empty() {
        aucun
    } else {
        marques.iter().all(|e| consentis.contains(e))
    }
}

async fn publications_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    gestion: bool,
) -> Result<Vec<PublicationVue>, EntDbError> {
    let pubs: Vec<PubLigne> = sqlx::query_as(
        "SELECT id, type, nature_point, texte, aucun_eleve_identifiable, publiee_le, retiree_le, gardee_pour_bilan
           FROM ent_sortie_publication
          WHERE etablissement_id = $1 AND sortie_id = $2 AND ($3 OR retiree_le IS NULL)
          ORDER BY publiee_le DESC, id",
    )
    .bind(etab)
    .bind(sortie)
    .bind(gestion)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let ids: Vec<Uuid> = pubs.iter().map(|p| p.id).collect();
    let photos: Vec<PhotoLigne> = sqlx::query_as(
        "SELECT id, publication_id, nom, taille FROM ent_sortie_publication_photo
          WHERE etablissement_id = $1 AND publication_id = ANY($2) ORDER BY cree_le, id",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let photo_ids: Vec<Uuid> = photos.iter().map(|p| p.id).collect();
    let marques: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT photo_id, eleve_id FROM ent_sortie_publication_photo_eleve
          WHERE etablissement_id = $1 AND photo_id = ANY($2)",
    )
    .bind(etab)
    .bind(&photo_ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut par_photo: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (p, e) in &marques {
        par_photo.entry(*p).or_default().push(*e);
    }
    let eleves: Vec<Uuid> = {
        let mut v: Vec<Uuid> = marques.iter().map(|m| m.1).collect();
        v.sort();
        v.dedup();
        v
    };
    let consentis: HashSet<Uuid> = etats_effectifs(conn, etab, &eleves, Some(OBJET_DROIT_IMAGE), None)
        .await?
        .into_iter()
        .filter(|(_, e)| *e == "accepte")
        .map(|(id, _)| id)
        .collect();
    let mut out = Vec::new();
    for p in pubs {
        let mut montrees = Vec::new();
        let mut masquees = 0i64;
        for ph in photos.iter().filter(|x| x.publication_id == p.id) {
            let m = par_photo.get(&ph.id).map(Vec::as_slice).unwrap_or(&[]);
            if photo_montrable(m, p.aucun_eleve_identifiable, &consentis) {
                montrees.push(PhotoVue { id: ph.id, nom: ph.nom.clone(), taille: ph.taille });
            } else {
                masquees += 1;
            }
        }
        out.push(PublicationVue {
            id: p.id,
            type_: p.type_,
            nature_point: p.nature_point,
            texte: p.texte,
            publiee_le: p.publiee_le,
            retiree: gestion.then_some(p.retiree_le.is_some()),
            gardee_pour_bilan: gestion.then_some(p.gardee_pour_bilan),
            photos: montrees,
            photos_masquees: gestion.then_some(masquees),
        });
    }
    Ok(out)
}

/// Peut publier : le porteur, un délégué, ou qui gère la sortie (`voyages.modifier`).
async fn peut_publier(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne, acces: &Acces) -> Result<bool, EntDbError> {
    if acces.gere(Action::VoyagesModifier) {
        return Ok(true);
    }
    let Some(moi) = personne_de(conn, a).await? else {
        return Ok(false);
    };
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_sortie_journal_delegue d
                          JOIN ent_sortie_encadrant e ON e.etablissement_id = d.etablissement_id AND e.id = d.encadrant_id
                         WHERE d.etablissement_id = $1 AND d.sortie_id = $2 AND e.personne_id = $3)",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .bind(moi)
    .fetch_one(conn)
    .await
    .map_err(db)
}

#[derive(FromRow)]
struct JournalLigne {
    ferme_le: Option<DateTime<Utc>>,
}

async fn journal_de(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<Option<JournalLigne>, EntDbError> {
    sqlx::query_as("SELECT ferme_le FROM ent_sortie_journal_voyage WHERE etablissement_id = $1 AND sortie_id = $2")
        .bind(etab)
        .bind(sortie)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

async fn vue_gestion(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    peut: bool,
) -> Result<JournalVue, EntDbError> {
    let j = journal_de(conn, a.etablissement_id, s.id).await?;
    let delegues: Vec<DelegueVue> = sqlx::query_as(
        "SELECT e.id AS encadrant_id, p.nom, COALESCE(p.prenom, '') AS prenom
           FROM ent_sortie_journal_delegue d
           JOIN ent_sortie_encadrant e ON e.etablissement_id = d.etablissement_id AND e.id = d.encadrant_id
           JOIN ent_personne p ON p.etablissement_id = e.etablissement_id AND p.id = e.personne_id
          WHERE d.etablissement_id = $1 AND d.sortie_id = $2 ORDER BY p.nom, p.prenom",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let liens: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_journal_lien
          WHERE etablissement_id = $1 AND sortie_id = $2 AND revoque_le IS NULL AND expire_le > now()",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let publications = publications_de(conn, a.etablissement_id, s.id, true).await?;
    Ok(JournalVue {
        actif: j.as_ref().is_some_and(|x| x.ferme_le.is_none()),
        ferme_le: j.and_then(|x| x.ferme_le),
        delegues,
        publications,
        liens_actifs: liens,
        peut_publier: peut,
    })
}

fn garde_gestion(acces: &Acces) -> Result<(), EntDbError> {
    if acces.gere(Action::VoyagesModifier) {
        Ok(())
    } else if acces.detail() {
        Err(interdit())
    } else {
        Err(introuvable())
    }
}

// ───────────────────────── Gestion du journal ─────────────────────────

impl PgEnt {
    /// VOY-176 (NX-33) : active le journal de voyage (audience = titulaires des participants).
    pub async fn voyage_journal_activer(&self, a: &Acteur<'_>, id: Uuid) -> Result<JournalVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        if !ETATS_PUBLIABLES.contains(&s.etat.as_str()) {
            return Err(refus(RefusMetier::Conflit, "Le journal s'ouvre une fois la sortie autorisée."));
        }
        if journal_de(&mut t, a.etablissement_id, id).await?.is_some() {
            return Err(conflit());
        }
        sqlx::query(
            "INSERT INTO ent_sortie_journal_voyage (id, etablissement_id, sortie_id, active_par_user_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, id, Fait::simple("journal_active")).await?;
        let ev = publier(&mut t, a, "voyages.journal_active", id, json!({})).await?;
        let v = vue_gestion(&mut t, a, &s, true).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// VOY-179 : ferme le journal (révocation immédiate des liens). Les publications restent jusqu'à la purge.
    pub async fn voyage_journal_fermer(&self, a: &Acteur<'_>, id: Uuid) -> Result<JournalVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        let motif = if acces.porteur { "organisateur" } else { "direction" };
        let n = sqlx::query(
            "UPDATE ent_sortie_journal_voyage SET ferme_le = now(), ferme_motif = $3
              WHERE etablissement_id = $1 AND sortie_id = $2 AND ferme_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(motif)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_sortie_journal_lien SET revoque_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND revoque_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, id, Fait::simple("journal_ferme")).await?;
        let v = vue_gestion(&mut t, a, &s, false).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// VOY-178 : délégués de publication, choisis parmi les **encadrants de la sortie** (remplacement).
    pub async fn voyage_journal_delegues_definir(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        encadrants: Vec<Uuid>,
    ) -> Result<JournalVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        let mut ids = encadrants;
        ids.sort();
        ids.dedup();
        let ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_sortie_encadrant WHERE etablissement_id = $1 AND sortie_id = $2 AND id = ANY($3)",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&ids)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if ok as usize != ids.len() {
            return Err(refus(RefusMetier::Invalide, "Un délégué doit être un encadrant de la sortie."));
        }
        sqlx::query("DELETE FROM ent_sortie_journal_delegue WHERE etablissement_id = $1 AND sortie_id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        for e in &ids {
            sqlx::query(
                "INSERT INTO ent_sortie_journal_delegue (id, etablissement_id, sortie_id, encadrant_id, cree_par_user_id)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(id)
            .bind(e)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        journaliser(&mut t, a, id, Fait::simple("journal_delegues_modifies")).await?;
        let v = vue_gestion(&mut t, a, &s, true).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Lecture de gestion : l'organisateur, les délégués et ceux qui gèrent la sortie.
    pub async fn voyage_journal_lire(&self, a: &Acteur<'_>, id: Uuid) -> Result<JournalVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let peut = peut_publier(&mut t, a, &s, &acces).await?;
        if !peut {
            return Err(introuvable());
        }
        let v = vue_gestion(&mut t, a, &s, true).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// VOY-177 (NX-33) : publie dans le journal. Auteur : porteur, délégué ou gestionnaire de la sortie.
    /// Une photo sans élève marqué exige la déclaration « aucun élève identifiable ».
    pub async fn voyage_journal_publier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        saisie: PublicationSaisie,
    ) -> Result<PublicationVue, EntDbError> {
        let point = saisie.type_ == "point_de_situation";
        if !matches!(saisie.type_.as_str(), "message" | "point_de_situation")
            || point != saisie.nature_point.is_some()
            || saisie.nature_point.as_deref().is_some_and(|n| !matches!(n, "arrivee" | "depart" | "changement_programme"))
            || saisie.texte.chars().count() > if point { 300 } else { 4000 }
            || saisie.photos.len() > PHOTOS_MAX
            || (saisie.texte.trim().is_empty() && saisie.photos.is_empty() && !point)
        {
            return Err(invalide());
        }
        if point && !saisie.photos.is_empty() {
            return Err(refus(RefusMetier::Invalide, "Un point de situation ne porte pas de photo."));
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_publier(&mut t, a, &s, &acces).await? {
            return Err(introuvable());
        }
        if !ETATS_PUBLIABLES.contains(&s.etat.as_str()) {
            return Err(refus(RefusMetier::Conflit, "Le journal ne reçoit de publication qu'une fois la sortie autorisée."));
        }
        match journal_de(&mut t, a.etablissement_id, id).await? {
            Some(j) if j.ferme_le.is_none() => {}
            _ => return Err(refus(RefusMetier::Conflit, "Le journal de voyage n'est pas ouvert.")),
        }
        // Validation des photos et des élèves marqués AVANT toute écriture.
        let mut valides = Vec::new();
        for p in &saisie.photos {
            if p.eleves.len() > ELEVES_PAR_PHOTO_MAX {
                return Err(invalide());
            }
            if p.eleves.is_empty() && !saisie.aucun_eleve_identifiable {
                return Err(refus(
                    RefusMetier::Invalide,
                    "Marquez les élèves visibles sur la photo, ou déclarez qu'aucun élève n'est identifiable.",
                ));
            }
            if !p.eleves.is_empty() && saisie.aucun_eleve_identifiable {
                return Err(refus(RefusMetier::Invalide, "Une photo marquée ne peut pas être déclarée sans élève identifiable."));
            }
            let mut marques = p.eleves.clone();
            marques.sort();
            marques.dedup();
            let ok: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_sortie_participant
                  WHERE etablissement_id = $1 AND sortie_id = $2 AND eleve_id = ANY($3)
                    AND etat NOT IN ('propose', 'liste_attente', 'desiste', 'retire')",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(&marques)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if ok as usize != marques.len() {
                return Err(refus(RefusMetier::Invalide, "Un élève marqué ne participe pas à la sortie."));
            }
            let v = self
                .politique_fichiers()
                .valider(UsageFichier::Photo, &p.nom, p.mime.as_deref(), &p.contenu)
                .map_err(|_| refus(RefusMetier::Invalide, "Photo refusée (type, taille ou contenu)."))?;
            valides.push((v.extension, v.mime, ligne_propre(&v.nom, 200), marques, p.contenu.clone()));
        }
        let pid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_publication
                 (id, etablissement_id, sortie_id, auteur_user_id, type, nature_point, texte, aucun_eleve_identifiable)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(pid)
        .bind(a.etablissement_id)
        .bind(id)
        .bind(a.user_id)
        .bind(&saisie.type_)
        .bind(&saisie.nature_point)
        .bind(saisie.texte.trim())
        .bind(saisie.aucun_eleve_identifiable)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        for (ext, mime, nom, marques, octets) in valides {
            let ph = Uuid::new_v4();
            let cle = ObjectStore::key_for(a.etablissement_id, "journal_voyage", ph, ext);
            let info = self.objet_put(&cle, &octets).await?;
            crate::objets::enregistrer_objet(&mut *t, a.etablissement_id, &info, Some(mime)).await?;
            sqlx::query(
                "INSERT INTO ent_sortie_publication_photo (id, etablissement_id, publication_id, cle_objet, nom, mime, taille, sha256)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(ph)
            .bind(a.etablissement_id)
            .bind(pid)
            .bind(&cle)
            .bind(&nom)
            .bind(mime)
            .bind(octets.len() as i32)
            .bind(&info.sha256)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            for e in marques {
                sqlx::query(
                    "INSERT INTO ent_sortie_publication_photo_eleve (id, etablissement_id, photo_id, eleve_id) VALUES ($1,$2,$3,$4)",
                )
                .bind(Uuid::new_v4())
                .bind(a.etablissement_id)
                .bind(ph)
                .bind(e)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            }
        }
        journaliser(&mut t, a, id, Fait { champ: Some(if point { "point_de_situation" } else { "message" }), ..Fait::simple("journal_publication") })
            .await?;
        let ev = publier(&mut t, a, "voyages.publication", id, json!({ "type": saisie.type_ })).await?;
        let mut v = publications_de(&mut t, a.etablissement_id, id, true).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        v.retain(|x| x.id == pid);
        v.pop().ok_or_else(introuvable)
    }

    /// VOY-179 : retire une publication (auteur ou gestionnaire).
    pub async fn voyage_journal_publication_retirer(&self, a: &Acteur<'_>, id: Uuid, publication: Uuid) -> Result<(), EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let auteur: Option<Uuid> = sqlx::query_scalar(
            "SELECT auteur_user_id FROM ent_sortie_publication WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(publication)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let Some(auteur) = auteur else {
            return Err(introuvable());
        };
        if !(acces.gere(Action::VoyagesModifier) || (auteur == a.user_id && peut_publier(&mut t, a, &s, &acces).await?)) {
            return Err(introuvable());
        }
        let n = sqlx::query(
            "UPDATE ent_sortie_publication SET retiree_le = now(), retiree_par_user_id = $4
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 AND retiree_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(publication)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit());
        }
        journaliser(&mut t, a, id, Fait::simple("journal_publication_retiree")).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// Garde (ou ne garde plus) une publication pour le bilan pédagogique : **aucun élève non consenti**.
    pub async fn voyage_journal_publication_garder(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        publication: Uuid,
        garder: bool,
    ) -> Result<(), EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        garde_gestion(&acces)?;
        if garder {
            let v = publications_de(&mut t, a.etablissement_id, id, true).await?;
            let p = v.iter().find(|p| p.id == publication && p.retiree == Some(false)).ok_or_else(introuvable)?;
            if p.photos_masquees.unwrap_or(0) > 0 {
                return Err(refus(
                    RefusMetier::Conflit,
                    "Une photo de cette publication montre un élève sans consentement au droit à l'image.",
                ));
            }
        }
        let n = sqlx::query(
            "UPDATE ent_sortie_publication SET gardee_pour_bilan = $4
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(publication)
        .bind(garder)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        t.commit().await.map_err(db)?;
        Ok(())
    }
}

// ───────────────────────── Lecture par les familles ─────────────────────────

const AUDIENCE_COMPTE: &str = "
    EXISTS (SELECT 1 FROM ent_eleve_lien l
             WHERE l.etablissement_id = sp.etablissement_id AND l.eleve_id = sp.eleve_id AND l.user_id = $2
               AND l.suspendu_le IS NULL
               AND (l.date_debut IS NULL OR l.date_debut <= ent_aujourdhui_etab($1))
               AND (l.date_fin IS NULL OR l.date_fin >= ent_aujourdhui_etab($1))
               AND l.autorite_parentale)
 OR EXISTS (SELECT 1 FROM ent_eleve_compte c
             WHERE c.etablissement_id = sp.etablissement_id AND c.eleve_id = sp.eleve_id AND c.user_id = $2)";

impl PgEnt {
    /// Journaux de voyage visibles de l'appelant (titulaire ou élève d'un participant qui part encore).
    pub async fn voyages_journal_famille(&self, a: &Acteur<'_>) -> Result<Vec<JournalFamille>, EntDbError> {
        let mut t = tx(self, a).await?;
        let sql = format!(
            "SELECT DISTINCT s.id, s.titre, s.debut, s.fin
               FROM ent_sortie_participant sp
               JOIN ent_sortie s ON s.etablissement_id = sp.etablissement_id AND s.id = sp.sortie_id
               JOIN ent_sortie_journal_voyage j ON j.etablissement_id = s.etablissement_id AND j.sortie_id = s.id
              WHERE sp.etablissement_id = $1 AND j.ferme_le IS NULL
                AND sp.etat NOT IN ('propose', 'liste_attente', 'desiste', 'retire', 'exclu_disciplinaire')
                AND ({AUDIENCE_COMPTE})
              ORDER BY s.debut"
        );
        let sorties: Vec<(Uuid, String, DateTime<Utc>, DateTime<Utc>)> =
            sqlx::query_as(&sql).bind(a.etablissement_id).bind(a.user_id).fetch_all(&mut *t).await.map_err(db)?;
        let mut out = Vec::new();
        for (sid, titre, debut, fin) in sorties {
            let publications = publications_de(&mut t, a.etablissement_id, sid, false).await?;
            out.push(JournalFamille { sortie_id: sid, titre, debut, fin, auteur: "L'équipe du voyage", publications });
        }
        t.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Photo d'une publication, pour la famille (compte) ou la gestion : mêmes contrôles que la lecture.
    pub async fn voyage_journal_photo(
        &self,
        a: &Acteur<'_>,
        sortie: Uuid,
        photo: Uuid,
    ) -> Result<(String, Vec<u8>), EntDbError> {
        let mut t = tx(self, a).await?;
        let gestion = match charger_avec_acces(self, &mut t, a, sortie, false).await {
            Ok((s, acces)) => peut_publier(&mut t, a, &s, &acces).await?,
            Err(_) => false,
        };
        if !gestion {
            let sql = format!(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_participant sp
                                  JOIN ent_sortie_journal_voyage j ON j.etablissement_id = sp.etablissement_id AND j.sortie_id = sp.sortie_id
                                 WHERE sp.etablissement_id = $1 AND sp.sortie_id = $3 AND j.ferme_le IS NULL
                                   AND sp.etat NOT IN ('propose', 'liste_attente', 'desiste', 'retire', 'exclu_disciplinaire')
                                   AND ({AUDIENCE_COMPTE}))"
            );
            let ok: bool = sqlx::query_scalar(&sql)
                .bind(a.etablissement_id)
                .bind(a.user_id)
                .bind(sortie)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        let r = lire_photo(self, &mut t, a.etablissement_id, sortie, photo, gestion).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }
}

async fn lire_photo(
    e: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    photo: Uuid,
    gestion: bool,
) -> Result<(String, Vec<u8>), EntDbError> {
    // La photo doit appartenir à une publication visible de CETTE sortie (jamais d'accès par identifiant seul).
    let visibles = publications_de(conn, etab, sortie, gestion).await?;
    let montree = visibles
        .iter()
        .filter(|p| p.retiree != Some(true))
        .any(|p| p.photos.iter().any(|x| x.id == photo));
    if !montree {
        return Err(introuvable());
    }
    let (cle, mime, sha): (String, String, String) = sqlx::query_as(
        "SELECT cle_objet, mime, sha256 FROM ent_sortie_publication_photo WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(photo)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let octets = e.lire_objet_empreinte(etab, &cle, Some(&sha)).await?;
    Ok((mime, octets))
}

// ───────────────────────── Liens nominatifs (familles sans compte) ─────────────────────────

fn empreinte_jeton(jeton: &str) -> String {
    ent_object::sha256_hex(jeton.as_bytes())
}

/// Jeton : `<établissement>.<secret>` — l'établissement n'est pas un secret, il oriente seulement la recherche.
fn nouveau_jeton(etab: Uuid) -> String {
    format!("{}.{}{}", etab.simple(), Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn etab_du_jeton(jeton: &str) -> Option<Uuid> {
    let (p, secret) = jeton.split_once('.')?;
    if p.len() != 32 || secret.len() != 64 || !secret.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Uuid::parse_str(p).ok()
}

struct LienLu {
    id: Uuid,
    sortie_id: Uuid,
}

async fn lien_valide(conn: &mut PgConnection, etab: Uuid, jeton: &str) -> Result<LienLu, EntDbError> {
    let row: Option<(Uuid, Uuid, i32, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT l.id, l.sortie_id, l.fenetre_nombre, l.fenetre_debut
           FROM ent_sortie_journal_lien l
           JOIN ent_sortie_participant sp ON sp.etablissement_id = l.etablissement_id AND sp.id = l.participant_id
           JOIN ent_sortie_journal_voyage j ON j.etablissement_id = l.etablissement_id AND j.sortie_id = l.sortie_id
          WHERE l.etablissement_id = $1 AND l.jeton_hash = $2 AND l.revoque_le IS NULL AND l.expire_le > now()
            AND j.ferme_le IS NULL
            AND sp.etat NOT IN ('propose', 'liste_attente', 'desiste', 'retire', 'exclu_disciplinaire')
            AND EXISTS (SELECT 1 FROM ent_eleve_lien_sollicitable k
                         WHERE k.etablissement_id = l.etablissement_id AND k.eleve_id = sp.eleve_id
                           AND k.personne_id = l.personne_id AND k.autorite_parentale)
          FOR UPDATE OF l",
    )
    .bind(etab)
    .bind(empreinte_jeton(jeton))
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((id, sortie_id, n, debut)) = row else {
        return Err(introuvable());
    };
    let recent = debut.is_some_and(|d| (Utc::now() - d).num_seconds() < 600);
    if recent && n >= LIEN_LECTURES_MAX {
        return Err(refus(RefusMetier::Interdit, "Trop de lectures : réessayez dans quelques minutes."));
    }
    sqlx::query(
        "UPDATE ent_sortie_journal_lien
            SET dernier_acces_le = now(),
                fenetre_debut = CASE WHEN $3 THEN fenetre_debut ELSE now() END,
                fenetre_nombre = CASE WHEN $3 THEN fenetre_nombre + 1 ELSE 1 END
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .bind(recent)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(LienLu { id, sortie_id })
}

impl PgEnt {
    /// Émet un lien nominatif par (participant, titulaire **sans compte**) pour la sortie, et le remet au
    /// port de distribution. Un lien actif existant n'est jamais doublé ; un titulaire sans autorité
    /// parentale, suspendu ou qu'une interdiction prive de messagerie n'en reçoit pas. Retourne le nombre
    /// de liens émis. Le jeton n'est jamais renvoyé à l'appelant ni conservé.
    pub async fn voyage_journal_liens_emettre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        port: &dyn DistributionLien,
    ) -> Result<usize, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        match journal_de(&mut t, a.etablissement_id, id).await? {
            Some(j) if j.ferme_le.is_none() => {}
            _ => return Err(refus(RefusMetier::Conflit, "Le journal de voyage n'est pas ouvert.")),
        }
        let cibles: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT sp.id, k.personne_id
               FROM ent_sortie_participant sp
               JOIN ent_eleve_lien_sollicitable k ON k.etablissement_id = sp.etablissement_id AND k.eleve_id = sp.eleve_id
              WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2
                AND sp.etat NOT IN ('propose', 'liste_attente', 'desiste', 'retire', 'exclu_disciplinaire')
                AND k.autorite_parentale AND k.user_id IS NULL
                AND NOT EXISTS (SELECT 1 FROM ent_sortie_journal_lien x
                                 WHERE x.etablissement_id = sp.etablissement_id AND x.participant_id = sp.id
                                   AND x.personne_id = k.personne_id AND x.revoque_le IS NULL AND x.expire_le > now())",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut jetons: Vec<(Uuid, String)> = Vec::new();
        for (participant, personne) in cibles {
            let jeton = nouveau_jeton(a.etablissement_id);
            sqlx::query(
                "INSERT INTO ent_sortie_journal_lien
                     (id, etablissement_id, sortie_id, participant_id, personne_id, jeton_hash, expire_le, cree_par_user_id)
                 VALUES ($1,$2,$3,$4,$5,$6, $7::timestamptz + interval '15 days', $8)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(id)
            .bind(participant)
            .bind(personne)
            .bind(empreinte_jeton(&jeton))
            .bind(s.fin)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            jetons.push((personne, jeton));
        }
        journaliser(&mut t, a, id, Fait { champ: Some("liens"), apres: Some(jetons.len().to_string()), ..Fait::simple("journal_liens_emis") })
            .await?;
        t.commit().await.map_err(db)?;
        for (personne, jeton) in &jetons {
            port.distribuer(a.etablissement_id, id, *personne, jeton);
        }
        Ok(jetons.len())
    }

    /// Révoque tous les liens actifs d'un titulaire (ou de la sortie entière si `personne` est absent).
    pub async fn voyage_journal_liens_revoquer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        personne: Option<Uuid>,
    ) -> Result<u64, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        let n = sqlx::query(
            "UPDATE ent_sortie_journal_lien SET revoque_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND revoque_le IS NULL
                AND ($3::uuid IS NULL OR personne_id = $3)",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(personne)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        journaliser(&mut t, a, id, Fait::simple("journal_liens_revoques")).await?;
        t.commit().await.map_err(db)?;
        Ok(n)
    }

    /// Lecture par lien nominatif (route publique, `noindex` à la charge de la couche HTTP). Un jeton
    /// inconnu, expiré, révoqué, ou dont le participant ne part plus, répond « introuvable ».
    pub async fn voyage_journal_par_lien(&self, jeton: &str) -> Result<JournalFamille, EntDbError> {
        let etab = etab_du_jeton(jeton).ok_or_else(introuvable)?;
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lien = lien_valide(&mut t, etab, jeton).await?;
        let (titre, debut, fin): (String, DateTime<Utc>, DateTime<Utc>) =
            sqlx::query_as("SELECT titre, debut, fin FROM ent_sortie WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(lien.sortie_id)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let publications = publications_de(&mut t, etab, lien.sortie_id, false).await?;
        t.commit().await.map_err(db)?;
        let _ = lien.id;
        Ok(JournalFamille { sortie_id: lien.sortie_id, titre, debut, fin, auteur: "L'équipe du voyage", publications })
    }

    /// Photo par lien nominatif : mêmes contrôles que la lecture, plus le consentement de chaque élève.
    pub async fn voyage_journal_photo_par_lien(&self, jeton: &str, photo: Uuid) -> Result<(String, Vec<u8>), EntDbError> {
        let etab = etab_du_jeton(jeton).ok_or_else(introuvable)?;
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lien = lien_valide(&mut t, etab, jeton).await?;
        let r = lire_photo(self, &mut t, etab, lien.sortie_id, photo, false).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Purge (VOY-179, §21.3) : à la fin du séjour + 15 jours, publications et photos sont détruites, sauf
    /// la sélection gardée pour le bilan ; les liens sont révoqués et le journal fermé. Appelée une fois par
    /// jour (aucun ordonnanceur branché ; la direction peut la déclencher). Retourne les publications détruites.
    pub async fn voyages_journal_purger(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let sorties: Vec<Uuid> = sqlx::query_scalar(
            "SELECT s.id FROM ent_sortie s
               JOIN ent_sortie_journal_voyage j ON j.etablissement_id = s.etablissement_id AND j.sortie_id = s.id
              WHERE s.etablissement_id = $1 AND s.fin + interval '15 days' <= now()
                AND (j.ferme_le IS NULL
                     OR EXISTS (SELECT 1 FROM ent_sortie_publication p
                                 WHERE p.etablissement_id = s.etablissement_id AND p.sortie_id = s.id
                                   AND NOT p.gardee_pour_bilan))
              FOR UPDATE OF j",
        )
        .bind(etab)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut detruites = 0usize;
        for sid in sorties {
            let cles: Vec<String> = sqlx::query_scalar(
                "SELECT ph.cle_objet FROM ent_sortie_publication_photo ph
                   JOIN ent_sortie_publication p ON p.etablissement_id = ph.etablissement_id AND p.id = ph.publication_id
                  WHERE ph.etablissement_id = $1 AND p.sortie_id = $2 AND NOT p.gardee_pour_bilan",
            )
            .bind(etab)
            .bind(sid)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            sqlx::query(
                "DELETE FROM ent_sortie_publication_photo_eleve WHERE etablissement_id = $1 AND photo_id IN
                   (SELECT ph.id FROM ent_sortie_publication_photo ph
                      JOIN ent_sortie_publication p ON p.etablissement_id = ph.etablissement_id AND p.id = ph.publication_id
                     WHERE ph.etablissement_id = $1 AND p.sortie_id = $2 AND NOT p.gardee_pour_bilan)",
            )
            .bind(etab)
            .bind(sid)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            sqlx::query(
                "DELETE FROM ent_sortie_publication_photo WHERE etablissement_id = $1 AND publication_id IN
                   (SELECT id FROM ent_sortie_publication WHERE etablissement_id = $1 AND sortie_id = $2 AND NOT gardee_pour_bilan)",
            )
            .bind(etab)
            .bind(sid)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            if !cles.is_empty() {
                // Suppression logique au registre des objets (jamais de DELETE) ; la destruction physique
                // est celle du socle 27.
                sqlx::query("UPDATE ent_objet SET supprime_le = now() WHERE etablissement_id = $1 AND cle_objet = ANY($2) AND supprime_le IS NULL")
                    .bind(etab)
                    .bind(&cles)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
            }
            detruites += sqlx::query("DELETE FROM ent_sortie_publication WHERE etablissement_id = $1 AND sortie_id = $2 AND NOT gardee_pour_bilan")
                .bind(etab)
                .bind(sid)
                .execute(&mut *t)
                .await
                .map_err(db)?
                .rows_affected() as usize;
            sqlx::query(
                "UPDATE ent_sortie_journal_lien SET revoque_le = now() WHERE etablissement_id = $1 AND sortie_id = $2 AND revoque_le IS NULL",
            )
            .bind(etab)
            .bind(sid)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_sortie_journal_voyage SET ferme_le = now(), ferme_motif = 'purge'
                  WHERE etablissement_id = $1 AND sortie_id = $2 AND ferme_le IS NULL",
            )
            .bind(etab)
            .bind(sid)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_sortie_journal (id, etablissement_id, sortie_id, acteur_user_id, action) VALUES ($1,$2,$3,NULL,'journal_purge')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(sid)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        t.commit().await.map_err(db)?;
        Ok(detruites)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_montrable_selon_consentements_et_marquage() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let consentis: HashSet<Uuid> = [a].into_iter().collect();
        assert!(photo_montrable(&[a], false, &consentis));
        assert!(!photo_montrable(&[a, b], false, &consentis), "un seul non consentant masque la photo");
        assert!(!photo_montrable(&[], false, &consentis), "sans marquage ni déclaration : masquée");
        assert!(photo_montrable(&[], true, &consentis), "déclaration explicite : montrée");
        assert!(!photo_montrable(&[b], true, &consentis));
    }

    #[test]
    fn jeton_oriente_vers_l_etablissement_sans_etre_devinable() {
        let e = Uuid::new_v4();
        let j = nouveau_jeton(e);
        assert_eq!(etab_du_jeton(&j), Some(e));
        assert_ne!(nouveau_jeton(e), j);
        assert_eq!(etab_du_jeton("abc"), None);
        assert_eq!(etab_du_jeton(&format!("{}.zz", e.simple())), None);
        assert_ne!(empreinte_jeton(&j), j);
    }
}
