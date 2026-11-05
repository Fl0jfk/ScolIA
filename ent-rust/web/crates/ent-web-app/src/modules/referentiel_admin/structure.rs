//! Sites, niveaux (nomenclature nationale) et MEF (import SIECLE / BCN).

use ent_types::referentiel_admin::{MefDto, NiveauDto, SiteDto};
use ent_web_ui::{Alerte, Badge, Bouton, ChampTexte, EtatVide, Niveau, Squelette, TonEtat, Variante, ZoneTexte};
use leptos::prelude::*;
use leptos_meta::Title;

use super::modele::{composer_site, libelle_niveau, lire_import_mef, resume_import_mef};
use super::{peut_gerer, EnTeteReferentiel, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageStructure() -> impl IntoView {
    view! {
        <Title text="Sites, niveaux et MEF"/>
        <EnTeteReferentiel
            titre="Sites, niveaux et MEF"
            sous_titre="Implantations de l'établissement, niveaux de la nomenclature nationale et modules élémentaires de formation."
            courant="/web/referentiel/structure"
        />
        <SectionSites/>
        <SectionNiveaux/>
    }
}

#[component]
fn SectionSites() -> impl IntoView {
    let gerer = peut_gerer();
    let sites = LocalResource::new(|| async { session::client().sites().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| sites.refetch()));
    view! {
        <section class="ra-section" aria-labelledby="ra-sites">
            <h2 id="ra-sites" class="vs-section-titre">"Sites"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match sites.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des sites…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| sites.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun site" message="L'établissement n'a pas encore de site déclaré."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <TableauSites sites=l ecriture=ecriture/> }.into_any(),
            }}
            {move || gerer.get().then(|| view! { <FormulaireSite ecriture=ecriture/> })}
        </section>
    }
}

