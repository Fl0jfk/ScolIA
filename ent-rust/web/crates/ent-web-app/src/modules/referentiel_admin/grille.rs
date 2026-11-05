//! Grille horaire (créneaux M1…S4) et génération des séances datées.

use chrono::{Datelike, Duration, NaiveDate};
use ent_types::referentiel_admin::{AnneeStatutDto, RapportSeancesDto, SeanceDto};
use ent_web_ui::{
    Badge, Bouton, ChampTexte, Confirmation, EtatVide, ResumeErreurs, Selection, Squelette, Taille,
    TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use super::modele::{
    annee_courante, composer_grille, composer_plage_generation, lignes_depuis_grille,
    resume_seances, trier_annees, LigneCreneau, DEMI_JOURNEES,
};
use super::{peut_gerer, Ecriture, EnTeteReferentiel, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_complete, heure_courte};

#[component]
pub fn PageGrille() -> impl IntoView {
    view! {
        <Title text="Grille horaire et séances"/>
        <EnTeteReferentiel
            titre="Grille horaire et séances"
            sous_titre="Créneaux de la journée type, puis génération des séances datées de l'année."
            courant="/web/referentiel/grille"
        />
        <SectionGrille/>
        <SectionSeances/>
    }
}

#[derive(Clone, Copy)]
struct CreneauEdit {
    cle: u32,
    code: RwSignal<String>,
    libelle: RwSignal<String>,
    demi: RwSignal<String>,
    debut: RwSignal<String>,
    fin: RwSignal<String>,
}

impl CreneauEdit {
    fn depuis(cle: u32, l: &LigneCreneau) -> Self {
        Self {
            cle,
            code: RwSignal::new(l.code.clone()),
            libelle: RwSignal::new(l.libelle.clone()),
            demi: RwSignal::new(l.demi_journee.clone()),
            debut: RwSignal::new(l.debut.clone()),
            fin: RwSignal::new(l.fin.clone()),
        }
    }

    fn lire(&self) -> LigneCreneau {
        LigneCreneau {
            code: self.code.get_untracked(),
            libelle: self.libelle.get_untracked(),
            demi_journee: self.demi.get_untracked(),
            debut: self.debut.get_untracked(),
            fin: self.fin.get_untracked(),
        }
    }
}

#[component]
fn SectionGrille() -> impl IntoView {
    let gerer = peut_gerer();
    let grille = LocalResource::new(|| async { session::client().grille(None).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| grille.refetch()));
    let lignes = RwSignal::new(Vec::<CreneauEdit>::new());
    let cle = RwSignal::new(0u32);
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let confirmer = RwSignal::new(false);
    Effect::new(move |_| {
        if let Some(Ok(g)) = grille.get() {
            let mut c = cle.get_untracked();
            let v = lignes_depuis_grille(&g)
                .iter()
                .map(|l| {
                    c += 1;
                    CreneauEdit::depuis(c, l)
                })
                .collect();
            cle.set(c);
            lignes.set(v);
            erreurs.set(Vec::new());
        }
    });
    let ajouter = Callback::new(move |_| {
        let c = cle.get_untracked() + 1;
        cle.set(c);
        lignes.update(|v| {
            v.push(CreneauEdit::depuis(
                c,
                &LigneCreneau {
                    demi_journee: "matin".into(),
                    ..Default::default()
                },
            ))
        });
    });
    let amorcer = Callback::new(move |_| {
        ecriture.lancer(async move {
            let g = session::client().amorcer_grille().await?;
            Ok(format!("Grille type chargée : {} créneau(x).", g.len()))
        });
    });
    let enregistrer = Callback::new(move |_| {
        let saisies: Vec<LigneCreneau> = lignes
            .get_untracked()
            .iter()
            .map(CreneauEdit::lire)
            .collect();
        match composer_grille(&saisies) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    let g = session::client().definir_grille(None, &corps).await?;
                    Ok(format!("Grille enregistrée : {} créneau(x).", g.len()))
                });
            }
        }
    });
    view! {
        <section class="ra-section" aria-labelledby="ra-grille">
            <h2 id="ra-grille" class="vs-section-titre">"Grille horaire"</h2>
            <RetourEcriture ecriture=ecriture/>
            <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
            {move || match grille.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement de la grille…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| grille.refetch())/>
                }
                .into_any(),
                Some(Ok(_)) => view! {
                    {move || lignes.with(Vec::is_empty).then(|| view! {
                        <EtatVide titre="Grille vide" message="Chargez la grille type (M1…M4, S1…S4) ou ajoutez vos créneaux."/>
                    })}
                    <ul class="ra-liste">
                        <For each=move || lignes.get() key=|l| l.cle let:l>
                            <LigneCreneauEdit ligne=l lignes=lignes gerer=gerer/>
                        </For>
                    </ul>
                }
                .into_any(),
            }}
            {move || gerer.get().then(|| view! {
                <div class="ra-actions">
                    <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=amorcer>
                        "Charger la grille type"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire on_click=ajouter>"Ajouter un créneau"</Bouton>
                    <Bouton en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>
                        "Enregistrer la grille"
                    </Bouton>
                </div>
            })}
            <Confirmation
                ouvert=confirmer
                id="conf-grille"
                titre="Remplacer la grille horaire ?"
                message="Les créneaux saisis remplacent la grille de l'établissement. Les séances déjà générées ne sont pas modifiées : relancez la génération ensuite."
                libelle_confirmer="Enregistrer"
                on_confirmer=enregistrer
            />
        </section>
    }
}

