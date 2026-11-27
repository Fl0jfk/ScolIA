Tu es une session ScolIA (Sonnet). Réponds en français. Le coordinateur du projet, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au coordinateur et termine.

Ce fichier est la consigne d'un **palier** (règle 39) : le socle 27 **G3b** (extrait assemblé, masquages, remise chiffrée) puis **G4** (journal des accès visible), en **cinq lots** sur **une seule branche**, **sans PR** jusqu'au dernier lot, et **une seule CI**, à la fin.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 21, 23, 25, 27, 28, 31, 33, 34, 35, 36, 38 et **39** (le palier) ;
  - `git show origin/ent-mc0hoe:docs/socles/27_RGPD_ARCHIVES.md` : §0, §4.1–4.2, **§9 en entier**, **§11 en entier**, §12,
    §13.1 et §13.3, les actions **RGP-149 à RGP-166** (§14.3) et **RGP-190 à RGP-199** (§14.6), §15 (R-RG8, R-RG15 à R-RG19,
    R-RG27 à R-RG31), §16, **§17.4 et §17.5**, §18, §20, §21, §23 (S-541 à S-554) et **§26** (l'emporte : R-RG41 à R-RG50,
    RGP-302 à RGP-316, CL27-63 à CL27-70) ;
  - `git show origin/ent-mc0hoe:docs/socles/PERSONAS_27.md` : §0 (bloquants **B** et **E**), P03, P04, P05, P06, P08, P09,
    P11, P14, P16 ;
  - `docs/socles/27_G3A_LIVRAISON.md` (sur `origin/ENT`) : ce qui existe, §5 (règle D-069 du délai), §6 (non fait), §12 ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/27_G3A_CONTRE_REVUE.md` et
    `git show origin/ent-mc0hoe:docs/pilotage/revues/27_G3A_CORRECTIFS_CONTRE_REVUE.md` : faibles **Pf1 à Pf8** et la section
    « Moyens : vérification des correctifs » (lot 1 ci-dessous) ;
  - `git show origin/ent-mc0hoe:docs/PORTABILITE_EXPORTS.md` §7 (contenu, chiffrement, rétention de l'archive) ;
  - `docs/FRONTEND_LEPTOS.md` (avant tout écran) ; `docs/securite/T1_SEARCH_PATH.md` ; `docs/RLS_ISOLATION.md` §3.1.
- **Code existant à réutiliser** (ne rien dupliquer) : `ent-db/src/droits_demandes.rs` (guichet G3a),
  `ent-db/src/extrait_personne.rs` (22 fournisseurs **déclarés**, aucun écrit ; `SectionExtrait`), `ent-db/src/registre_donnees.rs`
  (`export`, `colonnes_hors_export`, `export_si`, `hors_export_par_eleve` : conçus **pour cet assemblage**),
  `ent-db/src/champs_chiffres.rs` (`SecretBox`, AES-256-GCM, dérivation par usage), `ent-pdf` (PDF maison),
  `documents_signature_signer.rs` (défi passkey lié à une empreinte et à une intention, usage unique, 5 min),
  `eleve_confidentialite.rs` (`tracer_consultation_a`, point unique des traces de consultation, 19 appelants),
  `service.rs` (`classes_dans_perimetre`, `exiger_perimetre_prof`), `personnels_missions.rs` (`expliquer_acces_for_actor`).
- **Branche :** crée **`ent-socle-27-palier-g3b-g4`** depuis `origin/ENT` **à jour** (`git fetch origin ENT` juste avant).
  **Aucune PR avant la fin du lot 5.** Le coordinateur peut fusionner ENT dans ta branche chaque matin : au début de
  **chaque** lot, `git fetch origin ent-socle-27-palier-g3b-g4 && git merge --ff-only FETCH_HEAD` (ou fusion normale si tu as
  des commits locaux), puis recalcule les compteurs (règle 28).
- **Migrations :** 2540–2544 sont versées et ne changent **jamais** (règles 15 et 25). Ce palier prend **2545** (lot 1) et
  **2546** (lot 4), vérifiées libres sur les 335 branches distantes le 2026-11-26 ; revérifie avant d'écrire
  (`for b in $(git branch -r); do git ls-tree --name-only $b ent-rust/migrations/; done | grep -E '/254[56]_'` doit être vide).
  Chacune s'ajoute **à la fin** du tableau de `migrate.rs` (règle 38), dans l'ordre 2545 puis 2546. Toute fonction créée porte
  `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.` ; aucune garde sur `PG_CONTEXT` ni
  `set_config`. `MIGRATION_TAG` reste 3808 (2545 et 2546 sont dessous). **`migrations_versees.txt`** : si
  `2544_rgpd_g3_demandes` n'y figure pas encore sur `origin/ENT` à ton départ, ajoute-la à la fin dans **le premier commit**
  de la branche, commit séparé (règle 38, « fusion d'ENT du lot suivant ») ; n'y mets **jamais** 2545 ni 2546.
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ;
  `cargo fmt --all` (le `--all --check` de la CI est permis) ; `#[ignore]` ; modifier `POINT_QUOTIDIEN.md`, `PILOTAGE.md` ou
  les fichiers de conception (`27_RGPD_ARCHIVES.md`, `PERSONAS_27.md`) ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans
  les tests (règle 33) ; **toute nouvelle dépendance** (règle 23, D-062 : pas de crate `zip`, `age` ni autre ; voir lot 3) ;
  sous-agents autres que `review-sonnet` (contre-revue) et `explore-haiku` (recherche).
