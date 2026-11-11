# Socle 11 — Applications mobiles (coquille commune)

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031), listées au §29. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js a été **lue en lecture seule ciblée** (trois routes `app/api/mobile/*`), rien n'y a été touché.
**Branche :** `ent-socle-11-conception` (depuis `origin/ent-mc0hoe`, `afeb8c4`). Code évalué : `ent-mc0hoe` (serveur, 231 migrations jusqu'à 642, 4 prototypes `ent-rust/apps/`) et les branches non fusionnées `origin/ent-mobile` (`746540c`) et `origin/ent-mobile-app` (`5da17f4`, la plus avancée), lues par `git show`.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (**D-003**, **D-009**, **D-011**, **D-012**, **D-013**, D-017, D-018, D-021, D-024, D-026, D-029, D-031, D-034, D-035, **D-036**, D-037 à D-041) ; `docs/SOCLES.md` §11 ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/MOBILE.md` de la branche `ent-mobile-app` ; les conceptions qui attendent quelque chose du mobile : `08_NOTIFICATIONS.md` (N7, `ent_ntf_appareil`, R-N57), `09_INTEROP.md` §22 (fédération sur mobile), `12_ASSIDUITE.md` §9.4 (appel hors ligne, R-AS9 à R-AS11, R-AS32, R-AS38), `14_ACCUEIL_SURETE.md` §16 (appareils, R-AS11, R-AS12), `29_PORTAILS.md` §5.1, §11, §14.3, R-PO33, R-PO34, `34_SANTE.md` (fiche d'urgence hors ligne), `41_INTERNAT.md` (appel de nuit hors ligne), `42_SORTIES_VOYAGES.md` §24 point 5 et CV-40, `04_ADMINISTRATIF.md` R-A10, `16_MESSAGERIE.md` (MSG-151, 152), `20`, `21`, `22`, `06_PERSONNELS.md` (casquettes, P12).
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée) ; **[V~]** texte officiel connu par un extrait ou par une conception déjà vérifiée du dépôt (source citée) ; **[NV]** non vérifié ; **[H]** hypothèse ; **[R]** recommandation (non décidée). Identifiants propres au socle : actions **MOB-100** et suivantes, règles **R-MO1**, cas limites **CMO-01**, défauts de l'existant **DS-** (serveur), **DA-** (appli), **DX-** (Next.js, signalés seulement), questions **Q-MO1**, réglages signalés **S-1101**, manques de la passe personas **M-1100** (`PERSONAS_11.md`).
**Plage de migrations imposée : 2220–2239** (§26, inscrite dans `MIGRATIONS.md`).

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 La journée et l'année vues du téléphone · §5 Réglementaire · §6 Frontières et ports · §7 Concepts · §8 Architecture des applis · §9 Rattachement, connexion, session · §10 Contextes : établissements, casquettes, enfants · §11 La coquille et ses écrans · §12 Hors ligne : baux, profils, opérations · §13 Notifications reçues · §14 Sécurité de l'appareil · §15 Confidentialité et minimisation · §16 Élèves, téléphone à l'école, mineurs · §17 Appareils partagés et appareils de l'établissement · §18 Accessibilité, langues, ergonomie · §19 Distribution, versions, support · §20 IA · §21 Actions · §22 Règles · §23 Cas limites · §24 Droits · §25 Événements · §26 Modèle de données et migrations · §27 Lots · §28 Tests et recette · §29 Décisions appliquées, réglages, questions · §30 Faits, hypothèses, contradictions · §31 Sources · §32 Compléments de la passe personas.

---

## 0. En bref

1. **Le socle 11 n'a pas d'écran métier.** Il livre la **coquille** commune aux deux applis (une iOS en SwiftUI, une Android en Compose, D-003) et le **kit mobile** sur lequel chaque socle branche ses écrans quotidiens : connexion par passkey native, session longue liée à l'appareil, rattachement à un ou plusieurs établissements, contextes (établissement × casquette × enfant), manifeste d'écrans calculé **par le serveur**, réception des notifications, **moteur hors ligne borné** (baux, purge, opérations différées), appareils partagés, accessibilité, publication et support. La règle de D-003 tient en une phrase : **le téléphone n'est jamais une source de droit ; il affiche ce que l'API autorise et oublie ce qu'il n'a plus le droit de garder.**
2. **L'existant est sérieux mais non intégré.** La branche `ent-mobile-app` a déjà fusionné les quatre prototypes en deux applis uniques (cœurs Kotlin et Swift testés : 34 + 31 tests, Android compilé en debug et release, 8 tests Compose ; iOS jamais compilé faute de Mac) avec de bons réflexes (jeton au Keystore / Keychain `ThisDeviceOnly`, aucune redirection suivie, écrans selon les droits D-017, confirmation avant clôture d'appel et envoi). Elle n'est fusionnée nulle part. Les quatre prototypes (5 625 lignes, sans projet de build, jeton en mémoire, `http` par défaut) sont encore dans `ent-mc0hoe`.
3. **Sept défauts à corriger tout de suite (lot MO0)** : (DS1) **révoquer une session ne coupe pas les notifications** du téléphone perdu — `ent_push_device` n'a pas de `session_id` et la révocation ne touche que `ent_session` ; (DS2) le **type de client est déclaré par le client** : n'importe quel script obtient une session de 90 jours en écrivant `"client":"mobile"` ; (DS3) jeton push **en clair**, **renvoyé** dans la réponse, réaffecté au dernier compte sans trace ; (DS4) aucune désinscription push ; (DS5) jetons d'appareils **exportés** à la portabilité et **non neutralisés** à la restauration ; (DA1) le **cache hors ligne de l'appli n'expire jamais** (notes, messages, EDT), contre R-PO33 et R-AS32 ; (DA2) l'appli Android cible l'API 35 alors que Google Play exige l'**API 36** pour toute mise à jour depuis le 31/08/2026 [V~]. S'y ajoutent deux **failles probables de l'API mobile Next.js** (DX1, DX2), signalées à Florian sans y toucher.
4. **Deux textes de 2026 changent la conception** : la **loi du 24 août 2026** étend au **lycée** l'interdiction du téléphone pendant le temps scolaire (L511-5, déjà en vigueur au collège, « portable en pause » généralisé en 2025) [V~] : l'appli élève est pensée **hors du temps scolaire** et ne sollicite jamais un élève pendant ses cours (R-MO22) ; l'**arrêté du 7 septembre 2026** impose aux ENT publics une **plage calme** (20 h–7 h, week-end), le **RGAA applis mobiles comprises** et la mesure d'audience par le **DNMA** [V~, `29_PORTAILS.md` §5.1].
5. **Le hors ligne est un contrat, pas un cache.** Chaque socle déclare ses **profils hors ligne** (ce qui part sur l'appareil, pour combien de temps, ce qui l'efface) ; le serveur émet un **bail** daté pour chaque lot de données confié à un appareil et le **consigne** : en cas de perte ou de vol, la direction et le DPO savent **exactement** quelles données étaient sur ce téléphone (art. 33 RGPD), et l'ordre de purge détruit la clé qui les chiffre. Les gestes faits hors ligne (appel, comptage, remise, positionnement) sont des **opérations signées par la clé de l'appareil**, idempotentes, relues contre les droits à la réception et confiées au socle propriétaire pour les conflits.
6. **Ce qui fera la différence** [H] : appel au gymnase sans réseau sans y penser ; une seule appli pour le professeur qui est aussi parent d'élève dans son établissement et enseigne dans un deuxième ; « Aujourd'hui » réuni sur l'appareil pour la famille de deux établissements, sans qu'aucun serveur ne voie la réunion ; téléphone perdu révoqué en un geste **avec la liste de ce qu'il contenait** ; aucun SDK tiers, aucune mesure d'audience privée, aucune donnée sensible sur l'écran verrouillé.
7. **Chiffres** : **186 actions** (MOB-100 à MOB-285, dont 30 ajoutées par la passe personas), **46 règles** (R-MO1 à R-MO46), **74 cas limites** (CMO-01 à CMO-74), **15 lots** (MO0 à MO14) ; 13 tables nouvelles (dont une de plateforme), **aucune colonne JSON** ; migrations **2220–2239**. Parcours minimal vendable : MO0, MO1, MO2, MO3, MO4, MO5, MO7, MO8, MO11.
8. **Démo de début décembre** (D-034) : l'appel **en ligne** sur Android (appli existante, après MO0 et MO1) contre le labo ; iOS dès qu'un Mac est disponible (Q-MO3) ; le hors ligne est montré en **janvier** avec le lot AS10 du socle 12, comme prévu par `12_ASSIDUITE.md`.
9. **Pour Florian** (§29.3) : comptes développeur Apple et Google au nom de l'organisation (coût, D-U-N-S, contrat), un **Mac** ou une CI macOS pour construire l'appli iOS, le choix du **transport des notifications** (APNs et FCM font transiter les enveloppes par Apple et Google, DPO), l'AIPD mobile avant toute donnée réelle, les deux failles Next.js, et l'étude d'un **cœur Rust partagé** entre les deux applis (Q-MO1, non appliquée par défaut parce qu'elle change l'architecture de D-003).
10. **(ajout personas)** La passe (`PERSONAS_11.md`, 17 personas dont deux attaquants et un auditeur RGPD, 64 manques M-1100 à M-1163) trouve **quatre bloquants**, appliqués par défaut (§32) : (A) le **téléphone familial partagé** entre frères et sœurs ou avec un parent mélange les comptes et les caches → profils d'appareil séparés et verrou par compte (R-MO40) ; (B) un **lien de rattachement piégé** (faux sous-domaine, QR collé par-dessus l'affiche) mène à une page d'hameçonnage → nom et logo de l'établissement confirmés par le guichet central avant toute saisie, jamais de passkey demandée sur une page web ouverte depuis un QR (R-MO41) ; (C) un **parent interdit** garde l'appli et son cache après la décision du juge → purge et coupure en moins d'une minute, sans révéler la cause (R-MO42) ; (D) l'**encadrant d'un voyage à l'étranger** perd son téléphone avec les fiches d'urgence → bail de sortie à purge par la direction depuis la France et compte rendu automatique au DPO (R-MO43).

---

## 1. Thèse : le téléphone affiche, il ne décide pas, et il oublie

Le téléphone est l'outil de travail le plus présent dans un établissement, et le moins maîtrisé : il appartient presque toujours à la personne, il sort de l'établissement, il est prêté, partagé, perdu, volé, réparé, revendu ; son horloge peut mentir ; il est souvent hors réseau au gymnase, en sortie, à l'internat la nuit. Le socle 11 part de ce constat et pose seize principes [R] :

1. **Une seule appli par plateforme, pour tout le monde** (D-003). Élèves, familles, professeurs, vie scolaire, accueil, direction : même appli, écrans selon les droits. Aucune appli « famille » et « personnel » séparées : un professeur parent d'élève, une AED étudiante dont le petit frère est au collège, un parent de deux établissements n'installent qu'une appli.
2. **Le serveur décide, l'appli affiche.** Les droits, les écrans disponibles, les durées de cache, les versions acceptées sont calculés par le serveur et **servis** à l'appli (manifeste, §11.2). Aucune règle de droit n'est réécrite en Kotlin ou en Swift : la liste des écrans de `Capacites.kt` / `Capacites.swift` devient une lecture du manifeste. Un écran affiché à tort reçoit un 403, géré proprement.
3. **Parité stricte avec le web.** Même catalogue d'actions (D-017), mêmes portées (D-022), mêmes DTO (`ent-types`), mêmes refus. Toute capacité mobile d'un socle est une capacité déjà servie au web, testée par le même contrat (§28). Le mobile ne crée jamais de raccourci de droits.
4. **Version quotidienne, pas version complète** (D-003). Ce qui se fait debout, entre deux salles, dans le car ou le soir sur le canapé : appel, regarder, justifier, répondre, autoriser, signer, déclarer, compter, remettre. Ce qui demande un grand écran, une réflexion ou une double validation reste au web (paramétrage, imports, conseil de classe, procédure disciplinaire, séjours, finances).
5. **Passkey native d'abord** (D-009, D-013) ; aucun mot de passe tapé sur un téléphone quand la personne a une passkey ; le mot de passe n'est qu'un secours suivi d'un second facteur.
6. **Session longue, mais liée à l'appareil** (D-011). Le jeton Bearer seul ne suffit plus : une **clé d'appareil** non exportable (Secure Enclave, StrongBox ou TEE) signe chaque requête sensible et chaque lot hors ligne ; un jeton copié ailleurs ne sert à rien (§9.4).
7. **Le téléphone sera perdu demain.** Tout ce qui y est stocké est chiffré par une clé liée à la session, borné dans le temps, effaçable à distance par **destruction de la clé**, et **inventorié côté serveur** (baux, §12) pour que la perte soit un incident mesurable et non une inconnue.
8. **Le hors ligne est un contrat déclaré par chaque socle** (§12) : contenu minimal, durée maximale, événements de purge, exigence de déverrouillage, opérations différées avec fenêtre et règles de conflit. Rien n'est mis hors ligne « parce que c'est pratique ».
9. **Rien de sensible hors de l'ENT** (socle 8, P3). Une notification ne porte qu'une **enveloppe** neutre ; le contenu se lit après ouverture authentifiée ; l'écran verrouillé, le sélecteur d'applis, les widgets, les captures et le presse-papiers ne montrent jamais de donnée nominative sensible.
10. **Aucun tiers dans l'appli.** Pas de SDK de mesure d'audience, de publicité, de rapport de plantage externe, de carte, de police ou d'image chargés depuis un tiers. Les seuls tiers techniques sont les services du système (passkeys, notifications, magasins) et ils sont listés (§15.4). La mesure d'audience du public passe par le DNMA côté serveur (arrêté du 7/09/2026, `29_PORTAILS.md` §5.1).
11. **Les mineurs d'abord.** L'élève n'est jamais sollicité pendant ses cours (L511-5 ; R-MO22) ; ses notifications respectent la plage calme ; son compte n'est jamais ouvert par un parent « à sa place » ; l'élève majeur retrouve la maîtrise de ses accès (socle 4).
12. **Plusieurs établissements, aucune jointure serveur** (D-013, `29_PORTAILS.md` §11). L'appli tient une session par établissement ; elle **réunit sur l'appareil** ce que chaque établissement lui sert séparément ; aucun établissement n'apprend que la personne est connue ailleurs.
13. **Appareils partagés et appareils de l'établissement** (tablette d'accueil, iPad de classe, téléphone de service de l'internat) : même appli, mode **appareil partagé** inscrit par l'établissement, session personnelle courte par-dessus, aucun cache personnel durable (§17).
14. **Accessible pour de vrai** (RGAA, RAAM, arrêté du 7/09/2026) : tailles de texte du système, lecteurs d'écran, contrastes, gestes simples, langues des familles, **état de conformité affiché dans l'appli** (décret 2019-768).
15. **On publie comme un éditeur responsable** : deux fiches de magasin au nom de l'organisation, version minimale servie par le serveur, correctif de sécurité poussé et imposé en 48 h, nomenclature des composants (SBOM), politique de signalement des vulnérabilités (règlement européen sur la cyber-résilience, §5.6), période de support annoncée.
16. **Peu de réglages, de bons défauts** (D-023). La politique mobile d'un établissement tient en un écran (verrouillage, appareils partagés, aperçus de notifications) ; les durées hors ligne ont des plafonds fixés par les socles, que l'établissement peut **raccourcir**, jamais allonger.

**Ce qui fera la différence** [H, comparaison de mémoire avec les applis Pronote, École Directe et des ENT régionaux, non vérifiée en détail] : l'appel hors ligne réel ; la journée du professeur partagé entre deux établissements réunie sur son téléphone ; la famille séparée qui ne voit jamais rien de l'autre foyer ; le téléphone perdu révoqué **avec l'inventaire de ce qu'il portait** ; une appli sans aucun traceur ; l'élève jamais dérangé en cours.

---

## 2. Évaluation franche de l'existant

Lecture seule, rien exécuté (aucun Gradle, aucun `swift test` relancé dans cette session : les résultats cités sont ceux de `docs/MOBILE.md` de la branche, datés du 2026-11-06). Deux sous-agents de revue lancés pour cette conception ont été interrompus par une limite de quota ; les constats ci-dessous ont donc été **relus directement** aux lignes citées.

### 2.1 Ce qu'il y a [F]

| Brique | Où | Ce qu'elle fait | Verdict |
|---|---|---|---|
| **4 prototypes** | `ent-rust/apps/{ios,android}-{eleve,famille}` sur `ent-mc0hoe` (5 625 lignes ; plus gros fichiers : `android-famille/.../EntApiClient.kt` 1 597 lignes, `ios-famille/Sources/FamilleApp.swift` 1 077) ; `ent-rust/apps/README.md` | squelettes sans projet de build ; `POST /auth/login` + Bearer ; jeton **en mémoire** (`EntApiClient.kt:18`, `EntApiClient.swift:7`) ; base `http://127.0.0.1:3001` / `http://10.0.2.2:3001` par défaut ; paquet `fr.scola.*` (coquille) ; surfaces famille riches (sorties, documents, cantine, carnet, push) | **à retirer** : fusionnés dans `ent-mobile-app` (commit `2346b44` les supprime sur cette branche) ; les fonctions perdues sont listées au §2.4 |
| **Appli Android unique** | `origin/ent-mobile-app:ent-rust/apps/android` : `:core` (Kotlin JVM 17 : `ApiClient`, `Capacites`, `HorsLigne`, `Modeles`, `Passkey`, `Rattachement`, `Session`) et `:app` (Compose, Material 3 : `AppModele`, `MainActivity`, `ecrans/*`, `securite/*`, `ui/*`) | connexion passkey (Credential Manager), secours mot de passe + second facteur, enrôlement bloquant, verrou biométrique, rattachement par lien / QR (zxing) / saisie, appel, absences famille, notes, saisie de notes, cahier, EDT, messagerie (réponse), « Mon compte » (appareils, révocation) ; `allowBackup=false`, `FLAG_SECURE`, `minSdk 28`, `targetSdk 35` (`app/build.gradle.kts:16-17`) | **à garder** comme base |
| **Appli iOS unique** | `origin/ent-mobile-app:ent-rust/apps/ios` : `ScolIACore` (SwiftPM, compile sous Linux), `ScolIAUI` (SwiftUI), `App/` (XcodeGen `project.yml`), `ScolIA.entitlements` (`webcredentials:scolia.fr`, `applinks:*.scolia.fr`) | mêmes fonctions ; Keychain `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` ; `ASAuthorizationPlatformPublicKeyCredentialProvider` ; Face ID ; scanner AVFoundation | **à garder** ; jamais compilé (pas de SDK iOS hors macOS) |
| **Doc d'architecture** | `origin/ent-mobile-app:docs/MOBILE.md` (§1 à §12) | organisation, écrans par rôle, API utilisées, API manquantes **M1–M12**, vérifications, revue indépendante | **à fondre** dans ce document (§2.3) ; `MOBILE.md` devient le guide de build |
| **Sessions** | `ent_session` (005, 095, 357) ; `ent-db/src/auth_repo.rs:27-34` (`ENT_SESSION_MOBILE_JOURS` 90, `..._MAX_JOURS` 365), `:44-60` (`ClientSession::parse_client` : « tout inconnu → web »), `:522`, `:582`, `:941-943` (glissement plafonné) ; jeton opaque 32 octets, **haché SHA-256** en base (`ent-auth/src/token.rs:5-10, 25-29`) ; `MeDto.client` et `portee` (`ent-types/src/auth.rs:83-91`) ; casquette de session (357:52) | sessions à deux durées, portée « enrôlement », assurance | **à garder** ; DS2 |
| **Passkeys** | `ent-auth/src/passkey.rs:55-92` (`ENT_RP_ID`, origines supplémentaires dont `android:apk-key-hash:…`) | rp unique, origines natives Android prévues | **à garder** ; associations de domaine absentes (DS6) |
| **Appareils push** | `058_*.sql:3-21` (`ent_push_device` : `user_id`, `platform`, `token` **texte**, `app_id`, `revoked_at`, unique `(etablissement_id, token)`) ; `POST /devices/push` (`ent-api/src/routes.rs:195`, `:2933-2957`) ; `ent-db/src/push_repo.rs:64-69` (`ON CONFLICT … DO UPDATE SET user_id = EXCLUDED.user_id`) ; `ent-push` sans connecteur réel (`08_NOTIFICATIONS.md` E12) | inscription d'un jeton ; aucun envoi réel | **à remplacer** par `ent_ntf_appareil` (socle 8, 3803) relié à l'appareil du socle 11 ; DS1, DS3, DS4, DS5 |
| **Établissement par l'adresse** | `GET /auth/etablissement` (`routes.rs:143`, `:13233-13251`, extracteur `TenantHote`) | nom public de l'établissement d'un sous-domaine | **à garder** (premier appel du rattachement) |
| **Invitations** | `ent_agent_invitation` (357:76-110 : lien e-mail 7 jours ou QR, jeton haché, une en attente par agent) | rattachement d'un **personnel** | **à réutiliser** ; rien pour familles et élèves (code de rattachement du socle 9, `ent_fed_*`, 3910) |
| **Registre des tables** | `ent-db/src/registre_donnees.rs:524-525` (`ent_push_device` « perso, export » ; `ent_push_outbox` « journal seul ») | classement E12 | DS5 |
| **API mobile Next.js** (production, lecture seule) | `app/api/mobile/{me,today,appel}/route.ts` (227 lignes), `app/lib/mobile-auth.ts:25-50`, `app/lib/channel-access.ts:11-31`, page `app/app-mobile/page.tsx` | « staff-lite » : contexte, journée (créneaux et appels), appel ; familles et élèves limités à `/app-mobile`, `/quotidien`, `/api/famille`, `/api/eleve` | **à ne pas reprendre** (DX1, DX2) ; aucune appli native connue côté Next.js [H : rien dans le dépôt] |

