//! Socle 29 (portails), lot P1 : la page « Aujourd'hui » de chaque profil, composée côté serveur
//! à partir des ports **existants** (`docs/socles/29_PORTAILS.md` §9, §24 lot P1 ; D-034).
//!
//! * **Aucune donnée propre, aucun index** : lectures directes des services propriétaires, qui
//!   appliquent eux-mêmes leurs droits (lien famille, moteur D-017, cloisonnement établissement,
//!   `TenantTx`). Le portail ne contourne jamais un contrôle : un service qui répond
//!   `Forbidden` fait **disparaître** la tuile (DP-06), il ne l'affiche pas en erreur.
//! * **Une source pas encore codée n'a pas de tuile** ; une source en erreur ou hors délai donne
//!   une tuile `indisponible` sans faire tomber la page (R-PO12).
//! * **Minimisation** (§15.3) : titres neutres, compteurs plutôt que listes de noms, jamais de
//!   motif ni de note (les notes restent « sur geste », DP-03) ; un responsable sans exercice de
//!   l'autorité parentale ne reçoit aucune tâche de décision (R-PO8).
//! * Le jour vient de l'appelant (`maintenant`, heure de l'établissement) : aucune horloge ici.

use crate::service::{EntDbError, PgEnt};
use chrono::{NaiveDate, NaiveDateTime};
use domain_eleve::EdtExceptionKind;
use ent_auth::{Action, Role};
use ent_types::portail::{
    ActionPortail, EtatTuile, LignePortail, PortailAujourdhui, ProfilPortail, TonLigne,
    TuilePortail,
};
use std::future::Future;
use std::time::Duration;
use uuid::Uuid;

/// Budget de temps d'une source (R-PO12) : au-delà, la tuile est « indisponible ».
const BUDGET_SOURCE: Duration = Duration::from_millis(2500);
/// Nombre de lignes affichées par tuile (la suite est derrière le lien).
const MAX_LIGNES: usize = 5;

/// Résultat d'une source : `Ok(None)` = pas de tuile (rien à proposer à ce profil).
type Source = Result<Option<TuilePortail>, EntDbError>;

/// Applique le budget de temps et traduit les erreurs : `Forbidden` → tuile absente (jamais
/// une tuile d'erreur 403), autre erreur ou délai → tuile « indisponible ».
async fn avec_budget<F>(code: &str, titre: &str, largeur: u8, f: F) -> Option<TuilePortail>
where
    F: Future<Output = Source>,
{
    match tokio::time::timeout(BUDGET_SOURCE, f).await {
        Ok(Ok(t)) => t,
        Ok(Err(EntDbError::Forbidden)) => None,
        Ok(Err(_)) | Err(_) => Some(TuilePortail::indisponible(code, titre, largeur)),
    }
}

fn pluriel(n: i64, singulier: &str, pluriel: &str) -> String {
    if n > 1 {
        format!("{n} {pluriel}")
    } else {
        format!("{n} {singulier}")
    }
}

fn hhmm(h: chrono::NaiveTime) -> String {
    h.format("%H:%M").to_string()
}

fn action(libelle: &str, lien: &str) -> Option<ActionPortail> {
    Some(ActionPortail { libelle: libelle.to_string(), lien: lien.to_string() })
}

/// Tuile « chiffre » : un compteur actionnable, vide si zéro.
fn tuile_compteur(
    code: &str,
    titre: &str,
    largeur: u8,
    n: i64,
    (un, plusieurs): (&str, &str),
    vide: &str,
    act: Option<ActionPortail>,
) -> TuilePortail {
    let mut t = TuilePortail::nouvelle(code, titre, largeur);
    t.nombre = Some(n);
    if n > 0 {
        t.etat = EtatTuile::Ok;
        t.principal = Some(pluriel(n, un, plusieurs));
        t.action = act;
    } else {
        t.principal = Some(vide.to_string());
    }
    t
}

