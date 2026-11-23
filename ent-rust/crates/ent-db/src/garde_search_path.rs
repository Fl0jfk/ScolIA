//! Chantier transverse T1 : **aucune fonction de la base n'est trompée par une table temporaire**
//! (docs/securite/T1_SEARCH_PATH.md, migration 690).
//!
//! PostgreSQL cherche les relations dans `pg_temp` en premier, et le privilège TEMP appartient à PUBLIC par défaut :
//! `CREATE TEMP TABLE ent_xxx (LIKE public.ent_xxx)` masque, dans la session de l'appelant, la vraie table pour toute
//! fonction qui la lit sans la qualifier — donc désarme une garde « en base » en silence. La règle, vérifiée ici sur le
//! catalogue d'une base migrée jusqu'au bout (donc à chaque lot futur) :
//!
//! 1. toute fonction de `public` (hors extension) porte `search_path = pg_catalog, …, pg_temp` (`pg_temp` en dernier) ;
//! 2. sauf deux familles **nommées et justifiées ci-dessous** (fichier unique), où `SET` ferait perdre l'inlining :
//!    - [`PURES`] : le corps ne lit aucune relation (politiques RLS, index, contraintes CHECK, requêtes chaudes) ;
//!    - [`QUALIFIEES`] : fonctions `LANGUAGE sql` qui retournent un ensemble de lignes, inlinées dans les requêtes qui
//!      les appellent ; toutes leurs relations sont écrites `public.xxx` (la migration 690 le fait à partir de la
//!      définition en vigueur).
//!
//!    Les deux listes sont recopiées dans `migrations/690_t1_search_path_fonctions.sql` : un test les compare.
//!
//! **Si un test de ce fichier échoue pour votre lot** : la fonction que vous créez ou redéfinissez n'a pas
//! `SET search_path = pg_catalog, public, pg_temp` (un `CREATE OR REPLACE FUNCTION` efface la clause : la remettre à
//! chaque redéfinition, y compris dans une migration de reprise). Ne l'ajoutez à une liste d'exclusions que si le corps
//! ne lit aucune relation (`PURES`) ou si l'inlining est mesuré indispensable (`QUALIFIEES`), avec la raison, dans ce
//! fichier **et** dans la migration 690.

use super::*;
use uuid::Uuid;

