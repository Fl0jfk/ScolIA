//! Reconnexion rapide (D-011) : alerte avant l'expiration de la session et fenêtre de
//! reconnexion **par-dessus la page** quand elle a expiré — la page, ses saisies et sa
//! position sont conservées.
//!
//! - [`ContexteReconnexion`] : échéances estimées (voir [`super::expiration`]), état de la
//!   fenêtre et compteur de reprise (`version`), fournis par la coquille.
//! - [`signaler_session_perdue`] : à appeler quand un appel renvoie 401 **après** le chargement
//!   de la page. Ouvre la fenêtre au lieu de rediriger vers la page de connexion.
//! - [`BandeauExpiration`] et [`ReconnexionRapide`] : les deux vues.
//!
//! Le mot de passe saisi ici part vers `/web/session/connexion` (le relais pose le cookie) ;
//! rien n'est conservé dans la page.

use std::cell::Cell;

use ent_web_api::{ApiError, Connexion, MoiSecurite, Session};
use ent_web_ui::{Alerte, Bouton, ChampTexte, Niveau, Variante};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};

use super::expiration::{libelle_reste, Echeances, Phase};
use super::session::{self, Message};
use super::webauthn;

/// Intervalle du contrôle des échéances (navigateur).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const BATTEMENT: std::time::Duration = std::time::Duration::from_secs(15);

/// Au plus un message d'activité aux autres onglets par intervalle (secondes).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const DIFFUSION_ACTIVITE_S: i64 = 30;

#[derive(Clone, Copy)]
pub struct ContexteReconnexion {
    /// Fenêtre de reconnexion ouverte.
    pub ouverte: RwSignal<bool>,
    /// Incrémenté après une reconnexion réussie : les chargements en échec se relancent.
    pub version: RwSignal<u32>,
    /// Échéances estimées ; `None` tant que `/web/session/etat` n'a pas répondu (ou refusé,
    /// ex. session d'enrôlement).
    pub echeances: RwSignal<Option<Echeances>>,
    /// Horloge (secondes Unix), mise à jour par le contrôle périodique.
    pub maintenant: RwSignal<i64>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    activite_diffusee: RwSignal<i64>,
}

thread_local! {
    static COURANT: Cell<Option<ContexteReconnexion>> = const { Cell::new(None) };
}

fn maintenant() -> i64 {
    chrono::Utc::now().timestamp()
}

impl ContexteReconnexion {
    /// Phase de la session selon les échéances connues (suit les signaux).
    pub fn phase(&self) -> Option<Phase> {
        let now = self.maintenant.get();
        self.echeances.get().map(|e| e.phase(now))
    }

    /// Relit le temps restant au serveur (compte comme une activité : « Rester connecté »).
    /// 401 → la session est déjà perdue : fenêtre de reconnexion.
    pub fn rafraichir(self) {
        spawn_local(async move {
            match session::client().etat_session().await {
                Ok(e) => {
                    let now = maintenant();
                    self.echeances.set(Some(Echeances::depuis_etat(
                        now,
                        e.expire_dans,
                        e.inactivite_max,
                    )));
                    self.maintenant.set(now);
                }
                Err(ApiError::NonConnecte) => self.ouverte.set(true),
                // 403 (session d'enrôlement) ou réseau : pas d'échéance connue, rien à afficher.
                Err(_) => {}
            }
        });
    }

    /// Contrôle périodique : horloge, activité vue ailleurs, ouverture de la fenêtre à l'échéance.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn battement(self) {
        let now = maintenant();
        self.maintenant.set(now);
        let activite = ent_web_api::derniere_activite();
        self.echeances
            .update(|e| *e = e.map(|x| x.avec_activite(activite)));
        if activite > self.activite_diffusee.get_untracked() + DIFFUSION_ACTIVITE_S {
            self.activite_diffusee.set(activite);
            session::annoncer(Message::Activite(activite));
        }
        if matches!(
            self.echeances.get_untracked().map(|e| e.phase(now)),
            Some(Phase::Expiree)
        ) && !self.ouverte.get_untracked()
        {
            self.ouverte.set(true);
        }
    }
}

