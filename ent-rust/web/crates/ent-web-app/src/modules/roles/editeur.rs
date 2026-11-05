//! Création et modification d'un rôle : nom, tableau « action × portée », classes et
//! groupes désignés, suppression, membres.

use std::collections::BTreeSet;

use ent_web_api::{ApiError, CatalogueRoles, PersonnelApi, RoleApi};
use ent_web_ui::{
    Alerte, Annonce, Bouton, ChampTexte, Confirmation, Dialogue, EnTetePage, Niveau, Selection,
    Squelette, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map};
use uuid::Uuid;

use super::modele::{
    aide_portee, candidats, etiquette_membre, libelle_cible, libelle_court_portee, lire_date_fin,
    message_erreur_attribution, message_erreur_role, modele_par_id, trier_cibles, Cible,
    EditionRole, MODELES, PORTEE_DESIGNES,
};
use super::{adresse_role, retour_connexion, GardeRoles};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

/// Ce que l'écran charge avant de s'afficher.
#[derive(Clone)]
struct Donnees {
    catalogue: CatalogueRoles,
    role: Option<RoleApi>,
    personnel: Vec<PersonnelApi>,
}

async fn charger(id: Option<Uuid>) -> Result<Donnees, ApiError> {
    let client = session::client();
    let catalogue = client.catalogue_roles().await?;
    let role = match id {
        Some(id) => Some(
            client
                .roles()
                .await?
                .into_iter()
                .find(|r| r.id == id)
                .ok_or_else(|| ApiError::Introuvable("rôle".into()))?,
        ),
        None => None,
    };
    // Le personnel sert aux membres : inutile à la création.
    let personnel = if id.is_some() {
        client.personnel("").await?
    } else {
        Vec::new()
    };
    Ok(Donnees {
        catalogue,
        role,
        personnel,
    })
}

/// Classes (et groupes rattachés à une classe) que l'on peut désigner.
pub(super) async fn charger_cibles() -> Result<Vec<Cible>, ApiError> {
    let client = session::client();
    let jour = aujourdhui();
    let classes = client.classes(jour).await?;
    let mut cibles: Vec<Cible> = classes
        .iter()
        .map(|c| Cible {
            id: c.id,
            libelle: c.libelle.clone(),
            groupe: false,
        })
        .collect();
    let mut vus = BTreeSet::new();
    for c in &classes {
        // Un groupe est listé avec le détail de sa classe ; une classe illisible n'empêche pas les autres.
        if let Ok(detail) = client.classe(c.id, jour).await {
            for g in detail.groupes {
                if vus.insert(g.id) {
                    cibles.push(Cible {
                        id: g.id,
                        libelle: g.libelle.clone(),
                        groupe: true,
                    });
                }
            }
        }
    }
    Ok(trier_cibles(cibles))
}

#[component]
pub fn PageNouveauRole() -> impl IntoView {
    view! {
        <Title text="Nouveau rôle"/>
        <GardeRoles>
            <ChargeurEditeur id=None/>
        </GardeRoles>
    }
}

#[component]
pub fn PageRole() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|v| Uuid::parse_str(&v).ok())
    };
    view! {
        <Title text="Modifier un rôle"/>
        <GardeRoles>
            {move || match id() {
                Some(id) => view! { <ChargeurEditeur id=Some(id)/> }.into_any(),
                None => view! {
                    <EnTetePage titre="Rôle introuvable".to_string() surtitre="Rôles et droits".to_string()/>
                    <p><a href=navigation::ROLES>"Retour à la liste des rôles"</a></p>
                }
                .into_any(),
            }}
        </GardeRoles>
    }
}

#[component]
fn ChargeurEditeur(id: Option<Uuid>) -> impl IntoView {
    let donnees = LocalResource::new(move || async move { charger(id).await });
    view! {
        {move || match donnees.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement du rôle…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EnTetePage titre="Rôle introuvable".to_string() surtitre="Rôles et droits".to_string()/>
                <Alerte niveau=Niveau::Erreur titre="Ce rôle n'existe pas ou n'existe plus.">
                    <p><a href=navigation::ROLES>"Retour à la liste des rôles"</a></p>
                </Alerte>
            }
            .into_any(),
            Some(Err(e)) => {
                view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/> }
                    .into_any()
            }
            Some(Ok(d)) => view! { <Editeur donnees=d/> }.into_any(),
        }}
    }
}

