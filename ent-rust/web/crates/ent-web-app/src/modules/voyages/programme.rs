//! Programme d'une sortie (étapes du séjour) : lecture dans la fiche et, pour qui peut modifier,
//! liste éditable (un signal par champ, ajout, suppression, tri par date) envoyée en bloc par
//! `PUT /voyages/{id}/programme`. Le contrôle local (modèle pur) n'est qu'un confort : le serveur
//! revérifie tout et ses refus sont affichés tels quels.

use ent_types::voyages::SortieDetailDto;
use ent_web_ui::{
    classes_bouton, Badge, Bouton, Case, ChampTexte, ResumeErreurs, Selection, Taille, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;

use super::modele_programme::{
    composer_programme, duree_lisible, edition_depuis, etape_par_defaut, id_champ,
    libelle_type_etape, ordre_chronologique, EtapeEdition, TYPES_ETAPE,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

/// Une ligne éditable : un signal par champ, la structure reste `Copy`.
#[derive(Clone, Copy)]
struct LigneEtape {
    cle: u32,
    type_etape: RwSignal<String>,
    date: RwSignal<String>,
    heure: RwSignal<String>,
    duree: RwSignal<String>,
    lieu: RwSignal<String>,
    note: RwSignal<String>,
    appel: RwSignal<bool>,
}

impl LigneEtape {
    fn nouvelle(e: &EtapeEdition) -> Self {
        Self {
            cle: e.cle,
            type_etape: RwSignal::new(e.type_etape.clone()),
            date: RwSignal::new(e.date.clone()),
            heure: RwSignal::new(e.heure.clone()),
            duree: RwSignal::new(e.duree.clone()),
            lieu: RwSignal::new(e.lieu.clone()),
            note: RwSignal::new(e.note.clone()),
            appel: RwSignal::new(e.appel_requis),
        }
    }

    fn lire(&self) -> EtapeEdition {
        EtapeEdition {
            cle: self.cle,
            type_etape: self.type_etape.get_untracked(),
            date: self.date.get_untracked(),
            heure: self.heure.get_untracked(),
            duree: self.duree.get_untracked(),
            lieu: self.lieu.get_untracked(),
            note: self.note.get_untracked(),
            appel_requis: self.appel.get_untracked(),
        }
    }
}

/// Section « Programme » de la fiche.
#[component]
pub fn Programme(d: SortieDetailDto, recharger: Callback<()>) -> impl IntoView {
    let peut = d.acces.modifier;
    let mut etapes = d.etapes.clone();
    etapes.sort_by_key(|e| (e.debut, e.ordre));
    view! {
        <section aria-labelledby="voy-programme">
            <h2 id="voy-programme" class="vs-section-titre">"Programme"</h2>
            {if etapes.is_empty() {
                view! { <p class="texte-attenue">"Aucune étape au programme pour le moment."</p> }.into_any()
            } else {
                view! {
                    <ol class="voy-liste voy-liste--simple">
                        {etapes.into_iter().map(|e| {
                            let detail = [
                                e.duree_minutes.map(duree_lisible),
                                e.lieu.clone(),
                                e.note.clone(),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" · ");
                            view! {
                                <li class="voy-ligne">
                                    <span>
                                        <strong>{date_heure_paris(e.debut)}</strong>
                                        {format!(" · {}", libelle_type_etape(&e.type_etape))}
                                        {(!detail.is_empty()).then(|| format!(" · {detail}"))}
                                    </span>
                                    {e.appel_requis.then(|| view! { <Badge libelle="Appel requis" ton=TonEtat::Info/> })}
                                </li>
                            }
                        }).collect_view()}
                    </ol>
                }.into_any()
            }}
            {peut.then(|| view! {
                <details class="voy-registre">
                    <summary>"Modifier le programme"</summary>
                    <EditeurProgramme d=d.clone() recharger=recharger/>
                </details>
            })}
        </section>
    }
}

#[component]
fn EditeurProgramme(d: SortieDetailDto, recharger: Callback<()>) -> impl IntoView {
    let id = d.sortie.id;
    let debut_sortie = d.sortie.debut;
    let voyage = d.sortie.forme == "voyage";
    let ecriture = Ecriture::nouveau(recharger);
    let mut existantes = d.etapes.clone();
    existantes.sort_by_key(|e| (e.debut, e.ordre));
    let compteur = RwSignal::new(existantes.len() as u32);
    let lignes = RwSignal::new(
        existantes
            .iter()
            .enumerate()
            .map(|(i, e)| LigneEtape::nouvelle(&edition_depuis(e, i as u32)))
            .collect::<Vec<_>>(),
    );
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let lues = move || -> Vec<EtapeEdition> {
        lignes
            .get_untracked()
            .iter()
            .map(LigneEtape::lire)
            .collect()
    };
    let ajouter = Callback::new(move |_| {
        let cle = compteur.get_untracked();
        compteur.set(cle + 1);
        let nouvelle = etape_par_defaut(cle, &lues(), debut_sortie);
        lignes.update(|v| v.push(LigneEtape::nouvelle(&nouvelle)));
        ecriture
            .annonce
            .set("Étape ajoutée à la fin de la liste.".into());
    });
    let trier = Callback::new(move |_| {
        let ordre = ordre_chronologique(&lues());
        lignes.update(|v| v.sort_by_key(|l| ordre.iter().position(|c| *c == l.cle)));
        ecriture
            .annonce
            .set("Étapes classées par date et heure.".into());
    });
    let supprimer = Callback::new(move |cle: u32| {
        lignes.update(|v| v.retain(|l| l.cle != cle));
        erreurs.update(|e| e.retain(|(c, _)| !c.starts_with(&format!("voy-pg-{cle}-"))));
        ecriture.annonce.set("Étape supprimée de la liste.".into());
    });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        match composer_programme(&lues()) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    let n = corps.len();
                    session::client().voyage_programme(id, &corps).await?;
                    Ok(format!("Programme enregistré : {n} étape(s)."))
                });
            }
        }
    };
    view! {
        <form class="voy-formulaire voy-formulaire--large" on:submit=envoyer>
            {voyage.then(|| view! {
                <p class="texte-attenue">
                    "Modifier le programme d'un voyage rouvre la liste des élèves déjà confirmée : elle sera à reconfirmer."
                </p>
            })}
            <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
            {move || lignes.get().is_empty().then(|| view! {
                <p class="texte-attenue">"Aucune étape : ajoutez-en une, ou enregistrez pour vider le programme."</p>
            })}
            <ul class="voy-liste">
                <For
                    each=move || lignes.get()
                    key=|l| l.cle
                    children=move |l| view! { <LigneEtapeVue l=l erreurs=erreurs supprimer=supprimer/> }
                />
            </ul>
            <RetourEcriture ecriture=ecriture/>
            <div class="voy-actions">
                <Bouton variante=Variante::Secondaire on_click=ajouter>"Ajouter une étape"</Bouton>
                <Bouton variante=Variante::Secondaire on_click=trier>"Classer par date"</Bouton>
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer le programme"</Bouton>
            </div>
        </form>
    }
}

