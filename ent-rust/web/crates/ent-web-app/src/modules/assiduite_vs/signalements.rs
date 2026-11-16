//! Signalements d'assiduité (socle 12, AS14e) : les élèves au seuil du mois, la préparation du
//! signalement (proposition, relecture du dossier pré-rempli), son suivi (validation, refus, trace de
//! la transmission) et les rectifications qui touchent un signalement déjà décidé (R-AS16, CA-40).
//!
//! Routes existantes : `GET /vs/decompte`, `GET /vs/signalements`, `POST /vs/signalements/proposer`,
//! `POST /vs/signalements/{id}/decider`, `GET /assiduite/mois/{mois}/rectifications`,
//! `POST /assiduite/rectifications/{id}/vue`. Le serveur décide des droits : proposer = CPE
//! (`assiduite.signaler`) ; décider = direction (`discipline.gerer`). ScolIA **n'envoie rien** à
//! l'académie (R-AS26) : « transmis » est la trace d'un geste fait par une personne.
//!
//! Ce qui n'existe pas encore côté serveur (lot AS9) est **dit à l'écran** et n'est pas simulé :
//! validation par passkey, dossier annuel d'événements, export par profil académique, note de
//! transmission (date, outil, personne).

use std::collections::HashMap;

use chrono::NaiveDate;
use ent_types::assiduite::RectificationDto;
use ent_types::discipline::{DecompteEleveDto, DecompteMoisDto, SignalementDto};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, Confirmation, EnTetePage, EtatVide, Niveau, Squelette,
    Statistique, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::dossier_modele::{
    decisions_signalement, franchit_le_seuil, libelle_mois, libelle_statut_signalement,
    lignes_dossier_signalement, lire_mois, phrase_rectification, resume_signalement,
    ton_statut_signalement, EtatSeuil,
};
use super::modele::libelle_periode_avis;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

/// Ce que l'écran liste comme « manquant » : le serveur n'a pas encore ces fonctions (lot AS9).
pub const MANQUE_AS9: [&str; 5] = [
    "Le dossier annuel d'événements (entretiens, commission éducative, référent, engagements) : seules les absences, contacts, courriers et rectifications sont rassemblés.",
    "La validation par la direction avec ré-authentification (passkey) : aujourd'hui, un clic de confirmation.",
    "L'export du signalement (PDF, CSV) selon le profil de l'académie.",
    "La trace de la transmission (date, outil utilisé, personne) : « Noter comme transmis » n'enregistre que l'état.",
    "Les profils académiques (niveaux et seuils de persistance, âge au-delà duquel le niveau pénal est désactivé).",
];

#[component]
pub fn PageSignalements() -> impl IntoView {
    // Le mois est posé après l'hydratation : aucune date dans le HTML rendu côté serveur.
    let mois_texte = RwSignal::new(String::new());
    Effect::new(move |_| {
        if mois_texte.get_untracked().is_empty() {
            mois_texte.set(aujourdhui().format("%Y-%m").to_string());
        }
    });
    let mois = Memo::new(move |_| lire_mois(&mois_texte.get()));
    view! {
        <Title text="Signalements d'assiduité"/>
        <EnTetePage
            surtitre="Vie scolaire".to_string()
            titre="Signalements d'assiduité".to_string()
            sous_titre="Élèves au seuil du mois, préparation, décision et suivi des signalements.".to_string()
        />
        <div class="jvs-barre">
            <ChampTexte id="sig-mois" libelle="Mois" valeur=mois_texte genre="month"/>
        </div>
        {move || match mois.get() {
            None => view! { <EtatVide titre="Mois invalide" message="Choisissez un mois."/> }.into_any(),
            Some(m) => view! { <ContenuMois mois=m/> }.into_any(),
        }}
    }
}

/// Noms connus par le décompte (les signalements et rectifications ne portent que l'identifiant).
pub fn noms_depuis_decompte(d: &DecompteMoisDto) -> HashMap<Uuid, String> {
    d.eleves
        .iter()
        .map(|e| (e.eleve_id, format!("{} {}", e.prenom, e.nom).trim().to_string()))
        .collect()
}

/// Élèves à suivre ce mois-ci : au moins une demi-journée non justifiée, ceux au seuil d'abord.
pub fn eleves_a_suivre(d: &DecompteMoisDto) -> Vec<DecompteEleveDto> {
    let mut v: Vec<DecompteEleveDto> = d
        .eleves
        .iter()
        .filter(|e| e.demi_journees_non_justifiees > 0)
        .cloned()
        .collect();
    v.sort_by(|a, b| {
        b.demi_journees_non_justifiees
            .cmp(&a.demi_journees_non_justifiees)
            .then_with(|| a.nom.cmp(&b.nom))
    });
    v
}

