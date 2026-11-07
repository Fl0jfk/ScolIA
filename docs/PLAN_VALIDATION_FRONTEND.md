# Plan de validation du frontend Leptos

Date : 2026-11-02 — Statut : **plan**, rien n'est encore exécuté (sauf mention).
Objet : compléter les tests du prototype d'appel (`docs/LEPTOS_CALL_PROTOTYPE.md` §7) avant de reconstruire d'autres modules. Les mêmes contrôles s'appliqueront ensuite à chaque nouvel écran.

## 0. Déjà fait (prototype)
Tests automatisés verts ; navigateur Chrome (pane de l'application + Chrome sans fenêtre) ; axe-core 0 violation ; mobile 375 px ; mesures à processeur normal et ralenti ×4. **Non fait** : tout ce qui suit.

## 1. Lecteurs d'écran

| Combinaison | Pourquoi | Qui / où |
|---|---|---|
| **NVDA** (gratuit) + Firefox, Windows | combinaison la plus courante en France sur PC | poste Windows de l'établissement ou de développement |
| **NVDA** + Chrome / Edge, Windows | navigateurs des postes scolaires | idem |
| **VoiceOver** + Safari, macOS | utilisateurs Mac | un Mac (prêt ou personnel) |
| **VoiceOver** + Safari, iPhone / iPad | familles et professeurs en mobilité | un iPhone ou un iPad |
| (option) **TalkBack** + Chrome, Android | familles Android | un téléphone Android |

Scénarios (à dérouler pour l'écran d'appel, puis pour chaque écran) :
1. Connexion : champs annoncés avec leur étiquette, erreur « mot de passe incorrect » annoncée.
2. Arrivée sur l'appel : lien d'évitement, titre de page, région « Choix de la séance ».
3. Choix de la séance, puis navigation dans le tableau (titres de colonnes lus, nom de l'élève lu avec sa ligne).
4. Changement de statut (souris, clavier, raccourcis P/A/R) : nouveau statut annoncé.
5. Recherche et filtres : nombre de résultats perceptible ; état « pressé » des filtres annoncé.
6. Sélection multiple + action groupée : nombre de lignes concernées et sélections masquées annoncés.
7. Dialogue de confirmation : focus à l'ouverture, contenu lu, Échap, retour du focus.
8. Validation : message de succès annoncé ; feuille en lecture seule compréhensible.
9. Erreurs : session expirée, perte du réseau, échec partiel.

Résultat attendu : chaque étape réalisable **sans la vue**, sans blocage ni information manquante. Consigner les défauts (étape, lecteur, navigateur, gravité).

## 2. Navigateurs

| Navigateur | Version cible | Points à vérifier |
|---|---|---|
| Chrome / Edge (Chromium) | 2 dernières versions | déjà couvert par le prototype |
| **Firefox** | dernière + **ESR** (souvent déployée dans les établissements) | WebAssembly, `<dialog>`, champs `type=date`, styles des radios |
| **Safari** | macOS et iOS récents | `<dialog>`, date, compression brotli, défilement de la barre collante, zoom à 200 % |

Pour chaque navigateur : parcours complet de l'appel, console sans erreur (hydratation comprise), mesure du temps d'affichage (même script que le prototype quand le navigateur le permet), zoom 200 % sans perte d'information, mode « contraste élevé » de Windows.

## 3. Poste scolaire réel

- Relever : processeur, mémoire, version de Windows et du navigateur, extensions imposées, **proxy / filtrage** de l'établissement, débit réel.
- Dérouler le parcours d'appel complet et mesurer : premier chargement (cache vide), chargement suivant (cache rempli), affichage de 38 élèves, clic → affichage, recherche.
- Vérifier que le filtrage de l'établissement laisse passer le WebAssembly (`application/wasm`) et ne décompresse pas les réponses (constat du prototype : un intermédiaire sur le poste de développement décompressait le trafic des navigateurs).
- Tester un poste partagé : connexion, déconnexion, connexion d'un second utilisateur, passkey par QR code depuis un téléphone (après le lot passkeys).
- Seuils indicatifs : page utilisable en < 3 s au premier chargement sur le réseau de l'établissement ; clic → affichage < 100 ms pour une classe.

## 4. Audit RGAA

- Référentiel : **RGAA 4.1** (106 critères), obligatoire pour les services publics et attendu des établissements.
- Échantillon : connexion, appel (choix de séance, feuille, confirmation, résultat), page d'erreur, plus chaque nouveau module.
- Méthode : grille RGAA critère par critère (outils : axe-core, inspecteur du navigateur, lecteurs d'écran du §1, contrôle manuel des contrastes, navigation clavier, zoom, désactivation des styles).
- Livrables : taux de conformité, liste des non-conformités avec correction proposée, puis **déclaration d'accessibilité** et schéma pluriannuel quand l'ENT sera en service.
- Recommandation : un premier audit interne sur le prototype, puis un **audit externe** par un auditeur RGAA avant la mise en service.

## 5. CSP avec nonce

Constat dans le code de Leptos 0.8.23 : `leptos::nonce::provide_nonce()` génère un nonce aléatoire par requête et `HydrationScripts` l'ajoute automatiquement au script d'hydratation en ligne ; `use_nonce()` le rend disponible pour d'autres balises.

Mise en place proposée (dans `ent-web-app` / `ent-web-server`, aucune modification d'`ent-api`) :
1. appeler `provide_nonce()` au début du rendu serveur (coquille) ;
2. envoyer l'en-tête correspondant via les options de réponse de `leptos_axum` :
   `Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'nonce-<valeur>'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-src 'self'; frame-ancestors 'self'; base-uri 'none'; form-action 'self'; object-src 'none'`
3. d'abord en mode **observation** (`Content-Security-Policy-Report-Only`, avec un point de collecte des violations) pendant les tests, puis en mode bloquant.

Contrôles :
- aucune violation dans la console (Chrome, Firefox, Safari) sur tous les parcours ;
- nonce **différent à chaque requête** (deux chargements → deux valeurs) ;
- un script injecté sans nonce est bien **bloqué** (test volontaire) ;
- l'iframe ArtCraft (D-006) reste autorisée (`frame-src 'self'`) quand elle sera intégrée ;
- l'ancienne interface `/app` n'est **pas** concernée (son JavaScript en ligne demanderait `'unsafe-inline'`) : la CSP stricte s'applique aux seules pages `/web`.

## 6. Ordre proposé
1. CSP en mode observation (petit changement dans le prototype, testable tout de suite).
2. NVDA + Firefox et Chrome sur le prototype ; corrections.
3. Firefox ESR et Safari.
4. Poste scolaire réel.
5. Audit RGAA interne ; audit externe avant mise en service.
