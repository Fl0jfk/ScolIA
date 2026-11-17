//! Rapprochement de la reprise (socle 28, lot R3), côté **outil** : instantané de ScolIA lu pour comparer, trace de
//! la file « à décider », purge des résumés. Toutes les requêtes passent par `TenantTx` (RLS d'établissement) et
//! filtrent sur `etablissement_id`. **Rien n'est écrit dans les tables métier** : la file et les décisions sont des
//! tables `rep_*`.
//!
//! Les actions d'une personne habilitée (voir la file, décider, annuler) sont dans [`crate::reprise_file`].

use crate::reprise::{ErreurReprise, Execution};
use crate::tenant_tx::TenantTx;
use chrono::NaiveDate;
use ent_reprise::correspondance::id_derive;
use ent_reprise::rapprochement::donnees::{
    CibleAgent, CibleAnnee, CibleClasse, CibleCompte, CibleEleve, CiblePersonne,
    CibleRapprochement, CibleSite,
};
use ent_reprise::rapprochement::{Resume, Synonymes};
use ent_reprise::simulation::LigneFile;
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

fn ids(v: Vec<Uuid>) -> Vec<String> {
    v.into_iter().map(|u| u.to_string()).collect()
}

/// Instantané de ScolIA pour l'établissement cible : élèves, personnes, agents, comptes, années, sites, classes de
/// l'année courante, synonymes validés. Lecture seule.
pub async fn charger_cible(pool: &PgPool, etab: Uuid) -> Result<CibleRapprochement, ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    let mut c = CibleRapprochement::default();

    for l in sqlx::query(
        "SELECT id, nom, nom_naissance, prenom, prenoms, ine, date_naissance, classe, status
           FROM ent_eleve WHERE etablissement_id = $1 ORDER BY id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        let statut: String = l.get("status");
        let prenoms: Option<Vec<String>> = l.get("prenoms");
        c.eleves.push(CibleEleve {
            id: id.to_string(),
            nom: l.get("nom"),
            nom_naissance: l.get("nom_naissance"),
            prenom: l.get("prenom"),
            prenoms: prenoms.unwrap_or_default(),
            ine: l.get("ine"),
            date_naissance: l.get("date_naissance"),
            classe: l.get("classe"),
            actif: statut == "actif",
        });
    }

    for l in sqlx::query(
        "SELECT p.id, p.nom, p.prenom, p.email, p.telephone,
                COALESCE(array_agg(DISTINCT k.eleve_id) FILTER (WHERE k.eleve_id IS NOT NULL), ARRAY[]::uuid[]) AS eleves
           FROM ent_personne p
           LEFT JOIN ent_eleve_lien k ON k.etablissement_id = p.etablissement_id AND k.personne_id = p.id
          WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL
          GROUP BY p.id, p.nom, p.prenom, p.email, p.telephone
          ORDER BY p.id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        c.personnes.push(CiblePersonne {
            id: id.to_string(),
            nom: l.get("nom"),
            prenom: l.get("prenom"),
            email: l.get("email"),
            telephone: l.get("telephone"),
            eleves: ids(l.get("eleves")),
        });
    }

    for l in sqlx::query(
        "SELECT a.id, p.nom, p.prenom, p.email, u.email AS email_compte
           FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
           LEFT JOIN ent_user u ON u.id = p.user_id
          WHERE a.etablissement_id = $1 AND a.fusionne_dans_agent_id IS NULL
          ORDER BY a.id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        let emails: Vec<String> = [l.get::<Option<String>, _>("email"), l.get("email_compte")]
            .into_iter()
            .flatten()
            .collect();
        c.agents.push(CibleAgent {
            id: id.to_string(),
            nom: l.get("nom"),
            prenom: l.get("prenom"),
            emails,
        });
    }

    for l in sqlx::query(
        "SELECT u.id, u.email FROM ent_user u
           JOIN ent_membership m ON m.user_id = u.id
          WHERE m.etablissement_id = $1 ORDER BY u.id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        c.comptes.push(CibleCompte {
            id: id.to_string(),
            email: l.get("email"),
        });
    }

    for l in sqlx::query(
        "SELECT id, libelle, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 ORDER BY date_debut, id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        c.annees.push(CibleAnnee {
            id: id.to_string(),
            libelle: l.get("libelle"),
            debut: l.get("date_debut"),
            fin: l.get("date_fin"),
        });
    }

    for l in sqlx::query(
        "SELECT id, code, libelle FROM ent_site WHERE etablissement_id = $1 AND actif ORDER BY id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        c.sites.push(CibleSite {
            id: id.to_string(),
            code: l.get("code"),
            libelle: l.get("libelle"),
        });
    }

    for l in sqlx::query(
        "SELECT k.id, k.code, k.libelle FROM ent_classe k
           JOIN ent_annee_scolaire a ON a.etablissement_id = k.etablissement_id AND a.id = k.annee_id
          WHERE k.etablissement_id = $1 AND a.courante ORDER BY k.id",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let id: Uuid = l.get("id");
        c.classes.push(CibleClasse {
            id: id.to_string(),
            code: l.get("code"),
            libelle: l.get("libelle"),
        });
    }
    c.synonymes = synonymes_valides_tx(&mut tx, etab).await?;
    tx.rollback().await?;
    Ok(c)
}

/// Équivalences de libellés validées par une personne et non annulées.
pub async fn synonymes_valides(pool: &PgPool, etab: Uuid) -> Result<Synonymes, ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    let s = synonymes_valides_tx(&mut tx, etab).await?;
    tx.rollback().await?;
    Ok(s)
}