/// Rectifications du mois qui touchent un signalement, non encore vues, d'abord.
pub fn rectifications_a_traiter(v: &[RectificationDto]) -> Vec<RectificationDto> {
    let mut r: Vec<RectificationDto> = v.iter().filter(|r| r.touche_signalement).cloned().collect();
    r.sort_by_key(|r| (r.vue_par.is_some(), std::cmp::Reverse(r.cree_le)));
    r
}

#[component]
fn ContenuMois(mois: NaiveDate) -> impl IntoView {
    let decompte =
        LocalResource::new(move || async move { session::client().decompte_mensuel(mois).await });
    let signalements =
        LocalResource::new(move || async move { session::client().signalements(mois).await });
    let rectifs =
        LocalResource::new(move || async move { session::client().rectifications_mois(mois).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        decompte.refetch();
        signalements.refetch();
        rectifs.refetch();
    }));
    let moteur = use_droits_moteur();
    let confirmer_proposition = RwSignal::new(false);
    let proposer = Callback::new(move |_| {
        ecriture.lancer(async move {
            let l = session::client().proposer_signalements(mois).await?;
            Ok(format!("{} signalement(s) proposé(s) pour ce mois.", l.len()))
        });
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="sig-seuil">
            <h2 id="sig-seuil" class="vs-section-titre">{format!("Élèves à suivre en {}", libelle_mois(mois))}</h2>
            {move || match decompte.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement du décompte…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| decompte.refetch())/> }.into_any(),
                Some(Ok(d)) => {
                    let signalements_lus = signalements.get().and_then(Result::ok).unwrap_or_default();
                    view! { <ElevesASuivre decompte=d signalements=signalements_lus/> }.into_any()
                }
            }}
            {move || {
                let au_seuil = decompte
                    .get()
                    .and_then(Result::ok)
                    .map(|d| d.eleves.iter().filter(|e| e.seuil_atteint).count())
                    .unwrap_or(0);
                moteur.get().peut_signaler_assiduite().then(|| view! {
                    <p>
                        <Bouton
                            variante=Variante::Secondaire
                            en_cours=ecriture.en_cours
                            on_click=Callback::new(move |_| confirmer_proposition.set(true))
                        >
                            "Préparer les signalements du mois"
                        </Bouton>
                        <span class="texte-attenue">
                            {format!(" {au_seuil} élève(s) au seuil. Rien n'est envoyé : la proposition est ensuite relue et décidée.")}
                        </span>
                    </p>
                })
            }}
        </section>
        <section class="ra-section" aria-labelledby="sig-liste">
            <h2 id="sig-liste" class="vs-section-titre">"Signalements du mois"</h2>
            {move || match signalements.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des signalements…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| signalements.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun signalement" message="Aucun signalement n'est proposé pour ce mois."/>
                }.into_any(),
                Some(Ok(l)) => {
                    let noms = decompte.get().and_then(Result::ok).map(|d| noms_depuis_decompte(&d)).unwrap_or_default();
                    view! { <ListeSignalements signalements=l noms=noms ecriture=ecriture/> }.into_any()
                }
            }}
        </section>
        <section class="ra-section" aria-labelledby="sig-rectifs">
            <h2 id="sig-rectifs" class="vs-section-titre">"Rectifications après arrêt du mois"</h2>
            {move || match rectifs.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des rectifications…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! {
                    <Alerte niveau=Niveau::Info titre="Rectifications non accessibles">
                        <p>"Vos droits ne permettent pas de lire les rectifications."</p>
                    </Alerte>
                }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| rectifs.refetch())/> }.into_any(),
                Some(Ok(l)) => {
                    let noms = decompte.get().and_then(Result::ok).map(|d| noms_depuis_decompte(&d)).unwrap_or_default();
                    let seuil = decompte.get().and_then(Result::ok).map(|d| d.seuil_signalement).unwrap_or(0);
                    view! { <ListeRectifications rectifications=l noms=noms seuil=seuil ecriture=ecriture/> }.into_any()
                }
            }}
        </section>
        <section class="ra-section" aria-labelledby="sig-manque">
            <h2 id="sig-manque" class="vs-section-titre">"Ce que ScolIA ne fait pas encore"</h2>
            <Alerte niveau=Niveau::Info titre="Signalement : fonctions à venir (lot AS9)">
                <p>"ScolIA prépare, la direction décide, une personne transmet : rien n'est envoyé automatiquement. Pas encore disponible :"</p>
                <ul>{MANQUE_AS9.iter().map(|m| view! { <li>{*m}</li> }).collect_view()}</ul>
            </Alerte>
        </section>
        <Confirmation
            ouvert=confirmer_proposition
            id="conf-proposition"
            titre="Préparer les signalements du mois ?"
            message="Un signalement est proposé pour chaque élève au seuil. Rien n'est envoyé à l'académie ni aux familles : la direction relit et décide ensuite. L'opération est sans effet sur un signalement déjà proposé."
            libelle_confirmer="Préparer"
            on_confirmer=proposer
        />
    }
}

