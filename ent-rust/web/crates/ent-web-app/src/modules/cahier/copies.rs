//! Écran enseignant : copies reçues pour un travail à rendre, téléchargement par lien signé
//! court, retour à l'élève (commentaire et copie annotée facultative).

use ent_web_api::cahier_types::{LigneSuiviDto, RemiseDto, SuiviTravailDto};
use ent_web_api::depot_fichier;
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Badge, Bouton, EnTetePage, EtatVide, LienBouton,
    Niveau, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use super::fichiers::{lire_fichier_choisi, FichierChoisi, Lecture};
use super::modele::{libelle_limite, libelle_remise, libelle_taille, trier_suivi, valider_retour};
use super::{gestionnaire_echec, ouvrir_piece};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::date_longue;

#[component]
pub fn PageCopies() -> impl IntoView {
    let droits = use_droits();
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|s| s.parse::<Uuid>().ok())
    });
    view! {
        <Title text="Copies reçues"/>
        {move || {
            if !droits.get().cahier_ecrire {
                return view! { <AccesRefuse message="Les copies sont réservées aux enseignants."/> }.into_any();
            }
            match id.get() {
                None => view! { <EtatVide titre="Travail introuvable" message="L'adresse de cette page est incorrecte."/> }.into_any(),
                Some(id) => view! { <SuiviTravail id=id/> }.into_any(),
            }
        }}
    }
}

#[component]
fn SuiviTravail(id: Uuid) -> impl IntoView {
    let suivi = LocalResource::new(move || async move { session::client().cahier_suivi(id).await });
    view! {
        {move || match suivi.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement des copies…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| suivi.refetch())/> }.into_any(),
            Some(Ok(s)) => view! { <TableauSuivi suivi=s on_change=Callback::new(move |_| suivi.refetch())/> }.into_any(),
        }}
    }
}

#[component]
fn TableauSuivi(suivi: SuiviTravailDto, on_change: Callback<()>) -> impl IntoView {
    let t = suivi.travail.clone();
    let titre = format!("{} — {}", t.matiere_libelle, t.titre);
    let sous_titre = format!(
        "Classe ou groupe {} · pour le {}{}",
        t.cible,
        date_longue(t.pour_le),
        libelle_limite(&t)
            .map(|l| format!(" · {l}"))
            .unwrap_or_default()
    );
    let resume = format!(
        "{} copie{} reçue{} sur {} attendue{}, dont {} en retard.",
        suivi.nb_rendus,
        if suivi.nb_rendus > 1 { "s" } else { "" },
        if suivi.nb_rendus > 1 { "s" } else { "" },
        suivi.nb_attendus,
        if suivi.nb_attendus > 1 { "s" } else { "" },
        suivi.nb_en_retard
    );
    let erreur = RwSignal::new(None::<String>);
    let lignes = trier_suivi(suivi.lignes);
    view! {
        <EnTetePage titre=titre sous_titre=sous_titre surtitre="Copies reçues".to_string()>
            <LienBouton href=navigation::CAHIER variante=Variante::Secondaire taille=Taille::Compacte>
                "Retour au cahier"
            </LienBouton>
        </EnTetePage>
        <p class="cdt-resume" role="status">{resume}</p>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {if lignes.is_empty() {
            view! { <EtatVide titre="Aucun élève concerné" message="Aucun élève n'est affecté à cette classe ou à ce groupe."/> }.into_any()
        } else {
            view! {
                <ul class="cdt-liste">
                    {lignes
                        .into_iter()
                        .map(|l| view! { <li><LigneCopie ligne=l erreur=erreur on_change=on_change/></li> })
                        .collect_view()}
                </ul>
            }
            .into_any()
        }}
    }
}

#[component]
fn LigneCopie(
    ligne: LigneSuiviDto,
    erreur: RwSignal<Option<String>>,
    on_change: Callback<()>,
) -> impl IntoView {
    let nom = format!("{} {}", ligne.prenom, ligne.nom);
    let (libelle, ton) = libelle_remise(ligne.remise.as_ref());
    let remise = ligne.remise.clone();
    view! {
        <article class="cdt-copie">
            <header class="cdt-travail__entete">
                <h3 class="cdt-travail__titre">{nom.clone()}</h3>
                <Badge libelle=libelle ton=ton/>
            </header>
            {remise.map(|r| view! { <DetailCopie remise=r nom=nom erreur=erreur on_change=on_change/> })}
        </article>
    }
}

