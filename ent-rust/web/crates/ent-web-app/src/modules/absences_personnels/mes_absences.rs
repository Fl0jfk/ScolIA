//! « Mes absences » (ABP-148, ABP-112) : la liste des absences de l'agent connecté, à venir puis passées,
//! avec un accès direct à la déclaration. La nature n'est celle que de l'agent lui-même (portée `moi`).

use ent_web_ui::{Alerte, EtatVide, LienBouton, Niveau, Squelette};
use leptos::prelude::*;
use leptos_meta::Title;

use super::commun::{CarteAbsence, GardePersonnel};
use super::modele::partager_mes_absences;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageMesAbsences() -> impl IntoView {
    view! {
        <Title text="Mes absences"/>
        <GardePersonnel
            titre="Mes absences"
            sous_titre="Vos absences déclarées, leur état et ce qui reste à faire."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().absp_mes_absences().await });
    view! {
        <p class="absp-barre">
            <LienBouton href="/web/personnel/absences/declarer">"Déclarer une absence"</LienBouton>
        </p>
        {move || match liste.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos absences…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(v)) if v.is_empty() => view! {
                <EtatVide
                    titre="Aucune absence déclarée"
                    message="Quand vous serez absent, déclarez-le ici : la direction et l'organisation des remplacements en sont informées sans connaître votre motif."
                />
            }
            .into_any(),
            Some(Ok(v)) => {
                let signalees = v.iter().filter(|a| a.etat == "signalee").count();
                let (a_venir, passees) = partager_mes_absences(v, chrono::Utc::now());
                view! {
                    {(signalees > 0).then(|| view! {
                        <Alerte
                            niveau=Niveau::Avertissement
                            titre=if signalees == 1 {
                                "Une absence a été signalée à votre nom : ouvrez-la pour la confirmer.".to_string()
                            } else {
                                format!("{signalees} absences ont été signalées à votre nom : ouvrez-les pour les confirmer.")
                            }
                        />
                    })}
                    <section class="absp-section" aria-labelledby="absp-a-venir">
                        <h2 id="absp-a-venir" class="vs-section-titre">"En cours et à venir"</h2>
                        {if a_venir.is_empty() {
                            view! { <p class="texte-attenue">"Aucune absence en cours ni à venir."</p> }.into_any()
                        } else {
                            view! {
                                <ul class="absp-liste">
                                    {a_venir.into_iter().map(|a| view! { <CarteAbsence a=a/> }).collect_view()}
                                </ul>
                            }
                            .into_any()
                        }}
                    </section>
                    {(!passees.is_empty()).then(|| view! {
                        <section class="absp-section" aria-labelledby="absp-passees">
                            <h2 id="absp-passees" class="vs-section-titre">"Passées"</h2>
                            <ul class="absp-liste">
                                {passees.into_iter().map(|a| view! { <CarteAbsence a=a/> }).collect_view()}
                            </ul>
                        </section>
                    })}
                }
                .into_any()
            }
        }}
    }
}