/// Profil d'accueil d'un rôle de base.
pub fn profil_du_role(role: Role) -> ProfilPortail {
    match role {
        Role::Parent => ProfilPortail::Famille,
        Role::Eleve => ProfilPortail::Eleve,
        Role::Prof => ProfilPortail::Professeur,
        Role::Cpe => ProfilPortail::VieScolaire,
        Role::Accueil => ProfilPortail::Accueil,
        Role::Administratif => ProfilPortail::Secretariat,
        Role::Direction | Role::Admin => ProfilPortail::Direction,
        Role::Compta => ProfilPortail::Comptabilite,
        Role::Infirmerie | Role::Psy | Role::Personnel => ProfilPortail::Personnel,
    }
}

impl PgEnt {
    /// `GET /portail/aujourdhui` : la page d'accueil de l'acteur, sous ses droits.
    /// `maintenant` : heure locale de l'établissement ([`PgEnt::maintenant_local`]).
    pub async fn portail_aujourdhui(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        maintenant: NaiveDateTime,
    ) -> Result<PortailAujourdhui, EntDbError> {
        let base = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let profil = profil_du_role(base);
        let jour = maintenant.date();
        let (e, u, r) = (etablissement_id, user_id, role);

        let tuiles: Vec<TuilePortail> = match profil {
            ProfilPortail::Famille => {
                let (enfants, a_faire, messages, annonces, notes, sorties) = tokio::join!(
                    avec_budget("edt.enfants", "Aujourd'hui", 8, self.tuile_famille_edt(e, u, r, jour)),
                    avec_budget("afaire.famille", "À faire", 4, self.tuile_famille_a_faire(e, u, r)),
                    avec_budget("messagerie.non_lus", "Messages", 4, self.tuile_messages(e, u, r)),
                    avec_budget("annonces.une", "À la une", 8, self.tuile_annonces_famille(e, u, r)),
                    avec_budget("notes.geste", "Dernières notes", 4, self.tuile_famille_notes(e, u, r)),
                    avec_budget("sorties.famille", "Sortie à venir", 4, self.tuile_sorties(e, u, r)),
                );
                [a_faire, enfants, messages, annonces, sorties, notes].into_iter().flatten().collect()
            }
            ProfilPortail::Eleve => {
                let (edt, messages, annonces, notes, sorties) = tokio::join!(
                    avec_budget("edt.journee", "Aujourd'hui", 8, self.tuile_eleve_edt(e, u, r, jour)),
                    avec_budget("messagerie.non_lus", "Messages", 4, self.tuile_messages(e, u, r)),
                    avec_budget("annonces.une", "À la une", 8, self.tuile_annonces_eleve(e, u, r)),
                    avec_budget("notes.geste", "Mes notes", 4, self.tuile_eleve_notes(e, u, r)),
                    avec_budget("sorties.famille", "Sortie à venir", 4, self.tuile_sorties(e, u, r)),
                );
                [edt, messages, annonces, notes, sorties].into_iter().flatten().collect()
            }
            _ => {
                // Personnels : chaque tuile est conditionnée par son droit (D-017), pas par le
                // rôle fixe (DP-04) ; un service qui refuse retire la tuile.
                let (journee, vs, accueil, inscriptions, propositions, protections, rh, messages, annonces) = tokio::join!(
                    avec_budget("edt.journee", "Ma journée", 8, self.tuile_prof_journee(e, u, r, jour)),
                    avec_budget("vs.jour", "Vie scolaire du jour", 8, self.tuile_vs_jour(e, u, r, maintenant)),
                    avec_budget("accueil.absence", "Accueil", 4, self.tuile_accueil(e, u, r)),
                    avec_budget("inscriptions.instruire", "Inscriptions à instruire", 4, self.tuile_inscriptions(e, u, r)),
                    avec_budget("eleves.propositions", "Coordonnées à valider", 4, self.tuile_propositions(e, u, r)),
                    avec_budget("direction.protections", "Protections à confirmer", 4, self.tuile_protections(e, u, r)),
                    avec_budget("personnels.absences", "Personnels absents", 4, self.tuile_absences_personnels(e, u, r, jour, base)),
                    avec_budget("messagerie.non_lus", "Messages", 4, self.tuile_messages(e, u, r)),
                    avec_budget("annonces.une", "À la une", 8, self.tuile_annonces_personnel(e, r)),
                );
                [vs, journee, accueil, inscriptions, propositions, protections, rh, messages, annonces]
                    .into_iter()
                    .flatten()
                    .collect()
            }
        };
        Ok(PortailAujourdhui { profil, jour, prenom: None, tuiles })
    }

