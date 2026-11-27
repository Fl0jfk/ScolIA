//! Infirmerie (socle 2, lot S2-11) : tableau du jour des passages, saisie d'un passage, clôture
//! (issue, renvoi au domicile contrôlé), information de la famille, PAI à réviser ; fiche d'un
//! élève (PAI complet, signatures, contraintes publiées) ; fiche d'urgence imprimable.
//!
//! Routes d'`ent-api/src/sante.rs`. Le serveur décide des droits (`sante.lire`, `sante.saisir`,
//! `sante.urgence_lire`) : l'écran n'est proposé qu'à qui les a (navigation par les droits du
//! moteur) et traite les refus (403). Les données de santé ne sont affichées qu'à l'écran :
//! rien n'est copié dans l'adresse, le stockage du navigateur ni les notifications.

pub mod modele;

use chrono::NaiveDate;
use ent_types::sante::{
    CatalogueSanteDto, FicheUrgenceDto, PaiCompletDto, PassageSanteDto, SignerPaiBody,
};
use ent_web_ui::{
    AccesRefuse, Alerte, Annonce, Badge, Bouton, Case, ChampTexte, Dialogue, EnTetePage, EtatVide,
    LienBouton, Niveau, Selection, Squelette, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map};
use uuid::Uuid;

use crate::composants_metier::recherche_eleve::RechercheEleve;
use crate::composants_metier::sante::{
    delai_texte, heure_affichee, libelle_audience, libelle_info_famille, libelle_issue,
    libelle_motif, libelle_partie, nom_eleve, ton_delai, ton_info_famille, ISSUES_CLOTURE,
    ISSUES_CREATION, MOTIFS,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_complete, date_longue};
use modele::{
    composer_cloture, composer_contrainte, composer_pai, composer_passage, ErreurChamp,
    SaisieCloture, SaisieContrainte, SaisiePai, SaisiePassage,
};

/// Adresse de la fiche santé d'un élève.
pub fn adresse_eleve(id: Uuid) -> String {
    format!("{}/eleves/{id}", navigation::INFIRMERIE)
}

fn erreur_de(erreurs: RwSignal<Vec<ErreurChamp>>, champ: &'static str) -> Signal<Option<String>> {
    Signal::derive(move || {
        erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
    })
}

fn options_vides(liste: &[(&str, &str)], premier: &str) -> Vec<(String, String)> {
    std::iter::once((String::new(), premier.to_string()))
        .chain(liste.iter().map(|(c, l)| (c.to_string(), l.to_string())))
        .collect()
}

// ---------------------------------------------------------------------------
// Tableau du jour
// ---------------------------------------------------------------------------

