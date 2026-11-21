//! Tests de rendu (HTML produit côté serveur) : attributs d'accessibilité et états.

use leptos::prelude::*;

use crate::*;

fn rendu<V: IntoView + 'static>(f: impl FnOnce() -> V) -> String {
    let owner = Owner::new();
    owner.with(|| f().into_view().to_html())
}

#[test]
fn bouton_type_et_occupation() {
    let html = rendu(|| view! { <Bouton>"Valider"</Bouton> });
    assert!(html.contains(r#"type="button""#), "{html}");
    assert!(html.contains("ui-btn ui-btn--primaire"), "{html}");
    assert!(!html.contains("aria-busy"), "{html}");

    let html = rendu(
        || view! { <Bouton soumettre=true en_cours=true taille=Taille::Compacte>"Envoi"</Bouton> },
    );
    assert!(html.contains(r#"type="submit""#), "{html}");
    assert!(html.contains(r#"aria-busy="true""#), "{html}");
    assert!(
        html.contains("disabled"),
        "un bouton en cours est désactivé : {html}"
    );
    assert!(html.contains("ui-btn--compact"), "{html}");
}

#[test]
fn lien_bouton_reste_un_lien() {
    let html = rendu(
        || view! { <LienBouton href="/web/appel" variante=Variante::Secondaire>"Appel"</LienBouton> },
    );
    assert!(html.contains(r#"<a href="/web/appel""#), "{html}");
}

#[test]
fn champ_texte_relie_etiquette_aide_et_erreur() {
    let html = rendu(|| {
        let v = RwSignal::new(String::new());
        view! {
            <ChampTexte id="email" libelle="Adresse e-mail" valeur=v genre="email" requis=true
                aide="Adresse fournie par l'établissement" autocomplete="username"/>
        }
    });
    assert!(html.contains(r#"for="email""#), "{html}");
    assert!(html.contains(r#"id="email""#), "{html}");
    assert!(html.contains(r#"type="email""#), "{html}");
    assert!(html.contains(r#"aria-describedby="email-aide""#), "{html}");
    assert!(html.contains("(obligatoire)"), "requis annoncé : {html}");
    assert!(!html.contains("aria-invalid"), "{html}");

    let html = rendu(|| {
        let v = RwSignal::new(String::new());
        let e = Signal::derive(|| Some("Adresse invalide".to_string()));
        view! { <ChampTexte id="email" libelle="Adresse e-mail" valeur=v erreur=e/> }
    });
    assert!(html.contains(r#"aria-invalid="true""#), "{html}");
    assert!(
        html.contains(r#"aria-describedby="email-erreur""#),
        "{html}"
    );
    assert!(
        html.contains(r#"id="email-erreur""#) && html.contains("Adresse invalide"),
        "{html}"
    );
}

#[test]
fn selection_case_et_choix() {
    let html = rendu(|| {
        let v = RwSignal::new("b".to_string());
        view! { <Selection id="classe" libelle="Classe" valeur=v options=vec![("a".into(), "4A".into()), ("b".into(), "4B".into())]/> }
    });
    assert!(
        html.contains("<select") && html.contains(r#"value="b""#) && html.contains("4B"),
        "{html}"
    );

    let html = rendu(|| {
        let c = RwSignal::new(true);
        view! { <Case id="rappel" libelle="M'envoyer un rappel" coche=c/> }
    });
    assert!(
        html.contains(r#"type="checkbox""#) && html.contains(r#"for="rappel""#),
        "{html}"
    );

    let html = rendu(|| {
        let v = RwSignal::new("p".to_string());
        view! { <GroupeChoix nom="st" legende="Statut" valeur=v options=vec![("p".into(), "Présent".into()), ("a".into(), "Absent".into())]/> }
    });
    assert!(
        html.contains("<fieldset") && html.contains("<legend"),
        "{html}"
    );
    assert_eq!(html.matches(r#"type="radio""#).count(), 2, "{html}");
}

#[test]
fn resume_erreurs_liste_les_champs() {
    let html = rendu(|| {
        let e = Signal::derive(|| vec![("email".to_string(), "Adresse obligatoire".to_string())]);
        view! { <ResumeErreurs erreurs=e/> }
    });
    assert!(html.contains(r##"href="#email""##), "{html}");
    assert!(html.contains(r#"role="alert""#), "{html}");
    let vide = rendu(|| view! { <ResumeErreurs erreurs=Signal::derive(Vec::new)/> });
    assert!(!vide.contains("ui-resume-erreurs"), "{vide}");
}

#[test]
fn alertes_roles_et_fermeture() {
    let html = rendu(|| view! { <Alerte niveau=Niveau::Erreur titre="Échec"/> });
    assert!(html.contains(r#"role="alert""#), "{html}");
    let html = rendu(
        || view! { <Alerte niveau=Niveau::Succes titre="Enregistré" on_fermer=Callback::new(|_| ())/> },
    );
    assert!(html.contains(r#"role="status""#), "{html}");
    assert!(html.contains(r#"aria-label="Fermer le message""#), "{html}");
}

#[test]
fn confirmation_annuler_en_premier() {
    let html = rendu(|| {
        let o = RwSignal::new(false);
        view! {
            <Confirmation ouvert=o id="suppr" titre="Supprimer ?" message="Action définitive."
                libelle_confirmer="Supprimer" danger=true on_confirmer=Callback::new(|_| ())/>
        }
    });
    assert!(
        html.contains("<dialog") && html.contains(r#"aria-labelledby="suppr-titre""#),
        "{html}"
    );
    let annuler = html.find("Annuler").expect("Annuler");
    let confirmer = html.find("Supprimer<").unwrap_or(usize::MAX);
    assert!(
        annuler < confirmer,
        "Annuler doit précéder la confirmation : {html}"
    );
    assert!(html.contains("ui-btn--danger"), "{html}");
}

#[test]
fn etats_explicites() {
    let html = rendu(|| view! { <Chargement libelle="Chargement des élèves…"/> });
    assert!(
        html.contains(r#"role="status""#) && html.contains("Chargement des élèves"),
        "{html}"
    );
    let html = rendu(
        || view! { <EtatErreur titre="Impossible de charger" on_reessayer=Callback::new(|_| ())/> },
    );
    assert!(
        html.contains(r#"role="alert""#) && html.contains("Réessayer"),
        "{html}"
    );
    let html = rendu(|| view! { <AccesRefuse/> });
    assert!(html.contains("Accès refusé"), "{html}");
    let html = rendu(|| view! { <EtatVide titre="Aucun résultat"/> });
    assert!(html.contains("Aucun résultat"), "{html}");
}

#[test]
fn notifications_ajout_fermeture_et_affichage() {
    let owner = Owner::new();
    owner.with(|| {
        let n = fournir_notifications();
        let id = n.succes("Appel enregistré");
        n.info("Synchronisation terminée");
        assert_eq!(n.liste().len(), 2);
        let html = view! { <ZoneNotifications/> }.into_view().to_html();
        assert!(
            html.contains(r#"role="status""#) && html.contains(r#"aria-live="polite""#),
            "{html}"
        );
        assert!(
            html.contains("Appel enregistré") && html.contains("ui-notification--succes"),
            "{html}"
        );
        assert!(
            html.contains(r#"aria-label="Fermer la notification""#),
            "{html}"
        );
        n.fermer(id);
        assert_eq!(n.liste().len(), 1);
        assert_eq!(n.liste()[0].ton, Ton::Info);
    });
}

#[test]
fn initiales_robustes_aux_noms_difficiles() {
    assert_eq!(initiales("Léa MARTIN"), "LM");
    assert_eq!(initiales("Maximilien FONTAINE-DELACROIX"), "MD");
    assert_eq!(initiales("María José de la Cruz y Fernández"), "MF");
    assert_eq!(initiales("Ólafur Darri Ólafsson"), "ÓÓ");
    assert_eq!(initiales("dana"), "D");
    assert_eq!(initiales("J"), "J");
    assert_eq!(initiales("  Sam   Lee "), "SL");
    assert_eq!(initiales("🦊 Fox"), "F");
    assert_eq!(initiales("王秀英"), "王");
    assert_eq!(initiales(""), "?");
    assert_eq!(initiales("   "), "?");
    assert_eq!(initiales("Seán O'Brien"), "SO");
}

#[test]
fn lien_actif_selon_la_page() {
    assert!(lien_actif("/web", "/web"));
    assert!(lien_actif("/web", "/web/"));
    assert!(
        !lien_actif("/web", "/web/appel"),
        "l'accueil n'est actif que sur lui-même"
    );
    assert!(lien_actif("/web/appel", "/web/appel"));
    assert!(lien_actif("/web/appel", "/web/appel/detail"));
    assert!(!lien_actif("/web/appel", "/web/appels-archives"));
}

#[test]
fn coquille_navigation_accessible() {
    let html = rendu(|| {
        let entrees = vec![
            EntreeNav {
                libelle: "Aujourd'hui".into(),
                href: "/web".into(),
                icone: NomIcone::Accueil,
                badge: None,
            },
            EntreeNav {
                libelle: "Appel".into(),
                href: "/web/appel".into(),
                icone: NomIcone::Appel,
                badge: Some("3".into()),
            },
        ];
        view! {
            <Coquille
                entrees=entrees
                courant="/web/appel".to_string()
                etablissement="Collège de test".to_string()
                identite=Some(Identite { nom: "Claire Laurent".into(), detail: "Professeure".into() })
                on_deconnexion=Callback::new(|_| ())
            >
                <p>"contenu"</p>
            </Coquille>
        }
    });
    assert!(
        html.contains(r#"aria-label="Navigation principale""#),
        "{html}"
    );
    assert!(
        html.contains(r##"href="#contenu""##),
        "lien d'évitement : {html}"
    );
    assert!(html.contains(r#"id="contenu""#), "{html}");
    assert_eq!(
        html.matches(r#"aria-current="page""#).count(),
        1,
        "une seule page courante : {html}"
    );
    assert!(html.contains("ui-badge--compteur"), "{html}");
    assert!(html.contains(">CL<"), "initiales : {html}");
}

#[test]
fn tuile_reliee_a_son_titre() {
    let html = rendu(|| {
        view! {
            <GrilleBento libelle="Tableau de bord">
                <Tuile id="t1" titre="Séances du jour" largeur=Largeur::Quatre><p>"x"</p></Tuile>
            </GrilleBento>
        }
    });
    assert!(html.contains(r#"aria-labelledby="t1-titre""#), "{html}");
    assert!(html.contains(r#"<h2 id="t1-titre""#), "{html}");
    assert!(html.contains("ui-tuile--4"), "{html}");
}

#[test]
fn segmente_radios_natives_et_legende() {
    let html = rendu(|| {
        let v = RwSignal::new("semaine".to_string());
        view! {
            <Segmente nom="vue" legende="Affichage" valeur=v
                options=vec![OptionSegment::new("jour", "Jour"), OptionSegment::new("semaine", "Semaine")]/>
        }
    });
    assert!(html.contains("<fieldset"), "{html}");
    assert!(html.contains("<legend>Affichage</legend>"), "{html}");
    assert_eq!(html.matches(r#"type="radio""#).count(), 2, "{html}");
}
