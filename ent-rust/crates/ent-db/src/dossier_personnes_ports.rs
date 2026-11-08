//! Ports du socle 4 appelés par les autres socles (lot A0, `docs/socles/04_ADMINISTRATIF.md` §5.3
//! et §16) : **personne n'écrit** dans `ent_personne`, `ent_foyer`, `ent_eleve_lien` en dehors de
//! ces commandes.
//!
//! * `correspondances_tx` : lecture minimisée des personnes de l'établissement qui pourraient être
//!   un responsable déclaré. **Jamais de rapprochement implicite** : un candidat n'est qu'une
//!   proposition soumise à un choix humain.
//! * `integrer_declaration_tx` : déclaration de responsables + choix humains → personnes, foyers,
//!   liens. Appelable dans la transaction de l'appelant (fait + journal + événement ensemble).
//! * Un lien vers une personne qui **a un compte** n'ouvre aucun accès : il est écrit dans
//!   `ent_lien_attente` (invitation) et ne devient un lien réel (`ent_eleve_lien`) que par la
//!   confirmation de cette personne (`confirmer_liens_attente_tx`).
//! * `ajouter_responsable_for_actor` : commande unique (personne + lien) qui remplace les deux
//!   appels de l'écran.
//!
//! Les dates métier sont celles du fuseau de l'établissement (`ent_aujourdhui_etab()`, migration
//! 265 ; Europe/Paris par défaut).

use super::*;

/// Origine d'un lien (colonne `source` de `ent_lien_attente`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLien {
    /// Validation d'un dossier d'inscription (socle 3) : `source_ref` = responsable déclaré.
    Inscription,
    /// Saisie directe par le personnel habilité.
    Saisie,
    /// Import d'une liste d'élèves (cadre d'import S2-3) : toujours de nouvelles fiches.
    Import,
}

impl SourceLien {
    pub fn code(self) -> &'static str {
        match self {
            SourceLien::Inscription => "inscription",
            SourceLien::Saisie => "saisie",
            SourceLien::Import => "import",
        }
    }
    fn source_personne(self) -> &'static str {
        match self {
            SourceLien::Inscription => "preinscription",
            SourceLien::Saisie => "saisie",
            SourceLien::Import => "reprise",
        }
    }
}

/// Droits fins d'un lien (colonnes `peut_*` héritées, jusqu'au lot A4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DroitsDeclares {
    pub notes: bool,
    pub vs: bool,
    pub justifier: bool,
    pub sorties: bool,
}

