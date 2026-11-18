//! Socle 24, lot F1 — **le service unique de facturation** (R-F1, R-F25, R-F27, §19.1).
//!
//! Il n'y a qu'une logique financière, deux entrées :
//!
//! * le chemin `/finances/*` (`finances.rs`, `finances_sepa.rs`, `finances_reactions.rs`) ;
//! * le chemin historique `/facturation/*` (`service.rs`, `fac_repo.rs`), devenu **adaptateur** : ses
//!   routes gardent leur contrat JSON (test d'or `or_facturation_historique`), mais chaque geste passe
//!   par les mêmes briques que le chemin neuf : droits fins (`ent_auth::Action::Facturation*`),
//!   journal financier, événements `facturation.*` sans montant, écritures confinées à ce service.
//!
//! **Périmètre du service (R-F1)** : les seuls fichiers qui écrivent dans `ent_fac_*`, `ent_fin_*`
//! et `ent_foyer_iban_histo` sont ceux de [`FICHIERS_DU_SERVICE`]. Le test
//! `ent-db/tests/garde_finances_ecritures.rs` lit les sources et échoue si un autre module écrit
//! dans ces tables : les autres domaines **demandent** (commandes de ce service), ils n'écrivent
//! pas. Aucune exception : le mandat SEPA signé à l'inscription passe par
//! [`crate::facturation_mandats`].
//!
//! **Journal (R-F27)** : chaque geste écrit une ligne d'`ent_fin_journal` (ajout seul, migration
//! 521) dans la transaction du geste : qui, quelle casquette, par quel chemin, quoi, sur quel
//! objet, pour quel motif. Le montant n'est écrit qu'ici (droit `facturation.journal`).
//!
//! **Événements (R-F25)** : publiés sous le nom canonique `facturation.*` ; l'ancien nom `fac.*`
//! reste lisible par alias (`ent_events::equivalents_facturation`). La charge d'un événement
//! financier ne contient **aucun montant, aucun nom, aucun IBAN** : [`payload_neutre`] retire ces
//! clés quel que soit l'appelant, c'est donc une propriété du service et non une habitude.

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::finances::db;
use crate::service::EntDbError;

/// Fichiers de `ent-db/src` autorisés à écrire dans les tables financières (garde R-F1). Toute
/// nouvelle écriture financière s'ajoute **dans un de ces fichiers** (ou ici, avec une ligne de
/// justification) : le test de garde échoue sinon.
pub const FICHIERS_DU_SERVICE: &[&str] = &[
    "fac_repo.rs",
    "facturation_comptes.rs",
    "facturation_consequences.rs",
    "facturation_mandats.rs",
    "facturation_payeurs.rs",
    "facturation_service.rs",
    "finances.rs",
    "finances_reactions.rs",
    "finances_sepa.rs",
    "iban_histo_repo.rs",
];

/// Chemin par lequel un geste est arrivé (colonne `ent_fin_journal.chemin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chemin {
    /// Routes `/finances/*`.
    Finances,
    /// Routes `/facturation/*` (adaptateurs du service).
    Historique,
    /// Automatisme : consommateur d'événement, rattrapage.
    Systeme,
    /// Vue guichet du secrétariat et de l'accueil.
    Guichet,
    /// Comptes débiteurs, payeurs, prises en charge (lot F2).
    Comptes,
    /// Espace famille.
    Famille,
}

impl Chemin {
    pub fn code(self) -> &'static str {
        match self {
            Self::Finances => "finances",
            Self::Historique => "historique",
            Self::Systeme => "systeme",
            Self::Guichet => "guichet",
            Self::Comptes => "comptes",
            Self::Famille => "famille",
        }
    }
}

/// Une ligne du journal financier, construite avec des méthodes chaînées.
#[derive(Clone, Debug)]
pub struct Acte {
    pub etab: Uuid,
    pub acteur: Option<Uuid>,
    pub casquette: String,
    pub chemin: Chemin,
    pub action: String,
    pub objet_type: String,
    pub objet_id: Option<Uuid>,
    pub foyer_id: Option<Uuid>,
    pub compte_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub montant_centimes: Option<i64>,
    pub motif: Option<String>,
    pub detail: Option<String>,
}

impl Acte {
    /// `action` : verbe pointé (`facture.emettre`) ; `objet_type` : `facture`, `mandat`, `compte`…
    pub fn new(
        etab: Uuid,
        acteur: Option<Uuid>,
        casquette: &str,
        chemin: Chemin,
        action: &str,
        objet_type: &str,
    ) -> Self {
        Self {
            etab,
            acteur,
            casquette: casquette.to_string(),
            chemin,
            action: action.to_string(),
            objet_type: objet_type.to_string(),
            objet_id: None,
            foyer_id: None,
            compte_id: None,
            eleve_id: None,
            montant_centimes: None,
            motif: None,
            detail: None,
        }
    }

