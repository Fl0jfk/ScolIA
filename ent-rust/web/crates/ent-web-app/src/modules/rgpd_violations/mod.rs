//! **Registre des violations de données** (socle 27, palier G5 ; `27_RGPD_ARCHIVES.md` §8.7, §14.10, §18.2 point 5).
//!
//! | Écran | Adresse | Qui |
//! |---|---|---|
//! | Déclarer un incident, en trente secondes, et suivre mes déclarations | `/web/incident` | tout personnel (`violations.declarer`) |
//! | Registre (horloge des 72 h, retards, export) | `/web/rgpd/violations` | `violations.gerer` |
//! | Saisir une violation signalée hors ligne | `/web/rgpd/violations/saisir` | `violations.gerer` |
//! | Fiche (qualification, risque, CNIL, personnes, mesures, clôture, chronologie) | `/web/rgpd/violations/:id` | `violations.gerer` ; décisions `violations.decider` |
//!
//! Règles d'écran : le déclarant ne voit que la référence et l'avancement (R-RG32) ; l'horloge, la grille de risque
//! et ce qui manque pour clore viennent du serveur ; aucune page n'est bloquée côté client sur l'absence d'un droit
//! (le serveur répond 403, `ErreurChargement` l'affiche) ; les boutons d'écriture suivent le moteur de droits ;
//! l'information des personnes part après un **aperçu** et une confirmation **par le nombre** de destinataires.

pub mod modele;

use chrono::{DateTime, Utc};
use ent_web_api::rgpd_violations_types::{
    ApercuInformationDto, BrouillonCnilDto, ComplementCnilBody, DeclarationBody,
    FiltreRegistreQuery, InformationBody, MaDeclarationDto, MesureBody, NotificationCnilBody,
    RisqueBody, SaisieViolationBody, TexteBody, ViolationDto, ViolationLigneDto,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, GrilleBento, Largeur,
    LienBouton, Niveau, Selection, Squelette, Statistique, TonEtat, Tuile, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;
use modele::{
    compter, corps_personnes, corps_qualification, declaration_recevable, echeance_reglee,
    echeance_reglee_ligne, instant_saisi, libelle_avancement, libelle_etape, libelle_etat,
    libelle_origine, libelle_risque, lien_fiche_pdf, lien_registre_pdf,
    message_information_recevable, phrase_horloge, retard_a_motiver, saisie_depuis, trier_registre,
    urgence, SaisieQualification, Urgence, CATEGORIES, ETATS, RISQUES,
};

fn ton_urgence(u: Urgence) -> TonEtat {
    match u {
        Urgence::Aucune => TonEtat::Neutre,
        Urgence::Normale => TonEtat::Info,
        Urgence::Proche => TonEtat::Avertissement,
        Urgence::Critique => TonEtat::Mal,
    }
}

fn ton_risque(r: Option<&str>) -> TonEtat {
    match r {
        Some("eleve") => TonEtat::Mal,
        Some("avere") => TonEtat::Avertissement,
        Some("faible") => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

fn vide_si_blanc(s: String) -> Option<String> {
    Some(s.trim().to_string()).filter(|t| !t.is_empty())
}

// ═══════════════════════════════════ Déclarer (tout personnel) ═══════════════════════════════════

#[component]
pub fn PageDeclarerIncident() -> impl IntoView {
    let version = RwSignal::new(0u32);
    let miennes = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().violations_mes_declarations().await }
    });
    view! {
        <Title text="Signaler un incident de données"/>
        <EnTetePage
            surtitre="Protection des données".to_string()
            titre="Signaler un incident de données".to_string()
            sous_titre="Clé USB perdue, message envoyé à la mauvaise personne, écran resté ouvert, document affiché par erreur : dites-le tout de suite. Personne ne vous le reprochera ; un incident tu est bien plus grave.".to_string()
        />
        <FormulaireDeclaration version=version/>
        <section class="classe-section" aria-labelledby="vio-miennes">
            <h2 id="vio-miennes">"Mes signalements"</h2>
            {move || match miennes.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement de vos signalements…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| miennes.refetch())/>
                }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun signalement" message="Vous n'avez rien signalé."/>
                }.into_any(),
                Some(Ok(l)) => view! { <MesDeclarations lignes=l/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn FormulaireDeclaration(version: RwSignal<u32>) -> impl IntoView {
    let intitule = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let date = RwSignal::new(String::new());
    let heure = RwSignal::new(String::new());
    let securite = RwSignal::new(false);
    let recu = RwSignal::new(None::<MaDeclarationDto>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));

    let envoyer = move |_| {
        if let Err(m) = declaration_recevable(&intitule.get()) {
            ecriture.erreur.set(Some(m.into()));
            return;
        }
        let constatee_le = match instant_saisi(&date.get(), &heure.get()) {
            Ok(i) => i,
            Err(m) => {
                ecriture.erreur.set(Some(m.into()));
                return;
            }
        };
        let b = DeclarationBody {
            intitule: intitule.get().trim().to_string(),
            description: vide_si_blanc(description.get()),
            constatee_le,
            incident_securite: securite.get(),
        };
        ecriture.lancer(async move {
            session::client().violations_declarer(&b).await.map(|m| {
                let message = format!("Signalement {} reçu. Merci.", m.reference);
                recu.set(Some(m));
                intitule.set(String::new());
                description.set(String::new());
                date.set(String::new());
                heure.set(String::new());
                securite.set(false);
                message
            })
        });
    };

    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || recu.get().map(|m| view! {
            <Alerte niveau=Niveau::Succes titre=format!("Signalement {} reçu", m.reference)>
                <p>{m.message.clone()}</p>
                <p>"La direction est prévenue. Si on vous demande des précisions, gardez cette référence."</p>
            </Alerte>
        })}
        <section class="classe-section" aria-labelledby="vio-declarer">
            <h2 id="vio-declarer">"Que s'est-il passé ?"</h2>
            <ChampTexte id="vio-intitule".to_string() libelle="En une phrase".to_string() valeur=intitule requis=true
                placeholder="J'ai perdu une clé USB avec les notes de 4e B".to_string()/>
            <ZoneTexte id="vio-description".to_string() libelle="Précisions (facultatif)".to_string() valeur=description
                aide="Ce qui est touché, combien de personnes à peu près, ce que vous avez déjà fait. N'y recopiez pas les données elles-mêmes.".to_string()/>
            <div class="vio-ligne">
                <ChampTexte id="vio-date".to_string() libelle="Depuis quand (date, si vous la savez)".to_string()
                    valeur=date genre="date"/>
                <ChampTexte id="vio-heure".to_string() libelle="Heure (facultatif)".to_string() valeur=heure genre="time"/>
            </div>
            <Case id="vio-securite".to_string()
                libelle="Aucune donnée personnelle n'est touchée (tentative d'hameçonnage, virus bloqué…)".to_string()
                coche=securite/>
            <Bouton variante=Variante::Primaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                on_click=Callback::new(envoyer)>
                "Signaler"
            </Bouton>
        </section>
    }
}

