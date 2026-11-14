//! Déclaration d'absence (socle 2, S2-4) : logique pure (validation du formulaire, libellés)
//! et dialogue partagé entre l'accueil / la vie scolaire (« absence annoncée à l'accueil ») et
//! la famille (« déclarer une absence »). Le serveur reste l'autorité : droits, motifs connus,
//! fenêtre de dates, rapprochement avec l'appel.

use chrono::{NaiveDate, NaiveTime};
use ent_web_api::{ApiError, DeclarationAbsence, DeclarerAbsence, MotifAbsence};
use ent_web_ui::{Alerte, Bouton, ChampTexte, Chargement, Dialogue, Niveau, Selection, ZoneTexte};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::noyau::{reconnexion, session};

/// Longueur maximale du commentaire (miroir du serveur).
pub const COMMENTAIRE_MAX: usize = 1000;

/// Motif système des absences relevées à l'appel : jamais proposé pour une déclaration.
const CODE_APPEL: &str = "appel";

/// Qui déclare : l'identité de l'appelant n'est demandée qu'à l'accueil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origine {
    Accueil,
    Famille,
}

/// Saisie brute du formulaire (champs texte tels que le navigateur les fournit).
#[derive(Clone, Debug, Default)]
pub struct SaisieDeclaration {
    pub eleve: Option<Uuid>,
    pub debut_jour: String,
    pub debut_heure: String,
    pub fin_jour: String,
    pub fin_heure: String,
    pub motif: String,
    pub commentaire: String,
    pub declarant_nom: String,
    pub declarant_qualite: String,
}

fn jour(texte: &str, champ: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d")
        .map_err(|_| format!("{champ} : date invalide."))
}

fn heure(texte: &str, champ: &str) -> Result<Option<NaiveTime>, String> {
    let t = texte.trim();
    if t.is_empty() {
        return Ok(None);
    }
    NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .map(Some)
        .map_err(|_| format!("{champ} : heure invalide."))
}

