//! Grille de notes du professeur (socle 21, lot V3) : élèves × évaluations, saisie au clavier continue,
//! codes rapides, collage depuis un tableur, import d'un fichier, conflits de version résolus explicitement,
//! création et réglage d'une évaluation, publication, historique d'une note.
//!
//! * Le serveur décide de tout (droits, public à la date, période de chaque élève, moteur de calcul) ;
//!   l'écran envoie le **texte tapé** et la **version lue** de chaque case, et affiche ce qui revient.
//! * Aucune moyenne n'est calculée ici : après chaque enregistrement, la grille est relue et les moyennes
//!   viennent du moteur du serveur.
//! * Clavier : flèches, Entrée (bas), Maj+Entrée (haut), Tab (case suivante), Échap (annule la frappe) ;
//!   codes `abs`, `ar`, `disp`, `nr`, `nn` ; 375 px : la première colonne reste collée à gauche et le tableau
//!   défile horizontalement dans son cadre (jamais la page).

pub mod modele;

use chrono::{NaiveDate, NaiveDateTime};
use ent_types::evaluations_v3::{
    AnnulerEvaluationV3Body, CreerEvaluationV3Body, EnseignementV3Dto, EvaluationColonneDto,
    GrilleV3Dto, ImportApercuDto, ImportBody, LigneHistoriqueV3Dto, ModifierEvaluationV3Body,
    PeriodeV3Dto, PublierV3Body, SaisieLotBody, SousMatiereV3Dto, StatsV3Dto, CODES_RAPIDES,
};
use ent_web_ui::{
    Alerte, Annonce, Badge, Bouton, Case, ChampTexte, Dialogue, EnTetePage, EtatVide, Niveau,
    Selection, Squelette, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use modele::{
    bilan_lot, cases_a_envoyer, cible_touche, controler_evaluation, effet_retour, est_un_bloc,
    id_case, lire_tableur, milliemes_formulaire, moyenne_affichee, phrase_conflit, plan_collage,
    plan_fichier, texte_cellule, texte_conflit, CaseVue, EffetRetour,
};

use crate::noyau::ecriture::{message_echec, Ecriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

/// Délai de la frappe continue avant l'enregistrement automatique (ms).
const DELAI_AUTO_MS: u64 = 900;

fn date_courte(d: NaiveDate) -> String {
    d.format("%d/%m").to_string()
}

fn libelle_etat_publication(e: &EvaluationColonneDto) -> (&'static str, TonEtat) {
    if e.etat == "annulee" {
        ("Annulée", TonEtat::Mal)
    } else if e.publiee_le.is_some() {
        ("Publiée", TonEtat::Ok)
    } else if e.retiree_le.is_some() {
        ("Retirée pour vérification", TonEtat::Avertissement)
    } else if e.publication == "programmee" {
        ("Publication programmée", TonEtat::Info)
    } else if e.publication == "a_l_arret" {
        ("Publiée à l'arrêt des notes", TonEtat::Info)
    } else if e.en_cours_de_correction {
        ("En cours de correction", TonEtat::Avertissement)
    } else {
        ("Non publiée", TonEtat::Neutre)
    }
}

// ───────────────────────── Page ─────────────────────────

#[component]
pub fn PageGrilleV3() -> impl IntoView {
    let enseignements =
        LocalResource::new(|| async { session::client().eva3_enseignements().await });
    view! {
        <Title text="Grille de notes"/>
        <EnTetePage
            surtitre="Pédagogie".to_string()
            titre="Grille de notes".to_string()
            sous_titre="Saisie au clavier, collage depuis un tableur, import d'un fichier.".to_string()
        />
        {move || match enseignements.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de vos enseignements…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enseignements.refetch())/>
            }
            .into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucun enseignement" message="Aucun cours ne vous est confié sur l'année en cours."/>
            }
            .into_any(),
            Some(Ok(l)) => view! { <ChoixEtGrille liste=l/> }.into_any(),
        }}
    }
}

#[component]
fn ChoixEtGrille(liste: Vec<EnseignementV3Dto>) -> impl IntoView {
    let q = use_query_map();
    let cours = RwSignal::new(q.with_untracked(|q| q.get("cours")).unwrap_or_default());
    let periode = RwSignal::new(q.with_untracked(|q| q.get("periode")).unwrap_or_default());
    let reload = RwSignal::new(0u32);
    let options: Vec<(String, String)> =
        std::iter::once((String::new(), "Choisir un enseignement…".to_string()))
            .chain(liste.iter().map(|e| {
                let suffixe = if e.droit == "saisie" {
                    ""
                } else {
                    " (lecture)"
                };
                (
                    e.cours_id.to_string(),
                    format!("{} · {}{}", e.cible_code, e.libelle, suffixe),
                )
            }))
            .collect();
    let periodes = LocalResource::new(move || {
        let c = cours.get();
        async move {
            match Uuid::parse_str(&c) {
                Ok(c) => session::client().eva3_periodes(c).await,
                Err(_) => Ok(Vec::new()),
            }
        }
    });
    // Période par défaut : celle du jour (posée après l'hydratation), sinon la première.
    Effect::new(move |_| {
        if let Some(Ok(l)) = periodes.get() {
            let valide = Uuid::parse_str(&periode.get_untracked())
                .ok()
                .is_some_and(|p| l.iter().any(|x| x.id == p));
            if !valide {
                let auj = aujourdhui();
                let choix = l
                    .iter()
                    .find(|p| p.date_debut <= auj && auj <= p.date_fin)
                    .or(l.first());
                periode.set(choix.map(|p| p.id.to_string()).unwrap_or_default());
            }
        }
    });
    let grille = LocalResource::new(move || {
        let _ = reload.get();
        let c = Uuid::parse_str(&cours.get());
        let p = Uuid::parse_str(&periode.get());
        async move {
            match (c, p) {
                (Ok(c), Ok(p)) => session::client().eva3_grille(c, p).await.map(Some),
                _ => Ok(None),
            }
        }
    });
    let sous_matieres = LocalResource::new(move || {
        let c = cours.get();
        async move {
            match Uuid::parse_str(&c) {
                Ok(c) => session::client()
                    .eva3_sous_matieres(c)
                    .await
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        }
    });
    view! {
        <div class="eva3-choix">
            <Selection id="eva3-cours" libelle="Enseignement" valeur=cours options=options/>
            {move || match periodes.get() {
                Some(Ok(l)) if !l.is_empty() => {
                    let opts: Vec<(String, String)> = l
                        .iter()
                        .map(|p: &PeriodeV3Dto| (p.id.to_string(), format!("{} ({} – {})", p.libelle, date_courte(p.date_debut), date_courte(p.date_fin))))
                        .collect();
                    view! { <Selection id="eva3-periode" libelle="Période" valeur=periode options=opts/> }.into_any()
                }
                _ => view! { <span></span> }.into_any(),
            }}
        </div>
        {move || match grille.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la grille…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| grille.refetch())/>
            }
            .into_any(),
            Some(Ok(None)) => view! {
                <EtatVide titre="Choisissez un enseignement" message="La grille s'affiche dès qu'un enseignement et une période sont choisis."/>
            }
            .into_any(),
            Some(Ok(Some(g))) => {
                let sm = sous_matieres.get().unwrap_or_default();
                view! {
                    <TableauV3
                        grille=g
                        sous_matieres=sm
                        recharger=Callback::new(move |_| reload.update(|n| *n += 1))
                    />
                }
                .into_any()
            }
        }}
    }
}

// ───────────────────────── État d'une grille ─────────────────────────

#[derive(Clone, PartialEq)]
enum EtatCase {
    Normal,
    Refusee(String),
    Conflit(ent_types::evaluations_v3::ConflitDto),
}

/// Explication d'une moyenne : lignes du calcul et avertissements, ou message d'échec.
type Explication = Result<(Vec<String>, Vec<String>), String>;

