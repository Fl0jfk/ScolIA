//! Écran « Clôture et archives » (`/web/cahier/archives`, direction : `cahier.exporter` pour les archives et exports,
//! `cahier.cloturer` pour la clôture, la réouverture à double validation et la décision d'archive) : contrôle de clôture, clôture
//! du cahier d'une année (irréversible sauf réouverture motivée et validée par une seconde personne), archives annuelles à
//! empreinte, exports figés (litige, inspection).
//!
//! Aucune donnée structurée d'élève n'est écrite ici (ni remise, ni retour, ni variante) ; un nom d'élève saisi dans un texte
//! libre du cahier figurerait dans les archives et les exports. Le serveur décide : droits, contrôle, irréversibilité, échéance de
//! conservation. L'écran formate (`modele_c8`) et demande confirmation avant la clôture.

use ent_types::referentiel_admin::AnneeStatutDto;
use ent_web_api::cahier_c8_types::{
    ArchiveDto, ClotureBody, ExportBody, ExportContenuDto, ReouvrirBody, ValiderReouvertureBody,
};
use ent_web_ui::{
    AccesRefuse, Alerte, Badge, Bouton, Case, ChampTexte, Confirmation, EnTetePage, EtatVide,
    LienBouton, Niveau, Selection, Squelette, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele_c8::{empreinte_courte, libelle_decision, lire_jour, points_controle};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_longue;

pub const ADRESSE_ARCHIVES: &str = "/web/cahier/archives";

#[component]
pub fn PageArchives() -> impl IntoView {
    let moteur = use_droits_moteur();
    view! {
        <Title text="Clôture et archives"/>
        {move || {
            if moteur.get().a("cahier.exporter") || moteur.get().a("cahier.cloturer") {
                view! { <Contenu/> }.into_any()
            } else {
                view! { <AccesRefuse message="La clôture, les archives et les exports sont réservés à la direction."/> }.into_any()
            }
        }}
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let annees = LocalResource::new(|| async { session::client().annees_statut().await });
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Clôture et archives".to_string()
            sous_titre="Figer le cahier d'une année, retrouver les archives, produire un export daté et signé par empreinte.".to_string()
        >
            <LienBouton href=super::ADRESSE_VISA variante=Variante::Secondaire taille=Taille::Compacte>"Tenue et visa"</LienBouton>
        </EnTetePage>
        {move || match annees.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des années…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| annees.refetch())/> }.into_any(),
            Some(Ok(a)) if a.is_empty() => view! { <EtatVide titre="Aucune année" message="Aucune année scolaire n'est paramétrée."/> }.into_any(),
            Some(Ok(a)) => view! { <Annee annees=a/> }.into_any(),
        }}
    }
}

#[component]
fn Annee(annees: Vec<AnneeStatutDto>) -> impl IntoView {
    let defaut = annees
        .iter()
        .find(|a| a.courante)
        .or(annees.first())
        .map(|a| a.id.to_string())
        .unwrap_or_default();
    let choix = RwSignal::new(defaut);
    let options: Vec<(String, String)> = annees
        .iter()
        .map(|a| (a.id.to_string(), a.libelle.clone()))
        .collect();
    let bornes = StoredValue::new(
        annees
            .iter()
            .map(|a| (a.id.to_string(), a.date_debut, a.date_fin))
            .collect::<Vec<_>>(),
    );
    view! {
        <section class="cdt-seance" aria-label="Année">
            <Selection id="arch-annee" libelle="Année scolaire" valeur=choix options=options/>
        </section>
        {move || {
            let id = Uuid::parse_str(&choix.get()).ok();
            let b = bornes.get_value().into_iter().find(|(i, _, _)| *i == choix.get()).map(|(_, d, f)| (d, f));
            match (id, b) {
                (Some(id), Some((d, f))) => view! { <Corps annee=id debut=d.to_string() fin=f.to_string()/> }.into_any(),
                _ => ().into_any(),
            }
        }}
    }
}

#[component]
fn Corps(annee: Uuid, debut: String, fin: String) -> impl IntoView {
    // Deux droits distincts : `cahier.cloturer` (clôture, réouverture, décision) et `cahier.exporter` (archives, exports).
    let moteur = use_droits_moteur();
    view! {
        {move || moteur.get().a("cahier.cloturer").then(|| view! { <Cloture annee=annee/> })}
        {move || moteur.get().a("cahier.exporter").then(|| {
            let (d, f) = (debut.clone(), fin.clone());
            view! {
                <Archives annee=annee/>
                <Exports debut=d fin=f/>
            }
        })}
    }
}

