Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente** : `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` (sections Passe 2 et correctif du pilote).
- **Branche :** E1 passes 1 et 2 sont versées.
  - Crée `ent-socle-16-code-e1-suites` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - **482 à 492 sont versées et ne changent pas.**
  - Nouvelle migration **493** (plage 482–499). Mets à jour `migrate.rs`, `docs/socles/MIGRATIONS.md` et les compteurs.
  - **Règle 38 :** 493 se place **après 690** dans `migrate.rs`. Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`, et ses relations sont qualifiées `public.`.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production ;
  - les données réelles ;
  - `cargo fmt --all`, `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Qualité :**
  - zéro avertissement avec clippy 1.99 (les 3 commandes de la CI, `ent-lab` compris) ;
  - `--locked`, fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - rejoue les gardes statiques **sans** et **avec** la feature `labo` : `garde_rls_stricte`, `garde_messagerie_lecture`, `garde_search_path`.

## Correctifs (contre-revue de la passe 2 : `git show origin/ent-mc0hoe:docs/pilotage/SESSIONS_EN_COURS.md`, entrée « Contre-revue E1 passe 2 »)
1. **F-6, à corriger avant tout pilote réel.** Un fil dont le seul message a été annulé reste listé à la famille, avec son sujet.
   - Dans 493, recrée `ent_msg_fil_lisible` : la branche « fil sans aucun message » est réservée au **créateur du fil**, ou au côté établissement.
   - Le premier message en deux temps côté famille reste couvert : c'est le créateur.
   - Inverse le test de constat `p2_f6_constat_…` : la famille ne voit plus le fil.
   - Mesure le plan avant et après sur la liste des fils (`EXPLAIN`), et écris les chiffres dans la livraison.
2. **Jeton de garde d'annulation (492) falsifiable par le rôle applicatif.**
   - `ent_msg_annulation_ouvrir` reçoit l'auteur et vérifie que l'appelant est bien l'auteur d'un message pas encore remis.
   - Si le rôle applicatif n'est pas propriétaire des objets (vérifie-le), rends-la `SECURITY DEFINER` (propriétaire, `search_path` fixé) et `REVOKE INSERT, UPDATE, DELETE` de la table de garde au rôle applicatif.
   - Sinon, documente la limite.
   - Tests : un INSERT direct dans la table de garde est refusé ; ouvrir sur le message d'un autre auteur est refusé.
3. **Contrôle juste avant chaque envoi.** L'existence du message se vérifie juste avant **chaque** envoi, ou par lot d'un même message, et non plus en tête du lot.
   - `maintenant` vient de l'heure de la base.
   - Une erreur de base pendant le contrôle laisse l'avis `en_attente` au lieu d'interrompre le lot.
   - Test de vraie course : une transaction d'annulation reste ouverte sur une seconde connexion pendant que le distributeur tourne, et rien ne part pour le message annulé.
4. **Faux positif de l'alerte `xmin` gelé.** Ne compte comme gel que deux observations espacées de plus que le seuil, avec le même `xmin` **et** un `xmax` qui a avancé entre les deux.
   - Test de la transition « repos, longue pause, transaction brève ».
5. **Faibles :**
   - `message_existe_sous_verrou` en `pub(crate)` ;
   - contrôle d'existence regroupé par `message_id` ;
   - `system_identifier` mis en cache au démarrage ;
   - différé validé après son échéance (F-4) : documenté, ou servi au tour suivant ;
   - tests renforcés :
     - `bilan.reclames == 0` ;
     - témoin traité, pas resté `en_attente` ;
     - bornes réelles du curseur forgé ;
     - `tronquees >= 1` ;
     - rôle de la table de garde éprouvé sous rôle applicatif ;
   - curseur exempté en mémoire locale : documente le comportement derrière un répartiteur (affinité ou 429 sans perte).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison d'une section « Suites (passe 3) » : fait / non fait, tests, mesures, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
