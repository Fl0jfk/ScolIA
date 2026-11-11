//! Budget d'une sortie (socle 42, lot V0b) : section de la fiche. Versions numérotées en centimes,
//! part des familles calculée **par le serveur**, prix annoncé figé, PDF rendus côté serveur.
//!
//! L'écran n'ajoute aucune règle : il affiche ce que le serveur renvoie (équilibre, causes d'un
//! déficit, droits calculés `peut_ecrire`, `peut_valider`, `peut_decider_hausse`) et relaie ses
//! refus (403/409/422) tels quels. Le montant est envoyé **comme texte** (« 1 350,50 ») : le serveur
//! le contrôle strictement, « 1.350 » est refusé. Lecture réservée au porteur, à la comptabilité, à
//! la direction et au secrétariat : pour les autres profils, la section n'apparaît pas.

mod modele;

use ent_types::voyages::{
    BudgetDto, BudgetVersionDto, DocumentDto, ParamsBudgetDto, ParamsCommandeDto,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, Niveau, Selection, Squelette, Variante, ZoneTexte,
};
use leptos::prelude::*;
use uuid::Uuid;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::telechargement::enregistrer_telechargement;
use crate::noyau::temps::date_heure_paris;
use modele::{
    euros, libelle_equilibre, libelle_etat_version, libelle_poste, libelle_source,
    libelle_validation, lignes_du_sens, resume_prix, saisie_depuis_centimes, ton_equilibre,
    version_a_valider, SaisieLigne, AFFECTATIONS, POSTES_DEPENSE, POSTES_RECETTE,
};

/// Section « Budget » de la fiche. `etat` et `version` sont ceux de la sortie (la validation du
/// budget est une commande d'état qui porte la version lue du dossier et celle du budget).
#[component]
pub fn Budget(id: Uuid, etat: String, version: i32, recharger: Callback<()>) -> impl IntoView {
    let budget =
        LocalResource::new(move || async move { session::client().voyage_budget(id).await });
    let documents =
        LocalResource::new(move || async move { session::client().voyage_documents(id).await });
    let local = Callback::new(move |_| {
        budget.refetch();
        documents.refetch();
    });
    view! {
        {move || match budget.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement du budget…"/> }.into_any(),
            // Pas de droit sur le budget (profil, ou sortie d'un autre établissement) : rien à afficher.
            Some(Err(ApiError::Interdit(_) | ApiError::Introuvable(_))) => ().into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| budget.refetch())/> }.into_any(),
            Some(Ok(b)) => {
                let etat = etat.clone();
                view! { <VueBudget id=id b=b etat=etat version=version documents=documents local=local recharger=recharger/> }.into_any()
            }
        }}
    }
}

#[component]
fn VueBudget(
    id: Uuid,
    b: BudgetDto,
    etat: String,
    version: i32,
    documents: LocalResource<Result<Vec<DocumentDto>, ApiError>>,
    local: Callback<()>,
    recharger: Callback<()>,
) -> impl IntoView {
    if !b.a_budget {
        return view! {
            <section aria-labelledby="voy-budget">
                <h2 id="voy-budget" class="vs-section-titre">"Budget"</h2>
                <p class="texte-attenue">"Cette sortie n'a pas de budget à valider (ni voyage ni car)."</p>
            </section>
        }
        .into_any();
    }
    let ecriture = Ecriture::nouveau(local);
    // La validation du budget change l'étape de la sortie : on recharge toute la fiche.
    let ecriture_etat = Ecriture::nouveau(recharger);
    let numero_valide = version_a_valider(b.brouillon.as_ref(), b.validee.as_ref());
    let bouton_validation = (b.peut_valider)
        .then(|| libelle_validation(&etat, b.brouillon.is_some()))
        .flatten();
    let est_revision = etat != "budget_a_valider";
    let valider = Callback::new(move |_| {
        let Some((numero, rev)) = numero_valide else {
            return;
        };
        if est_revision {
            ecriture.lancer(async move {
                session::client()
                    .voyage_budget_valider_revision(id, numero, rev)
                    .await?;
                Ok(format!("Version {numero} du budget validée."))
            });
        } else {
            ecriture_etat.lancer(async move {
                let params = ParamsCommandeDto {
                    version: Some(version),
                    budget_numero: Some(numero),
                    budget_rev: Some(rev),
                    ..Default::default()
                };
                session::client()
                    .voyage_commande(id, "valider-budget", &params)
                    .await?;
                Ok("Budget validé : le dossier passe à l'autorisation.".to_string())
            });
        }
    });
    view! {
        <section aria-labelledby="voy-budget">
            <h2 id="voy-budget" class="vs-section-titre">"Budget"</h2>
            <RetourEcriture ecriture=ecriture/>
            <RetourEcriture ecriture=ecriture_etat/>
            <PrixAnnonce id=id b=b.clone() ecriture=ecriture/>
            {b.brouillon.clone().map(|v| view! {
                <Version id=id v=v b=b.clone() ecriture=ecriture/>
            })}
            {b.validee.clone().map(|v| view! {
                <Version id=id v=v b=b.clone() ecriture=ecriture/>
            })}
            {(b.brouillon.is_none() && b.validee.is_none()).then(|| view! {
                <p class="texte-attenue">"Aucune version de budget pour l'instant."</p>
            })}
            <div class="voy-actions">
                {bouton_validation.map(|l| view! {
                    <Bouton en_cours=ecriture.en_cours on_click=valider>{l}</Bouton>
                })}
                <NouvelleVersion id=id b=b.clone() ecriture=ecriture/>
            </div>
            <Documents id=id b=b.clone() documents=documents ecriture=ecriture/>
        </section>
    }
    .into_any()
}

