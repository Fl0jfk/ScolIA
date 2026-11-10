# Passe « personas » sur le socle 42 — Sorties et voyages scolaires

**Date :** 2026-11-10 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit, rien modifié).
**Branche :** `ent-socle-voyages-personas`, partie de `origin/ent-mc0hoe` (`b6caf7b`).
**Éprouve :** `docs/socles/42_SORTIES_VOYAGES.md` (lu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` D-031, **D-034**, **D-035** (et ses deux précisions), **D-036**, `docs/TRANSITION_NEXT_RUST.md`, `docs/TRANSITION_T0_INVENTAIRE.md` (M01, §4.1, R6) ; forme reprise de `PERSONAS_10.md`, `PERSONAS_21.md` et `PERSONAS_24_25.md` (personas P05, P06, P13 du 24–25 reprises ici sous l'angle du voyage).
**Contexte imposé :** le module voyages de Next.js est **fiable et utilisé en production** ; il est remplacé par ScolIA aux **vacances de printemps, du samedi 17 avril au lundi 3 mai 2027** (T3 de `TRANSITION_NEXT_RUST.md`), branché sur le cœur.
**Ajouts :** faits **directement** dans `42_SORTIES_VOYAGES.md`, marqués **« ajout personas »** (nouveau §32 pour l'essentiel, plus des corrections ponctuelles aux §5, §8.1, §25, §26, §27). **Rien n'est supprimé** ; les contradictions sont écrites au §31 du 42.
**Numérotation :** manques **M-2201 à M-2299** (plage libre : le plus haut numéro pris sur les branches est M-2071) ; existant Next.js **NX-75 à NX-86** ; défauts **VD-49, VD-50** ; actions **VOY-250 à VOY-289** ; règles **R-SV37 à R-SV47** ; cas limites **CV-65 à CV-84** ; questions **Q-V31 à Q-V43** ; réglages **S-436 à S-439**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

La conception est solide sur le fond : réécriture complète, commandes serveur, participants référencés, minimisation, réglementaire à jour. En la faisant vivre par seize personas **et en la confrontant au code Next.js**, quatre bloquants apparaissent. Les trois premiers concernent la **bascule d'avril 2027**, que la conception traite en une demi-page (§26) alors que c'est le moment le plus risqué du module.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Les voyages en cours à la bascule n'ont pas de régime.** Le §26 se contredit : le point 5 dit « séjours en cours repris dans leur état », le point 8 dit « séjours en cours terminés dans Next.js ». Or, mi-avril, l'établissement a typiquement des voyages de mai–juin **en pleine préparation** (devis signés, liste close, circulaire partie), des sorties de fin d'année en circuit, et parfois un séjour **qui part pendant les vacances** (P01, P16). | Terminer dans Next.js oblige à garder les deux outils jusqu'en juillet, avec les défauts critiques (VD-01 à VD-14) ; tout reprendre d'un coup sans règle casse les séjours en vol (journal des familles, rappels, avenants). Une seule règle évite la double saisie et le « qui a raison ? ». | **Propriété unique par séjour** (R-SV37) : à tout instant, un séjour est modifiable dans **un seul** outil ; ScolIA tient une **ombre en lecture** des séjours encore portés par Next.js, mise à jour à chaque reprise (rejouable, différentielle, VOY-259). **À la date de gel** (S-436, défaut : vendredi 16 avril 2027 au soir), **tous** les séjours non clos passent à ScolIA selon la table de correspondance du §32.3 du 42, **sauf les séjours « en vol »** (départ entre le gel et le 3 mai + 2 jours) : ceux-là finissent dans Next.js et passent à ScolIA après leur retour, comme archives. Les séjours terminés sont repris en archive. Tableau de bord de bascule (VOY-263). | V0 (reprise) ; Q-V31 |
| **B** | **Ce qu'on croit reprendre n'est pas dans Next.js.** Vérifié dans le code [F] : Next.js n'a **aucun paiement de famille** (EasyTransac ne facture que les établissements clients ; `facture`, `encaissement`, `facture_echeance` sans lien avec un voyage ; engagement de paiement sur **coupon papier**, `api/travels/generate-circular/route.ts:213-220`) et **aucune autorisation parentale recueillie** (coupon papier ; droit à l'image = booléen **vrai par défaut** coché par l'enseignant, `db/schema-ent-relational.ts:228` ; ni AST, ni pièce d'identité, ni fiche). Les « paiements partiels » et les « autorisations déjà signées » d'un voyage en cours sont **dans le classeur de l'organisateur et le tableur de la comptabilité**. | Un séjour repris « prêt » partirait avec **zéro autorisation** au sens de ScolIA, et l'appel de départ (ou le filtre du journal) bloquerait tout le monde ; à l'inverse, présumer « autorisé » pour tous serait faux. Les versements déjà faits seraient invisibles et les familles relancées à tort. | **État de départ attesté** (R-SV39) : à la bascule, l'organisateur (ou le secrétariat) **atteste** par participant les autorisations papier reçues (date, attesté par, scan facultatif, VOY-260), en masse avec confirmation ; la comptabilité **importe** l'état des versements tenu hors de Next.js (tableur : élève, payeur, montant, date, moyen) en « versements constatés avant bascule » (VOY-261). Le **droit à l'image Next.js n'est pas un consentement** : repris « inconnu » (R-SV40). Les **envois déjà faits** (rappels, calendrier, listes, avenants) sont repris pour ne rien renvoyer (R-SV38, VOY-262). | V0 ; Q-V32 |
| **C** | **L'argent en 2027-2028 contredit D-034.** Le lot V2 fait **facturer** la part familles par ScolIA (24 F10) et l'inscrit au « parcours minimal » ; or D-034 garde les **finances dans Charlemagne** pendant l'année de doublon (« ScolIA calcule en parallèle sans prélever »). Le module voyages, lui, est en **usage réel** (D-035). | Sans règle, soit ScolIA émet des factures que Charlemagne émet aussi (double facturation des familles), soit le module voyages n'a aucun suivi de l'argent, ce qui est le premier irritant de l'organisateur et de la comptable (P01, P05). | **Mode « suivi »** (R-SV44, S-437) tant que les finances restent dans Charlemagne : ScolIA **décide** la part (budget, part familles, échéancier proposé), **répartit** entre payeurs (socle 4), **pointe** les versements reçus (chèques remis par l'organisateur avec bordereau, espèces, virements, chèques-vacances) et **exporte** parts et versements vers la comptabilité ; il **n'émet aucune facture ni prélèvement**. Le lot V2 est scindé : **V2a suivi** (rentrée 2027, voire avant la bascule si prêt) et **V2b facturation réelle** (après le doublon, avec les socles 24–25). | V2a, V2b |
| **D** | **Le premier lot n'est pas encore la parité, et le modèle viole D-036.** (1) L'audit de code trouve **12 fonctions ou comportements** sans NX (§3, NX-75 à NX-86 : alerte des courriels non rattachés, suppression des dossiers de test, filtre « groupe scolaire », places d'accompagnateurs à pourvoir, e-mail d'activation du journal, « en sortie » lu par l'internat et la cantine, convives adultes sans compte, commande cuisine par jour de semaine…) et **trois reculs** (image de couverture en V12, lecture des pièces par IA en V12, droit à l'image en V1). (2) Le modèle cible (§25) garde du JSON et des tableaux sans clé étrangère : `ent_sortie_restauration.jours jsonb`, `ent_sortie_reprise.rapport jsonb`, `pieces[]`, `categories[]`, `eleves_visibles uuid[]`, `incidents_codes[]`, règles du moteur « données versionnées » sans table. | Une action utilisée qui disparaît le 3 mai, c'est un retour à Next.js ; un `jsonb` métier, c'est exactement ce que D-036 interdit (et ce qui a fait VD-26). | **V0 = parité branchée, datée pour la bascule** : les 12 manques entrent en V0 (VOY-250 à VOY-257), ainsi qu'une version minimale de VOY-171 (consentement image recueilli au guichet ou par le formulaire), VOY-173 (autorisation papier enregistrée), VOY-214 (pré-remplissage du budget, déjà utilisé) et le choix d'image dans le catalogue (la recherche web reste en V12, Q-V17 : **changement d'usage signalé**). Le §25 est **normalisé** (§32.6 du 42) ; seul `ent_sortie_journal.avant/apres` reste en `jsonb`, comme **instantané d'audit jamais relu**, exception déclarée à `MIGRATIONS.md`. | V0 ; R-SV43 |

**Deux défauts de Next.js à ajouter** à la liste VD (signalés à Florian, Next.js reste intouchable) : `send-transport` (VD-49) et `generate-circular` (VD-50) ne vérifient que la session et prennent le dossier fourni par le navigateur : n'importe quel utilisateur du module déclenche des envois à **tous** les transporteurs, ou une lecture par OCR de pièces désignées par lui.

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_21.md` §1 et `PERSONAS_24_25.md` §1 : seize personas réalistes, prises dans la consigne et dans le tableau « acteurs » du §3 de la conception, couvrant le privé sous contrat (lycée pilote, OGEC) et le public (EPLE), les familles ordinaires et les cas particuliers, plus un **persona de bascule** (P16 : Florian administrateur, le 16 avril 2027 au soir). Pour chacune : son **année** (ou sa journée), ses **irritants**, **peurs** et ce qui la **ravirait** ; les **manques** et **contradictions** ; les **cas limites** ; les exigences nouvelles.

**Grille transversale** posée à chaque persona : **(1) parité** — que fait-elle aujourd'hui dans Next.js, et le retrouve-t-elle le 3 mai ? **(2) reprise** — dans quel état est son séjour le 16 avril, que devient-il ? **(3) D-036** — ce qu'elle saisit est-il une colonne, une table, ou un JSON caché ? Puis les questions habituelles : qui voit quoi, ce qui change à une date, ce que la famille fait seule, ce que l'IA prépare sans décider.

**Audit de parité [F].** Un sous-agent a relu en lecture seule les **52 routes** de `app/api/travels/*`, les pages `(admin)/travels/*` et `voyages/suivi/[token]`, les composants et bibliothèques du module, et les consommateurs de « en sortie » ailleurs (internat, cantine, accueil). Chaque route a été mise en face d'un NX ; les champs du dossier (`lib/travels-types.ts`, formulaires `simple` et `complex`) ont été comparés au modèle du §25. Résultat au §3.

**Vérification réglementaire.** Un sous-agent de recherche a contrôlé le §5 du 42 le 2026-11-10 (Légifrance, eduscol, service-public, gov.uk, cnil.fr, justice.fr, FNOGEC). Le BO renvoie toujours 403. Résultats au §5, versés au §5 du 42.

**Gravité** (identique aux passes précédentes) : **bloquant** · **important** · **confort** · **différenciant**.

**Conventions.** [V] lu sur une source officielle ; [V-copie] lu sur copie intégrale ; [NT] non trouvé ; [H] hypothèse ; [R] recommandation ; [F] fait vérifié dans le code (chemins relatifs à `app/` de Next.js, sur `origin/main`).

**Limites.** Les personas sont construites, pas interviewées. Les pratiques de terrain (coupons papier, chèques collectés en classe, tableur de la comptable, séjours pendant les vacances, usage de l'option « groupe scolaire ») sont des hypothèses fondées sur le code et sur l'usage courant [H] ; Florian les corrige. Aucune donnée réelle n'a été lue : une adresse comptable écrite en dur dans `send-order/route.ts:23` est remplacée ici par « [donnée] ».

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **M. Lemaire**, professeur d'espagnol, organise un séjour à Séville (avion, agence, 42 élèves, départ le 10 mai 2027) | Lycée pilote, privé sous contrat | Agence, liste nominative pour les billets, chambres, AST, **séjour en préparation à la bascule** |
| P02 | **Mme Roche**, professeure d'EPS : une journée au parc des sports (car payé par l'établissement) et une série piscine | Lycée pilote | Création rapide, série, sortie obligatoire gratuite, pique-niques |
| P03 | **Mme Diallo**, AESH de Léo, et **M. Perrin**, parent bénévole | Lycée pilote | Ne pas compter dans l'encadrement, accord de l'employeur, compte limité, ne voir que son groupe |
| P04 | **Mme Duval**, cheffe d'établissement | Lycée pilote | Signer une fois, contrôles faits, Ariane, risques, délégation |
| P05 | **Mme Lambert**, comptable de l'OGEC, et **le secrétariat** | Lycée pilote | Devis d'agence, acomptes fournisseurs, chèques, part annoncée, Charlemagne |
| P06 | **M. et Mme Garnier**, séparés, payent chacun la moitié | Lycée pilote, fille en 1ʳᵉ | Deux parts, deux signataires, désaccord, droit à l'image, ne rien voir de l'autre |
| P07 | **Famille Benoît**, père au chômage depuis novembre | Lycée pilote | Fonds social, échelonnement, discrétion, « ne pas être celui qui ne part pas » |
| P08 | **Léo** (ULIS, fauteuil, AESH) et **Inès** (PAI allergie à l'arachide) | Lycée pilote | Accessibilité, surcoût jamais facturé, trousse, consignes sans diagnostic, chambre |
| P09 | **Hugo**, 18 ans, terminale | Lycée pilote | Signe et paie lui-même, ses parents payaient, qui est informé |
| P10 | **Famille Petit**, déménage en février, voyage déjà payé à moitié | Lycée pilote | Départ en cours de préparation, billet nominatif, remboursement au bon payeur |
| P11 | **Agence « Horizons » (fictive)**, organisatrice du forfait Séville | — (sans compte) | Devis forfait, liste nominative, rooming list, acomptes, avenants, données hors UE |
| P12 | **Mme Benali**, CPE, et **Yanis**, AED | Lycée pilote | Non-partants à accueillir, appel du jour, élèves restés au lycée |
| P13 | **M. Haddad**, professeur de mathématiques dont la 2ᵈᵉ B part, et **Mme Ferrand**, qui part et laisse ses autres classes | Lycée pilote | Classe vidée, devoir surveillé, travail à donner, remplacement |
| P14 | **M. Blanc**, délégué à la protection des données de l'OGEC | — | Registre, AIPD, durées, transferts hors UE, sous-traitants, droit d'accès |
| P15 | Deux **attaquants** : **Mme Sorel**, mère d'un participant qui veut la liste des autres ; **M. Valin**, professeur d'un autre établissement (même OGEC, puis autre client) | — | Toutes les portes : journal, calendrier, envois groupés, sorties liées, RLS, courriel entrant |
| P16 | **Florian**, administrateur, le **vendredi 16 avril 2027 au soir** | Lycée pilote | Gel, reprise, rapprochement, séjours en vol, rapport de contrôle, retour arrière |

---

## 3. Audit de parité avec Next.js (fait de code)

**Verdict.** Les 74 NX de la conception couvrent les 52 routes et l'essentiel des boutons [F]. Restent **12 fonctions ou comportements sans NX**, **3 reculs** (lot plus tardif que l'usage actuel), **2 défauts de sécurité absents de la liste VD**, et **une quinzaine de champs** du dossier absents du §25.

### 3.1 Fonctions et comportements sans NX (ajoutés au §2.2 du 42)

| N° | Existant [F] | Où | Reprise (ajout personas) |
|---|---|---|---|
| NX-75 | Alerte par e-mail d'un courriel entrant non rattaché, vers une adresse d'alerte de l'exploitant | `lib/travel-email-alert.ts:25-68`, `lib/travel-email-routing.ts:13-23` | VOY-250 : avis `voyages.courrier_a_trier` au **secrétariat de l'établissement** (jamais à l'exploitant, VD-47) |
| NX-76 | Fiche « actualité » de l'assistant IA réécrite à chaque enregistrement du dossier | `lib/brain-ai/sync/knowledge-writer.ts:141-158`, appelée par `update/route.ts:239-253` | **Abandonnée explicitement** (Q-V36) : l'assistant lit l'état vivant par la capacité `sorties.prochaine_etape` (§18.3), sous les droits de l'utilisateur ; une copie de connaissance serait une mémoire hors table (D-036) |
| NX-77 | Suppression de dossiers de test par le support, avec simulation (seul chemin de suppression ; `/api/travels/delete` cité dans `lib/accueil-access.ts:92` n'existe pas) | `support-cleanup-tests/route.ts:51-112` | VOY-251 : supprimer un **brouillon** ou un dossier marqué « essai », simulation d'abord ; un dossier soumis ne se supprime jamais (il s'annule) |
| NX-78 | Filtre de la liste par établissement ou « groupe scolaire » ; le libellé « Groupe scolaire » décide quelle direction signe | `(admin)/travels/page.tsx:79,166-181`, `lib/travels-types.ts:156`, `complex/page.tsx:198-207` | VOY-252 : filtre par établissement ; un séjour commun = **sorties liées** (CV-17, Q-V35), chaque établissement signe pour ses participants |
| NX-79 | Effectif d'accompagnateurs déclaré supérieur au nombre de noms (« noms à préciser plus tard ») | `update/route.ts:117-135` | VOY-253 : **places d'encadrant à pourvoir**, comptées dans le contrôle d'encadrement comme « manquantes » |
| NX-80 | E-mail d'activation du journal de voyage aux parents, avec le calendrier `.ics` joint | `lib/travels-parent-blog.ts:298-352` | VOY-254 : avis d'ouverture du journal (socle 8), destinataires recalculés, calendrier joint |
| NX-81 | Mode « Modifier » général (titre, destination, objectifs, accompagnateurs) et réédition complète d'une sortie simple (`?edit=`) | `TripDetailsLoaded.tsx:120,597-620`, `simple/page.tsx:55,97-145,289` | VOY-106 **précisée** : titre, objectifs, destination, encadrants modifiables, champ par champ, journalisés (jamais d'écrasement, VD-29) |
| NX-82 | « En sortie » lu par l'**internat** (repas du soir, autorisations de sortie) et la **prévision des passages à la cantine** | `lib/internat-repas-soir-db.ts:55,132`, `lib/internat-sorties-db.ts:194-199`, `lib/passages-prevision-shared.ts:26,100` | VOY-255 : le port `sorties::en_sortie` est consommé par les socles 40 et 41 dès V0 (CV-47 n'avait pas d'action) |
| NX-83 | Réouverture d'un dossier finalisé vers **cinq** étapes amont | `lib/travels-direction-permissions.ts:34-44` | §8.1 **corrigé** : `soumise`, `preparation`, `a_signer`, `budget_a_valider`, `a_autoriser` (la conception n'en gardait que trois) |
| NX-84 | Demandes de devis au transporteur **dès la création** d'un séjour avec car | `complex/page.tsx:117-141` | **Changement d'usage signalé** (Q-V34) : R-SV5 les place après la validation pédagogique (VD-32) ; pour ne pas faire perdre une à deux semaines, la direction valide la pédagogie depuis son tableau en un geste, et l'organisateur peut **préparer** les demandes avant (elles partent à la validation) |
| NX-85 | Commande cuisine saisie **par jour de semaine** (lundi à vendredi), avec **heure et lieu de livraison** | `complex/page.tsx:57-68` | §14 et §25 **précisés** : commande par **date**, heure et lieu de livraison ; conversion jour de semaine → dates à la reprise |
| NX-86 | Convives **adultes sans compte** (« autre ») qui mangent ; accompagnateurs en nom libre | `lib/travels-types.ts:81-90` | VOY-256 : convives externes (libellé, nombre, type de panier) en table typée ; encadrant sans compte = **intervenant** créé au socle 6 (Q-V19), jamais un nom libre |

### 3.2 Reculs de parité corrigés (entrent en V0)

| NX | Lot prévu | Usage aujourd'hui | Décision (D-031) |
|---|---|---|---|
| NX-21 image de couverture | V12 | automatique | V0 : choix dans le **catalogue** de l'établissement ou dépôt d'une image ; la recherche web et le choix par IA restent en V12, désactivés par défaut (Q-V17) — **changement d'usage signalé à Florian** |
| NX-52 pré-remplissage du budget par lecture des pièces | V12 (V0 sans IA) | utilisé par la comptabilité | V0 : VOY-214 avec IA N1 sur les **devis confirmés** seulement (montants à confirmer, Q-V24) |
| NX-25 droit à l'image | V1 (VOY-171) | case cochée par l'enseignant | V0 : consentement **minimal** recueilli (guichet, papier scanné, ou formulaire simple), filtre des publications en V0 ; la campagne complète du socle 18 reste en V1 |

### 3.3 Défauts de sécurité nouveaux (ajoutés au §2.3 du 42)

| N° | Gravité | Défaut | Où | Correction |
|---|---|---|---|---|
| VD-49 | **Critique** | `send-transport` ne vérifie que la session et prend le dossier fourni par le client : tout utilisateur déclenche des demandes de devis à **tous** les transporteurs | `api/travels/send-transport/route.ts:9-30` | Commande serveur VOY-156, droit `sorties.modifier`, état `preparation` (R-SV1, R-SV5) |
| VD-50 | Haute | `generate-circular` ne vérifie que la session, prend le dossier du client et lance l'OCR sur des clés ou URL de pièces fournies par lui | `api/travels/generate-circular/route.ts:10-32` | Rendu serveur depuis les faits (VOY-170) ; pièces par identifiant rattaché (VD-05) |

### 3.4 Champs absents du modèle (versés au §32.6 du 42)

Destination en texte libre (adresse, « lieu et programme ») ; effectifs **déclarés** élèves et accompagnateurs (distincts des inscrits) ; `needsBus`, car sur place, consignes libres au transporteur (numéro de vol) ; heure et lieu de livraison des repas ; points du calendrier des familles (dépôt, reprise, autre, durée, note, « inclure la durée du séjour ») ; image et attribution (auteur, licence, URL) ; **instantanés du devis et des dates commandés** (base du calcul des avenants) et avenant en attente ; envois déjà faits (`remindersSent`, `parentComLogs`, empreinte du calendrier, instantané de la liste des paniers) ; délégués, jeton et expiration du journal ; fiche budget : imputation (compte, ligne), lignes de facturation déclaratives, nombre d'élèves facturés, **marge figée en euros**. Champs morts à ne pas reprendre : `partFamille`, `partEtablissement`, `description` (toujours vides [F]).

### 3.5 États réels et correspondance (versée au §32.3 du 42)

Vérifié [F] : création → `EN_ATTENTE_DIR_INITIAL` ; séjour sans car et sortie simple sautent `PROF_LOGISTICS` ; `FINALISE_DIR_ATTENTE_ELEVES` passe à `VALIDE` à la confirmation de la liste ; refus et demandes de modification possibles depuis quatre étapes, avec `previousStatus` ; statuts hérités `BROUILLON` et `EN_ATTENTE_VALIDATION` encore testés (`TripDetailsLoaded.tsx:1047`) mais absents du §26. La table complète est au §32.3 du 42.

---

## 4. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait dans `42_SORTIES_VOYAGES.md` (identifiant) ou le socle où il est renvoyé.

### P01 — M. Lemaire, séjour à Séville en avion, par une agence, en préparation à la bascule

**Situation.** 42 élèves de 1ʳᵉ (dont un élève de nationalité marocaine, Inès avec PAI, Léo en fauteuil), 4 encadrants, avion Paris–Séville et forfait d'agence (vol, car sur place, hôtel, visites), 640 € par élève annoncés en décembre, départ le **lundi 10 mai 2027**. Le **16 avril**, dans Next.js : devis de l'agence signé (image collée), budget validé, liste confirmée, circulaire partie en janvier, coupons papier dans un classeur (39 reçus sur 42), chèques en partie remis à la comptabilité.
**Irritants.** Retaper la liste pour l'agence (avec dates de naissance et numéros de pièce), pour la préfecture, pour l'hôtel ; relancer trois familles pour un coupon ; ne pas savoir qui a payé. **Peurs.** Arriver à l'aéroport sans une AST ; une chambre non accessible pour Léo ; perdre son dossier au changement d'outil trois semaines avant le départ. **Ce qui le ravirait.** Retrouver le 3 mai **exactement** son séjour, avec ses 39 coupons cochés et les 3 manquants en rouge ; envoyer la liste à l'agence en un geste, avec les seuls champs demandés.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2201 | **Régime du séjour en préparation à la bascule** : repris dans son état, propriété transférée à ScolIA au gel, avec ses pièces, son journal et son budget validé | Contradiction §26 points 5 et 8 ; trois semaines avant un départ, on ne garde pas deux outils | **bloquant** | 42 | R-SV37, VOY-258, §32.3 |
| M-2202 | **Autorisations papier déjà reçues attestées** en masse (39 cochées, date de réception, attesté par M. Lemaire, scan facultatif) | Sans elles, le séjour repris n'a aucun participant « prêt » (bloquant B) | **bloquant** | 42, 18 | R-SV39, VOY-260 |
| M-2203 | **Liste nominative à l'agence selon le contrat** : nom, prénom, date de naissance, nationalité, type et numéro de pièce pour les billets d'avion ; champs choisis par le contrat, motif, trace, envoi N3 | S-426 (effectif seul) ne permet pas d'acheter un billet d'avion ; aujourd'hui le CSV au transporteur contient les **téléphones des parents** (VD-16) | **important** | 42, 7 | VOY-271, R-SV41 |
| M-2204 | **Répartition des chambres** (rooming list) : par chambre, élèves et adultes, chambre accessible, sans motif ni diagnostic ; transmise à l'agence | Demandée par tout hébergeur [H] ; aujourd'hui faite dans un tableur | important | 42, 34 | VOY-272 |
| M-2205 | **Devis forfait d'agence** : un devis couvre transport, hébergement, activités ; ventilation par poste au budget (pour la part encadrement exclue, R-SV15) ; pièces de l'agence (immatriculation Atout France, garantie financière) | Le §13 pense « transporteur » ; sans ventilation, la part encadrement ne peut être isolée | important | 42, 48 | VOY-269 |
| M-2206 | **Changement de nom ou annulation d'un billet** après désistement : frais réels du prestataire → base de l'avoir (§12.5) | Un billet d'avion nominatif n'est souvent ni remboursable ni cessible gratuitement [H] | important | 42, 24 | VOY-273, CV-71 |
| M-2207 | **Ressortissant d'un pays tiers vers l'Espagne** : liste de voyageurs (préfecture) **et** vérification de sa reconnaissance par le pays de destination | §5.4 : la liste couvre les mineurs non européens ; un majeur en est exclu [V] | important | 42 | §5.4 (précisé) |
| M-2208 | **Transfert hors UE signalé** quand l'hébergeur ou l'agence est hors UE (Maroc, Royaume-Uni…) ; information des familles dans la circulaire | RGPD chap. V ; le RU a une décision d'adéquation [H, à revoir par le DPO], d'autres pays non | important | 42 | VOY-289, Q-V43 |
| M-2209 | **Signature du devis faite dans Next.js** (image collée) reprise comme « signée hors ScolIA (image), le … par … », sans exiger une nouvelle signature tant qu'aucun avenant n'est émis | Re-signer un devis déjà commandé trois semaines avant le départ n'a pas de sens ; le premier avenant passe par la vraie signature (R-SV30) | important | 42, 7 | §32.3 |

**Cas limites.** CV-65 (séjour qui part pendant la bascule, variante du 20 avril), CV-71, CV-80.

### P02 — Mme Roche, une journée au parc des sports et une série piscine

**Situation.** Sortie **obligatoire** d'une journée pour trois classes de 2ᵈᵉ (car payé par l'établissement, pique-niques de la cuisine), et une série piscine le mardi de septembre à décembre. Aujourd'hui : formulaire « sortie simple » en série, validation de toute la série en un geste, paniers.
**Irritants.** Remplir un dossier de voyage pour une journée ; ressaisir la série chaque trimestre. **Peurs.** Une famille à qui l'on demande de l'argent pour une sortie obligatoire ; un élève sans panier. **Ce qui la ravirait.** Créer la journée en un écran ; la série dupliquée d'une année sur l'autre ; « 87 paniers, 4 sans porc, 2 sans arachide » calculé seul.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2210 | **Sortie obligatoire payante pour l'établissement** : le car est une dépense, la part familles est 0 imposée, le budget est équilibré par la ligne « établissement » | R-SV8 pose part = 0 mais le budget (§12.1) exige l'équilibre : l'écran doit le faire seul | important | 42 | §32.5 (précisé), CV-81 |
| M-2211 | **Série : séries de séries** — dupliquer une série d'une année ou d'un trimestre (VOY-102 le fait pour une sortie seule) | Usage fréquent [H] ; parité NX-01 | confort | 42 | VOY-102 (précisé) |
| M-2212 | **Commande cuisine par date** avec heure et lieu de livraison (NX-85) | Parité ; la cuisine a besoin de l'heure | important | 42, 40 | §14 (précisé) |
| M-2213 | **Transport en commun** : billets de groupe, pas de devis ; contournement NX-07 sans « motif » vécu comme une faute | Pour une sortie de proximité, le motif « transport en commun » est la norme | confort | 42 | §32.5 : motifs codés du contournement |

### P03 — Mme Diallo (AESH) et M. Perrin (parent bénévole)

**Situation.** Mme Diallo accompagne Léo au voyage à Séville ; M. Perrin, père d'une participante, accompagne une sortie au musée. Aujourd'hui : noms tapés ou choisis dans l'annuaire, **aucun e-mail aux accompagnateurs** [F], pas de compte pour le parent.
**Irritants (AESH).** Être comptée comme encadrante puis décomptée la veille ; ne pas savoir si son emploi du temps est modifié. **(Parent).** Ne rien recevoir ; ne pas savoir quel groupe il a. **Peurs.** Un élève perdu dans son groupe ; voir des informations d'élèves qu'il ne devrait pas connaître (le parent connaît les familles). **Ce qui les ravirait.** La liste de **son** groupe sur le téléphone, la fiche d'urgence de **ces** élèves seulement, effacée au retour.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2214 | **Accord de l'employeur de l'AESH et modification de son emploi du temps** (G2 fiches 4 et 6 [V]) | Pas dans le §10 ; sans accord, l'AESH ne peut pas partir | important | 42, 6, 10 | VOY-274 |
| M-2215 | **Groupes d'encadrement** : chaque élève rattaché à un adulte référent pour les appels d'étape et la remise | Pratique universelle [H] ; base de la minimisation du bénévole | important | 42 | VOY-276 |
| M-2216 | **Compte bénévole limité** : voit son groupe (nom, photo, classe), la fiche d'urgence de son groupe pendant les dates, les consignes de son groupe ; ni budget, ni paiements, ni liste complète | Q-V19 pose « compte limité au séjour » sans dire ce qu'il voit ; le parent connaît les familles | **important** | 42, 6, 34 | VOY-275, §21.2 (ligne ajoutée) |
| M-2217 | **Bénévole parent d'un participant** : son enfant n'est pas dans son groupe par défaut (proposition) | Neutralité, et l'enfant mérite sa sortie [H] | confort | 42 | CV-74 |
| M-2218 | **Avis aux encadrants** (convocation, programme, ordre de mission, changements) : aujourd'hui aucun [F] | Nouveau, mais irritant majeur | important | 42, 8 | §23 : nature `voyages.encadrant_convocation` |
| M-2219 | **FIJAIS dans le privé** : applicabilité non tranchée [NT] ; pratique variable selon la DSDEN | La conception le place en V8 « public » | important | 42 | Q-V39 [F] |

**Cas limites.** CV-74, CV-75.

### P04 — Mme Duval, cheffe d'établissement

**Situation.** 23 sorties et 4 voyages par an ; signe pédagogie, devis, autorisation finale. Aujourd'hui : tableau de bord des signatures (NX-18), réouverture, refus.
**Irritants.** Signer trois fois le même séjour ; découvrir à la validation finale que trois classes perdent leurs cours de maths de la semaine. **Peurs.** Un voyage sans Ariane ; un élève écarté pour raison financière ; un accident avec un bénévole non autorisé. **Ce qui la ravirait.** À la validation pédagogique, voir **ce que la sortie coûte en cours** (heures perdues par classe et par professeur) et les risques (PAI, étranger, nuitée) en une tuile.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2220 | **Impact pédagogique à la validation** : heures de cours perdues par classe et par professeur, devoirs et conseils en conflit, calculés avant `soumise` → `preparation` | Le §15.1 calcule après `prete` : trop tard pour décider | important | 42, 10 | VOY-285 |
| M-2221 | **Réouverture vers cinq étapes** (NX-83) | Parité | important | 42 | §8.1 (corrigé) |
| M-2222 | **Délégation datée** à l'adjoint (absence de la cheffe en période de départs) | `PERSONAS_24_25.md` M-669 l'a posée pour les finances ; même besoin ici | important | 42, 1 | §21.1 (précisé : délégation datée, déjà citée) |
| M-2223 | **Part annoncée avant bascule conservée** même si elle inclut le coût des accompagnateurs ou un arrondi supérieur (règles de Next.js) ; écart signalé à la direction, qui décide | Changer un prix annoncé aux familles trois semaines avant le départ est pire que l'écart | **important** | 42, 24 | R-SV46 |
| M-2224 | **Séjours « en vol » pendant les vacances** : liste à J-30 du gel, décision par séjour | Bloquant A | **bloquant** | 42 | VOY-258, Q-V31 |

### P05 — Mme Lambert (comptable de l'OGEC) et le secrétariat

**Situation.** Tient les budgets dans Next.js (fiche budget, marge, prix annoncé, synthèse APEL) et les versements dans un **tableur** à part ; reçoit des enveloppes de chèques des professeurs ; paie l'agence en trois acomptes (30 % à la réservation, 40 % à J-60, solde à J-30).
**Irritants.** Rapprocher le tableur, Charlemagne et la fiche budget ; recevoir des chèques sans liste ; ressaisir la ventilation du forfait. **Peurs.** Un chèque perdu entre la classe et son bureau ; payer l'agence en retard (pénalité) ; double facturation entre ScolIA et Charlemagne. **Ce qui la ravirait.** « Séville : 38 à jour, 3 en retard, 1 plan ; agence : acompte 2 dû le 10 mars » ; l'export vers sa comptabilité sans ressaisie.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2225 | **Mode suivi pendant le doublon** : parts, répartition, pointage des versements, export ; **aucune facture** émise par ScolIA | Bloquant C (D-034) | **bloquant** | 42, 24, 25 | R-SV44, S-437, VOY-267, VOY-268 |
| M-2226 | **Import de l'état des versements** tenu hors de Next.js à la bascule | Bloquant B | **bloquant** | 42 | VOY-261 |
| M-2227 | **Bordereau de remise des chèques** collectés par l'organisateur : liste nominative signée, remise le jour même au secrétariat, écart signalé | Un professeur qui manipule des chèques sans trace est le risque n°1 [H] ; dans le public, seul le régisseur manie les fonds | important | 42, 25 | VOY-266, R-SV45 |
| M-2228 | **Échéancier fournisseur** : acomptes dus à l'agence, payés, en retard ; alerte | Absent de la conception ; base de la trésorerie du séjour | important | 42, 48 | VOY-270 |
| M-2229 | **Reprise de la fiche budget** : marge figée en euros → points de base, imputation (compte, ligne), lignes de facturation déclaratives, nombre d'élèves facturés | Champs [F] absents du §25 | important | 42 | §32.6 |
| M-2230 | **TVA du voyage facturé par l'OGEC** et statut au regard du Code du tourisme : non tranchés [NT] | Rien sur les voyages dans le BOFiP lu ; L. 211-1 IV exclut l'organisateur occasionnel, non lucratif, groupe limité [V] | important | 24, 48 | Q-V38 [F] |
| M-2231 | **Synthèse APEL reprise** pour l'année 2026-2027 en cours (séjours d'avant et d'après la bascule dans le même document) | Sinon la synthèse de juin est faite à moitié dans chaque outil | important | 42 | §32.3 (archives comptées) |
| M-2232 | **Secrétariat : liste des pièces manquantes** par séjour (AST, identité, autorisations) commune à tous les séjours | VOY-149 est par séjour | confort | 42 | VOY-149 (précisé) |

### P06 — M. et Mme Garnier, séparés, chacun paie la moitié

**Situation.** Résidence alternée, autorité conjointe ; la mère signe le coupon ; le père veut payer sa moitié lui-même ; aucun désaccord déclaré. Aujourd'hui : un seul coupon, un seul chèque, l'autre parent rembourse l'un (hors de l'établissement).
**Irritants.** Recevoir la circulaire en double (ou pas du tout pour le père) ; payer la totalité puis se faire rembourser. **Peurs (mère).** Que le père voie son adresse ou ses paiements. **(Père).** Être informé après coup. **Ce qui les ravirait.** Chacun sa part, son échéancier, sa quittance ; la même information de séjour.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2233 | **Deux parts, mode suivi** : la répartition du socle 4 s'applique aussi au pointage (pas seulement à la facture) | R-SV44 ; sinon le pointage revient « par famille » | important | 42, 4 | VOY-267 (précisé) |
| M-2234 | **Droit à l'image : un ou deux titulaires ?** Art. 372-1 C. civ. (loi 2024-120) : « les parents protègent en commun le droit à l'image » [lu sur copies secondaires] ; des commentateurs y voient un acte non usuel pour une diffusion publique | La conception demande un consentement sans dire de qui | **important** | 42, 18 | Q-V33, S-439 : journal fermé = un titulaire + information de l'autre avec opposition possible ; diffusion publique = les deux |
| M-2235 | **Un payeur ne paie pas sa part** : l'élève part (CV-13), pas de report sur l'autre sans accord ; le recours relève du 24 (R-F29) | Cas fréquent [H] | important | 42, 24 | CV-77 |
| M-2236 | **Coupon signé avant la bascule par un parent qui a depuis perdu l'autorité** | Reprise d'une attestation : elle doit pouvoir être « à revoir » (§6.4) | important | 42 | CV-69 |

