//! Campagnes d'information (COM-05) : préparation, **aperçu des destinataires**, confirmation
//! humaine, bilan des envois.
//!
//! Données : routes `/campagnes/*` d'`ent-api`. Le ciblage, les droits, les plafonds et
//! l'empreinte de l'aperçu sont décidés par le serveur ; l'écran ne propose le lancement
//! qu'après un aperçu et une confirmation explicite (et, pour les SMS, une seconde case).

pub mod modele;

use ent_types::communication::{ApercuCampagneDto, CampagneDto};
use ent_web_ui::{
    Badge, Bouton, Case, ChampTexte, Confirmation, EnTetePage, EtatVide, Selection, Squelette,
    Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    canaux_texte, composer, corps_lancement, libelle_audience, libelle_canal, libelle_statut,
    libelle_statut_envoi, lignes_apercu, message_confirmation, obstacle, peut_confirmer,
    ton_statut, SaisieCampagne,
};

#[component]
pub fn PageCampagnes() -> impl IntoView {
    let campagnes = LocalResource::new(|| async { session::client().campagnes().await });
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let apercu = RwSignal::new(None::<(Uuid, ApercuCampagneDto)>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| campagnes.refetch()));
    view! {
        <Title text="Campagnes"/>
        <EnTetePage
            surtitre="Communication".to_string()
            titre="Campagnes d'information".to_string()
            sous_titre="Un message à des familles par e-mail, SMS ou mobile. Rien ne part avant votre confirmation.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match apercu.get() {
            Some((id, a)) => view! { <PanneauApercu id=id apercu=a signal=apercu ecriture=ecriture/> }.into_any(),
            None => view! { <FormulaireCampagne classes=classes apercu=apercu ecriture=ecriture/> }.into_any(),
        }}
        <section class="ra-section" aria-labelledby="camp-liste">
            <h2 id="camp-liste" class="vs-section-titre">"Campagnes"</h2>
            {move || match campagnes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des campagnes…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| campagnes.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune campagne" message="Préparez votre première campagne ci-dessus."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeCampagnes campagnes=l apercu=apercu ecriture=ecriture/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn FormulaireCampagne(
    classes: LocalResource<Result<Vec<ent_web_api::ClasseApi>, ent_web_api::ApiError>>,
    apercu: RwSignal<Option<(Uuid, ApercuCampagneDto)>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let corps = RwSignal::new(String::new());
    let email = RwSignal::new(true);
    let sms = RwSignal::new(false);
    let push = RwSignal::new(false);
    let audience = RwSignal::new("classe".to_string());
    let classe = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let preparer = Callback::new(move |_| {
        let saisie = SaisieCampagne {
            titre: titre.get_untracked(),
            corps: corps.get_untracked(),
            email: email.get_untracked(),
            sms: sms.get_untracked(),
            push: push.get_untracked(),
            audience: audience.get_untracked(),
            classe_id: classe.get_untracked(),
        };
        match composer(&saisie) {
            Err(m) => erreur.set(Some(m)),
            Ok(body) => {
                erreur.set(None);
                ecriture.lancer(async move {
                    let c = session::client().creer_campagne_information(&body).await?;
                    let a = session::client().apercu_campagne(c.id).await?;
                    apercu.set(Some((c.id, a)));
                    Ok(
                        "Aperçu des destinataires prêt : vérifiez-le avant de confirmer."
                            .to_string(),
                    )
                });
            }
        }
    });
    view! {
        <form
            class="ra-section"
            on:submit=move |ev| {
                ev.prevent_default();
                preparer.run(());
            }
        >
            <h2 class="vs-section-titre">"Nouvelle campagne"</h2>
            <ChampTexte id="camp-titre" libelle="Titre" valeur=titre requis=true/>
            <ZoneTexte id="camp-corps" libelle="Message" valeur=corps requis=true/>
            <fieldset class="ui-champ">
                <legend class="ui-champ__libelle">"Canaux"</legend>
                <Case id="camp-email" libelle="E-mail" coche=email/>
                <Case id="camp-push" libelle="Notification sur mobile" coche=push/>
                <Case id="camp-sms" libelle="SMS (coût, soumis à plafond)" coche=sms/>
            </fieldset>
            <Selection
                id="camp-audience"
                libelle="Destinataires"
                valeur=audience
                options=vec![
                    ("classe".to_string(), libelle_audience("classe").to_string()),
                    ("etablissement".to_string(), libelle_audience("etablissement").to_string()),
                ]
            />
            {move || (audience.get() == "classe").then(|| match classes.get() {
                Some(Ok(l)) => {
                    let mut options = vec![(String::new(), "— Choisir une classe —".to_string())];
                    options.extend(l.into_iter().map(|c| (c.id.to_string(), c.libelle)));
                    view! { <Selection id="camp-classe" libelle="Classe" valeur=classe options=options/> }.into_any()
                }
                _ => view! { <Squelette lignes=1 libelle="Chargement des classes…"/> }.into_any(),
            })}
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                "Voir les destinataires"
            </Bouton>
        </form>
    }
}

