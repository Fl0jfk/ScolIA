//! Socle 29, lot P5 : **recherche globale** (`docs/socles/29_PORTAILS.md` §12.1, §18.6, §22.4, S-2907).
//!
//! * **Côté serveur uniquement** : aucune liste d'élèves n'est envoyée au navigateur pour un classement local.
//! * **Sources fédérées, chacune sous les droits du lecteur** : élèves (port E6 `search_eleves_pour_acteur`,
//!   portée de l'annuaire, élèves protégés et confidentialité renforcée déjà filtrés **dans la requête**),
//!   personnes du socle 4 (droit `personnes.gerer` ; adresse protégée ou « ne pas solliciter » : ni résultat
//!   ni indice sans `responsables.protege.lire`), personnels (annuaire : nom seul, jamais l'adresse
//!   électronique), classes, groupes et salles du socle 5 (`referentiel.lire`).
//! * **Règles** : au moins 2 caractères, insensible aux accents et à la casse (`ent_cle_recherche`), 10 résultats
//!   par source, 40 au total, aucune pagination. Résultat = libellé minimal et lien ; jamais coordonnée,
//!   identifiant national, motif ni donnée de santé (ces sources ne sont pas interrogées).
//! * **Anti-énumération** : un compteur par personne et par jour (`ent_recherche_compteur`, sans aucun texte,
//!   R-PO19). Passé 75 % du plafond : ralentissement ; au-delà du plafond (400, secrétariat 1 500) ou après trop de
//!   recherches de lettres (≤ 2 caractères) : blocage temporaire 1 h, tâche à l'administrateur, réponse unique
//!   sans chiffre. Déblocage personnel une fois par jour avec un motif de liste fermée (POR-259) ; sinon
//!   `recherche.debloquer` (POR-196).
//!
//! Le jour est celui de l'établissement (`ent_aujourdhui_etab`) ; l'instant est fourni par l'appelant.

use crate::afaire::Destinataire;
use crate::afaire::{
    fermer_tache, ouvrir_tache, version_de, FermetureTache, MotifClotureAfaire, OuvertureTache,
};
use crate::afaire_files::nom_court;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::{Action, Role};
use ent_types::recherche::{
    BlocageRecherche, ReponseRecherche, ResultatRecherche, SourceRecherche,
    MESSAGE_RECHERCHE_BLOQUEE,
};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

/// Résultats par source et au total (§12.1).
pub const PAR_SOURCE: i64 = 10;
pub const TOTAL: usize = 40;
/// Plafond quotidien (S-2907) : 400 pour le personnel, 1 500 pour le secrétariat (rentrée).
pub const PLAFOND_PERSONNEL: i32 = 400;
pub const PLAFOND_SECRETARIAT: i32 = 1500;
/// Recherches de lettres (≤ 2 caractères) tolérées par jour avant blocage.
pub const PLAFOND_LETTRES: i32 = 60;
/// Durée d'un blocage temporaire.
pub const DUREE_BLOCAGE_MINUTES: i64 = 60;
pub const MOTIFS_DEBLOCAGE: [&str; 3] = ["rentree", "inscriptions", "autre"];

/// Plafond du jour selon le rôle de base.
pub fn plafond_de(role: Role) -> i32 {
    match role {
        Role::Administratif | Role::Admin => PLAFOND_SECRETARIAT,
        _ => PLAFOND_PERSONNEL,
    }
}

/// Étage du compteur : rien, ralentissement (≥ 75 % du plafond), blocage (plafond dépassé ou lettres).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etage {
    Libre,
    Ralenti,
    Bloque,
}

pub fn etage(nb: i32, nb_courtes: i32, plafond: i32) -> Etage {
    if nb > plafond || nb_courtes > PLAFOND_LETTRES {
        Etage::Bloque
    } else if nb * 4 >= plafond * 3 {
        Etage::Ralenti
    } else {
        Etage::Libre
    }
}

/// Jetons de la requête : clé de recherche normalisée par la fonction SQL du socle 2 (aucune dérive).
async fn jetons(conn: &mut PgConnection, q: &str) -> Result<Vec<String>, EntDbError> {
    if q.contains('\0') {
        return Err(EntDbError::SaisieInvalide("requête de recherche invalide"));
    }
    let (cle,): (String,) = sqlx::query_as("SELECT ent_cle_recherche($1)")
        .bind(q.chars().take(200).collect::<String>())
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    Ok(cle
        .split_whitespace()
        .take(5)
        .map(|t| t.chars().take(40).collect())
        .collect())
}

