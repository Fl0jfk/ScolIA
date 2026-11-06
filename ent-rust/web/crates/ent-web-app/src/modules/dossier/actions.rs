//! Actions métier de la fiche : ajouter un responsable (avec ou sans e-mail), définir
//! les payeurs, ajouter / proposer / valider / retirer un contact d'urgence, suspendre
//! ou rétablir un lien. Chaque écriture : erreurs affichées dans le dialogue, arrêt sur
//! 401 (reconnexion par-dessus la page), rechargement de la fiche après succès.

use ent_web_api::{ApiError, ContactUrgenceDto, ResponsableDto, ResultatLien};
use ent_web_ui::{
    use_notifications, Alerte, Badge, Bouton, Case, ChampTexte, Confirmation, Dialogue, Niveau,
    Selection, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use super::modele::{
    contact_depuis, nom_personne, parts_payeurs, valider_motif, SaisieResponsable,
};
use crate::noyau::{reconnexion, session};

/// Message d'échec d'une écriture ; une session perdue ouvre la reconnexion.
fn message_echec(e: ApiError) -> String {
    match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            reconnexion::MESSAGE_SESSION_EXPIREE.into()
        }
        autre => autre.message_utilisateur(),
    }
}

#[component]
pub fn DialogueResponsable(
    ouvert: RwSignal<bool>,
    eleve_id: Uuid,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let nom = RwSignal::new(String::new());
    let prenom = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let telephone = RwSignal::new(String::new());
    let lien = RwSignal::new(String::new());
    let garde = RwSignal::new(String::new());
    let autorite = RwSignal::new(true);
    let voir_notes = RwSignal::new(true);
    let voir_vs = RwSignal::new(true);
    let justifier = RwSignal::new(true);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let envoyer = Callback::new(move |_| {
        let saisie = SaisieResponsable {
            nom: nom.get_untracked(),
            prenom: prenom.get_untracked(),
            email: email.get_untracked(),
            telephone: telephone.get_untracked(),
            lien: lien.get_untracked(),
            garde: garde.get_untracked(),
            autorite: autorite.get_untracked(),
            voir_notes: voir_notes.get_untracked(),
            voir_vs: voir_vs.get_untracked(),
            justifier: justifier.get_untracked(),
        };
        let ajout = match saisie.ajout() {
            Ok(a) => a,
            Err(m) => return erreur.set(Some(m)),
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let client = session::client();
            // Une seule commande : la personne et son lien partent (ou échouent) ensemble.
            let r = client.ajouter_responsable(eleve_id, &ajout).await;
            en_cours.set(false);
            match r {
                Ok(r) => {
                    notifications.succes(match r {
                        ResultatLien::Actif(r) => {
                            format!("{} est lié(e) à l'élève.", nom_personne(&r.personne))
                        }
                        ResultatLien::EnAttente(_) => "Invitation enregistrée : le lien sera actif \
                            quand la personne l'aura confirmé."
                            .to_string(),
                    });
                    for s in [nom, prenom, email, telephone, lien, garde] {
                        s.set(String::new());
                    }
                    ouvert.set(false);
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    view! {
        <Dialogue ouvert=ouvert titre="Ajouter un responsable" id="fiche-responsable">
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); envoyer.run(()); }>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <ChampTexte id="resp-nom" libelle="Nom" valeur=nom requis=true autocomplete="off"/>
                <ChampTexte id="resp-prenom" libelle="Prénom" valeur=prenom autocomplete="off"/>
                <ChampTexte id="resp-lien" libelle="Lien avec l'élève" valeur=lien requis=true aide="Mère, père, tuteur, grand-parent…"/>
                <ChampTexte id="resp-email" libelle="Adresse e-mail" valeur=email genre="email"
                    aide="Facultative : un parent sans e-mail figure sur la fiche, sans compte. Parents séparés : une adresse par parent."/>
                <ChampTexte id="resp-tel" libelle="Téléphone" valeur=telephone genre="tel" inputmode="tel"/>
                <Selection id="resp-garde" libelle="Garde" valeur=garde options=vec![
                    (String::new(), "Non renseignée".into()),
                    ("exclusive".into(), "Exclusive".into()),
                    ("alternee".into(), "Alternée".into()),
                    ("aucune".into(), "Sans garde".into()),
                ]/>
                <fieldset class="pile">
                    <legend>"Autorité et droits"</legend>
                    <Case id="resp-autorite" libelle="Exerce l'autorité parentale" coche=autorite/>
                    <Case id="resp-notes" libelle="Voit les notes" coche=voir_notes/>
                    <Case id="resp-vs" libelle="Voit la vie scolaire" coche=voir_vs/>
                    <Case id="resp-justifier" libelle="Peut justifier les absences" coche=justifier/>
                </fieldset>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
pub fn DialoguePayeurs(
    ouvert: RwSignal<bool>,
    eleve_id: Uuid,
    responsables: StoredValue<Vec<ResponsableDto>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    // Une saisie par lien, préremplie avec la part actuelle.
    let saisies: Vec<(Uuid, String, RwSignal<String>, bool)> = responsables.with_value(|l| {
        l.iter()
            .map(|r| {
                (
                    r.lien_id,
                    nom_personne(&r.personne),
                    RwSignal::new(
                        r.part_paiement
                            .filter(|_| r.est_payeur)
                            .map(|p| p.to_string())
                            .unwrap_or_default(),
                    ),
                    r.foyer_id.is_some(),
                )
            })
            .collect()
    });
    let valeurs = StoredValue::new(
        saisies
            .iter()
            .map(|(id, _, s, _)| (*id, *s))
            .collect::<Vec<_>>(),
    );
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let envoyer = Callback::new(move |_| {
        let parts: Vec<(Uuid, String)> =
            valeurs.with_value(|v| v.iter().map(|(id, s)| (*id, s.get_untracked())).collect());
        let parts = match parts_payeurs(&parts) {
            Ok(p) => p,
            Err(m) => return erreur.set(Some(m)),
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().definir_payeurs(eleve_id, parts).await;
            en_cours.set(false);
            match r {
                Ok(_) => {
                    notifications.succes("Payeurs enregistrés.");
                    ouvert.set(false);
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    view! {
        <Dialogue ouvert=ouvert titre="Payeurs et parts" id="fiche-payeurs">
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); envoyer.run(()); }>
                <p class="texte-attenue">"Indiquez la part de chaque payeur en pourcentage (total 100 %). Laissez vide pour un non-payeur. Un payeur doit être rattaché à un foyer."</p>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                {saisies.into_iter().map(|(id, nom, s, foyer)| {
                    let aide = (!foyer).then(|| "Sans foyer : ne peut pas être payeur.".to_string());
                    view! {
                        <ChampTexte id=format!("part-{id}") libelle=format!("Part de {nom} (%)") valeur=s inputmode="numeric" aide=aide.unwrap_or_default()/>
                    }
                }).collect_view()}
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
pub fn DialogueContact(
    ouvert: RwSignal<bool>,
    eleve_id: Uuid,
    proposition: bool,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let nom = RwSignal::new(String::new());
    let prenom = RwSignal::new(String::new());
    let telephone = RwSignal::new(String::new());
    let relation = RwSignal::new(String::new());
    let recuperer = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let envoyer = Callback::new(move |_| {
        let corps = match contact_depuis(
            &nom.get_untracked(),
            &prenom.get_untracked(),
            &telephone.get_untracked(),
            &relation.get_untracked(),
            recuperer.get_untracked(),
        ) {
            Ok(c) => c,
            Err(m) => return erreur.set(Some(m)),
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().ajouter_contact(eleve_id, &corps).await;
            en_cours.set(false);
            match r {
                Ok(c) => {
                    notifications.succes(if c.statut == "propose" {
                        "Contact proposé : l'établissement le validera."
                    } else {
                        "Contact d'urgence ajouté."
                    });
                    for s in [nom, prenom, telephone, relation] {
                        s.set(String::new());
                    }
                    ouvert.set(false);
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    let titre = if proposition {
        "Proposer un contact d'urgence"
    } else {
        "Ajouter un contact d'urgence"
    };
    view! {
        <Dialogue ouvert=ouvert titre=titre id="fiche-contact">
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); envoyer.run(()); }>
                {proposition.then(|| view! { <p class="texte-attenue">"Votre proposition sera vérifiée par l'établissement avant d'être utilisée."</p> })}
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <ChampTexte id="contact-nom" libelle="Nom" valeur=nom requis=true autocomplete="off"/>
                <ChampTexte id="contact-prenom" libelle="Prénom" valeur=prenom autocomplete="off"/>
                <ChampTexte id="contact-tel" libelle="Téléphone" valeur=telephone genre="tel" inputmode="tel" requis=true/>
                <ChampTexte id="contact-relation" libelle="Relation avec l'élève" valeur=relation requis=true aide="Grand-parent, voisin, assistante maternelle…"/>
                <Case id="contact-recuperer" libelle="Autorisé(e) à récupérer l'élève" coche=recuperer/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
pub fn ListeContacts(
    contacts: Vec<ContactUrgenceDto>,
    gerer: bool,
    recharger: Callback<()>,
) -> impl IntoView {
    if contacts.is_empty() {
        return view! { <p class="texte-attenue">"Aucun contact d'urgence."</p> }.into_any();
    }
    let notifications = use_notifications();
    let erreur = RwSignal::new(None::<String>);
    let a_retirer = RwSignal::new(None::<(Uuid, String)>);
    let confirmation = RwSignal::new(false);
    Effect::new(move |_| confirmation.set(a_retirer.get().is_some()));
    let valider = move |id: Uuid| {
        spawn_local(async move {
            match session::client().valider_contact(id).await {
                Ok(()) => {
                    notifications.succes("Contact validé.");
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    };
    let retirer = Callback::new(move |_| {
        let Some((id, _)) = a_retirer.get_untracked() else {
            return;
        };
        a_retirer.set(None);
        spawn_local(async move {
            match session::client().retirer_contact(id).await {
                Ok(()) => {
                    notifications.succes("Contact retiré.");
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    view! {
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        <ul class="ui-liste">
            {contacts.into_iter().map(|c| {
                let nom = nom_personne(&c.personne);
                let tel = c.personne.telephone.clone().unwrap_or_default();
                let mut detail = format!("{} · {}", c.relation, tel);
                if c.peut_recuperer {
                    detail.push_str(" · peut récupérer l'élève");
                }
                let propose = c.statut == "propose";
                let (id, pour) = (c.id, nom.clone());
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal">
                            <span class="ui-liste__titre">{nom}</span>
                            <span class="ui-liste__detail">{detail}</span>
                        </div>
                        {propose.then(|| view! { <Badge libelle="Proposé" ton=TonEtat::Avertissement/> })}
                        {(gerer && propose).then(|| view! {
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                libelle_accessible=format!("Valider le contact {pour}")
                                on_click=Callback::new(move |_| valider(id))>"Valider"</Bouton>
                        })}
                        {gerer.then(|| {
                            let pour = pour.clone();
                            view! {
                                <Bouton variante=Variante::Discret taille=Taille::Compacte
                                    libelle_accessible=format!("Retirer le contact {pour}")
                                    on_click=Callback::new(move |_| a_retirer.set(Some((id, pour.clone()))))>"Retirer"</Bouton>
                            }
                        })}
                    </li>
                }
            }).collect_view()}
        </ul>
        <Confirmation
            ouvert=confirmation
            id="fiche-contact-retrait"
            titre="Retirer ce contact ?"
            message="Le contact n'apparaîtra plus sur la fiche de l'élève. La personne reste connue de l'établissement."
            libelle_confirmer="Retirer"
            danger=true
            on_confirmer=retirer
        />
    }
    .into_any()
}

#[component]
pub fn DialogueSuspension(
    lien: RwSignal<Option<(Uuid, String)>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let ouvert = RwSignal::new(false);
    Effect::new(move |_| ouvert.set(lien.get().is_some()));
    Effect::new(move |_| {
        if !ouvert.get() {
            lien.set(None);
        }
    });
    let motif = RwSignal::new(String::new());
    let reference = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let envoyer = Callback::new(move |_| {
        let Some((id, _)) = lien.get_untracked() else {
            return;
        };
        let m = match valider_motif(&motif.get_untracked()) {
            Ok(m) => m,
            Err(e) => return erreur.set(Some(e)),
        };
        let r = Some(reference.get_untracked()).filter(|r| !r.trim().is_empty());
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let res = session::client().suspendre_lien(id, m, r).await;
            en_cours.set(false);
            match res {
                Ok(_) => {
                    notifications
                        .succes("Lien suspendu : les accès de ce responsable sont coupés.");
                    motif.set(String::new());
                    reference.set(String::new());
                    ouvert.set(false);
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    view! {
        <Dialogue ouvert=ouvert titre="Suspendre le lien d'un responsable" id="fiche-suspension">
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); envoyer.run(()); }>
                <p>{move || lien.get().map(|(_, n)| format!(
                    "{n} n'aura plus accès au dossier, aux notes, à la vie scolaire ni aux messages concernant cet élève, dès la confirmation."
                ))}</p>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <ZoneTexte id="suspension-motif" libelle="Motif (décision de justice…)" valeur=motif requis=true
                    aide="Visible du seul personnel habilité ; jamais transmis aux familles."/>
                <ChampTexte id="suspension-ref" libelle="Référence de la décision" valeur=reference autocomplete="off"/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton variante=Variante::Danger soumettre=true en_cours=en_cours>"Suspendre le lien"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
pub fn DialogueRetablir(
    lien: RwSignal<Option<(Uuid, String)>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let ouvert = RwSignal::new(false);
    Effect::new(move |_| ouvert.set(lien.get().is_some()));
    let erreur = RwSignal::new(None::<String>);
    let confirmer = Callback::new(move |_| {
        let Some((id, _)) = lien.get_untracked() else {
            return;
        };
        lien.set(None);
        spawn_local(async move {
            match session::client().retablir_lien(id).await {
                Ok(_) => {
                    notifications.succes("Lien rétabli.");
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    view! {
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        <Confirmation
            ouvert=ouvert
            id="fiche-retablir"
            titre="Rétablir ce lien ?"
            message="Le responsable retrouvera immédiatement les accès que lui donnent ses droits sur ce lien."
            libelle_confirmer="Rétablir"
            on_confirmer=confirmer
        />
    }
}
