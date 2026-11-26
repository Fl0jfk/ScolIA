//! **Extrait assemblé** d'une demande de droit d'accès (socle 27, palier G3b, lot 1). Réf. :
//! `27_RGPD_ARCHIVES.md` §9.4 (points 3 à 6), §17.4, §26 (RGP-302), R-RG14, R-RG15, R-RG18, R-RG29, R-RG32,
//! R-RG38, R-RG40, R-RG44, R-RG46, CL27-01, CL27-02, CL27-18, M-2711.
//!
//! # Principe
//! Le **contenu d'un extrait n'est jamais stocké en base** (§17.4) : il est reconstruit à la demande depuis les
//! fournisseurs de chaque socle. La base ne garde que la structure (une section par fournisseur : statut, nombre de
//! lignes), l'**empreinte** SHA-256 de chaque section (preuve qu'on relit ce qu'on a assemblé) et les décisions
//! humaines. Si une empreinte diffère entre l'assemblage relu et la lecture ou la remise, l'extrait est **périmé** :
//! une nouvelle version s'assemble et les masquages sont à relire.
//!
//! # Deux moteurs
//! * sujet **élève** : moteur **générique piloté par le registre des tables**. Pour chaque table exportable d'un
//!   fournisseur, les lignes de l'établissement rattachées à l'élève (`eleve_id`, ou la règle de rattachement
//!   indirect déclarée dans [`rattachements_indirects`]), le filtre `export_si` appliqué, les colonnes
//!   `colonnes_hors_export` retirées, les tables `hors_export_par_eleve` sautées. Les champs chiffrés ne sont jamais
//!   rendus en clair (un IBAN est masqué `FR76 **** **** 1234`, PORTABILITE §7). Les tables de **santé** sont exclues
//!   tant que le module santé n'est pas actif (R-RG29, R-RG44) et **listées** « exclues : motif ».
//! * sujet **personne** (parent pour lui-même, personnel) : fournisseurs **écrits** pour les comptes (socle 1), le
//!   dossier des personnes (4), les personnels (6), la messagerie (16 : seulement les messages dont la personne est
//!   **auteur ou destinataire**, M-2711) et les documents (7). Tout autre domaine donne une section **non
//!   couverte** : l'extrait est alors incomplet, jamais présenté comme complet.
//!
//! # Ce qui n'apparaît jamais, et qu'on ne dit pas
//! Les protections (adresse à protéger, « ne pas solliciter », confidentialité renforcée) ne sont **ni rendues ni
//! mentionnées** (R-RG38, R-RG46, CL27-02) : leurs tables et leurs colonnes ne sont jamais lues, et aucune
//! exclusion ne les nomme. Le marquage des données propres de l'autre parent, de l'autre élève ou du rédacteur
//! d'une note est le travail du lot suivant (masquages et relecture humaine).
//!
//! # Règles d'accès
//! * assembler, soumettre au DPO, arbitrer une remise incomplète : `droits.traiter` (le dernier :
//!   `droits.arbitrer`, motivé par section) ;
//! * **lire le contenu** : `droits.traiter`, ou `droits.arbitrer` quand la demande a été **soumise** au DPO
//!   (§12.2 : le DPO ne voit que l'extrait qu'on lui soumet). Toute ouverture du contenu est une consultation
//!   sensible **tracée avant d'être servie** (R-RG8) : l'échec de la trace refuse la lecture ;
//! * aucun établissement ne fuit vers un autre : tout passe par `TenantTx` et refuse en 404 neutre.

use crate::champs_chiffres::{ChampsSensibles, CHAMPS_CHIFFRES};
use crate::conservation::{categorie, categorie_de_table};
use crate::droits_demandes::{
    db, etat_courant, exiger_ouverte, identite_etablie, invalide, journal, qualite_a_la_date,
    qualite_de_soi, texte,
};
use crate::extrait_personne::{fournisseurs_de, hors_extrait, Fournisseur, FOURNISSEURS};
use crate::registre_donnees::{ClasseTable, Portee, REGISTRE};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::{is_sealed, Action};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// Plafond de lignes lues par table et par extrait (garde-fou mémoire : au-delà, la section est en échec).
const LIGNES_MAX_PAR_TABLE: i64 = 50_000;

/// Tables que l'extrait ne lit **jamais** et ne nomme **jamais** : elles décrivent une protection (R-RG38, R-RG46).
const TABLES_JAMAIS: &[&str] = &[
    "ent_adresse_protegee",
    "ent_personne_reserve",
    "ent_eleve_confidentialite",
    "ent_eleve_identite_scellee",
    "ent_personne_version",
];

/// Colonnes que l'extrait ne rend **jamais**, dans aucune table (protections, secrets, clés d'objet).
const COLONNES_JAMAIS: &[&str] = &[
    "adresse_protegee",
    "adresse_a_proteger",
    "ne_pas_solliciter",
    "ne_pas_solliciter_ref",
    "ne_pas_solliciter_le",
    "ne_pas_solliciter_par",
    "ne_pas_solliciter_confirme_le",
    "ne_pas_solliciter_confirme_par",
    "identite_restreinte",
    "identite_restreinte_motif",
    "identite_restreinte_depuis",
    "motif_scelle",
    "valeur_scellee",
    "valeurs_anterieures",
    "cle_recherche",
    "cle_recherche_civil",
    "msg_xid",
    "retire_xid",
];

/// Un nom de colonne qui contient l'un de ces fragments n'est jamais rendu (secrets, clés de stockage).
const FRAGMENTS_JAMAIS: &[&str] = &[
    "password",
    "secret",
    "token",
    "jeton",
    "storage_key",
    "cle_objet",
];

/// Statut d'une section d'extrait.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutSection {
    Ok,
    Echec,
    Vide,
    NonCouverte,
    /// Écartée par règle (santé tant que le module est inactif) : listée, sans rendre l'extrait incomplet.
    Exclue,
}

impl StatutSection {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Echec => "echec",
            Self::Vide => "vide",
            Self::NonCouverte => "non_couverte",
            Self::Exclue => "exclue",
        }
    }
    /// Une section en échec ou non couverte rend l'extrait **incomplet** (R-RG18).
    pub fn incomplete(self) -> bool {
        matches!(self, Self::Echec | Self::NonCouverte)
    }
}

/// Une ligne de contenu : l'identifiant de la ligne (quand la table en a un) et ses valeurs rendues en texte.
#[derive(Clone, Debug, Serialize)]
pub struct LigneContenu {
    pub id: Option<Uuid>,
    /// `(colonne, valeur)`, triées par colonne : le contenu est déterministe (l'empreinte en dépend).
    pub valeurs: Vec<(String, String)>,
}

/// Les lignes d'une table dans une section.
#[derive(Clone, Debug, Serialize)]
pub struct TableContenu {
    pub nom: String,
    pub lignes: Vec<LigneContenu>,
}

/// Ce qui est écarté d'une section, avec la raison (jamais une protection, R-RG46).
#[derive(Clone, Debug, Serialize)]
pub struct Exclusion {
    pub table: String,
    pub motif: String,
}

/// Une section assemblée par un fournisseur.
#[derive(Clone, Debug, Serialize)]
pub struct SectionContenu {
    pub fournisseur: String,
    pub socle: String,
    pub statut: StatutSection,
    pub motif_code: Option<String>,
    pub tables: Vec<TableContenu>,
    pub exclusions: Vec<Exclusion>,
    /// Tables exportables du domaine qui ne se rattachent pas à ce sujet (aucune colonne qui le désigne).
    pub tables_sans_rattachement: usize,
    pub lignes: usize,
    pub empreinte: Option<String>,
}

/// Une durée de conservation communiquée à la personne (catalogue ScolIA, sous réserve des règles de l'établissement).
#[derive(Clone, Debug, Serialize)]
pub struct DureeContenu {
    pub categorie: String,
    pub libelle: String,
    pub utilite_mois: Option<u16>,
    pub intermediaire_mois: Option<u16>,
    pub source: String,
}

/// Le contenu complet d'un extrait, tel qu'il est reconstruit (jamais stocké).
#[derive(Clone, Debug, Serialize)]
pub struct ContenuExtrait {
    pub sujet_type: String,
    pub sujet_id: Uuid,
    pub periode_du: Option<NaiveDate>,
    pub periode_au: Option<NaiveDate>,
    pub sections: Vec<SectionContenu>,
    pub complet: bool,
    /// Empreinte globale : SHA-256 des empreintes de section, dans l'ordre des fournisseurs.
    pub empreinte: String,
    pub nb_lignes: usize,
    /// Destinataires : rôles internes qui lisent les domaines assemblés, destinataires externes déclarés,
    /// sous-traitants de la plateforme (§9.4 point 3).
    pub destinataires_internes: Vec<String>,
    pub destinataires_externes: Vec<String>,
    pub soustraitants: Vec<String>,
    pub durees: Vec<DureeContenu>,
    pub sources: Vec<String>,
    /// Éléments marqués à relire (lot G3b-2) ; vide hors lecture. Ne fait pas partie de l'empreinte.
    pub marquages: Vec<crate::droits_extrait_masquage::MarquageVue>,
}

/// Vue d'une section **sans contenu** (ce que la fiche de la demande affiche).
#[derive(Clone, Debug, Serialize)]
pub struct SectionVue {
    pub fournisseur: String,
    pub socle: String,
    pub statut: String,
    pub motif_code: Option<String>,
    pub lignes: i32,
    /// Remise de cette section incomplète décidée par la direction (R-RG18) : motif écrit.
    pub remise_incomplete_motif: Option<String>,
}

