# Socle 32 — Espaces de travail et bureautique

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code Rust du domaine. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Le code Next.js a été **lu seulement**, de façon ciblée (module « Espace documents » M19 et « Office »), sans rien exécuter ni construire.
**Branche :** `ent-socle-32-33-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_32.md` (même branche) ; ses ajouts sont versés au **§30** de ce document, marqués **« ajout personas »**. **En cas de contradiction, le §30 l'emporte** ; rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (**D-005** conversion maison, **D-006** ArtCraft, D-017 droits, D-021, D-023 peu de réglages, D-026 briques, D-031, **D-034** doublon, **D-035** reprise de l'add-on, **D-036** pas de JSON-mémoire, D-038 exigences E1–E14) ; `docs/PRA_SAUVEGARDES.md` §18.4 ; `docs/TRANSITION_T0_INVENTAIRE.md` (M19) ; `docs/MATRICE_CHARLEMAGNE.md` (DOC-09, DOC-10, COM-17, T-22) ; conceptions `07_DOCUMENTS.md` (noyau des fichiers, analyse, conversion, §5.1 « socle 32 »), `16_MESSAGERIE.md` (CM-28 : pas de messagerie entre élèves, « espaces de classe modérés »), `18_FORMULAIRES.md` (usage `production_publiee`), `20_CAHIER_DE_TEXTES.md` (§2 frontières : remises et dépôts au 20, documents de l'espace au 32), `33_VISIO.md` (tableau partagé d'une classe virtuelle), `IA_COEUR.md` (§5 contrat, §6.2 niveaux N0–N3, §7.2 sensibilités C0–C4). Modèles de forme : `42_SORTIES_VOYAGES.md`, `10_EMPLOI_DU_TEMPS.md`.

**Conventions.** **[F]** fait vérifié dans le code (Next.js : chemins relatifs à la racine du dépôt ; Rust : depuis `ent-rust/`) ; **[V]** règle lue sur une source officielle (URL et date au §5) ; **[S]** fait lu sur une source publique non officielle (dépôt, documentation d'éditeur) ; **[NV]** non vérifié ; **[H]** hypothèse ; **[R]** recommandation, non décidée.

**Numérotation propre au socle :**

| Élément | Numéros |
|---|---|
| Actions existantes de Next.js | **NX32-01…** |
| Défauts de Next.js | **ND32-01…** |
| Actions cibles | **ETR-100 à ETR-299** (ETR = espaces de travail) |
| Règles | **R-ET1…** |
| Cas limites | **CET-01…** |
| Questions | **Q-ET1…** |
| Réglages signalés | **S-3201…** |
| Manques (passe personas) | **M-3201…** |

**Plage de migrations imposée : 2640–2659** (vérifiée libre sur les 134 branches distantes le 2026-11-11 : aucun fichier `264x`–`265x`, aucune mention dans un `MIGRATIONS.md`).

---

## 0. En bref

1. **Ce qui existe.** Côté Rust : **rien** pour les espaces ni la bureautique [F] ; mais le **noyau des fichiers** est là : registre immuable `ent_objet` (migration 500, empreinte SHA-256, `supprime_le`, RLS), dépôt contrôlé `ent-object/src/upload.rs` (liste blanche par usage, refus des macros et des fichiers actifs, plafond 64 Mio), liens pré-signés courts (`presign.rs` : 60 s par défaut, 900 s au plus). Côté Next.js : un **espace documents des personnels** réellement utilisé [H] (M19, classé « Partiel », « à conserver : ce sont les fichiers des utilisateurs ») et un module « Office » **vide** (création de fichiers ODF vierges, téléchargement, aucun éditeur, masqué faute de serveur bureautique) [F].
2. **Ce que fait Next.js et qu'on garde** (NX32-01 à NX32-22, §2.2) : dossiers personnels et dossiers partagés entre collègues, partage d'un fichier en lecture, dépôt par glisser-déposer d'une arborescence, déplacement, corbeille par confirmation, quota de 2 Gio par personne, invitation par avis, récents bureautiques.
3. **Ce qui est fragile** (ND32-01 à ND32-20, §2.3, **signalés à Florian ; Next.js reste intouchable**) : le **quota et la taille maximale se contournent** (taille déclarée par le navigateur, URL d'envoi sans taille signée) ; un membre d'un partage **écrase en silence** les fichiers du propriétaire ; les **membres d'un partage ne sont pas vérifiés** côté serveur ; aucun contrôle de type ni antivirus ; liens de téléchargement valables **une heure** et non révocables ; métadonnées en **JSON-mémoire** (collections `documents__shares`, D-036) ; partage de fichier par **pointeur de chemin** qui casse au déplacement.
4. **La thèse (§1).** Un espace de travail n'est pas un disque partagé : c'est **un lieu de travail daté, rattaché à un collectif réel** (une classe, un groupe, un cours, une équipe, une instance, un projet), dont **les membres se déduisent de la structure** (socles 5, 6) et changent seuls quand elle change. Dedans, des **éléments vivants versionnés** (fichiers, documents bureautiques, liens, pages, tableaux de bord de projet) ; à côté, l'**espace personnel** de chacun, le **casier** (ce qu'un adulte remet individuellement à un élève), et le **portfolio** de l'élève (ce qu'il choisit de garder et de montrer de son parcours).
5. **La bureautique (D-006, §11).** Suite **ArtCraft** figée — Wordcraft **v0.17.0**, GridCraft **v0.18.0**, DeckCraft **v0.17.0** (2026-11-09) [S] — et non plus « pré-alpha v0.4 » comme l'écrit D-006 : **mise à jour de D-006 proposée** (Q-ET1). Licence **MIT OU Apache-2.0**, marques et logos exclus (clause du README : les versions modifiées doivent les retirer) [S] ; builds WebAssembly publiés ; intégration par le protocole **`craft-web-host`** en iframe **de même origine** (`postMessage` : `open`, `save`, `dirty`, `saved`) [S]. **Pas de coédition en temps réel** chez ArtCraft [S] : ScolIA pose un **verrou d'édition** (un rédacteur à la fois, les autres lisent la dernière version, « demander la main ») et des **versions** ; la coédition reste une question (Q-ET2).
6. **La conversion (D-005, §12).** Moteur maison `ent-convert`, processus isolé sans réseau, avec limites de temps et de mémoire ; **le moteur ArtCraft côté serveur** (crates Rust de Wordcraft, GridCraft, DeckCraft, MIT/Apache) pour les aperçus PDF des formats OOXML ; **LibreOffice headless** (MPL-2.0, exécuté comme programme séparé non modifié) en repli pour ODF et formats anciens ; `image`, `resvg`, `pdfium-render`, `lopdf`, `typst` ; **jamais de code convt** (AGPL-3.0 confirmée) ; **pandoc exclu** (GPL) ; FFmpeg en build **LGPL** seulement, s'il sert un jour.
7. **Élèves et collectif (§9).** Pas de messagerie entre élèves (socle 16, CM-28) : les espaces de classe sont le **seul lieu d'échange entre élèves**, **animés par un adulte** : contributions nominatives, commentaires sous l'élément, modération, signalement, gel, plage calme, aucune conversation privée entre élèves, aucun espace d'élèves sans adulte.
8. **Droit d'auteur (§5.3).** L'exception pédagogique est désormais au **12° de l'article L122-5 et à l'article L122-5-4 du CPI** (ordonnance 2021-1518) [V] : elle vaut dans un « **environnement numérique sécurisé** » réservé aux élèves et personnels — ScolIA en est un **s'il n'a aucun lien public** ; rappel au dépôt, pas de publication hors de l'établissement (R-ET9).
9. **Volume.** **145 actions** numérotées dans la plage ETR-100 à ETR-299 (113 de la conception, 32 ajoutées par la passe personas ; numéros laissés libres entre les familles pour les ajouts), **72 cas limites**, **44 règles**, **15 lots** (ET0 à ET14). **Pas de lot 0 de correctifs Rust** (aucun code du domaine) : le **lot ET0** est le **préalable sur le noyau** (usage de dépôt « espace », taille réelle signée, quotas atomiques, versions d'objets) et l'**inventaire de reprise** de M19.
10. **Décisions appliquées par défaut (D-031)** : §28.1. **Soumis à Florian** (§28.2) : mise à jour de D-006 (versions, protocole d'intégration) ; coédition temps réel (absente d'ArtCraft) ; accessibilité d'un éditeur rendu en canvas (RGAA) ; usage réel de M19 et date de bascule ; quotas (coût du stockage) ; reprise des fichiers réels (accord de l'établissement, DPO).

---

## 1. Thèse : un espace est un lieu de travail daté, rattaché à un collectif réel

Sept principes [R] :

1. **Un espace a un ancrage.** Il est rattaché à une **entité de structure** (classe, groupe, cours du socle 5 ; équipe, service, instance du socle 6 ou 46 ; projet déclaré) **pour une année**. Ses membres sont **calculés** à partir de l'ancrage (élèves du groupe à la date, professeurs du cours, membres de l'équipe), plus des **membres ajoutés à la main** nominativement et datés. Quand l'élève change de groupe, il **sort** de l'espace du groupe à la date (ses contributions restent, attribuées), sans geste du professeur. Il n'y a jamais de liste de membres recopiée (défaut ND32-03 corrigé).
2. **Trois espaces par personne, au plus utiles.** L'**espace personnel** (tout le monde, quota), les **espaces collectifs** auxquels on appartient, le **casier** (élève : ce qui lui est remis ; adulte : ce qu'il a remis). Rien d'autre par défaut (D-023).
3. **Un élément est vivant, une version est figée.** Un élément (fichier, document bureautique, lien, page, note) a des **versions immuables** : chaque enregistrement crée un nouvel objet `ent_objet` (E5) ; l'élément pointe sa version courante ; on revient à une version antérieure en **créant** une nouvelle version. Rien n'est écrasé (défaut ND32-02 corrigé).
4. **Le droit vient de l'espace, pas du fichier.** Rôles d'espace fixes : **animateur** (adulte responsable), **contributeur**, **lecteur** ; un dossier peut **restreindre** (lecture seule pour les élèves, dépôt sans lecture des autres), jamais élargir au-delà de l'espace. Le partage hors espace n'existe que **nominatif, entre personnes de l'établissement, et daté** ; aucun lien public (R-ET9).
5. **L'espace n'est pas le dossier administratif.** Ce qui doit être **émis, signé, gardé** (attestation, convention, compte rendu officiel, copie notée) relève du **socle 7** ; un élément d'espace devient document du socle 7 quand on l'**émet** (geste explicite, version figée, ETR-190). Ce qui est **à rendre et à évaluer** relève du **socle 20** (remises) et du 21 (notes) ; l'espace fournit le document de travail, le 20 le fige au dépôt.
6. **L'élève est protégé du collectif.** Entre élèves : pas d'échange privé, pas d'espace sans adulte, contributions nominatives modérées ; l'élève mineur ne voit que ce qui lui est destiné ; ses parents **ne lisent pas** ses espaces de travail (comme la messagerie, socle 16), mais voient ce que l'élève ou un professeur leur **montre** (portfolio, casier « à montrer à la famille »).
7. **Tout reste chez nous et se rend à la sortie.** Fichiers, conversion, aperçus, bureautique, recherche plein texte : sur nos serveurs (D-005). À la sortie d'une personne, son espace personnel et son portfolio **s'exportent** (portabilité, `PORTABILITE_EXPORTS.md`), puis s'effacent selon une règle validée par le DPO.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]

| Élément | Où | Ce qui sert au socle 32 |
|---|---|---|
| Registre d'objets immuables | `migrations/500_objet_registre.sql` | une ligne par objet (`cle_objet` préfixée par l'établissement, `taille`, `sha256`, `mime`, `cree_le`, `supprime_le`), trigger d'immuabilité, pas de `DELETE`, RLS → **chaque version d'élément est un `ent_objet`** |
| Dépôt contrôlé | `crates/ent-object/src/upload.rs` | type réel par signature, liste blanche par usage (`UsageFichier` : `Document`, `PieceMessage`, `PiecePai`, `Photo`, `PieceCahier`, `Copie`), refus des macros, ActiveX et fichiers actifs (HTML, SVG, scripts), lecture bornée des ZIP (20 000 entrées) sans décompression, nom assaini (200 octets), plafond **64 Mio** |
| Liens pré-signés | `crates/ent-object/src/presign.rs` | jeton signé (destinataire, établissement, expiration, disposition), **60 s par défaut, 900 s au plus** |
| Fournisseur S3 | `crates/ent-object/src/s3.rs`, `provider.rs` | écriture S3 signée (lot `ent-stockage-objets-s3`) |
| Pièces vers objets | `migrations/601_pieces_vers_objets.sql` | modèle de sortie des `BYTEA` (D-040) |
| Bureautique, espaces, partages, casier, portfolio | — | **absents** |

Il manque au noyau, pour le socle 32 : un **usage de dépôt « espace »** (quotas et tailles propres, formats bureautiques étendus), le **contrôle de la taille réelle** lors des envois directs vers S3 (taille signée dans l'URL d'envoi), la **réservation atomique de quota**, et l'**état d'analyse** (quarantaine, antivirus) prévu par le socle 7 (`ent_fichier`, `07_DOCUMENTS.md` §8.1). C'est le lot ET0.

### 2.2 Ce que fait Next.js : actions existantes, toutes reprises [F]

Lecture ciblée de `app/api/documents/**`, `app/lib/documents-cloud.ts`, `documents-notify.ts`, `documents-page-model.ts`, `office-*.ts`, `app/(admin)/documents/**`, `app/components/documents/**`.

| N° | Action | Source | Reprise |
|---|---|---|---|
| NX32-01 | Parcourir son espace personnel par dossiers | `documents-cloud.ts:413-511` | ETR-110 |
| NX32-02 | Voir un dossier virtuel « Fichiers partagés » (fichiers reçus) à la racine | `:47, :489-501` | ETR-131 |
| NX32-03 | Créer un dossier | `:513-533` | ETR-112 |
| NX32-04 | Téléverser par URL d'envoi directe vers le stockage | `upload/route.ts:29-70`, `:574-608` | ETR-114 (taille signée) |
| NX32-05 | Déposer une arborescence par glisser-déposer (chemins relatifs conservés) | `upload/route.ts:19-26`, `documents-page-model.ts:281` | ETR-115 |
| NX32-06 | Télécharger un fichier (nom forcé) | `get-url/route.ts:14-71` | ETR-118 |
| NX32-07 | Déplacer fichier ou dossier dans le même espace ; refus si la destination existe, ou d'un dossier dans lui-même | `:875-960` | ETR-120 |
| NX32-08 | Supprimer un fichier ou un dossier | `:1023-1070` | ETR-122 (corbeille) |
| NX32-09 | Supprimer un dossier entier avec confirmation tapée | `:986-1021` | ETR-123 |
| NX32-10 | Créer un dossier partagé nommé avec des collègues | `:661-685` | ETR-150 |
| NX32-11 | Modifier les membres d'un partage (propriétaire) | `:687-720` | ETR-153 |
| NX32-12 | Lister les partages dont on est propriétaire ou membre | `:160-172` | ETR-111 |
| NX32-13 | Quitter un dossier partagé (membre) | `:962-984` | ETR-155 |
| NX32-14 | Partager un fichier unitaire en lecture | `:305-351` | ETR-130 |
| NX32-15 | Modifier les destinataires d'un fichier partagé | `:353-365` | ETR-130 |
| NX32-16 | Quitter un fichier partagé | `:367-382` | ETR-132 |
| NX32-17 | Choisir des collègues dans l'annuaire de l'établissement | `:763-766` | ETR-151 |
| NX32-18 | Voir son quota (2 Gio) | `quota/route.ts`, `:20` | ETR-260 |
| NX32-19 | Badge « invitation non vue » sur le tableau de bord, levé à la première ouverture | `:723-761` | ETR-152 (avis socle 8, tâche du 29) |
| NX32-20 | Avis par e-mail aux nouveaux membres d'un dossier partagé | `shares/route.ts:11-35`, `documents-notify.ts` | ETR-152 |
| NX32-21 | Créer un document vierge texte / tableur / présentation (nom dédoublonné « Sans titre (2) ») | `office/new/route.ts`, `office-create.ts:23-99` | ETR-170 |
| NX32-22 | Récents bureautiques (10 personnels, 10 partagés) et recherche par nom | `office/recents/route.ts:25-197` | ETR-117, ETR-176 |