    // ---------------------------------------------------------------------------------------
    // Transverse : messages, annonces, sorties
    // ---------------------------------------------------------------------------------------

    /// Messages non lus : **compte** de fils non lus, jamais d'objet ni d'aperçu.
    async fn tuile_messages(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let fils = self.list_fils_for_actor(e, u, r, None, false, None).await?;
        let ids: Vec<Uuid> = fils.iter().map(|f| f.id).collect();
        let non_lus = if ids.is_empty() {
            0
        } else {
            self.count_msg_non_lus_par_fils(e, u, &ids)
                .await?
                .values()
                .filter(|n| **n > 0)
                .count() as i64
        };
        Ok(Some(tuile_compteur(
            "messagerie.non_lus",
            "Messages",
            4,
            non_lus,
            ("conversation non lue", "conversations non lues"),
            "Aucun message non lu",
            action("Ouvrir la messagerie", "/web/messages"),
        )))
    }

    fn lignes_annonces(list: Vec<domain_eleve::Annonce>) -> TuilePortail {
        let mut t = TuilePortail::nouvelle("annonces.une", "À la une", 8);
        if list.is_empty() {
            t.principal = Some("Aucune annonce pour le moment".into());
            return t;
        }
        t.etat = EtatTuile::Ok;
        t.lignes = list
            .into_iter()
            .take(3)
            .map(|a| LignePortail {
                libelle: a.titre,
                detail: Some(a.created_at.format("%d/%m/%Y").to_string()),
                ton: TonLigne::Neutre,
                lien: None,
            })
            .collect();
        t
    }

    async fn tuile_annonces_personnel(&self, e: Uuid, r: &str) -> Source {
        let list = self.list_annonces_for_actor(e, r, 3).await?;
        Ok(Some(Self::lignes_annonces(list)))
    }

    /// DP-01 : la famille reçoit les annonces par la route famille, **par enfant**, dédoublonnées.
    async fn tuile_annonces_famille(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let enfants = self.mes_enfants_for_parent(e, u, r).await?;
        let mut vues: Vec<domain_eleve::Annonce> = Vec::new();
        for enf in &enfants {
            for a in self.list_annonces_famille_for_actor(e, u, r, enf.eleve.id).await? {
                if !vues.iter().any(|v| v.id == a.id) {
                    vues.push(a);
                }
            }
        }
        vues.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(Some(Self::lignes_annonces(vues)))
    }

    async fn tuile_annonces_eleve(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let eleve = self.eleve_id_for_compte(e, u).await?.ok_or(EntDbError::Forbidden)?;
        let mut list = self.list_annonces_famille_for_actor(e, u, r, eleve).await?;
        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(Some(Self::lignes_annonces(list)))
    }

    /// Prochaine sortie où mon enfant (ou moi) est inscrit : projection déjà minimale du socle 42.
    async fn tuile_sorties(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let acteur = crate::ref_commun::Acteur { etablissement_id: e, user_id: u, role: r };
        let v = self.voyages_famille(&acteur).await?;
        let mut t = TuilePortail::nouvelle("sorties.famille", "Sortie à venir", 4);
        match v.first() {
            None => t.principal = Some("Aucune sortie annoncée".into()),
            Some(s) => {
                t.etat = EtatTuile::Ok;
                t.principal = Some(s.titre.clone());
                t.lignes.push(LignePortail {
                    libelle: format!("{} · du {}", s.enfant_prenom, s.debut.format("%d/%m")),
                    detail: s.point_rendez_vous.clone(),
                    ton: TonLigne::Neutre,
                    lien: None,
                });
                t.action = action("Voir la sortie", "/web/voyages");
            }
        }
        Ok(Some(t))
    }

    // ---------------------------------------------------------------------------------------
    // Famille
    // ---------------------------------------------------------------------------------------

