//! Demandes et flux de travail (socle N1, lot N1-4) : la porte unique « Écrire à l'établissement » et « Faire une
//! demande », « Mes demandes » (familles, élèves, personnel), « Ma file » (personnel), les HSE (en **heures**, jamais
//! en euros) et la reprographie (file d'impression par site).
//!
//! Routes : `/demandes…` d'`ent-api`. Le serveur décide de tout (droits, 404 neutre, versions) ; l'écran n'affiche que
//! ce qu'il reçoit et laisse le serveur répondre 403 : un rôle refusé voit « accès refusé », jamais un écran vide trompeur.
//! Charte : fond `#F5F5F7`, un seul accent bleu, tableaux denses pour la saisie, tuiles pour les tableaux de bord.

pub mod modele;

use ent_types::demandes::{
    DecisionHseDto, DemandeFile, DepotDemandeDto, DepotHseDto, EtatDemandeDto, Guichet,
    LigneCopiesDto, LigneHseDto, MaDemande, NatureGuichet, SeanceDto, ValidationCopiesDto,
};
use ent_types::vie_scolaire::EnfantFamilleDto;
use ent_web_ui::{
    Alerte, Bouton, ChampTexte, EnTetePage, EtatVide, GrilleBento, Largeur, Niveau, Selection,
    Squelette, Taille, Tuile, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{
    controler_hse, filtrer_natures, heures_vers_quarts, libelle_decision, libelle_etat_copies,
    libelle_motif_hse, libelle_statut, peut_annuler, quarts_vers_texte, MOTIFS_DECISION,
    MOTIFS_HSE,
};

// ───────────────────────── Faire une demande (porte unique) ─────────────────────────

#[component]
pub fn PageDemandes() -> impl IntoView {
    let guichet = LocalResource::new(|| async { session::client().demandes_guichet().await });
    view! {
        <Title text="Faire une demande"/>
        <EnTetePage
            titre="Faire une demande".to_string()
            sous_titre="Écrire à l'établissement : décrivez votre besoin, il arrive au bon service.".to_string()
        />
        {move || match guichet.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des démarches…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| guichet.refetch())/>
            }.into_any(),
            Some(Ok(g)) => view! { <PorteUnique guichet=g/> }.into_any(),
        }}
    }
}

#[component]
fn PorteUnique(guichet: Guichet) -> impl IntoView {
    let Guichet {
        natures,
        raccourcis,
        signalement_grave,
        numeros_urgence,
    } = guichet;
    let natures = StoredValue::new(natures);
    let requete = RwSignal::new(String::new());
    let choisie = RwSignal::new(None::<NatureGuichet>);
    let urgence = numeros_urgence.join(", ");
    view! {
        <Alerte niveau=Niveau::Avertissement titre=format!("{signalement_grave} : en cas de danger, appelez les secours ({urgence}).")/>
        <section aria-labelledby="porte-titre">
            <h2 id="porte-titre">"Que voulez-vous demander ?"</h2>
            <ChampTexte
                id="porte-recherche"
                libelle="Décrivez en quelques mots (par exemple « attestation », « panne », « mot de passe »)"
                valeur=requete
                genre="search"
            />
            {move || {
                let trouvees: Vec<NatureGuichet> =
                    natures.with_value(|n| filtrer_natures(n, &requete.get()).into_iter().cloned().collect());
                if trouvees.is_empty() {
                    view! { <EtatVide titre="Aucune démarche ne correspond" message="Essayez un autre mot, ou videz la recherche."/> }.into_any()
                } else {
                    view! {
                        <ul class="ui-liste" aria-label="Démarches proposées">
                            {trouvees.into_iter().map(|n| view! { <LigneNature nature=n choisie=choisie/> }).collect_view()}
                        </ul>
                    }.into_any()
                }
            }}
        </section>
        {(!raccourcis.is_empty()).then(|| view! {
            <section aria-labelledby="raccourcis-titre">
                <h2 id="raccourcis-titre">"Ces démarches se font ailleurs"</h2>
                <ul class="ui-liste">
                    {raccourcis.into_iter().map(|r| view! {
                        <li class="ui-liste__element">
                            <a class="ui-bouton ui-bouton--secondaire" href=r.lien>{r.libelle}</a>
                        </li>
                    }).collect_view()}
                </ul>
            </section>
        })}
        {move || choisie.get().map(|n| view! { <FormulaireDepot nature=n choisie=choisie/> })}
    }
}

