//! Socle 7, lot D3 : **premières natures** (certificat de scolarité, attestation d'inscription) et **vérification
//! publique** (`docs/socles/07_DOCUMENTS.md` §4.1, §9.1, §9.7, §17.7, §23 ligne D3).
//!
//! Natures **sans signature personnelle** : le document est scellé par le cachet de l'établissement ; quand l'émetteur
//! n'est pas le signataire désigné pour la nature, la mention « par délégation » est portée par la version. Le
//! parapheur (signature d'une personne habilitée) vient avec D6.
//!
//! Règles appliquées ici :
//! * **contrôles bloquants avant tout rendu** (§9.1) : élève scolarisé à la date, identité complète (date de naissance
//!   exacte), au moins un destinataire autorisé ; chacun refuse **sans rien écrire** (aucun numéro consommé) ;
//! * **un certificat destiné à un tiers n'imprime que le niveau**, jamais la division ni un dispositif révélateur
//!   (SEGPA, ULIS, UPE2A…) (D-027 B) : la classe n'est ni lue ni transmise au moteur ;
//! * **adresse protégée jamais rendue** : les natures de la famille « attestation » n'impriment aucune adresse ; pour
//!   les familles qui en impriment, [`PgEnt::adresse_postale_permise`] ne renvoie jamais celle d'une personne protégée ;
//! * **destinataires** : les titulaires de l'autorité parentale, liens non suspendus et actifs à la date ; l'élève
//!   majeur reçoit lui-même son exemplaire. Le droit de lien `recevoir_documents` (socle 4, Q-D29) n'est pas encore
//!   codé : en attendant, **l'autorité parentale vaut droit de recevoir** — l'adaptateur est isolé dans
//!   [`titulaires_destinataires`] ;
//! * **vérification publique** : par code (3 modes, §9.7) ou par dépôt du PDF ; réponse **identique** pour un code
//!   inexistant, mal formé ou d'un autre établissement ; jamais d'adresse, de classe ni de contenu.

use chrono::{Datelike, NaiveDate};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::documents::*;
use crate::documents_repo::*;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Clés des données figées d'une attestation : le contrat avec le moteur de rendu (D1) et avec la vérification publique.
pub const CLE_NOM: &str = "titulaire.nom";
pub const CLE_PRENOM: &str = "titulaire.prenom";
/// Date de naissance ISO `AAAA-MM-JJ`.
pub const CLE_NAISSANCE: &str = "titulaire.naissance";
/// Niveau d'enseignement seul (« 4e ») : jamais la division ni un dispositif.
pub const CLE_NIVEAU: &str = "scolarite.niveau";
/// Année scolaire (« 2026-2027 »).
pub const CLE_ANNEE: &str = "scolarite.annee";
/// Date d'entrée dans l'établissement, ISO.
pub const CLE_ENTREE: &str = "scolarite.date_entree";

fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

/// Ce qu'une attestation sait de son élève (lu au socle 2 à la date d'émission).
#[derive(Debug, Clone)]
pub struct DonneesAttestation {
    pub nom: String,
    pub prenom: String,
    pub naissance: NaiveDate,
    pub niveau: Option<String>,
    pub annee_scolaire: Option<String>,
    pub date_entree: NaiveDate,
    pub majeur: bool,
    pub personne_eleve_id: Option<Uuid>,
}

impl DonneesAttestation {
    /// Données figées, dans l'ordre des clés du contrat. Aucune classe, aucune adresse.
    pub fn en_donnees(&self) -> Vec<DonneeRendu> {
        let mut v = vec![
            DonneeRendu {
                cle: CLE_NOM.into(),
                valeur: self.nom.clone(),
            },
            DonneeRendu {
                cle: CLE_PRENOM.into(),
                valeur: self.prenom.clone(),
            },
            DonneeRendu {
                cle: CLE_NAISSANCE.into(),
                valeur: self.naissance.format("%Y-%m-%d").to_string(),
            },
            DonneeRendu {
                cle: CLE_ENTREE.into(),
                valeur: self.date_entree.format("%Y-%m-%d").to_string(),
            },
        ];
        if let Some(n) = &self.niveau {
            v.push(DonneeRendu {
                cle: CLE_NIVEAU.into(),
                valeur: n.clone(),
            });
        }
        if let Some(a) = &self.annee_scolaire {
            v.push(DonneeRendu {
                cle: CLE_ANNEE.into(),
                valeur: a.clone(),
            });
        }
        v
    }
}