fn optionnel(texte: &str) -> Option<String> {
    let t = texte.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Valide la saisie et compose le corps envoyé au serveur. Premier message d'erreur sinon.
pub fn composer_declaration(
    s: &SaisieDeclaration,
    origine: Origine,
) -> Result<DeclarerAbsence, String> {
    let eleve_id = s.eleve.ok_or("Choisissez l'élève.")?;
    if s.motif.trim().is_empty() {
        return Err("Choisissez un motif.".into());
    }
    let debut_jour = jour(&s.debut_jour, "Premier jour d'absence")?;
    // Fin vide : absence d'un seul jour.
    let fin_jour = if s.fin_jour.trim().is_empty() {
        debut_jour
    } else {
        jour(&s.fin_jour, "Dernier jour d'absence")?
    };
    if fin_jour < debut_jour {
        return Err("Le dernier jour ne peut pas précéder le premier.".into());
    }
    let debut_heure = heure(&s.debut_heure, "Heure de début")?;
    let fin_heure = heure(&s.fin_heure, "Heure de fin")?;
    if let (true, Some(d), Some(f)) = (fin_jour == debut_jour, debut_heure, fin_heure) {
        if f <= d {
            return Err("L'heure de fin doit suivre l'heure de début.".into());
        }
    }
    if s.commentaire.chars().count() > COMMENTAIRE_MAX {
        return Err(format!(
            "Le commentaire est limité à {COMMENTAIRE_MAX} caractères."
        ));
    }
    let (declarant_nom, declarant_qualite) = match origine {
        Origine::Accueil => {
            let nom = optionnel(&s.declarant_nom)
                .ok_or("Indiquez le nom de la personne qui a prévenu.")?;
            (Some(nom), optionnel(&s.declarant_qualite))
        }
        Origine::Famille => (None, None),
    };
    Ok(DeclarerAbsence {
        eleve_id,
        debut_jour,
        debut_heure,
        fin_jour,
        fin_heure,
        motif: s.motif.trim().to_string(),
        commentaire: optionnel(&s.commentaire),
        declarant_nom,
        declarant_qualite,
        piece_document_id: None,
    })
}

fn jj_mm_aaaa(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

fn hh_mm(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

/// « le 05/11/2026 », « le 05/11/2026 de 08:00 à 09:00 », « du 05/11/2026 au 07/11/2026 »…
pub fn libelle_periode_declaration(d: &DeclarationAbsence) -> String {
    if d.debut_jour == d.fin_jour {
        let base = format!("le {}", jj_mm_aaaa(d.debut_jour));
        return match (d.debut_heure, d.fin_heure) {
            (Some(a), Some(b)) => format!("{base} de {} à {}", hh_mm(a), hh_mm(b)),
            (Some(a), None) => format!("{base} à partir de {}", hh_mm(a)),
            (None, Some(b)) => format!("{base} jusqu'à {}", hh_mm(b)),
            (None, None) => base,
        };
    }
    let debut = match d.debut_heure {
        Some(a) => format!("du {} à partir de {}", jj_mm_aaaa(d.debut_jour), hh_mm(a)),
        None => format!("du {}", jj_mm_aaaa(d.debut_jour)),
    };
    let fin = match d.fin_heure {
        Some(b) => format!("au {} jusqu'à {}", jj_mm_aaaa(d.fin_jour), hh_mm(b)),
        None => format!("au {}", jj_mm_aaaa(d.fin_jour)),
    };
    format!("{debut} {fin}")
}

/// « Accueil », « Famille ».
pub fn libelle_source(source: &str) -> &'static str {
    match source {
        "accueil" => "Accueil",
        "famille" => "Famille",
        _ => "Autre",
    }
}

/// Motifs proposés : ceux du serveur, hors motif système « absent à l'appel ».
pub fn options_motifs_declaration(motifs: &[MotifAbsence]) -> Vec<(String, String)> {
    motifs
        .iter()
        .filter(|m| m.code != CODE_APPEL)
        .map(|m| (m.code.clone(), m.libelle.clone()))
        .collect()
}

/// Message d'échec ; une session perdue ouvre la fenêtre de reconnexion (sans perdre la saisie).
fn message_echec(e: ApiError) -> String {
    match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            reconnexion::MESSAGE_SESSION_EXPIREE.to_string()
        }
        autre => autre.message_utilisateur(),
    }
}