/// Crée et fournit le contexte (une fois, par la coquille).
pub fn fournir() -> ContexteReconnexion {
    let ctx = ContexteReconnexion {
        ouverte: RwSignal::new(false),
        version: RwSignal::new(0),
        echeances: RwSignal::new(None),
        maintenant: RwSignal::new(0),
        activite_diffusee: RwSignal::new(0),
    };
    COURANT.with(|c| c.set(Some(ctx)));
    on_cleanup(|| COURANT.with(|c| c.set(None)));
    provide_context(ctx);

    #[cfg(target_arch = "wasm32")]
    {
        ctx.maintenant.set(maintenant());
        if let Ok(h) = set_interval_with_handle(move || ctx.battement(), BATTEMENT) {
            on_cleanup(move || h.clear());
        }
    }
    ctx
}

pub fn use_reconnexion() -> Option<ContexteReconnexion> {
    use_context::<ContexteReconnexion>()
}

/// Un appel a renvoyé 401 alors que la page est déjà affichée : ouvre la fenêtre de
/// reconnexion (sans changer de page). Sans effet hors de la coquille authentifiée.
pub fn signaler_session_perdue() {
    COURANT.with(|c| {
        if let Some(ctx) = c.get() {
            ctx.ouverte.set(true);
        }
    });
}

/// Avertissement « votre session expire dans X min ».
#[component]
pub fn BandeauExpiration(ctx: ContexteReconnexion) -> impl IntoView {
    view! {
        {move || match ctx.phase() {
            Some(Phase::Avertir { reste }) if !ctx.ouverte.get() => {
                let titre = format!(
                    "Votre session expire dans {} faute d'activité.",
                    libelle_reste(reste)
                );
                view! {
                    <Alerte niveau=Niveau::Avertissement titre=titre>
                        <p>
                            <Bouton
                                variante=Variante::Secondaire
                                on_click=Callback::new(move |_| ctx.rafraichir())
                            >
                                "Rester connecté"
                            </Bouton>
                        </p>
                    </Alerte>
                }
                    .into_any()
            }
            _ => ().into_any(),
        }}
    }
}

