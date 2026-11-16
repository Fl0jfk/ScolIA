//! Absences par épisode pour la famille et l'élève (socle 12, AS14c) : état clair, « donner le motif »
//! en quelques gestes, communications reçues. Remplace la liste d'absences de l'ancien circuit.
//!
//! Routes : `GET /assiduite/eleves/{id}/episodes` (libellé à liste fermée, jamais la note interne ni le
//! texte d'un autre responsable, R-AS22), `GET /assiduite/motifs-famille`, `POST
//! /assiduite/episodes/{id}/justificatifs` (texte), `GET /assiduite/eleves/{id}/communications`.
//! Le serveur décide qui peut lire et qui peut justifier (lien du responsable, élève majeur) ; l'écran
//! ne propose « Donner le motif » qu'à qui peut agir, et traite les refus.

use ent_types::assiduite::{
    CommunicationFamilleDto, DeposerJustificatifBody, EpisodeVueFamilleDto, MotifAsdDto,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    use_notifications, Alerte, Annonce, Badge, Bouton, Chargement, Dialogue, EtatVide, Niveau,
    Selection, Squelette, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use super::modele::CODE_AUTRE;
use crate::composants_metier::absences::{libelle_periode, nombre_de_jours};
use crate::noyau::ecriture::message_echec;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

/// Longueur maximale du texte déposé (le serveur refuse au-delà de 1000 caractères).
pub const TEXTE_MAX: usize = 1000;

/// Plus récent d'abord.
pub fn recentes_d_abord(mut v: Vec<EpisodeVueFamilleDto>) -> Vec<EpisodeVueFamilleDto> {
    v.sort_by(|a, b| {
        b.jour_debut
            .cmp(&a.jour_debut)
            .then_with(|| a.id.cmp(&b.id))
    });
    v
}

/// Absences qui attendent un motif.
pub fn nb_a_justifier(v: &[EpisodeVueFamilleDto]) -> usize {
    v.iter().filter(|e| e.a_justifier).count()
}

/// Ton du libellé : jamais la seule couleur (le libellé complet est affiché).
pub fn ton_libelle(code: Option<&str>) -> TonEtat {
    match code {
        Some("absence_justifiee" | "absence_excusee") => TonEtat::Ok,
        Some(c) if c.starts_with("motif_non_retenu") => TonEtat::Mal,
        _ => TonEtat::Avertissement,
    }
}

/// Motifs proposés : actifs et visibles pour la famille ; « Autre » en dernier recours.
pub fn options_motifs(motifs: &[MotifAsdDto]) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = motifs
        .iter()
        .filter(|m| m.actif && m.visible_famille)
        .map(|m| {
            (
                m.code.clone(),
                m.libelle_famille
                    .clone()
                    .filter(|l| !l.trim().is_empty())
                    .unwrap_or_else(|| m.libelle.clone()),
            )
        })
        .collect();
    if !v.iter().any(|(c, _)| c == CODE_AUTRE) {
        v.push((CODE_AUTRE.to_string(), "Autre motif".to_string()));
    }
    v
}

/// Texte déposé : « Motif — précision ». Un motif est obligatoire ; « Autre » exige une précision.
pub fn composer_motif(
    options: &[(String, String)],
    code: &str,
    precision: &str,
) -> Result<String, &'static str> {
    let Some((_, libelle)) = options.iter().find(|(c, _)| c == code) else {
        return Err("Choisissez un motif.");
    };
    let precision = precision.trim();
    if code == CODE_AUTRE && precision.is_empty() {
        return Err("Pour « Autre motif », dites en quelques mots ce qui s'est passé.");
    }
    let texte = if precision.is_empty() {
        libelle.clone()
    } else {
        format!("{libelle} — {precision}")
    };
    if texte.chars().count() > TEXTE_MAX {
        return Err("Le texte est trop long (1 000 caractères au plus).");
    }
    Ok(texte)
}

/// Ce que la famille a reçu de la vie scolaire, en français (jamais l'auteur ni la note).
pub fn libelle_communication(c: &CommunicationFamilleDto) -> String {
    let canal = match c.canal.as_str() {
        "telephone" => "Appel téléphonique",
        "sms" => "SMS",
        "courriel" => "Courriel",
        "rencontre" => "Rencontre",
        "courrier" => "Courrier postal",
        _ => "Message",
    };
    let quoi = match c.type_communication.as_str() {
        "courrier" => "courrier posté",
        _ => "contact",
    };
    format!("{canal} ({quoi}) · {}", date_heure_paris(c.cree_le))
}