/// Un an est « révolu » : la majorité civile tombe le jour du 18ᵉ anniversaire (29 février → 1ᵉʳ mars hors bissextile).
pub fn est_majeur(naissance: NaiveDate, jour: NaiveDate) -> bool {
    let an = naissance.year() + 18;
    let anniversaire = NaiveDate::from_ymd_opt(an, naissance.month(), naissance.day())
        .or_else(|| NaiveDate::from_ymd_opt(an, 3, 1))
        .unwrap_or(NaiveDate::MAX);
    jour >= anniversaire
}

/// Désignation minimale d'un titulaire pour la vérification publique : `L. DUPONT, né(e) en 2011` (§9.7).
pub fn designation_minimale(prenom: &str, nom: &str, naissance: &str) -> String {
    let initiale: String = prenom
        .trim()
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    let annee = naissance.get(..4).unwrap_or("");
    let nom = nom.trim().to_uppercase();
    match (initiale.is_empty(), annee.is_empty()) {
        (false, false) => format!("{initiale}. {nom}, né(e) en {annee}"),
        (false, true) => format!("{initiale}. {nom}"),
        (true, false) => format!("{nom}, né(e) en {annee}"),
        (true, true) => nom,
    }
}

/// État d'une version telle que le dit la vérification publique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatVerifie {
    Valide,
    /// Une version plus récente existe depuis ce jour (jour de l'établissement).
    Remplace {
        le: NaiveDate,
    },
    Annule {
        le: NaiveDate,
    },
}

/// Réponse publique à un code (§9.7). Rien d'autre que ce que le mode de la nature autorise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReponseVerification {
    /// Code inexistant, mal formé, d'un autre établissement ou d'une nature sans vérification : **une seule réponse**.
    Inconnu,
    /// `authenticite_seule` : « document authentique émis par … le … », sans nature ni identité.
    Authentique {
        emetteur: String,
        emis_le: NaiveDate,
        etat: EtatVerifie,
    },
    /// `complete` : nature, état, empreinte et désignation minimale du titulaire.
    Complete {
        emetteur: String,
        nature_libelle: String,
        emis_le: NaiveDate,
        etat: EtatVerifie,
        empreinte_sha256: String,
        titulaire: String,
    },
}

/// Réponse à la vérification par dépôt (§9.7) : le fichier n'est pas conservé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReponseDepot {
    /// « Identique à l'original émis » (et l'état de la version correspondante).
    Identique {
        emetteur: String,
        emis_le: NaiveDate,
        etat: EtatVerifie,
    },
    /// Lot D6 : le fichier déposé est le **PDF signé** (ou le dossier de preuve) d'un circuit complet. Aucun nom de
    /// signataire n'est jamais rendu : seulement le nombre de signatures et le jour de la complétion.
    IdentiqueSigne {
        emetteur: String,
        emis_le: NaiveDate,
        etat: EtatVerifie,
        nb_signatures: i64,
        complet_le: NaiveDate,
        /// Le dossier de preuve (et non le PDF signé) a été déposé.
        dossier_de_preuve: bool,
    },
    Different,
}