#[component]
fn MesDeclarations(lignes: Vec<MaDeclarationDto>) -> impl IntoView {
    view! {
        <table class="ui-tableau">
            <caption class="ui-sr">"Mes signalements : référence, objet, date, avancement"</caption>
            <thead>
                <tr>
                    <th scope="col">"Référence"</th>
                    <th scope="col">"Objet"</th>
                    <th scope="col">"Signalé le"</th>
                    <th scope="col">"Avancement"</th>
                </tr>
            </thead>
            <tbody>
                {lignes.into_iter().map(|m| {
                    let ton = if m.avancement == "close" { TonEtat::Ok } else { TonEtat::Info };
                    view! {
                        <tr>
                            <th scope="row">{m.reference.clone()}</th>
                            <td>{m.intitule.clone()}</td>
                            <td>{date_heure_paris(m.connue_le)}</td>
                            <td><Badge libelle=libelle_avancement(&m.avancement).to_string() ton=ton/></td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table>
    }
}

// ═══════════════════════════════════ Registre (gestionnaires) ═══════════════════════════════════

#[component]
pub fn PageRegistreViolations() -> impl IntoView {
    let moteur = use_droits_moteur();
    let etat = RwSignal::new(String::new());
    let a_notifier = RwSignal::new(false);
    let registre = LocalResource::new(move || {
        let f = FiltreRegistreQuery {
            etat: vide_si_blanc(etat.get()),
            a_notifier: Some(a_notifier.get()),
            ..Default::default()
        };
        async move { session::client().violations_registre(&f).await }
    });
    let mut etats = vec![(String::new(), "Tous les états".to_string())];
    etats.extend(
        ETATS
            .iter()
            .map(|e| ((*e).to_string(), libelle_etat(e).to_string())),
    );
    view! {
        <Title text="Registre des violations"/>
        <EnTetePage
            surtitre="Protection des données".to_string()
            titre="Registre des violations".to_string()
            sous_titre="Toutes les violations, notifiées ou non (art. 33.5). La CNIL doit être notifiée dans les 72 heures, sauf risque faible justifié.".to_string()
        >
            {move || moteur.get().peut_gerer_violations().then(|| view! {
                <LienBouton href="/web/rgpd/violations/saisir" variante=Variante::Primaire>
                    "Saisir une violation signalée"
                </LienBouton>
                <a class="ui-bouton ui-bouton-secondaire" href="/rgpd/violations/export.csv" download="registre-violations.csv">
                    "Exporter le registre (CSV)"
                </a>
                <a class="ui-bouton ui-bouton-secondaire" download="registre-violations.pdf"
                    href=move || lien_registre_pdf(&etat.get(), a_notifier.get())>
                    "Registre en PDF (filtre appliqué)"
                </a>
            })}
        </EnTetePage>
        <div class="rgpd-filtres" role="group" aria-label="Filtrer le registre">
            <Selection id="vio-filtre-etat".to_string() libelle="État".to_string() valeur=etat options=etats/>
            <Case id="vio-filtre-notifier".to_string() libelle="Seulement celles à notifier".to_string() coche=a_notifier/>
        </div>
        {move || match registre.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement du registre…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| registre.refetch())/>
            }.into_any(),
            Some(Ok(mut l)) => {
                trier_registre(&mut l);
                view! { <Registre lignes=l/> }.into_any()
            }
        }}
    }
}

#[component]
fn Registre(lignes: Vec<ViolationLigneDto>) -> impl IntoView {
    let c = compter(&lignes);
    let ton_retard = if c.en_retard > 0 {
        TonEtat::Mal
    } else {
        TonEtat::Ok
    };
    let ton_horloge = if c.horloge_en_cours > 0 {
        TonEtat::Avertissement
    } else {
        TonEtat::Neutre
    };
    let vide = lignes.is_empty();
    view! {
        <GrilleBento libelle="Compteurs du registre".to_string()>
            <Tuile id="vio-compteurs".to_string() largeur=Largeur::Douze>
                <div class="rgpd-stats">
                    <Statistique libelle="Ouvertes".to_string() valeur=Signal::derive(move || c.ouvertes.to_string())/>
                    <Statistique libelle="Horloge en cours".to_string()
                        valeur=Signal::derive(move || c.horloge_en_cours.to_string()) ton=ton_horloge/>
                    <Statistique libelle="Échéance dépassée".to_string()
                        valeur=Signal::derive(move || c.en_retard.to_string()) ton=ton_retard/>
                    <Statistique libelle="Closes".to_string() valeur=Signal::derive(move || c.closes.to_string())/>
                </div>
            </Tuile>
        </GrilleBento>
        {if vide {
            view! { <EtatVide titre="Aucune violation" message="Rien ne correspond à ce filtre."/> }.into_any()
        } else {
            view! {
                <table class="ui-tableau">
                    <caption class="ui-sr">"Registre : référence, objet, prise de connaissance, échéance, risque, état"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Référence"</th>
                            <th scope="col">"Objet"</th>
                            <th scope="col">"Connue le"</th>
                            <th scope="col">"Échéance CNIL"</th>
                            <th scope="col">"Risque"</th>
                            <th scope="col">"État"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {lignes.into_iter().map(|l| {
                            let href = format!("/web/rgpd/violations/{}", l.id);
                            let reglee = echeance_reglee_ligne(&l);
                            let horloge = phrase_horloge(l.heures_restantes, l.en_retard, reglee)
                                .unwrap_or_else(|| if l.notifiee_cnil_le.is_some() {
                                    "Notifiée".to_string()
                                } else if l.non_notifiee {
                                    "Non-notification justifiée".to_string()
                                } else {
                                    "Sans objet".to_string()
                                });
                            let ton_h = ton_urgence(urgence(l.heures_restantes, l.en_retard, reglee));
                            let risque = libelle_risque(l.risque.as_deref().unwrap_or("")).to_string();
                            let ton_r = ton_risque(l.risque.as_deref());
                            let securite = l.nature_incident == "incident_securite";
                            let plateforme = l.origine == "plateforme";
                            view! {
                                <tr>
                                    <th scope="row"><a href=href>{l.reference.clone()}</a></th>
                                    <td>
                                        {l.intitule.clone()}
                                        {securite.then(|| view! { <Badge libelle="Incident de sécurité".to_string() ton=TonEtat::Info/> })}
                                        {plateforme.then(|| view! { <Badge libelle="Plateforme".to_string() ton=TonEtat::Info/> })}
                                    </td>
                                    <td>{date_heure_paris(l.connue_le)}</td>
                                    <td><Badge libelle=horloge ton=ton_h/></td>
                                    <td><Badge libelle=risque ton=ton_r/></td>
                                    <td>{libelle_etat(&l.etat)}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            }.into_any()
        }}
    }
}

// ═══════════════════════════════════ Saisie par un gestionnaire ═══════════════════════════════════

#[component]
pub fn PageSaisirViolation() -> impl IntoView {
    let intitule = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let connue_date = RwSignal::new(String::new());
    let connue_heure = RwSignal::new(String::new());
    let cree = RwSignal::new(None::<Uuid>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {}));
    let envoyer = move |_| {
        if let Err(m) = declaration_recevable(&intitule.get()) {
            ecriture.erreur.set(Some(m.into()));
            return;
        }
        let connue_le = match instant_saisi(&connue_date.get(), &connue_heure.get()) {
            Ok(i) => i,
            Err(m) => {
                ecriture.erreur.set(Some(m.into()));
                return;
            }
        };
        let b = SaisieViolationBody {
            declaration: DeclarationBody {
                intitule: intitule.get().trim().to_string(),
                description: vide_si_blanc(description.get()),
                constatee_le: None,
                incident_securite: false,
            },
            connue_le,
        };
        ecriture.lancer(async move {
            session::client().violations_saisir(&b).await.map(|v| {
                cree.set(Some(v.id));
                format!("Violation {} enregistrée.", v.reference)
            })
        });
    };
    view! {
        <Title text="Saisir une violation"/>
        <EnTetePage
            surtitre="Protection des données".to_string()
            titre="Saisir une violation signalée".to_string()
            sous_titre="Un appel, un courriel, un signalement au guichet : la prise de connaissance est l'instant où l'établissement l'a apprise, elle fait courir les 72 heures.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || cree.get().map(|id| {
            let href = format!("/web/rgpd/violations/{id}");
            view! {
                <Alerte niveau=Niveau::Succes titre="Violation enregistrée".to_string()>
                    <p><LienBouton href=href variante=Variante::Primaire>"Ouvrir la fiche"</LienBouton></p>
                </Alerte>
            }
        })}
        <section class="classe-section" aria-labelledby="vio-saisie">
            <h2 id="vio-saisie">"La violation"</h2>
            <ChampTexte id="vio-s-intitule".to_string() libelle="Ce qui s'est passé, en une phrase".to_string()
                valeur=intitule requis=true/>
            <ZoneTexte id="vio-s-description".to_string() libelle="Précisions".to_string() valeur=description/>
            <div class="vio-ligne">
                <ChampTexte id="vio-s-date".to_string() libelle="Apprise le (vide : maintenant)".to_string()
                    valeur=connue_date genre="date"
                    aide="Jamais dans le futur, au plus un an en arrière.".to_string()/>
                <ChampTexte id="vio-s-heure".to_string() libelle="À (heure)".to_string() valeur=connue_heure genre="time"/>
            </div>
            <Bouton variante=Variante::Primaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                on_click=Callback::new(envoyer)>
                "Enregistrer"
            </Bouton>
        </section>
    }
}

// ═══════════════════════════════════ Fiche ═══════════════════════════════════

#[component]
pub fn PageViolation() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|s| Uuid::parse_str(&s).ok())
    };
    let version = RwSignal::new(0u32);
    let fiche = LocalResource::new(move || {
        let _ = version.get();
        async move {
            match id() {
                Some(i) => session::client().violations_fiche(i).await,
                None => Err(ApiError::Introuvable("identifiant invalide".into())),
            }
        }
    });
    view! {
        <Title text="Violation de données"/>
        {move || match fiche.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la violation…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EtatVide titre="Violation introuvable" message="Elle n'existe pas, ou vous n'y avez pas accès.">
                    <LienBouton href="/web/rgpd/violations" variante=Variante::Secondaire>"Retour au registre"</LienBouton>
                </EtatVide>
            }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fiche.refetch())/>
            }.into_any(),
            Some(Ok(v)) => view! { <Fiche v=v version=version/> }.into_any(),
        }}
    }
}