#[component]
fn LigneNature(nature: NatureGuichet, choisie: RwSignal<Option<NatureGuichet>>) -> impl IntoView {
    let libelle = nature.libelle.clone();
    let accessible = format!("Faire cette demande : {libelle}");
    let delai = format!(
        "Réponse sous {} jours ouvrés environ",
        nature.delai_traitement_jours
    );
    let service = nature
        .role_traitant
        .clone()
        .map(|r| format!(" · traitée par {r}"))
        .unwrap_or_default();
    let dedie = nature.formulaire.clone();
    let n = nature.clone();
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{libelle}</span>
                <span class="ui-liste__detail">{delai}{service}</span>
            </div>
            {match dedie {
                Some(lien) => view! { <a class="ui-bouton ui-bouton--secondaire" href=lien aria-label=accessible>"Ouvrir le formulaire"</a> }.into_any(),
                None => view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=accessible
                        on_click=Callback::new(move |_| choisie.set(Some(n.clone())))>
                        "Faire cette demande"
                    </Bouton>
                }.into_any(),
            }}
        </li>
    }
}

#[component]
fn FormulaireDepot(
    nature: NatureGuichet,
    choisie: RwSignal<Option<NatureGuichet>>,
) -> impl IntoView {
    let objet = RwSignal::new(String::new());
    let texte = RwSignal::new(String::new());
    let eleve = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let envoyee = RwSignal::new(None::<EtatDemandeDto>);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let eleve_requis = nature.eleve_requis;
    let enfants = LocalResource::new(move || async move {
        if eleve_requis {
            session::client().enfants_famille().await
        } else {
            Ok(Vec::new())
        }
    });
    let code = nature.code.clone();
    let titre = nature.libelle.clone();
    let accueil = nature.message_accueil.clone();
    view! {
        <section aria-labelledby="depot-titre" class="classe-section">
            <h2 id="depot-titre">{titre}</h2>
            {accueil.map(|m| view! { <Alerte niveau=Niveau::Info titre=m/> })}
            <RetourEcriture ecriture=ecriture/>
            {move || envoyee.get().map(|e| view! {
                <Alerte niveau=Niveau::Succes titre=format!("Demande {} envoyée. Vous la retrouvez dans « Mes demandes ».", e.numero)/>
            })}
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            {eleve_requis.then(|| view! {
                {move || match enfants.get() {
                    Some(Ok(l)) => {
                        let mut options = vec![(String::new(), "Choisissez l'enfant".to_string())];
                        options.extend(l.into_iter().map(|e: EnfantFamilleDto| (e.id.to_string(), format!("{} {}", e.prenom, e.nom))));
                        view! { <Selection id="depot-eleve" libelle="Pour quel enfant ?" valeur=eleve options=options requis=true/> }.into_any()
                    }
                    _ => ().into_any(),
                }}
            })}
            <ChampTexte id="depot-objet" libelle="Objet (en une phrase)" valeur=objet requis=true/>
            <ZoneTexte id="depot-texte" libelle="Précisions" valeur=texte/>
            <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| {
                if objet.get_untracked().trim().is_empty() {
                    erreur.set(Some("L'objet de la demande est obligatoire.".into()));
                    return;
                }
                if eleve_requis && eleve.get_untracked().is_empty() {
                    erreur.set(Some("Choisissez l'enfant concerné.".into()));
                    return;
                }
                erreur.set(None);
                let depot = DepotDemandeDto {
                    nature_code: code.clone(),
                    objet: objet.get_untracked(),
                    texte: Some(texte.get_untracked()).filter(|t| !t.trim().is_empty()),
                    valeurs: vec![],
                    eleve_id: eleve.get_untracked().parse::<Uuid>().ok(),
                };
                ecriture.lancer(async move {
                    let e = session::client().demandes_deposer(&depot).await?;
                    envoyee.set(Some(e.clone()));
                    Ok(format!("Demande {} envoyée.", e.numero))
                });
            })>"Envoyer la demande"</Bouton>
            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| choisie.set(None))>"Fermer"</Bouton>
        </section>
    }
}

// ───────────────────────── Mes demandes ─────────────────────────

