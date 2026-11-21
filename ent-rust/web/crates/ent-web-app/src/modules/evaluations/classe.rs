//! Vue de classe (direction, professeur principal, vie scolaire) : moyennes par matière et
//! générales, moyenne / minimum / maximum de la classe. Lecture seule, calculs du serveur.

use ent_types::evaluations::VueClasseDto;
use ent_web_ui::{EnTetePage, EtatVide, Squelette};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_params_map, use_query_map};
use uuid::Uuid;

use super::grille::libelle_arrondi;
use super::modele::{moyenne_fr, nombre_fr};
use super::{lien_bulletins, BadgePeriode, RACINE};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageVueClasse() -> impl IntoView {
    let p = use_params_map();
    let q = use_query_map();
    let classe = Memo::new(move |_| {
        p.with(|p| p.get("id"))
            .and_then(|s| Uuid::parse_str(&s).ok())
    });
    let periode = Memo::new(move |_| {
        q.with(|q| q.get("periode"))
            .and_then(|s| Uuid::parse_str(&s).ok())
    });
    view! {
        <Title text="Moyennes de la classe"/>
        {move || match (classe.get(), periode.get()) {
            (Some(c), Some(p)) => view! { <VueClasse classe=c periode=p/> }.into_any(),
            _ => view! { <EtatVide titre="Classe introuvable" message="Ouvrez la classe depuis la page Évaluations."/> }.into_any(),
        }}
    }
}

#[component]
fn VueClasse(classe: Uuid, periode: Uuid) -> impl IntoView {
    let donnees =
        LocalResource::new(
            move || async move { session::client().vue_classe(classe, periode).await },
        );
    view! {
        {move || match donnees.get() {
            None => view! { <Squelette lignes=8 libelle="Calcul des moyennes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/> }.into_any(),
            Some(Ok(v)) => view! { <TableauClasse vue=v/> }.into_any(),
        }}
    }
}

#[component]
fn TableauClasse(vue: VueClasseDto) -> impl IntoView {
    let moteur = use_droits_moteur();
    let (classe, periode) = (vue.classe_id, vue.periode.id);
    view! {
        <EnTetePage
            surtitre="Évaluations".to_string()
            titre=format!("Classe {}", vue.classe_code)
            sous_titre=format!("{} — moyennes arrondies au {}", vue.periode.libelle, libelle_arrondi(&vue.arrondi))
        />
        <p class="eva-periode">
            <BadgePeriode statut=vue.periode.statut.clone()/>
            " "<a href=RACINE>"Toutes les évaluations"</a>
            {move || moteur.get().a_etablissement("bulletins.publier").then(|| view! {
                " · "<a href=lien_bulletins(classe, periode)>"Bulletins et livrets"</a>
            })}
        </p>
        <div class="eva-cadre">
            <table class="ui-tableau eva-grille">
                <caption class="ui-sr">{format!("Moyennes de la classe {}", vue.classe_code)}</caption>
                <thead>
                    <tr>
                        <th scope="col">"Élève"</th>
                        {vue.matieres.iter().map(|m| view! {
                            <th scope="col">{m.libelle.clone()}<span class="eva-col__info">{format!("coef {}", nombre_fr(m.coef))}</span></th>
                        }).collect_view()}
                        <th scope="col">"Générale"</th>
                    </tr>
                </thead>
                <tbody>
                    {vue.lignes.iter().map(|l| view! {
                        <tr>
                            <th scope="row">{format!("{} {}", l.nom, l.prenom)}</th>
                            {l.moyennes.iter().map(|m| view! { <td>{moyenne_fr(*m)}</td> }).collect_view()}
                            <td class="eva-moyenne">{moyenne_fr(l.moyenne_generale)}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
                <tfoot>
                    {[("Moyenne de la classe", 0usize), ("Minimum", 1), ("Maximum", 2)].into_iter().map(|(libelle, k)| {
                        let pick = move |s: &ent_types::evaluations::StatsDto| match k {
                            0 => s.moyenne,
                            1 => s.min,
                            _ => s.max,
                        };
                        view! {
                            <tr>
                                <th scope="row">{libelle}</th>
                                {vue.matieres.iter().map(|m| view! { <td class="eva-stats">{moyenne_fr(pick(&m.stats))}</td> }).collect_view()}
                                <td class="eva-stats">{moyenne_fr(pick(&vue.stats_generale))}</td>
                            </tr>
                        }
                    }).collect_view()}
                </tfoot>
            </table>
        </div>
    }
}
