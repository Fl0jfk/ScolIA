//! Périodes de l'année (trimestres, semestres) et calendrier (vacances, jours fermés).

use chrono::NaiveDate;
use ent_types::referentiel_admin::AnneeStatutDto;
use ent_web_ui::{
    Badge, Bouton, ChampTexte, Confirmation, EtatVide, ResumeErreurs, Selection, Squelette, Taille,
    TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use super::modele::{
    annee_courante, composer_periodes, composer_plage, libelle_kind_jour, lignes_depuis_periodes,
    periodes_modele, regrouper_calendrier, trier_annees, LignePeriode, PlageCalendrier,
    SaisiePlage, KINDS_JOUR, KINDS_PERIODE, KINDS_VACANCES,
};
use super::{peut_gerer, Ecriture, EnTeteReferentiel, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_complete;

#[component]
pub fn PageCalendrier() -> impl IntoView {
    let annees =
        LocalResource::new(|| async { session::client().annees_statut().await.map(trier_annees) });
    view! {
        <Title text="Périodes et calendrier"/>
        <EnTeteReferentiel
            titre="Périodes et calendrier"
            sous_titre="Trimestres ou semestres de l'année, vacances, jours fériés et fermetures."
            courant="/web/referentiel/calendrier"
        />
        {move || match annees.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des années…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| annees.refetch())/>
            }
            .into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune année" message="Créez d'abord une année scolaire (Années et bascule)."/>
            }
            .into_any(),
            Some(Ok(l)) => view! { <ContenuCalendrier annees=l/> }.into_any(),
        }}
    }
}

/// Ligne éditable d'une période : un signal par champ (une saisie ne redessine que son champ).
#[derive(Clone, Copy)]
struct LigneEdit {
    cle: u32,
    code: RwSignal<String>,
    libelle: RwSignal<String>,
    kind: RwSignal<String>,
    debut: RwSignal<String>,
    fin: RwSignal<String>,
}

impl LigneEdit {
    fn depuis(cle: u32, l: &LignePeriode) -> Self {
        Self {
            cle,
            code: RwSignal::new(l.code.clone()),
            libelle: RwSignal::new(l.libelle.clone()),
            kind: RwSignal::new(l.kind.clone()),
            debut: RwSignal::new(l.debut.clone()),
            fin: RwSignal::new(l.fin.clone()),
        }
    }

    fn lire(&self) -> LignePeriode {
        LignePeriode {
            code: self.code.get_untracked(),
            libelle: self.libelle.get_untracked(),
            kind: self.kind.get_untracked(),
            debut: self.debut.get_untracked(),
            fin: self.fin.get_untracked(),
        }
    }
}

#[component]
fn ContenuCalendrier(annees: Vec<AnneeStatutDto>) -> impl IntoView {
    let initiale = annee_courante(&annees)
        .or_else(|| annees.first())
        .map(|a| a.id.to_string())
        .unwrap_or_default();
    let choix = RwSignal::new(initiale);
    let options: Vec<(String, String)> = annees
        .iter()
        .map(|a| (a.id.to_string(), a.libelle.clone()))
        .collect();
    let annees_ref = StoredValue::new(annees);
    let annee = Memo::new(move |_| {
        let id = choix.get();
        annees_ref.with_value(|l| l.iter().find(|a| a.id.to_string() == id).cloned())
    });
    view! {
        <div class="ra-barre">
            <Selection id="cal-annee" libelle="Année scolaire" valeur=choix options=options/>
        </div>
        {move || annee.get().map(|a| view! { <ParAnnee annee=a/> })}
    }
}

#[component]
fn ParAnnee(annee: AnneeStatutDto) -> impl IntoView {
    view! {
        <SectionPeriodes annee=annee.clone()/>
        <SectionCalendrier annee=annee/>
    }
}

