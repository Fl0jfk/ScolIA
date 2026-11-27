//! Espace famille des finances (`/web/mes-factures`, FIN-17) : comptes des foyers
//! dont la personne connectée est **payeuse** (lien actif). Pièces émises seulement,
//! échéancier, solde, IBAN masqué. Le paiement en ligne n'est pas proposé tant que
//! la décision Q5 n'est pas prise : l'écran affiche le message du serveur.

use ent_web_api::types_finances::CompteFoyerDto;
use ent_web_api::ApiError;
use ent_web_ui::{AccesRefuse, Alerte, Badge, EnTetePage, EtatVide, Niveau, Squelette};
use leptos::prelude::*;
use leptos_meta::Title;

use super::modele::{
    euros, libelle_echeance, libelle_mode, libelle_statut, ton_echeance, ton_statut,
};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageFinancesFamille() -> impl IntoView {
    let espace = LocalResource::new(|| async { session::client().finances_famille().await });
    let recharger = Callback::new(move |_| espace.refetch());
    view! {
        <Title text="Factures"/>
        <EnTetePage titre="Factures et paiements" sous_titre="Les comptes dont vous êtes payeur dans l'établissement".to_string()/>
        {move || match espace.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de vos factures…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! { <AccesRefuse message="Cet espace est réservé aux responsables payeurs."/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(e)) if e.comptes.is_empty() => view! {
                <EtatVide titre="Aucun compte" message="Vous n'êtes désigné comme payeur pour aucun élève de l'établissement."/>
            }.into_any(),
            Some(Ok(e)) => view! {
                {e.paiement_en_ligne.map(|p| view! { <Alerte niveau=Niveau::Info titre=p.message/> })}
                {e.comptes.into_iter().map(|c| view! { <Compte compte=c/> }).collect_view()}
            }.into_any(),
        }}
    }
}

#[component]
fn Compte(compte: CompteFoyerDto) -> impl IntoView {
    let titre_id = format!("fam-compte-{}", compte.foyer_id.simple());
    let titre_ref = titre_id.clone();
    view! {
        <section class="pile carte" aria-labelledby=titre_ref>
            <h2 id=titre_id>{compte.foyer.clone()}</h2>
            <p class="fin-solde">
                <span>"Reste à payer : "<strong>{euros(compte.solde_centimes)}</strong></span>
                {(compte.credit_centimes > 0).then(|| view! { <span>"Crédit en votre faveur : "<strong>{euros(compte.credit_centimes)}</strong></span> })}
                <span>{libelle_mode(&compte.mode_paiement)}</span>
            </p>
            {(!compte.mandats.is_empty()).then(|| view! {
                <ul aria-label="Mandats de prélèvement">
                    {compte.mandats.iter().map(|m| view! {
                        <li>{format!("Mandat {} — {} — {}", m.rum, m.iban_masque, if m.statut == "actif" { "actif" } else { "révoqué" })}</li>
                    }).collect_view()}
                </ul>
            })}
            {if compte.echeances.is_empty() {
                view! { <p>"Aucune échéance."</p> }.into_any()
            } else {
                view! {
                    <div class="fin-defilement"><table class="fin-tableau">
                        <caption>"Échéancier"</caption>
                        <thead><tr><th scope="col">"Date"</th><th scope="col">"Facture"</th><th scope="col">"Montant"</th><th scope="col">"État"</th></tr></thead>
                        <tbody>
                            {compte.echeances.iter().map(|e| view! {
                                <tr>
                                    <th scope="row">{e.echeance.date_echeance.format("%d/%m/%Y").to_string()}</th>
                                    <td>{e.facture_numero.clone()}</td>
                                    <td class="fin-nombre">{euros(e.echeance.montant_centimes)}</td>
                                    <td><Badge libelle=libelle_echeance(&e.echeance.etat) ton=ton_echeance(&e.echeance.etat)/></td>
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table></div>
                }.into_any()
            }}
            <div class="fin-defilement"><table class="fin-tableau">
                <caption>"Factures et avoirs"</caption>
                <thead><tr><th scope="col">"Pièce"</th><th scope="col">"Libellé"</th><th scope="col">"Montant"</th><th scope="col">"Reste"</th><th scope="col">"État"</th></tr></thead>
                <tbody>
                    {compte.factures.iter().map(|f| view! {
                        <tr>
                            <th scope="row">{f.numero.clone()}</th>
                            <td>{f.libelle.clone()}</td>
                            <td class="fin-nombre">{euros(f.total_centimes)}</td>
                            <td class="fin-nombre">{euros(f.reste_centimes)}</td>
                            <td><Badge libelle=libelle_statut(&f.statut, &f.type_piece) ton=ton_statut(&f.statut)/></td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table></div>
        </section>
    }
}