- **Qualité :** zéro nouvel avertissement rustc et clippy 1.99 (règle 31) ; `--locked` ; fmt ciblé ; `cargo metadata --locked`
  dans `ent-rust/web` (règle 34) ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  compteurs de la règle 28 recalculés à chaque lot et à chaque fusion (`Action::TOUTES`, compte RLS de `tests_rls.rs`,
  inventaires de routes, registre des tables, tableau de `migrate.rs`).
- **Taille (règle 39) :** le palier reste **sous 300 fichiers** modifiés (estimation : 80 à 120). Contrôle à la fin de chaque
  lot : `git diff --stat origin/ENT...HEAD | tail -1`. Si le compte dépasse **220 fichiers** à la fin du lot 4, le lot 5 ne se
  code pas : finis le palier au lot 4 (PR et CI), et écris au coordinateur que le lot 5 part au palier suivant.

## Modèle de menace (ce que chaque lot doit tenir)
- **Le parent en conflit** (P04) : par l'extrait de ses enfants, il cherche l'adresse protégée de l'autre parent, ses
  factures, ses échanges avec le CPE, les métadonnées d'un PDF, ou une trace « données protégées » dans le journal.
- **L'usurpateur** (P11, « Mme X ») : une demande par courriel ou sans compte, avec **sa** propre adresse de remise ; ou un
  **compte de parent volé** qui télécharge l'extrait (CL27-67).
- **L'agent curieux** (P11, Yanis, AED) : consultation hors périmètre, motif « autre », rafale de dossiers la nuit
  (CL27-48), tentative d'obtenir les traces d'un collègue sans contrôle ouvert (R-RG27).
- **La minimisation** : le DPO externe et le secrétariat ne voient que ce que leur rôle justifie (R-RG32, §12.2) ; une
  protection n'est jamais révélée à qui elle vise (R-RG46) ; rien d'un établissement ne fuit vers un autre (404 neutres).

## Travail : cinq lots, dans l'ordre
Conception sur Opus déjà faite : tu codes ce qu'elle prescrit, en déduisant les cas limites (exigence « masterclass »).
**Chaque lot se termine ainsi**, avant le suivant :
(a) tests du lot écrits et verts en local ; (b) **équivalent local de la CI** (règle 39, commandes exactes de
`.github/workflows/ent-rust.yml`) : `cargo fmt --check` ciblé et `--all --check`, les **3 commandes clippy backend**, nextest
**avec et sans labo** sur base neuve (découpe en `--partition count:i/4` si le conteneur coupe les suites longues, F-012 ;
note chaque tranche), et, si le lot touche `web/`, `ent-types` ou `ent-api`, le clippy et les tests du frontend ;
(c) **contre-revue locale** par l'agent `review-sonnet`, en lecture seule, sur `git diff <tête du lot précédent>..HEAD`,
avec ce modèle de menace et la liste des tests du lot ; tout élevé et tout moyen corrigé et testé **dans le lot** ; les faibles
notés dans la livraison ; (d) section du lot ajoutée à la livraison (§ Fin) ; (e) commit(s) poussé(s) sur la branche du
palier (`git push -u origin ent-socle-27-palier-g3b-g4`, aucune PR, aucun `workflow_dispatch`).