/// Fonctions dont le corps ne cite **aucune** relation de `public` : expressions pures (réglages de session,
/// calendrier, normalisation de texte, contrôles de format), ou appels d'autres fonctions qui portent leur propre
/// `search_path`. Rien à détourner ; `SET` n'apporterait que la perte de l'inlining (politiques RLS sur 844 tables,
/// contraintes CHECK, index d'expressions, requêtes chaudes). Le test refuse toute entrée dont le corps citerait une
/// relation, et toute entrée qui aurait disparu du catalogue. Ordre alphabétique (identique à 690).
pub(crate) const PURES: &[(&str, &str)] = &[
    ("ent_aujourdhui", "date d'un instant à Paris : expression pure, appelée dans les requêtes chaudes"),
    ("ent_cle_recherche", "clé de recherche normalisée : expression pure, utilisée dans les index d'expressions et les recherches"),
    ("ent_coord_groupe", "regroupement de types de coordonnées : expression pure, utilisée par un index"),
    ("ent_dem_fait_canonique", "forme canonique d'un fait (empreinte) : expression sur la ligne reçue, aucune relation"),
    ("ent_dem_purge_en_cours", "drapeau de transaction `ent.dem_purge` : lit un réglage de session"),
    ("ent_dem_rapprochement_en_cours", "drapeau de transaction `ent.dem_rapprochement` : lit un réglage de session"),
    ("ent_dem_requalif_en_cours", "drapeau de transaction `ent.dem_requalif` : lit un réglage de session"),
    ("ent_eleve_majorite_le", "date de majorité : expression pure sur trois arguments"),
    ("ent_eleve_sexe_normalise", "normalisation d'un texte : expression pure"),
    ("ent_eleves_lecture_seule", "n'appelle que `ent_public_cours_jour`, dont le corps est qualifié ; aucune relation propre"),
    ("ent_enseignement_suivi_le", "prédicat sur la ligne d'enseignement reçue : aucune relation"),
    ("ent_frm_purge", "drapeau de transaction `ent.purge_etablissement` : lit un réglage de session"),
    ("ent_fusion_en_cours", "drapeau de transaction `ent.fusion_eleve` : lit un réglage de session"),
    ("ent_import_ligne_donnees", "projection de la ligne d'import reçue : aucune relation"),
    ("ent_inscription_etat_effectif", "table de correspondance d'états : expression pure"),
    ("ent_inscription_etat_valide", "états valides par nature : expression pure, utilisée par des contraintes CHECK"),
    ("ent_inscription_statut_depuis_etat", "table de correspondance d'états : expression pure"),
    ("ent_internat_ancre_nuit", "instant d'ancrage d'une nuit : n'appelle que `ent_internat_fuseau`, qui porte son `search_path`"),
    ("ent_internat_lien_parental", "reconnaissance d'un lien parental dans un texte : expression pure"),
    ("ent_internat_personne_vise", "n'appelle que d'autres fonctions qui portent leur `search_path` ; aucune relation propre"),
    ("ent_jour_de_annee_label", "premier jour d'une année scolaire d'après son libellé : expression pure"),
    ("ent_niveau_de_code", "niveau d'après un code de classe : expression pure"),
    ("ent_parametre_valeur_json", "projection de la ligne de paramètre reçue : aucune relation"),
    ("ent_rls_etablissement", "politiques RLS de 844 tables : lit `app.etablissement_id`, aucune relation ; inlinée dans chaque plan"),
    ("ent_rls_strict", "politiques RLS de 844 tables : lit `app.rls_strict`, aucune relation ; inlinée dans chaque plan"),
    ("ent_siret_cle_valide", "clé de contrôle d'un SIRET : expression pure, utilisée par une contrainte CHECK"),
    ("ent_unaccent", "enveloppe de `public.unaccent` (dictionnaire qualifié) : aucune relation ; utilisée par les index d'expressions"),
    ("eva_dec3", "arrondi à trois décimales : expression pure"),
    ("eva_etat_rang", "rang d'un état de période : expression pure"),
];