fn motif_like(t: &str) -> String {
    format!(
        "%{}%",
        t.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

impl PgEnt {
    /// `GET /recherche?q=` : recherche fédérée du personnel. Compte, plafonne, puis interroge les sources.
    pub async fn rechercher_global(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        q: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ReponseRecherche, EntDbError> {
        let base = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !base.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_action_etab(etab, user, role, Action::RechercheGlobale)
            .await?;
        let mots = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            jetons(&mut tx, q).await?
        };
        let longueur: usize = mots.iter().map(|m| m.chars().count()).sum();
        if longueur < 2 {
            // Trop court (ou sans lettre ni chiffre) : rien n'est cherché, rien n'est compté.
            return Ok(ReponseRecherche::default());
        }
        let ralenti = self
            .compter_recherche(etab, user, base, longueur <= 2, maintenant)
            .await?;

        let mut resultats: Vec<ResultatRecherche> = Vec::new();
        // Élèves : le port E6 filtre par portée, protection et groupes de champs.
        for e in self
            .search_eleves_pour_acteur(etab, user, role, q, PAR_SOURCE)
            .await?
        {
            let e = e.eleve;
            let classe = e.classe.as_deref().filter(|c| !c.is_empty());
            resultats.push(ResultatRecherche {
                source: SourceRecherche::Eleves,
                id: e.id,
                libelle: match classe {
                    Some(c) => format!("{} {} — {c}", e.nom.to_uppercase(), e.prenom),
                    None => format!("{} {}", e.nom.to_uppercase(), e.prenom),
                },
                lien: Some(format!("/web/eleves/{}", e.id)),
            });
        }
        let voit_protegees = self
            .a_action_etablissement(etab, user, role, Action::ResponsablesProtegeLire)
            .await?;
        let peut_personnes = self
            .a_action_etablissement(etab, user, role, Action::PersonnesGerer)
            .await?;
        let peut_referentiel = self
            .a_action_etablissement(etab, user, role, Action::ReferentielLire)
            .await?;
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            if peut_personnes {
                resultats.extend(personnes(&mut tx, etab, &mots, voit_protegees).await?);
            }
            if peut_referentiel {
                resultats.extend(referentiel(&mut tx, etab, &mots).await?);
            }
        }
        // Annuaire des personnels : le nom seulement (jamais l'adresse électronique).
        for p in self.search_staff(etab, q, PAR_SOURCE).await? {
            let nom = p
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty());
            let Some(nom) = nom else { continue };
            resultats.push(ResultatRecherche {
                source: SourceRecherche::Personnels,
                id: p.user_id,
                libelle: nom.to_string(),
                lien: None,
            });
        }
        resultats.truncate(TOTAL);
        Ok(ReponseRecherche { resultats, ralenti })
    }

    /// Compte la recherche (atomique), applique le blocage ; `Ok(ralenti)`.
    async fn compter_recherche(
        &self,
        etab: Uuid,
        user: Uuid,
        base: Role,
        courte: bool,
        maintenant: DateTime<Utc>,
    ) -> Result<bool, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_de(&mut tx, etab, maintenant).await?;
        let r = sqlx::query(
            "INSERT INTO ent_recherche_compteur (etablissement_id, compte_id, jour, nb, nb_courtes)
             VALUES ($1, $2, $3, 1, $4)
             ON CONFLICT (etablissement_id, compte_id, jour)
             DO UPDATE SET nb = ent_recherche_compteur.nb + 1,
                           nb_courtes = ent_recherche_compteur.nb_courtes + $4,
                           modifie_le = $5
             RETURNING nb, nb_courtes, bloque_jusqu_a",
        )
        .bind(etab)
        .bind(user)
        .bind(jour)
        .bind(i32::from(courte))
        .bind(maintenant)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (nb, courtes, bloque): (i32, i32, Option<DateTime<Utc>>) =
            (r.get(0), r.get(1), r.get(2));
        if bloque.is_some_and(|b| b > maintenant) {
            tx.commit().await.map_err(db)?;
            return Err(EntDbError::ConflitDossier(MESSAGE_RECHERCHE_BLOQUEE));
        }
        match etage(nb, courtes, plafond_de(base)) {
            Etage::Libre => {
                tx.commit().await.map_err(db)?;
                Ok(false)
            }
            Etage::Ralenti => {
                tx.commit().await.map_err(db)?;
                Ok(true)
            }
            Etage::Bloque => {
                sqlx::query(
                    "UPDATE ent_recherche_compteur SET bloque_jusqu_a = $4, modifie_le = $5
                      WHERE etablissement_id = $1 AND compte_id = $2 AND jour = $3",
                )
                .bind(etab)
                .bind(user)
                .bind(jour)
                .bind(maintenant + Duration::minutes(DUREE_BLOCAGE_MINUTES))
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                // Tâche de sécurité à l'administrateur : le titre est neutre, l'objet est le compte.
                ouvrir_tache(
                    &mut tx,
                    &OuvertureTache {
                        etablissement_id: etab,
                        nature: "recherche.bloquee",
                        objet_id: user,
                        eleve_id: None,
                        valeurs: vec![],
                        destinataires: vec![Destinataire::File(
                            crate::afaire::FILE_ADMIN.to_string(),
                        )],
                        echeance_le: None,
                        version_source: version_de(maintenant),
                        par_compte_id: None,
                        maintenant,
                    },
                )
                .await?;
                tx.commit().await.map_err(db)?;
                Err(EntDbError::ConflitDossier(MESSAGE_RECHERCHE_BLOQUEE))
            }
        }
    }

    /// `POST /recherche/debloquer-moi` (POR-259) : lever **soi-même** son blocage, **une fois par jour**, avec un
    /// motif de la liste fermée. Le motif et l'instant sont gardés sur la ligne du compteur (journal).
    pub async fn rechercher_debloquer_soi(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let base = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !base.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        if !MOTIFS_DEBLOCAGE.contains(&motif) {
            return Err(EntDbError::SaisieInvalide("motif de déblocage inconnu"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_de(&mut tx, etab, maintenant).await?;
        let n = sqlx::query(
            "UPDATE ent_recherche_compteur
                SET bloque_jusqu_a = NULL, debloque_soi_le = $4, motif_deblocage = $5,
                    nb = 0, nb_courtes = 0, modifie_le = $4
              WHERE etablissement_id = $1 AND compte_id = $2 AND jour = $3
                AND bloque_jusqu_a IS NOT NULL AND debloque_soi_le IS NULL",
        )
        .bind(etab)
        .bind(user)
        .bind(jour)
        .bind(maintenant)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            // Pas bloqué, ou déblocage personnel déjà utilisé aujourd'hui : l'administrateur seul peut lever.
            return Err(EntDbError::Forbidden);
        }
        fermer_blocage(&mut tx, etab, user, Some(user), maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// `GET /recherche/blocages` (administrateur) : qui est bloqué aujourd'hui (nom court, nombre ; aucun texte).
    pub async fn rechercher_blocages(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<BlocageRecherche>, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::RechercheDebloquer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT c.compte_id, u.display_name, c.bloque_jusqu_a, c.nb
               FROM ent_recherche_compteur c JOIN ent_user u ON u.id = c.compte_id
              WHERE c.etablissement_id = $1 AND c.bloque_jusqu_a > $2
              ORDER BY u.display_name, c.compte_id",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let nom: Option<String> = r.get(1);
                BlocageRecherche {
                    compte_id: r.get(0),
                    nom: nom_court(nom.as_deref()),
                    bloque_jusqu_a: r.get(2),
                    recherches_du_jour: r.get(3),
                }
            })
            .collect())
    }

    /// `POST /recherche/{compte}/debloquer` (POR-196) : l'administrateur lève un blocage ; journalisé par la
    /// tâche de sécurité qu'il ferme.
    pub async fn rechercher_debloquer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        compte: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RechercheDebloquer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_de(&mut tx, etab, maintenant).await?;
        let n = sqlx::query(
            "UPDATE ent_recherche_compteur SET bloque_jusqu_a = NULL, nb = 0, nb_courtes = 0, modifie_le = $4
              WHERE etablissement_id = $1 AND compte_id = $2 AND jour = $3 AND bloque_jusqu_a IS NOT NULL",
        )
        .bind(etab)
        .bind(compte)
        .bind(jour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(EntDbError::Domain(domain_eleve::DomainError::NotFound));
        }
        fermer_blocage(&mut tx, etab, compte, Some(acteur), maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Purge des compteurs de plus de 30 jours (volatile). Retourne le nombre de lignes retirées.
    pub async fn purger_recherche_compteurs(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<u64, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_de(&mut tx, etab, maintenant).await?;
        let n = sqlx::query(
            "DELETE FROM ent_recherche_compteur WHERE etablissement_id = $1 AND jour < $2",
        )
        .bind(etab)
        .bind(jour - Duration::days(30))
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        tx.commit().await.map_err(db)?;
        Ok(n)
    }
}

async fn jour_de(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<NaiveDate, EntDbError> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    Ok(j)
}

/// Ferme la tâche de sécurité d'un blocage levé.
async fn fermer_blocage(
    conn: &mut PgConnection,
    etab: Uuid,
    compte: Uuid,
    par: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    fermer_tache(
        conn,
        &FermetureTache {
            etablissement_id: etab,
            nature: "recherche.bloquee",
            objet_id: compte,
            destinataire: None,
            motif: MotifClotureAfaire::Faite,
            par_compte_id: par,
            version_source: version_de(maintenant),
            maintenant,
        },
    )
    .await?;
    Ok(())
}

/// Personnes du socle 4 (responsables, contacts) : nom seulement. Adresse protégée et « ne pas solliciter » :
/// ni résultat ni indice sans le droit de lire les protections.
async fn personnes(
    conn: &mut PgConnection,
    etab: Uuid,
    mots: &[String],
    voit_protegees: bool,
) -> Result<Vec<ResultatRecherche>, EntDbError> {
    let mut conds = String::new();
    for i in 0..mots.len() {
        conds += &format!(
            " AND ent_cle_recherche(coalesce(p.prenom, '') || ' ' || p.nom) LIKE ${}",
            i + 4
        );
    }
    let sql = format!(
        "SELECT p.id, p.nom, p.prenom FROM ent_personne p
          WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL
            AND ($2 OR (
                 NOT EXISTS (SELECT 1 FROM ent_adresse_protegee a
                              WHERE a.etablissement_id = p.etablissement_id AND a.personne_id = p.id)
             AND NOT EXISTS (SELECT 1 FROM ent_personne_reserve r
                              WHERE r.etablissement_id = p.etablissement_id AND r.personne_id = p.id
                                AND r.ne_pas_solliciter)))
            {conds}
          ORDER BY lower(p.nom), lower(coalesce(p.prenom, '')), p.id
          LIMIT $3"
    );
    let mut q = sqlx::query(&sql)
        .bind(etab)
        .bind(voit_protegees)
        .bind(PAR_SOURCE);
    for m in mots {
        q = q.bind(motif_like(m));
    }
    let rows = q.fetch_all(&mut *conn).await.map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let (nom, prenom): (String, Option<String>) = (r.get(1), r.get(2));
            ResultatRecherche {
                source: SourceRecherche::Personnes,
                id: r.get(0),
                libelle: match prenom.filter(|p| !p.is_empty()) {
                    Some(p) => format!("{} {p}", nom.to_uppercase()),
                    None => nom.to_uppercase(),
                },
                lien: None,
            }
        })
        .collect())
}

