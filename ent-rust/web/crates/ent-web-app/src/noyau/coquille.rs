//! Coquille authentifiée : charge la personne connectée une seule fois, protège les
//! pages, construit la navigation selon les droits, gère la déconnexion (y compris
//! dans les autres onglets) et la reconnexion rapide quand la session expire.

use ent_web_api::{ApiError, MoiSecurite};
use ent_web_ui::{Coquille, Identite};
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};

use super::droits::{droits_du_role, libelle_role, Droits};
use super::navigation;
use super::non_lus::{self, NonLus};
use super::portee::{self, Portee};
use super::reconnexion::{self, BandeauExpiration, ReconnexionRapide};
use super::session::{self, ContexteSession, SuiteDiffusion};
use crate::modules::connexion::config_connexion;

/// Droits de la personne connectée (aucun tant que `/auth/me` n'a pas répondu).
pub fn use_droits() -> Signal<Droits> {
    let ctx = session::use_session();
    Signal::derive(move || {
        ctx.moi
            .get()
            .and_then(Result::ok)
            .map(|m| droits_du_role(&m.role))
            .unwrap_or_default()
    })
}

#[component]
pub fn AppCoquille() -> impl IntoView {
    let navigate = use_navigate();
    let location = use_location();
    // LocalResource (non sérialisée) : lue hors <Suspense/>, un Resource provoquerait
    // l'avertissement Leptos et des erreurs d'hydratation.
    let config = LocalResource::new(|| async { config_connexion().await });

    let chemin_complet = move || {
        session::chemin_complet(
            &location.pathname.get_untracked(),
            &location.search.get_untracked(),
        )
    };

    // Ancien jeton `localStorage` (avant A6) : effacé et révoqué (navigateur seulement).
    Effect::new(move |_| session::purger_ancien_jeton());

    // Reconnexion rapide : échéances de la session, fenêtre par-dessus la page.
    let reco = reconnexion::fournir();

    // Une seule requête `/auth/me` : identité + portée et niveau de la session.
    let securite = RwSignal::new(None::<MoiSecurite>);
    let moi = LocalResource::new(move || async move {
        let m = session::client().moi_securite().await?;
        let identite = m.identite();
        securite.set(Some(m));
        Ok(identite)
    });
    provide_context(ContexteSession { moi, securite });

    // Passkey obligatoire non enrôlée : écran d'enrôlement, sans autre chemin.
    let en_enrolement = Memo::new(move |_| {
        securite.with(|s| {
            s.as_ref()
                .is_some_and(|m| Portee::depuis(&m.portee) == Portee::Enrolement)
        })
    });
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let portee = if en_enrolement.get() {
                Portee::Enrolement
            } else {
                Portee::Complete
            };
            if let Some(cible) = portee::decision_redirection(portee, &location.pathname.get()) {
                navigate(cible, Default::default());
            }
        });
    }

    // Pas de session (premier chargement) : connexion, puis retour ici. Session perdue
    // pendant l'usage (identité déjà connue) : fenêtre de reconnexion, la page reste.
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if let Some(Err(ApiError::NonConnecte)) = moi.get() {
                if securite.get_untracked().is_some() {
                    reco.ouverte.set(true);
                } else {
                    navigate(
                        &session::adresse_connexion(&chemin_complet()),
                        Default::default(),
                    );
                }
            }
        });
    }

    // Identité connue : on demande au serveur le temps restant (une fois), et on prévient
    // les autres onglets qu'une session est ouverte pour cette personne.
    Effect::new(move |deja: Option<bool>| {
        let connue = securite.with(Option::is_some);
        if connue && deja != Some(true) {
            reco.rafraichir();
        }
        connue
    });

    // Autres onglets : déconnexion, autre compte, reconnexion de la même personne.
    {
        let navigate = navigate.clone();
        session::ecouter(Some(std::rc::Rc::new(move |m| {
            let utilisateur = securite.get_untracked().map(|s| s.user_id);
            match session::suite_message(m, utilisateur) {
                SuiteDiffusion::Ignorer => {}
                SuiteDiffusion::Deconnexion => {
                    navigate("/web/connexion?fin=autre-onglet", Default::default());
                }
                // Autre compte ailleurs : on repart de zéro (identité, droits, données).
                SuiteDiffusion::Recharger => {
                    #[cfg(target_arch = "wasm32")]
                    let _ = window().location().reload();
                }
                SuiteDiffusion::Rouvrir => {
                    if reco.ouverte.get_untracked() {
                        reco.ouverte.set(false);
                        reco.version.update(|v| *v += 1);
                    }
                    reco.rafraichir();
                }
            }
        })));
        on_cleanup(|| session::ecouter(None));
    }

    // Après chaque changement de page : focus sur <main> et annonce du titre.
    #[cfg(target_arch = "wasm32")]
    {
        let chemin = Memo::new(move |_| location.pathname.get());
        Effect::new(move |precedent: Option<()>| {
            chemin.track();
            if precedent.is_some() {
                apres_navigation();
            }
        });
    }

    let droits = Signal::derive(move || {
        moi.get()
            .and_then(Result::ok)
            .map(|m| droits_du_role(&m.role))
            .unwrap_or_default()
    });
    // Messages non lus : lus au chargement, à chaque changement de page et sur demande
    // (la messagerie rafraîchit après lecture ou envoi).
    let version_non_lus = RwSignal::new(0u32);
    let total_non_lus = RwSignal::new(0i64);
    let non_lus_res = LocalResource::new(move || {
        version_non_lus.track();
        let autorise = droits.get().messagerie;
        async move {
            if autorise {
                session::client()
                    .fils_messages()
                    .await
                    .map(|l| non_lus::total_non_lus(&l))
                    .ok()
            } else {
                Some(0)
            }
        }
    });
    Effect::new(move |_| {
        if let Some(Some(n)) = non_lus_res.get() {
            total_non_lus.set(n);
        }
    });
    {
        let chemin = Memo::new(move |_| location.pathname.get());
        Effect::new(move |precedent: Option<()>| {
            chemin.track();
            if precedent.is_some() {
                version_non_lus.update(|v| *v += 1);
            }
        });
    }
    provide_context(NonLus {
        total: total_non_lus,
        rafraichir: Callback::new(move |_| version_non_lus.update(|v| *v += 1)),
    });
    let entrees = Signal::derive(move || {
        navigation::avec_compteur_messages(navigation::entrees(droits.get()), total_non_lus.get())
    });
    let courant = Signal::derive(move || location.pathname.get());
    let liens_pied = Signal::derive(navigation::liens_compte);
    let rail = Signal::derive(move || navigation::ecran_de_saisie(&location.pathname.get()));
    let identite = Signal::derive(move || {
        moi.get().and_then(Result::ok).map(|m| Identite {
            nom: m.email.clone(),
            detail: libelle_role(&m.role).to_string(),
        })
    });
    let etablissement = Signal::derive(move || match config.get().and_then(Result::ok) {
        Some(c) if c.labo => "Environnement de test".to_string(),
        _ => String::new(),
    });

    let en_deconnexion = RwSignal::new(false);
    let deconnecter = Callback::new(move |_| {
        if en_deconnexion.get_untracked() {
            return;
        }
        en_deconnexion.set(true);
        session::deconnecter(navigate.clone());
    });

    // Pendant la révocation, toute l'interface est démontée : aucune donnée de la
    // personne ni aucun bouton ne reste dans la page (la CSP interdit les styles en
    // ligne, donc pas de simple recouvrement).
    view! {
        {move || {
            if en_deconnexion.get() {
                view! {
                    <main id="contenu" class="connexion" tabindex="-1">
                        <p class="connexion__aide" role="status">"Déconnexion…"</p>
                    </main>
                }
                    .into_any()
            } else {
                view! {
                    <Coquille
                        entrees=entrees
                        courant=courant
                        etablissement=etablissement
                        identite=identite
                        rail=rail
                        liens_pied=liens_pied
                        on_deconnexion=deconnecter
                    >
                        <div id="annonce-page" class="ui-sr" role="status" aria-live="polite" aria-atomic="true"></div>
                        <BandeauExpiration ctx=reco/>
                        // Session « enrôlement » : aucune page métier ne se charge (redirection en cours).
                        {move || {
                            if en_enrolement.get() {
                                view! { <p role="status">"Redirection vers l'enregistrement de votre passkey…"</p> }
                                    .into_any()
                            } else {
                                view! { <Outlet/> }.into_any()
                            }
                        }}
                    </Coquille>
                    <ReconnexionRapide ctx=reco securite=securite/>
                }
                    .into_any()
            }
        }}
    }
}

/// Après un changement de page (navigateur uniquement) : donne le focus à `<main>` et
/// annonce le titre de la page dans la région `aria-live` polie `#annonce-page` si elle
/// existe. Le focus n'est pas volé si l'utilisateur est déjà dans la page (par ex. un
/// champ de l'appel qui l'a pris volontairement).
#[cfg(target_arch = "wasm32")]
fn apres_navigation() {
    use leptos::wasm_bindgen::JsCast;
    // Laisse `leptos_meta` mettre à jour `document.title` et la page se dessiner.
    set_timeout(
        move || {
            let doc = document();
            if let Some(main) = doc
                .get_element_by_id("contenu")
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let deja_dans_la_page = doc
                    .active_element()
                    .is_some_and(|a| a.id() != "contenu" && main.contains(Some(&a)));
                if !deja_dans_la_page {
                    let _ = main.focus();
                }
            }
            if let Some(zone) = doc.get_element_by_id("annonce-page") {
                zone.set_text_content(Some(&doc.title()));
            }
        },
        std::time::Duration::from_millis(80),
    );
}
