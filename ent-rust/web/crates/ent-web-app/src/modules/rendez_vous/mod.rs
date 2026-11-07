//! Rendez-vous et rencontres parents-professeurs (D21, AGD-02/03).
//!
//! - `/web/rencontres` : l'organisateur (`rencontres.gerer`) crée une rencontre, propose des
//!   créneaux par enseignant, l'ouvre aux familles (qui sont prévenues) et suit le planning ;
//! - `/web/rendez-vous` : la famille réserve ou annule un créneau pour un de ses enfants ;
//!   l'enseignant voit ses rendez-vous.
//!
//! Aucune règle métier ici : le serveur contrôle le lien, la classe, l'unicité et la concurrence.

pub mod modele;

use ent_types::communication::{EnseignantCreneauxDto, LignePlanningDto, RencontreDto};
use ent_web_ui::{
    Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Selection, Squelette, Taille, TonEtat,
    Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};
use modele::{
    composer_creneaux, composer_rencontre, grouper_par_enseignant, jour_de, libelle_etat_creneau,
    libelle_statut_rencontre, peut_cloturer, peut_ouvrir, plage, qui, resume_rencontre,
    ton_statut_rencontre,
};

// ---------------------------------------------------------------------------
// Organisateur
// ---------------------------------------------------------------------------

#[component]
pub fn PageRencontres() -> impl IntoView {
    let rencontres = LocalResource::new(|| async { session::client().rencontres().await });
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let choisie = RwSignal::new(None::<RencontreDto>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| rencontres.refetch()));
    view! {
        <Title text="Rencontres parents-professeurs"/>
        <EnTetePage
            surtitre="Agenda".to_string()
            titre="Rencontres parents-professeurs".to_string()
            sous_titre="Proposez des créneaux par enseignant, ouvrez la prise de rendez-vous aux familles de la classe.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        <FormulaireRencontre classes=classes ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="renc-liste">
            <h2 id="renc-liste" class="vs-section-titre">"Rencontres"</h2>
            {move || match rencontres.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des rencontres…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| rencontres.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune rencontre" message="Créez une rencontre pour une classe."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeRencontres rencontres=l choisie=choisie ecriture=ecriture/> }.into_any(),
            }}
        </section>
        {move || choisie.get().map(|r| view! { <DetailRencontre rencontre=r/> })}
    }
}

#[component]
fn FormulaireRencontre(
    classes: LocalResource<Result<Vec<ent_web_api::ClasseApi>, ent_web_api::ApiError>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let classe = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    view! {
        <form
            class="ra-section"
            on:submit=move |ev| {
                ev.prevent_default();
                match composer_rencontre(&titre.get_untracked(), &classe.get_untracked()) {
                    Err(m) => erreur.set(Some(m)),
                    Ok(body) => {
                        erreur.set(None);
                        ecriture.lancer(async move {
                            session::client().creer_rencontre(&body).await?;
                            Ok("Rencontre créée.".to_string())
                        });
                    }
                }
            }
        >
            <h2 class="vs-section-titre">"Nouvelle rencontre"</h2>
            <ChampTexte id="renc-titre" libelle="Titre" valeur=titre requis=true/>
            {move || match classes.get() {
                Some(Ok(l)) => {
                    let mut options = vec![(String::new(), "— Choisir une classe —".to_string())];
                    options.extend(l.into_iter().map(|c| (c.id.to_string(), c.libelle)));
                    view! { <Selection id="renc-classe" libelle="Classe" valeur=classe options=options/> }.into_any()
                }
                _ => view! { <Squelette lignes=1 libelle="Chargement des classes…"/> }.into_any(),
            }}
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                "Créer la rencontre"
            </Bouton>
        </form>
    }
}

#[component]
fn ListeRencontres(
    rencontres: Vec<RencontreDto>,
    choisie: RwSignal<Option<RencontreDto>>,
    ecriture: Ecriture,
) -> impl IntoView {
    view! {
        <ul class="jvs-liste">
            {rencontres.into_iter().map(|r| {
                let id = r.id;
                let (ouvrir, clore) = (peut_ouvrir(&r), peut_cloturer(&r));
                let courante = r.clone();
                view! {
                    <li class="jvs-ligne">
                        <div>
                            <strong>{format!("{} — {}", r.titre, r.classe_libelle)}</strong>
                            <span class="texte-attenue">{format!(" · {}", resume_rencontre(&r))}</span>
                        </div>
                        <Badge libelle=libelle_statut_rencontre(&r.statut) ton=ton_statut_rencontre(&r.statut)/>
                        <div class="ra-actions">
                            <Bouton
                                variante=Variante::Secondaire
                                taille=Taille::Compacte
                                libelle_accessible=format!("Gérer les créneaux de {}", r.titre)
                                on_click=Callback::new(move |_| choisie.set(Some(courante.clone())))
                            >
                                "Créneaux"
                            </Bouton>
                            {ouvrir.then(|| view! {
                                <Bouton
                                    variante=Variante::Primaire
                                    taille=Taille::Compacte
                                    en_cours=ecriture.en_cours
                                    libelle_accessible=format!("Ouvrir {} aux familles", r.titre)
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().ouvrir_rencontre(id).await?;
                                            Ok("Rencontre ouverte : les familles sont prévenues.".to_string())
                                        });
                                    })
                                >
                                    "Ouvrir aux familles"
                                </Bouton>
                            })}
                            {clore.then(|| view! {
                                <Bouton
                                    variante=Variante::Discret
                                    taille=Taille::Compacte
                                    en_cours=ecriture.en_cours
                                    libelle_accessible=format!("Clore {}", r.titre)
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().cloturer_rencontre(id).await?;
                                            Ok("Rencontre close.".to_string())
                                        });
                                    })
                                >
                                    "Clore"
                                </Bouton>
                            })}
                        </div>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

