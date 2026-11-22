# Socle 11 — Applications mobiles : périmètre quotidien par rôle, contrat d'API, menaces et plan de lots

**Date :** 2026-11-22 — **Statut :** conception (session Opus), **proposée**, non codée. Règles incertaines tranchées en paramètres d'établissement avec valeur prudente (D-069). Aucun code écrit ; aucun accès à une base, à un secret ou à un service de production ; Next.js non lu.
**Branche :** `ent-socle-11-conception` (depuis `origin/ent-mc0hoe`, `48715844`).
**Document compagnon (fait foi pour la coquille) :** `docs/socles/11_MOBILE.md` (2026-11-11) et sa passe `PERSONAS_11.md`. Ils décrivent en détail le rattachement, la session liée à l'appareil, le manifeste, les baux hors ligne, les opérations différées, les notifications reçues, les appareils partagés, la publication. **Ils restent la référence** : actions MOB-100 à MOB-285, règles R-MO1 à R-MO46, cas limites CMO-01 à CMO-74, tables et migrations 2220–2239. Je ne les recopie pas.
**Ce que ce document ajoute :** (1) l'état réel au 2026-11-22 et ce que D-062, D-068 et D-069 changent ; (2) la coquille résumée en décisions, avec les trous comblés ; (3) le **périmètre quotidien par rôle**, déduit des conceptions des socles 2 à 47, et ce qui **reste sur le web** ; (4) le **contrat d'API mobile** (erreurs codées, pagination, idempotence, versions), la table des **routes historiques à abandonner** et ce qui **manque côté serveur, socle par socle** ; (5) la **parité des droits** et ses tests ; (6) le **modèle de menace** consolidé ; (7) le **plan de lots MB0 à MB16**, qui remplace l'ordonnancement du §27 de `11_MOBILE.md` (conçu pour la démo de décembre, rendue caduque par D-068), avec la répartition Linux / Mac ; (8) les paramètres d'établissement et une seule vraie question.
**Conventions :** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[V~]** règle connue par une conception déjà vérifiée du dépôt ; **[H]** hypothèse ; **[R]** recommandation appliquée par défaut. Identifiants nouveaux : actions quotidiennes **MQ-100** et suivantes, règles **R-MO47** et suivantes (suite de la série), menaces **MEN-01**, paramètres **S-1109** et suivants, lots **MB0** à **MB16**.

