//! Confirmation dont le texte dépend de ce qui vient d'être choisi (nom de la personne,
//! séance visée…) : `Confirmation` du design system prend un message figé.
//! « Annuler » est placé en premier : c'est lui qui reçoit le focus à l'ouverture.

use ent_web_ui::{Bouton, Dialogue, Variante};
use leptos::prelude::*;

#[component]
pub fn ConfirmationDetaillee(
    ouvert: RwSignal<bool>,
    #[prop(into)] id: String,
    #[prop(into)] titre: String,
    /// Phrase principale (réactive).
    #[prop(into)]
    message: Signal<String>,
    /// Conséquence à rappeler en toutes lettres (ex. : qui sera prévenu).
    #[prop(optional, into)]
    rappel: Option<Signal<String>>,
    #[prop(default = "Confirmer".into(), into)] libelle_confirmer: String,
    #[prop(optional)] danger: bool,
    on_confirmer: Callback<()>,
) -> impl IntoView {
    view! {
        <Dialogue ouvert=ouvert titre=titre id=id>
            <p>{move || message.get()}</p>
            {rappel.map(|r| view! { <p class="confirmation__rappel"><strong>{move || r.get()}</strong></p> })}
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>
                    "Annuler"
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
