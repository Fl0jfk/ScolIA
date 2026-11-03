//! Page de démonstration du design system (`/web/_composants`) : chaque composant
//! dans ses états, pour la relecture visuelle et les tests d'accessibilité.
//! Aucune donnée, aucun appel réseau.

use ent_web_ui::*;
use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn PageComposants() -> impl IntoView {
    let notifications = use_notifications();
    let confirmation = RwSignal::new(false);
    let confirme = RwSignal::new(0u32);
    let en_cours = RwSignal::new(false);

    let nom = RwSignal::new(String::new());
    let email = RwSignal::new("adresse-invalide".to_string());
    let commentaire = RwSignal::new(String::new());
    let classe = RwSignal::new("4B".to_string());
    let rappel = RwSignal::new(true);
    let statut = RwSignal::new("present".to_string());
    let erreur_email = Signal::derive(move || {
        (!email.get().contains('@'))
            .then(|| "Saisissez une adresse e-mail valide (exemple : nom@domaine.fr).".to_string())
    });
    let erreurs = Signal::derive(move || {
        erreur_email
            .get()
            .map(|e| vec![("demo-email".to_string(), e)])
            .unwrap_or_default()
    });
    let alerte_visible = RwSignal::new(true);

    view! {
        <Title text="Composants"/>
        <a class="lien-evitement" href="#contenu">"Aller au contenu"</a>
        <main id="contenu" class="page page--large">
            <div class="page__entete">
                <h1>"Design system"</h1>
                <p class="page__soustitre">"Composants communs d'ent-web-ui, dans leurs différents états."</p>
            </div>

            <section class="carte pile" aria-labelledby="t-boutons">
                <h2 id="t-boutons">"Boutons"</h2>
                <div class="grille-demo">
                    <Bouton>"Primaire"</Bouton>
                    <Bouton variante=Variante::Secondaire>"Secondaire"</Bouton>
                    <Bouton variante=Variante::Discret>"Discret"</Bouton>
                    <Bouton variante=Variante::Danger>"Danger"</Bouton>
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire>"Compact"</Bouton>
                    <Bouton desactive=true>"Désactivé"</Bouton>
                    <Bouton
                        en_cours=Signal::derive(move || en_cours.get())
                        on_click=Callback::new(move |_| en_cours.set(true))
                    >
                        {move || if en_cours.get() { "Enregistrement…" } else { "Lancer (état en cours)" }}
                    </Bouton>
                    <Bouton variante=Variante::Discret on_click=Callback::new(move |_| en_cours.set(false))>
                        "Réinitialiser"
                    </Bouton>
                    <LienBouton href="/web/appel" variante=Variante::Secondaire>"Lien vers l'appel"</LienBouton>
                </div>
            </section>

            <section class="carte pile" aria-labelledby="t-champs">
                <h2 id="t-champs">"Champs"</h2>
                <ResumeErreurs erreurs=erreurs/>
                <ChampTexte id="demo-nom" libelle="Nom" valeur=nom requis=true aide="Tel qu'il figure sur les documents officiels." autocomplete="family-name"/>
                <ChampTexte id="demo-email" libelle="Adresse e-mail" valeur=email genre="email" erreur=erreur_email autocomplete="email"/>
                <ZoneTexte id="demo-commentaire" libelle="Commentaire" valeur=commentaire aide="Facultatif."/>
                <Selection id="demo-classe" libelle="Classe" valeur=classe
                    options=vec![("4A".into(), "4A".into()), ("4B".into(), "4B".into()), ("5A".into(), "5A".into())]/>
                <Case id="demo-rappel" libelle="M'envoyer un rappel par e-mail" coche=rappel/>
                <GroupeChoix nom="demo-statut" legende="Statut" valeur=statut
                    options=vec![("present".into(), "Présent".into()), ("absent".into(), "Absent".into()), ("retard".into(), "Retard".into())]/>
                <ChampTexte id="demo-desactive" libelle="Champ désactivé" valeur=RwSignal::new("Valeur figée".to_string()) desactive=true/>
            </section>

            <section class="carte pile" aria-labelledby="t-messages">
                <h2 id="t-messages">"Messages"</h2>
                <Alerte niveau=Niveau::Info titre="Information"><p>"Message d'information."</p></Alerte>
                <Alerte niveau=Niveau::Avertissement titre="Avertissement"/>
                <Alerte niveau=Niveau::Erreur titre="Erreur bloquante"><p>"Ce message est annoncé immédiatement."</p></Alerte>
                <Show when=move || alerte_visible.get()>
                    <Alerte niveau=Niveau::Succes titre="Succès (fermable)" on_fermer=Callback::new(move |_| alerte_visible.set(false))/>
                </Show>
            </section>

            <section class="carte pile" aria-labelledby="t-dialogues">
                <h2 id="t-dialogues">"Dialogues et notifications"</h2>
                <div class="grille-demo">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmation.set(true))>
                        "Ouvrir une confirmation"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| { notifications.succes("Modifications enregistrées."); })>
                        "Notification de succès"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| { notifications.info("Une nouvelle version des données est disponible."); })>
                        "Notification d'information"
                    </Bouton>
                </div>
                <p id="demo-confirmations">{move || format!("Confirmations reçues : {}", confirme.get())}</p>
                <Confirmation
                    ouvert=confirmation
                    id="demo-confirmation"
                    titre="Supprimer le brouillon ?"
                    message="Le brouillon sera définitivement supprimé."
                    libelle_confirmer="Supprimer"
                    danger=true
                    on_confirmer=Callback::new(move |_| confirme.update(|c| *c += 1))
                />
            </section>

            <section class="carte pile" aria-labelledby="t-etats">
                <h2 id="t-etats">"États"</h2>
                <Chargement libelle="Chargement des élèves…"/>
                <Squelette lignes=2 libelle="Chargement de la liste…"/>
                <EtatVide titre="Aucun résultat" message="Modifiez la recherche ou le filtre."/>
                <EtatErreur titre="Impossible de charger les données" message="Le serveur ne répond pas." on_reessayer=Callback::new(|_| ())/>
                <AccesRefuse/>
            </section>
        </main>
    }
}
