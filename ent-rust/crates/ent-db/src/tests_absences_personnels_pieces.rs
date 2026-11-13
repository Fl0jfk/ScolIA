//! Socle 15, lot A2 — pièces, fil, conservation, contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Un établissement neuf par test. Critères de fin du lot (`15_ABSENCES_PERSONNELS.md` §28.1) :
//! pièce lisible du seul cercle avec lecture journalisée ; purge à la date (test d'horloge) ; jamais de
//! pièce jointe ni de nom de fichier ; gel qui suspend la purge ; cloisonnement d'établissement.

use super::*;
use crate::cahier::FichierDepot;
use chrono::{Duration, NaiveDate};
use domain_eleve::{today_utc, DomainError};
use ent_object::presign::Presigner;
use ent_types::absences_personnels::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-A15-Aa9!";
const PDF: &[u8] = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
/// Nom d'origine « parlant » : il ne doit apparaître nulle part (minimisation).
const NOM_SENSIBLE: &str = "arret-chimiotherapie-dr-martin.pdf";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.installer_presigner(Presigner::local(b"cle-de-test-absences-0123456789abcdef", 60).unwrap());
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("a15p-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(
        &email,
        MDP,
        Some(&format!("Nom {role} {}", &Uuid::new_v4().simple().to_string()[..6])),
        etab,
        role,
    )
    .await
    .expect("compte")
    .id
}

struct X {
    ent: PgEnt,
    etab: Uuid,
    dir: Uuid,
    sec: Uuid,
    compta: Uuid,
    cpe: Uuid,
    admin: Uuid,
    prof: Uuid,
    prof2: Uuid,
}

impl X {
    async fn nouveau() -> Option<X> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let sec = compte(&ent, etab, "administratif").await;
        let compta = compte(&ent, etab, "compta").await;
        let cpe = compte(&ent, etab, "cpe").await;
        let admin = compte(&ent, etab, "admin").await;
        let prof = compte(&ent, etab, "prof").await;
        let prof2 = compte(&ent, etab, "prof").await;
        ent.referentiel_annee_courante_assuree(etab).await.unwrap();
        Some(X { ent, etab, dir, sec, compta, cpe, admin, prof, prof2 })
    }

    /// Absence du compte `user` (agent de l'État par défaut), nature `code`, dans les jours à venir.
    async fn declarer(&self, user: Uuid, code: &str) -> Uuid {
        let d = today_utc() + Duration::days(3);
        self.ent
            .declarer_absence_for_actor(
                self.etab,
                user,
                "prof",
                &DeclarerAbsenceBody {
                    nature_code: Some(code.to_string()),
                    debut: d,
                    fin: d + Duration::days(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_else(|e| panic!("déclaration refusée : {e:?}"))
            .absences[0]
            .id
    }

    fn fichier(contenu: &[u8]) -> FichierDepot {
        FichierDepot { nom: NOM_SENSIBLE.into(), mime_type: "application/pdf".into(), contenu: contenu.to_vec() }
    }

    async fn deposer(&self, user: Uuid, role: &str, absence: Uuid, type_piece: &str) -> PieceAbsenceDto {
        self.ent
            .deposer_piece_absence_for_actor(
                self.etab, user, role, absence, type_piece, true, Self::fichier(PDF),
            )
            .await
            .unwrap_or_else(|e| panic!("dépôt refusé : {e:?}"))
    }

    async fn jour(&self) -> NaiveDate {
        sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, now())")
            .bind(self.etab)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn compter(&self, sql: &str, absence: Uuid) -> i64 {
        sqlx::query_scalar(sql).bind(self.etab).bind(absence).fetch_one(self.ent.pool()).await.unwrap()
    }

    async fn nb_events(&self, type_name: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2")
            .bind(self.etab)
            .bind(type_name)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    /// L'agent du compte passe sous l'employeur de droit privé (OGEC) : purge à 3 ans, pas de transmission.
    async fn en_ogec(&self, user: Uuid) {
        sqlx::query(
            "UPDATE ent_agent_engagement SET employeur_type = 'organisme_gestion'
              WHERE etablissement_id = $1 AND agent_id = (SELECT a.id FROM ent_agent a
                    JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                   WHERE a.etablissement_id = $1 AND p.user_id = $2)",
        )
        .bind(self.etab)
        .bind(user)
        .execute(self.ent.pool())
        .await
        .unwrap();
    }

    async fn transmettre(&self, absence: Uuid) {
        sqlx::query(
            "INSERT INTO ent_absp_decision (id, etablissement_id, absence_id, type, auteur_user_id, voie_transmission)
             VALUES ($1,$2,$3,'transmission',$4,'rectorat')",
        )
        .bind(Uuid::new_v4())
        .bind(self.etab)
        .bind(absence)
        .bind(self.sec)
        .execute(self.ent.pool())
        .await
        .unwrap();
    }
}

macro_rules! x {
    () => {
        match X::nouveau().await {
            Some(c) => c,
            None => {
                eprintln!("skip : ENT_DATABASE_URL");
                return;
            }
        }
    };
}

fn introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(DomainError::NotFound))
}

fn est_conflit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(crate::service::RefusMetier::Conflit, _))
}

