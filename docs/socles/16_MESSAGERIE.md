# Socle 16 — Messagerie et conversations (avec ce que le socle 17 « informations, annonces et campagnes » en attend)

**Date :** 2026-11-09 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code (lot 7, S2-8 « communication », et correctif RP1-11 de la recette 1) pour le rattraper (`docs/PILOTAGE.md` §4, « rattrapage des domaines codés avant la méthode »). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à l'application Next.js (lecture ciblée de deux schémas seulement, §2.4).
**État du lot M0a (2026-11-09, branche `ent-socle-messagerie-m0a`, base `ENT`) :** **implémenté et testé en local** (tests Postgres `ent-db` `tests_messagerie_m0a`, voir §27.3) ; **CI GitHub : voir §27.3** ; migration **480** (repli 480–499, Q-B2 non tranchée). Non validé par Florian, non prêt pilote : le reste (participants, boîtes, pièces, etc.) vient dans M0b et M1 à M17.
**Branche :** `ent-socle-messagerie-conception` (partie de `ent-mc0hoe`, `91e3822`, qui contient `ent-socle2-communication` et `ent-recette-1-correctifs` ; `ent-recette-securite-3` lue sans être fusionnée).
**À lire avec :** `CLAUDE.md` (envois soumis à confirmation humaine ; l'IA n'a jamais plus de droits que l'utilisateur) ; `docs/PILOTAGE.md` ; `docs/DECISIONS.md` (D-013, D-014, D-017, D-021, **D-022**, **D-023**, D-024, D-026, **D-028**, **D-031**) ; `docs/SOCLES.md` (socles **16, 17**, 8, 18, 19, 30, 33, 36) ; `docs/MATRICE_CHARLEMAGNE.md` §3.9 (COM-01 à COM-21) ; `docs/socles/04_ADMINISTRATIF.md` (§8.6 droits du lien, §8.8 interdictions, §8.12 destinataires, §12.3 adresse protégée) ; `08_NOTIFICATIONS.md` et `PERSONAS_08.md` (**le socle 8 distribue, le socle 16 converse**) ; `06_PERSONNELS.md` (suppléances, départs, équipes) ; `07_DOCUMENTS.md` (pièces, coffre, conversion) ; `IA_COEUR.md` (§6.2 niveaux de confirmation, §8.2 tâches de réponse) ; `docs/VERIF_REGLEMENTAIRE.md` ; `docs/recette/RECETTE_PERSONAS_1.md` (RP1-11) ; `RECETTE_SECURITE_3.md` (branche `ent-recette-securite-3`) ; modèle de forme : `21_EVALUATIONS.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée) ; **[NV]** règle non vérifiée sur une source officielle ; **[H]** hypothèse (pratique du terrain, marché) ; **[R]** recommandation (non décidée). Numérotation propre à ce socle : actions **MSG-100** et suivantes (COM-01 à COM-09 sont pris par `SOCLE2_DOMAINES.md`), actions du socle 17 **ANN-100** et suivantes, règles **R-M1**…, cas limites **CM-01**…, questions **Q-M1**…, réglages signalés **S-132**… (après S-131 du socle 21).
**Plage de migrations :** **4600–4699** si la règle « 3000 + 100 × numéro de socle » (Q-B2) est adoptée ; **repli 480–499** sinon (vérifié libre sur toutes les branches distantes le 2026-11-09, plus haute migration trouvée : 460) ; le lot **M0a** peut partir tout de suite en **480**. Socle 17 : **4700–4799** ou repli **500–519**, réservé pour éviter une collision (registre `MIGRATIONS.md`).
**Méthode.** Lecture des décisions, de la carte, des conceptions 04, 06, 07, 08, de la passe personas 08, des recettes 1, 2 et de sécurité 3 ; lecture directe des migrations 023 et 375 ; audit indépendant du code (sous-agent, lecture seule, ~90 lectures), dont **les défauts D1, D2, D3, D4 et D7 ont été relus ligne à ligne** ; recherche réglementaire (sous-agent) sur Légifrance, cnil.fr, service-public.gouv.fr, les PDF officiels du ministère et le service d'assistance de la Commission (règlement IA) ; les pages HTML d'education.gouv.fr, d'eduscol et d'EUR-Lex renvoient 403 aux robots (points marqués [NV]).

Sommaire : §0 En bref · §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs · §4 Cas limites · §5 Réglementaire · §6 Frontières · §7 Concepts · §8 Matrice « qui peut écrire à qui » · §9 Parents séparés, interdictions · §10 Élèves · §11 Personnels · §12 Pièces jointes · §13 Lecture et accusés · §14 Modération, harcèlement, protection, urgence · §15 Conservation · §16 IA · §17 Socle 17 (esquisse) · §18 Carnet de correspondance · §19 Actions · §20 Règles transverses · §21 Droits et minimisation · §22 Événements · §23 Notifications · §24 Écrans · §25 Modèle de données · §26 Plan de migration · §27 Lots de code et migrations · §28 Tests et recette · §29 Réglages signalés · §30 Questions · §31 Faits, hypothèses, contradictions · §32 Les cinq décisions à prendre en priorité.

---

## 0. En bref

1. **Ce qui tient, ce qui fuit.** L'existant a de bons réflexes : un **seul contrôle d'accès** par lequel passent toutes les routes (web, mobile, UI historique), un cloisonnement entre établissements tenu (matrices de tests, recette de sécurité 3), la portée D-022 du professeur, une chaîne de notifications S2-8 solide, des campagnes à **aperçu, empreinte et confirmation**. Mais la lecture d'un fil se **déduit du lien avec l'élève** : tout fil est « le fil de l'élève ». D'où **quatre fuites critiques**, confirmées ligne à ligne (§2.2) : (D1) **l'interdiction de communication est ignorée** — un père visé par une ordonnance lit les fils de l'établissement et les réponses de la mère, et reçoit le carnet par e-mail et SMS ; (D2) **tout personnel qui suit l'élève lit les fils privés** d'un parent avec la direction et les relances de facture ; (D3) **l'élève lit les fils adressés à ses parents, et ses parents lisent les siens** — un élève qui confie à la CPE des violences à la maison est lu par le parent violent ; (D4) **le contenu part en clair** hors de l'ENT (sujet du fil en titre de notification mobile, carnet complet par e-mail et SMS). S'y ajoutent seize défauts (droits du lien ignorés, « ne pas solliciter » contourné par le push, carnet sans contrôle de périmètre, annonces sans aperçu ni retrait, aucune RLS…). Un **lot M0a de correctifs** passe avant tout.
2. **La thèse (§1).** Une conversation est une **relation entre des personnes nommées**, à propos d'un sujet, sous la responsabilité de l'établissement. **Lire = participer** (R-M1) : ni le lien avec l'élève, ni le rôle n'ouvrent un fil. Les familles écrivent d'abord **à des services** (boîtes de service : vie scolaire, secrétariat, infirmerie…), pas à des personnes : la continuité ne dépend plus de la présence de quelqu'un. Les destinataires sont **calculés à partir des faits** (socles 4, 5, 6), jamais cochés.
3. **Parents séparés** : un message « aux parents » crée **un fil par foyer** ; un parent ne voit jamais l'existence d'un fil de l'autre foyer ; les informations dues partent aux deux, chacun en son nom (« les deux responsables ont été informés »), comme le demande la doctrine ministérielle [V] ; l'interdiction de communication, le lien suspendu, le décès et « ne pas solliciter » sont relus à chaque lecture, écriture et avis.
4. **Élèves** : l'élève mineur peut écrire aux adultes de l'établissement, discrètement (CPE, infirmière, psychologue), et **ses parents ne lisent pas sa messagerie** ; aucun échange adulte–mineur n'est hors de portée de l'institution (**fils encadrés** : supervision motivée et journalisée, indicateur de vigilance déterministe sans lecture du contenu) ; **pas de messagerie libre entre élèves** (cyberharcèlement, obligation de prévention de L. 111-6 [V]).
5. **Le réglementaire tranche quatre points [V]** (§5) : (a) **plage calme nationale par défaut** — les outils ENT et vie scolaire ne diffusent aucune nouvelle information **de 20 h à 7 h et du vendredi 20 h au lundi 7 h**, saisie possible, remise différée, rappel à l'émetteur, dérogation de la direction (référentiel du numérique responsable, exigences 8 et 9) ; (b) les messages de l'ENT sont de la **correspondance privée** : contrôle par la direction seulement s'il est prévu par la charte, par des personnes désignées, journalisé (kit juridique du SDET 2025) — fondement fragile face à l'article 432-9 du code pénal, avis juridique requis ; (c) l'arrêté ENT de 2006 ne fixe **aucune durée** pour la messagerie et impose la **suppression 3 mois après la perte du droit au compte** ; dans le public, les échanges avec les familles sont des **archives publiques** (visa d'élimination) ; (d) toute demande d'information d'un parent **reçoit une réponse**, un refus est motivé (D. 111-4).
6. **Personnels protégés** : boîtes partagées avec prise en charge visible, réponse au nom du service ; suppléant qui hérite des fils du service ; départ sans fil orphelin ; plage calme et disponibilité ; restriction d'un parent harcelant ; pas d'indicateur de présence ni d'heure de lecture côté familles.
7. **Protection et preuve** : signalement en un geste, **gel** (plus de retrait, de clôture ni de purge), parcours harcèlement (3018) et information préoccupante sans avis automatique à la famille, bandeau d'urgence permanent (la messagerie n'est pas un canal d'urgence), **aucune suppression physique** par un utilisateur.
8. **L'IA aide, la personne envoie** (§16) : aide à la rédaction, résumé, tri, traduction pour les familles allophones — toujours **relu et confirmé** (N3, passkey récente) avant tout envoi, destinataires imposés par le fil, original visible, traduction et assistance **marquées** (règlement IA art. 50 [V]) ; jamais d'envoi, de modération, de supervision ni de profilage.
9. **Socle 17** (§17) : une publication (annonce, actualité, lettre, campagne) est un **acte éditorial** à audience décrite, résolue sous les droits de l'auteur ; sa **diffusion** (aperçu, coût, confirmation, relance des non-lecteurs) est la mécanique du socle 8 ; pas de commentaires publics, « poser une question » ouvre un fil.
10. **Le plan.** 150 actions (133 de MSG-100 à MSG-308, numérotation à trous, et 17 du socle 17, ANN-100 à ANN-116), 80 cas limites, 47 règles, 17 réglages signalés, **19 lots** (M0a, M0b, M1 à M17) avec critères de fin ; **parcours minimal vendable** : M0a, M0b, M1–M4, M6, M12 ; **avant tout pilote avec des élèves** : M8 et M10. Migrations 4600–4699 (ou 480–499). 22 questions, toutes avec une recommandation appliquée par défaut (D-031) sauf celles qui engagent l'extérieur (durées de conservation, moteur antivirus, infirmerie avant HDS). Cinq décisions prioritaires : §32.

---

## 1. Thèse : une conversation est une relation professionnelle entre des personnes nommées