/// Fonctions `LANGUAGE sql` qui retournent un ensemble de lignes et que le planificateur **inline** dans les requêtes
/// qui les appellent (messagerie, cours du jour, attendu de l'internat, évaluations…) : `SET` bloquerait l'inlining
/// (mesures : docs/securite/T1_SEARCH_PATH.md §3). La migration 690 recrée leur corps avec **toutes** les relations
/// qualifiées `public.` ; le test vérifie qu'aucune ne reste non qualifiée, que la fonction reste inlinée, et qu'une
/// table temporaire homonyme de chacune de ses relations n'apparaît jamais dans son plan. Ordre alphabétique.
pub(crate) const QUALIFIEES: &[(&str, &str)] = &[
    ("ent_cdt_reprise_compteurs", "compteurs de la reprise du cahier : lecture inlinée"),
    ("ent_coord_projetees", "coordonnées projetées d'une personne : lecture inlinée"),
    ("ent_cours_vacants", "cours sans enseignant : lecture inlinée"),
    ("ent_eleve_enseignement_comparaison", "comparaison d'enseignements : lecture inlinée"),
    ("ent_eleve_indicateurs_attendus", "indicateurs attendus d'un élève : lecture inlinée"),
    ("ent_eleve_majorites_a_traiter", "majorités à traiter : lecture inlinée"),
    ("ent_eleve_option_comparaison", "comparaison d'options : lecture inlinée"),
    ("ent_eleve_pai_courant", "PAI courant : lecture inlinée"),
    ("ent_enseignement_incoherences", "incohérences d'enseignement : lecture inlinée"),
    ("ent_groupe_membres_jour", "membres d'un groupe un jour donné : lecture inlinée"),
    ("ent_groupes_eleve_jour", "groupes d'un élève un jour donné : lecture inlinée"),
    ("ent_internat_attendu_calcule", "élèves attendus à l'internat : appelée par les gardes et les listes de la nuit, lecture inlinée"),
    ("ent_internat_personne_chaine", "chaîne de personnes d'un élève de l'internat : lecture inlinée"),
    ("ent_internat_remise", "personnes habilitées à recevoir l'élève : lecture inlinée"),
    ("ent_msg_boite_membres", "membres d'une boîte partagée : lecture inlinée dans chaque requête de messagerie"),
    ("ent_msg_fil_lisible", "fils lisibles par un compte : lecture inlinée dans chaque requête de messagerie (inbox)"),
    ("ent_msg_mes_participations", "participations d'un compte : lecture inlinée dans chaque requête de messagerie"),
    ("ent_public_cours_jour", "cours du jour visibles : appelée par 26 fichiers, lecture inlinée"),
    ("ent_service_intervenants_a", "intervenants d'un service à une date : lecture inlinée"),
    ("ent_services_vacants", "services sans enseignant : lecture inlinée"),
    ("ent_stage_siret_doublons", "doublons de SIRET des entreprises de stage : lecture inlinée"),
    ("eva_fiches_de_l_eleve", "fiches absorbées d'un élève : lecture inlinée"),
    ("eva_grille_eleves", "élèves d'une grille d'évaluation : lecture inlinée"),
    ("eva_periode_eleve", "période d'un élève : lecture inlinée"),
    ("eva_periode_eleve_jour", "période d'un élève un jour donné : lecture inlinée"),
];

const SQL_RELATIONS: &str = "SELECT string_agg(c.relname, '|' ORDER BY length(c.relname) DESC)
       FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
      WHERE n.nspname = 'public' AND c.relkind IN ('r', 'v', 'm', 'p', 'f', 'S')";