#[component]
fn Cloture(annee: Uuid) -> impl IntoView {
    let controle =
        LocalResource::new(
            move || async move { session::client().cdt_cloture_controle(annee).await },
        );
    let ecriture = Ecriture::nouveau(Callback::new(move |_| controle.refetch()));
    let confirmer = RwSignal::new(false);
    let motif = RwSignal::new(String::new());
    let dialogue = RwSignal::new(false);
    let cloturer = Callback::new(move |_| {
        let b = ClotureBody {
            annee_id: annee,
            confirmer: confirmer.get_untracked(),
        };
        ecriture.lancer(async move {
            let r = session::client().cdt_cloturer(&b).await?;
            Ok(format!(
                "Cahier figé : {} archive(s) produite(s).",
                r.archives_produites
            ))
        });
    });
    let rouvrir = move |_| {
        let b = ReouvrirBody {
            annee_id: annee,
            motif: motif.get_untracked(),
        };
        ecriture.lancer(async move {
            session::client().cdt_rouvrir(&b).await?;
            Ok("Demande de réouverture enregistrée : une seconde personne de la direction doit la valider.".to_string())
        });
    };
    let valider = move |_| {
        ecriture.lancer(async move {
            session::client()
                .cdt_rouvrir_valider(&ValiderReouvertureBody { annee_id: annee })
                .await?;
            Ok("Cahier rouvert : la réouverture est tracée (demandée et validée par deux personnes).".to_string())
        });
    };
    view! {
        <section class="cdt-seance" aria-label="Clôture du cahier">
            <h2 class="cdt-travail__titre">"Clôture du cahier"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match controle.get() {
                None => view! { <Squelette lignes=2 libelle="Contrôle de clôture…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| controle.refetch())/> }.into_any(),
                Some(Ok(c)) => {
                    let points = points_controle(&c);
                    let a_regler = !points.is_empty();
                    let noms = c.cours_non_vises.iter().map(|x| format!("{} ({})", x.libelle, x.public)).collect::<Vec<_>>().join(", ");
                    let fige = c.deja_fige;
                    let attente = c.reouverture_en_attente.clone();
                    view! {
                        <p class="cdt-resume">{format!("Année {} : {} cours, {} archive(s) déjà produite(s).", c.annee_libelle, c.cours_total, c.archives_existantes)}</p>
                        {a_regler.then(|| view! {
                            <Alerte niveau=Niveau::Avertissement titre="Le contrôle de clôture signale des points"/>
                            <ul class="cdt-liste">{points.into_iter().map(|p| view! { <li>{p}</li> }).collect_view()}</ul>
                            {(!noms.is_empty()).then(|| view! { <p class="cdt-travail__detail">{format!("Cours non visés : {noms}")}</p> })}
                        })}
                        {if fige {
                            match attente {
                                Some(d) => view! {
                                    <Badge libelle="Cahier figé" ton=TonEtat::Ok/>
                                    <Alerte niveau=Niveau::Avertissement titre="Réouverture demandée, en attente d'une seconde validation"/>
                                    <p class="cdt-travail__detail">{format!("Demandée par {} : « {} »", d.demandee_par, d.motif)}</p>
                                    {if d.demandee_par_vous {
                                        view! { <p class="cdt-resume">"Vous ne pouvez pas valider votre propre demande : une autre personne de la direction doit le faire."</p> }.into_any()
                                    } else {
                                        view! { <Bouton variante=Variante::Danger en_cours=ecriture.en_cours on_click=Callback::new(valider)>"Valider la réouverture"</Bouton> }.into_any()
                                    }}
                                }.into_any(),
                                None => view! {
                                    <Badge libelle="Cahier figé" ton=TonEtat::Ok/>
                                    <div class="cdt-form">
                                        <ZoneTexte id="arch-motif" libelle="Motif écrit de la demande de réouverture (20 caractères au moins, tracé ; une seconde personne de la direction devra la valider)" valeur=motif/>
                                        <Bouton variante=Variante::Danger en_cours=ecriture.en_cours on_click=Callback::new(rouvrir)>"Demander la réouverture"</Bouton>
                                    </div>
                                }.into_any(),
                            }
                        } else {
                            view! {
                                <div class="cdt-form">
                                    {a_regler.then(|| view! { <Case id="arch-confirmer" libelle="Je clôture malgré ces points" coche=confirmer/> })}
                                    <Bouton variante=Variante::Danger en_cours=ecriture.en_cours on_click=Callback::new(move |_| dialogue.set(true))>"Clôturer le cahier de l'année"</Bouton>
                                </div>
                            }.into_any()
                        }}
                    }.into_any()
                }
            }}
            <Confirmation
                ouvert=dialogue
                id="arch-confirmation"
                titre="Clôturer le cahier de l'année ?"
                message="Plus aucune écriture ne sera possible. Une archive datée est produite pour chaque cours. Seule une réouverture motivée, demandée par une personne de la direction et validée par une autre, permet de revenir en arrière."
                libelle_confirmer="Clôturer"
                danger=true
                on_confirmer=cloturer
            />
        </section>
    }
}