// ───────────────────────────────── Pièces : cercle, lecture, minimisation ─────────────────────────────────

/// Critère de fin : pièce lisible du seul cercle, lecture journalisée ; aucun nom de fichier.
#[tokio::test]
async fn a2_piece_lisible_du_seul_cercle_avec_lecture_journalisee() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    assert_eq!(p.etat, "deposee");
    // L'agent, la direction (lire_nature) et le secrétariat de l'État lisent ; chacun est journalisé.
    for (user, role) in [(x.prof, "prof"), (x.dir, "direction"), (x.sec, "administratif")] {
        let l = x
            .ent
            .lien_piece_absence_for_actor(x.etab, user, role, abs, p.id)
            .await
            .unwrap_or_else(|e| panic!("{role} devrait lire : {e:?}"));
        assert!(l.url.len() > 10);
        assert!(!l.url.contains("chimio") && !l.url.contains("martin"), "nom d'origine dans le lien : {}", l.url);
        let ttl = l.expires_at - ent_object::presign::now_unix();
        assert!((0..=300).contains(&ttl), "lien de {ttl} s : plus de 5 minutes");
    }
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_absp_piece_lecture l JOIN ent_absp_piece p ON p.etablissement_id = l.etablissement_id AND p.id = l.piece_id WHERE l.etablissement_id = $1 AND p.absence_id = $2", abs).await,
        3,
        "chaque lecture est journalisée"
    );
    // Hors cercle : « introuvable » (pas d'oracle), et aucune lecture journalisée en plus.
    for (user, role) in [
        (x.compta, "compta"),
        (x.prof2, "prof"),
        (x.cpe, "cpe"),
        (x.admin, "admin"),
    ] {
        let e = x.ent.lien_piece_absence_for_actor(x.etab, user, role, abs, p.id).await.unwrap_err();
        assert!(introuvable(&e), "{role} : {e:?}");
        let e = x.ent.pieces_absence_for_actor(x.etab, user, role, abs).await.unwrap_err();
        assert!(introuvable(&e), "{role} (liste) : {e:?}");
    }
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_absp_piece_lecture l JOIN ent_absp_piece p ON p.etablissement_id = l.etablissement_id AND p.id = l.piece_id WHERE l.etablissement_id = $1 AND p.absence_id = $2", abs).await,
        3,
        "un refus ne laisse aucune lecture"
    );
    // « Qui a lu mon arrêt ? » : l'agent voit les trois lectures, personne d'autre.
    let lues = x.ent.lectures_pieces_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert_eq!(lues.len(), 3);
    assert!(lues.iter().any(|l| l.lecteur_user_id == x.dir));
    let e = x.ent.lectures_pieces_absence_for_actor(x.etab, x.dir, "direction", abs).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    // Minimisation : le nom d'origine n'est ni en base ni dans le DTO.
    let colonnes: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::text FROM information_schema.columns WHERE table_name = 'ent_absp_piece'",
    )
    .fetch_all(x.ent.pool())
    .await
    .unwrap();
    assert!(!colonnes.iter().any(|c| c.contains("nom") || c.contains("filename")), "{colonnes:?}");
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("chimio") && !json.contains("martin"), "{json}");
    // Le journal ne porte que des codes.
    let h = x.ent.historique_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    let depot = h.iter().find(|j| j.action == "piece_deposee").expect("dépôt journalisé");
    assert_eq!(depot.valeur_apres.as_deref(), Some("arret"));
}

