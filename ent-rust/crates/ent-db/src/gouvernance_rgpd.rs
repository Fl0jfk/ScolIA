//! **Gouvernance RGPD** (socle 27, lot G2) : sous-traitants, mentions d'information, page « Vos données »
//! (côté serveur), points ouverts et état « prêt pour les données réelles ».
//!
//! * **Sous-traitants** (§8.8) : la liste de la **plateforme** (tables sans établissement, écrites par la
//!   console éditeur à venir : aucune route utilisateur) et ceux **propres à l'établissement**. Un nouveau
//!   sous-traitant est annoncé avec un délai d'objection de 30 jours (S-547).
//! * **Mentions** (§8.9) : texte **généré** depuis le registre, la politique de conservation et les
//!   sous-traitants ; une version publiée garde l'empreinte du texte publié.
//! * **« Vos données »** : page générique, par catégorie de public, sans donnée d'autrui ; la partie
//!   personnelle (extrait) vient avec G3.
//! * **Points ouverts** : tout ce qu'il reste à faire, avec sa gravité. **Le DPO absent n'en bloque
//!   aucun** (avertissement) ; seul le registre, les mentions, les sous-traitants et les écarts conditionnent
//!   l'état « prêt pour les données réelles ». ScolIA ne dit jamais « conforme » (R-RG9).

use crate::conservation_regles::EtatAffichage;
use crate::registre_rgpd::{
    documents_tx, soustraitants_plateforme, FicheRegistre, MESURES_SECURITE, TRANSFERTS_HORS_UE,
};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, Role};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// Délai d'objection à un nouveau sous-traitant, en jours (S-547).
pub const DELAI_OBJECTION_JOURS: i64 = 30;

fn texte_court(v: &str, max: usize) -> Option<String> {
    let v = v.trim();
    (!v.is_empty() && v.chars().count() <= max && !v.chars().any(char::is_control))
        .then(|| v.to_string())
}

// ---------------------------------------------------------------------------------------------
// Sous-traitants
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct SousTraitantVue {
    pub nom: String,
    pub service: String,
    pub region: Option<String>,
    pub pays: String,
    pub garanties: Option<String>,
    pub accord_version: Option<String>,
    pub categories: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnnonceVue {
    pub id: Uuid,
    pub soustraitant: String,
    pub nature: String,
    pub annonce_le: NaiveDate,
    pub effet_le: NaiveDate,
    /// Le délai d'objection court encore.
    pub objection_possible: bool,
    /// Cet établissement a déjà objecté.
    pub objection_deposee: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SousTraitantLocalVue {
    pub nom: String,
    pub service: String,
    pub region: Option<String>,
    pub accord_connu: bool,
}

/// Les sous-traitants vus par un établissement.
#[derive(Clone, Debug, Serialize)]
pub struct SousTraitants {
    pub plateforme: Vec<SousTraitantVue>,
    pub annonces: Vec<AnnonceVue>,
    pub locaux: Vec<SousTraitantLocalVue>,
}

/// Saisie d'un sous-traitant de plateforme.
#[derive(Clone, Debug)]
pub struct SaisieSousTraitant<'a> {
    pub nom: &'a str,
    pub service: &'a str,
    pub region: Option<&'a str>,
    pub pays: &'a str,
    pub garanties: Option<&'a str>,
    pub accord_version: Option<&'a str>,
    pub accord_le: Option<NaiveDate>,
    pub entree_le: NaiveDate,
    pub categories: &'a [&'a str],
}