/// Un responsable tel que la famille l'a déclaré (copie du formulaire, D-014).
#[derive(Debug, Clone)]
pub struct DeclarationResponsable {
    /// Clé de l'appelant (identifiant de la déclaration), rendue telle quelle dans le résultat.
    pub cle: Uuid,
    pub rang: i16,
    pub nom: String,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub adresse: Option<String>,
    pub lien: String,
    /// Habite avec le premier responsable déclaré.
    pub meme_foyer: bool,
    pub autorite_parentale: bool,
    pub droits: DroitsDeclares,
    pub payeur: bool,
    pub part_paiement: Option<i16>,
    /// Champs réservés du contrat v1 (`CONTRAT_DECLARATION_VERSION`) ; vides par défaut.
    pub reserve: ReserveDeclaree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionPersonne {
    NouvellePersonne,
    MemePersonne(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionFoyer {
    NouveauFoyer,
    MemeFoyer(Uuid),
}

/// Choix humain pour une déclaration (`cle`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub cle: Uuid,
    pub personne: Option<ResolutionPersonne>,
    pub foyer: Option<ResolutionFoyer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoyerCorrespondant {
    pub foyer_id: Uuid,
    pub libelle: String,
}

/// Personne qui pourrait être le responsable déclaré. Aucune coordonnée complète : l'écran de
/// choix n'affiche que l'e-mail masqué.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correspondance {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub email_masque: Option<String>,
    /// `email` (même e-mail déclaré, **non vérifié** : c'est un candidat, jamais une preuve) ou `nom`.
    pub raison: &'static str,
    pub a_un_compte: bool,
    pub foyers: Vec<FoyerCorrespondant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponsableIntegre {
    pub cle: Uuid,
    pub personne_id: Uuid,
    pub personne_creee: bool,
    pub foyer_id: Uuid,
    /// Lien réel créé (ou déjà existant). `None` si le lien est en attente.
    pub lien_id: Option<Uuid>,
    /// Lien en attente de confirmation (personne avec compte).
    pub attente_id: Option<Uuid>,
    pub payeur: bool,
    /// La personne est marquée « ne pas solliciter » : l'appelant ne lui envoie rien.
    pub ne_pas_solliciter: bool,
    /// Alerte de l'adresse à protéger mise de côté (jamais écrite dans la fiche).
    pub adresse_parquee: Option<Uuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultatIntegration {
    pub responsables: Vec<ResponsableIntegre>,
}

impl ResultatIntegration {
    /// Personnes dont le lien est réel et payeuses (pour la suite de la validation).
    pub fn personnes_payeuses(&self) -> Vec<Uuid> {
        self.responsables
            .iter()
            .filter(|r| r.payeur)
            .map(|r| r.personne_id)
            .collect()
    }
}

/// Contenu d'un lien à créer (réel ou en attente).
#[derive(Debug, Clone)]
pub struct LienSpec {
    pub foyer_id: Option<Uuid>,
    pub lien: String,
    pub autorite_parentale: bool,
    pub garde: Option<String>,
    pub date_debut: Option<NaiveDate>,
    pub date_fin: Option<NaiveDate>,
    pub droits: DroitsDeclares,
    pub payeur: bool,
    pub part_paiement: Option<i16>,
}

/// Résultat d'une demande de lien.
#[derive(Debug, Clone)]
pub enum LienResultat {
    Actif(Box<Responsable>),
    /// La personne a un compte : aucun accès avant sa confirmation.
    EnAttente {
        attente_id: Uuid,
        personne_id: Uuid,
    },
}

impl LienResultat {
    /// Le lien réel créé, ou `None` s'il est en attente de confirmation.
    pub fn actif(self) -> Responsable {
        match self {
            LienResultat::Actif(r) => *r,
            LienResultat::EnAttente { .. } => panic!("lien en attente : aucun responsable actif"),
        }
    }
}

/// Commande unique `ajouter_responsable` : une personne nouvelle OU existante, et son lien.
#[derive(Debug, Clone)]
pub struct AjoutResponsable {
    pub nouvelle_personne: Option<PersonneSaisie>,
    pub lien: LienSaisie,
}

/// Lien en attente, tel que la personne concernée le voit (minimisé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LienEnAttente {
    pub id: Uuid,
    pub eleve_prenom: String,
    pub eleve_initiale: String,
    pub lien: String,
    pub demande_le: DateTime<Utc>,
}

/// Filtre des liens en attente à décider.
#[derive(Debug, Clone, Default)]
pub struct FiltreAttente {
    pub source: Option<SourceLien>,
    pub source_ref: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub attente_id: Option<Uuid>,
}

fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

/// `j***@exemple.fr` : jamais l'adresse complète dans un écran de rapprochement.
pub fn masquer_email(e: &str) -> String {
    match e.split_once('@') {
        Some((l, d)) => format!("{}***@{d}", l.chars().next().unwrap_or('*')),
        None => "***".into(),
    }
}

/// Jour métier courant dans le fuseau de l'établissement (Europe/Paris par défaut), calculé par
/// la base.
pub async fn aujourdhui_tx(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    let (d,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    Ok(d)
}

// ---------------------------------------------------------------------------
// Correspondances (lecture)
// ---------------------------------------------------------------------------

/// Personnes de l'établissement qui pourraient être ce responsable : même e-mail déclaré (non
/// vérifié) ou même nom et prénom, avec leurs foyers. **Lecture seule.**
pub async fn correspondances_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DeclarationResponsable,
) -> Result<Vec<Correspondance>, EntDbError> {
    let email = opt(&d.email).map(|e| e.to_lowercase());
    let rows: Vec<(Uuid, String, Option<String>, Option<String>, bool, bool)> = sqlx::query_as(
        "SELECT p.id, p.nom, p.prenom, p.email, (p.user_id IS NOT NULL),
                ($2::text IS NOT NULL AND lower(p.email) = $2) AS par_email
           FROM ent_personne p
          WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL AND p.eleve_id IS NULL
            AND (($2::text IS NOT NULL AND lower(p.email) = $2)
                 OR ($4::text IS NOT NULL AND lower(p.nom) = lower($3)
                     AND lower(p.prenom) = lower($4)))
          ORDER BY par_email DESC, p.created_at LIMIT 10",
    )
    .bind(etab)
    .bind(&email)
    .bind(d.nom.trim())
    .bind(opt(&d.prenom))
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut out = Vec::new();
    for (personne_id, nom, prenom, p_email, a_un_compte, par_email) in rows {
        let foyers: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT f.id, COALESCE(f.libelle, '') FROM ent_eleve_lien l
               JOIN ent_foyer f ON f.id = l.foyer_id AND f.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.personne_id = $2",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        out.push(Correspondance {
            personne_id,
            nom,
            prenom,
            email_masque: p_email.as_deref().map(masquer_email),
            raison: if par_email { "email" } else { "nom" },
            a_un_compte,
            foyers: foyers
                .into_iter()
                .map(|(foyer_id, libelle)| FoyerCorrespondant { foyer_id, libelle })
                .collect(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Écritures en transaction
// ---------------------------------------------------------------------------

/// Crée une personne dans la transaction de l'appelant (journal + événement).
pub async fn creer_personne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &PersonneSaisie,
    source: SourceLien,
    actor: Uuid,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_personne (id, etablissement_id, nom, prenom, civilite, email,
                                   telephone, telephone_2, adresse, source)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(id)
    .bind(etab)
    .bind(&s.nom)
    .bind(&s.prenom)
    .bind(&s.civilite)
    .bind(&s.email)
    .bind(&s.telephone)
    .bind(&s.telephone_2)
    .bind(&s.adresse)
    .bind(source.source_personne())
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journal_tx(
        conn,
        etab,
        None,
        "personne_creee",
        "Personne créée",
        json!({ "personne_id": id, "sans_email": s.email.is_none(), "source": source.code() }),
        false,
        actor,
    )
    .await?;
    publier_tx(
        conn,
        etab,
        types::PERSONNE_CREEE,
        AggregateKind::Personne,
        id,
        None,
        json!({ "personne_id": id, "source": source.source_personne() }),
        actor,
    )
    .await?;
    Ok(id)
}

/// Crée un foyer (libellé dérivé du nom ; un libellé n'est jamais une clé de rapprochement).
pub async fn creer_foyer_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    nom: &str,
) -> Result<Uuid, EntDbError> {
    let f = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1,$2,$3)")
        .bind(f)
        .bind(etab)
        .bind(format!("Foyer {}", nom.trim().to_uppercase()))
        .execute(conn)
        .await
        .map_err(db)?;
    Ok(f)
}

/// Insère le lien réel (journal + événement). Erreur de conflit si la personne est déjà liée.
pub async fn inserer_lien_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    spec: &LienSpec,
    source: &str,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    let r = sqlx::query(
        "INSERT INTO ent_eleve_lien (
             id, etablissement_id, eleve_id, personne_id, foyer_id, lien,
             autorite_parentale, garde, date_debut, date_fin,
             peut_voir_notes, peut_voir_vs, peut_justifier_absence, peut_autoriser_sorties,
             est_payeur, part_paiement, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,now(),now())",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve_id)
    .bind(personne_id)
    .bind(spec.foyer_id)
    .bind(&spec.lien)
    .bind(spec.autorite_parentale)
    .bind(&spec.garde)
    .bind(spec.date_debut)
    .bind(spec.date_fin)
    .bind(spec.droits.notes)
    .bind(spec.droits.vs)
    .bind(spec.droits.justifier)
    .bind(spec.droits.sorties)
    .bind(spec.payeur)
    .bind(spec.part_paiement)
    .execute(&mut *conn)
    .await;
    if let Err(e) = r {
        let d = map_sqlx("dossier_personnes", e);
        return Err(if d == DomainError::Conflit {
            conflit("cette personne est déjà liée à l'élève")
        } else {
            EntDbError::Domain(d)
        });
    }
    let payload = json!({
        "lien_id": id, "personne_id": personne_id, "autorite_parentale": spec.autorite_parentale,
        "source": source,
        "droits": {
            "notes": spec.droits.notes, "vs": spec.droits.vs,
            "justifier": spec.droits.justifier, "sorties": spec.droits.sorties,
        }
    });
    journal_tx(
        conn,
        etab,
        Some(eleve_id),
        "lien_cree",
        "Responsable lié à l'élève",
        payload.clone(),
        false,
        actor,
    )
    .await?;
    publier_tx(
        conn,
        etab,
        types::ELEVE_LIEN_UPDATED,
        AggregateKind::Eleve,
        eleve_id,
        Some(eleve_id),
        payload,
        actor,
    )
    .await?;
    Ok(id)
}

/// Enregistre un lien EN ATTENTE de confirmation par la personne (qui a un compte). Aucun accès
/// n'est ouvert. Idempotent : une demande déjà en attente pour (élève, personne) est reprise.
#[allow(clippy::too_many_arguments)]
pub async fn creer_attente_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    spec: &LienSpec,
    source: SourceLien,
    source_ref: Option<Uuid>,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    let r = sqlx::query(
        "INSERT INTO ent_lien_attente
            (id, etablissement_id, eleve_id, personne_id, foyer_id, lien, autorite_parentale, garde,
             date_debut, date_fin, peut_voir_notes, peut_voir_vs, peut_justifier_absence,
             peut_autoriser_sorties, est_payeur, part_paiement, source, source_ref, demande_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
         ON CONFLICT (etablissement_id, eleve_id, personne_id) WHERE statut = 'en_attente' DO NOTHING",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve_id)
    .bind(personne_id)
    .bind(spec.foyer_id)
    .bind(&spec.lien)
    .bind(spec.autorite_parentale)
    .bind(&spec.garde)
    .bind(spec.date_debut)
    .bind(spec.date_fin)
    .bind(spec.droits.notes)
    .bind(spec.droits.vs)
    .bind(spec.droits.justifier)
    .bind(spec.droits.sorties)
    .bind(spec.payeur)
    .bind(spec.part_paiement)
    .bind(source.code())
    .bind(source_ref)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() == 1 {
        journal_tx(
            conn,
            etab,
            Some(eleve_id),
            "lien_en_attente",
            "Lien en attente de confirmation par la personne",
            json!({ "attente_id": id, "personne_id": personne_id, "source": source.code() }),
            false,
            actor,
        )
        .await?;
        return Ok(id);
    }
    // Une demande est déjà en attente pour (élève, personne) : même origine → la plus récente
    // remplace les droits et dates (la personne confirmera ce que l'établissement vient de
    // demander) ; origine différente → conflit net (jamais de confirmation de droits périmés,
    // ni de demande inconfirmable depuis l'autre canal).
    let (existant, src, src_ref): (Uuid, String, Option<Uuid>) = sqlx::query_as(
        "SELECT id, source, source_ref FROM ent_lien_attente
          WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3 AND statut = 'en_attente'
          FOR UPDATE",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(personne_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if src != source.code() || src_ref != source_ref {
        return Err(conflit(
            "une invitation est déjà en attente pour cette personne et cet élève",
        ));
    }
    sqlx::query(
        "UPDATE ent_lien_attente SET foyer_id = $3, lien = $4, autorite_parentale = $5, garde = $6,
                date_debut = $7, date_fin = $8, peut_voir_notes = $9, peut_voir_vs = $10,
                peut_justifier_absence = $11, peut_autoriser_sorties = $12, est_payeur = $13,
                part_paiement = $14, demande_par = $15
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(existant)
    .bind(spec.foyer_id)
    .bind(&spec.lien)
    .bind(spec.autorite_parentale)
    .bind(&spec.garde)
    .bind(spec.date_debut)
    .bind(spec.date_fin)
    .bind(spec.droits.notes)
    .bind(spec.droits.vs)
    .bind(spec.droits.justifier)
    .bind(spec.droits.sorties)
    .bind(spec.payeur)
    .bind(spec.part_paiement)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(existant)
}

/// Une personne avec compte → attente ; sinon lien réel. Point unique de la règle.
#[allow(clippy::too_many_arguments)]
async fn lier_ou_inviter_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    a_un_compte: bool,
    spec: &LienSpec,
    source: SourceLien,
    source_ref: Option<Uuid>,
) -> Result<(Option<Uuid>, Option<Uuid>), EntDbError> {
    if a_un_compte {
        let a = creer_attente_tx(
            conn,
            etab,
            actor,
            eleve_id,
            personne_id,
            spec,
            source,
            source_ref,
        )
        .await?;
        Ok((None, Some(a)))
    } else {
        let l = inserer_lien_tx(
            conn,
            etab,
            actor,
            eleve_id,
            personne_id,
            spec,
            source.code(),
        )
        .await?;
        Ok((Some(l), None))
    }
}

/// Intègre les responsables déclarés d'un dossier : personnes, foyers, liens. **Aucun
/// rapprochement implicite** : une déclaration qui a des correspondances ne passe pas sans choix
/// humain explicite (`Resolution`). Un lien vers une personne avec compte reste en attente.
pub async fn integrer_declaration_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    declarations: &[DeclarationResponsable],
    resolutions: &[Resolution],
    source: SourceLien,
    actor: Uuid,
) -> Result<ResultatIntegration, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !ok {
        return Err(EntDbError::Domain(DomainError::ReferenceInconnue));
    }
    for r in resolutions {
        if !declarations.iter().any(|d| d.cle == r.cle) {
            return Err(invalide("responsable inconnu dans cette déclaration"));
        }
    }
    // Contrat v1 : les champs réservés sont validés AVANT toute écriture.
    let mut reserves = Vec::with_capacity(declarations.len());
    for d in declarations {
        reserves.push(d.reserve.normaliser()?);
    }
    let mut deja: Vec<Uuid> = vec![];
    let mut foyer_principal: Option<Uuid> = None;
    let mut out = ResultatIntegration::default();
    for (d, reserve) in declarations.iter().zip(&reserves) {
        let res = resolutions.iter().find(|r| r.cle == d.cle);
        let choix_personne = res.and_then(|r| r.personne.clone());
        let choix_foyer = res.and_then(|r| r.foyer.clone());
        let (personne_id, a_un_compte, creee) = match choix_personne {
            Some(ResolutionPersonne::MemePersonne(p)) => {
                let row: Option<(bool,)> = sqlx::query_as(
                    "SELECT user_id IS NOT NULL FROM ent_personne
                      WHERE etablissement_id = $1 AND id = $2
                        AND fusionnee_dans IS NULL AND eleve_id IS NULL
                        FOR UPDATE",
                )
                .bind(etab)
                .bind(p)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                let (compte,) = row.ok_or(EntDbError::Domain(DomainError::ReferenceInconnue))?;
                if deja.contains(&p) {
                    return Err(conflit(
                        "une même personne ne peut pas être choisie pour deux responsables",
                    ));
                }
                let (foyers,): (i64,) = sqlx::query_as(
                    "SELECT count(DISTINCT foyer_id) FROM ent_eleve_lien
                      WHERE etablissement_id = $1 AND personne_id = $2 AND foyer_id IS NOT NULL",
                )
                .bind(etab)
                .bind(p)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                if foyers > 0 && choix_foyer.is_none() {
                    return Err(conflit(
                        "foyer à confirmer : cette personne appartient déjà à un foyer",
                    ));
                }
                deja.push(p);
                (p, compte, false)
            }
            Some(ResolutionPersonne::NouvellePersonne) => (
                creer_personne_declaree_tx(conn, etab, d, reserve, source, actor).await?,
                false,
                true,
            ),
            None => {
                if !correspondances_tx(conn, etab, d).await?.is_empty() {
                    return Err(conflit(
                        "rapprochement à confirmer : une personne correspond à un responsable du dossier",
                    ));
                }
                (
                    creer_personne_declaree_tx(conn, etab, d, reserve, source, actor).await?,
                    false,
                    true,
                )
            }
        };
        // Personne existante : l'adresse à protéger est parquée aussi, les champs réservés sont
        // fusionnés (pour une personne créée, c'est déjà fait par `creer_personne_declaree_tx`).
        let mut adresse_parquee = None;
        if !creee {
            if reserve.adresse_a_proteger {
                if let Some(a) = opt(&d.adresse) {
                    adresse_parquee = Some(
                        parquer_adresse_tx(conn, etab, personne_id, &a, "declaration", Some(d.cle), actor)
                            .await?,
                    );
                }
            }
            if !reserve.est_vide() {
                appliquer_reserve_tx(conn, etab, personne_id, reserve, actor).await?;
            }
        } else if reserve.adresse_a_proteger {
            adresse_parquee = adresse_parquee_de(conn, etab, personne_id, d.cle).await?;
        }
        let ne_pas_solliciter = personne_ne_pas_solliciter_tx(conn, etab, personne_id).await?;
        // Foyer : celui que l'humain a confirmé (fratrie), sinon le foyer du premier responsable
        // pour un conjoint « même foyer », sinon un nouveau foyer.
        let foyer = match choix_foyer {
            Some(ResolutionFoyer::MemeFoyer(f)) => {
                if creee {
                    return Err(conflit(
                        "un foyer existant ne se choisit que pour une personne existante",
                    ));
                }
                let ok: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT foyer_id FROM ent_eleve_lien
                      WHERE etablissement_id = $1 AND personne_id = $2 AND foyer_id = $3 LIMIT 1",
                )
                .bind(etab)
                .bind(personne_id)
                .bind(f)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                ok.ok_or(conflit("ce foyer n'est pas celui de la personne choisie"))?;
                f
            }
            Some(ResolutionFoyer::NouveauFoyer) => creer_foyer_tx(conn, etab, &d.nom).await?,
            None => match foyer_principal {
                Some(f) if d.rang != 1 && d.meme_foyer => f,
                _ => creer_foyer_tx(conn, etab, &d.nom).await?,
            },
        };
        if foyer_principal.is_none() {
            foyer_principal = Some(foyer);
        }
        // Lien déjà existant avec cette personne : rien à créer.
        let existant: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_lien
              WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(personne_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((lien_id,)) = existant {
            out.responsables.push(ResponsableIntegre {
                cle: d.cle,
                personne_id,
                personne_creee: creee,
                foyer_id: foyer,
                lien_id: Some(lien_id),
                attente_id: None,
                payeur: false,
                ne_pas_solliciter,
                adresse_parquee,
            });
            continue;
        }
        let spec = LienSpec {
            foyer_id: Some(foyer),
            lien: d.lien.trim().to_string(),
            autorite_parentale: d.autorite_parentale,
            garde: None,
            date_debut: None,
            date_fin: None,
            droits: d.droits,
            payeur: d.payeur,
            part_paiement: d.part_paiement,
        };
        let (lien_id, attente_id) = lier_ou_inviter_tx(
            conn,
            etab,
            actor,
            eleve_id,
            personne_id,
            a_un_compte,
            &spec,
            source,
            Some(d.cle),
        )
        .await?;
        out.responsables.push(ResponsableIntegre {
            cle: d.cle,
            personne_id,
            personne_creee: creee,
            foyer_id: foyer,
            lien_id,
            attente_id,
            payeur: d.payeur && lien_id.is_some(),
            ne_pas_solliciter,
            adresse_parquee,
        });
    }
    Ok(out)
}

async fn creer_personne_declaree_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DeclarationResponsable,
    reserve: &ReserveDeclaree,
    source: SourceLien,
    actor: Uuid,
) -> Result<Uuid, EntDbError> {
    let adresse = opt(&d.adresse);
    // Adresse à protéger : JAMAIS écrite dans la fiche, parquée scellée juste après.
    let saisie = PersonneSaisie {
        nom: d.nom.trim().to_string(),
        prenom: opt(&d.prenom),
        civilite: opt(&d.civilite),
        email: opt(&d.email).map(|e| e.to_lowercase()),
        telephone: opt(&d.telephone),
        telephone_2: None,
        adresse: if reserve.adresse_a_proteger { None } else { adresse.clone() },
    };
    let id = creer_personne_tx(conn, etab, &saisie, source, actor).await?;
    if !reserve.est_vide() {
        appliquer_reserve_tx(conn, etab, id, reserve, actor).await?;
    }
    if reserve.adresse_a_proteger {
        if let Some(a) = adresse {
            parquer_adresse_tx(conn, etab, id, &a, "declaration", Some(d.cle), actor).await?;
        }
    }
    Ok(id)
}