#[component]
fn TableauSites(sites: Vec<SiteDto>, ecriture: Ecriture) -> impl IntoView {
    let gerer = peut_gerer();
    view! {
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Sites de l'établissement"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Code"</th>
                        <th scope="col">"Libellé"</th>
                        <th scope="col">"UAI"</th>
                        <th scope="col">"État"</th>
                        <th scope="col"><span class="ui-sr">"Actions"</span></th>
                    </tr>
                </thead>
                <tbody>
                    {sites
                        .into_iter()
                        .map(|s| {
                            let (id, actif, nom) = (s.id, s.actif, s.libelle.clone());
                            let libelle_bouton = format!(
                                "{} le site {}",
                                if actif { "Désactiver" } else { "Réactiver" },
                                nom
                            );
                            view! {
                                <tr>
                                    <th scope="row">{s.code.clone()}</th>
                                    <td>
                                        {s.libelle.clone()}
                                        {s.adresse.clone().map(|a| view! { <span class="texte-attenue">{format!(" — {a}")}</span> })}
                                    </td>
                                    <td>{s.uai.clone().unwrap_or_else(|| "—".into())}</td>
                                    <td>
                                        <Badge
                                            libelle=if actif { "Actif" } else { "Inactif" }
                                            ton=if actif { TonEtat::Ok } else { TonEtat::Neutre }
                                        />
                                    </td>
                                    <td>
                                        {move || {
                                            gerer
                                                .get()
                                                .then(|| {
                                                    view! {
                                                        <Bouton
                                                            variante=Variante::Discret
                                                            taille=ent_web_ui::Taille::Compacte
                                                            en_cours=ecriture.en_cours
                                                            libelle_accessible=libelle_bouton.clone()
                                                            on_click=Callback::new(move |_| {
                                                                ecriture
                                                                    .lancer(async move {
                                                                        session::client().maj_site(id, None, None, Some(!actif)).await?;
                                                                        Ok("Site mis à jour.".to_string())
                                                                    });
                                                            })
                                                        >
                                                            {if actif { "Désactiver" } else { "Réactiver" }}
                                                        </Bouton>
                                                    }
                                                })
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn FormulaireSite(ecriture: Ecriture) -> impl IntoView {
    let code = RwSignal::new(String::new());
    let libelle = RwSignal::new(String::new());
    let adresse = RwSignal::new(String::new());
    let uai = RwSignal::new(String::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let err = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
        })
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        match composer_site(&code.get_untracked(), &libelle.get_untracked()) {
            Err(e) => erreurs.set(e),
            Ok((c, l)) => {
                erreurs.set(Vec::new());
                let (a, u) = (adresse.get_untracked(), uai.get_untracked());
                ecriture.lancer(async move {
                    let a = (!a.trim().is_empty()).then(|| a.trim().to_string());
                    let u = (!u.trim().is_empty()).then(|| u.trim().to_string());
                    session::client()
                        .creer_site(&c, &l, a.as_deref(), u.as_deref())
                        .await?;
                    Ok(format!("Site « {l} » créé."))
                });
                code.set(String::new());
                libelle.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="ra-site-nouveau">
            <h3 id="ra-site-nouveau" class="jvs-sous-titre">"Ajouter un site"</h3>
            <ChampTexte id="site-code" libelle="Code" valeur=code requis=true erreur=err("site-code")/>
            <ChampTexte id="site-libelle" libelle="Libellé" valeur=libelle requis=true erreur=err("site-libelle")/>
            <ChampTexte id="site-adresse" libelle="Adresse" valeur=adresse autocomplete="off"/>
            <ChampTexte id="site-uai" libelle="UAI (facultatif)" valeur=uai autocomplete="off"/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Ajouter le site"</Bouton>
        </form>
    }
}

#[component]
fn SectionNiveaux() -> impl IntoView {
    let gerer = peut_gerer();
    let niveaux = LocalResource::new(|| async { session::client().niveaux().await });
    let mefs = LocalResource::new(|| async { session::client().mefs().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        niveaux.refetch();
        mefs.refetch();
    }));
    let amorcer = Callback::new(move |_| {
        ecriture.lancer(async move {
            let r = session::client().amorcer_niveaux().await?;
            Ok(format!(
                "{} niveau(x) créé(s), {} classe(s) rattachée(s).",
                r.niveaux_crees, r.classes_rattachees
            ))
        });
    });
    view! {
        <section class="ra-section" aria-labelledby="ra-niveaux">
            <h2 id="ra-niveaux" class="vs-section-titre">"Niveaux"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || gerer.get().then(|| view! {
                <p>
                    <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=amorcer>
                        "Charger la nomenclature nationale"
                    </Bouton>
                    <span class="texte-attenue">" Idempotent : crée les niveaux manquants et rattache les classes existantes."</span>
                </p>
            })}
            {move || match niveaux.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des niveaux…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| niveaux.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun niveau" message="Chargez la nomenclature nationale pour créer 6e … Terminale."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <TableauNiveaux niveaux=l/> }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="ra-mef">
            <h2 id="ra-mef" class="vs-section-titre">"MEF"</h2>
            {move || match (mefs.get(), niveaux.get()) {
                (None, _) => view! { <Squelette lignes=3 libelle="Chargement des MEF…"/> }.into_any(),
                (Some(Err(e)), _) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| mefs.refetch())/>
                }
                .into_any(),
                (Some(Ok(l)), n) if l.is_empty() => {
                    let _ = n;
                    view! {
                        <EtatVide titre="Aucun MEF" message="Importez un fichier SIECLE ou BCN ci-dessous."/>
                    }
                    .into_any()
                }
                (Some(Ok(l)), n) => {
                    let n = n.and_then(Result::ok).unwrap_or_default();
                    view! { <TableauMef mefs=l niveaux=n/> }.into_any()
                }
            }}
            {move || gerer.get().then(|| view! { <ImportMef ecriture=ecriture/> })}
        </section>
    }
}

#[component]
fn TableauNiveaux(niveaux: Vec<NiveauDto>) -> impl IntoView {
    let mut tri = niveaux;
    tri.sort_by_key(|n| n.ordre);
    view! {
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Niveaux"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Code"</th>
                        <th scope="col">"Niveau"</th>
                        <th scope="col">"Cycle"</th>
                        <th scope="col">"Origine"</th>
                    </tr>
                </thead>
                <tbody>
                    {tri
                        .into_iter()
                        .map(|n| {
                            view! {
                                <tr>
                                    <th scope="row">{n.code}</th>
                                    <td>{n.libelle}</td>
                                    <td>{n.cycle.map(|c| c.to_string()).unwrap_or_else(|| "—".into())}</td>
                                    <td>{n.source}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn TableauMef(mefs: Vec<MefDto>, niveaux: Vec<NiveauDto>) -> impl IntoView {
    view! {
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Modules élémentaires de formation"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Code MEF"</th>
                        <th scope="col">"Libellé"</th>
                        <th scope="col">"Niveau"</th>
                        <th scope="col">"Origine"</th>
                    </tr>
                </thead>
                <tbody>
                    {mefs
                        .into_iter()
                        .map(|m| {
                            let niveau = libelle_niveau(&niveaux, m.niveau_id);
                            view! {
                                <tr>
                                    <th scope="row">{m.code_mef}</th>
                                    <td>{m.libelle_long}</td>
                                    <td>{niveau}</td>
                                    <td>{m.source}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn ImportMef(ecriture: Ecriture) -> impl IntoView {
    let texte = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let rejets = RwSignal::new(Vec::<String>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (lignes, illisibles) = lire_import_mef(&texte.get_untracked());
        rejets.set(illisibles);
        if lignes.is_empty() {
            erreur.set(Some("Aucune ligne exploitable à importer.".into()));
            return;
        }
        erreur.set(None);
        ecriture.lancer(async move {
            let r = session::client().importer_mefs("import_siecle", &lignes).await?;
            Ok(resume_import_mef(&r))
        });
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="ra-mef-import">
            <h3 id="ra-mef-import" class="jvs-sous-titre">"Importer des MEF"</h3>
            <ZoneTexte
                id="mef-texte"
                libelle="Une ligne par MEF : code MEF ; libellé ; niveau (ex. 6EME) ; MEFSTAT11"
                valeur=texte
                erreur=Signal::derive(move || erreur.get())
                aide="L'import est idempotent : un MEF déjà connu est mis à jour, une ligne invalide est rejetée sans bloquer les autres."
            />
            {move || {
                let r = rejets.get();
                (!r.is_empty())
                    .then(|| view! {
                        <Alerte niveau=Niveau::Avertissement titre=format!("{} ligne(s) illisible(s) ignorée(s)", r.len())>
                            <ul>{r.into_iter().map(|l| view! { <li>{l}</li> }).collect_view()}</ul>
                        </Alerte>
                    })
            }}
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Importer"</Bouton>
        </form>
    }
}
