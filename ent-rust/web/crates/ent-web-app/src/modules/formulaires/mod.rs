//! Formulaires, autorisations et consentements (D24, FRM-01→05).
//!
//! - famille / élève majeur : liste des formulaires à traiter pour chaque enfant, décision
//!   (accepter, refuser, retirer) ; l'effet d'un refus du droit à l'image est annoncé ;
//! - personnel (`formulaires.gerer`, `consentements.lire`) : création, publication, archivage,
//!   résultats agrégés (jamais d'identité) et lecture « qui consent » pour un objet et une classe.
//!
//! Le serveur décide qui peut signer et applique la décision (photo masquée au trombinoscope).

pub mod modele;

use ent_types::communication::{FormulaireDto, FormulaireFamilleDto};
use ent_web_ui::{
    Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Selection, Squelette, Taille, Variante,
    ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    a_traiter, composer, gestes, libelle_decision, libelle_statut, libelle_type, nb_refus,
    publiable, rappel_effet, reponse, ton_decision, ton_statut, SaisieFormulaire,
    OBJET_DROIT_IMAGE,
};

#[component]
pub fn PageFormulaires() -> impl IntoView {
    let moteur = use_droits_moteur();
    let mes = LocalResource::new(|| async { session::client().formulaires_famille().await });
    view! {
        <Title text="Formulaires et consentements"/>
        <EnTetePage
            surtitre="Communication".to_string()
            titre="Formulaires et consentements".to_string()
            sous_titre="Vos décisions sont datées et conservées ; vous pouvez les modifier à tout moment.".to_string()
        />
        {move || match mes.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des formulaires…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| mes.refetch())/>
            }
            .into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucun formulaire à traiter" message="Aucun formulaire ne vous attend pour le moment."/>
            }
            .into_any(),
            Some(Ok(l)) => view! { <FormulairesFamille formulaires=l mes=mes/> }.into_any(),
        }}
        {move || moteur.get().a("formulaires.gerer").then(|| view! { <GestionFormulaires/> })}
        {move || moteur.get().a("consentements.lire").then(|| view! { <QuiConsent/> })}
    }
}

#[component]
fn FormulairesFamille(
    formulaires: Vec<FormulaireFamilleDto>,
    mes: LocalResource<Result<Vec<FormulaireFamilleDto>, ent_web_api::ApiError>>,
) -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(move |_| mes.refetch()));
    let n = a_traiter(&formulaires);
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="form-mes">
            <h2 id="form-mes" class="vs-section-titre">{format!("À traiter ({n})")}</h2>
            <ul class="jvs-liste">
                {formulaires.into_iter().map(|f| {
                    let (id, eleve) = (f.formulaire_id, f.eleve_id);
                    let g = gestes(&f);
                    let ma = f.ma_decision.clone().unwrap_or_default();
                    let effet = rappel_effet(&f);
                    let etat = f.etat.clone();
                    let intitule = format!("{} — {}", f.titre, f.eleve_prenom);
                    let (a1, a2, a3) = (
                        format!("Accepter : {intitule}"),
                        format!("Refuser : {intitule}"),
                        format!("Retirer mon accord : {intitule}"),
                    );
                    view! {
                        <li class="jvs-ligne">
                            <div>
                                <strong>{format!("{} · {}", f.titre, f.eleve_prenom)}</strong>
                                <span class="texte-attenue">{format!(" · {} (version {})", libelle_type(&f.type_formulaire), f.version)}</span>
                                <p>{f.texte.clone()}</p>
                                {effet.map(|e| view! { <p class="texte-attenue">{e}</p> })}
                                {etat.map(|e| view! { <p class="texte-attenue">{format!("Décision retenue pour l'élève : {}", libelle_decision(&e))}</p> })}
                            </div>
                            <Badge libelle=libelle_decision(&ma) ton=ton_decision(&ma)/>
                            <div class="ra-actions">
                                {g.accepter.then(|| view! {
                                    <Bouton variante=Variante::Primaire taille=Taille::Compacte en_cours=ecriture.en_cours
                                        libelle_accessible=a1
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client().repondre_formulaire(id, &reponse(eleve, "accepte")).await?;
                                                Ok("Décision enregistrée : accepté.".to_string())
                                            });
                                        })>"Accepter"</Bouton>
                                })}
                                {g.refuser.then(|| view! {
                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours
                                        libelle_accessible=a2
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client().repondre_formulaire(id, &reponse(eleve, "refuse")).await?;
                                                Ok("Décision enregistrée : refusé.".to_string())
                                            });
                                        })>"Refuser"</Bouton>
                                })}
                                {g.retirer.then(|| view! {
                                    <Bouton variante=Variante::Discret taille=Taille::Compacte en_cours=ecriture.en_cours
                                        libelle_accessible=a3
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client().repondre_formulaire(id, &reponse(eleve, "retire")).await?;
                                                Ok("Accord retiré.".to_string())
                                            });
                                        })>"Retirer mon accord"</Bouton>
                                })}
                            </div>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </section>
    }
}