#[component]
fn ElevesASuivre(decompte: DecompteMoisDto, signalements: Vec<SignalementDto>) -> impl IntoView {
    let seuil = decompte.seuil_signalement;
    let au_seuil = decompte.eleves.iter().filter(|e| e.seuil_atteint).count();
    let a_suivre = eleves_a_suivre(&decompte);
    let arrete = decompte.arrete;
    let ton_seuil = if au_seuil > 0 { TonEtat::Avertissement } else { TonEtat::Ok };
    if a_suivre.is_empty() {
        return view! {
            <EtatVide titre="Aucun élève à suivre" message="Aucune demi-journée non justifiée n'est comptée ce mois-ci."/>
        }
        .into_any();
    }
    let statut_de: HashMap<Uuid, String> = signalements
        .iter()
        .map(|s| (s.eleve_id, s.statut.clone()))
        .collect();
    view! {
        <div class="jvs-stats">
            <Statistique libelle="Élèves à suivre" valeur=Signal::derive({ let n = a_suivre.len(); move || n.to_string() })/>
            <Statistique
                libelle="Au seuil"
                valeur=Signal::derive(move || au_seuil.to_string())
                ton=ton_seuil
            />
        </div>
        <p class="texte-attenue">
            {format!("Seuil de l'établissement : {seuil} demi-journée(s) non justifiée(s) dans le mois. ")}
            {if arrete {
                "Mois arrêté : les chiffres sont figés, les corrections apparaissent en rectifications."
            } else {
                "Mois non arrêté : les chiffres peuvent encore changer (qualification, justificatif, correction d'appel)."
            }}
        </p>
        <div class="ui-tableau-defile">
            <table class="ui-tableau asd-tableau">
                <caption class="ui-sr">"Élèves ayant des demi-journées non justifiées dans le mois"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Élève"</th>
                        <th scope="col">"Non justifiées"</th>
                        <th scope="col">"Position"</th>
                        <th scope="col">"Signalement"</th>
                        <th scope="col"><span class="ui-sr">"Dossier"</span></th>
                    </tr>
                </thead>
                <tbody>
                    {a_suivre
                        .into_iter()
                        .map(|e| {
                            let etat = EtatSeuil::de(e.demi_journees_non_justifiees, seuil);
                            let nom = format!("{} {}", e.prenom, e.nom).trim().to_string();
                            let href = format!("{}/eleves/{}", navigation::ASSIDUITE, e.eleve_id);
                            let accessible = format!("Ouvrir le dossier d'assiduité de {nom}");
                            let statut = statut_de.get(&e.eleve_id).cloned();
                            view! {
                                <tr>
                                    <th scope="row">{nom}</th>
                                    <td>{format!("{} sur {seuil}", e.demi_journees_non_justifiees)}</td>
                                    <td><Badge libelle=etat.libelle() ton=etat.ton()/></td>
                                    <td>
                                        {match statut {
                                            Some(s) => view! { <Badge libelle=libelle_statut_signalement(&s) ton=ton_statut_signalement(&s)/> }.into_any(),
                                            None => view! { <span class="texte-attenue">"Aucun"</span> }.into_any(),
                                        }}
                                    </td>
                                    <td>
                                        <a href=href aria-label=accessible>"Dossier"</a>
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

#[component]
fn ListeSignalements(
    signalements: Vec<SignalementDto>,
    noms: HashMap<Uuid, String>,
    ecriture: Ecriture,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let cible = RwSignal::new(None::<(Uuid, &'static str, &'static str)>);
    let confirmer = RwSignal::new(false);
    view! {
        <ul class="jvs-liste asd-signalements">
            {signalements
                .into_iter()
                .map(|s| {
                    let nom = noms.get(&s.eleve_id).cloned().unwrap_or_else(|| "Élève".into());
                    let (id, statut, resume) = (s.id, s.statut.clone(), resume_signalement(&s));
                    let decisions = decisions_signalement(&statut);
                    let lignes = lignes_dossier_signalement(&s);
                    let href = format!("{}/eleves/{}", navigation::ASSIDUITE, s.eleve_id);
                    let accessible_dossier = format!("Ouvrir le dossier d'assiduité de {nom}");
                    let nom_decision = nom.clone();
                    let valide = statut == "valide";
                    view! {
                        <li class="jvs-ligne asd-signalement">
                            <div>
                                <strong>{nom.clone()}</strong>
                                <span class="texte-attenue">{format!(" — {resume}")}</span>
                            </div>
                            <Badge libelle=libelle_statut_signalement(&statut) ton=ton_statut_signalement(&statut)/>
                            <details class="asd-relecture">
                                <summary>{format!("Relire le dossier pré-rempli de {nom}")}</summary>
                                <p class="texte-attenue">
                                    "Demi-journées non justifiées retenues, figées à la proposition. Le dossier complet (absences, contacts, courriers) est dans le dossier de l'élève."
                                </p>
                                {if lignes.is_empty() {
                                    view! { <p>"Aucune demi-journée n'est rattachée à ce signalement."</p> }.into_any()
                                } else {
                                    view! {
                                        <ul>
                                            {lignes.iter().map(|l| view! {
                                                <li>{format!("{}, {} ({} min)", date_longue(l.jour), libelle_periode_avis(&l.periode), l.minutes)}</li>
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }}
                                <p><a href=href aria-label=accessible_dossier>"Ouvrir le dossier d'assiduité"</a></p>
                            </details>
                            {valide.then(|| view! {
                                <p class="texte-attenue">
                                    "Validé : ScolIA n'envoie rien à l'académie. Transmettez le signalement par l'outil de votre académie, puis notez-le ici."
                                </p>
                            })}
                            {move || {
                                let peut = moteur.get().peut_gerer_discipline();
                                if decisions.is_empty() {
                                    return None;
                                }
                                Some(if peut {
                                    let nom = nom_decision.clone();
                                    view! {
                                        <div class="ra-actions">
                                            {decisions
                                                .iter()
                                                .map(|(action, libelle)| {
                                                    let (action, libelle) = (*action, *libelle);
                                                    let accessible = format!("{libelle} : {nom}");
                                                    view! {
                                                        <Bouton
                                                            variante=if action == "refuser" { Variante::Discret } else { Variante::Secondaire }
                                                            taille=Taille::Compacte
                                                            en_cours=ecriture.en_cours
                                                            libelle_accessible=accessible
                                                            on_click=Callback::new(move |_| {
                                                                cible.set(Some((id, action, libelle)));
                                                                confirmer.set(true);
                                                            })
                                                        >
                                                            {libelle}
                                                        </Bouton>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    }
                                    .into_any()
                                } else {
                                    view! { <p class="texte-attenue">"La décision revient à la direction."</p> }.into_any()
                                })
                            }}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
        <Confirmation
            ouvert=confirmer
            id="conf-signalement"
            titre="Confirmer la décision ?"
            message="La décision est enregistrée et tracée. Elle ne notifie ni l'élève ni sa famille, et n'envoie rien à l'académie depuis cet écran."
            libelle_confirmer="Confirmer"
            on_confirmer=Callback::new(move |_| {
                if let Some((id, action, libelle)) = cible.get_untracked() {
                    ecriture.lancer(async move {
                        session::client().decider_signalement(id, action).await?;
                        Ok(format!("{libelle} : fait."))
                    });
                }
            })
        />
    }
}

#[component]
fn ListeRectifications(
    rectifications: Vec<RectificationDto>,
    noms: HashMap<Uuid, String>,
    seuil: i32,
    ecriture: Ecriture,
) -> impl IntoView {
    let a_traiter = rectifications_a_traiter(&rectifications);
    let autres = rectifications.len() - a_traiter.len();
    if a_traiter.is_empty() {
        return view! {
            <p class="texte-attenue">
                {if autres == 0 {
                    "Aucune rectification pour ce mois."
                } else {
                    "Aucune rectification ne touche un signalement ce mois-ci."
                }}
            </p>
        }
        .into_any();
    }
    view! {
        <p class="texte-attenue">
            "Une absence a été requalifiée (ou un appel corrigé) après l'arrêt du mois : le décompte du mois n'est pas réécrit, mais un signalement est touché. La direction décide d'une éventuelle rectification."
        </p>
        <ul class="jvs-liste asd-rectifs">
            {a_traiter
                .into_iter()
                .map(|r| {
                    let nom = noms.get(&r.eleve_id).cloned().unwrap_or_else(|| "Élève".into());
                    let (id, vue) = (r.id, r.vue_par.is_some());
                    let franchit = franchit_le_seuil(&r, seuil);
                    let phrase = phrase_rectification(&r);
                    let accessible = format!("Marquer comme vue la rectification de {nom}");
                    view! {
                        <li class="jvs-ligne">
                            <div>
                                <strong>{nom}</strong>
                                <span class="texte-attenue">{format!(" — {phrase}")}</span>
                            </div>
                            {franchit.then(|| view! { <Badge libelle="Change la position par rapport au seuil" ton=TonEtat::Mal/> })}
                            {if vue {
                                view! { <Badge libelle="Vue" ton=TonEtat::Ok/> }.into_any()
                            } else {
                                view! {
                                    <Bouton
                                        variante=Variante::Secondaire
                                        taille=Taille::Compacte
                                        en_cours=ecriture.en_cours
                                        libelle_accessible=accessible
                                        on_click=Callback::new(move |_| {
                                            ecriture.lancer(async move {
                                                session::client().rectification_vue(id).await?;
                                                Ok("Rectification notée comme vue.".to_string())
                                            });
                                        })
                                    >
                                        "Marquer comme vue"
                                    </Bouton>
                                }.into_any()
                            }}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn neuf() -> Uuid {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Uuid::from_u128(u128::from(N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)))
    }

    fn eleve(nom: &str, non_just: i64, atteint: bool) -> DecompteEleveDto {
        DecompteEleveDto {
            eleve_id: neuf(),
            nom: nom.into(),
            prenom: "X".into(),
            demi_journees_comptees: non_just + 1,
            demi_journees_non_justifiees: non_just,
            seuil_atteint: atteint,
        }
    }

    fn rectif(touche: bool, vue: bool, heure: u32) -> RectificationDto {
        RectificationDto {
            id: neuf(),
            eleve_id: Uuid::nil(),
            mois: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            avant_demi_journees: 5,
            avant_non_justifiees: 5,
            apres_demi_journees: 3,
            apres_non_justifiees: 3,
            cause: "requalification".into(),
            touche_signalement: touche,
            cree_le: Utc.with_ymd_and_hms(2026, 11, 5, heure, 0, 0).unwrap(),
            vue_par: vue.then(neuf),
        }
    }

    #[test]
    fn eleves_a_suivre_au_seuil_d_abord_sans_les_zero() {
        let d = DecompteMoisDto {
            mois: NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            seuil_signalement: 4,
            eleves: vec![
                eleve("Petit", 1, false),
                eleve("Zola", 4, true),
                eleve("Aubry", 4, true),
                eleve("Nul", 0, false),
            ],
            arrete: false,
        };
        let v = eleves_a_suivre(&d);
        let noms: Vec<&str> = v.iter().map(|e| e.nom.as_str()).collect();
        assert_eq!(noms, vec!["Aubry", "Zola", "Petit"], "à égalité, ordre alphabétique");
        assert_eq!(noms_depuis_decompte(&d).len(), 4);
    }

    #[test]
    fn rectifications_non_vues_et_recentes_d_abord() {
        let v = vec![
            rectif(true, true, 8),
            rectif(false, false, 9),
            rectif(true, false, 7),
            rectif(true, false, 10),
        ];
        let t = rectifications_a_traiter(&v);
        assert_eq!(t.len(), 3, "celles qui ne touchent aucun signalement sont écartées");
        assert!(t[0].vue_par.is_none() && t[1].vue_par.is_none() && t[2].vue_par.is_some());
        assert!(t[0].cree_le > t[1].cree_le, "la plus récente d'abord");
    }

    #[test]
    fn le_manque_est_dit() {
        assert_eq!(MANQUE_AS9.len(), 5);
        assert!(MANQUE_AS9.iter().any(|m| m.contains("passkey")));
        assert!(MANQUE_AS9.iter().any(|m| m.contains("export")|| m.contains("PDF")));
    }
}