#[component]
pub fn PageInfirmerie() -> impl IntoView {
    let jour_texte = RwSignal::new(String::new());
    // Le jour est posé après l'hydratation : aucune date dans le HTML rendu côté serveur.
    Effect::new(move |_| {
        if jour_texte.get_untracked().is_empty() {
            jour_texte.set(aujourdhui().to_string());
        }
    });
    let jour =
        Memo::new(move |_| NaiveDate::parse_from_str(jour_texte.get().trim(), "%Y-%m-%d").ok());
    let passages = LocalResource::new(move || {
        let j = jour.get();
        async move {
            match j {
                Some(j) => session::client().passages_du_jour(j).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let a_reviser = LocalResource::new(|| async { session::client().pai_a_reviser(30).await });
    let recharger = Callback::new(move |_| {
        passages.refetch();
        a_reviser.refetch();
    });
    let ecriture = Ecriture::nouveau(recharger);
    let navigate = use_navigate();
    let moteur = use_droits_moteur();
    let ouvrir_dossier = Callback::new(move |(id, _): (Uuid, String)| {
        navigate(&adresse_eleve(id), Default::default());
    });
    let cloture = RwSignal::new(None::<PassageSanteDto>);

    view! {
        <Title text="Infirmerie"/>
        <EnTetePage
            surtitre="Santé".to_string()
            titre="Infirmerie".to_string()
            sous_titre="Passages du jour, PAI à réviser et dossiers de santé. Chaque consultation est journalisée.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        <div class="jvs-barre">
            <ChampTexte id="inf-jour" libelle="Jour" valeur=jour_texte genre="date"/>
            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| recharger.run(()))>
                "Actualiser"
            </Bouton>
        </div>
        <section class="ra-section" aria-labelledby="inf-passages">
            <h2 id="inf-passages" class="vs-section-titre">
                {move || jour.get().map(|j| format!("Passages — {}", date_longue(j))).unwrap_or_else(|| "Passages".into())}
            </h2>
            {move || match passages.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des passages…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| passages.refetch())/> }.into_any(),
                Some(Ok(None)) => view! { <EtatVide titre="Date invalide" message="Choisissez un jour valide."/> }.into_any(),
                Some(Ok(Some(l))) => view! { <ListePassages passages=l ecriture=ecriture cloture=cloture/> }.into_any(),
            }}
        </section>
        {move || moteur.get().peut_saisir_sante().then(|| view! {
            <section class="ra-section" aria-labelledby="inf-nouveau">
                <h2 id="inf-nouveau" class="vs-section-titre">"Enregistrer un passage"</h2>
                <NouveauPassage jour_texte=jour_texte ecriture=ecriture/>
            </section>
        })}
        <section class="ra-section" aria-labelledby="inf-reviser">
            <h2 id="inf-reviser" class="vs-section-titre">"PAI à réviser (30 jours)"</h2>
            {move || match a_reviser.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des PAI à réviser…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| a_reviser.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune révision en vue" message="Aucun PAI n'arrive à échéance dans les 30 jours."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|p| {
                            let nom = nom_eleve(&p.prenom, &p.nom);
                            let href = adresse_eleve(p.eleve_id);
                            view! {
                                <li class="jvs-ligne">
                                    <a href=href>{format!("{nom}{}", p.classe.as_deref().map(|c| format!(" · {c}")).unwrap_or_default())}</a>
                                    <Badge libelle=format!("Révision {}", delai_texte(p.jours_restants)) ton=ton_delai(p.jours_restants)/>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="inf-dossier">
            <h2 id="inf-dossier" class="vs-section-titre">"Ouvrir le dossier d'un élève"</h2>
            <RechercheEleve
                id="inf-eleve-q"
                libelle="Rechercher un élève (nom, prénom ou classe)"
                action="Ouvrir le dossier de santé de"
                on_choisir=ouvrir_dossier
            />
        </section>
        <DialogueCloture cloture=cloture ecriture=ecriture/>
    }
}

#[component]
fn ListePassages(
    passages: Vec<PassageSanteDto>,
    ecriture: Ecriture,
    cloture: RwSignal<Option<PassageSanteDto>>,
) -> impl IntoView {
    if passages.is_empty() {
        return view! { <EtatVide titre="Aucun passage" message="Aucun passage enregistré ce jour."/> }
            .into_any();
    }
    let moteur = use_droits_moteur();
    view! {
        <table class="ui-tableau">
            <caption class="visuellement-cache">"Passages à l'infirmerie"</caption>
            <thead>
                <tr>
                    <th scope="col">"Heure"</th>
                    <th scope="col">"Élève"</th>
                    <th scope="col">"Motif"</th>
                    <th scope="col">"Issue"</th>
                    <th scope="col">"Famille"</th>
                    <th scope="col">"Actions"</th>
                </tr>
            </thead>
            <tbody>
                {passages.into_iter().map(|p| {
                    let nom = nom_eleve(&p.prenom, &p.nom);
                    let (id, informee) = (p.id, p.famille_information == "informee");
                    let href = adresse_eleve(p.eleve_id);
                    let ouverte = p.heure_sortie.is_none() && p.issue == "non_renseignee";
                    let a = format!("Informer la famille du passage de {nom}");
                    let b = format!("Clôturer le passage de {nom}");
                    let pour_cloture = p.clone();
                    let detail = p.recupere_par_nom.clone().map(|n| format!(" ({n})")).unwrap_or_default();
                    view! {
                        <tr>
                            <td>{heure_affichee(p.heure_arrivee)}</td>
                            <td><a href=href>{nom.clone()}</a>{p.classe.clone().map(|c| format!(" · {c}"))}</td>
                            <td>{libelle_motif(&p.motif)}</td>
                            <td>{format!("{}{detail}", libelle_issue(&p.issue))}</td>
                            <td><Badge libelle=libelle_info_famille(&p.famille_information) ton=ton_info_famille(&p.famille_information)/></td>
                            <td>
                                {move || moteur.get().peut_saisir_sante().then(|| {
                                    let (a, b, pc) = (a.clone(), b.clone(), pour_cloture.clone());
                                    view! {
                                        <div class="ra-actions">
                                            {(!informee).then(|| view! {
                                                <Bouton
                                                    variante=Variante::Secondaire
                                                    taille=Taille::Compacte
                                                    en_cours=ecriture.en_cours
                                                    libelle_accessible=a
                                                    on_click=Callback::new(move |_| {
                                                        ecriture.lancer(async move {
                                                            session::client().informer_famille_passage(id).await?;
                                                            Ok("La famille a été informée (sans détail médical).".to_string())
                                                        });
                                                    })
                                                >
                                                    "Informer la famille"
                                                </Bouton>
                                            })}
                                            <Bouton
                                                variante=if ouverte { Variante::Primaire } else { Variante::Discret }
                                                taille=Taille::Compacte
                                                libelle_accessible=b
                                                on_click=Callback::new(move |_| cloture.set(Some(pc.clone())))
                                            >
                                                "Clôturer"
                                            </Bouton>
                                        </div>
                                    }
                                })}
                            </td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Nouveau passage
// ---------------------------------------------------------------------------

#[component]
fn NouveauPassage(jour_texte: RwSignal<String>, ecriture: Ecriture) -> impl IntoView {
    let eleve = RwSignal::new(None::<(Uuid, String)>);
    let (heure, motif, note, soins, issue) = (
        RwSignal::new(String::new()),
        RwSignal::new("soins".to_string()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new("non_renseignee".to_string()),
    );
    let (visible, informer) = (RwSignal::new(false), RwSignal::new(false));
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisiePassage {
            eleve: eleve.get_untracked().map(|(id, _)| id),
            jour: jour_texte.get_untracked(),
            heure: heure.get_untracked(),
            motif: motif.get_untracked(),
            note_soins: note.get_untracked(),
            soins: soins.get_untracked(),
            issue: issue.get_untracked(),
            visible_famille: visible.get_untracked(),
            informer_famille: informer.get_untracked(),
        };
        match composer_passage(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    let p = session::client().creer_passage_sante(&corps).await?;
                    Ok(if p.famille_information == "informee" {
                        "Passage enregistré ; la famille a été informée (sans détail médical)."
                            .to_string()
                    } else {
                        "Passage enregistré.".to_string()
                    })
                });
                note.set(String::new());
                soins.set(String::new());
                heure.set(String::new());
                eleve.set(None);
            }
        }
    };
    view! {
        {move || match eleve.get() {
            None => view! {
                <RechercheEleve
                    id="pas-eleve-q"
                    libelle="Élève (nom, prénom ou classe)"
                    action="Enregistrer un passage pour"
                    on_choisir=Callback::new(move |e| eleve.set(Some(e)))
                />
                {move || erreur_de(erreurs, "pas-eleve").get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            }.into_any(),
            Some((_, nom)) => view! {
                <p>
                    <strong>{nom}</strong>
                    " "
                    <Bouton variante=Variante::Discret taille=Taille::Compacte libelle_accessible="Changer d'élève"
                        on_click=Callback::new(move |_| eleve.set(None))>"Changer d'élève"</Bouton>
                </p>
                <form class="ra-formulaire" on:submit=envoyer novalidate aria-label="Nouveau passage à l'infirmerie">
                    <ChampTexte id="pas-jour" libelle="Jour" valeur=jour_texte genre="date" requis=true erreur=erreur_de(erreurs, "pas-jour")/>
                    <ChampTexte id="pas-heure" libelle="Heure d'arrivée (facultatif)" valeur=heure genre="time" erreur=erreur_de(erreurs, "pas-heure")/>
                    <Selection id="pas-motif" libelle="Motif" valeur=motif options=options_vides(&MOTIFS, "— choisir —") requis=true erreur=erreur_de(erreurs, "pas-motif")/>
                    <ZoneTexte id="pas-note" libelle="Note clinique (confidentielle, jamais montrée à la famille)" valeur=note erreur=erreur_de(erreurs, "pas-note")/>
                    <ZoneTexte id="pas-soins" libelle="Soins donnés" valeur=soins/>
                    <Selection id="pas-issue" libelle="Issue" valeur=issue options=options_vides(&ISSUES_CREATION, "— à préciser —")/>
                    <Case id="pas-visible" libelle="Le passage apparaît dans l'espace famille (sans note ni soins)" coche=visible/>
                    <Case id="pas-informer" libelle="Informer la famille maintenant (un accident ou un appel de la famille l'impose déjà)" coche=informer/>
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer le passage"</Bouton>
                </form>
            }.into_any(),
        }}
    }
}

// ---------------------------------------------------------------------------
// Clôture d'un passage
// ---------------------------------------------------------------------------

#[component]
fn DialogueCloture(
    cloture: RwSignal<Option<PassageSanteDto>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let ouvert = RwSignal::new(false);
    crate::noyau::dialogue::lier_ouverture(ouvert, cloture);
    let (heure, issue, soins, personne) = (
        RwSignal::new(String::new()),
        RwSignal::new("retour_classe".to_string()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    );
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    // La fiche d'urgence (consultation journalisée) n'est chargée que pour un renvoi au domicile :
    // elle fournit les personnes habilitées à récupérer l'élève.
    let fiche = LocalResource::new(move || {
        let demande = cloture.get().filter(|_| issue.get() == "renvoi_domicile");
        async move {
            match demande {
                Some(p) => session::client().fiche_urgence(p.eleve_id).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(p) = cloture.get_untracked() else {
            return;
        };
        let saisie = SaisieCloture {
            heure_sortie: heure.get_untracked(),
            issue: issue.get_untracked(),
            soins: soins.get_untracked(),
            personne: personne.get_untracked(),
        };
        match composer_cloture(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                let id = p.id;
                ecriture.lancer(async move {
                    session::client().cloturer_passage(id, &corps).await?;
                    Ok("Passage clôturé.".to_string())
                });
                ouvert.set(false);
            }
        }
    };
    view! {
        <Dialogue ouvert=ouvert titre="Clôturer le passage" id="dlg-cloture">
            <form class="ra-formulaire" on:submit=envoyer novalidate>
                {move || cloture.get().map(|p| view! {
                    <p class="texte-attenue">{format!("{} — {}", nom_eleve(&p.prenom, &p.nom), libelle_motif(&p.motif))}</p>
                })}
                <Selection id="clo-issue" libelle="Issue" valeur=issue options=options_vides(&ISSUES_CLOTURE, "— choisir —") requis=true erreur=erreur_de(erreurs, "clo-issue")/>
                <ChampTexte id="clo-heure" libelle="Heure de sortie (facultatif)" valeur=heure genre="time" erreur=erreur_de(erreurs, "clo-heure")/>
                {move || (issue.get() == "renvoi_domicile").then(|| view! {
                    {match fiche.get() {
                        None => view! { <Squelette lignes=2 libelle="Chargement des personnes habilitées…"/> }.into_any(),
                        Some(Err(e)) => view! { <Alerte niveau=Niveau::Erreur titre=e.message_utilisateur()/> }.into_any(),
                        Some(Ok(None)) => ().into_any(),
                        Some(Ok(Some(f))) => {
                            let options = personnes_habilitees(&f);
                            view! {
                                <Selection id="clo-personne" libelle="Personne qui récupère l'élève" valeur=personne options=options requis=true erreur=erreur_de(erreurs, "clo-personne")/>
                            }.into_any()
                        }
                    }}
                })}
                <ZoneTexte id="clo-soins" libelle="Soins complémentaires (facultatif)" valeur=soins/>
                <div class="ra-actions">
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Clôturer"</Bouton>
                    <Bouton variante=Variante::Discret on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Responsables (liens actifs) et contacts habilités à récupérer l'élève.
fn personnes_habilitees(f: &FicheUrgenceDto) -> Vec<(String, String)> {
    std::iter::once((String::new(), "— choisir —".to_string()))
        .chain(
            f.responsables
                .iter()
                .chain(f.contacts.iter())
                .filter(|p| p.peut_recuperer)
                .map(|p| {
                    (
                        p.personne_id.to_string(),
                        format!("{} ({})", p.nom, p.relation),
                    )
                }),
        )
        .collect()
}

// ---------------------------------------------------------------------------
// Fiche santé d'un élève : PAI complet, signatures, contraintes publiées
// ---------------------------------------------------------------------------

fn id_param() -> Memo<Option<Uuid>> {
    let params = use_params_map();
    Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|i| i.parse::<Uuid>().ok())
    })
}

#[component]
pub fn PageInfirmerieEleve() -> impl IntoView {
    let id = id_param();
    view! {
        <Title text="Dossier de santé"/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::INFIRMERIE>"Infirmerie"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">"Dossier de santé"</span>
        </nav>
        {move || match id.get() {
            None => view! { <EtatVide titre="Élève introuvable" message="Identifiant d'élève invalide."/> }.into_any(),
            Some(id) => view! { <DossierSante eleve_id=id/> }.into_any(),
        }}
    }
}

#[component]
fn DossierSante(eleve_id: Uuid) -> impl IntoView {
    let pai =
        LocalResource::new(move || async move { session::client().pai_complet(eleve_id).await });
    let contraintes = LocalResource::new(move || async move {
        session::client().contraintes_eleve(eleve_id, None).await
    });
    let passages =
        LocalResource::new(move || async move { session::client().passages_eleve(eleve_id).await });
    let catalogue = LocalResource::new(|| async { session::client().catalogue_sante().await });
    let recharger = Callback::new(move |_| {
        pai.refetch();
        contraintes.refetch();
        passages.refetch();
    });
    let ecriture = Ecriture::nouveau(recharger);
    let moteur = use_droits_moteur();
    view! {
        <RetourEcriture ecriture=ecriture/>
        <EnTetePage titre="Dossier de santé".to_string() sous_titre="Données confidentielles : chaque consultation est journalisée.".to_string()>
            <LienBouton href=format!("{}/eleves/{eleve_id}/urgence", navigation::INFIRMERIE) variante=Variante::Secondaire>
                "Fiche d'urgence"
            </LienBouton>
        </EnTetePage>
        <section class="ra-section" aria-labelledby="san-pai">
            <h2 id="san-pai" class="vs-section-titre">"Projet d'accueil individualisé (PAI)"</h2>
            {move || match pai.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement du PAI…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| pai.refetch())/> }.into_any(),
                Some(Ok(p)) => view! {
                    <FichePai pai=p eleve_id=eleve_id ecriture=ecriture/>
                }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="san-contraintes">
            <h2 id="san-contraintes" class="vs-section-titre">"Contraintes publiées (sans donnée médicale)"</h2>
            <p class="texte-attenue">
                "Seules ces contraintes sont communiquées à la restauration, aux enseignants et aux sorties : \
                 jamais le diagnostic, les traitements ni le protocole."
            </p>
            {move || match (contraintes.get(), catalogue.get()) {
                (None, _) | (_, None) => view! { <Squelette lignes=3 libelle="Chargement des contraintes…"/> }.into_any(),
                (Some(Err(e)), _) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| contraintes.refetch())/> }.into_any(),
                (_, Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| catalogue.refetch())/> }.into_any(),
                (Some(Ok(l)), Some(Ok(cat))) => view! {
                    <ListeContraintes contraintes=l ecriture=ecriture/>
                    {move || moteur.get().peut_saisir_sante().then(|| view! { <FormulaireContrainte eleve_id=eleve_id catalogue=cat.clone() ecriture=ecriture/> })}
                }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="san-passages">
            <h2 id="san-passages" class="vs-section-titre">"Passages à l'infirmerie"</h2>
            {move || match passages.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des passages…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| passages.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun passage" message="Aucun passage enregistré pour cet élève."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|p| view! {
                            <li class="jvs-ligne">
                                <div>
                                    <strong>{date_complete(p.jour)}</strong>
                                    <span class="texte-attenue">{format!(" — {} · {}", libelle_motif(&p.motif), libelle_issue(&p.issue))}</span>
                                </div>
                                <Badge libelle=libelle_info_famille(&p.famille_information) ton=ton_info_famille(&p.famille_information)/>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn FichePai(pai: Option<PaiCompletDto>, eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let modifier = RwSignal::new(false);
    let existe = pai.is_some();
    let pai_stocke = StoredValue::new(pai.clone());
    let resume = pai.clone().map(|p| {
        let revision = p.date_revision.map(|d| {
            let jours = (d - aujourdhui()).num_days() as i32;
            (d, jours)
        });
        view! {
            <dl class="ra-definitions">
                <dt>"Statut"</dt>
                <dd>
                    <Badge libelle=if p.pai.actif { "Actif" } else { "Inactif" } ton=if p.pai.actif { TonEtat::Ok } else { TonEtat::Neutre }/>
                    " "
                    <Badge libelle=format!("Version {}", p.version) ton=TonEtat::Neutre/>
                </dd>
                <dt>"Période"</dt>
                <dd>{format!("depuis le {}{}", date_complete(p.pai.date_debut), p.pai.date_fin.map(|f| format!(" jusqu'au {}", date_complete(f))).unwrap_or_default())}</dd>
                <dt>"Motif"</dt>
                <dd>{p.pai.motif.clone()}</dd>
                <dt>"Mesures d'accueil"</dt>
                <dd>{p.pai.mesures_accueil.clone()}</dd>
                <dt>"Protocole d'urgence"</dt>
                <dd>{if p.pai.protocole_urgence.is_empty() { "—".to_string() } else { p.pai.protocole_urgence.clone() }}</dd>
                <dt>"Trousse d'urgence"</dt>
                <dd>
                    {if p.trousse_urgence { format!("Oui — {}", p.trousse_lieu) } else { "Non".to_string() }}
                    {p.trousse_perimee.then(|| view! { " " <Badge libelle="Trousse périmée" ton=TonEtat::Mal/> })}
                </dd>
                <dt>"Révision annuelle"</dt>
                <dd>
                    {match revision {
                        Some((d, j)) => view! { {date_complete(d)} " " <Badge libelle=delai_texte(j) ton=ton_delai(j)/> }.into_any(),
                        None => "—".into_any(),
                    }}
                </dd>
            </dl>
            <h3 class="jvs-sous-titre">"Signatures de la version en cours"</h3>
            <SignaturesPai pai=p eleve_id=eleve_id ecriture=ecriture/>
        }
    });
    view! {
        {match resume {
            Some(v) => v.into_any(),
            None => view! { <EtatVide titre="Aucun PAI" message="Aucun projet d'accueil individualisé n'est enregistré pour cet élève."/> }.into_any(),
        }}
        {move || moteur.get().peut_saisir_sante().then(|| view! {
            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| modifier.update(|m| *m = !*m))>
                {move || if modifier.get() { "Fermer le formulaire" } else if existe { "Modifier le PAI" } else { "Créer le PAI" }}
            </Bouton>
        })}
        {move || (modifier.get() && moteur.get().peut_saisir_sante()).then(|| view! {
            <FormulairePai eleve_id=eleve_id existant=pai_stocke.get_value() ecriture=ecriture modifier=modifier/>
        })}
    }
}

#[component]
fn SignaturesPai(pai: PaiCompletDto, eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let nom_signataire = RwSignal::new(String::new());
    let partie = RwSignal::new("medecin".to_string());
    let signees: Vec<String> = pai.signatures.iter().map(|s| s.partie.clone()).collect();
    let manquantes = pai.parties_manquantes.clone();
    let options: Vec<(String, String)> =
        ["medecin", "infirmier", "chef_etablissement", "collectivite"]
            .iter()
            .filter(|p| !signees.iter().any(|s| s == *p))
            .map(|p| (p.to_string(), libelle_partie(p).to_string()))
            .collect();
    let peut_signer = !options.is_empty() && pai.pai.actif;
    let version = pai.version;
    let envoyer = Callback::new(move |_| {
        let corps = SignerPaiBody {
            partie: partie.get_untracked(),
            signataire_nom: Some(nom_signataire.get_untracked().trim().to_string())
                .filter(|n| !n.is_empty()),
            version: Some(version),
        };
        ecriture.lancer(async move {
            session::client().signer_pai(eleve_id, &corps).await?;
            Ok("Signature enregistrée.".to_string())
        });
        nom_signataire.set(String::new());
    });
    view! {
        <ul class="jvs-liste">
            {pai.signatures.iter().map(|s| view! {
                <li class="jvs-ligne">
                    <span><strong>{libelle_partie(&s.partie)}</strong>{format!(" — {}", s.signataire_nom)}</span>
                    <Badge libelle="Signé" ton=TonEtat::Ok/>
                </li>
            }).collect_view()}
            {manquantes.iter().map(|p| view! {
                <li class="jvs-ligne">
                    <span>{libelle_partie(p)}</span>
                    <Badge libelle="À signer" ton=TonEtat::Avertissement/>
                </li>
            }).collect_view()}
        </ul>
        {if pai.complet {
            view! { <Alerte niveau=Niveau::Succes titre="PAI signé par toutes les parties requises."/> }.into_any()
        } else {
            view! { <p class="texte-attenue">"Toute modification du PAI invalide les signatures : elles sont à refaire."</p> }.into_any()
        }}
        {move || (peut_signer && moteur.get().peut_saisir_sante()).then(|| view! {
            <div class="ra-formulaire">
                <p class="texte-attenue">"Consigner la signature d'une partie (médecin, direction, collectivité) : le nom du signataire est obligatoire. La famille signe depuis son espace."</p>
                <Selection id="sig-partie" libelle="Partie" valeur=partie options=options.clone()/>
                <ChampTexte id="sig-nom" libelle="Nom du signataire" valeur=nom_signataire requis=true/>
                <Bouton en_cours=ecriture.en_cours desactive=Signal::derive(move || nom_signataire.get().trim().is_empty()) on_click=envoyer>
                    "Consigner la signature"
                </Bouton>
            </div>
        })}
    }
}

#[component]
fn FormulairePai(
    eleve_id: Uuid,
    existant: Option<PaiCompletDto>,
    ecriture: Ecriture,
    modifier: RwSignal<bool>,
) -> impl IntoView {
    let p = existant.as_ref();
    let (motif, mesures, protocole, note) = (
        RwSignal::new(p.map(|p| p.pai.motif.clone()).unwrap_or_default()),
        RwSignal::new(p.map(|p| p.pai.mesures_accueil.clone()).unwrap_or_default()),
        RwSignal::new(
            p.map(|p| p.pai.protocole_urgence.clone())
                .unwrap_or_default(),
        ),
        RwSignal::new(p.map(|p| p.pai.note_interne.clone()).unwrap_or_default()),
    );
    let (debut, fin, revision, peremption, lieu) = (
        RwSignal::new(p.map(|p| p.pai.date_debut.to_string()).unwrap_or_default()),
        RwSignal::new(
            p.and_then(|p| p.pai.date_fin)
                .map(|d| d.to_string())
                .unwrap_or_default(),
        ),
        RwSignal::new(
            p.and_then(|p| p.date_revision)
                .map(|d| d.to_string())
                .unwrap_or_default(),
        ),
        RwSignal::new(
            p.and_then(|p| p.trousse_peremption)
                .map(|d| d.to_string())
                .unwrap_or_default(),
        ),
        RwSignal::new(p.map(|p| p.trousse_lieu.clone()).unwrap_or_default()),
    );
    let (actif, trousse) = (
        RwSignal::new(p.map(|p| p.pai.actif).unwrap_or(true)),
        RwSignal::new(p.map(|p| p.trousse_urgence).unwrap_or(false)),
    );
    // Création : le début est posé après l'hydratation (aucune date dans le HTML du serveur).
    Effect::new(move |_| {
        if debut.get_untracked().is_empty() {
            debut.set(aujourdhui().to_string());
        }
    });
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisiePai {
            motif: motif.get_untracked(),
            mesures: mesures.get_untracked(),
            protocole: protocole.get_untracked(),
            note_interne: note.get_untracked(),
            debut: debut.get_untracked(),
            fin: fin.get_untracked(),
            actif: actif.get_untracked(),
            revision: revision.get_untracked(),
            trousse: trousse.get_untracked(),
            trousse_lieu: lieu.get_untracked(),
            trousse_peremption: peremption.get_untracked(),
        };
        match composer_pai(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client().enregistrer_pai(eleve_id, &corps).await?;
                    Ok("PAI enregistré ; les signatures sont à refaire.".to_string())
                });
                modifier.set(false);
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-label="Projet d'accueil individualisé">
            <Alerte niveau=Niveau::Info titre="Enregistrer un PAI crée une nouvelle version : les signatures précédentes ne comptent plus."/>
            <ZoneTexte id="pai-motif" libelle="Motif (confidentiel)" valeur=motif requis=true erreur=erreur_de(erreurs, "pai-motif")/>
            <ZoneTexte id="pai-mesures" libelle="Mesures d'accueil" valeur=mesures requis=true erreur=erreur_de(erreurs, "pai-mesures")/>
            <ZoneTexte id="pai-protocole" libelle="Protocole d'urgence" valeur=protocole/>
            <ZoneTexte id="pai-note" libelle="Note interne (jamais montrée à la famille)" valeur=note/>
            <ChampTexte id="pai-debut" libelle="Début" valeur=debut genre="date" requis=true erreur=erreur_de(erreurs, "pai-debut")/>
            <ChampTexte id="pai-fin" libelle="Fin (facultatif)" valeur=fin genre="date" erreur=erreur_de(erreurs, "pai-fin")/>
            <ChampTexte id="pai-revision" libelle="Prochaine révision (un an après le début par défaut)" valeur=revision genre="date" erreur=erreur_de(erreurs, "pai-revision")/>
            <Case id="pai-trousse" libelle="Trousse d'urgence" coche=trousse/>
            {move || trousse.get().then(|| view! {
                <ChampTexte id="pai-lieu" libelle="Lieu de rangement de la trousse" valeur=lieu requis=true erreur=erreur_de(erreurs, "pai-lieu")/>
                <ChampTexte id="pai-peremption" libelle="Péremption de la trousse (facultatif)" valeur=peremption genre="date" erreur=erreur_de(erreurs, "pai-peremption")/>
            })}
            <Case id="pai-actif" libelle="PAI actif" coche=actif/>
            <div class="ra-actions">
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer le PAI"</Bouton>
                <Bouton variante=Variante::Discret on_click=Callback::new(move |_| modifier.set(false))>"Annuler"</Bouton>
            </div>
        </form>
    }
}

#[component]
fn ListeContraintes(
    contraintes: Vec<ent_types::sante::ContraintePublieeDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    if contraintes.is_empty() {
        return view! { <EtatVide titre="Aucune contrainte publiée" message="Rien n'est communiqué aux autres services pour cet élève."/> }
            .into_any();
    }
    let moteur = use_droits_moteur();
    view! {
        <ul class="jvs-liste">
            {contraintes.into_iter().map(|c| {
                let id = c.id;
                let a = format!("Retirer la contrainte « {} »", c.libelle);
                let dest = c.audiences.iter().map(|a| libelle_audience(a)).collect::<Vec<_>>().join(", ");
                let origine = match c.source.as_deref() {
                    Some("pai") => " · issue du PAI",
                    _ => "",
                };
                view! {
                    <li class="jvs-ligne">
                        <div>
                            <strong>{c.libelle.clone()}</strong>
                            <span class="texte-attenue">{format!(" — pour : {dest}{origine}")}</span>
                        </div>
                        {move || moteur.get().peut_saisir_sante().then(|| {
                            let a = a.clone();
                            view! {
                                <Bouton variante=Variante::Discret taille=Taille::Compacte en_cours=ecriture.en_cours libelle_accessible=a
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().retirer_contrainte(id).await?;
                                            Ok("Contrainte retirée.".to_string())
                                        });
                                    })>
                                    "Retirer"
                                </Bouton>
                            }
                        })}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn FormulaireContrainte(
    eleve_id: Uuid,
    catalogue: CatalogueSanteDto,
    ecriture: Ecriture,
) -> impl IntoView {
    let code = RwSignal::new(String::new());
    let (restauration, enseignants, sorties) = (
        RwSignal::new(false),
        RwSignal::new(false),
        RwSignal::new(false),
    );
    let fin = RwSignal::new(String::new());
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let options: Vec<(String, String)> =
        std::iter::once((String::new(), "— choisir —".to_string()))
            .chain(
                catalogue
                    .contraintes
                    .iter()
                    .map(|c| (c.code.clone(), c.libelle.clone())),
            )
            .collect();
    let catalogue = StoredValue::new(catalogue);
    let permises = move || {
        catalogue.with_value(|c| {
            c.contraintes
                .iter()
                .find(|e| e.code == code.get())
                .map(|e| e.audiences.clone())
                .unwrap_or_default()
        })
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieContrainte {
            code: code.get_untracked(),
            restauration: restauration.get_untracked(),
            enseignants: enseignants.get_untracked(),
            sorties: sorties.get_untracked(),
            fin: fin.get_untracked(),
        };
        match composer_contrainte(&saisie, &permises()) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client()
                        .publier_contrainte(eleve_id, &corps)
                        .await?;
                    Ok("Contrainte publiée.".to_string())
                });
                code.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="con-titre">
            <h3 id="con-titre" class="jvs-sous-titre">"Publier une contrainte"</h3>
            <Selection id="con-code" libelle="Contrainte" valeur=code options=options requis=true erreur=erreur_de(erreurs, "con-code")/>
            <fieldset>
                <legend>"Destinataires"</legend>
                {move || {
                    let p = permises();
                    view! {
                        {p.iter().any(|a| a == "restauration").then(|| view! { <Case id="con-rest" libelle="Restauration" coche=restauration/> })}
                        {p.iter().any(|a| a == "enseignants").then(|| view! { <Case id="con-ens" libelle="Enseignants" coche=enseignants/> })}
                        {p.iter().any(|a| a == "sorties").then(|| view! { <Case id="con-sor" libelle="Sorties et séjours" coche=sorties/> })}
                    }
                }}
                {move || erreur_de(erreurs, "con-audiences").get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            </fieldset>
            <ChampTexte id="con-fin" libelle="Jusqu'au (facultatif)" valeur=fin genre="date" erreur=erreur_de(erreurs, "con-fin")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Publier"</Bouton>
        </form>
    }
}

// ---------------------------------------------------------------------------
// Fiche d'urgence imprimable
// ---------------------------------------------------------------------------

#[component]
pub fn PageFicheUrgence() -> impl IntoView {
    let id = id_param();
    let fiche = LocalResource::new(move || {
        let id = id.get();
        async move {
            match id {
                Some(id) => session::client().fiche_urgence(id).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let annonce = RwSignal::new(String::new());
    view! {
        <Title text="Fiche d'urgence"/>
        <Annonce message=Signal::derive(move || annonce.get())/>
        {move || match fiche.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la fiche d'urgence…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fiche.refetch())/> }.into_any(),
            Some(Ok(None)) => view! { <AccesRefuse message="Identifiant d'élève invalide."/> }.into_any(),
            Some(Ok(Some(f))) => view! { <ContenuFicheUrgence f=f annonce=annonce/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFicheUrgence(f: FicheUrgenceDto, annonce: RwSignal<String>) -> impl IntoView {
    let complete = f.niveau == "complete";
    let titre = format!("Fiche d'urgence — {}", nom_eleve(&f.prenom, &f.nom));
    let imprimer = Callback::new(move |_| {
        annonce.set("Ouverture de la fenêtre d'impression.".into());
        #[cfg(feature = "hydrate")]
        if let Some(w) = web_sys::window() {
            let _ = w.print();
        }
    });
    view! {
        <article class="fiche-urgence" aria-labelledby="urg-titre">
            <header class="fiche-urgence__entete">
                <h1 id="urg-titre">{titre}</h1>
                <p class="texte-attenue">
                    {f.classe.clone().unwrap_or_default()}
                    {f.date_naissance.map(|d| format!(" · né(e) le {}", date_complete(d)))}
                    {format!(" · niveau {}", if complete { "complet (personnel de santé)" } else { "minimal (encadrement)" })}
                </p>
                <div class="fiche-urgence__actions">
                    <Bouton on_click=imprimer>"Imprimer"</Bouton>
                </div>
            </header>
            <Alerte niveau=Niveau::Avertissement titre=f.consigne_generale.clone()/>
            <section aria-labelledby="urg-resp">
                <h2 id="urg-resp">"Responsables"</h2>
                <ListePersonnes personnes=f.responsables.clone()/>
            </section>
            <section aria-labelledby="urg-contacts">
                <h2 id="urg-contacts">"Autres contacts d'urgence"</h2>
                <ListePersonnes personnes=f.contacts.clone()/>
            </section>
            <section aria-labelledby="urg-contraintes">
                <h2 id="urg-contraintes">"À respecter"</h2>
                {if f.contraintes.is_empty() && !f.trousse_urgence {
                    view! { <p>"Aucune contrainte publiée."</p> }.into_any()
                } else {
                    view! {
                        <ul>
                            {f.trousse_urgence.then(|| view! { <li><strong>"Trousse d'urgence à emporter"</strong>{format!(" — {}", f.trousse_lieu)}</li> })}
                            {f.contraintes.iter().filter(|c| c.code != "trousse_urgence").map(|c| view! { <li>{c.libelle.clone()}</li> }).collect_view()}
                        </ul>
                    }.into_any()
                }}
            </section>
            {complete.then(|| view! {
                <section aria-labelledby="urg-sante">
                    <h2 id="urg-sante">"Conduite à tenir (personnel de santé)"</h2>
                    <p>{f.protocole_urgence.clone().unwrap_or_else(|| "Aucun protocole d'urgence enregistré.".to_string())}</p>
                    <p>
                        {f.medecin_nom.clone().map(|m| format!("Médecin : {m}. "))}
                        {f.urgence_tel.clone().map(|t| format!("Téléphone d'urgence : {t}."))}
                    </p>
                </section>
            })}
            <footer class="texte-attenue">
                "Document confidentiel, à ne pas conserver : détruire après la sortie ou le séjour. \
                 Consultation journalisée."
            </footer>
        </article>
    }
}

#[component]
fn ListePersonnes(personnes: Vec<ent_types::sante::PersonneUrgenceDto>) -> impl IntoView {
    if personnes.is_empty() {
        return view! { <p>"Aucune personne enregistrée."</p> }.into_any();
    }
    view! {
        <ul>
            {personnes.into_iter().map(|p| {
                let tel = [p.telephone.clone(), p.telephone_2.clone()]
                    .into_iter()
                    .flatten()
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
                    .join(" / ");
                view! {
                    <li>
                        <strong>{p.nom.clone()}</strong>
                        {format!(" ({})", p.relation)}
                        {(!tel.is_empty()).then(|| format!(" — {tel}"))}
                        {p.peut_recuperer.then_some(" · peut récupérer l'élève")}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
    .into_any()
}