#[component]
fn Archives(annee: Uuid) -> impl IntoView {
    let liste =
        LocalResource::new(
            move || async move { session::client().cdt_archives(Some(annee)).await },
        );
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    let lecture: RwSignal<Option<ExportContenuDto>> = RwSignal::new(None);
    view! {
        <section class="cdt-seance" aria-label="Archives">
            <h2 class="cdt-travail__titre">"Archives de l'année"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des archives…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune archive" message="Elles sont produites à la clôture du cahier."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="cdt-liste">{l.into_iter().map(|a| view! { <li><LigneArchive a=a ecriture=ecriture lecture=lecture/></li> }).collect_view()}</ul>
                }.into_any(),
            }}
            {move || lecture.get().map(|c| view! { <Document c=c/> })}
        </section>
    }
}

#[component]
fn LigneArchive(
    a: ArchiveDto,
    ecriture: Ecriture,
    lecture: RwSignal<Option<ExportContenuDto>>,
) -> impl IntoView {
    let id = a.id;
    // La décision d'archive relève de `cahier.cloturer` (et non du simple droit de lire les archives).
    let a_decider =
        a.decision == "a_decider" && use_droits_moteur().get_untracked().a("cahier.cloturer");
    let titre = format!(
        "{} — {} (version {})",
        a.cours.libelle, a.cours.public, a.version
    );
    let detail = format!(
        "Empreinte {}… · conservée jusqu'au {}",
        empreinte_courte(&a.empreinte),
        date_longue(a.echeance_archive)
    );
    let lire = move |_| {
        ecriture.lancer(async move {
            let c = session::client().cdt_archive_contenu(id).await?;
            lecture.set(Some(c));
            Ok("Archive ouverte.".to_string())
        });
    };
    let decider = move |d: &'static str| {
        ecriture.lancer(async move {
            session::client().cdt_archive_decider(id, d).await?;
            Ok("Décision enregistrée.".to_string())
        });
    };
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{titre}</p>
                <Badge libelle=libelle_decision(&a.decision) ton=if a_decider { TonEtat::Info } else { TonEtat::Neutre }/>
            </header>
            <p class="cdt-travail__detail">{detail}</p>
            <div class="cdt-actions">
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(lire)>"Lire l'archive"</Bouton>
                {a_decider.then(|| view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| decider("versee"))>"Verser aux archives"</Bouton>
                    <Bouton variante=Variante::Danger taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| decider("detruite"))>"Décider la destruction"</Bouton>
                })}
            </div>
        </article>
    }
}

#[component]
fn Document(c: ExportContenuDto) -> impl IntoView {
    view! {
        <article class="cdt-travail" aria-label="Document figé">
            <p class="cdt-travail__titre">{c.export.titre.clone()}</p>
            <p class="cdt-travail__detail">{format!("Empreinte SHA-256 : {} · {} entrée(s), {} travail(aux)", c.export.empreinte, c.export.nb_entrees, c.export.nb_travaux)}</p>
            <pre class="cdt-travail__consigne">{c.contenu.clone()}</pre>
        </article>
    }
}