/// R-08 : un arrêt exige l'attestation « volets 2 et 3 seulement » ; le type de pièce est contrôlé ; le
/// fichier est contrôlé par sa signature binaire (D2).
#[tokio::test]
async fn a2_arret_attestation_type_et_signature_du_fichier() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let e = x
        .ent
        .deposer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, "arret", false, X::fichier(PDF))
        .await
        .unwrap_err();
    assert!(format!("{e:?}").contains("volet"), "{e:?}");
    let e = x
        .ent
        .deposer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, "diagnostic", true, X::fichier(PDF))
        .await
        .unwrap_err();
    assert!(format!("{e:?}").contains("type de pièce"), "{e:?}");
    // Un HTML, même nommé .pdf, est refusé ; aucune ligne n'est créée.
    let e = x
        .ent
        .deposer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, "justificatif", true, X::fichier(b"<html><script>1</script></html>"))
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Fichier(_)), "{e:?}");
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2", abs).await, 0);
}

/// Dépôt : l'agent lui-même ou l'instructeur ; ni un collègue, ni le reste du cercle de lecture.
#[tokio::test]
async fn a2_depot_reserve_a_l_agent_et_a_l_instructeur() {
    let x = x!();
    let abs = x.declarer(x.prof, "convocation").await;
    for (user, role) in [(x.prof2, "prof"), (x.cpe, "cpe"), (x.admin, "admin")] {
        let e = x
            .ent
            .deposer_piece_absence_for_actor(x.etab, user, role, abs, "convocation", true, X::fichier(PDF))
            .await
            .unwrap_err();
        assert!(introuvable(&e), "{role} : {e:?}");
    }
    // L'instructeur de l'État (secrétariat) peut déposer pour l'agent.
    let p = x.deposer(x.sec, "administratif", abs, "convocation").await;
    assert_eq!(p.deposee_par_user_id, x.sec);
    // Absence annulée : plus de dépôt.
    sqlx::query("UPDATE ent_absp_absence SET etat = 'annulee' WHERE etablissement_id = $1 AND id = $2")
        .bind(x.etab)
        .bind(abs)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let e = x
        .ent
        .deposer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, "convocation", true, X::fichier(PDF))
        .await
        .unwrap_err();
    assert!(est_conflit(&e) || format!("{e:?}").contains("annulée"), "{e:?}");
}

/// Retrait : l'agent avant traitement, l'instructeur ensuite ; la ligne n'est jamais supprimée ; plus de lien.
#[tokio::test]
async fn a2_retrait_d_une_piece_etat_et_plus_de_lien() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    let r = x.ent.retirer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p.id).await.unwrap();
    assert_eq!(r.etat, "retiree");
    assert_eq!(r.purge_prevue_le, Some(x.jour().await), "purge au prochain cycle");
    let e = x.ent.lien_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::FichierAbsent), "{e:?}");
    let e = x.ent.retirer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p.id).await.unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    // Après traitement, l'agent ne retire plus ; l'instructeur le peut.
    let p2 = x.deposer(x.prof, "prof", abs, "justificatif").await;
    sqlx::query("UPDATE ent_absp_absence SET etat = 'traitee' WHERE etablissement_id = $1 AND id = $2")
        .bind(x.etab)
        .bind(abs)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let e = x.ent.retirer_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p2.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    x.ent.retirer_piece_absence_for_actor(x.etab, x.sec, "administratif", abs, p2.id).await.unwrap();
    // Toujours deux lignes : rien ne s'efface.
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_piece WHERE etablissement_id = $1 AND absence_id = $2", abs).await, 2);
}

// ───────────────────────────────── Fil ─────────────────────────────────

