# Socle 17 — Informations, annonces et campagnes

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout nouveau code ; aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Le code Next.js a été **lu seulement**, de façon ciblée (affiches, actualités du site, invitations, salons du personnel, notifications famille), dans le dépôt, sans rien exécuter.
**Branche :** `ent-socle-17-18-conception`, partie de `origin/ent-mc0hoe`.
**Conçu avec le socle 18** (`18_FORMULAIRES.md`), dont il consomme le port « a-t-on le droit ? » (images, productions d'élèves, liste des représentants) et vers lequel il renvoie toute réponse structurée.
**Passe personas :** `PERSONAS_17.md` ; ses ajouts sont faits **ici**, marqués **« ajout personas »** (pour l'essentiel au §35). **En cas de contradiction, le §35 l'emporte** ; rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017 (droits portés par les actions), D-022 (portée du professeur au groupe), D-023 (modèles prêts, peu de réglages), D-024, D-026 (briques), D-028 (Scaleway TEM, « ne pas solliciter »), D-031 (recommandations appliquées par défaut), D-034 (feuille de route, doublon 2027-2028), D-035 (add-on Next.js), D-036 (aucun JSON-mémoire), D-038 (E1–E14) ; `docs/SOCLES.md` §17 ; `16_MESSAGERIE.md` §17, §19.11, §33.4, §33.6 (esquisse du socle 17, ANN-100 à ANN-121) ; `MESSAGERIE_M0B.md` (D8, D18, D27 : annonces à aperçu, confirmation, retrait) ; `08_NOTIFICATIONS.md` §13 (mécanique des diffusions, Q-N2), §14 (urgence), §15 (personnes à protéger) ; `04_ADMINISTRATIF.md` §8.6 (droits du lien), §8.12 (destinataires) ; `07_DOCUMENTS.md` §10 (niveaux S0–S2) ; `IA_COEUR.md` §5–§6 ; `PRA_SAUVEGARDES.md` §18.4 ; `TRANSITION_T0_INVENTAIRE.md` (M17, M20, P06). Modèles de forme : `42_SORTIES_VOYAGES.md`, `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`.

**Conventions.** **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins depuis la racine) ; **[V]** règle lue sur une source officielle (URL et date au §4) ; **[NV]** règle non vérifiée ; **[H]** hypothèse (terrain, marché) ; **[R]** recommandation, non décidée.

**Numérotation propre au socle** (préfixes vérifiés libres dans `docs/` le 2026-11-11) :

| Élément | Numéros | Remarque |
|---|---|---|
| Défauts de l'existant Rust | **PD-01**… | |
| Défauts Next.js (à ne pas reproduire) | **PN-01**… | Next.js reste intouchable |
| Actions | **ANN-100 à ANN-121** (reprises de `16_MESSAGERIE.md`), **ANN-122 à ANN-259** | |
| Règles | **R-PB1**… | |
| Cas limites | **CP-01**… | |
| Questions | **Q-PB1**… | |
| Réglages signalés (D-023) | **S17-01**… | préfixe par socle, pour éviter toute collision avec les sessions parallèles |
| Manques de la passe personas | **M17-001**… | dans `PERSONAS_17.md` |

**Plage de migrations imposée : 2340–2359** (consigne de lancement du 2026-11-11). Elle remplace les réservations antérieures du socle 17 (4700–4799 si Q-B2, repli 500–519), qui sont **abandonnées** : `500` est déjà pris par `500_objet_registre` [F]. Registre : `MIGRATIONS.md`.

---

## 0. En bref

1. **Ce qu'est une publication.** Un **acte éditorial de l'établissement**, signé par un auteur, à **audience décrite par des critères** (jamais une liste tapée), résolue **sous les droits de l'auteur** et par les règles du socle 4, affichée là où les gens regardent (fil d'actualité, accueil de l'enfant, écran du hall), et **notifiée seulement si c'est utile** par une diffusion du socle 8. On sait **qui l'a vue**, on relance **les seuls non-lecteurs**, et toute **réponse** structurée passe par le socle 18 (formulaire), le socle 19 (créneau), le socle 7 (document) ou le socle 25 (paiement) — jamais par des commentaires publics.
2. **Huit natures**, une seule mécanique : annonce, actualité, lettre d'information, **campagne à réponse attendue**, note de service aux personnels, publication des représentants de parents, information d'un partenaire, avis d'organisation (fermeture, horaires : « information due »). L'urgence reste au socle 8 §14.
3. **L'existant est un bon départ, mais il fuit à six endroits** (§2, PD-01 à PD-17). Le lot M0b a déjà rendu les annonces à deux temps (aperçu, confirmation, retrait) et neutres hors de l'ENT. Restent notamment : **une route qui publie sans aperçu** (`confirmer:true`, PD-01, confirmé par lecture), un **compte élève qui lit les annonces destinées aux parents** (PD-02), un **parent sous interdiction qui lit les annonces de classe** (PD-03), des **brouillons lisibles par tout le personnel** (PD-04), une annonce **sans portée** (un CPE de 6ᵉ écrit à tout le collège, PD-05), un aperçu **jamais périmé** (PD-06). → **Lot AN0**, avant la démo de décembre.
4. **Next.js** n'a pas de module d'annonces aux familles ; il a des **affiches** (générées en PDF, texte suggéré par une IA externe), des **actualités du site public**, des **pages d'invitation** à un événement (avec réponse) et un **salon « Général » du personnel**. On reprend les fonctions, pas le code (D-035), en corrigeant leurs défauts (§2.4, PN-01 à PN-12, dont une recherche publique qui **révèle l'e-mail d'un parent** à partir du nom et de la date de naissance d'un élève).
5. **Sobriété d'abord** (« faire mieux ») : budget d'attention par famille et par semaine, résumé hebdomadaire au lieu de dix courriels, notification décidée par la nature (pas par l'humeur de l'auteur), aucune heure de lecture conservée, et une règle simple : **publier n'est pas notifier**.
6. **Droit à l'image et productions d'élèves vérifiés avant publication** (port du socle 18) : toute image où un élève est reconnaissable exige un accord valide **pour cet usage** ; un retrait d'accord **masque automatiquement** l'image dans les publications concernées (contrôle de cohérence A15). Pas de reconnaissance faciale.
7. **Familles séparées, interdictions, élève majeur, « ne pas solliciter »** : chacun reçoit en son nom ; l'interdiction de communication l'emporte ; l'élève majeur est destinataire principal ; une personne « à ne pas solliciter » voit le fil mais n'est jamais notifiée, sauf **information due** décidée par la direction (D-028).
8. **IA** : propose un texte, une traduction, un résumé hebdomadaire, une version FALC, une liste de relance, un texte alternatif d'image ; **ne publie, ne notifie, ne valide rien** (niveau N3 du cœur IA, geste humain et passkey).
9. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 160 | ANN-100 à ANN-259 |
   | Règles | 52 | R-PB1 à R-PB52 |
   | Cas limites | 70 | CP-01 à CP-70 |
   | Réglages signalés | 14 | S17-01 à S17-14 |
   | Lots | 14 | AN0 à AN13 |

   **Parcours minimal** pour la démo de décembre 2026 et la V1 de janvier–février 2027 (D-034) : **AN0, AN1, AN2, AN3** ; pour l'année de doublon (en **usage réel**, Charlemagne n'ayant pas d'équivalent moderne [H]) : + AN4, AN5, AN6, AN9.
10. **Ajout personas (`PERSONAS_17.md`, §35 ci-dessous)** : trois bloquants tranchés — **titres « diffusables »** dans le résumé e-mail (sinon compte seulement), **mode « miroir »** pendant l'année de doublon (aucune double notification), **avis d'organisation « pour demain »** dans la fenêtre veille / matin ; 58 manques, 11 actions (ANN-260 à ANN-270), 15 règles (R-PB53 à R-PB67).
11. **Décisions appliquées par défaut (D-031)** : §33.1 (24 points). **Soumis à Florian** : §33.2 (écran du hall dans les espaces publics, partenaires commerciaux, fournisseur de traduction, rubrique pastorale du privé, arrêt des écrans Next.js).

---

## 1. Thèse : informer moins, mieux, et savoir qui sait

Le problème réel des familles n'est pas le manque d'information, c'est son **désordre** : dix canaux (cahier de liaison, courriels du professeur principal, SMS de la vie scolaire, papier dans le cartable, ENT, réseaux de l'association, groupe de parents), des messages en double, l'information importante noyée dans l'anecdotique, et le parent séparé qui « n'est jamais au courant ». Côté établissement, le problème est symétrique : **on ne sait pas qui a lu**, on renvoie à tout le monde « pour être sûr », et l'on fatigue ceux qui lisent pour atteindre ceux qui ne lisent pas.