#[component]
fn LigneCreneauEdit(
    ligne: CreneauEdit,
    lignes: RwSignal<Vec<CreneauEdit>>,
    gerer: Signal<bool>,
) -> impl IntoView {
    let n = ligne.cle;
    let options: Vec<(String, String)> = DEMI_JOURNEES
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect();
    let off = Signal::derive(move || !gerer.get());
    view! {
        <li class="ra-ligne ra-ligne--periode">
            <ChampTexte id=format!("creneau-{n}-code") libelle="Code" valeur=ligne.code desactive=off/>
            <ChampTexte id=format!("creneau-{n}-libelle") libelle="Libellé" valeur=ligne.libelle desactive=off/>
            <Selection id=format!("creneau-{n}-demi") libelle="Demi-journée" valeur=ligne.demi options=options desactive=off/>
            <ChampTexte id=format!("creneau-{n}-debut") libelle="Début (HH:MM)" valeur=ligne.debut genre="time" desactive=off/>
            <ChampTexte id=format!("creneau-{n}-fin") libelle="Fin (HH:MM)" valeur=ligne.fin genre="time" desactive=off/>
            {move || {
                gerer.get().then(|| view! {
                    <Bouton
                        variante=Variante::Discret
                        taille=Taille::Compacte
                        libelle_accessible="Retirer ce créneau"
                        on_click=Callback::new(move |_| lignes.update(|v| v.retain(|l| l.cle != n)))
                    >
                        "Retirer"
                    </Bouton>
                })
            }}
        </li>
    }
}

#[component]
fn SectionSeances() -> impl IntoView {
    let annees =
        LocalResource::new(|| async { session::client().annees_statut().await.map(trier_annees) });
    view! {
        <section class="ra-section" aria-labelledby="ra-seances">
            <h2 id="ra-seances" class="vs-section-titre">"Séances datées"</h2>
            {move || match annees.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des années…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| annees.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) => match annee_courante(&l).or_else(|| l.first()).cloned() {
                    Some(a) => view! { <GenerationSeances annee=a/> }.into_any(),
                    None => view! { <EtatVide titre="Aucune année" message="Créez d'abord une année scolaire."/> }.into_any(),
                },
            }}
        </section>
    }
}

/// Lundi de la semaine de `jour` (point de départ de la liste des séances).
pub fn lundi_de(jour: NaiveDate) -> NaiveDate {
    jour - Duration::days(i64::from(jour.weekday().num_days_from_monday()))
}