#[component]
pub fn PageMesDemandes() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().demandes_mes().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    view! {
        <Title text="Mes demandes"/>
        <EnTetePage
            titre="Mes demandes".to_string()
            sous_titre="Où en est chacune de vos demandes.".to_string()
        >
            <a class="ui-bouton ui-bouton--primaire" href="/web/demandes">"Faire une demande"</a>
        </EnTetePage>
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos demandes…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune demande" message="Vos demandes apparaîtront ici dès que vous en aurez envoyé une."/>
            }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="ui-liste" aria-label="Mes demandes">
                    {l.into_iter().map(|d| view! { <LigneMaDemande d=d ecriture=ecriture/> }).collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}

#[component]
fn LigneMaDemande(d: MaDemande, ecriture: Ecriture) -> impl IntoView {
    let MaDemande {
        id,
        numero,
        statut,
        objet,
        version,
        ..
    } = d;
    let annulable = peut_annuler(&statut);
    let etat = libelle_statut(&statut);
    let accessible = format!("Annuler la demande {numero}");
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{objet.unwrap_or_else(|| "Demande".to_string())}</span>
                <span class="ui-liste__detail">{format!("{numero} · {etat}")}</span>
            </div>
            {annulable.then(|| view! {
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=accessible
                    on_click=Callback::new(move |_| ecriture.lancer(async move {
                        session::client().demandes_annuler(id, version).await.map(|_| "Demande annulée.".to_string())
                    }))>"Annuler"</Bouton>
            })}
        </li>
    }
}

// ───────────────────────── Ma file (personnel) ─────────────────────────

#[component]
pub fn PageMaFile() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().demandes_ma_file().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    view! {
        <Title text="Ma file"/>
        <EnTetePage
            titre="Ma file".to_string()
            sous_titre="Les demandes adressées à vos services, les plus anciennes d'abord.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de votre file…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Rien à traiter" message="Aucune demande n'attend votre service."/>
            }.into_any(),
            Some(Ok(l)) => view! { <TableauFile lignes=l ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn TableauFile(lignes: Vec<DemandeFile>, ecriture: Ecriture) -> impl IntoView {
    view! {
        <table class="ui-tableau">
            <caption class="ui-sr">"Demandes de votre file"</caption>
            <thead>
                <tr>
                    <th scope="col">"Numéro"</th>
                    <th scope="col">"Objet"</th>
                    <th scope="col">"État"</th>
                    <th scope="col">"Urgence"</th>
                    <th scope="col"><span class="ui-sr">"Action"</span></th>
                </tr>
            </thead>
            <tbody>
                {lignes.into_iter().map(|d| view! { <LigneFile d=d ecriture=ecriture/> }).collect_view()}
            </tbody>
        </table>
    }
}

#[component]
fn LigneFile(d: DemandeFile, ecriture: Ecriture) -> impl IntoView {
    let DemandeFile {
        id,
        numero,
        statut,
        urgence,
        objet,
        prise,
        version,
        ..
    } = d;
    let accessible = format!("Prendre en charge la demande {numero}");
    let urgence = match urgence.as_str() {
        "securite" => "Sécurité",
        "haute" => "Haute",
        _ => "Normale",
    };
    view! {
        <tr>
            <th scope="row">{numero}</th>
            <td>{objet.unwrap_or_default()}</td>
            <td>{libelle_statut(&statut)}</td>
            <td>{urgence}</td>
            <td>
                {(!prise).then(|| view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=accessible
                        on_click=Callback::new(move |_| ecriture.lancer(async move {
                            session::client().demandes_prendre(id, version).await.map(|_| "Demande prise en charge.".to_string())
                        }))>"Prendre"</Bouton>
                })}
            </td>
        </tr>
    }
}

// ───────────────────────── HSE ─────────────────────────