    /// « Aujourd'hui de {enfant} » : une ligne de tête par enfant, ses cours du jour avec les
    /// changements (annulé, remplacé, salle). Aucun motif de l'exception n'est transmis.
    async fn tuile_famille_edt(&self, e: Uuid, u: Uuid, r: &str, jour: NaiveDate) -> Source {
        let enfants = self.mes_enfants_for_parent(e, u, r).await?;
        let mut t = TuilePortail::nouvelle("edt.enfants", "Aujourd'hui", 8);
        if enfants.is_empty() {
            t.principal = Some("Aucun enfant n'est rattaché à votre compte".into());
            return Ok(Some(t));
        }
        let exceptions = self.edt_repo().list_exceptions_jour(e, jour).await.map_err(EntDbError::from)?;
        let plusieurs = enfants.len() > 1;
        for enf in &enfants {
            let creneaux = self.list_edt_eleve_jour_for_actor(e, r, u, enf.eleve.id, jour).await?;
            let prenom = enf.eleve.prenom.clone();
            if creneaux.is_empty() {
                t.lignes.push(LignePortail {
                    libelle: if plusieurs { format!("{prenom} · pas de cours aujourd'hui") } else { "Pas de cours aujourd'hui".into() },
                    detail: None,
                    ton: TonLigne::Neutre,
                    lien: Some("/web/emploi-du-temps".into()),
                });
                continue;
            }
            for c in creneaux.iter().take(MAX_LIGNES) {
                let ex = exceptions.iter().find(|x| x.creneau_id == c.id);
                let (ton, mention) = match ex.map(|x| x.kind) {
                    Some(EdtExceptionKind::Annulation) => (TonLigne::Alerte, Some("annulé".to_string())),
                    Some(EdtExceptionKind::Remplacement) => (TonLigne::Attention, Some("remplacé".to_string())),
                    Some(EdtExceptionKind::ChangementSalle) => (
                        TonLigne::Attention,
                        Some(format!(
                            "salle {}",
                            ex.and_then(|x| x.nouvelle_salle.clone()).unwrap_or_default()
                        )),
                    ),
                    _ => (TonLigne::Neutre, None),
                };
                let salle = c.salle.as_ref().filter(|s| !s.is_empty()).map(|s| format!("salle {s}"));
                let detail = mention.or(salle);
                let libelle = if plusieurs {
                    format!("{} {} · {} ({prenom})", hhmm(c.heure_debut), c.matiere_libelle, c.classe)
                } else {
                    format!("{} {}", hhmm(c.heure_debut), c.matiere_libelle)
                };
                t.lignes.push(LignePortail { libelle, detail, ton, lien: Some("/web/emploi-du-temps".into()) });
            }
        }
        t.etat = EtatTuile::Ok;
        t.action = action("Emploi du temps", "/web/emploi-du-temps");
        Ok(Some(t))
    }

    /// « À faire » famille : **comptes** par type (justifier, signer), seulement pour les enfants
    /// où le lien le permet. Sans exercice de l'autorité parentale (`peut_justifier_absence`
    /// faux) : aucune tâche de décision, mention sobre (R-PO8).
    async fn tuile_famille_a_faire(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let enfants = self.mes_enfants_for_parent(e, u, r).await?;
        let mut t = TuilePortail::nouvelle("afaire.famille", "À faire", 4);
        let (mut a_justifier, mut carnets) = (0_i64, 0_i64);
        let mut sans_decision = false;
        for enf in &enfants {
            if enf.peut_justifier_absence {
                a_justifier += self.absences_a_justifier_eleve(e, enf.eleve.id).await?.len() as i64;
            } else {
                sans_decision = true;
            }
            if enf.peut_voir_vs && enf.peut_justifier_absence {
                carnets += self.carnet_a_signer_eleve(e, enf.eleve.id).await?.len() as i64;
            }
        }
        let total = a_justifier + carnets;
        t.nombre = Some(total);
        if a_justifier > 0 {
            t.lignes.push(LignePortail {
                libelle: pluriel(a_justifier, "absence à justifier", "absences à justifier"),
                detail: None,
                ton: TonLigne::Attention,
                lien: Some("/web/absences".into()),
            });
        }
        if carnets > 0 {
            t.lignes.push(LignePortail {
                libelle: pluriel(carnets, "mot du carnet à signer", "mots du carnet à signer"),
                detail: None,
                ton: TonLigne::Attention,
                lien: Some("/web/cahier".into()),
            });
        }
        if total > 0 {
            t.etat = EtatTuile::Ok;
            t.principal = Some(pluriel(total, "chose à faire", "choses à faire"));
            t.action = action("Voir et traiter", "/web/absences");
        } else {
            t.principal = Some("Rien à faire pour l'instant".into());
        }
        if sans_decision && total == 0 {
            t.lignes.push(LignePortail::simple(
                "Les décisions sont prises par le ou les titulaires de l'autorité parentale.",
            ));
        }
        Ok(Some(t))
    }

