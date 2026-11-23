Tu es une session de correctifs ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **Lis :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` ;
  - la contre-revue : `git show origin/ent-mc0hoe:docs/pilotage/consignes/16_E1_CONTRE_REVUE.md`.
- **Branche :** E1 est versé sur ENT.
  - Crée `ent-socle-16-code-e1-p2` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - **491 est versée et ne change pas** (règle 25). Aucune autre migration versée ne change non plus.
  - Nouvelle migration **492** (plage du socle 16 : 482–499 ; 492 est libre). Mets à jour `migrate.rs`, `docs/socles/MIGRATIONS.md` et les compteurs (règle 28).
  - **Les fonctions de garde que tu crées ou recrées ont `SET search_path = pg_catalog, public, pg_temp`.**
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all`, `#[ignore]` ;
  - toucher à POINT_QUOTIDIEN ou à PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Exigences :**
  - zéro avertissement avec clippy 1.99 (les 3 commandes de la CI, `ent-lab` compris), `--locked`, fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - une base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR`.

## Décisions du pilote (D-069)
- **Curseur (M-2)** : pas de signature HMAC (aucune clé nouvelle à gérer).
  - Le serveur **borne** le curseur reçu : `xid` dans [xmin − marge, xmin courant], `differe` jamais dans le futur ni hors de la fenêtre, `emis` jamais postérieur à maintenant.
  - Hors bornes, le curseur est **refusé** (400 explicite) ou **ramené** au point de resynchronisation. Choisis l'un des deux et documente-le.
  - Le `rappel_libre` n'est accordé que si le curseur reçu est **exactement** celui que le serveur a rendu à cette session (mémoire du limiteur).
- **Restauration logique (F-1)** : lie le curseur à l'identité du cluster, par exemple `system_identifier` de `pg_control_system()`, ou une valeur stockée à l'installation. Un curseur d'un autre cluster est ramené au point de resynchronisation.

## Correctifs
- **M-1 (en tête)** : l'annulation de l'avis planifié se fait **dans la même transaction** que celle du message, avant la suppression de `ent_msg`.
  - Les avis `en_cours` sont annulés eux aussi. À défaut, le distributeur vérifie l'existence du message avant l'envoi, sous verrou.
  - Tests :
    - un échec simulé après la suppression ne laisse aucun avis en attente ;
    - course annulation / prise de l'avis par le distributeur à l'échéance : rien ne part pour un message annulé.
- **M-2** : comme décidé ci-dessus. Tests : curseur forgé (xid 0, `emis` futur, `differe` futur) refusé ou ramené ; rappel libre refusé pour un curseur forgé.
- **M-3** : migration 492.
  - Recrée la garde de 491 avec `SET search_path = pg_catalog, public, pg_temp` et des tables qualifiées.
  - Remplace le GUC libre par un **jeton lié à la transaction** : la fonction d'annulation inscrit `pg_current_xact_id()` et l'identifiant du message dans une table de garde ; la garde vérifie les deux.
  - Test : table temporaire homonyme ; GUC posé à la main sur un message déjà remis ; dans les deux cas, refus.
- **Faibles F-2 à F-6** : à faire s'ils sont sûrs, sinon écris pourquoi dans la livraison. En particulier :
  - `tronquees >= 1` asserté dans le test de transaction longue ;
  - erreurs exactes (23514 et le texte) ;
  - un réglage sur A ne permet pas de supprimer les versions de B ;
  - un fil dont le premier message est annulé ne montre plus son sujet à la famille.
- **Alerte d'exploitation** : un `xmin` gelé (transaction longue, transaction préparée orpheline) déclenche un `tracing::warn!` ou `error!` au-delà d'un seuil. C'est un paramètre d'établissement ou de déploiement, 10 min par défaut.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Si elle est rouge, corrige la cause. Un job « runner lost communication » se relance une fois.
  - Si la PR est en conflit avec `ENT`, aucune CI ne part : fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison d'une section « Passe 2 » : fait / non fait, tests, compteurs, tête, CI.
- Puis écris au pilote (send_message vers @parent) la tête finale et l'état de la CI.
