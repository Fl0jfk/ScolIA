# Contre-revue de la passe 2 du lot I5 (PR 120, tête cf878246) — revue indépendante du pilote

Numéros de ligne : ceux de cf878246 ; les relire sur ENT (I5 est versé).

**Contre-revue du lot I5, passe 2 (copie figée cf878246) : aucun point de gravité Élevée.**

Je n'ai trouvé aucune fuite active vers la famille, aucune double attribution de place, aucun contournement de cloisonnement ni de droit sur la nouvelle route. Les défauts confirmés sont de gravité Moyenne : ordre des verrous, registre d'export, reprise de la migration 313, garde manquante à la publication. Aucun test n'a été exécuté (pas de base de données) : tout vient de la lecture du code. Chemins sous `/tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/rev-i5/ent-rust/`.

## Moyen

**1. Ordre des verrous F5/F12 non tenu : interblocage possible (confirmé par lecture)**
- `crates/ent-db/src/inscriptions_attente.rs:371-404` et `:430` : `designer_suivants_tx` verrouille « le premier suivant avant l'offre ». Le commentaire est faux dès que l'appelant tient déjà l'offre, ce qui est le cas de quatre appelants sur cinq :
  - `apres_transition_tx` (`:494-519`) ;
  - `ecarter_proposition_for_actor` (offre prise par `verrouiller_proposition_tx`, `:1033`) ;
  - `inscriptions_decisions.rs:526`, juste après `retenir_voeu_tx` ;
  - le désistement ou le refus d'un dossier admis.
