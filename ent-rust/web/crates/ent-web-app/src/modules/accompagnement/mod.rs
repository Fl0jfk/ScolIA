//! Accompagnement des élèves à besoins particuliers (socle 2, lot S2-11, ACC-01→05) : dispositifs
//! (PAP, PPS, PPRE, ULIS), aménagements pédagogiques et d'examen, AESH, équipe éducative,
//! révision et clôture.
//!
//! Routes d'`ent-api/src/sante.rs`. Le serveur décide des droits (`accompagnement.gerer`,
//! `sante.saisir` pour ouvrir un PAP) ; l'écran n'est proposé qu'à qui a `accompagnement.gerer`
//! et traite les refus (403). Les aménagements viennent d'un catalogue fermé : jamais de
//! diagnostic dans ce que voient les professeurs.

pub mod modele;

use ent_types::sante::{
    AmenagementDto, CatalogueSanteDto, DispositifDto, DossierAccompagnementDto,
};
use ent_web_ui::{
    Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Selection, Squelette, Taille, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map};
use uuid::Uuid;

use crate::composants_metier::recherche_eleve::RechercheEleve;
use crate::composants_metier::sante::{
    delai_texte, libelle_kind_dispositif, libelle_portee_amenagement, libelle_reunion, ton_delai,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::date_complete;
use modele::{
    composer_aesh, composer_amenagement, composer_dispositif, composer_reunion, ErreurChamp,
    SaisieAesh, SaisieAmenagement, SaisieDispositif, KINDS, PORTEES, REUNIONS,
};

pub fn adresse_eleve(id: Uuid) -> String {
    format!("{}/eleves/{id}", navigation::ACCOMPAGNEMENT)
}

fn erreur_de(erreurs: RwSignal<Vec<ErreurChamp>>, champ: &'static str) -> Signal<Option<String>> {
    Signal::derive(move || {
        erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
    })
}

fn options(liste: &[(&str, &str)], premier: &str) -> Vec<(String, String)> {
    std::iter::once((String::new(), premier.to_string()))
        .chain(liste.iter().map(|(c, l)| (c.to_string(), l.to_string())))
        .collect()
}

// ---------------------------------------------------------------------------
// Accueil : dispositifs à réviser, recherche d'un élève
// ---------------------------------------------------------------------------

#[component]
pub fn PageAccompagnement() -> impl IntoView {
    let a_reviser =
        LocalResource::new(|| async { session::client().dispositifs_a_reviser(30).await });
    let navigate = use_navigate();
    let ouvrir = Callback::new(move |(id, _): (Uuid, String)| {
        navigate(&adresse_eleve(id), Default::default());
    });
    view! {
        <Title text="Accompagnement"/>
        <EnTetePage
            surtitre="Vie scolaire".to_string()
            titre="Accompagnement".to_string()
            sous_titre="PAP, PPS, PPRE, ULIS : dispositifs, aménagements pédagogiques et d'examen, AESH, équipe éducative.".to_string()
        />
        <section class="ra-section" aria-labelledby="acc-reviser">
            <h2 id="acc-reviser" class="vs-section-titre">"Dispositifs à réviser (30 jours)"</h2>
            {move || match a_reviser.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des révisions…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| a_reviser.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune révision en vue" message="Aucun dispositif n'arrive à échéance dans les 30 jours."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|d| {
                            let href = adresse_eleve(d.eleve_id);
                            let nom = format!("{} {}", d.prenom, d.nom);
                            view! {
                                <li class="jvs-ligne">
                                    <a href=href>{format!("{nom}{}", d.classe.as_deref().map(|c| format!(" · {c}")).unwrap_or_default())}</a>
                                    <span class="texte-attenue">{libelle_kind_dispositif(&d.kind)}</span>
                                    <Badge libelle=format!("Révision {}", delai_texte(d.jours_restants)) ton=ton_delai(d.jours_restants)/>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="acc-dossier">
            <h2 id="acc-dossier" class="vs-section-titre">"Suivre un élève"</h2>
            <RechercheEleve
                id="acc-eleve-q"
                libelle="Rechercher un élève (nom, prénom ou classe)"
                action="Ouvrir l'accompagnement de"
                on_choisir=ouvrir
            />
        </section>
    }
}

// ---------------------------------------------------------------------------
// Suivi d'un élève
// ---------------------------------------------------------------------------

#[component]
pub fn PageAccompagnementEleve() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|i| i.parse::<Uuid>().ok())
    });
    view! {
        <Title text="Accompagnement de l'élève"/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::ACCOMPAGNEMENT>"Accompagnement"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">"Suivi de l'élève"</span>
        </nav>
        {move || match id.get() {
            None => view! { <EtatVide titre="Élève introuvable" message="Identifiant d'élève invalide."/> }.into_any(),
            Some(id) => view! { <SuiviEleve eleve_id=id/> }.into_any(),
        }}
    }
}