#[component]
fn PanneauApercu(
    id: Uuid,
    apercu: ApercuCampagneDto,
    signal: RwSignal<Option<(Uuid, ApercuCampagneDto)>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let sms_coche = RwSignal::new(false);
    let ouvert = RwSignal::new(false);
    let a = StoredValue::new(apercu.clone());
    let sms_requise = modele::confirmation_sms_requise(&apercu);
    let lancer = Callback::new(move |_| {
        let body = a.with_value(|a| corps_lancement(a, sms_coche.get_untracked()));
        ecriture.lancer(async move {
            session::client().lancer_campagne(id, &body).await?;
            signal.set(None);
            Ok("Campagne lancée.".to_string())
        });
    });
    let annuler = Callback::new(move |_| {
        ecriture.lancer(async move {
            session::client().annuler_campagne(id).await?;
            signal.set(None);
            Ok("Campagne abandonnée : rien n'a été envoyé.".to_string())
        });
    });
    let message = message_confirmation(&apercu);
    view! {
        <section class="ra-section" aria-labelledby="camp-apercu">
            <h2 id="camp-apercu" class="vs-section-titre">"Aperçu des destinataires"</h2>
            <ul>
                {lignes_apercu(&apercu).into_iter().map(|l| view! { <li>{l}</li> }).collect_view()}
            </ul>
            <details>
                <summary>{format!("Voir les destinataires ({} affichés)", apercu.destinataires.len())}</summary>
                <ul class="jvs-liste">
                    {apercu.destinataires.iter().map(|d| {
                        let canaux = d.canaux.iter().map(|c| libelle_canal(c)).collect::<Vec<_>>().join(", ");
                        view! {
                            <li class="jvs-ligne">
                                <span>{format!("{} · {} élève(s)", d.nom, d.nb_eleves)}</span>
                                <span class="texte-attenue">{if canaux.is_empty() { "ENT seulement".to_string() } else { canaux }}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>
                {apercu.tronque.then(|| view! { <p class="texte-attenue">"Liste tronquée : le total ci-dessus est exact."</p> })}
            </details>
            {sms_requise.then(|| view! {
                <Case id="camp-confirme-sms" libelle="Je confirme l'envoi de SMS payants à ces destinataires" coche=sms_coche/>
            })}
            {move || a.with_value(|a| obstacle(a, sms_coche.get())).map(|m| view! { <p class="texte-attenue" role="status">{m}</p> })}
            <div class="ra-actions">
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=annuler>
                    "Abandonner"
                </Bouton>
                <Bouton
                    variante=Variante::Primaire
                    desactive=Signal::derive(move || !a.with_value(|a| peut_confirmer(a, sms_coche.get())))
                    en_cours=ecriture.en_cours
                    on_click=Callback::new(move |_| ouvert.set(true))
                >
                    "Envoyer la campagne…"
                </Bouton>
            </div>
            <Confirmation
                ouvert=ouvert
                id="camp-confirmation"
                titre="Envoyer la campagne ?"
                message=message
                libelle_confirmer="Envoyer maintenant"
                on_confirmer=lancer
            />
        </section>
    }
}

#[component]
fn ListeCampagnes(
    campagnes: Vec<CampagneDto>,
    apercu: RwSignal<Option<(Uuid, ApercuCampagneDto)>>,
    ecriture: Ecriture,
) -> impl IntoView {
    view! {
        <ul class="jvs-liste">
            {campagnes.into_iter().map(|c| {
                let id = c.id;
                let reprendre = c.statut == "brouillon" || c.statut == "apercu";
                let lancee = c.statut == "lancee";
                let detail = format!(
                    "{} · {}{}",
                    libelle_audience(&c.audience),
                    canaux_texte(&c),
                    c.nb_destinataires.map(|n| format!(" · {n} destinataire(s)")).unwrap_or_default()
                );
                view! {
                    <li class="jvs-ligne">
                        <div>
                            <strong>{c.titre.clone()}</strong>
                            <span class="texte-attenue">{format!(" — {detail}")}</span>
                            {lancee.then(|| view! { <BilanCampagne id=id/> })}
                        </div>
                        <Badge libelle=libelle_statut(&c.statut) ton=ton_statut(&c.statut)/>
                        {reprendre.then(|| view! {
                            <Bouton
                                variante=Variante::Secondaire
                                taille=Taille::Compacte
                                en_cours=ecriture.en_cours
                                libelle_accessible=format!("Revoir les destinataires de {}", c.titre)
                                on_click=Callback::new(move |_| {
                                    ecriture.lancer(async move {
                                        let a = session::client().apercu_campagne(id).await?;
                                        apercu.set(Some((id, a)));
                                        Ok("Aperçu recalculé.".to_string())
                                    });
                                })
                            >
                                "Revoir l'aperçu"
                            </Bouton>
                        })}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

/// Bilan des envois d'une campagne lancée (par canal et statut).
#[component]
fn BilanCampagne(id: Uuid) -> impl IntoView {
    let bilan =
        LocalResource::new(move || async move { session::client().bilan_campagne(id).await });
    view! {
        {move || match bilan.get() {
            Some(Ok(l)) if !l.is_empty() => view! {
                <p class="texte-attenue">
                    {l.into_iter()
                        .map(|b| format!("{} · {} : {}", libelle_canal(&b.canal), libelle_statut_envoi(&b.statut), b.nombre))
                        .collect::<Vec<_>>()
                        .join(" ; ")}
                </p>
            }
            .into_any(),
            _ => ().into_any(),
        }}
    }
}