/// Alerte d'adresse parquée pour cette déclaration (retrouvée après création de la personne).
async fn adresse_parquee_de(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    cle: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_adresse_protegee
          WHERE etablissement_id = $1 AND personne_id = $2 AND source_ref = $3
          ORDER BY created_at DESC LIMIT 1",
    )
    .bind(etab)
    .bind(personne_id)
    .bind(cle)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.map(|(id,)| id))
}

/// Commande du socle 4 : rattache un **compte de connexion** à une personne qui n'en a pas, une
/// fois que la personne l'a confirmé (acceptation d'une invitation). Jamais de réassignation : la
/// personne doit être sans compte, et le compte ne doit pas être déjà rattaché à une autre fiche de
/// l'établissement. Journal et événement dans la transaction de l'appelant.
pub async fn rattacher_compte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    user_id: Uuid,
) -> Result<(), EntDbError> {
    let deja: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND id <> $3",
    )
    .bind(etab)
    .bind(user_id)
    .bind(personne_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if deja.is_some() {
        return Err(conflit(
            "cette identité est déjà rattachée à une autre fiche : contactez l'établissement",
        ));
    }
    let r = sqlx::query(
        "UPDATE ent_personne SET user_id = $3, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2 AND user_id IS NULL
            AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(personne_id)
    .bind(user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() != 1 {
        return Err(conflit("cette fiche vient d'être rattachée à un compte"));
    }
    journal_tx(
        conn,
        etab,
        None,
        "compte_rattache",
        "Compte rattaché à la personne",
        json!({ "personne_id": personne_id }),
        false,
        user_id,
    )
    .await?;
    Ok(())
}

/// Confirmation par la personne : les liens en attente deviennent des liens réels (jamais avant).
/// `decideur` = utilisateur qui confirme (la personne elle-même). Renvoie les liens créés et le
/// nombre de demandes closes (une demande dont le lien existe déjà est close sans nouveau lien).
pub async fn confirmer_liens_attente_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    filtre: &FiltreAttente,
    decideur: Uuid,
) -> Result<(Vec<Uuid>, usize), EntDbError> {
    confirmer_liens_attente_voie_tx(
        conn,
        etab,
        personne_id,
        filtre,
        decideur,
        VoieConfirmation::Invitation,
    )
    .await
}

/// Comme [`confirmer_liens_attente_tx`], en traçant la voie de confirmation (invitation,
/// personne connectée, guichet). Au guichet : type de pièce et auteur, jamais la copie.
pub async fn confirmer_liens_attente_voie_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    filtre: &FiltreAttente,
    decideur: Uuid,
    voie: VoieConfirmation,
) -> Result<(Vec<Uuid>, usize), EntDbError> {
    let lignes = attentes_tx(conn, etab, personne_id, filtre).await?;
    let decidees = lignes.len();
    let mut liens = Vec::new();
    for a in lignes {
        let foyer = match a.foyer_id {
            Some(f) => f,
            None => {
                let (nom,): (String,) = sqlx::query_as(
                    "SELECT nom FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(personne_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                creer_foyer_tx(conn, etab, &nom).await?
            }
        };
        let existe: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_lien
              WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3",
        )
        .bind(etab)
        .bind(a.eleve_id)
        .bind(personne_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if existe.is_none() {
            let spec = LienSpec {
                foyer_id: Some(foyer),
                lien: a.lien.clone(),
                autorite_parentale: a.autorite_parentale,
                garde: a.garde.clone(),
                date_debut: a.date_debut,
                date_fin: a.date_fin,
                droits: DroitsDeclares {
                    notes: a.peut_voir_notes,
                    vs: a.peut_voir_vs,
                    justifier: a.peut_justifier_absence,
                    sorties: a.peut_autoriser_sorties,
                },
                payeur: a.est_payeur,
                part_paiement: a.part_paiement,
            };
            let l = inserer_lien_tx(
                conn,
                etab,
                decideur,
                a.eleve_id,
                personne_id,
                &spec,
                voie.source_lien(),
            )
            .await?;
            liens.push(l);
        }
        sqlx::query(
            "UPDATE ent_lien_attente SET statut = 'confirme', decide_le = now(), decide_par = $3,
                    confirmation_voie = $4, piece_identite_type = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(a.id)
        .bind(decideur)
        .bind(voie.code())
        .bind(voie.piece())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        if matches!(voie, VoieConfirmation::Guichet(_)) {
            journal_tx(
                conn,
                etab,
                Some(a.eleve_id),
                "lien_confirme_guichet",
                "Lien confirmé au guichet (identité vérifiée)",
                json!({ "attente_id": a.id, "personne_id": personne_id,
                        "piece_identite_type": voie.piece() }),
                false,
                decideur,
            )
            .await?;
        }
    }
    Ok((liens, decidees))
}

/// Refus par la personne : la demande est close, aucun accès n'a été ouvert.
pub async fn refuser_liens_attente_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    filtre: &FiltreAttente,
    decideur: Option<Uuid>,
) -> Result<usize, EntDbError> {
    let lignes = attentes_tx(conn, etab, personne_id, filtre).await?;
    for a in &lignes {
        sqlx::query(
            "UPDATE ent_lien_attente SET statut = 'refuse', decide_le = now(), decide_par = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(a.id)
        .bind(decideur)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        // Refus par jeton d'invitation : l'auteur n'est pas forcément un compte (journal alors omis).
        if let Some(u) = decideur {
            journal_tx(
                conn,
                etab,
                Some(a.eleve_id),
                "lien_refuse",
                "Lien refusé par la personne désignée",
                json!({ "attente_id": a.id, "personne_id": personne_id }),
                false,
                u,
            )
            .await?;
        }
    }
    Ok(lignes.len())
}

/// Suspend le lien réel d'une personne sur un élève (ex. refus d'une invitation reçue après
/// l'ouverture du lien d'une personne qui n'avait pas encore de compte).
pub async fn suspendre_lien_personne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    eleve_id: Uuid,
    motif: &str,
    decideur: Option<Uuid>,
) -> Result<bool, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_eleve_lien SET suspendu_le = now(), suspendu_par = $4, updated_at = now(),
                suspension_motif = $5
          WHERE etablissement_id = $1 AND personne_id = $2 AND eleve_id = $3 AND suspendu_le IS NULL",
    )
    .bind(etab)
    .bind(personne_id)
    .bind(eleve_id)
    .bind(decideur)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() > 0 {
        if let Some(u) = decideur {
            journal_tx(
                conn,
                etab,
                Some(eleve_id),
                "lien_suspendu",
                "Lien suspendu",
                json!({ "personne_id": personne_id }),
                false,
                u,
            )
            .await?;
        }
    }
    Ok(r.rows_affected() > 0)
}