Sommaire : §0 En bref · §1 État au 2026-11-22 · §2 Coquille commune : décisions · §3 Périmètre quotidien par rôle · §4 Ce qui reste sur le web · §5 Contrat d'API mobile · §6 Routes historiques et manques serveur par socle · §7 Parité des droits · §8 Modèle de menace, minimisation, journal, révocation · §9 Plan de lots MB · §10 Linux, Mac, appareils : comment avancer sans bloquer · §11 Paramètres d'établissement · §12 Règles ajoutées · §13 Question ouverte · §14 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **Une appli par plateforme, pour tous les rôles, qui affiche ce que le serveur autorise et oublie ce qu'elle n'a plus le droit de garder** (D-003, thèse de `11_MOBILE.md` §1). Le socle 11 livre la coquille et le kit ; chaque socle branche ses écrans du quotidien ; le reste demeure sur le web.
2. **Rien n'a bougé dans le code depuis le 11/11** [F] : le lot de correctifs MO0 n'est pas fait (le client se déclare toujours « mobile » pour obtenir 90 jours, `ent-db/src/auth_repo.rs:58-63`) ; la branche `ent-mobile-app` (appli unique Android et iOS, cœurs testés) n'est toujours fusionnée nulle part ; les 4 prototypes sont toujours dans `ent-rust/apps/` ; l'appli appelle des **routes historiques** que les conceptions 16, 20 et 21 ont reconnues fautives (bulletin lisible sans droit, cahier régénéré, fils lus par déduction) ; l'API ne renvoie que des erreurs `(StatusCode, String)` sans code stable [F, `ent-api/src/portail.rs`, `routes.rs:1176`].
3. **Trois décisions postérieures changent la conception** : **D-068** (bascule à la rentrée 2027, démo de haut niveau au printemps, plus de démo mobile à marche forcée en décembre) ; **D-062** (dépendances minimales : les DTO ne sont pas générés par un outil tiers, ils sont **vérifiés par des fixtures dorées** produites par le serveur ; OkHttp est retiré) ; **D-069** (toute règle incertaine devient un paramètre d'établissement ; les questions Q-MO1 à Q-MO11 sont tranchées ici, §2.3).
4. **Périmètre quotidien** (§3) : **151 actions** MQ-100 à MQ-250 réparties sur 17 rôles (professeur, professeur principal, remplaçant, CPE, AED, surveillant d'internat, accueil, direction, secrétariat, infirmière, AESH, encadrant de sortie, agent, documentaliste, famille, élève mineur, élève majeur). Toutes renvoient à une action déjà conçue dans un socle (pas de droit inventé). **Reste sur le web** (§4) : paramétrage, imports, constructions (EDT, services), conseils de classe, procédures disciplinaires, séjours, finances, comptabilité, paie, dossiers de santé, RGPD, exports.
5. **Contrat d'API** (§5) : mêmes routes, mêmes droits, mêmes DTO que le web ; enveloppe d'erreur **codée** et stable ; 404 neutre pour tout objet hors droits ; pagination par curseur ; `Idempotency-Key` obligatoire sur toute écriture mobile ; `ETag` / `If-Match` ; dates ISO 8601 au fuseau de l'établissement (D-067) ; version d'appli en en-tête, compatibilité N-2, `426` sous le minimum. Seules les routes `/mobile/*` de la coquille sont propres au mobile (contexte, appareils, baux, lots, diagnostic).
6. **Parité des droits** (§7) : un banc de parité rejoue, pour chaque écran et chaque opération déclarés au manifeste, la matrice personas × action × cible en **cookie web** et en **Bearer mobile** ; toute différence fait échouer la CI. Il étend `web/crates/ent-web-server/tests/droits_parite.rs` [F].
7. **Menaces** (§8) : 22 menaces MEN-01 à MEN-22 avec mesure, test et risque résiduel ; les plus dures : l'**enfant sur le téléphone du parent** (il justifie sa propre absence) → **présence du titulaire exigée** pour les gestes ★ (justifier, autoriser, signer, payer), par la clé d'appareil liée à la biométrie ; le **parent séparé** → tout est calculé par foyer et par lien côté serveur, rien n'est réuni sur l'appareil entre foyers ; le **téléphone perdu** → révocation en une transaction, inventaire des baux, purge par destruction de clé.
8. **Plan de lots** (§9) : **MB0** correctifs (tout de suite) → **MB1** intégration et CI → **MB2** contrat et parité → **MB3** domaines, passkeys, rattachement → **MB4** appareil et session liée → **MB5** contexte et manifeste → **MB6** professeur en ligne → **MB7** hors ligne (baux et opérations : appel au gymnase, appel de nuit à l'internat) → **MB8** famille → **MB9** notifications réelles → **MB10** élève → **MB11** vie scolaire et direction → **MB12** appareils partagés et accueil → **MB13** accessibilité et langues → **MB14** perte, versions, diagnostic → **MB15** publication → **MB16** verticaux secondaires. **Démo du printemps 2027** = MB0 à MB11 sur Android et iOS installés hors magasin, contre le labo.
9. **Linux / Mac** (§10) : tout le serveur, tout Android (build, tests JVM et Robolectric, APK) et le cœur Swift (`swift test`) se font dans un conteneur Linux ; seuls l'interface iOS, la signature et les essais sur iPhone exigent un Mac. Méthode : cœurs miroirs testés sous Linux, écrans iOS écrits en miroir des écrans Compose, **une passe Mac par lot** sur le Mac de Florian (D-055), jamais bloquante pour Android.
10. **Une seule question** (§13) : D-055 repousse les comptes développeur Apple et Google « plus tard » ; or les **notifications iOS (APNs) et TestFlight exigent le compte Apple payant**. Recommandation : ouvrir le compte Apple d'organisation avant mars 2027 ; d'ici là, démo iOS sans notifications réelles.

---

## 1. État au 2026-11-22

### 1.1 Faits vérifiés aujourd'hui [F]
| Sujet | Constat | Preuve |
|---|---|---|
| Correctifs MO0 | **non faits** : `ClientSession::parse_client` accepte toujours `"mobile"` déclaré par le corps de la requête ; `/devices/push` inchangé | `ent-rust/crates/ent-db/src/auth_repo.rs:58-63` ; `ent-api/src/routes.rs:195` |
| Prototypes | toujours présents : `apps/{android,ios}-{eleve,famille}` + `README.md` (jeton en mémoire, `http://127.0.0.1:3001` par défaut) | `ent-rust/apps/README.md` |
| Appli unique | branche `origin/ent-mobile-app` (`5da17f42`), non fusionnée ; dernier état : écrans du personnel selon les droits, tests Compose sans appareil (Robolectric), iOS jamais compilé | `git log origin/ent-mobile-app` |
| Routes appelées par l'appli | `/auth/*`, `/roles/mes-droits`, `/edt/mes-creneaux`, `/annonces`, `/appels*`, `/vs/absences`, `/famille/enfants*`, `/absences/{id}/justifier`, `/eleves/{id}/notes` (lecture **et écriture**), `/note-periodes`, `/devoirs`, `/classes/{c}/effectif`, `/cahier-texte`, `/eleves/{id}/edt`, `/messages/fils*`, `/auth/sessions` | `origin/ent-mobile-app:docs/MOBILE.md` §2 |
| Erreurs de l'API | `Result<_, (StatusCode, String)>` : message en clair, **aucun code stable** que l'appli puisse interpréter ou traduire | `ent-api/src/portail.rs` (fonction `aujourdhui`), `routes.rs:1176`, `:1418` |
| Accueil | `GET /portail/aujourdhui` existe (socle 29, lot P1), composé côté serveur par les services propriétaires | `ent-api/src/portail.rs` |
| Parité web | un test de parité des droits existe côté web | `ent-rust/web/crates/ent-web-server/tests/droits_parite.rs` |
| Migrations | 240 fichiers jusqu'à `667_*` ; plage 2220–2239 réservée au socle 11 et libre | `ent-rust/migrations/`, `docs/socles/MIGRATIONS.md` (mise à jour du 2026-11-11) |
| Outils du conteneur | `java` et `gradle` présents ; pas de `swift` ni de SDK Android installés à ce jour | `which` |

### 1.2 Ce que les décisions postérieures au 11/11 changent
| Décision | Effet sur le socle 11 |
|---|---|
| **D-068** (bascule à la rentrée 2027 ; démo « masterclass » au printemps, juin acceptable) | l'ordre « MO0 + MO1 pour l'appel en ligne en décembre » de `11_MOBILE.md` §27 est remplacé par le plan MB (§9), qui vise une démo **complète** par rôle et non un écran |
| **D-062** (dépendances seulement si on ne peut pas coder à la main ; cryptographie jamais à la main) | (a) pas de générateur de DTO tiers : DTO Kotlin et Swift écrits à la main, **vérifiés** par des fixtures JSON dorées émises par les tests du serveur (remplace A-18 de `11_MOBILE.md`) ; (b) OkHttp retiré au profit de `HttpsURLConnection` ; (c) zxing gardé (décodage QR : pas raisonnable à la main) ; (d) cryptographie : plateformes seulement (Keystore, CryptoKit, Secure Enclave), serveur avec `ring` déjà présent pour vérifier les signatures d'appareil |
| **D-069** (règle incertaine = paramètre, défaut prudent) | Q-MO1 à Q-MO11 tranchées au §2.3 ; nouveaux paramètres S-1109 à S-1120 (§11) |
| **D-055** (Florian dispose de Macs ; comptes Apple et Google plus tard) | Q-MO3 tranchée : builds iOS sur le Mac de Florian ; publication et notifications iOS attendent le compte Apple (§13) |
| **D-067** (jiff, format JSON des dates inchangé) | le contrat de dates (§5.6) est figé par le test de non-régression des sérialisations ; l'appli ne dépend que du format ISO 8601 |
| **D-059** (tout-en-un, aucun service externe requis) | APNs et FCM sont des **transports** imposés par les systèmes, pas des services métier ; l'appli doit rester pleinement utile sans notifications (tout est dans « Aujourd'hui » et le centre d'avis) |

---

## 2. Coquille commune : décisions

### 2.1 Tableau de décisions (détail dans `11_MOBILE.md`)
| Sujet | Décision [R] | Détail |
|---|---|---|
| **Authentification** | passkey native (rp `scolia.fr`, D-013) ; mot de passe seulement en secours suivi du second facteur ; fédération (ÉduConnect, Guichet Agents, OIDC) par session d'authentification du système, jamais de vue web ; **jeton Bearer opaque** + **clé d'appareil** non exportable qui signe écritures, lots et renouvellements ; session longue (90 j glissants, plafond 365 j) **seulement** si passkey et clé d'appareil, sinon durée web | `11_MOBILE.md` §9, R-MO3 à R-MO5 |
| **Passkey obligatoire à terme** (D-009) | enrôlement bloquant (portée `enrolement`) dès la première connexion mobile ; sans appareil compatible : passkey sur un autre appareil ou clé de sécurité, sinon web + code à usage unique et durée web | §9.1 point 4 ; S-1110 |
| **Multi-établissements** | une session par établissement, chiffrée séparément ; même passkey ; vue réunie **sur l'appareil** seulement (aucune jointure serveur entre établissements) | §9.2, §10 point 3 |
| **Multi-enfants** | sélecteur d'enfant ; droits **par lien** (socle 4) ; « Aujourd'hui » de la fratrie réuni par le serveur dans l'établissement, par l'appareil entre établissements | §10 point 2 |
| **Changement de rôle (casquette)** | une session = une casquette (socle 6, P2) ; deux casquettes = deux sessions, deux caches, sélecteur « Professeure · Mère de Léo » ; aucun mélange d'écran ou de cache | §10 point 1 |
| **Navigation selon les droits** | **manifeste servi par le serveur** (`GET /mobile/contexte`) : écrans, profils hors ligne, opérations, onglets (4 + « Plus »), politique, version minimale ; calculé par le même moteur que le web (D-017) ; l'appli ne calcule aucun droit | §11.2 ; R-MO47 |
| **Notifications** | APNs (iOS), FCM HTTP v1 (Android) ; **charge opaque** : identifiant d'avis, catégorie, nom d'établissement ou « ScolIA » ; aperçu C1 récupéré après coup par l'extension avec la session si la personne l'a permis et que l'appareil est déverrouillé ; aucune action métier depuis l'écran verrouillé ; plage calme et retenue pendant les cours des élèves (socle 8) | §13 ; socle 8 N7 |
| **Hors ligne** | **baux** déclarés par les socles (profil, contenu minimal, plafond, purge), inventoriés côté serveur ; chiffrement AES-256-GCM par bail, clé enveloppée par la clé d'appareil ; expiration = clé indéchiffrable ; opérations différées signées, idempotentes, relues contre les droits à la réception, conflits tranchés par le socle propriétaire | §12 ; §2.2 ci-dessous |
| **Effacement à distance** | révocation (personne, direction, révocation générale) = sessions + jeton de notification + baux + ordre de purge **dans une transaction** ; purge par destruction de clé au prochain contact ; à défaut, expiration du bail | R-MO6, §12.7 |
| **Pièces et photos** | sélecteur du système (jamais la photothèque entière) ; EXIF retirés sur l'appareil ; HEIC → JPEG ; taille bornée ; aperçu avant envoi ; rien gardé dans la galerie ; envoi en URL pré-signée courte émise après contrôle des droits (socle 7) | §15.2 ; R-MO49 |
| **Accessibilité** | RGAA / RAAM, tailles du système, contrastes AA, aucun sens par la couleur seule, cibles ≥ 48 dp / 44 pt, TalkBack et VoiceOver joués sur chaque parcours de la démo | §18 |
| **Langues** | français par défaut ; langues des familles par établissement ; droite-gauche ; décisions et textes juridiques en français avec traduction marquée ; messages d'erreur traduits **à partir du code** (§5.3), jamais à partir du texte du serveur | §18 |
| **Appareils partagés** | poste inscrit par l'établissement (QR à usage unique ou gestion de flotte) ; session personnelle courte par-dessus ; seuls les profils **du poste** y vivent ; tablette de classe : session élève sans cache, fermée en fin de séance | §17 ; S-1111 |
| **Téléphone familial partagé** | profils d'appareil séparés, verrou par compte (R-MO40) ; un parent ne se connecte jamais « comme » son enfant | §16 point 3 |
| **Version minimale, mises à jour forcées** | `X-ScolIA-Appli: <plateforme>/<version>` ; compatibilité N-2 mineures ; `426` + écran de mise à jour sous le minimum ; blocage de sécurité décidé à deux personnes, effectif 48 h après l'avis ; les lots hors ligne d'une version sous le minimum restent acceptés sauf faille de sécurité | R-MO30, S-1107 |

### 2.2 Ce qui manquait à la coquille
1. **Présence du titulaire pour les gestes ★** (MEN-03) : justifier une absence, donner une autorisation, signer, payer, envoyer un message à un personnel, clore un appel, poser une mesure : l'appli exige une **authentification de l'utilisateur de l'appareil** (biométrie ou code) **au moment du geste**, par une signature de la clé d'appareil créée avec `userPresence` (iOS) / `setUserAuthenticationParameters(0, …)` (Android). Le serveur **refuse** un geste ★ dont la signature n'atteste pas la présence (`code: presence.requise`). Paramètre S-1109 : délai de grâce après un déverrouillage (défaut **0 s** pour les familles, 60 s pour les personnels pendant un appel). R-MO48.
2. **Stockage local hors bail interdit** : aucune donnée métier n'est écrite sur l'appareil en dehors d'un bail (pas de « cache de confort » d'images, de PDF, de brouillons non déclarés). Les PDF ouverts (bulletin, attestation) sont affichés en mémoire et supprimés à la fermeture ; « Enregistrer dans Fichiers » est une **exportation explicite** (le document sort de l'ENT par la volonté de la personne, journalisée côté serveur comme un téléchargement). R-MO49.
3. **Conflits de synchronisation, présentés à la personne** : trois issues seulement, toujours affichées ligne par ligne : **appliqué**, **appliqué avec mention** (ex. : « l'infirmerie avait déjà noté Léa présente à 9 h 05 : votre absence n'a pas été retenue »), **refusé** avec la raison codée (droit retiré, fenêtre dépassée, objet arrêté). Jamais de fusion silencieuse ; jamais de « dernier qui écrit gagne » hors des règles déclarées par le socle. R-MO50.
4. **Durées hors ligne par défaut** (plafonds de `11_MOBILE.md` §12.2 confirmés, et compléments) : `appel.jour` jusqu'à minuit ; `internat.nuit` (41) jusqu'à la fin du service de nuit + 2 h ; `sortie.urgence` jusqu'au retour + 24 h ; `accueil.secours` minuit ; `restauration.self` (40) fin du service ; `cdi.comptoir` (45) fin de journée ; lecture famille et élève (`aujourdhui.lecture`) 24 h sans contact. L'établissement peut **raccourcir**, jamais allonger (S-1102).
5. **Démarrage sans réseau** : le profil `contexte` permet d'ouvrir l'appli au gymnase ; sans profil valide, l'appli dit « Pas de réseau et rien d'enregistré sur ce téléphone pour aujourd'hui » et propose la **feuille papier de secours** (socle 12) au lieu d'un écran vide.
6. **Batterie et données** : aucune synchronisation périodique en arrière-plan hors renouvellement de baux planifié par le système (WorkManager, `BGAppRefreshTask`) ; lectures en `If-None-Match` ; photos compressées avant envoi ; « économie de données » respectée.

### 2.3 Questions de `11_MOBILE.md` tranchées (D-069)
| Question | Décision | Pourquoi |
|---|---|---|
| Q-MO1 cœur Rust partagé | **non** : cœurs natifs Kotlin et Swift, en miroir, mêmes noms, mêmes tests ; la dérive est mesurée par les fixtures dorées et le banc de parité | D-003 fixe les technologies natives ; une couche de liaison générée ajouterait une dépendance structurante (D-062) et une chaîne de build iOS/Android plus lourde ; réexamen possible si la dérive mesurée dépasse 2 défauts par trimestre |
| Q-MO2 repli sans passkey | S-1110 : passkey sur un autre appareil ou clé de sécurité ; sinon web + code à usage unique, **durée web**, aucun hors ligne | protège sans exclure |
| Q-MO3 build iOS | Mac de Florian (D-055), une passe par lot ; runner macOS loué seulement si la passe bloque deux lots de suite | coût nul, aucun service externe imposé |
| Q-MO4 attestation | **signal** seulement, jamais bloquant ; activable par l'établissement (S-1112, défaut **désactivé** tant que le DPO n'a pas qualifié le transit chez Apple et Google) | minimisation ; un appareil sans services Google reste utilisable |
| Q-MO5 épinglage de certificat | non | risque de couper toutes les applis au renouvellement |
| Q-MO6 magasins | catégorie « Éducation », audience générale, pas la catégorie « enfants » | l'appli sert d'abord adultes et personnels |
| Q-MO7 qualification d'Apple et Google | inscrits au registre comme destinataires techniques (enveloppes neutres) ; qualification par le DPO, sans bloquer le code (D-064) | |
| Q-MO8 mesure d'audience nationale | côté serveur seulement si un contrat public l'exige ; jamais de SDK dans l'appli | R-MO23 |
| Q-MO9 base légale moins de 15 ans | mission d'intérêt public / exécution du contrat de scolarisation, pas de consentement | aucune fonction fondée sur le consentement d'un mineur |
| Q-MO10 période de support | 3 ans de correctifs de sécurité par version majeure, version minimale relevée au moins une fois par an | traite l'appli comme couverte par le règlement sur la cyber-résilience (A-15) |
| Q-MO11 iPad de classe | dans l'appli, mode « poste de classe » (S-1111, défaut **désactivé**) | aucun établissement pilote ne l'a demandé ; prêt si besoin |

---

## 3. Périmètre quotidien par rôle

Méthode : pour chaque acteur, ce qu'il fait **debout, entre deux salles, dans le car ou le soir sur le canapé**, ce qui lui fait perdre du temps, ce qu'il redoute ; chaque action renvoie à l'action déjà conçue par le socle propriétaire (aucun droit nouveau) et au lot mobile de ce socle. Colonnes : **HL** = possible hors ligne (L = lecture sous bail, É = écriture différée), **★** = confirmation humaine et présence du titulaire (R-MO48). Les droits sont ceux du web, calculés par le serveur : une action n'apparaît que si le manifeste l'ouvre.

### 3.1 Professeur (dont professeur partagé entre deux établissements)
Ce qu'il redoute : l'appel perdu au gymnase, dix applis, être joint à 22 h, garder des listes d'élèves sur son téléphone personnel. Ce qui le ravirait : l'appel en dix secondes sans réseau, sa journée des deux établissements au même endroit, rien sur l'écran verrouillé.
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-100 | Voir ma journée (séances, salles, changements du jour, remplacements) des deux établissements | 10 T12, 29 P8 | L | |
| MQ-101 | Ouvrir l'appel de la séance en cours depuis la notification ou « Aujourd'hui » | 12 AS14 | L | |
| MQ-102 | Faire l'appel : tous présents puis exceptions, absent, retard (heure), parti en cours | 12 AS2 | É | |
| MQ-103 | Clore l'appel (récapitulatif, nombre d'absents) | 12 AS2 | É | ★ |
| MQ-104 | Corriger un appel clos dans la fenêtre permise (motif) | 12 | É | ★ |
| MQ-105 | Voir les situations connues avant l'appel (infirmerie, dispense, sortie, exclusion, stage) | 12, 34, 13 | L | |
| MQ-106 | Faire l'appel d'une activité (étude, sortie courte, UNSS) | 12 | É | ★ |
| MQ-107 | Signaler un élève arrivé en retard après l'appel | 12 | É | |
| MQ-108 | Exclure de cours depuis l'appel (vers la vie scolaire, avec accompagnant) | 13 D11 | É | ★ |
| MQ-109 | Noter une observation ou un incident rapide (fait, sans qualification) | 13 D11 | É | ★ |
| MQ-110 | Proposer une punition (retenue, devoir supplémentaire) ; la qualification reste au web | 13 D11 | | ★ |
| MQ-111 | Remplir la fin de séance du cahier : fait en classe, travail donné (pour quand), pièces | 20 C11 | É (brouillon) | ★ à la publication |
| MQ-112 | Photographier le tableau et l'attacher à la séance (EXIF retirés) | 20 C11, 7 | | |
| MQ-113 | Voir la charge de travail de la classe avant de donner un devoir (par élève) | 20 C11 | L | |
| MQ-114 | Voir les dépôts reçus d'un devoir, qui n'a pas rendu | 20 C11 | | |
| MQ-115 | Saisir des notes rapides d'une évaluation déjà créée (grille élève par élève) | 21 V9 | É (`notes.saisie`) | |
| MQ-116 | Marquer absent, dispensé, non rendu (jamais un zéro d'absence, mémento eduscol) | 21 V9 | É | |
| MQ-117 | Publier les notes d'une évaluation | 21 V9 | | ★ |
| MQ-118 | Positionner rapidement un élève sur une compétence / observation | 22 C10 | É | |
| MQ-119 | Lire et répondre à un message (fil où je suis participant) | 16 M12 | É (brouillon) | ★ à l'envoi |
| MQ-120 | Écrire à la famille d'un élève de mes classes (destinataires calculés par la matrice « qui peut écrire à qui ») | 16 M12 | | ★ |
| MQ-121 | Lire une annonce, en prendre connaissance, voir une note de service | 17 AN10 | L | |
| MQ-122 | Accepter ou refuser un remplacement proposé, voir ma séance de remplacement | 10, 15 A12 | | ★ |
| MQ-123 | Déclarer mon absence imprévue (maladie le matin) et voir ce qui en découle | 15 A12 | | ★ |
| MQ-124 | Fiche allégée d'un élève de mes classes (photo si permise, PAP/PAI : seulement les aménagements utiles en classe) | 2 E10, 35 | L (séance) | |
| MQ-125 | « Où est-il maintenant ? » pour un élève de mes classes (séance en cours, sans historique) | 6 P12, 10 | | |
| MQ-126 | Mes rendez-vous parents, émargement d'une réunion, salle libre | 19 AG8 | | |
| MQ-127 | Rejoindre une visio (cours, réunion) | 33 VI9 | | |
| MQ-128 | Recevoir une alerte de mise à l'abri et faire le comptage de ma classe | 14 AS10 | É (`surete.comptage`) | |
| MQ-129 | Signaler une salle ou un matériel en panne (photo, lieu) | 44 | | |
| MQ-130 | Basculer vers ma casquette « parent » si je suis aussi parent d'élève | 6 P12, 11 | | |

### 3.2 Professeur principal
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-131 | Voir les alertes de ma classe (absences répétées, retards, mesures, dépôts manquants) | 12, 13, 29 | L | |
| MQ-132 | Voir l'état de préparation du conseil (appréciations manquantes) ; la saisie des appréciations reste au web | 23 B12 | | |
| MQ-133 | Lire la fiche d'orientation d'un élève et l'avis du conseil | 23 B12, 37 | | |

### 3.3 Remplaçant, contractuel
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-134 | Voir mes séances de remplacement et leurs classes **pendant** la mission seulement | 10, 15, 6 | L | |
| MQ-135 | Appel et fin de séance comme le titulaire, dans la portée de la mission | 12, 20 | É | ★ |
| MQ-136 | Perdre l'accès aux classes à la fin de la mission, sans geste (manifeste et purge) | 6, 11 | | |

### 3.4 CPE
Ce qu'elle redoute : courir entre les salles, l'appel non fait de 8 h, l'élève parti sans qu'on sache où.
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-137 | File du matin : appels non faits, absents non prévenus, retards, élèves à rappeler | 12 AS14b | L | |
| MQ-138 | Relancer un professeur dont l'appel manque | 12 | | |
| MQ-139 | Appeler une famille depuis la fiche (numéro affiché, appel tracé comme contact) | 12 AS5, 4 A13 | | |
| MQ-140 | Consigner un contact famille (appel, SMS reçu, venue) | 12 AS5 | É | |
| MQ-141 | Qualifier rapidement une absence (motif reçu, justifiée) ; les régularisations en masse restent au web | 12 AS14d | | ★ |
| MQ-142 | Recevoir un élève exclu de cours, le prendre en charge, le placer en étude | 13 D11 | É | |
| MQ-143 | Poser une retenue (date, lieu) et voir les retenues du jour | 13 D11 | | ★ |
| MQ-144 | Localiser un élève (séance prévue, dernière présence du jour) | 12, 10 | | |
| MQ-145 | Message à une famille, réponse à un fil de la boîte de service vie scolaire | 16 M12 | | ★ |
| MQ-146 | Accorder une sortie exceptionnelle signée par la famille (autorisation reçue) | 18 FO10, 4 | | ★ |
| MQ-147 | Comptage et présents lors d'une alerte | 14 AS10 | É | |

### 3.5 AED, vie scolaire
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-148 | Billet de retard à l'arrivée (heure serveur, motif dit par l'élève) | 12 | É | |
| MQ-149 | Appel d'étude, de permanence, de cantine (si l'établissement le fait) | 12, 40 | É | ★ |
| MQ-150 | Surveillance d'une retenue : présence, fin | 13 D11 | É | ★ |
| MQ-151 | Contrôle de sortie à la grille : carte ou QR dynamique de l'élève → « peut sortir / ne peut pas » | 14 AS5, 18 FO10 | L (`accueil.secours`) | |
| MQ-152 | Voir les élèves de la zone dont j'ai la charge pendant mon service | 12, 14 | L | |
| MQ-153 | Téléphone de service : ma session personnelle par-dessus le poste, fin de service | 11 MB12 | | |

