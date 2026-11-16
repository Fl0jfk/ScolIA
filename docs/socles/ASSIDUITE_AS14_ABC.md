# Socle 12 — lots AS14a, AS14b, AS14c (écrans web) : appel, journée de la vie scolaire, famille et élève

**Date :** 2026-11-16 — **Branche :** `ent-socle-12-code-as14` (depuis `origin/ENT`, fusionné à `45ad45eb` juste avant la CI) — **Niveau :** implémenté + testé en local (PostgreSQL 16, Chromium) ; CI : voir le journal du pilote ; **non validé par Florian, pas prêt pilote.**
**Conception :** `12_ASSIDUITE.md` §20 (écrans clés 1, 3, 4, 9, 10), §24.1 (ligne AS14), §24.2 (démonstration de décembre), AS-216, AS-249 (sur `origin/ent-mc0hoe`). **Aucune migration** ; **aucune nouvelle action** (`TOUTES` inchangé) ; aucune nouvelle source de routes (la route ajoutée est dans `assiduite.rs`, déjà inventorié). Règle 28 relue : aucun compteur touché.

## 1. Ce qui est fait

### AS14a — appel web (`/web/appel`, module `appel`)
- Feuille d'appel **v2** : ouverture idempotente, lecture pré-remplie (`/appels/{id}/v2` : situations connues, libellés courts « prévenu », « en sortie »… **sans motif**), saisie présent / absent / retard (minutes), raccourcis P / A / R, sélection multiple (lot sur lignes affichées seulement), recherche et filtres.
- « **Appel fait** » : envoi **des seules lignes modifiées**, validation, confirmation récapitulative ; en mode « présents seulement », les non pointés sont listés et exigent la confirmation ; échec partiel : l'appel n'est **pas** validé, échecs listés.
- **Corrections** après l'appel : motif codé + précision (200 car.), version lue envoyée (conflit signalé), refus traduit (intervenant le jour même, vie scolaire ensuite) ; **historique** de la ligne ; incohérences après l'appel.
- **Séance non assurée** : appel neutralisé affiché (absences retirées, démentis émis par le serveur), lecture seule.
- États jamais portés par la seule couleur (libellé complet lu par le lecteur d'écran : « Léa Martin, absent, prévenu »).

### AS14b — vie scolaire (`/web/assiduite`, `/web/assiduite/accueil`, module `assiduite_vs`)
- **File « à qualifier »** (trois mois), **justificatifs** à examiner, détail d'un épisode (séances, textes déclarés, justificatifs, propositions des AED, historique), **qualification** individuelle (version lue, nature L131-8 pour « motif légitime », libellé famille cohérent, note interne), **qualification en masse** avec **aperçu puis confirmation par empreinte** (200 max).
- **Avis aux familles** (AS5) : par élève, prévus / en cours / partis / annulés / **démentis**, avec la cause ; **file « sans réponse »** (relance graduée J+5, J+8).
- **Accueil (AS-216)** : recherche d'un élève → « en sortie », « prévenu » (heure d'arrivée prévue), localisé, « absent ce matin », **sans motif** ; déclaration réutilisant le dialogue commun ; mention « consultation enregistrée ».
- **Seul ajout backend** : `GET /assiduite/eleves/{id}/situation-accueil` (`absences.declarer`, un élève à la fois, **lecture journalisée** par l'événement `assiduite.situation_accueil_lue`, mêmes réponses pour « inexistant » et « interdit » ; `ent-db/src/assiduite_as14.rs`). Cas de séparation ajouté (`assiduite-situation-accueil`).

### AS14c — famille et élève (`/web/absences`, module `absences`)
- Absences **par épisode** (libellé à liste fermée, jamais la note interne), « **Donner le motif** » (motifs visibles pour la famille + précision → justificatif texte), retards, échanges reçus de la vie scolaire (contacts, courriers postés, sans auteur ni note). Vue **élève** en lecture (AS-249) ; seul le responsable ayant le droit propose « Donner le motif ».
- Remplace la liste d'absences de l'ancien circuit (`/eleves/{id}/absences`, `justifier`) dans l'écran.

### Transverse
- `ent-types::assiduite` (DTO JSON identiques à `ent-db`), `ent-web-api::assiduite` (client typé), navigation (« Assiduité », « Accueil : élève », selon le moteur de droits), `docs/FRONTEND_LEPTOS.md` §14.
- Correctifs d'accessibilité du design system trouvés par axe-core : zone de notifications (`ol role=status` → `div role=status` + `ul`), jeton `--c-ok` (contraste 4,47 → 4,98), onglets à 375 px.

## 2. Non fait / écarts (à ne pas masquer)
1. **Dépôt d'une pièce** (photo du mot, certificat) côté web : le serveur sait la recevoir, l'écran ne l'offre pas (texte seulement). Pas de lecture de pièce côté famille.
2. **Vue « mes épisodes » du professeur** (R-AS21) non affichée : la route ne rend pas les noms ; à enrichir côté backend.
3. **AS-250** (missions de délégué « instance ») : aucune route ; non fait. AS-249 : « avis différés pendant ses cours » (R-AS39) est une règle serveur, rien de propre à l'écran.
4. **Pas de brouillon serveur** de l'appel : la saisie est locale jusqu'à « Appel fait » (fermer l'onglet la perd) ; l'enregistrement local et le hors ligne sont AS10.
5. **Séance non assurée** : affichée, mais non déclenchable depuis l'appel (le serveur la pose à l'annulation de séance par l'emploi du temps) ; non jouée dans le navigateur (couverte côté serveur par AS2).
6. **Avis reçus par la famille** : l'écran montre les échanges (`communications`) ; les avis d'absence eux-mêmes passent par les notifications (pas de route d'historique côté famille).
7. **Requalification, relances manuelles, contacts, courriers, signalements, dossier annuel** (AS14d à AS14f) : hors périmètre. Mobile : socle 11.
8. L'état « ouvert » d'un épisode (appel fait, décision attendue) est libellé « Ouvert, à qualifier » ; le détail ne rend pas le nom de l'élève (repris de la file).
9. Un retard sans minutes est accepté par le serveur (calculé depuis le début de la séance) ; l'écran, lui, **demande** les minutes.
10. **Lecteur d'écran** : non testé avec un vrai lecteur (axe-core ne suffit pas, `FRONTEND_LEPTOS.md` §6) ; à faire avant de déclarer les écrans accessibles.

## 3. Vérifications (locales, 2026-11-16)
- **rustc / clippy 1.99.0, `-D warnings`, zéro avertissement** : frontend (SSR, serveur, serveur+labo, wasm `hydrate`) et backend (`ent-types`, `ent-db`, `ent-api`, `--all-targets`, labo). Formatage : `cargo fmt -p` (jamais `--all`).
- **Tests** : `ent-web-ui` 14, `ent-web-api` 29, `ent-web-app` **459** (dont modèles d'appel v2, qualification, accueil, épisodes famille, navigation) ; **contre le vrai backend** `assiduite_as14` (4) + `vie_scolaire_s2` (3) + `droits_parite` (21) + `entetes_securite` (3) ; **séparation** `ent-api` : 50 verts (dont `assiduite-situation-accueil` et le « double membre » C6). Aucun `#[ignore]`.
- **Permissions vérifiées** : professeur refusé sur épisodes, justificatifs, avis, accueil ; autre parent refusé et **même réponse** que pour un identifiant inexistant (R-AS35) ; correction du professeur refusée hors du jour même ; version périmée → conflit ; qualification en masse refusée sans l'empreinte de l'aperçu ; accueil : identifiant inconnu = introuvable.
- **Navigateur (Chromium + Playwright, vrai serveur, base de labo)** : parcours *prof* (appel au clavier : Harry absent, Hermione 12 min de retard → « Appel fait » → correction → historique) → *famille* (voit l'absence, « Donner le motif ») → *vie scolaire* (file, détail, qualification, avis) → *accueil* (situation) → *élève*. **axe-core (wcag2a/aa, 21, best-practice) : 0 violation sur 13 vues**, y compris à **375 px** ; aucun débordement horizontal à 375 px.
- Captures : `docs/socles/captures_as14/` (21 images : 01–08 appel, 10–13 famille, 20–25 vie scolaire, 30–31 accueil, 40 élève).
- Environnement : PostgreSQL 16 local, `ENT_ENV=lab`. Pour lier l'exécutable en local il a fallu compiler les dépendances C sans informations de débogage (`CFLAGS=-g0`, `CARGO_PROFILE_DEV_DEBUG=0`) : limite de l'éditeur de liens, sans effet sur le code.
- CI : une seule exécution (`workflow_dispatch` `ent-rust.yml`) sur la tête de la branche ; numéro de run communiqué au pilote.