    /// Geste d'un automatisme (aucun acteur humain).
    pub fn systeme(etab: Uuid, chemin: Chemin, action: &str, objet_type: &str) -> Self {
        Self::new(etab, None, "systeme", chemin, action, objet_type)
    }

    pub fn objet(mut self, id: Uuid) -> Self {
        self.objet_id = Some(id);
        self
    }
    pub fn foyer(mut self, id: Uuid) -> Self {
        self.foyer_id = Some(id);
        self
    }
    pub fn compte(mut self, id: Uuid) -> Self {
        self.compte_id = Some(id);
        self
    }
    pub fn eleve(mut self, id: Uuid) -> Self {
        self.eleve_id = Some(id);
        self
    }
    pub fn montant(mut self, centimes: i64) -> Self {
        self.montant_centimes = Some(centimes);
        self
    }
    pub fn motif(mut self, motif: &str) -> Self {
        self.motif = coupe(motif, 500);
        self
    }
    pub fn detail(mut self, detail: &str) -> Self {
        self.detail = coupe(detail, 500);
        self
    }
}

fn coupe(s: &str, max: usize) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.chars().take(max).collect())
}

/// Écrit la ligne de journal **dans la transaction du geste** (ajout seul, migration 521).
pub async fn journaliser_tx(conn: &mut PgConnection, a: &Acte) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_fin_journal (id, etablissement_id, acteur_user_id, casquette, chemin, action,
                                      objet_type, objet_id, foyer_id, compte_id, eleve_id,
                                      montant_centimes, motif, detail)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(id)
    .bind(a.etab)
    .bind(a.acteur)
    .bind(&a.casquette)
    .bind(a.chemin.code())
    .bind(&a.action)
    .bind(&a.objet_type)
    .bind(a.objet_id)
    .bind(a.foyer_id)
    .bind(a.compte_id)
    .bind(a.eleve_id)
    .bind(a.montant_centimes)
    .bind(&a.motif)
    .bind(&a.detail)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// Variante pour les dépôts qui rendent `DomainError` (`fac_repo`) : même ligne, même transaction.
pub(crate) async fn journaliser_domain_tx(
    conn: &mut PgConnection,
    a: &Acte,
) -> Result<(), domain_eleve::DomainError> {
    journaliser_tx(conn, a).await.map(|_| ()).map_err(|e| match e {
        EntDbError::Domain(d) => d,
        _ => domain_eleve::DomainError::Infrastructure,
    })
}

/// Une clé de charge d'événement qui ne doit jamais voyager (R-F25) : montants, noms, coordonnées
/// bancaires, libellés saisis, références de paiement.
pub fn cle_interdite(cle: &str) -> bool {
    let c = cle.to_ascii_lowercase();
    c.contains("centimes")
        || c.starts_with("montant")
        || c.contains("iban")
        || c == "bic"
        || c.starts_with("bic_")
        || c.ends_with("_bic")
        || c == "rum"
        || c == "nom"
        || c == "prenom"
        || c.ends_with("_nom")
        || c == "libelle"
        || c == "titulaire"
        || c == "reference"
        || c == "email"
        || c == "telephone"
        || c == "adresse"
        || c == "empreinte"
}