#[derive(sqlx::FromRow)]
struct AttenteRow {
    id: Uuid,
    eleve_id: Uuid,
    foyer_id: Option<Uuid>,
    lien: String,
    autorite_parentale: bool,
    garde: Option<String>,
    date_debut: Option<NaiveDate>,
    date_fin: Option<NaiveDate>,
    peut_voir_notes: bool,
    peut_voir_vs: bool,
    peut_justifier_absence: bool,
    peut_autoriser_sorties: bool,
    est_payeur: bool,
    part_paiement: Option<i16>,
}

async fn attentes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    f: &FiltreAttente,
) -> Result<Vec<AttenteRow>, EntDbError> {
    sqlx::query_as(
        "SELECT id, eleve_id, foyer_id, lien, autorite_parentale, garde, date_debut, date_fin,
                peut_voir_notes, peut_voir_vs, peut_justifier_absence, peut_autoriser_sorties,
                est_payeur, part_paiement
           FROM ent_lien_attente
          WHERE etablissement_id = $1 AND personne_id = $2 AND statut = 'en_attente'
            AND ($3::text IS NULL OR source = $3)
            AND ($4::uuid IS NULL OR source_ref = $4)
            AND ($5::uuid IS NULL OR eleve_id = $5)
            AND ($6::uuid IS NULL OR id = $6)
          ORDER BY created_at
          FOR UPDATE",
    )
    .bind(etab)
    .bind(personne_id)
    .bind(f.source.map(SourceLien::code))
    .bind(f.source_ref)
    .bind(f.eleve_id)
    .bind(f.attente_id)
    .fetch_all(conn)
    .await
    .map_err(db)
}

