//! Socle 20, lot C5 : **rendu et retour**, côté élève et famille (`20_CAHIER_DE_TEXTES.md` §10.1, §10.3, §10.4).
//!
//! * Un dépôt est **immuable** (déclencheur d'ajout seul) ; un redépôt est un nouveau dépôt ; l'heure retenue est celle du
//!   **début** de réception par le serveur (R-C10, CC-31) ; « en retard » se calcule sur l'échéance effective de l'élève
//!   (la prolongation individuelle la plus récente fait foi), puis ne change plus (CC-40) sauf prolongation accordée après
//!   coup (CC-41, recalcul tracé par l'ajout seul de la prolongation).
//! * Les fichiers passent par le socle 7 : type réel, taille, liste blanche, objet empreinte dans `ent_objet`. **Aucun nom
//!   de fichier n'est stocké** : la pièce s'appelle `copie-N.ext` ; le nom « NOM Prénom – titre » d'une archive se fabrique à
//!   la lecture. Lien court après contrôle des droits (R-C13).
//! * Une copie n'est lue que de son auteur (et des membres de son groupe), de ses responsables habilités pour la **copie
//!   annotée rendue** seulement (R-C37, jamais les dépôts), des intervenants en saisie à la date ; la direction sur motif
//!   journalisé ; personne d'autre.
//! * Le motif d'une prolongation n'est jamais servi : « délai accordé » seulement. La coche n'est jamais lue ici (R-C14).

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::Role;
use ent_events::types;
use ent_object::presign::{Disposition, LinkSpec, PresignError};
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use ent_types::cahier_c5::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::{publier_cdt, FichierDepot};
use crate::cahier_c2::{conflit, invalide, refus};
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, LienFichier, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Dépôt ou brouillon reçu de l'API (fichiers déjà décodés : le décodage base64 est fait par la couche HTTP).
#[derive(Debug, Default)]
pub struct DepotEntree {
    pub texte: Option<String>,
    pub commentaire: Option<String>,
    pub fichiers: Vec<FichierDepot>,
}

/// Plafond de pièces par dépôt (brouillon compris).
pub(crate) const MAX_PIECES_DEPOT: i64 = 10;

#[derive(Debug, Clone, FromRow)]
pub(crate) struct TravailC5 {
    pub id: Uuid,
    pub cours_id: Uuid,
    pub annee_id: Uuid,
    pub titre: String,
    pub etat: String,
    pub visible_a: Option<DateTime<Utc>>,
    pub rendu: String,
    pub retard_accepte: bool,
    pub echeance_le: DateTime<Utc>,
    pub cloture_ramassage_le: Option<DateTime<Utc>>,
    pub remise_par_groupe: bool,
    pub inscription_groupes_jusqu_au: Option<DateTime<Utc>>,
    pub taille_groupe: Option<i16>,
    pub public: String,
}

const SELECT_TRAVAIL_C5: &str = "SELECT id, cours_id, annee_id, titre, etat, visible_a, rendu, retard_accepte, echeance_le,
        cloture_ramassage_le, remise_par_groupe, inscription_groupes_jusqu_au, taille_groupe, public
   FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2";

#[derive(Debug, Clone, FromRow)]
pub(crate) struct RemiseRowC5 {
    pub id: Uuid,
    pub travail_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub etat: String,
    pub dernier_depot_id: Option<Uuid>,
    pub en_retard: bool,
    pub brouillon_texte: Option<String>,
    pub brouillon_commentaire: Option<String>,
    pub brouillon_le: Option<DateTime<Utc>>,
}

pub(crate) const SELECT_REMISE_C5: &str = "SELECT id, travail_id, eleve_id, groupe_id, etat, dernier_depot_id, en_retard,
        brouillon_texte, brouillon_commentaire, brouillon_le
   FROM ent_cdt_remise WHERE etablissement_id = $1";

/// Fichier déjà contrôlé et rangé dans le magasin (étape qui précède la transaction du dépôt).
pub(crate) struct FichierPret {
    pub objet_id: Uuid,
    pub extension: &'static str,
    pub mime: &'static str,
    pub taille: i64,
    pub sha256: String,
}

pub(crate) async fn charger_travail(tx: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<TravailC5, EntDbError> {
    sqlx::query_as::<_, TravailC5>(SELECT_TRAVAIL_C5)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Échéance effective d'un élève et clôture du ramassage (S20-09 : par défaut 14 jours après l'échéance effective).
pub(crate) async fn echeances_de(
    tx: &mut PgConnection,
    etab: Uuid,
    t: &TravailC5,
    eleve: Uuid,
) -> Result<(DateTime<Utc>, DateTime<Utc>, bool), EntDbError> {
    let (effective, cloture, prolonge): (DateTime<Utc>, DateTime<Utc>, bool) = sqlx::query_as(
        "SELECT e.effective,
                COALESCE($4, e.effective + make_interval(days => COALESCE(
                    (SELECT r.cloture_ramassage_jours FROM ent_cdt_reglage r
                      WHERE r.etablissement_id = $1 AND r.annee_id = $5), 14))),
                e.effective <> $6
           FROM (SELECT ent_cdt_echeance_effective($1, $2, $3, $6) AS effective) e",
    )
    .bind(etab)
    .bind(t.id)
    .bind(eleve)
    .bind(t.cloture_ramassage_le)
    .bind(t.annee_id)
    .bind(t.echeance_le)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    Ok((effective, cloture, prolonge))
}

/// Jour et heure de l'établissement d'un instant.
pub(crate) async fn jour_heure_locaux(
    tx: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<(NaiveDate, NaiveTime), EntDbError> {
    sqlx::query_as(
        "SELECT (x.l)::date, (x.l)::time
           FROM (SELECT $2::timestamptz AT TIME ZONE COALESCE(
                    (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1), 'Europe/Paris') AS l) x",
    )
    .bind(etab)
    .bind(instant)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)
}

/// Instant d'un jour et d'une heure de l'établissement.
pub(crate) async fn instant_local(
    tx: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    heure: NaiveTime,
) -> Result<DateTime<Utc>, EntDbError> {
    let (i,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_instant_local($1, $2, $3)")
        .bind(etab)
        .bind(jour)
        .bind(heure)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
    Ok(i)
}

/// L'élève est-il dans le public du travail (cours à la date du jour, ou liste restreinte) ?
pub(crate) async fn eleve_dans_public(
    tx: &mut PgConnection,
    etab: Uuid,
    t: &TravailC5,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $3, $2, NULL, $4, false))
            AND ($5 = 'cours' OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k
                                          WHERE k.etablissement_id = $1 AND k.travail_id = $6 AND k.eleve_id = $4))",
    )
    .bind(etab)
    .bind(t.cours_id)
    .bind(jour)
    .bind(eleve)
    .bind(&t.public)
    .bind(t.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    Ok(ok)
}

fn travail_visible(t: &TravailC5) -> bool {
    t.etat == "publie" && t.visible_a.is_some_and(|v| v <= Utc::now())
}

/// Texte, jamais vide après nettoyage, borné.
fn texte_optionnel(t: &Option<String>, max: usize, message: &'static str) -> Result<Option<String>, EntDbError> {
    match t.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() > max => Err(invalide(message)),
        Some(s) => Ok(Some(s.to_string())),
    }
}

