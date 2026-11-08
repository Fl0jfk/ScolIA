//! Admissions, préinscriptions, inscriptions et réinscriptions (socle 2, lot S2-3 ;
//! D-013, D-014, D-017 ; INS-01 → INS-15).
//!
//! * **Campagnes** (admission, réinscription) paramétrées par l'établissement :
//!   année cible, dates, niveaux (capacité), pièces.
//! * **Préinscription par la famille** : e-mail **vérifié** (code à 6 chiffres ou
//!   session existante, passkey comprise) **avant tout affichage** ; la réponse
//!   au démarrage est identique que le compte existe ou non (anti-énumération) ;
//!   « Utiliser mes informations ? » ; l'établissement reçoit **sa propre copie**
//!   du formulaire, rien d'autre.
//! * **Instruction** (pièces, étapes, commentaires, relances), **décision** ★,
//!   **validation** ★ : création de l'élève, des personnes, foyers et liens (S2-1)
//!   et publication de `inscription.validee` dans l'outbox (S2-0), dans la même
//!   transaction. Les effets inter-domaines (affectation, invitations, accueil)
//!   sont des **consommateurs** (`inscriptions_reactions.rs`).
//! * **Coordonnées bancaires** à la validation seulement : IBAN d'un autre
//!   établissement reproposé **masqué** si l'identité a été réutilisée ; un
//!   **nouveau mandat SEPA** est toujours signé.
//! * **Réinscription** : dossiers préparés à l'ouverture de la campagne, confirmés
//!   (ou départ annoncé) par les responsables, validés par l'administration.
//!
//! Droits : moteur D-017 (`inscriptions.gerer`, `inscriptions.decider`, `inscriptions.valider`) ; familles
//! par le jeton de préinscription (e-mail vérifié) ou par le lien actif (S2-1).
//! Toute requête est bornée à l'établissement ; une référence d'un autre
//! établissement répond « introuvable ».

// Lignes SQL lues en tuples (requêtes ponctuelles) : types longs assumés.
#![allow(clippy::type_complexity)]

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use domain_eleve::{normalize_iban, plan_creer_mandat_sepa, CreerMandatSepa, DomainError};
use ent_auth::{generate_reset_token, hash_token, Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_mail::OutgoingMail;
use ent_types::inscriptions::*;
use serde_json::{json, Value};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::dossier_personnes::{self, PersonneSaisie};
use crate::eleve_ports;
use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

/// Message affiché à la famille dans tous les cas (D-014).
pub const MESSAGE_CONFIDENTIALITE: &str = "Les informations saisies ici ne sont transmises qu'à cet établissement. Aucune donnée n'est partagée avec un autre établissement sans votre accord.";
/// Réponse au démarrage : identique que le compte existe ou non.
pub const MESSAGE_DEMARRER: &str = "Un code de vérification a été envoyé à cette adresse s'il est possible d'y écrire. Saisissez-le pour continuer.";
pub const CODE_TTL_MINUTES: i64 = 15;
pub const CODE_ESSAIS_MAX: i16 = 5;
pub const JETON_TTL_MINUTES: i64 = 120;
pub const INVITATION_TTL_JOURS: i64 = 14;
/// Délai minimal entre deux relances de pièces manquantes (INS-15).
pub const RELANCE_DELAI_JOURS: i64 = 3;
/// Dossiers déposés au plus par e-mail et par campagne (anti-abus).
pub const DOSSIERS_MAX_PAR_DEPOSANT: i64 = 10;

const T_DEMANDE: &str = "ent_mandat_sepa_demande";
const T_MANDAT: &str = "ent_fac_sepa_mandat";

pub const STATUTS_DOSSIER: [&str; 9] = [
    "brouillon",
    "deposee",
    "en_instruction",
    "incomplet",
    "liste_attente",
    "admis",
    "refuse",
    "validee",
    "desistement",
];

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("inscriptions", err))
}

fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

/// Jeton de préinscription absent, inconnu, expiré ou d'un autre établissement.
fn jeton_invalide() -> EntDbError {
    EntDbError::Auth(ent_auth::AuthError::InvalidSession)
}

fn opt(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn borne(v: Option<String>, max: usize, msg: &'static str) -> Result<Option<String>, EntDbError> {
    match v {
        Some(s) if s.chars().count() > max => Err(invalide(msg)),
        o => Ok(o),
    }
}

/// E-mail normalisé (minuscules) ou erreur de format.
pub fn normaliser_email(e: &str) -> Result<String, EntDbError> {
    let e = e.trim().to_lowercase();
    let ok = e.len() <= 254
        && e.split_once('@')
            .map(|(a, d)| {
                !a.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.')
            })
            .unwrap_or(false)
        && !e.chars().any(char::is_whitespace);
    if ok {
        Ok(e)
    } else {
        Err(invalide("adresse e-mail mal formée"))
    }
}

/// `FR76 •••• 1234` : quatre premiers et quatre derniers caractères seulement.
pub fn masquer_iban(iban: &str) -> String {
    let c: Vec<char> = iban.chars().filter(|c| !c.is_whitespace()).collect();
    if c.len() < 8 {
        return "••••".into();
    }
    let debut: String = c[..4].iter().collect();
    let fin: String = c[c.len() - 4..].iter().collect();
    format!("{debut} •••• {fin}")
}

fn initiales(nom: &str) -> String {
    nom.split_whitespace()
        .filter_map(|m| m.chars().next())
        .map(|c| format!("{}.", c.to_uppercase()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Code à 6 chiffres (aléatoire du système, via un UUID v4).
fn code_six_chiffres() -> String {
    let b = Uuid::new_v4().into_bytes();
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) % 1_000_000;
    format!("{n:06}")
}

/// Empreinte d'un code de vérification : HMAC-SHA-256 sous un secret serveur
/// (`ENT_SECRETS_KEY`, usage dédié). Un code à 6 chiffres n'a que 10⁶ valeurs : sans
/// secret, une base volée permettrait de le retrouver par force brute (I0, §2.2 pt 20).
fn empreinte_code_avec(
    secret: &ent_auth::SecretBox,
    demande_id: Uuid,
    code: &str,
) -> Result<String, EntDbError> {
    Ok(secret.hmac_hex(
        "preinscription-code-v1",
        format!("{demande_id}:{}", code.trim()).as_bytes(),
    )?)
}

/// Fail-closed hors labo/test si le secret serveur manque.
fn empreinte_code(demande_id: Uuid, code: &str) -> Result<String, EntDbError> {
    empreinte_code_avec(&ent_auth::SecretBox::from_env()?, demande_id, code)
}

/// `"2027-2028"` (années consécutives).
pub fn annee_cible_valide(a: &str) -> bool {
    let Some((d, f)) = a.split_once('-') else {
        return false;
    };
    match (d.parse::<i32>(), f.parse::<i32>()) {
        (Ok(d), Ok(f)) => d.to_string().len() == 4 && f == d + 1,
        _ => false,
    }
}

fn bornee(v: Option<String>, max: usize, msg: &'static str) -> Result<Option<String>, EntDbError> {
    match v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() <= max => Ok(Some(s)),
        Some(_) => Err(invalide(msg)),
    }
}

fn regime_valide(r: &Option<String>) -> Result<Option<String>, EntDbError> {
    match opt(r) {
        None => Ok(None),
        Some(r) if ["externe", "demi_pension", "interne"].contains(&r.as_str()) => Ok(Some(r)),
        Some(_) => Err(invalide("régime inconnu (externe, demi_pension, interne)")),
    }
}

fn lien_valide(l: &Option<String>) -> Result<String, EntDbError> {
    match opt(l) {
        None => Ok("autre".into()),
        Some(l) if ["mere", "pere", "tuteur", "autre"].contains(&l.as_str()) => Ok(l),
        Some(_) => Err(invalide("lien inconnu (mere, pere, tuteur, autre)")),
    }
}

/// Élève du formulaire, normalisé.
pub fn normaliser_eleve(e: &EleveFormulaireDto) -> Result<EleveFormulaireDto, EntDbError> {
    let nom = e.nom.trim().to_string();
    let prenom = e.prenom.trim().to_string();
    if nom.is_empty()
        || nom.chars().count() > 120
        || prenom.is_empty()
        || prenom.chars().count() > 120
    {
        return Err(invalide(
            "nom et prénom de l'élève obligatoires (120 caractères au plus)",
        ));
    }
    let sexe = match opt(&e.sexe).map(|s| s.to_uppercase()) {
        None => None,
        Some(s) if s == "F" || s == "M" => Some(s),
        Some(_) => return Err(invalide("sexe : F ou M")),
    };
    if let Some(d) = e.date_naissance {
        let annee = Utc::now().year();
        if d.year() < annee - 30 || d.year() > annee {
            return Err(invalide("date de naissance invraisemblable"));
        }
    }
    let ine = match opt(&e.ine) {
        None => None,
        Some(i) => Some(domain_eleve::normalize_ine(&i).map_err(|_| invalide("INE invalide"))?),
    };
    Ok(EleveFormulaireDto {
        nom,
        prenom,
        date_naissance: e.date_naissance,
        sexe,
        ine,
        niveau_demande: borne(opt(&e.niveau_demande), 40, "niveau trop long")?,
        regime_demande: regime_valide(&e.regime_demande)?,
        etablissement_origine: borne(
            opt(&e.etablissement_origine),
            200,
            "établissement d'origine trop long",
        )?,
    })
}

/// Responsable du formulaire, normalisé (mêmes règles que les personnes, S2-1).
pub fn normaliser_responsable(
    r: &ResponsableFormulaireDto,
) -> Result<ResponsableFormulaireDto, EntDbError> {
    let s = PersonneSaisie {
        nom: r.nom.clone(),
        prenom: r.prenom.clone(),
        civilite: r.civilite.clone(),
        email: r.email.clone(),
        telephone: r.telephone.clone(),
        telephone_2: None,
        adresse: r.adresse.clone(),
    }
    .normaliser()?;
    // D-014 : parents séparés → un e-mail par parent obligatoire.
    if !r.meme_foyer && s.email.is_none() {
        return Err(invalide(
            "parents séparés : un e-mail par parent est obligatoire",
        ));
    }
    Ok(ResponsableFormulaireDto {
        nom: s.nom,
        prenom: s.prenom,
        civilite: s.civilite,
        email: s.email,
        telephone: s.telephone,
        adresse: s.adresse,
        lien: Some(lien_valide(&r.lien)?),
        meme_foyer: r.meme_foyer,
        autorite_parentale: r.autorite_parentale,
        reserve: match &r.reserve {
            None => None,
            Some(d) => {
                let n = reserve_depuis_dto(d).normaliser()?;
                Some(ReserveResponsableDto {
                    adresse_a_proteger: n.adresse_a_proteger,
                    ne_pas_solliciter: n.ne_pas_solliciter,
                    ne_pas_solliciter_ref: n.ne_pas_solliciter_ref,
                    langue: n.langue,
                    besoin_interprete: n.besoin_interprete,
                    organisme: n.organisme,
                    nature_code: n.nature_code,
                })
            }
        },
    })
}

fn reserve_depuis_dto(d: &ReserveResponsableDto) -> dossier_personnes::ReserveDeclaree {
    dossier_personnes::ReserveDeclaree {
        adresse_a_proteger: d.adresse_a_proteger,
        ne_pas_solliciter: d.ne_pas_solliciter,
        ne_pas_solliciter_ref: d.ne_pas_solliciter_ref.clone(),
        langue: d.langue.clone(),
        besoin_interprete: d.besoin_interprete,
        organisme: d.organisme.clone(),
        nature_code: d.nature_code.clone(),
    }
}

/// Transitions d'instruction admises (personnel, `inscriptions.gerer`).
pub fn transition_instruction_admise(de: &str, vers: &str) -> bool {
    matches!(
        (de, vers),
        ("deposee", "en_instruction")
            | ("deposee", "incomplet")
            | ("en_instruction", "incomplet")
            | ("incomplet", "en_instruction")
            | ("liste_attente", "en_instruction")
    )
}

/// Décision possible depuis cet état (admission seulement).
pub fn decision_admise(de: &str) -> bool {
    matches!(
        de,
        "deposee" | "en_instruction" | "incomplet" | "liste_attente"
    )
}

/// Actions proposées au personnel pour un dossier (le serveur revérifie tout).
pub fn actions_personnel(
    nature: &str,
    statut: &str,
    gerer: bool,
    decider: bool,
    valider: bool,
) -> Vec<String> {
    let mut a = Vec::new();
    let ouvert = !matches!(statut, "validee" | "refuse" | "desistement" | "brouillon");
    if gerer && ouvert {
        a.push("commenter".into());
        a.push("pieces".into());
        a.push("relancer".into());
        for vers in ["en_instruction", "incomplet"] {
            if transition_instruction_admise(statut, vers) {
                a.push(format!("statut:{vers}"));
            }
        }
    }
    if decider && nature == "admission" && decision_admise(statut) {
        a.push("decider".into());
    }
    if valider {
        let validable = match nature {
            "admission" => statut == "admis",
            _ => matches!(statut, "deposee" | "en_instruction"),
        };
        if validable {
            a.push("valider".into());
        }
    }
    a
}

// ---------------------------------------------------------------------------
// Lignes SQL
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct CampagneRow {
    id: Uuid,
    nature: String,
    libelle: String,
    annee_cible: String,
    statut: String,
    date_ouverture: Option<NaiveDate>,
    date_cloture: Option<NaiveDate>,
    message_famille: Option<String>,
}

const COLS_CAMPAGNE: &str =
    "id, nature, libelle, annee_cible, statut, date_ouverture, date_cloture, message_famille";

#[derive(sqlx::FromRow, Clone)]
struct DossierRow {
    id: Uuid,
    campagne_id: Uuid,
    nature: String,
    statut: String,
    eleve_id: Option<Uuid>,
    eleve_nom: String,
    eleve_prenom: String,
    eleve_date_naissance: Option<NaiveDate>,
    eleve_sexe: Option<String>,
    eleve_ine: Option<String>,
    niveau_demande: Option<String>,
    regime_demande: Option<String>,
    parcours_demande: Option<String>,
    etablissement_origine: Option<String>,
    classe_cible_id: Option<Uuid>,
    deposant_email: Option<String>,
    reutilise_identite: bool,
    decision_motif: Option<String>,
    depose_le: Option<DateTime<Utc>>,
    valide_le: Option<DateTime<Utc>>,
    derniere_relance_le: Option<DateTime<Utc>>,
}

const COLS_DOSSIER: &str = "id, campagne_id, nature, statut, eleve_id, eleve_nom, eleve_prenom,
    eleve_date_naissance, eleve_sexe, eleve_ine, niveau_demande, regime_demande,
    parcours_demande, etablissement_origine, classe_cible_id, deposant_email,
    reutilise_identite, decision_motif, depose_le, valide_le, derniere_relance_le";

#[derive(sqlx::FromRow, Clone)]
struct ResponsableRow {
    id: Uuid,
    rang: i16,
    nom: String,
    prenom: Option<String>,
    civilite: Option<String>,
    email: Option<String>,
    telephone: Option<String>,
    adresse: Option<String>,
    lien: String,
    meme_foyer: bool,
    autorite_parentale: bool,
    personne_id: Option<Uuid>,
    ne_pas_solliciter: bool,
    ne_pas_solliciter_ref: Option<String>,
    adresse_a_proteger: bool,
    adresse_protegee: Option<String>,
    langue: Option<String>,
    besoin_interprete: bool,
    organisme: Option<String>,
    nature_code: Option<String>,
}

const COLS_RESPONSABLE: &str = "id, rang, nom, prenom, civilite, email, telephone, adresse, lien,
    meme_foyer, autorite_parentale, personne_id, ne_pas_solliciter, ne_pas_solliciter_ref,
    adresse_a_proteger, adresse_protegee, langue, besoin_interprete, organisme, nature_code";

impl ResponsableRow {
    /// Déclaration remise au socle 4 (le premier responsable est payeur à 100 %, comme avant).
    /// Une adresse à protéger est descellée ici pour le seul port, qui la reparque scellée.
    fn declaration(&self) -> Result<dossier_personnes::DeclarationResponsable, EntDbError> {
        let adresse = if self.adresse_a_proteger {
            ChampsSensibles::depuis_env()
                .ouvrir_opt(
                    "ent_inscription_responsable",
                    "adresse_protegee",
                    self.id,
                    self.adresse_protegee.clone(),
                )
                .map_err(EntDbError::Domain)?
        } else {
            self.adresse.clone()
        };
        let reference = ChampsSensibles::depuis_env()
            .ouvrir_opt(
                "ent_inscription_responsable",
                "ne_pas_solliciter_ref",
                self.id,
                self.ne_pas_solliciter_ref.clone(),
            )
            .map_err(EntDbError::Domain)?;
        Ok(dossier_personnes::DeclarationResponsable {
            cle: self.id,
            rang: self.rang,
            nom: self.nom.clone(),
            prenom: self.prenom.clone(),
            civilite: self.civilite.clone(),
            email: self.email.clone(),
            telephone: self.telephone.clone(),
            adresse,
            lien: self.lien.clone(),
            meme_foyer: self.meme_foyer,
            autorite_parentale: self.autorite_parentale,
            droits: dossier_personnes::DroitsDeclares {
                notes: false,
                vs: false,
                justifier: false,
                sorties: false,
            },
            payeur: self.rang == 1,
            part_paiement: if self.rang == 1 { Some(100) } else { None },
            reserve: dossier_personnes::ReserveDeclaree {
                adresse_a_proteger: self.adresse_a_proteger,
                ne_pas_solliciter: self.ne_pas_solliciter,
                ne_pas_solliciter_ref: reference,
                langue: self.langue.clone(),
                besoin_interprete: self.besoin_interprete,
                organisme: self.organisme.clone(),
                nature_code: self.nature_code.clone(),
            },
        })
    }
}

#[derive(Default)]
struct PlanRapprochement {
    eleve_existant: Option<Uuid>,
    /// Choix humains, transmis tels quels au port `integrer_declaration` du socle 4.
    resolutions: Vec<dossier_personnes::Resolution>,
}

struct DroitsLien {
    notes: bool,
    vs: bool,
    justifier: bool,
    sorties: bool,
}

/// `j***@exemple.fr` : l'écran de rapprochement n'affiche jamais l'adresse complète.
fn masquer_email(e: &str) -> String {
    match e.split_once('@') {
        Some((l, d)) => format!("{}***@{d}", l.chars().next().unwrap_or('*')),
        None => "***".into(),
    }
}

/// Contexte d'un jeton de préinscription valide.
#[derive(Debug, Clone)]
pub struct Verifie {
    pub email: String,
    pub user_id: Option<Uuid>,
}

#[allow(clippy::too_many_arguments)]
async fn publier(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate: AggregateKind,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: Value,
    actor: Option<Uuid>,
) -> Result<ent_events::MetierEvent, EntDbError> {
    Ok(conn
        .publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: type_name.to_string(),
            aggregate,
            aggregate_id,
            eleve_id,
            payload,
            actor_user_id: actor.map(|a| a.to_string()),
        })
        .await?)
}