/// Une case : un signal par élément modifiable, seule la case touchée se redessine.
#[derive(Clone, Copy)]
struct CaseSig {
    evaluation_id: Uuid,
    eleve_id: Uuid,
    concerne: bool,
    saisie_ouverte: bool,
    texte: RwSignal<String>,
    base: RwSignal<String>,
    version: RwSignal<Option<i32>>,
    etat: RwSignal<EtatCase>,
    corrige: RwSignal<bool>,
}

impl CaseSig {
    fn vue(&self) -> CaseVue {
        CaseVue {
            evaluation_id: self.evaluation_id,
            eleve_id: self.eleve_id,
            texte: self.texte.get_untracked(),
            base: self.base.get_untracked(),
            version: self.version.get_untracked(),
            concerne: self.concerne,
            saisie_ouverte: self.saisie_ouverte,
            en_conflit: matches!(self.etat.get_untracked(), EtatCase::Conflit(_)),
        }
    }
}

#[derive(Clone)]
struct LigneSig {
    eleve_id: Uuid,
    nom: String,
    mouvement: Option<String>,
    cases: Vec<CaseSig>,
    moyenne: RwSignal<Option<i64>>,
    nb_comptees: RwSignal<usize>,
}

fn classe_case(c: &CaseSig, editable: bool, texte: &str, etat: &EtatCase) -> String {
    let mut k = String::from("eva3-case");
    if !c.concerne {
        k.push_str(" eva3-case--hors");
    } else if !editable {
        k.push_str(" eva3-case--lecture");
    }
    if c.concerne && !c.saisie_ouverte {
        k.push_str(" eva3-case--verrou");
    }
    match etat {
        EtatCase::Conflit(_) => k.push_str(" eva3-case--conflit"),
        EtatCase::Refusee(_) => k.push_str(" eva3-case--refusee"),
        EtatCase::Normal => {
            if c.concerne
                && texte.trim() != c.base.get_untracked().trim()
                && !texte.trim().is_empty()
            {
                k.push_str(" eva3-case--modifiee");
            }
        }
    }
    k
}

#[cfg(target_arch = "wasm32")]
async fn lire_texte_fichier(input: &web_sys::HtmlInputElement) -> Result<Option<String>, String> {
    let Some(fichier) = input.files().and_then(|l| l.get(0)) else {
        return Ok(None);
    };
    if fichier.size() > 2_000_000.0 {
        return Err(
            "Ce fichier est trop volumineux pour un import de notes (2 Mo au plus).".into(),
        );
    }
    let valeur = wasm_bindgen_futures::JsFuture::from(fichier.text())
        .await
        .map_err(|_| "Ce fichier n'a pas pu être lu.".to_string())?;
    valeur
        .as_string()
        .map(Some)
        .ok_or_else(|| "Ce fichier n'est pas un texte lisible.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn lire_texte_fichier(_input: &web_sys::HtmlInputElement) -> Result<Option<String>, String> {
    Ok(None)
}

fn focaliser(id: &str) {
    use leptos::wasm_bindgen::JsCast;
    if let Some(el) = document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = el.focus();
    }
}

fn curseur(ev: &leptos::ev::KeyboardEvent) -> (bool, bool) {
    use leptos::wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|i| {
            let n = i.value().chars().count() as u32;
            let (a, b) = (
                i.selection_start().ok().flatten().unwrap_or(0),
                i.selection_end().ok().flatten().unwrap_or(0),
            );
            (a == 0 && b == 0, a == n && b == n)
        })
        .unwrap_or((false, false))
}

// ───────────────────────── Le tableau ─────────────────────────