### Lot 1 — Extrait assemblé (migration 2545) et faibles G3a du service
1. **Migration 2545** : `rgp_extrait`, `rgp_extrait_section`, `rgp_extrait_masquage` (§17.4), **`rgp_demande_sujet`**
   (§26.6, RGP-302 : déplacée de 2556 vers 2545 parce qu'elle sert l'assemblage ; 2556 garde `cons_visa_exclusion` et
   `cons_categorie.source_ligne`, à noter dans la livraison pour le coordinateur), et une table de **réglages RGPD** de
   l'établissement à colonnes typées (une ligne par établissement, historisée par le journal du 27 ; D-069) qui portera les
   paramètres du palier. `rgp_requisition` n'est **pas** créée (réquisitions = G10, 2553 : pas de table sans usage).
   RLS stricte (`ent_rls_poser`), FK composites, `id UUID`, `cree_le`/`modifie_le`, aucun JSON, tableau ni BYTEA (E1–E14),
   registre des tables et catégorie `rgpd.demande` à jour.
2. **Moteur d'assemblage** (RGP-149, R-RG14, R-RG18) dans un nouveau fichier `ent-db/src/droits_extrait.rs` :
   - sujet **élève** : moteur **générique piloté par le registre** : pour chaque table `export` du domaine d'un fournisseur,
     lignes de l'établissement où `eleve_id` = sujet, sur la période demandée, filtre `export_si` appliqué, colonnes
     `colonnes_hors_export` retirées, tables `hors_export_par_eleve` sautées ; champs chiffrés déchiffrés seulement si la
     catégorie l'autorise, sinon masqués (`FR76 **** 1234`, PORTABILITE §7) ; tables `sante` **exclues** tant que le module
     santé n'est pas actif (R-RG29, R-RG44), listées « exclues : motif » ;
   - sujet **personne** (parent pour lui-même, personnel, ancien élève majeur) : fournisseurs **écrits** pour les comptes
     (socle 1), le dossier des personnes (4), les personnels (6), la messagerie (16 : seulement les messages dont il est
     **auteur ou destinataire**, M-2711) et les documents (7) ; tout autre domaine donne une section **« non couverte »** ;
   - chaque section : socle, fournisseur, statut `ok` / `echec` / `vide` / `non_couverte`, nombre de lignes, **empreinte**
     SHA-256 de son contenu ; plus les **destinataires** (rôles internes, externes, sous-traitants depuis le registre G2),
     **durées** (catégories de conservation) et **sources** (§9.4.3) ;
   - un fournisseur en échec ou une section non couverte rend l'extrait **incomplet** : il ne se remet jamais comme complet ;
     la remise d'un extrait incomplet exige une décision `droits.arbitrer` motivée par section, et la réponse le dit ;
   - le **contenu n'est pas stocké** en base (§17.4) : il est reconstruit à la demande ; si une empreinte diffère entre
     l'assemblage relu et la remise, l'extrait est **périmé** : nouvelle version, masquages à relire ;
   - **fratrie** (RGP-302) : une demande à plusieurs sujets, un extrait par sujet, une réponse commune ; les sujets
     ajoutés passent la même vérification de qualité que le sujet principal (socle 4, à la date de réception) ;
   - **lecture tracée** : toute ouverture du contenu d'un extrait par un gestionnaire ou le DPO (RGP-152) est une
     consultation sensible tracée **avant** d'être servie (R-RG8, `tracer_consultation_a`) ; le DPO ne voit que l'extrait
     d'une demande qu'on lui **soumet** (§12.2).
3. **Faibles G3a qui touchent `droits_demandes.rs`** : **Pf1** plafond des dépôts publics en **réglage d'établissement**
   (défaut actuel conservé) et contrôle verrouillé (pas de dépassement concurrent : verrou consultatif par établissement ou
   compteur `FOR UPDATE`) ; **Pf5** numéro attribué à la vérification réussie, même `a_arbitrer` : comportement conservé et
   écrit dans la livraison ; **Pf6** sélection de la tâche de destruction en `FOR UPDATE SKIP LOCKED` (une seule ligne
   `piece_detruite`), alerte visible de la direction pour une pièce **partagée** restée en attente, et refus pour
   **identité non établie** possible sans la décision d'information des titulaires (rien n'est imputable au mineur) ;
   **Pf8** horloge injectée jusque dans `notifier_personne` ; **effet de bord de M-a** : la même pièce peut être
   référencée par un second essai **sur la même demande** (refus seulement si elle est rattachée à une autre) ;
   **doc rustdoc** de `inserer` remise à sa place ; **remarque de la première revue** : la notification de prolongation ne
   porte plus le motif en texte libre (code de motif et phrase générique).