/// Vue d'un extrait **sans contenu** (structure, empreinte, décisions).
#[derive(Clone, Debug, Serialize)]
pub struct ExtraitVue {
    pub id: Uuid,
    pub demande_id: Uuid,
    pub sujet_type: String,
    pub sujet_id: Uuid,
    pub version: i32,
    pub assemble_le: DateTime<Utc>,
    pub complet: bool,
    pub perime: bool,
    pub perime_motif: Option<String>,
    pub nb_sections: i32,
    pub nb_lignes: i32,
    pub empreinte: String,
    pub sections: Vec<SectionVue>,
    pub soumis_dpo: bool,
    pub avis_dpo_code: Option<String>,
    pub avis_dpo_texte: Option<String>,
    /// Vrai quand l'extrait peut partir : complet, ou chaque section incomplète a une décision motivée de la direction.
    pub remissible_en_l_etat: bool,
    /// Marquages restant à décider : aucune remise tant qu'il en reste (RGP-150).
    pub masquages_a_decider: i32,
}

// ---------------------------------------------------------------------------------------------
// Outils purs : rendu des valeurs, masquage, empreintes
// ---------------------------------------------------------------------------------------------

fn identifiant_sur(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Masque un IBAN : `FR76 **** **** 1234` (PORTABILITE §7). Trop court : entièrement masqué.
pub fn masquer_iban(clair: &str) -> String {
    let s: String = clair.chars().filter(|c| !c.is_whitespace()).collect();
    let n = s.chars().count();
    if n < 8 {
        return "**** ****".into();
    }
    let debut: String = s.chars().take(4).collect();
    let fin: String = s.chars().skip(n - 4).collect();
    format!("{debut} **** **** {fin}")
}

/// Rend une valeur JSON en texte lisible, déterministe.
fn rendre(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(true) => "oui".into(),
        Value::Bool(false) => "non".into(),
        Value::Number(n) => n.to_string(),
        autre => autre.to_string(),
    }
}

/// Texte rendu pour un champ **chiffré** : jamais le clair, sauf le masque d'un IBAN (la fin seule).
fn masquer_champ_chiffre(
    champs: &ChampsSensibles,
    table: &str,
    colonne: &str,
    id: Option<Uuid>,
    stocke: &str,
) -> String {
    if colonne == "iban" {
        if let Some(id) = id {
            if let Ok(clair) = champs.ouvrir(table, colonne, id, stocke) {
                return masquer_iban(&clair);
            }
        }
        return "**** **** ****".into();
    }
    "[champ chiffré : non communiqué]".into()
}

/// Empreinte d'une section : SHA-256 du contenu canonique (tables triées, lignes triées, colonnes triées).
fn empreinte_section(tables: &[TableContenu]) -> String {
    let mut h = Sha256::new();
    for t in tables {
        h.update(b"T\x1f");
        h.update(t.nom.as_bytes());
        h.update(b"\x1e");
        for l in &t.lignes {
            for (c, v) in &l.valeurs {
                h.update(c.as_bytes());
                h.update(b"\x1f");
                h.update(v.as_bytes());
                h.update(b"\x1f");
            }
            h.update(b"\x1e");
        }
    }
    hex(&h.finalize())
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|b| format!("{b:02x}")).collect()
}

fn empreinte_globale(sections: &[SectionContenu]) -> String {
    let mut h = Sha256::new();
    for s in sections {
        h.update(s.fournisseur.as_bytes());
        h.update(b"\x1f");
        h.update(s.statut.code().as_bytes());
        h.update(b"\x1f");
        h.update(s.empreinte.as_deref().unwrap_or("-").as_bytes());
        h.update(b"\x1e");
    }
    hex(&h.finalize())
}

// ---------------------------------------------------------------------------------------------
// Règles de lecture
// ---------------------------------------------------------------------------------------------

/// Une règle de lecture : une table, la condition qui la rattache au sujet (alias `x`, paramètres de `p`) et, au
/// choix, la liste blanche des colonnes rendues (sinon toutes, moins ce qui est retiré par le registre).
struct Regle {
    table: &'static str,
    condition: String,
    colonnes: Option<&'static [&'static str]>,
    /// La table n'a pas de colonne `etablissement_id` (compte multi-établissement) : la condition suffit.
    sans_etab: bool,
}

fn regle(table: &'static str, condition: &str) -> Regle {
    Regle {
        table,
        condition: condition.to_string(),
        colonnes: None,
        sans_etab: false,
    }
}