#[component]
fn Editeur(donnees: Donnees) -> impl IntoView {
    let Donnees {
        catalogue,
        role,
        personnel,
    } = donnees;
    let existant = role.as_ref().map(|r| r.id);
    let nom_role = role.as_ref().map(|r| r.nom.clone()).unwrap_or_default();
    let depart = match &role {
        Some(r) => EditionRole::depuis_role(r),
        None => EditionRole::vide(),
    };
    let notifications = ent_web_ui::use_notifications();
    let navigate = use_navigate();
    let reconnecter = retour_connexion();

    let catalogue = StoredValue::new(catalogue);
    let edition = RwSignal::new(depart.clone());
    let initial = RwSignal::new(depart.clone());
    let nom = RwSignal::new(depart.nom.clone());
    let description = RwSignal::new(depart.description.clone());
    let modele_choisi = RwSignal::new(String::new());
    let erreurs_champs = RwSignal::new(Vec::<(&'static str, String)>::new());
    let erreur_envoi = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let confirmation_ouverte = RwSignal::new(false);
    let suppression_ouverte = RwSignal::new(false);

    // Classes et groupes : chargés seulement quand un droit « désignés » est coché.
    let besoin_cibles = RwSignal::new(
        depart
            .droits_a_envoyer()
            .iter()
            .any(|d| d.portee == PORTEE_DESIGNES),
    );
    let cibles = LocalResource::new(move || {
        let besoin = besoin_cibles.get();
        async move {
            if besoin {
                charger_cibles().await
            } else {
                Ok(Vec::new())
            }
        }
    });

    let courant = move || {
        let mut e = edition.get();
        e.nom = nom.get();
        e.description = description.get();
        e
    };
    let modifie = Memo::new(move |_| courant() != initial.get());

    let erreur_champ = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs_champs
                .read()
                .iter()
                .find(|(c, _)| *c == champ)
                .map(|(_, m)| m.clone())
        })
    };

    // Pré-remplissage depuis un modèle (création seulement).
    let appliquer_modele = Callback::new(move |_: ()| {
        let id = modele_choisi.get_untracked();
        if let Some(m) = modele_par_id(&id) {
            let e = catalogue.with_value(|c| EditionRole::depuis_modele(m, c));
            nom.set(e.nom.clone());
            description.set(e.description.clone());
            edition.set(e);
            erreurs_champs.set(Vec::new());
            annonce.set(format!(
                "Modèle « {} » appliqué : vérifiez les droits.",
                m.nom
            ));
        }
    });

    let enregistrer = {
        let reconnecter = reconnecter.clone();
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            if en_cours.get_untracked() {
                return;
            }
            let e = courant_untracked(edition, nom, description);
            en_cours.set(true);
            erreur_envoi.set(None);
            let saisie = e.saisie();
            let reconnecter = reconnecter.clone();
            let navigate = navigate.clone();
            spawn_local(async move {
                let client = session::client();
                let resultat = match existant {
                    Some(id) => client.modifier_role(id, &saisie).await,
                    None => client.creer_role(&saisie).await,
                };
                en_cours.set(false);
                confirmation_ouverte.set(false);
                match resultat {
                    Ok(r) => {
                        if existant.is_some() {
                            let nouveau = EditionRole::depuis_role(&r);
                            initial.set(nouveau.clone());
                            edition.set(nouveau);
                            notifications
                                .succes("Rôle enregistré : les droits s'appliquent immédiatement.");
                            annonce.set("Rôle enregistré.".into());
                        } else {
                            notifications.succes("Rôle créé. Vous pouvez maintenant l'attribuer.");
                            navigate(&adresse_role(r.id), Default::default());
                        }
                    }
                    Err(ApiError::NonConnecte) => reconnecter(),
                    Err(err) => {
                        let m = message_erreur_role(&err);
                        annonce.set(m.clone());
                        erreur_envoi.set(Some(m));
                    }
                }
            });
        })
    };

    let soumettre = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let e = courant_untracked(edition, nom, description);
        let erreurs = e.erreurs();
        if !erreurs.is_empty() {
            annonce.set(
                erreurs
                    .iter()
                    .map(|(_, m)| m.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            erreurs_champs.set(erreurs);
            return;
        }
        erreurs_champs.set(Vec::new());
        if existant.is_some() {
            confirmation_ouverte.set(true);
        } else {
            enregistrer.run(());
        }
    };

    let supprimer = {
        let reconnecter = reconnecter.clone();
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            let Some(id) = existant else { return };
            let reconnecter = reconnecter.clone();
            let navigate = navigate.clone();
            en_cours.set(true);
            spawn_local(async move {
                let resultat = session::client().supprimer_role(id).await;
                en_cours.set(false);
                match resultat {
                    Ok(()) => {
                        notifications.succes("Rôle supprimé.");
                        navigate(navigation::ROLES, Default::default());
                    }
                    Err(ApiError::NonConnecte) => reconnecter(),
                    Err(err) => {
                        let m = message_erreur_role(&err);
                        annonce.set(m.clone());
                        erreur_envoi.set(Some(m));
                    }
                }
            });
        })
    };

    let titre = if existant.is_some() {
        nom_role.clone()
    } else {
        "Nouveau rôle".to_string()
    };
    let nombre_membres = role.as_ref().map(|r| r.membres.len()).unwrap_or(0);
    let message_suppression = format!(
        "Supprimer le rôle « {nom_role} » ? Les {nombre_membres} personne(s) qui le portent perdent ces droits immédiatement ; elles gardent les droits de leur profil. Cette action est définitive."
    );
    let options_modeles: Vec<(String, String)> =
        std::iter::once((String::new(), "Choisir un modèle…".to_string()))
            .chain(
                MODELES
                    .iter()
                    .map(|m| (m.id.to_string(), m.nom.to_string())),
            )
            .collect();
    let resume = Signal::derive(move || courant().resume_changements(&initial.get()));

    view! {
        <EnTetePage
            titre=titre
            surtitre="Rôles et droits".to_string()
            sous_titre="Cochez ce que ce rôle permet, et sur quels élèves.".to_string()
        >
            <a class=ent_web_ui::classes_bouton(Variante::Secondaire, ent_web_ui::Taille::Normale) href=navigation::ROLES>
                "Retour à la liste"
            </a>
        </EnTetePage>
        <form class="roles__formulaire" on:submit=soumettre novalidate=true>
            {existant.is_none().then(|| view! {
                <section class="securite__bloc" aria-labelledby="roles-modele-titre">
                    <h2 id="roles-modele-titre" class="securite__titre">"Point de départ"</h2>
                    <p class="securite__aide">
                        "Un modèle pré-remplit le nom et les droits ; tout reste modifiable avant l'enregistrement. "
                        "Vos modifications actuelles seront remplacées."
                    </p>
                    <div class="roles__modele">
                        <Selection id="roles-modele" libelle="Modèle" valeur=modele_choisi options=options_modeles/>
                        <Bouton variante=Variante::Secondaire on_click=appliquer_modele
                            desactive=Signal::derive(move || modele_choisi.get().is_empty())>
                            "Appliquer le modèle"
                        </Bouton>
                    </div>
                </section>
            })}
            <section class="securite__bloc" aria-labelledby="roles-identite-titre">
                <h2 id="roles-identite-titre" class="securite__titre">"Identité du rôle"</h2>
                <ChampTexte id="role-nom" libelle="Nom du rôle" valeur=nom requis=true
                    aide="80 caractères au plus (ex. « Professeur de théâtre »)."
                    erreur=erreur_champ("nom")/>
                <ZoneTexte id="role-description" libelle="Description (facultative)" valeur=description
                    erreur=erreur_champ("description")/>
            </section>
            <TableauDroits catalogue=catalogue edition=edition besoin_cibles=besoin_cibles cibles=cibles/>
            {move || {
                erreur_champ("droits")
                    .get()
                    .map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })
            }}
            {move || erreur_envoi.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <div class="roles__actions">
                <Bouton soumettre=true variante=Variante::Primaire en_cours=en_cours
                    desactive=Signal::derive(move || existant.is_some() && !modifie.get())>
                    {if existant.is_some() { "Enregistrer les modifications" } else { "Créer le rôle" }}
                </Bouton>
                {existant.is_some().then(|| view! {
                    <Bouton variante=Variante::Danger on_click=Callback::new(move |_| suppression_ouverte.set(true))>
                        "Supprimer le rôle"
                    </Bouton>
                })}
            </div>
        </form>
        <Dialogue ouvert=confirmation_ouverte titre="Enregistrer les modifications ?" id="dialogue-role">
            <p>{move || resume.get()}</p>
            <p>"Les droits s'appliquent immédiatement à toutes les personnes qui portent ce rôle."</p>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmation_ouverte.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton variante=Variante::Primaire en_cours=en_cours on_click=enregistrer>
                    "Confirmer"
                </Bouton>
            </div>
        </Dialogue>
        {existant.is_some().then(|| view! {
            <Confirmation ouvert=suppression_ouverte id="dialogue-suppression-role"
                titre="Supprimer ce rôle ?" message=message_suppression
                libelle_confirmer="Supprimer" danger=true on_confirmer=supprimer/>
        })}
        {existant.map(|id| view! {
            <SectionMembres role_id=id membres_initiaux=role.as_ref().map(|r| r.membres.clone()).unwrap_or_default() personnel=personnel.clone()/>
        })}
        <Annonce message=Signal::derive(move || annonce.get())/>
    }
}