// ---------------------------------------------------------------------------
// Commandes avec contrôle d'accès
// ---------------------------------------------------------------------------

/// Contrôles et écriture d'un lien (existant ou déjà créé) : corps commun de `lier_personne` et
/// d'`ajouter_responsable`, dans la transaction de l'appelant.
pub(super) async fn lier_dans_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    saisie: &LienSaisie,
) -> Result<(Option<Uuid>, Option<Uuid>), EntDbError> {
    let (lien, garde) = saisie.valider()?;
    let p = personne_tx(conn, etab, personne_id, true)
        .await?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    if p.fusionnee_dans.is_some() {
        return Err(conflit("personne fusionnée dans une autre fiche"));
    }
    // L'élève majeur lié à lui-même : sans autorité parentale, sans droit de famille (ses
    // accès viennent de son compte élève).
    let soi_meme = p.eleve_id.is_some();
    if soi_meme && (p.eleve_id != Some(eleve_id) || lien != LIEN_ELEVE_MAJEUR) {
        return Err(invalide("la fiche d'un élève ne se lie qu'à lui-même"));
    }
    if !soi_meme && lien == LIEN_ELEVE_MAJEUR {
        return Err(invalide("lien réservé à l'élève majeur"));
    }
    if let Some(f) = saisie.foyer_id {
        let ok: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_foyer WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(f)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
        ok.ok_or(EntDbError::Domain(DomainError::ReferenceInconnue))?;
    }
    let spec = LienSpec {
        foyer_id: saisie.foyer_id,
        lien,
        autorite_parentale: saisie.autorite_parentale && !soi_meme,
        garde,
        date_debut: saisie.date_debut,
        date_fin: saisie.date_fin,
        droits: DroitsDeclares {
            notes: saisie.peut_voir_notes && !soi_meme,
            vs: saisie.peut_voir_vs && !soi_meme,
            justifier: saisie.peut_justifier_absence && !soi_meme,
            sorties: saisie.peut_autoriser_sorties && !soi_meme,
        },
        payeur: false,
        part_paiement: None,
    };
    // Un lien vers une personne qui a un compte = invitation (jamais d'accès avant confirmation).
    lier_ou_inviter_tx(
        conn,
        etab,
        actor,
        eleve_id,
        personne_id,
        p.user_id.is_some() && !soi_meme,
        &spec,
        SourceLien::Saisie,
        None,
    )
    .await
}

