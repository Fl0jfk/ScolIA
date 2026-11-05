//! Classes (rattachement site / niveau / MEF, archivage, professeur principal), groupes et
//! ateliers (création, responsables).

use ent_types::referentiel_admin::{GroupeAdminDto, PersonnelDto};
use ent_web_api::ClasseApi;
use ent_web_ui::{
    Badge, Bouton, ChampTexte, Dialogue, EtatVide, Selection, Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele::{
    ajouter_responsable, choix_option, filtrer_personnel, libelle_groupe, retirer_responsable,
    trier_classes_admin, Responsable,
};
use super::{message_echec, peut_gerer, EnTeteReferentiel, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageClassesAdmin() -> impl IntoView {
    let classes = LocalResource::new(|| async {
        session::client().classes(aujourdhui()).await.map(trier_classes_admin)
    });
    let groupes = LocalResource::new(|| async { session::client().groupes_admin(None).await });
    let recharger = Callback::new(move |_| {
        classes.refetch();
        groupes.refetch();
    });
    let ecriture = Ecriture::nouveau(recharger);
    view! {
        <Title text="Classes et groupes"/>
        <EnTeteReferentiel
            titre="Classes et groupes"
            sous_titre="Rattachement des classes, professeur principal, groupes et ateliers avec leurs responsables."
            courant="/web/referentiel/classes"
        />
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="ra-classes">
            <h2 id="ra-classes" class="vs-section-titre">"Classes de l'année courante"</h2>
            {move || match classes.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des classes…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune classe" message="Préparez l'année ou amorcez le référentiel depuis « Classes »."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <TableauClasses classes=l ecriture=ecriture/> }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="ra-groupes">
            <h2 id="ra-groupes" class="vs-section-titre">"Groupes et ateliers"</h2>
            {move || match (groupes.get(), classes.get()) {
                (None, _) | (_, None) => view! { <Squelette lignes=3 libelle="Chargement des groupes…"/> }.into_any(),
                (Some(Err(e)), _) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| groupes.refetch())/>
                }
                .into_any(),
                (Some(Ok(g)), c) => {
                    let c = c.and_then(Result::ok).unwrap_or_default();
                    view! { <SectionGroupes groupes=g classes=c ecriture=ecriture/> }.into_any()
                }
            }}
        </section>
    }
}

#[component]
fn TableauClasses(classes: Vec<ClasseApi>, ecriture: Ecriture) -> impl IntoView {
    let gerer = peut_gerer();
    let rattachement = RwSignal::new(None::<ClasseApi>);
    let ouvert_rattachement = RwSignal::new(false);
    let pp = RwSignal::new(None::<ClasseApi>);
    let ouvert_pp = RwSignal::new(false);
    view! {
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Classes de l'année courante"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Classe"</th>
                        <th scope="col">"Niveau"</th>
                        <th scope="col">"Effectif"</th>
                        <th scope="col">"Professeur principal"</th>
                        <th scope="col"><span class="ui-sr">"Actions"</span></th>
                    </tr>
                </thead>
                <tbody>
                    {classes
                        .into_iter()
                        .map(|c| {
                            let (pour_rattachement, pour_pp) = (c.clone(), c.clone());
                            let (a, b) = (
                                format!("Modifier le rattachement de {}", c.libelle),
                                format!("Désigner le professeur principal de {}", c.libelle),
                            );
                            view! {
                                <tr>
                                    <th scope="row">{c.libelle.clone()}</th>
                                    <td>{c.niveau.clone().unwrap_or_else(|| "—".into())}</td>
                                    <td>{c.effectif}</td>
                                    <td>{c.professeur_principal_label.clone().unwrap_or_else(|| "—".into())}</td>
                                    <td>
                                        {move || {
                                            gerer
                                                .get()
                                                .then(|| {
                                                    let (r, p) = (pour_rattachement.clone(), pour_pp.clone());
                                                    view! {
                                                        <div class="ra-actions">
                                                            <Bouton
                                                                variante=Variante::Discret
                                                                taille=Taille::Compacte
                                                                libelle_accessible=a.clone()
                                                                on_click=Callback::new(move |_| {
                                                                    rattachement.set(Some(r.clone()));
                                                                    ouvert_rattachement.set(true);
                                                                })
                                                            >
                                                                "Rattachement"
                                                            </Bouton>
                                                            <Bouton
                                                                variante=Variante::Discret
                                                                taille=Taille::Compacte
                                                                libelle_accessible=b.clone()
                                                                on_click=Callback::new(move |_| {
                                                                    pp.set(Some(p.clone()));
                                                                    ouvert_pp.set(true);
                                                                })
                                                            >
                                                                "Professeur principal"
                                                            </Bouton>
                                                        </div>
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
        <DialogueRattachement classe=rattachement ouvert=ouvert_rattachement ecriture=ecriture/>
        <DialoguePp classe=pp ouvert=ouvert_pp ecriture=ecriture/>
    }
}

/// Site, niveau, MEF et archivage d'une classe. « inchangé » n'envoie rien pour ce champ.
#[component]
fn DialogueRattachement(
    classe: RwSignal<Option<ClasseApi>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let sites = LocalResource::new(|| async { session::client().sites().await.unwrap_or_default() });
    let niveaux = LocalResource::new(|| async { session::client().niveaux().await.unwrap_or_default() });
    let mefs = LocalResource::new(|| async { session::client().mefs().await.unwrap_or_default() });
    let site = RwSignal::new(String::new());
    let niveau = RwSignal::new(String::new());
    let mef = RwSignal::new(String::new());
    let archivage = RwSignal::new(String::new());
    Effect::new(move |_| {
        if ouvert.get() {
            for s in [site, niveau, mef, archivage] {
                s.set(String::new());
            }
        }
    });
    let enregistrer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(c) = classe.get_untracked() else { return };
        let (s, n, m) = (
            choix_option(&site.get_untracked()),
            choix_option(&niveau.get_untracked()),
            choix_option(&mef.get_untracked()),
        );
        let a = match archivage.get_untracked().as_str() {
            "archiver" => Some(true),
            "desarchiver" => Some(false),
            _ => None,
        };
        if s.is_none() && n.is_none() && m.is_none() && a.is_none() {
            ouvert.set(false);
            return;
        }
        ouvert.set(false);
        let libelle = c.libelle.clone();
        ecriture.lancer(async move {
            session::client().modifier_classe(c.id, s, n, m, a).await?;
            Ok(format!("Classe {libelle} mise à jour."))
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Rattachement de la classe" id="dialogue-rattachement">
            {move || {
                classe
                    .get()
                    .map(|c| {
                        let sites = sites.get().unwrap_or_default();
                        let niveaux = niveaux.get().unwrap_or_default();
                        let mefs = mefs.get().unwrap_or_default();
                        let option_vide = || (String::new(), "— inchangé —".to_string());
                        let mut o_site = vec![option_vide()];
                        o_site.extend(sites.iter().map(|s| (s.id.to_string(), s.libelle.clone())));
                        let mut o_niv = vec![option_vide()];
                        o_niv.extend(niveaux.iter().map(|n| (n.id.to_string(), n.libelle.clone())));
                        let mut o_mef = vec![option_vide()];
                        o_mef.extend(mefs.iter().map(|m| (m.id.to_string(), format!("{} — {}", m.code_mef, m.libelle_long))));
                        let o_arch = vec![
                            option_vide(),
                            ("archiver".to_string(), "Archiver la classe".to_string()),
                            ("desarchiver".to_string(), "Désarchiver la classe".to_string()),
                        ];
                        view! {
                            <form class="ra-formulaire" on:submit=enregistrer novalidate>
                                <p>{format!("Classe {}", c.libelle)}</p>
                                <Selection id="rat-site" libelle="Site" valeur=site options=o_site/>
                                <Selection id="rat-niveau" libelle="Niveau" valeur=niveau options=o_niv/>
                                <Selection id="rat-mef" libelle="MEF" valeur=mef options=o_mef/>
                                <Selection id="rat-archivage" libelle="Archivage" valeur=archivage options=o_arch/>
                                <div class="ui-dialogue__actions">
                                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>
                                        "Annuler"
                                    </Bouton>
                                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer"</Bouton>
                                </div>
                            </form>
                        }
                    })
            }}
        </Dialogue>
    }
}

/// Recherche d'un membre du personnel (champ + liste de résultats choisissables).
#[component]
fn RechercheDePersonnel(
    id: &'static str,
    libelle: &'static str,
    /// Appelé avec la personne choisie.
    on_choisir: Callback<PersonnelDto>,
    /// Personnes à ne pas proposer (déjà choisies).
    #[prop(into)]
    exclus: Signal<Vec<Uuid>>,
) -> impl IntoView {
    let requete = RwSignal::new(String::new());
    let resultats = RwSignal::new(None::<Vec<PersonnelDto>>);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let chercher = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let q = requete.get_untracked();
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            match session::client().chercher_personnel(&q).await {
                Ok(l) => resultats.set(Some(l)),
                Err(e) => {
                    resultats.set(None);
                    erreur.set(Some(message_echec(e)));
                }
            }
            en_cours.set(false);
        });
    };
    view! {
        <form role="search" on:submit=chercher novalidate>
            <ChampTexte
                id=id
                libelle=libelle
                valeur=requete
                genre="search"
                autocomplete="off"
                erreur=Signal::derive(move || erreur.get())
            />
            <Bouton soumettre=true variante=Variante::Secondaire en_cours=en_cours>"Rechercher"</Bouton>
        </form>
        {move || {
            resultats
                .get()
                .map(|l| {
                    let l = filtrer_personnel(l, &exclus.get());
                    if l.is_empty() {
                        view! { <p class="texte-attenue" role="status">"Aucune personne à proposer."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="jvs-liste" aria-label="Personnel trouvé">
                                {l
                                    .into_iter()
                                    .map(|p| {
                                        let nom = format!("{} ({})", p.label, p.role);
                                        let a = format!("Choisir {nom}");
                                        view! {
                                            <li>
                                                <Bouton
                                                    variante=Variante::Secondaire
                                                    libelle_accessible=a
                                                    on_click=Callback::new(move |_| on_choisir.run(p.clone()))
                                                >
                                                    {nom}
                                                </Bouton>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                        .into_any()
                    }
                })
        }}
    }
}

#[component]
fn DialoguePp(
    classe: RwSignal<Option<ClasseApi>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    view! {
        <Dialogue ouvert=ouvert titre="Professeur principal" id="dialogue-pp">
            {move || {
                classe
                    .get()
                    .map(|c| {
                        let libelle = c.libelle.clone();
                        view! {
                            <p>{format!("Classe {libelle}. Recherchez le professeur à désigner.")}</p>
                            <RechercheDePersonnel
                                id="pp-recherche"
                                libelle="Nom ou adresse du professeur"
                                exclus=Signal::derive(Vec::new)
                                on_choisir=Callback::new(move |p: PersonnelDto| {
                                    ouvert.set(false);
                                    let (id, nom, l) = (c.id, p.label.clone(), libelle.clone());
                                    ecriture.lancer(async move {
                                        session::client().designer_professeur_principal(id, p.user_id).await?;
                                        Ok(format!("{nom} est professeur principal de {l}."))
                                    });
                                })
                            />
                            <div class="ui-dialogue__actions">
                                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>
                                    "Fermer"
                                </Bouton>
                            </div>
                        }
                    })
            }}
        </Dialogue>
    }
}

#[component]
fn SectionGroupes(
    groupes: Vec<GroupeAdminDto>,
    classes: Vec<ClasseApi>,
    ecriture: Ecriture,
) -> impl IntoView {
    let gerer = peut_gerer();
    let cible = RwSignal::new(None::<GroupeAdminDto>);
    let ouvert = RwSignal::new(false);
    let liste = if groupes.is_empty() {
        view! { <EtatVide titre="Aucun groupe" message="Créez un groupe de classe ou un atelier transverse ci-dessous."/> }.into_any()
    } else {
        let classes_ref = classes.clone();
        view! {
            <div class="ra-tableau-conteneur">
                <table class="ra-tableau">
                    <caption class="ui-sr">"Groupes et ateliers"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Groupe"</th>
                            <th scope="col">"Rattachement"</th>
                            <th scope="col">"Responsables"</th>
                            <th scope="col"><span class="ui-sr">"Actions"</span></th>
                        </tr>
                    </thead>
                    <tbody>
                        {groupes
                            .into_iter()
                            .map(|g| {
                                let classe = match g.classe_id.and_then(|id| classes_ref.iter().find(|c| c.id == id)) {
                                    Some(c) => view! { <span>{c.libelle.clone()}</span> }.into_any(),
                                    None if g.classe_id.is_some() => view! { <span>"Classe d'une autre année"</span> }.into_any(),
                                    None => view! { <Badge libelle="Atelier transverse" ton=TonEtat::Info/> }.into_any(),
                                };
                                let resp = if g.responsables.is_empty() {
                                    "—".to_string()
                                } else {
                                    g.responsables.iter().map(|r| r.label.clone()).collect::<Vec<_>>().join(", ")
                                };
                                let a = format!("Définir les responsables de {}", libelle_groupe(&g));
                                let pour_dialogue = g.clone();
                                view! {
                                    <tr>
                                        <th scope="row">{libelle_groupe(&g)}</th>
                                        <td>{classe}</td>
                                        <td>{resp}</td>
                                        <td>
                                            {move || {
                                                gerer
                                                    .get()
                                                    .then(|| {
                                                        let g2 = pour_dialogue.clone();
                                                        view! {
                                                            <Bouton
                                                                variante=Variante::Discret
                                                                taille=Taille::Compacte
                                                                libelle_accessible=a.clone()
                                                                on_click=Callback::new(move |_| {
                                                                    cible.set(Some(g2.clone()));
                                                                    ouvert.set(true);
                                                                })
                                                            >
                                                                "Responsables"
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
        .into_any()
    };
    view! {
        {liste}
        <DialogueResponsables groupe=cible ouvert=ouvert ecriture=ecriture/>
        {move || gerer.get().then(|| view! { <FormulaireGroupe classes=classes.clone() ecriture=ecriture/> })}
    }
}

#[component]
fn FormulaireGroupe(classes: Vec<ClasseApi>, ecriture: Ecriture) -> impl IntoView {
    let code = RwSignal::new(String::new());
    let libelle = RwSignal::new(String::new());
    let classe = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let mut options = vec![(String::new(), "Atelier transverse (aucune classe)".to_string())];
    options.extend(classes.iter().map(|c| (c.id.to_string(), format!("Demi-groupe de {}", c.libelle))));
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let c = code.get_untracked();
        if c.trim().is_empty() {
            erreur.set(Some("Le code est obligatoire.".into()));
            return;
        }
        erreur.set(None);
        let l = libelle.get_untracked();
        let cl = choix_option(&classe.get_untracked());
        ecriture.lancer(async move {
            let l = (!l.trim().is_empty()).then(|| l.trim().to_string());
            session::client()
                .creer_groupe(c.trim(), l.as_deref(), cl, None)
                .await?;
            Ok(format!("Groupe {} créé.", c.trim()))
        });
        code.set(String::new());
        libelle.set(String::new());
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="ra-groupe-nouveau">
            <h3 id="ra-groupe-nouveau" class="jvs-sous-titre">"Créer un groupe ou un atelier"</h3>
            <ChampTexte
                id="groupe-code"
                libelle="Code"
                valeur=code
                requis=true
                erreur=Signal::derive(move || erreur.get())
            />
            <ChampTexte id="groupe-libelle" libelle="Libellé (ex. Atelier théâtre)" valeur=libelle/>
            <Selection id="groupe-classe" libelle="Rattachement" valeur=classe options=options/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Créer"</Bouton>
        </form>
    }
}

/// Responsables d'un groupe : liste modifiable, enregistrée d'un bloc (droit `roles.gerer`).
#[component]
fn DialogueResponsables(
    groupe: RwSignal<Option<GroupeAdminDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let choisis = RwSignal::new(Vec::<Responsable>::new());
    Effect::new(move |_| {
        if ouvert.get() {
            choisis.set(
                groupe
                    .get_untracked()
                    .map(|g| {
                        g.responsables
                            .iter()
                            .map(|r| Responsable { user_id: r.user_id, label: r.label.clone() })
                            .collect()
                    })
                    .unwrap_or_default(),
            );
        }
    });
    let exclus = Signal::derive(move || choisis.get().iter().map(|r| r.user_id).collect::<Vec<_>>());
    let enregistrer = Callback::new(move |_| {
        let Some(g) = groupe.get_untracked() else { return };
        let ids: Vec<Uuid> = choisis.get_untracked().iter().map(|r| r.user_id).collect();
        ouvert.set(false);
        let nom = libelle_groupe(&g);
        ecriture.lancer(async move {
            session::client().definir_responsables_groupe(g.id, &ids).await?;
            Ok(format!("Responsables de {nom} enregistrés."))
        });
    });
    view! {
        <Dialogue ouvert=ouvert titre="Responsables du groupe" id="dialogue-responsables">
            {move || {
                groupe
                    .get()
                    .map(|g| {
                        view! {
                            <p>{format!("Groupe {}", libelle_groupe(&g))}</p>
                            <h3 class="jvs-sous-titre">"Responsables actuels"</h3>
                            {move || {
                                let l = choisis.get();
                                if l.is_empty() {
                                    view! { <p class="texte-attenue">"Aucun responsable."</p> }.into_any()
                                } else {
                                    view! {
                                        <ul class="jvs-liste">
                                            {l
                                                .into_iter()
                                                .map(|r| {
                                                    let (id, nom) = (r.user_id, r.label.clone());
                                                    let a = format!("Retirer {nom}");
                                                    view! {
                                                        <li class="jvs-ligne">
                                                            <span>{nom}</span>
                                                            <Bouton
                                                                variante=Variante::Discret
                                                                taille=Taille::Compacte
                                                                libelle_accessible=a
                                                                on_click=Callback::new(move |_| choisis.update(|v| retirer_responsable(v, id)))
                                                            >
                                                                "Retirer"
                                                            </Bouton>
                                                        </li>
                                                    }
                                                })
                                                .collect_view()}
                                        </ul>
                                    }
                                    .into_any()
                                }
                            }}
                            <RechercheDePersonnel
                                id="resp-recherche"
                                libelle="Ajouter un responsable (nom ou adresse)"
                                exclus=exclus
                                on_choisir=Callback::new(move |p: PersonnelDto| {
                                    choisis.update(|v| ajouter_responsable(v, p.user_id, &p.label))
                                })
                            />
                            <p class="texte-attenue">
                                "Réservé aux personnes pouvant gérer les rôles ; le serveur refuse sinon."
                            </p>
                            <div class="ui-dialogue__actions">
                                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>
                                    "Annuler"
                                </Bouton>
                                <Bouton en_cours=ecriture.en_cours on_click=enregistrer>"Enregistrer"</Bouton>
                            </div>
                        }
                    })
            }}
        </Dialogue>
    }
}
