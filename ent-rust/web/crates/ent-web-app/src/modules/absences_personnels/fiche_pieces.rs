//! Pièces et fil d'une absence (ABP-140 à 146) : cercle RH seulement (l'agent, le valideur, l'instructeur).
//! Un nom de fichier n'est jamais affiché (minimisation) ; la lecture d'une pièce passe par un lien
//! pré-signé de 5 minutes, émis après contrôle du droit et journalisé par le serveur.

use ent_types::absences_personnels::{AbsenceDto, DeposerPieceBody, PieceAbsenceDto};
use ent_web_api::{depot_fichier, ApiError};
use ent_web_ui::{
    AccesRefuse, Alerte, Badge, Bouton, Case, Niveau, Selection, Squelette, Taille, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use super::fichiers::{lire_piece_choisie, ouvrir_adresse, Lecture, PieceChoisie};
use super::modele::{libelle_etat_piece, libelle_type_piece, TYPES_PIECE};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{date_heure_paris, date_longue};

/// Taille lisible (« 1,2 Mo », « 340 Ko »).
pub fn libelle_taille(octets: i64) -> String {
    const KO: f64 = 1024.0;
    let o = octets.max(0) as f64;
    if o < KO {
        format!("{octets} o")
    } else if o < KO * KO {
        format!("{} Ko", (o / KO).round() as i64)
    } else {
        format!("{:.1} Mo", o / (KO * KO)).replace('.', ",")
    }
}

#[component]
pub fn OngletPieces(a: AbsenceDto, mienne: Option<bool>, instructeur: bool) -> impl IntoView {
    let id = a.id;
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let pieces = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().absp_pieces(id).await }
    });
    let est_mienne = mienne.unwrap_or(false);
    let vivante = !matches!(a.etat.as_str(), "annulee" | "refusee" | "close");
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="absp-section" aria-labelledby="absp-pieces">
            <h2 id="absp-pieces" class="vs-section-titre">"Pièces"</h2>
            <p class="texte-attenue">
                "Seules la personne absente et celles qui instruisent la demande voient ces pièces. Chaque lecture est journalisée."
            </p>
            {move || match pieces.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des pièces…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_) | ApiError::Introuvable(_))) => view! {
                    <AccesRefuse message="Les pièces ne sont visibles que de la personne absente et des personnes qui instruisent cette absence."/>
                }
                .into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| pieces.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! { <p class="texte-attenue">"Aucune pièce déposée."</p> }.into_any(),
                Some(Ok(v)) => view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|p| view! { <LignePiece p=p id_absence=id ecriture=ecriture/> }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
            {(instructeur && !est_mienne && vivante).then(|| view! {
                <p>
                    <Bouton
                        variante=Variante::Secondaire
                        taille=Taille::Compacte
                        on_click=Callback::new(move |_| {
                            ecriture.lancer(async move {
                                session::client().absp_relancer_piece(id).await?;
                                Ok("Relance envoyée à la personne.".to_string())
                            });
                        })
                    >
                        "Relancer la pièce"
                    </Bouton>
                </p>
            })}
        </section>
        {vivante.then(|| view! { <FormulaireDepot id_absence=id ecriture=ecriture/> })}
        {est_mienne.then(|| view! { <Lectures id_absence=id version=version/> })}
    }
}

