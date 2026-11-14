//! D3 — chiffrement applicatif des champs les plus sensibles (IBAN/BIC/titulaire,
//! fiche santé, dossier psy) : AES-256-GCM, nonce aléatoire (12 o), via `SecretBox`.
//!
//! - **Clé** : dérivée de `ENT_SECRETS_KEY` pour cet usage (`SecretBox::derive`) — la
//!   même clé maîtresse que les secrets OIDC, mais une clé de travail distincte.
//! - **AAD** : `ent-champ:v1:{table}.{colonne}:{id de la ligne}` — un chiffré recopié
//!   dans une autre ligne, une autre colonne ou une autre table ne s'ouvre pas.
//! - **Format** : `enc:v1:` + base64url(nonce ‖ chiffré ‖ tag). Toute valeur sans ce
//!   préfixe est un **héritage en clair** : lue telle quelle, rechiffrée à la prochaine
//!   écriture de la ligne ou par [`rechiffrer_champs_sensibles`].
//! - **Chaîne vide** : conservée vide (pas de chiffré pour « rien ») ; `NULL` conservé.
//! - **Fail-closed** : sans clé (hors labo/test, `connect` refuse déjà de démarrer), toute
//!   écriture ou lecture d'un champ chiffré échoue (`DomainError::Infrastructure`) ; une
//!   valeur n'est jamais écrite en clair faute de clé.
//!
//! Rien de sensible n'apparaît dans les journaux ni les erreurs.

use domain_eleve::DomainError;
use ent_auth::{is_sealed, SecretBox, SEALED_PREFIX};
use sqlx::PgPool;
use uuid::Uuid;

/// Usage de dérivation de la clé (changer la valeur rendrait les données illisibles).
const USAGE_DERIVATION: &str = "champs-sensibles:v1";

/// Champs chiffrés : `(table, colonnes)`. Toutes ont une clé primaire `id UUID`.
pub const CHAMPS_CHIFFRES: &[(&str, &[&str])] = &[
    ("ent_fac_sepa_mandat", &["iban", "bic", "titulaire"]),
    ("ent_foyer_iban_histo", &["iban", "bic", "titulaire"]),
    ("ent_mandat_sepa_demande", &["iban", "bic", "titulaire"]),
    (
        "ent_fiche_sante",
        &[
            "allergies",
            "traitements",
            "contre_indications",
            "medecin_nom",
            "urgence_tel",
            "note_confidentielle",
        ],
    ),
    (
        "ent_psy_dossier",
        &["axes_suivi", "contacts_externes", "note_confidentielle"],
    ),
    ("ent_psy_entretien", &["note_confidentielle"]),
    ("ent_psy_rdv", &["motif", "note_interne"]),
    // S2-11 : PAI, passages à l'infirmerie, comptes rendus d'équipe éducative.
    (
        "ent_pai",
        &["motif", "mesures_accueil", "protocole_urgence", "note_interne"],
    ),
    ("ent_infirmerie_passage", &["note_soins", "soins"]),
    ("ent_equipe_reunion", &["compte_rendu"]),
    // Socle 2, lot E5 : valeurs actuelle et proposée d'une proposition (toujours scellées).
    ("ent_eleve_proposition_champ", &["actuel", "propose"]),
    // Socle 4, lot A0 bis : adresse à protéger parquée, référence de la pièce de protection.
    ("ent_adresse_protegee", &["adresse"]),
    ("ent_personne_reserve", &["ne_pas_solliciter_ref"]),
    ("ent_personne_version", &["valeurs_anterieures"]),
    (
        "ent_inscription_responsable",
        &["adresse_protegee", "ne_pas_solliciter_ref"],
    ),
];

/// Coffre des champs sensibles (clone bon marché).
#[derive(Clone)]
pub struct ChampsSensibles {
    boite: Option<SecretBox>,
}

impl std::fmt::Debug for ChampsSensibles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChampsSensibles")
            .field(
                "cle",
                &if self.boite.is_some() {
                    "[présente]"
                } else {
                    "[absente]"
                },
            )
            .finish()
    }
}

impl ChampsSensibles {
    /// Depuis `ENT_SECRETS_KEY` / `ENT_ENV` : clé de développement publique en
    /// labo/test, **aucune clé** (donc tout échoue) hors labo/test sans clé valide.
    pub fn depuis_env() -> Self {
        Self {
            boite: SecretBox::from_env()
                .ok()
                .map(|b| b.derive(USAGE_DERIVATION)),
        }
    }

    /// À partir d'une clé maîtresse explicite (tests).
    pub fn avec_cle_maitresse(maitresse: &SecretBox) -> Self {
        Self {
            boite: Some(maitresse.derive(USAGE_DERIVATION)),
        }
    }

    /// Sans clé : toute opération sur un champ non vide échoue (fail-closed).
    pub fn sans_cle() -> Self {
        Self { boite: None }
    }

    fn aad(table: &str, colonne: &str, id: Uuid) -> Vec<u8> {
        format!("ent-champ:v1:{table}.{colonne}:{id}").into_bytes()
    }