### 3.6 Surveillant d'internat (nuit)
Ce qu'il redoute : le réseau absent dans les étages, l'élève absent de sa chambre à 22 h sans savoir s'il a une autorisation, le téléphone perdu avec la liste.
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-154 | Appel du soir par étage ou chambre, hors ligne | 41 I10 | É (`internat.nuit`) | ★ à la clôture |
| MQ-155 | Voir les autorisations de la soirée (sortie, retour tardif, absence prévue) | 41 | L | |
| MQ-156 | Signaler un absent → alerte à l'astreinte et à la famille selon la procédure | 41 I10, 8 | É (mise en file) | ★ |
| MQ-157 | Ronde : passage noté, incident, fiche de nuit | 41 I10 | É | |
| MQ-158 | Fiche d'urgence d'un interne (allergies, contacts) en cas de malaise | 34, 41 | L (déverrouillage à chaque ouverture) | |
| MQ-159 | Relève : transmettre la fiche de nuit au service du matin | 41 | É | ★ |

### 3.7 Accueil
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-160 | Remise d'un élève à un adulte : identité vérifiée, autorisé ou non, **interdictions toujours en tête** | 4 A13, 14 | L | ★ |
| MQ-161 | Visiteur : arrivée, badge, départ | 14 AS5 | É | |
| MQ-162 | Saisir l'appel d'un parent (« Léa sera absente ») → transmis à la vie scolaire | 12 AS5 | É | |
| MQ-163 | Poste d'accueil partagé : session courte, changement d'agent | 11 MB12 | | |

### 3.8 Direction (chef, adjoint)
Ce qu'elle redoute : déclencher une alerte depuis la cour, découvrir à 10 h un absent non signalé, une diffusion partie par erreur.
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-164 | Tableau du jour : effectifs présents, absences du personnel et remplacements, appels non faits, incidents, retenues, visites, sorties en cours | 29 P8, 47, 15 | L | |
| MQ-165 | Alertes : élève non retrouvé, incident grave, signalement, sortie en difficulté | 29, 13, 42 | | |
| MQ-166 | Déclencher une alerte de sûreté (deux gestes + passkey) et suivre le comptage | 14 AS10 | É | ★ |
| MQ-167 | Valider un remplacement, une absence de personnel, une sortie (demandes en attente) | 15, 42, N1 | | ★ |
| MQ-168 | Lire et répondre à un message, valider une annonce préparée | 16, 17 | | ★ |
| MQ-169 | Diffusion d'urgence (fermeture, intempérie) depuis un modèle préparé au web | 8, 17 | | ★ (deux personnes si paramétré) |
| MQ-170 | Révoquer l'appareil perdu d'un personnel, voir l'inventaire des données qu'il contenait | 11 MB14 | | ★ |
| MQ-171 | Appeler l'astreinte, voir qui est d'astreinte | 6, 41 | | |

### 3.9 Secrétariat, administratif
Presque tout au web (poste fixe). Sur mobile :
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-172 | Messages de la boîte de service, réponse simple | 16 | | ★ |
| MQ-173 | Pointer la présence d'une famille à un rendez-vous d'inscription (QR) | 3 I15, 19 | | |
| MQ-174 | Voir l'état du compte d'une famille qui appelle (« invitation envoyée, non activée ») et renvoyer l'invitation | 1, 11 MB3 | | ★ |

### 3.10 Infirmière
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-175 | Registre rapide d'un passage (arrivée, sortie, retour en classe) → met à jour l'appel | 34 S11, 12 | | |
| MQ-176 | Voir qui est à l'infirmerie maintenant | 34 S11 | | |
| MQ-177 | Remise d'un élève malade à sa famille (avec l'accueil) | 34, 4 | | ★ |
| *(aucune)* | Dossier de santé, PAI, ordonnances en lecture : **web seulement** ; aucune copie locale d'un document du cabinet (34 S11) | | | |

### 3.11 AESH
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-178 | Ma journée : séances des élèves accompagnés, salles, changements | 10, 35 | L | |
| MQ-179 | Être prévenue de l'absence de l'élève accompagné | 12, 8 | | |
| MQ-180 | Aménagements utiles en classe de l'élève accompagné (rien d'autre du dossier) | 35, 2 E10 | | |

### 3.12 Encadrant de sortie ou de voyage
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-181 | Appel de départ, d'étape, de retour, hors ligne | 42 V6 | É (`appel.activite`) | ★ |
| MQ-182 | Fiches d'urgence des participants (déverrouillage à chaque ouverture) | 34, 42 | L (`sortie.urgence`) | |
| MQ-183 | Point de situation déclaratif (« arrivés à Rome »), diffusion aux familles préparée | 42 V6, 8 | É | ★ |
| MQ-184 | Signaler un incident, un élève manquant | 42 V6 | É | ★ |
| MQ-185 | Tout effacer au retour (automatique, ou purge par la direction depuis la France) | 11 | | |

