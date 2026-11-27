//! Finances (socle 2, lot S2-9).
//!
//! Écrans :
//! - `/web/comptabilite` (personnel, `facturation.lire` / `facturation.gerer`) :
//!   génération annuelle, factures et avoirs (émission ★, avoir ★, règlement manuel),
//!   prélèvements SEPA (préparation avec pré-notification, fichier pain.008, exécution,
//!   rejets), relances (confirmation humaine ★), paramétrage (tarifs), export comptable ★ ;
//! - `/web/mes-factures` (`famille.rs`) : comptes des foyers dont la personne est payeuse.
//!
//! Routes d'`ent-api` : `/finances/*`. Le serveur décide de tout (droits, montants,
//! répartition entre payeurs, transitions) ; l'écran affiche et relaie.

mod famille;
pub mod modele;

pub use famille::PageFinancesFamille;

use ent_web_api::types_finances::{
    ArticleBody, AvoirBody, ExecutionBody, ExportComptableBody, ExportPrelevementsBody,
    FactureDetailDto, FactureResumeDto, GenerationBody, ParametresFinancesBody, PreparationBody,
    ReglementBody, TarifBody,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    AccesRefuse, Alerte, Badge, Bouton, ChampTexte, Confirmation, EnTetePage, EtatVide, Niveau,
    OptionSegment, Segmente, Selection, Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{
    annee_valide, centimes_depuis_saisie, emettables, euros, libelle_echeance, libelle_prelevement,
    libelle_relance, libelle_statut, libelle_suite, lien_telechargement, ton_echeance, ton_statut,
    total_selection, CODES_REJET, MOYENS,
};

fn date(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

#[component]
pub fn PageComptabilite() -> impl IntoView {
    let onglet = RwSignal::new("factures".to_string());
    let moteur = use_droits_moteur();
    let gestion = Signal::derive(move || moteur.get().peut_gerer_facturation());
    view! {
        <Title text="Comptabilité"/>
        <EnTetePage titre="Comptabilité" sous_titre="Facturation des familles, prélèvements, relances et export comptable".to_string()/>
        <Segmente nom="fin-onglet" legende="Rubrique" valeur=onglet options=vec![
            OptionSegment::new("factures", "Factures"),
            OptionSegment::new("prelevements", "Prélèvements"),
            OptionSegment::new("relances", "Relances"),
            OptionSegment::new("parametrage", "Tarifs"),
            OptionSegment::new("export", "Export comptable"),
        ]/>
        {move || match onglet.get().as_str() {
            "prelevements" => view! { <OngletPrelevements gestion=gestion/> }.into_any(),
            "relances" => view! { <OngletRelances gestion=gestion/> }.into_any(),
            "parametrage" => view! { <OngletParametrage gestion=gestion/> }.into_any(),
            "export" => view! { <OngletExport gestion=gestion/> }.into_any(),
            _ => view! { <OngletFactures gestion=gestion/> }.into_any(),
        }}
    }
}

fn refus(e: &ApiError) -> bool {
    matches!(e, ApiError::Interdit(_))
}

// ---------------------------------------------------------------------------
// Factures, génération, émission, avoirs, règlements
// ---------------------------------------------------------------------------

#[component]
fn OngletFactures(gestion: Signal<bool>) -> impl IntoView {
    let filtre = RwSignal::new("brouillon".to_string());
    let factures = LocalResource::new(move || {
        let f = filtre.get();
        async move {
            let statut = (f != "tous").then_some(f);
            session::client().factures_finances(statut.as_deref()).await
        }
    });
    let recharger = Callback::new(move |_| factures.refetch());
    let detail = RwSignal::new(None::<Uuid>);
    view! {
        <Show when=move || gestion.get()>
            <Generation recharger=recharger/>
        </Show>
        <section class="pile" aria-labelledby="fin-factures">
            <h2 id="fin-factures">"Pièces"</h2>
            <Segmente nom="fin-filtre" legende="Afficher" valeur=filtre options=vec![
                OptionSegment::new("brouillon", "Brouillons"),
                OptionSegment::new("emise", "Émises"),
                OptionSegment::new("soldee", "Soldées"),
                OptionSegment::new("tous", "Toutes"),
            ]/>
            {move || match factures.get() {
                None => view! { <Squelette lignes=6 libelle="Chargement des factures…"/> }.into_any(),
                Some(Err(e)) if refus(&e) => view! { <AccesRefuse message="La comptabilité est réservée aux personnes qui en ont le droit."/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(liste)) => view! { <TableFactures factures=liste gestion=gestion detail=detail recharger=recharger/> }.into_any(),
            }}
        </section>
        {move || detail.get().map(|id| view! { <DetailFacture id=id gestion=gestion recharger=recharger/> })}
    }
}

#[component]
fn Generation(recharger: Callback<()>) -> impl IntoView {
    let annee = RwSignal::new(String::new());
    let rapport = RwSignal::new(None::<String>);
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(recharger);
    let lancer = move |apercu: bool| {
        let a = annee.get_untracked().trim().to_string();
        if !annee_valide(&a) {
            ecriture
                .erreur
                .set(Some("Année scolaire au format AAAA-AAAA.".into()));
            return;
        }
        ecriture.lancer(async move {
            let r = session::client()
                .generer_annee(&GenerationBody {
                    annee_scolaire: a,
                    apercu,
                })
                .await?;
            let texte = format!(
                "{} : {} élève(s), {} ligne(s) {}, {} déjà présente(s), {} ; {} anomalie(s).",
                if r.apercu { "Aperçu" } else { "Génération" },
                r.eleves,
                r.lignes_creees,
                if r.apercu { "à créer" } else { "créées" },
                r.lignes_deja_presentes,
                euros(r.montant_centimes),
                r.anomalies.len()
            );
            let detail: Vec<String> = r
                .anomalies
                .iter()
                .map(|a| format!("{} : {}", a.eleve, a.motif))
                .collect();
            rapport.set(Some(if detail.is_empty() {
                texte.clone()
            } else {
                format!("{texte} {}", detail.join(" ; "))
            }));
            Ok(texte)
        });
    };
    view! {
        <details class="pile">
            <summary>"Génération annuelle depuis les inscriptions validées"</summary>
            <p>"Les lignes sont créées en brouillon, une seule fois par élève, article et foyer payeur ; relancer la génération ne double rien."</p>
            <ChampTexte id="fin-gen-annee" libelle="Année scolaire" valeur=annee placeholder="2027-2028" requis=true/>
            <div class="fin-rangee">
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| lancer(true))>"Aperçu"</Bouton>
                <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>"Générer"</Bouton>
            </div>
            <Confirmation ouvert=confirmer id="fin-gen-confirmer" titre="Générer les factures de l'année ?"
                message="Les brouillons seront créés pour chaque foyer payeur. Rien n'est envoyé aux familles avant l'émission."
                libelle_confirmer="Générer" on_confirmer=Callback::new(move |_| lancer(false))/>
            <RetourEcriture ecriture=ecriture/>
            {move || rapport.get().map(|r| view! { <Alerte niveau=Niveau::Info titre=r/> })}
        </details>
    }
}

#[component]
fn TableFactures(
    factures: Vec<FactureResumeDto>,
    gestion: Signal<bool>,
    detail: RwSignal<Option<Uuid>>,
    recharger: Callback<()>,
) -> impl IntoView {
    if factures.is_empty() {
        return view! { <EtatVide titre="Aucune pièce" message="Aucune facture ni aucun avoir dans cette catégorie."/> }.into_any();
    }
    let cochees = RwSignal::new(Vec::<Uuid>::new());
    let liste = StoredValue::new(factures.clone());
    let selection = Memo::new(move |_| liste.with_value(|l| emettables(l, &cochees.get())));
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(recharger);
    let emettre = Callback::new(move |_| {
        let ids = selection.get_untracked();
        ecriture.lancer(async move {
            let r = session::client().emettre_factures(ids).await?;
            Ok(format!(
                "{} pièce(s) émise(s){}.",
                r.emises.len(),
                if r.refus.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", {} refusée(s) : {}",
                        r.refus.len(),
                        r.refus
                            .iter()
                            .map(|x| x.motif.clone())
                            .collect::<Vec<_>>()
                            .join(" ; ")
                    )
                }
            ))
        });
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        <div class="fin-defilement"><table class="fin-tableau">
            <thead><tr>
                <th scope="col"><span class="ui-sr">"Sélection"</span></th>
                <th scope="col">"Pièce"</th><th scope="col">"Foyer"</th><th scope="col">"Libellé"</th>
                <th scope="col">"État"</th><th scope="col">"Total"</th><th scope="col">"Reste dû"</th><th scope="col">"Détail"</th>
            </tr></thead>
            <tbody>
                {factures.into_iter().map(|f| {
                    let id = f.id;
                    let brouillon = f.statut == "brouillon";
                    let libelle_case = format!("Sélectionner {}", f.libelle);
                    view! {
                        <tr>
                            <td>{brouillon.then(|| view! {
                                <input type="checkbox" aria-label=libelle_case.clone()
                                    prop:checked=move || cochees.with(|c| c.contains(&id))
                                    on:change=move |_| cochees.update(|c| if let Some(i) = c.iter().position(|x| *x == id) { c.remove(i); } else { c.push(id); })/>
                            })}</td>
                            <th scope="row">{f.numero.clone()}</th>
                            <td>{f.foyer.clone()}</td>
                            <td>{f.libelle.clone()}</td>
                            <td><Badge libelle=libelle_statut(&f.statut, &f.type_piece) ton=ton_statut(&f.statut)/></td>
                            <td class="fin-nombre">{euros(f.total_centimes)}</td>
                            <td class="fin-nombre">{euros(f.reste_centimes)}</td>
                            <td><Bouton taille=Taille::Compacte variante=Variante::Discret on_click=Callback::new(move |_| detail.set(Some(id)))>"Ouvrir"</Bouton></td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table></div>
        <Show when=move || gestion.get()>
            <div class="fin-rangee">
                <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours
                    desactive=Signal::derive(move || selection.get().is_empty())
                    on_click=Callback::new(move |_| confirmer.set(true))>
                    {move || format!("Émettre la sélection ({}, {})", selection.get().len(), liste.with_value(|l| euros(total_selection(l, &selection.get()))))}
                </Bouton>
            </div>
        </Show>
        <Confirmation ouvert=confirmer id="fin-emettre" titre="Émettre ces pièces ?"
            message="Une pièce émise reçoit son numéro définitif et ne se modifie plus (correction par avoir) ; les payeurs avec un compte sont notifiés."
            libelle_confirmer="Émettre" on_confirmer=emettre/>
    }.into_any()
}

#[component]
fn DetailFacture(id: Uuid, gestion: Signal<bool>, recharger: Callback<()>) -> impl IntoView {
    let detail =
        LocalResource::new(move || async move { session::client().facture_finances(id).await });
    let recharger_detail = Callback::new(move |_| {
        detail.refetch();
        recharger.run(());
    });
    view! {
        <section class="pile carte" aria-labelledby="fin-detail">
            <h2 id="fin-detail">"Détail de la pièce"</h2>
            {move || match detail.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement du détail…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger_detail/> }.into_any(),
                Some(Ok(d)) => view! { <ContenuDetail d=d gestion=gestion recharger=recharger_detail/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ContenuDetail(
    d: FactureDetailDto,
    gestion: Signal<bool>,
    recharger: Callback<()>,
) -> impl IntoView {
    let f = d.facture.clone();
    let id = f.id;
    let foyer = f.foyer_id;
    let peut_avoir = f.type_piece == "facture" && f.statut != "brouillon";
    let peut_regler = f.type_piece == "facture" && f.statut == "emise";
    view! {
        <p>
            <strong>{f.numero.clone()}</strong>" — "{f.libelle.clone()}" — "{f.foyer.clone()}" — "
            <Badge libelle=libelle_statut(&f.statut, &f.type_piece) ton=ton_statut(&f.statut)/>
        </p>
        <p>{format!("Total {} ; réglé {} ; reste {}", euros(f.total_centimes), euros(f.regle_centimes), euros(f.reste_centimes))}</p>
        <div class="fin-defilement"><table class="fin-tableau">
            <caption>"Lignes"</caption>
            <thead><tr><th scope="col">"Élève"</th><th scope="col">"Libellé"</th><th scope="col">"Période"</th><th scope="col">"Montant"</th></tr></thead>
            <tbody>
                {d.lignes.into_iter().map(|l| view! {
                    <tr>
                        <td>{l.eleve.unwrap_or_default()}</td>
                        <th scope="row">{l.libelle}</th>
                        <td>{match (l.periode_debut, l.periode_fin) {
                            (Some(a), Some(b)) => format!("{} → {}", a.format("%d/%m/%Y"), b.format("%d/%m/%Y")),
                            _ => String::new(),
                        }}</td>
                        <td class="fin-nombre">{euros(l.total_centimes)}</td>
                    </tr>
                }).collect_view()}
            </tbody>
        </table></div>
        {(!d.echeances.is_empty()).then(|| view! {
            <div class="fin-defilement"><table class="fin-tableau">
                <caption>"Échéancier"</caption>
                <thead><tr><th scope="col">"Échéance"</th><th scope="col">"Date"</th><th scope="col">"Montant"</th><th scope="col">"Couvert"</th><th scope="col">"État"</th></tr></thead>
                <tbody>
                    {d.echeances.into_iter().map(|e| view! {
                        <tr>
                            <th scope="row">{e.libelle}</th>
                            <td>{e.date_echeance.format("%d/%m/%Y").to_string()}</td>
                            <td class="fin-nombre">{euros(e.montant_centimes)}</td>
                            <td class="fin-nombre">{euros(e.couvert_centimes)}</td>
                            <td><Badge libelle=libelle_echeance(&e.etat) ton=ton_echeance(&e.etat)/>
                                {e.prelevement.map(|p| format!(" ({})", libelle_prelevement(&p)))}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table></div>
        })}
        {(!d.lettrages.is_empty()).then(|| view! {
            <ul aria-label="Imputations et lettrage">
                {d.lettrages.into_iter().map(|l| view! {
                    <li>{format!("{} {} — {}", if l.piece == "avoir" { "Avoir" } else { "Encaissement" }, euros(l.montant_centimes),
                        if l.code.is_empty() { "non lettré".to_string() } else { format!("lettrage {}", l.code) })}</li>
                }).collect_view()}
            </ul>
        })}
        <Show when=move || gestion.get() && peut_avoir>
            <FormAvoir facture=id recharger=recharger/>
        </Show>
        <Show when=move || gestion.get() && peut_regler>
            <FormReglement foyer=foyer recharger=recharger/>
        </Show>
    }
}

#[component]
fn FormAvoir(facture: Uuid, recharger: Callback<()>) -> impl IntoView {
    let motif = RwSignal::new(String::new());
    let montant = RwSignal::new(String::new());
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(recharger);
    let creer = Callback::new(move |_| {
        let m = montant.get_untracked();
        let montant_centimes = if m.trim().is_empty() {
            None
        } else {
            match centimes_depuis_saisie(&m) {
                Ok(c) => Some(c),
                Err(e) => return ecriture.erreur.set(Some(e)),
            }
        };
        let b = AvoirBody {
            motif: motif.get_untracked(),
            montant_centimes,
        };
        ecriture.lancer(async move {
            let d = session::client().creer_avoir(facture, &b).await?;
            Ok(format!(
                "Avoir {} préparé (brouillon, à émettre).",
                euros(d.facture.total_centimes)
            ))
        });
    });
    view! {
        <details class="pile">
            <summary>"Créer un avoir"</summary>
            <ChampTexte id="fin-avoir-motif" libelle="Motif" valeur=motif requis=true/>
            <ChampTexte id="fin-avoir-montant" libelle="Montant (vide : avoir total)" valeur=montant inputmode="decimal"/>
            <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>"Préparer l'avoir"</Bouton>
            <Confirmation ouvert=confirmer id="fin-avoir-confirmer" titre="Préparer cet avoir ?"
                message="L'avoir est créé en brouillon ; il sera imputé sur la facture à son émission."
                libelle_confirmer="Préparer" on_confirmer=creer/>
            <RetourEcriture ecriture=ecriture/>
        </details>
    }
}

#[component]
fn FormReglement(foyer: Uuid, recharger: Callback<()>) -> impl IntoView {
    let montant = RwSignal::new(String::new());
    let moyen = RwSignal::new("cheque".to_string());
    let reference = RwSignal::new(String::new());
    let ecriture = Ecriture::nouveau(recharger);
    let enregistrer = Callback::new(move |_| {
        let montant_centimes = match centimes_depuis_saisie(&montant.get_untracked()) {
            Ok(c) => c,
            Err(e) => return ecriture.erreur.set(Some(e)),
        };
        let r = reference.get_untracked();
        let b = ReglementBody {
            foyer_id: foyer,
            montant_centimes,
            moyen: moyen.get_untracked(),
            reference: (!r.trim().is_empty()).then_some(r),
            recu_le: None,
        };
        ecriture.lancer(async move {
            let r = session::client().enregistrer_reglement(&b).await?;
            Ok(format!(
                "Règlement de {} imputé sur {} facture(s){}.",
                euros(r.montant_centimes),
                r.imputations.len(),
                if r.non_impute_centimes > 0 {
                    format!(" ; {} au crédit du foyer", euros(r.non_impute_centimes))
                } else {
                    String::new()
                }
            ))
        });
    });
    view! {
        <details class="pile">
            <summary>"Enregistrer un règlement du foyer"</summary>
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); enregistrer.run(()); }>
                <ChampTexte id="fin-reg-montant" libelle="Montant reçu" valeur=montant inputmode="decimal" requis=true/>
                <Selection id="fin-reg-moyen" libelle="Moyen" valeur=moyen
                    options=MOYENS.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect()/>
                <ChampTexte id="fin-reg-ref" libelle="Référence (n° de chèque, virement…)" valeur=reference/>
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>"Enregistrer"</Bouton>
            </form>
            <RetourEcriture ecriture=ecriture/>
        </details>
    }
}

// ---------------------------------------------------------------------------
// Prélèvements et rejets
// ---------------------------------------------------------------------------

#[component]
fn OngletPrelevements(gestion: Signal<bool>) -> impl IntoView {
    let prelevements = LocalResource::new(|| async { session::client().prelevements(None).await });
    let rejets = LocalResource::new(|| async { session::client().rejets().await });
    let recharger = Callback::new(move |_| {
        prelevements.refetch();
        rejets.refetch();
    });
    let date_exec = RwSignal::new(String::new());
    let format = RwSignal::new("pain008".to_string());
    let fichier = RwSignal::new(None::<(String, String)>);
    let ecriture = Ecriture::nouveau(recharger);
    let preparer = Callback::new(move |_| {
        let Some(d) = date(&date_exec.get_untracked()) else {
            return ecriture
                .erreur
                .set(Some("Date d'exécution attendue.".into()));
        };
        ecriture.lancer(async move {
            let r = session::client().preparer_prelevements(&PreparationBody { date_execution: d }).await?;
            Ok(format!(
                "{} prélèvement(s) préparé(s) pour {}, {} pré-notification(s) envoyée(s) ; {} foyer(s) écarté(s).",
                r.prelevements_crees, euros(r.montant_centimes), r.prenotifications, r.ecartes.len()
            ))
        });
    });
    let exporter = Callback::new(move |_| {
        let Some(d) = date(&date_exec.get_untracked()) else {
            return ecriture
                .erreur
                .set(Some("Date d'exécution attendue.".into()));
        };
        let f = format.get_untracked();
        ecriture.lancer(async move {
            let contenu = session::client()
                .exporter_prelevements(&ExportPrelevementsBody {
                    date_execution: d,
                    format: Some(f.clone()),
                })
                .await?;
            let (mime, ext) = if f == "csv" {
                ("text/csv", "csv")
            } else {
                ("application/xml", "xml")
            };
            fichier.set(Some((
                lien_telechargement(&contenu, mime),
                format!("prelevements-{d}.{ext}"),
            )));
            Ok(
                "Fichier de prélèvements prêt : enregistrez-le puis remettez-le à la banque."
                    .into(),
            )
        });
    });
    let executer = Callback::new(move |_| {
        let Some(d) = date(&date_exec.get_untracked()) else {
            return ecriture
                .erreur
                .set(Some("Date d'exécution attendue.".into()));
        };
        ecriture.lancer(async move {
            let r = session::client()
                .constater_execution(&ExecutionBody { date_execution: d })
                .await?;
            Ok(format!(
                "{} prélèvement(s) encaissé(s), {}.",
                r.executes,
                euros(r.montant_centimes)
            ))
        });
    });
    view! {
        <Show when=move || gestion.get()>
            <section class="pile carte" aria-labelledby="fin-sepa">
                <h2 id="fin-sepa">"Lot de prélèvements"</h2>
                <p>"Préparer : une ligne par échéance arrivée à terme, pré-notification envoyée aux payeurs (délai paramétré). Exporter : fichier remis à la banque. Exécution : à constater après la date, sans rejet reçu."</p>
                <ChampTexte id="fin-sepa-date" libelle="Date d'exécution" genre="date" valeur=date_exec requis=true/>
                <Selection id="fin-sepa-format" libelle="Format du fichier" valeur=format options=vec![
                    ("pain008".to_string(), "SEPA pain.008 (XML)".to_string()),
                    ("csv".to_string(), "CSV".to_string()),
                ]/>
                <div class="fin-rangee">
                    <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=preparer>"Préparer et pré-notifier"</Bouton>
                    <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=exporter>"Exporter le fichier"</Bouton>
                    <Bouton variante=Variante::Discret en_cours=ecriture.en_cours on_click=executer>"Constater l'exécution"</Bouton>
                </div>
                <RetourEcriture ecriture=ecriture/>
                {move || fichier.get().map(|(href, nom)| view! {
                    <p><a href=href download=nom.clone()>{format!("Enregistrer {nom}")}</a></p>
                })}
            </section>
        </Show>
        <section class="pile" aria-labelledby="fin-prelevements">
            <h2 id="fin-prelevements">"Prélèvements"</h2>
            {move || match prelevements.get() {
                None => view! { <Squelette lignes=5 libelle="Chargement des prélèvements…"/> }.into_any(),
                Some(Err(e)) if refus(&e) => view! { <AccesRefuse message="Accès réservé à la comptabilité."/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun prélèvement" message="Préparez un lot à une date d'exécution."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <div class="fin-defilement"><table class="fin-tableau">
                        <thead><tr><th scope="col">"Foyer"</th><th scope="col">"Facture"</th><th scope="col">"Date"</th>
                            <th scope="col">"Montant"</th><th scope="col">"IBAN"</th><th scope="col">"État"</th><th scope="col">"Rejet"</th></tr></thead>
                        <tbody>
                            {l.into_iter().map(|p| {
                                let exporte = p.statut == "exporte";
                                view! {
                                    <tr>
                                        <th scope="row">{p.foyer.clone()}</th>
                                        <td>{p.facture_numero.clone()}</td>
                                        <td>{p.date_execution.format("%d/%m/%Y").to_string()}</td>
                                        <td class="fin-nombre">{euros(p.montant_centimes)}</td>
                                        <td>{p.iban_masque.clone()}</td>
                                        <td>{libelle_prelevement(&p.statut)}{(p.presentation > 1).then(|| format!(" (présentation {})", p.presentation))}</td>
                                        <td>{(exporte && gestion.get_untracked()).then(|| view! { <SaisieRejet prelevement=p.id recharger=recharger/> })}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table></div>
                }.into_any(),
            }}
        </section>
        <section class="pile" aria-labelledby="fin-rejets">
            <h2 id="fin-rejets">"Rejets traités"</h2>
            {move || match rejets.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des rejets…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun rejet" message="Aucun rejet de prélèvement n'a été traité."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <div class="fin-defilement"><table class="fin-tableau">
                        <thead><tr><th scope="col">"Foyer"</th><th scope="col">"Motif"</th><th scope="col">"Montant"</th><th scope="col">"Frais"</th><th scope="col">"Suite"</th></tr></thead>
                        <tbody>
                            {l.into_iter().map(|r| view! {
                                <tr>
                                    <th scope="row">{r.foyer}</th>
                                    <td>{format!("{} — {}", r.code_motif, r.libelle_motif)}</td>
                                    <td class="fin-nombre">{euros(r.montant_centimes)}</td>
                                    <td class="fin-nombre">{euros(r.frais_centimes)}</td>
                                    <td>{libelle_suite(&r.suite)}</td>
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table></div>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn SaisieRejet(prelevement: Uuid, recharger: Callback<()>) -> impl IntoView {
    let code = RwSignal::new("AM04".to_string());
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(recharger);
    let id_sel = format!("fin-rejet-{}", prelevement.simple());
    let id_dlg = format!("fin-rejet-dlg-{}", prelevement.simple());
    let traiter = Callback::new(move |_| {
        let c = code.get_untracked();
        ecriture.lancer(async move {
            let r = session::client().traiter_rejet(prelevement, &c).await?;
            Ok(format!("Rejet traité : {}.", libelle_suite(&r.suite)))
        });
    });
    view! {
        <Selection id=id_sel libelle="Motif" valeur=code
            options=CODES_REJET.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect()/>
        <Bouton taille=Taille::Compacte variante=Variante::Danger en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>"Enregistrer le rejet"</Bouton>
        <Confirmation ouvert=confirmer id=id_dlg titre="Enregistrer ce rejet ?" danger=true
            message="L'échéance est rouverte, le payeur est prévenu ; des frais peuvent être facturés et, selon le motif, le foyer repasse au paiement manuel."
            libelle_confirmer="Enregistrer le rejet" on_confirmer=traiter/>
        <RetourEcriture ecriture=ecriture/>
    }
}

// ---------------------------------------------------------------------------
// Relances (confirmation humaine)
// ---------------------------------------------------------------------------

#[component]
fn OngletRelances(gestion: Signal<bool>) -> impl IntoView {
    let relances = LocalResource::new(|| async { session::client().relances(None).await });
    let recharger = Callback::new(move |_| relances.refetch());
    let ecriture = Ecriture::nouveau(recharger);
    let proposer = Callback::new(move |_| {
        ecriture.lancer(async move {
            let r = session::client().proposer_relances().await?;
            Ok(format!(
                "{} relance(s) proposée(s) : rien n'est envoyé avant votre confirmation.",
                r.proposees.len()
            ))
        });
    });
    view! {
        <Show when=move || gestion.get()>
            <div class="fin-rangee">
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=proposer>"Rechercher les retards"</Bouton>
            </div>
        </Show>
        <RetourEcriture ecriture=ecriture/>
        {move || match relances.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des relances…"/> }.into_any(),
            Some(Err(e)) if refus(&e) => view! { <AccesRefuse message="Accès réservé à la comptabilité."/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune relance" message="Aucune échéance en retard n'a été relevée."/> }.into_any(),
            Some(Ok(l)) => view! {
                <div class="fin-defilement"><table class="fin-tableau">
                    <thead><tr><th scope="col">"Foyer"</th><th scope="col">"Facture"</th><th scope="col">"Palier"</th>
                        <th scope="col">"Dû"</th><th scope="col">"Retard"</th><th scope="col">"État"</th><th scope="col">"Décision"</th></tr></thead>
                    <tbody>
                        {l.into_iter().map(|r| {
                            let id = r.id;
                            let proposee = r.statut == "proposee";
                            let decider = move |confirmer: bool| {
                                ecriture.lancer(async move {
                                    let r = session::client().decider_relance(id, confirmer).await?;
                                    Ok(format!("Relance « {} » : {}.", r.libelle, libelle_relance(&r.statut).to_lowercase()))
                                });
                            };
                            view! {
                                <tr>
                                    <th scope="row">{r.foyer.clone()}</th>
                                    <td>{r.facture_numero.clone()}</td>
                                    <td>{format!("{} — {}", r.palier, r.libelle)}</td>
                                    <td class="fin-nombre">{euros(r.montant_du_centimes)}</td>
                                    <td>{format!("{} j", r.jours_retard)}</td>
                                    <td><Badge libelle=libelle_relance(&r.statut) ton=if proposee { TonEtat::Avertissement } else { TonEtat::Neutre }/></td>
                                    <td>{(proposee && gestion.get_untracked()).then(|| view! {
                                        <Bouton taille=Taille::Compacte variante=Variante::Primaire on_click=Callback::new(move |_| decider(true))>"Envoyer"</Bouton>
                                        <Bouton taille=Taille::Compacte variante=Variante::Discret on_click=Callback::new(move |_| decider(false))>"Ne pas envoyer"</Bouton>
                                    })}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table></div>
            }.into_any(),
        }}
    }
}

// ---------------------------------------------------------------------------
// Paramétrage : paramètres, articles, tarifs
// ---------------------------------------------------------------------------

#[component]
fn OngletParametrage(gestion: Signal<bool>) -> impl IntoView {
    let params = LocalResource::new(|| async { session::client().parametres_finances().await });
    let articles = LocalResource::new(|| async { session::client().articles_finances().await });
    let annee = RwSignal::new(String::new());
    let tarifs = LocalResource::new(move || {
        let a = annee.get();
        async move {
            if annee_valide(&a) {
                session::client().tarifs(a.trim()).await.map(Some)
            } else {
                Ok(None)
            }
        }
    });
    let recharger = Callback::new(move |_| {
        params.refetch();
        articles.refetch();
        tarifs.refetch();
    });
    view! {
        {move || match params.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des paramètres…"/> }.into_any(),
            Some(Err(e)) if refus(&e) => view! { <AccesRefuse message="Accès réservé à la comptabilité."/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(p)) => view! {
                <section class="pile carte" aria-labelledby="fin-params">
                    <h2 id="fin-params">"Paramètres"</h2>
                    <ul>
                        <li>{format!("Année facturée : {} mois à partir du mois {}", p.nb_mois, p.mois_debut)}</li>
                        <li>{format!("Pré-notification : {} jours avant le prélèvement", p.jours_prenotification)}</li>
                        <li>{format!("Frais de rejet : {} ; retour au paiement manuel après {} rejet(s)", euros(p.frais_rejet_centimes), p.rejets_avant_manuel)}</li>
                        <li>{format!("Remise d'ordre : absence justifiée de {} jours ou plus", p.seuil_absence_jours)}</li>
                        <li>{format!("Créancier SEPA : {} — {} — ICS {}", p.crediteur_nom.clone().unwrap_or_else(|| "non renseigné".into()),
                            p.crediteur_iban.clone().unwrap_or_else(|| "IBAN non renseigné".into()), p.crediteur_ics.clone().unwrap_or_default())}</li>
                        <li>{format!("Relances : {}", p.paliers.iter().map(|x| format!("{} à {} j", x.libelle, x.jours_retard)).collect::<Vec<_>>().join(", "))}</li>
                    </ul>
                    <Show when=move || gestion.get()>
                        <FormCrediteur recharger=recharger/>
                    </Show>
                </section>
            }.into_any(),
        }}
        <section class="pile" aria-labelledby="fin-articles">
            <h2 id="fin-articles">"Articles"</h2>
            {move || match articles.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des articles…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul>{l.into_iter().map(|a| view! { <li>{format!("{} — {} ({}, compte {})", a.code, a.libelle, a.categorie, a.compte_produit)}</li> }).collect_view()}</ul>
                    <Show when=move || gestion.get()><FormArticle recharger=recharger/></Show>
                }.into_any(),
            }}
        </section>
        <section class="pile" aria-labelledby="fin-tarifs">
            <h2 id="fin-tarifs">"Grille tarifaire"</h2>
            <ChampTexte id="fin-tarifs-annee" libelle="Année scolaire" valeur=annee placeholder="2027-2028"/>
            {move || match tarifs.get() {
                None | Some(Ok(None)) => view! { <p>"Indiquez une année pour afficher sa grille."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(Some(l))) => view! {
                    <div class="fin-defilement"><table class="fin-tableau">
                        <thead><tr><th scope="col">"Article"</th><th scope="col">"Niveau"</th><th scope="col">"Régime"</th><th scope="col">"Quotient"</th><th scope="col">"Montant annuel"</th></tr></thead>
                        <tbody>{l.into_iter().map(|t| view! {
                            <tr>
                                <th scope="row">{format!("{} ({})", t.article_libelle, t.categorie)}</th>
                                <td>{t.niveau.unwrap_or_else(|| "tous".into())}</td>
                                <td>{t.regime.unwrap_or_else(|| "tous".into())}</td>
                                <td>{match (t.qf_min_centimes, t.qf_max_centimes) {
                                    (None, None) => "tous".to_string(),
                                    (a, b) => format!("{} → {}", a.map(euros).unwrap_or_default(), b.map(euros).unwrap_or_else(|| "…".into())),
                                }}</td>
                                <td class="fin-nombre">{euros(t.montant_annuel_centimes)}</td>
                            </tr>
                        }).collect_view()}</tbody>
                    </table></div>
                    <Show when=move || gestion.get()><FormTarif annee=annee recharger=recharger/></Show>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn FormCrediteur(recharger: Callback<()>) -> impl IntoView {
    let nom = RwSignal::new(String::new());
    let iban = RwSignal::new(String::new());
    let bic = RwSignal::new(String::new());
    let ics = RwSignal::new(String::new());
    let frais = RwSignal::new(String::new());
    let ecriture = Ecriture::nouveau(recharger);
    let enregistrer = Callback::new(move |_| {
        let opt = |s: RwSignal<String>| {
            let v = s.get_untracked();
            (!v.trim().is_empty()).then_some(v)
        };
        let frais_rejet_centimes = match opt(frais) {
            None => None,
            Some(f) => match centimes_depuis_saisie(&f) {
                Ok(c) => Some(c),
                Err(e) => return ecriture.erreur.set(Some(e)),
            },
        };
        let b = ParametresFinancesBody {
            crediteur_nom: opt(nom),
            crediteur_iban: opt(iban),
            crediteur_bic: opt(bic),
            crediteur_ics: opt(ics),
            frais_rejet_centimes,
            ..Default::default()
        };
        ecriture.lancer(async move {
            session::client().modifier_parametres_finances(&b).await?;
            Ok("Paramètres enregistrés.".into())
        });
    });
    view! {
        <details class="pile">
            <summary>"Modifier le créancier SEPA et les frais"</summary>
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); enregistrer.run(()); }>
                <ChampTexte id="fin-cr-nom" libelle="Nom du créancier" valeur=nom/>
                <ChampTexte id="fin-cr-iban" libelle="IBAN de l'établissement" valeur=iban aide="Jamais réaffiché en clair."/>
                <ChampTexte id="fin-cr-bic" libelle="BIC" valeur=bic/>
                <ChampTexte id="fin-cr-ics" libelle="Identifiant créancier SEPA (ICS)" valeur=ics/>
                <ChampTexte id="fin-cr-frais" libelle="Frais par rejet" valeur=frais inputmode="decimal"/>
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>"Enregistrer"</Bouton>
            </form>
            <RetourEcriture ecriture=ecriture/>
        </details>
    }
}

#[component]
fn FormArticle(recharger: Callback<()>) -> impl IntoView {
    let code = RwSignal::new(String::new());
    let libelle = RwSignal::new(String::new());
    let categorie = RwSignal::new("scolarite".to_string());
    let compte = RwSignal::new(String::new());
    let ecriture = Ecriture::nouveau(recharger);
    let creer = Callback::new(move |_| {
        let c = compte.get_untracked();
        let b = ArticleBody {
            code: code.get_untracked(),
            libelle: libelle.get_untracked(),
            categorie: categorie.get_untracked(),
            compte_produit: (!c.trim().is_empty()).then_some(c),
            actif: None,
        };
        ecriture.lancer(async move {
            let a = session::client().creer_article(&b).await?;
            Ok(format!("Article {} créé.", a.code))
        });
    });
    view! {
        <details class="pile">
            <summary>"Nouvel article"</summary>
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); creer.run(()); }>
                <ChampTexte id="fin-art-code" libelle="Code" valeur=code requis=true/>
                <ChampTexte id="fin-art-libelle" libelle="Libellé" valeur=libelle requis=true/>
                <Selection id="fin-art-cat" libelle="Catégorie" valeur=categorie options=vec![
                    ("scolarite".to_string(), "Scolarité".to_string()),
                    ("restauration".to_string(), "Restauration (demi-pension, internat)".to_string()),
                    ("internat".to_string(), "Internat".to_string()),
                    ("frais".to_string(), "Frais annuels".to_string()),
                    ("activite".to_string(), "Activité (consommation)".to_string()),
                ]/>
                <ChampTexte id="fin-art-compte" libelle="Compte de produit" valeur=compte placeholder="706000"/>
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>"Créer"</Bouton>
            </form>
            <RetourEcriture ecriture=ecriture/>
        </details>
    }
}

#[component]
fn FormTarif(annee: RwSignal<String>, recharger: Callback<()>) -> impl IntoView {
    let articles = LocalResource::new(|| async { session::client().articles_finances().await });
    let article = RwSignal::new(String::new());
    let regime = RwSignal::new(String::new());
    let niveau = RwSignal::new(String::new());
    let montant = RwSignal::new(String::new());
    let ecriture = Ecriture::nouveau(recharger);
    let creer = Callback::new(move |_| {
        let Ok(article_id) = Uuid::parse_str(&article.get_untracked()) else {
            return ecriture.erreur.set(Some("Choisissez un article.".into()));
        };
        let montant_annuel_centimes = match centimes_depuis_saisie(&montant.get_untracked()) {
            Ok(c) => c,
            Err(e) => return ecriture.erreur.set(Some(e)),
        };
        let (r, n) = (regime.get_untracked(), niveau.get_untracked());
        let b = TarifBody {
            annee_scolaire: annee.get_untracked().trim().to_string(),
            article_id,
            niveau: (!n.trim().is_empty()).then_some(n),
            regime: (!r.is_empty()).then_some(r),
            qf_min_centimes: None,
            qf_max_centimes: None,
            montant_annuel_centimes,
        };
        ecriture.lancer(async move {
            session::client().creer_tarif(&b).await?;
            Ok("Tarif ajouté.".into())
        });
    });
    view! {
        <details class="pile">
            <summary>"Ajouter un tarif"</summary>
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); creer.run(()); }>
                {move || articles.get().and_then(|r| r.ok()).map(|l| {
                    let mut options = vec![(String::new(), "— choisir —".to_string())];
                    options.extend(l.into_iter().map(|a| (a.id.to_string(), format!("{} — {}", a.code, a.libelle))));
                    view! { <Selection id="fin-tarif-article" libelle="Article" valeur=article options=options/> }
                })}
                <Selection id="fin-tarif-regime" libelle="Régime" valeur=regime options=vec![
                    (String::new(), "Tous".to_string()),
                    ("externe".to_string(), "Externe".to_string()),
                    ("demi_pension".to_string(), "Demi-pensionnaire".to_string()),
                    ("interne".to_string(), "Interne".to_string()),
                ]/>
                <ChampTexte id="fin-tarif-niveau" libelle="Niveau (vide : tous)" valeur=niveau/>
                <ChampTexte id="fin-tarif-montant" libelle="Montant annuel" valeur=montant inputmode="decimal" requis=true/>
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>"Ajouter"</Bouton>
            </form>
            <RetourEcriture ecriture=ecriture/>
        </details>
    }
}

// ---------------------------------------------------------------------------
// Export comptable
// ---------------------------------------------------------------------------

#[component]
fn OngletExport(gestion: Signal<bool>) -> impl IntoView {
    let exports = LocalResource::new(|| async { session::client().exports_comptables().await });
    let recharger = Callback::new(move |_| exports.refetch());
    let debut = RwSignal::new(String::new());
    let fin = RwSignal::new(String::new());
    let fichier = RwSignal::new(None::<(String, String)>);
    let confirmer = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(recharger);
    let exporter = Callback::new(move |_| {
        let (Some(d), Some(f)) = (date(&debut.get_untracked()), date(&fin.get_untracked())) else {
            return ecriture
                .erreur
                .set(Some("Période : dates de début et de fin attendues.".into()));
        };
        ecriture.lancer(async move {
            let e = session::client()
                .exporter_comptabilite(&ExportComptableBody { debut: d, fin: f })
                .await?;
            if let Some(c) = e.contenu.as_deref() {
                fichier.set(Some((
                    lien_telechargement(c, "text/plain"),
                    format!("{}.txt", e.numero),
                )));
            }
            Ok(format!(
                "Export {} : {} écritures, équilibré ({}).",
                e.numero,
                e.nb_ecritures,
                euros(e.total_debit_centimes)
            ))
        });
    });
    view! {
        <Show when=move || gestion.get()>
            <section class="pile carte" aria-labelledby="fin-export">
                <h2 id="fin-export">"Nouvel export"</h2>
                <p>"Période close seulement. Chaque pièce n'est exportée qu'une fois ; format scolia_ecritures_v1 (colonnes du fichier des écritures comptables)."</p>
                <ChampTexte id="fin-exp-debut" libelle="Du" genre="date" valeur=debut requis=true/>
                <ChampTexte id="fin-exp-fin" libelle="Au" genre="date" valeur=fin requis=true/>
                <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>"Exporter"</Bouton>
                <Confirmation ouvert=confirmer id="fin-exp-confirmer" titre="Exporter la période ?"
                    message="Les pièces exportées ne seront plus reprises dans un export suivant." libelle_confirmer="Exporter" on_confirmer=exporter/>
                <RetourEcriture ecriture=ecriture/>
                {move || fichier.get().map(|(href, nom)| view! { <p><a href=href download=nom.clone()>{format!("Enregistrer {nom}")}</a></p> })}
            </section>
        </Show>
        <section class="pile" aria-labelledby="fin-exports">
            <h2 id="fin-exports">"Exports réalisés"</h2>
            {move || match exports.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des exports…"/> }.into_any(),
                Some(Err(e)) if refus(&e) => view! { <AccesRefuse message="Accès réservé à la comptabilité."/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun export" message="Aucun export comptable n'a encore été produit."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <div class="fin-defilement"><table class="fin-tableau">
                        <thead><tr><th scope="col">"Export"</th><th scope="col">"Période"</th><th scope="col">"Écritures"</th><th scope="col">"Total"</th><th scope="col">"Empreinte"</th></tr></thead>
                        <tbody>{l.into_iter().map(|e| view! {
                            <tr>
                                <th scope="row">{e.numero}</th>
                                <td>{format!("{} → {}", e.debut.format("%d/%m/%Y"), e.fin.format("%d/%m/%Y"))}</td>
                                <td>{e.nb_ecritures}</td>
                                <td class="fin-nombre">{euros(e.total_debit_centimes)}</td>
                                <td><code>{e.empreinte.chars().take(16).collect::<String>()}"…"</code></td>
                            </tr>
                        }).collect_view()}</tbody>
                    </table></div>
                }.into_any(),
            }}
        </section>
    }
}
