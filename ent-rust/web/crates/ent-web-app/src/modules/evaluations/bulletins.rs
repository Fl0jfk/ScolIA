//! Bulletins et livrets d'une classe (direction, EVA-10, EVA-12, EVA-14) : génération
//! (période fermée ; nouvelle version seulement si le fond a changé, motif obligatoire),
//! publication aux familles (archivage au dossier, notification), versions, PDF, et exports
//! LSU / LSL **au format documenté non validé** [NV] avec leurs anomalies.

use ent_types::evaluations::{
    BulletinDto, ExportLivretDto, GenererBulletinsBody, PublierBulletinsBody,
};
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, Confirmation, EnTetePage, EtatVide, Niveau, Squelette,
    Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::modele::{libelle_statut_bulletin, nom_fichier, option_code};
use super::{lien_classe, RACINE};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::telechargement::enregistrer_telechargement;

#[component]
pub fn PageBulletins() -> impl IntoView {
    let q = use_query_map();
    let lire = move |cle: &'static str| {
        Memo::new(move |_| {
            q.with(|q| q.get(cle))
                .and_then(|s| Uuid::parse_str(&s).ok())
        })
    };
    let (classe, periode) = (lire("classe"), lire("periode"));
    view! {
        <Title text="Bulletins et livrets"/>
        <EnTetePage
            surtitre="Évaluations".to_string()
            titre="Bulletins et livrets".to_string()
            sous_titre="Génération, publication aux familles et exports nationaux.".to_string()
        />
        {move || match (classe.get(), periode.get()) {
            (Some(c), Some(p)) => view! { <Bulletins classe=c periode=p/> }.into_any(),
            _ => view! { <EtatVide titre="Classe introuvable" message="Ouvrez les bulletins depuis la page Évaluations."/> }.into_any(),
        }}
    }
}