    /// Notes : **jamais affichées d'emblée** (DP-03, R-PO13) ; la tuile n'existe que si un lien
    /// permet de les voir, et n'embarque aucune note.
    async fn tuile_famille_notes(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let enfants = self.mes_enfants_for_parent(e, u, r).await?;
        if !enfants.iter().any(|x| x.peut_voir_notes) {
            return Ok(None);
        }
        let mut t = TuilePortail::nouvelle("notes.geste", "Dernières notes", 4);
        t.etat = EtatTuile::Ok;
        t.sur_geste = true;
        t.principal = Some("Les notes s'affichent à votre demande.".into());
        t.action = action("Afficher les notes", "/web/evaluations");
        Ok(Some(t))
    }

    // ---------------------------------------------------------------------------------------
    // Élève
    // ---------------------------------------------------------------------------------------

    async fn tuile_eleve_edt(&self, e: Uuid, u: Uuid, r: &str, jour: NaiveDate) -> Source {
        let creneaux = self.list_mes_creneaux(e, u, r, jour).await?;
        let mut t = TuilePortail::nouvelle("edt.journee", "Aujourd'hui", 8);
        if creneaux.is_empty() {
            t.principal = Some("Pas de cours aujourd'hui".into());
        } else {
            t.etat = EtatTuile::Ok;
            t.principal = Some(pluriel(creneaux.len() as i64, "cours aujourd'hui", "cours aujourd'hui"));
            t.lignes = creneaux
                .iter()
                .take(8)
                .map(|c| LignePortail {
                    libelle: format!("{} {}", hhmm(c.heure_debut), c.matiere_libelle),
                    detail: c.salle.as_ref().filter(|s| !s.is_empty()).map(|s| format!("salle {s}")),
                    ton: TonLigne::Neutre,
                    lien: Some("/web/emploi-du-temps".into()),
                })
                .collect();
        }
        t.action = action("Emploi du temps", "/web/emploi-du-temps");
        Ok(Some(t))
    }

    async fn tuile_eleve_notes(&self, e: Uuid, u: Uuid, _r: &str) -> Source {
        self.eleve_id_for_compte(e, u).await?.ok_or(EntDbError::Forbidden)?;
        let mut t = TuilePortail::nouvelle("notes.geste", "Mes notes", 4);
        t.etat = EtatTuile::Ok;
        t.sur_geste = true;
        t.principal = Some("Tes notes s'affichent à ta demande.".into());
        t.action = action("Afficher mes notes", "/web/evaluations");
        Ok(Some(t))
    }

    // ---------------------------------------------------------------------------------------
    // Personnels
    // ---------------------------------------------------------------------------------------