#[component]
fn SuiviEleve(eleve_id: Uuid) -> impl IntoView {
    let dossier = LocalResource::new(move || async move {
        session::client().dossier_accompagnement(eleve_id).await
    });
    let catalogue = LocalResource::new(|| async { session::client().catalogue_sante().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| dossier.refetch()));
    view! {
        <RetourEcriture ecriture=ecriture/>
        <EnTetePage titre="Accompagnement".to_string() sous_titre="Les professeurs de l'élève ne voient que les aménagements, jamais le dispositif ni son libellé.".to_string()/>
        {move || match (dossier.get(), catalogue.get()) {
            (None, _) | (_, None) => view! { <Squelette lignes=8 libelle="Chargement du suivi…"/> }.into_any(),
            (Some(Err(e)), _) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| dossier.refetch())/> }.into_any(),
            (_, Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| catalogue.refetch())/> }.into_any(),
            (Some(Ok(d)), Some(Ok(c))) => view! { <ContenuSuivi d=d catalogue=c eleve_id=eleve_id ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuSuivi(
    d: DossierAccompagnementDto,
    catalogue: CatalogueSanteDto,
    eleve_id: Uuid,
    ecriture: Ecriture,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let actifs: Vec<DispositifDto> = d
        .dispositifs
        .iter()
        .filter(|x| x.statut == "actif")
        .cloned()
        .collect();
    view! {
        <section class="ra-section" aria-labelledby="acc-dispositifs">
            <h2 id="acc-dispositifs" class="vs-section-titre">"Dispositifs"</h2>
            <ListeDispositifs dispositifs=d.dispositifs.clone() ecriture=ecriture/>
            {move || moteur.get().peut_gerer_accompagnement().then(|| view! { <FormulaireDispositif eleve_id=eleve_id ecriture=ecriture/> })}
        </section>
        <section class="ra-section" aria-labelledby="acc-amenagements">
            <h2 id="acc-amenagements" class="vs-section-titre">"Aménagements (visibles des professeurs de l'élève)"</h2>
            <ListeAmenagements amenagements=d.amenagements.clone() ecriture=ecriture/>
            {
                let (catalogue, actifs) = (catalogue.clone(), actifs.clone());
                move || moteur.get().peut_gerer_accompagnement().then(|| view! {
                    <FormulaireAmenagement eleve_id=eleve_id catalogue=catalogue.clone() dispositifs=actifs.clone() ecriture=ecriture/>
                })
            }
        </section>
        <section class="ra-section" aria-labelledby="acc-aesh">
            <h2 id="acc-aesh" class="vs-section-titre">"AESH"</h2>
            {if d.aesh.is_empty() {
                view! { <EtatVide titre="Aucun AESH affecté" message="Aucune affectation enregistrée."/> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {d.aesh.iter().map(|a| view! {
                            <li class="jvs-ligne">
                                <span><strong>{a.aesh_nom.clone()}</strong>{a.heures_hebdo.map(|h| format!(" · {h} h par semaine"))}</span>
                                <span class="texte-attenue">{format!("depuis le {}{}", date_complete(a.date_debut), a.date_fin.map(|f| format!(" jusqu'au {}", date_complete(f))).unwrap_or_default())}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            {move || moteur.get().peut_gerer_accompagnement().then(|| view! { <FormulaireAesh eleve_id=eleve_id ecriture=ecriture/> })}
        </section>
        <section class="ra-section" aria-labelledby="acc-reunions">
            <h2 id="acc-reunions" class="vs-section-titre">"Équipe éducative et de suivi"</h2>
            {if d.reunions.is_empty() {
                view! { <EtatVide titre="Aucune réunion" message="Aucune réunion planifiée ni tenue."/> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {d.reunions.iter().map(|r| {
                            let (id, tenue) = (r.id, r.statut == "tenue");
                            view! {
                                <li class="jvs-ligne">
                                    <div>
                                        <strong>{libelle_reunion(&r.kind)}</strong>
                                        <span class="texte-attenue">{format!(" — {}", date_complete(r.date_reunion))}</span>
                                        {(!r.compte_rendu.is_empty()).then(|| view! { <p>{r.compte_rendu.clone()}</p> })}
                                    </div>
                                    <Badge libelle=if tenue { "Tenue" } else { "Planifiée" } ton=if tenue { TonEtat::Ok } else { TonEtat::Info }/>
                                    {(!tenue).then(|| view! { <CompteRendu reunion=id ecriture=ecriture/> })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            {move || moteur.get().peut_gerer_accompagnement().then(|| view! { <FormulaireReunion eleve_id=eleve_id ecriture=ecriture/> })}
        </section>
    }
}

#[component]
fn ListeDispositifs(dispositifs: Vec<DispositifDto>, ecriture: Ecriture) -> impl IntoView {
    if dispositifs.is_empty() {
        return view! { <EtatVide titre="Aucun dispositif" message="Aucun dispositif ouvert pour cet élève."/> }.into_any();
    }
    let moteur = use_droits_moteur();
    view! {
        <ul class="jvs-liste">
            {dispositifs.into_iter().map(|d| {
                let (id, actif) = (d.id, d.statut == "actif");
                let nom = libelle_kind_dispositif(&d.kind);
                let a = format!("Prolonger la révision du {nom}");
                let b = format!("Clore le {nom}");
                let revision = d.date_revision.map(|r| {
                    let jours = (r - crate::noyau::temps::aujourdhui()).num_days() as i32;
                    (r, jours)
                });
                view! {
                    <li class="jvs-ligne">
                        <div>
                            <strong>{nom}</strong>
                            {d.libelle.clone().map(|l| format!(" — {l}"))}
                            <span class="texte-attenue">{format!(" · depuis le {}", date_complete(d.date_debut))}</span>
                        </div>
                        <Badge libelle=if actif { "Actif" } else { "Clos" } ton=if actif { TonEtat::Ok } else { TonEtat::Neutre }/>
                        {actif.then(|| revision.map(|(r, j)| view! {
                            <Badge libelle=format!("Révision {} ({})", delai_texte(j), date_complete(r)) ton=ton_delai(j)/>
                        }))}
                        {move || (actif && moteur.get().peut_gerer_accompagnement()).then(|| {
                            let (a, b) = (a.clone(), b.clone());
                            view! {
                                <div class="ra-actions">
                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours libelle_accessible=a
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client().reviser_dispositif(id, &ent_types::sante::ReviserDispositifBody { date_revision: None, clore: false }).await?;
                                                Ok("Révision enregistrée : prochaine échéance dans un an.".to_string())
                                            });
                                        })>
                                        "Révisé (+ 1 an)"
                                    </Bouton>
                                    <Bouton variante=Variante::Discret taille=Taille::Compacte en_cours=ecriture.en_cours libelle_accessible=b
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client().reviser_dispositif(id, &ent_types::sante::ReviserDispositifBody { date_revision: None, clore: true }).await?;
                                                Ok("Dispositif clos ; ses aménagements sont retirés.".to_string())
                                            });
                                        })>
                                        "Clore"
                                    </Bouton>
                                </div>
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
fn FormulaireDispositif(eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let (kind, libelle, debut, revision) = (
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    );
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let s = SaisieDispositif {
            kind: kind.get_untracked(),
            libelle: libelle.get_untracked(),
            debut: debut.get_untracked(),
            revision: revision.get_untracked(),
        };
        match composer_dispositif(&s) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client()
                        .ouvrir_dispositif(eleve_id, &corps)
                        .await?;
                    Ok("Dispositif ouvert.".to_string())
                });
                libelle.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="dis-titre">
            <h3 id="dis-titre" class="jvs-sous-titre">"Ouvrir un dispositif"</h3>
            <Selection id="dis-kind" libelle="Type" valeur=kind options=options(&KINDS, "— choisir —") requis=true erreur=erreur_de(erreurs, "dis-kind")/>
            <ChampTexte id="dis-libelle" libelle="Libellé court (facultatif, sans diagnostic)" valeur=libelle erreur=erreur_de(erreurs, "dis-libelle")/>
            <ChampTexte id="dis-debut" libelle="Début (aujourd'hui par défaut)" valeur=debut genre="date" erreur=erreur_de(erreurs, "dis-debut")/>
            <ChampTexte id="dis-revision" libelle="Prochaine révision (un an par défaut)" valeur=revision genre="date" erreur=erreur_de(erreurs, "dis-revision")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Ouvrir le dispositif"</Bouton>
        </form>
    }
}

#[component]
fn ListeAmenagements(amenagements: Vec<AmenagementDto>, ecriture: Ecriture) -> impl IntoView {
    if amenagements.is_empty() {
        return view! { <EtatVide titre="Aucun aménagement" message="Aucun aménagement défini pour cet élève."/> }.into_any();
    }
    let moteur = use_droits_moteur();
    view! {
        <ul class="jvs-liste">
            {amenagements.into_iter().map(|a| {
                let id = a.id;
                let retirer = format!("Retirer l'aménagement « {} »", a.libelle);
                view! {
                    <li class="jvs-ligne">
                        <div>
                            <strong>{a.libelle.clone()}</strong>
                            <span class="texte-attenue">{format!(" — {}{}", libelle_portee_amenagement(&a.portee), a.date_fin.map(|f| format!(" jusqu'au {}", date_complete(f))).unwrap_or_default())}</span>
                        </div>
                        {move || moteur.get().peut_gerer_accompagnement().then(|| {
                            let r = retirer.clone();
                            view! {
                                <Bouton variante=Variante::Discret taille=Taille::Compacte en_cours=ecriture.en_cours libelle_accessible=r
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().retirer_amenagement(id).await?;
                                            Ok("Aménagement retiré.".to_string())
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
fn FormulaireAmenagement(
    eleve_id: Uuid,
    catalogue: CatalogueSanteDto,
    dispositifs: Vec<DispositifDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    let (code, portee, dispositif, fin) = (
        RwSignal::new(String::new()),
        RwSignal::new("les_deux".to_string()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    );
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let codes: Vec<(String, String)> = std::iter::once((String::new(), "— choisir —".to_string()))
        .chain(
            catalogue
                .amenagements
                .iter()
                .map(|e| (e.code.clone(), e.libelle.clone())),
        )
        .collect();
    let lies: Vec<(String, String)> =
        std::iter::once((String::new(), "Aucun dispositif en particulier".to_string()))
            .chain(dispositifs.iter().map(|d| {
                (
                    d.id.to_string(),
                    libelle_kind_dispositif(&d.kind).to_string(),
                )
            }))
            .collect();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let s = SaisieAmenagement {
            code: code.get_untracked(),
            portee: portee.get_untracked(),
            dispositif: dispositif.get_untracked(),
            fin: fin.get_untracked(),
        };
        match composer_amenagement(&s) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client()
                        .definir_amenagement(eleve_id, &corps)
                        .await?;
                    Ok("Aménagement défini.".to_string())
                });
                code.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="ame-titre">
            <h3 id="ame-titre" class="jvs-sous-titre">"Définir un aménagement"</h3>
            <Selection id="ame-code" libelle="Aménagement" valeur=code options=codes requis=true erreur=erreur_de(erreurs, "ame-code")/>
            <Selection id="ame-portee" libelle="S'applique" valeur=portee options=options(&PORTEES, "— choisir —") requis=true erreur=erreur_de(erreurs, "ame-portee")/>
            <Selection id="ame-dispositif" libelle="Dispositif lié (facultatif)" valeur=dispositif options=lies erreur=erreur_de(erreurs, "ame-dispositif")/>
            <ChampTexte id="ame-fin" libelle="Jusqu'au (facultatif)" valeur=fin genre="date" erreur=erreur_de(erreurs, "ame-fin")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Définir l'aménagement"</Bouton>
        </form>
    }
}

#[component]
fn FormulaireAesh(eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let (nom, heures, debut, fin) = (
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    );
    Effect::new(move |_| {
        if debut.get_untracked().is_empty() {
            debut.set(crate::noyau::temps::aujourdhui().to_string());
        }
    });
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let s = SaisieAesh {
            nom: nom.get_untracked(),
            heures: heures.get_untracked(),
            debut: debut.get_untracked(),
            fin: fin.get_untracked(),
        };
        match composer_aesh(&s) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client().affecter_aesh(eleve_id, &corps).await?;
                    Ok("AESH affecté.".to_string())
                });
                nom.set(String::new());
                heures.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="aes-titre">
            <h3 id="aes-titre" class="jvs-sous-titre">"Affecter un AESH"</h3>
            <ChampTexte id="aes-nom" libelle="Nom de l'AESH" valeur=nom requis=true erreur=erreur_de(erreurs, "aes-nom")/>
            <ChampTexte id="aes-heures" libelle="Heures par semaine (facultatif)" valeur=heures genre="number" inputmode="numeric" erreur=erreur_de(erreurs, "aes-heures")/>
            <ChampTexte id="aes-debut" libelle="Début" valeur=debut genre="date" requis=true erreur=erreur_de(erreurs, "aes-debut")/>
            <ChampTexte id="aes-fin" libelle="Fin (facultatif)" valeur=fin genre="date" erreur=erreur_de(erreurs, "aes-fin")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Affecter"</Bouton>
        </form>
    }
}

#[component]
fn FormulaireReunion(eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let (kind, date) = (
        RwSignal::new("equipe_suivi".to_string()),
        RwSignal::new(String::new()),
    );
    let erreurs = RwSignal::new(Vec::<ErreurChamp>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        match composer_reunion(&kind.get_untracked(), &date.get_untracked()) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client()
                        .planifier_reunion(eleve_id, &corps)
                        .await?;
                    Ok("Réunion planifiée.".to_string())
                });
                date.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="reu-titre">
            <h3 id="reu-titre" class="jvs-sous-titre">"Planifier une réunion"</h3>
            <Selection id="reu-kind" libelle="Type" valeur=kind options=options(&REUNIONS, "— choisir —") requis=true erreur=erreur_de(erreurs, "reu-kind")/>
            <ChampTexte id="reu-date" libelle="Date" valeur=date genre="date" requis=true erreur=erreur_de(erreurs, "reu-date")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Planifier"</Bouton>
        </form>
    }
}

/// Compte rendu d'une réunion planifiée (chiffré côté serveur ; la réunion devient « tenue »).
#[component]
fn CompteRendu(reunion: Uuid, ecriture: Ecriture) -> impl IntoView {
    let texte = RwSignal::new(String::new());
    let ouvert = RwSignal::new(false);
    view! {
        <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| ouvert.update(|o| *o = !*o))>
            "Compte rendu"
        </Bouton>
        {move || ouvert.get().then(|| view! {
            <div class="ra-formulaire">
                <ZoneTexte id=format!("reu-cr-{reunion}") libelle="Compte rendu (confidentiel)" valeur=texte/>
                <Bouton en_cours=ecriture.en_cours desactive=Signal::derive(move || texte.get().trim().is_empty())
                    on_click=Callback::new(move |_| {
                        let t = texte.get_untracked();
                        ecriture.lancer(async move {
                            session::client().consigner_reunion(reunion, &t).await?;
                            Ok("Compte rendu enregistré.".to_string())
                        });
                        ouvert.set(false);
                    })>
                    "Enregistrer"
                </Bouton>
            </div>
        })}
    }
}
