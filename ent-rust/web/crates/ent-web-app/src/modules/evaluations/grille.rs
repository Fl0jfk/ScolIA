//! Grille de saisie du professeur (EVA-01, EVA-02, EVA-03, EVA-05) : une ligne par élève,
//! une colonne par évaluation, la moyenne calculée par le serveur. Une cellule accepte une
//! note (« 12,5 »), « abs », « disp », « nn » ou rien. « Enregistrer » n'envoie que les
//! cellules modifiées, devoir par devoir, puis les appréciations modifiées.

use ent_types::evaluations::{
    AppreciationBody, CreerDevoirGrilleBody, GrilleDto, SaisieGrilleBody,
};
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Niveau, Squelette, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::modele::{changements, lire_devoir, moyenne_fr, nombre_fr, texte_resultat, Cellule};
use super::{lien_classe, BadgePeriode, RACINE};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageGrille() -> impl IntoView {
    let q = use_query_map();
    let lire = move |cle: &'static str| {
        Memo::new(move |_| {
            q.with(|q| q.get(cle))
                .and_then(|s| Uuid::parse_str(&s).ok())
        })
    };
    let (service, periode) = (lire("service"), lire("periode"));
    view! {
        <Title text="Grille de notes"/>
        {move || match (service.get(), periode.get()) {
            (Some(s), Some(p)) => view! { <Grille service=s periode=p/> }.into_any(),
            _ => view! {
                <EtatVide titre="Grille introuvable" message="Ouvrez une grille depuis la page Évaluations."/>
            }
            .into_any(),
        }}
    }
}

#[component]
fn Grille(service: Uuid, periode: Uuid) -> impl IntoView {
    let donnees = LocalResource::new(move || async move {
        session::client().grille_notes(service, periode).await
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| donnees.refetch()));
    view! {
        {move || match donnees.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la grille…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/>
            }
            .into_any(),
            Some(Ok(g)) => view! { <TableauGrille grille=g ecriture=ecriture/> }.into_any(),
        }}
    }
}

/// Une ligne éditable : un signal par cellule (seule la cellule modifiée se redessine).
#[derive(Clone)]
struct LigneEdit {
    eleve_id: Uuid,
    nom: String,
    moyenne: Option<f64>,
    cellules: Vec<(String, RwSignal<String>)>,
    appreciation_initiale: String,
    appreciation: RwSignal<String>,
}