/// Prix annoncé aux familles : montant courant, historique, hausse en attente de décision.
#[component]
fn PrixAnnonce(id: Uuid, b: BudgetDto, ecriture: Ecriture) -> impl IntoView {
    let Some(courant) = b.prix_courant_centimes else {
        return ().into_any();
    };
    let motif = RwSignal::new(String::new());
    let decider = b.peut_decider_hausse && b.hausse_en_attente;
    let hausse = b.hausse_en_attente;
    let cible = b.validee.as_ref().and_then(|v| v.part_familles_centimes);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let m = motif.get_untracked();
        if m.trim().is_empty() {
            ecriture
                .erreur
                .set(Some("Une décision de hausse est motivée.".into()));
            return;
        }
        ecriture.lancer(async move {
            session::client().voyage_prix_hausser(id, m.trim()).await?;
            Ok("Prix annoncé augmenté ; un nouvel avis aux familles est à préparer.".to_string())
        });
    };
    view! {
        <div class="voy-etape">
            <h3>"Prix annoncé aux familles"</h3>
            <p><strong>{euros(courant)}</strong>" par élève (montant fixe)."</p>
            <ul class="voy-liste voy-liste--simple">
                {b.prix_fige.iter().map(|p| view! {
                    <li>{resume_prix(p, &date_heure_paris(p.fige_le))}
                        {p.motif.clone().map(|m| format!(" (motif : {m})"))}</li>
                }).collect_view()}
            </ul>
            {hausse.then(|| view! {
                <Alerte niveau=Niveau::Avertissement
                    titre=format!(
                        "La version validée ({}) dépasse le prix annoncé : seule la direction peut décider d'augmenter le prix, avec un nouvel avis aux familles.",
                        cible.map(euros).unwrap_or_default()
                    )/>
            })}
            {decider.then(|| view! {
                <form class="voy-motif" on:submit=envoyer>
                    <ZoneTexte id="voy-hausse-motif" libelle="Motif de la décision (obligatoire, conservé au journal)" valeur=motif requis=true/>
                    <Bouton soumettre=true variante=Variante::Danger en_cours=ecriture.en_cours>"Augmenter le prix annoncé"</Bouton>
                </form>
            })}
        </div>
    }
    .into_any()
}

