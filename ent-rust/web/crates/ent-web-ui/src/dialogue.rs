//! Boîtes de dialogue modales (`<dialog>` natif).

use leptos::prelude::*;

use crate::bouton::{Bouton, Variante};

/// Boîte de dialogue modale : piège du focus, touche Échap et retour du focus
/// gérés par le navigateur. `ouvert` pilote l'ouverture ; Échap le remet à `false`.
#[component]
pub fn Dialogue(
    ouvert: RwSignal<bool>,
    #[prop(into)] titre: String,
    #[prop(into)] id: String,
    children: Children,
) -> impl IntoView {
    let noeud = NodeRef::<leptos::html::Dialog>::new();
    let titre_id = format!("{id}-titre");
    let reference_titre = titre_id.clone();

    // Les effets ne s'exécutent que dans le navigateur : aucun appel DOM côté serveur.
    Effect::new(move |_| {
        let veut_ouvrir = ouvert.get();
        if let Some(dialog) = noeud.get() {
            let el: &web_sys::HtmlDialogElement = &dialog;
            if veut_ouvrir && !el.open() {
                let _ = el.show_modal();
            } else if !veut_ouvrir && el.open() {
                el.close();
            }
        }
    });

    view! {
        <dialog
            node_ref=noeud
            id=id
            class="ui-dialogue"
            aria-labelledby=reference_titre
            on:close=move |_| ouvert.set(false)
        >
            <h2 class="ui-dialogue__titre" id=titre_id>{titre}</h2>
            {children()}
        </dialog>
    }
}

/// Demande de confirmation prête à l'emploi. « Annuler » est placé en premier :
/// c'est lui qui reçoit le focus à l'ouverture (choix sans risque).
#[component]
pub fn Confirmation(
    ouvert: RwSignal<bool>,
    #[prop(into)] id: String,
    #[prop(into)] titre: String,
    #[prop(into)] message: String,
    #[prop(default = "Confirmer".into(), into)] libelle_confirmer: String,
    #[prop(default = "Annuler".into(), into)] libelle_annuler: String,
    /// Action destructrice : bouton de confirmation en rouge.
    #[prop(optional)]
    danger: bool,
    on_confirmer: Callback<()>,
) -> impl IntoView {
    view! {
        <Dialogue ouvert=ouvert titre=titre id=id>
            <p>{message}</p>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>
                    {libelle_annuler}
                </Bouton>
                <Bouton
                    variante=if danger { Variante::Danger } else { Variante::Primaire }
                    on_click=Callback::new(move |_| {
                        ouvert.set(false);
                        on_confirmer.run(());
                    })
                >
                    {libelle_confirmer}
                </Bouton>
            </div>
        </Dialogue>
    }
}
