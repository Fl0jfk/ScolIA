//! Déclarer une absence (ABP-100, 101, 107, 109, 114, 115, 117) : trois choses à dire (quand, quoi, une
//! pièce si on l'a), puis « mes séances touchées ». Le motif libre n'existe pas : une **nature** dans une
//! liste courte, ou « je préciserai plus tard » ; le texte libre est réservé aux natures non sensibles.
//!
//! Le serveur calcule l'impact, le circuit et les chevauchements (proposition seulement, jamais un
//! blocage) ; l'écran les affiche après coup.

use ent_types::absences_personnels::{AbsenceDto, DeposerPieceBody, ImpactDto, NatureDto};
use ent_types::personnels::AgentResumeDto;
use ent_web_api::{depot_fichier, ApiError};
use ent_web_ui::{
    Alerte, Bouton, Case, ChampTexte, EtatVide, GroupeChoix, LienBouton, Niveau, ResumeErreurs,
    Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::GardePersonnel;
use super::fichiers::{lire_piece_choisie, Lecture, PieceChoisie};
use super::modele::{
    composer_declaration, libelle_etat_impact, libelle_famille, libelle_piece_attendue,
    libelle_seance, libelle_traitement, resume_declaration, ErreursSaisie, SaisieDeclaration,
    TEXTE_LIBRE_MAX, TRAITEMENTS, TYPES_PIECE,
};
use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageDeclarerAbsence() -> impl IntoView {
    view! {
        <Title text="Déclarer une absence"/>
        <GardePersonnel
            titre="Déclarer une absence"
            sous_titre="Quand, quoi, et une pièce si vous l'avez. Votre motif reste confidentiel."
        >
            <Chargement/>
        </GardePersonnel>
    }
}

#[component]
fn Chargement() -> impl IntoView {
    let natures = LocalResource::new(|| async { session::client().absp_natures().await });
    view! {
        {move || match natures.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement du formulaire…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| natures.refetch())/> }.into_any(),
            Some(Ok(n)) if n.is_empty() => view! {
                <EtatVide
                    titre="Aucune nature d'absence disponible"
                    message="Votre engagement n'a pas de nature d'absence active : contactez le secrétariat."
                />
            }
            .into_any(),
            Some(Ok(n)) => view! { <Formulaire natures=n/> }.into_any(),
        }}
    }
}

/// Ce que montre l'écran après une déclaration réussie.
#[derive(Clone, Debug)]
struct Resultat {
    absences: Vec<AbsenceDto>,
    chevauchements: Vec<Uuid>,
    impacts: Vec<ImpactDto>,
    echec_piece: Option<String>,
    pour_autrui: bool,
}

#[derive(Clone, Copy)]
struct AutrePeriode {
    cle: u32,
    debut: RwSignal<String>,
    fin: RwSignal<String>,
}

