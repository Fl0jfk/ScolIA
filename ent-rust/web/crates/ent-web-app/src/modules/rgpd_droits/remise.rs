//! Sections **Réponse et remise** (fiche d'une demande) et **Mes extraits** (côté personne), socle 27, lot G3b-3.
//!
//! * Remise : passkey **récente** du gestionnaire, défi lié à l'extrait (`webauthn::signer_defi`) ; en ligne
//!   seulement sur le compte du demandeur, sinon en main propre avec émargement. Aucune adresse fournie par la
//!   demande n'est jamais utilisée.
//! * Mes données : fichiers, échéance de mise à disposition, téléchargement derrière la passkey du demandeur.
//!
//! Logique pure : `modele_remise.rs` ; `ui.rs` n'est pas étendu.

use ent_web_api::rgpd_extrait_types::{ExtraitDto, RemettreBody, RenvoiBody, TelechargerBody};
use ent_web_api::ApiError;
use ent_web_ui::{Alerte, Badge, Bouton, ChampTexte, Niveau, Squelette, TonEtat, Variante};
use leptos::prelude::*;
use uuid::Uuid;

use super::modele_remise::{
    emargement_recevable, etat_fichier, libelle_mode, libelle_renvoi, nom_lisible, remettable,
    taille_lisible, telechargeable,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::telechargement::enregistrer_telechargement;
use crate::noyau::webauthn;

/// Échec du navigateur (passkey refusée, annulée) : présenté comme une saisie refusée.
fn echec_navigateur(m: String) -> ApiError {
    ApiError::Invalide(m)
}

#[component]
pub fn SectionReponseRemise(demande: Uuid, ouverte: bool) -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let extraits = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().rgpd_extraits(demande).await }
    });
    view! {
        <section class="classe-section" aria-labelledby="rgpd-remise-titre">
            <h2 id="rgpd-remise-titre">"Réponse et remise"</h2>
            <p>
                "La remise engage l'établissement : elle demande votre passkey, liée à l'extrait relu. \
                 En ligne, elle ne part que vers le compte du demandeur dans cet établissement ; sinon, \
                 remettez l'extrait en main propre et faites émarger la personne."
            </p>
            <RetourEcriture ecriture=ecriture/>
            {move || match extraits.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des extraits…"/> }.into_any(),
                Some(Err(ApiError::Introuvable(_) | ApiError::Interdit(_))) => {
                    view! { <p class="texte-attenue">"Aucun extrait à remettre."</p> }.into_any()
                }
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| extraits.refetch())/>
                }.into_any(),
                Some(Ok(v)) => {
                    let courants: Vec<ExtraitDto> = v.into_iter().filter(|e| !e.perime).collect();
                    if courants.is_empty() {
                        view! { <p class="texte-attenue">"Aucun extrait assemblé pour le moment."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="ui-liste">
                                {courants.into_iter().map(|e| view! {
                                    <CarteRemise e=e demande=demande ecriture=ecriture ouverte=ouverte/>
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }
                }
            }}
            <Renvois demande=demande ouverte=ouverte/>
        </section>
    }
}