/// Classes, groupes et salles (socle 5) : libellé et code, rien d'autre.
async fn referentiel(
    conn: &mut PgConnection,
    etab: Uuid,
    mots: &[String],
) -> Result<Vec<ResultatRecherche>, EntDbError> {
    let mut out = Vec::new();
    for (source, table, lien) in [
        (SourceRecherche::Classes, "ent_classe", Some("/web/classes")),
        (SourceRecherche::Groupes, "ent_groupe", None),
        (SourceRecherche::Salles, "ent_salle", None),
    ] {
        let mut conds = String::new();
        for i in 0..mots.len() {
            conds += &format!(
                " AND ent_cle_recherche(t.libelle || ' ' || t.code) LIKE ${}",
                i + 3
            );
        }
        let actif = if table == "ent_salle" {
            " AND t.actif"
        } else {
            ""
        };
        let sql = format!(
            "SELECT t.id, t.libelle FROM {table} t
              WHERE t.etablissement_id = $1{actif}{conds}
              ORDER BY lower(t.libelle), t.id LIMIT $2"
        );
        let mut q = sqlx::query(&sql).bind(etab).bind(PAR_SOURCE);
        for m in mots {
            q = q.bind(motif_like(m));
        }
        for r in q.fetch_all(&mut *conn).await.map_err(db)? {
            let id: Uuid = r.get(0);
            out.push(ResultatRecherche {
                source,
                id,
                libelle: r.get(1),
                lien: lien.map(|l| format!("{l}/{id}")),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_etages_du_compteur() {
        let p = PLAFOND_PERSONNEL;
        assert_eq!(etage(1, 0, p), Etage::Libre);
        assert_eq!(etage(299, 0, p), Etage::Libre);
        assert_eq!(etage(300, 0, p), Etage::Ralenti);
        assert_eq!(etage(400, 0, p), Etage::Ralenti);
        assert_eq!(etage(401, 0, p), Etage::Bloque);
        // Les lettres (a, b, c…) bloquent bien avant le plafond.
        assert_eq!(etage(61, 61, p), Etage::Bloque);
        assert_eq!(etage(60, 60, p), Etage::Libre);
    }

    #[test]
    fn le_secretariat_a_un_plafond_plus_haut_a_la_rentree() {
        assert_eq!(plafond_de(Role::Prof), 400);
        assert_eq!(plafond_de(Role::Cpe), 400);
        assert_eq!(plafond_de(Role::Administratif), 1500);
        assert_eq!(etage(500, 0, plafond_de(Role::Administratif)), Etage::Libre);
    }

    #[test]
    fn les_jokers_de_la_requete_sont_neutralises() {
        assert_eq!(motif_like("100%_a\\"), "%100\\%\\_a\\\\%");
    }
}