#[tokio::test]
async fn a2_fil_en_ajout_seul_borne_et_reserve_au_cercle() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let m1 = x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, "  Je transmets l'arrêt demain.  ").await.unwrap();
    assert_eq!((m1.role_affiche.as_str(), m1.texte.as_str()), ("agent", "Je transmets l'arrêt demain."));
    let m2 = x.ent.poster_message_absence_for_actor(x.etab, x.sec, "administratif", abs, "Bien reçu.").await.unwrap();
    assert_eq!(m2.role_affiche, "instructeur");
    let m3 = x.ent.poster_message_absence_for_actor(x.etab, x.dir, "direction", abs, "Validé.").await.unwrap();
    assert!(["valideur", "direction"].contains(&m3.role_affiche.as_str()), "{}", m3.role_affiche);
    let fil = x.ent.fil_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert_eq!(fil.len(), 3);
    assert_eq!(fil[0].id, m1.id);
    // Bornes : vide et > 4 000 caractères refusés ; 4 000 exactement acceptés.
    assert!(x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, "   ").await.is_err());
    assert!(x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, &"é".repeat(4001)).await.is_err());
    x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, &"é".repeat(4000)).await.unwrap();
    // Hors cercle : introuvable, en lecture comme en écriture.
    for (user, role) in [(x.prof2, "prof"), (x.cpe, "cpe"), (x.admin, "admin"), (x.compta, "compta")] {
        assert!(introuvable(&x.ent.fil_absence_for_actor(x.etab, user, role, abs).await.unwrap_err()), "{role}");
        assert!(introuvable(&x.ent.poster_message_absence_for_actor(x.etab, user, role, abs, "x").await.unwrap_err()), "{role}");
    }
    // Ajout seul, en base : ni modification ni suppression.
    let maj = sqlx::query("UPDATE ent_absp_message SET texte = 'x' WHERE id = $1").bind(m1.id).execute(x.ent.pool()).await;
    assert!(maj.is_err(), "message modifié");
    let sup = sqlx::query("DELETE FROM ent_absp_message WHERE id = $1").bind(m1.id).execute(x.ent.pool()).await;
    assert!(sup.is_err(), "message supprimé hors purge");
}

// ───────────────────────────────── Conservation : purge, gel ─────────────────────────────────

/// Critère de fin : purge à la date (test d'horloge). Agent de l'État : 30 jours après la transmission.
#[tokio::test]
async fn a2_purge_a_la_date_agent_de_l_etat_trente_jours_apres_la_transmission() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, "Voici mon arrêt.").await.unwrap();
    x.ent.lien_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p.id).await.unwrap();
    // Pas encore transmise : aucune date, rien n'est purgé, même dans dix ans.
    assert_eq!(p.purge_prevue_le, None);
    let loin = x.jour().await + Duration::days(3650);
    let b = x.ent.purger_echeances_etab(x.etab, Some(loin)).await.unwrap();
    assert_eq!((b.dates_calculees, b.pieces_purgees), (0, 0), "{b:?}");
    // Transmission au rectorat : la date est calculée (+ 30 jours).
    x.transmettre(abs).await;
    let jour = x.jour().await;
    let b = x.ent.purger_echeances_etab(x.etab, Some(jour)).await.unwrap();
    assert_eq!((b.dates_calculees, b.pieces_purgees), (1, 0), "{b:?}");
    let pieces = x.ent.pieces_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert_eq!(pieces[0].purge_prevue_le, Some(jour + Duration::days(30)));
    // La veille de la date : rien. Le jour dit : la pièce est purgée, avec le fil.
    let b = x.ent.purger_echeances_etab(x.etab, Some(jour + Duration::days(29))).await.unwrap();
    assert_eq!((b.pieces_purgees, b.messages_supprimes), (0, 0), "{b:?}");
    let b = x.ent.purger_echeances_etab(x.etab, Some(jour + Duration::days(30))).await.unwrap();
    assert_eq!((b.pieces_purgees, b.messages_supprimes), (1, 1), "{b:?}");
    let pieces = x.ent.pieces_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert_eq!(pieces[0].etat, "purgee");
    assert!(pieces[0].purgee_le.is_some());
    assert!(x.ent.fil_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap().is_empty());
    // Plus aucun lien ; l'objet est marqué supprimé au registre ; la lecture passée reste visible.
    let e = x.ent.lien_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::FichierAbsent), "{e:?}");
    let supprimes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_objet o JOIN ent_absp_piece p ON p.etablissement_id = o.etablissement_id AND p.cle_objet = o.cle_objet
          WHERE o.etablissement_id = $1 AND p.absence_id = $2 AND o.supprime_le IS NOT NULL",
    )
    .bind(x.etab)
    .bind(abs)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(supprimes, 1);
    assert_eq!(x.ent.lectures_pieces_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap().len(), 1);
    // Journalisée ; idempotente ; irréversible.
    let h = x.ent.historique_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert!(h.iter().any(|j| j.action == "piece_purgee" && j.valeur_apres.as_deref() == Some("echeance")));
    assert!(h.iter().any(|j| j.action == "fil_purge"));
    let b = x.ent.purger_echeances_etab(x.etab, Some(jour + Duration::days(40))).await.unwrap();
    assert_eq!((b.pieces_purgees, b.messages_supprimes), (0, 0));
    let revient = sqlx::query("UPDATE ent_absp_piece SET etat = 'deposee', purgee_le = NULL, purgee_cause = NULL WHERE id = $1")
        .bind(p.id)
        .execute(x.ent.pool())
        .await;
    assert!(revient.is_err(), "une pièce purgée est revenue");
}