#[component]
pub fn PageHse() -> impl IntoView {
    let mes = LocalResource::new(|| async { session::client().hse_mes().await });
    let a_decider = LocalResource::new(|| async { session::client().hse_a_decider().await });
    let enveloppe = LocalResource::new(|| async { session::client().hse_enveloppe().await });
    let recharger = Callback::new(move |_| {
        mes.refetch();
        a_decider.refetch();
        enveloppe.refetch();
    });
    let ecriture = Ecriture::nouveau(recharger);
    view! {
        <Title text="Heures supplémentaires (HSE)"/>
        <EnTetePage
            titre="Heures supplémentaires (HSE)".to_string()
            sous_titre="Comptées en heures ; la direction valide.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        <FormulaireHse ecriture=ecriture/>
        <section aria-labelledby="hse-mes-titre" class="classe-section">
            <h2 id="hse-mes-titre">"Mes HSE"</h2>
            {move || match mes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de vos HSE…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| mes.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune demande d'HSE"/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="ui-liste" aria-label="Mes HSE">
                        {l.into_iter().map(|h| view! { <LigneMesHse h=h ecriture=ecriture/> }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
        // Réservé à qui décide : les autres reçoivent un refus du serveur, qu'on n'affiche pas comme une erreur.
        {move || a_decider.get().and_then(|r| r.ok()).map(|l| view! {
            <section aria-labelledby="hse-decider-titre" class="classe-section">
                <h2 id="hse-decider-titre">"À décider"</h2>
                {move || enveloppe.get().and_then(|r| r.ok()).filter(|e| e.saisie).map(|e| view! {
                    <GrilleBento libelle="Enveloppe de l'année".to_string()>
                        <Tuile id="env-total" titre="Enveloppe".to_string() largeur=Largeur::Quatre>
                            <p>{quarts_vers_texte(e.total_quarts)}</p>
                        </Tuile>
                        <Tuile id="env-engage" titre="Accordé".to_string() largeur=Largeur::Quatre>
                            <p>{quarts_vers_texte(e.engage_quarts)}</p>
                        </Tuile>
                        <Tuile id="env-restant" titre="Restant".to_string() largeur=Largeur::Quatre>
                            <p>{quarts_vers_texte(e.restant_quarts)}</p>
                        </Tuile>
                    </GrilleBento>
                })}
                {if l.is_empty() {
                    view! { <EtatVide titre="Aucune demande à décider"/> }.into_any()
                } else {
                    view! {
                        <ul class="ui-liste" aria-label="HSE à décider">
                            {l.into_iter().map(|h| view! { <LigneADecider h=h ecriture=ecriture/> }).collect_view()}
                        </ul>
                    }.into_any()
                }}
            </section>
        })}
    }
}

#[component]
fn LigneMesHse(h: LigneHseDto, ecriture: Ecriture) -> impl IntoView {
    let _ = ecriture;
    let detail = format!(
        "{} · demandé {} · accordé {} · fait {}",
        libelle_motif_hse(&h.motif),
        quarts_vers_texte(i64::from(h.volume_demande_quarts)),
        h.volume_accorde_quarts
            .map_or("—".to_string(), |v| quarts_vers_texte(i64::from(v))),
        quarts_vers_texte(h.fait_quarts),
    );
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{format!("{} · {}", h.numero, libelle_decision(h.decision.as_deref()))}</span>
                <span class="ui-liste__detail">{detail}</span>
            </div>
        </li>
    }
}

#[component]
fn FormulaireHse(ecriture: Ecriture) -> impl IntoView {
    let motif = RwSignal::new("soutien".to_string());
    let objet = RwSignal::new(String::new());
    let heures = RwSignal::new(String::new());
    let minutes = RwSignal::new("0".to_string());
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let fait = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);
    let options: Vec<(String, String)> = MOTIFS_HSE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let min_options: Vec<(String, String)> = ["0", "15", "30", "45"]
        .iter()
        .map(|m| ((*m).to_string(), format!("{m} min")))
        .collect();
    view! {
        <section aria-labelledby="hse-demande-titre" class="classe-section">
            <h2 id="hse-demande-titre">"Demander des HSE"</h2>
            <p class="texte-attenue">"Heures déjà faites ou prévues. Le volume se compte en heures et en quarts d'heure."</p>
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Selection id="hse-motif" libelle="Motif" valeur=motif options=options requis=true/>
            <ChampTexte id="hse-objet" libelle="Objet (par exemple « Soutien brevet blanc »)" valeur=objet requis=true/>
            <ChampTexte id="hse-heures" libelle="Heures" valeur=heures genre="number" inputmode="numeric" requis=true/>
            <Selection id="hse-minutes" libelle="Minutes" valeur=minutes options=min_options/>
            <ChampTexte id="hse-du" libelle="Du" valeur=du genre="date" requis=true/>
            <ChampTexte id="hse-au" libelle="Au" valeur=au genre="date" requis=true/>
            <label class="ui-case">
                <input type="checkbox" prop:checked=move || fait.get() on:change=move |ev| fait.set(event_target_checked(&ev))/>
                <span>"Ces heures sont déjà faites"</span>
            </label>
            <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| {
                let quarts = controler_hse(
                    &objet.get_untracked(),
                    heures_vers_quarts(&heures.get_untracked(), &minutes.get_untracked()),
                    &du.get_untracked(),
                    &au.get_untracked(),
                );
                let q = match quarts {
                    Ok(q) => q,
                    Err(m) => {
                        erreur.set(Some(m.to_string()));
                        return;
                    }
                };
                let (Ok(d1), Ok(d2)) = (du.get_untracked().parse(), au.get_untracked().parse()) else {
                    erreur.set(Some("Les dates sont invalides.".into()));
                    return;
                };
                erreur.set(None);
                let seances = if fait.get_untracked() {
                    vec![SeanceDto { jour: d2, debut: None, duree_quarts: q.min(48), fait: true }]
                } else {
                    vec![]
                };
                let depot = DepotHseDto {
                    motif: motif.get_untracked(),
                    objet: objet.get_untracked(),
                    precisions: None,
                    volume_quarts: q,
                    periode_du: d1,
                    periode_au: d2,
                    lieu: None,
                    seances: if q <= 48 { seances } else { vec![] },
                    pour_le_compte_de: None,
                };
                ecriture.lancer(async move {
                    session::client().hse_deposer(&depot).await.map(|e| format!("Demande {} envoyée à la direction.", e.numero))
                });
            })>"Envoyer la demande"</Bouton>
        </section>
    }
}