#[component]
fn GestionFormulaires() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().formulaires().await });
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    let type_f = RwSignal::new("consentement".to_string());
    let objet = RwSignal::new(OBJET_DROIT_IMAGE.to_string());
    let titre = RwSignal::new(String::new());
    let texte = RwSignal::new(String::new());
    let audience = RwSignal::new("etablissement".to_string());
    let classe = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    view! {
        <section class="ra-section" aria-labelledby="form-gestion">
            <h2 id="form-gestion" class="vs-section-titre">"Gérer les formulaires"</h2>
            <RetourEcriture ecriture=ecriture/>
            <form
                on:submit=move |ev| {
                    ev.prevent_default();
                    let s = SaisieFormulaire {
                        type_formulaire: type_f.get_untracked(),
                        objet: objet.get_untracked(),
                        titre: titre.get_untracked(),
                        texte: texte.get_untracked(),
                        audience: audience.get_untracked(),
                        classe_id: classe.get_untracked(),
                    };
                    match composer(&s) {
                        Err(m) => erreur.set(Some(m)),
                        Ok(body) => {
                            erreur.set(None);
                            ecriture.lancer(async move {
                                session::client().creer_formulaire(&body).await?;
                                Ok("Brouillon créé : publiez-le pour prévenir les familles.".to_string())
                            });
                        }
                    }
                }
            >
                <Selection
                    id="form-type"
                    libelle="Type"
                    valeur=type_f
                    options=vec![
                        ("consentement".to_string(), "Consentement (versionné)".to_string()),
                        ("autorisation".to_string(), "Autorisation".to_string()),
                        ("sondage".to_string(), "Sondage".to_string()),
                    ]
                />
                {move || (type_f.get() != "sondage").then(|| view! {
                    <ChampTexte id="form-objet" libelle="Objet (identifiant stable)" valeur=objet
                        aide="Ex. droit_image : un refus masque la photo dans le trombinoscope.".to_string()/>
                })}
                <ChampTexte id="form-titre" libelle="Titre" valeur=titre requis=true/>
                <ZoneTexte id="form-texte" libelle="Texte soumis aux familles" valeur=texte requis=true/>
                <Selection
                    id="form-audience"
                    libelle="Destinataires"
                    valeur=audience
                    options=vec![
                        ("etablissement".to_string(), "Tout l'établissement".to_string()),
                        ("classe".to_string(), "Une classe".to_string()),
                    ]
                />
                {move || (audience.get() == "classe").then(|| match classes.get() {
                    Some(Ok(l)) => {
                        let mut options = vec![(String::new(), "— Choisir une classe —".to_string())];
                        options.extend(l.into_iter().map(|c| (c.id.to_string(), c.libelle)));
                        view! { <Selection id="form-classe" libelle="Classe" valeur=classe options=options/> }.into_any()
                    }
                    _ => view! { <Squelette lignes=1 libelle="Chargement des classes…"/> }.into_any(),
                })}
                {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                    "Créer le brouillon"
                </Bouton>
            </form>
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des formulaires…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeGestion formulaires=l ecriture=ecriture/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ListeGestion(formulaires: Vec<FormulaireDto>, ecriture: Ecriture) -> impl IntoView {
    view! {
        <ul class="jvs-liste">
            {formulaires.into_iter().map(|f| {
                let id = f.id;
                let publier = publiable(&f);
                let archiver = f.statut != "archive";
                let publie = f.statut == "publie";
                let nom = f.titre.clone();
                let (a1, a2) = (format!("Publier {nom}"), format!("Archiver {nom}"));
                view! {
                    <li class="jvs-ligne">
                        <div>
                            <strong>{f.titre.clone()}</strong>
                            <span class="texte-attenue">{format!(" · {} v{}", libelle_type(&f.type_formulaire), f.version)}</span>
                            {publie.then(|| view! { <ResultatsFormulaire id=id/> })}
                        </div>
                        <Badge libelle=libelle_statut(&f.statut) ton=ton_statut(&f.statut)/>
                        <div class="ra-actions">
                            {publier.then(|| view! {
                                <Bouton variante=Variante::Primaire taille=Taille::Compacte en_cours=ecriture.en_cours
                                    libelle_accessible=a1
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().publier_formulaire(id).await?;
                                            Ok("Formulaire publié : les familles sont prévenues.".to_string())
                                        });
                                    })>"Publier"</Bouton>
                            })}
                            {archiver.then(|| view! {
                                <Bouton variante=Variante::Discret taille=Taille::Compacte en_cours=ecriture.en_cours
                                    libelle_accessible=a2
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().archiver_formulaire(id).await?;
                                            Ok("Formulaire archivé.".to_string())
                                        });
                                    })>"Archiver"</Bouton>
                            })}
                        </div>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

