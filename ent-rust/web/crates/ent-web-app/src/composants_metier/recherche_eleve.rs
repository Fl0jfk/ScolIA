//! Recherche d'un élève par le personnel (`GET /eleves/search`) : champ, résultats et choix.
//! Le serveur décide de ce que la personne a le droit de voir ; l'écran affiche le résultat.

use ent_web_api::EleveTrouve;
use ent_web_ui::{Bouton, ChampTexte, Variante};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::noyau::ecriture::message_echec;
use crate::noyau::session;

/// Libellé « Prénom Nom · classe ».
pub fn libelle_eleve(e: &EleveTrouve) -> String {
    let nom = format!("{} {}", e.prenom, e.nom).trim().to_string();
    match e.classe.as_deref().filter(|c| !c.is_empty()) {
        Some(c) => format!("{nom} · {c}"),
        None => nom,
    }
}

/// Champ de recherche (2 caractères au moins) puis liste de boutons ; `on_choisir` reçoit
/// l'identifiant et le libellé de l'élève choisi.
#[component]
pub fn RechercheEleve(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    /// Début du libellé accessible des boutons : « Choisir l'élève Harry Potter ».
    #[prop(default = "Choisir l'élève".into(), into)]
    action: String,
    on_choisir: Callback<(Uuid, String)>,
) -> impl IntoView {
    let requete = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let resultats = RwSignal::new(None::<Vec<EleveTrouve>>);
    let en_cours = RwSignal::new(false);
    let chercher = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let texte = requete.get_untracked();
        if texte.trim().chars().count() < 2 {
            erreur.set(Some("Saisissez au moins 2 caractères.".into()));
            return;
        }
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            match session::client().messagerie_chercher_eleves(&texte).await {
                Ok(l) => resultats.set(Some(l)),
                Err(e) => {
                    resultats.set(None);
                    erreur.set(Some(message_echec(e)));
                }
            }
            en_cours.set(false);
        });
    };
    view! {
        <form role="search" on:submit=chercher novalidate>
            <ChampTexte
                id=id
                libelle=libelle
                valeur=requete
                genre="search"
                erreur=Signal::derive(move || erreur.get())
                autocomplete="off"
            />
            <Bouton soumettre=true en_cours=en_cours>"Rechercher"</Bouton>
        </form>
        {move || {
            resultats
                .get()
                .map(|liste| {
                    if liste.is_empty() {
                        view! { <p class="texte-attenue" role="status">"Aucun élève ne correspond."</p> }.into_any()
                    } else {
                        let action = action.clone();
                        view! {
                            <ul class="jvs-liste" aria-label="Élèves trouvés">
                                {liste
                                    .into_iter()
                                    .map(|e| {
                                        let libelle = libelle_eleve(&e);
                                        let (pour_choix, accessible) = (libelle.clone(), format!("{action} {libelle}"));
                                        let id = e.id;
                                        view! {
                                            <li>
                                                <Bouton
                                                    variante=Variante::Secondaire
                                                    libelle_accessible=accessible
                                                    on_click=Callback::new(move |_| on_choisir.run((id, pour_choix.clone())))
                                                >
                                                    {libelle}
                                                </Bouton>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                        .into_any()
                    }
                })
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libelle_avec_et_sans_classe() {
        let e = |classe: Option<&str>| EleveTrouve {
            id: Uuid::nil(),
            nom: "Potter".into(),
            prenom: "Harry".into(),
            classe: classe.map(str::to_string),
        };
        assert_eq!(libelle_eleve(&e(Some("5A"))), "Harry Potter · 5A");
        assert_eq!(libelle_eleve(&e(Some(""))), "Harry Potter");
        assert_eq!(libelle_eleve(&e(None)), "Harry Potter");
    }
}