### P07 — Famille Benoît, en difficulté

**Situation.** Père au chômage ; deux enfants, dont un en 1ʳᵉ au voyage à Séville (640 €). Aujourd'hui : l'aide APEL et le fonds social sont notés **nominativement** dans la fiche budget (VD-37) et dans son PDF.
**Irritants.** Expliquer sa situation à l'organisateur. **Peurs.** Que l'enfant sache, ou que la classe sache ; ne pas pouvoir payer et que l'enfant ne parte pas. **Ce qui la ravirait.** Demander l'aide depuis son téléphone, payer en quatre fois, n'en parler à personne.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2237 | **Reprise des aides nominatives de Next.js** : signalées à la comptabilité seule (Q-V29) ; **retirées** du budget repris et de ses PDF | Sinon la fuite VD-37 est reprise | important | 42, 26 | §32.3 |
| M-2238 | **Refus du fonds social** : la direction décide la prise en charge par l'établissement, jamais l'organisateur | « Aucun élève écarté » [V-copie C24 §2.3] | important | 42, 26 | CV-84 |
| M-2239 | **Échéancier au-delà de 3 fois** demandé par la famille : accordé par la comptabilité, sans le dire à l'organisateur | §12.2 le permet ; le mode suivi doit le porter | confort | 42 | VOY-267 (précisé) |