/// **Liste blanche par type d'événement** (revue M7, puis suite du lot F1-F2) : pour chaque type canonique
/// (`facturation.*`, `fin.*` rendu canonique, et les deux événements `eleve.*` que le service des finances
/// publie), la liste **exacte** des clés qui peuvent voyager — identifiants, codes, dates, compteurs. Tout le
/// reste est retiré, même sous un nom qu'aucune règle ne prévoyait (restes à payer, cumuls, valeurs avant/après,
/// empreintes…), et une clé autorisée pour un type ne l'est pas pour un autre. Un type **absent** de la table est
/// fermé : sa charge est vide (ajouter la ligne ici en publiant un nouveau type).
///
/// Les clés des objets imbriqués (`lignes: [{ ligne_id }]`) sont filtrées par la liste du même type.
pub const CLES_PAR_TYPE: &[(&str, &[&str])] = &[
    ("facturation.brouillon_cree", &["foyer_id", "type_piece", "origine", "annee_scolaire", "numero", "lignes"]),
    ("facturation.ligne_ajoutee", &["foyer_id", "lignes", "origine", "type_piece", "facture_origine_id"]),
    ("facturation.piece_emise", &["facture_id", "foyer_id", "numero", "echeances"]),
    ("facturation.avoir_emis", &["avoir_id", "facture_id", "foyer_id", "numero"]),
    ("facturation.echeancier_pose", &["numero", "nb", "mode"]),
    ("facturation.encaissement_enregistre", &["encaissement_id", "moyen", "quittance_id", "quittance_numero"]),
    ("facturation.quittance_emise", &["quittance_id", "numero", "encaissement_id"]),
    ("facturation.piece_soldee", &["facture_id", "foyer_id", "lettrage", "numero"]),
    ("facturation.mandat_modifie", &["foyer_id", "statut", "prelevements_annules", "prelevements_deja_exportes"]),
    ("facturation.prelevement_prepare", &["prelevement_id", "date_execution", "end_to_end_id", "prelevements"]),
    ("facturation.prelevements_exportes", &["date_execution", "debut", "fin", "nb", "format"]),
    ("facturation.prelevement_execute", &["prelevement_id", "encaissement_id", "quittance_id", "date_execution", "nb"]),
    (
        "facturation.prelevement_rejete",
        &["prelevement_id", "facture_id", "foyer_id", "statut", "motif_rejet", "code", "suite", "frais_facture_id"],
    ),
    (
        "facturation.rappel_envoye",
        &["rappel_id", "echeance_id", "kind", "canal", "fil_id", "relance_id", "facture_id", "foyer_id", "palier"],
    ),
    ("facturation.prenotification_emise", &["foyer_id", "date_execution", "prelevements"]),
    ("facturation.reglement_enregistre", &["reglement_id", "foyer_id", "moyen"]),
    ("facturation.relance_proposee", &["relance_id", "facture_id", "foyer_id", "palier"]),
    ("facturation.relance_annulee", &["relance_id", "facture_id", "palier"]),
    ("facturation.export_comptable", &["export_id", "numero", "debut", "fin", "ecritures"]),
    ("facturation.crediteur_modifie", &["ics_change", "nom_change"]),
    ("facturation.parametres_modifies", &["crediteur_modifie"]),
    ("facturation.article_enregistre", &["article_id"]),
    ("facturation.tarif_defini", &["tarif_id", "annee_scolaire", "article_id"]),
    ("facturation.reduction_accordee", &["reduction_id"]),
    ("facturation.generation_annuelle", &["annee_scolaire", "lignes", "factures"]),
    ("facturation.consequences_proposees", &["groupe_id", "categorie", "date_effet", "propositions"]),
    ("facturation.tache_payeur", &["nature", "categorie", "compte_id"]),
    // Événements d'élève que le service des finances publie par le même chemin (lus par `finances_reactions`).
    ("eleve.regime_changed", &["eleve_id", "ancien", "nouveau", "date_effet"]),
    ("eleve.sortie", &["eleve_id", "date"]),
];

/// Clés autorisées pour un type (nom canonique ou ancien nom `fac.*` / `fin.*`) ; `None` : type inconnu, fermé.
pub fn cles_du_type(type_name: &str) -> Option<&'static [&'static str]> {
    let canon = ent_events::canonique_facturation(type_name);
    CLES_PAR_TYPE.iter().find(|(t, _)| *t == canon).map(|(_, c)| *c)
}

/// Retire de la charge d'un événement financier tout ce que R-F25 interdit : récursif. Deux filtres :
/// la liste noire [`cle_interdite`] (montants, noms, coordonnées) **et** la liste blanche **du type**
/// ([`CLES_PAR_TYPE`]). Un type inconnu donne une charge vide.
pub fn payload_neutre_pour(type_name: &str, v: Value) -> Value {
    let Some(cles) = cles_du_type(type_name) else {
        return Value::Object(Default::default());
    };
    filtrer(cles, v)
}