/// Salarié de l'OGEC : date connue dès le dépôt — fin de l'année scolaire + 3 ans.
#[tokio::test]
async fn a2_purge_a_la_date_salarie_de_l_ogec_trois_ans_apres_la_fin_de_l_annee() {
    let x = x!();
    x.en_ogec(x.prof).await;
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    let fin_annee: NaiveDate = sqlx::query_scalar(
        "SELECT date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= ent_aujourdhui_etab($1, now() + interval '3 days')
          AND date_fin >= ent_aujourdhui_etab($1, now() + interval '3 days') ORDER BY date_debut DESC LIMIT 1",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let attendue = crate::absences_personnels::pieces::ajouter_annees(fin_annee, 3);
    assert_eq!(p.purge_prevue_le, Some(attendue));
    // Pas de purge la veille, purge le jour dit — sans aucune transmission.
    let b = x.ent.purger_echeances_etab(x.etab, Some(attendue - Duration::days(1))).await.unwrap();
    assert_eq!(b.pieces_purgees, 0, "{b:?}");
    let b = x.ent.purger_echeances_etab(x.etab, Some(attendue)).await.unwrap();
    assert_eq!(b.pieces_purgees, 1, "{b:?}");
}

/// Gel (contentieux) : suspend la purge programmée et la purge à la demande ; levé, la purge reprend.
#[tokio::test]
async fn a2_un_gel_de_conservation_suspend_la_purge() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, "message").await.unwrap();
    x.transmettre(abs).await;
    let jour = x.jour().await;
    x.ent.purger_echeances_etab(x.etab, Some(jour)).await.unwrap();
    let echue = jour + Duration::days(30);
    // Droit : la direction seule (ni le secrétariat, ni l'agent, ni l'administrateur technique).
    for (user, role) in [(x.sec, "administratif"), (x.prof, "prof"), (x.admin, "admin"), (x.compta, "compta")] {
        let e = x.ent.poser_gel_absence_for_actor(x.etab, user, role, abs, "contentieux", jour, None).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{role} : {e:?}");
    }
    // Motif obligatoire ; dates cohérentes.
    assert!(x.ent.poser_gel_absence_for_actor(x.etab, x.dir, "direction", abs, " ", jour, None).await.is_err());
    assert!(x.ent.poser_gel_absence_for_actor(x.etab, x.dir, "direction", abs, "contentieux", jour, Some(jour - Duration::days(1))).await.is_err());
    let g = x.ent.poser_gel_absence_for_actor(x.etab, x.dir, "direction", abs, "Recours prud'homal en cours", jour, None).await.unwrap();
    assert_eq!(x.nb_events("absence_personnel.gel_pose").await, 1, "le DPO est averti (identifiants seulement)");
    // Purge programmée : suspendue (et comptée), fil conservé.
    let b = x.ent.purger_echeances_etab(x.etab, Some(echue + Duration::days(1))).await.unwrap();
    assert_eq!((b.pieces_purgees, b.pieces_gelees, b.messages_supprimes), (0, 1, 0), "{b:?}");
    // Purge à la demande : refusée sous gel.
    let e = x.ent.purger_piece_absence_for_actor(x.etab, x.dir, "direction", abs, p.id, "autre").await.unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    // Un gel ne se modifie ni ne se supprime ; il se lève.
    assert!(sqlx::query("UPDATE ent_absp_gel SET motif = 'autre chose' WHERE id = $1").bind(g.id).execute(x.ent.pool()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_absp_gel WHERE id = $1").bind(g.id).execute(x.ent.pool()).await.is_err());
    x.ent.lever_gel_absence_for_actor(x.etab, x.dir, "direction", abs, g.id).await.unwrap();
    assert!(x.ent.lever_gel_absence_for_actor(x.etab, x.dir, "direction", abs, g.id).await.is_err(), "levé deux fois");
    let gels = x.ent.gels_absence_for_actor(x.etab, x.dir, "direction", abs).await.unwrap();
    assert!(gels[0].leve_le.is_some());
    let b = x.ent.purger_echeances_etab(x.etab, Some(echue + Duration::days(1))).await.unwrap();
    assert_eq!((b.pieces_purgees, b.messages_supprimes), (1, 1), "{b:?}");
}

/// Purge à la demande (DPO / direction) : motif codé ; purges à venir avec la mention du gel.
#[tokio::test]
async fn a2_purge_a_la_demande_et_purges_a_venir() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    let q = x.deposer(x.prof, "prof", abs, "justificatif").await;
    let jour = x.jour().await;
    // Purges à venir : droit `conserver` ; les deux pièces (date inconnue) y figurent.
    for (user, role) in [(x.sec, "administratif"), (x.prof, "prof"), (x.admin, "admin")] {
        let e = x.ent.purges_a_venir_for_actor(x.etab, user, role, jour).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{role} : {e:?}");
    }
    let l = x.ent.purges_a_venir_for_actor(x.etab, x.dir, "direction", jour).await.unwrap();
    assert_eq!(l.len(), 2);
    assert!(l.iter().all(|e| !e.gelee));
    // Motif inconnu refusé (jamais un texte libre) ; droit exigé.
    assert!(x.ent.purger_piece_absence_for_actor(x.etab, x.dir, "direction", abs, p.id, "parce que").await.is_err());
    let e = x.ent.purger_piece_absence_for_actor(x.etab, x.sec, "administratif", abs, p.id, "autre").await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let r = x.ent.purger_piece_absence_for_actor(x.etab, x.dir, "direction", abs, p.id, "demande_agent").await.unwrap();
    assert_eq!(r.etat, "purgee");
    let e = x.ent.purger_piece_absence_for_actor(x.etab, x.dir, "direction", abs, p.id, "autre").await.unwrap_err();
    assert!(est_conflit(&e), "déjà purgée : {e:?}");
    // L'autre pièce reste lisible.
    x.ent.lien_piece_absence_for_actor(x.etab, x.prof, "prof", abs, q.id).await.unwrap();
    let h = x.ent.historique_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert!(h.iter().any(|j| j.action == "purge_demandee" && j.valeur_apres.as_deref() == Some("demande_agent")));
    // Sous gel, la liste le signale.
    x.ent.poser_gel_absence_for_actor(x.etab, x.dir, "direction", abs, "contentieux", jour, None).await.unwrap();
    let l = x.ent.purges_a_venir_for_actor(x.etab, x.dir, "direction", jour).await.unwrap();
    assert_eq!(l.len(), 1);
    assert!(l[0].gelee);
}

