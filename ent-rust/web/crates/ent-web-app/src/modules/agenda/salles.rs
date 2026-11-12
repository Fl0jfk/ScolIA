//! Réserver une salle (socle 19, lot AG3) : recherche d'une salle libre sur un créneau (règle de la salle,
//! emploi du temps réel, réservations), réservation, mes réservations et annulation.
//!
//! Aucune règle ici : le serveur décide des droits, des conflits (il les dit par nom et par date), des
//! salles de classe libérées par un cours supprimé (D-058, jamais le motif d'absence) et de la validation
//! éventuelle. La date du jour n'est posée qu'après l'hydratation.

use chrono::{Duration, NaiveDate};
use ent_types::agenda::{AnnulationReservation, RechercheSalle, ReservationVue, SalleDispo};
use ent_web_api::ApiError;
use ent_web_ui::{Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Squelette, Taille, Variante};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele::{
    composer_reservation, libelle_statut_reservation, resume_rapport, ton_statut_reservation,
    SaisieReservation,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageSalles() -> impl IntoView {
    let jour = RwSignal::new(String::new());
    let h_debut = RwSignal::new("14:00".to_string());
    let h_fin = RwSignal::new("15:00".to_string());
    let objet = RwSignal::new(String::new());
    let commentaire = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let recherche = RwSignal::new(None::<RechercheSalle>);
    let rechargement = RwSignal::new(0u32);
    let du = RwSignal::new(None::<NaiveDate>);
    Effect::new(move |_| {
        let j = aujourdhui();
        du.set(Some(j));
        if jour.get_untracked().is_empty() {
            jour.set((j + Duration::days(1)).to_string());
        }
    });
    let recharger = Callback::new(move |_| rechargement.update(|n| *n += 1));
    let ecriture = Ecriture::nouveau(recharger);

    let salles: LocalResource<Result<Vec<SalleDispo>, ApiError>> = LocalResource::new(move || {
        let _ = rechargement.get();
        let q = recherche.get();
        async move {
            match q {
                None => Ok(vec![]),
                Some(q) => session::client().agenda_rechercher_salles(&q).await,
            }
        }
    });
    let miennes: LocalResource<Result<Vec<ReservationVue>, ApiError>> =
        LocalResource::new(move || {
            let _ = rechargement.get();
            let d = du.get();
            async move {
                match d {
                    None => Ok(vec![]),
                    Some(d) => {
                        session::client()
                            .agenda_mes_reservations(d, d + Duration::days(90))
                            .await
                    }
                }
            }
        });

    view! {
        <Title text="Réserver une salle"/>
        <EnTetePage
            surtitre="Agenda".to_string()
            titre="Réserver une salle".to_string()
            sous_titre="Toutes les salles, classes comprises : un cours prime toujours ; une salle dont le cours est supprimé est proposée.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        <form
            class="agd-salles-recherche"
            on:submit=move |ev| {
                ev.prevent_default();
                let (j, hd, hf) = (jour.get_untracked(), h_debut.get_untracked(), h_fin.get_untracked());
                match composer_reservation(&SaisieReservation {
                    salle_id: Uuid::nil(),
                    jour: &j,
                    heure_debut: &hd,
                    heure_fin: &hf,
                    intitule: "-",
                    commentaire: "",
                }) {
                    Err(m) => erreur.set(Some(m)),
                    Ok(n) => {
                        erreur.set(None);
                        recherche.set(Some(RechercheSalle {
                            jour: n.jour,
                            heure_debut: n.heure_debut,
                            heure_fin: n.heure_fin,
                            capacite_min: None,
                            type_salle: None,
                        }));
                    }
                }
            }
        >
            <ChampTexte id="sal-jour" libelle="Jour" genre="date" valeur=jour requis=true/>
            <ChampTexte id="sal-debut" libelle="De" genre="time" valeur=h_debut requis=true/>
            <ChampTexte id="sal-fin" libelle="À" genre="time" valeur=h_fin requis=true/>
            <ChampTexte id="sal-objet" libelle="Objet de la réservation" valeur=objet requis=true/>
            <ChampTexte id="sal-commentaire" libelle="Commentaire (visible de vous et du gestionnaire)" valeur=commentaire/>
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Primaire soumettre=true>"Chercher une salle libre"</Bouton>
        </form>
        {move || match salles.get() {
            None => view! { <Squelette lignes=3 libelle="Recherche des salles…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| salles.refetch())/>
            }.into_any(),
            Some(Ok(l)) if recherche.get().is_none() => {
                let _ = l;
                view! { <p>"Choisissez un créneau pour voir les salles libres."</p> }.into_any()
            }
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune salle libre sur ce créneau".to_string() message="Essayez une autre heure ou un autre jour.".to_string()/>
            }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="agd-salles-liste">
                    {l.into_iter().map(|s| {
                        let id = s.salle_id;
                        view! {
                            <li class="agd-salle">
                                <strong>{s.libelle.clone()}</strong>
                                {s.capacite.map(|c| format!(" · {c} places"))}
                                {s.raison_libre.clone().map(|r| view! { <Badge libelle=r/> })}
                                {(s.mode == "sur_validation").then(|| view! { <Badge libelle="Sur validation"/> })}
                                <Bouton
                                    variante=Variante::Secondaire
                                    taille=Taille::Compacte
                                    en_cours=ecriture.en_cours
                                    on_click=Callback::new(move |_| {
                                        let saisie = SaisieReservation {
                                            salle_id: id,
                                            jour: &jour.get_untracked(),
                                            heure_debut: &h_debut.get_untracked(),
                                            heure_fin: &h_fin.get_untracked(),
                                            intitule: &objet.get_untracked(),
                                            commentaire: &commentaire.get_untracked(),
                                        };
                                        match composer_reservation(&saisie) {
                                            Err(m) => erreur.set(Some(m)),
                                            Ok(n) => {
                                                erreur.set(None);
                                                ecriture.lancer(async move {
                                                    let r = session::client().agenda_reserver_lieu(&n).await?;
                                                    Ok(resume_rapport(&r))
                                                });
                                            }
                                        }
                                    })
                                >
                                    "Réserver"
                                </Bouton>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
        <h2>"Mes réservations"</h2>
        {move || match miennes.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| miennes.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune réservation à venir".to_string() message="Vos réservations des 90 prochains jours apparaissent ici.".to_string()/>
            }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="agd-reservations">
                    {l.into_iter().map(|r| {
                        let id = r.id;
                        let actif = matches!(r.statut.as_str(), "demandee" | "confirmee");
                        view! {
                            <li>
                                <strong>{r.salle_libelle.clone()}</strong>
                                {format!(" · {} · {}–{} · {}", r.jour.format("%d/%m/%Y"), r.heure_debut.format("%H:%M"), r.heure_fin.format("%H:%M"), r.intitule)}
                                <Badge ton=ton_statut_reservation(&r.statut) libelle=libelle_statut_reservation(&r.statut)/>
                                {r.motif_decision.clone().map(|m| format!(" ({m})"))}
                                {actif.then(|| view! {
                                    <Bouton
                                        variante=Variante::Danger
                                        taille=Taille::Compacte
                                        en_cours=ecriture.en_cours
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client()
                                                    .agenda_annuler_reservation(id, &AnnulationReservation { motif: None, portee: None })
                                                    .await?;
                                                Ok("Réservation annulée.".to_string())
                                            });
                                        })
                                    >
                                        "Annuler"
                                    </Bouton>
                                })}
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}