### P08 — Léo (ULIS, fauteuil, AESH) et Inès (PAI allergie)

**Situation.** Léo part à Séville avec Mme Diallo ; l'hôtel doit avoir une chambre accessible, le car une rampe ; Inès a un PAI (adrénaline auto-injectable) et un panier sans arachide.
**Peurs (familles).** Que l'hôtel « oublie » ; que l'allergie soit connue de toute la classe. **(Infirmière).** Que la trousse ne revienne pas. **Ce qui les ravirait.** L'accessibilité vérifiée avant la signature du devis ; « 1 panier sans arachide, étiqueté Inès » sans le mot allergie.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2240 | **Besoins d'accessibilité dans la demande de devis** (chambre, transport adapté) **sans nom ni diagnostic** (« 1 participant en fauteuil ») | CV-18 dit « case » ; le prestataire doit le savoir avant de chiffrer | important | 42 | VOY-156 (précisé) |
| M-2241 | **Surcoût lié au handicap** : ligne dédiée, jamais dans la part familles [V-copie C24 §2.2] ; un car adapté plus cher en est un | Déjà R-SV15 ; à porter jusqu'à la ventilation du forfait (M-2205) | important | 42 | VOY-269 |
| M-2242 | **Chambre et rooming list** : chambre accessible, AESH à proximité, aucun motif transmis | M-2204 | important | 42 | VOY-272 |
| M-2243 | **PAI créé après la bascule pour un séjour repris** : répercussion §6.4 active dès V0 pour les séjours repris | V7 est tard pour un départ le 10 mai | important | 42, 34 | §32.4 (répercussions minimales en V0) |