/// Absences de l'enfant ou de l'élève, par épisode.
#[component]
pub fn SectionEpisodes(
    eleve_id: Uuid,
    /// Le lien (ou la majorité) permet de donner le motif : le serveur revérifie.
    peut_agir: bool,
) -> impl IntoView {
    let notifications = use_notifications();
    let annonce = RwSignal::new(String::new());
    let liste = LocalResource::new(move || async move {
        session::client()
            .episodes_famille(eleve_id)
            .await
            .map(recentes_d_abord)
    });
    let motifs = LocalResource::new(|| async { session::client().motifs_famille().await });

    let ouvert = RwSignal::new(false);
    let cible = RwSignal::new(None::<EpisodeVueFamilleDto>);
    let motif = RwSignal::new(String::new());
    let precision = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);

    let options =
        Memo::new(move |_| options_motifs(&motifs.get().and_then(Result::ok).unwrap_or_default()));
    Effect::new(move |_| {
        if ouvert.get() {
            precision.set(String::new());
            erreur.set(None);
            motif.set(
                options
                    .get_untracked()
                    .first()
                    .map(|(c, _)| c.clone())
                    .unwrap_or_default(),
            );
        }
    });
    let donner = Callback::new(move |e: EpisodeVueFamilleDto| {
        cible.set(Some(e));
        ouvert.set(true);
    });
    let envoyer = move || {
        let Some(e) = cible.get_untracked() else {
            return;
        };
        let texte = match composer_motif(
            &options.get_untracked(),
            &motif.get_untracked(),
            &precision.get_untracked(),
        ) {
            Ok(t) => t,
            Err(m) => {
                erreur.set(Some(m.to_string()));
                return;
            }
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let depot = DeposerJustificatifBody {
                texte: Some(texte),
                medical: None,
                vu_non_conserve: false,
            };
            let r = session::client().deposer_justificatif(e.id, &depot).await;
            en_cours.set(false);
            match r {
                Ok(_) => {
                    ouvert.set(false);
                    let t = "Motif transmis : la vie scolaire va l'examiner.";
                    notifications.succes(t);
                    annonce.set(t.to_string());
                    liste.refetch();
                }
                Err(ApiError::Interdit(_)) => erreur.set(Some(
                    "Votre lien avec cet enfant ne vous permet pas de donner le motif.".into(),
                )),
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    };

    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        <section class="abs-section" aria-labelledby="abs-titre">
            <h2 id="abs-titre" class="vs-section-titre">"Absences"</h2>
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des absences…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune absence" message="Aucune absence n'est enregistrée."/> }.into_any(),
                Some(Ok(l)) => {
                    let nb = nb_a_justifier(&l);
                    view! {
                        {(nb > 0 && peut_agir)
                            .then(|| view! {
                                <p class="abs-a-justifier" role="status">
                                    {format!("{nb} absence{} attend{} un motif", if nb > 1 { "s" } else { "" }, if nb > 1 { "ent" } else { "" })}
                                </p>
                            })}
                        <ul class="abs-liste">
                            {l.into_iter()
                                .map(|e| {
                                    let periode = libelle_periode(e.jour_debut, e.jour_fin);
                                    let jours = nombre_de_jours(e.jour_debut, e.jour_fin);
                                    let accessible = format!("Donner le motif de l'absence : {periode}");
                                    let proposer = peut_agir && e.a_justifier;
                                    let pour_donner = e.clone();
                                    view! {
                                        <li class="abs-carte" data-statut=e.libelle_code.clone().unwrap_or_else(|| "a_regulariser".into())>
                                            <div class="abs-carte__tete">
                                                <p class="abs-carte__periode">
                                                    {periode}
                                                    {(jours > 1).then(|| view! { <span class="abs-carte__jours">{format!(" ({jours} jours)")}</span> })}
                                                </p>
                                                <Badge libelle=e.libelle.clone() ton=ton_libelle(e.libelle_code.as_deref())/>
                                            </div>
                                            {e.prevenue.then(|| view! { <p class="abs-carte__detail">"Vous aviez prévenu l'établissement."</p> })}
                                            {(e.a_justifier && !proposer)
                                                .then(|| view! { <p class="abs-carte__detail texte-attenue">"En attente d'un motif de la part du responsable."</p> })}
                                            {proposer
                                                .then(|| view! {
                                                    <div class="abs-carte__actions">
                                                        <Bouton
                                                            taille=Taille::Compacte
                                                            libelle_accessible=accessible
                                                            on_click=Callback::new(move |_| donner.run(pour_donner.clone()))
                                                        >
                                                            "Donner le motif"
                                                        </Bouton>
                                                    </div>
                                                })}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                        .into_any()
                }
            }}
        </section>

        <Dialogue ouvert=ouvert titre="Donner le motif de l'absence" id="abs-motif-dialogue">
            <p class="abs-dialogue__periode">
                {move || cible.get().map(|e| format!("Absence : {}", libelle_periode(e.jour_debut, e.jour_fin))).unwrap_or_default()}
            </p>
            {move || match motifs.get() {
                None => view! { <Chargement libelle="Chargement des motifs…"/> }.into_any(),
                Some(_) => view! {
                    <Selection id="abs-motif" libelle="Motif" valeur=motif options=options.get() requis=true desactive=en_cours/>
                }
                    .into_any(),
            }}
            <ZoneTexte
                id="abs-precision"
                libelle="Précision"
                valeur=precision
                aide="Facultatif, sauf pour « Autre motif ». N'écrivez pas de détail médical : la vie scolaire demandera une pièce si besoin."
                desactive=en_cours
            />
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire desactive=en_cours on_click=Callback::new(move |_| ouvert.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton en_cours=en_cours on_click=Callback::new(move |_| envoyer())>
                    "Envoyer le motif"
                </Bouton>
            </div>
        </Dialogue>
    }
}

/// Ce que la vie scolaire a fait pour joindre la famille (contacts et courriers postés).
#[component]
pub fn SectionCommunications(eleve_id: Uuid) -> impl IntoView {
    let liste = LocalResource::new(move || async move {
        session::client().communications_famille(eleve_id).await
    });
    view! {
        <section class="abs-section" aria-labelledby="com-titre">
            <h2 id="com-titre" class="vs-section-titre">"Échanges avec la vie scolaire"</h2>
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des échanges…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_) | ApiError::Introuvable(_))) => view! { <p class="texte-attenue">"Les échanges ne sont pas accessibles avec votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <p class="texte-attenue">"Aucun échange récent. Les avis d'absence arrivent par vos notifications."</p> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="abs-liste">
                        {l.iter().map(|c| view! { <li class="abs-carte"><p class="abs-carte__detail">{libelle_communication(c)}</p></li> }).collect_view()}
                    </ul>
                }
                    .into_any(),
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn ep(n: u128, jour: u32, a_justifier: bool, code: Option<&str>) -> EpisodeVueFamilleDto {
        EpisodeVueFamilleDto {
            id: Uuid::from_u128(n),
            jour_debut: NaiveDate::from_ymd_opt(2026, 12, jour).unwrap(),
            jour_fin: NaiveDate::from_ymd_opt(2026, 12, jour).unwrap(),
            prevenue: false,
            a_justifier,
            libelle: "x".into(),
            libelle_code: code.map(str::to_string),
        }
    }

    fn motif(code: &str, famille: bool, actif: bool) -> MotifAsdDto {
        MotifAsdDto {
            id: Uuid::nil(),
            code: code.into(),
            nature_code: "maladie_enfant".into(),
            libelle: format!("Libellé {code}"),
            libelle_famille: Some(format!("Famille {code}")),
            visible_famille: famille,
            actif,
        }
    }

    #[test]
    fn recentes_d_abord_et_compte_des_motifs_attendus() {
        let v = recentes_d_abord(vec![
            ep(1, 2, true, None),
            ep(2, 9, false, Some("absence_justifiee")),
            ep(3, 5, true, None),
        ]);
        assert_eq!(
            v.iter()
                .map(|e| e.jour_debut.format("%d").to_string())
                .collect::<Vec<_>>(),
            ["09", "05", "02"]
        );
        assert_eq!(nb_a_justifier(&v), 2);
    }

    #[test]
    fn ton_selon_le_libelle() {
        assert_eq!(ton_libelle(Some("absence_justifiee")), TonEtat::Ok);
        assert_eq!(ton_libelle(Some("motif_non_retenu_delai")), TonEtat::Mal);
        assert_eq!(
            ton_libelle(Some("motif_non_communique")),
            TonEtat::Avertissement
        );
        assert_eq!(ton_libelle(None), TonEtat::Avertissement);
    }

    #[test]
    fn motifs_actifs_visibles_et_autre_toujours_present() {
        let o = options_motifs(&[
            motif("maladie", true, true),
            motif("interne", false, true),
            motif("ancien", true, false),
        ]);
        assert_eq!(
            o.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
            ["maladie", "autre"]
        );
        assert_eq!(o[0].1, "Famille maladie");
    }

    #[test]
    fn composition_du_motif() {
        let o = options_motifs(&[motif("maladie", true, true)]);
        assert_eq!(
            composer_motif(&o, "maladie", "").unwrap(),
            "Famille maladie"
        );
        assert_eq!(
            composer_motif(&o, "maladie", " grippe ").unwrap(),
            "Famille maladie — grippe"
        );
        assert!(composer_motif(&o, "", "x").is_err());
        assert!(composer_motif(&o, "inconnu", "x").is_err());
        assert!(composer_motif(&o, CODE_AUTRE, "  ").is_err());
        assert!(composer_motif(&o, CODE_AUTRE, "panne de réveil").is_ok());
        assert!(composer_motif(&o, "maladie", &"x".repeat(TEXTE_MAX)).is_err());
    }

    #[test]
    fn communication_sans_auteur_ni_note() {
        let c = CommunicationFamilleDto {
            type_communication: "courrier".into(),
            canal: "courrier".into(),
            cree_le: chrono::DateTime::from_timestamp(1_796_000_000, 0).unwrap(),
        };
        let t = libelle_communication(&c);
        assert!(t.starts_with("Courrier postal (courrier posté)"));
    }
}