#[component]
fn TableauGrille(grille: GrilleDto, ecriture: Ecriture) -> impl IntoView {
    let devoirs = StoredValue::new(grille.devoirs.clone());
    let lignes: Vec<LigneEdit> = grille
        .lignes
        .iter()
        .map(|l| LigneEdit {
            eleve_id: l.eleve_id,
            nom: format!("{} {}", l.nom, l.prenom),
            moyenne: l.moyenne,
            cellules: grille
                .devoirs
                .iter()
                .map(|d| {
                    let initial = l
                        .resultats
                        .iter()
                        .find(|r| r.devoir_id == d.id)
                        .map(|r| texte_resultat(&r.statut, r.valeur))
                        .unwrap_or_default();
                    (initial.clone(), RwSignal::new(initial))
                })
                .collect(),
            appreciation_initiale: l.appreciation.clone().unwrap_or_default(),
            appreciation: RwSignal::new(l.appreciation.clone().unwrap_or_default()),
        })
        .collect();
    let lignes = StoredValue::new(lignes);
    let erreurs_cellules: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let saisie_ouverte = grille.saisie_ouverte;
    let appreciations_ouvertes = grille.appreciations_ouvertes;
    let (matiere_id, periode_id) = (grille.service.matiere_id, grille.periode.id);

    let enregistrer = Callback::new(move |_| {
        // Contrôle local de toutes les cellules avant tout envoi.
        let mut lots = Vec::new();
        let mut erreurs = Vec::new();
        devoirs.with_value(|ds| {
            lignes.with_value(|ls| {
                for (i, d) in ds.iter().enumerate() {
                    let cellules: Vec<Cellule> = ls
                        .iter()
                        .map(|l| {
                            let (ini, sig) = &l.cellules[i];
                            (l.eleve_id, ini.clone(), sig.get_untracked())
                        })
                        .collect();
                    match changements(&cellules, d.sur) {
                        Ok(c) if !c.is_empty() => lots.push((d.id, c)),
                        Ok(_) => {}
                        Err(e) => {
                            for (eleve, m) in e {
                                let nom = ls
                                    .iter()
                                    .find(|l| l.eleve_id == eleve)
                                    .map(|l| l.nom.clone())
                                    .unwrap_or_default();
                                erreurs.push(format!("{} — {nom} : {m}", d.titre));
                            }
                        }
                    }
                }
            })
        });
        let apprs: Vec<AppreciationBody> = lignes.with_value(|ls| {
            ls.iter()
                .filter(|l| l.appreciation.get_untracked().trim() != l.appreciation_initiale.trim())
                .filter(|l| !l.appreciation.get_untracked().trim().is_empty())
                .map(|l| AppreciationBody {
                    eleve_id: l.eleve_id,
                    periode_id,
                    matiere_id: Some(matiere_id),
                    texte: l.appreciation.get_untracked(),
                })
                .collect()
        });
        erreurs_cellules.set(erreurs.clone());
        if !erreurs.is_empty() {
            return;
        }
        if lots.is_empty() && apprs.is_empty() {
            ecriture
                .annonce
                .set("Aucune modification à enregistrer.".into());
            return;
        }
        ecriture.lancer(async move {
            let client = session::client();
            let (mut ok, mut refus) = (0u32, 0usize);
            // Un devoir à la fois ; une erreur (401, 403, verrou) arrête la suite.
            for (devoir, resultats) in lots {
                let r = client
                    .saisir_resultats(devoir, &SaisieGrilleBody { resultats })
                    .await?;
                ok += r.enregistres;
                refus += r.refuses.len();
            }
            for a in &apprs {
                client.saisir_appreciation(a).await?;
            }
            let mut msg = format!(
                "{ok} résultat(s) et {} appréciation(s) enregistrés.",
                apprs.len()
            );
            if refus > 0 {
                msg.push_str(&format!(" {refus} saisie(s) refusée(s) par le serveur."));
            }
            Ok(msg)
        });
    });

    let publier = move |devoir: Uuid| {
        Callback::new(move |_| {
            ecriture.lancer(async move {
                let d = session::client().publier_devoir(devoir).await?;
                Ok(format!("« {} » publié aux familles.", d.titre))
            })
        })
    };

    let titre = format!(
        "{} · {}",
        grille.service.cible_code, grille.service.matiere_libelle
    );
    let stats = grille.stats.clone();
    let classe = grille.service.classe_id;
    view! {
        <EnTetePage
            surtitre="Évaluations".to_string()
            titre=titre
            sous_titre=format!("{} — moyennes arrondies au {}", grille.periode.libelle, libelle_arrondi(&grille.arrondi))
        />
        <p class="eva-periode">
            <BadgePeriode statut=grille.periode.statut.clone()/>
            " "<a href=RACINE>"Toutes les évaluations"</a>
            {classe.map(|c| view! { " · "<a href=lien_classe(c, periode_id)>"Moyennes de la classe"</a> })}
        </p>
        <RetourEcriture ecriture=ecriture/>
        {move || {
            let e = erreurs_cellules.get();
            (!e.is_empty()).then(|| view! {
                <Alerte niveau=Niveau::Erreur titre="Saisies à corriger avant l'enregistrement">
                    <ul>{e.into_iter().map(|m| view! { <li>{m}</li> }).collect_view()}</ul>
                </Alerte>
            })
        }}
        {(!saisie_ouverte).then(|| view! {
            <Alerte niveau=Niveau::Info titre="Saisie des notes verrouillée pour cette période."/>
        })}
        <p class="texte-attenue">"Saisir une note (12,5), « abs » (absent), « disp » (dispensé), « nn » (non noté) ; vider la cellule efface la saisie. Absent, dispensé et non noté ne comptent pas dans la moyenne."</p>
        <div class="eva-cadre">
            <table class="ui-tableau eva-grille">
                <caption class="ui-sr">{format!("Grille de notes : {} élève(s)", grille.lignes.len())}</caption>
                <thead>
                    <tr>
                        <th scope="col">"Élève"</th>
                        {grille.devoirs.iter().map(|d| {
                            let id = d.id;
                            let publie = d.publie_le.is_some();
                            view! {
                                <th scope="col" class="eva-col">
                                    <span class="eva-col__titre">{d.titre.clone()}</span>
                                    <span class="eva-col__info">{format!("{} · coef {} · /{}", d.date_devoir.format("%d/%m"), nombre_fr(d.coef), nombre_fr(d.sur))}</span>
                                    {if publie {
                                        view! { <Badge libelle="Publié" ton=TonEtat::Ok/> }.into_any()
                                    } else {
                                        view! {
                                            <Bouton variante=Variante::Discret taille=ent_web_ui::Taille::Compacte en_cours=ecriture.en_cours on_click=publier(id)>
                                                "Publier"
                                            </Bouton>
                                        }.into_any()
                                    }}
                                </th>
                            }
                        }).collect_view()}
                        <th scope="col">"Moyenne"</th>
                        <th scope="col">"Appréciation"</th>
                    </tr>
                </thead>
                <tbody>
                    {lignes.get_value().into_iter().map(|l| {
                        let nom = l.nom.clone();
                        let titres: Vec<String> = devoirs.with_value(|ds| ds.iter().map(|d| d.titre.clone()).collect());
                        view! {
                            <tr>
                                <th scope="row">{nom.clone()}</th>
                                {l.cellules.iter().zip(titres).map(|((_, sig), titre)| {
                                    let sig = *sig;
                                    view! {
                                        <td>
                                            <input
                                                class="eva-cellule"
                                                type="text"
                                                inputmode="decimal"
                                                autocomplete="off"
                                                aria-label=format!("{titre} — {nom}")
                                                disabled=!saisie_ouverte
                                                prop:value=move || sig.get()
                                                on:input=move |ev| sig.set(event_target_value(&ev))
                                            />
                                        </td>
                                    }
                                }).collect_view()}
                                <td class="eva-moyenne">{moyenne_fr(l.moyenne)}</td>
                                <td>
                                    <input
                                        class="eva-appreciation"
                                        type="text"
                                        maxlength="1500"
                                        aria-label=format!("Appréciation — {}", l.nom)
                                        disabled=!appreciations_ouvertes
                                        prop:value=move || l.appreciation.get()
                                        on:input=move |ev| l.appreciation.set(event_target_value(&ev))
                                    />
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
                <tfoot>
                    <tr>
                        <th scope="row">"Moyenne / min / max"</th>
                        {grille.devoirs.iter().map(|d| view! {
                            <td class="eva-stats">{format!("{} · {} · {}", moyenne_fr(d.stats.moyenne), moyenne_fr(d.stats.min), moyenne_fr(d.stats.max))}</td>
                        }).collect_view()}
                        <td class="eva-stats">{format!("{} · {} · {}", moyenne_fr(stats.moyenne), moyenne_fr(stats.min), moyenne_fr(stats.max))}</td>
                        <td></td>
                    </tr>
                </tfoot>
            </table>
        </div>
        {(saisie_ouverte || appreciations_ouvertes).then(|| view! {
            <div class="eva-actions">
                <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=enregistrer>
                    "Enregistrer les modifications"
                </Bouton>
            </div>
        })}
        {saisie_ouverte.then(|| view! {
            <NouvelleEvaluation service=grille.service.service_id periode=periode_id ecriture=ecriture/>
        })}
    }
}

pub fn libelle_arrondi(a: &str) -> &'static str {
    match a {
        "dixieme" => "dixième",
        "demi_point" => "demi-point",
        _ => "centième",
    }
}

#[component]
fn NouvelleEvaluation(service: Uuid, periode: Uuid, ecriture: Ecriture) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let date = RwSignal::new(String::new());
    let coef = RwSignal::new("1".to_string());
    let sur = RwSignal::new("20".to_string());
    let erreur: RwSignal<Option<String>> = RwSignal::new(None);
    // Date du jour posée après l'hydratation.
    Effect::new(move |_| {
        if date.get_untracked().is_empty() {
            date.set(aujourdhui().format("%Y-%m-%d").to_string());
        }
    });
    let creer = Callback::new(move |_| {
        let lu = lire_devoir(
            &titre.get_untracked(),
            &coef.get_untracked(),
            &sur.get_untracked(),
        );
        let jour = chrono::NaiveDate::parse_from_str(&date.get_untracked(), "%Y-%m-%d");
        let ((t, c, s), Ok(j)) = (
            match lu {
                Ok(v) => v,
                Err(m) => {
                    erreur.set(Some(m));
                    return;
                }
            },
            jour,
        ) else {
            erreur.set(Some("Date invalide.".into()));
            return;
        };
        erreur.set(None);
        let body = CreerDevoirGrilleBody {
            service_id: service,
            periode_id: periode,
            titre: t,
            date_devoir: j,
            coef: c,
            sur: s,
        };
        ecriture.lancer(async move {
            let d = session::client().creer_devoir_grille(&body).await?;
            titre.set(String::new());
            Ok(format!("Évaluation « {} » créée.", d.titre))
        });
    });
    view! {
        <details class="eva-details">
            <summary>"Nouvelle évaluation"</summary>
            <div class="eva-formulaire">
                <ChampTexte id="dev-titre" libelle="Titre" valeur=titre requis=true erreur=Signal::derive(move || erreur.get())/>
                <ChampTexte id="dev-date" libelle="Date" valeur=date genre="date" requis=true/>
                <ChampTexte id="dev-coef" libelle="Coefficient" valeur=coef inputmode="decimal" requis=true/>
                <ChampTexte id="dev-sur" libelle="Barème" valeur=sur inputmode="decimal" requis=true aide="Note maximale (20, 10…)"/>
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=creer>"Créer l'évaluation"</Bouton>
            </div>
        </details>
    }
}