### 3.13 Agent technique, restauration, documentaliste
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-186 | Agent : demandes d'intervention du jour, prise en charge, clôture avec photo | 44 | | |
| MQ-187 | Restauration : passage au self par QR de l'élève, régime et allergènes **en filtre local** | 40 R10 | É (`restauration.self`) | |
| MQ-188 | Restauration : effectifs prévus du service | 40 R10 | L | |
| MQ-189 | CDI : prêt et retour par scan ISBN et carte, comptoir hors ligne | 45 BIB9 | É | |
| MQ-190 | CDI : récolement au scan | 45 BIB9 | É | |

### 3.14 Famille (responsable légal ; dont séparé, sans exercice de l'autorité, interdit, allophone, deux établissements)
Ce qu'elle redoute : une appli par établissement, apprendre l'absence par l'enfant, que l'autre parent voie ce qu'elle a écrit, tout en français, la notification à 23 h. Ce qui la ravirait : une appli, trois enfants, sa langue, « Aujourd'hui » de la fratrie, justifier en deux gestes.
Règle de lecture : chaque ligne est **soumise aux droits du lien** (socle 4 : voir les notes, justifier, autoriser, recevoir les documents…) ; un parent sans le droit ne voit pas l'écran ; un parent interdit n'a plus d'accès (purge < 1 min, R-MO42).
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-191 | « Aujourd'hui » de toute la fratrie (absences, retards, travail pour demain, messages, à faire) | 29 P8 | L (24 h) | |
| MQ-192 | Être prévenue d'une absence ou d'un retard (avis par foyer, demi-journée) | 12 AS5, 8 | | |
| MQ-193 | **Justifier une absence** : motif dans la liste, commentaire, pièce (photo) | 12 AS14c | | ★ |
| MQ-194 | Prévenir d'une absence à venir (rendez-vous médical) | 12 AS14c | | ★ |
| MQ-195 | Voir l'historique des absences et retards de l'enfant et leur état | 12 AS14c | | |
| MQ-196 | Voir l'emploi du temps du jour et du lendemain, les cours annulés | 10, 29 | L (jamais au-delà du jour, R-PO33) | |
| MQ-197 | Voir le travail à faire et le contenu des séances | 20 C11 | L | |
| MQ-198 | Voir les **notes publiées** et les moyennes expliquées ; jamais une appréciation avant le conseil | 21 V9 | | |
| MQ-199 | Voir les bulletins publiés (PDF en mémoire), télécharger explicitement | 23 B12, 7 | | |
| MQ-200 | Voir les positionnements de compétences publiés | 22 C10 | | |
| MQ-201 | Lire et répondre à un message, écrire à un personnel permis par la matrice | 16 M12 | É (brouillon) | ★ |
| MQ-202 | Lire une annonce, en accuser réception, répondre à une attente (« je viendrai ») | 17 AN10 | | ★ si réponse |
| MQ-203 | Donner ou refuser une autorisation (sortie, photo, voyage, sortie exceptionnelle) | 18 FO10, 42 | | ★ |
| MQ-204 | Signer un document (passkey native) | 7 D13 | | ★ |
| MQ-205 | Déposer une pièce demandée (photo de document, recto/verso) | 3 I15, 7 D13 | | ★ |
| MQ-206 | Voir une retenue, une mesure, une convocation, en accuser réception | 13 D11 | | ★ |
| MQ-207 | Prendre un rendez-vous avec un professeur, une réunion parents-professeurs | 19 AG8 | | ★ |
| MQ-208 | Rejoindre une visio de rendez-vous | 33 VI9 | | |
| MQ-209 | **Paiement simple** : voir ce qui est dû (cantine, sortie, facture), payer par la **page hébergée du prestataire** ouverte dans le navigateur du système ; aucune donnée de carte dans l'appli ni en base | 24, 25 P8 | | ★ |
| MQ-210 | Réserver ou annuler un repas, voir le menu et les allergènes | 40 R10 | | ★ |
| MQ-211 | Voir les personnes autorisées à récupérer l'enfant ; en ajouter une ponctuellement (durée bornée) | 4 A13 | | ★ |
| MQ-212 | Corriger mes coordonnées (demande soumise à la validation du secrétariat si paramétré) | 4 A13 | | ★ |
| MQ-213 | Mes démarches : réinscription, fiche de rentrée, pièces manquantes, état | 3 I15 | | ★ |
| MQ-214 | Voir l'équipe de mon enfant (professeur principal, CPE) et comment les joindre | 6 P12 | | |
| MQ-215 | Suivre une sortie ou un voyage : programme, points de situation | 42 V6 | | |
| MQ-216 | Internat : autorisations de sortie du week-end, retours | 41 | | ★ |
| MQ-217 | Passer d'un enfant à l'autre, d'un établissement à l'autre | 11 | | |
| MQ-218 | Choisir ma langue ; les décisions restent en français avec traduction marquée | 11, 29 | | |
| MQ-219 | Mode discret (prénoms seulement, pas de photo) en public | 29, 11 | | |
| MQ-220 | Mes appareils, déclarer un téléphone perdu, mes notifications par catégorie | 11 | | ★ |

Cas propres aux familles, réglés côté serveur, jamais par l'appli : parent séparé (deux foyers, deux fils de messages, deux avis, les mêmes documents à chacun, aucune adresse ni téléphone de l'autre foyer affichés) ; parent sans exercice de l'autorité parentale (droits du lien réduits, socle 4) ; tiers autorisé à récupérer (aucun compte famille : il est vérifié à l'accueil) ; élève devenu majeur (droits des parents recalculés au prochain contact, message neutre, R-MO14).

### 3.15 Élève mineur (collège, lycée)
Principe (L511-5, loi du 24/08/2026, R-MO22) : **aucune fonction n'exige le téléphone pendant le temps scolaire** ; l'appli met en avant le soir (« pour demain ») et le matin ; aucune notification non urgente pendant ses séances.
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-221 | Ma journée et demain (séances, salles, cours annulés, remplacements) | 10, 29 | L (`edt.jour`) | |
| MQ-222 | Mon travail à faire, par date, avec la charge de la semaine | 20 C11 | L | |
| MQ-223 | Rendre un devoir (photo → PDF, fichier) avant l'échéance ; accusé de dépôt horodaté | 20 C11, 7 | | ★ |
| MQ-224 | Mes notes publiées et mes moyennes expliquées | 21 V9 | | |
| MQ-225 | Mes compétences, mes bulletins | 22, 23 B12 | | |
| MQ-226 | Mes messages avec les personnels (fils encadrés ; pas de messagerie entre élèves) | 16 M12 | | ★ |
| MQ-227 | Les annonces qui me concernent | 17 AN10 | | |
| MQ-228 | Mes absences et retards, leur état (sans les échanges du parent avec la CPE) | 12 AS14c | | |
| MQ-229 | Ma carte d'élève numérique (QR dynamique) en **complément** de la carte imprimée | 14 AS5 | L | |
| MQ-230 | Réserver un repas, voir le menu | 40 R10 | | ★ |
| MQ-231 | Mes emprunts au CDI | 45 | | |
| MQ-232 | Ma fiche d'orientation (lecture, vœux saisis au web) | 23 B12, 37 | | |
| MQ-233 | Rejoindre une visio (cours à distance, SAPAD) | 33 VI9 | | |
| MQ-234 | Signaler un harcèlement (rappels 3018 et 119, fil protégé) | 16 M10 | | ★ |

### 3.16 Élève majeur
Comme l'élève mineur, plus :
| N° | Action | Socle · lot | HL | ★ |
|---|---|---|---|---|
| MQ-235 | Justifier mes absences moi-même | 12 AS14c | | ★ |
| MQ-236 | Décider de ce que mes parents continuent de voir (réglages de majorité) | 4 A13 | | ★ |
| MQ-237 | Donner mes autorisations, signer, payer si je suis le payeur | 18, 7, 25 | | ★ |
| MQ-238 | Mes démarches (réinscription, pièces) | 3 I15 | | ★ |

### 3.17 Gestes communs à tous
| N° | Action | Socle · lot |
|---|---|---|
| MQ-239 | Rejoindre un établissement (QR, lien, recherche), confirmation du nom officiel | 11 MB3 |
| MQ-240 | Se connecter par passkey, enrôler une passkey, récupération | 11 MB3, 1 |
| MQ-241 | Déverrouiller l'appli (biométrie, code) | 11 MB4 |
| MQ-242 | Changer de contexte (établissement, casquette, enfant) | 11 MB5 |
| MQ-243 | Centre d'avis, « pourquoi ai-je reçu ceci ? » | 8, 11 MB9 |
| MQ-244 | « Hors ligne et envois » : ce qui est sur le téléphone, jusqu'à quand, envois en attente, conflits, tout effacer | 11 MB7 |
| MQ-245 | Mes appareils, sessions, révoquer, déclarer perdu | 11 MB4, MB14 |
| MQ-246 | Réglages (langue, taille, contraste, aperçus, sons, mode discret) | 11 MB13 |
| MQ-247 | Aide, contact de l'établissement, envoyer un diagnostic sans donnée personnelle | 11 MB14 |
| MQ-248 | À propos : version, notice RGPD de l'établissement, conformité d'accessibilité, licences | 11 MB15 |
| MQ-249 | Demander la fermeture de mon compte (traitée par l'établissement) | 11 MB15 |
| MQ-250 | Mise à jour exigée : écran clair, ce qui reste possible | 11 MB14 |

---

## 4. Ce qui reste sur le web

Critère : un geste reste au web s'il demande un **grand écran** (tableaux, comparaisons), une **réflexion longue**, une **double validation**, un **paramétrage** qui engage l'établissement, ou une **donnée trop sensible** pour un appareil personnel. L'appli affiche alors « À faire sur ordinateur » avec un lien qui ouvre le web au bon endroit (le lien ne transporte aucune session : la personne se connecte par passkey sur l'ordinateur).

| Domaine | Reste au web | Pourquoi |
|---|---|---|
| Paramétrage (tous socles) | rôles et droits, politique mobile, plafonds, modèles, calendriers, structures | engage l'établissement, double contrôle |
| Imports, exports, reprise | SIECLE, STS, Charlemagne, portabilité | volumétrie, vérification |
| EDT (10) | construction, vœux, publication, modifications en série | grand écran |
| Services, personnels (6, 15) | services, missions, contrats, absences longues, planning des remplacements | tableaux |
| Inscriptions (3) | instruction des dossiers, commissions, répartition des classes | comparaison, décision |
| Évaluations (21) | création d'évaluation à barème complexe, arrêt de période, recalculs | contrôle, lisibilité |
| Bulletins, conseils (23) | appréciations, conseil de classe, décisions d'orientation | réflexion, séance |
| Discipline (13) | qualification, procédure disciplinaire, conseil de discipline, sanctions | procédure contradictoire |
| Vie scolaire (12) | régularisations en masse, statistiques, signalement d'absentéisme | décision, volumétrie |
| Messagerie (16) | campagnes, diffusion large, supervision motivée | aperçu, empreinte, confirmation |
| Formulaires, documents (18, 7) | conception de formulaire, parapheur complexe, coffre complet | grand écran |
| Santé (34), suivi social (36), inclusion (35) | dossiers, PAI, informations préoccupantes | sensibilité ; seule la fiche d'urgence de sortie part hors ligne |
| Finances, paiements, bourses (24, 25, 26), comptabilité (48), paie (49) | facturation, échéanciers, remises, rapprochements, écritures, paie | double validation, montants |
| Sorties, voyages (42) | montage, budget, autorisations en série, dossier préfecture | procédure |
| Internat (41) | répartition des chambres, règlement, statistiques | grand écran |
| RGPD, archives (27) | demandes d'accès, purges, registre, AIPD | DPO |
| Pilotage (47) | tableaux de bord détaillés, exports | grand écran ; le **tableau du jour** est mobile (MQ-164) |
| Bureautique (32) | Wordcraft, GridCraft, DeckCraft | ArtCraft en iframe WASM, poste de travail |
| Assistant IA (30) | conversation, brouillons longs | l'IA n'a jamais plus de droits que la personne ; dictée locale seulement sur mobile |