4. **Tests attendus** (labo, nouveau fichier `tests_rgpd_g3b.rs`) : assemblage d'un élève du jeu fictif (sections, lignes,
   colonnes hors export absentes, `export_si` respecté) ; table de santé exclue et listée ; fournisseur en échec simulé →
   incomplet, remise refusée comme complète ; empreinte changée → extrait périmé ; fratrie à deux sujets dont un hors
   qualité ; cloisonnement (un sujet d'un autre établissement : 404, aucune section) ; DPO sans soumission → 403, avec
   soumission → lecture tracée ; plus un test par faible corrigé. Test de garde : **chaque fournisseur déclaré** a une
   implémentation ou rend « non couverte » (plus aucun `EtatFournisseur::Declare` muet).

### Lot 2 — Masquages et relecture humaine
1. **Marquage automatique, déterministe, jamais l'IA** (R-RG15, R-RG34 ; CL27-01, 02, 18, 20 ; R-RG38, R-RG45 à R-RG48) :
   données propres de l'**autre parent** (coordonnées, payeur et situation financière, échanges) ; **adresse protégée** et
   tout indice de localisation (établissement de la fratrie, transport) ; **autre élève** dans un fait collectif ;
   **rédacteur** d'une note interne ; **signalement** en cours (socle 36, s'il existe des tables) ; pour un **élève
   demandeur**, les données propres de ses parents réduites à leur identité de responsables (R-RG48). Chaque marquage
   écrit une ligne `rgp_extrait_masquage` (`raison`, `propose_par = sys`, élément référencé par table, ligne et colonne).
2. **Pièces** (R-RG45) : une pièce du coffre qui concerne aussi un tiers n'est **jamais** remise ni caviardée
   automatiquement ; elle est listée (« 2 documents concernent aussi une autre personne ; ils ne vous sont pas remis ;
   motif »). Ne sont remises que les pièces déposées par le demandeur ou par le sujet, ou émises **pour** le sujet seul.
3. **Relecture** (RGP-150, RGP-151) : chaque marquage est accepté (masqué) ou levé avec motif par `droits.traiter` ; lever
   un marquage `adresse_protegee` ou `autre_parent` exige `droits.arbitrer` ; aucune remise tant qu'un marquage reste
   non décidé ; ajout manuel d'un masquage, d'une précision ou d'une pièce ; soumission au DPO (RGP-152) et retour.
4. **Faibles G3a liés à la protection** : **Pf3** la qualité `restreinte` ne s'applique plus qu'au demandeur **visé** par
   la protection, ou quand on ne sait pas qui elle vise (prudent) ; la protection d'un autre parent ne bloque plus le
   dépôt en ligne du titulaire, puisque l'extrait la masque désormais ; **Pf4** un porteur de `droits.traiter` sans
   `droits.arbitrer` ne voit que « à arbitrer par la direction », jamais le code `restreinte` / `sans_autorite` (R-RG46).