fn regle_cols(table: &'static str, condition: &str, colonnes: &'static [&'static str]) -> Regle {
    Regle {
        table,
        condition: condition.to_string(),
        colonnes: Some(colonnes),
        sans_etab: false,
    }
}

/// Rattachements **indirects** à un élève (la table n'a pas de colonne `eleve_id`) : condition SQL, alias `x`,
/// paramètres de `p` (`p.etab`, `p.sujet`). Toute autre table exportable sans `eleve_id` est comptée dans
/// `tables_sans_rattachement` de sa section : aucune colonne ne la rattache à l'élève (elle relève d'une autre
/// personne : agent, famille payeuse…). Voir le test `chaque_table_exportable_est_rattachee_ou_declaree`.
pub fn rattachements_indirects() -> Vec<(&'static str, &'static str)> {
    const DOSSIER: &str = "x.dossier_id IN (SELECT d.id FROM public.ent_inscription_dossier d \
        WHERE d.etablissement_id = p.etab AND d.eleve_id = p.sujet)";
    const RESULTAT: &str = "x.resultat_id IN (SELECT r.id FROM public.eva_resultat r \
        WHERE r.etablissement_id = p.etab AND r.eleve_id = p.sujet)";
    const SUJET: &str = "x.sujet_eleve_id = p.sujet";
    vec![
        ("ent_eleve", "x.id = p.sujet"),
        ("ent_inscription_accord", DOSSIER),
        ("ent_inscription_condition", DOSSIER),
        ("ent_inscription_decision", DOSSIER),
        ("ent_inscription_piece", DOSSIER),
        ("ent_inscription_proposition_place", DOSSIER),
        ("ent_inscription_responsable", DOSSIER),
        ("ent_inscription_suivi_dossier", DOSSIER),
        ("ent_inscription_voeu", DOSSIER),
        ("eva_correction_demande", RESULTAT),
        ("eva_resultat_item", RESULTAT),
        ("eva_revision_demande", RESULTAT),
        ("ent_cdt_retour", RESULTAT),
        ("ent_acc_decision", SUJET),
        ("ent_acc_desaccord", SUJET),
        ("ent_acc_journal", SUJET),
        (
            "ent_sortie_participant_fait",
            "x.participant_id IN (SELECT sp.id FROM public.ent_sortie_participant sp \
              WHERE sp.etablissement_id = p.etab AND sp.eleve_id = p.sujet)",
        ),
    ]
}

/// Tables de la **messagerie** : traitées à part (M-2711, CL27-18), jamais par le rattachement `eleve_id`.
const TABLES_MESSAGERIE: &[&str] = &[
    "ent_msg",
    "ent_msg_accuse",
    "ent_msg_lecture",
    "ent_msg_piece",
    "ent_msg_telephone",
    "ent_msg_version",
    "ent_msg_fil",
    "ent_msg_fil_eleve",
    "ent_msg_participant",
    "ent_msg_brouillon",
    "ent_msg_lien_decision",
];

/// Tables de finances rattachées à un **foyer payeur** (et non à l'élève) : elles décrivent la famille qui paie,
/// donc aussi l'autre parent (CL27-01) ; elles relèvent de l'extrait de la personne payeuse.
fn est_table_de_foyer_payeur(nom: &str) -> bool {
    matches!(
        nom,
        "ent_fac_facture"
            | "ent_fac_quittance"
            | "ent_fac_sepa_mandat"
            | "ent_fin_compte"
            | "ent_fin_compte_payeur"
            | "ent_fin_reglement"
            | "ent_fin_rejet"
            | "ent_foyer_quotient"
            | "ent_mandat_sepa_demande"
            | "ent_bank_rapprochement"
    )
}

/// Les fils de messagerie que le sujet **lit**, selon le sujet.
fn fils_de_messagerie(sujet_est_eleve: bool) -> String {
    if sujet_est_eleve {
        // Élève : seuls les fils rattachés à l'élève dont le DEMANDEUR est participant (lui-même s'il est
        // l'élève). Les échanges de l'autre parent avec le CPE n'y figurent jamais (M-2711).
        "(SELECT pa.fil_id FROM public.ent_msg_participant pa \
           WHERE pa.etablissement_id = p.etab \
             AND (pa.personne_id = p.dem_personne OR (p.dem_est_sujet AND pa.eleve_id = p.sujet)) \
             AND pa.fil_id IN (SELECT fe.fil_id FROM public.ent_msg_fil_eleve fe \
                                WHERE fe.etablissement_id = p.etab AND fe.eleve_id = p.sujet))"
            .to_string()
    } else {
        "(SELECT pa.fil_id FROM public.ent_msg_participant pa \
           WHERE pa.etablissement_id = p.etab AND pa.personne_id = p.personne)"
            .to_string()
    }
}

fn regles_messagerie(sujet_est_eleve: bool) -> Vec<Regle> {
    let f = fils_de_messagerie(sujet_est_eleve);
    vec![
        regle_cols(
            "ent_msg_fil",
            &format!("x.id IN {f}"),
            &["id", "sujet", "created_at", "last_message_at", "nature", "archive"],
        ),
        regle_cols(
            "ent_msg",
            &format!("x.fil_id IN {f} AND x.retire_le IS NULL"),
            &[
                "id",
                "fil_id",
                "auteur_user_id",
                "auteur_personne_id",
                "corps",
                "created_at",
                "type",
                "modifie_le",
            ],
        ),
        regle_cols(
            "ent_msg_version",
            &format!("x.message_id IN (SELECT m.id FROM public.ent_msg m WHERE m.etablissement_id = p.etab AND m.fil_id IN {f} AND m.retire_le IS NULL)"),
            &["message_id", "version", "corps", "remplace_le"],
        ),
        regle_cols(
            "ent_msg_participant",
            &format!("x.fil_id IN {f}"),
            &["fil_id", "cote", "qualite", "du", "au"],
        ),
        regle_cols(
            "ent_msg_accuse",
            &format!("x.fil_id IN {f} AND x.lecteur_user_id = p.compte"),
            &["message_id", "lu_at"],
        ),
        regle_cols(
            "ent_msg_piece",
            &format!("x.fil_id IN {f}"),
            &["fil_id", "message_id", "nom", "mime_type", "taille_octets", "created_at", "etat"],
        ),
    ]
}

/// Règles du sujet **personne** pour un fournisseur écrit (socles 1, 4, 6, 7, 16).
fn regles_personne(code: &str) -> Vec<Regle> {
    match code {
        "comptes" => vec![
            Regle {
                table: "ent_user",
                condition: "x.id = p.compte".into(),
                colonnes: Some(&["id", "email", "display_name", "telephone", "created_at"]),
                sans_etab: true,
            },
            regle_cols(
                "ent_membership",
                "x.user_id = p.compte",
                &["role", "created_at", "etat", "etat_depuis", "date_fin"],
            ),
            regle_cols(
                "ent_role_attribution",
                "x.user_id = p.compte",
                &["role_id", "debut", "fin", "created_at", "source"],
            ),
            // Les clés de connexion ne sont jamais rendues : seulement leur existence (nom, dates).
            regle_cols(
                "ent_passkey_credential",
                "x.user_id = p.compte",
                &["name", "created_at", "last_used_at"],
            ),
        ],
        "administratif" => vec![
            regle_cols(
                "ent_personne",
                "x.id = p.personne",
                &[
                    "nom",
                    "prenom",
                    "civilite",
                    "email",
                    "telephone",
                    "telephone_2",
                    "adresse",
                    "source",
                    "created_at",
                ],
            ),
            regle_cols(
                "ent_personne_coordonnee",
                "x.personne_id = p.personne",
                &[
                    "type",
                    "valeur",
                    "libelle",
                    "usages",
                    "principal",
                    "verifiee_le",
                    "du",
                    "au",
                    "source",
                ],
            ),
            regle_cols(
                "ent_personne_consentement",
                "x.personne_id = p.personne",
                &[
                    "objet",
                    "reponse",
                    "texte_version",
                    "canal",
                    "du",
                    "retire_le",
                ],
            ),
            regle_cols(
                "ent_personne_preference",
                "x.personne_id = p.personne",
                &[
                    "canal_email",
                    "canal_sms",
                    "courrier_papier",
                    "langue",
                    "accessibilite",
                    "fuseau",
                    "updated_at",
                ],
            ),
            // Les liens de la personne avec des élèves : ses propres liens, jamais ceux d'un autre responsable.
            regle_cols(
                "ent_eleve_lien",
                "(x.personne_id = p.personne OR x.user_id = p.compte)",
                &[
                    "eleve_id",
                    "lien",
                    "autorite_parentale",
                    "garde",
                    "est_payeur",
                    "date_debut",
                    "date_fin",
                    "peut_voir_notes",
                    "peut_voir_vs",
                    "peut_justifier_absence",
                ],
            ),
        ],
        "personnels" => vec![
            regle_cols(
                "ent_agent",
                "x.id = p.agent",
                &[
                    "matricule_interne",
                    "nom_affiche_familles",
                    "personnel_mineur",
                    "photo_visible_collegues",
                    "photo_visible_eleves",
                    "photo_visible_familles",
                    "photo_consentement_le",
                    "source",
                    "cree_le",
                ],
            ),
            regle_cols(
                "ent_agent_fonction",
                "x.agent_id = p.agent",
                &[
                    "fonction_id",
                    "principale",
                    "du",
                    "au",
                    "annule_le",
                    "cree_le",
                ],
            ),
            regle_cols(
                "ent_agent_engagement",
                "x.agent_id = p.agent",
                &[
                    "du",
                    "au",
                    "nature",
                    "employeur_type",
                    "quotite_pct",
                    "jours_travailles",
                    "heures_dues_hebdo",
                    "motif_arrivee",
                    "motif_fin",
                    "installe_le",
                    "annule_le",
                    "cree_le",
                ],
            ),
            regle_cols(
                "ent_agent_identifiant",
                "x.agent_id = p.agent",
                &["systeme", "valeur", "du", "au", "cree_le"],
            ),
            regle_cols(
                "ent_agent_contact_urgence",
                "x.agent_id = p.agent",
                &[
                    "ordre",
                    "nom",
                    "prenom",
                    "lien",
                    "telephone",
                    "du",
                    "au",
                    "cree_le",
                ],
            ),
        ],
        "messagerie" => regles_messagerie(false),
        "documents" => vec![
            regle_cols(
                "ent_doc_document",
                "(x.personne_id = p.personne OR x.agent_id = p.agent)",
                &[
                    "id",
                    "nature_code",
                    "titre",
                    "etat",
                    "origine",
                    "domaine",
                    "numero_annee",
                    "numero",
                    "cree_le",
                    "annule_le",
                ],
            ),
            regle_cols(
                "ent_doc_piece",
                "(x.personne_id = p.personne OR x.agent_id = p.agent)",
                &[
                    "id",
                    "type_code",
                    "deposee_le",
                    "etat",
                    "valable_jusqu_au",
                    "taille_originale",
                    "mime_original",
                    "nb_pages",
                ],
            ),
        ],
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------------------------
// Assemblage
// ---------------------------------------------------------------------------------------------

/// Tout ce que le moteur sait du sujet et de la demande qui l'ouvre.
#[derive(Clone, Debug)]
struct ContexteSujet {
    etab: Uuid,
    sujet_type: String,
    sujet_id: Uuid,
    /// Compte, personne et agent du sujet (sujet personne ou agent).
    compte: Option<Uuid>,
    personne: Option<Uuid>,
    agent: Option<Uuid>,
    /// Personne du **demandeur** (rapprochée ou issue de son compte) : la messagerie d'un élève s'arrête là.
    dem_personne: Option<Uuid>,
    /// Le demandeur est l'élève lui-même (qualité « pour soi »).
    dem_est_sujet: bool,
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
    module_sante: bool,
}

impl ContexteSujet {
    fn est_eleve(&self) -> bool {
        self.sujet_type == "eleve"
    }
}

/// Colonnes des tables lues : `table → [(colonne, type)]`.
type Colonnes = BTreeMap<String, Vec<(String, String)>>;

async fn colonnes_des_tables(
    conn: &mut PgConnection,
    tables: &[&str],
) -> Result<Colonnes, EntDbError> {
    let noms: Vec<String> = tables.iter().map(|t| t.to_string()).collect();
    let lignes: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text
           FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name = ANY($1)
          ORDER BY table_name, ordinal_position",
    )
    .bind(&noms)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut m: Colonnes = BTreeMap::new();
    for (t, c, ty) in lignes {
        m.entry(t).or_default().push((c, ty));
    }
    Ok(m)
}

fn colonne_interdite(c: &str, ty: &str) -> bool {
    ty == "bytea" || COLONNES_JAMAIS.contains(&c) || FRAGMENTS_JAMAIS.iter().any(|f| c.contains(f))
}

/// Colonne d'horodatage qui borne une table à la période demandée : la première présente de cette liste. Une table sans
/// ces colonnes est une table d'**état** (le profil, la fiche) : elle n'est jamais filtrée par la période.
fn colonne_de_periode(cols: &[(String, String)]) -> Option<&'static str> {
    ["cree_le", "created_at", "deposee_le", "date_depot"]
        .into_iter()
        .find(|c| cols.iter().any(|(n, _)| n == c))
}

/// Lit une table selon une règle : lignes rendues en texte, champs chiffrés masqués, colonnes interdites retirées.
async fn lire_table(
    conn: &mut PgConnection,
    ctx: &ContexteSujet,
    champs: &ChampsSensibles,
    regle: &Regle,
    classe: Option<&ClasseTable>,
    cols: &[(String, String)],
    extra: Option<&str>,
) -> Result<TableContenu, EntDbError> {
    let t = regle.table;
    if !identifiant_sur(t) {
        return Err(invalide("table inconnue"));
    }
    // Colonnes rendues : la liste blanche de la règle, sinon toutes moins ce que le registre retire.
    let retirees: BTreeSet<&str> = classe
        .map(|c| c.colonnes_hors_export.iter().copied().collect())
        .unwrap_or_default();
    let rendues: Vec<&str> = match regle.colonnes {
        Some(liste) => liste
            .iter()
            .copied()
            .filter(|c| {
                cols.iter()
                    .any(|(n, ty)| n == c && !colonne_interdite(n, ty))
            })
            .collect(),
        None => cols
            .iter()
            .filter(|(n, ty)| !colonne_interdite(n, ty) && !retirees.contains(n.as_str()))
            .map(|(n, _)| n.as_str())
            .collect(),
    };
    if rendues.is_empty() || !rendues.iter().all(|c| identifiant_sur(c)) {
        return Err(invalide("colonnes inconnues"));
    }
    let objet = rendues
        .iter()
        .map(|c| format!("'{c}', x.{c}"))
        .collect::<Vec<_>>()
        .join(", ");
    // Identifiant de la ligne (pour masquer un champ chiffré et pour les masquages du lot suivant).
    let a_id = cols.iter().any(|(n, ty)| n == "id" && ty == "uuid");
    let id_expr = if a_id { "x.id::text" } else { "NULL::text" };
    let etab = if regle.sans_etab {
        ""
    } else {
        "x.etablissement_id = p.etab AND "
    };
    let export_si = classe
        .and_then(|c| c.export_si)
        .map(|s| format!(" AND ({s})"))
        .unwrap_or_default();
    let periode = match (ctx.du.or(ctx.au), colonne_de_periode(cols)) {
        (Some(_), Some(c)) => {
            format!(" AND (p.du IS NULL OR x.{c} >= p.du) AND (p.au IS NULL OR x.{c} < p.au + 1)")
        }
        _ => String::new(),
    };
    let sql = format!(
        "WITH p AS (SELECT $1::uuid AS etab, $2::uuid AS sujet, $3::uuid AS compte, $4::uuid AS personne, \
                           $5::uuid AS agent, $6::uuid AS dem_personne, $7::bool AS dem_est_sujet, \
                           $8::date AS du, $9::date AS au)
         SELECT {id_expr} AS id, jsonb_build_object({objet})::text AS j
           FROM public.{t} x, p
          WHERE {etab}({cond}){export_si}{periode}{extra}
          ORDER BY 2
          LIMIT {limite}",
        cond = regle.condition,
        extra = extra.unwrap_or(""),
        limite = LIGNES_MAX_PAR_TABLE + 1,
    );
    let rows = sqlx::query(&sql)
        .bind(ctx.etab)
        .bind(ctx.sujet_id)
        .bind(ctx.compte)
        .bind(ctx.personne)
        .bind(ctx.agent)
        .bind(ctx.dem_personne)
        .bind(ctx.dem_est_sujet)
        .bind(ctx.du)
        .bind(ctx.au)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    if rows.len() as i64 > LIGNES_MAX_PAR_TABLE {
        return Err(invalide("table trop volumineuse pour un extrait"));
    }
    let chiffrees: BTreeSet<&str> = CHAMPS_CHIFFRES
        .iter()
        .filter(|(n, _)| *n == t)
        .flat_map(|(_, c)| c.iter().copied())
        .collect();
    let mut lignes = Vec::with_capacity(rows.len());
    for r in rows {
        let id: Option<String> = r.get("id");
        let id = id.and_then(|s| Uuid::parse_str(&s).ok());
        let j: String = r.get("j");
        let Value::Object(m) =
            serde_json::from_str::<Value>(&j).map_err(|_| invalide("ligne illisible"))?
        else {
            return Err(invalide("ligne illisible"));
        };
        let mut valeurs: Vec<(String, String)> = Vec::with_capacity(m.len());
        for (c, v) in &m {
            let rendu = rendre(v);
            let rendu = if chiffrees.contains(c.as_str()) || is_sealed(&rendu) {
                masquer_champ_chiffre(champs, t, c, id, &rendu)
            } else {
                rendu
            };
            valeurs.push((c.clone(), rendu));
        }
        valeurs.sort_by(|a, b| a.0.cmp(&b.0));
        lignes.push(LigneContenu { id, valeurs });
    }
    Ok(TableContenu {
        nom: t.to_string(),
        lignes,
    })
}

fn motif_exclusion_sante() -> String {
    "donnée de santé : module santé inactif".to_string()
}

/// Les tables exportables d'un fournisseur (registre), hors tables techniques ou jamais lues.
fn tables_du_fournisseur(f: &Fournisseur) -> Vec<&'static ClasseTable> {
    REGISTRE
        .iter()
        .filter(|t| {
            matches!(
                t.portee,
                Portee::Etablissement | Portee::CompteMultiEtablissement
            )
        })
        .filter(|t| t.export_portabilite && hors_extrait(t).is_none())
        .filter(|t| fournisseurs_de(t).any(|x| x.code == f.code))
        .filter(|t| !TABLES_JAMAIS.contains(&t.nom))
        .collect()
}