1. **Lire = participer.** Un fil a des **participants nommés** (des personnes, avec ou sans compte, ou des boîtes de service), avec des dates d'entrée et de sortie. Personne d'autre ne le lit, sauf une supervision **motivée, journalisée, prévue par la charte**. C'est le changement de fond : aujourd'hui, être lié à l'élève suffit.
2. **On écrit à un service, pas à quelqu'un.** « La vie scolaire de Léa », « le secrétariat », « l'infirmerie » : une boîte partagée, des membres tirés des missions du socle 6, une prise en charge visible, un délai annoncé. Un congé, une mutation, une suppléance ne font plus perdre un message.
3. **Le logiciel choisit les destinataires, pas la personne.** Le professeur choisit des élèves, un cours, une classe ; le socle 4 dit qui sont les responsables, avec quels droits, dans quels foyers ; le socle 5 dit quels élèves sont dans le cours **à la date** ; le socle 6 dit qui est dans l'équipe. La messagerie affiche la raison (« vous pouvez écrire à Mme B. : professeure de mathématiques de Léo »).
4. **Un foyer, un fil.** Deux parents séparés ne partagent jamais un fil ; les informations dues partent à chacun.
5. **Le temps est une règle, pas une politesse.** Plage calme nationale (20 h–7 h, week-end) pour tout ce que l'établissement envoie ; disponibilité des personnels pour ce qu'ils reçoivent ; aucun adulte ne joint un élève la nuit.
6. **Un message est un acte professionnel conservé.** On le corrige vite et on le retire en le traçant ; on ne le détruit qu'à la purge de conservation ; un fil signalé ou lié à une procédure est **gelé**.
7. **L'action se fait dans la conversation.** Justifier une absence, signer un mot, répondre à un formulaire, prendre rendez-vous, ouvrir un document : des **cartes d'action** exécutées par leur socle avec les droits de la personne (c'est la surface du futur « chat first », socle 30).
8. **Rien de sensible hors de l'ENT.** Les canaux externes (notification mobile, SMS, e-mail) ne portent qu'une enveloppe neutre ; le contenu se lit authentifié (socle 8 §9.6).
9. **Conversation ≠ diffusion.** Au-delà de 30 fils individuels, ce n'est plus une conversation : c'est une publication (socle 17) ou une diffusion (socle 8), avec aperçu et confirmation.
10. **L'IA écrit des brouillons, jamais des envois.** Elle aide à rédiger, résumer, trier, traduire ; une personne relit et confirme chaque envoi.

---
## 2. Évaluation franche de l'existant

Code lu sur `ent-socle-messagerie-conception` (partie de `ent-mc0hoe`, `91e3822`, qui contient `ent-socle2-communication` et `ent-recette-1-correctifs`) ; chemins depuis `ent-rust/`. Audit indépendant en lecture seule (sous-agent, ~90 lectures) ; **les défauts D1, D2, D3, D4 et D7 ont été relus ligne à ligne** par le concepteur (`msg_repo.rs:61-90`, `service.rs:17289-17346`, `service.rs:17384-17412`, `service.rs:17199-17208`, `service.rs:17040-17080`).

### 2.1 Ce qu'il y a [F]
| Élément | Où | Ce que ça fait |
|---|---|---|
| Fils et messages | `023_messagerie.sql`, `msg_repo.rs`, `domain-eleve/src/messagerie.rs` | `ent_msg_fil` (**`eleve_id NOT NULL`** : tout fil est le fil d'un élève), `ent_msg` (corps sans version ni état) |
| Accusés | `037_msg_accuse.sql` | `UNIQUE (message_id, lecteur_user_id)`, posés **à la lecture**, par tout lecteur |
| Pièces | `038_msg_piece.sql`, `ent-object/src/upload.rs:164-225` | contenu en **`BYTEA`**, `CHECK taille <= 262144` ; signature binaire et liste blanche contrôlées ; antivirus : point d'accroche `AucuneAnalyse` ; lien court signé HMAC (`service.rs:17871-17910`) |
| Modèles, archive, priorité, dossier | `047`, `050` | colonnes **globales au fil** (pas par personne) |
| Fil privé, fil de foyer (RP1-11) | `375_msg_fil_prive.sql`, `msg_repo.rs:61-90` | `prive_user_id` (fil ouvert par un parent), `foyer_id` (relance de facture, payeurs) |
| Contrôle d'accès | `service.rs:17289-17346` | **un seul passage** (`actor_can_access_fil`) pour liste, lecture, réponse, archive, pièce, lien court ; personnel : `autoriser_eleve(MessagerieEleve)` (portée D-022 au groupe, `perimetre.rs:54`) |
| Carnet de correspondance | `020_vs_carnet.sql`, `044`, `service.rs:17005-17100` | entrée (catégorie, titre, corps, visible famille, signature) ; crée un fil non privé et envoie e-mail, SMS, push |
| Annonces | `086_annonce.sql`, `111`, `annonce_repo.rs` | audiences `tous`, `parents`, `personnels`, `classe` (par **libellé**) ; une seule route POST |
| Notifications S2-8 | `190_notifications.sql`, `notifications.rs` | chaîne bien faite : préférences, heures calmes, `cle_dedup`, plafonds SMS sous verrou, droits relus à l'envoi |
| Campagnes | `191_campagnes.sql`, `campagnes.rs` | brouillon → aperçu à empreinte → lancement refusé si l'empreinte change ; double confirmation SMS ; plafonds ; portée de `campagnes.lancer` |
| Rencontres, formulaires | `192`, `193`, `rencontres.rs`, `formulaires.rs` | réservation sous verrou ; décisions immuables à empreinte ; texte de notification **neutre** (le bon modèle) |
| Écrans | `web/crates/ent-web-app/src/modules/messagerie`, `ui.rs`, applis `apps/*` | mêmes routes (`routes.rs:705-717`) : aucun contournement propre aux clients |
| IA | `/copilote/ask` | lecture seule, aucune intention de messagerie : l'IA ne lit ni n'envoie aucun message aujourd'hui |
| Tests | `tests/separation.rs:616-703`, `recette_personas_1.rs:689, 1091`, `ent-web-server/tests/messagerie.rs:227`, `tests_communication.rs` | isolation entre établissements tenue ; RP1-11 ; Leptos (autre famille, pièces, professeur hors groupe) ; chaîne S2-8 (aucun test de messagerie dans ce fichier) |

### 2.2 Défauts trouvés (à corriger avant toute extension)

**C** = confirmé par lecture ; **S** = supposé, à rejouer.

| # | Gravité | Défaut | Preuve | Scénario | Correctif |
|---|---|---|---|---|---|
| **D1** | **critique** | **L'interdiction de communication est ignorée.** Un fil ouvert par l'établissement (carnet, alerte d'absences, CPE) est lisible avec ses réponses par **tout lien actif** ; le parent interdit reçoit aussi le push, le carnet complet par e-mail et par SMS | `msg_repo.rs:75` (`prive_user_id IS NULL` laisse tout passer) ; `service.rs:17305-17309` (seul `lien_parent_eleve` est vérifié) ; `service.rs:1606-1620` ; aucune lecture de `ent_eleve_interdiction` (271) dans la messagerie (C) | Le père visé par une ordonnance lit le fil où la CPE et la mère parlent de l'enfant, et la réponse de la mère | M0a |
| **D2** | **critique** | **Le personnel lit tous les fils de l'élève**, fils privés d'un parent et relances de facture compris, dès qu'il a `messagerie.eleve` | `service.rs:17345` (`Ok(true)` sans autre filtre) (C) | Le professeur d'allemand lit le fil où la mère explique sa séparation et sa nouvelle adresse à la direction, et la relance de facture (montants) | M0a (règle provisoire), M1 (participants) |
| **D3** | **critique** | **L'élève lit et écrit dans tous les fils non privés de son dossier**, y compris ceux de l'établissement **aux parents** (alerte de seuil d'absences, carnet) ; **ses propres fils sont lus par tous ses parents**, car le fil n'est rendu privé que pour le rôle parent | `service.rs:17388` (`prive` seulement si `Role::Parent`) ; `service.rs:8586-8593` ; `17005` (C) | Un élève écrit à la CPE qu'il est battu ; le parent violent lit le message | M0a |
| **D4** | **critique** | **Le contenu part en clair hors de l'ENT** : sujet du fil en **titre du push** (Apple, Google) ; carnet : corps complet par e-mail, titre par SMS ; annonces et campagnes : corps par e-mail, SMS, push ; sujet recopié dans l'événement immuable `msg.fil_ouvert` | `service.rs:17409`, `17459`, `17040-17080`, `13480`, `campagnes.rs:725`, `17394-17400` (C) | Le titre « Signalement de harcèlement de Léa » s'affiche sur l'écran verrouillé et transite chez Apple | M0a (textes neutres), socle 8 N0 (outbox) |
| D5 | élevée | Parent **sans** `peut_voir_vs` ou sans autorité parentale : lit les fils de vie scolaire (alertes, carnet) | `msg_repo.rs:61-90` (C) | Parent à qui la direction a retiré la vie scolaire la lit dans la messagerie | M0a |
| D6 | élevée | « Ne pas solliciter » respecté par l'e-mail seulement, pas par le push (messagerie, annonces), ni par les campagnes et les rencontres | `service.rs:1606` ; `campagnes.rs:246-300` ; `foyer_repo.rs:647` (C) | Parent sous décision « ne pas solliciter » relancé par push et invité aux rencontres | M0a |
| D7 | élevée | **Carnet sans contrôle de périmètre** : tout professeur écrit sur n'importe quel élève et déclenche e-mail, SMS et push ; **tout parent lié signe**, autorité parentale non vérifiée | `service.rs:17199-17208` (test de rôle seul) ; `17232-17260` (C) | Un professeur sans lien avec l'élève envoie une observation à sa famille | M0a |
| D8 | élevée | Annonces **sans aperçu, ni confirmation, ni retrait** ; envoi synchrone en boucle sur tous les élèves, erreurs ignorées | `service.rs:13410-13510` ; `routes.rs:481` (C) | Une annonce erronée part à 900 familles sans possibilité d'arrêt ; contraire à la confirmation humaine des envois (CLAUDE.md) | M0b |
| D9 | élevée | `role_can_messagerie_staff` et `role_can_ecrire_carnet` : tests **de rôle**, pas les droits effectifs D-017 | `service.rs:17278`, `15978` (C) | Un rôle « AED » paramétré ne peut pas traiter les messages ; un profil retiré garde la messagerie | M0a |
| D10 | élevée | **Aucune RLS** sur `ent_msg*`, `ent_annonce`, `ent_carnet_entree`, `ent_campagne*`, `ent_rencontre`, `ent_creneau_rdv`, `ent_formulaire*`, `ent_notification*` | grep des migrations ; RF-20 ouvert (RS3) (C) | Une requête oubliant `etablissement_id` lit un autre établissement | M0a (permissive), M17 (stricte) |
| D11 | moyenne | Une réponse sur un fil archivé **reste dans l'archive** ; aucun désarchivage | `msg_repo.rs:279-288` ; `047:20-22` (C) | La réponse d'un parent disparaît de la boîte du personnel | M0b |
| D12 | moyenne | Boîte du personnel tronquée à **50 fils**, sans pagination ; recherche côté client sur ces 50 seulement | `service.rs:17512`, `17521` ; `modele.rs:113-127` (C) | La CPE ne retrouve pas un fil d'octobre | M0b |
| D13 | moyenne | **Le personnel n'est jamais notifié** quand une famille écrit | `reactions.rs:33-40` (C) | Un message de famille attend que quelqu'un ouvre la page | M0b |
| D14 | moyenne | Push synchrone dans la requête ; erreurs d'envoi ignorées (`let _ =`) | `service.rs:17405-17412` (C) | Panne du fournisseur invisible | M0b, socle 8 |
| D15 | faible | Accusé posé par **n'importe quel lecteur** ; « Lu par N personnes » trompeur | `service.rs:17584-17610` (C) | Le directeur ouvre le fil : la famille croit que le professeur a lu | M0b |
| D16 | faible | Taille de pièce réglable par variable mais bornée à 256 Kio en base (erreur 500 au-delà) ; aucun antivirus ; trace du lien court sans élève | `038:13` ; `upload.rs:213`, `18`, `322` (C) | Une photo de certificat (3 Mo) est refusée | M0b (cohérence), M5 (stockage objet) |
| D17 | faible | Fil ouvert depuis un modèle : aucun push ; aucune longueur maximale du sujet et du corps | `service.rs:17954-18018` ; `messagerie.rs:231-260` (C) | — | M0b |
| D18 | faible | Ciblage des annonces par **libellé** de classe, `classe_id` inutilisé | `annonce_repo.rs:73-74` (C) | Classe renommée : l'annonce n'atteint plus personne | M0b |
| D19 | faible | Aucune clé composite d'établissement entre fil, message, pièce, accusé | migrations 023, 037, 038 (C) | Cohérence tenue par le code seul | M1 |
| D20 | à vérifier | Fils attachés au mauvais élève après une fusion de fiches (stratégie « d ») | `fusion.rs:133` (S) | — | test en M0b |

Défaut voisin relevé par la recette de sécurité 3 (non fusionnée, `ent-recette-securite-3`) : **RS3-06** (ouvert) — le courrier d'orientation du psychologue part à tous les liens actifs sans vérifier les droits : même famille de défaut que D5 et D6, traité par le socle 8 (N0). La recette 3 ne relève **aucune fuite inter-établissements** dans la messagerie, ce que confirme l'audit.

### 2.3 Verdict : garder, corriger, remplacer
- **Garder** : le cloisonnement par établissement et ses matrices de tests ; le **passage obligé** par un seul contrôle d'accès ; la portée D-022 du professeur ; la chaîne S2-8 (préférences, idempotence, plafonds, droits relus) — elle devient la brique du socle 8 ; le cycle **aperçu → empreinte → confirmation** des campagnes ; les formulaires immuables et leur texte neutre ; les liens courts signés ; les écrans Leptos de la messagerie (à étendre, pas à refaire) ; les identifiants des fils et messages (stables pour la reprise).
- **Corriger tout de suite** (lot **M0**) : D1 à D18, D20.
- **Remplacer** : (1) la visibilité « calculée depuis l'élève » par des **participants nommés** (table) — c'est la seule réponse durable à D1, D2, D3 ; (2) les envois synchrones par les natures d'avis du socle 8 à texte neutre (D4, D13, D14) ; (3) les pièces en `BYTEA` par le stockage objet avec quarantaine et analyse (D16) ; (4) les annonces actuelles par les publications du socle 17 diffusées par le socle 8 (D8) ; (5) le carnet 020 par les mots du carnet (§18).

### 2.4 Ce que Next.js fait en production, et qu'il ne faut pas perdre (lecture seule, `db/schema-messaging.ts`, `db/schema-famille-messaging.ts`)
| Fonction Next.js [F Next] | Reprise dans ScolIA [R] |
|---|---|
| Messagerie interne du personnel : conversations directes et de groupe, participants, « lu jusqu'à » et « remis jusqu'à » par participant, muet, réponse à un message, transfert, modification et suppression **logique** (`edited_at`, `deleted_at`), pièces en stockage objet, réactions, présence, recherche, flux temps réel | Fils internes, participants, lecture par participant, muet, réponse citée, correction et retrait tracés, stockage objet, recherche, temps réel (M12) ; **réactions** : signalées (S-144) ; **présence** : refusée côté familles (S-137) |
| Fils famille adressés au **foyer** (`famille_thread.foyer_id`), diffusion multi-foyers (`is_broadcast`) | **Non repris tel quel** (04 §2.4) : un fil a des personnes nommées ; la diffusion est une publication (17) ou une diffusion (8) |
| Réglages : rôles autorisés à ouvrir un fil (liste JSON), professeur limité à ses classes, diffusion autorisée, pièces des familles | Remplacés par la matrice calculée et sept réglages bornés (§8.3) |

---

## 3. Acteurs, besoins réels, irritants

Ce que vit chacun, ce qui lui fait perdre du temps, ce qu'il redoute, ce qui le ravirait. Les pratiques des logiciels du marché (Pronote, École Directe, Charlemagne) sont décrites **de mémoire et par la matrice** (`MATRICE_CHARLEMAGNE.md` §3.9) : **[H]** partout, à confirmer à la passe personas.

| Acteur | Ce qu'il fait par la messagerie | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Famille (parents vivant ensemble)** | Prévenir d'un retard, d'un rendez-vous médical ; poser une question au professeur ; répondre à un mot ; demander un document | Ne pas savoir **à qui** écrire (le CPE ? la vie scolaire ? le PP ?) ; trois enfants dans deux établissements = trois boîtes ; aucune idée si le message a été lu ; un message au professeur absent reste sans réponse trois semaines | Que l'enfant lise ses messages sur la tablette familiale ; être « mal vu » ; ne jamais avoir de réponse | Écrire « à la vie scolaire de Léa » sans connaître personne ; voir « lu par l'établissement » puis « pris en charge » ; justifier l'absence **dans** la conversation (carte d'action) | hebdomadaire |
| **Parent séparé** (autre foyer) | Comme ci-dessus, en son nom propre | Les logiciels écrivent aux « responsables 1 et 2 » dans **la même** discussion : chacun lit les réponses de l'autre ; ou l'oublient (« le père n'a jamais rien reçu ») | Que l'autre parent lise ce qu'il écrit (nouvelle adresse, conflit, violence) ; être traité en parent de seconde zone | Un fil à lui, la même information que l'autre, et la phrase « les deux responsables ont été informés » sur les messages de l'établissement (M-358 du socle 8) | hebdomadaire |
| **Parent « à ne pas solliciter »** (décision de la direction, socle 4 / socle 8 Q-N9) | Écrit parfois, souvent en conflit | — | Être relancé, invité, sondé | Qu'on lui réponde quand il écrit, sans le solliciter autrement | rare |
| **Parent visé par une interdiction de communication** (décision de justice, ordonnance) | Ne doit rien recevoir ni rien obtenir sur l'enfant concerné | Aujourd'hui : rien ne l'empêche si son compte est encore lié | — (c'est l'autre parent qui redoute) | — | rare mais grave |
| **Élève mineur** | Écrire à un professeur (devoir, absence), à la CPE (problème), à l'infirmière, au psychologue ; recevoir les mots de la vie scolaire | Pas de canal discret pour dire « je suis harcelé » ; il passe par un réseau social | Que ses parents ou ses camarades lisent ; qu'un adulte lui réponde à 23 h ; qu'on le juge | « Parler à quelqu'un » en deux gestes, savoir qui lira et quand ; signaler un message blessant | quotidienne au lycée |
| **Élève majeur** | Tout ce que fait un parent, pour lui-même ; il justifie ses absences | Les parents continuent de recevoir sans qu'il ait été consulté, ou ne reçoivent plus rien sans prévenir (PERSONAS_08, Hugo) | Que ses parents lisent ses échanges avec l'établissement | Choisir clairement ce que ses parents continuent de recevoir (socle 4, Q-A7) | hebdomadaire |
| **Professeur** | Écrire aux familles des élèves de **ses cours** (D-022) ; répondre ; mots sur le carnet ; échanger avec l'équipe | Écrire vingt fois le même message (« devoir non rendu ») ; ne pas savoir que les parents sont séparés ni qu'un parent est interdit ; recevoir des messages à 22 h et le dimanche ; groupe WhatsApp personnel avec les parents pour la sortie (PERSONAS_08 P07) | Écrire au mauvais parent ; un parent agressif ou harcelant ; une capture d'écran sortie de son contexte ; être accusé d'une relation inappropriée avec un élève | « Écrire aux familles des 3 élèves qui n'ont pas rendu le devoir » en un geste, **trois fils distincts** ; le logiciel choisit les bons destinataires ; heures de disponibilité respectées des deux côtés | quotidienne |
| **Professeur principal** | Familles de toute la classe ; fil de l'équipe pédagogique de la classe ; préparation du conseil | Constituer à la main la liste des collègues de la classe ; relayer les mêmes informations à chaque collègue | Oublier un collègue, un remplaçant | Le **fil d'équipe de la 4B** tenu à jour seul par le socle 6 (arrivée d'une suppléante, départ d'un collègue) | hebdomadaire |
| **CPE** | Destinataire principal des familles (absences, conflits, harcèlement) ; mots aux familles ; coordination avec PP et direction | Une boîte personnelle qui déborde ; quand elle est absente, personne ne voit rien ; recopier l'absence signalée dans le module d'appel | Un message qui révèle un danger, lu le lundi ; un fil perdu dans un départ | Une **boîte de service** « vie scolaire » partagée avec les AED, avec prise en charge, statut, échéance, modèles de réponse, et la carte « enregistrer cette absence » | plusieurs fois par heure |
| **Vie scolaire (AED)** | Trier la boîte, répondre aux messages simples, enregistrer les absences signalées | Ne pas savoir qui a déjà répondu ; répondre deux fois | Lire ce qu'il ne devrait pas (infirmerie, situation familiale) | La prise en charge visible (« Paul s'en occupe ») ; ce qu'il n'a pas à voir, il ne le voit pas | quotidienne |
| **Direction** | Escalades, réclamations, parents agressifs, signalements, crise ; supervision | Se faire mettre en copie de tout ; ne rien voir de ce qui compte | Une relation inappropriée adulte–élève non détectée ; une fuite d'adresse protégée ; un message de menace effacé ; une plainte sans preuve | Ne rien lire par défaut, **tout pouvoir retrouver avec un motif tracé** ; un tableau des délais de réponse ; un gel en un geste quand un fil devient une affaire | hebdomadaire, crises |
| **Secrétariat** | Questions administratives (certificats, inscriptions, bourses) ; boîte « secrétariat » | Les mêmes questions cent fois ; envoyer un document en pièce jointe par e-mail | Envoyer un document au mauvais parent | Répondre par une **carte « document »** du coffre (socle 7), jamais une pièce jointe libre | quotidienne |
| **Infirmière** | Échanges avec élèves et familles (traitement, PAI, retour à la maison) ; avec la CPE sans détail médical | Tout passe par le téléphone ; la messagerie générale n'est pas confidentielle | Que la vie scolaire ou la direction lisent un échange de santé ; héberger des données de santé sans HDS (`VERIF_REGLEMENTAIRE.md` R23) | Une boîte « infirmerie » cloisonnée (C3), invisible de toute supervision ordinaire | quotidienne |
| **Psychologue de l'éducation nationale** | Rendez-vous, échanges avec élèves et familles | Secret professionnel mal protégé par les outils généraux | Qu'un courrier ou un message soit lu par un tiers (défaut connu : courrier du psychologue en pièce jointe, socle 8 N0) | Boîte « psychologue » C3, sans supervision ordinaire, conservation propre | hebdomadaire |
| **AESH** | Échanger avec les professeurs et le PP sur l'élève accompagné ; parfois avec la famille | Ne pas être dans les canaux de l'équipe ; passer par le cahier papier | Écrire à une famille sans cadre ; voir des élèves qu'il n'accompagne pas | Être **membre du fil d'équipe de l'élève accompagné** (socle 6 / 35), avec un cadre clair pour les familles | quotidienne |
| **Accueil** | Prendre un appel (« Mme X a appelé, rappeler », « Léo sera absent ») et le transmettre | Post-it, Excel partagé, téléphone qui sonne dans le vide (D-023, exemple de Florian) | Transmettre au mauvais service ; lire ce qui ne le regarde pas | « Message téléphonique » en trois champs, routé vers la boîte du bon service ou la personne, avec la carte « absence signalée » | plusieurs fois par heure |
| **Intendance, comptabilité** | Questions de factures, d'échéanciers, de cantine | Messages de factures mêlés aux messages pédagogiques | Écrire à un non-payeur ; révéler la situation financière à l'autre parent | Boîte « intendance » ; fils de facture aux seuls **payeurs** (RP1-11) | hebdomadaire |
| **Référent harcèlement, équipe pHARe** | Recevoir les signalements, suivre une situation | Signalements dispersés (oral, e-mail, mot) | Perdre la preuve ; exposer l'élève qui signale | Signalement qui garde la preuve (gel), cloisonné, suivi daté | ponctuelle |
| **Assistante sociale, conseiller d'orientation, documentaliste, DDFPT, chef de travaux** | Boîtes de service selon les missions (socle 6) | — | Pour l'assistante sociale : lecture par des tiers (C3) | Une boîte par mission, attribuée par le socle 6 | variable |
| **Tiers sans compte** (tuteur de stage, référent ASE, interprète, médecin) | Répondre à une question, déposer une pièce | E-mail personnel du professeur | — | Lien nominatif à usage limité (socle 7 R-D8, socle 8 §10.5), jamais une adresse personnelle | rare |
| **Assistant IA** (socle 30, IA de cœur) | Aider à rédiger, résumer, trier, traduire | — | — | **Jamais** plus de droits que la personne ; jamais d'envoi sans geste humain (`IA_COEUR.md` §6.2, §8.2) | — |

**Ce que montrent ces portraits [R].** (1) Les familles écrivent à **des services** plus qu'à des personnes : la boîte de service est le cœur du domaine, pas une option. (2) Le danger principal n'est pas technique : c'est **écrire à la mauvaise personne** ou **laisser lire la mauvaise personne** ; la sélection des destinataires doit être faite par le logiciel à partir des faits (socles 4, 5, 6), jamais par une liste à cocher. (3) Les personnels ont besoin de **frontières** (horaires, absence, protection contre le harcèlement) autant que de fonctions. (4) La messagerie est l'endroit où **les actions arrivent** (justifier, signer, prendre rendez-vous, payer) : c'est la surface du futur « chat first » (socle 30).

---

## 4. Cas limites (catalogue)

Chaque cas doit être traité **sans contournement** et avoir un test ou une règle. Renvois : règles R-M (§20), actions MSG (§19).

### 4.1 Familles, parents séparés, protections

| # | Situation | Ce qui doit se passer [R sauf mention] |
|---|---|---|
| CM-01 | La mère ouvre un fil avec la CPE et y donne sa nouvelle adresse | Fil privé : participants = la mère + la boîte vie scolaire ; le père ne voit ni le fil, ni son sujet, ni son existence, ni son compteur (RP1-11 étendu à toutes les routes) |
| CM-02 | Le professeur veut « écrire aux parents de Léo », parents séparés, deux foyers | **Deux fils jumeaux** (un par foyer), même texte, réponses séparées ; l'écran le dit (« 2 conversations : foyer de Mme A, foyer de M. B ») ; aucune adresse d'un foyer visible de l'autre (R-M4) |
| CM-03 | Parents vivant ensemble, un seul foyer | Un fil commun au foyer est permis (les deux parents y participent) ; chacun peut demander à sortir du fil commun et à avoir le sien (MSG-118) |
| CM-04 | Parents ensemble au départ, séparation déclarée en cours d'année | À la création du second foyer (socle 4 `foyer.cree`), les fils communs **restent lisibles** par les deux pour le passé ; tout nouveau message part en fils séparés ; le personnel voit « foyer divisé le 12/01 » |
| CM-05 | Interdiction de communication posée sur le père (ordonnance) pour Léo | Le père est retiré de tous les fils « à propos de Léo » (participation close, motif non affiché) ; il ne peut plus en ouvrir ; il ne reçoit plus d'avis ; les fils d'un autre enfant (non visé) restent ouverts ; le personnel qui tenterait de l'ajouter reçoit « destinataire exclu (décision) » sans motif (R-M6) |
| CM-06 | Interdiction posée **après** des échanges : le père a lu des fils où la mère est intervenue ? | Ne doit pas arriver (fils séparés) ; si un ancien fil commun existe, il est **figé** pour le père (lecture close) et la direction est alertée de l'existence d'un fil commun antérieur |
| CM-07 | Fil « à propos de » **deux** enfants d'une fratrie, interdiction sur un seul | Le fil est rattaché à chaque enfant ; une interdiction sur l'un retire la personne du fil entier ; l'écran propose de scinder (MSG-139) |
| CM-08 | Adresse protégée de la mère : le professeur écrit aux deux foyers | Aucune coordonnée dans les fils, les aperçus, les accusés, les exports ; la signature automatique d'un message ne contient jamais l'adresse d'un destinataire |
| CM-09 | Demande d'un parent « je ne veux pas que son père soit informé », sans décision de justice | Pas d'effet automatique sur la messagerie (C. civ. 372, 372-2 ; socle 4 §8.8, VERIF R9) ; signalement transmis à la direction ; message neutre au parent |
| CM-10 | Parent sans exercice de l'autorité parentale | Il **doit être informé des choix importants** (C. civ. 373-2-1 [V]) : il reçoit, dans ses propres fils, les informations dues (bulletins, absences, sanctions, orientation, décisions importantes), mais pas « tous les détails de la vie scolaire » (brochure DGESCO [V], §5 R-4) ; il peut **écrire à la direction et au secrétariat**, qui lui répondent (C. éduc. D. 111-4 [V]) ; il n'écrit pas aux professeurs et n'est pas destinataire de leurs messages ordinaires, sauf ajustement de la direction ; refuser l'ENT à ce parent est légal s'il reste informé (TA Toulouse 2022 [V]) ; le titulaire voit « l'autre parent reçoit les informations dues » |
| CM-11 | Parent « à ne pas solliciter » | L'établissement n'**ouvre** aucun fil vers lui, ne l'inclut dans aucune annonce ni campagne ; s'il écrit, on lui **répond** dans son fil (réponse ≠ sollicitation) ; une information due passe par la décision de la direction (socle 8 R-N5) |
| CM-12 | Parent décédé | Ses participations sont closes à la date ; aucun avis, aucune relance ; ses messages passés restent dans les fils (archives) ; aucun fil jumeau créé vers lui |
| CM-13 | Enfant confié à l'ASE | Le référent du service gardien et l'assistant familial participent selon leurs droits de lien ; les parents selon la décision ; l'adresse d'accueil n'apparaît jamais ; fils séparés par défaut |
| CM-14 | Beau-parent avec droit `messagerie` mais sans autorité | Peut participer au fil de son foyer ; ne reçoit pas les natures réservées aux titulaires (convocation) ; affichage « beau-père » |
| CM-15 | Lien non confirmé par la personne (compte existant, lien en attente) | Aucun fil, aucun accès (socle 4 ADM-08) ; le personnel voit « lien non confirmé » |
| CM-16 | Lien suspendu (décision provisoire) | Participation suspendue (pas close) : ni lecture ni écriture ; reprise automatique à la levée, sans les messages échangés pendant la suspension sauf décision |
| CM-17 | Désaccord déclaré entre parents (socle 4 C23) | Aucun effet sur la messagerie ; les cartes d'action qui exigent l'accord des titulaires (formulaire, signature) se bloquent avec un message neutre |
| CM-18 | Un parent écrit au nom de l'autre (« mon ex et moi pensons que… ») | Rien à faire techniquement ; le fil reste le sien ; la réponse ne va qu'à lui |
| CM-19 | Un parent harcèle un professeur (vingt messages par jour, insultes) | Signalement par le professeur (MSG-221) ; la direction peut **restreindre** : écriture limitée à une boîte de service, ou fil en lecture seule, motivé, daté, réexaminé (MSG-226) ; le parent garde l'information due |
| CM-20 | Parent qui écrit dans une langue étrangère | Traduction proposée au lecteur, marquée « traduction automatique » ; la réponse peut être traduite, **relue et confirmée** avant envoi (§16) |
| CM-21 | Parent non lecteur, sans numérique | Le fil n'est pas le bon canal : le personnel voit « joindre autrement » (socle 8 NTF-64) ; le message téléphonique de l'accueil crée un fil interne de suivi |
| CM-22 | Famille qui répond par SMS à un avis | Réponse entrante rattachée au numéro (personne **présumée**), déposée dans la boîte du service émetteur comme message « non vérifié » (socle 8 CN-22) ; jamais une justification automatique |
| CM-23 | Famille qui répond à l'e-mail d'avis | L'adresse de réponse n'est jamais celle d'un personnel (socle 8 §10.3) ; réponse renvoyée vers « répondez dans ScolIA » ou, si l'établissement l'active, déposée dans la boîte comme message non vérifié (Q-M14) |
| CM-24 | Tuteur légal d'un parent majeur protégé | Participe selon la mesure (socle 4 C42 [NV]) |

### 4.2 Élèves

| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CM-25 | Élève mineur qui écrit à son professeur | Permis si la matrice l'ouvre (défaut : vers ses professeurs, son PP, la vie scolaire, la CPE, l'infirmerie, le psychologue, le CDI) ; le fil est un **fil encadré** : il n'est lisible que de ses participants, mais la direction peut l'ouvrir avec un motif tracé (§10.2) ; l'élève le sait (mention permanente) |
| CM-26 | Élève qui écrit à un adulte à 23 h | Le message est enregistré ; l'adulte n'est pas prévenu avant 7 h ; l'élève voit « sera lu à partir de demain 7 h » ; **aucun message d'un adulte à un élève n'est remis entre 20 h et 7 h** ni du vendredi 20 h au lundi 7 h (référentiel du numérique responsable, exigence 8 [V], §5 R-18) ; seule la direction peut lever le différé pour une urgence, tracé (exigence 9) (R-M21) |
| CM-27 | Adulte qui écrit en tête-à-tête à un élève mineur, de façon répétée, hors de tout cours | Pas d'interdiction, mais un **indicateur de vigilance** (déterministe : fréquence, horaires, absence de lien d'enseignement) visible du seul référent désigné (direction), jamais de l'adulte ni de l'élève, sans lecture du contenu (R-M23, Q-M7) |
| CM-28 | Élève qui veut écrire à un autre élève | **Fermé par défaut** (Q-M6) : pas de messagerie libre entre élèves dans ScolIA ; travail de groupe : espaces de classe modérés (socle 32) ; délégués : fil des délégués avec un adulte référent |
| CM-29 | Élève qui reçoit un message blessant d'un camarade (si l'établissement a ouvert élève–élève) | Signaler en un geste (MSG-220) ; le message est gelé (preuve) et masqué pour la victime ; le référent harcèlement est prévenu ; le 3018 est rappelé à l'élève (§14) |
| CM-30 | Élève qui révèle des violences à la maison dans un message à la CPE | Le fil est **gelé** (pas de suppression possible, R-M30) ; la CPE dispose du parcours « information préoccupante » (socle 36) ; **aucun avis** n'est envoyé à la famille (socle 8 R-N58) ; le fil n'est jamais visible des parents |
| CM-31 | Élève qui écrit à l'infirmière | Boîte infirmerie (C3) ; invisible de la vie scolaire, de la direction et des parents ; supervision ordinaire impossible ; seule l'infirmière (et sa remplaçante désignée) lit |
| CM-32 | Élève qui devient majeur | Le jour de la majorité : ses fils restent les siens ; les fils de ses parents « à propos de lui » restent ouverts si leur lien garde `messagerie` (choix de l'élève majeur, socle 4) ; à défaut, ils sont clos pour l'avenir avec message neutre (« les échanges passent désormais par votre enfant ») ; ELV-39 l'informe à J-30 |
| CM-33 | Élève majeur qui interdit à ses parents de recevoir ses absences | Les fils de la vie scolaire sur ses absences vont à lui seul ; un parent qui écrit reçoit une réponse qui ne révèle aucune donnée de l'élève sans son accord (alerte de rédaction au personnel, R-M12) |
| CM-34 | Élève qui écrit depuis le compte d'un parent (ou inversement) | Pas détectable en certitude ; le fil porte le compte ; un parent qui signale un usage frauduleux déclenche la sécurité du compte (socle 1) |
| CM-35 | Élève qui change de classe ou de groupe | Les fils avec ses anciens professeurs restent lisibles ; un ancien professeur ne peut plus en **ouvrir** de nouveau (D-022 à la date) ; il peut répondre pendant 15 jours dans un fil existant (paramètre borné, S-140) |
| CM-36 | Élève qui quitte l'établissement | Compte fermé (socle 3) ; ses fils sont clos et entrent en conservation ; les fils de ses parents aussi ; une famille partie peut demander l'export de ses échanges (droit d'accès, MSG-245) |
| CM-37 | Élève à confidentialité renforcée (socle 2 §7.6) | Ne figure dans aucune liste de destinataires hors des personnes autorisées ; un fil d'équipe ne cite pas sa nouvelle classe ; le moteur de recherche ne le trouve pas pour un tiers |
| CM-38 | Élève d'un groupe de LV2 dans une classe mixte | Le professeur d'allemand n'écrit qu'aux familles des élèves de son groupe (D-022) ; l'aperçu le montre |

### 4.3 Personnels

| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CM-39 | Professeur en congé de maladie trois semaines | Les familles qui lui écrivent voient « absent jusqu'au 3 avril ; votre message est transmis à M. X / à la vie scolaire » (réponse d'absence, sans motif) ; ses fils ouverts sont **visibles du suppléant** désigné par le socle 6 pour les seuls élèves du service, à partir de la date de suppléance (R-M17) ; aucune notification au titulaire pendant l'absence (socle 8 CN-29) |
| CM-40 | Suppléante qui arrive un lundi | Elle hérite des fils du service (élèves et familles du cours) ; pas des fils personnels du titulaire avec des collègues |
| CM-41 | Départ d'un personnel (mutation au 1ᵉʳ septembre) | Ses fils ouverts avec des familles passent au **successeur sur le service** ou à la boîte du PP / vie scolaire, avec message neutre aux familles ; ses fils internes sont clos ; aucune perte ; ses messages restent signés de son nom |
| CM-42 | Décès d'un personnel | Comme le départ, sans aucun message automatique ; information rédigée par la direction (socle 6 P8) |
| CM-43 | Suspension conservatoire d'un agent | Accès coupé, fils redistribués sans motif ; ses fils sont **gelés** automatiquement (preuve) |
| CM-44 | CPE qui part le vendredi à 17 h, message grave à 18 h | La boîte de service le reçoit ; le personnel d'astreinte (si l'établissement en a désigné) est prévenu pour les seuls messages marqués « urgent » par le tri déterministe (mots d'alerte) ; la famille voit les horaires de la boîte et le numéro d'urgence (3018, 119, 15, 17) affiché en permanence (R-M26) |
| CM-45 | Message d'un professeur à une famille à 22 h | **Différé** au lendemain 7 h (et du vendredi 20 h au lundi 7 h), conformément au réglage **par défaut** exigé des outils ENT et vie scolaire (référentiel du numérique responsable §4.4, exigence 8 [V]) ; le professeur le voit avant d'envoyer (« partira lundi à 7 h ») ; pas d'« envoyer maintenant » pour lui : seule la direction déroge pour une urgence, tracé (exigence 9) |
| CM-46 | Professeur qui reçoit un message d'une famille un samedi | Aucune notification avant sa disponibilité (socle 8 §16) ; la famille voit la mention |
| CM-47 | Professeur qui partage son téléphone personnel avec les familles | Aucun champ ne l'affiche ; la messagerie et la diffusion légère depuis le mobile (socle 8 NTF-136) rendent le contournement inutile |
| CM-48 | AESH qui veut écrire à la famille | Selon le paramètre de l'établissement (défaut : **participer** au fil ouvert par le PP ou le professeur, ne pas en ouvrir seul — Q-M11) |
| CM-49 | Accueil qui prend un appel « Léo est malade » | Message téléphonique (MSG-171) : personne présumée, boîte vie scolaire, carte « absence signalée » (socle 12) ; l'accueil ne lit pas la suite du fil |
| CM-50 | Personnel qui cumule CPE et professeur | Ses deux casquettes (socle 6) : la boîte vie scolaire et ses fils de professeur ; l'écran sépare les deux |
| CM-51 | Professeur qui enseigne dans deux établissements ScolIA | Deux messageries distinctes (D-013 : aucune visibilité entre établissements) ; « mes établissements » pour basculer ; jamais un fil qui traverse deux établissements |
| CM-52 | Fil d'équipe de la 4B quand un collègue part | Retiré du fil à la date de fin de son intervention (socle 6) ; il ne lit plus la suite ; ce qu'il a écrit reste |
| CM-53 | Délégation de direction (intérim) | L'adjoint reçoit les droits de supervision pendant la délégation (socle 6 P33), puis plus rien |

### 4.4 Messages, pièces, lecture, conservation

| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CM-54 | Professeur qui envoie un message au mauvais fil | **Retrait** (masquage) dans les 10 minutes avec trace « message retiré par son auteur » ; au-delà, retrait par modération seulement ; le contenu reste conservé (preuve) et lisible de la direction avec motif (R-M28) |
| CM-55 | Faute de frappe à corriger | Correction possible pendant 15 minutes **si personne n'a lu** ; sinon nouveau message ; chaque correction est versionnée et marquée « modifié » (R-M27) |
| CM-56 | Parent qui veut « effacer » un message envoyé sous le coup de la colère | Retrait par son auteur dans les 10 minutes ; au-delà, demande de retrait à l'établissement ; jamais de suppression physique (le message peut être une pièce d'une procédure) |
| CM-57 | Pièce jointe : photo d'un certificat médical prise au téléphone (8 Mo, HEIC) | Acceptée (taille plafonnée à 20 Mo), convertie (socle 7, D-005) ; **quarantaine** jusqu'à l'analyse ; téléchargement par URL pré-signée courte après contrôle des droits ; si c'est un justificatif d'absence, la carte « joindre à l'absence » le classe au bon endroit |
| CM-58 | Pièce jointe infectée | Refusée après analyse ; l'auteur et le destinataire voient « pièce refusée (sécurité) » ; jamais servie |
| CM-59 | PDF protégé par mot de passe ou à contenu actif | Refusé ou désarmé (socle 7 D16) |
| CM-60 | Document sensible envoyé en pièce jointe par un personnel (bulletin, compte rendu) | Refusé pour les natures du coffre : le personnel joint une **carte document** du socle 7, lisible selon les droits du destinataire au moment de la lecture (R-M33) |
| CM-61 | Accusé de lecture d'un parent qui n'a jamais ouvert l'appli | « Non lu » ; l'escalade est celle du socle 8 (pas de relance infinie) ; le personnel voit « non lu après 3 jours » |
| CM-62 | Famille qui veut savoir si son message a été lu | Elle voit « lu par la vie scolaire » (le service, pas la personne ni l'heure exacte pour une boîte ; la personne pour un fil individuel — Q-M8) |
| CM-63 | Mot du carnet qui exige une signature | Carte « j'ai pris connaissance » signée par un titulaire ; relance par le socle 8 ; un seul titulaire suffit sauf règle contraire ; la signature d'un mineur ne vaut pas pour un mot destiné aux parents |
| CM-64 | Demande de communication des messages par un parent (droit d'accès, RGPD art. 15) | Export des **fils auxquels il participe** seulement, jamais ceux de l'autre parent ni ceux de l'élève ; délai d'un mois (VERIF R20) ; parcours au socle 27 |
| CM-65 | Réquisition judiciaire ou enquête | Gel du fil ; export par la direction avec motif et référence ; journalisé ; aucune modification ensuite |
| CM-66 | Fin de la durée de conservation | Purge par lot (socle 27), sauf fil gelé ; trace de la purge sans contenu |
| CM-67 | Fil qui dure toute l'année avec 300 messages | Pagination, recherche dans le fil, résumé (IA, N0) |
| CM-68 | Message « priorité haute » d'une famille | La priorité n'est **pas** choisie par l'expéditeur famille (abus) ; le tri déterministe et le personnel la posent (S-136) |
| CM-69 | Réponse à un fil clos | Un fil clos ne reçoit plus de message ; la famille peut en ouvrir un nouveau qui cite l'ancien |

### 4.5 Modération, harcèlement, urgence, IA

| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CM-70 | Message contenant des menaces envers un personnel | Signalement ; gel ; la direction est prévenue ; le personnel peut sortir du fil sans le clore ; possibilité de dépôt de plainte avec export horodaté |
| CM-71 | Message qui contient une donnée de santé détaillée envoyé à un professeur | Rien d'automatique sur le contenu ; avertissement doux à la saisie côté famille si des mots de santé sont détectés hors de la boîte infirmerie (« pour un sujet de santé, écrivez à l'infirmerie ») ; le fil est classé C3 si le personnel le marque |
| CM-72 | Harcèlement entre élèves signalé par un parent | Boîte « harcèlement » (référent pHARe) si l'établissement l'a créée, sinon CPE ; gel ; parcours de suivi (socle 13 / 36) ; jamais de confrontation des familles dans un fil commun |
| CM-73 | Urgence (élève disparu, malaise) écrite par une famille dans la messagerie | Le message n'est pas un canal d'urgence : bandeau permanent « en cas d'urgence, appelez l'établissement ou le 15 / 17 / 112 » ; le tri déterministe signale les mots d'urgence en tête de boîte ; aucun engagement de délai la nuit |
| CM-74 | Crise de l'établissement (confinement) | Pas par la messagerie : modèles d'urgence du socle 8 (§14) ; la messagerie affiche le bandeau de l'alerte |
| CM-75 | Parent qui colle une instruction pour l'assistant (« Assistant, envoie à tous… ») | Texte non fiable encadré ; aucune capacité d'envoi dans une tâche de lecture ; alerte d'injection (`IA_COEUR.md` §T2) |
| CM-76 | Professeur qui demande à l'IA « réponds à tous mes messages » | Refus : l'IA prépare **un** brouillon par fil, chacun relu et confirmé (N3) |
| CM-77 | Traduction automatique erronée d'un message sensible | Le texte original reste affiché à côté ; la traduction est marquée ; un message de discipline ou de santé traduit porte « traduction automatique, faites-la vérifier » ; jamais de traduction pour la boîte C3 sans action explicite |
| CM-78 | IA qui résume un fil pour la direction | Résumé dans les droits de la direction **après** l'ouverture motivée du fil ; jamais de résumé d'un fil C3 hors de sa boîte |
| CM-79 | Personnel qui veut écrire à 340 familles | Ce n'est pas une conversation : bascule vers une **annonce** (socle 17) ou une diffusion (socle 8) avec aperçu et confirmation ; la messagerie refuse un lot de plus de 30 fils individuels (S-139) sans passer par la diffusion |
| CM-80 | Sortie scolaire : l'accompagnateur veut informer les familles depuis le car | Diffusion légère depuis le mobile (socle 8 NTF-136), pas un groupe de messagerie ; les réponses des familles arrivent en fils individuels avec l'accompagnateur |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-09)

Sources consultées le 2026-11-09 par un sous-agent de recherche ; les PDF officiels ont été téléchargés et lus en texte. Pages bloquées aux robots : HTML d'education.gouv.fr (BO), d'eduscol, EUR-Lex. Ce tableau ne remplace ni le juriste ni le DPO (colonne « Avis » comme dans `VERIF_REGLEMENTAIRE.md`).

| # | Règle | Référence | Source | Effet sur la conception | Avis |
|---|---|---|---|---|---|
| R-1 | L'autorité parentale s'exerce dans l'intérêt de l'enfant, qui est associé aux décisions selon son âge et sa maturité | C. civ. 371-1 (loi 2024-120) **[V]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000038749626 | Les échanges de l'élève ne sont pas mécaniquement ceux de ses parents (R-M9, Q-M5) | J, DPO |
| R-2 | Exercice en commun, présomption d'accord pour les actes usuels, séparation sans incidence sur l'exercice ; le parent autorisé à dissimuler son domicile n'informe pas l'autre d'un déménagement ; le parent sans exercice garde un droit de surveillance et « doit être informé des choix importants » | C. civ. 372, 372-2, 373-2 (loi 2024-233), 373-2-1, 373-2-9 **[V]** | https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006070721/LEGISCTA000006117839/ | Trois statuts (titulaire, sans exercice, tiers) ; fils séparés ; aucune coordonnée d'un foyer chez l'autre (R-M3, R-M4, CM-10) | — |
| R-3 | Les parents sont membres de la communauté éducative ; information régulière ; **toute demande d'information ou d'entrevue reçoit une réponse, un refus est motivé** | C. éduc. L. 111-4, D. 111-3, D. 111-4 **[V]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000038901844 ; https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166752/ | Suivi des demandes, réponse exigée avant « traité » (R-M45), pilotage des demandes sans réponse | — |
| R-4 | Doctrine ministérielle : mêmes relations avec les deux parents ; parents séparés : **envoyer systématiquement à chacun les mêmes documents et convocations** ; parent sans exercice : bulletins, absences, sanctions, orientation, décisions importantes, mais pas « tous les détails de la vie scolaire », le titulaire en est informé ; seule une décision du JAF fait obstacle au droit de surveillance | brochure DGESCO et médiateur « L'exercice de l'autorité parentale en milieu scolaire » (2011, « à actualiser ») **[V]** ; circulaires 94-149 et 2006-137 citées **[NV]** | https://eduscol.education.gouv.fr/sites/default/files/document/autoriteparentale170278pdf-68094.pdf | Dédoublement par foyer des informations dues (§9.1 point 9) ; périmètre du parent sans exercice (CM-10, Q-M22) | J |
| R-5 | Refuser l'accès à l'ENT à un parent privé de l'exercice est légal s'il reste informé (bulletins, absences) | TA Toulouse, 18 mai 2022, n° 1905811 (LIJ n° 223) ; TA Orléans, 1ᵉʳ février 2022, n° 1902557 **[V]** | https://www.education.gouv.fr/sites/default/files/document/LIJ%20223_janvier%202023.pdf-342726.pdf | La messagerie de ce parent peut être restreinte, l'information due jamais (CM-10) | — |
| R-6 | Aucun texte ne permet à un parent de lire les échanges de l'autre ; les messages de l'ENT sont de la correspondance privée (R-11) ; le droit d'accès ne porte pas atteinte aux droits d'autrui | analyse sur R-10, R-11, R-15 **[V]** | — | Pas de boîte « famille » partagée par défaut ; export expurgé des tiers (§15 point 5) | DPO |
| R-7 | Ordonnance de protection : interdiction d'entrer en relation, dissimulation du domicile, 12 mois prolongeables ; contrôle judiciaire : s'abstenir d'entrer en relation | C. civ. 515-11 (loi 2024-536) **[V]** ; 515-13-1 **[NV]** ; C. proc. pén. 138 9° **[NV]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049715065 | Interdiction saisie par la direction avec pièce et date de fin, relue partout ; message d'erreur neutre (R-M6) ; aucun texte de l'Éducation nationale n'organise la mise en œuvre | J |
| R-8 | Majorité à 18 ans ; l'élève majeur justifie ses absences ; **aucun texte trouvé** sur « parents destinataires sauf opposition de l'élève majeur » | C. civ. 414 **[NV]** ; guide juridique du chef d'établissement (2009), fiche 34 **[NV]** | — | La messagerie lit les droits conservés du socle 4 (Q-A7) et ne décide rien seule (§10.3) | J, DPO |
| R-9 | Consentement d'un mineur à 15 ans : seulement pour les services fondés sur le consentement ; information claire pour le mineur | loi 78-17 art. 45 **[V]** ; RGPD art. 8 **[V]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037823135 ; https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre2 | ENT fondé sur la mission d'intérêt public (VERIF R18) ; charte courte lisible par un élève (MSG-129) ; option IA facultative éventuelle | DPO |
| R-10 | Ouvrir, supprimer, **retarder**, détourner ou prendre frauduleusement connaissance d'une correspondance : délit ; aggravé pour une personne chargée d'un service public « hors les cas prévus par la loi » | C. pén. 226-15, 432-9 **[V]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042193573 ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006418513 | Supervision motivée, prévue par la charte, journalisée (R-M24) ; différé annoncé, jamais silencieux (§11.3) ; aucune suppression par un utilisateur (R-M28) | **J** |
| R-11 | Les messages de l'ENT « relèvent par principe du droit de la correspondance privée » ; contrôle du chef d'établissement **mentionné dans la charte**, personnes désignées, « vie privée résiduelle » ; espaces collectifs : lecture et retrait des contenus inappropriés ; journalisation des accès privilégiés | kit juridique du SDET 2025 (mis à jour février 2026) §4.4, 4.5, 4.7 ; annexe opérationnelle du SDET 2024 §3.8.3 **[V]** | https://www.education.gouv.fr/sites/default/files/2025-07/kit-juridique-441540.pdf ; https://eduscol.education.fr/document/58941/download | Deux régimes : fils privés (supervision exceptionnelle) et fils collectifs de classe et de délégués (modération ordinaire) ; journal non modifiable (R-M24, §10.4) | J |
| R-12 | Le salarié a droit au secret de ses correspondances même au travail ; les messages de l'outil professionnel sont présumés professionnels sauf identification comme personnels | Cass. soc. 2 oct. 2001, n° 99-42.942 ; 16 mai 2013, n° 12-11.866 **[V]** ; 23 oct. 2019, n° 17-28.448 **[NV]** | https://www.legifrance.gouv.fr/juri/id/JURITEXT000007046161/ ; https://www.legifrance.gouv.fr/juri/id/JURITEXT000027430700/ | Marque « personnel » sur un fil interne, respectée par toute supervision (R-M46) ; transposition aux agents publics non vérifiée | J |
| R-13 | Arrêté ENT : finalités (dont échanges), données par catégorie, parents destinataires de la vie scolaire **de leurs enfants** ; **suppression dans les 3 mois après la perte du droit au compte** ; **aucune durée propre à la messagerie** | arrêté du 30 novembre 2006 modifié (dernière modification 13 octobre 2017), art. 6 **[V]** | https://www.legifrance.gouv.fr/loda/id/JORFTEXT000000427578 | Purge 3 mois après la sortie, après versement au dossier (§15 point 3) | DPO |
| R-14 | Données du compte « pour la durée de l'année scolaire ou, au besoin, du cycle » ; traces de connexion **12 mois au plus** ; durées détaillées renvoyées à un « kit de conservation » introuvable ; **rien sur les accusés de lecture** | kit juridique du SDET 2025 **[V]** ; kit de conservation **[NV]** (403) | (même URL que R-11) | Journaux 12 mois ; accusés : choix de minimisation (§13) | DPO |
| R-15 | RGPD : copie des données sans atteinte aux droits d'autrui (15 §4) ; pas d'effacement en cas de mission d'intérêt public, d'archivage ou de défense en justice (17 §3) ; pas de décision uniquement automatisée (22) ; protection dès la conception et par défaut, pas d'accès à un nombre indéterminé de personnes (25) ; AIPD si risque élevé (35) ; données de santé interdites sauf exceptions (9) | RGPD art. 8, 9, 15, 17, 22, 23, 25, 35 **[V]** (copie CNIL) ; art. 5 et 6 **[NV]** | https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre3 ; …/chapitre4 | Export expurgé ; effacement encadré (MSG-254) ; IA sans décision (§16) ; **AIPD de la messagerie** (mineurs, santé possible, IA) ; avertissement santé hors infirmerie (CM-71) | DPO |
| R-16 | « Aucun élève ne doit subir de faits de harcèlement » : prévenir, détecter, orienter, informer chaque année, y compris sur le cyberharcèlement ; délit de harcèlement scolaire (jusqu'à 10 ans) ; **L. 511-3-1 abrogé** ; 3018 gratuit et anonyme, 7 j/7 de 9 h à 23 h | loi 2022-299 ; C. éduc. L. 111-6 ; C. pén. 222-33-2-3 **[V]** ; service-public A17444 **[V]** ; pHARe, remplacement du 3020 **[NV]** | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000045287658 ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000045289333 ; https://www.service-public.gouv.fr/particuliers/actualites/A17444 | Signalement, gel, export horodaté, 3018 affiché ; élève–élève fermé (§10.4, §14) | — |
| R-17 | Information préoccupante transmise sans délai au président du conseil départemental (CRIP), parents informés « sauf intérêt contraire de l'enfant » ; avis au procureur par tout fonctionnaire (art. 40 al. 2, **abrogé au 1ᵉʳ janvier 2029** par l'ordonnance 2025-1091) | CASF L. 226-2-1 **[V]** ; C. proc. pén. 40 **[V]** ; L. 226-2-2, L. 226-3, 119 **[NV]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000027572899 ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006574933 | Gel, accès restreint et journalisé, aucun avis automatique à la famille, références légales paramétrables (§14 point 4) | J |
| R-18 | **Outils ENT et vie scolaire paramétrés par défaut pour ne diffuser aucune nouvelle information de 20 h à 7 h en semaine et du vendredi 20 h au lundi 7 h** (messagerie, blog, tchat, notes, devoirs) ; saisie possible, visibilité différée, rappel à l'émetteur (exigence 8) ; la direction ajuste, notamment pour les urgences (exigence 9) | référentiel du numérique responsable pour l'éducation (doctrine technique), §4.4 **[V]** ; circulaire « numérique raisonné », BO n° 28 du 10 juillet 2025 **[NV]** ; C. trav. L. 2242-17 **[NV]** | https://www.education.gouv.fr/sites/default/files/2025-09/doctrine-technique-du-num-rique-pour-l-ducation---r-f-rentiel-du-num-rique-responsable-pour-l-ducation-439554.pdf | Plage calme par défaut, remise à 7 h, dérogation de la direction tracée (R-M20, R-M21, §11.3) ; cohérence à vérifier avec le socle 8 (heures calmes, §31) | — |
| R-19 | Une messagerie fermée relève de la correspondance privée : pas d'obligations de plateforme (DSA, LCEN) tant que les services ne sont pas accessibles au public ; signalement abusif puni | kit juridique §4.3 **[V]** ; LCEN art. 6 (version du 26 août 2026) **[V]** ; texte du DSA **[NV]** | https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000049577522 | Procédure interne de signalement et de modération (§14) ; pas de commentaires publics des publications (Q-M19) ; laïcité et neutralité non recherchées **[NV]** | J |
| R-20 | Règlement IA : annexe III point 3 (admission, évaluation des acquis, niveau, surveillance des examens) ; art. 6 §3 (tâche procédurale étroite hors haut risque, **profilage toujours à haut risque**) ; art. 50 (information, **marquage lisible par machine** des contenus générés sauf aide à l'édition standard) ; calendrier : art. 50 depuis le 2 août 2026, annexe III à partir du 2 décembre 2027 (omnibus IA) ; référentiel numérique responsable §5.3 : pas d'IA générative grand public avec des données personnelles | règlement 2024/1689 **[V]** (service d'assistance de la Commission) | https://ai-act-service-desk.ec.europa.eu/en/ai-act/annex-3 ; …/article-6 ; …/article-50 ; https://digital-strategy.ec.europa.eu/en/news/ai-omnibus-enters-force | Tri, rédaction, traduction : hors annexe III a priori ; mention visible et marque machine (R-M47) ; aucun profilage d'élève ni de famille (§16.2) ; aucune obligation trouvée sur la traduction pour les familles allophones | J |
| R-21 | Accessibilité des services en ligne publics (RGAA), déclaration d'accessibilité | loi 2005-102 art. 47 **[NV]** ; référentiel numérique responsable, exigence 3 **[V]** | — | RGAA dès la conception (§24) | — |
| R-22 | Archives : ensemble des documents « y compris les données » ; archives publiques des établissements publics et des missions de service public ; instruction EPLE 2005 : mots d'excuse et rappels aux familles 1 an, conseils de discipline 10 ans (procès-verbal gardé), chronos de courrier 5 ans, signalements au procureur 1 an | C. patr. L. 211-1 **[V]**, L. 211-4 **[NV]** ; instruction n° 2005-003 **[V]** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032860025 ; https://www.education.gouv.fr/sites/default/files/document/Annexe%201%20sur%20le%20traitement%20des%20archives-245694.pdf | Versement au dossier avant purge ; lot d'élimination visé (§15) | Service d'archives |
| R-23 | Obligation d'utiliser la messagerie académique pour les agents publics | **[NV]** | — | Pour mémoire ; les échanges avec les familles restent dans ScolIA | — |

**Zones floues signalées par la recherche** : (1) durée de conservation des messages (arrêté muet, kit « année ou cycle », instruction de 2005 antérieure aux ENT) — à arbitrer avec le service d'archives ; (2) élève majeur (aucun texte) ; (3) pouvoir de contrôle du chef d'établissement (charte contre « cas prévus par la loi » de 432-9) ; (4) parent sans exercice : latitude de l'établissement ; (5) ordonnance de protection : aucun texte scolaire, décision de la direction sur pièce ; (6) art. 40 abrogé en 2029, destruction des signalements à 1 an contre conservation de la preuve ; (7) 3020 encore cité par des pages anciennes ; (8) date du référentiel (septembre 2026 sur la couverture, dossier 2025-09 ; note qui cite l'ancien L. 2242-8) ; (9) privé sous contrat : assujettissement à l'accessibilité, au code du patrimoine et à 432-9 non établi ; (10) textes non lus : circulaires 94-149, 2006-137, 10 juillet 2025, DSA, RGPD art. 5 et 6.

---
## 6. Frontières

### 6.1 Qui possède quoi

| Sujet | Socle 16 (messagerie) | Socle 17 (annonces) | Socle 8 (notifications) | Autres |
|---|---|---|---|---|
| Conversations, fils, participants, messages, pièces de message, lectures | **possède** | — | notifie « nouveau message » aux **participants nommés** (nature `messagerie.nouveau_message`) | — |
| Qui peut écrire à qui (matrice) | **possède** la règle et la calcule | réutilise pour l'audience des annonces | — | **4** : liens, droits du lien (`messagerie`), interdictions, foyers ; **5** : cours ; **6** : missions, interventions, équipes |
| Destinataires d'un message « aux parents de… » | applique la règle `messagerie` du port `destinataires` (04 §8.12) et crée **un fil par foyer** | — | — | **4** calcule |
| Boîtes de service (vie scolaire, secrétariat…) | **possède** | — | — | **6** fournit les membres (missions, rôles) |
| Carnet de correspondance numérique | **possède** (mots, signature de prise de connaissance) | — | relance la signature | **13** lit les observations qui deviennent disciplinaires (jamais l'inverse sans geste) |
| Cartes d'action (justifier, signer, prendre RDV, payer, document) | affiche et transmet | — | — | **le socle propriétaire** exécute (12, 7, 19, 24/25) avec les droits de la personne |
| Annonce, actualité, lettre d'information : contenu, rubriques, audience décrite, périodes, validation éditoriale | — | **possède** | **mécanique de diffusion** : aperçu, coût, confirmation, exécution, suivi, relance des non-lecteurs (Q-N2) | — |
| Envoi groupé (SMS, e-mail, push en nombre) | **jamais** (au-delà de 30 fils individuels : bascule vers 17 / 8) | décrit le contenu | **possède** | — |
| Réponses entrantes par SMS ou e-mail | **reçoit** dans une boîte (message non vérifié) | — | **reçoit** du fournisseur et transmet | — |
| Rendez-vous, réunions | carte « prendre rendez-vous » | — | — | **19** possède |
| Formulaires, autorisations, sondages | carte « répondre au formulaire » | — | — | **18** possède |
| Information préoccupante, signalement au procureur | parcours depuis un fil (gel, transmission) | — | **aucun avis** famille (R-N58) | **36** possède le suivi |
| Discipline | carte « observation » du carnet ; signalement | — | — | **13** possède les punitions et sanctions |
| Visio depuis une conversation | lien vers la salle | — | — | **33** |
| Assistant conversationnel | s'insère dans la messagerie (même surface) | — | — | **30** et IA de cœur |
| Conservation, purge, export des droits | applique | applique | applique | **27** fixe les durées (DPO) |
| Stockage des pièces | demande un dépôt | idem | — | **noyau / 7** : stockage objet, quarantaine, conversion (D-005), URL pré-signées |
| Annuaire « l'équipe de mon enfant » | utilise | — | — | **6** possède |

### 6.2 Règles de frontière [R]
1. **La messagerie n'envoie rien elle-même** hors de l'ENT : elle déclare ses natures d'avis au socle 8 (§23) et ne dépend d'aucun adaptateur `ent-mail`, `ent-sms`, `ent-push` (garde `cargo metadata`, comme 08 §5.2 point 1).
2. **La messagerie ne calcule pas les liens familiaux** : elle appelle `personnes::destinataires(eleve, NatureEnvoi::Messagerie, date)` et `droits_lien` du socle 4 ; elle ne lit jamais `ent_eleve_lien*` directement (garde de code du socle 4 §5.2 point 4, étendue à la lecture des tables sensibles par un test qui interdit `ent_eleve_interdiction` hors du socle 4).
3. **La messagerie ne décide pas qui enseigne à qui** : port `cours_de(personnel, date)` (socle 5) et `missions_de` / `intervenants` (socle 6).
4. **Une carte d'action n'est qu'un lien typé** : la messagerie n'exécute jamais l'action, elle appelle le service publié du socle propriétaire, qui revérifie les droits.
5. **Une conversation n'est pas une diffusion** : au-delà d'un seuil (S-139, défaut 30 fils), l'écran propose l'annonce (17) ou la diffusion (8) ; la messagerie refuse.
6. **Rien de la messagerie ne sort en clair** : le socle 8 n'emporte qu'une enveloppe (C2 par défaut pour un message, 08 §9.6) ; le contenu se lit dans l'ENT authentifié.

### 6.3 Ports exposés par le socle 16 [R]
| Port | Appelé par | Nature | Règle |
|---|---|---|---|
| `messagerie::ouvrir_fil_systeme(sujet, participants_regle, nature, carte)` | 12 (absence à justifier), 13 (observation), 24 (relance de facture), 7 (document à signer), 3 (dossier d'inscription) | commande, dans la transaction de l'appelant | participants calculés par la messagerie à partir de la règle déclarée ; jamais une liste fournie par l'appelant |
| `messagerie::deposer_message_entrant(canal, personne_presumee, boite, texte)` | 8 (SMS et e-mails entrants) | commande | message marqué `non_verifie`, texte non fiable |
| `messagerie::fils_a_propos(eleve, acteur)` | 2 (chronologie du personnel) | lecture | uniquement les fils que l'acteur peut lire ; titres neutres pour les fils C3 (« échange confidentiel ») ; **jamais** dans une chronologie lue par une famille |
| `messagerie::peut_ecrire(acteur, cible)` | 17, 19, 30 | lecture | la matrice, avec la raison |
| `messagerie::geler(fil, motif)` | 13, 36, 27 | commande | gel non levable par l'auteur |
| `messagerie::export_personne(personne)` | 27 (droits RGPD) | lecture | seuls les fils où la personne participe |

### 6.4 Ports consommés [R]
Noyau : `tx`, `autoriser`, `autoriser_eleve`, `eleves_visibles`, `publier`, `journaliser`, `tracer_acces_sensible`, `exiger_assurance` (N3), stockage objet. Socle 2 : `resume`, `capacite` (majorité), confidentialité renforcée. Socle 4 : `destinataires`, `droits_lien`, `foyer_du_jour`, interdictions (par `destinataires` seulement), préférences (langue), décès. Socle 5 : `cours_de`, `eleves_du_cours(date)`, calendrier, fuseau. Socle 6 : `missions_de`, `intervenants(service, date)`, `equipe_de(classe | eleve)`, absences et suppléances, disponibilité. Socle 7 : dépôt de pièce, carte document. Socle 8 : `aviser` (natures du §23), disponibilité des personnels. IA de cœur : registre des capacités (§16).

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]
| Terme | Définition | Table |
|---|---|---|
| **Fil** (conversation) | Échange entre des **participants nommés**, à propos d'un sujet, dans un établissement ; a une nature, une sensibilité, un état | `ent_msg_fil` |
| **Participant** | Une **personne** (socle 4, avec ou sans compte) ou une **boîte de service**, inscrite au fil avec une qualité (auteur, destinataire, équipe, suppléant, superviseur motivé), des dates d'entrée et de sortie, un droit d'écrire | `ent_msg_participant` |
| **Côté** | Établissement (personnels, boîtes) ou famille / élève. Une réponse du côté établissement est faite **au nom de la boîte** quand le fil est adressé à une boîte, avec le prénom et l'initiale de l'agent (Q-M9) | colonne |
| **Boîte de service** | Destinataire collectif attaché à une fonction (vie scolaire, secrétariat, direction, infirmerie, psychologue, intendance, orientation, harcèlement, CDI, stages) dont les membres viennent du socle 6 ; a des horaires, un message d'accueil, des modèles, un délai de réponse visé | `ent_msg_boite` |
| **Prise en charge** | État de traitement d'un fil dans une boîte : nouveau, pris (par qui), en attente de la famille, traité | `ent_msg_traitement` |
| **Fils jumeaux** | Fils créés ensemble par un même envoi vers plusieurs foyers (parents séparés) : même texte initial, réponses séparées | `ent_msg_fil.lot_id`, `jumeau_de` |
| **Lot d'envoi individuel** | Un geste qui crée N fils distincts (« aux familles de 3 élèves ») avec aperçu ; borné à 30 | `ent_msg_lot` |
| **À propos de** | Rattachement d'un fil à un ou plusieurs élèves (facultatif pour les fils internes) ; règle les droits et la chronologie | `ent_msg_fil_eleve` |
| **Mot du carnet** | Message typé (information, observation, demande de rendez-vous, autorisation ponctuelle, retard, oubli de matériel) avec, si demandé, une **prise de connaissance signée** | `ent_msg_mot` |
| **Carte d'action** | Objet d'un autre socle inséré dans un message (absence à justifier, document, formulaire, rendez-vous, facture), exécuté par son socle | `ent_msg_carte` |
| **Retrait** | Masquage d'un message pour les participants, avec trace (« retiré par son auteur » / « retiré par l'établissement ») ; le contenu reste conservé | colonnes de `ent_msg` |
| **Gel** | Interdiction de toute modification, retrait, purge ou clôture d'un fil (preuve) ; posé par signalement, procédure, réquisition | `ent_msg_gel` |
| **Supervision motivée** | Lecture d'un fil par une personne qui n'en est pas participante (direction, référent), sur motif écrit, journalisée comme accès sensible, visible du DPO ; impossible pour les boîtes C3 | `ent_msg_acces` |
| **Restriction** | Limitation décidée par la direction de ce qu'une personne peut écrire (vers une boîte seulement, lecture seule), motivée, datée, réexaminée | `ent_msg_restriction` |
| **Fil encadré** | Fil entre un adulte et un élève mineur : règles horaires et indicateur de vigilance (§10.2) | `nature = 'eleve'` |
| **Message non vérifié** | Message entrant par SMS ou e-mail, rattaché à une personne présumée | `ent_msg.origine` |

### 7.2 Quatre distinctions qui manquent aujourd'hui [R]
1. **Participant ≠ « lié à l'élève »** : aujourd'hui la lecture d'un fil se déduit du lien avec l'élève (fil « de l'élève ») ; demain elle se déduit **de la participation**. C'est la correction de fond de RP1-11.
2. **Personne ≠ boîte** : une famille écrit « à la vie scolaire », pas « à Mme Durand » ; la continuité ne dépend plus de la présence d'une personne.
3. **Message ≠ diffusion** : un message est une relation ; une annonce ou une campagne est un acte éditorial ; mécaniques distinctes (16 / 17 + 8).
4. **Retirer ≠ supprimer** : on masque, on ne détruit pas ; la destruction n'arrive qu'à la purge de conservation.

---

## 8. La matrice « qui peut écrire à qui »

### 8.1 Principe [R]
Une personne **peut ouvrir** un fil vers une cible si une **raison** existe à la date (lien d'enseignement, lien familial avec droit `messagerie`, mission, boîte publique, équipe), si **aucune interdiction** ne s'y oppose, et si la **règle de l'établissement** (bornée) ne la ferme pas. La raison est affichée (« vous pouvez écrire à Mme B. : professeure de mathématiques de Léo »). **Répondre** dans un fil où l'on est participant est toujours permis, sauf fil clos, gelé, restriction, interdiction ou sortie du fil. Le serveur recalcule la matrice à chaque envoi (l'écran n'est qu'une aide).

### 8.2 Matrice par défaut [R]
« ✓ » ouvert par défaut ; « P » paramétrable par l'établissement (défaut entre parenthèses) ; « — » fermé, non paramétrable.

| De ↓ / vers → | Famille (titulaire avec `messagerie`) | Élève | Professeur | PP | Boîte vie scolaire / CPE | Boîte secrétariat, intendance, CDI, orientation | Direction (boîte) | Infirmerie, psychologue, assistante sociale | AESH | Équipe (fil d'équipe) |
|---|---|---|---|---|---|---|---|---|---|---|
| **Famille** | — (pas d'annuaire des familles ; socle 19 / associations à part) | — (pas d'écriture parent → autre élève) | P (✓ vers les professeurs de **son** enfant) | ✓ | ✓ | ✓ | P (✓) | ✓ (boîte) | P (—) | — |
| **Élève mineur** | — | P (— , Q-M6) | P (✓ vers **ses** professeurs) | ✓ | ✓ | ✓ (CDI, orientation) ; P (secrétariat) | P (—) | ✓ | P (✓ son AESH) | — |
| **Élève majeur** | — | P (—) | ✓ (ses professeurs) | ✓ | ✓ | ✓ | P (✓) | ✓ | P (✓) | — |
| **Professeur** | ✓ familles des élèves de **ses cours** à la date (D-022) | ✓ élèves de ses cours | ✓ collègues de l'établissement | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ AESH de ses élèves | ✓ ses équipes |
| **PP** | ✓ familles de sa classe | ✓ sa classe | ✓ | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ équipe de la classe |
| **CPE, vie scolaire** | ✓ toutes les familles de son périmètre | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Secrétariat, intendance** | ✓ (par nature de fil : facture → payeurs) | P (✓ élèves majeurs) | ✓ | ✓ | ✓ | ✓ | ✓ | — (sauf boîte) | — | — |
| **Direction** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (boîte) | ✓ | ✓ |
| **Infirmerie, psychologue, AS** | ✓ (familles des élèves suivis) | ✓ | ✓ (sans détail C3) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| **AESH** | P (— ouvrir ; ✓ répondre dans un fil où il est ajouté) | ✓ élève accompagné | ✓ équipe de l'élève | ✓ | ✓ | — | — | — | ✓ | ✓ équipe de l'élève |
| **Accueil** | — (sauf message téléphonique, qui crée un fil interne) | — | ✓ (message de liaison) | ✓ | ✓ | ✓ | ✓ | ✓ | — | — |

Bornes non paramétrables [R] : jamais d'écriture vers une personne sous interdiction de communication pour l'élève concerné ; jamais un fil famille ↔ famille ; jamais un professeur vers une famille hors de ses cours à la date (sauf réponse pendant 15 jours dans un fil existant, CM-35) ; jamais une boîte C3 lue par la supervision ordinaire ; jamais un élève mineur vers un élève **d'un autre établissement**.

### 8.3 Pourquoi pas une matrice libre « par rôle » [R, D-023]
L'existant Next.js (`famille_messaging_settings.roles_can_initiate`, liste de rôles en JSON) et les logiciels du marché [H] offrent une matrice libre par rôle : elle produit des erreurs (un rôle « AED » oublié, une famille qui écrit à tous les professeurs de l'établissement). ScolIA calcule la matrice **à partir des faits** et ne laisse à l'établissement que **sept réglages bornés** (S-132) : familles → professeurs (oui / non), familles → direction (oui / non), élèves → professeurs (oui / non), élèves → élèves (non / délégués seulement / classe), AESH → familles (non / répondre / ouvrir), secrétariat → élèves majeurs, pièces jointes des familles (oui / non).

---

## 9. Parents séparés, interdictions, « ne pas solliciter »

### 9.1 Règles [R]
1. **Un fil famille a des participants nommés.** Un message de l'établissement « aux parents » crée **un fil par foyer** (socle 4 §6.2, sens « foyer de résidence ») parmi les liens qui ont `messagerie` ; deux parents d'un même foyer partagent un fil, sauf demande (MSG-118).
2. **Un parent ne voit que les fils où il participe.** Ni liste, ni compteur, ni sujet, ni recherche, ni notification, ni pièce, ni accusé, ni export ne révèlent l'existence d'un fil d'un autre foyer (test unique au niveau du contrôle d'accès, §28).
3. **Aucune donnée d'un foyer dans un fil de l'autre** : la messagerie ne compose jamais de texte avec des coordonnées ; les aperçus montrent des noms et des foyers numérotés (« foyer 1, foyer 2 »), jamais d'adresse.
4. **Un fil ouvert par l'établissement n'est plus « visible de chaque responsable de l'élève »** (règle de RP1-11, **remplacée**) : il a ses participants, choisis par la règle de la nature (absence : titulaires avec `voir_vie_scolaire` ; facture : payeurs ; carnet : titulaires) et, pour deux foyers, il est dédoublé.
5. **Interdiction de communication** (socle 4 `ent_eleve_interdiction`, objet `communication`) : la personne est exclue de **tout** fil à propos de l'élève visé, retirée des fils existants dès la pose (sans message), exclue des annonces ; l'exclusion est relue **à chaque envoi et à chaque lecture** ; l'aperçu du personnel non habilité montre une exclusion agrégée (socle 8 R-N51).
6. **« Ne pas solliciter »** : la messagerie n'ouvre aucun fil vers la personne, sauf information due décidée par la direction (socle 8 Q-N9) ; elle **répond** dans un fil ouvert par la personne.
7. **Lien suspendu** : participation suspendue (ni lecture ni écriture), sans effacement.
8. **Adresse protégée** : aucun effet propre (la messagerie ne manipule pas d'adresse) ; vérification que la signature et les modèles n'insèrent jamais de coordonnées d'une personne.
9. **Les informations dues vont aux deux** : un fil de l'établissement d'une nature « information due » (convocation, absence) est dédoublé vers chaque titulaire qui a le droit ; la phrase « les deux responsables ont été informés » s'affiche côté personnel.

### 9.2 Reprise des fils existants (RP1-11) [R]
Fils `prive_user_id` → participants (ce parent + l'établissement) ; fils `foyer_id` (factures) → participants payeurs actifs du foyer ; fils ouverts par l'établissement sans `prive_user_id` ni `foyer_id` → participants = **les responsables qui ont réellement lu ou écrit** dans le fil, plus ceux qui avaient le droit à la date de création **et** appartiennent au même foyer que le premier lecteur ; en cas de doute (deux foyers, aucun lecteur) : fil **dédoublé** à la reprise, rapport à la direction (MIG-3, §26).

---

## 10. Élèves

### 10.1 Élève mineur [R]
- Écrit aux destinataires de la matrice ; ne voit pas les fils de ses parents ; **ses parents ne lisent pas sa messagerie** (ni par l'écran famille, ni par l'export d'un droit d'accès d'un parent) ; Q-M5.
- Ses fils avec l'infirmerie, le psychologue, l'assistante sociale sont C3 et invisibles de toute supervision ordinaire.
- Aucun fil n'est ouvert vers lui par un adulte entre 20 h et 7 h sans différer la remise (R-M21) ; les avis ne le dérangent pas en cours (socle 8 R-N66, C. éduc. L511-5, VERIF R7).
- Il peut **signaler** un message, **sortir** d'un fil d'équipe ou de classe (sauf fil de la vie scolaire), **rendre muet** un fil.

### 10.2 Fils encadrés adulte–élève mineur [R]
Principe : un adulte de l'établissement peut échanger avec un élève mineur, mais **aucun échange adulte–mineur n'est hors de portée de l'institution**. Concrètement :
1. Mention permanente dans le fil : « Échange dans le cadre de l'établissement ; en cas de besoin, la direction peut le consulter. »
2. Supervision motivée possible (direction, référent désigné), journalisée, sauf boîtes C3 (infirmerie, psychologue, AS), qui ont leur propre responsabilité professionnelle. Fondement : les messages de l'ENT relèvent de la **correspondance privée** ; le pouvoir de contrôle du chef d'établissement doit être **prévu par la charte**, exercé par des personnes **désignées**, journalisé, en respectant la « vie privée résiduelle » (kit juridique du SDET §4.4–4.7, annexe opérationnelle §3.8.3 [V]) ; ce fondement est **fragile** face à C. pén. 432-9 (« hors les cas prévus par la loi ») : avis juridique exigé avant la mise en service (Q-M7).
3. **Indicateur de vigilance** déterministe, sans lecture du contenu : fréquence élevée, messages hors horaires, aucun lien d'enseignement ni de mission avec l'élève, fil ouvert par l'adulte ; visible du seul référent désigné ; jamais de score, jamais d'IA sur le contenu (Q-M7).
4. Pas de pièces jointes « image » d'un adulte vers un mineur hors cours et hors boîte de service (P, défaut : autorisé seulement depuis un cours ou une boîte) [R, S-141].

### 10.3 Élève majeur [R]
- Destinataire principal de ce qui le concerne ; ses parents participent aux fils « à propos de lui » selon les droits que le socle 4 conserve après la majorité (Q-A7 du socle 4, Q-RP1-4) ; la messagerie ne décide rien elle-même.
- Il voit, dans ses réglages, « vos parents reçoivent encore : … » (lecture du socle 4), et peut demander un changement au secrétariat.

### 10.4 Entre élèves [R, Q-M6]
**Fermé par défaut.** Raisons : le cyberharcèlement passe d'abord par les messageries, et l'établissement a une obligation de prévention (C. éduc. L. 111-6 [V]) ; ScolIA n'a ni modération humaine permanente ni intérêt pédagogique suffisant à offrir une messagerie libre de plus. Ouvertures possibles, bornées : **fil des délégués** (élèves élus + CPE ou PP), **fil de classe** animé par un adulte (lecture et réponse, ouverture par l'adulte seulement), **groupe de travail** au socle 32. Jamais d'échange privé élève–élève.

---

## 11. Personnels

### 11.1 Boîtes de service [R]
- Créées par l'établissement à partir d'un **catalogue fourni** (vie scolaire, secrétariat, direction, infirmerie, psychologue, assistante sociale, intendance, orientation, CDI, stages, harcèlement) ; nom libre ; une boîte personnalisée possible (D-022 : liberté des données).
- Membres = personnels porteurs d'une **mission** (socle 6) ou d'un rôle qui donne `messagerie.boite.traiter` sur cette boîte ; jamais une liste figée à la main.
- Horaires, message d'accueil, délai de réponse visé (défaut 2 jours ouvrés, affiché aux familles), réponse automatique hors horaires, astreinte (facultative) pour les messages marqués urgents.
- Traitement : nouveau → pris par (une personne) → en attente de la famille → traité ; réouverture automatique quand la famille répond ; échéance ; étiquettes (liste courte) ; transfert vers une autre boîte avec note interne ; **notes internes** invisibles de la famille.
- Réponse **au nom de la boîte** avec le prénom et l'initiale de l'agent (« Vie scolaire — Paul D. ») ; la famille ne voit jamais la liste des membres.
- C3 : boîtes infirmerie, psychologue, assistante sociale : membres restreints (la professionnelle et sa remplaçante désignée), aucune supervision ordinaire, aucune recherche transversale, conservation propre (§15).

### 11.2 Absence, suppléance, départ, délégation [R]
- Absence d'un personnel (socle 15) : réponse d'absence automatique **sans motif**, redirection proposée (vers le suppléant du service, le PP, la vie scolaire) ; aucune notification pendant l'absence.
- Suppléance (socle 6 intervention `suppleant`) : le suppléant devient participant des fils **du service** (familles et élèves du cours), à partir de la date ; le titulaire garde la lecture et redevient seul à son retour ; les fils personnels du titulaire avec des collègues ne sont pas transmis.
- Départ : fils famille et élève → successeur du service ou boîte du PP ; fils internes clos ; message neutre aux familles (« Mme B. a quitté l'établissement ; M. C. reprend la conversation ») ; les messages restent signés.
- Délégation (intérim) : droits de supervision et de boîte pendant la délégation seulement.

### 11.3 Déconnexion [R sur une base V]
- **Règle nationale par défaut** [V, §5 R-18] : les outils qui relient établissement, familles et élèves (ENT, vie scolaire) sont paramétrés **par défaut** pour ne diffuser **aucune nouvelle information de 20 h à 7 h en semaine et du vendredi 20 h au lundi 7 h** ; la saisie reste possible, la visibilité est différée à la fin de la plage, un rappel est affiché à l'émetteur (exigence 8) ; la direction peut ajuster ces règles, notamment pour les urgences (exigence 9). ScolIA applique exactement ce défaut : **tout message d'un personnel vers une famille ou un élève** écrit dans la plage calme est **remis à 7 h** ; l'auteur le voit avant d'envoyer ; seule la direction (action `messagerie.urgence_hors_plage`, tracée) remet immédiatement.
- Dans l'autre sens, un message de famille ou d'élève est enregistré à toute heure ; les personnels ne sont pas avertis hors de leur disponibilité (socle 8 §16, défaut 7 h 30–19 h 00) ; les boîtes C3 et l'astreinte suivent leurs propres horaires.
- Le différé est un **paramétrage connu et annoncé** (charte, mention dans le fil), jamais une rétention silencieuse : retarder une correspondance de mauvaise foi est une infraction (C. pén. 226-15, 432-9 [V], §5 R-10).
- La famille voit, avant d'envoyer : « Mme B. lira votre message à partir de lundi 7 h 30 » ; « La vie scolaire répond sous 2 jours ouvrés. »
- Aucun indicateur de présence (« en ligne ») ni de « vu à 23 h 14 » pour les personnels côté familles (S-137).

### 11.4 Fils d'équipe [R]
Équipes tenues par le socle 6 (équipe pédagogique d'une classe, d'un élève accompagné, d'un niveau, d'une discipline, d'une instance) : le fil d'équipe est **créé et tenu à jour automatiquement** (arrivées, départs, suppléances) ; les familles n'y sont jamais ; les AESH de l'élève y sont ; un fil d'équipe d'élève à besoins particuliers est C2.

---

## 12. Pièces jointes [R]
1. **Stockage objet** (pas de `BYTEA`), dépôt en **quarantaine**, analyse antivirus, conversion (photos HEIC/JPEG → PDF si demandé, désarmement des PDF, D-005) par l'adaptateur du socle 7 ; jamais servie avant l'analyse.
2. Téléchargement par **URL pré-signée courte** (60 s) émise après contrôle des droits **du lecteur** sur le fil (CLAUDE.md) ; jamais d'URL durable dans un e-mail.
3. Formats admis : PDF, images, documents bureautiques ouverts ; refus des exécutables, archives chiffrées, PDF protégés ; taille 20 Mo par pièce, 5 pièces par message (S-138).
4. Côté personnels : un document du coffre (bulletin, certificat, compte rendu) se joint par **carte document** (droits relus à la lecture), pas en fichier.
5. Une pièce d'un message retiré reste conservée ; elle n'est plus servie aux participants.
6. Justificatif d'absence reçu en pièce : carte « joindre à l'absence du … » (socle 12) ; la pièce rejoint le justificatif sans copie manuelle.
7. Image d'un adulte vers un élève mineur : règle §10.2 point 4.

---

## 13. Lecture, accusés, relances [R]
- Par participant : « lu jusqu'au message n » ; pour une boîte, la lecture est celle **du premier membre** qui ouvre.
- Ce que voit le personnel : lu / non lu par chaque participant famille ou élève, et depuis quand.
- Ce que voit la famille : « lu par l'établissement » (boîte) ou « lu par Mme B. » (fil individuel), **sans l'heure** (Q-M8) ; « pris en charge » pour une boîte.
- **Prise de connaissance signée** (carnet, information importante) : geste explicite « j'ai pris connaissance », distinct de l'ouverture ; preuve opposable légère (horodatage, personne, version du texte) ; relance par le socle 8 (une relance, puis tâche au personnel).
- Pas d'accusé imposé à la famille pour un message ordinaire ; pas de pixel ; « lu partout » (08 §0 point 6).

---

## 14. Modération, signalement, harcèlement, protection de l'enfance, urgence [R]
1. **Signaler** un message (tout participant) : catégories courtes (insulte, menace, harcèlement, contenu inapproprié, autre) ; le message est **gelé** et masqué pour celui qui signale s'il le demande ; la direction (ou le référent harcèlement si la catégorie le désigne) reçoit une tâche ; l'auteur n'est pas prévenu automatiquement.
2. **Modération** par la direction : retrait d'un message (motif), clôture d'un fil, **restriction** d'une personne (vers une boîte seulement, lecture seule), datée et réexaminée (rappel à 30 jours) ; la personne restreinte est informée par un message neutre rédigé par la direction ; l'information due continue.
3. **Harcèlement entre élèves** : obligation de prévenir, détecter et orienter (C. éduc. L. 111-6, loi 2022-299 [V]) ; délit de harcèlement scolaire (C. pén. 222-33-2-3 [V]) ; parcours vers le référent harcèlement (programme pHARe [NV]) ; conservation de la preuve (gel) ; rappel du **3018** (gratuit, anonyme, 7 j/7 de 9 h à 23 h [V]) à l'élève qui signale ; jamais de confrontation des familles dans un fil commun.
4. **Révélation d'une maltraitance** dans un message : gel dès que le personnel le marque ; accès limité à la direction et aux personnels sociaux et de santé désignés, chaque accès journalisé ; **aucun avis automatique** à la famille (socle 8 R-N58) : l'information préalable des parents est une décision humaine, écartée si l'intérêt de l'enfant s'y oppose (CASF L. 226-2-1 [V]) ; parcours « information préoccupante » (CRIP) ou avis au procureur (C. proc. pén. 40 al. 2 [V], **abrogé au 1ᵉʳ janvier 2029** par l'ordonnance 2025-1091 : références légales **paramétrables**, jamais en dur) ; le **119** est affiché dans l'espace élève [NV].
5. **Urgence** : la messagerie n'est **pas** un canal d'urgence ; bandeau permanent avec les numéros (établissement, 15, 17, 112, 119, 3018) ; tri déterministe des mots d'alerte en tête de boîte ; astreinte facultative.
6. **Aucun filtrage automatique bloquant du contenu** (risque d'erreur, de censure, de faux sentiment de sécurité) ; un avertissement doux avant envoi pour les mots injurieux évidents côté élève (P, S-142).

---

## 15. Conservation, archivage, droits des personnes [R sur une base V ; durées : DPO et service d'archives]
1. **Aucune suppression physique par un utilisateur.** Retrait = masquage ; la destruction n'arrive qu'à la purge. Supprimer une correspondance est en soi un acte sensible (C. pén. 226-15 [V]).
2. **Ce que disent les textes** [V, §5] : l'arrêté du 30 novembre 2006 modifié (ENT) ne fixe **aucune durée propre à la messagerie** et impose la **suppression des données dans les 3 mois** après la perte du droit au compte (art. 6) ; le kit juridique du SDET (2025, mis à jour en février 2026) : données du compte « pour la durée de l'année scolaire ou, au besoin, pour la durée du cycle », traces de connexion **12 mois au plus** ; l'instruction d'archives des EPLE de 2005 : mots d'excuse et rappels aux familles **1 an** puis destruction, correspondance de conseil de discipline 10 ans (on garde le procès-verbal), signalements au procureur 1 an ; dans un EPLE public, les échanges avec les familles sont des **archives publiques** (C. patr. L. 211-1 [V], L. 211-4 [NV]) : toute élimination suppose le visa du service d'archives (VERIF R40).
3. **Durées proposées** (hypothèses de travail, Q-M10) : un fil famille ou élève vit pendant la scolarité ; à la sortie de l'élève (ou à la fin du lien), il est **clos**, l'échange qui a une valeur administrative est **versé au dossier** (socle 7, MSG-149, proposé automatiquement pour les fils signalés, disciplinaires, d'orientation), puis le fil est **purgé 3 mois après** la perte du droit au compte (arrêté de 2006, art. 6) **par lot visé** du service d'archives pour le public ; mots du carnet : 1 an après l'année (comme les mots d'excuse) ; fils internes des personnels : fin de l'année scolaire suivante ; boîtes C3 : règle de la profession (socles 34, 36) ; fils gelés : jusqu'à la levée du gel ; journal des supervisions et des accès : 12 mois (kit SDET, VERIF R43).
4. **Archives publiques** : un échange qui fonde une décision (discipline, orientation, information préoccupante) est **déposé au dossier** (socle 7) et suit la durée du dossier ; la purge du reste se fait par **lot d'élimination** soumis au visa (public) ; pour le privé sous contrat, statut d'archives publiques à confirmer (VERIF R39).
5. **Droit d'accès** (RGPD art. 15 [V]) : export par personne des seuls fils où elle participe (PDF et JSON), **sans porter atteinte aux droits d'autrui** (§4 : les données de tiers — autre parent, autre élève — sont expurgées) ; délai d'un mois ; parcours au socle 27. **Effacement** (art. 17 §3 [V]) : refusé quand la conservation sert la mission d'intérêt public, l'archivage ou la défense en justice.
6. **Gel** : prioritaire sur la purge ; levée par la direction seulement, motivée. Note de tension : l'instruction de 2005 prévoit la destruction des signalements au procureur à 1 an ; un fil gelé pour une procédure en cours reste gelé jusqu'à la clôture de la procédure (décision de la direction, avis du service d'archives).
7. **Départ de l'élève** : ses fils et ceux de ses parents sont clos et entrent en conservation ; consultables par la direction avec motif jusqu'à la purge.

---

## 16. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §8.2)

### 16.1 Ce que l'IA fait [R]
| Capacité | Niveau (`IA_COEUR.md` §6.2) | Règle |
|---|---|---|
| **Aide à la rédaction** d'un message ou d'une réponse (ton, clarté, longueur, formule de politesse) | brouillon **N3** si envoi (passkey récente, aperçu complet : destinataires, texte final, pièces) | destinataires **imposés par le fil** (jamais choisis par le modèle) ; le texte n'utilise que le fil et ce que la personne voit ; mention « rédigé avec l'aide de l'assistant » côté auteur, et côté destinataire si l'établissement le choisit (Q-M12) |
| **Résumé d'un fil** | N0 | dans les droits du lecteur ; jamais d'un fil C3 hors de sa boîte ; sources (messages) citées |
| **Tri d'une boîte** (proposer une boîte, une étiquette, un niveau d'urgence, « justificatif d'absence probable ») | N1 (étiquette proposée, posée d'un geste) | **aucune décision** : ni clôture, ni réponse, ni transfert sans geste ; tri déterministe d'abord (mots d'alerte), modèle de langage ensuite (Q-M13) |
| **Traduction** pour les familles allophones (lecture d'un message entrant ; préparation d'une réponse dans la langue de la famille) | N0 à la lecture ; N3 pour l'envoi | original toujours visible ; mention « traduction automatique » dans le message envoyé, avec le texte français joint ; pas de traduction automatique d'un gabarit système (socle 8 §9.7) ; avertissement sur les sujets de discipline et de santé |
| **Préparer une réponse type** à partir des modèles de la boîte | N3 si envoi | la personne choisit le modèle ; l'IA remplit |
| **Extraire une action** d'un message (« Léo sera absent demain matin pour un rendez-vous ») | N2 (carte proposée : déclarer l'absence) | exécution par le socle 12 avec les droits de la personne ; le message reste un message non fiable |
| **Repérer une injection** dans un message (instruction adressée à l'assistant) | — | alerte affichée sur tout brouillon issu du fil (`IA_COEUR.md` T2) |

### 16.2 Ce que l'IA ne fait pas [R]
Jamais d'envoi, d'ajout de destinataire, de clôture, de retrait, de restriction, de supervision, de lecture d'un fil que la personne ne peut pas lire ; jamais de « répondre à tout » (un brouillon par fil) ; jamais d'analyse du contenu des fils encadrés adulte–élève pour l'indicateur de vigilance ; jamais de lecture des boîtes C3 hors de la professionnelle ; jamais de profilage des familles (« famille difficile ») ; jamais d'apprentissage sur les messages (fournisseur sans rétention, `IA_COEUR.md`) ; fonctionnement complet sans IA.

### 16.3 Capacités déclarées (registre `ent-ia`) [R]
Lectures : `messagerie.fil.lire` (participants et supervision motivée déjà ouverte), `messagerie.boite.lister` (membres). Commandes : `messagerie.brouillon.preparer` (N1, non envoyé), `messagerie.message.envoyer` (N3, destinataires du fil), `messagerie.etiquette.poser` (N1), `messagerie.carte.proposer` (N2, exécutée par le socle propriétaire). **Fermées à l'IA** : `messagerie.fil.ouvrir_vers` (choix des destinataires), `messagerie.lot.envoyer`, `messagerie.moderer`, `messagerie.restreindre`, `messagerie.superviser`, `messagerie.geler`, `messagerie.exporter`, tout le paramétrage.

### 16.4 Cadre [§5 R-20]
Règlement européen sur l'IA [V] : aide à la rédaction et traduction relèvent des obligations de **transparence** de l'art. 50 (applicable depuis le 2 août 2026 : information, marquage lisible par machine des contenus générés sauf simple aide à l'édition) ; le tri d'une boîte n'évalue pas un élève et ne décide rien (hors annexe III point 3, applicable à partir du 2 décembre 2027) ; **tout profilage** ferait basculer en haut risque (art. 6 §3), d'où l'interdiction du §16.2 ; qualification finale par l'étude juridique déjà demandée au socle 21 (Q-EV8). Mistral prioritaire, sous garanties (CLAUDE.md) ; aucune clé côté client.

---

## 17. Socle 17 — Informations, annonces et campagnes (esquisse de conception)

Le socle 17 aura sa conception propre ; ce qui suit fixe la **frontière** et les choix qui conditionnent la messagerie, pour que le lot M15 et le lot N9 du socle 8 puissent avancer.

### 17.1 Ce qu'est une publication [R]
Un **acte éditorial** de l'établissement : annonce (courte, datée, éventuellement épinglée), actualité (vie de l'établissement, image), lettre d'information (plusieurs rubriques), **campagne** (information ciblée avec réponse attendue : coupon-réponse, document à rendre, renvoi vers un formulaire du socle 18). Elle a un **auteur**, éventuellement un **valideur** (circuit à deux personnes pour les audiences larges), une **audience décrite** (et non une liste), une période d'affichage, une rubrique, une langue (traductions relues), des pièces (carte document), un **suivi de lecture**.

### 17.2 Audiences [R]
Décrites par des critères (établissement, site, niveau, classe, cours, groupe, régime, transport, inscrits de la rentrée, payeurs, personnels par mission, élèves, familles, les deux) ; **résolues par le socle 4 et le socle 8 sous les droits de l'auteur** (un professeur ne publie qu'à ses cours, D-022) ; « ne pas solliciter » et interdictions appliqués à chaque remise ; aperçu des destinataires et des exclus (agrégés pour un non habilité, R-N51).

### 17.3 Diffusion et suivi [R]
Publier ≠ notifier : une publication s'affiche dans le fil d'actualité des personnes de l'audience ; la **notification** passe par une diffusion du socle 8 (catégorie « Vie de l'établissement », canaux, coût SMS, confirmation N3, quatre yeux au-delà du seuil, délai d'annulation). Suivi : lu / non lu par personne, par classe ; **relance des seuls non-lecteurs** (nouvel aperçu) ; accusé « j'ai pris connaissance » si demandé. Les réponses à une publication **ne sont pas des commentaires publics** : un bouton « poser une question » ouvre un fil individuel avec l'auteur ou sa boîte.

### 17.4 Ce qui reste hors du socle 17 [R]
Formulaires et autorisations (18), réunions et inscriptions à des créneaux (19), urgence (8 §14), site public (nouveau socle N3 ou 17 plus tard, `MATRICE_CHARLEMAGNE.md` T-17).

### 17.5 Existant à reprendre [F → §2]
Annonces (086) et campagnes (191) : contenu repris en publications ; empreinte d'aperçu et confirmation de `campagnes.rs` conservées et déplacées dans la mécanique du socle 8 (08 §13).

---

## 18. Carnet de correspondance numérique [R]
- **Mots** typés : information, observation (comportement, travail), oubli de matériel, retard, demande de rendez-vous (vers le socle 19), autorisation ponctuelle (sortie anticipée : carte du socle 14), mot de la famille (absence, demande).
- Un mot est un message d'un fil **carnet** de l'élève (un par foyer si séparés) ; il peut exiger une **prise de connaissance signée** par un titulaire (CM-63).
- Une observation n'est **jamais** une sanction : la discipline (socle 13) peut **citer** un mot, jamais l'inverse ; l'observation reste dans le carnet, effacée à la fin de l'année suivante (durée à confirmer, §15).
- L'élève voit les mots qui le concernent, sauf ceux que le personnel adresse « aux responsables seulement » (P).
- Vue du carnet par élève (personnel : PP, CPE, professeurs de l'élève ; famille : ses mots) ; statistiques simples (nombre d'oublis) sans classement.
- Remplace l'existant `020_vs_carnet` (§2) par reprise.

---

## 19. Actions métier (MSG-100 et suivantes ; ANN-100 et suivantes pour le socle 17)

Abréviations des acteurs : PAR (responsable), ELV (élève mineur), ELM (élève majeur), PRF (professeur), PP, CPE, VS (vie scolaire, AED), DIR (direction), ADF (secrétariat), INT (intendance), INF (infirmière), PSY, AS (assistante sociale), AESH, ACC (accueil), SYS (système), IA (assistant, toujours au nom d'une personne). ★ = confirmation renforcée (N3 : passkey récente). Les droits sont ceux du §21.

### 19.1 Écrire et lire (famille, élève)
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-100 | Voir **mes conversations** (tous enfants, tous établissements) | PAR, ELV, ELM · participation | filtres par enfant, par établissement, non lus, en attente de réponse ; seuls les fils où la personne participe | — |
| MSG-101 | **Écrire à un service** (« la vie scolaire de Léa ») | PAR, ELV, ELM · `messagerie.ecrire` | choix guidé par motif (absence, question de cours, cantine, document, autre) → boîte ; horaires et délai affichés avant l'envoi | `messagerie.fil_ouvert` |
| MSG-102 | Écrire à un **professeur** de son enfant | PAR · matrice | liste « l'équipe de mon enfant » (socle 6) ; matière et rôle affichés, jamais de coordonnées | `messagerie.fil_ouvert` |
| MSG-103 | Écrire au **PP** | PAR, ELV | — | idem |
| MSG-104 | Répondre dans un fil | participants | fil ouvert, non gelé contre l'écriture, pas de restriction | `messagerie.message_envoye` |
| MSG-105 | Joindre une pièce | participants · réglage | §12 ; quarantaine ; refus des formats interdits | `messagerie.piece_deposee` |
| MSG-106 | Voir que le message a été **lu / pris en charge** | PAR, ELV | §13 ; sans l'heure | — |
| MSG-107 | Retirer son message (10 min) | auteur | §15 ; trace « retiré par son auteur » | `messagerie.message_retire` |
| MSG-108 | Corriger son message (15 min, non lu) | auteur | versionné, « modifié » | `messagerie.message_corrige` |
| MSG-109 | Archiver / désarchiver un fil pour soi | participant | état personnel, sans effet pour les autres | — |
| MSG-110 | Rendre muet un fil | participant | sauf fils de nature « information due » (plancher) | — |
| MSG-111 | Rechercher dans ses fils | participant | texte, enfant, période ; jamais au-delà de ses fils | — |
| MSG-112 | Signaler un message | participant | §14 | `messagerie.signalement_cree` |
| MSG-113 | Voir « pourquoi je peux écrire à cette personne » | tous | raison de la matrice | — |
| MSG-114 | Voir « quand sera-t-il lu ? » | PAR, ELV | disponibilité, horaires de la boîte | — |
| MSG-115 | Lire la **traduction** d'un message | PAR, ELV | §16 ; original visible | — |
| MSG-116 | Exécuter une **carte d'action** reçue (justifier, signer, répondre, payer, prendre RDV, ouvrir un document) | PAR, ELV, ELM · droits du socle propriétaire | le socle propriétaire revérifie | événement du socle propriétaire |
| MSG-117 | Signer la **prise de connaissance** d'un mot | PAR (titulaire), ELM | §13, §18 | `messagerie.mot_signe` |
| MSG-118 | Demander à **avoir son propre fil** (sortir d'un fil commun de foyer) | PAR | le fil se dédouble pour l'avenir ; le passé reste lisible des deux | `messagerie.fil_dedouble` |
| MSG-119 | Exporter ses échanges (droit d'accès) | PAR, ELM, ELV (avec ses responsables selon l'âge, Q-M5) | §15 ; via socle 27 | `messagerie.export_realise` |
| MSG-120 | Voir les mots du carnet de son enfant | PAR (lien avec `voir_vie_scolaire`) | uniquement les mots adressés à son foyer | — |
| MSG-121 | Écrire un **mot au carnet** (retard prévu, oubli, demande) | PAR · titulaire | type choisi ; vers la vie scolaire ou le PP | `messagerie.mot_cree` |
| MSG-122 | Choisir sa langue de lecture | PAR, ELV | préférence du socle 4 | — |
| MSG-123 | Voir l'état « en attente de votre réponse » | PAR | la boîte a posé l'état | — |
| MSG-124 | Clore un fil de son côté (« merci, c'est réglé ») | PAR, ELV | passe la prise en charge en « traité » si la boîte n'a rien d'autre | `messagerie.fil_clos` |
| MSG-125 | Rouvrir par une nouvelle question qui cite un fil clos | PAR, ELV | nouveau fil lié | `messagerie.fil_ouvert` |
| MSG-126 | Sortir d'un fil de classe ou de délégués | ELV | sauf fil vie scolaire | — |
| MSG-127 | Voir les **numéros d'urgence** et la mention « la messagerie n'est pas un canal d'urgence » | tous | bandeau permanent | — |
| MSG-128 | Écrire de façon **discrète** à la CPE / infirmerie / psychologue (« je veux parler à quelqu'un ») | ELV | parcours court, boîte choisie par l'élève ; aucune trace dans l'espace famille | `messagerie.fil_ouvert` (sans sujet dans l'événement) |
| MSG-129 | Être informé des règles de la messagerie (charte courte, première connexion) | tous | version acceptée tracée | — |

### 19.2 Écrire (personnels)
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-130 | Écrire **aux responsables d'un élève** | PRF, PP, CPE, DIR, ADF · matrice | un fil par foyer (R-M3) ; aperçu « 2 conversations » ; exclusions agrégées | `messagerie.fil_ouvert` × n |
| MSG-131 | Écrire **à un élève** | PRF, PP, CPE, DIR · matrice | fil encadré si mineur ; envoi différé la nuit | `messagerie.fil_ouvert` |
| MSG-132 | Écrire aux responsables **et** à l'élève | idem | fils séparés famille / élève par défaut ; fil commun si l'auteur le choisit et si l'élève est majeur ou si la nature le permet (Q-M5) | idem |
| MSG-133 | **Lot d'envoi individuel** (« aux familles de ces 3 élèves ») ★ si > 5 fils | PRF, PP, CPE · matrice | ≤ 30 fils (S-139) ; aperçu nominatif ; même texte, variables (prénom de l'élève) ; un fil par foyer | `messagerie.lot_envoye` |
| MSG-134 | Écrire à un **collègue** ou à une **boîte** | personnels | fil interne ; « à propos de » un élève facultatif (droits du destinataire sur l'élève vérifiés) | `messagerie.fil_ouvert` |
| MSG-135 | Écrire à l'**équipe** d'une classe, d'un élève, d'une discipline | membres · équipe socle 6 | fil d'équipe tenu à jour | `messagerie.message_envoye` |
| MSG-136 | Utiliser un **modèle** de message | personnels | modèles personnels, de boîte, d'établissement (≤ 50, S-59 du socle 8) ; variables documentées | — |
| MSG-137 | Programmer un envoi (date, heure) | personnels | sauf vers un mineur la nuit ; annulable jusqu'à l'envoi | `messagerie.envoi_programme` |
| MSG-138 | Remettre immédiatement un message différé par la plage calme (urgence) ★ | DIR · `messagerie.urgence_hors_plage` | motif ; tracé ; exigence 9 du référentiel (§5 R-18) | `messagerie.derogation_plage` |
| MSG-139 | **Scinder** un fil (fratrie, sujet mêlé) | CPE, DIR | crée deux fils, garde la trace | `messagerie.fil_scinde` |
| MSG-140 | Ajouter un participant à un fil (collègue, suppléant, AESH) | participant personnel | matrice ; la famille voit « M. C. a rejoint la conversation » ; jamais l'ajout d'un autre foyer à un fil famille | `messagerie.participant_ajoute` |
| MSG-141 | Retirer un participant personnel | auteur, DIR | trace ; il ne lit plus la suite | `messagerie.participant_retire` |
| MSG-142 | Insérer une **carte d'action** (absence à justifier, document, formulaire, rendez-vous, facture) | personnels · droit du socle propriétaire | le socle propriétaire crée l'objet ; la carte affiche son état en direct | `messagerie.carte_inseree` |
| MSG-143 | Écrire une **observation** au carnet | PRF, CPE, VS | catégories courtes ; signature demandée ou non ; jamais une sanction | `messagerie.mot_cree` |
| MSG-144 | Demander une **prise de connaissance signée** | personnels | §13 | — |
| MSG-145 | Rédiger avec l'**assistant** | personnels · `ia.utiliser` | §16 | journal IA |
| MSG-146 | Traduire une réponse dans la langue de la famille | personnels | relue, confirmée, marquée | — |
| MSG-147 | Clore un fil | auteur, boîte, DIR | message de clôture facultatif ; réouverture par nouvelle question | `messagerie.fil_clos` |
| MSG-148 | Marquer un fil « sensible » (C3) | CPE, DIR, INF, PSY, AS | lecture restreinte, retiré de la recherche transversale, titre neutre dans la chronologie | `messagerie.sensibilite_changee` |
| MSG-149 | Déposer un message ou un fil **au dossier** de l'élève (socle 7) | CPE, DIR, ADF | copie figée avec empreinte ; suit la conservation du dossier | `documents.depose` |
| MSG-150 | Voir l'historique des échanges avec une famille (tous ses fils avec l'établissement) | CPE, DIR · supervision ou participation | seulement les fils lisibles ; vue par foyer, jamais fusion de deux foyers | — |
| MSG-151 | Répondre depuis le **mobile** (appli unique, D-003) | personnels | mêmes droits ; brouillon hors ligne | — |
| MSG-152 | Dicter un message (mobile) | personnels | transcription locale ou fournisseur autorisé ; relu avant envoi | — |
| MSG-153 | Voir les familles **non lectrices** d'un fil ou d'un lot | auteur | et « joindre autrement » (socle 8 NTF-64) | — |
| MSG-154 | Relancer un fil sans réponse | auteur | une relance ; puis tâche | `messagerie.relance` |

### 19.3 Boîtes de service et traitement
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-160 | Créer une boîte de service (depuis le catalogue) | DIR · `messagerie.boites.gerer` | nom, nature, sensibilité, horaires, message d'accueil, délai visé | `messagerie.boite_creee` |
| MSG-161 | Rattacher les membres par **mission** (socle 6) | DIR | aucune liste figée ; aperçu des membres à la date | — |
| MSG-162 | Voir la boîte : nouveaux, pris, en attente, traités | membres · `messagerie.boite.traiter` | tri par urgence déterministe, échéance, enfant | — |
| MSG-163 | **Prendre en charge** un fil | membre | « Paul s'en occupe » visible des membres | `messagerie.fil_pris` |
| MSG-164 | Transférer à une autre boîte, avec note interne | membre | la famille voit « transmis au secrétariat » | `messagerie.fil_transfere` |
| MSG-165 | Écrire une **note interne** dans un fil | membre | invisible de la famille ; marquée | — |
| MSG-166 | Poser l'état « en attente de la famille » | membre | relance automatique facultative | — |
| MSG-167 | Poser une échéance, une étiquette | membre | liste courte d'étiquettes (S-135) | — |
| MSG-168 | Répondre au nom de la boîte | membre | « Vie scolaire — Paul D. » | `messagerie.message_envoye` |
| MSG-169 | Définir la **réponse automatique** hors horaires | DIR, responsable de boîte | texte court, numéros d'urgence | — |
| MSG-170 | Désigner une **astreinte** pour les messages urgents | DIR | facultatif ; personnes et plages | — |
| MSG-171 | Saisir un **message téléphonique** (accueil) | ACC, VS · `messagerie.message_tel.saisir` | personne présumée, motif, boîte ou personne destinataire, rappel demandé, carte « absence signalée » | `messagerie.message_tel_saisi` |
| MSG-172 | Traiter un **message entrant SMS / e-mail** (rattacher, répondre, classer) | VS · `messagerie.boite.traiter` | non vérifié ; jamais d'effet automatique (socle 8 NTF-74) | — |
| MSG-173 | Voir les délais de réponse de la boîte | responsable de boîte, DIR | médiane, plus anciens non traités ; jamais par agent nominatif hors de la direction (S-143) | — |
| MSG-174 | Fusionner deux fils d'une même famille sur un même sujet | membre | même foyer seulement | `messagerie.fils_fusionnes` |
| MSG-175 | Désigner la remplaçante d'une boîte C3 | INF, PSY, AS, DIR | datée | — |

### 19.4 Absence, départ, déconnexion
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-180 | Poser sa **réponse d'absence** et sa redirection | personnels | lue aussi au socle 15 ; sans motif | — |
| MSG-181 | Rendre ses fils de service au suppléant (automatique à la suppléance) | SYS (socle 6) | §11.2 | `messagerie.participant_ajoute` |
| MSG-182 | **Parcours de départ** : redistribuer les fils | SYS + DIR | proposition automatique, validation par la direction | `messagerie.fils_redistribues` |
| MSG-183 | Régler ses horaires de disponibilité | personnels | socle 8 §16 | — |
| MSG-184 | Mettre en **pause** (1 h, demain, lundi) | personnels | socle 8 | — |
| MSG-185 | Voir ce qui l'attend au retour (résumé des fils) | personnels | résumé déterministe ; IA facultative (N0) | — |
| MSG-186 | Voir « mes établissements » et basculer | personnels multi-établissements | aucune donnée croisée (D-013) | — |

### 19.5 Modération, protection, supervision
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-220 | Signaler un message (élève, parent, personnel) | participants | §14 | `messagerie.signalement_cree` |
| MSG-221 | Signaler un **comportement** (harcèlement d'un parent, d'un élève) | personnels | tâche à la direction | idem |
| MSG-222 | Traiter un signalement (classer, retirer, restreindre, transmettre) | DIR, référent · `messagerie.moderer` | motif ; décision tracée | `messagerie.signalement_traite` |
| MSG-223 | **Retirer** un message (modération) ★ | DIR · `messagerie.moderer` | contenu conservé ; trace « retiré par l'établissement » | `messagerie.message_retire` |
| MSG-224 | **Geler** un fil ★ | DIR, CPE, référent · `messagerie.geler` | preuve ; aucune purge, clôture, retrait | `messagerie.fil_gele` |
| MSG-225 | Lever un gel ★ | DIR | motif | `messagerie.gel_leve` |
| MSG-226 | **Restreindre** une personne (boîte seulement, lecture seule) ★ | DIR · `messagerie.restreindre` | datée, motivée, réexaminée à 30 jours ; information due maintenue | `messagerie.restriction_posee` |
| MSG-227 | Lire un fil en **supervision motivée** ★ | DIR, référent désigné · `messagerie.superviser` | motif écrit, journalisé (accès sensible), impossible sur C3 ; durée d'ouverture 24 h | `messagerie.supervision_ouverte` |
| MSG-228 | Voir les **indicateurs de vigilance** (fils encadrés) | référent désigné · `messagerie.vigilance.lire` | §10.2 ; sans contenu | — |
| MSG-229 | Déclencher le parcours **information préoccupante** depuis un fil | CPE, INF, AS, DIR | gel ; socle 36 ; aucun avis famille | `messagerie.fil_gele` |
| MSG-230 | Rappeler le **3018** à l'élève qui signale un harcèlement | SYS | message système dans le fil de signalement | — |
| MSG-231 | Exporter un fil pour une plainte ou une réquisition ★ | DIR · `messagerie.exporter` | gel préalable ; PDF horodaté avec empreintes ; journalisé | `messagerie.export_realise` |
| MSG-232 | Voir le **journal des supervisions** | DIR, DPO · `messagerie.journal.lire` | qui, quand, quel fil, motif | — |
| MSG-233 | Ouvrir un fil **de classe** ou **de délégués** animé par un adulte | PP, CPE | §10.4 | `messagerie.fil_ouvert` |

### 19.6 Carnet de correspondance
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-240 | Voir le carnet d'un élève (tous ses mots) | PP, CPE, PRF de l'élève (D-022), DIR | mots des deux foyers visibles du personnel, jamais d'un foyer à l'autre | — |
| MSG-241 | Écrire un mot (types) | PRF, VS, CPE | §18 | `messagerie.mot_cree` |
| MSG-242 | Mot en lot (oubli de matériel pour 4 élèves) | PRF | lot individuel (MSG-133) | `messagerie.lot_envoye` |
| MSG-243 | Suivre les signatures manquantes | auteur, PP, CPE | relance socle 8 | — |
| MSG-244 | Citer un mot dans une procédure disciplinaire | CPE, DIR (socle 13) | lien en lecture ; le mot ne devient pas une sanction | — |
| MSG-245 | Reprendre l'ancien carnet (020) | SYS | §26 | — |

### 19.7 Conservation, droits, export
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-250 | Exporter les échanges d'une personne (droit d'accès) | DPO, DIR · `messagerie.exporter` | §15 point 4 | `messagerie.export_realise` |
| MSG-251 | Appliquer la **purge** de conservation | SYS (socle 27) | sauf gel ; trace sans contenu | `messagerie.purge_realisee` |
| MSG-252 | Voir les fils gelés et leurs motifs | DIR | — | — |
| MSG-253 | Rectifier (droit de rectification) | DIR | on ne réécrit pas un message : on ajoute une mention de rectification | `messagerie.mention_ajoutee` |
| MSG-254 | Effacer un message à la demande d'une personne (droit à l'effacement) | DIR, DPO | refusé si conservation justifiée (preuve, mission d'intérêt public) ; sinon retrait + purge anticipée | `messagerie.message_efface` |

### 19.8 Paramétrage et pilotage
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| MSG-260 | Régler les **sept réglages** de la matrice (§8.3) | DIR · `messagerie.parametrer` | bornes non paramétrables ; aperçu des effets | `messagerie.parametre_change` |
| MSG-261 | Activer les pièces jointes des familles | DIR | — | idem |
| MSG-262 | Régler la plage calme de l'établissement | DIR | défaut national : 20 h–7 h en semaine, vendredi 20 h–lundi 7 h (§5 R-18) ; élargir librement, réduire avec motif | idem |
| MSG-263 | Gérer les modèles d'établissement | DIR, responsables de boîte | ≤ 50 ; revue annuelle | — |
| MSG-264 | Choisir la mention « rédigé avec l'assistant » côté destinataire | DIR | Q-M12 | idem |
| MSG-265 | Tableau de pilotage : volumes, délais par boîte, fils sans réponse, signalements | DIR | agrégé ; aucun classement des agents | — |
| MSG-266 | Désigner le **référent** de supervision et de vigilance | DIR | une ou deux personnes ; journalisé | idem |
| MSG-267 | Choisir les ouvertures élèves (délégués, classe) | DIR | §10.4 | idem |
| MSG-268 | Choisir le traitement des réponses par e-mail (rejet / boîte) | DIR | Q-M14 | idem |

### 19.9 Système (automatismes)
| ID | Action | Qui | Règles | Événement |
|---|---|---|---|---|
| MSG-280 | Retirer une personne des fils à la pose d'une **interdiction** | SYS (sur `eleve.interdiction_changee`) | sans message ; relu à chaque lecture | `messagerie.participant_retire` |
| MSG-281 | Suspendre / reprendre les participations d'un **lien suspendu** | SYS | — | idem |
| MSG-282 | Clore les participations d'une personne **décédée** | SYS (sur décès) | aucune relance | idem |
| MSG-283 | Basculer à la **majorité** | SYS (sur `eleve.majeur`) | §10.3 ; lit les droits conservés du socle 4 | — |
| MSG-284 | Dédoubler à la **séparation** d'un foyer | SYS (sur `foyer.cree`) | CM-04 | `messagerie.fil_dedouble` |
| MSG-285 | Tenir les **fils d'équipe** à jour | SYS (socle 6) | §11.4 | `messagerie.participant_ajoute` / `_retire` |
| MSG-286 | Appliquer les envois différés | SYS | file horaire ; fuseau de l'établissement | `messagerie.message_envoye` |
| MSG-287 | Rouvrir un fil traité quand la famille répond | SYS | — | — |
| MSG-288 | Clore les fils d'un élève parti | SYS (sur sortie) | entrée en conservation | `messagerie.fil_clos` |
| MSG-289 | Geler à la suspension conservatoire d'un agent | SYS (socle 6) | CM-43 | `messagerie.fil_gele` |
| MSG-290 | Recalculer l'indicateur de vigilance | SYS | nocturne, sans contenu | — |
| MSG-291 | Retirer un ancien professeur d'un fil 15 jours après la fin de son cours | SYS | CM-35 | `messagerie.participant_retire` |

### 19.10 IA
| ID | Action | Qui | Niveau | Règle |
|---|---|---|---|---|
| MSG-300 | Proposer une reformulation (ton, clarté) | IA au nom de la personne | N0 / N3 à l'envoi | §16 |
| MSG-301 | Préparer une réponse à un fil | IA | N3 | destinataires du fil |
| MSG-302 | Résumer un fil | IA | N0 | droits du lecteur |
| MSG-303 | Résumer ma boîte du matin | IA | N0 | membres |
| MSG-304 | Proposer une étiquette, une boîte, une urgence | IA | N1 | tri déterministe d'abord |
| MSG-305 | Traduire un message entrant | IA | N0 | original visible |
| MSG-306 | Traduire une réponse | IA | N3 | marquée, relue |
| MSG-307 | Proposer une carte d'action à partir d'un message | IA | N2 | le socle propriétaire exécute |
| MSG-308 | Signaler une injection probable | IA | — | alerte |

### 19.11 Socle 17 — publications (esquisse)
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| ANN-100 | Rédiger une annonce, une actualité, une lettre | DIR, CPE, ADF, PP (à sa classe), PRF (à ses cours) · `publications.rediger` | audience décrite sous les droits de l'auteur | `publications.brouillon_cree` |
| ANN-101 | Soumettre à validation | auteur | circuit si audience large (seuil du socle 8) | `publications.soumise` |
| ANN-102 | Valider ou renvoyer ★ | DIR · `publications.valider` | quatre yeux | `publications.validee` |
| ANN-103 | Publier, programmer, épingler, dépublier | auteur, DIR | période d'affichage | `publications.publiee` / `_retiree` |
| ANN-104 | Notifier l'audience (diffusion du socle 8) ★ | auteur · `notifications.diffuser` | aperçu, coût, confirmation N3 | `notifications.diffusion_lancee` |
| ANN-105 | Voir le suivi de lecture et relancer les non-lecteurs ★ | auteur | nouvel aperçu restreint | — |
| ANN-106 | Demander une prise de connaissance | auteur | comme §13 | — |
| ANN-107 | Joindre un document (carte document du socle 7) | auteur | droits relus | — |
| ANN-108 | Traduire (relu) | auteur | versions par langue | — |
| ANN-109 | Rubriques et modèles de publication | DIR | liste courte (D-023) | — |
| ANN-110 | Lire le fil d'actualité, filtrer par enfant et rubrique | PAR, ELV, personnels | audience à la date | — |
| ANN-111 | « Poser une question » sur une publication | PAR, ELV | ouvre un fil individuel avec l'auteur ou sa boîte (pas de commentaires publics) | `messagerie.fil_ouvert` |
| ANN-112 | Lettre d'information périodique (assemblage d'actualités) | ADF, DIR | gabarit prêt (D-023) | `publications.lettre_publiee` |
| ANN-113 | Campagne avec réponse attendue (coupon, document à rendre, renvoi au formulaire du socle 18) | DIR, CPE, ADF | suivi des réponses | `publications.campagne_lancee` |
| ANN-114 | Version papier pour les familles sans numérique | ADF | lot d'impression (socle 7) | — |
| ANN-115 | Corriger une publication envoyée | auteur | correctif aux seuls lecteurs et notifiés | `publications.corrigee` |
| ANN-116 | Archiver les publications de l'année | SYS | conservation | — |

---

## 20. Règles transverses (R-M1 et suivantes) [R]

| # | Règle | Lot |
|---|---|---|
| R-M1 | **Lire = participer.** Un compte lit un fil s'il en est participant actif (personne ou membre d'une boîte participante) **ou** s'il a ouvert une supervision motivée en cours de validité ; rien d'autre (ni le lien avec l'élève, ni le rôle) | M1 |
| R-M2 | **Un seul contrôle d'accès** (`peut_lire_fil`, `peut_ecrire_fil`) appelé par toute route qui touche un fil, un message, une pièce, un accusé, un export, une recherche, une notification ; test de garde qui échoue si une requête SQL de la brique lit `ent_msg` sans passer par la vue filtrée | M1 |
| R-M3 | **Un fil famille par foyer** : un message « aux responsables » crée un fil par foyer de résidence parmi les liens qui ont le droit de la nature ; jamais deux foyers dans un même fil | M2 |
| R-M4 | **Aucune coordonnée** (adresse, téléphone, e-mail) d'une personne n'est jamais composée dans un message, un aperçu, une signature ou un modèle par le logiciel | M0a, M2 |
| R-M5 | Le personnel ne choisit jamais une liste de familles à cocher : il choisit des **élèves**, un **cours** ou une **classe**, et le logiciel calcule les destinataires (socle 4), avec la raison | M2 |
| R-M6 | **Interdiction de communication** : exclusion de tout fil sur l'élève visé, relue à chaque lecture, écriture et avis ; retrait des fils existants à la pose ; exclusion affichée **agrégée** au personnel non habilité | M0a, M2 |
| R-M7 | **« Ne pas solliciter »** : aucun fil ouvert par l'établissement vers la personne (hors information due décidée par la direction), aucune publication ; réponse permise dans un fil qu'elle a ouvert | M0a, M2 |
| R-M8 | Lien suspendu ou non confirmé : aucune participation active ; décès : participations closes | M0a, M2 |
| R-M9 | **L'élève ne lit pas les fils adressés à ses responsables** ; ses responsables ne lisent pas ses fils ; un fil commun famille + élève n'existe que si l'auteur le choisit et que la nature le permet | M0a, M8 |
| R-M10 | Le professeur n'ouvre un fil vers une famille ou un élève que pour les élèves de **ses cours à la date** (D-022) ; il garde la réponse 15 jours après la fin (CM-35) | M2 |
| R-M11 | Les **boîtes C3** (infirmerie, psychologue, assistante sociale) ne sont lisibles que de leurs membres ; aucune supervision ordinaire, aucune recherche transversale, aucun résumé hors de la boîte, titre neutre partout ailleurs | M3 |
| R-M12 | Après la majorité, la réponse du personnel à un parent ne révèle aucune donnée de l'élève majeur que le socle 4 ne lui ouvre pas ; avertissement de rédaction si le fil contient une donnée d'assiduité, de notes ou de santé | M8 |
| R-M13 | **Aucun envoi de masse par la messagerie** : lot individuel ≤ 30 fils (S-139) ; au-delà, publication (17) ou diffusion (8) | M4 |
| R-M14 | Toute écriture de la brique publie ses événements **dans la transaction**, sans contenu ni sujet (identifiants, nature, compte) | M0a, M1 |
| R-M15 | La messagerie ne notifie que par les **natures déclarées** au socle 8 (§23) ; aucun appel à un fournisseur | M6 |
| R-M16 | **Rien de sensible hors de l'ENT** : enveloppe neutre (« Nouveau message de l'établissement à propos de Léa » au plus) ; jamais le sujet, le corps, le nom de l'auteur pour une boîte C3 | M0a, M6 |
| R-M17 | Suppléance : le suppléant devient participant des fils **du service** à la date ; jamais des fils personnels du titulaire | M9 |
| R-M18 | Départ, décès, suspension d'un personnel : aucun fil famille sans interlocuteur ; redistribution proposée, validée par la direction | M9 |
| R-M19 | **Lu par le destinataire**, pas par n'importe qui : une lecture en supervision, par un membre d'une autre boîte ou par l'auteur ne pose pas d'accusé | M0b, M6 |
| R-M20 | **Plage calme** (défaut national : 20 h–7 h en semaine, vendredi 20 h–lundi 7 h ; §5 R-18) : tout message d'un personnel vers une famille ou un élève écrit pendant la plage est **remis à sa fin** ; l'auteur le voit avant l'envoi ; le différé est annoncé dans la charte | M4 |
| R-M21 | Seule la direction remet immédiatement un message pendant la plage calme, pour une urgence, avec motif tracé ; vers un élève mineur, seulement depuis une boîte C3 ou pour une urgence | M4, M8 |
| R-M22 | Pas d'indicateur de présence ni d'heure de lecture des personnels côté familles et élèves | M6 |
| R-M23 | **Indicateur de vigilance** des fils encadrés : déterministe, sans contenu, sans IA, visible du seul référent ; jamais un score sur une personne | M10 |
| R-M24 | Supervision motivée : motif écrit, durée 24 h, journalisée comme accès sensible, visible du DPO ; impossible sur C3 | M10 |
| R-M25 | Signalement : gel immédiat du message signalé ; l'auteur n'est pas prévenu automatiquement | M10 |
| R-M26 | Bandeau d'urgence permanent ; la messagerie ne promet aucun délai la nuit | M3 |
| R-M27 | Correction : 15 minutes, si non lu ; version gardée ; marque « modifié » | M4 |
| R-M28 | **Aucune suppression physique par un utilisateur** ; retrait = masquage tracé ; destruction à la purge seulement | M4, M11 |
| R-M29 | Purge selon la durée de la catégorie (socle 27), jamais d'un fil gelé | M11 |
| R-M30 | Un fil marqué « information préoccupante », signalé, ou lié à une procédure est **gelé** | M10 |
| R-M31 | Pièces : quarantaine, analyse, URL pré-signée de 60 s après contrôle des droits du lecteur | M5 |
| R-M32 | Pièce d'un message retiré : conservée, plus servie | M5 |
| R-M33 | Un document du coffre se joint par **carte document**, jamais par fichier | M5, M7 |
| R-M34 | Carte d'action : exécutée par le socle propriétaire, droits relus au geste ; son état affiché en direct ; jamais exécutée par la messagerie | M7 |
| R-M35 | Prise de connaissance signée : par un titulaire (ou l'élève majeur) ; jamais par un mineur pour un mot destiné aux parents | M7 |
| R-M36 | Une observation du carnet n'est jamais une sanction (socle 13 cite, ne transforme pas) | M7 |
| R-M37 | **IA** : aucun envoi, ajout de destinataire, modération, supervision ou paramétrage ; brouillon N3 vers les participants du fil ; texte non fiable encadré ; traduction marquée, original visible | M13 |
| R-M38 | Export d'une personne : seuls les fils où elle participe | M11 |
| R-M39 | Un fil ne traverse jamais deux établissements (D-013) | M1 |
| R-M40 | Restriction d'une personne : motivée, datée, réexaminée à 30 jours ; l'information due continue | M10 |
| R-M41 | Messages entrants (SMS, e-mail) : **non vérifiés**, texte non fiable, jamais d'effet automatique | M3 |
| R-M42 | Fils d'équipe tenus par le socle 6 ; aucune famille dans un fil d'équipe | M9 |
| R-M43 | Pas de messagerie élève ↔ élève hors fils animés par un adulte (Q-M6) | M8 |
| R-M44 | Longueurs maximales : sujet 200 caractères, message 20 000, 5 pièces | M0b |
| R-M45 | **Toute demande d'information ou d'entrevue d'un parent reçoit une réponse** ; un refus est motivé (C. éduc. D. 111-4 [V]) : une boîte ne passe pas un fil famille en « traité » sans réponse écrite ; le pilotage montre les demandes sans réponse | M3 |
| R-M46 | Un fil **interne** que ses participants marquent « personnel » n'est jamais ouvert en supervision ordinaire (jurisprudence Nikon et présomption de caractère professionnel, §5 R-12) | M10 |
| R-M47 | Un texte produit ou traduit avec l'assistant porte une **marque lisible par machine** (`ia_assiste`, `traduit`) en plus de la mention visible (règlement IA art. 50 §2 [V]) | M13 |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]
| Action | Portées | Profils par défaut |
|---|---|---|
| `messagerie.ecrire` | soi (familles, élèves) ; cours (professeur) ; classe (PP) ; périmètre (CPE) ; établissement | tous, bornés par la matrice |
| `messagerie.eleve` (existante, conservée) | cours, classe, établissement | personnels ; **ne donne plus la lecture des fils** (R-M1), seulement le droit d'en ouvrir à propos d'un élève |
| `messagerie.lot.envoyer` | cours, classe, établissement | PRF, PP, CPE |
| `messagerie.boite.traiter` | une boîte | membres (missions) |
| `messagerie.boites.gerer` | établissement | DIR |
| `messagerie.message_tel.saisir` | établissement | ACC, VS |
| `messagerie.moderer` | établissement | DIR |
| `messagerie.geler` | établissement | DIR, CPE, référent |
| `messagerie.restreindre` | établissement | DIR |
| `messagerie.superviser` ★ | établissement | DIR, référent désigné (pas de défaut au-delà) |
| `messagerie.urgence_hors_plage` ★ | établissement | DIR |
| `messagerie.vigilance.lire` | établissement | référent désigné |
| `messagerie.exporter` ★ | établissement | DIR, DPO |
| `messagerie.journal.lire` | établissement | DIR, DPO |
| `messagerie.parametrer` | établissement | DIR |
| `carnet.ecrire` | cours, classe, périmètre | PRF, PP, CPE, VS |
| `carnet.lire` | cours, classe, périmètre | PRF, PP, CPE, VS, DIR |
| `publications.rediger` | cours, classe, établissement | PRF, PP, CPE, ADF, DIR |
| `publications.valider` ★ | établissement | DIR |

### 21.2 Ce que voit chacun [R]
| Qui | Voit | Ne voit jamais |
|---|---|---|
| Parent | ses fils ; les mots adressés à son foyer ; les publications de son audience | les fils de l'autre foyer, de son enfant, des personnels ; la liste des membres d'une boîte ; l'heure de lecture d'un personnel |
| Élève | ses fils ; les mots qui lui sont adressés ; les publications élèves | les fils de ses parents ; les notes internes |
| Professeur | ses fils ; ceux des boîtes dont il est membre ; le carnet des élèves de ses cours | les fils privés d'un parent avec d'autres services ; les relances de facture ; les boîtes C3 |
| CPE / VS | la boîte vie scolaire ; ses fils | les boîtes C3 ; les fils de l'intendance |
| Direction | ses fils ; ses boîtes ; **par supervision motivée**, tout fil non C3 | les boîtes C3 |
| Infirmière, psychologue, AS | leur boîte | les autres boîtes sauf participation |
| AESH | les fils d'équipe de l'élève accompagné ; les fils où il est ajouté | tout le reste |
| Accueil | les messages téléphoniques qu'il a saisis (jusqu'à leur remise) | la suite des fils |

### 21.3 Pièges de minimisation à tester [R]
1. Compteur de non-lus qui inclut les fils d'un autre foyer (fuite par le nombre).
2. Sujet dans l'événement, la chronologie, le journal, la notification, l'objet d'e-mail.
3. Recherche plein texte qui renvoie un extrait d'un fil non lisible.
4. Accusé « lu par Mme X » qui révèle à un parent qu'un autre membre du personnel a lu.
5. Aperçu « 1 personne exclue (décision) » dans une classe de 25 (socle 8 R-N51).
6. Export d'un parent qui contient les messages de l'autre parent cités dans un fil commun ancien.
7. Pièce servie par une URL durable copiée dans un autre fil.
8. Nom d'un élève tiers (victime) dans l'enveloppe d'un avis.
9. Résumé IA d'un fil C3 dans une boîte non C3.
10. Chronologie de l'élève lue par une famille qui listerait les fils du personnel.

---

## 22. Événements (outbox, dans la transaction du fait) [R]
Charges sans contenu ni sujet. Noms historiques gelés (`msg.fil_ouvert`, `msg.envoye`, `msg.accuse`, `msg.piece`, `msg.fil_archive`) : charges réduites en M0a (retrait du sujet), puis remplacés par les noms `messagerie.*`.

| Événement | Charge | Consommé par |
|---|---|---|
| `messagerie.fil_ouvert` | fil, nature, sensibilité, élèves concernés (ids), boîte | 8 (avis), 2 (chronologie du personnel, titre neutre) |
| `messagerie.message_envoye` | fil, message, côté de l'auteur | 8 |
| `messagerie.participant_ajoute` / `_retire` | fil, personne, raison codée | 8 (avis « a rejoint »), journal |
| `messagerie.fil_pris`, `_transfere`, `_clos` | fil, boîte | pilotage |
| `messagerie.mot_cree`, `messagerie.mot_signe` | mot, élève, type | 8 (relance), 13 (lecture) |
| `messagerie.carte_inseree` | message, socle, objet | socle propriétaire |
| `messagerie.signalement_cree` / `_traite` | signalement, catégorie | 8 (tâche direction), 13, 36 |
| `messagerie.fil_gele` / `gel_leve` | fil, motif codé | 27 (purge suspendue) |
| `messagerie.restriction_posee` / `_levee` | personne, portée | 8 |
| `messagerie.supervision_ouverte` | fil, superviseur, motif codé | journal sensible |
| `messagerie.message_retire`, `_corrige` | message, par qui (auteur / établissement) | 8 (avis retiré), journal |
| `messagerie.export_realise` | personne ou fil, motif codé | 27 |
| `messagerie.purge_realisee` | nombre, catégorie | 27 |
| `publications.publiee`, `_retiree`, `_corrigee` | publication, audience décrite | 8 |

**Consommés** : `eleve.interdiction_changee`, `lien.suspendu`, `lien.clos`, `personne.decedee`, `foyer.cree`, `eleve.majeur`, `eleve.sorti`, `eleve.confidentialite_changee`, `eleve.fusionne` (socles 2, 4) ; `intervention.ouverte` / `_close`, `agent.parti`, `agent.suspendu`, `equipe.changee` (socle 6) ; `cours.membres_changes` (socle 5) ; `absence_personnel.*` (socle 15).

---

## 23. Notifications (natures déclarées au socle 8) [R]

| Nature | Urgence | Sensibilité | Destinataires | Plancher | Remarque |
|---|---|---|---|---|---|
| `messagerie.nouveau_message` | U1 | C2 (enveloppe) ; C3 pour une boîte C3 | participants nommés du fil, hors auteur | in-app | regroupement par fil ; une relance au plus ; disponibilité des personnels |
| `messagerie.nouveau_message_boite` | U1 | C2 | membres de la boîte (in-app), astreinte si urgence déterministe | in-app | personnels seulement |
| `messagerie.reponse_attendue` | U1 | C2 | participant famille | non | « la vie scolaire attend votre réponse » |
| `messagerie.mot_a_signer` (= `messagerie.carnet_a_signer` du socle 8) | U1 | C2 | titulaires du foyer du fil | 1 externe | relance J+2 |
| `messagerie.participant_ajoute` | U0 | C1 | participants famille | non | « M. C. a rejoint la conversation » |
| `messagerie.signalement` | U2 | C3 | direction, référent | in-app | jamais l'auteur signalé |
| `messagerie.restriction` | U1 | C2 | personne restreinte | in-app | texte de la direction |
| `publications.nouvelle` | U0 | C0 si déclarée publique, sinon C2 | audience résolue | non | diffusion du socle 8 |

---

## 24. Écrans clés [R]
| Écran | Route | Pour | Contenu |
|---|---|---|---|
| **Mes conversations** | `/web/messages` | tous | liste unique (fils, mots, publications à part), filtres enfant / établissement / état ; non lus ; « en attente de votre réponse » |
| **Écrire** | `/web/messages/nouveau` | familles, élèves | « À qui voulez-vous écrire ? » par motif → boîte ou personne ; horaires et délai affichés ; bandeau d'urgence |
| **Écrire (personnel)** | `/web/messages/nouveau` | personnels | choisir des élèves, un cours, une classe, une équipe ; aperçu des fils créés (« 3 élèves → 4 conversations ») et des exclusions agrégées ; modèles ; programmation ; assistant |
| **Fil** | `/web/messages/{id}` | participants | messages, pièces (état d'analyse), cartes d'action vivantes, mots, participants (côté personnel), notes internes, traduction, « lu », signaler |
| **Boîte de service** | `/web/messages/boites/{id}` | membres | colonnes nouveau / pris / en attente / traité, échéances, étiquettes, prise en charge, transfert ; tri par urgence |
| **Carnet de l'élève** | `/web/eleves/{id}/carnet` | personnels, familles | mots, signatures manquantes |
| **Message téléphonique** | `/web/accueil/message` | accueil | trois champs, destinataire, carte absence |
| **Signalements et gels** | `/web/messages/moderation` | direction, référent | file, décisions, restrictions, journal des supervisions |
| **Paramètres** | `/web/parametres/messagerie` | direction | sept réglages, boîtes, plages, référent |
| **Publications** | `/web/publications` | auteurs, lecteurs | rédaction, validation, suivi de lecture |
| **Mobile** (appli unique, D-003) | — | tous | conversations, écrire, répondre, carnet, signer, cartes ; dictée ; hors ligne en brouillon |

Exigences : clavier complet, 375 px, lecteur d'écran (RGAA, §5), temps réel (M12) ; les écrans Leptos existants (`modules/messagerie`) sont étendus, pas réécrits ; `ui.rs` n'est pas étendu (F-001).

---

## 25. Modèle de données cible (brique `messagerie`)

Crates : `brique-messagerie` (serveur), `brique-messagerie-types` (types purs, compilables en WebAssembly), `brique-messagerie-web` (écrans) (D-026). Toutes les tables portent `etablissement_id`, une clé composite `(etablissement_id, id)` et la RLS dès la création ; aucune clé étrangère vers une autre brique (source par triplet), clés vers le noyau seulement (`ent_personne`, `ent_user`, `ent_eleve`).

```
ent_msg_fil                                        -- existante (023), enrichie ; id conservé
  id, etablissement_id
  nature        TEXT NOT NULL CHECK (nature IN ('famille','eleve','interne','equipe','carnet','boite','systeme','classe','delegues'))
  sujet         TEXT NOT NULL CHECK (length(sujet) <= 200)
  sensibilite   TEXT NOT NULL DEFAULT 'C2' CHECK (sensibilite IN ('C1','C2','C3'))
  etat          TEXT NOT NULL DEFAULT 'ouvert' CHECK (etat IN ('ouvert','clos','gele'))   -- gel aussi dans ent_msg_gel (historique)
  origine       TEXT NOT NULL CHECK (origine IN ('personnel','famille','eleve','systeme','telephone','sms_entrant','courriel_entrant'))
  ouvert_par_personne_id UUID NOT NULL → ent_personne ; ouvert_par_user_id UUID NULL
  lot_id        UUID NULL → ent_msg_lot ; jumeau_de UUID NULL → ent_msg_fil   -- fils jumeaux (R-M3)
  source_brique TEXT NULL, source_type TEXT NULL, source_id UUID NULL   -- fil système (absence, facture, document)
  langue        TEXT NULL
  cree_le, dernier_message_le, clos_le, clos_par
  -- colonnes héritées en sursis : eleve_id (→ ent_msg_fil_eleve), created_by_user_id, archive, archived_at,
  --   priorite, dossier (→ état par participant), prive_user_id, foyer_id (→ participants), modele_id

ent_msg_fil_eleve                                  -- « à propos de » (0..n élèves)
  etablissement_id, fil_id, eleve_id → ent_eleve ; PK (fil_id, eleve_id)

ent_msg_participant
  id, etablissement_id, fil_id
  personne_id   UUID NULL → ent_personne           -- personne (avec ou sans compte)
  boite_id      UUID NULL → ent_msg_boite          -- ou une boîte (exactement un des deux)
  cote          TEXT NOT NULL CHECK (cote IN ('etablissement','famille','eleve'))
  qualite       TEXT NOT NULL CHECK (qualite IN ('auteur','destinataire','equipe','suppleant','ajoute','successeur'))
  raison        TEXT NOT NULL                       -- code : 'titulaire_foyer','payeur','eleve','professeur_cours','pp','mission','boite','equipe' …
  peut_ecrire   BOOLEAN NOT NULL DEFAULT true
  du, au NULL, suspendu_le NULL, motif_sortie TEXT NULL CHECK (codes)   -- jamais de motif en clair (interdiction)
  -- état personnel :
  lu_jusqu_a_message_id UUID NULL, lu_le TIMESTAMPTZ NULL, muet BOOLEAN, archive_le NULL, epingle BOOLEAN
  UNIQUE (fil_id, personne_id) WHERE au IS NULL ; UNIQUE (fil_id, boite_id) WHERE au IS NULL

ent_msg                                            -- existante (023), enrichie ; id conservé
  id, etablissement_id, fil_id
  auteur_personne_id UUID NOT NULL, auteur_user_id UUID NULL, au_nom_de_boite_id UUID NULL
  type          TEXT NOT NULL DEFAULT 'texte' CHECK (type IN ('texte','note_interne','systeme','mot','carte','telephone'))
  corps         TEXT NOT NULL CHECK (length(corps) <= 20000)
  version       INT NOT NULL DEFAULT 1, modifie_le NULL
  reponse_a     UUID NULL → ent_msg
  remise_prevue_le TIMESTAMPTZ NULL                -- envoi différé (R-M20, R-M21) ; NULL = remis à la création
  remis_le      TIMESTAMPTZ NULL
  retire_le NULL, retire_par NULL, retrait_par TEXT NULL CHECK (retrait_par IN ('auteur','etablissement')), retrait_motif_code NULL
  verifie       BOOLEAN NOT NULL DEFAULT true       -- false : SMS / e-mail entrant
  ia_assiste    BOOLEAN NOT NULL DEFAULT false
  langue_source TEXT NULL
  empreinte     TEXT NOT NULL                       -- SHA-256 du corps, preuve

ent_msg_version                                    -- ajout seul
  message_id, version, corps, remplace_le, par

ent_msg_piece                                      -- existante (038), remplacée par l'objet
  id, etablissement_id, message_id
  objet_cle TEXT NOT NULL, nom, mime, taille, empreinte
  etat_analyse TEXT NOT NULL CHECK (etat_analyse IN ('quarantaine','saine','refusee'))
  converti_cle TEXT NULL
  -- contenu BYTEA en sursis (reprise vers l'objet, puis colonne retirée)

ent_msg_boite
  id, etablissement_id, code (catalogue), libelle, sensibilite ('C2','C3'), actif
  horaires JSONB, message_accueil TEXT, reponse_auto TEXT, delai_vise_jours SMALLINT
  mission_codes TEXT[]                              -- membres = agents porteurs de ces missions (socle 6)
  astreinte JSONB NULL

ent_msg_traitement                                 -- un par (fil, boîte)
  fil_id, boite_id, statut ('nouveau','pris','attente_famille','traite'), pris_par NULL, pris_le NULL, echeance NULL,
  etiquettes TEXT[], urgence_deterministe BOOLEAN, maj_le

ent_msg_lot                                        -- lot d'envoi individuel (≤ 30)
  id, etablissement_id, auteur_personne_id, empreinte_apercu, nb_fils, confirme_le, assurance ('simple','passkey')

ent_msg_mot                                        -- carnet
  message_id PK, eleve_id, type ('information','observation','oubli','retard','demande_rdv','autorisation','mot_famille'),
  signature_requise BOOLEAN, signe_par_personne_id NULL, signe_le NULL, version_signee INT NULL,
  responsables_seulement BOOLEAN

ent_msg_carte
  message_id, brique, type, objet_id, libelle_neutre    -- l'état est lu en direct auprès du socle propriétaire

ent_msg_signalement
  id, etablissement_id, message_id, fil_id, par_personne_id, categorie, commentaire NULL (non fiable),
  statut ('ouvert','classe','traite'), decision_code NULL, traite_par NULL, traite_le NULL

ent_msg_gel                                        -- ajout seul
  id, fil_id, motif_code ('signalement','information_preoccupante','procedure','requisition','suspension_agent','autre'),
  reference NULL, par, du, leve_le NULL, leve_par NULL, leve_motif NULL

ent_msg_restriction
  id, personne_id, portee ('boite_seulement','lecture_seule'), boite_id NULL, motif_scelle, du, au, reexamen_le, par

ent_msg_supervision                                -- ajout seul ; double le journal d'accès sensible
  id, fil_id, superviseur_user_id, motif TEXT NOT NULL, ouvert_le, expire_le

ent_msg_absence                                    -- réponse d'absence d'un personnel
  personne_id, du, au, texte, redirige_vers_boite_id NULL, redirige_vers_personne_id NULL

ent_msg_parametre                                  -- les sept réglages et les plages (§8.3), versionnés
  etablissement_id, cle, valeur, du, par

ent_msg_modele                                     -- existante (047), enrichie : portee ('personnel','boite','etablissement'), boite_id NULL

ent_msg_vigilance                                  -- indicateurs calculés (sans contenu)
  fil_id, calcule_le, criteres TEXT[]
```

**Lectures filtrées** : vue `ent_msg_fil_lisible(user_id)` = fils où l'utilisateur est participant actif (personne liée au compte ou membre d'une boîte participante à la date) ∪ supervisions en cours ; **toutes** les requêtes de lecture de la brique passent par elle (R-M2).

**Socle 17 (esquisse, plage propre)** : `ent_pub_publication` (type, titre, corps, rubrique, audience décrite JSON, période, épingle, prise de connaissance demandée, état brouillon / à valider / programmée / publiée / retirée, auteur, valideur, diffusion_id du socle 8), `ent_pub_version` (ajout seul), `ent_pub_traduction`, `ent_pub_lecture` (ou lecture des preuves du socle 8), `ent_pub_rubrique` ; reprise de `ent_annonce` (086) et du contenu de `ent_campagne` (191).

---

## 26. Plan de migration depuis l'existant (sans casser le web, le mobile ni le labo)
1. **M0 sur les tables actuelles** : correctifs sans changer le modèle (filtres, textes neutres, RLS permissive) ; une seule migration (colonne `audience` provisoire, `prive_user_id` posé pour les fils ouverts par un élève).
2. **Participants (M1)** : création des tables, **reprise** des fils (MIG-1 à MIG-5 ci-dessous), vue filtrée, bascule du contrôle d'accès sur la participation ; les colonnes `prive_user_id` et `foyer_id` deviennent des colonnes en sursis maintenues en lecture seule.
3. **Reprise** :
   - MIG-1 fil ouvert par un parent (`prive_user_id`) → participants : ce parent (cote famille) + boîte « vie scolaire » si l'auteur du premier message du personnel en est membre, sinon les personnels qui ont écrit ;
   - MIG-2 fil de relance (`foyer_id`) → payeurs actifs du foyer + boîte intendance ;
   - MIG-3 fil ouvert par l'établissement → responsables du foyer de chaque lecteur effectif ; si deux foyers ont lu : **dédoublé** en deux fils jumeaux (messages copiés avec le même identifiant d'origine, horodatages conservés) ; si personne n'a lu : un fil par foyer ; rapport à la direction ;
   - MIG-4 fil ouvert par un élève → l'élève + personnels qui ont écrit ; **aucun** parent ;
   - MIG-5 carnet (020) → mots dans un fil carnet par foyer ; signatures reprises ;
   - pièces `BYTEA` → stockage objet (M5), état `saine` après analyse a posteriori.
4. **Compatibilité** : les routes `/messages/*` actuelles restent servies par la nouvelle brique (mêmes formes de réponse) jusqu'à la migration des clients (M16) ; tests existants conservés et rejoués.
5. **Retrait** (M17) : colonnes en sursis, `ent_msg_accuse`, `ent_carnet_entree`, contenu `BYTEA`, après la durée d'archive.

---

## 27. Lots de code (équipe Sonnet)

### 27.1 Tableau des lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **M0a** correctifs urgents — fuites | D1 (interdiction de communication relue à la lecture, à l'écriture et dans les envois de la messagerie, du carnet et des annonces) ; D2 (règle provisoire : un fil privé ou de foyer n'est lisible du personnel que par la direction, la CPE, le secrétariat ou l'intendance pour les relances, et par les personnels qui y ont écrit) ; D3 (fil ouvert par un élève privé à l'élève et au personnel ; fils système et carnet « aux responsables » invisibles de l'élève ; colonne `audience`) ; D5 (droits du lien : `peut_voir_vs` pour les fils de vie scolaire, autorité pour la signature) ; D6 (« ne pas solliciter » dans le push, les annonces, les campagnes, les rencontres) ; D7 (carnet : `autoriser_eleve`, signature d'un titulaire) ; D9 (droits D-017 au lieu des rôles) ; D4 (textes neutres : push, SMS, e-mail du carnet ; sujet retiré des événements) ; D10 (RLS permissive sur toutes les tables de communication) | rien (**peut partir tout de suite**) | tests : parent interdit ne lit, ne reçoit, ne signe rien ; professeur ne lit pas le fil privé d'un parent ni une relance ; élève ne lit pas l'alerte d'absences adressée aux parents ; parents ne lisent pas le fil ouvert par l'élève ; parent sans `peut_voir_vs` ne lit pas la vie scolaire ; « ne pas solliciter » exclu des campagnes et rencontres ; professeur hors périmètre n'écrit pas au carnet ; aucun sujet dans `ent_metier_event` ni dans un titre de push ; inventaire RLS ; non-régression RP1-11 et de `separation.rs` ; CI verte |
| **M0b** correctifs urgents — fiabilité | D8 (annonces : aperçu chiffré, confirmation, retrait, envoi par la chaîne S2-8, plus de boucle synchrone) ; D11 (réponse = désarchivage, route de désarchivage) ; D12 (pagination de la boîte) ; D13 (notification in-app du personnel) ; D14 (erreurs d'envoi journalisées) ; D15 (accusé posé par le côté destinataire seulement) ; D16 (taille bornée au `CHECK`, réponse 413 claire) ; D17 (push du fil de modèle, longueurs maximales R-M44) ; D18 (annonces par `classe_id`) ; D20 (test de fusion) | M0a | tests de chaque défaut ; aucune régression ; CI verte |
| **M1** participants et contrôle d'accès | tables §25 (fil enrichi, `fil_eleve`, participants, messages enrichis, versions), clés composites (D19), vue filtrée, contrôle unique R-M1/R-M2, reprise MIG-1 à MIG-4 et rapport, événements `messagerie.*` en outbox | M0a | test d'or de reprise (aucun lecteur ne gagne ni ne perd un fil hors MIG-3 documenté) ; garde « aucune lecture hors vue » ; matrice de séparation étendue (familles, élève, personnels, boîtes) ; `MIGRATION_TAG` calculé |
| **M2** matrice et destinataires | `peut_ecrire` (§8), ports socles 4, 5, 6, sept réglages bornés, fils par foyer (R-M3), fils jumeaux, exclusions agrégées, interdiction / « ne pas solliciter » / suspension / décès relus, dédoublement à la séparation (MSG-118, MSG-284), majorité (raccord socle 4) | M1 ; socle 4 port `destinataires` (ou adaptateur provisoire sur les liens actuels) ; socle 5 cours ; socle 6 P2 | CM-01 à CM-18 en tests ; professeur de LV2 (CM-38) ; aperçu sans coordonnées |
| **M3** boîtes de service | boîtes du catalogue, membres par mission, traitement (états, prise en charge, transfert, notes internes, échéances, étiquettes), réponse au nom de la boîte, horaires, réponse automatique, astreinte, message téléphonique (accueil), messages entrants non vérifiés, boîtes C3 (R-M11), bandeau d'urgence | M2 ; socle 6 (missions) | CM-31, CM-44, CM-49 ; aucune lecture C3 hors membres ; aucune liste de membres côté famille |
| **M4** écrire mieux | lots individuels ≤ 30 avec aperçu, modèles (personnel, boîte, établissement), programmation, envoi différé par plage (R-M20, R-M21), correction 15 min, retrait 10 min, brouillons, scission et fusion de fils | M2 | CM-26, CM-45, CM-54 à CM-56, CM-79 ; aucun message d'adulte remis à un mineur la nuit |
| **M5** pièces | stockage objet, quarantaine, antivirus (moteur à choisir, Q-M15), conversion (socle 7, D-005), URL pré-signées 60 s, reprise des `BYTEA`, carte document | M1 ; socle 7 (dépôt, conversion) | CM-57 à CM-60 ; URL expirée refusée ; pièce d'un fil non lisible refusée |
| **M6** lecture et avis | « lu jusqu'à » par participant, accusés du côté destinataire (R-M19), prise en charge visible, natures du §23 déclarées au socle 8 (ou adaptateur sur la chaîne S2-8 si N1 n'est pas prêt), enveloppes neutres, disponibilité des personnels | M1 ; socle 8 N1 (ou adaptateur) | « lu » n'apparaît jamais après une supervision ; aucune donnée C2 dans un push ; CM-61, CM-62 |
| **M7** carnet et cartes d'action | mots typés, prise de connaissance signée, reprise du carnet 020 (MIG-5), cartes (absence, document, formulaire, rendez-vous, facture) branchées sur les services publiés des socles propriétaires | M1 ; socles 12, 7, 18, 19, 24 (cartes ajoutées au fil de leur disponibilité) | CM-63 ; une carte exécutée revérifie les droits ; observation jamais transformée en sanction |
| **M8** élèves | matrice élève, fils encadrés, horaires des mineurs, discrétion (MSG-128), fils de délégués et de classe animés, élève–élève fermé (R-M43), majorité (R-M12) | M2, M4 | CM-25 à CM-38 |
| **M9** personnels | fils d'équipe tenus par le socle 6, suppléance (R-M17), départ et redistribution (R-M18), absence et réponse automatique, délégations, « mes établissements » | M3 ; socle 6 (interventions, équipes) ; socle 15 (absences) | CM-39 à CM-53 |
| **M10** protection | signalements, gel, retrait par modération, restrictions, supervision motivée journalisée, indicateur de vigilance déterministe, parcours information préoccupante (raccord socle 36), rappels 3018 / 119 | M1, M3 | CM-29, CM-30, CM-65, CM-70, CM-72 ; aucune supervision possible sur C3 ; journal complet |
| **M11** conservation et droits | durées par catégorie (socle 27), purge par lot, export par personne (R-M38), rectification par mention, effacement encadré, dépôt au dossier (socle 7) | M1, M10 ; socle 27 (durées, DPO) | CM-64, CM-66 ; un fil gelé n'est jamais purgé |
| **M12** temps réel et mobile | flux serveur (SSE) par compte, filtré par la vue (R-M2), API de l'appli unique (D-003), brouillons hors ligne, dictée | M1, M6 | aucun événement poussé à un non-participant ; parité des droits web / mobile |
| **M13** IA (après le cœur IA) | capacités §16.3, brouillons N3, résumé, tri (déterministe d'abord), traduction marquée, cartes proposées, détection d'injection ; fournisseur nul et simulé | M3, M6 ; cœur IA | jeux d'injection (CM-75) ; refus « répondre à tout » (CM-76) ; aucune capacité fermée accessible |
| **M14** écrans | écrans du §24 en Leptos, accessibilité, 375 px | au fil des lots | axe-core ; clavier ; lecteur d'écran |
| **M15** publications (socle 17) | publications, rubriques, audiences décrites, validation, traductions, suivi et relance par le socle 8 N9, « poser une question » → fil ; reprise de 086 et du contenu de 191 | M2 ; socle 8 N9 | conception propre du socle 17 relue ; aucune publication sans aperçu et confirmation |
| **M16** migration des clients | UI JS (F-001, non étendue) et applis sur les nouvelles routes ; fermeture des anciennes | M12, M14 | aucun appel aux anciennes routes pendant 30 jours |
| **M17** RLS stricte et retraits | RLS stricte, retrait des colonnes et tables en sursis | M16 | inventaire RLS ; tests d'isolation |

**Parcours minimal vendable** : M0a, M0b, M1, M2, M3, M4, M6, M12 (mobile minimal) — une famille écrit au bon service, la bonne personne répond, personne d'autre ne lit, chacun est prévenu sans fuite. **Avant tout pilote avec des élèves** : M8 et M10 (fils encadrés, signalement, gel).

### 27.3 État du lot M0a (implémenté 2026-11-09)

Niveau : **implémenté et testé en local** ; validé par la CI GitHub : voir `docs/PROJECT_STATUS.md`, « Messagerie, lot M0a » ; non validé par Florian. Règle de lecture : **découvert** = §2.2 ; **corrigé** = code + test d'attaque (un témoin précède chaque attaque).

| Défaut | État | Correctif (code) | Test d'attaque (`ent-db`, `tests_messagerie_m0a.rs`) |
|---|---|---|---|
| **D1** interdiction de communication | corrigé | vue `ent_eleve_lien_communicant` (480) = lien actif sans interdiction `communication` en cours (dates, levée, personne fusionnée) ; contrôle d'accès du fil, ouverture, réponse, carnet (liste, signature), e-mails, SMS, push, annonces, campagnes, rencontres, notifications S2-8 passent par cette vue | `m0a_d1_parent_interdit_…` (lecture, écriture, signature, e-mail/SMS/push, annonce, levée) ; `m0a_d1_interdiction_echue_ou_autre_objet_sans_effet` |
| **D2** personnel lit tout fil privé / de foyer | corrigé (règle provisoire) | un fil privé d'un parent n'est lisible que par `admin`, `direction`, `administratif`, `cpe` ; une relance (`foyer_id`) que par `admin`, `direction`, `administratif`, `compta` ; dans les deux cas aussi par le personnel qui y a écrit ; toujours après le droit `messagerie.eleve` et la portée D-022 ; liste, lecture, réponse, pièces, liens courts (point unique `actor_can_access_fil`) | `m0a_d2_le_personnel_qui_suit_l_eleve_…` |
| **D3** élève | corrigé | colonne `ent_msg_fil.audience` (`responsables` par défaut, `famille`, `eleve`) ; un fil ouvert par un élève est `eleve` + `prive_user_id` (reprise 480) : lisible de l'élève et du personnel, jamais d'un parent, jamais notifié aux parents ; carnet, alertes et fils de l'établissement : `responsables`, invisibles de l'élève | `m0a_d3_l_eleve_…` |
| **D4** contenu hors de l'ENT | corrigé pour la messagerie, le carnet, les annonces et campagnes | titre de push fixe (« Nouveau message ») ; carnet : e-mail, SMS et push sans titre ni contenu ; annonces et campagnes : SMS et push sans contenu (le texte reste dans l'ENT et l'e-mail) ; `sujet` / `titre` retirés des événements `msg.fil_ouvert` et `eleve.carnet_entree` ; titre de l'alerte d'absences neutre | `m0a_d4_ni_sujet_ni_contenu_…` |
| **D5** droits du lien | corrigé pour les fils | colonne `vie_scolaire` (alerte de seuil, carnet) : un parent ne lit ce fil qu'avec `peut_voir_vs` **et** l'autorité parentale ; la signature du carnet exige l'autorité | `m0a_d5_parent_sans_droit_vie_scolaire_…` ; `m0a_d5_d7_signature_…` |
| **D6** « ne pas solliciter » | corrigé | vue `ent_eleve_lien_sollicitable` (communicant sans « ne pas solliciter ») pour le push, les campagnes, les rencontres et les notifications S2-8 ; la personne reste lectrice et répondante de ses propres fils ; l'e-mail et le SMS l'excluaient déjà | `m0a_d6_ne_pas_solliciter_…` |
| **D7** carnet sans périmètre | corrigé | écriture : droit effectif `suivi.eleve` sur l'élève (`autoriser_eleve`, portée D-022) ; plus de test de rôle | `m0a_d7_carnet_professeur_hors_perimetre_…` |
| **D9** rôles codés en dur | corrigé pour la messagerie et le carnet | `messagerie.eleve` / `suivi.eleve` effectifs (`a_action_quelconque`, `autoriser_eleve`) : boîte, modèles, archivage, classement, ouverture depuis un modèle | `m0a_d9_messagerie_du_personnel_selon_les_droits_effectifs` |
| **D10** RLS | corrigé (permissive) | 17 tables de communication : RLS activée et forcée, politique `ent_rls_etablissement` (migration 480) ; la stricte reste en M17 | `m0a_d10_rls_posee_…` ; inventaire de `tests_rls.rs` (+17) |

**Choix à faire valider** : (1) règle provisoire D2 par **rôle de base** (direction, administratif, CPE ; compta pour les relances) en attendant les participants nommés (M1) : un rôle paramétré D-017 ne donne pas accès aux fils privés ; (2) un fil ouvert par l'élève est lisible de **tout personnel ayant `messagerie.eleve` sur lui** (professeur de ses classes compris), conformément à la ligne M0a (« privé à l'élève et au personnel ») ; (3) les fils existants ouverts par l'établissement passent en audience `responsables` : **l'élève ne les lit plus** (voulu, D3) ; (4) interdiction par `personne_libelle` seul (sans fiche personne) : aucun effet, faute de lien à exclure ; (5) les envois de l'ENT hors messagerie (cahier de textes, EDT, finances, formulaires, inscriptions) lisent toujours `ent_eleve_lien_actif` : même famille de défaut que RS3-06, à traiter avec le socle 8 (N0).

**Reste pour M0b** : D8, D11 à D18, D20 ; l'envoi des carnets et alertes aux seuls parents ayant `peut_voir_vs` (le contenu n'est plus envoyé, seule l'existence d'une entrée l'est) viendra avec les destinataires par nature (socle 8).

### 27.4 État des lots M1 et M2 (implémentés 2026-11-11, branche `ent-socle-16-code-1`, base `ENT` c376da4)

Niveau : **implémenté et testé en local** (PostgreSQL 16, feature `labo`) ; CI GitHub : voir `docs/PROJECT_STATUS.md`, « Messagerie, lots M1 et M2 » ; **non validé par Florian, non prêt pilote**. Migrations **482** (tables et fonctions de participation), **483** (reprise MIG-1 à MIG-4), **484** (sept réglages, historique, majorité). `MIGRATION_TAG` reste la plus haute du répertoire (667) : 482–484 sont dans la plage de repli 480–499.

**M1 — participants et contrôle d'accès (R-M1, R-M2, R-M14)**
| Élément du §27.1 | État | Où |
|---|---|---|
| Fil enrichi (nature, sensibilité, état, origine, ouvreur, jumeau), `ent_msg_fil_eleve`, messages enrichis (type, version, empreinte SHA-256 posée par déclencheur, copie), `ent_msg_version` | **implémenté** | 482 |
| Participants (`ent_msg_participant`) : côté, qualité, raison **codée**, écriture, dates, suspension, motif de sortie codé | **implémenté** | 482 |
| Lecture = participation : la fonction `ent_msg_fil_lisible(établissement, compte)` ; la famille ne lit que par participation (lien communicant, vie scolaire, payeur et remise différée relus à chaque lecture) | **implémenté** | 482, `msg_repo.rs` |
| `peut_lire_fil` / `peut_ecrire_fil` (fil ouvert, participation qui autorise l'écriture) | **implémenté** | `service.rs` |
| Participants posés à la création et à la première écriture ; avis aux seuls participants | **implémenté** | `msg_repo.rs` |
| Reprise MIG-1 à MIG-4, dédoublement MIG-3, rapport à la direction (`ent_msg_reprise_rapport`) | **implémenté** | 483 |
| Garde « aucune lecture hors contrôle » (3 contrôles statiques) | **implémenté** | `garde_messagerie_lecture.rs` |
| Événements `messagerie.fil_ouvert`, `message_envoye`, `participant_ajoute`, `participant_retire`, `fil_dedouble` (sans contenu ni sujet) | **implémenté** (publiés après la transaction du fait, comme les événements existants ; pas encore dans la même transaction) | `service.rs`, `msg_m2.rs` |
| Boîtes de service (participant `boite_id`), supervision motivée | **non fait** : M3 et M10 | — |
| Matrice de séparation « boîtes » | **non fait** (M3) | — |

Écarts assumés au §25 : un participant est une **personne** ou un **élève** (`eleve_id`), car un élève mineur n'a pas toujours de fiche personne ; `boite_id` arrive avec M3. Le **pont des rôles réservés** (D26 : direction, CPE, secrétariat lisent encore sans participer les fils réservés) reste en place, signalé **provisoire jusqu'à M3** : sans lui, la CPE et la direction perdraient la lecture des fils de leurs élèves avant que les boîtes existent. Dès qu'un personnel y écrit, il devient participant. La lecture du personnel n'est donc **pas encore strictement R-M1** ; celle des familles et des élèves l'est.

**M2 — matrice et destinataires (§8, R-M3, R-M5 à R-M8, R-M12)**
| Élément du §27.1 | État | Où |
|---|---|---|
| `peut_ecrire` : tableau §8.2 (11 acteurs × 12 cibles), bornes non paramétrables, refus neutre « destinataire exclu (décision) » sans motif, réponse 15 jours (CM-35) | **implémenté** (logique pure, testée sans base) ; **branché sur la base** pour famille, élève, professeur (périmètre D-022), direction, CPE, secrétariat | `msg_matrice.rs`, `msg_m2.rs` |
| Sept réglages bornés, colonnes typées, historique en ajout seul | **implémenté** | 484 |
| Réglages **appliqués** par le serveur | `pieces_familles` (pièces des familles et des élèves) et `secretariat_vers_majeurs` : **appliqués**. Les cinq autres (familles → professeurs, familles → direction, élèves → professeurs, élèves → élèves, AESH → familles) : **stockés et décidés par la matrice, mais sans effet sur un envoi** tant que les cibles nominatives n'existent pas (boîtes M3, élèves M8) : le fil actuel s'adresse à l'établissement, pas à un professeur nommé | — |
| Fils par foyer (R-M3), **fils jumeaux** (`jumeau_de`) | **implémenté** | `service.rs` |
| Aperçu des destinataires sans coordonnées, exclusions **agrégées** | **implémenté** | `apercu_destinataires_for_actor` |
| Interdiction, « ne pas solliciter » (R-M7 : aucun fil ouvert vers la personne), lien suspendu, relus à chaque lecture, écriture et ouverture | **implémenté** | vues 480, `groupes_communicants` |
| Dédoublement à la séparation (MSG-118, MSG-284, CM-03, CM-04) : le passé reste lisible des deux, l'avenir est séparé | **implémenté** (service `separer_fil_for_actor` ; pas de déclenchement automatique à la création du second foyer : il dépend de `foyer.cree`, socle 4) | 483, `msg_m2.rs` |
| Majorité (R-M12) | **provisoire** : à la majorité, les responsables sortent des fils ouverts à propos de l'élève par l'établissement ; aucun droit conservé du socle 4 n'existe (Q-A7) | 484, `appliquer_majorite_eleve` (à appeler sur `eleve.majeur`) |
| Décès (CM-12), tuteur, ASE (CM-13, CM-24) | **non fait** : aucune donnée de décès dans le modèle | — |
| Ports des socles 5 (fin de cours, CM-35 en base) et 6 (missions, équipes) | **non fait** : adaptateur provisoire sur `autoriser_eleve` ; le délai de 15 jours est dans la matrice pure | — |

**À faire valider (choix faits par défaut, D-031)** : (1) pont des rôles réservés maintenu jusqu'à M3 ; (2) fil de vie scolaire à deux foyers : **un seul fil**, signalé dans le rapport (dédoublement avec le carnet, M7) ; (3) MIG-3 : un fil de l'établissement jamais lu et sans portée est **dédoublé par foyer** (un lecteur seul : l'autre foyer perd le fil, documenté) ; (4) à la séparation, **tout le passé est copié** dans le fil jumeau (les deux parents l'avaient lu) ; (5) la majorité retire les responsables des fils « à propos de lui » ouverts par l'établissement, pas des fils qu'ils ont ouverts ; (6) un responsable « à ne pas solliciter » n'entre dans aucun fil ouvert par l'établissement : si tous les responsables sont dans ce cas, le fil du personnel reste sans destinataire (l'aperçu le montre : « 0 conversation, N exclus ») ; (7) rôle `Personnel` classé « secrétariat » dans la matrice.

### 27.2 Migrations (registre `MIGRATIONS.md`)
Vérifié libre le 2026-11-09 sur **toutes les branches distantes** (plus haute migration trouvée : 460, `ent-socle-evaluations-v0`).
Si Q-B2 est adoptée : **4600** M0a (RLS permissive sur toutes les tables de communication, colonne `audience` provisoire, `prive_user_id` des fils ouverts par un élève, longueurs `NOT VALID`) ; **4601** M0b (index de pagination, réglage de taille) ; **4602** fil enrichi, `fil_eleve`, clés composites ; **4603** participants et reprise (MIG-1 à MIG-4) ; **4604** messages enrichis, versions ; **4605** boîtes, traitement, modèles enrichis ; **4606** lots, envois différés ; **4607** pièces (objet, analyse) ; **4608** carnet (mots, reprise MIG-5) et cartes ; **4609** signalements, gels, restrictions, supervisions, vigilance ; **4610** absences des personnels, paramètres ; **4611** vues filtrées et compatibilité ; 4612–4689 réserve ; **4690–4699** RLS stricte et retrait des anciennes tables et colonnes (`ent_msg_accuse`, `ent_carnet_entree`, `ent_msg_piece.contenu`, `prive_user_id`, `foyer_id`, `archive`, `priorite`, `dossier`) après la durée d'archive.
**Repli** si Q-B2 n'est pas adoptée : **480–499** (480–491 dans le même ordre, 492–497 réserve, 498–499 RLS stricte et retraits). Le lot **M0a peut partir avant toute décision sur Q-B2** : il prend alors **480** (et M0b **481** s'il en a besoin).
**Socle 17** (publications, lot M15) : **4700–4799** si Q-B2, repli **500–519** ; réservé ici pour éviter une collision, à détailler à la conception propre du socle 17.

---

## 28. Tests et recette

1. **Matrice de séparation de la messagerie** (générée, comme `separation.rs`) : pour chaque route de la brique × chaque profil d'attaquant — parent de l'autre foyer, parent interdit, parent suspendu, parent « ne pas solliciter », parent sans `peut_voir_vs`, élève, élève d'une autre classe, professeur hors cours, professeur de l'autre groupe (LV2), AED hors boîte, membre d'une boîte non C3 face à une boîte C3, direction sans supervision ouverte, accueil, autre établissement — sur liste, lecture, compteur, recherche, pièce, URL pré-signée, accusé, export, flux temps réel, notification.
2. **Pièges de minimisation** du §21.3, chacun un test.
3. **Scénarios métier** : CM-01 à CM-80, un test par cas marqué « test ».
4. **Reprise** : jeu de fils de labo couvrant MIG-1 à MIG-5, rapport attendu figé.
5. **IA** : jeux d'injection dans messages, noms de pièces, sujets ; refus des capacités fermées.
6. **Recette par personas** (D-021, étape 4) : familles séparées (Lily et James Potter du labo), parent interdit, élève qui signale, professeur LV2, CPE et AED sur la boîte, infirmière, accueil, direction en supervision, suppléante, élève majeur.

---

## 29. Réglages signalés « usine à gaz » (D-023) — signalés, non supprimés

| # | Réglage | Pourquoi il tente | Recommandation |
|---|---|---|---|
| S-132 | Matrice libre « qui peut écrire à qui » par rôle (Next.js `roles_can_initiate`) | habitude des logiciels | **sept réglages bornés** calculés sur les faits (§8.3) |
| S-133 | Dossiers personnels illimités, couleurs, règles de classement | boîte e-mail classique | archive + étiquettes de boîte (S-135) + recherche |
| S-134 | Signature personnalisée riche (HTML, logo) | personnalisation | signature fixe générée (nom, fonction, établissement), jamais de coordonnée personnelle |
| S-135 | Étiquettes libres en nombre | tri fin | 10 étiquettes au plus par boîte |
| S-136 | Priorité « haute » choisie par l'expéditeur famille | familles pressées | posée par le tri et le personnel seulement |
| S-137 | Présence (« en ligne »), « vu à 23 h 14 », indicateur de saisie | messageries grand public | **refusé** côté familles et élèves ; facultatif entre personnels |
| S-138 | Taille et nombre de pièces par rôle | contrôle fin | une règle : 20 Mo, 5 pièces |
| S-139 | Seuil du lot individuel | — | fixe : 30 |
| S-140 | Durée de réponse d'un ancien professeur | — | fixe : 15 jours |
| S-141 | Images adulte → mineur | — | règle fixe §10.2 point 4 |
| S-142 | Filtre de mots interdits bloquant | sécurité apparente | avertissement doux côté élève seulement, jamais de blocage |
| S-143 | Statistiques de réponse par agent | pilotage | agrégées par boîte ; nominatives pour la direction seulement |
| S-144 | Réactions (émojis) aux messages | Next.js les a | refusées dans les fils famille et élève (ambiguïté, captures d'écran) ; possibles entre personnels |
| S-145 | Accusé de lecture configurable par message par l'expéditeur | contrôle | « lu » automatique + prise de connaissance signée pour ce qui compte |
| S-146 | Délai de correction et de retrait réglables | — | fixes : 15 et 10 minutes |
| S-147 | Modèles de messages illimités | habitude | 50 par établissement, revue annuelle (socle 8 S-59) |
| S-148 | Plages horaires par personne pour l'envoi | finesse | plage calme de l'établissement pour l'envoi (défaut national), plage personnelle pour la réception (socle 8) |

---

## 30. Questions à Florian (avec recommandations, appliquées par défaut — D-031)

| # | Question | Recommandation (appliquée par défaut, D-031, sauf mention) |
|---|---|---|
| Q-M1 | Lancer **M0a** tout de suite, avant toute autre évolution de la communication ? | **Oui** : quatre fuites critiques (D1 à D4) dans le code intégré. |
| Q-M2 | **Participants nommés** : la lecture d'un fil découle de la participation, plus du lien avec l'élève (remplace la règle de RP1-11 « un fil de l'établissement est visible de chaque responsable ») ? | **Oui** (R-M1). |
| Q-M3 | **Parents séparés** : un fil par foyer par défaut, fil commun seulement à l'intérieur d'un même foyer, chacun pouvant demander le sien ? | **Oui** (R-M3, MSG-118). |
| Q-M4 | **Boîtes de service** comme destinataire principal des familles (vie scolaire, secrétariat, direction, infirmerie…), avec prise en charge ? | **Oui**. |
| Q-M5 | Les parents d'un élève mineur lisent-ils sa messagerie ? | **Non** : ni ses fils, ni par un export ; l'élève ne lit pas non plus ceux de ses parents. À faire confirmer par le DPO (autorité parentale contre confidentialité de l'élève, §5). |
| Q-M6 | Messagerie **entre élèves** ? | **Fermée par défaut**, seulement des fils de délégués et de classe animés par un adulte. |
| Q-M7 | **Fils encadrés adulte–mineur** : supervision motivée possible et indicateur de vigilance déterministe, sans lecture du contenu ? | **Oui** ; le référent est désigné par la direction ; DPO à consulter (journalisation, information des personnels). |
| Q-M8 | Ce que voit la famille de la lecture : « lu par l'établissement / par Mme B. », sans l'heure ? | **Oui**. |
| Q-M9 | Réponse d'une boîte : au nom de la boîte avec prénom et initiale de l'agent ? | **Oui** (« Vie scolaire — Paul D. »). |
| Q-M10 | **Durées de conservation** proposées (§15 point 3 : purge 3 mois après la perte du droit au compte, après versement au dossier de ce qui a une valeur administrative ; carnet 1 an ; journaux 12 mois) ? | **Appliquées comme hypothèses de travail** ; **à valider par le DPO et, pour le public, par le service d'archives (visa) avant la mise en service** (engagement extérieur, D-031 : soumis à Florian). |
| Q-M11 | AESH et familles : participer aux fils ouverts par le PP ou le professeur, sans en ouvrir seul ? | **Oui** (réglage borné). |
| Q-M12 | Mention « rédigé avec l'aide de l'assistant » visible du destinataire ? | **Oui par défaut** (transparence, art. 50 du règlement IA à confirmer) ; l'établissement peut la retirer pour les seules reformulations. |
| Q-M13 | IA de tri : déterministe d'abord, modèle de langage ensuite, sans aucune décision ? | **Oui** ; les fonctions à modèle de langage attendent l'étude juridique (comme Q-EV8). |
| Q-M14 | Réponses par e-mail aux avis : rejetées avec renvoi vers ScolIA, ou déposées dans la boîte comme messages non vérifiés ? | **Rejetées par défaut** (risque d'usurpation d'adresse) ; dépôt possible par réglage. |
| Q-M15 | Moteur d'analyse antivirus des pièces (libre, sur nos serveurs) ? | **Choix à instruire** (licence, activité, sécurité — CLAUDE.md) ; en attendant, quarantaine + types restreints + conversion ; **soumis à Florian** (dépendance nouvelle). |
| Q-M16 | Carnet de correspondance : mots signés par un seul titulaire suffisent-ils ? | **Oui**, sauf mot marqué « les deux » par le personnel. |
| Q-M17 | Plage calme : appliquer **exactement** le défaut national (20 h–7 h, vendredi 20 h–lundi 7 h) à tous les messages du personnel vers les familles et les élèves, dérogation d'urgence par la direction seulement ? | **Oui** (exigences 8 et 9 du référentiel du numérique responsable, §5 R-18). |
| Q-M18 | Fils système (absence à justifier, relance de facture, document à signer) : participants calculés par la nature (titulaires avec le droit, payeurs), dédoublés par foyer ? | **Oui**. |
| Q-M19 | Socle 17 : pas de commentaires publics sur une publication, seulement « poser une question » en fil individuel ? | **Oui**. |
| Q-M20 | Plage de migrations 4600–4699 (Q-B2) ou repli 480–499 ; socle 17 4700–4799 ou 500–519 ? | **Réservées** ; M0a part en **480** si Q-B2 n'est pas tranchée. |
| Q-M22 | Parent **sans exercice** de l'autorité parentale : informations dues dans ses propres fils, écriture vers la direction et le secrétariat, pas de messages ordinaires des professeurs (CM-10) ? | **Oui**, ajustable par la direction au vu de la pièce ; aligné sur la brochure DGESCO et TA Toulouse 2022 (§5 R-4, R-5). |
| Q-M21 | Messagerie de l'infirmerie avant l'hébergement de données de santé (HDS) ? | **Boîte infirmerie limitée à la logistique** (rendez-vous, retour à la maison, PAI renvoyé au socle 34) avec avertissement « pas de détail médical » tant que l'HDS n'est pas en place (`VERIF_REGLEMENTAIRE.md` R23) ; **soumis au DPO**. |

---

## 31. Faits, hypothèses, contradictions
**Faits [F]** : §2 (code lu, défauts confirmés, lignes citées) ; schéma Next.js §2.4 (lecture seule).
**Vérifié [V]** : §5 (sources citées).
**Hypothèses [H]** : pratiques de Pronote, d'École Directe et de Charlemagne (§3) ; fréquences ; usage des groupes de messagerie personnels par les enseignants (déjà [H] au socle 8) ; volumes (300 messages par fil, 30 fils par lot).
**Contradictions relevées** :
1. **RP1-11** (`RECETTE_PERSONAS_1.md`) : « un fil ouvert par l'établissement reste visible de chaque responsable de l'élève » — **contredit** par R-M1 et R-M3 ; c'est précisément ce qui permet D1 et D5. La règle de RP1-11 est remplacée (Q-M2).
2. **Test Leptos** `ent-web-server/tests/messagerie.rs:227` valide que « l'élève lit les fils qui le concernent » — contredit R-M9 pour les fils adressés aux responsables ; le test sera réécrit en M0a.
3. **`SOCLES.md` §3, socle 17** range les « campagnes mail / SMS / push avec coût et double confirmation » au 17 ; le socle 8 (Q-N2) et ce document placent la **mécanique** au 8 et le **contenu** au 17 (déjà signalé par le socle 8, §29 point 1).
4. **Socle 8 §15** (« le professeur qui diffuse voit 1 personne exclue (décision) ») contre R-N51 : ce document suit **R-N51** (exclusions agrégées).
5. **`SOCLE2_DOMAINES.md` COM-07** (« message à propos d'un élève visible dans son dossier ») : précisé — visible dans la chronologie **du personnel** seulement, titre neutre pour C3, jamais dans une vue famille.

6. **Plage calme et heures calmes du socle 8** : le socle 8 (§9.3, §16) prévoit des heures calmes réglables (défaut établissement) et une disponibilité des personnels de 7 h 30 à 19 h 00 ; le référentiel du numérique responsable [V] impose un **défaut** de 20 h–7 h et du vendredi 20 h au lundi 7 h pour tout ce que les outils ENT et vie scolaire diffusent. Le socle 8 devra aligner son défaut d'établissement sur ce texte (à signaler à sa passe personas ou à son lot N3).
7. **Socle 8 §16** (« les messages de familles hors horaires : la famille voit "le professeur lira votre message à partir de 7 h 30" ») : conservé ; c'est la disponibilité du personnel, distincte de la plage calme d'envoi.

---

## 32. Les cinq décisions à prendre en priorité

1. **Lancer le lot M0a maintenant** (correctifs des fuites, migration 480 ou 4600) : un parent visé par une interdiction de communication lit les fils de l'établissement et reçoit le carnet ; tout personnel qui suit l'élève lit les fils privés d'un parent et les relances ; l'élève lit les fils adressés à ses parents et ses parents lisent les siens ; le contenu part en clair dans les notifications mobiles, les SMS et les e-mails. **Recommandation : oui, avant tout autre lot du domaine** (appliquée par défaut, D-031).
2. **Participants nommés et un fil par foyer** (Q-M2, Q-M3) : la lecture d'un fil découle de la participation, plus du lien avec l'élève ; un message « aux parents » crée un fil par foyer ; la règle de RP1-11 « un fil de l'établissement est visible de chaque responsable » est remplacée. **Recommandation : oui** — c'est la seule réponse durable à D1, D2, D3, et la doctrine ministérielle demande d'envoyer à chaque parent séparé.
3. **Élèves** (Q-M5, Q-M6, Q-M7) : les parents d'un mineur ne lisent pas sa messagerie ; pas de messagerie libre entre élèves ; fils adulte–mineur encadrés (supervision motivée prévue par la charte, indicateur de vigilance sans lecture du contenu). **Recommandation : oui**, avec un **avis juridique** sur le contrôle du chef d'établissement (C. pén. 432-9) et un passage au DPO **avant tout pilote avec des élèves**.
4. **Temps et conservation** (Q-M17, Q-M10) : plage calme nationale par défaut (20 h–7 h, week-end), dérogation d'urgence par la direction seulement ; aucune suppression par un utilisateur ; purge 3 mois après la perte du droit au compte, après versement au dossier de ce qui a une valeur administrative. **Recommandation : oui pour la plage calme** (texte national) ; **durées appliquées comme hypothèses de travail, à faire valider par le DPO et le service d'archives** avant la mise en service.
5. **Cadre de l'IA** (Q-M12, Q-M13) : aide à la rédaction, résumé, tri, traduction ; chaque envoi relu et confirmé (N3, passkey récente), destinataires imposés par le fil, mention visible et marque machine, aucune décision ni profilage ; tri déterministe d'abord. **Recommandation : oui** ; les fonctions à modèle de langage attendent l'étude juridique déjà demandée au socle 21 (Q-EV8) et le choix du fournisseur (Mistral prioritaire, sous garanties).