impl PgEnt {
    /// **Appel système** (console plateforme, à venir) : enregistre un sous-traitant de la plateforme. Aucune
    /// route utilisateur ne l'expose ; aucun droit d'établissement ne le permet.
    #[cfg_attr(not(test), allow(dead_code))] // appelé par la console plateforme (à venir) ; les tests l'exercent
    pub(crate) async fn declarer_soustraitant_plateforme(
        &self,
        s: SaisieSousTraitant<'_>,
    ) -> Result<Uuid, EntDbError> {
        let nom = texte_court(s.nom, 200).ok_or(invalide("nom invalide"))?;
        let service = texte_court(s.service, 300).ok_or(invalide("service invalide"))?;
        let pays = texte_court(s.pays, 100)
            .filter(|p| p.chars().count() >= 2)
            .ok_or(invalide("pays invalide"))?;
        for c in s.categories {
            if crate::conservation::categorie(c).is_none() {
                return Err(invalide("catégorie de données inconnue"));
            }
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO rgp_soustraitant (id, nom, service, region, pays, garanties, accord_version, accord_le, entree_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(nom)
        .bind(service)
        .bind(s.region.and_then(|r| texte_court(r, 100)))
        .bind(pays)
        .bind(s.garanties.and_then(|r| texte_court(r, 1000)))
        .bind(s.accord_version.and_then(|r| texte_court(r, 60)))
        .bind(s.accord_le)
        .bind(s.entree_le)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for c in s.categories {
            sqlx::query("INSERT INTO rgp_soustraitant_donnee (soustraitant_id, categorie_code) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(id)
                .bind(*c)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(id)
    }

    /// **Appel système** : annonce l'entrée, la sortie ou le changement d'un sous-traitant. L'effet n'arrive
    /// pas avant le délai d'objection (30 jours, S-547).
    #[cfg_attr(not(test), allow(dead_code))] // idem
    pub(crate) async fn annoncer_soustraitant(
        &self,
        soustraitant_id: Uuid,
        nature: &str,
        annonce_le: NaiveDate,
        effet_le: NaiveDate,
    ) -> Result<Uuid, EntDbError> {
        if !matches!(nature, "entree" | "sortie" | "changement") {
            return Err(invalide("nature d'annonce inconnue"));
        }
        if effet_le < annonce_le + Duration::days(DELAI_OBJECTION_JOURS) {
            return Err(invalide(
                "l'effet doit suivre l'annonce d'au moins 30 jours (délai d'objection)",
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO rgp_soustraitant_annonce (id, soustraitant_id, nature, annonce_le, effet_le) VALUES ($1, $2, $3, $4, $5)")
            .bind(id)
            .bind(soustraitant_id)
            .bind(nature)
            .bind(annonce_le)
            .bind(effet_le)
            .execute(self.pool())
            .await
            .map_err(db)?;
        Ok(id)
    }

    /// Sous-traitants de la plateforme, annonces en cours et sous-traitants propres à l'établissement.
    /// Droit `conformite.lire`.
    pub async fn sous_traitants(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<SousTraitants, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query(
            "SELECT s.nom, s.service, s.region, s.pays, s.garanties, s.accord_version,
                    COALESCE((SELECT array_agg(d.categorie_code ORDER BY d.categorie_code)
                                FROM rgp_soustraitant_donnee d WHERE d.soustraitant_id = s.id), ARRAY[]::text[]) AS cats
               FROM rgp_soustraitant s
              WHERE s.entree_le <= $1 AND (s.sortie_le IS NULL OR s.sortie_le >= $1) ORDER BY s.nom, s.service",
        )
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let plateforme = rows
            .iter()
            .map(|r| SousTraitantVue {
                nom: r.get("nom"),
                service: r.get("service"),
                region: r.get("region"),
                pays: r.get("pays"),
                garanties: r.get("garanties"),
                accord_version: r.get("accord_version"),
                categories: r.get("cats"),
            })
            .collect();
        let rows = sqlx::query(
            "SELECT a.id, s.nom, a.nature, a.annonce_le, a.effet_le,
                    EXISTS (SELECT 1 FROM rgp_soustraitant_objection o WHERE o.annonce_id = a.id AND o.etablissement_id = $2) AS objectee
               FROM rgp_soustraitant_annonce a JOIN rgp_soustraitant s ON s.id = a.soustraitant_id
              WHERE a.effet_le >= $1 ORDER BY a.effet_le, s.nom",
        )
        .bind(jour)
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let annonces = rows
            .iter()
            .map(|r| AnnonceVue {
                id: r.get("id"),
                soustraitant: r.get("nom"),
                nature: r.get("nature"),
                annonce_le: r.get("annonce_le"),
                effet_le: r.get("effet_le"),
                objection_possible: true,
                objection_deposee: r.get("objectee"),
            })
            .collect();
        let rows = sqlx::query("SELECT nom, service, region, accord_connu FROM rgp_soustraitant_local WHERE etablissement_id = $1 ORDER BY nom, service")
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let locaux = rows
            .iter()
            .map(|r| SousTraitantLocalVue {
                nom: r.get("nom"),
                service: r.get("service"),
                region: r.get("region"),
                accord_connu: r.get("accord_connu"),
            })
            .collect();
        Ok(SousTraitants {
            plateforme,
            annonces,
            locaux,
        })
    }

    /// Déclare un sous-traitant propre à l'établissement (RGP-180 et suivants). Droit `soustraitants.gerer`.
    #[allow(clippy::too_many_arguments)]
    pub async fn declarer_soustraitant_local(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        nom: &str,
        service: &str,
        region: Option<&str>,
        accord_connu: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::SoustraitantsGerer)
            .await?;
        let nom = texte_court(nom, 200).ok_or(invalide("nom invalide"))?;
        let service = texte_court(service, 300).ok_or(invalide("service invalide"))?;
        let region = region.and_then(|r| texte_court(r, 100));
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "INSERT INTO rgp_soustraitant_local (id, etablissement_id, nom, service, region, accord_connu, cree_par_compte_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (etablissement_id, nom, service) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(nom)
        .bind(service)
        .bind(region)
        .bind(accord_connu)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier(
                "ce sous-traitant est déjà déclaré",
            ));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Objecte à l'annonce d'un sous-traitant, tant que le délai d'objection court. Droit `soustraitants.gerer`.
    /// Une seule objection par établissement et par annonce.
    pub async fn objecter_annonce(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        annonce_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::SoustraitantsGerer)
            .await?;
        let motif = texte_court(motif, 1000)
            .filter(|m| m.chars().count() >= 3)
            .ok_or(invalide("motif d'objection obligatoire"))?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let effet: Option<NaiveDate> =
            sqlx::query_scalar("SELECT effet_le FROM rgp_soustraitant_annonce WHERE id = $1")
                .bind(annonce_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let effet = effet.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if effet < jour {
            return Err(EntDbError::ConflitDossier("le délai d'objection est échu"));
        }
        let r = sqlx::query(
            "INSERT INTO rgp_soustraitant_objection (id, etablissement_id, annonce_id, motif, par_compte_id)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (etablissement_id, annonce_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annonce_id)
        .bind(motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier(
                "une objection existe déjà pour cette annonce",
            ));
        }
        tx.commit().await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// Mentions d'information
// ---------------------------------------------------------------------------------------------

/// Publics des mentions (§8.9).
pub const PUBLICS_MENTION: &[&str] = &[
    "familles",
    "eleves_college",
    "eleves_lycee",
    "personnels",
    "visiteurs",
    "candidats",
];

/// Publics sans lesquels l'établissement n'est pas prêt pour des données réelles.
const PUBLICS_REQUIS: &[&str] = &["familles", "personnels"];

#[derive(Clone, Debug, Serialize)]
pub struct SectionMention {
    pub titre: String,
    pub lignes: Vec<String>,
}

/// Mention générée pour un public.
#[derive(Clone, Debug, Serialize)]
pub struct Mention {
    pub public: String,
    pub sections: Vec<SectionMention>,
    pub empreinte: String,
    pub version_publiee: Option<i32>,
    pub publiee_le: Option<DateTime<Utc>>,
    /// Le texte généré est celui de la dernière version publiée.
    pub a_jour: bool,
}

/// Traitements qui concernent chaque public (codes des traitements de base).
fn traitements_du_public(public: &str) -> &'static [&'static str] {
    match public {
        "familles" => &[
            "scolarite_dossier",
            "inscriptions",
            "vie_scolaire_assiduite",
            "discipline",
            "evaluations_bulletins",
            "cahier_textes",
            "communication",
            "finances_familles",
            "restauration_internat",
            "sorties_voyages",
            "sante",
            "comptes_securite",
        ],
        "eleves_college" | "eleves_lycee" => &[
            "scolarite_dossier",
            "vie_scolaire_assiduite",
            "discipline",
            "evaluations_bulletins",
            "cahier_textes",
            "communication",
            "restauration_internat",
            "sorties_voyages",
            "sante",
            "comptes_securite",
        ],
        "personnels" => &[
            "personnels",
            "comptes_securite",
            "communication",
            "organisation_pilotage",
            "conformite",
        ],
        "candidats" => &["inscriptions"],
        _ => &["comptes_securite"],
    }
}

fn empreinte_texte(sections: &[SectionMention]) -> String {
    let mut h = Sha256::new();
    for s in sections {
        h.update(s.titre.as_bytes());
        h.update([0u8]);
        for l in &s.lignes {
            h.update(l.as_bytes());
            h.update([0u8]);
        }
        h.update([1u8]);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn duree_texte(m: Option<i32>) -> String {
    match m {
        None => "non fixée".into(),
        Some(0) => "aucune".into(),
        Some(n) if n % 12 == 0 => format!("{} an(s)", n / 12),
        Some(n) => format!("{n} mois"),
    }
}

fn ligne_fiche(f: &FicheRegistre) -> Vec<String> {
    let mut l = vec![
        format!("Finalité : {}", f.fiche.finalite),
        format!("Base légale : {}", base_legale_texte(&f.base_legale)),
        format!("Données : {}", f.fiche.categories_donnees.join(", ")),
        format!(
            "Destinataires internes : {}",
            f.fiche.destinataires_internes.join(", ")
        ),
    ];
    if !f.fiche.destinataires_externes.is_empty() {
        l.push(format!(
            "Destinataires externes : {}",
            f.fiche.destinataires_externes.join(", ")
        ));
    }
    if !f.fiche.soustraitants.is_empty() {
        l.push(format!(
            "Sous-traitants : {}",
            f.fiche.soustraitants.join(" ; ")
        ));
    }
    for d in &f.fiche.durees {
        let etat = match d.etat {
            EtatAffichage::Validee => "",
            _ => " (durée proposée, non validée)",
        };
        l.push(format!(
            "Conservation — {} : utilité courante {}, conservation intermédiaire {}{etat}",
            d.libelle,
            duree_texte(d.utilite_mois),
            duree_texte(d.intermediaire_mois)
        ));
    }
    l
}

fn base_legale_texte(b: &str) -> &'static str {
    match b {
        "mission_interet_public" => "mission d'intérêt public",
        "obligation_legale" => "obligation légale",
        "contrat" => "exécution d'un contrat",
        "consentement" => "consentement",
        "interet_legitime" => "intérêt légitime",
        _ => "sauvegarde des intérêts vitaux",
    }
}

impl PgEnt {
    /// Texte généré de la mention d'un public, à partir du registre, de la politique et du DPO. Un texte
    /// non validé est signalé comme tel ; jamais « conforme ».
    async fn mention_tx(
        &self,
        c: &mut sqlx::PgConnection,
        etab: Uuid,
        public: &str,
    ) -> Result<Mention, EntDbError> {
        if !PUBLICS_MENTION.contains(&public) {
            return Err(invalide("public inconnu"));
        }
        let vue = self.vue_registre_tx(c, etab).await?;
        let row = sqlx::query(
            "SELECT dpo_nom, dpo_contact FROM rgp_gouvernance WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *c)
        .await
        .map_err(db)?;
        let dpo: Option<(String, String)> = row.and_then(|r| {
            match (
                r.get::<Option<String>, _>("dpo_nom"),
                r.get::<Option<String>, _>("dpo_contact"),
            ) {
                (Some(n), Some(c)) => Some((n, c)),
                _ => None,
            }
        });
        let mut sections = vec![SectionMention {
            titre: "Qui traite vos données".into(),
            lignes: vec![
                "Votre établissement est responsable du traitement de vos données ; la plateforme ScolIA agit comme sous-traitant (article 28 du RGPD).".into(),
                match &dpo {
                    Some((n, c)) => format!("Délégué à la protection des données : {n} — {c}."),
                    None => "Aucun délégué à la protection des données n'est désigné à ce jour : adressez-vous à la direction de l'établissement.".into(),
                },
            ],
        }];
        for code in traitements_du_public(public) {
            let Some(f) = vue
                .registre
                .fiches
                .iter()
                .find(|f| f.fiche.code == *code && f.fiche.actif)
            else {
                continue;
            };
            sections.push(SectionMention {
                titre: f.fiche.libelle.clone(),
                lignes: ligne_fiche(f),
            });
        }
        sections.push(SectionMention {
            titre: "Sécurité et transferts".into(),
            lignes: MESURES_SECURITE
                .iter()
                .map(|s| s.to_string())
                .chain([TRANSFERTS_HORS_UE.to_string()])
                .collect(),
        });
        sections.push(SectionMention {
            titre: "Vos droits".into(),
            lignes: vec![
                "Vous pouvez demander l'accès à vos données, leur rectification, leur effacement, leur limitation, vous opposer à un traitement ou demander leur portabilité, lorsque la loi le permet.".into(),
                "Vous pouvez introduire une réclamation auprès de la CNIL.".into(),
            ],
        });
        let empreinte = empreinte_texte(&sections);
        let pub_row = sqlx::query("SELECT version, empreinte, publiee_le FROM rgp_mention WHERE etablissement_id = $1 AND public = $2 ORDER BY version DESC LIMIT 1")
            .bind(etab)
            .bind(public)
            .fetch_optional(&mut *c)
            .await
            .map_err(db)?;
        let (version_publiee, publiee_le, a_jour) = match pub_row {
            Some(r) => (
                Some(r.get::<i32, _>("version")),
                Some(r.get::<DateTime<Utc>, _>("publiee_le")),
                r.get::<String, _>("empreinte") == empreinte,
            ),
            None => (None, None, false),
        };
        Ok(Mention {
            public: public.to_string(),
            sections,
            empreinte,
            version_publiee,
            publiee_le,
            a_jour,
        })
    }

    /// Mention générée pour un public (RGP-210 et suivants). Droit `conformite.lire`.
    pub async fn mention_generee(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        public: &str,
    ) -> Result<Mention, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        self.mention_tx(&mut tx, etab, public).await
    }

    /// Publie une version de la mention d'un public. Droit `conformite.valider`. Exige que **toutes les fiches
    /// actives soient validées** : on ne publie pas un texte dont le registre est en cours de revue. Idempotent :
    /// republier un texte identique ne crée pas de version.
    pub async fn publier_mention(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        public: &str,
    ) -> Result<Mention, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteValider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let vue = self.vue_registre_tx(&mut tx, etab).await?;
        let pas_valides: Vec<_> = vue
            .registre
            .fiches
            .iter()
            .filter(|f| f.fiche.actif && f.etat != "valide")
            .map(|f| f.fiche.code.clone())
            .collect();
        if !pas_valides.is_empty() {
            return Err(EntDbError::ConflitDossierDetail(format!(
                "le registre doit être validé avant de publier une mention ; fiches concernées : {}",
                pas_valides.join(", ")
            )));
        }
        let m = self.mention_tx(&mut tx, etab, public).await?;
        if m.a_jour {
            return Ok(m);
        }
        let version: i32 = sqlx::query_scalar("SELECT COALESCE(max(version), 0) + 1 FROM rgp_mention WHERE etablissement_id = $1 AND public = $2")
            .bind(etab)
            .bind(public)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("INSERT INTO rgp_mention (id, etablissement_id, public, version, empreinte, publiee_par_compte_id) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(public)
            .bind(version)
            .bind(&m.empreinte)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let m = self.mention_tx(&mut tx, etab, public).await?;
        tx.commit().await?;
        Ok(m)
    }
}

// ---------------------------------------------------------------------------------------------
// Page « Vos données » (côté serveur)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct PageVosDonnees {
    /// `familles`, `eleves` ou `personnels` : la catégorie de public servie à ce rôle.
    pub public: String,
    pub dpo_contact: Option<String>,
    pub rubriques: Vec<SectionMention>,
    pub droits: Vec<String>,
    /// La mention de ce public est publiée et à jour.
    pub mention_publiee: bool,
    /// Texte honnête : tant que la mention n'est pas publiée, la page l'indique.
    pub avertissement: Option<String>,
}

impl PgEnt {
    /// Page « Vos données » pour le rôle de la session. **Aucun droit particulier** : tout membre
    /// authentifié de l'établissement (le rôle vient de la session, jamais d'un paramètre) voit ce que
    /// l'établissement fait des données de son public. La page est **générique** : elle ne contient aucune
    /// donnée de personne ; l'extrait personnel vient avec le guichet des droits (G3).
    pub async fn vos_donnees(&self, etab: Uuid, role: &str) -> Result<PageVosDonnees, EntDbError> {
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let (public, mention_public) = match r {
            Role::Parent => ("familles", "familles"),
            Role::Eleve => ("eleves", "eleves_lycee"),
            _ => ("personnels", "personnels"),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let m = self.mention_tx(&mut tx, etab, mention_public).await?;
        let dpo: Option<String> = sqlx::query_scalar(
            "SELECT dpo_contact FROM rgp_gouvernance WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .flatten();
        let mut rubriques = m.sections.clone();
        let droits = rubriques.pop().map(|s| s.lignes).unwrap_or_default();
        let publiee = m.version_publiee.is_some() && m.a_jour;
        Ok(PageVosDonnees {
            public: public.into(),
            dpo_contact: dpo,
            rubriques,
            droits,
            mention_publiee: publiee,
            avertissement: (!publiee).then(|| {
                "Ces informations sont en cours de validation par l'établissement.".to_string()
            }),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Points ouverts et état « prêt pour les données réelles »
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gravite {
    /// Empêche l'état « prêt pour les données réelles ».
    Bloquant,
    /// À traiter, mais n'empêche rien d'autre que ce qu'il dit (purge réelle pour le DPO absent).
    Avertissement,
}

#[derive(Clone, Debug, Serialize)]
pub struct PointOuvert {
    pub code: &'static str,
    pub gravite: Gravite,
    pub libelle: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct EtatPret {
    pub pret: bool,
    pub points: Vec<PointOuvert>,
    pub texte: String,
}

fn point(code: &'static str, gravite: Gravite, libelle: impl Into<String>) -> PointOuvert {
    PointOuvert {
        code,
        gravite,
        libelle: libelle.into(),
    }
}

impl PgEnt {
    /// Points ouverts de conformité et état « prêt pour les données réelles ». Droit `conformite.lire`.
    ///
    /// Le **DPO absent** donne un avertissement (« en attente de DPO », seule la purge réelle attend) et ne
    /// bloque rien. **Port vers la mise en service (socle 28, MS2)** : la porte `donnees_reelles` lit `pret`.
    pub async fn points_ouverts(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<EtatPret, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let vue = self.vue_registre_tx(&mut tx, etab).await?;
        let mut p: Vec<PointOuvert> = Vec::new();
        // Gouvernance et conservation.
        if !vue.politique.dpo_nomme {
            p.push(point("dpo_absent", Gravite::Avertissement, "Aucun DPO n'est nommé : règles de conservation en attente de DPO ; seule la destruction réelle de données attend."));
        }
        if !vue.politique.non_chargees.is_empty() {
            p.push(point(
                "catalogue_non_charge",
                Gravite::Bloquant,
                format!(
                    "Le catalogue de conservation n'est pas chargé ({} catégorie(s)).",
                    vue.politique.non_chargees.len()
                ),
            ));
        } else {
            let non_validees = vue.politique.regles.len() - vue.politique.nb_validees;
            if non_validees > 0 {
                p.push(point("regles_non_validees", Gravite::Avertissement, format!("{non_validees} règle(s) de conservation non validée(s) : aucune destruction tant qu'elles ne le sont pas.")));
            }
        }
        // Registre.
        if vue
            .registre
            .fiches
            .iter()
            .all(|f| f.etat == "non_initialise")
        {
            p.push(point(
                "registre_non_initialise",
                Gravite::Bloquant,
                "Le registre des traitements n'est pas initialisé.",
            ));
        } else {
            let a_completer = vue.registre.nb_a_completer;
            if a_completer > 0 {
                p.push(point(
                    "fiches_a_completer",
                    Gravite::Bloquant,
                    format!("{a_completer} fiche(s) du registre à compléter ou à valider."),
                ));
            }
            if vue.registre.nb_a_revoir > 0 {
                p.push(point(
                    "fiches_a_revoir",
                    Gravite::Bloquant,
                    format!(
                        "{} fiche(s) du registre à revoir (écart détecté).",
                        vue.registre.nb_a_revoir
                    ),
                ));
            }
        }
        let ecarts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM rgp_ecart WHERE etablissement_id = $1 AND traite_le IS NULL",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if ecarts > 0 {
            p.push(point(
                "ecarts_ouverts",
                Gravite::Bloquant,
                format!("{ecarts} écart(s) du registre non traité(s)."),
            ));
        }
        // Sous-traitants.
        if soustraitants_plateforme(&mut tx, jour).await?.is_empty() {
            p.push(point(
                "soustraitants_plateforme_non_renseignes",
                Gravite::Bloquant,
                "La liste des sous-traitants de la plateforme n'est pas renseignée.",
            ));
        }
        let sans_accord: i64 = sqlx::query_scalar("SELECT count(*) FROM rgp_soustraitant_local WHERE etablissement_id = $1 AND NOT accord_connu")
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if sans_accord > 0 {
            p.push(point("soustraitants_locaux_sans_accord", Gravite::Avertissement, format!("{sans_accord} sous-traitant(s) propre(s) à l'établissement sans accord de traitement connu.")));
        }
        // Mentions.
        for public in PUBLICS_MENTION {
            let m = self.mention_tx(&mut tx, etab, public).await?;
            if m.version_publiee.is_none() {
                let g = if PUBLICS_REQUIS.contains(public) {
                    Gravite::Bloquant
                } else {
                    Gravite::Avertissement
                };
                p.push(point(
                    "mention_non_publiee",
                    g,
                    format!("Mention d'information non publiée pour le public « {public} »."),
                ));
            } else if !m.a_jour {
                let g = if PUBLICS_REQUIS.contains(public) {
                    Gravite::Bloquant
                } else {
                    Gravite::Avertissement
                };
                p.push(point("mention_perimee", g, format!("La mention « {public} » publiée ne correspond plus au registre : à republier.")));
            }
        }
        // Documents.
        let manquants: Vec<String> = documents_tx(&mut tx, etab)
            .await?
            .into_iter()
            .filter(|(_, _, e)| e == "manquant")
            .map(|(c, _, _)| c)
            .collect();
        if !manquants.is_empty() {
            p.push(point(
                "documents_manquants",
                Gravite::Avertissement,
                format!(
                    "Documents de conformité non renseignés : {}.",
                    manquants.join(", ")
                ),
            ));
        }
        let pret = !p.iter().any(|x| x.gravite == Gravite::Bloquant);
        let texte = if pret {
            "Prêt pour les données réelles : les points bloquants sont traités. Les avertissements restent à suivre ; ce constat décrit l'état du paramétrage, il ne vaut pas attestation de conformité.".to_string()
        } else {
            format!(
                "Pas encore prêt pour les données réelles : {} point(s) bloquant(s).",
                p.iter().filter(|x| x.gravite == Gravite::Bloquant).count()
            )
        };
        Ok(EtatPret {
            pret,
            points: p,
            texte,
        })
    }
}