#[component]
fn TableauV3(
    grille: GrilleV3Dto,
    sous_matieres: Vec<SousMatiereV3Dto>,
    recharger: Callback<()>,
) -> impl IntoView {
    let cours_id = grille.enseignement.cours_id;
    let periode_id = grille.periode.id;
    let peut_saisir = grille.peut_saisir;
    let peut_corriger = grille.peut_corriger;
    let echelle_m = grille.echelle_m;
    let evals = StoredValue::new(grille.evaluations.clone());
    let nb_cols = grille.evaluations.len();
    let nb_lignes = grille.eleves.len();
    let stats_cols: Vec<RwSignal<Option<StatsV3Dto>>> = grille
        .evaluations
        .iter()
        .map(|e| RwSignal::new(e.stats.clone()))
        .collect();
    let stats_cols = StoredValue::new(stats_cols);
    let moyenne_classe = RwSignal::new(grille.moyenne_classe.clone());
    let avertissements = grille.avertissements.clone();
    let delai = grille.delai_publication_max_jours;

    let lignes: Vec<LigneSig> = grille
        .eleves
        .iter()
        .map(|l| LigneSig {
            eleve_id: l.eleve_id,
            nom: format!("{} {}", l.nom, l.prenom),
            mouvement: l.mouvement.clone(),
            cases: l
                .cellules
                .iter()
                .zip(grille.evaluations.iter())
                .map(|(c, e)| {
                    let t = texte_cellule(c);
                    CaseSig {
                        evaluation_id: e.id,
                        eleve_id: l.eleve_id,
                        concerne: c.concerne,
                        saisie_ouverte: c.saisie_ouverte,
                        texte: RwSignal::new(t.clone()),
                        base: RwSignal::new(t),
                        version: RwSignal::new(c.version),
                        etat: RwSignal::new(EtatCase::Normal),
                        corrige: RwSignal::new(c.corrige),
                    }
                })
                .collect(),
            moyenne: RwSignal::new(l.moyenne_m),
            nb_comptees: RwSignal::new(l.nb_comptees),
        })
        .collect();
    let lignes = StoredValue::new(lignes);

    // Contrôle de l'enregistrement continu.
    let generation = RwSignal::new(0u32);
    let en_vol = RwSignal::new(false);
    let a_refaire = RwSignal::new(false);
    let statut_enreg = RwSignal::new(String::new());
    let annonce = RwSignal::new(String::new());
    let erreur_globale: RwSignal<Option<String>> = RwSignal::new(None);
    let motif_correction = RwSignal::new(String::new());
    let selection: RwSignal<Option<(usize, usize)>> = RwSignal::new(None);

    // Relit la grille pour les moyennes (moteur du serveur) : met à jour moyennes et statistiques seulement.
    let rafraichir_moyennes = move || {
        spawn_local(async move {
            if let Ok(g) = session::client().eva3_grille(cours_id, periode_id).await {
                lignes.with_value(|ls| {
                    for (l, n) in ls.iter().zip(g.eleves.iter()) {
                        if l.eleve_id == n.eleve_id {
                            l.moyenne.set(n.moyenne_m);
                            l.nb_comptees.set(n.nb_comptees);
                        }
                    }
                });
                stats_cols.with_value(|sc| {
                    for (s, e) in sc.iter().zip(g.evaluations.iter()) {
                        s.set(e.stats.clone());
                    }
                });
                moyenne_classe.set(g.moyenne_classe);
            }
        });
    };

    let enregistrer: StoredValue<Option<Callback<()>>> = StoredValue::new(None);
    let enregistrer_cb = Callback::new(move |_: ()| {
        if en_vol.get_untracked() {
            a_refaire.set(true);
            return;
        }
        let motif = motif_correction.get_untracked();
        let motif = (!motif.trim().is_empty()).then_some(motif);
        // (ligne, colonne, vue de la case) des cases à envoyer, dans l'ordre du lot.
        let mut envoi: Vec<(usize, usize, CaseVue)> = Vec::new();
        lignes.with_value(|ls| {
            for (i, l) in ls.iter().enumerate() {
                for (j, c) in l.cases.iter().enumerate() {
                    let v = c.vue();
                    if v.modifiee() && !v.en_conflit {
                        envoi.push((i, j, v));
                    }
                }
            }
        });
        if envoi.is_empty() {
            return;
        }
        let vues: Vec<CaseVue> = envoi.iter().map(|(_, _, v)| v.clone()).collect();
        let cellules = cases_a_envoyer(&vues, motif.as_deref());
        if cellules.is_empty() {
            return;
        }
        en_vol.set(true);
        statut_enreg.set("Enregistrement…".into());
        spawn_local(async move {
            let r = session::client()
                .eva3_saisir(&SaisieLotBody {
                    cours_id,
                    cellules,
                    origine: None,
                })
                .await;
            match r {
                Ok(retour) => {
                    let mut ecrit = false;
                    lignes.with_value(|ls| {
                        for ((i, j, vue), rc) in envoi.iter().zip(retour.cellules.iter()) {
                            let c = ls[*i].cases[*j];
                            match effet_retour(rc) {
                                EffetRetour::Enregistree { version, base } => {
                                    ecrit = ecrit || rc.issue == "ecrit";
                                    c.version.set(Some(version));
                                    c.base.set(base.clone());
                                    // La frappe continue : si la personne a retapé entre-temps, on la garde.
                                    if c.texte.get_untracked().trim() == vue.texte.trim() {
                                        c.texte.set(base);
                                    }
                                    c.etat.set(EtatCase::Normal);
                                    if rc.issue == "ecrit" && vue.version.is_some() {
                                        c.corrige.set(true);
                                    }
                                }
                                EffetRetour::Vide => {}
                                EffetRetour::Conflit(x) => c.etat.set(EtatCase::Conflit(x)),
                                EffetRetour::Refusee(m) => c.etat.set(EtatCase::Refusee(m)),
                            }
                        }
                    });
                    let bilan = bilan_lot(&retour.cellules);
                    annonce.set(bilan.clone());
                    statut_enreg.set(bilan);
                    erreur_globale.set(None);
                    if ecrit {
                        rafraichir_moyennes();
                    }
                }
                Err(e) => {
                    let m = message_echec(e);
                    statut_enreg.set("Enregistrement impossible.".into());
                    erreur_globale.set(Some(m));
                }
            }
            en_vol.set(false);
            if a_refaire.get_untracked() {
                a_refaire.set(false);
                if let Some(cb) = enregistrer.get_value() {
                    cb.run(());
                }
            }
        });
    });
    enregistrer.set_value(Some(enregistrer_cb));

    // Enregistrement continu : une pause de frappe, puis un lot ; la génération évite les lots en rafale.
    let programmer = move |delai_ms: u64| {
        generation.update(|g| *g += 1);
        let g0 = generation.get_untracked();
        statut_enreg.set("Modifications en attente…".into());
        set_timeout(
            move || {
                if generation.get_untracked() == g0 {
                    enregistrer_cb.run(());
                }
            },
            std::time::Duration::from_millis(delai_ms),
        );
    };

    // Conflits à résoudre.
    let conflits = Memo::new(move |_| {
        lignes.with_value(|ls| {
            let mut out = Vec::new();
            for (i, l) in ls.iter().enumerate() {
                for (j, c) in l.cases.iter().enumerate() {
                    if let EtatCase::Conflit(x) = c.etat.get() {
                        out.push((i, j, x));
                    }
                }
            }
            out
        })
    });

    // Dialogues.
    let dlg_nouvelle = RwSignal::new(false);
    let dlg_colle = RwSignal::new(false);
    let dlg_import = RwSignal::new(false);
    let dlg_apercu = RwSignal::new(false);
    let dlg_eval = RwSignal::new(false);
    let eval_ouverte: RwSignal<Option<usize>> = RwSignal::new(None);
    let apercu: RwSignal<Option<(ImportBody, ImportApercuDto)>> = RwSignal::new(None);
    let erreur_import: RwSignal<Option<String>> = RwSignal::new(None);

    // Aperçu d'un bloc collé à partir de la case (i, j).
    let lancer_collage = move |i: usize, j: usize, texte: String| {
        let bloc = lire_tableur(&texte);
        let eleves: Vec<Uuid> = lignes.with_value(|ls| ls.iter().map(|l| l.eleve_id).collect());
        let ids: Vec<Uuid> = evals.with_value(|e| e.iter().map(|x| x.id).collect());
        match plan_collage(&bloc, i, j, &eleves, &ids) {
            Err(m) => erreur_globale.set(Some(m)),
            Ok((cibles, lignes_import)) => {
                let corps = ImportBody {
                    cours_id,
                    mode: "collage".into(),
                    cibles,
                    lignes: lignes_import,
                    empreinte: None,
                };
                spawn_local(async move {
                    match session::client().eva3_apercu_import(&corps).await {
                        Ok(a) => {
                            erreur_globale.set(None);
                            apercu.set(Some((corps, a)));
                            dlg_colle.set(false);
                            dlg_apercu.set(true);
                        }
                        Err(e) => erreur_globale.set(Some(message_echec(e))),
                    }
                });
            }
        }
    };

    let colonnes_titres: Vec<(Uuid, String)> = grille
        .evaluations
        .iter()
        .map(|e| (e.id, e.titre.clone()))
        .collect();
    let colonnes_titres = StoredValue::new(colonnes_titres);

    let titre_grille = format!(
        "{} · {}",
        grille.enseignement.cible_code, grille.enseignement.libelle
    );
    let avert_vue = avertissements
        .iter()
        .map(|a| view! { <Alerte niveau=Niveau::Avertissement titre=a.clone()/> })
        .collect_view();

    let colonnes = StoredValue::new(grille.evaluations.clone());
    let entete = move || {
        colonnes
            .get_value()
            .into_iter()
            .enumerate()
            .map(|(j, e)| {
                let (lib, ton) = libelle_etat_publication(&e);
                let titre_btn = format!("{} — réglages et publication", e.titre);
                let ouvrir = move |_| {
                    eval_ouverte.set(Some(j));
                    dlg_eval.set(true);
                };
                let stats = stats_cols.with_value(|s| s[j]);
                view! {
                    <th scope="col" class="eva3-col">
                        <button type="button" class="eva3-col__titre" aria-label=titre_btn on:click=ouvrir>
                            {e.titre.clone()}
                        </button>
                        <span class="eva3-col__meta">
                            {date_courte(e.date_eval)} " · /" {modele::moyenne_affichee(Some(e.bareme_m))}
                            " · coef " {modele::moyenne_affichee(Some(e.coef_m))}
                        </span>
                        <Badge libelle=lib ton=ton/>
                        <span class="eva3-col__stats">
                            {move || match stats.get() {
                                Some(s) => format!("moy. {}", moyenne_affichee(Some(s.moyenne_m))),
                                None => "moy. –".to_string(),
                            }}
                        </span>
                    </th>
                }
            })
            .collect_view()
    };

    let corps_tableau = move || {
        lignes
            .get_value()
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                let cellules = l
                    .cases
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(j, c)| {
                        let (titre_ev, bareme) = evals.with_value(|e| (e[j].titre.clone(), e[j].bareme_m));
                        let etiquette = format!(
                            "{}, {} sur {}{}",
                            l.nom,
                            titre_ev,
                            ent_types::evaluations_v3::formater_milliemes(bareme),
                            if c.concerne { "" } else { ", non concerné(e)" }
                        );
                        let editable = c.concerne && ((peut_saisir && c.saisie_ouverte) || (peut_corriger && !c.saisie_ouverte));
                        let id = id_case(i, j);
                        let (texte, etat) = (c.texte, c.etat);
                        view! {
                            <td class="eva3-td">
                                <input
                                    id=id
                                    type="text"
                                    autocomplete="off"
                                    spellcheck="false"
                                    class=move || classe_case(&c, editable, &texte.get(), &etat.get())
                                    aria-label=etiquette
                                    aria-invalid=move || matches!(etat.get(), EtatCase::Refusee(_) | EtatCase::Conflit(_)).then_some("true")
                                    aria-describedby="eva3-aide"
                                    readonly=!editable
                                    prop:value=move || texte.get()
                                    on:focus=move |_| selection.set(Some((i, j)))
                                    on:input=move |ev| {
                                        texte.set(event_target_value(&ev));
                                        if !matches!(etat.get_untracked(), EtatCase::Conflit(_)) {
                                            etat.set(EtatCase::Normal);
                                        }
                                        programmer(DELAI_AUTO_MS);
                                    }
                                    on:blur=move |_| {
                                        if c.vue().modifiee() {
                                            programmer(150);
                                        }
                                    }
                                    on:keydown=move |ev| {
                                        let touche = ev.key();
                                        if touche == "Escape" {
                                            texte.set(c.base.get_untracked());
                                            etat.set(EtatCase::Normal);
                                            return;
                                        }
                                        let (debut, fin) = curseur(&ev);
                                        if let Some((ni, nj)) =
                                            cible_touche(&touche, ev.shift_key(), (i, j), (nb_lignes, nb_cols), (debut, fin))
                                        {
                                            ev.prevent_default();
                                            if c.vue().modifiee() {
                                                programmer(150);
                                            }
                                            focaliser(&id_case(ni, nj));
                                        }
                                    }
                                    on:paste=move |ev: web_sys::ClipboardEvent| {
                                        let colle = ev
                                            .clipboard_data()
                                            .and_then(|d| d.get_data("text").ok())
                                            .unwrap_or_default();
                                        if editable && est_un_bloc(&colle) {
                                            ev.prevent_default();
                                            lancer_collage(i, j, colle);
                                        }
                                    }
                                />
                                {move || match etat.get() {
                                    EtatCase::Refusee(m) => view! { <span class="eva3-msg" role="alert">{m}</span> }.into_any(),
                                    EtatCase::Conflit(_) => view! { <span class="eva3-msg" role="alert">"Conflit : voir ci-dessous"</span> }.into_any(),
                                    EtatCase::Normal => view! { <span></span> }.into_any(),
                                }}
                            </td>
                        }
                    })
                    .collect_view();
                let moyenne = l.moyenne;
                let nb = l.nb_comptees;
                let mouvement = l.mouvement.clone();
                let eleve_id = l.eleve_id;
                view! {
                    <tr>
                        <th scope="row" class="eva3-nom">
                            <span>{l.nom.clone()}</span>
                            {mouvement.map(|m| view! { <span class="eva3-mouvement">{m}</span> })}
                        </th>
                        {cellules}
                        <td class="eva3-moy">
                            <button
                                type="button"
                                class="eva3-moy__btn"
                                aria-label=format!("Moyenne de {} : voir le calcul", l.nom)
                                on:click=move |_| selection.set(Some((i, usize::MAX)))
                            >
                                {move || moyenne_affichee(moyenne.get())}
                            </button>
                            <span class="eva3-moy__n">{move || format!("{} éval.", nb.get())}</span>
                            <span hidden=true>{eleve_id.to_string()}</span>
                        </td>
                    </tr>
                }
            })
            .collect_view()
    };

    let pied_classe = move || {
        match moyenne_classe.get() {
        Some(s) => format!(
            "Moyenne de la classe {} · min {} · max {} · médiane {} · {} élève(s) · calculée par le serveur",
            moyenne_affichee(Some(s.moyenne_m)),
            moyenne_affichee(Some(s.min_m)),
            moyenne_affichee(Some(s.max_m)),
            moyenne_affichee(Some(s.mediane_m)),
            s.effectif
        ),
        None => "Pas encore de moyenne de classe.".to_string(),
    }
    };
    let echelle = ent_types::evaluations_v3::formater_milliemes(echelle_m);

    let eleves_pour_dialogues: Vec<(Uuid, String)> =
        lignes.with_value(|ls| ls.iter().map(|l| (l.eleve_id, l.nom.clone())).collect());
    let eleves_pour_dialogues = StoredValue::new(eleves_pour_dialogues);
    let sous_matieres = StoredValue::new(sous_matieres);
    let periode_defaut = grille.periode.clone();
    let evals_pour_import = colonnes_titres;
    let grille_vide = nb_cols == 0;

    view! {
        <section class="eva3" aria-labelledby="eva3-titre">
            <h2 id="eva3-titre" class="eva3-titre">{titre_grille}</h2>
            <p class="eva3-aide" id="eva3-aide">
                "Flèches, Entrée et Tab pour passer d'une case à l'autre. Codes : "
                {CODES_RAPIDES.iter().map(|(c, _, l)| format!("{c} = {l}")).collect::<Vec<_>>().join(", ")}
                ". Collez un bloc depuis un tableur sur une case. Échap annule la frappe en cours."
                {if peut_saisir { "" } else { " Vous consultez cette grille en lecture seule." }}
            </p>
            {avert_vue}
            <Annonce message=annonce/>
            {move || erreur_globale.get().map(|m| view! {
                <Alerte niveau=Niveau::Erreur titre=m on_fermer=Callback::new(move |_| erreur_globale.set(None))/>
            })}
            <div class="eva3-barre">
                {if peut_saisir {
                    view! {
                        <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| dlg_nouvelle.set(true))>"Nouvelle évaluation"</Bouton>
                        <Bouton variante=Variante::Secondaire desactive=grille_vide on_click=Callback::new(move |_| dlg_colle.set(true))>"Coller un bloc"</Bouton>
                        <Bouton variante=Variante::Secondaire desactive=grille_vide on_click=Callback::new(move |_| { erreur_import.set(None); dlg_import.set(true) })>"Importer un fichier"</Bouton>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| enregistrer_cb.run(()))>"Enregistrer maintenant"</Bouton>
                <span class="eva3-statut" role="status">{move || statut_enreg.get()}</span>
            </div>
            {if peut_corriger {
                view! {
                    <ChampTexte
                        id="eva3-motif"
                        libelle="Motif écrit des corrections (période arrêtée)"
                        valeur=motif_correction
                        aide="Exigé pour toute case dont la période est arrêtée ; journalisé avec votre nom."
                    />
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
            {if delai.is_some() {
                view! { <p class="eva3-aide">{format!("Délai maximal de publication : {} jours après la date de l'évaluation (sauf « en cours de correction »).", delai.unwrap_or(0))}</p> }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
            {if grille_vide {
                view! { <EtatVide titre="Aucune évaluation sur cette période" message="Créez une évaluation pour commencer la saisie."/> }.into_any()
            } else {
                view! {
                    <div class="eva3-cadre" role="region" aria-label="Grille de notes" tabindex="-1">
                        <table class="eva3-table">
                            <caption class="ui-sr">"Notes des élèves, une colonne par évaluation"</caption>
                            <thead>
                                <tr>
                                    <th scope="col" class="eva3-nom">"Élève"</th>
                                    {entete()}
                                    <th scope="col" class="eva3-moy">{format!("Moyenne /{echelle}")}</th>
                                </tr>
                            </thead>
                            <tbody>{corps_tableau()}</tbody>
                        </table>
                    </div>
                }.into_any()
            }}
            <p class="eva3-pied" role="status">{pied_classe}</p>

            {move || {
                let c = conflits.get();
                if c.is_empty() {
                    view! { <span></span> }.into_any()
                } else {
                    view! {
                        <section class="eva3-conflits" aria-labelledby="eva3-conflits-t">
                            <h3 id="eva3-conflits-t">"Conflits à résoudre"</h3>
                            <p>"Ces cases ont été modifiées ailleurs (autre onglet, autre appareil, autre personne). Rien n'a été écrasé : choisissez."</p>
                            <ul>
                                {c.into_iter().map(|(i, j, x)| {
                                    let cs = lignes.with_value(|ls| ls[i].cases[j]);
                                    let (nom, titre) = (lignes.with_value(|ls| ls[i].nom.clone()), evals.with_value(|e| e[j].titre.clone()));
                                    let ma = cs.texte.get_untracked();
                                    let en_base = phrase_conflit(&x);
                                    let (x1, x2) = (x.clone(), x);
                                    view! {
                                        <li>
                                            <p>{format!("{nom} — {titre} : votre saisie « {ma} », en base {en_base}.")}</p>
                                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| {
                                                cs.base.set(texte_conflit(&x1));
                                                cs.version.set(Some(x1.version));
                                                cs.etat.set(EtatCase::Normal);
                                                programmer(100);
                                            })>"Garder ma saisie"</Bouton>
                                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| {
                                                let t = texte_conflit(&x2);
                                                cs.base.set(t.clone());
                                                cs.texte.set(t);
                                                cs.version.set(Some(x2.version));
                                                cs.etat.set(EtatCase::Normal);
                                            })>"Prendre la valeur en base"</Bouton>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        </section>
                    }.into_any()
                }
            }}

            <PanneauCase
                selection=selection
                lignes=lignes
                evals=evals
                cours_id=cours_id
                periode_id=periode_id
            />

            <DialogueNouvelle
                ouvert=dlg_nouvelle
                cours_id=cours_id
                periode=periode_defaut
                eleves=eleves_pour_dialogues
                sous_matieres=sous_matieres
                recharger=recharger
            />
            <DialogueColle ouvert=dlg_colle on_valider=Callback::new(move |t: String| {
                let (i, j) = selection.get_untracked().filter(|(_, j)| *j != usize::MAX).unwrap_or((0, 0));
                lancer_collage(i, j, t);
            })/>
            <DialogueImport
                ouvert=dlg_import
                erreur=erreur_import
                evaluations=evals_pour_import
                cours_id=cours_id
                on_apercu=Callback::new(move |(corps, a): (ImportBody, ImportApercuDto)| {
                    apercu.set(Some((corps, a)));
                    dlg_import.set(false);
                    dlg_apercu.set(true);
                })
            />
            <DialogueApercu ouvert=dlg_apercu apercu=apercu recharger=recharger annonce=annonce/>
            <DialogueEvaluation
                ouvert=dlg_eval
                eval_ouverte=eval_ouverte
                evals=evals
                recharger=recharger
                sous_matieres=sous_matieres
            />
        </section>
    }
}

// ───────────────────────── Panneau de la case sélectionnée ─────────────────────────

#[component]
fn PanneauCase(
    selection: RwSignal<Option<(usize, usize)>>,
    lignes: StoredValue<Vec<LigneSig>>,
    evals: StoredValue<Vec<EvaluationColonneDto>>,
    cours_id: Uuid,
    periode_id: Uuid,
) -> impl IntoView {
    let historique: RwSignal<Option<Result<Vec<LigneHistoriqueV3Dto>, String>>> =
        RwSignal::new(None);
    let explication: RwSignal<Option<Explication>> = RwSignal::new(None);
    Effect::new(move |_| {
        selection.track();
        historique.set(None);
        explication.set(None);
    });
    let voir_historique = move |_| {
        let Some((i, j)) = selection.get_untracked().filter(|(_, j)| *j != usize::MAX) else {
            return;
        };
        let eleve = lignes.with_value(|ls| ls[i].eleve_id);
        let ev = evals.with_value(|e| e[j].id);
        spawn_local(async move {
            let r = session::client()
                .eva3_historique(ev, eleve)
                .await
                .map_err(message_echec);
            historique.set(Some(r));
        });
    };
    let voir_calcul = move |_| {
        let Some((i, _)) = selection.get_untracked() else {
            return;
        };
        let eleve = lignes.with_value(|ls| ls[i].eleve_id);
        spawn_local(async move {
            let r = session::client()
                .eva3_explication(cours_id, periode_id, eleve)
                .await
                .map(|x| (x.lignes, x.avertissements))
                .map_err(message_echec);
            explication.set(Some(r));
        });
    };
    view! {
        <section class="eva3-panneau" aria-labelledby="eva3-panneau-t">
            <h3 id="eva3-panneau-t">"Case sélectionnée"</h3>
            {move || match selection.get() {
                None => view! { <p>"Choisissez une case ou une moyenne pour voir son historique ou son calcul."</p> }.into_any(),
                Some((i, j)) => {
                    let nom = lignes.with_value(|ls| ls[i].nom.clone());
                    let sur_case = j != usize::MAX;
                    let titre = if sur_case { evals.with_value(|e| e[j].titre.clone()) } else { "Moyenne du cours".to_string() };
                    view! {
                        <p><strong>{nom}</strong>" · "{titre}</p>
                        <div class="eva3-barre">
                            {if sur_case {
                                view! { <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| voir_historique(())) >"Voir l'historique de la note"</Bouton> }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| voir_calcul(()))>"Expliquer la moyenne"</Bouton>
                        </div>
                    }.into_any()
                }
            }}
            {move || historique.get().map(|r| match r {
                Err(m) => view! { <Alerte niveau=Niveau::Erreur titre=m/> }.into_any(),
                Ok(l) if l.is_empty() => view! { <p>"Aucun historique : la case n'a jamais été saisie."</p> }.into_any(),
                Ok(l) => view! {
                    <table class="eva3-histo">
                        <caption class="ui-sr">"Historique de la note"</caption>
                        <thead><tr><th scope="col">"Quand"</th><th scope="col">"Qui"</th><th scope="col">"Avant"</th><th scope="col">"Après"</th><th scope="col">"Motif"</th></tr></thead>
                        <tbody>
                            {l.into_iter().map(|h| {
                                let av = match (h.statut_avant.as_deref(), h.valeur_avant_m) {
                                    (Some("note"), Some(m)) => ent_types::evaluations_v3::formater_milliemes(m),
                                    (Some(s), _) => ent_types::evaluations_v3::libelle_statut(s).to_string(),
                                    _ => "—".to_string(),
                                };
                                let ap = match (h.statut_apres.as_str(), h.valeur_apres_m) {
                                    ("note", Some(m)) => ent_types::evaluations_v3::formater_milliemes(m),
                                    (s, _) => ent_types::evaluations_v3::libelle_statut(s).to_string(),
                                };
                                let marque = if h.correction { " (correction après l'arrêt)" } else { "" };
                                view! {
                                    <tr>
                                        <td>{h.le.format("%d/%m/%Y %H:%M UTC").to_string()}</td>
                                        <td>{h.auteur_nom.clone().unwrap_or_else(|| "—".into())}{format!(" · {}", h.source)}</td>
                                        <td>{av}</td>
                                        <td>{ap}{marque}</td>
                                        <td>{h.motif.clone().unwrap_or_default()}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                }.into_any(),
            })}
            {move || explication.get().map(|r| match r {
                Err(m) => view! { <Alerte niveau=Niveau::Erreur titre=m/> }.into_any(),
                Ok((l, a)) => view! {
                    <div>
                        <ul class="eva3-calcul">{l.into_iter().map(|x| view! { <li>{x}</li> }).collect_view()}</ul>
                        {a.into_iter().map(|x| view! { <Alerte niveau=Niveau::Avertissement titre=x/> }).collect_view()}
                    </div>
                }.into_any(),
            })}
        </section>
    }
}

// ───────────────────────── Dialogue : nouvelle évaluation ─────────────────────────

#[component]
fn DialogueNouvelle(
    ouvert: RwSignal<bool>,
    cours_id: Uuid,
    periode: PeriodeV3Dto,
    eleves: StoredValue<Vec<(Uuid, String)>>,
    sous_matieres: StoredValue<Vec<SousMatiereV3Dto>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let date = RwSignal::new(String::new());
    let bareme = RwSignal::new("20".to_string());
    let coef = RwSignal::new("1".to_string());
    let comptage = RwSignal::new("normal".to_string());
    let hors = RwSignal::new(false);
    let sous = RwSignal::new(String::new());
    let publication = RwSignal::new("explicite".to_string());
    let annoncee = RwSignal::new(false);
    let restreint = RwSignal::new(false);
    let visees: Vec<(Uuid, String, RwSignal<bool>)> = eleves
        .get_value()
        .into_iter()
        .map(|(id, n)| (id, n, RwSignal::new(false)))
        .collect();
    let visees = StoredValue::new(visees);
    let erreurs: RwSignal<Vec<(String, String)>> = RwSignal::new(Vec::new());
    // Date proposée : aujourd'hui borné à la période (posé après l'hydratation, jamais dans le HTML du serveur).
    Effect::new(move |_| {
        if ouvert.get() && date.get_untracked().is_empty() {
            let auj = aujourdhui().clamp(periode.date_debut, periode.date_fin);
            date.set(auj.format("%Y-%m-%d").to_string());
        }
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        ouvert.set(false);
        titre.set(String::new());
        recharger.run(());
    }));
    let valider = move |_| {
        let e = controler_evaluation(
            &titre.get_untracked(),
            &date.get_untracked(),
            &bareme.get_untracked(),
            &coef.get_untracked(),
        );
        erreurs.set(e.iter().map(|(c, m)| (c.to_string(), m.clone())).collect());
        if !e.is_empty() {
            return;
        }
        let Ok(d) = NaiveDate::parse_from_str(date.get_untracked().trim(), "%Y-%m-%d") else {
            erreurs.set(vec![("date".into(), "Date invalide.".into())]);
            return;
        };
        let restreinte_a: Vec<Uuid> = if restreint.get_untracked() {
            visees.with_value(|v| {
                v.iter()
                    .filter(|(_, _, c)| c.get_untracked())
                    .map(|(id, _, _)| *id)
                    .collect()
            })
        } else {
            Vec::new()
        };
        if restreint.get_untracked() && restreinte_a.is_empty() {
            erreurs.set(vec![(
                "public".into(),
                "Cochez au moins un élève, ou décochez « Public restreint ».".into(),
            )]);
            return;
        }
        let corps = CreerEvaluationV3Body {
            cours_id,
            titre: titre.get_untracked().trim().to_string(),
            date_eval: d,
            bareme_m: milliemes_formulaire(&bareme.get_untracked()).unwrap_or(20_000),
            coef_m: milliemes_formulaire(&coef.get_untracked()).unwrap_or(1_000),
            comptage: Some(comptage.get_untracked()),
            hors_bareme_autorise: hors.get_untracked(),
            sous_matiere_id: Uuid::parse_str(&sous.get_untracked()).ok(),
            restreinte_a,
            publication: Some(publication.get_untracked()),
            publication_prevue_le: None,
            annoncee: annoncee.get_untracked(),
        };
        ecriture.lancer(async move {
            let e = session::client().eva3_creer(&corps).await?;
            Ok(format!("Évaluation « {} » créée.", e.titre))
        });
    };
    let err = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
        })
    };
    let opts_sous: Vec<(String, String)> =
        std::iter::once((String::new(), "Aucune (la matière elle-même)".to_string()))
            .chain(
                sous_matieres
                    .get_value()
                    .into_iter()
                    .map(|s| (s.id.to_string(), s.libelle)),
            )
            .collect();
    view! {
        <Dialogue ouvert=ouvert titre="Nouvelle évaluation" id="eva3-dlg-nouvelle">
            <div class="eva3-form">
                <ChampTexte id="eva3-n-titre" libelle="Titre" valeur=titre requis=true erreur=err("titre")/>
                <ChampTexte id="eva3-n-date" libelle="Date" valeur=date genre="date" requis=true erreur=err("date")/>
                <ChampTexte id="eva3-n-bareme" libelle="Barème (points)" valeur=bareme inputmode="decimal" requis=true erreur=err("bareme")/>
                <ChampTexte id="eva3-n-coef" libelle="Coefficient" valeur=coef inputmode="decimal" requis=true erreur=err("coef")/>
                <Selection id="eva3-n-comptage" libelle="Comptage" valeur=comptage options=vec![
                    ("normal".into(), "Normal".into()),
                    ("si_ameliore".into(), "Compte si elle améliore la moyenne".into()),
                    ("ne_compte_pas".into(), "Ne compte pas dans la moyenne".into()),
                    ("bonus".into(), "Points bonus".into()),
                ]/>
                <Case id="eva3-n-hors" libelle="Points au-delà du barème autorisés" coche=hors/>
                <Selection id="eva3-n-sous" libelle="Sous-matière" valeur=sous options=opts_sous/>
                <Selection id="eva3-n-pub" libelle="Publication aux familles" valeur=publication options=vec![
                    ("explicite".into(), "Je publie moi-même".into()),
                    ("a_l_arret".into(), "À l'arrêt des notes".into()),
                ]/>
                <Case id="eva3-n-annoncee" libelle="Évaluation annoncée (visible comme « prévue »)" coche=annoncee/>
                <Case id="eva3-n-restreint" libelle="Public restreint (demi-groupe, volontaires)" coche=restreint/>
                {move || restreint.get().then(|| view! {
                    <fieldset class="eva3-visees">
                        <legend>"Élèves concernés"</legend>
                        {visees.get_value().into_iter().map(|(id, nom, c)| view! {
                            <Case id=format!("eva3-v-{id}") libelle=nom coche=c/>
                        }).collect_view()}
                    </fieldset>
                })}
                {move || err("public").get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                {move || ecriture.erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <div class="eva3-barre">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(valider)>"Créer l'évaluation"</Bouton>
                </div>
            </div>
        </Dialogue>
    }
}

// ───────────────────────── Dialogue : coller un bloc ─────────────────────────

#[component]
fn DialogueColle(ouvert: RwSignal<bool>, on_valider: Callback<String>) -> impl IntoView {
    let texte = RwSignal::new(String::new());
    view! {
        <Dialogue ouvert=ouvert titre="Coller un bloc depuis un tableur" id="eva3-dlg-colle">
            <p>"Collez les cellules copiées dans votre tableur (une ligne par élève, une colonne par évaluation). Le bloc est posé à partir de la case sélectionnée ; vous vérifiez un aperçu avant toute écriture."</p>
            <ZoneTexte id="eva3-colle-texte" libelle="Cellules copiées" valeur=texte/>
            <div class="eva3-barre">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| {
                    let t = texte.get_untracked();
                    if !t.trim().is_empty() {
                        on_valider.run(t);
                    }
                })>"Voir l'aperçu"</Bouton>
            </div>
        </Dialogue>
    }
}

// ───────────────────────── Dialogue : importer un fichier ─────────────────────────

#[component]
fn DialogueImport(
    ouvert: RwSignal<bool>,
    erreur: RwSignal<Option<String>>,
    evaluations: StoredValue<Vec<(Uuid, String)>>,
    cours_id: Uuid,
    on_apercu: Callback<(ImportBody, ImportApercuDto)>,
) -> impl IntoView {
    let cible = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let champ = NodeRef::<leptos::html::Input>::new();
    let opts: Vec<(String, String)> =
        std::iter::once((String::new(), "Selon les titres du fichier".to_string()))
            .chain(
                evaluations
                    .get_value()
                    .into_iter()
                    .map(|(id, t)| (id.to_string(), t)),
            )
            .collect();
    let lancer = move |_| {
        let Some(input) = champ.get() else { return };
        let cible_unique = Uuid::parse_str(&cible.get_untracked()).ok();
        let evals = evaluations.get_value();
        en_cours.set(true);
        erreur.set(None);
        let input: web_sys::HtmlInputElement = input;
        spawn_local(async move {
            let r: Result<Option<ImportApercuDto>, String> = async {
                let Some(texte) = lire_texte_fichier(&input).await? else {
                    return Err(
                        "Choisissez un fichier (CSV, TSV ou texte copié d'un tableur).".to_string(),
                    );
                };
                let table = lire_tableur(&texte);
                let plan = plan_fichier(&table, &evals, cible_unique)?;
                let corps = ImportBody {
                    cours_id,
                    mode: "fichier".into(),
                    cibles: plan.cibles.clone(),
                    lignes: plan.lignes.clone(),
                    empreinte: None,
                };
                let a = session::client()
                    .eva3_apercu_import(&corps)
                    .await
                    .map_err(message_echec)?;
                on_apercu.run((corps, a));
                if !plan.colonnes_ignorees.is_empty() {
                    erreur.set(Some(format!(
                        "Colonnes laissées de côté : {}.",
                        plan.colonnes_ignorees.join(", ")
                    )));
                }
                Ok(None)
            }
            .await;
            if let Err(m) = r {
                erreur.set(Some(m));
            }
            en_cours.set(false);
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Importer un fichier de notes" id="eva3-dlg-import">
            <p>"Fichier CSV, TSV ou texte copié d'un tableur : première colonne = nom de l'élève (« NOM Prénom » dans n'importe quel ordre), puis une colonne par évaluation, dont l'en-tête reprend le titre de l'évaluation. Rien n'est écrit avant votre confirmation."</p>
            <label class="ui-champ__etiquette" for="eva3-fichier">"Fichier"</label>
            <input id="eva3-fichier" type="file" accept=".csv,.tsv,.txt,text/csv,text/plain" node_ref=champ/>
            <Selection id="eva3-import-cible" libelle="Évaluation visée (fichier sans titres de colonnes)" valeur=cible options=opts/>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Avertissement titre=m/> })}
            <div class="eva3-barre">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                <Bouton variante=Variante::Primaire en_cours=en_cours on_click=Callback::new(lancer)>"Voir l'aperçu"</Bouton>
            </div>
        </Dialogue>
    }
}

// ───────────────────────── Dialogue : aperçu d'un import ─────────────────────────

#[component]
fn DialogueApercu(
    ouvert: RwSignal<bool>,
    apercu: RwSignal<Option<(ImportBody, ImportApercuDto)>>,
    recharger: Callback<()>,
    annonce: RwSignal<String>,
) -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        ouvert.set(false);
        apercu.set(None);
        recharger.run(());
    }));
    let confirmer = move |_| {
        let Some((mut corps, a)) = apercu.get_untracked() else {
            return;
        };
        corps.empreinte = Some(a.empreinte.clone());
        ecriture.lancer(async move {
            let r = session::client().eva3_confirmer_import(&corps).await?;
            Ok(format!("Import terminé : {}", bilan_lot(&r.cellules)))
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Aperçu avant écriture" id="eva3-dlg-apercu">
            {move || match apercu.get() {
                None => view! { <p>"Aucun aperçu."</p> }.into_any(),
                Some((_, a)) => {
                    let t = a.totaux.clone();
                    let a_ecrire = t.nouvelles + t.modifiees;
                    view! {
                        <div>
                            <p role="status">
                                {format!("{} ligne(s) lue(s), {} rapprochée(s), {} non rapprochée(s). {} case(s) nouvelle(s), {} modifiée(s), {} inchangée(s), {} en erreur. {} élève(s) de la grille absent(s) du fichier.",
                                    t.lignes, t.rapprochees, t.non_rapprochees, t.nouvelles, t.modifiees, t.inchangees, t.erreurs, a.eleves_absents_du_fichier)}
                            </p>
                            <div class="eva3-cadre">
                                <table class="eva3-apercu">
                                    <caption class="ui-sr">"Aperçu de l'import, ligne par ligne"</caption>
                                    <thead><tr><th scope="col">"Ligne"</th><th scope="col">"Élève"</th><th scope="col">"Cases"</th></tr></thead>
                                    <tbody>
                                        {a.lignes.into_iter().map(|l| {
                                            let eleve = match (&l.eleve, l.rapprochement.as_str()) {
                                                (Some(e), _) => format!("{} {}", e.nom, e.prenom),
                                                (None, "ambigu") => format!("« {} » : ambigu ou en double, laissé de côté", l.nom_saisi.clone().unwrap_or_default()),
                                                (None, "hors_public") => "hors du public du cours, laissé de côté".to_string(),
                                                (None, _) => format!("« {} » : non rapproché, laissé de côté", l.nom_saisi.clone().unwrap_or_else(|| format!("ligne {}", l.numero))),
                                            };
                                            let cases = l.cellules.iter().map(|c| {
                                                let avant = match (c.avant_statut.as_deref(), c.avant_valeur_m) {
                                                    (Some("note"), Some(m)) => format!(" (avant : {})", ent_types::evaluations_v3::formater_milliemes(m)),
                                                    (Some(s), _) => format!(" (avant : {})", ent_types::evaluations_v3::libelle_statut(s)),
                                                    _ => String::new(),
                                                };
                                                let lib = match c.etat.as_str() {
                                                    "nouveau" => "nouveau",
                                                    "modifie" => "modifié",
                                                    "inchange" => "inchangé",
                                                    "vide" => "vide",
                                                    "verrouille" => "verrouillé",
                                                    _ => "erreur",
                                                };
                                                let detail = c.erreur.clone().map(|e| format!(" — {e}")).unwrap_or_default();
                                                format!("{} → {lib}{avant}{detail}", if c.texte.is_empty() { "∅".to_string() } else { c.texte.clone() })
                                            }).collect::<Vec<_>>();
                                            view! {
                                                <tr>
                                                    <td>{l.numero}</td>
                                                    <th scope="row">{eleve}</th>
                                                    <td><ul>{cases.into_iter().map(|c| view! { <li>{c}</li> }).collect_view()}</ul></td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                            {move || ecriture.erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                            <div class="eva3-barre">
                                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| { ouvert.set(false); annonce.set("Import abandonné : rien n'a été écrit.".into()); })>"Abandonner"</Bouton>
                                <Bouton variante=Variante::Primaire desactive={a_ecrire == 0} en_cours=ecriture.en_cours on_click=Callback::new(confirmer)>
                                    {format!("Confirmer l'écriture de {a_ecrire} case(s)")}
                                </Bouton>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </Dialogue>
    }
}

// ───────────────────────── Dialogue : réglage et publication d'une évaluation ─────────────────────────

#[component]
fn DialogueEvaluation(
    ouvert: RwSignal<bool>,
    eval_ouverte: RwSignal<Option<usize>>,
    evals: StoredValue<Vec<EvaluationColonneDto>>,
    recharger: Callback<()>,
    sous_matieres: StoredValue<Vec<SousMatiereV3Dto>>,
) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let date = RwSignal::new(String::new());
    let bareme = RwSignal::new(String::new());
    let coef = RwSignal::new(String::new());
    let comptage = RwSignal::new("normal".to_string());
    let hors = RwSignal::new(false);
    let annoncee = RwSignal::new(false);
    let motif = RwSignal::new(String::new());
    let motif_retrait = RwSignal::new(String::new());
    let motif_annul = RwSignal::new(String::new());
    let programme_le = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let erreurs: RwSignal<Vec<(String, String)>> = RwSignal::new(Vec::new());
    let _ = sous_matieres;
    Effect::new(move |_| {
        if let (true, Some(j)) = (ouvert.get(), eval_ouverte.get()) {
            if let Some(e) = evals.with_value(|v| v.get(j).cloned()) {
                titre.set(e.titre.clone());
                date.set(e.date_eval.format("%Y-%m-%d").to_string());
                bareme.set(ent_types::evaluations_v3::formater_milliemes(e.bareme_m));
                coef.set(ent_types::evaluations_v3::formater_milliemes(e.coef_m));
                comptage.set(e.comptage.clone());
                hors.set(e.hors_bareme_autorise);
                annoncee.set(e.annoncee);
                en_cours.set(e.en_cours_de_correction);
                motif.set(String::new());
                motif_retrait.set(String::new());
                motif_annul.set(String::new());
                programme_le.set(String::new());
                erreurs.set(Vec::new());
            }
        }
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        ouvert.set(false);
        recharger.run(());
    }));
    let courante = move || {
        eval_ouverte
            .get()
            .and_then(|j| evals.with_value(|v| v.get(j).cloned()))
    };
    let enregistrer = move |_| {
        let Some(e) = courante() else { return };
        let er = controler_evaluation(
            &titre.get_untracked(),
            &date.get_untracked(),
            &bareme.get_untracked(),
            &coef.get_untracked(),
        );
        erreurs.set(er.iter().map(|(c, m)| (c.to_string(), m.clone())).collect());
        if !er.is_empty() {
            return;
        }
        let Ok(d) = NaiveDate::parse_from_str(date.get_untracked().trim(), "%Y-%m-%d") else {
            erreurs.set(vec![("date".into(), "Date invalide.".into())]);
            return;
        };
        let corps = ModifierEvaluationV3Body {
            version_attendue: e.version,
            titre: Some(titre.get_untracked()),
            date_eval: Some(d),
            bareme_m: milliemes_formulaire(&bareme.get_untracked()),
            coef_m: milliemes_formulaire(&coef.get_untracked()),
            comptage: Some(comptage.get_untracked()),
            hors_bareme_autorise: Some(hors.get_untracked()),
            annoncee: Some(annoncee.get_untracked()),
            motif: Some(motif.get_untracked()).filter(|m| !m.trim().is_empty()),
        };
        ecriture.lancer(async move {
            let r = session::client().eva3_modifier(e.id, &corps).await?;
            Ok(format!("« {} » mise à jour.", r.titre))
        });
    };
    let publier = move |action: &'static str| {
        Callback::new(move |_| {
            let Some(e) = courante() else { return };
            let mut corps = PublierV3Body {
                action: action.into(),
                prevue_le: None,
                prevue_locale: None,
                motif: None,
            };
            match action {
                "retirer" => corps.motif = Some(motif_retrait.get_untracked()),
                "programmer" => {
                    let t = programme_le.get_untracked();
                    match NaiveDateTime::parse_from_str(t.trim(), "%Y-%m-%dT%H:%M") {
                        Ok(l) => corps.prevue_locale = Some(l),
                        Err(_) => {
                            erreurs.set(vec![(
                                "programme".into(),
                                "Choisissez la date et l'heure de publication.".into(),
                            )]);
                            return;
                        }
                    }
                }
                _ => {}
            }
            ecriture.lancer(async move {
                let r = session::client().eva3_publication(e.id, &corps).await?;
                Ok(format!(
                    "« {} » : {}.",
                    r.titre,
                    libelle_etat_publication(&r).0.to_lowercase()
                ))
            });
        })
    };
    let basculer_correction = move |_| {
        let Some(e) = courante() else { return };
        let v = !en_cours.get_untracked();
        ecriture.lancer(async move {
            let r = session::client()
                .eva3_en_cours_de_correction(e.id, v)
                .await?;
            Ok(format!(
                "« {} » : {}.",
                r.titre,
                if v {
                    "en cours de correction"
                } else {
                    "correction terminée"
                }
            ))
        });
    };
    let annuler = move |_| {
        let Some(e) = courante() else { return };
        let corps = AnnulerEvaluationV3Body {
            version_attendue: e.version,
            motif: motif_annul.get_untracked(),
        };
        ecriture.lancer(async move {
            let r = session::client().eva3_annuler(e.id, &corps).await?;
            Ok(format!("« {} » annulée.", r.titre))
        });
    };
    let err = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
        })
    };
    view! {
        <Dialogue ouvert=ouvert titre="Évaluation : réglages et publication" id="eva3-dlg-eval">
            {move || courante().map(|e| {
                let (lib, ton) = libelle_etat_publication(&e);
                let modifiable = e.peut_modifier;
                let annulee = e.etat == "annulee";
                let publiee = e.publiee_le.is_some();
                let retrait = e.retrait_possible;
                let a_res = e.a_des_resultats;
                view! {
                    <div class="eva3-form">
                        <p><Badge libelle=lib ton=ton/>{e.restreinte.then_some(" · public restreint")}{e.commune.then_some(" · évaluation commune")}</p>
                        {e.motif_annulation.clone().map(|m| view! { <p>{format!("Motif d'annulation : {m}")}</p> })}
                        {if modifiable && !annulee {
                            view! {
                                <fieldset class="eva3-fieldset">
                                    <legend>"Réglages"</legend>
                                    <ChampTexte id="eva3-e-titre" libelle="Titre" valeur=titre requis=true erreur=err("titre")/>
                                    <ChampTexte id="eva3-e-date" libelle="Date" valeur=date genre="date" requis=true erreur=err("date")/>
                                    <ChampTexte id="eva3-e-bareme" libelle="Barème (points)" valeur=bareme inputmode="decimal" requis=true erreur=err("bareme")/>
                                    <ChampTexte id="eva3-e-coef" libelle="Coefficient" valeur=coef inputmode="decimal" requis=true erreur=err("coef")/>
                                    <Selection id="eva3-e-comptage" libelle="Comptage" valeur=comptage options=vec![
                                        ("normal".into(), "Normal".into()),
                                        ("si_ameliore".into(), "Compte si elle améliore la moyenne".into()),
                                        ("ne_compte_pas".into(), "Ne compte pas dans la moyenne".into()),
                                        ("bonus".into(), "Points bonus".into()),
                                    ]/>
                                    <Case id="eva3-e-hors" libelle="Points au-delà du barème autorisés" coche=hors/>
                                    <Case id="eva3-e-annoncee" libelle="Évaluation annoncée" coche=annoncee/>
                                    {a_res.then(|| view! {
                                        <ChampTexte id="eva3-e-motif" libelle="Motif du changement (obligatoire : des résultats existent)" valeur=motif/>
                                    })}
                                    <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=Callback::new(enregistrer)>"Enregistrer les réglages"</Bouton>
                                </fieldset>
                            }.into_any()
                        } else {
                            view! { <p>"Les réglages de cette évaluation ne sont pas modifiables par vous (auteur différent, évaluation annulée ou lecture seule)."</p> }.into_any()
                        }}
                        {if modifiable && !annulee {
                            view! {
                                <fieldset class="eva3-fieldset">
                                    <legend>"Publication aux familles"</legend>
                                    {if !publiee {
                                        view! {
                                            <div class="eva3-barre">
                                                <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=publier("maintenant")>"Publier maintenant"</Bouton>
                                                <Bouton variante=Variante::Secondaire on_click=publier("a_l_arret")>"Publier à l'arrêt des notes"</Bouton>
                                                <Bouton variante=Variante::Discret on_click=publier("annuler_programmation")>"Annuler la programmation"</Bouton>
                                            </div>
                                            <ChampTexte id="eva3-e-prog" libelle="Programmer la publication (heure de l'établissement)" valeur=programme_le genre="datetime-local" erreur=err("programme")/>
                                            <Bouton variante=Variante::Secondaire on_click=publier("programmer")>"Programmer"</Bouton>
                                            <Case id="eva3-e-encours" libelle="En cours de correction (suspend la publication automatique au délai maximal)" coche=en_cours/>
                                            <Bouton variante=Variante::Discret on_click=Callback::new(basculer_correction)>"Appliquer « en cours de correction »"</Bouton>
                                        }.into_any()
                                    } else {
                                        view! { <p>"Publiée : les familles voient les résultats."</p> }.into_any()
                                    }}
                                </fieldset>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if publiee && retrait {
                            view! {
                                <fieldset class="eva3-fieldset">
                                    <legend>"Retirer la publication"</legend>
                                    <p>"Les familles verront « résultats retirés pour vérification ». Retrait possible 24 h après la publication ; ensuite, la direction."</p>
                                    <ChampTexte id="eva3-e-retrait" libelle="Motif du retrait" valeur=motif_retrait requis=true/>
                                    <Bouton variante=Variante::Danger on_click=publier("retirer")>"Retirer la publication"</Bouton>
                                </fieldset>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if !annulee && (modifiable || retrait) {
                            view! {
                                <fieldset class="eva3-fieldset">
                                    <legend>"Annuler l'évaluation"</legend>
                                    <p>"Les résultats sont conservés mais ne comptent plus (sujet diffusé, incident…)."</p>
                                    <ChampTexte id="eva3-e-annul" libelle="Motif de l'annulation" valeur=motif_annul requis=true/>
                                    <Bouton variante=Variante::Danger on_click=Callback::new(annuler)>"Annuler l'évaluation"</Bouton>
                                </fieldset>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {move || ecriture.erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                        <div class="eva3-barre">
                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                        </div>
                    </div>
                }
            })}
        </Dialogue>
    }
}