#[component]
fn CarteRemise(e: ExtraitDto, demande: Uuid, ecriture: Ecriture, ouverte: bool) -> impl IntoView {
    let id = e.id;
    let prete = remettable(&e);
    let nom = RwSignal::new(String::new());
    let piece = RwSignal::new(String::new());
    let titre = format!("Extrait version {} — {} ligne(s)", e.version, e.nb_lignes);
    let remettre = move |mode: &'static str| {
        let (n, p) = (nom.get_untracked(), piece.get_untracked());
        ecriture.lancer(async move {
            let client = session::client();
            let defi = client.rgpd_remise_defi(demande, id).await?;
            let assertion = webauthn::signer_defi(&defi.options)
                .await
                .map_err(echec_navigateur)?;
            let corps = RemettreBody {
                mode: mode.to_string(),
                defi_id: defi.defi_id,
                credential: assertion,
                emargement_nom: (mode == "main_propre").then(|| n.clone()),
                emargement_piece_vue: (mode == "main_propre").then(|| p.clone()),
            };
            let remise = client.rgpd_remettre(demande, id, &corps).await?;
            // En main propre, le PDF est rendu une seule fois : il s'enregistre pour être imprimé.
            if let Some(b64) = remise.pdf_base64.as_deref() {
                if let Some(octets) = decoder_base64(b64) {
                    let _ = enregistrer_telechargement("extrait.pdf", "application/pdf", &octets);
                }
            }
            Ok(if mode == "en_ligne" {
                "Extrait remis en ligne : la personne en est avertie.".to_string()
            } else {
                "Remise en main propre enregistrée : imprimez le PDF et recueillez l'émargement."
                    .to_string()
            })
        });
    };
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{titre}</span>
                <span class="ui-liste__detail">
                    <Badge libelle=if prete { "Prêt à être remis".to_string() } else { "Pas encore remissible".to_string() }
                        ton=if prete { TonEtat::Ok } else { TonEtat::Avertissement }/>
                </span>
                {(!prete).then(|| view! {
                    <Alerte niveau=Niveau::Info titre="Remise impossible pour le moment".to_string()>
                        "Décidez d'abord les marquages et les sections incomplètes dans la section « Extrait »."
                    </Alerte>
                })}
                {(ouverte && prete).then(|| view! {
                    <div>
                        <Bouton variante=Variante::Primaire
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            on_click=Callback::new(move |_| remettre("en_ligne"))>
                            {libelle_mode("en_ligne")}
                        </Bouton>
                        <fieldset>
                            <legend>{libelle_mode("main_propre")}</legend>
                            <ChampTexte id=format!("rgpd-emarge-nom-{id}") libelle="Nom de la personne qui émarge".to_string()
                                valeur=nom requis=true/>
                            <ChampTexte id=format!("rgpd-emarge-piece-{id}") libelle="Pièce d'identité vue".to_string()
                                valeur=piece requis=true/>
                            <Bouton variante=Variante::Secondaire
                                desactive=Signal::derive(move || {
                                    ecriture.en_cours.get() || !emargement_recevable(&nom.get(), &piece.get())
                                })
                                on_click=Callback::new(move |_| remettre("main_propre"))>
                                "Remettre en main propre"
                            </Bouton>
                        </fieldset>
                    </div>
                })}
            </div>
        </li>
    }
}

/// Renvois tracés : rectification, retrait de consentement, CADA d'un tiers.
#[component]
fn Renvois(demande: Uuid, ouverte: bool) -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let liste = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().rgpd_renvois(demande).await }
    });
    let nature = RwSignal::new("rectification".to_string());
    let cible = RwSignal::new(String::new());
    let texte = RwSignal::new(String::new());
    view! {
        <h3>"Renvois"</h3>
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=1 libelle="Chargement des renvois…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! { <p class="texte-attenue">"Réservé au gestionnaire."</p> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <p class="texte-attenue">"Aucun renvoi tracé."</p> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="ui-liste">
                    {l.into_iter().map(|r| view! {
                        <li class="ui-liste__element">
                            <div class="ui-liste__principal">
                                <span class="ui-liste__titre">{libelle_renvoi(&r.nature)}</span>
                                <span class="ui-liste__detail">{format!("{} — {}", r.cible, r.texte)}</span>
                            </div>
                        </li>
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
        {ouverte.then(|| view! {
            <fieldset>
                <legend>"Tracer un renvoi"</legend>
                <label for="rgpd-renvoi-nature">"Nature"</label>
                <select id="rgpd-renvoi-nature"
                    on:change=move |ev| nature.set(event_target_value(&ev))>
                    <option value="rectification">{libelle_renvoi("rectification")}</option>
                    <option value="retrait_consentement">{libelle_renvoi("retrait_consentement")}</option>
                    <option value="cada_tiers">{libelle_renvoi("cada_tiers")}</option>
                </select>
                <ChampTexte id="rgpd-renvoi-cible".to_string() libelle="Cible (socle, service, SIECLE…)".to_string()
                    valeur=cible requis=false/>
                <ChampTexte id="rgpd-renvoi-texte".to_string() libelle="Ce qui est renvoyé ou corrigé".to_string()
                    valeur=texte requis=true/>
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || ecriture.en_cours.get() || texte.get().trim().chars().count() < 3)
                    on_click=Callback::new(move |_| {
                        let b = RenvoiBody { nature: nature.get(), cible: cible.get(), texte: texte.get() };
                        ecriture.lancer(async move {
                            session::client().rgpd_renvoi_tracer(demande, &b).await
                                .map(|_| "Renvoi tracé.".to_string())
                        });
                    })>
                    "Tracer le renvoi"
                </Bouton>
            </fieldset>
        })}
    }
}

/// Base64 standard → octets (l'extrait remis en main propre arrive ainsi dans la réponse de l'API).
fn decoder_base64(texte: &str) -> Option<Vec<u8>> {
    let valeur = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let t = texte.as_bytes();
    if !t.len().is_multiple_of(4) {
        return None;
    }
    let mut sortie = Vec::with_capacity(t.len() / 4 * 3);
    for bloc in t.chunks(4) {
        let pad = bloc.iter().filter(|&&c| c == b'=').count();
        if pad > 2 {
            return None;
        }
        let mut n = 0u32;
        for &c in &bloc[..4 - pad] {
            n = (n << 6) | valeur(c)?;
        }
        n <<= 6 * pad as u32;
        sortie.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8][..3 - pad]);
    }
    Some(sortie)
}