#[component]
fn DetailCopie(
    remise: RemiseDto,
    nom: String,
    erreur: RwSignal<Option<String>>,
    on_change: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let remise_id = remise.id;
    let corrigee = remise.statut == "corrige";
    let echec = gestionnaire_echec(erreur);
    let commentaire = RwSignal::new(remise.retour_commentaire.clone().unwrap_or_default());
    let err_commentaire = RwSignal::new(None::<String>);
    let fichier = RwSignal::new(None::<FichierChoisi>);
    let err_fichier = RwSignal::new(None::<String>);
    let champ = NodeRef::<leptos::html::Input>::new();
    let en_cours = RwSignal::new(false);

    let choisir = move |_| {
        let Some(input) = champ.get() else { return };
        spawn_local(async move {
            match lire_fichier_choisi(&input).await {
                Lecture::Aucun => fichier.set(None),
                Lecture::Fichier(f) => {
                    err_fichier.set(None);
                    fichier.set(Some(f));
                }
                Lecture::Refuse(m) => {
                    fichier.set(None);
                    input.set_value("");
                    err_fichier.set(Some(m));
                }
            }
        });
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        match valider_retour(&commentaire.get_untracked()) {
            Err(m) => err_commentaire.set(Some(m)),
            Ok(texte) => {
                err_commentaire.set(None);
                let depot = fichier
                    .get_untracked()
                    .map(|f| depot_fichier(&f.nom, &f.mime, &f.octets));
                en_cours.set(true);
                spawn_local(async move {
                    let r = session::client()
                        .cahier_retour(remise_id, &texte, depot)
                        .await;
                    en_cours.set(false);
                    match r {
                        Ok(_) => {
                            notifications.succes(
                                "Correction envoyée : l'élève et sa famille seront prévenus.",
                            );
                            on_change.run(());
                        }
                        Err(e) => echec.run(e),
                    }
                });
            }
        }
    };
    let pieces = remise.pieces.clone();
    let pref = format!("cdt-r-{remise_id}");
    let commentaire_eleve = remise.commentaire_eleve.clone();
    view! {
        {(!commentaire_eleve.is_empty()).then(|| view! { <p class="cdt-copie__mot">{commentaire_eleve}</p> })}
        <ul class="cdt-pieces">
            {pieces
                .into_iter()
                .map(|p| {
                    let pid = p.id;
                    let etiquette = format!(
                        "{} ({}){}",
                        p.nom,
                        libelle_taille(p.taille_octets),
                        if p.genre == "annotation" { " — copie annotée" } else { "" }
                    );
                    let aria = format!("Télécharger {} de {nom}", p.nom);
                    view! {
                        <li>
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria
                                on_click=Callback::new(move |_| ouvrir_piece(pid, erreur))>
                                {etiquette}
                            </Bouton>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
        <details class="cdt-seance__saisie">
            <summary>{if corrigee { "Modifier la correction" } else { "Corriger et renvoyer" }}</summary>
            <form class="cdt-form" on:submit=envoyer novalidate=true>
                <ZoneTexte id=format!("{pref}-commentaire") libelle="Commentaire de correction" valeur=commentaire erreur=err_commentaire requis=true/>
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for=format!("{pref}-fichier")>"Copie annotée (facultative)"</label>
                    <input id=format!("{pref}-fichier") class="ui-champ__controle" type="file" node_ref=champ
                        aria-describedby=format!("{pref}-fichier-aide") on:change=choisir/>
                    <p id=format!("{pref}-fichier-aide") class="ui-champ__aide">"10 Mo au maximum. Le contenu est contrôlé par le serveur."</p>
                    {move || err_fichier.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                    {move || fichier.get().map(|f| view! { <p class="ui-champ__aide">{format!("Fichier choisi : {}", f.nom)}</p> })}
                </div>
                <Bouton soumettre=true en_cours=en_cours>"Renvoyer la copie"</Bouton>
            </form>
        </details>
    }
}
