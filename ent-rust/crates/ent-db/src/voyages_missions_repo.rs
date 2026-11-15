//! Socle 42, lot V1 : ordres de mission et autorisations écrites des adultes, liste nominative transmise à un
//! prestataire (`docs/socles/42_SORTIES_VOYAGES.md` §10, §13.2, §14, §32.7 ; VOY-152, VOY-271, VOY-289 ; R-SV41).
//!
//! * **ordre de mission** par encadrant (`voyages.autoriser`, direction) : dates, destination, prise en charge
//!   des frais ; **autorisation écrite** du chef d'établissement pour chaque adulte. Documents PDF rendus côté
//!   serveur, déposés au magasin d'objets avec leur empreinte, **servis à la direction et à l'adulte concerné
//!   seulement** (aucune URL publique). La signature de niveau S2 du socle 7 n'existe pas encore : le document est
//!   **émis** par la direction connectée, sa signature électronique viendra avec le socle 7 ;
//! * **liste nominative** (corrige VD-16) : un prestataire ne reçoit par défaut que des **effectifs** ; une liste
//!   nominative demande des **champs choisis dans une liste fermée** (nom, prénom, classe, date de naissance,
//!   sexe : jamais de contact de famille, d'INE ni de santé), un **motif**, le **pays** du destinataire, la
//!   **validation de la direction** et laisse une **trace** (qui, quand, quels champs, quel pays). Un transfert
//!   hors de l'Union européenne est signalé et doit être confirmé (VOY-289) ;
//! * l'envoi passe par la file des prestataires (V0c) : adresse **de l'annuaire** vérifiée, jamais fournie.

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_object::ObjectStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::voyages_exigences::{champ_liste_valide, hors_ue, CHAMPS_LISTE};
use crate::voyages_pdf::{rendre, Bloc};
use crate::voyages_prestataires::{message_envoi, reference_courrier};
use crate::voyages_prestataires_repo::{mettre_en_file, nom_etablissement, NouvelEnvoi};
use crate::voyages_repo::{
    charger_avec_acces, interdit, journaliser, personne_de, publier, refus, texte, texte_opt, tx, Acces, Fait, SortieLigne,
};