    /// « Ma journée » du professeur : séances du jour avec l'état de l'appel (fait / à faire).
    async fn tuile_prof_journee(&self, e: Uuid, u: Uuid, r: &str, jour: NaiveDate) -> Source {
        if Role::parse(r) != Some(Role::Prof) {
            return Ok(None);
        }
        let creneaux = self.list_mes_creneaux(e, u, r, jour).await?;
        let etats = self.etat_appels_du_jour(e, jour).await?;
        let mut t = TuilePortail::nouvelle("edt.journee", "Ma journée", 8);
        if creneaux.is_empty() {
            t.principal = Some("Pas de cours aujourd'hui".into());
            t.action = action("Emploi du temps", "/web/emploi-du-temps");
            return Ok(Some(t));
        }
        let a_faire = creneaux.iter().filter(|c| !etats.iter().any(|(id, s)| id == &c.id && s == "clos")).count() as i64;
        t.etat = EtatTuile::Ok;
        t.nombre = Some(a_faire);
        t.principal = Some(if a_faire == 0 {
            "Tous vos appels sont faits".to_string()
        } else {
            pluriel(a_faire, "appel à faire", "appels à faire")
        });
        t.lignes = creneaux
            .iter()
            .map(|c| {
                let fait = etats.iter().any(|(id, s)| id == &c.id && s == "clos");
                LignePortail {
                    libelle: format!("{} {} · {}", hhmm(c.heure_debut), c.classe, c.matiere_libelle),
                    detail: Some(format!(
                        "{}{}",
                        if fait { "appel fait" } else { "appel à faire" },
                        c.salle.as_ref().filter(|s| !s.is_empty()).map(|s| format!(" · salle {s}")).unwrap_or_default()
                    )),
                    ton: if fait { TonLigne::Neutre } else { TonLigne::Attention },
                    lien: Some(format!("/web/appel?jour={jour}&creneau={}", c.id)),
                }
            })
            .collect();
        t.action = action("Emploi du temps", "/web/emploi-du-temps");
        Ok(Some(t))
    }

