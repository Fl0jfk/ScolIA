//! Application Leptos de l'ENT — tranche 1 : parcours enseignant (connexion, accueil,
//! emploi du temps, appel). Architecture : `docs/FRONTEND_CIBLE.md` §3.
//!
//! Rendu : le serveur produit le HTML (coquille, en-têtes, états de chargement), puis
//! le navigateur l'**hydrate** avec le WebAssembly. Les données métier sont chargées
//! côté navigateur avec la session (voir `noyau/session.rs`).

// Les vues Leptos produisent des types très imbriqués : sans effacement des
// composants (`erase_components`), la limite par défaut (128) est dépassée.
#![recursion_limit = "512"]

mod composants;
pub mod composants_metier;
pub mod modules;
pub mod noyau;

// Chemins historiques (tests contre le backend, serveur).
pub use modules::appel;
pub use modules::connexion::{config_connexion, ParametresConnexion};
pub use noyau::session;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{ParentRoute, Route, Router, Routes},
    path,
};

/// Document HTML complet rendu par le serveur.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="fr">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
                <meta name="color-scheme" content="light"/>
                <meta name="theme-color" content="#f5f5f7"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    ent_web_ui::fournir_notifications();
    // À la racine : uniquement les balises leptos_meta et <Router> (voir FRONTEND_LEPTOS.md §8).
    view! {
        <Stylesheet id="ent-web" href="/pkg/ent-web.css"/>
        <Title formatter=|titre: String| format!("{titre} · ScolIA")/>
        <Router>
            <Routes fallback=PageIntrouvable>
                <Route path=path!("/web/connexion") view=modules::connexion::PageConnexion/>
                <Route path=path!("/web/securite/enrolement") view=modules::securite::PageEnrolement/>
                <Route path=path!("/web/recuperation-passkey") view=modules::recuperation::PageDemandeRecuperation/>
                // Liens des e-mails de récupération (D-018) : adresses telles que les envoie `ent-api`.
                <Route path=path!("/recuperation-passkey/confirmer") view=modules::recuperation::PageConfirmerRecuperation/>
                <Route path=path!("/recuperation-passkey/annuler") view=modules::recuperation::PageAnnulerRecuperation/>
                <Route path=path!("/recuperation-passkey/finaliser") view=modules::recuperation::PageFinaliserRecuperation/>
                <Route path=path!("/web/_composants") view=composants::PageComposants/>
                // Parcours famille sans compte (S2-3) : publics, hors coquille.
                <Route path=path!("/web/preinscription") view=modules::inscriptions::PagePreinscription/>
                <Route path=path!("/web/invitation") view=modules::inscriptions::PageInvitation/>
                <ParentRoute path=path!("/web") view=noyau::coquille::AppCoquille>
                    <Route path=path!("") view=modules::accueil::PageAccueil/>
                    <Route path=path!("mes-liens") view=modules::liens_en_attente::PageMesLiens/>
                    <Route path=path!("emploi-du-temps") view=modules::edt::PageEmploiDuTemps/>
                    <Route path=path!("edt/semaine") view=modules::edt_semaine::PageSemaineEdt/>
                    <Route path=path!("edt/remplacements") view=modules::remplacements::PageRemplacements/>
                    <Route path=path!("changements-edt") view=modules::changements_edt::PageChangementsEdt/>
                    <Route path=path!("delegations") view=modules::delegations::PageDelegations/>
                    <Route path=path!("appel") view=modules::appel::PageAppel/>
                    <Route path=path!("classes") view=modules::classes::PageClasses/>
                    <Route path=path!("classes/:id") view=modules::classes::PageClasse/>
                    <Route path=path!("mes-classes") view=modules::classes::PageMesClasses/>
                    <Route path=path!("vie-scolaire") view=modules::vie_scolaire::PageVieScolaire/>
                    <Route path=path!("journee") view=modules::journee_vs::PageJourneeVs/>
                    <Route path=path!("absences") view=modules::absences::PageAbsences/>
                    <Route path=path!("accompagnement") view=modules::accompagnement::PageAccompagnement/>
                    <Route path=path!("accompagnement/eleves/:id") view=modules::accompagnement::PageAccompagnementEleve/>
                    <Route path=path!("amenagements") view=modules::amenagements::PageAmenagements/>
                    <Route path=path!("sante-enfant") view=modules::sante_famille::PageSanteEnfant/>
                    <Route path=path!("infirmerie") view=modules::infirmerie::PageInfirmerie/>
                    <Route path=path!("infirmerie/eleves/:id") view=modules::infirmerie::PageInfirmerieEleve/>
                    <Route path=path!("infirmerie/eleves/:id/urgence") view=modules::infirmerie::PageFicheUrgence/>
                    <Route path=path!("eleves/:id") view=modules::dossier::PageFicheEleve/>
                    <Route path=path!("inscriptions") view=modules::inscriptions::PageInscriptions/>
                    <Route path=path!("reinscription") view=modules::inscriptions::PageReinscription/>
                    <Route path=path!("cahier") view=modules::cahier::PageCahier/>
                    <Route path=path!("cahier/travaux/:id") view=modules::cahier::PageCopies/>
                    <Route path=path!("agenda") view=modules::cahier::PageAgenda/>
                    <Route path=path!("voyages") view=modules::voyages::PageVoyages/>
                    <Route path=path!("voyages/nouveau") view=modules::voyages::PageNouveauVoyage/>
                    <Route path=path!("voyages/serie") view=modules::voyages::PageNouvelleSerie/>
                    <Route path=path!("voyages/:id") view=modules::voyages::PageVoyage/>
                    <Route path=path!("comptabilite") view=modules::finances::PageComptabilite/>
                    <Route path=path!("mes-factures") view=modules::finances::PageFinancesFamille/>
                    <Route path=path!("messages") view=modules::messagerie::PageMessages/>
                    <Route path=path!("messages/nouveau") view=modules::messagerie::PageNouveauMessage/>
                    <Route path=path!("messages/:id") view=modules::messagerie::PageConversation/>
                    <Route path=path!("securite") view=modules::securite::PageSecurite/>
                    <Route path=path!("notifications") view=modules::notifications::PagePreferencesNotification/>
                    <Route path=path!("campagnes") view=modules::campagnes::PageCampagnes/>
                    <Route path=path!("rencontres") view=modules::rendez_vous::PageRencontres/>
                    <Route path=path!("rendez-vous") view=modules::rendez_vous::PageRendezVous/>
                    <Route path=path!("formulaires") view=modules::formulaires::PageFormulaires/>
                    <Route path=path!("discipline") view=modules::discipline_vs::PageDiscipline/>
                    <Route path=path!("signalements") view=modules::discipline_vs::PageSignalements/>
                    <Route path=path!("evaluations") view=modules::evaluations::PageEvaluations/>
                    <Route path=path!("evaluations/grille") view=modules::evaluations::PageGrille/>
                    <Route path=path!("evaluations/classes/:id") view=modules::evaluations::PageVueClasse/>
                    <Route path=path!("evaluations/conseils/:id") view=modules::evaluations::PageConseil/>
                    <Route path=path!("evaluations/bulletins") view=modules::evaluations::PageBulletins/>
                    <Route path=path!("evaluations/eleves/:id") view=modules::evaluations::PageEvaluationsEleve/>
                    <Route path=path!("referentiel") view=modules::referentiel_admin::PageReferentiel/>
                    <Route path=path!("referentiel/annees") view=modules::referentiel_admin::PageAnnees/>
                    <Route path=path!("referentiel/structure") view=modules::referentiel_admin::PageStructure/>
                    <Route path=path!("referentiel/classes") view=modules::referentiel_admin::PageClassesAdmin/>
                    <Route path=path!("referentiel/calendrier") view=modules::referentiel_admin::PageCalendrier/>
                    <Route path=path!("referentiel/grille") view=modules::referentiel_admin::PageGrille/>
                    <Route path=path!("direction/securite") view=modules::securite::PagePolitiqueSecurite/>
                    <Route path=path!("direction/protections") view=modules::protections::PageProtections/>
                    <Route path=path!("direction/roles") view=modules::roles::PageRoles/>
                    <Route path=path!("direction/roles/nouveau") view=modules::roles::PageNouveauRole/>
                    <Route path=path!("direction/roles/groupes") view=modules::roles::PageResponsables/>
                    <Route path=path!("direction/roles/:id") view=modules::roles::PageRole/>
                </ParentRoute>
            </Routes>
            <ent_web_ui::ZoneNotifications/>
        </Router>
    }
}

#[component]
fn PageIntrouvable() -> impl IntoView {
    view! {
        <Title text="Page introuvable"/>
        <main id="contenu" class="page page--etroite">
            <h1>"Page introuvable"</h1>
            <p><a href="/web">"Revenir à l'accueil"</a></p>
        </main>
    }
}

/// Point d'entrée WebAssembly : hydrate la page rendue par le serveur.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