#[component]
fn Fiche(v: ViolationDto, version: RwSignal<u32>) -> impl IntoView {
    let moteur = use_droits_moteur();
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|n| *n += 1)));
    let close = v.etat == "close";
    let violation = v.nature_incident == "violation";
    let reglee = echeance_reglee(&v);
    let horloge = phrase_horloge(v.heures_restantes, v.en_retard, reglee);
    let niveau_horloge = match urgence(v.heures_restantes, v.en_retard, reglee) {
        Urgence::Critique => Niveau::Erreur,
        Urgence::Proche => Niveau::Avertissement,
        _ => Niveau::Info,
    };
    let echeance = date_heure_paris(v.echeance_cnil_le);
    let sous_titre = format!(
        "{} · connue le {} · {}",
        libelle_origine(&v.origine),
        date_heure_paris(v.connue_le),
        libelle_etat(&v.etat)
    );
    let chronologie = StoredValue::new(v.chronologie.clone());
    let mesures = StoredValue::new(v.mesures.clone());
    let notifications = StoredValue::new(v.notifications.clone());
    let manque = v.manque_pour_clore.clone();
    let plateforme = v.plateforme_element.clone();
    let description = v.description.clone();
    let constatee = v.constatee_le.map(date_heure_paris);
    let fiche = StoredValue::new(v.clone());
    view! {
        <EnTetePage
            surtitre=format!("Violation {}", v.reference)
            titre=v.intitule.clone()
            sous_titre=sous_titre
        >
            <LienBouton href="/web/rgpd/violations" variante=Variante::Secondaire>"Registre"</LienBouton>
            <a class="ui-bouton ui-bouton-secondaire" href=lien_fiche_pdf(v.id)
                download=format!("violation-{}.pdf", v.reference)>
                "Fiche PDF"
            </a>
        </EnTetePage>
        <RetourEcriture ecriture=ecriture/>
        {horloge.map(|h| view! {
            <Alerte niveau=niveau_horloge titre=h>
                <p>{format!("Échéance : {echeance}.")}</p>
            </Alerte>
        })}
        {plateforme.map(|p| view! {
            <section class="classe-section" aria-labelledby="vio-plateforme">
                <h2 id="vio-plateforme">{format!("Signalée par l'éditeur ({})", p.reference)}</h2>
                <p><strong>{p.intitule.clone()}</strong></p>
                <p>{p.description.clone()}</p>
                <p>{format!("Ce qui concerne l'établissement : {}", p.elements)}</p>
                <p class="texte-attenue">{format!(
                    "Connue de l'éditeur le {}, communiquée à l'établissement le {}.",
                    date_heure_paris(p.connue_editeur_le),
                    date_heure_paris(p.informe_le)
                )}</p>
                {p.couche_compte.then(|| view! {
                    <p>"Les comptes de la plateforme sont touchés : ScolIA notifie la CNIL pour sa part. L'établissement décide de sa propre notification."</p>
                })}
            </section>
        })}
        <section class="classe-section" aria-labelledby="vio-declaration">
            <h2 id="vio-declaration">"Déclaration"</h2>
            {description.map(|d| view! { <p>{d}</p> })}
            {constatee.map(|c| view! { <p>{format!("Constatée le {c}.")}</p> })}
        </section>
        {move || (!close && moteur.get().peut_gerer_violations()).then(|| view! {
            <Qualification v=fiche.get_value() ecriture=ecriture/>
        })}
        {violation.then(|| view! { <Risque v=fiche.get_value() ecriture=ecriture/> })}
        {violation.then(|| view! { <NotificationCnil v=fiche.get_value() ecriture=ecriture/> })}
        {violation.then(|| view! { <Personnes v=fiche.get_value() ecriture=ecriture/> })}
        <section class="classe-section" aria-labelledby="vio-mesures">
            <h2 id="vio-mesures">"Mesures"</h2>
            {move || {
                let l = mesures.get_value();
                if l.is_empty() {
                    view! { <p class="texte-attenue">"Aucune mesure enregistrée."</p> }.into_any()
                } else {
                    view! {
                        <ul>
                            {l.into_iter().map(|m| {
                                let nature = if m.nature == "prise" { "Prise" } else { "Envisagée" };
                                view! { <li>{format!("{nature} ({}) : {}", date_heure_paris(m.le), m.texte)}</li> }
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }
            }}
            {(!close).then(|| view! { <AjoutMesure id=fiche.get_value().id ecriture=ecriture/> })}
        </section>
        {move || {
            let l = notifications.get_value();
            (!l.is_empty()).then(|| view! {
                <section class="classe-section" aria-labelledby="vio-notifs">
                    <h2 id="vio-notifs">"Notifications à la CNIL"</h2>
                    <ul>
                        {l.into_iter().map(|n| {
                            let nature = if n.nature == "initiale" { "Initiale" } else { "Complémentaire" };
                            view! { <li>{format!(
                                "{nature} n° {} le {} (référence {}){}",
                                n.rang,
                                date_heure_paris(n.notifiee_le),
                                n.reference_cnil,
                                n.resume.map(|r| format!(" : {r}")).unwrap_or_default()
                            )}</li> }
                        }).collect_view()}
                    </ul>
                </section>
            })
        }}
        {(!close).then(|| view! { <Cloture id=fiche.get_value().id manque=manque ecriture=ecriture/> })}
        <section class="classe-section" aria-labelledby="vio-chrono">
            <h2 id="vio-chrono">"Chronologie"</h2>
            <ol class="rgpd-journal">
                {move || chronologie.get_value().into_iter().map(|e| {
                    let qui = e.par_role.map(|r| format!(" · {r}")).unwrap_or_default();
                    view! {
                        <li>
                            <strong>{libelle_etape(&e.etape)}</strong>
                            {format!(" — {}{qui}", date_heure_paris(e.le))}
                            {e.texte.map(|t| view! { <p>{t}</p> })}
                        </li>
                    }
                }).collect_view()}
            </ol>
        </section>
    }
}

#[component]
fn Qualification(v: ViolationDto, ecriture: Ecriture) -> impl IntoView {
    let s = saisie_depuis(&v);
    let securite = RwSignal::new(s.incident_securite);
    let c = RwSignal::new(s.confidentialite);
    let i = RwSignal::new(s.integrite);
    let d = RwSignal::new(s.disponibilite);
    let personnes = RwSignal::new(s.personnes);
    let estime = RwSignal::new(s.personnes_estime);
    let mineurs = RwSignal::new(s.mineurs);
    let consequences = RwSignal::new(s.consequences);
    // Un signal par catégorie (FRONTEND_LEPTOS.md §4).
    let cases: Vec<(&'static str, &'static str, RwSignal<bool>)> = CATEGORIES
        .iter()
        .map(|(code, lib)| {
            (
                *code,
                *lib,
                RwSignal::new(s.categories.iter().any(|x| x == code)),
            )
        })
        .collect();
    let cases = StoredValue::new(cases);
    let id = v.id;
    let titre = if v.qualifiee_le.is_some() {
        "Qualification (corriger)"
    } else {
        "Qualifier"
    };
    let envoyer = move |_| {
        let saisie = SaisieQualification {
            incident_securite: securite.get(),
            confidentialite: c.get(),
            integrite: i.get(),
            disponibilite: d.get(),
            categories: cases.with_value(|l| {
                l.iter()
                    .filter(|(_, _, s)| s.get())
                    .map(|(code, _, _)| (*code).to_string())
                    .collect()
            }),
            personnes: personnes.get(),
            personnes_estime: estime.get(),
            mineurs: mineurs.get(),
            consequences: consequences.get(),
        };
        match corps_qualification(&saisie) {
            Err(m) => ecriture.erreur.set(Some(m.into())),
            Ok(b) => ecriture.lancer(async move {
                session::client()
                    .violations_qualifier(id, &b)
                    .await
                    .map(|v| {
                        format!(
                            "Qualification enregistrée : {}.",
                            libelle_risque(v.risque_propose.as_deref().unwrap_or(""))
                        )
                    })
            }),
        }
    };
    view! {
        <section class="classe-section" aria-labelledby="vio-qualif">
            <h2 id="vio-qualif">{titre}</h2>
            <Case id="vio-q-securite".to_string()
                libelle="Incident de sécurité sans atteinte aux données personnelles".to_string() coche=securite/>
            <fieldset class="rgpd-choix">
                <legend>"Ce qui est atteint"</legend>
                <Case id="vio-q-c".to_string() libelle="Confidentialité (des données ont été vues ou emportées)".to_string() coche=c/>
                <Case id="vio-q-i".to_string() libelle="Intégrité (des données ont été modifiées)".to_string() coche=i/>
                <Case id="vio-q-d".to_string() libelle="Disponibilité (des données ont été perdues ou inaccessibles)".to_string() coche=d/>
            </fieldset>
            <fieldset class="rgpd-choix">
                <legend>"Données touchées"</legend>
                {move || cases.get_value().into_iter().map(|(code, lib, s)| {
                    view! { <Case id=format!("vio-q-cat-{code}") libelle=lib.to_string() coche=s/> }
                }).collect_view()}
            </fieldset>
            <div class="vio-ligne">
                <ChampTexte id="vio-q-personnes".to_string() libelle="Nombre de personnes".to_string()
                    valeur=personnes inputmode="numeric".to_string()/>
                <Case id="vio-q-estime".to_string() libelle="Nombre estimé".to_string() coche=estime/>
                <Case id="vio-q-mineurs".to_string() libelle="Des mineurs sont concernés".to_string() coche=mineurs/>
            </div>
            <ZoneTexte id="vio-q-consequences".to_string() libelle="Conséquences probables".to_string() valeur=consequences/>
            <Bouton variante=Variante::Primaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                on_click=Callback::new(envoyer)>
                "Enregistrer la qualification"
            </Bouton>
        </section>
    }
}

#[component]
fn Risque(v: ViolationDto, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let id = v.id;
    let close = v.etat == "close";
    let avis = RwSignal::new(String::new());
    let risque = RwSignal::new(v.risque_propose.clone().unwrap_or_else(|| "avere".into()));
    let motif = RwSignal::new(String::new());
    let options: StoredValue<Vec<(String, String)>> = StoredValue::new(
        RISQUES
            .iter()
            .map(|r| ((*r).to_string(), libelle_risque(r).to_string()))
            .collect(),
    );
    let propose = v
        .risque_propose
        .clone()
        .map(|r| libelle_risque(&r).to_string());
    let raisons = v.raisons_proposition.clone();
    let decide = v.risque.clone().map(|r| {
        format!(
            "{} — {}",
            libelle_risque(&r),
            v.risque_motif.clone().unwrap_or_default()
        )
    });
    let ton = ton_risque(v.risque.as_deref());
    let avis_actuel = v.avis_dpo.clone();
    let decidable = v.qualifiee_le.is_some() && v.avis_dpo.is_some() && !close;
    let donner_avis = move |_| {
        let b = TexteBody {
            texte: avis.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_avis_dpo(id, &b)
                .await
                .map(|_| "Avis du DPO enregistré.".to_string())
        });
    };
    let decider = move |_| {
        let b = RisqueBody {
            risque: risque.get(),
            motif: motif.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_decider_risque(id, &b)
                .await
                .map(|v| {
                    format!(
                        "{} décidé.",
                        libelle_risque(v.risque.as_deref().unwrap_or(""))
                    )
                })
        });
    };
    view! {
        <section class="classe-section" aria-labelledby="vio-risque">
            <h2 id="vio-risque">"Risque pour les personnes"</h2>
            {match propose {
                Some(p) => view! {
                    <p>{format!("Proposé par la grille : {p}.")}</p>
                    <ul>{raisons.into_iter().map(|r| view! { <li>{r}</li> }).collect_view()}</ul>
                }.into_any(),
                None => view! { <p class="texte-attenue">"Qualifiez d'abord : la grille proposera un niveau de risque."</p> }.into_any(),
            }}
            {avis_actuel.map(|a| view! { <p>{format!("Avis du DPO : {a}")}</p> })}
            {decide.map(|d| view! { <p><Badge libelle=d ton=ton/></p> })}
            {move || (!close && moteur.get().peut_gerer_violations()).then(|| view! {
                <ZoneTexte id="vio-r-avis".to_string() libelle="Avis du DPO".to_string() valeur=avis
                    aide="Le DPO donne son avis avant que la direction décide.".to_string()/>
                <Bouton variante=Variante::Secondaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(donner_avis)>
                    "Enregistrer l'avis"
                </Bouton>
            })}
            {move || (decidable && moteur.get().peut_decider_violations()).then(|| view! {
                <Selection id="vio-r-risque".to_string() libelle="Risque décidé".to_string() valeur=risque options=options.get_value()/>
                <ChampTexte id="vio-r-motif".to_string() libelle="Motif de la décision".to_string() valeur=motif requis=true/>
                <Bouton variante=Variante::Primaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(decider)>
                    "Décider"
                </Bouton>
            })}
        </section>
    }
}

#[component]
fn NotificationCnil(v: ViolationDto, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let id = v.id;
    let close = v.etat == "close";
    let echeance = v.echeance_cnil_le;
    let notifiee = v.notifiee_cnil_le.is_some();
    let non_notifiee = v.non_notification_motif.clone();
    let faible = v.risque.as_deref() == Some("faible");
    let brouillon = RwSignal::new(None::<Result<BrouillonCnilDto, String>>);
    let date = RwSignal::new(String::new());
    let heure = RwSignal::new(String::new());
    let reference = RwSignal::new(String::new());
    let retard = RwSignal::new(String::new());
    let resume = RwSignal::new(String::new());
    let justification = RwSignal::new(String::new());

    let voir_brouillon = move |_| {
        leptos::task::spawn_local(async move {
            let r = session::client()
                .violations_brouillon_cnil(id)
                .await
                .map_err(crate::noyau::ecriture::message_echec);
            brouillon.set(Some(r));
        });
    };
    let instant = move || -> Result<DateTime<Utc>, String> {
        match instant_saisi(&date.get(), &heure.get()) {
            Ok(Some(i)) => Ok(i),
            Ok(None) => Err("Indiquez la date de la notification.".into()),
            Err(m) => Err(m.into()),
        }
    };
    let notifier = move |_| {
        let le = match instant() {
            Ok(i) => i,
            Err(m) => return ecriture.erreur.set(Some(m)),
        };
        if retard_a_motiver(echeance, le) && retard.get().trim().is_empty() {
            return ecriture.erreur.set(Some(
                "Notification après 72 h : motivez le retard (art. 33.1).".into(),
            ));
        }
        let b = NotificationCnilBody {
            notifiee_le: le,
            reference_cnil: reference.get().trim().to_string(),
            retard_motif: vide_si_blanc(retard.get()),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_notifier_cnil(id, &b)
                .await
                .map(|_| "Notification à la CNIL enregistrée.".to_string())
        });
    };
    let completer = move |_| {
        let le = match instant() {
            Ok(i) => i,
            Err(m) => return ecriture.erreur.set(Some(m)),
        };
        let b = ComplementCnilBody {
            notifiee_le: le,
            reference_cnil: reference.get().trim().to_string(),
            resume: resume.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_complement_cnil(id, &b)
                .await
                .map(|_| "Notification complémentaire enregistrée.".to_string())
        });
    };
    let justifier = move |_| {
        let b = TexteBody {
            texte: justification.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_non_notification(id, &b)
                .await
                .map(|_| "Non-notification justifiée au registre.".to_string())
        });
    };
    let gerer = move || !close && moteur.get().peut_gerer_violations();
    let decider = move || !close && moteur.get().peut_decider_violations();
    view! {
        <section class="classe-section" aria-labelledby="vio-cnil">
            <h2 id="vio-cnil">"Notification à la CNIL"</h2>
            <p class="texte-attenue">"La notification se fait sur le téléservice de la CNIL : ScolIA prépare le brouillon, vous le recopiez, puis vous saisissez ici la date et la référence."</p>
            {non_notifiee.map(|m| view! { <p>{format!("Non notifiée : {m}")}</p> })}
            <Bouton variante=Variante::Secondaire on_click=Callback::new(voir_brouillon)>"Préparer le brouillon"</Bouton>
            {move || brouillon.get().map(|b| match b {
                Err(m) => view! { <Alerte niveau=Niveau::Erreur titre=m/> }.into_any(),
                Ok(b) => view! { <Brouillon b=b/> }.into_any(),
            })}
            {move || gerer().then(|| view! {
                <div class="vio-ligne">
                    <ChampTexte id="vio-n-date".to_string() libelle="Notifiée le".to_string() valeur=date genre="date"/>
                    <ChampTexte id="vio-n-heure".to_string() libelle="À".to_string() valeur=heure genre="time"/>
                    <ChampTexte id="vio-n-ref".to_string() libelle="Référence donnée par la CNIL".to_string() valeur=reference/>
                </div>
                {(!notifiee).then(|| view! {
                    <ZoneTexte id="vio-n-retard".to_string() libelle="Motif du retard (au-delà de 72 h)".to_string() valeur=retard/>
                    <Bouton variante=Variante::Primaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                        on_click=Callback::new(notifier)>
                        "Enregistrer la notification"
                    </Bouton>
                })}
                {notifiee.then(|| view! {
                    <ZoneTexte id="vio-n-resume".to_string() libelle="Ce que la notification complémentaire ajoute".to_string() valeur=resume/>
                    <Bouton variante=Variante::Secondaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                        on_click=Callback::new(completer)>
                        "Enregistrer une notification complémentaire"
                    </Bouton>
                })}
            })}
            {move || (decider() && faible && !notifiee).then(|| view! {
                <ZoneTexte id="vio-n-justif".to_string() libelle="Pourquoi la CNIL n'est pas notifiée (risque faible)".to_string()
                    valeur=justification aide="La justification reste au registre (art. 33.5).".to_string()/>
                <Bouton variante=Variante::Secondaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(justifier)>
                    "Justifier la non-notification"
                </Bouton>
            })}
        </section>
    }
}

