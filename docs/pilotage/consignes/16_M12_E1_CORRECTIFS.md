Tu es une session de correctifs ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente** : `ScheduleWakeup` ne fonctionne pas ici (il ne se déclenche qu'en mode /loop). Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **Lis d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - la conception du socle 16 (messagerie) sous `docs/socles/`, M12 compris ;
  - la revue indépendante complète : `git show origin/ent-mc0hoe:docs/pilotage/consignes/16_M12_REVUE.md`. Ses numéros de ligne datent de la tête 96b73e09 : relis-les sur la base actuelle.
- **Branche et base :**
  - Crée la branche `ent-socle-16-code-m12-e1` depuis `origin/ENT`, qui contient déjà les suites M12 (PR 123).
  - Ouvre une PR en brouillon vers `ENT`.
- **Migration :**
  - Prends la nouvelle migration **491** (plage du socle 16 : 482–499 ; 482–490 sont versées et **ne changent pas**, règle 25).
  - Mets à jour `migrate.rs`, `MIGRATION_TAG` si 491 devient la plus haute, `docs/socles/MIGRATIONS.md` et le registre des données si besoin.
  - Recalcule les compteurs (règle 28).
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production ;
  - toute donnée réelle ;
  - `cargo fmt --all` et `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests : fixe un « maintenant » et compte en durées relatives.
- **Exigences de qualité :**
  - zéro avertissement avec clippy 1.99, sur les 3 commandes de la CI, `ent-lab` compris ;
  - `--locked` ;
  - fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - une base locale **neuve dont le nom contient « test »** ;
  - un seul `CARGO_TARGET_DIR`.

## Règles générales
1. **L'annulation d'un message différé est totale ou n'a pas lieu**, et l'auteur le sait. Jamais de remise d'un message que son auteur a annulé. Jamais d'erreur technique opaque (`ReferenceInconnue`) renvoyée à l'API.
2. **Le flux ne saute rien et ne répète rien sans limite.** La position du curseur ne dépend ni de l'horloge murale ni d'un catalogue qui n'est pas cloisonné par établissement.
3. **Un test prouve ce que son nom affirme.** Pas de preuve par l'horloge réelle. L'absence de N+1 se prouve en comptant les requêtes.

## Correctifs
Chemins sous `ent-rust/`. Le détail de chaque point est dans la revue.
- **E1 (élevé)** : corriger puis annuler un message différé échoue, et le message part quand même.
  - Cause : `ent_msg_version` a une clé étrangère sans `ON DELETE` et un déclencheur d'ajout seul. L'exception `ent.msg_annulation` de la migration 490 ne la couvre pas.
  - Correctif, dans la migration 491 :
    - étends l'exception bornée à `ent_msg_version` ;
    - supprime les versions dans la même transaction, avant `ent_msg`.
  - Côté API :
    - erreur métier exploitable si l'annulation est refusée ;
    - si le message est déjà remis, refus explicite « déjà remis : retire-le ».
  - Tests :
    - « corriger puis annuler » : plus rien n'est remis, et il ne reste ni version, ni carte, ni avis planifié ;
    - « corriger deux fois puis annuler » ;
    - « annuler après la remise » : refus exact ;
    - test HTTP de la route.
- **M1** : curseur fondé sur `xid8`.
  - Ajoute une colonne `xid8 DEFAULT pg_current_xact_id()` sur `ent_msg`. Ajoute aussi l'équivalent pour les événements de retrait et de modification, si le flux les lit.
  - Borne du curseur : `pg_snapshot_xmin(pg_current_snapshot())`.
  - Abandonne `pg_stat_activity`, la dépendance à `pg_read_all_stats` (F4), le plafond `borne_sure` et la fuite F1.
  - Le format du curseur côté client peut changer : c'est une branche non déployée. Documente-le.
  - Les lignes existantes doivent rester lisibles par la reprise.
- **M2** : un vrai test de transaction longue.
  - Une transaction ouverte écrit tôt et valide tard, après qu'un curseur est passé.
  - Le message est servi au tour suivant.
  - Combine ce test avec une troncature.
- **M3** : plus de répétition permanente et plus de boucle serrée.
  - Une interrogation longue n'est servie que pour un événement strictement postérieur à la dernière position rendue.
  - Après un événement, le rappel suivant attend.
  - Teste-le : deux rappels successifs, le second ne renvoie pas le même événement et attend.
- **M4** : renforce les tests.
  - `revue_f6` couvre : plusieurs élèves, un élève protégé, un fil de boîte, un fil réservé, un parent sans lien communicant.
  - Le sans-N+1 se prouve par un compte de requêtes, pas par une durée.
  - Pour le HTTP :
    - l'en-tête `Retry-After` est vérifié ;
    - un rappel après `tronque` n'est pas refusé ;
    - le plafond de 8 n'écrase pas une pagination (voir F2) ;
    - aucune attente réelle de 3 s (temps injecté) ;
    - plus de `limite_flux().vider()` global partagé entre tests : instance par test.
- **F1-F7** : à faire. Tu peux écarter un point en écrivant pourquoi dans la livraison.
  - F1 et F4 tombent avec M1.
  - F2 : exempter les rappels libres du plafond.
  - F3 : isoler l'accès au catalogue dans un module dédié, avec un extrait exact et unique. S'il disparaît avec M1, retire l'exception.
  - F5 : `<` strict.
  - F6 : ordre lisibilité → auteur dans `annuler_message_differe_for_actor`.
  - F7 : `Utc::now()` restants dans la messagerie, à remplacer par l'horloge unique si c'est sans risque. Sinon, en faire la liste.

## Fin
- Pousse sur `ent-socle-16-code-m12-e1` et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Si elle est rouge, corrige la cause.
  - Un job « runner lost communication » se relance une fois.
  - Si la PR est en conflit avec `ENT`, aucune CI ne part : fusionne `origin/ENT` (règle 28) et repousse.
- Écris `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : la tête finale et l'état de la CI.
