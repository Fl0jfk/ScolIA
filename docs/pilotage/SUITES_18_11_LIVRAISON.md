# Suites du 18 novembre : livraison

Branche `ent-suites-18-11` (partie de `ENT`, tête e5ddda1e, run 404 vert ; `origin/ENT` n'a pas bougé : fusion sans objet). Suites laissées ouvertes par V2a (`42_SORTIES_VOYAGES_V2A_LIVRAISON.md` §5 écart 7, §6 F7), F1-F2 (`24_FINANCES_F1_F2_LIVRAISON.md`, suites notées) et E9 (`02_ELEVE_E9_LIVRAISON.md`, F2 « noté, pas fait »).

**Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.** La CI GitHub fait foi. Aucune dépendance ajoutée, aucune migration, Next.js, base et secrets de production : non touchés.

> Note : `docs/PILOTAGE.md` ne contient que les règles 1 à 9 dans ce dépôt ; les règles 17, 25 et 27 à 35 citées dans la consigne n'y figurent pas. La consigne de lancement a été appliquée telle quelle ; la règle 28 est lue comme les rapports précédents la lisent (recalcul des compteurs `TOUTES`, `SOURCES`, RLS, `MIGRATION_TAG`).

## 1. Planificateur : alertes d'acomptes et purge des exports

**Un seul planificateur** : le planificateur d'`ent-api` (`main.rs`, à côté de `taches_eleves`). Une seule tâche de fond ajoutée, **toutes les heures** (le jour dépend du fuseau de chaque établissement), idempotente, **désactivable par `ENT_TACHES_VOYAGES_OBJETS=0`** (aussi `false`, `non`, `off`). Elle appelle `PgEnt::passages_quotidiens(jour)` (`ent-db/src/passages_quotidiens.rs`), qui parcourt les établissements ; une étape en échec est comptée et journalisée sans bloquer les suivantes ni les autres établissements.

| Étape | Fonction existante, rendue appelable avec un jour | Comportement |
|---|---|---|
| Alertes d'échéance des acomptes fournisseurs (VOY-270) | `voyages_acomptes_alertes_traiter_au(etab, jour)` | `jour = None` : jour de l'établissement ; une seule alerte par acompte (`alerte_le`) |
| Purge des exports CSV (`sor.export_argent`, 30 jours) | `voyages_argent_exports_purger_au(etab, jour)` | l'âge se compte en **jours de l'établissement** (`ent_aujourdhui_etab(cree_le) + 30 <= jour`) : un export créé le jour J est détruit à partir de J + 30 ; l'objet est supprimé, `purge_le` est inscrit et le registre des objets est marqué (`supprime_le`) |

**Pas d'heure réelle dans les tests** : `passages_quotidiens(Option<NaiveDate>)` et `passages_quotidiens_etablissement(etab, Option<NaiveDate>)` prennent le jour. Les anciennes entrées (`voyages_acomptes_alertes_traiter`, `voyages_argent_exports_purger`, route manuelle) restent, au jour de l'établissement.

Changement de comportement assumé : la purge ne compare plus à `now() - 30 jours` (à l'instant près) mais au jour de l'établissement (au plus 23 h de plus). *À valider si le DPO veut l'instant.*

**Tests** (`tests_voyages_v2a.rs`) : `passage_quotidien_alertes_d_acomptes_et_purge_des_exports_au_jour_donne` (un mois avant : 0 ; le jour J : 2 alertes ; relance : 0 ; plus tard : le lointain ; un seul avis par acompte ; export créé le 1er janvier : conservé le 30 janvier, détruit le 31 ; idempotent) ; variable d'environnement : `tests_planificateur::la_variable_desactive_les_passages_quotidiens` (`ent-api`).

## 2. Quota du guichet : jour de l'établissement

`controler_quota_guichet` (`facturation_comptes.rs`) comptait « le jour de Paris » (`date_trunc('day', now() AT TIME ZONE 'Europe/Paris')`). Il compte désormais les consultations dont `ent_aujourdhui_etab(etab, le)` est égal à `ent_aujourdhui_etab(etab, maintenant)` ; une borne de 26 h garde l'index de temps. La date de la tâche d'alerte `guichet_quota` est aussi le jour de l'établissement (plus `aujourd_hui()` de Paris). `controler_quota_guichet_a(…, instant)` prend l'instant : aucun test ne dépend de l'horloge.

**Test** `quota_du_guichet_compte_le_jour_de_l_etablissement_autour_de_minuit` (`tests_finances_f2.rs`, feature `labo`) : Auckland (UTC+12) — quota atteint à 23 h 45 locale, compteur remis à zéro à 0 h 30 locale alors que le jour de Paris n'a pas changé, **et** refus à 10 h 30 locale (0 h 30 à Paris : le jour de Paris a changé, pas celui de l'établissement) ; Cayenne (UTC−3) — même contrôle ; les consultations d'un autre agent ne comptent pas. Le test échoue avec l'ancien code dans les deux sens.

## 3. Outbox des finances : liste blanche par type d'événement (M7)

La liste blanche globale (`cle_autorisee` : tout `id`, `*_id`, `*_ids` + une liste commune) est **remplacée** par `CLES_PAR_TYPE` (`facturation_service.rs`) : pour chaque type canonique (`facturation.*`, `fin.*` rendu canonique par `canonique_facturation`, plus `eleve.regime_changed` et `eleve.sortie` que le service des finances publie par le même chemin) la liste **exacte** des clés (identifiants compris). Une clé permise pour un type est retirée d'un autre ; **un type absent de la table est fermé** (charge vide). La liste noire `cle_interdite` (montants, noms, IBAN…) reste, appliquée avant. `publier_finance` et les 14 appels directs de `service.rs` passent par `payload_neutre_pour(type, charge)` ; **`publier` (neutre, autres domaines) n'est pas touché** : demandes et internat publient comme avant.

Les listes sont l'union des clés réellement publiées par chaque type (28 sites de `publier_finance`, 14 de `service.rs`). `end_to_end_id` (prélèvement préparé) reste autorisé comme avant.

**Tests** : `facturation_service::tests` — `la_liste_blanche_est_propre_a_chaque_type`, `un_ancien_nom_donne_la_liste_du_nom_canonique`, `un_type_inconnu_est_ferme`, `la_table_ne_contient_aucune_cle_interdite_ni_doublon`, `chaque_type_publie_a_sa_liste_blanche` (**garde de complétude** : lit les sources, retrouve chaque type passé à `publier_finance` / `payload_neutre_pour` et exige sa ligne ; échoue si un nouveau type est publié sans liste). Les tests M7 existants (`f1_journal_evenements_…`, `payload_neutre_retire_…`) sont verts. **Tests `tests_demandes*` et `tests_internat*`** (qui avaient cassé la dernière fois) : exécutés avec la feature `labo` (les modules sont `cfg(feature = "labo")`, un `cargo test` sans la feature ne les compile pas) : verts, avec `tests_finances*` (159 tests au total).

## 4. Photos (F2 d'E9) et objets orphelins

Cause : `stocker_objet` écrit l'objet puis inscrit `ent_objet` dans **sa propre transaction** ; la ligne `ent_eleve_photo` n'arrive qu'après. Un arrêt du processus entre les deux laisse un objet inscrit au registre, jamais référencé. Le retrait « au mieux » existant ne couvre que l'échec de la transaction, pas l'arrêt du processus.

**Balayage** (`passages_quotidiens.rs`, même planificateur) : objets du registre (`ent_objet`, non supprimés) de **deux familles de clés seulement** — photos de forme immuable `{etab}/trombi/{eleve}/{uuid}.{ext}` (la forme historique `{etab}/trombi/{eleve}.{ext}` n'est **jamais** touchée) et exports `{etab}/voyages-argent/{uuid}.csv` — **absents de la table de référence** (`ent_eleve_photo.object_key`, `ent_sortie_argent_export.cle_objet`).
- **Délai de grâce** : jamais avant `GRACE_OBJETS_JOURS = 2` jours de l'établissement après l'inscription. Un objet récent n'est donc jamais supprimé.
- **Borné** : `LOT_BALAYAGE = 200` objets par famille, par établissement et par passage (le reste attend le passage suivant ; la ligne de journal dit si la borne est atteinte).
- **Journalisé** : ligne de journal d'application par établissement et par famille (nombres seulement, jamais de clé : celle d'une photo porte l'identifiant de l'élève) ; trace durable : `supprime_le` du registre (immuable).
- **Ordre sûr** : objet physique supprimé d'abord, registre marqué ensuite ; une suppression qui échoue laisse le registre intact, reprise au passage suivant. Idempotent.
- Aucune migration : le registre et les tables de référence existent.

**Tests** : `balayage_des_objets_orphelins_delai_de_grace_et_perimetre_ferme` (orpheline ancienne : balayée ; référencée : intacte ; récente, 1 jour : intacte puis balayée au 2e jour ; forme historique : intacte ; autre famille : intacte ; export orphelin balayé, export référencé intact ; idempotent) ; `balayage_des_objets_orphelins_est_borne` (205 orphelins : 200 puis 5).

## 5. Export CSV de V2a (F7) : ordre d'écriture

**Constat** : le fichier était écrit **avant** le commit (`objet_put`), et l'inscription au registre se faisait **dans la transaction du geste** : un arrêt du processus avant le commit laissait un objet **sans aucune trace** (ni registre, ni ligne d'export), donc impossible à balayer. Le retrait n'existait que pour l'échec du `commit`, pas pour une erreur plus tôt dans la fonction.