/// Une version du budget : lignes, calcul du serveur, paramètres et saisie (si brouillon et droit).
#[component]
fn Version(id: Uuid, v: BudgetVersionDto, b: BudgetDto, ecriture: Ecriture) -> impl IntoView {
    let brouillon = v.etat == "brouillon";
    let peut = b.peut_ecrire && brouillon;
    let (numero, rev) = (v.numero, v.rev);
    let c = v.calcul.clone();
    let titre = format!("Version {} — {}", v.numero, libelle_etat_version(&v.etat));
    let abandonner = Callback::new(move |_| {
        ecriture.lancer(async move {
            session::client()
                .voyage_budget_abandonner(id, numero, rev)
                .await?;
            Ok("Version abandonnée.".to_string())
        });
    });
    let ligne_vue = move |l: ent_types::voyages::BudgetLigneDto| {
        let lid = l.id;
        let nom = l.libelle.clone();
        view! {
            <li class="voy-ligne">
                <span>
                    {format!("{} — {}", libelle_poste(&l.poste), l.libelle)}
                    {(l.affectation.as_deref() == Some("encadrement")).then_some(" (affectée à l'encadrement)")}
                    <span class="texte-attenue">{format!(" · {} : {}", libelle_source(&l.source), l.motif.clone().unwrap_or_default())}</span>
                </span>
                <strong>{euros(l.montant_centimes)}</strong>
                {peut.then(|| view! {
                    <Bouton variante=Variante::Discret libelle_accessible=format!("Supprimer la ligne {nom}") en_cours=ecriture.en_cours
                        on_click=Callback::new(move |_| ecriture.lancer(async move {
                            session::client().voyage_budget_ligne_supprimer(id, numero, lid, rev).await?;
                            Ok("Ligne supprimée.".to_string())
                        }))>"Supprimer"</Bouton>
                })}
            </li>
        }
    };
    let titre_aria = titre.clone();
    let depenses: Vec<_> = lignes_du_sens(&v, "depense").into_iter().cloned().collect();
    let recettes: Vec<_> = lignes_du_sens(&v, "recette").into_iter().cloned().collect();
    view! {
        <div class="voy-etape" aria-label=titre_aria>
            <h3>{titre}</h3>
            <p>
                <Badge libelle=libelle_equilibre(&c) ton=ton_equilibre(&c)/>
                {format!(" {} participant(s) payant(s) · marge de sécurité {} %", v.nb_payants, v.marge_pct)}
            </p>
            {v.motif_revision.clone().map(|m| view! { <p class="voy-note">{format!("Motif de la révision : {m}")}</p> })}
            <h4>"Dépenses"</h4>
            {if depenses.is_empty() { view! { <p class="texte-attenue">"Aucune dépense."</p> }.into_any() } else {
                view! { <ul class="voy-liste voy-liste--simple">{depenses.into_iter().map(ligne_vue).collect_view()}</ul> }.into_any() }}
            <p><strong>{format!("Total des dépenses : {}", euros(c.depenses_total))}</strong></p>
            <h4>"Recettes (hors participation des familles)"</h4>
            {if recettes.is_empty() { view! { <p class="texte-attenue">"Aucune recette."</p> }.into_any() } else {
                view! { <ul class="voy-liste voy-liste--simple">{recettes.into_iter().map(ligne_vue).collect_view()}</ul> }.into_any() }}
            <p><strong>{format!("Total des recettes : {}", euros(c.recettes_total))}</strong></p>
            <dl class="voy-infos">
                <dt>"Part des familles par élève"</dt>
                <dd><strong>{c.part_familles.map_or("À calculer (effectif payant ?)".to_string(), euros)}</strong>" (arrondie à l'euro inférieur)"</dd>
                <dt>"Coût réel par élève"</dt><dd>{c.cout_reel_par_eleve.map_or("—".to_string(), euros)}</dd>
                <dt>"Marge de sécurité"</dt><dd>{euros(c.marge)}</dd>
                <dt>"Participation totale des familles"</dt><dd>{euros(c.total_familles)}</dd>
                {(c.arrondi_pris_en_charge > 0).then(|| view! { <dt>"Arrondi pris en charge par l'établissement"</dt><dd>{euros(c.arrondi_pris_en_charge)}</dd> })}
                {(c.excedent > 0).then(|| view! { <dt>"Excédent prévisionnel"</dt><dd>{euros(c.excedent)}</dd> })}
            </dl>
            {(!c.causes.is_empty()).then(|| view! {
                <Alerte niveau=Niveau::Avertissement titre="Pourquoi le budget n'est pas équilibré">
                    <ul>{c.causes.iter().map(|x| view! { <li>{x.clone()}</li> }).collect_view()}</ul>
                </Alerte>
            })}
            {peut.then(|| view! {
                <Parametres id=id numero=numero rev=rev nb=v.nb_payants marge=v.marge_pct motif=v.marge_motif.clone() prereglee=b.marge_prereglee ecriture=ecriture/>
                <SaisieLigneForm id=id numero=numero rev=rev ecriture=ecriture/>
                <div class="voy-actions">
                    <Bouton variante=Variante::Discret en_cours=ecriture.en_cours on_click=abandonner>"Abandonner cette version"</Bouton>
                </div>
            })}
        </div>
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn Parametres(
    id: Uuid,
    numero: i32,
    rev: i32,
    nb: i32,
    marge: i32,
    motif: Option<String>,
    prereglee: i32,
    ecriture: Ecriture,
) -> impl IntoView {
    let nb_s = RwSignal::new(nb.to_string());
    let marge_s = RwSignal::new(marge.to_string());
    let motif_s = RwSignal::new(motif.unwrap_or_default());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (Ok(n), Ok(m)) = (
            nb_s.get_untracked().trim().parse::<i32>(),
            marge_s.get_untracked().trim().parse::<i32>(),
        ) else {
            ecriture.erreur.set(Some(
                "L'effectif payant et la marge sont des nombres entiers.".into(),
            ));
            return;
        };
        let t = motif_s.get_untracked();
        let p = ParamsBudgetDto {
            rev,
            nb_payants: Some(n),
            marge_pct: Some(m),
            marge_motif: (!t.trim().is_empty()).then(|| t.trim().to_string()),
        };
        ecriture.lancer(async move {
            session::client()
                .voyage_budget_parametrer(id, numero, &p)
                .await?;
            Ok("Paramètres du budget enregistrés.".to_string())
        });
    };
    view! {
        <form class="voy-formulaire voy-formulaire--ligne" on:submit=envoyer>
            <ChampTexte id="voy-budget-nb" libelle="Participants payants" valeur=nb_s genre="number" requis=true/>
            <ChampTexte id="voy-budget-marge" libelle="Marge de sécurité (%)" valeur=marge_s genre="number"
                aide=format!("Préréglage du séjour : {prereglee} %. Une autre valeur demande un motif.")/>
            <ChampTexte id="voy-budget-marge-motif" libelle="Motif de la marge (si différente du préréglage)" valeur=motif_s/>
            <Bouton soumettre=true variante=Variante::Secondaire en_cours=ecriture.en_cours>"Enregistrer"</Bouton>
        </form>
    }
}

#[component]
fn SaisieLigneForm(id: Uuid, numero: i32, rev: i32, ecriture: Ecriture) -> impl IntoView {
    let sens = RwSignal::new("depense".to_string());
    let poste = RwSignal::new(String::new());
    let libelle = RwSignal::new(String::new());
    let montant = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let affectation = RwSignal::new("general".to_string());
    let options_sens = vec![
        ("depense".to_string(), "Dépense".to_string()),
        ("recette".to_string(), "Recette".to_string()),
    ];
    let options_affectation: Vec<(String, String)> = AFFECTATIONS
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieLigne {
            sens: sens.get_untracked(),
            poste: poste.get_untracked(),
            libelle: libelle.get_untracked(),
            montant: montant.get_untracked(),
            motif: motif.get_untracked(),
            affectation: affectation.get_untracked(),
        };
        if let Err(m) = saisie.controle() {
            ecriture.erreur.set(Some(m));
            return;
        }
        let dto = saisie.vers_dto(rev);
        ecriture.lancer(async move {
            session::client()
                .voyage_budget_ligne_ajouter(id, numero, &dto)
                .await?;
            libelle.set(String::new());
            montant.set(String::new());
            motif.set(String::new());
            Ok("Ligne ajoutée.".to_string())
        });
    };
    view! {
        <form class="voy-formulaire voy-formulaire--ligne" on:submit=envoyer>
            <Selection id="voy-ligne-sens" libelle="Type de ligne" valeur=sens options=options_sens/>
            {move || {
                let est_recette = sens.get() == "recette";
                let catalogue: &[(&str, &str)] = if est_recette { &POSTES_RECETTE } else { &POSTES_DEPENSE };
                let mut options = vec![(String::new(), "Choisir…".to_string())];
                options.extend(catalogue.iter().map(|(c, l)| ((*c).to_string(), (*l).to_string())));
                poste.set(String::new());
                view! { <Selection id="voy-ligne-poste" libelle="Poste" valeur=poste options=options requis=true/> }
            }}
            {move || (sens.get() == "recette").then(|| view! {
                <Selection id="voy-ligne-affectation" libelle="Affectation" valeur=affectation options=options_affectation.clone()/>
            })}
            <ChampTexte id="voy-ligne-libelle" libelle="Libellé" valeur=libelle requis=true/>
            <ChampTexte id="voy-ligne-montant" libelle="Montant (€)" valeur=montant requis=true
                aide=format!("Au format français, par exemple {}. « 1.350 » est refusé.", saisie_depuis_centimes(135_050))/>
            <ChampTexte id="voy-ligne-motif" libelle="Motif ou source" valeur=motif requis=true
                aide="D'où vient ce montant : devis, estimation, décision."/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Ajouter la ligne"</Bouton>
        </form>
    }
}