#[component]
fn Formulaire(natures: Vec<NatureDto>) -> impl IntoView {
    let droits = use_droits();
    let moteur = use_droits_moteur();
    let resultat = RwSignal::new(None::<Resultat>);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));

    let collegue = RwSignal::new(String::new());
    let nature = RwSignal::new(String::new());
    let quand = RwSignal::new("aujourdhui".to_string());
    let debut = RwSignal::new(String::new());
    let fin = RwSignal::new(String::new());
    let partie = RwSignal::new("journee".to_string());
    let heure_debut = RwSignal::new(String::new());
    let heure_fin = RwSignal::new(String::new());
    let fin_provisoire = RwSignal::new(false);
    let texte = RwSignal::new(String::new());
    let preference = RwSignal::new(String::new());
    let periodes = RwSignal::new(Vec::<AutrePeriode>::new());
    let prochaine_cle = RwSignal::new(0u32);
    let piece = RwSignal::new(None::<PieceChoisie>);
    let type_piece = RwSignal::new("justificatif".to_string());
    let volet_exclu = RwSignal::new(false);
    let erreur_piece = RwSignal::new(None::<String>);
    let champ_piece = NodeRef::<leptos::html::Input>::new();
    let erreurs = RwSignal::new(ErreursSaisie::new());
    let resume = RwSignal::new(String::new());

    let natures = StoredValue::new(natures);
    let nature_choisie = move || {
        let code = nature.get();
        natures.with_value(|l| l.iter().find(|n| n.code == code).cloned())
    };
    let texte_permis = Memo::new(move |_| {
        nature_choisie()
            .map(|n| n.texte_libre_permis)
            .unwrap_or(false)
    });
    let piece_attendue = Memo::new(move |_| {
        nature_choisie()
            .map(|n| libelle_piece_attendue(&n.piece_attendue))
            .unwrap_or("")
    });
    let est_retard = Memo::new(move |_| nature.get() == "retard");

    let options_nature = {
        let mut o = vec![(String::new(), "Je préciserai plus tard".to_string())];
        natures.with_value(|l| {
            o.extend(l.iter().map(|n| {
                (
                    n.code.clone(),
                    format!("{} — {}", n.libelle, libelle_famille(&n.famille)),
                )
            }))
        });
        o
    };
    let options_preference = {
        let mut o = vec![(String::new(), "Aucune préférence".to_string())];
        o.extend(
            TRAITEMENTS
                .iter()
                .map(|c| ((*c).to_string(), libelle_traitement(c))),
        );
        o
    };
    let options_type_piece: Vec<(String, String)> = TYPES_PIECE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let options_type_piece = StoredValue::new(options_type_piece);

    let saisie = move || SaisieDeclaration {
        collegue: collegue.get_untracked(),
        nature: nature.get_untracked(),
        quand: quand.get_untracked(),
        debut: debut.get_untracked(),
        fin: fin.get_untracked(),
        partie: partie.get_untracked(),
        heure_debut: heure_debut.get_untracked(),
        heure_fin: heure_fin.get_untracked(),
        autres_periodes: periodes
            .get_untracked()
            .iter()
            .map(|p| (p.debut.get_untracked(), p.fin.get_untracked()))
            .collect(),
        fin_provisoire: fin_provisoire.get_untracked(),
        texte_libre: texte.get_untracked(),
        preference: preference.get_untracked(),
    };

    // Résumé avant l'envoi : calculé après l'hydratation (la date du jour n'est jamais dans le HTML serveur).
    Effect::new(move |_| {
        collegue.track();
        nature.track();
        quand.track();
        debut.track();
        fin.track();
        partie.track();
        heure_debut.track();
        heure_fin.track();
        fin_provisoire.track();
        texte.track();
        preference.track();
        for p in periodes.get() {
            p.debut.track();
            p.fin.track();
        }
        let n = nature_choisie();
        resume.set(
            match composer_declaration(&saisie(), aujourdhui(), n.as_ref()) {
                Ok(b) => resume_declaration(&b, n.as_ref()),
                Err(_) => String::new(),
            },
        );
    });

    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };

    let choisir_piece = move |_| {
        let Some(input) = champ_piece.get() else {
            return;
        };
        spawn_local(async move {
            match lire_piece_choisie(&input).await {
                Lecture::Aucun => piece.set(None),
                Lecture::Fichier(f) => {
                    erreur_piece.set(None);
                    piece.set(Some(f));
                }
                Lecture::Refuse(m) => {
                    input.set_value("");
                    piece.set(None);
                    erreur_piece.set(Some(m));
                }
            }
        });
    };

    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let n = nature_choisie();
        let corps = match composer_declaration(&saisie(), aujourdhui(), n.as_ref()) {
            Ok(c) => c,
            Err(e) => {
                erreurs.set(e);
                return;
            }
        };
        let mut e_piece: ErreursSaisie = Vec::new();
        if piece.get_untracked().is_some()
            && type_piece.get_untracked() == "arret"
            && !volet_exclu.get_untracked()
        {
            e_piece.push((
                "decl-volet".into(),
                "Confirmez n'avoir joint que les volets 2 et 3 : le volet 1 contient le diagnostic et ne doit jamais être déposé.".into(),
            ));
        }
        if !e_piece.is_empty() {
            erreurs.set(e_piece);
            return;
        }
        erreurs.set(Vec::new());
        let fichier = piece.get_untracked();
        let type_p = type_piece.get_untracked();
        let volet = volet_exclu.get_untracked();
        let pour_autrui = corps.agent_id.is_some();
        ecriture.lancer(async move {
            let client = session::client();
            let d = client.absp_declarer(&corps).await?;
            let mut echec_piece = None;
            if let (Some(f), Some(a)) = (fichier, d.absences.first()) {
                let r = client
                    .absp_deposer_piece(
                        a.id,
                        &DeposerPieceBody {
                            type_piece: type_p,
                            fichier: depot_fichier(&f.nom, &f.mime, &f.octets),
                            volet_1_exclu: volet,
                        },
                    )
                    .await;
                if let Err(e) = r {
                    echec_piece = Some(message_echec(e));
                }
            }
            let mut impacts = Vec::new();
            for a in &d.absences {
                // L'impact est une information en plus : son échec n'annule pas la déclaration.
                if let Ok(mut i) = client.absp_impact(a.id).await {
                    impacts.append(&mut i);
                }
            }
            let message = if d.absences.len() > 1 {
                format!(
                    "{} absences enregistrées (une par engagement).",
                    d.absences.len()
                )
            } else {
                "Absence enregistrée.".to_string()
            };
            resultat.set(Some(Resultat {
                absences: d.absences,
                chevauchements: d.chevauchements,
                impacts,
                echec_piece,
                pour_autrui,
            }));
            Ok::<String, ApiError>(message)
        });
    };

    let ajouter_periode = move |_| {
        let cle = prochaine_cle.get_untracked();
        prochaine_cle.set(cle + 1);
        periodes.update(|l| {
            l.push(AutrePeriode {
                cle,
                debut: RwSignal::new(String::new()),
                fin: RwSignal::new(String::new()),
            })
        });
    };

    // « Déclarer une autre absence » repart d'un formulaire vierge : ni pièce, ni attestation « volets 2 et 3 »
    // (une confirmation de santé ne se reporte jamais d'une déclaration à l'autre).
    let remettre = Callback::new(move |()| {
        collegue.set(String::new());
        nature.set(String::new());
        quand.set("aujourdhui".to_string());
        debut.set(String::new());
        fin.set(String::new());
        partie.set("journee".to_string());
        heure_debut.set(String::new());
        heure_fin.set(String::new());
        fin_provisoire.set(false);
        texte.set(String::new());
        preference.set(String::new());
        periodes.set(Vec::new());
        piece.set(None);
        type_piece.set("justificatif".to_string());
        volet_exclu.set(false);
        erreur_piece.set(None);
        erreurs.set(Vec::new());
        resultat.set(None);
    });
    // Un texte libre masqué (nature qui ne le permet plus) n'est ni envoyé ni bloquant.
    Effect::new(move |_| {
        if !texte_permis.get() {
            texte.set(String::new());
        }
    });
    let peut_pour_autrui = move || moteur.get().a_etablissement("personnels.absences.declarer");
    let peut_cahier = move || droits.get().cahier_ecrire;

    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match resultat.get() {
            Some(r) => view! { <Confirme r=r cahier=peut_cahier() autre=remettre/> }.into_any(),
            None => view! {
                <form class="absp-formulaire" on:submit=envoyer novalidate=true>
                    <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
                    {move || peut_pour_autrui().then(|| view! { <ChoixCollegue valeur=collegue/> })}

                    <GroupeChoix
                        nom="decl-quand"
                        legende="Quand ?"
                        valeur=quand
                        options=vec![
                            ("aujourdhui".to_string(), "Aujourd'hui".to_string()),
                            ("demain".to_string(), "Demain".to_string()),
                            ("dates".to_string(), "À des dates précises".to_string()),
                        ]
                    />
                    {move || (quand.get() == "dates").then(|| view! {
                        <div class="absp-dates">
                            <ChampTexte id="decl-debut" libelle="Premier jour" genre="date" valeur=debut requis=true erreur=erreur_de("decl-debut")/>
                            <ChampTexte
                                id="decl-fin"
                                libelle="Dernier jour (inclus)"
                                genre="date"
                                valeur=fin
                                aide="Laissez vide pour une seule journée."
                                erreur=erreur_de("decl-fin")
                            />
                        </div>
                        <Case id="decl-provisoire" libelle="Je prolongerai peut-être : la fin est une estimation" coche=fin_provisoire/>
                    })}

                    <GroupeChoix
                        nom="decl-partie"
                        legende="Quelle partie de la journée ?"
                        valeur=partie
                        options=vec![
                            ("journee".to_string(), "Toute la journée".to_string()),
                            ("matin".to_string(), "Le matin".to_string()),
                            ("apres_midi".to_string(), "L'après-midi".to_string()),
                            ("heures".to_string(), "À des heures précises".to_string()),
                        ]
                    />
                    {move || erreur_de("decl-partie-journee").get().map(|m| view! { <p class="ui-champ__erreur" id="decl-partie-erreur">{m}</p> })}
                    {move || (partie.get() == "heures").then(|| view! {
                        <div class="absp-dates">
                            <ChampTexte id="decl-heure-debut" libelle="De" genre="time" valeur=heure_debut requis=true erreur=erreur_de("decl-heure-debut")/>
                            <ChampTexte id="decl-heure-fin" libelle="À" genre="time" valeur=heure_fin requis=true erreur=erreur_de("decl-heure-fin")/>
                        </div>
                    })}

                    <fieldset class="absp-periodes">
                        <legend>"Absence en plusieurs périodes"</legend>
                        <p class="texte-attenue">
                            "Une seule déclaration pour des jours non consécutifs (par exemple tous les lundis du mois)."
                        </p>
                        <For
                            each=move || periodes.get()
                            key=|p| p.cle
                            children=move |p| {
                                let numero = p.cle + 1;
                                view! {
                                    <div class="absp-dates absp-dates--ligne">
                                        <ChampTexte
                                            id=format!("decl-autre-debut-{}", p.cle)
                                            libelle=format!("Période {numero} : premier jour")
                                            genre="date"
                                            valeur=p.debut
                                        />
                                        <ChampTexte
                                            id=format!("decl-autre-fin-{}", p.cle)
                                            libelle=format!("Période {numero} : dernier jour")
                                            genre="date"
                                            valeur=p.fin
                                        />
                                        <Bouton
                                            variante=Variante::Secondaire
                                            taille=Taille::Compacte
                                            libelle_accessible=format!("Retirer la période {numero}")
                                            on_click=Callback::new(move |_| periodes.update(|l| l.retain(|x| x.cle != p.cle)))
                                        >
                                            "Retirer"
                                        </Bouton>
                                    </div>
                                }
                            }
                        />
                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |()| ajouter_periode(()))>
                            "Ajouter une période"
                        </Bouton>
                    </fieldset>

                    <Selection
                        id="decl-nature"
                        libelle="Nature de l'absence"
                        valeur=nature
                        options=options_nature.clone()
                        aide="Seules les personnes qui instruisent votre dossier voient cette nature."
                        erreur=erreur_de("decl-nature")
                    />
                    {move || {
                        let p = piece_attendue.get();
                        (!p.is_empty()).then(|| view! {
                            <p class="texte-attenue">{format!("Pièce attendue : {p}.")}</p>
                        })
                    }}
                    {move || est_retard.get().then(|| view! {
                        <p class="texte-attenue">"Un retard dure moins de 4 heures, le même jour : au-delà, il est requalifié en absence."</p>
                    })}
                    {move || texte_permis.get().then(|| view! {
                        <ZoneTexte
                            id="decl-texte"
                            libelle="Précision (facultatif)"
                            valeur=texte
                            aide=format!("{TEXTE_LIBRE_MAX} caractères au plus. Ne mentionnez aucune information de santé.")
                            erreur=erreur_de("decl-texte")
                        />
                    })}
                    <Selection
                        id="decl-preference"
                        libelle="Traitement des heures souhaité (facultatif)"
                        valeur=preference
                        options=options_preference.clone()
                        aide="Proposé à la direction, jamais imposé."
                        erreur=erreur_de("decl-preference")
                    />

                    <fieldset class="absp-piece">
                        <legend>"Pièce justificative (facultatif)"</legend>
                        <div class="ui-champ">
                            <label class="ui-champ__libelle" for="decl-piece">"Joindre un fichier"</label>
                            <input
                                id="decl-piece"
                                class="ui-champ__controle"
                                type="file"
                                node_ref=champ_piece
                                aria-describedby="decl-piece-aide"
                                on:change=choisir_piece
                            />
                            <p id="decl-piece-aide" class="ui-champ__aide">
                                "PDF ou image, 10 Mo au maximum. Vous pourrez aussi la déposer plus tard depuis la fiche de l'absence."
                            </p>
                            {move || erreur_piece.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                        </div>
                        {move || piece.get().map(|f| view! {
                            <p class="texte-attenue">{format!("Un fichier est prêt ({} Ko).", f.octets.len() / 1024 + 1)}</p>
                            <Selection id="decl-type-piece" libelle="Type de pièce" valeur=type_piece options=options_type_piece.get_value()/>
                            {move || (type_piece.get() == "arret").then(|| view! {
                                <Case
                                    id="decl-volet"
                                    libelle="Je n'ai joint que les volets 2 et 3 de l'arrêt, jamais le volet 1 (diagnostic)"
                                    coche=volet_exclu
                                />
                                {move || erreur_de("decl-volet").get().map(|m| view! { <p class="ui-champ__erreur">{m}</p> })}
                            })}
                        })}
                    </fieldset>

                    {move || {
                        let r = resume.get();
                        (!r.is_empty()).then(|| view! {
                            <p class="absp-resume" role="status">
                                <strong>"Vous allez déclarer : "</strong>{r}
                            </p>
                        })
                    }}
                    <div class="absp-actions">
                        <Bouton soumettre=true en_cours=ecriture.en_cours>"Déclarer l'absence"</Bouton>
                    </div>
                </form>
            }
            .into_any(),
        }}
    }
}