#[component]
fn Brouillon(b: BrouillonCnilDto) -> impl IntoView {
    let a_completer = b.a_completer.clone();
    view! {
        <div class="vio-brouillon">
            <h3>{format!("Brouillon pour {}", b.reference)}</h3>
            {(!a_completer.is_empty()).then(|| view! {
                <Alerte niveau=Niveau::Avertissement titre="À compléter avant de notifier".to_string()>
                    <ul>{a_completer.into_iter().map(|r| view! { <li>{r}</li> }).collect_view()}</ul>
                </Alerte>
            })}
            <dl class="rgpd-definitions">
                {b.rubriques.into_iter().map(|r| view! {
                    <dt>{r.titre}</dt>
                    <dd>{r.texte}</dd>
                }).collect_view()}
            </dl>
        </div>
    }
}

#[component]
fn Personnes(v: ViolationDto, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let id = v.id;
    let close = v.etat == "close";
    let informees = v.personnes_informees_le.map(|le| {
        format!(
            "{} destinataire(s) informé(s) le {}.",
            v.personnes_informees_nombre.unwrap_or(0),
            date_heure_paris(le)
        )
    });
    let exception = v.personnes_non_information_motif.clone();
    let risque_eleve_ou_avere = matches!(v.risque.as_deref(), Some("eleve") | Some("avere"));
    let origine = RwSignal::new("eleves".to_string());
    let ids = RwSignal::new(String::new());
    let apercu = RwSignal::new(None::<Result<ApercuInformationDto, String>>);
    let message = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let origines = StoredValue::new(vec![
        ("eleves".to_string(), "Des élèves".to_string()),
        ("classe".to_string(), "Une classe entière".to_string()),
        (
            "comptes".to_string(),
            "Des comptes (personnels, familles)".to_string(),
        ),
    ]);
    let calculer = move |_| match corps_personnes(&origine.get(), &ids.get()) {
        Err(m) => ecriture.erreur.set(Some(m.into())),
        Ok(b) => {
            apercu.set(None);
            ecriture.lancer(async move {
                session::client()
                    .violations_calculer_personnes(id, &b)
                    .await
                    .map(|v| {
                        format!(
                            "{} personne(s) concernée(s).",
                            v.personnes_nombre.unwrap_or(0)
                        )
                    })
            })
        }
    };
    let voir_apercu = move |_| {
        leptos::task::spawn_local(async move {
            let r = session::client()
                .violations_apercu_information(id)
                .await
                .map_err(crate::noyau::ecriture::message_echec);
            apercu.set(Some(r));
        });
    };
    let informer = move |_| {
        let Some(Ok(a)) = apercu.get() else {
            return ecriture
                .erreur
                .set(Some("Relisez d'abord l'aperçu des destinataires.".into()));
        };
        if !message_information_recevable(&message.get()) {
            return ecriture.erreur.set(Some(
                "Message : 20 à 4 000 caractères, en langage clair.".into(),
            ));
        }
        let b = InformationBody {
            message: message.get().trim().to_string(),
            confirmation: a.destinataires,
        };
        ecriture.lancer(async move {
            session::client()
                .violations_informer(id, &b)
                .await
                .map(|v| {
                    format!(
                        "{} destinataire(s) informé(s).",
                        v.personnes_informees_nombre.unwrap_or(0)
                    )
                })
        });
    };
    let excepter = move |_| {
        let b = TexteBody {
            texte: motif.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_non_information(id, &b)
                .await
                .map(|_| "Exception à l'information motivée.".to_string())
        });
    };
    let deja = informees.is_some();
    let gerer = move || !close && !deja && moteur.get().peut_gerer_violations();
    let decider =
        move || !close && !deja && risque_eleve_ou_avere && moteur.get().peut_decider_violations();
    let eleve = v.risque.as_deref() == Some("eleve");
    view! {
        <section class="classe-section" aria-labelledby="vio-personnes">
            <h2 id="vio-personnes">"Personnes concernées"</h2>
            {informees.map(|t| view! { <p><Badge libelle=t ton=TonEtat::Ok/></p> })}
            {exception.map(|m| view! { <p>{format!("Non informées (art. 34.3) : {m}")}</p> })}
            {move || gerer().then(|| view! {
                <Selection id="vio-p-origine".to_string() libelle="Qui est concerné".to_string() valeur=origine options=origines.get_value()/>
                <ZoneTexte id="vio-p-ids".to_string() libelle="Identifiants (un par ligne)".to_string() valeur=ids
                    aide="Les responsables légaux de chaque élève sont retrouvés par le serveur ; l'élève lui-même à partir de 15 ans.".to_string()/>
                <Bouton variante=Variante::Secondaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(calculer)>
                    "Calculer les personnes"
                </Bouton>
            })}
            {move || decider().then(|| view! {
                <h3>"Informer les personnes (art. 34)"</h3>
                <Bouton variante=Variante::Secondaire on_click=Callback::new(voir_apercu)>"Voir l'aperçu des destinataires"</Bouton>
                {move || apercu.get().map(|a| match a {
                    Err(m) => view! { <Alerte niveau=Niveau::Erreur titre=m/> }.into_any(),
                    Ok(a) => view! {
                        <Alerte niveau=Niveau::Info titre=format!(
                            "{} personne(s), {} destinataire(s)", a.personnes, a.destinataires)>
                            <p>{format!("{} mineur(s) ; {} personne(s) sans destinataire joignable (à informer par courrier).",
                                a.mineurs, a.personnes_sans_destinataire)}</p>
                        </Alerte>
                    }.into_any(),
                })}
                <ZoneTexte id="vio-p-message".to_string() libelle="Message aux personnes, en langage clair".to_string()
                    valeur=message
                    aide="Ce qui s'est passé, les conséquences possibles, ce qui est fait, ce qu'elles peuvent faire. Le contact du DPO est ajouté.".to_string()/>
                <Bouton variante=Variante::Primaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    desactive=Signal::derive(move || !matches!(apercu.get(), Some(Ok(_))))
                    on_click=Callback::new(informer)>
                    "Envoyer l'information aux destinataires de l'aperçu"
                </Bouton>
            })}
            {move || (decider() && eleve).then(|| view! {
                <ZoneTexte id="vio-p-exception".to_string()
                    libelle="Ou : pourquoi les personnes ne sont pas informées (chiffrement, mesures prises, effort disproportionné)".to_string()
                    valeur=motif/>
                <Bouton variante=Variante::Secondaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(excepter)>
                    "Motiver l'exception"
                </Bouton>
            })}
        </section>
    }
}

