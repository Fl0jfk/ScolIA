//! Bandeau de charge du lot C4, affiché **au moment de fixer l'échéance** dans « Donner un travail » : ce que les élèves
//! réels du ou des cours ont déjà ce jour-là (nombres et natures seulement), l'alerte éventuelle et deux dates alternatives.
//! Il n'empêche rien : le professeur peut passer outre en enregistrant ; la dérogation est alors comptée côté serveur.

use ent_web_ui::{Alerte, Annonce, Niveau, Squelette};
use leptos::prelude::*;
use uuid::Uuid;

use super::modele_c4::{
    corps_apercu, libelle_date_proposee, lignes_distribution, message_alerte, titre_bandeau,
};
use crate::noyau::session;

#[component]
pub fn BandeauCharge(
    cours: Vec<(Uuid, RwSignal<bool>)>,
    mode: RwSignal<String>,
    jour: RwSignal<String>,
    nature: RwSignal<String>,
    duree: RwSignal<String>,
    /// Vrai quand le bandeau affiche une alerte (lu à l'envoi pour compter la dérogation).
    alerte_active: RwSignal<bool>,
) -> impl IntoView {
    let apercu = LocalResource::new(move || {
        let ids: Vec<Uuid> = cours
            .iter()
            .filter(|(_, s)| s.get())
            .map(|(id, _)| *id)
            .collect();
        let corps = corps_apercu(ids, &mode.get(), &jour.get(), &nature.get(), &duree.get());
        async move {
            match corps {
                None => None,
                Some(b) => Some(session::client().cdt_charge_apercu(&b).await),
            }
        }
    });
    Effect::new(move |_| {
        let active = matches!(apercu.get().flatten(), Some(Ok(a)) if a.alerte);
        alerte_active.set(active);
    });
    view! {
        <section class="cdt-bandeau" aria-label="Charge de travail de vos élèves">
            {move || match apercu.get() {
                None => view! { <Squelette lignes=2 libelle="Calcul de la charge de vos élèves…"/> }.into_any(),
                Some(None) => view! { <p class="cdt-resume">"Choisissez un cours et une échéance pour voir la charge de vos élèves."</p> }.into_any(),
                // Un refus ou une panne ne bloque jamais la saisie : le bandeau se tait.
                Some(Some(Err(_))) => view! { <p class="cdt-resume">"La charge de vos élèves n'a pas pu être calculée."</p> }.into_any(),
                Some(Some(Ok(a))) => {
                    let message = titre_bandeau(&a);
                    let niveau = if a.alerte { Niveau::Avertissement } else { Niveau::Info };
                    let distribution = lignes_distribution(&a.evaluations_ce_jour, a.nb_eleves);
                    let alertes: Vec<String> = a.alertes.iter().map(message_alerte).collect();
                    let autres: Vec<String> = a
                        .autres_echeances
                        .iter()
                        .map(|e| {
                            let quoi = match &e.matiere {
                                Some(m) => format!("{} ({m})", e.nature),
                                None => e.nature.clone(),
                            };
                            format!("{} élève(s) : {quoi}", e.nb_eleves)
                        })
                        .collect();
                    let dates: Vec<String> = a.dates_proposees.iter().map(|d| libelle_date_proposee(d.jour, d.nb_eleves_au_dessus)).collect();
                    let phrase = message.clone();
                    view! {
                        <Annonce message=Signal::derive(move || phrase.clone())/>
                        <Alerte niveau=niveau titre=message>
                            {alertes.into_iter().map(|m| view! { <p>{m}</p> }).collect_view()}
                        </Alerte>
                        <ul class="cdt-liste" aria-label="Échéances déjà annoncées ce jour-là">
                            {distribution.into_iter().map(|l| view! { <li class="cdt-travail__detail">{l}</li> }).collect_view()}
                            {autres.into_iter().map(|l| view! { <li class="cdt-travail__detail">{l}</li> }).collect_view()}
                        </ul>
                        {(!dates.is_empty()).then(|| view! {
                            <p class="cdt-travail__detail">"Dates plus légères : "{dates.join(" · ")}</p>
                        })}
                    }
                    .into_any()
                }
            }}
        </section>
    }
}