- Seul `designer_suivants_for_actor` respecte l'ordre.
- Scénario : T1 (refus de la famille sur X) tient X puis l'offre et attend le dossier Y, le suivant. T2 (décision de la direction sur Y) tient Y et attend l'offre. Résultat : 40P01.
- Conséquence : le code le mappe en `ConflitDossier`, donc pas de double attribution, mais l'action légitime échoue au hasard.
- Même famille de défauts :
  - `crates/ent-db/src/inscriptions_decisions.rs:523-527` verrouille l'offre `apres` puis `avant`, dans un ordre qui dépend des données (deux déplacements croisés A→B et B→A) ;
  - `reprendre_place_designee_tx` (`inscriptions_attente.rs:152-164`) verrouille les propositions d'autres dossiers sans leur verrou de dossier ;
  - `crates/ent-db/src/inscriptions_pieces.rs:1171-1172` (`reutiliser_core`) verrouille le dossier cible puis le dossier source, dans un ordre quelconque (deux réutilisations croisées s'interbloquent).
- Correctif : poser la règle « dossier → offre → propositions » en un point unique. Par exemple, `apres_transition_tx` calcule le suivant en lecture, verrouille ce dossier, puis l'offre. Pour plusieurs dossiers ou plusieurs offres, verrouiller dans l'ordre des UUID.

**2. Registre et export RGPD (M-D) : déclaration incomplète et sans effet vérifié**
- Aucun code ne lit `colonnes_hors_export` ni `export_si`, hors tests. Les commentaires renvoient à un « lot G3 » à venir.
- Le test `p2_md_registre_export_publie_seulement` (`crates/ent-db/src/tests_inscriptions_i5_revue.rs:1910-1976`) n'exerce aucun export : il compare des constantes et vérifie que les colonnes existent. Son titre sur l'export n'est donc pas prouvé.
- Les règles déclarées laissent passer de l'interne (`crates/ent-db/src/registre_donnees.rs`) :
  - `:833` `ent_inscription_proposition_place` n'a aucun `export_si`. Les propositions `designee`, `ecartee` et `annulee`, jamais lues par la famille, seraient exportées, avec `decision_id` (décision éventuellement non publiée), `place_no` et `designee_le`.
  - `:805` `ent_inscription_condition` n'a aucun `export_si`. Le `libelle` d'une condition `reserve` issue d'une décision non publiée serait exporté.
  - `:817-820` `ent_inscription_dossier` : ne sont pas retirés `voeu_retenu_id`, `clos_le`, `motif_cloture` (posés par `transitionner_tx` avec le motif communiqué d'une décision non publiée) ni `decide_le`.
  - `:843` `ent_inscription_voeu` : le statut `retenu` / `non_retenu` est exporté.
  - `:839-842` `ent_inscription_suivi_dossier` : l'`export_si` ne filtre pas `visible_famille`. Les lignes internes sans `decision_id` (`kind` `avis`, `deport`, `liste_attente`) sont exportées avec leur type et leur date.
  - `:811-813` `ent_inscription_decision` : `remplace_decision_id` d'une décision publiée peut pointer vers une décision antérieure jamais publiée (fuite de l'identifiant seulement). `depassement_capacite` et `alternative` ne sont pas retirés.
- Correctif : ajouter les `export_si` manquants (par exemple `statut IN ('envoyee','acceptee','declinee','expiree')` pour les propositions, et pour le suivi `visible_famille AND (decision_id IS NULL OR …publiée)`). Étendre `hors_export` aux colonnes ci-dessus. Ajouter un test qui interroge réellement les lignes exportables après une décision non publiée.

**3. Publication d'une décision sans garde sur l'état courant du dossier (confirmé par lecture)**
- `crates/ent-db/src/inscriptions_decisions.rs:563-658` (`publier_decision_tx`), appelé par la tâche de fond (`:873-915`) et par `publier_ou_programmer_tx`.
- Scénario : décision « admis » programmée à J+7. Avant J+7, le secrétariat annule le dossier ou la famille le retire, ce qui annule la proposition. À J+7 la décision est quand même publiée : la famille reçoit « Une décision a été prise » et lit `sens = admis` alors que l'état est `annule`. L'événement `ADMISSION_DECIDEE` part aussi.
- Correctif : à la publication, vérifier que l'état du dossier correspond encore à `etat_cible(sens)`. Sinon refuser ou classer la décision « caduque », avec trace.

**4. Reprise de `ordre_attente` : séquence non avancée (confirmé par lecture)**
- `migrations/313_inscription_decision_i5.sql:373-380` recopie `ordre_attente = rang_attente` (valeurs 1..n), mais la séquence `ent_inscription_attente_ordre_seq` démarre à 1 (`crates/ent-db/src/inscriptions_modele.rs:529-530`).
- Scénario : un dossier repris porte l'ordre 3 et un nouvel entrant reçoit `nextval` = 1. `rang_famille_tx` (`inscriptions_attente.rs:265`) classe le nouvel entrant devant l'ancien, alors que `SUIVANT_A_DESIGNER` (qui trie sur `rang_attente`) le met derrière. Le rang vu par la famille et l'ordre de désignation divergent.
- Correctif : après la reprise, `setval('ent_inscription_attente_ordre_seq', GREATEST(max(ordre_attente), 1))`.

**5. Écarter une proposition d'origine `decision` ou rattachée à une décision non publiée (confirmé par lecture)**
- `inscriptions_attente.rs:1464-1515` (`ecarter_proposition_for_actor`) vérifie seulement `statut = designee`, pas `origine = liste_attente`.
- Les propositions créées par `preparer_proposition_tx` (`:584-600`) n'ont pas de `place_no`. La condition `e.place_no = $4` ne vaut donc jamais et la règle M-C ne joue pas pour elles.
- Sur une décision « admis » programmée, la proposition `designee` est rattachée à la décision. L'écarter laisse la décision publiable. À la publication, `envoyer_proposition_publiee_tx` ne trouve aucune `designee` : admission publiée sans proposition `envoyee`, sans délai, sans action « répondre » côté famille.
- Correctif : n'accepter que `origine = liste_attente AND decision_id IS NULL`. Sinon, passer par une nouvelle décision.

**6. Réparation de chaîne de la migration 313 : cas partiel non couvert**
- `migrations/313_inscription_decision_i5.sql:90-112` ne répare que les dossiers dont toutes les décisions sont des racines (`racines = total`).
- Un historique partiel (d1 racine, d2 qui remplace d1, d3 racine) fait échouer l'index unique `ent_inscription_decision_racine_uq` (`:116`), et `successeur_uq` si deux décisions remplacent la même. La migration abandonne et bloque le démarrage.
- Le test `p2_f16_313_rejouable_sur_ancienne_version` (`tests_inscriptions_i5_revue.rs:2343-2429`) ne couvre que le cas « toutes racines » et n'assert que `racines == 1`, pas l'ordre de la chaîne.
- Correctif : rechaîner tout dossier qui a plus d'une racine ou un double successeur, avec un ordre déterministe, et ajouter ces cas au test.

## Faible

- **Route d'acceptation au guichet (F15)** (`inscriptions_attente.rs:1522-1607`) :
  - **Correct** :
    - ordre droit (`InscriptionsGerer`, 403) → existence et écart (introuvable) → état de la proposition ;
    - cloisonnement par établissement (RLS, filtres, test `crates/ent-api/tests/separation.rs:1296`) ;
    - motif de 3 à 500 caractères ;
    - journal en ajout seul (genre `guichet`), suivi, événement ;
    - une seule acceptation possible (un second appel donne un conflit) ;
    - verrous dossier → offre → proposition.
  - **Manque** :
    - aucune vérification de `repondre_avant` : acceptation au guichet possible après le délai, tant que l'expiration n'est pas constatée, sans mention dans la trace ;
    - aucune notification à la famille (une seule ligne de suivi visible) ;
    - « dossier plus à l'état admis » et « déjà répondue » donnent la même variante.
  - **Tests** : seuls `SaisieInvalide` (motif vide) et `ConflitDossier` (seconde saisie) sont prouvés. Non prouvés : droit insuffisant (403), proposition `designee`, `expiree`, `annulee`, `declinee`, acteur écarté (introuvable).
  - **Effet de bord** : après acceptation au guichet ou levée de `reponse_famille`, la proposition reste `envoyee`. La famille voit encore l'action « répondre », qui échoue ensuite sur « aucune place proposée » (`:1707`).
- **Motif de proposition visible de tout le personnel** : `PropositionDto.motif` (`:980-984`, `propositions_for_actor`) n'est pas masqué hors `inscriptions.decider`, alors que le motif interne des décisions l'est. Il contient « Tour passé… » et « Place reprise par une admission directe : {motif} », avec parfois des informations sur un autre candidat. La liste est filtrée pour les écartés après un `LIMIT 500` (`:1379`, `:1388`) qui peut tronquer.
- **Message « en cours de modification »** (`inscriptions_decisions.rs:1629-1640`) : la famille apprend qu'une décision non publiée existe (message neutre, mais indice binaire).
- **Suivi rattaché à une décision remplacée avant publication** : jamais lisible (`inscriptions.rs:1523-1528`), y compris des informations légitimes comme « pièce refusée » écrites entre-temps.
- **Fonctionnement de M-C** :
  - le numéro de place est global par offre. Une hausse de capacité de +3 n'incrémente `liberations` que de 1 (`inscriptions_modele.rs:584`, `:951`), donc un dossier qui a passé son tour n'est pas reproposé pour les deux autres places ;
  - toute hausse de capacité, même sans place réellement libre (offre en dépassement), remet les dossiers écartés éligibles ;
  - les `ecartee` antérieures à la migration ont `place_no` NULL et ne protègent plus.
- **Désignation manquée en silence** : `SUIVANT_A_DESIGNER … LIMIT 1 FOR UPDATE OF d` (`inscriptions_attente.rs:287-297`, `:430`). Sous READ COMMITTED, si la ligne verrouillée a changé entre-temps, la requête renvoie 0 ligne et la boucle fait `break` alors que d'autres dossiers sont éligibles. La place reste sans désignation jusqu'au prochain déclencheur.
- **Lecture non atomique** : `dossier_dto` (`inscriptions.rs:1497-1565`) ouvre une transaction par lecture. Un état et un rang peuvent venir de deux instants différents (obsolescence, pas de fuite).
- **Rang lu de deux façons** : `decision_famille` exige `sens == liste_attente` (`inscriptions_decisions.rs:1563`), `dossier_dto` non (`inscriptions.rs:1556`). Un dossier en liste sans aucune décision a un rang dans l'un seulement. Le helper `rang_vu` des tests les compare en `assert_eq!`, mais ce cas n'est pas testé.
- **Migration 313** :
  - `information_schema.columns` est interrogé sans `table_schema` (`:23-24`) ;
  - le `DROP` puis `ADD CONSTRAINT` de `ent_inscription_suivi_dossier_kind_i5_chk` (`:214-219`) reprend un verrou exclusif et revalide la table à chaque rejeu ;
  - hypothèse non vérifiée : si un ancien déclencheur d'ajout seul total existait déjà sur `ent_inscription_decision` sans `publiee_le`, l'`UPDATE` de reprise (`:32`) échouerait. Le test ne simule pas ce cas.
- **Tests** :
  - `r_m4_chaine_et_concurrence` (`tests_inscriptions_i5_revue.rs:809`) : `r3.is_ok() || r4.is_ok()`, assertion lâche pour une course.
  - Tous les tests de la base se terminent par un `return` silencieux sans `ENT_DATABASE_URL` : en CI sans base, la passe est verte sans rien prouver.
  - Les instants des tests p2 sont fixes (pas de dépendance à l'heure réelle ni au jour UTC), mais `transitionner_tx` calcule l'échéance avec `ent_aujourdhui_etab($2)` (`inscriptions.rs:696`), forme à 2 arguments dont je n'ai pas lu la définition. Si elle lit l'horloge réelle, les assertions sur `echeance_famille` en dépendent. Je n'ai pas vérifié.
  - Aucun test M-C pour « place ajoutée sans place libre » ni « écarter une proposition liée à une décision ».
- **Non vérifié** : `decider_tx` prend un verrou sur la ligne de campagne (`inscriptions_decisions.rs:373-381`) après le dossier et avant l'offre. Je n'ai pas cherché si un chemin tient la campagne puis le dossier (ce qui inverserait l'ordre).

## Vérifié correct (bref)

- **Rang famille (M-A)** : un seul calcul `rang_famille_tx` (`inscriptions_attente.rs:252-283`), via `ent_inscription_en_liste_visible`, utilisé par `dossier_dto` et `decision_famille`. Les décisions non publiées sont masquées partout où la famille lit : état, rang, échéance, actions, suivi rattaché à la décision, conditions, proposition, motif communiqué. Les notifications et les événements `ADMISSION_DECIDEE`, `INSCRIPTION_PLACE_PROPOSEE`, `INSCRIPTION_DECISIONS_PUBLIEES` ne partent qu'à la publication.
- **Actions de la famille** (réponse, dépôt de pièce, désistement, retrait) : protégées par `exiger_dossier_stable_famille_tx`, sous le verrou du dossier.
- **Chaîne des décisions** : racine et successeur uniques imposés en base, aucun tri sur `now()`.
- **Garde d'écart** : `ecarte_tx` / `dossier_visible_tx` avec second verrou en base ; le droit passe avant l'écart, l'écarté reçoit « introuvable ».
- **Pas de `SKIP LOCKED`** dans les fichiers du lot.
- **Migration 313** : RLS et FORCE posés par `ent_rls_poser` sur les 8 nouvelles tables (`ent_inscription_decision` l'est déjà par la migration 586). Les fonctions sont en `SECURITY INVOKER`, donc pas de `search_path` à fixer. L'ordre d'application suit le tableau de `crates/ent-db/src/migrate.rs` (313 passe après 580).