// ───────────────────────────────── Relance, échéances ─────────────────────────────────

/// ABP-141 : relance de l'instructeur, plafonnée (S-1502) ; plus de relance une fois la pièce déposée.
#[tokio::test]
async fn a2_relance_de_la_piece_plafonnee_et_reservee_a_l_instructeur() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    for (user, role) in [(x.prof, "prof"), (x.prof2, "prof"), (x.cpe, "cpe")] {
        let e = x.ent.relancer_piece_absence_for_actor(x.etab, user, role, abs).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden) || introuvable(&e), "{role} : {e:?}");
    }
    for n in 1..=3u32 {
        assert_eq!(x.ent.relancer_piece_absence_for_actor(x.etab, x.sec, "administratif", abs).await.unwrap(), n);
    }
    let e = x.ent.relancer_piece_absence_for_actor(x.etab, x.sec, "administratif", abs).await.unwrap_err();
    assert!(est_conflit(&e), "plafond : {e:?}");
    assert_eq!(x.nb_events("absence_personnel.piece_relancee").await, 3);
    // Une nature sans pièce attendue ne se relance pas ; une pièce déposée non plus.
    let abs2 = x.declarer(x.prof2, "maladie").await;
    x.deposer(x.prof2, "prof", abs2, "arret").await;
    let e = x.ent.relancer_piece_absence_for_actor(x.etab, x.sec, "administratif", abs2).await.unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
}