async fn synonymes_valides_tx(
    tx: &mut PgConnection,
    etab: Uuid,
) -> Result<Synonymes, ErreurReprise> {
    let mut s = Synonymes::new();
    for l in sqlx::query(
        "SELECT domaine, valeur_cle, valeur_cible_id FROM rep_synonyme
          WHERE etablissement_id = $1 AND source = 'next' AND valide_le IS NOT NULL AND annule_le IS NULL",
    )
    .bind(etab)
    .fetch_all(&mut *tx)
    .await?
    {
        let cible: Uuid = l.get("valeur_cible_id");
        s.insert((l.get("domaine"), l.get("valeur_cle")), cible.to_string());
    }
    Ok(s)
}

fn jour(r: &Resume) -> Option<NaiveDate> {
    r.naissance
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// Écrit la file « à décider » d'une exécution : propositions, candidats, raisons (codes). Dans la transaction de
/// la trace. Identifiants dérivés : rejouer la même exécution donne les mêmes lignes.
pub async fn ecrire_file(
    tx: &mut PgConnection,
    etab: Uuid,
    exec: Execution,
    lignes: &[LigneFile],
) -> Result<u64, ErreurReprise> {
    let etab_txt = etab.to_string();
    let mut n = 0u64;
    for l in lignes {
        let cle = format!("{}:{}", exec.id, l.source_id);
        let id = Uuid::parse_str(&id_derive(&etab_txt, "rep_proposition", &l.objet, &cle))
            .map_err(|_| ErreurReprise::Refus("identifiant de proposition invalide".into()))?;
        let r = &l.resume_source;
        sqlx::query(
            "INSERT INTO rep_proposition
                 (id, etablissement_id, execution_id, objet, source_id, source_ref, cible_objet, motif, meilleure_confiance,
                  empreinte_source, src_nom, src_prenom, src_naissance, src_classe, src_ine_fin, src_contact, src_detail)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
        )
        .bind(id)
        .bind(etab)
        .bind(exec.id)
        .bind(&l.objet)
        .bind(&l.source_id)
        .bind(&l.source_ref)
        .bind(&l.cible_objet)
        .bind(l.motif.code())
        .bind(l.candidats.first().map(|c| c.confiance.code()))
        .bind(&l.empreinte)
        .bind(&r.nom)
        .bind(&r.prenom)
        .bind(jour(r))
        .bind(&r.classe)
        .bind(&r.ine_fin)
        .bind(&r.contact_masque)
        .bind(&r.detail)
        .execute(&mut *tx)
        .await?;
        for (i, c) in l.candidats.iter().enumerate() {
            let rang = i16::try_from(i + 1)
                .map_err(|_| ErreurReprise::Refus("trop de candidats".into()))?;
            let cible = Uuid::parse_str(&c.cible_id)
                .map_err(|_| ErreurReprise::Refus("identifiant de candidat invalide".into()))?;
            sqlx::query(
                "INSERT INTO rep_proposition_candidat
                     (etablissement_id, proposition_id, rang, cible_id, confiance, cand_nom, cand_prenom, cand_naissance,
                      cand_classe, cand_ine_fin, cand_contact, cand_detail)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .bind(etab)
            .bind(id)
            .bind(rang)
            .bind(cible)
            .bind(c.confiance.code())
            .bind(&c.resume.nom)
            .bind(&c.resume.prenom)
            .bind(jour(&c.resume))
            .bind(&c.resume.classe)
            .bind(&c.resume.ine_fin)
            .bind(&c.resume.contact_masque)
            .bind(&c.resume.detail)
            .execute(&mut *tx)
            .await?;
            for raison in &c.raisons {
                sqlx::query(
                    "INSERT INTO rep_proposition_raison (etablissement_id, proposition_id, rang, code, polarite)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(etab)
                .bind(id)
                .bind(rang)
                .bind(raison.code())
                .bind(raison.polarite().code())
                .execute(&mut *tx)
                .await?;
            }
        }
        n += 1;
    }
    Ok(n)
}

/// Purge des résumés minimisés de la file (mise à NULL) pour cette exécution et les précédentes : à faire quand la
/// zone de reprise est vidée (MSE-179). Les propositions, raisons et décisions restent comme preuve, sans donnée
/// d'identité. Retourne le nombre de propositions purgées.
pub async fn purger_resumes(
    tx: &mut PgConnection,
    etab: Uuid,
    exec: Execution,
) -> Result<u64, ErreurReprise> {
    sqlx::query(
        "UPDATE rep_proposition_candidat SET cand_nom = NULL, cand_prenom = NULL, cand_naissance = NULL,
                cand_classe = NULL, cand_ine_fin = NULL, cand_contact = NULL, cand_detail = NULL
          WHERE etablissement_id = $1
            AND proposition_id IN (
                SELECT p.id FROM rep_proposition p
                  JOIN rep_execution e ON e.etablissement_id = p.etablissement_id AND e.id = p.execution_id
                 WHERE p.etablissement_id = $1 AND e.numero <= $2)",
    )
    .bind(etab)
    .bind(exec.numero)
    .execute(&mut *tx)
    .await?;
    let r = sqlx::query(
        "UPDATE rep_proposition p SET src_nom = NULL, src_prenom = NULL, src_naissance = NULL, src_classe = NULL,
                src_ine_fin = NULL, src_contact = NULL, src_detail = NULL, resume_purge_le = now()
           FROM rep_execution e
          WHERE p.etablissement_id = $1 AND e.etablissement_id = p.etablissement_id AND e.id = p.execution_id
            AND e.numero <= $2 AND p.resume_purge_le IS NULL",
    )
    .bind(etab)
    .bind(exec.numero)
    .execute(&mut *tx)
    .await?;
    Ok(r.rows_affected())
}
