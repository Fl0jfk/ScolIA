//! Paie de l'OGEC et file DSN (A11d, ABP-207, 209, 210, 211) : éléments variables par salarié et par mois,
//! préparation, export (les éléments « préparés » deviennent « envoyés » et ne se réécrivent plus), arrêts à
//! signaler en DSN sous cinq jours.
//!
//! La nature d'une absence n'est jamais un libellé : l'élément de paie porte un **code** (AD-03), réservé par le
//! serveur à la comptabilité et à la direction. Une erreur après envoi est une **régularisation** du mois
//! ouvert, jamais une réécriture.

use chrono::NaiveDate;
use ent_types::absences_personnels::{DsnSuiviDto, ElementPaieDto};
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, EtatVide, Niveau, Squelette, Taille, Variante,
};
use leptos::prelude::*;

use super::commun::lancer_export;
use super::modele_heures::{
    libelle_etat_element, libelle_mois, libelle_unite, ton_etat_element, Noms,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::Ecriture;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_longue;

/// Quantité lisible : « 7 h », « 1,5 j », « 2 demi-j. ».
pub fn libelle_quantite(e: &ElementPaieDto) -> String {
    let q = if (e.quantite - e.quantite.round()).abs() < 0.005 {
        format!("{}", e.quantite.round() as i64)
    } else {
        format!("{:.2}", e.quantite)
            .trim_end_matches('0')
            .replace('.', ",")
    };
    format!("{q} {}", libelle_unite(&e.unite))
}

#[component]
pub fn SectionPaie(
    mois: Signal<NaiveDate>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let erreur_export = RwSignal::new(None::<String>);
    let elements = LocalResource::new(move || {
        let _ = ecriture.annonce.get();
        let m = mois.get();
        async move { session::client().absp_elements_paie(m).await }
    });
    view! {
        <section class="absp-section" aria-labelledby="absp-paie">
            <h2 id="absp-paie" class="vs-section-titre">
                {move || format!("Éléments variables de paie — {}", libelle_mois(mois.get()))}
            </h2>
            <p class="texte-attenue">
                "Absences par nature codée, jours, heures. L'export envoie les éléments préparés à la paie ; une correction ultérieure est une régularisation du mois ouvert."
            </p>
            <p class="absp-actions">
                <Bouton
                    on_click=Callback::new(move |_| {
                        let m = mois.get_untracked();
                        ecriture.lancer(async move {
                            let b = session::client().absp_preparer_paie(m).await?;
                            Ok(format!(
                                "Éléments préparés : {} créé(s), {} régularisation(s).",
                                b.crees, b.regularisations
                            ))
                        });
                    })
                >
                    "Préparer les éléments du mois"
                </Bouton>
                {move || moteur.get().a("personnels.paie.absences").then(|| view! {
                    <Bouton
                        variante=Variante::Secondaire
                        on_click=Callback::new(move |_| {
                            let m = mois.get_untracked();
                            lancer_export(
                                async move { session::client().absp_paie_csv(m).await },
                                "elements-variables-paie.csv",
                                erreur_export,
                                ecriture,
                            );
                        })
                    >
                        "Exporter (CSV)"
                    </Bouton>
                })}
            </p>
            {move || erreur_export.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || match elements.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des éléments…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| elements.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! {
                    <EtatVide
                        titre="Aucun élément pour ce mois"
                        message="Préparez les éléments du mois pour les voir ici."
                    />
                }
                .into_any(),
                Some(Ok(v)) => view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|e| view! { <CarteElement e=e noms=noms/> }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn CarteElement(e: ElementPaieDto, noms: LocalResource<Noms>) -> impl IntoView {
    let agent = e.agent_id;
    let regul = e.regularise_de_id.is_some();
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{move || noms.get().unwrap_or_default().nom(agent)}</strong>
                <Badge libelle=libelle_etat_element(&e.etat) ton=ton_etat_element(&e.etat)/>
            </div>
            <p class="absp-carte__detail">
                <span>"Code "</span><code>{e.nature_code.clone()}</code>
                {format!(" · {}", libelle_quantite(&e))}
                {regul.then_some(" · régularise un élément déjà envoyé")}
            </p>
        </li>
    }
}

fn echeance_dsn(d: &DsnSuiviDto) -> String {
    match d.signale_le {
        Some(s) => format!("Signalé en DSN le {}.", date_longue(s)),
        None if d.en_retard => format!(
            "À signaler : échéance dépassée le {}.",
            date_longue(d.echeance_le)
        ),
        None => format!("À signaler avant le {}.", date_longue(d.echeance_le)),
    }
}

/// Arrêts à signaler en DSN dans les cinq jours (ABP-210) ; « signalé le » (ABP-211).
#[component]
pub fn SectionDsn(noms: LocalResource<Noms>, ecriture: Ecriture, auj: NaiveDate) -> impl IntoView {
    let liste = LocalResource::new(move || {
        let _ = ecriture.annonce.get();
        async { session::client().absp_dsn().await }
    });
    view! {
        <section class="absp-section" aria-labelledby="absp-dsn">
            <h2 id="absp-dsn" class="vs-section-titre">"Arrêts à signaler en DSN"</h2>
            <p class="texte-attenue">"Un arrêt de travail se signale en DSN dans les cinq jours ouvrés."</p>
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de la file DSN…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! {
                    <EtatVide titre="Aucun arrêt à signaler" message="Les arrêts de travail à déclarer en DSN apparaissent ici."/>
                }
                .into_any(),
                Some(Ok(v)) => view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|d| view! { <CarteDsn d=d noms=noms ecriture=ecriture auj=auj/> }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn CarteDsn(
    d: DsnSuiviDto,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
    auj: NaiveDate,
) -> impl IntoView {
    let agent = d.agent_id;
    let absence = d.absence_id;
    let jour = RwSignal::new(auj.format("%Y-%m-%d").to_string());
    let erreur = RwSignal::new(None::<String>);
    let a_signaler = d.signale_le.is_none();
    let (libelle, ton) = if d.signale_le.is_some() {
        ("Signalé", ent_web_ui::TonEtat::Ok)
    } else if d.en_retard {
        ("En retard", ent_web_ui::TonEtat::Mal)
    } else {
        ("À signaler", ent_web_ui::TonEtat::Avertissement)
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Ok(le) = NaiveDate::parse_from_str(jour.get_untracked().trim(), "%Y-%m-%d") else {
            erreur.set(Some("Indiquez la date du signalement.".into()));
            return;
        };
        erreur.set(None);
        ecriture.lancer(async move {
            session::client().absp_dsn_signale(absence, le).await?;
            Ok("Signalement en DSN noté.".to_string())
        });
    };
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{move || noms.get().unwrap_or_default().nom(agent)}</strong>
                <Badge libelle=libelle ton=ton/>
            </div>
            <p class="absp-carte__detail">{echeance_dsn(&d)}</p>
            {a_signaler.then(|| view! {
                <form class="absp-dates absp-dates--ligne" on:submit=envoyer>
                    <ChampTexte
                        id=format!("absp-dsn-{absence}")
                        libelle="Signalé en DSN le"
                        valeur=jour
                        genre="date"
                        erreur=erreur
                    />
                    <Bouton soumettre=true taille=Taille::Compacte libelle_accessible="Marquer cet arrêt comme signalé en DSN">
                        "Marquer signalé"
                    </Bouton>
                </form>
            })}
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn element(q: f64, unite: &str) -> ElementPaieDto {
        ElementPaieDto {
            id: Uuid::nil(),
            agent_id: Uuid::nil(),
            absence_id: None,
            heure_id: None,
            mois: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            nature_code: "MAL".into(),
            quantite: q,
            unite: unite.into(),
            etat: "prepare".into(),
            regularise_de_id: None,
        }
    }

    #[test]
    fn les_quantites_se_lisent_avec_leur_unite() {
        assert_eq!(libelle_quantite(&element(7.0, "heure")), "7 h");
        assert_eq!(libelle_quantite(&element(1.5, "jour")), "1,5 j");
        assert_eq!(libelle_quantite(&element(2.0, "demi_jour")), "2 demi-j.");
        assert_eq!(libelle_quantite(&element(0.25, "jour")), "0,25 j");
    }

    #[test]
    fn l_echeance_dsn_dit_ce_qui_reste() {
        let base = DsnSuiviDto {
            absence_id: Uuid::nil(),
            agent_id: Uuid::nil(),
            echeance_le: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
            signale_le: None,
            en_retard: false,
        };
        assert!(echeance_dsn(&base).starts_with("À signaler avant le"));
        let retard = DsnSuiviDto {
            en_retard: true,
            ..base.clone()
        };
        assert!(echeance_dsn(&retard).contains("dépassée"));
        let fait = DsnSuiviDto {
            signale_le: NaiveDate::from_ymd_opt(2026, 11, 18),
            ..base
        };
        assert!(echeance_dsn(&fait).starts_with("Signalé en DSN le"));
    }
}