### P09 — Hugo, 18 ans, terminale

**Situation.** Majeur depuis décembre ; ses parents payent ; il signe lui-même son autorisation. Aucune règle officielle trouvée sur l'information des parents d'un élève majeur pour un voyage [NT].
**Peurs.** Que ses parents reçoivent tout « comme avant ». **(Parents).** Payer sans être informés de rien. **Ce qui le ravirait.** Signer en deux gestes ; ses parents voient leur part et le calendrier, pas plus, s'il le veut.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2244 | **Parents payeurs d'un élève majeur** : ils voient leur part, leur échéancier et les dates du séjour (nécessaires au paiement) ; le reste (journal, avis) seulement avec l'accord d'Hugo (socle 4) | Aucun texte [NT] ; minimisation et capacité civile | important | 42, 4 | VOY-278, Q-V41 |
| M-2245 | **Majorité pendant la préparation** : coupon signé par un parent avant ses 18 ans, reste valable (§6.4) ; nouvelles signatures par lui | Déjà §6.4 ; à tester sur un séjour repris | confort | 42 | CV-76 |
| M-2246 | **Élève majeur de pays tiers** : exclu de la liste de voyageurs, titre de séjour, passeport, visa [V] | §5.4 | important | 42 | §5.4 (précisé) |

### P10 — Famille Petit, part de l'établissement en février

