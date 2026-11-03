//! Module d'appel : choix de la séance, puis feuille d'appel.
//! Adresse : `/web/appel?jour=AAAA-MM-JJ&creneau=<id>` (lien direct depuis l'accueil
//! et l'emploi du temps). Règles métier et droits : côté serveur, inchangés.

mod feuille;
pub mod modele;

use chrono::NaiveDate;
use ent_web_api::Creneau;
use ent_web_ui::Niveau;
use ent_web_ui::{
    Alerte, Badge, EnTetePage, EtatVide, Icone, LienBouton, NomIcone, Squelette, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_query_map};
use uuid::Uuid;

use modele::jour_demande;

use crate::composants_metier::seances::{lien_appel, moment, trier, Moment};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte, maintenant};

/// Libellé d'une séance : « 08:00–09:00 · Potions · 4B · Donjons ».
pub fn libelle_creneau(c: &Creneau) -> String {
    let heures = format!(
        "{}–{}",
        heure_courte(&c.heure_debut),
        heure_courte(&c.heure_fin)
    );
    let mut parts = vec![heures, c.matiere_libelle.clone(), c.classe.clone()];
    if let Some(s) = c.salle.as_ref().filter(|s| !s.is_empty()) {
        parts.push(s.clone());
    }
    parts.join(" · ")
}

/// Classes distinctes des séances, triées.
pub fn classes_de(creneaux: &[Creneau]) -> Vec<String> {
    let mut v: Vec<String> = creneaux.iter().map(|c| c.classe.clone()).collect();
    v.sort();
    v.dedup();
    v
}

#[component]
pub fn PageAppel() -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    let jour_brut =
        Memo::new(move |_| query.with(|q| jour_demande(q.get("jour").as_deref(), aujourdhui())));
    let jour = Memo::new(move |_| jour_brut.get().0);
    let jour_invalide = Memo::new(move |_| jour_brut.get().1);
    let creneau_id = Memo::new(move |_| {
        query
            .with(|q| q.get("creneau"))
            .and_then(|c| c.parse::<Uuid>().ok())
    });
    let creneaux = LocalResource::new(move || {
        let j = jour.get();
        async move { session::client().mes_creneaux(j).await.map(trier) }
    });
    let seance = Memo::new(move |_| {
        let id = creneau_id.get()?;
        creneaux
            .get()
            .and_then(Result::ok)
            .and_then(|l| l.into_iter().find(|c| c.id == id))
    });

    let titre = Signal::derive(move || match seance.get() {
        Some(c) => format!("{} · {}", c.classe, c.matiere_libelle),
        None => "Faire l'appel".into(),
    });
    let sous_titre = Signal::derive(move || match seance.get() {
        Some(c) => {
            let mut s = format!(
                "{} · {} – {}",
                date_longue(jour.get()),
                heure_courte(&c.heure_debut),
                heure_courte(&c.heure_fin)
            );
            if let Some(salle) = c.salle.filter(|s| !s.is_empty()) {
                s.push_str(&format!(" · Salle {salle}"));
            }
            s
        }
        None => date_longue(jour.get()),
    });

    let changer_jour = move |ev: leptos::ev::Event| {
        if let Ok(j) = event_target_value(&ev).parse::<NaiveDate>() {
            navigate(
                &format!("{}?jour={j}", navigation::APPEL),
                Default::default(),
            );
        }
    };

    view! {
        <Title text=move || titre.get()/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::ACCUEIL>"Aujourd'hui"</a>
            <span aria-hidden="true">" › "</span>
            {move || if creneau_id.get().is_some() {
                view! { <a href=move || format!("{}?jour={}", navigation::APPEL, jour.get())>"Appel"</a> }.into_any()
            } else {
                view! { <span aria-current="page">"Appel"</span> }.into_any()
            }}
        </nav>
        <EnTetePage titre=titre sous_titre=sous_titre compacte=true>
            <Show when=move || creneau_id.get().is_none()>
                <label class="appel-jour">
                    <span class="ui-sr">"Date de l'appel"</span>
                    <input
                        id="jour"
                        type="date"
                        prop:value=move || jour.get().to_string()
                        on:change=changer_jour.clone()
                    />
                </label>
            </Show>
        </EnTetePage>

        {move || {
            jour_invalide
                .get()
                .then(|| {
                    view! {
                        <Alerte niveau=Niveau::Avertissement titre="La date demandée dans l'adresse est invalide.">
                            <p>{format!("Affichage du jour courant ({}).", date_longue(jour.get()))}</p>
                        </Alerte>
                    }
                })
        }}

        {move || match (creneau_id.get(), creneaux.get()) {
            (_, None) => view! { <Squelette lignes=4 libelle="Chargement des séances…"/> }.into_any(),
            (_, Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| creneaux.refetch())/> }.into_any(),
            (None, Some(Ok(list))) => view! { <ChoixSeance creneaux=list jour=jour.get()/> }.into_any(),
            (Some(id), Some(Ok(list))) => {
                if list.iter().any(|c| c.id == id) {
                    view! { <feuille::FeuilleAppel creneau_id=id jour=jour.get()/> }.into_any()
                } else {
                    view! {
                        <EtatVide
                            titre="Séance introuvable"
                            message="Cette séance ne figure pas dans votre emploi du temps à cette date."
                        >
                            <LienBouton href=format!("{}?jour={}", navigation::APPEL, jour.get()) variante=Variante::Secondaire>
                                "Choisir une autre séance"
                            </LienBouton>
                        </EtatVide>
                    }
                        .into_any()
                }
            }
        }}
    }
}

#[component]
fn ChoixSeance(creneaux: Vec<Creneau>, jour: NaiveDate) -> impl IntoView {
    if creneaux.is_empty() {
        return view! {
            <EtatVide titre="Aucune séance ce jour" message="Vous n'avez pas de cours à appeler à cette date.">
                <LienBouton href=navigation::EMPLOI_DU_TEMPS variante=Variante::Secondaire>"Voir l'emploi du temps"</LienBouton>
            </EtatVide>
        }
        .into_any();
    }
    let heure = maintenant();
    let est_aujourdhui = jour == aujourdhui();
    view! {
        <section aria-labelledby="choix-titre" class="appel-choix">
            <h2 id="choix-titre">"Choisissez une séance"</h2>
            <ul class="appel-seances">
                {creneaux
                    .into_iter()
                    .map(|c| {
                        let m = est_aujourdhui.then(|| moment(&c, heure));
                        let lien = lien_appel(&c, jour);
                        let salle = c.salle.clone().filter(|s| !s.is_empty()).map(|s| format!("Salle {s}"));
                        view! {
                            <li>
                                <a class="appel-seance" href=lien>
                                    <span class="appel-seance__heure">
                                        {format!("{} – {}", heure_courte(&c.heure_debut), heure_courte(&c.heure_fin))}
                                    </span>
                                    <span class="appel-seance__principal">
                                        <span class="appel-seance__titre">{format!("{} · {}", c.classe, c.matiere_libelle)}</span>
                                        <span class="appel-seance__detail">{salle.unwrap_or_else(|| "Salle non précisée".into())}</span>
                                    </span>
                                    {match m {
                                        Some(Moment::EnCours) => Some(view! { <Badge libelle="En cours" ton=TonEtat::Info/> }),
                                        Some(Moment::Terminee) => Some(view! { <Badge libelle="Terminé"/> }),
                                        _ => None,
                                    }}
                                    <Icone nom=NomIcone::ChevronDroite petite=true/>
                                </a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </section>
    }
    .into_any()
}