### 2.2 Défauts trouvés

Gravité : **G** grave (sécurité ou perte de données), **I** important (résultat faux, blocage de publication), **C** confort ou dette.

| N° | Gravité | Défaut | Preuve | Correction (lot) |
|---|---|---|---|---|
| **DS1** | **G** | **Révoquer une session ne coupe pas les notifications** : `ent_push_device` n'a ni `session_id` ni appareil ; la révocation ne met à jour que `ent_session` ; un téléphone perdu et révoqué continue de recevoir les enveloppes (nom de l'établissement, catégorie, parfois enfant) tant que le compte existe | `058:3-21` ; `auth_repo.rs:652-653` (`UPDATE ent_session SET revoked_at …` seul) ; aucun `ent_push_device` dans `auth_repo.rs` | MO0 : colonne `session_id`, révocation en cascade dans la même transaction, test |
| **DS2** | **G** | **Le client se déclare « mobile »** : `parse_client` lit le champ du corps ; tout client (script, navigateur détourné, poste partagé) obtient 90 jours glissants et jusqu'à un an, sans clé d'appareil ni attestation (déjà relevé E7 au socle 1, M4 de `MOBILE.md`) | `auth_repo.rs:57-60`, `:522`, `:941-943` | MO0 : durée mobile **seulement** si l'assurance est `passkey` et qu'une clé d'appareil est enregistrée (preuve de possession) ; sinon durée web. MO4 : signal d'attestation |
| **DS3** | **I** | Jeton push **en clair** en base, **renvoyé** dans la réponse `201`, et **réaffecté** sans trace au dernier compte qui l'inscrit (un parent qui se connecte sur le téléphone de sa fille récupère ses notifications à elle et réciproquement, sans avertissement) | `058:8` ; `routes.rs:2951-2955` ; `push_repo.rs:68-69` | MO0 : réponse sans jeton ; jeton chiffré + empreinte (modèle `ent_ntf_appareil`) ; réaffectation = révocation de l'ancienne inscription + avis de sécurité à l'ancien titulaire (R-MO17) |
| **DS4** | **I** | **Aucune désinscription** (`DELETE /devices/push/{id}` absent, M5) ; la déconnexion laisse le jeton actif | `routes.rs:195` (POST seul) | MO0 |
| **DS5** | **I** | `ent_push_device` classé « export » (les jetons d'appareils partiraient dans l'archive de portabilité, contraire à la minimisation) et **non neutralisé** à la restauration (E9) : après une restauration à J-7, un appareil révoqué entre-temps redevient joignable | `registre_donnees.rs:524` | MO0 : `neutralise(N_PURGE)`, exclu de l'export ; les révocations sont rejouées depuis `ent_mob_ordre_purge` (§26) |
| **DS6** | **I** | **Pas de fichiers d'association de domaine** (`/.well-known/assetlinks.json`, `/.well-known/apple-app-site-association`) : passkeys natives et liens universels impossibles (M1) ; le seul `.well-known` du serveur est la découverte OIDC sortante | aucune route `well-known` hors `ent-oidc` (recherche du 2026-11-11) | MO2 |
| **DS7** | **I** | **Aucune API hors ligne** (lots d'opérations, baux, ordres de purge), ni ETag ni pagination pour les lectures mises en cache (M9) | — | MO7, MO8 |
| **DS8** | **I** | **Aucune invitation famille ou élève** consommable par l'appli (M2) ; `GET /auth/etablissement` suppose que la personne connaît l'adresse ; aucune recherche d'établissement (M3) | 357 (agents seulement) | MO3, avec le socle 9 (codes de rattachement) |
| **DA1** | **G** | **Le cache hors ligne n'expire jamais** : une entrée n'a qu'un horodatage ; elle est servie « en secours » quel que soit son âge ; purge seulement au 401 ou à la déconnexion. Un téléphone volé et tenu hors réseau garde notes, fils de messages, EDT et liste d'enfants **indéfiniment** ; contraire à R-PO33 (24 h, jamais l'EDT au-delà du jour), R-AS32 (minuit) | `origin/ent-mobile-app:…/core/HorsLigne.kt:10, 22, 34, 56-59` ; `MOBILE.md` §7 | MO0 : plafond provisoire (minuit, 24 h au plus) et purge au démarrage ; MO7 : baux |
| **DA2** | **I** | `targetSdk 35` : Google Play exige l'**API 36** pour les nouvelles applis et les mises à jour depuis le **31/08/2026** [V~, https://developer.android.com/google/play/requirements/target-sdk] ; aucune publication possible en l'état | `app/build.gradle.kts:17` | MO0 (avec essais de régression : bord à bord imposé, retour prédictif) |
| **DA3** | **I** | **Aucune écriture différée** : l'appel exige le réseau, contraire au socle 12 (§9.4, AS10) et à `SOCLES.md` §11 | `MOBILE.md` §7 | MO8 |
| **DA4** | **I** | Clé du jeton **non liée** à l'authentification de l'appareil : le verrou biométrique est un verrou d'interface ; une extraction du stockage d'une appli déverrouillée suffit | `MOBILE.md` §4 point 5 | MO4 |
| **DA5** | **I** | **Démarrage à froid hors ligne impossible** (le cache n'est lisible qu'après `/auth/me`) : au gymnase, l'appli rouverte n'affiche rien | `MOBILE.md` §12 « Restant » | MO7 (contexte mis en cache sous bail) |
| **DA6** | **I** | **Casquettes** (socle 6, P2 : une session = une casquette, `principale` ou `parent`) ignorées : le profil se réduit à `ELEVE / FAMILLE / PERSONNEL` ; un professeur parent d'élève dans le même établissement ne peut pas voir son enfant | `MOBILE.md` §10 « Profil se réduit à… » ; `357:52-58` | MO5 |
| **DA7** | **C** | DTO **écrits à la main** sans test de contrat contre `ent-api` (M11) : toute évolution de l'API casse l'appli en silence | `MOBILE.md` §1 | MO1 |
| **DA8** | **C** | iOS : interface **jamais compilée** ; au-delà de 5 onglets, onglet « Plus » du système (famille : 7) | `MOBILE.md` §10, §12 | MO1 (sur Mac) |
| **DA9** | **C** | Liens universels et App Links déclarés sur `*.scolia.fr` : chaque sous-domaine d'établissement doit servir (ou la racine couvrir) les fichiers d'association [H sur le mode exact de vérification des jokers par iOS et Android, à vérifier au lot MO2] ; build debug `fr.scolia.mobile.debug` à déclarer aussi | `ScolIA.entitlements` ; `AndroidManifest.xml:33-37` | MO2 |
| **DA10** | **C** | Dépendances anciennes (`androidx.biometric:1.1.0`, `credentials:1.3.0`, `core-ktx:1.13.1`) ; aucune politique de mise à jour ni SBOM | `app/build.gradle.kts:81-93` | MO12 |
| **DX1** | signalé | **Next.js `/api/mobile/today`** : quand le nom de l'enseignant ne correspond à aucun créneau, **tous les créneaux de l'établissement** sont renvoyés (`mine.length ? mine : creneaux`), avec la liste des appels manquants de tout l'établissement ; filtre par ressemblance de prénom | `app/api/mobile/today/route.ts:29-46` | **aucune action** (Next.js intouchable) ; à vérifier par Florian (§29.3) |
| **DX2** | signalé | **Next.js `/api/mobile/appel?classe=…`** : tout personnel « staff » lit la liste de **n'importe quelle classe** avec INE et URL de photos, sans contrôle de périmètre ni de module (confirme P-1 de `12_ASSIDUITE.md` §27.2) | `app/api/mobile/appel/route.ts:44-50` ; `mobile-auth.ts:25-50` | idem |

### 2.3 Garder, corriger, remplacer

| Verdict | Quoi | Pourquoi |
|---|---|---|
| **Garder** | Les deux applis de `ent-mobile-app` (organisation cœur testable / interface, mêmes noms et mêmes tests des deux côtés) ; stockage du jeton (Keystore AES-GCM non exportable, Keychain `ThisDeviceOnly`) ; aucune redirection suivie ; `toString` masqués ; rattachement strict (`https`, hôte sous `.scolia.fr`, sous-domaines techniques refusés, code borné) ; `rpId` vérifié avant l'appel au système ; 401 = purge ; confirmation avant clôture d'appel, envoi, révocation ; `allowBackup=false`, `FLAG_SECURE`, contenu masqué hors premier plan ; accessibilité de base (tailles du système, cibles ≥ 48 dp, libellés) ; tests Compose sans appareil (Robolectric) ; serveur : jetons hachés, sessions à deux durées et glissement plafonné, portée « enrôlement », origines natives WebAuthn | Bien orientés, testés, conformes aux décisions |
| **Corriger tout de suite (MO0)** | DS1, DS2, DS3, DS4, DS5, DA1, DA2 | sécurité, publication bloquée |
| **Remplacer** | `Capacites` (écrans calculés dans l'appli) → **manifeste servi par le serveur** (§11.2) ; `HorsLigne` (cache sans bail) → **moteur hors ligne** (§12) ; `ent_push_device` → `ent_ntf_appareil` (8) + `ent_mob_appareil` (11) ; DTO écrits à la main → DTO **générés** depuis `ent-types` (§8.3) ; profils `ELEVE/FAMILLE/PERSONNEL` → contextes établissement × casquette × enfant (§10) | conçus avant les socles 6, 8, 12, 29 |
| **Retirer** | les 4 prototypes de `ent-mc0hoe` (après fusion de `ent-mobile-app`, lot MO1) ; `ent-rust/apps/README.md` ancien ; l'adresse `http` par défaut | doublons, risque de régression |
| **Ne pas reprendre** | l'API « staff-lite » de Next.js (contexte plat, filtre par nom, liste de classe ouverte) | DX1, DX2 ; le modèle cible est la séance du socle 10 et la portée D-022 |

### 2.4 Fonctions des prototypes non portées (à ne pas perdre, D-035 point 5)

Les prototypes famille offraient (`ent-rust/apps/README.md`, tableau) : sorties, documents, cantine, vie scolaire, carnet, push. `MOBILE.md` §2 les déclare « hors périmètre mobile » pour l'instant. Arbitrage [R] : chacune revient **par son socle**, dans son lot mobile déjà prévu (« carnet à signer » → 16 ; documents et signature → 7, D13 ; cantine → 40 ; sorties → 42 ; internat → 41), sans écran provisoire dans la coquille. La liste des lots mobiles des socles est au §27.3.

---

## 3. Acteurs, besoins réels, irritants

Abréviations reprises des socles 10 et 12 : PRF, PP, REM (remplaçant, contractuel), CPE, AED, ACC (accueil), DIR (chef, adjoint), SEC, INF, AESH, ENC (encadrant de sortie), INT (surveillant d'internat), AGT (agent technique), FAM (responsable), MAJ (élève majeur), ELV, REF (référent numérique de l'établissement), DPO, ADM (administrateur de l'établissement dans ScolIA), EDI (éditeur ScolIA : support, publication).

| Acteur | Ce qu'il fait sur le téléphone | Ce qui lui fait perdre du temps ou ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Professeur** (dont REM, professeur partagé, PP) | appel, retard, mot sur un élève, fin de cours (cahier), une note, répondre à un parent, regarder sa journée, accepter un remplacement | réseau absent (gymnase, labo au sous-sol) ; appel perdu ; dix applis et dix mots de passe ; répondre le soir et être relancé ; téléphone personnel qui contient des données d'élèves ; être joint à 22 h | appel en dix secondes hors ligne ; une seule appli pour ses deux établissements et pour son enfant ; rien sur l'écran verrouillé ; la plage calme qui le protège aussi |
| **CPE, AED** | appels non faits, absences du matin, billets, localiser un élève, retenues, exclusions de cours, comptage en alerte | courir entre les salles ; téléphone de service partagé entre AED ; ne pas savoir qui a le téléphone | file du matin dans la poche ; téléphone de service avec sa session personnelle par-dessus |
| **Accueil** | remise d'un élève, visiteur, appel d'un parent saisi, fiche de secours | tablette partagée, panne réseau à 16 h 30, interdiction de remise oubliée | interdictions toujours en tête, même hors ligne ; session qui se ferme seule |
| **Direction** | alerte PPMS, crise, fermeture, validation rapide, situation du jour | déclencher une alerte depuis la cour ; lancer une diffusion sur un écran minuscule | deux gestes + passkey pour l'alerte ; tableau de crise lisible |
| **Infirmière** | rare sur mobile (poste fixe) ; fiche d'urgence pour une sortie | données de santé sur un téléphone | aucune donnée de santé hors ligne hors fiche d'urgence de sortie (34) |
| **AESH** | journée des élèves accompagnés, avis d'absence de l'élève | être prévenue trop tard ; voir ce qui ne la regarde pas | sa journée, rien d'autre |
| **Encadrant de sortie, surveillant d'internat** | appel de départ et d'étape, trousses, fiches d'urgence, point de situation ; appel de nuit | réseau à l'étranger, batterie, téléphone perdu dans le car | tout marche sans réseau, et tout s'efface au retour |
| **Agent technique** | signalement, salle fermée (44, 10) | — | photo + lieu en deux gestes |
| **Secrétariat** | presque rien (poste fixe) ; rattachements à expliquer aux familles | familles qui n'arrivent pas à se connecter, appels « l'appli ne marche pas » | un QR de rattachement imprimé sur la feuille de rentrée, un état clair de chaque compte |
| **Famille** (dont séparée, sans exercice de l'autorité, interdite, allophone, sans smartphone récent) | absences et justification, EDT du jour, notes, messages, autoriser, signer, payer (lien), RDV, sortie | une appli par établissement ; un mot de passe de plus ; tout en français ; notification « nouveau message » à 23 h ; voir ce que l'autre parent a écrit (et l'inverse) ; parent séparé qui apprend tout par l'enfant | une appli, deux établissements, trois enfants ; passkey du téléphone ; sa langue ; « Aujourd'hui » de toute la fratrie ; aucune fuite vers l'autre foyer |
| **Élève** (collégien de 11 ans, lycéen, majeur, BTS) | EDT, devoirs, notes, messages, carte d'élève, emprunts CDI | téléphone confisqué ou en pause pendant la journée ; téléphone partagé avec un frère ; parents qui lisent par-dessus l'épaule | rien pendant les cours ; le soir, tout ce qu'il faut pour demain ; à 18 ans, la main sur ses accès |
| **Référent numérique** (REF) | parc d'iPad et de tablettes, tablettes de l'accueil, téléphones de service | inscrire vingt appareils un par un ; appareils prêtés jamais rendus | inscription en série par la gestion de flotte (MDM), état de chaque appareil |
| **DPO** | rien sur le téléphone ; registre, AIPD, incidents | « quelles données étaient sur le téléphone volé ? » sans réponse | l'inventaire des baux et le compte rendu d'incident automatiques |
| **Éditeur (EDI)** | publication, support, correctif urgent | appli bloquée par un magasin la veille de la rentrée ; vieilles versions qui cassent l'API | version minimale servie par le serveur ; correctif imposé en 48 h ; journal de diagnostic sans donnée personnelle |
| **Attaquant** (élève, ex-conjoint, ex-agent, inconnu) | voler un téléphone de professeur en classe, rejouer un lot, poser un faux QR, détourner un lien, extraire un cache | — | ne rien obtenir (§28.3) |

---

## 4. La journée et l'année vues du téléphone [H, d'après le terrain et les conceptions 10, 12, 14, 29]

**Une journée.** *6 h 45* : la mère reçoit (fin de plage calme à 7 h) « Cours de 8 h annulé pour Léo » ; l'appli réunit les deux établissements de ses enfants. *7 h 30* : le professeur d'EPS ouvre l'appli ; le profil hors ligne « appel du jour » se met à jour (séances du jour, effectifs minimaux, situations connues), bail émis jusqu'à minuit. *8 h 05* : au gymnase, sans réseau, il fait l'appel ; « enregistré sur le téléphone, 1 lot à envoyer ». *8 h 50* : retour du réseau, lot signé envoyé, appliqué, un conflit (élève localisé à l'infirmerie) tranché par la règle R-AS10. *10 h 15* : l'AED prend le téléphone de service, sa session personnelle s'ouvre par passkey par-dessus le mode partagé ; la file du matin s'affiche. *Pendant la journée* : les élèves, téléphone en pause, ne reçoivent **aucune** notification non urgente ; elles sont regroupées pour la fin des cours (R-MO22). *12 h 30* : un parent justifie une absence en photographiant le mot ; l'appli retire les métadonnées de position de la photo avant envoi. *16 h 30* : panne réseau à l'accueil ; la tablette affiche la fiche de secours du jour (interdictions en tête). *17 h 10* : un lycéen, à la sortie, voit son travail pour demain. *20 h 30* : un professeur répond à un parent ; l'appli lui rappelle le droit à la déconnexion et l'heure à laquelle le message sera visible (7 h). *23 h 59* : les baux « appel du jour » expirent ; les clés sont détruites sur les téléphones.

**Une année.** *Juin* : l'établissement prépare la rentrée ; codes de rattachement imprimés sur les dossiers (socles 3, 9). *Fin août* : nouveaux personnels invités (QR, 357) ; tablettes d'accueil et iPad réinscrits par la gestion de flotte. *Rentrée* : pic de rattachements familles ; « vérification de joignabilité » (socle 8) ; appels au secrétariat. *Octobre* : première version minimale relevée (correctif) ; les vieilles versions reçoivent « mettez à jour ». *Novembre–mars* : sorties et voyages (baux de sortie) ; PPMS (exercice). *Juin* : fin d'année ; comptes des élèves qui quittent : appli qui le dit proprement (« votre accès à X s'arrête le … ») ; élèves majeurs. *Toute l'année* : téléphones perdus (déclaration, purge, compte rendu au DPO) ; nouveaux appareils ; versions du système d'exploitation ; une version de l'appli par mois [H].

---

## 5. Ce qui est réglementaire

Recherche du 2026-11-11 (moteur de recherche ; plusieurs pages officielles connues par des extraits, d'où les [V~]). Les points déjà vérifiés par d'autres conceptions sont repris avec leur source.

### 5.1 Téléphone portable à l'école, au collège et au lycée
- **Article L511-5 du code de l'éducation** : l'utilisation d'un téléphone mobile ou de tout équipement terminal de communications électroniques par un élève est interdite dans les écoles et les collèges (et pendant les activités liées à l'enseignement hors de l'établissement), sauf circonstances, lieux et usages pédagogiques prévus par le règlement intérieur [V~, https://www.education.gouv.fr/interdiction-du-telephone-portable-dans-les-ecoles-et-les-colleges-et-pause-numerique-455181].
- **« Portable en pause »** généralisé à tous les collèges en 2025-2026 par la circulaire du 10 juillet 2025 « Pour un numérique raisonné à l'école » (BO n° 28, NOR MENE2519904C) [V~, https://www.education.gouv.fr/bo/2025/Hebdo28/MENE2519904C].
- **Loi du 24 août 2026** (JO du 25 août 2026) visant à protéger les mineurs des risques des réseaux sociaux : la mesure sur les réseaux sociaux des moins de 15 ans a été censurée par le Conseil constitutionnel (14/08/2026) ; la disposition qui **étend aux lycées, dès la rentrée 2026-2027, l'interdiction du téléphone pendant le temps scolaire** (modification de L511-5), modalités et exceptions fixées par le règlement intérieur, a été promulguée [V~, https://www.vie-publique.fr/loi/301799-proteger-les-mineurs-risques-des-reseaux-sociaux-loi-du-24-aout-2026 ; https://www.info.gouv.fr/actualite/interdiction-des-telephones-portables-au-lycee-ce-que-dit-la-loi] ; texte consolidé de L511-5 à relire sur Légifrance avant le lot MO5 [NV].
- **Conséquences** [R] : (a) l'appli élève sert **hors temps scolaire** ; aucune fonction élève ne doit **exiger** le téléphone pendant la journée (carte d'élève et QR de sortie : voir C-1, §30) ; (b) aucune notification non urgente à un élève **pendant ses séances** (R-MO22) ; (c) les usages pédagogiques autorisés par le règlement intérieur (activité en classe, sortie) passent par les appareils de l'établissement (§17) ou par une exception déclarée par l'établissement (S-1108) ; (d) les personnels ne sont pas concernés.

### 5.2 Cadre de référence du numérique pour l'éducation (arrêté du 7 septembre 2026)
Repris de `29_PORTAILS.md` §5.1 et `09_INTEROP.md` (VERIF_09) [V~] : **plage calme** 20 h–7 h et vendredi 20 h–lundi 7 h (consultation et saisie possibles, diffusion suspendue, rappel à l'émetteur, ajustements du chef d'établissement), délai 10 mois ; **RGAA** pour les sites, logiciels **et applications mobiles** ; **DNMA** pour la mesure d'audience ; ÉduConnect, Guichet Agents (délais 10 et 22 mois) ; IA générative seulement anonymisée via le GAR. **Conséquences** : la plage calme est appliquée par le serveur (socles 8, 16, 29), l'appli ne programme rien elle-même et affiche à l'émetteur l'heure de visibilité (MOB-171) ; accessibilité au §18 ; aucune mesure d'audience dans l'appli (le DNMA est servi côté serveur si le contrat l'exige pour le mobile, Q-MO8) ; connexion fédérée au §9.3.

### 5.3 Accessibilité
Article 47 de la loi 2005-102 (applications mobiles incluses), décret 2019-768 (« **état de conformité précisé dans l'application** », déclaration chez l'éditeur ou avec les informations de téléchargement, mise à jour à chaque refonte), RGAA 4.1.2, **RAAM** pour les applis mobiles recommandé par la doctrine technique du numérique pour l'éducation [V / V~, `29_PORTAILS.md` §5.2]. Acte européen sur l'accessibilité (directive (UE) 2019/882) : applicabilité à un ENT non tranchée (Q-PO8 du 29), sans effet sur la conception puisque le RGAA est appliqué partout.

### 5.4 Protection des données
- **Recommandation de la CNIL sur les applications mobiles** (version finale publiée le 24/09/2024, délibération n° 2024-061 [V~, https://www.cnil.fr/fr/recommandations-applications-mobiles ; PDF https://www.cnil.fr/sites/cnil/files/2024-09/recommandation-applications-mobiles.pdf]) : vise éditeurs, développeurs, fournisseurs de SDK, de systèmes et de magasins ; répartition des responsabilités, permissions demandées au moment du besoin et proportionnées, information claire, maîtrise des SDK tiers, consentement aux traceurs non strictement nécessaires. Une source secondaire indique un remplacement par une délibération n° 2025-024 du 27/03/2025 [NV, à vérifier sur cnil.fr avant MO11]. Une **campagne de contrôle** des applis mobiles a été annoncée pour 2025 [V~].
- **Traceurs** (article 82 de la loi Informatique et libertés) : sans SDK tiers ni mesure d'audience dans l'appli, les seuls accès au terminal sont **strictement nécessaires** au service demandé (session, clé d'appareil, cache sous bail, jeton de notification après accord du système) [R, raisonnement sur le texte, NV pour l'avis de la CNIL sur le jeton push].
- **Base légale** : mission d'intérêt public (RGPD art. 6.1.e) pour l'ENT, journaux 12 mois au plus, suppression 3 mois après la perte de vocation au compte (arrêté du 30/11/2006 modifié, kit juridique du SDET) [V~, `29_PORTAILS.md` §5.4] ; le consentement de l'article 45 (15 ans) n'est donc pas la base des traitements de l'ENT [R, à confirmer par le DPO, Q-MO9] ; il le redeviendrait pour toute fonction facultative fondée sur le consentement (aucune n'est prévue).
- **RGPD art. 25 et 32** : protection dès la conception et sécurité ; **art. 33** : notification d'une violation à la CNIL sous 72 h (perte d'un téléphone non chiffré ou dont les données ne sont pas maîtrisées) → registre des baux et compte rendu automatique (§12.7).

### 5.5 Magasins d'applications
- **Apple, règle 5.1.1(v)** : une appli qui permet de **créer** un compte doit permettre d'en **demander la suppression** depuis l'appli ; les secteurs réglementés peuvent passer par un service client [V~, https://developer.apple.com/support/offering-account-deletion-in-your-app]. ScolIA ne crée pas de compte dans l'appli (comptes créés par l'établissement ou par la préinscription web) ; l'appli offre néanmoins **« demander la fermeture de mon compte »** (MOB-240) qui ouvre une demande à l'établissement, et une page web publique qui l'explique [R].
- **Apple, règle 4.8** (connexion par un service tiers → proposer une option respectueuse de la vie privée) : exception pour une connexion avec le compte de l'établissement ou un compte d'éducation [NV : connu par une source secondaire] ; ScolIA n'offre ni Google ni Facebook dans l'appli ; la fédération ÉduConnect et l'OIDC d'établissement relèvent de l'exception [H, à vérifier au lot MO11].
- **Google Play** : niveau d'API cible 36 pour les nouvelles applis et les mises à jour depuis le 31/08/2026 [V~, page officielle] ; formulaire « sécurité des données » ; exigence d'un **lien web de suppression de compte** pour les applis qui créent des comptes [NV] ; règles « Familles » si l'audience cible inclut des enfants [NV] — l'appli ne cible pas les enfants (tous rôles, comptes fournis par l'établissement), à déclarer correctement (Q-MO6).
- **Comptes développeur d'organisation** (D-U-N-S, vérification d'identité) : coût et engagement contractuel → Florian (§29.3).

### 5.6 Cyber-résilience (règlement (UE) 2024/2847)
Obligations de **signalement des vulnérabilités activement exploitées et des incidents graves** (article 14 : alerte précoce sous 24 h, notification sous 72 h, à l'ENISA et au CSIRT désigné) applicables depuis le **11 septembre 2026** ; application générale le **11 décembre 2027** (exigences essentielles, documentation, SBOM, période de support, marquage) [V~ : sources secondaires concordantes, ex. https://www.crowell.com/en/insights/client-alerts/eu-cyber-resilience-act-countdown-11-september-2026-incidentvulnerability-reporting-deadline-is-less-than-100-days-away ; texte officiel à relire, NV]. La qualification d'une appli mobile gratuite liée à un service en ligne comme « produit comportant des éléments numériques » n'est pas tranchée par les sources lues ; les services de traitement de données à distance sont exclus sous conditions [NV]. **Recommandation appliquée** : traiter les deux applis **comme si** elles étaient couvertes (politique de divulgation coordonnée publiée, procédure de signalement 24 h / 72 h, SBOM à chaque version, période de support annoncée, mises à jour de sécurité séparables), parce que le coût est faible et que l'exclusion est incertaine (§19.5) ; avis juridique à demander (Q-MO10, Florian).

### 5.7 Droit à la déconnexion des personnels
Privé : négociation annuelle sur le droit à la déconnexion (code du travail L2242-17) [NV] ; public : principe de l'arrêté du 7/09/2026 pour les ENT, chartes d'établissement [V~]. Conséquence : les notifications aux **personnels** respectent la plage calme par défaut, sauf natures d'urgence (socle 8 U3) et la disponibilité que la personne déclare (« en service », socle 8) ; l'appli n'impose aucun « vu » à un collègue (socle 16).

### 5.8 Référentiels de sécurité
OWASP **MASVS** (exigences de sécurité des applis mobiles) et MASTG (tests) comme grille de revue [NV pour la version en vigueur, à relever au lot MO12] ; recommandations de l'ANSSI pour le développement sécurisé [NV].

---

## 6. Frontières et ports

### 6.1 Tableau
| Sujet | Socle 11 | Autres socles |
|---|---|---|
| Écrans métier mobiles | **ne les fait pas** ; fournit le kit, la navigation, les composants communs (liste d'élèves, grille d'appel générique avec le 12, sélecteur de contexte, appareil photo de pièce, signature passkey) | **chaque socle** livre ses écrans dans son lot mobile (§27.3) |
| Création de session, passkeys, récupération | appelle ; parcours natif ; clé d'appareil | **1** (`ent-auth`) crée la session, vérifie les passkeys, D-018 |
| Fédération ÉduConnect, Agents, OIDC d'établissement | session d'authentification du système (`ASWebAuthenticationSession`, Custom Tabs) + PKCE | **9** conçoit (guichet central `connexion.scolia.fr`), **1** exécute |
| Rattachement (invitation, code, QR) | lecture, vérification, parcours | **6** (personnels, 357), **9** (codes de rattachement), **3/4** (familles) émettent |
| Appareils | **possède** l'appareil mobile (`ent_mob_appareil`), sa clé, ses baux, ses ordres de purge | **8** possède le jeton de notification (`ent_ntf_appareil`), lié à l'appareil ; **14** inscrit les appareils d'accueil et de portail (ACS-320) **par** le registre du 11 |
| Notifications | **réception** : jeton, autorisation du système (R-N57), extension d'aperçu, catégories, ouverture vers l'écran | **8** : natures, destinataires, plage calme, enveloppes, envoi APNs / FCM (N7) |
| Hors ligne | **moteur** : baux, chiffrement, purge, file d'opérations, transport des lots, idempotence | **chaque socle** déclare ses profils et ses opérations et **juge** les conflits (12 appel, 14 contrôles et remises, 21 notes, 22 positionnements, 41 nuit, 42 sortie, 34 fiche d'urgence, 16 et 20 brouillons) |
| « Aujourd'hui », à-faire, recherche | rendu natif, réunion multi-établissements sur l'appareil | **29** (contrat de tuile, index des tâches) |
| Droits | aucun calcul ; affiche le manifeste | **1** (D-017), chaque socle déclare ses actions |
| Interdictions, suspensions de lien | applique la purge et la coupure | **4** pose ; **1** révoque les sessions |
| Accessibilité, langues | composants, réglages, état de conformité | **7** et **29** (gabarits traduits) |
| Publication, versions | **possède** | — |

### 6.2 Ports exposés par le socle 11 [R]
| Port | Pour qui | Contenu |
|---|---|---|
| `mobile::declarer_ecran` | tous les socles (kit D-026) | code d'écran, capacité (action D-017 + portée) qui l'ouvre, profil(s) concerné(s), version d'appli minimale, profil hors ligne éventuel ; **déclaration en code**, compilée, pas en base (D-036) |
| `mobile::declarer_profil_hors_ligne` | 12, 14, 16, 20, 21, 22, 29, 34, 41, 42 | code, données (types `ent-types` listés champ par champ), classe de sensibilité maximale (C0–C3), durée maximale, événements de purge, déverrouillage exigé, taille maximale, qui peut le recevoir (capacité) |
| `mobile::declarer_operation` | 12, 14, 21, 22, 41, 42 | type, validateur serveur (fonction du socle), fenêtre d'acceptation, champs, règle de conflit (renvoyée au socle) |
| `mobile::emettre_bail` / `rendre_bail` | les socles via le moteur | inscription des baux |
| `mobile::ordonner_purge` | 1 (révocation), 4 (lien suspendu, interdiction), 6 (fin de fonction), 34, 42, 14 | purge d'un appareil, d'une session ou de tous les appareils d'une personne dans l'établissement, avec motif typé |
| `mobile::appareils_de(personne)` | 1 (« Mon compte »), 8 (joignabilité), 14 (appareils d'accueil), DPO | liste minimale |
| `mobile::inventaire_baux(appareil)` | DIR, DPO (incident) | ce que l'appareil portait, en catégories et en nombre de personnes, jamais la liste des noms hors procédure |

### 6.3 Ports consommés
`auth::session_mobile`, `auth::passkey_*`, `auth::revoquer` (1) ; `federation::ticket` (9) ; `rattachement::lire_invitation` (6, 9) ; `droits::manifeste` (1, à partir du registre des capacités d'`IA_COEUR.md` §5) ; `notifications::inscrire_appareil`, `::autorisation` (8) ; `portail::aujourdhui`, `::a_faire_de` (29) ; événements `lien.suspendu`, `lien.interdiction_posee`, `personne.departie`, `compte.desactive`, `session.revoquee`, `droits.modifies`, `etablissement.retire` (§25.2).

---

## 7. Concepts et vocabulaire [R]

| Terme | Définition | Table |
|---|---|---|
| **Installation** | une copie de l'appli sur un téléphone ou une tablette, identifiée par un identifiant aléatoire tiré au premier lancement, **jamais** par un identifiant du constructeur | locale à l'appareil seulement |
| **Appareil inscrit** | une installation connue d'**un** établissement pour **un** compte : clé publique d'appareil, plateforme, version, dernier contact, état ; un même téléphone est inscrit séparément dans chaque établissement (D-013), sans lien serveur entre ces inscriptions | `ent_mob_appareil` |
| **Clé d'appareil** | paire de clés générée dans le matériel sécurisé du téléphone (non exportable), liée à une session ; signe les requêtes sensibles et les lots ; détruite à la déconnexion ou à la purge | clé publique dans `ent_mob_appareil` |
| **Session mobile** | session de `ent-auth` de client `mobile`, liée à un appareil inscrit et à une casquette (357) | `ent_session` (+ `appareil_id`) |
| **Contexte** | établissement × casquette × (enfant pour une famille) : ce que l'écran montre | — |
| **Manifeste** | liste servie par le serveur des écrans, profils hors ligne, opérations et réglages ouverts à une session, avec la version minimale de l'appli | calculé |
| **Profil hors ligne** | déclaration d'un socle (§6.2) : quoi, combien de temps, qui, purge | code |
| **Bail** | autorisation datée de garder un lot de données d'un profil sur un appareil ; inscrit côté serveur ; expire, se rend, se purge | `ent_mob_bail` |
| **Opération différée** | geste fait hors ligne, signé, idempotent, appliqué au plus une fois par le socle propriétaire | `ent_mob_operation_recue` (transport) + table du socle (sens) |
| **Lot** | ensemble d'opérations envoyées ensemble, signé par la clé d'appareil | `ent_mob_lot_recu` |
| **Ordre de purge** | commande serveur qui fait détruire à l'appareil les clés et les données d'un périmètre ; livrée au prochain contact (et par notification silencieuse si possible) ; confirmée | `ent_mob_ordre_purge` |
| **Mode appareil partagé** | appareil de l'établissement inscrit par lui ; session personnelle courte par-dessus ; aucun bail personnel au-delà de la session | `ent_mob_appareil.partage` |
| **Version minimale** | version de l'appli en dessous de laquelle le serveur refuse les écritures (et, pour un correctif de sécurité, tout) | `ent_mob_version` |

---

## 8. Architecture des applis

### 8.1 Organisation (reprise de `ent-mobile-app`, complétée) [F + R]
| | Android | iOS |
|---|---|---|
| Cœur testable sans SDK | module `:core` (Kotlin JVM) | cible SwiftPM `ScolIACore` (compile sous Linux) |
| Interface | `:app` (Compose, Material 3) | `ScolIAUI` (SwiftUI) + `App/` (XcodeGen) |
| Extensions [R] | service de notifications (messages de données) | **Notification Service Extension** (aperçu C1 autorisé, §13), widget (plus tard, socle 29 P16) |
| Versions minimales | Android 9 (API 28) ; cible **API 36** (DA2) | iOS 17 [R : garder ; à revoir chaque été, R-MO31] |
| Dépendances autorisées | AndroidX, Compose, Credential Manager, Biometric, zxing (Apache-2.0) ; OkHttp | Foundation, SwiftUI, AuthenticationServices, LocalAuthentication, AVFoundation, CryptoKit |

### 8.2 Ce que fait chaque couche [R]
- **Cœur** : client API typé, contexte, manifeste, moteur hors ligne (baux, chiffrement, purge, file d'opérations), clé d'appareil (interface), rattachement, passkeys (pont JSON WebAuthn), mise à jour exigée, journal de diagnostic local. **Toute règle s'y teste sans appareil.**
- **Interface** : écrans de la coquille et des socles, composants partagés, accessibilité, aiguillage.
- **Rien de métier n'est calculé dans l'appli** hors de l'affichage (tri, filtre, regroupement) et des **pré-contrôles** de confort (un champ vide) que le serveur refait toujours.

### 8.3 Contrat avec l'API [R]
1. **DTO générés** depuis `ent-types` (Rust) : schéma JSON produit au build par le serveur, puis génération des classes Kotlin (`kotlinx.serialization`) et Swift (`Codable`) ; aucun DTO écrit à la main pour une route nouvelle ; les DTO actuels sont remplacés au fil des lots [R ; outil à choisir au lot MO1 après analyse de licence, sans dépendance cloud].
2. **Tests de contrat** (M11) : les fixtures des tests mobiles sont rejouées contre `ent-api` en CI (comme `contre_backend.rs` côté web) ; une modification de DTO qui casse une version d'appli encore supportée échoue en CI.
3. **Version d'API** : chaque requête porte `X-ScolIA-Appli: <plateforme>/<version>` ; le serveur garantit la compatibilité de **N-2 versions mineures** et refuse explicitement (`426`, message lisible, lien vers le magasin) une version sous le minimum (R-MO30).
4. **Lecture économe** : `ETag` / `If-None-Match` sur les lectures mises en cache, pagination par curseur, compression ; un seul appel « contexte » au démarrage (`GET /mobile/contexte` : moi, casquettes, manifeste, baux à renouveler, ordres de purge en attente, version minimale) au lieu de 3 à 5 (M8).

### 8.4 Un cœur Rust partagé ? (Q-MO1, non appliqué)
D-002 veut « tout l'ENT en Rust » ; D-003 fixe SwiftUI et Compose pour l'**interface**. Une option serait un **cœur Rust commun** aux deux applis (moteur hors ligne, chiffrement, signature des lots, DTO = `ent-types` eux-mêmes), exposé à Kotlin et Swift par une couche de liaison générée [H : outils de type UniFFI, licence MPL-2.0 à vérifier]. **Pour** : une seule implémentation des règles les plus délicates (baux, purge, conflits), testée en Rust avec le serveur, plus de dérive entre plateformes. **Contre** : chaîne de build plus lourde (cibles iOS et Android, taille du binaire), débogage croisé, remplacement d'un cœur Kotlin/Swift déjà testé, compétences. **Recommandation** : ne pas l'adopter par défaut ; garder les cœurs natifs (existant testé) et mesurer la dérive par les tests de contrat ; **étude bornée** (prototype du seul moteur hors ligne, une semaine) proposée à Florian avant le lot MO7. Ce point n'est pas tranché par D-031 parce qu'il change la façon de mettre en œuvre D-003 et introduit une dépendance structurante.

---

## 9. Rattachement, connexion, session

### 9.1 Premier lancement [R]
1. **Écran d'accueil** : « Rejoindre mon établissement » avec trois voies, dans cet ordre : **scanner le QR** (feuille de rentrée, invitation d'agent, affiche) ; **ouvrir le lien** reçu (lien universel / App Link `https://<etab>.scolia.fr/rejoindre?…`) ; **chercher** l'établissement par nom ou ville (annuaire public minimal servi par `connexion.scolia.fr`, sans donnée personnelle, M3) ; en secours, saisir l'adresse (`lpnb`, `lpnb.scolia.fr`).
2. **Vérification** : l'appli interroge le guichet central (pas la page ouverte) pour obtenir le **nom officiel, la commune et le logo** de l'établissement, et les affiche **avant** toute saisie (bloquant B, R-MO41) ; un hôte inconnu, un sous-domaine technique, un schéma personnalisé, un port ou des identifiants dans l'URL sont refusés (règles existantes, testées).
3. **Connexion** : passkey native si la personne en a une (rp `scolia.fr`) ; sinon parcours d'**activation** (code de rattachement ou invitation → création ou rattachement de l'identité par le socle 1, D-013, avec confirmation) puis **enrôlement immédiat** d'une passkey sur ce téléphone (D-009) ; mot de passe seulement en secours, suivi du second facteur.
4. **Personnes sans appareil compatible** (D-009 « solution de repli à décider ») [R, appliqué par défaut] : (a) passkey **sur un autre appareil** de la personne (clé de sécurité, ordinateur) ; (b) **passkey hybride** (le téléphone d'un proche n'est **pas** une solution : la passkey appartient à la personne) ; (c) à défaut, le compte reste en **web + code à usage unique par e-mail** avec durée web, et l'appli explique pourquoi elle ne peut pas ouvrir de session longue ; (d) l'accueil ou le secrétariat peut accompagner (socle 1, D-018). Le repli exact est une décision de sécurité du socle 1 (Q-MO2).
5. **Notifications** : la permission du système est demandée **après** la première connexion, au moment où l'appli explique ce qu'elle enverra (CNIL : au moment du besoin) ; refus respecté, rapporté au serveur (R-N57).

### 9.2 Plusieurs établissements
Un établissement de plus = refaire 1 à 3 avec la **même passkey** (rp unique `scolia.fr`, D-013) ; l'appli garde une session par établissement, chiffrée séparément ; la liste des établissements est **locale**. Le guichet central peut, après authentification, montrer **à la personne seule** ses établissements (`09_INTEROP.md` §10, point d'entrée sans établissement) pour éviter de refaire le rattachement [R]. Retirer un établissement : confirmation, révocation serveur au mieux, purge locale (existant).

### 9.3 Connexion fédérée (ÉduConnect, Guichet Agents, OIDC d'établissement)
Session d'authentification **du système** (`ASWebAuthenticationSession`, Custom Tabs), jamais une vue web intégrée ; PKCE vers le guichet central ; ticket à usage unique échangé contre la session mobile de l'établissement choisi (`09_INTEROP.md` §22). La fédération **authentifie** ; la passkey reste exigée selon la politique du socle 1 (Q-X1 du 9). Pour le public, ÉduConnect et le Guichet Agents deviennent obligatoires dans les délais de l'arrêté (§5.2).

### 9.4 Session longue liée à l'appareil [R]
- À la connexion mobile, l'appli génère une **clé d'appareil** (iOS : Secure Enclave, `kSecAttrTokenIDSecureEnclave`, accès `.userPresence` pour les opérations hors ligne ; Android : Keystore, StrongBox si présent, sinon TEE ; `setUserAuthenticationParameters` pour la clé du cache) et envoie la **clé publique** ; le serveur l'inscrit (`ent_mob_appareil`) et crée la session `mobile`.
- **Preuve de possession** : chaque requête qui écrit, chaque lot hors ligne, chaque renouvellement de bail porte une signature (horodatage serveur récent + méthode + chemin + empreinte du corps) par la clé d'appareil ; un jeton Bearer copié ailleurs **sans** la clé ne peut que lire, et seulement pendant 15 minutes après la dernière requête signée [R, R-MO5]. Format à fixer au lot MO4 [H : proche de DPoP, RFC 9449, à vérifier].
- **Durées** (D-011, existant) : 90 jours glissants, plafond 365 jours ; **durée mobile seulement si** assurance `passkey` (ou fédération + passkey) **et** clé d'appareil inscrite (DS2) ; sinon durée web.
- **Attestation** (App Attest, Play Integrity) : **signal** à l'inscription de l'appareil, jamais une condition bloquante seule (un téléphone sans services Google reste utilisable en durée web) ; elle fait transiter une demande chez Apple ou Google : à inscrire au registre (§15.4) [R, Q-MO4].
- **Déverrouillage** : à l'ouverture et après 5 minutes en arrière-plan (S-1101 : 1 à 15 min) ; biométrie ou code de l'appareil ; si l'appareil n'a **aucun** verrou configuré, l'appli refuse la session longue et tout profil hors ligne (R-MO9) — renversement de l'existant qui « déverrouille automatiquement » dans ce cas.
- **Révocation** : depuis l'appli (« Mes appareils »), le web, la direction (personne d'un seul établissement), la **révocation générale** (R-X25 du 9) ; effet : sessions, jeton de notification, baux et ordre de purge en **une transaction** (R-MO6).

### 9.5 Enrôlement bloquant, récupération
Portée `enrolement` (existant) : seul l'écran d'enrôlement est servi. Récupération en libre-service (D-018) : lancée depuis l'appli, poursuivie par e-mail avec délai ; pendant le délai, l'appli affiche « récupération en cours, annulable » ; les alertes de sécurité (socle 8, classe « sécurité ») arrivent sur **tous** les appareils encore actifs.

---

## 10. Contextes : établissements, casquettes, enfants

1. **Contexte = établissement × casquette × enfant.** Une session porte **une** casquette (357) ; une personne qui a deux casquettes dans un établissement (professeure **et** mère d'élève) a **deux sessions** dans l'appli, ouvertes par la même passkey, et un **sélecteur** « Professeure · Mère de Léo » (DA6). Aucune donnée de la casquette « parent » n'est montrée dans un écran « professeure », et inversement (le socle 6 a posé la règle ; l'appli ne mélange pas les caches : un bail par session).
2. **Plusieurs enfants** : sélecteur d'enfant en tête des écrans famille, droits **par enfant** (droits du lien du socle 4 : voir notes, justifier, autoriser, recevoir bulletins…) ; « Aujourd'hui » de toute la fratrie (29, POR-175).
3. **Plusieurs établissements** : vue réunie **sur l'appareil** (29 §11) avec une étiquette d'établissement par ligne ; un conflit d'horaire entre deux établissements (professeur partagé) n'est signalé que sur l'appareil (CPO-31 du 29).
4. **Élève majeur, élève qui devient majeur** : à la date de majorité, le socle 4 recalcule les droits des responsables ; l'appli des parents perd ce qui n'est plus permis au **prochain contact** (manifeste), avec purge des baux concernés et un message neutre (« L'accès à certaines informations de Hugo a changé ») ; l'appli de l'élève lui montre ses nouveaux réglages (socle 4) [R, R-MO14].
5. **Parent interdit, lien suspendu, parent sans exercice de l'autorité** : la coupure suit le socle 4 ; l'appli n'affiche jamais la cause (« Cet accès n'est plus disponible. Contactez l'établissement. ») ; purge en moins d'une minute quand l'appareil est joignable (R-MO42, R-A10 du 4).
6. **Mode discret** (`29_PORTAILS.md` §14.5) : disponible sur le mobile (prénoms seulement, pas de photo) pour l'usage en public ; les alertes de restriction de remise restent visibles sous forme neutre (R-PO35).

---

## 11. La coquille et ses écrans

### 11.1 Aiguillage (existant, complété)
Pas d'établissement → rattachement ; pas de session → connexion ; version sous le minimum → écran de mise à jour ; session mais appli verrouillée → déverrouillage ; portée `enrolement` → enrôlement ; ordre de purge en attente → purge puis message ; sinon : **Aujourd'hui** du contexte courant.

### 11.2 Manifeste servi par le serveur [R]
`GET /mobile/contexte` renvoie, pour la session : casquette, enfants (famille), **écrans** (codes déclarés par les socles, ouverts par une capacité effective), **profils hors ligne** permis et leurs durées effectives, **opérations** permises, réglages de la politique de l'établissement, version minimale, ordres de purge, baux à renouveler. Le calcul utilise le même moteur de droits que le web (D-017) ; la liste d'écrans de l'appli n'est plus calculée dans `Capacites` (qui devient un simple lecteur). Un écran inconnu de la version de l'appli est ignoré ; un écran attendu mais absent du manifeste n'est pas affiché. Les quatre premiers onglets sont choisis par le serveur selon la casquette (professeur : Aujourd'hui, Appel, Messages, EDT ; famille : Aujourd'hui, Absences, Messages, Notes ; élève : Aujourd'hui, EDT, Devoirs, Messages), le reste dans « Plus » (menu maison sur les deux plateformes, DA8).

### 11.3 Écrans de la coquille (propres au socle 11)
| Écran | Contenu |
|---|---|
| Rattachement | QR, lien, recherche, saisie ; confirmation de l'établissement (nom, commune, logo) |
| Connexion, enrôlement, récupération | passkey native, secours, état d'une récupération |
| Déverrouillage | biométrie / code |
| Aujourd'hui | composé par le socle 29 (tuiles), rendu natif ; bandeau « hors ligne, à jour à 7 h 42 » |
| Sélecteur de contexte | établissements, casquettes, enfants |
| Centre de notifications | avis du socle 8 (lus, non lus), filtre par catégorie ; « pourquoi ai-je reçu ceci ? » (COM-11) |
| Hors ligne et envois | données présentes sur le téléphone (profils, jusqu'à quand), lots en attente, conflits, « tout effacer maintenant » |
| Mon compte | identité affichée, passkeys, **mes appareils** (révoquer, nommer), sessions, langue, récupération |
| Réglages | taille et contraste (en plus du système), aperçus de notifications, mode discret, délai de verrouillage (dans la politique), langue, sons |
| Aide | foire aux questions, contact de l'établissement (pas un numéro personnel), « envoyer un diagnostic » (§19.6) |
| À propos | version, mentions, notice d'information RGPD (par établissement), **état de conformité d'accessibilité**, licences des composants, politique de divulgation des vulnérabilités |
| Fermeture de compte | explication et demande à l'établissement (§5.5) |
| Mise à jour exigée | version minimale, lien vers le magasin, ce qui reste possible (lecture si ce n'est pas un correctif de sécurité) |

### 11.4 Composants communs fournis aux socles [R]
Liste d'élèves (photo si permise, prénom d'usage, état), grille d'appel générique (avec le socle 12, `assiduite::composant_appel`), sélecteur de séance du jour, appareil photo de pièce (recadrage, **retrait des métadonnées**, conversion HEIC → JPEG, compression, socle 7), signature par passkey native (socle 7), lecteur de QR (rattachement, trousses, carte d'élève), boîte de confirmation « ★ » (D-003), bandeau d'état hors ligne, champ de dictée locale (§20), écrans d'erreur, de refus (403 propre), de conflit de version.

---

## 12. Hors ligne : baux, profils, opérations

### 12.1 Pourquoi un bail [R]
Un cache sans bail est une copie qui vit sa vie (DA1). Le bail fait trois choses : il **borne** (date de fin, profil, appareil, session), il **inventorie** (le serveur sait ce qui est sorti, pour qui, combien de personnes, quelle sensibilité), il **rend la purge vérifiable** (l'appareil confirme la destruction ; à défaut, l'expiration suffit parce que la clé de chiffrement elle-même expire).

### 12.2 Profils hors ligne déclarés par les socles (plafonds) [R, à confirmer par chaque socle]
| Profil | Socle | Contenu (minimal) | Plafond | Purge en plus de l'expiration |
|---|---|---|---|---|
| `aujourdhui.lecture` | 29 | dernière version de l'accueil et de l'à-faire | 24 h sans contact ; jamais l'EDT au-delà du jour pour un responsable (R-PO33) | lien suspendu, retrait, révocation |
| `edt.jour` | 10 | séances du jour et du lendemain de la personne (élève, personnel) | fin du lendemain | changement de version publiée |
| `appel.jour` | 12 | séances du jour, effectifs minimaux (nom d'usage, prénom, identifiant, photo si visible), situations connues | **minuit** (R-AS32) | fin de fonction, révocation |
| `appel.activite` | 12, 41, 42 | liste d'une activité (étude, internat, sortie) | validation + 12 h (41) ; retour + 24 h (42) | clôture |
| `sortie.urgence` | 34, 42 | fiches d'urgence des participants (C3 minimal, déverrouillage à chaque ouverture) | retour + 24 h ou clôture (SEJ-08) | décision de la direction (R-MO43) |
| `accueil.secours` | 4, 14 | interdictions et autorisés du jour, décisions de sortie précalculées | minuit | pose ou levée d'interdiction (mise à jour < 1 min, R-A10) |
| `surete.comptage` | 14 | liste de comptage de la zone | fin d'alerte + 24 h | — |
| `notes.saisie` | 21, 22 | une évaluation en cours de saisie (élèves du groupe, version) | 72 h | arrêt de période |
| `messages.brouillon` | 16 | brouillons de la personne (texte seul) | 7 jours | envoi, révocation |
| `cahier.brouillon` | 20 | brouillon de fin de séance | 7 jours | publication |
| `contexte` | 11 | moi, casquette, manifeste (pour le démarrage à froid, DA5) | durée de la session | révocation |

Règles communes : **aucune** coordonnée de famille hors fiche de secours de l'accueil (R-AS11) ; **aucune** donnée de santé hors `sortie.urgence` ; **aucun** motif, historique ou texte libre d'un tiers ; chaque profil liste ses champs, un champ non listé est **retiré par le serveur** avant l'envoi (test de schéma). L'établissement peut **raccourcir** un plafond (S-1102), jamais l'allonger.

### 12.3 Chiffrement et purge
Une **clé de données par bail** (AES-256-GCM), elle-même chiffrée par la clé d'appareil liée à la session et, pour les profils marqués « déverrouillage », exigeant la présence de l'utilisateur ; expiration = la clé n'est plus déchiffrable (vérification de date serveur dernière connue + horloge monotone de l'appareil, R-MO11) ; purge = destruction de la clé puis des fichiers (effacement cryptographique). Fichiers dans le stockage privé de l'appli, exclus des sauvegardes, noms hachés (existant).

### 12.4 Opérations différées
Reprend `12_ASSIDUITE.md` §9.4 et le généralise : `operation_id` (UUID tiré sur l'appareil), type déclaré, objet, champs, **heure de l'appareil** et **heure serveur de référence** (dernière connue + écart monotone), version de base ; journal local append-only ; **lot** signé par la clé d'appareil ; à la réception : signature vérifiée, appareil et session actifs, **droits relus à la date de l'objet**, fenêtre d'acceptation (S-469 : jour même et veille pour l'appel), idempotence par `operation_id` (registre du socle 11), puis remise au **validateur du socle** qui applique, applique avec mention, met en conflit ou refuse. L'appareil reçoit le résultat ligne par ligne et l'affiche. **Rien n'est rejoué en silence** pour une action qui n'est pas une opération déclarée (R-PO20).

### 12.5 Horloge
L'heure de l'appareil n'est jamais une preuve. Chaque opération garde les deux heures ; un écart de plus de 5 minutes avec l'heure serveur est signalé dans le résultat (« heure du téléphone décalée de 47 min ») et l'heure retenue est l'heure serveur de référence corrigée par l'horloge monotone [R, R-MO12].

### 12.6 Ce que voit la personne
Bandeau « hors ligne, à jour à HH:MM » ; « 3 opérations à envoyer » ; écran « Hors ligne et envois » ; à l'expiration : « Les données de l'appel d'aujourd'hui ont été effacées de ce téléphone » ; aucune donnée expirée n'est montrée « faute de mieux ».

### 12.7 Perte ou vol d'un appareil [R]
1. La personne (ou la direction pour elle) déclare la perte depuis un autre appareil ou le web : révocation et ordre de purge immédiats (MOB-196).
2. Le serveur produit l'**inventaire des baux actifs** de l'appareil au moment de la dernière synchronisation : profils, nombre de personnes concernées, classes de sensibilité, dates de fin, état de verrouillage déclaré par l'appareil.
3. Un **compte rendu d'incident** pré-rempli est proposé au DPO (socle 27) : nature, date, données exposées **chiffrées** (clé liée au matériel et au déverrouillage), probabilité de risque ; la décision de notifier la CNIL (72 h) reste humaine.
4. Si l'appareil reprend contact, la purge est exécutée et confirmée ; la confirmation est jointe à l'incident.

---

## 13. Notifications reçues

1. **Transport** : APNs pour iOS (seul transport possible pour une appli de l'App Store [H]), FCM HTTP v1 pour Android ; l'envoi appartient au socle 8 (N7). **UnifiedPush** pour les téléphones Android sans services Google : plus tard, sur demande (S-1104) [R].
2. **Contenu** : enveloppe opaque du socle 8 (titre = nom de l'établissement ou « ScolIA », corps neutre, identifiant d'avis) ; l'extension (iOS) ou le service (Android) **récupère l'aperçu autorisé** (C1 seulement, si la personne a activé les aperçus et si l'appareil est déverrouillé) avec la session de l'appareil ; à défaut, l'enveloppe seule s'affiche [R, H technique à valider au lot MO6].
3. **Catégories et canaux** : un canal Android par catégorie du socle 8 (absences, messages, travail, rendez-vous, sécurité du compte, urgences), réglables dans le système ; iOS : catégories et niveaux d'interruption (urgence = « time-sensitive » ; **alertes critiques** seulement si Apple les accorde, Q-N8 du 8).
4. **Actions depuis la notification** : ouvrir l'écran exact (lien profond vers l'avis) ; **aucune** action métier exécutée depuis l'écran verrouillé (pas de « Justifier » en un geste sans déverrouillage).
5. **Notifications silencieuses** : invalidation de cache et ordre de purge (R-A10) ; non garanties par les systèmes (limitées, différées) [H] : la purge repose **d'abord** sur l'expiration du bail et le prochain contact.
6. **Pastille** = compteur d'à-faire du socle 29 (somme des établissements calculée sur l'appareil).
7. **Autorisation** rapportée à chaque ouverture (R-N57) ; « vous ne recevez pas nos notifications sur ce téléphone » affiché.
8. **Élèves pendant les cours** : le serveur (socle 8) retient les notifications non urgentes d'un élève pendant ses séances du jour (EDT du socle 10) et les remet à la fin des cours ou à la récréation suivante selon le réglage S-1108 (R-MO22).

---

## 14. Sécurité de l'appareil [R]

| Sujet | Règle |
|---|---|
| Stockage | jeton : Keystore / Keychain `ThisDeviceOnly` (existant) ; clé d'appareil non exportable ; caches sous bail ; rien dans les préférences en clair ; aucune donnée dans les journaux système |
| Réseau | HTTPS seul, autorités du système, aucune CA utilisateur, aucune redirection suivie, délais courts ; **pas d'épinglage de certificat** au départ (risque de couper toutes les applis à un renouvellement) ; surveillance de la transparence des certificats côté exploitation [R, Q-MO5] |
| Captures et aperçus | Android `FLAG_SECURE` sur les écrans nominatifs (existant : partout) ; iOS : contenu masqué hors premier plan, avertissement discret à la capture sur les écrans sensibles (iOS ne permet pas d'interdire la capture [H]) |
| Presse-papiers | copie interdite des données nominatives sensibles ; presse-papiers local seulement (pas de copie universelle) |
| Clavier | champs sensibles sans apprentissage ni suggestions (mot de passe, code) |
| Appareil modifié (jailbreak, root) | **signal** remonté et affiché à la personne ; pas de blocage (faux positifs, accessibilité) ; profils hors ligne C2+ refusés sur un appareil signalé (S-1105) |
| Liens | rattachement strict (existant) ; tout autre lien externe s'ouvre dans le navigateur du système |
| Vue web | aucune vue web pour l'authentification ; médiacentre et contenus externes dans le navigateur du système (09 §22) |
| Mises à jour de sécurité | version minimale servie par le serveur ; correctif imposé en 48 h (R-MO30) |
| Revue | grille MASVS à chaque lot de sécurité (MO0, MO4, MO7, MO8) + revue `security-review` |

---

## 15. Confidentialité et minimisation

### 15.1 Permissions, demandées au moment du besoin
Appareil photo (QR, pièce) ; photos via le **sélecteur du système** (aucun accès à la photothèque entière) ; micro (dictée locale, à la demande) ; notifications ; biométrie. **Jamais** : contacts, position (aucune géolocalisation, même pour les sorties : le point de situation est déclaratif, `42`), Bluetooth, calendrier en écriture (l'abonnement iCal passe par le lien `webcal` du socle 9), suivi publicitaire.

### 15.2 Photos et pièces
Métadonnées EXIF (dont la position GPS) **retirées sur l'appareil** avant l'envoi ; HEIC converti ; taille bornée ; aperçu avant envoi ; la photo n'est jamais conservée dans la galerie par l'appli.

### 15.3 Ce que l'appli ne fait pas
Aucune mesure d'audience, aucun SDK tiers, aucun rapport de plantage externe (le diagnostic est envoyé **à la demande** de la personne, sans donnée personnelle, §19.6), aucune publicité, aucune vente, aucun profilage ; aucune donnée d'un établissement envoyée à un autre.

### 15.4 Tiers techniques (registre de traitement)
Apple (App Store, APNs, passkeys synchronisées iCloud si la personne les utilise, App Attest), Google (Play, FCM, Credential Manager / services Play, Play Integrity) : enveloppes et métadonnées techniques seulement, hors UE [H, à qualifier par le DPO, Q-MO7] ; inscrits au registre (socle 27, NTF-78 du 8).

### 15.5 Fiches des magasins
Étiquettes de confidentialité (Apple) et « sécurité des données » (Google) rédigées depuis ce §15 et revues à chaque version qui change une permission (MOB-248).

---

## 16. Élèves, téléphone à l'école, mineurs

1. **Hors temps scolaire d'abord** (L511-5, loi du 24/08/2026) : l'appli élève met en avant le soir (« pour demain »), le matin avant 8 h (« ta journée »), les vacances (travail) ; **aucune fonction élève n'exige le téléphone pendant la journée** ; la carte d'élève et le contrôle de sortie s'appuient d'abord sur la **carte imprimée** (C-1).
2. **Pas de sollicitation pendant les cours** (R-MO22, §13 point 8) ; urgences exceptées (alerte, mise à l'abri : l'appli du personnel porte le comptage, pas celle de l'élève).
3. **Compte de l'élève à l'élève** : un parent ne se connecte jamais « comme » son enfant ; sur un téléphone partagé, chaque compte a son verrou (R-MO40) ; le parent n'a pas accès aux messages de l'élève avec la CPE (socle 16, D3).
4. **Majorité** (§10 point 4) ; **élève de moins de 15 ans** : pas de fonction fondée sur le consentement ; mentions d'information rédigées pour son âge (FALC) [R].
5. **Téléphone confisqué** (règlement intérieur) : l'appli verrouillée ne montre rien sur l'écran verrouillé ; aucun code de déverrouillage ne peut être exigé par l'établissement [V~ : L511-5 ne prévoit que la confiscation et la restitution].

---

## 17. Appareils partagés et appareils de l'établissement

1. **Inscription par l'établissement** (`mobile.appareils_etab.inscrire`, ACS-320 du 14) : nom du poste (« Accueil 1 », « iPad 14 »), lieu, usage (accueil, portail, internat, classe, service AED) ; par QR d'inscription à usage unique ou par la **gestion de flotte** (MDM) de l'établissement qui pousse une configuration gérée (établissement, mode partagé) [R : configuration gérée standard iOS et Android Enterprise, H sur le détail].
2. **Session personnelle par-dessus** : la personne s'identifie par passkey (de son téléphone en hybride, ou clé de sécurité, ou passkey du poste si l'établissement l'autorise) ; session **courte** (S-1106 : 15 min d'inactivité pour l'accueil, fin de service pour l'internat) ; changement de personne en un geste ; aucun bail personnel ne survit à la session.
3. **Données du poste** : seuls les profils **du poste** (fiche de secours de l'accueil, décisions de sortie du portail) y vivent, sous bail de l'appareil, effacés à minuit (R-AS11).
4. **iPad de classe** (élèves) : session élève courte, aucun cache, déconnexion à la fin de la séance (EDT) ; ou hors périmètre de l'appli si l'établissement utilise une autre solution de classe (Q-MO11).
5. **Téléphone de service** (AED, internat) : mode partagé ; la personne de service est connue à chaque instant (journal des sessions du poste).

---

## 18. Accessibilité, langues, ergonomie

- **RGAA / RAAM** : audit avant le doublon (MO10), état de conformité dans l'appli (À propos), déclaration publiée par l'éditeur, schéma pluriannuel (§5.3) ; tests TalkBack et VoiceOver réels (non faits à ce jour) ; tailles de texte du système jusqu'aux plus grandes, contrastes AA, aucun sens porté par la couleur seule (états d'appel : texte + forme + couleur), cibles ≥ 48 dp / 44 pt, gestes simples (pas de geste à plusieurs doigts indispensable), réduction des animations respectée, retour haptique sur la validation d'un appel.
- **Langues** : français par défaut ; langues des familles (anglais, arabe, espagnol, portugais, turc, ukrainien… selon l'établissement, Q-PO6 du 29) ; **droite-gauche** supporté ; textes juridiques et décisions en français avec mention traduite (29 §14) ; langue de l'appli indépendante de la langue du téléphone si la personne le souhaite.
- **Ergonomie de terrain** : utilisation à une main, gros boutons pour l'appel en extérieur (EPS, sortie), mode sombre, lisibilité au soleil (contraste renforcé), état « enregistré sur le téléphone » toujours visible.

---

## 19. Distribution, versions, support

### 19.1 Comptes et fiches
Un compte développeur **d'organisation** par magasin ; une appli « ScolIA » par magasin (`fr.scolia.mobile`) ; captures et textes en français et anglais ; comptes de démonstration pour la revue d'Apple (labo, données fictives) ; classification d'âge correcte ; l'appli ne vise pas la catégorie enfants [R, Q-MO6].

### 19.2 Pistes de diffusion
Labo (build debug), **pilote** (TestFlight, test interne Google Play), production. Distribution gérée (MDM, applis personnalisées) pour les appareils de l'établissement [H].

### 19.3 Versions
Version mineure mensuelle [H] ; correctif de sécurité **hors cycle** ; compatibilité serveur N-2 ; **version minimale** par plateforme (`ent_mob_version`), relevée par l'éditeur ; un correctif de sécurité peut **bloquer** une version (lecture et écriture) après 48 h d'avis (R-MO30) ; les versions du système supportées sont revues chaque été (iOS N-2, Android 9 tant que Credential Manager le permet, R-MO31).

### 19.4 Signature et clés
Clés de signature des applis et clés APNs / FCM inscrites à l'**inventaire des clés** (E11) avec séquestre (D-042) ; « signature d'appli par Google Play » acceptée pour Android (Google détient la clé de diffusion, ScolIA la clé d'envoi) [R] ; empreintes déclarées dans `assetlinks.json`.

### 19.5 Cyber-résilience (§5.6)
Politique de divulgation coordonnée publiée (`security.txt` sur `scolia.fr`), procédure 24 h / 72 h, SBOM à chaque version (Gradle et SwiftPM), vérification des licences (aucune AGPL, `cargo deny` équivalent pour Gradle et SwiftPM), période de support annoncée (5 ans par version majeure ? Q-MO10), mises à jour de sécurité séparées des nouveautés quand c'est possible.

### 19.6 Support et diagnostic
« Envoyer un diagnostic » : version, système, modèle (générique), état du rattachement (établissements en clair **seulement** pour l'établissement destinataire), derniers codes d'erreur, état des baux (profils, dates) — **aucune** donnée d'élève, aucun jeton ; envoyé à l'établissement choisi (et à l'éditeur si l'établissement l'escalade) ; journal local tournant de 7 jours, effacé à la déconnexion [R].

---

## 20. IA

- L'appli est **un client des capacités IA** comme le web (`IA_COEUR.md` §3) : propositions affichées, confirmation par la personne, aucun droit propre ; aucune clé ni appel de modèle depuis l'appli (IA-P10).
- **Dictée** (MSG-152, CDT-122) : reconnaissance vocale **sur l'appareil** (moteurs du système en mode local) ; si le système ne garantit pas le traitement local, la dictée est désactivée pour les comptes de l'établissement [R, H technique] ; jamais pour un texte de santé.
- **Aide à la connexion et au support** : réponses de la foire aux questions par l'assistant du socle 30 côté serveur, sans donnée personnelle [R, plus tard].
- **Jamais** : IA sur l'appareil qui lirait les caches, classement automatique d'une photo de justificatif comme « médical », résumé d'un fil sensible dans une notification.

---
## 21. Actions (MOB-100 et suivantes)

Acteurs : abréviations du §3 ; TOUS = toute personne connectée ; SYS = le système ; ★ = confirmation forte (passkey récente). Les actions métier des écrans mobiles appartiennent à leurs socles (§27.3) ; celles-ci sont les actions de la coquille et du kit.

### 21.1 Installer et rejoindre
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-100 | Installer l'appli et voir l'écran d'accueil sans compte (aucune donnée envoyée avant le rattachement) | TOUS | R-MO1 |
| MOB-101 | Scanner le **QR** d'une feuille de rentrée, d'une invitation ou d'une affiche | TOUS | rattachement strict ; R-MO41 |
| MOB-102 | Ouvrir un **lien d'invitation** (lien universel / App Link) | TOUS | idem |
| MOB-103 | **Chercher** l'établissement par nom ou ville | TOUS | annuaire public du guichet central, sans donnée personnelle (M3) |
| MOB-104 | Saisir l'adresse de l'établissement | TOUS | secours |
| MOB-105 | Voir le **nom officiel, la commune et le logo** confirmés par le guichet central avant toute saisie | TOUS | R-MO41 |
| MOB-106 | **Activer** un compte avec un code de rattachement (famille, élève) | FAM, ELV, MAJ | socle 9 ; aucune création de personne (D-013) |
| MOB-107 | Accepter une **invitation d'agent** (QR 10 min, lien 7 jours) | personnels | 357 |
| MOB-108 | Rattacher une **identité existante** (même passkey) à un nouvel établissement, avec confirmation | TOUS | D-013 |
| MOB-109 | Voir, depuis le guichet central, **ses** établissements après authentification | TOUS | 09 §10 |
| MOB-110 | **Retirer** un établissement de l'appli (révocation, purge, e-mail oublié) | TOUS | existant ; R-MO6 |
| MOB-111 | Comprendre pourquoi un rattachement échoue (code expiré, déjà utilisé, mauvais établissement) | TOUS | messages typés, sans révéler l'existence d'un compte |
| MOB-112 | Signaler un **QR ou un lien suspect** | TOUS | R-MO41, M-1104 |
| MOB-113 | Être rattaché à un établissement par la **gestion de flotte** (appareil de l'établissement) | REF | §17 |
| MOB-114 | Voir la **notice d'information** de l'établissement avant de continuer | TOUS | socle 27 |

### 21.2 Se connecter, sessions
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-115 | Se connecter par **passkey native** | TOUS | D-009, D-013 ; rp vérifié avant l'appel système |
| MOB-116 | Se connecter par **mot de passe** en secours, puis second facteur | TOUS | aucun jeton avant la fin |
| MOB-117 | Se connecter par **ÉduConnect / Guichet Agents / OIDC d'établissement** (session du système, PKCE) | TOUS | 09 §22 |
| MOB-118 | **Enrôler** une passkey sur ce téléphone (portée `enrolement`) | TOUS | bloquant tant que non fait |
| MOB-119 | Ajouter une autre passkey (clé de sécurité, ordinateur) | TOUS | D-009 |
| MOB-120 | Générer et inscrire la **clé d'appareil** à la connexion | SYS | R-MO4, R-MO5 |
| MOB-121 | Obtenir une **session longue** (passkey + clé d'appareil) ou une session courte expliquée | SYS | DS2, R-MO3 |
| MOB-122 | **Déverrouiller** l'appli (biométrie, code) | TOUS | R-MO9 |
| MOB-123 | Prolonger la session à l'usage, dans le plafond | SYS | D-011 |
| MOB-124 | Se **déconnecter** (révocation serveur, purge locale, même hors ligne) | TOUS | R-MO6 |
| MOB-125 | Voir **mes sessions et mes appareils** ; nommer un appareil | TOUS | — |
| MOB-126 | **Révoquer** un autre appareil ★ | TOUS | R-MO6 |
| MOB-127 | Lancer une **récupération** de passkey (D-018) et voir son état | TOUS | délai, annulable |
| MOB-128 | **Annuler** une récupération que je n'ai pas demandée | TOUS | alerte sur tous les appareils |
| MOB-129 | Recevoir les **alertes de sécurité** du compte (nouvel appareil, passkey ajoutée, récupération) | TOUS | socle 8, classe « sécurité », non désactivable |
| MOB-130 | Être **renvoyé à la connexion** proprement sur un 401 (purge, message) | SYS | existant |
| MOB-131 | Refaire une **authentification forte** pour une action ★ (passkey < 5 min) | TOUS | D-041, `IA_COEUR.md` §6 |
| MOB-132 | Ouvrir une session sur un appareil **sans verrou** : refusé, explication | SYS | R-MO9 |
| MOB-133 | Voir l'écran « **mise à jour exigée** » | TOUS | R-MO30 |
| MOB-134 | Voir pourquoi la session est courte (pas de passkey, appareil non attesté, politique) | TOUS | R-MO3 |

### 21.3 Contextes
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-135 | Changer d'**établissement** | TOUS | une session par établissement |
| MOB-136 | Changer de **casquette** (professeure / mère) | personnels parents | 357 ; deux sessions, deux caches |
| MOB-137 | Changer d'**enfant** | FAM | droits par lien |
| MOB-138 | Voir **Aujourd'hui** réuni de tous ses établissements et enfants | FAM, PRF partagé | réunion sur l'appareil (29) |
| MOB-139 | Voir le **conflit horaire** entre deux établissements (sur l'appareil seulement) | PRF partagé | CPO-31 |
| MOB-140 | Activer le **mode discret** | TOUS | 29 §14.5, R-PO35 |
| MOB-141 | Voir ce qui change à la **majorité** (élève, parents) | MAJ, FAM | R-MO14 |
| MOB-142 | Voir un accès **qui n'est plus disponible** (lien suspendu, fin de scolarité) sans la cause | FAM, ELV | R-MO42 |
| MOB-143 | Voir la **fin annoncée** de son accès (élève parti, personnel en fin de contrat) | ELV, personnels | 3 mois (arrêté 2006) |
| MOB-144 | Garder ses établissements dans un **ordre** choisi | TOUS | local |

### 21.4 Coquille et navigation
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-145 | Charger le **contexte** en un appel (moi, casquette, manifeste, baux, purges, version) | SYS | M8 |
| MOB-146 | Afficher les **onglets** choisis par le serveur, le reste dans « Plus » | SYS | §11.2 |
| MOB-147 | Ouvrir un écran depuis un **lien profond** (notification, e-mail, QR) | TOUS | contrôle de contexte avant affichage |
| MOB-148 | **Rechercher** (élèves, personnes, pages) selon ses droits | personnels | socle 29 |
| MOB-149 | Voir un **refus** propre (403) sans écran cassé | SYS | R-MO2 |
| MOB-150 | Voir un **conflit de version** (« modifié par X ») | SYS | — |
| MOB-151 | Tirer pour **actualiser** ; voir l'heure de la dernière mise à jour | TOUS | — |
| MOB-152 | Utiliser le **composant photo de pièce** (recadrage, métadonnées retirées, conversion) | TOUS | §15.2 |
| MOB-153 | **Signer** par passkey native (socle 7) | TOUS | — |
| MOB-154 | **Confirmer** une action ★ ou un envoi (récapitulatif) | TOUS | D-003 |
| MOB-155 | Lire le **QR** d'une trousse, d'une carte d'élève, d'un poste | personnels | 14, 34, 42 |
| MOB-156 | Voir l'**aide** contextuelle d'un écran | TOUS | — |
| MOB-157 | Revenir où l'on était après un déverrouillage ou une reconnexion, **sans perdre une saisie** | TOUS | D-011 |
| MOB-158 | Ouvrir sur le **web** une tâche réservée au web (lien vers la bonne page, session web demandée) | TOUS | D-003 |
| MOB-159 | Voir les **nouveautés** de la version (une fois) | TOUS | — |

### 21.5 Notifications
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-160 | Autoriser les notifications **après** explication | TOUS | §9.1 point 5 |
| MOB-161 | Inscrire le **jeton** de l'appareil auprès du socle 8, lié à la session | SYS | DS1, R-MO16 |
| MOB-162 | **Rapporter l'autorisation** du système à chaque ouverture | SYS | R-N57 |
| MOB-163 | Recevoir une **enveloppe** et, si permis, son aperçu C1 | TOUS | §13 |
| MOB-164 | Choisir les **aperçus** (aucun, sur appareil déverrouillé seulement) | TOUS | défaut : déverrouillé seulement |
| MOB-165 | Régler les **canaux** par catégorie (Android) / catégories (iOS) | TOUS | planchers du socle 8 |
| MOB-166 | Ouvrir l'**avis** exact depuis la notification | TOUS | MOB-147 |
| MOB-167 | Voir le **centre de notifications** (lus, non lus, filtres) | TOUS | socle 8 |
| MOB-168 | Savoir **pourquoi** j'ai reçu un avis | TOUS | COM-11 |
| MOB-169 | Voir « vous ne recevez pas nos notifications sur ce téléphone » | TOUS | R-N57 |
| MOB-170 | Recevoir une **alerte d'urgence** (son, time-sensitive, critique si accordée) | personnels, FAM | socle 8 U3, 14 |
| MOB-171 | Voir, en écrivant pendant la plage calme, l'**heure de visibilité** et le rappel du droit à la déconnexion | émetteurs | arrêté 7/09/2026 |
| MOB-172 | **Désinscrire** le jeton à la déconnexion | SYS | DS4 |
| MOB-173 | Recevoir une **notification silencieuse** d'invalidation ou de purge | SYS | §13 point 5 |
| MOB-174 | Voir la **pastille** de l'icône = à-faire réuni | SYS | 29 |

### 21.6 Hors ligne
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-175 | Recevoir un **bail** d'un profil (préparation du matin, ouverture d'un appel, départ d'une sortie) | SYS | §12 |
| MOB-176 | **Renouveler** un bail à l'ouverture si les données ont changé | SYS | ETag |
| MOB-177 | **Lire** une donnée sous bail hors réseau, avec son heure | TOUS | R-MO10 |
| MOB-178 | Ne **plus** lire une donnée expirée (message) | SYS | R-MO11 |
| MOB-179 | **Rendre** un bail (fin de l'usage) | SYS | — |
| MOB-180 | Exécuter un **ordre de purge** et le confirmer | SYS | R-MO7 |
| MOB-181 | **Enregistrer une opération** hors ligne (appel, comptage, remise, positionnement) | personnels | R-MO19 |
| MOB-182 | Voir « enregistré sur le téléphone, non envoyé » et le nombre d'opérations | personnels | — |
| MOB-183 | **Envoyer** un lot au retour du réseau (automatique) | SYS | signé, idempotent |
| MOB-184 | **Renvoyer** manuellement un lot | personnels | AS-223 |
| MOB-185 | Voir le **résultat** ligne par ligne (appliquée, compatible, conflit, refusée) | personnels | — |
| MOB-186 | Voir et **suivre** un conflit (tranché par le socle propriétaire) | personnels | AS-134 |
| MOB-187 | Être prévenu d'une **heure de téléphone décalée** | SYS | R-MO12 |
| MOB-188 | Voir **ce qui est sur le téléphone** (profils, jusqu'à quand) | TOUS | — |
| MOB-189 | **Tout effacer** maintenant (données hors ligne) | TOUS | opérations non envoyées : avertissement explicite |
| MOB-190 | Démarrer **à froid hors ligne** (contexte sous bail) | TOUS | DA5 |
| MOB-191 | Ouvrir un profil marqué « déverrouillage » (présence exigée) | personnels | R-AS38 |
| MOB-192 | Garder un **brouillon** (message, cahier) et l'envoyer au retour du réseau **après relecture** | personnels | R-PO20 |
| MOB-193 | Voir qu'une opération a été **refusée** hors fenêtre et que faire | personnels | S-469 |
| MOB-194 | Recevoir la **fiche de secours** du poste (appareil partagé) | ACC | 14 |

### 21.7 Appareils et sécurité
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-195 | Voir ses appareils et leur **dernier contact** | TOUS | — |
| MOB-196 | **Déclarer un appareil perdu ou volé** (depuis un autre appareil ou le web) ★ | TOUS ; DIR pour un agent | §12.7 |
| MOB-197 | Ordonner la **purge** d'un appareil ou de tous les appareils d'une personne dans l'établissement | DIR, ADM | `mobile.purge.ordonner` |
| MOB-198 | Voir l'**inventaire des baux** d'un appareil perdu | DIR, DPO | R-MO8 |
| MOB-199 | Recevoir le **compte rendu d'incident** pré-rempli | DPO | §12.7 |
| MOB-200 | Voir le **signal** d'appareil modifié et ses effets | TOUS | §14 |
| MOB-201 | Être averti d'une **capture d'écran** sur un écran sensible (iOS) | TOUS | §14 |
| MOB-202 | Voir l'**attestation** de son appareil (niveau) si elle existe | TOUS | Q-MO4 |
| MOB-203 | Voir qu'un **jeton de notification** a été déplacé vers un autre compte sur ce téléphone | TOUS | R-MO17 |
| MOB-204 | Changer de téléphone : **réinscrire** sans perdre ses établissements (nouvelle passkey ou passkey synchronisée), ancien appareil révoqué | TOUS | — |
| MOB-205 | Faire réparer ou **vendre** son téléphone : « préparer l'appareil » (tout révoquer, tout purger) | TOUS | — |
| MOB-206 | Voir l'état « **liste non vérifiée depuis HH:MM** » d'un profil critique | ACC, AED | R-A10 |
| MOB-207 | Bloquer l'ouverture des profils C2+ sur un appareil signalé | SYS | S-1105 |
| MOB-208 | Exiger une **passkey récente** pour ouvrir une fiche d'urgence | ENC | 34 |
| MOB-209 | Purger les données d'**un établissement** à la fin de l'adhésion | SYS | `etablissement.retire` |

### 21.8 Appareils partagés et de l'établissement
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-210 | **Inscrire** un appareil de l'établissement (QR à usage unique) | REF, ADM | ACS-320 |
| MOB-211 | Inscrire en série par la **gestion de flotte** (configuration gérée) | REF | §17 |
| MOB-212 | Nommer le poste, son lieu, son usage | REF | — |
| MOB-213 | Ouvrir une **session personnelle** sur un poste partagé | personnels | S-1106 |
| MOB-214 | **Changer de personne** sur le poste en un geste | personnels | — |
| MOB-215 | Fermer la session à l'inactivité, à la fin de service | SYS | — |
| MOB-216 | Voir **qui** utilisait le poste à une heure donnée | DIR, ADM | journal du poste |
| MOB-217 | Voir l'**état hors ligne** des postes (dernière synchronisation, données présentes) | ADM, REF | ACS-329 |
| MOB-218 | **Révoquer** un poste (effacement au prochain contact) | ADM, REF | — |
| MOB-219 | Ouvrir une session **élève** courte sur un iPad de classe | ELV | Q-MO11 |

### 21.9 Accessibilité, langues, réglages
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-220 | Utiliser l'appli avec **VoiceOver / TalkBack** | TOUS | RAAM |
| MOB-221 | Agrandir le texte, renforcer le contraste, réduire les animations | TOUS | — |
| MOB-222 | Choisir la **langue** de l'appli indépendamment du téléphone | TOUS | — |
| MOB-223 | Lire l'appli en **droite-gauche** (arabe) | TOUS | — |
| MOB-224 | Utiliser le **mode extérieur** (gros boutons, contraste fort) | PRF EPS, ENC | — |
| MOB-225 | Voir l'**état de conformité** d'accessibilité et signaler un défaut | TOUS | décret 2019-768 |
| MOB-226 | Dicter un texte (traitement local) | personnels | §20 |

### 21.10 Administration par l'établissement (web)
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-227 | Régler la **politique mobile** (délai de verrouillage, appareils partagés, aperçus par défaut) | ADM, DIR | S-1101, S-1103 |
| MOB-228 | **Raccourcir** un plafond hors ligne | ADM, DIR | S-1102 ; jamais allonger |
| MOB-229 | Voir les **appareils inscrits** (nombre par profil, versions, dernier contact), sans liste nominative par défaut | ADM | minimisation |
| MOB-230 | Voir les appareils **d'une personne** (support) | ADM, SEC | journalisé |
| MOB-231 | Imprimer les **QR de rattachement** (feuilles de rentrée, invitations) | SEC, ADM | socles 3, 6, 9 |
| MOB-232 | Voir les **rattachements en échec** (codes expirés, familles bloquées) et relancer | SEC | socle 8 |
| MOB-233 | Déclarer les **exceptions du règlement intérieur** à la pause des élèves (usage pédagogique) | DIR | S-1108 |
| MOB-234 | Voir les **lots hors ligne en conflit** de l'établissement | CPE, DIR | socle 12 |
| MOB-235 | Voir les **incidents d'appareil** et leur suite | DIR, DPO | §12.7 |
| MOB-236 | Voir la **répartition des versions** d'appli et qui doit mettre à jour (sans nom) | ADM | — |

### 21.11 Compte, conformité, support (la personne)
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-237 | Lire la **notice d'information** et les mentions | TOUS | — |
| MOB-238 | Exercer ses **droits** (accès, rectification) : lien vers la démarche du socle 27 | TOUS | — |
| MOB-239 | **Exporter** ses données personnelles : renvoi au web | TOUS | D-003 |
| MOB-240 | Demander la **fermeture de son compte** (règle 5.1.1(v)) | TOUS | demande à l'établissement ; explication des obligations de conservation |
| MOB-241 | **Envoyer un diagnostic** | TOUS | §19.6 |
| MOB-242 | Contacter l'**établissement** (pas l'éditeur) pour l'aide | TOUS | — |
| MOB-243 | Voir les **licences** des composants | TOUS | — |
| MOB-244 | Lire la **politique de divulgation** des vulnérabilités | TOUS | §19.5 |

### 21.12 Éditeur (plateforme)
| ID | Action | Acteur | Règles clés |
|---|---|---|---|
| MOB-245 | **Publier** une version (piste labo, pilote, production) | EDI | §19 |
| MOB-246 | Relever la **version minimale** ; bloquer une version pour raison de sécurité ★ | EDI | R-MO30 ; deux personnes |
| MOB-247 | Produire la **SBOM** et vérifier les licences de chaque version | EDI | CI |
| MOB-248 | Tenir à jour les **fiches des magasins** (confidentialité, sécurité des données) | EDI | §15.5 |
| MOB-249 | Publier les **fichiers d'association** de domaine (`assetlinks.json`, `apple-app-site-association`) | EDI | MO2 ; aucun contenu tiers sous `scolia.fr` (D-013) |
| MOB-250 | Recevoir un **signalement de vulnérabilité** et lancer la procédure 24 h / 72 h | EDI | §5.6 |
| MOB-251 | Faire **tourner** une clé (APNs, FCM, signature d'envoi) | EDI | E11 |
| MOB-252 | Voir les **plantages** remontés volontairement (diagnostics escaladés) | EDI | sans donnée personnelle |
| MOB-253 | Désactiver une **fonction** à distance (interrupteur) en cas de défaut grave | EDI | journalisé ; jamais pour contourner un droit |
| MOB-254 | Préparer les **comptes de revue** (Apple, Google) sur le labo | EDI | données fictives |
| MOB-255 | Annoncer la **fin de support** d'une version de système | EDI | R-MO31 |

---

## 22. Règles (R-MO1 et suivantes) [R]

| ID | Règle |
|---|---|
| R-MO1 | Aucune donnée n'est envoyée par l'appli avant un rattachement explicite ; aucun identifiant du constructeur ni publicitaire n'est lu. |
| R-MO2 | L'appli ne calcule aucun droit : elle affiche le **manifeste** ; tout 403 est géré comme un état normal. |
| R-MO3 | Session `mobile` (longue) **si et seulement si** assurance passkey (ou fédération + passkey) **et** clé d'appareil inscrite ; sinon durée web ; la raison est affichée. |
| R-MO4 | La clé d'appareil est non exportable, générée sur l'appareil, liée à une session ; sa clé publique est la seule chose qui en sort. |
| R-MO5 | Toute écriture, tout lot, tout renouvellement de bail est **signé** par la clé d'appareil ; un jeton sans signature ne lit que pendant 15 min après la dernière requête signée. |
| R-MO6 | Révoquer une session révoque **dans la même transaction** son jeton de notification, ses baux et crée l'ordre de purge ; la déconnexion fait de même. |
| R-MO7 | Un ordre de purge est livré au prochain contact (et tenté par notification silencieuse) ; l'appareil confirme ; une purge non confirmée reste visible de la direction. |
| R-MO8 | Tout lot de données sorti vers un appareil a un **bail** inscrit côté serveur (profil, nombre de personnes, sensibilité, fin). |
| R-MO9 | Pas de verrou d'appareil configuré → pas de session longue, pas de profil hors ligne. |
| R-MO10 | Une donnée lue hors ligne affiche **toujours** son heure de mise à jour. |
| R-MO11 | Une donnée expirée n'est **jamais** montrée ; l'expiration se vérifie par la dernière heure serveur connue et l'horloge monotone, pas par l'heure réglée du téléphone. |
| R-MO12 | Toute opération porte l'heure de l'appareil et l'heure serveur de référence ; un écart > 5 min est signalé ; l'heure serveur corrigée fait foi. |
| R-MO13 | Un profil hors ligne ne contient que les champs qu'il déclare ; le serveur retire tout le reste avant envoi (test de schéma). |
| R-MO14 | Tout changement de droits (majorité, lien, fin de fonction) s'applique au prochain contact : manifeste, purge des baux concernés, message neutre. |
| R-MO15 | Jamais de donnée de santé hors ligne hors `sortie.urgence` ; jamais de coordonnées de famille hors `accueil.secours`. |
| R-MO16 | Le jeton de notification est lié à **une** session ; il est chiffré au repos, jamais renvoyé par l'API, jamais journalisé. |
| R-MO17 | Un jeton déjà inscrit pour un autre compte n'est pas réaffecté en silence : l'ancienne inscription est révoquée et son titulaire reçoit un avis de sécurité. |
| R-MO18 | Aucune action métier n'est exécutable depuis une notification sur écran verrouillé. |
| R-MO19 | Une opération hors ligne est appliquée **au plus une fois** (idempotence par `operation_id`) ; les droits sont relus à la réception, à la date de l'objet. |
| R-MO20 | Les conflits sont jugés par le socle propriétaire ; le socle 11 ne choisit jamais un gagnant. |
| R-MO21 | Un brouillon n'est **jamais** envoyé automatiquement au retour du réseau : il est reproposé à la personne (R-PO20). |
| R-MO22 | Aucune notification non urgente à un **élève** pendant ses séances du jour ; remise à la fin des cours (ou à la récréation, S-1108). |
| R-MO23 | Aucun SDK tiers, aucune mesure d'audience, aucun rapport de plantage externe dans l'appli. |
| R-MO24 | Permissions demandées au moment du besoin ; photothèque par le sélecteur du système ; aucune géolocalisation. |
| R-MO25 | Métadonnées des photos retirées sur l'appareil avant tout envoi. |
| R-MO26 | Rien de nominatif sensible sur l'écran verrouillé, dans le sélecteur d'applis, dans un widget ou dans le presse-papiers universel. |
| R-MO27 | Une seule appli pour toutes les casquettes ; un cache **par session** ; aucun mélange d'écrans entre casquettes. |
| R-MO28 | Plusieurs établissements : aucune requête ne porte d'information d'un établissement vers un autre ; la réunion se fait sur l'appareil. |
| R-MO29 | Appareil partagé : aucun bail personnel ne survit à la session personnelle. |
| R-MO30 | Version minimale servie par le serveur ; sous le minimum : écriture refusée (`426`) ; correctif de sécurité : blocage total après 48 h d'avis, décision à deux personnes. |
| R-MO31 | Versions de système supportées revues chaque été et annoncées 6 mois avant une fin de support. |
| R-MO32 | Les DTO mobiles sont générés depuis `ent-types` ; les fixtures mobiles sont rejouées contre l'API en CI. |
| R-MO33 | Toute capacité mobile existe à l'identique sur le web (parité, mêmes tests de séparation). |
| R-MO34 | Le diagnostic ne contient aucune donnée d'élève ni jeton ; il n'est envoyé qu'à la demande de la personne. |
| R-MO35 | Les fichiers d'association de domaine ne déclarent que les applis signées de ScolIA ; aucun contenu tiers sous `scolia.fr`. |
| R-MO36 | Un lien profond n'ouvre un écran qu'après vérification du contexte (établissement, casquette, enfant) ; sinon écran neutre. |
| R-MO37 | La politique d'un établissement peut durcir (raccourcir, interdire), jamais assouplir un plafond fixé par un socle. |
| R-MO38 | Fermeture de compte demandée depuis l'appli : demande tracée à l'établissement, réponse sous un mois (RGPD art. 12) avec les durées de conservation applicables. |
| R-MO39 | Les clés d'appli (signature, APNs, FCM) sont inscrites à l'inventaire des clés avec séquestre (E11). |

---

## 23. Cas limites (CMO-01 et suivants)

| ID | Situation | Réponse |
|---|---|---|
| CMO-01 | Téléphone perdu, hors réseau, avec un appel du jour | bail expire à minuit ; révocation à distance ; l'appel non reçu reste « à faire » (CA-44 du 12) |
| CMO-02 | Téléphone volé déverrouillé | profils « déverrouillage » redemandent la présence ; session révocable ; inventaire au DPO |
| CMO-03 | Appli désinstallée puis réinstallée | jetons Keychain purgés au premier lancement (existant) ; nouvelle clé d'appareil ; ancienne inscription expirée à 30 jours sans contact |
| CMO-04 | Téléphone restauré depuis une sauvegarde iCloud / Google | aucune clé ni cache dans la sauvegarde (`ThisDeviceOnly`, `allowBackup=false`) ; nouvelle connexion |
| CMO-05 | Deux téléphones pour la même personne | deux appareils inscrits, deux sessions ; baux séparés |
| CMO-06 | Parent professeur dans le même établissement | deux casquettes, deux sessions, sélecteur (§10) |
| CMO-07 | Professeur dans deux établissements ScolIA | deux sessions ; journée réunie sur l'appareil |
| CMO-08 | Famille de deux établissements, dont un **hors ScolIA** | seul l'établissement ScolIA apparaît ; aucune mention de l'autre |
| CMO-09 | Établissement qui quitte ScolIA | purge des données de cet établissement au prochain contact ; message ; les autres établissements intacts |
| CMO-10 | Heure du téléphone décalée de 2 h | R-MO11, R-MO12 |
| CMO-11 | Changement d'heure pendant un appel hors ligne | heures murales de la séance (10) ; heure serveur de référence |
| CMO-12 | Lot hors ligne reçu deux fois | idempotence |
| CMO-13 | Lot d'un appareil révoqué | refusé ; tâche de vérification (CA-46 du 12) |
| CMO-14 | Lot signé par une clé inconnue | refusé, journal de sécurité |
| CMO-15 | Lot vieux de trois jours | refusé hors fenêtre (S-469) |
| CMO-16 | Deux co-intervenants, l'un hors ligne | règle du 12 (CA-07) |
| CMO-17 | Version d'appli sous le minimum, en sortie scolaire | lecture et opérations hors ligne déjà sous bail conservées ; envoi refusé jusqu'à la mise à jour, **sauf** pour un correctif non lié aux lots (S-1107) |
| CMO-18 | Mise à jour obligatoire un matin de rentrée | avis 48 h avant ; jamais un blocage décidé pendant les heures de cours sans urgence |
| CMO-19 | Magasin qui refuse une version la veille d'une échéance | version précédente reste servie ; serveur compatible N-2 |
| CMO-20 | Notifications bloquées dans le système | R-N57 ; plancher du socle 8 |
| CMO-21 | Jeton de notification inscrit par un autre compte sur le même téléphone | R-MO17 |
| CMO-22 | Téléphone sans services Google | passkeys selon le système [H] ; pas de FCM → pas de push (UnifiedPush plus tard) ; plancher e-mail/SMS |
| CMO-23 | Ancien Android (8) | non supporté ; web ; message clair |
| CMO-24 | Téléphone sans verrou | R-MO9 |
| CMO-25 | Téléphone rooté | §14 ; S-1105 |
| CMO-26 | Biométrie changée (nouvelle empreinte) | clé liée à la biométrie invalidée [H Android] → reconnexion par passkey, baux à reprendre |
| CMO-27 | Élève qui devient majeur en pleine nuit | R-MO14 au prochain contact |
| CMO-28 | Parent dont le lien est suspendu pendant qu'il lit l'EDT | prochaine requête : 403 neutre, purge (R-PO33) |
| CMO-29 | Parent interdit qui garde l'appli ouverte hors réseau | bail ≤ 24 h ; R-MO42 |
| CMO-30 | Élève parti en cours d'année | accès « s'arrête le … » ; purge des baux ; compte conservé 3 mois (arrêté 2006) |
| CMO-31 | Personnel en fin de contrat | casquette retirée ; purge ; e-mail personnel non utilisé |
| CMO-32 | Lien profond vers un enfant qui n'est plus le mien | R-MO36 |
| CMO-33 | Lien profond d'un autre établissement que le contexte courant | bascule proposée si la personne y est rattachée, sinon écran neutre |
| CMO-34 | QR de rattachement photographié et partagé | code à usage unique et durée courte ; rattachement exige l'identité (socle 9) |
| CMO-35 | Faux QR collé sur l'affiche | R-MO41 |
| CMO-36 | Sous-domaine trompeur (`lpnb-scolia.fr`, `lpnb.scolia.fr.exemple.com`) | refusé par la lecture stricte |
| CMO-37 | Passkey synchronisée sur l'iPad familial | la passkey est à la personne ; l'iPad familial reste un appareil distinct, avec verrou ; conseil affiché |
| CMO-38 | Tablette d'accueil oubliée ouverte | fermeture de session à l'inactivité (S-1106) |
| CMO-39 | Coupure réseau à 16 h 30 à l'accueil | fiche de secours sous bail (14) |
| CMO-40 | Alerte PPMS sans réseau | liste de comptage précalculée à la réception ; si jamais reçue : la direction compte depuis son téléphone (plan B du 8) |
| CMO-41 | Encadrant à l'étranger, données mobiles coupées | profils de sortie déjà sous bail ; envoi au wifi |
| CMO-42 | Téléphone de l'encadrant volé à l'étranger | R-MO43 |
| CMO-43 | Batterie à 3 % pendant l'appel | journal écrit à chaque geste (aucune perte) |
| CMO-44 | Appli tuée par le système pendant un envoi | lot repris au lancement ; idempotence |
| CMO-45 | Stockage du téléphone plein | profil refusé avec message ; aucune écriture partielle |
| CMO-46 | Photo de justificatif de 20 Mo en HEIC | conversion, compression, métadonnées retirées |
| CMO-47 | Capture d'écran de la liste d'appel (iOS) | avertissement ; Android : bloquée |
| CMO-48 | Mode discret dans le bus | prénoms seulement ; alertes de remise neutres |
| CMO-49 | Lecteur d'écran sur la grille d'appel | chaque élève : nom, état, actions annoncés |
| CMO-50 | Parent arabophone | appli en arabe, droite-gauche ; textes juridiques en français + mention |
| CMO-51 | Plage calme : professeur qui répond à 22 h | rappel et heure de visibilité (MOB-171) |
| CMO-52 | Élève qui reçoit un « devoir modifié » pendant les cours | retenu jusqu'à la fin des cours (R-MO22) |
| CMO-53 | Urgence (mise à l'abri) pendant les cours | les personnels sont alertés ; l'appli élève ne porte pas l'alerte (14) |
| CMO-54 | Fédération ÉduConnect indisponible | passkey directe si la personne en a une ; sinon message et web |
| CMO-55 | Récupération de passkey lancée par un tiers | alertes sur tous les appareils ; annulation en un geste |
| CMO-56 | Session web et mobile ouvertes en même temps | indépendantes ; révocation de l'une sans l'autre |
| CMO-57 | Restauration de la base à J-7 | appareils révoqués après J-7 rejoués depuis les ordres de purge et la révocation générale ; jetons de notification neutralisés (E9) puis réinscrits au prochain contact |
| CMO-58 | Un socle ajoute un champ à un profil hors ligne | refusé tant que la déclaration du profil n'est pas mise à jour et revue (R-MO13) |
| CMO-59 | Ancienne version qui ne connaît pas un écran du manifeste | écran ignoré, message « mettez à jour pour accéder à … » |
| CMO-60 | Appareil inscrit sans contact depuis 30 jours | inscription expirée, baux expirés ; session conservée jusqu'à sa propre fin mais réinscription exigée |

---

## 24. Droits (catalogue D-017) [R]

| Action | Portée | Modèles par défaut | Remarque |
|---|---|---|---|
| `mobile.appareils.lire` | établissement | Direction, Administratif | statistiques sans nom par défaut |
| `mobile.appareils.lire_personne` | établissement | Direction, Secrétariat (support) | journalisé |
| `mobile.purge.ordonner` | établissement | Direction | ★ |
| `mobile.politique.parametrer` | établissement | Direction, administrateur | — |
| `mobile.postes.gerer` | établissement | administrateur, référent numérique | inscription, révocation des appareils partagés |
| `mobile.incidents.lire` | établissement | Direction ; DPO (rôle) | — |
| `mobile.conflits.lire` | établissement | CPE, Direction | lecture ; trancher = action du socle propriétaire |
| `mobile.rattachements.suivre` | établissement | Secrétariat | échecs, relances |
| (plateforme) `plateforme.mobile.versions` | plateforme | éditeur | hors établissements ; deux personnes pour un blocage |

Chaque personne gère **ses** appareils sans droit particulier (portée « moi-même »). Aucun droit mobile n'ouvre la lecture d'une donnée métier : l'appareil n'est qu'un canal.

---

## 25. Événements

### 25.1 Émis (outbox, dans la transaction du fait)
`mobile.appareil_inscrit`, `mobile.appareil_revoque` (motif typé), `mobile.appareil_declare_perdu`, `mobile.purge_ordonnee`, `mobile.purge_confirmee`, `mobile.bail_emis` (journal technique, non diffusé), `mobile.lot_recu` (comptes, sans contenu), `mobile.poste_session_ouverte` / `_fermee`, `mobile.version_minimale_relevee`, `mobile.version_bloquee`, `mobile.incident_ouvert`. **Aucun** texte libre, aucun nom d'élève dans les événements.

### 25.2 Consommés
| Événement | Source | Effet |
|---|---|---|
| `session.revoquee`, `compte.desactive` | 1 | R-MO6 |
| `revocation.generale` | 9 (R-X25) | tous les appareils de la personne dans l'établissement |
| `lien.suspendu`, `lien.interdiction_posee`, `lien.droits_modifies` | 4 | purge des baux famille concernés, manifeste |
| `eleve.majeur` | 2, 4 | R-MO14 |
| `personne.departie`, `scolarite.terminee` | 2, 3 | fin d'accès annoncée |
| `casquette.retiree`, `fonction.terminee` | 6 | purge des baux de la casquette |
| `droits.modifies` (rôle, D-017) | 1 | manifeste au prochain contact |
| `edt.version_publiee` | 10 | renouvellement de `edt.jour` |
| `sortie.cloturee`, `alerte.levee`, `internat.appel_valide` | 42, 14, 41 | expiration anticipée des baux |
| `etablissement.retire` | plateforme | MOB-209 |

---

## 26. Modèle de données et migrations

Conventions E1–E14 : `etablissement_id UUID NOT NULL` partout (sauf `ent_mob_version`, table **de plateforme** justifiée au registre E12) ; FK composites `(etablissement_id, x_id)` ; UUID applicatifs ; `cree_le` / `modifie_le` ; aucune suppression physique d'un objet qui a servi (états datés) ; RLS dès la création ; **aucune colonne JSON** ni tableau ; aucun `BYTEA` (les diagnostics sont des colonnes typées, pas des fichiers).

### 26.1 Tables [R]
```
-- 2221 (MO4) : appareils
ent_mob_appareil
  id UUID PK, etablissement_id UUID NOT NULL, compte_id UUID NOT NULL (ent_user), session_id UUID NULL (session courante),
  plateforme TEXT CHECK (plateforme IN ('ios','android')),
  libelle TEXT NOT NULL CHECK (length(libelle) <= 60),          -- « iPhone de Claire », saisi ou générique, jamais l'identifiant constructeur
  modele_generique TEXT NULL CHECK (length(modele_generique) <= 40),
  version_systeme TEXT NOT NULL, version_appli TEXT NOT NULL,
  cle_publique TEXT NOT NULL, cle_algo TEXT CHECK (cle_algo IN ('p256')), cle_empreinte TEXT NOT NULL,
  cle_materielle BOOLEAN NOT NULL,                              -- Secure Enclave / StrongBox / TEE déclarés
  attestation TEXT NOT NULL DEFAULT 'aucune' CHECK (attestation IN ('aucune','app_attest','play_integrity','echec')),
  attestation_le TIMESTAMPTZ NULL,
  verrou_configure BOOLEAN NOT NULL, signal_modifie BOOLEAN NOT NULL DEFAULT false,
  partage BOOLEAN NOT NULL DEFAULT false, poste_id UUID NULL (FK composite ent_mob_poste),
  inscrit_le TIMESTAMPTZ NOT NULL, dernier_contact_le TIMESTAMPTZ NOT NULL,
  etat TEXT CHECK (etat IN ('actif','expire','revoque','perdu')), etat_le TIMESTAMPTZ NOT NULL,
  revoque_motif TEXT NULL CHECK (revoque_motif IN ('deconnexion','revocation_personne','revocation_direction','perte','purge_lien','fin_fonction','sans_contact','securite','etablissement_retire')),
  cree_le, modifie_le
  UNIQUE (etablissement_id, cle_empreinte) ; UNIQUE (etablissement_id, id)
ALTER ent_session ADD appareil_id UUID NULL (FK composite) ; CHECK (client <> 'mobile' OR appareil_id IS NOT NULL) après reprise

-- 2222 (MO4) : politique de l'établissement
ent_mob_politique
  etablissement_id PK, verrouillage_min SMALLINT CHECK (1..15) DEFAULT 5,
  apercus_defaut TEXT CHECK (IN ('aucun','deverrouille')) DEFAULT 'deverrouille',
  postes_partages BOOLEAN DEFAULT true, refuser_appareil_modifie_c2 BOOLEAN DEFAULT true,
  pause_eleves TEXT CHECK (IN ('fin_des_cours','recreation')) DEFAULT 'fin_des_cours',
  modifie_par, modifie_le

-- 2223 (MO12) : versions (PLATEFORME, sans etablissement_id, classée « plateforme » au registre)
ent_mob_version
  id UUID PK, plateforme, version TEXT, publiee_le, minimale_depuis TIMESTAMPTZ NULL,
  bloquee_depuis TIMESTAMPTZ NULL, motif_code TEXT NULL CHECK (IN ('securite','compatibilite')),
  decide_par UUID, second_valideur UUID NULL CHECK (second_valideur <> decide_par), cree_le
  UNIQUE (plateforme, version)

-- 2224 (MO7) : baux
ent_mob_bail
  id UUID PK, etablissement_id, appareil_id (FK composite), session_id,
  profil_code TEXT NOT NULL,                                     -- déclaré en code par un socle
  objet_type TEXT NULL, objet_id UUID NULL,                      -- séance, sortie, poste… (référence typée)
  sensibilite TEXT CHECK (IN ('C0','C1','C2','C3')), nb_personnes INTEGER NOT NULL CHECK (nb_personnes >= 0),
  empreinte_contenu TEXT NOT NULL, emis_le, expire_le CHECK (expire_le > emis_le),
  etat TEXT CHECK (IN ('actif','rendu','expire','purge_demandee','purge_confirmee')), etat_le
  INDEX (etablissement_id, appareil_id) WHERE etat = 'actif'

-- 2225 (MO7) : ordres de purge
ent_mob_ordre_purge
  id UUID PK, etablissement_id, portee TEXT CHECK (IN ('appareil','session','compte_etablissement','profil')),
  appareil_id NULL, session_id NULL, compte_id NULL, profil_code NULL,
  motif TEXT CHECK (IN ('revocation','deconnexion','perte','lien_suspendu','interdiction','fin_fonction','majorite','etablissement_retire','securite','demande_personne')),
  ordonne_par UUID NULL (NULL = système), ordonne_le, livre_le NULL, confirme_le NULL, cree_le
  -- rejouées après une restauration (CMO-57) : la table est conservée, pas neutralisée

-- 2226 (MO7) : plafonds raccourcis par l'établissement
ent_mob_profil_reglage
  etablissement_id, profil_code, duree_max_min INTEGER CHECK (duree_max_min > 0), desactive BOOLEAN,
  modifie_par, modifie_le ; PK (etablissement_id, profil_code)
  -- l'application vérifie duree_max_min <= plafond déclaré par le socle (R-MO37)

-- 2227 (MO8) : transport des opérations
ent_mob_lot_recu
  id UUID PK (tiré par l'appareil), etablissement_id, appareil_id, session_id, recu_le, nb_operations INTEGER,
  signature_valide BOOLEAN, resultat TEXT CHECK (IN ('traite','refuse_appareil','refuse_signature','refuse_version')), cree_le
ent_mob_operation_recue
  operation_id UUID PK, etablissement_id, lot_id (FK composite), domaine TEXT NOT NULL, type_code TEXT NOT NULL,
  objet_type TEXT, objet_id UUID, saisie_appareil_le TIMESTAMPTZ, heure_reference_le TIMESTAMPTZ, ecart_horloge_s INTEGER,
  recue_le, resultat TEXT CHECK (IN ('appliquee','compatible','conflit','refusee')), raison_code TEXT NULL, cree_le
  -- registre d'idempotence ; le sens (état, minutes…) reste dans la table du socle (ex. ent_appel_operation du 12, FK vers operation_id)
  -- conservé 3 mois (aligné sur le 12), à neutraliser à la restauration (E9) : une opération non rejouée est réémise par l'appareil

-- 2228 (MO9) : postes partagés
ent_mob_poste
  id UUID PK, etablissement_id, nom TEXT CHECK (length <= 60), lieu_id UUID NULL (socle 5), usage TEXT CHECK (IN ('accueil','portail','internat','classe','service_aed','autre')),
  inscrit_par, inscrit_le, etat, etat_le, cree_le, modifie_le
ent_mob_poste_session
  id, etablissement_id, poste_id, compte_id, ouverte_le, fermee_le NULL, fermeture TEXT CHECK (IN ('personne','inactivite','fin_service','revocation'))

-- 2229 (MO14) : incidents d'appareil
ent_mob_incident
  id, etablissement_id, appareil_id, declare_par, declare_le, nature TEXT CHECK (IN ('perte','vol','soupcon')),
  nb_baux_actifs INTEGER, nb_personnes_max INTEGER, sensibilite_max TEXT, verrou_configure BOOLEAN,
  purge_confirmee_le NULL, transmis_dpo_le NULL, decision_dpo TEXT NULL CHECK (IN ('pas_de_notification','notification_cnil','information_personnes')), decision_le NULL
  -- les baux concernés sont retrouvés par appareil_id ; aucune liste de noms copiée

-- 2230 (MO13) : diagnostics reçus (colonnes typées, aucune donnée d'élève)
ent_mob_diagnostic
  id, etablissement_id, appareil_id NULL, recu_le, version_appli, version_systeme, plateforme,
  dernier_code_erreur TEXT CHECK (length <= 40), nb_baux_actifs SMALLINT, nb_operations_en_attente SMALLINT,
  commentaire TEXT CHECK (length <= 500), escalade_editeur_le NULL
  -- conservé 3 mois
```

### 26.2 Reprise et coordination
- **`ent_push_device` (058)** : en MO0 (2220) colonne `session_id` (FK composite vers `ent_session`, nullable pour la reprise ; inscriptions sans session = celles d'avant MO0, **révoquées** à 30 jours sans réinscription), `token_empreinte`, `token_chiffre` (format `enc:v2:<id_cle>:`, E11) avec copie puis effacement de `token` en clair ; révocation en cascade. Le socle 8 (N7, 3803/3812) remplace la table par `ent_ntf_appareil`, qui reçoit `appareil_id` (FK vers `ent_mob_appareil`) ; retrait de 058 en **2239** après la bascule.
- **`ent_appel_operation` (12, 2253)** : sa clé `operation_id` devient FK vers `ent_mob_operation_recue.operation_id` (à accorder avec le socle 12 ; C-3).
- **`ent_session`** : `appareil_id` en 2221 ; reprise : sessions `mobile` existantes sans appareil → conservées jusqu'à leur fin, **pas prolongées** (R-MO3).

### 26.3 Registre (E12) et restauration (E9)
| Table | Portée | Export | Restauration |
|---|---|---|---|
| `ent_mob_appareil` | établissement | non (technique) | conservée ; appareils révoqués après la date restaurée rejoués depuis `ent_mob_ordre_purge` et la révocation générale |
| `ent_mob_politique`, `ent_mob_profil_reglage`, `ent_mob_poste` | établissement | simple | conservée |
| `ent_mob_version` | **plateforme** | non | conservée (globale) |
| `ent_mob_bail` | établissement | non | conservée |
| `ent_mob_ordre_purge` | établissement | non | conservée **et rejouée** |
| `ent_mob_lot_recu`, `ent_mob_operation_recue` | établissement | non | **neutralisée** (E9) |
| `ent_mob_poste_session`, `ent_mob_incident` | établissement | double (journal) | conservée |
| `ent_mob_diagnostic` | établissement | non | purge |
| `ent_push_device` (058, jusqu'au retrait) | établissement | **non** (corrige DS5) | **neutralisée** (purge) |

### 26.4 Plan des migrations (plage imposée 2220–2239)
| N° | Lot | Contenu |
|---|---|---|
| **2220** | MO0 | `ent_push_device` : `session_id`, `token_empreinte`, `token_chiffre`, index ; fonction de révocation en cascade ; (le classement du registre est dans le code) |
| **2221** | MO4 | `ent_mob_appareil` ; `ent_session.appareil_id` |
| **2222** | MO4 | `ent_mob_politique` |
| **2223** | MO12 | `ent_mob_version` (plateforme) |
| **2224** | MO7 | `ent_mob_bail` |
| **2225** | MO7 | `ent_mob_ordre_purge` |
| **2226** | MO7 | `ent_mob_profil_reglage` |
| **2227** | MO8 | `ent_mob_lot_recu`, `ent_mob_operation_recue` |
| **2228** | MO9 | `ent_mob_poste`, `ent_mob_poste_session` ; `ent_mob_appareil.poste_id` (FK) |
| **2229** | MO14 | `ent_mob_incident` |
| **2230** | MO13 | `ent_mob_diagnostic` |
| **2231** | personas | `ent_mob_signalement_lien` (§32.4) |
| **2232** | personas | `ent_mob_appareil.profil_local` (téléphone familial, §32.4) |
| 2233–2237 | — | réserve |
| **2238** | — | RLS stricte (rôle non propriétaire, test strict) |
| **2239** | — | retrait de `ent_push_device` (058) et de `token` en clair, **après** la bascule du socle 8 (N11) et décision |

---

## 27. Lots de code (équipe Sonnet)

Chaque lot livre : migrations dans la plage, service en transaction unique, routes, types `ent-types`, tests Postgres (isolation entre établissements, matrice de droits), tests des cœurs Kotlin et Swift, écrans, documentation (ce document, `MOBILE.md`, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; **revue sécurité** (`security-review`, grille MASVS) sur MO0, MO2, MO4, MO7, MO8, MO9. Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

### 27.1 Tableau
| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **MO0 Correctifs urgents** | serveur : DS1 (révocation → jeton de notification, même transaction), DS2 (durée mobile seulement après passkey ; en attendant la clé d'appareil de MO4 : **passkey** exigée), DS3 (réponse sans jeton, jeton chiffré + empreinte, réaffectation = révocation + avis), DS4 (`DELETE /devices/push/{id}`, appel à la déconnexion), DS5 (registre : non exporté, neutralisé) ; appli : DA1 (expiration provisoire : minuit pour appel et EDT, 24 h pour le reste, purge au démarrage et à l'ouverture), DA2 (`targetSdk 36`, essais bord à bord et retour prédictif) ; documentation : DX1, DX2 transmis à Florian | — | tests rouges avant, verts après : session révoquée → aucune remise push ; `client:"mobile"` + mot de passe → durée web ; `POST /devices/push` ne renvoie pas le jeton ; jeton inscrit par B pour le téléphone de A → A révoqué et averti ; export de portabilité sans jeton ; cache de 25 h → non servi ; build `release` API 36 | M |
| **MO1 Intégration et CI** | fusion de `ent-mobile-app` dans la chaîne d'intégration (hors `ENT` tant que Florian ne l'autorise pas) ; retrait des 4 prototypes ; CI : `:core:test`, `:app:testDebugUnitTest`, `assembleRelease`, `lintDebug`, `swift test` (Linux) ; DTO générés (§8.3) pour les nouvelles routes ; **tests de contrat** (M11) ; iOS compilé sur Mac (Q-MO3) | MO0 | CI verte des deux cœurs ; une modification de DTO qui casse l'appli échoue ; aucun fichier des prototypes | M |
| **MO2 Domaines et passkeys natives** | `/.well-known/assetlinks.json` (release + debug) et `apple-app-site-association` (`webcredentials`, `applinks` `/rejoindre`) servis par `scolia.fr` et les sous-domaines ; origines `android:apk-key-hash` ; vérification des jokers ; R-MO35 | MO1 | passkey créée et utilisée sur Android et iOS réels contre le labo ; lien `/rejoindre` ouvre l'appli ; fichier refusé s'il contient une autre appli | S |
| **MO3 Rattachement** | annuaire public (M3) et confirmation par le guichet central (R-MO41) ; codes de rattachement (socle 9) et invitations d'agents (357) consommés (M2) ; MOB-100 à 114 | MO2 ; socle 9 (X8 ou repli local) | rattachement famille, élève, agent ; QR piégé refusé ; aucun indice d'existence de compte | M |
| **MO4 Appareil et session liée** | `ent_mob_appareil`, `ent_mob_politique` (2221–2222) ; clé d'appareil et signatures (R-MO4, R-MO5) ; R-MO3, R-MO9 ; « Mes appareils » ; attestation en signal (Q-MO4) ; clé du cache liée à la présence (DA4) | MO2 | jeton copié sans clé → écriture refusée ; appareil sans verrou → durée web ; révocation → purge ; 401 propre | L |
| **MO5 Manifeste et contextes** | `GET /mobile/contexte` (M8) ; écrans déclarés par les socles (`mobile::declarer_ecran`) ; casquettes (DA6) ; enfants, établissements, mode discret ; liens profonds (R-MO36) ; onglets servis | MO4 ; socles 1 (registre), 6 (P2), 29 (P8) | rôle libre D-017 → bons écrans sans code mobile ; parent-professeur → deux contextes, zéro mélange ; lien profond hors contexte → neutre | M |
| **MO6 Notifications reçues** | avec le socle 8 N7 : inscription liée à l'appareil, autorisation rapportée, extension d'aperçu, canaux et catégories, silencieuses, pastille, R-MO18, R-MO22 (avec 8 et 10) | MO4 ; socle 8 N7 | bacs à sable APNs / FCM ; aucune donnée C2 dans une charge ; élève en cours → retenue ; révocation → plus rien | M |
| **MO7 Moteur hors ligne (baux)** | 2224–2226 ; déclaration des profils ; émission, renouvellement (ETag), expiration, purge, inventaire ; démarrage à froid (DA5) ; écran « Hors ligne et envois » ; R-MO8 à R-MO15 ; **condition d'entrée** des lots AS10 (12), P8 (29), 41, 42 et 14 AS5 | MO4, MO5 | profil `appel.jour` reçu, lu hors ligne, effacé à minuit (horloge truquée comprise) ; lien suspendu → purge < 1 min si joignable ; inventaire exact d'un appareil déclaré perdu | L |
| **MO8 Opérations différées** | 2227 ; file locale, lots signés, idempotence, fenêtre, résultat ligne par ligne, horloge (R-MO12), validateurs des socles ; premier client : appel (12 AS10) | MO7 ; socle 12 AS2 | coupure simulée pendant l'appel → aucune perte ; lot rejoué → aucun doublon ; lot d'appareil révoqué → refus ; signature fausse → refus et journal | L |
| **MO9 Appareils partagés** | 2228 ; inscription par QR et par configuration gérée ; session personnelle par-dessus ; fermeture ; journal du poste ; état des postes (ACS-329) | MO4, MO7 ; socle 14 AS5 | tablette d'accueil : deux agents successifs, aucune donnée du premier visible du second ; poste révoqué → effacé | M |
| **MO10 Accessibilité et langues** | audit RAAM des écrans de la coquille ; corrections ; état de conformité ; langues et droite-gauche ; mode extérieur | MO5 | audit écrit ; TalkBack et VoiceOver joués sur le parcours d'appel et de justification | M |
| **MO11 Publication** | comptes d'organisation (Florian) ; fiches, étiquettes de confidentialité, sécurité des données ; fermeture de compte (MOB-240) ; comptes de revue ; pistes pilote | MO2 à MO6 | appli acceptée sur les deux magasins en piste pilote | M |
| **MO12 Versions et cyber-résilience** | 2223 ; version minimale, blocage à deux personnes, `426` ; SBOM et licences en CI ; politique de divulgation, procédure 24 h / 72 h ; mise à jour des dépendances (DA10) | MO1 | version bloquée → écran de mise à jour ; SBOM jointe à chaque version | S |
| **MO13 Support et diagnostic** | 2230 ; diagnostic à la demande, sans donnée personnelle ; vues de l'établissement (MOB-229 à 236) | MO7 | diagnostic relu : aucun nom, aucun jeton (test) | S |
| **MO14 Perte et incident** | 2229 ; déclaration, inventaire, compte rendu au DPO, suivi | MO7 | téléphone de labo déclaré perdu → inventaire juste, purge confirmée au retour du réseau | S |

**Ordre recommandé** : **MO0** tout de suite → MO1 → MO2 → (MO3, MO4) → MO5 → MO7 → MO8 (avec 12 AS10, janvier) → MO6 (avec 8 N7) → MO9 → MO14 → MO10 → MO11 → MO12, MO13 au fil de l'eau. **Démo de décembre** : MO0 + MO1 (Android, appel en ligne) ; **janvier** : MO7, MO8 et l'appel hors ligne.

### 27.2 Ce que chaque socle ajoute quand il livre son lot mobile
Déclaration de ses écrans, profils hors ligne et opérations (§6.2), tests de parité de droits (R-MO33), DTO dans `ent-types`, aucune route réservée au mobile sauf justification écrite.

### 27.3 Lots mobiles déjà prévus par les autres socles
3 I15 · 4 A13 · 5 (lecture) · 6 P12 · 7 D13 · 8 N7 · 9 §22 · 10 T12 (mobile au 11) · 12 AS10, AS14 (mobile) · 13 D11 · 14 AS5, AS10 · 15 A12 · 16 (MSG-151, 152) · 17 AN10 · 18 FO10 · 19 AG8 · 20 C11 · 21 V9 · 22 C10 · 23 B12 · 29 P8 · 34 (fiche d'urgence) · 40, 41, 42 (mobile de l'encadrant, de l'internat, de la restauration).

---

## 28. Tests et recette

### 28.1 Automatiques
1. **Serveur** (Postgres) : R-MO3, R-MO5, R-MO6, R-MO7, R-MO13, R-MO16, R-MO17, R-MO19, R-MO30 ; isolation entre établissements pour chaque table et chaque route ; restauration simulée (CMO-57) ; `grep -i "json\|bytea"` sur 2220–2239.
2. **Cœurs** (JVM, Linux) : lecture des liens et QR (dont pièges CMO-35, CMO-36), baux (expiration avec horloge truquée, monotone), purge, file d'opérations, signature, idempotence locale, manifeste inconnu (CMO-59), contexte.
3. **Contrat** : fixtures rejouées contre `ent-api` (R-MO32) ; parité des droits web / mobile (R-MO33).
4. **Interface** : Compose (Robolectric) et XCUITest sur les parcours de la coquille.
5. **Propriétés** : « aucune donnée expirée affichée » ; « une opération ⇒ au plus une application » ; « aucun champ non déclaré dans un profil » ; « aucune donnée C2+ dans une charge de notification ».

### 28.2 Sur appareils réels (consignés)
Passkeys (création, connexion, passkey synchronisée, clé de sécurité), biométrie, QR, liens universels, notifications (bacs à sable), hors ligne réel (mode avion pendant l'appel), TalkBack, VoiceOver, grandes tailles de texte, Android 9 et dernière version, iPhone SE et grand écran, tablette.

### 28.3 Recette par personas (D-021 étape 4), dont attaques
Élève qui prend le téléphone du professeur posé sur le bureau ; parent interdit qui garde l'appli hors réseau ; ex-conjoint qui connaît le code du téléphone familial ; script qui se déclare « mobile » ; lot rejoué, falsifié, d'un appareil révoqué, signé par une autre clé ; faux QR sur l'affiche ; sous-domaine trompeur ; lien profond vers l'enfant d'une autre famille ; capture d'écran d'une liste d'appel ; restauration de la base suivie d'un appareil « ressuscité » ; version bloquée ; appareil rooté ; diagnostic qui contiendrait un nom.

---
## 29. Décisions appliquées, réglages, questions

### 29.1 Recommandations appliquées par défaut (D-031)
| # | Recommandation appliquée | Où |
|---|---|---|
| A-1 | Le socle 11 ne porte **aucun écran métier** ; chaque socle livre ses écrans mobiles par le kit | §6, §27.3 |
| A-2 | **Manifeste servi par le serveur** au lieu d'écrans calculés dans l'appli | §11.2 |
| A-3 | **Session longue seulement avec passkey et clé d'appareil** ; sinon durée web (corrige DS2) | R-MO3 |
| A-4 | **Clé d'appareil** non exportable et signature des écritures (preuve de possession) | §9.4 |
| A-5 | **Baux** hors ligne inscrits côté serveur, purge par destruction de clé, inventaire en cas de perte | §12 |
| A-6 | Profils hors ligne **déclarés par les socles**, plafonds que l'établissement peut seulement raccourcir | §12.2, R-MO37 |
| A-7 | Opérations différées : transport et idempotence au 11, sens et conflits au socle propriétaire | §12.4 |
| A-8 | **Aucun SDK tiers**, aucune mesure d'audience, aucun rapport de plantage externe | R-MO23 |
| A-9 | **Pas d'épinglage de certificat** au départ | §14 |
| A-10 | Attestation d'appli en **signal**, jamais seule condition | §9.4 |
| A-11 | Appareil **sans verrou** : ni session longue ni hors ligne (renverse l'existant) | R-MO9 |
| A-12 | Élèves : aucune notification non urgente pendant leurs séances ; carte imprimée d'abord | R-MO22, §16 |
| A-13 | Casquettes = deux sessions, deux caches, sélecteur | §10 |
| A-14 | Demande de fermeture de compte dans l'appli, traitée par l'établissement | MOB-240 |
| A-15 | Traiter les applis **comme couvertes** par le règlement sur la cyber-résilience (SBOM, divulgation, 24 h / 72 h) | §5.6 |
| A-16 | Versions : compatibilité N-2, version minimale servie, blocage de sécurité à deux personnes après 48 h d'avis | R-MO30 |
| A-17 | Dictée **locale** seulement, désactivée sinon | §20 |
| A-18 | Cœurs natifs conservés ; DTO générés depuis `ent-types` ; tests de contrat | §8.3 |
| A-19 | Bloquants de la passe personas A à D | §32 |

### 29.2 Réglages signalés (D-023)
| ID | Réglage | Nature | Recommandation |
|---|---|---|---|
| S-1101 | Délai de verrouillage (1–15 min, défaut 5) | établissement | garder |
| S-1102 | Raccourcir un plafond hors ligne | établissement | garder, rarement utile |
| S-1103 | Aperçus de notifications par défaut | établissement | garder (défaut : déverrouillé seulement) |
| S-1104 | UnifiedPush pour Android sans Google | plateforme | plus tard, à la demande |
| S-1105 | Refuser les profils C2+ sur appareil modifié | établissement | garder, défaut oui |
| S-1106 | Durée de session sur poste partagé | établissement | garder (15 min accueil, fin de service internat) |
| S-1107 | Laisser passer les lots hors ligne d'une version sous le minimum (hors correctif de sécurité) | plateforme | fixe : oui |
| S-1108 | Remise des notifications des élèves : fin des cours ou récréation ; exceptions pédagogiques | établissement | garder ; défaut fin des cours |

### 29.3 Ce qui engage Florian (non tranché par D-031)
| # | Sujet | Pourquoi Florian | Recommandation |
|---|---|---|---|
| F-1 | **Comptes développeur** Apple (programme d'organisation, D-U-N-S) et Google Play (organisation, vérification d'identité) | coût annuel, contrat, identité de l'organisme éditeur | ouvrir les deux comptes avant janvier (revue Apple : plusieurs jours) |
| F-2 | **Mac** (ou CI macOS louée) pour construire et signer l'appli iOS (Q-MO3) | achat ou abonnement | un Mac mini dédié au build + appareils de test (un iPhone, un Android ancien et un récent) |
| F-3 | **Transport des notifications** par Apple (APNs) et Google (FCM) : enveloppes hors UE, contrats | DPO, contrat | accepter APNs + FCM avec enveloppes neutres (seul moyen sur iOS), inscrits au registre ; UnifiedPush plus tard |
| F-4 | **AIPD mobile** (appareils personnels, mineurs, hors ligne) avant toute donnée réelle | DPO | à mener au printemps 2027 avec celle de l'assiduité (P-9 du 12) |
| F-5 | **Failles probables Next.js** DX1, DX2 (lecture seule, non exécuté) | Next.js intouchable pour nous | vérifier la matrice de modules en production ; corriger côté Next.js si confirmé (décision et action de Florian) |
| F-6 | **Cœur Rust partagé** entre les deux applis (Q-MO1) | change la mise en œuvre de D-003 | étude bornée d'une semaine avant MO7, décision ensuite |
| F-7 | **Avis juridique** cyber-résilience et période de support (Q-MO10) | engagement contractuel | demander l'avis ; en attendant, A-15 |
| F-8 | Publication des applis **sous la marque** et l'organisme éditeur | identité commerciale | nom « ScolIA », éditeur = l'organisme de Florian |

### 29.4 Questions (avec recommandation)
| ID | Question | Recommandation |
|---|---|---|
| Q-MO1 | Cœur Rust partagé ? | F-6 |
| Q-MO2 | Repli pour les personnes sans appareil compatible passkey (D-009) ? | passkey sur un autre appareil ou clé de sécurité ; sinon web + code à usage unique, durée web ; à trancher au socle 1 |
| Q-MO3 | Construction iOS : Mac dédié ou CI macOS ? | F-2 |
| Q-MO4 | Attestation (App Attest, Play Integrity) : la demander ? | oui, en signal, après avis du DPO sur le transit chez Apple et Google |
| Q-MO5 | Épinglage de certificat ? | non au départ (A-9) ; à réexaminer après un an d'exploitation |
| Q-MO6 | Catégorie et public déclarés aux magasins (enfants dans l'audience) ? | « éducation », audience générale, pas la catégorie enfants ; relire les règles « Familles » de Google avant MO11 |
| Q-MO7 | Qualification d'Apple et Google (sous-traitants, destinataires) ? | DPO (F-3) |
| Q-MO8 | DNMA sur le mobile (public) ? | côté serveur seulement si le contrat l'exige ; jamais de SDK dans l'appli |
| Q-MO9 | Base légale pour les élèves de moins de 15 ans : mission d'intérêt public, pas de consentement ? | oui, à confirmer par le DPO |
| Q-MO10 | Période de support et cyber-résilience ? | F-7 |
| Q-MO11 | iPad de classe : dans l'appli (mode partagé élève) ou hors périmètre ? | dans l'appli, session courte sans cache ; hors V1 si aucun établissement pilote ne l'utilise |

---

## 30. Faits, hypothèses, contradictions

**Faits** : §2 (chemins cités). **Hypothèses** principales : vérification des jokers `*.scolia.fr` par iOS et Android (DA9) ; possibilité de récupérer l'aperçu depuis l'extension de notification (§13) ; comportement des notifications silencieuses ; invalidation des clés Android à un changement de biométrie (CMO-26) ; détail des configurations gérées (§17) ; cadence mensuelle des versions.

**Contradictions relevées**
- **C-1** `14_ACCUEIL_SURETE.md` (§11, ACS-167) prévoit un **QR dynamique dans l'appli de l'élève** pour le contrôle de sortie ; la loi du 24/08/2026 étend l'interdiction du téléphone au **lycée** pendant le temps scolaire (L511-5 déjà au collège). **Proposition** : la carte imprimée reste le moyen principal ; le QR de l'appli n'est proposé que si le règlement intérieur le prévoit (exception déclarée, S-1108), au portail de sortie. À porter à la relecture du 14.
- **C-2** `MOBILE.md` (§4 point 5) déverrouille automatiquement l'appli quand l'appareil n'a aucun verrou ; ce document l'interdit (R-MO9). Ce document prévaut après relecture.
- **C-3** Le socle 12 prévoit `ent_appel_operation.operation_id` comme clé primaire propre ; ce document met le registre d'idempotence au 11 (`ent_mob_operation_recue`) et la clé du 12 en référence. À accorder au lot AS10 (pas de double registre).
- **C-4** `08_NOTIFICATIONS.md` (§26) prévoit d'associer la session au jeton « à la prochaine connexion, sinon expiré à 30 jours » ; MO0 associe la session **dès l'inscription** et révoque en cascade (DS1) : compatible, plus strict.
- **C-5** `SOCLES.md` §11 décrit le hors ligne comme « lecture, puis saisie de l'appel » ; ce document le généralise (profils de plusieurs socles) sans changer l'ordre (appel d'abord).
- **C-6** `29_PORTAILS.md` R-PO33 fixe 24 h sans contact serveur pour le cache d'un responsable ; le profil `edt.jour` (lendemain inclus) est **réservé aux élèves et personnels** ; pour un responsable, il est coupé à la journée en cours.

---

## 31. Sources (consultées le 2026-11-11)
- Téléphone : https://www.education.gouv.fr/interdiction-du-telephone-portable-dans-les-ecoles-et-les-colleges-et-pause-numerique-455181 · https://www.education.gouv.fr/bo/2025/Hebdo28/MENE2519904C · https://www.vie-publique.fr/loi/301799-proteger-les-mineurs-risques-des-reseaux-sociaux-loi-du-24-aout-2026 · https://www.info.gouv.fr/actualite/interdiction-des-telephones-portables-au-lycee-ce-que-dit-la-loi
- CNIL : https://www.cnil.fr/fr/recommandations-applications-mobiles · https://www.cnil.fr/sites/cnil/files/2024-09/recommandation-applications-mobiles.pdf · https://cnil.fr/fr/permissions-applications-mobiles-recommandations-de-la-cnil-pour-respecter-la-vie-privee
- Magasins : https://developer.apple.com/support/offering-account-deletion-in-your-app · https://developer.apple.com/help/app-review/guideline-reference/5-1-1-account-deletion · https://developer.android.com/google/play/requirements/target-sdk
- Cyber-résilience : https://www.crowell.com/en/insights/client-alerts/eu-cyber-resilience-act-countdown-11-september-2026-incidentvulnerability-reporting-deadline-is-less-than-100-days-away · https://www.kirkland.com/publications/kirkland-alert/2026/09/the-eu-cyber-resilience-act (sources secondaires ; règlement (UE) 2024/2847 à relire sur EUR-Lex)
- Repris des conceptions du dépôt : arrêté du 7/09/2026, loi 2005-102 art. 47, décret 2019-768, RAAM, SDET, arrêté du 30/11/2006 (`29_PORTAILS.md` §5) ; ÉduConnect, Guichet Agents (`09_INTEROP.md`, `VERIF_09_SOURCES.md`).

---
## 32. Compléments de la passe personas (ajout personas, 2026-11-11)

Source : `docs/socles/PERSONAS_11.md` (17 personas, 64 manques M-1100 à M-1163 : 4 bloquants, 38 importants, 20 confort, 2 différenciants). Bloquants et importants **appliqués par défaut** (D-031). Rien n'est retiré de ce qui précède.

### 32.1 Règles ajoutées (R-MO40 à R-MO46)
| ID | Règle | Manques |
|---|---|---|
| **R-MO40** | **Téléphone partagé** (bloquant A) : une installation peut porter plusieurs **profils locaux** (un par compte), chacun avec son propre déverrouillage (passkey du compte ou code propre au profil, en plus du verrou de l'appareil), ses sessions, ses caches et ses baux ; passer d'un profil à l'autre exige le déverrouillage du profil cible ; un profil élève n'est jamais ouvert par le déverrouillage d'un profil parent. Le serveur connaît seulement l'identifiant aléatoire du profil local (`profil_local`, 2232), jamais les autres comptes de l'appareil. | M-1129, M-1134, M-1135 |
| **R-MO41** | **Rattachement sûr** (bloquant B) : nom officiel, commune et logo obtenus du **guichet central** et affichés avant toute saisie ; l'appli n'ouvre **aucune page web** pour saisir un mot de passe ou créer une passkey à partir d'un QR ; QR imprimés avec un **code court de vérification** que l'appli affiche aussi ; tout QR ou lien refusé peut être **signalé** (2231) ; un établissement reçoit l'alerte quand plusieurs signalements visent son nom. | M-1104, M-1163 |
| **R-MO42** | **Coupure d'un responsable** (bloquant C) : à la pose d'une interdiction ou à la suspension d'un lien, ordre de purge **dans la transaction** de la décision du socle 4, notification silencieuse, coupure au prochain contact (objectif < 1 min si l'appareil est joignable) ; les profils famille ne portent jamais plus que **la journée en cours** ; message neutre ; aucun nouveau code de rattachement n'est délivré pour ce lien. | M-1126 à M-1128 |
| **R-MO43** | **Appareil d'encadrant perdu en sortie** (bloquant D) : la direction peut déclarer la perte **pour** l'encadrant, ordonner la purge des baux de sortie et émettre le profil de sortie sur l'appareil d'un autre encadrant de la même sortie ; le compte rendu d'incident est prérempli avec l'inventaire des baux (§12.7). | M-1141, M-1142 |
| **R-MO44** | **Téléphone personnel jamais obligatoire** pour un personnel : toute fonction mobile d'un personnel a un équivalent sur un poste de l'établissement (tablette de salle, poste partagé) ou sur le web ; la charte le dit. | M-1103, M-1116, M-1121 |
| **R-MO45** | **Contexte toujours affiché** : chaque écran, chaque notification (enveloppe comprise) indique l'établissement, la casquette et, sur un appareil partagé, le profil local, sans contenu sensible. | M-1108, M-1135 |
| **R-MO46** | **Urgences jamais bloquées** : alerte, comptage, fiche de secours et fiches d'urgence restent utilisables sous une version minimale ou une purge de confort ; seule une révocation de sécurité les coupe. | M-1120 |

### 32.2 Actions ajoutées (MOB-256 à MOB-285)
| ID | Action | Acteur | Règles |
|---|---|---|---|
| MOB-256 | Voir sur Aujourd'hui l'**état des envois** en attente (et l'entendre au lecteur d'écran) | personnels | M-1100, M-1146 |
| MOB-257 | Faire un appel **par exceptions** en mode extérieur (composant du 12) | PRF, ENC | M-1101 |
| MOB-258 | Faire l'appel ou le comptage **sans téléphone personnel** (tablette de salle, poste partagé) | personnels | R-MO44 |
| MOB-259 | Imprimer des **QR de rattachement vérifiables** (code court affiché par l'appli) | SEC | R-MO41 |
| MOB-260 | Voir la **fiche d'état de connexion** d'une personne (rattachement, passkey, appareils, notifications, version), sans ses données | SEC, ADM | journalisé |
| MOB-261 | Produire les **lettres de rattachement** par famille, dans sa langue | SEC | socle 7 (trames) |
| MOB-262 | Déclarer un appareil perdu **dans tous ses établissements** en un geste (guichet central, sans croisement) | TOUS | M-1111 |
| MOB-263 | Se connecter à un poste par **clé de sécurité / badge NFC** | personnels | socle 1 |
| MOB-264 | **Rideau** : masquer l'écran d'un geste | ACC | — |
| MOB-265 | Être orienté vers le **web** quand l'appareil n'est pas supporté | TOUS | M-1122 |
| MOB-266 | Préparer une **justification** hors ligne, envoyée après relecture | FAM, MAJ | R-MO21 |
| MOB-267 | **Verrou propre** au profil élève sur un téléphone partagé | ELV | R-MO40 |
| MOB-268 | Être **guidé** pour activer le verrou du téléphone | TOUS | R-MO9 |
| MOB-269 | **Accompagnement** à l'accueil : l'agent guide, la personne fait les gestes, l'écran de l'agent ne montre rien de ses données | ACC, FAM | M-1133 |
| MOB-270 | **Parcours guidé** le jour de la majorité | MAJ | socle 4 |
| MOB-271 | Ordonner pour un encadrant la **purge des baux de sortie** | DIR | R-MO43 |
| MOB-272 | **Émettre le profil de sortie** sur l'appareil d'un autre encadrant | DIR, ENC | R-MO43 |
| MOB-273 | **N'envoyer qu'en wifi** (itinérance) | TOUS | M-1143 |
| MOB-274 | Grille d'appel au lecteur d'écran avec **actions personnalisées** | PRF | M-1145 |
| MOB-275 | Ouvrir une session élève sur un iPad de classe par **QR personnel imprimé** | ELV | M-1148 |
| MOB-276 | Régler une **fenêtre de mise à jour** de la flotte | REF | M-1149 |
| MOB-277 | Recevoir le profil **`internat.urgence`** (fiches d'urgence des internes la nuit, mêmes garde-fous que la sortie) | INT | à déclarer par 34 et 41 |
| MOB-278 | Voir les **statistiques des baux** (sensibilité, durées réelles, purges non confirmées) pour l'AIPD | DPO | M-1154 |
| MOB-279 | Publier la **SBOM** et prouver l'absence de SDK tiers (test CI) | EDI, DPO | M-1156 |
| MOB-280 | Lire la notice « **ce qui peut être sur votre téléphone** », par profil | TOUS | M-1157 |
| MOB-281 | Voir l'**échéance des 72 h** d'un incident | DPO | M-1158 |
| MOB-282 | **Verrou court** des écrans de saisie pendant une séance | SYS | S-1109 |
| MOB-283 | **Signaler** une appli contrefaite ; lien vers la fiche officielle unique | TOUS, EDI | M-1163 |
| MOB-284 | Voir les appareils **non attestés** de l'établissement (nombre) | ADM | Q-MO4 |
| MOB-285 | **Verrouillage d'urgence** de toutes les sessions mobiles d'un établissement (compromission) ★ | DIR + EDI | deux personnes |

### 32.3 Cas limites ajoutés (CMO-61 à CMO-74)
| ID | Situation | Réponse |
|---|---|---|
| CMO-61 | Élève en retard pendant un appel hors ligne | opération « retard » horodatée (12) |
| CMO-62 | Responsable sans e-mail | activation par code de rattachement, passkey ; récupération par l'établissement seulement (socle 1) |
| CMO-63 | Professeure qui enseigne à son enfant | casquettes séparées ; conflit d'intérêts du socle 4 appliqué ; aucun raccourci entre casquettes |
| CMO-64 | Deux établissements, plages calmes différentes | chacune servie par son établissement ; l'appli n'en déduit rien |
| CMO-65 | Localiser un élève hors ligne | indisponible (lecture unitaire journalisée), message clair |
| CMO-66 | Alerte depuis une zone blanche | wifi, autre personne habilitée, procédure papier du PPMS ; l'appli le dit et propose d'appeler |
| CMO-67 | L'autre parent cherche à savoir si elle a l'appli | aucune information entre foyers (socle 4) |
| CMO-68 | Parent interdit qui redemande un code | refusé par le socle 4/9 ; l'agent voit « code non disponible pour ce lien » |
| CMO-69 | Android sans compte Google : passkey impossible [H] | repli du socle 1 (Q-MO2) ; durée web |
| CMO-70 | Élève sans téléphone | web, carte imprimée ; aucune fonction exigée sur mobile |
| CMO-71 | Fin de scolarité | accès 3 mois ; export depuis le web (MOB-239) |
| CMO-72 | Voyage dans un autre fuseau | heures de l'établissement, heure locale en regard |
| CMO-73 | Taille de texte maximale | grille d'appel en liste, test d'interface |
| CMO-74 | Photo de l'écran d'appel par un autre téléphone | non empêchable ; écran minimal (pas de motif, pas de santé) |

### 32.4 Modèle de données ajouté
```
-- 2231 : signalements de liens et QR
ent_mob_signalement_lien
  id UUID PK, etablissement_id UUID NULL   -- NULL si l'hôte ne correspond à aucun établissement (portée « plateforme », justifiée au registre)
  hote_signale TEXT CHECK (length <= 253), motif TEXT CHECK (IN ('hote_inconnu','nom_trompeur','code_invalide','autre')),
  signale_le TIMESTAMPTZ, traite_le NULL, traite_par NULL, issue TEXT NULL CHECK (IN ('faux_positif','hameconnage','affiche_remplacee'))
  -- aucune donnée de la personne qui signale ; conservé 12 mois

-- 2232 : profil local (téléphone partagé)
ALTER ent_mob_appareil ADD profil_local UUID NULL   -- identifiant aléatoire du profil local, jamais relié aux autres comptes de l'appareil
```

### 32.5 Durées de conservation proposées (à valider par le DPO)
| Table | Durée | Après |
|---|---|---|
| `ent_mob_appareil` | 30 jours après révocation ou expiration | suppression |
| `ent_mob_bail` | 3 mois après la fin | suppression (agrégats gardés pour MOB-278) |
| `ent_mob_ordre_purge` | 12 mois | suppression |
| `ent_mob_lot_recu`, `ent_mob_operation_recue` | 3 mois (comme le 12) | suppression |
| `ent_mob_poste_session` | 12 mois (journal) | suppression |
| `ent_mob_incident` | 5 ans [NV, durée de preuve d'une violation à fixer par le DPO] | archive |
| `ent_mob_diagnostic` | 3 mois | suppression |
| `ent_mob_signalement_lien` | 12 mois | suppression |

### 32.6 Place dans les lots
MO3 : R-MO41, MOB-259 à 261 ; MO4 : R-MO40, R-MO44 (avec MO9), MOB-263, 267, 268 ; MO5 : R-MO45, MOB-265, 270 ; MO7 : R-MO42 (condition d'entrée), R-MO46, MOB-256, 266, 273, 277, 278, 280 ; MO8 : MOB-257, 258 ; MO9 : MOB-264, 269, 275, 276 ; MO10 : MOB-274 ; MO12 : MOB-279, 283 ; MO14 : R-MO43, MOB-262, 271, 272, 281, 284, 285 ; S-1109 (verrou court des écrans de saisie, défaut 2 min) : MO4.
