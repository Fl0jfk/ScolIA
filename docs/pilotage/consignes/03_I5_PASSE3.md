Tu es une session de correctifs ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md` ;
  - la contre-revue : `git show origin/ent-mc0hoe:docs/pilotage/consignes/03_I5_P2_CONTRE_REVUE.md`. Ses numéros de ligne sont ceux de cf878246 ; relis-les sur la base actuelle.
- **Branche :** I5 est **versé sur ENT**.
  - Crée `ent-socle-3-code-i5-p3` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - **313 est versée et ne change pas** (règle 25), pas plus qu'aucune autre migration versée.
  - Nouvelle migration **314** (plage du socle 3 : 300–329 ; 314 est libre). Mets à jour `migrate.rs`, `docs/socles/MIGRATIONS.md`, le registre et les compteurs (règle 28).
  - **Les fonctions de garde que tu crées ou recrées ont `SET search_path = pg_catalog, public, pg_temp`.** Une table temporaire homonyme ne doit pas les tromper.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all`, `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Exigences :**
  - zéro avertissement avec clippy 1.99 (les 3 commandes de la CI, `ent-lab` compris), `--locked`, fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - **rejoue les tests du frontend contre le backend** (`ent-web-server`, `--features labo`, inscriptions et demo_p1), comme la CI.

## Règles générales
1. **Un seul ordre des verrous**, posé en un point unique : dossier → offre → propositions.
   - Plusieurs dossiers ou plusieurs offres : par ordre d'UUID.
   - Un appelant qui tient déjà l'offre ne verrouille jamais un dossier après elle. Il calcule d'abord en lecture, puis verrouille dans l'ordre.
2. **Ce que voit la famille ne dépend que des décisions publiées**, export RGPD compris.
3. **Une décision publiée correspond à l'état du dossier au moment de la publication**, sinon elle est caduque et tracée.

## Correctifs (moyens de la contre-revue, numérotés comme elle)
1. **Ordre des verrous.**
   - Concerne : `designer_suivants_tx` et ses appelants (`apres_transition_tx`, `ecarter_proposition_for_actor`, la décision après `retenir_voeu_tx`, désistement/refus), les déplacements croisés (offre `apres`/`avant`), `reprendre_place_designee_tx` et `reutiliser_core` (dossier cible / source).
   - Applique la règle 1.
   - **Test de concurrence réel** : deux transactions croisées (refus de la famille sur X / décision de la direction sur le suivant Y ; deux déplacements A→B et B→A ; deux réutilisations croisées) se terminent toutes deux, sans 40P01 ni conflit.
2. **Registre et export.**
   - Ajoute les `export_si` manquants :
     - propositions : seulement `envoyee`, `acceptee`, `declinee`, `expiree` ;
     - conditions : seulement celles d'une décision publiée ;
     - suivi : `visible_famille`, et décision nulle ou publiée.
   - Étends `hors_export` : `voeu_retenu_id`, `clos_le`, `motif_cloture`, `decide_le` du dossier tant que la décision n'est pas publiée (ou en entier si une règle de ligne est impossible), statut des vœux `retenu`/`non_retenu` avant publication, `remplace_decision_id`, `depassement_capacite`, `alternative`.
   - **Test qui interroge réellement les lignes exportables** après une décision non publiée (avec la condition SQL de `export_si` exécutée).
3. **Publication caduque.**
   - À la publication (tâche de fond et `publier_ou_programmer_tx`), vérifie que l'état du dossier correspond encore à `etat_cible(sens)`.
   - Sinon, la décision est marquée **caduque** (journal en ajout seul, motif), elle n'est **ni publiée ni notifiée**, et aucun événement `ADMISSION_DECIDEE` n'est émis.
   - Test : admission programmée à J+7, dossier annulé à J+3 ; à J+7 rien n'est publié et la caducité est tracée.
4. **Séquence `ordre_attente`.** Dans 314, `setval('ent_inscription_attente_ordre_seq', GREATEST(max(ordre_attente), 1))`.
   - Test : un dossier repris d'ordre 3, puis un nouvel entrant, passe derrière, côté famille comme côté désignation.
5. **Écarter une proposition.**
   - N'accepte que `origine = liste_attente AND decision_id IS NULL`.
   - Sinon, refus exact (variante explicite) : passer par une nouvelle décision.
   - Test.
6. **Réparation de chaîne (dans 314).** Rechaîne, de façon déterministe, tout dossier qui a plus d'une racine ou un double successeur. Les index uniques ne doivent jamais faire échouer la migration.
   - Test sur un historique partiel (d1 racine, d2 remplace d1, d3 racine), avec un ordre de chaîne vérifié.

## Décision du pilote (D-069) sur l'acceptation au guichet (F15)
- L'acceptation au guichet **sans « deux personnes »** est conservée (motif obligatoire et journal en ajout seul).
- Elle est en revanche **refusée après `repondre_avant`** si l'expiration n'est pas constatée.
- Elle passe la proposition à `acceptee`, ce qui fait disparaître l'action « répondre » côté famille.
- Elle **notifie la famille**.
- Les refus doivent être distincts : « plus à l'état admis » et « déjà répondue ».
- Tests : 403 sans droit ; proposition `designee`, `expiree`, `annulee`, `declinee` ; acteur écarté → introuvable.

## Faibles (à faire s'ils sont sûrs ; sinon, écrire pourquoi dans la livraison)
- motif de proposition masqué hors `inscriptions.decider` ;
- filtre des écartés **avant** le `LIMIT` ;
- `liberations` incrémenté du nombre de places ajoutées ;
- désignation relancée si la ligne verrouillée a changé (pas de `break` silencieux) ;
- `dossier_dto` lu dans une seule transaction ;
- rang lu d'une seule façon (même règle dans `decision_famille` et `dossier_dto`) ;
- `information_schema` filtré par `table_schema` ;
- `r_m4_…` : assertion exacte.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - PR en conflit avec `ENT` : aucune CI ne part. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison I5 d'une section « Passe 3 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