/// Création d'une première version, ou d'une révision (motivée) d'un budget validé.
#[component]
fn NouvelleVersion(id: Uuid, b: BudgetDto, ecriture: Ecriture) -> impl IntoView {
    if !b.peut_ecrire || b.brouillon.is_some() {
        return ().into_any();
    }
    let revision = b.validee.is_some();
    let motif = RwSignal::new(String::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let m = motif.get_untracked();
        if revision && m.trim().is_empty() {
            ecriture.erreur.set(Some(
                "Une révision du budget est motivée : indiquez pourquoi il change.".into(),
            ));
            return;
        }
        ecriture.lancer(async move {
            let m = (!m.trim().is_empty()).then(|| m.trim().to_string());
            session::client()
                .voyage_budget_version_creer(id, m.as_deref())
                .await?;
            Ok("Nouvelle version du budget créée.".to_string())
        });
    };
    view! {
        <form class="voy-formulaire voy-formulaire--ligne" on:submit=envoyer>
            {revision.then(|| view! { <ChampTexte id="voy-budget-revision" libelle="Motif de la révision" valeur=motif requis=true/> })}
            <Bouton soumettre=true variante=Variante::Secondaire en_cours=ecriture.en_cours>
                {if revision { "Réviser le budget (version suivante)" } else { "Créer le budget" }}
            </Bouton>
        </form>
    }
    .into_any()
}