**Situation.** Inscrite au voyage à Séville, 320 € versés, billet d'avion déjà émis au nom de l'élève ; mutation en février, radiation le 26 février. Dans Next.js : l'élève reste dans la liste et « en sortie » (VD-42) ; rien ne se passe côté argent.
**Peurs.** Payer un billet perdu ; être remboursé sur le compte de l'ex-conjoint. **Ce qui la ravirait.** Un calcul clair : ce qui est engagé et non récupérable, ce qui est rendu, à qui.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2247 | **Départ de l'élève = proposition au prestataire** : changement de nom vers un élève en liste d'attente, ou annulation selon le contrat ; frais réels → avoir | §6.4 propose l'avoir, pas l'action prestataire | important | 42 | VOY-286, CV-72 |
| M-2248 | **Accès de la famille partie** : journal fermé, avis arrêtés, mais quittances et remboursement accessibles (socle 3 / 4 : ancien élève) | Sinon elle perd la preuve de ses versements | important | 42, 4 | VOY-286 (précisé) |
| M-2249 | **Reprise d'un participant parti depuis** (radié dans le cœur avant la bascule) : repris `retire`, motif « départ », versements importés pour régularisation | Cas sûr dans la reprise d'avril | important | 42 | CV-68 |

### P11 — Agence « Horizons » (fictive), forfait Séville

**Situation.** Sans compte ; échange par e-mail ; envoie devis, contrat, facture d'acompte, convocation aéroport, liste de chambres à remplir.
**Irritants.** Listes incomplètes ; effectifs qui changent sans avenant ; trois interlocuteurs. **Peurs.** Une annulation tardive sans clause ; ne pas être payée. **Ce qui la ravirait.** Une demande de devis complète, une liste nominative au format qu'elle demande, un avenant numéroté.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2250 | **Contrôle de l'agence** à l'entrée dans l'annuaire : immatriculation au registre des opérateurs de voyages, garantie financière, RC professionnelle (pièces datées) [NT article par article] | Protection des familles en cas de défaillance (CV-73) | important | 42, 48 | VOY-269 |
| M-2251 | **Liste au format demandé** (colonnes choisies parmi les champs autorisés par le contrat) | M-2203 | important | 42 | VOY-271 |
| M-2252 | **Courriel de l'agence d'un domaine voisin** (hameçonnage, changement de RIB) : alerte, et un changement de RIB d'un prestataire se fait **par un humain, avec contre-appel** | Fraude classique sur les acomptes [H] | **important** | 42, 48 | R-SV47 |
| M-2253 | **Informer les prestataires du changement d'outil** (nouvelle adresse de réponse, référence des séjours conservée) à la bascule | Sinon les réponses arrivent dans la boîte de Next.js gelée | important | 42 | VOY-264 |

### P12 — Mme Benali (CPE) et Yanis (AED) : les élèves restés au lycée