/// Choix de la personne absente quand on déclare pour un collègue (droit `declarer` sur l'établissement).
#[component]
fn ChoixCollegue(valeur: RwSignal<String>) -> impl IntoView {
    let annuaire = LocalResource::new(|| async {
        let mut a = session::client().absp_annuaire().await?;
        a.retain(|x| x.etat == "actif");
        a.sort_by(|x, y| x.nom_affiche.cmp(&y.nom_affiche));
        Ok::<Vec<AgentResumeDto>, ApiError>(a)
    });
    view! {
        {move || match annuaire.get() {
            None => view! { <Squelette lignes=1 libelle="Chargement de l'annuaire…"/> }.into_any(),
            Some(Err(_)) => view! {
                <Alerte niveau=Niveau::Avertissement titre="L'annuaire n'a pas pu être chargé : vous ne pouvez déclarer que pour vous-même."/>
            }
            .into_any(),
            Some(Ok(a)) => {
                let mut o = vec![(String::new(), "Moi-même".to_string())];
                o.extend(a.into_iter().map(|x| (x.agent_id.to_string(), x.nom_affiche)));
                view! {
                    <Selection
                        id="decl-collegue"
                        libelle="Pour qui ?"
                        valeur=valeur
                        options=o
                        aide="La personne est prévenue et confirme. Les natures proposées sont celles de votre engagement : le serveur contrôle celles de la personne."
                    />
                }
                .into_any()
            }
        }}
    }
}

