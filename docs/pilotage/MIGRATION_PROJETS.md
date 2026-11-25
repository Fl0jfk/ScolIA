# Migration du pilotage vers les Projets (bêta) de Claude Code

Décision de Florian du 2026-11-25. Florian a accès à la bêta. Ce document sert de kit : il réunit ce qu'il faut coller
dans le Projet, et ce qu'il faut vérifier avant et après la bascule.

**État** : en préparation. Le pilote donne le feu vert à Florian quand les sessions en cours sont terminées, revues et
versées (voir § 1). La bascule se fait ensuite avec Florian.

---

## 0. Ce que change l'outil, et ce qu'il ne change pas

Ce que l'outil change :
- **Fils** : le Projet coordonne des **fils**. Chaque fil est une session cloud de Claude Code, avec sa propre branche et sa
  propre copie du dépôt.
- **Suivi** : Florian voit l'état de chaque fil au même endroit, ce que tenait jusqu'ici `SESSIONS_EN_COURS.md`.
- **Lancements** : ils deviennent plus simples, car le contexte est commun aux fils.

Ce que l'outil ne change pas : **tous les garde-fous de ScolIA viennent de nos règles, pas de l'outil.** Ils doivent être
écrits dans les instructions du Projet (§ 2) et dans chaque consigne de fil (§ 3). Sans cela, ils disparaissent. En
particulier :
- au plus **2 fils qui codent** à la fois ;
- une **contre-revue indépendante** avant tout versement ;
- **seul le coordinateur verse sur `ENT`**, en avance rapide ;
- la **CI d'ENT est verte** avant chaque nouveau versement ;
- **ordre des migrations** (règle 38) et **chaînes de PR** ;
- interdits absolus de `CLAUDE.md` : Next.js, production, secrets, `main` et `dev`.

Limites de la bêta, à garder en tête : un projet n'a qu'un propriétaire, il ne se partage pas, et il n'y a pas de
réglages d'organisation.

---

## 1. Avant la bascule (pilote actuel)