**Situation.** Le 10 mai, 42 élèves de 1ʳᵉ partent ; 21 élèves des mêmes classes restent ; le même jour, la journée EPS emmène trois classes de 2ᵈᵉ. Aujourd'hui : « en sortie » à l'appel (NX-35), sur la journée entière, y compris pour les désistés (VD-42).
**Irritants.** Découvrir le matin les élèves restés ; appeler des familles d'élèves « absents » qui sont en sortie. **Peurs.** Un élève désisté resté chez lui sans que personne ne le voie. **Ce qui la ravirait.** La veille, la liste des restés par plage, une salle et un surveillant prévus, le travail des professeurs déposé.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2254 | **Planifier l'accueil des non-partants** : salle, surveillant, plages ; alerte si rien n'est prévu à J-3 | R-SV11 publie la liste, ne planifie pas | important | 42, 12, 10 | VOY-282 |
| M-2255 | **Travail à faire pour les non-partants** déposé par les professeurs dont les séances sont neutralisées | Le cours est vidé pour une partie de la classe | confort | 42, 20 | VOY-283 |
| M-2256 | **Non-partant gardé à la maison par sa famille** = absence ordinaire à justifier, pas « en sortie » | C24 : accueil obligatoire [V-copie] | important | 42, 12 | CV-82 |
| M-2257 | **Bascule le 3 mai** : les séjours repris doivent produire « en sortie » dès la première journée de cours (port du 12, VOY-200, en V0) | Le premier départ suit la rentrée de quelques jours | **important** | 42, 12 | §32.4 |

### P13 — M. Haddad (classe vidée) et Mme Ferrand (qui part)