#[component]
fn LignePiece(p: PieceAbsenceDto, id_absence: Uuid, ecriture: Ecriture) -> impl IntoView {
    let piece = p.id;
    let deposee = p.etat == "deposee";
    let libelle = format!(
        "{} · {} · déposée le {}",
        libelle_type_piece(&p.type_piece),
        libelle_taille(p.taille_octets),
        date_heure_paris(p.deposee_le)
    );
    let ton = if deposee {
        TonEtat::Ok
    } else {
        TonEtat::Neutre
    };
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{libelle_type_piece(&p.type_piece)}</strong>
                <Badge libelle=libelle_etat_piece(&p.etat) ton=ton/>
            </div>
            <p class="absp-carte__detail">{libelle}</p>
            {p.purge_prevue_le.map(|d| view! {
                <p class="absp-carte__detail">{format!("Suppression programmée le {}.", date_longue(d))}</p>
            })}
            {deposee.then(|| view! {
                <p class="absp-carte__actions">
                    <Bouton
                        variante=Variante::Secondaire
                        taille=Taille::Compacte
                        libelle_accessible=format!("Lire la pièce : {}", libelle_type_piece(&p.type_piece))
                        on_click=Callback::new(move |_| {
                            ecriture.lancer(async move {
                                let lien = session::client().absp_lien_piece(id_absence, piece).await?;
                                ouvrir_adresse(&lien.url).map_err(ApiError::Invalide)?;
                                Ok("Pièce ouverte : le lien expire dans quelques minutes.".to_string())
                            });
                        })
                    >
                        "Lire"
                    </Bouton>
                    <Bouton
                        variante=Variante::Discret
                        taille=Taille::Compacte
                        libelle_accessible=format!("Retirer la pièce : {}", libelle_type_piece(&p.type_piece))
                        on_click=Callback::new(move |_| {
                            ecriture.lancer(async move {
                                session::client().absp_retirer_piece(id_absence, piece).await?;
                                Ok("Pièce retirée.".to_string())
                            });
                        })
                    >
                        "Retirer"
                    </Bouton>
                </p>
            })}
        </li>
    }
}

#[component]
fn FormulaireDepot(id_absence: Uuid, ecriture: Ecriture) -> impl IntoView {
    let piece = RwSignal::new(None::<PieceChoisie>);
    let type_piece = RwSignal::new("justificatif".to_string());
    let volet = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);
    let champ = NodeRef::<leptos::html::Input>::new();
    let options: Vec<(String, String)> = TYPES_PIECE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let choisir = move |_| {
        let Some(input) = champ.get() else { return };
        spawn_local(async move {
            match lire_piece_choisie(&input).await {
                Lecture::Aucun => piece.set(None),
                Lecture::Fichier(f) => {
                    erreur.set(None);
                    piece.set(Some(f));
                }
                Lecture::Refuse(m) => {
                    input.set_value("");
                    piece.set(None);
                    erreur.set(Some(m));
                }
            }
        });
    };
    let deposer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(f) = piece.get_untracked() else {
            erreur.set(Some("Choisissez d'abord un fichier.".into()));
            return;
        };
        let t = type_piece.get_untracked();
        if t == "arret" && !volet.get_untracked() {
            erreur.set(Some(
                "Confirmez n'avoir joint que les volets 2 et 3 : le volet 1 contient le diagnostic.".into(),
            ));
            return;
        }
        erreur.set(None);
        let exclu = volet.get_untracked();
        if let Some(input) = champ.get_untracked() {
            input.set_value("");
        }
        piece.set(None);
        volet.set(false);
        ecriture.lancer(async move {
            session::client()
                .absp_deposer_piece(
                    id_absence,
                    &DeposerPieceBody {
                        type_piece: t,
                        fichier: depot_fichier(&f.nom, &f.mime, &f.octets),
                        volet_1_exclu: exclu,
                    },
                )
                .await?;
            Ok("Pièce déposée.".to_string())
        });
    };
    view! {
        <section class="absp-section" aria-labelledby="absp-depot">
            <h2 id="absp-depot" class="vs-section-titre">"Déposer une pièce"</h2>
            <form class="absp-formulaire" on:submit=deposer novalidate=true>
                <Selection id="absp-depot-type" libelle="Type de pièce" valeur=type_piece options=options/>
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for="absp-depot-fichier">"Fichier"</label>
                    <input
                        id="absp-depot-fichier"
                        class="ui-champ__controle"
                        type="file"
                        node_ref=champ
                        aria-describedby="absp-depot-aide"
                        on:change=choisir
                    />
                    <p id="absp-depot-aide" class="ui-champ__aide">
                        "PDF ou image, 10 Mo au maximum. Le contenu est contrôlé par le serveur."
                    </p>
                </div>
                {move || (type_piece.get() == "arret").then(|| view! {
                    <Case
                        id="absp-depot-volet"
                        libelle="Je n'ai joint que les volets 2 et 3 de l'arrêt, jamais le volet 1 (diagnostic)"
                        coche=volet
                    />
                })}
                {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Déposer la pièce"</Bouton>
            </form>
        </section>
    }
}

