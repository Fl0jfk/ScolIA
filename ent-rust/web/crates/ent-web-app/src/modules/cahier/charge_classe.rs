//! Écran C4 « Charge des classes » (`/web/cahier/charge`) : calendrier d'une classe sur quatre semaines, avec les matières
//! (l'équipe lit déjà le cahier de la classe), la charge réelle de ses élèves agrégée (jamais un nom), les alertes ignorées
//! comptées et, pour la direction, le choix de la position des repères. Aucun blocage : c'est une discussion d'équipe.

use ent_web_ui::{
    AccesRefuse, Alerte, Annonce, Bouton, EnTetePage, EtatVide, Niveau, OptionSegment, Segmente,
    Selection, Squelette, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use super::gestionnaire_echec;
use super::modele_c3::fin_de_semaine;
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

pub const ADRESSE_CHARGE: &str = "/web/cahier/charge";

#[component]
pub fn PageChargeClasses() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Charge des classes"/>
        {move || {
            let d = droits.get();
            if d.cahier_ecrire || d.voir_classes {
                view! { <Contenu/> }.into_any()
            } else {
                view! { <AccesRefuse message="La charge des classes est réservée à l'équipe pédagogique et à la direction."/> }.into_any()
            }
        }}
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let classes =
        LocalResource::new(|| async { session::client().mes_classes(aujourdhui()).await });
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Charge des classes".to_string()
            sous_titre="Évaluations et travaux longs par jour, charge réelle des élèves. Des repères, jamais un blocage.".to_string()
        />
        {move || match classes.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des classes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune classe" message="Aucune classe n'est accessible."/> }.into_any(),
            Some(Ok(l)) => {
                let options: Vec<(String, String)> = l.iter().map(|c| (c.id.to_string(), c.code.clone())).collect();
                let choix = RwSignal::new(l[0].id.to_string());
                view! {
                    <Selection id="cdt-charge-classe" libelle="Classe" valeur=choix options=options/>
                    {move || choix.get().parse::<Uuid>().ok().map(|id| view! { <CalendrierClasse classe=id/> })}
                    <Reperes/>
                    <Alertes/>
                }.into_any()
            }
        }}
    }
}

#[component]
fn CalendrierClasse(classe: Uuid) -> impl IntoView {
    let cal = LocalResource::new(move || async move {
        let du = aujourdhui();
        session::client()
            .cdt_calendrier_classe(classe, du, fin_de_semaine(du + chrono::Duration::days(21)))
            .await
    });
    view! {
        {move || match cal.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement du calendrier…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| cal.refetch())/> }.into_any(),
            Some(Ok(c)) if c.jours.is_empty() => view! { <EtatVide titre="Rien d'annoncé" message="Aucune évaluation ni travail long sur ces quatre semaines."/> }.into_any(),
            Some(Ok(c)) => view! {
                <p class="cdt-resume">{format!("{} élèves, repères « {} ».", c.nb_eleves, c.position)}</p>
                <div class="cdt-semaine">
                    {c.jours.into_iter().map(|j| {
                        let au_dessus = (j.nb_eleves_au_dessus > 0).then(|| format!("{} élève(s) au-dessus d'un repère", j.nb_eleves_au_dessus));
                        view! {
                            <section class="cdt-jour">
                                <h2 class="cdt-jour__titre">
                                    {date_longue(j.jour)}
                                    {au_dessus.map(|t| view! { <span class="cdt-jour__total">{t}</span> })}
                                </h2>
                                <ul class="cdt-liste">
                                    {j.lignes.into_iter().map(|l| view! {
                                        <li class="cdt-travail__detail">{format!("{} — {} ({})", l.matiere, l.titre, l.nature)}</li>
                                    }).collect_view()}
                                </ul>
                            </section>
                        }
                    }).collect_view()}
                </div>
            }.into_any(),
        }}
    }
}

#[component]
fn Reperes() -> impl IntoView {
    let droits = use_droits();
    let reperes = LocalResource::new(|| async { session::client().cdt_reperes().await });
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let echec = gestionnaire_echec(erreur);
    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {move || match reperes.get() {
            Some(Ok(r)) => {
                let choix = RwSignal::new(r.position.clone());
                let peut = droits.get().gerer_classes;
                let options = ["souple", "normal", "strict"].map(|p| OptionSegment::new(p.to_string(), p.to_string())).to_vec();
                view! {
                    <section aria-label="Repères de charge">
                        <h2>"Repères de l'établissement"</h2>
                        <ul class="cdt-liste">
                            {r.groupes.iter().map(|g| view! {
                                <li class="cdt-travail__detail">{format!("{} : {} évaluation(s) par jour, {} min par soir, {} par semaine", g.libelle, g.evaluations_jour, g.minutes_soir, g.evaluations_semaine)}</li>
                            }).collect_view()}
                        </ul>
                        {peut.then(|| view! {
                            <Segmente nom="cdt-position" legende="Position" options=options valeur=choix/>
                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| {
                                let p = choix.get_untracked();
                                spawn_local(async move {
                                    match session::client().cdt_reperes_regler(&p).await {
                                        Ok(_) => { annonce.set("Repères enregistrés.".into()); reperes.refetch(); }
                                        Err(e) => echec.run(e),
                                    }
                                });
                            })>"Enregistrer la position"</Bouton>
                        })}
                    </section>
                }.into_any()
            }
            _ => ().into_any(),
        }}
    }
}

#[component]
fn Alertes() -> impl IntoView {
    let alertes = LocalResource::new(|| async {
        let au = aujourdhui();
        session::client()
            .cdt_alertes_ignorees(au - chrono::Duration::days(60), au)
            .await
    });
    view! {
        {move || match alertes.get() {
            Some(Ok(a)) => view! {
                <section aria-label="Alertes ignorées">
                    <h2>"Alertes passées outre (60 derniers jours)"</h2>
                    <p class="cdt-resume">{format!("{} travail(aux) donné(s) malgré une alerte.", a.total)}</p>
                    <ul class="cdt-liste">
                        {a.par_classe.into_iter().map(|c| view! { <li class="cdt-travail__detail">{format!("{} : {}", c.classe, c.nb)}</li> }).collect_view()}
                        {a.mes_alertes.into_iter().map(|m| view! { <li class="cdt-travail__detail">{format!("{} — {} ({} > {})", m.titre, m.repere, m.valeur, m.seuil)}</li> }).collect_view()}
                    </ul>
                </section>
            }.into_any(),
            _ => ().into_any(),
        }}
    }
}