/// Dialogue de déclaration d'absence. `eleve` : l'élève concerné (déjà choisi par l'écran).
#[component]
pub fn DialogueDeclaration(
    ouvert: RwSignal<bool>,
    origine: Origine,
    #[prop(into)] eleve: Signal<Option<(Uuid, String)>>,
    /// Appelé après un envoi réussi (rechargement de l'écran).
    on_fait: Callback<DeclarationAbsence>,
) -> impl IntoView {
    let motifs = LocalResource::new(|| async { session::client().motifs_absence().await });
    let debut_jour = RwSignal::new(String::new());
    let debut_heure = RwSignal::new(String::new());
    let fin_jour = RwSignal::new(String::new());
    let fin_heure = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let commentaire = RwSignal::new(String::new());
    let nom = RwSignal::new(String::new());
    let qualite = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);

    // À chaque ouverture : formulaire vierge (jamais d'heure ni de date dans le HTML rendu au premier passage).
    Effect::new(move |_| {
        if ouvert.get() {
            for s in [
                debut_jour,
                debut_heure,
                fin_jour,
                fin_heure,
                commentaire,
                nom,
                qualite,
            ] {
                s.set(String::new());
            }
            debut_jour.set(crate::noyau::temps::aujourdhui().to_string());
            motif.set(String::new());
            erreur.set(None);
        }
    });

    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let saisie = SaisieDeclaration {
            eleve: eleve.get_untracked().map(|(id, _)| id),
            debut_jour: debut_jour.get_untracked(),
            debut_heure: debut_heure.get_untracked(),
            fin_jour: fin_jour.get_untracked(),
            fin_heure: fin_heure.get_untracked(),
            motif: motif.get_untracked(),
            commentaire: commentaire.get_untracked(),
            declarant_nom: nom.get_untracked(),
            declarant_qualite: qualite.get_untracked(),
        };
        let corps = match composer_declaration(&saisie, origine) {
            Ok(c) => c,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = match origine {
                Origine::Accueil => session::client().declarer_absence_accueil(&corps).await,
                Origine::Famille => session::client().declarer_absence_famille(&corps).await,
            };
            en_cours.set(false);
            match r {
                Ok(d) => {
                    ouvert.set(false);
                    on_fait.run(d);
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    };

    let titre = match origine {
        Origine::Accueil => "Absence annoncée à l'accueil",
        Origine::Famille => "Déclarer une absence",
    };
    view! {
        <Dialogue ouvert=ouvert titre=titre id="declaration-absence">
            <form class="decl-form" on:submit=envoyer novalidate>
                <p class="decl-eleve">
                    {move || eleve.get().map(|(_, l)| format!("Élève : {l}")).unwrap_or_default()}
                </p>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <div class="decl-ligne">
                    <ChampTexte id="decl-debut-jour" libelle="Premier jour" valeur=debut_jour genre="date" requis=true/>
                    <ChampTexte id="decl-debut-heure" libelle="À partir de (facultatif)" valeur=debut_heure genre="time"/>
                </div>
                <div class="decl-ligne">
                    <ChampTexte
                        id="decl-fin-jour"
                        libelle="Dernier jour (vide : un seul jour)"
                        valeur=fin_jour
                        genre="date"
                    />
                    <ChampTexte id="decl-fin-heure" libelle="Jusqu'à (facultatif)" valeur=fin_heure genre="time"/>
                </div>
                {move || match motifs.get() {
                    None => view! { <Chargement libelle="Chargement des motifs…"/> }.into_any(),
                    Some(Err(_)) => view! {
                        <Alerte niveau=Niveau::Erreur titre="Les motifs n'ont pas pu être chargés. Fermez puis rouvrez cette fenêtre."/>
                    }
                        .into_any(),
                    Some(Ok(liste)) => {
                        let mut options = vec![(String::new(), "— Choisir —".to_string())];
                        options.extend(options_motifs_declaration(&liste));
                        view! { <Selection id="decl-motif" libelle="Motif" valeur=motif options=options requis=true/> }
                            .into_any()
                    }
                }}
                {(origine == Origine::Accueil)
                    .then(|| view! {
                        <div class="decl-ligne">
                            <ChampTexte
                                id="decl-nom"
                                libelle="Nom de la personne qui prévient"
                                valeur=nom
                                requis=true
                                autocomplete="off"
                            />
                            <ChampTexte
                                id="decl-qualite"
                                libelle="Qualité (mère, père, tuteur…)"
                                valeur=qualite
                                autocomplete="off"
                            />
                        </div>
                    })}
                <ZoneTexte id="decl-commentaire" libelle="Commentaire (facultatif)" valeur=commentaire/>
                <div class="decl-actions">
                    <Bouton variante=ent_web_ui::Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>
                        "Annuler"
                    </Bouton>
                    <Bouton soumettre=true en_cours=en_cours>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saisie() -> SaisieDeclaration {
        SaisieDeclaration {
            eleve: Some(Uuid::nil()),
            debut_jour: "2026-11-05".into(),
            motif: "maladie".into(),
            declarant_nom: "Mme Granger".into(),
            ..Default::default()
        }
    }

    #[test]
    fn declaration_minimale_valide() {
        let c = composer_declaration(&saisie(), Origine::Accueil).unwrap();
        assert_eq!(c.debut_jour, c.fin_jour, "fin vide : un seul jour");
        assert_eq!(c.declarant_nom.as_deref(), Some("Mme Granger"));
        assert!(c.commentaire.is_none() && c.debut_heure.is_none());
    }

    #[test]
    fn la_famille_n_envoie_jamais_d_identite_de_declarant() {
        let c = composer_declaration(&saisie(), Origine::Famille).unwrap();
        assert!(c.declarant_nom.is_none() && c.declarant_qualite.is_none());
        let mut s = saisie();
        s.declarant_nom = String::new();
        assert!(composer_declaration(&s, Origine::Famille).is_ok());
        assert!(composer_declaration(&s, Origine::Accueil).is_err());
    }

    #[test]
    fn erreurs_de_saisie() {
        let err = |f: fn(&mut SaisieDeclaration)| {
            let mut s = saisie();
            f(&mut s);
            composer_declaration(&s, Origine::Accueil).unwrap_err()
        };
        assert!(err(|s| s.eleve = None).contains("élève"));
        assert!(err(|s| s.motif = "  ".into()).contains("motif"));
        assert!(err(|s| s.debut_jour = "05/11/2026".into()).contains("date invalide"));
        assert!(err(|s| s.fin_jour = "2026-11-04".into()).contains("précéder"));
        assert!(err(|s| s.debut_heure = "8h".into()).contains("heure invalide"));
        assert!(err(|s| {
            s.debut_heure = "09:00".into();
            s.fin_heure = "09:00".into();
        })
        .contains("suivre"));
        assert!(err(|s| s.commentaire = "x".repeat(COMMENTAIRE_MAX + 1)).contains("limité"));
    }

    #[test]
    fn heures_inversees_acceptees_sur_plusieurs_jours() {
        let mut s = saisie();
        s.fin_jour = "2026-11-06".into();
        s.debut_heure = "15:00".into();
        s.fin_heure = "10:00".into();
        let c = composer_declaration(&s, Origine::Famille).unwrap();
        assert_eq!(c.fin_heure, NaiveTime::from_hms_opt(10, 0, 0));
    }

    fn decl(dj: &str, dh: Option<&str>, fj: &str, fh: Option<&str>) -> DeclarationAbsence {
        let t = |h: &str| NaiveTime::parse_from_str(h, "%H:%M").unwrap();
        DeclarationAbsence {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            source: "famille".into(),
            debut_jour: dj.parse().unwrap(),
            debut_heure: dh.map(t),
            fin_jour: fj.parse().unwrap(),
            fin_heure: fh.map(t),
            motif: "maladie".into(),
            commentaire: None,
            declarant_nom: None,
            declarant_qualite: None,
            statut: "active".into(),
        }
    }

    #[test]
    fn libelles_de_periode() {
        let p = |d| libelle_periode_declaration(&d);
        assert_eq!(
            p(decl("2026-11-05", None, "2026-11-05", None)),
            "le 05/11/2026"
        );
        assert_eq!(
            p(decl(
                "2026-11-05",
                Some("08:00"),
                "2026-11-05",
                Some("09:00")
            )),
            "le 05/11/2026 de 08:00 à 09:00"
        );
        assert_eq!(
            p(decl("2026-11-05", Some("14:00"), "2026-11-05", None)),
            "le 05/11/2026 à partir de 14:00"
        );
        assert_eq!(
            p(decl("2026-11-05", None, "2026-11-05", Some("10:00"))),
            "le 05/11/2026 jusqu'à 10:00"
        );
        assert_eq!(
            p(decl("2026-11-05", Some("14:00"), "2026-11-07", None)),
            "du 05/11/2026 à partir de 14:00 au 07/11/2026"
        );
    }

    #[test]
    fn motifs_sans_le_motif_systeme() {
        let m = |c: &str| MotifAbsence {
            code: c.into(),
            libelle: c.into(),
            etablissement_id: None,
        };
        let o = options_motifs_declaration(&[m("maladie"), m("appel"), m("autre")]);
        assert_eq!(o.len(), 2);
        assert!(o.iter().all(|(c, _)| c != "appel"));
        assert_eq!(libelle_source("accueil"), "Accueil");
        assert_eq!(libelle_source("x"), "Autre");
    }
}