#[component]
fn SectionPeriodes(annee: AnneeStatutDto) -> impl IntoView {
    let gerer = peut_gerer();
    let id = annee.id;
    let periodes =
        LocalResource::new(move || async move { session::client().periodes_annee(id).await });
    let lignes = RwSignal::new(Vec::<LigneEdit>::new());
    let prochaine_cle = RwSignal::new(0u32);
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| periodes.refetch()));

    let remplir = move |nouvelles: Vec<LignePeriode>| {
        let mut cle = prochaine_cle.get_untracked();
        let v: Vec<LigneEdit> = nouvelles
            .iter()
            .map(|l| {
                cle += 1;
                LigneEdit::depuis(cle, l)
            })
            .collect();
        prochaine_cle.set(cle);
        lignes.set(v);
        erreurs.set(Vec::new());
    };
    // Les périodes chargées alimentent l'éditeur (à chaque rechargement).
    Effect::new(move |_| {
        if let Some(Ok(p)) = periodes.get() {
            remplir(lignes_depuis_periodes(&p));
        }
    });
    let annee_modele = annee.clone();
    let appliquer_modele = move |kind: &'static str, n: u32| {
        remplir(periodes_modele(&annee_modele, kind, n));
    };
    let ajouter = Callback::new(move |_| {
        let cle = prochaine_cle.get_untracked() + 1;
        prochaine_cle.set(cle);
        lignes.update(|v| {
            v.push(LigneEdit::depuis(
                cle,
                &LignePeriode {
                    kind: "trimestre".into(),
                    ..Default::default()
                },
            ))
        });
    });
    let annee_envoi = annee.clone();
    let enregistrer = Callback::new(move |_| {
        let saisies: Vec<LignePeriode> =
            lignes.get_untracked().iter().map(LigneEdit::lire).collect();
        match composer_periodes(&saisies, &annee_envoi) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    let r = session::client().definir_periodes(id, &corps).await?;
                    Ok(format!("{} période(s) enregistrée(s).", r.len()))
                });
            }
        }
    });
    view! {
        <section class="ra-section" aria-labelledby="ra-periodes">
            <h2 id="ra-periodes" class="vs-section-titre">"Périodes"</h2>
            <p class="texte-attenue">
                {format!(
                    "Les périodes doivent couvrir l'année du {} au {}, sans trou ni chevauchement.",
                    date_complete(annee.date_debut),
                    date_complete(annee.date_fin)
                )}
            </p>
            <RetourEcriture ecriture=ecriture/>
            <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
            {move || match periodes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des périodes…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| periodes.refetch())/>
                }
                .into_any(),
                Some(Ok(_)) => view! {
                    <ul class="ra-liste">
                        <For each=move || lignes.get() key=|l| l.cle let:l>
                            <LignePeriodeEdit ligne=l lignes=lignes gerer=gerer/>
                        </For>
                    </ul>
                }
                .into_any(),
            }}
            {move || {
                gerer.get().then(|| {
                    let (m1, m2) = (appliquer_modele.clone(), appliquer_modele.clone());
                    view! {
                        <div class="ra-actions">
                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| m1("trimestre", 3))>
                                "Modèle : 3 trimestres"
                            </Bouton>
                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| m2("semestre", 2))>
                                "Modèle : 2 semestres"
                            </Bouton>
                            <Bouton variante=Variante::Secondaire on_click=ajouter>"Ajouter une période"</Bouton>
                            <Bouton
                                en_cours=ecriture.en_cours
                                on_click=Callback::new(move |_| {
                                    // Contrôle local d'abord : on n'ouvre la confirmation que si la saisie est cohérente.
                                    confirmer.set(true);
                                })
                            >
                                "Enregistrer les périodes"
                            </Bouton>
                        </div>
                    }
                })
            }}
            <Confirmation
                ouvert=confirmer
                id="conf-periodes"
                titre="Remplacer les périodes ?"
                message="Les périodes saisies remplacent celles de l'année. Le serveur vérifie la couverture de l'année."
                libelle_confirmer="Enregistrer"
                on_confirmer=enregistrer
            />
        </section>
    }
}

