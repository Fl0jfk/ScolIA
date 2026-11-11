# Alertes de sécurité — application Next.js en production

**Date :** 2026-11-11. **Statut :** registre de constats. **Rien n'a été vérifié en production ni corrigé.**

**Décision de Florian (D-051, 2026-11-11)** : le Next.js en production reste tel quel jusqu'à la bascule ; **chaque défaut ci-dessous doit être fermé dans la version Rust** (test de non-régression par défaut, PILOTAGE règle 14). Les « urgences » ci-dessous sont des recommandations que Florian n'a pas retenues pour le Next.js.

**Pour qui :** Florian. Il est seul à décider d'une correction, d'une restriction ou d'un arrêt de Next.js. Le pilote n'y touche pas (CLAUDE.md : Next.js est intouchable, aucun accès à la base ni aux secrets de production).

**D'où viennent ces constats.** Les sessions de conception des socles ont lu le code Next.js en lecture seule (`origin/main`, dans le dépôt). Elles n'ont rien exécuté, rien testé et ne se sont connectées à aucune base. Ce document regroupe leurs constats, fusionne les doublons (même défaut relevé par deux socles) et les trie par gravité. Les chemins de fichiers sont ceux que citent les documents sources, relatifs à la racine Next.js.

**Ce qui n'est pas dans ce registre :**
- les défauts du code **Rust** (RS3-…, DV-…, RI-…, RR-…) : ils sont suivis dans `PROJECT_STATUS.md` et dans les socles ;
- les défauts purement fonctionnels ou d'ergonomie.

Les défauts de calcul financier, qui touchent l'argent des familles sans être des failles de sécurité, sont rappelés pour mémoire à la fin.

---

## Résumé

1. **75 défauts** : **29 critiques**, **28 élevés**, **13 moyens**, **5 faibles**. Une partie n'est que probable (section « Non vérifié »).
2. Les plus graves exposent des **données de santé** d'élèves et de salariés (NJ-01, NJ-03, NJ-14 à NJ-18). Ils permettent aussi à **tout compte connecté** de lire ou de modifier ce qui ne le concerne pas : salons du personnel, dossiers de voyage, pièces jointes (NJ-04, NJ-07 à NJ-09, NJ-29). Enfin, des **pages publiques** révèlent l'e-mail des parents (NJ-05, NJ-06).
3. **Urgence 1 — couper les traitements d'IA en arrière-plan sur les élèves** : analyse « pilotage » (NJ-01), « bien-être » (NJ-03). Faire **révoquer les liens de partage OneDrive** créés automatiquement (NJ-02). Prévenir le DPO.
4. **Urgence 2 — fermer ou restreindre à un rôle** : les salons du personnel (`/api/channels`, NJ-04) ; la recherche publique des invitations et des rendez-vous d'inscription (NJ-05, NJ-06) ; le module voyages, réservé aux porteurs, à la direction et à la comptabilité (NJ-07 à NJ-13).
5. **Urgence 3 — réserver les données de santé à l'infirmerie** : module santé, extraits « voyage », fiches d'internat, alertes de cantine, motifs médicaux des absences du personnel (NJ-14 à NJ-18). Vérifier aussi que **toutes les variables secrètes** sont bien définies en production. Plusieurs modules ont en effet un **secret de repli codé en dur** (NJ-03, NJ-41, NJ-48, NJ-52).

---

## Comment lire chaque fiche

- **Gravité.** **Critique** : fuite ou modification de données sensibles à la portée de n'importe quel compte, ou de n'importe qui. **Élevée** : fuite ou abus à la portée d'un rôle interne trop large, ou risque fort sur des mineurs. **Moyenne** : exposition limitée ou défaut de protection (journal, purge, minimisation). **Faible** : durcissement. Quand la source donne une gravité, elle est reprise. Sinon, la gravité est estimée ici et marquée « (estimée) ».
- **Preuve.** Fichiers cités par la source, avec le numéro de ligne quand la source le donne.
- **Mesure immédiate.** Ce que Florian peut décider : couper une route ou un module, restreindre un rôle dans la matrice des modules, arrêter un traitement en arrière-plan, révoquer des liens, prévenir le DPO, faire corriger. **Le pilote n'exécute aucune de ces mesures.**

---

## 1. Critiques (29)

### NJ-01 — Pilotage des élèves : données nominatives et indices de santé envoyés à une IA externe, même module masqué
- **Module :** Pilotage, tableau de bord élèves (`pilotage-eleves`).
- **Ce qui se passe :** après chaque OCR, une analyse se lance en arrière-plan. Elle envoie à Mistral le nom, le prénom, les appréciations et les absences de l'élève, des indices PAP/PAI/PPS déduits des noms de fichiers, et le texte brut des bulletins. Elle colore aussi l'élève selon une « humeur » déduite automatiquement (profilage nominatif). Les routes restent actives alors que le module est masqué, et le rôle « master » voit tout.
- **Qui est exposé :** tous les élèves dont un bulletin ou une pièce a été lu par OCR. Sont en jeu des données de santé et de handicap, et une décision automatisée (RGPD art. 22).
- **Preuve :** `pilotage-eleves-analyze.ts:69-110, 160-244` ; `pilotage-eleves-logic.ts:514-588` ; `PilotageElevesClient.tsx:533-542` ; `pilotage-eleves-access.ts:39-42`.
- **Source :** `socles/47_PILOTAGE.md` §2.3–2.4 (PD-01, PD-03, PD-04) ; `socles/IA_COEUR.md` §2.4.
- **Mesure immédiate :** faire couper l'appel `schedulePilotageDossierRefresh` et désactiver les routes `pilotage-eleves`. Prévenir le DPO. À l'arrêt du module, faire supprimer les dossiers dérivés `pilotage/` du stockage objet (profilage, à ne jamais importer).

### NJ-02 — Pilotage : liens de partage OneDrive « toute l'organisation » créés pour chaque pièce
- **Module :** Pilotage (indexation OneDrive).
- **Ce qui se passe :** pour chaque pièce analysée, un lien de partage de niveau « organisation » est créé automatiquement. Les bulletins et les pièces PAP/PAI peuvent ainsi être ouverts par toute l'organisation Microsoft.
- **Qui est exposé :** les élèves (bulletins, pièces de santé et de handicap) ; l'établissement (violation de données possible).
- **Preuve :** `pilotage-eleves-analyze.ts:283`.
- **Source :** `socles/47_PILOTAGE.md` §2.3–2.4 (PD-09).
- **Mesure immédiate :** faire inventorier et **révoquer** les liens existants depuis la console d'administration Microsoft 365, puis couper leur création (NJ-01). Voir avec le DPO s'il faut notifier la CNIL, selon les accès constatés.

### NJ-03 — « Bien-être » : conversations d'élèves mineurs en détresse envoyées à une IA externe
- **Module :** Bien-être (bulle d'écoute de l'élève).
- **Ce qui se passe :** les messages d'élèves mineurs (détresse, harcèlement) sont envoyés à Mistral, classés, puis résumés pour le psychologue, sans analyse d'impact (AIPD) connue. Si la variable d'environnement manque, la signature de session retombe sur un **secret codé en dur**. Le numéro d'aide affiché est faux (« 116 » au lieu du 116 111).
- **Qui est exposé :** élèves mineurs (santé mentale).
- **Preuve :** `app/api/bien-etre/*`, `app/lib/bien-etre-*.ts`, `bien-etre-session.ts:10-16`.
- **Source :** `socles/IA_COEUR.md` §2.4 et §31 ; `socles/34_SANTE.md` §2.4.
- **Mesure immédiate :** suspendre l'appel à l'IA, ou le module lui-même. Faire vérifier que la variable secrète est bien définie en production, sans la montrer au pilote. Faire corriger le numéro d'aide. Lancer l'AIPD avec le DPO.

