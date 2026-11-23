Tu es une session de correctifs ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/socles/41_INTERNAT.md` (I4) ;
  - la livraison I4 sur la branche (`docs/socles/41_INTERNAT_I4_LIVRAISON.md`) ;
  - la contre-revue : `git show origin/ent-mc0hoe:docs/pilotage/consignes/41_I4_CONTRE_REVUE.md`. Ses numéros de ligne sont ceux de 79b436cb.
- **Branche :** `ent-socle-41-code-i4` (PR n° 122, brouillon).
  - Pars de sa tête distante (79b436cb).
  - Puis **fusionne `origin/ENT`** et recalcule les compteurs depuis la base commune (règle 28).
- **Migrations :**
  - **2828 et 2829 ne sont pas versées : tu peux les modifier.**
  - Aucune autre migration versée ne change.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all`, `#[ignore]` ;
  - toucher à POINT_QUOTIDIEN ou à PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Exigences de qualité :**
  - zéro avertissement, avec clippy 1.99 sur les 3 commandes de la CI, `ent-lab` compris ;
  - `--locked` ;
  - fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »** ;
  - un seul `CARGO_TARGET_DIR`.

## Décisions du pilote (D-069, ne pas redemander)
- **É1** : un élève parti en activité reste suivi jusqu'à son retour pointé, quoi qu'il arrive au temps.
  - **L'annulation d'un temps reste possible**, car c'est le cas de l'urgence. La veille et la marque « en activité » suivent pourtant tout inscrit pointé `present` sans retour, **quel que soit le statut du temps ou le retrait de son inscription**.
  - Le retour se pointe aussi sur un temps annulé : seul le retour, pas un nouveau départ.
  - La désinscription d'un élève pointé `present` sans retour est **refusée**, avec un message explicite.
  - Il faut des tests pour : l'annulation avec des élèves en activité, la désinscription refusée, et le retour pointé sur un temps annulé, qui ferme le cas.
- **M7** : l'accompagnateur compte ses élèves, protégés compris.
  - Un élève protégé apparaît comme « élève protégé », sans nom, dans les listes de sortie et de temps du soir. Son état et son départ ou retour restent visibles.
  - Les événements ne portent jamais son nom.
  - Aligne les temps du soir sur ce comportement : l'élève n'en est plus retiré, il est masqué. Teste les deux.
- **F-g** : un refus s'enregistre toujours, même saisi pour un parent interdit. Il est plus restrictif et il est tracé.
- Tolérance « non rentré » : 15 min (déjà en paramètre d'établissement, à garder).

## Correctifs
Les chemins sont sous `ent-rust/`. Le détail de chaque point est dans la contre-revue.
- **É1** : comme décidé ci-dessus.
- **M1** : la veille n'ouvre pas de nouveau cas si un cas existe déjà, quelle que soit son issue, pour le même triplet (élève, sortie ou temps, nuit).
  - Si un cas est déjà ouvert par l'appel, rattache-lui la sortie ou le temps et l'heure prévue, pour que le retour le ferme.
  - Tests : cas fermé par le CPE, puis minute suivante ; cas fermé par l'appel ; cas ouvert par l'appel avant la veille.
- **M2** : un point de reprise par élève, sur le modèle de `veille_sorties_bilan_a` : une ligne en erreur ne bloque pas l'établissement.
  - Ajoute `suivre_echecs_veille` aux deux nouvelles veilles.
  - Test : une ligne qui échoue n'empêche pas les autres.
- **M3** : un inscrit pointé « absent » à l'étude ouvre un cas ou un signal, comme un non-pointé.
  - Branche `internat.etude_non_pointee` et `internat.alerte` sur un destinataire réel : CPE ou vie scolaire de l'internat, selon le modèle de I3.
  - Si I3 ne consomme pas `INTERNAT_CAS_OUVERT`, branche-le.
  - Test : le destinataire reçoit bien quelque chose.
- **M4** : un accord annuel « soirée » ne couvre aucune sortie qui **recoupe** [ancre de la nuit, matin 6 h]. Il ne suffit plus qu'elle ne la couvre pas entièrement.
  - Test sur 00 h 30 → 02 h 00, et sur une sortie 21 h → 23 h qui reste autorisée.
- **M5** : registre des données.
  - `.hors_export(&["motif_code", "reponse_user_id"])` sur `ent_internat_sortie_groupe_participant`.
  - Vérifie aussi `ent_internat_cas` (`issue_note`, nature `interdiction_auteur`) et applique la même règle.
  - Le test doit lire la déclaration réelle des colonnes.
- **M6** : tests de régression :
  - le branchement `internat_sorties_tache` → nouvelles veilles : la suppression d'un appel doit faire échouer un test ;
  - une sortie à cheval sur minuit ;
  - un cas déjà ouvert par l'appel ;
  - É1 et M1.
- **F-a à F-f** : à faire si sûrs, sinon écrire pourquoi dans la livraison.
  - F-a : la cause exacte de la chute de l'accord.
  - F-b : un signal par élève.
  - F-c : une erreur claire.
  - F-d : test de fusion avec « parti » chez l'absorbé.
  - F-e : écrire les paramètres seulement s'ils manquent.
  - F-f : noter la limite.

## Fin
- Pousse sur `ent-socle-41-code-i4` et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Si elle est rouge, corrige la cause. Un job « runner lost communication » se relance une fois.
  - Si la PR est en conflit avec `ENT`, aucune CI ne part : fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison I4 avec une section « Passe 2 » : fait / non fait, tests, compteurs, tête, CI.
- Puis écris au pilote (send_message vers @parent) la tête finale et l'état de la CI.