/// Écran de confirmation : l'absence, les chevauchements à vérifier, « mes séances touchées ».
#[component]
fn Confirme(r: Resultat, cahier: bool, autre: Callback<()>) -> impl IntoView {
    let premiere = r.absences.first().cloned();
    let href = premiere
        .as_ref()
        .map(|a| format!("/web/personnel/absences/{}", a.id))
        .unwrap_or_else(|| "/web/personnel/absences".to_string());
    let nb_impacts = r.impacts.len();
    let pour_autrui = r.pour_autrui;
    view! {
        <section class="absp-confirme" aria-labelledby="absp-confirme-titre">
            <h2 id="absp-confirme-titre" class="vs-section-titre">"C'est enregistré"</h2>
            {if pour_autrui {
                view! { <p>"La personne concernée est prévenue et doit confirmer son absence."</p> }.into_any()
            } else {
                view! { <p>"Votre absence est enregistrée. La direction et l'organisation des remplacements sont informées, sans connaître votre motif."</p> }.into_any()
            }}
            {r.echec_piece.clone().map(|m| view! {
                <Alerte niveau=Niveau::Avertissement titre="L'absence est déclarée, mais la pièce n'a pas pu être déposée.">
                    <p>{m}</p>
                    <p>"Vous pourrez la déposer depuis la fiche de l'absence."</p>
                </Alerte>
            })}
            {(!r.chevauchements.is_empty()).then(|| view! {
                <Alerte niveau=Niveau::Avertissement titre="Cette période recoupe une absence déjà déclarée.">
                    <p>"Vérifiez que ce n'est pas un doublon. Rien n'a été bloqué ; vous pouvez annuler l'une des deux depuis sa fiche."</p>
                    <ul>
                        {r.chevauchements.iter().map(|id| view! {
                            <li><a href=format!("/web/personnel/absences/{id}")>"Voir l'absence déjà déclarée"</a></li>
                        }).collect_view()}
                    </ul>
                </Alerte>
            })}
            <h3 class="absp-sous-titre">"Mes séances touchées"</h3>
            {if nb_impacts == 0 {
                view! { <p class="texte-attenue">"Aucune séance de l'emploi du temps n'est touchée par cette période."</p> }.into_any()
            } else {
                view! {
                    <ul class="absp-liste absp-liste--serree">
                        {r.impacts.iter().map(|i| view! {
                            <li class="absp-ligne">
                                <span>{libelle_seance(i)}</span>
                                <span class="texte-attenue">{libelle_etat_impact(&i.etat)}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            }}
            {(cahier && !pour_autrui && nb_impacts > 0).then(|| view! {
                <p>
                    "Déposez le travail prévu pour vos élèves dans "
                    <a href="/web/cahier/continuite">"la continuité pédagogique"</a>"."
                </p>
            })}
            <p class="absp-actions">
                <LienBouton href=href>"Ouvrir la fiche de l'absence"</LienBouton>
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| autre.run(()))>
                    "Déclarer une autre absence"
                </Bouton>
            </p>
        </section>
    }
}
