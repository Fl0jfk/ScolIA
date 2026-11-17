//! E9b — fiche hub d'un élève (`/web/eleves/:id`) : en-tête « carte » (photo, désignation, classe,
//! statut, pastilles), puis trois sections : **Synthèse** (une tuile par domaine, masquée avec sa raison
//! quand le droit manque), **Identité** (ce que les groupes de champs ouverts laissent voir, identifiants
//! compris) et **Historique** (frise des versions d'identité, consultations pour l'audit).
//!
//! L'écran ne calcule aucun droit. Un élève absent, d'un autre établissement ou protégé pour ce lecteur
//! répond la même chose (« fiche introuvable ») : rien ne permet de les distinguer.

use ent_types::dossier::MasqueChampsDto;
use ent_types::eleve_synthese::{EleveFicheDto, PastilleDto, SyntheseDto, TuileDto};
use ent_web_api::{adresse_photo_eleve, ApiError};
use ent_web_ui::{
    AccesRefuse, Alerte, Annonce, Avatar, Badge, EtatVide, GrilleBento, Largeur, LienBouton,
    Niveau, OptionSegment, Segmente, Squelette, TonEtat, Tuile, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use uuid::Uuid;

use super::modele::{
    corps_tuile_sans_ecran, date_fr, date_naissance_affichee, designation, destination_tuile,
    etapes_frise, libelle_consultation, libelle_ouverture, libelle_statut,
    libelle_statut_identifiant, libelle_systeme, phrase_pastille, raison_masquage, ton_gravite,
    ton_statut, ton_statut_identifiant, Destination, Onglet,
};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

fn adresse_onglet(eleve: Uuid, onglet: Onglet) -> String {
    match onglet {
        Onglet::Synthese => format!("{}/{eleve}", navigation::ELEVES),
        o => format!("{}/{eleve}?onglet={}", navigation::ELEVES, o.code()),
    }
}

#[component]
pub fn PageFicheHub() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|i| i.parse::<Uuid>().ok())
    });
    let fiche = LocalResource::new(move || {
        let id = id.get();
        async move {
            match id {
                Some(id) => session::client().eleve_fiche(id).await,
                None => Err(ApiError::Introuvable("identifiant d'élève invalide".into())),
            }
        }
    });
    let recharger = Callback::new(move |_| fiche.refetch());
    view! {
        {move || match fiche.get() {
            None => {
                view! {
                    <Title text="Fiche élève"/>
                    <Squelette lignes=8 libelle="Chargement de la fiche…"/>
                }
                    .into_any()
            }
            // Absent, d'un autre établissement ou protégé : une seule et même réponse.
            Some(Err(ApiError::Introuvable(_))) => {
                view! {
                    <Title text="Fiche introuvable"/>
                    <EtatVide
                        titre="Fiche introuvable"
                        message="Cet élève n'existe pas dans votre établissement, ou sa fiche ne vous est pas ouverte."
                    >
                        <LienBouton href=navigation::ELEVES variante=Variante::Secondaire>
                            "Retour à la liste des élèves"
                        </LienBouton>
                    </EtatVide>
                }
                    .into_any()
            }
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(f)) => view! { <ContenuFiche fiche=f/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFiche(fiche: EleveFicheDto) -> impl IntoView {
    let id = fiche.id;
    let nom = designation(&fiche.prenom, &fiche.nom);
    let titre = format!("{nom} · fiche élève");
    let query = use_query_map();
    let navigate = use_navigate();
    let onglet = Memo::new(move |_| {
        Onglet::depuis_code(&query.with(|q| q.get("onglet")).unwrap_or_default())
    });
    // Le contrôle segmenté et l'adresse restent d'accord (lien direct, retour arrière, tuiles).
    let valeur = RwSignal::new(onglet.get_untracked().code().to_string());
    Effect::new(move |_| {
        let c = onglet.get().code().to_string();
        if valeur.get_untracked() != c {
            valeur.set(c);
        }
    });
    Effect::new(move |_| {
        let demande = Onglet::depuis_code(&valeur.get());
        if demande != onglet.get_untracked() {
            navigate(&adresse_onglet(id, demande), Default::default());
        }
    });
    let annonce = RwSignal::new(String::new());
    Effect::new(move |_| annonce.set(format!("Section {}", onglet.get().libelle())));

    let synthese =
        LocalResource::new(move || async move { session::client().eleve_synthese(id).await });
    let fiche = StoredValue::new(fiche);
    let options: Vec<OptionSegment> = Onglet::TOUS
        .iter()
        .map(|o| OptionSegment::new(o.code(), o.libelle()))
        .collect();

    let ligne_statut = {
        let f = fiche.get_value();
        let mut morceaux = Vec::new();
        if let Some(c) = f.classe.clone().filter(|c| !c.trim().is_empty()) {
            morceaux.push(format!("Classe {c}"));
        }
        if let Some(r) = f.regime.clone().filter(|r| !r.trim().is_empty()) {
            morceaux.push(format!("régime {r}"));
        }
        morceaux.join(" · ")
    };
    let statut = fiche.with_value(|f| f.status.clone());
    let photo_ok = RwSignal::new(true);

    view! {
        <Title text=titre/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::ELEVES>"Élèves"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">{nom.clone()}</span>
        </nav>
        <Annonce message=annonce/>
        <header class="fiche-entete">
            <div class="fiche-photo">
                {move || {
                    if photo_ok.get() {
                        view! {
                            <img
                                class="fiche-photo__image"
                                src=adresse_photo_eleve(id)
                                alt=""
                                width="96"
                                height="96"
                                on:error=move |_| photo_ok.set(false)
                            />
                        }
                            .into_any()
                    } else {
                        view! { <span class="fiche-photo__initiales"><Avatar nom=fiche.with_value(|f| designation(&f.prenom, &f.nom)) accent=true/></span> }
                            .into_any()
                    }
                }}
            </div>
            <div class="fiche-entete__texte">
                <h1 class="fiche-entete__nom">{nom.clone()}</h1>
                <p class="fiche-entete__ligne">
                    <Badge libelle=libelle_statut(&statut) ton=ton_statut(&statut)/>
                    {(!ligne_statut.is_empty()).then(|| view! { <span>{ligne_statut.clone()}</span> })}
                </p>
                {move || match synthese.get() {
                    Some(Ok(s)) => view! { <ListePastilles pastilles=s.pastilles/> }.into_any(),
                    _ => ().into_any(),
                }}
            </div>
            <div class="fiche-entete__actions">
                <LienBouton href=format!("{}/{id}/dossier", navigation::ELEVES) variante=Variante::Secondaire>
                    "Dossier complet"
                </LienBouton>
                <LienBouton href=navigation::ELEVES variante=Variante::Secondaire>"Liste des élèves"</LienBouton>
            </div>
        </header>
        <Segmente nom="fiche-onglet" legende="Sections de la fiche" options=options valeur=valeur/>
        {move || match onglet.get() {
            Onglet::Synthese => view! { <SectionSynthese id=id synthese=synthese/> }.into_any(),
            Onglet::Identite => view! { <SectionIdentite id=id fiche=fiche/> }.into_any(),
            Onglet::Historique => view! { <SectionHistorique id=id/> }.into_any(),
        }}
    }
}

/// Les pastilles visibles : toujours un libellé lisible, la couleur ne fait que doubler.
#[component]
fn ListePastilles(pastilles: Vec<PastilleDto>) -> impl IntoView {
    if pastilles.is_empty() {
        return ().into_any();
    }
    view! {
        <ul class="fiche-pastilles" aria-label="Pastilles">
            {pastilles
                .into_iter()
                .map(|p| {
                    let phrase = phrase_pastille(&p);
                    view! {
                        <li>
                            <Badge libelle=phrase ton=ton_gravite(&p.gravite)/>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

// ───────────────────────────── Synthèse ─────────────────────────────

#[component]
fn SectionSynthese(
    id: Uuid,
    synthese: LocalResource<Result<SyntheseDto, ApiError>>,
) -> impl IntoView {
    let recharger = Callback::new(move |_| synthese.refetch());
    view! {
        {move || match synthese.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la synthèse…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => {
                view! {
                    <AccesRefuse message="La synthèse de la fiche est réservée au personnel de l'établissement."/>
                }
                    .into_any()
            }
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(s)) => {
                view! {
                    <GrilleBento libelle="Synthèse par domaine">
                        {s.tuiles.into_iter().map(|t| view! { <TuileDomaine id=id tuile=t/> }).collect_view()}
                    </GrilleBento>
                }
                    .into_any()
            }
        }}
    }
}

/// Une tuile : visible (pastilles et lien vers l'écran du domaine quand il existe) ou masquée avec sa
/// raison. Une tuile masquée ne montre rien de son contenu.
#[component]
fn TuileDomaine(id: Uuid, tuile: TuileDto) -> impl IntoView {
    let ident = format!("tuile-{}", tuile.domaine);
    if tuile.etat != "visible" {
        let raison = raison_masquage(tuile.raison.as_deref());
        return view! {
            <Tuile id=ident titre=tuile.libelle.clone() largeur=Largeur::Six>
                <p class="fiche-tuile__masquee">
                    <Badge libelle="Masqué" ton=TonEtat::Neutre/>
                </p>
                <p>{raison}</p>
            </Tuile>
        }
        .into_any();
    }
    let destination = destination_tuile(&tuile.domaine, id);
    let corps_sans_ecran =
        matches!(destination, Destination::Aucune).then(|| corps_tuile_sans_ecran(&tuile.domaine));
    let lien = match &destination {
        Destination::Onglet(o) => Some(adresse_onglet(id, *o)),
        Destination::Page(p) => Some(p.clone()),
        Destination::Aucune => None,
    };
    let ouverture = libelle_ouverture(&tuile.domaine);
    view! {
        <Tuile id=ident titre=tuile.libelle.clone() largeur=Largeur::Six>
            <ListePastilles pastilles=tuile.pastilles.clone()/>
            {corps_sans_ecran.map(|t| view! { <p>{t}</p> })}
            {lien
                .map(|l| {
                    view! {
                        <p>
                            <a href=l>{ouverture}</a>
                        </p>
                    }
                })}
        </Tuile>
    }
    .into_any()
}

// ───────────────────────────── Identité ─────────────────────────────

#[component]
fn SectionIdentite(id: Uuid, fiche: StoredValue<EleveFicheDto>) -> impl IntoView {
    let f = fiche.get_value();
    let mut lignes: Vec<(&'static str, String)> =
        vec![("Prénom", f.prenom.clone()), ("Nom", f.nom.to_uppercase())];
    let masques: Vec<MasqueChampsDto> = f.masques.clone();
    let masque = |groupe: &str| masques.iter().any(|m| m.groupe == groupe);
    if !masque("etat_civil") {
        lignes.push((
            "Sexe à l'état civil",
            match f.sexe.as_deref() {
                Some("F") => "Féminin".into(),
                Some("M") => "Masculin".into(),
                _ => "—".into(),
            },
        ));
        lignes.push((
            "Date de naissance",
            date_naissance_affichee(f.date_naissance, "jour", "declaree"),
        ));
    }
    if !masque("scolarite") {
        lignes.push((
            "Classe",
            f.classe
                .clone()
                .filter(|c| !c.is_empty())
                .unwrap_or_else(|| "—".into()),
        ));
        lignes.push((
            "Régime",
            f.regime
                .clone()
                .filter(|c| !c.is_empty())
                .unwrap_or_else(|| "—".into()),
        ));
        if let Some(d) = f.date_entree {
            lignes.push(("Entré le", date_fr(d)));
        }
        if let Some(d) = f.date_sortie {
            lignes.push(("Sorti le", date_fr(d)));
        }
    }
    let identifiants =
        LocalResource::new(move || async move { session::client().eleve_identifiants(id).await });
    view! {
        <Tuile id="fiche-identite" titre="État civil et scolarité" largeur=Largeur::Douze>
            <dl class="fiche-liste">
                {lignes
                    .into_iter()
                    .map(|(l, v)| {
                        view! {
                            <div class="fiche-liste__ligne">
                                <dt>{l}</dt>
                                <dd>{v}</dd>
                            </div>
                        }
                    })
                    .collect_view()}
            </dl>
            {(!masques.is_empty())
                .then(|| {
                    view! {
                        <div class="fiche-masques">
                            <Alerte niveau=Niveau::Info titre="Certaines informations ne vous sont pas ouvertes.">
                                <ul>
                                    {masques
                                        .iter()
                                        .map(|m| view! { <li>{m.message.clone()}</li> })
                                        .collect_view()}
                                </ul>
                            </Alerte>
                        </div>
                    }
                })}
        </Tuile>
        <div id="identifiants">
            <Tuile id="fiche-identifiants" titre="Identifiants" largeur=Largeur::Douze>
                {move || match identifiants.get() {
                    None => view! { <Squelette lignes=2 libelle="Chargement des identifiants…"/> }.into_any(),
                    Some(Err(ApiError::Interdit(_))) => {
                        view! {
                            <p>"Les identifiants (INE, badge) ne sont pas ouverts avec vos droits."</p>
                        }
                            .into_any()
                    }
                    Some(Err(e)) => {
                        view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| identifiants.refetch())/> }
                            .into_any()
                    }
                    Some(Ok(v)) if v.is_empty() => view! { <p>"Aucun identifiant enregistré."</p> }.into_any(),
                    Some(Ok(v)) => {
                        view! {
                            <ul class="fiche-identifiants">
                                {v
                                    .into_iter()
                                    .map(|i| {
                                        view! {
                                            <li>
                                                <span class="fiche-identifiants__systeme">{libelle_systeme(&i.systeme)}</span>
                                                <span class="fiche-identifiants__valeur">{i.valeur.clone()}</span>
                                                <Badge
                                                    libelle=libelle_statut_identifiant(&i.statut)
                                                    ton=ton_statut_identifiant(&i.statut)
                                                />
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }}
            </Tuile>
        </div>
    }
}

// ───────────────────────────── Historique ─────────────────────────────

#[component]
fn SectionHistorique(id: Uuid) -> impl IntoView {
    let versions =
        LocalResource::new(
            move || async move { session::client().eleve_historique_identite(id).await },
        );
    let consultations =
        LocalResource::new(move || async move { session::client().eleve_consultations(id).await });
    view! {
        <Tuile id="fiche-historique" titre="Historique de l'identité" largeur=Largeur::Douze>
            {move || match versions.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement de l'historique…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => {
                    view! {
                        <p>"L'historique de l'identité est réservé au secrétariat et à la direction."</p>
                    }
                        .into_any()
                }
                Some(Err(e)) => {
                    view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| versions.refetch())/> }
                        .into_any()
                }
                Some(Ok(v)) if v.is_empty() => view! { <p>"Aucune version d'identité visible."</p> }.into_any(),
                Some(Ok(v)) => {
                    view! {
                        <ol class="fiche-frise">
                            {etapes_frise(&v)
                                .into_iter()
                                .map(|e| {
                                    let champs = e.champs.join(", ");
                                    view! {
                                        <li class="fiche-frise__etape">
                                            <p class="fiche-frise__titre">
                                                <Badge libelle=e.nature ton=e.ton/>
                                                <span>{format!("effet au {}", date_fr(e.effet_du))}</span>
                                            </p>
                                            {(!champs.is_empty()).then(|| view! { <p>{format!("Modifié : {champs}")}</p> })}
                                            {e.motif.clone().map(|m| view! { <p>{format!("Motif : {m}")}</p> })}
                                            <p class="fiche-frise__meta">
                                                {e.source}
                                                {e.avec_piece.then_some(" · pièce justificative jointe")}
                                                {e.corrigee.then_some(" · version corrigée depuis")}
                                                {e.sensible.then_some(" · information sensible")}
                                            </p>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ol>
                    }
                        .into_any()
                }
            }}
        </Tuile>
        {move || match consultations.get() {
            Some(Ok(c)) if !c.is_empty() => {
                view! {
                    <Tuile id="fiche-consultations" titre="Consultations de cette fiche" largeur=Largeur::Douze>
                        <ul class="fiche-consultations">
                            {c
                                .into_iter()
                                .map(|x| {
                                    view! {
                                        <li>
                                            <span>{libelle_consultation(&x.action)}</span>
                                            <span>{date_heure_paris(x.quand)}</span>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    </Tuile>
                }
                    .into_any()
            }
            _ => ().into_any(),
        }}
    }
}