/// (nom de naissance, nom d'usage, prénoms, rang du prénom usuel, prénom d'usage, naissance, précision, identifiant de version).
type LigneIdentite = (
    String,
    Option<String>,
    Vec<String>,
    i16,
    Option<String>,
    Option<NaiveDate>,
    Option<String>,
    Uuid,
);
/// (version, document, nature, mode, état, n° de version, version courante, empreinte, émise le, annulé le, émetteur).
type LigneVerification = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    i32,
    i32,
    String,
    NaiveDate,
    Option<NaiveDate>,
    String,
);
/// (document, état, n° de version, version courante, émise le, annulé le, émetteur, mode).
type LigneDepot = (
    Uuid,
    String,
    i32,
    i32,
    NaiveDate,
    Option<NaiveDate>,
    String,
    String,
);

/// Plafond du dépôt de vérification (§17.7).
pub const DEPOT_MAX_OCTETS: usize = 10 * 1024 * 1024;

/// Adaptateur provisoire du droit de lien `recevoir_documents` (socle 4, Q-D29 : non codé) : les titulaires de
/// l'autorité parentale dont le lien n'est pas suspendu et est actif à `jour`, dans un ordre stable (nom, identifiant).
/// Le jour où le droit existera, **cette seule fonction** change.
pub(crate) async fn titulaires_destinataires(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<Vec<DestinataireDoc>, EntDbError> {
    let lignes: Vec<(Uuid, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT p.id, p.nom, p.prenom, p.user_id IS NOT NULL
           FROM ent_eleve_lien l
           JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2
            AND l.autorite_parentale AND l.suspendu_le IS NULL
            AND (l.date_debut IS NULL OR l.date_debut <= $3)
            AND (l.date_fin IS NULL OR l.date_fin >= $3)
            AND p.fusionnee_dans IS NULL
          ORDER BY p.nom, p.id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    let mut vus = std::collections::HashSet::new();
    Ok(lignes
        .into_iter()
        .filter(|(id, ..)| vus.insert(*id))
        .map(|(id, nom, prenom, compte)| DestinataireDoc {
            personne_id: Some(id),
            agent_id: None,
            nom: Some(nom_complet(prenom.as_deref(), &nom)),
            adresse: None,
            au_coffre: compte,
        })
        .collect())
}

fn nom_complet(prenom: Option<&str>, nom: &str) -> String {
    match prenom.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => format!("{p} {}", nom.trim()),
        None => nom.trim().to_string(),
    }
}

impl PgEnt {
    /// Lit, **à la date `jour`**, ce qu'une attestation sait de l'élève, ou refuse (contrôles bloquants §9.1) :
    /// élève non scolarisé à la date (certificat) ou sans scolarisation en cours ni à venir (attestation d'inscription),
    /// identité sans date de naissance exacte. La classe n'est jamais lue : seul le niveau est imprimé (D-027 B).
    pub(crate) async fn donnees_attestation(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        jour: NaiveDate,
        n: &NatureDecl,
    ) -> Result<DonneesAttestation, EntDbError> {
        let ident: Option<LigneIdentite> =
            sqlx::query_as(
                "SELECT nom_naissance, nom_usage, prenoms, rang_prenom_usuel, prenom_usage, date_naissance, date_naissance_precision, id
                   FROM ent_eleve_identite
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND remplacee_par IS NULL AND effet_du <= $3
                  ORDER BY effet_du DESC LIMIT 1",
            )
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
        let (mut nom_n, mut nom_u, mut prenoms, mut rang, mut prenom_u, naissance, precision, version_id) =
            ident.ok_or_else(introuvable)?;
        // Socle 2, E7 : une version sensible dépassée est scellée ; une attestation d'une période
        // passée est un usage légitime de l'état civil d'alors, on rend ses valeurs.
        if nom_n == domain_eleve::MARQUEUR_SCELLE {
            let v = crate::eleve_identite_scellee::desceller_identite_courte(&mut *conn, etab, version_id).await?;
            nom_n = v.get("nom_naissance").cloned().unwrap_or(nom_n);
            nom_u = v.get("nom_usage").cloned();
            prenoms = v.get("prenoms").map(|p| domain_eleve::decoder_prenoms(p)).unwrap_or(prenoms);
            rang = v.get("rang_prenom_usuel").and_then(|r| r.parse().ok()).unwrap_or(rang);
            prenom_u = v.get("prenom_usage").cloned();
        }
        let naissance = match (naissance, precision.as_deref()) {
            (Some(d), Some("jour") | None) => d,
            _ => {
                return Err(refus(
                    RefusMetier::Conflit,
                    "identité incomplète : la date de naissance exacte de l'élève est absente",
                ))
            }
        };
        let nom = nom_u.filter(|x| !x.trim().is_empty()).unwrap_or(nom_n);
        let prenom = prenom_u
            .filter(|x| !x.trim().is_empty())
            .or_else(|| prenoms.get((rang.max(1) - 1) as usize).cloned())
            .or_else(|| prenoms.first().cloned())
            .ok_or_else(|| refus(RefusMetier::Conflit, "identité incomplète : prénom absent"))?;
        // Scolarisation : [entrée, sortie[ (la date de sortie est le premier jour hors établissement).
        let a_venir = n.code == ATTESTATION_INSCRIPTION.code;
        let scol: Option<(NaiveDate,)> = sqlx::query_as(
            "SELECT date_entree FROM ent_eleve_scolarisation
              WHERE etablissement_id = $1 AND eleve_id = $2
                AND ($4 OR date_entree <= $3)
                AND (date_sortie IS NULL OR date_sortie > $3)
              ORDER BY date_entree DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .bind(a_venir)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let (date_entree,) = scol.ok_or_else(|| {
            refus(
                RefusMetier::Conflit,
                "l'élève n'est pas scolarisé dans l'établissement à cette date",
            )
        })?;
        // Niveau et année : la courante, à défaut (attestation d'inscription) l'inscription prévue.
        let niveau: Option<(Option<String>, String)> = sqlx::query_as(
            "SELECT niveau, annee_scolaire FROM ent_eleve_scolarite_annee
              WHERE etablissement_id = $1 AND eleve_id = $2
                AND (statut = 'courante' OR ($3 AND statut IN ('projetee', 'reinscription')))
              ORDER BY (statut = 'courante') DESC, annee_scolaire DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve)
        .bind(a_venir)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let personne_eleve: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2 AND fusionnee_dans IS NULL")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
        Ok(DonneesAttestation {
            nom: nom.trim().to_string(),
            prenom: prenom.trim().to_string(),
            naissance,
            niveau: niveau
                .as_ref()
                .and_then(|(n, _)| n.clone())
                .filter(|x| !x.trim().is_empty()),
            annee_scolaire: niveau.map(|(_, a)| a),
            date_entree,
            majeur: est_majeur(naissance, jour),
            personne_eleve_id: personne_eleve.map(|p| p.0),
        })
    }

    /// Adresse postale **imprimable** d'une personne : jamais celle d'une personne dont une adresse est protégée
    /// (ADM-25, D7) — l'adresse parquée scellée du socle 4 n'est jamais relue ici, et la fiche ne la porte pas.
    pub async fn adresse_postale_permise(
        &self,
        etab: Uuid,
        personne: Uuid,
    ) -> Result<Option<String>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(Option<String>, bool)> = sqlx::query_as(
            "SELECT p.adresse, EXISTS (SELECT 1 FROM ent_adresse_protegee a
                                        WHERE a.etablissement_id = p.etablissement_id AND a.personne_id = p.id)
               FROM ent_personne p WHERE p.etablissement_id = $1 AND p.id = $2",
        )
        .bind(etab)
        .bind(personne)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(match r {
            Some((adresse, false)) => adresse
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty()),
            _ => None,
        })
    }

    /// **Certificat de scolarité** : « l'élève X est scolarisé dans l'établissement à la date du jour ». Un geste au
    /// comptoir : contrôles, numéro, code, rendu, cachet, coffre des titulaires, journal, `documents.emis`.
    pub async fn emettre_certificat_scolarite_for_actor(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        moteur: &Moteur<'_>,
    ) -> Result<DocumentEmis, EntDbError> {
        self.emettre_attestation_for_actor(a, &CERTIFICAT_SCOLARITE, eleve_id, moteur)
            .await
    }

    /// **Attestation d'inscription** : « l'élève X est inscrit pour l'année Y en niveau Z » (socle 3).
    pub async fn emettre_attestation_inscription_for_actor(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        moteur: &Moteur<'_>,
    ) -> Result<DocumentEmis, EntDbError> {
        self.emettre_attestation_for_actor(a, &ATTESTATION_INSCRIPTION, eleve_id, moteur)
            .await
    }

    async fn emettre_attestation_for_actor(
        &self,
        a: &Acteur<'_>,
        n: &'static NatureDecl,
        eleve_id: Uuid,
        moteur: &Moteur<'_>,
    ) -> Result<DocumentEmis, EntDbError> {
        // Le droit d'abord : on ne lit rien du dossier de l'élève pour qui ne peut pas émettre.
        for action in
            std::iter::once(&ent_auth::Action::DocumentsEmettre).chain(n.actions_emission.iter())
        {
            if !self.doc_peut_sur_sujet(a, *action, Some(eleve_id)).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
            .bind(a.etablissement_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let d = self
            .donnees_attestation(&mut tx, a.etablissement_id, eleve_id, jour, n)
            .await?;
        // Destinataires : l'élève majeur reçoit son exemplaire ; sinon les titulaires de l'autorité parentale.
        let destinataires = if d.majeur {
            match d.personne_eleve_id {
                Some(p) => {
                    let (prenom, nom, compte): (Option<String>, String, bool) = sqlx::query_as(
                        "SELECT prenom, nom, user_id IS NOT NULL FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(a.etablissement_id)
                    .bind(p)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    vec![DestinataireDoc {
                        personne_id: Some(p),
                        agent_id: None,
                        nom: Some(nom_complet(prenom.as_deref(), &nom)),
                        adresse: None,
                        au_coffre: compte,
                    }]
                }
                None => Vec::new(),
            }
        } else {
            titulaires_destinataires(&mut tx, a.etablissement_id, eleve_id, jour).await?
        };
        // Signataire désigné pour la nature : si l'émetteur n'est pas lui, « par délégation » (R-D3, §9.6).
        let designe: Option<(Option<Uuid>,)> = sqlx::query_as(
            "SELECT signataire_defaut_agent_id FROM ent_doc_nature_param WHERE etablissement_id = $1 AND nature_code = $2",
        )
        .bind(a.etablissement_id)
        .bind(n.code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let emetteur: Option<(Uuid,)> = sqlx::query_as(
            "SELECT ag.id FROM ent_agent ag JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE ag.etablissement_id = $1 AND p.user_id = $2 LIMIT 1",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let au_nom_de = match (designe.and_then(|x| x.0), emetteur) {
            (Some(signataire), Some((moi,))) if signataire != moi => Some(signataire),
            (Some(signataire), None) => Some(signataire),
            _ => None,
        };
        let titre = format!("{} — {} {}", n.libelle, d.prenom, d.nom);
        self.emettre_document_for_actor(
            a,
            DemandeEmission {
                nature_code: n.code.to_string(),
                sujet: SujetDoc::Eleve(eleve_id),
                titre,
                annee_scolaire_id: None,
                donnees: d.en_donnees(),
                destinataires,
                source: None,
                au_nom_de_agent_id: au_nom_de,
                langue: "fr".to_string(),
            },
            moteur,
        )
        .await
    }

    /// Données figées d'une version, **rouvertes** (champs scellés) : sert la réémission et la vérification publique.
    pub(crate) async fn donnees_figees(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        version: Uuid,
    ) -> Result<Vec<DonneeRendu>, EntDbError> {
        let lignes: Vec<(String, String)> = sqlx::query_as(
            "SELECT cle, valeur FROM ent_doc_version_donnee WHERE etablissement_id = $1 AND version_id = $2 ORDER BY cle",
        )
        .bind(etab)
        .bind(version)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        let coffre = ChampsSensibles::depuis_env();
        lignes
            .into_iter()
            .map(|(cle, v)| {
                coffre
                    .ouvrir("ent_doc_version_donnee", "valeur", version, &v)
                    .map(|valeur| DonneeRendu { cle, valeur })
                    .map_err(EntDbError::Domain)
            })
            .collect()
    }

    // ───────────────────────── Vérification publique (§9.7, §17.7) ─────────────────────────

    /// Vérification **par code** sur la page publique de l'établissement de l'adresse (D-012). Sans session : aucun
    /// acteur, la seule entrée est l'établissement de l'hôte. Un code mal formé, inexistant, d'un autre établissement
    /// ou d'une nature sans vérification donne **la même réponse** ([`ReponseVerification::Inconnu`]). Le numéro
    /// séquentiel n'est jamais accepté comme code. Le débit est limité par la route (adresse et code).
    pub async fn verifier_code_public(
        &self,
        etab: Uuid,
        saisie: &str,
    ) -> Result<ReponseVerification, EntDbError> {
        let Some(code) = normaliser_code(saisie) else {
            return Ok(ReponseVerification::Inconnu);
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne: Option<LigneVerification> =
            sqlx::query_as(
                "SELECT v.id, d.id, d.nature_code, d.verification_publique, d.etat, v.numero_version, d.version_courante,
                        v.empreinte_sha256, ent_aujourdhui_etab(v.etablissement_id, v.emise_le),
                        CASE WHEN d.annule_le IS NULL THEN NULL ELSE ent_aujourdhui_etab(d.etablissement_id, d.annule_le) END,
                        e.raison_sociale
                   FROM ent_doc_version v
                   JOIN ent_doc_document d ON d.etablissement_id = v.etablissement_id AND d.id = v.document_id
                   JOIN ent_doc_entite e ON e.etablissement_id = d.etablissement_id AND e.id = d.entite_id
                  WHERE v.etablissement_id = $1 AND v.code_verification = $2",
            )
            .bind(etab)
            .bind(&code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        let Some((
            version_id,
            document_id,
            nature_code,
            mode,
            etat_doc,
            num,
            courante,
            empreinte,
            emis_le,
            annule_le,
            emetteur,
        )) = ligne
        else {
            tx.commit().await.map_err(db)?;
            return Ok(ReponseVerification::Inconnu);
        };
        let Some(mode) = Verification::parse(&mode).filter(|m| *m != Verification::Aucune) else {
            tx.commit().await.map_err(db)?;
            return Ok(ReponseVerification::Inconnu);
        };
        let etat = etat_verifie(
            &mut tx,
            etab,
            document_id,
            &etat_doc,
            num,
            courante,
            annule_le,
        )
        .await?;
        let reponse = match mode {
            Verification::AuthenticiteSeule => ReponseVerification::Authentique {
                emetteur,
                emis_le,
                etat,
            },
            Verification::Complete => {
                let donnees = self.donnees_figees(&mut tx, etab, version_id).await?;
                let valeur = |cle: &str| {
                    donnees
                        .iter()
                        .find(|d| d.cle == cle)
                        .map(|d| d.valeur.as_str())
                        .unwrap_or("")
                };
                ReponseVerification::Complete {
                    emetteur,
                    nature_libelle: nature(&nature_code)
                        .map(|n| n.libelle.to_string())
                        .unwrap_or_default(),
                    emis_le,
                    etat,
                    empreinte_sha256: empreinte,
                    titulaire: designation_minimale(
                        valeur(CLE_PRENOM),
                        valeur(CLE_NOM),
                        valeur(CLE_NAISSANCE),
                    ),
                }
            }
            Verification::Aucune => ReponseVerification::Inconnu,
        };
        tx.commit().await.map_err(db)?;
        Ok(reponse)
    }

    /// Vérification **par dépôt** du PDF reçu : l'empreinte est calculée sur les octets (plafond 10 Mio, rien n'est
    /// décodé ni conservé) puis comparée aux empreintes des versions et des exemplaires émis. Une nature sans
    /// vérification publique répond comme un fichier inconnu.
    pub async fn verifier_par_depot(
        &self,
        etab: Uuid,
        octets: &[u8],
    ) -> Result<ReponseDepot, EntDbError> {
        if octets.is_empty() || octets.len() > DEPOT_MAX_OCTETS {
            return Ok(ReponseDepot::Different);
        }
        let sha = ent_object::sha256_hex(octets);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne: Option<LigneDepot> = sqlx::query_as(
            "SELECT d.id, d.etat, v.numero_version, d.version_courante, ent_aujourdhui_etab(v.etablissement_id, v.emise_le),
                    CASE WHEN d.annule_le IS NULL THEN NULL ELSE ent_aujourdhui_etab(d.etablissement_id, d.annule_le) END,
                    e.raison_sociale, d.verification_publique
               FROM ent_doc_version v
               JOIN ent_doc_document d ON d.etablissement_id = v.etablissement_id AND d.id = v.document_id
               JOIN ent_doc_entite e ON e.etablissement_id = d.etablissement_id AND e.id = d.entite_id
              WHERE v.etablissement_id = $1 AND d.verification_publique <> 'aucune'
                AND (v.empreinte_sha256 = $2
                     OR EXISTS (SELECT 1 FROM ent_doc_exemplaire x
                                 WHERE x.etablissement_id = v.etablissement_id AND x.version_id = v.id AND x.empreinte_sha256 = $2))
              ORDER BY v.numero_version DESC LIMIT 1",
        )
        .bind(etab)
        .bind(&sha)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((document_id, etat_doc, num, courante, emis_le, annule_le, emetteur, _mode)) =
            ligne
        else {
            // Lot D6 : le fichier est peut-être un PDF signé ou un dossier de preuve.
            let signe = sqlx::query(
                "SELECT d.id, d.etat, v.numero_version, d.version_courante,
                        ent_aujourdhui_etab(v.etablissement_id, v.emise_le) AS emis_le,
                        CASE WHEN d.annule_le IS NULL THEN NULL ELSE ent_aujourdhui_etab(d.etablissement_id, d.annule_le) END AS annule_le,
                        e.raison_sociale, c.id AS circuit_id,
                        ent_aujourdhui_etab(c.etablissement_id, c.clos_le) AS complet_le, (c.preuve_sha256 = $2) AS preuve
                   FROM ent_doc_circuit c
                   JOIN ent_doc_version v ON v.etablissement_id = c.etablissement_id AND v.id = c.version_id
                   JOIN ent_doc_document d ON d.etablissement_id = c.etablissement_id AND d.id = c.document_id
                   JOIN ent_doc_entite e ON e.etablissement_id = d.etablissement_id AND e.id = d.entite_id
                  WHERE c.etablissement_id = $1 AND c.etat = 'complet' AND d.verification_publique <> 'aucune'
                    AND (c.pdf_signe_sha256 = $2 OR c.preuve_sha256 = $2)
                  LIMIT 1",
            )
            .bind(etab)
            .bind(&sha)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some(r) = signe else {
                tx.commit().await.map_err(db)?;
                return Ok(ReponseDepot::Different);
            };
            use sqlx::Row;
            let circuit_id: Uuid = r.get("circuit_id");
            let (nb,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_doc_signature WHERE etablissement_id = $1 AND circuit_id = $2",
            )
            .bind(etab)
            .bind(circuit_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let etat = etat_verifie(
                &mut tx,
                etab,
                r.get("id"),
                &r.get::<String, _>("etat"),
                r.get("numero_version"),
                r.get("version_courante"),
                r.get("annule_le"),
            )
            .await?;
            tx.commit().await.map_err(db)?;
            return Ok(ReponseDepot::IdentiqueSigne {
                emetteur: r.get("raison_sociale"),
                emis_le: r.get("emis_le"),
                etat,
                nb_signatures: nb,
                complet_le: r.get("complet_le"),
                dossier_de_preuve: r.get("preuve"),
            });
        };
        let etat = etat_verifie(
            &mut tx,
            etab,
            document_id,
            &etat_doc,
            num,
            courante,
            annule_le,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ReponseDepot::Identique {
            emetteur,
            emis_le,
            etat,
        })
    }
}

/// État d'une version : annulée (le document l'est), remplacée (une version plus récente existe, avec son jour), ou
/// valide.
async fn etat_verifie(
    conn: &mut PgConnection,
    etab: Uuid,
    document_id: Uuid,
    etat_doc: &str,
    numero_version: i32,
    courante: i32,
    annule_le: Option<NaiveDate>,
) -> Result<EtatVerifie, EntDbError> {
    if etat_doc == "annule" {
        return Ok(EtatVerifie::Annule {
            le: annule_le.unwrap_or(NaiveDate::MIN),
        });
    }
    if numero_version < courante {
        let (le,): (NaiveDate,) = sqlx::query_as(
            "SELECT ent_aujourdhui_etab(etablissement_id, emise_le) FROM ent_doc_version
              WHERE etablissement_id = $1 AND document_id = $2 AND numero_version = $3",
        )
        .bind(etab)
        .bind(document_id)
        .bind(numero_version + 1)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        return Ok(EtatVerifie::Remplace { le });
    }
    Ok(EtatVerifie::Valide)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    #[test]
    fn majorite_le_jour_du_dix_huitieme_anniversaire() {
        assert!(!est_majeur(d(2008, 11, 15), d(2026, 11, 14)));
        assert!(est_majeur(d(2008, 11, 14), d(2026, 11, 14)));
        assert!(est_majeur(d(2008, 11, 13), d(2026, 11, 14)));
        // Né un 29 février : majeur le 1ᵉʳ mars d'une année non bissextile.
        assert!(!est_majeur(d(2008, 2, 29), d(2026, 2, 28)));
        assert!(est_majeur(d(2008, 2, 29), d(2026, 3, 1)));
    }

    #[test]
    fn designation_minimale_sans_donnee_en_trop() {
        assert_eq!(
            designation_minimale("Laura", "Dupont", "2011-05-17"),
            "L. DUPONT, né(e) en 2011"
        );
        assert_eq!(
            designation_minimale("élodie", "Martin-Çelik", "2010-01-02"),
            "É. MARTIN-ÇELIK, né(e) en 2010"
        );
        assert_eq!(designation_minimale("", "Dupont", ""), "DUPONT");
        // Ni jour, ni mois, ni prénom complet.
        let s = designation_minimale("Laura", "Dupont", "2011-05-17");
        assert!(!s.contains("05") && !s.contains("17") && !s.contains("aura"));
    }

    #[test]
    fn donnees_sans_classe_ni_adresse() {
        let x = DonneesAttestation {
            nom: "POTTER".into(),
            prenom: "Harry".into(),
            naissance: d(2012, 3, 10),
            niveau: Some("4e".into()),
            annee_scolaire: Some("2026-2027".into()),
            date_entree: d(2026, 9, 1),
            majeur: false,
            personne_eleve_id: None,
        };
        let cles: Vec<_> = x.en_donnees().into_iter().map(|d| d.cle).collect();
        assert_eq!(
            cles,
            [
                CLE_NOM,
                CLE_PRENOM,
                CLE_NAISSANCE,
                CLE_ENTREE,
                CLE_NIVEAU,
                CLE_ANNEE
            ]
        );
        assert!(cles
            .iter()
            .all(|c| !c.contains("classe") && !c.contains("adresse")));
    }
}