5. **Écrans Leptos** (fiche demande, `modules/rgpd_droits/`) : section **Extrait** (assembler, état des sections,
   incomplet, périmé), **Relecture** (éléments marqués surlignés, compteur restant, accepter / lever avec motif), **DPO**
   (soumettre, voir l'avis) ; logique pure dans `modele.rs` ; `ui.rs` non étendu.
6. **Tests attendus** : jeu fictif **« parents séparés + adresse protégée »** (§22) : l'extrait du parent 1 ne contient
   aucune donnée propre du parent 2 ni l'adresse protégée, ni dans les sections, ni dans les pièces, ni dans le CSV
   (CL27-01, CL27-02, M-2709) ; messages de la mère avec le CPE absents de l'extrait du père (M-2711) ; élève de 15 ans
   demandeur : pas de situation financière des parents (R-RG48) ; levée d'un marquage protégé refusée au secrétariat ;
   remise refusée tant qu'un marquage est ouvert ; Pf3 et Pf4 ; modèle d'écran (natif) et rendu serveur.

### Lot 3 — Remise, clôture, renvois
1. **Fichiers remis** (§9.4.5, RGP-163, RGP-305, R-RG47) : **PDF** lisible par `ent-pdf` (sections, destinataires, durées,
   sources, liste des exclusions et pièces non remises, masquages rendus « [masqué : autre personne] ») et **CSV** par
   section (écrit à la main, échappement RFC 4180, UTF-8 avec BOM pour les tableurs) ; pièces remises **nettoyées de leurs
   métadonnées** ; documents émis (certificat de scolarité, attestation) **regénérés** sans les champs de tiers quand le
   socle 7 sait les générer, sinon listés et non remis.
2. **Chiffrement sans nouvelle dépendance** (D-062 ; Q-EXP1 non tranchée) : valeur prudente D-069 notée dans la
   livraison : pas d'archive ZIP AES ni `age` ; chaque fichier est **scellé** au repos par `SecretBox` (usage dérivé
   propre, par exemple `extrait-rgpd:v1`, AAD liée à l'extrait, à la version et au fichier), sous un préfixe d'objets
   réservé ; empreintes SHA-256 dans `rgp_extrait` ; servi déchiffré **seulement** au demandeur authentifié, sur TLS ;
   **mise à disposition 7 jours** (réglage), puis objets supprimés par la tâche horaire, suppression confirmée avant
   d'écrire « purgé » (même méthode que E1).
3. **Remise** (RGP-154 ★, R-RG42, CL27-67) : `droits.remettre` avec **passkey récente** du gestionnaire (défi lié à
   l'empreinte de l'extrait et à l'intention « remettre », motif de `documents_signature_signer.rs`) ; **en ligne**
   seulement sur le **compte du demandeur dans cet établissement**, téléchargement derrière une **passkey récente du
   demandeur** (défi lié à l'extrait), lien nominatif court, chaque téléchargement journalisé et compté ; demandeur sans
   compte ou sans passkey : **remise en main propre** au guichet, PDF imprimé, émargement (nom saisi, date, pièce vue,
   gestionnaire) ; **jamais** une adresse ou un numéro fournis par la demande (bloquant B) ; notification
   `rgpd.extrait_pret` sans nom d'élève.
4. **Clôture** (RGP-166) : éléments remis par **empreinte** (jamais les données) ; refus partiels motivés ; renvois
   tracés : **rectification** vers le socle propriétaire (RGP-156, la réponse liste ce qui a été corrigé, « à corriger dans
   la source » pour SIECLE), **retrait de consentement** relié au socle 18 (RGP-159), **CADA d'un tiers** : document agrégé
   ou occulté (RGP-316, L311-6).
5. **Faibles G3a des écrans et de l'API** : **Pf2** seau HTTP par contact **normalisé** (`anti_abus.rs`, même
   normalisation que le service) ; **Pf7** la pièce se choisit parmi les pièces d'identité du coffre de l'établissement
   (ou se dépose depuis la fiche), plus de saisie d'UUID ; pour un dépôt sans compte « pour soi », la vérification
   réussie exige un sujet rapproché qui **est** le demandeur.
6. **Écrans** : fiche demande, section **Réponse et remise** (modèle de réponse, mode de remise, passkey, émargement) ;
   côté demandeur, **Mes données** : « votre extrait est prêt », fichiers, échéance de mise à disposition, téléchargement
   par passkey ; page publique inchangée (aucune confirmation de scolarisation).
7. **Tests attendus** : PDF et CSV produits, sans aucun nom du tiers du jeu fictif (recherche textuelle dans les octets
   rendus) ; métadonnées absentes des fichiers remis ; scellé ouvert seulement avec la bonne AAD (un fichier recopié sur
   un autre extrait ne s'ouvre pas) ; remise sans passkey récente → refus ; téléchargement par un autre compte → 404 ;
   adresse fournie par la demande jamais utilisée ; purge à 7 jours avec un jour **injecté** ; empreintes à la clôture ;
   Pf2 et Pf7 ; HTTP (401, 403, 404, 409) ; rendu serveur des nouvelles pages (CSP à nonce, aucune donnée
   d'établissement dans le HTML du serveur).

### Lot 4 — Traces d'accès séparées (migration 2546) et page « Consultations du dossier »
1. **Migration 2546** : `acc_trace` **partitionnée par mois** sur `le` (§11.6, §17.5, R-RG30, R-RG31), partitions créées à
   l'avance par une fonction (mois courant et deux suivants), RLS stricte sur le parent **et** sur chaque partition,
   ajout seul (ni `UPDATE`, ni `DELETE`, ni `TRUNCATE` hors procédure de purge) ; `acc_controle`, `acc_alerte` ; un
   petit registre des purges de traces (`cons_execution` n'existe qu'en G6, 2549 : à reprendre alors) ; colonnes de
   réglages ajoutées à la table de réglages de 2545 (S-541 durée 12 mois, S-542 motifs, S-543 et S-554 seuils, S-549 nom
   des agents : non). **Reprise** des traces de consultation existantes d'`ent_journal_modif` par copie, sans rien
   supprimer du journal (2559 reste réservée). Premier usage du partitionnement dans ScolIA : adapte les gardes
   (registre des tables, RLS stricte, `garde_search_path`, colonnes JSON, neutralisation) pour qu'une partition soit
   couverte par son parent, **avec un test** ; aucune garde affaiblie pour les autres tables.
2. **Écriture** : `tracer_consultation_a` écrit aussi `acc_trace` **dans la même transaction** (R-RG8 : échec = lecture
   refusée), fonction et service de l'acteur **figés** (R-RG31), nature, section, sujet ; le journal actuel continue
   d'être écrit (double écriture jusqu'à la décision 2559).
3. **Hors périmètre** (RGP-191, RGP-310, R-RG49) : calculé avec le moteur de périmètre existant (classes, groupes,
   missions, service) et les **périmètres temporaires** (remplacement, conseil de classe, permanence, sortie) ; une
   consultation hors périmètre de la fiche élève demande un **motif** de la liste S-542 en un geste (l'API répond
   « motif requis », l'écran propose la liste, puis la lecture est servie et tracée avec le motif) ; un geste couvert par
   un périmètre temporaire n'est pas « hors périmètre ».
4. **Page « Consultations du dossier »** (RGP-192, §11.2, §11.4) : famille (pour chaque enfant dont elle est titulaire à
   la date), élève majeur, personnel (son propre dossier) ; date, **fonction et service** (jamais le nom, §11.3, S-549),
   nature, section, motif ; regroupée par jour ; année en cours et précédente. **Jamais** : consultations de santé ou de
   suivi psy (R-RG29), consultations différées (R-RG28), le support d'éditeur sans mention (« Support ScolIA,
   intervention autorisée le … »), une section protégée autrement que sous un **code générique**, et **jamais** au parent
   visé par la protection (R-RG46).
5. **Tests attendus** : trace écrite dans la transaction de lecture, échec de trace = lecture refusée (déclencheur de test
   qui refuse l'insertion) ; fonction figée après changement de poste ; partition du mois suivant créée, RLS active sur
   la partition interrogée directement ; reprise des anciennes traces (nombre égal) ; remplaçant dans son périmètre
   temporaire : pas de motif demandé ; hors périmètre : motif exigé ; parent séparé visé par une protection : ne voit
   pas la trace « coordonnées protégées » ; aucune trace de santé ; cloisonnement ; aucun test à l'heure réelle (mois
   et jour injectés).

### Lot 5 — Contrôles internes, alertes, purge, traces dans l'extrait
1. **Actions** (D-017, règle 21 : seule la fonction `socle_27_rgpd` d'`actions.rs`) : `acces.consulter_traces` (Direction,
   DPO) et `acces.controle_ouvrir` ★ (Direction, passkey récente) ; `Action::TOUTES` **286 → 288** (à recalculer si ENT a
   bougé). Sortie de `modele_par_defaut` comparée pour les 12 rôles avant et après (aucun autre changement).
2. **Traces d'un sujet** (RGP-193) seulement dans le cadre d'une demande ouverte ou d'un contrôle ouvert, la lecture
   étant elle-même tracée ; **contrôle interne** sur un agent (RGP-194, RGP-195, R-RG27) : motif, durée bornée, clôture,
   aucune statistique individuelle ni classement « qui consulte le plus » (§11.4) ; **différer** l'affichage d'une
   consultation liée à un signalement (RGP-197, R-RG28) sous `droits.arbitrer`, motivé, tracé, borné.
3. **Alertes** (RGP-196, §11.5, CL27-48) : règles déterministes (rafale hors périmètre S-543, seuils par rôle S-554,
   élève à adresse protégée consulté par un compte sans lien, consultation en conservation intermédiaire ou d'archives) ;
   `acc_alerte`, notification `acces.alerte` à la direction et au DPO, **jamais** à l'agent concerné ; une alerte par
   fenêtre (idempotente) ; suite tracée (`rien`, `violation`, `controle`).
4. **Purge** (RGP-198, S-541, CL27-53) : suppression de partition **par procédure** journalisée, jamais un `DELETE`
   applicatif ; une partition qui contient des traces d'un sujet visé par une **demande ouverte** n'est pas purgée
   (purge reportée, alerte au DPO).
5. **Dans l'extrait** (RGP-199, RGP-165) : section « consultations » de l'extrait du sujet (mêmes règles que la page du
   lot 4) ; nom d'un agent communiqué seulement sur arbitrage motivé de la direction (`droits.arbitrer`, §11.3, CL27-16).
6. **Écrans** : réglages RGPD de l'établissement (direction : paramètres du palier avec leur valeur par défaut et
   l'historique) ; traces d'un sujet (dans la fiche demande) ; contrôles internes ; alertes.
7. **Tests attendus** : traces d'un agent sans contrôle ouvert → 403 ; contrôle ouvert sans passkey récente → refus ;
   rafale du jeu fictif → une alerte à la direction et au DPO, aucune à l'agent ; seuil AED distinct ; purge à 12 mois
   avec mois injecté, partition gelée par une demande ouverte conservée ; section « consultations » de l'extrait sans
   santé ni trace différée ; nom d'agent absent sans arbitrage, présent après ; droits par défaut (12 rôles) ; HTTP et
   rendu serveur.

### Règles métier non tranchées
Toute règle non tranchée prend une **valeur par défaut prudente**, réglable par la direction quand c'est un choix local,
et notée dans la livraison (D-069). Ne sollicite pas Florian. Déjà fixées par cette consigne : chiffrement par scellé
`SecretBox` sans archive ZIP (Q-EXP1 ouverte) ; mise à disposition 7 jours ; remise en ligne seulement au compte du
demandeur avec passkey récente ; santé exclue de l'extrait tant que le module est inactif ; pièce de tiers jamais remise ;
nom des agents jamais affiché sans arbitrage ; purge reportée tant qu'une demande ouverte concerne la partition ; S-555
(la vérification d'identité ne suspend pas le délai) inchangé, à reporter au §23 par le coordinateur.

## Fin
- **Avant d'ouvrir la PR** (fin du lot 5, ou du lot 4 si la taille l'impose) : fusionne `origin/ENT` (règle 30, compteurs
  de la règle 28 recalculés depuis `git merge-base`), repasse l'équivalent local de la CI complet, et termine
  **`docs/socles/27_G3B_G4_LIVRAISON.md`** : par lot, fait / non fait, actions couvertes, cas limites, tests, faibles de la
  contre-revue locale, valeurs prudentes D-069, compteurs, taille du diff ; et pour le coordinateur : 2556 allégée de
  `rgp_demande_sujet`, S-555 à reporter, tables partitionnées et gardes adaptées. La livraison part **dans le dernier
  commit avant la PR**.
- Ouvre la PR **en brouillon** vers `ENT` (règle 35), corps : lots, migrations 2545 et 2546, compteurs, lien vers la
  livraison.
- **Après l'ouverture de la PR, plus aucun commit hors correctif** (règle 39) : aucun commit de doc, même pour noter la CI.
- Attends la CI par **une seule** commande `until` en arrière-plan sur
  `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause, regroupe les corrections, un seul nouveau push. Un job « runner lost communication » se
    relance une fois.
  - **Aucune CI au bout de 10 min** : vérifie `mergeable` ; la PR est en conflit avec `ENT` : fusionne `origin/ENT`
    (règle 28) et repousse.
  - Une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au coordinateur.
- Écris ensuite au coordinateur : numéro de PR, tête finale, état de la CI (numéro du run, jobs), taille du diff, et les
  faibles restés en file. C'est lui qui note la CI dans la doc, sur `ent-mc0hoe`.