/// « Qui a lu mes pièces ? » (ABP-144) : les lectures de ses propres pièces. Le journal ne donne pas de nom.
#[component]
fn Lectures(id_absence: Uuid, version: RwSignal<u32>) -> impl IntoView {
    let lectures = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().absp_lectures(id_absence).await }
    });
    view! {
        <section class="absp-section" aria-labelledby="absp-lectures">
            <h2 id="absp-lectures" class="vs-section-titre">"Consultations de mes pièces"</h2>
            {move || match lectures.get() {
                None => view! { <Squelette lignes=1 libelle="Chargement…"/> }.into_any(),
                Some(Err(_)) => view! {
                    <Alerte niveau=Niveau::Info titre="Le journal des consultations n'est pas disponible pour le moment."/>
                }
                .into_any(),
                Some(Ok(v)) if v.is_empty() => view! { <p class="texte-attenue">"Personne n'a encore consulté vos pièces."</p> }.into_any(),
                Some(Ok(v)) => view! {
                    <ul class="absp-liste absp-liste--serree">
                        {v.iter().map(|l| view! {
                            <li class="absp-ligne">
                                <span>"Une personne habilitée a consulté une pièce"</span>
                                <span class="texte-attenue">{date_heure_paris(l.le)}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
    }
}

// ───────────────────────────── Fil ─────────────────────────────

fn libelle_auteur(role: &str) -> &'static str {
    match role {
        "agent" => "La personne absente",
        "valideur" => "Le valideur",
        "instructeur" => "L'instructeur",
        "direction" => "La direction",
        _ => "Un intervenant",
    }
}

const FIL_MAX: usize = 4000;

#[component]
pub fn OngletFil(id: Uuid) -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let texte = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let fil = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().absp_fil(id).await }
    });
    let poster = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let t = texte.get_untracked();
        let t = t.trim().to_string();
        if t.is_empty() {
            erreur.set(Some("Écrivez un message.".into()));
            return;
        }
        if t.chars().count() > FIL_MAX {
            erreur.set(Some(format!(
                "Le message est limité à {FIL_MAX} caractères."
            )));
            return;
        }
        erreur.set(None);
        ecriture.lancer(async move {
            session::client().absp_poster(id, &t).await?;
            texte.set(String::new());
            Ok("Message ajouté au fil.".to_string())
        });
    };
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="absp-section" aria-labelledby="absp-fil">
            <h2 id="absp-fil" class="vs-section-titre">"Fil de l'absence"</h2>
            <p class="texte-attenue">
                "Échanges entre la personne et celles qui instruisent. Ajout seul, sans pièce jointe : n'y écrivez aucune information de santé."
            </p>
            {move || match fil.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement du fil…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_) | ApiError::Introuvable(_))) => view! {
                    <AccesRefuse message="Le fil n'est visible que de la personne absente et des personnes qui instruisent cette absence."/>
                }
                .into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fil.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! { <p class="texte-attenue">"Aucun message."</p> }.into_any(),
                Some(Ok(v)) => view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|m| view! {
                            <li class="absp-carte">
                                <div class="absp-carte__tete">
                                    <strong class="absp-carte__titre">{libelle_auteur(&m.role_affiche)}</strong>
                                    <span class="texte-attenue">{date_heure_paris(m.le)}</span>
                                </div>
                                <p class="absp-message">{m.texte}</p>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
            <form class="absp-formulaire" on:submit=poster novalidate=true>
                <ZoneTexte
                    id="absp-fil-texte"
                    libelle="Votre message"
                    valeur=texte
                    aide=format!("{FIL_MAX} caractères au plus.")
                    erreur=Signal::derive(move || erreur.get())
                />
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Envoyer"</Bouton>
            </form>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_taille_se_lit_en_clair() {
        assert_eq!(libelle_taille(12), "12 o");
        assert_eq!(libelle_taille(2048), "2 Ko");
        assert_eq!(libelle_taille(1_572_864), "1,5 Mo");
    }

    #[test]
    fn un_auteur_inconnu_reste_anonyme() {
        assert_eq!(libelle_auteur("agent"), "La personne absente");
        assert_eq!(libelle_auteur("autre"), "Un intervenant");
    }
}