/// Assemble la section d'un fournisseur pour un sujet **élève** (moteur générique du registre).
async fn section_eleve(
    conn: &mut PgConnection,
    ctx: &ContexteSujet,
    champs: &ChampsSensibles,
    f: &Fournisseur,
) -> Result<SectionContenu, EntDbError> {
    let tables = tables_du_fournisseur(f);
    let indirects: BTreeMap<&str, &str> = rattachements_indirects().into_iter().collect();
    let noms: Vec<&str> = tables.iter().map(|t| t.nom).collect();
    let toutes_colonnes = colonnes_des_tables(conn, &noms).await?;
    let mut contenu: Vec<TableContenu> = Vec::new();
    let mut exclusions: Vec<Exclusion> = Vec::new();
    let mut sans_rattachement = 0usize;
    let messagerie = regles_messagerie(true);
    // La messagerie passe par ses règles propres, avant tout rattachement par `eleve_id` (M-2711).
    let mut a_traiter: Vec<(Regle, Option<&'static ClasseTable>)> = Vec::new();
    for t in &tables {
        if t.hors_export_par_eleve.is_some() {
            continue;
        }
        if t.sante && !ctx.module_sante {
            exclusions.push(Exclusion {
                table: t.nom.to_string(),
                motif: motif_exclusion_sante(),
            });
            continue;
        }
        if TABLES_MESSAGERIE.contains(&t.nom) {
            continue;
        }
        if est_table_de_foyer_payeur(t.nom) {
            exclusions.push(Exclusion {
                table: t.nom.to_string(),
                motif:
                    "données du foyer payeur : elles relèvent de l'extrait de la personne payeuse"
                        .into(),
            });
            continue;
        }
        let cols = toutes_colonnes.get(t.nom).cloned().unwrap_or_default();
        if let Some(cond) = indirects.get(t.nom) {
            a_traiter.push((regle(t.nom, cond), Some(*t)));
        } else if cols.iter().any(|(n, ty)| n == "eleve_id" && ty == "uuid") {
            a_traiter.push((regle(t.nom, "x.eleve_id = p.sujet"), Some(*t)));
        } else {
            sans_rattachement += 1;
        }
    }
    if f.code == "messagerie" {
        for r in messagerie {
            a_traiter.push((r, None));
        }
    }
    let cols_msg = if f.code == "messagerie" {
        let n: Vec<&str> = TABLES_MESSAGERIE.to_vec();
        colonnes_des_tables(conn, &n).await?
    } else {
        BTreeMap::new()
    };
    let mut lignes = 0usize;
    for (r, classe) in &a_traiter {
        let cols = toutes_colonnes
            .get(r.table)
            .or_else(|| cols_msg.get(r.table))
            .cloned()
            .unwrap_or_default();
        // Un point de sauvegarde par table : l'échec d'une lecture n'invalide pas la transaction de l'appelant.
        let mut sp = conn.begin().await.map_err(db)?;
        let t = lire_table(
            &mut sp,
            ctx,
            champs,
            r,
            classe.or_else(|| crate::registre_donnees::classer(r.table)),
            &cols,
            None,
        )
        .await?;
        sp.commit().await.map_err(db)?;
        if !t.lignes.is_empty() {
            lignes += t.lignes.len();
            contenu.push(t);
        }
    }
    contenu.sort_by(|a, b| a.nom.cmp(&b.nom));
    let exclue_entierement =
        f.code == "sante" && !ctx.module_sante && !exclusions.is_empty() && contenu.is_empty();
    Ok(finir_section(
        f,
        contenu,
        exclusions,
        sans_rattachement,
        lignes,
        exclue_entierement,
    ))
}

fn finir_section(
    f: &Fournisseur,
    tables: Vec<TableContenu>,
    exclusions: Vec<Exclusion>,
    sans_rattachement: usize,
    lignes: usize,
    exclue: bool,
) -> SectionContenu {
    if exclue {
        return SectionContenu {
            fournisseur: f.code.into(),
            socle: f.socle.into(),
            statut: StatutSection::Exclue,
            motif_code: Some("module_sante_inactif".into()),
            tables: vec![],
            exclusions,
            tables_sans_rattachement: sans_rattachement,
            lignes: 0,
            empreinte: None,
        };
    }
    let statut = if lignes == 0 {
        StatutSection::Vide
    } else {
        StatutSection::Ok
    };
    let empreinte = Some(empreinte_section(&tables));
    SectionContenu {
        fournisseur: f.code.into(),
        socle: f.socle.into(),
        statut,
        motif_code: None,
        tables,
        exclusions,
        tables_sans_rattachement: sans_rattachement,
        lignes,
        empreinte,
    }
}

fn section_sans_contenu(f: &Fournisseur, statut: StatutSection, motif: &str) -> SectionContenu {
    SectionContenu {
        fournisseur: f.code.into(),
        socle: f.socle.into(),
        statut,
        motif_code: Some(motif.into()),
        tables: vec![],
        exclusions: vec![],
        tables_sans_rattachement: 0,
        lignes: 0,
        empreinte: None,
    }
}

/// Assemble la section d'un fournisseur pour un sujet **personne** : écrit pour cinq fournisseurs, « non couverte »
/// pour les autres.
async fn section_personne(
    conn: &mut PgConnection,
    ctx: &ContexteSujet,
    champs: &ChampsSensibles,
    f: &Fournisseur,
) -> Result<SectionContenu, EntDbError> {
    if f.code == "sante" && !ctx.module_sante {
        let exclusions = tables_du_fournisseur(f)
            .into_iter()
            .map(|t| Exclusion {
                table: t.nom.into(),
                motif: motif_exclusion_sante(),
            })
            .collect();
        return Ok(finir_section(f, vec![], exclusions, 0, 0, true));
    }
    if !f.personne {
        return Ok(section_sans_contenu(
            f,
            StatutSection::NonCouverte,
            "non_ecrit_pour_ce_sujet",
        ));
    }
    let mut regles = regles_personne(f.code);
    // Tables « de domaine » reliées à la personne ou à l'agent par leur clé, déclarées au registre, non écrites
    // ci-dessus : même lecture générique que pour l'élève (colonnes du registre, jamais la liste blanche).
    let deja: BTreeSet<&str> = regles.iter().map(|r| r.table).collect();
    let tables = tables_du_fournisseur(f);
    let noms: Vec<&str> = tables
        .iter()
        .map(|t| t.nom)
        .chain(regles.iter().map(|r| r.table))
        .collect();
    let toutes_colonnes = colonnes_des_tables(conn, &noms).await?;
    let mut sans_rattachement = 0usize;
    let mut generiques: Vec<Regle> = Vec::new();
    for t in &tables {
        if deja.contains(t.nom) || TABLES_MESSAGERIE.contains(&t.nom) {
            continue;
        }
        if t.sante && !ctx.module_sante {
            continue;
        }
        let cols = toutes_colonnes.get(t.nom).cloned().unwrap_or_default();
        let cle = |n: &str| cols.iter().any(|(c, ty)| c == n && ty == "uuid");
        if f.code == "personnels" && cle("agent_id") {
            generiques.push(regle(t.nom, "x.agent_id = p.agent"));
        } else if f.code == "administratif" && cle("personne_id") {
            generiques.push(regle(t.nom, "x.personne_id = p.personne"));
        } else {
            sans_rattachement += 1;
        }
    }
    regles.extend(generiques);
    let mut contenu = Vec::new();
    let mut lignes = 0usize;
    for r in &regles {
        let cols = toutes_colonnes.get(r.table).cloned().unwrap_or_default();
        let mut sp = conn.begin().await.map_err(db)?;
        let t = lire_table(
            &mut sp,
            ctx,
            champs,
            r,
            crate::registre_donnees::classer(r.table),
            &cols,
            None,
        )
        .await?;
        sp.commit().await.map_err(db)?;
        if !t.lignes.is_empty() {
            lignes += t.lignes.len();
            contenu.push(t);
        }
    }
    contenu.sort_by(|a, b| a.nom.cmp(&b.nom));
    Ok(finir_section(
        f,
        contenu,
        vec![],
        sans_rattachement,
        lignes,
        false,
    ))
}

impl PgEnt {
    /// Assemble le **contenu** d'un extrait (jamais stocké). `echecs_simules` : codes de fournisseurs forcés en échec
    /// (tests : un fournisseur qui tombe rend l'extrait incomplet, R-RG18).
    async fn construire_contenu(
        &self,
        conn: &mut PgConnection,
        ctx: &ContexteSujet,
        echecs_simules: &[&str],
    ) -> Result<ContenuExtrait, EntDbError> {
        let champs = ChampsSensibles::depuis_env();
        let mut sections: Vec<SectionContenu> = Vec::with_capacity(FOURNISSEURS.len());
        for f in FOURNISSEURS {
            let r = if echecs_simules.contains(&f.code) {
                Err(invalide("échec simulé"))
            } else if ctx.est_eleve() {
                section_eleve(conn, ctx, &champs, f).await
            } else {
                section_personne(conn, ctx, &champs, f).await
            };
            sections.push(match r {
                Ok(s) => s,
                Err(e) => {
                    // Le détail n'est jamais écrit dans l'extrait (il pourrait contenir une donnée) : un code neutre.
                    tracing::warn!(fournisseur = f.code, erreur = %e, "extrait RGPD : fournisseur en échec");
                    section_sans_contenu(f, StatutSection::Echec, "fournisseur_en_echec")
                }
            });
        }
        let complet = sections.iter().all(|s| !s.statut.incomplete());
        let empreinte = empreinte_globale(&sections);
        let nb_lignes = sections.iter().map(|s| s.lignes).sum();
        let meta = meta_extrait(conn, ctx, &sections).await?;
        Ok(ContenuExtrait {
            sujet_type: ctx.sujet_type.clone(),
            sujet_id: ctx.sujet_id,
            periode_du: ctx.du,
            periode_au: ctx.au,
            sections,
            complet,
            empreinte,
            nb_lignes,
            destinataires_internes: meta.0,
            destinataires_externes: meta.1,
            soustraitants: meta.2,
            durees: meta.3,
            sources: meta.4,
            marquages: Vec::new(),
        })
    }
}

type Meta = (
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<DureeContenu>,
    Vec<String>,
);

/// Destinataires, sous-traitants, durées et sources (§9.4 point 3), tirés du registre des tables, du catalogue de
/// conservation et du registre G2 de l'établissement.
async fn meta_extrait(
    conn: &mut PgConnection,
    ctx: &ContexteSujet,
    sections: &[SectionContenu],
) -> Result<Meta, EntDbError> {
    let codes: BTreeSet<&str> = sections
        .iter()
        .filter(|s| matches!(s.statut, StatutSection::Ok))
        .map(|s| s.fournisseur.as_str())
        .collect();
    let mut domaines: BTreeSet<&'static str> = BTreeSet::new();
    let mut categories: BTreeSet<&'static str> = BTreeSet::new();
    for f in FOURNISSEURS.iter().filter(|f| codes.contains(f.code)) {
        domaines.extend(f.domaines.iter().copied());
    }
    for s in sections
        .iter()
        .filter(|s| matches!(s.statut, StatutSection::Ok))
    {
        for t in &s.tables {
            if let Some(c) = crate::registre_donnees::classer(&t.nom) {
                categories.insert(categorie_de_table(c));
            }
        }
    }
    let internes: Vec<String> = crate::registre_rgpd::destinataires_internes_de(
        &domaines.iter().copied().collect::<Vec<_>>(),
    )
    .into_iter()
    .collect();
    let externes: Vec<(String, String)> = sqlx::query_as(
        "SELECT DISTINCT libelle, nature FROM rgp_destinataire_externe WHERE etablissement_id = $1 ORDER BY 1, 2",
    )
    .bind(ctx.etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
        .bind(ctx.etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let st = crate::registre_rgpd::soustraitants_plateforme(conn, jour).await?;
    let durees: Vec<DureeContenu> = categories
        .iter()
        .filter_map(|c| categorie(c))
        .map(|c| DureeContenu {
            categorie: c.code.into(),
            libelle: c.libelle.into(),
            utilite_mois: c.utilite_mois,
            intermediaire_mois: c.intermediaire_mois,
            source: c.source.into(),
        })
        .collect();
    let mut sources: BTreeSet<String> = BTreeSet::new();
    sources.insert(
        "Données saisies dans l'ENT par les personnels de l'établissement, par les familles et par les élèves, \
         ou importées depuis l'application nationale de l'établissement."
            .into(),
    );
    for d in &durees {
        sources.insert(format!("Durée « {} » : {}", d.libelle, d.source));
    }
    Ok((
        internes,
        externes
            .into_iter()
            .map(|(l, n)| format!("{l} ({n})"))
            .collect(),
        st.into_iter()
            .map(|s| format!("{} — {} ({})", s.nom, s.service, s.pays))
            .collect(),
        durees,
        sources.into_iter().collect(),
    ))
}

// ---------------------------------------------------------------------------------------------
// Services : sujets, assemblage, lecture, DPO, remise incomplète
// ---------------------------------------------------------------------------------------------

/// Ce que le service sait de la demande pour assembler.
struct DemandeInfo {
    qualite: String,
    sujet_type: String,
    sujet_id: Option<Uuid>,
    demandeur_compte: Option<Uuid>,
    demandeur_personne: Option<Uuid>,
    recue_le: NaiveDate,
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
}

async fn info_demande(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
) -> Result<DemandeInfo, EntDbError> {
    let r = sqlx::query(
        "SELECT qualite, sujet_type, sujet_id, demandeur_compte_id, demandeur_personne_id, recue_le,
                perimetre_du, perimetre_au
           FROM rgp_demande WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(demande)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    Ok(DemandeInfo {
        qualite: r.get("qualite"),
        sujet_type: r.get("sujet_type"),
        sujet_id: r.get("sujet_id"),
        demandeur_compte: r.get("demandeur_compte_id"),
        demandeur_personne: r.get("demandeur_personne_id"),
        recue_le: r.get("recue_le"),
        du: r.get("perimetre_du"),
        au: r.get("perimetre_au"),
    })
}

/// Les sujets d'une demande : le sujet principal d'abord, puis ceux qui ont été ajoutés (RGP-302).
async fn sujets_de(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    info: &DemandeInfo,
) -> Result<Vec<(String, Uuid, String)>, EntDbError> {
    let mut v: Vec<(String, Uuid, String)> = Vec::new();
    if let (true, Some(id)) = (
        matches!(info.sujet_type.as_str(), "eleve" | "personne" | "agent"),
        info.sujet_id,
    ) {
        let q: Option<String> = sqlx::query_scalar(
            "SELECT qualite_constatee FROM rgp_demande_identite
              WHERE etablissement_id = $1 AND demande_id = $2 AND qualite_constatee IS NOT NULL
              ORDER BY cree_le DESC LIMIT 1",
        )
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        v.push((
            info.sujet_type.clone(),
            id,
            q.unwrap_or_else(|| "non_verifiable".into()),
        ));
    }
    let autres = sqlx::query(
        "SELECT sujet_type, sujet_id, qualite_constatee FROM rgp_demande_sujet
          WHERE etablissement_id = $1 AND demande_id = $2 AND NOT principal ORDER BY cree_le, id",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for r in autres {
        v.push((
            r.get("sujet_type"),
            r.get("sujet_id"),
            r.get("qualite_constatee"),
        ));
    }
    Ok(v)
}

async fn contexte_sujet(
    conn: &mut PgConnection,
    etab: Uuid,
    info: &DemandeInfo,
    sujet_type: &str,
    sujet_id: Uuid,
    module_sante: bool,
) -> Result<ContexteSujet, EntDbError> {
    let (mut compte, mut personne, mut agent) = (None, None, None);
    if sujet_type != "eleve" {
        let r: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT user_id FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(sujet_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(u) = r else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        compte = u;
        personne = Some(sujet_id);
        agent = sqlx::query_scalar(
            "SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2",
        )
        .bind(etab)
        .bind(sujet_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
    }
    // Personne du demandeur : celle qui a été rapprochée, sinon celle de son compte.
    let dem_personne = match (info.demandeur_personne, info.demandeur_compte) {
        (Some(p), _) => Some(p),
        (None, Some(c)) => sqlx::query_scalar(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL LIMIT 1",
        )
        .bind(etab)
        .bind(c)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?,
        _ => None,
    };
    Ok(ContexteSujet {
        etab,
        sujet_type: sujet_type.to_string(),
        sujet_id,
        compte,
        personne,
        agent,
        dem_personne,
        dem_est_sujet: sujet_type == "eleve" && qualite_de_soi(&info.qualite),
        du: info.du,
        au: info.au,
        module_sante,
    })
}

fn vue_depuis(r: &sqlx::postgres::PgRow, sections: Vec<SectionVue>) -> ExtraitVue {
    let complet: bool = r.get("complet");
    let remissible = complet
        || sections
            .iter()
            .filter(|s| s.statut == "echec" || s.statut == "non_couverte")
            .all(|s| s.remise_incomplete_motif.is_some());
    let perime_le: Option<DateTime<Utc>> = r.get("perime_le");
    ExtraitVue {
        id: r.get("id"),
        demande_id: r.get("demande_id"),
        sujet_type: r.get("sujet_type"),
        sujet_id: r.get("sujet_id"),
        version: r.get("version"),
        assemble_le: r.get("assemble_le"),
        complet,
        perime: perime_le.is_some(),
        perime_motif: r.get("perime_motif"),
        nb_sections: r.get("nb_sections"),
        nb_lignes: r.get("nb_lignes"),
        empreinte: r.get("empreinte"),
        sections,
        soumis_dpo: r.get::<Option<DateTime<Utc>>, _>("soumis_dpo_le").is_some(),
        avis_dpo_code: r.get("dpo_avis_code"),
        avis_dpo_texte: r.get("dpo_avis_texte"),
        remissible_en_l_etat: remissible,
        masquages_a_decider: 0,
    }
}

const COLONNES_EXTRAIT: &str =
    "id, demande_id, sujet_type, sujet_id, version, assemble_le, complet, perime_le, \
    perime_motif, nb_sections, nb_lignes, empreinte, soumis_dpo_le, dpo_avis_code, dpo_avis_texte";

async fn charger_extrait(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    extrait: Uuid,
) -> Result<ExtraitVue, EntDbError> {
    let r = sqlx::query(&format!(
        "SELECT {COLONNES_EXTRAIT} FROM rgp_extrait WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3"
    ))
    .bind(etab)
    .bind(demande)
    .bind(extrait)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    let sections = sections_de(conn, etab, extrait).await?;
    let mut v = vue_depuis(&r, sections);
    completer_masquages(conn, etab, &mut v).await?;
    Ok(v)
}

/// Compte les marquages à décider : tant qu'il en reste, l'extrait n'est pas remissible.
async fn completer_masquages(
    conn: &mut PgConnection,
    etab: Uuid,
    v: &mut ExtraitVue,
) -> Result<(), EntDbError> {
    let n = crate::droits_extrait_masquage::restant_a_decider(conn, etab, v.id).await?;
    v.masquages_a_decider = n as i32;
    if n > 0 {
        v.remissible_en_l_etat = false;
    }
    Ok(())
}

async fn sections_de(
    conn: &mut PgConnection,
    etab: Uuid,
    extrait: Uuid,
) -> Result<Vec<SectionVue>, EntDbError> {
    let rows = sqlx::query(
        "SELECT socle, fournisseur_code, statut, motif_code, lignes, remise_incomplete_motif
           FROM rgp_extrait_section WHERE etablissement_id = $1 AND extrait_id = $2
          ORDER BY (socle)::int, fournisseur_code",
    )
    .bind(etab)
    .bind(extrait)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .map(|s| SectionVue {
            fournisseur: s.get("fournisseur_code"),
            socle: s.get("socle"),
            statut: s.get("statut"),
            motif_code: s.get("motif_code"),
            lignes: s.get("lignes"),
            remise_incomplete_motif: s.get("remise_incomplete_motif"),
        })
        .collect())
}

impl PgEnt {
    /// **RGP-302** — ajoute un sujet à la demande (fratrie) : même vérification de qualité que le sujet principal,
    /// au jour de réception (socle 4). Un extrait par sujet, une réponse commune. `droits.traiter`.
    pub async fn droits_sujet_ajouter(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        sujet_type: &str,
        sujet_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        if !matches!(sujet_type, "eleve" | "personne" | "agent") {
            return Err(invalide("personne concernée incohérente"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        crate::droits_demandes::verifier_sujet(&mut tx, etab, sujet_type, Some(sujet_id), None)
            .await?;
        let info = info_demande(&mut tx, etab, demande).await?;
        if info.sujet_id == Some(sujet_id) && info.sujet_type == sujet_type {
            return Err(EntDbError::ConflitDossier(
                "cette personne est déjà le sujet principal de la demande",
            ));
        }
        let q = qualite_a_la_date(
            &mut tx,
            etab,
            info.demandeur_compte,
            info.demandeur_personne,
            &info.qualite,
            sujet_type,
            Some(sujet_id),
            info.recue_le,
        )
        .await?;
        let n = sqlx::query(
            "INSERT INTO rgp_demande_sujet (id, etablissement_id, demande_id, sujet_type, sujet_id, principal,
                                            qualite_constatee, ajoute_par_compte_id)
             VALUES ($1, $2, $3, $4, $5, false, $6, $7)
             ON CONFLICT (etablissement_id, demande_id, sujet_type, sujet_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(demande)
        .bind(sujet_type)
        .bind(sujet_id)
        .bind(q.code())
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(EntDbError::ConflitDossier(
                "cette personne est déjà un sujet de la demande",
            ));
        }
        journal(
            &mut tx,
            etab,
            demande,
            "sujet_ajoute",
            Some(&format!(
                "sujet ajouté : qualité constatée « {} »",
                q.code()
            )),
            Some(acteur),
            Some(role),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// **RGP-149** — assemble l'extrait de la demande : un par sujet (le principal, puis ceux de la fratrie), chacun en
    /// une **nouvelle version** (les précédentes passent « remplacées »). `droits.traiter`. `sujet` : un seul sujet, sinon tous.
    pub async fn droits_extrait_assembler(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        sujet: Option<(&str, Uuid)>,
    ) -> Result<Vec<ExtraitVue>, EntDbError> {
        self.droits_extrait_assembler_interne(etab, acteur, role, demande, sujet, &[])
            .await
    }

    pub(crate) async fn droits_extrait_assembler_interne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        sujet: Option<(&str, Uuid)>,
        echecs_simules: &[&str],
    ) -> Result<Vec<ExtraitVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let peut_arbitrer = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        let module_sante = self.module_sante_actif(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        // Rien ne s'assemble pour un demandeur dont l'identité et la qualité ne sont pas établies (E2, R-RG42).
        if !identite_etablie(&mut tx, etab, demande).await? {
            return Err(EntDbError::ConflitDossier(
                "identité et qualité du demandeur à établir avant d'assembler l'extrait",
            ));
        }
        let info = info_demande(&mut tx, etab, demande).await?;
        let tous = sujets_de(&mut tx, etab, demande, &info).await?;
        if tous.is_empty() {
            return Err(EntDbError::ConflitDossier(
                "la personne concernée n'est pas encore identifiée",
            ));
        }
        let choisis: Vec<&(String, Uuid, String)> = match sujet {
            Some((t, id)) => {
                let v: Vec<_> = tous.iter().filter(|s| s.0 == t && s.1 == id).collect();
                if v.is_empty() {
                    return Err(EntDbError::Domain(DomainError::NotFound));
                }
                v
            }
            None => tous.iter().collect(),
        };
        let mut vues = Vec::new();
        for (st, sid, qualite) in choisis {
            // Qualité du demandeur POUR CE SUJET (RGP-302) : aucune pour `absente` ; l'arbitrage de la direction
            // pour une qualité restreinte, sans autorité ou non vérifiable (CL27-02, CL27-03, D-069).
            match qualite.as_str() {
                "absente" => {
                    return Err(EntDbError::ConflitDossier(
                        "qualité du demandeur non établie pour cette personne : aucun extrait",
                    ))
                }
                "confirmee" | "sans_objet" => {}
                _ if peut_arbitrer => {}
                _ => {
                    return Err(EntDbError::ConflitDossier(
                        "qualité du demandeur à arbitrer par la direction pour cette personne",
                    ))
                }
            }
            let ctx = contexte_sujet(&mut tx, etab, &info, st, *sid, module_sante).await?;
            let contenu = self
                .construire_contenu(&mut tx, &ctx, echecs_simules)
                .await?;
            let marques = if ctx.est_eleve() {
                let faits = crate::droits_extrait_masquage::faits_eleve(
                    &mut tx,
                    etab,
                    *sid,
                    info.demandeur_compte,
                    ctx.dem_personne,
                    ctx.dem_est_sujet,
                )
                .await?;
                crate::droits_extrait_masquage::marquer(&contenu, &faits)
            } else {
                Vec::new()
            };
            let v = self
                .persister_extrait(&mut tx, etab, demande, acteur, role, &contenu, &marques)
                .await?;
            vues.push(v);
        }
        // Un extrait assemblé n'est jamais remis sans relecture humaine (R-RG15) : la demande passe « à relire ».
        sqlx::query(
            "UPDATE rgp_demande SET etat = 'a_relire', modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat IN ('recue', 'en_cours', 'a_relire', 'prete')",
        )
        .bind(etab)
        .bind(demande)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(vues)
    }

    async fn persister_extrait(
        &self,
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        demande: Uuid,
        acteur: Uuid,
        role: &str,
        c: &ContenuExtrait,
        marques: &[crate::droits_extrait_masquage::MarquageCalcule],
    ) -> Result<ExtraitVue, EntDbError> {
        // Sérialise les assemblages d'un même sujet : la version suivante est calculée sous le verrou de la demande.
        let version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(version), 0) + 1 FROM rgp_extrait
              WHERE etablissement_id = $1 AND demande_id = $2 AND sujet_type = $3 AND sujet_id = $4",
        )
        .bind(etab)
        .bind(demande)
        .bind(&c.sujet_type)
        .bind(c.sujet_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE rgp_extrait SET perime_le = now(), perime_motif = 'remplace', modifie_le = now()
              WHERE etablissement_id = $1 AND demande_id = $2 AND sujet_type = $3 AND sujet_id = $4
                AND perime_le IS NULL",
        )
        .bind(etab)
        .bind(demande)
        .bind(&c.sujet_type)
        .bind(c.sujet_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO rgp_extrait (id, etablissement_id, demande_id, sujet_type, sujet_id, version, periode_du,
                                      periode_au, assemble_par_compte_id, complet, nb_sections, nb_lignes, empreinte)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(etab)
        .bind(demande)
        .bind(&c.sujet_type)
        .bind(c.sujet_id)
        .bind(version)
        .bind(c.periode_du)
        .bind(c.periode_au)
        .bind(acteur)
        .bind(c.complet)
        .bind(c.sections.len() as i32)
        .bind(c.nb_lignes as i32)
        .bind(&c.empreinte)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        for s in &c.sections {
            sqlx::query(
                "INSERT INTO rgp_extrait_section (id, etablissement_id, extrait_id, socle, fournisseur_code, statut,
                                                  motif_code, lignes, empreinte)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(&s.socle)
            .bind(&s.fournisseur)
            .bind(s.statut.code())
            .bind(&s.motif_code)
            .bind(s.lignes as i32)
            .bind(&s.empreinte)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
        crate::droits_extrait_masquage::persister_marquages(
            tx,
            etab,
            demande,
            id,
            &c.sujet_type,
            c.sujet_id,
            marques,
        )
        .await?;
        // Le sujet a une ligne dans `rgp_demande_sujet` : l'extrait courant lui est rattachée.
        sqlx::query(
            "INSERT INTO rgp_demande_sujet (id, etablissement_id, demande_id, sujet_type, sujet_id, principal,
                                            qualite_constatee, ajoute_par_compte_id, extrait_courant_id)
             SELECT $1, $2, $3, $4, $5, d.sujet_id = $5 AND d.sujet_type = $4, 'non_verifiable', $6, NULL
               FROM rgp_demande d WHERE d.etablissement_id = $2 AND d.id = $3
             ON CONFLICT (etablissement_id, demande_id, sujet_type, sujet_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(demande)
        .bind(&c.sujet_type)
        .bind(c.sujet_id)
        .bind(acteur)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE rgp_demande_sujet SET extrait_courant_id = $5
              WHERE etablissement_id = $1 AND demande_id = $2 AND sujet_type = $3 AND sujet_id = $4",
        )
        .bind(etab)
        .bind(demande)
        .bind(&c.sujet_type)
        .bind(c.sujet_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        journal(
            tx,
            etab,
            demande,
            "extrait_assemble",
            Some(&format!(
                "extrait assemblé, version {version} : {} section(s), {}",
                c.sections.len(),
                if c.complet { "complet" } else { "INCOMPLET" }
            )),
            Some(acteur),
            Some(role),
        )
        .await?;
        charger_extrait(tx, etab, demande, id).await
    }

    /// Les extraits de la demande (structure seulement, jamais le contenu), versions courantes d'abord.
    pub async fn droits_extraits(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
    ) -> Result<Vec<ExtraitVue>, EntDbError> {
        let traite = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let arbitre = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if !traite && !arbitre {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        info_demande(&mut tx, etab, demande).await?;
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES_EXTRAIT} FROM rgp_extrait
              WHERE etablissement_id = $1 AND demande_id = $2
                AND ($3 OR soumis_dpo_le IS NOT NULL)
              ORDER BY sujet_type, sujet_id, version DESC"
        ))
        .bind(etab)
        .bind(demande)
        .bind(traite)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let id: Uuid = r.get("id");
            let sections = sections_de(&mut tx, etab, id).await?;
            let mut v = vue_depuis(&r, sections);
            completer_masquages(&mut tx, etab, &mut v).await?;
            out.push(v);
        }
        Ok(out)
    }

    /// **Lit le contenu** d'un extrait (RGP-152 : gestionnaire ou DPO). La consultation est **tracée avant** d'être
    /// servie (R-RG8) ; le contenu est reconstruit, son empreinte comparée à celle de l'assemblage : si elle diffère,
    /// l'extrait est **périmé** (409) et rien n'est servi. Le DPO ne lit que l'extrait d'une demande qu'on lui a
    /// **soumise** (§12.2).
    pub async fn droits_extrait_lire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<ContenuExtrait, EntDbError> {
        let traite = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let arbitre = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if !traite && !arbitre {
            return Err(EntDbError::Forbidden);
        }
        let module_sante = self.module_sante_actif(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let info = info_demande(&mut tx, etab, demande).await?;
        let r = sqlx::query(
            "SELECT sujet_type, sujet_id, empreinte, perime_le, soumis_dpo_le, periode_du, periode_au
               FROM rgp_extrait WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3",
        )
        .bind(etab)
        .bind(demande)
        .bind(extrait)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let soumis: Option<DateTime<Utc>> = r.get("soumis_dpo_le");
        if !traite && soumis.is_none() {
            // Le DPO n'ouvre que ce qu'on lui soumet : même refus que s'il n'avait aucun droit.
            return Err(EntDbError::Forbidden);
        }
        let sujet_type: String = r.get("sujet_type");
        let sujet_id: Uuid = r.get("sujet_id");
        let stockee: String = r.get("empreinte");
        let perime: Option<DateTime<Utc>> = r.get("perime_le");
        if perime.is_some() {
            return Err(EntDbError::ConflitDossier(
                "extrait périmé : assembler une nouvelle version",
            ));
        }
        let eleve = (sujet_type == "eleve").then_some(sujet_id);
        // R-RG8 : la trace d'abord. Un échec d'écriture refuse la lecture.
        self.tracer_consultation_a(
            etab,
            acteur,
            role,
            eleve,
            "extrait_droits",
            "Lecture du contenu d'un extrait de demande de droits",
            false,
            Utc::now(),
        )
        .await?;
        let mut ctx =
            contexte_sujet(&mut tx, etab, &info, &sujet_type, sujet_id, module_sante).await?;
        ctx.du = r.get("periode_du");
        ctx.au = r.get("periode_au");
        let contenu = self.construire_contenu(&mut tx, &ctx, &[]).await?;
        if contenu.empreinte != stockee {
            sqlx::query(
                "UPDATE rgp_extrait SET perime_le = now(), perime_motif = 'empreinte_changee', modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND perime_le IS NULL",
            )
            .bind(etab)
            .bind(extrait)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journal(
                &mut tx,
                etab,
                demande,
                "extrait_perime",
                Some("le contenu a changé depuis l'assemblage : nouvelle version à assembler, masquages à relire"),
                Some(acteur),
                Some(role),
            )
            .await?;
            tx.commit().await?;
            return Err(EntDbError::ConflitDossier(
                "extrait périmé : assembler une nouvelle version",
            ));
        }
        journal(
            &mut tx,
            etab,
            demande,
            "extrait_lu",
            Some("contenu de l'extrait ouvert"),
            Some(acteur),
            Some(role),
        )
        .await?;
        let marques =
            crate::droits_extrait_masquage::marquages_de(&mut tx, etab, extrait, arbitre).await?;
        let contenu = if arbitre {
            crate::droits_extrait_masquage::remplacer_vide(contenu, &marques)
        } else {
            crate::droits_extrait_masquage::cacher_protections(&contenu, &marques)
        };
        tx.commit().await?;
        Ok(contenu)
    }

    /// Contenu **prêt à remettre** d'un extrait (lot G3b-3) : reconstruit, empreinte comparée (écart → périmé, 409),
    /// remissible en l'état (R-RG18 : sections incomplètes décidées ; RGP-150 : plus aucun marquage à décider), puis
    /// **masqué** : chaque élément non levé est remplacé par son libellé. Aucun droit n'est vérifié ici : l'appelant
    /// (`droits.remettre`) l'a fait. Renvoie aussi la vue de l'extrait et le demandeur (compte) de la demande.
    pub(crate) async fn contenu_a_remettre(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<(ExtraitVue, ContenuExtrait, Option<Uuid>), EntDbError> {
        let module_sante = self.module_sante_actif(etab).await?;
        let info = info_demande(&mut *tx, etab, demande).await?;
        let vue = charger_extrait(&mut *tx, etab, demande, extrait).await?;
        if vue.perime {
            return Err(EntDbError::ConflitDossier(
                "extrait périmé : assembler une nouvelle version",
            ));
        }
        let r = sqlx::query(
            "SELECT periode_du, periode_au FROM rgp_extrait WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(extrait)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut ctx = contexte_sujet(&mut *tx, etab, &info, &vue.sujet_type, vue.sujet_id, module_sante).await?;
        ctx.du = r.get("periode_du");
        ctx.au = r.get("periode_au");
        let contenu = self.construire_contenu(&mut *tx, &ctx, &[]).await?;
        if contenu.empreinte != vue.empreinte {
            return Err(EntDbError::ConflitDossier(
                "extrait périmé : le contenu a changé, assembler une nouvelle version",
            ));
        }
        if !vue.remissible_en_l_etat {
            return Err(EntDbError::ConflitDossier(
                "remise impossible : sections incomplètes sans décision ou marquages à décider",
            ));
        }
        let marques =
            crate::droits_extrait_masquage::marquages_de(&mut *tx, etab, extrait, true).await?;
        let contenu = crate::droits_extrait_masquage::appliquer_masques(&contenu, &marques);
        Ok((vue, contenu, info.demandeur_compte))
    }

    /// **R-RG18** — décision motivée de la direction : remettre l'extrait **malgré** des sections en échec ou non
    /// couvertes. Une décision par section, motif écrit ; la réponse au demandeur le dit. `droits.arbitrer`.
    pub async fn droits_extrait_arbitrer_incomplet(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
        decisions: &[(&str, &str)],
    ) -> Result<ExtraitVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if decisions.is_empty() {
            return Err(invalide("au moins une section"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        let v = charger_extrait(&mut tx, etab, demande, extrait).await?;
        if v.perime {
            return Err(EntDbError::ConflitDossier(
                "extrait périmé : assembler une nouvelle version",
            ));
        }
        for (code, motif) in decisions {
            let Some(motif) = texte(motif, 3, 2000) else {
                return Err(invalide(
                    "motif obligatoire pour chaque section (3 à 2000 caractères)",
                ));
            };
            let s = v
                .sections
                .iter()
                .find(|s| s.fournisseur == *code)
                .ok_or(EntDbError::Domain(DomainError::NotFound))?;
            if s.statut != "echec" && s.statut != "non_couverte" {
                return Err(invalide(
                    "seule une section en échec ou non couverte se décide ainsi",
                ));
            }
            sqlx::query(
                "UPDATE rgp_extrait_section
                    SET remise_incomplete_motif = $4, remise_incomplete_par_compte_id = $5, remise_incomplete_le = now()
                  WHERE etablissement_id = $1 AND extrait_id = $2 AND fournisseur_code = $3",
            )
            .bind(etab)
            .bind(extrait)
            .bind(code)
            .bind(&motif)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journal(
                &mut tx,
                etab,
                demande,
                "extrait_incomplet_arbitre",
                Some(&format!(
                    "remise sans la section « {code} » décidée par la direction"
                )),
                Some(acteur),
                Some(role),
            )
            .await?;
        }
        let v = charger_extrait(&mut tx, etab, demande, extrait).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-152** — soumet l'extrait au DPO pour relecture : dès lors, un porteur de `droits.arbitrer` sans
    /// `droits.traiter` peut l'ouvrir (§12.2). `droits.traiter`.
    pub async fn droits_extrait_soumettre_dpo(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<ExtraitVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        let v = charger_extrait(&mut tx, etab, demande, extrait).await?;
        if v.perime {
            return Err(EntDbError::ConflitDossier(
                "extrait périmé : assembler une nouvelle version",
            ));
        }
        // Une nouvelle soumission efface l'avis précédent : l'avis porte sur ce qui vient d'être soumis.
        sqlx::query(
            "UPDATE rgp_extrait
                SET soumis_dpo_le = now(), soumis_dpo_par_compte_id = $3,
                    dpo_avis_le = NULL, dpo_avis_par_compte_id = NULL, dpo_avis_code = NULL, dpo_avis_texte = NULL,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(extrait)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "extrait_soumis_dpo",
            Some("extrait soumis au DPO"),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger_extrait(&mut tx, etab, demande, extrait).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Avis du DPO sur l'extrait qu'on lui a soumis : `favorable` ou `reserves`, avec un texte. `droits.arbitrer`,
    /// et seulement sur un extrait soumis.
    pub async fn droits_extrait_avis_dpo(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
        avis: &str,
        texte_avis: &str,
    ) -> Result<ExtraitVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if !matches!(avis, "favorable" | "reserves") {
            return Err(invalide("avis inconnu"));
        }
        let Some(txt) = texte(texte_avis, 3, 4000) else {
            return Err(invalide("avis motivé (3 à 4000 caractères)"));
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let v = charger_extrait(&mut tx, etab, demande, extrait).await?;
        if !v.soumis_dpo || v.perime {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query(
            "UPDATE rgp_extrait
                SET dpo_avis_le = now(), dpo_avis_par_compte_id = $3, dpo_avis_code = $4, dpo_avis_texte = $5,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(extrait)
        .bind(acteur)
        .bind(avis)
        .bind(&txt)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "extrait_avis_dpo",
            Some(&format!("avis du DPO : {avis}")),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger_extrait(&mut tx, etab, demande, extrait).await?;
        tx.commit().await?;
        Ok(v)
    }
}

// ---------------------------------------------------------------------------------------------
// Réglages RGPD de l'établissement (Pf1 et lot G3b) : valeurs typées, bornées, journalisées.
// ---------------------------------------------------------------------------------------------

/// Réglages RGPD d'un établissement ; l'absence de ligne vaut valeurs par défaut (D-069).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReglageRgpd {
    /// Plafond de dépôts publics sans compte par jour (anti-spam).
    pub plafond_depots_publics_jour: i32,
    /// Jours de mise à disposition de l'extrait remis en ligne.
    pub extrait_disponible_jours: i32,
}

impl PgEnt {
    /// Lit les réglages (droit `conformite.gerer` ou `droits.traiter`).
    pub async fn droits_reglage_lire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<ReglageRgpd, EntDbError> {
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ConformiteGerer)
            .await?
            && !self
                .a_action_etablissement(etab, acteur, role, Action::DroitsTraiter)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = lire_reglage(&mut tx, etab).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Modifie un réglage (`conformite.gerer`) ; l'ancienne et la nouvelle valeur sont journalisées.
    pub async fn droits_reglage_modifier(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        reglage: &str,
        valeur: i32,
    ) -> Result<ReglageRgpd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let (colonne, min, max) = match reglage {
            "plafond_depots_publics_jour" => ("plafond_depots_publics_jour", 1, 1000),
            "extrait_disponible_jours" => ("extrait_disponible_jours", 1, 60),
            _ => return Err(invalide("réglage inconnu")),
        };
        if !(min..=max).contains(&valeur) {
            return Err(invalide("valeur hors des bornes du réglage"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let avant = lire_reglage(&mut tx, etab).await?;
        let ancienne = match colonne {
            "plafond_depots_publics_jour" => avant.plafond_depots_publics_jour,
            _ => avant.extrait_disponible_jours,
        };
        // Colonne choisie parmi une liste fermée ci-dessus : jamais d'entrée libre dans le SQL.
        let sql = format!(
            "INSERT INTO rgp_reglage (etablissement_id, {colonne}, modifie_par_compte_id)
             VALUES ($1, $2, $3)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET {colonne} = EXCLUDED.{colonne}, modifie_par_compte_id = $3, modifie_le = now()"
        );
        sqlx::query(&sql)
            .bind(etab)
            .bind(valeur)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO rgp_reglage_journal
                 (id, etablissement_id, reglage, ancienne_valeur, nouvelle_valeur, par_compte_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(reglage)
        .bind(ancienne.to_string())
        .bind(valeur.to_string())
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let apres = lire_reglage(&mut tx, etab).await?;
        tx.commit().await?;
        Ok(apres)
    }
}

async fn lire_reglage(conn: &mut PgConnection, etab: Uuid) -> Result<ReglageRgpd, EntDbError> {
    let r: Option<(i32, i32)> = sqlx::query_as(
        "SELECT plafond_depots_publics_jour, extrait_disponible_jours FROM rgp_reglage
          WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.map(|(p, e)| ReglageRgpd {
        plafond_depots_publics_jour: p,
        extrait_disponible_jours: e,
    })
    .unwrap_or(ReglageRgpd {
        plafond_depots_publics_jour: 30,
        extrait_disponible_jours: 7,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extrait_personne::fournisseurs_de;

    #[test]
    fn un_iban_est_masque_sauf_sa_fin() {
        assert_eq!(
            masquer_iban("FR76 3000 6000 0112 3456 7890 189"),
            "FR76 **** **** 0189"
        );
        assert_eq!(masquer_iban("FR76"), "**** ****");
    }

    #[test]
    fn le_rendu_des_valeurs_est_deterministe() {
        assert_eq!(rendre(&Value::Null), "");
        assert_eq!(rendre(&serde_json::json!(true)), "oui");
        assert_eq!(rendre(&serde_json::json!(12)), "12");
        assert_eq!(rendre(&serde_json::json!(["a", "b"])), r#"["a","b"]"#);
    }

    #[test]
    fn l_empreinte_depend_du_contenu_et_pas_de_l_ordre_des_colonnes_assemblees() {
        let l = |v: &str| LigneContenu {
            id: None,
            valeurs: vec![("a".into(), v.into())],
        };
        let t1 = TableContenu {
            nom: "x".into(),
            lignes: vec![l("1")],
        };
        let t2 = TableContenu {
            nom: "x".into(),
            lignes: vec![l("2")],
        };
        assert_ne!(
            empreinte_section(std::slice::from_ref(&t1)),
            empreinte_section(&[t2])
        );
        assert_eq!(
            empreinte_section(std::slice::from_ref(&t1)),
            empreinte_section(std::slice::from_ref(&t1))
        );
    }

    #[test]
    fn les_protections_ne_sont_jamais_lues_ni_nommees() {
        for t in TABLES_JAMAIS {
            let c = crate::registre_donnees::classer(t).expect("classée");
            // Même classées « exportables », elles sont écartées de tout fournisseur.
            let f = Fournisseur {
                code: "x",
                socle: "0",
                domaines: Box::leak(Box::new([c.domaine])),
                personne: false,
            };
            let lues = tables_du_fournisseur(&f);
            assert!(lues.iter().all(|x| x.nom != *t), "{t}");
        }
        assert!(colonne_interdite("adresse_protegee", "text"));
        assert!(colonne_interdite("passkey_secret", "text"));
        assert!(colonne_interdite("contenu", "bytea"));
        assert!(!colonne_interdite("nom", "text"));
    }

    /// R-RG14 : tout fournisseur déclaré a une implémentation (élève : moteur générique du registre ; personne : écrit,
    /// ou section « non couverte ») : aucune déclaration ne reste muette.
    #[test]
    fn chaque_fournisseur_a_une_implementation_ou_rend_non_couvert() {
        for f in FOURNISSEURS {
            if f.personne {
                assert!(
                    !regles_personne(f.code).is_empty(),
                    "{} : fournisseur « personne » sans règle",
                    f.code
                );
            } else {
                assert!(
                    regles_personne(f.code).is_empty(),
                    "{} : règles pour un fournisseur non écrit",
                    f.code
                );
            }
        }
        let ecrits: Vec<_> = FOURNISSEURS
            .iter()
            .filter(|f| f.personne)
            .map(|f| f.code)
            .collect();
        assert_eq!(
            ecrits,
            vec![
                "administratif",
                "personnels",
                "messagerie",
                "comptes",
                "documents"
            ]
        );
    }

    #[test]
    fn les_rattachements_indirects_visent_des_tables_exportables_sans_eleve_id() {
        for (t, _) in rattachements_indirects() {
            let c =
                crate::registre_donnees::classer(t).unwrap_or_else(|| panic!("{t} non classée"));
            assert!(
                c.export_portabilite,
                "{t} : rattachement indirect d'une table non exportable"
            );
            assert!(
                fournisseurs_de(c).next().is_some(),
                "{t} : sans fournisseur"
            );
        }
    }
}