/// Résultats agrégés d'un formulaire publié (comptages, jamais d'identité).
#[component]
fn ResultatsFormulaire(id: Uuid) -> impl IntoView {
    let r =
        LocalResource::new(move || async move { session::client().resultats_formulaire(id).await });
    view! {
        {move || match r.get() {
            Some(Ok(r)) => view! {
                <p class="texte-attenue">
                    {format!("{} élève(s) concerné(s) : {} accepté(s), {} refusé(s), {} sans réponse.", r.cibles, r.accepte, r.refuse, r.sans_reponse)}
                </p>
            }
            .into_any(),
            _ => ().into_any(),
        }}
    }
}

/// FRM-04 : qui consent à un objet précis, pour une classe.
#[component]
fn QuiConsent() -> impl IntoView {
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let classe = RwSignal::new(String::new());
    let objet = RwSignal::new(OBJET_DROIT_IMAGE.to_string());
    let demande = RwSignal::new(None::<(String, Uuid)>);
    let liste = LocalResource::new(move || {
        let d = demande.get();
        async move {
            match d {
                Some((o, c)) => session::client()
                    .consentements_classe(&o, c)
                    .await
                    .map(Some),
                None => Ok(None),
            }
        }
    });
    view! {
        <section class="ra-section" aria-labelledby="form-qui">
            <h2 id="form-qui" class="vs-section-titre">"Qui consent ?"</h2>
            <p class="texte-attenue">"Lecture d'un seul objet à la fois, pour une classe."</p>
            <form on:submit=move |ev| {
                ev.prevent_default();
                if let Ok(c) = Uuid::parse_str(classe.get_untracked().trim()) {
                    demande.set(Some((objet.get_untracked().trim().to_string(), c)));
                }
            }>
                <ChampTexte id="qui-objet" libelle="Objet" valeur=objet/>
                {move || match classes.get() {
                    Some(Ok(l)) => {
                        let mut options = vec![(String::new(), "— Choisir une classe —".to_string())];
                        options.extend(l.into_iter().map(|c| (c.id.to_string(), c.libelle)));
                        view! { <Selection id="qui-classe" libelle="Classe" valeur=classe options=options/> }.into_any()
                    }
                    _ => view! { <Squelette lignes=1 libelle="Chargement des classes…"/> }.into_any(),
                }}
                <Bouton variante=Variante::Secondaire soumettre=true>"Afficher"</Bouton>
            </form>
            {move || match liste.get() {
                Some(Ok(Some(l))) => view! {
                    <p class="texte-attenue">{format!("{} refus dans cette classe.", nb_refus(&l))}</p>
                    <ul class="jvs-liste">
                        {l.into_iter().map(|c| view! {
                            <li class="jvs-ligne">
                                <span>{format!("{} {}", c.prenom, c.nom)}</span>
                                <Badge libelle=libelle_decision(&c.decision) ton=ton_decision(&c.decision)/>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
                }
                .into_any(),
                _ => ().into_any(),
            }}
        </section>
    }
}