const ETATS_DOCUMENTS: [&str; 3] = ["ouverte", "prete", "en_cours"];
const ADULTES_MAX: i64 = 200;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DocumentAdulte {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_: String,
    pub nom: String,
    pub personne_id: Uuid,
    pub sha256: String,
    pub emis_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransmissionSaisie {
    pub prestataire_id: Uuid,
    pub champs: Vec<String>,
    pub motif: String,
    pub pays_destinataire: String,
    /// Obligatoire quand le pays est hors de l'Union européenne (le serveur calcule, l'écran confirme).
    #[serde(default)]
    pub confirme_hors_ue: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TransmissionVue {
    pub id: Uuid,
    pub prestataire_id: Uuid,
    pub raison_sociale: String,
    pub motif: String,
    pub pays_destinataire: String,
    pub hors_ue: bool,
    pub nb_lignes: i32,
    pub statut: String,
    pub demande_le: DateTime<Utc>,
    pub decide_le: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    pub champs: Vec<String>,
}

// ───────────────────────── Ordres de mission ─────────────────────────

fn libelle_role(r: &str) -> &'static str {
    match r {
        "responsable" => "responsable de la sortie",
        "encadrant" => "encadrant",
        "non_compte" => "accompagnant non compté dans l'encadrement",
        _ => "intervenant",
    }
}

fn dates_fr(debut: DateTime<Utc>, fin: DateTime<Utc>) -> String {
    format!("du {} au {}", debut.format("%d/%m/%Y"), fin.format("%d/%m/%Y"))
}

#[allow(clippy::too_many_arguments)] // même forme que `emettre_document` du lot V0b, avec la personne en plus
async fn emettre_pour_adulte(
    e: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    type_: &str,
    personne: Uuid,
    nom: &str,
    pdf: &[u8],
) -> Result<Uuid, EntDbError> {
    let doc = Uuid::new_v4();
    let cle = ObjectStore::key_for(a.etablissement_id, "voyages", doc, "pdf");
    let info = e.objet_put(&cle, pdf).await?;
    crate::objets::enregistrer_objet(&mut *conn, a.etablissement_id, &info, Some("application/pdf")).await?;
    sqlx::query(
        "INSERT INTO ent_sortie_document
             (id, etablissement_id, sortie_id, type, cle_objet, nom, taille, sha256, emis_par_user_id, personne_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(doc)
    .bind(a.etablissement_id)
    .bind(s.id)
    .bind(type_)
    .bind(&cle)
    .bind(nom)
    .bind(pdf.len() as i32)
    .bind(&info.sha256)
    .bind(a.user_id)
    .bind(personne)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(doc)
}

fn peut_gerer_adultes(acces: &Acces) -> bool {
    acces.a(Action::VoyagesAutoriser)
}

impl PgEnt {
    /// VOY-152 : émet l'ordre de mission et l'autorisation écrite de **chaque** adulte de la sortie. Direction
    /// seule (`voyages.autoriser`). Une réémission ajoute de nouveaux documents (le plus récent fait foi, les
    /// précédents restent : ajout seul).
    pub async fn voyage_ordres_mission_emettre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        prise_en_charge: Option<&str>,
    ) -> Result<Vec<DocumentAdulte>, EntDbError> {
        let frais = texte_opt(prise_en_charge, 300)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_gerer_adultes(&acces) {
            return if acces.detail() { Err(interdit()) } else { Err(introuvable()) };
        }
        if !ETATS_DOCUMENTS.contains(&s.etat.as_str()) {
            return Err(refus(RefusMetier::Conflit, "Les ordres de mission s'émettent une fois la sortie autorisée."));
        }
        let adultes: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
            "SELECT p.id, p.nom, p.prenom, se.role
               FROM ent_sortie_encadrant se
               JOIN ent_personne p ON p.etablissement_id = se.etablissement_id AND p.id = se.personne_id
              WHERE se.etablissement_id = $1 AND se.sortie_id = $2 ORDER BY p.nom, p.prenom LIMIT $3",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(ADULTES_MAX)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        if adultes.is_empty() {
            return Err(refus(RefusMetier::Conflit, "Aucun adulte n'encadre cette sortie."));
        }
        let etab = nom_etablissement(&mut t, a.etablissement_id).await?;
        let dest = s.destination_libelle.clone().unwrap_or_else(|| "destination à préciser".into());
        let mut out = Vec::new();
        for (personne, nom, prenom, role) in adultes {
            let identite = format!("{} {}", prenom.as_deref().unwrap_or(""), nom).trim().to_string();
            let mut ordre = vec![
                Bloc::Titre("Ordre de mission".into()),
                Bloc::Espace,
                Bloc::Paragraphe(format!("{etab} charge {identite} d'une mission pour la sortie « {} » ({}).", s.titre, s.reference)),
                Bloc::Ligne { gauche: "Fonction pendant la sortie".into(), droite: libelle_role(&role).into(), gras: false },
                Bloc::Ligne { gauche: "Dates".into(), droite: dates_fr(s.debut, s.fin), gras: false },
                Bloc::Ligne { gauche: "Destination".into(), droite: dest.clone(), gras: false },
            ];
            ordre.push(Bloc::Espace);
            ordre.push(Bloc::SousTitre("Prise en charge des frais".into()));
            ordre.push(Bloc::Paragraphe(
                frais.clone().unwrap_or_else(|| "Selon les règles de l'établissement ; à préciser par la direction.".into()),
            ));
            ordre.push(Bloc::Espace);
            ordre.push(Bloc::Paragraphe("Le chef d'établissement".into()));
            let autorisation = vec![
                Bloc::Titre("Autorisation écrite de participation à l'encadrement".into()),
                Bloc::Espace,
                Bloc::Paragraphe(format!(
                    "Le chef d'établissement de {etab} autorise {identite} à encadrer la sortie « {} » ({}) {} à {dest}.",
                    s.titre,
                    s.reference,
                    dates_fr(s.debut, s.fin)
                )),
                Bloc::Espace,
                Bloc::Paragraphe("Le chef d'établissement".into()),
            ];
            for (type_, blocs, nom_fichier) in [
                ("ordre_mission", ordre, format!("ordre-mission-{}-{}.pdf", s.reference, &personne.simple().to_string()[..8])),
                ("autorisation_adulte", autorisation, format!("autorisation-adulte-{}-{}.pdf", s.reference, &personne.simple().to_string()[..8])),
            ] {
                let pdf = rendre(&format!("ScolIA — {} — {}", if type_ == "ordre_mission" { "ordre de mission" } else { "autorisation" }, s.reference), &blocs);
                let doc = emettre_pour_adulte(self, &mut t, a, &s, type_, personne, &nom_fichier, &pdf).await?;
                out.push(doc);
            }
        }
        journaliser(&mut t, a, id, Fait { champ: Some("ordres_mission"), apres: Some((out.len() / 2).to_string()), ..Fait::simple("ordres_mission_emis") }).await?;
        let ev = publier(&mut t, a, "voyages.ordres_mission_emis", id, json!({ "adultes": out.len() / 2 })).await?;
        let docs = lister(&mut t, a, id, None).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(docs.into_iter().filter(|d| out.contains(&d.id)).collect())
    }

    /// Documents des adultes : la direction voit tous ceux de la sortie ; un adulte ne voit que les siens.
    pub async fn voyage_ordres_mission_lister(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<DocumentAdulte>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let moi = personne_de(&mut t, a).await?;
        let filtre = if peut_gerer_adultes(&acces) { None } else { Some(moi.ok_or_else(introuvable)?) };
        let v = lister(&mut t, a, id, filtre).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Relit un document d'adulte (nom, octets) : direction, ou l'adulte concerné. Empreinte vérifiée.
    pub async fn voyage_ordre_mission_lire(&self, a: &Acteur<'_>, id: Uuid, doc: Uuid) -> Result<(String, Vec<u8>), EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let moi = personne_de(&mut t, a).await?;
        let r: Option<(String, String, String, Uuid)> = sqlx::query_as(
            "SELECT cle_objet, nom, sha256, personne_id FROM ent_sortie_document
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 AND personne_id IS NOT NULL
                AND type IN ('ordre_mission', 'autorisation_adulte')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(doc)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        let Some((cle, nom, sha, personne)) = r else {
            return Err(introuvable());
        };
        if !(peut_gerer_adultes(&acces) || moi == Some(personne)) {
            return Err(introuvable());
        }
        let octets = self.lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha)).await?;
        Ok((nom, octets))
    }

    // ───────────────── Liste nominative à un prestataire ─────────────────

    /// VOY-271 (R-SV41) : prépare la transmission d'une liste nominative à un prestataire. Le porteur ou
    /// `voyages.prestataires` demande ; la **direction** valide. Champs en liste fermée, motif, pays obligatoires ;
    /// destinataire hors UE : confirmation expresse.
    pub async fn voyage_transmission_demander(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        saisie: TransmissionSaisie,
    ) -> Result<TransmissionVue, EntDbError> {
        let motif = texte(&saisie.motif, 500)?;
        let pays = texte(&saisie.pays_destinataire, 80)?;
        let mut champs: Vec<String> = saisie.champs.clone();
        champs.sort();
        champs.dedup();
        if champs.is_empty() || champs.iter().any(|c| !champ_liste_valide(c)) {
            return Err(refus(
                RefusMetier::Invalide,
                "Champ non autorisé : une liste nominative ne porte jamais de contact de famille, d'INE ni de donnée de santé.",
            ));
        }
        let hors = hors_ue(&pays);
        if hors && !saisie.confirme_hors_ue {
            return Err(refus(
                RefusMetier::Invalide,
                "Le destinataire est hors de l'Union européenne : confirmez ce transfert de données.",
            ));
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesPrestataires) {
            return if acces.detail() { Err(interdit()) } else { Err(introuvable()) };
        }
        if crate::voyages::est_terminal(&s.etat) {
            return Err(refus(RefusMetier::Conflit, "Ce dossier n'est plus modifiable."));
        }
        // Un prestataire de l'annuaire de CET établissement, actif et joignable (adresse vérifiée).
        let joignable: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_prestataire p
                              JOIN ent_prestataire_adresse ad ON ad.etablissement_id = p.etablissement_id AND ad.prestataire_id = p.id
                             WHERE p.etablissement_id = $1 AND p.id = $2 AND p.actif AND ad.desactivee_le IS NULL)",
        )
        .bind(a.etablissement_id)
        .bind(saisie.prestataire_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !joignable {
            return Err(introuvable());
        }
        let nb: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2 AND etat IN ('inscrit', 'pret', 'parti')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let tid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_transmission
                 (id, etablissement_id, sortie_id, prestataire_id, motif, pays_destinataire, hors_ue, nb_lignes, demande_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(tid)
        .bind(a.etablissement_id)
        .bind(id)
        .bind(saisie.prestataire_id)
        .bind(&motif)
        .bind(&pays)
        .bind(hors)
        .bind(nb as i32)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        for c in &champs {
            sqlx::query("INSERT INTO ent_sortie_transmission_champ (id, etablissement_id, transmission_id, champ) VALUES ($1,$2,$3,$4)")
                .bind(Uuid::new_v4())
                .bind(a.etablissement_id)
                .bind(tid)
                .bind(c)
                .execute(&mut *t)
                .await
                .map_err(db)?;
        }
        journaliser(&mut t, a, id, Fait { champ: Some("transmission"), apres: Some(champs.join(",")), ..Fait::simple("liste_nominative_demandee") }).await?;
        let v = vue_transmissions(&mut t, a.etablissement_id, id, Some(tid)).await?;
        t.commit().await.map_err(db)?;
        v.into_iter().next().ok_or_else(introuvable)
    }

    /// Transmissions demandées pour la sortie (organisateur et direction ; jamais un autre profil).
    pub async fn voyage_transmissions_lire(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<TransmissionVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.gere(Action::VoyagesPrestataires) || acces.a(Action::VoyagesAutoriser)) {
            return if acces.detail() { Err(interdit()) } else { Err(introuvable()) };
        }
        let v = vue_transmissions(&mut t, a.etablissement_id, id, None).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Décision de la **direction** (`voyages.autoriser`) sur une transmission : à l'acceptation, la liste est
    /// composée côté serveur (champs choisis seulement, participants qui partent), déposée et mise en file
    /// d'envoi vers l'adresse **de l'annuaire** ; au refus, un motif.
    pub async fn voyage_transmission_decider(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        transmission: Uuid,
        accepter: bool,
        motif_refus: Option<&str>,
    ) -> Result<TransmissionVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesAutoriser) {
            return if acces.detail() { Err(interdit()) } else { Err(introuvable()) };
        }
        let (prestataire, statut, hors): (Uuid, String, bool) = sqlx::query_as(
            "SELECT prestataire_id, statut, hors_ue FROM ent_sortie_transmission
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(transmission)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if statut != "a_valider" {
            return Err(conflit());
        }
        if !accepter {
            let m = texte(motif_refus.unwrap_or(""), 500)?;
            sqlx::query(
                "UPDATE ent_sortie_transmission SET statut = 'refusee', decide_par_user_id = $4, decide_le = now(), motif_refus = $5
                  WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(transmission)
            .bind(a.user_id)
            .bind(&m)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            journaliser(&mut t, a, id, Fait::simple("liste_nominative_refusee")).await?;
        } else {
            let champs: Vec<String> = sqlx::query_scalar(
                "SELECT champ FROM ent_sortie_transmission_champ WHERE etablissement_id = $1 AND transmission_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(transmission)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            let champs: Vec<&str> = CHAMPS_LISTE.iter().copied().filter(|c| champs.iter().any(|x| x == c)).collect();
            let lignes = lignes_liste(&mut t, a, &s, &champs).await?;
            let etab = nom_etablissement(&mut t, a.etablissement_id).await?;
            let mut blocs = vec![
                Bloc::Titre(format!("Liste des participants — {}", s.titre)),
                Bloc::Espace,
                Bloc::Paragraphe(format!("{etab} — sortie {} {}.", s.reference, dates_fr(s.debut, s.fin))),
                Bloc::Paragraphe("Document confidentiel, établi pour l'organisation de la sortie : ne pas le transmettre.".into()),
                Bloc::Espace,
                Bloc::Paragraphe(format!("Champs : {}", champs.join(", "))),
            ];
            blocs.extend(lignes.iter().map(|l| Bloc::Paragraphe(l.clone())));
            let pdf = rendre(&format!("ScolIA — liste nominative — {}", s.reference), &blocs);
            let doc = Uuid::new_v4();
            let cle = ObjectStore::key_for(a.etablissement_id, "voyages", doc, "pdf");
            let info = self.objet_put(&cle, &pdf).await?;
            crate::objets::enregistrer_objet(&mut *t, a.etablissement_id, &info, Some("application/pdf")).await?;
            sqlx::query(
                "INSERT INTO ent_sortie_document (id, etablissement_id, sortie_id, type, cle_objet, nom, taille, sha256, emis_par_user_id)
                 VALUES ($1,$2,$3,'liste_nominative',$4,$5,$6,$7,$8)",
            )
            .bind(doc)
            .bind(a.etablissement_id)
            .bind(id)
            .bind(&cle)
            .bind(format!("liste-{}.pdf", s.reference))
            .bind(pdf.len() as i32)
            .bind(&info.sha256)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            // Adresse DE L'ANNUAIRE (la plus ancienne vérifiée), jamais fournie par la demande.
            let adresse: Uuid = sqlx::query_scalar(
                "SELECT id FROM ent_prestataire_adresse WHERE etablissement_id = $1 AND prestataire_id = $2 AND desactivee_le IS NULL
                  ORDER BY verifiee_le, id LIMIT 1",
            )
            .bind(a.etablissement_id)
            .bind(prestataire)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?
            .ok_or_else(|| refus(RefusMetier::Conflit, "Le prestataire n'a plus d'adresse vérifiée."))?;
            let reference = reference_courrier(&s.reference, a.etablissement_id, s.id);
            let (objet, corps) = message_envoi("liste_nominative", &etab, &reference, None);
            let envoi = mettre_en_file(
                &mut t,
                a,
                id,
                NouvelEnvoi {
                    prestataire,
                    adresse,
                    type_: "liste_nominative",
                    reference: &reference,
                    objet,
                    corps,
                    document: Some(doc),
                    commande: None,
                    copie: None,
                },
            )
            .await?;
            sqlx::query(
                "UPDATE ent_sortie_transmission SET statut = 'validee', decide_par_user_id = $4, decide_le = now(), document_id = $5, envoi_id = $6
                  WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(transmission)
            .bind(a.user_id)
            .bind(doc)
            .bind(envoi)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            journaliser(&mut t, a, id, Fait { champ: Some(if hors { "hors_ue" } else { "ue" }), ..Fait::simple("liste_nominative_validee") }).await?;
        }
        let ev = publier(&mut t, a, if accepter { "voyages.liste_nominative_validee" } else { "voyages.liste_nominative_refusee" }, id, json!({ "hors_ue": hors }))
            .await?;
        let v = vue_transmissions(&mut t, a.etablissement_id, id, Some(transmission)).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        if accepter {
            let _ = self.voyages_envois_traiter(a.etablissement_id, 20).await;
        }
        v.into_iter().next().ok_or_else(introuvable)
    }
}

async fn lister(conn: &mut PgConnection, a: &Acteur<'_>, id: Uuid, personne: Option<Uuid>) -> Result<Vec<DocumentAdulte>, EntDbError> {
    sqlx::query_as(
        "SELECT id, type, nom, personne_id, sha256, emis_le FROM ent_sortie_document
          WHERE etablissement_id = $1 AND sortie_id = $2 AND type IN ('ordre_mission', 'autorisation_adulte')
            AND ($3::uuid IS NULL OR personne_id = $3)
          ORDER BY emis_le DESC, id",
    )
    .bind(a.etablissement_id)
    .bind(id)
    .bind(personne)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn vue_transmissions(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    seule: Option<Uuid>,
) -> Result<Vec<TransmissionVue>, EntDbError> {
    let mut v: Vec<TransmissionVue> = sqlx::query_as(
        "SELECT x.id, x.prestataire_id, p.raison_sociale, x.motif, x.pays_destinataire, x.hors_ue, x.nb_lignes,
                x.statut, x.demande_le, x.decide_le
           FROM ent_sortie_transmission x JOIN ent_prestataire p ON p.etablissement_id = x.etablissement_id AND p.id = x.prestataire_id
          WHERE x.etablissement_id = $1 AND x.sortie_id = $2 AND ($3::uuid IS NULL OR x.id = $3)
          ORDER BY x.demande_le, x.id",
    )
    .bind(etab)
    .bind(sortie)
    .bind(seule)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for x in &mut v {
        x.champs = sqlx::query_scalar("SELECT champ FROM ent_sortie_transmission_champ WHERE etablissement_id = $1 AND transmission_id = $2 ORDER BY champ")
            .bind(etab)
            .bind(x.id)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
    }
    Ok(v)
}

/// Lignes de la liste : **seulement** les champs choisis, pour les participants qui partent. Aucun contact de
/// famille, aucun INE, aucune donnée de santé n'est lu (corrige VD-16).
async fn lignes_liste(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne, champs: &[&str]) -> Result<Vec<String>, EntDbError> {
    let participants = crate::voyages_repo::liste_participants(conn, a, s, &["inscrit", "pret", "parti"]).await?;
    let mut naissance: std::collections::HashMap<Uuid, (Option<NaiveDate>, Option<String>)> = std::collections::HashMap::new();
    if champs.iter().any(|c| matches!(*c, "date_naissance" | "sexe")) {
        let rows: Vec<(Uuid, Option<NaiveDate>, Option<String>)> = sqlx::query_as(
            "SELECT e.id, e.date_naissance, e.sexe FROM ent_eleve e
               JOIN ent_sortie_participant sp ON sp.etablissement_id = e.etablissement_id AND sp.eleve_id = e.id
              WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        naissance = rows.into_iter().map(|(e, d, x)| (e, (d, x))).collect();
    }
    let mut out = Vec::new();
    for p in participants {
        let (dn, sexe) = naissance.get(&p.eleve_id).cloned().unwrap_or((None, None));
        let mut morceaux: Vec<String> = Vec::new();
        for c in champs {
            match *c {
                "nom" => morceaux.push(crate::voyages_prestataires::ligne_propre(&p.nom, 80)),
                "prenom" => morceaux.push(crate::voyages_prestataires::ligne_propre(&p.prenom, 80)),
                "classe" => morceaux.push(p.classe.clone().unwrap_or_default()),
                "date_naissance" => morceaux.push(dn.map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_default()),
                "sexe" => morceaux.push(sexe.clone().unwrap_or_default()),
                _ => return Err(invalide()),
            }
        }
        out.push(morceaux.join(" — "));
    }
    Ok(out)
}