---

## 5. Contrat d'API mobile

### 5.1 Principes [R]
1. **Une seule API.** L'appli appelle les routes des socles, celles du web. Les seules routes propres au mobile sont celles de la coquille : `/mobile/contexte`, `/mobile/appareils*`, `/mobile/baux*`, `/mobile/lots*`, `/mobile/purges*`, `/mobile/diagnostics`, plus `/.well-known/assetlinks.json` et `/.well-known/apple-app-site-association`. Toute autre route « mobile » demande une justification écrite dans la conception du socle (R-MO51).
2. **Aucune logique de droit dans l'appli.** L'appli lit le manifeste, affiche, et traite proprement un 403/404 ; le serveur refait **tous** les contrôles (y compris les pré-contrôles de confort).
3. **Mêmes DTO** (`ent-types`) que le web. Un DTO allégé n'existe que si la minimisation l'exige (fiche élève E10) ; il est alors une **vue serveur** (`?vue=resume`), pas un filtrage dans l'appli.
4. **Fixtures dorées** (D-062) : chaque test de route du serveur qui sert le mobile écrit sa réponse type dans `ent-rust/contrat/mobile/<route>/<cas>.json` ; les tests JVM (Kotlin) et `swift test` décodent toutes les fixtures ; une fixture modifiée sans mise à jour des deux cœurs fait échouer la CI. Les fixtures ne contiennent que des données du labo.
5. **Compatibilité ascendante** : un champ ajouté est toujours facultatif pour les versions supportées ; un champ retiré ou dont le sens change exige une nouvelle version de route (`/v2/…` pour cette route seulement) maintenue tant qu'une version d'appli supportée l'appelle.