/// ABP-147 : les échéances de l'instructeur, cloisonnées par employeur (secrétariat : État ; comptabilité : OGEC).
#[tokio::test]
async fn a2_echeances_de_l_instructeur_cloisonnees_par_employeur() {
    let x = x!();
    let etat = x.declarer(x.prof, "maladie").await;
    x.en_ogec(x.prof2).await;
    let ogec = x.declarer(x.prof2, "maladie").await;
    let echeances = |user: Uuid, role: &'static str| x.ent.echeances_absences_for_actor(x.etab, user, role);
    let sec = echeances(x.sec, "administratif").await.unwrap();
    assert!(sec.iter().any(|e| e.absence_id == etat && e.genre == "piece_manquante"), "{sec:?}");
    assert!(sec.iter().all(|e| e.absence_id != ogec), "le secrétariat ne voit pas l'OGEC");
    let cpt = echeances(x.compta, "compta").await.unwrap();
    assert!(cpt.iter().any(|e| e.absence_id == ogec), "{cpt:?}");
    assert!(cpt.iter().all(|e| e.absence_id != etat), "la comptabilité ne voit pas l'État");
    // Pas instructeur : refusé.
    for (user, role) in [(x.prof, "prof"), (x.cpe, "cpe")] {
        assert!(matches!(echeances(user, role).await.unwrap_err(), EntDbError::Forbidden), "{role}");
    }
    // Une fois la pièce déposée, l'échéance disparaît.
    x.deposer(x.prof, "prof", etat, "arret").await;
    let sec = echeances(x.sec, "administratif").await.unwrap();
    assert!(sec.iter().all(|e| e.absence_id != etat));
}

// ───────────────────────────────── Confidentialité renforcée ─────────────────────────────────

/// ABP-164 : l'absence marquée disparaît de la vue d'organisation et de la liste historique pour la vie
/// scolaire ; la direction et l'agent la voient ; réversible, journalisé, droit `conserver`.
#[tokio::test]
async fn a2_confidentialite_renforcee_reduit_la_vue_d_organisation() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    x.ent
        .valider_absence_for_actor(x.etab, x.dir, "direction", abs, &DecisionBody::default())
        .await
        .unwrap();
    let du = today_utc();
    let au = du + Duration::days(10);
    let (ent, etab) = (&x.ent, x.etab);
    let org = move |user: Uuid, role: &'static str| async move {
        ent.absences_organisation_for_actor(etab, user, role, du, au).await.unwrap()
    };
    assert!(org(x.cpe, "cpe").await.iter().any(|a| a.id == abs), "visible avant le marquage");
    for (user, role) in [(x.cpe, "cpe"), (x.sec, "administratif"), (x.prof2, "prof")] {
        let e = x.ent.definir_confidentialite_renforcee_for_actor(x.etab, user, role, abs, true).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{role} : {e:?}");
    }
    x.ent.definir_confidentialite_renforcee_for_actor(x.etab, x.dir, "direction", abs, true).await.unwrap();
    assert!(org(x.cpe, "cpe").await.iter().all(|a| a.id != abs), "la vie scolaire ne voit plus l'agent");
    assert!(org(x.dir, "direction").await.iter().any(|a| a.id == abs), "la direction la voit");
    // Liste historique (écran des remplacements) : même règle.
    let jour = today_utc();
    let hist = move |user: Uuid, role: &'static str| async move {
        ent.list_absences_rh_for_actor(etab, user, role, jour, jour + Duration::days(10)).await.unwrap()
    };
    assert!(hist(x.cpe, "cpe").await.iter().all(|a| a.id != abs));
    assert!(hist(x.dir, "direction").await.iter().any(|a| a.id == abs));
    assert!(hist(x.prof, "prof").await.iter().any(|a| a.id == abs), "l'agent voit sa propre absence");
    // Réversible, et journalisé.
    x.ent.definir_confidentialite_renforcee_for_actor(x.etab, x.dir, "direction", abs, false).await.unwrap();
    assert!(org(x.cpe, "cpe").await.iter().any(|a| a.id == abs));
    let h = x.ent.historique_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert!(h.iter().any(|j| j.action == "confidentialite_renforcee"));
    assert!(h.iter().any(|j| j.action == "confidentialite_normale"));
}

// ───────────────────────────────── Garde de la base, cloisonnement ─────────────────────────────────

