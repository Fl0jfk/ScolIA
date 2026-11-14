# Mission — corriger les failles critiques et élevées du Next.js en production (D-053)

**Décision** : D-053 (Florian, 2026-11-11). Lis-la dans `docs/DECISIONS.md`, ainsi que `docs/ALERTES_NEXTJS_PROD.md` (registre des failles NJ-xx).

## Ce que tu fais
Tu corriges, **dans le code Next.js de `main`**, les failles **critiques** (section 1 du registre) puis **élevées** (section 2 et ajouts NJ-76, NJ-78, NJ-79). Tu procèdes par **petits lots**, un module ou une famille de failles par lot, dans cet ordre :
1. NJ-01, NJ-03 : IA en arrière-plan sur les données d'élèves (pilotage, bien-être). Ce sont des changements de comportement : tu **prépares** le lot, puis tu demandes au pilote avant de le livrer.
2. NJ-04 et toutes les failles « tout compte connecté lit/modifie » : contrôles de rôle et d'établissement côté serveur.
3. Données de santé (NJ-14 à NJ-18) : réservées aux rôles habilités.
4. Voyages (NJ-07 à NJ-13, NJ-28), stages, pièces jointes et URL signées (NJ-29, NJ-39), NJ-02 (liens OneDrive : arrêter d'en créer, côté code).
5. Les autres élevées, NJ-76, NJ-78, NJ-79.

## Comment
- **Travaille dans un worktree** partant de `origin/main` (`git worktree add ../nx -b nextjs-correctifs-securite origin/main`). Pousse **uniquement** sur la branche `nextjs-correctifs-securite` : un commit par lot, avec un message qui cite les NJ-xx fermés. **Ne pousse jamais sur `main`** : c'est le pilote qui relit et pousse.
- Après chaque lot, préviens le pilote par `send_message` (session `session_01KZCbFEY2KWGpFMKfofkJxT`). Donne-lui le SHA, les NJ fermés, les fichiers touchés, ce qui change pour les utilisateurs et tes vérifications. Puis enchaîne le lot suivant, partant de ta branche (le pilote la rebase si besoin).
- **Vérifications avant chaque envoi** (dans le worktree) : `npm ci`, `npm run typecheck`, `npm run lint`, et les tests `test:*` qui ne demandent pas de base (pas les `:db`). Ajoute un petit test `tsx --test` quand c'est simple, par exemple pour une fonction de contrôle d'accès.
- **Correction minimale** : ajoute le contrôle qui manque (session, rôle, établissement, propriétaire). Ne refactore pas, ne reformate pas les fichiers, ne renomme rien.

## Interdits (en plus de CLAUDE.md)
- Aucune migration Drizzle et aucun changement de `db/schema*` : une faille qui l'exigerait est signalée au pilote, pas corrigée.
- Aucun accès à la base ni aux secrets ; ne lis aucun fichier `.env*`. Ne touche ni `package.json` ni le lockfile, ni `.github/`, ni Dockerfile, ni configuration de déploiement.
- **Secrets de repli codés en dur** (NJ-03, NJ-41, NJ-48, NJ-52) : ne les retire pas sans accord, car si la variable manque en production, la fonction casse. Signale-les au pilote, qui fera vérifier par Florian.
- Tout changement que les familles ou le personnel verraient (page publique fermée, IA coupée, nouveau parcours, message d'erreur à la place d'une donnée) : prépare-le, décris-le et **attends l'accord** transmis par le pilote.
- Ne modifie pas `ALERTES_NEXTJS_PROD.md` : le pilote tient les statuts à jour.

## Fin
Quand les critiques et les élevées sont fermées ou signalées, envoie au pilote un bilan : NJ fermés, NJ en attente d'accord, NJ impossibles sans migration ou sans comptes parents.