fn filtrer(cles: &[&str], v: Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.into_iter()
                .filter(|(k, _)| !cle_interdite(k) && cles.contains(&k.as_str()))
                .map(|(k, x)| (k, filtrer(cles, x)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.into_iter().map(|x| filtrer(cles, x)).collect()),
        autre => autre,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn payload_neutre_retire_montants_noms_et_coordonnees() {
        let v = payload_neutre_pour(
            "facturation.piece_emise",
            json!({
                "facture_id": "a", "foyer_id": "b", "numero": "F2030-000001",
                "total_centimes": 5500, "montant_du_centimes": 1, "iban_avant": "FR76…",
                "rum": "SCOLIA1", "libelle": "Cantine", "titulaire": "M. X", "echeances": 3,
                "reference": "CHQ-1", "deja_plus_ce": 4850, "reste": 12, "iban_masque": "FR76 ••• 1234", "restant": 5,
            }),
        );
        assert_eq!(v, json!({ "facture_id": "a", "foyer_id": "b", "numero": "F2030-000001", "echeances": 3 }));
    }

    #[test]
    fn la_liste_blanche_est_propre_a_chaque_type() {
        // Une clé permise pour un type est retirée d'un autre : `echeances` appartient à la pièce émise, pas au règlement.
        let charge = json!({ "foyer_id": "b", "moyen": "cheque", "echeances": 3, "palier": 2, "ancien": "x" });
        assert_eq!(
            payload_neutre_pour("facturation.reglement_enregistre", charge.clone()),
            json!({ "foyer_id": "b", "moyen": "cheque" })
        );
        assert_eq!(
            payload_neutre_pour("facturation.relance_proposee", charge.clone()),
            json!({ "foyer_id": "b", "palier": 2 })
        );
        // Les identifiants d'un type ne passent pas dans un autre non plus (plus de règle « `*_id` partout »).
        assert_eq!(
            payload_neutre_pour("facturation.article_enregistre", json!({ "article_id": "a", "facture_id": "b", "foyer_id": "c" })),
            json!({ "article_id": "a" })
        );
        // Imbriqué : filtré par la liste du même type.
        assert_eq!(
            payload_neutre_pour(
                "facturation.generation_annuelle",
                json!({ "lignes": [{ "lignes": 1, "nom": "Y", "montant_centimes": 3 }], "annee_scolaire": "2030-2031" })
            ),
            json!({ "lignes": [{ "lignes": 1 }], "annee_scolaire": "2030-2031" })
        );
    }

    #[test]
    fn un_ancien_nom_donne_la_liste_du_nom_canonique() {
        assert_eq!(cles_du_type("fac.reglement"), cles_du_type("facturation.reglement_enregistre"));
        assert!(cles_du_type("fac.reglement").is_some());
        assert_eq!(cles_du_type("fin.crediteur_modifie"), cles_du_type("facturation.crediteur_modifie"));
    }

    #[test]
    fn un_type_inconnu_est_ferme() {
        assert_eq!(payload_neutre_pour("facturation.type_invente", json!({ "id": "a", "foyer_id": "b" })), json!({}));
    }

    #[test]
    fn la_table_ne_contient_aucune_cle_interdite_ni_doublon() {
        let mut vus = std::collections::BTreeSet::new();
        for (t, cles) in CLES_PAR_TYPE {
            assert!(vus.insert(*t), "type en double : {t}");
            assert_eq!(ent_events::canonique_facturation(t), *t, "nom non canonique : {t}");
            assert!(t.starts_with("facturation.") || t.starts_with("eleve."), "{t}");
            for c in *cles {
                assert!(!cle_interdite(c), "clé interdite dans la liste de {t} : {c}");
            }
        }
    }

    /// Garde de complétude : chaque type publié par `publier_finance` ou `payload_neutre_pour` a sa ligne dans
    /// [`CLES_PAR_TYPE`] (sinon sa charge serait vide en silence). Les constantes sont lues dans le source.
    #[test]
    fn chaque_type_publie_a_sa_liste_blanche() {
        let constantes = include_str!("../../ent-events/src/lib.rs");
        let valeur = |nom: &str| -> String {
            let motif = format!("pub const {nom}: &str = \"");
            let i = constantes.find(&motif).unwrap_or_else(|| panic!("constante inconnue : {nom}")) + motif.len();
            constantes[i..i + constantes[i..].find('"').unwrap()].to_string()
        };
        let sources = [
            include_str!("finances.rs"),
            include_str!("finances_sepa.rs"),
            include_str!("facturation_payeurs.rs"),
            include_str!("service.rs"),
        ];
        let mut vus = 0;
        for src in sources {
            for (i, _) in src.match_indices("publier_finance(").chain(src.match_indices("payload_neutre_pour(")) {
                let fin = (i + 400).min(src.len());
                let fin = (fin..=src.len()).find(|k| src.is_char_boundary(*k)).unwrap();
                let fenetre = &src[i..fin];
                let Some(j) = fenetre.find("types::") else { continue };
                let nom: String = fenetre[j + 7..].chars().take_while(|c| c.is_ascii_uppercase() || *c == '_' || c.is_ascii_digit()).collect();
                if nom.is_empty() {
                    continue;
                }
                let v = valeur(&nom);
                assert!(cles_du_type(&v).is_some(), "type publié sans liste blanche : {v} ({nom})");
                vus += 1;
            }
        }
        assert!(vus >= 28, "le scan ne trouve plus les publications ({vus})");
    }

    #[test]
    fn le_service_ne_liste_que_des_fichiers_connus() {
        let mut v: Vec<_> = FICHIERS_DU_SERVICE.to_vec();
        let n = v.len();
        v.sort();
        v.dedup();
        assert_eq!(n, v.len(), "doublon dans FICHIERS_DU_SERVICE");
    }

    #[test]
    fn acte_tronque_motif_et_detail() {
        let a = Acte::systeme(Uuid::new_v4(), Chemin::Systeme, "x.y", "facture")
            .motif(&"m".repeat(900))
            .detail("   ");
        assert_eq!(a.motif.as_deref().map(str::len), Some(500));
        assert_eq!(a.detail, None);
    }
}