#[component]
fn DetailRencontre(rencontre: RencontreDto) -> impl IntoView {
    let (id, classe_id) = (rencontre.id, rencontre.classe_id);
    let planning =
        LocalResource::new(move || async move { session::client().planning_rencontre(id).await });
    let enseignants = LocalResource::new(move || async move {
        session::client().enseignants_de_classe(classe_id).await
    });
    let enseignant = RwSignal::new(String::new());
    let jour = RwSignal::new(String::new());
    let heure = RwSignal::new("17:00".to_string());
    let duree = RwSignal::new("10".to_string());
    let nombre = RwSignal::new("6".to_string());
    let erreur = RwSignal::new(None::<String>);
    let ecriture_creneaux = Ecriture::nouveau(Callback::new(move |_| planning.refetch()));
    view! {
        <section class="ra-section" aria-labelledby="renc-detail">
            <h2 id="renc-detail" class="vs-section-titre">{format!("Créneaux — {}", rencontre.titre)}</h2>
            <RetourEcriture ecriture=ecriture_creneaux/>
            <form
                on:submit=move |ev| {
                    ev.prevent_default();
                    match composer_creneaux(
                        &enseignant.get_untracked(),
                        &jour.get_untracked(),
                        &heure.get_untracked(),
                        &duree.get_untracked(),
                        &nombre.get_untracked(),
                    ) {
                        Err(m) => erreur.set(Some(m)),
                        Ok(body) => {
                            erreur.set(None);
                            ecriture_creneaux.lancer(async move {
                                session::client().ajouter_creneaux(id, &body).await?;
                                Ok("Créneaux ajoutés.".to_string())
                            });
                        }
                    }
                }
            >
                {move || match enseignants.get() {
                    Some(Ok(l)) => {
                        let mut options = vec![(String::new(), "— Choisir un enseignant —".to_string())];
                        options.extend(l.into_iter().map(|e| (e.user_id.to_string(), format!("{} ({})", e.nom, e.matieres))));
                        view! { <Selection id="cren-prof" libelle="Enseignant" valeur=enseignant options=options/> }.into_any()
                    }
                    _ => view! { <Squelette lignes=1 libelle="Chargement des enseignants…"/> }.into_any(),
                }}
                <ChampTexte id="cren-jour" libelle="Jour" valeur=jour genre="date" requis=true/>
                <ChampTexte id="cren-heure" libelle="Première heure (heure de Paris)" valeur=heure genre="time" requis=true/>
                <ChampTexte id="cren-duree" libelle="Durée d'un créneau (minutes)" valeur=duree genre="number" inputmode="numeric"/>
                <ChampTexte id="cren-nombre" libelle="Nombre de créneaux" valeur=nombre genre="number" inputmode="numeric"/>
                {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture_creneaux.en_cours>
                    "Ajouter les créneaux"
                </Bouton>
            </form>
            {move || match planning.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement du planning…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| planning.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <PlanningGroupe lignes=l/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn PlanningGroupe(lignes: Vec<LignePlanningDto>) -> impl IntoView {
    if lignes.is_empty() {
        return view! { <EtatVide titre="Aucun créneau" message="Ajoutez des créneaux pour un enseignant."/> }.into_any();
    }
    view! {
        {grouper_par_enseignant(&lignes).into_iter().map(|(nom, l)| view! {
            <h3>{nom}</h3>
            <ul class="jvs-liste">
                {l.into_iter().map(|c| view! {
                    <li class="jvs-ligne">
                        <span>{format!("{} {}", date_longue(jour_de(c.debut)), plage(c.debut, c.fin))}</span>
                        <span class="texte-attenue">{qui(&c)}</span>
                    </li>
                }).collect_view()}
            </ul>
        }).collect_view()}
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Famille et enseignants
// ---------------------------------------------------------------------------

#[component]
pub fn PageRendezVous() -> impl IntoView {
    let rencontres = LocalResource::new(|| async { session::client().rencontres_famille().await });
    let mes = LocalResource::new(|| async { session::client().mes_creneaux_rencontre().await });
    let choix = RwSignal::new(None::<(Uuid, Uuid, String)>);
    view! {
        <Title text="Rendez-vous"/>
        <EnTetePage
            surtitre="Agenda".to_string()
            titre="Rendez-vous avec les enseignants".to_string()
            sous_titre="Réservez un créneau pour votre enfant lors des rencontres parents-professeurs.".to_string()
        />
        <section class="ra-section" aria-labelledby="rdv-rencontres">
            <h2 id="rdv-rencontres" class="vs-section-titre">"Rencontres ouvertes"</h2>
            {move || match rencontres.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des rencontres…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| rencontres.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune rencontre ouverte" message="Vous serez prévenu dès qu'une rencontre est ouverte pour la classe de votre enfant."/>
                }
                .into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|r| {
                            let (rid, eid) = (r.rencontre_id, r.eleve_id);
                            let intitule = format!("{} — {}", r.titre, r.eleve_prenom);
                            let pour = intitule.clone();
                            view! {
                                <li class="jvs-ligne">
                                    <span>{format!("{} · classe {} · {}", r.titre, r.classe_libelle, r.eleve_prenom)}</span>
                                    <Bouton
                                        variante=Variante::Primaire
                                        taille=Taille::Compacte
                                        libelle_accessible=format!("Choisir un créneau : {pour}")
                                        on_click=Callback::new(move |_| choix.set(Some((rid, eid, intitule.clone()))))
                                    >
                                        "Choisir un créneau"
                                    </Bouton>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
        {move || choix.get().map(|(r, e, t)| view! { <CreneauxFamille rencontre=r eleve=e titre=t/> })}
        {move || match mes.get() {
            Some(Ok(l)) if !l.is_empty() => view! {
                <section class="ra-section" aria-labelledby="rdv-mes">
                    <h2 id="rdv-mes" class="vs-section-titre">"Mes rendez-vous (enseignant)"</h2>
                    <PlanningGroupe lignes=l/>
                </section>
            }
            .into_any(),
            _ => ().into_any(),
        }}
    }
}

#[component]
fn CreneauxFamille(rencontre: Uuid, eleve: Uuid, titre: String) -> impl IntoView {
    let creneaux = LocalResource::new(move || async move {
        session::client().creneaux_famille(rencontre, eleve).await
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| creneaux.refetch()));
    view! {
        <section class="ra-section" aria-labelledby="rdv-creneaux">
            <h2 id="rdv-creneaux" class="vs-section-titre">{format!("Créneaux — {titre}")}</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match creneaux.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des créneaux…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| creneaux.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeEnseignants enseignants=l eleve=eleve ecriture=ecriture/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ListeEnseignants(
    enseignants: Vec<EnseignantCreneauxDto>,
    eleve: Uuid,
    ecriture: Ecriture,
) -> impl IntoView {
    view! {
        {enseignants.into_iter().map(|e| view! {
            <h3>{format!("{} · {}", e.nom, e.matieres)}</h3>
            {if e.creneaux.is_empty() {
                view! { <p class="texte-attenue">"Aucun créneau proposé."</p> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {e.creneaux.into_iter().map(|c| {
                            let id = c.id;
                            let etat = libelle_etat_creneau(&c);
                            let quand = format!("{} {}", date_longue(jour_de(c.debut)), plage(c.debut, c.fin));
                            let (libre, mien) = (c.etat == "libre", c.etat == "reserve_par_moi");
                            let a1 = format!("Réserver le créneau {quand} avec {}", e.nom);
                            let a2 = format!("Annuler le rendez-vous {quand} avec {}", e.nom);
                            view! {
                                <li class="jvs-ligne">
                                    <span>{quand}</span>
                                    <Badge libelle=etat ton=if mien { TonEtat::Ok } else if libre { TonEtat::Info } else { TonEtat::Neutre }/>
                                    {libre.then(|| view! {
                                        <Bouton variante=Variante::Primaire taille=Taille::Compacte
                                            en_cours=ecriture.en_cours libelle_accessible=a1
                                            on_click=Callback::new(move |_| {
                                                ecriture.lancer(async move {
                                                    session::client().reserver_creneau(id, eleve).await?;
                                                    Ok("Rendez-vous réservé.".to_string())
                                                });
                                            })>
                                            "Réserver"
                                        </Bouton>
                                    })}
                                    {mien.then(|| view! {
                                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                            en_cours=ecriture.en_cours libelle_accessible=a2
                                            on_click=Callback::new(move |_| {
                                                ecriture.lancer(async move {
                                                    session::client().annuler_reservation(id).await?;
                                                    Ok("Rendez-vous annulé.".to_string())
                                                });
                                            })>
                                            "Annuler"
                                        </Bouton>
                                    })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            }}
        }).collect_view()}
    }
}