### NJ-04 — Salons du personnel : tout compte connecté lit, modifie et supprime tous les messages
- **Module :** Salons / channels (`/api/channels/**`).
- **Ce qui se passe :** seule la session est vérifiée. Tout utilisateur connecté lit tous les messages, modifie ou supprime n'importe quel salon et n'importe quel message. Les salons « privés » ne sont pas appliqués et l'annuaire des e-mails du personnel est ouvert.
- **Qui est exposé :** les personnels (échanges internes, e-mails). L'exposition aux comptes famille reste à vérifier.
- **Preuve :** `app/api/channels/route.ts:15-79` ; `channels/messages/route.ts:64-125` ; `channels/users/list/route.ts:5-21` ; `channel-access.ts:11-31`.
- **Source :** `socles/PERSONAS_16.md` §4.3 (NXD-1) ; `socles/17_ANNONCES.md` §2.4 (PN-03) ; `socles/16_MESSAGERIE.md` Q-M29.
- **Mesure immédiate :** couper les salons, ou les restreindre au seul personnel en attendant une correction. Vérifier qu'aucun compte famille ou élève n'y accède.

### NJ-05 — Invitations à un événement : une page publique révèle l'e-mail des parents et permet de modifier la réponse d'une autre famille
- **Module :** Invitations (`/api/invitation/**`).
- **Ce qui se passe :** la recherche publique se fait sans authentification ni limite de débit. Un nom (ou un prénom) et une date de naissance suffisent pour obtenir l'e-mail du parent, le diplôme et la situation de l'élève. Quand la liste d'éligibles n'est pas exigée, ce qui est le cas par défaut, n'importe qui peut aussi remplacer la réponse d'une autre famille, e-mail compris.
- **Qui est exposé :** familles et élèves invités.
- **Preuve :** `app/api/invitation/lookup/route.ts:12-55` ; `app/lib/invitation-db.ts:716-749, 915-942, 976-990` ; `db/schema-invitation.ts:45`.
- **Source :** `socles/17_ANNONCES.md` §2.4 et §33.2 (PN-01, PN-02) ; `socles/19_AGENDA_RDV.md` §2.3 (ND-30), Q-AG13.
- **Mesure immédiate :** fermer les pages d'invitation publiques, ou au minimum désactiver la recherche (`lookup`) et exiger la liste d'éligibles. Faire contrôler les réponses modifiées.

### NJ-06 — Rendez-vous d'inscription : nom complet, classe et e-mails des responsables révélés
- **Module :** M16, rendez-vous d'inscription (parcours public).
- **Ce qui se passe :** il suffit d'un e-mail vérifié, n'importe lequel, puis d'une date de naissance avec un nom **ou** un prénom. La page renvoie alors le nom complet de l'élève, sa classe et ses responsables avec leur e-mail. Le plafond se contourne en multipliant les adresses.
- **Qui est exposé :** familles et élèves de l'établissement.
- **Preuve :** aucun fichier cité ; module `app/(public-rentree)/rdv-inscription/`, `app/lib/rdv-inscription-*.ts` (d'après `03_INSCRIPTIONS.md` §2.4).
- **Source :** `socles/19_AGENDA_RDV.md` §2.3 (ND-20), Q-AG13 ; `socles/PERSONAS_19.md` (gravité confirmée).
- **Mesure immédiate :** suspendre le rapprochement automatique avec les enfants dans le parcours public, ou fermer la prise de rendez-vous hors campagne. Faire corriger : aucune donnée nominative ne doit être renvoyée avant un rattachement vérifié.

### NJ-07 — Voyages : n'importe quel utilisateur du module réécrit n'importe quel dossier, budget validé compris
- **Module :** Sorties et voyages (`/api/travels/*`).
- **Ce qui se passe :** `update` ne vérifie que la session. Un professeur, un surveillant, un CPE ou l'infirmerie peut réécrire n'importe quel dossier de l'établissement : statut (directement « validé »), historique, messages, budget. Le navigateur envoie le dossier entier et calcule lui-même le coût par élève. Le budget reste modifiable après validation. L'auteur d'une action de série vient du corps de la requête. La synthèse APEL est ouverte à tous.
- **Qui est exposé :** familles (montants demandés), établissement (validations falsifiables).
- **Preuve :** `api/travels/update/route.ts:30-189` ; `TripDetailsLoaded.tsx:281-289, 664-691` ; `compta-sheet/route.ts:141-146` ; `series-action/route.ts:55-57, 90-94` ; `compta-apel-summary/route.ts:21-37`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-01, VD-02, VD-18, VD-22, VD-27), Q-V30.
- **Mesure immédiate :** dans la matrice des modules, réserver voyages aux porteurs de projet, à la direction et à la comptabilité. Faire relire l'historique des statuts et des budgets validés de l'année. Faire corriger.

### NJ-08 — Voyages : le dossier complet est lisible par tout rôle du module, accueil compris
- **Module :** Sorties et voyages.
- **Ce qui se passe :** `get` et `list` renvoient tout : budget, élèves, droit à l'image, e-mails des parents. Le sélecteur d'élèves renvoie la fiche complète de **tous** les élèves (naissance, coordonnées des parents, régime, photo). L'assistant IA lit l'état et l'audit de tout dossier, avec les e-mails des parents, et les envoie au fournisseur.
- **Qui est exposé :** élèves et familles.
- **Preuve :** `api/travels/get/route.ts:15-63` ; `lib/travel-db.ts:104-143` ; `api/travels/eleves-picker/route.ts:350` ; `lib/brain-ai/tools/handlers/travels.ts` ; `travels-workflow-audit.ts:97-132, 372-386`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-03, VD-15, VD-24).
- **Mesure immédiate :** même restriction de rôles que NJ-07, en retirant l'accueil. Désactiver l'outil IA « voyages ».

### NJ-09 — Voyages : le téléchargement ouvre les pièces des autres dossiers
- **Module :** Sorties et voyages (téléchargement).
- **Ce qui se passe :** une session suffit. La recherche renvoie le premier fichier dont la clé **contient** le nom fourni (par exemple `.pdf`), y compris dans un autre dossier.
- **Qui est exposé :** toutes les pièces des voyages (identité, autorisations, devis…).
- **Preuve :** `lib/travels-s3.ts:295-319` ; `api/travels/download/route.ts:20`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-04).
- **Mesure immédiate :** faire corriger en priorité. À défaut, couper la route de téléchargement pour tout autre que le porteur du dossier.

