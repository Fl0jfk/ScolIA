use chrono::NaiveDate;
use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppelStatut {
    Ouvert,
    Clos,
}

impl AppelStatut {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ouvert => "ouvert",
            Self::Clos => "clos",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s == "clos" {
            Self::Clos
        } else {
            Self::Ouvert
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LigneEtat {
    /// AS2 : mode « présents seulement » : l'élève n'a pas encore été pointé.
    NonPointe,
    Present,
    Absent,
    Retard,
    /// AS2 : localisé ailleurs dans l'établissement (infirmerie, vie scolaire, CDI, entretien…).
    Localise,
    /// AS2 : attendu ailleurs (sortie, stage, examen, mission, dispense…) : jamais une absence.
    AttenduAilleurs,
    /// AS2 : présent mais dispensé de pratique.
    PresentDispense,
}

impl LigneEtat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NonPointe => "non_pointe",
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Retard => "retard",
            Self::Localise => "localise",
            Self::AttenduAilleurs => "attendu_ailleurs",
            Self::PresentDispense => "present_dispense",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "non_pointe" => Some(Self::NonPointe),
            "present" => Some(Self::Present),
            "absent" => Some(Self::Absent),
            "retard" => Some(Self::Retard),
            "localise" => Some(Self::Localise),
            "attendu_ailleurs" => Some(Self::AttenduAilleurs),
            "present_dispense" => Some(Self::PresentDispense),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Appel {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub classe: String,
    pub jour: NaiveDate,
    pub auteur_user_id: Uuid,
    pub statut: AppelStatut,
    pub creneau_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppelLigne {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub appel_id: Uuid,
    pub eleve_id: Uuid,
    pub etat: LigneEtat,
    /// AS2 : minutes de retard (état `retard`), lieu codé (`localise`, `attendu_ailleurs`),
    /// situation connue à l'origine du pré-remplissage, version de la ligne (concurrence).
    pub minutes_retard: Option<i32>,
    pub lieu_code: Option<String>,
    pub situation_id: Option<Uuid>,
    pub version: i32,
}

impl AppelLigne {
    /// Ligne neuve d'un appel (version 1, aucune précision).
    pub fn nouvelle(etablissement_id: Uuid, appel_id: Uuid, eleve_id: Uuid, etat: LigneEtat) -> Self {
        Self {
            id: Uuid::new_v4(),
            etablissement_id,
            appel_id,
            eleve_id,
            etat,
            minutes_retard: None,
            lieu_code: None,
            situation_id: None,
            version: 1,
        }
    }
}

/// Une écriture de ligne d'appel (état, précisions, canal et motif de l'historique).
#[derive(Debug, Clone)]
pub struct EcritureLigne {
    pub etat: LigneEtat,
    pub minutes: Option<i32>,
    pub lieu: Option<String>,
    pub canal: &'static str,
    pub motif_code: Option<&'static str>,
    pub motif_texte: Option<String>,
}

/// Colonnes de l'appel propres à la v2 (typage, horaires figés, périmètre, mode de pointage).
#[derive(Debug, Clone)]
pub struct AppelExt {
    pub type_activite: String,
    pub activite_cle: Option<String>,
    pub heure_debut: Option<chrono::NaiveTime>,
    pub heure_fin: Option<chrono::NaiveTime>,
    pub effectif_complet: bool,
    pub mode_pointage: String,
}

impl AppelExt {
    pub fn cours(heure_debut: chrono::NaiveTime, heure_fin: chrono::NaiveTime, effectif_complet: bool) -> Self {
        Self {
            type_activite: "cours".into(),
            activite_cle: None,
            heure_debut: Some(heure_debut),
            heure_fin: Some(heure_fin),
            effectif_complet,
            mode_pointage: "tous_presents".into(),
        }
    }
}

pub struct PgAppelRepo {
    pool: PgPool,
}

impl PgAppelRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_appel(&self, a: &Appel) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, a.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_appel (
                id, etablissement_id, classe, jour, auteur_user_id, statut,
                creneau_id, matiere_libelle
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(&a.classe)
        .bind(a.jour)
        .bind(a.auteur_user_id)
        .bind(a.statut.as_str())
        .bind(a.creneau_id)
        .bind(a.matiere_libelle.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<Option<Appel>, DomainError> {
        let row = sqlx::query_as::<_, AppelRow>(
            r#"
            SELECT id, etablissement_id, classe, jour, auteur_user_id, statut,
                   creneau_id, matiere_libelle
            FROM ent_appel
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AppelRow::into_appel))
    }

    pub async fn get_by_creneau_jour(
        &self,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<Appel>, DomainError> {
        let row = sqlx::query_as::<_, AppelRow>(
            r#"
            SELECT id, etablissement_id, classe, jour, auteur_user_id, statut,
                   creneau_id, matiere_libelle
            FROM ent_appel
            WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AppelRow::into_appel))
    }

    /// Ouverture **atomique** (AS2, DA-08) : l'appel et toutes ses lignes entrent dans la même
    /// transaction, ou rien. Deux ouvertures simultanées de la même séance (ou de la même activité du
    /// même jour) aboutissent au même appel : la seconde ne crée rien (`Ok(false)`) et relit l'existant.
    pub async fn inserer_atomique_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        a: &Appel,
        ext: &AppelExt,
        lignes: &[AppelLigne],
    ) -> Result<bool, sqlx::Error> {
        let r = sqlx::query(
            r#"
            INSERT INTO ent_appel (
                id, etablissement_id, classe, jour, auteur_user_id, statut,
                creneau_id, matiere_libelle, type_activite, activite_cle, heure_debut, heure_fin,
                effectif_complet, mode_pointage, ouvert_par, etat
            ) VALUES ($1,$2,$3,$4,$5,'ouvert',$6,$7,$8,$9,$10,$11,$12,$13,$5,'en_cours')
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(&a.classe)
        .bind(a.jour)
        .bind(a.auteur_user_id)
        .bind(a.creneau_id)
        .bind(a.matiere_libelle.as_deref())
        .bind(&ext.type_activite)
        .bind(ext.activite_cle.as_deref())
        .bind(ext.heure_debut)
        .bind(ext.heure_fin)
        .bind(ext.effectif_complet)
        .bind(&ext.mode_pointage)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Ok(false);
        }
        for l in lignes {
            sqlx::query(
                "INSERT INTO ent_appel_ligne (id, etablissement_id, appel_id, eleve_id, etat)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(l.id)
            .bind(l.etablissement_id)
            .bind(l.appel_id)
            .bind(l.eleve_id)
            .bind(l.etat.as_str())
            .execute(&mut *tx)
            .await?;
        }
        Ok(true)
    }

    /// Appel d'une activité (sans créneau) identifié par sa clé stable et son jour.
    pub async fn get_by_activite(
        &self,
        etablissement_id: Uuid,
        type_activite: &str,
        activite_cle: &str,
        jour: NaiveDate,
    ) -> Result<Option<Appel>, DomainError> {
        let row = sqlx::query_as::<_, AppelRow>(
            r#"
            SELECT id, etablissement_id, classe, jour, auteur_user_id, statut,
                   creneau_id, matiere_libelle
            FROM ent_appel
            WHERE etablissement_id = $1 AND type_activite = $2 AND activite_cle = $3 AND jour = $4
            "#,
        )
        .bind(etablissement_id)
        .bind(type_activite)
        .bind(activite_cle)
        .bind(jour)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AppelRow::into_appel))
    }

    pub async fn insert_ligne(&self, l: &AppelLigne) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, l.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_appel_ligne (id, etablissement_id, appel_id, eleve_id, etat)
            VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (appel_id, eleve_id) DO UPDATE SET etat = EXCLUDED.etat
            "#,
        )
        .bind(l.id)
        .bind(l.etablissement_id)
        .bind(l.appel_id)
        .bind(l.eleve_id)
        .bind(l.etat.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    /// Modifie l'état d'une ligne **uniquement si l'appel est ouvert** (ancien point d'entrée,
    /// conservé) : voir [`Self::ecrire_ligne_si_ouvert`]. `Ok(false)` si l'appel est absent ou clos,
    /// ou si la ligne n'existe pas.
    pub async fn set_etat_si_ouvert(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        etat: &LigneEtat,
        modifie_par: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let w = EcritureLigne {
            etat: etat.clone(),
            minutes: None,
            lieu: None,
            canal: "web",
            motif_code: Some("saisie"),
            motif_texte: None,
        };
        self.ecrire_ligne_si_ouvert(etablissement_id, appel_id, eleve_id, &w, Some(modifie_par))
            .await
    }

    /// Écrit une ligne **uniquement si l'appel est ouvert**, de façon sérialisée avec la validation :
    /// verrou partagé (`FOR SHARE`) sur l'appel (la validation prend `FOR UPDATE` et attend), ligne
    /// verrouillée, version incrémentée et **historique en ajout seul** dans la même transaction
    /// (R-AS8). `Ok(false)` si l'appel est absent ou non ouvert, ou si la ligne n'existe pas.
    pub async fn ecrire_ligne_si_ouvert(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        w: &EcritureLigne,
        auteur: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await?;
        let statut: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT statut FROM ent_appel
            WHERE etablissement_id = $1 AND id = $2
            FOR SHARE
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await?;
        if statut.as_ref().map(|s| s.0.as_str()) != Some("ouvert") {
            return Ok(false);
        }
        let ok = Self::ecrire_ligne_tx(&mut tx, etablissement_id, appel_id, eleve_id, w, auteur).await?;
        if !ok {
            return Ok(false);
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Écriture d'une ligne dans la transaction de l'appelant (verrou de ligne, version, historique).
    /// `Ok(false)` si la ligne n'existe pas. Aucune vérification du statut de l'appel ici.
    pub async fn ecrire_ligne_tx(
        tx: &mut sqlx::PgConnection,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        w: &EcritureLigne,
        auteur: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        let avant: Option<(Uuid, String, i32)> = sqlx::query_as(
            "SELECT id, etat, version FROM ent_appel_ligne
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((ligne_id, etat_avant, version)) = avant else {
            return Ok(false);
        };
        let nouvelle = version + 1;
        sqlx::query(
            "UPDATE ent_appel_ligne
                SET etat = $4, minutes_retard = $5, lieu_code = $6, version = $7,
                    modifie_par = $8, modifie_le = now()
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .bind(w.etat.as_str())
        .bind(w.minutes)
        .bind(w.lieu.as_deref())
        .bind(nouvelle)
        .bind(auteur)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_appel_ligne_histo
                (id, etablissement_id, appel_id, ligne_id, eleve_id, version, etat_avant, etat_apres,
                 minutes, lieu_code, auteur_id, canal, motif_code, motif_texte)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(ligne_id)
        .bind(eleve_id)
        .bind(nouvelle)
        .bind(&etat_avant)
        .bind(w.etat.as_str())
        .bind(w.minutes)
        .bind(w.lieu.as_deref())
        .bind(auteur)
        .bind(w.canal)
        .bind(w.motif_code)
        .bind(w.motif_texte.as_deref())
        .execute(&mut *tx)
        .await?;
        Ok(true)
    }

    /// Clôture atomique, dans la transaction de l'appelant : verrou exclusif
    /// (`FOR UPDATE`) sur l'appel, statut contrôlé, lignes lues **sous verrou**,
    /// passage à « clos ». L'appelant y écrit aussi absences, retards et
    /// événements (outbox) puis valide le tout en une fois. `Ok(None)` si absent
    /// ou déjà clos.
    pub async fn clore_avec_lignes_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        etablissement_id: Uuid,
        appel_id: Uuid,
        valide_par: Uuid,
        canal: &str,
    ) -> Result<Option<Vec<AppelLigne>>, sqlx::Error> {
        let statut: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT statut FROM ent_appel
            WHERE etablissement_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await?;
        if statut.as_ref().map(|s| s.0.as_str()) != Some("ouvert") {
            return Ok(None);
        }
        let rows = sqlx::query_as::<_, LigneRow>(
            r#"
            SELECT id, etablissement_id, appel_id, eleve_id, etat, minutes_retard, lieu_code, situation_id, version
            FROM ent_appel_ligne
            WHERE etablissement_id = $1 AND appel_id = $2
            ORDER BY eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *tx)
        .await?;
        let result = sqlx::query(
            r#"
            UPDATE ent_appel
            SET statut = 'clos', etat = 'valide', closed_at = now(), valide_par = $3, valide_le = now(),
                canal_validation = $4, modifie_le = now()
            WHERE etablissement_id = $1 AND id = $2 AND statut = 'ouvert'
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(valide_par)
        .bind(canal)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        Ok(Some(rows.into_iter().map(LigneRow::into_ligne).collect()))
    }

    pub async fn list_lignes(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<Vec<AppelLigne>, DomainError> {
        let rows = sqlx::query_as::<_, LigneRow>(
            r#"
            SELECT id, etablissement_id, appel_id, eleve_id, etat, minutes_retard, lieu_code, situation_id, version
            FROM ent_appel_ligne
            WHERE etablissement_id = $1 AND appel_id = $2
            ORDER BY eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(LigneRow::into_ligne).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AppelRow {
    id: Uuid,
    etablissement_id: Uuid,
    classe: String,
    jour: NaiveDate,
    auteur_user_id: Uuid,
    statut: String,
    creneau_id: Option<Uuid>,
    matiere_libelle: Option<String>,
}

impl AppelRow {
    fn into_appel(self) -> Appel {
        Appel {
            id: self.id,
            etablissement_id: self.etablissement_id,
            classe: self.classe,
            jour: self.jour,
            auteur_user_id: self.auteur_user_id,
            statut: AppelStatut::parse(&self.statut),
            creneau_id: self.creneau_id,
            matiere_libelle: self.matiere_libelle,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LigneRow {
    id: Uuid,
    etablissement_id: Uuid,
    appel_id: Uuid,
    eleve_id: Uuid,
    etat: String,
    minutes_retard: Option<i32>,
    lieu_code: Option<String>,
    situation_id: Option<Uuid>,
    version: i32,
}

impl LigneRow {
    fn into_ligne(self) -> AppelLigne {
        AppelLigne {
            id: self.id,
            etablissement_id: self.etablissement_id,
            appel_id: self.appel_id,
            eleve_id: self.eleve_id,
            etat: LigneEtat::parse(&self.etat).unwrap_or(LigneEtat::Present),
            minutes_retard: self.minutes_retard,
            lieu_code: self.lieu_code,
            situation_id: self.situation_id,
            version: self.version,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("appel_repo", err)
}