#[component]
fn GenerationSeances(annee: AnneeStatutDto) -> impl IntoView {
    let gerer = peut_gerer();
    let id = annee.id;
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let rapport = RwSignal::new(None::<RapportSeancesDto>);
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let plage = RwSignal::new(None::<(Option<NaiveDate>, Option<NaiveDate>)>);
    let preparer = Callback::new(move |_| {
        match composer_plage_generation(&du.get_untracked(), &au.get_untracked()) {
            Err(e) => erreurs.set(e),
            Ok(p) => {
                erreurs.set(Vec::new());
                plage.set(Some(p));
                confirmer.set(true);
            }
        }
    });
    let generer = Callback::new(move |_| {
        let Some((d, a)) = plage.get_untracked() else {
            return;
        };
        ecriture.lancer(async move {
            let r = session::client().generer_seances(id, d, a).await?;
            let texte = resume_seances(&r);
            rapport.set(Some(r));
            Ok(texte)
        });
    });
    let err = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
        })
    };
    let portee = format!(
        "Année {} : du {} au {}.",
        annee.libelle,
        date_complete(annee.date_debut),
        date_complete(annee.date_fin)
    );
    view! {
        <p class="texte-attenue">{portee}</p>
        <RetourEcriture ecriture=ecriture/>
        {move || gerer.get().then(|| view! {
            <form class="ra-formulaire" on:submit=move |ev| { ev.prevent_default(); preparer.run(()); } novalidate aria-labelledby="ra-generer">
                <h3 id="ra-generer" class="jvs-sous-titre">"Générer ou régénérer les séances"</h3>
                <p class="texte-attenue">
                    "Idempotent : les séances saisies à la main ne sont jamais écrasées ; vacances et jours fermés sont exclus. \
                     Sans dates, toute l'année est générée."
                </p>
                <ChampTexte id="gen-du" libelle="Du (facultatif)" valeur=du genre="date" erreur=err("gen-du")/>
                <ChampTexte id="gen-au" libelle="Au (facultatif)" valeur=au genre="date" erreur=err("gen-au")/>
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Générer les séances"</Bouton>
            </form>
        })}
        {move || rapport.get().map(|r| view! {
            <div class="ra-apercu" role="region" aria-label="Résultat de la génération">
                <h3 class="jvs-sous-titre">"Résultat de la génération"</h3>
                <p>{resume_seances(&r)}</p>
            </div>
        })}
        <Confirmation
            ouvert=confirmer
            id="conf-generer"
            titre="Générer les séances ?"
            message="Les séances de la plage choisie sont créées ou mises à jour d'après les créneaux, la grille et le calendrier. Les séances saisies à la main restent intactes."
            libelle_confirmer="Générer"
            on_confirmer=generer
        />
        <ListeSeances/>
    }
}

#[component]
fn ListeSeances() -> impl IntoView {
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let plage = RwSignal::new(None::<(NaiveDate, NaiveDate)>);
    // La date du jour est posée après l'hydratation (aucune date dans le HTML rendu côté serveur).
    Effect::new(move |_| {
        if du.get_untracked().is_empty() {
            let lundi = lundi_de(aujourdhui());
            du.set(lundi.to_string());
            au.set((lundi + Duration::days(6)).to_string());
            plage.set(Some((lundi, lundi + Duration::days(6))));
        }
    });
    let seances = LocalResource::new(move || {
        let p = plage.get();
        async move {
            match p {
                Some((d, a)) => session::client().seances_datees(d, a, None).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let afficher = Callback::new(move |_| {
        if let (Some(d), Some(a)) = (
            super::modele::lire_date(&du.get_untracked()),
            super::modele::lire_date(&au.get_untracked()),
        ) {
            if d <= a {
                plage.set(Some((d, a)));
            }
        }
    });
    view! {
        <h3 class="jvs-sous-titre">"Séances générées"</h3>
        <div class="jvs-barre">
            <ChampTexte id="seances-du" libelle="Du" valeur=du genre="date"/>
            <ChampTexte id="seances-au" libelle="Au" valeur=au genre="date"/>
            <Bouton variante=Variante::Secondaire on_click=afficher>"Afficher"</Bouton>
        </div>
        {move || match seances.get() {
            None | Some(Ok(None)) => view! { <Squelette lignes=3 libelle="Chargement des séances…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| seances.refetch())/>
            }
            .into_any(),
            Some(Ok(Some(l))) if l.is_empty() => view! {
                <EtatVide titre="Aucune séance" message="Aucune séance générée sur cette période."/>
            }
            .into_any(),
            Some(Ok(Some(l))) => view! { <TableauSeances seances=l/> }.into_any(),
        }}
    }
}

#[component]
fn TableauSeances(seances: Vec<SeanceDto>) -> impl IntoView {
    view! {
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Séances de la période"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Jour"</th>
                        <th scope="col">"Horaire"</th>
                        <th scope="col">"Classe"</th>
                        <th scope="col">"Matière"</th>
                        <th scope="col">"Salle"</th>
                        <th scope="col">"Statut"</th>
                    </tr>
                </thead>
                <tbody>
                    {seances
                        .into_iter()
                        .map(|s| {
                            let h = format!(
                                "{}–{}",
                                heure_courte(&s.heure_debut.to_string()),
                                heure_courte(&s.heure_fin.to_string())
                            );
                            let ton = if s.statut == "prevue" { TonEtat::Neutre } else { TonEtat::Avertissement };
                            view! {
                                <tr>
                                    <th scope="row">{date_complete(s.jour)}</th>
                                    <td>{h}{s.grille_code.map(|c| format!(" ({c})"))}</td>
                                    <td>{s.classe}</td>
                                    <td>{s.matiere_libelle}</td>
                                    <td>{s.salle.unwrap_or_else(|| "—".into())}</td>
                                    <td><Badge libelle=s.statut.clone() ton=ton/></td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}