/// Une pièce ne se supprime pas et ses champs d'origine sont figés ; les lectures sont en ajout seul ; la
/// suppression du fil n'est permise qu'à la purge de conservation.
#[tokio::test]
async fn a2_gardes_de_la_base_pieces_lectures_et_fil() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    x.ent.lien_piece_absence_for_actor(x.etab, x.prof, "prof", abs, p.id).await.unwrap();
    let m = x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, "bonjour").await.unwrap();
    assert!(sqlx::query("DELETE FROM ent_absp_piece WHERE id = $1").bind(p.id).execute(x.ent.pool()).await.is_err());
    assert!(sqlx::query("UPDATE ent_absp_piece SET type_piece = 'autre' WHERE id = $1").bind(p.id).execute(x.ent.pool()).await.is_err());
    assert!(sqlx::query("UPDATE ent_absp_piece SET cle_objet = etablissement_id::text || '/x/y.pdf' WHERE id = $1").bind(p.id).execute(x.ent.pool()).await.is_err());
    assert!(sqlx::query("UPDATE ent_absp_piece_lecture SET lecteur_user_id = $1 WHERE piece_id = $2").bind(x.dir).bind(p.id).execute(x.ent.pool()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_absp_piece_lecture WHERE piece_id = $1").bind(p.id).execute(x.ent.pool()).await.is_err());
    // Clé d'objet d'un autre établissement refusée (préfixe obligatoire).
    let autre = Uuid::new_v4();
    let e = sqlx::query(
        "INSERT INTO ent_absp_piece (id, etablissement_id, absence_id, type_piece, cle_objet, sha256, taille_octets, mime, deposee_par_user_id)
         VALUES ($1,$2,$3,'autre',$4,$5,10,'application/pdf',$6)",
    )
    .bind(Uuid::new_v4())
    .bind(x.etab)
    .bind(abs)
    .bind(format!("{autre}/absence_personnel/z.pdf"))
    .bind("0".repeat(64))
    .bind(x.prof)
    .execute(x.ent.pool())
    .await;
    assert!(e.is_err(), "clé d'un autre établissement acceptée");
    // Suppression du fil : seulement sous le réglage de purge, dans la transaction de la purge.
    let mut tx = x.ent.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.absp_purge', 'on', true)").execute(&mut *tx).await.unwrap();
    let n = sqlx::query("DELETE FROM ent_absp_message WHERE id = $1").bind(m.id).execute(&mut *tx).await.unwrap().rows_affected();
    assert_eq!(n, 1);
    tx.rollback().await.unwrap();
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_message WHERE etablissement_id = $1 AND absence_id = $2", abs).await, 1);
}

/// Cloisonnement d'établissement : la pièce, le fil et les gels d'un établissement sont invisibles d'un autre,
/// même pour la direction (R-AP42, D-013).
#[tokio::test]
async fn a2_cloisonnement_d_etablissement() {
    let x = x!();
    let abs = x.declarer(x.prof, "maladie").await;
    let p = x.deposer(x.prof, "prof", abs, "arret").await;
    x.ent.poster_message_absence_for_actor(x.etab, x.prof, "prof", abs, "bonjour").await.unwrap();
    let autre = Uuid::new_v4();
    let dir_b = compte(&x.ent, autre, "direction").await;
    for r in [
        x.ent.pieces_absence_for_actor(autre, dir_b, "direction", abs).await.map(|_| ()),
        x.ent.fil_absence_for_actor(autre, dir_b, "direction", abs).await.map(|_| ()),
        x.ent.lien_piece_absence_for_actor(autre, dir_b, "direction", abs, p.id).await.map(|_| ()),
        x.ent.poster_message_absence_for_actor(autre, dir_b, "direction", abs, "intrus").await.map(|_| ()),
        x.ent.poser_gel_absence_for_actor(autre, dir_b, "direction", abs, "intrus", today_utc(), None).await.map(|_| ()),
        x.ent.purger_piece_absence_for_actor(autre, dir_b, "direction", abs, p.id, "autre").await.map(|_| ()),
        x.ent.definir_confidentialite_renforcee_for_actor(autre, dir_b, "direction", abs, true).await,
    ] {
        let e = r.unwrap_err();
        assert!(introuvable(&e), "{e:?}");
    }
    // Les purges d'un autre établissement ne touchent pas celui-ci.
    let b = x.ent.purger_echeances_etab(autre, Some(today_utc() + Duration::days(4000))).await.unwrap();
    assert_eq!(b.pieces_purgees, 0);
    let pieces = x.ent.pieces_absence_for_actor(x.etab, x.prof, "prof", abs).await.unwrap();
    assert_eq!(pieces[0].etat, "deposee");
}