    /// Chiffre une valeur pour `(table, colonne, id)`. Vide → vide.
    pub fn sceller(
        &self,
        table: &str,
        colonne: &str,
        id: Uuid,
        valeur: &str,
    ) -> Result<String, DomainError> {
        if valeur.is_empty() {
            return Ok(String::new());
        }
        let Some(b) = &self.boite else {
            tracing::error!(
                table,
                colonne,
                "chiffrement des champs sensibles : clé absente (fail-closed)"
            );
            return Err(DomainError::Infrastructure);
        };
        b.seal(valeur, &Self::aad(table, colonne, id)).map_err(|_| {
            tracing::error!(table, colonne, "chiffrement d'un champ sensible impossible");
            DomainError::Infrastructure
        })
    }

    /// Déchiffre ; une valeur sans préfixe (héritage en clair) est rendue telle quelle.
    pub fn ouvrir(
        &self,
        table: &str,
        colonne: &str,
        id: Uuid,
        stocke: &str,
    ) -> Result<String, DomainError> {
        if !is_sealed(stocke) {
            return Ok(stocke.to_string());
        }
        let Some(b) = &self.boite else {
            tracing::error!(
                table,
                colonne,
                "lecture d'un champ chiffré : clé absente (fail-closed)"
            );
            return Err(DomainError::Infrastructure);
        };
        b.open(stocke, &Self::aad(table, colonne, id)).map_err(|_| {
            tracing::error!(table, colonne, %id, "champ chiffré illisible (clé inadaptée, ligne copiée ou donnée altérée)");
            DomainError::Infrastructure
        })
    }

    pub fn sceller_opt(
        &self,
        table: &str,
        colonne: &str,
        id: Uuid,
        valeur: Option<&str>,
    ) -> Result<Option<String>, DomainError> {
        valeur
            .map(|v| self.sceller(table, colonne, id, v))
            .transpose()
    }

    pub fn ouvrir_opt(
        &self,
        table: &str,
        colonne: &str,
        id: Uuid,
        stocke: Option<String>,
    ) -> Result<Option<String>, DomainError> {
        stocke
            .map(|v| self.ouvrir(table, colonne, id, &v))
            .transpose()
    }
}

/// Rapport d'un rechiffrement.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RapportRechiffrement {
    /// Valeurs rechiffrées (héritage en clair → `enc:v1:`).
    pub champs_rechiffres: u64,
    /// Détail `table.colonne` → nombre.
    pub detail: Vec<(String, u64)>,
}

/// Connexion d'un lot de rechiffrement : transaction d'établissement, ou connexion du pool.
enum Conn {
    Tenant(crate::tenant_tx::TenantTx<'static>),
    Pool(sqlx::pool::PoolConnection<sqlx::Postgres>),
}

impl Conn {
    fn connexion(&mut self) -> &mut sqlx::PgConnection {
        match self {
            Self::Tenant(t) => &mut *t,
            Self::Pool(c) => &mut *c,
        }
    }

