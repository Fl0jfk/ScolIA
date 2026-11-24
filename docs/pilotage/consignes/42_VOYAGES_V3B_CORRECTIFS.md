Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/42_VOYAGES_V3_LIVRAISON.md` **en entier** (dont « Correctifs du pilote après contre-revue des suites ») ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/42_VOYAGES_V3B_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** reprends la branche **existante** `ent-socle-42-code-v3b-suites` (PR 145, brouillon) ; fusionne d'abord `origin/ENT` (règle 28).
- **Migrations :** 640–655 sont versées et ne changent **jamais** (règle 25). **656 n'est pas versée** : corrige-la en place (elle reste, placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (Moyens 1 à 3 bloquants, puis faibles)
1. **Moyen 1, pas d'autorisation « en blanc »** : la demande de contournement porte `cible_id` (acompte visé pour payer ou
   annuler), `montant_max_centimes` et `prestataire_id` (créer, payer) ; l'autorisatrice les voit (API) ;
   `ent_sortie_contournement_valide` et le déclencheur de pouvoir refusent toute écriture hors cible ou au-delà du plafond.
   Tests : autre acompte, montant supérieur, autre prestataire → refusés (service et SQL direct).
2. **Moyen 2, contrôle en base complet** : expiration (30 jours depuis l'autorisation) ; autorisatrice membre active de
   l'établissement avec la capacité `voyages.argent` (ou la règle la plus proche existant en base) ; horodatages posés par la
   base (`DEFAULT now()` ou déclencheur, jamais fournis par l'appelant) ; garde de base pour l'annulation collective. Tests
   SQL directs.
3. **Moyen 3, rattrapage sous RLS stricte** : énumération des établissements comme dans 2220
   (`ent_etablissement UNION ent_membership`) et restauration du réglage précédent ; test du rattrapage avec la RLS stricte.
4. **Faibles 1 à 8** du rapport (fenêtre d'ouverture, droit revérifié à l'usage, 403/404 harmonisés, consigne « aucun nom »
   et durée de conservation documentée, trace agrégée au-delà du plafond, approximation `rentree_le` documentée, borne de
   `rentree_le`, contrainte `NOT VALID` documentée). Faible 9 : documenté seulement.
5. Corrige le compteur annoncé : `Action::TOUTES` vaut **282** (vérifie dans `ent-auth/src/actions.rs`).
6. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse sur la même branche et laisse la PR 145 en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/42_VOYAGES_V3_LIVRAISON.md` une section « Suites V3b, correctifs » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