D'où quatre principes :
1. **Un seul lieu** où l'établissement publie (le fil d'actualité, décliné par enfant), un seul circuit (rédaction → validation selon l'audience → publication → diffusion éventuelle → suivi), une seule trace.
2. **L'audience est un fait calculé**, pas une liste : « les familles des 4ᵉ qui partent en Espagne », « les personnels de vie scolaire du site B », « les parents de la 2ᵈᵉ 4 qui ont accepté de recevoir les comptes rendus des délégués ». Elle se recalcule quand l'élève arrive, part, change de classe ou de responsable.
3. **Publier n'est pas notifier.** Une publication s'affiche ; une notification dérange. La nature de la publication (et non l'auteur) dit si l'on dérange, par quel canal, avec quel plafond.
4. **Savoir qui sait.** Le suivi de lecture est la valeur ajoutée (« on sait qui n'a pas lu, et on le relance seul », `SOCLES.md` §17), mais il est **minimisé** : une date sans heure, pas de suivi nominatif des personnels hors note de service à prise de connaissance, agrégats pour qui n'est pas l'auteur.

---

## 2. Évaluation franche de l'existant

Audit en lecture seule (sous-agent, ~40 lectures) sur la branche de rédaction ; les défauts PD-01, PD-07 (version) et la lecture par lien actif ont été **relus ligne à ligne** par le concepteur (`crates/ent-api/src/routes.rs:5574-5584`, `crates/ent-db/src/formulaires.rs:55-69, 170-195, 560-592`).

### 2.1 Ce qu'il y a [F]

| Élément | Où | Ce que ça fait |
|---|---|---|
| Annonces | `migrations/086_annonce.sql`, `481_msg_m0b_securite_fiabilite.sql:187-229`, `annonce_repo.rs`, `messagerie_m0b.rs:365-672` | `ent_annonce` (titre, corps, audience `tous / parents / personnels / classe`, `cible_classe` texte, `classe_id`) ; depuis M0b : `statut` `apercu / publiee / retiree`, aperçu chiffré (nombre d'élèves, de responsables), confirmation, retrait motivé qui annule les envois en attente ; destinataires par la vue `ent_eleve_lien_sollicitable` (interdiction et « ne pas solliciter » appliqués) ; avis dédoublonné par responsable |
| Campagnes | `191_campagnes.sql`, `campagnes.rs:161-720` | brouillon → aperçu à **empreinte** (contenu + destinataires) → lancement refusé si l'empreinte change ; verrou `FOR UPDATE` ; double confirmation SMS ; plafonds ; portée de l'auteur (`eleves_autorises`) ; audiences `etablissement / niveau / classe / groupe` ; `cible_ids UUID[]` |
| Notifications | `190_notifications.sql`, `notifications.rs:443-445, 855-885` | e-mail, SMS et push **neutres** pour annonces, campagnes et messagerie famille ; `droit_requis` relu à l'envoi |
| Routes | `routes.rs:482-486`, `5561-5694` ; `communication_s28.rs:46-77` | `GET/POST /annonces`, `/{id}/confirmer`, `/{id}/retirer`, `GET /eleves/{id}/annonces` ; `/campagnes` (CRUD, aperçu, lancer, annuler, bilan) |
| Droits | `ent-auth/src/actions.rs:206-215, 755-800, 1303-1308` ; `roles.rs:315` | `annonces.publier` (portée établissement seule), `campagnes.lancer` (portées partielles acceptées), `notifications.gerer` |
| Écrans | `ui.rs` ~53147-53235 (page annonces : aperçu puis confirmer) ; Leptos : module `campagnes/`, tuile de lecture des annonces dans `accueil/mod.rs:48, 240` | aucune page Leptos de rédaction d'annonce ; aucun écran mobile |
| Tests | `tests_messagerie_m0b.rs:1310-1456`, `tests_messagerie_m0a.rs:303-319, 798-806`, `tests_communication.rs:537-745`, `ent-api/src/lib.rs:1630`, `ent-web-server/tests/communication_s28.rs` | aperçu, retrait, D18, interdiction, isolation entre établissements |
| RLS | `480_msg_m0a_fuites.sql:83-110` | **permissive** sur `ent_annonce`, `ent_campagne*` ; aucun accès par `TenantTx` dans ces modules |

### 2.2 Défauts trouvés (à corriger avant toute extension)

**C** = confirmé par lecture ; **S** = supposé, à rejouer.

| # | Gravité | Défaut | Preuve | Scénario | Lot |
|---|---|---|---|---|---|
| **PD-01** | **élevée** | **`confirmer:true` publie sans aperçu.** Le corps de `POST /annonces` accepte `confirmer`, qui appelle directement `publier_annonce_for_actor` ; le commentaire de `messagerie_m0b.rs:452-453` affirme le contraire | `routes.rs:5574-5584` ; `dto.rs:3782-3785` (C, relu) | Un script, un client mobile ou une IA mal bornée envoie `confirmer:true` : l'annonce part à 900 familles sans aperçu ni nombre — exactement ce que D8 devait supprimer (CLAUDE.md : envois soumis à confirmation humaine) | AN0 |
| **PD-02** | **élevée** | **Le compte élève lit les annonces destinées aux parents** | `list_annonces_famille_for_actor`, `service.rs:13491-13509` : aucune distinction parent / élève (C) | « Réunion d'information sur le conseil de discipline de vendredi » ou « rappel : impayés de cantine » lus par les élèves de la classe | AN0 |
| **PD-03** | **élevée** | **Lecture par lien actif, pas par lien communicant** : le parent visé par une interdiction de communication lit les annonces de la classe (il n'est pas notifié, mais il lit) | `service.rs:13491-13509` (`lien_parent_eleve`) ; vue `ent_eleve_lien_communicant` existante (`480:15-39`) (C pour le chemin, S pour le cas complet) | Père sous ordonnance de protection : il lit la date, l'heure et le lieu du retour de voyage de la classe de sa fille | AN0 |
| **PD-04** | élevée | **Brouillons et aperçus lisibles par tout le personnel** | `list_annonces_for_actor`, `service.rs:13476-13487` (C) | L'agent d'entretien lit le brouillon d'une annonce de fermeture non décidée | AN0 |
| **PD-05** | élevée | **Annonce sans portée** : `annonces.publier` n'accepte que la portée établissement ; ni niveau ni groupe (D-022) | `actions.rs:~790-800` (C) | Un CPE de 6ᵉ écrit à tout le collège ; un professeur ne peut jamais écrire à son groupe d'allemand | AN0 (portée), AN1 (critères) |
| **PD-06** | moyenne | **Aperçu d'annonce jamais périmé** : ni empreinte, ni durée de vie ; un aperçu à **zéro** destinataire est confirmable | `messagerie_m0b.rs:399-450, 638` (C) | Aperçu de septembre confirmé en février : d'autres familles qu'annoncé | AN0 |
| PD-07 | moyenne | **Élève majeur ignoré** : ses parents reçoivent, lui non | `campagnes.rs:246-299` ; `messagerie_m0b.rs:637-672` (C pour l'absence de filtre, S pour la règle) | Étudiant de BTS de 20 ans : ses parents reçoivent les annonces de sa classe, lui rien | AN0 |
| PD-08 | moyenne | **Pas de retrait d'une campagne lancée** (seulement brouillon ou aperçu) | `campagnes.rs:479+` (C) | Campagne erronée : impossible d'arrêter les SMS encore en file | AN0 |
| PD-09 | moyenne | **Journal incomplet** : retrait d'annonce et lancement de campagne = événement seulement, pas de journal interne (acteur, motif) | `messagerie_m0b.rs:467-511` ; `campagnes.rs` (C) | Après une plainte, on ne sait plus qui a retiré une annonce et pourquoi | AN0 |
| PD-10 | moyenne | **Cibles en `UUID[]` sans clé étrangère** ; classe supprimée = cible fantôme | `191:13` (C) | Campagne programmée vers une classe fusionnée en cours d'année : zéro destinataire, sans alerte | AN1 |
| PD-11 | moyenne | **`ON DELETE CASCADE` depuis `ent_user`** sur `ent_campagne_destinataire` et `ent_notification_envoi` : la suppression d'un compte efface la **preuve d'envoi** | `191:43`, `190:42` (C) | Litige « je n'ai jamais été prévenu » après fermeture du compte : plus de preuve | AN0 (RESTRICT), AN13 |
| PD-12 | moyenne | **Classe par libellé en repli**, sensible à la casse, sans alerte si `classe_id` reste nul | `annonce_repo.rs:132-133` ; `messagerie_m0b.rs:379-380, 601-610` (C) | « 4E2 » au lieu de « 4e2 » : annonce à personne | AN0 |
| PD-13 | faible | Longueurs comptées en **octets** (`annonce.rs:133-136`), aucun `CHECK` SQL sur `ent_annonce` | (C) | Titre accentué refusé à 180 caractères | AN0 |
| PD-14 | faible | **Aucune expiration** : une annonce reste visible indéfiniment (dans la limite de 100) ; **aucun correctif** possible après envoi | (C) | « Sortie annulée demain » encore en tête du fil trois mois plus tard | AN2 |
| PD-15 | faible | Même personne crée et lance une campagne d'établissement : **pas de quatre yeux** | `campagnes.rs:~598-601` (C) | Erreur d'un seul geste pour 1 300 familles | AN2 |
| PD-16 | faible | Corps de campagne recopié **en clair par destinataire** dans `ent_notification` et `ent_notification_envoi`, sans purge identifiée | (S) | Volume et conservation non maîtrisés | AN13 (avec socle 8) |
| PD-17 | à vérifier | RLS inopérante (aucun `TenantTx`), alors que **toutes** les requêtes lues filtrent bien `etablissement_id` (aucune fuite entre établissements trouvée) | (C) | Une requête future qui oublie le filtre n'est arrêtée par rien | AN13 (strict), AN1 (`TenantTx` dans le nouveau code) |

### 2.3 Verdict
- **Garder** : le cycle aperçu → empreinte → confirmation des campagnes (il devient la mécanique du socle 8, Q-N2) ; le retrait motivé qui annule les envois en attente ; les rendus neutres hors de l'ENT ; les vues `ent_eleve_lien_sollicitable` / `_communicant` ; le filtre `etablissement_id` systématique ; les tests M0a / M0b.
- **Corriger tout de suite** (lot **AN0**) : PD-01 à PD-09, PD-11 à PD-13.
- **Remplacer** : `ent_annonce` et le *contenu* d'`ent_campagne` par les **publications** (§27), avec reprise et double écriture le temps d'une version (§28) ; la mécanique d'envoi des campagnes devient la **diffusion** du socle 8 (lot N9).

### 2.4 Ce que Next.js fait, et qu'il ne faut pas perdre (lecture seule, D-035)
Inventaire en lecture seule (sous-agent, ~90 lectures). **Aucun module d'annonces aux familles** [F] : les familles ne reçoivent que des notifications de messagerie (`famille_notif`, aperçu de 280 caractères en clair, `app/lib/famille-messaging-db.ts:279-300`).

| Fonction Next.js [F] | Reprise dans ScolIA [R] |
|---|---|
| **Affiches** (`app/api/posters/**`) : modèles, formats, palette, identité de l'établissement ; brouillons ; rendu PDF ; logos de partenaires et images de fond ; **suggestion de titre et de texte par un modèle de langage externe** (`suggest-copy/route.ts:51-63`) | **ANN-200 à ANN-205** : affiche rendue côté serveur **depuis une publication** (3 ou 4 trames, D-023), QR code vers la publication, PDF ; images par le socle 7 (types vérifiés, SVG refusé, désarmement) ; suggestion de texte par l'IA du cœur (fournisseur souverain à l'étude, jamais une clé dans le frontend) |
| **Actualités du site public** (`app/api/site-cms/**`, `public/site/posts`) : billets, statut, date de publication, image de couverture par URL libre ; lecture publique sans authentification | **Hors socle 17** (nouveau socle N3 « présence publique », `SOCLES.md` §10 T-17) ; le 17 prépare seulement l'action « proposer au site public » (ANN-206), désactivée tant que N3 n'existe pas ; aucune actualité n'est publique par défaut (R-PB30) |
| **Pages d'invitation à un événement** (`app/api/invitation/**`) : page par lien, liste d'éligibles importée (texte, tableur), réponse et nombre d'accompagnants, doublons, tableau de bord, export, saisie par le personnel, clôture | Événement et réponse : **socle 19** (T-16) ; le 17 porte la **campagne d'invitation** (ANN-150 à ANN-157) qui renvoie au créneau ou au formulaire de réponse ; **aucune page publique par simple adresse lisible** ; les invités sans compte reçoivent un **lien nominatif** (socle 8 §10.5) |
| **Salon « Général » du personnel** (`app/api/channels/**`) | **Note de service** et publication interne (ANN-120, ANN-170 à ANN-179) ; les échanges restent à la messagerie (16) |

**Défauts Next.js à ne pas reproduire** (signalés à Florian ; Next.js reste intouchable) :

| # | Défaut | Preuve [F] |
|---|---|---|
| **PN-01** | **Recherche publique d'une réponse d'invitation par nom, prénom et date de naissance**, sans limite de débit : renvoie l'**e-mail du parent**, le diplôme et la situation de l'élève | `app/api/invitation/lookup/route.ts:12-55` ; `app/lib/invitation-db.ts:716-749` |
| **PN-02** | Toute personne qui connaît nom et date de naissance **remplace la réponse d'une autre famille** (et son e-mail) quand la liste d'éligibles n'est pas exigée (défaut) | `invitation-db.ts:915-942, 976-990` ; `db/schema-invitation.ts:45` |
| **PN-03** | Salons du personnel : **aucun contrôle de rôle ni d'appartenance** ; tout compte authentifié lit tous les messages, modifie ou supprime ceux des autres ; annuaire des e-mails du personnel ouvert | `app/api/channels/**` ; `channels/users/list/route.ts:5-21` |
| PN-04 | Messages anonymes dans les salons, sans trace | `channels/messages/route.ts:94` |
| PN-05 | Pièces des salons : type déclaré par le client, sans taille maximale, **adresse publique non signée** | `channels/upload/route.ts:18-23` |
| PN-06 | Affiches : clés d'image arbitraires, signature de **n'importe quel objet du stockage** | `app/lib/posters/storage.ts:169-177` ; `render-pdf.ts:228-252` |
| PN-07 | Affiches : SVG accepté, type contrôlé sur la déclaration du client | `posters/upload/route.ts:30-49` |
| PN-08 | Texte d'affiche (et nom du partenaire) envoyé à un fournisseur externe de modèle de langage, sans garantie contractuelle vérifiée | `posters/suggest-copy/route.ts:38-63` |
| PN-09 | Actualités du site : corps et image non assainis (risque d'injection de script selon le rendu) | `site-cms/posts/route.ts:43-45` |
| PN-10 | Suppression incomplète (brouillons d'affiche, billets du site) | `posters/storage.ts:150-156` ; `site-cms/storage.ts:36-56` |
| PN-11 | Export des réponses d'invitation : **injection de formule** possible (`=`, `+`, `-`, `@` non neutralisés) dans un champ libre saisi par les familles | `app/lib/invitation-export.ts:12-18` |
| PN-12 | Invitations : aucune trace de l'auteur d'une réponse, suppressions définitives, aucune purge des données de mineurs (date de naissance, e-mail, situation) | `db/schema-invitation.ts:87-125` |

### 2.5 Ce que font les logiciels du marché [H, de mémoire, non vérifié]
Pronote (« Informations et sondages » : information avec accusé de lecture, sondage à questions, destinataires par classe ou groupe, casier) ; École Directe (actualités, sondages, formulaires) ; Charlemagne (publipostage, envois de courriels et SMS groupés, courriers) ; ENT des collectivités (actualités, blogs, « pages » de classe). Ce qui manque partout [H] : le **suivi des non-lecteurs par famille** (et non par compte), la **séparation des foyers**, la **sobriété** (on additionne les canaux), la **vérification du droit à l'image** avant publication, la **campagne qui sait ce qu'elle attend**.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il doit faire | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Direction** (chef d'établissement, adjoint) | informer de l'organisation (rentrée, examens, fermetures, grèves, travaux), valider les publications larges, porter la parole de l'établissement | relire dix versions, recopier dans trois outils, répondre à « je n'ai pas été prévenu » | l'annonce erronée à 1 300 familles ; la photo d'un enfant protégé ; une publication d'association hors des limites légales | voir d'un coup d'œil « qui ne sait pas » et relancer en un geste ; un circuit de validation qui ne bloque que ce qui doit l'être |
| **Secrétariat** | mettre en forme, traduire, imprimer pour les familles sans numérique, envoyer par courrier aux parents séparés | publipostage manuel, enveloppes, « est-ce que le père l'a reçu ? » | oublier un parent séparé ; envoyer à la mauvaise adresse | une version papier nominative en un clic, uniquement pour ceux qui en ont besoin |
| **CPE et vie scolaire** | rappels (assiduité, carnet, règlement), information de niveau, campagnes « document à rendre » | relancer à la main les mêmes familles ; savoir qui a rendu quoi | toucher des familles hors de son niveau ; contredire la direction | une campagne qui relance seule les non-répondants dans la plage calme |
| **Professeur principal** | informer sa classe (réunion, conseil, sortie, compte rendu) | courriels dispersés, adresses périmées, parents qui ne lisent pas | écrire à un parent interdit ; se tromper de classe | un fil de classe propre, avec « lu par 24 familles sur 27 » |
| **Professeur** | informer son groupe (matériel, sortie pédagogique, travaux exposés) | aucun moyen d'écrire à un groupe sans passer par la vie scolaire | voir des données qu'il ne devrait pas voir | publier à son groupe en trente secondes, avec photo si l'accord existe |
| **Accueil** | relayer une information d'organisation affichée dans le hall ; répondre au téléphone « c'est quand la réunion ? » | chercher l'information | donner une information non publiée | l'écran du hall et le fil à jour, la même source |
| **Personnels (tous)** | recevoir les notes de service, consignes de sécurité, convocations internes | salon interne bavard, consignes perdues | ne pas avoir vu une consigne de sécurité | une note de service avec prise de connaissance, et le reste au calme |
| **AESH, assistants d'éducation, intervenants** | être informés de l'organisation (fermeture, exercice) | ne pas avoir de compte ou n'avoir que celui de la DSDEN [H] | arriver un jour de fermeture | être dans l'audience « personnels » sans démarche |
| **Familles** (dont **séparées**, **allophones**, **sans numérique**, **avec handicap**) | savoir ce qui concerne leur enfant, à temps | trop de messages, mélange de l'important et de l'anecdotique, informations reçues par l'autre parent seulement | rater une date ; que l'autre parent voie leurs coordonnées | un fil par enfant, un résumé hebdomadaire, leur langue, une version simple, rien en double |
| **Élèves mineurs** | connaître les informations qui les concernent (sorties, clubs, examens, CDI) | informations qui passent par les parents seulement | lire une information qui ne leur est pas destinée (et l'inverse) | un fil élève lisible sur téléphone |
| **Élèves majeurs** | recevoir à leur nom ce qui les concerne | parents encore destinataires « par défaut » | que leurs parents reçoivent sans leur accord | être traités en adultes (D23 du lot M0b) |
| **Représentants de parents, associations** (APEL, FCPE, PEEP…) | diffuser comptes rendus, convocations d'assemblée, informations | ne pas avoir les adresses (et ne pas devoir les avoir) | être censurés ; être soupçonnés de démarchage | diffuser par l'établissement à ceux qui l'acceptent, sans jamais voir les adresses |
| **DPO** | registre, minimisation, conservation, droits des personnes | suivi de lecture sans limite, photos sans base | une image publiée sans accord ; un suivi de lecture des personnels assimilable à une surveillance | durées par nature, aucune heure de lecture, contrôle automatique des accords |
| **Partenaires** (mairie, transporteur, entreprise, association sportive) | faire passer une information utile | — | — | une information validée par la direction, marquée « partenaire » |

---

## 4. Ce qui est réglementaire

Recherche du 2026-11-11 (sous-agent, ~86 lectures) sur Légifrance, EUR-Lex, cnil.fr, eduscol (PDF), assemblee-nationale.fr ; le BO et les pages HTML d'eduscol renvoient 403 (PDF lus localement). Détail commun aux socles 17 et 18 : `18_FORMULAIRES.md` §4 (tableau des sources). Ce qui commande le socle 17 :

| # | Règle | Source | Statut |
|---|---|---|---|
| RG-1 | **Les deux parents reçoivent les mêmes informations** : « les établissements scolaires doivent entretenir avec les deux parents, quelle que soit leur situation familiale, des relations de même nature, leur faire parvenir les mêmes documents, convocations, etc. » | guide DGESCO « L'exercice de l'autorité parentale en milieu scolaire » (2011), https://eduscol.education.gouv.fr/sites/default/files/document/autoriteparentale170278pdf-68094.pdf ; C. civ. 373-2 (la séparation est sans incidence sur l'autorité parentale), https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049294125 ; 373-2-1 (le parent qui n'exerce pas l'autorité « doit être informé des choix importants »), https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000022469781 | [V] (guide antérieur à 2024) |
| RG-2 | **Information régulière des parents** ; l'établissement « s'assure » qu'ils en prennent connaissance ; deux réunions par an et par classe au moins | C. éduc. D111-2, D111-3, https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006151393/ | [V] (version de 2019 lue) |
| RG-3 | **Associations de parents** : le chef d'établissement doit leur « permettre […] de faire connaître leur action » ; documents diffusés **sans contrôle a priori**, clairement identifiés, sous leur responsabilité, dans le respect de la laïcité, de la vie privée, sans injure ni diffamation, sans propagande politique ou commerciale ; modalités en concertation ; désaccord porté à l'autorité académique, qui se prononce sous **7 jours**, à défaut diffusion comme prévu ; comptes rendus des représentants diffusés de même (D111-15) | C. éduc. **D111-9**, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006525723 ; D111-13, D111-15 | [V] |
| RG-4 | Les associations n'obtiennent la **liste des parents** que si chacun a **expressément accepté** | C. éduc. **D111-8** | [V] (déjà au socle 4, VERIF R5) |
| RG-5 | **Neutralité commerciale** : pratiques publicitaires interdites dans les établissements publics ; partenariat par convention, à caractère pédagogique, marque « discrète » | circulaire n° 2001-053 du 28-3-2001 (code de bonne conduite), confirmée en vigueur par une réponse ministérielle (13ᵉ législature, QE 30260), https://www.assemblee-nationale.fr/dyn/13/questions/QANR5L13QE30260.pdf ; CNIL 2026 : « neutralité commerciale », pas de traceur publicitaire, https://www.cnil.fr/fr/enseignement-premier-second-degres-bonnes-pratiques-outils-collaboratifs | [V] réponse ministérielle ; statut 2026 de la circulaire [NV] |
| RG-6 | **Base légale** de la communication établissement-familles par l'ENT : **mission d'intérêt public** (service public du numérique éducatif), pas le consentement ; information claire (RGPD art. 12-14), adaptée aux mineurs (langage clair, FALC), à la première connexion et à chaque rentrée ; AIPD « probable » ; s'applique au public **et au privé** | CNIL, « Enseignement : bonnes pratiques pour les outils collaboratifs », 24-8-2026, URL ci-dessus | [V] |
| RG-7 | **Droit à l'image** : les parents le protègent « en commun » et y associent l'enfant selon son âge et sa maturité ; la CNIL demande le consentement écrit des représentants légaux pour toute utilisation de photos d'élèves (journal, trombinoscope, site, « tout autre support ») | C. civ. **372-1** (loi n° 2024-120), https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006426505/ ; CNIL, https://cnil.fr/fr/cnil-direct/question/mon-enfant-ete-pris-en-photo-lecole-puis-je-mopposer-lutilisation-de-son-image | [V] ; qualification usuel / non usuel **non tranchée** (18 §4) |
| RG-8 | **Productions d'élèves** : l'élève est auteur dès la création ; toute reproduction ou représentation sans son consentement est illicite | CPI L111-1, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042814694 ; L122-4, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006278911 | [V] ; droit moral (L121-1, nom de l'auteur) [NV] |
| RG-9 | **Accessibilité** : RGAA 4.1.2 (version 5 annoncée fin 2026) pour les personnes morales de droit public (un EPLE l'est) et les délégataires de service public ; déclaration d'accessibilité ; le privé sous contrat est **incertain** | https://accessibilite.numerique.gouv.fr/obligations/champ-application/ ; loi n° 2005-102 art. 47 [NV] | [V] page RGAA |
| RG-10 | **Familles allophones** : document d'information traduit dans la mesure du possible (CASNAV) ; aucune obligation trouvée pour un ENT | circulaire n° 2012-141 (403, non lue) | [NV] |
| RG-11 | **Élections des représentants de parents** : le vote électronique est encadré (système audité, séparation listes / votes, secret) : **une publication ou un sondage ne vaut jamais scrutin** | arrêté du 2-7-2024, https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000049871171 | [V] (établissements publics) |
| RG-12 | **Contrôle de l'activité des personnels** : un suivi de lecture nominatif des personnels doit être proportionné et annoncé | [NV, principe général du droit du travail et du RGPD] | Q-PB4 (DPO) |

**Conséquences retenues** : RG-1 → chaque titulaire reçoit en son nom, aucune information « au foyer » (R-PB8) ; RG-2 → suivi de lecture et relance (§11), signal de famille non touchée ; RG-3 → publication des représentants **sans validation de fond**, suspension motivée seulement et renvoi à l'autorité académique (§16) ; RG-4 → audience « ayant accepté » par consentement du socle 18 ; RG-5 → partenaires validés et marqués, offres commerciales refusées par défaut ; RG-6 → pas de consentement demandé pour recevoir l'information ; mentions d'information à la première connexion ; RG-7 et RG-8 → contrôle d'usage par le socle 18 (§13) ; RG-9 → texte alternatif, aucun contenu seulement dans une image, trames accessibles ; RG-11 → « consultation, sans valeur de scrutin » ; RG-12 → pas de suivi nominatif des personnels hors prise de connaissance.

**Privé sous contrat** : RG-3 (D111-9) vise les établissements publics [H] ; ScolIA applique **par défaut** les mêmes règles à l'APEL et aux représentants dans le privé (bonne pratique, comme le socle 42 pour les sorties), réglable (S17-12).

## 5. Frontières

### 5.1 Qui possède quoi

| Objet | Possédé par | Le socle 17 |
|---|---|---|
| Publications (contenu, versions, audience décrite, validation, période d'affichage, suivi de lecture, attente de réponse) | **17** | possède |
| Mécanique d'envoi groupé : aperçu des destinataires, coût, confirmation N3, quatre yeux, délai d'annulation, exécution, correctif, relance | **8** (Q-N2, `08_NOTIFICATIONS.md` §13) | appelle le service de diffusion ; reçoit les accusés |
| Urgence et crise (confinement, évacuation, fermeture de dernière minute) | **8 §14** et **14** | lien « publier aussi au fil » après l'alerte (ANN-129) |
| Destinataires d'un élève (titulaires, droits du lien, foyers, interdictions, « ne pas solliciter », majorité) | **4** (`destinataires`, `droits_lien`, vues `ent_eleve_lien_communicant` / `_sollicitable`) | consomme |
| Classes, niveaux, groupes, cours, sites, régimes | **5** ; portée du professeur (D-022) : **1 / 5** | consomme (`eleves_visibles`, `cours_de`) |
| Personnels, missions, équipes | **6** | consomme |
| Formulaires, autorisations, consentements, droit à l'image, liste des représentants | **18** | consomme `verifier`, déclare ses usages (`declarer_usage`) |
| Documents joints, gabarits PDF, lots d'impression, publipostage | **7** | consomme (carte document, rendu, lot) |
| Événements, réunions, créneaux | **19** | une campagne renvoie à un créneau |
| Paiement d'une participation | **24 / 25** | une campagne renvoie à un paiement |
| Conversations (« poser une question ») | **16** | ouvre un fil (ANN-111) |
| Écran conversationnel de l'assistant | **30** | expose ses capacités au registre `ent-ia` |
| Site public | **N3** (à créer) | prépare seulement « proposer au site » |
| Menus de cantine, calendrier annuel | **40**, **19** | les affiche dans le fil par port de lecture, sans les posséder |

### 5.2 Règles de frontière [R]
1. Le 17 **n'appelle jamais un fournisseur d'envoi** : il demande une diffusion au socle 8 (R-PB1).
2. Le 17 **ne résout jamais lui-même** les responsables d'un élève : il demande au socle 4 (R-PB2).
3. Le 17 **ne stocke aucune réponse structurée** : coupon, choix, autorisation = formulaire du 18 ; il ne garde que le lien et l'état « répondu / non répondu » lu par port (R-PB3).
4. Le 17 **ne décide pas** du droit à l'image : il demande au 18, avant publication **et** à chaque affichage d'une image (R-PB4).

### 5.3 Ports exposés par le socle 17 [R]
| Port | Consommateurs | Effet |
|---|---|---|
| `publications::fil(personne, contexte, date) -> Vec<Resume>` | 29 (portail « Aujourd'hui »), 11 (mobile), 30 (assistant) | publications visibles à cette date, par enfant |
| `publications::publier_depuis(socle, modele, audience, texte)` | 42 (circulaire de voyage), 19 (convocation de réunion), 12 (rappel d'assiduité de niveau), 46 (professions de foi), 40 (changement de menu exceptionnel) | crée un **brouillon** au nom de l'utilisateur (jamais publié par un autre socle sans geste humain) |
| `publications::suivi(publication) -> Bilan` | 47 (pilotage : taux de lecture par niveau) | agrégats seulement |
| `publications::images_de(personne) -> Vec<Usage>` | 18 (cohérence), 27 (droit d'accès) | où apparaît l'image d'une personne |

### 5.4 Ports consommés [R]
`personnes::destinataires(eleve, nature, date)` et `droits_lien` (4) ; `structure::eleves_de(critere, date)`, `cours_de(personnel)` (5) ; `personnels::de(mission, equipe, site)` (6) ; `autorisations::verifier(sujets, objet, usage, date)` et `declarer_usage` (18) ; `notifications::diffuser(…)`, `::correctif`, `::relancer` (8) ; `documents::carte`, `::rendre`, `::lot_impression` (7) ; `messagerie::ouvrir_fil` (16) ; `agenda::creneau` (19) ; `paiements::demande` (25).

---

## 6. Concepts et vocabulaire

### 6.1 Vocabulaire fixé [R]
| Mot | Sens | Table |
|---|---|---|
| **Publication** | acte éditorial d'une nature donnée, avec auteur, audience, versions | `ent_pub_publication` |
| **Version** | texte figé d'une publication (titre, chapeau, corps), avec empreinte ; toute correction après publication est une **nouvelle version** | `ent_pub_version` |
| **Audience** | critères d'inclusion et d'exclusion, et **public** visé (responsables, élèves, personnels) | `ent_pub_audience_critere` |
| **Destinataire** | personne qui entre dans l'audience à une date, **pour un élève** (famille) ou pour elle-même (personnel, élève) | `ent_pub_destinataire` |
| **Affichage** | présence dans le fil, sur une période | colonnes `affiche_du`, `affiche_au` |
| **Diffusion** | envoi d'avis par le socle 8 pour une publication | `ent_pub_diffusion` (lien) |
| **Lecture** | la publication a été ouverte (date sans heure) | `ent_pub_lecture.lu_le` |
| **Prise de connaissance** | geste explicite « j'ai pris connaissance » (S0 du socle 7), horodaté | `ent_pub_lecture.pris_connaissance_le` |
| **Attente** | ce qu'une campagne attend de chaque cible (formulaire, document, créneau, paiement, prise de connaissance), avec échéance | `ent_pub_attente` |
| **Information due** | information d'organisation que chacun doit recevoir (fermeture, horaire, sécurité) ; soumise à la décision de la direction pour une personne « à ne pas solliciter » (D-028) | `information_due` |
| **Budget d'attention** | nombre de publications **notifiées** reçues par une famille sur sept jours glissants | calculé |

### 6.2 Quatre distinctions qui manquent aujourd'hui [F → R]
1. **Publier ≠ notifier** (aujourd'hui une annonce confirmée envoie toujours).
2. **Lire ≠ prendre connaissance** (aujourd'hui aucun suivi de lecture des annonces).
3. **Famille ≠ compte** : un élève est « touché » quand **au moins un** titulaire l'a lue, « pleinement » quand **chaque** titulaire destinataire l'a lue ; les deux chiffres sont montrés (R-PB18).
4. **Audience pour les parents ≠ pour les élèves** (aujourd'hui l'élève lit les annonces « parents », PD-02).

---

## 7. Les natures de publication

| Nature | Pour quoi | Auteurs par défaut | Validation par défaut | Notification par défaut | Durée d'affichage par défaut |
|---|---|---|---|---|---|
| **Annonce** | information courte et datée (réunion, rappel, changement) | direction, CPE, secrétariat, PP (sa classe), professeur (ses cours) | aucune sous 200 personnes ; quatre yeux au-delà du seuil (R-PB10) | in-app ; résumé hebdomadaire ; push si « importante » | 30 jours, ou jusqu'à la date de l'événement + 1 jour |
| **Actualité** | vie de l'établissement, souvent avec images | idem + documentaliste, référents (projets) | direction si images ou audience établissement | aucune (fil et résumé hebdomadaire) | 90 jours |
| **Lettre d'information** | assemblage périodique d'actualités et d'annonces | direction, secrétariat | direction | e-mail (diffusion) + in-app | jusqu'à la lettre suivante |
| **Campagne** | information + **réponse attendue** (§12) | direction, CPE, secrétariat, PP, organisateur de sortie | direction si établissement ; aucune sinon | in-app + push ; relances aux seuls non-répondants | jusqu'à l'échéance + 7 jours |
| **Avis d'organisation** (« information due ») | fermeture prévue, horaires modifiés, grève, examens, travaux, exercice de sécurité annoncé | direction (ou délégation) | quatre yeux si établissement | push + e-mail ; **information due** | jusqu'à la date + 1 jour |
| **Note de service** | consignes et informations aux personnels, avec ou sans prise de connaissance | direction, gestion, CPE (vie scolaire) | aucune | in-app ; push si prise de connaissance requise | 1 an |
| **Publication des représentants** | compte rendu de conseil, convocation d'assemblée, information d'une association de parents | représentants élus, bureau de l'association | **aucune validation de fond** ; contrôle des limites légales seulement (§16) | in-app ; résumé | 60 jours |
| **Information d'un partenaire** | mairie, transporteur, association sportive, entreprise, collectivité | saisie par la direction ou le secrétariat **au nom** du partenaire | direction (toujours) | aucune | 30 jours |

Règles communes : une nature déclare son modèle (3 ou 4 trames, D-023), ses canaux par défaut, son plafond, son droit d'auteur (D-017), sa durée de conservation. L'établissement **ne crée pas** de nature : il crée des **rubriques** (liste courte : « Vie de l'établissement », « Pédagogie », « Vie scolaire », « Sorties », « Restauration », « Orientation », « Associations »… ; S17-03).

---

## 8. Audiences

### 8.1 Critères [R]
Une audience est une **union de critères d'inclusion**, moins des **critères d'exclusion**, appliquée à un **public** : `responsables`, `eleves`, `responsables_et_eleves`, `personnels`.

| Critère | Résolu par | Remarques |
|---|---|---|
| établissement entier, site | 5 | |
| niveau, classe, cours (classe entière ou groupe, D-022), groupe | 5 | par **identifiant**, jamais par libellé (corrige PD-12) |
| régime (demi-pensionnaire, interne, externe), ligne de transport | 5, 40, 41, 43 | |
| inscrits de la rentrée prochaine (admis, réinscrits) | 3 | familles de futurs élèves ; aucun accès à l'historique de l'établissement |
| participants d'une sortie, d'une activité, d'un club | 42, 43 | port du socle propriétaire |
| liste d'élèves choisis (au plus 200) | 2 | l'auteur ne choisit que des élèves qu'il voit |
| personnels par mission, rôle, équipe, site, discipline | 6 | |
| instance (conseil d'administration, conseil pédagogique, CVL…) | 46 | |
| « parents de la classe ayant accepté la liste des représentants » | 18 | consentement exprès (§16) |

**Critères interdits** (R-PB6) : tout critère qui révèle une donnée sensible ou réservée — boursier, aide sociale, PAI, santé, handicap, dispositif (ULIS, SEGPA, UPE2A), suivi social, sanction, impayé, signalement. Une information propre à ces publics passe par le **socle propriétaire** (avis individuel du 8, ou message du 16), jamais par une publication à audience nommée. Exception : un critère de **dispositif** est permis à la seule direction et aux personnels du dispositif, pour une audience **personnels**.

### 8.2 Résolution [R]
1. **Sous les droits de l'auteur** : un professeur ne vise que ses cours (D-022) ; un PP que sa classe ; un CPE que son périmètre (niveau, site) ; la portée de l'action `publications.rediger` borne les critères permis (corrige PD-05).
2. **Par le socle 4** pour chaque élève de l'audience : titulaires et autres responsables ayant le droit du lien `recevoir_informations` **à la date** ; lien **communicant** (interdiction appliquée, corrige PD-03) ; foyers séparés : chacun reçoit **en son nom** ; élève majeur : destinataire principal, parents seulement selon la règle de majorité du socle 2/4 (corrige PD-07, R-PB8).
3. **Personnels** : comptes actifs à la date (départ, suspension exclus) ; AESH et assistants d'éducation compris s'ils ont un compte (R-PB9).
4. **Dédoublonnage** : une personne reçoit **une** publication, même pour trois enfants ; la publication porte la liste des enfants concernés (« pour Léa et Hugo »).
5. **Audience vivante** pour l'affichage (un élève arrivé après la publication la voit si elle est encore affichée et le concerne) ; **audience figée** pour le suivi et la diffusion (le dénominateur est l'audience à la diffusion, **plus** les arrivées, marquées « entrée après »). Un départ retire l'accès au fil de la classe à la date du départ (R-PB7).

### 8.3 Aperçu de l'audience [R]
Avant toute publication : nombre d'élèves, de foyers, de personnes ; **exclus** (fondus avec les non-joignables sous le seuil de 3, R-N51 du socle 8 : jamais « 1 personne exclue (décision) » dans une classe de 25) ; personnes sans canal (et ce qui sera fait : papier, liste d'appel) ; langues présentes ; trois exemplaires du rendu. L'aperçu porte une **empreinte** (audience + texte + date) et **expire** au bout de 24 h ou au premier changement d'audience (corrige PD-06). Zéro destinataire : publication refusée.

---

## 9. Cycle de vie

### 9.1 États [R]
`brouillon` → (`soumise` → `validee` | `renvoyee`) → `programmee` → `publiee` → `expiree` → `archivee` ; `retiree` depuis `programmee` ou `publiee` (motif obligatoire). Une publication **corrigée** reste `publiee` avec une nouvelle version.

### 9.2 Validation [R]
- Déclenchée par la **nature** et l'**audience** (seuil par défaut : établissement entier, ou plus de 200 destinataires, ou nature « partenaire » ; S17-01). En dessous : publication directe par l'auteur.
- **Quatre yeux** : le valideur est une autre personne (compte, personne D-020 et passkey distincts) ; il voit l'aperçu exact ; il valide, renvoie avec commentaire, ou modifie (ce qui crée une version dont il devient co-auteur, et la renvoie à l'auteur pour accord). Délégation possible par rubrique (« la CPE valide la vie scolaire », R-PB11).
- **Absence du valideur** : file visible, suppléance par `ent_delegation` (socle 1) ; jamais de validation automatique au bout d'un délai.

### 9.3 Programmer, épingler, expirer [R]
- Programmation à une date et une heure ; à l'heure dite, **aperçu recalculé** ; si l'audience a varié de plus de 5 % ou si un critère est devenu vide, la publication attend une reconfirmation (comme NTF-51).
- Épingle : au plus 2 publications épinglées par public et par enfant ; fin d'épingle obligatoire (au plus 30 jours).
- Expiration à la date d'affichage ; l'expiration retire du fil, pas de l'historique de la personne (« publications reçues »).

### 9.4 Corriger, retirer [R]
- **Correction** après publication : nouvelle version, motif, choix « correction de forme » (rien n'est renvoyé) ou « correction de fond » (correctif notifié **aux seuls lecteurs et notifiés**, NTF-54) ; l'ancienne version reste lisible dans l'historique, barrée.
- **Retrait** : motif obligatoire ; disparition du fil ; diffusions en attente annulées ; si la publication avait été notifiée, un avis de retrait neutre (« Une information du 12 mars a été retirée ») part aux notifiés, sauf retrait « pour erreur d'audience » où il ne part **qu'**à l'auteur et à la direction (ne pas attirer l'attention de ceux qui n'auraient pas dû la recevoir, R-PB14).
- **Aucune suppression** par l'interface ; purge selon la conservation (§30).

---

## 10. Affichage, notification et sobriété

### 10.1 Où l'on voit [R]
- **Fil d'actualité** de la personne : web et mobile, filtrable par enfant, par rubrique, « non lues ».
- **Carte de l'enfant** (fiche famille, portail 29) : les publications qui le concernent.
- **Accueil du personnel** : notes de service, publications internes, publications de ses classes.
- **Écran du hall** (§18) : seulement les publications marquées `affichage_ecran` et de classe C0.

### 10.2 Quand on dérange [R]
| Importance | Canal par défaut | Plafond |
|---|---|---|
| normale | aucun avis ; résumé hebdomadaire | — |
| importante | push (ou e-mail sans application) | budget d'attention |
| information due | push + e-mail ; SMS si choisi et budgété | hors budget, mais **lint** : l'auteur justifie |
| urgence | **socle 8 §14** (pas une publication) | — |

### 10.3 Budget d'attention [R, différenciant]
- Calcul : nombre de publications **notifiées** reçues par une personne sur 7 jours glissants, tous auteurs confondus.
- Seuil par défaut : 4 (S17-04). Au-delà, l'aperçu prévient l'auteur (« 312 familles de 4ᵉ ont déjà reçu 5 avis cette semaine ; cette annonce sera mise dans le résumé de vendredi sauf si vous la marquez importante ») ; « importante » au-delà du seuil exige une justification enregistrée.
- Pilotage (47) : moyenne et distribution par niveau, par mois ; la direction voit quels auteurs consomment le budget.

### 10.4 Résumé hebdomadaire [R]
Chaque vendredi à 17 h (S17-05, fuseau de l'établissement) : les publications non lues de la semaine, par enfant, dans la langue de la personne, par e-mail (ou push qui ouvre le résumé). Pas de résumé vide. Désactivable par la personne (préférence du socle 8), **sauf** pour les informations dues non lues, qui restent signalées.

### 10.5 « Ne pas solliciter », interdictions [R]
- « Ne pas solliciter » (D-028) : la personne **voit** le fil si elle se connecte ; elle n'est **jamais** notifiée d'une publication ni d'une relance ; pour une **information due**, la décision de la direction sur la personne s'applique (courrier, coffre, suspension) ; à défaut, tâche bloquante pour la direction.
- Interdiction de communication : ni lecture ni notification (lien communicant) ; l'auteur voit des exclusions **fondues** (R-PB13).

---

## 11. Suivi de lecture, prise de connaissance, relances

### 11.1 Ce qui est enregistré [R, minimisation]
- **Lecture** : la publication a été **ouverte** (détail affiché), par cette personne, **à cette date** (`lu_le DATE`, sans heure ; cohérent avec `16_MESSAGERIE.md` §33.5). Une simple présence dans le fil n'est pas une lecture.
- **Prise de connaissance** : geste explicite, horodaté, avec empreinte de la version (S0 du socle 7) ; seulement quand l'auteur l'a demandée.
- **Rien d'autre** : ni durée de lecture, ni appareil, ni nombre d'ouvertures, ni lieu.

### 11.2 Qui voit quoi [R]
| Lecteur du suivi | Familles et élèves | Personnels |
|---|---|---|
| Auteur, valideur, direction | par élève et par classe : touché (≥ 1 titulaire), pleinement (tous), non touché ; **noms des non-lecteurs** de l'audience (l'auteur connaît déjà son audience) | **agrégats seulement** ; nominatif **uniquement** pour une note de service à prise de connaissance (R-PB21) |
| PP | idem pour sa classe | — |
| Autres personnels | rien | rien |
| Pilotage (47) | taux agrégés par niveau, sans nom | taux agrégés |

Le suivi nominatif des personnels hors prise de connaissance n'est pas proposé : il serait assimilable à un contrôle de l'activité des salariés, qui doit être proportionné et annoncé [NV, à confirmer par le DPO, Q-PB4].

### 11.3 Relancer [R]
- **Relance des seuls non-lecteurs** (ANN-105) : nouvelle diffusion liée, aperçu restreint, confirmation N3 ; au plus 2 relances par publication (S17-06) ; jamais d'une personne « à ne pas solliciter » ; dans la plage calme.
- **Proposition de canal alternatif** pour les non-touchés après deux relances : version papier remise à l'élève (ANN-114), courrier (adresse du parent séparé), appel par la vie scolaire (liste d'appel, sans enregistrement du contenu de l'appel).
- **Famille non touchée de façon répétée** : signal au PP et au secrétariat (« 3 informations dues non lues ce mois-ci ») — indicateur de décrochage familial, jamais transmis à un tiers.

---

## 12. Campagnes à réponse attendue

### 12.1 Principe [R]
Une campagne = une publication + **une attente** par cible. L'attente est d'un type :
| Type | Réponse portée par | État lu par le 17 |
|---|---|---|
| prise de connaissance | 17 (S0, socle 7) | pris / non pris |
| formulaire (coupon, choix, autorisation, consentement) | **18** | répondu / non répondu / état d'autorisation (sans le contenu) |
| document à rendre (pièce, attestation) | **7** (demande de pièce) | fourni / validé / refusé |
| créneau (réunion, rencontre parents-professeurs) | **19** | inscrit / non inscrit |
| paiement (participation) | **25** | payé / partiel / non payé (jamais le montant pour un non habilité) |

Une campagne peut porter **plusieurs attentes** (ex. sortie : autorisation + paiement + document), chacune avec son échéance.

### 12.2 Cible et unité [R]
La cible est l'**élève** (une famille répond pour un enfant), la **personne** (sondage des parents, inscription d'un personnel) ou le **foyer**. Le 17 n'invente pas l'unité : c'est celle du formulaire du 18 ou de la demande du 7.

### 12.3 Suivi et relance [R]
- Tableau par cible : état de chaque attente, date ; filtres « rien fait », « en partie », « à jour » ; par classe.
- Relance **des seuls non-répondants** à J-7, J-3, J-1 de l'échéance par défaut (S17-07), dans la plage calme, par le socle 8 ; jamais à une personne « à ne pas solliciter » ni à un parent sous interdiction.
- **Papier** : coupon imprimable avec code de la cible ; saisie au guichet par le secrétariat (le 18 enregistre « papier, saisi par X le … »).
- Clôture à l'échéance ; prolongation tracée.
- **Exports** : liste « à jour / pas à jour » par classe, **sans contenu des réponses** (le contenu est exporté par le 18, sous ses droits).

### 12.4 Invitations à un événement [R, reprend PN-01, PN-02]
Remise de diplômes, portes ouvertes internes, cérémonie, spectacle : campagne d'invitation à audience (ou liste d'éligibles importée par le secrétariat), réponse « je viens / je ne viens pas / nombre d'accompagnants » par un formulaire du 18 ou un créneau du 19 ; pour les invités **sans compte** (anciens élèves, familles d'anciens élèves), **lien nominatif** à usage limité (socle 8 §10.5), jamais une page publique où l'on « retrouve sa réponse » avec un nom et une date de naissance.

---

## 13. Images, productions d'élèves et droit à l'image

### 13.1 Règles [R]
1. **Toute image** jointe à une publication passe par le socle 7 (types image bornés, SVG refusé, ré-encodage, **métadonnées retirées** dont la géolocalisation) et porte un **texte alternatif** obligatoire (R-M61 / ANN-119).
2. L'auteur déclare les **personnes reconnaissables** : « aucune » (paysage, dos, mains, objets — attestation de l'auteur) ou une **liste** de personnes (élèves, personnels, parents) choisies dans l'audience visible de l'auteur. **Aucune reconnaissance faciale** ni aide automatique à l'identification (donnée biométrique, RGPD art. 9) (R-PB25).
3. Pour chaque personne listée, le 17 demande au 18 `verifier(personne, droit_image, usage, date)` où l'usage dépend de l'audience : `fil_classe` (audience classe ou cours), `fil_etablissement`, `lettre_information`, `affiche_imprimee`, `ecran_hall`, `site_public` (N3), `presse`. Verdict ≠ `autorise` → image **bloquée** pour cette publication ; l'auteur peut recadrer ou flouter (outil manuel de floutage, côté serveur) puis redéclarer.
4. Le 17 **déclare l'usage** au 18 (`declarer_usage`) à la publication ; il le **retire** à l'expiration ou au retrait.
5. **Retrait d'accord** (événement `autorisations.retire` du 18) : les images des publications **en cours d'affichage** concernées sont **masquées immédiatement** (vignette neutre « image retirée »), l'auteur est prévenu ; les archives ne sont plus servies qu'aux personnels habilités (preuve) ; aucune nouvelle diffusion (R-PB26).
6. **Productions d'élèves** (dessin, texte, photo prise par l'élève, vidéo) : l'élève est **auteur** ; publication soumise à l'accord `publication_productions` (élève **et** titulaires pour un mineur) pour l'usage visé ; le nom de l'élève n'apparaît que si l'accord l'autorise (« prénom seul », « prénom et initiale », « anonyme ») (R-PB27).
7. **Personnels** : leur image relève de leur propre accord (socle 18, sujet personnel) ; même mécanique.
8. **Élève protégé** (confidentialité renforcée, changement de classe pour protection, adresse protégée d'un parent) : son image **n'est jamais publiable**, quel que soit l'accord (le 18 renvoie `refuse:protection`, sans motif) (R-PB28).

### 13.2 Ce que la famille voit [R]
« Votre enfant apparaît dans 3 publications » (ANN-188), avec lien vers chacune et vers ses accords ; la personne majeure voit les siennes. Retirer l'accord se fait depuis cet écran en un geste (le 18).

---

## 14. Lettre d'information, affiches, papier

### 14.1 Lettre d'information [R]
Assemblage d'actualités et d'annonces de la période (proposé par l'IA ou par sélection manuelle), dans une trame prête (3 ou 4, D-023), avec éditorial facultatif ; version e-mail (HTML sémantique + texte), version in-app, version PDF imprimable ; même audience et même suivi qu'une publication ; une image n'y figure que si l'usage `lettre_information` est autorisé pour chaque personne reconnaissable.

### 14.2 Affiches [R, parité Next.js]
Rendu serveur d'une affiche (A4, A3, format écran) **à partir d'une publication** ou d'un texte libre, dans une trame ; logo de partenaire possible (image du socle 7) ; QR code vers la publication (qui exige d'être connecté : l'affiche du hall ne publie rien à l'extérieur) ; PDF ; historique des affiches générées. Suggestion de texte par l'IA du cœur (N0).

### 14.3 Papier et courrier [R]
- **Familles sans numérique** (préférence « papier » du socle 4) : lot d'impression nominatif par classe, remis à l'élève par le PP avec émargement facultatif (« remis le »), ou par courrier.
- **Parent séparé à une autre adresse** sans compte : courrier pour les **informations dues** seulement (coût affiché dans l'aperçu).
- Publipostage et rendu : socle 7 ; aucune adresse protégée n'est imprimée (« adresse protégée, voir direction »).

---

## 15. Langues, accessibilité, simplicité

- **Langue de la personne** (socle 4) : traduction **proposée** par l'IA (fournisseur souverain, D-024), marquée « traduction automatique relue par … », **relue** par un humain avant publication ; le français est toujours joint dessous ; aucune traduction automatique non relue pour une information due (R-PB32).
- **FALC** (facile à lire et à comprendre) : version proposée par l'IA, relue, pour les natures à fort enjeu (avis d'organisation, campagnes) ; la famille choisit « version simple » dans ses préférences.
- **Accessibilité** : texte alternatif obligatoire ; aucun contenu porteur d'information **seulement** dans une image ou un PDF scanné (un PDF joint exige un résumé texte) ; contraste et structure des titres dans les trames ; lecture vocale possible (balisage) ; référentiel RGAA (§4).
- **Lisibilité** : titre ≤ 120 caractères, chapeau ≤ 300, corps ≤ 8 000 ; repère de lisibilité affiché à l'auteur (phrases longues, jargon : « conseil de classe du T2 » → « conseil de classe du 2ᵉ trimestre ») (S17-08).

---

## 16. Représentants de parents, associations, partenaires, pastorale

### 16.1 Représentants et associations de parents [R ; règles au §4]
- Les représentants élus d'une classe et le bureau d'une association reconnue ont l'action `publications.representants.rediger`, bornée à leur classe ou à l'établissement selon leur mandat (socle 46).
- **Audiences** : (a) « tous les parents » pour les **documents d'association** que l'établissement relaie (le relais par l'établissement ne donne aucune liste à l'association) ; (b) « parents de la classe ayant accepté » pour les comptes rendus et échanges des représentants (consentement exprès du socle 18, refusé par défaut, révocable).
- **Pas de validation de fond** : la direction ne corrige pas un compte rendu ; elle peut **suspendre** une publication pour un motif des limites légales (injure, diffamation, caractère commercial, atteinte à la vie privée d'un élève nommé) avec motif, renvoi à l'autorité compétente, et information des auteurs (ANN-118) — détail des limites au §4.
- Les représentants **ne voient jamais** les adresses ni la liste nominative des parents ; ils voient le nombre de destinataires et un suivi agrégé.
- Période électorale (socle 46) : professions de foi relayées à l'identique pour toutes les listes, à la même date (R-PB36).

### 16.2 Partenaires [R]
Information d'un partenaire (mairie, transporteur, collectivité, club) saisie par la direction ou le secrétariat, marquée « Information de [partenaire] », **validée par la direction** ; aucun lien de suivi externe, aucune donnée transmise au partenaire ; offres commerciales : refusées par défaut (neutralité, §4), sauf avantage collectif décidé par la direction (Q-PB2).

### 16.3 Caractère propre (privé sous contrat) [R]
Rubrique « Pastorale » ou « Vie spirituelle » activable dans le privé ; audience « familles ayant choisi » une activité facultative si l'information porte sur une participation (pas d'audience déduite d'une appartenance religieuse : donnée sensible) ; désactivée dans le public (Q-PB3).

---

## 17. Personnels : notes de service et publications internes

- **Note de service** : audience personnels (mission, équipe, site, rôle), prise de connaissance demandée ou non, échéance de prise de connaissance, relance des seuls non-signataires, **liste nominative** des prises de connaissance visible de l'auteur et de la direction (preuve de diffusion d'une consigne de sécurité, d'un protocole, d'un calendrier d'examen).
- **Publication interne** (remplace le salon « Général » de Next.js, ANN-120) : informations de la vie de l'équipe (pot de départ, covoiturage, travaux) ; **pas de suivi nominatif** ; réponses par la messagerie (16).
- **Personnels absents ou en congé** : notes de service visibles à leur retour, marquées « publiée pendant votre absence » ; jamais de notification pendant un congé (socle 6) sauf information due de sécurité (R-PB38).
- **Personnels partis** : plus d'accès ; leurs prises de connaissance restent (preuve).

---

## 18. Écran d'affichage du hall [R, lot AN12]

- Écran déclaré par la direction (site, emplacement, appareil) ; **jeton d'appareil** stocké haché, révocable, limité à la lecture d'un **flux d'affichage** ; aucune session d'utilisateur.
- Contenu : publications marquées `affichage_ecran` (C0 seulement : aucune donnée personnelle, aucune image d'élève sans usage `ecran_hall` autorisé), informations du jour fournies par port (menus du 40, absences et remplacements de cours du 15 **sous forme « cours de 4ᵉ2 de 10 h annulé »** sans nom de personnel par défaut — S17-10, Q-PB5 —, horaires d'ouverture, alerte en cours du 8 §14).
- Hors des heures d'ouverture : écran neutre.

---

## 19. Cas limites (catalogue)

### 19.1 Familles et élèves
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CP-01 | Parents séparés, deux foyers, deux comptes | chacun reçoit en son nom ; suivi « touché / pleinement » ; aucun ne voit que l'autre a lu |
| CP-02 | Second parent sans compte, sans e-mail vérifié | information due : courrier (coût affiché) ; autres publications : rien, mais l'élève est compté « partiellement touché » |
| CP-03 | Parent visé par une interdiction de communication | ni lecture ni avis ; exclusions fondues dans l'aperçu (R-PB13) |
| CP-04 | Parent « à ne pas solliciter » | voit le fil, jamais notifié ; information due selon la décision de la direction (D-028) |
| CP-05 | Parent sans autorité parentale, avec `recevoir_informations` | reçoit (RG-1, droit d'information) |
| CP-06 | Beau-parent, grand-parent avec compte et droit `recevoir_informations` accordé | reçoit seulement si le droit est accordé sur son lien |
| CP-07 | Personne de confiance (nourrice) | ne reçoit que les avis d'organisation non médicaux si la catégorie est cochée (R-N60) |
| CP-08 | Fratrie de trois enfants dans trois classes | une seule publication « pour Léa, Hugo et Inès » ; une lecture vaut pour les trois |
| CP-09 | Fratrie dans deux établissements ScolIA | deux fils distincts (D-013), présentés côte à côte, jamais fusionnés |
| CP-10 | Élève arrivé le 10 mars, publication du 2 mars encore affichée | il la voit (audience vivante), comptée « entrée après » dans le suivi |
| CP-11 | Élève parti le 15 mars | plus d'accès au fil de sa classe à cette date ; ses prises de connaissance restent |
| CP-12 | Élève qui change de classe en cours d'année | voit les publications de sa nouvelle classe à partir de la date ; garde l'historique de ce qui lui a été adressé |
| CP-13 | Élève qui devient majeur le 4 février | destinataire principal dès ce jour ; parents selon sa décision (socles 2/4) ; aucune rétroactivité |
| CP-14 | Élève majeur sous tutelle | le tuteur reçoit selon son lien (socle 4) |
| CP-15 | Élève placé (ASE), famille d'accueil | destinataires du lien « service gardien » et de l'accueillant ; jamais d'avis à l'adresse des parents pour une nature réservée par le juge [NV] |
| CP-16 | Famille allophone (arabe) | traduction relue + français ; version FALC si choisie |
| CP-17 | Parent malvoyant | e-mail texte, lecture vocale, aucun contenu seulement dans une image |
| CP-18 | Famille sans numérique | lot papier nominatif remis à l'élève ; émargement facultatif |
| CP-19 | Élève protégé (changement de classe pour protection) | jamais d'image ; aucune publication ne cite son ancienne ou nouvelle classe à son sujet |
| CP-20 | Décès d'un élève | gel des avis vers sa famille (socle 8) ; retrait de ses images des publications en cours ; aucune relance |
| CP-21 | Décès d'un responsable | retiré des destinataires ; aucune relance |
| CP-22 | Élève en internat à l'étranger (échange) | reste destinataire de sa classe d'origine sauf exclusion par l'auteur |

### 19.2 Auteurs, validation, audiences
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CP-23 | Professeur qui vise « toute la 4ᵉ2 » alors qu'il n'a que le groupe d'allemand | critère refusé ; proposition « votre groupe d'allemand de 4ᵉ2 » (D-022) |
| CP-24 | PP qui publie à sa classe et à un groupe qu'il n'enseigne pas | groupe refusé |
| CP-25 | CPE de niveau qui vise l'établissement | refusé sauf droit de portée établissement |
| CP-26 | Critère sensible demandé (« familles boursières ») | refusé ; renvoi au socle 26 (avis individuel) (R-PB6) |
| CP-27 | Audience vide (classe fermée, critère périmé) | publication refusée ; programmée : bloquée avec alerte |
| CP-28 | Valideur = auteur | refusé (quatre yeux) |
| CP-29 | Valideur absent une semaine | suppléant par délégation ; sinon la file reste visible, jamais de validation automatique |
| CP-30 | Publication programmée le samedi 9 h | partira dans la plage calme ? non : la plage calme ne concerne que les avis ; l'affichage se fait à 9 h, l'avis lundi 7 h (sauf information due) |
| CP-31 | Programmation, puis 12 % de l'audience change avant l'heure | attente d'une reconfirmation |
| CP-32 | Deux auteurs publient la même information le même jour | l'aperçu signale une publication semblable dans les 48 h (même audience, titre proche) |
| CP-33 | Auteur qui quitte l'établissement | ses publications restent, attribuées à « l'établissement (ancien auteur) » ; les corrections passent à la direction |
| CP-34 | Erreur d'audience détectée après publication | retrait « pour erreur d'audience » sans avis aux destinataires (R-PB14), publication correcte recréée |
| CP-35 | Correction d'une date de réunion | correctif notifié aux seuls lecteurs et notifiés (correction de fond) |
| CP-36 | Retrait d'une annonce déjà lue par 400 familles | avis de retrait neutre aux notifiés |

### 19.3 Contenu, images, représentants
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CP-37 | Photo de classe de 28 élèves, 2 refus pour le fil | image bloquée ; l'auteur floute deux visages (outil serveur) ou choisit une autre photo |
| CP-38 | Photo où l'auteur déclare « aucune personne reconnaissable » alors qu'on reconnaît un élève | signalement par la famille (ANN-189) → masquage immédiat, examen par la direction |
| CP-39 | Retrait d'accord d'image en mai | images masquées dans les publications en cours ; auteur prévenu ; archives servies aux seuls habilités |
| CP-40 | Accord accordé par un parent, refusé par l'autre | désaccord → non autorisé (18) |
| CP-41 | Dessin d'élève publié dans la lettre | accord `publication_productions` pour `lettre_information` ; prénom seul si l'accord le dit |
| CP-42 | Photo d'un personnel | accord du personnel (sujet adulte) |
| CP-43 | Photo prise par un parent lors d'une sortie et transmise | refusée comme pièce de publication si l'auteur ne peut pas déclarer les personnes ; le parent n'est pas un auteur de publication |
| CP-44 | Vidéo d'un spectacle | même règle que l'image ; usage `video` |
| CP-45 | Publication qui nomme un élève (« félicitations à Léa ») | lint : nom d'élève dans une publication à audience large → confirmation et accord `mention_nominative` [R, Q-PB6] |
| CP-46 | Corps contenant un numéro de téléphone personnel | lint de minimisation (socle 8) : avertissement |
| CP-47 | Compte rendu de conseil des représentants qui cite un élève | suspension possible (vie privée) ; motif ; renvoi à l'autorité ; les représentants sont prévenus |
| CP-48 | Association qui veut joindre un bulletin d'adhésion avec paiement | lien externe affiché « site de l'association » ; aucun paiement dans ScolIA pour un tiers |
| CP-49 | Information d'un partenaire commercial (soldes d'un magasin de fournitures) | refusée par défaut (RG-5) |
| CP-50 | Liste « Pastorale » dans un établissement public | rubrique indisponible |
| CP-51 | Profession de foi d'une liste aux élections | relais à l'identique, à la même date, pour toutes les listes (R-PB36) |
| CP-52 | Lien externe dans une publication | affiché avec son domaine ; avertissement à l'ouverture ; aucun suivi |

### 19.4 Diffusion, suivi, sobriété
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CP-53 | Sixième publication notifiée de la semaine pour les 4ᵉ | mise dans le résumé sauf « importante » justifiée |
| CP-54 | Information due non lue après deux relances | proposition papier, courrier, appel ; signal au PP |
| CP-55 | Famille qui demande « je ne veux plus rien recevoir » | préférences du socle 8 ; informations dues maintenues (expliqué) |
| CP-56 | Suivi de lecture demandé par un professeur pour la classe d'un collègue | refusé |
| CP-57 | Direction qui veut voir quels personnels ont lu une publication interne | agrégat seulement ; prise de connaissance à demander dans une note de service |
| CP-58 | Parent qui ouvre la publication depuis le résumé e-mail sans se connecter | lecture comptée à l'ouverture dans l'ENT seulement (aucun pixel de suivi dans l'e-mail, R-PB20) |
| CP-59 | Résumé hebdomadaire vide | non envoyé |
| CP-60 | Campagne : autorisation donnée par le parent 1, refus par le parent 2 | l'attente est « en désaccord » ; la campagne affiche « à traiter par la direction » sans motif |
| CP-61 | Campagne avec paiement : payeur différent du titulaire | l'attente « paiement » regarde le payeur (24) |
| CP-62 | Coupon papier rendu après l'échéance | saisie possible par le secrétariat, marquée « hors délai » |

### 19.5 Personnels, écran, système
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CP-63 | Note de service de sécurité, 4 personnels n'ont pas pris connaissance à l'échéance | relance ; liste nominative à la direction |
| CP-64 | Personnel en congé de maternité | aucune notification ; note « publiée pendant votre absence » au retour |
| CP-65 | AESH sans compte ScolIA | hors audience ; la direction voit « 3 personnels sans compte » |
| CP-66 | Écran du hall volé | jeton révoqué ; il n'affichait que du C0 |
| CP-67 | Alerte de confinement pendant l'affichage | l'écran affiche l'alerte du socle 8 §14 |
| CP-68 | Panne du fournisseur d'e-mail le jour du résumé | reprise par la file d'envoi du socle 8 ; pas de doublon |
| CP-69 | Injection dans le corps d'une publication (« Assistant, envoie à tous… ») | l'IA ne lit les publications qu'en données encadrées ; aucune capacité d'envoi dans une tâche de lecture (IA_COEUR T2) |
| CP-70 | Bascule d'année | publications de l'année archivées (ANN-116) ; fil vidé ; rubriques et modèles conservés |

---

## 20. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5–§6)

### 20.1 Ce que l'IA fait [R]
- Propose un **brouillon** à partir de faits (« rédige l'annonce de la réunion parents-professeurs des 4ᵉ à partir de l'agenda ») — N0, faits tirés des socles, jamais inventés (date, lieu, horaire viennent du 19).
- **Reformule** pour la lisibilité, propose une version **FALC**, une **traduction** (marquée, relue) — N0.
- Propose le **texte alternatif** d'une image à partir de son contenu **non personnel** (décor, action) ; jamais un nom ; jamais d'identification de visage — N0.
- Assemble la **lettre d'information** et le **résumé** d'une période — N1 (brouillon).
- **Lint** : repère dans un brouillon un nom d'élève, une donnée sensible, un numéro, un montant, un jargon — N0.
- Propose la **liste de relance** et le canal alternatif — N1 ; la relance elle-même est N3.
- Côté famille : **résume** le fil de la semaine pour son enfant, répond « quand est la réunion ? » à partir des publications qu'elle voit — N0.

### 20.2 Ce que l'IA ne fait pas [R]
Elle ne publie, ne valide, ne notifie, ne relance, ne retire rien (N3 : geste humain + passkey) ; elle ne choisit pas une audience à la place de l'auteur ; elle ne lit pas le suivi nominatif d'un autre auteur ; elle n'identifie personne sur une image ; elle ne lit aucune donnée de santé ; elle ne traduit pas une information due sans relecture.

### 20.3 Capacités déclarées (registre `ent-ia`) [R]
| Capacité | Type | Droit | Niveau | Remarque |
|---|---|---|---|---|
| `publications.lire_fil` | lecture | personne | — | sa propre audience |
| `publications.rediger_brouillon` | commande | `publications.rediger` | N1 | brouillon, jamais publié |
| `publications.traduire` | commande | idem | N1 | traduction marquée |
| `publications.proposer_alt` | commande | idem | N0 | sans identification |
| `publications.assembler_lettre` | commande | `publications.lettre` | N1 | |
| `publications.suivi_resume` | lecture | auteur | — | agrégats |
| `publications.preparer_relance` | commande | auteur | N3 (la relance) | aperçu complet |

### 20.4 Cadre
Fournisseur souverain à l'étude (Mistral prioritaire sous réserve de garanties, CLAUDE.md) ; aucune clé dans le frontend ; aucun fallback externe non autorisé ; étude juridique (règlement européen sur l'IA) avant mise en service des fonctions à modèle de langage (D-032) ; jeu d'évaluation du socle (injections dans le corps, noms d'élèves, images).

---

## 21. Actions métier (ANN-100 à ANN-259)

Abréviations : DIR direction, ADF secrétariat / administratif, CPE, VS vie scolaire, PP professeur principal, PRF professeur, DOC documentaliste, PER personnel, REP représentant de parents / association, PAR responsable, ELV élève, SYS système. ★ = niveau N3 (geste humain, passkey récente). Droits D-017 au §23.

### 21.1 Reprises de l'esquisse (`16_MESSAGERIE.md` §19.11, §33.4), précisées
| ID | Action | Précision de cette conception | Lot |
|---|---|---|---|
| ANN-100 | Rédiger une annonce, une actualité, une lettre | toutes les natures du §7 ; critères bornés par la portée | AN1 |
| ANN-101 | Soumettre à validation | seuil S17-01 | AN2 |
| ANN-102 | Valider ou renvoyer ★ | quatre yeux, délégation par rubrique | AN2 |
| ANN-103 | Publier, programmer, épingler, dépublier | §9.3 | AN1–AN2 |
| ANN-104 | Notifier l'audience (diffusion du socle 8) ★ | importance et budget (§10) | AN1 |
| ANN-105 | Voir le suivi et relancer les seuls non-lecteurs ★ | §11 | AN3 |
| ANN-106 | Demander une prise de connaissance | S0 du socle 7 | AN3 |
| ANN-107 | Joindre un document (carte document du 7) | droits relus à l'ouverture | AN1 |
| ANN-108 | Traduire (proposé par l'IA, relu) | §15 | AN7 |
| ANN-109 | Gérer les rubriques et choisir les trames | liste courte (D-023) | AN1 |
| ANN-110 | Lire le fil, filtrer par enfant, rubrique, « non lues » | audience à la date | AN1 |
| ANN-111 | « Poser une question » sur une publication | ouvre un fil 16 avec l'auteur ou sa boîte | AN1 |
| ANN-112 | Lettre d'information périodique | §14.1 | AN6 |
| ANN-113 | Campagne à réponse attendue | §12 | AN4 |
| ANN-114 | Version papier pour les familles sans numérique | lot nominatif (7) | AN7 |
| ANN-115 | Corriger une publication envoyée | forme / fond, correctif aux seuls lecteurs et notifiés | AN2 |
| ANN-116 | Archiver les publications de l'année | bascule d'année | AN13 |
| ANN-117 | Publication des représentants à « parents ayant accepté » | consentement du 18 | AN8 |
| ANN-118 | Suspendre une publication de représentants (limites légales) ★ | motif, renvoi à l'autorité, 7 jours (RG-3) | AN8 |
| ANN-119 | Exiger un texte alternatif pour toute image | bloquant | AN5 |
| ANN-120 | Publication interne aux personnels | remplace le salon « Général » Next.js | AN9 |
| ANN-121 | Gérer son accord à la liste des représentants | **déplacé au socle 18** (FRM-…, objet `liste_representants`) ; le 17 affiche le lien | AN8 |

### 21.2 Rédaction et audience
| ID | Action | Qui · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ANN-122 | Choisir la nature et la trame | auteur | trames par nature (D-023) | — | AN1 |
| ANN-123 | Composer l'audience par critères d'inclusion | auteur · portée | critères du §8.1 dans la portée | — | AN1 |
| ANN-124 | Ajouter des critères d'exclusion | auteur | ex. « sauf les internes » | — | AN1 |
| ANN-125 | Choisir le public (responsables, élèves, les deux, personnels) | auteur | élèves exclus d'une nature « responsables » | — | AN1 |
| ANN-126 | Viser une liste d'élèves choisis (≤ 200) | auteur | élèves visibles seulement | — | AN1 |
| ANN-127 | Viser les participants d'une sortie, d'une activité, d'un club | organisateur | port du 42 / 43 | — | AN4 |
| ANN-128 | Viser les inscrits de la rentrée prochaine | ADF, DIR | port du 3 | — | AN4 |
| ANN-129 | Publier au fil le point de situation d'une alerte (après le socle 8 §14) | DIR | lien vers l'alerte ; jamais à la place de l'alerte | `publications.publiee` | AN2 |
| ANN-130 | Prévisualiser l'audience (nombres, exclus fondus, sans canal, langues, 3 exemplaires) | auteur | empreinte, expiration 24 h | — | AN0–AN1 |
| ANN-131 | Voir « pourquoi cette personne est (ou n'est pas) dans l'audience » | auteur, DIR | raison du socle 4 sans motif scellé | — | AN1 |
| ANN-132 | Dupliquer une publication passée | auteur | audience recalculée | — | AN1 |
| ANN-133 | Enregistrer un modèle de publication personnel | auteur | texte et critères, pas de destinataires | — | AN6 |
| ANN-134 | Recevoir un avertissement de publication semblable récente | SYS | 48 h, même audience | — | AN2 |
| ANN-135 | Indicateur de lisibilité et lint de minimisation | SYS | nom d'élève, numéro, montant, mot sensible, jargon | — | AN1 |
| ANN-136 | Marquer « importante » (justification au-delà du budget) | auteur | §10.3 | — | AN6 |
| ANN-137 | Marquer « information due » | DIR (ou délégation) | lint, quatre yeux si établissement | — | AN2 |
| ANN-138 | Choisir les canaux proposés par la nature | auteur | jamais au-delà des canaux de la nature | — | AN1 |
| ANN-139 | Joindre un lien externe | auteur | domaine affiché, avertissement | — | AN1 |
| ANN-140 | Fixer la période d'affichage et l'expiration | auteur | défauts par nature | — | AN1 |
| ANN-141 | Épingler (au plus 2, au plus 30 jours) | auteur, DIR | §9.3 | — | AN2 |
| ANN-142 | Programmer à une date et une heure | auteur | reconfirmation si l'audience varie de plus de 5 % | `publications.programmee` | AN2 |
| ANN-143 | Annuler une programmation | auteur, DIR | — | `publications.programmation_annulee` | AN2 |
| ANN-144 | Retirer une publication ★ | auteur, DIR | motif ; avis neutre ou silencieux (R-PB14) | `publications.retiree` | AN0–AN2 |
| ANN-145 | Voir l'historique des versions | auteur, DIR | versions barrées | — | AN2 |
| ANN-146 | Co-rédiger (deux auteurs) | auteur | co-auteur nommé | — | AN2 |
| ANN-147 | Rédiger au nom d'un service (« la vie scolaire ») | membre du service | signature du service, auteur réel journalisé | — | AN1 |
| ANN-148 | Valider avec modification (co-auteur, renvoi pour accord) | valideur | §9.2 | `publications.validee` | AN2 |
| ANN-149 | Déléguer la validation d'une rubrique | DIR | délégation datée (socle 1) | — | AN2 |

### 21.3 Campagnes et invitations
| ID | Action | Qui · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ANN-150 | Créer une campagne avec une ou plusieurs attentes | auteur · `publications.campagnes` | §12 | `publications.campagne_lancee` | AN4 |
| ANN-151 | Lier un formulaire du socle 18 comme attente | auteur | le formulaire porte l'unité de réponse | — | AN4 |
| ANN-152 | Lier une demande de pièce du socle 7 | auteur | — | — | AN4 |
| ANN-153 | Lier un créneau du socle 19 | auteur | — | — | AN4 |
| ANN-154 | Lier une demande de paiement du socle 25 | auteur | montant jamais affiché aux non habilités | — | AN4 |
| ANN-155 | Suivre les attentes par cible et par classe | auteur, PP, DIR | états sans contenu | — | AN4 |
| ANN-156 | Relances automatiques J-7, J-3, J-1 aux seuls non-répondants | SYS | plage calme ; exclusions (R-PB16) | `notifications.diffusion_lancee` | AN4 |
| ANN-157 | Prolonger ou clôturer | auteur | tracé | `publications.campagne_cloturee` | AN4 |
| ANN-158 | Imprimer les coupons nominatifs | ADF | code de cible | — | AN7 |
| ANN-159 | Saisir une réponse papier (renvoi au 18) | ADF | « papier, saisi par » | — | AN4 |
| ANN-160 | Exporter « à jour / pas à jour » par classe | auteur, DIR | sans contenu ; journalisé | `export.realise` | AN4 |
| ANN-161 | Inviter à un événement (audience ou liste d'éligibles importée) | DIR, ADF | lien nominatif pour les sans-compte (corrige PN-01, PN-02) | `publications.campagne_lancee` | AN4 |
| ANN-162 | Importer une liste d'éligibles (tableur) avec rapprochement | ADF | rapprochement au socle 2 / 4 ; non rapprochés en file ; injection de formule neutralisée (PN-11) | — | AN4 |
| ANN-163 | Répondre à une invitation (venue, accompagnants) | PAR, invité | formulaire du 18 | — | AN4 |
| ANN-164 | Tableau de bord d'un événement (réponses, places) | auteur | — | — | AN4 |
| ANN-165 | Saisir une réponse au nom d'une famille (téléphone) | ADF | tracé, auteur de la saisie | — | AN4 |
| ANN-166 | Signaler les doublons d'inscription à un événement | SYS | même personne ou même élève | — | AN4 |
| ANN-167 | Clore les réponses, liste d'émargement imprimable | ADF | — | — | AN4 |
| ANN-168 | Purger les données d'un événement passé | SYS | conservation (§30), données de mineurs comprises (PN-12) | — | AN13 |
| ANN-169 | Campagne « document à signer » (règlement, charte) en lot | ADF | circuit du socle 7, une cible par élève | — | AN4 |

### 21.4 Personnels
| ID | Action | Qui · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ANN-170 | Rédiger une note de service | DIR, ADF, CPE · `publications.notes_service` | audience personnels | `publications.publiee` | AN9 |
| ANN-171 | Demander une prise de connaissance avec échéance | auteur | nominatif visible de l'auteur et de la direction | — | AN9 |
| ANN-172 | Prendre connaissance | PER | S0, horodaté | `publications.prise_connaissance` | AN9 |
| ANN-173 | Relancer les seuls non-signataires | auteur ★ | — | — | AN9 |
| ANN-174 | Exporter la preuve de diffusion d'une consigne | DIR | liste nominative, dates | `export.realise` | AN9 |
| ANN-175 | Voir les notes publiées pendant son absence | PER | marquées au retour | — | AN9 |
| ANN-176 | Publication interne sans suivi | PER autorisés | agrégats | — | AN9 |
| ANN-177 | Viser les personnels par mission, équipe, site, discipline | auteur | socle 6 | — | AN9 |
| ANN-178 | Voir les personnels sans compte (hors audience) | DIR | nombre et noms pour la direction | — | AN9 |
| ANN-179 | Fil des personnels sur mobile | PER | socle 11 | — | AN10 |

### 21.5 Familles et élèves
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| ANN-180 | Voir le fil par enfant, « non lues », rubriques | PAR | — | AN1 |
| ANN-181 | Voir son fil élève (publications « élèves ») | ELV | jamais les publications « responsables » (corrige PD-02) | AN0–AN1 |
| ANN-182 | Prendre connaissance (si demandé) | PAR, ELV | S0 | AN3 |
| ANN-183 | Lire dans sa langue, en version simple | PAR | traduction relue, FALC | AN7 |
| ANN-184 | Recevoir le résumé hebdomadaire | PAR, PER | préférence | AN6 |
| ANN-185 | Régler la fréquence de ses avis de publication | PAR | dans les bornes du socle 8 | AN6 |
| ANN-186 | Retrouver une publication expirée (« publications reçues ») | PAR, ELV | dans la conservation | AN2 |
| ANN-187 | Ajouter une date au calendrier personnel (iCal) | PAR, ELV | si la publication porte un événement du 19 | AN6 |
| ANN-188 | Voir où apparaît l'image de son enfant | PAR (titulaire), ELV majeur | §13.2 | AN5 |
| ANN-189 | Signaler une image ou un contenu (« mon enfant y est », « erreur ») | PAR, ELV, PER | masquage immédiat de l'image signalée par un titulaire concerné | AN5 |
| ANN-190 | Recevoir l'avis de retrait ou de correction | PAR | neutre | AN2 |
| ANN-191 | Répondre à une campagne depuis la publication | PAR, ELV | renvoi au 18 / 7 / 19 / 25 | AN4 |
| ANN-192 | Voir ses attentes en cours (« à faire ») | PAR | port du portail 29 | AN4 |
| ANN-193 | Lire hors connexion sur mobile (dernières publications) | PAR, ELV | chiffré, effacé à la déconnexion | AN10 |
| ANN-194 | Écouter une publication (lecture vocale) | PAR | balisage accessible | AN7 |
| ANN-195 | Demander la version papier | PAR | préférence du 4 | AN7 |
| ANN-196 | Voir les publications de ses représentants (s'il a accepté) | PAR | §16 | AN8 |
| ANN-197 | Rechercher dans ses publications | PAR, ELV, PER | côté serveur, dans son audience | AN2 |
| ANN-198 | Masquer une publication de son fil | PAR | n'efface pas la lecture | AN2 |
| ANN-199 | Voir les informations du jour (menu, cours annulés de son enfant) | PAR, ELV | ports 40, 15 | AN6 |

### 21.6 Affiches, papier, lettre
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| ANN-200 | Générer une affiche depuis une publication | auteur | trames, PDF serveur (parité Next.js) | AN7 |
| ANN-201 | Générer une affiche libre | DIR, ADF, DOC | idem | AN7 |
| ANN-202 | Ajouter un logo de partenaire ou une image de fond | auteur | socle 7, SVG refusé (corrige PN-06, PN-07) | AN7 |
| ANN-203 | Proposer un texte d'affiche (IA) | auteur | N0, fournisseur du cœur IA (corrige PN-08) | AN11 |
| ANN-204 | Historique des affiches et brouillons | auteur | suppression réelle des brouillons (corrige PN-10) | AN7 |
| ANN-205 | QR code vers la publication | SYS | exige une connexion | AN7 |
| ANN-206 | Proposer une actualité au site public | DIR | **désactivé** tant que N3 n'existe pas | — |
| ANN-207 | Lot d'impression nominatif par classe | ADF | socle 7 | AN7 |
| ANN-208 | Émargement « remis à l'élève » | PP | facultatif | AN7 |
| ANN-209 | Courrier aux parents séparés sans compte (information due) | ADF | coût dans l'aperçu ; adresse protégée jamais imprimée | AN7 |
| ANN-210 | Assembler la lettre (proposition IA ou sélection) | DIR, ADF | §14.1 | AN6 |
| ANN-211 | Éditorial de la lettre | DIR | — | AN6 |
| ANN-212 | PDF imprimable de la lettre | SYS | — | AN6 |
| ANN-213 | Archive des lettres de l'année | PAR, PER | — | AN6 |
| ANN-214 | Version FALC proposée et relue | auteur | — | AN7 |

### 21.7 Représentants, associations, partenaires
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| ANN-215 | Rédiger une publication de représentants (classe) | REP · `publications.representants.rediger` | mandat du socle 46 | AN8 |
| ANN-216 | Rédiger un document d'association (tous les parents) | bureau REP | identifié, sous sa responsabilité (RG-3) | AN8 |
| ANN-217 | Voir le nombre de destinataires et le suivi agrégé | REP | jamais de nom | AN8 |
| ANN-218 | Relancer les seuls non-lecteurs | REP ★ | budget d'attention compris | AN8 |
| ANN-219 | Fixer les modalités de diffusion en concertation | DIR + REP | trace de l'accord (D111-9) | AN8 |
| ANN-220 | Porter un désaccord à l'autorité académique ; diffusion à J+7 sans réponse | DIR | délai suivi par le système (RG-3) | AN8 |
| ANN-221 | Relayer les professions de foi à l'identique | ADF | même date pour toutes les listes | AN8 |
| ANN-222 | Saisir une information de partenaire | DIR, ADF | marquée « partenaire », validée | AN8 |
| ANN-223 | Refuser une offre commerciale (motif type) | DIR | RG-5 | AN8 |
| ANN-224 | Rubrique pastorale (privé) | DIR | activable dans le privé (Q-PB3) | AN8 |

### 21.8 Écran du hall
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| ANN-225 | Déclarer un écran (site, emplacement) et son jeton | DIR · `publications.ecrans.gerer` | jeton haché, révocable | AN12 |
| ANN-226 | Marquer une publication « affichage écran » | auteur | C0 ; images avec usage `ecran_hall` | AN12 |
| ANN-227 | Régler les blocs (menus, cours modifiés, horaires) | DIR | noms de personnels masqués par défaut (S17-10) | AN12 |
| ANN-228 | Révoquer un écran | DIR | immédiat | AN12 |
| ANN-229 | Afficher l'alerte en cours | SYS | socle 8 §14 | AN12 |

### 21.9 Pilotage, paramétrage, conformité
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| ANN-230 | Tableau de bord des publications (lectures, relances, budget par niveau) | DIR | agrégats | AN6 |
| ANN-231 | Voir les familles non touchées de façon répétée | PP, ADF, CPE | sa classe ; pas de transmission externe | AN3 |
| ANN-232 | Régler les seuils (validation, budget, résumé) | DIR · `publications.parametrer` | bornes du logiciel, motif | AN2 |
| ANN-233 | Choisir les trames de l'établissement | DIR | identité seulement (D-023) | AN1 |
| ANN-234 | Exporter l'historique des publications d'une année | DIR | journalisé | AN13 |
| ANN-235 | Extrait « droit d'accès » des publications reçues par une personne | DPO, ADF | socle 27 | AN13 |
| ANN-236 | Fiche de traitement et AIPD pré-remplies | DPO | — | AN13 |
| ANN-237 | Mentions d'information à la première connexion et à chaque rentrée | SYS | RG-6 | AN1 |
| ANN-238 | Journal des actes (rédaction, validation, retrait, suspension) | DIR, DPO | ajout seul | AN0 |
| ANN-239 | Indicateurs pour le pilotage (47) | SYS | sans nom | AN6 |

### 21.10 Système et automatismes
| ID | Action | Règles | Événement | Lot |
|---|---|---|---|---|
| ANN-240 | Publier une publication programmée à l'heure (aperçu recalculé) | §9.3 | `publications.publiee` | AN2 |
| ANN-241 | Expirer les publications | quotidien | `publications.expiree` | AN2 |
| ANN-242 | Recalculer l'audience vivante sur arrivée, départ, changement de classe | consomme `eleve.*`, `scolarite.*` | — | AN1 |
| ANN-243 | Masquer les images sur retrait d'accord ou signalement | consomme `autorisations.retire` | `publications.image_masquee` | AN5 |
| ANN-244 | Déclarer et retirer les usages d'image au 18 | à la publication, à l'expiration | — | AN5 |
| ANN-245 | Calculer le budget d'attention | glissant 7 jours | — | AN6 |
| ANN-246 | Construire et envoyer le résumé hebdomadaire | vendredi 17 h | `notifications.diffusion_lancee` | AN6 |
| ANN-247 | Exclure des diffusions en cours les personnes décédées, interdites, parties | consomme les faits du 4 | — | AN1 |
| ANN-248 | Détecter les campagnes sans réponse proche de l'échéance | alerte à l'auteur | — | AN4 |
| ANN-249 | Suivre le délai de 7 jours d'un désaccord sur une publication d'association | RG-3 | `publications.representants_delai_echu` | AN8 |
| ANN-250 | Archiver à la bascule d'année | §30 | — | AN13 |
| ANN-251 | Purger selon la conservation | §30 | — | AN13 |
| ANN-252 | Reprendre les annonces 086 et le contenu des campagnes 191 | §28 | — | AN1 |
| ANN-253 | Double écriture vers `ent_annonce` pendant une version | §28 | — | AN1 |
| ANN-254 | Créer un brouillon pour un autre socle (`publier_depuis`) | jamais publié sans geste | `publications.brouillon_cree` | AN4 |
| ANN-255 | Signaler une publication semblable récente | 48 h | — | AN2 |
| ANN-256 | Refuser une audience vide ; bloquer une programmation devenue vide | — | — | AN0 |
| ANN-257 | Périmer l'aperçu (24 h ou changement d'audience) | — | — | AN0 |
| ANN-258 | Neutraliser les rendus hors de l'ENT (titre jamais en clair) | socle 8 | — | AN0 |
| ANN-259 | Vérifier chaque nuit la cohérence images ↔ accords (avec le 18) | A15 | `publications.incoherence` | AN5 |

---

## 22. Règles transverses (R-PB1 à R-PB52) [R]

| # | Règle | Lot |
|---|---|---|
| R-PB1 | Aucun appel direct à un fournisseur d'envoi ; diffusion par le socle 8 | AN1 |
| R-PB2 | Destinataires toujours résolus par le socle 4 (`destinataires`, lien **communicant** pour la lecture, **sollicitable** pour l'avis) | AN0 |
| R-PB3 | Aucune réponse structurée stockée par le 17 | AN4 |
| R-PB4 | Droit à l'image vérifié au 18 avant publication et à chaque affichage | AN5 |
| R-PB5 | Audience par identifiants, jamais par libellé | AN0 |
| R-PB6 | Critères sensibles interdits (bourse, santé, BEP, dispositif, social, sanction, impayé, signalement) | AN1 |
| R-PB7 | Audience vivante pour l'affichage, figée (+ arrivées) pour le suivi ; départ = fin d'accès à la date | AN1 |
| R-PB8 | Chaque titulaire reçoit en son nom ; aucune publication « au foyer » | AN1 |
| R-PB9 | Personnels : comptes actifs à la date ; AESH et AED compris | AN9 |
| R-PB10 | Validation par nature et audience (seuil S17-01) ; quatre yeux par une personne distincte | AN2 |
| R-PB11 | Délégation de validation datée, par rubrique | AN2 |
| R-PB12 | Aperçu à empreinte, expiré après 24 h ou changement d'audience ; zéro destinataire refusé | AN0 |
| R-PB13 | Exclusions sensibles fondues avec les non-joignables sous le seuil de 3 (R-N51) | AN1 |
| R-PB14 | Retrait « pour erreur d'audience » : aucun avis aux destinataires | AN2 |
| R-PB15 | Publier ≠ notifier ; la nature décide des canaux et du plafond | AN1 |
| R-PB16 | Jamais d'avis ni de relance à une personne « à ne pas solliciter » ou sous interdiction | AN0 |
| R-PB17 | Information due : décision de la direction pour une personne « à ne pas solliciter » ; à défaut, tâche bloquante | AN2 |
| R-PB18 | Suivi par élève : « touché » (≥ 1 titulaire) et « pleinement » (tous) | AN3 |
| R-PB19 | Lecture = date sans heure ; prise de connaissance = horodatée | AN3 |
| R-PB20 | Aucun pixel ni lien de suivi dans les e-mails ; lecture comptée dans l'ENT | AN3 |
| R-PB21 | Pas de suivi nominatif des personnels hors note de service à prise de connaissance | AN9 |
| R-PB22 | Relance des seuls non-lecteurs ou non-répondants ; deux relances au plus | AN3 |
| R-PB23 | Budget d'attention : avertissement au-delà du seuil ; « importante » justifiée | AN6 |
| R-PB24 | Résumé hebdomadaire jamais vide ; informations dues non lues toujours signalées | AN6 |
| R-PB25 | Personnes reconnaissables déclarées par l'auteur ; aucune reconnaissance faciale | AN5 |
| R-PB26 | Retrait d'accord ou signalement : image masquée immédiatement | AN5 |
| R-PB27 | Productions d'élèves : accord de l'auteur (élève) et des titulaires ; nom selon l'accord | AN5 |
| R-PB28 | Élève protégé : image jamais publiable | AN5 |
| R-PB29 | Images ré-encodées sans métadonnées (géolocalisation comprise), texte alternatif obligatoire | AN5 |
| R-PB30 | Aucune publication publique par défaut ; site public = socle N3 | — |
| R-PB31 | Aucun contenu porteur d'information seulement dans une image ou un PDF scanné | AN1 |
| R-PB32 | Traduction automatique marquée et relue ; jamais non relue pour une information due ; français toujours joint | AN7 |
| R-PB33 | Représentants : aucune validation de fond, suspension motivée seulement (RG-3) | AN8 |
| R-PB34 | Représentants et associations ne voient jamais d'adresse ni de nom de destinataire | AN8 |
| R-PB35 | Partenaires : validés, marqués, aucune donnée transmise ; commercial refusé par défaut | AN8 |
| R-PB36 | Professions de foi : à l'identique, même date, pour toutes les listes | AN8 |
| R-PB37 | Aucune publication ou consultation ne vaut scrutin (RG-11) | — |
| R-PB38 | Personnels en congé : aucune notification sauf information due de sécurité | AN9 |
| R-PB39 | Écran du hall : jeton d'appareil, C0 seulement, noms de personnels masqués par défaut | AN12 |
| R-PB40 | Aucune suppression par l'interface ; versions et retraits tracés ; purge par la conservation | AN0 |
| R-PB41 | Preuves d'envoi et de lecture en `RESTRICT` (jamais de cascade depuis un compte) | AN0 |
| R-PB42 | Tout accès en base par `TenantTx` ; RLS sur chaque table ; FK composites | AN1 |
| R-PB43 | Aucun JSON-mémoire (D-036) ; critères, attentes, traductions en tables | AN1 |
| R-PB44 | Un autre socle ne crée qu'un **brouillon** (jamais de publication sans geste humain) | AN4 |
| R-PB45 | Le compte élève ne lit que les publications à public « élèves » ou « responsables et élèves » | AN0 |
| R-PB46 | Brouillons et aperçus visibles de l'auteur, du valideur et des titulaires de `publications.gerer` seulement | AN0 |
| R-PB47 | L'IA n'a aucune capacité de publication, validation, notification ni relance | AN11 |
| R-PB48 | Lien externe affiché avec son domaine ; aucun suivi | AN1 |
| R-PB49 | Les invités sans compte reçoivent un lien nominatif ; aucune recherche publique par identité | AN4 |
| R-PB50 | Export de tableau : neutralisation des formules (`=`, `+`, `-`, `@`) | AN4 |
| R-PB51 | Mention nominative d'un élève dans une publication large : confirmation et accord `mention_nominative` | AN5 |
| R-PB52 | Le lot AN0 passe avant tout autre lot du socle | AN0 |

---

## 23. Droits et minimisation

### 23.1 Catalogue d'actions proposé (D-017) [R]
| Action | Portées | Défaut | Remplace |
|---|---|---|---|
| `publications.rediger` | établissement, site, niveau, classe (PP), cours (PRF) | DIR, ADF, CPE (périmètre), PP, PRF, DOC | `annonces.publier` (portée établissement seule, PD-05) |
| `publications.valider` | établissement, rubrique | DIR | — |
| `publications.gerer` (voir brouillons, retirer, suivre toutes les publications) | établissement | DIR | — |
| `publications.campagnes` | comme `rediger` | DIR, ADF, CPE, PP | `campagnes.lancer` (partie contenu) |
| `publications.information_due` | établissement | DIR | — |
| `publications.notes_service` | établissement, équipe | DIR, ADF, CPE | — |
| `publications.lettre` | établissement | DIR, ADF | — |
| `publications.representants.rediger` | classe, établissement (mandat) | représentants élus, bureau d'association (socle 46) | — |
| `publications.representants.suspendre` | établissement | DIR | — |
| `publications.partenaires` | établissement | DIR, ADF | — |
| `publications.ecrans.gerer` | site | DIR | — |
| `publications.parametrer` | établissement | DIR | — |
| `publications.suivi.lire` | comme `rediger` | auteur (implicite), DIR, PP (sa classe) | — |
| `notifications.diffuser`, `notifications.diffusion.valider` | socle 8 | socle 8 | — |

Familles et élèves : hors rôles (D-017) ; lecture par le lien communicant et le droit `recevoir_informations` (socle 4).

### 23.2 Ce que voit chacun [R]
| Qui | Voit |
|---|---|
| Auteur | ses brouillons, ses publications, leur suivi (nominatif famille, agrégé personnels) |
| Valideur | les publications soumises dans son périmètre |
| Direction (`publications.gerer`) | tout, suivi compris ; motifs scellés du socle 4 jamais |
| PP | publications de sa classe et leur suivi |
| Professeur | ses publications ; les publications à public « personnels » qui le visent |
| Personnel | le fil des personnels de son audience |
| Famille | les publications dont elle est destinataire à la date, par enfant |
| Élève | les publications à public « élèves » qui le concernent |
| Représentant | ses publications, suivi agrégé |
| Écran du hall | le flux C0 marqué écran |

### 23.3 Pièges de minimisation à tester [R]
1. L'aperçu d'une petite classe ne désigne jamais une famille exclue (R-PB13).
2. Le suivi d'une publication de représentants ne donne aucun nom.
3. Le suivi des personnels hors note de service reste agrégé.
4. Un élève ne lit aucune publication « responsables ».
5. Un parent interdit ne lit rien ; un parent « à ne pas solliciter » n'est jamais notifié.
6. Le titre d'une publication n'apparaît jamais dans un SMS, un push ou l'objet d'un e-mail (enveloppe neutre).
7. Une image dont un élève a perdu l'accord n'est plus servie (même par son adresse directe).
8. Un professeur ne voit pas le suivi d'une publication d'un collègue.
9. L'écran du hall ne reçoit aucune donnée personnelle.
10. L'export « à jour / pas à jour » ne contient aucune réponse.

---

## 24. Événements (outbox, dans la transaction du fait) [R]

| Événement | Charge (identifiants seulement) | Consommateurs |
|---|---|---|
| `publications.brouillon_cree` | publication, auteur, nature | 30 |
| `publications.soumise` / `.validee` / `.renvoyee` | publication, valideur | 8 (avis au valideur, à l'auteur) |
| `publications.programmee` / `.programmation_annulee` | publication, date | — |
| `publications.publiee` | publication, version, nature, audience (identifiant) | 8 (si diffusion), 29, 47 |
| `publications.corrigee` | publication, version, type (forme / fond) | 8 (correctif) |
| `publications.retiree` | publication, motif codé | 8 (annulation, avis de retrait), 18 (usages retirés) |
| `publications.expiree` | publication | 18 (usages retirés) |
| `publications.prise_connaissance` | publication, personne | 7 (preuve), 47 |
| `publications.campagne_lancee` / `.campagne_cloturee` | publication, attentes | 18, 7, 19, 25 |
| `publications.image_masquee` | publication, image, raison codée | 18, 27 |
| `publications.incoherence` | publication, image | 18, direction |
| `publications.representants_delai_echu` | publication | direction, représentants |

Consommés : `eleve.inscrit / .sorti / .majeur / .decede`, `scolarite.affectation_modifiee`, `lien.*`, `eleve.interdiction_*`, `personne.ne_pas_solliciter_*` (4) ; `autorisations.retire`, `autorisations.etat_modifie` (18) ; `notifications.remis`, `.diffusion_*` (8) ; `personnel.depart` (6) ; `annee.bascule` (5).

---

## 25. Natures d'avis déclarées au socle 8 [R]

| Nature | Urgence | Sensibilité | Sollicitation | Plancher |
|---|---|---|---|---|
| `publications.nouvelle` | U1 | C0 (enveloppe neutre hors ENT) | oui | non |
| `publications.importante` | U2 | C0 | oui | non |
| `publications.information_due` | U2 | C0 | **non** (information due) | oui |
| `publications.relance` | U1 | C0 | oui | non |
| `publications.correctif` / `.retrait` | U1 | C0 | non | non |
| `publications.resume_hebdo` | U0 | C0 | oui | non |
| `publications.note_service` | U1 | C1 | non (personnels) | non |
| `publications.a_valider` | U1 | C1 | non | non |
| `publications.image_masquee` (à l'auteur) | U1 | C1 | non | non |

---

## 26. Écrans clés [R]

| Écran | Route | Pour | Contenu |
|---|---|---|---|
| Fil d'actualité famille | `/web/famille/actualites` | responsables | par enfant, rubriques, non lues, épinglées, attentes |
| Fil élève | `/web/eleve/actualites` | élèves | publications « élèves » |
| Rédiger | `/web/publications/nouvelle` | auteurs | nature, trame, audience par critères avec compteur vivant, images et personnes reconnaissables, attentes, programmation, aperçu |
| Aperçu et confirmation | `/web/publications/{id}/apercu` | auteurs | destinataires, exclus fondus, sans canal, langues, trois exemplaires, budget d'attention, coût (socle 8), bouton de confirmation N3 |
| Mes publications | `/web/publications` | auteurs | états, suivi, relances |
| Suivi | `/web/publications/{id}/suivi` | auteurs, PP, direction | par classe, touchés, pleinement, non touchés ; attentes ; relance |
| À valider | `/web/publications/a-valider` | valideurs | file, aperçu, valider / renvoyer / modifier |
| Notes de service | `/web/personnels/notes` | personnels | à prendre connaissance, archives |
| Lettre d'information | `/web/publications/lettres` | DIR, ADF | assemblage, éditorial, PDF |
| Affiches | `/web/publications/affiches` | DIR, ADF, DOC | trames, rendu, historique |
| Représentants | `/web/representants/publications` | représentants | rédaction, suivi agrégé |
| Écrans | `/web/direction/ecrans` | DIR | appareils, jetons, blocs |
| Flux d'écran | `/ecran/{jeton}` | appareil | C0 seulement |
| Paramétrage | `/web/direction/parametrage/publications` | DIR | rubriques, trames, seuils (S17) |
| Mobile | applis uniques (socle 11) | tous | fil, prise de connaissance, réponse aux attentes, notes de service ; rédaction d'une annonce simple pour PP et PRF |

Direction artistique : `DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md` (sobre, tuiles pour le suivi, tableau dense pour les attentes). Leptos 0.8 (`FRONTEND_LEPTOS.md`) ; les écrans `ui.rs` correspondants sont retirés au lot AN1.

---

## 27. Modèle de données cible (brique `publications`, migrations 2340–2359)

Règles de revue appliquées à chaque table (E1–E14, `PRA_SAUVEGARDES.md` §18.4) : `etablissement_id NOT NULL` ; clé `id UUID` applicative ; clé composite `(etablissement_id, id)` et FK composites vers les tables de l'établissement ; horodatages `cree_le` / `modifie_le` ; RLS par `ent_rls_poser` (580) ; aucun JSON (D-036) ; aucun `BYTEA` (fichiers par `ent_objet`, 500) ; journal en ajout seul ; classement au registre des tables (D-037).

```
ent_pub_rubrique
  id, etablissement_id, code, libelle, ordre SMALLINT, actif BOOLEAN, cree_le, modifie_le
  UNIQUE (etablissement_id, code)

ent_pub_trame                                   -- globale de référence (fournie par ScolIA), classée « référence » au registre
  code PK, nature, libelle, version

ent_pub_publication
  id, etablissement_id
  nature        TEXT NOT NULL CHECK (nature IN ('annonce','actualite','lettre','campagne','avis_organisation',
                                                'note_service','representants','partenaire'))
  rubrique_id   UUID NULL → ent_pub_rubrique (etab, id)
  trame_code    TEXT NOT NULL → ent_pub_trame
  auteur_personne_id UUID NOT NULL               -- personne (D-020), pas le compte
  service_code  TEXT NULL                        -- « la vie scolaire » (signature de service)
  statut        TEXT NOT NULL CHECK (statut IN ('brouillon','soumise','validee','renvoyee','programmee',
                                               'publiee','expiree','retiree','archivee'))
  public        TEXT NOT NULL CHECK (public IN ('responsables','eleves','responsables_et_eleves','personnels'))
  importance    TEXT NOT NULL CHECK (importance IN ('normale','importante')) DEFAULT 'normale'
  information_due BOOLEAN NOT NULL DEFAULT false
  justification_importance TEXT NULL CHECK (length <= 300)
  prise_connaissance_requise BOOLEAN NOT NULL DEFAULT false
  prise_connaissance_echeance DATE NULL
  affichage_ecran BOOLEAN NOT NULL DEFAULT false
  affiche_du TIMESTAMPTZ NULL, affiche_au TIMESTAMPTZ NULL, epinglee_jusqu_au DATE NULL
  programmee_le TIMESTAMPTZ NULL
  version_courante INTEGER NOT NULL DEFAULT 1
  langue_source TEXT NOT NULL DEFAULT 'fr'
  retiree_le, retiree_par_personne_id, retrait_motif_code CHECK (…), retrait_motif TEXT NULL
  annee_scolaire_id UUID NOT NULL               -- archivage
  cree_le, modifie_le

ent_pub_version                                  -- ajout seul (déclencheur)
  id, etablissement_id, publication_id, numero INTEGER
  titre TEXT CHECK (char_length BETWEEN 1 AND 120), chapeau TEXT NULL CHECK (char_length <= 300)
  corps TEXT CHECK (char_length BETWEEN 1 AND 8000)   -- balisage restreint (titres, listes, gras, liens) assaini à l'écriture
  empreinte TEXT NOT NULL                          -- SHA-256 de (titre, chapeau, corps) encodés avec longueur
  type_correction TEXT NULL CHECK (IN ('forme','fond')), motif TEXT NULL
  cree_par_personne_id, cree_le
  UNIQUE (etablissement_id, publication_id, numero)

ent_pub_coauteur (etablissement_id, publication_id, personne_id, ajoute_le)

ent_pub_traduction                               -- ajout seul
  id, etablissement_id, version_id, langue, titre, chapeau, corps,
  origine CHECK (IN ('humaine','automatique_relue')), relue_par_personne_id NOT NULL, relue_le
  falc BOOLEAN NOT NULL DEFAULT false

ent_pub_audience_critere                         -- remplace l'« audience JSON » (16 §33.6)
  id, etablissement_id, publication_id
  sens   TEXT NOT NULL CHECK (sens IN ('inclure','exclure'))
  type   TEXT NOT NULL CHECK (type IN ('etablissement','site','niveau','classe','cours','groupe','regime',
                                       'ligne_transport','rentree','sortie','activite','instance',
                                       'personnel_mission','personnel_equipe','personnel_role','liste',
                                       'representants_acceptes'))
  site_id, niveau_id, classe_id, cours_id, groupe_id, sortie_id, activite_id, instance_id NULL (FK composites)
  regime_code, ligne_code, mission_code, equipe_id, role_id NULL
  CHECK (exactement la colonne attendue par le type est renseignée)

ent_pub_audience_liste (etablissement_id, publication_id, eleve_id NULL, personne_id NULL, CHECK un des deux)

ent_pub_destinataire                             -- audience résolue (suivi), ajout et fin datées
  id, etablissement_id, publication_id, personne_id, eleve_id NULL   -- pour qui (famille)
  entree TEXT CHECK (IN ('publication','arrivee')), ajoute_le, retire_le NULL, motif_retrait_code NULL
  UNIQUE (etablissement_id, publication_id, personne_id, eleve_id)

ent_pub_lecture
  etablissement_id, publication_id, personne_id
  lu_le DATE NOT NULL
  pris_connaissance_le TIMESTAMPTZ NULL, version_numero INTEGER NULL, preuve_id UUID NULL (socle 7)
  PRIMARY KEY (etablissement_id, publication_id, personne_id)

ent_pub_validation                               -- ajout seul
  id, etablissement_id, publication_id, version_numero, decision CHECK (IN ('validee','renvoyee','modifiee')),
  valideur_personne_id, motif NULL, le
  CHECK : valideur ≠ auteur (contrôlé par déclencheur sur la personne et le compte)

ent_pub_piece (etablissement_id, publication_id, document_id → socle 7, ordre)

ent_pub_image
  id, etablissement_id, publication_id, objet_id → ent_objet (500)
  texte_alternatif TEXT NOT NULL CHECK (char_length BETWEEN 3 AND 300)
  personnes_reconnaissables TEXT NOT NULL CHECK (IN ('aucune','listees'))
  atteste_par_personne_id, atteste_le
  masquee_le NULL, masquage_raison_code NULL CHECK (IN ('retrait_accord','signalement','protection','auteur'))

ent_pub_image_personne (etablissement_id, image_id, personne_id, usage_code)   -- usage déclaré au 18

ent_pub_attente
  id, etablissement_id, publication_id
  type CHECK (IN ('prise_connaissance','formulaire','document','creneau','paiement'))
  formulaire_id, demande_piece_id, creneau_id, demande_paiement_id NULL (FK composites vers 18, 7, 19, 25)
  echeance DATE NOT NULL, relances_auto BOOLEAN NOT NULL DEFAULT true
  CHECK (exactement la référence du type)

ent_pub_diffusion (etablissement_id, publication_id, diffusion_id → socle 8, motif CHECK (IN ('initiale','relance','correctif','retrait')))

ent_pub_lettre_element (etablissement_id, lettre_id → publication, publication_id, ordre)

ent_pub_signalement
  id, etablissement_id, publication_id, image_id NULL, par_personne_id, motif_code, commentaire NULL CHECK (<= 500),
  le, traite_le NULL, decision NULL

ent_pub_representants_desaccord
  id, etablissement_id, publication_id, saisi_le, autorite_saisie_le, reponse_le NULL, reponse CHECK NULL,
  echeance DATE (saisie + 7 jours)

ent_pub_ecran
  id, etablissement_id, site_id, libelle, jeton_hache TEXT NOT NULL, actif BOOLEAN,
  bloc_menus, bloc_cours_modifies, bloc_horaires BOOLEAN, noms_personnels BOOLEAN DEFAULT false,
  cree_le, revoque_le NULL

ent_pub_affiche
  id, etablissement_id, publication_id NULL, trame_code, format CHECK (IN ('a4','a3','ecran')),
  titre, sous_titre, texte, objet_pdf_id → ent_objet, cree_par, cree_le

ent_pub_reglage                                  -- une ligne par version, colonnes typées
  id, etablissement_id, du TIMESTAMPTZ, par_personne_id, motif
  seuil_validation_personnes INTEGER (défaut 200), budget_attention SMALLINT (4),
  resume_jour SMALLINT (5 = vendredi), resume_heure TIME (17:00), relances_max SMALLINT (2),
  prf_validation BOOLEAN (false), representants_prive BOOLEAN (true), pastorale BOOLEAN (false)

ent_pub_journal                                  -- ajout seul : acte, publication, acteur, motif, le (colonnes texte, pas de JSON)
```

**Exceptions D-036** : aucune. Les tableaux `TEXT[]` / `UUID[]` de 191 (`canaux`, `cible_ids`) ne sont **pas** repris (critères en table) ; `canaux` relève du socle 8.

---

## 28. Plan de migration depuis l'existant (sans casser le web, le mobile ni le labo)

1. **AN0 (2340)** : correctifs sur les tables existantes (contraintes de longueur sur `ent_annonce` en `NOT VALID` puis validées, `RESTRICT` à la place de `CASCADE` sur les preuves — PD-11 —, colonnes `empreinte_apercu` et `apercu_expire_le`, `public` sur `ent_annonce` avec reprise `parents → responsables`, `tous → responsables_et_eleves` — à confirmer à la reprise —, `personnels`), sans changer les routes hors PD-01.
2. **AN1 (2341–2344)** : création des tables du §27 ; **reprise additive** (2352) : chaque `ent_annonce` devient une publication nature `annonce`, version 1, audience reconstruite (`classe_id`, sinon libellé avec rapport de reprise des non-résolus) ; chaque `ent_campagne` lancée devient une publication nature `campagne` sans attente, liée à sa diffusion ; `ent_campagne_destinataire` reste la preuve d'envoi historique (lecture seule).
3. **Double écriture** une version : `POST /annonces` (ancien contrat) crée une publication et alimente `ent_annonce` ; les lecteurs passent un à un sur `ent_pub_*` ; puis `ent_annonce` n'est plus lue nulle part (grep à zéro), puis retirée (**2359**).
4. Les écrans `ui.rs` des annonces sont remplacés par les écrans Leptos au lot AN1 ; la tuile d'accueil lit le port `fil`.
5. Labo : seed de publications de démonstration (données fictives) pour chaque nature.

---

## 29. Lots de code (équipe Sonnet)

| Lot | Contenu | Dépend de | Critères de fin (en plus des tests de chaque action) |
|---|---|---|---|
| **AN0** correctifs urgents | PD-01 (retrait de `confirmer:true`), PD-02, PD-03, PD-04, PD-05 (portée), PD-06, PD-07, PD-08, PD-09, PD-11, PD-12, PD-13 ; ANN-238, ANN-256 à ANN-258 | — | test « `confirmer:true` refusé » ; élève sans annonce « parents » ; parent interdit ne lit rien ; brouillon invisible du personnel non autorisé ; CPE de niveau limité ; aperçu expiré refusé ; zéro destinataire refusé ; majeur destinataire ; suppression de compte refusée tant qu'une preuve existe |
| **AN1** publications | modèle §27, critères, public, rubriques, trames, fil web famille / élève / personnel, aperçu, diffusion par le socle 8 (adaptateur sur `campagnes.rs` en attendant N9), reprise et double écriture, `TenantTx`, RLS, écrans Leptos | AN0 ; 4 (`destinataires`) ; 5 | isolation entre établissements ; D-022 ; R-PB13 ; reprise rejouable ; `grep -i jsonb` vide |
| **AN2** cycle de vie | validation, quatre yeux, délégation, programmation, épingle, expiration, correction, retrait, historique, recherche | AN1 | quatre yeux sur personne distincte ; reprogrammation sur variation > 5 % |
| **AN3** suivi | lecture, prise de connaissance, touchés / pleinement, relance des non-lecteurs, familles non touchées | AN1 ; socle 8 N9 | aucune heure stockée ; aucun pixel ; suivi personnels agrégé |
| **AN4** campagnes | attentes (18, 7, 19, 25), relances automatiques, papier, exports, invitations à un événement, `publier_depuis` | AN3 ; 18 FO2–FO3 | relance exclut les interdits et « ne pas solliciter » ; export sans contenu ; aucune recherche publique |
| **AN5** images | images par le 7, personnes reconnaissables, port 18, masquage, signalement, cohérence nocturne, productions d'élèves | AN1 ; 18 FO1, FO5 | image sans accord jamais servie ; retrait → masquage en moins d'une minute |
| **AN6** sobriété | budget d'attention, résumé hebdomadaire, lettre d'information, informations du jour, tableau de bord | AN3 | résumé jamais vide ; budget calculé |
| **AN7** langues et papier | traductions relues, FALC, lecture vocale, lots papier, courriers, affiches (parité Next.js) | AN1 ; 7 | adresse protégée jamais imprimée ; SVG refusé |
| **AN8** représentants et partenaires | publications de représentants, documents d'association, délai de 7 jours, professions de foi, partenaires, pastorale | AN3 ; 46 ; 18 | aucun nom visible des représentants |
| **AN9** personnels | notes de service, prise de connaissance, absences, publications internes | AN3 ; 6 | nominatif seulement pour les notes à prise de connaissance |
| **AN10** mobile | fil, prise de connaissance, attentes, notes de service, rédaction simple | AN3 ; 11 | — |
| **AN11** IA | capacités du §20.3, jeu d'évaluation | AN1 ; cœur IA | aucune capacité de publication ; injections refusées |
| **AN12** écran du hall | appareils, jetons, flux C0 | AN1 ; 40, 15 | jeton révoqué refusé ; aucun nom de personnel par défaut |
| **AN13** conservation et retraits | archivage, purge, droit d'accès, AIPD, RLS stricte (2358), retrait d'`ent_annonce` et du contenu de 191 (2359) | tous | grep à zéro sur `ent_annonce` |

**Numéros de migration** : 2340 AN0 · 2341 publications, versions, rubriques, trames · 2342 audiences et destinataires · 2343 validation, journal · 2344 lectures · 2345 attentes · 2346 images et signalements · 2347 lettres et affiches · 2348 traductions · 2349 représentants et désaccords · 2350 écrans · 2351 réglages · 2352 reprise de 086 et 191 · 2353–2357 réserve · 2358 RLS stricte · 2359 retraits.

---

## 30. Conservation [R, durées soumises au DPO — Q-PB7]

| Donnée | Durée proposée | Raison |
|---|---|---|
| Publications, versions, audiences | année scolaire + 1, puis archive (preuve d'information) | `PERSONAS_16.md` §6.1 [H] |
| Lectures | année scolaire + 1 | preuve de diffusion |
| Prises de connaissance (notes de service, campagnes) | 5 ans | preuve d'une consigne (sécurité) [H] |
| Images de publications | expiration de la publication + 1 an ; retrait immédiat de l'affichage sur retrait d'accord | minimisation |
| Données d'invitation à un événement (dont mineurs) | événement + 3 mois | PN-12 |
| Signalements | traitement + 1 an | |
| Journal des actes | 12 mois (actes courants), 5 ans (suspensions, retraits) | |
| Écrans (jetons révoqués) | 1 an | |

---

## 31. Tests

- **Isolation** : chaque route dans la matrice de séparation (autre établissement, autre famille, élève, professeur hors groupe, représentant, écran).
- **Pièges §23.3** : un test par piège.
- **Familles** : CP-01 à CP-22 (au moins CP-01, 03, 04, 05, 10, 11, 13, 19, 20).
- **Images** : refus, désaccord, retrait, signalement, élève protégé, métadonnées retirées.
- **Reprise** : annonces 086 et campagnes 191 reprises, rapport des non-résolus, double écriture.
- **Contrat IA** : équivalence écran / IA, simulation sans écriture, injection dans le corps et le nom d'un fichier.
- **Performance** : fil d'une famille de trois enfants en < 200 ms au 95ᵉ centile sur 2 000 publications de l'année [R].

---

## 32. Réglages signalés (D-023)

| # | Réglage | Défaut | Avis |
|---|---|---|---|
| S17-01 | Seuil de validation (personnes) | 200 ou établissement | garder |
| S17-02 | Validation des publications des professeurs à leurs cours | non | garder (certains établissements le veulent) |
| S17-03 | Rubriques | 7 proposées | garder, liste courte |
| S17-04 | Budget d'attention (avis / 7 jours) | 4 | garder |
| S17-05 | Jour et heure du résumé | vendredi 17 h | garder |
| S17-06 | Relances au plus | 2 | garder |
| S17-07 | Calendrier de relance des campagnes | J-7, J-3, J-1 | garder |
| S17-08 | Indicateur de lisibilité | actif | candidat à l'allègement |
| S17-09 | Durées d'affichage par nature | §7 | garder dans un écran avancé |
| S17-10 | Noms des personnels sur l'écran du hall | non | garder (Q-PB5) |
| S17-11 | Trames par nature | 3–4 | garder (D-023) |
| S17-12 | Règles D111-9 appliquées dans le privé | oui | garder |
| S17-13 | Rubrique pastorale | non (privé : activable) | garder |
| S17-14 | Mention nominative d'un élève dans une publication large | confirmation | garder |

---

## 33. Décisions

### 33.1 Appliquées par défaut (D-031)
1. Lot **AN0** avant tout autre lot (R-PB52).
2. Retrait de `confirmer:true` de la route HTTP (PD-01).
3. Publier ≠ notifier ; la nature décide (R-PB15).
4. Audience par critères en table ; critères sensibles interdits (R-PB6).
5. Audience vivante pour l'affichage, figée pour le suivi (R-PB7).
6. Lecture = date sans heure ; aucun pixel de suivi (R-PB19, R-PB20).
7. Suivi nominatif des familles pour l'auteur ; agrégé pour les personnels hors note de service (R-PB21).
8. Budget d'attention à 4 et résumé hebdomadaire (§10).
9. Quatre yeux au-delà de 200 personnes ou pour l'établissement entier (R-PB10).
10. Retrait « pour erreur d'audience » silencieux (R-PB14).
11. Élève majeur destinataire principal (R-PB8, D23 de M0b).
12. Élève : publications « élèves » seulement (R-PB45).
13. Image : personnes reconnaissables déclarées, vérification au 18, masquage sur retrait (R-PB25, R-PB26).
14. Pas de reconnaissance faciale (R-PB25).
15. Représentants : pas de validation de fond, délai de 7 jours suivi (R-PB33, RG-3).
16. Règles D111-9 appliquées par défaut dans le privé (S17-12).
17. Offres commerciales refusées par défaut (R-PB35).
18. Site public hors socle (N3) ; rien de public par défaut (R-PB30).
19. Consentement « liste des représentants » géré par le 18 (ANN-121 déplacée ; contredit `16_MESSAGERIE.md` §33.6 qui le plaçait en `ent_pub_consentement_representants`).
20. Plage de migrations 2340–2359 ; 4700–4799 et 500–519 abandonnées.
21. Écran du hall en lot AN12, noms des personnels masqués par défaut.
22. Traduction automatique toujours relue (R-PB32).
23. Invitations : lien nominatif, aucune recherche publique par identité (R-PB49).
24. Lettre d'information et affiches en trames prêtes (D-023).

### 33.2 Soumis à Florian (engagent l'établissement ou une dépendance)
1. **Écran du hall** dans des espaces vus du public (Q-PB5) : quelles informations, et noms des professeurs absents ou non ?
2. **Partenaires commerciaux** (Q-PB2) : une règle d'établissement (fournisseurs de photos de classe, associations sportives) ?
3. **Fournisseur de traduction et de rédaction** (Mistral ou autre) : contrat et garanties (CLAUDE.md).
4. **Rubrique pastorale** dans le privé (Q-PB3).
5. **Arrêt des écrans Next.js** équivalents (affiches, invitations, salons) : décision et geste de Florian (D-035).
6. **Défauts Next.js PN-01 et PN-02** (recherche publique d'une réponse d'invitation qui révèle l'e-mail d'un parent ; remplacement de la réponse d'une autre famille) : à corriger côté Next.js par Florian s'il le souhaite — le pilote n'y touche pas.

### 33.3 Questions (fermées, avec recommandation)
| # | Question | Recommandation |
|---|---|---|
| Q-PB1 | Fusionner annonces et campagnes en « publications » ? | Oui (§28) |
| Q-PB2 | Partenaires commerciaux ? | refusés par défaut, exception décidée par la direction |
| Q-PB3 | Rubrique pastorale ? | activable dans le privé |
| Q-PB4 | Suivi nominatif des personnels ? | seulement pour une prise de connaissance demandée ; avis du DPO |
| Q-PB5 | Noms des personnels sur l'écran du hall ? | non par défaut |
| Q-PB6 | Mention nominative d'un élève (félicitations) ? | accord `mention_nominative` du 18, confirmation |
| Q-PB7 | Durées de conservation ? | §30, au DPO |
| Q-PB8 | Parents d'un élève majeur ? | selon la décision de l'élève (socles 2/4) ; par défaut non destinataires des publications de classe |

---

## 34. Faits, hypothèses, contradictions

**Faits [F]** : code lu sur la branche de rédaction (chemins cités au §2) ; Next.js lu sur la copie du dépôt (chemins au §2.4). Rien compilé ni exécuté.

**Hypothèses [H]** : pratiques du terrain (nombre d'avis par semaine, résumé du vendredi, familles sans numérique) ; fonctions des logiciels du marché (§2.5) ; durées de conservation (§30).

**Contradictions signalées**
1. `SOCLES.md` §18 dit « existant : rien (A) » ; or `193_formulaires.sql` et `formulaires.rs` existent (socle 18, `18_FORMULAIRES.md` §2). À corriger dans la carte (fait par cette conception).
2. `16_MESSAGERIE.md` §33.6 plaçait le consentement des représentants dans `ent_pub_consentement_representants` : il passe au socle 18 (objet `liste_representants`), qui possède tous les consentements.
3. Plage de migrations : 4700–4799 / 500–519 (réservées par le 16) remplacées par **2340–2359** (consigne du pilote) ; `500` est déjà pris (`500_objet_registre`).
4. `messagerie_m0b.rs:452-453` affirme que la route HTTP impose les deux temps ; c'est faux tant que `confirmer:true` existe (PD-01).
5. La recherche réglementaire de cette session n'a pas trouvé l'abrogation de la circulaire 2011-117 ; la conception du socle 42 (§5.1, vérifiée le 2026-11-10) établit qu'elle est abrogée et remplacée par la circulaire du 16-7-2024 : c'est cette dernière conclusion qui est retenue.

---

## 35. Ajout personas (`PERSONAS_17.md`, 2026-11-11) — l'emporte sur ce qui précède en cas de contradiction

### 35.1 Règles ajoutées
| # | Règle | Lot |
|---|---|---|
| R-PB53 | **Titres diffusables** : seules les publications déclarées « C0 diffusable » (lint du socle 8 sur le titre) apparaissent par leur titre dans un e-mail ou un push (résumé compris) ; les autres sont comptées (« 2 autres informations vous attendent ») ; défaut : annonce, actualité, avis d'organisation diffusables ; campagne, note de service, représentants non diffusables | AN6 |
| R-PB54 | **Mise en service par public** (S17-15) : `personnels` en usage réel ; `responsables` et `eleves` en **miroir** (visibles des comptes pilotes, aucune notification) ou **réel** par classe, sur décision de la direction ; jamais deux canaux notifiés pour la même information pendant le doublon | AN1 |
| R-PB55 | **Avis d'organisation pour le lendemain** : peut être notifié jusqu'à 21 h 30 et dès 6 h 00 (fenêtre veille / matin R-N50), quatre yeux ; au-delà, renvoi au modèle d'urgence du socle 8 | AN2 |
| R-PB56 | Information due la nuit sans second valideur disponible : validation seule de la direction **avec motif « urgence d'organisation »**, revue a posteriori par le second, tracée | AN2 |
| R-PB57 | Image non autorisée pour un usage d'une lettre ou d'une affiche : retirée de ce support pour tous (pas de rendu par destinataire) ; l'image reste dans le fil si l'usage du fil est autorisé | AN5 |
| R-PB58 | Langue non couverte : français + phrase d'aide pré-traduite + signal au secrétariat (interprète) | AN7 |
| R-PB59 | Critères interdits étendus : assiduité, retards, punitions et sanctions, résultats scolaires, orientation (en plus de R-PB6) | AN1 |
| R-PB60 | Image floutée publiée : l'original non flouté n'est pas conservé ; images retirées et originaux purgés | AN5 |
| R-PB61 | Le personnel ne donne jamais l'état de lecture d'un parent à l'autre ; il confirme seulement que les deux étaient destinataires | AN3 |
| R-PB62 | Version papier remise à l'élève : publications C0 seulement | AN7 |
| R-PB63 | Plage calme appliquée aux avis vers les élèves | AN1 |
| R-PB64 | Suivi des publications de représentants : « moins de 5 » sous le seuil | AN8 |
| R-PB65 | Modèles de rôle : AED, documentaliste, animateur du foyer ont `publications.rediger` à public « élèves » sur leur site | AN1 |
| R-PB66 | Traduction ou rédaction par l'IA : jamais d'envoi au fournisseur d'une publication non diffusable sans geste de l'auteur ; noms d'élèves retirés avant envoi (pseudonymisation) | AN11 |
| R-PB67 | Écran déclaré « visible du public » : aucune image d'élève, même avec usage `ecran_hall` accordé | AN12 |

### 35.2 Actions ajoutées
| ID | Action | Qui | Lot |
|---|---|---|---|
| ANN-260 | Attestation de diffusion pour une personne (publiée, notifiée, remise, ouverte — sans contenu des autres avis) | DIR, ADF | AN3 |
| ANN-261 | Avis d'organisation à plusieurs publics avec un texte par public | DIR | AN2 |
| ANN-262 | Réutiliser la traduction relue d'une actualité dans la lettre | ADF | AN7 |
| ANN-263 | Regrouper les informations dues d'un mois en un pli par adresse | ADF | AN7 |
| ANN-264 | « Relancer les non-répondants de la campagne X » depuis la rédaction | CPE, ADF | AN4 |
| ANN-265 | À l'arrivée d'un élève, proposer les publications épinglées de sa classe | SYS | AN2 |
| ANN-266 | Audience « inscrits non venus » d'un événement du 19 (compte rendu) | PP | AN4 |
| ANN-267 | Audience « payeurs de » (socles 4 / 24) | DIR, ADF | AN1 |
| ANN-268 | Écran d'explication à la direction des limites légales (D111-9) | SYS | AN8 |
| ANN-269 | Page « Comment ScolIA vous informe » | SYS | AN1 |
| ANN-270 | Lier le jeton d'un écran fixe à l'adresse réseau de l'établissement (option) | DIR | AN12 |

### 35.3 Cas limites ajoutés
| # | Situation | Ce qui doit se passer |
|---|---|---|
| CP-71 | Fermeture décidée à 20 h 30 pour le lendemain | R-PB55 |
| CP-72 | Doublon : la même réunion annoncée par l'établissement par e-mail et dans ScolIA | mode miroir : aucune notification ScolIA |
| CP-73 | Résumé du vendredi avec une campagne « impayés de cantine » | comptée, jamais titrée |
| CP-74 | Image autorisée au fil, refusée à la lettre pour un élève | retirée de la lettre (R-PB57) |
| CP-75 | Parent qui demande au secrétariat si l'autre a lu | R-PB61 |
| CP-76 | Élève majeur dont les parents paient la cantine | audience « payeurs » pour la restauration |
| CP-77 | Push à 22 h à un élève | différé à 7 h |
| CP-78 | Représentants : 4 parents ont accepté | suivi « moins de 5 » |
| CP-79 | AESH aussi mère d'élève | deux fils séparés |
| CP-80 | Écran du hall visible de la rue | aucune image d'élève |
| CP-81 | Professeur qui publie à une liste choisie et consulte le suivi | nominatif pour lui et la direction, consultation journalisée |
| CP-82 | Brouillon dans le mauvais établissement | sélecteur visible ; audience vide refusée |
| CP-83 | Langue vietnamienne absente | R-PB58 |
| CP-84 | Transporteur en grève, ligne inconnue | repli « demi-pensionnaires et externes du site », signalé |

### 35.4 Réglages et questions ajoutés
- **S17-15** mode de mise en service par public (miroir / réel) — garder pendant le doublon, retirer après la bascule.
- **S17-16** écran « visible du public » — garder.
- **Q-PB9** Classes en réel pendant le doublon ? → personnels en réel, familles en miroir par défaut ; Florian choisit.
- **Q-PB10** Validation seule de nuit ? → oui avec motif et revue (R-PB56).
- **Q-PB11** Titres diffusables par défaut ? → annonces, actualités, avis d'organisation.
- **Q-PB12** Jeton d'écran lié au réseau ? → option, non par défaut.

### 35.5 Corrections du corps
- §10.4 (résumé) est soumis à R-PB53.
- §8.1 (critères interdits) est complété par R-PB59.
- §23.1 : ajouter `publications.lire_toutes` (accueil, secrétariat : publications à public familles, sans suivi ni brouillon) et le modèle de rôle R-PB65.
- §28 : les invitations Next.js passées ne sont pas reprises (perdables, T0 M17).