    async fn valider(self) -> Result<(), sqlx::Error> {
        match self {
            Self::Tenant(t) => t.commit().await,
            Self::Pool(_) => Ok(()),
        }
    }
}

/// Rechiffre toutes les valeurs encore en clair des colonnes de [`CHAMPS_CHIFFRES`].
/// Idempotent, reprenable (traite par lots de 500), sans verrou long : chaque ligne est
/// mise à jour seulement si elle n'a pas changé depuis sa lecture. Les noms de table et
/// de colonne viennent de la constante ci-dessus (jamais d'une entrée utilisateur).
pub async fn rechiffrer_champs_sensibles(
    pool: &PgPool,
    champs: &ChampsSensibles,
) -> Result<RapportRechiffrement, DomainError> {
    let mut rapport = RapportRechiffrement::default();
    // RLS stricte (docs/securite/RLS_STRICTE.md §4) : tâche transverse nommée
    // `ContexteSysteme::Rechiffrement`. Les tables « par établissement » sont traitées **un
    // établissement à la fois**, dans son contexte (`TenantTx`), avec un filtre explicite sur
    // `etablissement_id` (le mode `off` ne filtre pas en base). Les tables sans établissement
    // (aucune aujourd'hui) restent traitées sur le pool.
    let etabs = crate::tenant_tx::etablissements_systeme(pool, crate::tenant_tx::ContexteSysteme::Rechiffrement)
        .await
        .map_err(|e| crate::pg_err::map_sqlx("champs_chiffres", e))?;
    let err = |e| crate::pg_err::map_sqlx("champs_chiffres", e);
    for (table, colonnes) in CHAMPS_CHIFFRES {
        let par_etab = crate::registre_donnees::classer(table)
            .map(|c| c.portee == crate::registre_donnees::Portee::Etablissement)
            .unwrap_or(true);
        // `None` : une seule passe sans contexte (table sans établissement).
        let portees: Vec<Option<Uuid>> = if par_etab { etabs.iter().copied().map(Some).collect() } else { vec![None] };
        for colonne in *colonnes {
            let mut n_col = 0u64;
            for portee in &portees {
                loop {
                    let filtre = if portee.is_some() { "AND etablissement_id = $1 " } else { "" };
                    let sql = format!(
                        "SELECT id, {colonne} FROM {table} \
                         WHERE {colonne} IS NOT NULL AND {colonne} <> '' \
                           AND {colonne} NOT LIKE '{SEALED_PREFIX}%' {filtre}\
                         ORDER BY id LIMIT 500"
                    );
                    let mut conn = match portee {
                        Some(e) => Conn::Tenant(crate::tenant_tx::TenantTx::begin(pool, *e).await.map_err(err)?),
                        None => Conn::Pool(pool.acquire().await.map_err(err)?),
                    };
                    let mut q = sqlx::query_as::<_, (Uuid, String)>(&sql);
                    if let Some(e) = portee {
                        q = q.bind(*e);
                    }
                    let lignes: Vec<(Uuid, String)> = q.fetch_all(conn.connexion()).await.map_err(err)?;
                    if lignes.is_empty() {
                        break;
                    }
                    let mut progres = 0u64;
                    for (id, clair) in lignes {
                        let scelle = champs.sceller(table, colonne, id, &clair)?;
                        let upd = format!(
                            "UPDATE {table} SET {colonne} = $2 WHERE id = $1 AND {colonne} = $3"
                        );
                        let n = sqlx::query(&upd)
                            .bind(id)
                            .bind(&scelle)
                            .bind(&clair)
                            .execute(conn.connexion())
                            .await
                            .map_err(err)?
                            .rows_affected();
                        progres += n;
                    }
                    conn.valider().await.map_err(err)?;
                    n_col += progres;
                    if progres == 0 {
                        // Lignes modifiées en concurrence : la prochaine exécution les reprendra.
                        break;
                    }
                }
            }
            if n_col > 0 {
                rapport.champs_rechiffres += n_col;
                rapport.detail.push((format!("{table}.{colonne}"), n_col));
            }
        }
    }
    Ok(rapport)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coffre() -> ChampsSensibles {
        ChampsSensibles::avec_cle_maitresse(&SecretBox::dev())
    }

    #[test]
    fn aller_retour_et_valeurs_distinctes() {
        let c = coffre();
        let id = Uuid::new_v4();
        let a = c
            .sceller("ent_fiche_sante", "allergies", id, "arachides")
            .unwrap();
        let b = c
            .sceller("ent_fiche_sante", "allergies", id, "arachides")
            .unwrap();
        assert!(a.starts_with("enc:v1:") && !a.contains("arachides"));
        assert_ne!(a, b, "nonce aléatoire");
        assert_eq!(
            c.ouvrir("ent_fiche_sante", "allergies", id, &a).unwrap(),
            "arachides"
        );
    }

    #[test]
    fn aad_autre_ligne_colonne_ou_table_refusee() {
        let c = coffre();
        let id = Uuid::new_v4();
        let s = c.sceller("ent_fiche_sante", "allergies", id, "x").unwrap();
        assert!(c
            .ouvrir("ent_fiche_sante", "allergies", Uuid::new_v4(), &s)
            .is_err());
        assert!(c.ouvrir("ent_fiche_sante", "traitements", id, &s).is_err());
        assert!(c.ouvrir("ent_psy_dossier", "allergies", id, &s).is_err());
    }

    #[test]
    fn heritage_en_clair_relu_et_vide_conserve() {
        let c = coffre();
        let id = Uuid::new_v4();
        assert_eq!(
            c.ouvrir("t", "c", id, "ancienne valeur").unwrap(),
            "ancienne valeur"
        );
        assert_eq!(c.sceller("t", "c", id, "").unwrap(), "");
        assert_eq!(c.sceller_opt("t", "c", id, None).unwrap(), None);
    }

    #[test]
    fn sans_cle_echoue_sans_ecrire_en_clair() {
        let c = ChampsSensibles::sans_cle();
        let id = Uuid::new_v4();
        assert!(c.sceller("t", "c", id, "secret").is_err());
        let s = coffre().sceller("t", "c", id, "secret").unwrap();
        assert!(c.ouvrir("t", "c", id, &s).is_err());
        // Clair hérité : lisible même sans clé (rien à déchiffrer).
        assert_eq!(c.ouvrir("t", "c", id, "clair").unwrap(), "clair");
    }

    #[test]
    fn cle_derivee_distincte_de_la_maitresse() {
        let id = Uuid::new_v4();
        let maitresse = SecretBox::dev();
        let champs = ChampsSensibles::avec_cle_maitresse(&maitresse);
        let s = champs.sceller("t", "c", id, "v").unwrap();
        // La clé maîtresse seule (usage OIDC) n'ouvre pas un champ.
        assert!(maitresse
            .open(&s, &ChampsSensibles::aad("t", "c", id))
            .is_err());
    }
}