/// PDF rendus par le serveur (fiche budget pour la comptabilité et la direction, circulaire pour le
/// porteur) et téléchargement : le serveur contrôle le droit et l'empreinte à chaque lecture.
#[component]
fn Documents(
    id: Uuid,
    b: BudgetDto,
    documents: LocalResource<Result<Vec<DocumentDto>, ApiError>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let peut_budget = (b.peut_valider || b.peut_decider_hausse) && b.validee.is_some();
    let erreur = RwSignal::new(None::<String>);
    let rendre_budget = Callback::new(move |_| {
        ecriture.lancer(async move {
            session::client().voyage_budget_pdf(id).await?;
            Ok("Fiche budget rendue en PDF.".to_string())
        });
    });
    let rendre_circulaire = Callback::new(move |_| {
        ecriture.lancer(async move {
            session::client().voyage_circulaire_pdf(id).await?;
            Ok("Circulaire rendue en PDF.".to_string())
        });
    });
    let telecharger = move |d: DocumentDto| {
        let (doc, nom) = (d.id, d.nom.clone());
        leptos::task::spawn_local(async move {
            match session::client().voyage_document(id, doc).await {
                Ok(octets) => {
                    if let Err(m) = enregistrer_telechargement(&nom, "application/pdf", &octets) {
                        erreur.set(Some(m));
                    }
                }
                Err(e) => erreur.set(Some(e.message_utilisateur())),
            }
        });
    };
    view! {
        <h3>"Documents"</h3>
        <div class="voy-actions">
            {peut_budget.then(|| view! {
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=rendre_budget>"Rendre la fiche budget en PDF"</Bouton>
            })}
            <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=rendre_circulaire>"Rendre la circulaire en PDF"</Bouton>
        </div>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {move || match documents.get() {
            None => view! { <Squelette lignes=1 libelle="Chargement des documents…"/> }.into_any(),
            Some(Err(_)) => view! { <p class="texte-attenue">"Documents indisponibles."</p> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <p class="texte-attenue">"Aucun document émis."</p> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="voy-liste voy-liste--simple">
                    {l.into_iter().map(|d| {
                        let d2 = d.clone();
                        let nom = d.nom.clone();
                        view! {
                            <li class="voy-ligne">
                                <span>
                                    {format!("{} — {}", if d.type_ == "budget" { "Fiche budget" } else { "Circulaire" }, date_heure_paris(d.emis_le))}
                                    {d.actuel.then_some(" (actuelle)")}
                                </span>
                                <Bouton variante=Variante::Discret libelle_accessible=format!("Télécharger {nom}")
                                    on_click=Callback::new(move |_| telecharger(d2.clone()))>"Télécharger"</Bouton>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}