#[component]
fn LigneADecider(h: LigneHseDto, ecriture: Ecriture) -> impl IntoView {
    let id = h.demande_id;
    let version = RwSignal::new(0_i32);
    let motif = RwSignal::new("budget".to_string());
    let options: Vec<(String, String)> = MOTIFS_DECISION
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let detail = format!(
        "{} · demandé {}",
        libelle_motif_hse(&h.motif),
        quarts_vers_texte(i64::from(h.volume_demande_quarts))
    );
    let numero = h.numero.clone();
    let (a1, a2) = (format!("Accorder {numero}"), format!("Refuser {numero}"));
    // La version vient d'« Ma file » : à défaut, le serveur répond 409 et l'écran demande de recharger.
    let _ = version;
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{numero}</span>
                <span class="ui-liste__detail">{detail}</span>
                <Selection id=format!("hse-motif-refus-{id}") libelle="Motif en cas de refus" valeur=motif options=options/>
            </div>
            <Bouton variante=Variante::Primaire taille=Taille::Compacte libelle_accessible=a1
                on_click=Callback::new(move |_| ecriture.lancer(async move {
                    let v = version_de(id).await?;
                    let d = DecisionHseDto { decision: "accordee".into(), volume_quarts: None, motif: None,
                        confirmer_depassement: false, motif_depassement: None, version: v };
                    session::client().hse_decider(id, &d).await.map(|r| format!("Accordée : {}.", quarts_vers_texte(i64::from(r.volume_accorde_quarts))))
                }))>"Accorder"</Bouton>
            <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=a2
                on_click=Callback::new(move |_| ecriture.lancer(async move {
                    let v = version_de(id).await?;
                    let d = DecisionHseDto { decision: "refusee".into(), volume_quarts: None, motif: Some(motif.get_untracked()),
                        confirmer_depassement: false, motif_depassement: None, version: v };
                    session::client().hse_decider(id, &d).await.map(|_| "Demande refusée.".to_string())
                }))>"Refuser"</Bouton>
        </li>
    }
}

/// La version courante de la demande, lue dans « Ma file » (le serveur la compare à l'écriture : 409 si elle a changé).
async fn version_de(id: Uuid) -> Result<i32, ent_web_api::ApiError> {
    let file = session::client().demandes_ma_file().await?;
    file.into_iter()
        .find(|d| d.id == id)
        .map(|d| d.version)
        .ok_or(ent_web_api::ApiError::Conflit(
            "La demande a changé entre-temps : rechargez la page.".to_string(),
        ))
}

// ───────────────────────── Copies (reprographie) ─────────────────────────