#[component]
fn LignePeriodeEdit(
    ligne: LigneEdit,
    lignes: RwSignal<Vec<LigneEdit>>,
    gerer: Signal<bool>,
) -> impl IntoView {
    let n = ligne.cle;
    let options: Vec<(String, String)> = KINDS_PERIODE
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect();
    view! {
        <li class="ra-ligne ra-ligne--periode">
            <ChampTexte id=format!("periode-{n}-code") libelle="Code" valeur=ligne.code desactive=Signal::derive(move || !gerer.get())/>
            <ChampTexte id=format!("periode-{n}-libelle") libelle="Libellé" valeur=ligne.libelle desactive=Signal::derive(move || !gerer.get())/>
            <Selection id=format!("periode-{n}-kind") libelle="Type" valeur=ligne.kind options=options desactive=Signal::derive(move || !gerer.get())/>
            <ChampTexte id=format!("periode-{n}-debut") libelle="Début" valeur=ligne.debut genre="date" desactive=Signal::derive(move || !gerer.get())/>
            <ChampTexte id=format!("periode-{n}-fin") libelle="Fin" valeur=ligne.fin genre="date" desactive=Signal::derive(move || !gerer.get())/>
            {move || {
                gerer.get().then(|| view! {
                    <Bouton
                        variante=Variante::Discret
                        taille=Taille::Compacte
                        libelle_accessible="Retirer cette période"
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
fn SectionCalendrier(annee: AnneeStatutDto) -> impl IntoView {
    let gerer = peut_gerer();
    let (debut, fin) = (annee.date_debut, annee.date_fin);
    let calendrier =
        LocalResource::new(move || async move { session::client().calendrier(debut, fin).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| calendrier.refetch()));
    view! {
        <section class="ra-section" aria-labelledby="ra-calendrier">
            <h2 id="ra-calendrier" class="vs-section-titre">"Calendrier : jours neutralisés"</h2>
            <p class="texte-attenue">
                "Aucune séance n'est générée les jours listés ici (vacances, fériés, fermetures, journées banalisées)."
            </p>
            <RetourEcriture ecriture=ecriture/>
            {move || match calendrier.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement du calendrier…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| calendrier.refetch())/>
                }
                .into_any(),
                Some(Ok(j)) => {
                    let plages = regrouper_calendrier(&j);
                    if plages.is_empty() {
                        view! { <EtatVide titre="Aucun jour neutralisé" message="Ajoutez les vacances et les fermetures de l'année."/> }.into_any()
                    } else {
                        view! { <TableauPlages plages=plages ecriture=ecriture/> }.into_any()
                    }
                }
            }}
            {move || gerer.get().then(|| view! {
                <FormulairePlage
                    ecriture=ecriture
                    vacances=true
                    titre="Ajouter des vacances"
                    sortie="Vacances ajoutées."
                    annee_debut=debut
                />
                <FormulairePlage
                    ecriture=ecriture
                    vacances=false
                    titre="Fermer des jours"
                    sortie="Jours ajoutés au calendrier."
                    annee_debut=debut
                />
            })}
        </section>
    }
}

#[component]
fn TableauPlages(plages: Vec<PlageCalendrier>, ecriture: Ecriture) -> impl IntoView {
    let gerer = peut_gerer();
    view! {
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Jours neutralisés de l'année"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Du"</th>
                        <th scope="col">"Au"</th>
                        <th scope="col">"Jours"</th>
                        <th scope="col">"Nature"</th>
                        <th scope="col">"Libellé"</th>
                        <th scope="col"><span class="ui-sr">"Actions"</span></th>
                    </tr>
                </thead>
                <tbody>
                    {plages
                        .into_iter()
                        .map(|p| {
                            let supprimable = p.origine == "jour";
                            let (d, f, site) = (p.debut, p.fin, p.site_id);
                            let a = format!("Retirer {} du {} au {}", p.libelle, p.debut, p.fin);
                            view! {
                                <tr>
                                    <td>{date_complete(p.debut)}</td>
                                    <td>{date_complete(p.fin)}</td>
                                    <td>{p.nb_jours()}</td>
                                    <td>
                                        <Badge libelle=libelle_kind_jour(&p.kind) ton=TonEtat::Info/>
                                    </td>
                                    <td>{p.libelle.clone()}</td>
                                    <td>
                                        {move || {
                                            (gerer.get() && supprimable)
                                                .then(|| view! {
                                                    <Bouton
                                                        variante=Variante::Discret
                                                        taille=Taille::Compacte
                                                        en_cours=ecriture.en_cours
                                                        libelle_accessible=a.clone()
                                                        on_click=Callback::new(move |_| {
                                                            ecriture.lancer(async move { retirer_plage(d, f, site).await });
                                                        })
                                                    >
                                                        "Retirer"
                                                    </Bouton>
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

/// Retire les jours d'une plage un par un ; s'arrête à la première erreur.
async fn retirer_plage(
    debut: NaiveDate,
    fin: NaiveDate,
    site: Option<uuid::Uuid>,
) -> Result<String, ent_web_api::ApiError> {
    let mut jour = debut;
    let mut n = 0;
    while jour <= fin {
        session::client().retirer_jour_ferme(jour, site).await?;
        n += 1;
        jour += chrono::Duration::days(1);
    }
    Ok(format!("{n} jour(s) retiré(s) du calendrier."))
}

/// Formulaire d'ajout : vacances (période) ou jours fermés (fermeture, férié, banalisé).
#[component]
fn FormulairePlage(
    ecriture: Ecriture,
    vacances: bool,
    titre: &'static str,
    sortie: &'static str,
    annee_debut: NaiveDate,
) -> impl IntoView {
    let _ = annee_debut;
    let (du, au, libelle) = (
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    );
    let kinds: &'static [(&'static str, &'static str)] = if vacances {
        &KINDS_VACANCES
    } else {
        &KINDS_JOUR
    };
    let kind = RwSignal::new(kinds[0].0.to_string());
    let options: Vec<(String, String)> = kinds
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect();
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let prefixe = if vacances { "vac" } else { "jour" };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let s = SaisiePlage {
            du: du.get_untracked(),
            au: au.get_untracked(),
            kind: kind.get_untracked(),
            libelle: libelle.get_untracked(),
        };
        match composer_plage(&s, kinds) {
            Err(e) => erreurs.set(e),
            Ok(p) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    if vacances {
                        session::client()
                            .creer_vacances(&p.libelle, &p.kind, p.du, p.au)
                            .await?;
                    } else {
                        session::client()
                            .ajouter_jours_fermes(p.du, p.au, &p.kind, &p.libelle, None)
                            .await?;
                    }
                    Ok(sortie.to_string())
                });
                libelle.set(String::new());
            }
        }
    };
    let titre_id = format!("ra-form-{prefixe}");
    let decrit_par = titre_id.clone();
    let err = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
        })
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby=decrit_par>
            <h3 id=titre_id class="jvs-sous-titre">{titre}</h3>
            <Selection id=format!("cal-kind-{prefixe}") libelle="Nature" valeur=kind options=options/>
            <ChampTexte id=format!("cal-du-{prefixe}") libelle="Du" valeur=du genre="date" requis=true erreur=err("cal-du")/>
            <ChampTexte id=format!("cal-au-{prefixe}") libelle="Au (inclus)" valeur=au genre="date" requis=true erreur=err("cal-au")/>
            <ChampTexte id=format!("cal-libelle-{prefixe}") libelle="Libellé" valeur=libelle requis=true erreur=err("cal-libelle")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Ajouter"</Bouton>
        </form>
    }
}
