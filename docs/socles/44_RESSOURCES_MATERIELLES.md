# Socle 44 — Ressources matérielles, prêts et maintenance

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code de ce socle. Aucun code écrit ni modifié. Aucun accès à une base, à un secret ni à un environnement de production. Le code Next.js a été **lu seulement** (lecture ciblée dans le dépôt, rien exécuté, rien construit, aucun `.env` lu).
**Branche :** `ent-socle-43-44-conception`, partie de `origin/ent-mc0hoe` (`afeb8c4`).
**Passe personas :** `docs/socles/PERSONAS_44.md` (même branche). Ses ajouts sont faits **ici**, marqués **« ajout personas »**, et regroupés au **§30**. **En cas de contradiction, le §30 l'emporte** ; rien n'a été supprimé.
**Socle jumeau :** `docs/socles/43_PERISCOLAIRE_TRANSPORTS.md` (matériel de l'AS, casiers et clés du foyer, salles des activités).
**Frontière décisive avec le socle 19** : la conception du 19 (Q-AG1, appliquée par défaut, D-031) porte **le moteur de réservation de lieux** (salles et lieux du 5) et la **parité du module Next.js de réservation de salles** (M02, NX-01 à NX-19, lot AG3) ; elle a aussi pris le **planning par domaines** (M03, lot AG6b, Q-AG2). Le 44 **ne le refait pas** : il garde le **matériel**, les **prêts**, les **manuels**, l'**inventaire**, la **maintenance**, les **clés**, la **reprographie** et la **mise en place** demandée par une réservation, et il **réutilise le moteur du 19** pour réserver du matériel (« ressource générique »). Les défauts de M02 que ce socle a trouvés en plus de ceux du 19 sont relayés au §2.3 et ajoutés au 19 par le §30.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017, D-020, D-021, D-022, D-023, D-026, D-027, D-028, D-031, **D-034**, **D-035**, **D-036**, D-037 à D-041 ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (M02, M03, M04 demandes, M08 photocopies) ; `docs/SOCLES.md` §44 et §10 (T-01, T-02 : socle N1 « Demandes ») ; `docs/MATRICE_CHARLEMAGNE.md` (VOY-12, VOY-13, DEM-04) ; conceptions `03_INSCRIPTIONS.md` (prêts et manuels à solder au départ, l. 376 et 901), `05_STRUCTURE.md` (salles typées, équipements, `reservable` « lu par le socle 44 », disponibilités, port `structure_disponibilite::disponible`, l. 1155 « contrat à écrire au socle 44 »), `06_PERSONNELS.md` (clés et badge remis au départ l. 643, prêt de clé de sécurité PRS-119 « ou socle 44 »), `07_DOCUMENTS.md` (conventions, signatures), `08_NOTIFICATIONS.md`, `10_EMPLOI_DU_TEMPS.md` (ports `edt::occupe`, `edt::plages_libres` ; ressource = lieu ou équipement l. 285), `14_ACCUEIL_SURETE.md` (PPMS, alerte), `15_ABSENCES_PERSONNELS.md`, `19_AGENDA_RDV.md` (§14, AG3, `ent_agenda_reservation_besoin`), `24_FINANCES.md` (pertes et dégradations facturées, entité émettrice), `25_PAIEMENTS.md`, `29_PORTAILS.md` (« Mes demandes », N1), `34_SANTE.md` (trousses), `41_INTERNAT.md` (INT-188 interventions, INT-227 états des lieux, INT-228 prêts, casiers et clés l. 291), `42_SORTIES_VOYAGES.md` (matériel emporté), `45` (CDI, non conçu : prêts documentaires) et `IA_COEUR.md`.

**Conventions.** **[F]** fait vérifié dans le code (Next.js : chemins relatifs à `app/` ; Rust : chemins depuis `ent-rust/`). **[V]** règle lue sur une source officielle (URL et date au §5). **[S]** source officielle vue seulement en extrait ou par une source officielle secondaire (education.gouv.fr a renvoyé 403). **[NV]** non vérifié. **[H]** hypothèse de terrain. **[R]** recommandation, non décidée.

Numérotation propre à ce socle (préfixes vérifiés libres dans `docs/` le 2026-11-11) :

| Élément | Numéros | Remarque |
|---|---|---|
| Actions existantes de Next.js | **NRM-01**… | §2.2 (photocopies, et renvoi aux NX du 19 pour les salles) |
| Défauts de Next.js | **DRM-01**… | §2.3 |
| Défauts du code Rust existant | **RRM-01**… | §2.5 |
| Actions cibles | **RSM-100**… | (RES- est pris par la restauration) |
| Règles | **R-RM1**… | |
| Cas limites | **CRM-01**… | |
| Questions | **Q-RM1**… | |
| Réglages signalés (D-023) | **S-4400 à S-4449** | (S-449 à trois chiffres est au socle 42 ; aucun S-44xx à quatre chiffres) |
| Manques (passe personas) | **M-4401**… | |

**Plage de migrations imposée : 2880–2899** (inscrite dans `docs/socles/MIGRATIONS.md`). Le lot 0 (RM0) n'a **pas de migration** (correctifs de code du 5 et du 10) ; le premier lot à tables part en **2880**.

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année vue du terrain · §5 Réglementaire · §6 Frontières et branchements · §7 Concepts · §8 Biens et inventaire · §9 Matériel réservable · §10 Prêts · §11 Manuels scolaires · §12 Demandes d'intervention et maintenance · §13 Sécurité des locaux et vérifications · §14 Clés, badges, casiers, états des lieux · §15 Reprographie · §16 Locaux prêtés à des tiers et mise en place · §17 Cas limites · §18 IA · §19 Actions · §20 Règles · §21 Droits et minimisation · §22 Événements · §23 Écrans et mobile · §24 Modèle de données · §25 Lots · §26 Reprise · §27 Tests · §28 Réglages · §29 Questions et décisions · §30 Ajouts personas · §31 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **Ce socle tient le registre de tout ce qui se touche, se prête, se casse et s'ouvre** : un ordinateur, une tablette, un manuel, une calculatrice, un vidéoprojecteur, une clé, un badge, un casier, une chambre d'internat à l'état des lieux, un copieur et ses compteurs. Trois questions reviennent chaque jour : **« où est-il, et qui l'a ? »**, **« est-ce que ça marche, et qui le répare ? »**, **« qu'est-ce que l'élève doit rendre avant de partir ? »**.
2. **Ce qui existe** [F] :
   - **Next.js** : la **réservation de salles** (M02, fiable) est **reprise par le socle 19** ; le **planning par domaines** (M03) aussi (AG6b). Le module **photocopies** (M08, partiel) est le seul module de l'add-on qui revient au 44 : deux circuits (noir et blanc direct, couleur validée par la direction du site), dépôt pour le compte d'un enseignant, file d'impression, « prête », PDF signés. **10 actions** (NRM-01 à NRM-10), **toutes reprises**, et **13 défauts** (DRM-01 à DRM-13) dont 4 graves (file d'impression ouverte à tous les sites, PDF d'un autre rattachable, secret codé en dur, lien public qui modifie). Aucun module de prêt, d'inventaire, de manuels, de clés ni de maintenance (les demandes de maintenance passent par le module « Demandes », M04, qui relève du futur socle N1).
   - **Rust** : salles du 5 (typées, équipements en tableau de textes, `reservable` « lu par le socle 44 », disponibilités, indisponibilités, port `structure_disponibilite::disponible`). **Rien** pour le matériel, les prêts, l'inventaire, les clés ou la maintenance. Le code de salles a **trois défauts** qui touchent directement le 44 et le 19 (RRM-01 à RRM-03) : le changement de salle d'une séance ignore les indisponibilités et n'est pas protégé en concurrence ; deux chemins d'écriture des salles aux règles divergentes ; le port de disponibilité n'est appelé que par les tests. **Lot RM0**.
3. **Thèse** (§1) : un **bien** a une identité stable (étiquette), un **emplacement** et un **détenteur** datés, un **état** et une **vie** (entrée, affectation, prêt, panne, réparation, réforme), tous faits en ajout seul. Un **prêt** est un engagement daté d'une **personne** (élève, personnel), signé quand il le faut, qui se **solde** : rendu, rendu abîmé, perdu, racheté. Une **demande d'intervention** est un fil daté qui va de « ça ne marche pas » à « c'est réparé et vérifié », avec un responsable à chaque instant. La sécurité d'abord : une demande qui touche la sécurité (porte coupe-feu, alarme, fuite, électricité) **n'attend pas dans une file**.
4. **Le réglementaire** (§5) commande plusieurs garde-fous : **manuels gratuits au collège** (l'État, C. éduc. L211-8) et **caution interdite** pour le prêt de manuels, la famille répondant d'une **perte ou dégradation** selon un **barème dégressif** [V réponse ministérielle, circulaires anciennes à reconfirmer] ; **tarifs de remplacement** votés par le **conseil d'administration** (R421-20) ; inventaire et sortie d'inventaire selon la M9-6 (seuil 800 € HT, sortie votée) [S] ; **registre de sécurité** des établissements recevant du public (CCH R143-44, nouvelle version au 01/07/2026) : travaux, personnel de sécurité, consignes, **vérifications**, **exercices** ; registre santé-sécurité au travail et registre des dangers graves et imminents [S] ; **licence CFC** de reprographie (au plus 10 % d'un livre, 30 % d'une revue, 180 pages par élève et par an, déclaration par échantillon) [V notice] ; **mise à disposition des locaux** à des tiers par **convention tripartite** (L213-2-2, L214-6-2). Aucune règle nationale sur les clés.
5. **Ce que ScolIA apporte [H]** :
   - **un inventaire vivant** : chaque bien étiqueté (QR code), rattaché à une salle du 5 ou à une personne, inventaire physique annuel **au téléphone**, écarts listés, réforme tracée, valeur et immobilisation exportées vers la comptabilité (48) ;
   - **les prêts de rentrée en série** : manuels de la classe ou ordinateurs de la région distribués **par code-barres** en quelques minutes par classe, convention signée en ligne par les bons titulaires (7, 18), retour de fin d'année avec l'état, **pertes facturées au bon payeur au barème voté** (24), et **solde automatique au départ de l'élève** (3) ;
   - **le matériel réservable** (chariot de tablettes, kit de robotique, malle de sciences, minibus) réservé **avec la salle** dans le même geste, par le moteur du 19 ;
   - **les demandes d'intervention** de n'importe quel personnel (ou d'un élève de l'internat par son surveillant), en deux gestes depuis le téléphone, photo comprise, **lieu choisi sur la liste des salles**, routées vers l'agent, le service informatique, la collectivité (public) ou l'organisme de gestion (privé), suivies jusqu'à la clôture ; les **urgences de sécurité** passent en alerte ;
   - **le registre de sécurité tenu** : vérifications périodiques avec échéances, observations de la commission levées une à une, exercices tracés (avec le 14) ;
   - **les clés** : organigramme, qui détient quoi, remise et restitution signées, **rien ne part avec un agent qui quitte l'établissement** (6) ;
   - **la reprographie** reprise de Next.js, **cloisonnée par site**, avec compteurs par service et les **plafonds CFC** rappelés à l'enseignant ;
   - une **IA qui prépare** (catégorie et urgence proposées pour une demande, doublons détectés, commande de manuels prévue d'après les effectifs de la rentrée, brouillon de convention, lecture d'une facture de réparation) **et ne décide ni n'envoie rien**, **jamais de surveillance des personnes**.
6. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 146 | RSM-100 à RSM-200 et RSM-225 à RSM-249, plus RSM-250 à RSM-269 (ajout personas) ; RSM-201 à RSM-224 réservés |
   | Cas limites | 58 | CRM-01 à CRM-58 (dont CRM-47 à CRM-58 ajout personas) |
   | Règles | 40 | R-RM1 à R-RM40 (dont R-RM35 à R-RM40 ajout personas) |
   | Réglages signalés | 22 | S-4400 à S-4421 (dont S-4418 à S-4421 ajout personas) |
   | Lots | 13 | RM0 à RM12 |

   - **Lot RM0** (correctifs urgents du code de salles du 5 et du 10, sans migration) : changement de salle d'une séance contrôlé par le port de disponibilité **et** sérialisé par salle ; un seul chemin d'écriture des salles avec la bonne action et le journal ; borne de capacité alignée (§25.1).
   - **Parcours minimal pour la rentrée 2027** : RM0, RM1 (biens et inventaire de base), RM2 (prêts, dont manuels), RM5 (reprographie : remplace le module Next.js en usage), RM4 (demandes d'intervention, dès que le socle N1 ou à défaut le 44 porte le guichet).
7. **Décisions appliquées par défaut (D-031)** : 27 questions tranchées (§29.1), plus 10 décisions de la passe personas (§30, `PERSONAS_44.md` §8.1). **Soumis à Florian** (§29.2) : la caution pour le matériel autre que les manuels (privé et public, avec l'agent comptable) ; le barème de remplacement (vote du CA ou décision de l'organisme de gestion) ; la création du socle N1 « Demandes » avant février 2027 ; le partage des demandes avec la collectivité (public) ; le registre de sécurité tenu dans ScolIA ou ailleurs ; la licence CFC de l'établissement ; ajout personas : Q-RM37 à Q-RM39 (produits chimiques, date de bascule de la reprographie, astreinte technique).
8. **Ajout personas (§30, `PERSONAS_44.md`)** : quatre bloquants tranchés — **manuels suivis en quantité** l'année 1 et étiquetés au retour (R-RM35) ; **astreinte technique** et escalade de l'alerte de sécurité (R-RM36) ; **réceptionnaire de reprographie désigné pour un ou plusieurs sites** (R-RM27 précisée) ; **pertes en mode suivi** pendant le doublon (R-RM37). Six défauts de la réservation de salles de Next.js, dont deux critiques (DRM-14, DRM-15), sont **relayés au 19**.

---

## 1. Thèse : un bien a un emplacement, un détenteur et un état datés ; un prêt se solde

1. **Le bien référence, il ne recopie pas.** Un emplacement est une **salle** ou un **lieu** du 5 ; un détenteur est une **personne** (élève du 2, agent du 6) ; un payeur est une **part** du 4 ; une convention de prêt est un **document** signé du 7. Aucun nom ni « salle B12 » en texte.
2. **Tout est un fait daté en ajout seul.** Entrée à l'inventaire, déplacement, affectation, prêt, retour, panne, réparation, perte, réforme : chacun est un fait, avec son auteur et sa date. L'état courant se calcule ; l'historique ne se réécrit jamais. « Qui avait l'ordinateur n° 412 le 3 février ? » a une réponse exacte.
3. **Un prêt se solde.** Il ne disparaît pas : il est **rendu** (état noté), **rendu abîmé** (constat, barème), **perdu** (déclaration), **racheté** ou **remis** (décision motivée). Un élève qui part a ses prêts **soldés** avant que le dossier se ferme (3) ; un agent qui part rend ses clés (6).
4. **La sécurité n'attend pas.** Une demande d'intervention qui touche la sécurité des personnes est une **alerte** : elle prévient tout de suite la direction et l'agent d'astreinte, propose la condamnation du lieu (indisponibilité du 5) et ne se clôt qu'après vérification.
5. **L'argent passe par le 24 en quantités et en barèmes votés.** Le 44 constate (perte, dégradation, état) ; le 24 facture au bon payeur, au tarif voté, avec l'entité émettrice juste ; aucune caution n'est demandée pour les manuels.
6. **On ne surveille pas les personnes.** Le 44 suit des **objets**. Il n'utilise ni géolocalisation des appareils prêtés, ni historique de navigation, ni lecture des badges d'accès pour suivre qui va où. Un journal d'accès électronique, s'il existe, reste dans son système et n'est consulté qu'en cas d'incident, par la direction.
7. **Le serveur décide, l'IA propose.**

---

## 2. Évaluation franche de l'existant

### 2.1 Où et comment Next.js le fait [F]

| Fonction | Où | Stockage | Maturité (T0) | Socle cible |
|---|---|---|---|---|
| **Réservation de salles** (M02) | `app/api/reservation-rooms/*`, `app/lib/reservation-rooms-db.ts`, `db/schema-reservation-rooms.ts`, migrations 0025, 0031, 0034 | tables `reservation_room`, `reservation_room_booking` (dates en texte sans fuseau) | **Fiable** (Florian) | **19** (AG3) — le 44 ne la refait pas |
| **Planning par domaines** (M03, EVARS) | `app/api/domain-planning/*` | collections KV | Partiel | **19** (AG6b) |
| **Photocopies** (M08) | `app/api/photocopies-couleur/route.ts` (GET, POST, PATCH), `upload-url`, `document`, `directory-users`, `mark-ready`, `mark-ready-seen` ; `app/lib/photocopies-couleur-{types,access,ops,ops-server,ready-token}.ts` ; page `app/(admin)/photocopies/page.tsx` (1 070 lignes) ; assistant `app/lib/brain-ai/tools/handlers/photocopies.ts` | **un seul document** KV `photocopies-couleur` réécrit en entier ; PDF sur S3 | Partiel | **44** (§15) |
| **Maintenance, travaux** | module « Demandes » (`app/lib/requests*.ts`, `app/api/requests/*`) : file « maintenance » avec aiguillage par mots-clés (`requests-routing-defaults.ts:44-48, 113`, `requests.ts:617`) | T + KV + S3 | Partiel | **N1** (guichet) + **44** (suivi technique, §12) |
| **Assistance technique** | `app/api/assistance/route.ts` : ticket par courriel, numéro de référence, pièces jointes, **aucun stockage ni suivi** | — | ébauche | 44 (§12, catégorie « informatique ») |
| Prêts, manuels, cautions, inventaire, parc informatique, clés, badges, casiers | — | — | **inexistants** (recherche ciblée : « manuels » seulement dans les listes de fournitures, « caution » dans des modèles de documents, « badge » = badges d'interface) | 44 |

### 2.2 Ce que fait Next.js : actions reprises [F]

**Photocopies (M08)**

| NRM | Action | Où | Reprise |
|---|---|---|---|
| NRM-01 | Préparer l'envoi d'un PDF : URL S3 signée (PUT, 1 h, PDF seulement) ; rôles professeur, administratif, accueil, surveillant ou CPE, administrateur | `upload-url/route.ts:26-57`, `photocopies-couleur-access.ts:24-35` | RSM-225 (dépôt du 7, identifiant émis par le serveur) |
| NRM-02 | Créer une demande : site, motif (≤ 8 000), classes ou matière (≤ 4 000), nombre, 0 à 5 PDF, type couleur ou noir et blanc ; **noir et blanc accepté d'emblée**, **couleur en attente de la direction du site** ; courriels à la direction, à la file d'impression, accusé au demandeur | `route.ts:321-575` | RSM-226, RSM-227 |
| NRM-03 | Déposer pour le compte d'un enseignant (administratif, accueil, comptabilité, direction), déposant tracé | `route.ts:365-400`, `access.ts:38-49` | RSM-228 |
| NRM-04 | Lister ses demandes ; file d'impression (acceptées et prêtes) ; direction : celles de son site ; compteur « à imprimer » | `route.ts:268-319`, `access.ts:60-78` | RSM-229, RSM-231 |
| NRM-05 | Décision de la direction (accepter, refuser, note), couleur seulement, une fois ; contrôle que la direction est celle du site | `route.ts:701-797`, `access.ts:51-58` | RSM-230 |
| NRM-06 | Marquer « prête » (acceptée → prête), avis au demandeur ; réceptionnaire configurable (liste d'adresses, droit par personne ou par rôle, **accueil par défaut**) | `route.ts:625-699`, `ops-server.ts:35-47`, `module-access-defaults.ts:273` | RSM-232 |
| NRM-07 | Marquer prête par lien de courriel sans connexion (jeton 30 j) — **code mort mais exposé** | `mark-ready/route.ts:58-190`, `public-routes.ts:104` | **non repris** (DRM-05) |
| NRM-08 | Acquitter le signal « prête » à l'ouverture du module | `mark-ready-seen/route.ts` | RSM-233 |
| NRM-09 | Consulter un PDF (URL signée 15 min) | `document/route.ts` | RSM-229 |
| NRM-10 | Lister et créer par l'assistant IA | `handlers/photocopies.ts:104, 159` | RSM-239 (même commande serveur) |

**Réservation de salles (M02)** : les 19 actions NX-01 à NX-19 sont reprises par le **socle 19** (§2.2.1 de `19_AGENDA_RDV.md`, AGD-231 à AGD-238, AGD-280). Le 44 y ajoute la **mise en place** demandée (§16.2) et le **matériel** réservé avec la salle (§9).

### 2.3 Défauts trouvés dans Next.js

**Photocopies**

| DRM | Gravité | Défaut | Où [F] | Correctif ScolIA |
|---|---|---|---|---|
| DRM-01 | **critique** | **File d'impression sans cloisonnement par site** : un réceptionnaire — par défaut **tout compte « accueil »** — voit, télécharge et marque prêtes les demandes **de tous les sites** du compte, PDF compris | `access.ts:73-76`, `route.ts:626`, `module-access-defaults.ts:273` | portée par site (R-RM27) ; réceptionnaire désigné explicitement, jamais un rôle entier par défaut |
| DRM-02 | **critique** | **PDF d'un autre rattachable** : toute clé commençant par `photocopies-couleur/uploads/` est acceptée ; la clé est prévisible (horodatage + nom de fichier) ; on peut joindre puis lire le PDF de quelqu'un d'autre ; l'assistant accepte aussi `brain-ai/uploads/` | `route.ts:72-74, 123`, `upload-url/route.ts:333`, `handlers/photocopies.ts:38` | identifiants d'objets émis par le serveur et liés au déposant (socle 7) |
| DRM-03 | haute | Aucune taille maximale sur l'envoi signé | `upload-url/route.ts:337-342` | taille et type contrôlés (7) |
| DRM-04 | haute | Secret de jeton **codé en dur** en repli (`dev-photocopies-ready-token-secret`) | `photocopies-couleur-ready-token.ts:18` | aucun secret de repli ; route supprimée |
| DRM-05 | haute | **GET public qui modifie** des données (lien « prête ») | `mark-ready/route.ts`, `public-routes.ts:104` | toute action par lien = page puis bouton (R-AG23 du 19) ; non repris |
| DRM-06 | haute | HTML non échappé dans la page publique et les courriels (nom, classes, motif) | `mark-ready/route.ts:94, 174`, `route.ts:230-245, 683-691` | rendu échappé du socle 8 |
| DRM-07 | moyenne | **Validation de la direction contournable** : tout demandeur choisit « noir et blanc », quantité bornée à 1 000 000 | `route.ts:418, 436-446` | seuils par type et par volume (S-4410), au-delà validation ; quantité bornée |
| DRM-08 | moyenne | Dépôt pour autrui sans vérifier que la cible est enseignant ; le déposant ne voit plus la demande | `route.ts:379` | cible = personne avec service (6) ; déposant voit ce qu'il a déposé |
| DRM-09 | moyenne | Direction sans adresse : courriel vers « » sans alerte | `route.ts:50, 532` | destinataires résolus côté serveur, échec visible (8) |
| DRM-10 | moyenne | Document unique réécrit à chaque écriture (pertes en concurrence) | stockage KV | tables (D-036) |
| DRM-11 | moyenne | « Prête par » = nom en texte ; site = libellé | types | personne, site du 5 |
| DRM-12 | moyenne | **PDF pédagogiques envoyés en pièces jointes** à plusieurs destinataires (peuvent contenir des données d'élèves) ; aucune durée de conservation | `route.ts` | lien vers le dépôt, jamais de pièce jointe (D-027 G) ; effacement des PDF à J+30 après « retirée » (§21.3) |
| DRM-13 | faible | Aucun quota, compteur, coût, annulation par le demandeur, statistiques | — | §15 |

**Réservation de salles (M02) : défauts trouvés en plus de ceux du 19** (ND-01 à ND-12 de `19_AGENDA_RDV.md`) — à ajouter au 19 (§30) :

| DRM | Gravité | Défaut | Où [F] | Correctif (au 19) |
|---|---|---|---|---|
| DRM-14 | **critique** | **Annulation de n'importe quelle réservation**, ou de toute une série par son `groupId`, par tout compte authentifié : le serveur ne vérifie que la session ; le contrôle « propriétaire ou administrateur » n'existe que dans l'écran | `app/api/reservation-rooms/reservations/delete/route.ts:33-61` ; écran `page.tsx:861-862` | commande serveur vérifiant auteur, bénéficiaire ou gestionnaire (R-AG du 19) |
| DRM-15 | **critique** | **Modification de la réservation d'autrui** (heure, date, matière, commentaire) par tout compte authentifié | `reservations/update/route.ts:14-24` | idem |
| DRM-16 | haute | La modification **ne revérifie ni l'EDT ni « réservable »** et peut viser une réservation annulée | `update/route.ts:54-57, 125-143` | mêmes contrôles qu'à la création |
| DRM-17 | haute | La modification **réécrit toutes les réservations de l'établissement** (dernier écrivain gagnant) | `update/route.ts:171` | écriture unitaire, verrou optimiste |
| DRM-18 | moyenne | Série sans limite d'occurrences pour les administrateurs ; `untilDate < date` accepté ; horizon configurable négatif ou énorme | `create/route.ts:146-162`, `module-config/route.ts:48-51` | bornes serveur |
| DRM-19 | moyenne | Administrateur global autorisé à paramétrer mais pas à réserver pour autrui (droits incohérents) | `create/route.ts:110` contre `145` | droits D-017 |

**Points de sécurité à remonter à Florian maintenant** (Next.js en production, D-035 point 3 ; le pilote n'y touche pas) : **DRM-14 et DRM-15** (tout personnel peut annuler ou déplacer toutes les réservations de salles de l'établissement) ; **DRM-01** (la file d'impression montre à l'accueil de chaque site les documents de tous les sites) ; **DRM-02** (lecture du PDF d'un collègue) ; **DRM-04 et DRM-05** (secret codé en dur, lien public qui modifie).

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)
- **Photocopies** : les deux circuits (noir et blanc direct, couleur validée par la direction **du site**) ; le dépôt pour le compte d'un enseignant avec le déposant tracé ; la file d'impression et son compteur ; le réceptionnaire configurable ; jusqu'à 5 PDF par demande ; le signal « prête, pas encore vue » ; les avis aux trois étapes ; la compatibilité avec les anciennes demandes (type absent = couleur).
- **Salles** (au 19) : la synchronisation qui ne rétrograde jamais une salle spéciale en salle de classe (`reservation-rooms-db.ts:157-210`) — à garder dans le rapprochement des salles (5).

### 2.5 Code Rust existant [F]

| Élément | Où | Constat |
|---|---|---|
| Salles | `migrations/076_param_salle.sql`, enrichie en `337_structure_lieux.sql` : `site_id`, `batiment_id`, `lieu_id`, `etage`, `type` (11 valeurs), **`equipements TEXT[]`** (≤ 40), `accessible_pmr`, **`reservable BOOLEAN DEFAULT false`** (commentaire « Lu par le socle 44 », `337:96`), `ferme_le` ; RLS (`587:47`) | les équipements sont des **étiquettes** (« vidéoprojecteur ») sans identité : le 44 en fait des biens (§8.4) |
| Bâtiments, lieux, disponibilités, indisponibilités | `337` (`ent_batiment`, `ent_lieu`, `ent_disponibilite`, `ent_salle_indisponibilite` immuable par déclencheur) | base de la condamnation d'un lieu (§12.4) |
| Port de disponibilité | `crates/ent-db/src/structure_lieux.rs:1640-1756` (`structure_disponibilite::disponible`) | **appelé seulement par les tests** (`tests_structure_s2.rs:447-495`) |
| Écriture des salles | (a) `structure_lieux.rs:939-1148` (droits `ReferentielGerer`, journal) ; (b) **héritée** `service.rs:14700` `creer_salle_for_actor` (contrôle **par rôle**, sans action ni journal), `salle_repo.rs:16-40` ; (c) `edt_admin.rs:706` `edt_salle_modifier` sous `EdtGerer` | trois chemins, règles divergentes |
| Changement de salle d'une séance | `crates/ent-db/src/edt_remplacement.rs:1011-1060` (`edt_changer_salle`), route `/edt/changements-salle` (`ent-api/src/edt_s2.rs:52`) | ne regarde que les séances ; salle par **code texte** ; verrou sur le créneau, pas sur la salle |
| Fonctions de personnel | `353:67-72` (`agent_maintenance`), `355:105` (`responsable_laboratoire`) | destinataires possibles des interventions et des prêts de laboratoire |
| Tables de biens, prêts, manuels, interventions, clés, reprographie | — | **absentes** ; aucune action D-017 ; l'espace d'événements `ressources.*` cité par le 41 (INT-188, 227, 228) n'est pas codé |

**Défauts du code Rust existant (RRM)**

| RRM | Gravité | Défaut | Où [F] | Correctif |
|---|---|---|---|---|
| RRM-01 | **haute** | `edt_changer_salle` **ignore** fermeture, indisponibilités (travaux), fenêtres et capacité ; il n'appelle pas `structure_disponibilite::disponible` ; le test d'occupation se fait hors verrou, le verrou advisory porte sur `etab:creneau_id` et non sur la salle : **deux changements simultanés vers la même salle passent** ; une première transaction est ouverte puis masquée par une seconde (l. 1035 puis 1053) | `edt_remplacement.rs:1011-1060` | RM0 : appel du port, verrou advisory sur `etab:salle_id`, une seule transaction, test de concurrence et test « salle en travaux refusée » |
| RRM-02 | **haute** | **Trois chemins d'écriture des salles** : l'ancien contrôle par rôle sans journal ; `edt_salle_modifier` accepte une capacité **0–5 000** alors que la base impose **≤ 500** (erreur SQL au lieu d'un refus), sans journal, sous `EdtGerer` | `service.rs:14700`, `edt_admin.rs:706-736` | RM0 : chemins hérités redirigés vers `structure_lieux.rs` (action `ReferentielGerer`, journal), borne 500 au service, retrait des doublons quand les écrans basculent (le 10 l'a relevé : D16) |
| RRM-03 | moyenne | Port de disponibilité **non branché** en production ; `creneaux_touches` joint par **texte** (`lower(c.salle) = lower(s.code)`) ; les créneaux de l'EDT n'ont pas de `salle_id` | `structure_lieux.rs:1598, 1640` | RM0 : le port devient l'unique contrôle de lieu (19, 44, 10) ; la FK séance → salle est un lot du **10** (l. 1169 du 10), rappelé ici |

### 2.6 Verdict
- **Ne pas refaire** la réservation de salles ni le planning de domaines : ils sont au 19. Lui **relayer** six défauts critiques ou hauts qu'il n'a pas listés (DRM-14 à DRM-19).
- **Reprendre** la reprographie de Next.js (actions, circuits), en la **cloisonnant** et en la branchant sur les personnes, les sites et les dépôts du cœur.
- **Concevoir de zéro** biens, inventaire, prêts, manuels, interventions, sécurité des locaux, clés.
- **Corriger d'abord** le contrôle de disponibilité des salles (RM0), car le 19 et le 44 vont s'y appuyer.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** | répond de la sécurité des locaux, préside la commission interne, signe les conventions | registre de sécurité papier incomplet le jour de la commission ; observations non levées oubliées | un accident sur un équipement signalé et non réparé | tableau « sécurité » : vérifications à jour, observations ouvertes, demandes de sécurité en cours |
| **Adjoint gestionnaire, intendant** (public) ; **directeur administratif et financier** (privé) | inventaire, achats, prêts, cautions, conventions avec la collectivité, organisme de gestion | inventaire annuel sur tableur ; manuels comptés à la main ; facturation des pertes à la main | écarts d'inventaire inexpliqués ; manuels non rendus par les élèves partis | inventaire au téléphone, pertes facturées au barème, export comptable |
| **Agent comptable** (public) | encaissements, sorties d'inventaire, immobilisations | justificatifs dispersés | encaisser une caution interdite | sorties d'inventaire avec délibération jointe, aucun encaissement non autorisé |
| **Agents techniques, d'entretien** (souvent de la collectivité dans le public) | réparent, installent, mettent en place les salles | demandes orales dans le couloir, sans lieu précis, en double | oublier une urgence ; être appelé pour une demande déjà faite | une liste du jour par bâtiment, la salle et la photo, « fait » en un geste |
| **Référent numérique, informaticien** | parc, pannes, prêts d'ordinateurs, comptes | tickets par courriel sans suivi ; numéros de série recopiés | perdre la trace d'un portable prêté | parc avec numéros de série, garantie, détenteur, historique des pannes |
| **Professeur** | réserve du matériel, signale une panne, demande des copies | réserver salle et chariot de tablettes séparément ; panne du vidéoprojecteur signalée trois fois | arriver en cours sans le matériel réservé | réserver salle + matériel en un geste ; panne signalée en 10 s depuis la salle (QR code) |
| **Professeur documentaliste, responsable de laboratoire** | prêts spécialisés, inventaire de laboratoire | cahiers de prêt | produits ou matériel dangereux non suivis | prêts et inventaire de sa salle |
| **Secrétariat, accueil** | distribue et récupère clés, badges, matériel ; reçoit les photocopies | cahier des clés ; demandes de copies de tous les sites (Next.js) | remettre une clé sans savoir à qui | remise et retour signés, liste « clés dehors » |
| **CPE, vie scolaire, surveillants d'internat** | casiers, matériel de vie scolaire, états des lieux des chambres | papiers d'états des lieux | litige de dégradation sans preuve | état des lieux photographié, signé, comparé |
| **Famille** (dont séparée) | signe la convention de prêt, paie une perte | papiers en début d'année, factures surprises | payer pour l'autre parent ; payer une dégradation contestée | convention en ligne, constat clair avec photo, barème connu |
| **Élève mineur, majeur** | emprunte, rend, signale | queues pour rendre les manuels | être accusé d'une dégradation d'avant | état noté à la remise, voir ce qu'il doit rendre |
| **Collectivité territoriale** (public) | propriétaire des locaux, agents, gros entretien, ordinateurs régionaux | demandes reçues par courriel sans suivi | doublons, demandes non qualifiées | un export ou un flux des demandes qualifiées qui la concernent |
| **Organisme de gestion** (privé) | propriétaire, travaux, assurances | idem | idem | idem |

---

## 4. L'année vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui se passe |
|---|---|
| **Juin** | retour des manuels et des ordinateurs (par classe, sur créneaux), constats, pertes ; inventaire physique annuel ; commande de manuels pour la rentrée (effectifs prévisionnels du 3 et du 5, changements de programmes) ; vérifications périodiques souvent en été |
| **Juillet–août** | travaux (collectivité, organisme de gestion) ; préparation des lots de manuels par classe ; mise à jour du parc ; clés des nouveaux agents |
| **Rentrée** | distribution des manuels par classe (1 à 2 jours) ; ordinateurs régionaux pour certains niveaux ; casiers ; clés et badges des nouveaux personnels ; premier exercice d'évacuation du mois (avec le 14) |
| **Septembre–octobre** | demandes d'intervention nombreuses (rentrée), réservations de matériel, reprographie intense |
| **Toute l'année** | pannes, réparations, prêts ponctuels, reprographie, commission de sécurité (périodique ou inopinée) |
| **Fin d'année civile** | arrêté comptable : immobilisations, sorties d'inventaire votées au CA (public) |

## 5. Ce qui est réglementaire (recherche du 2026-11-11)

Recherche par sous-agent (Légifrance en version au 11/11/2026, BO, réponses ministérielles, CNIL, CFC). education.gouv.fr a renvoyé 403 : la M9-6 et plusieurs BO n'ont été vus qu'en extrait **[S]**. URL consultées le **2026-11-11**.

### 5.1 Manuels scolaires
- **Collège** : C. éduc. **L211-8, 5°** (version du 02/09/2024) : l'État a la charge des dépenses pédagogiques des collèges « ainsi que de la fourniture des manuels scolaires dans les collèges » [V]. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166572/
- **Caution interdite, responsabilité des familles** : réponse ministérielle (QE AN n° 3765, JO du 6 septembre 1993) : « l'existence d'une caution pour le prêt des manuels […] est une pratique formellement proscrite » (note de service 85-222 du 12 juin 1985) ; la « responsabilité pécuniaire des familles est engagée dans les conditions du droit commun » en cas de perte ou de dégradation, selon un **barème dégressif** : 1re année prix du neuf, 2e année 50 %, 3e année 30 %, 4e année 20 % (circulaire IV/70-68 du 5 février 1970) ; sommes perçues **réservées** à la remise en état ou au renouvellement des manuels (circulaire 78-114) [V réponse ; **circulaires anciennes, maintien en vigueur non vérifié**]. https://www.assemblee-nationale.fr/dyn/10/questions/QANR5L10QE3765
- **Conseil d'administration** : **R421-20** : adopte « les tarifs des ventes des produits et de prestations de services » (base d'une délibération sur les tarifs de remplacement) ; **R421-23** : avis sur les principes de choix des manuels [V]. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000018377460/
- **Lycée** : aucune obligation nationale ; gratuité selon les régions [NV]. **Privé sous contrat** : financement des manuels de collège non vérifié article par article [NV]. **Ordinateurs et tablettes des collectivités** : aucun cadre national ; convention et charte de chaque collectivité [NV].
- **Conséquences** : (a) **aucune caution** n'est possible sur un prêt de manuel (R-RM12, bloquant) ; (b) le **barème dégressif** est le barème par défaut d'un manuel perdu ou dégradé, **modifiable seulement par une délibération** (public) ou une décision de l'organisme (privé) enregistrée avec sa date (S-4404) ; (c) l'**année d'entrée** de l'exemplaire en service est donc une donnée obligatoire (§11.1) ; (d) les sommes perçues sont **affectées** au renouvellement des manuels (indication au 24 et au 48).

### 5.2 Cautions et encaissement (EPLE)
- Régies d'avances et de recettes des EPLE : **décret 2020-542 du 7 mai 2020** adaptant le décret 2019-798 ; régie créée par décision de l'ordonnateur, qui ne peut être régisseur [S, source juridique secondaire].
- **Chèque de caution non encaissé** pour du matériel autre que les manuels : **aucun texte trouvé** ; le compte M9-6 « 165 dépôts et cautionnements reçus » n'a pas pu être consulté [NV]. **À faire valider par un agent comptable** (Q-RM2).
- Tarifs de remplacement du matériel perdu ou dégradé : délibération du CA (R421-20) [V pour la compétence].
- **Conséquence** : ScolIA **ne conserve jamais de chèque** et **ne gère pas** de caution dans V1 ; un **dépôt de garantie** n'est possible, sur un matériel autre que les manuels, que si l'établissement le déclare (réglage S-4405, désactivé), avec la référence de la délibération ou de la décision, et passe par le 25 comme un encaissement réel remboursable (R-RM13).

### 5.3 Inventaire (M9-6) [S]
- Seuil d'immobilisation **800 € HT** (en dessous, le bien « peut ne pas être immobilisé ») ; **sortie d'inventaire** pour perte, vol, destruction, réforme ou non-emploi, autorisée par le **conseil d'administration**, puis décisions budgétaires ; devenir : rebut, transfert, désaffectation (autorisation du préfet) — fiche académique de Caen. https://wordpress.ac-caen.fr/cic/wp-content/uploads/2014/06/inventaires_fiche_procedure_sorties_inventaire.pdf
- **BO n° 7 du 12 février 2026** (MENF2600269J) annonce des règles de sortie d'inventaire **simplifiées** (§2.1.2.2) [S, non lu] : **à lire avant de coder RM1** (Q-RM8). https://www.education.gouv.fr/bo/2026/Hebdo7/MENF2600269J
- Déchets d'équipements électriques et électroniques : Code de l'environnement (filière D3E) [NV].
- **Conséquences** : distinction **bien immobilisé** (valeur ≥ seuil, export au 48) / **bien suivi** (petit matériel) ; une **sortie d'inventaire** est une **proposition** groupée soumise à la décision (délibération du CA dans le public, décision de l'organisme dans le privé), jamais une suppression ; le seuil est un réglage (S-4400, 800 € HT par défaut).

### 5.4 Sécurité des locaux [V, S]
- **Registre de sécurité** des établissements recevant du public : CCH **R143-44** (nouvelle version au **01/07/2026**, décret 2025-1100 ; ancien R123-51) : (1) travaux d'aménagement (dates, nature, entreprises) ; (2) liste du personnel de sécurité ; (3) consignes incendie, dont l'évacuation selon les types de handicap ; (4) **dates et résultats des vérifications** ; (5) **dates des exercices** [V]. **R143-34** : vérifications périodiques par des organismes agréés, périodicité fixée par le règlement de sécurité ; **R143-41** : visites périodiques et inopinées de la commission ; **R143-38** : visite de réception [V]. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006074096/LEGISCTA000043819015/
- **Exercices d'évacuation** : article R 33 du règlement de sécurité (type R) : premier exercice dans le mois de la rentrée, au moins deux par an, exercices de nuit en internat [S].
- **Santé et sécurité au travail** : décret 82-453 : art. 3-2 registre de santé et de sécurité au travail ; art. 5-6 à 5-9 alerte et retrait, **registre des dangers graves et imminents** (5-8) [S] ; agents territoriaux : décret 85-603 [NV].
- **Clés** : **aucune règle nationale** ; organisation interne et convention avec la collectivité.
- **Conséquences** : ScolIA **peut tenir** le registre de sécurité sous forme numérique (rubriques de R143-44), ou seulement **exporter** ses éléments si l'établissement tient un registre papier (Q-RM5) ; les **vérifications** ont des échéances et des observations ; les **exercices** sont tracés avec le 14 (qui les déclenche) ; une demande d'intervention qui révèle un **danger grave et imminent** propose l'inscription au registre dédié (RSM-200).

### 5.5 Reprographie : licence CFC [V, S]
- Protocole ministère, CFC et SEAM du 3 mars 2023 ; circulaire **BO n° 44 du 23 novembre 2023** (MENE2329331C) : chaque établissement du second degré, **public et privé sous contrat**, est invité à signer une licence [S]. https://www.education.gouv.fr/bo/2023/Hebdo44/MENE2329331C
- **Notice de licence CFC (janvier 2025)** [V, document lu] : copies papier pour la formation initiale ; extraits seulement : **au plus 10 % d'un livre ou d'une partition, 30 % d'un journal ou d'une revue** ; **au plus 180 pages par élève et par an** ; références à côté de chaque extrait ; affiche près des copieurs ; redevance 2025 **1,80 € HT** par élève (1 à 100 pages) ou **3,65 € HT** (101 à 180) ; déclaration par un **échantillon de 1 200 établissements** par an, où les enseignants déclarent anonymement les œuvres copiées pendant **4 semaines consécutives**. https://cfcopies.com/media/785/download/Notice-licence-sec-reprographie.pdf
- Usages numériques : exception pédagogique (CPI L122-5) et accords sectoriels [NV].
- **Conséquences** : la demande de reprographie porte une case « contient des extraits d'œuvres protégées » ; ScolIA **rappelle** les plafonds (10 %, 30 %) et tient un **compteur de pages d'œuvres par classe** ou par élève pour la tranche CFC (100 ou 180 pages) ; quand l'établissement est **tiré dans l'échantillon**, il active une **période de déclaration** de 4 semaines qui pré-remplit, pour chaque enseignant, la liste des œuvres déclarées dans ses demandes (référence saisie, jamais le contenu) ; l'export suit le format du CFC [NV].

### 5.6 Mise à disposition des locaux à des tiers [V]
- **C. éduc. L213-2-2** (collèges : président du conseil départemental) et **L214-6-2** (lycées : région) : hors besoins de formation ; avis du conseil d'administration ; accord du propriétaire ; neutralité et laïcité ; **convention obligatoire tripartite** (collectivité, établissement, organisateur) précisant sécurité, responsabilités et réparation des dommages, conditions financières. **L212-15** pour les écoles (maire, convention facultative). https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182383/ · https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182386/
- **Privé sous contrat** : non applicable ; l'organisme de gestion décide par contrat [hypothèse].
- **Conséquence** : une réservation de salle par un **tiers** (association extérieure, entreprise) au 19 exige au 44 une **convention de mise à disposition** valide (document du 7, signée par les trois parties dans le public), avec ses créneaux et ses salles ; sans convention valide, la réservation est refusée (R-RM30).

### 5.7 Données et conservation
- Instruction de tri **2005-003 du 22 février 2005** [S] ; aucune durée spécifique aux prêts, clés ou interventions : durées par finalité à fixer avec le DPO (§21.3).
- Appareils prêtés : **aucune géolocalisation** ni lecture du contenu (principe de minimisation, RGPD 5.1.c ; CNIL, géolocalisation) [R].

### 5.8 Ce qui reste à vérifier [NV]
1. BO n° 7 du 12 février 2026 (sorties d'inventaire simplifiées) et la M9-6 en vigueur.
2. Maintien en vigueur des circulaires de 1970, 1978 et 1985 sur les manuels.
3. Compte 165 de la M9-6 (dépôts et cautionnements) ; pratique des agents comptables.
4. Arrêté du 25 juin 1980 (R 33, GE 4) : périodicités de vérification par équipement.
5. Format de la déclaration CFC par échantillon.
6. Conventions de prêt d'ordinateurs de la région de l'établissement pilote.

---

## 6. Frontières et branchements sur le cœur

### 6.1 Qui possède quoi
| Objet | Propriétaire | Le 44 |
|---|---|---|
| Sites, bâtiments, salles, lieux, disponibilités, indisponibilités, `reservable` | 5 | lit ; **pose une indisponibilité** par la commande du 5 quand une intervention condamne un lieu |
| Réservation de salles et de lieux | **19** | **fournit** le matériel réservable comme ressource du moteur du 19 ; reçoit la mise en place demandée (`ent_agenda_reservation_besoin`) |
| Biens, inventaire, parc | **44** | possède |
| Prêts (matériel, manuels, clés, badges, casiers) | **44** | possède ; le CDI (45) aura ses prêts documentaires propres sur le même modèle de prêt [R] |
| Demandes (guichet, routage, délégation, à-faire) | **N1** (à créer, T-01) ; à défaut **44** pour les seules demandes d'intervention et de reprographie | possède le **suivi technique** (intervention, pièces, coût, vérification) |
| Heures supplémentaires effectives (demandes d'HSE de M08) | 15 puis 49 (DEM-05, PER-15) | rien |
| Personnes, missions (agent de maintenance, référent numérique, responsable de laboratoire), départ d'un agent | 6 | lit ; réagit au départ (clés, prêts) |
| Élève, départ | 2, 3 | solde des prêts avant clôture du dossier |
| Payeur, facture de perte ou de dégradation, dépôt de garantie | 4, 24, 25 | constate, transmet une **quantité et un barème** |
| Conventions, constats, photos, PDF de reprographie | 7 | dépose |
| Avis | 8 | déclare ses natures |
| Exercices d'évacuation, PPMS, alerte | 14 | trace dans le registre de sécurité ; fournit les équipements (alarme, extincteurs) |
| États des lieux des chambres d'internat | 41 (demande) / 44 (constat) | constat, photos, comparaison |
| Matériel emporté en sortie | 42 | prêt au porteur de la sortie |
| Comptabilité, immobilisations, fournisseurs | 48 | export des biens immobilisés, coûts d'intervention ; annuaire des prestataires partagé (42, 43) |

### 6.2 Ports consommés
| Port | Socle | Usage |
|---|---|---|
| `structure::salles`, `structure_disponibilite::disponible`, `structure::indisponibilite_poser` | 5 | emplacement ; condamnation |
| `agenda::ressource_declarer`, `agenda::reservation(id)`, `agenda::besoins(reservation)` | 19 | matériel réservable ; mise en place |
| `personnels::mission(type, date)`, `personnels::depart(agent)` | 6 | destinataires ; départ |
| `eleve::scolarite_a(date)`, `inscriptions::depart_en_cours(eleve)` | 2, 3 | prêts de rentrée ; solde |
| `administratif::titulaires`, `::payeurs(eleve, categorie)` | 4 | convention de prêt ; payeur d'une perte |
| `documents::deposer`, `documents::signer` | 7 | convention, constat, photo |
| `facturation::ligne(source, payeur, bareme, quantite)` | 24 | perte, dégradation |
| `edt::occupe(salle, plage)` | 10 | conflit (via le 19) |

### 6.3 Ports exposés par le 44
| Port | Consommateurs | Contenu |
|---|---|---|
| `ressources::prets_en_cours(personne)` | 3 (départ), 6 (départ d'un agent), 2 (fiche 360°), 29 | prêts non soldés |
| `ressources::cles_detenues(personne)` | 6, 14 | clés et badges |
| `ressources::equipements(salle)` | 5 (étiquettes d'équipements), 19 (recherche de salle), 10 (contraintes de salle) | équipements **en service** d'une salle (un vidéoprojecteur en panne n'en est plus un) |
| `ressources::materiel_reservable` | 19 | ressources génériques |
| `ressources::lieu_condamne(salle)` | 5, 19, 10 | condamnation en cours |
| `ressources::registre_securite(etab)` | 14, 47 | état des vérifications et observations |
| `ressources::interventions_ouvertes(salle)` | 19, 41 | avertissement « vidéoprojecteur en panne signalé » à la réservation |

### 6.4 Répercussions automatiques
| Événement | Effet |
|---|---|
| `eleve.depart` (2) / départ en cours (3) | liste des prêts à solder présentée au secrétariat ; dossier non clos tant qu'ils ne sont pas soldés **ou** marqués « à recouvrer » (décision) |
| départ d'un agent (6) | clés, badges, matériel à rendre ; tâche bloquante avant la fin des accès (R-RM20) |
| salle fermée, indisponible (5) | biens de la salle signalés (déplacer ?) ; interventions en cours rattachées |
| réservation de salle avec mise en place (19) | tâche de mise en place pour les agents à J-1 (§16.2) |
| réservation annulée (19) | tâche de mise en place annulée |
| sortie (42) avec matériel | prêt au porteur ; retour attendu à la date de retour |
| titulaire ou payeur changé (4) | conventions de prêt valides ; une facture future suit le payeur du jour |
| fin d'année (5) | campagne de retour des manuels ; inventaire proposé |
| intervention « sécurité » ouverte | indisponibilité proposée au 5 ; avis à la direction ; registre |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]
| Terme | Définition |
|---|---|
| **Catégorie** | famille de biens (ordinateur portable, tablette, vidéoprojecteur, manuel, calculatrice, instrument, matériel de sport, clé, badge, casier, mobilier, véhicule, copieur…) avec ses attributs (numéro de série, garantie) |
| **Modèle** | référence commune (« manuel Hatier Maths 4e, ISBN … », « tablette X 64 Go ») |
| **Bien** | objet identifié par une **étiquette** (code-barres ou QR code) ; immobilisé ou suivi |
| **Lot** | ensemble de biens de même modèle suivis en quantité (rames de papier, ballons) sans étiquette individuelle |
| **Emplacement** | salle ou lieu du 5 ; « réserve » ; « chez le détenteur » |
| **Détenteur** | personne qui a le bien par un prêt |
| **Prêt** | engagement daté d'un bien à une personne, avec convention si nécessaire, qui se solde |
| **Campagne de prêt** | distribution en série (manuels de la 4e, ordinateurs de la 2de) |
| **Constat** | état noté, avec photo, à la remise ou au retour |
| **Demande d'intervention** | « ça ne marche pas » : lieu, objet, description, urgence, photo |
| **Intervention** | travail pour la traiter : qui, quand, pièces, coût, résultat |
| **Condamnation** | indisponibilité posée au 5 pour la sécurité ou des travaux |
| **Vérification** | contrôle périodique réglementaire ou interne (extincteurs, alarme, électricité, gaz, ascenseur, aires de jeux, hottes) avec échéance et observations |
| **Organigramme des clés** | arbre des clés (passe général, passes partiels, clés de salle) et des cylindres |

### 7.2 Distinctions qui manquent aujourd'hui [R]
1. **Équipement de salle (étiquette du 5) et bien (44).** Le 5 dit « cette salle a un vidéoprojecteur » (étiquette, pour la recherche) ; le 44 sait **lequel**, **s'il marche** et **depuis quand**. Règle : l'étiquette du 5 est **calculée** depuis les biens en service de la salle quand le 44 est actif ; sinon saisie (R-RM5).
2. **Prêt et affectation.** Un bien **affecté** à une salle ou à un service n'est pas prêté (le vidéoprojecteur de la salle B12) ; un bien **prêté** l'est à une personne, avec un retour attendu.
3. **Demande et intervention.** Une demande peut donner zéro (doublon, refus), une ou plusieurs interventions.
4. **Perte, vol, dégradation, usure.** L'usure normale n'est jamais facturée ; la dégradation et la perte suivent le barème ; le vol suit une déclaration (dépôt de plainte de la famille ou de l'établissement) et la décision de la direction.

---

## 8. Biens et inventaire

### 8.1 Catalogue
- **Catégories** préréglées (§7.1) et **modèles** ; un modèle porte prix d'achat, durée d'usage prévue, fournisseur (annuaire commun 42, 43, 48).
- **Entrée** d'un bien : par lot (livraison de 30 tablettes : 30 étiquettes imprimées et collées, numéros de série scannés), par import (tableur du fournisseur), ou unitaire. Le bien porte : modèle, étiquette, numéro de série, date d'entrée, valeur, **financeur** (État, collectivité, établissement, organisme, don), **propriétaire** (l'établissement ou la collectivité, ex. ordinateurs régionaux), garantie, emplacement.
- **Étiquettes** : PDF d'étiquettes QR (identifiant opaque, jamais de nom) ; une étiquette perdue se réimprime.

### 8.2 Vie du bien
Faits datés en ajout seul : `entre`, `affecte` (salle, service), `deplace`, `prete`, `rendu`, `en_panne`, `en_reparation`, `repare`, `perdu`, `vole`, `retrouve`, `propose_reforme`, `reforme` (avec la décision), `sorti` (rebut, don, transfert, D3E). L'état courant (`en_service`, `en_pret`, `en_panne`, `hors_service`, `perdu`, `sorti`) est calculé.

### 8.3 Inventaire physique
- **Campagne** par site, bâtiment ou salle ; sur téléphone, l'agent **scanne** chaque étiquette dans la salle ; ScolIA compare à l'attendu : **présent**, **absent** (attendu ici, non vu), **déplacé** (vu ici, attendu ailleurs), **inconnu** (étiquette non enregistrée).
- **Écarts** listés pour traitement (recherche, déplacement confirmé, déclaration de perte) ; rien n'est corrigé seul.
- **Rapport** daté (document du 7), export comptable des biens immobilisés (48).

### 8.4 Parc informatique
- Attributs propres : numéro de série, adresse matérielle (facultative, pour l'équipe réseau), système, date de fin de garantie, **date de fin de support**, état d'effacement à la réforme.
- **Aucune** collecte automatique sur les appareils, **aucune** géolocalisation, **aucun** accès au contenu (§1.6) ; un outil de gestion de flotte externe éventuel n'est pas intégré dans V1 (Q-RM9).
- **Effacement des données** attesté avant réaffectation ou réforme (fait `efface` avec méthode et auteur).

### 8.5 Sortie d'inventaire
- **Proposition** groupée (biens hors service, perdus, volés, obsolètes) avec motif ; **décision** : délibération du CA (public) ou décision de l'organisme (privé), référencée (document du 7) ; puis sortie effective (rebut, D3E, don, transfert) avec la date. Jamais de suppression.

---

## 9. Matériel réservable (moteur du 19)

- Une catégorie ou un bien peut être **réservable** (chariot de 30 tablettes, malle de sciences, kit de robotique, caméra, sono, **minibus** de l'établissement) ; le 44 le **déclare** comme ressource au moteur du 19 (`agenda::ressource_declarer`) avec : quantité (lot) ou unité, règle (validation, délai, durée maximale, qui peut réserver), emplacement de retrait, conditions (permis de conduire valide pour le minibus, vérifié au 6 [R]).
- **Réserver une salle avec du matériel** : la commande du 19 réserve les deux ensemble (refus groupé si l'un manque), avec rapport par occurrence d'une série.
- **Retrait et retour** : le matériel réservé se **retire** (prêt court au réservant, scan) et se **rend** (scan, état) ; non rendu à l'heure : rappel ; casse : constat et demande d'intervention.
- **Disponibilité réelle** : un bien en panne ou en réparation n'est pas réservable ; une réservation existante sur un bien qui tombe en panne est mise **en conflit** (avis au réservant, remplacement proposé).
- **Minibus** : carnet de bord (kilomètres, carburant, incidents), contrôle technique (vérification §13), conducteur autorisé [R].

---

## 10. Prêts

### 10.1 Prêt individuel
- **À qui** : élève (via ses titulaires s'il est mineur), personnel, intervenant.
- **Quoi** : un bien étiqueté, un lot (quantité), une clé, un badge, un casier.
- **Conditions** : convention exigée selon la catégorie (ordinateur, instrument : oui ; calculatrice : non) ; signée par **un titulaire** pour un mineur (acte usuel ; l'autre informé), par l'élève majeur, par le personnel ; **jamais de caution sur un manuel** (R-RM12) ; dépôt de garantie seulement si S-4405 (§5.2).
- **Remise** : scan, **constat de remise** (état, photo facultative), date de retour attendue (fin d'année, J+7…), signature de remise si la catégorie l'exige.
- **Retour** : scan, **constat de retour** comparé au constat de remise ; usure normale, dégradation (photo, description codée), manquant (chargeur, housse).
- **Retard** : rappels (S-4407), avis à la famille ; jamais de sanction automatique.
- **Perte, vol** : déclaration (par la famille, l'élève, le personnel), pièce (récépissé de plainte facultatif), décision de la direction : facturation au barème, remise gracieuse (motivée), ou recherche.

### 10.2 Campagnes de prêt (manuels, ordinateurs de la collectivité)
- **Préparation** : liste des élèves concernés (classe, niveau, option, régime) et du matériel attendu par élève (liste de manuels du niveau et des options, §11.2) ; **conventions** envoyées en ligne avant la distribution (18, 7) ; élèves sans convention signalés.
- **Distribution** par classe, sur un créneau (réservé au 19), mode « caisse » sur téléphone : scan de la carte de l'élève (ou choix dans la liste), scan des étiquettes, constat global « neuf / bon », reçu dans l'espace famille.
- **Retour** de fin d'année par classe, sur créneaux, même mode ; constats ; manquants listés ; facturation proposée au barème (§11.3) ; reliquat pour le départ ou la rentrée suivante.
- **Élève arrivé en cours d'année** : prêt individuel depuis la réserve ; **élève parti** : solde (§6.4).

---

## 11. Manuels scolaires

### 11.1 Exemplaires
- Chaque manuel est un **bien** (étiquette collée), d'un **modèle** (titre, éditeur, ISBN, discipline, niveau, édition), avec son **année de mise en service** (base du barème dégressif, §5.1) et son état (neuf, bon, usé, à réformer).
- **Manuels numériques** : pas des biens ; accès par le GAR (socle 9) et le CDI (45).

### 11.2 Listes par niveau
- Pour chaque niveau et chaque **enseignement suivi** (offre du 5 : langue vivante, option), la liste des manuels prêtés ; l'élève reçoit ce qui correspond à **ses** enseignements de l'année.
- **Prévision de commande** : effectifs prévus (3, 5) × listes − stock en bon état ; proposée (IA ou calcul), validée par le gestionnaire ; avis du CA sur les principes de choix (R421-23) rappelé au changement de manuel.

### 11.3 Perte et dégradation (R-RM12 à R-RM14)
- **Barème par défaut** : prix du neuf l'année de mise en service, 50 % la 2e, 30 % la 3e, 20 % la 4e et suivantes ; dégradation partielle : fraction décidée par le gestionnaire selon un tableau simple (S-4404) ; usure normale : 0.
- **Délibération** (public) ou décision (privé) enregistrée (date, référence, document) ; un barème sans décision enregistrée ne s'applique pas (refus de facturer).
- **Facturation** : ligne au 24, payeur de la catégorie « fournitures » (4), entité émettrice de l'établissement ; motif et photo du constat joints à l'avis ; **contestation** possible dans un délai (S-4408), revue par le gestionnaire.
- **Affectation** des sommes au renouvellement des manuels (indication analytique au 48).
- **Jamais de rétention** d'un document scolaire (bulletin, certificat) pour un manuel non rendu (R-RM15) [R, principe : la dette suit sa voie de recouvrement].

---

## 12. Demandes d'intervention et maintenance

### 12.1 Signaler (corrige l'absence de suivi de l'assistance Next.js)
- **Qui** : tout personnel ; un élève de l'internat ou un délégué par un personnel (pas de compte élève dans V1, S-4409) ; un intervenant pour son lieu d'activité.
- **Comment** : depuis le téléphone, **en scannant l'étiquette** de la salle ou du bien (QR code à la porte) ou en choisissant la salle ; catégorie (électricité, plomberie, chauffage, menuiserie, serrurerie, informatique, audiovisuel, ménage, espaces verts, sécurité incendie, autre), description courte, photo, urgence proposée (IA ou règle).
- **Doublons** : une demande ouverte sur la même salle et la même catégorie est montrée (« déjà signalé hier par… : +1 ? ») ; le signalement s'ajoute (compteur, abonné aux avis) au lieu de créer un doublon.
- **Sécurité** (catégorie sécurité, ou mot-clé détecté et confirmé) : **alerte** immédiate à la direction et à l'agent d'astreinte (8), proposition de **condamnation** du lieu (indisponibilité du 5, N2), inscription proposée au registre des dangers graves et imminents (§13.3).

### 12.2 Router
- **Destinataire** selon la catégorie et le site : agent ou équipe interne (mission du 6), référent numérique, **collectivité** (public : gros entretien, agents territoriaux) ou **organisme de gestion** (privé), **prestataire** sous contrat (ascenseur, chauffage).
- **Vers l'extérieur** (collectivité, prestataire) : envoi **qualifié** (lieu, catégorie, description, photo, sans nom d'élève) par courriel ou export, avec la référence de la demande ; les réponses sont saisies (ou reçues par la boîte de réception du 42 si elle est mutualisée [R]).
- **Guichet N1** : quand le socle N1 existe, la demande **naît au guichet** et le 44 porte le suivi technique ; d'ici là, le 44 porte le guichet des demandes d'intervention seulement (Q-RM1).

### 12.3 Traiter
- Prise en charge (qui, quand), planification, **interventions** (temps passé, pièces, coût, prestataire, facture jointe), mise en attente motivée (pièce commandée, attente de la collectivité), **résolution**, **vérification** par le demandeur ou la direction pour la sécurité, clôture ; réouverture possible (même problème dans les 30 jours).
- **Avis** au demandeur et aux abonnés à chaque étape (8) ; jamais une avalanche : récapitulatif quotidien sauf urgence.
- **Délais** : cible par urgence (S-4411) ; dépassement signalé au responsable.
- **Maintenance préventive** : tâches récurrentes (nettoyage des filtres, purge, changement des ampoules d'un vidéoprojecteur à N heures) générées par modèle.

### 12.4 Condamner un lieu
- Commande du 5 (`structure::indisponibilite_poser`) avec motif `securite` ou `travaux`, depuis l'intervention ; le 19 met les réservations en conflit, le 10 signale les séances ; **levée** à la clôture vérifiée.

---

## 13. Sécurité des locaux et vérifications

### 13.1 Registre de sécurité (CCH R143-44)
- Rubriques : **travaux** (dates, nature, entreprises), **personnel de sécurité** (liste, formations), **consignes** (dont évacuation selon les types de handicap), **vérifications** (dates, organismes, résultats), **exercices** (dates, déroulé ; reçus du 14).
- Tenue **dans ScolIA** ou **export** vers le registre tenu ailleurs (Q-RM5) ; impression du registre à jour pour la commission.

### 13.2 Vérifications périodiques
- **Équipements soumis** (extincteurs, alarme incendie, désenfumage, électricité, gaz, ascenseur, portes automatiques, installations sportives, hottes, aires de jeux, minibus) : modèle de vérification (périodicité **saisie par l'établissement**, l'arrêté ne l'ayant pas été relu, §5.8), organisme, date, résultat, **observations**.
- **Observation** = tâche à lever (une par observation), avec responsable et échéance ; levée justifiée (facture, photo, attestation) ; la commission de sécurité est une vérification particulière avec son avis.
- **Échéances** : rappel à J-30 ; dépassement visible au tableau « sécurité ».

### 13.3 Registres santé-sécurité au travail
- **Registre de santé et de sécurité au travail** (observations des personnels) et **registre des dangers graves et imminents** : tenus par l'établissement [S] ; ScolIA propose une **saisie** et un **suivi** (lien avec la demande d'intervention et sa résolution), sans remplacer la procédure d'alerte et de retrait ; accès restreint (direction, assistant de prévention). Le socle 49 (santé au travail, T-06) en reprendra la gouvernance [R].

---

## 14. Clés, badges, casiers, états des lieux

### 14.1 Organigramme des clés
- **Cylindres** (serrures) rattachés aux salles du 5 ; **clés** (numérotées, type : passe général, passe partiel, clé de salle) ; quelles clés ouvrent quels cylindres.
- **Remise** à une personne (personnel, intervenant, prestataire) : signature, date de retour attendue (fin de contrat, fin de travaux) ; **passe général** remis seulement sur décision de la direction (★).
- **Restitution** : scan ou saisie ; **perte** : déclaration, proposition de **changement de cylindres** concernés (intervention), décision.
- **Départ d'un agent** (6) : tâche bloquante « clés à rendre » avant la fin des accès (R-RM20).
- **Liste « clés dehors »** par personne et par salle ; inventaire des clés annuel.

### 14.2 Badges d'accès
- Si l'établissement a un contrôle d'accès électronique : le 44 suit le **badge** comme un bien prêté (numéro, détenteur, dates) ; il **n'importe pas** le journal des passages (R-RM22) ; l'activation et la désactivation restent dans le système d'accès (procédure : désactivation demandée au départ de la personne, tracée).
- Le badge de cantine (40) n'est pas un badge d'accès ; il reste au 40.

### 14.3 Casiers
- Casiers (élèves, personnels) : emplacement, attribution datée, clé ou code (le code n'est **jamais** stocké en clair : il est remis et changé à la libération), état des lieux à la libération ; campagne d'attribution de rentrée (par niveau, demande des familles, tirage si nécessaire).

### 14.4 États des lieux (internat, logements de fonction, locaux prêtés)
- Modèle d'état des lieux par type de local (chambre d'internat : murs, mobilier, literie, sanitaires, clés) ; **entrée** et **sortie** avec photos, signatures (élève et titulaire pour un mineur ; personnel), **comparaison** ; dégradation constatée → facturation au barème (24) ou intervention.
- Demandé par le 41 (INT-227) pour les chambres ; le 44 porte le constat.
- **Logements de fonction** (personnels logés) : états des lieux et inventaire des équipements du logement ; le reste (attribution, avantages en nature) relève du 6 et du 49.

---

## 15. Reprographie (reprend NRM-01 à NRM-10 ; corrige DRM-01 à DRM-13)

### 15.1 Demande
- **Qui** : professeurs, administratifs, vie scolaire, accueil (pour eux) ; **dépôt pour le compte** d'un enseignant (personne avec un service au 6) par l'administratif, l'accueil, la comptabilité ou la direction, le déposant tracé et la demande visible des deux.
- **Contenu** : site (du 5), un à cinq PDF (dépôt du 7, identifiants émis par le serveur, taille plafonnée), nombre d'exemplaires, recto-verso, agrafage, couleur ou noir et blanc, classes ou groupes destinataires (du 5 : le nombre d'élèves pré-remplit le nombre d'exemplaires), date souhaitée, motif, case **« contient des extraits d'œuvres protégées »** (titre, auteur, éditeur, pages) (§5.5).
- **Circuit** :
  - **noir et blanc** sous le seuil (S-4410, par défaut 500 copies) : accepté d'emblée ;
  - **couleur**, ou noir et blanc au-delà du seuil : **validation par la direction du site** (ou un délégué désigné) ; décision une fois, note facultative ;
  - **œuvres protégées** au-delà des plafonds CFC (10 % d'un livre, 30 % d'une revue) : **avertissement** à l'enseignant, sans blocage (il peut ajuster) ; dépassement du compteur annuel de la classe (100 ou 180 pages selon la tranche souscrite) : avertissement au gestionnaire.
- **Annulation** par le demandeur tant que ce n'est pas imprimé.

### 15.2 File d'impression
- **Par site** : le réceptionnaire (personne désignée, ou mission « reprographie » du 6 ; **jamais un rôle entier par défaut**) ne voit que les demandes acceptées **de son site** (corrige DRM-01).
- Ouvre le PDF (lien signé court, journalisé), imprime, marque **prête** (lieu de retrait) ; avis au demandeur ; signal « prête, pas encore vue » ; **retirée** (facultatif).
- **Compteurs** : copies par service, discipline, demandeur et site ; couleur et noir ; coût unitaire saisi (contrat du copieur) ; export pour la refacturation interne éventuelle (48).
- **Copieurs** : biens du 44 (§8), compteurs relevés (saisie ou import du prestataire), pannes en demande d'intervention.

### 15.3 Déclaration CFC
- Quand l'établissement est **tiré dans l'échantillon** du CFC : période de 4 semaines activée par le gestionnaire ; chaque enseignant voit les œuvres qu'il a déclarées dans ses demandes et complète ; export au format demandé [NV] ; aucune donnée d'élève.
- **Redevance** : nombre d'élèves déclaré (5) et tranche (100 ou 180 pages) : information pour le gestionnaire.

---

## 16. Locaux prêtés à des tiers et mise en place

### 16.1 Mise à disposition à des tiers (§5.6)
- **Convention** : organisateur (association, entreprise, partenaire), collectivité (public), salles et lieux, créneaux (récurrence), période, obligations de sécurité, assurance, conditions financières ; document du 7 signé par les parties ; avis du CA (public) référencé.
- La réservation au 19 par ou pour le tiers **exige** une convention valide couvrant la salle et le créneau (R-RM30) ; clés remises au tiers (§14.1), état des lieux si la convention le prévoit.

### 16.2 Mise en place demandée par une réservation
- Le 19 enregistre les **besoins** d'une réservation (`ent_agenda_reservation_besoin` : tables en U, 60 chaises, vidéoprojecteur, micro, chauffage) ; le 44 en fait des **tâches** pour les agents (liste du jour par bâtiment, à J-1 par défaut, S-4412), avec le matériel à apporter (réservé au besoin) ; « fait » en un geste ; annulée avec la réservation.

## 17. Cas limites (catalogue)

### 17.1 Prêts et manuels
| CRM | Cas | Traitement |
|---|---|---|
| CRM-01 | Parents séparés : la mère signe la convention du portable, le père est payeur des fournitures ; l'ordinateur est cassé | facture au payeur de la catégorie (4) ; la signataire est informée ; contestation ouverte aux deux |
| CRM-02 | Un gestionnaire demande un chèque de caution pour les manuels | refus (R-RM12) ; message qui cite la règle |
| CRM-03 | Manuel perdu, mis en service il y a 5 ans | barème : 20 % du neuf |
| CRM-04 | Manuel rendu abîmé par l'eau, sans constat de remise | constat de remise absent = état « bon » présumé à la remise ? Non : **sans constat de remise, pas de facturation de dégradation** (R-RM14) ; seule la perte reste facturable |
| CRM-05 | L'élève part en cours d'année avec ses manuels | solde demandé ; si non rendus : facture au barème, dossier clos avec mention « à recouvrer » |
| CRM-06 | Élève en garde alternée qui laisse ses manuels chez l'autre parent en juin | relance aux deux titulaires ; délai |
| CRM-07 | Élève qui change d'option en octobre (allemand → espagnol) | retour du manuel d'allemand, prêt de l'espagnol |
| CRM-08 | Famille qui conteste la dégradation | photo de remise et de retour montrées ; revue par le gestionnaire ; remise gracieuse possible (motivée) |
| CRM-09 | Ordinateur régional volé dans le bus | déclaration, récépissé de plainte facultatif, avis à la collectivité propriétaire ; décision selon la convention de la collectivité |
| CRM-10 | Un élève majeur signe lui-même la convention | valide ; il est payeur seulement s'il l'est au 4 |
| CRM-11 | Élève sans titulaire joignable (placement ASE) | convention par le représentant désigné au 4 |
| CRM-12 | Famille en difficulté financière qui ne peut payer une perte | proposition d'aide (26) ; remise gracieuse ; jamais de rétention d'un document (R-RM15) |
| CRM-13 | Deux exemplaires portent la même étiquette (erreur d'impression) | unicité en base : la seconde saisie est refusée ; réimpression |

### 17.2 Inventaire, matériel, réservations
| CRM | Cas | Traitement |
|---|---|---|
| CRM-14 | À l'inventaire, 3 tablettes vues en salle C04 alors qu'attendues en B12 | « déplacé » : confirmation ou recherche ; jamais corrigé seul |
| CRM-15 | Bien inconnu scanné | « inconnu » : enregistrement proposé (modèle, financeur) |
| CRM-16 | Le chariot de tablettes réservé tombe en panne la veille | réservation en conflit ; avis au réservant ; chariot de remplacement proposé |
| CRM-17 | Le minibus réservé n'a plus de contrôle technique valide | réservation refusée (vérification échue) |
| CRM-18 | Le conducteur prévu du minibus n'a pas de permis enregistré valide | refus ; autre conducteur |
| CRM-19 | Matériel emporté en sortie (42) non rendu au retour | rappel au porteur ; constat |
| CRM-20 | Sortie d'inventaire de 40 ordinateurs : la délibération n'est pas votée | sortie en attente ; biens « proposés à la réforme », toujours visibles |
| CRM-21 | Ordinateur réformé non effacé | refus de sortie sans fait `efface` |
| CRM-22 | Bien de la collectivité (ordinateur régional) à réformer | sortie décidée par la collectivité propriétaire (pas par le CA) ; trace |

### 17.3 Interventions et sécurité
| CRM | Cas | Traitement |
|---|---|---|
| CRM-23 | Porte coupe-feu qui ne ferme plus signalée à 10 h | alerte immédiate direction et astreinte ; condamnation proposée ; registre |
| CRM-24 | Fuite d'eau au plafond de la salle informatique | urgence ; salle condamnée ; séances signalées (10), réservations en conflit (19) |
| CRM-25 | Cinq signalements du même vidéoprojecteur | un seul fil, compteur 5, abonnés |
| CRM-26 | Demande qui contient un nom d'élève (« Lucas a cassé la vitre ») | avertissement ; le fait disciplinaire va au 13 ; la demande garde « vitre cassée » ; le nom n'est pas transmis à la collectivité |
| CRM-27 | La collectivité ne répond pas depuis 3 semaines | dépassement signalé au gestionnaire ; relance proposée |
| CRM-28 | Intervention close mais le problème revient en 10 jours | réouverture avec l'historique |
| CRM-29 | Observation de la commission de sécurité non levée à l'échéance | tableau « sécurité » en rouge ; avis à la direction |
| CRM-30 | Vérification des extincteurs dépassée de 2 mois | idem ; rappel à J-30 non traité tracé |
| CRM-31 | Prestataire qui intervient dans une salle de classe pendant un cours | avertissement à la planification (séance au 10) |
| CRM-32 | Danger grave et imminent signalé par un agent | registre dédié proposé ; alerte ; pas de dépendance au réseau (impression) |

### 17.4 Clés, badges, casiers, états des lieux
| CRM | Cas | Traitement |
|---|---|---|
| CRM-33 | Un agent quitte l'établissement avec le passe partiel du bâtiment B | tâche bloquante au départ (6) ; si non rendu : perte ; changement de cylindres proposé |
| CRM-34 | Clé de salle prêtée à un intervenant extérieur non rendue | rappel ; convention |
| CRM-35 | Passe général demandé par un nouveau professeur | refus par défaut ; décision de la direction (★) |
| CRM-36 | Badge d'accès perdu | désactivation demandée tout de suite (procédure tracée) ; nouveau badge |
| CRM-37 | Code de casier oublié | changement de code, jamais de lecture de l'ancien |
| CRM-38 | État des lieux de chambre : dégradation contestée par la famille | photos d'entrée et de sortie ; revue ; barème |
| CRM-39 | État des lieux de sortie impossible (élève parti précipitamment) | constat contradictoire avec un second personnel, signé ; avis à la famille |

### 17.5 Reprographie
| CRM | Cas | Traitement |
|---|---|---|
| CRM-40 | Un enseignant choisit « noir et blanc » pour 5 000 copies | au-delà du seuil : validation (corrige DRM-07) |
| CRM-41 | PDF de 300 pages d'un manuel entier | avertissement CFC (10 %) ; demande possible après confirmation |
| CRM-42 | Réceptionnaire du site A tente d'ouvrir une demande du site B | refus (portée par site) |
| CRM-43 | PDF contenant une liste d'élèves avec leurs notes | rappel à la saisie (case « contient des données d'élèves ») ; effacement à J+30 |
| CRM-44 | Directeur du site sans adresse | la demande reste visible dans l'appli ; échec d'avis signalé (corrige DRM-09) |
| CRM-45 | Établissement tiré dans l'échantillon CFC | période de 4 semaines activée ; pré-remplissage |
| CRM-46 | Copie demandée pour le lendemain 8 h, déposée à 22 h | date souhaitée affichée ; jamais d'avis de nuit au réceptionnaire |

---

## 18. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Propose** catégorie, urgence et doublon d'une demande d'intervention depuis le texte et la photo (N1 ; la sécurité est confirmée par la personne).
- **Prépare** la commande de manuels (effectifs prévus × listes − stock) et la liste des biens à réformer (âge, pannes, fin de support) (N0, validées par le gestionnaire).
- **Lit** une facture de réparation, un bon de livraison ou un rapport de vérification et propose les champs (N1).
- **Rédige** une convention de prêt ou de mise à disposition à partir d'une trame (N0) ; un brouillon de relance (N0, envoi N3).
- **Résume** l'état du registre de sécurité avant une commission (N0).

### 18.2 Ce que l'IA ne fait pas [R]
- Ne décide jamais d'une facturation, d'une remise gracieuse, d'une sortie d'inventaire, d'une condamnation.
- Ne lit aucune donnée de santé ni d'aide ; ne lit pas le contenu des PDF de reprographie (sauf pour l'avertissement CFC, si l'établissement l'active, S-4413, désactivé par défaut).
- Ne surveille aucune personne (aucune analyse des prêts ou des clés « par comportement »).

### 18.3 Capacités déclarées (registre `ent-ia`)
| Capacité | Niveau | Droit requis |
|---|---|---|
| `ressources.qualifier_demande` | N1 | `ressources.signaler` |
| `ressources.prevoir_manuels` | N0 | `ressources.prets_gerer` |
| `ressources.proposer_reforme` | N0 | `ressources.inventaire_gerer` |
| `ressources.lire_piece` | N1 | `ressources.interventions_traiter` |
| `ressources.rediger_convention` | N0 | `ressources.prets_gerer` |
| `ressources.resume_securite` | N0 | `ressources.securite` |

### 18.4 Cadre
Jeu d'évaluation (20 cas) ; tests de garde : injection dans la description d'une demande, demande de « qui a eu le plus de pertes », facture frauduleuse ; aucune donnée interdite dans un contexte (test).

---

## 19. Actions métier (RSM-100 à RSM-249)

Colonnes : acteur · droit D-017 (§21.1) ; règles ; événement émis ; lot. ★ = réauthentification récente. Acteurs : DIR direction ; GES gestionnaire (adjoint gestionnaire, DAF) ; CPT agent comptable, comptabilité ; AGT agent technique ; NUM référent numérique ; DOC professeur documentaliste, responsable de laboratoire ; PRF professeur ; ADM secrétariat ; ACC accueil ; CPE ; SUR surveillant ; TIT titulaire ; ELV élève ; MAJ élève majeur ; REC réceptionnaire de reprographie ; SYS système.

### 19.1 Catalogue, biens, inventaire
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-100 | Créer, modifier une catégorie et un modèle | GES · `ressources.inventaire_gerer` | attributs par catégorie | `ressources.modele_modifie` | RM1 |
| RSM-101 | Entrer des biens par lot (livraison), imprimer les étiquettes | GES, NUM · idem | unicité de l'étiquette ; financeur, propriétaire | `ressources.bien_entre` | RM1 |
| RSM-102 | Importer des biens (tableur fournisseur) | GES · idem | aperçu, rejets ; rien de créé sans validation | idem | RM1 |
| RSM-103 | Affecter un bien à une salle ou un service | GES, AGT, NUM · idem | salle du 5 | `ressources.bien_affecte` | RM1 |
| RSM-104 | Déplacer un bien (scan à l'arrivée) | AGT, NUM · idem | fait daté | `ressources.bien_deplace` | RM1 |
| RSM-105 | Voir la fiche d'un bien (vie complète) | personnel · `ressources.lire` | détenteur : nom visible des gestionnaires seulement | — | RM1 |
| RSM-106 | Voir les biens d'une salle | personnel · `ressources.lire` | | — | RM1 |
| RSM-107 | Calculer les équipements d'une salle pour le 5 et le 19 | SYS | R-RM5 | `ressources.equipements_salle` | RM1 |
| RSM-108 | Lancer une campagne d'inventaire (site, bâtiment, salles) | GES · idem | | `ressources.inventaire_ouvert` | RM1 |
| RSM-109 | Scanner une salle (mobile, hors ligne) | AGT, NUM · idem | présent, absent, déplacé, inconnu | — | RM1 |
| RSM-110 | Traiter les écarts (confirmer un déplacement, déclarer une perte, enregistrer un inconnu) | GES · idem | jamais automatique | `ressources.ecart_traite` | RM1 |
| RSM-111 | Clore l'inventaire, rapport | GES · idem | document du 7 | `ressources.inventaire_clos` | RM1 |
| RSM-112 | Proposer une sortie d'inventaire (lot, motif) | GES · idem | | `ressources.reforme_proposee` | RM1 |
| RSM-113 | Enregistrer la décision (délibération, décision de l'organisme ou de la collectivité propriétaire) et sortir les biens | GES, CPT · `ressources.reformer` | R-RM8 ; effacement attesté pour le numérique | `ressources.bien_sorti` | RM1 |
| RSM-114 | Exporter les biens immobilisés (48) | CPT · `ressources.inventaire_gerer` | seuil S-4400 | — | RM1 |
| RSM-115 | Attester l'effacement des données d'un appareil | NUM · idem | méthode, auteur | `ressources.bien_efface` | RM1 |
| RSM-116 | Suivre garantie et fin de support (rappels) | SYS → NUM | | — | RM1 |
| RSM-117 | Déclarer un bien perdu, volé, retrouvé | GES, NUM | | `ressources.bien_perdu` | RM1 |
| RSM-118 | Rechercher un bien (étiquette, série, modèle, salle) | personnel | | — | RM1 |
| RSM-119 | Gérer un lot en quantité (entrée, sortie, stock) | GES · idem | | `ressources.lot_mouvement` | RM1 |

### 19.2 Matériel réservable
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-120 | Déclarer un bien ou une catégorie réservable au 19 (quantité, règle, retrait) | GES, NUM · `ressources.inventaire_gerer` | | `agenda.ressource_declaree` (19) | RM3 |
| RSM-121 | Réserver du matériel (seul ou avec une salle) | PRF, personnel · droit du 19 | disponibilité réelle (panne) | `agenda.reservation_*` | RM3 |
| RSM-122 | Retirer le matériel réservé (prêt court, scan) | réservant, ACC | | `ressources.pret_ouvert` | RM3 |
| RSM-123 | Rendre le matériel réservé, constat | réservant, ACC | | `ressources.pret_solde` | RM3 |
| RSM-124 | Mettre en conflit les réservations d'un bien tombé en panne | SYS | avis, remplacement proposé | `agenda.reservation_conflit` | RM3 |
| RSM-125 | Tenir le carnet de bord du minibus | conducteur | km, carburant, incidents | `ressources.carnet` | RM3 |
| RSM-126 | Vérifier le conducteur autorisé (permis enregistré au 6) | SYS | CRM-18 | — | RM3 |
| RSM-127 | Avertir à la réservation d'une salle d'une intervention ouverte sur son équipement | SYS | port `interventions_ouvertes` | — | RM3 |

### 19.3 Prêts individuels
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-128 | Définir les règles de prêt d'une catégorie (convention, durée, signature, garantie) | GES · `ressources.prets_gerer` | R-RM12, S-4405 | — | RM2 |
| RSM-129 | Prêter un bien à une personne (scan, constat de remise) | GES, ACC, DOC, NUM · idem | convention si exigée | `ressources.pret_ouvert` | RM2 |
| RSM-130 | Faire signer la convention par le bon signataire (titulaire, majeur, personnel) | SYS → TIT, MAJ | 7, 18 ; un titulaire suffit | `documents.signe` | RM2 |
| RSM-131 | Rendre un bien (scan, constat de retour comparé) | GES, ACC, DOC · idem | | `ressources.pret_solde` | RM2 |
| RSM-132 | Constater une dégradation (photo, codes) | idem | R-RM14 | `ressources.degradation` | RM2 |
| RSM-133 | Rappeler un retard de retour | SYS | S-4407 | — | RM2 |
| RSM-134 | Déclarer une perte ou un vol (famille, élève, personnel) | TIT, MAJ, personnel · `soi` | pièce facultative | `ressources.perte_declaree` | RM2 |
| RSM-135 | Décider : facturer au barème, remise gracieuse (motif), recherche | GES · `ressources.facturer` | N2 | `ressources.perte_decidee` | RM6 |
| RSM-136 | Voir ses prêts et ce qu'il faut rendre | TIT, ELV, MAJ, personnel · `soi` | | — | RM2 |
| RSM-137 | Solder les prêts au départ d'un élève (3) | ADM, GES | liste ; « à recouvrer » si non rendus | `ressources.prets_soldes` | RM2 |
| RSM-138 | Solder les prêts et clés au départ d'un agent (6) | GES, ACC | tâche bloquante | idem | RM2 |
| RSM-139 | Prêter du matériel au porteur d'une sortie (42) | GES, DOC | retour à la date de fin | `ressources.pret_ouvert` | RM2 |

### 19.4 Manuels et campagnes
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-140 | Créer les modèles de manuels (ISBN, édition, discipline, niveau) | GES, DOC · `ressources.prets_gerer` | | — | RM2 |
| RSM-141 | Définir les listes par niveau et enseignement suivi (5) | GES · idem | | — | RM2 |
| RSM-142 | Prévoir la commande (effectifs × listes − stock) | GES (IA N0) | | — | RM8 |
| RSM-143 | Entrer et étiqueter une livraison de manuels avec l'année de mise en service | GES | | `ressources.bien_entre` | RM2 |
| RSM-144 | Préparer une campagne de distribution (élèves, listes, conventions) | GES · idem | | `ressources.campagne_ouverte` | RM2 |
| RSM-145 | Envoyer les conventions avant la distribution (18) | GES | N3 | `formulaires.*` | RM2 |
| RSM-146 | Distribuer par classe en mode « caisse » (mobile) | GES, ADM, PRF | scan élève + étiquettes | `ressources.pret_ouvert` (par exemplaire) | RM2 |
| RSM-147 | Remettre un reçu dans l'espace famille | SYS | | — | RM2 |
| RSM-148 | Échanger un manuel (changement d'option) | ADM | | idem | RM2 |
| RSM-149 | Organiser le retour de fin d'année par classe (créneaux du 19) | GES | | `ressources.campagne_retour` | RM2 |
| RSM-150 | Rendre en mode « caisse », constats | GES, ADM | | `ressources.pret_solde` | RM2 |
| RSM-151 | Calculer les pertes au barème dégressif | SYS | R-RM13 | — | RM6 |
| RSM-152 | Enregistrer le barème et sa délibération ou décision | GES, DIR · `ressources.bareme` | S-4404 ; ★ | `ressources.bareme_enregistre` | RM6 |
| RSM-153 | Transmettre les pertes au 24 (payeur, entité, affectation) | SYS | N2 par le gestionnaire | `facturation.*` | RM6 |
| RSM-154 | Contester une facturation de perte ou de dégradation | TIT, MAJ · `soi` | délai S-4408 | `ressources.contestation` | RM6 |
| RSM-155 | Revoir une contestation | GES · `ressources.facturer` | avoir (24) | `ressources.contestation_traitee` | RM6 |
| RSM-156 | Réformer les manuels usés en lot | GES | §8.5 | `ressources.reforme_proposee` | RM2 |
| RSM-157 | Préparer la campagne des ordinateurs de la collectivité (convention, charte de la collectivité) | GES, NUM | | `ressources.campagne_ouverte` | RM2 |
| RSM-158 | Exporter le suivi des prêts pour la collectivité propriétaire | GES | minimal | — | RM2 |
| RSM-159 | Voir le tableau de la campagne (distribués, rendus, manquants) | GES, DIR | | — | RM2 |

### 19.5 Demandes d'intervention et maintenance
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-160 | Signaler (scan QR ou salle, catégorie, description, photo) | personnel · `ressources.signaler` | doublon proposé | `ressources.demande_creee` | RM4 |
| RSM-161 | Ajouter son signalement à une demande existante | personnel | compteur, abonné | `ressources.demande_appuyee` | RM4 |
| RSM-162 | Signaler pour un élève (surveillant, CPE) | SUR, CPE | sans nom d'élève | idem | RM4 |
| RSM-163 | Qualifier (catégorie, urgence, sécurité) | GES, AGT (IA N1) | | `ressources.demande_qualifiee` | RM4 |
| RSM-164 | Déclencher l'alerte de sécurité | SYS sur qualification | direction, astreinte | `ressources.alerte_securite` | RM4 |
| RSM-165 | Condamner un lieu (indisponibilité du 5) | GES, DIR · `ressources.condamner` | N2 | `referentiel.salle_indisponible` (5) | RM4 |
| RSM-166 | Lever la condamnation après vérification | GES, DIR | | idem (annulation) | RM4 |
| RSM-167 | Router vers un agent, une équipe, le numérique | GES · `ressources.interventions_router` | mission du 6 | `ressources.demande_routee` | RM4 |
| RSM-168 | Transmettre à la collectivité ou à l'organisme de gestion (qualifiée, sans nom) | GES · idem | N3 | `ressources.demande_transmise` | RM4 |
| RSM-169 | Transmettre à un prestataire sous contrat | GES · idem | N3 | idem | RM4 |
| RSM-170 | Prendre en charge, planifier | AGT, NUM · `ressources.interventions_traiter` | avertissement si cours (10) | `ressources.intervention_planifiee` | RM4 |
| RSM-171 | Saisir une intervention (temps, pièces, coût, facture) | AGT, NUM, GES | | `ressources.intervention_saisie` | RM4 |
| RSM-172 | Mettre en attente (motif) | AGT, GES | | `ressources.demande_attente` | RM4 |
| RSM-173 | Résoudre | AGT, NUM | | `ressources.demande_resolue` | RM4 |
| RSM-174 | Vérifier et clore (demandeur ou direction pour la sécurité) | demandeur, DIR | | `ressources.demande_close` | RM4 |
| RSM-175 | Rouvrir dans les 30 jours | demandeur | | `ressources.demande_rouverte` | RM4 |
| RSM-176 | Voir mes demandes et leur état | personnel · `soi` | | — | RM4 |
| RSM-177 | Voir la liste du jour par bâtiment (agent, mobile) | AGT | | — | RM4 |
| RSM-178 | Créer des tâches de maintenance préventive (modèles récurrents) | GES | | `ressources.preventif_cree` | RM9 |
| RSM-179 | Voir les délais et dépassements | GES, DIR | S-4411 | — | RM4 |

### 19.6 Sécurité des locaux
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-180 | Déclarer les équipements soumis à vérification et leurs modèles (périodicité saisie) | GES · `ressources.securite` | | — | RM7 |
| RSM-181 | Enregistrer une vérification (organisme, date, résultat, rapport) | GES | document du 7 | `ressources.verification` | RM7 |
| RSM-182 | Créer une observation à lever (responsable, échéance) | GES | une par observation | `ressources.observation` | RM7 |
| RSM-183 | Lever une observation (justificatif) | GES, AGT | | `ressources.observation_levee` | RM7 |
| RSM-184 | Enregistrer un passage de la commission de sécurité et son avis | DIR, GES | | `ressources.commission` | RM7 |
| RSM-185 | Tenir les rubriques du registre (travaux, personnel de sécurité, consignes) | GES, DIR | R143-44 | — | RM7 |
| RSM-186 | Recevoir les exercices du 14 dans le registre | SYS | | — | RM7 |
| RSM-187 | Imprimer ou exporter le registre de sécurité | DIR, GES | | — | RM7 |
| RSM-188 | Rappeler les échéances (J-30) et signaler les dépassements | SYS | | — | RM7 |
| RSM-189 | Voir le tableau « sécurité » | DIR, GES | | — | RM7 |
| RSM-200 | Inscrire un danger grave et imminent au registre dédié (depuis une demande ou directement) | personnel | accès restreint | `ressources.dgi` | RM7 |

### 19.7 Clés, badges, casiers, états des lieux
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-190 | Décrire l'organigramme (cylindres, clés, ouvertures) | GES · `ressources.cles_gerer` | | — | RM5 |
| RSM-191 | Remettre une clé (signature, retour attendu) | ACC, GES · idem | passe général : DIR ★ | `ressources.cle_remise` | RM5 |
| RSM-192 | Restituer une clé | ACC, GES | | `ressources.cle_rendue` | RM5 |
| RSM-193 | Déclarer une clé perdue ; proposer le changement de cylindres | détenteur, GES | intervention | `ressources.cle_perdue` | RM5 |
| RSM-194 | Voir « clés dehors » (par personne, par salle) | GES, ACC, DIR | | — | RM5 |
| RSM-195 | Prêter, rendre un badge d'accès ; demander la désactivation | GES, ACC | pas de journal de passages | `ressources.badge_*` | RM5 |
| RSM-196 | Attribuer, libérer un casier (code changé à la libération) | CPE, ADM · `ressources.casiers` | | `ressources.casier_*` | RM5 |
| RSM-197 | Campagne d'attribution des casiers | CPE | tirage si besoin | — | RM5 |
| RSM-198 | Faire un état des lieux d'entrée ou de sortie (photos, signatures) | SUR, GES, CPE · `ressources.etats_lieux` | modèle par type | `ressources.etat_des_lieux` | RM5 |
| RSM-199 | Comparer entrée et sortie, constater, facturer au barème | GES | 24 | `ressources.degradation` | RM6 |

### 19.8 Reprographie
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-225 | Déposer les PDF d'une demande (NRM-01) | PRF, personnel · `ressources.reprographie_demander` | 7 ; taille ; identifiant serveur | `documents.depose` | RM5 |
| RSM-226 | Créer une demande (site, exemplaires, options, classes, œuvres) (NRM-02) | idem | seuils S-4410 ; avertissement CFC | `ressources.repro_demandee` | RM5 |
| RSM-227 | Avis à la direction du site et à la file (NRM-02) | SYS | lien, jamais pièce jointe | — | RM5 |
| RSM-228 | Déposer pour le compte d'un enseignant (NRM-03) | ADM, ACC, CPT, DIR · `ressources.reprographie_deposer` | cible avec service (6) | idem | RM5 |
| RSM-229 | Voir ses demandes, ouvrir un PDF (NRM-04, NRM-09) | demandeur, déposant | lien court journalisé | — | RM5 |
| RSM-230 | Décider (accepter, refuser, note) (NRM-05) | DIR du site, délégué · `ressources.reprographie_valider` | une fois | `ressources.repro_decidee` | RM5 |
| RSM-231 | Voir la file du site (NRM-04) | REC · `ressources.reprographie_traiter` (son site) | R-RM27 | — | RM5 |
| RSM-232 | Marquer prête, lieu de retrait (NRM-06) | REC | avis | `ressources.repro_prete` | RM5 |
| RSM-233 | Acquitter « prête » ; marquer retirée (NRM-08) | demandeur | | — | RM5 |
| RSM-234 | Annuler sa demande avant impression | demandeur | | `ressources.repro_annulee` | RM5 |
| RSM-235 | Relever les compteurs d'un copieur | REC, GES | | `ressources.compteur` | RM5 |
| RSM-236 | Voir les compteurs par service, discipline, site | GES, DIR | | — | RM5 |
| RSM-237 | Activer la période de déclaration CFC, pré-remplir, exporter | GES · `ressources.reprographie_gerer` | | — | RM10 |
| RSM-238 | Effacer les PDF à J+30 après retrait | SYS | §21.3 | — | RM5 |
| RSM-239 | Créer une demande par l'assistant (NRM-10) | demandeur (IA N1) | même commande | idem | RM11 |

### 19.9 Locaux pour tiers, mise en place, système
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-240 | Créer une convention de mise à disposition (tiers, salles, créneaux, période) | GES, DIR · `ressources.conventions` | 7 ; avis du CA (public) | `ressources.convention` | RM9 |
| RSM-241 | Vérifier la convention à la réservation d'un tiers (19) | SYS | R-RM30 | — | RM9 |
| RSM-242 | Générer les tâches de mise en place depuis les besoins d'une réservation (19) | SYS | J-1 (S-4412) | `ressources.mise_en_place` | RM3 |
| RSM-243 | Marquer une mise en place faite | AGT | | idem | RM3 |
| RSM-244 | Tableau de bord (parc, prêts en cours, demandes, sécurité, reprographie) | DIR, GES · `ressources.lire` | | — | RM10 |
| RSM-245 | Indicateurs pour le 47 | SYS | | — | RM10 |
| RSM-246 | Paramétrer (réglages §28) | DIR, GES · `etablissement.parametrer` | | — | RM1 |
| RSM-247 | Reprendre les photocopies de Next.js | SYS | §26 | — | RM11 |
| RSM-248 | Répondre à une demande d'accès RGPD (prêts, constats) | DPO | | — | RM12 |
| RSM-249 | Purger selon les durées (§21.3) | SYS | | — | RM12 |

Les numéros RSM-201 à RSM-224 sont **réservés** (non attribués) pour garder des tranches lisibles.

---

## 20. Règles transverses (R-RM1 à R-RM34) [R]

| R | Règle |
|---|---|
| R-RM1 | Un bien a une étiquette unique par établissement, un emplacement ou un détenteur, un état calculé depuis des faits en ajout seul. |
| R-RM2 | Aucune suppression de bien : sortie datée avec décision. |
| R-RM3 | Un bien de la collectivité ou de l'organisme est sorti par décision de son propriétaire. |
| R-RM4 | Un appareil numérique n'est réaffecté ou sorti qu'après effacement attesté. |
| R-RM5 | Les étiquettes d'équipement d'une salle (5) sont calculées depuis les biens en service quand le 44 est actif. |
| R-RM6 | Un bien en panne n'est pas réservable ; ses réservations passent en conflit. |
| R-RM7 | Un inventaire ne corrige jamais seul un écart. |
| R-RM8 | Une sortie d'inventaire exige la référence de la décision (délibération, organisme, collectivité). |
| R-RM9 | Un prêt se solde (rendu, abîmé, perdu, racheté, remis) ; il ne disparaît pas. |
| R-RM10 | Un prêt à un mineur avec convention est signé par un titulaire (acte usuel) ; l'autre est informé ; le payeur d'une perte est celui du 4. |
| R-RM11 | Aucun nom de détenteur n'est visible d'un autre que les gestionnaires et le détenteur. |
| R-RM12 | **Aucune caution** sur un prêt de manuel. |
| R-RM13 | Manuel perdu : barème dégressif par année de mise en service (100 / 50 / 30 / 20 %) sauf décision enregistrée ; sommes affectées au renouvellement. |
| R-RM14 | Sans constat de remise, aucune dégradation n'est facturée ; l'usure normale ne l'est jamais. |
| R-RM15 | Aucune rétention de document scolaire pour une dette de prêt. |
| R-RM16 | Toute facturation de perte ou de dégradation est proposée par le système et décidée par le gestionnaire (N2), contestable. |
| R-RM17 | Un départ d'élève n'est pas clos sans prêts soldés ou marqués « à recouvrer ». |
| R-RM18 | Un appareil prêté n'est jamais géolocalisé ni lu. |
| R-RM19 | Une demande d'intervention de sécurité déclenche une alerte et propose une condamnation. |
| R-RM20 | Le départ d'un agent crée une tâche bloquante « clés, badges, matériel ». |
| R-RM21 | Un passe général n'est remis que sur décision de la direction (★). |
| R-RM22 | Le 44 n'importe aucun journal de passages de badges. |
| R-RM23 | Un code de casier n'est jamais stocké en clair. |
| R-RM24 | Une demande transmise à l'extérieur ne contient aucun nom d'élève. |
| R-RM25 | Une demande n'est close qu'après vérification pour la sécurité ; elle se rouvre dans les 30 jours. |
| R-RM26 | Une observation de vérification ou de commission est une tâche avec échéance. |
| R-RM27 | La file de reprographie est **par site** ; le réceptionnaire est désigné, jamais un rôle entier par défaut. |
| R-RM28 | Un PDF de reprographie est désigné par un identifiant émis par le serveur, lié au déposant ; il n'est jamais envoyé en pièce jointe. |
| R-RM29 | La validation de la direction s'applique à la couleur et au-delà d'un volume, quel que soit le type déclaré. |
| R-RM30 | Une réservation par un tiers exige une convention valide couvrant la salle et le créneau. |
| R-RM31 | Toute commande passe par le serveur (écran, mobile, IA). |
| R-RM32 | Les lectures de détenteurs et de constats sont journalisées. |
| R-RM33 | Un état des lieux est signé par les deux parties ou contradictoire avec un second personnel. |
| R-RM34 | Toute action par lien (avis par courriel) ouvre une page puis exige un bouton ; jamais un GET qui modifie. |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]
| Action | Portée | Rôles par défaut |
|---|---|---|
| `ressources.lire` | établissement | personnels (sans détenteurs) |
| `ressources.inventaire_gerer` | établissement ; ses catégories | gestionnaire, référent numérique |
| `ressources.reformer` | établissement | gestionnaire + agent comptable |
| `ressources.prets_gerer` | établissement ; ses catégories (laboratoire, CDI) | gestionnaire, secrétariat, documentaliste, responsable de laboratoire |
| `ressources.facturer`, `ressources.bareme` | établissement | gestionnaire ; direction pour le barème |
| `ressources.signaler` | établissement | tous les personnels, intervenants (leur lieu) |
| `ressources.interventions_router`, `ressources.interventions_traiter` | établissement ; son équipe | gestionnaire ; agents, référent numérique |
| `ressources.condamner` | établissement | direction, gestionnaire |
| `ressources.securite` | établissement | direction, gestionnaire, assistant de prévention |
| `ressources.cles_gerer` | établissement | gestionnaire, accueil |
| `ressources.casiers`, `ressources.etats_lieux` | établissement ; internat | CPE, vie scolaire, surveillants d'internat |
| `ressources.reprographie_demander`, `_deposer`, `_valider` (son site), `_traiter` (son site), `_gerer` | site | professeurs et personnels ; administratifs ; direction du site ; réceptionnaire ; gestionnaire |
| `ressources.conventions` | établissement | direction, gestionnaire |

### 21.2 Ce que voit chacun (projections) [R]
| Qui | Voit | Ne voit pas |
|---|---|---|
| Personnel quelconque | biens d'une salle, état, disponibilité ; ses prêts, ses demandes | détenteurs des autres, constats des autres |
| Agent technique | demandes routées vers lui, lieu, photo, demandeur (pour le joindre) | prêts, détenteurs |
| Collectivité, prestataire | demande qualifiée (lieu, catégorie, description, photo) | tout nom de personne sauf le contact désigné |
| Famille | prêts de ses enfants, conventions, constats, factures | autres familles |
| Élève | ses prêts, ce qu'il doit rendre | — |
| Réceptionnaire | demandes de son site acceptées, PDF | les autres sites, les demandes en attente de décision |
| Direction du site | demandes de son site | les autres sites |

### 21.3 Conservation [R, durées soumises au DPO]
| Donnée | Durée proposée |
|---|---|
| Biens et leur vie (sans détenteur) | durée de vie du bien + 5 ans après sortie |
| Prêts et constats (avec détenteur) | 1 an après le solde ; 5 ans si facturé (durée comptable du 24) |
| Conventions de prêt | 1 an après le solde |
| Demandes d'intervention | 3 ans ; sécurité : 10 ans (cohérence avec le registre) [R] |
| Registre de sécurité, vérifications, observations | 10 ans [R, à confirmer] |
| Clés, badges (détention) | 1 an après restitution |
| Codes de casier | jamais stockés |
| PDF de reprographie | effacés 30 jours après retrait ou refus |
| Demandes de reprographie (sans PDF) | 1 an ; compteurs agrégés 5 ans |
| Conventions de mise à disposition | 5 ans après la fin |

---

## 22. Événements
`ressources.modele_modifie`, `bien_entre`, `bien_affecte`, `bien_deplace`, `bien_perdu`, `bien_efface`, `bien_sorti`, `lot_mouvement`, `equipements_salle` ; `ressources.inventaire_ouvert`, `ecart_traite`, `inventaire_clos`, `reforme_proposee` ; `ressources.pret_ouvert`, `pret_solde`, `prets_soldes`, `degradation`, `perte_declaree`, `perte_decidee`, `contestation`, `contestation_traitee`, `campagne_ouverte`, `campagne_retour`, `bareme_enregistre` ; `ressources.demande_creee`, `demande_appuyee`, `demande_qualifiee`, `alerte_securite`, `demande_routee`, `demande_transmise`, `intervention_planifiee`, `intervention_saisie`, `demande_attente`, `demande_resolue`, `demande_close`, `demande_rouverte`, `preventif_cree` ; `ressources.verification`, `observation`, `observation_levee`, `commission`, `dgi` ; `ressources.cle_remise`, `cle_rendue`, `cle_perdue`, `badge_*`, `casier_*`, `etat_des_lieux` ; `ressources.repro_demandee`, `repro_decidee`, `repro_prete`, `repro_annulee`, `compteur` ; `ressources.convention`, `mise_en_place`, `carnet`. Le 41 attend `ressources.intervention_*`, `ressources.etat_des_lieux` et `ressources.pret_*` (INT-188, 227, 228) : ils sont ici. Publiés dans la transaction du fait (outbox).

Natures de notifications (8) : `ressources.pret_retard` (rappel), `ressources.perte_facturee` (information, contestation), `ressources.alerte_securite` (urgent, direction et astreinte), `ressources.demande_suivi` (récapitulatif quotidien), `ressources.observation_echeance` (rappel), `ressources.repro_decision`, `ressources.repro_prete`, `ressources.cle_a_rendre` (départ).

## 23. Écrans et mobile
| Écran | Pour qui | Contenu |
|---|---|---|
| **Ressources** (tuiles Bento) | direction, gestionnaire | parc par catégorie, prêts en cours et en retard, demandes ouvertes par urgence, sécurité (échéances, observations), reprographie du jour |
| **Bien** | gestionnaire, numérique | fiche, vie, prêts, pannes, garantie |
| **Salle** (depuis le 5) | personnels | biens de la salle, état, « signaler une panne » |
| **Inventaire** (mobile) | agents | campagne, scan, écarts |
| **Campagne de prêt** (mobile, mode caisse) | gestionnaire, secrétariat | scan élève, scan exemplaires, constats |
| **Mes prêts** | familles, élèves, personnels | à rendre, conventions, factures |
| **Signaler** (mobile, depuis le QR code de la porte) | tous les personnels | catégorie, photo, doublon |
| **Interventions** | gestionnaire, agents | file par urgence, liste du jour par bâtiment |
| **Sécurité** | direction | registre, vérifications, observations, commission |
| **Clés** | gestionnaire, accueil | organigramme, clés dehors, remise et retour |
| **Reprographie** | demandeurs ; direction du site ; réceptionnaire | demande, décision, file, compteurs |

## 24. Modèle de données (sans JSON, D-036 ; E1–E14)

Règles communes : `id UUID PRIMARY KEY` généré par l'application (E3) ; `etablissement_id UUID NOT NULL` avec le contrôle « non nul uuid » (E1) ; `UNIQUE (etablissement_id, id)` et **FK composites** (E2) ; `cree_le`, `cree_par`, `modifie_le` (E6) ; `version` sur les objets modifiables ; faits en **ajout seul** (`ent_ajout_seul()`) ; RLS par `ent_rls_poser` (E13) ; aucune colonne `jsonb`, `json`, `bytea`, aucun tableau (E5, E10) ; photos, constats, conventions, rapports, PDF de reprographie = objets `ent_objet` (500) ; aucun nom de personne dans un nom d'objet ni dans un journal applicatif (E14) ; registre des tables (E12).

```
-- Biens (RM1 ; 2880–2881)
ent_ressource_categorie (id, etablissement_id NULL -- NULL = référence globale, code, libelle,
  suivi CHECK (unitaire|quantite), serie_requise BOOLEAN, convention_requise BOOLEAN, reservable_defaut BOOLEAN)
ent_ressource_modele (id, etablissement_id, categorie_id, libelle, reference TEXT NULL, isbn TEXT NULL,
  discipline_code NULL, niveau_id NULL, edition TEXT NULL, prix_centimes BIGINT NULL, duree_usage_mois INT NULL,
  fournisseur_id NULL -- annuaire commun (ent_partenaire / prestataire), cree_le, modifie_le)
ent_ressource_bien (id, etablissement_id, modele_id, etiquette TEXT, numero_serie TEXT NULL,
  entre_le DATE, mise_en_service_annee SMALLINT NULL, valeur_centimes BIGINT NULL, immobilise BOOLEAN,
  financeur CHECK (etat|collectivite|etablissement|organisme_gestion|don|autre),
  proprietaire CHECK (etablissement|collectivite|organisme_gestion), garantie_au DATE NULL, fin_support_au DATE NULL,
  reservable BOOLEAN DEFAULT false, version, cree_le, modifie_le,
  UNIQUE (etablissement_id, etiquette), UNIQUE (etablissement_id, modele_id, numero_serie))
ent_ressource_bien_fait (id, etablissement_id, bien_id, a TIMESTAMPTZ, type CHECK (entre|affecte|deplace|prete|rendu|
  en_panne|en_reparation|repare|perdu|vole|retrouve|efface|propose_reforme|reforme|sorti),
  salle_id NULL, lieu_id NULL, service_code NULL, pret_id NULL, intervention_id NULL, decision_objet_id NULL,
  methode_code NULL, motif_code NULL, acteur_personne_id) -- ajout seul ; l'état courant est une vue
ent_ressource_lot (id, etablissement_id, modele_id, salle_id NULL, quantite INT CHECK (quantite >= 0), version)
ent_ressource_lot_mouvement (id, etablissement_id, lot_id, a, delta INT, motif_code, acteur_personne_id) -- ajout seul

-- Inventaire (RM1 ; 2882)
ent_ressource_inventaire (id, etablissement_id, perimetre CHECK (site|batiment|salles), site_id NULL, batiment_id NULL,
  ouvert_le, clos_le NULL, rapport_objet_id NULL)
ent_ressource_inventaire_salle (id, etablissement_id, inventaire_id, salle_id)
ent_ressource_inventaire_lecture (id, etablissement_id, inventaire_id, salle_id, etiquette TEXT, bien_id NULL,
  lu_le, lu_par) -- ajout seul ; l'écart est calculé
ent_ressource_ecart (id, etablissement_id, inventaire_id, bien_id NULL, etiquette TEXT NULL,
  type CHECK (absent|deplace|inconnu), traitement CHECK (deplacement_confirme|perte_declaree|enregistre|retrouve) NULL,
  traite_par NULL, traite_le NULL)
ent_ressource_sortie_decision (id, etablissement_id, type CHECK (deliberation_ca|decision_organisme|decision_collectivite),
  reference TEXT, date_decision DATE, objet_id)
ent_ressource_sortie_bien (id, etablissement_id, decision_id, bien_id, devenir CHECK (rebut|d3e|don|transfert|desaffectation))

-- Prêts et manuels (RM2 ; 2883–2884)
ent_ressource_regle_pret (id, etablissement_id, categorie_id, duree_jours INT NULL, retour_fin_annee BOOLEAN,
  convention_trame_code NULL, signature_requise BOOLEAN, garantie_autorisee BOOLEAN DEFAULT false
  -- refusée par contrainte pour la catégorie « manuel » (R-RM12))
ent_ressource_campagne (id, etablissement_id, type CHECK (distribution|retour), annee_id, libelle, ouverte_le, close_le NULL)
ent_ressource_campagne_cible (id, etablissement_id, campagne_id, classe_id NULL, niveau_id NULL)
ent_ressource_liste_niveau (id, etablissement_id, annee_id, niveau_id, offre_id NULL -- enseignement suivi du 5, modele_id)
ent_ressource_pret (id, etablissement_id, bien_id NULL, lot_id NULL, quantite INT DEFAULT 1,
  emprunteur_eleve_id NULL, emprunteur_personne_id NULL, campagne_id NULL, sortie_id NULL -- 42,
  reservation_id NULL -- 19, ouvert_le, retour_attendu_le DATE NULL, etat CHECK (ouvert|rendu|rendu_abime|perdu|vole|rachete|remis|a_recouvrer),
  convention_objet_id NULL, signature_id NULL, version, cree_le, modifie_le,
  CHECK (num_nonnulls(bien_id, lot_id) = 1), CHECK (num_nonnulls(emprunteur_eleve_id, emprunteur_personne_id) = 1))
  -- un seul prêt ouvert par bien : index unique partiel (etablissement_id, bien_id) WHERE etat = 'ouvert'
ent_ressource_constat (id, etablissement_id, pret_id NULL, etat_des_lieux_id NULL, moment CHECK (remise|retour|entree|sortie),
  etat CHECK (neuf|bon|use|abime|hors_service|manquant), objet_id NULL -- photo, auteur_personne_id, cree_le)
ent_ressource_constat_point (id, etablissement_id, constat_id, code -- élément (chargeur, écran, couverture, mur, literie),
  etat CHECK (...), note_code NULL)
ent_ressource_bareme (id, etablissement_id, categorie_id, decision_type CHECK (deliberation_ca|decision_organisme|defaut_national),
  reference TEXT NULL, date_decision DATE NULL, objet_id NULL, du DATE, au DATE NULL)
ent_ressource_bareme_palier (id, etablissement_id, bareme_id, annee_usage SMALLINT, pourcentage SMALLINT CHECK (0..100))
ent_ressource_facturation (id, etablissement_id, pret_id NULL, etat_des_lieux_id NULL, bareme_id, montant_base_centimes BIGINT,
  pourcentage SMALLINT, decision CHECK (proposee|facturee|remise_gracieuse|annulee), decide_par NULL, motif_code NULL,
  facturation_source TEXT NULL -- référence au 24, conteste_le NULL, contestation_issue CHECK (maintenue|reduite|annulee) NULL)
ent_ressource_declaration_perte (id, etablissement_id, pret_id, declare_par_personne_id, declare_le, nature CHECK (perte|vol),
  piece_objet_id NULL)

-- Matériel réservable, mise en place (RM3 ; 2885)
ent_ressource_reservable (id, etablissement_id, bien_id NULL, categorie_id NULL, quantite INT NULL, retrait_salle_id NULL,
  ressource_agenda_id -- identifiant au moteur du 19)
ent_ressource_carnet (id, etablissement_id, bien_id, a, conducteur_personne_id, km_depart INT, km_retour INT NULL,
  incident_code NULL) -- minibus
ent_ressource_mise_en_place (id, etablissement_id, reservation_id -- 19, besoin_code, quantite INT NULL, a_faire_le DATE,
  fait_le NULL, fait_par NULL, annulee_le NULL)

-- Interventions (RM4 ; 2886–2887)
ent_ressource_demande (id, etablissement_id, salle_id NULL, lieu_id NULL, bien_id NULL, categorie CHECK (electricite|plomberie|
  chauffage|menuiserie|serrurerie|informatique|audiovisuel|menage|espaces_verts|securite_incendie|autre),
  description VARCHAR(1000), urgence CHECK (securite|haute|normale|basse), securite BOOLEAN,
  etat CHECK (ouverte|qualifiee|routee|transmise|planifiee|en_attente|resolue|close|rejetee|doublon),
  doublon_de_id NULL, demandeur_personne_id, destinataire_type CHECK (agent|equipe|numerique|collectivite|organisme|prestataire) NULL,
  destinataire_ref TEXT NULL, version, cree_le, modifie_le)
ent_ressource_demande_appui (id, etablissement_id, demande_id, personne_id, cree_le, UNIQUE (etablissement_id, demande_id, personne_id))
ent_ressource_demande_piece (id, etablissement_id, demande_id, objet_id)
ent_ressource_demande_fait (id, etablissement_id, demande_id, a, type CHECK (creee|qualifiee|alerte|condamnation|routee|
  transmise|prise|planifiee|attente|resolue|verifiee|close|rouverte|rejetee), motif_code NULL, acteur_personne_id,
  indisponibilite_id NULL) -- ajout seul
ent_ressource_intervention (id, etablissement_id, demande_id NULL, preventif_id NULL, intervenant_personne_id NULL,
  prestataire_id NULL, debut, fin NULL, minutes INT NULL, cout_centimes BIGINT NULL, facture_objet_id NULL, resultat_code NULL)
ent_ressource_intervention_piece (id, etablissement_id, intervention_id, libelle, quantite INT, cout_centimes BIGINT NULL)
ent_ressource_preventif (id, etablissement_id, bien_id NULL, salle_id NULL, libelle, periodicite_jours INT, prochaine_le DATE)

-- Sécurité (RM7 ; 2888)
ent_ressource_verification_modele (id, etablissement_id, objet CHECK (extincteurs|alarme|desenfumage|electricite|gaz|ascenseur|
  portes_automatiques|equipements_sportifs|hottes|aires_jeux|vehicule|autre), libelle, periodicite_mois INT, organisme_requis BOOLEAN)
ent_ressource_verification (id, etablissement_id, modele_id, bien_id NULL, batiment_id NULL, realisee_le DATE, organisme TEXT NULL,
  resultat CHECK (conforme|observations|non_conforme), rapport_objet_id NULL, prochaine_le DATE)
ent_ressource_observation (id, etablissement_id, verification_id NULL, commission_id NULL, libelle, responsable_personne_id,
  echeance DATE, levee_le NULL, justificatif_objet_id NULL)
ent_ressource_commission (id, etablissement_id, passee_le DATE, type CHECK (periodique|inopinee|reception), avis CHECK (favorable|defavorable),
  pv_objet_id NULL)
ent_ressource_registre_entree (id, etablissement_id, rubrique CHECK (travaux|personnel_securite|consignes|exercice),
  date DATE, libelle, entreprise TEXT NULL, objet_id NULL, exercice_ref TEXT NULL -- référence au 14)
ent_ressource_dgi (id, etablissement_id, signale_le, lieu_salle_id NULL, description VARCHAR(1000), signale_par,
  mesures TEXT NULL, clos_le NULL) -- accès restreint

-- Clés, badges, casiers, états des lieux (RM5 ; 2889)
ent_ressource_cylindre (id, etablissement_id, salle_id NULL, libelle, numero TEXT)
ent_ressource_cle (id, etablissement_id, numero TEXT, type CHECK (passe_general|passe_partiel|cle_salle|autre), bien_id NULL,
  UNIQUE (etablissement_id, numero))
ent_ressource_cle_ouverture (id, etablissement_id, cle_id, cylindre_id)
  -- remise et restitution = ent_ressource_pret (bien de catégorie « clé »)
ent_ressource_casier (id, etablissement_id, salle_id NULL, numero TEXT, public CHECK (eleve|personnel), etat CHECK (actif|hors_service))
ent_ressource_casier_attribution (id, etablissement_id, casier_id, eleve_id NULL, personne_id NULL, du, au NULL, code_remis_le NULL)
  -- aucun code stocké (R-RM23)
ent_ressource_etat_des_lieux (id, etablissement_id, type_local CHECK (chambre_internat|logement|local_prete|casier),
  salle_id NULL, occupant_eleve_id NULL, occupant_personne_id NULL, moment CHECK (entree|sortie), realise_le,
  signataire_1_personne_id, signataire_2_personne_id NULL, contradictoire BOOLEAN, entree_id NULL -- pour la comparaison)

-- Reprographie (RM5 ; 2890)
ent_repro_demande (id, etablissement_id, site_id, demandeur_personne_id, depose_par_personne_id NULL, exemplaires INT CHECK (1..20000),
  couleur BOOLEAN, recto_verso BOOLEAN, agrafage BOOLEAN, souhaitee_le TIMESTAMPTZ NULL, motif VARCHAR(500) NULL,
  contient_oeuvres BOOLEAN, contient_donnees_eleves BOOLEAN,
  etat CHECK (en_attente|acceptee|refusee|prete|retiree|annulee), decide_par NULL, decide_le NULL, note_direction VARCHAR(500) NULL,
  prete_le NULL, prete_par NULL, retrait_lieu TEXT NULL, vue_le NULL, version, cree_le, modifie_le)
ent_repro_demande_document (id, etablissement_id, demande_id, objet_id, pages INT NULL, efface_le NULL)
ent_repro_demande_cible (id, etablissement_id, demande_id, classe_id NULL, groupe_id NULL)
ent_repro_oeuvre (id, etablissement_id, demande_id, titre, auteur NULL, editeur NULL, type CHECK (livre|revue|partition),
  pages_copiees INT, pages_totales INT NULL)
ent_repro_receptionnaire (id, etablissement_id, site_id, personne_id, du, au NULL)
ent_repro_copieur_releve (id, etablissement_id, bien_id, releve_le, compteur_noir BIGINT, compteur_couleur BIGINT)
ent_repro_cfc_periode (id, etablissement_id, du DATE, au DATE, export_objet_id NULL)

-- Locaux pour tiers (RM9 ; 2891)
ent_ressource_convention (id, etablissement_id, tiers_id -- annuaire, collectivite TEXT NULL, du DATE, au DATE,
  avis_ca_reference TEXT NULL, objet_id, etat CHECK (brouillon|signee|resiliee|expiree))
ent_ressource_convention_creneau (id, etablissement_id, convention_id, salle_id NULL, lieu_id NULL, jour_semaine, heure_debut, heure_fin)

-- Reprise (RM11 ; 2892)
ent_ressource_reprise (id, etablissement_id, source CHECK (nextjs_photocopies), source_id TEXT, cible_id NULL,
  etat CHECK (reprise|rejetee|a_decider), raison_code NULL, cree_le)
```

**Exceptions D-036 demandées : aucune.** Table de référence globale (E12, `globale_reference`) : les lignes à `etablissement_id NULL` de `ent_ressource_categorie` (catégories préréglées). Les PDF de reprographie sont des objets **effacés** à J+30 (exception E5/E6 déclarée : effacement réel au titre de la minimisation, inscrite dans `MIGRATIONS.md`). Aucune file d'effets de bord propre (avis par le 8, E9).

---

## 25. Lots

### 25.1 Lot RM0 — correctifs urgents du code de salles (5, 10), sans migration
| # | Correctif | Défaut | Équipe |
|---|---|---|---|
| 1 | `edt_changer_salle` appelle `structure_disponibilite::disponible` (fermeture, indisponibilité, fenêtres), contrôle la capacité (avertissement), **verrou advisory sur `etab:salle_id`** pris avant le test d'occupation, une seule transaction ; tests « salle en travaux refusée » et « deux changements simultanés vers la même salle : un seul passe » | RRM-01 | socle 10 |
| 2 | `creer_salle_for_actor` et `edt_salle_modifier` redirigés vers `structure_lieux.rs` (action `ReferentielGerer`, journal) ; capacité bornée à 500 par le service (refus propre) | RRM-02 | socle 5 |
| 3 | Le port `disponible` devient l'unique contrôle de lieu, appelé en production par le 10 et, dès AG3, par le 19 ; test qui échoue si une écriture de séance ou de réservation ne l'appelle pas | RRM-03 | socles 5, 10, 19 |

### 25.2 Lots suivants
| Lot | Contenu | Dépend de | Critère de fin | Migrations |
|---|---|---|---|---|
| **RM1** Biens et inventaire | catégories, modèles, biens, étiquettes, faits, lots, affectation, équipements calculés pour le 5, inventaire mobile, écarts, sorties sur décision, effacement attesté, export comptable | RM0 ; 5, 7 | R-RM1 à R-RM8 testés ; BO du 12/02/2026 lu (Q-RM8) | 2880–2882 |
| **RM2** Prêts et manuels | règles de prêt, prêts, constats, conventions (7, 18), campagnes de distribution et de retour (mode caisse), listes par niveau, solde au départ (3, 6), prêt à une sortie (42) | RM1 ; 3, 4, 6, 18 | R-RM9 à R-RM12, R-RM14, R-RM17 testés ; une classe de 30 distribuée en moins de 10 min sur données fictives | 2883–2884 |
| **RM3** Matériel réservable et mise en place | ressources déclarées au 19, retrait et retour, conflits de panne, minibus, mise en place depuis les besoins | RM1 ; 19 (AG3) | R-RM6 testé | 2885 |
| **RM4** Demandes d'intervention | signalement par QR, doublons, qualification, alerte de sécurité, condamnation (5), routage, transmission externe, interventions, délais, vérification, réouverture | RM1 ; 5, 6, 8 ; **N1** si créé | R-RM19, R-RM24, R-RM25 testés | 2886–2887 |
| **RM5** Clés, casiers, états des lieux, reprographie | organigramme, remises, clés dehors, badges, casiers, états des lieux (41) ; **reprographie** (parité NRM-01 à NRM-10, cloisonnée), compteurs | RM2 ; 7, 41 | DRM-01 à DRM-13 impossibles ; R-RM20 à R-RM23, R-RM27 à R-RM29 testés | 2889–2890 |
| **RM6** Argent | barèmes et décisions, pertes et dégradations au 24, contestations, affectation au renouvellement | RM2 ; 24, 25 | R-RM13, R-RM15, R-RM16 testés | — |
| **RM7** Sécurité des locaux | registre (R143-44), vérifications, observations, commission, exercices reçus du 14, registre des dangers graves et imminents | RM4 ; 14 | échéances et dépassements testés | 2888 |
| **RM8** Prévision et rentrée | prévision de commande de manuels, préparation des lots par classe, casiers de rentrée | RM2 ; 3, 5 | | — |
| **RM9** Conventions et préventif | conventions de mise à disposition (R-RM30), maintenance préventive | RM4 ; 19 | | 2891 |
| **RM10** Pilotage et CFC | tableau de bord, indicateurs (47), déclaration CFC | RM5 ; 47 | | — |
| **RM11** Reprise et IA | reprise des demandes de photocopies en cours ; capacités IA (§18.3) | RM5 ; `ent-ia` | 6 pièges réussis | 2892 |
| **RM12** Conformité | purges (§21.3), accès RGPD, RLS stricte | tous ; 27 | §21.3 vérifié | 2898–2899 |

**Parcours minimal pour la rentrée 2027** : RM0, RM1, RM2 (manuels de rentrée), RM5 (reprographie, qui remplace le module Next.js en usage, D-035), RM4 (demandes d'intervention), RM6 (pertes de juin 2028 : peut suivre). Ordre recommandé : RM0 → RM1 → RM5 (reprographie, gain immédiat et défauts critiques de Next.js) → RM2 → RM4 → RM3 → RM6 → RM7 → RM8 → RM9 → RM10 → RM11 → RM12.

### 25.3 Plage de migrations (2880–2899)
| N° | Lot | Objet |
|---|---|---|
| 2880 | RM1 | catégories (référence globale + locales), modèles |
| 2881 | RM1 | biens, faits du bien, lots et mouvements, vue d'état courant |
| 2882 | RM1 | inventaires, lectures, écarts, décisions de sortie |
| 2883 | RM2 | règles de prêt, prêts (unicité partielle), constats et points |
| 2884 | RM2 | campagnes, cibles, listes par niveau, barèmes, paliers, facturations, déclarations de perte |
| 2885 | RM3 | ressources réservables, carnet de bord, mise en place |
| 2886 | RM4 | demandes, appuis, pièces, faits |
| 2887 | RM4 | interventions, pièces, préventif |
| 2888 | RM7 | vérifications, observations, commission, registre, dangers graves et imminents |
| 2889 | RM5 | cylindres, clés, ouvertures, casiers, attributions, états des lieux |
| 2890 | RM5 | reprographie : demandes, documents, cibles, œuvres, réceptionnaires, relevés, périodes CFC |
| 2891 | RM9 | conventions de mise à disposition et créneaux |
| 2892 | RM11 | correspondances de reprise |
| 2893–2897 | réserve | |
| 2898 | RM12 | RLS stricte, index de pilotage |
| 2899 | RM12 | retraits (sur décision, E8) |

---

## 26. Reprise de Next.js
1. **Photocopies** (collection KV `photocopies-couleur`, un document) : décodage strict (D-036 ; clé inconnue = ligne refusée et listée) ; seules les demandes **non terminées** (en attente, acceptées) sont reprises, rattachées au demandeur par compte (jamais par e-mail seul), au site par correspondance exacte ; PDF copiés vers le dépôt du 7 avec empreinte ; demandes terminées : **perdables** (T0) — compteurs agrégés repris si Florian refacture (Q-RM7).
2. **Salles et réservations** : au **19** (AG11).
3. **Bascule** : propriété unique par **site** et par date (une demande vit dans un seul système) ; Next.js reste ouvert en lecture tant que Florian ne l'arrête pas (D-035 point 3).
4. **Contrôles** : comptages, empreintes, idempotence, annulation d'une exécution.

## 27. Tests et recette
- **Unitaires** : état courant depuis les faits ; unicité de l'étiquette et du prêt ouvert ; barème dégressif (années 1 à 5) ; refus de caution sur un manuel ; écarts d'inventaire ; seuils de reprographie ; portée par site ; condamnation et levée.
- **Concurrence** : deux prêts simultanés du même bien (un seul passe) ; deux réservations du même chariot ; deux changements de salle (RM0).
- **Isolation** : matrice entre établissements (tables 2880–2892) ; RLS stricte ; réceptionnaire d'un site sans accès à l'autre ; collectivité sans nom d'élève.
- **Pièges** : PDF d'un autre non rattachable (identifiant forgé refusé) ; aucun GET qui modifie ; aucun code de casier lisible ; aucune géolocalisation ; facture de perte sans constat de remise refusée pour une dégradation.
- **Recette** : parité NRM-01 à NRM-10 action par action contre Next.js ; campagne de distribution complète sur données fictives (300 élèves) ; inventaire de 3 salles au téléphone ; demande de sécurité de bout en bout ; à 375 px et au clavier (axe-core).

## 28. Réglages signalés (D-023)
| S | Réglage | Défaut |
|---|---|---|
| S-4400 | Seuil d'immobilisation | 800 € HT |
| S-4401 | Catégories actives | préréglage (manuels, informatique, audiovisuel, sport, clés, casiers, mobilier) |
| S-4402 | Équipements de salle calculés depuis les biens | oui dès RM1 |
| S-4403 | Durée de prêt par défaut (hors campagnes) | 14 jours |
| S-4404 | Barème des manuels | dégressif 100/50/30/20 % (sauf décision enregistrée) |
| S-4405 | Dépôt de garantie autorisé (hors manuels) | non |
| S-4406 | Convention exigée pour les ordinateurs et tablettes | oui |
| S-4407 | Rappels de retard | J+1, J+7 |
| S-4408 | Délai de contestation d'une facture de perte | 30 jours |
| S-4409 | Signalement direct par les élèves | non (par un personnel) |
| S-4410 | Seuil de validation de la reprographie noir et blanc | 500 copies |
| S-4411 | Délais cibles par urgence | sécurité 4 h, haute 2 j, normale 10 j, basse 30 j |
| S-4412 | Délai de la tâche de mise en place | J-1 |
| S-4413 | Lecture des PDF par l'IA pour l'avertissement CFC | non |
| S-4414 | Registre de sécurité tenu dans ScolIA | non (export) jusqu'à Q-RM5 |
| S-4415 | Tranche CFC souscrite | 100 pages |
| S-4416 | Photo exigée au constat de retour d'un ordinateur | oui |
| S-4417 | Passe général : double validation | non (direction seule, ★) |

## 29. Questions et décisions

### 29.1 Appliquées par défaut (D-031)
| Q | Question | Recommandation appliquée |
|---|---|---|
| Q-RM3 | Réservation de salles au 44 ? | Non : au 19 (Q-AG1) ; le 44 fournit matériel et mise en place |
| Q-RM4 | Planning de domaines au 44 ? | Non : 19 (AG6b) |
| Q-RM10 | Équipements de salle : étiquettes du 5 ou biens du 44 ? | Biens du 44, étiquettes calculées (R-RM5) |
| Q-RM11 | Caution pour les manuels ? | Jamais (R-RM12) |
| Q-RM12 | Barème des manuels par défaut ? | Dégressif national (R-RM13) |
| Q-RM13 | Facturer une dégradation sans constat de remise ? | Non (R-RM14) |
| Q-RM14 | Retenir un bulletin pour un manuel non rendu ? | Jamais (R-RM15) |
| Q-RM15 | Suppression d'un bien ? | Jamais : sortie sur décision |
| Q-RM16 | Géolocaliser les appareils prêtés ? | Jamais (R-RM18) |
| Q-RM17 | Importer le journal des badges ? | Non (R-RM22) |
| Q-RM18 | Stocker les codes de casier ? | Non |
| Q-RM19 | Élèves qui signalent directement ? | Non par défaut, par un personnel (S-4409) |
| Q-RM20 | Nom d'élève dans une demande transmise ? | Jamais (R-RM24) |
| Q-RM21 | Clôture d'une demande de sécurité ? | Après vérification |
| Q-RM22 | Réceptionnaire de reprographie par défaut = rôle accueil (comme Next.js) ? | Non : personne désignée par site (R-RM27) |
| Q-RM23 | Noir et blanc toujours accepté (comme Next.js) ? | Sous un seuil seulement (R-RM29) |
| Q-RM24 | PDF en pièce jointe aux courriels ? | Jamais : lien (R-RM28) |
| Q-RM25 | Lien « prête » sans connexion ? | Non repris (DRM-05) |
| Q-RM26 | Plafonds CFC bloquants ? | Avertissement, pas de blocage |
| Q-RM27 | Mise en place : au 19 ou au 44 ? | Besoins au 19, tâches au 44 |
| Q-RM28 | États des lieux d'internat : au 41 ou au 44 ? | Demande au 41, constat au 44 |
| Q-RM29 | Prêts du CDI ? | Au 45, sur le même modèle de prêt |
| Q-RM30 | Réservation par un tiers sans convention ? | Refusée (R-RM30) |
| Q-RM31 | Registre des dangers graves et imminents dans ScolIA ? | Saisie et suivi proposés, accès restreint ; gouvernance au 49 |
| Q-RM32 | Durées de conservation | §21.3, à confirmer avec le DPO |
| Q-RM33 | Défauts M02 trouvés ici (DRM-14 à DRM-19) | relayés au 19 (§30) |
| Q-RM34 | Demandes de photocopies terminées de Next.js | non reprises (perdables) |

### 29.2 Soumis à Florian (engagent l'établissement, des données réelles, ou des faits que seul Florian connaît)
| Q | Question | Recommandation |
|---|---|---|
| **Q-RM1** | Créer le socle **N1 « Demandes et flux de travail »** avant février 2027 (déjà recommandé par `SOCLES.md` §10 et le 29, Q-PO3) | **oui** ; d'ici là, le 44 porte seulement le guichet des interventions et de la reprographie |
| **Q-RM2** | Dépôt de garantie pour le matériel autre que les manuels (ordinateurs, instruments, clés) : public (avec l'agent comptable) et privé | **non** dans V1 ; responsabilité de droit commun et barème voté |
| **Q-RM5** | Le registre de sécurité est-il tenu ailleurs (papier, logiciel du chef de sécurité) ? | export d'abord ; tenue dans ScolIA après accord de la direction |
| **Q-RM6** | Partage des demandes avec la collectivité (public) ou l'organisme de gestion (privé) : courriel, export, accès ? | export et courriel qualifiés d'abord ; accès direct plus tard |
| **Q-RM7** | Les photocopies sont-elles refacturées (services, associations) ? | compteurs repris si oui |
| **Q-RM8** | Lire le BO n° 7 du 12/02/2026 (sorties d'inventaire simplifiées) et la M9-6 avec l'agent comptable avant RM1 | oui |
| **Q-RM9** | Outil de gestion de flotte (tablettes, ordinateurs) de la collectivité ou de l'établissement à relier ? | non dans V1 |
| **Q-RM35** | L'établissement a-t-il signé la licence CFC (tranche 100 ou 180 pages) ? | à confirmer |
| **Q-RM36** | Barème de remplacement des manuels : délibération (public) ou décision de l'organisme (privé) à faire prendre | barème dégressif national par défaut |

---

## 30. Ajouts personas (`PERSONAS_44.md`, 2026-11-11)

**En cas de contradiction avec les §0 à §29, ce paragraphe l'emporte.** Manques M-4401 à M-4440 ; quatre bloquants (A à D) appliqués par défaut (D-031).

### 30.1 Bloquants tranchés
| # | Bloquant | Décision appliquée | Effet sur la conception |
|---|---|---|---|
| A | Étiquetage initial irréaliste (milliers de manuels sans étiquette) | **manuels en quantité par élève** l'année 1 (modèle × quantité × état global, année de mise en service estimée par lot) ; **étiquetage progressif** au retour de juin 2028 | R-RM35 ; RSM-250, RSM-251 ; §11.1 et §10.2 précisés |
| B | Alerte de sécurité hors des heures sans destinataire | **astreinte technique** déclarée par semaine (6) ; escalade SMS après 15 min sans accusé (8) ; numéros d'urgence et des concessionnaires affichés au signalement | R-RM36 ; RSM-252, RSM-253 |
| C | Un seul reprographe pour plusieurs sites | réceptionnaire **désigné explicitement** pour un ou plusieurs sites ; seule la désignation implicite par rôle est interdite ; validation de la couleur par la direction du site de la demande ; choix du site d'impression par le demandeur | R-RM27 précisée ; RSM-254 |
| D | Pertes facturées pendant le doublon | **mode suivi** : constat et calcul au barème dans ScolIA, **export** vers Charlemagne ; aucune facture émise par ScolIA tant que le 24 n'est pas en production | R-RM37 ; RSM-255 |

### 30.2 Actions ajoutées (RSM-250 à RSM-269)
| RSM | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RSM-250 | Prêter des manuels **en quantité** par élève (modèle × quantité × état global) | GES, ADM · `ressources.prets_gerer` | R-RM35 | `ressources.pret_ouvert` | RM2 |
| RSM-251 | Étiqueter un exemplaire au retour (conversion d'un prêt en quantité en exemplaires) | GES, ADM | | `ressources.bien_entre` | RM2 |
| RSM-252 | Déclarer l'astreinte technique de la semaine (personne, téléphone de service) | GES, DIR · `ressources.securite` | 6 | `ressources.astreinte` | RM4 |
| RSM-253 | Escalader une alerte de sécurité non accusée (15 min) | SYS | R-RM36 | `ressources.alerte_escaladee` | RM4 |
| RSM-254 | Désigner un réceptionnaire pour un ou plusieurs sites ; choisir le site d'impression | DIR, GES · `ressources.reprographie_gerer` ; demandeur | R-RM27 précisée | — | RM5 |
| RSM-255 | Exporter les pertes et dégradations à facturer (mode suivi) | GES | R-RM37 | — | RM6 |
| RSM-256 | Produire l'état « manuels non rendus par classe » (sans montant) | GES → PP | | — | RM2 |
| RSM-257 | Lier une panne à un prêt et prêter un appareil de remplacement | NUM | | `ressources.pret_ouvert` | RM4 |
| RSM-258 | Imprimer en lot les QR codes des salles | GES, AGT | identifiant opaque | — | RM4 |
| RSM-259 | Joindre le constat à l'avis de facturation et mentionner l'assurance de la famille | SYS | | — | RM6 |
| RSM-260 | Produire le dossier de commission de sécurité (registre, vérifications, observations, exercices) | DIR, GES · `ressources.securite` | | — | RM7 |
| RSM-261 | Rattacher une action en cours (commande, relance) à une vérification échue | GES | | — | RM7 |
| RSM-262 | Déclarer qu'une vérification est commandée par la collectivité ; relancer | GES | | — | RM7 |
| RSM-263 | Constater contradictoirement (second personnel) un état des lieux sans signature de la famille ; avis avec photos | SUR, CPE | R-RM33 | `ressources.etat_des_lieux` | RM5 |
| RSM-264 | Imputer une dégradation à un élève auteur après la procédure du 13 | GES | R-RM39 | `ressources.degradation` | RM6 |
| RSM-265 | Inventorier les clés en lot (numéro, détenteur) sans organigramme | GES, ACC | R-RM38 | — | RM5 |
| RSM-266 | Proposer un doublon par salle, quelle que soit la catégorie (7 jours) | SYS | | — | RM4 |
| RSM-267 | Travailler hors ligne sur la liste du jour (agent) et synchroniser | AGT | | — | RM4 |
| RSM-268 | Prêter un appareil à un étudiant pour la durée d'un stage (39) | NUM | retour = fin de convention | `ressources.pret_ouvert` | RM2 |
| RSM-269 | Effacer les PDF marqués « données d'élèves » 7 jours après « prête » | SYS | S-4420 | — | RM5 |

### 30.3 Règles ajoutées et précisées
| R | Règle |
|---|---|
| R-RM35 | Un manuel peut être prêté **en quantité** (sans étiquette) ; l'année de mise en service est estimée par lot ; l'étiquetage se fait au retour ; un prêt en quantité suit les mêmes règles de solde et de barème. |
| R-RM36 | Une alerte de sécurité est accusée par l'astreinte technique ; sans accusé en 15 min, elle est escaladée (direction, SMS selon le 8) ; sans astreinte déclarée, l'écran affiche les numéros d'urgence et alerte la direction. |
| R-RM37 | Tant que le 24 n'est pas en production (doublon, D-034), aucune facture de perte ou de dégradation n'est émise par ScolIA : constat, calcul, export. |
| R-RM38 | L'organigramme des clés est facultatif ; une clé peut être suivie par son seul numéro. |
| R-RM39 | Une dégradation de local partagé n'est jamais facturée à tous les occupants par défaut ; seulement à un auteur établi par la procédure du 13. |
| R-RM40 | Aucune donnée du 44 (prêts, clés, réservations, reprographie, badges) ne sert à évaluer ni à surveiller un personnel ; aucune vue « par personne » hors gestionnaire et intéressé. |
| R-RM13 (précisée) | Année de mise en service inconnue : taux le plus favorable à la famille (4e année et plus). |
| R-RM17 (précisée) | Le départ d'un élève n'est **jamais** bloqué : « à recouvrer » se pose en un geste et ne retient aucun document (R-RM15). |
| R-RM27 (précisée) | Le réceptionnaire est désigné explicitement pour un ou plusieurs sites ; la désignation implicite par rôle est interdite. |

### 30.4 Cas limites ajoutés (CRM-47 à CRM-58)
| CRM | Cas | Traitement |
|---|---|---|
| CRM-47 | Rentrée 2027 : 4 800 manuels sans étiquette | prêt en quantité ; étiquetage au retour |
| CRM-48 | Fuite d'eau un samedi à 9 h | astreinte avisée ; escalade à 9 h 15 ; numéros affichés |
| CRM-49 | Aucune astreinte déclarée cette semaine | alerte direction ; numéros d'urgence affichés |
| CRM-50 | Un reprographe pour deux sites | désigné pour les deux ; il voit les deux files |
| CRM-51 | Enseignant des deux sites qui imprime sur le site où il fait cours le jeudi | choix du site d'impression |
| CRM-52 | Pertes de juin 2028 pendant le doublon | export vers Charlemagne ; aucune facture ScolIA |
| CRM-53 | Manuel d'année inconnue perdu | 20 % du neuf |
| CRM-54 | Yanis part en février avec 2 manuels perdus | certificat de radiation délivré ; « à recouvrer » |
| CRM-55 | Ordinateur prêté en panne | remplacement prêté, original en réparation, un seul geste |
| CRM-56 | Chambre dégradée par un tiers non identifié | pas de facture à la chambre ; intervention |
| CRM-57 | La direction demande « à quelle heure M. X a pris la clé du labo chaque jour » | refusé (R-RM40) |
| CRM-58 | PDF de reprographie avec une liste d'élèves et leurs notes | effacé 7 jours après « prête » ; jamais lu par l'IA |

### 30.5 Défauts relayés au socle 19 (à reporter dans `19_AGENDA_RDV.md`)
DRM-14 (annulation de n'importe quelle réservation ou série par tout compte), DRM-15 (modification de la réservation d'autrui), DRM-16 (modification sans contrôle de l'EDT ni de « réservable »), DRM-17 (réécriture de toutes les réservations à chaque modification), DRM-18 (bornes absentes des séries et de l'horizon), DRM-19 (droits incohérents de l'administrateur global) — §2.3. **DRM-14 et DRM-15 sont à signaler à Florian maintenant** (Next.js en production). Tests d'AG3 à compléter : un personnel quelconque ne peut ni annuler ni modifier la réservation d'un autre.

### 30.6 Questions nouvelles et réglages
Q-RM37 à Q-RM39 (soumises à Florian, `PERSONAS_44.md` §8.2). Réglages ajoutés : **S-4418** délai d'escalade d'une alerte de sécurité (15 min), **S-4419** fenêtre de détection de doublon par salle (7 jours), **S-4420** effacement des PDF « données d'élèves » (7 jours après « prête »), **S-4421** prêt de manuels en quantité (oui la première année).

### 30.7 Lots révisés
- **RM2** reçoit le bloquant A (RSM-250, RSM-251, RSM-256, RSM-268) : la campagne de **septembre 2027** se fait en quantité.
- **RM4** reçoit le bloquant B (RSM-252, RSM-253, RSM-257, RSM-258, RSM-266, RSM-267) : aucune mise en service des alertes de sécurité sans astreinte.
- **RM5** reçoit le bloquant C (RSM-254, RSM-263, RSM-265, RSM-269) ; **bascule de la reprographie Next.js aux vacances de la Toussaint 2027** (Q-RM38).
- **RM6** reçoit le bloquant D (RSM-255, RSM-259, RSM-264) : mode suivi pendant le doublon.
- **RM7** reçoit RSM-260 à RSM-262.
- Hors du 44 : défauts relayés au **19** (§30.5) ; agents territoriaux, habilitation `conduite_minibus` et astreinte technique au **6** ; produits chimiques au **49** (Q-RM37).

## 31. Faits, hypothèses, contradictions

**Faits [F]** : Next.js : photocopies dans un document KV unique, file d'impression lisible de tous les sites par le rôle « accueil » par défaut, clés d'objet prévisibles acceptées sans lien au déposant, secret de jeton codé en dur, route publique GET qui modifie ; réservation de salles : annulation et modification sans contrôle de propriété côté serveur (`delete/route.ts:33-61`, `update/route.ts:14-24`) ; maintenance par le module Demandes ; aucun module de prêts, d'inventaire, de manuels, de clés. Rust : `ent_salle` (076, 337) avec `equipements TEXT[]` et `reservable` ; `ent_salle_indisponibilite` immuable ; port `disponible` appelé seulement par les tests ; `edt_changer_salle` sans contrôle d'indisponibilité ni verrou par salle (`edt_remplacement.rs:1011-1060`) ; trois chemins d'écriture des salles ; aucune table ni action du 44 ; plus haute migration écrite : 642 ; aucun fichier 2860–2899.

**Hypothèses [H]** : calendrier (§4) ; usage réel de la reprographie Next.js dans l'établissement pilote ; existence d'un contrat de copieur avec compteurs ; organisation des agents (internes dans le privé, de la collectivité dans le public).

**Contradictions relevées** :
1. `SOCLES.md` §44 (« réservation de salles et de matériel sans conflit avec l'emploi du temps ») et `10_EMPLOI_DU_TEMPS.md` (« 44 possède ») contre `19_AGENDA_RDV.md` §14.1 (réservation de lieux au 19, appliquée par défaut) : ce document **suit le 19** ; `SOCLES.md` §44 et le 10 sont à corriger (fait pour `SOCLES.md` dans cette branche, §30).
2. `05_STRUCTURE.md` (l. 1155 : « contrat de disponibilité à écrire au socle 44 ») : le contrat est **écrit au 19** (§14.3, point 2) et rappelé ici (port unique, RM0 point 3).
3. `29_PORTAILS.md` et `MATRICE_CHARLEMAGNE.md` (DEM-04) placent la reprographie dans le futur socle **N1** ; la mission de ce socle la place au 44 : **le 44 porte la reprographie** (file, compteurs, CFC) et le **guichet** passera à N1 quand il existera (Q-RM1).
4. `06_PERSONNELS.md` (PRS-119 : prêt de clé de sécurité « ou socle 44 ») : les clés sont au 44 (§14.1) ; la clé de sécurité numérique (passkey matérielle) reste au 1 et au 6.
5. `05_STRUCTURE.md` garde `equipements TEXT[]` sur `ent_salle` (tableau, contraire à l'esprit de D-036 pour une donnée relue) : le 44 propose de le **calculer** depuis les biens (R-RM5) ; la colonne devient une projection, à normaliser par le 5 en `ent_salle_equipement` si elle reste saisie [R].