#[allow(clippy::too_many_arguments)]
async fn suivi_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
    kind: &str,
    statut: Option<(&str, &str)>,
    texte: Option<&str>,
    visible_famille: bool,
    auteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_inscription_suivi_dossier
            (id, etablissement_id, dossier_id, kind, statut_avant, statut_apres, texte,
             visible_famille, auteur_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(dossier_id)
    .bind(kind)
    .bind(statut.map(|s| s.0))
    .bind(statut.map(|s| s.1))
    .bind(texte)
    .bind(visible_famille)
    .bind(auteur)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn dossier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<DossierRow, EntDbError> {
    sqlx::query_as::<_, DossierRow>(&format!(
        "SELECT {COLS_DOSSIER} FROM ent_inscription_dossier
          WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn changer_statut_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    vers: &str,
    texte: Option<&str>,
    auteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_inscription_dossier SET statut = $3, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(d.id)
    .bind(vers)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    suivi_tx(
        conn,
        etab,
        d.id,
        "statut",
        Some((&d.statut, vers)),
        texte,
        true,
        auteur,
    )
    .await
}

async fn commit(tx: Transaction<'_, Postgres>) -> Result<(), EntDbError> {
    tx.commit().await.map_err(db)
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    async fn ins_exiger(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        a: Action,
    ) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, actor, role, a).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    async fn ins_peut(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        a: Action,
    ) -> Result<bool, EntDbError> {
        self.a_action_etablissement(etab, actor, role, a).await
    }

    async fn ins_personnel(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<(bool, bool, bool), EntDbError> {
        let g = self
            .ins_peut(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let d = self
            .ins_peut(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let v = self
            .ins_peut(etab, actor, role, Action::InscriptionsValider)
            .await?;
        if !g && !d && !v {
            return Err(EntDbError::Forbidden);
        }
        Ok((g, d, v))
    }

    // -- Campagnes -----------------------------------------------------------

    async fn campagne_dto(
        &self,
        etab: Uuid,
        r: CampagneRow,
        compteurs: bool,
    ) -> Result<CampagneDto, EntDbError> {
        let niveaux: Vec<(String, Option<i32>)> = sqlx::query_as(
            "SELECT code, capacite FROM ent_inscription_campagne_niveau
              WHERE etablissement_id = $1 AND campagne_id = $2 ORDER BY code",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let pieces: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT code, libelle, obligatoire FROM ent_inscription_campagne_piece
              WHERE etablissement_id = $1 AND campagne_id = $2 ORDER BY ordre, code",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let compteurs = if compteurs {
            let c: Vec<(String, i64)> = sqlx::query_as(
                "SELECT statut, count(*) FROM ent_inscription_dossier
                  WHERE etablissement_id = $1 AND campagne_id = $2 GROUP BY statut ORDER BY statut",
            )
            .bind(etab)
            .bind(r.id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            c.into_iter()
                .map(|(statut, nombre)| CompteurStatutDto { statut, nombre })
                .collect()
        } else {
            Vec::new()
        };
        Ok(CampagneDto {
            id: r.id,
            nature: r.nature,
            libelle: r.libelle,
            annee_cible: r.annee_cible,
            statut: r.statut,
            date_ouverture: r.date_ouverture,
            date_cloture: r.date_cloture,
            message_famille: r.message_famille,
            niveaux: niveaux
                .into_iter()
                .map(|(code, capacite)| NiveauCampagneDto { code, capacite })
                .collect(),
            pieces: pieces
                .into_iter()
                .map(|(code, libelle, obligatoire)| PieceCampagneDto {
                    code,
                    libelle,
                    obligatoire,
                })
                .collect(),
            compteurs,
        })
    }

    async fn campagne_row(&self, etab: Uuid, id: Uuid) -> Result<CampagneRow, EntDbError> {
        sqlx::query_as::<_, CampagneRow>(&format!(
            "SELECT {COLS_CAMPAGNE} FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    fn valider_campagne(b: &CampagneBody) -> Result<(), EntDbError> {
        if !["admission", "reinscription"].contains(&b.nature.as_str()) {
            return Err(invalide("nature de campagne : admission ou reinscription"));
        }
        let l = b.libelle.trim();
        if l.is_empty() || l.chars().count() > 160 {
            return Err(invalide(
                "libellé de campagne obligatoire (160 caractères au plus)",
            ));
        }
        if !annee_cible_valide(b.annee_cible.trim()) {
            return Err(invalide("année cible au format 2027-2028"));
        }
        if let (Some(o), Some(c)) = (b.date_ouverture, b.date_cloture) {
            if c < o {
                return Err(invalide("la clôture précède l'ouverture"));
            }
        }
        if b.message_famille
            .as_deref()
            .is_some_and(|m| m.chars().count() > 2000)
        {
            return Err(invalide("message aux familles trop long"));
        }
        let mut codes = std::collections::HashSet::new();
        for p in &b.pieces {
            let ok = !p.code.is_empty()
                && p.code.len() <= 40
                && p.code
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !ok || p.libelle.trim().is_empty() || p.libelle.chars().count() > 160 {
                return Err(invalide(
                    "pièce : code (a-z, 0-9, _) et libellé obligatoires",
                ));
            }
            if !codes.insert(p.code.clone()) {
                return Err(invalide("code de pièce en double"));
            }
        }
        let mut niv = std::collections::HashSet::new();
        for n in &b.niveaux {
            if n.code.trim().is_empty() || n.code.len() > 40 || n.capacite.is_some_and(|c| c < 0) {
                return Err(invalide("niveau : code obligatoire, capacité positive"));
            }
            if !niv.insert(n.code.trim().to_string()) {
                return Err(invalide("niveau en double"));
            }
        }
        Ok(())
    }

    async fn ecrire_details_campagne(
        conn: &mut PgConnection,
        etab: Uuid,
        id: Uuid,
        b: &CampagneBody,
    ) -> Result<(), EntDbError> {
        sqlx::query("DELETE FROM ent_inscription_campagne_niveau WHERE etablissement_id = $1 AND campagne_id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM ent_inscription_campagne_piece WHERE etablissement_id = $1 AND campagne_id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        for n in &b.niveaux {
            sqlx::query(
                "INSERT INTO ent_inscription_campagne_niveau (campagne_id, etablissement_id, code, capacite)
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(id)
            .bind(etab)
            .bind(n.code.trim())
            .bind(n.capacite)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        for (i, p) in b.pieces.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_inscription_campagne_piece
                    (campagne_id, etablissement_id, code, libelle, obligatoire, ordre)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(id)
            .bind(etab)
            .bind(&p.code)
            .bind(p.libelle.trim())
            .bind(p.obligatoire)
            .bind(i16::try_from(i).unwrap_or(i16::MAX))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        Ok(())
    }

    /// INS-01 / INS-06 : créer une campagne (brouillon).
    pub async fn creer_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CampagneBody,
    ) -> Result<CampagneDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        Self::valider_campagne(b)?;
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_inscription_campagne
                (id, etablissement_id, nature, libelle, annee_cible, date_ouverture, date_cloture,
                 message_famille, created_by_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(&b.nature)
        .bind(b.libelle.trim())
        .bind(b.annee_cible.trim())
        .bind(b.date_ouverture)
        .bind(b.date_cloture)
        .bind(opt(&b.message_famille))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::ecrire_details_campagne(&mut tx, etab, id, b).await?;
        commit(tx).await?;
        let r = self.campagne_row(etab, id).await?;
        self.campagne_dto(etab, r, true).await
    }

    /// Modifier une campagne non fermée (niveaux et pièces : dossiers futurs seulement).
    pub async fn modifier_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &CampagneBody,
    ) -> Result<CampagneDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        Self::valider_campagne(b)?;
        let r = self.campagne_row(etab, id).await?;
        if r.statut == "fermee" {
            return Err(conflit("campagne fermée"));
        }
        if r.nature != b.nature {
            return Err(conflit("la nature d'une campagne ne change pas"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_inscription_campagne SET libelle = $3, annee_cible = $4, date_ouverture = $5,
                    date_cloture = $6, message_famille = $7, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(b.libelle.trim())
        .bind(b.annee_cible.trim())
        .bind(b.date_ouverture)
        .bind(b.date_cloture)
        .bind(opt(&b.message_famille))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::ecrire_details_campagne(&mut tx, etab, id, b).await?;
        commit(tx).await?;
        let r = self.campagne_row(etab, id).await?;
        self.campagne_dto(etab, r, true).await
    }

    /// Ouvrir une campagne. Réinscription : un dossier « brouillon » est préparé
    /// pour chaque élève actif (pré-rempli : identité, responsables ayant
    /// l'autorité parentale) ; rouvrir ne duplique rien.
    pub async fn ouvrir_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<OuvertureCampagneDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let r = self.campagne_row(etab, id).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let maj = sqlx::query(
            "UPDATE ent_inscription_campagne SET statut = 'ouverte', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await;
        if let Err(e) = maj {
            return Err(match map_sqlx("inscriptions", e) {
                DomainError::Conflit => {
                    conflit("une campagne de même nature est déjà ouverte pour cette année")
                }
                d => EntDbError::Domain(d),
            });
        }
        let mut prepares = 0_i64;
        if r.nature == "reinscription" {
            prepares = Self::preparer_reinscriptions_tx(&mut tx, etab, id).await?;
        }
        publier(
            &mut tx,
            etab,
            if r.nature == "reinscription" {
                types::REINSCRIPTION_CAMPAGNE_OUVERTE
            } else {
                types::ADMISSION_CAMPAGNE_OUVERTE
            },
            AggregateKind::Admission,
            id,
            None,
            json!({ "campagne_id": id, "nature": r.nature, "annee_cible": r.annee_cible, "dossiers_prepares": prepares }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        let r = self.campagne_row(etab, id).await?;
        Ok(OuvertureCampagneDto {
            campagne: self.campagne_dto(etab, r, true).await?,
            dossiers_prepares: prepares,
        })
    }

    async fn preparer_reinscriptions_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        campagne_id: Uuid,
    ) -> Result<i64, EntDbError> {
        let eleves: Vec<(
            Uuid,
            String,
            String,
            Option<NaiveDate>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, nom, prenom, date_naissance, sexe, ine FROM ent_eleve
                  WHERE etablissement_id = $1 AND status = 'actif' ORDER BY nom, prenom",
        )
        .bind(etab)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut n = 0_i64;
        for (eleve_id, nom, prenom, naissance, sexe, ine) in eleves {
            let dossier_id = Uuid::new_v4();
            let sexe = sexe
                .map(|s| s.to_uppercase())
                .filter(|s| s == "F" || s == "M");
            let ins = sqlx::query(
                "INSERT INTO ent_inscription_dossier
                    (id, etablissement_id, campagne_id, nature, statut, eleve_id, eleve_nom,
                     eleve_prenom, eleve_date_naissance, eleve_sexe, eleve_ine)
                 VALUES ($1,$2,$3,'reinscription','brouillon',$4,$5,$6,$7,$8,$9)
                 ON CONFLICT (campagne_id, eleve_id) WHERE eleve_id IS NOT NULL DO NOTHING",
            )
            .bind(dossier_id)
            .bind(etab)
            .bind(campagne_id)
            .bind(eleve_id)
            .bind(nom.trim())
            .bind(prenom.trim())
            .bind(naissance)
            .bind(sexe)
            .bind(ine.filter(|i| i.len() <= 20))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            if ins.rows_affected() == 0 {
                continue;
            }
            n += 1;
            // Responsables : autorité parentale, liens actifs, deux au plus.
            let resp: Vec<(Uuid, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String)> =
                sqlx::query_as(
                    "SELECT p.id, p.nom, p.prenom, p.civilite, p.email, p.telephone, p.adresse, l.lien
                       FROM ent_eleve_lien_actif l
                       JOIN ent_personne p ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
                      WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.autorite_parentale
                      ORDER BY l.est_payeur DESC, l.created_at, l.id
                      LIMIT 2",
                )
                .bind(etab)
                .bind(eleve_id)
                .fetch_all(&mut *conn)
                .await
                .map_err(db)?;
            for (i, (pid, pnom, pprenom, civ, email, tel, adr, lien)) in
                resp.into_iter().enumerate()
            {
                let lien = if ["mere", "pere", "tuteur"].contains(&lien.as_str()) {
                    lien
                } else {
                    "autre".into()
                };
                sqlx::query(
                    "INSERT INTO ent_inscription_responsable
                        (id, etablissement_id, dossier_id, rang, nom, prenom, civilite, email,
                         telephone, adresse, lien, meme_foyer, autorite_parentale, personne_id)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,true,true,$12)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(dossier_id)
                .bind(i16::try_from(i + 1).unwrap_or(2))
                .bind(pnom)
                .bind(pprenom)
                .bind(civ)
                .bind(email)
                .bind(tel)
                .bind(adr)
                .bind(lien)
                .bind(pid)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
            Self::copier_pieces_tx(conn, etab, campagne_id, dossier_id).await?;
        }
        Ok(n)
    }

    async fn copier_pieces_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        campagne_id: Uuid,
        dossier_id: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "INSERT INTO ent_inscription_piece (id, etablissement_id, dossier_id, code, libelle, obligatoire)
             SELECT gen_random_uuid(), etablissement_id, $3, code, libelle, obligatoire
               FROM ent_inscription_campagne_piece
              WHERE etablissement_id = $1 AND campagne_id = $2
             ON CONFLICT (dossier_id, code) DO NOTHING",
        )
        .bind(etab)
        .bind(campagne_id)
        .bind(dossier_id)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(())
    }

    pub async fn fermer_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<CampagneDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let r = self.campagne_row(etab, id).await?;
        if r.statut != "ouverte" {
            return Err(conflit("seule une campagne ouverte peut être fermée"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_inscription_campagne SET statut = 'fermee', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::ADMISSION_CAMPAGNE_FERMEE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "campagne_id": id }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        let r = self.campagne_row(etab, id).await?;
        self.campagne_dto(etab, r, true).await
    }

    pub async fn list_campagnes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<CampagneDto>, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let rows = sqlx::query_as::<_, CampagneRow>(&format!(
            "SELECT {COLS_CAMPAGNE} FROM ent_inscription_campagne WHERE etablissement_id = $1
              ORDER BY annee_cible DESC, created_at DESC"
        ))
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(self.campagne_dto(etab, r, true).await?);
        }
        Ok(out)
    }

    // -- Lecture des dossiers -----------------------------------------------

    async fn dossier_row(&self, etab: Uuid, id: Uuid) -> Result<DossierRow, EntDbError> {
        sqlx::query_as::<_, DossierRow>(&format!(
            "SELECT {COLS_DOSSIER} FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    async fn responsables_dossier(
        &self,
        etab: Uuid,
        id: Uuid,
    ) -> Result<Vec<ResponsableRow>, EntDbError> {
        sqlx::query_as::<_, ResponsableRow>(
            &format!("SELECT {COLS_RESPONSABLE}
               FROM ent_inscription_responsable
              WHERE etablissement_id = $1 AND dossier_id = $2 ORDER BY rang"),
        )
        .bind(etab)
        .bind(id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Dossier complet. `famille` : commentaires internes retirés.
    async fn dossier_dto(
        &self,
        etab: Uuid,
        d: DossierRow,
        famille: bool,
        actions: Vec<String>,
    ) -> Result<DossierDto, EntDbError> {
        let campagne = self.campagne_row(etab, d.campagne_id).await?;
        let invitations: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT responsable_id, statut FROM ent_inscription_invitation
              WHERE etablissement_id = $1 AND dossier_id = $2",
        )
        .bind(etab)
        .bind(d.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let responsables = self
            .responsables_dossier(etab, d.id)
            .await?
            .into_iter()
            .map(|r| ResponsableDossierDto {
                rang: r.rang,
                invitation: invitations
                    .iter()
                    .find(|(id, _)| *id == r.id)
                    .map(|(_, s)| s.clone()),
                formulaire: ResponsableFormulaireDto {
                    nom: r.nom,
                    prenom: r.prenom,
                    civilite: r.civilite,
                    email: r.email,
                    telephone: r.telephone,
                    adresse: r.adresse,
                    lien: Some(r.lien),
                    meme_foyer: r.meme_foyer,
                    autorite_parentale: r.autorite_parentale,
                    // Jamais rendu à la famille (le second parent pourrait lire le dossier) ;
                    // au personnel : les indicateurs, jamais l'adresse ni la référence.
                    reserve: (!famille).then(|| ReserveResponsableDto {
                        adresse_a_proteger: r.adresse_a_proteger,
                        ne_pas_solliciter: r.ne_pas_solliciter,
                        ne_pas_solliciter_ref: None,
                        langue: r.langue,
                        besoin_interprete: r.besoin_interprete,
                        organisme: r.organisme,
                        nature_code: r.nature_code,
                    }),
                },
            })
            .collect();
        let pieces: Vec<(String, String, bool, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT code, libelle, obligatoire, statut, nom_fichier, motif_refus FROM ent_inscription_piece
              WHERE etablissement_id = $1 AND dossier_id = $2 ORDER BY obligatoire DESC, code",
        )
        .bind(etab)
        .bind(d.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let suivi: Vec<(DateTime<Utc>, String, Option<String>, Option<String>, bool)> = sqlx::query_as(
            "SELECT created_at, kind, statut_apres, texte, visible_famille FROM ent_inscription_suivi_dossier
              WHERE etablissement_id = $1 AND dossier_id = $2 AND (visible_famille OR NOT $3)
              ORDER BY created_at, id",
        )
        .bind(etab)
        .bind(d.id)
        .bind(famille)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(DossierDto {
            id: d.id,
            campagne_id: d.campagne_id,
            campagne_libelle: campagne.libelle,
            nature: d.nature,
            statut: d.statut,
            eleve: EleveFormulaireDto {
                nom: d.eleve_nom,
                prenom: d.eleve_prenom,
                date_naissance: d.eleve_date_naissance,
                sexe: d.eleve_sexe,
                ine: d.eleve_ine,
                niveau_demande: d.niveau_demande,
                regime_demande: d.regime_demande,
                etablissement_origine: d.etablissement_origine,
            },
            eleve_id: d.eleve_id,
            classe_cible_id: d.classe_cible_id,
            responsables,
            pieces: pieces
                .into_iter()
                .map(
                    |(code, libelle, obligatoire, statut, nom_fichier, motif_refus)| {
                        PieceDossierDto {
                            code,
                            libelle,
                            obligatoire,
                            statut,
                            nom_fichier,
                            motif_refus,
                        }
                    },
                )
                .collect(),
            suivi: suivi
                .into_iter()
                .map(
                    |(quand, kind, statut_apres, texte, visible_famille)| SuiviDossierDto {
                        quand,
                        kind,
                        statut_apres,
                        texte,
                        visible_famille,
                    },
                )
                .collect(),
            decision_motif: d.decision_motif,
            depose_le: d.depose_le,
            valide_le: d.valide_le,
            reutilise_identite: d.reutilise_identite,
            actions,
        })
    }

    async fn resumes(
        &self,
        etab: Uuid,
        filtre_sql: &str,
        binds: (Option<Uuid>, Option<String>),
    ) -> Result<Vec<DossierResumeDto>, EntDbError> {
        let sql = format!(
            "SELECT d.id, d.campagne_id, d.nature, d.statut, d.eleve_nom, d.eleve_prenom, d.niveau_demande,
                    d.eleve_id, d.depose_le,
                    (SELECT count(*) FROM ent_inscription_piece p
                      WHERE p.dossier_id = d.id AND p.obligatoire AND p.statut <> 'acceptee') AS manquantes
               FROM ent_inscription_dossier d
              WHERE d.etablissement_id = $1 {filtre_sql}
              ORDER BY d.depose_le DESC NULLS LAST, d.eleve_nom, d.eleve_prenom
              LIMIT 2000"
        );
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<Uuid>,
            Option<DateTime<Utc>>,
            i64,
        )> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(binds.0)
            .bind(binds.1)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| DossierResumeDto {
                id: r.0,
                campagne_id: r.1,
                nature: r.2,
                statut: r.3,
                eleve_nom: r.4,
                eleve_prenom: r.5,
                niveau_demande: r.6,
                eleve_id: r.7,
                depose_le: r.8,
                pieces_manquantes: r.9,
            })
            .collect())
    }

    pub async fn list_dossiers_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Option<Uuid>,
        statut: Option<String>,
    ) -> Result<Vec<DossierResumeDto>, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        if let Some(s) = &statut {
            if !STATUTS_DOSSIER.contains(&s.as_str()) {
                return Err(invalide("statut de dossier inconnu"));
            }
        }
        self.resumes(
            etab,
            "AND ($2::uuid IS NULL OR d.campagne_id = $2) AND ($3::text IS NULL OR d.statut = $3)",
            (campagne_id, statut),
        )
        .await
    }

    pub async fn dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<DossierDto, EntDbError> {
        let (g, dec, val) = self.ins_personnel(etab, actor, role).await?;
        let d = self.dossier_row(etab, id).await?;
        let actions = actions_personnel(&d.nature, &d.statut, g, dec, val);
        self.dossier_dto(etab, d, false, actions).await
    }

    // -- Instruction (INS-03, INS-15) ----------------------------------------

    pub async fn changer_statut_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &StatutBody,
    ) -> Result<DossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let motif = opt(&b.motif);
        if motif.as_deref().is_some_and(|m| m.chars().count() > 1000) {
            return Err(invalide("motif trop long"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        if !transition_instruction_admise(&d.statut, &b.statut) {
            return Err(conflit(
                "changement d'étape impossible depuis l'état actuel",
            ));
        }
        if b.statut == "incomplet" && motif.is_none() {
            return Err(invalide("motif obligatoire pour un dossier incomplet"));
        }
        changer_statut_tx(&mut tx, etab, &d, &b.statut, motif.as_deref(), Some(actor)).await?;
        publier(
            &mut tx,
            etab,
            types::ADMISSION_ETAPE_CHANGEE,
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "de": d.statut, "vers": b.statut }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        self.dossier_for_actor(etab, actor, role, id).await
    }

    pub async fn decider_piece_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        code: &str,
        b: &PieceDecisionBody,
    ) -> Result<DossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let motif = opt(&b.motif);
        match b.statut.as_str() {
            "acceptee" => {}
            "refusee" if motif.is_some() => {}
            "refusee" => return Err(invalide("motif obligatoire pour refuser une pièce")),
            _ => return Err(invalide("statut de pièce : acceptee ou refusee")),
        }
        if motif.as_deref().is_some_and(|m| m.chars().count() > 500) {
            return Err(invalide("motif trop long"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        if matches!(d.statut.as_str(), "validee" | "refuse" | "desistement") {
            return Err(conflit("dossier clos"));
        }
        let n = sqlx::query(
            "UPDATE ent_inscription_piece SET statut = $4, motif_refus = $5, maj_par = $6, maj_le = now()
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(code)
        .bind(&b.statut)
        .bind(if b.statut == "refusee" { motif.clone() } else { None })
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if n.rows_affected() == 0 {
            return Err(introuvable());
        }
        let texte = if b.statut == "acceptee" {
            format!("Pièce « {code} » acceptée")
        } else {
            format!("Pièce « {code} » refusée : {}", motif.unwrap_or_default())
        };
        suivi_tx(
            &mut tx,
            etab,
            id,
            "piece",
            None,
            Some(&texte),
            true,
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        self.dossier_for_actor(etab, actor, role, id).await
    }

    pub async fn commenter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &CommentaireBody,
    ) -> Result<DossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let t = b.texte.trim();
        if t.is_empty() || t.chars().count() > 2000 {
            return Err(invalide("commentaire vide ou trop long (2000 caractères)"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        dossier_tx(&mut tx, etab, id).await?;
        suivi_tx(
            &mut tx,
            etab,
            id,
            "commentaire",
            None,
            Some(t),
            b.visible_famille,
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        self.dossier_for_actor(etab, actor, role, id).await
    }

    /// INS-15 : relancer les pièces obligatoires manquantes (une relance tous
    /// les [`RELANCE_DELAI_JOURS`] jours au plus). Renvoie le nombre de pièces relancées.
    pub async fn relancer_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<i64, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        if !matches!(
            d.statut.as_str(),
            "deposee" | "en_instruction" | "incomplet" | "admis" | "brouillon"
        ) {
            return Err(conflit("dossier sans relance possible dans cet état"));
        }
        if d.derniere_relance_le
            .is_some_and(|r| Utc::now() - r < Duration::days(RELANCE_DELAI_JOURS))
        {
            return Err(conflit("relance trop récente"));
        }
        let manquantes: Vec<(String,)> = sqlx::query_as(
            "SELECT libelle FROM ent_inscription_piece
              WHERE etablissement_id = $1 AND dossier_id = $2 AND obligatoire
                AND statut IN ('attendue', 'refusee') ORDER BY code",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if manquantes.is_empty() {
            return Err(conflit("aucune pièce obligatoire manquante"));
        }
        // A0-c : aucune relance vers une personne « ne pas solliciter » (le destinataire d'une
        // relance est l'adresse du déposant) : rien n'est tracé ni envoyé, le personnel agit à la main.
        if let Some(to) = d.deposant_email.as_deref() {
            let (protege,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_personne_reserve nps
                                  JOIN ent_personne p ON p.id = nps.personne_id
                                   AND p.etablissement_id = nps.etablissement_id
                                 WHERE nps.etablissement_id = $1 AND nps.ne_pas_solliciter
                                   AND lower(p.email) = lower($2))
                    OR EXISTS (SELECT 1 FROM ent_inscription_responsable r
                                WHERE r.etablissement_id = $1 AND r.dossier_id = $3
                                  AND r.ne_pas_solliciter AND lower(r.email) = lower($2))",
            )
            .bind(etab)
            .bind(to)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if protege {
                return Err(conflit(
                    "relance impossible : le destinataire est marqué « ne pas solliciter »",
                ));
            }
        }
        sqlx::query(
            "UPDATE ent_inscription_dossier SET derniere_relance_le = now(), updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let liste = manquantes
            .iter()
            .map(|(l,)| format!("- {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        suivi_tx(
            &mut tx,
            etab,
            id,
            "relance",
            None,
            Some("Relance des pièces manquantes"),
            true,
            Some(actor),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::ADMISSION_RELANCE_ENVOYEE,
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "pieces": manquantes.len() }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        // Envoi après validation : la relance reste tracée même si le mail échoue.
        if let Some(to) = d.deposant_email.clone() {
            let res = self
                .mailer()
                .send(OutgoingMail {
                    to,
                    subject: "Pièces manquantes de votre dossier d'inscription".into(),
                    body_text: format!(
                        "Bonjour,\n\nLe dossier d'inscription de {} {} attend encore les pièces suivantes :\n{liste}\n\n\
Vous pouvez les déposer depuis l'espace de préinscription de l'établissement.\n",
                        d.eleve_prenom, d.eleve_nom
                    ),
                    attachments: vec![],
                })
                .await;
            // I0 pt 17 : un échec d'envoi est tracé dans l'historique du dossier (jamais avalé).
            let parti = matches!(&res, Ok(r) if matches!(r.status, ent_mail::MailSendStatus::Sent | ent_mail::MailSendStatus::DryRun));
            if !parti {
                tracing::warn!(dossier = %id, "relance : message non envoyé");
                let mut tx = self.pool().begin().await.map_err(db)?;
                suivi_tx(
                    &mut tx,
                    etab,
                    id,
                    "relance",
                    None,
                    Some("Relance enregistrée mais e-mail non envoyé"),
                    false,
                    Some(actor),
                )
                .await?;
                commit(tx).await?;
            }
        }
        Ok(i64::try_from(manquantes.len()).unwrap_or(i64::MAX))
    }

    // -- Décision et validation (INS-04, INS-05) ------------------------------

    pub async fn decider_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &DecisionBody,
    ) -> Result<DossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let motif = opt(&b.motif);
        if !["admis", "liste_attente", "refuse"].contains(&b.decision.as_str()) {
            return Err(invalide("décision : admis, liste_attente ou refuse"));
        }
        if b.decision != "admis" && motif.is_none() {
            return Err(invalide(
                "motif obligatoire pour un refus ou une liste d'attente",
            ));
        }
        if motif.as_deref().is_some_and(|m| m.chars().count() > 1000) {
            return Err(invalide("motif trop long"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        if d.nature != "admission" {
            return Err(conflit(
                "une réinscription se valide sans décision d'admission",
            ));
        }
        if !decision_admise(&d.statut) || d.statut == b.decision {
            return Err(conflit("décision impossible depuis l'état actuel"));
        }
        if b.decision == "admis" {
            // Capacité du niveau (verrou sur la campagne : deux admissions
            // concurrentes ne dépassent pas la capacité).
            sqlx::query("SELECT id FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                .bind(etab)
                .bind(d.campagne_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            if let Some(niveau) = &d.niveau_demande {
                let cap: Option<(Option<i32>,)> = sqlx::query_as(
                    "SELECT capacite FROM ent_inscription_campagne_niveau
                      WHERE etablissement_id = $1 AND campagne_id = $2 AND code = $3",
                )
                .bind(etab)
                .bind(d.campagne_id)
                .bind(niveau)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                if let Some((Some(cap),)) = cap {
                    let (pris,): (i64,) = sqlx::query_as(
                        "SELECT count(*) FROM ent_inscription_dossier
                          WHERE etablissement_id = $1 AND campagne_id = $2 AND niveau_demande = $3
                            AND statut IN ('admis', 'validee')",
                    )
                    .bind(etab)
                    .bind(d.campagne_id)
                    .bind(niveau)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    if pris >= i64::from(cap) {
                        return Err(conflit(
                            "capacité du niveau atteinte : liste d'attente possible",
                        ));
                    }
                }
            }
        }
        sqlx::query(
            "UPDATE ent_inscription_dossier SET decision_motif = $3, decide_par = $4, decide_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&motif)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        changer_statut_tx(
            &mut tx,
            etab,
            &d,
            &b.decision,
            motif.as_deref(),
            Some(actor),
        )
        .await?;
        // Le motif (texte libre) n'entre pas dans l'événement (§4.1-3).
        publier(
            &mut tx,
            etab,
            types::ADMISSION_DECIDEE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "dossier_id": id, "decision": b.decision }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        self.dossier_for_actor(etab, actor, role, id).await
    }

    /// INS-05 ★ : valider l'inscription. Une seule transaction : élève (admission),
    /// personnes, foyers, liens (S2-1), statut du dossier et `inscription.validee`.
    pub async fn valider_inscription_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ValidationBody,
    ) -> Result<ValidationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsValider)
            .await?;
        let regime = regime_valide(&b.regime)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        let pret = match d.nature.as_str() {
            "admission" => d.statut == "admis",
            _ => matches!(d.statut.as_str(), "deposee" | "en_instruction"),
        };
        if !pret {
            return Err(conflit(if d.nature == "admission" {
                "seul un dossier admis peut être validé"
            } else {
                "la réinscription doit être confirmée par la famille"
            }));
        }
        let (manquantes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_piece
              WHERE etablissement_id = $1 AND dossier_id = $2 AND obligatoire AND statut <> 'acceptee'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if manquantes > 0 {
            return Err(conflit(
                "dossier incomplet : pièces obligatoires non acceptées",
            ));
        }
        let classe = match b.classe_id.or(d.classe_cible_id) {
            None => None,
            Some(c) => {
                let r: Option<(Uuid, Option<String>)> = sqlx::query_as(
                    "SELECT c.id, c.niveau FROM ent_classe c WHERE c.etablissement_id = $1 AND c.id = $2
                       AND c.archivee_le IS NULL",
                )
                .bind(etab)
                .bind(c)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                Some(
                    r.ok_or(EntDbError::Domain(DomainError::ReferenceInconnue))?
                        .0,
                )
            }
        };
        let regime = regime.or(d.regime_demande.clone());
        let niveau = bornee(b.niveau.clone().or(d.niveau_demande.clone()), 40, "niveau trop long")?;
        let parcours = bornee(
            b.parcours.clone().or(d.parcours_demande.clone()),
            120,
            "parcours trop long",
        )?;
        // Arrivée en cours d'année = classe choisie ET classe de l'année courante. Sinon aucune
        // affectation : la constitution des classes relève du socle 5 (M10, §2.2 n° 6).
        let classe_affectation = match classe {
            Some(c) if eleve_ports::classe_de_l_annee_courante_tx(&mut tx, etab, c).await? => {
                Some(c)
            }
            _ => None,
        };
        let responsables = Self::responsables_tx(&mut tx, etab, id).await?;

        let mut payeurs = Vec::new();
        let mut resolutions = Value::Null;
        let eleve_id = match (d.nature.as_str(), d.eleve_id) {
            ("reinscription", Some(e)) => {
                payeurs.extend(responsables.iter().filter_map(|r| r.personne_id));
                e
            }
            _ => {
                // Rien n'est rapproché ni créé sans décision humaine explicite.
                let plan = Self::planifier_tx(&mut tx, etab, &d, &responsables, b).await?;
                resolutions = json!({
                    "eleve": b.eleve,
                    "responsables": b.responsables,
                });
                let eleve_id = match plan.eleve_existant {
                    Some(e) => e,
                    None => {
                        Self::creer_eleve_tx(&mut tx, etab, &d, regime.as_deref(), actor).await?
                    }
                };
                payeurs = Self::rattacher_responsables_tx(
                    &mut tx,
                    etab,
                    eleve_id,
                    &responsables,
                    &plan,
                    actor,
                )
                .await?;
                eleve_id
            }
        };
        let campagne: (String,) =
            sqlx::query_as("SELECT annee_cible FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(d.campagne_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        // Inscription annuelle PRÉVUE (socle 2) : année, niveau, parcours, régime.
        let inscription_annuelle_id = eleve_ports::enregistrer_inscription_annuelle_tx(
            &mut tx,
            etab,
            &eleve_ports::InscriptionAnnuellePrevue {
                eleve_id,
                annee_scolaire: &campagne.0,
                niveau: niveau.as_deref(),
                parcours: parcours.as_deref(),
                regime: regime.as_deref(),
                source: if d.nature == "reinscription" {
                    eleve_ports::SourceInscription::Reinscription
                } else {
                    eleve_ports::SourceInscription::Admission
                },
                source_ref: Some(id),
            },
            actor,
        )
        .await?;
        // Arrivée en cours d'année : affectation initiale par la commande du référentiel.
        let affecte_en_classe = match classe_affectation {
            Some(c) => {
                eleve_ports::affecter_arrivee_en_cours_d_annee_tx(&mut tx, etab, eleve_id, c, "saisie")
                    .await?
            }
            None => false,
        };
        sqlx::query(
            "UPDATE ent_inscription_dossier SET statut = 'validee', eleve_id = $3, classe_cible_id = $4,
                    regime_demande = $5, valide_par = $6, valide_le = now(), updated_at = now(),
                    resolutions = $7, niveau_demande = $8, parcours_demande = $9,
                    inscription_annuelle_id = $10
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(eleve_id)
        .bind(classe)
        .bind(&regime)
        .bind(actor)
        .bind(resolutions)
        .bind(&niveau)
        .bind(&parcours)
        .bind(inscription_annuelle_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        suivi_tx(
            &mut tx,
            etab,
            id,
            "statut",
            Some((&d.statut, "validee")),
            Some("Inscription validée"),
            true,
            Some(actor),
        )
        .await?;
        // Charge utile minimale (§4.2) : identifiants, pas de coordonnées ni d'IBAN.
        let evt = publier(
            &mut tx,
            etab,
            types::INSCRIPTION_VALIDEE,
            AggregateKind::Eleve,
            eleve_id,
            Some(eleve_id),
            json!({
                "dossier_id": id,
                "eleve_id": eleve_id,
                "nature": d.nature,
                "annee_cible": campagne.0,
                "classe_id": classe_affectation,
                "inscription_annuelle_id": inscription_annuelle_id,
                "niveau": niveau,
                "parcours": parcours,
                "regime": regime,
                "payeurs": payeurs,
            }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        Ok(ValidationDto {
            dossier_id: id,
            eleve_id,
            evenement_id: evt.id,
            inscription_annuelle_id: Some(inscription_annuelle_id),
            affecte_en_classe,
        })
    }

    // -- Rapprochement humain (I0, §2.2 pts 1-3) ------------------------------

    /// Fiches élève qui pourraient être la même personne (homonyme exact ou INE).
    async fn candidats_eleve_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        d: &DossierRow,
    ) -> Result<Vec<EleveCandidatDto>, EntDbError> {
        let rows: Vec<(Uuid, String, String, Option<NaiveDate>, String, bool, i64)> = sqlx::query_as(
            "SELECT id, nom, prenom, date_naissance, status,
                    ($5::text IS NOT NULL AND ine = $5) AS ine_identique,
                    (SELECT count(*) FROM ent_eleve_lien_actif l
                      WHERE l.etablissement_id = $1 AND l.eleve_id = ent_eleve.id) AS nb_liens
               FROM ent_eleve
              WHERE etablissement_id = $1
                AND ((lower(nom) = lower($2) AND lower(prenom) = lower($3)
                      AND date_naissance IS NOT DISTINCT FROM $4)
                     OR ($5::text IS NOT NULL AND ine = $5))
              ORDER BY created_at LIMIT 10",
        )
        .bind(etab)
        .bind(&d.eleve_nom)
        .bind(&d.eleve_prenom)
        .bind(d.eleve_date_naissance)
        .bind(&d.eleve_ine)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(eleve_id, nom, prenom, date_naissance, statut, ine_identique, responsables_lies)| {
                    EleveCandidatDto {
                        eleve_id,
                        nom,
                        prenom,
                        date_naissance,
                        statut,
                        ine_identique,
                        responsables_lies,
                    }
                },
            )
            .collect())
    }

    /// Personnes de l'établissement qui pourraient être ce responsable : lecture du port
    /// `correspondances` du socle 4 (jamais de lien implicite).
    async fn candidats_personne_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        r: &ResponsableRow,
    ) -> Result<Vec<PersonneCandidateDto>, EntDbError> {
        let cands = dossier_personnes::correspondances_tx(conn, etab, &r.declaration()?).await?;
        Ok(cands
            .into_iter()
            .map(|c| PersonneCandidateDto {
                personne_id: c.personne_id,
                nom: c.nom,
                prenom: c.prenom,
                email_masque: c.email_masque,
                raison: c.raison.into(),
                a_un_compte: c.a_un_compte,
                foyers: c
                    .foyers
                    .into_iter()
                    .map(|f| FoyerCandidatDto {
                        foyer_id: f.foyer_id,
                        libelle: f.libelle,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Correspondances possibles d'un dossier (écran de validation) — droit `valider`.
    pub async fn correspondances_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<CorrespondancesDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsValider)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        let responsables = Self::responsables_tx(&mut tx, etab, id).await?;
        let mut out = CorrespondancesDto {
            eleves: vec![],
            responsables: vec![],
        };
        if d.nature == "admission" && d.eleve_id.is_none() {
            out.eleves = Self::candidats_eleve_tx(&mut tx, etab, &d).await?;
            for r in &responsables {
                out.responsables.push(ResponsableCorrespondancesDto {
                    responsable_id: r.id,
                    rang: r.rang,
                    candidats: Self::candidats_personne_tx(&mut tx, etab, r).await?,
                });
            }
        }
        tx.rollback().await.map_err(db)?;
        Ok(out)
    }

    async fn responsables_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        dossier_id: Uuid,
    ) -> Result<Vec<ResponsableRow>, EntDbError> {
        sqlx::query_as(&format!(
            "SELECT {COLS_RESPONSABLE}
               FROM ent_inscription_responsable WHERE etablissement_id = $1 AND dossier_id = $2
              ORDER BY rang"
        ))
        .bind(etab)
        .bind(dossier_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)
    }

    /// Contrôle les choix humains contre les correspondances : un responsable ou un
    /// élève qui a des correspondances possibles ne passe pas sans décision explicite.
    async fn planifier_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        d: &DossierRow,
        responsables: &[ResponsableRow],
        b: &ValidationBody,
    ) -> Result<PlanRapprochement, EntDbError> {
        let mut plan = PlanRapprochement::default();
        // Élève : rattachement à la fiche existante (ancien élève) ou création confirmée.
        let cands = Self::candidats_eleve_tx(conn, etab, d).await?;
        match &b.eleve {
            None if !cands.is_empty() => {
                return Err(conflit(
                    "une fiche élève correspond déjà : choisissez le rattachement ou la création",
                ))
            }
            None | Some(ChoixEleve::Nouveau) => {
                if cands.iter().any(|c| c.ine_identique) {
                    return Err(conflit(
                        "INE identique à une fiche existante : le rattachement est obligatoire",
                    ));
                }
            }
            Some(ChoixEleve::Existant {
                eleve_id,
                confirme_responsables_existants,
            }) => {
                let ok: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(eleve_id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                ok.ok_or(EntDbError::Domain(DomainError::ReferenceInconnue))?;
                let (liens,): (i64,) = sqlx::query_as(
                    "SELECT count(*) FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND eleve_id = $2",
                )
                .bind(etab)
                .bind(eleve_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                if liens > 0 && !confirme_responsables_existants {
                    return Err(conflit(
                        "cet élève a déjà des responsables : confirmation explicite requise avant tout rattachement",
                    ));
                }
                plan.eleve_existant = Some(*eleve_id);
            }
        }
        for res in &b.responsables {
            if !responsables.iter().any(|r| r.id == res.responsable_id) {
                return Err(invalide("responsable inconnu dans ce dossier"));
            }
        }
        // Les contrôles de rapprochement (correspondance sans choix, foyer à confirmer, personne
        // choisie deux fois…) sont ceux du port `integrer_declaration` : ils tournent dans la même
        // transaction que l'écriture, donc tout échoue ensemble.
        for res in &b.responsables {
            plan.resolutions.push(dossier_personnes::Resolution {
                cle: res.responsable_id,
                personne: Some(match &res.personne {
                    ChoixPersonne::Nouvelle => dossier_personnes::ResolutionPersonne::NouvellePersonne,
                    ChoixPersonne::Existante { personne_id } => {
                        dossier_personnes::ResolutionPersonne::MemePersonne(*personne_id)
                    }
                }),
                foyer: res.foyer.as_ref().map(|f| match f {
                    ChoixFoyer::Nouveau => dossier_personnes::ResolutionFoyer::NouveauFoyer,
                    ChoixFoyer::Existant { foyer_id } => {
                        dossier_personnes::ResolutionFoyer::MemeFoyer(*foyer_id)
                    }
                }),
            });
        }
        Ok(plan)
    }

    /// Droits d'un lien selon le paramétrage de l'établissement ; à défaut, seul un
    /// titulaire de l'autorité parentale voit notes et vie scolaire (jamais codé en dur à « tout »).
    async fn droits_lien_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        lien: &str,
        autorite: bool,
    ) -> Result<DroitsLien, EntDbError> {
        let p: Option<(bool, bool, bool, bool)> = sqlx::query_as(
            "SELECT peut_voir_notes, peut_voir_vs, peut_justifier_absence, peut_autoriser_sorties
               FROM ent_inscription_parametre_lien WHERE etablissement_id = $1 AND lien = $2",
        )
        .bind(etab)
        .bind(lien)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        Ok(match p {
            Some((n, v, j, s)) => DroitsLien {
                notes: n,
                vs: v,
                justifier: j,
                sorties: s,
            },
            None => DroitsLien {
                notes: autorite,
                vs: autorite,
                justifier: autorite,
                sorties: false,
            },
        })
    }

    /// Fiche élève : création par la **commande du socle 2** (`eleve_ports::creer_eleve_tx`) ;
    /// ce module n'écrit jamais lui-même dans `ent_eleve`. Élève « préinscrit » tant qu'aucune
    /// classe de l'année courante ne lui est affectée (arrivée en cours d'année seulement).
    async fn creer_eleve_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        d: &DossierRow,
        regime: Option<&str>,
        actor: Uuid,
    ) -> Result<Uuid, EntDbError> {
        eleve_ports::creer_eleve_tx(
            conn,
            etab,
            &eleve_ports::NouvelEleve {
                nom: &d.eleve_nom,
                prenom: &d.eleve_prenom,
                ine: d.eleve_ine.as_deref(),
                sexe: d.eleve_sexe.as_deref(),
                date_naissance: d.eleve_date_naissance,
                regime,
                statut: eleve_ports::StatutInitial::Preinscrit,
            },
            eleve_ports::OrigineFait {
                source: eleve_ports::SourceEleve::Admission,
                source_ref: Some(d.id),
            },
            actor,
        )
        .await
    }

    /// Personnes, foyers et liens : **une seule commande du socle 4**
    /// (`integrer_declaration_tx`), dans la transaction de la validation. Un lien vers une
    /// personne qui a déjà un compte reste en attente de sa confirmation. Renvoie les personnes
    /// payeuses.
    async fn rattacher_responsables_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve_id: Uuid,
        responsables: &[ResponsableRow],
        plan: &PlanRapprochement,
        actor: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let mut declarations = Vec::new();
        for r in responsables {
            let mut d = r.declaration()?;
            let droits = Self::droits_lien_tx(conn, etab, &r.lien, r.autorite_parentale).await?;
            d.droits = dossier_personnes::DroitsDeclares {
                notes: droits.notes,
                vs: droits.vs,
                justifier: droits.justifier,
                sorties: droits.sorties,
            };
            declarations.push(d);
        }
        let resultat = dossier_personnes::integrer_declaration_tx(
            conn,
            etab,
            eleve_id,
            &declarations,
            &plan.resolutions,
            dossier_personnes::SourceLien::Inscription,
            actor,
        )
        .await?;
        for r in &resultat.responsables {
            sqlx::query(
                "UPDATE ent_inscription_responsable SET personne_id = $3 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(r.cle)
            .bind(r.personne_id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        Ok(resultat.personnes_payeuses())
    }

    // -- Préinscription : vérification (anti-énumération) --------------------

    /// INS-02, étape 1 — public. Même traitement et même réponse que le compte
    /// existe ou non : une demande est créée et un code envoyé dans tous les cas.
    pub async fn demarrer_preinscription(
        &self,
        etab: Uuid,
        email: &str,
    ) -> Result<DemarrerDto, EntDbError> {
        let email = normaliser_email(email)?;
        let id = Uuid::new_v4();
        let code = code_six_chiffres();
        sqlx::query(
            "INSERT INTO ent_preinscription_verification (id, etablissement_id, email, code_hash, expire_le)
             VALUES ($1,$2,$3,$4, now() + make_interval(mins => $5::int))",
        )
        .bind(id)
        .bind(etab)
        .bind(&email)
        .bind(empreinte_code(id, &code)?)
        .bind(i32::try_from(CODE_TTL_MINUTES).unwrap_or(15))
        .execute(self.pool())
        .await
        .map_err(db)?;
        let envoi = self
            .mailer()
            .send(OutgoingMail {
                to: email,
                subject: "Votre code de vérification (préinscription)".into(),
                body_text: format!(
                    "Bonjour,\n\nVotre code de vérification pour la préinscription est : {code}\n\
Il est valable {CODE_TTL_MINUTES} minutes.\n\n\
Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.\n"
                ),
                attachments: vec![],
            })
            .await;
        if !matches!(envoi, Ok(ref r) if self.envoi_reussi(r)) {
            tracing::warn!("préinscription : code non envoyé (messagerie)");
        }
        Ok(DemarrerDto {
            demande_id: id,
            message: MESSAGE_DEMARRER.into(),
        })
    }

    fn nouveau_jeton() -> (String, String, DateTime<Utc>) {
        let (raw, hashed) = generate_reset_token();
        (
            raw,
            hashed,
            Utc::now() + Duration::minutes(JETON_TTL_MINUTES),
        )
    }

    /// INS-02, étape 2 — code reçu par e-mail. Erreur unique (« code invalide ou
    /// expiré ») quelle que soit la cause ; cinq essais au plus.
    pub async fn verifier_preinscription(
        &self,
        etab: Uuid,
        demande_id: Uuid,
        code: &str,
    ) -> Result<JetonPreinscriptionDto, EntDbError> {
        let generique = || invalide("code invalide ou expiré");
        let code = code.trim();
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(generique());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(String, i16, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT code_hash, tentatives, expire_le, verifie_le FROM ent_preinscription_verification
              WHERE etablissement_id = $1 AND id = $2 AND methode = 'code' FOR UPDATE",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((h, tentatives, expire, verifie)) = row else {
            return Err(generique());
        };
        if verifie.is_some() || expire < Utc::now() || tentatives >= CODE_ESSAIS_MAX {
            return Err(generique());
        }
        if !ent_auth::ct_eq(&h, &empreinte_code(demande_id, code)?) {
            sqlx::query("UPDATE ent_preinscription_verification SET tentatives = tentatives + 1 WHERE id = $1 AND etablissement_id = $2")
                .bind(demande_id)
                .bind(etab)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            commit(tx).await?;
            return Err(generique());
        }
        let (raw, hashed, fin) = Self::nouveau_jeton();
        sqlx::query(
            "UPDATE ent_preinscription_verification
                SET verifie_le = now(), jeton_hash = $2, jeton_expire_le = $3 WHERE id = $1 AND etablissement_id = $4",
        )
        .bind(demande_id)
        .bind(hashed)
        .bind(fin)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        commit(tx).await?;
        Ok(JetonPreinscriptionDto {
            jeton: raw,
            expire_le: fin,
        })
    }

    /// Variante « passkey / session » : une session ouverte dans cet
    /// établissement (passkey ou mot de passe + passkey) vaut vérification.
    pub async fn jeton_preinscription_depuis_session(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<JetonPreinscriptionDto, EntDbError> {
        // I0 pt 12 : seuls les responsables (profil parent) et les élèves majeurs reçoivent
        // un jeton famille — pas un professeur, un agent ni un élève mineur.
        match Role::parse(role) {
            Some(Role::Parent) => {}
            Some(Role::Eleve) => {
                let (majeur,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (
                        SELECT 1 FROM ent_eleve_compte c
                          JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id
                         WHERE c.etablissement_id = $1 AND c.user_id = $2
                           AND e.date_naissance IS NOT NULL
                           AND e.date_naissance <= (now() AT TIME ZONE 'UTC')::date - interval '18 years')",
                )
                .bind(etab)
                .bind(user_id)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                if !majeur {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ => return Err(EntDbError::Forbidden),
        }
        let (email,): (String,) = sqlx::query_as("SELECT lower(email) FROM ent_user WHERE id = $1")
            .bind(user_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?
            .ok_or(EntDbError::Forbidden)?;
        let (raw, hashed, fin) = Self::nouveau_jeton();
        sqlx::query(
            "INSERT INTO ent_preinscription_verification
                (id, etablissement_id, email, code_hash, expire_le, verifie_le, methode, jeton_hash, jeton_expire_le)
             VALUES ($1,$2,$3,'-',now(),now(),'session',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(&email)
        .bind(hashed)
        .bind(fin)
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(JetonPreinscriptionDto {
            jeton: raw,
            expire_le: fin,
        })
    }

    /// Jeton → e-mail vérifié (et compte s'il existe), dans CET établissement.
    pub async fn resoudre_jeton_preinscription(
        &self,
        etab: Uuid,
        jeton: &str,
    ) -> Result<Verifie, EntDbError> {
        if jeton.trim().is_empty() || jeton.len() > 256 {
            return Err(jeton_invalide());
        }
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT email FROM ent_preinscription_verification
              WHERE jeton_hash = $1 AND etablissement_id = $2 AND verifie_le IS NOT NULL
                AND jeton_expire_le > now()",
        )
        .bind(hash_token(jeton.trim()))
        .bind(etab)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let Some((email,)) = row else {
            return Err(jeton_invalide());
        };
        let user: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_user WHERE lower(email) = $1")
                .bind(&email)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
        Ok(Verifie {
            email,
            user_id: user.map(|u| u.0),
        })
    }

    /// Établissement auquel un jeton de préinscription est lié (adresse sans
    /// établissement connu, mode souple) ; `None` si le jeton est inconnu.
    pub async fn etablissement_du_jeton_preinscription(
        &self,
        jeton: &str,
    ) -> Result<Option<Uuid>, EntDbError> {
        if jeton.trim().is_empty() || jeton.len() > 256 {
            return Ok(None);
        }
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT etablissement_id FROM ent_preinscription_verification WHERE jeton_hash = $1",
        )
        .bind(hash_token(jeton.trim()))
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(r.map(|r| r.0))
    }

    /// Établissement d'une demande de vérification (mode souple ; le code reste exigé).
    pub async fn etablissement_de_demande_preinscription(
        &self,
        demande_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT etablissement_id FROM ent_preinscription_verification WHERE id = $1",
        )
        .bind(demande_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(r.map(|r| r.0))
    }

    /// Établissement d'une invitation (même usage que ci-dessus).
    pub async fn etablissement_de_invitation(
        &self,
        jeton: &str,
    ) -> Result<Option<Uuid>, EntDbError> {
        if jeton.trim().is_empty() || jeton.len() > 256 {
            return Ok(None);
        }
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT etablissement_id FROM ent_inscription_invitation WHERE jeton_hash = $1",
        )
        .bind(hash_token(jeton.trim()))
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(r.map(|r| r.0))
    }

    async fn campagnes_ouvertes(
        &self,
        etab: Uuid,
        nature: &str,
    ) -> Result<Vec<CampagneDto>, EntDbError> {
        let rows = sqlx::query_as::<_, CampagneRow>(&format!(
            "SELECT {COLS_CAMPAGNE} FROM ent_inscription_campagne
              WHERE etablissement_id = $1 AND nature = $2 AND statut = 'ouverte'
                AND (date_ouverture IS NULL OR date_ouverture <= CURRENT_DATE)
                AND (date_cloture IS NULL OR date_cloture >= CURRENT_DATE)
              ORDER BY annee_cible, libelle"
        ))
        .bind(etab)
        .bind(nature)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(self.campagne_dto(etab, r, false).await?);
        }
        Ok(out)
    }

    /// Espace famille (après vérification) : « Nous avons déjà vos informations ? »,
    /// campagnes ouvertes, dossiers déposés avec cette adresse dans CET établissement.
    pub async fn espace_preinscription(
        &self,
        etab: Uuid,
        jeton: &str,
    ) -> Result<EspacePreinscriptionDto, EntDbError> {
        let v = self.resoudre_jeton_preinscription(etab, jeton).await?;
        let profil = match v.user_id {
            None => None,
            Some(u) => {
                let p: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> =
                    sqlx::query_as(
                        "SELECT nom, prenom, civilite, telephone, adresse FROM ent_profil_personnel WHERE user_id = $1",
                    )
                    .bind(u)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db)?;
                match p {
                    Some((nom, prenom, civilite, telephone, adresse)) => Some(ProfilDto {
                        nom,
                        prenom,
                        civilite,
                        telephone,
                        adresse,
                    }),
                    None => {
                        let (nom,): (Option<String>,) =
                            sqlx::query_as("SELECT display_name FROM ent_user WHERE id = $1")
                                .bind(u)
                                .fetch_one(self.pool())
                                .await
                                .map_err(db)?;
                        Some(ProfilDto {
                            nom,
                            ..Default::default()
                        })
                    }
                }
            }
        };
        Ok(EspacePreinscriptionDto {
            email: v.email.clone(),
            connu: v.user_id.is_some(),
            profil,
            campagnes: self.campagnes_ouvertes(etab, "admission").await?,
            dossiers: self
                .resumes(
                    etab,
                    "AND d.deposant_email = $3 AND ($2::uuid IS NULL)",
                    (None, Some(v.email)),
                )
                .await?,
            message_confidentialite: MESSAGE_CONFIDENTIALITE.into(),
        })
    }

    /// INS-02 : dépôt d'une préinscription. Le déposant est l'adresse vérifiée.
    pub async fn deposer_preinscription(
        &self,
        etab: Uuid,
        jeton: &str,
        b: &PreinscriptionBody,
    ) -> Result<DossierDto, EntDbError> {
        let v = self.resoudre_jeton_preinscription(etab, jeton).await?;
        let eleve = normaliser_eleve(&b.eleve)?;
        let mut r1 = b.responsable.clone();
        r1.email = Some(v.email.clone());
        r1.meme_foyer = true;
        let r1 = normaliser_responsable(&r1)?;
        let r2 = match &b.second_parent {
            None => None,
            Some(s) => {
                let s = normaliser_responsable(s)?;
                if s.email.as_deref() == Some(v.email.as_str()) {
                    return Err(invalide(
                        "le second parent doit avoir sa propre adresse e-mail",
                    ));
                }
                Some(s)
            }
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let c: Option<(String, String, Option<NaiveDate>, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT nature, statut, date_ouverture, date_cloture FROM ent_inscription_campagne
              WHERE etablissement_id = $1 AND id = $2 FOR SHARE",
        )
        .bind(etab)
        .bind(b.campagne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((nature, statut, ouv, clo)) = c else {
            return Err(introuvable());
        };
        let today = Utc::now().date_naive();
        if nature != "admission"
            || statut != "ouverte"
            || ouv.is_some_and(|o| o > today)
            || clo.is_some_and(|c| c < today)
        {
            return Err(conflit("campagne d'admission non ouverte"));
        }
        let niveaux: Vec<(String,)> = sqlx::query_as(
            "SELECT code FROM ent_inscription_campagne_niveau WHERE etablissement_id = $1 AND campagne_id = $2",
        )
        .bind(etab)
        .bind(b.campagne_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !niveaux.is_empty()
            && !eleve
                .niveau_demande
                .as_ref()
                .is_some_and(|n| niveaux.iter().any(|(c,)| c == n))
        {
            return Err(invalide("niveau demandé non proposé par cette campagne"));
        }
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_dossier
              WHERE etablissement_id = $1 AND campagne_id = $2 AND deposant_email = $3",
        )
        .bind(etab)
        .bind(b.campagne_id)
        .bind(&v.email)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja >= DOSSIERS_MAX_PAR_DEPOSANT {
            return Err(conflit(
                "nombre maximal de dossiers atteint pour cette campagne",
            ));
        }
        let id = Uuid::new_v4();
        let reutilise = b.utiliser_mes_infos && v.user_id.is_some();
        sqlx::query(
            "INSERT INTO ent_inscription_dossier
                (id, etablissement_id, campagne_id, nature, statut, eleve_nom, eleve_prenom,
                 eleve_date_naissance, eleve_sexe, eleve_ine, niveau_demande, regime_demande,
                 etablissement_origine, deposant_email, deposant_user_id, reutilise_identite, depose_le)
             VALUES ($1,$2,$3,'admission','deposee',$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14, now())",
        )
        .bind(id)
        .bind(etab)
        .bind(b.campagne_id)
        .bind(&eleve.nom)
        .bind(&eleve.prenom)
        .bind(eleve.date_naissance)
        .bind(&eleve.sexe)
        .bind(&eleve.ine)
        .bind(&eleve.niveau_demande)
        .bind(&eleve.regime_demande)
        .bind(&eleve.etablissement_origine)
        .bind(&v.email)
        .bind(v.user_id)
        .bind(reutilise)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (rang, r) in [(1_i16, Some(&r1)), (2, r2.as_ref())] {
            let Some(r) = r else { continue };
            let resp_id = Uuid::new_v4();
            let reserve = r.reserve.as_ref().map(reserve_depuis_dto).unwrap_or_default();
            let coffre = ChampsSensibles::depuis_env();
            // Adresse à protéger : jamais en clair dans le dossier (colonne `adresse` vide).
            let (adresse, adresse_protegee) = if reserve.adresse_a_proteger {
                (
                    None,
                    coffre
                        .sceller_opt(
                            "ent_inscription_responsable",
                            "adresse_protegee",
                            resp_id,
                            r.adresse.as_deref(),
                        )
                        .map_err(EntDbError::Domain)?,
                )
            } else {
                (r.adresse.clone(), None)
            };
            let reference = coffre
                .sceller_opt(
                    "ent_inscription_responsable",
                    "ne_pas_solliciter_ref",
                    resp_id,
                    reserve.ne_pas_solliciter_ref.as_deref(),
                )
                .map_err(EntDbError::Domain)?;
            sqlx::query(
                "INSERT INTO ent_inscription_responsable
                    (id, etablissement_id, dossier_id, rang, nom, prenom, civilite, email, telephone,
                     adresse, lien, meme_foyer, autorite_parentale, ne_pas_solliciter,
                     ne_pas_solliciter_ref, adresse_a_proteger, adresse_protegee, langue,
                     besoin_interprete, organisme, nature_code)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)",
            )
            .bind(resp_id)
            .bind(etab)
            .bind(id)
            .bind(rang)
            .bind(&r.nom)
            .bind(&r.prenom)
            .bind(&r.civilite)
            .bind(&r.email)
            .bind(&r.telephone)
            .bind(adresse)
            .bind(r.lien.as_deref().unwrap_or("autre"))
            .bind(r.meme_foyer)
            .bind(r.autorite_parentale)
            .bind(reserve.ne_pas_solliciter)
            .bind(reference)
            .bind(reserve.adresse_a_proteger)
            .bind(adresse_protegee)
            .bind(&reserve.langue)
            .bind(reserve.besoin_interprete)
            .bind(&reserve.organisme)
            .bind(&reserve.nature_code)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        Self::copier_pieces_tx(&mut tx, etab, b.campagne_id, id).await?;
        suivi_tx(
            &mut tx,
            etab,
            id,
            "statut",
            Some(("brouillon", "deposee")),
            Some("Préinscription déposée"),
            true,
            None,
        )
        .await?;
        if let Some(u) = v.user_id {
            if reutilise {
                Self::journal_identite_tx(&mut tx, u, "informations_reutilisees", etab).await?;
            }
            if b.enregistrer_profil {
                sqlx::query(
                    "INSERT INTO ent_profil_personnel (user_id, nom, prenom, civilite, telephone, adresse, updated_at)
                     VALUES ($1,$2,$3,$4,$5,$6, now())
                     ON CONFLICT (user_id) DO UPDATE SET nom = EXCLUDED.nom, prenom = EXCLUDED.prenom,
                        civilite = EXCLUDED.civilite, telephone = EXCLUDED.telephone,
                        adresse = EXCLUDED.adresse, updated_at = now()",
                )
                .bind(u)
                .bind(&r1.nom)
                .bind(&r1.prenom)
                .bind(&r1.civilite)
                .bind(&r1.telephone)
                .bind(&r1.adresse)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                Self::journal_identite_tx(&mut tx, u, "profil_enregistre", etab).await?;
            }
        }
        publier(
            &mut tx,
            etab,
            types::ADMISSION_DEPOSEE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "dossier_id": id, "campagne_id": b.campagne_id, "second_parent": r2.is_some() }),
            v.user_id,
        )
        .await?;
        commit(tx).await?;
        self.dossier_famille(etab, jeton, id).await
    }

    async fn journal_identite_tx(
        conn: &mut PgConnection,
        user_id: Uuid,
        action: &str,
        etab: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "INSERT INTO ent_identite_journal (id, user_id, action, etablissement_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(action)
        .bind(etab)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Dossier déposé avec l'adresse vérifiée (sinon « introuvable »).
    async fn dossier_du_deposant(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<(Verifie, DossierRow), EntDbError> {
        let v = self.resoudre_jeton_preinscription(etab, jeton).await?;
        let d = self.dossier_row(etab, id).await?;
        if d.deposant_email.as_deref() != Some(v.email.as_str()) {
            return Err(introuvable());
        }
        Ok((v, d))
    }

    pub async fn dossier_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<DossierDto, EntDbError> {
        let (_, d) = self.dossier_du_deposant(etab, jeton, id).await?;
        let mut actions = Vec::new();
        if matches!(
            d.statut.as_str(),
            "deposee" | "en_instruction" | "incomplet" | "liste_attente" | "admis"
        ) {
            actions.push("deposer_piece".to_string());
        }
        if d.statut == "validee" {
            actions.push("coordonnees_bancaires".to_string());
        }
        self.dossier_dto(etab, d, true, actions).await
    }

    /// La famille déclare une pièce déposée (nom du fichier ; le stockage du
    /// fichier passera par le service documentaire, voir limites).
    pub async fn deposer_piece_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
        code: &str,
        nom_fichier: &str,
    ) -> Result<DossierDto, EntDbError> {
        let (_, d) = self.dossier_du_deposant(etab, jeton, id).await?;
        let nom = nom_fichier.trim();
        if nom.is_empty() || nom.chars().count() > 200 || nom.contains(['/', '\\']) {
            return Err(invalide("nom de fichier invalide"));
        }
        if !matches!(
            d.statut.as_str(),
            "deposee" | "en_instruction" | "incomplet" | "liste_attente" | "admis"
        ) {
            return Err(conflit("dossier clos"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_inscription_piece SET statut = 'deposee', nom_fichier = $4, motif_refus = NULL,
                    maj_par = NULL, maj_le = now()
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3
                AND statut IN ('attendue', 'refusee', 'deposee')",
        )
        .bind(etab)
        .bind(id)
        .bind(code)
        .bind(nom)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if n.rows_affected() == 0 {
            return Err(introuvable());
        }
        suivi_tx(
            &mut tx,
            etab,
            id,
            "piece",
            None,
            Some(&format!("Pièce « {code} » déposée")),
            true,
            None,
        )
        .await?;
        commit(tx).await?;
        self.dossier_famille(etab, jeton, id).await
    }

    // -- Coordonnées bancaires (D-014, INS-14) --------------------------------

    /// IBAN connus de la personne dans d'AUTRES établissements (mandats actifs
    /// des foyers où elle est payeuse), déchiffrés pour être masqués.
    async fn ibans_reprenables(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, String, Option<String>, String)>, EntDbError> {
        let rows: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
            "SELECT DISTINCT m.id, m.iban, m.bic, m.titulaire
               FROM ent_eleve_lien l
               JOIN ent_fac_sepa_mandat m
                 ON m.foyer_id = l.foyer_id AND m.etablissement_id = l.etablissement_id
              WHERE l.user_id = $1 AND l.est_payeur AND l.etablissement_id <> $2 AND m.statut = 'actif'",
        )
        .bind(user_id)
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let champs = ChampsSensibles::depuis_env();
        let mut out: Vec<(Uuid, String, Option<String>, String)> = Vec::new();
        for (id, iban, bic, tit) in rows {
            let iban = champs.ouvrir(T_MANDAT, "iban", id, &iban)?;
            let bic = champs.ouvrir_opt(T_MANDAT, "bic", id, bic)?;
            let tit = champs.ouvrir(T_MANDAT, "titulaire", id, &tit)?;
            if !out.iter().any(|o| o.1 == iban) {
                out.push((id, iban, bic, tit));
            }
        }
        Ok(out)
    }

    async fn demande_mandat_dto(
        &self,
        etab: Uuid,
        dossier_id: Uuid,
    ) -> Result<Option<MandatDemandeDto>, EntDbError> {
        let r: Option<(Uuid, String, String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, iban, source, statut, mandat_id FROM ent_mandat_sepa_demande
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut IN ('a_signer', 'signe')",
        )
        .bind(etab)
        .bind(dossier_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let Some((id, iban, source, statut, mandat_id)) = r else {
            return Ok(None);
        };
        let iban = ChampsSensibles::depuis_env().ouvrir(T_DEMANDE, "iban", id, &iban)?;
        let rum = match mandat_id {
            Some(m) => sqlx::query_as::<_, (String,)>(
                "SELECT rum FROM ent_fac_sepa_mandat WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(m)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?
            .map(|r| r.0),
            None => None,
        };
        Ok(Some(MandatDemandeDto {
            id,
            iban_masque: masquer_iban(&iban),
            source,
            statut,
            rum,
        }))
    }

    async fn dossier_valide_du_deposant(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<(Verifie, DossierRow), EntDbError> {
        let (v, d) = self.dossier_du_deposant(etab, jeton, id).await?;
        if d.statut != "validee" {
            return Err(conflit(
                "coordonnées bancaires demandées à la validation de l'inscription",
            ));
        }
        Ok((v, d))
    }

    pub async fn coordonnees_bancaires_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<CoordonneesBancairesDto, EntDbError> {
        let (v, d) = self.dossier_valide_du_deposant(etab, jeton, id).await?;
        let propositions = match (d.reutilise_identite, v.user_id) {
            (true, Some(u)) => self
                .ibans_reprenables(etab, u)
                .await?
                .into_iter()
                .map(|(pid, iban, _, tit)| IbanProposeDto {
                    proposition_id: pid,
                    iban_masque: masquer_iban(&iban),
                    titulaire_initiales: initiales(&tit),
                })
                .collect(),
            _ => Vec::new(),
        };
        Ok(CoordonneesBancairesDto {
            propositions,
            mandat: self.demande_mandat_dto(etab, id).await?,
        })
    }

    /// Reprendre un IBAN proposé (masqué) ou en saisir un : crée un mandat **à signer**.
    pub async fn demander_mandat_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
        b: &MandatBody,
    ) -> Result<MandatDemandeDto, EntDbError> {
        let (v, d) = self.dossier_valide_du_deposant(etab, jeton, id).await?;
        let (iban, bic, titulaire, source) = match b.proposition_id {
            Some(pid) => {
                let (true, Some(u)) = (d.reutilise_identite, v.user_id) else {
                    return Err(introuvable());
                };
                let p = self
                    .ibans_reprenables(etab, u)
                    .await?
                    .into_iter()
                    .find(|p| p.0 == pid)
                    .ok_or_else(introuvable)?;
                (p.1, p.2, p.3, "reprise")
            }
            None => {
                let iban = normalize_iban(b.iban.as_deref().unwrap_or_default())
                    .map_err(|_| invalide("IBAN invalide"))?;
                let tit = opt(&b.titulaire).ok_or(invalide("titulaire du compte obligatoire"))?;
                if tit.chars().count() > 70 {
                    return Err(invalide("titulaire trop long (70 caractères)"));
                }
                (iban, opt(&b.bic), tit, "saisie")
            }
        };
        // Foyer payeur : celui du lien du déposant (rang 1) avec l'élève.
        let foyer: Option<(Uuid,)> = sqlx::query_as(
            "SELECT l.foyer_id FROM ent_inscription_responsable r
               JOIN ent_eleve_lien l ON l.personne_id = r.personne_id AND l.etablissement_id = r.etablissement_id
              WHERE r.etablissement_id = $1 AND r.dossier_id = $2 AND r.rang = 1
                AND l.eleve_id = $3 AND l.foyer_id IS NOT NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(d.eleve_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let foyer = foyer
            .ok_or(conflit("aucun foyer payeur pour ce dossier"))?
            .0;
        let demande_id = Uuid::new_v4();
        let champs = ChampsSensibles::depuis_env();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_mandat_sepa_demande SET statut = 'annule'
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'a_signer'",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ins = sqlx::query(
            "INSERT INTO ent_mandat_sepa_demande
                (id, etablissement_id, dossier_id, foyer_id, iban, bic, titulaire, source, demande_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(demande_id)
        .bind(etab)
        .bind(id)
        .bind(foyer)
        .bind(champs.sceller(T_DEMANDE, "iban", demande_id, &iban)?)
        .bind(champs.sceller_opt(T_DEMANDE, "bic", demande_id, bic.as_deref())?)
        .bind(champs.sceller(T_DEMANDE, "titulaire", demande_id, &titulaire)?)
        .bind(source)
        .bind(v.user_id)
        .execute(&mut *tx)
        .await;
        if let Err(e) = ins {
            return Err(match map_sqlx("inscriptions", e) {
                DomainError::Conflit => conflit("mandat déjà signé pour ce dossier"),
                d => EntDbError::Domain(d),
            });
        }
        if source == "reprise" {
            if let Some(u) = v.user_id {
                Self::journal_identite_tx(&mut tx, u, "iban_repris", etab).await?;
            }
        }
        publier(
            &mut tx,
            etab,
            types::MANDAT_SEPA_A_SIGNER,
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "demande_id": demande_id, "source": source }),
            v.user_id,
        )
        .await?;
        commit(tx).await?;
        self.demande_mandat_dto(etab, id)
            .await?
            .ok_or_else(introuvable)
    }

    /// Signature du nouveau mandat (consentement explicite horodaté) : le mandat
    /// actif est créé dans l'établissement (domaine facturation, chiffré D3).
    pub async fn signer_mandat_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<MandatDemandeDto, EntDbError> {
        let (v, _d) = self.dossier_valide_du_deposant(etab, jeton, id).await?;
        let champs = ChampsSensibles::depuis_env();
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r: Option<(Uuid, Uuid, String, Option<String>, String)> = sqlx::query_as(
            "SELECT id, foyer_id, iban, bic, titulaire FROM ent_mandat_sepa_demande
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'a_signer' FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (demande_id, foyer, iban, bic, tit) = r.ok_or(conflit("aucun mandat à signer"))?;
        let iban = champs.ouvrir(T_DEMANDE, "iban", demande_id, &iban)?;
        let bic = champs.ouvrir_opt(T_DEMANDE, "bic", demande_id, bic)?;
        let tit = champs.ouvrir(T_DEMANDE, "titulaire", demande_id, &tit)?;
        let (actif,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_fac_sepa_mandat WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'actif'",
        )
        .bind(etab)
        .bind(foyer)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if actif > 0 {
            return Err(conflit("un mandat actif existe déjà pour ce foyer"));
        }
        let rum = format!(
            "INS-{}",
            demande_id.simple().to_string()[..16].to_uppercase()
        );
        let m = plan_creer_mandat_sepa(CreerMandatSepa {
            etablissement_id: etab,
            foyer_id: foyer,
            rum,
            iban,
            bic,
            titulaire: tit,
            signe_at: Utc::now(),
            actor_user_id: v.user_id.unwrap_or_default(),
        })
        .map_err(|_| invalide("coordonnées bancaires invalides"))?;
        sqlx::query(
            "INSERT INTO ent_fac_sepa_mandat (id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                                              statut, signe_at, created_by_user_id, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'actif',$8,$9, now())",
        )
        .bind(m.id)
        .bind(etab)
        .bind(foyer)
        .bind(&m.rum)
        .bind(champs.sceller(T_MANDAT, "iban", m.id, &m.iban)?)
        .bind(champs.sceller_opt(T_MANDAT, "bic", m.id, m.bic.as_deref())?)
        .bind(champs.sceller(T_MANDAT, "titulaire", m.id, &m.titulaire)?)
        .bind(m.signe_at)
        .bind(v.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let h = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_foyer_iban_histo (id, etablissement_id, foyer_id, iban, bic, titulaire, mandat_id,
                                               created_by_user_id, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())",
        )
        .bind(h)
        .bind(etab)
        .bind(foyer)
        .bind(champs.sceller("ent_foyer_iban_histo", "iban", h, &m.iban)?)
        .bind(champs.sceller_opt("ent_foyer_iban_histo", "bic", h, m.bic.as_deref())?)
        .bind(champs.sceller("ent_foyer_iban_histo", "titulaire", h, &m.titulaire)?)
        .bind(m.id)
        .bind(v.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_mandat_sepa_demande SET statut = 'signe', mandat_id = $2, signe_le = now() WHERE id = $1 AND etablissement_id = $3",
        )
        .bind(demande_id)
        .bind(m.id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::FAC_SEPA_MANDAT,
            AggregateKind::Facture,
            m.id,
            None,
            json!({ "foyer_id": foyer, "mandat_id": m.id, "source": "inscription" }),
            v.user_id,
        )
        .await?;
        commit(tx).await?;
        self.demande_mandat_dto(etab, id)
            .await?
            .ok_or_else(introuvable)
    }

    // -- Invitations (INS-13, D-014 second parent) ---------------------------

    /// Accepter une invitation (lien reçu par e-mail = adresse confirmée), **en une seule
    /// transaction** : compte (créé ou existant), appartenance, rattachement de la fiche
    /// et activation des liens en attente. Une fiche déjà rattachée à un compte n'est
    /// jamais réassignée : seul son titulaire peut confirmer.
    pub async fn accepter_invitation(
        &self,
        etab: Uuid,
        jeton: &str,
    ) -> Result<InvitationAccepteeDto, EntDbError> {
        if jeton.trim().is_empty() || jeton.len() > 256 {
            return Err(invalide("invitation invalide ou expirée"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let inv: Option<(Uuid, Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT i.id, i.personne_id, i.responsable_id, i.email FROM ent_inscription_invitation i
              WHERE i.jeton_hash = $1 AND i.etablissement_id = $2
                AND i.statut IN ('a_envoyer', 'envoyee', 'non_envoyee')
                AND i.expire_le > now()
              FOR UPDATE",
        )
        .bind(hash_token(jeton.trim()))
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (inv_id, personne_id, responsable_id, email) =
            inv.ok_or(invalide("invitation invalide ou expirée"))?;
        let (nom, prenom, user_fiche, email_fiche): (String, Option<String>, Option<Uuid>, Option<String>) = sqlx::query_as(
            "SELECT nom, prenom, user_id, email FROM ent_personne WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // Une fiche déjà connue de l'établissement (rapprochée sur choix humain) ne peut être
        // rattachée qu'à un compte de SON adresse : une adresse saisie par un tiers dans un
        // dossier ne suffit jamais à s'approprier une fiche existante.
        if user_fiche.is_none()
            && !email_fiche
                .as_deref()
                .is_some_and(|e| e.trim().eq_ignore_ascii_case(email.trim()))
        {
            return Err(conflit(
                "l'adresse de l'invitation ne correspond pas à la fiche : contactez l'établissement",
            ));
        }
        let existant: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_user WHERE lower(email) = lower($1)")
                .bind(&email)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let (user_id, cree) = match (user_fiche, existant) {
            // Confirmation par le titulaire du compte déjà rattaché à la fiche.
            (Some(pu), Some((u,))) if pu == u => (u, false),
            (Some(_), _) => {
                return Err(conflit(
                    "cette fiche est déjà rattachée à un autre compte : contactez l'établissement",
                ))
            }
            (None, Some((u,))) => (u, false),
            (None, None) => {
                let mdp = format!("{}Aa9!", ent_auth::generate_temp_password());
                let libelle = match &prenom {
                    Some(p) => format!("{p} {nom}"),
                    None => nom.clone(),
                };
                let hash = ent_auth::hash_password(&mdp).map_err(ent_auth::AuthError::from)?;
                let u = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
                     VALUES ($1,$2,$3,$4,now(),now())",
                )
                .bind(u)
                .bind(&email)
                .bind(hash)
                .bind(libelle)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (u, true)
            }
        };
        // Rattachement à l'établissement (confirmé par la personne).
        sqlx::query(
            "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
             VALUES ($1,$2,'parent',now()) ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if user_fiche.is_none() {
            // Commande du socle 4 : garde « personne sans compte » et « compte déjà rattaché ».
            dossier_personnes::rattacher_compte_tx(&mut tx, etab, personne_id, user_id).await?;
        }
        // Liens en attente de CETTE invitation : actifs seulement maintenant, par la commande
        // de confirmation du socle 4.
        dossier_personnes::confirmer_liens_attente_tx(
            &mut tx,
            etab,
            personne_id,
            &dossier_personnes::FiltreAttente {
                source: Some(dossier_personnes::SourceLien::Inscription),
                source_ref: Some(responsable_id),
                ..Default::default()
            },
            user_id,
        )
        .await?;
        sqlx::query(
            "UPDATE ent_inscription_invitation SET statut = 'acceptee', accepte_le = now(), user_id = $2 WHERE id = $1 AND etablissement_id = $3",
        )
        .bind(inv_id)
        .bind(user_id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::journal_identite_tx(
            &mut tx,
            user_id,
            if cree {
                "compte_cree"
            } else {
                "compte_rattache"
            },
            etab,
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::PERSONNE_COMPTE_RATTACHE,
            AggregateKind::Personne,
            personne_id,
            None,
            json!({ "personne_id": personne_id, "source": "invitation" }),
            Some(user_id),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INVITATION_ACCEPTEE,
            AggregateKind::Personne,
            personne_id,
            None,
            json!({ "invitation_id": inv_id }),
            Some(user_id),
        )
        .await?;
        commit(tx).await?;
        if cree {
            let base = std::env::var("ENT_PUBLIC_BASE_URL").unwrap_or_default();
            if let Err(e) = self
                .envoyer_lien_reinitialisation(
                    user_id,
                    &base,
                    60,
                    "Activez votre compte ENT",
                    "Votre compte a été créé. Pour choisir votre mot de passe puis enregistrer une passkey, ouvrez ce lien (valable 60 minutes)",
                )
                .await
            {
                tracing::warn!("invitation acceptée : lien d'activation non envoyé ({e})");
            }
        }
        Ok(InvitationAccepteeDto {
            resultat: if cree {
                "compte_cree"
            } else {
                "compte_rattache"
            }
            .into(),
            message: if cree {
                "Votre compte est créé. Un lien pour choisir votre mot de passe vous a été envoyé."
                    .into()
            } else {
                "Votre compte existant est désormais rattaché à cet établissement.".into()
            },
        })
    }

    /// Droits par type de lien (paramétrage de l'établissement ; valeurs par défaut sinon).
    pub async fn parametres_liens_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ParametreLienDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let mut conn = self.pool().acquire().await.map_err(db)?;
        let mut out = Vec::new();
        for lien in ["mere", "pere", "tuteur", "autre"] {
            let d = Self::droits_lien_tx(&mut conn, etab, lien, lien != "autre").await?;
            out.push(ParametreLienDto {
                lien: lien.into(),
                peut_voir_notes: d.notes,
                peut_voir_vs: d.vs,
                peut_justifier_absence: d.justifier,
                peut_autoriser_sorties: d.sorties,
            });
        }
        Ok(out)
    }

    pub async fn definir_parametre_lien_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        p: &ParametreLienDto,
    ) -> Result<(), EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        if !matches!(p.lien.as_str(), "mere" | "pere" | "tuteur" | "autre") {
            return Err(invalide("type de lien inconnu"));
        }
        sqlx::query(
            "INSERT INTO ent_inscription_parametre_lien
                (etablissement_id, lien, peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                 peut_autoriser_sorties, modifie_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (etablissement_id, lien) DO UPDATE SET
                peut_voir_notes = $3, peut_voir_vs = $4, peut_justifier_absence = $5,
                peut_autoriser_sorties = $6, modifie_par = $7, updated_at = now()",
        )
        .bind(etab)
        .bind(&p.lien)
        .bind(p.peut_voir_notes)
        .bind(p.peut_voir_vs)
        .bind(p.peut_justifier_absence)
        .bind(p.peut_autoriser_sorties)
        .bind(actor)
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Refuser une invitation : le lien en attente est annulé, aucun accès n'est ouvert.
    pub async fn refuser_invitation(&self, etab: Uuid, jeton: &str) -> Result<(), EntDbError> {
        if jeton.trim().is_empty() || jeton.len() > 256 {
            return Err(invalide("invitation invalide ou expirée"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let inv: Option<(Uuid, Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, personne_id, responsable_id FROM ent_inscription_invitation
              WHERE jeton_hash = $1 AND etablissement_id = $2
                AND statut IN ('a_envoyer', 'envoyee', 'non_envoyee') AND expire_le > now()
              FOR UPDATE",
        )
        .bind(hash_token(jeton.trim()))
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (inv_id, personne_id, responsable_id) =
            inv.ok_or(invalide("invitation invalide ou expirée"))?;
        dossier_personnes::refuser_liens_attente_tx(
            &mut tx,
            etab,
            personne_id,
            &dossier_personnes::FiltreAttente {
                source: Some(dossier_personnes::SourceLien::Inscription),
                source_ref: Some(responsable_id),
                ..Default::default()
            },
            None,
        )
        .await?;
        // Lien déjà actif (personne sans compte au moment de la validation) : suspendu, pour
        // qu'il ne s'active jamais avec un compte ouvert plus tard sur cette fiche.
        let dossier: Option<(Uuid,)> = sqlx::query_as(
            "SELECT d.eleve_id FROM ent_inscription_invitation i
               JOIN ent_inscription_dossier d ON d.id = i.dossier_id AND d.etablissement_id = i.etablissement_id
              WHERE i.id = $1 AND i.etablissement_id = $2 AND d.eleve_id IS NOT NULL",
        )
        .bind(inv_id)
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((eleve_id,)) = dossier {
            dossier_personnes::suspendre_lien_personne_tx(
                &mut tx,
                etab,
                personne_id,
                eleve_id,
                "Refus du responsable désigné (invitation)",
                None,
            )
            .await?;
        }
        sqlx::query(
            "UPDATE ent_inscription_invitation SET statut = 'refusee' WHERE id = $1 AND etablissement_id = $2",
        )
        .bind(inv_id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        commit(tx).await?;
        Ok(())
    }

    // -- Réinscription par la famille (INS-07, INS-08) -------------------------

    async fn exiger_lien_famille(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        if Role::parse(role) != Some(Role::Parent) {
            return Err(EntDbError::Forbidden);
        }
        let Some(e) = eleve_id else {
            return Err(introuvable());
        };
        match self.lien_parent_eleve(etab, actor, e).await? {
            Some(_) => Ok(()),
            None => Err(introuvable()),
        }
    }

    /// Dossiers de réinscription des enfants du responsable (campagnes ouvertes).
    pub async fn reinscriptions_famille(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<DossierDto>, EntDbError> {
        if Role::parse(role) != Some(Role::Parent) {
            return Err(EntDbError::Forbidden);
        }
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT d.id FROM ent_inscription_dossier d
               JOIN ent_inscription_campagne c ON c.id = d.campagne_id AND c.etablissement_id = d.etablissement_id
               JOIN ent_eleve_lien_actif l ON l.eleve_id = d.eleve_id AND l.etablissement_id = d.etablissement_id
              WHERE d.etablissement_id = $1 AND d.nature = 'reinscription' AND c.statut = 'ouverte'
                AND l.user_id = $2 AND l.autorite_parentale
              ORDER BY d.eleve_nom, d.eleve_prenom",
        )
        .bind(etab)
        .bind(actor)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            let d = self.dossier_row(etab, id).await?;
            let mut actions = Vec::new();
            if matches!(d.statut.as_str(), "brouillon" | "deposee") {
                actions.push("confirmer".into());
                actions.push("annoncer_depart".into());
            }
            out.push(self.dossier_dto(etab, d, true, actions).await?);
        }
        Ok(out)
    }

    pub async fn confirmer_reinscription_famille(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ConfirmationReinscriptionBody,
    ) -> Result<DossierDto, EntDbError> {
        let regime = regime_valide(&b.regime)?;
        let d = self.dossier_row(etab, id).await?;
        if d.nature != "reinscription" {
            return Err(introuvable());
        }
        self.exiger_lien_famille(etab, actor, role, d.eleve_id)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        let (ouverte,): (bool,) = sqlx::query_as(
            "SELECT statut = 'ouverte' FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(d.campagne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !ouverte || !matches!(d.statut.as_str(), "brouillon" | "deposee") {
            return Err(conflit("réinscription non modifiable"));
        }
        let email: Option<(String,)> =
            sqlx::query_as("SELECT lower(email) FROM ent_user WHERE id = $1")
                .bind(actor)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        sqlx::query(
            "UPDATE ent_inscription_dossier SET regime_demande = COALESCE($3, regime_demande),
                    deposant_user_id = $4, deposant_email = $5, depose_le = now(), updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&regime)
        .bind(actor)
        .bind(email.map(|e| e.0))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if d.statut == "brouillon" {
            changer_statut_tx(
                &mut tx,
                etab,
                &d,
                "deposee",
                Some("Réinscription confirmée par la famille"),
                Some(actor),
            )
            .await?;
        }
        let annee: (String,) =
            sqlx::query_as("SELECT annee_cible FROM ent_inscription_campagne WHERE id = $1 AND etablissement_id = $2")
                .bind(d.campagne_id)
                .bind(etab)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::ELEVE_REINSCRIPTION_CONFIRMEE,
            AggregateKind::Eleve,
            d.eleve_id.unwrap_or_default(),
            d.eleve_id,
            json!({ "dossier_id": id, "annee_cible": annee.0 }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        let d = self.dossier_row(etab, id).await?;
        self.dossier_dto(
            etab,
            d,
            true,
            vec!["confirmer".into(), "annoncer_depart".into()],
        )
        .await
    }

    pub async fn annoncer_depart_famille(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &DepartBody,
    ) -> Result<DossierDto, EntDbError> {
        let motif = b.motif.trim();
        if motif.is_empty() || motif.chars().count() > 1000 {
            return Err(invalide(
                "motif du départ obligatoire (1000 caractères au plus)",
            ));
        }
        let d = self.dossier_row(etab, id).await?;
        if d.nature != "reinscription" {
            return Err(introuvable());
        }
        self.exiger_lien_famille(etab, actor, role, d.eleve_id)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        if !matches!(d.statut.as_str(), "brouillon" | "deposee") {
            return Err(conflit("réinscription non modifiable"));
        }
        changer_statut_tx(&mut tx, etab, &d, "desistement", Some(motif), Some(actor)).await?;
        // Motif hors de l'événement (texte libre, §4.1-3).
        publier(
            &mut tx,
            etab,
            types::SCOLARITE_DEPART_ANNONCE,
            AggregateKind::Eleve,
            d.eleve_id.unwrap_or_default(),
            d.eleve_id,
            json!({ "dossier_id": id, "date_depart": b.date_depart }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        let d = self.dossier_row(etab, id).await?;
        self.dossier_dto(etab, d, true, Vec::new()).await
    }
}

#[cfg(test)]
mod tests_unitaires {
    use super::*;

    #[test]
    fn masque_iban() {
        assert_eq!(
            masquer_iban("FR7630006000011234567890189"),
            "FR76 •••• 0189"
        );
        assert_eq!(
            masquer_iban("FR76 3000 6000 0112 3456 7890 189"),
            "FR76 •••• 0189"
        );
        assert_eq!(masquer_iban("court"), "••••");
    }

    #[test]
    fn annee_cible() {
        assert!(annee_cible_valide("2027-2028"));
        assert!(!annee_cible_valide("2027-2029"));
        assert!(!annee_cible_valide("27-28"));
        assert!(!annee_cible_valide("2027"));
    }

    #[test]
    fn transitions_et_actions() {
        assert!(transition_instruction_admise("deposee", "en_instruction"));
        assert!(!transition_instruction_admise("validee", "en_instruction"));
        assert!(!transition_instruction_admise("deposee", "validee"));
        let a = actions_personnel("admission", "admis", true, false, false);
        assert!(
            !a.contains(&"valider".to_string()),
            "valider exige inscriptions.valider"
        );
        let a = actions_personnel("admission", "admis", false, false, true);
        assert!(a.contains(&"valider".to_string()));
        let a = actions_personnel("reinscription", "deposee", true, true, true);
        assert!(a.contains(&"valider".to_string()) && !a.contains(&"decider".to_string()));
    }

    #[test]
    fn parents_separes_email_obligatoire() {
        let r = ResponsableFormulaireDto {
            nom: "Durand".into(),
            meme_foyer: false,
            autorite_parentale: true,
            ..Default::default()
        };
        assert!(normaliser_responsable(&r).is_err());
        let r = ResponsableFormulaireDto {
            email: Some("Autre@Exemple.fr".into()),
            ..r
        };
        let n = normaliser_responsable(&r).unwrap();
        assert_eq!(n.email.as_deref(), Some("autre@exemple.fr"));
        assert_eq!(n.lien.as_deref(), Some("autre"));
    }

    #[test]
    fn code_six_chiffres_format() {
        for _ in 0..50 {
            let c = code_six_chiffres();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|c| c.is_ascii_digit()));
        }
        let id = Uuid::new_v4();
        let sb = ent_auth::SecretBox::dev();
        let e = |i, c| empreinte_code_avec(&sb, i, c).unwrap();
        assert_eq!(e(id, "123456"), e(id, " 123456 "));
        assert_ne!(e(id, "123456"), e(Uuid::new_v4(), "123456"));
        // L'empreinte n'est plus un simple SHA-256 rejouable sans secret.
        assert_ne!(
            e(id, "123456"),
            hash_token(&format!("preinscription:{id}:123456"))
        );
    }

    #[test]
    fn initiales_titulaire() {
        assert_eq!(initiales("jean dupont"), "J. D.");
    }
}
