//! Accueil (socle 12, AS14b, AS-216) : avant de déclarer une absence ou de répondre à un appel, on voit
//! où en est l'élève — « en sortie », « prévenu », « absent ce matin » — **sans motif**.
//!
//! Route : `GET /assiduite/eleves/{id}/situation-accueil` (`absences.declarer`). Lecture **unitaire**
//! (un élève choisi à la fois, jamais la liste du jour) et **journalisée** côté serveur : l'écran le dit.
//! La déclaration en trois gestes réutilise le dialogue commun (`composants_metier::declaration`).

use ent_types::assiduite::SituationAccueilDto;
use ent_web_ui::{Alerte, Badge, Bouton, EnTetePage, EtatVide, Niveau, Squelette, TonEtat};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::declaration::{DialogueDeclaration, Origine};
use crate::composants_metier::recherche_eleve::RechercheEleve;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use ent_web_ui::use_notifications;

/// Une ligne d'état lisible : libellé, ton. Jamais porté par la seule couleur : le libellé dit tout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EtatAccueil {
    pub libelle: String,
    pub ton: TonEtat,
}

/// Lieu codé d'une localisation, à l'accueil : « Il est à l'infirmerie. »
fn libelle_lieu(code: &str) -> &'static str {
    match code {
        "infirmerie" => "à l'infirmerie",
        "vie_scolaire" => "à la vie scolaire",
        "cdi" => "au CDI",
        "entretien" => "en entretien",
        "autre_activite" => "dans une autre activité",
        _ => "dans l'établissement",
    }
}

/// Ce que l'accueil lit pour un élève, dans l'ordre : sortie, prévenu, localisé, absent ce matin.
pub fn etats_accueil(s: &SituationAccueilDto) -> Vec<EtatAccueil> {
    let mut v = Vec::new();
    if s.en_sortie {
        v.push(EtatAccueil {
            libelle: "En sortie : il est attendu ailleurs aujourd'hui".into(),
            ton: TonEtat::Info,
        });
    }
    if s.prevenu {
        let mut t = "Prévenu : sa famille a annoncé son absence ou son retard".to_string();
        if let Some(h) = s.heure_arrivee_prevue {
            t.push_str(&format!(" (arrivée prévue à {})", h.format("%H:%M")));
        }
        v.push(EtatAccueil {
            libelle: t,
            ton: TonEtat::Ok,
        });
    }
    if let Some(l) = s.lieu_code.as_deref() {
        v.push(EtatAccueil {
            libelle: format!("Localisé : il est {}", libelle_lieu(l)),
            ton: TonEtat::Info,
        });
    }
    if s.absent_ce_matin {
        v.push(EtatAccueil {
            libelle: "Absent ce matin à l'appel".into(),
            ton: TonEtat::Avertissement,
        });
    }
    v
}

#[component]
pub fn PageAccueilEleve() -> impl IntoView {
    let notifications = use_notifications();
    let eleve = RwSignal::new(None::<(Uuid, String)>);
    let decl_ouvert = RwSignal::new(false);
    let situation = LocalResource::new(move || {
        let e = eleve.get();
        async move {
            match e {
                Some((id, _)) => session::client().situation_accueil(id).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let declaree = Callback::new(move |_| {
        notifications.succes("Absence enregistrée : elle sera rapprochée de l'appel.");
        situation.refetch();
    });
    view! {
        <Title text="Accueil : situation d'un élève"/>
        <EnTetePage
            titre="Accueil : situation d'un élève".to_string()
            sous_titre="Avant de déclarer : l'élève est-il en sortie, prévenu, absent ce matin ?".to_string()
        />
        <RechercheEleve
            id="acc-recherche"
            libelle="Rechercher un élève"
            action="Voir la situation de"
            on_choisir=Callback::new(move |e| eleve.set(Some(e)))
        />
        {move || match (eleve.get(), situation.get()) {
            (None, _) => view! {
                <EtatVide titre="Aucun élève choisi" message="Recherchez un élève par son nom pour voir sa situation."/>
            }
                .into_any(),
            (Some(_), None) => view! { <Squelette lignes=3 libelle="Chargement de la situation…"/> }.into_any(),
            (Some(_), Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| situation.refetch())/> }.into_any(),
            (Some((_, nom)), Some(Ok(None))) => view! { <p>{nom}</p> }.into_any(),
            (Some((_, nom)), Some(Ok(Some(s)))) => {
                let etats = etats_accueil(&s);
                view! {
                    <section class="acc-situation" aria-labelledby="acc-titre">
                        <h2 id="acc-titre" class="vs-section-titre">{nom.clone()}</h2>
                        {if etats.is_empty() {
                            view! { <p role="status">"Rien de particulier : ni sortie, ni absence annoncée, ni absence à l'appel ce matin."</p> }.into_any()
                        } else {
                            view! {
                                <ul class="acc-etats" role="status">
                                    {etats
                                        .into_iter()
                                        .map(|e| view! { <li><Badge libelle=e.libelle ton=e.ton/></li> })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }}
                        <Bouton on_click=Callback::new(move |_| decl_ouvert.set(true))>
                            "Déclarer une absence ou un retard"
                        </Bouton>
                        <Alerte niveau=Niveau::Info titre="Cette consultation est enregistrée.">
                            <p>"Seule la situation de l'élève choisi est lue ; aucun motif n'est affiché."</p>
                        </Alerte>
                    </section>
                }
                    .into_any()
            }
        }}
        <DialogueDeclaration
            ouvert=decl_ouvert
            origine=Origine::Accueil
            eleve=Signal::derive(move || eleve.get())
            on_fait=declaree
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn s() -> SituationAccueilDto {
        SituationAccueilDto {
            eleve_id: Uuid::nil(),
            ..Default::default()
        }
    }

    #[test]
    fn rien_a_dire() {
        assert!(etats_accueil(&s()).is_empty());
    }

    #[test]
    fn sortie_prevenu_arrivee_et_absence_du_matin() {
        let mut x = s();
        x.en_sortie = true;
        x.prevenu = true;
        x.heure_arrivee_prevue = NaiveTime::from_hms_opt(10, 5, 0);
        x.absent_ce_matin = true;
        let t: Vec<String> = etats_accueil(&x).into_iter().map(|e| e.libelle).collect();
        assert_eq!(t.len(), 3);
        assert!(t[0].starts_with("En sortie"));
        assert!(t[1].contains("arrivée prévue à 10:05"));
        assert!(t[2].starts_with("Absent ce matin"));
    }

    #[test]
    fn localise_dit_le_lieu_sans_motif() {
        let mut x = s();
        x.lieu_code = Some("infirmerie".into());
        let t = etats_accueil(&x);
        assert_eq!(t[0].libelle, "Localisé : il est à l'infirmerie");
    }
}