#[component]
fn LigneEtapeVue(
    l: LigneEtape,
    erreurs: RwSignal<Vec<(String, String)>>,
    supprimer: Callback<u32>,
) -> impl IntoView {
    let cle = l.cle;
    let erreur_de = move |champ: &'static str| {
        let cible = id_champ(cle, champ);
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| *c == cible)
                .map(|(_, m)| m)
        })
    };
    let options: Vec<(String, String)> = TYPES_ETAPE
        .iter()
        .map(|(c, lib)| ((*c).to_string(), (*lib).to_string()))
        .collect();
    let intitule = move || {
        format!(
            "{} du {} à {}",
            libelle_type_etape(&l.type_etape.get()),
            l.date.get(),
            l.heure.get()
        )
    };
    view! {
        <li class="voy-carte">
            <fieldset class="voy-etape-ligne">
                <legend>{move || libelle_type_etape(&l.type_etape.get())}</legend>
                <Selection id=id_champ(cle, "type") libelle="Type d'étape" valeur=l.type_etape options=options requis=true erreur=erreur_de("type")/>
                <ChampTexte id=id_champ(cle, "date") libelle="Date" genre="date" valeur=l.date requis=true erreur=erreur_de("date")/>
                <ChampTexte id=id_champ(cle, "heure") libelle="Heure" genre="time" valeur=l.heure requis=true erreur=erreur_de("heure")/>
                <ChampTexte id=id_champ(cle, "duree") libelle="Durée (minutes)" genre="number" valeur=l.duree erreur=erreur_de("duree")/>
                <ChampTexte id=id_champ(cle, "lieu") libelle="Lieu" valeur=l.lieu erreur=erreur_de("lieu")/>
                <ZoneTexte id=id_champ(cle, "note") libelle="Note" valeur=l.note erreur=erreur_de("note")/>
                <Case id=id_champ(cle, "appel") libelle="Appel requis à cette étape" coche=l.appel/>
                <button
                    type="button"
                    class=classes_bouton(Variante::Discret, Taille::Compacte)
                    aria-label=move || format!("Supprimer l'étape : {}", intitule())
                    on:click=move |_| supprimer.run(cle)
                >
                    "Supprimer"
                </button>
            </fieldset>
        </li>
    }
}