// ═══════════════════════════════════ Côté personne ═══════════════════════════════════

/// « Mes données » : « votre extrait est prêt », fichiers, échéance, téléchargement par passkey.
#[component]
pub fn MesExtraits() -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let liste = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().rgpd_mes_extraits().await }
    });
    view! {
        <section class="classe-section" aria-labelledby="rgpd-mes-extraits">
            <h2 id="rgpd-mes-extraits">"Mes extraits"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement de vos extraits…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
                }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <p class="texte-attenue">"Aucun extrait n'est disponible pour le moment."</p>
                }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="ui-liste">
                        {l.into_iter().map(|m| {
                            let extrait = m.remise.extrait_id;
                            let maintenant = chrono::Utc::now();
                            view! {
                                <li class="ui-liste__element">
                                    <div class="ui-liste__principal">
                                        <span class="ui-liste__titre">
                                            {format!("Votre extrait est prêt — demande {}", m.demande_reference)}
                                        </span>
                                        <ul>
                                            {m.remise.fichiers.into_iter().map(|f| {
                                                let ok = telechargeable(&f, maintenant);
                                                let (fid, nom) = (f.id, f.nom.clone());
                                                view! {
                                                    <li>
                                                        <span>{format!("{} · {}", nom_lisible(&f), taille_lisible(f.octets))}</span>
                                                        <span class="ui-liste__detail">{etat_fichier(&f, maintenant)}</span>
                                                        <Bouton variante=Variante::Secondaire
                                                            desactive=Signal::derive(move || ecriture.en_cours.get() || !ok)
                                                            on_click=Callback::new(move |_| {
                                                                let nom = nom.clone();
                                                                ecriture.lancer(async move {
                                                                    let client = session::client();
                                                                    let defi = client.rgpd_telechargement_defi(extrait).await?;
                                                                    let assertion = webauthn::signer_defi(&defi.options)
                                                                        .await
                                                                        .map_err(echec_navigateur)?;
                                                                    let f = client
                                                                        .rgpd_telecharger(fid, &TelechargerBody {
                                                                            defi_id: defi.defi_id,
                                                                            credential: assertion,
                                                                        })
                                                                        .await?;
                                                                    let octets = webauthn::b64url_decoder(&f.contenu_base64url)
                                                                        .ok_or_else(|| ApiError::Decodage("fichier illisible".into()))?;
                                                                    let mime = if f.type_fichier == "pdf" {
                                                                        "application/pdf"
                                                                    } else {
                                                                        "text/csv"
                                                                    };
                                                                    enregistrer_telechargement(&nom, mime, &octets)
                                                                        .map_err(ApiError::Invalide)?;
                                                                    Ok("Fichier enregistré.".to_string())
                                                                });
                                                            })>
                                                            "Télécharger (passkey)"
                                                        </Bouton>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    </div>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::decoder_base64;

    #[test]
    fn decodage_base64_de_la_remise_en_main_propre() {
        assert_eq!(decoder_base64("TWFu").unwrap(), b"Man");
        assert_eq!(decoder_base64("TWE=").unwrap(), b"Ma");
        assert_eq!(decoder_base64("TQ==").unwrap(), b"M");
        assert!(decoder_base64("TWF").is_none());
        assert!(decoder_base64("T*Fu").is_none());
    }
}