/// Empreinte d'un dépôt : texte et empreintes des fichiers, dans l'ordre (R-C9).
fn empreinte_depot(texte: Option<&str>, fichiers: &[String]) -> String {
    let mut h = Sha256::new();
    h.update(texte.unwrap_or("").as_bytes());
    for f in fichiers {
        h.update(b"\n");
        h.update(f.as_bytes());
    }
    format!("{:x}", h.finalize())
}

impl PgEnt {
    /// L'acteur lit la situation de `eleve` : lui-même, ou son enfant (responsable habilité, mineur). Contrairement à
    /// l'agenda, un élève **parti** garde la lecture de ses propres remises (R-C6, CC-35) ; il ne peut plus déposer.
    pub(crate) async fn cdt_proprietaire_remise(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
    ) -> Result<(Uuid, bool, bool), EntDbError> {
        let etab = a.etablissement_id;
        let (eleve, famille) = match Role::parse(a.role) {
            Some(Role::Eleve) => {
                let propre = self.eleve_du_compte(a).await?.ok_or(EntDbError::Forbidden)?;
                if eleve_id.is_some_and(|e| e != propre) {
                    return Err(EntDbError::Forbidden);
                }
                (propre, false)
            }
            Some(Role::Parent) => {
                let eleve = eleve_id.ok_or_else(|| invalide("indiquez l'élève"))?;
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif l
                        JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
                       WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3 AND m.role = 'parent'
                         AND l.autorite_parentale)
                     AND NOT ent_cdt_eleve_majeur($1, $3)",
                )
                .bind(etab)
                .bind(a.user_id)
                .bind(eleve)
                .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await
                .map_err(db)?;
                if !ok {
                    return Err(EntDbError::Forbidden);
                }
                (eleve, true)
            }
            _ => return Err(EntDbError::Forbidden),
        };
        let (present,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.etablissement_id = $1 AND e.id = $2
                             AND (e.date_sortie IS NULL OR e.date_sortie > ent_aujourdhui_etab($1)))",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok((eleve, famille, !present))
    }

    /// Le responsable voit-il les notes de l'enfant (droit « voir les notes » du lien) ?
    pub(crate) async fn cdt_parent_voit_notes(&self, a: &Acteur<'_>, eleve: Uuid) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif l
               WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3
                 AND l.autorite_parentale AND l.peut_voir_notes)",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(eleve)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Contrôle (type réel, taille, liste blanche) puis rangement dans le magasin d'objets : **avant** la transaction du
    /// dépôt (un fichier refusé ne laisse aucun dépôt, CC-32). Le nom d'origine n'est jamais retenu.
    pub(crate) async fn cdt_preparer_fichiers(
        &self,
        etab: Uuid,
        fichiers: &[FichierDepot],
        formats: &[String],
    ) -> Result<Vec<FichierPret>, EntDbError> {
        let mut prets = Vec::new();
        for f in fichiers {
            let octets = &f.contenu;
            let valide = self
                .politique_fichiers()
                .valider(UsageFichier::Copie, &f.nom, Some(&f.mime_type), octets)
                .map_err(EntDbError::Fichier)?;
            if !formats.is_empty() && !formats.iter().any(|x| x == valide.extension) {
                return Err(invalide("format de fichier non accepté pour ce travail"));
            }
            let cle = ObjectStore::key_for(etab, "cahier-copie", Uuid::new_v4(), valide.extension);
            let info = self.stocker_objet(etab, &cle, Some(valide.mime), octets).await?;
            let (objet_id,): (Uuid,) =
                sqlx::query_as("SELECT id FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2")
                    .bind(etab)
                    .bind(&cle)
                    .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                    .await
                    .map_err(db)?;
            prets.push(FichierPret {
                objet_id,
                extension: valide.extension,
                mime: valide.mime,
                taille: octets.len() as i64,
                sha256: info.sha256,
            });
        }
        Ok(prets)
    }

    /// Formats acceptés d'un travail (extensions).
    pub(crate) async fn cdt_formats(&self, tx: &mut PgConnection, etab: Uuid, travail: Uuid) -> Result<Vec<String>, EntDbError> {
        sqlx::query_scalar("SELECT format_code FROM ent_cdt_travail_format WHERE etablissement_id = $1 AND travail_id = $2 ORDER BY 1")
            .bind(etab)
            .bind(travail)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)
    }

    /// Groupe actif de l'élève pour ce travail, avec membres et complétude.
    async fn groupe_c5(&self, tx: &mut PgConnection, etab: Uuid, t: &TravailC5, eleve: Uuid) -> Result<Option<GroupeDto>, EntDbError> {
        let g: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT g.id, g.libelle FROM ent_cdt_groupe g
               JOIN ent_cdt_groupe_membre m ON m.etablissement_id = g.etablissement_id AND m.groupe_id = g.id
              WHERE g.etablissement_id = $1 AND g.travail_id = $2 AND m.eleve_id = $3 AND m.jusqu_au IS NULL",
        )
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match g {
            None => Ok(None),
            Some((id, libelle)) => Ok(Some(self.groupe_dto(tx, etab, t, id, libelle).await?)),
        }
    }

    async fn groupe_dto(&self, tx: &mut PgConnection, etab: Uuid, t: &TravailC5, id: Uuid, libelle: String) -> Result<GroupeDto, EntDbError> {
        let membres: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom FROM ent_cdt_groupe_membre m
               JOIN ent_eleve e ON e.etablissement_id = m.etablissement_id AND e.id = m.eleve_id
              WHERE m.etablissement_id = $1 AND m.groupe_id = $2 AND m.jusqu_au IS NULL
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let complet = t.taille_groupe.is_some_and(|n| membres.len() as i64 >= i64::from(n));
        Ok(GroupeDto {
            id,
            libelle,
            membres: membres
                .into_iter()
                .map(|(eleve_id, nom, prenom)| MembreGroupeDto { eleve_id, nom, prenom })
                .collect(),
            complet,
        })
    }

    /// La remise de l'élève (individuelle ou de son groupe), si elle existe.
    pub(crate) async fn cdt_remise_de(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        t: &TravailC5,
        eleve: Uuid,
    ) -> Result<Option<RemiseRowC5>, EntDbError> {
        sqlx::query_as::<_, RemiseRowC5>(&format!(
            "{SELECT_REMISE_C5} AND travail_id = $2
               AND (eleve_id = $3 OR groupe_id IN (SELECT m.groupe_id FROM ent_cdt_groupe_membre m
                                                    WHERE m.etablissement_id = $1 AND m.eleve_id = $3 AND m.jusqu_au IS NULL))
             ORDER BY (eleve_id IS NULL) LIMIT 1"
        ))
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)
    }

    async fn pieces_c5(&self, tx: &mut PgConnection, etab: Uuid, porteur: &str, id: Uuid) -> Result<Vec<PieceC5Dto>, EntDbError> {
        let lignes: Vec<(Uuid, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT p.id, p.nom, o.mime, o.taille FROM ent_cdt_piece_jointe p
               JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.id = p.objet_id
              WHERE p.etablissement_id = $1 AND p.porteur = $2 AND p.porteur_id = $3 AND p.retiree_le IS NULL
              ORDER BY p.ordre, p.cree_le, p.id",
        )
        .bind(etab)
        .bind(porteur)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|(id, nom, mime, taille)| PieceC5Dto { id, nom, mime, taille })
            .collect())
    }

    async fn accuses_de(&self, tx: &mut PgConnection, etab: Uuid, r: &RemiseRowC5) -> Result<Vec<AccuseDto>, EntDbError> {
        #[derive(FromRow)]
        struct Row {
            id: Uuid,
            numero: i32,
            recu_le: DateTime<Utc>,
            au_nom_de: bool,
            par_user_id: Option<Uuid>,
            texte: Option<String>,
            empreinte: String,
            nb_fichiers: i16,
            commentaire_eleve: Option<String>,
        }
        let lignes: Vec<Row> = sqlx::query_as(
            "SELECT id, numero, recu_le, au_nom_de, par_user_id, texte, empreinte::text AS empreinte, nb_fichiers, commentaire_eleve
               FROM ent_cdt_depot WHERE etablissement_id = $1 AND remise_id = $2 ORDER BY numero DESC",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for l in lignes {
            let (jour, heure) = jour_heure_locaux(tx, etab, l.recu_le).await?;
            let pieces = self.pieces_c5(tx, etab, "depot", l.id).await?;
            let depose_par = if l.au_nom_de {
                match l.par_user_id {
                    Some(u) => Some(
                        sqlx::query_scalar::<_, String>("SELECT ent_cdt_nom_affichage($1, $2)")
                            .bind(etab)
                            .bind(u)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(db)?,
                    ),
                    None => None,
                }
            } else {
                None
            };
            sortie.push(AccuseDto {
                depot_id: l.id,
                numero: l.numero,
                recu_le: l.recu_le,
                recu_jour: jour,
                recu_heure: heure,
                au_nom_de: l.au_nom_de,
                depose_par,
                nb_fichiers: l.nb_fichiers,
                empreinte: l.empreinte,
                texte: l.texte,
                commentaire_eleve: l.commentaire_eleve,
                pieces,
                courant: r.dernier_depot_id == Some(l.id),
                en_retard: r.dernier_depot_id == Some(l.id) && r.en_retard,
            });
        }
        Ok(sortie)
    }

    /// Retour **rendu** lu par l'élève ou la famille (la note seulement si le professeur l'a choisi, et pour un
    /// responsable si le droit « voir les notes » est ouvert).
    pub(crate) async fn cdt_retour_lu(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        remise: Uuid,
        note_autorisee: bool,
    ) -> Result<Option<RetourLuDto>, EntDbError> {
        #[derive(FromRow)]
        struct Row {
            id: Uuid,
            version: i32,
            rendu_le: Option<DateTime<Utc>>,
            commentaire: String,
            note_id: Option<Uuid>,
            montrer_note: bool,
        }
        let r: Option<Row> = sqlx::query_as(
            "SELECT id, version, rendu_le, commentaire, note_id, montrer_note FROM ent_cdt_retour
              WHERE etablissement_id = $1 AND remise_id = $2 AND etat = 'rendu'",
        )
        .bind(etab)
        .bind(remise)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(r) = r else { return Ok(None) };
        let criteres: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT libelle, niveau FROM ent_cdt_retour_critere WHERE etablissement_id = $1 AND retour_id = $2 ORDER BY ordre",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // V3c (M2) : la note liée se lit dans `eva_resultat_publie` (évaluation PUBLIÉE seulement), **revérifiée pour
        // l'élève de la remise** (`eleve_id`), et jamais par la vue de compatibilité `ent_note` qui rendait 0 pour un
        // absent, un dispensé ou un annulé. Valeur numérique seulement pour le statut `note` ; sinon le STATUT est
        // montré (« Absent », « Dispensé »…), jamais un « 0 / 20 ».
        let (valeur, sur, statut) = match (r.montrer_note && note_autorisee, r.note_id) {
            (true, Some(n)) => {
                let v: Option<(String, Option<f64>, f64)> = sqlx::query_as(
                    "SELECT p.statut, p.valeur::double precision, p.bareme::double precision
                       FROM eva_resultat_publie p
                       JOIN ent_cdt_remise t ON t.etablissement_id = p.etablissement_id AND t.eleve_id = p.eleve_id
                      WHERE p.etablissement_id = $1 AND p.id = $2 AND t.id = $3",
                )
                .bind(etab)
                .bind(n)
                .bind(remise)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                match v {
                    Some((st, Some(val), bareme)) if st == "note" => (Some(val), Some(bareme), None),
                    Some((st, _, _)) if st != "note" => (None, None, Some(st)),
                    _ => (None, None, None),
                }
            }
            _ => (None, None, None),
        };
        let pieces = self.pieces_c5(tx, etab, "retour", r.id).await?;
        Ok(Some(RetourLuDto {
            retour_id: r.id,
            version: r.version,
            modifie: r.version > 1,
            rendu_le: r.rendu_le,
            commentaire: r.commentaire,
            criteres: criteres.into_iter().map(|(libelle, niveau)| CritereDto { libelle, niveau }).collect(),
            note_valeur: valeur,
            note_sur: sur,
            note_statut: statut,
            pieces,
        }))
    }

    /// CDT-06 (lecture), CDT-162 : la situation d'un élève devant un travail à rendre.
    pub async fn cdt_remise_lire(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        eleve_id: Option<Uuid>,
    ) -> Result<RemiseLueDto, EntDbError> {
        let (eleve, famille, parti) = self.cdt_proprietaire_remise(a, eleve_id).await?;
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t = charger_travail(&mut tx, etab, travail_id).await?;
        let remise = self.cdt_remise_de(&mut tx, etab, &t, eleve).await?;
        // Le travail doit exister pour cet élève : publié, visible, dans son public — ou déjà porteur de sa remise
        // (R-C6 : il garde toujours ses propres remises et retours). Sinon : introuvable, jamais « interdit » (pas de sondage).
        let concerne = travail_visible(&t) && eleve_dans_public(&mut tx, etab, &t, eleve, jour).await?;
        if !concerne && remise.is_none() {
            return Err(introuvable());
        }
        if t.rendu == "aucun" && remise.is_none() {
            return Err(conflit("ce travail n'est pas à rendre"));
        }
        let (effective, cloture, prolonge) = echeances_de(&mut tx, etab, &t, eleve).await?;
        let (echeance_jour, echeance_heure) = jour_heure_locaux(&mut tx, etab, effective).await?;
        let maintenant = Utc::now();
        let (peut_deposer, raison) = fermeture(&t, effective, cloture, maintenant, famille, parti, concerne);
        let statut: Option<String> = sqlx::query_scalar("SELECT ent_cdt_statut_eleve($1, $2, $3, $4, $5)")
            .bind(etab)
            .bind(t.id)
            .bind(eleve)
            .bind(effective)
            .bind(&t.rendu)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let formats = self.cdt_formats(&mut tx, etab, t.id).await?;
        let groupe = self.groupe_c5(&mut tx, etab, &t, eleve).await?;
        let note_ok = if famille { self.cdt_parent_voit_notes(a, eleve).await? } else { true };
        let (depots, brouillon, retour, nb_depots) = match &remise {
            None => (vec![], None, None, 0),
            Some(r) => {
                let accuses = if famille { vec![] } else { self.accuses_de(&mut tx, etab, r).await? };
                let nb: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_depot WHERE etablissement_id = $1 AND remise_id = $2")
                    .bind(etab)
                    .bind(r.id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                let brouillon = if famille {
                    None
                } else if r.brouillon_le.is_some() {
                    Some(BrouillonDto {
                        texte: r.brouillon_texte.clone(),
                        commentaire: r.brouillon_commentaire.clone(),
                        pieces: self.pieces_c5(&mut tx, etab, "remise", r.id).await?,
                        enregistre_le: r.brouillon_le,
                    })
                } else {
                    None
                };
                // Un responsable ne lit que la copie annotée rendue (R-C37) : le retour, jamais les dépôts.
                let retour = self.cdt_retour_lu(&mut tx, etab, r.id, note_ok).await?;
                (accuses, brouillon, retour, nb as i32)
            }
        };
        let demande: Option<(Uuid, String, Option<String>, Option<String>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, etat, message, reponse, cree_le FROM ent_cdt_delai_demande
              WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3 ORDER BY cree_le DESC LIMIT 1",
        )
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let signalement: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT max(cree_le) FROM ent_cdt_depot_signalement WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3",
        )
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(RemiseLueDto {
            travail_id: t.id,
            eleve_id: eleve,
            titre: t.titre.clone(),
            rendu: t.rendu.clone(),
            etat: remise.as_ref().map(|r| r.etat.clone()).unwrap_or_else(|| "aucune".into()),
            statut,
            echeance_le: effective,
            echeance_jour,
            echeance_heure,
            delai_accorde: prolonge,
            cloture_ramassage_le: Some(cloture),
            retard_accepte: t.retard_accepte,
            peut_deposer,
            raison_fermeture: raison,
            formats,
            groupe,
            remise_par_groupe: t.remise_par_groupe,
            brouillon,
            depots,
            nb_depots,
            retour,
            demande_delai: demande.map(|(id, etat, message, reponse, cree_le)| DelaiLuDto { id, etat, message, reponse, cree_le }),
            signalement_le: signalement,
        })
    }

    /// CDT-06 (brouillon) : l'élève prépare sa remise ; seul le dépôt compte. Texte, commentaire et fichiers en attente.
    pub async fn cdt_brouillon_enregistrer(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        body: DepotEntree,
    ) -> Result<RemiseLueDto, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let (eleve, famille, parti) = self.cdt_proprietaire_remise(a, None).await?;
        if famille || parti {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let texte = texte_optionnel(&body.texte, 20000, "texte trop long")?;
        let commentaire = texte_optionnel(&body.commentaire, 2000, "commentaire trop long")?;
        let jour = self.jour_etab(etab).await?;
        let (t, formats) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let t = charger_travail(&mut tx, etab, travail_id).await?;
            if !travail_visible(&t) || !eleve_dans_public(&mut tx, etab, &t, eleve, jour).await? {
                return Err(introuvable());
            }
            if t.rendu != "en_ligne" {
                return Err(conflit("ce travail n'est pas à rendre en ligne"));
            }
            let f = self.cdt_formats(&mut tx, etab, t.id).await?;
            (t, f)
        };
        let prets = self.cdt_preparer_fichiers(etab, &body.fichiers, &formats).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = self.cdt_remise_verrouiller(&mut tx, etab, &t, eleve).await?;
        if r.etat == "deposee" {
            return Err(conflit("un dépôt a déjà été fait : redéposez pour le remplacer"));
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_cdt_piece_jointe WHERE etablissement_id = $1 AND porteur = 'remise' AND porteur_id = $2 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n + prets.len() as i64 > MAX_PIECES_DEPOT {
            return Err(conflit("nombre maximal de pièces atteint"));
        }
        sqlx::query(
            "UPDATE ent_cdt_remise SET brouillon_texte = $3, brouillon_commentaire = $4, brouillon_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(r.id)
        .bind(&texte)
        .bind(&commentaire)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        attacher_pieces(&mut tx, etab, "remise", r.id, n as i16, &prets, "copie").await?;
        tx.commit().await.map_err(db)?;
        self.cdt_remise_lire(a, travail_id, None).await
    }

    /// Retire une pièce du brouillon (jamais d'un dépôt : immuable).
    pub async fn cdt_brouillon_retirer_piece(&self, a: &Acteur<'_>, travail_id: Uuid, piece_id: Uuid) -> Result<RemiseLueDto, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let (eleve, famille, parti) = self.cdt_proprietaire_remise(a, None).await?;
        if famille || parti {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t = charger_travail(&mut tx, etab, travail_id).await?;
        let r = self.cdt_remise_de(&mut tx, etab, &t, eleve).await?.ok_or_else(introuvable)?;
        let n = sqlx::query(
            "UPDATE ent_cdt_piece_jointe SET retiree_le = now(), retiree_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2 AND porteur = 'remise' AND porteur_id = $3 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(piece_id)
        .bind(r.id)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        tx.commit().await.map_err(db)?;
        self.cdt_remise_lire(a, travail_id, None).await
    }

    /// Crée (ou retrouve) la ligne de remise de l'élève ou de son groupe et la verrouille.
    pub(crate) async fn cdt_remise_verrouiller(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        t: &TravailC5,
        eleve: Uuid,
    ) -> Result<RemiseRowC5, EntDbError> {
        let groupe: Option<Uuid> = if t.remise_par_groupe {
            sqlx::query_scalar(
                "SELECT g.id FROM ent_cdt_groupe g
                   JOIN ent_cdt_groupe_membre m ON m.etablissement_id = g.etablissement_id AND m.groupe_id = g.id
                  WHERE g.etablissement_id = $1 AND g.travail_id = $2 AND m.eleve_id = $3 AND m.jusqu_au IS NULL",
            )
            .bind(etab)
            .bind(t.id)
            .bind(eleve)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
        } else {
            None
        };
        if t.remise_par_groupe && groupe.is_none() {
            return Err(conflit("ce travail se rend par groupe : rejoignez ou créez un groupe d'abord"));
        }
        if let Some(g) = groupe {
            sqlx::query(
                "INSERT INTO ent_cdt_remise (id, etablissement_id, travail_id, groupe_id) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (etablissement_id, travail_id, groupe_id) WHERE groupe_id IS NOT NULL DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(t.id)
            .bind(g)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query_as::<_, RemiseRowC5>(&format!("{SELECT_REMISE_C5} AND travail_id = $2 AND groupe_id = $3 FOR UPDATE"))
                .bind(etab)
                .bind(t.id)
                .bind(g)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)
        } else {
            sqlx::query(
                "INSERT INTO ent_cdt_remise (id, etablissement_id, travail_id, eleve_id) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (etablissement_id, travail_id, eleve_id) WHERE eleve_id IS NOT NULL DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(t.id)
            .bind(eleve)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query_as::<_, RemiseRowC5>(&format!("{SELECT_REMISE_C5} AND travail_id = $2 AND eleve_id = $3 FOR UPDATE"))
                .bind(etab)
                .bind(t.id)
                .bind(eleve)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)
        }
    }

    /// CDT-06 : l'élève dépose (premier dépôt ou redépôt). `recu_le` est l'instant de **début** de réception, pris à
    /// l'entrée de la requête (R-C10).
    pub async fn cdt_deposer(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        body: DepotEntree,
        recu_le: DateTime<Utc>,
    ) -> Result<AccuseDto, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let (eleve, famille, parti) = self.cdt_proprietaire_remise(a, None).await?;
        // C-D5 : un élève parti ne dépose plus ; un responsable ne dépose jamais à la place de l'élève.
        if famille || parti {
            return Err(EntDbError::Forbidden);
        }
        self.cdt_deposer_pour(a, travail_id, eleve, body.texte, body.commentaire, body.fichiers, recu_le, false)
            .await
    }

    /// Dépôt commun à l'élève et au professeur (« au nom de »).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn cdt_deposer_pour(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        eleve: Uuid,
        texte: Option<String>,
        commentaire: Option<String>,
        fichiers: Vec<FichierDepot>,
        recu_le: DateTime<Utc>,
        au_nom_de: bool,
    ) -> Result<AccuseDto, EntDbError> {
        let etab = a.etablissement_id;
        let texte = texte_optionnel(&texte, 20000, "texte trop long")?;
        let commentaire = texte_optionnel(&commentaire, 2000, "commentaire trop long")?;
        self.cdt_projeter_echus(etab).await?;
        let jour = self.jour_etab(etab).await?;
        let (t, formats) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let t = charger_travail(&mut tx, etab, travail_id).await?;
            if t.rendu != "en_ligne" {
                return Err(conflit("ce travail n'est pas à rendre en ligne"));
            }
            if !au_nom_de && !travail_visible(&t) {
                return Err(introuvable());
            }
            if !eleve_dans_public(&mut tx, etab, &t, eleve, jour).await? {
                return Err(if au_nom_de { invalide("cet élève n'est pas dans le public du travail") } else { introuvable() });
            }
            let f = self.cdt_formats(&mut tx, etab, t.id).await?;
            (t, f)
        };
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        // Contrôle puis rangement des fichiers AVANT la transaction : un fichier refusé ne laisse aucun dépôt (CC-32).
        let prets = self.cdt_preparer_fichiers(etab, &fichiers, &formats).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = self.cdt_remise_verrouiller(&mut tx, etab, &t, eleve).await?;
        let a_retour_rendu: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_retour WHERE etablissement_id = $1 AND remise_id = $2 AND etat = 'rendu')",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // CC-34 : après le retour du professeur, le redépôt n'est ouvert que si la remise est rouverte (« à refaire »).
        if r.etat == "deposee" && a_retour_rendu {
            return Err(conflit("la copie a été rendue : le professeur doit rouvrir la remise pour un nouveau dépôt"));
        }
        let (effective, cloture, _) = {
            let membre = if let Some(g) = r.groupe_id { self.echeance_de_groupe(&mut tx, etab, &t, g).await? } else { echeances_de(&mut tx, etab, &t, eleve).await? };
            membre
        };
        let en_retard = !au_nom_de && recu_le > effective;
        if !au_nom_de {
            if recu_le > cloture {
                return Err(conflit("le ramassage de ce travail est clos"));
            }
            if en_retard && !t.retard_accepte {
                return Err(refus(RefusMetier::Interdit, "l'échéance est dépassée et le retard n'est pas accepté"));
            }
        }
        let (n_brouillon,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_cdt_piece_jointe WHERE etablissement_id = $1 AND porteur = 'remise' AND porteur_id = $2 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let texte_final = texte.clone().or(r.brouillon_texte.clone());
        let commentaire_final = commentaire.clone().or(r.brouillon_commentaire.clone());
        if prets.is_empty() && n_brouillon == 0 && texte_final.is_none() {
            return Err(invalide("rien à déposer : ajoutez un fichier ou un texte"));
        }
        if n_brouillon + prets.len() as i64 > MAX_PIECES_DEPOT {
            return Err(conflit("nombre maximal de pièces atteint"));
        }
        let numero: i32 = sqlx::query_scalar("SELECT COALESCE(max(numero), 0)::int + 1 FROM ent_cdt_depot WHERE etablissement_id = $1 AND remise_id = $2")
            .bind(etab)
            .bind(r.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let depot_id = Uuid::new_v4();
        // Empreintes des fichiers du brouillon (déjà rangés) puis des nouveaux, dans l'ordre.
        let mut empreintes: Vec<String> = sqlx::query_scalar(
            "SELECT o.sha256 FROM ent_cdt_piece_jointe p JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.id = p.objet_id
              WHERE p.etablissement_id = $1 AND p.porteur = 'remise' AND p.porteur_id = $2 AND p.retiree_le IS NULL
              ORDER BY p.ordre, p.cree_le, p.id",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        empreintes.extend(prets.iter().map(|p| p.sha256.clone()));
        let empreinte = empreinte_depot(texte_final.as_deref(), &empreintes);
        let nb_fichiers = (n_brouillon + prets.len() as i64) as i16;
        sqlx::query(
            "INSERT INTO ent_cdt_depot (id, etablissement_id, remise_id, numero, recu_le, par_user_id, au_nom_de, texte,
                    empreinte, nb_fichiers, commentaire_eleve)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(depot_id)
        .bind(etab)
        .bind(r.id)
        .bind(numero)
        .bind(recu_le)
        .bind(a.user_id)
        .bind(au_nom_de)
        .bind(&texte_final)
        .bind(&empreinte)
        .bind(nb_fichiers)
        .bind(&commentaire_final)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Les pièces du brouillon deviennent celles du dépôt ; les nouvelles s'y ajoutent.
        sqlx::query(
            "UPDATE ent_cdt_piece_jointe SET porteur = 'depot', porteur_id = $3
              WHERE etablissement_id = $1 AND porteur = 'remise' AND porteur_id = $2 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(r.id)
        .bind(depot_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        attacher_pieces(&mut tx, etab, "depot", depot_id, n_brouillon as i16, &prets, "copie").await?;
        if r.etat == "a_refaire" {
            // Nouveau dépôt après réouverture : le retour rendu sur la copie précédente repasse en brouillon (sa version
            // reste lisible) ; il sera rendu à nouveau sur la nouvelle copie.
            sqlx::query(
                "INSERT INTO ent_cdt_retour_version (id, etablissement_id, retour_id, version, commentaire, note_id, rendu_le, auteur_user_id)
                 SELECT gen_random_uuid(), etablissement_id, id, version, commentaire, note_id, rendu_le, auteur_user_id
                   FROM ent_cdt_retour WHERE etablissement_id = $1 AND remise_id = $2 AND etat = 'rendu'",
            )
            .bind(etab)
            .bind(r.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_cdt_retour SET etat = 'brouillon', rendu_le = NULL, version = version + 1, modifie_le = now()
                  WHERE etablissement_id = $1 AND remise_id = $2 AND etat = 'rendu'",
            )
            .bind(etab)
            .bind(r.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "UPDATE ent_cdt_remise SET etat = 'deposee', dernier_depot_id = $3, en_retard = $4, version = version + 1,
                    brouillon_texte = NULL, brouillon_commentaire = NULL, brouillon_le = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(r.id)
        .bind(depot_id)
        .bind(en_retard)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let premier = numero == 1;
        let charge = json!({ "travail_id": t.id, "remise_id": r.id, "depot_id": depot_id, "numero": numero, "en_retard": en_retard });
        publier_cdt(
            &mut tx,
            a,
            if premier { types::CAHIER_TRAVAIL_RENDU } else { types::CAHIER_REDEPOT },
            t.id,
            Some(eleve),
            charge,
        )
        .await?;
        let r2: RemiseRowC5 = sqlx::query_as(&format!("{SELECT_REMISE_C5} AND id = $2"))
            .bind(etab)
            .bind(r.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let mut accuses = self.accuses_de(&mut tx, etab, &r2).await?;
        tx.commit().await.map_err(db)?;
        let mut accuse = accuses.drain(..).find(|x| x.depot_id == depot_id).ok_or_else(introuvable)?;
        accuse.en_retard = en_retard;
        Ok(accuse)
    }

    /// Échéance effective et clôture d'une remise de groupe : la plus tardive des échéances effectives des membres.
    async fn echeance_de_groupe(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        t: &TravailC5,
        groupe: Uuid,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>, bool), EntDbError> {
        let membres: Vec<Uuid> = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_cdt_groupe_membre WHERE etablissement_id = $1 AND groupe_id = $2 AND jusqu_au IS NULL",
        )
        .bind(etab)
        .bind(groupe)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut meilleur: Option<(DateTime<Utc>, DateTime<Utc>, bool)> = None;
        for m in membres {
            let e = echeances_de(tx, etab, t, m).await?;
            if meilleur.as_ref().is_none_or(|b| e.0 > b.0) {
                meilleur = Some(e);
            }
        }
        match meilleur {
            Some(e) => Ok(e),
            None => echeances_sans_eleve(tx, etab, t).await,
        }
    }

    /// CC-32 : « je n'arrive pas à déposer » : prévient le professeur avec l'heure (preuve de bonne foi).
    pub async fn cdt_depot_signaler(&self, a: &Acteur<'_>, travail_id: Uuid, body: SignalementBody) -> Result<DateTime<Utc>, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let (eleve, famille, parti) = self.cdt_proprietaire_remise(a, None).await?;
        if famille || parti {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let message = texte_optionnel(&body.message, 1000, "message trop long")?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t = charger_travail(&mut tx, etab, travail_id).await?;
        if !travail_visible(&t) || !eleve_dans_public(&mut tx, etab, &t, eleve, jour).await? {
            return Err(introuvable());
        }
        let le: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO ent_cdt_depot_signalement (id, etablissement_id, travail_id, eleve_id, message)
             VALUES ($1, $2, $3, $4, $5) RETURNING cree_le",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .bind(&message)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_DEPOT_IMPOSSIBLE_SIGNALE,
            t.id,
            Some(eleve),
            json!({ "travail_id": t.id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(le)
    }

    /// CDT-195 (côté demandeur) : l'élève, ou son responsable habilité, demande un délai. Une demande ouverte à la fois.
    pub async fn cdt_delai_demander(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        eleve_id: Option<Uuid>,
        body: DemandeDelaiBody,
    ) -> Result<DelaiLuDto, EntDbError> {
        let (eleve, _famille, parti) = self.cdt_proprietaire_remise(a, eleve_id).await?;
        if parti {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let message = texte_optionnel(&body.message, 1000, "message trop long")?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t = charger_travail(&mut tx, etab, travail_id).await?;
        if !travail_visible(&t) || !eleve_dans_public(&mut tx, etab, &t, eleve, jour).await? {
            return Err(introuvable());
        }
        if t.rendu != "en_ligne" {
            return Err(conflit("ce travail n'est pas à rendre en ligne"));
        }
        let ouverte: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_delai_demande WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3 AND etat = 'ouverte')",
        )
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if ouverte {
            return Err(conflit("une demande de délai est déjà en attente"));
        }
        let personne: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 ORDER BY id LIMIT 1",
        )
        .bind(etab)
        .bind(a.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        let cree_le: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO ent_cdt_delai_demande (id, etablissement_id, travail_id, eleve_id, demandeur_personne_id, message)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING cree_le",
        )
        .bind(id)
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .bind(personne)
        .bind(&message)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_DELAI_DEMANDE, t.id, Some(eleve), json!({ "travail_id": t.id, "demande_id": id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(DelaiLuDto { id, etat: "ouverte".into(), message, reponse: None, cree_le })
    }

    // ───────────────────────────── groupes ─────────────────────────────

    /// CDT-185 (composition) : le professeur compose les groupes d'un travail à remise par groupe ; un élève ne figure que
    /// dans un groupe ; seuls des élèves du public (R-C25). Recomposer remplace les groupes **sans remise** seulement.
    pub async fn cdt_groupes_composer(&self, a: &Acteur<'_>, travail_id: Uuid, body: GroupesBody) -> Result<Vec<GroupeDto>, EntDbError> {
        let etab = a.etablissement_id;
        let t = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            charger_travail(&mut tx, etab, travail_id).await?
        };
        self.cdt_exiger_saisie(a, t.cours_id, None).await?;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        if !t.remise_par_groupe {
            return Err(conflit("ce travail ne se rend pas par groupe"));
        }
        if body.groupes.is_empty() || body.groupes.len() > 40 {
            return Err(invalide("de 1 à 40 groupes"));
        }
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut vus = std::collections::HashSet::new();
        for g in &body.groupes {
            let libelle = g.libelle.trim();
            if libelle.is_empty() || libelle.chars().count() > 80 {
                return Err(invalide("libellé de groupe vide ou trop long"));
            }
            if g.eleve_ids.is_empty() || t.taille_groupe.is_some_and(|n| g.eleve_ids.len() as i64 > i64::from(n)) {
                return Err(invalide("groupe vide ou plus grand que la taille fixée"));
            }
            for e in &g.eleve_ids {
                if !vus.insert(*e) {
                    return Err(invalide("un élève ne peut figurer que dans un groupe"));
                }
                if !eleve_dans_public(&mut tx, etab, &t, *e, jour).await? {
                    return Err(invalide("un élève hors du public du travail ne peut pas être ajouté"));
                }
            }
        }
        // Les groupes déjà porteurs d'une remise ne se défont pas : rien ne disparaît.
        let porteurs: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_cdt_remise WHERE etablissement_id = $1 AND travail_id = $2 AND groupe_id IS NOT NULL",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if porteurs > 0 {
            return Err(conflit("des remises de groupe existent : les groupes ne se recomposent plus"));
        }
        // Retrait logique des appartenances en cours (jamais de suppression).
        sqlx::query(
            "UPDATE ent_cdt_groupe_membre SET jusqu_au = now() + interval '1 microsecond'
              WHERE etablissement_id = $1 AND jusqu_au IS NULL
                AND groupe_id IN (SELECT id FROM ent_cdt_groupe WHERE etablissement_id = $1 AND travail_id = $2)",
        )
        .bind(etab)
        .bind(t.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for g in &body.groupes {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_cdt_groupe (id, etablissement_id, travail_id, libelle) VALUES ($1, $2, $3, $4)")
                .bind(id)
                .bind(etab)
                .bind(t.id)
                .bind(g.libelle.trim())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            for e in &g.eleve_ids {
                sqlx::query(
                    "INSERT INTO ent_cdt_groupe_membre (etablissement_id, groupe_id, eleve_id, depuis) VALUES ($1, $2, $3, now() + interval '2 microseconds')",
                )
                .bind(etab)
                .bind(id)
                .bind(e)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        publier_cdt(&mut tx, a, types::CAHIER_GROUPES_COMPOSES, t.id, None, json!({ "travail_id": t.id, "nb_groupes": body.groupes.len() })).await?;
        tx.commit().await.map_err(db)?;
        self.cdt_groupes_actifs(a, travail_id).await
    }

    /// Groupes actifs d'un travail (professeur en saisie, ou élève du public pour s'inscrire).
    pub async fn cdt_groupes_actifs(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<Vec<GroupeDto>, EntDbError> {
        let etab = a.etablissement_id;
        let t = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            charger_travail(&mut tx, etab, travail_id).await?
        };
        if Role::parse(a.role) == Some(Role::Eleve) {
            let (eleve, _, parti) = self.cdt_proprietaire_remise(a, None).await?;
            let jour = self.jour_etab(etab).await?;
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            if parti || !travail_visible(&t) || !eleve_dans_public(&mut tx, etab, &t, eleve, jour).await? {
                return Err(introuvable());
            }
        } else {
            self.cdt_exiger_saisie(a, t.cours_id, None).await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, libelle FROM ent_cdt_groupe WHERE etablissement_id = $1 AND travail_id = $2 ORDER BY libelle, id",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for (id, libelle) in lignes {
            sortie.push(self.groupe_dto(&mut tx, etab, &t, id, libelle).await?);
        }
        Ok(sortie)
    }

    /// Inscription libre : l'élève crée un groupe ou rejoint un groupe existant avant la date limite, dans la taille fixée.
    pub async fn cdt_groupe_libre(&self, a: &Acteur<'_>, travail_id: Uuid, body: GroupeLibreBody) -> Result<GroupeDto, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let (eleve, famille, parti) = self.cdt_proprietaire_remise(a, None).await?;
        if famille || parti {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t = charger_travail(&mut tx, etab, travail_id).await?;
        if !travail_visible(&t) || !eleve_dans_public(&mut tx, etab, &t, eleve, jour).await? {
            return Err(introuvable());
        }
        if !t.remise_par_groupe {
            return Err(conflit("ce travail ne se rend pas par groupe"));
        }
        match t.inscription_groupes_jusqu_au {
            Some(limite) if Utc::now() <= limite => {}
            _ => return Err(conflit("l'inscription dans les groupes n'est pas ouverte")),
        }
        // Verrou sur le travail : deux élèves qui rejoignent en même temps ne dépassent pas la taille fixée.
        sqlx::query("SELECT 1 FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(t.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if self.groupe_c5(&mut tx, etab, &t, eleve).await?.is_some() {
            return Err(conflit("vous êtes déjà dans un groupe"));
        }
        let deja_remise: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_remise WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3)",
        )
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja_remise {
            return Err(conflit("vous avez déjà commencé une remise individuelle"));
        }
        let groupe_id = match (body.groupe_id, body.libelle.as_deref().map(str::trim).filter(|l| !l.is_empty())) {
            (Some(g), _) => {
                let (existe,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_cdt_groupe WHERE etablissement_id = $1 AND travail_id = $2 AND id = $3)")
                    .bind(etab)
                    .bind(t.id)
                    .bind(g)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if !existe {
                    return Err(introuvable());
                }
                let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_groupe_membre WHERE etablissement_id = $1 AND groupe_id = $2 AND jusqu_au IS NULL")
                    .bind(etab)
                    .bind(g)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if t.taille_groupe.is_some_and(|max| n >= i64::from(max)) {
                    return Err(conflit("ce groupe est complet"));
                }
                g
            }
            (None, Some(libelle)) => {
                if libelle.chars().count() > 80 {
                    return Err(invalide("libellé de groupe trop long"));
                }
                let id = Uuid::new_v4();
                sqlx::query("INSERT INTO ent_cdt_groupe (id, etablissement_id, travail_id, libelle) VALUES ($1, $2, $3, $4)")
                    .bind(id)
                    .bind(etab)
                    .bind(t.id)
                    .bind(libelle)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                id
            }
            _ => return Err(invalide("indiquez le groupe à rejoindre ou le libellé du nouveau groupe")),
        };
        sqlx::query("INSERT INTO ent_cdt_groupe_membre (etablissement_id, groupe_id, eleve_id, depuis) VALUES ($1, $2, $3, now())")
            .bind(etab)
            .bind(groupe_id)
            .bind(eleve)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let libelle: String = sqlx::query_scalar("SELECT libelle FROM ent_cdt_groupe WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(groupe_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let dto = self.groupe_dto(&mut tx, etab, &t, groupe_id, libelle).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // ───────────────────────────── pièces ─────────────────────────────

    /// Lien court vers une pièce d'une remise ou d'un retour, après contrôle des droits (R-C13). `motif` : lecture de la
    /// direction, journalisée (jamais d'accès sans motif).
    pub async fn cdt_piece_lien(&self, a: &Acteur<'_>, piece_id: Uuid, motif: Option<&str>) -> Result<LienFichier, EntDbError> {
        let etab = a.etablissement_id;
        #[derive(FromRow)]
        struct P {
            porteur: String,
            porteur_id: Uuid,
            cle_objet: String,
            mime: Option<String>,
            nom: String,
        }
        let p: P = sqlx::query_as(
            "SELECT p.porteur, p.porteur_id, o.cle_objet, o.mime, p.nom FROM ent_cdt_piece_jointe p
               JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.id = p.objet_id
              WHERE p.etablissement_id = $1 AND p.id = $2 AND p.retiree_le IS NULL",
        )
        .bind(etab)
        .bind(piece_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        // Remise concernée selon le porteur.
        let remise: Uuid = match p.porteur.as_str() {
            "remise" => p.porteur_id,
            "depot" => sqlx::query_scalar("SELECT remise_id FROM ent_cdt_depot WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(p.porteur_id)
                .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await
                .map_err(db)?,
            "retour" => sqlx::query_scalar("SELECT remise_id FROM ent_cdt_retour WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(p.porteur_id)
                .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await
                .map_err(db)?,
            _ => return Err(introuvable()),
        };
        self.cdt_lecture_copie_autorisee(a, remise, &p.porteur, motif).await?;
        let lien = self
            .presigner()
            .issue(
                &LinkSpec {
                    key: p.cle_objet.clone(),
                    user_id: a.user_id,
                    etablissement_id: etab,
                    disposition: Disposition::Attachment,
                    filename: p.nom.clone(),
                    mime: p.mime.clone().unwrap_or_else(|| "application/octet-stream".into()),
                },
                ent_object::presign::now_unix(),
            )
            .map_err(|e| match e {
                PresignError::Disabled => EntDbError::LiensIndisponibles,
                _ => EntDbError::FichierAbsent,
            })?;
        if !self.objet_existe(&p.cle_objet).await? {
            return Err(EntDbError::FichierAbsent);
        }
        self.tracer_acces_sensible(etab, a.user_id, a.role, None, "telechargement", "cahier_copie", &format!("Lien court vers une pièce de copie {piece_id}"))
            .await?;
        Ok(LienFichier { url: lien.url, expires_at: lien.expires_at })
    }

    /// R-C13 : qui lit une pièce de copie. `porteur` : `remise` et `depot` (le travail de l'élève), `retour` (copie annotée).
    async fn cdt_lecture_copie_autorisee(
        &self,
        a: &Acteur<'_>,
        remise: Uuid,
        porteur: &str,
        motif: Option<&str>,
    ) -> Result<(), EntDbError> {
        let etab = a.etablissement_id;
        let (travail, eleve, groupe): (Uuid, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
            "SELECT travail_id, eleve_id, groupe_id FROM ent_cdt_remise WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(remise)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        match Role::parse(a.role) {
            Some(Role::Eleve) => {
                let moi = self.eleve_du_compte(a).await?.ok_or(EntDbError::Forbidden)?;
                let membre: bool = match (eleve, groupe) {
                    (Some(e), _) => e == moi,
                    (_, Some(g)) => sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM ent_cdt_groupe_membre WHERE etablissement_id = $1 AND groupe_id = $2 AND eleve_id = $3)",
                    )
                    .bind(etab)
                    .bind(g)
                    .bind(moi)
                    .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                    .await
                    .map_err(db)?,
                    _ => false,
                };
                // Pièce de retour : seulement si le retour est rendu (jamais le brouillon du professeur).
                if membre && (porteur != "retour" || self.retour_rendu(etab, remise).await?) {
                    return Ok(());
                }
                Err(EntDbError::Forbidden)
            }
            Some(Role::Parent) => {
                // R-C37 : la copie annotée rendue seulement, jamais les dépôts.
                let Some(e) = eleve else { return Err(EntDbError::Forbidden) };
                let (e, famille, _) = self.cdt_proprietaire_remise(a, Some(e)).await?;
                if famille && porteur == "retour" && self.retour_rendu(etab, remise).await? && self.cdt_parent_voit_notes(a, e).await? {
                    return Ok(());
                }
                Err(EntDbError::Forbidden)
            }
            _ => {
                let cours: Uuid = sqlx::query_scalar("SELECT cours_id FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(travail)
                    .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                    .await
                    .map_err(db)?;
                if self.cdt_exiger_saisie(a, cours, None).await.is_ok() {
                    return Ok(());
                }
                // Personne saisie par un signalement du professeur (CDT-199) : la lit, journalisée.
                let vers = match Role::parse(a.role) {
                    Some(Role::Cpe) => Some("cpe"),
                    Some(Role::Direction) => Some("direction"),
                    _ => None,
                };
                if let Some(vers) = vers {
                    let saisie: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM ent_cdt_signalement_copie WHERE etablissement_id = $1 AND remise_id = $2 AND vers = $3)",
                    )
                    .bind(etab)
                    .bind(remise)
                    .bind(vers)
                    .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                    .await
                    .map_err(db)?;
                    if saisie {
                        self.journaliser_lecture_copie(etab, a.user_id, remise, "signalement du professeur").await?;
                        return Ok(());
                    }
                }
                // Direction : sur motif journalisé, jamais autrement.
                if Role::parse(a.role) == Some(Role::Direction) {
                    let motif = motif.map(str::trim).filter(|m| m.chars().count() >= 5).ok_or_else(|| invalide("un motif est obligatoire pour lire une copie"))?;
                    if motif.chars().count() > 500 {
                        return Err(invalide("motif trop long"));
                    }
                    self.journaliser_lecture_copie(etab, a.user_id, remise, motif).await?;
                    return Ok(());
                }
                Err(EntDbError::Forbidden)
            }
        }
    }

    /// Journal des lectures de copie sur motif (ajout seul, `ent_cdt_consultation`) : écrit **avant** de servir le lien.
    async fn journaliser_lecture_copie(&self, etab: Uuid, user: Uuid, remise: Uuid, motif: &str) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_consultation (id, etablissement_id, qui_user_id, nature, objet_id, motif)
             VALUES ($1, $2, $3, 'copie_motif', $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(user)
        .bind(remise)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn retour_rendu(&self, etab: Uuid, remise: Uuid) -> Result<bool, EntDbError> {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_cdt_retour WHERE etablissement_id = $1 AND remise_id = $2 AND etat = 'rendu')")
            .bind(etab)
            .bind(remise)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)
    }
}

/// Échéance d'un travail pour un groupe sans membre (cas dégénéré) : celle du travail.
async fn echeances_sans_eleve(tx: &mut PgConnection, etab: Uuid, t: &TravailC5) -> Result<(DateTime<Utc>, DateTime<Utc>, bool), EntDbError> {
    let cloture: DateTime<Utc> = sqlx::query_scalar(
        "SELECT COALESCE($3, $4 + make_interval(days => COALESCE((SELECT r.cloture_ramassage_jours FROM ent_cdt_reglage r
                                                                    WHERE r.etablissement_id = $1 AND r.annee_id = $2), 14)))",
    )
    .bind(etab)
    .bind(t.annee_id)
    .bind(t.cloture_ramassage_le)
    .bind(t.echeance_le)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    Ok((t.echeance_le, cloture, false))
}

/// Ce que l'élève peut faire maintenant, et pourquoi pas (message clair, CC-32).
#[allow(clippy::too_many_arguments)]
fn fermeture(
    t: &TravailC5,
    effective: DateTime<Utc>,
    cloture: DateTime<Utc>,
    maintenant: DateTime<Utc>,
    famille: bool,
    parti: bool,
    concerne: bool,
) -> (bool, Option<String>) {
    let non = |m: &str| (false, Some(m.to_string()));
    if famille {
        return non("le dépôt est fait par l'élève");
    }
    if parti {
        return non("l'élève n'est plus dans l'établissement");
    }
    if t.rendu != "en_ligne" {
        return non("ce travail ne se rend pas en ligne");
    }
    if !concerne {
        return non("ce travail n'est plus ouvert");
    }
    if maintenant > cloture {
        return non("le ramassage est clos");
    }
    if maintenant > effective && !t.retard_accepte {
        return non("l'échéance est dépassée et le retard n'est pas accepté");
    }
    (true, None)
}

/// Rattache des fichiers prêts à un porteur (dépôt, remise, retour) : nom neutre `copie-N.ext`, jamais le nom d'origine.
pub(crate) async fn attacher_pieces(
    tx: &mut PgConnection,
    etab: Uuid,
    porteur: &str,
    porteur_id: Uuid,
    deja: i16,
    prets: &[FichierPret],
    prefixe: &str,
) -> Result<(), EntDbError> {
    let mut noms: HashMap<usize, String> = HashMap::new();
    for (i, p) in prets.iter().enumerate() {
        let rang = deja as usize + i + 1;
        noms.insert(i, format!("{prefixe}-{rang}.{}", p.extension));
        sqlx::query(
            "INSERT INTO ent_cdt_piece_jointe (id, etablissement_id, objet_id, porteur, porteur_id, nom, ordre)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(p.objet_id)
        .bind(porteur)
        .bind(porteur_id)
        .bind(&noms[&i])
        .bind(rang as i16)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let _ = (p.mime, p.taille);
    }
    Ok(())
}