**Situation.** M. Haddad a un devoir surveillé en 1ʳᵉ C le 11 mai : 14 élèves sur 30 partent. Mme Ferrand part 5 jours et laisse 18 heures de cours dans quatre autres classes.
**Irritants.** Apprendre la sortie par les élèves ; corriger un devoir à deux vitesses. **Peurs.** Un rattrapage impossible avant le conseil. **Ce qui la ravirait.** Être prévenu à la validation pédagogique, pas la veille ; ses séances proposées au remplacement.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2258 | **Avis au professeur dont la séance est vidée** : à la validation, puis à J-7, avec le nombre d'élèves restants et le conflit éventuel | §15.1 signale les conflits sans dire à qui ni quand | important | 42, 8, 21 | VOY-284 |
| M-2259 | **Impact sur les collègues visible par la direction** avant la validation | M-2220 | important | 42 | VOY-285 |
| M-2260 | **Classe entière partie, professeur resté** : heures libérées proposées à la direction (soutien, remplacement d'un collègue parti) | CV-83 ; gain réel pour l'adjoint [H] | confort | 42, 15 | CV-83 |

### P14 — M. Blanc, délégué à la protection des données

**Situation.** Doit inscrire le traitement au registre, juger s'il faut une AIPD, fixer les durées, répondre aux demandes d'accès, encadrer les sous-traitants (agence, transporteurs, hébergeurs, fournisseur d'IA).
**Irritants.** Découvrir les traitements après coup. **Peurs.** Une liste d'élèves avec dates de naissance chez un hôtel hors UE ; des photos de mineurs sans consentement ; un journal de voyage indexé. **Ce qui le ravirait.** Un registre pré-rempli par le socle, avec ses destinataires et transferts.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2261 | **Fiche de registre et AIPD** avant la bascule : mineurs, données de santé (consignes, trousses), localisation, transferts, à grande échelle → AIPD très probable [H, à l'avis du DPO] | Préalable à toute donnée réelle (D-034, D-035 point 4) | **important** | 42 | VOY-287, Q-V40 [F] |
| M-2262 | **Destinataires et transferts** tenus par séjour (qui a reçu quelle liste, quand, quels champs, quel pays) | R-SV41 ; réponse au droit d'accès | important | 42, 7 | VOY-271 (trace) |
| M-2263 | **Droit d'accès d'une famille** (CV-62) sans les autres élèves, y compris les envois au prestataire | CV-62 sans action | confort | 42 | VOY-288 |
| M-2264 | **Données reprises de Next.js** : durées appliquées dès la reprise (les copies d'identité, il n'y en a pas ; le blog n'est pas repris ; les CSV au transporteur avec téléphones des parents sont des pièces à **ne pas** reprendre) | Ne pas reprendre ce qu'on n'aurait pas dû garder | important | 42 | §32.3 |
| M-2265 | **Contexte IA** du séjour repris : la capacité de lecture des devis ne reçoit pas l'historique du séjour Next.js (qui contient des e-mails de parents, VD-24) | Minimisation | important | 42 | R-SV34 (précisé) |

### P15 — Les attaquants

**Mme Sorel**, mère d'un participant, veut « la liste des enfants pour organiser un covoiturage vers l'aéroport » ; **M. Valin**, professeur d'un autre établissement (du même OGEC, puis d'un autre client de ScolIA), veut voir les séjours et les prix.

| ID | Porte essayée | Ce qui doit se passer | Gravité | Ajout |
|---|---|---|---|---|
| M-2266 | **Calendrier `.ics`** et avis groupés : aujourd'hui, parents en **copie cachée par lots** [F] ; une erreur de lot ou une réponse à tous expose les adresses | Un envoi par destinataire (socle 8) ; l'`.ics` ne contient ni participants ni encadrants nominatifs | **important** | R-SV42 |
| M-2267 | **Journal de voyage** : prénoms en légende, élèves marqués sur les photos, commentaires | Le marquage des élèves visibles est **interne** (filtre du droit à l'image), jamais affiché aux familles ; pas de commentaires | **important** | R-SV42 |
| M-2268 | **Fil de réponse aux avis** | Fil individuel (Q-V28) ; jamais de fil de groupe des familles par défaut | important | R-SV42 |
| M-2269 | **« Covoiturage »** : Next.js a un module covoiturage (M-T0) [F, inventaire T0] | Refus de la liste ; proposition d'un fil **à adhésion** (chaque famille choisit d'apparaître) — relève du module covoiturage, pas du 42 | confort | CV-78 |
| M-2270 | **Liste « qui mange »**, export, rooming list | Pas accessibles aux familles ; accompagnant parent : son groupe seulement (M-2216) | important | §21.2 |
| M-2271 | **Professeur d'un autre établissement du même OGEC** sur une sortie liée | Il voit le séjour lié (titre, dates, programme) et **ses** participants, jamais ceux de l'autre établissement ni le budget de l'autre | important | CV-79 |
| M-2272 | **Professeur d'un autre client** : identifiants devinés, référence de courriel, sous-adresse `+etab` | RLS (R-SV14), référence non devinable (Q-V9), **jamais** de rattachement entre établissements (§13.3) ; tests du §21.4 | **important** | §21.4 (pièges ajoutés) |
| M-2273 | **Ombre en lecture pendant la reprise** : séjours Next.js visibles dans ScolIA avant leur bascule | L'ombre suit les **mêmes projections** par rôle que les séjours ScolIA (sinon elle reprend VD-03) | **important** | R-SV37 (précisé) |

### P16 — Florian, administrateur, le 16 avril 2027 au soir

**Situation.** 31 séjours dans Next.js pour l'année : 19 terminés, 7 en préparation (dont Séville), 3 sorties de fin d'année en circuit, 1 brouillon, **1 séjour en vol** (Londres, du 19 au 23 avril). Exports déjà répétés aux vacances de Noël et d'hiver (T1, T2).
**Irritants.** Trois outils (Next.js, ScolIA, tableur) ouverts toute la soirée. **Peurs.** Un séjour perdu, des familles relancées à tort le 3 mai, un journal de voyage coupé en plein séjour à Londres. **Ce qui le ravirait.** Un tableau de bascule vert, séjour par séjour, et un retour arrière en un geste.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2274 | **Classement de chaque séjour à la bascule** (terminé, en préparation, en circuit, brouillon, en vol) et régime associé | Bloquant A | **bloquant** | 42 | VOY-258 |
| M-2275 | **Reprise différentielle** : rejouer la reprise met à jour l'ombre des séjours encore portés par Next.js, sans jamais écraser un séjour dont ScolIA est propriétaire | Les répétitions T1, T2 et la reprise finale du séjour en vol après son retour | **bloquant** | 42 | VOY-259 |
| M-2276 | **Tableau de bord de bascule** : par séjour, propriétaire, état source et cible, participants rapprochés, attestations faites, versements importés, pièces copiées, écarts de montants | Rapport de contrôle de `TRANSITION_NEXT_RUST.md` §2 point 3, rendu par séjour | **important** | 42 | VOY-263 |
| M-2277 | **Envois déjà faits repris** : aucun rappel J-15, calendrier ou liste de paniers ne repart le 3 mai pour un séjour où il est déjà parti | Familles relancées à tort | **important** | 42, 8 | R-SV38, VOY-262 |
| M-2278 | **Liens déjà distribués** (journal de voyage par jeton) : séjour en vol → le lien Next.js reste actif jusqu'au retour ; séjours à venir → nouveau lien nominatif envoyé à l'ouverture du journal | `TRANSITION_T0_INVENTAIRE.md` R-09 | important | 42 | VOY-265 |
| M-2279 | **Retour arrière** : si la reprise échoue, Next.js redevient propriétaire de tous les séjours (sa base n'a pas bougé) ; les séjours créés dans ScolIA entre-temps sont listés | `TRANSITION_NEXT_RUST.md` §2 point 4 dit « trivial » ; ce n'est vrai qu'avant la première écriture dans ScolIA | important | 42 | §32.3 |
| M-2280 | **Statuts hérités** `BROUILLON`, `EN_ATTENTE_VALIDATION` et dossier `soumise` dont les demandes de devis sont **déjà parties** | Absents du §26 [F] | important | 42 | §32.3, CV-66, CV-67 |
| M-2281 | **Format des données** : `travel_attr` (chemin → texte) reconstruit dans une zone d'import **en colonnes texte**, purgée à la clôture de la reprise ; rapport en **lignes de table** (pas de `rapport jsonb`) | D-036 | important | 42 | §32.6 |

### Manques transverses : l'IA, le mobile, les tests

| ID | Ce qui manque | Gravité | Ajout |
|---|---|---|---|
| M-2282 | **Tests de bascule** : reprise d'un export anonymisé contenant chaque état Next.js, rejouée deux fois (idempotence), puis différentielle ; aucun envoi produit par la reprise | **important** | §28 (ajout) |
| M-2283 | **Recette de parité** : la liste NX-01 à NX-86 devient une **check-list de recette** signée par Florian (et un organisateur réel) avant le gel | important | §28 (ajout) |
| M-2284 | **Mobile de l'encadrant** (V6) pas prêt au 3 mai : l'appel de départ (V3) se fait sur le web mobile pour les séjours de mai–juin 2027 | important | §32.5 |
| M-2285 | **IA et séjour repris** : les capacités ne lisent que les tables ScolIA, jamais la zone d'import | important | R-SV34 (précisé) |

---

## 5. Vérification réglementaire (2026-11-10)

Contrôle du §5 du 42 par un sous-agent de recherche. Le BO (education.gouv.fr) renvoie toujours 403 ; ac-paris et ac-reunion aussi.

| # | Point | Résultat | Source | Statut | Écart avec le §5 du 42 |
|---|---|---|---|---|---|
| 1 | Texte en vigueur (public) | Circulaire du 16-7-2024 (NOR MENE2407159C) en vigueur, abroge celle du 13-6-2023 ; le guide éduscol de décembre 2025 la cite comme texte de référence ; aucune circulaire 2025-2026 trouvée (une mention « 16 juillet 2026 » dans un article de presse est une coquille) | copie de Besançon (§5 du 42) ; https://eduscol.education.gouv.fr/sites/default/files/document/guide-sorties-et-voyages-scolaires-second-degre-101619.pdf ; https://www.service-public.gouv.fr/particuliers/vosdroits/F22575 (vérifiée le 17-2-2026) | [V-copie], [V] | aucun |
| 2 | Coût des accompagnateurs | « D'aucune manière, le coût de la sortie scolaire des accompagnateurs ne peut être imputé, même indirectement, aux familles » : c'est **G2 fiche 5**, pas la circulaire ; F22575 : « leurs frais […] sont pris en charge par l'établissement scolaire » | G2 fiche 5 ; F22575 | [V] | **source à corriger** au §5.5 |
| 3 | Privé sous contrat | Aucun texte officiel n'impose ni n'autorise la facturation de la part des accompagnateurs ; C24 : remontée d'informations par les organismes de gestion ; FNOGEC (fiche « Tarification sociale », 7-5-2024) : éviter que le prix des sorties soit source de discrimination ; une note académique (La Réunion, 1-9-2025, 403, lue en extrait) citerait R. 442-55 | C24 ; FNOGEC | [V], [NT] | aucun ; ajouter la note non vérifiée |
| 4 | Assurance | Ne peut être exigée (G2 fiche 10) ; F22575 : « pas obligatoire » ; la page CNIL contraire date du **12-9-2017** ; justice.fr dit encore « doit être assuré » | G2 ; F22575 ; cnil.fr | [V] | dater la page CNIL au §5.7 |
| 5 | Encadrement | AESH et service civique non comptés ; accord de l'AESH et de son employeur si son emploi du temps change (G2 fiches 4 et 6) ; FIJAIS pour chaque voyage dans le public (C24 §1.3, G2 fiche 4) ; **privé : non tranché** | G2 ; C24 | [V], [V-copie], [NT] | ajouter l'accord de l'employeur de l'AESH ; FIJAIS du privé à demander à la DSDEN |
| 6 | Mineurs à l'étranger | AST : un seul titulaire signe, validité ≤ 1 an, copie de la pièce du signataire (CNI ou passeport, valide ou périmé depuis moins de 5 ans ; non européen : passeport et titre de séjour), formulaire en ligne R46121 ; OST : départ impossible ; IST : après levée. G2 fiche 7 : noms différents → pièce prouvant l'autorité parentale ; conflit connu → accord recherché, désaccord écrit. **Royaume-Uni** (gov.uk, mise à jour du 5-6-2026) : groupe d'au moins 5 enfants de 18 ans ou moins d'une école française, formulaire « France-UK school trip travel information form » à la préfecture **au moins 15 jours avant**, CNI acceptée pour UE/EEE/Suisse, enfants dispensés de visa et d'ETA quelle que soit leur nationalité, **adultes : passeport et ETA ou visa** | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032634037 ; https://www.service-public.gouv.fr/particuliers/vosdroits/F1359 (vérifiée le 30-10-2026) ; gov.uk « Visit the UK as part of a French school trip » | [V] | **remplacer « à vérifier pour le Royaume-Uni »** au §5.4 |
| 7 | Élève majeur | Aucun texte : C24 et G2 visent l'élève mineur | — | [NT] | ajouter [NT] ; DPO et juriste (Q-V41) |
| 8 | Données | CNIL 2017 : mention du cas sans nature du handicap ; allergies avec consentement exprès ; fiche sanitaire non informatisée ; rien sur l'image ni les durées. **Droit à l'image** : art. 372-1 C. civ. (loi 2024-120) « les parents protègent en commun » | cnil.fr ; copies secondaires de la loi | [V], [V partiel] | **ajouter le droit à l'image** au §5.10 |
| 9 | Parents séparés | Un titulaire suffit, accord de l'autre présumé sauf désaccord (G2 fiche 7, F22575), y compris à l'étranger ; aucune jurisprudence trouvée sur « voyage à l'étranger = acte non usuel » | G2 ; F22575 | [V], [NT] | aucun |
| 10 | Non-partants | Accueillis dans l'établissement, avec un programme adapté | C24 §2 ; F22575 | [V-copie], [V] | aucun |
| 11 | TVA, Code du tourisme | Exonération de l'enseignement et de ses compléments (CGI 261-4-4°, BOFiP) : **rien sur les voyages** ; C. tourisme L. 211-1 IV : exclusion de l'organisateur « à titre occasionnel, dans un but non lucratif et pour un groupe limité de voyageurs » ; agences : immatriculation et garantie non lues article par article | Légifrance (L. 211-1) ; BOFiP BOI-TVA-CHAMP-30-10-20-50 | [V], [NT] | ajouter (Q-V38) |
| 12 | Chèques-vacances | Possibles si l'établissement est agréé ANCV (EPLE) ; pour un OGEC, conventionnement de droit commun [NT] | G2 ; F22575 | [V], [NT] | aucun |

---

## 6. Manques qui relèvent d'autres socles

| Socle | Manque | Renvoi |
|---|---|---|
| 4 | Port `personnes::assurance` et `opposition_sortie_territoire` déjà « à créer » ; **règle de l'élève majeur** (ce que voient ses parents payeurs) | M-2244 |
| 6 | Accord de l'employeur de l'AESH ; intervenant ponctuel créé en un geste depuis le séjour | M-2214, NX-86 |
| 8 | Natures `voyages.encadrant_convocation`, `voyages.cours_impacte`, `voyages.journal_ouvert` ; remises historiques importées (envois déjà faits) | M-2218, M-2258, R-SV38 |
| 10 | Impact calculé **avant** validation (simulation de neutralisation sans publication) | M-2220 |
| 12 | Port `localiser` disponible pour les séjours repris dès le 3 mai 2027 | M-2257 |
| 18 | Consentement image : règle un / deux titulaires (S-439) | M-2234 |
| 20 | Travail déposé pour les non-partants (cahier de textes) | M-2255 |
| 24, 25 | **Mode suivi** (pointage sans facture) et export vers Charlemagne pendant le doublon | M-2225 |
| 40, 41 | Consommation de `sorties::en_sortie` (prévision cantine, repas du soir et sorties de l'internat) | NX-82 |
| 48 | Annuaire des prestataires partagé ; échéancier fournisseur ; changement de RIB avec contre-appel | M-2228, M-2252 |

---

## 7. Synthèse

### 7.1 Manques par gravité

| Gravité | Nombre | Exemples |
|---|---|---|
| Bloquant | 7 | M-2201, M-2202, M-2224, M-2225, M-2226, M-2274, M-2275 (plus le bloquant D du §0, qui n'est pas un manque numéroté) |
| Important | 68 | M-2203, M-2216, M-2223, M-2234, M-2252, M-2261, M-2266, M-2277 |
| Confort | 10 | M-2211, M-2213, M-2239, M-2260, M-2263 |
| **Total** | **85** | M-2201 à M-2285 |

Ajouts jugés **différenciants** (rares sur le marché [H]) : VOY-285 (impact pédagogique avant validation), VOY-263 (tableau de bascule), VOY-272 (répartition des chambres sans motif), VOY-276 (groupes d'encadrement).

### 7.2 Ajouts dans la conception (`42_SORTIES_VOYAGES.md`)

- §2.2 : NX-75 à NX-86 ; §2.3 : VD-49, VD-50.
- §5 : corrections et ajouts réglementaires (sources, Royaume-Uni, droit à l'image, élève majeur, AESH, FIJAIS privé, tourisme et TVA).
- §8.1 : réouverture vers cinq étapes.
- **§32 (nouveau, « ajout personas »)** : bascule d'avril (§32.1–32.4 : propriété par séjour, classement, correspondance des états, état de départ attesté, envois repris, retour arrière), parcours et argent pendant le doublon (§32.5), normalisation D-036 du modèle (§32.6), actions VOY-250 à VOY-289 (§32.7), règles R-SV37 à R-SV47 (§32.8), cas limites CV-65 à CV-84 (§32.9), lots révisés (§32.10), réglages S-436 à S-439 et questions Q-V31 à Q-V43 (§32.11).
- §25, §26, §27 : renvoi en tête vers le §32 (le texte d'origine est gardé ; le §32 l'emporte en cas de contradiction, écrite au §31).
- `MIGRATIONS.md` : exceptions D-036 du socle 42 déclarées.

### 7.3 Les dix ajouts les plus importants

1. **Propriété unique par séjour** et ombre en lecture (R-SV37, VOY-258, VOY-259).
2. **État de départ attesté** : autorisations papier et versements tenus hors de Next.js (R-SV39, VOY-260, VOY-261).
3. **Mode suivi de l'argent** pendant le doublon, conforme à D-034 (R-SV44, V2a).
4. **V0 = parité complète** : NX-75 à NX-86 et les trois reculs corrigés.
5. **Envois déjà faits repris** : rien ne repart le 3 mai (R-SV38).
6. **Part annoncée conservée** pour les séjours repris (R-SV46).
7. **Liste nominative au prestataire selon le contrat**, transferts hors UE tracés (R-SV41, VOY-271, VOY-289).
8. **Aucune famille ne voit les autres participants** (R-SV42).
9. **Modèle sans JSON métier** (§32.6).
10. **Impact pédagogique avant validation** et avis aux professeurs dont la séance est vidée (VOY-284, VOY-285).

### 7.4 Ce que la passe a montré sur la conception

- Le fond est juste ; le **moment** de la bascule était sous-conçu. Ce n'est pas une reprise de données, c'est une **passation de séjours vivants**.
- « Reprendre les actions métier » (D-035) suppose de savoir **où vivent les faits** : pour les voyages, une partie essentielle (autorisations, versements) n'a jamais été dans Next.js.
- D-034 et D-035 se croisent sur l'argent : le module en usage réel ne peut pas facturer tant que les finances sont en doublon.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)

| N° | Décision appliquée |
|---|---|
| Q-V31 (défaut) | Gel du module Next.js le vendredi 16 avril 2027 au soir ; tous les séjours non clos passent à ScolIA, sauf les séjours en vol (départ du 17 avril au 5 mai), qui finissent dans Next.js et sont repris en archive après leur retour. **La date et le geste du gel restent à Florian** (§8.2). |
| Q-V33 | Droit à l'image : journal fermé aux familles des participants = consentement d'un titulaire, information de l'autre avec opposition possible ; toute diffusion publique = les deux titulaires (S-439) ; à l'avis du DPO |
| Q-V34 | Demandes de devis après la validation pédagogique (R-SV5 maintenue), préparées avant ; **changement d'usage signalé** |
| Q-V35 | « Groupe scolaire » = sorties liées, chaque établissement signe pour ses participants |
| Q-V36 | Fiche « actualité » de l'assistant IA abandonnée |
| Q-V37 | Mode suivi de l'argent tant que les finances restent dans Charlemagne (S-437) |
| Q-V41 | Parents payeurs d'un élève majeur : part, échéancier et dates ; le reste avec l'accord de l'élève |
| Q-V42 | Remise des chèques collectés le jour même, avec bordereau (S-438) |
| — | V0 élargi (NX-75 à NX-86, image de catalogue, pré-remplissage du budget, consentement image minimal, autorisation papier) ; V2 scindé en V2a et V2b ; §25 normalisé |

### 8.2 Soumis à Florian (engagent l'établissement à l'extérieur, ou sont des faits que seul Florian connaît)

| N° | Question | Pourquoi Florian |
|---|---|---|
| Q-V31 [F] | **Date du gel** et **liste des séjours en vol** ; geste de mise en lecture seule de Next.js (et maintien en écriture des seuls séjours en vol, si Next.js le permet) | Next.js intouchable pour le pilote (CLAUDE.md) |
| Q-V32 [F] | **Où vivent aujourd'hui** les coupons et les versements des voyages (classeur, tableur, Charlemagne ?) ; qui fournit l'état de départ | Fait de terrain inconnu du pilote |
| Q-V38 [F] | TVA de la participation facturée par l'OGEC ; statut au regard du Code du tourisme | Expert-comptable, rescrit |
| Q-V39 [F] | Contrôle FIJAIS des bénévoles dans le privé | Démarche auprès de la DSDEN |
| Q-V40 [F] | Fiche de registre et AIPD avant la bascule | DPO ; préalable aux données réelles |
| Q-V43 [F] | Transferts hors UE (agences, hébergeurs) : base et information des familles | DPO |
| — | Les questions [F] déjà ouvertes au §30 du 42 (Q-V1, Q-V2, Q-V3, Q-V6, Q-V7, Q-V13, Q-V30) | inchangées |
| — | Défauts VD-49 et VD-50 de Next.js (à ajouter à Q-V30) | décision et geste de Florian |
| — | Les changements d'usage signalés : demandes de devis après validation (Q-V34), image de couverture sans recherche web par défaut (Q-V17), arrondi de la part à l'euro inférieur (Q-V4) pour les **nouveaux** séjours | usage des personnels |

## 8 bis. Réglages signalés (D-023)

| N° | Réglage | Défaut |
|---|---|---|
| S-436 | Date de gel du module Next.js (reprise) | fixée par Florian ; proposition : vendredi 16 avril 2027 au soir |
| S-437 | Mode de l'argent des voyages : suivi / facturation | suivi tant que D-034 garde les finances dans Charlemagne |
| S-438 | Délai de remise des chèques collectés par l'organisateur | le jour même |
| S-439 | Consentement au droit à l'image pour le journal fermé | un titulaire + information de l'autre ; diffusion publique : les deux |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** (code Next.js, `origin/main`, lu seulement) : 52 routes `api/travels/*` ; aucun paiement de famille lié aux voyages (EasyTransac facture les établissements clients ; `facture`, `encaissement`, `facture_echeance` sans lien voyage ; `depense.travel_id` jamais passé par `/api/compta`) ; engagement de paiement et autorisation sur coupon papier ; `droit_image_ok` vrai par défaut ; aucun e-mail aux accompagnateurs ; envois aux parents en copie cachée par lots ; demandes de devis à la création ; réouverture vers cinq étapes ; statuts hérités encore testés ; stockage en `travel_attr` (chemin → texte), `settings__modules__travels`, `settings__notifications`, fichiers et JSON S3 (`travels/email-devis-unmatched.json`, marqueurs d'ingestion).

**Hypothèses [H]** : coupons dans un classeur, versements dans un tableur ; séjours pendant les vacances ; usage de l'option « groupe scolaire » ; agences avec acomptes ; billets nominatifs non cessibles ; AIPD nécessaire.

**Contradictions relevées** :
1. `42` §26 point 5 (séjours en cours repris dans leur état) contre point 8 (séjours en cours terminés dans Next.js) : tranché par R-SV37 et §32.
2. `42` §27 (« parcours minimal pour la rentrée 2027 » avec V2 facturation) contre D-034 (finances dans Charlemagne pendant le doublon) : tranché par le mode suivi (V2a).
3. `42` §25 (`jours jsonb`, `rapport jsonb`, tableaux sans clé étrangère) contre D-036 : tranché par §32.6.
4. `42` §8.1 (réouverture vers trois étapes) contre l'usage Next.js (cinq) : corrigé.
5. `42` §5.5 attribuait à la circulaire de 2024 la règle sur les accompagnateurs, qui vient du guide de 2025 : corrigé.
6. `TRANSITION_NEXT_RUST.md` §2 point 4 (« retour arrière trivial ») : vrai seulement avant la première écriture dans ScolIA (M-2279).
7. `PERSONAS_24_25.md` M-753 cite la circulaire du 13-6-2023 (abrogée par celle de 2024) : la règle « en aucun cas » est reprise à l'identique par C24 §2.3 ; à corriger à la prochaine relecture du 24.