/// Fonctions de `public` créées par nos migrations (hors fonctions d'extension, hors agrégats).
const SQL_FONCTIONS: &str = "SELECT p.oid::regprocedure::text, p.proname::text, l.lanname::text, p.prosecdef, p.proretset,
        coalesce((SELECT c FROM unnest(p.proconfig) c WHERE c LIKE 'search_path=%' LIMIT 1), '') AS sp
   FROM pg_proc p
   JOIN pg_namespace n ON n.oid = p.pronamespace
   JOIN pg_language l ON l.oid = p.prolang
  WHERE n.nspname = 'public' AND p.prokind IN ('f', 'p')
    AND NOT EXISTS (SELECT 1 FROM pg_depend d
                     WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e')
  ORDER BY 1";

/// Relations de `public` citées par le corps (`sans_point` : seulement celles qui ne sont pas déjà qualifiées).
async fn relations_citees(pool: &sqlx::PgPool, signature: &str, sans_point: bool) -> Vec<String> {
    let (re,): (String,) = sqlx::query_as(&format!("SELECT ({SQL_RELATIONS})"))
        .fetch_one(pool)
        .await
        .unwrap();
    let avant = if sans_point {
        "[^A-Za-z0-9_.\"']"
    } else {
        "[^A-Za-z0-9_]"
    };
    let motif = format!("(^|{avant})({re})(?![A-Za-z0-9_])");
    let sql = "SELECT DISTINCT lower(m[2]) FROM pg_proc p, regexp_matches(p.prosrc, $2, 'gi') m WHERE p.oid = $1::regprocedure ORDER BY 1";
    sqlx::query_scalar::<_, String>(sql)
        .bind(signature)
        .bind(motif)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// Noms listés dans un tableau SQL de `690` entre deux marqueurs.
fn liste_de_690(marqueur: &str) -> Vec<String> {
    let sql = include_str!("../../../migrations/690_t1_search_path_fonctions.sql");
    let debut = sql
        .find(&format!("-- {marqueur}-DEBUT"))
        .unwrap_or_else(|| panic!("marqueur {marqueur}-DEBUT absent de 690"));
    let fin = sql
        .find(&format!("-- {marqueur}-FIN"))
        .unwrap_or_else(|| panic!("marqueur {marqueur}-FIN absent de 690"));
    sql[debut..fin]
        .lines()
        .skip(1)
        .filter_map(|l| {
            l.trim()
                .trim_end_matches(',')
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
                .map(str::to_string)
        })
        .collect()
}

/// Toutes les violations de la règle sur le catalogue de la base : vide si la règle est respectée. Prend le verrou
/// partagé des rejeux : aucun rejeu ne tourne pendant la lecture (une fonction y est brièvement sans clause).
pub(crate) async fn violations(pool: &sqlx::PgPool) -> Vec<String> {
    let garde = crate::migrate::garde_rejeu_partagee(pool).await;
    let r = violations_sans_verrou(pool).await;
    garde.liberer().await;
    r
}

async fn violations_sans_verrou(pool: &sqlx::PgPool) -> Vec<String> {
    let fonctions: Vec<(String, String, String, bool, bool, String)> =
        sqlx::query_as(SQL_FONCTIONS).fetch_all(pool).await.unwrap();
    let pures: Vec<&str> = PURES.iter().map(|(n, _)| *n).collect();
    let qualifiees: Vec<&str> = QUALIFIEES.iter().map(|(n, _)| *n).collect();
    let mut v = Vec::new();
    for (sig, nom, langue, secdef, retset, sp) in &fonctions {
        let nom = nom.as_str();
        let conforme = (sp.starts_with("search_path=pg_catalog, ") && sp.ends_with("pg_temp"))
            || sp == "search_path=pg_catalog, pg_temp";
        if pures.contains(&nom) {
            if *secdef {
                v.push(format!("{sig} : SECURITY DEFINER ne peut pas être une fonction « pure » sans `search_path`"));
            }
            let cites = relations_citees(pool, sig, false).await;
            if !cites.is_empty() {
                v.push(format!("{sig} : listée dans PURES mais son corps cite des relations {cites:?} : lui fixer `search_path` (et la retirer de PURES et de 690)"));
            }
        } else if qualifiees.contains(&nom) {
            if *secdef || langue != "sql" || !*retset {
                v.push(format!("{sig} : QUALIFIEES est réservée aux fonctions `LANGUAGE sql` retournant un ensemble, hors SECURITY DEFINER"));
            }
            if !sp.is_empty() {
                v.push(format!("{sig} : listée dans QUALIFIEES mais porte `{sp}` (l'inlining est perdu) : la retirer de la liste ou retirer la clause"));
            }
            let nues = relations_citees(pool, sig, true).await;
            if !nues.is_empty() {
                v.push(format!("{sig} : relations non qualifiées {nues:?} : écrire `public.xxx` partout (ou lui fixer `search_path` et la retirer de QUALIFIEES et de 690)"));
            }
        } else if !conforme {
            v.push(format!(
                "{sig} [{langue}{}] : `search_path` {} — ajouter `SET search_path = pg_catalog, public, pg_temp` au CREATE OR REPLACE FUNCTION de \
                 la migration qui la crée ou la redéfinit (un CREATE OR REPLACE efface la clause)",
                if *secdef { ", SECURITY DEFINER" } else { "" },
                if sp.is_empty() { "absent".to_string() } else { format!("`{sp}` (il doit commencer par pg_catalog et finir par pg_temp)") }
            ));
        }
    }
    // La liste ne ment jamais : aucune entrée disparue du catalogue.
    for (nom, _) in PURES.iter().chain(QUALIFIEES.iter()) {
        if !fonctions.iter().any(|(_, n, ..)| n == nom) {
            v.push(format!("{nom} : listée dans PURES ou QUALIFIEES mais absente du catalogue (entrée obsolète, à retirer aussi de 690)"));
        }
    }
    v
}

fn noms(l: &[(&str, &str)]) -> Vec<String> {
    l.iter().map(|(n, _)| n.to_string()).collect()
}

/// Ce que 690 écrit et ce que le test vérifie est une seule et même liste, triée, sans doublon, justifiée.
#[test]
fn les_listes_sont_celles_de_la_migration_690_et_justifiees() {
    for (liste, marqueur, nom) in [
        (PURES, "PURES", "PURES"),
        (QUALIFIEES, "QUALIFIEES", "QUALIFIEES"),
    ] {
        assert_eq!(
            noms(liste),
            liste_de_690(marqueur),
            "{nom} diffère de la liste de la migration 690 : les deux se modifient ensemble"
        );
        let mut tri = noms(liste);
        tri.sort();
        tri.dedup();
        assert_eq!(noms(liste), tri, "{nom} : ordre alphabétique, sans doublon");
        for (n, raison) in liste {
            assert!(raison.trim().len() >= 20, "{nom} : {n} sans justification");
        }
    }
    let communs: Vec<_> = noms(PURES)
        .into_iter()
        .filter(|n| noms(QUALIFIEES).contains(n))
        .collect();
    assert!(
        communs.is_empty(),
        "une fonction ne peut être à la fois pure et qualifiée : {communs:?}"
    );
}

/// Les relations d'une fonction qualifiée sont masquées par des tables temporaires homonymes : le plan n'en lit aucune
/// et la fonction reste inlinée.
async fn plan_sous_masque(pool: &sqlx::PgPool, signature: &str, nom: &str) -> String {
    let (re,): (String,) = sqlx::query_as(&format!("SELECT ({SQL_RELATIONS})"))
        .fetch_one(pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let relations: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT lower(m[2]) FROM pg_proc p, regexp_matches(p.prosrc, '(^|[^A-Za-z0-9_])(' || $2 || ')(?![A-Za-z0-9_])', 'gi') m
          WHERE p.oid = $1::regprocedure",
    )
    .bind(signature)
    .bind(&re)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert!(
        !relations.is_empty(),
        "{signature} : aucune relation citée, la fonction n'a rien à faire dans QUALIFIEES"
    );
    for r in &relations {
        sqlx::query(&format!(
            "CREATE TEMP TABLE {r} (LIKE public.{r}) ON COMMIT DROP"
        ))
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    let args: Vec<String> = sqlx::query_scalar(
        "SELECT CASE format_type(t, NULL) WHEN 'uuid' THEN '''00000000-0000-0000-0000-0000000000a1''::uuid'
                                          WHEN 'date' THEN '''2026-01-15''::date'
                                          WHEN 'boolean' THEN 'false' WHEN 'text' THEN '''x''::text'
                                          ELSE 'NULL' END
           FROM unnest((SELECT proargtypes::oid[] FROM pg_proc WHERE oid = $1::regprocedure)) WITH ORDINALITY AS u(t, o) ORDER BY o",
    )
    .bind(signature)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let plan: Vec<String> = sqlx::query_scalar(&format!(
        "EXPLAIN (VERBOSE, COSTS OFF) SELECT * FROM {nom}({})",
        args.join(", ")
    ))
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    plan.join("\n")
}

/// Vérifie les 25 fonctions qualifiées sous masque total, avec un témoin non qualifié qui doit, lui, être détourné.
async fn verifier_qualifiees(pool: &sqlx::PgPool) {
    // Témoin : une fonction du même genre, NON qualifiée, créée le temps du test — son plan lit la table temporaire.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("CREATE FUNCTION public.t1_temoin(p uuid) RETURNS TABLE (id uuid) LANGUAGE sql STABLE AS $f$ SELECT f.id FROM ent_msg_fil f WHERE f.etablissement_id = p $f$")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("CREATE TEMP TABLE ent_msg_fil (LIKE public.ent_msg_fil) ON COMMIT DROP")
        .execute(&mut *tx)
        .await
        .unwrap();
    let plan: Vec<String> = sqlx::query_scalar("EXPLAIN (VERBOSE, COSTS OFF) SELECT * FROM t1_temoin('00000000-0000-0000-0000-0000000000a1'::uuid)")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(
        plan.join("\n").contains("pg_temp"),
        "le témoin doit être détourné, sinon la méthode ne prouve rien : {plan:?}"
    );

    for (nom, _) in QUALIFIEES {
        let sigs: Vec<String> = sqlx::query_scalar(
            "SELECT p.oid::regprocedure::text FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'public' AND p.proname = $1",
        )
        .bind(nom)
        .fetch_all(pool)
        .await
        .unwrap();
        assert!(!sigs.is_empty(), "{nom} : absente du catalogue");
        for sig in sigs {
            let plan = plan_sous_masque(pool, &sig, nom).await;
            assert!(
                !plan.contains("pg_temp"),
                "{sig} lit une table temporaire (relation non qualifiée) :\n{plan}"
            );
            let non_inlinee = plan
                .lines()
                .any(|l| l.contains("Function Scan on") && l.contains(nom));
            assert!(
                !non_inlinee,
                "{sig} n'est plus inlinée (une clause SET a été posée ?) :\n{plan}"
            );
        }
    }
}

/// Message d'échec de la garde : dit quoi faire.
fn exiger_zero(contexte: &str, v: &[String]) {
    assert!(
        v.is_empty(),
        "T1 (docs/securite/T1_SEARCH_PATH.md), {contexte} : {} fonction(s) de `public` ne respectent pas la règle « une table temporaire ne \
         trompe aucune fonction ». Quoi faire : dans la migration de votre lot, écrire `SET search_path = pg_catalog, public, pg_temp` dans \
         chaque CREATE OR REPLACE FUNCTION (la clause est effacée par tout CREATE OR REPLACE ultérieur, y compris dans une migration de \
         reprise), ou, si le corps ne lit AUCUNE relation, ajouter la fonction à PURES (garde_search_path.rs) et à la liste de 690, avec la \
         raison.\n{}",
        v.len(),
        v.join("\n")
    );
}

/// Base neuve, créée pour ce test et supprimée à la fin : l'état d'une **installation** (tout le tableau `MIGRATIONS`,
/// dans l'ordre, une fois), indépendant des rejeux partiels que d'autres tests laissent dans la base commune.
struct BaseNeuve {
    admin: sqlx::PgPool,
    nom: String,
    url: String,
}

impl BaseNeuve {
    async fn creer() -> Option<Self> {
        let url = crate::url_guard::url_labo_ou_echec()?;
        assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
        let admin = sqlx::PgPool::connect(&url)
            .await
            .expect("connexion à la base commune");
        let nom = format!("ent_t1_{}_test", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {nom}"))
            .execute(&admin)
            .await
            .expect("CREATE DATABASE (rôle superutilisateur local requis)");
        let mut u = url::Url::parse(&url).unwrap();
        u.set_path(&nom);
        Some(Self {
            admin,
            nom,
            url: u.to_string(),
        })
    }

    async fn supprimer(self) {
        let _ = sqlx::query(&format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.nom
        ))
        .execute(&self.admin)
        .await;
    }
}

/// **La garde.** Sur une base NEUVE migrée jusqu'au bout (l'installation) : zéro fonction hors règle, les exclusions
/// existent, les fonctions qualifiées résistent à un masque total. Puis **rejeu** de toutes les migrations : chaque
/// `CREATE OR REPLACE FUNCTION` efface la clause, 690 rejouée en dernier remet tout en place, et la garde repasse.
#[tokio::test]
async fn la_garde_du_catalogue_sur_une_base_neuve_puis_apres_rejeu() {
    let Some(base) = BaseNeuve::creer().await else {
        return;
    };
    let resultat = tokio::spawn({
        let url = base.url.clone();
        async move {
            let ent = PgEnt::connect(&url).await.expect("connect");
            ent.migrate().await.expect("migrate");

            // Installation.
            exiger_zero("après installation", &violations(ent.pool()).await);
            let fonctions: Vec<(String, String, String, bool, bool, String)> =
                sqlx::query_as(SQL_FONCTIONS)
                    .fetch_all(ent.pool())
                    .await
                    .unwrap();
            assert!(
                fonctions.len() > 400,
                "inventaire anormalement court : {}",
                fonctions.len()
            );
            let exclues: Vec<&str> = PURES
                .iter()
                .chain(QUALIFIEES.iter())
                .map(|(n, _)| *n)
                .collect();
            let sans_clause: Vec<&str> = fonctions
                .iter()
                .filter(|f| f.5.is_empty())
                .map(|f| f.1.as_str())
                .collect();
            assert!(
                sans_clause.iter().all(|n| exclues.contains(n)),
                "des fonctions sans clause ne sont ni pures ni qualifiées : {:?}",
                sans_clause
                    .iter()
                    .filter(|n| !exclues.contains(n))
                    .collect::<Vec<_>>()
            );
            verifier_qualifiees(ent.pool()).await;

            // Rejeu de toutes les migrations, 690 en dernier.
            crate::migrate::rejouer_tout(ent.pool())
                .await
                .expect("rejeu");
            exiger_zero(
                "après rejeu de toutes les migrations (690 doit être rejouée en dernier)",
                &violations(ent.pool()).await,
            );
            verifier_qualifiees(ent.pool()).await;

            // La garde a des dents : une fonction nouvelle sans clause, ou un `CREATE OR REPLACE` qui efface la clause d'une
            // fonction existante, sont vus ; le message dit laquelle.
            sqlx::query("CREATE FUNCTION public.t1_nouvelle_fonction() RETURNS int LANGUAGE sql AS 'SELECT 1'").execute(ent.pool()).await.unwrap();
            sqlx::query("CREATE OR REPLACE FUNCTION public.ent_msg_retire_xid() RETURNS trigger LANGUAGE plpgsql AS $f$ BEGIN RETURN NEW; END $f$")
                .execute(ent.pool())
                .await
                .unwrap();
            let v = violations(ent.pool()).await;
            assert_eq!(v.len(), 2, "{v:?}");
            assert!(v.iter().any(|m| m.contains("t1_nouvelle_fonction") && m.contains("SET search_path = pg_catalog, public, pg_temp")), "{v:?}");
            assert!(v.iter().any(|m| m.contains("ent_msg_retire_xid") && m.contains("absent")), "{v:?}");
            // Et un `search_path` mal formé (sans `pg_temp` en dernier) l'est aussi.
            sqlx::query("DROP FUNCTION public.t1_nouvelle_fonction()").execute(ent.pool()).await.unwrap();
            sqlx::query("CREATE OR REPLACE FUNCTION public.ent_msg_retire_xid() RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, public AS $f$ BEGIN RETURN NEW; END $f$")
                .execute(ent.pool())
                .await
                .unwrap();
            let v = violations(ent.pool()).await;
            assert!(v.len() == 1 && v[0].contains("pg_temp"), "{v:?}");
            ent.pool().close().await;
        }
    })
    .await;
    base.supprimer().await;
    resultat.expect("la garde du catalogue a échoué (voir le message ci-dessus)");
}