/// Fenêtre de reconnexion par-dessus la page.
#[component]
pub fn ReconnexionRapide(
    ctx: ContexteReconnexion,
    securite: RwSignal<Option<MoiSecurite>>,
) -> impl IntoView {
    let navigate = use_navigate();
    let location = use_location();
    let noeud = NodeRef::<leptos::html::Dialog>::new();
    let mot_de_passe = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);

    // Ouverture / fermeture de la boîte modale native (navigateur seulement).
    Effect::new(move |_| {
        let veut = ctx.ouverte.get();
        if let Some(d) = noeud.get() {
            let el: &web_sys::HtmlDialogElement = &d;
            if veut && !el.open() {
                let _ = el.show_modal();
            } else if !veut && el.open() {
                el.close();
            }
        }
    });

    let compte = move || {
        securite.with(|s| {
            s.as_ref()
                .map(|m| (m.user_id, m.email.clone(), m.etablissement_id))
        })
    };

    // Suite d'une connexion réussie : même personne → on reste sur la page ; autre compte
    // → rechargement (identité, droits et données ne doivent pas se mélanger).
    let terminer = move |s: Session| {
        mot_de_passe.set(String::new());
        erreur.set(None);
        session::annoncer(Message::Ouverte(s.user_id));
        if compte().is_some_and(|(id, _, _)| id == s.user_id) {
            ctx.ouverte.set(false);
            ctx.version.update(|v| *v += 1);
            ctx.rafraichir();
        } else {
            #[cfg(target_arch = "wasm32")]
            let _ = window().location().reload();
        }
    };

    let echec = move |e: ApiError| {
        en_cours.set(false);
        erreur.set(Some(match e {
            ApiError::NonConnecte | ApiError::Interdit(_) => {
                "Identifiants refusés. Vérifiez votre mot de passe.".to_string()
            }
            autre => autre.message_utilisateur(),
        }));
    };

    // Signature du défi par l'appareil, puis ouverture de la session.
    let finir_passkey = move |defi: ent_web_api::PasskeyChallengeDto| {
        spawn_local(async move {
            let resultat = match webauthn::signer_defi(&defi.options).await {
                Ok(assertion) => {
                    session::client()
                        .connexion_passkey_fin(&defi.token, assertion)
                        .await
                }
                Err(m) => {
                    en_cours.set(false);
                    erreur.set(Some(format!("Passkey : {m}.")));
                    return;
                }
            };
            en_cours.set(false);
            match resultat {
                Ok(s) => terminer(s),
                Err(e) => echec(e),
            }
        });
    };

    let soumettre = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let Some((_, email, etab)) = compte() else {
            return;
        };
        erreur.set(None);
        en_cours.set(true);
        let mdp = mot_de_passe.get_untracked();
        spawn_local(async move {
            match session::client().connexion(&email, &mdp, etab).await {
                Ok(Connexion::Ouverte(s)) => {
                    en_cours.set(false);
                    terminer(s)
                }
                Ok(Connexion::PasskeyRequise(defi)) => finir_passkey(*defi),
                Err(e) => echec(e),
            }
        });
    };

    let par_passkey = Callback::new(move |_| {
        if en_cours.get_untracked() {
            return;
        }
        let Some((_, email, etab)) = compte() else {
            return;
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            match session::client()
                .connexion_passkey_debut(&email, etab)
                .await
            {
                Ok(defi) => finir_passkey(defi),
                Err(e) => echec(e),
            }
        });
    });

    let quitter = Callback::new(move |_| {
        let chemin = session::chemin_complet(
            &location.pathname.get_untracked(),
            &location.search.get_untracked(),
        );
        ctx.ouverte.set(false);
        navigate(&session::adresse_connexion(&chemin), Default::default());
    });

    view! {
        <dialog
            node_ref=noeud
            id="reconnexion"
            class="ui-dialogue"
            aria-labelledby="reconnexion-titre"
            // Pas de fermeture par Échap : la page n'est plus utilisable sans session.
            on:cancel=move |ev: leptos::ev::Event| ev.prevent_default()
        >
            <h2 class="ui-dialogue__titre" id="reconnexion-titre">"Votre session a expiré"</h2>
            <p>
                "Reconnectez-vous pour continuer. Vous restez sur cette page : "
                "ce que vous avez saisi n'est pas perdu."
            </p>
            <form on:submit=soumettre novalidate>
                <p>
                    <strong>{move || compte().map(|(_, e, _)| e).unwrap_or_default()}</strong>
                </p>
                <ChampTexte
                    id="reconnexion-mdp"
                    libelle="Mot de passe"
                    valeur=mot_de_passe
                    genre="password"
                    autocomplete="current-password"
                />
                {move || {
                    erreur
                        .get()
                        .map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })
                }}
                <p class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=quitter>
                        "Quitter"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire on_click=par_passkey>
                        "Utiliser ma passkey"
                    </Bouton>
                    <Bouton soumettre=true en_cours=Signal::derive(move || en_cours.get())>
                        "Se reconnecter"
                    </Bouton>
                </p>
            </form>
        </dialog>
    }
}

/// Message affiché là où une action a échoué faute de session (la fenêtre de reconnexion
/// s'ouvre en même temps ; l'action n'est pas rejouée automatiquement).
pub const MESSAGE_SESSION_EXPIREE: &str =
    "Votre session a expiré. Reconnectez-vous dans la fenêtre ouverte, puis recommencez.";