**Correction** (`voyages_argent_repo.rs`, `voyage_argent_export`) : l'objet est écrit **et inscrit au registre dans sa propre transaction validée avant** celle du geste (`stocker_objet`), comme les photos. Tout arrêt ensuite laisse un objet **connu du registre sans ligne d'export** : le balayage de la famille `voyages-argent` le détruit après le délai de grâce. Toute **erreur** (pas seulement le `commit`) après l'écriture retire tout de suite l'objet et marque le registre, sauf si la ligne d'export existe (commit dont la réponse s'est perdue) : `retirer_export_orphelin`. La purge à 30 jours marque aussi le registre. Le fichier n'est pas déplacé après le commit (le lien court émis dans le geste exige l'objet existant) : c'est le balayage qui couvre l'arrêt du processus, « de la même façon » que les photos.

**Test** `export_argent_objet_inscrit_puis_reference_jamais_balaye` : un export réel est inscrit au registre ; dans 10 jours (grâce écoulée) le balayage ne le touche pas parce qu'il est référencé ; dans 31 jours la purge de conservation le détruit et marque le registre. Le test `f7_export_purge_apres_trente_jours` existant est vert.

## 6. Compteurs (règle 28, `origin/ENT` = e5ddda1e, non bougé)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271** (inchangé ; aucune action) |
| `SOURCES` / `NOMS_SOURCES` | **65** (inchangé ; aucune route, aucun fichier de routes) |
| Compte RLS de `tests_rls.rs` | inchangé (aucune table) |
| Registre des données / `VERSION_CATALOGUE` | inchangés (`2026-11-18-v2a`) |
| Migrations | **aucune** (plage 653–659 inscrite dans `MIGRATIONS.md` : aucune consommée, toutes libres) ; `MIGRATION_TAG` `3808_ntf_gels_liens_a_prevenir` inchangé |