### 5.2 En-têtes
| En-tête | Sens | Obligatoire |
|---|---|---|
| `Authorization: Bearer <jeton>` | session mobile | oui (hors rattachement et connexion) |
| `X-ScolIA-Appli: android/1.4.2 (build 142)` | plateforme et version ; base du `426` et des statistiques techniques (aucun identifiant d'appareil) | oui |
| `X-ScolIA-Signature` | signature de la clé d'appareil (méthode, chemin, empreinte du corps, horodatage serveur récent, drapeau de présence) — format proche de DPoP (RFC 9449), fixé au lot MB4 | sur toute écriture, lot, renouvellement de bail ; **avec présence** pour les gestes ★ |
| `Idempotency-Key: <uuid v4>` | toute écriture (`POST`, `PUT`, `PATCH`, `DELETE`) depuis le mobile ; même clé + même corps = même réponse rejouée pendant 24 h ; même clé + corps différent = `409 idempotence.conflit` | oui sur écriture |
| `If-None-Match` / `ETag` | lectures mises en cache | recommandé |
| `If-Match` | modification d'un objet versionné (brouillon, saisie de notes, fin de séance) | oui sur ces routes |
| `Accept-Language` | langue des **libellés** renvoyés (les codes d'erreur ne changent pas) | non |

### 5.3 Enveloppe d'erreur codée
Remplace `(StatusCode, String)` pour **toutes** les routes servies au mobile (et, à terme, au web) :
```json
{ "erreur": { "code": "absence.fenetre_depassee",
              "message": "Le délai pour justifier cette absence est dépassé.",
              "champs": [ { "champ": "motif", "code": "champ.requis" } ],
              "reessayer_apres_s": null,
              "trace": "01JD…" } }
```
- `code` : identifiant stable `domaine.cause`, catalogue dans `ent-types::erreurs` (un `enum` Rust sérialisé en texte), seule chose que l'appli interprète ; le message est affiché tel quel ou remplacé par la traduction locale du code.
- `trace` : identifiant de requête pour le support (corrélé au journal serveur), jamais une donnée personnelle.
- Le message **ne révèle jamais** la cause d'un refus d'accès à une personne (« accès non disponible », jamais « vous êtes visé par une interdiction »).

| HTTP | Quand | Ce que fait l'appli |
|---|---|---|
| 400 | requête mal formée, champ invalide (`champs`) | affiche les champs en erreur |
| 401 `session.expiree` / `session.revoquee` | session absente, expirée, révoquée | **purge** des baux de la session, retour à la connexion |
| 403 `droit.refuse` | action non permise sur une cible **dont l'existence est déjà connue** de la personne (ex. : écran retiré du manifeste entre-temps) | message neutre, rafraîchit le manifeste |
| 403 `presence.requise` | geste ★ sans signature de présence | redemande la biométrie |
| 403 `appareil.non_lie` | écriture sans signature valide | refait l'inscription de l'appareil |
| 404 `objet.introuvable` | objet inexistant **ou hors droits** (pas d'oracle d'existence) | message neutre |
| 409 `version.conflit` (avec `version_courante`) | `If-Match` périmé | propose de recharger, garde la saisie locale |
| 409 `idempotence.conflit` | même clé, corps différent | erreur de programmation : diagnostic |
| 412 | précondition échouée | idem 409 |
| 422 `<domaine>.<regle>` | règle métier (période arrêtée, fenêtre dépassée, plage calme pour un envoi…) | message du code |
| 423 `objet.verrouille` | objet figé (appel clos hors fenêtre, bulletin) | lecture seule |
| 426 `appli.trop_ancienne` | version sous le minimum | écran de mise à jour |
| 429 `debit.depasse` (+ `reessayer_apres_s`) | limite de débit | attend, ne rejoue pas en boucle |
| 503 `service.maintenance` (+ `Retry-After`) | maintenance | bandeau, hors ligne si bail |

### 5.4 Pagination
- **Curseur opaque** (`?apres=<curseur>&limite=50`, `limite` ≤ 100) ; réponse `{ "elements": [...], "suivant": "<curseur>|null" }` ; tri stable déclaré par la route ; aucun `offset` sur les listes qui bougent (messages, avis, absences).
- Listes bornées par nature (élèves d'une séance, séances du jour) : non paginées, mais plafonnées côté serveur (ex. 300) avec `422 liste.trop_longue` au-delà.
- Recherche : 2 caractères au moins, 20 résultats au plus, limitée à la portée de la personne.

### 5.5 Idempotence et opérations hors ligne
- En ligne : `Idempotency-Key` (registre côté serveur, table 2227 de `11_MOBILE.md`, durée 24 h).
- Hors ligne : **lot** `POST /mobile/lots` = liste d'opérations déclarées (`operation_id` UUID, type, objet, champs, heure appareil, heure serveur de référence, version de base), signé par la clé d'appareil ; réponse ligne par ligne (`appliquee`, `appliquee_avec_mention`, `refusee` + code). Un lot rejoué ne crée aucun doublon ; un lot d'un appareil révoqué est refusé et journalisé.
- Les types d'opérations différées sont **déclarés** par les socles (appel, retard, comptage, appel d'activité, appel de nuit, ronde, passage au self, prêt CDI, brouillons) ; tout autre geste exige le réseau.

### 5.6 Formats
- Dates et heures : ISO 8601 avec décalage (`2027-03-14T08:05:00+01:00`) ; dates seules (`2027-03-14`) au **jour de l'établissement** ; format figé par le test de non-régression de D-067.
- Identifiants : UUID en texte ; aucun identifiant interne séquentiel exposé.
- Montants : entiers en centimes, devise explicite.
- Textes : UTF-8, NFC ; libellés d'établissement servis tels quels.
- Fichiers : jamais dans le JSON ; URL pré-signée courte (≤ 5 min) obtenue par une route contrôlée ; téléversement en deux temps (demande d'URL, dépôt, confirmation avec empreinte).

### 5.7 Notifications (charge)
`{ "avis": "<uuid>", "etab": "<nom public>", "cat": "absences|messages|travail|rdv|securite|urgence" }` et rien d'autre. Aucun nom d'élève, aucun sujet, aucun extrait. L'aperçu C1, s'il est permis, est récupéré par l'extension par `GET /notifications/avis/{id}/apercu` avec la session de l'appareil.

### 5.8 Débit, taille, délais
Limites par session et par appareil (lecture 600/min, écriture 60/min, lots 10/min) [R, paramètre plateforme] ; corps ≤ 1 Mo hors fichiers ; délai serveur ≤ 10 s, l'appli abandonne à 15 s et bascule sur le hors ligne si le profil le permet.

---

## 6. Routes historiques et manques serveur par socle

### 6.1 Routes aujourd'hui appelées par l'appli, et leur remplacement
| Route historique [F, `MOBILE.md` §2] | Problème connu | Cible | Lot |
|---|---|---|---|
| `POST /auth/login`, `/auth/passkey/*` avec `"client":"mobile"` | durée mobile déclarée par le client (DS2) | même route ; durée mobile **seulement** si passkey + clé d'appareil | MB0, MB4 |
| `POST /devices/push` | jeton en clair, renvoyé, réaffecté sans trace, pas de désinscription, pas lié à la session (DS1, DS3, DS4) | `/mobile/appareils/{id}/notifications` (socle 8, `ent_ntf_appareil`) | MB0 puis MB9 |
| `GET /roles/mes-droits` + `Capacites` dans l'appli | droits recalculés dans l'appli | `GET /mobile/contexte` (manifeste) | MB5 |
| `GET /edt/mes-creneaux?jour=` | créneaux sans la séance stable du socle 10 | séances du jour du socle 10 (`/edt/seances?jour=`) [H sur le nom exact, fixé par T12] | MB6 |
| `POST /appels`, `/appels/{id}/lignes`, `/appels/{id}/clore` | sans idempotence ni opération différée | routes du socle 12 (AS2) + `/mobile/lots` (AS10) | MB6, MB7 |
| `GET /vs/absences` | liste sans file ni portée fine | file du matin du socle 12 (AS14b) | MB11 |
| `GET /famille/enfants`, `/famille/enfants/{id}/resume` | droits du lien partiellement appliqués (16 D3, 21 D3) | « Aujourd'hui » du socle 29 + routes par lien du socle 4 | MB8 |
| `POST /absences/{id}/justifier` | ni présence du titulaire ni idempotence | route du socle 12 (AS14c), geste ★ | MB8 |
| `GET /eleves/{id}/notes` | appréciations lisibles avant le conseil, bulletin sans droit (21 D3) | relevé publié du socle 21 (V9) | MB8 |
| `POST /eleves/{id}/notes`, `GET /devoirs`, `/note-periodes` | saisie hors service et hors période, notes écrasées sans historique (21 D1, D6, D7) | saisie du socle 21 (V3, V9) avec version et `If-Match` | MB6 |
| `GET /classes/{classe}/effectif?jour=` | effectif sans portée D-022 fine | élèves de la séance (socles 10, 12) | MB6 |
| `GET /cahier-texte` | contenu régénéré, copies lisibles par des tiers (20 C0) | `/cahier/*` du socle 20 (C11) | MB6, MB8 |
| `GET /eleves/{id}/edt` | EDT au-delà du jour pour un responsable (R-PO33) | « Aujourd'hui » et EDT du jour (29, 10) | MB8 |
| `GET /messages/fils*`, `POST …/repondre` | lecture déduite du lien avec l'élève (16 D1–D3), sujet dans la notification (D4) | participants nommés du socle 16 (M1, M12) | MB6, MB8 |
| `GET /annonces` | sans aperçu ni retrait (16) | fil du socle 17 (AN10) | MB6 |
| `GET /auth/sessions`, `DELETE /auth/sessions/{id}` | sessions sans appareil | `/mobile/appareils` (sessions par appareil, inventaire des baux) | MB4 |

Règle : **l'appli ne branche aucun nouvel écran sur une route historique** ; une route historique n'est retirée qu'après la version d'appli qui ne l'appelle plus + N-2 (R-MO52).

### 6.2 Ce qui manque côté serveur, socle par socle
« Existe » = route présente sur `ent-mc0hoe` ; sinon le lot du socle qui la livre. Je n'ai vérifié le code que pour les lignes marquées [F].
| Socle | Besoin mobile | État | Lot du socle | Lot MB qui l'attend |
|---|---|---|---|---|
| 1 sécurité | durée mobile liée à la passkey (DS2), révocation en cascade (DS1), enveloppe d'erreur codée, `Idempotency-Key` générique | manque [F] | 11 MB0, MB1 | MB0, MB1 |
| 1 / 11 | clé d'appareil, signature, présence, `/mobile/appareils` | manque | 11 MB4 | MB4 |
| 2 élève | fiche allégée `?vue=resume` minimisée | manque | 2 E10 | MB6 |
| 4 administratif | remise, autorisés, autorisation ponctuelle, droits par lien lus par le manifeste, majorité | partiel (A0–A1 versés) | 4 A13 | MB8, MB12 |
| 6 personnels | casquettes dans la session et sélecteur, « où est-il », équipe de mon enfant | partiel (casquette de session existe) | 6 P12 | MB5, MB11 |
| 7 documents | URL pré-signées, signature par passkey native, dépôt photo | manque | 7 D13 | MB8 |
| 8 notifications | `ent_ntf_appareil`, APNs, FCM, aperçu C1, plage calme, retenue pendant les cours | manque (`ent-push` sans connecteur) | 8 N7 | MB9 |
| 9 interop | codes de rattachement famille / élève, guichet central, fédération mobile | manque | 9 §22 | MB3 |
| 10 EDT | séances du jour et du lendemain d'une personne, changements | partiel | 10 T12 | MB6 |
| 12 assiduité | appel (AS2), opérations différées (AS10), justification (AS14c), file du matin (AS14b), avis par foyer (AS5) | partiel (appel historique) | 12 AS2, AS5, AS10, AS14 | MB6, MB7, MB8, MB11 |
| 13 discipline | observation, incident, exclusion de cours, retenue | partiel (historique) | 13 D11 | MB11 |
| 14 accueil, sûreté | QR dynamique, contrôle de sortie hors ligne, comptage | manque | 14 AS5, AS10 | MB12 |
| 15 absences des personnels | déclarer mon absence, accepter un remplacement | manque | 15 A12 | MB11 |
| 16 messagerie | participants nommés, boîtes de service, brouillons, SSE ou rafraîchissement | manque (historique fautif) | 16 M0a, M1, M12 | MB6, MB8 |
| 17 annonces | fil, prise de connaissance, attentes | manque | 17 AN10 | MB6 |
| 18 formulaires | autorisations, accords, verdicts de sortie | manque | 18 FO10 | MB8 |
| 19 agenda | rendez-vous, émargement | manque | 19 AG8 | MB16 |
| 20 cahier | fin de séance, travail, dépôts, charge | manque (historique fautif) | 20 C0, C11 | MB6, MB8, MB10 |
| 21 évaluations | saisie versionnée, publication, relevé publié | manque (historique fautif) | 21 V0, V3, V9 | MB6, MB8 |
| 22 compétences | positionnement rapide, lecture | manque | 22 C10 | MB16 |
| 23 bulletins | bulletins publiés, fiche d'orientation | manque | 23 B12 | MB10 |
| 25 paiements | ce qui est dû, lien vers la page hébergée | manque (prestataire à choisir, Q-P1 du 25) | 25 P8 | MB16 |
| 29 portails | « Aujourd'hui » et à-faire, pastille | **existe** (P1) [F] ; mobile P8 manque | 29 P8 | MB6, MB8 |
| 33 visio | rejoindre | manque | 33 VI9 | MB16 |
| 34 santé | registre rapide, fiche d'urgence de sortie | manque | 34 S11 | MB16 |
| 40 restauration | réservation, self hors ligne | manque | 40 R10 | MB16 |
| 41 internat | appel de nuit hors ligne, ronde, fiche de nuit | partiel (I2 en cours) | 41 I10 | MB7 |
| 42 sorties | appel d'étape hors ligne, fiches d'urgence, point de situation | manque | 42 V6 | MB16 |
| 45 CDI | prêt, retour, récolement | manque | 45 BIB9 | MB16 |
| 47 pilotage | tableau du jour (lecture) | manque [H] | 47 (à déclarer) | MB11 |

---

## 7. Parité des droits

### 7.1 Règle
Pour toute action d'un socle servie au mobile : **même réponse** (statut, code, contenu) qu'au web, pour la même personne, la même casquette, la même cible, au même instant. Les seules différences permises sont celles de la coquille : la durée de session, la signature d'appareil, la présence du titulaire (plus stricte sur mobile), les baux. R-MO33 de `11_MOBILE.md`, précisée par R-MO53.

### 7.2 Banc de parité
1. **Où** : `ent-rust/crates/ent-api/tests/parite_mobile.rs` [R], sur le modèle de `web/crates/ent-web-server/tests/droits_parite.rs` [F].
2. **Matrice** : personas du labo (professeur, professeur d'une autre classe, remplaçant en mission et hors mission, CPE, AED, direction, secrétariat, infirmière, parent avec droits complets, parent sans droit aux notes, parent séparé de l'autre foyer, parent interdit, élève mineur, élève majeur, personne d'un autre établissement) × **chaque** écran et opération déclarés au manifeste × cibles (élève suivi, non suivi, autre établissement, objet inexistant).
3. **Deux clients** : cookie web (session web) et Bearer mobile (session mobile signée) ; comparaison des réponses normalisées (horodatages et `trace` masqués).
4. **Manifeste** : pour chaque persona, l'ensemble des écrans du manifeste = ensemble des écrans dont **au moins une** action est permise par `autoriser` ; un écran présent sans action permise, ou l'inverse, fait échouer le test.
5. **Profils hors ligne** : le contenu d'un bail ⊆ ce que la même personne lirait en ligne au même instant (test de propriété) ; aucun champ non déclaré par le profil.
6. **Opérations différées** : une opération reçue après le retrait du droit est refusée (droits relus **à la réception** et à la date de l'objet).
7. **CI** : le banc tourne à chaque lot d'un socle qui déclare un écran mobile ; un socle ne peut pas déclarer d'écran sans ligne dans la matrice (test de complétude).

### 7.3 Ce que l'appli teste de son côté
Décodage de toutes les fixtures dorées ; comportement sur chaque code d'erreur du §5.3 ; manifeste inconnu (écran ignoré) ; purge sur 401 ; aucune donnée expirée affichée (horloge truquée) ; aucun écran affiché hors manifeste.

---

## 8. Modèle de menace, minimisation, journal, révocation

### 8.1 Menaces
Gravité résiduelle : **F** faible, **M** moyenne, **É** élevée.
| N° | Menace | Scénario | Mesures | Test | Résiduel |
|---|---|---|---|---|---|
| MEN-01 | Appareil perdu ou volé, déverrouillé | professeur qui oublie son téléphone ouvert en salle | verrou de l'appli après 5 min d'arrière-plan (S-1101), écrans nominatifs masqués hors premier plan, présence exigée pour les gestes ★, baux courts | élève qui prend le téléphone posé : ne peut clore un appel ni écrire | M |
| MEN-02 | Appareil perdu ou volé, verrouillé, attaque hors ligne | extraction du stockage | baux chiffrés par clé enveloppée par une clé matérielle liée au déverrouillage ; expiration par la clé ; aucun secret en clair ; révocation + purge ; inventaire pour le DPO | extraction sur appareil de labo rooté : rien de lisible | F |
| MEN-03 | **Enfant sur le téléphone d'un parent** | Inès connaît le code du téléphone et justifie elle-même son absence, autorise sa sortie | présence du titulaire **biométrique** exigée pour les gestes ★ (S-1109, défaut 0 s de grâce pour les familles, biométrie forte seulement si paramétré S-1113) ; avis au parent de chaque geste ★ fait (« Vous avez justifié… ») sur ses autres appareils et par e-mail | geste ★ sans présence → 403 `presence.requise` | M (si l'enfant est enrôlé dans la biométrie du téléphone, le système ne distingue pas : d'où l'avis systématique et S-1113 « biométrie de classe forte et liste des visages enrôlés à vérifier » proposé à la famille) |
| MEN-04 | Téléphone familial partagé entre frères et sœurs | caches mélangés | profils d'appareil séparés, un verrou par compte (R-MO40) | deux comptes sur un appareil : aucune donnée de l'un visible de l'autre | F |
| MEN-05 | **Parent séparé** | un parent tente de voir l'adresse, les messages ou les rendez-vous de l'autre foyer | fils, avis, documents par foyer calculés par le serveur ; coordonnées de l'autre foyer jamais servies ; aucune réunion entre foyers sur l'appareil ; même document à chacun | banc de parité : parent A ne lit rien du foyer B | F |
| MEN-06 | Parent interdit (décision de justice) | garde l'appli hors réseau | coupure serveur immédiate, purge au prochain contact (< 1 min si joignable), expiration du bail (24 h max pour la lecture famille), message neutre | lien suspendu → plus rien, aucune cause révélée | F |
| MEN-07 | Capture d'écran, enregistrement d'écran | liste d'appel ou notes partagées sur un réseau social | Android `FLAG_SECURE` sur les écrans nominatifs ; iOS : détection de capture + avertissement discret et contenu masqué à l'enregistrement d'écran ; filigrane léger du nom de l'utilisateur sur les listes d'élèves (S-1114, défaut activé pour les personnels) | capture Android bloquée ; enregistrement iOS masqué | M (iOS ne permet pas d'interdire la capture) |
| MEN-08 | Jailbreak / root | appareil modifié pour lire la mémoire | signal remonté et affiché, pas de blocage ; profils hors ligne C2+ refusés (S-1105) | appareil de labo rooté : aucun bail C2+ | M |
| MEN-09 | Faux QR, faux lien, sous-domaine piégé | hameçonnage de la passkey ou du mot de passe | rattachement strict, nom officiel confirmé par le guichet central, jamais de passkey dans une page web ouverte par QR (R-MO41) ; la passkey liée à `scolia.fr` ne fonctionne pas sur un autre domaine | QR piégé refusé | F |
| MEN-10 | Jeton Bearer copié (sauvegarde, journal, proxy) | script qui rejoue le jeton | jeton seul = lecture 15 min après la dernière requête signée ; écriture exige la clé d'appareil ; jeton jamais journalisé | jeton copié → écriture refusée | F |
| MEN-11 | Client qui se déclare « mobile » | session de 90 jours obtenue par un script | durée mobile seulement après passkey + clé d'appareil (DS2) | test MB0 | F après MB0 (**É aujourd'hui**) |
| MEN-12 | Réseau hostile (Wi-Fi public) | interception | HTTPS seul, autorités du système, aucune CA utilisateur, aucune redirection suivie | proxy de labo avec CA utilisateur : refus | F |
| MEN-13 | Lot hors ligne rejoué, falsifié, d'un appareil révoqué | doublons d'absences, appel antidaté | signature, idempotence, fenêtre, heure serveur de référence, droits relus | jeu d'attaques de `11_MOBILE.md` §28.3 | F |
| MEN-14 | Horloge de l'appareil truquée | garder un bail au-delà de minuit | horloge monotone + dernière heure serveur ; écart > 5 min signalé ; expiration côté clé | horloge reculée : bail expiré quand même | F |
| MEN-15 | Notifications lues sur l'écran verrouillé, chez Apple ou Google | nom d'enfant, sujet d'un message | charge opaque ; aperçu C1 seulement déverrouillé et si permis | « aucune donnée C2+ dans une charge » (test de propriété) | F |
| MEN-16 | Personnel qui exfiltre des listes | export, copie | aucune exportation de liste depuis le mobile ; presse-papiers interdit sur les données nominatives ; consultations sensibles tracées comme au web | copie impossible | M (photo d'écran par un autre appareil) |
| MEN-17 | Remplaçant ou agent en fin de contrat | garde l'accès | droits lus à chaque requête ; fin de mission → manifeste et purge ; fin de contrat → révocation générale (socle 6) | fin de mission simulée : plus aucune classe | F |
| MEN-18 | Élève contraint de montrer son appli (racket, conjoint violent pour un élève majeur) | lecture forcée | mode discret, aucune donnée sur l'écran verrouillé, verrou par compte ; signalement protégé (16 M10) | — | M |
| MEN-19 | Version obsolète avec faille | appli non mise à jour | version minimale, `426`, blocage de sécurité à deux personnes après 48 h | version bloquée → écran de mise à jour | F |
| MEN-20 | Lien profond vers l'objet d'un autre | `scolia://…/eleves/<autre>` | liens universels seulement ; l'objet est relu sous les droits ; 404 neutre | lien vers l'enfant d'une autre famille → 404 neutre | F |
| MEN-21 | Restauration de la base | appareil révoqué « ressuscité » | jetons et appareils neutralisés à la restauration, révocations rejouées depuis les ordres de purge (DS5) | restauration simulée | F |
| MEN-22 | Diagnostic ou journal contenant des données | fuite par le support | diagnostic à la demande, sans nom, sans jeton, relu par test | test de propriété sur le diagnostic | F |

### 8.2 Minimisation
Aucun SDK tiers, aucune mesure d'audience, aucun rapport de plantage externe, aucune géolocalisation, aucun accès aux contacts ni à la photothèque entière ; baux limités aux champs déclarés ; coordonnées des familles seulement dans la fiche de secours de l'accueil ; santé seulement dans la fiche d'urgence de sortie ; aucune donnée d'un établissement envoyée à un autre ; la réunion multi-établissements se fait sur l'appareil. Permissions demandées au moment du besoin (`11_MOBILE.md` §15).

### 8.3 Journal (côté serveur, jamais sur l'appareil)
| Événement | Contenu | Durée [R, à valider par le DPO, socle 27] |
|---|---|---|
| Inscription, révocation d'appareil, déclaration de perte | personne, appareil (nom donné), plateforme et version, acteur réel | 1 an après la révocation |
| Émission, renouvellement, purge de bail | profil, nombre de personnes, classe de sensibilité, dates | 1 an |
| Lot reçu | appareil, nombre d'opérations, issues, écart d'horloge | 1 an (les faits métier suivent la durée de leur socle) |
| Geste ★ | journal du socle propriétaire + drapeau « présence attestée » | durée du socle |
| Consultation sensible | comme au web (`tracer_acces_sensible`) | durée du web |
| Refus de signature, lot rejeté, appareil signalé | appareil, code, nombre | 1 an |
Le contenu métier n'est jamais copié dans le journal technique.

### 8.4 Révocation
Chemins : la personne (« Mes appareils », autre appareil, web), la direction (personnes de son établissement), la révocation générale (fin de contrat, compromission), l'expiration. Effet en **une transaction** : sessions de l'appareil, inscription de notification, baux, ordre de purge ; au prochain contact, purge confirmée et jointe à l'incident éventuel ; sans contact, expiration des baux (la plus longue : `sortie.urgence`, retour + 24 h ; sinon minuit ou 24 h). Le téléphone d'un personnel déclaré perdu produit l'**inventaire** et le compte rendu pré-rempli pour le DPO (`11_MOBILE.md` §12.7).

---

## 9. Plan de lots MB

Ce plan **remplace l'ordonnancement** du §27.1 de `11_MOBILE.md` ; le contenu des lots MO y reste détaillé (correspondance en colonne « MO »). Chaque lot livre : migrations dans 2220–2239, service en transaction unique, routes, `ent-types`, enveloppe d'erreur codée, fixtures dorées, tests Postgres (isolation entre établissements, matrice de droits), banc de parité, tests des deux cœurs, écrans Compose **et** SwiftUI, documentation (ce document, `MOBILE.md` qui devient le guide de build, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot, **revue sécurité** (grille MASVS) sur MB0, MB3, MB4, MB7, MB12, MB14. Effort : S ≤ 3 j, M 1–2 sem., L > 2 sem. [H]. Colonne **Où** : **L** = faisable et testable entièrement dans un conteneur Linux ; **L+M** = Linux, puis passe Mac pour l'interface iOS ; **A** = appareils réels nécessaires.

| Lot | Contenu | MO | Dépend de | Critère de « fini » | Où | Effort |
|---|---|---|---|---|---|---|
| **MB0 Correctifs** | DS1–DS5 (serveur), DA1 (expiration provisoire des caches : minuit pour appel et EDT, 24 h sinon), DA2 (`targetSdk 36`), retrait d'OkHttp (D-062) | MO0 | — | tests rouges avant, verts après (`11_MOBILE.md` §27.1 MO0) | L | M |
| **MB1 Intégration et CI** | fusion de `ent-mobile-app` sur la branche d'intégration (pas `ENT` sans accord) ; retrait des 4 prototypes ; CI Linux : `:core:test`, `:app:testDebugUnitTest` (Robolectric), `assembleRelease`, `lintDebug`, `swift test` du cœur ; **enveloppe d'erreur codée** et `Idempotency-Key` génériques côté serveur | MO1 | MB0 | CI verte ; plus aucun fichier des prototypes ; une route du labo renvoie l'enveloppe codée ; passe Mac : l'appli iOS compile et démarre dans le simulateur | L+M | M |
| **MB2 Contrat et parité** | fixtures dorées (`ent-rust/contrat/mobile/`) ; décodage par les deux cœurs ; banc `parite_mobile.rs` (matrice §7.2) sur les routes déjà appelées ; règle « aucun écran sur route historique » vérifiée en CI (liste blanche) | MO1 | MB1 | modifier un DTO sans mettre à jour les cœurs → CI rouge ; banc vert pour les personas du labo | L | M |
| **MB3 Domaines, passkeys, rattachement** | `assetlinks.json`, `apple-app-site-association` ; origines `android:apk-key-hash` ; annuaire public et confirmation par le guichet central ; codes de rattachement famille et élève (socle 9 ou repli local), invitations d'agents | MO2, MO3 | MB1 ; 9 §22 | passkey créée et utilisée sur un Android et un iPhone réels contre le labo ; QR piégé refusé ; aucun indice d'existence de compte | L (serveur, Android) + A | M |
| **MB4 Appareil et session liée** | `ent_mob_appareil`, politique (2221–2222) ; clé d'appareil, signature, **présence du titulaire** (R-MO48) ; durée mobile conditionnée ; « Mes appareils » ; clé du cache liée au déverrouillage | MO4 | MB3 | jeton copié sans clé → écriture refusée ; geste ★ sans présence → 403 ; appareil sans verrou → durée web ; révocation → purge | L+M + A | L |
| **MB5 Contexte et manifeste** | `GET /mobile/contexte` ; écrans déclarés par les socles ; casquettes, enfants, établissements ; onglets servis ; liens universels profonds ; mode discret | MO5 | MB4 ; 6 P2 ; 29 P1 [F] | rôle libre D-017 → bons écrans sans code mobile ; parent-professeur → deux contextes sans mélange ; lien profond hors contexte → 404 neutre | L+M | M |
| **MB6 Professeur, en ligne** | MQ-100 à MQ-105, MQ-111 à MQ-117, MQ-119, MQ-121, MQ-124, MQ-130 sur les **nouvelles** routes (12 AS2, 20 C11, 21 V3/V9, 16 M12, 17 AN10, 2 E10, 10, 29 P8) | — | MB5 ; 12 AS2 ; 20 C0 ; 21 V0 ; 16 M0a | journée, appel, fin de séance, notes, messages joués de bout en bout sur Android et iOS contre le labo ; banc de parité vert ; aucune route historique appelée | L+M | L |
| **MB7 Hors ligne** | baux (2224–2226), ordres de purge, démarrage à froid, écran « Hors ligne et envois » ; opérations différées (2227), lots signés, conflits ligne par ligne ; premiers clients : **appel** (12 AS10) et **appel de nuit** (41 I10) | MO7, MO8 | MB6 ; 12 AS10 ; 41 I10 | appel complet en mode avion, 0 perte, 0 doublon au retour ; bail effacé à minuit horloge truquée comprise ; inventaire exact d'un appareil déclaré perdu ; appel de nuit d'un internat de labo de 80 élèves en mode avion | L+M + A | L |
| **MB8 Famille** | MQ-191 à MQ-206, MQ-211, MQ-213, MQ-217 à MQ-220 (12 AS14c, 21 V9, 20 C11, 16 M12, 17 AN10, 18 FO10, 7 D13, 4 A13, 3 I15, 29 P8) ; gestes ★ avec présence | — | MB5 ; MB4 ; lots des socles cités | parcours « absence → avis → justification » ; parent sans droit aux notes : aucun écran notes ; parent séparé : rien de l'autre foyer ; deux établissements réunis sur l'appareil | L+M | L |
| **MB9 Notifications réelles** | avec 8 N7 : inscription liée à l'appareil, FCM (Android), APNs (iOS **si compte Apple**), aperçu C1, canaux, pastille, retenue pendant les cours, plage calme | MO6 | MB4 ; 8 N7 ; compte Apple (§13) | bacs à sable FCM (et APNs) ; aucune donnée C2+ dans une charge ; révocation → plus rien ; élève en cours → retenue | L (Android) + M + A | M |
| **MB10 Élève** | MQ-221 à MQ-238 ; carte numérique ; dépôt de devoir ; majorité | — | MB8 ; 20 C11 ; 23 B12 ; 14 AS5 (carte) | parcours du soir (« pour demain ») ; élève majeur justifie lui-même ; à la majorité, l'appli des parents perd ce qui n'est plus permis | L+M | M |
| **MB11 Vie scolaire, direction, personnels** | MQ-122, MQ-123, MQ-131 à MQ-150, MQ-164 à MQ-171 (12 AS14b, 13 D11, 15 A12, 29, 47) | — | MB6 ; 13 D11 ; 15 A12 | file du matin, exclusion de cours de bout en bout (professeur → CPE), tableau du jour de la direction, absence du personnel → remplacement accepté | L+M | L |
| **MB12 Appareils partagés et accueil** | postes partagés (2228), session par-dessus, téléphone de service, accueil (MQ-151, MQ-160 à MQ-163), contrôle de sortie hors ligne, comptage d'alerte | MO9 | MB7 ; 14 AS5, AS10 ; 4 A13 | tablette d'accueil : deux agents successifs sans fuite ; interdiction posée → visible < 1 min ; exercice d'alerte réseau coupé | L+M + A | M |
| **MB13 Accessibilité et langues** | audit RAAM des parcours de la démo, corrections, état de conformité, langues des familles, droite-gauche, mode extérieur | MO10 | MB8, MB10 | TalkBack et VoiceOver joués sur appel, justification, travail ; plus grande taille de texte sans perte | A | M |
| **MB14 Perte, versions, diagnostic** | incidents d'appareil (2229), inventaire et compte rendu DPO ; versions (2223), `426`, blocage à deux personnes ; SBOM et licences en CI ; diagnostic sans donnée (2230) | MO12, MO13, MO14 | MB7 | téléphone de labo déclaré perdu → inventaire juste, purge confirmée au retour du réseau ; version bloquée → écran de mise à jour ; diagnostic relu par test | L+M | M |
| **MB15 Publication** | comptes d'organisation (Florian), fiches, étiquettes de confidentialité, comptes de revue, pistes pilote (TestFlight, test interne Play), fermeture de compte | MO11 | MB3 à MB14 ; comptes | appli acceptée sur les deux magasins en piste pilote | M + A | M |
| **MB16 Verticaux secondaires** | au fil des lots des socles : rendez-vous (19 AG8), visio (33 VI9), paiements simples (25 P8), restauration (40 R10), CDI (45 BIB9), sorties (42 V6), santé (34 S11), compétences (22 C10), discipline famille (13 D11), internat famille (41) | — | MB7, MB8 et le lot du socle | chacun : parité verte, hors ligne s'il est déclaré, recette par personas | L+M | — |

**Ordre et parallélisme** : MB0 → MB1 → MB2 ; MB3 et la préparation de MB4 en parallèle ; MB5 ; puis **deux voies** : *personnel* (MB6 → MB7 → MB11 → MB12) et *familles et élèves* (MB8 → MB10), MB9 dès que le socle 8 livre N7 ; MB13 et MB14 avant la démo ; MB15 quand les comptes existent.

**Démo du printemps 2027 (D-068)** [H sur les dates, l'effort dépend des lots des autres socles] :
| Période | Lots | Ce qu'on peut montrer |
|---|---|---|
| décembre 2026 – janvier 2027 | MB0, MB1, MB2 | appli unique installée sur Android et iPhone (hors magasin), CI verte, contrat vérifié |
| février 2027 | MB3, MB4, MB5 | rejoindre par QR, passkey native, deux casquettes, téléphone perdu révoqué |
| mars 2027 | MB6, MB7 | journée du professeur, appel **en mode avion** au gymnase, appel de nuit à l'internat |
| avril 2027 | MB8, MB9 (Android ; iOS si compte), MB10 | famille : absence → avis → justification ; notes publiées ; travail ; élève le soir |
| mai 2027 | MB11, MB12, MB13, MB14 | vie scolaire, direction, accueil, accessibilité, perte d'appareil avec inventaire |
| juin 2027 | MB15 si comptes ; MB16 selon les socles | pilote en magasins |

**Critère de démo complète** : chaque rôle du §3 joue au moins son parcours principal de bout en bout sur les deux plateformes contre le labo ; banc de parité vert ; aucun appel à une route historique ; hors ligne démontré en mode avion ; perte d'appareil démontrée ; TalkBack et VoiceOver joués sur trois parcours.

**Installation sans magasin pour la démo** : Android par APK signé (aucun compte Google requis) ; iOS par Xcode sur les iPhone de démo avec l'équipe de développement de Florian (sans compte payant, profil de 7 jours à renouveler [H] ; avec le compte payant, profil d'un an et TestFlight).

---

## 10. Linux, Mac, appareils : comment avancer sans bloquer

| Travail | Conteneur Linux | Mac | Appareil réel |
|---|---|---|---|
| Serveur (routes, migrations, banc de parité, fixtures) | **tout** | — | — |
| Cœur Android (`:core`, Kotlin JVM) | **tout** (JDK, Gradle présents) | — | — |
| Interface Android (Compose) | build debug et release, tests Robolectric, lint, APK [H : SDK Android à installer dans l'environnement, hôtes Maven/Google à autoriser] | — | passkeys, biométrie, QR, notifications, mode avion |
| Émulateur Android | non (pas de virtualisation matérielle attendue dans le conteneur) [H] | possible | — |
| Cœur iOS (`ScolIACore`, SwiftPM) | `swift test` [H : chaîne Swift pour Linux à installer] | oui | — |
| Interface iOS (SwiftUI), signature, simulateur, XCUITest | **non** | **obligatoire** | passkeys, Face ID, notifications |

Méthode [R] :
1. **Cœurs miroirs** : toute la logique non visuelle (contexte, manifeste, baux, chiffrement par interface, file d'opérations, rattachement, erreurs) vit dans les cœurs, mêmes noms, **mêmes cas de test** des deux côtés, tous exécutés sous Linux.
2. **Écrans en miroir** : chaque écran Compose a son jumeau SwiftUI écrit dans le même lot, avec la même structure ; le code SwiftUI est relu en revue mais compilé en **passe Mac**.
3. **Passe Mac par lot** (Florian dispose de Macs, D-055) : build, simulateur, XCUITest du parcours du lot, essai sur iPhone ; défauts corrigés dans le lot suivant au plus tard ; un écart iOS ne bloque **jamais** la fusion d'un lot Android et serveur, il est noté « iOS à passer ».
4. **Si la passe Mac bloque deux lots de suite** : session distante sur le Mac de Florian (dossier partagé par Remote Control) ou runner macOS loué pour la seule CI iOS ; jamais de dépendance permanente.
5. **Appareils de test** : un Android ancien (Android 9) et un récent ; un iPhone récent et un iPhone SE ; une tablette. Jeux d'essais consignés (`11_MOBILE.md` §28.2).

---

## 11. Paramètres d'établissement (D-069) — ajoutés

S-1101 à S-1108 sont dans `11_MOBILE.md` §29.2.
| ID | Paramètre | Défaut prudent | Bornes |
|---|---|---|---|
| S-1109 | Délai de grâce de la présence pour les gestes ★ | familles et élèves **0 s** ; personnels 60 s pendant un appel ouvert | 0–300 s |
| S-1110 | Repli sans passkey possible | web + code à usage unique, durée web, aucun hors ligne | ne peut pas ouvrir de session longue |
| S-1111 | Mode « poste de classe » (iPad élèves) | désactivé | session ≤ durée de la séance |
| S-1112 | Attestation d'appli (signal) | désactivée | jamais bloquante |
| S-1113 | Biométrie forte exigée pour les gestes ★ des familles (refuse le seul code de l'appareil) | désactivée (code accepté), avec avis de chaque geste ★ | — |
| S-1114 | Filigrane du nom de l'utilisateur sur les listes d'élèves | activé pour les personnels | — |
| S-1115 | Exclusion de cours depuis le mobile | activée, avec accompagnant obligatoire | — |
| S-1116 | Saisie de notes sur mobile | activée, publication ★ | — |
| S-1117 | Écriture d'un nouveau message par les familles depuis le mobile | activée selon la matrice du socle 16 | jamais hors matrice |
| S-1118 | Appel d'activité hors ligne (étude, cantine) | désactivé | — |
| S-1119 | Diffusion d'urgence depuis le mobile à deux personnes | activée (deux personnes) | — |
| S-1120 | Correction de ses coordonnées par la famille | soumise à validation du secrétariat | — |

---

## 12. Règles ajoutées (R-MO47 et suivantes)

- **R-MO47** — L'appli n'affiche que les écrans du manifeste ; aucun calcul de droit dans l'appli ; le manifeste est recalculé à chaque contexte et après tout 403.
- **R-MO48** — Tout geste ★ exige une signature de la clé d'appareil attestant la présence du titulaire, dans le délai S-1109 ; sinon `403 presence.requise`. Chaque geste ★ d'une famille est notifié à ses autres appareils.
- **R-MO49** — Aucune donnée métier stockée sur l'appareil hors bail ; documents ouverts en mémoire ; « enregistrer » = exportation explicite journalisée.
- **R-MO50** — Conflit de synchronisation : appliqué, appliqué avec mention ou refusé avec code, affiché ligne par ligne ; jamais de fusion silencieuse.
- **R-MO51** — Pas de route réservée au mobile hors `/mobile/*` et `.well-known` sans justification écrite dans la conception du socle.
- **R-MO52** — Aucun nouvel écran sur une route historique ; retrait d'une route historique après la dernière version d'appli qui l'appelle + N-2.
- **R-MO53** — Parité : pour toute action servie au mobile, même statut, même code, même contenu qu'au web ; écarts permis limités à la coquille ; banc de parité obligatoire pour tout écran déclaré.
- **R-MO54** — Toute écriture mobile porte un `Idempotency-Key` ; même clé et même corps rejouent la même réponse pendant 24 h.
- **R-MO55** — Toute route servie au mobile renvoie l'enveloppe d'erreur codée ; 404 neutre pour tout objet hors droits ; aucune cause de refus révélée sur la personne.
- **R-MO56** — Charge de notification limitée à `{avis, etab, cat}`.
- **R-MO57** — Aucune fonction élève n'exige le téléphone pendant le temps scolaire.
- **R-MO58** — Un écart iOS ne bloque pas la fusion d'un lot serveur et Android ; il est inscrit « iOS à passer » et levé au plus tard au lot suivant.

---

## 13. Question ouverte (contredit un choix déjà fait)

**Q-MB1 — Compte développeur Apple.** D-055 : « comptes Apple et Google plus tard (coût) ». Or D-003 rend les applis obligatoires au MVP, et sur iOS **les notifications (APNs), TestFlight et la publication exigent le programme Apple payant** [H : règle connue des magasins, à revérifier au lot MB9]. Sans ce compte : démo iOS installée par Xcode, **sans notifications réelles**, profils de courte durée. Le compte Google n'est pas nécessaire avant la publication (APK signé, FCM accessible avec un compte Google gratuit [H]).
**Recommandation** : ouvrir le compte Apple **d'organisation** avant mars 2027 (délai d'obtention du numéro D-U-N-S et de la revue) pour que MB9 couvre iOS à la démo d'avril ; le compte Google Play avant MB15. En attendant, le plan avance sans blocage (MB9 sur Android seulement).

Tout le reste est tranché : §2.3 et §11.

---

## 14. Faits, hypothèses, contradictions

**Faits** (vérifiés le 2026-11-22) : §1.1.
**Hypothèses** : noms exacts des futures routes des socles (fixés par leurs lots) ; installation du SDK Android et de Swift pour Linux dans l'environnement ; absence d'émulateur Android dans le conteneur ; règles des magasins (APNs et TestFlight réservés au programme payant, profils de 7 jours sans compte payant) ; efforts et dates du §9.
**Contradictions relevées** :
1. `11_MOBILE.md` A-18 (« DTO générés depuis `ent-types` ») contredit D-062 (postérieure) : remplacé ici par les fixtures dorées (§5.1 point 4).
2. `11_MOBILE.md` §27.1 vise une démo de l'appel en décembre (D-034) ; D-068 fixe une démo complète au printemps : ordonnancement remplacé par le §9.
3. `11_MOBILE.md` §8.1 garde OkHttp ; D-062 le fait retirer (MB0).
4. `MOBILE.md` (`ent-mobile-app`) range « signature du carnet, pièces jointes, internat, cantine, réinscription » hors mobile ; les conceptions 7, 16, 18, 3, 40, 41 les prévoient au mobile : le §3 suit les conceptions des socles.
5. Les durées de journal du §8.3 sont des propositions ; le DPO n'est pas une condition (D-064), le socle 27 les porte en paramètres.

**Prochaines étapes** : versement de ce document sur `ent-mc0hoe` par le pilote ; lancement de **MB0** (sécurité, peut partir tout de suite, Linux seul) ; mise à jour de `MIGRATIONS.md` inutile (plage 2220–2239 inchangée).