#[component]
fn AjoutMesure(id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let nature = RwSignal::new("prise".to_string());
    let texte = RwSignal::new(String::new());
    let natures = vec![
        ("prise".to_string(), "Mesure prise".to_string()),
        ("envisagee".to_string(), "Mesure envisagée".to_string()),
    ];
    let ajouter = move |_| {
        let b = MesureBody {
            nature: nature.get(),
            texte: texte.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_ajouter_mesure(id, &b)
                .await
                .map(|_| {
                    texte.set(String::new());
                    "Mesure ajoutée.".to_string()
                })
        });
    };
    move || {
        moteur.get().peut_gerer_violations().then(|| {
            let natures = natures.clone();
            view! {
                <div class="vio-ligne">
                    <Selection id="vio-m-nature".to_string() libelle="Nature".to_string() valeur=nature options=natures/>
                    <ChampTexte id="vio-m-texte".to_string() libelle="Mesure".to_string() valeur=texte/>
                </div>
                <Bouton variante=Variante::Secondaire en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(ajouter)>
                    "Ajouter la mesure"
                </Bouton>
            }
        })
    }
}

#[component]
fn Cloture(id: Uuid, manque: Vec<String>, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let retour = RwSignal::new(String::new());
    let possible = manque.is_empty();
    let clore = move |_| {
        let b = TexteBody {
            texte: retour.get().trim().to_string(),
        };
        ecriture.lancer(async move {
            session::client()
                .violations_clore(id, &b)
                .await
                .map(|_| "Violation close.".to_string())
        });
    };
    let manque = StoredValue::new(manque);
    move || {
        moteur.get().peut_gerer_violations().then(|| view! {
            <section class="classe-section" aria-labelledby="vio-cloture">
                <h2 id="vio-cloture">"Clôture"</h2>
                {(!possible).then(|| view! {
                    <Alerte niveau=Niveau::Info titre="Avant de clore".to_string()>
                        <ul>{manque.get_value().into_iter().map(|m| view! { <li>{m}</li> }).collect_view()}</ul>
                    </Alerte>
                })}
                <ZoneTexte id="vio-c-retour".to_string() libelle="Retour d'expérience : ce qui change pour que cela ne se reproduise pas".to_string()
                    valeur=retour/>
                <Bouton variante=Variante::Primaire desactive=Signal::derive(move || !possible)
                    en_cours=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(clore)>
                    "Clore"
                </Bouton>
            </section>
        })
    }
}