Règles reprises : quota par personne **2 Gio** [F `:20`] ; un dépôt dans un partage est compté **au propriétaire** [F `:555-560`] (repris en « compté à l'espace », R-ET20) ; taille **50 Mo** par fichier [F `documents-page-model.ts:62-63`] ; suppression dans un partage réservée au propriétaire [F `:1033-1041`] ; téléchargement journalisé [F `get-url/route.ts:57-65`].

### 2.3 Défauts trouvés dans Next.js (à ne pas reproduire ; signalés à Florian, Next.js intouchable)

| N° | Défaut | Source | Gravité | Correctif côté ScolIA |
|---|---|---|---|---|
| ND32-01 | Taille et quota **déclarés par le navigateur** ; URL d'envoi sans taille signée : on dépasse 50 Mo et 2 Gio | `upload/route.ts:40`, `documents-cloud.ts:598-606` | **critique** | R-ET21 (taille signée, réservation atomique, contrôle après envoi) |
| ND32-02 | Un **membre écrase** sans avertissement un fichier du propriétaire (qu'il ne peut pas supprimer) | `:535-571` | **critique** | versions immuables (R-ET3) |
| ND32-03 | **Membres de partage non vérifiés** côté serveur (identifiants arbitraires) | `shares/route.ts:57`, `:325, :361, :668` | **critique** | R-ET12 (membres résolus par le cœur, même établissement) |
| ND32-04 | Contrôle d'accès des routes par la **session seule** ; rôle vérifié dans le seul intermédiaire | `intranet-auth.ts:41-66`, `proxy.ts:596-658` | élevée | droits D-017 vérifiés dans chaque commande |
| ND32-05 | **Aucune liste blanche** ni antivirus ; type fourni par le client | `upload/route.ts` | élevée | noyau du socle 7 (quarantaine, analyse, désarmement) |
| ND32-06 | Liens de téléchargement et d'envoi valables **1 h**, transmissibles, non révocables | `:605`, `get-url/route.ts:52` | élevée | jetons de 60 s liés à la session (`presign.rs`) |
| ND32-07 | Métadonnées de partage en **JSON-mémoire** (`documents__shares`, `documents__file-shares`, `office-recents`) | `ent-json-postgres.ts:13-47, 532-594` | élevée (D-036) | tables normalisées (§24) |
| ND32-08 | Partage de fichier par **pointeur de chemin** : un déplacement ou une suppression casse le partage sans nettoyage | `:384-399, :393` | moyenne | partage sur l'identifiant de l'élément (R-ET13) |
| ND32-09 | Déplacement et suppression **non atomiques** (copie puis suppression objet par objet) | `:948-957, :1065-1067` | moyenne | arborescence en base, objets jamais déplacés (R-ET4) |
| ND32-10 | Contrôle de chemin naïf (`includes("..")`), nom de fichier non contrôlé à l'envoi | `:117, :546, :641` | moyenne | clés opaques `{etab}/espaces/{uuid}` (E14) |
| ND32-11 | Écriture lors d'une lecture (`memberSeenAt` mis à jour au parcours) | `:428-434` | faible | état « vu » par commande |
| ND32-12 | Avis envoyés en **tâche détachée** (perdus au recyclage du conteneur) ; rien sans SMTP de l'établissement | `shares/route.ts:18-34` | moyenne | outbox dans la transaction (socle 8) |
| ND32-13 | Erreur technique renvoyée telle quelle au client | `get-url/route.ts:69` | faible | erreurs typées |
| ND32-14 | Envoi multipart **chargé entièrement en mémoire** (50 Mo par requête) | `upload/route.ts:96` | faible | envoi direct borné |
| ND32-15 | Chargement de **tous les partages** de l'établissement puis filtrage ; inventaire complet du stockage **à chaque envoi** pour le quota ; course sur le quota | `:160-226, :276-303` | moyenne | compteurs en base, verrouillés |
| ND32-16 | Aucune limite de membres, de longueur de nom ni de profondeur | — | faible | R-ET25 |
| ND32-17 | Aucun rôle « lecture seule » dans un dossier partagé | `:181-188` | moyenne | rôles d'espace (R-ET5) |
| ND32-18 | Clés de stockage sans identifiant d'établissement (isolation par seau seulement) | `s3-path.ts`, `:63-69` | moyenne (PRA) | `cle_objet` préfixée (500) |
| ND32-19 | Bureautique **vide** : fichiers vierges et téléchargement, aucune édition ; code mort (`placeOfficeDocument`) | `office-create.ts:101-229`, `intranet-modules.ts:174-183` | — | §11 |
| ND32-20 | Récents mis à jour en lecture-modification-écriture concurrente ; nom dédoublonné non atomique | `office-recents.ts:34-54`, `office-create.ts:69` | faible | index unique, compteur |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

- Les **fichiers des personnels** eux-mêmes (perte = incident) et leur arborescence, y compris les chemins relatifs déposés.
- Les **partages en cours** (qui partage quoi avec qui) : repris comme espaces d'équipe ou partages nominatifs, **proposés** à leur propriétaire avant activation (§26).
- La **simplicité** : un clic pour partager un dossier avec trois collègues ; ne pas imposer d'ancrage structurel à un partage ad hoc (« espace libre entre personnels », §8.4).
- Le **badge** d'invitation non vue.

### 2.5 Verdict

M19 : **partiel, à reprendre** (usage réel probable, défauts critiques de quota, d'écrasement et de membres). « Office » : **à refaire de zéro** (il n'y a rien à reprendre hormis la création de fichiers vierges). Aucun code Rust à corriger : pas de lot 0 de correctifs, mais un **lot ET0 de préalables** sur le noyau.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Professeur** | Prépare ses cours, partage supports et corrigés, fait travailler des groupes, récupère des productions | Clé USB, « je n'ai pas reçu le document », recréer chaque année l'espace de chaque classe, renvoyer le même fichier à 32 élèves | Qu'un élève modifie ou supprime le support ; qu'un document circule hors de la classe (droit d'auteur) ; perdre son travail | Espace de chaque cours **créé seul** à la rentrée avec les élèves du groupe ; **reconduire** sa structure de dossiers de l'an passé ; distribuer un document **personnalisé** à chaque élève en un geste ; voir qui a ouvert |
| **Professeur principal** | Coordonne la classe, documents de la vie de classe | Faire circuler un tableau de suivi entre collègues par mail | Version qui diverge | Un **espace d'équipe pédagogique** de la classe, avec un tableur partagé et son historique |
| **Équipe disciplinaire, coordonnateur** | Progressions communes, sujets communs, banque d'exercices | Dossiers dispersés, départ d'un collègue qui emporte tout | Perte de la banque au départ d'un collègue | Espace d'**équipe pérenne** (au-delà de l'année) qui survit aux départs |
| **Direction** | Documents des instances, projet d'établissement, notes internes | Versions par mail, « quelle est la dernière ? » | Fuite d'un document de direction ; un document confidentiel visible d'une équipe | Espaces d'**instance** (conseil pédagogique, CA) avec version courante unique ; émission en document officiel (socle 7) d'un clic |
| **Secrétariat, gestion** | Modèles, tableaux de suivi | Fichiers personnels non retrouvés pendant une absence | Un tableau avec des données d'élèves partagé trop largement | Espace de **service** partagé, repris quand une collègue part |
| **CPE, vie scolaire** | Projets éducatifs, documents des délégués | Échanges avec les délégués par mail personnel | Échanges entre élèves non encadrés | Espace des **délégués** animé par le CPE (ce que la messagerie ne fait pas, socle 16) |
| **Professeur documentaliste** | Projets de recherche, ressources | Liens morts, ressources GAR hors de l'espace | Droit d'auteur | Liens GAR et ressources du CDI insérables dans l'espace (socles 9, 45) |
| **AESH** | Accompagne un ou plusieurs élèves | N'a pas accès aux documents adaptés | Voir trop (autres élèves) | Accès aux **seuls** espaces des cours de l'élève accompagné, en lecture, et au casier de l'élève pour y déposer des supports adaptés (droit du socle 35) |
| **Élève** | Consulte, travaille en groupe, rend, garde ses productions | Ne retrouve pas le document ; perd son travail ; ne peut pas travailler à plusieurs sans messagerie privée | Qu'un camarade efface ou se moque de son travail ; être exposé | Un seul endroit par matière ; **travail de groupe** dans un document partagé avec l'historique de qui a fait quoi ; **portfolio** qu'il garde après l'établissement |
| **Élève majeur, post-bac** | Idem, plus d'autonomie | — | — | Partages entre élèves plus ouverts (réglage borné, R-ET32) |
| **Élève en situation de handicap** | Utilise un lecteur d'écran, agrandissement | Éditeur inaccessible | Exclusion | Aperçu accessible, alternative téléchargeable, formats adaptés |
| **Famille (responsables)** | Suit le travail | Ne voit rien, ou tout | Ne pas savoir ; un parent séparé qui voit ce que l'autre ne voit pas | Voir ce que l'élève ou le professeur **montre** (portfolio, casier « à montrer ») ; même vue pour les deux parents |
| **Administrateur de l'établissement** | Quotas, paramètres, départs | Fichiers orphelins à chaque départ | Saturation du stockage ; effacement accidentel | Tableau des volumes ; **transfert** guidé au départ ; corbeille de 30 jours |
| **DPO** | Registre, conservation, droits | Fichiers personnels de durée inconnue | Données de santé ou sociales dans un tableur partagé | Règles de conservation par type d'espace, export à la demande, alerte de contenu sensible (déterministe, §15) |

---

## 4. L'année des espaces vue du terrain [H, à confirmer avec Florian]

| Moment | Ce qui se passe | Ce que fait ScolIA |
|---|---|---|
| Fin août – rentrée | Groupes et cours créés (socles 5, 10) | **Espaces de cours proposés** à chaque professeur (un par cours), reconduction de sa structure de N-1 proposée, élèves inscrits **à la date** d'ouverture choisie par le professeur |
| Septembre | Changements de groupes, arrivées | membres recalculés ; l'arrivant voit l'historique de l'espace (réglage par espace : « depuis la rentrée » ou « depuis son arrivée ») |
| Toute l'année | Supports, travaux de groupe, distributions | versions, verrous, casier, aperçus |
| Conseils de classe | Documents des équipes | espace d'équipe de la classe |
| Fermeture, continuité (socle 33) | Travail à distance | espaces de cours = point d'appui du plan de continuité |
| Juin | Fin d'année | **archivage** : espaces de l'année en lecture seule ; élèves invités à **verser au portfolio** ce qu'ils veulent garder |
| Juillet | Départs d'élèves et de personnels | export du portfolio et de l'espace personnel ; transfert des éléments d'équipe ; effacement programmé |
| Année de doublon 2027-2028 (D-034) | Charlemagne officiel ; M19 en usage réel (D-035) | **usage réel** dès la bascule de M19 (le socle 32 remplace un module de l'add-on, pas un module de Charlemagne) |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

### 5.1 Statut des sources
Lu sur Légifrance [V] ; documents éduscol partiellement en 403, lus par extrait de moteur de recherche [V-extrait] ; dépôts publics [S].

### 5.2 Données personnelles
- **Minimisation** et **limitation de conservation** (RGPD art. 5) : les espaces de classe de l'année sont archivés puis effacés selon une règle validée par le DPO (§21.3) [V, principe].
- **ENT** : traitement couvert par le cadre ENT du ministère (SDET ; arrêté ENT) [NV : version 2025 du SDET non lue, `education.gouv.fr/les-referentiels-450069` en 403] ; le SDET 2024 liste un service « Espace de stockage et de partage de fichiers » [V-extrait : `eduscol.education.gouv.fr/sites/default/files/document/sdetdocument-principalversion2024pdf-106014.pdf`].
- **Pas de solutions extra-européennes gratuites** : réponse ministérielle QE n° 971 (JO du 15/11/2022) : pas d'offres gratuites Office 365 ni Google Workspace dans les écoles (doctrine « cloud au centre », commande publique) [V : `questions.assemblee-nationale.fr/dyn/16/questions/QANR5L16QE971.pdf`] ; réponse QE n° 10342 (JO du 10/02/2026) : courrier aux recteurs du 28 février pour **proscrire** les suites collaboratives d'éditeurs non européens et privilégier les ENT [V : `assemblee-nationale.fr/dyn/17/questions/QANR5L17QE10342`]. **Argument commercial direct** pour une bureautique intégrée et hébergée en France.
- **Doctrine technique du numérique pour l'éducation** : n'exige pas SecNumCloud pour les communs ; le **privé sous contrat n'est pas concerné par son opposabilité** (FAQ DNE du 19/11/2025) [V : `education.gouv.fr/sites/default/files/document/Fiche%20Questions-R%C3%A9ponses…476241.pdf`]. Le service « Nuage » (Nextcloud) est réservé aux enseignants, intégré aux ENT courant 2026 [V, même source] : ScolIA ne s'y raccorde pas par défaut (Q-ET9).

### 5.3 Droit d'auteur [V]
- **Exception pédagogique** : CPI **art. L122-5, 12°** et **art. L122-5-4** (ordonnance n° 2021-1518 du 24/11/2021, en vigueur le 26/11/2021) ; le 3° e) ne vise plus que la recherche [V : `legifrance.gouv.fr/codes/article_lc/LEGIARTI000044363194`, `…/LEGIARTI000037388886/`]. Conditions : utilisation **sous la responsabilité de l'établissement**, dans ses locaux **ou au moyen d'un environnement numérique sécurisé accessible aux seuls élèves et personnels** ; rémunération forfaitaire ; **exception écartée pour les usages numériques quand des licences adéquates sont proposées** ; œuvres conçues à des fins pédagogiques et partitions exclues.
- **Accords sectoriels** (livres, musique imprimée, presse, arts visuels) : dernier texte trouvé, avenant au BO n° 7 du 13/02/2020 ; protocole de 2016 : extraits limités (ordre de 10 %) sur l'ENT, œuvres courtes et arts visuels en intégralité [V-extrait ; texte postérieur non trouvé, **à vérifier auprès du CFC**, Q-ET10].
- **CFC reprographie** (papier, pas le numérique) : protocole du 3/3/2023, circulaire du 30/9/2023 (BO n° 44 du 23/11/2023) [V].
- **Conséquences pour le logiciel** [R] : (a) **aucun lien public**, aucune page d'espace indexable (R-ET9) ; (b) rappel court au dépôt dans un espace d'élèves (« extraits seulement, sauf licence ») ; (c) liens GAR plutôt que copies des ressources sous licence (socle 9) ; (d) un document marqué « ressource sous licence » ne se télécharge pas hors de l'espace, ne se verse pas au portfolio, ne s'exporte pas à la sortie (R-ET10) ; (e) le logiciel ne juge pas la licence : il trace la déclaration de l'auteur du dépôt.

### 5.4 Image, voix, productions des élèves [V]
- **C. civ. art. 9** (vie privée) et **art. 372-1** (loi n° 2024-120 du 19/02/2024 : les parents protègent **en commun** le droit à l'image de l'enfant, en l'associant selon son âge) [V : `legifrance.gouv.fr/codes/article_lc/LEGIARTI000006426505`].
- Une **production d'élève** (texte, dessin, vidéo) est une œuvre de l'élève [H : CPI L111-1, auteur mineur] : sa **diffusion hors de l'espace de classe** (exposition, site, journal) exige l'accord vérifié au socle 18 (usage `production_publiee`, `18_FORMULAIRES.md` §5) ; une photo ou vidéo où l'élève apparaît exige l'usage `droit_image` correspondant. Dans l'espace de classe, aucune diffusion externe : pas d'accord requis pour le travail ordinaire [R].

### 5.5 Portfolio [V]
- **Folios** (ONISEP) **fermé fin décembre 2024**, remplacé par **Avenir(s)** (généralisation à partir de septembre 2025, raccordé à ÉduConnect, sans GAR) [V : `education.gouv.fr/plan-avenir-l-orientation-pour-que-l-avenir-soit-un-choix-450556`]. Le portfolio ScolIA **n'est pas** le parcours Avenir : il garde les productions ; les parcours éducatifs (Avenir, citoyen, santé, artistique) relèvent du socle 37 et du 22 ; un lien d'export vers Avenir(s) n'existe pas [NV] (Q-ET11).

### 5.6 Accessibilité [V-principe]
- Obligation d'accessibilité des services numériques des organismes chargés d'une mission de service public (loi n° 2005-102, art. 47 ; RGAA) [NV : version du RGAA en vigueur non relue] ; l'éditeur ArtCraft est rendu en **canvas** (egui, wgpu) [S], dont l'accessibilité aux lecteurs d'écran est **à vérifier** : ScolIA garantit une **alternative accessible** (aperçu HTML/PDF balisé, téléchargement, édition hors ligne puis dépôt) pour toute fonction d'édition (R-ET30, Q-ET3).

### 5.7 Ce qui reste à vérifier [NV]
SDET 2025 (exigences de stockage) ; accords sectoriels postérieurs à 2020 ; RGI et formats ouverts (ODF) recommandés aux administrations ; durée de conservation des productions d'élèves (DPO) ; statut des fichiers d'un agent public parti (propriété de l'administration ou de l'agent).

---

## 6. Branchements sur le cœur

### 6.1 Qui possède quoi

| Donnée | Propriétaire | Le 32 |
|---|---|---|
| Classes, groupes, cours, années | 5 | lit (ancrage, membres calculés) |
| Personnels, équipes, services, départs | 6 | lit (membres, transfert au départ) |
| Élèves, scolarité, sortie | 2 | lit (membres, export au départ) |
| Responsables, autorité parentale, interdictions | 4 | lit (vue famille du portfolio et du casier) |
| Octets, analyse, conversion de dépôt, émission, signature, coffre | 7 (noyau et brique `documents`) | consomme `deposer`, `analyser` ; appelle `emettre` |
| Avis | 8 | déclare des natures |
| Ressources GAR | 9 | insère des liens profonds |
| Séances | 10 | lit (ouverture d'un espace par séance, continuité) |
| Remises de travaux | 20 | **fournit** un élément figé au dépôt ; ne possède pas la remise |
| Notes | 21 | rien |
| Accords (production publiée, image) | 18 | vérifie `autorisations::verifier` |
| Visio, tableau partagé de la salle | 33 | fournit un élément d'espace comme tableau ou support de séance |
| BEP, AESH | 35 | lit (droit d'accès de l'AESH, sans contenu du plan) |
| Matériel prêté | 44 | rien (le prêt d'ordinateur est au 44) |
| CDI | 45 | insère des notices |
| Instances | 46 | ancrage d'espace d'instance |

### 6.2 Ports consommés
`structure::membres_a_date(entite, date)` (5) ; `personnels::equipe_membres(equipe, date)` (6) ; `eleve::scolarite_a_date` (2) ; `administratif::responsables_titulaires(eleve, date)` (4) ; `fichiers::deposer(usage, octets) -> fichier_id` puis événement `fichiers.analyse` (7 noyau) ; `documents::emettre(nature, version)` (7) ; `autorisations::verifier(sujets, objet, usage, date)` (18) ; `notifications::publier(nature, destinataires)` (8) ; `ia::registre` (cœur IA).

### 6.3 Ports exposés
- `espaces::element_fige(element_id, version) -> objet_id` : pour le 20 (dépôt d'un document de l'espace, figé), le 7 (émission), le 33 (support de séance).
- `espaces::espace_du_cours(cours_id, annee) -> espace_id` : pour le 20 (lien « documents du cours ») et le 33.
- `espaces::casier_remettre(eleves, element, consigne)` : pour le 20 (copie corrigée rendue individuellement), le 23 (document individuel), le 35 (support adapté).
- `portfolio::verser(eleve, element)` : pour le 20 (production retenue), le 22 (preuve d'une compétence), le 42 (carnet de voyage).

### 6.4 Répercussions automatiques (le cœur bouge, l'espace suit)

| Fait | Effet |
|---|---|
| Élève change de groupe ou de classe (5) | sort des espaces de l'ancien groupe à la date, entre dans les nouveaux ; contributions conservées et attribuées ; son casier suit |
| Élève quitte l'établissement (2) | perd l'accès aux espaces collectifs ; portfolio et espace personnel **exportables** (lien du coffre du socle 7, durée S-3203) puis effacés ; contributions aux espaces collectifs conservées, attribuées (« ancien élève ») jusqu'à l'archivage de l'espace |
| Élève devient majeur (2) | la vue « famille » du portfolio et du casier s'arrête sauf accord de l'élève (R-ET35) |
| Nouveau responsable, retrait d'autorité, interdiction (4) | vue famille recalculée à la date |
| Professeur remplacé (15) | le suppléant devient animateur des espaces des cours suppléés pendant la suppléance |
| Personnel quitte l'établissement (6) | parcours de **transfert** : éléments d'équipe à un repreneur désigné, espace personnel exportable puis effacé (S-3204), propriété d'espaces collectifs transférée à l'animateur restant ou à la direction |
| Cours fermé ou fusionné (5) | espace archivé en lecture seule ; proposition de fusion d'espaces |
| Fermeture de l'établissement, continuité (33) | espaces de cours mis en avant dans le portail (29), « séance à distance » liée |
| Accord `production_publiee` retiré (18) | publication hors espace masquée automatiquement |

---

## 7. Concepts et vocabulaire

| Terme | Définition [R] |
|---|---|
| **Espace** | Lieu de travail daté, d'un **type** (`personnel`, `cours`, `groupe`, `classe`, `equipe`, `service`, `instance`, `projet`, `libre_personnels`), avec un ancrage optionnel et une année (sauf `equipe` pérenne et `personnel`) |
| **Ancrage** | Entité de structure qui calcule les membres (cours, groupe, classe, équipe, instance) |
| **Membre** | Personne avec un **rôle d'espace** à une date : `animateur`, `contributeur`, `lecteur` ; origine `calculee` ou `ajoutee` |
| **Dossier** | Nœud d'arborescence de l'espace, avec une **règle de dossier** facultative : `lecture_seule_eleves`, `depot_sans_lecture` (chacun voit ses dépôts), `animateurs_seuls` |
| **Élément** | Objet de l'espace : `fichier`, `document` (bureautique éditable), `lien`, `page` (texte riche simple), `ressource_gar`, `notice_cdi` |
| **Version** | Contenu figé d'un élément (un `ent_objet`), numérotée, auteur, date, commentaire |
| **Verrou d'édition** | Droit exclusif et temporaire d'enregistrer une nouvelle version d'un document bureautique |
| **Partage nominatif** | Accès donné à une personne nommée de l'établissement sur un élément ou un dossier, en lecture (ou écriture entre personnels), daté |
| **Casier** | Pour un élève : les éléments **remis individuellement** par un adulte (document personnalisé, correction, support adapté) ; pour l'adulte : ses remises |
| **Distribution** | Remise d'un élément au casier de plusieurs élèves, **à l'identique** ou **personnalisée** (une version par élève, publipostage) |
| **Portfolio** | Sélection de productions **de l'élève**, sous son contrôle, conservée au-delà de l'année et exportable |
| **Contribution** | Élément, version ou commentaire produit par un élève dans un espace collectif |
| **Commentaire** | Texte court rattaché à un élément (pas de fil de discussion libre) |
| **Archivage** | Passage de l'espace en lecture seule, à la fin de l'année ou à la fermeture de l'ancrage |
| **Corbeille** | État `supprime` d'un élément pendant 30 jours (S-3205), restaurable, puis effacement de l'objet |

---

## 8. Les espaces

### 8.1 Création et cycle de vie
États : `propose` → `ouvert` → `archive` → `efface`. Un espace de cours est **proposé** à la rentrée (ETR-100), **ouvert** par l'animateur (date d'ouverture aux élèves choisie, défaut : premier jour de cours) ; **archivé** au 31 juillet de l'année (lecture seule pour les membres de l'année, S-3206) ; **effacé** après la durée de conservation (S-3207, DPO).

### 8.2 Types et règles par type [R]

| Type | Ancrage | Membres calculés | Animateurs | Élèves | Durée |
|---|---|---|---|---|---|
| `cours` | cours (5) | professeurs du cours + élèves du public du cours | professeurs du cours | contributeurs (règles de dossier) | année |
| `groupe` (projet de groupe d'élèves, travail en petits groupes) | sous-ensemble d'un espace de cours | choisis par l'animateur parmi les élèves de l'espace parent | animateurs du parent | contributeurs | année |
| `classe` (vie de classe) | classe | PP, équipe, élèves | PP | lecteurs (contributeurs sur dossiers ouverts) | année |
| `equipe` (pédagogique de classe, disciplinaire, de projet) | équipe (6) ou classe | personnels | coordonnateur | **aucun** | pérenne |
| `service` (secrétariat, vie scolaire…) | service (6) | personnels du service | responsable | aucun | pérenne |
| `instance` (CA, conseil pédagogique, CVL…) | instance (46) | membres de l'instance à la date | président / secrétaire de séance | élus élèves en lecture ou contribution | mandat |
| `delegues` | délégués (46) | CPE + délégués | CPE | contributeurs | année |
| `projet` (EPI, atelier, sortie, échange) | déclaré (avec cours ou sortie 42 facultatif) | ajoutés | porteur | contributeurs | durée du projet |
| `libre_personnels` (reprise NX32-10) | aucun | ajoutés nominativement | créateur | **interdit** | pérenne, revue annuelle |
| `personnel` | la personne | elle seule | — | — | durée du lien |

### 8.3 Membres calculés et membres ajoutés
- Membres calculés **à la date** par les ports du 5 et du 6, jamais recopiés ; une **vue** `ent_espace_membre_effectif` joint calculés et ajoutés (R-ET11).
- Ajout manuel : nominatif, daté (début, fin facultative), rôle, motif court ; un élève n'est ajouté **qu'à un espace dont un animateur l'a en cours** à la date (R-ET12) ; un personnel d'un autre établissement **jamais** (D-013).
- AESH : accès en **lecteur** aux espaces des cours de l'élève accompagné et au casier de cet élève (dépôt de supports adaptés), si le droit `espaces.aesh` lui est donné et que le socle 35 atteste l'accompagnement à la date (sans contenu du plan) (R-ET14).
- Invité extérieur (intervenant, partenaire) : **pas de compte invité** en V1 ; partage par **lien nominatif à usage limité** d'un élément en lecture seule seulement, par un animateur, journalisé, 7 jours au plus (S-3208), jamais un espace avec des élèves (R-ET15).

### 8.4 Espaces libres entre personnels
Reprise des dossiers partagés de Next.js : un personnel crée un espace `libre_personnels` en un geste et ajoute des collègues. Sans élève, sans ancrage ; propriété transférable ; **revue annuelle** proposée à l'administrateur (espaces sans activité depuis 12 mois).

---

## 9. Travailler ensemble entre élèves (sans messagerie privée)

Le socle 16 ferme la messagerie libre entre élèves (CM-28) et renvoie le travail de groupe aux **espaces modérés**. Règles [R] :
1. **Aucun espace d'élèves sans adulte** : tout espace où écrit un élève a au moins un animateur adulte actif (R-ET16) ; à la sortie du dernier animateur, l'espace passe en lecture seule.
2. **Contributions nominatives** : chaque élément, version et commentaire porte son auteur, visible des membres ; aucune contribution anonyme.
3. **Commentaires** sous un élément, courts (1 000 caractères), sans pièce jointe, sans mention d'une personne extérieure à l'espace ; **pas de fil libre ni de message privé**.
4. **Modération** : l'animateur masque une contribution (motif codé), gèle un élève dans l'espace (lecture seule) ou ferme les commentaires ; l'élève masqué voit « masqué par l'animateur » ; un élève qui retire sa propre contribution est prévenu que l'animateur peut encore la voir **(ajout personas)** ; rien n'est supprimé définitivement pendant la vie de l'espace (preuve en cas de harcèlement, socle 13).
5. **Signalement** par tout membre (contenu blessant, données d'un camarade, contenu illicite) : file de l'animateur et du CPE ; rappels 3018 / 119 comme dans la messagerie (socle 16 M10) ; pas de modération automatique, une **alerte déterministe** sur liste de mots signalée à l'animateur sans masquer (R-ET17).
6. **Plage calme** : avis aux élèves seulement en journée (plage nationale du socle 16 : 20 h – 7 h, week-end) ; l'activité reste possible.
7. **Élèves majeurs, post-bac** : partage nominatif entre élèves permis par réglage borné de l'établissement (S-3209, défaut fermé) ; jamais vers des mineurs.

---

## 10. Éléments, versions, dossiers

1. **Dépôt** : par le noyau du socle 7 (`fichiers::deposer`, usage `espace`) : type réel, liste blanche (§12.1), quarantaine, antivirus, **désarmement** des PDF entrants pour les espaces avec élèves ; l'élément est visible des autres membres **après** l'analyse ; l'auteur le voit « en cours d'analyse ».
2. **Versions** : chaque envoi d'un fichier du même nom dans le même dossier **propose** « nouvelle version de X » ou « nouveau fichier X (2) » (jamais d'écrasement) ; versions listées, comparées pour les documents bureautiques (texte, ETR-182) ; retour à une version = nouvelle version ; **plafond de versions** conservées : toutes pendant 30 jours, puis une par jour, puis une par semaine au-delà de 3 mois, la courante toujours (S-3210) ; une version **figée** par un autre socle (remise du 20, émission du 7) n'est jamais élaguée (R-ET6).
3. **Déplacement, renommage** : en base seulement ; l'objet ne bouge jamais (R-ET4) ; entre deux espaces : **copie** (nouvel élément, version 1 = même objet, pas de recopie d'octets) ; un élément d'un espace d'élèves ne se déplace pas vers un espace personnel d'adulte sans copie explicite.
4. **Corbeille** 30 jours, restaurable par l'auteur ou l'animateur ; au-delà, effacement de l'objet (`supprime_le`, puis effacement physique par le ramasse-miettes du noyau, en respectant les sauvegardes, PRA §18).
5. **Aperçu** en ligne pour tout format courant (PDF, images, bureautique via conversion §12, texte, audio et vidéo courts lus par le navigateur), sans téléchargement ; le téléchargement reste possible sauf ressource sous licence (R-ET10) ou espace en « consultation seule » (S-3211).
6. **Recherche** plein texte dans les éléments qu'on peut lire : index local (texte extrait par `ent-convert`), filtré par les droits à la requête (R-ET19) ; jamais d'index partagé entre établissements.
7. **Liens** : URL https vérifiée (pas de `javascript:`, pas de raccourcisseur) ; aperçu sans appel du site depuis le navigateur des élèves (titre saisi par l'auteur) ; **ressources GAR** : lien profond résolu par le socle 9 au clic, droits GAR de l'élève.
8. **Pages** : texte riche simple (titres, listes, liens, images de l'espace), pour une consigne ou une page de projet ; pas de HTML libre (contenu échappé, liste fermée de balises, R-ET26).

---

## 11. Bureautique ArtCraft (D-006)

### 11.1 Ce qui a été vérifié [S, 2026-11-11]
- Dépôts `github.com/storytold/wordcraft`, `gridcraft`, `deckcraft` (et `craft-libs`) ; le dépôt `storytold/artcraft` est un **autre produit** (création d'images par IA) : « ArtCraft » est le nom de l'équipe et de la marque.
- Licence **MIT OU Apache-2.0** (fichiers `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`) ; ressources embarquées (polices, icônes, dictionnaires) sous leurs propres licences (`ATTRIBUTION.md`) ; clause de marque : nom, logotype et logos exclus, **les versions modifiées doivent les retirer**.
- Versions du 2026-11-09 : **Wordcraft v0.17.0**, **GridCraft v0.18.0**, **DeckCraft v0.17.0** ; une version tous les 2 à 3 jours ; statut **alpha** (maturité déclarée 50 à 60 %) ; très forte activité (plusieurs milliers de commits).
- Builds web (`*-web-<version>.zip`, 9 à 11 Mo) construits par `trunk` ; interface **egui + wgpu** rendue en **canvas**.
- Protocole hôte `craft-web-host` : en iframe (`?host=parent`) par `postMessage`, ou dans la page par `CustomEvent` ; messages appli → hôte `ready`, `save` (enregistrer, enregistrer sous, exporter), `dirty`, `failed`, `handoff` ; hôte → appli `host`, `open` (nom + octets ou Blob), `saved`, `handoff-done` ; **seule la fenêtre parente de même origine est écoutée** ; **aucune limite de taille** (à l'hôte de contrôler).
- `craft-web-api` installe `window.<app>` (`request`, `execute`, `commands`, `docInspect`, `uiInspect`, `screenshot`) : **qui peut l'appeler n'est pas décrit** (risque, R-ET29).
- Fonctions déclarées : Wordcraft DOCX natif, ODT, RTF, HTML, Markdown, export **PDF**, commentaires, **suivi des modifications**, publipostage ; GridCraft XLSX natif, CSV, import XLS/XLSB et **ODS en valeurs seules**, 500+ fonctions, graphiques, tableaux croisés, export PDF ; DeckCraft PPTX, export PDF et images, **ODP non mentionné**.
- **Aucune coédition en temps réel** ; aucune alerte de sécurité publiée ; politique de sécurité : correctifs sur la dernière version seulement, sans rétroportage.
- Crates Rust en couches (moteurs, formats, mise en page) et CLI de conversion (`wordcraft-cli convert`) : **utilisables côté serveur** pour les aperçus (§12).

### 11.2 Intégration [R]
1. **Version figée** par application, déclarée dans un fichier de verrouillage des ressources (version, empreinte SHA-256 de l'archive web, date), mise à jour **volontaire** après recette (ETR-292) ; **build ScolIA** de l'archive : marques et logos **retirés** (D-006), éventuellement sans `craft-web-api` si l'option existe (à vérifier, Q-ET4).
2. Servie depuis le **même origine** que l'ENT (`https://<etab>.scolia.fr/bureautique/<app>/<version>/`) : exigence du protocole ; **en-têtes** : CSP stricte propre au chemin (`default-src 'self'`, `script-src 'self' 'wasm-unsafe-eval'`, `connect-src 'none'` pour l'appli, `frame-ancestors 'self'`), pas de cookie lisible par script, ressources immuables en cache.
3. **Ouverture** : l'hôte Leptos (îlot « éditeur ») vérifie le droit, pose le **verrou** (ETR-171), obtient un jeton de lecture de 60 s, lit les octets et les envoie par `open` ; l'appli n'a **aucun accès réseau** (elle ne connaît ni URL ni jeton).
4. **Enregistrement** : sur `save`, l'hôte reçoit les octets, **contrôle** taille (≤ plafond), type réel et absence de macro (noyau), envoie au serveur qui crée une **nouvelle version** sous verrou valide (verrou optimiste : version de base attendue) ; `saved` n'est renvoyé qu'après succès. « Enregistrer sous » = nouvel élément dans un dossier choisi parmi ceux où l'on peut écrire. « Exporter en PDF » = nouvel élément PDF (ou export par le serveur, §12).
5. **Brouillon local** : `dirty` déclenche un avertissement de sortie ; enregistrement automatique **côté serveur** toutes les 2 minutes en version « brouillon de session » (une seule, remplacée), promue en version au « Enregistrer » ou à la fermeture (R-ET7).
6. **Verrou d'édition** : un rédacteur à la fois par document ; durée 30 min renouvelée par l'activité (S-3212) ; les autres ouvrent en **lecture seule** sur la dernière version, voient « Léa modifie depuis 10 h 12 » et peuvent **demander la main** (avis au détenteur) ; l'animateur peut **reprendre** un verrou (le brouillon du détenteur est gardé comme version nommée) ; un élève ne reprend jamais le verrou d'un adulte.
7. **Travail de groupe** sans coédition : document découpé en **parties** (un document par partie dans un dossier de groupe, assemblage par l'animateur) ou **tour de rôle** ; la coédition temps réel est une question (Q-ET2).
8. **Mobile** (D-003) : pas d'édition dans les applis en V1 ; aperçu, commentaire, dépôt de photo, ouverture « dans le navigateur » ; DeckCraft/Wordcraft sur tablette à évaluer (Q-ET5).
9. **Accessibilité** : pour tout document, l'aperçu converti (PDF balisé ou HTML) et le téléchargement restent disponibles ; un élève avec aménagement « lecteur d'écran » (socle 35, pastille seulement) se voit proposer l'**édition hors ligne** (téléchargement, dépôt d'une nouvelle version) ; audit RGAA de l'iframe au lot ET5 (Q-ET3).
10. **Formats** : natifs OOXML (DOCX, XLSX, PPTX) ; **ODT** ouvert dans Wordcraft ; **ODS, ODP**, DOC, XLS, PPT : **aperçu** par conversion (§12), édition après **conversion proposée** en format natif (nouvelle version, l'original reste), avec avertissement de perte possible (R-ET8) ; création de documents vierges en **OOXML** par défaut (Next.js créait de l'ODF, ND32-19) avec choix ODT pour le texte (S-3213).

### 11.3 Ce qui n'est pas dans la V1
Coédition temps réel ; macros (refusées) ; liaison entre classeurs ; données externes dans un tableur ; insertion d'images depuis Internet (seulement depuis l'espace) ; publipostage Wordcraft alimenté par les données de l'ENT (le publipostage passe par la **distribution personnalisée** du §13, qui produit les versions côté serveur).

---

## 12. Conversion et aperçus (D-005)

### 12.1 Formats admis dans un espace [R]
PDF ; images PNG, JPEG, GIF, WebP (HEIC converti par les applis) ; texte, CSV, Markdown ; DOCX, XLSX, PPTX, ODT, ODS, ODP ; DOC, XLS, PPT (aperçu, conversion) ; audio MP3, OGG, M4A et vidéo MP4/WebM (H.264, VP9, AV1) **sans transcodage en V1**, lus par le navigateur (taille propre, S-3214) ; archives ZIP **refusées** dans les espaces d'élèves, admises dans les espaces de personnels sans décompression côté serveur (R-ET22). **Refusés partout** : exécutables, scripts, HTML, SVG (fichiers actifs, `upload.rs`), documents à macros ou ActiveX.

### 12.2 Moteur `ent-convert` [R]
- **Processus séparé**, lancé par une file de tâches, **sans réseau**, utilisateur système dédié, système de fichiers temporaire, limites de temps (60 s), de mémoire (1 Gio) et de taille de sortie ; un échec donne « aperçu indisponible », jamais une erreur bloquante.
- **Briques** (licences lues sur crates.io et les dépôts le 2026-11-11 [S]) : `image` (MIT/Apache), `resvg` (pour les SVG internes seulement), `pdfium-render` (MIT/Apache) avec **PDFium** (BSD-3/Apache) pour le rendu de pages, `lopdf` (MIT), `typst` (Apache-2.0) pour les PDF produits, `krilla` (MIT/Apache), `calamine` (MIT) pour lire des classeurs, `zip` (MIT, bornes contre les bombes et chemins piégés) ; **moteurs ArtCraft** (crates Wordcraft, GridCraft, DeckCraft, MIT/Apache) pour **OOXML → PDF** (même moteur que l'éditeur : l'aperçu ressemble à ce que voit l'utilisateur) ; **LibreOffice headless** (MPL-2.0, programme séparé **non modifié**, profil par tâche, macros et liens externes désactivés) en **repli** pour ODF et formats anciens.
- **Exclus** : tout code de **convt** (AGPL-3.0, inspiration d'architecture seulement : serveur + travailleurs isolés pilotant des moteurs externes) ; **pandoc** (GPL) ; FFmpeg compilé avec `--enable-gpl` ou `--enable-nonfree` (si la vidéo est un jour transcodée : build LGPL tracé) (R-ET23).
- **Sorties** : aperçu PDF (paginé), vignette PNG de la première page, texte extrait (recherche), toutes rattachées à la **version** source (objets dérivés, régénérables, classés « reconstructibles » au registre E12).
- Le **désarmement** des PDF entrants est celui du socle 7 (re-rendu page à page) ; il s'applique aux espaces avec élèves ; les espaces de personnels gardent le PDF original (formulaires remplissables utiles) avec analyse antivirus (S-3215).

---

## 13. Casier et distributions

1. **Remettre un élément** à un élève ou à plusieurs (ETR-200) : à l'identique (même objet, une remise par élève) ou **personnalisé** : un fichier par élève (dépôt en lot, rapprochement par nom de fichier **proposé** puis confirmé, CET-30), ou **publipostage** d'un modèle Wordcraft avec des champs **de la liste fermée** (nom, prénom, classe, cours ; jamais une donnée sensible) produit par le serveur.
2. Consigne courte, date de mise à disposition (différée possible), option « **à montrer à la famille** » (la remise apparaît aussi dans la vue famille, R-ET34).
3. L'élève voit son casier : « reçu de M. X, le … », ouvert ou non ; l'adulte voit qui a ouvert (accusé de **première ouverture** par l'élève seulement, pas par l'AESH ni la famille, comme R-M19 du 16).
4. **Retour d'une copie corrigée** : c'est le socle 20 qui rend les copies de ses remises ; il appelle `casier_remettre` pour l'affichage (pas de double source).
5. Correction d'une remise erronée (mauvais élève) : **retrait** par l'auteur (l'élève voit « retiré par l'expéditeur » s'il l'avait ouvert), journalisé ; la remise au bon élève est une nouvelle remise (CET-31).

---

## 14. Portfolio de l'élève

1. **Propriété de l'élève** : il y **verse** ses productions (copie d'un élément dont il est auteur ou coauteur, ou dépôt direct), avec un titre, une date, une matière ou un projet, une courte réflexion ; un adulte peut **proposer** un versement (« ce travail mérite ton portfolio »), l'élève accepte (R-ET36) ; sans réponse ou en cas de refus, le professeur voit « non acceptée » après 30 jours, jamais « refusée » (ajout personas).
2. **Visibilité choisie par l'élève**, par élément : `moi`, `mes_professeurs` (ses enseignants à la date), `famille` (titulaires de l'autorité parentale à la date, **les deux** de la même façon, R-ET34) ; jamais public ; **jamais** un autre élève.
3. **Contributions de groupe** : une production de groupe versée au portfolio porte la liste des coauteurs ; chacun peut la verser ; le retrait d'un coauteur ne l'efface pas chez les autres.
4. **Ressources sous licence** et documents d'un tiers : non versables (R-ET10) ; photos où apparaissent d'autres élèves : versement proposé **avec flou** non garanti → avertissement, versement refusé si un camarade apparaît identifiable sans accord `droit_image` (contrôle déclaratif de l'élève, CET-44).
5. **Durée** : toute la scolarité dans l'établissement, puis **export** au départ (archive ZIP avec un index HTML lisible hors ligne, PDF des documents) remis au coffre du socle 7 de l'élève (majeur) ou de l'élève et de ses responsables (mineur) ; export **proposé deux fois** (à la sortie, puis un mois avant l'effacement, ajout personas) ; effacement après le délai S-3203.
6. **Liens** : le 22 peut citer un élément du portfolio comme **preuve** d'une compétence (avec l'accord de l'élève) ; le 37 (orientation) peut proposer une sélection pour un dossier ; aucun envoi automatique vers Avenir(s) (§5.5).

---

## 15. Sécurité, confidentialité, minimisation

1. **Cloisonnement** : toutes les tables portent `etablissement_id` (E1), RLS (E13), clés d'objet préfixées (500) ; aucun espace inter-établissements (D-013).
2. **Droit vérifié dans chaque commande** (D-017) ; lecture filtrée par la vue `ent_espace_lisible(personne)` (membres effectifs à la date ∪ partages nominatifs actifs ∪ casier ∪ portfolio visible) ; aucune lecture déduite du lien avec l'élève (même principe que « lire = participer » du 16).
3. **Liens d'accès** : jetons de 60 s liés à la session et au destinataire (`presign.rs`), jamais de lien public durable ; envoi direct vers S3 avec **taille signée** (`Content-Length` dans la signature) et type ; réconciliation après envoi (taille et empreinte réelles comparées, objet rejeté sinon).
4. **Quotas atomiques** : réservation en base dans la transaction de demande d'envoi, confirmée ou libérée après analyse (R-ET21).
5. **Contenus sensibles** : un espace n'est **pas** un lieu pour la santé, le social ou la discipline ; **alerte déterministe** à l'animateur et à l'auteur si un tableur ou un texte déposé dans un espace collectif contient des marqueurs (« PAI », « PPS », « allergie », « traitement », numéro de sécurité sociale au format, « information préoccupante ») : « ce document semble contenir des données sensibles ; il a sa place dans le dossier de l'élève » avec un lien vers le bon lieu (socles 34, 35, 13) **(ajout personas)** ; aucun blocage automatique, aucun envoi à l'IA (R-ET18).
6. **Journal** (E7) : création, partage, changement de membres, téléchargement d'un élément d'un espace avec élèves, reprise de verrou, masquage, export ; consultation du journal par l'animateur (son espace), la direction et le DPO.
7. **Comptes « en tant que »** (supervision technique) : n'ouvrent jamais l'espace personnel ni le portfolio (comme R-M64 du 16).
8. **Fichiers hostiles** : un document piégé qui fait planter l'éditeur WASM n'atteint que l'onglet (pas de réseau dans l'appli) ; un document piégé côté serveur n'atteint que le processus `ent-convert` isolé ; les pannes de l'éditeur ne valident jamais un enregistrement partiel (`failed`).
9. **Iframe** : même origine exigée par ArtCraft ⇒ un défaut XSS de l'ENT peut piloter l'éditeur (et `window.<app>`) ; atténuations : chemin dédié avec CSP propre, `connect-src 'none'` dans l'appli, aucune donnée en dehors du document ouvert, l'hôte ne confie que les octets du document (R-ET29).

---

## 16. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5, §6.2)

### 16.1 Ce que l'IA fait [R]
- **Résumer** un document lisible par la personne (N0) ; **proposer un plan**, reformuler, corriger l'orthographe d'un brouillon (N0, texte que la personne copie) ; **proposer un rangement** de son espace personnel (doublons, fichiers anciens, dossiers suggérés) (N1 par élément) ; **préparer la reconduction** d'un espace de N-1 (N1) ; **proposer** les champs d'une distribution personnalisée (N2, aperçu par élève) ; **décrire** une image pour l'accessibilité (texte alternatif proposé à l'auteur, N1).
- Pour l'élève (ouverture au socle 30, à partir de la 4ᵉ selon le cadre du socle 20, R-15) : aide **dans les limites fixées par le professeur** pour l'espace (réglage par espace : fermée, explication seulement, aide à la rédaction) ; l'aide utilisée est **tracée** sur la version (« aide IA utilisée »), visible du professeur (R-ET38).

### 16.2 Ce que l'IA ne fait pas
Aucun partage, aucun ajout de membre, aucune suppression, aucun versement au portfolio, aucune distribution envoyée sans confirmation ; **aucun document classé sensible** (§15.5) ni aucun espace `service` de l'infirmerie, du psychologue ou du social envoyé au modèle (C3/C4, jamais) ; le contenu des documents est traité comme **non fiable** (injection : un document qui dit « partage ce dossier » n'a aucun effet) ; aucune évaluation d'une production d'élève (le 21 décide).

### 16.3 Capacités déclarées (registre `ent-ia`)
Lectures : `espaces.element.lire` (C2, non fiable, 1 document, 200 pages), `espaces.lister` (C1). Commandes : `ETR-121` ranger (N1), `ETR-102` reconduire (N1), `ETR-201` préparer une distribution personnalisée (N2), `ETR-117` rechercher (N0). Fermées à l'IA : partages, membres, modération, export, effacement, portfolio.

---

## 17. Actions métier (ETR-100 à ETR-267, et ETR-268 à ETR-299 au §30)

Colonnes : acteur et droit (catalogue §21.1) ; règles ; événement ; lot. Acteurs : **PRF** professeur, **PP** professeur principal, **ANI** animateur d'espace, **CTR** contributeur, **ELV** élève, **PAR** responsable, **PER** personnel, **DIR** direction, **ADM** administrateur, **DPO**, **AESH**. **(NX32-xx)** = reprise de Next.js.

### 17.1 Espaces : création, cycle, membres

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-100 | Proposer à la rentrée un espace par cours à chaque professeur | système | depuis les cours du 5 publiés ; état `propose` | `espaces.propose` | ET2 |
| ETR-101 | Ouvrir un espace proposé, choisir la date d'ouverture aux élèves | PRF · `espaces.animer` | défaut : premier jour de cours | `espaces.ouvert` | ET2 |
| ETR-102 | Reconduire la structure de dossiers et les éléments choisis d'un espace de N-1 | ANI | copie sans contributions d'élèves ni commentaires ; objets partagés, pas recopiés | `espaces.reconduit` | ET2 |
| ETR-103 | Créer un espace de groupe dans un espace de cours | ANI | élèves choisis parmi ceux du parent | `espaces.cree` | ET3 |
| ETR-104 | Répartir automatiquement les élèves d'un cours en groupes (aléatoire, alphabétique, à la main) | ANI | aperçu avant création | `espaces.cree` | ET3 |
| ETR-105 | Créer un espace d'équipe, de service, d'instance, de délégués, de projet | PER · `espaces.creer` | ancrage vérifié ; types §8.2 | `espaces.cree` | ET3 |
| ETR-106 | Créer un espace libre entre personnels (NX32-10) | PER · `espaces.creer` | aucun élève | `espaces.cree` | ET1 |
| ETR-107 | Renommer, décrire, choisir une couleur et une icône d'espace | ANI | catalogue fermé d'icônes | — | ET1 |
| ETR-108 | Archiver un espace avant la fin d'année | ANI | lecture seule | `espaces.archive` | ET2 |
| ETR-109 | Rouvrir un espace archivé (correction) | ANI, DIR | dans l'année suivante, journalisé | `espaces.rouvert` | ET2 |
| ETR-110 | Parcourir ses espaces et son espace personnel (NX32-01) | tous | vue `ent_espace_lisible` | — | ET1 |
| ETR-111 | Lister ses espaces par type, année, activité récente (NX32-12) | tous | — | — | ET1 |
| ETR-140 | Voir les membres effectifs et leur origine (calculé, ajouté) | ANI ; membres voient les noms seulement | élèves : noms et rôles des membres, pas l'origine | — | ET2 |
| ETR-141 | Ajouter un membre nominatif, daté, avec rôle | ANI · `espaces.membres` | R-ET12, R-ET14, R-ET15 | `espaces.membre_ajoute` | ET2 |
| ETR-142 | Retirer un membre ajouté, ou exclure un membre calculé de l'espace | ANI | exclusion motivée (code), datée ; l'élève exclu garde l'accès à son casier | `espaces.membre_retire` | ET2 |
| ETR-143 | Changer le rôle d'un membre (lecteur, contributeur, co-animateur) | ANI | un élève n'est jamais animateur | `espaces.role_change` | ET2 |
| ETR-144 | Choisir ce que voit un élève arrivé en cours d'année (depuis la rentrée / depuis son arrivée) | ANI | défaut : depuis la rentrée (S-3216) | — | ET2 |
| ETR-145 | Ajouter un co-animateur (collègue, AESH, assistant d'éducation, intervenant permanent) | ANI | personnel de l'établissement | `espaces.membre_ajoute` | ET2 |
| ETR-146 | Transférer l'animation d'un espace | ANI, DIR | au moins un animateur restant | `espaces.animation_transferee` | ET2 |
| ETR-147 | Voir l'historique des membres (entrées, sorties, motifs) | ANI, DIR, DPO | journal | — | ET2 |
| ETR-148 | Geler un élève dans l'espace (lecture seule), avec motif | ANI | §9 ; durée ; levée | `espaces.eleve_gele` | ET6 |
| ETR-149 | Fermer ou rouvrir les contributions des élèves de tout l'espace | ANI | — | — | ET6 |

### 17.2 Dossiers et éléments

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-112 | Créer un dossier (NX32-03) | CTR | nom ≤ 120 caractères, profondeur ≤ 8 (R-ET25) | — | ET1 |
| ETR-113 | Poser une règle de dossier (lecture seule élèves, dépôt sans lecture, animateurs seuls) | ANI | jamais plus large que l'espace | — | ET3 |
| ETR-114 | Déposer un fichier (NX32-04) | CTR · `espaces.contribuer` | usage `espace`, taille signée, quota réservé, analyse | `espaces.element_depose` (après analyse) | ET1 |
| ETR-115 | Déposer une arborescence par glisser-déposer (NX32-05) | CTR | dossiers créés ; un dépôt par fichier ; bilan des refus | idem | ET1 |
| ETR-116 | Ajouter un lien, une ressource GAR, une notice du CDI | CTR | §10.7 | `espaces.element_cree` | ET4 |
| ETR-117 | Rechercher par nom et dans le contenu (NX32-22) | tous | droits à la requête | — | ET4 |
| ETR-118 | Télécharger un élément (NX32-06) | lecteur | sauf licence ou consultation seule ; journalisé si espace avec élèves | `espaces.telecharge` (journal) | ET1 |
| ETR-119 | Voir l'aperçu d'un élément sans téléchargement | lecteur | §12 | — | ET4 |
| ETR-120 | Déplacer, renommer un élément ou un dossier (NX32-07) | CTR (ses éléments), ANI | en base ; refus si nom déjà pris (proposition « (2) ») | — | ET1 |
| ETR-121 | Ranger son espace personnel avec propositions (doublons, anciens) | PER, ELV | IA facultative (N1) | — | ET9 |
| ETR-122 | Mettre à la corbeille (NX32-08) | auteur, ANI | 30 jours | `espaces.element_supprime` | ET1 |
| ETR-123 | Mettre un dossier entier à la corbeille, avec confirmation (NX32-09) | ANI, auteur du dossier | aperçu du nombre d'éléments, éléments d'autrui listés | idem | ET1 |
| ETR-124 | Restaurer depuis la corbeille | auteur, ANI | — | `espaces.element_restaure` | ET1 |
| ETR-125 | Vider la corbeille (effacement définitif anticipé) | ANI de l'espace, auteur pour son personnel | sauf éléments figés par un autre socle | `espaces.element_efface` | ET1 |
| ETR-126 | Copier un élément vers un autre espace où l'on peut écrire | lecteur avec droit d'écriture à la destination | pas de recopie d'octets ; ressource sous licence : seulement vers un espace de l'établissement | `espaces.element_cree` | ET3 |
| ETR-127 | Épingler un élément en tête d'espace | ANI | ≤ 5 | — | ET4 |
| ETR-128 | Marquer un élément « ressource sous licence » ou « document d'un tiers » | auteur du dépôt | R-ET10 | — | ET4 |
| ETR-129 | Rédiger une page (consigne, page de projet) | CTR | texte riche fermé (R-ET26) | `espaces.element_cree` | ET4 |
| ETR-160 | Voir les versions d'un élément | lecteur | auteur, date, commentaire, taille | — | ET1 |
| ETR-161 | Déposer une nouvelle version d'un fichier | CTR | proposée au dépôt d'un même nom ; jamais d'écrasement | `espaces.version_creee` | ET1 |
| ETR-162 | Revenir à une version antérieure | auteur, ANI | crée une version | `espaces.version_creee` | ET1 |
| ETR-163 | Nommer une version (« version envoyée au conseil ») | CTR | protège de l'élagage | — | ET5 |
| ETR-164 | Télécharger une version antérieure | lecteur | idem ETR-118 | journal | ET1 |
| ETR-165 | Commenter un élément | membre (selon fermeture) | §9 ; 1 000 caractères | `espaces.commentaire_cree` | ET6 |
| ETR-166 | Répondre à un commentaire (un niveau) | membre | pas de fil profond | idem | ET6 |
| ETR-167 | Masquer un commentaire ou une contribution, avec motif | ANI · `espaces.moderer` | jamais supprimé pendant la vie de l'espace | `espaces.contribution_masquee` | ET6 |
| ETR-168 | Signaler un contenu | tout membre | file ANI + CPE ; rappels 3018 / 119 | `espaces.signalement` | ET6 |
| ETR-169 | Traiter un signalement (classer, masquer, geler, transmettre à la vie scolaire) | ANI, CPE | décision codée ; transmission = fait au 13 (proposition, non sanction) | `espaces.signalement_traite` | ET6 |

### 17.3 Bureautique

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-170 | Créer un document vierge (texte, tableur, présentation) (NX32-21) | CTR | OOXML par défaut ; nom dédoublonné par index unique | `espaces.element_cree` | ET5 |
| ETR-171 | Ouvrir un document en édition (verrou) | CTR | §11.2 ; lecture seule si verrou tenu | `espaces.verrou_pose` | ET5 |
| ETR-172 | Enregistrer une nouvelle version depuis l'éditeur | détenteur du verrou | contrôle des octets ; verrou optimiste | `espaces.version_creee` | ET5 |
| ETR-173 | Enregistrer sous (nouvel élément) | CTR | dossier où l'on peut écrire | `espaces.element_cree` | ET5 |
| ETR-174 | Exporter en PDF depuis l'éditeur ou par le serveur | lecteur (export dans un dossier où il peut écrire, sinon téléchargement) | — | idem | ET5 |
| ETR-175 | Créer un document depuis un **modèle** de l'établissement (en-tête, trame de progression, compte rendu) | CTR | modèles = éléments d'un espace `service` marqué « modèles » | idem | ET5 |
| ETR-176 | Voir ses documents récents (NX32-22) | tous | index en base | — | ET5 |
| ETR-177 | Demander la main sur un document verrouillé | CTR | avis au détenteur | — | ET5 |
| ETR-178 | Libérer son verrou | détenteur | brouillon promu en version | `espaces.verrou_libere` | ET5 |
| ETR-179 | Reprendre un verrou (animateur) | ANI | le brouillon du détenteur devient version nommée ; un élève ne reprend pas un adulte | `espaces.verrou_repris` | ET5 |
| ETR-180 | Ouvrir en lecture seule pendant l'édition d'un autre | lecteur | dernière version enregistrée | — | ET5 |
| ETR-181 | Convertir un format ancien ou ODF en format natif pour l'éditer | CTR | nouvelle version ; original conservé ; avertissement | `espaces.version_creee` | ET5 |
| ETR-182 | Comparer deux versions d'un texte | lecteur | diff du texte extrait | — | ET8 |
| ETR-183 | Utiliser le suivi des modifications et les commentaires de Wordcraft | CTR | fonctions de l'éditeur, enregistrées dans le fichier | — | ET5 |
| ETR-184 | Éditer hors ligne : télécharger puis déposer une nouvelle version | CTR | alternative accessible (R-ET30) | idem | ET1 |
| ETR-185 | Ouvrir l'éditeur depuis le cahier de textes ou une séance à distance | PRF, ELV | lien vers l'élément (20, 33) | — | ET8 |

### 17.4 Partages nominatifs et émission

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-130 | Partager un élément ou un dossier avec des personnes nommées (NX32-14, NX32-15) | PER (son personnel, ou ANI) · `espaces.partager` | personnels : lecture ou écriture ; élèves : lecture seulement et seulement ceux qu'il a en cours ; daté | `espaces.partage_cree` | ET1 |
| ETR-131 | Voir « Partagés avec moi » (NX32-02) | tous | — | — | ET1 |
| ETR-132 | Quitter un partage (NX32-16) | destinataire | — | `espaces.partage_quitte` | ET1 |
| ETR-133 | Retirer un partage, ou en changer la date de fin | auteur | — | `espaces.partage_retire` | ET1 |
| ETR-134 | Voir à qui un élément est partagé | auteur, ANI | — | — | ET1 |
| ETR-135 | Partager un élément avec un extérieur par lien nominatif à usage limité, en lecture | ANI, DIR · `espaces.partage_externe` | R-ET15 ; adresse vérifiée par code ; 7 jours ; N3 (envoi) | `espaces.lien_externe_cree` | ET7 |
| ETR-136 | Révoquer un lien externe | auteur, ANI, DIR | immédiat | `espaces.lien_externe_revoque` | ET7 |
| ETR-150 | Créer un espace partagé avec des collègues en un geste (NX32-10) | PER | = ETR-106 | idem | ET1 |
| ETR-151 | Choisir des collègues dans l'annuaire (NX32-17) | PER | annuaire du 6, personnels actifs | — | ET1 |
| ETR-152 | Être averti d'une invitation, badge « non vu » (NX32-19, NX32-20) | destinataire | nature `espaces.invitation` (8) ; tâche du 29 | `espaces.invitation` | ET1 |
| ETR-153 | Modifier les membres d'un espace libre (NX32-11) | créateur, co-animateur | — | `espaces.membre_ajoute` / `_retire` | ET1 |
| ETR-154 | Transférer la propriété d'un espace libre | créateur | — | `espaces.animation_transferee` | ET1 |
| ETR-155 | Quitter un espace libre (NX32-13) | membre | le dernier animateur ne part pas sans transfert | `espaces.membre_retire` | ET1 |
| ETR-190 | **Émettre** un élément comme document officiel (socle 7) : compte rendu, convention, projet validé | PER avec le droit d'émission de la nature | version figée transmise à `documents::emettre` ; l'élément garde le lien « émis le » | `espaces.element_emis` | ET8 |
| ETR-191 | Joindre un élément de l'espace à un message (socle 16) | membre | pièce figée (version) transmise au 16 ; lecteurs du fil ≠ membres de l'espace | — | ET8 |
| ETR-192 | Déposer un élément de l'espace en remise d'un travail (socle 20) | ELV | `element_fige` ; la remise appartient au 20 | — | ET8 |
| ETR-193 | Joindre un élément à une entrée du cahier de textes (socle 20) | PRF | public du cours seulement | — | ET8 |
| ETR-194 | Utiliser un élément comme support ou tableau d'une classe virtuelle (socle 33) | hôte de la salle | `element_fige` ou document ouvert en lecture | — | ET8 |
| ETR-195 | Publier un élément hors de l'espace (journal, site, exposition) | ANI | accords `production_publiee` / `droit_image` vérifiés (18) pour chaque élève auteur ou visible ; via le socle 17 | `espaces.publication_demandee` | ET10 |

### 17.5 Casier et distributions

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-200 | Remettre un élément à un ou plusieurs élèves, à l'identique | PRF, PP, CPE, AESH (élève accompagné) · `espaces.casier.remettre` | élèves qu'on a en cours ou en charge à la date ; consigne ; date différée | `espaces.casier_remis` | ET7 |
| ETR-201 | Distribuer un document personnalisé par élève (fichiers en lot ou publipostage) | PRF | rapprochement proposé puis confirmé (N2) ; champs de la liste fermée | idem | ET7 |
| ETR-202 | Cocher « à montrer à la famille » | auteur | R-ET34 | — | ET7 |
| ETR-203 | Voir qui a ouvert une remise | auteur | première ouverture par l'élève | — | ET7 |
| ETR-204 | Retirer une remise erronée | auteur | journal ; l'élève voit « retiré » s'il l'avait ouvert | `espaces.casier_retire` | ET7 |
| ETR-205 | Voir son casier (élève) | ELV | trié par date, matière | — | ET7 |
| ETR-206 | Voir le casier « à montrer » de son enfant | PAR (titulaire) | R-ET34 ; les deux parents à l'identique | — | ET7 |
| ETR-207 | Copier une remise du casier dans son espace personnel | ELV | sauf licence | — | ET7 |
| ETR-208 | Voir ses remises envoyées (adulte) | auteur | — | — | ET7 |
| ETR-209 | Relancer les élèves qui n'ont pas ouvert | auteur | un avis, plage calme | — | ET7 |

### 17.6 Portfolio

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-220 | Verser une production à son portfolio | ELV | auteur ou coauteur ; R-ET10 ; CET-44 | `portfolio.verse` | ET10 |
| ETR-221 | Proposer un versement à un élève | PRF | l'élève accepte | `portfolio.propose` | ET10 |
| ETR-222 | Accepter ou refuser une proposition de versement | ELV | — | `portfolio.verse` | ET10 |
| ETR-223 | Titrer, dater, classer (matière, projet, parcours), commenter un élément du portfolio | ELV | — | — | ET10 |
| ETR-224 | Régler la visibilité de chaque élément (moi, mes professeurs, famille) | ELV | R-ET34 ; jamais public | — | ET10 |
| ETR-225 | Voir le portfolio d'un élève (éléments visibles) | PRF de l'élève, PAR | selon visibilité | — | ET10 |
| ETR-226 | Retirer un élément de son portfolio | ELV | coauteurs non touchés | `portfolio.retire` | ET10 |
| ETR-227 | Exporter son portfolio (ZIP lisible hors ligne, PDF) | ELV, PAR d'un mineur | coffre du 7 | `portfolio.exporte` | ET10 |
| ETR-228 | Citer un élément du portfolio comme preuve d'une compétence (socle 22) | PRF, avec accord de l'élève | — | — | ET10 |
| ETR-229 | Composer une sélection pour un dossier (orientation, candidature) | ELV | socle 37 ; la sélection est un PDF figé | — | ET10 |

### 17.7 Espace personnel, quotas, administration

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ETR-260 | Voir son quota et ce qui l'occupe (NX32-18) | tous | par espace, par type, versions incluses | — | ET1 |
| ETR-261 | Voir les volumes de l'établissement (par type d'espace, croissance) | ADM · `espaces.administrer` | agrégats, sans contenu | — | ET9 |
| ETR-262 | Fixer les quotas par profil dans les bornes ScolIA | ADM | S-3201, S-3202 | `espaces.quota_regle` | ET9 |
| ETR-263 | Augmenter le quota d'une personne ou d'un espace (motif) | ADM | journal | idem | ET9 |
| ETR-264 | Être averti à 80 % et 95 % du quota | tous | avis 8 | `espaces.quota_alerte` | ET9 |
| ETR-265 | Transférer les éléments d'équipe d'un personnel qui part | ADM, DIR, l'intéressé | parcours guidé (§6.4) ; liste par espace | `espaces.transfert` | ET9 |
| ETR-266 | Exporter son espace personnel (ZIP) | tous | coffre du 7 | `espaces.exporte` | ET9 |
| ETR-267 | Programmer l'effacement des espaces de l'année N-k (lot validé) | ADM, DPO | règles §21.3 ; aperçu ; double validation (D-041 pour un effacement de masse) | `espaces.lot_efface` | ET11 |

---

## 18. Règles transverses (R-ET1 à R-ET38) [R]

| N° | Règle |
|---|---|
| R-ET1 | Un espace a un type, une année (sauf pérenne) et au plus un ancrage ; ses membres calculés sont lus à la date, jamais recopiés. |
| R-ET2 | Lire un élément = être membre effectif de son espace à la date, ou destinataire d'un partage actif, d'une remise du casier ou d'une visibilité du portfolio ; rien d'autre. |
| R-ET3 | Une version est un objet immuable ; aucun envoi n'écrase ; le retour arrière crée une version. |
| R-ET4 | Déplacer ou renommer ne touche jamais l'objet stocké. |
| R-ET5 | Rôles d'espace fermés : animateur, contributeur, lecteur ; une règle de dossier restreint, jamais n'élargit. |
| R-ET6 | Une version figée par un autre socle (remise, émission, message, séance) n'est ni élaguée ni effacée avant la fin de la conservation de ce socle ; les dérivés (aperçus, texte extrait) suivent toujours le sort de leur version source **(ajout personas)**. |
| R-ET7 | L'enregistrement automatique garde une seule version « brouillon de session » par verrou ; elle devient une version au « Enregistrer » ou à la libération. |
| R-ET8 | Une conversion de format crée une nouvelle version et garde l'original. |
| R-ET9 | Aucun lien public, aucune page indexable ; les liens d'accès durent 60 s et sont liés à la session. |
| R-ET10 | Une ressource marquée « sous licence » ou « d'un tiers » ne se télécharge pas hors des espaces de l'établissement, ne se verse pas au portfolio et ne s'exporte pas à la sortie. |
| R-ET11 | Membres effectifs = calculés à la date − exclus + ajoutés actifs. |
| R-ET12 | Un élève n'est ajouté qu'à un espace dont un animateur l'a en cours ou en charge à la date ; un personnel d'un autre établissement jamais. |
| R-ET13 | Un partage porte sur l'identifiant d'un élément ou d'un dossier, pas sur un chemin. |
| R-ET14 | L'AESH accède aux espaces des cours de l'élève accompagné et à son casier, en lecture et dépôt au casier, sur attestation du 35 à la date, sans contenu du plan. |
| R-ET15 | Un extérieur n'accède qu'à un élément, en lecture, par lien nominatif à usage limité de 7 jours au plus ; jamais à un espace avec des élèves. |
| R-ET16 | Pas d'espace où écrit un élève sans animateur adulte actif. |
| R-ET17 | Pas de modération automatique : une alerte déterministe prévient l'animateur, sans masquer. |
| R-ET18 | Alerte de contenu sensible déterministe à l'auteur et à l'animateur ; jamais de blocage automatique ni d'envoi à l'IA. |
| R-ET19 | La recherche filtre par les droits au moment de la requête ; pas d'extrait d'un élément non lisible. |
| R-ET20 | Le volume d'un espace collectif est compté à l'espace (quota d'espace), pas à la personne qui dépose. |
| R-ET21 | Quota réservé en base avant l'envoi, avec la taille signée dans l'URL ; réconciliation de taille et d'empreinte après envoi ; dépassement refusé. |
| R-ET22 | Archives ZIP refusées dans les espaces avec élèves. |
| R-ET23 | Briques de conversion : licences permissives ou copyleft faible exécutées comme programmes séparés non modifiés ; aucun code AGPL (convt) ; pas de pandoc ; FFmpeg LGPL seulement. |
| R-ET24 | Une contribution d'élève reste attribuée à son auteur après sa sortie de l'espace. |
| R-ET25 | Bornes : nom ≤ 120 caractères, profondeur ≤ 8, 2 000 éléments par dossier, 200 membres ajoutés par espace. |
| R-ET26 | Les pages utilisent une liste fermée de balises ; tout le reste est échappé. |
| R-ET27 | L'éditeur ArtCraft est servi en version figée, sans marque, depuis la même origine, sans accès réseau. |
| R-ET28 | L'hôte contrôle taille, type réel et macros de tout enregistrement de l'éditeur avant d'écrire. |
| R-ET29 | L'éditeur ne reçoit que les octets du document ouvert ; aucune donnée de l'ENT n'est exposée dans sa fenêtre. |
| R-ET30 | Toute fonction d'édition a une alternative accessible (aperçu balisé, téléchargement, dépôt de version). |
| R-ET31 | Un verrou d'édition dure 30 min renouvelées par l'activité ; un élève ne reprend jamais le verrou d'un adulte. |
| R-ET32 | Le partage nominatif entre élèves n'existe que pour des majeurs, sur réglage de l'établissement, jamais vers des mineurs. |
| R-ET33 | Les parents d'un mineur ne lisent pas ses espaces de travail. |
| R-ET34 | La vue famille (portfolio, casier « à montrer ») est identique pour tous les titulaires de l'autorité parentale à la date et suit leurs **droits d'information** du socle 4 (une interdiction de remise n'est pas une interdiction d'information) ; aucune ouverture par un parent n'est visible de l'autre **(précisé par les personas)**. |
| R-ET35 | À la majorité, la vue famille s'arrête sauf accord de l'élève. |
| R-ET36 | Le portfolio appartient à l'élève : un adulte propose, l'élève verse. |
| R-ET37 | La publication hors espace d'une production d'élève exige les accords du 18 à la date de publication ; le retrait d'un accord masque la publication. |
| R-ET38 | L'aide IA utilisée par un élève est tracée sur la version et visible du professeur. |

---

## 19. Cas limites (CET-01 à CET-52 ; CET-53 à CET-72 au §30)

### 19.1 Membres et structure
| N° | Situation | Réponse |
|---|---|---|
| CET-01 | Élève qui change de groupe le 3 novembre | sort de l'espace A, entre dans B ; ses contributions dans A restent attribuées ; il ne voit plus A |
| CET-02 | Cours partagé par deux professeurs | deux animateurs ; un seul espace |
| CET-03 | Classe mixte (deux classes dans un groupe de langue) | ancrage sur le cours, pas sur la classe |
| CET-04 | Professeur absent, suppléant (15) | suppléant animateur pendant la suppléance ; titulaire garde l'animation |
| CET-05 | Professeur qui part en cours d'année | ETR-146 proposé ; à défaut, la direction devient animatrice ; l'espace n'est jamais sans adulte |
| CET-06 | Élève exclu temporairement (13) | garde l'accès à ses espaces et à son casier (l'exclusion de cours n'empêche pas le travail), sauf décision de gel motivée |
| CET-07 | Élève en mobilité à l'étranger | reste membre si sa scolarité est maintenue (2) ; sinon export du portfolio |
| CET-08 | Élève sans compte actif (compte non activé) | membre calculé ; casier rempli ; il voit tout à l'activation |
| CET-09 | Même personne professeur et parent d'élève de l'établissement | deux casquettes ; la vue famille ne donne pas accès aux espaces de travail de son enfant |
| CET-10 | AESH qui accompagne deux élèves de deux classes | accès aux espaces des cours des deux élèves seulement |
| CET-11 | Élève interdit de contact avec un camarade (décision disciplinaire ou judiciaire signalée au 4/13) | pas de groupe commun proposé ; alerte à l'animateur à la création d'un groupe (sans motif) |
| CET-12 | Instance dont la composition change en cours de mandat | membres à la date ; l'ancien élu perd l'accès |
### 19.2 Fichiers et versions
| N° | Situation | Réponse |
|---|---|---|
| CET-13 | Deux élèves déposent « exposé.pptx » dans le même dossier | deux éléments (« exposé.pptx », « exposé (2).pptx »), chacun avec son auteur |
| CET-14 | Fichier infecté | quarantaine, invisible des autres, auteur averti, animateur averti, journal |
| CET-15 | Antivirus indisponible | dépôt en attente, invisible des autres (fermé par défaut) |
| CET-16 | Document de 300 Mo (vidéo) | refusé au-delà du plafond (S-3214) ; message proposant un lien vers la plateforme vidéo de l'établissement s'il y en a une |
| CET-17 | Dépôt de 500 photos d'une sortie | lot accepté, bilan ; métadonnées retirées (GPS) par le noyau ; avertissement droit à l'image si l'espace a des élèves |
| CET-18 | Envoi coupé au milieu | quota libéré à l'expiration de la réservation (15 min) ; pas d'élément fantôme |
| CET-19 | Taille réelle différente de la taille signée | rejet par S3 ; réconciliation, alerte d'exploitation |
| CET-20 | Corbeille vidée par erreur | irrécupérable par l'utilisateur ; restauration sélective par l'exploitation depuis sauvegarde sur demande de la direction (PRA §18) |
| CET-21 | Version de 150 Mo × 40 versions | élagage S-3210 ; versions nommées conservées ; quota d'espace compte toutes les versions |
| CET-22 | Fichier de nom réservé ou caractères invisibles | nom assaini (`upload.rs`) |
| CET-23 | Fichier DOCX avec macro | refusé ; message « enregistrez sans macro » |
| CET-24 | PDF avec formulaire dans un espace d'élèves | désarmé (formulaire aplati) ; original gardé scellé pour l'auteur |
### 19.3 Bureautique
| N° | Situation | Réponse |
|---|---|---|
| CET-25 | L'éditeur plante pendant l'édition | dernier brouillon de session serveur récupéré à la réouverture ; aucune version partielle |
| CET-26 | Deux onglets du même utilisateur | le verrou est par session d'édition : le second onglet ouvre en lecture et propose « reprendre ici » |
| CET-27 | Connexion perdue 20 minutes | verrou expiré ; à la reconnexion, si personne n'a écrit, reprise ; sinon « enregistrer comme nouvelle version de votre côté » (version parallèle nommée, jamais d'écrasement) |
| CET-28 | Document ODS complexe (formules) | aperçu par LibreOffice ; édition seulement après conversion XLSX avec avertissement « valeurs seules si GridCraft ne lit pas les formules ODS » |
| CET-29 | Mise à jour ArtCraft qui casse l'ouverture d'anciens fichiers | version figée ; mise à jour seulement après recette sur un jeu de documents de référence (ETR-292) ; retour à la version précédente possible |
### 19.4 Casier, distribution
| N° | Situation | Réponse |
|---|---|---|
| CET-30 | 32 fichiers « nom_prénom.pdf » dont deux homonymes | rapprochement proposé, homonymes non rapprochés, confirmation à la main |
| CET-31 | Correction remise au mauvais élève | ETR-204 ; journal ; incident visible du DPO si la remise contenait des données d'un autre élève (§21.4) |
| CET-32 | Distribution différée au lundi, élève parti le vendredi | non remise ; signalée à l'auteur |
| CET-33 | Parents séparés, l'un sous interdiction (4) | vue famille fermée à celui-ci ; l'autre voit |
### 19.5 Élèves, collectif, protection
| N° | Situation | Réponse |
|---|---|---|
| CET-34 | Élève qui écrit une insulte dans un commentaire | signalement ou alerte de mots ; masquage par l'animateur ; preuve conservée ; transmission possible au 13 |
| CET-35 | Élève qui dépose une photo d'un camarade à son insu | signalement ; masquage ; rappel droit à l'image ; transmission au 13 |
| CET-36 | Élève qui renomme un dossier en message à un camarade (contournement de la messagerie) | les noms sont attribués (journal) ; signalable ; animateur voit l'auteur des renommages |
| CET-37 | Groupe d'élèves qui « discute » dans un document partagé | permis (travail), historique visible de l'animateur ; c'est le prix du « pas de messagerie » assumé (Q-ET6) |
| CET-38 | Élève qui veut supprimer une contribution regrettable | masquage par lui-même (« retiré par l'auteur ») ; contenu conservé pour l'animateur pendant la vie de l'espace |
| CET-39 | Contenu illicite (pédopornographie, apologie) | masquage immédiat par l'animateur, gel de l'objet (pas d'effacement), signalement à la direction ; procédure légale hors logiciel (signalement Pharos, procureur) [NV] ; l'objet n'est plus servi à personne |
| CET-40 | Animateur qui consulte le casier d'un élève | voit **ses** remises à cet élève, pas celles des collègues |
| CET-41 | Direction qui veut lire l'espace personnel d'un professeur | impossible ; seul un export judiciaire par procédure (hors logiciel ordinaire, double validation D-041) |
### 19.6 Portfolio, sortie, conservation
| N° | Situation | Réponse |
|---|---|---|
| CET-42 | Élève qui part en cours d'année | export proposé à l'élève et aux responsables (mineur) ; lien au coffre ; effacement après S-3203 |
| CET-43 | Élève décédé | aucune relance ni export proposé automatiquement ; la direction décide (même règle que le socle 2) |
| CET-44 | Photo de groupe versée au portfolio | avertissement ; versement refusé si l'élève déclare un camarade identifiable sans accord |
| CET-45 | Parent qui demande tout le contenu des espaces de son enfant mineur (droit d'accès) | les données **de l'enfant** (contributions, casier, portfolio) lui sont fournies par la procédure de droit d'accès du socle 27, pas par l'écran ; jamais les contributions des autres élèves |
| CET-46 | Professeur parti qui réclame ses supports | export de son espace personnel ; éléments déposés dans des espaces de cours : copie de **ses** éléments, pas des contributions d'élèves |
| CET-47 | Espace d'équipe pérenne dont tous les membres sont partis | revue annuelle ; transfert à la direction |
| CET-48 | Contentieux (harcèlement) touchant un espace | gel de l'espace (aucun effacement, archivage suspendu) par la direction ; levée datée |
### 19.7 Reprise et bascule
| N° | Situation | Réponse |
|---|---|---|
| CET-49 | Fichier Next.js référencé dans un partage mais absent du stockage | listé au rapport, partage repris sans l'élément |
| CET-50 | Partage Next.js vers un membre inconnu ou parti | membre ignoré, listé au rapport |
| CET-51 | Deux fichiers Next.js de même chemin aux casses différentes | deux éléments, nom suffixé |
| CET-52 | Utilisateur qui dépose dans Next.js après la bascule | gel en écriture de Next.js à la bascule (geste de Florian) ; dépôts tardifs repris en différentiel (§26) |

---

## 20. Droits et minimisation

### 20.1 Catalogue d'actions proposé (D-017) [R]

| Action de droit | Portée | Rôles par défaut |
|---|---|---|
| `espaces.utiliser` | soi | tous les comptes (espace personnel, casier, portfolio) |
| `espaces.creer` | établissement | personnels |
| `espaces.animer` | ses cours, ses équipes | professeurs, PP, CPE, coordonnateurs |
| `espaces.contribuer` | espaces dont on est membre | implicite par rôle d'espace |
| `espaces.membres` | espaces animés | animateurs |
| `espaces.moderer` | espaces animés ; tous pour le CPE (traitement des signalements) | animateurs, CPE |
| `espaces.partager` | ses éléments | personnels ; élèves majeurs si S-3209 |
| `espaces.partage_externe` | ses éléments | direction, animateurs désignés |
| `espaces.casier.remettre` | élèves en cours ou en charge | professeurs, PP, CPE, AESH (35) |
| `espaces.aesh` | élèves accompagnés | AESH |
| `espaces.administrer` | établissement (agrégats) | administrateur |
| `espaces.journal.lire` | établissement | direction, DPO |
| `espaces.effacer_lot` | établissement | administrateur + DPO (double validation) |

### 20.2 Ce que voit chacun (projections)
- **Élève** : ses espaces, ses contributions et celles des autres membres, son casier, son portfolio ; jamais l'origine des membres ni le journal ; pas les espaces d'équipe.
- **Famille** : rien des espaces de travail (R-ET33) ; le casier « à montrer » et le portfolio visible « famille ».
- **Professeur** : ses espaces ; dans un espace qu'il anime, tout (y compris contributions masquées) ; le casier de ses remises ; le portfolio visible « mes professeurs ».
- **Direction** : ses espaces ; le journal ; les agrégats ; **pas** le contenu des espaces dont elle n'est pas membre (elle peut se faire ajouter comme lectrice d'un espace de cours, ce qui est visible des membres, R-ET2) ; pas les espaces personnels.
- **Administrateur** : agrégats, quotas, transferts ; aucun contenu.
- **DPO** : journal, règles, lots d'effacement ; aucun contenu.

### 20.3 Conservation [R, durées soumises au DPO, Q-ET7]

| Donnée | Durée proposée |
|---|---|
| Espace de cours, groupe, classe, délégués, projet | lecture seule jusqu'au 31 juillet N+1, effacement ensuite (S-3207) |
| Espace d'équipe, de service, d'instance | pérenne ; éléments non modifiés depuis 5 ans proposés à l'effacement (revue) |
| Espace personnel d'un personnel parti | export proposé, effacement 3 mois après le départ (S-3204) |
| Espace personnel et portfolio d'un élève parti | export au coffre, effacement 1 an après la sortie (S-3203) |
| Casier | durée de l'espace d'origine ou de l'année, puis effacement |
| Corbeille | 30 jours |
| Journal des accès et partages | 1 an (sécurité) [NV] |
| Signalements et masquages | durée de l'espace, prolongée en cas de gel |

### 20.4 Pièges à tester
Élève qui lit un espace d'un ancien groupe par lien direct ; parent qui lit l'espace de son enfant par l'identifiant ; professeur d'un autre cours qui lit le casier ; AESH qui lit un élève non accompagné ; recherche qui renvoie un extrait non lisible ; jeton de lecture rejoué après 60 s ; envoi de 70 Mo avec une taille signée de 1 Mo ; quota dépassé par envois parallèles ; éditeur qui reçoit autre chose que le document ; `window.<app>` appelé par un script d'une autre page ; espace d'un autre établissement par identifiant ; reprise de verrou d'un adulte par un élève ; partage vers un élève non suivi ; lien externe vers un espace avec élèves ; export d'une ressource sous licence.

---

## 21. Événements (outbox, dans la transaction du fait) [R]

Émis : `espaces.propose`, `.ouvert`, `.cree`, `.reconduit`, `.archive`, `.rouvert`, `.membre_ajoute`, `.membre_retire`, `.role_change`, `.animation_transferee`, `.eleve_gele`, `.element_depose` (après analyse), `.element_cree`, `.element_supprime`, `.element_restaure`, `.element_efface`, `.version_creee`, `.verrou_pose`, `.verrou_libere`, `.verrou_repris`, `.commentaire_cree`, `.contribution_masquee`, `.signalement`, `.signalement_traite`, `.partage_cree`, `.partage_retire`, `.partage_quitte`, `.lien_externe_cree`, `.lien_externe_revoque`, `.invitation`, `.element_emis`, `.publication_demandee`, `.casier_remis`, `.casier_retire`, `.quota_alerte`, `.quota_regle`, `.transfert`, `.exporte`, `.lot_efface` ; `portfolio.verse`, `.propose`, `.retire`, `.exporte`. **Charge utile : identifiants seulement** (jamais un nom de fichier ni un titre, E14).
Consommés : `structure.*` (membres, cours fermés), `personnels.depart`, `eleve.sortie`, `eleve.majorite`, `administratif.lien_*`, `autorisations.retiree` (18), `fichiers.analyse` (7), `absences_personnels.suppleance_*` (15), `continuite.activee` (33).

---

## 22. Notifications (natures déclarées au socle 8) [R]

| Nature | Destinataires | Urgence | Remarque |
|---|---|---|---|
| `espaces.invitation` | nouveau membre ajouté, nouveau partage | U1 | regroupées ; pas pour les membres calculés (sinon 30 avis à la rentrée) |
| `espaces.casier_remis` | élève | U1 | résumé quotidien possible ; plage calme |
| `espaces.casier_a_montrer` | titulaires | U1 | résumé hebdomadaire par défaut |
| `espaces.main_demandee` | détenteur du verrou | U1 (temps réel dans l'éditeur) | — |
| `espaces.signalement` | animateur, CPE | U2 | — |
| `espaces.contenu_sensible` | auteur, animateur | U1 | R-ET18 |
| `espaces.quota_alerte` | personne ou animateurs | U1 | 80 %, 95 % |
| `espaces.depart_transfert` | personnel partant, administrateur | U1 | J-30 avant la fin du lien |
| `portfolio.proposition` | élève | U1 | — |
| `espaces.export_pret` | demandeur | U1 | lien au coffre |

Titres **neutres** (« Un document vous a été remis »), jamais le nom du fichier dans l'avis (E14, contenu des notifications du 16).

---

## 23. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`, Leptos 0.8)

1. **Mes espaces** (`/web/espaces`) : tuiles par espace (couleur, icône, activité récente, éléments nouveaux), filtre par année, « Mon espace », « Partagés avec moi », « Casier ».
2. **Espace** : arborescence à gauche, liste dense à droite (nom, auteur, modifié, taille, état d'analyse), épinglés en tête, barre de dépôt, aperçu en panneau ; onglet « Membres », onglet « Activité », onglet « Signalements » pour l'animateur.
3. **Éditeur** : plein écran, bandeau ScolIA (nom du document, état « enregistré il y a 1 min », verrou, « Demander la main », versions) au-dessus de l'iframe.
4. **Versions** : chronologie, comparer, restaurer, nommer.
5. **Distribution** : choix des élèves (par cours, par groupe), dépôt en lot, tableau de rapprochement (fichier → élève), aperçu, confirmation.
6. **Casier de l'élève** et **vue famille** : liste chronologique simple.
7. **Portfolio** : grille de productions, filtres par matière et année, réglage de visibilité par élément, export.
8. **Administration** : volumes (tuiles Bento), quotas, départs à traiter, lots d'effacement.
9. **Mobile** (D-003) : « Mes espaces » et « Casier » en lecture, aperçu, dépôt de photo ou de fichier, commentaire ; pas d'éditeur (§11.2 point 8).

---

## 24. Modèle de données cible (brique `espaces`) [R]

Toutes les tables : `etablissement_id UUID NOT NULL` (E1), clé primaire UUID applicative (E3), FK composites `(etablissement_id, …)` (E2), `cree_le` / `modifie_le` (E6), RLS standard (E13), **aucune colonne JSON, aucun tableau, aucun `BYTEA`** (E5, E10, D-036). Formes indicatives.

```sql
-- espaces et membres
ent_espace (id, etablissement_id, type CHECK (personnel|cours|groupe|classe|equipe|service|instance|delegues|projet|libre_personnels),
            annee_id NULL, parent_espace_id NULL, libelle, description NULL, couleur_code, icone_code,
            etat CHECK (propose|ouvert|archive|efface), ouvert_eleves_le DATE NULL, archive_le NULL, efface_le NULL,
            gel_contentieux_jusqu_au NULL, visibilite_arrivant CHECK (depuis_rentree|depuis_arrivee),
            contributions_eleves_ouvertes BOOL, commentaires_ouverts BOOL, ia_eleves CHECK (fermee|explication|redaction),
            quota_octets BIGINT, cree_par_personne_id)
ent_espace_ancrage (espace_id, etablissement_id, type CHECK (cours|groupe|classe|equipe|service|instance|delegues|sortie|personne),
                    cours_id NULL, groupe_id NULL, classe_id NULL, equipe_id NULL, service_id NULL, instance_id NULL,
                    sortie_id NULL, personne_id NULL,
                    CHECK (exactement une référence non nulle correspondant au type))
ent_espace_membre_ajoute (id, etablissement_id, espace_id, personne_id, role CHECK (animateur|contributeur|lecteur),
                          debut DATE, fin DATE NULL, motif_code, ajoute_par)
ent_espace_membre_exclu (id, etablissement_id, espace_id, personne_id, debut, fin NULL, motif_code, exclu_par)
ent_espace_role_calcule (espace_id, etablissement_id, source CHECK (enseignant|eleve|membre_equipe|membre_instance|pp|cpe|aesh),
                         role CHECK (animateur|contributeur|lecteur))       -- règle par type, non une liste de personnes
ent_espace_gel_eleve (id, etablissement_id, espace_id, eleve_id, debut, fin NULL, motif_code, par)
-- vue ent_espace_membre_effectif(espace_id, personne_id, role, origine) : calculés (ports 5/6) − exclus + ajoutés

-- arborescence, éléments, versions
ent_espace_dossier (id, etablissement_id, espace_id, parent_id NULL, nom, regle CHECK (aucune|lecture_seule_eleves|depot_sans_lecture|animateurs_seuls),
                    supprime_le NULL, UNIQUE (etablissement_id, espace_id, parent_id, nom_normalise) WHERE supprime_le IS NULL)
ent_espace_element (id, etablissement_id, espace_id, dossier_id NULL, nature CHECK (fichier|document|lien|page|ressource_gar|notice_cdi),
                    nom, nom_normalise, auteur_personne_id, version_courante_id NULL, epingle_rang SMALLINT NULL,
                    licence CHECK (libre|sous_licence|tiers), consultation_seule BOOL,
                    etat CHECK (analyse|disponible|quarantaine|masque|corbeille|efface), corbeille_le NULL,
                    lien_url TEXT NULL, gar_ressource_ref TEXT NULL, notice_ref TEXT NULL)
ent_espace_version (id, etablissement_id, element_id, numero INT, objet_id (→ ent_objet), auteur_personne_id,
                    commentaire NULL, nom_version NULL, type_reel, taille, origine CHECK (depot|editeur|conversion|restauration|brouillon_session),
                    aide_ia BOOL, figee_par CHECK (aucun|remise_20|emission_7|message_16|seance_33|portfolio) DEFAULT aucun,
                    UNIQUE (etablissement_id, element_id, numero))
ent_espace_version_figee (version_id, etablissement_id, socle_code, reference_id, jusqu_au DATE NULL)  -- plusieurs figeages possibles
ent_espace_derive (id, etablissement_id, version_id, sorte CHECK (apercu_pdf|vignette|texte), objet_id, moteur_code, moteur_version)  -- reconstructible
ent_espace_page_bloc (id, etablissement_id, element_id, version_id, rang, sorte CHECK (titre|paragraphe|liste|lien|image|citation), texte, cible_element_id NULL)

-- bureautique
ent_espace_verrou (element_id PRIMARY KEY, etablissement_id, detenteur_personne_id, session_edition_id, pose_le, expire_le, version_base_id)
ent_espace_verrou_journal (id, etablissement_id, element_id, personne_id, evenement CHECK (pose|renouvele|libere|expire|repris|main_demandee), le)
ent_bureautique_version_app (id, application CHECK (wordcraft|gridcraft|deckcraft), version_texte, sha256_archive, en_service_le, retiree_le NULL)
   -- table globale de référence (E12 : globale_reference), sans donnée d'établissement
ent_espace_modele (element_id, etablissement_id, categorie_code)   -- modèles de l'établissement

-- partages, liens externes
ent_espace_partage (id, etablissement_id, element_id NULL, dossier_id NULL, destinataire_personne_id, droit CHECK (lecture|ecriture),
                    debut, fin NULL, par, retire_le NULL, CHECK (element_id IS NOT NULL OR dossier_id IS NOT NULL))
ent_espace_lien_externe (id, etablissement_id, element_id, adresse_empreinte, empreinte_jeton, cree_par, expire_le, usages_max, usages, revoque_le NULL)

-- collectif
ent_espace_commentaire (id, etablissement_id, element_id, parent_id NULL, auteur_personne_id, texte, etat CHECK (visible|masque|retire_auteur), masque_motif_code NULL, masque_par NULL)
ent_espace_signalement (id, etablissement_id, espace_id, cible_type CHECK (element|version|commentaire|dossier), cible_id, auteur_personne_id,
                        motif_code, texte NULL, etat CHECK (ouvert|classe|traite|transmis), traite_par NULL, decision_code NULL)
ent_espace_alerte_mot (id, etablissement_id, cible_type, cible_id, liste_code, le, vue_par NULL)

-- casier, distributions
ent_casier_distribution (id, etablissement_id, auteur_personne_id, consigne NULL, a_montrer_famille BOOL, disponible_le, mode CHECK (identique|fichiers|publipostage))
ent_casier_remise (id, etablissement_id, distribution_id, eleve_id, version_id, ouvert_le NULL, retire_le NULL, retrait_motif_code NULL)
ent_casier_publipostage_champ (distribution_id, etablissement_id, champ_code CHECK (nom|prenom|classe|cours|date))

-- portfolio
ent_portfolio_element (id, etablissement_id, eleve_id, titre, date_production, matiere_id NULL, projet_libelle NULL, reflexion NULL,
                       version_id, origine CHECK (versement|proposition|depot), retire_le NULL)
ent_portfolio_visibilite (portfolio_element_id, etablissement_id, cible CHECK (mes_professeurs|famille))   -- absence = moi seul
ent_portfolio_coauteur (portfolio_element_id, etablissement_id, personne_id)
ent_portfolio_proposition (id, etablissement_id, eleve_id, version_id, propose_par, message NULL, etat CHECK (ouverte|acceptee|refusee|expiree))

-- quotas, exports, effacements
ent_espace_quota_profil (etablissement_id, profil CHECK (personnel|eleve|espace_cours|espace_equipe|espace_service|espace_libre), octets BIGINT)
ent_espace_occupation (espace_id, etablissement_id, octets_confirmes BIGINT, octets_reserves BIGINT, modifie_le)   -- compteur verrouillé (SELECT … FOR UPDATE)
ent_espace_reservation (id, etablissement_id, espace_id, octets, expire_le, confirmee_le NULL)
ent_espace_export (id, etablissement_id, demandeur_personne_id, sujet CHECK (espace_personnel|portfolio|espace), sujet_id, document_id NULL, etat)
ent_espace_lot_effacement (id, etablissement_id, regle_code, annee_id NULL, initie_par, approuve_par NULL, execute_le NULL, nb_elements)
ent_espace_journal (id, etablissement_id, espace_id NULL, element_id NULL, acteur_personne_id, action_code, le, detail_code NULL)   -- ajout seul
```

**Registre des tables (E12)** : toutes `etablissement` / export `simple`, sauf `ent_espace_derive` (« reconstructible », régénérée après restauration), `ent_espace_verrou` et `ent_espace_reservation` (« volatile », vidées à la restauration, E9), `ent_bureautique_version_app` (`globale_reference`), `ent_portfolio_*` (export **particulier** : remis à l'élève). File d'effets de bord nouvelle : tâches `ent-convert` (« à neutraliser puis rejouer à la restauration », E9).

---

## 25. Pilotage (socle 47)
Indicateurs agrégés sans contenu : espaces ouverts par cours (%), activité hebdomadaire par type, volume et croissance, distributions par mois, signalements par mois, délai de traitement des signalements, part des documents édités en ligne ; jamais de classement de professeurs ni d'élèves.

---

## 26. Plan de reprise des données de Next.js (M19) (D-035 point 4)

1. **Export fourni par Florian** (jamais lu par le pilote en base) : copie du préfixe `documents/` du seau de l'établissement (`documents/users/<id>/`, `documents/shared/<id>/`) avec liste des objets (taille, empreinte) et export des collections `documents__shares`, `documents__file-shares`, `documents__users__*` (`office-recents`) **décodées puis ventilées** en tables (D-036 : aucune collection ne passe en JSON).
2. **Correspondances** : utilisateur Next.js → personne ScolIA (rapprochement de `TRANSITION_T0_INVENTAIRE.md` §6) ; `documents/users/<id>/<chemin>` → espace `personnel` de la personne, dossiers recréés, un élément par fichier, version 1 = nouvel `ent_objet` (copie vers une clé opaque `{etab}/espaces/{uuid}`, empreinte recalculée) ; `documents/shared/<id>` → espace `libre_personnels` avec ses membres ; `file-shares` → `ent_espace_partage` sur l'élément (plus de pointeur de chemin) ; récents → `ent_espace_element.modifie_le` (pas de table de récents).
3. **Analyse** : chaque fichier repris passe par le noyau (antivirus, type réel) ; les refus (types actifs, macros) sont **conservés à part** et listés à leur propriétaire (« ces 12 fichiers n'ont pas été repris ; téléchargez-les avant le … »), jamais perdus en silence.
4. **Proposition avant activation** : chaque personnel voit « Votre espace a été repris : 1 243 fichiers, 3 partages ; vérifiez » ; les partages repris sont actifs mais signalés.
5. **Bascule** : gel en écriture de M19 dans Next.js (geste de Florian), reprise différentielle des dépôts faits entre l'export initial et le gel ; comparaison des comptes (fichiers, octets, partages) par personne ; **retour arrière** : Next.js reste lisible tant que Florian ne l'arrête pas.
6. **Données réelles** : seulement après l'accord écrit de l'établissement et l'avis du DPO (D-035 point 4) ; essais sur un export **anonymisé** (fichiers de remplacement de même taille).

---

## 27. Lots de code (équipe Sonnet) et migrations

### 27.1 Tableau des lots

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **ET0** préalables du noyau et inventaire | usage de dépôt `espace` dans `ent-object` (liste blanche §12.1, plafonds), URL d'envoi à **taille signée**, réservation atomique de quota, réconciliation taille/empreinte, rattachement à l'état d'analyse du 7 (ou adaptateur « analyse non disponible = fermé ») ; inventaire de reprise M19 sur export anonymisé | `ent-object`, 500 ; socle 7 (noyau) | tests : envoi de 70 Mo signé 1 Mo refusé ; 20 envois parallèles ne dépassent pas le quota ; objet non analysé invisible |
| **ET1** espace personnel et espaces libres (parité M19) | espaces `personnel` et `libre_personnels`, dossiers, éléments, versions, corbeille, partages nominatifs, quota, téléchargement, aperçu minimal, avis d'invitation, export | ET0 ; 6 (annuaire), 8 | parité NX32-01 à NX32-20 vérifiée action par action ; ND32-01 à ND32-18 impossibles (tests) |
| **ET2** espaces de cours | proposition à la rentrée, ancrages, membres calculés, ajouts, exclusions, archivage, reconduction | ET1 ; 5, 10 | CET-01 à CET-05 |
| **ET3** autres espaces et groupes | classe, équipe, service, instance, délégués, projet, groupes, règles de dossier | ET2 ; 6, 46 | CET-10, CET-12 |
| **ET4** éléments riches | liens, GAR, CDI, pages, épingles, licence, recherche plein texte, aperçus | ET1 ; `ent-convert` (ET5a) ; 9, 45 | R-ET19 testée |
| **ET5** bureautique | (a) `ent-convert` (processus isolé, moteurs ArtCraft serveur, LibreOffice en repli) ; (b) éditeur : version figée sans marque, iframe même origine, protocole hôte, verrou, brouillon de session, conversion de formats, modèles ; audit RGAA | ET1 | CET-25 à CET-29 ; R-ET27 à R-ET31 ; jeu de 50 documents de référence ouverts et enregistrés sans perte |
| **ET6** collectif et modération | commentaires, masquage, gel, signalements, alerte de mots, plage calme | ET2 ; 13, 16 (rappels) | CET-34 à CET-39 |
| **ET7** casier, distributions, liens externes | remises, publipostage, rapprochement, vue famille, liens externes | ET2 ; 4, 7 (coffre) | CET-30 à CET-33 |
| **ET8** branchements | émission (7), pièce de message (16), remise et cahier (20), support de visio (33), comparaison de versions | ET5 ; 7, 16, 20, 33 | une version figée n'est jamais élaguée (test) |
| **ET9** administration | volumes, quotas, alertes, départs et transferts, exports, rangement assisté | ET1 ; 6 | CET-46, CET-47 |
| **ET10** portfolio et publication | portfolio, propositions, visibilités, export, citations (22), publication (17, 18) | ET2 ; 18, 22 | R-ET34 à R-ET37 |
| **ET11** conservation | archivages et effacements programmés, gel de contentieux, lots validés | ET2 ; 27 | §20.3 appliqué sur le labo |
| **ET12** IA | capacités §16.3 | ET4 ; `ent-ia` | test d'injection par document ; aucun contenu sensible dans un contexte |
| **ET13** mobile | lecture, aperçu, dépôt, casier | ET7 ; 11 | — |
| **ET14** reprise M19 et bascule | §26 | ET1, ET9 | reprise d'un export anonymisé sans perte ; comparaison par personne |

**Parcours minimal** : ET0, ET1, ET14 pour remplacer M19 (usage réel, D-035) ; puis ET2, ET5, ET7 pour la V1 « espaces de cours et bureautique » de février 2027 (D-034).

### 27.2 Migrations (plage imposée 2640–2659, registre `MIGRATIONS.md`)

| N° | Lot | Contenu |
|---|---|---|
| 2640 | ET0 | réservations et occupations de quota, profils de quota |
| 2641 | ET1 | `ent_espace`, ancrage, membres ajoutés et exclus, rôles calculés, vue des membres effectifs |
| 2642 | ET1 | dossiers, éléments, versions, figeages, dérivés |
| 2643 | ET1 | partages nominatifs, journal en ajout seul |
| 2644 | ET2 | gel d'élève, réglages d'espace, archivage |
| 2645 | ET4 | blocs de page, liens, références GAR et CDI, index de recherche (table de texte extrait, sans JSON) |
| 2646 | ET5 | verrous, journal des verrous, versions d'applications, modèles |
| 2647 | ET6 | commentaires, signalements, alertes de mots |
| 2648 | ET7 | distributions, remises, champs de publipostage, liens externes |
| 2649 | ET9 | exports, transferts au départ |
| 2650 | ET10 | portfolio, visibilités, coauteurs, propositions |
| 2651 | ET11 | lots d'effacement, gels de contentieux |
| 2652 | ET14 | zone de reprise M19 (colonnes texte, chiffrée, détruite 60 jours après la bascule) |
| 2653–2656 | réserve | ajouts personas (§30) : 2653 suivi des groupes et parties, 2654 continuité (dossiers de continuité), 2655 accessibilité (préférences d'éditeur), 2656 réserve |
| 2657 | — | réserve |
| 2658 | ET11 | RLS stricte et validation des FK composites |
| 2659 | — | retraits (réservé) |

Exceptions D-036 : **aucune**. Contrôle de revue : `grep -i "json\|bytea\|\[\]"` sur 2640–2659.

---

## 28. Décisions appliquées par défaut (D-031) et questions à Florian

### 28.1 Appliquées par défaut (D-031)
1. Espaces **ancrés sur la structure** avec membres calculés (§1, §8).
2. **Versions immuables** pour tout élément, verrou d'édition exclusif faute de coédition (§11.2).
3. **Pas d'espace d'élèves sans adulte**, pas de partage entre élèves mineurs, pas de modération automatique (§9).
4. Parents sans accès aux espaces de travail ; vue famille par le casier « à montrer » et le portfolio (R-ET33, R-ET34).
5. Portfolio **propriété de l'élève**, jamais public (§14).
6. Aucun lien public ; liens d'accès de 60 s (R-ET9).
7. Quotas : **2 Gio** par personnel (reprise de M19), **1 Gio** par élève, **10 Gio** par espace de cours, **20 Gio** par espace d'équipe ou de service, plafond par fichier **64 Mio** (plafond actuel d'`ent-object`), **vidéo 500 Mio** dans les espaces de personnels seulement (S-3201, S-3202, S-3214) — soumis à Florian pour le coût (§28.2).
8. Création de documents vierges en **OOXML** ; ODF ouvert et converti (§11.2 point 10).
9. Moteurs ArtCraft côté serveur pour les aperçus, LibreOffice en repli ; pandoc exclu (§12.2).
10. Conservation proposée au §20.3, effective seulement après l'avis du DPO.
11. Aucun raccordement au « Nuage » national par défaut (§5.2).
12. Reprise M19 proposée à chaque propriétaire avant activation, refus conservés à part (§26).

### 28.2 Soumis à Florian
| N° | Question | Recommandation |
|---|---|---|
| Q-ET1 | **Mettre à jour D-006** : versions v0.17 / v0.18 alpha (et non v0.4), dépôts exacts, protocole `craft-web-host` en iframe de même origine, moteurs Rust côté serveur pour les aperçus | Oui ; figer Wordcraft 0.17.0, GridCraft 0.18.0, DeckCraft 0.17.0 au lot ET5 et refaire la recette à chaque mise à jour |
| Q-ET2 | **Coédition en temps réel** (absente d'ArtCraft) : attendre, contribuer en amont, ou s'en passer en V1 | S'en passer en V1 (verrou, parties, versions) ; observer le projet ; ne pas adopter une autre suite sans votre accord |
| Q-ET3 | **Accessibilité** d'un éditeur rendu en canvas | Audit RGAA au lot ET5 ; alternative accessible garantie (R-ET30) ; si l'audit échoue, l'éditeur reste un « plus » et non le seul moyen |
| Q-ET4 | Build ScolIA d'ArtCraft sans marques et sans `craft-web-api` | Oui si l'option existe ; sinon CSP et isolement (R-ET29) |
| Q-ET5 | Édition sur tablette (applis mobiles) | Non en V1 |
| Q-ET6 | Accepter que des élèves « discutent » dans un document partagé (conséquence du « pas de messagerie entre élèves ») | Oui, avec historique visible de l'animateur et signalement |
| Q-ET7 | Durées de conservation (§20.3) | Avis du DPO |
| Q-ET8 | **Usage réel de M19** aujourd'hui, volume, date de bascule | Fait que seul Florian connaît ; bascule proposée aux vacances d'hiver 2027 après ET1 et ET14 |
| Q-ET9 | Raccordement au « Nuage » national pour les enseignants | Non par défaut |
| Q-ET10 | Accords sectoriels en vigueur (exception pédagogique) : à vérifier auprès du CFC | Vérification juridique avant la V1 |
| Q-ET11 | Export vers Avenir(s) | Pas d'interface connue ; à revoir avec le socle 37 |
| Q-ET12 | Coût du stockage (quotas §28.1 point 7) | Estimation à faire sur l'offre Scaleway retenue, avec les deux destinations de sauvegarde (D-040) |

---

## 29. Faits, hypothèses, contradictions

- **Faits** : aucun code Rust du domaine ; `ent_objet`, `upload.rs`, `presign.rs` présents ; M19 inventorié (22 actions, 20 défauts) ; « Office » vide ; ArtCraft vérifié sur ses dépôts le 2026-11-11 (versions, licence, protocole, absence de coédition) ; exception pédagogique au 12° de L122-5 et L122-5-4 ; Folios fermé.
- **Hypothèses** : M19 réellement utilisé (« Partiel [H] » dans T0) ; accessibilité d'egui insuffisante ; une version ArtCraft tous les 2–3 jours impose une recette à chaque mise à jour ; les chiffres d'activité des dépôts ont été lus par résumé de page et sont indicatifs.
- **Contradictions relevées** : (1) **D-006** décrit ArtCraft en « pré-alpha v0.4 » ; les dépôts sont en v0.17–v0.18 alpha (Q-ET1) ; (2) D-006 parle d'« iframe » sans préciser : le protocole exige la **même origine**, ce qui réduit l'isolement d'une iframe ; (3) Next.js créait des documents **ODF** alors qu'ArtCraft est centré sur **OOXML** (DeckCraft ne mentionne pas ODP, GridCraft lit ODS en valeurs seules) ; (4) `07_DOCUMENTS.md` §11.4 refuse les documents bureautiques au dépôt famille, alors qu'`ent-object` les accepte pour l'usage `Document` [F `upload.rs:175-193`] : à trancher par le socle 7, sans effet sur le 32 (les familles ne déposent pas dans les espaces).

---

## 30. Ajouts personas (`PERSONAS_32.md`)

Voir la passe pour le détail des personas et des manques (M-3201 à M-3234). Ajouts versés ici ; ils l'emportent sur les sections précédentes en cas de contradiction.

### 30.1 Bloquants tranchés (appliqués par défaut, D-031)
- **A. Le devoir à rendre et le document de groupe se confondent** (M-3201) : une production de groupe faite dans un espace de groupe se **rend** au socle 20 par **un** dépôt au nom du groupe (`element_fige` + liste des membres du groupe à la date), et non par le partage du document au professeur ; le 20 enregistre une remise par élève pointant la même version (R-ET39).
- **B. Le professeur supprime un dossier contenant des productions d'élèves** (M-3205) : la mise à la corbeille d'un dossier par l'animateur **exclut** les éléments d'élèves non figés : ils sont déplacés dans un dossier « Retrouvés » de l'espace, visibles de leurs auteurs (R-ET40).
- **C. Attaque par document piégé visant l'éditeur ou le moteur de conversion** (M-3218) : `ent-convert` tourne sous **seccomp / espace de noms sans réseau** dans un conteneur séparé de l'API ; l'iframe de l'éditeur reçoit en plus l'attribut `sandbox="allow-scripts allow-same-origin allow-downloads"` (sans `allow-top-navigation`, `allow-popups`, `allow-forms`) (R-ET41) ; un fichier qui fait échouer trois conversions est marqué « conversion impossible » et n'est plus retenté.
- **D. Le DPO ne peut pas répondre à une demande d'accès d'un élève** (M-3224) : action ETR-280 « rassembler les données d'une personne » (contributions, casier, portfolio, espace personnel, journal la concernant), en lot, pour la procédure du socle 27.

### 30.2 Actions ajoutées (ETR-268 à ETR-299)

| N° | Action | Acteur · droit | Règles | Lot |
|---|---|---|---|---|
| ETR-268 | Rendre une production de groupe au socle 20 au nom du groupe | ELV membre du groupe | R-ET39 ; les membres voient qui a rendu | ET8 |
| ETR-269 | Découper un document de groupe en parties attribuées, puis assembler | ANI | parties = éléments liés ; assemblage = nouvelle version d'un document maître | ET5 |
| ETR-270 | Voir « qui a contribué à quoi » dans un document de groupe (versions par auteur) | ANI, membres du groupe | — | ET5 |
| ETR-271 | Préparer à l'avance le contenu d'un espace avant l'ouverture aux élèves (brouillon d'espace) | ANI | invisible des élèves avant `ouvert_eleves_le` | ET2 |
| ETR-272 | Programmer la visibilité d'un élément (date et heure) | ANI | ex. corrigé visible après le devoir | ET4 |
| ETR-273 | Masquer temporairement un élément aux élèves | ANI | — | ET4 |
| ETR-274 | Dossier « Retrouvés » des productions d'élèves après suppression d'un dossier par l'animateur | système | R-ET40 | ET1 |
| ETR-275 | Voir l'espace « comme un élève » (aperçu des droits) | ANI | sans connexion en tant que | ET2 |
| ETR-276 | Signaler un fichier qui n'a pas pu être converti, demander une nouvelle tentative | auteur | trois échecs au plus | ET5 |
| ETR-277 | Choisir ses préférences d'éditeur (taille d'interface, contraste, ouvrir par défaut en téléchargement) | tous | accessibilité (R-ET30) | ET5 |
| ETR-278 | Demander une version adaptée d'un document (gros caractères, police adaptée) au professeur ou à l'AESH | ELV avec pastille d'aménagement (35) | remise au casier par l'AESH (R-ET14) | ET7 |
| ETR-279 | Déposer un élément depuis le téléphone par photo (plusieurs pages → un PDF) | ELV, PER | conversion par l'appli ou le noyau | ET13 |
| ETR-280 | Rassembler les données d'une personne pour une demande d'accès (socle 27) | DPO · `espaces.journal.lire` + procédure 27 | export figé, sans contributions d'autrui | ET11 |
| ETR-281 | Geler un espace pour contentieux, lever le gel | DIR | CET-48 | ET11 |
| ETR-282 | Voir les espaces dormants (aucune activité depuis 12 mois) | ADM | revue annuelle | ET9 |
| ETR-283 | Fusionner deux espaces (cours fusionnés) | ANI des deux | éléments déplacés, historique conservé | ET3 |
| ETR-284 | Créer un dossier de continuité dans chaque espace de cours lors de l'activation d'un plan (socle 33) | système, à la demande de la direction | dossier daté, épinglé | ET8 |
| ETR-285 | Voir les élèves qui n'ont ouvert aucun élément d'un espace depuis N jours (continuité) | ANI, PP, CPE (si plan actif) | nombres et noms pour l'animateur ; aucun profilage | ET8 |
| ETR-286 | Consulter un document en mode « présentation » (diaporama plein écran) depuis l'aperçu | lecteur | — | ET5 |
| ETR-287 | Imprimer un élément (PDF prêt à imprimer) | lecteur | sauf licence (imprimer une ressource sous licence : CFC, papier) | ET4 |
| ETR-288 | Ajouter un texte alternatif à une image | auteur | proposition IA (N1) facultative | ET4 |
| ETR-289 | Créer un espace de projet lié à une sortie (socle 42) | organisateur | ancrage `sortie` ; membres = participants et encadrants | ET3 |
| ETR-290 | Désigner un repreneur pour son espace personnel avant un départ (éléments choisis) | PER | — | ET9 |
| ETR-291 | Copier l'espace d'un collègue parti, avec son accord écrit à l'export | ADM | — | ET9 |
| ETR-292 | Mettre à jour la version d'ArtCraft après recette sur le jeu de référence | exploitation ScolIA | journal des versions ; retour possible | ET5 |
| ETR-293 | Revenir à la version précédente d'ArtCraft | exploitation | — | ET5 |
| ETR-294 | Voir les versions d'applications en service et leurs notes | ADM | — | ET5 |
| ETR-295 | Bloquer l'ouverture d'un document par l'éditeur (le rendre téléchargeable seulement) | ANI | document lourd ou piégé | ET5 |
| ETR-296 | Recevoir un récapitulatif hebdomadaire de l'activité de ses espaces (animateur) | ANI | facultatif | ET9 |
| ETR-297 | Restituer à un élève une contribution masquée par erreur | ANI | — | ET6 |
| ETR-298 | Transmettre un signalement au 13 (fait de discipline proposé) | CPE | proposition, jamais sanction | ET6 |
| ETR-299 | Voir la liste des ressources « sous licence » de l'établissement (contrôle droit d'auteur) | DIR, documentaliste | — | ET4 |

### 30.3 Règles ajoutées
| N° | Règle |
|---|---|
| R-ET39 | Une production de groupe se rend au 20 par un dépôt au nom du groupe à la date ; une remise par élève, même version. |
| R-ET40 | Supprimer un dossier n'emporte jamais les productions d'élèves non figées : elles vont dans « Retrouvés ». |
| R-ET41 | `ent-convert` s'exécute dans un conteneur séparé, sans réseau, sous filtre d'appels système ; l'iframe de l'éditeur est en `sandbox` restreint. |
| R-ET42 | Le contenu préparé avant l'ouverture ou programmé n'est visible des élèves qu'à la date. |
| R-ET43 | L'aperçu « comme un élève » ne donne aucun droit et ne laisse aucune trace d'ouverture au nom de l'élève. |
| R-ET44 | Les indicateurs « élèves sans activité » ne servent qu'à l'animateur et, plan de continuité actif, au PP et au CPE ; aucun classement ni score. |

### 30.4 Cas limites ajoutés (CET-53 à CET-72)
| N° | Situation | Réponse |
|---|---|---|
| CET-53 | Groupe de 4 où un élève quitte le groupe la veille du rendu | rendu au nom des membres à la date du dépôt ; l'élève parti garde ses versions attribuées |
| CET-54 | Élève qui rend pour le groupe sans prévenir | les membres voient « rendu par Léa le … » ; le 20 permet une nouvelle remise avant l'échéance |
| CET-55 | Corrigé programmé à 18 h, devoir décalé | l'animateur reprogramme ; aucune ouverture anticipée possible |
| CET-56 | Professeur qui prépare son espace en août avec un mauvais cours | déplacement d'ancrage proposé avant ouverture |
| CET-57 | Document de 900 pages ouvert dans Wordcraft sur un vieil ordinateur | avertissement de poids ; ouverture en aperçu par défaut (ETR-295) |
| CET-58 | Appareil de l'élève sans WebGPU / WebGL | éditeur indisponible annoncé ; aperçu et dépôt hors ligne |
| CET-59 | Lecteur d'écran sur l'éditeur | alternative proposée d'emblée (préférence ETR-277) |
| CET-60 | Fichier qui fait échouer la conversion trois fois | « conversion impossible », téléchargement seul, alerte d'exploitation |
| CET-61 | Élève qui verse au portfolio une copie corrigée avec la note | permis (c'est sa production) ; la note visible seulement selon la visibilité choisie |
| CET-62 | Parent qui demande qu'on retire une photo de son enfant d'un espace de classe | animateur saisi par message (16) ; masquage ; c'est un retrait d'accord si une publication existait (18) |
| CET-63 | Espace de projet d'une sortie annulée | archivé avec la sortie |
| CET-64 | Professeur en congé long qui ne libère pas un verrou | expiration ; reprise par le co-animateur |
| CET-65 | Personnel qui part et refuse le transfert de ses éléments d'équipe | les éléments d'équipe appartiennent à l'espace d'équipe (déposés dans un espace collectif) : ils restent ; seul l'espace personnel lui est exporté |
| CET-66 | Deux établissements du même organisme qui veulent un espace commun | impossible (D-013) ; partage par lien externe nominatif d'un élément |
| CET-67 | Attaquant qui dépose un DOCX avec relations externes (images distantes, modèles distants) | relations externes supprimées par l'hôte avant l'ouverture dans l'éditeur et par `ent-convert` ; l'éditeur n'a pas de réseau |
| CET-68 | Attaquant qui envoie un ZIP renommé en DOCX contenant 10 Go décompressés | lecture bornée du noyau (20 000 entrées) ; `ent-convert` borne la taille décompressée ; rejet |
| CET-69 | Élève qui tente d'ouvrir l'URL de l'éditeur avec l'identifiant d'un autre document | l'éditeur ne lit rien par URL : l'hôte n'envoie que le document autorisé |
| CET-70 | Script injecté dans un nom de fichier | échappé partout ; noms jamais interprétés |
| CET-71 | Grande quantité de petits dépôts pour saturer la file de conversion | quota de tâches par personne et par heure ; file prioritaire pour les personnels |
| CET-72 | Fuite d'un lien externe | usage limité, 7 jours, adresse vérifiée ; révocation ; journal |

### 30.5 Lots révisés
- **ET0** inclut le **conteneur isolé** de `ent-convert` (R-ET41), pas seulement le noyau.
- **ET8** inclut ETR-268 (rendu de groupe) et ETR-284/285 (continuité, coordonnés avec le lot VI8 du socle 33).
- **ET11** inclut ETR-280 (demande d'accès).
- Migrations : 2653 (parties de groupe, programmations de visibilité), 2654 (dossiers de continuité), 2655 (préférences d'éditeur, blocages d'ouverture).