## 7. Vérifications (locales, PostgreSQL 16 jetable, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`)

- `ent-db` (lib) : voyages (110 tests dont 4 nouveaux), `facturation_service` (liste blanche), conservation, registre, photos ; **avec la feature `labo`** : `tests_finances*`, `tests_demandes*`, `tests_internat*` = **159** verts, quota inclus ; gardes `jour_utc_interdit` (2), `isolation_sql_statique` (2), `garde_finances_ecritures` (5), `garde_inscriptions_ecritures` (3) ; `ent-api` : séparation et suites `recette_finances_f0` / `or_facturation_historique` vertes, test du planificateur vert.
- Formatage (deux commandes bloquantes du job « Formatage », 1.99.0, jamais `--all`) : propre ; les fichiers du backend ne sont pas reformatés (dette connue non bloquante).
- `--locked` partout ; `Cargo.lock` inchangés.
- **Clippy 1.99.0 `-D warnings`, `--locked`, en trois commandes** (workspace ; workspace avec `--features ent-api/labo,ent-lab/labo` ; `-p ent-types`) : **0 avertissement** (toolchain 1.99.0 installée localement, identique à la CI).

## 8. Écarts, hypothèses, limites

1. **Test préexistant instable (hors lot)** : `recette_securite_3::rs3_02_double_encaissement_sepa_rejeu_et_concurrence` échoue (« aucun payeur joignable : la pré-notification ne peut pas être remise ») quand il tourne **en parallèle** de `rs3_01`, qui bascule temporairement `est_payeur` du lien de Lily sur le foyer de Harry. **Reproduit à l'identique sur `ENT` e5ddda1e sans aucune de mes modifications** (3 échecs sur 3 pour la paire, vert sur un seul fil) ; en fichier complet il a passé sur `ENT` et échoué deux fois avec mes changements : course temporelle sur les mêmes données de labo. Je n'y ai pas touché (hors périmètre, fichier non modifié) ; correction possible : sérialiser les deux tests ou donner à `rs3_01` son propre foyer. À traiter par un lot dédié.
2. **Cadence horaire** : le balayage et la purge interrogent le registre à chaque passage horaire (borné par `LIMIT`, filtré par famille de clés). Si le registre devient très gros, il faudra une marque « passage du jour » (migration) ou un index ; non nécessaire à l'échelle des pilotes.
3. **Âge des exports** : voir §1 (jour de l'établissement, pas l'instant).
4. **Orphelins hors des deux familles** : seules les photos d'élèves et les exports sont balayés ; les autres écritures du magasin (pièces, espaces, etc.) ont leurs propres tâches ou ne sont pas couvertes. Chaque nouvelle famille demande sa table de référence : à ajouter explicitement, jamais par motif générique.
5. **Charges d'événements `eleve.*` via `publier_finance`** : `eleve.regime_changed` et `eleve.sortie` figurent dans la table ; un autre type d'élève publié par ce chemin serait fermé (charge vide) jusqu'à sa ligne.
6. **Limites de test** : la suite complète d'`ent-db` n'a pas été rejouée localement (limites de temps de la session) ; la CI (20 tranches) fait foi.