/// Personne déjà connue de l'établissement pour un e-mail (RP2-03).
#[derive(Debug, Clone, Copy)]
pub(crate) struct PersonneParEmail {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub ne_pas_solliciter: bool,
}

impl PgEnt {
    /// RP2-03 : la personne (non fusionnée) de l'établissement portant cet e-mail — celle qui a un
    /// compte d'abord. Permet à l'ancien chemin de réutiliser la fiche au lieu d'en créer une
    /// seconde.
    pub(crate) async fn personne_par_email(
        &self,
        etab: Uuid,
        email: &str,
    ) -> Result<Option<PersonneParEmail>, EntDbError> {
        let row: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, user_id FROM ent_personne
              WHERE etablissement_id = $1 AND lower(email) = lower($2)
                AND fusionnee_dans IS NULL AND eleve_id IS NULL
              ORDER BY (user_id IS NOT NULL) DESC, created_at LIMIT 1",
        )
        .bind(etab)
        .bind(email.trim())
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let Some((id, user_id)) = row else {
            return Ok(None);
        };
        let mut conn = self.pool().acquire().await.map_err(db)?;
        let ne_pas_solliciter = personne_ne_pas_solliciter_tx(&mut conn, etab, id).await?;
        Ok(Some(PersonneParEmail {
            id,
            user_id,
            ne_pas_solliciter,
        }))
    }

    /// RP2-03 : rattache à une fiche existante sans compte le compte que l'ancien chemin vient de
    /// trouver ou créer (aucune seconde fiche). Sans effet si la fiche a déjà ce compte.
    pub(crate) async fn rattacher_compte_a_la_fiche(
        &self,
        etab: Uuid,
        personne_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_personne SET user_id = $3, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2 AND user_id IS NULL
                AND NOT EXISTS (SELECT 1 FROM ent_personne q
                                 WHERE q.etablissement_id = $1 AND q.user_id = $3)",
        )
        .bind(etab)
        .bind(personne_id)
        .bind(user_id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(())
    }

    /// DOS-05 / ADM-08 : lier une personne existante à un élève. Une personne avec compte reçoit
    /// une invitation (lien en attente) ; sinon le lien est créé.
    pub async fn lier_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        saisie: &LienSaisie,
    ) -> Result<LienResultat, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        self.exiger_pas_de_conflit(etab, actor, role, eleve_id).await?;
        self.dossier_exiger_eleve(etab, eleve_id).await?;
        saisie.valider()?;
        let personne_id = saisie
            .personne_id
            .ok_or(invalide("personne à lier obligatoire"))?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let (lien, attente) =
            lier_dans_tx(&mut tx, etab, actor, eleve_id, personne_id, saisie).await?;
        commit(tx).await?;
        self.resultat_lien(etab, personne_id, lien, attente).await
    }

    /// Commande unique `ajouter_responsable` : crée la personne (ou reprend une personne
    /// existante) ET son lien dans **une** transaction. Plus de personne orpheline en cas d'erreur.
    pub async fn ajouter_responsable_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        ajout: &AjoutResponsable,
    ) -> Result<LienResultat, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        self.exiger_pas_de_conflit(etab, actor, role, eleve_id).await?;
        self.dossier_exiger_eleve(etab, eleve_id).await?;
        ajout.lien.valider()?;
        let nouvelle = match (&ajout.nouvelle_personne, ajout.lien.personne_id) {
            (Some(_), Some(_)) | (None, None) => return Err(invalide(
                "indiquer soit une personne existante, soit les données d'une nouvelle personne",
            )),
            (Some(p), None) => Some(p.normaliser()?),
            (None, Some(_)) => None,
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let personne_id = match &nouvelle {
            Some(s) => creer_personne_tx(&mut tx, etab, s, SourceLien::Saisie, actor).await?,
            None => ajout.lien.personne_id.unwrap_or_default(),
        };
        let (lien, attente) =
            lier_dans_tx(&mut tx, etab, actor, eleve_id, personne_id, &ajout.lien).await?;
        commit(tx).await?;
        self.resultat_lien(etab, personne_id, lien, attente).await
    }

    async fn resultat_lien(
        &self,
        etab: Uuid,
        personne_id: Uuid,
        lien: Option<Uuid>,
        attente: Option<Uuid>,
    ) -> Result<LienResultat, EntDbError> {
        match (lien, attente) {
            (Some(l), _) => Ok(LienResultat::Actif(Box::new(
                self.responsable_par_lien(etab, l).await?,
            ))),
            (None, Some(a)) => Ok(LienResultat::EnAttente {
                attente_id: a,
                personne_id,
            }),
            (None, None) => Err(EntDbError::Domain(DomainError::NotFound)),
        }
    }

    /// Liens en attente de confirmation, vus par la personne connectée (saisie directe ou
    /// inscription : « Vous êtes déclaré responsable de X : confirmer »). Minimisé :
    /// prénom et initiale du nom de l'élève, nature du lien.
    pub async fn mes_liens_en_attente(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<LienEnAttente>, EntDbError> {
        let rows: Vec<(Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT a.id, e.prenom, left(e.nom, 1), a.lien, a.created_at
               FROM ent_lien_attente a
               JOIN ent_personne p ON p.id = a.personne_id AND p.etablissement_id = a.etablissement_id
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND p.user_id = $2 AND p.fusionnee_dans IS NULL
                AND a.statut = 'en_attente'
                -- A0-c : une personne « ne pas solliciter » ne voit jamais cette liste (elle
                -- révélerait l'élève et l'établissement) ; sa confirmation passe par le guichet.
                AND NOT EXISTS (SELECT 1 FROM ent_personne_reserve r
                                 WHERE r.etablissement_id = a.etablissement_id
                                   AND r.personne_id = a.personne_id AND r.ne_pas_solliciter)
              ORDER BY a.created_at",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, eleve_prenom, eleve_initiale, lien, demande_le)| LienEnAttente {
                    id,
                    eleve_prenom,
                    eleve_initiale,
                    lien,
                    demande_le,
                },
            )
            .collect())
    }

    /// La personne connectée accepte (le lien devient réel) ou refuse (aucun accès).
    pub async fn repondre_lien_en_attente(
        &self,
        etab: Uuid,
        user_id: Uuid,
        attente_id: Uuid,
        accepter: bool,
    ) -> Result<(), EntDbError> {
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_personne
              WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL FOR UPDATE",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (personne_id,) = p.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        // A0-c : « ne pas solliciter » → aucune réponse en ligne (même réponse qu'une demande inconnue).
        if personne_ne_pas_solliciter_tx(&mut tx, etab, personne_id).await? {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        // A0-d : toute demande en attente de la personne (saisie directe ou inscription) se
        // confirme en un geste depuis son compte ; seule elle peut le faire.
        let filtre = FiltreAttente {
            attente_id: Some(attente_id),
            ..Default::default()
        };
        let n = if accepter {
            confirmer_liens_attente_voie_tx(
                &mut tx,
                etab,
                personne_id,
                &filtre,
                user_id,
                VoieConfirmation::PersonneConnectee,
            )
            .await?
            .1
        } else {
            refuser_liens_attente_tx(&mut tx, etab, personne_id, &filtre, Some(user_id)).await?
        };
        if n == 0 {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        commit(tx).await?;
        Ok(())
    }

    /// Préférences de canal de la personne (e-mail, SMS, courrier). Réservé au personnel habilité
    /// (la personne les modifiera elle-même au lot A7).
    pub async fn definir_preferences_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        canal_email: bool,
        canal_sms: bool,
        courrier_papier: bool,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        self.exiger_pas_de_conflit_personne(etab, actor, role, personne_id).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, personne_id, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        sqlx::query(
            "INSERT INTO ent_personne_preference
                (personne_id, etablissement_id, canal_email, canal_sms, courrier_papier, updated_by)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT (personne_id) DO UPDATE SET canal_email = $3, canal_sms = $4,
                courrier_papier = $5, updated_by = $6, updated_at = now()",
        )
        .bind(personne_id)
        .bind(etab)
        .bind(canal_email)
        .bind(canal_sms)
        .bind(courrier_papier)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            None,
            "preferences_modifiees",
            "Préférences de contact modifiées",
            json!({ "personne_id": personne_id }),
            false,
            actor,
        )
        .await?;
        commit(tx).await?;
        Ok(())
    }
}

/// Commande inverse d'`integrer_declaration_tx` pour l'annulation d'une validation erronée
/// (INS-68) : suspend, sans les effacer, les liens créés par la validation — ceux écrits dans la
/// transaction de la validation, qui portent donc son horodatage. Un lien antérieur (élève déjà
/// connu) n'est jamais touché. Renvoie le nombre de liens suspendus.
pub async fn suspendre_liens_crees_le_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    cree_le: DateTime<Utc>,
    motif: &str,
    decideur: Uuid,
) -> Result<i64, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_eleve_lien SET suspendu_le = now(), suspendu_par = $4, updated_at = now(),
                suspension_motif = $5
          WHERE etablissement_id = $1 AND eleve_id = $2 AND created_at = $3 AND suspendu_le IS NULL",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(cree_le)
    .bind(decideur)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let n = i64::try_from(r.rows_affected()).unwrap_or(i64::MAX);
    if n > 0 {
        journal_tx(
            conn,
            etab,
            Some(eleve_id),
            "liens_suspendus_annulation",
            "Liens suspendus (validation annulée)",
            json!({ "nombre": n }),
            false,
            decideur,
        )
        .await?;
    }
    Ok(n)
}