fn courant_untracked(
    edition: RwSignal<EditionRole>,
    nom: RwSignal<String>,
    description: RwSignal<String>,
) -> EditionRole {
    let mut e = edition.get_untracked();
    e.nom = nom.get_untracked();
    e.description = description.get_untracked();
    e
}

/// Tableau « action × portée » : une case par droit, accessible au clavier.
#[component]
fn TableauDroits(
    catalogue: StoredValue<CatalogueRoles>,
    edition: RwSignal<EditionRole>,
    besoin_cibles: RwSignal<bool>,
    cibles: LocalResource<Result<Vec<Cible>, ApiError>>,
) -> impl IntoView {
    let portees: Vec<(String, &'static str, &'static str)> = catalogue.with_value(|c| {
        c.portees
            .iter()
            .map(|p| {
                (
                    p.code.clone(),
                    libelle_court_portee(&p.code),
                    aide_portee(&p.code),
                )
            })
            .collect()
    });
    let actions = catalogue.with_value(|c| c.actions.clone());
    let portees = StoredValue::new(portees);
    let colonnes = portees.with_value(|p| p.len()) + 1;

    let lignes = actions
        .into_iter()
        .map(|a| {
            let code = a.code.clone();
            let libelle = a.libelle.clone();
            let cellules = portees
                .get_value()
                .into_iter()
                .map(|(portee, court, _)| {
                    let admise = a.portees.contains(&portee);
                    if !admise {
                        return view! {
                            <td class="roles__cellule roles__cellule--vide">
                                <span class="ui-sr">{format!("{court} : non disponible pour cette action")}</span>
                                <span aria-hidden="true">"—"</span>
                            </td>
                        }
                        .into_any();
                    }
                    let (action_c, portee_c) = (code.clone(), portee.clone());
                    let (action_e, portee_e) = (code.clone(), portee.clone());
                    let nom_accessible = format!("{libelle} — {court}");
                    let id = format!("droit-{code}-{portee}");
                    let pour = id.clone();
                    let est_designes = portee == PORTEE_DESIGNES;
                    view! {
                        <td class="roles__cellule">
                            <label class="roles__case" for=pour>
                                <input
                                    id=id
                                    type="checkbox"
                                    prop:checked=move || edition.with(|e| e.coche(&action_c, &portee_c))
                                    on:change=move |ev| {
                                        let coche = event_target_checked(&ev);
                                        if coche && est_designes {
                                            besoin_cibles.set(true);
                                        }
                                        catalogue.with_value(|cat| {
                                            edition.update(|e| {
                                                e.definir(&action_e, &portee_e, coche, cat);
                                            })
                                        });
                                    }
                                />
                                <span class="roles__case-nom" aria-hidden="true">{court}</span>
                                <span class="ui-sr">{nom_accessible}</span>
                            </label>
                        </td>
                    }
                    .into_any()
                })
                .collect_view();
            let code_cibles = code.clone();
            let libelle_cibles = libelle.clone();
            view! {
                <tr>
                    <th scope="row" class="roles__action">{libelle.clone()}</th>
                    {cellules}
                </tr>
                {move || {
                    let code = code_cibles.clone();
                    let libelle = libelle_cibles.clone();
                    edition
                        .with(|e| e.coche(&code, PORTEE_DESIGNES))
                        .then(|| view! { <LigneCibles action=code libelle=libelle edition=edition cibles=cibles colonnes=colonnes/> })
                }}
            }
        })
        .collect_view();

    view! {
        <section class="securite__bloc" aria-labelledby="roles-droits-titre">
            <h2 id="roles-droits-titre" class="securite__titre">"Droits du rôle"</h2>
            <ul class="roles__legende">
                {portees
                    .get_value()
                    .into_iter()
                    .map(|(_, court, aide)| view! { <li><strong>{court}</strong>" : "{aide}</li> })
                    .collect_view()}
            </ul>
            <div class="roles__defilement" role="region" aria-label="Tableau des droits" tabindex="0">
                <table class="roles__tableau">
                    <caption class="ui-sr">"Droits accordés par ce rôle, par action et par portée"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Action"</th>
                            {portees
                                .get_value()
                                .into_iter()
                                .map(|(_, court, _)| view! { <th scope="col">{court}</th> })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody>{lignes}</tbody>
                </table>
            </div>
        </section>
    }
}

/// Choix des classes et groupes d'un droit « désignés ».
#[component]
fn LigneCibles(
    action: String,
    libelle: String,
    edition: RwSignal<EditionRole>,
    cibles: LocalResource<Result<Vec<Cible>, ApiError>>,
    colonnes: usize,
) -> impl IntoView {
    let legende = format!("Classes et groupes désignés pour « {libelle} »");
    view! {
        <tr class="roles__cibles">
            <td colspan=colonnes.to_string()>
                <fieldset class="roles__cibles-champ">
                    <legend>{legende}</legend>
                    {move || match cibles.get() {
                        None => view! { <p class="securite__aide">"Chargement des classes et groupes…"</p> }.into_any(),
                        Some(Err(e)) => view! { <Alerte niveau=Niveau::Erreur titre=e.message_utilisateur()/> }.into_any(),
                        Some(Ok(liste)) if liste.is_empty() => view! {
                            <p class="securite__aide">"Aucune classe ni aucun groupe dans le référentiel de l'année courante."</p>
                        }
                        .into_any(),
                        Some(Ok(liste)) => {
                            let action = action.clone();
                            view! {
                                <div class="roles__cibles-liste">
                                    {liste
                                        .into_iter()
                                        .map(|c| {
                                            let (a1, a2) = (action.clone(), action.clone());
                                            let id = format!("cible-{action}-{}", c.id);
                                            let pour = id.clone();
                                            let cid = c.id;
                                            view! {
                                                <label class="ui-case" for=pour>
                                                    <input
                                                        id=id
                                                        type="checkbox"
                                                        prop:checked=move || edition.with(|e| e.cibles(&a1).contains(&cid))
                                                        on:change=move |ev| {
                                                            let coche = event_target_checked(&ev);
                                                            edition.update(|e| e.definir_cible(&a2, cid, coche));
                                                        }
                                                    />
                                                    <span>{libelle_cible(&c)}</span>
                                                </label>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }
                    }}
                </fieldset>
            </td>
        </tr>
    }
}

/// Membres du rôle : ajout (avec date de fin facultative) et retrait.
#[component]
fn SectionMembres(
    role_id: Uuid,
    membres_initiaux: Vec<Uuid>,
    personnel: Vec<PersonnelApi>,
) -> impl IntoView {
    let notifications = ent_web_ui::use_notifications();
    let reconnecter = retour_connexion();
    let membres = RwSignal::new(membres_initiaux);
    let personnel = StoredValue::new(personnel);
    let recherche = RwSignal::new(String::new());
    let choix = RwSignal::new(String::new());
    let fin = RwSignal::new(String::new());
    let erreur_fin = RwSignal::new(None::<String>);
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let a_retirer = RwSignal::new(None::<Uuid>);
    let retrait_ouvert = RwSignal::new(false);

    let propositions = Memo::new(move |_| {
        personnel.with_value(|p| candidats(p, &membres.get(), &recherche.get()))
    });

    let attribuer = {
        let reconnecter = reconnecter.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            if en_cours.get_untracked() {
                return;
            }
            let Ok(user) = Uuid::parse_str(&choix.get_untracked()) else {
                erreur.set(Some("Choisissez une personne dans la liste.".into()));
                return;
            };
            let date = match lire_date_fin(&fin.get_untracked(), aujourdhui()) {
                Ok(d) => d,
                Err(m) => {
                    erreur_fin.set(Some(m.into()));
                    return;
                }
            };
            erreur_fin.set(None);
            erreur.set(None);
            en_cours.set(true);
            let reconnecter = reconnecter.clone();
            spawn_local(async move {
                let resultat = session::client().attribuer_role(role_id, user, date).await;
                en_cours.set(false);
                match resultat {
                    Ok(()) => {
                        membres.update(|m| {
                            if !m.contains(&user) {
                                m.push(user);
                            }
                        });
                        choix.set(String::new());
                        fin.set(String::new());
                        let nom = personnel.with_value(|p| etiquette_membre(p, user));
                        notifications.succes(format!("Rôle attribué à {nom}."));
                        annonce.set(format!("Rôle attribué à {nom}."));
                    }
                    Err(ApiError::NonConnecte) => reconnecter(),
                    Err(e) => {
                        let m = message_erreur_attribution(&e);
                        annonce.set(m.clone());
                        erreur.set(Some(m));
                    }
                }
            });
        }
    };

    let retirer = {
        let reconnecter = reconnecter.clone();
        Callback::new(move |_: ()| {
            let Some(user) = a_retirer.get_untracked() else {
                return;
            };
            en_cours.set(true);
            erreur.set(None);
            let reconnecter = reconnecter.clone();
            spawn_local(async move {
                let resultat = session::client().retirer_role(role_id, user).await;
                en_cours.set(false);
                retrait_ouvert.set(false);
                match resultat {
                    Ok(()) | Err(ApiError::Introuvable(_)) => {
                        membres.update(|m| m.retain(|x| *x != user));
                        let nom = personnel.with_value(|p| etiquette_membre(p, user));
                        notifications.succes(format!("Rôle retiré à {nom}."));
                        annonce.set(format!("Rôle retiré à {nom}."));
                    }
                    Err(ApiError::NonConnecte) => reconnecter(),
                    Err(e) => {
                        let m = message_erreur_attribution(&e);
                        annonce.set(m.clone());
                        erreur.set(Some(m));
                    }
                }
            });
        })
    };

    let nom_a_retirer = Signal::derive(move || {
        a_retirer
            .get()
            .map(|u| personnel.with_value(|p| etiquette_membre(p, u)))
            .unwrap_or_default()
    });

    view! {
        <section class="securite__bloc" aria-labelledby="roles-membres-titre">
            <h2 id="roles-membres-titre" class="securite__titre">"Membres"</h2>
            <p class="securite__aide">
                "Les droits de ce rôle s'ajoutent à ceux du profil de la personne et de ses autres rôles. "
                "Une date de fin retire le rôle automatiquement."
            </p>
            {move || {
                let liste = membres.get();
                if liste.is_empty() {
                    view! { <p>"Personne n'a encore ce rôle."</p> }.into_any()
                } else {
                    view! {
                        <ul class="securite__liste">
                            {liste
                                .into_iter()
                                .map(|u| {
                                    let nom = personnel.with_value(|p| etiquette_membre(p, u));
                                    let libelle = format!("Retirer le rôle à {nom}");
                                    view! {
                                        <li class="securite__element">
                                            <span class="securite__texte"><strong>{nom}</strong></span>
                                            <Bouton variante=Variante::Secondaire libelle_accessible=libelle
                                                on_click=Callback::new(move |_| {
                                                    a_retirer.set(Some(u));
                                                    retrait_ouvert.set(true);
                                                })>
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
            <form class="securite__formulaire" on:submit=attribuer novalidate=true>
                <h3 class="securite__sous-titre">"Attribuer le rôle"</h3>
                <ChampTexte id="membre-recherche" libelle="Filtrer le personnel" valeur=recherche genre="search"
                    aide="Nom ou adresse e-mail. Seul le personnel peut recevoir un rôle."/>
                {move || {
                    let options: Vec<(String, String)> = std::iter::once((String::new(), format!("Choisir une personne ({})", propositions.get().len())))
                        .chain(propositions.get().into_iter().map(|p| (p.user_id.to_string(), format!("{} — {}", p.label, p.email))))
                        .collect();
                    view! { <Selection id="membre-choix" libelle="Personne" valeur=choix options=options requis=true/> }
                }}
                <ChampTexte id="membre-fin" libelle="Date de fin (facultative)" valeur=fin genre="date"
                    erreur=Signal::derive(move || erreur_fin.get())/>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <div>
                    <Bouton soumettre=true variante=Variante::Primaire en_cours=en_cours>"Attribuer"</Bouton>
                </div>
            </form>
        </section>
        <Dialogue ouvert=retrait_ouvert titre="Retirer ce rôle ?" id="dialogue-retrait-role">
            <p>
                {move || format!("{} perd immédiatement les droits de ce rôle ; elle garde ceux de son profil et de ses autres rôles.", nom_a_retirer.get())}
            </p>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| retrait_ouvert.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton variante=Variante::Danger en_cours=en_cours on_click=retirer>"Retirer"</Bouton>
            </div>
        </Dialogue>
        <Annonce message=Signal::derive(move || annonce.get())/>
    }
}
