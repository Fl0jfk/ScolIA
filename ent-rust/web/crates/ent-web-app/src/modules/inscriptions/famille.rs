//! Parcours famille sans compte (S2-3, D-014) : préinscription et invitation.
//!
//! 1. « Votre adresse e-mail » → la réponse est la même que l'adresse soit connue ou non.
//! 2. Code reçu par e-mail → jeton gardé **en mémoire** (jamais dans le stockage du
//!    navigateur) ; **rien n'est affiché avant cette étape**.
//! 3. Espace : « Nous avons déjà vos informations. Les utiliser ? », campagnes ouvertes,
//!    dossiers déposés, message de confidentialité toujours visible.
//! 4. Formulaire (pré-rempli ou libre), dépôt, suivi, pièces, coordonnées bancaires à la
//!    validation (IBAN masqué, nouveau mandat à signer).

use ent_web_api::types_inscriptions::{
    CampagneDto, CoordonneesBancairesDto, DossierDto, EspacePreinscriptionDto, MandatBody,
};
use ent_web_api::{ApiError, ClientPreinscription};
use ent_web_ui::{
    Alerte, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, Niveau, ResumeErreurs,
    Selection, Squelette, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::modele::{
    code_saisi, corps_preinscription, email_plausible, erreurs_formulaire, jeton_de_requete,
    libelle_nature, libelle_regime, libelle_statut, pieces_a_fournir, pre_remplir, ton_statut,
    Etape, SaisieFormulaire, REGIMES,
};

fn message(e: &ApiError) -> String {
    match e {
        ApiError::NonConnecte => {
            "Votre vérification a expiré : saisissez de nouveau votre adresse e-mail.".into()
        }
        autre => autre.message_utilisateur(),
    }
}

/// `/web/preinscription` (public).
#[component]
pub fn PagePreinscription() -> impl IntoView {
    let client = RwSignal::new(ClientPreinscription::new(""));
    let etape = RwSignal::new(Etape::Email);
    let email = RwSignal::new(String::new());
    let code = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let info = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let espace = RwSignal::new(None::<EspacePreinscriptionDto>);

    let charger_espace = move || {
        spawn_local(async move {
            match client.get_untracked().espace().await {
                Ok(e) => espace.set(Some(e)),
                Err(e) => {
                    if matches!(e, ApiError::NonConnecte) {
                        client.set(ClientPreinscription::new(""));
                        etape.set(Etape::Email);
                    }
                    erreur.set(Some(message(&e)));
                }
            }
        })
    };

    // Établissement : `?etab=` (labo local) ou configuration du serveur ; à terme, déduit
    // de l'adresse du site (D-012), ce qui rend les deux inutiles.
    let requete = use_query_map();
    let demarrer = Callback::new(move |_| {
        let etab_url = requete
            .with_untracked(|q| q.get("etab"))
            .and_then(|e| e.trim().parse::<Uuid>().ok());
        let e = email.get_untracked();
        if !email_plausible(&e) {
            erreur.set(Some("Saisissez une adresse e-mail valide.".into()));
            return;
        }
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let etab = match etab_url {
                Some(id) => Some(id),
                None => crate::modules::connexion::config_connexion()
                    .await
                    .ok()
                    .and_then(|c| c.etablissement_id),
            };
            let r = client.get_untracked().demarrer(&e, etab).await;
            en_cours.set(false);
            match r {
                Ok(d) => {
                    info.set(Some(d.message));
                    etape.set(Etape::Code {
                        demande_id: d.demande_id,
                    });
                }
                Err(e) => erreur.set(Some(message(&e))),
            }
        });
    });

    let verifier = Callback::new(move |_| {
        let Etape::Code { demande_id } = etape.get_untracked() else {
            return;
        };
        let Some(c) = code_saisi(&code.get_untracked()) else {
            erreur.set(Some("Le code comporte 6 chiffres.".into()));
            return;
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = client.get_untracked().verifier(demande_id, &c).await;
            en_cours.set(false);
            match r {
                Ok(verifie) => {
                    client.set(verifie);
                    info.set(None);
                    code.set(String::new());
                    etape.set(Etape::Espace);
                    charger_espace();
                }
                Err(e) => erreur.set(Some(message(&e))),
            }
        });
    });

    view! {
        <Title text="Préinscription"/>
        <main id="contenu" class="page page--etroite">
            <EnTetePage titre="Préinscription" sous_titre="Demande d'inscription dans l'établissement".to_string()/>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || info.get().map(|m| view! { <Alerte niveau=Niveau::Info titre=m/> })}
            {move || match etape.get() {
                Etape::Email => view! {
                    <form class="pile" on:submit=move |ev| { ev.prevent_default(); demarrer.run(()); }>
                        <ChampTexte id="pi-email" libelle="Votre adresse e-mail" valeur=email genre="email"
                            requis=true autocomplete="email"
                            aide="Un code de vérification vous sera envoyé. Aucune information n'est affichée avant cette vérification."/>
                        <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Continuer"</Bouton>
                    </form>
                }.into_any(),
                Etape::Code { .. } => view! {
                    <form class="pile" on:submit=move |ev| { ev.prevent_default(); verifier.run(()); }>
                        <ChampTexte id="pi-code" libelle="Code de vérification (6 chiffres)" valeur=code
                            requis=true autocomplete="one-time-code" inputmode="numeric"/>
                        <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Vérifier"</Bouton>
                        <Bouton variante=Variante::Discret on_click=Callback::new(move |_| { etape.set(Etape::Email); info.set(None); })>
                            "Changer d'adresse"
                        </Bouton>
                    </form>
                }.into_any(),
                Etape::Espace => view! {
                    <Espace espace=espace etape=etape/>
                }.into_any(),
                Etape::Formulaire { campagne_id } => view! {
                    <Formulaire client=client espace=espace campagne_id=campagne_id etape=etape
                        apres=Callback::new(move |_| charger_espace())/>
                }.into_any(),
                Etape::Dossier { id } => view! {
                    <SuiviDossier client=client id=id etape=etape/>
                }.into_any(),
            }}
        </main>
    }
}