    /// Appels du jour de l'établissement : `(créneau, statut)`. Lecture sous `TenantTx` ; le
    /// professeur ne reçoit que l'état des créneaux de **sa** liste (filtré par l'appelant).
    async fn etat_appels_du_jour(&self, e: Uuid, jour: NaiveDate) -> Result<Vec<(Uuid, String)>, EntDbError> {
        let mut tx = crate::TenantTx::begin(self.pool(), e).await?;
        let l: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT creneau_id, statut FROM ent_appel
              WHERE etablissement_id = $1 AND jour = $2 AND creneau_id IS NOT NULL",
        )
        .bind(e)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(crate::vie_scolaire::db)?;
        Ok(l)
    }

    /// Vie scolaire : absents du jour, justificatifs à trancher, retards, appels non faits.
    /// Compteurs et séances (classe, matière, heure) : aucun nom d'élève sur l'accueil.
    async fn tuile_vs_jour(&self, e: Uuid, u: Uuid, r: &str, maintenant: NaiveDateTime) -> Source {
        let tj = self.tableau_du_jour_for_actor(e, u, r, maintenant.date(), maintenant).await?;
        let mut t = TuilePortail::nouvelle("vs.jour", "Vie scolaire du jour", 8);
        let sans_appel = tj.verification.seances_sans_appel.len() as i64;
        let absents = tj.absents.len() as i64;
        t.etat = EtatTuile::Ok;
        t.nombre = Some(sans_appel + tj.a_traiter);
        t.principal = Some(format!("{} aujourd'hui", pluriel(absents, "élève absent", "élèves absents")));
        t.lignes.push(LignePortail {
            libelle: pluriel(tj.a_traiter, "justificatif à traiter", "justificatifs à traiter"),
            detail: None,
            ton: if tj.a_traiter > 0 { TonLigne::Attention } else { TonLigne::Neutre },
            lien: Some("/web/journee".into()),
        });
        t.lignes.push(LignePortail {
            libelle: pluriel(tj.retards.len() as i64, "retard", "retards"),
            detail: None,
            ton: TonLigne::Neutre,
            lien: Some("/web/journee".into()),
        });
        t.lignes.push(LignePortail {
            libelle: if sans_appel == 0 { "Tous les appels sont faits".into() } else { pluriel(sans_appel, "appel non fait", "appels non faits") },
            detail: None,
            ton: if sans_appel > 0 { TonLigne::Alerte } else { TonLigne::Neutre },
            lien: Some("/web/journee".into()),
        });
        for s in tj.verification.seances_sans_appel.iter().take(MAX_LIGNES) {
            t.lignes.push(LignePortail {
                libelle: format!("{} {} · {}", hhmm(s.heure_debut), s.classe, s.matiere),
                detail: Some(if s.etat == "non_clos" { "appel non clos".into() } else { "appel non fait".into() }),
                ton: TonLigne::Alerte,
                lien: Some("/web/journee".into()),
            });
        }
        t.action = action("Ouvrir la vie scolaire", "/web/journee");
        Ok(Some(t))
    }

    /// Accueil : « Déclarer une absence » (appel d'un parent). Conditionné par le droit D-017.
    async fn tuile_accueil(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        if !self.a_action_etablissement(e, u, r, Action::AbsencesDeclarer).await? {
            return Ok(None);
        }
        let mut t = TuilePortail::nouvelle("accueil.absence", "Accueil", 4);
        t.etat = EtatTuile::Ok;
        t.principal = Some("Un parent appelle pour une absence ?".into());
        t.action = action("Déclarer une absence", "/web/journee");
        Ok(Some(t))
    }

    /// Secrétariat : dossiers d'inscription à instruire (déposés, en instruction, incomplets).
    async fn tuile_inscriptions(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        if Role::parse(r).is_some_and(|b| matches!(b, Role::Parent | Role::Eleve)) {
            return Ok(None);
        }
        let dossiers = self.list_dossiers_for_actor(e, u, r, None, None).await?;
        let n = dossiers
            .iter()
            .filter(|d| matches!(d.statut.as_str(), "deposee" | "en_instruction" | "incomplet"))
            .count() as i64;
        Ok(Some(tuile_compteur(
            "inscriptions.instruire",
            "Inscriptions à instruire",
            4,
            n,
            ("dossier à instruire", "dossiers à instruire"),
            "Aucun dossier en attente",
            action("Ouvrir les inscriptions", "/web/inscriptions"),
        )))
    }

    /// Secrétariat : propositions de changement de coordonnées en attente de décision.
    async fn tuile_propositions(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let l = self
            .propositions_pour_acteur(r, e, u, crate::propositions::FiltrePropositions::default())
            .await?;
        Ok(Some(tuile_compteur(
            "eleves.propositions",
            "Coordonnées à valider",
            4,
            l.len() as i64,
            ("proposition à traiter", "propositions à traiter"),
            "Aucune proposition en attente",
            action("Voir les propositions", "/web/eleves"),
        )))
    }

    /// Direction : protections à confirmer (adresses parquées, « ne pas solliciter »). Comptes
    /// seulement : l'identité des personnes protégées n'est jamais sur l'accueil.
    async fn tuile_protections(&self, e: Uuid, u: Uuid, r: &str) -> Source {
        let a = self.alertes_protection_for_actor(e, u, r).await?;
        let n = (a.adresses_a_traiter.len() + a.ne_pas_solliciter_a_confirmer.len()) as i64;
        let mut t = tuile_compteur(
            "direction.protections",
            "Protections à confirmer",
            4,
            n,
            ("protection à confirmer", "protections à confirmer"),
            "Aucune protection à confirmer",
            action("Examiner", "/web/direction/protections"),
        );
        if n > 0 {
            t.lignes.push(LignePortail {
                libelle: "Ne peut pas attendre".into(),
                detail: None,
                ton: TonLigne::Alerte,
                lien: Some("/web/direction/protections".into()),
            });
        }
        Ok(Some(t))
    }

    /// Direction : personnels absents aujourd'hui (compte et noms de service ; **jamais le motif**).
    /// Réservé à la direction : la lecture brute du service est ouverte à tout le personnel.
    async fn tuile_absences_personnels(&self, e: Uuid, u: Uuid, r: &str, jour: NaiveDate, base: Role) -> Source {
        if !matches!(base, Role::Direction | Role::Admin) {
            return Ok(None);
        }
        let l = self.list_absences_rh_for_actor(e, u, r, jour, jour).await?;
        let n = l.iter().filter(|a| a.statut != "annulee" && a.statut != "refusee").count() as i64;
        Ok(Some(tuile_compteur(
            "personnels.absences",
            "Personnels absents",
            4,
            n,
            ("personnel absent aujourd'hui", "personnels absents aujourd'hui"),
            "Aucun personnel absent",
            None,
        )))
    }
}