1. **Faire aboutir les fils en cours** : contre-revue, correctifs, versement, puis CI d'ENT verte.
   - MB1 serveur (enveloppe d'erreur codée, `Idempotency-Key`, migration 2227).
   - G3a, guichet des droits RGPD (migration 2544).
   - Correctifs F3 suites 4 (PR 156, 1 élevé) : lancés à la première place libre (consigne
     `consignes/24_F3_SUITES_4_CORRECTIFS.md`).
2. **Ne lancer aucun autre lot.**
3. **Verser l'état final** dans `SESSIONS_EN_COURS.md` et dans le point quotidien : ENT, PR ouvertes, file.
4. **Nettoyer** : archiver les sessions terminées et supprimer les worktrees de revue du pilote.
5. **Feu vert à Florian.**

## 2. Instructions du Projet (à coller par Florian à la création)

> Tu es le **coordinateur** du chantier ScolIA (nouvel ENT en Rust, branche `ENT`). Réponds toujours à Florian en
> français. Lis d'abord `CLAUDE.md`, puis `docs/PILOTAGE.md` et `docs/pilotage/MIGRATION_PROJETS.md` sur la branche
> `ent-mc0hoe`. Ils font foi.
>
> **Interdits absolus** (`CLAUDE.md`) :
> - aucune modification de Next.js, sauf le chantier D-053 ;
> - aucun accès à la base de production, même en lecture ;
> - aucun secret de production consulté ni demandé ;
> - aucun déploiement ;
> - aucun commit ni push sur `main`, `dev` ou une branche de production ;
> - aucun code AGPL ;
> - aucune donnée réelle.
>
> **Rôles** : tu conçois les consignes, tu lances les fils, tu fais les contre-revues, tu verses. Les fils codent. Modèle
> **Sonnet** pour coder, **Opus** seulement pour la conception. Au plus **2 fils qui codent** à la fois.
>
> **Décisions métier** (D-069) : ne sollicite pas Florian pour une règle métier. Prends la valeur par défaut prudente et
> note-la dans la doc du lot.
>
> **Avant tout versement** :
> 1. Contre-revue indépendante en lecture seule (agent `review-sonnet`) avec un rapport classé élevé / moyen / faible,
>    versé dans `docs/pilotage/revues/`.
> 2. Élevé : le lot est retenu et une consigne de correctifs est écrite.
> 3. Moyens simples : corrigés par le coordinateur sur la branche du lot, avec tests locaux et clippy.
> 4. Faibles : mis en file.
>
> **Versement sur `ENT`** : par toi seul, en avance rapide uniquement (`git push origin <sha>:refs/heads/ENT`), après
> chacune de ces vérifications :
> - CI verte du lot ;
> - CI d'ENT verte sur la tête précédente : ENT reste **gelé** entre deux versements jusqu'au verdict, annoncé par le
>   commentaire du bot sur la PR 93 ;
> - `merge-base --is-ancestor origin/ENT <sha>` ;
> - `cargo metadata --locked` dans `ent-rust/web`.
>
> **Chaîne** : quand plusieurs lots attendent, tu fusionnes la tête du lot précédent dans le suivant. Les migrations
> versées s'ajoutent à `ent-rust/crates/ent-db/src/migrations_versees.txt` au moment de la fusion de chaîne. Toute
> nouvelle migration va en fin du tableau de `migrate.rs` (règle 38).
>
> **Pilotage écrit** : tout ce qui compte s'écrit sur `ent-mc0hoe` (consignes, rapports, décisions, point quotidien),
> jamais seulement dans un fil.

## 3. Modèle de consigne d'un fil

Les consignes actuelles (`docs/pilotage/consignes/*.md`) servent telles quelles. Leurs rubriques obligatoires :
- **Cadre**
  - À lire d'abord : `CLAUDE.md`, les règles de `docs/PILOTAGE.md` (17, 25, 27, 28, 31, 33, 34, 36, 38), la doc du socle,
    le rapport de revue s'il y en a un.
  - Branche : elle part d'`origin/ENT`, sauf chaîne explicite. PR en brouillon vers `ENT`.
  - Migrations : numéro réservé **vérifié libre sur toutes les branches** ; aucune migration versée modifiée ;
    `SET search_path = pg_catalog, public, pg_temp` et relations `public.` ; fin de tableau ; ne pas toucher
    `migrations_versees.txt`.
  - Interdits : Next.js, `main`, `dev`, pousser sur `ENT`, production, secrets, `cargo fmt --all`, `#[ignore]`,
    extension de `ui.rs`, heure réelle dans les tests.
  - Qualité : clippy 1.99 à zéro avertissement sur les 3 commandes de la CI ; `--locked` ; `cargo metadata --locked`
    dans `ent-rust/web` ; base locale neuve dont le nom contient « test » ; compteurs de la règle 28.
- **Travail** : liste numérotée, cas limites et tests attendus, règle D-069.
- **Fin**
  - CI attendue par **une seule** boucle `until` en arrière-plan ; une CI « queued » longtemps n'est pas une panne ;
    s'il n'y a aucune CI après 10 min, fusionner `origin/ENT`.
  - Section « livraison » dans la doc du socle.
  - Message au coordinateur avec la tête finale et l'état de la CI.

## 4. File au moment de la bascule (à mettre à jour)

1. Correctifs F3 suites 4 s'ils ne sont pas encore versés.
2. **G3b** RGPD : extrait assemblé, masquages, remise chiffrée (migration 2545).
3. **MB1 client** : intégration de `ent-mobile-app` et CI Android / Swift. La conception dit « pas sur `ENT` sans accord » :
   à trancher avec Florian.
4. Transverse : contrôle par une deuxième personne (règle 3) fondé sur la personne physique, pas seulement sur le compte.
   Conception Opus d'abord.
5. Faibles en file, à regrouper en lots de fiabilité :
   - T3 et T2 ;
   - V3b suites 2 (F-1 à F-8, verrou dans les déclencheurs de faits) ;
   - cahier (rejeu 2419/2420, verrou du test de REVOKE, export RGPD des lignes absorbées) ;
   - E1 passe 5 ;
   - I5 passe 7.
6. Socle 27 suivants : **G4** (journal des accès visible), **G5** (violations).

## 5. Ce qu'il faut rebrancher à la bascule

- **Point quotidien** de 18 h 43, heure de Paris (tâche planifiée `trig_012VePETeAZCn5WQqRfp2itw`) : elle vise la
  session du pilote actuel. À arrêter et à recréer dans le Projet, ou à confier au coordinateur.
- **Veille de la PR 93** (commentaires du bot sur la CI d'ENT) : l'abonnement est lié à la session actuelle. Le
  coordinateur doit s'y réabonner.
- Outils locaux du pilote (scratchpad) : `attente.sh` (attente d'une CI par SHA) et PostgreSQL 16 local
  (`/var/tmp/pgpilote`). À recréer dans l'environnement du coordinateur si besoin.

## 6. Premier essai après la bascule

Un lot **isolé et sans risque** (une suite de faibles, sans migration) mené de bout en bout dans le Projet : consigne,
fil, contre-revue, versement, point quotidien. On compare la lisibilité et le coût en contexte avec la méthode actuelle.
On ne bascule tout qu'ensuite.