#[component]
fn Exports(debut: String, fin: String) -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().cdt_exports().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    let lecture: RwSignal<Option<ExportContenuDto>> = RwSignal::new(None);
    let (d0, f0) = (debut.clone(), fin.clone());
    let cours_liste = LocalResource::new(move || {
        let (d, f) = (lire_jour(&d0), lire_jour(&f0));
        async move {
            match (d, f) {
                (Some(d), Some(f)) => session::client().cdt_tenue(d, f).await.map(|t| t.lignes),
                _ => Ok(vec![]),
            }
        }
    });
    let nature = RwSignal::new("litige".to_string());
    let cours = RwSignal::new(String::new());
    let du = RwSignal::new(debut);
    let au = RwSignal::new(fin);
    let motif = RwSignal::new(String::new());
    let produire = move |_| {
        let (Some(c), Some(d), Some(a)) = (
            Uuid::parse_str(&cours.get_untracked()).ok(),
            lire_jour(&du.get_untracked()),
            lire_jour(&au.get_untracked()),
        ) else {
            ecriture
                .erreur
                .set(Some("Choisissez un cours et deux dates valides.".into()));
            return;
        };
        let m = motif.get_untracked();
        let b = ExportBody {
            nature: nature.get_untracked(),
            cours_id: Some(c),
            agent_id: None,
            du: d,
            au: a,
            motif: (!m.trim().is_empty()).then(|| m.trim().to_string()),
        };
        ecriture.lancer(async move {
            let e = session::client().cdt_export_produire(&b).await?;
            Ok(format!(
                "Export produit : empreinte {}….",
                empreinte_courte(&e.empreinte)
            ))
        });
    };
    view! {
        <section class="cdt-seance" aria-label="Exports figés">
            <h2 class="cdt-travail__titre">"Exports figés"</h2>
            <p class="cdt-resume">"Entrées et travaux publiés, aucune donnée structurée d'élève (ni remise, ni retour, ni variante), avec une empreinte : le même cahier donne toujours la même empreinte. Un nom d'élève écrit dans un texte libre y figurerait."</p>
            <RetourEcriture ecriture=ecriture/>
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des exports…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun export" message="Produisez un export pour un litige ou une inspection."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="cdt-liste">
                        {l.into_iter().map(|e| {
                            let id = e.id;
                            let lire = move |_| ecriture.lancer(async move {
                                let c = session::client().cdt_export_lire(id).await?;
                                lecture.set(Some(c));
                                Ok("Export ouvert.".to_string())
                            });
                            view! {
                                <li class="cdt-travail">
                                    <p class="cdt-travail__titre">{e.titre.clone()}</p>
                                    <p class="cdt-travail__detail">{format!("{} · du {} au {} · produit le {} par {} · empreinte {}…", e.nature, date_longue(e.du), date_longue(e.au), date_longue(e.produit_le.date_naive()), e.produit_par, empreinte_courte(&e.empreinte))}</p>
                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(lire)>"Lire"</Bouton>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
            {move || lecture.get().map(|c| view! { <Document c=c/> })}
            <div class="cdt-form">
                <h3 class="cdt-travail__titre">"Nouvel export"</h3>
                <Selection id="exp-nature" libelle="Nature" valeur=nature options=vec![("litige".to_string(), "Litige ou demande de communication".to_string()), ("inspection".to_string(), "Inspection".to_string()), ("autre".to_string(), "Autre".to_string())]/>
                {move || match cours_liste.get() {
                    Some(Ok(l)) => {
                        let mut options = vec![(String::new(), "— choisir un cours —".to_string())];
                        options.extend(l.into_iter().map(|x| (x.cours_id.to_string(), format!("{} — {}", x.libelle, x.public))));
                        view! { <Selection id="exp-cours" libelle="Cours" valeur=cours options=options requis=true/> }.into_any()
                    }
                    _ => view! { <Squelette lignes=1 libelle="Chargement des cours…"/> }.into_any(),
                }}
                <ChampTexte id="exp-du" libelle="Du" valeur=du genre="date"/>
                <ChampTexte id="exp-au" libelle="Au" valeur=au genre="date"/>
                <ZoneTexte id="exp-motif" libelle="Motif (obligatoire pour un litige)" valeur=motif/>
                <Bouton en_cours=ecriture.en_cours on_click=Callback::new(produire)>"Produire l'export"</Bouton>
            </div>
        </section>
    }
}