#[component]
pub fn PageCopies() -> impl IntoView {
    let mes = LocalResource::new(|| async { session::client().copies_mes().await });
    let file = LocalResource::new(|| async { session::client().copies_file().await });
    let a_valider = LocalResource::new(|| async { session::client().copies_a_valider().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        mes.refetch();
        file.refetch();
        a_valider.refetch();
    }));
    view! {
        <Title text="Copies"/>
        <EnTetePage titre="Copies (reprographie)".to_string() sous_titre="Vos demandes de copies et la file d'impression de votre site.".to_string()/>
        <RetourEcriture ecriture=ecriture/>
        <Alerte niveau=Niveau::Info titre="Le dépôt de fichiers à imprimer arrive avec le coffre de documents : en attendant, déposez votre demande au secrétariat."/>
        <section aria-labelledby="copies-mes-titre" class="classe-section">
            <h2 id="copies-mes-titre">"Mes copies"</h2>
            {move || match mes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de vos copies…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| mes.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune demande de copies"/> }.into_any(),
                Some(Ok(l)) => view! { <ListeCopies lignes=l ecriture=ecriture mode=ModeCopies::Lecture/> }.into_any(),
            }}
        </section>
        {move || a_valider.get().and_then(|r| r.ok()).filter(|l| !l.is_empty()).map(|l| view! {
            <section aria-labelledby="copies-valider-titre" class="classe-section">
                <h2 id="copies-valider-titre">"À valider"</h2>
                <ListeCopies lignes=l ecriture=ecriture mode=ModeCopies::Valider/>
            </section>
        })}
        {move || file.get().and_then(|r| r.ok()).map(|l| view! {
            <section aria-labelledby="copies-file-titre" class="classe-section">
                <h2 id="copies-file-titre">"File d'impression de mon site"</h2>
                {if l.is_empty() {
                    view! { <EtatVide titre="Rien à imprimer"/> }.into_any()
                } else {
                    view! { <ListeCopies lignes=l ecriture=ecriture mode=ModeCopies::Imprimer/> }.into_any()
                }}
            </section>
        })}
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModeCopies {
    Lecture,
    Valider,
    Imprimer,
}

#[component]
fn ListeCopies(lignes: Vec<LigneCopiesDto>, ecriture: Ecriture, mode: ModeCopies) -> impl IntoView {
    view! {
        <ul class="ui-liste">
            {lignes.into_iter().map(|c| {
                let LigneCopiesDto { demande_id: id, numero, etat, format, couleur, copies_total, pour_le, avertissement_delai, retrait_lieu, .. } = c;
                let detail = format!(
                    "{} · {} copies {} ({}) · pour le {}{}",
                    libelle_etat_copies(&etat),
                    copies_total,
                    if couleur { "en couleur" } else { "en noir et blanc" },
                    format.to_uppercase(),
                    pour_le,
                    retrait_lieu.map(|l| format!(" · retrait : {l}")).unwrap_or_default(),
                );
                let a = etat.clone();
                let (acc1, acc2) = (format!("Valider {numero}"), format!("Refuser {numero}"));
                let (acc3, acc4) = (format!("Imprimer {numero}"), format!("Marquer prête {numero}"));
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal">
                            <span class="ui-liste__titre">{numero}</span>
                            <span class="ui-liste__detail">{detail}</span>
                            {avertissement_delai.then(|| view! { <span class="texte-attenue">"Délai court : traitée au mieux."</span> })}
                        </div>
                        {(mode == ModeCopies::Valider).then(|| view! {
                            <Bouton variante=Variante::Primaire taille=Taille::Compacte libelle_accessible=acc1
                                on_click=Callback::new(move |_| ecriture.lancer(async move {
                                    session::client().copies_valider(id, &ValidationCopiesDto { accepter: true, note: None }).await
                                        .map(|_| "Demande validée.".to_string())
                                }))>"Valider"</Bouton>
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=acc2
                                on_click=Callback::new(move |_| ecriture.lancer(async move {
                                    session::client().copies_valider(id, &ValidationCopiesDto { accepter: false,
                                        note: Some("Refusée par la direction".into()) }).await.map(|_| "Demande refusée.".to_string())
                                }))>"Refuser"</Bouton>
                        })}
                        {(mode == ModeCopies::Imprimer && a == "a_imprimer").then(|| view! {
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=acc3
                                on_click=Callback::new(move |_| ecriture.lancer(async move {
                                    session::client().copies_impression(id).await.map(|_| "En impression.".to_string())
                                }))>"Imprimer"</Bouton>
                        })}
                        {(mode == ModeCopies::Imprimer && matches!(a.as_str(), "a_imprimer" | "en_impression")).then(|| view! {
                            <Bouton variante=Variante::Primaire taille=Taille::Compacte libelle_accessible=acc4
                                on_click=Callback::new(move |_| ecriture.lancer(async move {
                                    session::client().copies_prete(id, None).await.map(|_| "Copies prêtes : le demandeur est prévenu.".to_string())
                                }))>"Prête"</Bouton>
                        })}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}