### NJ-10 — Le serveur va chercher une adresse web fournie par le navigateur (SSRF)
- **Modules :** Voyages (envoi GED, signature, commande, ZIP, circulaire) et absences du personnel (justificatifs).
- **Ce qui se passe :** quand une pièce n'est pas trouvée, le serveur télécharge l'URL fournie (`fetch(fileUrl)`) et la joint à un e-mail ou à un document. `document-url` renvoie l'URL brute si elle ne correspond à aucune clé. La circulaire lance l'OCR sur des clés ou des URL fournies par le client. Un attaquant peut faire lire au serveur des adresses internes, ou exfiltrer des données. L'exploitabilité reste à vérifier.
- **Qui est exposé :** l'infrastructure (services internes, métadonnées d'hébergement) et indirectement toutes les données.
- **Preuve :** `lib/travels-s3.ts:382` ; `document-url/route.ts:52` ; `api/travels/generate-circular/route.ts:10-32`.
- **Source :** `socles/15_ABSENCES_PERSONNELS.md` §2.2 (AD-01) ; `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-05, VD-50) ; `socles/PERSONAS_42.md` §3.3.
- **Mesure immédiate :** faire supprimer le repli `fetch`. En attendant, Florian peut demander à l'hébergeur un filtrage des sorties réseau du serveur Next.js (aucune adresse interne ni de métadonnées).

### NJ-11 — Voyages : relais d'e-mails ouverts (destinataires fournis par le navigateur, HTML non échappé)
- **Module :** Voyages (commande, transport, cuisine, Zeendoc, rappels).
- **Ce qui se passe :** pour `send-order`, le destinataire, le titre et le nom viennent du client et sont injectés sans échappement dans le HTML. `reply-transport-email` accepte un destinataire libre. `send-cuisine` envoie des copies à des adresses fournies par le client. `send-zeendoc` ne contrôle aucun rôle. Avec `send-transport`, tout utilisateur déclenche des demandes de devis à **tous** les transporteurs. Les rappels permettent d'écrire à n'importe quel porteur. Enfin, une adresse « contact » extraite par l'IA d'un PDF piégé passe avant l'expéditeur réel. L'adresse de l'établissement peut ainsi servir à l'hameçonnage.
- **Qui est exposé :** familles, prestataires, réputation de l'établissement.
- **Preuve :** `send-order/route.ts:170-276` ; `reply-transport-email/route.ts:42-97` ; `send-cuisine/route.ts:163-223` ; `send-zeendoc/route.ts:13-15` ; `api/travels/send-transport/route.ts:9-30` ; `api/travels/reminders/route.ts:27-117` ; `lib/travels-transport-shared.ts:9-12`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-06, VD-17, VD-21, VD-49) ; `socles/PERSONAS_42.md` §3.3.
- **Mesure immédiate :** restriction de rôles (NJ-07). Faire relire les journaux d'envoi du serveur de messagerie à la recherche de destinataires inhabituels. Faire corriger.

### NJ-12 — Voyages : adresse comptable et nom d'établissement codés en dur dans la commande au transporteur
- **Module :** Voyages (commande).
- **Ce qui se passe :** en multi-établissement, les devis signés de **tous** les établissements partent vers la boîte d'un seul.
- **Qui est exposé :** les autres établissements de la plateforme (devis, listes).
- **Preuve :** `send-order/route.ts:23, 306-330`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-07).
- **Mesure immédiate :** si plusieurs établissements utilisent la plateforme, vérifier où sont partis les devis et faire corriger. Sinon, noter le risque avant tout ajout d'établissement.

### NJ-13 — Voyages : la « signature » d'un devis n'a aucune valeur probante et le fichier signé peut être substitué
- **Module :** Voyages (signature PDF).
- **Ce qui se passe :** le serveur signe un PDF situé à n'importe quelle URL. Le PDF « signé » repasse par le navigateur, qui le redépose, et peut donc être remplacé par n'importe quel fichier. La signature est une image collée, sans signature électronique ni trace.
- **Qui est exposé :** l'établissement (engagements contractuels), la direction (signature imitée).
- **Preuve :** `api/travels/sign-pdf:22, 56` ; `TripDetailsLoaded.tsx:1429-1444`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-08).
- **Mesure immédiate :** ne pas considérer ces signatures comme des preuves. En cas de doute, faire signer les commandes sur papier ou par un outil de signature reconnu jusqu'à la bascule.

### NJ-14 — Santé : les extraits « voyage » sont lisibles pour n'importe quel voyage
- **Module :** Voyages ↔ santé (`/api/travels/sante-extraits`).
- **Ce qui se passe :** tout utilisateur qui peut entrer dans le détail d'un voyage lit les extraits de santé de n'importe quel voyage, et pas seulement les encadrants de ce séjour. Il n'y a ni contrôle du dossier ni trace d'accès.
- **Qui est exposé :** élèves (données de santé).
- **Preuve :** `api/travels/sante-extraits/route.ts:21-37, 115-132`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-09) ; `socles/PERSONAS_34.md` §3.1 (NXS-11).
- **Mesure immédiate :** faire couper cette route, et remettre une fiche sanitaire papier aux encadrants. À défaut, la restreindre à l'infirmerie.

### NJ-15 — Module santé ouvert bien au-delà de l'infirmerie, sans journal des accès
- **Module :** Santé et infirmerie (M13, `app/api/sante/`).
- **Ce qui se passe :** le module est ouvert aux directions, à `admin`, à `administratif`, au CPE, aux surveillants et au psychologue, ainsi qu'à tout administrateur d'organisation. Une seule garde couvre la lecture et l'écriture. Les textes de soins, les antécédents, les protocoles de PAI et les motifs d'inaptitude sont en clair. Il n'y a ni chiffrement ni journal des accès, et la suppression d'un élève efface ses données de santé en cascade.
- **Qui est exposé :** élèves (dossier infirmier complet).
- **Preuve :** `app/lib/intranet-modules.ts:922-931`, `:1158` ; `app/lib/intranet-auth.ts:196-219` ; `drizzle/0054`–`0056` ; `sante-*-db.ts`.
- **Source :** `socles/34_SANTE.md` §2.4, §30.5 (Q-S34) ; `socles/PERSONAS_34.md` §3.1 (NXS-10, NXS-12).
- **Mesure immédiate :** dans la matrice des modules, restreindre `sante` à l'infirmerie seule. Prévenir le DPO. Poser la question de l'hébergement HDS.

### NJ-16 — Internat : allergies, PAI, traitements et notes médicales lisibles par les surveillants et le secrétariat
- **Module :** Internat (fiche de l'interne).
- **Ce qui se passe :** la fiche JSON de l'interne contient des allergies, des PAI, des traitements et des notes médicales. Les surveillants et le secrétariat la lisent, sans journal.
- **Qui est exposé :** internes (données de santé).
- **Preuve :** `api/internat/students` (GET).
- **Source :** `socles/41_INTERNAT.md` §2.3 (DI-02).
- **Mesure immédiate :** faire masquer ces champs, ou retirer la lecture de la fiche aux rôles non soignants. Lister avec le DPO les données concernées (elles ne seront pas reprises).

### NJ-17 — Cantine : libellé de santé libre affiché à l'accueil et aux surveillants
- **Module :** Restauration (passages, alertes « cantine »).
- **Ce qui se passe :** le libellé d'alerte est un texte libre (allergies, parfois diagnostics). Il s'affiche à l'accueil et aux surveillants au passage, sans journal de lecture.
- **Qui est exposé :** élèves (données de santé).
- **Preuve :** `GET /api/passages?alertes=cantine`.
- **Source :** `socles/40_RESTAURATION.md` §2.3 (DC-03).
- **Mesure immédiate :** demander à l'infirmerie de relire les libellés pour n'y laisser qu'une consigne (« sans arachide »), jamais un diagnostic. Restreindre l'affichage au personnel de cantine.

### NJ-18 — Absences du personnel : motif médical en clair pour les surveillants, dans les e-mails et dans la paie
- **Module :** Absences du personnel (M05).
- **Ce qui se passe :** pour une absence de professeur, le `GET` renvoie le motif médical en clair à l'administratif et à **tout** surveillant ou CPE. Le motif apparaît aussi dans tous les e-mails, et dans `paie_element.libelle`, jamais purgé. Les pièces PDF sont envoyées par e-mail. La configuration des destinataires de ces pièces est ouverte à tout rôle dont le libellé contient « direction ».
- **Qui est exposé :** salariés et agents (données de santé).
- **Preuve :** `route.ts:388` ; `absences-workflow-mail.ts` ; `absences-paie-sync.ts:60` ; `loadAbsenceValidationAttachments` ; `viewerCanConfigureAbsenceProcessors`.
- **Source :** `socles/15_ABSENCES_PERSONNELS.md` §2.2 (AD-03, AD-04, AD-09), Q-AP-F1.
- **Mesure immédiate :** retirer l'accès au module aux surveillants et au CPE. Faire purger les motifs des libellés de paie. Demander que les e-mails ne contiennent plus que « absence à traiter ». Prévenir le DPO.

### NJ-19 — Absences du personnel : un valideur valide sa propre absence
- **Module :** Absences du personnel (M05).
- **Ce qui se passe :** seules la file et l'outil IA excluent l'auto-validation. La route l'accepte.
- **Qui est exposé :** l'établissement (paie, contrôle interne).
- **Preuve :** `route.ts:551-561`.
- **Source :** `socles/15_ABSENCES_PERSONNELS.md` §2.2 (AD-02).
- **Mesure immédiate :** faire relire les absences validées par leur propre auteur. Faire corriger.

### NJ-20 — Cantine : un surveillant ou l'accueil change le mode de facturation et crée des factures
- **Module :** Restauration (`/api/passages`).
- **Ce qui se passe :** une seule garde (`requireModule("passages")`) protège tout. Tout rôle du module peut changer le mode de facturation, créer des factures et lire la prévision et la facturation par foyer. La recherche porte sur tout l'établissement.
- **Qui est exposé :** familles (factures, quantités par foyer).
- **Preuve :** `api/passages/route.ts:15-16, 70-71, 88, 97`.
- **Source :** `socles/40_RESTAURATION.md` §2.3, §30.7 (DC-01, DC-04, DC-23) ; `socles/PERSONAS_40.md` §3.3.
- **Mesure immédiate :** retirer le module aux surveillants et à l'accueil, sauf pour pointer les passages si c'est leur usage. Faire relire les factures de cantine émises.

### NJ-21 — Internat : le périmètre collège / lycée n'est qu'un filtre d'affichage, et les droits sont incohérents
- **Module :** Internat (appel du soir, droits).
- **Ce qui se passe :** la validation et la modification de l'appel ne vérifient pas le périmètre. Deux régimes de droits coexistent : des rôles en JSON, et un module typé qui inclut l'infirmerie sans distinguer lecture, gestion et appel. Le rôle `internat` peut créer des alertes et supprimer des activités. L'appel typé accepte une marque pour n'importe quel élève de l'établissement.
- **Qui est exposé :** internes (appel faussé), personnels.
- **Preuve :** `api/internat/roll-call/route.ts:140, 274` ; `lib/internat-rbac.ts:32-39` ; `lib/intranet-modules.ts:585` ; `alerts/route.ts:14` ; `lib/internat-appel-db.ts`.
- **Source :** `socles/41_INTERNAT.md` §2.3 (DI-01, DI-03, DI-04, DI-19).
- **Mesure immédiate :** limiter le module internat à l'équipe d'internat et à la vie scolaire. Faire corriger le contrôle de périmètre.

### NJ-22 — Internat : jetons de sortie sans expiration, et « autoriser à la place du parent » sans preuve
- **Module :** Internat (sorties).
- **Ce qui se passe :** les jetons de décision de sortie n'expirent jamais. Ils sont cherchés parmi les 200 dernières sorties, le nom du répondant est saisi librement et le limiteur de débit est en mémoire. Le personnel peut autoriser une sortie à la place d'un parent, sans accord de la direction ni preuve.
- **Qui est exposé :** internes mineurs (sortie sans autorisation valable), familles.
- **Preuve :** `lib/internat-storage.ts:339-347` ; `lib/internat-outing-decision.ts`.
- **Source :** `socles/41_INTERNAT.md` §2.3 (DI-05, DI-06).
- **Mesure immédiate :** exiger, par consigne écrite, une trace (e-mail ou écrit du parent) pour toute autorisation saisie par le personnel. Faire corriger l'expiration des jetons.

### NJ-23 — Internat : deux appels divergents le même soir (double source, écritures qui s'écrasent)
- **Module :** Internat (chambres, appel, sorties).
- **Ce qui se passe :** deux sources de vérité (JSON et tables typées) coexistent sans synchronisation. Les écritures portent sur des documents entiers, sans atomicité, et un cache dure 45 s. Deux surveillants s'écrasent mutuellement, et un élève absent peut ne pas être repéré.
- **Qui est exposé :** internes mineurs (sécurité physique la nuit).
- **Preuve :** `lib/internat-storage.ts:56-97`.
- **Source :** `socles/41_INTERNAT.md` §2.3 (DI-11, DI-13).
- **Mesure immédiate :** consigne de terrain : un seul surveillant valide l'appel par bâtiment, et un contrôle papier double l'appel. Faire choisir une seule source.

### NJ-24 — Voyages : un élève désisté ou absent le jour J reste « en sortie » et son absence n'est pas détectée
- **Module :** Voyages ↔ appel (occupation).
- **Ce qui se passe :** il n'existe pas d'état par élève. Un élève qui ne part pas reste « en sortie » et disparaît des absences de l'appel. Un dossier en attente de liste compte déjà ses élèves. Une sortie de 14 h couvre toute la journée.
- **Qui est exposé :** élèves mineurs (absence non signalée aux familles).
- **Preuve :** `lib/occupancy/travels-read.ts:9, 39-40, 107-119` ; `travel-db.ts:344-345`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-42), Q-V30.
- **Mesure immédiate :** consigne : l'encadrant envoie la liste des présents au départ à la vie scolaire, qui contrôle les non-partants. Faire corriger.

### NJ-25 — Droit à l'image « oui » par défaut, jamais vérifié avant publication
- **Module :** Voyages (liste des élèves, envoi aux parents, journal).
- **Ce qui se passe :** à l'ajout d'un élève, la case est pré-cochée « OK » (vrai par défaut en base). Le consentement est donc présumé, et il n'est jamais lu quand on publie ou envoie des photos.
- **Qui est exposé :** élèves mineurs (image), familles qui ont refusé.
- **Preuve :** `lib/travels-eleves-list.ts:58` ; `send-parents/route.ts:91-112` ; `travels-parent-blog.ts:594-621` ; `app/components/travels/hub/TripElevesListPanel.tsx:869-870` ; `db/schema-ent-relational.ts:228`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-10) ; `socles/18_FORMULAIRES.md` §2.4 (FN-01).
- **Mesure immédiate :** consigne aux organisateurs : ne publier aucune photo sans vérifier l'autorisation papier ou Charlemagne. Faire changer la valeur par défaut en « inconnu ».

### NJ-26 — Journal de voyage public par jeton, sans révocation
- **Module :** Voyages (journal des familles, `public-blog`).
- **Ce qui se passe :** le jeton ne peut être ni désactivé ni changé. Les parents d'un élève retiré gardent l'accès. Le lien est transférable, sans `noindex` ni limitation de débit, et les noms des personnels sont publics. Un délégué d'un voyage peut aussi effacer les photos d'un **autre** voyage.
- **Qui est exposé :** élèves (photos), personnels (noms).
- **Preuve :** `api/travels/public-blog/route.ts` ; `travels-parent-blog.ts:655-668`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-11, VD-12).
- **Mesure immédiate :** faire désactiver les journaux des séjours terminés et vérifier qu'aucun n'est indexé par un moteur de recherche. Pour les séjours en cours, éviter les photos d'élèves.

### NJ-27 — Voyages : un dossier peut écraser celui d'un autre établissement
- **Module :** Voyages (stockage).
- **Ce qui se passe :** l'identifiant de dossier est global, en texte, et prévisible (`trip-<horodatage>`). La mise à jour sur conflit **réécrit l'établissement** du dossier.
- **Qui est exposé :** les autres établissements de la plateforme.
- **Preuve :** `lib/travel-db.ts:353-381, 450-462` ; `complex/page.tsx:112`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-13).
- **Mesure immédiate :** si plusieurs établissements utilisent la plateforme, faire vérifier qu'aucun dossier n'a changé d'établissement. Faire corriger avant d'en ajouter un.

### NJ-28 — Voyages : e-mails entrants acceptés de n'importe qui, recopiés vers d'autres transporteurs, analysés par l'IA pour tous les établissements
- **Module :** Voyages (boîte de réception, IMAP).
- **Ce qui se passe :**
  - n'importe qui peut écrire à `boite+<établissement>@`, et son message entre dans le séjour ;
  - le chauffeur et son téléphone, saisis en texte libre, sont **renvoyés dans les PDF** adressés aux autres transporteurs (fuite entre concurrents, porte ouverte à l'hameçonnage) ;
  - toute la boîte partagée, réponses de parents comprises, part à l'IA avec l'OCR des pièces et le catalogue de **tous** les établissements ;
  - tous les e-mails des transporteurs sont copiés à l'adresse d'alerte de l'exploitant.
- **Qui est exposé :** familles (réponses), prestataires, autres établissements.
- **Preuve :** `lib/travel-email-routing.ts:29-44, 98-122` ; `travel-email-ingest.ts:428-450` ; `travels-transport-quote-pdf.ts:203-213` ; `travel-imap-poller.ts:104-332` ; `travel-tenant-match.ts:37-51`.
- **Source :** `socles/42_SORTIES_VOYAGES.md` §2.3 (VD-14, VD-46, VD-47).
- **Mesure immédiate :** suspendre le relevé automatique de la boîte (poller IMAP) et l'analyse par IA, et traiter les devis à la main. Faire retirer la copie vers l'exploitant.

### NJ-29 — Messagerie famille : la clé de la pièce jointe est fournie par le navigateur, puis servie en URL signée
- **Module :** Messagerie famille.
- **Ce qui se passe :** le parent indique lui-même la clé de stockage de sa pièce, sans contrôle de dépôt ni de préfixe. Le serveur la sert ensuite en URL signée, ce qui permettrait de récupérer un autre fichier du stockage. L'exploitabilité reste à confirmer.
- **Qui est exposé :** tout fichier du bucket de l'établissement (pièces d'élèves, justificatifs…).
- **Preuve :** `app/api/famille/messages/route.ts:53-77` ; `famille-messaging-db.ts:253-277`.
- **Source :** `socles/PERSONAS_16.md` §4.3 (NXD-2).
- **Mesure immédiate :** faire désactiver les pièces jointes côté famille jusqu'à la correction.

---

## 2. Élevés (28)

| ID | Module | Ce qui se passe | Qui est exposé | Preuve | Source | Mesure immédiate |
|---|---|---|---|---|---|---|
| NJ-30 | Vie scolaire, absences | La route des absences n'exige que le module `vs-absences` **ou** `vs-appels`, et ce dernier inclut le professeur par défaut. Un professeur peut lire les absences à traiter de tout l'établissement, note interne du CPE comprise, et en modifier le statut (faille probable). | élèves, familles | `app/api/vie-scolaire/absences/route.ts:15, 33` ; `app/lib/intranet-modules.ts:696` | `12_ASSIDUITE.md` §2.4, §27.2 (P-1) | Vérifier la matrice de modules réelle (en base). Si c'est confirmé, retirer cette route aux professeurs et faire corriger |
| NJ-31 | Appel mobile | `/api/mobile/appel` ne contrôle aucun module (faille probable). | élèves (appel) | `route.ts:16, 55` ; `channel-access.ts:40` | `12_ASSIDUITE.md` (P-1) | Vérifier qui peut appeler cette route. Faire ajouter le contrôle de module |
| NJ-32 | Discipline, carnet | Un professeur lit le carnet de tout l'établissement, motifs de sanction compris. Tout rôle du module peut prononcer n'importe quelle « sanction ». | élèves, familles | `app/lib/vs-carnet-db.ts`, `vs-sanctions-db.ts`, `app/api/vie-scolaire/{sanctions,carnet}` | `13_DISCIPLINE.md` §2.3 (VD-01, VD-02) | Restreindre la lecture du carnet aux professeurs de la classe, ou au CPE. Réserver les sanctions à la direction et au CPE |
| NJ-33 | Cahier de textes | L'écriture ne vérifie que la connexion : tout utilisateur connecté à l'intranet crée, modifie ou masque n'importe quelle entrée. `visible_famille` est vrai par défaut. | élèves, familles, professeurs | `app/api/cahier-texte/route.ts:14, 32` ; `db/schema-socle-faits.ts:373-399` | `20_CAHIER_DE_TEXTES.md` §2.1 (NX-CDT-1) ; `PERSONAS_20.md` | Module jugé « vide probable » par T0 : le couper s'il n'est pas utilisé, sinon le réserver aux professeurs |
| NJ-34 | Accueil, absences | `canDeclareAccueilAbsence` renvoie toujours vrai : tout titulaire du module déclare une absence pour tout élève et tout personnel. | élèves, personnels | `lib/accueil-absences-access.ts:10-12` | `14_ACCUEIL_SURETE.md` §2.3 (VA-01) | Limiter le module accueil aux agents d'accueil et à la vie scolaire |
| NJ-35 | Accueil, absences | Le motif libre (400 caractères) reçoit des informations de santé ou de situation familiale, visibles de tout le tableau. Le rôle accueil voit les absences du personnel OGEC. | élèves, familles, personnels | `api/accueil/absences/route.ts:28` ; `accueil-absences-access.ts:19-28` | `14_ACCUEIL_SURETE.md` §2.3 (VA-02, VA-05) | Consigne écrite : motif court sans santé ni situation familiale. Retirer à l'accueil la vue des absences du personnel |
| NJ-36 | Messagerie famille | IDOR : la route de pièce jointe ne vérifie pas que le fil appartient au foyer du parent (un commentaire du code le reconnaît). | familles | `famille/messages/attachment/[id]/route.ts:13-17` | `PERSONAS_16.md` §4.3 (NXD-3) | Faire corriger ; en attendant, même mesure que NJ-29 |
| NJ-37 | Messagerie famille (personnel) | La règle « professeur limité à ses classes » se contourne avec un `foyerId` ou un `threadId` quelconque. La liste du personnel renvoie les 100 derniers fils de l'établissement. Les classes sont comparées par préfixe (« 4 » correspond à « 4B »). | familles, élèves | `vie-scolaire/messages-foyer/route.ts:42-148` ; `famille-messaging-db.ts:146-168` | `PERSONAS_16.md` §4.3 (NXD-4, NXD-15) | Retirer aux professeurs l'accès aux fils famille hors de leurs élèves, ou le réserver à la vie scolaire |
| NJ-38 | Dépôts de fichiers (salons, affiches, voyages) | Le type de fichier est déclaré par le client, sans antivirus. Zip et tous les `image/*`, `audio/*`, `video/*` sont admis, ainsi que le SVG. `channels/upload` n'a aucune limite et sert les fichiers à une **adresse publique non signée**. Le dépôt des voyages accepte du HTML. | tous les utilisateurs (fichiers piégés), personnels | `channels/upload/route.ts:18-23` ; `posters/upload/route.ts:30-49` ; `api/travels/upload/route.ts:17-49` | `PERSONAS_16.md` (NXD-5) ; `17_ANNONCES.md` (PN-05, PN-07) ; `42_SORTIES_VOYAGES.md` (VD-23) | Couper `channels/upload` (avec NJ-04). Faire vérifier qu'aucun fichier public sensible n'est en ligne |
| NJ-39 | Affiches | Les clés d'image sont arbitraires : le serveur signe **n'importe quel objet du stockage** (potentiellement critique si on peut viser des pièces d'élèves). | tout le bucket | `app/lib/posters/storage.ts:169-177` ; `render-pdf.ts:228-252` | `17_ANNONCES.md` §2.4 (PN-06) | Couper le module affiches hors besoin immédiat ; faire corriger |
| NJ-40 | Rendez-vous d'inscription | La clé de stockage du PAP est fournie par le client, rattachée comme document de santé, puis **réécrite** sans contrôle. | familles candidates, élèves (santé) | — | `19_AGENDA_RDV.md` §2.3 (ND-21), Q-AG13 | Désactiver le dépôt du PAP avant rendez-vous (il n'est pas nécessaire avant l'admission) |
| NJ-41 | Rendez-vous d'inscription, portes ouvertes | Google Agenda reçoit la mention du PAP, le téléphone et l'e-mail du parent. Les jetons sont en clair en base, y compris le jeton de rafraîchissement Google. Le jeton de garde est réutilisable 2 h. Il existe un secret de repli. | familles (santé, coordonnées) | `db/schema-rdv-inscription.ts` | `19_AGENDA_RDV.md` §2.3 (ND-25, ND-26) ; `03_INSCRIPTIONS.md` §10 | Faire retirer le PAP et les coordonnées des événements Google. Vérifier que le secret est défini en production. Voir avec le DPO le statut de Google comme sous-traitant |
| NJ-42 | Tâches de fond M16/M17 | Les tâches parcourent **tous les établissements** avec le jeton, le SMTP et le domaine de l'établissement de la requête en cours. | autres établissements | — | `19_AGENDA_RDV.md` §2.3 (ND-23) | Si plusieurs établissements utilisent la plateforme : faire corriger ; sinon, noter le risque avant tout ajout |
| NJ-43 | Planning par domaines (M03) | Élévation de privilège : la responsable d'un domaine, ou tout rôle dont le libellé contient « administratif » ou « direction », réécrit tous les domaines, leurs responsables et leurs séances. | professeurs, organisation | — | `19_AGENDA_RDV.md` §2.3 (ND-13), Q-AG13 | Vérifier les libellés de rôles ; limiter le module aux responsables de domaine connus |
| NJ-44 | IA (configuration) | Repli de clé entre établissements : un établissement sans clé utilise la **première clé trouvée chez un autre** et transmet ses données sous le contrat de celui-ci. | établissements, leurs élèves | `app/lib/tenant-config.ts:17-44` | `IA_COEUR.md` §2.4, §31 | Faire supprimer ce repli ; vérifier quel établissement consomme quelle clé |
| NJ-45 | IA, extraction RH | Le texte intégral des bulletins de paie est envoyé au modèle, **invité à extraire le NIR**. | salariés | `app/lib/personnel-match.ts:92` | `IA_COEUR.md` §2.4 | Couper cette fonction ; voir avec le DPO (le NIR est très encadré) |
| NJ-46 | IA, base de connaissances | L'ingestion de « connaissances » exige seulement une session, sans contrôle de rôle ni relecture. Le texte est injecté ensuite dans les consignes de l'assistant (injection persistante). | tous les utilisateurs de l'assistant | `app/api/chatbot/ingest/route.ts` (l. 21-29) | `IA_COEUR.md` §2.4 | Réserver l'ingestion à l'administrateur ; faire relire le contenu déjà ingéré |
| NJ-47 | Pilotage | Un rôle `administratif` sans profil reçoit des secteurs par repli, et le paramètre `userId` est ignoré. Aucun seuil de petits effectifs n'est appliqué (réidentification). | élèves | `pilotage-eleves-access.ts:48-67` ; `pilotage-eleves.ts:108-130` | `47_PILOTAGE.md` §2.3 (PD-02, PD-05) | Couvert par l'arrêt du module (NJ-01) |
| NJ-48 | Pilotage, supervision | La supervision est ouverte à toute direction. Le secret du cookie a une **valeur de repli de développement**. | personnels, données de supervision | `supervision.ts:40-46` | `47_PILOTAGE.md` §2.3–2.4 (PD-14) | Vérifier que le secret est défini en production ; restreindre la supervision à l'administrateur |
| NJ-49 | Pilotage, OneDrive | Le jeton Microsoft passe dans le corps de la requête et le `folderPath` du client est accepté tel quel. Les notes confidentielles de direction sont écrites depuis le navigateur dans le OneDrive personnel, sans contrôle ni audit. | élèves, direction | `index/route.ts:29, 93` | `47_PILOTAGE.md` §2.3 (PD-08, PD-10) | Couvert par l'arrêt du module (NJ-01) ; faire relire les dossiers OneDrive concernés |
| NJ-50 | Tableau de semaine | `week-sheet/reparse` n'exige que `requireAuth` : tout connecté déclenche un OCR et un appel Mistral payants. | établissement (coûts), données envoyées | `app/api/dashboard/week-sheet/*` | `47_PILOTAGE.md` §2.3–2.4 (PD-13) | Module « perdable » : le couper |
| NJ-51 | Internat, confirmations par lien | La confirmation d'installation est consommée par un simple **GET** : l'antivirus d'une messagerie peut confirmer à la place du parent. Même défaut pour l'annulation de rendez-vous d'inscription : les analyseurs de liens annulent des rendez-vous. | familles | `api/internat/installation/confirm` | `41_INTERNAT.md` (DI-07) ; `19_AGENDA_RDV.md` (ND-22) | Vérifier les confirmations et annulations suspectes (faites dans la seconde qui suit l'envoi) ; faire passer par une page avec un bouton |
| NJ-52 | Tâches planifiées (internat, voyages) | Les routes planifiées sont protégées par un secret transmis dans le corps, comparé en temps non constant, avec **repli sur un autre secret**. | familles (récapitulatifs, rappels) | `weekly-parent-digest`, `roll-call/reminder` ; `api/travels/reminders/route.ts:27-117` | `41_INTERNAT.md` (DI-08) ; `42_SORTIES_VOYAGES.md` (VD-21) | Vérifier que chaque secret est défini et distinct ; faire corriger |
| NJ-53 | Scripts d'import internat | `NODE_TLS_REJECT_UNAUTHORIZED=0` est posé par défaut : la vérification des certificats est désactivée, ce qui permet une interception. | élèves importés | `scripts/import-internat-excel.ts:113` ; `import-siecle-eleves-internat.ts:56` | `41_INTERNAT.md` (DI-09) | Ne plus lancer ces scripts tant qu'ils ne sont pas corrigés |
| NJ-54 | Internat, chambres | L'affectation vérifie la capacité puis insère, sans transaction. Ni le sexe, ni l'aile, ni le régime, ni la date ne sont vérifiés. | internes mineurs | `lib/internat-chambres-db.ts:236-338` | `41_INTERNAT.md` (DI-15) ; `PERSONAS_41.md` (DI-27) | Contrôle manuel du plan des chambres à chaque modification |
| NJ-55 | Voyages, prestataires | Les e-mails et téléphones des parents de toute la liste partent au transporteur. L'INE et les régimes alimentaires (« sans porc », « végétarien ») partent en clair à la cuisine et à une liste libre. | familles, élèves (convictions présumées) | `travels-eleves-list.ts:303-312, 399-422` | `42_SORTIES_VOYAGES.md` (VD-16) ; `40_RESTAURATION.md` (DC-06) | Consigne : n'envoyer aux prestataires que les effectifs, sauf exigence écrite (avion) |
| NJ-56 | Voyages, fil interne | Le fil interne est lisible et inscriptible par **toute** direction et toute comptabilité du groupe. L'auteur des messages n'est pas stocké. | personnels, familles citées | `lib/travels-thread-unread.ts:107-119` ; `db/schema-ent-relational.ts:258-275` | `42_SORTIES_VOYAGES.md` (VD-20) | Restreindre aux personnes du séjour (matrice) ; faire stocker l'auteur |
| NJ-57 | Absences du personnel | Les surveillants et le CPE peuvent saisir et importer des absences **déjà validées** (`canAdminIngest = canViewCalendar`). | salariés (paie) | `route.ts:209-223` | `15_ABSENCES_PERSONNELS.md` (AD-05) | Avec NJ-18 : retirer le module aux surveillants et au CPE |

> Également rangé « haute » par la source : **DC-10**, un `ALTER TABLE` exécuté depuis une route (`lib/eleve-scolarite-schema.ts`, `40_RESTAURATION.md`). C'est une modification du schéma de production pendant une requête. Il est compté parmi les moyens ci-dessous (NJ-63), car il n'expose pas de données directement.
>
> Les messages **anonymes** des salons (NXD-6, rangé « élevée » par la source ; PN-04) sont réglés avec NJ-04.

---

## 3. Moyens (13)

| ID | Module | Ce qui se passe | Qui est exposé | Preuve | Source | Mesure immédiate |
|---|---|---|---|---|---|---|
| NJ-58 | Messagerie, notifications | 120 caractères du message partent dans le Web Push, et 280 dans `famille_notif.preview`. Le contenu s'affiche ainsi sur l'écran verrouillé, et chez un tiers de notification. | familles, élèves | `app/lib/famille-messaging-db.ts:279-300` | `PERSONAS_16.md` (NXD-8) ; `17_ANNONCES.md` §2.4 | Faire réduire l'aperçu à « nouveau message » |
| NJ-59 | Messagerie | La présence de tout membre est lisible de tous. Le « vu par » montre l'e-mail et l'image. Les erreurs internes sont renvoyées au client (même défaut pour les rendez-vous, ND-27). Le contrôle de clé de pièce se fait par `includes` au lieu de `startsWith`. | personnels, familles | — | `PERSONAS_16.md` (NXD-9, NXD-12) ; `19_AGENDA_RDV.md` (ND-27) | Faire corriger avec NJ-29 / NJ-36 |
| NJ-60 | Messagerie | L'édition et la suppression écrasent le contenu, sans historique. Une purge physique à 365 jours se lance à la main. Une conversation est détruite quand le dernier membre part. Aucun gel n'est possible en cas de litige. | personnels, familles (preuve) | — | `PERSONAS_16.md` (NXD-10, NXD-11) | Ne pas lancer la purge sans décision ; prévoir un export si un litige est en cours |
| NJ-61 | Visio | Le STUN de Google est imposé : les adresses IP des participants partent chez un tiers. Les salles et le limiteur sont en mémoire. | participants | — | `PERSONAS_16.md` (NXD-13) | Voir avec le DPO ; à défaut, informer les utilisateurs |
| NJ-62 | Agenda, salles | **Toutes** les réservations (e-mails, commentaires, annulées comprises) sont lisibles par tout compte, sans borne de période. Le destinataire de repli de l'annulation est fourni par le navigateur. Les e-mails (salles, domaines, portes ouvertes) subissent une **injection HTML** à partir de champs saisis, parfois publiquement. | personnels, visiteurs | — | `19_AGENDA_RDV.md` (ND-05, ND-06, ND-07, ND-17, ND-29) | Faire échapper les gabarits ; restreindre la lecture des réservations |
| NJ-63 | Restauration | Une **lecture** crée un tarif à 5 € codé en dur, et un `ALTER TABLE` s'exécute depuis une route (DDL en production). | établissement, familles | `ensureRepasTarif` ; `lib/eleve-scolarite-schema.ts` | `40_RESTAURATION.md` (DC-09, DC-10) | Faire retirer le DDL des routes ; vérifier les tarifs créés |
| NJ-64 | Portes ouvertes, consentements | Le consentement est **forcé à vrai** sans case (portes ouvertes). Le consentement du rendez-vous d'inscription est un booléen non versionné. Le covoiturage pose `consentAt` sans geste. Les confirmations partent à une adresse non vérifiée. Le secret de support passe dans l'URL et se compare en temps non constant. | familles candidates | `app/api/portes-ouvertes/register/route.ts:95-96` ; `app/api/accueil/portes-ouvertes/route.ts:238` ; `app/api/rdv-inscription/[direction]/book/route.ts:44-47` ; `app/api/covoiturage/route.ts:198-211` | `18_FORMULAIRES.md` §2.4 (FN-02 à FN-04) ; `19_AGENDA_RDV.md` (ND-29) | Ne pas réutiliser ces « consentements » pour recontacter les prospects ; faire ajouter une vraie case |
| NJ-65 | Site public, invitations | Le corps et l'image des actualités ne sont pas assainis (injection de script possible selon le rendu). L'export des réponses d'invitation permet l'**injection de formule** dans un tableur. Les invitations n'ont ni purge des données de mineurs (naissance, e-mail, situation) ni trace de l'auteur d'une réponse. | visiteurs du site, personnels qui ouvrent l'export, familles | `site-cms/posts/route.ts:43-45` ; `app/lib/invitation-export.ts:12-18` ; `db/schema-invitation.ts:87-125` | `17_ANNONCES.md` (PN-09, PN-11, PN-12) | Consigne : ne pas ouvrir l'export dans un tableur sans désactiver les formules. Faire purger les invitations closes |
| NJ-66 | Discipline, internat | Le motif libre est montré à la famille. Les **témoins sont nommés dans le courriel** d'incident d'internat. Les données ne sont jamais effacées (effacement légal des sanctions). Ni l'autorité parentale ni la majorité ne sont prises en compte. Les incidents d'internat sont supprimés définitivement, et la suppression d'un élève efface son historique en cascade. | élèves témoins, élèves sanctionnés, familles | `app/api/internat/incidents/route.ts` | `13_DISCIPLINE.md` §2.3 (VD-04, VD-05, VD-07, VD-10, VD-15) ; `41_INTERNAT.md` (DI-17, DI-21) | Consigne : ne pas nommer de témoin dans les courriels. Faire purger les sanctions à effacer |
| NJ-67 | Fiches de dialogue (M10, en usage réel) | L'élève est rattaché par e-mail. Les responsables sont pris dans des adresses importées une fois : les liens vivants, le droit de recevoir les documents et une **interdiction de communication** ne sont pas pris en compte. Un parent écarté par une décision peut donc recevoir la fiche. | familles séparées, élèves | rapport `docs/rapport-fiches-dialogue-personas-v2.md` | `23_BULLETINS.md` §2.3 (NXFD-1, NXFD-2) | Avant chaque campagne, faire vérifier à la main les destinataires des familles à situation particulière |
| NJ-68 | Foyers, messagerie famille | À l'import SIECLE, un foyer est retrouvé par son **libellé d'adresse**, ce qui peut fusionner deux familles. La messagerie famille est adressée au **foyer** : deux parents d'un même foyer lisent les échanges l'un de l'autre. | familles séparées ou homonymes | `siecle-people-import.ts:74-76, 207-230` ; `db/schema-famille-messaging.ts:28` | `04_ADMINISTRATIF.md` §2.4 ; `09_INTEROP.md` §6 ; `16_MESSAGERIE.md` §2.4 | Contrôler les foyers fusionnés après chaque import |
| NJ-69 | Facturation (M15) | IBAN, BIC et RUM des familles sont en clair en base (`foyer_facturation`). | payeurs | — | `TRANSITION_T0_INVENTAIRE.md` §3 (M15), §4 | Si le module n'est pas utilisé (« vide probable »), faire vider ces colonnes ; sinon, restreindre l'accès à la base |
| NJ-70 | Divers, IA et recherche | L'INE, les notes, les appréciations, le « comportement » et les souhaits des parents (« professeur à éviter ») partent à l'IA pour la répartition des classes. L'OCR classe **automatiquement** une pièce dans le dossier de l'élève dès 0,9 de « confiance ». Les e-mails des voyages sont rattachés automatiquement ; si l'IA tombe en panne, ils sont marqués lus et perdus. La recherche globale charge **tous les élèves inscrits** dans le navigateur. | élèves, familles | `class-allocation-wish-match.ts` ; `ocr-analyze-eleve.ts:516-529` ; `travel-email-ingest.ts:605-636` ; `GlobalEleveSearch.tsx` | `IA_COEUR.md` §2.4 ; `05_STRUCTURE.md` §2.4 ; `42_SORTIES_VOYAGES.md` (VD-45) ; `29_PORTAILS.md` (NXP-14) | Faire passer le classement OCR en proposition à relire ; ne pas relancer la répartition par IA ; restreindre la recherche globale |

---

## 4. Faibles (5)

| ID | Module | Ce qui se passe | Preuve | Source | Mesure immédiate |
|---|---|---|---|---|---|
| NJ-71 | Vie scolaire, appel | Le professeur est rattaché à ses créneaux **par ressemblance de nom**. | `vs-appels-access.ts:48-52` | `12_ASSIDUITE.md` §2.4 | Vérifier les homonymes parmi les professeurs |
| NJ-72 | Limitation de débit | Il n'y a aucune limitation de débit, sauf pour l'assistant. Les limiteurs sont en mémoire et font confiance à `x-forwarded-for`. La préinscription publique n'a pas de limite visible. | — | `PERSONAS_16.md` (NXD-17) ; `19_AGENDA_RDV.md` (ND-32) ; `03_INSCRIPTIONS.md` §2.4 | Demander une limitation au niveau du proxy ou de l'hébergeur |
| NJ-73 | Assistant IA | Les arguments d'outils sont journalisés en clair (tronqués), et le texte d'erreur du fournisseur est affiché tel quel. L'outil « absence accueil » n'a ni plafond ni contrôle de portée propre. Le chemin IA des réservations ne vérifie ni « réservable » ni l'emploi du temps. | `app/lib/brain-ai/tools/registry.ts:248-275` | `IA_COEUR.md` §2.4 ; `14_ACCUEIL_SURETE.md` (VA-07) ; `19_AGENDA_RDV.md` (ND-09) | Faire réduire les journaux ; à traiter avec la correction des routes concernées |
| NJ-74 | Affiches, site | Le texte d'affiche et le nom du partenaire partent à un fournisseur externe de modèle de langage. La suppression est incomplète (brouillons, billets). | `posters/suggest-copy/route.ts:38-63` ; `posters/storage.ts:150-156` ; `site-cms/storage.ts:36-56` | `17_ANNONCES.md` (PN-08, PN-10) | Faible sensibilité : aucune urgence |
| NJ-75 | Tableau de bord | Le cache des signaux (60 s par personne) n'est pas vidé quand les droits changent (lien suspendu, départ). | `valkey-keys.ts` | `29_PORTAILS.md` (NXP-16) | Aucune urgence |

---

## 5. Pour mémoire : erreurs de calcul qui touchent l'argent des familles (hors sécurité)

Ces défauts ne sont pas comptés ci-dessus, mais ils peuvent faire payer une famille à tort :
- **Cantine** :
  - la prévision exclut les élèves en voyage, mais la facture au forfait les compte ; les absences, fermetures et jours fériés sont facturés (DC-07) ;
  - le numéro de facture vaut `Date.now()`, sans idempotence : deux clics font deux factures par foyer (DC-08, `lib/passages-facturation-db.ts:320`) ;
  - plusieurs scolarités « en cours » dupliquent la prévision et la facture (DC-11) ;
  - en mode réel, les re-passages sont comptés (DC-12) ;
  - source : `40_RESTAURATION.md` §2.3.
- **Voyages** :
  - les montants sont en virgule flottante, et `parseEuroAmount("1.350")` donne 1,35 € (VD-33) ;
  - le **coût des accompagnateurs est mis à la charge des familles**, ce qui est interdit dans le public (VD-34) ;
  - la circulaire et la comptabilité affichent deux montants différents (VD-35) ;
  - la synthèse APEL compte les séjours annulés (VD-36) ;
  - source : `42_SORTIES_VOYAGES.md` §2.3.
- **Absences du personnel** : rouvrir une absence ne défait pas l'élément de paie. L'élément est ignoré sans période ouverte, et des doublons sont possibles (AD-10, AD-11, `15_ABSENCES_PERSONNELS.md` §2.2).

Mesure : faire relire par la comptabilité les factures de cantine et les budgets de voyage de l'année.

---

## 5 bis. Ajouts du 2026-11-11 (conceptions 43 et 44, non classés dans les décomptes ci-dessus)

Source : `43_PERISCOLAIRE_TRANSPORTS.md` et `44_RESSOURCES_MATERIELLES.md` (§2), lecture de code, non testé.

- **NJ-76 (élevé) — Réservation de salles : n'importe quel compte connecté peut annuler ou modifier la réservation d'un autre** (DRM-14, DRM-15). Le serveur vérifie seulement que la session existe, pas que la réservation appartient à l'utilisateur. À reprendre au socle 19.
- **NJ-77 (moyen) — Photocopies : la file d'attente affiche les demandes de tous les sites** (DRM-01).
- **NJ-78 (élevé) — Photocopies : on peut rattacher à sa demande le PDF déposé par quelqu'un d'autre** (DRM-02).
- **NJ-79 (élevé) — Partenariats : l'inscription publique fonctionne sans authentification** (DPS-17).

## 6. Non vérifié (hypothèses des sources)

Ces points sont présentés par les sources comme **probables**, **à confirmer** ou déduits. Florian doit les faire vérifier avant de décider.

1. **NJ-30 et NJ-31** sont des « failles probables ». La matrice de modules réelle, stockée en base, peut corriger le défaut par défaut ; le pilote ne peut pas la lire (`12_ASSIDUITE.md` P-1).
2. **NJ-04** : l'exposition des salons aux **comptes famille** est « à vérifier ». Le proxy des comptes « appli » ne liste pas `/api/channels` (`PERSONAS_16.md` NXD-1).
3. **NJ-29** (clé de pièce fournie par le client) et **NJ-10** (requête vers une URL arbitraire) : l'exploitabilité est « à confirmer » ou « à vérifier » (`PERSONAS_16.md`, `15_ABSENCES_PERSONNELS.md` AD-01).
4. **Voyages** : `42_SORTIES_VOYAGES.md` distingue les défauts « certains » (lus dans le code) des défauts « probables » (déduits, à confirmer par un essai), sans toujours préciser lesquels sont probables.
5. **Discipline** : le rattachement parent–enfant **par simple égalité d'adresse e-mail** (VD-03) et l'établissement **déduit de l'hôte** (VD-06) sont notés [H] (`13_DISCIPLINE.md` §2.3). S'ils sont confirmés, un parent pourrait voir les sanctions d'un enfant qui n'est pas le sien, ou un établissement les données d'un autre.
6. **Usage réel des modules** : la maturité et l'usage de chaque module dans l'inventaire T0 sont des hypothèses [H] (`TRANSITION_T0_INVENTAIRE.md` §3.0). Un module inutilisé (cahier de textes, facturation M15, tableau de semaine, pilotage) peut être **coupé** sans gêne. C'est souvent la mesure la plus simple.
7. **Bien-être et IA** : l'absence d'AIPD est « connue » des sources par défaut d'information, pas par vérification (`IA_COEUR.md`). Les garanties contractuelles réelles avec Mistral ne sont pas vérifiées.
8. **Secrets dans le stockage objet** : le registre des établissements garde, à côté de `tenants/index.json`, un **dossier de secrets par établissement** (`app/lib/tenant-registry.ts:14, 47-50`). La source ne le classe pas comme défaut ; elle demande seulement de l'exclure de tout export (`TRANSITION_T0_INVENTAIRE.md` §2). Il reste à vérifier qui peut lire ce préfixe, en particulier avec NJ-29 et NJ-39 (signature d'objets arbitraires).
9. **Bandeau de cookies** : le constat « aucun bandeau ; cookies techniques seulement » est marqué [à vérifier] (`18_FORMULAIRES.md` §2.4).
10. **Contradiction relevée** : `42_SORTIES_VOYAGES.md` §33.3 point 2 attribue l'envoi aux transporteurs sans contrôle à VD-14, alors que le §2.3 et `PERSONAS_42.md` le numérotent **VD-49** (VD-14 porte sur les e-mails entrants). Ce registre suit le §2.3.

---

## 7. Rappel des limites

- Aucune de ces failles n'a été **exploitée ni testée**. Ce sont des lectures de code, à des dates différentes (2026-11-06 à 2026-11-11) : le code de production a pu changer depuis.
- Le pilote **n'exécute aucune mesure** sur Next.js, ne lit ni la base ni les secrets de production, et ne demande aucun identifiant. Toute correction, restriction ou coupure est une **décision et un geste de Florian** (CLAUDE.md ; D-035 point 3).
- Les questions déjà ouvertes dans les socles qui renvoient ici : Q-AP-F1 (15), Q-M29 (16), §33.2 point 6 (17), Q-AG13 (19), NX-CDT-1 (20), Q-S34 (34), Q-V30 (42), §2.4 et §30.2 (47).