#[component]
fn Espace(
    espace: RwSignal<Option<EspacePreinscriptionDto>>,
    etape: RwSignal<Etape>,
) -> impl IntoView {
    move || match espace.get() {
        None => view! { <Squelette lignes=4 libelle="Chargement de votre espace…"/> }.into_any(),
        Some(e) => {
            let campagnes = e.campagnes.clone();
            let dossiers = e.dossiers.clone();
            view! {
                <section class="pile" aria-labelledby="pi-espace">
                    <h2 id="pi-espace">{format!("Adresse vérifiée : {}", e.email)}</h2>
                    <Alerte niveau=Niveau::Info titre=e.message_confidentialite.clone()/>
                    {e.connu.then(|| view! {
                        <p>"Nous avons déjà vos informations : vous pourrez les utiliser pour remplir le formulaire."</p>
                    })}
                    <h3>"Demandes ouvertes"</h3>
                    {if campagnes.is_empty() {
                        view! { <EtatVide titre="Aucune campagne d'admission ouverte" message="Revenez pendant la période d'inscription de l'établissement."/> }.into_any()
                    } else {
                        view! {
                            <ul class="liste">
                                {campagnes.into_iter().map(|c: CampagneDto| {
                                    let id = c.id;
                                    view! {
                                        <li class="ligne-liste">
                                            <span>{format!("{} — année {}", c.libelle, c.annee_cible)}</span>
                                            {c.message_famille.clone().map(|m| view! { <p class="aide">{m}</p> })}
                                            <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| etape.set(Etape::Formulaire { campagne_id: id }))>
                                                "Déposer une demande"
                                            </Bouton>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }}
                    <h3>"Mes demandes"</h3>
                    {if dossiers.is_empty() {
                        view! { <p>"Aucune demande déposée avec cette adresse."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="liste">
                                {dossiers.into_iter().map(|d| {
                                    let id = d.id;
                                    view! {
                                        <li class="ligne-liste">
                                            <span>{format!("{} {}", d.eleve_prenom, d.eleve_nom)}</span>
                                            <Badge libelle=libelle_statut(&d.statut) ton=ton_statut(&d.statut)/>
                                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| etape.set(Etape::Dossier { id }))>
                                                "Suivre"
                                            </Bouton>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }}
                </section>
            }.into_any()
        }
    }
}

#[component]
fn Formulaire(
    client: RwSignal<ClientPreinscription>,
    espace: RwSignal<Option<EspacePreinscriptionDto>>,
    campagne_id: Uuid,
    etape: RwSignal<Etape>,
    apres: Callback<()>,
) -> impl IntoView {
    let e = espace.get_untracked();
    let profil = e.as_ref().and_then(|e| e.profil.clone());
    let niveaux: Vec<String> = e
        .as_ref()
        .and_then(|e| e.campagnes.iter().find(|c| c.id == campagne_id))
        .map(|c| c.niveaux.iter().map(|n| n.code.clone()).collect())
        .unwrap_or_default();
    let niveaux = StoredValue::new(niveaux);
    // Champs (un signal par champ).
    let eleve_nom = RwSignal::new(String::new());
    let eleve_prenom = RwSignal::new(String::new());
    let naissance = RwSignal::new(String::new());
    let niveau = RwSignal::new(niveaux.with_value(|n| n.first().cloned().unwrap_or_default()));
    let regime = RwSignal::new(String::new());
    let resp_nom = RwSignal::new(String::new());
    let resp_prenom = RwSignal::new(String::new());
    let resp_tel = RwSignal::new(String::new());
    let resp_adresse = RwSignal::new(String::new());
    let resp_lien = RwSignal::new(String::new());
    let resp_adresse_protegee = RwSignal::new(false);
    let second = RwSignal::new(false);
    let second_lien = RwSignal::new(String::new());
    let second_ne_pas_solliciter = RwSignal::new(false);
    let second_nom = RwSignal::new(String::new());
    let second_prenom = RwSignal::new(String::new());
    let second_email = RwSignal::new(String::new());
    let second_meme_foyer = RwSignal::new(true);
    let utiliser = RwSignal::new(false);
    let enregistrer = RwSignal::new(false);
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let a_profil = profil.is_some();
    let profil = StoredValue::new(profil);

    let utiliser_infos = Callback::new(move |_| {
        if let Some(p) = profil.get_value() {
            let mut s = SaisieFormulaire::default();
            pre_remplir(&mut s, &p);
            resp_nom.set(s.resp_nom);
            resp_prenom.set(s.resp_prenom);
            resp_tel.set(s.resp_telephone);
            resp_adresse.set(s.resp_adresse);
            utiliser.set(true);
        }
    });

    let envoyer = Callback::new(move |_| {
        let s = SaisieFormulaire {
            eleve_nom: eleve_nom.get_untracked(),
            eleve_prenom: eleve_prenom.get_untracked(),
            eleve_naissance: naissance.get_untracked(),
            niveau: niveau.get_untracked(),
            regime: regime.get_untracked(),
            resp_nom: resp_nom.get_untracked(),
            resp_prenom: resp_prenom.get_untracked(),
            resp_telephone: resp_tel.get_untracked(),
            resp_adresse: resp_adresse.get_untracked(),
            resp_lien: resp_lien.get_untracked(),
            resp_adresse_protegee: resp_adresse_protegee.get_untracked(),
            second: second.get_untracked(),
            second_lien: second_lien.get_untracked(),
            second_ne_pas_solliciter: second_ne_pas_solliciter.get_untracked(),
            second_nom: second_nom.get_untracked(),
            second_prenom: second_prenom.get_untracked(),
            second_email: second_email.get_untracked(),
            second_meme_foyer: second_meme_foyer.get_untracked(),
            utiliser_mes_infos: utiliser.get_untracked(),
            enregistrer_profil: enregistrer.get_untracked(),
        };
        let e = niveaux.with_value(|n| erreurs_formulaire(&s, n));
        if !e.is_empty() {
            erreurs.set(e);
            return;
        }
        erreurs.set(Vec::new());
        erreur.set(None);
        en_cours.set(true);
        let corps = corps_preinscription(campagne_id, &s);
        spawn_local(async move {
            let r = client.get_untracked().deposer(&corps).await;
            en_cours.set(false);
            match r {
                Ok(d) => {
                    apres.run(());
                    etape.set(Etape::Dossier { id: d.id });
                }
                Err(e) => erreur.set(Some(message(&e))),
            }
        });
    });

    let options_niveaux: Vec<(String, String)> =
        niveaux.with_value(|n| n.iter().map(|c| (c.clone(), c.clone())).collect());
    let options_regimes: Vec<(String, String)> =
        std::iter::once((String::new(), "Non précisé".to_string()))
            .chain(
                REGIMES
                    .iter()
                    .map(|r| (r.to_string(), libelle_regime(r).to_string())),
            )
            .collect();
    let options_liens = vec![
        (String::new(), "Non précisé".to_string()),
        ("mere".to_string(), "Mère".to_string()),
        ("pere".to_string(), "Père".to_string()),
        ("tuteur".to_string(), "Tuteur ou tutrice".to_string()),
        ("autre".to_string(), "Autre".to_string()),
    ];
    view! {
        <form class="pile" on:submit=move |ev| { ev.prevent_default(); envoyer.run(()); }>
            <h2>"Formulaire de préinscription"</h2>
            <ResumeErreurs erreurs=erreurs/>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {a_profil.then(|| view! {
                <Alerte niveau=Niveau::Info titre="Nous avons déjà vos informations. Les utiliser pour cette préinscription ?">
                    <Bouton variante=Variante::Secondaire on_click=utiliser_infos>"Oui, pré-remplir (modifiable)"</Bouton>
                </Alerte>
            })}
            <fieldset class="ui-champ">
                <legend>"L'élève"</legend>
                <ChampTexte id="pi-eleve-nom" libelle="Nom" valeur=eleve_nom requis=true/>
                <ChampTexte id="pi-eleve-prenom" libelle="Prénom" valeur=eleve_prenom requis=true/>
                <ChampTexte id="pi-eleve-naissance" libelle="Date de naissance" valeur=naissance genre="date"/>
                {(!options_niveaux.is_empty()).then(|| view! {
                    <Selection id="pi-niveau" libelle="Niveau demandé" valeur=niveau options=options_niveaux.clone() requis=true/>
                })}
                <Selection id="pi-regime" libelle="Régime souhaité" valeur=regime options=options_regimes/>
            </fieldset>
            <fieldset class="ui-champ">
                <legend>"Vous (responsable légal)"</legend>
                <ChampTexte id="pi-resp-nom" libelle="Nom" valeur=resp_nom requis=true autocomplete="family-name"/>
                <ChampTexte id="pi-resp-prenom" libelle="Prénom" valeur=resp_prenom autocomplete="given-name"/>
                <ChampTexte id="pi-resp-tel" libelle="Téléphone" valeur=resp_tel genre="tel" autocomplete="tel"/>
                <ChampTexte id="pi-resp-adresse" libelle="Adresse postale" valeur=resp_adresse autocomplete="street-address"/>
                <Selection id="pi-resp-lien" libelle="Lien avec l'élève" valeur=resp_lien options=options_liens.clone()/>
                <Case id="pi-resp-protegee" libelle="Mon adresse doit rester confidentielle (protection)" coche=resp_adresse_protegee/>
                <p class="aide">"Si vous cochez cette case, l'adresse n'est conservée nulle part en clair : seule la direction peut l'ouvrir, et elle n'est montrée à aucun autre parent ni au personnel."</p>
            </fieldset>
            <Case id="pi-second" libelle="Indiquer le second parent (il recevra une invitation à la validation du dossier)" coche=second/>
            {move || second.get().then(|| view! {
                <fieldset class="ui-champ">
                    <legend>"Second parent"</legend>
                    <ChampTexte id="pi-second-nom" libelle="Nom" valeur=second_nom requis=true/>
                    <ChampTexte id="pi-second-prenom" libelle="Prénom" valeur=second_prenom/>
                    <Selection id="pi-second-lien" libelle="Lien avec l'élève" valeur=second_lien options=options_liens.clone()/>
                    <Case id="pi-second-foyer" libelle="Même foyer que vous" coche=second_meme_foyer/>
                    <ChampTexte id="pi-second-email" libelle="Adresse e-mail du second parent" valeur=second_email genre="email"
                        aide="Obligatoire si les parents sont séparés (une fiche et un compte par parent)."/>
                    <Case id="pi-second-nps" libelle="Ne pas solliciter ce parent (décision de justice, mesure de protection…)" coche=second_ne_pas_solliciter/>
                    <p class="aide">"Il ne recevra ni invitation, ni demande de confirmation, ni relance. La direction vérifie et confirme cette mention avant toute prise de contact."</p>
                </fieldset>
            })}
            <Case id="pi-profil" libelle="Enregistrer ces informations dans votre profil" coche=enregistrer/>
            <p class="aide">"Les coordonnées bancaires ne sont pas demandées maintenant : elles le seront à la validation de l'inscription."</p>
            <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Envoyer ma demande"</Bouton>
            <Bouton variante=Variante::Discret on_click=Callback::new(move |_| etape.set(Etape::Espace))>"Annuler"</Bouton>
        </form>
    }
}

#[component]
fn SuiviDossier(
    client: RwSignal<ClientPreinscription>,
    id: Uuid,
    etape: RwSignal<Etape>,
) -> impl IntoView {
    let dossier = RwSignal::new(None::<Result<DossierDto, ApiError>>);
    let banque = RwSignal::new(None::<CoordonneesBancairesDto>);
    let message_action = RwSignal::new(None::<(bool, String)>);
    let charger = move || {
        spawn_local(async move {
            let c = client.get_untracked();
            let d = c.dossier(id).await;
            if matches!(&d, Ok(d) if d.statut == "validee") {
                banque.set(c.coordonnees_bancaires(id).await.ok());
            }
            dossier.set(Some(d));
        })
    };
    charger();
    let fichier = RwSignal::new(String::new());
    let iban = RwSignal::new(String::new());
    let titulaire = RwSignal::new(String::new());
    let resultat = move |r: Result<(), ApiError>, ok: &str| match r {
        Ok(()) => {
            message_action.set(Some((true, ok.to_string())));
            charger();
        }
        Err(e) => message_action.set(Some((false, message(&e)))),
    };
    view! {
        <Bouton variante=Variante::Discret on_click=Callback::new(move |_| etape.set(Etape::Espace))>"‹ Mes demandes"</Bouton>
        {move || message_action.get().map(|(ok, m)| view! {
            <Alerte niveau=if ok { Niveau::Succes } else { Niveau::Erreur } titre=m/>
        })}
        {move || match dossier.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement du dossier…"/> }.into_any(),
            Some(Err(e)) => view! { <Alerte niveau=Niveau::Erreur titre=message(&e)/> }.into_any(),
            Some(Ok(d)) => {
                let manquantes: Vec<(String, String)> = pieces_a_fournir(&d.pieces)
                    .into_iter()
                    .map(|p| (p.code.clone(), p.libelle.clone()))
                    .collect();
                let peut_deposer = d.actions.iter().any(|a| a == "deposer_piece");
                view! {
                    <section class="pile" aria-labelledby="pi-dossier">
                        <h2 id="pi-dossier">{format!("{} {} — {}", d.eleve.prenom, d.eleve.nom, libelle_nature(&d.nature))}</h2>
                        <p><Badge libelle=libelle_statut(&d.statut) ton=ton_statut(&d.statut)/></p>
                        {d.decision_motif.clone().map(|m| view! { <p>{format!("Motif : {m}")}</p> })}
                        <h3>"Pièces"</h3>
                        <ul class="liste">
                            {d.pieces.iter().map(|p| view! {
                                <li class="ligne-liste">
                                    <span>{format!("{}{}", p.libelle, if p.obligatoire { " (obligatoire)" } else { "" })}</span>
                                    <Badge libelle=libelle_statut(&p.statut) ton=ton_statut(&p.statut)/>
                                    {p.motif_refus.clone().map(|m| view! { <span class="aide">{m}</span> })}
                                </li>
                            }).collect_view()}
                        </ul>
                        {(peut_deposer && !manquantes.is_empty()).then(|| {
                            let premiere = manquantes[0].clone();
                            view! {
                                <form class="pile" on:submit=move |ev| {
                                    ev.prevent_default();
                                    let code = premiere.0.clone();
                                    let nom = fichier.get_untracked();
                                    spawn_local(async move {
                                        let r = client.get_untracked().deposer_piece(id, &code, &nom).await.map(|_| ());
                                        resultat(r, "Pièce déposée.");
                                    });
                                }>
                                    <ChampTexte id="pi-fichier" libelle=format!("Nom du fichier pour « {} »", manquantes[0].1) valeur=fichier requis=true/>
                                    <Bouton variante=Variante::Secondaire soumettre=true>"Déclarer la pièce"</Bouton>
                                </form>
                            }
                        })}
                        <h3>"Suivi"</h3>
                        <ol class="liste">
                            {d.suivi.iter().map(|s| view! {
                                <li>{format!("{} — {}", s.quand.format("%d/%m/%Y %H:%M"), s.texte.clone().unwrap_or_else(|| libelle_statut(s.statut_apres.as_deref().unwrap_or_default()).to_string()))}</li>
                            }).collect_view()}
                        </ol>
                        {(d.statut == "validee").then(|| view! {
                            <section class="pile" aria-labelledby="pi-banque">
                                <h3 id="pi-banque">"Coordonnées bancaires"</h3>
                                <p class="aide">"Un nouveau mandat de prélèvement SEPA est à signer pour cet établissement."</p>
                                {move || banque.get().map(|b| {
                                    let mandat = b.mandat.clone();
                                    view! {
                                        {mandat.clone().map(|m| view! {
                                            <p>{format!("Mandat {} : {}", if m.statut == "signe" { "signé" } else { "à signer" }, m.iban_masque)}</p>
                                            {(m.statut == "a_signer").then(|| view! {
                                                <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| spawn_local(async move {
                                                    let r = client.get_untracked().signer_mandat(id).await.map(|_| ());
                                                    resultat(r, "Mandat signé.");
                                                }))>"Je signe le mandat de prélèvement"</Bouton>
                                            })}
                                        })}
                                        {mandat.is_none().then(|| view! {
                                            {b.propositions.iter().map(|p| {
                                                let pid = p.proposition_id;
                                                view! {
                                                    <p>{format!("Reprendre vos coordonnées bancaires : {} ({})", p.iban_masque, p.titulaire_initiales)}</p>
                                                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| spawn_local(async move {
                                                        let b = MandatBody { proposition_id: Some(pid), ..Default::default() };
                                                        let r = client.get_untracked().demander_mandat(id, &b).await.map(|_| ());
                                                        resultat(r, "Coordonnées reprises : le mandat est à signer.");
                                                    }))>"Reprendre ces coordonnées"</Bouton>
                                                }
                                            }).collect_view()}
                                            <form class="pile" on:submit=move |ev| {
                                                ev.prevent_default();
                                                let b = MandatBody {
                                                    iban: Some(iban.get_untracked()),
                                                    titulaire: Some(titulaire.get_untracked()),
                                                    ..Default::default()
                                                };
                                                spawn_local(async move {
                                                    let r = client.get_untracked().demander_mandat(id, &b).await.map(|_| ());
                                                    resultat(r, "Coordonnées enregistrées : le mandat est à signer.");
                                                });
                                            }>
                                                <ChampTexte id="pi-iban" libelle="IBAN" valeur=iban requis=true autocomplete="off"/>
                                                <ChampTexte id="pi-titulaire" libelle="Titulaire du compte" valeur=titulaire requis=true/>
                                                <Bouton variante=Variante::Secondaire soumettre=true>"Enregistrer"</Bouton>
                                            </form>
                                        })}
                                    }
                                })}
                            </section>
                        })}
                    </section>
                }.into_any()
            }
        }}
    }
}

/// `/web/invitation?jeton=…` (public) : confirmer l'adresse et créer ou rattacher le compte.
#[component]
pub fn PageInvitation() -> impl IntoView {
    let requete = use_query_map();
    let jeton = Memo::new(move |_| {
        requete
            .with(|q| q.get("jeton"))
            .and_then(|j| jeton_de_requete(&format!("jeton={j}")))
    });
    let etat = RwSignal::new(None::<Result<String, String>>);
    let en_cours = RwSignal::new(false);
    let accepter = Callback::new(move |_| {
        let Some(j) = jeton.get_untracked() else {
            return;
        };
        en_cours.set(true);
        spawn_local(async move {
            let r = ClientPreinscription::new("").accepter_invitation(&j).await;
            en_cours.set(false);
            etat.set(Some(
                r.map(|a| a.message).map_err(|e| e.message_utilisateur()),
            ));
        });
    });
    view! {
        <Title text="Invitation"/>
        <main id="contenu" class="page page--etroite">
            <EnTetePage titre="Invitation à l'espace famille"/>
            {move || match (jeton.get(), etat.get()) {
                (None, _) => view! { <EtatVide titre="Lien incomplet" message="Ouvrez le lien reçu par e-mail en entier."/> }.into_any(),
                (_, Some(Ok(m))) => view! {
                    <Alerte niveau=Niveau::Succes titre=m/>
                    <p><a href="/web/connexion">"Se connecter"</a></p>
                }.into_any(),
                (_, Some(Err(m))) => view! { <Alerte niveau=Niveau::Erreur titre=m/> }.into_any(),
                (Some(_), None) => view! {
                    <p>"En acceptant, vous confirmez votre adresse e-mail. Votre compte est créé, ou votre compte existant est rattaché à cet établissement."</p>
                    <Bouton variante=Variante::Primaire en_cours=en_cours on_click=accepter>"Accepter l'invitation"</Bouton>
                }.into_any(),
            }}
        </main>
    }
}