fn ton_bulletin(s: &str) -> TonEtat {
    match s {
        "publie" => TonEtat::Ok,
        "genere" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// Télécharge le PDF d'une version (aucune adresse de stockage n'est exposée).
pub fn telecharger_pdf(b: &BulletinDto, erreur: RwSignal<Option<String>>) {
    let (id, nom) = (
        b.id,
        nom_fichier(&format!(
            "bulletin-{}-{}-v{}.pdf",
            b.eleve_nom, b.eleve_prenom, b.version
        )),
    );
    leptos::task::spawn_local(async move {
        match session::client().bulletin_pdf(id).await {
            Ok(octets) => {
                if let Err(m) = enregistrer_telechargement(&nom, "application/pdf", &octets) {
                    erreur.set(Some(m));
                }
            }
            Err(e) => erreur.set(Some(e.message_utilisateur())),
        }
    });
}

#[component]
fn Bulletins(classe: Uuid, periode: Uuid) -> impl IntoView {
    let liste = LocalResource::new(move || async move {
        session::client().bulletins_classe(classe, periode).await
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    let motif = RwSignal::new(String::new());
    let confirmer = RwSignal::new(false);
    let export: RwSignal<Option<ExportLivretDto>> = RwSignal::new(None);
    let erreur_dl: RwSignal<Option<String>> = RwSignal::new(None);
    let generer = Callback::new(move |_| {
        let body = GenererBulletinsBody {
            classe_id: classe,
            periode_id: periode,
            motif: option_code(&motif.get_untracked()),
        };
        ecriture.lancer(async move {
            let r = session::client().generer_bulletins(&body).await?;
            Ok(format!(
                "{} bulletin(s) généré(s), {} déjà à jour.",
                r.bulletins.len(),
                r.inchanges
            ))
        });
    });
    let publier = Callback::new(move |_| {
        let body = PublierBulletinsBody {
            classe_id: classe,
            periode_id: periode,
        };
        ecriture.lancer(async move {
            let r = session::client().publier_bulletins(&body).await?;
            Ok(format!(
                "{} bulletin(s) publié(s) aux familles et archivé(s) au dossier.",
                r.bulletins.len()
            ))
        });
    });
    let exporter = move |format: &'static str| {
        Callback::new(move |_| {
            ecriture.lancer(async move {
                let e = session::client()
                    .exporter_livret(format, classe, periode)
                    .await?;
                let msg = format!(
                    "Export {} prêt : {} élève(s).",
                    e.format.to_uppercase(),
                    e.nb_eleves
                );
                export.set(Some(e));
                Ok(msg)
            })
        })
    };
    view! {
        <p class="eva-periode">
            <a href=RACINE>"Toutes les évaluations"</a>" · "<a href=lien_classe(classe, periode)>"Moyennes de la classe"</a>
        </p>
        <RetourEcriture ecriture=ecriture/>
        {move || erreur_dl.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        <section class="ra-section" aria-labelledby="bul-actions">
            <h2 id="bul-actions" class="vs-section-titre">"Générer et publier"</h2>
            <p class="texte-attenue">"La génération exige une période fermée. Une nouvelle version n'est créée que si le contenu a changé (correction d'une note, décision du conseil…) ; le motif est alors obligatoire."</p>
            <div class="eva-formulaire">
                <ChampTexte id="bul-motif" libelle="Motif d'une nouvelle version" valeur=motif aide="Obligatoire à partir de la version 2."/>
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=generer>"Générer les bulletins"</Bouton>
                <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>"Publier aux familles"</Bouton>
            </div>
            <Confirmation
                ouvert=confirmer
                id="bul-confirmer"
                titre="Publier les bulletins ?"
                message="La dernière version générée de chaque élève sera publiée aux responsables autorisés et à l'élève, archivée dans son dossier, et une notification sera envoyée. La version précédente reste archivée."
                libelle_confirmer="Publier"
                on_confirmer=publier
            />
        </section>
        <section class="ra-section" aria-labelledby="bul-versions">
            <h2 id="bul-versions" class="vs-section-titre">"Versions"</h2>
            {move || match liste.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des bulletins…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun bulletin" message="Aucun bulletin généré pour cette classe et cette période."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|b| {
                            let bb = b.clone();
                            view! {
                                <li class="jvs-ligne">
                                    <span>
                                        <strong>{format!("{} {}", b.eleve_nom, b.eleve_prenom)}</strong>
                                        {format!(" · version {}", b.version)}
                                        {b.motif.clone().map(|m| format!(" ({m})"))}
                                    </span>
                                    <span class="eva-liens">
                                        <Badge libelle=libelle_statut_bulletin(&b.statut) ton=ton_bulletin(&b.statut)/>
                                        <Bouton variante=Variante::Discret taille=Taille::Compacte on_click=Callback::new(move |_| telecharger_pdf(&bb, erreur_dl))>"PDF"</Bouton>
                                    </span>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="bul-exports">
            <h2 id="bul-exports" class="vs-section-titre">"Livrets (LSU, LSL)"</h2>
            <Alerte niveau=Niveau::Avertissement titre="Format non validé [NV]">
                "Les fichiers suivent un format documenté par ScolIA ; ils ne sont pas validés contre le schéma officiel (réservé aux éditeurs agréés). Ne pas les importer dans LSU ou LSL sans qualification."
            </Alerte>
            <div class="eva-actions">
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=exporter("lsu")>"Exporter LSU (collège)"</Bouton>
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=exporter("lsl")>"Exporter LSL (lycée)"</Bouton>
            </div>
            {move || export.get().map(|e| {
                let (nom, contenu) = (nom_fichier(&e.nom_fichier), e.contenu.clone());
                view! {
                    <div class="eva-export">
                        <p class="texte-attenue">{e.avertissement.clone()}</p>
                        {(!e.anomalies.is_empty()).then(|| view! {
                            <Alerte niveau=Niveau::Avertissement titre=format!("{} anomalie(s) à corriger", e.anomalies.len())>
                                <ul>{e.anomalies.iter().map(|a| view! { <li>{a.clone()}</li> }).collect_view()}</ul>
                            </Alerte>
                        })}
                        <p class="texte-attenue">{format!("Empreinte SHA-256 : {}", e.empreinte)}</p>
                        <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| {
                            if let Err(m) = enregistrer_telechargement(&nom, "application/xml", contenu.as_bytes()) {
                                erreur_dl.set(Some(m));
                            }
                        })>"Télécharger le fichier"</Bouton>
                    </div>
                }
            })}
        </section>
    }
}
